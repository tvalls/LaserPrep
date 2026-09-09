//! LaserPrep application shell: Tauri bootstrap only. Conversion logic
//! lives in `crates/*` and is orchestrated from here via Tauri commands
//! (added starting Phase 1) — the UI never depends on those crates
//! directly. See `docs/architecture.md`.

mod commands;
mod pipeline;

use commands::{DecodedSourceState, SettingsState, SourceImageState};
use laserprep_settings::SettingsStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::convert_image_file,
            commands::convert_current_source,
            commands::save_svg_file,
            commands::save_svg_files,
            commands::write_text_file,
            commands::save_project,
            commands::open_project,
            commands::current_source_image_data_url,
            commands::get_settings,
            commands::save_settings,
            commands::get_diagnostic_info,
            commands::export_diagnostics,
        ])
        .setup(|app| {
            // Logging can't start until the app handle exists (the log
            // directory is resolved from it), so this is as early as
            // structured logging (CLAUDE.md Section 20) can begin — a
            // small, unavoidable gap before this line covers only
            // Tauri's own internal bootstrap, not app code.
            let log_dir = app.path().app_log_dir()?;
            std::fs::create_dir_all(&log_dir)?;
            // Rolls to a new file per day; nothing prunes old files yet
            // (CLAUDE.md doesn't require it and log files are small
            // plain text, but it's a known gap — see "Exportar
            // diagnóstico"/commands::export_diagnostics, which reads
            // every file in this directory).
            let file_appender = tracing_appender::rolling::daily(&log_dir, "laserprep.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            tracing_subscriber::fmt()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .init();
            // Dropping this stops the background thread that flushes
            // log writes to disk, so it must outlive the app.
            app.manage(guard);

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

            tracing::info!(
                language = %settings.ui.language,
                log_dir = %log_dir.display(),
                "LaserPrep starting"
            );

            app.manage(store);
            app.manage(SettingsState::new(settings));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LaserPrep");
}
