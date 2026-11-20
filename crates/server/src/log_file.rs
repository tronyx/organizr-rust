//! The log file: each line of the log of the application, also as a JSON record in the
//! file of the day in `<data folder>/logs`, as PHP Organizr keeps its log.
//!
//! The file gets the lines of the crates of this application only, the lines that the
//! standard output also shows when its level lets them through. Lines of the libraries
//! below, for example of the database driver, stay out of the file. The `logLevel`
//! setting chooses the lowest level of the file, and `maxLogFiles` the number of files
//! that stay. A change of either setting counts from the next line on.
//!
//! A request never waits for the disk: a line goes to a queue, and a thread of its own
//! writes the queue to the file. When the queue is full, a line is dropped. When the
//! folder cannot be written, the server says so once on the standard error and goes on.

use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use organizr_api::SettingsWatch;
use organizr_api::log_files::{self, Record};
use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// The number of lines that can wait for the disk. More are dropped.
const QUEUE: usize = 8192;

/// The most lines that are written in one go.
const BATCH: usize = 1024;

/// The level of the file before the settings are read: the default of `logLevel`.
const FIRST_LEVEL: &str = "INFO";

/// The number of files before the settings are read: the default of `maxLogFiles`.
const FIRST_MAX_FILES: u32 = 7;

/// What the writer needs to know from the settings.
pub trait Limits: Send + Sync {
    /// The rank of the lowest level that is written (see [`log_files::rank`]).
    fn threshold(&self) -> u16;
    /// How many files stay. 0 keeps all.
    fn max_files(&self) -> u32;
}

impl Limits for SettingsWatch {
    fn threshold(&self) -> u16 {
        self.read(|settings| log_files::rank(&settings.log_level))
    }

    fn max_files(&self) -> u32 {
        self.read(|settings| settings.max_log_files)
    }
}

/// Where the limits come from: nothing until the settings are read.
type Source = Arc<OnceLock<Box<dyn Limits>>>;

fn threshold(source: &Source) -> u16 {
    source
        .get()
        .map_or_else(|| log_files::rank(FIRST_LEVEL), |limits| limits.threshold())
}

fn max_files(source: &Source) -> u32 {
    source
        .get()
        .map_or(FIRST_MAX_FILES, |limits| limits.max_files())
}

/// What goes to the thread of the writer.
enum Message {
    /// A line for the file of a day.
    Line { date: String, text: String },
    /// A request to answer when every line before it is written.
    Flush(SyncSender<()>),
}

/// The handle of the log file, for the server: it gives the writer the settings, and
/// waits for the last lines when the server stops.
pub struct FileLog {
    sender: SyncSender<Message>,
    source: Source,
}

impl FileLog {
    /// Starts the writer for the folder of the log. Returns the handle and the layer that
    /// takes the lines of the log.
    pub fn start(folder: PathBuf) -> (Self, FileLayer) {
        let (sender, receiver) = std::sync::mpsc::sync_channel(QUEUE);
        let source: Source = Arc::new(OnceLock::new());
        let writer = Writer::new(folder, Arc::clone(&source));
        if let Err(error) = std::thread::Builder::new()
            .name("log-file".to_owned())
            .spawn(move || writer.run(&receiver))
        {
            eprintln!("organizr: the log file is not written: the writer cannot start: {error}");
        }
        let layer = FileLayer {
            sender: sender.clone(),
            source: Arc::clone(&source),
        };
        (Self { sender, source }, layer)
    }

    /// From now on, the level and the number of files come from these limits.
    pub fn follow(&self, limits: impl Limits + 'static) {
        if self.source.set(Box::new(limits)).is_err() {
            eprintln!("organizr: the log file follows the settings already");
        }
    }

    /// Waits until the lines so far are on the disk, for at most `wait`.
    pub fn flush(&self, wait: Duration) {
        let (done, finished) = std::sync::mpsc::sync_channel(1);
        if self.sender.send(Message::Flush(done)).is_ok() {
            let _ = finished.recv_timeout(wait);
        }
    }
}

/// Which lines may reach the file: those of this application, at the level `DEBUG` and
/// above. The `logLevel` setting is checked for each line.
pub fn may_reach_the_file(metadata: &Metadata<'_>) -> bool {
    metadata.target().starts_with("organizr") && *metadata.level() <= Level::DEBUG
}

