//! UNIF images (lot MAPPERS-P, mapper 532): read and rewritten as the NES 2.0 image this core
//! loads, before the header is parsed.
//!
//! The chunk rules and the board names are puNES 0.111's (`src/core/unif.c`, read, not copied):
//! `MAPR` names the board (leading spaces skipped, one `NES-`/`UNL-`/`HVC-`/`BTL-`/`BMC-` prefix
//! stripped, compared without case), `PRG0`-`PRGF` and `CHR0`-`CHRF` are concatenated in their
//! number order, `MIRR`, `BATR` and `TVCI` set the mirroring, the battery and the region, and
//! `FONT` (*China Ersan 2*'s glyphs) becomes the image's NES 2.0 miscellaneous ROM. Other chunks
//! (`NAME`, `WRTR`, `READ`, `DINF`, `CTRL`, `VROR`, `PCKn`, `CCKn`) carry nothing the core uses.
//! puNES matches a board name as a prefix of its table's names; this one matches it whole
//! (FIDELITY-LEADS J37).
use super::{RomLoadError, RomLoadFailure};

/// puNES's `find_board`: the board name, its mapper and submapper, whether the board forces
/// four-screen tables, and whether it forces a battery.
const BOARDS: &[(&str, i32, u8, bool, bool)] = &[
    ("NROM", 0, 0, false, false),
    ("NROM-128", 0, 0, false, false),
    ("NROM-256", 0, 0, false, false),
    ("Transformer", 0, 0, false, false),
    ("SLROM", 1, 0, false, false),
    ("UOROM", 2, 0, false, false),
    ("UNROM", 2, 0, false, false),
    ("CNROM", 3, 0, false, false),
    ("TLROM", 4, 0, false, false),
    ("TBROM", 4, 0, false, false),
    ("TFROM", 4, 0, false, false),
    ("TKROM", 4, 0, false, false),
    ("ANROM", 7, 1, false, false),
    ("AMROM", 7, 2, false, false),
    ("AOROM", 7, 0, false, false),
    ("SL1632", 14, 0, false, false),
    ("CC-21", 27, 0, false, false),
    ("AC08", 42, 0, false, false),
    ("LH09", 42, 0, false, false),
    ("SuperHIK8in1", 45, 0, false, false),
    ("STREETFIGTER-GAME4IN1", 49, 0, false, false),
    ("A60AS", 51, 1, false, false),
    ("Supervision16in1", 53, 0, false, false),
    ("MARIO1-MALEE2", 55, 0, false, false),
    ("T3H53", 59, 0, false, false),
    ("D1038", 59, 0, false, false),
    ("NTBROM", 68, 1, false, false),
    ("SA-016-1M", 79, 0, false, false),
    ("VRC7", 85, 0, false, false),
    ("TEK90", 90, 0, false, false),
    ("BB", 108, 0, false, false),
    ("H2288", 123, 0, false, false),
    ("LH32", 125, 0, false, false),
    ("22211", 132, 0, false, false),
    ("SA-72008", 133, 0, false, false),
    ("T4A54A", 134, 0, false, false),
    ("Sachen-8259D", 137, 0, false, false),
    ("Sachen-8259B", 138, 0, false, false),
    ("Sachen-8259C", 139, 0, false, false),
    ("Sachen-8259A", 141, 0, false, false),
    ("KS7032", 142, 0, false, false),
    ("SA-NROM", 143, 0, false, false),
    ("SA-72007", 145, 0, false, false),
    ("TC-U01-1.5M", 147, 0, false, false),
    ("SA-0037", 148, 0, false, false),
    ("SA-0036", 149, 0, false, false),
    ("Sachen-74LS374N", 150, 0, false, false),
    ("FS304", 162, 0, false, false),
    ("Super24in1SC03", 176, 0, false, false),
    ("WAIXING-FS005", 176, 0, false, false),
    ("FK23C", 176, 0, false, false),
    ("FK23CA", 176, 0, false, false),
    ("FC-28-5027", 197, 1, false, false),
    ("NovelDiamond9999999in1", 201, 0, false, false),
    ("JC-016-2", 205, 0, false, false),
    ("8237", 215, 0, false, false),
    ("8237A", 215, 1, false, false),
    ("N625092", 221, 0, false, false),
    ("Ghostbusters63in1", 226, 0, false, false),
    ("WAIXING-FW01", 227, 0, false, false),
    ("42in1ResetSwitch", 233, 0, false, false),
    ("150in1A", 235, 0, false, false),
    ("212-HONG-KONG", 235, 0, false, false),
    ("70in1", 236, 0, false, false),
    ("70in1B", 236, 0, false, false),
    ("603-5052", 238, 0, false, false),
    ("43272", 242, 0, false, false),
    ("OneBus", 256, 0, false, false),
    ("DANCE", 256, 0, false, false),
    ("158B", 258, 0, false, false),
    ("F-15", 259, 0, false, false),
    ("HPXX", 260, 0, false, false),
    ("HP2018-A", 260, 0, false, false),
    ("810544-C-A1", 261, 0, false, false),
    ("SHERO", 262, 0, true, false),
    ("KOF97", 263, 0, false, false),
    ("YOKO", 264, 0, false, false),
    ("T-262", 265, 0, false, false),
    ("CITYFIGHT", 266, 0, false, false),
    ("COOLBOY", 268, 0, false, false),
    ("MINDKIDS", 224, 1, false, false),
    ("22026", 271, 0, false, false),
    ("80013-B", 274, 0, false, false),
    ("GS-2004", 283, 0, false, false),
    ("GS-2013", 283, 0, false, false),
    ("DRIPGAME", 284, 0, false, false),
    ("A65AS", 285, 0, false, false),
    ("BS-5", 286, 0, false, false),
    ("411120-C", 287, 0, false, false),
    ("K-3088", 287, 0, false, false),
    ("GKCXIN1", 288, 0, false, false),
    ("60311C", 289, 0, false, false),
    ("NTD-03", 290, 0, false, false),
    ("DRAGONFIGHTER", 292, 0, false, false),
    ("13in1JY110", 295, 0, false, false),
    ("TF1201", 298, 0, false, false),
    ("11160", 299, 0, false, false),
    ("190in1", 300, 0, false, false),
    ("8157", 301, 0, false, false),
    ("KS7057", 302, 0, false, false),
    ("KS7017", 303, 0, false, false),
    ("SMB2J", 304, 0, false, false),
    ("KS7031", 305, 0, false, false),
    ("KS7016", 306, 0, false, false),
    ("KS7037", 307, 0, false, false),
    ("TH2131-1", 308, 0, false, false),
    ("LH51", 309, 0, false, false),
    ("KS7013B", 312, 0, false, false),
    ("RESET-TXROM", 313, 0, false, false),
    ("64in1NoRepeat", 314, 0, false, false),
    ("830134C", 315, 0, false, false),
    ("HP898F", 319, 0, false, false),
    ("830425C-4391T", 320, 0, false, false),
    ("K-3033", 322, 0, false, false),
    ("FARID_SLROM_8-IN-1", 323, 0, false, false),
    ("FARID_UNROM_8-IN-1", 324, 0, false, false),
    ("MALISB", 325, 0, false, false),
    ("10-24-C-A1", 327, 0, false, false),
    ("RT-01", 328, 0, false, false),
    ("EDU2000", 329, 0, false, false),
    ("12-IN-1", 331, 0, false, false),
    ("WS", 332, 0, false, false),
    ("8-IN-1", 333, 0, false, false),
    ("NEWSTAR-GRM070-8IN1", 333, 0, false, false),
    ("CTC-09", 335, 0, false, false),
    ("K-3046", 336, 0, false, false),
    ("CTC-12IN1", 337, 0, false, false),
    ("SA005-A", 338, 0, false, false),
    ("K-3006", 339, 0, false, false),
    ("K-3036", 340, 0, false, false),
    ("TJ-03", 341, 0, false, false),
    ("COOLGIRL", 342, 0, false, false),
    ("RESETNROM-XIN1", 343, 0, false, false),
    ("GN-26", 344, 0, false, false),
    ("L6IN1", 345, 0, false, false),
    ("KS7012", 346, 0, false, false),
    ("KS7030", 347, 0, false, false),
    ("830118C", 348, 0, false, false),
    ("G-146", 349, 0, false, false),
    ("891227", 350, 0, false, false),
    ("KS106C", 352, 0, false, false),
    ("3D-BLOCK", 355, 0, false, false),
    ("SB-5013", 359, 0, false, false),
    ("N49C-300", 369, 0, false, false),
    ("820561C", 377, 0, false, false),
    ("M2C52A", 380, 0, false, false),
    ("830752C", 396, 0, false, false),
    ("BS-400R", 422, 0, false, false),
    ("BS-4040R", 422, 0, false, false),
    ("S-2009", 434, 0, false, false),
    ("K-3010", 438, 0, false, false),
    ("K-3071", 438, 0, false, false),
    ("SA-9602B", 513, 0, false, false),
    ("DANCE2000", 518, 0, false, false),
    ("EH8813A", 519, 0, false, false),
    ("DREAMTECH01", 521, 0, false, false),
    ("LH10", 522, 0, false, false),
    ("900218", 524, 0, false, false),
    ("KS7021A", 525, 0, false, false),
    ("BJ-56", 526, 0, false, false),
    ("AX-40G", 527, 0, false, false),
    ("831128C", 528, 0, false, false),
    ("T-230", 529, 0, false, false),
    ("AX5705", 530, 0, false, false),
    ("82112C", 540, 0, false, false),
    ("CHINA_ER_SAN2", 532, 0, false, true),
    ("KONAMI-QTAI", 547, 0, false, false),
];

