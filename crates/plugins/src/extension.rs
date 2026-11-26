//! The host of extensions: WebAssembly components that answer requests.
//!
//! An extension runs in a sandbox. It gets no files, no environment and no arguments.
//! It can call only the functions of the interface `host` of `wit/extension.wit`, and it
//! reaches the network only through `send-request`, which the server checks with an
//! [`HttpGate`] for the sites that an Admin approved. Each call has a limit for its memory,
//! for its work and for its time, so an extension that is broken or hostile cannot stop
//! the server.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, ResourceLimiter, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "extension",
        // A call runs as a future, so that the deadline of the call can end it also
        // while the extension waits.
        exports: { default: async },
    });
}

use bindings::organizr::extension::host::{
    Host, HttpRequest as WitHttpRequest, HttpResponse as WitHttpResponse, LogLevel,
};

/// The most memory of one call of an extension, in bytes.
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

/// The work that one call of an extension can do. A unit is about one instruction of
/// WebAssembly, so this is a few seconds on one processor core.
const FUEL: u64 = 2_000_000_000;

/// The work after which a call lets the deadline be checked, in units of fuel.
const FUEL_BETWEEN_CHECKS: u64 = 10_000_000;

/// The most time of one call of an extension, from the start of the instance to the
/// answer. A wait, as for a clock of the system interface, uses no fuel, so only this
/// stops it. The requests of a call to other sites count too.
pub const CALL_DEADLINE: Duration = Duration::from_secs(30);

/// The most bytes of the values that an extension stores, keys and values together.
pub const STORAGE_LIMIT: usize = 1024 * 1024;

/// The most bytes of the body of an answer of an extension.
pub const RESPONSE_LIMIT: usize = 8 * 1024 * 1024;

/// The most characters of one line that an extension writes to the log.
const LOG_LINE_LIMIT: usize = 2000;

/// The most lines that one call writes to the log.
const LOG_LINES: u32 = 50;

/// The most requests to other sites that one call sends.
pub const REQUEST_COUNT_LIMIT: u32 = 20;

/// The most bytes of the body of a request of an extension to another site.
pub const OUTGOING_BODY_LIMIT: usize = 1024 * 1024;

/// A request of an extension to another site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// The method in capital letters.
    pub method: String,
    /// The whole address.
    pub url: String,
    /// The headers.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

/// The answer of another site to an extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// The HTTP status.
    pub status: u16,
    /// The headers.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

/// What sends the requests of an extension: the server, which checks the address
/// against the sites that an Admin approved. The call of an extension runs on a thread
/// for blocking work, so `send` blocks.
pub trait HttpGate: Send + Sync {
    /// Sends a request, or says why not.
    fn send(&self, request: HttpRequest) -> Result<HttpResponse, String>;
}

/// Why an extension gave no answer.
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    /// The runtime could not be set up.
    #[error("the runtime for extensions cannot start: {0}")]
    Runtime(String),
    /// The file is no component for this interface.
    #[error("the file is no extension for this version of Organizr: {0}")]
    NotAnExtension(String),
    /// The call used more work or more memory than a call can, or the extension
    /// stopped with an error.
    #[error("the extension stopped: {0}")]
    Stopped(String),
    /// The answer is larger than an answer can be.
    #[error("the answer of the extension is larger than {RESPONSE_LIMIT} bytes")]
    AnswerTooLarge,
}

/// The person who sent a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    /// The user name. A guest has none.
    pub username: Option<String>,
    /// The access level of the group of the caller.
    pub level: u16,
}

/// A request to a route of an extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The method in capital letters.
    pub method: String,
    /// The path below the route of the extension. It starts with `/`.
    pub path: String,
    /// The query without the question mark.
    pub query: String,
    /// The headers that the extension can see.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
    /// Who sent the request.
    pub caller: Caller,
}

/// The answer of an extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The HTTP status.
    pub status: u16,
    /// The headers.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

