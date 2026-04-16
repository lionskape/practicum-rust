use std::{
    env::consts::EXE_EXTENSION,
    fs,
    path::{Path, PathBuf},
};

use assert_cmd::Command;
use image::{Rgba, RgbaImage};
use predicates::str::contains;
use serde_json::json;
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should exist")
        .to_path_buf()
}

fn target_debug_dir() -> PathBuf {
    workspace_root().join("target/debug")
}

fn binary_path(name: &str) -> PathBuf {
    let mut path = target_debug_dir().join(name);
    if !EXE_EXTENSION.is_empty() {
        path.set_extension(EXE_EXTENSION);
    }
    path
}

fn write_png(path: &Path, width: u32, height: u32, pixels: &[[u8; 4]]) {
    let mut image = RgbaImage::new(width, height);
    for (index, pixel) in pixels.iter().enumerate() {
        let x = (index as u32) % width;
        let y = (index as u32) / width;
        image.put_pixel(x, y, Rgba(*pixel));
    }
    image.save(path).expect("fixture png should be written");
}

fn read_png_pixels(path: &Path) -> Vec<[u8; 4]> {
    image::open(path)
        .expect("output png should be readable")
        .to_rgba8()
        .pixels()
        .map(|pixel| pixel.0)
        .collect()
}

fn write_json(path: &Path, value: serde_json::Value) {
    let payload = serde_json::to_vec(&value).expect("json fixture should serialize");
    fs::write(path, payload).expect("json fixture should be written");
}

fn assert_binary_exists() -> PathBuf {
    let path = binary_path("image_processor");
    assert!(
        path.exists(),
        "image_processor binary should be built before e2e tests: {}",
        path.display()
    );
    path
}

#[test]
fn mirror_plugin_processes_png_and_matches_expected_output() {
    let binary = assert_binary_exists();
    let temp = tempdir().expect("tempdir should exist");
    let input = temp.path().join("input.png");
    let output = temp.path().join("output.png");
    let params = temp.path().join("params.json");

    write_png(
        &input,
        2,
        2,
        &[[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]],
    );
    write_json(&params, json!({ "horizontal": true, "vertical": false }));

    Command::new(&binary)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--plugin")
        .arg("mirror_plugin")
        .arg("--params")
        .arg(&params)
        .arg("--plugin-path")
        .arg(target_debug_dir())
        .assert()
        .success();

    assert!(output.exists(), "output image should be created");
    assert_eq!(
        read_png_pixels(&output),
        vec![[0, 255, 0, 255], [255, 0, 0, 255], [255, 255, 255, 255], [0, 0, 255, 255],]
    );
}

#[test]
fn blur_plugin_processes_png_and_matches_expected_output() {
    let binary = assert_binary_exists();
    let temp = tempdir().expect("tempdir should exist");
    let input = temp.path().join("input.png");
    let output = temp.path().join("output.png");
    let params = temp.path().join("params.json");
    let input_pixels =
        vec![[10, 10, 10, 255], [20, 20, 20, 255], [30, 30, 30, 255], [40, 40, 40, 255]];

    write_png(&input, 2, 2, &input_pixels);
    write_json(&params, json!({ "radius": 1, "iterations": 1 }));

    Command::new(&binary)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--plugin")
        .arg("blur_plugin")
        .arg("--params")
        .arg(&params)
        .arg("--plugin-path")
        .arg(target_debug_dir())
        .assert()
        .success();

    let output_pixels = read_png_pixels(&output);
    assert_ne!(output_pixels, input_pixels, "blurred image should differ from input");
    assert_eq!(
        output_pixels,
        vec![[20, 20, 20, 255], [23, 23, 23, 255], [26, 26, 26, 255], [30, 30, 30, 255],]
    );
}

#[test]
fn missing_plugin_returns_a_clear_error() {
    let binary = assert_binary_exists();
    let temp = tempdir().expect("tempdir should exist");
    let input = temp.path().join("input.png");
    let output = temp.path().join("output.png");
    let params = temp.path().join("params.json");
    let plugin_dir = temp.path().join("plugins");
    fs::create_dir_all(&plugin_dir).expect("plugin dir should be created");

    write_png(&input, 1, 1, &[[1, 2, 3, 255]]);
    write_json(&params, json!({ "horizontal": true, "vertical": false }));

    Command::new(&binary)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--plugin")
        .arg("mirror_plugin")
        .arg("--params")
        .arg(&params)
        .arg("--plugin-path")
        .arg(&plugin_dir)
        .assert()
        .failure()
        .stderr(contains("plugin library not found"));
}

#[test]
fn invalid_json_params_fail_before_plugin_execution() {
    let binary = assert_binary_exists();
    let temp = tempdir().expect("tempdir should exist");
    let input = temp.path().join("input.png");
    let output = temp.path().join("output.png");
    let params = temp.path().join("params.json");

    write_png(&input, 1, 1, &[[1, 2, 3, 255]]);
    fs::write(&params, r#"{"horizontal":"#).expect("invalid json fixture should be written");

    Command::new(&binary)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--plugin")
        .arg("mirror_plugin")
        .arg("--params")
        .arg(&params)
        .arg("--plugin-path")
        .arg(target_debug_dir())
        .assert()
        .failure()
        .stderr(contains("does not contain valid JSON"));
}

#[test]
fn missing_input_fails_with_image_load_error() {
    let binary = assert_binary_exists();
    let temp = tempdir().expect("tempdir should exist");
    let output = temp.path().join("output.png");
    let params = temp.path().join("params.json");

    write_json(&params, json!({ "horizontal": true, "vertical": false }));

    Command::new(&binary)
        .arg("--input")
        .arg(temp.path().join("missing.png"))
        .arg("--output")
        .arg(&output)
        .arg("--plugin")
        .arg("mirror_plugin")
        .arg("--params")
        .arg(&params)
        .arg("--plugin-path")
        .arg(target_debug_dir())
        .assert()
        .failure()
        .stderr(contains("failed to load image"));
}
