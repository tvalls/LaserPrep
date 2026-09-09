//! The `presets` subcommand: lists the built-in presets
//! (`laserprep_presets::ALL_PRESETS`) and their parameters, so
//! `laser-vector convert --preset <name>` values are always
//! discoverable from the tool itself — no separate documentation to
//! keep in sync as presets are added (`docs/roadmap.md` Phase 7:
//! "extensibility for new pipelines/presets/algorithms").

use crate::args::preset_name_label;
use laserprep_presets::ALL_PRESETS;

pub fn run() {
    println!(
        "{:<10} {:>6} {:>10} {:>8} {:>15}",
        "preset", "tones", "min-area", "legend", "merge-adjacent"
    );
    for preset in ALL_PRESETS {
        println!(
            "{:<10} {:>6} {:>10} {:>8} {:>15}",
            preset_name_label(preset.name),
            preset.tone_count,
            preset.min_area_px2,
            preset.include_legend,
            preset.merge_adjacent,
        );
    }
}