/// Which lines may reach the standard output, on top of `ORGANIZR_LOG`: all but the
/// `TRACE` lines of the FTP library, which write each command to the FTP server, with
/// the password of a login.
pub fn may_reach_the_output(metadata: &Metadata<'_>) -> bool {
    !(metadata.target().starts_with("suppaftp") && *metadata.level() == Level::TRACE)
}

/// The layer of `tracing` that makes a record of each line and hands it to the writer.
pub struct FileLayer {
    sender: SyncSender<Message>,
    source: Source,
}

/// The request id of a span, kept in the extensions of the span.
struct SpanRequestId(String);

/// Finds the field `request_id` of a span.
#[derive(Default)]
struct RequestIdField(Option<String>);

impl Visit for RequestIdField {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "request_id" {
            self.0 = Some(format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "request_id" {
            self.0 = Some(value.to_owned());
        }
    }
}

impl<S> Layer<S> for FileLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: Context<'_, S>) {
        let mut field = RequestIdField::default();
        attributes.record(&mut field);
        if let (Some(request_id), Some(span)) = (field.0, context.span(id)) {
            span.extensions_mut().insert(SpanRequestId(request_id));
        }
    }

    fn on_event(&self, event: &Event<'_>, context: Context<'_, S>) {
        let level = level_name(*event.metadata().level());
        if log_files::rank(level) < threshold(&self.source) {
            return;
        }
        let now = jiff::Timestamp::now();
        let mut record = record_of(event, level, now);
        record.request_id = context.event_scope(event).and_then(|scope| {
            scope.into_iter().find_map(|span| {
                span.extensions()
                    .get::<SpanRequestId>()
                    .map(|id| id.0.clone())
            })
        });
        let Ok(mut text) = serde_json::to_string(&record) else {
            return;
        };
        text.push('\n');
        let date = now.strftime("%Y-%m-%d").to_string();
        match self.sender.try_send(Message::Line { date, text }) {
            Ok(()) | Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {}
        }
    }
}

/// The name of a level of `tracing` in the words of PHP Organizr.
fn level_name(level: Level) -> &'static str {
    match level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARNING",
        Level::INFO => "INFO",
        _ => "DEBUG",
    }
}

