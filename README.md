# ConfigTrace

ConfigTrace is an open, standalone configuration-change tracing engine for games and applications.

It can be used by itself or as a diagnostic evidence source for another program such as CrashScope.

## What it does

ConfigTrace can:

- snapshot a configuration directory;
- fingerprint regular files with streaming SHA-256;
- compare snapshots deterministically;
- show field-level JSON and INI/CFG/CONF/property changes;
- redact sensitive-looking values while still detecting that they changed;
- watch a configuration tree using the platform's native filesystem notification mechanism;
- convert noisy raw filesystem activity into a timestamped semantic JSONL journal.

## CLI

```powershell
configtrace snapshot "C:\path\to\config" --out before.json --label session-start
configtrace snapshot "C:\path\to\config" --out after.json --label crash-time

configtrace diff before.json after.json
configtrace diff before.json after.json --json

configtrace watch "C:\path\to\config" --out session.jsonl --settle 200ms --label game-session

configtrace version
```

`watch` runs until stopped unless `--duration` is supplied.

## Machine contracts

Snapshot and diff JSON use schema version 1:

`docs/CONTRACT_V1.md`

Live journal JSONL uses schema version 1:

`docs/JOURNAL_CONTRACT_V1.md`

## CrashScope integration

The recommended first CrashScope integration is process isolation:

1. CrashScope selects the relevant configuration root.
2. CrashScope launches `configtrace watch`.
3. CrashScope waits for the `session_start` JSONL record.
4. ConfigTrace records semantic changes throughout the session.
5. At incident time CrashScope reads a bounded pre-incident window.
6. CrashScope presents nearby changes as correlation evidence.

ConfigTrace deliberately does not claim that a nearby setting change caused a crash.

## Privacy

Potentially sensitive structured values are represented as:

```text
<redacted>
```

ConfigTrace uses an internal SHA-256 value fingerprint so it can detect that a secret changed without storing or emitting the plaintext secret.

## Parsing

Field-level:

- JSON
- INI
- CFG
- CONF
- properties

Whole-file fingerprint:

- other text
- binary / unknown files

## Resource philosophy

ConfigTrace's live watcher is event-driven. It does not continuously rescan configuration trees on a fixed high-frequency polling loop.

Raw filesystem notifications are only wake-up signals. After a short quiet period ConfigTrace creates a semantic snapshot and records meaningful changes.

See `docs/RESOURCE_VALIDATION.md` for the local 1.0 validation measurements.

## Safety

- source configuration files are read only;
- symlinks are skipped;
- `.git` and `target` trees are ignored;
- no accounts;
- no telemetry;
- no network service;
- no registry modification;
- no Windows service installation;
- no administrator privileges required.

## Build

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
```

## License

MIT