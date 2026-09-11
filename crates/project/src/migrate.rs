use crate::model::{CURRENT_SCHEMA_VERSION, ProjectFile};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("failed to access the project file: {0}")]
    Io(#[from] std::io::Error),
    #[error("project file is not valid JSON: {0}")]
    InvalidJson(serde_json::Error),
    #[error("failed to deserialize project: {0}")]
    Deserialize(serde_json::Error),
    #[error("project schema version {0} has no migration path in this build")]
    UnsupportedSchemaVersion(u32),
    #[error(
        "project was written by a newer version of LaserPrep (schema {0}, this build supports up to {CURRENT_SCHEMA_VERSION}); refusing to modify it"
    )]
    NewerSchemaVersion(u32),
}

/// Migrates a raw JSON `.lvp` document to [`CURRENT_SCHEMA_VERSION`]
/// and deserializes it into [`ProjectFile`].
///
/// Mirrors `laserprep_settings::migrate`'s approach: a missing
/// `schemaVersion` is treated as schema `1`, and a `schemaVersion`
/// newer than this build understands is rejected rather than silently
/// reinterpreted (auto-ci Standard 7's forward-version handling rule).
pub fn migrate(mut data: Value) -> Result<ProjectFile, ProjectError> {
    let version = data
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;

    // Example for the next schema bump, once CURRENT_SCHEMA_VERSION > 1:
    // if version == 1 { data = migrate_v1_to_v2(data); version = 2; }
    if version < CURRENT_SCHEMA_VERSION {
        return Err(ProjectError::UnsupportedSchemaVersion(version));
    }

    if version > CURRENT_SCHEMA_VERSION {
        return Err(ProjectError::NewerSchemaVersion(version));
    }

    if let Value::Object(map) = &mut data {
        map.insert(
            "schemaVersion".to_string(),
            Value::from(CURRENT_SCHEMA_VERSION),
        );
    }

    serde_json::from_value(data).map_err(ProjectError::Deserialize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_v1_project() -> Value {
        json!({
            "schemaVersion": 1,
            "algorithmVersion": "0.1.0",
            "createdAt": "2026-01-01T00:00:00+00:00",
            "modifiedAt": "2026-01-01T00:00:00+00:00",
            "sourceImage": { "fileName": "a.png", "dataBase64": "eA==" },
            "params": {
                "dpi": 96.0,
                "toneCount": 5,
                "minAreaPx2": 16,
                "includeLegend": false,
                "mergeAdjacent": false
            }
        })
    }

    #[test]
    fn missing_schema_version_is_treated_as_v1() {
        let project = migrate(minimal_v1_project()).expect("should migrate");
        assert_eq!(project.schema_version, 1);
        assert_eq!(project.source_image.file_name.as_deref(), Some("a.png"));
    }

    #[test]
    fn missing_optional_fields_default_to_none() {
        let project = migrate(minimal_v1_project()).expect("should migrate");
        assert!(project.preset.is_none());
        assert!(project.classification.is_none());
        assert!(project.result.is_none());
    }

    #[test]
    fn missing_curve_simplification_defaults_to_vtracers_stock_tolerance() {
        // A project saved before this field existed must reconvert
        // with the same behavior it had at the time, not a value that
        // silently changes its output.
        let project = migrate(minimal_v1_project()).expect("should migrate");
        assert_eq!(project.params.curve_simplification, 4.0);
    }

    #[test]
    fn rejects_schema_version_newer_than_supported() {
        let mut data = minimal_v1_project();
        data["schemaVersion"] = json!(999);
        let err = migrate(data).unwrap_err();
        assert!(matches!(err, ProjectError::NewerSchemaVersion(999)));
    }

    #[test]
    fn preserves_unknown_top_level_fields() {
        let mut data = minimal_v1_project();
        data["someFutureFieldThisBuildDoesNotKnow"] = json!({ "nested": true });
        let project = migrate(data).expect("should migrate");
        assert_eq!(
            project.unknown.get("someFutureFieldThisBuildDoesNotKnow"),
            Some(&json!({ "nested": true }))
        );
    }
}