/// What one call of an extension works with.
#[derive(Clone, Default)]
pub struct Call {
    /// The id of the extension, for the log.
    pub extension: String,
    /// Whether the extension can store values.
    pub can_store: bool,
    /// The values that the extension stored before.
    pub values: BTreeMap<String, String>,
    /// What sends the requests of the extension to other sites, when it may send any.
    pub http: Option<Arc<dyn HttpGate>>,
}

impl std::fmt::Debug for Call {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Call")
            .field("extension", &self.extension)
            .field("can_store", &self.can_store)
            .field("values", &self.values.len())
            .field("http", &self.http.is_some())
            .finish()
    }
}

/// What a call gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The answer.
    pub response: Response,
    /// The values of the extension after the call, when the call changed them.
    pub values: Option<BTreeMap<String, String>>,
}

/// The most instances, tables and linear memories of one call.
const MOST_PARTS: usize = 20;

/// The memory of one call: the linear memories and the tables of all its instances
/// together stay within [`MEMORY_LIMIT`] (D-143). A limit for each memory alone would
/// let a call with many memories, or with a large table, take far more.
#[derive(Debug, Default)]
struct CallLimits {
    /// The bytes that the memories and the tables of the call take.
    used: usize,
}

impl CallLimits {
    fn grow(&mut self, more: usize) -> bool {
        match self.used.checked_add(more) {
            Some(used) if used <= MEMORY_LIMIT => {
                self.used = used;
                true
            }
            _ => false,
        }
    }
}

impl ResourceLimiter for CallLimits {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(self.grow(desired.saturating_sub(current)))
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        // Each element of a table takes a pointer's worth of memory.
        let more = desired
            .saturating_sub(current)
            .saturating_mul(std::mem::size_of::<usize>());
        Ok(self.grow(more))
    }

    fn instances(&self) -> usize {
        MOST_PARTS
    }

    fn tables(&self) -> usize {
        MOST_PARTS
    }

    fn memories(&self) -> usize {
        MOST_PARTS
    }
}

/// What the store of one call holds.
struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: CallLimits,
    call: Call,
    changed: bool,
    log_lines: u32,
    requests: u32,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl State {
    fn stored_bytes(&self) -> usize {
        self.call
            .values
            .iter()
            .map(|(key, value)| key.len() + value.len())
            .sum()
    }
}

impl Host for State {
    fn log(&mut self, level: LogLevel, message: String) {
        // An extension cannot fill the log.
        if self.log_lines >= LOG_LINES {
            return;
        }
        self.log_lines += 1;
        let message: String = message.chars().take(LOG_LINE_LIMIT).collect();
        let extension = self.call.extension.as_str();
        match level {
            LogLevel::Debug => tracing::debug!(extension, "{message}"),
            LogLevel::Info => tracing::info!(extension, "{message}"),
            LogLevel::Warn => tracing::warn!(extension, "{message}"),
            LogLevel::Error => tracing::error!(extension, "{message}"),
        }
    }

    fn get_value(&mut self, key: String) -> Option<String> {
        if !self.call.can_store {
            return None;
        }
        self.call.values.get(&key).cloned()
    }

    fn set_value(&mut self, key: String, value: String) -> Result<(), String> {
        if !self.call.can_store {
            return Err("this extension has no permission to store values".to_owned());
        }
        let before = self
            .call
            .values
            .get(&key)
            .map_or(0, |old| key.len() + old.len());
        if self.stored_bytes() - before + key.len() + value.len() > STORAGE_LIMIT {
            return Err(format!(
                "the values of an extension can have {STORAGE_LIMIT} bytes at most"
            ));
        }
        self.call.values.insert(key, value);
        self.changed = true;
        Ok(())
    }

    fn delete_value(&mut self, key: String) {
        if self.call.can_store && self.call.values.remove(&key).is_some() {
            self.changed = true;
        }
    }

