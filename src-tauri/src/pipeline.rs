//! Orchestrates the Phase 1 conversion pipeline (CLAUDE.md Section 6:
//! IMPORTAÇÃO → ... → QUANTIZAÇÃO EM TONS → ... → GERAÇÃO DO SVG) by
//! sequencing calls into `crates/imaging`, `crates/quantize`, and
//! `crates/svggen`. Contains no algorithms of its own — see
//! `docs/architecture.md` ("UI nunca acoplada aos algoritmos").

use laserprep_domain::{ToneCount, ToneCountError};
use laserprep_imaging::{ImagingError, load_luminance_from_bytes};
use laserprep_quantize::{QuantizeError, quantize_linear};
use laserprep_svggen::{SvgOptions, tone_map_to_svg};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error(transparent)]
    InvalidToneCount(#[from] ToneCountError),
    #[error(transparent)]
    Imaging(#[from] ImagingError),
    #[error(transparent)]
    Quantize(#[from] QuantizeError),
    #[error("failed to read image file: {0}")]
    ReadFile(std::io::Error),
    #[error("failed to write SVG file: {0}")]
    WriteFile(std::io::Error),
}

/// Decodes, quantizes, and renders `bytes` as an SVG document.
pub fn convert_bytes_to_svg(
    bytes: &[u8],
    dpi: f64,
    tone_count: u8,
) -> Result<String, PipelineError> {
    let tone_count = ToneCount::new(tone_count)?;
    let image = load_luminance_from_bytes(bytes)?;
    let tone_map = quantize_linear(&image.samples, image.width, image.height, tone_count)?;
    Ok(tone_map_to_svg(&tone_map, &SvgOptions { dpi }))
}

/// Reads the image at `path` and runs [`convert_bytes_to_svg`] on it.
pub fn convert_file_to_svg(path: &Path, dpi: f64, tone_count: u8) -> Result<String, PipelineError> {
    let bytes = std::fs::read(path).map_err(PipelineError::ReadFile)?;
    convert_bytes_to_svg(&bytes, dpi, tone_count)
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
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(2, 2);
        buffer.put_pixel(0, 0, Rgb([0, 0, 0]));
        buffer.put_pixel(1, 0, Rgb([255, 255, 255]));
        buffer.put_pixel(0, 1, Rgb([0, 0, 0]));
        buffer.put_pixel(1, 1, Rgb([255, 255, 255]));

        let mut bytes = Vec::new();
        buffer
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn converts_a_decoded_image_end_to_end() {
        let svg = convert_bytes_to_svg(&synthetic_png(), 96.0, 2).unwrap();
        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("<g id=\"tone-0\">"));
        assert!(svg.contains("<g id=\"tone-1\">"));
    }

    #[test]
    fn rejects_an_invalid_tone_count() {
        let err = convert_bytes_to_svg(&synthetic_png(), 96.0, 1).unwrap_err();
        assert!(matches!(err, PipelineError::InvalidToneCount(_)));
    }

    #[test]
    fn rejects_undecodable_bytes() {
        let err = convert_bytes_to_svg(&[0, 1, 2, 3], 96.0, 5).unwrap_err();
        assert!(matches!(err, PipelineError::Imaging(_)));
    }

    #[test]
    fn round_trips_a_file_through_convert_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("input.png");
        std::fs::write(&input_path, synthetic_png()).unwrap();

        let svg = convert_file_to_svg(&input_path, 96.0, 2).unwrap();

        let output_path = dir.path().join("output.svg");
        save_svg_to_file(&output_path, &svg).unwrap();

        assert_eq!(std::fs::read_to_string(&output_path).unwrap(), svg);
    }
}
