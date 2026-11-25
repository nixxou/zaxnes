//! Records the `target-cpu` of the build flags, for the frame profiler's start-up line, and
//! with the feature `shaders` zips the slang shader pack into the executable; puts the 3D profiles
//! made for Zaxnes (`rust/profiles`) into it too.

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
    if let Err(error) = gamedb() {
        println!("cargo::error=the game database: {error}");
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

/// The 3D profiles made for Zaxnes: the workspace's `profiles` folder.
fn profiles_source(manifest: &Path) -> Option<PathBuf> {
    let folder = manifest.parent()?.parent()?.join("profiles");
    folder.is_dir().then_some(folder)
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

/// The fields of one CSV line (`tools/gamedb_build.py` writes them with Python's `csv`: a field
/// with a comma or a quote is quoted, a quote inside doubled).
fn csv_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => fields.push(std::mem::take(&mut field)),
            _ => field.push(c),
        }
    }
    fields.push(field);
    fields
}

/// What a name is compared by: its letters and digits, in lowercase (as
/// `ui::handheld::rom_list::normalise` and `gamedb::normalise`).
fn normalise(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The data lines of the CSV `path`, each split in fields.
fn csv_rows(path: &Path) -> std::io::Result<Vec<Vec<String>>> {
    println!("cargo:rerun-if-changed={}", path.display());
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(csv_fields)
        .collect())
}

/// Writes `gamedb-tables.rs` from `data/gamedb_roms.csv` and `data/gamedb_games.csv`
/// (`tools/gamedb_build.py`): the ROMs sorted by data CRC, the games sorted by id, and the indexes
/// by game, by No-Intro name and by whole-file CRC (see `src/gamedb.rs`). A malformed, unsorted or
/// repeated line, or a ROM of an unknown game, fails the build.
fn gamedb() -> std::io::Result<()> {
    use std::fmt::Write as _;
    let bad = |what: String| std::io::Error::other(what);
    let data = environment("CARGO_MANIFEST_DIR")?.join("data");
    let roms = csv_rows(&data.join("gamedb_roms.csv"))?;
    let games = csv_rows(&data.join("gamedb_games.csv"))?;

    let mut code = String::from("pub static GAMES: &[Game] = &[\n");
    let mut last_game: Option<&str> = None;
    for row in &games {
        let [id, profile, threedsen] = row.as_slice() else {
            return Err(bad(format!("gamedb_games.csv: 3 fields expected: {row:?}")));
        };
        if last_game.is_some_and(|last| id.as_str() <= last) {
            return Err(bad(format!(
                "gamedb_games.csv: not sorted, or repeated: {id}"
            )));
        }
        last_game = Some(id);
        let _ = writeln!(
            code,
            "    Game {{ id: {id:?}, profile: {profile:?}, threedsen_profiles: &[{}] }},",
            names_list(threedsen)
        );
    }
    code.push_str("];\n\npub static ROMS: &[Rom] = &[\n");
    let mut last_crc: Option<u32> = None;
    let mut by_game = Vec::new();
    let mut by_no_intro = Vec::new();
    let mut by_file = Vec::new();
    for (index, row) in roms.iter().enumerate() {
        let [crc, game, name, region, md5, files, threedsen] = row.as_slice() else {
            return Err(bad(format!("gamedb_roms.csv: 7 fields expected: {row:?}")));
        };
        let crc = u32::from_str_radix(crc, 16).map_err(|_| bad(format!("bad CRC {crc}")))?;
        if last_crc.is_some_and(|last| crc <= last) {
            return Err(bad(format!(
                "gamedb_roms.csv: not sorted, or repeated: {crc:08X}"
            )));
        }
        last_crc = Some(crc);
        if games
            .binary_search_by(|row| row[0].as_str().cmp(game))
            .is_err()
        {
            return Err(bad(format!(
                "gamedb_roms.csv: {crc:08X} of an unknown game {game}"
            )));
        }
        if !(md5.is_empty() || md5.len() == 32 && md5.bytes().all(|b| b.is_ascii_hexdigit())) {
            return Err(bad(format!("gamedb_roms.csv: {crc:08X}: bad MD5 {md5}")));
        }
        let mut file_crcs = Vec::new();
        for file in files.split_whitespace() {
            let file = u32::from_str_radix(file, 16).map_err(|_| bad(format!("bad CRC {file}")))?;
            file_crcs.push(format!("0x{:04X}_{:04X}", file >> 16, file & 0xFFFF));
            by_file.push((file, index));
        }
        by_game.push((normalise(game), index));
        if !name.is_empty() {
            by_no_intro.push((normalise(name), index));
        }
        let _ = writeln!(
            code,
            "    Rom {{ data_crc: 0x{:04X}_{:04X}, game: {game:?}, no_intro_name: {name:?}, \
             region: {region:?}, data_md5: {md5:?}, file_crc32s: &[{}], threedsen_profiles: \
             &[{}] }},",
            crc >> 16,
            crc & 0xFFFF,
            file_crcs.join(", "),
            names_list(threedsen)
        );
    }
    code.push_str("];\n");
    by_game.sort();
    by_no_intro.sort();
    by_file.sort_unstable();
    for (table, index) in [("BY_GAME", &by_game), ("BY_NO_INTRO_NAME", &by_no_intro)] {
        let _ = writeln!(code, "\npub static {table}: &[(&str, u32)] = &[");
        for (key, rom) in index {
            let _ = writeln!(code, "    ({key:?}, {rom}),");
        }
        code.push_str("];\n");
    }
    ra_games(&data.join("ra_games.csv"), &mut code)?;
    code.push_str("\npub static BY_FILE_CRC32: &[(u32, u32)] = &[\n");
    for (crc, rom) in &by_file {
        let _ = writeln!(
            code,
            "    (0x{:04X}_{:04X}, {rom}),",
            crc >> 16,
            crc & 0xFFFF
        );
    }
    code.push_str("];\n");
    std::fs::write(environment("OUT_DIR")?.join("gamedb-tables.rs"), code)
}

