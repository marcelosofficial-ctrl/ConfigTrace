# ConfigTrace Project State

## Purpose

ConfigTrace is an open, standalone configuration-change tracing engine for games and applications.

It is independent from CrashScope. CrashScope 1.2 can consume ConfigTrace 1.0.1 through stable schema-versioned JSON/JSONL contracts without duplicating its logic or linking directly to Rust ABI.

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
CT-1.0: GitHub publication and portfolio integration. COMPLETE.
CT-1.0.1: sensitive-key redaction maintenance release driven by CrashScope integration. COMPLETE.

## Version

1.0.1 public release

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

## CrashScope 1.2 boundary

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
Authoritative 1.0.1 public commit: b629c970dfc14fca5df1e0ef2b0d1d07d0d8c56c
Version: 1.0.1
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
- public release: https://github.com/marcelosofficial-ctrl/ConfigTrace/releases/tag/v1.0.1
- ZIP: ConfigTrace-1.0.1-win-x64.zip
- ZIP SHA-256: 9374128c42e3bf3e1fd8f48bdddd7485e421da543c80f54d454985e1bcccbafe
- clean extraction version smoke: PASS
- clean extraction snapshot/diff smoke: PASS
- clean extraction watcher smoke: PASS
- clean extraction secret-leak smoke: PASS

## GitHub status

Public repository: https://github.com/marcelosofficial-ctrl/ConfigTrace
Current public release: v1.0.1
Portfolio case study: https://marcelosofficial-ctrl.github.io/portfolio/projects/configtrace/

ConfigTrace 1.0.1 is publicly released and remains independently usable as a standalone utility.

## Current maintenance state

ConfigTrace 1.0.1 is released. The 1.0.1 maintenance release fixed camelCase/PascalCase sensitive-key recognition discovered through real CrashScope integration.

Future ConfigTrace work should be driven by a concrete standalone or integration requirement. Keep the process-isolated schema-versioned boundary stable.
