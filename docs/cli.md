# CLI Guide

`ophi` stores an investigation in a local SQLite file. The database defaults to `ophi.db` in the current directory. Use `--db` explicitly when moving between sessions; its parent directory must already exist.

## Install

From the repository root, with Rust stable and a C compiler installed:

```bash
cargo install --path crates/ophi-cli --locked
```

For development, `cargo run --bin ophi -- --help` runs the same CLI without installing it.

## Create and retrieve a Case

```bash
ophi --db ./ophi.db case create --title "Investigate a conjecture"
```

Alternatively, supply JSON on stdin:

```bash
printf '%s' '{"title":"Investigate a conjecture"}' | ophi --db ./ophi.db case create
```

Copy `data.id` from the result. Retrieve the Case with:

```bash
ophi --db ./ophi.db case get <case-id>
```

## Append an Entry

Replace `<case-id>` with the returned ID:

```bash
ophi --db ./ophi.db entry append <<'JSON'
{
  "case_id": "<case-id>",
  "kind": "observation",
  "content": "The calculation depends on the choice of orientation.",
  "author": "human",
  "references": []
}
JSON
```

Core generates the record's `id`, `created_at`, and `schema_version`. Do not include them in the request. `references` is optional; all other fields shown are required.

## Link a correction to its original

Append another Entry with an appropriate reference:

```json
{
  "case_id": "<case-id>",
  "kind": "correction",
  "content": "State the orientation convention before comparing the signs.",
  "author": "human",
  "references": [{"target_id":"<original-entry-id>","relation":"supersedes"}]
}
```

The target must exist in the same Case. The original remains available with `entry get`; `supersedes` is an explicit relationship, not an instruction to replace data.

## Retrieve and paginate history

```bash
ophi --db ./ophi.db entry get <entry-id>
ophi --db ./ophi.db case history <case-id> --limit 2
```

The response contains `data.entries` and `data.next_cursor`. If the cursor is non-null, pass it to the next query:

```bash
ophi --db ./ophi.db case history <case-id> --limit 2 --after <next-cursor>
```

Results follow append order, not timestamp order. Cursors belong to the original Case and database. See the [protocol specification](../spec/v0.1.md) for pagination behavior when new records arrive.

## Use the JSON interface

Successful commands write one JSON line to stdout with `protocol_version: 1` and a `data` field. Failures write a structured `error` to stderr and return a nonzero exit code. Help and version output are plain text.

A subprocess integration should pass JSON through stdin, parse stdout only on success, and inspect stderr and the exit code on failure. It should use an argument array rather than building a shell command from user input.

There is no TypeScript SDK yet. The CLI protocol is the intended first integration boundary. The [Casson invariant example](../examples/mathematics/casson_invariant.py) demonstrates this using Python's standard library.

## Storage and limits

Each write is atomic. Multiple local processes may use the same database; SQLite serializes writes and waits up to five seconds for a lock. The CLI exposes no update or delete operation.

Requests and serialized records are each limited to 1 MiB. Unknown JSON fields are rejected. Field lengths, reference counts, error codes, and schema versions are specified in [v0.1](../spec/v0.1.md).

Keep the database file and its IDs for later sessions. Use SQLite-aware backups or close all writers before copying the database. JSON export/import and migration commands are not yet available.
