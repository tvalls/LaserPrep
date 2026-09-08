//! Raster image decoding — the IMPORTAÇÃO stage of the pipeline
//! (CLAUDE.md Section 6). Kept separate from quantization,
//! vectorization, and heuristic classification so the core conversion
//! pipeline is usable and testable without the GUI (CLAUDE.md
//! Section 5).

use image::{DynamicImage, GenericImageView};

/// Images wider or taller than this are downscaled (preserving aspect
/// ratio) before quantization. Bounds working memory and processing
/// time against malformed or unexpectedly huge input files (CLAUDE.md
/// Section 18: "limites de memória, proteção contra arquivos
/// malformados").
pub const MAX_DIMENSION: u32 = 4096;

/// A decoded image reduced to 8-bit luminance samples, row-major, one
/// sample per pixel, ready for [`laserprep_quantize::quantize_linear`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuminanceImage {
    pub width: u32,
    pub height: u32,
    pub samples: Vec<u8>,
}

/// A decoded image as 8-bit RGB samples, row-major, one `[r, g, b]`
/// triple per pixel — for heuristics that need color, not just
/// luminance (CLAUDE.md Section 6: saturation, color clustering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbImage {
    pub width: u32,
    pub height: u32,
    pub samples: Vec<[u8; 3]>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImagingError {
    #[error("failed to decode image: {0}")]
    Decode(#[from] image::ImageError),
    #[error("image has zero width or height ({width}x{height})")]
    EmptyImage { width: u32, height: u32 },
}

/// Decodes `bytes` and downscales it to fit within [`MAX_DIMENSION`]
/// if needed. The image format is auto-detected from its content, not
/// from a file name or extension.
fn decode_and_resize(bytes: &[u8]) -> Result<DynamicImage, ImagingError> {
    let decoded = image::load_from_memory(bytes)?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return Err(ImagingError::EmptyImage { width, height });
    }

    Ok(if width > MAX_DIMENSION || height > MAX_DIMENSION {
        decoded.resize(
            MAX_DIMENSION,
            MAX_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        decoded
    })
}

/// Decodes `bytes` and converts the result to grayscale luminance.
pub fn load_luminance_from_bytes(bytes: &[u8]) -> Result<LuminanceImage, ImagingError> {
    let decoded = decode_and_resize(bytes)?;
    let (width, height) = decoded.dimensions();

    Ok(LuminanceImage {
        width,
        height,
        samples: decoded.to_luma8().into_raw(),
    })
}

/// Decodes `bytes` and keeps the result as RGB (dropping any alpha
/// channel).
pub fn load_rgb_from_bytes(bytes: &[u8]) -> Result<RgbImage, ImagingError> {
    let decoded = decode_and_resize(bytes)?;
    let (width, height) = decoded.dimensions();
    let raw = decoded.to_rgb8().into_raw();

    Ok(RgbImage {
        width,
        height,
        samples: raw.as_chunks::<3>().0.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::io::Cursor;

    fn encode_png(pixels: &[[u8; 3]], width: u32, height: u32) -> Vec<u8> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(width, height);
        for (index, pixel) in pixels.iter().enumerate() {
            let x = index as u32 % width;
            let y = index as u32 / width;
            buffer.put_pixel(x, y, Rgb(*pixel));
        }

        let mut bytes = Vec::new();
        buffer
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .expect("encoding a synthetic test PNG must succeed");
        bytes
    }

    #[test]
    fn decodes_dimensions_and_converts_to_luminance() {
        let bytes = encode_png(&[[0, 0, 0], [255, 255, 255]], 2, 1);
        let decoded = load_luminance_from_bytes(&bytes).unwrap();

        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.samples, vec![0, 255]);
    }

    #[test]
    fn rejects_bytes_that_are_not_a_supported_image() {
        let err = load_luminance_from_bytes(&[1, 2, 3, 4]).unwrap_err();
        assert!(matches!(err, ImagingError::Decode(_)));
    }

    #[test]
    fn downscales_images_wider_than_the_maximum_dimension() {
        let oversized_width = MAX_DIMENSION + 100;
        let buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(oversized_width, 1, Rgb([128; 3]));
        let mut bytes = Vec::new();
        buffer
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();

        let decoded = load_luminance_from_bytes(&bytes).unwrap();
        assert!(decoded.width <= MAX_DIMENSION);
        assert_eq!(
            decoded.samples.len(),
            (decoded.width * decoded.height) as usize
        );
    }

    #[test]
    fn decodes_dimensions_and_keeps_rgb_samples() {
        let bytes = encode_png(&[[10, 20, 30], [200, 100, 50]], 2, 1);
        let decoded = load_rgb_from_bytes(&bytes).unwrap();

        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.samples, vec![[10, 20, 30], [200, 100, 50]]);
    }
}
