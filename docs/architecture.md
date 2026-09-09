# Architecture

## Overview

LaserPrep is a Tauri 2 desktop application: a Rust backend (core domain
logic + OS integration) driving a React/TypeScript webview frontend. The
conversion pipeline itself has zero UI dependency and zero AI/ML
dependency (see `docs/adr/0002-no-ai-ml-classification.md`).

```text
src-tauri/     Tauri app shell: commands, window/menu bootstrap, settings
               load, logging init. Calls into crates/pipeline for actual
               conversion — no pipeline algorithms live here. Thin —
               orchestration only.
src/           React frontend: components, state (zustand), i18n, preview.
               Never imports crates/* directly — only calls Tauri commands.
crates/
  domain/      Core types (Image, ToneSet, VectorPath, Project, ...).
               No I/O, no dependency on any other crate here.
  imaging/     Raster decoding to grayscale luminance/RGB, with a maximum
               working-dimension downscale (import + pre-processing).
               Format support (PNG/JPEG/BMP/GIF/TIFF/WebP) is content-
               detected, not tied to a file extension allowlist — see
               docs/adr/0009-cli-and-additional-formats.md.
  analysis/    Heuristic feature extraction + content classification.
  quantize/    N-tone quantization (linear/perceptual/adaptive/histogram).
  vectorize/   Raster→SVG tracing. Wraps `vtracer` behind a `Vectorizer`
               trait — see docs/adr/0003-vectorization-library.md.
  optimize/    Laser path optimization: closing, simplification, merging,
               ordering. See docs/adr/0004-geometry-boolean-ops.md.
  svggen/      SVG document assembly (tone groups, legend, metadata) and
               the SVG/LightBurn-compatibility validator.
  presets/     Named bundles of conversion parameters per category
               (`ALL_PRESETS`, a plain data table — see "Extensibility"
               below).
  pipeline/    Orchestrates import → classify → quantize → vectorize →
               optimize → SVG generation → validate into one call per
               conversion. No UI dependency at all — both `src-tauri`
               and `crates/cli` depend on this crate, not on each other.
  cli/         `laser-vector`: the command-line front end (`docs/roadmap.md`
               Phase 7). Depends on `pipeline` and `presets` only.
  project/     `.lvp` project file format (versioned).
  settings/    Versioned settings.json + migrations.
               See docs/adr/0008-settings-store-custom.md.
```

Dependency direction: `domain` has no internal dependencies; every other
crate under `crates/` depends only on `domain` (and, where relevant, on
each other in one direction: `optimize` depends on `vectorize`'s output
types from `domain`, not on `vectorize` itself; `pipeline` depends on
`analysis`/`imaging`/`quantize`/`vectorize`/`optimize`/`svggen`/`presets`,
and `cli` depends only on `pipeline`/`presets`). `src-tauri` and
`crates/cli` are the two UI-shell crates — each is a thin front end over
`crates/pipeline`, never the other way around. The React frontend never
touches Rust types directly; it calls Tauri `#[command]` functions and
receives serialized (`serde`) data.

## Conceptual pipeline

```text
Import → Heuristic analysis → Rule-based classification → Pipeline choice
  → Pre-processing → Tone quantization → Segmentation → Vectorization
  → Path closing → Geometric optimization → Path ordering
  → SVG generation → Legend generation → Validation → Preview → Export
```

Each arrow is a boundary between crates (roughly: analysis → quantize →
vectorize → optimize → svggen), so each stage is independently testable
and independently swappable.

## Why no ML anywhere

See `docs/adr/0002-no-ai-ml-classification.md`. This is a product
constraint, not a temporary technical limitation.

## Extensibility

`docs/roadmap.md` Phase 7 asks for extensibility "for new
pipelines/presets/algorithms". The seams below already exist and are
where each kind of extension attaches — none of them required a new
abstraction to add:

- **New preset**: add a `pub const` to `crates/presets/src/lib.rs` and
  list it in `ALL_PRESETS`. Every caller (the desktop UI's preset
  picker, `laser-vector presets`, `laser-vector convert --preset`)
  reads that table — nothing else to register it in.
- **New vectorization backend/algorithm**: implement
  `laserprep_vectorize::Vectorizer` (see
  `docs/adr/0003-vectorization-library.md`). Callers in
  `crates/pipeline` depend on the trait, not on `vtracer` directly, so
  a second implementation can be swapped in per tone or per category
  without touching `quantize`/`optimize`/`svggen`.
- **New content category / classification rule**: add a variant to
  `laserprep_analysis::ContentCategory` and a rule in `classify()`; map
  it to a preset in `laserprep_presets::suggest_preset`. See the module
  doc comment on `ContentCategory` for why some CLAUDE.md Section 6
  categories (portrait vs. animal vs. drawing) are deliberately not
  auto-detected yet — nothing in a global image statistic can tell them
  apart without real subject recognition, which CLAUDE.md Section 2
  forbids.
- **New input raster format**: enable the `image` crate's feature for
  it in the workspace `Cargo.toml` and add a `guess_mime_type` match
  arm in `crates/imaging`. See
  `docs/adr/0009-cli-and-additional-formats.md`.
- **New front end**: depend on `crates/pipeline` directly, the way
  `src-tauri` and `crates/cli` both do. It has no UI dependency of any
  kind (CLAUDE.md Section 5).

Nothing here is a plugin system or a runtime-loaded registry — every
extension point above is a compile-time Rust seam (a trait, an enum, a
`const` table). That is a deliberate choice, not a gap: CLAUDE.md
Section 13 rejects abstractions built ahead of an actual second
implementation, and a compiled, single-vendor desktop app has no
requirement (unlike a plugin-hosting platform) to load third-party code
at runtime.

## Further reading

- `docs/image-processing.md` — heuristics and pipeline detail (expanded
  in Phase 3 as pipelines are implemented)
- `docs/lightburn-compatibility.md` — SVG compatibility constraints
- `docs/development.md` — local dev setup
- `docs/roadmap.md` — phased delivery plan
- `docs/adr/` — all architecture decision records
