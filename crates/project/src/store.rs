use crate::migrate::{ProjectError, migrate};
use crate::model::ProjectFile;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Reads and atomically writes a [`ProjectFile`] at a fixed `.lvp`
/// path. Mirrors `laserprep_settings::SettingsStore`'s save/load/backup
/// shape.
pub struct ProjectStore {
    path: PathBuf,
}

impl ProjectStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads and migrates the project at this store's path. Unlike
    /// settings, a missing project file is an error rather than a
    /// default value — there is no meaningful "default project"
    /// without a source image.
    pub fn load(&self) -> Result<ProjectFile, ProjectError> {
        let raw = fs::read_to_string(&self.path)?;
        let value: Value = serde_json::from_str(&raw).map_err(ProjectError::InvalidJson)?;
        migrate(value)
    }

    /// Atomically writes `project`: serialize to a temporary file in
    /// the same directory, then rename over the destination. Rename is
    /// atomic on both Windows and POSIX filesystems when source and
    /// destination share a directory.
    pub fn save(&self, project: &ProjectFile) -> Result<(), ProjectError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(project).map_err(ProjectError::Deserialize)?;
        let tmp_path = self.path.with_extension("lvp.tmp");
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, &self.path)?;
        Ok(())
    }

    /// Copies the current on-disk project file to `<name>.lvp.bak`, if
    /// it exists. Call this before performing a destructive/non-trivial
    /// migration.
    pub fn backup(&self) -> Result<(), ProjectError> {
        if self.path.exists() {
            fs::copy(&self.path, self.path.with_extension("lvp.bak"))?;
        }
        Ok(())
    }
}

/// Distinguishes "file does not exist" from other I/O failures, for
/// callers that want to offer "create a new project" instead of
/// surfacing a raw error when opening a path for the first time.
impl ProjectError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, ProjectError::Io(err) if err.kind() == io::ErrorKind::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ConversionParams, SourceImage};

    fn store_in_temp_dir() -> (tempfile::TempDir, ProjectStore) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let store = ProjectStore::new(dir.path().join("project.lvp"));
        (dir, store)
    }

    fn sample_project() -> ProjectFile {
        let source = SourceImage::from_bytes(Some("dog.jpg".to_string()), b"fake-image-bytes");
        let params = ConversionParams {
            dpi: 96.0,
            tone_count: 5,
            min_area_px2: 16,
            curve_simplification: 4.0,
            include_legend: true,
            merge_adjacent: false,
            order_paths: false,
        };
        ProjectFile::new(source, params, None)
    }

    #[test]
    fn loading_a_missing_file_is_a_not_found_error() {
        let (_dir, store) = store_in_temp_dir();
        let err = store.load().unwrap_err();
        assert!(err.is_not_found());
    }

    #[test]
    fn round_trips_through_save_and_load() {
        let (_dir, store) = store_in_temp_dir();
        let project = sample_project();

        store.save(&project).expect("save should succeed");
        let loaded = store.load().expect("load should succeed");

        assert_eq!(loaded, project);
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temp_file_behind() {
        let (_dir, store) = store_in_temp_dir();
        store.save(&sample_project()).expect("save should succeed");

        assert!(store.path().exists());
        assert!(!store.path().with_extension("lvp.tmp").exists());
    }

    #[test]
    fn corrupt_json_is_preserved_and_reported() {
        let (_dir, store) = store_in_temp_dir();
        fs::write(store.path(), b"{ not valid json").unwrap();

        let err = store.load().unwrap_err();
        assert!(matches!(err, ProjectError::InvalidJson(_)));
        let contents = fs::read_to_string(store.path()).unwrap();
        assert_eq!(contents, "{ not valid json");
    }

    #[test]
    fn backup_copies_existing_file_to_bak_suffix() {
        let (_dir, store) = store_in_temp_dir();
        store.save(&sample_project()).expect("save should succeed");

        store.backup().expect("backup should succeed");

        let backup_path = store.path().with_extension("lvp.bak");
        assert!(backup_path.exists());
    }

    #[test]
    fn backup_is_a_no_op_when_no_file_exists_yet() {
        let (_dir, store) = store_in_temp_dir();
        store
            .backup()
            .expect("backup should succeed even with no file");
        assert!(!store.path().with_extension("lvp.bak").exists());
    }

    #[test]
    fn round_trips_a_result_and_classification() {
        let (_dir, store) = store_in_temp_dir();
        let mut project = sample_project();
        project.classification = Some(laserprep_analysis::Classification {
            category: laserprep_analysis::ContentCategory::Logo,
            confidence: 0.75,
        });
        project.result = Some(crate::model::ProjectResult {
            svg: "<svg></svg>".to_string(),
            validation: laserprep_svggen::validate("<svg></svg>", 10, 10, 5),
            optimization_score: Some(laserprep_optimize::LaserOptimizationScore {
                travel_distance: 0.0,
                optimal_travel_distance: 0.0,
                efficiency: 1.0,
            }),
        });

        store.save(&project).expect("save should succeed");
        let loaded = store.load().expect("load should succeed");

        assert_eq!(loaded, project);
    }
}
