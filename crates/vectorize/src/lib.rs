//! Raster-to-SVG tracing.
//!
//! Implemented in Phase 2. Exposes our own [`Vectorizer`] trait and
//! [`VectorPath`]/[`BinaryMask`] types so the tracing backend
//! (`vtracer`, see `docs/adr/0003-vectorization-library.md`) is never
//! depended on directly by other crates — only [`VtracerVectorizer`]'s
//! implementation touches the `vtracer`/`visioncortex` crates.

use laserprep_quantize::ToneMap;
use vtracer::{ColorImage, ColorMode, Config, Preset};

/// A row-major boolean mask over an image: `true` marks a pixel that
/// belongs to the tone being traced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryMask {
    pub width: u32,
    pub height: u32,
    pub foreground: Vec<bool>,
}

impl BinaryMask {
    /// Builds the mask of pixels in `tone_map` equal to `tone`.
    pub fn from_tone_map(tone_map: &ToneMap, tone: u8) -> Self {
        Self {
            width: tone_map.width,
            height: tone_map.height,
            foreground: tone_map.tones.iter().map(|&t| t == tone).collect(),
        }
    }
}

/// One traced region, already a complete, renderable SVG `<path>`
/// element (its `d`, `fill`, and any coordinate-offset `transform`
/// attributes are already resolved) — callers embed it verbatim; they
/// never parse or reconstruct path data themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorPath {
    pub svg_element: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VectorizeError {
    #[error("mask buffer has {actual} samples, expected {expected} for a {width}x{height} mask")]
    SizeMismatch {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
    #[error("tracing failed: {0}")]
    Trace(String),
}

/// Traces a [`BinaryMask`] into vector paths. Implemented by
/// [`VtracerVectorizer`]; kept as a trait so the tracing backend can be
/// swapped later without touching callers (docs/adr/0003).
pub trait Vectorizer {
    fn trace(&self, mask: &BinaryMask) -> Result<Vec<VectorPath>, VectorizeError>;
}

/// The default [`Vectorizer`], backed by `vtracer` in binary-tracing
/// mode (one call per tone mask).
#[derive(Debug, Clone, Copy, Default)]
pub struct VtracerVectorizer;

impl Vectorizer for VtracerVectorizer {
    fn trace(&self, mask: &BinaryMask) -> Result<Vec<VectorPath>, VectorizeError> {
        let expected = mask.width as usize * mask.height as usize;
        if mask.foreground.len() != expected {
            return Err(VectorizeError::SizeMismatch {
                width: mask.width,
                height: mask.height,
                expected,
                actual: mask.foreground.len(),
            });
        }

        // vtracer's binary tracing mode treats pixels with r < 128 as
        // foreground, so foreground -> black, background -> white.
        let mut pixels = Vec::with_capacity(expected * 4);
        for &is_foreground in &mask.foreground {
            let channel = if is_foreground { 0u8 } else { 255u8 };
            pixels.extend_from_slice(&[channel, channel, channel, 255]);
        }

        let image = ColorImage {
            pixels,
            width: mask.width as usize,
            height: mask.height as usize,
        };

        let mut config = Config::from_preset(Preset::Bw);
        config.color_mode = ColorMode::Binary;

        let svg = vtracer::convert(image, config).map_err(VectorizeError::Trace)?;

        Ok(svg
            .paths
            .into_iter()
            .map(|path| VectorPath {
                svg_element: path.to_string(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laserprep_domain::ToneCount;
    use laserprep_quantize::quantize_linear;

    fn solid_square_mask(width: u32, height: u32) -> BinaryMask {
        let mut foreground = vec![false; (width * height) as usize];
        for y in 2..height - 2 {
            for x in 2..width - 2 {
                foreground[(y * width + x) as usize] = true;
            }
        }
        BinaryMask {
            width,
            height,
            foreground,
        }
    }

    #[test]
    fn traces_a_solid_square_into_one_path() {
        let mask = solid_square_mask(12, 12);
        let paths = VtracerVectorizer.trace(&mask).unwrap();

        assert_eq!(paths.len(), 1);
        assert!(paths[0].svg_element.contains("<path"));
    }

    #[test]
    fn an_all_background_mask_produces_no_paths() {
        let mask = BinaryMask {
            width: 4,
            height: 4,
            foreground: vec![false; 16],
        };
        let paths = VtracerVectorizer.trace(&mask).unwrap();
        assert!(paths.is_empty());
    }

    #[test]
    fn rejects_a_mismatched_mask_buffer() {
        let mask = BinaryMask {
            width: 4,
            height: 4,
            foreground: vec![false; 10],
        };
        let err = VtracerVectorizer.trace(&mask).unwrap_err();
        assert!(matches!(err, VectorizeError::SizeMismatch { .. }));
    }

    #[test]
    fn builds_a_mask_from_a_quantized_tone_map() {
        let tone_count = ToneCount::new(2).unwrap();
        let tone_map = quantize_linear(&[0, 255, 0, 255], 2, 2, tone_count).unwrap();

        let dark = BinaryMask::from_tone_map(&tone_map, 0);
        assert_eq!(dark.foreground, vec![true, false, true, false]);

        let light = BinaryMask::from_tone_map(&tone_map, 1);
        assert_eq!(light.foreground, vec![false, true, false, true]);
    }
}
