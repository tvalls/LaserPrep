//! SVG validator (CLAUDE.md Section 9): counts total/closed/open/
//! degenerate paths and total nodes, flags non-finite coordinates, and
//! checks for SVG features LightBurn does not import reliably
//! (`docs/lightburn-compatibility.md`).
//!
//! Self-intersection detection and "duplicates removed"/"tiny areas
//! removed" counts are not implemented here: the latter two describe
//! what `crates/optimize` removes, and that crate has no logic yet
//! (Phase 2 roadmap item, tracked separately, not faked here).
//! Self-intersection detection needs real segment/curve geometry,
//! which `VectorPath` (a plain `<path>` element string, by design —
//! see `docs/adr/0003-vectorization-library.md`) does not expose; it
//! is deferred until there is a concrete need for it.

/// The result of validating one generated SVG document.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationReport {
    pub width: u32,
    pub height: u32,
    pub tone_count: u8,
    pub total_paths: usize,
    pub closed_paths: usize,
    pub open_paths: usize,
    pub degenerate_paths: usize,
    pub total_nodes: usize,
    pub invalid_coordinate_paths: usize,
    pub lightburn_incompatibilities: Vec<&'static str>,
}

impl ValidationReport {
    /// `true` if every path is closed — CLAUDE.md Section 8's default
    /// goal ("0 paths abertos em regiões preenchidas").
    pub fn has_no_open_paths(&self) -> bool {
        self.open_paths == 0
    }

    /// `true` if no path contains a non-finite (NaN/infinite)
    /// coordinate.
    pub fn has_valid_coordinates(&self) -> bool {
        self.invalid_coordinate_paths == 0
    }

    /// `true` if none of the LightBurn-incompatible features below
    /// were found in the document.
    pub fn is_lightburn_compatible(&self) -> bool {
        self.lightburn_incompatibilities.is_empty()
    }
}

/// SVG features LightBurn does not import reliably: filters, complex
/// masks/clips, blend modes, embedded scripts/rasters, and other
/// renderer-specific or proprietary constructs.
const INCOMPATIBLE_MARKERS: &[&str] = &[
    "<filter",
    "<mask",
    "<clipPath",
    "<foreignObject",
    "<script",
    "<image",
    "mix-blend-mode",
];

/// Validates a generated SVG document. `width`/`height`/`tone_count`
/// are the values the document was generated with — they are recorded
/// in the report rather than re-derived by re-parsing the SVG.
pub fn validate(svg: &str, width: u32, height: u32, tone_count: u8) -> ValidationReport {
    let mut report = ValidationReport {
        width,
        height,
        tone_count,
        total_paths: 0,
        closed_paths: 0,
        open_paths: 0,
        degenerate_paths: 0,
        total_nodes: 0,
        invalid_coordinate_paths: 0,
        lightburn_incompatibilities: Vec::new(),
    };

    for path_data in extract_path_d_attributes(svg) {
        let stats = analyze_path_data(&path_data);

        report.total_paths += 1;
        if stats.closed {
            report.closed_paths += 1;
        } else {
            report.open_paths += 1;
        }
        if stats.node_count <= 1 {
            report.degenerate_paths += 1;
        }
        report.total_nodes += stats.node_count;
        if !stats.all_finite {
            report.invalid_coordinate_paths += 1;
        }
    }

    for marker in INCOMPATIBLE_MARKERS {
        if svg.contains(marker) {
            report.lightburn_incompatibilities.push(marker);
        }
    }

    report
}

struct PathStats {
    node_count: usize,
    closed: bool,
    all_finite: bool,
}

/// Parses a `d` attribute value into per-command node/finiteness
/// stats. Scoped to the commands `laserprep_vectorize` ever emits
/// (`M`, `L`, `C`, `Z`, all uppercase/absolute) rather than the full
/// SVG path grammar — this validator is for our own generated output,
/// not arbitrary imported SVGs.
fn analyze_path_data(d: &str) -> PathStats {
    let closed = d.trim_end().ends_with(['Z', 'z']);
    let mut node_count = 0usize;
    let mut all_finite = true;

    for (command, args) in split_by_commands(d) {
        let numbers = args
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|token| !token.is_empty())
            .map(|token| token.parse::<f64>().unwrap_or(f64::NAN));

        for number in numbers {
            if !number.is_finite() {
                all_finite = false;
            }
        }

        match command {
            'M' | 'L' | 'C' => node_count += 1,
            _ => {}
        }
    }

    PathStats {
        node_count,
        closed,
        all_finite,
    }
}

