//! `zax-harness packs <roms dir>`: how the built-in hack packs apply to each ROM of a folder (a
//! game's dumps, translations and ROM hacks): the patch found by its CRC or by a pack's
//! signatures, its components and groups that apply, and those that do not with why.

use std::fmt::Write as _;
use std::path::Path;

use zax_nes::builtin_patches;

use crate::HarnessError;

/// One line per ROM of `dir` (sorted by name), then a count of each outcome.
///
/// # Errors
/// [`HarnessError::Io`] when the folder or a file cannot be read.
pub fn run(dir: &Path) -> Result<String, HarnessError> {
    let mut names: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    names.sort();
    let mut out = String::new();
    let (mut whole, mut partial, mut none) = (0, 0, 0);
    for path in names {
        let image = std::fs::read(&path)?;
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let by_crc = builtin_patches::for_data_crc(builtin_patches::data_crc(&image)).is_some();
        let line = match builtin_patches::pack_for(&image) {
            None => {
                none += 1;
                "-".to_owned()
            }
            Some((patch, applied)) => {
                if applied.refused.is_empty() {
                    whole += 1;
                } else {
                    partial += 1;
                }
                let parts: Vec<String> = applied
                    .components
                    .iter()
                    .map(|component| format!("{}[{}]", component.name, component.groups.join(",")))
                    .collect();
                let refused: Vec<String> = applied
                    .refused
                    .iter()
                    .map(|(component, group, why)| format!("{component}/{group}:{why:?}"))
                    .collect();
                format!(
                    "{} by {}: {}{}",
                    patch.name,
                    if by_crc { "CRC" } else { "signatures" },
                    parts.join(" "),
                    if refused.is_empty() {
                        String::new()
                    } else {
                        format!(" | left out {}", refused.join(" "))
                    }
                )
            }
        };
        let _ = writeln!(out, "{line}  <- {name}");
    }
    let _ = writeln!(out, "whole: {whole}, in part: {partial}, none: {none}");
    Ok(out)
}
