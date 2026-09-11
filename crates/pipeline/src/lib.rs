//! Orchestrates the Phase 1/2/3 conversion pipeline (CLAUDE.md
//! Section 6: IMPORTAÇÃO → ANÁLISE HEURÍSTICA → ... → QUANTIZAÇÃO EM
//! TONS → SEGMENTAÇÃO → VETORIZAÇÃO → ... → GERAÇÃO DO SVG →
//! VALIDAÇÃO) by sequencing calls into `crates/imaging`,
//! `crates/analysis`, `crates/quantize`, `crates/vectorize`, and
//! `crates/svggen`. Contains no algorithms of its own — see
//! `docs/architecture.md` ("UI nunca acoplada aos algoritmos").
//!
//! A standalone crate (not part of `src-tauri`) so the conversion core
//! is usable and testable without the desktop UI (CLAUDE.md Section 5)
//! — both the Tauri app and `crates/cli` depend on it.

use laserprep_analysis::{Classification, analyze, classify};
use laserprep_domain::{ToneCount, ToneCountError};
use laserprep_imaging::{ImagingError, RgbImage, load_rgb_from_bytes};
use laserprep_optimize::{
    LaserOptimizationScore, combine_scores, merge_adjacent_regions, order_paths_by_travel_distance,
    score_path_order,
};
use laserprep_presets::{Preset, suggest_preset};
use laserprep_quantize::{QuantizeError, quantize_linear, tone_mean_luminance};
use laserprep_svggen::{SvgOptions, ValidationReport, validate, vectorized_tones_to_svg};
use laserprep_vectorize::{
    BinaryMask, VectorPath, VectorizeError, Vectorizer, VtracerVectorizer,
    polygon_shape_to_vector_path,
};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error(transparent)]
    InvalidToneCount(#[from] ToneCountError),
    #[error(transparent)]
    Imaging(#[from] ImagingError),
    #[error(transparent)]
    Quantize(#[from] QuantizeError),
    #[error(transparent)]
    Vectorize(#[from] VectorizeError),
    #[error("failed to write SVG file: {0}")]
    WriteFile(std::io::Error),
}

/// A generated SVG document plus its [`ValidationReport`] and
/// heuristic [`Classification`], returned together so callers never
/// have to re-derive validation inputs (width/height/tone count)
/// separately from the conversion call that produced them.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub svg: String,
    pub validation: ValidationReport,
    pub classification: Classification,
    /// The preset (CLAUDE.md Section 12) `laserprep_presets` suggests
    /// for `classification.category` — a starting point a caller can
    /// offer to apply, not a decision this pipeline makes for the user.
    pub suggested_preset: Preset,
    /// How much inter-path travel distance the generated document's
    /// path order costs relative to the greedy-optimal order for the
    /// same paths (CLAUDE.md Section 8, `docs/roadmap.md` Phase 5).
    /// Computed per tone layer and combined, regardless of whether
    /// `order_paths` was requested — so a caller can show "already
    /// optimal" or "N% of optimal, reorder to improve" either way.
    pub optimization_score: LaserOptimizationScore,
}

/// A decoded image plus its heuristic [`Classification`] — the part of
/// the pipeline that depends only on the source image bytes, not on
/// any conversion parameter (tone count, minimum area, legend, merge
/// adjacent). Callers that reconvert the same image with new
/// parameters (the desktop UI's Reconvert action, undo/redo) can
/// decode and classify once and reuse this across calls to
/// [`convert_decoded_to_svg`] instead of repeating that work.
pub struct DecodedSource {
    pub image: RgbImage,
    pub classification: Classification,
}

/// Decodes `bytes` and runs the heuristic content classifier
/// (CLAUDE.md Section 6) on the result.
pub fn decode_and_classify(bytes: &[u8]) -> Result<DecodedSource, PipelineError> {
    let image = load_rgb_from_bytes(bytes)?;
    let classification = classify(&analyze(&image));
    Ok(DecodedSource {
        image,
        classification,
    })
}

/// Quantizes, vectorizes, and renders an already-decoded [`DecodedSource`]
/// as an SVG document — one traced `<path>` set per tone level — and
/// validates the result. `min_area_px2` is CLAUDE.md Section 8's
/// "Minimum Area" noise filter (see [`VtracerVectorizer`]);
/// `curve_simplification` is CLAUDE.md Section 8's "Curve
/// Simplification"/"Node Reduction" (see [`VtracerVectorizer`]'s doc
/// comment for why those are one parameter here); `include_legend`
/// adds the optional tone legend (CLAUDE.md Section 10);
/// `merge_adjacent` applies `laserprep_optimize::merge_adjacent_regions`
/// per tone (CLAUDE.md Section 8: "Merge Adjacent Regions"), trading
/// curve fitting for straight-line polygons on the tones it merges
/// (see `laserprep_vectorize::PolygonShape`); `order_paths` applies
/// `laserprep_optimize::order_paths_by_travel_distance` per tone
/// (CLAUDE.md Section 8: "Path Ordering", `docs/roadmap.md` Phase 5)
/// to reduce laser travel between cuts within each tone layer.
#[allow(clippy::too_many_arguments)]
pub fn convert_decoded_to_svg(
    source: &DecodedSource,
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    curve_simplification: f64,
    include_legend: bool,
    merge_adjacent: bool,
    order_paths: bool,
) -> Result<ConversionResult, PipelineError> {
    let tone_count = ToneCount::new(tone_count)?;
    let image = source.image.to_luminance();
    let tone_map = quantize_linear(&image.samples, image.width, image.height, tone_count)?;
    let tone_grays = tone_mean_luminance(&image.samples, &tone_map);

    let vectorizer = VtracerVectorizer::new(min_area_px2, curve_simplification);
    let paths_by_tone: Vec<Vec<VectorPath>> = (0..tone_count.get())
        .map(|tone| -> Result<Vec<VectorPath>, VectorizeError> {
            let mask = BinaryMask::from_tone_map(&tone_map, tone);
            let traced = if merge_adjacent {
                let shapes = vectorizer.trace_polygons(&mask)?;
                merge_adjacent_regions(&shapes)
                    .iter()
                    .map(polygon_shape_to_vector_path)
                    .collect()
            } else {
                vectorizer.trace(&mask)?
            };
            Ok(if order_paths {
                order_paths_by_travel_distance(&traced)
            } else {
                traced
            })
        })
        .collect::<Result<_, _>>()?;

    let optimization_score =
        combine_scores(paths_by_tone.iter().map(|paths| score_path_order(paths)));

    let svg = vectorized_tones_to_svg(
        tone_map.width,
        tone_map.height,
        tone_count,
        &paths_by_tone,
        &tone_grays,
        &SvgOptions {
            dpi,
            include_legend,
        },
    );
    let validation = validate(&svg, tone_map.width, tone_map.height, tone_count.get());
    let suggested_preset = suggest_preset(source.classification.category);

    Ok(ConversionResult {
        svg,
        validation,
        classification: source.classification,
        suggested_preset,
        optimization_score,
    })
}

/// Decodes, classifies, and converts `bytes` in one call — a
/// convenience for callers (such as `crates/cli`) that have no reason
/// to cache the decoded/classified intermediate the way the desktop
/// UI does across Reconvert/undo/redo.
#[allow(clippy::too_many_arguments)]
pub fn convert_bytes_to_svg(
    bytes: &[u8],
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    curve_simplification: f64,
    include_legend: bool,
    merge_adjacent: bool,
    order_paths: bool,
) -> Result<ConversionResult, PipelineError> {
    let source = decode_and_classify(bytes)?;
    convert_decoded_to_svg(
        &source,
        dpi,
        tone_count,
        min_area_px2,
        curve_simplification,
        include_legend,
        merge_adjacent,
        order_paths,
    )
}

/// Writes `svg` to `path`, overwriting any existing file.
pub fn save_svg_to_file(path: &Path, svg: &str) -> Result<(), PipelineError> {
    std::fs::write(path, svg).map_err(PipelineError::WriteFile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::io::Cursor;

    fn convert(
        bytes: &[u8],
        dpi: f64,
        tone_count: u8,
        min_area_px2: u32,
        include_legend: bool,
        merge_adjacent: bool,
    ) -> Result<ConversionResult, PipelineError> {
        convert_bytes_to_svg(
            bytes,
            dpi,
            tone_count,
            min_area_px2,
            laserprep_vectorize::DEFAULT_CURVE_SIMPLIFICATION,
            include_legend,
            merge_adjacent,
            false,
        )
    }

    fn synthetic_png() -> Vec<u8> {
        // A 12x12 image with a solid dark 8x8 square on a light
        // background, large enough to survive vtracer's speckle filter
        // and produce a real traced path per tone.
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(12, 12, Rgb([255, 255, 255]));
        for y in 2..10 {
            for x in 2..10 {
                buffer.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }

        let mut bytes = Vec::new();
        buffer
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn converts_a_decoded_image_end_to_end() {
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap();
        assert!(result.svg.starts_with("<svg "));
        assert!(result.svg.contains("<g id=\"tone-0\""));
        assert!(result.svg.contains("<g id=\"tone-1\""));
        assert!(result.svg.contains("<path"));
    }

    #[test]
    fn renders_each_tone_group_in_a_grayscale_matching_its_source_darkness() {
        // synthetic_png is a dark 8x8 square (tone 0) on a light
        // background (tone 1) — tone 0's fill must be a darker gray
        // than tone 1's, not the flat black both used to render as.
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap();

        let tone_0_fill = result
            .svg
            .split("<g id=\"tone-0\" fill=\"#")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("tone-0 group must declare a fill");
        let tone_1_fill = result
            .svg
            .split("<g id=\"tone-1\" fill=\"#")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("tone-1 group must declare a fill");

        let gray = |hex: &str| u8::from_str_radix(&hex[0..2], 16).unwrap();
        assert!(
            gray(tone_0_fill) < gray(tone_1_fill),
            "tone-0 (#{tone_0_fill}) should be darker than tone-1 (#{tone_1_fill})"
        );
    }

    #[test]
    fn validates_the_generated_document() {
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap();
        assert!(result.validation.has_no_open_paths());
        assert!(result.validation.has_valid_coordinates());
        assert!(result.validation.is_lightburn_compatible());
        // One path for the dark 8x8 square (tone 0) and one for the
        // light background ring around it (tone 1).
        assert_eq!(result.validation.total_paths, 2);
        assert_eq!(result.validation.tone_count, 2);
    }

    #[test]
    fn includes_a_legend_group_when_requested() {
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            true,
            false,
        )
        .unwrap();
        assert!(result.svg.contains("<g id=\"legend\""));
    }

    #[test]
    fn merges_adjacent_regions_when_requested() {
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            true,
        )
        .unwrap();
        assert!(result.svg.contains("<path"));
        assert!(result.validation.has_no_open_paths());
    }

    #[test]
    fn includes_a_heuristic_classification() {
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap();
        assert!((0.0..=1.0).contains(&result.classification.confidence));
    }

    #[test]
    fn includes_a_suggested_preset_matching_the_classification() {
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            result.suggested_preset,
            laserprep_presets::suggest_preset(result.classification.category)
        );
    }

    #[test]
    fn rejects_an_invalid_tone_count() {
        let err = convert(
            &synthetic_png(),
            96.0,
            1,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, PipelineError::InvalidToneCount(_)));
    }

    #[test]
    fn rejects_undecodable_bytes() {
        let err = convert(
            &[0, 1, 2, 3],
            96.0,
            5,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, PipelineError::Imaging(_)));
    }

    #[test]
    fn round_trips_a_file_through_convert_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("input.png");
        std::fs::write(&input_path, synthetic_png()).unwrap();

        let bytes = std::fs::read(&input_path).unwrap();
        let result = convert(
            &bytes,
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap();

        let output_path = dir.path().join("output.svg");
        save_svg_to_file(&output_path, &result.svg).unwrap();

        assert_eq!(std::fs::read_to_string(&output_path).unwrap(), result.svg);
    }

    /// A 60x60 image with three well-separated 8x8 dark squares on a
    /// light background: tone 0 (dark) traces into three disjoint
    /// paths with real travel distance between them to order, unlike
    /// `synthetic_png`'s single square.
    fn scattered_squares_png() -> Vec<u8> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(60, 60, Rgb([255, 255, 255]));
        for (left, top) in [(2, 2), (2, 50), (50, 2)] {
            for y in top..top + 8 {
                for x in left..left + 8 {
                    buffer.put_pixel(x, y, Rgb([0, 0, 0]));
                }
            }
        }

        let mut bytes = Vec::new();
        buffer
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn includes_an_optimization_score_within_bounds() {
        let result = convert(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
            false,
        )
        .unwrap();

        assert!((0.0..=1.0).contains(&result.optimization_score.efficiency));
        assert!(result.optimization_score.travel_distance >= 0.0);
    }

    #[test]
    fn ordering_paths_reaches_full_optimization_efficiency() {
        let source = decode_and_classify(&scattered_squares_png()).unwrap();

        let result = convert_decoded_to_svg(
            &source,
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            laserprep_vectorize::DEFAULT_CURVE_SIMPLIFICATION,
            false,
            false,
            true,
        )
        .unwrap();

        assert_eq!(
            result.validation.total_paths, 4,
            "3 dark squares + 1 background region"
        );
        assert!(
            result.optimization_score.efficiency > 0.999,
            "expected near-1.0 efficiency after ordering, got {}",
            result.optimization_score.efficiency
        );
    }

    /// A staircase boundary (jagged at pixel scale) — the kind of
    /// high-frequency edge real photo texture (skin, fur, JPEG
    /// compression blocks) produces, and what motivated exposing
    /// `curve_simplification` in the first place (a real user's
    /// high-resolution portrait export had one single path with over
    /// 35,000 nodes).
    fn staircase_png() -> Vec<u8> {
        let size = 60u32;
        let mut buffer =
            ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(size, size, Rgb([255, 255, 255]));
        for y in 0..size {
            for x in 0..size {
                if x < y {
                    buffer.put_pixel(x, y, Rgb([0, 0, 0]));
                }
            }
        }
        let mut bytes = Vec::new();
        buffer
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn higher_curve_simplification_reduces_total_nodes_end_to_end() {
        let bytes = staircase_png();

        let detailed = convert_bytes_to_svg(&bytes, 96.0, 2, 1, 0.5, false, false, false).unwrap();
        let simplified =
            convert_bytes_to_svg(&bytes, 96.0, 2, 1, 30.0, false, false, false).unwrap();

        assert!(
            simplified.validation.total_nodes < detailed.validation.total_nodes,
            "expected fewer total nodes at higher curve_simplification: {} (simplified) vs {} (detailed)",
            simplified.validation.total_nodes,
            detailed.validation.total_nodes,
        );
    }
}
