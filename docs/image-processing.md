# Image processing

> Stub — expanded as Phases 1-3 implement each pipeline stage. See
> `docs/architecture.md` for the current conceptual pipeline and
> `docs/adr/` for the standing technical decisions (no ML, `vtracer`
> usage, geometry libraries).

Planned sections (filled in as each lands):

- Heuristic feature extraction (`crates/analysis`): histogram, edge
  density, color clustering, saturation/hue, uniform-background ratio,
  symmetry, aspect ratio — and the weighted scoring model per content
  category.
- Tone quantization strategies (`crates/quantize`): linear, perceptual,
  adaptive, histogram-based, per-region; manual threshold editing.
- Per-category pipelines (`crates/vectorize` + category-specific
  parameter sets in `crates/presets`): portrait, animal, logo,
  drawing/illustration, landscape, generic photograph.
