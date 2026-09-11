use configtrace_core::{FileChangeKind, SnapshotOptions, compare_snapshots, snapshot_directory};
use std::fs;
use tempfile::tempdir;

#[test]
fn real_directory_snapshot_detects_json_and_ini_changes() {
    let dir = tempdir().unwrap();

    fs::write(
        dir.path().join("graphics.json"),
        r#"{"video":{"hdr":false,"width":1920},"account":{"token":"first"}}"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("game.ini"),
        "[Graphics]\nQuality=High\nVSync=true\n",
    )
    .unwrap();

    let before = snapshot_directory(dir.path(), &SnapshotOptions::default()).unwrap();

    fs::write(
        dir.path().join("graphics.json"),
        r#"{"video":{"hdr":true,"width":2560},"account":{"token":"second"}}"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("game.ini"),
        "[Graphics]\nQuality=Ultra\nVSync=true\n",
    )
    .unwrap();

    fs::write(dir.path().join("new.cfg"), "render_scale=1.25\n").unwrap();

    let after = snapshot_directory(dir.path(), &SnapshotOptions::default()).unwrap();
    let report = compare_snapshots(&before, &after).unwrap();

    assert_eq!(report.summary.added, 1);
    assert_eq!(report.summary.modified, 2);

    let graphics = report
        .files
        .iter()
        .find(|file| file.path == "graphics.json")
        .unwrap();

    assert_eq!(graphics.change, FileChangeKind::Modified);

    let hdr = graphics
        .fields
        .iter()
        .find(|field| field.key == "/video/hdr")
        .unwrap();

    assert_eq!(hdr.before.as_deref(), Some("false"));
    assert_eq!(hdr.after.as_deref(), Some("true"));

    let token = graphics
        .fields
        .iter()
        .find(|field| field.key == "/account/token")
        .unwrap();

    assert!(token.sensitive);
    assert_eq!(token.before.as_deref(), Some("<redacted>"));
    assert_eq!(token.after.as_deref(), Some("<redacted>"));
}
