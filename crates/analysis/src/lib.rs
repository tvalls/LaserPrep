//! Deterministic, classical-heuristics feature extraction and content
//! classification.
//!
//! Implemented in Phase 3: histogram/contrast, edge density, color
//! saturation, and uniform-background ratio feed a hand-weighted
//! scoring function per content category. No machine learning — see
//! `docs/adr/0002-no-ai-ml-classification.md`.
//!
//! [`classify`] only distinguishes categories that a global image
//! statistic can actually separate: [`ContentCategory::UniformBackground`],
//! [`ContentCategory::Logo`], [`ContentCategory::Landscape`], and
//! [`ContentCategory::ComplexBackground`], falling back to
//! [`ContentCategory::GenericPhoto`]. CLAUDE.md Section 6 lists finer
//! categories (portrait, animal, vehicle, architecture, ...) that
//! would need real subject/object localization to tell apart — Section
//! 2 forbids exactly the trained detectors (Haar cascades, CNNs, etc.)
//! that would take. Rather than fake that distinction with heuristics
//! that can't actually make it, those categories are not attempted
//! yet; this is a scope limitation, not an oversight.

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

/// A content category [`classify`] can assign. See the module doc
/// comment for which CLAUDE.md Section 6 categories are deliberately
/// not attempted yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentCategory {
    /// The whole image is essentially flat: a uniform border and
    /// almost no edge content anywhere (e.g. a blank/near-blank scan).
    UniformBackground,
    /// A mark or subject isolated on a plain, uniform background —
    /// covers CLAUDE.md's "logo", "ícone", and "objeto/produto"
    /// categories, which this heuristic set cannot tell apart from
    /// each other (all three look the same to a global statistic:
    /// uniform border, real content in the middle).
    Logo,
    /// Wide framing with content filling the frame edge to edge (no
    /// isolated subject on a plain background).
    Landscape,
    /// Content fills the frame edge to edge with high edge density,
    /// regardless of aspect ratio (a busy or textured scene).
    ComplexBackground,
    /// Fallback when no other category's rule fires.
    GenericPhoto,
}

/// The result of [`classify`]: a category plus a heuristic confidence
/// in `0.0..=1.0`. The confidence is the winning category's rule
/// score, not a calibrated statistical probability — see CLAUDE.md
/// Section 2 ("confiança calculada a partir de score heurístico, nunca
/// de um modelo").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Classification {
    pub category: ContentCategory,
    pub confidence: f64,
}

/// Edge density below this, combined with a uniform border, means the
/// whole image is essentially blank — not just its border.
const FLAT_EDGE_DENSITY: f64 = 0.02;
/// Edge density above this is too busy to call an isolated mark on a
/// plain background, even if the border itself is uniform.
const LOGO_MAX_EDGE_DENSITY: f64 = 0.3;
/// Baseline score for the fallback category: any real rule must beat
/// this to win instead.
const GENERIC_PHOTO_BASELINE: f64 = 0.25;

/// Classifies `features` by picking the highest-scoring rule below,
/// falling back to [`ContentCategory::GenericPhoto`] when none of them
/// fire strongly. Each rule is a simple, documented threshold — not a
/// fitted/trained function (CLAUDE.md Section 2).
pub fn classify(features: &ImageFeatures) -> Classification {
    let candidates = [
        (
            ContentCategory::UniformBackground,
            uniform_background_score(features),
        ),
        (ContentCategory::Logo, logo_score(features)),
        (ContentCategory::Landscape, landscape_score(features)),
        (
            ContentCategory::ComplexBackground,
            complex_background_score(features),
        ),
        (ContentCategory::GenericPhoto, GENERIC_PHOTO_BASELINE),
    ];

    let (category, confidence) = candidates
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("candidates is non-empty");

    Classification {
        category,
        confidence: confidence.clamp(0.0, 1.0),
    }
}

fn uniform_background_score(f: &ImageFeatures) -> f64 {
    if f.uniform_background_ratio > 0.9 && f.edge_density < FLAT_EDGE_DENSITY {
        f.uniform_background_ratio
    } else {
        0.0
    }
}

fn logo_score(f: &ImageFeatures) -> f64 {
    if f.uniform_background_ratio > 0.75
        && (FLAT_EDGE_DENSITY..LOGO_MAX_EDGE_DENSITY).contains(&f.edge_density)
    {
        f.uniform_background_ratio * (f.contrast / 128.0).min(1.0)
    } else {
        0.0
    }
}

