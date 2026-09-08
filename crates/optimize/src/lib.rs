//! Laser-optimization post-processing over vectorized paths: gap
//! closing, micro-segment/noise removal, minimum-area island removal,
//! adjacent-region merging, curve simplification, node reduction,
//! duplicate removal, hole/winding handling, and path ordering. See
//! CLAUDE.md Section 8 and `docs/adr/0004-geometry-boolean-ops.md`.
//! Implemented starting Phase 2 (ordering lands in Phase 5).
