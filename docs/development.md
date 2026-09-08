# Development setup

LaserPrep targets Windows first (see `README.md`). This guide covers
building it from source; end users should download release binaries
instead.

## Prerequisites

- **Rust** (stable), installed via [rustup](https://rustup.rs).
- **MSVC Build Tools** (Windows only): the `stable-x86_64-pc-windows-msvc`
  Rust toolchain needs the MSVC linker and Windows SDK. Install via the
  [Visual Studio Build Tools](https://aka.ms/vs/17/release/vs_buildtools.exe)
  installer with the "Desktop development with C++" workload
  (`Microsoft.VisualStudio.Workload.VCTools` +
  `Microsoft.VisualStudio.Component.Windows11SDK.22621`), **run as
  Administrator**. Without this, `cargo build` fails to link.
- **Node.js** 20+ and npm.
- **Tauri CLI**: `cargo install tauri-cli --version "^2"` (or use
  `npx @tauri-apps/cli` from the frontend workspace).
- **GitHub CLI** (`gh`), only needed if you plan to interact with CI/PRs
  from the command line.

## Getting started

```powershell
git clone https://github.com/tvalls/LaserPrep.git
cd LaserPrep
npm install
cargo tauri dev
```

## Running tests

```powershell
cargo test                      # Rust unit + integration tests
npm test                        # Frontend unit tests (vitest)
node scripts/validate-locales.mjs   # Locale key-parity validation
```

## Building a release package locally

```powershell
cargo tauri build
```

Produces MSI and NSIS installers under `src-tauri/target/release/bundle/`.
The build is unsigned (see `docs/adr/0006-code-signing-deferred.md`); a
locally built installer will trigger a Windows SmartScreen warning.

## Formatting and linting

```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
npm run lint
```

## Project layout

See `docs/architecture.md`.

## Settings and logs (for debugging your own dev build)

Settings file and logs live under the OS-standard app-data directory for
the app identifier `io.github.tvalls.laserprep` (Tauri resolves this via
its path APIs — see `crates/settings`).
