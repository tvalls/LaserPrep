# Roadmap

LaserPrep is developed in phases. Each phase builds on a working,
tested state of the previous one — there is no phase where the
application is left in a partially-fake state (see CLAUDE.md Section 13).

- **Phase 0** — Architecture, CI, repository, initial documentation,
  minimal buildable skeleton. *(done)*
- **Phase 1** — Import, preview, basic processing, quantization, basic
  SVG, export. *(done)*
- **Phase 2** — Advanced vectorization, path closing, simplification,
  noise removal, legend, SVG validator. *(done)*
- **Phase 3** — Heuristic content classification, category-specific
  pipelines, presets. *(done — see docs/image-processing.md for the
  honest scope: classification and presets cover only the categories a
  global image statistic can actually separate; category-specific
  *pipeline* choice is realized as preset parameter differentiation,
  not structurally different code paths)*
- **Phase 4** — Batch processing, `.lvp` projects, undo/history,
  intermediate cache. *(done — the intermediate cache covers decode
  and heuristic classification, the two stages that never depend on
  conversion parameters and were being needlessly repeated on every
  Reconvert/undo/redo; quantization, vectorization, and SVG generation
  still re-run in full on every conversion, which stays fast enough at
  this stage not to need their own caching yet)*
- **Phase 5** — LightBurn path-ordering optimization, Laser Optimization
  Score, advanced preview (before/after, per-tone, LightBurn preview).
  *(current)*
- **Phase 6** — Auto-update, bug reporting, crash reporting, full release
  pipeline.
- **Phase 7** — CLI (`laser-vector convert input.jpg --preset portrait
  --tones 5 --output output.svg`), extensibility for new
  pipelines/presets/algorithms, additional formats/platforms.

See the project's GitHub Issues/Projects for granular, per-phase task
tracking as phases begin.
