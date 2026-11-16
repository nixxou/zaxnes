//! **What the cartridge database would change**, ROM by ROM, without loading anything into a
//! core (lot CARTDB step 1): for each `.nes` of a directory, its header, whether the database
//! knows its data CRC, and the corrections `zax_nes::rom::cartdb::corrected_header` would make.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use zax_nes::builtin_patches::data_crc;
use zax_nes::rom::cartdb::{corrected_header, lookup};
use zax_nes::rom::InesHeader;

use crate::HarnessError;

pub const USAGE: &str = "zax-harness cartdb-survey <roms dir> [--all]
    For every .nes under the directory (recursively): the header, whether the cartridge database
    knows the dump, and what it would correct. Without --all only the ROMs it would change are
    listed, then a summary.";

/// The command.
///
/// # Errors
/// A bad argument or an unreadable directory.
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let usage = || HarnessError::Usage(USAGE.to_owned());
    let dir = PathBuf::from(args.first().ok_or_else(usage)?);
    let all = args.iter().any(|arg| arg == "--all");
    let mut files = Vec::new();
    collect(&dir, &mut files)?;
    files.sort();
    let (mut known, mut nes20, mut changed, mut unknown, mut mismatch) = (0, 0, 0, 0, 0);
    let mut text = String::new();
    for path in &files {
        let Ok(image) = fs::read(path) else { continue };
        let Ok(header) = InesHeader::try_parse(&image) else {
            continue;
        };
        let crc = data_crc(&image);
        let name = path
            .strip_prefix(&dir)
            .unwrap_or(path)
            .display()
            .to_string();
        let entry = lookup(crc);
        if entry.is_some() {
            known += 1;
        } else {
            unknown += 1;
        }
        if header.is_nes20 {
            nes20 += 1;
        }
        if let Some(correction) = corrected_header(&header, crc) {
            changed += 1;
            let _ = writeln!(
                text,
                "FIX  {name} [{crc:08X}] {}",
                correction.changes.join("; ")
            );
        } else {
            {
                let sizes_differ = entry.as_ref().is_some_and(|entry| {
                    entry.prg_kib * 1024 != header.prg_rom_size
                        || entry.chr_kib * 1024 != header.chr_rom_size
                        || entry.mapper != header.mapper
                });
                if sizes_differ && !header.is_nes20 {
                    mismatch += 1;
                    let _ = writeln!(
                        text,
                        "KEPT {name} [{crc:08X}] the database names another mapper or other sizes"
                    );
                } else if all {
                    let state = if header.is_nes20 {
                        "NES 2.0, kept"
                    } else if entry.is_some() {
                        "matches"
                    } else {
                        "unknown"
                    };
                    let _ = writeln!(text, "ok   {name} [{crc:08X}] {state}");
                }
            }
        }
    }
    let _ = writeln!(
        text,
        "\n{} ROMs: {known} known to the database, {unknown} unknown, {nes20} NES 2.0 headers (kept); {changed} would be corrected, {mismatch} known with another mapper or other sizes (kept)",
        files.len()
    );
    print!("{text}");
    Ok(())
}

fn collect(dir: &PathBuf, out: &mut Vec<PathBuf>) -> Result<(), HarnessError> {
    for entry in fs::read_dir(dir).map_err(HarnessError::Io)? {
        let path = entry.map_err(HarnessError::Io)?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nes"))
        {
            out.push(path);
        }
    }
    Ok(())
}
