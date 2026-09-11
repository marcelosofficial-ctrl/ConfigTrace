use crate::model::{FileChange, SCHEMA_VERSION, SnapshotOptions};
use crate::{compare_snapshots, snapshot_directory};
use notify::{Event, RecursiveMode, Watcher};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct WatchOptions {
    pub label: Option<String>,
    pub settle: Duration,
    pub duration: Option<Duration>,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            label: None,
            settle: Duration::from_millis(200),
            duration: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchResult {
    pub changes: u64,
    pub batches: u64,
    pub warnings: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
pub enum JournalRecord {
    SessionStart {
        schema_version: u32,
        observed_unix_ms: u128,
        root: String,
        label: Option<String>,
        settle_ms: u64,
    },
    Change {
        schema_version: u32,
        sequence: u64,
        observed_unix_ms: u128,
        root: String,
        file: FileChange,
    },
    WatchWarning {
        schema_version: u32,
        observed_unix_ms: u128,
        root: String,
        message: String,
    },
    SessionEnd {
        schema_version: u32,
        observed_unix_ms: u128,
        root: String,
        changes: u64,
        batches: u64,
        warnings: u64,
        reason: String,
    },
}

pub fn watch_directory(
    root: &Path,
    journal_path: &Path,
    options: &WatchOptions,
) -> Result<WatchResult, String> {
    if options.settle.is_zero() {
        return Err("settle duration must be greater than zero".to_owned());
    }

    if options.duration.is_some_and(|duration| duration.is_zero()) {
        return Err("watch duration must be greater than zero".to_owned());
    }

    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve root {}: {error}", root.display()))?;

    if !canonical_root.is_dir() {
        return Err(format!("root is not a directory: {}", root.display()));
    }

    let journal_path = absolute_output_path(journal_path)?;

    let journal_file = File::create(&journal_path)
        .map_err(|error| format!("cannot create journal {}: {error}", journal_path.display()))?;

    let mut writer = BufWriter::new(journal_file);

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)
        .map_err(|error| format!("cannot create filesystem watcher: {error}"))?;

    watcher
        .watch(&canonical_root, RecursiveMode::Recursive)
        .map_err(|error| format!("cannot watch {}: {error}", canonical_root.display()))?;

    let snapshot_options = SnapshotOptions {
        label: options.label.clone(),
        excluded_paths: vec![journal_path.clone()],
        ..SnapshotOptions::default()
    };

    let mut baseline = snapshot_directory(&canonical_root, &snapshot_options)?;

    while rx.try_recv().is_ok() {}

    let root_text = baseline.root.clone();

    write_record(
        &mut writer,
        &JournalRecord::SessionStart {
            schema_version: SCHEMA_VERSION,
            observed_unix_ms: now_unix_ms(),
            root: root_text.clone(),
            label: options.label.clone(),
            settle_ms: duration_ms(options.settle),
        },
    )?;

    let started = Instant::now();
    let mut changes = 0_u64;
    let mut batches = 0_u64;
    let mut warnings = 0_u64;
    let mut sequence = 0_u64;
    let reason = loop {
        if options
            .duration
            .is_some_and(|duration| started.elapsed() >= duration)
        {
            break "duration_elapsed".to_owned();
        }

        let wait = next_wait(started, options.duration);

        match rx.recv_timeout(wait) {
            Ok(Ok(event)) => {
                if event_is_only_journal(&event, &journal_path) {
                    continue;
                }

                settle_events(
                    &rx,
                    options.settle,
                    &journal_path,
                    &mut writer,
                    &root_text,
                    &mut warnings,
                )?;

                let after = snapshot_directory(&canonical_root, &snapshot_options)?;
                let report = compare_snapshots(&baseline, &after)?;

                if !report.files.is_empty() {
                    batches += 1;
                    let observed_unix_ms = now_unix_ms();

                    for file in report.files {
                        sequence += 1;
                        changes += 1;

                        write_record(
                            &mut writer,
                            &JournalRecord::Change {
                                schema_version: SCHEMA_VERSION,
                                sequence,
                                observed_unix_ms,
                                root: root_text.clone(),
                                file,
                            },
                        )?;
                    }
                }

                baseline = after;
            }
            Ok(Err(error)) => {
                warnings += 1;

                write_record(
                    &mut writer,
                    &JournalRecord::WatchWarning {
                        schema_version: SCHEMA_VERSION,
                        observed_unix_ms: now_unix_ms(),
                        root: root_text.clone(),
                        message: error.to_string(),
                    },
                )?;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                break "watcher_disconnected".to_owned();
            }
        }
    };

    write_record(
        &mut writer,
        &JournalRecord::SessionEnd {
            schema_version: SCHEMA_VERSION,
            observed_unix_ms: now_unix_ms(),
            root: root_text,
            changes,
            batches,
            warnings,
            reason,
        },
    )?;

    drop(watcher);

    Ok(WatchResult {
        changes,
        batches,
        warnings,
    })
}

fn settle_events(
    rx: &mpsc::Receiver<notify::Result<Event>>,
    settle: Duration,
    journal_path: &Path,
    writer: &mut BufWriter<File>,
    root: &str,
    warnings: &mut u64,
) -> Result<(), String> {
    loop {
        match rx.recv_timeout(settle) {
            Ok(Ok(event)) => {
                if event_is_only_journal(&event, journal_path) {
                    continue;
                }
            }
            Ok(Err(error)) => {
                *warnings += 1;

                write_record(
                    writer,
                    &JournalRecord::WatchWarning {
                        schema_version: SCHEMA_VERSION,
                        observed_unix_ms: now_unix_ms(),
                        root: root.to_owned(),
                        message: error.to_string(),
                    },
                )?;
            }
            Err(RecvTimeoutError::Timeout) => return Ok(()),
            Err(RecvTimeoutError::Disconnected) => {
                return Err("filesystem watcher disconnected during settle".to_owned());
            }
        }
    }
}

fn event_is_only_journal(event: &Event, journal_path: &Path) -> bool {
    !event.paths.is_empty()
        && event
            .paths
            .iter()
            .all(|path| paths_equivalent(path, journal_path))
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    let left = normalize_path(left);
    let right = normalize_path(right);

    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn normalize_path(path: &Path) -> String {
    if path.is_absolute() {
        path.to_string_lossy().replace('\\', "/")
    } else {
        match std::env::current_dir() {
            Ok(current) => current.join(path).to_string_lossy().replace('\\', "/"),
            Err(_) => path.to_string_lossy().replace('\\', "/"),
        }
    }
}

fn absolute_output_path(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("journal path has no file name: {}", path.display()))?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "cannot create journal directory {}: {error}",
            parent.display()
        )
    })?;

    let canonical_parent = parent.canonicalize().map_err(|error| {
        format!(
            "cannot resolve journal directory {}: {error}",
            parent.display()
        )
    })?;

    Ok(canonical_parent.join(file_name))
}

