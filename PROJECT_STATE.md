# ConfigTrace Project State

## Purpose

ConfigTrace is an open, standalone configuration-change tracing engine for games and applications.

It is independent from CrashScope. CrashScope 1.1 can consume ConfigTrace through stable schema-versioned JSON/JSONL contracts without duplicating its logic or linking directly to Rust ABI.

## Portfolio role

Focused Project demonstrating:

- Rust
- public core-library design
- deterministic filesystem snapshots
- streaming SHA-256
- structured configuration parsing
- privacy-aware redaction
- native event-driven filesystem watching
- semantic debounce / stabilization
- JSONL event journaling
- integration contract design
- resource-conscious background tooling
- release engineering

## Milestones

CT-01: Rust bootstrap, public core library, snapshot model, CLI. COMPLETE.
CT-02: deterministic diff, field-level structured changes, secret redaction, snapshot/diff contract. COMPLETE.
CT-03: timestamped event-driven semantic journal. COMPLETE.
CT-04: performance validation, documentation, standalone packaging, clean extraction validation. COMPLETE.
CT-1.0: GitHub publication and portfolio integration. DEFERRED.

## Version

1.0.0 local release candidate

## Current commands

configtrace snapshot <directory> --out <snapshot.json> [--label <name>]
configtrace diff <before.json> <after.json>
configtrace diff <before.json> <after.json> --json
configtrace watch <directory> --out <journal.jsonl> [--duration 10s] [--settle 200ms] [--label <name>]
configtrace version

## Contracts

Snapshot/diff schema: v1
docs/CONTRACT_V1.md

Journal schema: v1
docs/JOURNAL_CONTRACT_V1.md

## CrashScope 1.1 boundary

Recommended initial integration remains process-isolated:

1. CrashScope selects relevant configuration roots.
2. CrashScope launches ConfigTrace.
3. CrashScope waits for session_start.
4. ConfigTrace writes semantic JSONL evidence.
5. CrashScope correlates nearby change records with an incident.
6. CrashScope presents changes as correlation evidence, not proof of causation.

## Privacy

Potentially sensitive structured values are never emitted as plaintext.

ConfigTrace stores <redacted> in serialized structured evidence while preserving SHA-256 value fingerprints in snapshot state for change detection.

The CT-04 active and clean-package tests explicitly scanned output for injected secret strings and found none.

## Validation

Validated: 2026-09-11 19:44:57 +09:00
Authoritative pre-CT04 commit: 103daf1
Version: 1.0.0
rustfmt: PASS
Clippy -D warnings: PASS
Tests: 16 PASS

Idle watcher:
- duration: 10 s
- peak working set: 4.3 MB
- peak private memory: 0.8 MB
- CPU time: 0.016 s
- semantic changes: 0

Active watcher:
- duration: 8 s
- semantic changes: 5
- peak working set: 4.3 MB
- peak private memory: 0.9 MB
- CPU time: 0 s

Journal:
- JSONL parse: PASS
- session lifecycle: PASS
- monotonic timestamps: PASS
- contiguous sequence numbers: PASS
- semantic added/removed/modified evidence: PASS
- secret plaintext leak check: PASS

Release:
- executable size: 532.5 KiB
- ZIP: dist/release/ConfigTrace-1.0.0-win-x64.zip
- ZIP size: 264 KiB
- ZIP SHA-256: 60f20f240657af20bc1ba6a5d9489962e6f34801d4ec38b812b321e0cd89e4c9
- clean extraction version smoke: PASS
- clean extraction snapshot/diff smoke: PASS
- clean extraction watcher smoke: PASS
- clean extraction secret-leak smoke: PASS

## GitHub status

Development remains local.
No GitHub operations or GitHub Actions were used.

## Freeze

ConfigTrace 1.0 is feature-frozen after CT-04.

The next ConfigTrace work should be publication/preflight or a future post-1.0 milestone driven by a real CrashScope integration requirement. Do not expand 1.0 simply to add features.