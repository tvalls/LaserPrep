//! Raster-to-SVG tracing.
//!
//! Implemented in Phase 2. Exposes a `Vectorizer` trait so the tracing
//! backend (`vtracer`, see `docs/adr/0003-vectorization-library.md`) is
//! never depended on directly by other crates.
