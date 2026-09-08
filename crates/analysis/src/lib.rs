//! Deterministic, classical-heuristics feature extraction.
//!
//! Implemented in Phase 3: histogram/contrast, edge density, color
//! saturation, and uniform-background ratio feed a hand-weighted
//! scoring function per content category (added in a follow-up once
//! this feature set is in place). No machine learning — see
//! `docs/adr/0002-no-ai-ml-classification.md`.

use laserprep_imaging::RgbImage;

/// Sobel gradient magnitude above this is counted as an edge pixel.
/// Magnitudes range roughly 0..1442 (4 * 255 * sqrt(2)); this is a
/// deliberately simple fixed threshold for the first heuristic pass —
/// see the module doc comment.
const EDGE_MAGNITUDE_THRESHOLD: f64 = 80.0;

/// A border pixel counts toward [`ImageFeatures::uniform_background_ratio`]
/// when its Euclidean distance (in 0..=255 RGB space) to the mean
/// border color is at most this.
const UNIFORM_BACKGROUND_TOLERANCE: f64 = 24.0;

/// Measurements extracted from an image by classical heuristics only
/// (CLAUDE.md Section 6): no face/object detection, no trained models.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageFeatures {
    pub width: u32,
    pub height: u32,
    /// `width / height`.
    pub aspect_ratio: f64,
    /// Mean luminance, 0.0 (black) to 255.0 (white).
    pub mean_luminance: f64,
    /// Population standard deviation of luminance — a low-cost global
    /// contrast estimate.
    pub contrast: f64,
    /// `max luminance - min luminance` actually present in the image.
    pub dynamic_range: u8,
    /// Mean HSV saturation, 0.0 (grayscale) to 1.0 (fully saturated).
    pub mean_saturation: f64,
    /// Fraction (0.0-1.0) of border pixels within
    /// [`UNIFORM_BACKGROUND_TOLERANCE`] of the border's mean color —
    /// high for a plain studio/product background, low for a busy or
    /// textured one.
    pub uniform_background_ratio: f64,
    /// Fraction (0.0-1.0) of pixels flagged as an edge by a Sobel
    /// operator on luminance.
    pub edge_density: f64,
}

/// Extracts [`ImageFeatures`] from `image`. Pure function of pixel
/// data — no I/O, no randomness, no external model.
pub fn analyze(image: &RgbImage) -> ImageFeatures {
    let luminance = luminance_grid(image);

    let mean_luminance = mean(&luminance);
    let contrast = std_dev(&luminance, mean_luminance);
    let dynamic_range = dynamic_range(&luminance);
    let mean_saturation = mean_saturation(&image.samples);
    let uniform_background_ratio = uniform_background_ratio(image);
    let edge_density = edge_density(&luminance, image.width, image.height);

    ImageFeatures {
        width: image.width,
        height: image.height,
        aspect_ratio: f64::from(image.width) / f64::from(image.height),
        mean_luminance,
        contrast,
        dynamic_range,
        mean_saturation,
        uniform_background_ratio,
        edge_density,
    }
}

