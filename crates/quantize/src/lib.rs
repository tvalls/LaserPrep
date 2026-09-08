//! Tone quantization: reduces an image to a `ToneCount` (2-16) of
//! discrete grayscale levels, per CLAUDE.md Section 7. Implemented in
//! Phase 1 (linear/basic) and extended in Phase 2-3 (perceptual,
//! adaptive, histogram-based, per-region, manually editable thresholds).

use laserprep_domain::ToneCount;

/// Per-pixel tone indices (`0..tone_count`) for an image of `width` by
/// `height` pixels, in row-major order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneMap {
    pub width: u32,
    pub height: u32,
    pub tone_count: ToneCount,
    pub tones: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum QuantizeError {
    #[error(
        "luminance buffer has {actual} samples, expected {expected} for a {width}x{height} image"
    )]
    SizeMismatch {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}

/// Splits the 0-255 luminance range into `tone_count` equal-width bands
/// and assigns each pixel the index of the band it falls into. This is
/// the simplest supported distribution; perceptual, adaptive,
/// histogram-based, and per-region strategies (CLAUDE.md Section 7)
/// are added in later phases.
pub fn quantize_linear(
    luminance: &[u8],
    width: u32,
    height: u32,
    tone_count: ToneCount,
) -> Result<ToneMap, QuantizeError> {
    let expected = width as usize * height as usize;
    if luminance.len() != expected {
        return Err(QuantizeError::SizeMismatch {
            width,
            height,
            expected,
            actual: luminance.len(),
        });
    }

    let levels = u32::from(tone_count.get());
    let band_width = 256.0 / levels as f64;
    let max_index = (levels - 1) as u8;

    let tones = luminance
        .iter()
        .map(|&sample| {
            let band = (f64::from(sample) / band_width) as u8;
            band.min(max_index)
        })
        .collect();

    Ok(ToneMap {
        width,
        height,
        tone_count,
        tones,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_mismatched_buffer_length() {
        let err = quantize_linear(&[0, 1, 2], 2, 2, ToneCount::default()).unwrap_err();
        assert_eq!(
            err,
            QuantizeError::SizeMismatch {
                width: 2,
                height: 2,
                expected: 4,
                actual: 3,
            }
        );
    }

    #[test]
    fn splits_full_range_into_equal_bands() {
        let tone_count = ToneCount::new(2).unwrap();
        let map = quantize_linear(&[0, 127, 128, 255], 2, 2, tone_count).unwrap();
        assert_eq!(map.tones, vec![0, 0, 1, 1]);
    }

    #[test]
    fn never_produces_an_index_outside_the_tone_count() {
        let tone_count = ToneCount::new(3).unwrap();
        let luminance: Vec<u8> = (0..=255).collect();
        let map = quantize_linear(&luminance, 256, 1, tone_count).unwrap();
        assert!(map.tones.iter().all(|&t| t < tone_count.get()));
        // The darkest and brightest pixels must land in the first and
        // last band respectively.
        assert_eq!(map.tones.first(), Some(&0));
        assert_eq!(map.tones.last(), Some(&2));
    }

    #[test]
    fn single_tone_count_maps_everything_to_band_zero() {
        let tone_count = ToneCount::new(2).unwrap();
        let map = quantize_linear(&[0, 255], 2, 1, tone_count).unwrap();
        assert_eq!(map.tones, vec![0, 1]);
    }
}
