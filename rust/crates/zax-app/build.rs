//! Records the `target-cpu` of the build flags, for the frame profiler's start-up line, and
//! with the feature `shaders` zips the slang shader pack into the executable; puts the 3D profiles
//! made for Zaxnes (the `zaxnes_3dn` repository, kept out of these sources) into it too.

use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let mut parts = flags.split('\u{1f}');
    let mut cpu = None;
    while let Some(part) = parts.next() {
        let codegen = if part == "-C" {
            parts.next()
        } else {
            part.strip_prefix("-C")
        };
        if let Some(value) = codegen.and_then(|option| option.strip_prefix("target-cpu=")) {
            cpu = Some(value.to_owned());
        }
    }
    if let Some(cpu) = cpu {
        println!("cargo:rustc-env=ZAXNES_TARGET_CPU={cpu}");
    }
    if let Err(error) = bundled_profiles() {
        println!("cargo::error=the bundled 3D profiles: {error}");
    }
    if std::env::var_os("CARGO_FEATURE_SHADERS").is_some() {
        if let Err(error) = shader_pack() {
            println!("cargo::error=the slang shader pack: {error}");
        }
    }
}

/// The copy of libretro/slang-shaders the pack is read from: `ZAXNES_SLANG_SHADERS`, else a
/// `slang-shaders` folder beside the workspace or one level above it.
fn shader_source(manifest: &Path) -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed=ZAXNES_SLANG_SHADERS");
    if let Some(chosen) = std::env::var_os("ZAXNES_SLANG_SHADERS") {
        return Some(PathBuf::from(chosen));
    }
    let workspace = manifest.parent()?.parent()?;
    [
        workspace.join("..").join("slang-shaders"),
        workspace.join("..").join("..").join("slang-shaders"),
    ]
    .into_iter()
    .find(|folder| folder.is_dir())
}

/// The copy of the `zaxnes_3dn` repository (the 3D profiles made for Zaxnes, under a licence of
/// their own): `ZAXNES_3DN_DIR`, else a `zaxnes_3dn` folder beside the workspace or one or two
/// levels above it. None: the program carries no such profile.
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

/// Writes `bundled-profiles.rs`: the `.3dn` files of [`profiles_source`], each with its file name,
/// included in the executable (`bundled_profiles.rs` installs them).
fn bundled_profiles() -> std::io::Result<()> {
    use std::fmt::Write as _;
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

fn environment(name: &str) -> std::io::Result<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other(format!("{name} is not set")))
}

/// Zips the files of `shaders/pack-files.txt` (`tools/shader_pack.py`) into `slang-pack.zip`,
/// and records its version, a hash of the names and contents. Without the collection the pack
/// is empty and the shaders window says so.
fn shader_pack() -> std::io::Result<()> {
    let manifest = environment("CARGO_MANIFEST_DIR")?;
    // Every build carries the full pack, the Android package too (owner decision: some devices
    // run the heavy presets, each player picks for his own); `ZAXNES_SHADER_PACK=light` builds
    // with the light one (`pack-files-light.txt`).
    println!("cargo:rerun-if-env-changed=ZAXNES_SHADER_PACK");
    let light = std::env::var("ZAXNES_SHADER_PACK").as_deref() == Ok("light");
    let list = manifest.join("shaders").join(if light {
        "pack-files-light.txt"
    } else {
        "pack-files.txt"
    });
    println!("cargo:rerun-if-changed={}", list.display());
    let out = environment("OUT_DIR")?.join("slang-pack.zip");
    let names: Vec<String> = std::fs::read_to_string(&list)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&out)?);
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut fold = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    let mut written = 0usize;
    if let Some(source) = shader_source(&manifest) {
        for name in &names {
            let path = source.join(name);
            println!("cargo:rerun-if-changed={}", path.display());
            let Ok(bytes) = std::fs::read(&path) else {
                println!("cargo:warning=shader pack: {name} is missing from the collection");
                continue;
            };
            // The pictures are compressed already.
            let picture = Path::new(name).extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("png") || extension.eq_ignore_ascii_case("jpg")
            });
            let method = if picture {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            };
            let options = zip::write::SimpleFileOptions::default().compression_method(method);
            zip.start_file(name.as_str(), options)
                .map_err(std::io::Error::other)?;
            zip.write_all(&bytes)?;
            fold(name.as_bytes());
            fold(&bytes);
            written += 1;
        }
    } else {
        println!(
            "cargo:warning=no slang-shaders collection (ZAXNES_SLANG_SHADERS): the shader pack is empty"
        );
    }
    zip.finish().map_err(std::io::Error::other)?;
    // The names the pack holds, for the catalogue to show only the presets it can draw.
    let packed = if written == 0 {
        String::new()
    } else {
        names.join("\n")
    };
    std::fs::write(environment("OUT_DIR")?.join("slang-pack-files.txt"), packed)?;
    let version = if written == 0 {
        "none".to_owned()
    } else {
        format!("{hash:016x}")
    };
    println!("cargo:rustc-env=ZAXNES_SHADER_PACK_VERSION={version}");
    Ok(())
}
