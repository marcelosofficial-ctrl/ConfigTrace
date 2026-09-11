# ConfigTrace machine contract v1

ConfigTrace is designed to be consumed by other programs, including CrashScope.

## Compatibility rule

Every machine-readable ConfigTrace artifact contains:

```json
"schema_version": 1
```

Consumers must reject schema versions they do not understand.

## Snapshot

Create a snapshot:

```text
configtrace snapshot <directory> --out <snapshot.json> [--label <label>]
```

Snapshot rules:

- paths in `files[]` are relative to the selected root;
- snapshot paths use `/` separators;
- `files[]` is lexicographically sorted;
- every regular file has `size` and `sha256`;
- JSON and common key/value configs may also contain `fields`;
- sensitive structured values use `display: "<redacted>"`;
- `value_sha256` supports change detection without exposing the sensitive value;
- `warnings[]` records skipped or unreadable evidence.

## Diff JSON

Create a machine-readable diff:

```text
configtrace diff <before.json> <after.json> --json
```

Top-level fields:

- `schema_version`
- `before_created_unix_ms`
- `after_created_unix_ms`
- `summary`
- `files`

`summary` contains `added`, `removed`, `modified`, and `unchanged`.

Changed files contain:

- `path`
- `change`
- optional before/after SHA-256
- zero or more structured field changes

Field changes contain:

- `key`
- `change`
- optional `before`
- optional `after`
- `sensitive`

## CrashScope integration

CrashScope 1.1 should initially use process isolation:

1. CrashScope owns lifecycle and policy.
2. ConfigTrace remains independently executable and testable.
3. CrashScope chooses the configuration roots.
4. CrashScope invokes ConfigTrace and parses documented schema-v1 JSON.
5. ConfigTrace never requires CrashScope-specific state.

CT-03 extends this contract with an event-driven JSONL journal for timestamped near-crash correlation.