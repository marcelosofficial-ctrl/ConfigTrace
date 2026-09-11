# ConfigTrace Project State

## Purpose

Open, standalone configuration-change tracing for games and applications.

ConfigTrace is being designed as an independent Rust core plus CLI so CrashScope 1.1 can consume it through a stable JSON contract without duplicating configuration-tracking logic.

## Portfolio role

Focused Project demonstrating:

- Rust
- public library/API design
- deterministic filesystem snapshots
- streaming hashing
- structured parsing
- privacy-aware redaction
- stable machine-readable contracts
- integration-oriented architecture

## Roadmap

CT-01: Rust bootstrap, public core library, snapshot model, CLI. IN PROGRESS.
CT-02: deterministic snapshot comparison, structured field diffs, secret redaction, integration contract. IN PROGRESS.
CT-03: timestamped watch/journal mode for near-crash change correlation.
CT-04: resource validation, packaging, docs, local 1.0 release candidate.
CT-1.0: later GitHub/portfolio publication.

## CrashScope boundary

ConfigTrace must remain usable without CrashScope.

CrashScope should initially integrate through the ConfigTrace CLI and schema-versioned JSON rather than link directly to Rust ABI.

This gives CrashScope:
- process isolation
- replaceable ConfigTrace versions
- simple C# JSON parsing
- independent ConfigTrace tests
- an easy future path to tighter integration if justified

## Privacy rule

Potential secrets are never emitted as plaintext in structured snapshot fields. They are represented as `<redacted>` with a SHA-256 fingerprint for change detection.

## GitHub policy

Local development only for now.
No GitHub operations or Actions during development.