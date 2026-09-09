//! The `.lvp` project file format (CLAUDE.md Section 12): original
//! image, parameters, preset, analysis result, tones, dimensions, and
//! conversion result, plus the algorithm version that produced it.
//!
//! A `.lvp` file is a single self-contained JSON document (the source
//! image is embedded as base64, see [`SourceImage`]) so a project can
//! be copied, backed up, or shared without carrying a separate image
//! file alongside it. Reading/writing follows the same
//! migrate-then-deserialize, atomic-write-then-rename shape as
//! `laserprep_settings`.

mod migrate;
mod model;
mod store;

pub use migrate::{ProjectError, migrate};
pub use model::{
    CURRENT_SCHEMA_VERSION, ConversionParams, ProjectFile, ProjectResult, SourceImage,
};
pub use store::ProjectStore;
