# ConfigTrace journal contract v1

The journal is designed for live consumers such as CrashScope.

## Command

```text
configtrace watch <directory> --out <journal.jsonl> [--duration 10s] [--settle 200ms] [--label <name>]
```

Without `--duration`, ConfigTrace watches until the process is stopped.

The journal uses JSON Lines: one complete JSON object per line. Each line is flushed immediately so another process can safely tail the file.

## Architecture

ConfigTrace uses the operating system's recommended filesystem watcher. Raw filesystem activity is not exposed as CrashScope evidence.

Instead:

1. the watcher detects activity;
2. ConfigTrace waits for the configured quiet `settle` period;
3. ConfigTrace takes a new semantic snapshot;
4. it compares that snapshot with the previous baseline;
5. only meaningful configuration changes are written to the journal.

This reduces duplicate/noisy filesystem events and gives CrashScope semantic evidence.

## Common fields

Every journal record contains:

- `schema_version: 1`
- `record_type`
- `observed_unix_ms`
- `root`

`observed_unix_ms` is when ConfigTrace observed the stabilized semantic state. It is not claimed to be the filesystem's exact write timestamp.

## Record types

### session_start

Marks a ready watcher after the initial baseline has been captured.

Additional fields:

- `label`
- `settle_ms`

CrashScope may treat the appearance of this record as "ConfigTrace is ready."

### change

One semantic changed file.

Additional fields:

- `sequence`
- `file`

`file` uses the same `FileChange` contract as snapshot diff JSON, including field-level redaction.

### watch_warning

The underlying watcher reported a recoverable error.

Additional field:

- `message`

### session_end

Written on a normal duration-based end.

Additional fields:

- `changes`
- `batches`
- `warnings`
- `reason`

If the ConfigTrace process is forcibly terminated, consumers must not require a `session_end` line. All prior records are individually flushed and remain valid.

## CrashScope 1.1 integration

Recommended initial flow:

1. CrashScope chooses one or more relevant config roots.
2. It starts one ConfigTrace watcher per root or per selected root group.
3. It waits for `session_start`.
4. It tails JSONL or reads it at incident time.
5. When an incident occurs, CrashScope selects changes within a bounded pre-incident window.
6. CrashScope presents those changes as correlation evidence, not as proof of causation.

Example interpretation:

> HDR changed 18 seconds before this incident.

Good.

> HDR caused this crash.

Not justified by ConfigTrace evidence alone.

This distinction should be preserved in CrashScope UI and incident scoring.