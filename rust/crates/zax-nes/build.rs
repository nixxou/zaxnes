//! Builds the hack packs of `patches/` into the binary (see `src/pack_files.rs`): every `.zxpk`
//! there, by file name. A file of the same name in the `zxpk` folder next to the executable
//! replaces the built-in one at run time.
//!
//! Also turns `data/cartdb.csv` into a sorted Rust table (`src/rom/cartdb.rs`), so that nothing
//! is parsed at run time and a malformed, unsorted or repeated line fails the build.

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
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR")?);
    fs::write(out_dir.join("embedded_packs.rs"), code).map_err(|error| error.to_string())?;
    cartdb(
        &manifest.join("data").join("cartdb.csv"),
        &out_dir.join("cartdb_table.rs"),
    )
}

/// Writes `cartdb_table.rs`: one `(crc, CartEntry)` per line of `source`, in the CSV's order,
/// which must be strictly increasing by CRC (`tools/cartdb_from_mesen.py` sorts it).
///
/// `crc,system,mapper,submapper,prg_kib,chr_kib,chr_ram_kib,work_ram_kib,save_ram_kib,battery,
/// mirroring,chip`; a save RAM written `bNNN` is a serial EEPROM of NNN bytes.
fn cartdb(source: &Path, out: &Path) -> Result<(), String> {
    println!("cargo:rerun-if-changed={}", source.display());
    let text =
        fs::read_to_string(source).map_err(|error| format!("'{}': {error}", source.display()))?;
    let mut code = String::from("&[\n");
    let mut last: Option<u32> = None;
    for (number, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |what: &str| format!("cartdb.csv line {}: {what}: {line}", number + 1);
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != 12 {
            return Err(at("12 fields expected"));
        }
        let crc = u32::from_str_radix(f[0], 16).map_err(|_| at("bad CRC"))?;
        if last.is_some_and(|last| crc <= last) {
            return Err(at("not sorted, or repeated"));
        }
        last = Some(crc);
        let mapper: i32 = f[2].parse().map_err(|_| at("bad mapper"))?;
        let number_of = |value: &str| value.parse::<usize>().unwrap_or(0);
        let submapper = f[3]
            .parse::<i32>()
            .map_or_else(|_| "None".to_owned(), |value| format!("Some({value})"));
        let eeprom = f[8]
            .strip_prefix('b')
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let mirroring = f[10].chars().next().unwrap_or('h');
        writeln!(
            code,
            concat!(
                "    (0x{hi:04X}_{lo:04X}, CartEntry {{ system: {system:?}, mapper: {mapper}, ",
                "submapper: {submapper}, prg_kib: {prg}, chr_kib: {chr}, chr_ram_kib: {chr_ram}, ",
                "work_ram_kib: {work}, save_ram_kib: {save}, save_eeprom_bytes: {eeprom}, ",
                "battery: {battery}, mirroring: {mirroring:?}, chip: {chip:?} }}),"
            ),
            hi = crc >> 16,
            lo = crc & 0xFFFF,
            system = f[1],
            mapper = mapper,
            submapper = submapper,
            eeprom = eeprom,
            mirroring = mirroring,
            prg = number_of(f[4]),
            chr = number_of(f[5]),
            chr_ram = number_of(f[6]),
            work = number_of(f[7]),
            save = number_of(f[8]),
            battery = f[9] == "1",
            chip = f[11],
        )
        .map_err(|error| error.to_string())?;
    }
    code.push_str("]\n");
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
