use std::ffi::{CStr, c_char};

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct BlurParams {
    radius: u32,
    iterations: u32,
}

/// Applies an in-place box blur to the provided RGBA buffer.
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

    // SAFETY: The caller promises `rgba_data` points to a valid RGBA buffer of
    // `width * height * 4` bytes for the duration of this call.
    let rgba = unsafe { std::slice::from_raw_parts_mut(rgba_data, len) };
    apply_box_blur(width, height, rgba, &parsed_params);
}

fn parse_params(params: *const c_char) -> Option<BlurParams> {
    // SAFETY: The caller provides a valid NUL-terminated C string pointer.
    let raw = unsafe { CStr::from_ptr(params) };
    let text = raw.to_str().ok()?;
    let parsed: BlurParams = serde_json::from_str(text).ok()?;
    if parsed.radius == 0 || parsed.iterations == 0 {
        return None;
    }

    Some(parsed)
}

fn rgba_buffer_len(width: u32, height: u32) -> Option<usize> {
    let len = width.checked_mul(height)?.checked_mul(4)?;
    usize::try_from(len).ok()
}

fn apply_box_blur(width: u32, height: u32, rgba: &mut [u8], params: &BlurParams) {
    let Some((width, height)) = usize::try_from(width).ok().zip(usize::try_from(height).ok())
    else {
        return;
    };
    if width == 0 || height == 0 {
        return;
    }
    if rgba.len() != width.saturating_mul(height).saturating_mul(4) {
        return;
    }

    let radius = params.radius as i64;
    let max_x = width as i64 - 1;
    let max_y = height as i64 - 1;

    for _ in 0..params.iterations {
        let source = rgba.to_vec();
        for y in 0..height {
            for x in 0..width {
                let mut sums = [0_u64; 4];
                let mut count = 0_u64;

                for delta_y in -radius..=radius {
                    let yy = (y as i64 + delta_y).clamp(0, max_y) as usize;
                    for delta_x in -radius..=radius {
                        let xx = (x as i64 + delta_x).clamp(0, max_x) as usize;
                        let source_idx = pixel_offset(width, xx, yy);
                        for channel in 0..4 {
                            sums[channel] += u64::from(source[source_idx + channel]);
                        }
                        count += 1;
                    }
                }

                let target_idx = pixel_offset(width, x, y);
                for channel in 0..4 {
                    rgba[target_idx + channel] = (sums[channel] / count) as u8;
                }
            }
        }
    }
}

fn pixel_offset(width: usize, x: usize, y: usize) -> usize {
    (y * width + x) * 4
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::{BlurParams, apply_box_blur, process_image};

    fn grayscale_pixels() -> Vec<u8> {
        vec![
            10, 10, 10, 255, 20, 20, 20, 255, //
            30, 30, 30, 255, 40, 40, 40, 255,
        ]
    }

    #[test]
    fn one_iteration_radius_one_matches_expected_box_blur() {
        let mut rgba = grayscale_pixels();
        apply_box_blur(2, 2, &mut rgba, &BlurParams { radius: 1, iterations: 1 });

        assert_eq!(
            rgba,
            vec![
                20, 20, 20, 255, 23, 23, 23, 255, //
                26, 26, 26, 255, 30, 30, 30, 255,
            ]
        );
    }

    #[test]
    fn multiple_iterations_are_deterministic() {
        let mut rgba = grayscale_pixels();
        apply_box_blur(2, 2, &mut rgba, &BlurParams { radius: 1, iterations: 2 });

        assert_eq!(
            rgba,
            vec![
                23, 23, 23, 255, 24, 24, 24, 255, //
                25, 25, 25, 255, 26, 26, 26, 255,
            ]
        );
    }

    #[test]
    fn invalid_radius_or_iterations_result_in_safe_noop() {
        let mut rgba = grayscale_pixels();
        let expected = rgba.clone();
        let params = CString::new(r#"{"radius":0,"iterations":1}"#)
            .expect("params should be valid c string");

        // SAFETY: The buffer and params live for the duration of the call.
        unsafe {
            process_image(2, 2, rgba.as_mut_ptr(), params.as_ptr());
        }

        assert_eq!(rgba, expected);
    }
}
