# ConfigTrace

ConfigTrace is an open configuration-change tracing engine and CLI.

It is intentionally independent from any one application so other tools can consume it through a stable machine-readable contract. The first intended integration is CrashScope 1.1.

## Why

A crash report becomes much more useful when it can answer questions such as:

- Did a game rewrite its graphics configuration shortly before the crash?
- Did HDR, resolution, renderer, or quality settings change?
- Was a config file added, removed, or modified?
- Which structured settings changed between two points in time?

ConfigTrace captures lightweight snapshots and compares them deterministically.

## Commands

```powershell
configtrace snapshot "C:\path\to\config" --out before.json --label "session-start"
configtrace snapshot "C:\path\to\config" --out after.json --label "crash-time"

configtrace diff before.json after.json
configtrace diff before.json after.json --json
```

## Open integration contract

ConfigTrace 0.1 uses `schema_version: 1`.

The Rust library exposes the core snapshot and comparison functions. The CLI is only a thin adapter around that core.

CrashScope can therefore begin integration safely by:

1. invoking `configtrace.exe`;
2. storing snapshot JSON locally;
3. parsing `configtrace diff ... --json`;
4. correlating ConfigTrace timestamps and changes with CrashScope incidents.

No native DLL/FFI dependency is required for the first integration.

## Privacy

ConfigTrace never uploads data.

Structured settings with sensitive-looking names such as passwords, tokens, authorization values, cookies, and session IDs are redacted. ConfigTrace retains a SHA-256 fingerprint of the value so it can detect that the secret changed without recording the secret itself.

## Current parsing

- JSON: structured field-level comparison
- INI / CFG / CONF / properties: key/value field-level comparison
- other text: whole-file fingerprint
- binary / unknown: whole-file fingerprint

All files receive a streaming SHA-256 fingerprint.

## Safety

- selected directories only
- read-only source scanning
- symlinks are skipped
- `.git` and `target` directories are ignored
- snapshot output can live inside the scanned tree without snapshotting itself
- snapshots are committed through a temporary file
- no accounts
- no telemetry
- no registry modification
- no service installation