//! Core domain types shared across LaserPrep's crates.
//!
//! This crate has no I/O and depends on nothing else in the workspace, so
//! every other crate can depend on it without pulling in unrelated
//! concerns. Types here are added as later phases need them (Image,
//! ToneSet, VectorPath, Project, ...) — this crate intentionally starts
//! small rather than pre-declaring empty placeholder types.

mod length;
mod tone_count;

pub use length::Length;
pub use tone_count::{ToneCount, ToneCountError};
