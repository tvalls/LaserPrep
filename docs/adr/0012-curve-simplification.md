# ADR 0012: Curve Simplification / Node Reduction as one parameter

## Status

Accepted

## Context

CLAUDE.md Section 8 lists "Curve Simplification" and "Node Reduction"
among the required exposed laser-optimization parameters, alongside
Minimum Area, Merge Adjacent Regions, and Path Ordering. Only the
latter three were ever implemented — Curve Simplification/Node
Reduction shipped nowhere: not in `crates/vectorize`, not in presets,
not in the desktop UI, not in the CLI.

Two independent real-world tests in the same session hit this gap
directly:

- A full-body photo with a reflective, textured metal background (an
  elevator) produced 809 paths / 37,808 total nodes. Raising Minimum
  Area helped (fewer, larger discarded background regions), but didn't
  touch the *shape* of the paths that survived.
- A close-up portrait (1788×2048, natural skin/hair detail) produced
  1,555 paths / 78,414 nodes, with a single path alone accounting for
  35,756 of them. This is a different failure mode: not many small
  separate regions (which Minimum Area targets) but one region with an
  extremely detailed boundary. No amount of raising Minimum Area fixes
  a single oversized, overly-detailed path — smoothing the curve
  itself is the only lever that helps.

## Decision

Expose vtracer's own curve-fitting tolerance
(`vtracer::Config::length_threshold`, `Preset::Bw`'s stock default
`4.0`) as `curve_simplification: f64`, threaded through every layer:
`VtracerVectorizer::new(min_area_px2, curve_simplification)` →
`crates/pipeline`'s `convert_decoded_to_svg`/`convert_bytes_to_svg` →
`crates/presets::Preset` → `crates/project::ConversionParams` (`.lvp`
persistence, with `#[serde(default = "default_curve_simplification")]`
= `4.0` so projects saved before this field existed keep reconverting
identically) → the desktop UI (`src-tauri/src/commands.rs`,
`src/App.tsx`) → the CLI (`--curve-simplification`).

**One parameter, not two**, despite CLAUDE.md naming both "Curve
Simplification" and "Node Reduction": in vtracer's curve-fitting
algorithm, fewer nodes is a direct, inseparable consequence of
allowing more simplification (a higher fitting-error tolerance), not
an independently tunable axis. Inventing a second control with
identical real effect would be exactly the kind of duplicate/fake
control CLAUDE.md Section 13 warns against — it would look like two
knobs but only ever do one thing.

Per-preset defaults were set from the two real test cases above, not
guessed: `Photo`/`Logo`/`Drawing` keep vtracer's stock `4.0` (logos and
line art want sharp corners preserved, not smoothed away); `Portrait`
and `Animal` — content with a lot of natural high-frequency texture
(skin, fur) — get `8.0`; `Landscape` (skies/foliage/terrain, already
documented as needing detail reduction) gets `10.0`, the highest of
any preset.

## Consequences

- `VtracerVectorizer` derives `PartialEq` but no longer `Eq` (`f64`
  doesn't implement `Eq`); nothing in the codebase relied on `Eq`
  there. Same change to `laserprep_presets::Preset`.
- `convert_decoded_to_svg` and `convert_bytes_to_svg` in
  `crates/pipeline` now carry `#[allow(clippy::too_many_arguments)]`
  (8 parameters) rather than being bundled into a struct — these are
  exactly CLAUDE.md Section 8's named conversion knobs, already
  grouped one layer up as `laserprep_project::ConversionParams` at the
  Tauri command boundary; wrapping them in a second struct here would
  just move the sprawl, not reduce it.
- Verified the mechanism against the two real test images that
  motivated this ADR (not just synthetic fixtures): on the elevator
  photo, raising `curve_simplification` alone (4.0 → 20.0, `min_area`
  unchanged) cut total nodes from 37,808 to 34,226 — real but modest,
  because that image's dominant problem is *many small separate
  regions* (Minimum Area's territory), not one over-detailed path.
  Combined with a higher Minimum Area (50), paths dropped from 809 to
  275 and nodes to 30,542. The portrait case (one 35,756-node path)
  wasn't available to re-test directly, but is exactly the failure
  mode the synthetic "jagged staircase boundary" tests in
  `crates/vectorize`, `crates/pipeline`, and `crates/cli` reproduce and
  confirm this parameter fixes.
- Minimum Area and Curve Simplification solve different problems and
  are both worth raising together for texture-heavy photos: Minimum
  Area discards whole small regions; Curve Simplification smooths the
  boundary of the regions (however large) that remain.
