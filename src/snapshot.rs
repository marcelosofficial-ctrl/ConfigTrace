use crate::model::{FileRecord, SCHEMA_VERSION, Snapshot, SnapshotOptions};
use crate::structured::{classify, parse_structured};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn snapshot_directory(root: &Path, options: &SnapshotOptions) -> Result<Snapshot, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve root {}: {error}", root.display()))?;

    if !canonical_root.is_dir() {
        return Err(format!("root is not a directory: {}", root.display()));
    }

    let excluded: Vec<PathBuf> = options
        .excluded_paths
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .collect();

    let mut files = Vec::new();
    let mut warnings = Vec::new();

    walk(
        &canonical_root,
        &canonical_root,
        &excluded,
        options.max_structured_bytes,
        &mut files,
        &mut warnings,
    )?;

    files.sort_by(|left, right| left.path.cmp(&right.path));
    warnings.sort();

    Ok(Snapshot {
        schema_version: SCHEMA_VERSION,
        created_unix_ms: now_unix_ms(),
        root: normalize_path(&canonical_root),
        label: options.label.clone(),
        files,
        warnings,
    })
}

pub fn save_snapshot(path: &Path, snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "cannot save schema version {}, expected {}",
            snapshot.schema_version, SCHEMA_VERSION
        ));
    }

    let bytes = serde_json::to_vec_pretty(snapshot)
        .map_err(|error| format!("cannot serialize snapshot: {error}"))?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;

    let temporary = path.with_extension("tmp");

    fs::write(&temporary, bytes)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;

    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("cannot replace {}: {error}", path.display()))?;
    }

    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "cannot commit snapshot {} -> {}: {error}",
            temporary.display(),
            path.display()
        )
    })?;

    Ok(())
}

pub fn load_snapshot(path: &Path) -> Result<Snapshot, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;

    let snapshot: Snapshot = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid snapshot {}: {error}", path.display()))?;

    if snapshot.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported snapshot schema {}, expected {}",
            snapshot.schema_version, SCHEMA_VERSION
        ));
    }

    Ok(snapshot)
}

fn walk(
    root: &Path,
    directory: &Path,
    excluded: &[PathBuf],
    max_structured_bytes: u64,
    files: &mut Vec<FileRecord>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot read directory {}: {error}", directory.display()))?;

    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!(
                    "directory entry error in {}: {error}",
                    directory.display()
                ));
                continue;
            }
        };

        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if file_name == ".git" || file_name == "target" {
            continue;
        }

        if excluded.contains(&path) {
            continue;
        }

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                warnings.push(format!("metadata error {}: {error}", path.display()));
                continue;
            }
        };

        if metadata.file_type().is_symlink() {
            warnings.push(format!("skipped symlink: {}", relative(root, &path)));
            continue;
        }

        if metadata.is_dir() {
            walk(root, &path, excluded, max_structured_bytes, files, warnings)?;
            continue;
        }

        if !metadata.is_file() {
            continue;
        }

        match record_file(root, &path, metadata.len(), max_structured_bytes) {
            Ok(record) => files.push(record),
            Err(error) => warnings.push(error),
        }
    }

    Ok(())
}

fn record_file(
    root: &Path,
    path: &Path,
    size: u64,
    max_structured_bytes: u64,
) -> Result<FileRecord, String> {
    let sha256 = hash_file(path)?;

    let parse_bytes = if size <= max_structured_bytes {
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?
    } else {
        Vec::new()
    };

    let kind = if parse_bytes.is_empty() && size > 0 {
        extension_kind(path)
    } else {
        classify(path, &parse_bytes)
    };

    let fields = if size <= max_structured_bytes {
        parse_structured(&kind, &parse_bytes).unwrap_or_default()
    } else {
        Default::default()
    };

    Ok(FileRecord {
        path: relative(root, path),
        size,
        sha256,
        kind,
        fields,
    })
}

fn extension_kind(path: &Path) -> crate::model::FileKind {
    use crate::model::FileKind;

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "json" => FileKind::Json,
        "ini" | "cfg" | "conf" | "properties" => FileKind::KeyValue,
        "txt" | "xml" | "yaml" | "yml" | "toml" | "log" => FileKind::Text,
        _ => FileKind::Binary,
    }
}

fn hash_file(path: &Path) -> Result<String, String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("cannot hash {}: {error}", path.display()))?;

        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(normalize_path)
        .unwrap_or_else(|_| normalize_path(path))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
    use tempfile::tempdir;

    #[test]
    fn snapshot_sorts_paths_and_skips_target_and_git() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("b.ini"), "b=2").unwrap();
        fs::write(dir.path().join("a.ini"), "a=1").unwrap();
        fs::create_dir(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("target").join("ignored.ini"), "x=1").unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git").join("ignored.ini"), "x=1").unwrap();

        let snapshot = snapshot_directory(dir.path(), &SnapshotOptions::default()).unwrap();

        let paths: Vec<_> = snapshot
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(paths, vec!["a.ini", "b.ini"]);
    }

    #[test]
    fn excluded_output_is_not_captured() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("snapshot.json");
        fs::write(&output, "{}").unwrap();
        fs::write(dir.path().join("settings.ini"), "quality=high").unwrap();

        let options = SnapshotOptions {
            excluded_paths: vec![output],
            ..SnapshotOptions::default()
        };

        let snapshot = snapshot_directory(dir.path(), &options).unwrap();

        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.files[0].path, "settings.ini");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("settings.json"), r#"{"hdr":true}"#).unwrap();

        let snapshot = snapshot_directory(dir.path(), &SnapshotOptions::default()).unwrap();
        let path = dir.path().join("out").join("snapshot.json");

        save_snapshot(&path, &snapshot).unwrap();
        let loaded = load_snapshot(&path).unwrap();

        assert_eq!(loaded, snapshot);
    }
}