/// The part of the application that wrote a line: the last part of its module, for
/// example `Login` for `organizr_api::handlers::login`.
fn channel_of(target: &str) -> String {
    let last = target.rsplit("::").next().unwrap_or(target);
    last.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The record of a line of the log.
fn record_of(event: &Event<'_>, level: &str, now: jiff::Timestamp) -> Record {
    let mut fields = Fields::default();
    event.record(&mut fields);
    let metadata = event.metadata();
    let username = match fields.context.remove("user") {
        Some(Value::String(name)) => name,
        Some(other) => other.to_string(),
        None => "System".to_owned(),
    };
    let remote_ip_address = match fields.context.remove("client") {
        Some(Value::String(address)) => Some(address),
        Some(other) => Some(other.to_string()),
        None => None,
    };
    Record {
        log_level: level.to_owned(),
        message: fields.message,
        channel: channel_of(metadata.target()),
        username,
        trace_id: uuid::Uuid::new_v4().to_string(),
        request_id: None,
        file: metadata.file().unwrap_or_default().replace('\\', "/"),
        line: metadata.line(),
        context: fields.context,
        remote_ip_address,
        datetime: now.strftime("%Y-%m-%d %H:%M:%S%.6f").to_string(),
        timezone: "UTC".to_owned(),
    }
}

/// The fields of a line: the message, and the others for the context.
#[derive(Default)]
struct Fields {
    message: String,
    context: Map<String, Value>,
}

impl Fields {
    fn put(&mut self, field: &Field, value: Value) {
        if field.name() == "message" {
            self.message = match value {
                Value::String(text) => text,
                other => other.to_string(),
            };
        } else {
            self.context.insert(field.name().to_owned(), value);
        }
    }
}

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let mut text = String::new();
        let _ = write!(text, "{value:?}");
        self.put(field, Value::String(text));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.put(field, Value::String(value.to_owned()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.put(field, Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.put(field, Value::from(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.put(field, Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.put(field, Value::Bool(value));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.put(field, Value::String(value.to_string()));
    }
}

/// The thread that writes the lines to the files.
struct Writer {
    folder: PathBuf,
    source: Source,
    /// Whether the last write failed. The failure is told once, until a write works.
    failing: bool,
    /// The day and the number of files of the last pruning.
    pruned: Option<(String, u32)>,
}

impl Writer {
    fn new(folder: PathBuf, source: Source) -> Self {
        Self {
            folder,
            source,
            failing: false,
            pruned: None,
        }
    }

    fn run(mut self, receiver: &Receiver<Message>) {
        while let Ok(first) = receiver.recv() {
            let mut lines = Vec::new();
            let mut flushes = Vec::new();
            let mut next = Some(first);
            while let Some(message) = next {
                match message {
                    Message::Line { date, text } => lines.push((date, text)),
                    Message::Flush(done) => flushes.push(done),
                }
                next = if lines.len() < BATCH {
                    receiver.try_recv().ok()
                } else {
                    None
                };
            }
            self.write(&lines);
            for done in flushes {
                let _ = done.send(());
            }
        }
    }

    /// Writes lines to the files of their days. A failure is told once.
    fn write(&mut self, lines: &[(String, String)]) {
        if lines.is_empty() {
            return;
        }
        match self.try_write(lines) {
            Ok(()) => self.failing = false,
            Err(error) => {
                if !self.failing {
                    eprintln!(
                        "organizr: the log file in {} cannot be written, so the log goes to the \
                         standard output only: {error}",
                        self.folder.display()
                    );
                }
                self.failing = true;
            }
        }
    }

    fn try_write(&mut self, lines: &[(String, String)]) -> io::Result<()> {
        std::fs::create_dir_all(&self.folder)?;
        let mut rest = lines;
        while let Some((date, _)) = rest.first() {
            let same = rest.iter().take_while(|(other, _)| other == date).count();
            let (day, later) = rest.split_at(same);
            let text: String = day.iter().map(|(_, text)| text.as_str()).collect();
            // The file is opened for each batch, so a file that was purged is made anew.
            append(&self.folder.join(log_files::file_name(date)), &text)?;
            self.prune(date)?;
            rest = later;
        }
        Ok(())
    }

    /// Deletes the oldest files over the number, on a new day or a new number.
    fn prune(&mut self, date: &str) -> io::Result<()> {
        let keep = max_files(&self.source);
        let now = (date.to_owned(), keep);
        if self.pruned.as_ref() != Some(&now) {
            log_files::prune(&self.folder, keep)?;
            self.pruned = Some(now);
        }
        Ok(())
    }
}

fn append(path: &Path, text: &str) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
    use std::time::Duration;

    use organizr_api::log_files::{self, Record};
    use tracing_subscriber::Layer as _;
    use tracing_subscriber::filter::filter_fn;
    use tracing_subscriber::layer::SubscriberExt as _;

    use super::{FileLog, Limits, Writer, channel_of, may_reach_the_file, may_reach_the_output};

    /// A writer of the standard output for a test: it keeps what it gets.
    #[derive(Clone, Default)]
    struct Kept(Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for Kept {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("it locks").extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn the_trace_lines_of_the_ftp_library_never_reach_the_output() {
        let kept = Kept::default();
        let writer = kept.clone();
        let layer = tracing_subscriber::fmt::layer()
            .with_writer(move || writer.clone())
            .with_filter(filter_fn(may_reach_the_output));
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!(target: "suppaftp::async_ftp", "CC OUT: PASS the-secret");
            tracing::debug!(target: "suppaftp::async_ftp", "Login OK");
            tracing::trace!(target: "organizr_api", "a line of this application");
        });
        let output = String::from_utf8(kept.0.lock().expect("it locks").clone()).expect("text");
        assert!(!output.contains("the-secret"), "{output}");
        assert!(output.contains("Login OK"), "{output}");
        assert!(output.contains("a line of this application"), "{output}");
    }

    /// Limits that a test can change while the writer runs.
    #[derive(Clone)]
    struct TestLimits(Arc<(AtomicU16, AtomicU32)>);

    impl TestLimits {
        fn new(level: &str, files: u32) -> Self {
            Self(Arc::new((
                AtomicU16::new(log_files::rank(level)),
                AtomicU32::new(files),
            )))
        }
        fn set_level(&self, level: &str) {
            self.0.0.store(log_files::rank(level), Ordering::Relaxed);
        }
    }

    impl Limits for TestLimits {
        fn threshold(&self) -> u16 {
            self.0.0.load(Ordering::Relaxed)
        }
        fn max_files(&self) -> u32 {
            self.0.1.load(Ordering::Relaxed)
        }
    }

    /// Runs `log` with the layer of a log file in `folder`, and waits for the disk.
    fn with_file_log(folder: &Path, limits: Option<TestLimits>, log: impl FnOnce(&FileLog)) {
        let (handle, layer) = FileLog::start(folder.to_owned());
        if let Some(limits) = limits {
            handle.follow(limits);
        }
        let subscriber =
            tracing_subscriber::registry().with(layer.with_filter(filter_fn(may_reach_the_file)));
        tracing::subscriber::with_default(subscriber, || log(&handle));
        handle.flush(Duration::from_secs(10));
    }

    fn records(folder: &Path) -> Vec<Record> {
        let files = log_files::list(folder).expect("the folder can be listed");
        let mut lines = log_files::read_lines(&files).expect("the files can be read");
        lines.reverse();
        lines
            .iter()
            .map(|line| serde_json::from_str(line).expect("each line is a record"))
            .collect()
    }

    #[test]
    fn a_line_becomes_a_record_in_the_file_of_the_day() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        with_file_log(folder.path(), None, |_| {
            tracing::warn!(
                user = "zoe",
                client = "192.0.2.7",
                attempts = 3,
                "a login failed"
            );
        });

        let files = log_files::list(folder.path()).expect("listed");
        assert_eq!(files.len(), 1);
        let today = jiff::Timestamp::now().strftime("%Y-%m-%d").to_string();
        assert_eq!(files[0].name(), format!("organizr-{today}.log"));

        let found = records(folder.path());
        assert_eq!(found.len(), 1);
        let record = &found[0];
        assert_eq!(record.log_level, "WARNING");
        assert_eq!(record.message, "a login failed");
        assert_eq!(record.channel, "Tests");
        assert_eq!(record.username, "zoe");
        assert_eq!(record.remote_ip_address.as_deref(), Some("192.0.2.7"));
        assert_eq!(
            record.context,
            serde_json::json!({"attempts": 3})
                .as_object()
                .cloned()
                .expect("an object")
        );
        assert_eq!(record.trace_id.len(), 36);
        assert!(record.file.ends_with("log_file.rs"), "{}", record.file);
        assert!(record.line.is_some());
        assert_eq!(record.timezone, "UTC");
        // `2026-11-20 03:15:00.123456`
        assert_eq!(record.datetime.len(), 26, "{}", record.datetime);
        assert!(record.datetime.starts_with(&today));

        // The fields of the line come in the order of PHP.
        let text = std::fs::read_to_string(&files[0].path).expect("readable");
        assert!(text.starts_with(r#"{"log_level":"WARNING","message":"a login failed","channel":"Tests","username":"zoe","trace_id":""#), "{text}");
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn a_line_in_the_span_of_a_request_carries_its_id() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        with_file_log(folder.path(), None, |_| {
            tracing::info!("outside");
            let span = tracing::info_span!("request", request_id = %"the-id");
            let _inside = span.enter();
            let inner = tracing::info_span!("inner", other = 1);
            let _deeper = inner.enter();
            tracing::info!("inside");
        });
        let found = records(folder.path());
        assert_eq!(found[0].message, "outside");
        assert_eq!(found[0].request_id, None);
        assert_eq!(found[1].message, "inside");
        assert_eq!(found[1].request_id.as_deref(), Some("the-id"));
        // The field comes next to the trace id, and is `null` outside a request.
        let files = log_files::list(folder.path()).expect("listed");
        let text = std::fs::read_to_string(&files[0].path).expect("readable");
        assert!(text.contains(r#","request_id":null,"file":"#), "{text}");
        assert!(text.contains(r#","request_id":"the-id","file":"#), "{text}");
    }

    #[test]
    fn the_level_of_the_settings_counts_from_the_next_line_on() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let limits = TestLimits::new("INFO", 7);
        with_file_log(folder.path(), Some(limits.clone()), |_| {
            tracing::debug!("left out at INFO");
            tracing::info!("kept at INFO");
            limits.set_level("ERROR");
            tracing::warn!("left out at ERROR");
            tracing::error!("kept at ERROR");
            limits.set_level("DEBUG");
            tracing::debug!("kept at DEBUG");
            tracing::trace!("never kept");
            limits.set_level("NOTICE");
            tracing::info!("left out at NOTICE");
            tracing::warn!("kept at NOTICE");
        });
        let messages: Vec<String> = records(folder.path())
            .into_iter()
            .map(|record| record.message)
            .collect();
        assert_eq!(
            messages,
            [
                "kept at INFO",
                "kept at ERROR",
                "kept at DEBUG",
                "kept at NOTICE"
            ]
        );
    }

    #[test]
    fn lines_of_other_crates_stay_out() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        with_file_log(folder.path(), Some(TestLimits::new("DEBUG", 7)), |_| {
            tracing::warn!(target: "sqlx::query", "select 1");
            tracing::warn!(target: "hyper", "a connection closed");
            tracing::warn!("ours");
        });
        let messages: Vec<String> = records(folder.path())
            .into_iter()
            .map(|record| record.message)
            .collect();
        assert_eq!(messages, ["ours"]);
    }

    #[test]
    fn the_oldest_files_over_the_number_are_deleted() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        for date in ["2026-11-10", "2026-11-11", "2026-11-12", "2026-11-13"] {
            std::fs::write(folder.path().join(log_files::file_name(date)), "{}\n")
                .expect("written");
        }
        let limits = TestLimits::new("INFO", 3);
        let source = Arc::new(std::sync::OnceLock::new());
        let _ = source.set(Box::new(limits.clone()) as Box<dyn Limits>);
        let mut writer = Writer::new(folder.path().to_owned(), source);

        writer.write(&[("2026-11-14".to_owned(), "{\"message\":\"a\"}\n".to_owned())]);
        let dates = || {
            log_files::list(folder.path())
                .expect("listed")
                .into_iter()
                .map(|file| file.date)
                .collect::<Vec<_>>()
        };
        assert_eq!(dates(), ["2026-11-14", "2026-11-13", "2026-11-12"]);

        // A smaller number counts at once, also on the same day.
        limits.0.1.store(1, Ordering::Relaxed);
        writer.write(&[("2026-11-14".to_owned(), "{\"message\":\"b\"}\n".to_owned())]);
        assert_eq!(dates(), ["2026-11-14"]);

        // Lines of two days go to two files; the number 0 keeps every file.
        limits.0.1.store(0, Ordering::Relaxed);
        writer.write(&[
            ("2026-11-14".to_owned(), "{\"message\":\"c\"}\n".to_owned()),
            ("2026-11-15".to_owned(), "{\"message\":\"d\"}\n".to_owned()),
        ]);
        assert_eq!(dates(), ["2026-11-15", "2026-11-14"]);
        let first = std::fs::read_to_string(folder.path().join("organizr-2026-11-14.log"))
            .expect("readable");
        assert_eq!(first.lines().count(), 3);
    }

    #[test]
    fn a_folder_that_cannot_be_written_is_told_once_and_nothing_breaks() {
        let place = tempfile::tempdir().expect("a temporary folder can be made");
        // A file where the folder should be: the folder cannot be made.
        let folder = place.path().join("logs");
        std::fs::write(&folder, "a file").expect("written");
        with_file_log(&folder, None, |_| {
            for _ in 0..3 {
                tracing::error!("goes nowhere");
            }
        });
        assert!(folder.is_file());

        let mut writer = Writer::new(folder.clone(), Arc::new(std::sync::OnceLock::new()));
        writer.write(&[("2026-11-14".to_owned(), "{}\n".to_owned())]);
        assert!(writer.failing);
        std::fs::remove_file(&folder).expect("removed");
        writer.write(&[("2026-11-14".to_owned(), "{}\n".to_owned())]);
        assert!(!writer.failing);
        assert!(folder.join("organizr-2026-11-14.log").is_file());
    }

    #[test]
    fn the_channel_is_the_module_in_words() {
        assert_eq!(channel_of("organizr_api::handlers::login"), "Login");
        assert_eq!(
            channel_of("organizr_api::handlers::connection_tests_more"),
            "Connection Tests More"
        );
        assert_eq!(channel_of("organizr"), "Organizr");
    }
}
