//! Builds the list of hack packs built into the binary (see `src/pack_files.rs`).
//!
//! The packs are the `packs` folder of a copy of the `zaxnes_3dn` repository (under a licence of
//! their own): `ZAXNES_3DN_DIR`, else a `zaxnes_3dn` folder beside the workspace or one or two
//! levels above it, as `zax-app/build.rs` finds the 3D profiles. `ZAXNES_EMBED_PACKS=none` builds
//! none in (the desktop packages, which ship a `zxpk` folder instead); `ZAXNES_EMBED_PACKS`
//! naming a folder builds that folder's packs in. Without packs, their games run without their
//! widescreen.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    println!("cargo:rerun-if-env-changed=ZAXNES_EMBED_PACKS");
    println!("cargo:rerun-if-env-changed=ZAXNES_3DN_DIR");
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR")?);
    let folder = match env::var("ZAXNES_EMBED_PACKS") {
        Ok(value) if value.eq_ignore_ascii_case("none") => None,
        Ok(value) if !value.is_empty() => {
            let folder = PathBuf::from(value);
            if !folder.is_dir() {
                return Err(format!(
                    "ZAXNES_EMBED_PACKS: '{}' is not a folder",
                    folder.display()
                ));
            }
            Some(folder)
        }
        _ => repository_packs(&manifest),
    };
    let mut code = String::from("/// File name and bytes of each pack built in.\n");
    code.push_str("pub static PACKS: &[(&str, &[u8])] = &[\n");
    match folder {
        Some(folder) => {
            println!("cargo:rerun-if-changed={}", folder.display());
            for path in packs_in(&folder)? {
                println!("cargo:rerun-if-changed={}", path.display());
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| format!("'{}': no file name", path.display()))?;
                let absolute = fs::canonicalize(&path).map_err(|error| error.to_string())?;
                writeln!(
                    code,
                    "    ({name:?}, include_bytes!({:?})),",
                    absolute.display().to_string()
                )
                .map_err(|error| error.to_string())?;
            }
        }
        None if env::var("ZAXNES_EMBED_PACKS").is_err() => println!(
            "cargo:warning=no hack packs built in: no zaxnes_3dn copy found (set ZAXNES_3DN_DIR); \
             the widescreen tests need them"
        ),
        None => {}
    }
    code.push_str("];\n");
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR")?).join("embedded_packs.rs");
    fs::write(out, code).map_err(|error| error.to_string())
}

/// The `packs` folder of the `zaxnes_3dn` copy, if one is found.
fn repository_packs(manifest: &Path) -> Option<PathBuf> {
    let repository = if let Some(chosen) = env::var_os("ZAXNES_3DN_DIR") {
        PathBuf::from(chosen)
    } else {
        let workspace = manifest.parent()?.parent()?;
        [
            workspace.join("..").join("zaxnes_3dn"),
            workspace.join("..").join("..").join("zaxnes_3dn"),
        ]
        .into_iter()
        .find(|folder| folder.is_dir())?
    };
    Some(repository.join("packs")).filter(|packs| packs.is_dir())
}

fn packs_in(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let entries =
        fs::read_dir(folder).map_err(|error| format!("'{}': {error}", folder.display()))?;
    let mut packs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("zxpk"))
        })
        .collect();
    packs.sort();
    Ok(packs)
}
