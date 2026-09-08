//! Deterministic, classical-heuristics content classification.
//!
//! Implemented in Phase 3: histogram/contrast, edge density, color
//! clustering, saturation/hue, uniform-background ratio, symmetry, and
//! aspect-ratio features feed a hand-weighted scoring function per
//! content category. No machine learning — see
//! `docs/adr/0002-no-ai-ml-classification.md`.
