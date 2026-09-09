# ADR 0009: CLI argument parsing (`clap`) and additional raster formats

## Status

Accepted

## Context

`docs/roadmap.md` Phase 7 calls for a command-line front end
(`laser-vector convert input.jpg --preset portrait --tones 5 --output
output.svg`) alongside the desktop app, plus broader raster format
support and documented extensibility for new pipelines/presets/
algorithms.

The CLI needs an argument-parsing approach. Options considered:

- **`clap`** (with the `derive` feature) — the de facto standard Rust CLI
  argument parser, MIT/Apache-2.0 dual-licensed, actively maintained,
  Windows-compatible, generates `--help`/`--version` and validation for
  free from a plain struct definition.
- Hand-rolled argument parsing over `std::env::args()` — no new
  dependency, but reimplements flag parsing, `--help` text, and error
  messages that `clap` already gets right, for a tool whose whole
  purpose is a clean CLI surface.

For raster formats, the pipeline already decodes through the `image`
crate (ADR-adjacent to vectorization, not separately recorded before
now); only PNG and JPEG were enabled via Cargo features. `image` itself
already supports BMP, GIF, TIFF, and WebP decoding as pure-Rust,
non-experimental features.

## Decision

- Use `clap` (`derive` feature) for `crates/cli`. It is not used
  anywhere else in the workspace — the desktop app has no CLI surface,
  and library crates (`crates/pipeline`, `crates/presets`, etc.) stay
  free of a CLI-parsing dependency so they remain usable from any front
  end, per CLAUDE.md Section 5.
- Enable the `image` crate's `bmp`, `gif`, `tiff`, and `webp` features
  in the workspace `Cargo.toml` alongside the existing `png`/`jpeg`.
  `laserprep_imaging::load_rgb_from_bytes`/`guess_mime_type` needed no
  structural change — format detection is already content-based
  (`image::guess_format`/`image::load_from_memory`), not an
  extension/MIME allowlist maintained by this codebase. Adding a format
  `image` already supports is a one-line feature addition plus a
  `guess_mime_type` match arm, not a new decoder to write or a new ADR
  each time.
- Document the pipeline's existing extension points (the
  `laserprep_vectorize::Vectorizer` trait from ADR 0003, and
  `laserprep_presets::ALL_PRESETS` as a data table rather than a
  hardcoded switch) in `docs/architecture.md` instead of introducing a
  new plugin/registry abstraction. Nothing in the current codebase
  needs one yet, and CLAUDE.md Section 13 rejects speculative
  abstractions built ahead of an actual second implementation.

## Consequences

- `clap` is a direct dependency of `crates/cli` only.
- Supported input formats are now PNG, JPEG, BMP, GIF, TIFF, and WebP,
  auto-detected from file content in both the desktop app and the CLI.
- Animated GIF/TIFF multi-frame input decodes only the first frame
  (`image`'s default `load_from_memory` behavior) — acceptable since
  LaserPrep converts a single still image per run; not documented as a
  limitation beyond this ADR unless a user report shows it is
  surprising in practice.
- Adding a genuinely new extension point (a second `Vectorizer`
  implementation, a plugin-loaded preset source) is still a real design
  decision when it happens, and should get its own ADR then — this one
  only records that the trait/data-table seams already exist and where
  they are documented.