/// Whether `image` is a UNIF image (`"UNIF"` and a version, then chunks from byte 32).
#[must_use]
pub(crate) fn is_unif(image: &[u8]) -> bool {
    image.starts_with(b"UNIF")
}

fn corrupt(message: String) -> RomLoadError {
    RomLoadError::new(RomLoadFailure::CorruptHeader, message)
}

/// `unif_MAPR`'s stripping, then the board's row.
fn board(name: &[u8]) -> Option<(i32, u8, bool, bool)> {
    let text: String = name
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| char::from(*byte))
        .collect();
    let mut text = text.trim_start_matches(' ');
    for prefix in ["NES-", "UNL-", "HVC-", "BTL-", "BMC-"] {
        if text.len() >= prefix.len()
            && text.is_char_boundary(prefix.len())
            && text[..prefix.len()].eq_ignore_ascii_case(prefix)
        {
            text = &text[prefix.len()..];
            break;
        }
    }
    let text = text.trim_end_matches(' ');
    BOARDS
        .iter()
        .find(|row| row.0.eq_ignore_ascii_case(text))
        .map(|row| (row.1, row.2, row.3, row.4))
}

/// A ROM padded to a whole number of `unit`-byte banks by repeating it, as a mirrored chip
/// reads.
fn padded(mut data: Vec<u8>, unit: usize) -> Vec<u8> {
    if data.is_empty() || data.len() % unit == 0 {
        return data;
    }
    let target = data.len().div_ceil(unit) * unit;
    let mut index = 0;
    while data.len() < target {
        data.push(data[index]);
        index += 1;
    }
    data
}

