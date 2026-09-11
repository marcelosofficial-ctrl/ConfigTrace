use configtrace_core::{
    DiffReport, FieldChangeKind, FileChangeKind, SnapshotOptions, compare_snapshots, load_snapshot,
    save_snapshot, snapshot_directory,
};
use std::env;
use std::path::{Path, PathBuf};
use std::process;

const VERSION: &str = "0.1.0-dev";

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().skip(1).collect();

    let Some(command) = arguments.first().map(String::as_str) else {
        print_usage();
        return Err("missing command".to_owned());
    };

    match command {
        "snapshot" => command_snapshot(&arguments[1..]),
        "diff" => command_diff(&arguments[1..]),
        "version" | "--version" | "-v" => {
            println!("ConfigTrace {VERSION}");
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown command {other:?}")),
    }
}

fn command_snapshot(arguments: &[String]) -> Result<(), String> {
    if arguments.is_empty() {
        return Err(
            "usage: configtrace snapshot <directory> --out <snapshot.json> [--label <name>]"
                .to_owned(),
        );
    }

    let root = PathBuf::from(&arguments[0]);
    let mut output: Option<PathBuf> = None;
    let mut label: Option<String> = None;

    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--out" => {
                index += 1;
                let Some(value) = arguments.get(index) else {
                    return Err("--out requires a file path".to_owned());
                };
                output = Some(PathBuf::from(value));
            }
            "--label" => {
                index += 1;
                let Some(value) = arguments.get(index) else {
                    return Err("--label requires text".to_owned());
                };
                label = Some(value.clone());
            }
            unknown => return Err(format!("unknown snapshot option {unknown:?}")),
        }

        index += 1;
    }

    let output = output.ok_or_else(|| "--out <snapshot.json> is required".to_owned())?;

    let absolute_output = absolute_path(&output)?;

    let options = SnapshotOptions {
        label,
        excluded_paths: vec![absolute_output.clone()],
        ..SnapshotOptions::default()
    };

    let snapshot = snapshot_directory(&root, &options)?;
    save_snapshot(&absolute_output, &snapshot)?;

    println!(
        "snapshot=PASS files={} warnings={} schema={} out={}",
        snapshot.files.len(),
        snapshot.warnings.len(),
        snapshot.schema_version,
        absolute_output.display()
    );

    Ok(())
}

fn command_diff(arguments: &[String]) -> Result<(), String> {
    if arguments.len() < 2 {
        return Err("usage: configtrace diff <before.json> <after.json> [--json]".to_owned());
    }

    let before_path = PathBuf::from(&arguments[0]);
    let after_path = PathBuf::from(&arguments[1]);
    let json = arguments[2..].iter().any(|argument| argument == "--json");

    for argument in &arguments[2..] {
        if argument != "--json" {
            return Err(format!("unknown diff option {argument:?}"));
        }
    }

    let before = load_snapshot(&before_path)?;
    let after = load_snapshot(&after_path)?;
    let report = compare_snapshots(&before, &after)?;

    if json {
        let text = serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize diff report: {error}"))?;
        println!("{text}");
    } else {
        print_human_diff(&report);
    }

    Ok(())
}

fn print_human_diff(report: &DiffReport) {
    println!(
        "added={} removed={} modified={} unchanged={}",
        report.summary.added,
        report.summary.removed,
        report.summary.modified,
        report.summary.unchanged
    );

    for file in &report.files {
        let change = match file.change {
            FileChangeKind::Added => "ADDED",
            FileChangeKind::Removed => "REMOVED",
            FileChangeKind::Modified => "MODIFIED",
        };

        println!("{change:<8} {}", file.path);

        for field in &file.fields {
            let change = match field.change {
                FieldChangeKind::Added => "+",
                FieldChangeKind::Removed => "-",
                FieldChangeKind::Modified => "~",
            };

            match (&field.before, &field.after) {
                (Some(before), Some(after)) => {
                    println!("  {change} {}: {} -> {}", field.key, before, after);
                }
                (None, Some(after)) => {
                    println!("  {change} {}: {}", field.key, after);
                }
                (Some(before), None) => {
                    println!("  {change} {}: {}", field.key, before);
                }
                (None, None) => {}
            }
        }
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| format!("cannot resolve current directory: {error}"))
}

fn print_usage() {
    println!(
        "ConfigTrace - open configuration change tracing\n\
         \n\
         Usage:\n\
           configtrace snapshot <directory> --out <snapshot.json> [--label <name>]\n\
           configtrace diff <before.json> <after.json> [--json]\n\
           configtrace version\n\
         \n\
         CrashScope integration:\n\
           Use snapshot files plus `diff --json`. The machine contract is schema_version 1."
    );
}
