//! `profile-dump` and `profile-build` (Rust-only): read a profile's patterns and their 3D looks,
//! and write a profile from a JSON description, for profiles made without the 3dSen editor (the
//! profiles of the homebrew games shipped with the program). The work is
//! [`zax_profiles::description`]; this module only reads the PNG textures.

use std::path::Path;

use zax_profiles::description::{self, RgbaPicture};

use crate::HarnessError;

/// The usage of the two commands.
pub const USAGE: &str = "zax-harness profile-dump <file.3dn> [--tiles]
  Lists the profile's patterns: family, grid, colours and the 3D look of each variant
  (depth, geometry, layer, scale, offset, flags, tags); --tiles adds the tile planes.

zax-harness profile-build <description.json> <out.3dn>
  Writes a profile from a JSON description (see zax-profiles/src/description.rs).";

/// A PNG file as the description reads it.
fn read_png(path: &Path) -> Result<RgbaPicture, String> {
    let image = zax_render::assets::png_image::load(path).map_err(|e| e.to_string())?;
    let pixels = (0..image.height)
        .flat_map(|y| (0..image.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (r, g, b, a) = image.pixel(x, y);
            [r, g, b, a]
        })
        .collect();
    Ok(RgbaPicture {
        width: image.width,
        height: image.height,
        pixels,
    })
}

/// `profile-dump`: the patterns of a profile, one block each.
///
/// # Errors
/// The file cannot be read or is no profile.
pub fn dump(path: &Path, tiles: bool) -> Result<String, HarnessError> {
    description::dump(path, tiles).map_err(HarnessError::Usage)
}

/// `profile-build`: writes the profile `description` describes to `out`.
///
/// # Errors
/// The description cannot be read or is malformed, or the profile cannot be written.
pub fn build(description: &Path, out: &Path) -> Result<String, HarnessError> {
    description::build(description, out, &read_png).map_err(HarnessError::Usage)
}
