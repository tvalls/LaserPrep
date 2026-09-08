# Architecture

## Overview

LaserPrep is a Tauri 2 desktop application: a Rust backend (core domain
logic + OS integration) driving a React/TypeScript webview frontend. The
conversion pipeline itself has zero UI dependency and zero AI/ML
dependency (see `docs/adr/0002-no-ai-ml-classification.md`).

```text
src-tauri/     Tauri app shell: commands, window/menu bootstrap, settings
               load, logging init. Thin — orchestration only.
src/           React frontend: components, state (zustand), i18n, preview.
               Never imports crates/* directly — only calls Tauri commands.
crates/
  domain/      Core types (Image, ToneSet, VectorPath, Project, ...).
               No I/O, no dependency on any other crate here.
  analysis/    Heuristic feature extraction + content classification.
  quantize/    N-tone quantization (linear/perceptual/adaptive/histogram).
  vectorize/   Raster→SVG tracing. Wraps `vtracer` behind a `Vectorizer`
               trait — see docs/adr/0003-vectorization-library.md.
  optimize/    Laser path optimization: closing, simplification, merging,
               ordering. See docs/adr/0004-geometry-boolean-ops.md.
  svggen/      SVG document assembly (tone groups, legend, metadata) and
               the SVG/LightBurn-compatibility validator.
  presets/     Named bundles of conversion parameters per category.
  project/     `.lvp` project file format (versioned).
  settings/    Versioned settings.json + migrations.
               See docs/adr/0008-settings-store-custom.md.
```

Dependency direction: `domain` has no internal dependencies; every other
crate under `crates/` depends only on `domain` (and, where relevant, on
each other in one direction: `optimize` depends on `vectorize`'s output
types from `domain`, not on `vectorize` itself). `src-tauri` is the only
crate allowed to depend on all of them plus the Tauri runtime. The React
frontend never touches Rust types directly; it calls Tauri `#[command]`
functions and receives serialized (`serde`) data.

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

## Further reading

- `docs/image-processing.md` — heuristics and pipeline detail (expanded
  in Phase 3 as pipelines are implemented)
- `docs/lightburn-compatibility.md` — SVG compatibility constraints
- `docs/development.md` — local dev setup
- `docs/roadmap.md` — phased delivery plan
- `docs/adr/` — all architecture decision records
