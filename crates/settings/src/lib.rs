//! Versioned JSON settings persistence, implementing the auto-ci skill's
//! Standard 7: an integer `schemaVersion`, explicit sequential
//! migrations, safe defaults for additive fields, unknown-field
//! preservation, atomic writes, pre-migration backups, corrupt-file
//! recovery, and forward-version handling (never destructively rewrite a
//! settings file written by a newer application version).
//!
//! This crate has no Tauri dependency — it is plain Rust, wired into
//! Tauri commands from `src-tauri`. See
//! `docs/adr/0008-settings-store-custom.md` for why this is hand-written
//! instead of using `tauri-plugin-store`.

mod migrate;
mod model;
mod store;

pub use migrate::{SettingsError, migrate};
pub use model::{CURRENT_SCHEMA_VERSION, Settings, Theme, UiSettings, UpdateSettings};
pub use store::SettingsStore;
