use std::{ffi::NulError, io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageProcessorError {
    #[error("failed to load image from '{path}': {source}")]
    InputImageLoad {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    #[error("failed to read params from '{path}': {source}")]
    ParamsRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("params file '{path}' contains an interior NUL byte: {source}")]
    ParamsContainNul {
        path: PathBuf,
        #[source]
        source: NulError,
    },
    #[error("params file '{path}' does not contain valid JSON: {source}")]
    ParamsInvalidJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("buffer size overflow for image dimensions {width}x{height}")]
    BufferSizeOverflow { width: u32, height: u32 },
    #[error("unexpected RGBA buffer length: expected {expected} bytes, got {actual}")]
    UnexpectedRgbaBufferLen { expected: usize, actual: usize },
    #[error("plugin library not found at '{path}'")]
    PluginLibraryNotFound { path: PathBuf },
    #[error("failed to load plugin library from '{path}': {source}")]
    PluginLoad {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },
    #[error("failed to load symbol '{symbol}' from '{path}': {source}")]
    PluginSymbolLoad {
        path: PathBuf,
        symbol: String,
        #[source]
        source: libloading::Error,
    },
    #[error("failed to save image to '{path}': {source}")]
    OutputImageSave {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
}
