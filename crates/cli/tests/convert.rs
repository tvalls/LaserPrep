//! End-to-end tests that exercise the compiled `laser-vector` binary
//! as a real user would, via `std::process::Command` — not just the
//! library functions behind it (`CARGO_BIN_EXE_laser-vector` is set by
//! Cargo for integration tests of a crate with a `[[bin]]`).

use image::{ImageBuffer, Rgb};
use std::process::Command;

fn synthetic_png(dir: &std::path::Path) -> std::path::PathBuf {
    let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(40, 40, Rgb([255, 255, 255]));
    for y in 10..30 {
        for x in 10..30 {
            buffer.put_pixel(x, y, Rgb([0, 0, 0]));
        }
    }
    let path = dir.join("input.png");
    buffer.save(&path).unwrap();
    path
}

fn laser_vector() -> Command {
    Command::new(env!("CARGO_BIN_EXE_laser-vector"))
}

/// A staircase boundary (jagged at pixel scale) — the kind of
/// high-frequency edge real photo texture (skin, fur, JPEG compression
/// blocks) produces, so `--curve-simplification` has something real to
/// simplify away.
fn staircase_png(dir: &std::path::Path) -> std::path::PathBuf {
    let size = 60u32;
    let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(size, size, Rgb([255, 255, 255]));
    for y in 0..size {
        for x in 0..size {
            if x < y {
                buffer.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
    }
    let path = dir.join("staircase.png");
    buffer.save(&path).unwrap();
    path
}

#[test]
fn converts_an_image_to_svg_with_a_preset() {
    let dir = tempfile::tempdir().unwrap();
    let input = synthetic_png(dir.path());
    let output = dir.path().join("output.svg");

    let result = laser_vector()
        .arg("convert")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--preset")
        .arg("logo")
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let svg = std::fs::read_to_string(&output).unwrap();
    assert!(svg.starts_with("<svg "));
    assert!(svg.contains("<path"));

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("classified as"));
    assert!(stdout.contains("validation:"));
}

#[test]
fn explicit_tones_flag_overrides_the_preset_default() {
    let dir = tempfile::tempdir().unwrap();
    let input = synthetic_png(dir.path());
    let output = dir.path().join("output.svg");

    let result = laser_vector()
        .arg("convert")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--preset")
        .arg("photo")
        .arg("--tones")
        .arg("3")
        .output()
        .unwrap();

    assert!(result.status.success());
    let svg = std::fs::read_to_string(&output).unwrap();
    assert!(svg.contains("<g id=\"tone-0\""));
    assert!(svg.contains("<g id=\"tone-1\""));
    assert!(svg.contains("<g id=\"tone-2\""));
    assert!(!svg.contains("<g id=\"tone-3\""));
}

#[test]
fn order_paths_flag_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let input = synthetic_png(dir.path());
    let output = dir.path().join("output.svg");

    let result = laser_vector()
        .arg("convert")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--order-paths")
        .output()
        .unwrap();

    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("path ordering efficiency"));
}

#[test]
fn curve_simplification_flag_reduces_reported_node_count() {
    let dir = tempfile::tempdir().unwrap();
    let input = staircase_png(dir.path());

    let node_count_for = |curve_simplification: &str| -> u64 {
        let output = dir.path().join(format!("out-{curve_simplification}.svg"));
        let result = laser_vector()
            .arg("convert")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--preset")
            .arg("photo")
            .arg("--tones")
            .arg("2")
            .arg("--min-area")
            .arg("1")
            .arg("--curve-simplification")
            .arg(curve_simplification)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
        let line = stdout
            .lines()
            .find(|line| line.starts_with("validation:"))
            .expect("a validation summary line");
        // "validation: N paths (... ), M nodes, LightBurn-compatible: ..."
        let words: Vec<&str> = line.split_whitespace().collect();
        let nodes_index = words
            .iter()
            .position(|&w| w == "nodes,")
            .expect("a 'nodes,' token");
        words[nodes_index - 1]
            .parse()
            .expect("node count should be a number")
    };

    let detailed = node_count_for("0.5");
    let simplified = node_count_for("30.0");

    assert!(
        simplified < detailed,
        "expected fewer nodes at higher curve simplification: {simplified} (simplified) vs {detailed} (detailed)"
    );
}

#[test]
fn reports_a_clear_error_for_a_missing_input_file_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.svg");

    let result = laser_vector()
        .arg("convert")
        .arg(dir.path().join("does-not-exist.png"))
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("error:"));
    assert!(!output.exists());
}

#[test]
fn rejects_an_invalid_tone_count_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let input = synthetic_png(dir.path());
    let output = dir.path().join("output.svg");

    let result = laser_vector()
        .arg("convert")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--tones")
        .arg("1")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(!output.exists());
}

#[test]
fn presets_subcommand_lists_every_built_in_preset() {
    let result = laser_vector().arg("presets").output().unwrap();

    assert!(result.status.success());
    let stdout = String::from_utf8_lossy(&result.stdout);
    for name in [
        "photo",
        "portrait",
        "animal",
        "logo",
        "drawing",
        "landscape",
    ] {
        assert!(
            stdout.contains(name),
            "missing preset '{name}' in: {stdout}"
        );
    }
}
