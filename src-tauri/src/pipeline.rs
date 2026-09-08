//! Orchestrates the Phase 1/2 conversion pipeline (CLAUDE.md Section 6:
//! IMPORTAÇÃO → ... → QUANTIZAÇÃO EM TONS → SEGMENTAÇÃO → VETORIZAÇÃO →
//! ... → GERAÇÃO DO SVG → VALIDAÇÃO) by sequencing calls into
//! `crates/imaging`, `crates/quantize`, `crates/vectorize`, and
//! `crates/svggen`. Contains no algorithms of its own — see
//! `docs/architecture.md` ("UI nunca acoplada aos algoritmos").

use laserprep_domain::{ToneCount, ToneCountError};
use laserprep_imaging::{ImagingError, load_luminance_from_bytes};
use laserprep_quantize::{QuantizeError, quantize_linear};
use laserprep_svggen::{SvgOptions, ValidationReport, validate, vectorized_tones_to_svg};
use laserprep_vectorize::{BinaryMask, VectorPath, VectorizeError, Vectorizer, VtracerVectorizer};
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
    #[error("failed to read image file: {0}")]
    ReadFile(std::io::Error),
    #[error("failed to write SVG file: {0}")]
    WriteFile(std::io::Error),
}

/// A generated SVG document plus its [`ValidationReport`], returned
/// together so the UI never has to re-derive validation inputs
/// (width/height/tone count) separately from the conversion call that
/// produced them.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub svg: String,
    pub validation: ValidationReport,
}

/// Decodes, quantizes, vectorizes, and renders `bytes` as an SVG
/// document — one traced `<path>` set per tone level — and validates
/// the result. `min_area_px2` is CLAUDE.md Section 8's "Minimum Area"
/// noise filter (see [`VtracerVectorizer`]); `include_legend` adds the
/// optional tone legend (CLAUDE.md Section 10).
pub fn convert_bytes_to_svg(
    bytes: &[u8],
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    include_legend: bool,
) -> Result<ConversionResult, PipelineError> {
    let tone_count = ToneCount::new(tone_count)?;
    let image = load_luminance_from_bytes(bytes)?;
    let tone_map = quantize_linear(&image.samples, image.width, image.height, tone_count)?;

    let vectorizer = VtracerVectorizer::new(min_area_px2);
    let paths_by_tone: Vec<Vec<VectorPath>> = (0..tone_count.get())
        .map(|tone| {
            let mask = BinaryMask::from_tone_map(&tone_map, tone);
            vectorizer.trace(&mask)
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

    Ok(ConversionResult { svg, validation })
}

/// Reads the image at `path` and runs [`convert_bytes_to_svg`] on it.
pub fn convert_file_to_svg(
    path: &Path,
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    include_legend: bool,
) -> Result<ConversionResult, PipelineError> {
    let bytes = std::fs::read(path).map_err(PipelineError::ReadFile)?;
    convert_bytes_to_svg(&bytes, dpi, tone_count, min_area_px2, include_legend)
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
        let result = convert_bytes_to_svg(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
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
        let result = convert_bytes_to_svg(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
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
        let result = convert_bytes_to_svg(
            &synthetic_png(),
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            true,
        )
        .unwrap();
        assert!(result.svg.contains("<g id=\"legend\""));
    }

    #[test]
    fn rejects_an_invalid_tone_count() {
        let err = convert_bytes_to_svg(
            &synthetic_png(),
            96.0,
            1,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, PipelineError::InvalidToneCount(_)));
    }

    #[test]
    fn rejects_undecodable_bytes() {
        let err = convert_bytes_to_svg(
            &[0, 1, 2, 3],
            96.0,
            5,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
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

        let result = convert_file_to_svg(
            &input_path,
            96.0,
            2,
            laserprep_vectorize::DEFAULT_MIN_AREA_PX2,
            false,
        )
        .unwrap();

        let output_path = dir.path().join("output.svg");
        save_svg_to_file(&output_path, &result.svg).unwrap();

        assert_eq!(std::fs::read_to_string(&output_path).unwrap(), result.svg);
    }
}
