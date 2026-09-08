# ADR 0004: Polygon geometry uses geo + i_overlay

## Status

Accepted

## Context

Laser-optimization post-processing (`crates/optimize`) needs polygon
representations, area/winding-rule calculations, and boolean operations
(union of adjacent same-tone regions, hole detection).

## Decision

- `geo` / `geo-types` for the core polygon data model, area calculation,
  and winding-rule handling. This is the de facto standard geometry crate
  in the Rust ecosystem (MIT/Apache-2.0, mature, widely depended upon).
- `i_overlay` for boolean operations (union/intersection/difference)
  needed to merge adjacent regions. It is actively maintained and
  currently the most robust pure-Rust polygon clipping option available.

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
