# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.3.0](https://github.com/tvalls/LaserPrep/compare/laserprep-v0.2.0...laserprep-v0.3.0) (2026-09-10)


### Features

* add GitHub social preview image (visual identity Phase 4) ([5418e58](https://github.com/tvalls/LaserPrep/commit/5418e58294dd3334f2d4b061182329766a8d6087))
* add GitHub social preview image (visual identity Phase 4) ([0ed3aaa](https://github.com/tvalls/LaserPrep/commit/0ed3aaa5b7a48d6e2e20cf65336844703809b12c))
* add LaserPrep visual identity (Phase 1: icons, tokens, theme) ([12d2c4c](https://github.com/tvalls/LaserPrep/commit/12d2c4c5d0b716500015fcb820c88036d2fe1602))
* add LaserPrep visual identity (Phase 1: icons, tokens, theme) ([e5ee8c4](https://github.com/tvalls/LaserPrep/commit/e5ee8c4fd897cd6e2dc5567c2c3ccb89ddab8f88))
* add Windows installer branding (visual identity Phase 3) ([0402627](https://github.com/tvalls/LaserPrep/commit/040262776dc8733e1ac8455217143870b92869ac))
* add Windows installer branding (visual identity Phase 3) ([78528b2](https://github.com/tvalls/LaserPrep/commit/78528b2bf21f8eb6024d2b680a65eb5b88b426a3))
* apply card/alert styling and preset icons (visual identity Phase 2) ([b7cc9c7](https://github.com/tvalls/LaserPrep/commit/b7cc9c7462ef4f5491fa51ebe1ed18901e266c2a))
* apply card/alert styling and preset icons (visual identity Phase 2) ([6502d04](https://github.com/tvalls/LaserPrep/commit/6502d048c2298aa1f84b9ef0b64081576f8a4594))
* render each tone in a representative grayscale, not flat black ([de44a2c](https://github.com/tvalls/LaserPrep/commit/de44a2cde72f7b1064aa86cbaa33f33511120fb6))
* render each tone in a representative grayscale, not flat black ([8adc177](https://github.com/tvalls/LaserPrep/commit/8adc17765e280da78fe25181b876c0c5cb4f0f78))


### Bug Fixes

* correct release.yml bundle path and tag-to-version stripping ([650e177](https://github.com/tvalls/LaserPrep/commit/650e17743924523c47d89fdf321c15016e9fe205))
* match Tauri's actual Windows updater signature filenames ([e6f0f8b](https://github.com/tvalls/LaserPrep/commit/e6f0f8b29478ac9e391290b90d0c0b5faf5d7301))
* match Tauri's actual Windows updater signature filenames ([7678432](https://github.com/tvalls/LaserPrep/commit/7678432e6b0632612d7c89723fd0b254ec27cb9d))
* use the workspace-root target dir and correct tag prefix in release.yml ([bd176bf](https://github.com/tvalls/LaserPrep/commit/bd176bf2bc1a735f3092c4341b89861cf54209e2))

## [0.2.0](https://github.com/tvalls/LaserPrep/compare/laserprep-v0.1.0...laserprep-v0.2.0) (2026-09-09)


### Features

* add laser-vector CLI and expand supported image formats ([d304699](https://github.com/tvalls/LaserPrep/commit/d304699381990038d41744a0d8255a3ebed6c5f6))
* add laser-vector CLI and expand supported image formats (Phase 7) ([8c5b7a0](https://github.com/tvalls/LaserPrep/commit/8c5b7a0ce216cc1ac67c395ea008f6bd55e7c091))
* add the optional tone legend (CLAUDE.md Section 10) ([7ca2e96](https://github.com/tvalls/LaserPrep/commit/7ca2e9698f47b093daf509184463843a81a6430a))
* add the optional tone legend (CLAUDE.md Section 10) ([401ea39](https://github.com/tvalls/LaserPrep/commit/401ea39bbe9d4aecddfd030f9cdaa89e7fd023ca))
* add the SVG validator (CLAUDE.md Section 9) ([c7a0e26](https://github.com/tvalls/LaserPrep/commit/c7a0e26259b162cf0cabedc4551d0e6ff43f3e50))
* **analysis:** add the rule-based content classifier ([f7d4ef4](https://github.com/tvalls/LaserPrep/commit/f7d4ef45b238e74c1fcd16ecf3aef915d1b6e928))
* **analysis:** add the rule-based content classifier ([4b0f6ff](https://github.com/tvalls/LaserPrep/commit/4b0f6ff255a5bb5ea1cad1e5b1cfc37295db5c9d))
* **batch:** add batch image conversion with per-item progress ([0c4b20e](https://github.com/tvalls/LaserPrep/commit/0c4b20eb70b214d476b20a9f3ef79a39aa9e6e3f))
* **batch:** add batch image conversion with per-item progress ([e719f8a](https://github.com/tvalls/LaserPrep/commit/e719f8af39d32904dbbc3282dd89861c59f14f9b))
* cap decoded image dimensions for memory safety ([4e7cb54](https://github.com/tvalls/LaserPrep/commit/4e7cb54a2b177ef6ccd135830e3b04f1e9e5bf15))
* **crash:** capture Rust panics and frontend crashes to the log ([45298dc](https://github.com/tvalls/LaserPrep/commit/45298dcd1b6bc03bf9f91042df64042e986f601e))
* **crash:** capture Rust panics and frontend crashes to the log ([e4bcfd4](https://github.com/tvalls/LaserPrep/commit/e4bcfd4680758c197f97c2b54f46d8668bf6c520))
* expose Minimum Area as a tunable vectorization parameter ([7be5f34](https://github.com/tvalls/LaserPrep/commit/7be5f34f09f11f6c6910107d8e24777169008577))
* expose Minimum Area as a tunable vectorization parameter ([57930cc](https://github.com/tvalls/LaserPrep/commit/57930cc0b44c8c414ebe0e2a8069c9380636955d))
* **imaging:** add raster decoding to grayscale luminance ([c0d949e](https://github.com/tvalls/LaserPrep/commit/c0d949e85944bf5e7886ac4973fdae4f4ad893e2))
* **imaging:** cap decoded image dimensions to protect memory/CPU ([86c03b5](https://github.com/tvalls/LaserPrep/commit/86c03b5f35e502d145b0ff4c6158c83a65c2ddc5))
* implement crates/optimize merge_adjacent_regions (closes Phase 2) ([3dd8ab4](https://github.com/tvalls/LaserPrep/commit/3dd8ab4da7b26922877bba6e0acb0b73eb7bee3b))
* implement crates/optimize's merge_adjacent_regions ([a05605b](https://github.com/tvalls/LaserPrep/commit/a05605bead57e97039830e99b5bf29f984385d8c))
* implement heuristic feature extraction (Phase 3, CLAUDE.md Section 6) ([abdf087](https://github.com/tvalls/LaserPrep/commit/abdf08773681c95fbc18156900f9b8a333a2dc01))
* implement heuristic feature extraction (Phase 3) ([1b7b529](https://github.com/tvalls/LaserPrep/commit/1b7b529f478e410c370c469f94d25de143f738d4))
* implement raster-to-SVG vectorization (vtracer backend) ([dae0100](https://github.com/tvalls/LaserPrep/commit/dae010023e3b5c9784c05a97dc7878f193575439))
* **optimize:** add laser path ordering and an optimization score ([d571e40](https://github.com/tvalls/LaserPrep/commit/d571e402a2a3774d62f5f3ed858e001624c7f6ea))
* **optimize:** add laser path ordering and an optimization score ([269d35b](https://github.com/tvalls/LaserPrep/commit/269d35b718555826962ea316df67858fa811ce92))
* Phase 1 core pipeline start (imaging, quantize, svggen) + CI security hardening ([b0605cf](https://github.com/tvalls/LaserPrep/commit/b0605cfc86ea732891ac928280409f876e38d09f))
* Phase 1 import/preview/export flow (Tauri commands + UI) ([784b004](https://github.com/tvalls/LaserPrep/commit/784b004dace350f491628f4209a3aadb8e5ed81f))
* **presets:** implement named conversion presets (CLAUDE.md Section 12) ([7f29f45](https://github.com/tvalls/LaserPrep/commit/7f29f45fe53126039797a504f68155cd38203d04))
* **presets:** implement named conversion presets (CLAUDE.md Section 12) ([ac2a0aa](https://github.com/tvalls/LaserPrep/commit/ac2a0aadd4844462a14befe710a30b20195547a1))
* **preview:** add before/after, per-tone, and LightBurn-style preview ([fe6a474](https://github.com/tvalls/LaserPrep/commit/fe6a4748ec45916c7486d4c486f32f45fa2d7029))
* **preview:** add before/after, per-tone, and LightBurn-style preview ([eb2a36c](https://github.com/tvalls/LaserPrep/commit/eb2a36c0f7d5918fecab452d8cb0081929a54635))
* **project:** implement the .lvp project file format ([f74f922](https://github.com/tvalls/LaserPrep/commit/f74f922f92ea45a28c97111cded68a74b8eea8da))
* **project:** implement the .lvp project file format ([e1ed65a](https://github.com/tvalls/LaserPrep/commit/e1ed65ad83d5330c20fa5996985edaaa87d0b969))
* **project:** wire .lvp save/open into Tauri commands and the UI ([8baa2de](https://github.com/tvalls/LaserPrep/commit/8baa2de5763de3543bffadf1dc76dcdb519754c1))
* **project:** wire .lvp save/open into Tauri commands and the UI ([05cf4a4](https://github.com/tvalls/LaserPrep/commit/05cf4a45179dafb10aa4b506b38fa0a68534cfa7))
* **quantize:** implement linear N-tone grayscale quantization ([ac6a653](https://github.com/tvalls/LaserPrep/commit/ac6a6537f46b461d99a76069225a25addfbb54ad))
* **report:** add bug reporting and structured diagnostic logging ([4b4a39d](https://github.com/tvalls/LaserPrep/commit/4b4a39dfae816332c11d166f264e6a423f49711c))
* **report:** add bug reporting and structured diagnostic logging ([cebb044](https://github.com/tvalls/LaserPrep/commit/cebb04486a286473ba26811029a69b30776c774f))
* scaffold Phase 0 repository, architecture, and CI/CD workflows ([2764592](https://github.com/tvalls/LaserPrep/commit/2764592659f7402d8f3e6ff336355dbce2c8680c))
* scaffold Phase 0 repository, architecture, and CI/CD workflows ([1d25f3e](https://github.com/tvalls/LaserPrep/commit/1d25f3ebca9e73356111a209064811169cf157f7))
* surface SVG validation results in the conversion result and UI ([01e3bff](https://github.com/tvalls/LaserPrep/commit/01e3bff0a1a3f55d50e03fad513e2dc2982dfa8d))
* surface SVG validation results in the UI ([313ba2f](https://github.com/tvalls/LaserPrep/commit/313ba2f967d23d87e8c8f9f54e617c03c1e9f5d4))
* **svggen:** add the SVG validator (CLAUDE.md Section 9) ([10a0f20](https://github.com/tvalls/LaserPrep/commit/10a0f20addec6b1fb5ae1f678fd258feabfde395))
* **svggen:** render a ToneMap as a basic per-tone SVG ([0d4d438](https://github.com/tvalls/LaserPrep/commit/0d4d438d8dc6b71b46cdad76b3f1efaf6eaf606f))
* **tauri:** wire imaging/quantize/svggen into IPC commands ([81249fb](https://github.com/tvalls/LaserPrep/commit/81249fba02fad92e86b7ef0bc39e63129fe9007d))
* **ui:** add import/preview/export flow to the main window ([41f6795](https://github.com/tvalls/LaserPrep/commit/41f6795fb08d0714d9e5f75c5aac8cbf9bd834b3))
* **ui:** add undo/redo for conversion parameters ([005e621](https://github.com/tvalls/LaserPrep/commit/005e62152feb9627d4325520892e703b94cab085))
* **ui:** add undo/redo for conversion parameters ([48587c5](https://github.com/tvalls/LaserPrep/commit/48587c5fb5ce88323249336acd351a8db89ca722))
* **update:** add auto-update via tauri-plugin-updater ([48a4842](https://github.com/tvalls/LaserPrep/commit/48a484237f0c0d8d82e1412ecac22a1e5c253941))
* **update:** add auto-update via tauri-plugin-updater ([1cd43db](https://github.com/tvalls/LaserPrep/commit/1cd43dbbe59b8e809ea00835486d4b5b86349f2a))
* **vectorize:** implement Vectorizer with a vtracer backend ([cfa3c31](https://github.com/tvalls/LaserPrep/commit/cfa3c31221f5db5201c64d36d105ff61cdd7d5fc))
* wire heuristic classification into the pipeline and UI ([a841418](https://github.com/tvalls/LaserPrep/commit/a841418c80b1a9b6586f4680bf3e3dc743a6d2cf))
* wire real vectorization into svggen and the conversion pipeline ([a1461a9](https://github.com/tvalls/LaserPrep/commit/a1461a9dc918fb1b4a07a837bd2214f9987bd073))
* wire real vectorization into the conversion pipeline ([b07c0cf](https://github.com/tvalls/LaserPrep/commit/b07c0cf05d28c62db489edbe29500b777d7e5f54))


### Bug Fixes

* resolve build/test/lint failures found by first real compilation ([d8229fb](https://github.com/tvalls/LaserPrep/commit/d8229fb9e3a84f069ebd04fda642b8199eb82d38))


### Performance Improvements

* **pipeline:** cache decoded/classified source across reconversions ([bfa12c5](https://github.com/tvalls/LaserPrep/commit/bfa12c5e6e3efbf94c4917b312c8efe0b2397b5a))
* **pipeline:** cache decoded/classified source across reconversions ([6c7977e](https://github.com/tvalls/LaserPrep/commit/6c7977ed3ca99a046f1215aa8a022c89c21ee28d))

## [Unreleased]

### Added

- Initial repository scaffolding: license (GPL-3.0-or-later), governance
  documents, architecture decision records, Cargo workspace layout,
  Tauri 2 + React/TypeScript application skeleton, CI/CD workflow
  definitions, and localization infrastructure (`en-US`, `pt-BR`, `es`,
  `zh-CN`).
