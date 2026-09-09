use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use laserprep_analysis::Classification;
use laserprep_optimize::LaserOptimizationScore;
use laserprep_presets::PresetName;
use laserprep_svggen::ValidationReport;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The `.lvp` schema version this build of LaserPrep understands.
/// Describes the project file's structure, not the application version
/// (see [`ProjectFile::algorithm_version`] for that).
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// A `.lvp` project (CLAUDE.md Section 12): the original image, the
/// conversion parameters applied to it, the preset they came from (if
/// any), the heuristic classification, and the last conversion result —
/// enough to reopen the project and continue editing without re-running
/// analysis from scratch, or to inspect exactly what produced a given
/// SVG later.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub schema_version: u32,
    /// The `laserprep-project` crate version that last wrote this file
    /// (CLAUDE.md Section 12: "versão do algoritmo"). Not a guarantee
    /// of bit-identical reconversion across versions, only a record of
    /// provenance.
    pub algorithm_version: String,
    /// RFC 3339 timestamp, set once when the project is first created.
    pub created_at: String,
    /// RFC 3339 timestamp, updated by [`ProjectFile::touch`] whenever
    /// the project is changed and about to be saved.
    pub modified_at: String,
    pub source_image: SourceImage,
    pub params: ConversionParams,
    #[serde(default)]
    pub preset: Option<PresetName>,
    #[serde(default)]
    pub classification: Option<Classification>,
    #[serde(default)]
    pub result: Option<ProjectResult>,
    /// Fields this build doesn't recognize, preserved verbatim so an
    /// upgrade/downgrade cycle doesn't unnecessarily destroy data
    /// (auto-ci Standard 7, rule 6).
    #[serde(flatten)]
    pub unknown: Map<String, Value>,
}

impl ProjectFile {
    /// Creates a new project around `source_image` and `params`, with
    /// no classification or result yet — the caller fills those in as
    /// the pipeline runs, then saves.
    pub fn new(
        source_image: SourceImage,
        params: ConversionParams,
        preset: Option<PresetName>,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            algorithm_version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: now.clone(),
            modified_at: now,
            source_image,
            params,
            preset,
            classification: None,
            result: None,
            unknown: Map::new(),
        }
    }

    /// Updates `modified_at` to the current time. Call before saving
    /// after any change to the project's parameters, preset, or result.
    pub fn touch(&mut self) {
        self.modified_at = Utc::now().to_rfc3339();
    }
}

/// The original raster image a project was created from, embedded
/// directly in the `.lvp` file (CLAUDE.md Section 12: "guardando
/// imagem original") so the file is a single portable artifact rather
/// than a reference that goes stale if the source file moves or the
/// project is shared with someone else.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceImage {
    /// The original file name, kept for display purposes only — not
    /// used to locate the image on disk.
    pub file_name: Option<String>,
    /// Base64-encoded original image bytes, exactly as imported
    /// (before any decoding/quantization).
    pub data_base64: String,
}

impl SourceImage {
    pub fn from_bytes(file_name: Option<String>, bytes: &[u8]) -> Self {
        Self {
            file_name,
            data_base64: BASE64.encode(bytes),
        }
    }

    /// Decodes the embedded image back to its original bytes.
    pub fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        BASE64.decode(&self.data_base64)
    }
}

/// The conversion parameters a project was run with — the same values
/// `laserprep_project`'s callers pass into
/// `laserprep::pipeline::convert_bytes_to_svg`, captured so the
/// project can be reopened and reconverted identically.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversionParams {
    pub dpi: f64,
    /// CLAUDE.md Section 7: 2-16 tones. Stored as a plain `u8` rather
    /// than `laserprep_domain::ToneCount` so a project written with a
    /// value a future/older build considers invalid can still be
    /// loaded and inspected instead of failing to deserialize; the
    /// value is validated again wherever it is actually used to
    /// reconvert.
    pub tone_count: u8,
    pub min_area_px2: u32,
    pub include_legend: bool,
    pub merge_adjacent: bool,
    /// CLAUDE.md Section 8's "Path Ordering" (`docs/roadmap.md` Phase
    /// 5). Defaults to `false` on projects saved before this field
    /// existed, matching their actual (unordered) behavior at the
    /// time.
    #[serde(default)]
    pub order_paths: bool,
}

/// The outcome of the last conversion run for this project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResult {
    pub svg: String,
    pub validation: ValidationReport,
    /// Absent on results saved before this field existed — recomputing
    /// it would need the source image decoded again, which a plain
    /// `#[serde(default)]` load shouldn't do as a side effect.
    #[serde(default)]
    pub optimization_score: Option<LaserOptimizationScore>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_image_round_trips_through_base64() {
        let original = b"not really an image, just some bytes";
        let source = SourceImage::from_bytes(Some("test.png".to_string()), original);
        assert_eq!(source.decode().unwrap(), original);
    }

    #[test]
    fn new_project_sets_matching_created_and_modified_timestamps() {
        let source = SourceImage::from_bytes(None, b"x");
        let params = ConversionParams {
            dpi: 96.0,
            tone_count: 5,
            min_area_px2: 16,
            include_legend: false,
            merge_adjacent: false,
            order_paths: false,
        };
        let project = ProjectFile::new(source, params, None);

        assert_eq!(project.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(project.created_at, project.modified_at);
        assert!(project.classification.is_none());
        assert!(project.result.is_none());
    }

    #[test]
    fn touch_advances_modified_at_without_changing_created_at() {
        let source = SourceImage::from_bytes(None, b"x");
        let params = ConversionParams {
            dpi: 96.0,
            tone_count: 5,
            min_area_px2: 16,
            include_legend: false,
            merge_adjacent: false,
            order_paths: false,
        };
        let mut project = ProjectFile::new(source, params, None);
        let created_at = project.created_at.clone();

        // Force a different timestamp instead of sleeping in a test.
        project.modified_at = "2000-01-01T00:00:00+00:00".to_string();
        project.touch();

        assert_eq!(project.created_at, created_at);
        assert_ne!(project.modified_at, "2000-01-01T00:00:00+00:00");
    }
}
