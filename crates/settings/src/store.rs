use crate::migrate::{SettingsError, migrate};
use crate::model::Settings;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Reads and atomically writes a [`Settings`] document at a fixed path.
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads settings, migrating as needed. A missing file yields
    /// `Settings::default()` without creating anything on disk. A file
    /// that fails to parse as JSON is left untouched on disk (never
    /// overwritten by `load`) so it can be recovered/inspected; the
    /// caller decides how to proceed (defaults, restore from backup, or
    /// surface the error to the user).
    pub fn load(&self) -> Result<Settings, SettingsError> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(Settings::default());
            }
            Err(err) => return Err(SettingsError::Io(err)),
        };

        let value: Value = serde_json::from_str(&raw).map_err(SettingsError::InvalidJson)?;
        migrate(value)
    }

    /// Atomically writes `settings`: serialize to a temporary file in the
    /// same directory, then rename over the destination. Rename is
    /// atomic on both Windows and POSIX filesystems when source and
    /// destination share a directory.
    pub fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(settings).map_err(SettingsError::Deserialize)?;
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, &self.path)?;
        Ok(())
    }

    /// Copies the current on-disk settings file to `<name>.json.bak`, if
    /// it exists. Call this before performing a destructive/non-trivial
    /// migration.
    pub fn backup(&self) -> Result<(), SettingsError> {
        if self.path.exists() {
            fs::copy(&self.path, self.path.with_extension("json.bak"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Theme;

    fn store_in_temp_dir() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let store = SettingsStore::new(dir.path().join("settings.json"));
        (dir, store)
    }

    #[test]
    fn loading_missing_file_returns_defaults() {
        let (_dir, store) = store_in_temp_dir();
        let settings = store.load().expect("load should succeed");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn round_trips_through_save_and_load() {
        let (_dir, store) = store_in_temp_dir();
        let mut settings = Settings::default();
        settings.ui.language = "pt-BR".to_string();
        settings.ui.theme = Theme::Dark;
        settings.updates.skipped_versions.push("v0.9.0".to_string());

        store.save(&settings).expect("save should succeed");
        let loaded = store.load().expect("load should succeed");

        assert_eq!(loaded, settings);
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temp_file_behind() {
        let (_dir, store) = store_in_temp_dir();
        store
            .save(&Settings::default())
            .expect("save should succeed");

        assert!(store.path().exists());
        assert!(!store.path().with_extension("json.tmp").exists());
    }

    #[test]
    fn corrupt_json_is_preserved_and_reported() {
        let (_dir, store) = store_in_temp_dir();
        fs::write(store.path(), b"{ not valid json").unwrap();

        let err = store.load().unwrap_err();
        assert!(matches!(err, SettingsError::InvalidJson(_)));
        // The corrupt file must still be there for diagnosis.
        let contents = fs::read_to_string(store.path()).unwrap();
        assert_eq!(contents, "{ not valid json");
    }

    #[test]
    fn backup_copies_existing_file_to_bak_suffix() {
        let (_dir, store) = store_in_temp_dir();
        store
            .save(&Settings::default())
            .expect("save should succeed");

        store.backup().expect("backup should succeed");

        let backup_path = store.path().with_extension("json.bak");
        assert!(backup_path.exists());
    }

    #[test]
    fn backup_is_a_no_op_when_no_file_exists_yet() {
        let (_dir, store) = store_in_temp_dir();
        store
            .backup()
            .expect("backup should succeed even with no file");
        assert!(!store.path().with_extension("json.bak").exists());
    }
}
