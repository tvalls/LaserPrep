//! CLI argument definitions and the `convert` subcommand's execution
//! logic. Kept separate from `main.rs` so both can be exercised by
//! integration tests without going through `std::process`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use laserprep_presets::{Preset, PresetName};
use std::path::PathBuf;

/// LaserPrep's default DPI (`src/App.tsx`'s `DEFAULT_DPI`), used when
/// `--dpi` is not given.
const DEFAULT_DPI: f64 = 96.0;

#[derive(Parser)]
#[command(
    name = "laser-vector",
    version,
    about = "Convert raster images to laser-ready, tone-separated SVGs (LaserPrep's conversion core, CLAUDE.md Section 5)."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Convert one image into a tone-separated SVG.
    Convert(ConvertArgs),
    /// List the built-in presets and their parameters.
    Presets,
}

#[derive(Args)]
pub struct ConvertArgs {
    /// Input image file (PNG or JPEG).
    pub input: PathBuf,

    /// Output SVG file path.
    #[arg(short, long)]
    pub output: PathBuf,

    /// Named starting point for conversion parameters (CLAUDE.md
    /// Section 12). Individual flags below override its values.
    #[arg(long, value_enum, default_value_t = PresetArg::Photo)]
    pub preset: PresetArg,

    /// Number of tones, 2-16 (CLAUDE.md Section 7). Overrides the
    /// preset's tone count when given.
    #[arg(long)]
    pub tones: Option<u8>,

    /// Minimum region area in pixels² below which a traced region is
    /// discarded as noise (CLAUDE.md Section 8's "Minimum Area").
    /// Overrides the preset's value when given.
    #[arg(long = "min-area")]
    pub min_area_px2: Option<u32>,

    /// Output DPI, used to compute the SVG's physical dimensions
    /// (CLAUDE.md Section 11).
    #[arg(long, default_value_t = DEFAULT_DPI)]
    pub dpi: f64,

    /// Include the tone legend group (CLAUDE.md Section 10). Overrides
    /// the preset's value when given.
    #[arg(long)]
    pub legend: bool,

    /// Merge adjacent same-tone regions (CLAUDE.md Section 8's "Merge
    /// Adjacent Regions"). Overrides the preset's value when given.
    #[arg(long = "merge-adjacent")]
    pub merge_adjacent: bool,

    /// Order each tone's paths to reduce laser travel distance
    /// (CLAUDE.md Section 8's "Path Ordering", `docs/roadmap.md`
    /// Phase 5).
    #[arg(long = "order-paths")]
    pub order_paths: bool,
}

/// Mirrors `laserprep_presets::PresetName` as a `clap::ValueEnum` so
/// `--preset` accepts lowercase, kebab-friendly names without
/// requiring `laserprep-presets` (a shared, UI-agnostic crate) to know
/// about `clap`.
#[derive(Clone, Copy, ValueEnum)]
pub enum PresetArg {
    Photo,
    Portrait,
    Animal,
    Logo,
    Drawing,
    Landscape,
}

impl PresetArg {
    fn resolve(self) -> Preset {
        match self {
            PresetArg::Photo => Preset::PHOTO,
            PresetArg::Portrait => Preset::PORTRAIT,
            PresetArg::Animal => Preset::ANIMAL,
            PresetArg::Logo => Preset::LOGO,
            PresetArg::Drawing => Preset::DRAWING,
            PresetArg::Landscape => Preset::LANDSCAPE,
        }
    }
}

pub fn preset_name_label(name: PresetName) -> &'static str {
    match name {
        PresetName::Photo => "photo",
        PresetName::Portrait => "portrait",
        PresetName::Animal => "animal",
        PresetName::Logo => "logo",
        PresetName::Drawing => "drawing",
        PresetName::Landscape => "landscape",
    }
}

/// Runs the `convert` subcommand: reads `args.input`, converts it
/// according to `args.preset` (as overridden by any explicit flags),
/// and writes the resulting SVG to `args.output`. Returns a
/// human-readable error message on failure — never panics on bad
/// input (CLAUDE.md Section 13).
pub fn run_convert(args: ConvertArgs) -> Result<(), String> {
    let preset = args.preset.resolve();
    let tone_count = args.tones.unwrap_or(preset.tone_count);
    let min_area_px2 = args.min_area_px2.unwrap_or(preset.min_area_px2);
    let include_legend = args.legend || preset.include_legend;
    let merge_adjacent = args.merge_adjacent || preset.merge_adjacent;

    let bytes = std::fs::read(&args.input)
        .map_err(|err| format!("failed to read {}: {err}", args.input.display()))?;

    let result = laserprep_pipeline::convert_bytes_to_svg(
        &bytes,
        args.dpi,
        tone_count,
        min_area_px2,
        include_legend,
        merge_adjacent,
        args.order_paths,
    )
    .map_err(|err| err.to_string())?;

    laserprep_pipeline::save_svg_to_file(&args.output, &result.svg)
        .map_err(|err| format!("failed to write {}: {err}", args.output.display()))?;

    println!(
        "converted {} -> {} ({} tones)",
        args.input.display(),
        args.output.display(),
        tone_count,
    );
    println!(
        "classified as {:?} (confidence {:.2}), suggested preset: {}",
        result.classification.category,
        result.classification.confidence,
        preset_name_label(result.suggested_preset.name),
    );
    println!(
        "validation: {} paths ({} closed, {} open, {} degenerate), {} nodes, LightBurn-compatible: {}",
        result.validation.total_paths,
        result.validation.closed_paths,
        result.validation.open_paths,
        result.validation.degenerate_paths,
        result.validation.total_nodes,
        result.validation.is_lightburn_compatible(),
    );
    if !result.validation.has_no_open_paths() {
        eprintln!(
            "warning: {} open path(s) remain — see CLAUDE.md Section 8 (paths are never closed \
             arbitrarily when closure is not mathematically safe)",
            result.validation.open_paths
        );
    }
    println!(
        "path ordering efficiency: {:.1}% of optimal travel distance",
        result.optimization_score.efficiency * 100.0
    );

    Ok(())
}
