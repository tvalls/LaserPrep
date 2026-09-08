//! LaserPrep application shell: Tauri bootstrap only. Conversion logic
//! lives in `crates/*` and is orchestrated from here via Tauri commands
//! (added starting Phase 1) — the UI never depends on those crates
//! directly. See `docs/architecture.md`.

use laserprep_settings::SettingsStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .setup(|app| {
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