fn write_record(writer: &mut BufWriter<File>, record: &JournalRecord) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, record)
        .map_err(|error| format!("cannot serialize journal record: {error}"))?;

    writer
        .write_all(b"\n")
        .map_err(|error| format!("cannot write journal newline: {error}"))?;

    writer
        .flush()
        .map_err(|error| format!("cannot flush journal: {error}"))
}

fn next_wait(started: Instant, duration: Option<Duration>) -> Duration {
    let heartbeat = Duration::from_millis(250);

    match duration {
        Some(limit) => limit.saturating_sub(started.elapsed()).min(heartbeat),
        None => heartbeat,
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FieldChange, FieldChangeKind, FileChangeKind};

    #[test]
    fn duration_is_capped_to_u64() {
        assert_eq!(duration_ms(Duration::from_millis(250)), 250);
    }

    #[test]
    fn change_record_serializes_contract_fields() {
        let record = JournalRecord::Change {
            schema_version: 1,
            sequence: 7,
            observed_unix_ms: 123,
            root: "C:/game".to_owned(),
            file: FileChange {
                path: "settings.json".to_owned(),
                change: FileChangeKind::Modified,
                before_sha256: Some("before".to_owned()),
                after_sha256: Some("after".to_owned()),
                fields: vec![FieldChange {
                    key: "/graphics/hdr".to_owned(),
                    change: FieldChangeKind::Modified,
                    before: Some("false".to_owned()),
                    after: Some("true".to_owned()),
                    sensitive: false,
                }],
            },
        };

        let json = serde_json::to_value(record).unwrap();

        assert_eq!(json["record_type"], "change");
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["sequence"], 7);
        assert_eq!(json["file"]["path"], "settings.json");
    }

    #[test]
    fn sensitive_change_serialization_remains_redacted() {
        let record = JournalRecord::Change {
            schema_version: 1,
            sequence: 1,
            observed_unix_ms: 123,
            root: "C:/game".to_owned(),
            file: FileChange {
                path: "account.ini".to_owned(),
                change: FileChangeKind::Modified,
                before_sha256: Some("before".to_owned()),
                after_sha256: Some("after".to_owned()),
                fields: vec![FieldChange {
                    key: "Account.token".to_owned(),
                    change: FieldChangeKind::Modified,
                    before: Some("<redacted>".to_owned()),
                    after: Some("<redacted>".to_owned()),
                    sensitive: true,
                }],
            },
        };

        let json = serde_json::to_string(&record).unwrap();

        assert!(json.contains("<redacted>"));
        assert!(!json.contains("secret"));
    }
}
