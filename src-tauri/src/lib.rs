//! LaserPrep application shell: Tauri bootstrap only. Conversion logic
//! lives in `crates/*` and is orchestrated from here via Tauri commands
//! (added starting Phase 1) — the UI never depends on those crates
//! directly. See `docs/architecture.md`.

mod commands;
mod pipeline;

use commands::{DecodedSourceState, SourceImageState};
use laserprep_settings::SettingsStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::convert_image_file,
            commands::convert_current_source,
            commands::save_svg_file,
            commands::save_svg_files,
            commands::save_project,
            commands::open_project,
        ])
        .setup(|app| {
            app.manage(SourceImageState::default());
            app.manage(DecodedSourceState::default());

            let config_dir = match app.path().app_config_dir() {
                Ok(dir) => dir,
                Err(err) => {
                    tracing::error!("failed to resolve app config directory: {err}");
                    return Err(Box::new(err));
                }
            };

            let store = SettingsStore::new(config_dir.join("settings.json"));
            let settings = store.load().unwrap_or_else(|err| {
                tracing::warn!("failed to load settings, falling back to defaults: {err}");
                Default::default()
            });

            tracing::info!(language = %settings.ui.language, "LaserPrep starting");

            app.manage(store);
            app.manage(settings);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LaserPrep");
}
