use crate::model::{
    DiffReport, DiffSummary, FieldChange, FieldChangeKind, FileChange, FileChangeKind, FileRecord,
    SCHEMA_VERSION, Snapshot, StructuredField,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn compare_snapshots(before: &Snapshot, after: &Snapshot) -> Result<DiffReport, String> {
    validate_snapshot(before)?;
    validate_snapshot(after)?;

    let before_map: BTreeMap<&str, &FileRecord> = before
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();

    let after_map: BTreeMap<&str, &FileRecord> = after
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();

    let paths: BTreeSet<&str> = before_map
        .keys()
        .copied()
        .chain(after_map.keys().copied())
        .collect();

    let mut summary = DiffSummary {
        added: 0,
        removed: 0,
        modified: 0,
        unchanged: 0,
    };

    let mut files = Vec::new();

    for path in paths {
        match (before_map.get(path), after_map.get(path)) {
            (None, Some(after_file)) => {
                summary.added += 1;
                files.push(FileChange {
                    path: path.to_owned(),
                    change: FileChangeKind::Added,
                    before_sha256: None,
                    after_sha256: Some(after_file.sha256.clone()),
                    fields: added_fields(&after_file.fields),
                });
            }
            (Some(before_file), None) => {
                summary.removed += 1;
                files.push(FileChange {
                    path: path.to_owned(),
                    change: FileChangeKind::Removed,
                    before_sha256: Some(before_file.sha256.clone()),
                    after_sha256: None,
                    fields: removed_fields(&before_file.fields),
                });
            }
            (Some(before_file), Some(after_file)) if before_file.sha256 != after_file.sha256 => {
                summary.modified += 1;
                files.push(FileChange {
                    path: path.to_owned(),
                    change: FileChangeKind::Modified,
                    before_sha256: Some(before_file.sha256.clone()),
                    after_sha256: Some(after_file.sha256.clone()),
                    fields: compare_fields(&before_file.fields, &after_file.fields),
                });
            }
            (Some(_), Some(_)) => {
                summary.unchanged += 1;
            }
            (None, None) => unreachable!("path came from one of the maps"),
        }
    }

    Ok(DiffReport {
        schema_version: SCHEMA_VERSION,
        before_created_unix_ms: before.created_unix_ms,
        after_created_unix_ms: after.created_unix_ms,
        summary,
        files,
    })
}

fn validate_snapshot(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported snapshot schema {}, expected {}",
            snapshot.schema_version, SCHEMA_VERSION
        ));
    }

    Ok(())
}

fn compare_fields(
    before: &BTreeMap<String, StructuredField>,
    after: &BTreeMap<String, StructuredField>,
) -> Vec<FieldChange> {
    let keys: BTreeSet<&str> = before
        .keys()
        .map(String::as_str)
        .chain(after.keys().map(String::as_str))
        .collect();

    let mut changes = Vec::new();

    for key in keys {
        match (before.get(key), after.get(key)) {
            (None, Some(after_field)) => changes.push(FieldChange {
                key: key.to_owned(),
                change: FieldChangeKind::Added,
                before: None,
                after: Some(after_field.display.clone()),
                sensitive: after_field.sensitive,
            }),
            (Some(before_field), None) => changes.push(FieldChange {
                key: key.to_owned(),
                change: FieldChangeKind::Removed,
                before: Some(before_field.display.clone()),
                after: None,
                sensitive: before_field.sensitive,
            }),
            (Some(before_field), Some(after_field))
                if before_field.value_sha256 != after_field.value_sha256 =>
            {
                changes.push(FieldChange {
                    key: key.to_owned(),
                    change: FieldChangeKind::Modified,
                    before: Some(before_field.display.clone()),
                    after: Some(after_field.display.clone()),
                    sensitive: before_field.sensitive || after_field.sensitive,
                });
            }
            _ => {}
        }
    }

    changes
}

fn added_fields(fields: &BTreeMap<String, StructuredField>) -> Vec<FieldChange> {
    fields
        .iter()
        .map(|(key, field)| FieldChange {
            key: key.clone(),
            change: FieldChangeKind::Added,
            before: None,
            after: Some(field.display.clone()),
            sensitive: field.sensitive,
        })
        .collect()
}

