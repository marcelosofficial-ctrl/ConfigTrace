# ConfigTrace 1.0.0

First local release candidate.

## Snapshot / diff

- deterministic directory snapshots
- streaming SHA-256 file fingerprints
- added / removed / modified classification
- JSON field-level comparison
- INI / CFG / CONF / properties field-level comparison
- schema-v1 machine-readable JSON

## Live journal

- native event-driven filesystem watcher
- settle/debounce window
- semantic snapshot comparison
- JSON Lines output
- individually flushed records
- monotonic change sequence numbers
- session start/end records
- watcher-warning records
- schema-v1 journal contract

## Privacy

Sensitive-looking structured values are redacted while preserving a SHA-256 value fingerprint for change detection.

## Integration

ConfigTrace is standalone and CrashScope-independent.

CrashScope 1.1 can initially integrate by launching the executable and consuming its versioned JSON/JSONL contracts. No Rust ABI/FFI dependency is required.

## Validation

- rustfmt: PASS
- Clippy with warnings denied: PASS
- complete Rust test suite: PASS
- idle watcher resource gate: PASS
- active watcher resource gate: PASS
- journal JSONL validity: PASS
- timestamp / sequence ordering: PASS
- secret plaintext leak check: PASS
- clean extracted release smoke tests: PASS