/// The low byte of `value`.
fn low(value: usize) -> u8 {
    value.to_le_bytes()[0]
}

/// Rewrites a UNIF image as a NES 2.0 image.
///
/// # Errors
///
/// A truncated chunk, no board name or PRG, or a board puNES's table does not name.
#[allow(clippy::too_many_lines)] // One chunk walk and one header, as puNES's `unif.c` reads them.
pub(crate) fn to_nes20(image: &[u8]) -> Result<Vec<u8>, RomLoadError> {
    let mut prg: [Vec<u8>; 16] = Default::default();
    let mut chr: [Vec<u8>; 16] = Default::default();
    let mut name = None;
    let mut mirroring = None;
    let mut battery = false;
    let mut region = 0u8;
    let mut font = Vec::new();
    let mut at = 32;
    while at + 8 <= image.len() {
        let id = &image[at..at + 4];
        let length = usize::try_from(u32::from_le_bytes([
            image[at + 4],
            image[at + 5],
            image[at + 6],
            image[at + 7],
        ]))
        .unwrap_or(usize::MAX);
        let start = at + 8;
        let Some(data) = start
            .checked_add(length)
            .and_then(|end| image.get(start..end))
        else {
            return Err(corrupt(format!(
                "Corrupt UNIF image: the {} chunk runs past the end of the file.",
                String::from_utf8_lossy(id)
            )));
        };
        let slot = |digit: u8| {
            char::from(digit)
                .to_digit(16)
                .and_then(|value| usize::try_from(value).ok())
        };
        match id {
            b"MAPR" => name = Some(data.to_vec()),
            [b'P', b'R', b'G', digit] => {
                if let Some(slot) = slot(*digit) {
                    prg[slot] = data.to_vec();
                }
            }
            [b'C', b'H', b'R', digit] => {
                if let Some(slot) = slot(*digit) {
                    chr[slot] = data.to_vec();
                }
            }
            b"MIRR" => mirroring = data.first().copied(),
            b"BATR" => battery = true,
            b"TVCI" => region = data.first().copied().unwrap_or(0),
            b"FONT" => font = data[..data.len().min(0x1_0000)].to_vec(),
            _ => {}
        }
        at = start + length;
    }
    let name = name.ok_or_else(|| corrupt("Corrupt UNIF image: no MAPR chunk.".to_owned()))?;
    let Some((mut mapper, submapper, four_screen, forced_battery)) = board(&name) else {
        let shown: String = String::from_utf8_lossy(&name)
            .chars()
            .filter(|character| *character != '\0')
            .collect();
        return Err(RomLoadError::new(
            RomLoadFailure::UnsupportedMapper,
            format!("Unsupported mapper: UNIF board {}.", shown.trim()),
        ));
    };
    let prg = padded(prg.concat(), 0x4000);
    let chr = padded(chr.concat(), 0x2000);
    if prg.is_empty() {
        return Err(corrupt("Corrupt UNIF image: no PRG chunk.".to_owned()));
    }
    // `find_board`: *SMB2J* under 80 KiB is 304, else 311.
    if mapper == 304 && prg.len() >= 81920 {
        mapper = 311;
    }
    let prg_units = prg.len() / 0x4000;
    let chr_units = chr.len() / 0x2000;
    if prg_units > 0xEFF || chr_units > 0xEFF {
        return Err(corrupt(
            "Corrupt UNIF image: a ROM too large for NES 2.0.".to_owned(),
        ));
    }
    let battery = battery || forced_battery;
    // `unif_MIRR`: 0 horizontal, 1 and 5 vertical, 4 four-screen; 2 and 3 (single-screen) have
    // no header bit, and those boards set their own.
    let vertical = matches!(mirroring, Some(1 | 5));
    let four = four_screen || mirroring == Some(4);
    let mapper = usize::try_from(mapper).unwrap_or(0);
    let mut header = [0u8; 16];
    header[..4].copy_from_slice(b"NES\x1A");
    header[4] = low(prg_units);
    header[5] = low(chr_units);
    header[6] = (low(mapper & 0x0F) << 4)
        | u8::from(vertical)
        | (u8::from(battery) << 1)
        | (u8::from(four) << 3);
    header[7] = low(mapper & 0xF0) | 0x08;
    header[8] = (submapper << 4) | low((mapper >> 8) & 0x0F);
    header[9] = (low(chr_units >> 8) << 4) | low(prg_units >> 8);
    // 8 KiB of work RAM (shift 7), battery-backed with `BATR`; 8 KiB of CHR-RAM without CHR.
    header[10] = if battery { 0x70 } else { 0x07 };
    header[11] = if chr.is_empty() { 0x07 } else { 0 };
    // `unif_TVCI`: 0 NTSC, 1 PAL, 2 both.
    header[12] = region.min(2);
    header[14] = u8::from(!font.is_empty());
    let mut out = Vec::with_capacity(16 + prg.len() + chr.len() + font.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&prg);
    out.extend_from_slice(&chr);
    out.extend_from_slice(&font);
    Ok(out)
}