    fn send_request(&mut self, request: WitHttpRequest) -> Result<WitHttpResponse, String> {
        let Some(gate) = self.call.http.clone() else {
            return Err("this extension has no permission to send requests".to_owned());
        };
        if self.requests >= REQUEST_COUNT_LIMIT {
            return Err(format!(
                "a call of an extension sends {REQUEST_COUNT_LIMIT} requests at most"
            ));
        }
        if request.body.len() > OUTGOING_BODY_LIMIT {
            return Err(format!(
                "the body of a request can have {OUTGOING_BODY_LIMIT} bytes at most"
            ));
        }
        self.requests += 1;
        let answer = gate.send(HttpRequest {
            method: request.method,
            url: request.url,
            headers: request.headers,
            body: request.body,
        })?;
        Ok(WitHttpResponse {
            status: answer.status,
            headers: answer.headers,
            body: answer.body,
        })
    }
}

fn runtime_failed(error: impl std::fmt::Display) -> ExtensionError {
    ExtensionError::Runtime(error.to_string())
}

/// The runtime for extensions. One runtime serves every extension.
#[derive(Clone)]
pub struct Extensions {
    engine: Engine,
    linker: Arc<Linker<State>>,
    /// The components that were compiled, by the SHA-256 of their file.
    compiled: Arc<Mutex<HashMap<String, Component>>>,
    /// The most time of one call.
    deadline: Duration,
}

impl std::fmt::Debug for Extensions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Extensions").finish_non_exhaustive()
    }
}

impl Extensions {
    /// Sets the runtime up.
    pub fn new() -> Result<Self, ExtensionError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).map_err(runtime_failed)?;
        let mut linker = Linker::new(&engine);
        // The system interface that a component built from Rust asks for. The context
        // of each call gives it nothing: no folder, no network, no variable.
        wasmtime_wasi::p2::add_to_linker_async(&mut linker).map_err(runtime_failed)?;
        bindings::Extension::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)
            .map_err(runtime_failed)?;
        Ok(Self {
            engine,
            linker: Arc::new(linker),
            compiled: Arc::new(Mutex::new(HashMap::new())),
            deadline: CALL_DEADLINE,
        })
    }

    /// The runtime with another most time of one call than [`CALL_DEADLINE`].
    #[must_use]
    pub fn with_call_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Runs the work of one call, and ends it when it takes longer than the deadline.
    fn run<T>(
        &self,
        work: impl Future<Output = Result<T, ExtensionError>>,
    ) -> Result<T, ExtensionError> {
        let deadline = Instant::now() + self.deadline;
        // The system interface waits with the timers of Tokio.
        wasmtime_wasi::runtime::with_ambient_tokio_runtime(|| wait_until(deadline, work))
            .unwrap_or_else(|| {
                Err(ExtensionError::Stopped(format!(
                    "the call took longer than {} seconds",
                    self.deadline.as_secs_f64()
                )))
            })
    }

    /// The compiled component of a file. `sha256` names the content of the file, so a
    /// file is compiled once.
    fn component(&self, wasm: &[u8], sha256: &str) -> Result<Component, ExtensionError> {
        if let Ok(compiled) = self.compiled.lock()
            && let Some(component) = compiled.get(sha256)
        {
            return Ok(component.clone());
        }
        let component = Component::new(&self.engine, wasm)
            .map_err(|error| ExtensionError::NotAnExtension(format!("{error:#}")))?;
        if let Ok(mut compiled) = self.compiled.lock() {
            compiled.insert(sha256.to_owned(), component.clone());
        }
        Ok(component)
    }

    /// Checks that a file is an extension for this interface, without a call.
    pub fn check(&self, wasm: &[u8], sha256: &str) -> Result<(), ExtensionError> {
        let component = self.component(wasm, sha256)?;
        let mut store = self.store(Call::default())?;
        self.run(async {
            bindings::Extension::instantiate_async(&mut store, &component, &self.linker)
                .await
                .map(|_| ())
                .map_err(|error| ExtensionError::NotAnExtension(format!("{error:#}")))
        })
    }

    /// Forgets the compiled component of a file, for a file that is gone.
    pub fn forget(&self, sha256: &str) {
        if let Ok(mut compiled) = self.compiled.lock() {
            compiled.remove(sha256);
        }
    }

    fn store(&self, call: Call) -> Result<Store<State>, ExtensionError> {
        let state = State {
            wasi: WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            limits: CallLimits::default(),
            call,
            changed: false,
            log_lines: 0,
            requests: 0,
        };
        let mut store = Store::new(&self.engine, state);
        store.limiter(|state| &mut state.limits);
        store.set_fuel(FUEL).map_err(runtime_failed)?;
        store
            .fuel_async_yield_interval(Some(FUEL_BETWEEN_CHECKS))
            .map_err(runtime_failed)?;
        Ok(store)
    }

    /// Sends a request to an extension and waits for its answer.
    ///
    /// This blocks while the extension works, so a server calls it on a thread for
    /// blocking work. A call that takes longer than its deadline is stopped.
    pub fn handle(
        &self,
        wasm: &[u8],
        sha256: &str,
        call: Call,
        request: Request,
    ) -> Result<Outcome, ExtensionError> {
        let component = self.component(wasm, sha256)?;
        let mut store = self.store(call)?;
        let request = bindings::Request {
            method: request.method,
            path: request.path,
            query: request.query,
            headers: request.headers,
            body: request.body,
            caller: bindings::Caller {
                username: request.caller.username,
                level: request.caller.level,
            },
        };
        let response = self.run(async {
            let instance =
                bindings::Extension::instantiate_async(&mut store, &component, &self.linker)
                    .await
                    .map_err(|error| ExtensionError::NotAnExtension(format!("{error:#}")))?;
            instance
                .call_handle(&mut store, &request)
                .await
                .map_err(|error| ExtensionError::Stopped(format!("{:#}", error.root_cause())))
        })?;
        if response.body.len() > RESPONSE_LIMIT {
            return Err(ExtensionError::AnswerTooLarge);
        }
        let state = store.into_data();
        Ok(Outcome {
            response: Response {
                status: response.status,
                headers: response.headers,
                body: response.body,
            },
            values: state.changed.then_some(state.call.values),
        })
    }
}

