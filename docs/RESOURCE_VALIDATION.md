# ConfigTrace 1.0 resource validation

Validation date: 2026-09-11

These are measurements from one Windows development machine and are not universal performance claims.

## Idle native watcher

Duration: 10 seconds
Semantic changes: 0
Peak working set: 4.3 MB
Peak private memory: 0.8 MB
Total CPU time: 0.016 seconds

Local guardrails:

- peak working set <= 64 MB
- total CPU time <= 1.0 second over this 10-second idle run

Result: PASS

## Active semantic-change session

Duration: 8 seconds
Semantic file-change records: 5
Peak working set: 4.3 MB
Peak private memory: 0.9 MB
Total CPU time: 0 seconds

The test included repeated JSON changes, INI modification/removal, an added CFG file, field-level semantic diffing, SHA-256 work, JSONL serialization, and secret-redaction validation.

Local guardrails:

- peak working set <= 64 MB
- total CPU time <= 2.0 seconds over this 8-second active run

Result: PASS