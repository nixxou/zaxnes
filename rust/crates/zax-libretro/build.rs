//! Puts the 3D profiles made for Zaxnes (the `zaxnes_3dn` repository, kept out of these sources
//! under a licence of their own) into the core, as `zax-app/build.rs` does for the application:
//! `ZAXNES_3DN_DIR`, else a `zaxnes_3dn` folder beside the workspace or one level above it. A
//! build without one carries none.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = bundled_profiles() {
        println!("cargo::error=the bundled 3D profiles: {error}");
    }
}

fn environment(name: &str) -> std::io::Result<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other(format!("{name} is not set")))
}

fn profiles_source(manifest: &Path) -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed=ZAXNES_3DN_DIR");
    if let Some(chosen) = std::env::var_os("ZAXNES_3DN_DIR") {
        return Some(PathBuf::from(chosen));
    }
    let workspace = manifest.parent()?.parent()?;
    [
        workspace.join("..").join("zaxnes_3dn"),
        workspace.join("..").join("..").join("zaxnes_3dn"),
    ]
    .into_iter()
    .find(|folder| folder.is_dir())
}

fn bundled_profiles() -> std::io::Result<()> {
    let manifest = environment("CARGO_MANIFEST_DIR")?;
    let mut files: Vec<PathBuf> = Vec::new();
    if let Some(source) = profiles_source(&manifest) {
        println!("cargo:rerun-if-changed={}", source.display());
        for entry in std::fs::read_dir(&source)? {
            let path = entry?.path();
            if path.extension().is_some_and(|extension| extension == "3dn") {
                println!("cargo:rerun-if-changed={}", path.display());
                files.push(path);
            }
        }
    }
    files.sort();
    let mut code = String::from("&[\n");
    for path in &files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let absolute = std::path::absolute(path)?;
        let _ = writeln!(
            code,
            "    ({name:?}, include_bytes!({:?}) as &[u8]),",
            absolute.display().to_string()
        );
    }
    code.push_str("]\n");
    std::fs::write(environment("OUT_DIR")?.join("bundled-profiles.rs"), code)
}