/// Wakes the thread that waits for a future.
struct WakeThread(std::thread::Thread);

impl Wake for WakeThread {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// Waits on this thread for a future, until the deadline. `None` when the deadline came
/// first: the future is then dropped, which ends the call in it.
fn wait_until<F: Future>(deadline: Instant, future: F) -> Option<F::Output> {
    let waker = Waker::from(Arc::new(WakeThread(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return Some(output);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        std::thread::park_timeout(left);
    }
}

#[cfg(test)]
mod tests {
    use wasmtime::ResourceLimiter;

    use super::{CallLimits, MEMORY_LIMIT};

    const MIB: usize = 1024 * 1024;

    #[test]
    fn memories_and_tables_of_a_call_share_one_limit() {
        // Two memories of 40 MiB are more than one call may have together (D-143).
        let mut limits = CallLimits::default();
        assert!(limits.memory_growing(0, 40 * MIB, None).expect("no trap"));
        assert!(!limits.memory_growing(0, 40 * MIB, None).expect("no trap"));
        assert!(limits.memory_growing(0, 20 * MIB, None).expect("no trap"));

        // A table counts its elements as pointers, in the same limit.
        let mut limits = CallLimits::default();
        let elements = MEMORY_LIMIT / std::mem::size_of::<usize>();
        assert!(
            !limits
                .table_growing(0, elements + 1, None)
                .expect("no trap")
        );
        assert!(limits.table_growing(0, 1000, None).expect("no trap"));
        assert!(limits.memory_growing(0, 60 * MIB, None).expect("no trap"));
        assert!(
            !limits
                .table_growing(1000, 1_000_000, None)
                .expect("no trap")
        );
    }
}
