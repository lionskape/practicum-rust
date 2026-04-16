use std::ffi::{CStr, c_char};

use serde::Deserialize;

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
struct MirrorParams {
    #[serde(default)]
    horizontal: bool,
    #[serde(default)]
    vertical: bool,
}

/// Applies an in-place mirror transformation to the provided RGBA buffer.
///
/// # Safety
///
/// - `rgba_data` must point to a valid mutable buffer of `width * height * 4` bytes.
/// - `params` must point to a valid NUL-terminated UTF-8 C string.
/// - Both pointers must stay valid for the entire duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) {
    let Some(len) = rgba_buffer_len(width, height) else {
        return;
    };
    if rgba_data.is_null() || params.is_null() {
        return;
    }

    let Some(parsed_params) = parse_params(params) else {
        return;
    };

    // SAFETY: The caller promises `rgba_data` points to a valid buffer of
    // `width * height * 4` bytes for the duration of this call.
    let rgba = unsafe { std::slice::from_raw_parts_mut(rgba_data, len) };
    apply_mirror(width, height, rgba, &parsed_params);
}

fn parse_params(params: *const c_char) -> Option<MirrorParams> {
    // SAFETY: The caller provides a valid NUL-terminated C string pointer.
    let raw = unsafe { CStr::from_ptr(params) };
    let text = raw.to_str().ok()?;
    serde_json::from_str(text).ok()
}

fn rgba_buffer_len(width: u32, height: u32) -> Option<usize> {
    let len = width.checked_mul(height)?.checked_mul(4)?;
    usize::try_from(len).ok()
}

fn apply_mirror(width: u32, height: u32, rgba: &mut [u8], params: &MirrorParams) {
    let Some((width, height)) = usize::try_from(width).ok().zip(usize::try_from(height).ok())
    else {
        return;
    };
    if rgba.len() != width.saturating_mul(height).saturating_mul(4) {
        return;
    }
    if !params.horizontal && !params.vertical {
        return;
    }

    if params.horizontal {
        for y in 0..height {
            for x in 0..(width / 2) {
                let left = y * width + x;
                let right = y * width + (width - 1 - x);
                swap_pixels(rgba, left, right);
            }
        }
    }

    if params.vertical {
        for y in 0..(height / 2) {
            let opposite_y = height - 1 - y;
            for x in 0..width {
                let top = y * width + x;
                let bottom = opposite_y * width + x;
                swap_pixels(rgba, top, bottom);
            }
        }
    }
}

fn swap_pixels(rgba: &mut [u8], left: usize, right: usize) {
    let left_offset = left * 4;
    let right_offset = right * 4;
    for channel in 0..4 {
        rgba.swap(left_offset + channel, right_offset + channel);
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::{MirrorParams, apply_mirror, process_image};

    fn sample_pixels() -> Vec<u8> {
        vec![
            1, 0, 0, 255, 2, 0, 0, 255, //
            3, 0, 0, 255, 4, 0, 0, 255,
        ]
    }

    #[test]
    fn mirrors_horizontally() {
        let mut rgba = sample_pixels();
        apply_mirror(2, 2, &mut rgba, &MirrorParams { horizontal: true, vertical: false });

        assert_eq!(
            rgba,
            vec![
                2, 0, 0, 255, 1, 0, 0, 255, //
                4, 0, 0, 255, 3, 0, 0, 255,
            ]
        );
    }

    #[test]
    fn mirrors_vertically() {
        let mut rgba = sample_pixels();
        apply_mirror(2, 2, &mut rgba, &MirrorParams { horizontal: false, vertical: true });

        assert_eq!(
            rgba,
            vec![
                3, 0, 0, 255, 4, 0, 0, 255, //
                1, 0, 0, 255, 2, 0, 0, 255,
            ]
        );
    }

    #[test]
    fn mirrors_horizontally_and_vertically() {
        let mut rgba = sample_pixels();
        apply_mirror(2, 2, &mut rgba, &MirrorParams { horizontal: true, vertical: true });

        assert_eq!(
            rgba,
            vec![
                4, 0, 0, 255, 3, 0, 0, 255, //
                2, 0, 0, 255, 1, 0, 0, 255,
            ]
        );
    }

    #[test]
    fn noop_when_both_flags_are_false() {
        let mut rgba = sample_pixels();
        let expected = rgba.clone();

        apply_mirror(2, 2, &mut rgba, &MirrorParams::default());

        assert_eq!(rgba, expected);
    }

    #[test]
    fn invalid_json_causes_safe_noop() {
        let mut rgba = sample_pixels();
        let expected = rgba.clone();
        let params =
            CString::new(r#"{"horizontal":"oops"}"#).expect("params should be valid c string");

        // SAFETY: The buffer and params live for the duration of the call.
        unsafe {
            process_image(2, 2, rgba.as_mut_ptr(), params.as_ptr());
        }

        assert_eq!(rgba, expected);
    }
}
