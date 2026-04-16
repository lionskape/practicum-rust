mod error;
mod plugin_loader;

use std::{
    ffi::CString,
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
pub use error::ImageProcessorError;
use image::{RgbaImage, open};
use log::debug;
use serde_json::Value;

use crate::plugin_loader::LoadedPlugin;

#[derive(Debug, Clone, Parser)]
#[command(about = "Apply image-processing plugins over FFI", version, propagate_version = true)]
pub struct Cli {
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub plugin: String,
    #[arg(long)]
    pub params: PathBuf,
    #[arg(long, default_value = "target/debug")]
    pub plugin_path: PathBuf,
}

pub fn run(cli: Cli) -> Result<(), ImageProcessorError> {
    debug!("loading input image from {:?}", cli.input);
    let image = open(&cli.input).map_err(|source| ImageProcessorError::InputImageLoad {
        path: cli.input.clone(),
        source,
    })?;

    let rgba_image = image.to_rgba8();
    let (width, height) = rgba_image.dimensions();
    let mut rgba_bytes = rgba_image.into_raw();
    validate_buffer_len(width, height, &rgba_bytes)?;

    let params = load_params(&cli.params)?;
    let plugin = LoadedPlugin::load(&cli.plugin, &cli.plugin_path)?;

    // SAFETY: `rgba_bytes` is a live mutable RGBA buffer with length validated
    // against `width * height * 4`, and `params` is an owned `CString` kept
    // alive for the entire duration of the plugin call.
    unsafe {
        plugin.process_image(width, height, rgba_bytes.as_mut_ptr(), params.as_ptr());
    }

    let expected_len = expected_buffer_len(width, height)?;
    let actual_len = rgba_bytes.len();
    let output_image = RgbaImage::from_raw(width, height, rgba_bytes).ok_or(
        ImageProcessorError::UnexpectedRgbaBufferLen { expected: expected_len, actual: actual_len },
    )?;

    debug!("saving output image to {:?}", cli.output);
    output_image
        .save(&cli.output)
        .map_err(|source| ImageProcessorError::OutputImageSave { path: cli.output, source })?;

    Ok(())
}

fn load_params(path: &Path) -> Result<CString, ImageProcessorError> {
    let raw = fs::read_to_string(path)
        .map_err(|source| ImageProcessorError::ParamsRead { path: path.to_path_buf(), source })?;

    let params = CString::new(raw.clone()).map_err(|source| {
        ImageProcessorError::ParamsContainNul { path: path.to_path_buf(), source }
    })?;

    let _ = serde_json::from_str::<Value>(&raw).map_err(|source| {
        ImageProcessorError::ParamsInvalidJson { path: path.to_path_buf(), source }
    })?;

    Ok(params)
}

fn validate_buffer_len(
    width: u32,
    height: u32,
    rgba_bytes: &[u8],
) -> Result<(), ImageProcessorError> {
    let expected = expected_buffer_len(width, height)?;
    let actual = rgba_bytes.len();
    if actual != expected {
        return Err(ImageProcessorError::UnexpectedRgbaBufferLen { expected, actual });
    }

    Ok(())
}

fn expected_buffer_len(width: u32, height: u32) -> Result<usize, ImageProcessorError> {
    let pixels = width
        .checked_mul(height)
        .and_then(|count| count.checked_mul(4))
        .ok_or(ImageProcessorError::BufferSizeOverflow { width, height })?;

    usize::try_from(pixels).map_err(|_| ImageProcessorError::BufferSizeOverflow { width, height })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    use super::{Cli, ImageProcessorError, run};

    fn write_png(path: &Path) {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([12, 34, 56, 255]));
        image.save(path).expect("fixture image should be written");
    }

    fn write_text(path: &Path, text: &str) {
        fs::write(path, text).expect("fixture text should be written");
    }

    #[test]
    fn missing_input_file_returns_image_error() {
        let temp = tempdir().expect("tempdir should be created");
        let params = temp.path().join("params.json");
        write_text(&params, r#"{"horizontal":true}"#);

        let error = run(Cli {
            input: temp.path().join("missing.png"),
            output: temp.path().join("output.png"),
            plugin: "mirror_plugin".to_owned(),
            params,
            plugin_path: temp.path().to_path_buf(),
        })
        .expect_err("missing input should fail");

        assert!(matches!(error, ImageProcessorError::InputImageLoad { .. }));
    }

    #[test]
    fn missing_params_file_returns_params_read_error() {
        let temp = tempdir().expect("tempdir should be created");
        let input = temp.path().join("input.png");
        write_png(&input);

        let error = run(Cli {
            input,
            output: temp.path().join("output.png"),
            plugin: "mirror_plugin".to_owned(),
            params: temp.path().join("missing.json"),
            plugin_path: temp.path().to_path_buf(),
        })
        .expect_err("missing params should fail");

        assert!(matches!(error, ImageProcessorError::ParamsRead { .. }));
    }

    #[test]
    fn missing_plugin_library_returns_plugin_not_found_error() {
        let temp = tempdir().expect("tempdir should be created");
        let input = temp.path().join("input.png");
        let params = temp.path().join("params.json");
        write_png(&input);
        write_text(&params, r#"{"horizontal":true}"#);

        let error = run(Cli {
            input,
            output: temp.path().join("output.png"),
            plugin: "mirror_plugin".to_owned(),
            params,
            plugin_path: temp.path().to_path_buf(),
        })
        .expect_err("missing plugin should fail");

        assert!(matches!(error, ImageProcessorError::PluginLibraryNotFound { .. }));
    }

    #[test]
    fn invalid_json_params_are_rejected_before_plugin_load() {
        let temp = tempdir().expect("tempdir should be created");
        let input = temp.path().join("input.png");
        let params = temp.path().join("params.json");
        write_png(&input);
        write_text(&params, r#"{"horizontal":"#);

        let error = run(Cli {
            input,
            output: temp.path().join("output.png"),
            plugin: "mirror_plugin".to_owned(),
            params,
            plugin_path: temp.path().to_path_buf(),
        })
        .expect_err("invalid json should fail");

        assert!(matches!(error, ImageProcessorError::ParamsInvalidJson { .. }));
    }

    #[test]
    fn params_with_interior_nul_are_rejected() {
        let temp = tempdir().expect("tempdir should be created");
        let input = temp.path().join("input.png");
        let params = temp.path().join("params.json");
        write_png(&input);
        fs::write(&params, b"{\"horizontal\":\0true}").expect("fixture bytes should be written");

        let error = run(Cli {
            input,
            output: temp.path().join("output.png"),
            plugin: "mirror_plugin".to_owned(),
            params,
            plugin_path: temp.path().to_path_buf(),
        })
        .expect_err("interior nul should fail");

        assert!(matches!(error, ImageProcessorError::ParamsContainNul { .. }));
    }
}
