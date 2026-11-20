//! The log of Organizr in the data folder: one file of JSON lines for each day.
//!
//! PHP Organizr writes its log with Monolog: the file `logs/organizr-<date>.log` in its data
//! folder, one JSON object on each line, a new file each day, and at most `maxLogFiles`
//! files. This application keeps the same files. The server writes them; the routes
//! `GET /log` and `DELETE /log` read and delete them.
//!
//! The date of a file and the time of a record are in UTC.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The name of the folder of the log in the data folder.
pub const FOLDER: &str = "logs";

/// What the name of a log file starts with.
const PREFIX: &str = "organizr-";

/// What the name of a log file ends with.
const SUFFIX: &str = ".log";

/// The folder of the log files in a data folder.
pub fn folder(data_dir: &Path) -> PathBuf {
    data_dir.join(FOLDER)
}

/// The name of the log file of a day, as Monolog names it: `organizr-2026-11-20.log`.
pub fn file_name(date: &str) -> String {
    format!("{PREFIX}{date}{SUFFIX}")
}

/// The day of a log file, `2026-11-20`, when the name is that of a log file.
pub fn date_of(name: &str) -> Option<&str> {
    let date = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let bytes = date.as_bytes();
    let shape = bytes.len() == 10
        && bytes.iter().enumerate().all(|(place, byte)| {
            if place == 4 || place == 7 {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        });
    shape.then_some(date)
}

/// A log file in the folder of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFile {
    /// The day of the file, `2026-11-20`.
    pub date: String,
    /// The path of the file.
    pub path: PathBuf,
}

impl LogFile {
    /// The name of the file.
    pub fn name(&self) -> String {
        file_name(&self.date)
    }
}

/// The log files in a folder, the newest first. A folder that does not exist has none.
/// Other files in the folder are left out.
pub fn list(folder: &Path) -> io::Result<Vec<LogFile>> {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(date) = date_of(&name)
            && entry.file_type()?.is_file()
        {
            files.push(LogFile {
                date: date.to_owned(),
                path: entry.path(),
            });
        }
    }
    files.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(files)
}

/// Deletes the oldest log files, so that at most `keep` stay. With `keep` 0, every file
/// stays, as with Monolog. Returns the days of the deleted files.
pub fn prune(folder: &Path, keep: u32) -> io::Result<Vec<String>> {
    if keep == 0 {
        return Ok(Vec::new());
    }
    let keep = usize::try_from(keep).unwrap_or(usize::MAX);
    let mut deleted = Vec::new();
    for file in list(folder)?.into_iter().skip(keep) {
        match std::fs::remove_file(&file.path) {
            Ok(()) => deleted.push(file.date),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(deleted)
}

/// The levels of a record, in the names and the order of PHP Organizr (Monolog).
pub const LEVELS: [&str; 8] = [
    "DEBUG",
    "INFO",
    "NOTICE",
    "WARNING",
    "ERROR",
    "CRITICAL",
    "ALERT",
    "EMERGENCY",
];

/// The rank of a level of Monolog: a record is written when its rank is at least the
/// rank of the `logLevel` setting.
pub fn rank(level: &str) -> u16 {
    match level {
        "DEBUG" => 100,
        "INFO" => 200,
        "NOTICE" => 250,
        "ERROR" => 400,
        "CRITICAL" => 500,
        "ALERT" => 550,
        "EMERGENCY" => 600,
        // PHP Organizr takes `WARNING` for `WARNING` and for every unknown value.
        _ => 300,
    }
}

/// One record of the log, with the fields of the records of PHP Organizr.
///
/// PHP also writes `server_ip_address`, `user_agent` and `process_time`; this
/// application does not. `trace_id` is a new random id for each record, as in PHP;
/// `request_id` is the id of the request (D-13).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    /// The level: one of [`LEVELS`].
    pub log_level: String,
    /// What happened.
    pub message: String,
    /// The part of the application that wrote the record. PHP calls it the function.
    pub channel: String,
    /// The user the record is about, or `System`.
    pub username: String,
    /// The id of the record.
    pub trace_id: String,
    /// The id of the request that wrote the record, as the header `X-Request-Id` of its
    /// answer names it, or `None` outside a request. PHP has no such field.
    pub request_id: Option<String>,
    /// The source file that wrote the record.
    pub file: String,
    /// The line in that file.
    pub line: Option<u32>,
    /// The other fields of the record.
    pub context: Map<String, Value>,
    /// The address of the client, when the record names one.
    pub remote_ip_address: Option<String>,
    /// The time, as `2026-11-20 03:15:00.123456`.
    pub datetime: String,
    /// The time zone of `datetime`: always `UTC`.
    pub timezone: String,
}

/// The value of the `filter` query of `GET /log`, as PHP reads it: a level, or `None`
/// for every record. `NONE` is every record; an unknown value is `DEBUG`.
pub fn filter_of(raw: &str) -> Option<&'static str> {
    let upper = raw.to_ascii_uppercase();
    if upper == "NONE" {
        return None;
    }
    Some(
        LEVELS
            .iter()
            .find(|level| **level == upper)
            .copied()
            .unwrap_or("DEBUG"),
    )
}

/// The lines of the log files, each file from its last line to its first, in the order
/// of the files. With the files of [`list`], the newest record comes first.
pub fn read_lines(files: &[LogFile]) -> io::Result<Vec<String>> {
    let mut lines = Vec::new();
    for file in files {
        let text = match std::fs::read(&file.path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        lines.extend(
            text.lines()
                .rev()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_owned),
        );
    }
    Ok(lines)
}

