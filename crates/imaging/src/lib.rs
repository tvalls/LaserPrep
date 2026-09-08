//! Raster image decoding into grayscale luminance samples — the
//! IMPORTAÇÃO stage of the pipeline (CLAUDE.md Section 6). Kept
//! separate from quantization, vectorization, and heuristic
//! classification so the core conversion pipeline is usable and
//! testable without the GUI (CLAUDE.md Section 5).

use image::GenericImageView;

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

#[derive(Debug, thiserror::Error)]
pub enum ImagingError {
    #[error("failed to decode image: {0}")]
    Decode(#[from] image::ImageError),
    #[error("image has zero width or height ({width}x{height})")]
    EmptyImage { width: u32, height: u32 },
}

/// Decodes `bytes`, downscales it to fit within [`MAX_DIMENSION`] if
/// needed, and converts the result to grayscale luminance. The image
/// format is auto-detected from its content, not from a file name or
/// extension.
pub fn load_luminance_from_bytes(bytes: &[u8]) -> Result<LuminanceImage, ImagingError> {
    let decoded = image::load_from_memory(bytes)?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return Err(ImagingError::EmptyImage { width, height });
    }

    let decoded = if width > MAX_DIMENSION || height > MAX_DIMENSION {
        decoded.resize(
            MAX_DIMENSION,
            MAX_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        decoded
    };
    let (width, height) = decoded.dimensions();

    Ok(LuminanceImage {
        width,
        height,
        samples: decoded.to_luma8().into_raw(),
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
}
