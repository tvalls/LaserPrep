//! Named conversion presets (Photo, Portrait, Animal, Logo, Drawing,
//! Landscape). Presets store SVG conversion/organization preferences
//! only — never laser power/speed, which belongs to LightBurn/the
//! machine (CLAUDE.md Section 12).

use laserprep_analysis::ContentCategory;

/// A named bundle of conversion parameters. Values are reasonable
/// starting defaults chosen by hand, not tuned against a real-image
/// benchmark — expect them to be adjusted as Phase 3 category-specific
/// pipelines mature (`docs/roadmap.md`).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub name: PresetName,
    /// CLAUDE.md Section 7: 2-16 tones (see `laserprep_domain::ToneCount`).
    pub tone_count: u8,
    /// CLAUDE.md Section 8's "Minimum Area" noise filter, in pixels²
    /// (see `laserprep_vectorize::VtracerVectorizer`).
    pub min_area_px2: u32,
    /// CLAUDE.md Section 8's "Curve Simplification"/"Node Reduction"
    /// (see `laserprep_vectorize::VtracerVectorizer`'s doc comment for
    /// why those are one parameter here). Higher values trace smoother,
    /// lower-node-count paths at the cost of fine detail — content
    /// with a lot of natural high-frequency texture (skin, fur,
    /// foliage) benefits from a higher value than flat-color content
    /// (logos, line art), which wants sharp corners preserved instead.
    pub curve_simplification: f64,
    /// CLAUDE.md Section 10's optional tone legend.
    pub include_legend: bool,
    /// CLAUDE.md Section 8's "Merge Adjacent Regions"
    /// (`laserprep_optimize::merge_adjacent_regions`).
    pub merge_adjacent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PresetName {
    Photo,
    Portrait,
    Animal,
    Logo,
    Drawing,
    Landscape,
}

impl Preset {
    /// Balanced defaults for an unclassified or generic photograph.
    pub const PHOTO: Preset = Preset {
        name: PresetName::Photo,
        tone_count: 5,
        min_area_px2: 16,
        curve_simplification: 4.0,
        include_legend: false,
        merge_adjacent: false,
    };
    /// A touch more tonal range for skin/hair gradients; smaller
    /// minimum area so fine facial detail survives noise filtering.
    /// Higher curve simplification than Photo: a real high-resolution
    /// portrait export (close-up, natural skin/hair texture) traced
    /// over 78,000 path nodes at the vtracer stock tolerance — smaller
    /// `min_area_px2` alone keeps that detail, so the smoothing has to
    /// come from here instead of discarding regions outright.
    pub const PORTRAIT: Preset = Preset {
        name: PresetName::Portrait,
        tone_count: 6,
        min_area_px2: 12,
        curve_simplification: 8.0,
        include_legend: false,
        merge_adjacent: false,
    };
    /// Merges adjacent regions to avoid an explosion of tiny paths in
    /// fur/feather texture (CLAUDE.md Section 6: "evitar explosão de
    /// paths em pelagem"); higher curve simplification for the same
    /// reason.
    pub const ANIMAL: Preset = Preset {
        name: PresetName::Animal,
        tone_count: 5,
        min_area_px2: 20,
        curve_simplification: 8.0,
        include_legend: false,
        merge_adjacent: true,
    };
    /// Few tones and an aggressive minimum area for solid-color marks;
    /// merges adjacent regions since a logo's flat color fills are
    /// exactly what that operation cleans up. Stock curve
    /// simplification: a logo's sharp corners are the point, not noise
    /// to smooth away.
    pub const LOGO: Preset = Preset {
        name: PresetName::Logo,
        tone_count: 2,
        min_area_px2: 4,
        curve_simplification: 4.0,
        include_legend: false,
        merge_adjacent: true,
    };
    /// Few tones, small minimum area — line art and flat-color
    /// illustrations rarely need more than a handful of levels. Stock
    /// curve simplification, for the same reason as Logo: preserve the
    /// artist's intentional linework.
    pub const DRAWING: Preset = Preset {
        name: PresetName::Drawing,
        tone_count: 3,
        min_area_px2: 8,
        curve_simplification: 4.0,
        include_legend: false,
        merge_adjacent: false,
    };
    /// More tones and a larger minimum area for skies/foliage/terrain,
    /// which otherwise produce a lot of small noisy regions; highest
    /// curve simplification of any preset for the same reason.
    pub const LANDSCAPE: Preset = Preset {
        name: PresetName::Landscape,
        tone_count: 8,
        min_area_px2: 24,
        curve_simplification: 10.0,
        include_legend: false,
        merge_adjacent: false,
    };
}

/// All built-in presets, in the order a UI would typically list them.
pub const ALL_PRESETS: [Preset; 6] = [
    Preset::PHOTO,
    Preset::PORTRAIT,
    Preset::ANIMAL,
    Preset::LOGO,
    Preset::DRAWING,
    Preset::LANDSCAPE,
];

/// Suggests a preset from a heuristic [`ContentCategory`]
/// (`laserprep_analysis::classify`).
///
/// Only the categories that heuristic classifier can actually detect
/// map to a suggestion here. [`Preset::PORTRAIT`], [`Preset::ANIMAL`],
/// and [`Preset::DRAWING`] remain selectable by hand in the preset
/// list above, but are never auto-suggested: nothing in
/// `laserprep_analysis`'s global image statistics can tell a portrait
/// from an animal from a line drawing — that would need real subject/
/// object recognition, which CLAUDE.md Section 2 forbids.
pub fn suggest_preset(category: ContentCategory) -> Preset {
    match category {
        ContentCategory::Logo | ContentCategory::UniformBackground => Preset::LOGO,
        ContentCategory::Landscape => Preset::LANDSCAPE,
        ContentCategory::ComplexBackground | ContentCategory::GenericPhoto => Preset::PHOTO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use laserprep_domain::ToneCount;

    #[test]
    fn every_built_in_preset_has_a_valid_tone_count() {
        for preset in ALL_PRESETS {
            assert!(
                ToneCount::new(preset.tone_count).is_ok(),
                "{:?} has an out-of-range tone_count {}",
                preset.name,
                preset.tone_count
            );
        }
    }

    #[test]
    fn all_presets_have_distinct_names() {
        let names: std::collections::HashSet<_> = ALL_PRESETS.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), ALL_PRESETS.len());
    }

    #[test]
    fn suggests_logo_for_logo_and_uniform_background() {
        assert_eq!(suggest_preset(ContentCategory::Logo), Preset::LOGO);
        assert_eq!(
            suggest_preset(ContentCategory::UniformBackground),
            Preset::LOGO
        );
    }

    #[test]
    fn suggests_landscape_for_landscape() {
        assert_eq!(
            suggest_preset(ContentCategory::Landscape),
            Preset::LANDSCAPE
        );
    }

    #[test]
    fn suggests_photo_for_complex_background_and_generic_photo() {
        assert_eq!(
            suggest_preset(ContentCategory::ComplexBackground),
            Preset::PHOTO
        );
        assert_eq!(suggest_preset(ContentCategory::GenericPhoto), Preset::PHOTO);
    }
}