fn removed_fields(fields: &BTreeMap<String, StructuredField>) -> Vec<FieldChange> {
    fields
        .iter()
        .map(|(key, field)| FieldChange {
            key: key.clone(),
            change: FieldChangeKind::Removed,
            before: Some(field.display.clone()),
            after: None,
            sensitive: field.sensitive,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FileKind, Snapshot};
    use std::collections::BTreeMap;

    fn field(display: &str, hash: &str, sensitive: bool) -> StructuredField {
        StructuredField {
            display: display.to_owned(),
            value_sha256: hash.to_owned(),
            sensitive,
        }
    }

    fn snapshot(files: Vec<FileRecord>) -> Snapshot {
        Snapshot {
            schema_version: SCHEMA_VERSION,
            created_unix_ms: 1,
            root: "C:/test".to_owned(),
            label: None,
            files,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn detects_added_removed_modified_and_unchanged_files() {
        let before = snapshot(vec![
            FileRecord {
                path: "removed.ini".to_owned(),
                size: 1,
                sha256: "a".to_owned(),
                kind: FileKind::KeyValue,
                fields: BTreeMap::new(),
            },
            FileRecord {
                path: "modified.ini".to_owned(),
                size: 1,
                sha256: "b".to_owned(),
                kind: FileKind::KeyValue,
                fields: BTreeMap::new(),
            },
            FileRecord {
                path: "same.ini".to_owned(),
                size: 1,
                sha256: "same".to_owned(),
                kind: FileKind::KeyValue,
                fields: BTreeMap::new(),
            },
        ]);

        let after = snapshot(vec![
            FileRecord {
                path: "added.ini".to_owned(),
                size: 1,
                sha256: "d".to_owned(),
                kind: FileKind::KeyValue,
                fields: BTreeMap::new(),
            },
            FileRecord {
                path: "modified.ini".to_owned(),
                size: 2,
                sha256: "c".to_owned(),
                kind: FileKind::KeyValue,
                fields: BTreeMap::new(),
            },
            FileRecord {
                path: "same.ini".to_owned(),
                size: 1,
                sha256: "same".to_owned(),
                kind: FileKind::KeyValue,
                fields: BTreeMap::new(),
            },
        ]);

        let report = compare_snapshots(&before, &after).unwrap();

        assert_eq!(report.summary.added, 1);
        assert_eq!(report.summary.removed, 1);
        assert_eq!(report.summary.modified, 1);
        assert_eq!(report.summary.unchanged, 1);
        assert_eq!(report.files.len(), 3);
    }

    #[test]
    fn reports_structured_field_change() {
        let before = snapshot(vec![FileRecord {
            path: "settings.json".to_owned(),
            size: 1,
            sha256: "old".to_owned(),
            kind: FileKind::Json,
            fields: BTreeMap::from([(
                "/graphics/hdr".to_owned(),
                field("false", "old-hash", false),
            )]),
        }]);

        let after = snapshot(vec![FileRecord {
            path: "settings.json".to_owned(),
            size: 1,
            sha256: "new".to_owned(),
            kind: FileKind::Json,
            fields: BTreeMap::from([(
                "/graphics/hdr".to_owned(),
                field("true", "new-hash", false),
            )]),
        }]);

        let report = compare_snapshots(&before, &after).unwrap();
        let change = &report.files[0].fields[0];

        assert_eq!(change.key, "/graphics/hdr");
        assert_eq!(change.before.as_deref(), Some("false"));
        assert_eq!(change.after.as_deref(), Some("true"));
    }

    #[test]
    fn sensitive_field_change_keeps_values_redacted() {
        let before = snapshot(vec![FileRecord {
            path: "account.ini".to_owned(),
            size: 1,
            sha256: "old".to_owned(),
            kind: FileKind::KeyValue,
            fields: BTreeMap::from([(
                "Account.password".to_owned(),
                field("<redacted>", "hash-one", true),
            )]),
        }]);

        let after = snapshot(vec![FileRecord {
            path: "account.ini".to_owned(),
            size: 1,
            sha256: "new".to_owned(),
            kind: FileKind::KeyValue,
            fields: BTreeMap::from([(
                "Account.password".to_owned(),
                field("<redacted>", "hash-two", true),
            )]),
        }]);

        let report = compare_snapshots(&before, &after).unwrap();
        let change = &report.files[0].fields[0];

        assert!(change.sensitive);
        assert_eq!(change.before.as_deref(), Some("<redacted>"));
        assert_eq!(change.after.as_deref(), Some("<redacted>"));
    }
}
