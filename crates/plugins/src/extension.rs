//! The host of extensions: WebAssembly components that answer requests.
//!
//! An extension runs in a sandbox. It gets no files, no network, no environment and no
//! arguments. It can call only the functions of the interface `host` of
//! `wit/extension.wit`. Each call has a limit for its memory and for its work, so an
//! extension that is broken or hostile cannot stop the server.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "extension",
    });
}

use bindings::organizr::extension::host::{Host, LogLevel};

/// The most memory of one call of an extension, in bytes.
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

/// The work that one call of an extension can do. A unit is about one instruction of
/// WebAssembly, so this is a few seconds on one processor core.
const FUEL: u64 = 2_000_000_000;

/// The most bytes of the values that an extension stores, keys and values together.
pub const STORAGE_LIMIT: usize = 1024 * 1024;

/// The most bytes of the body of an answer of an extension.
pub const RESPONSE_LIMIT: usize = 8 * 1024 * 1024;

/// The most characters of one line that an extension writes to the log.
const LOG_LINE_LIMIT: usize = 2000;

/// The most lines that one call writes to the log.
const LOG_LINES: u32 = 50;

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
#[derive(Debug, Clone, Default)]
pub struct Call {
    /// The id of the extension, for the log.
    pub extension: String,
    /// Whether the extension can store values.
    pub can_store: bool,
    /// The values that the extension stored before.
    pub values: BTreeMap<String, String>,
}

/// What a call gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The answer.
    pub response: Response,
    /// The values of the extension after the call, when the call changed them.
    pub values: Option<BTreeMap<String, String>>,
}

/// What the store of one call holds.
struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    call: Call,
    changed: bool,
    log_lines: u32,
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
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(runtime_failed)?;
        bindings::Extension::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)
            .map_err(runtime_failed)?;
        Ok(Self {
            engine,
            linker: Arc::new(linker),
            compiled: Arc::new(Mutex::new(HashMap::new())),
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
        bindings::Extension::instantiate(&mut store, &component, &self.linker)
            .map(|_| ())
            .map_err(|error| ExtensionError::NotAnExtension(format!("{error:#}")))
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
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY_LIMIT)
                .instances(20)
                .tables(20)
                .memories(20)
                .build(),
            call,
            changed: false,
            log_lines: 0,
        };
        let mut store = Store::new(&self.engine, state);
        store.limiter(|state| &mut state.limits);
        store.set_fuel(FUEL).map_err(runtime_failed)?;
        Ok(store)
    }

    /// Sends a request to an extension and waits for its answer.
    ///
    /// This blocks while the extension works, so a server calls it on a thread for
    /// blocking work.
    pub fn handle(
        &self,
        wasm: &[u8],
        sha256: &str,
        call: Call,
        request: Request,
    ) -> Result<Outcome, ExtensionError> {
        let component = self.component(wasm, sha256)?;
        let mut store = self.store(call)?;
        let instance = bindings::Extension::instantiate(&mut store, &component, &self.linker)
            .map_err(|error| ExtensionError::NotAnExtension(format!("{error:#}")))?;
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
        let response = instance
            .call_handle(&mut store, &request)
            .map_err(|error| ExtensionError::Stopped(format!("{:#}", error.root_cause())))?;
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
