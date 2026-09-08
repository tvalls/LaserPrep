# ADR 0003: Raster-to-SVG tracing uses vtracer, isolated behind a trait

## Status

Accepted

## Context

The Rust ecosystem has few mature raster-to-vector tracing libraries
compared to, say, Python or C (potrace/Autotrace). We need contour
tracing plus Bezier curve fitting, callable as a library (not a CLI
subprocess), for each per-tone binary mask produced by
`crates/quantize`.

Options considered:

- **`vtracer`** (visioncortex/vtracer) — pure Rust, MIT license, actively
  maintained, exposes both a CLI and a library API, supports a binary
  color mode suited to per-tone masks.
- Writing our own contour tracing (Moore-neighbor / marching squares) and
  Bezier fitting from scratch.
- Shelling out to `potrace` (C binary) as an external process.

## Decision

Use `vtracer` as a library dependency, called in its binary-tracing mode
once per tone mask (see ADR 0002 in the sense that this, too, is a
deterministic classical algorithm — contour tracing and curve fitting are
not machine learning).

`vtracer` is never called directly outside `crates/vectorize`. That crate
exposes our own `Vectorizer` trait; all call sites depend on the trait,
not on `vtracer` directly. This keeps the option open to swap or
supplement the tracing backend later without touching
`quantize`/`optimize`/`svggen`/the Tauri commands.

Rejected: shelling out to `potrace` — it would violate the "no runtime
dependency the end user must install" constraint (Section 4 of
CLAUDE.md) unless statically vendored, and mixing a C dependency into an
otherwise pure-Rust build complicates the Windows build/signing story for
no clear benefit over `vtracer`.

## Consequences

- `crates/vectorize/Cargo.toml` is the only place `vtracer` appears as a
  direct dependency.
- If `vtracer` proves insufficient for a specific pipeline (e.g., portrait
  contour continuity), the fix is either an isolated pre/post-processing
  step in `crates/vectorize` or `crates/optimize`, not a fork of the
  classification/no-AI boundary.
