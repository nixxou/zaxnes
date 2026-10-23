//! The cover art of the library entries (SPEC-09 REQ-PERS-036, Q-050): a PNG in the covers
//! folder, either a capture of the game or an imported image fitted to 256 × 192.

use std::fs;
use std::path::Path;

use zax_render::assets::png_image;
use zax_render::assets::ImageData;

use crate::user_data_paths::{remove_file_if_present, UserDataPaths};

/// Widest cover.
pub const MAXIMUM_WIDTH: usize = 256;
/// Tallest cover.
pub const MAXIMUM_HEIGHT: usize = 192;
/// Text between the title and the identifier in a cover's file name.
pub const NAME_INFIX: &str = "_3dsen_";
/// Notification text of an import that is not a readable PNG.
pub const UNSUPPORTED_SOURCE_MESSAGE: &str = "Only PNG images can be imported as cover art.";

const RESERVED_EVERYWHERE: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Replaces every character a file name cannot hold, on any platform, with an underscore.
#[must_use]
pub fn sanitise_file_name(name: &str) -> String {
    name.chars()
        .map(|value| {
            if value < ' ' || RESERVED_EVERYWHERE.contains(&value) {
                '_'
            } else {
                value
            }
        })
        .collect()
}

/// The file name of the cover of the entry `id` titled `title`.
#[must_use]
pub fn cover_file_name(title: &str, id: &str) -> String {
    format!("{}.png", sanitise_file_name(&format!("{title}{NAME_INFIX}{id}")))
}

/// The cover file of an entry.
#[must_use]
pub fn cover_file(paths: &UserDataPaths, title: &str, id: &str) -> std::path::PathBuf {
    paths.covers_directory.join(cover_file_name(title, id))
}

/// Fits the PNG at `source_path` into 256 × 192 and stores it as the cover of the entry.
///
/// # Errors
/// The text to show the user: the source is not a readable PNG, or the cover could not be
/// written.
pub fn import_cover(
    paths: &UserDataPaths,
    title: &str,
    id: &str,
    source_path: &Path,
) -> Result<String, String> {
    let image = png_image::load(source_path).map_err(|_| UNSUPPORTED_SOURCE_MESSAGE.to_owned())?;
    let (width, height) = fitted_size(image.width, image.height);
    let pixels = resample(&image, width, height);
    write(paths, title, id, width, height, &pixels)
}

/// Stores a 256 × 192 RGB capture of the game as the cover of the entry. Returns `None` when
/// the capture is too short to be one.
///
/// # Errors
/// The text to show the user: the cover could not be written.
#[must_use]
pub fn set_cover_from_capture(
    paths: &UserDataPaths,
    title: &str,
    id: &str,
    capture: &[u8],
) -> Option<Result<String, String>> {
    let size = MAXIMUM_WIDTH * MAXIMUM_HEIGHT * 3;
    let capture = capture.get(..size)?;
    Some(write(paths, title, id, MAXIMUM_WIDTH, MAXIMUM_HEIGHT, capture))
}

/// Deletes the cover of an entry: both the derived name and whatever the entry recorded, so a
/// cover imported by an earlier version, or renamed by hand, goes too. A cover that cannot be
/// removed must not stop the game from being deleted.
pub fn delete_cover(paths: &UserDataPaths, title: &str, id: &str, stored_name: Option<&str>) {
    let derived = cover_file_name(title, id);
    for name in [stored_name, Some(derived.as_str())].into_iter().flatten() {
        if name.is_empty() {
            continue;
        }
        let Some(file_name) = Path::new(name).file_name() else {
            continue;
        };
        remove_file_if_present(&paths.covers_directory.join(file_name));
    }
}

/// The size of an image of `width` × `height` pixels once fitted into 256 × 192 without
/// enlarging it.
///
/// # Panics
/// Never: a zero dimension is treated as one pixel.
#[must_use]
pub fn fitted_size(width: usize, height: usize) -> (usize, usize) {
    let width = width.max(1);
    let height = height.max(1);
    #[allow(clippy::cast_precision_loss)] // image sizes are far below 2^24
    let factor = (width as f32 / MAXIMUM_WIDTH as f32).max(height as f32 / MAXIMUM_HEIGHT as f32);
    if factor <= 1.0 {
        return (width, height);
    }
    (
        scaled(width, factor).clamp(1, MAXIMUM_WIDTH),
        scaled(height, factor).clamp(1, MAXIMUM_HEIGHT),
    )
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
// image sizes are far below 2^24 and the quotient is positive, as the C# MathF.Round cast
fn scaled(size: usize, factor: f32) -> usize {
    (size as f32 / factor).round() as usize
}

fn write(
    paths: &UserDataPaths,
    title: &str,
    id: &str,
    width: usize,
    height: usize,
    rgb: &[u8],
) -> Result<String, String> {
    let name = cover_file_name(title, id);
    let encoded = png_image::encode_rgb(to_u32(width), to_u32(height), rgb).map_err(|error| error.to_string())?;
    fs::create_dir_all(&paths.covers_directory).map_err(|error| error.to_string())?;
    fs::write(paths.covers_directory.join(&name), encoded).map_err(|error| error.to_string())?;
    Ok(name)
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn resample(image: &ImageData, width: usize, height: usize) -> Vec<u8> {
    let mut target = vec![0_u8; width * height * 3];
    for y in 0..height {
        let first_row = y * image.height / height;
        let last_row = (first_row + 1).max((y + 1) * image.height / height);
        for x in 0..width {
            let first_column = x * image.width / width;
            let last_column = (first_column + 1).max((x + 1) * image.width / width);
            let mut sums = [0_usize; 3];
            let mut count = 0_usize;
            for row in first_row..last_row {
                let line = row * image.width * 4;
                for column in first_column..last_column {
                    let at = line + column * 4;
                    for (sum, value) in sums.iter_mut().zip(&image.pixels[at..at + 3]) {
                        *sum += usize::from(*value);
                    }
                    count += 1;
                }
            }
            let offset = (y * width + x) * 3;
            for (slot, sum) in target[offset..offset + 3].iter_mut().zip(sums) {
                *slot = u8::try_from(sum / count).unwrap_or(u8::MAX);
            }
        }
    }
    target
}
