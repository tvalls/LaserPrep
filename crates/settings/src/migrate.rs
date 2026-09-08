use crate::model::{CURRENT_SCHEMA_VERSION, Settings};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("failed to access the settings file: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings file is not valid JSON: {0}")]
    InvalidJson(serde_json::Error),
    #[error("failed to deserialize settings: {0}")]
    Deserialize(serde_json::Error),
    #[error("settings schema version {0} has no migration path in this build")]
    UnsupportedSchemaVersion(u32),
    #[error(
        "settings were written by a newer version of LaserPrep (schema {0}, this build supports up to {CURRENT_SCHEMA_VERSION}); refusing to modify them"
    )]
    NewerSchemaVersion(u32),
}

/// Migrates a raw JSON settings document to [`CURRENT_SCHEMA_VERSION`]
/// and deserializes it into [`Settings`].
///
/// A missing `schemaVersion` is treated as schema `1` (the oldest
/// supported schema), matching pre-versioning settings files. A
/// `schemaVersion` newer than this build understands is rejected rather
/// than silently reinterpreted, per auto-ci Standard 7's forward-version
/// handling rule.
pub fn migrate(data: Value) -> Result<Settings, SettingsError> {
    let mut version = data
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;
    let mut data = data;

    while version < CURRENT_SCHEMA_VERSION {
        data = match version {
            // Example for the next schema bump:
            // 1 => { version = 2; migrate_v1_to_v2(data) }
            other => return Err(SettingsError::UnsupportedSchemaVersion(other)),
        };
    }

    if version > CURRENT_SCHEMA_VERSION {
        return Err(SettingsError::NewerSchemaVersion(version));
    }

    serde_json::from_value(data).map_err(SettingsError::Deserialize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_schema_version_is_treated_as_v1() {
        let settings = migrate(json!({})).expect("should migrate");
        assert_eq!(settings.schema_version, 1);
        assert_eq!(settings.ui.language, "en-US");
    }

    #[test]
    fn missing_optional_fields_use_defaults() {
        let settings = migrate(json!({ "schemaVersion": 1 })).expect("should migrate");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn rejects_schema_version_newer_than_supported() {
        let err = migrate(json!({ "schemaVersion": 999 })).unwrap_err();
        assert!(matches!(err, SettingsError::NewerSchemaVersion(999)));
    }

    #[test]
    fn preserves_unknown_top_level_fields() {
        let settings = migrate(json!({
            "schemaVersion": 1,
            "someFutureFieldThisBuildDoesNotKnow": { "nested": true }
        }))
        .expect("should migrate");
        assert_eq!(
            settings.unknown.get("someFutureFieldThisBuildDoesNotKnow"),
            Some(&json!({ "nested": true }))
        );
    }

    #[test]
    fn preserves_relevant_existing_values() {
        let settings = migrate(json!({
            "schemaVersion": 1,
            "ui": { "language": "pt-BR", "theme": "dark" },
            "updates": { "checkOnStartup": false, "skippedVersions": ["v2.0.0"] }
        }))
        .expect("should migrate");
        assert_eq!(settings.ui.language, "pt-BR");
        assert!(!settings.updates.check_on_startup);
        assert_eq!(
            settings.updates.skipped_versions,
            vec!["v2.0.0".to_string()]
        );
    }
}
