# ADR 0004: Polygon geometry uses i_overlay

## Status

Accepted (amended — see "Amendment" below)

## Context

Laser-optimization post-processing (`crates/optimize`) needs polygon
representations, area/winding-rule calculations, and boolean operations
(union of adjacent same-tone regions, hole detection).

## Decision

- `i_overlay` (MIT OR Apache-2.0, actively maintained — v8 at the time
  of implementation) for boolean operations (union/intersection/
  difference/self-union) needed to merge adjacent regions. Currently
  the most robust pure-Rust polygon clipping option available.
- Polygons are represented as `i_overlay`'s own native shape format
  (`Vec<Vec<[f64; 2]>>`: a list of contours, first = exterior, rest =
  holes) rather than a separate `geo`/`geo-types` model — see
  "Amendment" below.

### Amendment (Phase 2, `merge_adjacent_regions` implementation)

The original decision also named `geo`/`geo-types` as "the core
polygon data model, area calculation, and winding-rule handling."
Implementing `crates/optimize::merge_adjacent_regions` found this
unnecessary: `laserprep_vectorize::VtracerVectorizer::trace_polygons`
extracts `vtracer`'s `PathSimplifyMode::Polygon` output directly as
`Vec<Vec<[f64; 2]>>` (exterior contour first, holes after — the same
convention SVG's nonzero fill rule and `i_overlay`'s `Shape` format
both use), and `i_overlay::float::simplify::SimplifyShape` operates on
that shape directly. Adding `geo`/`geo-types` would only introduce a
conversion layer with no operation that needs it yet. If a later
feature genuinely needs `geo`'s richer analysis (e.g. precise area/
centroid calculations beyond what a self-union requires), add it then
rather than carrying an unused dependency now (CLAUDE.md Section 17:
verify necessity before adding a dependency).

## Consequences

- Rust's polygon-boolean-operation ecosystem is less battle-tested than
  C/C++ equivalents (CGAL, Clipper). We mitigate this with:
  - never performing a merge/close operation that would produce invalid
    geometry — if a boolean op fails or produces a degenerate/self-
    intersecting result, the original paths are preserved unmodified and
    the user is warned (per CLAUDE.md Section 8: "Quando o fechamento não
    for matematicamente seguro, preservar o path original e avisar o
    usuário").
  - an extensive geometry regression test suite in `crates/optimize`
    covering the fixture categories in `tests/fixtures/`.
- If `i_overlay` proves insufficient in practice, the fallback is
  re-evaluating other Rust boolean-ops crates; the dependency is isolated
  to `crates/optimize`, not leaked into the domain model.