/// Splits `d` into `(command_letter, argument_string)` pairs, e.g.
/// `"M1 2 L3 4 Z"` -> `[('M', "1 2 "), ('L', "3 4 "), ('Z', "")]`.
fn split_by_commands(d: &str) -> Vec<(char, String)> {
    let mut segments = Vec::new();
    let mut current_command = None;
    let mut current_args = String::new();

    for ch in d.chars() {
        if matches!(ch, 'M' | 'L' | 'C' | 'Z' | 'm' | 'l' | 'c' | 'z') {
            if let Some(command) = current_command {
                segments.push((command, std::mem::take(&mut current_args)));
            }
            current_command = Some(ch);
        } else {
            current_args.push(ch);
        }
    }
    if let Some(command) = current_command {
        segments.push((command, current_args));
    }

    segments
}

/// Extracts each `<path>` element's `d` attribute value from a full
/// SVG document string.
fn extract_path_d_attributes(svg: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut search_from = 0usize;

    while let Some(relative_start) = svg[search_from..].find("<path") {
        let tag_start = search_from + relative_start;
        let tag_end = svg[tag_start..]
            .find('>')
            .map(|offset| tag_start + offset)
            .unwrap_or(svg.len());
        let tag = &svg[tag_start..tag_end];

        if let Some(d_relative) = tag.find("d=\"") {
            let after_d = &tag[d_relative + 3..];
            if let Some(end_relative) = after_d.find('"') {
                result.push(after_d[..end_relative].to_string());
            }
        }

        if tag_end >= svg.len() {
            break;
        }
        search_from = tag_end + 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg_with_paths(paths: &[&str]) -> String {
        let mut svg = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\">\n");
        for d in paths {
            svg.push_str(&format!("<path d=\"{d}\" fill=\"#000000\"/>\n"));
        }
        svg.push_str("</svg>\n");
        svg
    }

    #[test]
    fn counts_total_closed_and_open_paths() {
        let svg = svg_with_paths(&["M0 0 L1 0 L1 1 Z", "M0 0 L1 0 L1 1"]);
        let report = validate(&svg, 10, 10, 2);

        assert_eq!(report.total_paths, 2);
        assert_eq!(report.closed_paths, 1);
        assert_eq!(report.open_paths, 1);
        assert!(!report.has_no_open_paths());
    }

    #[test]
    fn flags_a_single_point_path_as_degenerate() {
        let svg = svg_with_paths(&["M5 5 Z"]);
        let report = validate(&svg, 10, 10, 2);

        assert_eq!(report.degenerate_paths, 1);
        assert_eq!(report.total_nodes, 1);
    }

    #[test]
    fn counts_nodes_across_move_line_and_curve_commands() {
        let svg = svg_with_paths(&["M0 0 L1 1 C2 2 3 3 4 4 Z"]);
        let report = validate(&svg, 10, 10, 2);

        // M (1) + L (1) + C (1, the curve's endpoint) = 3 nodes.
        assert_eq!(report.total_nodes, 3);
    }

    #[test]
    fn detects_non_finite_coordinates() {
        let svg = svg_with_paths(&["M0 0 LNaN 1 Z", "M0 0 L1 1 Z"]);
        let report = validate(&svg, 10, 10, 2);

        assert_eq!(report.invalid_coordinate_paths, 1);
        assert!(!report.has_valid_coordinates());
    }

    #[test]
    fn flags_lightburn_incompatible_features() {
        let mut svg = svg_with_paths(&["M0 0 L1 1 Z"]);
        svg.push_str("<filter id=\"blur\"></filter>\n");

        let report = validate(&svg, 10, 10, 2);

        assert!(!report.is_lightburn_compatible());
        assert!(report.lightburn_incompatibilities.contains(&"<filter"));
    }

    #[test]
    fn a_clean_document_is_lightburn_compatible() {
        let svg = svg_with_paths(&["M0 0 L1 1 Z"]);
        let report = validate(&svg, 10, 10, 2);

        assert!(report.is_lightburn_compatible());
        assert!(report.lightburn_incompatibilities.is_empty());
    }

    #[test]
    fn records_the_requested_dimensions_and_tone_count() {
        let report = validate(&svg_with_paths(&[]), 640, 480, 5);
        assert_eq!(report.width, 640);
        assert_eq!(report.height, 480);
        assert_eq!(report.tone_count, 5);
        assert_eq!(report.total_paths, 0);
    }
}
