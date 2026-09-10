//! Optional tone legend (CLAUDE.md Section 10): a `<g id="legend">`
//! group of swatch/number pairs, one per tone level, laid out outside
//! the main artwork so it can be hidden or edited independently in
//! LightBurn (any top-level `<g>` is its own layer there).

use laserprep_domain::ToneCount;

const SWATCH_SIZE_PX: u32 = 20;
const ROW_HEIGHT_PX: u32 = 30;
const GAP_FROM_ARTWORK_PX: u32 = 20;

/// Total extra width the legend reserves to the right of the artwork,
/// including the gap separating it from the artwork.
pub const RESERVED_WIDTH_PX: u32 = 150;

/// Renders the `<g id="legend">` fragment: one filled swatch and its
/// tone index per row, positioned `artwork_width + gap` to the right
/// of the artwork's origin. Each swatch is filled with
/// `tone_grays[tone]` — the same representative grayscale
/// (`laserprep_quantize::tone_mean_luminance`) the matching `<g
/// id="tone-N">` artwork group uses — so the legend visually confirms
/// each layer's relative darkness rather than showing a generic black
/// square (CLAUDE.md Section 10). A missing or out-of-range index
/// falls back to black.
pub fn legend_fragment(tone_count: ToneCount, artwork_width: u32, tone_grays: &[u8]) -> String {
    let x_offset = artwork_width + GAP_FROM_ARTWORK_PX;

    let mut fragment = format!("  <g id=\"legend\" transform=\"translate({x_offset},0)\">\n");
    for tone in 0..tone_count.get() {
        let y = u32::from(tone) * ROW_HEIGHT_PX;
        let gray = tone_grays.get(tone as usize).copied().unwrap_or(0);
        fragment.push_str(&format!(
            "    <rect x=\"0\" y=\"{y}\" width=\"{SWATCH_SIZE_PX}\" height=\"{SWATCH_SIZE_PX}\" fill=\"#{gray:02x}{gray:02x}{gray:02x}\" />\n",
        ));
        fragment.push_str(&format!(
            "    <text x=\"{text_x}\" y=\"{text_y}\" font-size=\"14\">{tone}</text>\n",
            text_x = SWATCH_SIZE_PX + 6,
            text_y = y + SWATCH_SIZE_PX - 4,
        ));
    }
    fragment.push_str("  </g>\n");

    fragment
}

/// The legend's own height, so callers can size the document tall
/// enough to fit every row.
pub fn legend_height_px(tone_count: ToneCount) -> u32 {
    u32::from(tone_count.get()) * ROW_HEIGHT_PX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_one_swatch_and_label_per_tone() {
        let tone_count = ToneCount::new(3).unwrap();
        let fragment = legend_fragment(tone_count, 100, &[]);

        assert!(fragment.starts_with("  <g id=\"legend\""));
        assert_eq!(fragment.matches("<rect").count(), 3);
        for tone in 0..3 {
            assert!(fragment.contains(&format!(">{tone}</text>")));
        }
    }

    #[test]
    fn swatches_use_each_tones_representative_gray() {
        let tone_count = ToneCount::new(2).unwrap();
        let fragment = legend_fragment(tone_count, 100, &[10, 221]);

        assert!(fragment.contains("fill=\"#0a0a0a\""));
        assert!(fragment.contains("fill=\"#dddddd\""));
    }

    #[test]
    fn positions_the_group_past_the_artwork_plus_gap() {
        let fragment = legend_fragment(ToneCount::default(), 200, &[]);
        assert!(fragment.contains("translate(220,0)"));
    }

    #[test]
    fn height_scales_with_tone_count() {
        assert_eq!(legend_height_px(ToneCount::new(2).unwrap()), 60);
        assert_eq!(legend_height_px(ToneCount::new(5).unwrap()), 150);
    }
}
