//! Builds the hack packs of `patches/` into the binary (see `src/pack_files.rs`): every `.zxpk`
//! there, by file name. A file of the same name in the `zxpk` folder next to the executable
//! replaces the built-in one at run time.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR")?);
    let folder = manifest.join("patches");
    println!("cargo:rerun-if-changed={}", folder.display());
    let mut code = String::from("/// File name and bytes of each pack built in.\n");
    code.push_str("pub static PACKS: &[(&str, &[u8])] = &[\n");
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
    code.push_str("];\n");
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR")?).join("embedded_packs.rs");
    fs::write(out, code).map_err(|error| error.to_string())
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
