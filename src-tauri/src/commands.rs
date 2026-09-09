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
//!
//! Alongside it, [`DecodedSourceState`] caches the *decoded and
//! classified* form of that same image (`pipeline::DecodedSource`):
//! decoding and heuristic classification depend only on the source
//! bytes, never on conversion parameters, so re-running them on every
//! [`convert_current_source`] call (as the UI's Reconvert action and
//! undo/redo naturally do while the user tweaks tone count, minimum
//! area, etc.) would repeat work whose result cannot have changed.
//! Kept in sync with [`SourceImageState`] at the same two entry
//! points (import, open project) so the two are never out of step.

use crate::pipeline::{self, ConversionResult, DecodedSource};
use laserprep_analysis::Classification;
use laserprep_presets::PresetName;
use laserprep_project::{ConversionParams, ProjectFile, ProjectResult, ProjectStore, SourceImage};
use laserprep_settings::{Settings, SettingsStore};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::State;

/// The source image behind the conversion currently shown in the UI,
/// if any. `None` until an image is imported or a project is opened.
pub type SourceImageState = Mutex<Option<SourceImage>>;

/// The decoded/classified form of the same image `SourceImageState`
/// currently holds. `Arc` so reading it for a conversion is a cheap
/// reference-count bump rather than a clone of the full pixel buffer.
pub type DecodedSourceState = Mutex<Option<Arc<DecodedSource>>>;

/// The application settings currently loaded (CLAUDE.md Section 22:
/// update-check preferences; auto-ci Standard 7 more generally),
/// mutable so [`save_settings`] can update it in place after
/// persisting to disk.
pub type SettingsState = Mutex<Settings>;

fn lock_source<'a>(
    state: &'a State<'_, SourceImageState>,
) -> Result<std::sync::MutexGuard<'a, Option<SourceImage>>, String> {
    state
        .lock()
        .map_err(|_| "internal state lock was poisoned".to_string())
}

fn lock_decoded<'a>(
    state: &'a State<'_, DecodedSourceState>,
) -> Result<std::sync::MutexGuard<'a, Option<Arc<DecodedSource>>>, String> {
    state
        .lock()
        .map_err(|_| "internal state lock was poisoned".to_string())
}

fn lock_settings<'a>(
    state: &'a State<'_, SettingsState>,
) -> Result<std::sync::MutexGuard<'a, Settings>, String> {
    state
        .lock()
        .map_err(|_| "internal state lock was poisoned".to_string())
}

/// Returns the currently loaded settings.
#[tauri::command]
pub fn get_settings(settings: State<'_, SettingsState>) -> Result<Settings, String> {
    Ok(lock_settings(&settings)?.clone())
}

/// Persists `settings` to disk and updates the in-memory copy
/// [`get_settings`] (and anything else reading [`SettingsState`])
/// sees from then on.
#[tauri::command]
pub fn save_settings(
    settings: Settings,
    store: State<'_, SettingsStore>,
    state: State<'_, SettingsState>,
) -> Result<(), String> {
    store.save(&settings).map_err(|err| err.to_string())?;
    *lock_settings(&state)? = settings;
    Ok(())
}

#[tauri::command]
pub fn convert_image_file(
    path: String,
    params: ConversionParams,
    source: State<'_, SourceImageState>,
    decoded: State<'_, DecodedSourceState>,
) -> Result<ConversionResult, String> {
    let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
    let file_name = Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string());
    *lock_source(&source)? = Some(SourceImage::from_bytes(file_name, &bytes));

    let decoded_source =
        Arc::new(pipeline::decode_and_classify(&bytes).map_err(|err| err.to_string())?);
    *lock_decoded(&decoded)? = Some(Arc::clone(&decoded_source));

    pipeline::convert_decoded_to_svg(
        &decoded_source,
        params.dpi,
        params.tone_count,
        params.min_area_px2,
        params.include_legend,
        params.merge_adjacent,
        params.order_paths,
    )
    .map_err(|err| err.to_string())
}

