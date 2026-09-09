# LaserPrep

LaserPrep is an open-source Windows desktop application that converts
raster images (photos, drawings, logos, animals, people, landscapes) into
vectorized SVGs optimized for laser engraving, with grayscale tone
separation and an embedded legend to make parametrization in LightBurn
straightforward.

It is not a generic image tracer. LaserPrep is a rules-based
pre-processor that decides which vectorization strategy fits each kind of
image — a portrait, a dog, a logo, and a landscape do not go through the
same pipeline.

**LaserPrep uses no AI or machine learning anywhere in its pipeline.**
Content classification (portrait, animal, logo, drawing, landscape, ...)
is done entirely with classical, deterministic image-processing
heuristics: histograms, edge density (Sobel/Canny), color clustering
(k-means — deterministic, not a trained model), saturation, contrast,
uniform-background ratio, symmetry. See
[`docs/adr/0002-no-ai-ml-classification.md`](docs/adr/0002-no-ai-ml-classification.md).

> **Status:** Phases 0-6 of the roadmap are done (import/preview/export,
> advanced vectorization, heuristic classification and presets, batch
> processing and `.lvp` projects, laser path-ordering optimization, and
> the auto-update/crash-reporting/release pipeline). Phase 7 (CLI,
> extensibility, additional formats) is in progress; see
> [`docs/roadmap.md`](docs/roadmap.md) for exact scope and honest
> limitations of each phase.

## Supported platforms

Windows 10/11, x64, is the only officially supported platform for the
desktop app right now. The architecture keeps macOS and Linux desktop
builds open as a future addition without requiring a redesign, but they
are not built or tested yet. The `laser-vector` CLI (see below) has no
platform-specific code and is pure Rust, so it builds anywhere Rust
targets Windows/macOS/Linux — CI compiles and tests it on all three
today even though the desktop app is Windows-only for now.

## Supported input image formats

PNG, JPEG, BMP, GIF, TIFF, and WebP. The format is detected from file
content, not from the file extension. See
[`docs/adr/0009-cli-and-additional-formats.md`](docs/adr/0009-cli-and-additional-formats.md).

## Installation

> No stable releases are published yet. Once available, download
> pre-built installers (MSI/NSIS) from the
> [Releases](https://github.com/tvalls/LaserPrep/releases) page — you do
> not need to clone this repository or install Rust/Node/Cargo/any
> runtime to run LaserPrep.
>
> The installer is currently unsigned (no Windows code-signing
> certificate yet — see
> [`docs/adr/0006-code-signing-deferred.md`](docs/adr/0006-code-signing-deferred.md)),
> so Windows SmartScreen will show an "unknown publisher" warning. Choose
> "More info → Run anyway" if you trust the source you downloaded it
> from.

## Command-line interface

`laser-vector` runs the same conversion core as the desktop app
(`crates/pipeline`) without a GUI, for scripting and batch use outside
LightBurn-facing interactive editing:

```console
$ laser-vector convert input.jpg --preset portrait --tones 5 --output output.svg
converted input.jpg -> output.svg (5 tones)
classified as GenericPhoto (confidence 0.42), suggested preset: photo
validation: 7 paths (7 closed, 0 open, 0 degenerate), 214 nodes, LightBurn-compatible: true
path ordering efficiency: 61.3% of optimal travel distance

$ laser-vector presets
preset      tones   min-area   legend  merge-adjacent
photo           5         16    false           false
portrait        6         12    false           false
animal          5         20    false            true
logo            2          4    false            true
drawing         3          8    false           false
landscape       8         24    false           false

$ laser-vector convert --help
```

`--tones`, `--min-area`, `--legend`, `--merge-adjacent`, and
`--order-paths` override the chosen `--preset`'s defaults when given.
Build it with `cargo build -p laserprep-cli --release`; the binary is
`target/release/laser-vector[.exe]`.

## Building from source

See [`docs/development.md`](docs/development.md) for full prerequisites
(Rust, MSVC Build Tools, Node.js, Tauri CLI) and build/dev commands.

## Running tests

```powershell
cargo test
npm test
node scripts/validate-locales.mjs
```

## Configuration

Settings are stored as versioned JSON in the OS-standard per-user app
data directory for `io.github.tvalls.laserprep`, managed by
`crates/settings` (see
[`docs/adr/0008-settings-store-custom.md`](docs/adr/0008-settings-store-custom.md)).
Settings persist across updates through explicit, tested migrations.

## Updates

LaserPrep checks GitHub Releases for updates on startup and never
installs silently — you always see a dialog
with the current/available version and release notes, and can update now,
later, or skip that version. A manual "Check for Updates" action is also
available. See
[`docs/adr/0006-code-signing-deferred.md`](docs/adr/0006-code-signing-deferred.md)
for how update-package integrity is verified independently of platform
code signing.

## Supported languages

- English (`en-US`) — source/fallback locale
- Português do Brasil (`pt-BR`) — first-class from the start
- Español (`es`) and 简体中文 (`zh-CN`) — locale infrastructure ships
  from the start; not yet exposed in the language switcher pending a
  translation-quality pass. See
  [`docs/adr/0005-i18n-locale-scope.md`](docs/adr/0005-i18n-locale-scope.md).

### Adding a new language

1. Locale resources live in `locales/<locale>.json` (frontend) — `en-US`
   is the canonical key source; never remove or rename a key without
   updating every locale file.
2. Copy `locales/en-US.json` to `locales/<new-locale>.json` and translate
   the values. Keys and `{placeholder}` tokens must stay unchanged.
3. Register the new locale in `src/i18n/index.ts` (the `i18next`
   resources map).
4. Run `node scripts/validate-locales.mjs` — it checks that all required
   locales exist, share the same key set, and parse as valid JSON.
5. Language fallback order: exact locale (e.g. `pt-BR`) → base language
   (e.g. `pt`) → `en-US`. Missing keys never crash the app; they fall
   back to `en-US`.

## Release and versioning policy

[Semantic Versioning](https://semver.org/) (`vMAJOR.MINOR.PATCH`),
automated from [Conventional Commits](https://www.conventionalcommits.org/)
via `release-please` (see `.github/workflows/release.yml`). See
[`CHANGELOG.md`](CHANGELOG.md) for release history.

## Privacy

Your images stay on your computer. Nothing is uploaded anywhere by
default. Any future online integration is opt-in, disclosed, and can be
turned off.

## License

[GPL-3.0-or-later](LICENSE). See
[`docs/adr/0001-license-choice.md`](docs/adr/0001-license-choice.md) for
the rationale.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and the architecture decision
records in [`docs/adr/`](docs/adr/).
