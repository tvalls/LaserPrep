# ADR 0007: Test image fixtures are generated synthetically, not sourced photos

## Status

Accepted

## Context

CLAUDE.md Section 15 requires a per-category test image suite (face,
multiple people, dog, cat, furry animal, logo, drawing, landscape, white
background, complex background, high/low contrast) for regression and
golden/snapshot testing. Section 14 forbids committing personal test
photos, and licensing real photographs (even "free stock") for a public
open-source repository requires per-image license tracking that is easy
to get wrong.

## Decision

`tests/fixtures/` images are generated procedurally by code at test build
time (or committed as small generated PNGs produced by a documented,
re-runnable generator script), not sourced from real photographs:

- "portrait-like": a filled ellipse (head) + two smaller ellipses (eyes)
  on a plain background, tuned to trip the symmetry/skin-hue heuristics
- "animal-like": an irregular blob silhouette with a rounded head-like
  protrusion, higher edge-density texture pass
- "logo-like": a handful of solid-color geometric shapes on a uniform
  background, hard edges, no anti-aliasing
- "landscape-like": a horizontal gradient band composition
- "white-background" / "complex-background" / "high-contrast" /
  "low-contrast": straightforward parametric variants of the above

This is explicitly a **known limitation**: synthetic fixtures validate
pipeline correctness (closed paths, tone-group presence, no NaN, node/path
counts within bounds) but are not a substitute for real-world photographs
when judging perceptual quality of a category's dedicated pipeline
(Section 6 of CLAUDE.md).

## Consequences

- `tests/fixtures/generate.rs` (or equivalent) is the single source of
  truth for fixtures; images are regenerated, not hand-edited.
- A future addition of curated CC0/public-domain real photographs to
  `tests/fixtures/real/` is left open, with a per-image license file
  required before any such image is committed — not part of Phase 0.
