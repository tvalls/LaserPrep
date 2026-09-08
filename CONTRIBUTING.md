# Contributing to LaserPrep

Thanks for your interest in contributing. LaserPrep is a Windows-first,
open-source desktop application that converts raster images into
laser-engraving-optimized SVGs, without any AI/ML in its pipeline (see
`docs/adr/0002-no-ai-ml-classification.md` — this is a hard project
constraint, not open for debate in pull requests).

## Before you start

- Read `docs/architecture.md` and the ADRs in `docs/adr/` — most
  non-obvious technical decisions (why `vtracer`, why no
  `tauri-plugin-store`, why synthetic test fixtures, etc.) are recorded
  there.
- For anything beyond a small fix, open an issue first to discuss the
  approach before writing code.

## Development setup

See `docs/development.md` for the full toolchain setup (Rust, Node.js,
Tauri CLI, platform build prerequisites).

## Code standards

- Small functions, clear responsibilities, strong types, explicit error
  handling. Avoid indiscriminate `unwrap()` in Rust.
- No premature abstractions, no global singletons, no unnecessary
  dependencies.
- No fake functionality: don't ship a button that doesn't work, don't
  leave `TODO: implement later` in a core feature. Planned-but-unbuilt
  functionality is tracked as a GitHub issue instead.
- All engineering-facing content (code, comments, commit messages, PR
  descriptions, docs) is written in English. User-facing strings go
  through the localization resources in `locales/` and may be translated.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/), in English:

```text
feat: add automatic path closing for open regions
fix: preserve tone thresholds on preset switch
docs: document the LightBurn compatibility validator
```

Breaking changes use `!` or a `BREAKING CHANGE:` footer. This drives
automatic semantic versioning in CI — see `docs/architecture.md`.

## Pull requests

1. Fork/branch, make your change with tests.
2. `cargo fmt`, `cargo clippy`, `cargo test` must pass locally.
3. `npm run lint`, `npm test`, `npm run build` must pass locally for
   frontend changes.
4. Update `CHANGELOG.md` under "Unreleased" when the change is
   user-visible.
5. Open the PR against `main` with a clear description of what changed
   and how it was tested. CI must be green before merge.

## Adding a new language

See the "Adding a New Language" section in `README.md`.

## Reporting bugs / requesting features

Use the issue templates under `.github/ISSUE_TEMPLATE/`.
