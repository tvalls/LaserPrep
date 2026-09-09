//! Orchestrates the Phase 1/2/3 conversion pipeline (CLAUDE.md
//! Section 6: IMPORTAÇÃO → ANÁLISE HEURÍSTICA → ... → QUANTIZAÇÃO EM
//! TONS → SEGMENTAÇÃO → VETORIZAÇÃO → ... → GERAÇÃO DO SVG →
//! VALIDAÇÃO) by sequencing calls into `crates/imaging`,
//! `crates/analysis`, `crates/quantize`, `crates/vectorize`, and
//! `crates/svggen`. Contains no algorithms of its own — see
//! `docs/architecture.md` ("UI nunca acoplada aos algoritmos").

use laserprep_analysis::{Classification, analyze, classify};
use laserprep_domain::{ToneCount, ToneCountError};
use laserprep_imaging::{ImagingError, RgbImage, load_rgb_from_bytes};
use laserprep_optimize::merge_adjacent_regions;
use laserprep_presets::{Preset, suggest_preset};
use laserprep_quantize::{QuantizeError, quantize_linear};
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
/// heuristic [`Classification`], returned together so the UI never
/// has to re-derive validation inputs (width/height/tone count)
/// separately from the conversion call that produced them.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub svg: String,
    pub validation: ValidationReport,
    pub classification: Classification,
    /// The preset (CLAUDE.md Section 12) `laserprep_presets` suggests
    /// for `classification.category` — a starting point the UI can
    /// offer to apply, not a decision this pipeline makes for the user.
    pub suggested_preset: Preset,
}

/// A decoded image plus its heuristic [`Classification`] — the part of
/// the pipeline that depends only on the source image bytes, not on
/// any conversion parameter (tone count, minimum area, legend, merge
/// adjacent). Callers that reconvert the same image with new
/// parameters (the UI's Reconvert action, undo/redo) can decode and
/// classify once and reuse this across calls to
/// [`convert_decoded_to_svg`] instead of repeating that work — see
/// `commands::DecodedSourceCache`.
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
/// `include_legend` adds the optional tone legend (CLAUDE.md Section
/// 10); `merge_adjacent` applies `laserprep_optimize::merge_adjacent_regions`
/// per tone (CLAUDE.md Section 8: "Merge Adjacent Regions"), trading
/// curve fitting for straight-line polygons on the tones it merges
/// (see `laserprep_vectorize::PolygonShape`).
pub fn convert_decoded_to_svg(
    source: &DecodedSource,
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    include_legend: bool,
    merge_adjacent: bool,
) -> Result<ConversionResult, PipelineError> {
    let tone_count = ToneCount::new(tone_count)?;
    let image = source.image.to_luminance();
    let tone_map = quantize_linear(&image.samples, image.width, image.height, tone_count)?;

    let vectorizer = VtracerVectorizer::new(min_area_px2);
    let paths_by_tone: Vec<Vec<VectorPath>> = (0..tone_count.get())
        .map(|tone| {
            let mask = BinaryMask::from_tone_map(&tone_map, tone);
            if merge_adjacent {
                let shapes = vectorizer.trace_polygons(&mask)?;
                Ok(merge_adjacent_regions(&shapes)
                    .iter()
                    .map(polygon_shape_to_vector_path)
                    .collect())
            } else {
                vectorizer.trace(&mask)
            }
        })
        .collect::<Result<_, _>>()?;

    let svg = vectorized_tones_to_svg(
        tone_map.width,
        tone_map.height,
        tone_count,
        &paths_by_tone,
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
    })
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

    /// Decodes, classifies, and converts `bytes` in one call — what
    /// `pipeline::convert_bytes_to_svg` used to do before decode/
    /// classify and the parameter-dependent stages were split apart
    /// for `commands::DecodedSourceState` to cache the former. Kept
    /// test-only since production callers now have a real reason to
    /// call the two steps separately (`commands.rs`), and a `pub`
    /// wrapper with no other caller is exactly the dead code clippy's
    /// `-D warnings` (rightly) rejects.
    fn convert(
        bytes: &[u8],
        dpi: f64,
        tone_count: u8,
        min_area_px2: u32,
        include_legend: bool,
        merge_adjacent: bool,
    ) -> Result<ConversionResult, PipelineError> {
        let source = decode_and_classify(bytes)?;
        convert_decoded_to_svg(
            &source,
            dpi,
            tone_count,
            min_area_px2,
            include_legend,
            merge_adjacent,
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
        assert!(result.svg.contains("<g id=\"tone-0\">"));
        assert!(result.svg.contains("<g id=\"tone-1\">"));
        assert!(result.svg.contains("<path"));
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
}
