//! Raster-to-SVG tracing.
//!
//! Implemented in Phase 2. Exposes our own [`Vectorizer`] trait and
//! [`VectorPath`]/[`BinaryMask`] types so the tracing backend
//! (`vtracer`, see `docs/adr/0003-vectorization-library.md`) is never
//! depended on directly by other crates — only [`VtracerVectorizer`]'s
//! implementation touches the `vtracer`/`visioncortex` crates.

use laserprep_quantize::ToneMap;
use visioncortex::{CompoundPathElement, PathSimplifyMode};
use vtracer::{ColorImage, ColorMode, Config, Preset};

/// A row-major boolean mask over an image: `true` marks a pixel that
/// belongs to the tone being traced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryMask {
    pub width: u32,
    pub height: u32,
    pub foreground: Vec<bool>,
}

impl BinaryMask {
    /// Builds the mask of pixels in `tone_map` equal to `tone`.
    pub fn from_tone_map(tone_map: &ToneMap, tone: u8) -> Self {
        Self {
            width: tone_map.width,
            height: tone_map.height,
            foreground: tone_map.tones.iter().map(|&t| t == tone).collect(),
        }
    }
}

/// One traced region, already a complete, renderable SVG `<path>`
/// element (its `d`, `fill`, and any coordinate-offset `transform`
/// attributes are already resolved) — callers embed it verbatim; they
/// never parse or reconstruct path data themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorPath {
    pub svg_element: String,
}

/// One traced region as raw straight-line polygon geometry: a list of
/// contours, each a closed ring of `[x, y]` points. The first contour
/// is the exterior boundary; any further contours are holes. This
/// matches both SVG's nonzero-fill-rule hole convention and
/// `i_overlay`'s `Shape` format directly (see `crates/optimize`) — no
/// winding-direction correction is needed between the two.
///
/// Unlike [`VectorPath`], this loses curve fitting (straight
/// line segments only); it exists for `crates/optimize` to perform
/// boolean operations on, not for direct SVG rendering.
pub type PolygonShape = Vec<Vec<[f64; 2]>>;