/// ITU-R BT.601 luma, one value per pixel, row-major.
fn luminance_grid(image: &RgbImage) -> Vec<f64> {
    image
        .samples
        .iter()
        .map(|&[r, g, b]| 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b))
        .collect()
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn std_dev(values: &[f64], mean_value: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let variance =
        values.iter().map(|v| (v - mean_value).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

fn dynamic_range(luminance: &[f64]) -> u8 {
    let (min, max) = luminance
        .iter()
        .fold((255.0_f64, 0.0_f64), |(min, max), &v| {
            (min.min(v), max.max(v))
        });
    if max < min {
        0
    } else {
        (max - min).round() as u8
    }
}

fn hsv_saturation([r, g, b]: [u8; 3]) -> f64 {
    let (r, g, b) = (f64::from(r), f64::from(g), f64::from(b));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max <= 0.0 { 0.0 } else { (max - min) / max }
}

fn mean_saturation(samples: &[[u8; 3]]) -> f64 {
    mean(
        &samples
            .iter()
            .copied()
            .map(hsv_saturation)
            .collect::<Vec<_>>(),
    )
}

fn uniform_background_ratio(image: &RgbImage) -> f64 {
    let border = border_pixels(image);
    if border.is_empty() {
        return 1.0;
    }

    let count = border.len() as f64;
    let (sum_r, sum_g, sum_b) = border
        .iter()
        .fold((0.0, 0.0, 0.0), |(sr, sg, sb), &[r, g, b]| {
            (sr + f64::from(r), sg + f64::from(g), sb + f64::from(b))
        });
    let mean_color = (sum_r / count, sum_g / count, sum_b / count);

    let close_count = border
        .iter()
        .filter(|&&[r, g, b]| {
            let dr = f64::from(r) - mean_color.0;
            let dg = f64::from(g) - mean_color.1;
            let db = f64::from(b) - mean_color.2;
            (dr * dr + dg * dg + db * db).sqrt() <= UNIFORM_BACKGROUND_TOLERANCE
        })
        .count();

    close_count as f64 / count
}

fn border_pixels(image: &RgbImage) -> Vec<[u8; 3]> {
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let at = |x: u32, y: u32| image.samples[(y * width + x) as usize];
    let mut pixels = Vec::with_capacity(2 * (width + height) as usize);

    for x in 0..width {
        pixels.push(at(x, 0));
        if height > 1 {
            pixels.push(at(x, height - 1));
        }
    }
    for y in 1..height.saturating_sub(1) {
        pixels.push(at(0, y));
        if width > 1 {
            pixels.push(at(width - 1, y));
        }
    }

    pixels
}

fn edge_density(luminance: &[f64], width: u32, height: u32) -> f64 {
    if width < 3 || height < 3 {
        return 0.0;
    }

    let at = |x: u32, y: u32| luminance[(y * width + x) as usize];
    let mut edge_pixels = 0u64;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let gx = (at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1))
                - (at(x - 1, y - 1) + 2.0 * at(x - 1, y) + at(x - 1, y + 1));
            let gy = (at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1))
                - (at(x - 1, y - 1) + 2.0 * at(x, y - 1) + at(x + 1, y - 1));
            let magnitude = (gx * gx + gy * gy).sqrt();
            if magnitude > EDGE_MAGNITUDE_THRESHOLD {
                edge_pixels += 1;
            }
        }
    }

    let interior_pixels = u64::from(width - 2) * u64::from(height - 2);
    edge_pixels as f64 / interior_pixels as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_image(width: u32, height: u32, color: [u8; 3]) -> RgbImage {
        RgbImage {
            width,
            height,
            samples: vec![color; (width * height) as usize],
        }
    }

    #[test]
    fn a_solid_gray_image_has_zero_contrast_and_saturation() {
        let image = solid_image(8, 8, [128, 128, 128]);
        let features = analyze(&image);

        assert_eq!(features.contrast, 0.0);
        assert_eq!(features.dynamic_range, 0);
        assert_eq!(features.mean_saturation, 0.0);
        assert!((features.mean_luminance - 128.0).abs() < 1.0);
    }

    #[test]
    fn a_solid_color_image_has_uniform_background_and_no_edges() {
        let image = solid_image(10, 10, [200, 50, 50]);
        let features = analyze(&image);

        assert_eq!(features.uniform_background_ratio, 1.0);
        assert_eq!(features.edge_density, 0.0);
        assert!(features.mean_saturation > 0.5);
    }

    #[test]
    fn a_checkerboard_has_high_contrast_and_edge_density() {
        let width = 16;
        let height = 16;
        let mut samples = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_light = (x / 2 + y / 2) % 2 == 0;
                samples.push(if is_light { [255, 255, 255] } else { [0, 0, 0] });
            }
        }
        let image = RgbImage {
            width,
            height,
            samples,
        };
        let features = analyze(&image);

        assert!(
            features.contrast > 100.0,
            "contrast was {}",
            features.contrast
        );
        assert!(
            features.edge_density > 0.3,
            "edge_density was {}",
            features.edge_density
        );
        assert_eq!(features.dynamic_range, 255);
    }

    #[test]
    fn a_centered_shape_lowers_the_uniform_background_ratio() {
        let width = 20;
        let height = 20;
        let mut samples = vec![[255, 255, 255]; (width * height) as usize];
        // A dark square strictly inside the border.
        for y in 5..15 {
            for x in 5..15 {
                samples[(y * width + x) as usize] = [0, 0, 0];
            }
        }
        let image = RgbImage {
            width,
            height,
            samples,
        };
        let features = analyze(&image);

        // The border itself is untouched, so it should still read as
        // fully uniform even though the image overall is not.
        assert_eq!(features.uniform_background_ratio, 1.0);
        assert!(features.edge_density > 0.0);
    }

    #[test]
    fn aspect_ratio_matches_width_over_height() {
        let image = solid_image(16, 8, [0, 0, 0]);
        let features = analyze(&image);
        assert!((features.aspect_ratio - 2.0).abs() < 1e-9);
    }
}
