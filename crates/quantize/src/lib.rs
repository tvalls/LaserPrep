//! Tone quantization: reduces an image to a `ToneCount` (2-16) of
//! discrete grayscale levels, per CLAUDE.md Section 7. Implemented in
//! Phase 1 (linear/basic) and extended in Phase 2-3 (perceptual,
//! adaptive, histogram-based, per-region, manually editable thresholds).
