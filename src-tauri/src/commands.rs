//! Tauri IPC commands exposed to the frontend. Each command is a thin
//! wrapper around [`crate::pipeline`] or `laserprep_project`, mapping
//! their error types to a `String` for IPC (Tauri command errors must
//! be `Serialize`).
//!
//! The original image bytes behind the current conversion — whichever
//! of [`convert_image_file`] or [`open_project`] ran last — are kept
//! in [`SourceImageState`], managed by Tauri (see `lib.rs`). This lets
//! [`save_project`] and [`convert_current_source`] work from either an
//! imported file or a reopened `.lvp` project without the frontend
//! having to shuttle the (potentially large) original image bytes back
//! and forth over IPC just to hold onto them.

use crate::pipeline::{self, ConversionResult};
use laserprep_analysis::Classification;
use laserprep_presets::PresetName;
use laserprep_project::{ConversionParams, ProjectFile, ProjectResult, ProjectStore, SourceImage};
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

/// The source image behind the conversion currently shown in the UI,
/// if any. `None` until an image is imported or a project is opened.
pub type SourceImageState = Mutex<Option<SourceImage>>;

fn lock_source<'a>(
    state: &'a State<'_, SourceImageState>,
) -> Result<std::sync::MutexGuard<'a, Option<SourceImage>>, String> {
    state
        .lock()
        .map_err(|_| "internal state lock was poisoned".to_string())
}

#[tauri::command]
pub fn convert_image_file(
    path: String,
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    include_legend: bool,
    merge_adjacent: bool,
    source: State<'_, SourceImageState>,
) -> Result<ConversionResult, String> {
    let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
    let file_name = Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string());
    *lock_source(&source)? = Some(SourceImage::from_bytes(file_name, &bytes));

    pipeline::convert_bytes_to_svg(
        &bytes,
        dpi,
        tone_count,
        min_area_px2,
        include_legend,
        merge_adjacent,
    )
    .map_err(|err| err.to_string())
}

/// Reconverts the currently loaded source image (from the last
/// [`convert_image_file`] or [`open_project`] call) with new
/// parameters, without requiring the user to re-pick a file.
#[tauri::command]
pub fn convert_current_source(
    dpi: f64,
    tone_count: u8,
    min_area_px2: u32,
    include_legend: bool,
    merge_adjacent: bool,
    source: State<'_, SourceImageState>,
) -> Result<ConversionResult, String> {
    let source_image = lock_source(&source)?
        .clone()
        .ok_or_else(|| "no image has been imported or opened yet".to_string())?;
    let bytes = source_image.decode().map_err(|err| err.to_string())?;

    pipeline::convert_bytes_to_svg(
        &bytes,
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

/// One SVG document to write to `path`, as produced by a batch
/// conversion (CLAUDE.md Section 12).
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SvgFileToSave {
    pub path: String,
    pub svg: String,
}

/// The outcome of writing one file from [`save_svg_files`]. A plain
/// `Result` per file rather than a single `Result<(), String>` for the
/// whole batch, so one file failing (e.g. a permission error) doesn't
/// hide whether the others succeeded.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSvgFileOutcome {
    pub path: String,
    pub error: Option<String>,
}

/// Writes every file in `files`, continuing past individual failures.
#[tauri::command]
pub fn save_svg_files(files: Vec<SvgFileToSave>) -> Vec<SaveSvgFileOutcome> {
    files
        .into_iter()
        .map(|file| {
            let error = pipeline::save_svg_to_file(Path::new(&file.path), &file.svg)
                .err()
                .map(|err| err.to_string());
            SaveSvgFileOutcome {
                path: file.path,
                error,
            }
        })
        .collect()
}

/// Arguments for [`save_project`], bundled into one struct (rather than
/// flat command arguments) since it needs to carry every field of a
/// `.lvp` project except the source image, which comes from
/// [`SourceImageState`] instead.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectRequest {
    pub project_path: String,
    pub params: ConversionParams,
    pub preset: Option<PresetName>,
    pub classification: Option<Classification>,
    pub result: Option<ProjectResult>,
}

/// Saves a `.lvp` project (CLAUDE.md Section 12) at
/// `request.project_path`, built from the currently loaded source
/// image plus the parameters, preset, classification, and result the
/// frontend currently has on screen.
#[tauri::command]
pub fn save_project(
    request: SaveProjectRequest,
    source: State<'_, SourceImageState>,
) -> Result<(), String> {
    let source_image = lock_source(&source)?
        .clone()
        .ok_or_else(|| "no image has been imported or opened yet".to_string())?;

    let mut project = ProjectFile::new(source_image, request.params, request.preset);
    project.classification = request.classification;
    project.result = request.result;

    ProjectStore::new(&request.project_path)
        .save(&project)
        .map_err(|err| err.to_string())
}

/// The parts of a `.lvp` project the frontend needs to restore its own
/// state. Omits the embedded source image bytes — they stay server-side
/// in [`SourceImageState`] rather than round-tripping through IPC a
/// second time (`open_project` already read them once from disk).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedProject {
    pub source_file_name: Option<String>,
    pub params: ConversionParams,
    pub preset: Option<PresetName>,
    pub classification: Option<Classification>,
    pub result: Option<ProjectResult>,
}

#[tauri::command]
pub fn open_project(
    path: String,
    source: State<'_, SourceImageState>,
) -> Result<OpenedProject, String> {
    let project = ProjectStore::new(&path)
        .load()
        .map_err(|err| err.to_string())?;

    let opened = OpenedProject {
        source_file_name: project.source_image.file_name.clone(),
        params: project.params,
        preset: project.preset,
        classification: project.classification,
        result: project.result,
    };

    *lock_source(&source)? = Some(project.source_image);
    Ok(opened)
}
