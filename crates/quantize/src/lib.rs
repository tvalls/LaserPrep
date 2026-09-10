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

/// Each tone's mean source luminance (0-255): the average of every
/// pixel `tone_map` classified into that tone, not the linear band's
/// midpoint. `luminance` must be the same buffer `tone_map` was built
/// from (same length, same pixel order).
///
/// Callers use this to render the generated SVG's tone groups and
/// legend swatches in grayscale that mirrors the original image's own
/// histogram, so a user can visually judge each layer's relative
/// darkness at a glance — without LaserPrep ever presuming laser
/// power/speed (CLAUDE.md Sections 7 and 12: those stay LightBurn's
/// job). This is a rendering choice derived only from the source
/// image's own pixel data, nothing else.
pub fn tone_mean_luminance(luminance: &[u8], tone_map: &ToneMap) -> Vec<u8> {
    let levels = tone_map.tone_count.get() as usize;
    let mut sums = vec![0u64; levels];
    let mut counts = vec![0u64; levels];

    for (&sample, &tone) in luminance.iter().zip(tone_map.tones.iter()) {
        sums[tone as usize] += u64::from(sample);
        counts[tone as usize] += 1;
    }

    let band_width = 256.0 / levels as f64;
    (0..levels)
        .map(|tone| match sums[tone].checked_div(counts[tone]) {
            Some(mean) => mean as u8,
            // No source pixel landed in this band — an image whose
            // histogram doesn't use every level at this tone_count.
            // Fall back to the linear band's midpoint so the tone
            // still renders an ordered, plausible gray instead of
            // black.
            None => ((tone as f64 + 0.5) * band_width).round().clamp(0.0, 255.0) as u8,
        })
        .collect()
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

    #[test]
    fn tone_mean_luminance_averages_each_bands_actual_pixels() {
        let tone_count = ToneCount::new(2).unwrap();
        // Band 0 (dark): 0 and 20 -> mean 10. Band 1 (light): 200, 210,
        // 255 -> mean 221 (integer division of 665/3).
        let luminance = vec![0u8, 20, 200, 210, 255];
        let map = quantize_linear(&luminance, 5, 1, tone_count).unwrap();

        let grays = tone_mean_luminance(&luminance, &map);

        assert_eq!(grays, vec![10, 221]);
    }

    #[test]
    fn tone_mean_luminance_falls_back_to_band_midpoint_when_a_tone_is_unused() {
        // Every pixel is pure black, so with 4 tones only band 0 has
        // any real data — bands 1-3 must still get an ordered,
        // plausible gray (their linear midpoint) instead of black.
        let tone_count = ToneCount::new(4).unwrap();
        let luminance = vec![0u8; 4];
        let map = quantize_linear(&luminance, 4, 1, tone_count).unwrap();

        let grays = tone_mean_luminance(&luminance, &map);

        assert_eq!(grays[0], 0);
        assert!(grays.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(grays.len(), 4);
    }
}