/// Reconverts the currently loaded source image (from the last
/// [`convert_image_file`] or [`open_project`] call) with new
/// parameters, without requiring the user to re-pick a file or
/// re-decoding/re-classifying an image that hasn't changed.
#[tauri::command]
pub fn convert_current_source(
    params: ConversionParams,
    decoded: State<'_, DecodedSourceState>,
) -> Result<ConversionResult, String> {
    let decoded_source = lock_decoded(&decoded)?
        .clone()
        .ok_or_else(|| "no image has been imported or opened yet".to_string())?;

    pipeline::convert_decoded_to_svg(
        &decoded_source,
        params.dpi,
        params.tone_count,
        params.min_area_px2,
        params.include_legend,
        params.merge_adjacent,
        params.order_paths,
    )
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn save_svg_file(path: String, svg: String) -> Result<(), String> {
    pipeline::save_svg_to_file(Path::new(&path), &svg).map_err(|err| err.to_string())
}

/// Writes arbitrary plain text to `path` — used for saving a bug
/// report (CLAUDE.md Section 20) as a local file the user can attach
/// or paste wherever they choose.
#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|err| err.to_string())
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
    decoded: State<'_, DecodedSourceState>,
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

    let bytes = project
        .source_image
        .decode()
        .map_err(|err| err.to_string())?;
    let decoded_source =
        Arc::new(pipeline::decode_and_classify(&bytes).map_err(|err| err.to_string())?);
    *lock_decoded(&decoded)? = Some(decoded_source);
    *lock_source(&source)? = Some(project.source_image);

    Ok(opened)
}

/// The currently loaded source image (from the last [`convert_image_file`]
/// or [`open_project`] call), as a `data:` URL — the UI's before/after
/// preview embeds this directly in an `<img>` element rather than
/// asking Tauri's asset protocol to serve an arbitrary file path.
/// `SourceImage` already stores its bytes as base64 (CLAUDE.md Section
/// 12: the `.lvp` format embeds the source image the same way), so
/// this only needs to sniff a MIME type for the data URL prefix, not
/// re-encode anything.
#[tauri::command]
pub fn current_source_image_data_url(
    source: State<'_, SourceImageState>,
) -> Result<String, String> {
    let source_image = lock_source(&source)?
        .clone()
        .ok_or_else(|| "no image has been imported or opened yet".to_string())?;
    let bytes = source_image.decode().map_err(|err| err.to_string())?;
    let mime = laserprep_imaging::guess_mime_type(&bytes).unwrap_or("application/octet-stream");

    Ok(format!("data:{mime};base64,{}", source_image.data_base64))
}

/// Version/platform context a bug report needs (CLAUDE.md Section 20:
/// "Ajuda → Reportar problema" asks for título, descrição, passos,
/// logs, versão, SO, arquitetura) — the frontend pre-fills its report
/// form with this rather than asking the user to type it in by hand.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticInfo {
    pub app_version: String,
    pub os: String,
    pub arch: String,
}

#[tauri::command]
pub fn get_diagnostic_info() -> DiagnosticInfo {
    DiagnosticInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

/// Writes a plain-text diagnostic bundle (version/OS/architecture plus
/// every log file `lib.rs`'s daily-rolling appender has written) to
/// `path`. Deliberately never includes the user's image — there is
/// nothing here that could (CLAUDE.md Section 20: "diagnóstico
/// exportável nunca inclui a imagem original por padrão" — trivially
/// true today since this never touches image bytes at all, not just
/// filtered out).
#[tauri::command]
pub fn export_diagnostics(path: String, app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager as _;

    let info = get_diagnostic_info();
    let mut output = format!(
        "LaserPrep diagnostic export\nVersion: {}\nOS: {}\nArchitecture: {}\n\n",
        info.app_version, info.os, info.arch
    );

    let log_dir = app.path().app_log_dir().map_err(|err| err.to_string())?;
    let mut log_files: Vec<_> = std::fs::read_dir(&log_dir)
        .map_err(|err| err.to_string())?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    log_files.sort();

    for log_file in log_files {
        output.push_str(&format!("=== {} ===\n", log_file.display()));
        match std::fs::read_to_string(&log_file) {
            Ok(contents) => output.push_str(&contents),
            Err(err) => output.push_str(&format!("(failed to read: {err})\n")),
        }
        output.push('\n');
    }

    std::fs::write(&path, output).map_err(|err| err.to_string())
}