/// Renders a [`PolygonShape`] as a [`VectorPath`], using the same
/// `M x y L x y ... Z` command style and black fill as
/// [`VtracerVectorizer::trace`]'s curve-based output, so the two are
/// interchangeable as far as `crates/svggen` is concerned.
pub fn polygon_shape_to_vector_path(shape: &PolygonShape) -> VectorPath {
    let mut data = String::new();
    for contour in shape {
        let mut points = contour.iter();
        if let Some([x, y]) = points.next() {
            data.push_str(&format!("M{x} {y} "));
        }
        for [x, y] in points {
            data.push_str(&format!("L{x} {y} "));
        }
        data.push_str("Z ");
    }

    VectorPath {
        svg_element: format!("<path d=\"{}\" fill=\"#000000\"/>\n", data.trim_end()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VectorizeError {
    #[error("mask buffer has {actual} samples, expected {expected} for a {width}x{height} mask")]
    SizeMismatch {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
    #[error("tracing failed: {0}")]
    Trace(String),
}

/// Traces a [`BinaryMask`] into vector paths. Implemented by
/// [`VtracerVectorizer`]; kept as a trait so the tracing backend can be
/// swapped later without touching callers (docs/adr/0003).
pub trait Vectorizer {
    fn trace(&self, mask: &BinaryMask) -> Result<Vec<VectorPath>, VectorizeError>;
}

/// vtracer's `Bw` preset's default speckle filter (`filter_speckle: 4`,
/// squared into an area internally) — kept as our own named constant
/// so callers aren't left guessing where `16` comes from.
pub const DEFAULT_MIN_AREA_PX2: u32 = 16;

/// The default [`Vectorizer`], backed by `vtracer` in binary-tracing
/// mode (one call per tone mask).
///
/// `min_area_px2` is CLAUDE.md Section 8's exposed "Minimum Area"
/// parameter: connected regions smaller than this (in pixels²) are
/// discarded as noise before tracing, rather than becoming a tiny
/// stray path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VtracerVectorizer {
    min_area_px2: u32,
}

impl VtracerVectorizer {
    pub fn new(min_area_px2: u32) -> Self {
        Self { min_area_px2 }
    }
}

impl Default for VtracerVectorizer {
    fn default() -> Self {
        Self::new(DEFAULT_MIN_AREA_PX2)
    }
}

impl VtracerVectorizer {
    fn color_image(mask: &BinaryMask) -> Result<ColorImage, VectorizeError> {
        let expected = mask.width as usize * mask.height as usize;
        if mask.foreground.len() != expected {
            return Err(VectorizeError::SizeMismatch {
                width: mask.width,
                height: mask.height,
                expected,
                actual: mask.foreground.len(),
            });
        }

        // vtracer's binary tracing mode treats pixels with r < 128 as
        // foreground, so foreground -> black, background -> white.
        let mut pixels = Vec::with_capacity(expected * 4);
        for &is_foreground in &mask.foreground {
            let channel = if is_foreground { 0u8 } else { 255u8 };
            pixels.extend_from_slice(&[channel, channel, channel, 255]);
        }

        Ok(ColorImage {
            pixels,
            width: mask.width as usize,
            height: mask.height as usize,
        })
    }

    fn base_config(&self) -> Config {
        let mut config = Config::from_preset(Preset::Bw);
        config.color_mode = ColorMode::Binary;
        // Config::filter_speckle is a side length in pixels; vtracer
        // squares it internally into the area threshold we expose.
        config.filter_speckle = (f64::from(self.min_area_px2).sqrt().ceil() as usize).max(1);
        config
    }

    /// Traces `mask` into raw straight-line polygon geometry (see
    /// [`PolygonShape`]) instead of curve-fitted SVG paths — the input
    /// `crates/optimize` needs to perform boolean operations such as
    /// merging adjacent regions (docs/adr/0004-geometry-boolean-ops.md).
    pub fn trace_polygons(&self, mask: &BinaryMask) -> Result<Vec<PolygonShape>, VectorizeError> {
        let image = Self::color_image(mask)?;
        let mut config = self.base_config();
        config.mode = PathSimplifyMode::Polygon;

        let svg = vtracer::convert(image, config).map_err(VectorizeError::Trace)?;

        Ok(svg
            .paths
            .into_iter()
            .map(|svg_path| {
                svg_path
                    .path
                    .iter()
                    .filter_map(|element| match element {
                        CompoundPathElement::PathI32(path) => Some(
                            path.iter()
                                .map(|point| [f64::from(point.x), f64::from(point.y)])
                                .collect(),
                        ),
                        // Polygon mode only ever produces PathI32
                        // elements; other variants can't occur here.
                        CompoundPathElement::PathF64(_) | CompoundPathElement::Spline(_) => None,
                    })
                    .collect()
            })
            .collect())
    }
}

impl Vectorizer for VtracerVectorizer {
    fn trace(&self, mask: &BinaryMask) -> Result<Vec<VectorPath>, VectorizeError> {
        let image = Self::color_image(mask)?;
        let config = self.base_config();

        let svg = vtracer::convert(image, config).map_err(VectorizeError::Trace)?;

        Ok(svg
            .paths
            .into_iter()
            .map(|path| VectorPath {
                svg_element: path.to_string(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laserprep_domain::ToneCount;
    use laserprep_quantize::quantize_linear;

    fn solid_square_mask(width: u32, height: u32) -> BinaryMask {
        let mut foreground = vec![false; (width * height) as usize];
        for y in 2..height - 2 {
            for x in 2..width - 2 {
                foreground[(y * width + x) as usize] = true;
            }
        }
        BinaryMask {
            width,
            height,
            foreground,
        }
    }

    #[test]
    fn traces_a_solid_square_into_one_path() {
        let mask = solid_square_mask(12, 12);
        let paths = VtracerVectorizer::default().trace(&mask).unwrap();

        assert_eq!(paths.len(), 1);
        assert!(paths[0].svg_element.contains("<path"));
    }

    #[test]
    fn an_all_background_mask_produces_no_paths() {
        let mask = BinaryMask {
            width: 4,
            height: 4,
            foreground: vec![false; 16],
        };
        let paths = VtracerVectorizer::default().trace(&mask).unwrap();
        assert!(paths.is_empty());
    }

    #[test]
    fn rejects_a_mismatched_mask_buffer() {
        let mask = BinaryMask {
            width: 4,
            height: 4,
            foreground: vec![false; 10],
        };
        let err = VtracerVectorizer::default().trace(&mask).unwrap_err();
        assert!(matches!(err, VectorizeError::SizeMismatch { .. }));
    }

    #[test]
    fn filters_out_regions_smaller_than_the_minimum_area() {
        // A 2x2 (4px²) speckle on an otherwise empty 10x10 mask.
        let width = 10;
        let height = 10;
        let mut foreground = vec![false; (width * height) as usize];
        foreground[(3 * width + 3) as usize] = true;
        foreground[(3 * width + 4) as usize] = true;
        foreground[(4 * width + 3) as usize] = true;
        foreground[(4 * width + 4) as usize] = true;
        let mask = BinaryMask {
            width,
            height,
            foreground,
        };

        let filtered = VtracerVectorizer::new(16).trace(&mask).unwrap();
        assert!(
            filtered.is_empty(),
            "a 4px speckle should be filtered out at min_area=16"
        );

        let kept = VtracerVectorizer::new(1).trace(&mask).unwrap();
        assert_eq!(
            kept.len(),
            1,
            "the same speckle should survive at min_area=1"
        );
    }

    #[test]
    fn traces_polygons_as_a_single_closed_contour() {
        let mask = solid_square_mask(12, 12);
        let shapes = VtracerVectorizer::default().trace_polygons(&mask).unwrap();

        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 1, "a solid square has no holes");
        let contour = &shapes[0][0];
        assert!(
            contour.len() >= 4,
            "a square boundary needs at least 4 points"
        );
        for [x, y] in contour {
            assert!(x.is_finite() && y.is_finite());
        }
    }

    #[test]
    fn traces_a_ring_shape_as_exterior_plus_hole_contours() {
        // A 12x12 mask with a 4-pixel-wide ring: foreground everywhere
        // except a hole punched in the middle.
        let width = 12;
        let height = 12;
        let mut foreground = vec![true; (width * height) as usize];
        for y in 4..8 {
            for x in 4..8 {
                foreground[(y * width + x) as usize] = false;
            }
        }
        let mask = BinaryMask {
            width,
            height,
            foreground,
        };

        let shapes = VtracerVectorizer::default().trace_polygons(&mask).unwrap();

        assert_eq!(shapes.len(), 1, "the ring is a single connected region");
        assert_eq!(
            shapes[0].len(),
            2,
            "expected one exterior contour and one hole, got: {:?}",
            shapes[0]
        );
    }

    #[test]
    fn renders_a_polygon_shape_as_a_closed_svg_path() {
        let shape: PolygonShape = vec![vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]]];
        let path = polygon_shape_to_vector_path(&shape);

        assert!(path.svg_element.starts_with("<path d=\"M0 0 "));
        assert!(path.svg_element.contains("L4 0"));
        assert!(path.svg_element.contains("Z\""));
    }

    #[test]
    fn renders_holes_as_additional_contours() {
        let shape: PolygonShape = vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            vec![[3.0, 3.0], [3.0, 7.0], [7.0, 7.0], [7.0, 3.0]],
        ];
        let path = polygon_shape_to_vector_path(&shape);

        // Both the exterior and the hole become their own M...Z cycle
        // within the same `d` attribute (nonzero-fill-rule hole).
        assert_eq!(path.svg_element.matches('M').count(), 2);
        assert_eq!(path.svg_element.matches('Z').count(), 2);
    }

    #[test]
    fn builds_a_mask_from_a_quantized_tone_map() {
        let tone_count = ToneCount::new(2).unwrap();
        let tone_map = quantize_linear(&[0, 255, 0, 255], 2, 2, tone_count).unwrap();

        let dark = BinaryMask::from_tone_map(&tone_map, 0);
        assert_eq!(dark.foreground, vec![true, false, true, false]);

        let light = BinaryMask::from_tone_map(&tone_map, 1);
        assert_eq!(light.foreground, vec![false, true, false, true]);
    }
}