/// The fields of a line that a search needs.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Probe {
    log_level: String,
    trace_id: String,
}

fn probe(line: &str) -> Probe {
    serde_json::from_str(line).unwrap_or_default()
}

/// A record as the answer shows it: `None`, written as `null`, for a line that is not a
/// record, as PHP's `json_decode` gives.
fn record_of(line: &str) -> Option<Record> {
    serde_json::from_str(line).ok()
}

/// The records of one level, or all of them, from `offset` on, at most `size` of them.
/// Returns the number of records that match and the records of the page.
pub fn page(
    lines: &[String],
    level: Option<&str>,
    offset: usize,
    size: usize,
) -> (usize, Vec<Option<Record>>) {
    let matching: Vec<&String> = match level {
        None => lines.iter().collect(),
        Some(level) => lines
            .iter()
            .filter(|line| probe(line).log_level.eq_ignore_ascii_case(level))
            .collect(),
    };
    let records = matching
        .iter()
        .skip(offset)
        .take(size)
        .map(|line| record_of(line))
        .collect();
    (matching.len(), records)
}

/// The record with a trace id, in any letter case.
pub fn find_trace(lines: &[String], trace_id: &str) -> Option<Option<Record>> {
    lines
        .iter()
        .find(|line| probe(line).trace_id.eq_ignore_ascii_case(trace_id))
        .map(|line| record_of(line))
}

#[cfg(test)]
mod tests {
    use super::{LogFile, date_of, file_name, filter_of, find_trace, list, page, prune, rank};

    #[test]
    fn a_file_has_the_name_that_monolog_gives_it() {
        assert_eq!(file_name("2026-11-20"), "organizr-2026-11-20.log");
        assert_eq!(date_of("organizr-2026-11-20.log"), Some("2026-11-20"));
        for name in [
            "organizr.log",
            "organizr-2026-11-2.log",
            "organizr-2026_11_20.log",
            "organizr-2026-11-20.log.gz",
            "other-2026-11-20.log",
            "organizr-../../x.log",
        ] {
            assert_eq!(date_of(name), None, "{name}");
        }
    }

    #[test]
    fn the_files_are_listed_newest_first_and_pruned_to_the_number() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        for date in ["2026-11-18", "2026-11-20", "2026-11-19", "2026-11-17"] {
            std::fs::write(folder.path().join(file_name(date)), "{}\n").expect("written");
        }
        std::fs::write(folder.path().join("notes.txt"), "kept").expect("written");
        let dates =
            |files: Vec<LogFile>| files.into_iter().map(|file| file.date).collect::<Vec<_>>();
        assert_eq!(
            dates(list(folder.path()).expect("listed")),
            ["2026-11-20", "2026-11-19", "2026-11-18", "2026-11-17"]
        );

        assert_eq!(
            prune(folder.path(), 0).expect("pruned"),
            Vec::<String>::new()
        );
        assert_eq!(
            prune(folder.path(), 2).expect("pruned"),
            ["2026-11-18", "2026-11-17"]
        );
        assert_eq!(
            dates(list(folder.path()).expect("listed")),
            ["2026-11-20", "2026-11-19"]
        );
        assert!(folder.path().join("notes.txt").exists());
        assert_eq!(
            list(&folder.path().join("missing")).expect("listed"),
            Vec::new()
        );
    }

    #[test]
    fn levels_rank_as_in_monolog() {
        assert!(rank("DEBUG") < rank("INFO"));
        assert!(rank("INFO") < rank("NOTICE"));
        assert!(rank("NOTICE") < rank("WARNING"));
        assert!(rank("WARNING") < rank("ERROR"));
        assert!(rank("ERROR") < rank("EMERGENCY"));
        assert_eq!(rank("nonsense"), rank("WARNING"));
    }

    #[test]
    fn the_filter_falls_back_as_in_php() {
        assert_eq!(filter_of("NONE"), None);
        assert_eq!(filter_of("none"), None);
        assert_eq!(filter_of("warning"), Some("WARNING"));
        assert_eq!(filter_of("loud"), Some("DEBUG"));
        assert_eq!(filter_of(""), Some("DEBUG"));
    }

    fn line(level: &str, trace: &str) -> String {
        format!(r#"{{"log_level":"{level}","message":"m {trace}","trace_id":"{trace}"}}"#)
    }

    #[test]
    fn a_page_counts_the_matching_records() {
        let lines = vec![
            line("INFO", "a"),
            line("ERROR", "b"),
            "not json".to_owned(),
            line("INFO", "c"),
        ];
        let (total, records) = page(&lines, None, 1, 2);
        assert_eq!(total, 4);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].as_ref().expect("a record").trace_id, "b");
        assert!(records[1].is_none());

        let (total, records) = page(&lines, Some("INFO"), 0, 10);
        assert_eq!(total, 2);
        let third = records[1].as_ref().expect("a record");
        assert_eq!(third.trace_id, "c");
        assert!(third.context.is_empty());

        assert_eq!(page(&lines, Some("ALERT"), 0, 10).0, 0);
        assert_eq!(page(&lines, None, 9, 10).1.len(), 0);
    }

    #[test]
    fn a_trace_id_finds_its_record() {
        let lines = vec![line("INFO", "AbC"), line("ERROR", "def")];
        let found = find_trace(&lines, "abc").flatten().expect("found");
        assert_eq!(found.message, "m AbC");
        assert_eq!(find_trace(&lines, "zzz"), None);
    }
}
