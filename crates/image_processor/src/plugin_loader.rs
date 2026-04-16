use std::{
    ffi::c_char,
    path::{Path, PathBuf},
};

use libloading::Library;

use crate::ImageProcessorError;

pub(crate) type ProcessImageFn = unsafe extern "C" fn(u32, u32, *mut u8, *const c_char);

pub(crate) struct LoadedPlugin {
    _library: Library,
    process_image: ProcessImageFn,
}

impl LoadedPlugin {
    pub(crate) fn load(plugin_name: &str, plugin_dir: &Path) -> Result<Self, ImageProcessorError> {
        let path = library_path(plugin_dir, plugin_name);
        if !path.exists() {
            return Err(ImageProcessorError::PluginLibraryNotFound { path });
        }

        // SAFETY: The path points to a dynamic library selected by the user.
        // The loaded library is stored in the returned struct to keep it alive
        // for the entire lifetime of the function pointer.
        let library = unsafe { Library::new(&path) }
            .map_err(|source| ImageProcessorError::PluginLoad { path: path.clone(), source })?;

        // SAFETY: `process_image` is the only supported ABI symbol for plugins.
        // We immediately copy the symbol into a plain function pointer and keep
        // the `Library` alive in the same struct.
        let process_image = load_symbol(&library, &path)?;

        Ok(Self { _library: library, process_image })
    }

    pub(crate) unsafe fn process_image(
        &self,
        width: u32,
        height: u32,
        rgba_data: *mut u8,
        params: *const c_char,
    ) {
        unsafe {
            (self.process_image)(width, height, rgba_data, params);
        }
    }
}

fn load_symbol(library: &Library, path: &Path) -> Result<ProcessImageFn, ImageProcessorError> {
    let symbol = unsafe {
        library.get::<ProcessImageFn>(b"process_image").map_err(|source| {
            ImageProcessorError::PluginSymbolLoad {
                path: path.to_path_buf(),
                symbol: "process_image".to_owned(),
                source,
            }
        })?
    };

    Ok(*symbol)
}

pub(crate) fn library_path(plugin_dir: &Path, plugin_name: &str) -> PathBuf {
    plugin_dir.join(library_filename(plugin_name))
}

pub(crate) fn library_filename(plugin_name: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!("{plugin_name}.dll")
    }

    #[cfg(target_os = "macos")]
    {
        format!("lib{plugin_name}.dylib")
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        format!("lib{plugin_name}.so")
    }
}

#[cfg(test)]
mod tests {
    use super::library_filename;

    #[test]
    fn resolves_current_platform_library_name() {
        let expected = if cfg!(target_os = "windows") {
            "mirror_plugin.dll"
        } else if cfg!(target_os = "macos") {
            "libmirror_plugin.dylib"
        } else {
            "libmirror_plugin.so"
        };

        assert_eq!(library_filename("mirror_plugin"), expected);
    }
}