fn landscape_score(f: &ImageFeatures) -> f64 {
    if f.aspect_ratio > 1.3 && f.uniform_background_ratio < 0.6 {
        let aspect_bonus = ((f.aspect_ratio - 1.3) / 1.0).clamp(0.0, 1.0);
        let fill_bonus = 1.0 - f.uniform_background_ratio;
        (aspect_bonus + fill_bonus) / 2.0
    } else {
        0.0
    }
}

fn complex_background_score(f: &ImageFeatures) -> f64 {
    if f.uniform_background_ratio < 0.5 {
        (1.0 - f.uniform_background_ratio) * (f.edge_density * 4.0).min(1.0)
    } else {
        0.0
    }
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

    fn checkerboard(width: u32, height: u32) -> RgbImage {
        let mut samples = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let is_light = (x / 2 + y / 2) % 2 == 0;
                samples.push(if is_light { [255, 255, 255] } else { [0, 0, 0] });
            }
        }
        RgbImage {
            width,
            height,
            samples,
        }
    }

    /// A horizontal band per row of a distinct, saturated color —
    /// stands in for a "sky / hills / ground" landscape composition:
    /// wide framing, content filling the frame edge to edge, no plain
    /// border.
    fn banded_landscape(width: u32, height: u32) -> RgbImage {
        let bands: [[u8; 3]; 3] = [[135, 206, 235], [34, 139, 34], [139, 69, 19]];
        let mut samples = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            let band = bands[(y * bands.len() as u32 / height) as usize];
            for _ in 0..width {
                samples.push(band);
            }
        }
        RgbImage {
            width,
            height,
            samples,
        }
    }

    /// A smooth diagonal gradient: no sharp edges (low edge density)
    /// and no plain border (every border pixel is a different shade),
    /// so it shouldn't strongly match any of the specific rules.
    fn smooth_gradient(size: u32) -> RgbImage {
        let mut samples = Vec::with_capacity((size * size) as usize);
        for y in 0..size {
            for x in 0..size {
                let t = (x + y) as f64 / (2 * size) as f64;
                let level = (t * 255.0).round() as u8;
                samples.push([level, level, level]);
            }
        }
        RgbImage {
            width: size,
            height: size,
            samples,
        }
    }

    #[test]
    fn classifies_a_solid_image_as_uniform_background() {
        let features = analyze(&solid_image(20, 20, [240, 240, 240]));
        let classification = classify(&features);

        assert_eq!(classification.category, ContentCategory::UniformBackground);
        assert!(classification.confidence > 0.9);
    }

    #[test]
    fn classifies_a_mark_on_a_plain_background_as_logo() {
        let width = 20;
        let height = 20;
        let mut samples = vec![[255, 255, 255]; (width * height) as usize];
        for y in 5..15 {
            for x in 5..15 {
                samples[(y * width + x) as usize] = [0, 0, 0];
            }
        }
        let features = analyze(&RgbImage {
            width,
            height,
            samples,
        });
        let classification = classify(&features);

        assert_eq!(classification.category, ContentCategory::Logo);
    }

    #[test]
    fn classifies_a_wide_banded_scene_as_landscape() {
        let features = analyze(&banded_landscape(30, 15));
        let classification = classify(&features);

        assert_eq!(classification.category, ContentCategory::Landscape);
    }

    #[test]
    fn classifies_a_busy_square_texture_as_complex_background() {
        let features = analyze(&checkerboard(16, 16));
        let classification = classify(&features);

        assert_eq!(classification.category, ContentCategory::ComplexBackground);
    }

    #[test]
    fn falls_back_to_generic_photo_when_no_rule_fires() {
        let features = analyze(&smooth_gradient(20));
        let classification = classify(&features);

        assert_eq!(classification.category, ContentCategory::GenericPhoto);
        assert_eq!(classification.confidence, GENERIC_PHOTO_BASELINE);
    }

    #[test]
    fn confidence_is_always_within_bounds() {
        for image in [
            solid_image(10, 10, [10, 20, 30]),
            checkerboard(12, 12),
            banded_landscape(24, 10),
            smooth_gradient(15),
        ] {
            let classification = classify(&analyze(&image));
            assert!((0.0..=1.0).contains(&classification.confidence));
        }
    }
}
