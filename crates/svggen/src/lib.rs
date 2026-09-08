//! SVG document generation (per-tone `<g id="tone-N">` groups, optional
//! legend, embedded metadata) and the SVG validator (path counts,
//! closed/open/degenerate paths, self-intersections, node counts,
//! LightBurn-compatibility checks). See CLAUDE.md Sections 9-10 and
//! `docs/lightburn-compatibility.md`. Implemented in Phase 1 (basic SVG)
//! and Phase 2 (legend + validator).
//!
//! Phase 1's artwork is a direct, unoptimized run-length rectangle
//! encoding of the quantized tone map: real vector tracing (via
//! `vtracer`) lands in Phase 2 through `laserprep-vectorize`.

use laserprep_domain::Length;
use laserprep_quantize::ToneMap;

pub struct SvgOptions {
    /// Source image resolution in dots per inch, used to size the SVG
    /// in physical units (CLAUDE.md Section 11: mm is the default unit).
    pub dpi: f64,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self { dpi: 96.0 }
    }
}

/// Renders a [`ToneMap`] as an SVG document: one `<g id="tone-N">` per
/// tone level, each containing the horizontal-run rectangles covering
/// that tone's pixels.
pub fn tone_map_to_svg(tone_map: &ToneMap, options: &SvgOptions) -> String {
    let width_mm = Length::from_pixels(f64::from(tone_map.width), options.dpi).as_millimeters();
    let height_mm = Length::from_pixels(f64::from(tone_map.height), options.dpi).as_millimeters();

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width_mm:.3}mm\" height=\"{height_mm:.3}mm\" viewBox=\"0 0 {w} {h}\">\n",
        w = tone_map.width,
        h = tone_map.height,
    ));
    svg.push_str(&format!(
        "  <metadata>LaserPrep v{version}; tones={tones}; dpi={dpi}; algorithm=quantize-linear+rect-runs</metadata>\n",
        version = env!("CARGO_PKG_VERSION"),
        tones = tone_map.tone_count.get(),
        dpi = options.dpi,
    ));

    for tone in 0..tone_map.tone_count.get() {
        svg.push_str(&format!("  <g id=\"tone-{tone}\">\n"));
        for rect in rects_for_tone(tone_map, tone) {
            svg.push_str(&format!(
                "    <rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" />\n",
                x = rect.x,
                y = rect.y,
                width = rect.width,
                height = rect.height,
            ));
        }
        svg.push_str("  </g>\n");
    }

    svg.push_str("</svg>\n");
    svg
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Collapses each row of `tone_map` into contiguous horizontal runs of
/// the given `tone`, so a solid band of the same tone becomes one
/// `<rect>` instead of one per pixel.
fn rects_for_tone(tone_map: &ToneMap, tone: u8) -> Vec<Rect> {
    let mut rects = Vec::new();

    for y in 0..tone_map.height {
        let row_start = (y * tone_map.width) as usize;
        let row = &tone_map.tones[row_start..row_start + tone_map.width as usize];

        let mut x = 0usize;
        while x < row.len() {
            if row[x] != tone {
                x += 1;
                continue;
            }

            let run_start = x;
            while x < row.len() && row[x] == tone {
                x += 1;
            }

            rects.push(Rect {
                x: run_start as u32,
                y,
                width: (x - run_start) as u32,
                height: 1,
            });
        }
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use laserprep_domain::ToneCount;
    use laserprep_quantize::quantize_linear;

    #[test]
    fn emits_one_group_per_tone_level() {
        let tone_count = ToneCount::new(3).unwrap();
        let map = quantize_linear(&[0, 255, 128, 0], 2, 2, tone_count).unwrap();
        let svg = tone_map_to_svg(&map, &SvgOptions::default());

        for tone in 0..3 {
            assert!(
                svg.contains(&format!("<g id=\"tone-{tone}\">")),
                "missing group for tone {tone} in:\n{svg}"
            );
        }
        assert!(svg.starts_with("<svg "));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn merges_a_solid_row_into_a_single_rect() {
        let tone_count = ToneCount::new(2).unwrap();
        let map = quantize_linear(&[0, 0, 0, 0], 4, 1, tone_count).unwrap();
        let svg = tone_map_to_svg(&map, &SvgOptions::default());

        assert!(svg.contains("<rect x=\"0\" y=\"0\" width=\"4\" height=\"1\" />"));
        assert!(!svg.contains("width=\"1\" height=\"1\""));
    }

    #[test]
    fn splits_alternating_pixels_into_separate_rects() {
        let tone_count = ToneCount::new(2).unwrap();
        let map = quantize_linear(&[0, 255, 0, 255], 4, 1, tone_count).unwrap();

        let dark_rects = rects_for_tone(&map, 0);
        let light_rects = rects_for_tone(&map, 1);
        assert_eq!(dark_rects.len(), 2);
        assert_eq!(light_rects.len(), 2);
    }

    #[test]
    fn sizes_the_viewport_from_dpi() {
        let tone_count = ToneCount::default();
        let map = quantize_linear(&[0; 4], 2, 2, tone_count).unwrap();
        let svg = tone_map_to_svg(&map, &SvgOptions { dpi: 96.0 });

        // 2 px at 96 dpi = 2 / 96 in = 0.5292 mm.
        assert!(svg.contains("width=\"0.529mm\""));
        assert!(svg.contains("viewBox=\"0 0 2 2\""));
    }
}
