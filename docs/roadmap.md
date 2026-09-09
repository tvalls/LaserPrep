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
  *(done — "LightBurn preview" is scoped as adopting LightBurn's own
  layer-color convention (distinct stroke colors per tone) for visual
  clarity in our own preview, not a literal emulation of LightBurn's
  renderer)*
- **Phase 6** — Auto-update, bug reporting, crash reporting, full release
  pipeline. *(done — signed builds, checksums, GitHub Release
  publication, and a signed updater manifest (`latest.json`) are all
  in place; Rust panics and frontend rendering crashes are both
  captured to the local diagnostic log. Not done, and out of scope for
  what this project can set up on its own: Authenticode-signing the
  installer/binaries themselves needs a purchased code-signing
  certificate — a business decision for the maintainer, not an
  engineering gap)*
- **Phase 7** — CLI (`laser-vector convert input.jpg --preset portrait
  --tones 5 --output output.svg`), extensibility for new
  pipelines/presets/algorithms, additional formats/platforms. *(done —
  `crates/cli` ships `laser-vector convert`/`laser-vector presets`,
  depending only on the UI-agnostic `crates/pipeline` extracted from
  `src-tauri` for this purpose; input formats expanded from PNG/JPEG to
  also include BMP, GIF, TIFF, and WebP (`docs/adr/0009-cli-and-additional-formats.md`);
  extensibility is documented in `docs/architecture.md`'s
  "Extensibility" section against the trait/data-table seams that
  already existed (`Vectorizer`, `ALL_PRESETS`, `ContentCategory`)
  rather than a new plugin system nothing in this codebase needs yet.
  "Additional platforms" is honestly scoped like Phase 6's code-signing
  item: the CLI is pure Rust and CI now builds/tests it on Windows,
  Linux, and macOS runners; the desktop app itself stays Windows-only
  by design for now — see README's "Supported platforms" — since
  packaging src-tauri for macOS/Linux (notarization, WebKitGTK
  packaging, separate signing infrastructure) is a real, separate body
  of work with no user request driving it yet, not an engineering gap
  in the architecture)*

All seven roadmap phases are now done. See the project's GitHub
Issues/Projects for any further, non-phase-tracked work (bug reports,
small enhancements) going forward.
