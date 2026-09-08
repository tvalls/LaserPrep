//! SVG document generation (per-tone `<g id="tone-N">` groups, optional
//! legend, embedded metadata) and the SVG validator (path counts,
//! closed/open/degenerate paths, self-intersections, node counts,
//! LightBurn-compatibility checks). See CLAUDE.md Sections 9-10 and
//! `docs/lightburn-compatibility.md`. Implemented in Phase 1 (basic SVG)
//! and Phase 2 (real vectorization, legend, validator).

use laserprep_domain::{Length, ToneCount};
use laserprep_vectorize::VectorPath;

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

/// Renders vectorized tone layers as an SVG document: one
/// `<g id="tone-N">` per tone level (0..`tone_count`), containing that
/// tone's traced `<path>` elements. `paths_by_tone[tone]` holds the
/// paths for that tone; a missing or out-of-range index is treated as
/// an empty (but still present) group.
pub fn vectorized_tones_to_svg(
    width: u32,
    height: u32,
    tone_count: ToneCount,
    paths_by_tone: &[Vec<VectorPath>],
    options: &SvgOptions,
) -> String {
    let width_mm = Length::from_pixels(f64::from(width), options.dpi).as_millimeters();
    let height_mm = Length::from_pixels(f64::from(height), options.dpi).as_millimeters();

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width_mm:.3}mm\" height=\"{height_mm:.3}mm\" viewBox=\"0 0 {width} {height}\">\n",
    ));
    svg.push_str(&format!(
        "  <metadata>LaserPrep v{version}; tones={tones}; dpi={dpi}; algorithm=quantize-linear+vtracer</metadata>\n",
        version = env!("CARGO_PKG_VERSION"),
        tones = tone_count.get(),
        dpi = options.dpi,
    ));

    for tone in 0..tone_count.get() {
        svg.push_str(&format!("  <g id=\"tone-{tone}\">\n"));
        if let Some(paths) = paths_by_tone.get(tone as usize) {
            for path in paths {
                svg.push_str("    ");
                svg.push_str(&path.svg_element);
            }
        }
        svg.push_str("  </g>\n");
    }

    svg.push_str("</svg>\n");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(d: &str) -> VectorPath {
        VectorPath {
            svg_element: format!("<path d=\"{d}\" fill=\"#000000\"/>\n"),
        }
    }

    #[test]
    fn emits_one_group_per_tone_level_even_when_empty() {
        let tone_count = ToneCount::new(3).unwrap();
        let paths_by_tone = vec![vec![path("M0 0")], vec![], vec![]];
        let svg = vectorized_tones_to_svg(2, 2, tone_count, &paths_by_tone, &SvgOptions::default());

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
    fn embeds_each_tones_paths_verbatim() {
        let tone_count = ToneCount::new(2).unwrap();
        let paths_by_tone = vec![vec![path("M0 0 L1 1")], vec![path("M2 2 L3 3")]];
        let svg = vectorized_tones_to_svg(4, 4, tone_count, &paths_by_tone, &SvgOptions::default());

        assert!(svg.contains("d=\"M0 0 L1 1\""));
        assert!(svg.contains("d=\"M2 2 L3 3\""));
    }

    #[test]
    fn tolerates_fewer_path_lists_than_tones() {
        let tone_count = ToneCount::new(4).unwrap();
        let paths_by_tone = vec![vec![path("M0 0")]];
        let svg = vectorized_tones_to_svg(2, 2, tone_count, &paths_by_tone, &SvgOptions::default());

        for tone in 0..4 {
            assert!(svg.contains(&format!("<g id=\"tone-{tone}\">")));
        }
    }

    #[test]
    fn sizes_the_viewport_from_dpi() {
        let tone_count = ToneCount::default();
        let svg = vectorized_tones_to_svg(2, 2, tone_count, &[], &SvgOptions { dpi: 96.0 });

        // 2 px at 96 dpi = 2 / 96 in = 0.5292 mm.
        assert!(svg.contains("width=\"0.529mm\""));
        assert!(svg.contains("viewBox=\"0 0 2 2\""));
    }
}
