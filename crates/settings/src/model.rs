use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The settings schema version this build of LaserPrep understands.
/// `Settings.schema_version` describes the configuration schema, not the
/// application version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    #[serde(default)]
    pub ui: UiSettings,
    #[serde(default)]
    pub updates: UpdateSettings,
    /// Fields this build doesn't recognize, preserved verbatim so an
    /// upgrade/downgrade cycle doesn't unnecessarily destroy data
    /// (auto-ci Standard 7, rule 6). Only top-level unknown fields are
    /// preserved this way; deeper unknown-field preservation inside
    /// `ui`/`updates` is not yet implemented.
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            ui: UiSettings::default(),
            updates: UpdateSettings::default(),
            unknown: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiSettings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub theme: Theme,
}

fn default_language() -> String {
    "en-US".to_string()
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: Theme::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettings {
    #[serde(default = "default_check_on_startup")]
    pub check_on_startup: bool,
    #[serde(default)]
    pub skipped_versions: Vec<String>,
}

fn default_check_on_startup() -> bool {
    true
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            check_on_startup: default_check_on_startup(),
            skipped_versions: Vec::new(),
        }
    }
}
