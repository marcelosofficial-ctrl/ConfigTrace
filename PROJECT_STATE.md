# ConfigTrace Project State

## Purpose

ConfigTrace is an open, standalone configuration-change tracing engine for games and applications.

It is intentionally independent from CrashScope. CrashScope 1.1 can consume ConfigTrace through schema-versioned JSON/JSONL without duplicating its logic or linking directly to Rust ABI.

## Portfolio role

Focused Project demonstrating:

- Rust
- public core-library design
- deterministic filesystem snapshots
- streaming SHA-256
- structured JSON / INI-style parsing
- privacy-aware redaction
- native event-driven filesystem watching
- semantic debouncing
- stable machine-readable integration contracts

## Milestones

CT-01: Rust bootstrap, public core library, snapshot model, CLI. COMPLETE.
CT-02: deterministic diff, structured field changes, secret redaction, snapshot/diff contract. COMPLETE.
CT-03: event-driven timestamped watch/journal mode for near-crash correlation. COMPLETE.
CT-04: resource validation, packaging, final docs, local 1.0 release candidate. NEXT.
CT-1.0: later GitHub/portfolio publication.

## Current commands

configtrace snapshot <directory> --out <snapshot.json> [--label <name>]
configtrace diff <before.json> <after.json>
configtrace diff <before.json> <after.json> --json
configtrace watch <directory> --out <journal.jsonl> [--duration 10s] [--settle 200ms] [--label <name>]
configtrace version

## Machine contracts

Snapshot/diff schema: v1
Document: docs/CONTRACT_V1.md

Journal schema: v1
Document: docs/JOURNAL_CONTRACT_V1.md

## CT-03 journal architecture

ConfigTrace uses the platform-recommended filesystem watcher.

Raw filesystem notifications are treated only as wake-up signals.

After activity, ConfigTrace waits for a short quiet/settle period, takes a semantic snapshot, compares it with the previous baseline, and writes only meaningful file/config changes to JSONL.

This avoids exposing duplicate/noisy raw watcher events to CrashScope.

Journal records:

session_start
change
watch_warning
session_end

Each change record includes a monotonic sequence number and observed_unix_ms timestamp.

## CrashScope 1.1 boundary

Recommended first integration:

1. CrashScope selects relevant game/application config roots.
2. CrashScope launches ConfigTrace as a child process.
3. CrashScope waits for session_start.
4. ConfigTrace records semantic JSONL changes during the monitored session.
5. At incident time CrashScope reads a bounded pre-incident window.
6. CrashScope presents nearby config changes as correlation evidence.

CrashScope must not claim that a nearby ConfigTrace event caused a crash without separate evidence.

## Privacy

Sensitive-looking values remain <redacted> in snapshots, diffs, and journal entries.

A SHA-256 value fingerprint is retained internally in snapshot state so ConfigTrace can detect a secret changed without recording its plaintext.

## Validation

Validated: 2026-09-11 19:23:21 +09:00
rustfmt: PASS
clippy -D warnings: PASS
Tests: 16 PASS
Release build: PASS
CT-02 snapshot/diff regression: PASS
Native watch session_start: PASS
Semantic watch journal: PASS
Added-file watch event: PASS
Removed-file watch event: PASS
JSON field-level watch diff: PASS
Journal secret redaction: PASS
Monotonic change sequence: PASS
Snapshot contract v1: PASS
Journal contract v1: PASS
Executable size: 532.5 KiB

## GitHub status

Local development only.
No GitHub operations or GitHub Actions were used.

## Next milestone

CT-04:
- measure idle watcher CPU/memory
- measure change-processing overhead
- validate long-running journal behavior
- package standalone Windows x64 release
- clean-extraction validation
- freeze local 1.0 candidate