/// The `|`-separated names of `field` as the items of a Rust `&[&str]`.
fn names_list(field: &str) -> String {
    field
        .split('|')
        .filter(|name| !name.is_empty())
        .map(|name| format!("{name:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Appends `RA_GAMES` to `code`: RetroAchievements' games by MD5 from `source`
/// (`tools/ra_game_list.py`), which must be sorted by it.
fn ra_games(source: &Path, code: &mut String) -> std::io::Result<()> {
    use std::fmt::Write as _;
    let bad = |what: String| std::io::Error::other(what);
    code.push_str("\npub static RA_GAMES: &[RaGame] = &[\n");
    let mut last_md5: Option<String> = None;
    for row in csv_rows(source)? {
        let [md5, id, console, achievements, points, title] = row.as_slice() else {
            return Err(bad(format!("ra_games.csv: 6 fields expected: {row:?}")));
        };
        if md5.len() != 32 || !md5.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad(format!("ra_games.csv: bad MD5 {md5}")));
        }
        if last_md5.as_deref().is_some_and(|last| md5.as_str() <= last) {
            return Err(bad(format!("ra_games.csv: not sorted, or repeated: {md5}")));
        }
        last_md5 = Some(md5.clone());
        let number = |text: &str| {
            text.parse::<u32>()
                .map_err(|_| bad(format!("ra_games.csv: bad number {text}")))
        };
        let _ = writeln!(
            code,
            "    RaGame {{ md5: {md5:?}, game_id: {}, console: {console:?}, achievements: {}, \
             points: {}, title: {title:?} }},",
            number(id)?,
            number(achievements)?,
            number(points)?
        );
    }
    code.push_str("];\n");
    Ok(())
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
