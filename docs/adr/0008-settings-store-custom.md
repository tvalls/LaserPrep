# ADR 0008: Settings persistence is a hand-written versioned store, not tauri-plugin-store

## Status

Accepted

## Context

The auto-ci skill's Standard 7 requires versioned JSON settings with
explicit, sequential, backward-compatible migrations, atomic writes,
corruption recovery, and forward-version handling. `tauri-plugin-store` is
a convenient key-value wrapper around a JSON file, but it does not provide
schema versioning or migration semantics — those would have to be built on
top of it either way.

## Decision

Implement settings persistence directly in `crates/settings`:

- a `schemaVersion` integer field, sequential `migrate_vN_to_vN+1`
  functions per Standard 7
- atomic writes (write to a temp file in the same directory, then
  rename/replace)
- a `settings.json.bak` backup before destructive migrations
- corrupted-file recovery path that preserves the corrupt file for
  diagnosis and falls back to defaults
- forward-version handling (newer `schemaVersion` than the running app
  understands → safe read-only/default mode, never a destructive rewrite)

`tauri-plugin-store` is not a dependency of this project.

## Consequences

- Slightly more code to own than wrapping a plugin, but full control over
  migration correctness, which is directly tested per Standard 7's
  required test list (loading current schema, migrating each prior
  schema, missing optional fields, unknown-field preservation, corrupt
  JSON, newer-than-supported schema).
- `crates/settings` has no Tauri dependency itself — it is plain Rust,
  testable without the app shell, and only wired into Tauri commands from
  `src-tauri`.
