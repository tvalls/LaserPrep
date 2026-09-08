//! Tauri IPC commands exposed to the frontend. Each command is a thin
//! wrapper around [`crate::pipeline`] that maps [`PipelineError`] to a
//! `String` for IPC (Tauri command errors must be `Serialize`).

use crate::pipeline::{self, ConversionResult};
use std::path::Path;

#[tauri::command]
pub fn convert_image_file(
    path: String,
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    include_legend: bool,
    merge_adjacent: bool,
) -> Result<ConversionResult, String> {
    pipeline::convert_file_to_svg(
        Path::new(&path),
        dpi,
        tone_count,
        min_area_px2,
        include_legend,
        merge_adjacent,
    )
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn save_svg_file(path: String, svg: String) -> Result<(), String> {
    pipeline::save_svg_to_file(Path::new(&path), &svg).map_err(|err| err.to_string())
}
