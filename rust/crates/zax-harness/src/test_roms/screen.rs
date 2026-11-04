//! ROMs that report on screen rather than through memory.
//!
//! The reader turns a nametable into 30 rows of 32 characters, so a result the ROM only draws
//! can be quoted in the report instead of being left to a human. It reads
//! [`NesCore::ppu`]`().vram()` through [`NesCore::map_nametable`], both of which the core
//! already had, and never touches a per-cycle path.
//!
//! ## Why a font table and not an OCR
//!
//! A nametable holds tile numbers, not characters, so the reader needs the ROM's own
//! tile → character mapping. Two mappings cover the suites of this lot, and both were read off
//! the ROM's own CHR rather than guessed:
//!
//! * [`Font::Ascii6`] — Holy Mapperel. Its CHR holds `A` at tile `$01`, `M` at `$0D`, `0` at
//!   `$30`, `?` at `$3F` and the space at `$20`, which is ASCII masked to six bits
//!   (`tile = ascii & $3F`). Tiles `$40` and up are the frame and box graphics, not text.
//! * [`Font::Ascii`] — a ROM whose font sits at the tile its ASCII code names, which is what
//!   blargg's own print routines assume.
//!
//! Anything the mapping does not cover becomes a space, so a decoded row is the text of the
//! row and nothing else.

use serde_json::json;
use zax_nes::NesCore;

use super::{Outcome, Verdict};

/// Columns and rows of a nametable's visible text area.
pub const COLUMNS: usize = 32;
/// Rows of a nametable's visible text area.
pub const ROWS: usize = 30;

/// How a ROM's font maps a tile number to a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// `tile = ascii`.
    Ascii,
    /// `tile = ascii & $3F`: tiles `$01-$1F` are `A-Z` and the five punctuation marks after
    /// them, tiles `$20-$3F` are themselves. Holy Mapperel's font.
    Ascii6,
}

impl Font {
    /// The character tile `tile` stands for, or a space when the font does not cover it.
    #[must_use]
    pub const fn character(self, tile: u8) -> char {
        match self {
            Self::Ascii if tile.is_ascii_graphic() => tile as char,
            Self::Ascii6 => match tile {
                0x01..=0x1F => (tile | 0x40) as char,
                0x21..=0x3F => tile as char,
                _ => ' ',
            },
            Self::Ascii => ' ',
        }
    }
}

/// The 30 rows of 32 characters of the nametable whose first byte is at PPU address `base`
/// (`$2000`, `$2400`, `$2800` or `$2C00`), with trailing spaces trimmed.
///
/// The bytes come from [`Ppu2C02::read_nametable_byte`](zax_nes::ppu::Ppu2C02::read_nametable_byte),
/// the accessor the tile capture already uses, so a board that serves its own nametables — an
/// MMC5's `ExRAM` or its fill mode — reads correctly instead of through console VRAM that it
/// never wrote. The accessor is documented as having no side effect.
#[must_use]
pub fn rows_at(core: &NesCore, base: u16, font: Font) -> Vec<String> {
    let ppu = core.ppu();
    let cartridge = core.cartridge();
    (0..ROWS)
        .map(|row| {
            let line: String = (0..COLUMNS)
                .map(|column| {
                    let address =
                        base.wrapping_add(u16::try_from(row * COLUMNS + column).unwrap_or(0));
                    font.character(ppu.read_nametable_byte(address, cartridge))
                })
                .collect();
            line.trim_end().to_owned()
        })
        .collect()
}

/// The nametable the PPU is showing, as rows.
///
/// The base comes from bits 0 and 1 of PPUCTRL, which is where the ROM told the PPU to draw
/// from. Scanning all four and keeping the one with the most text instead reads *the other
/// physical page* when the ROM left something in it: `blargg_ppu_tests_2005.09.15b/vram_access`
/// draws a four-character `$01` on the page it shows and leaves the debris of an earlier VRAM
/// test on the other, and the debris wins on character count. The other three pages are only
/// tried when the shown one is blank.
#[must_use]
pub fn rows(core: &NesCore, font: Font) -> Vec<String> {
    let shown = 0x2000u16 + 0x400 * u16::from(core.ppu().control() & 0x03);
    let rows = rows_at(core, shown, font);
    if score(&rows) > 0 {
        return rows;
    }

    [0x2000u16, 0x2400, 0x2800, 0x2C00]
        .into_iter()
        .map(|base| rows_at(core, base, font))
        .max_by_key(|rows| score(rows))
        .unwrap_or_default()
}

/// The screen and the font that read it, choosing between the two mappings by which one yields
/// more letters and digits.
///
/// Nothing in a ROM file says where its font sits, and the two suites of this lot disagree:
/// Holy Mapperel masks ASCII to six bits while the pre-`$6000` blargg builds put each glyph at
/// the tile its ASCII code names. Reading a screen with the wrong mapping gives punctuation and
/// gaps rather than words, so the count of alphanumeric characters separates them cleanly. The
/// chosen font is recorded in the row, so a reader of the report can check the call.
#[must_use]
pub fn best(core: &NesCore) -> (Font, Vec<String>) {
    [Font::Ascii, Font::Ascii6]
        .into_iter()
        .map(|font| (font, rows(core, font)))
        .max_by_key(|(_, rows)| score(rows))
        .unwrap_or((Font::Ascii, Vec::new()))
}

/// How much readable text a decoded screen holds: its letters and digits.
fn score(rows: &[String]) -> usize {
    rows.iter()
        .flat_map(|row| row.chars())
        .filter(char::is_ascii_alphanumeric)
        .count()
}

/// The non-empty rows of a screen joined by `" / "`, which is how the JSON and the report
/// quote it.
#[must_use]
pub fn joined(rows: &[String]) -> String {
    rows.iter()
        .map(|row| row.trim())
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}

/// Runs `core` until its screen has stopped changing, or `budget` frames have been run.
///
/// A screen counts as settled once the nametable has held the same bytes for `SETTLED` frames
/// *and* carries at least `ENOUGH` characters of text. Both halves are needed: Holy Mapperel
/// leaves an almost blank screen up for seconds while the CHR and WRAM passes buzz, and a
/// reader that stops at the first stable frame reads that blank screen instead of the result.
/// Returns the frames run.
pub fn run_until_settled(core: &mut NesCore, budget: u32) -> u32 {
    /// Frames of an unchanging screen that mean the ROM has finished drawing. Two seconds:
    /// Holy Mapperel leaves a part-drawn screen up while the CHR and WRAM passes buzz.
    const SETTLED: u32 = 120;
    /// Letters and digits a settled screen must carry before it is believed to be the result.
    /// Two, because `blargg_ppu_tests_2005.09.15b` reports a whole suite as `$01`.
    const ENOUGH: usize = 2;

    let mut previous = Vec::new();
    let mut unchanged = 0;
    for frame in 1..=budget {
        core.run_frame(0);
        let (_, current) = best(core);
        if current == previous {
            unchanged += 1;
            if unchanged >= SETTLED && score(&current) >= ENOUGH {
                return frame;
            }
        } else {
            unchanged = 0;
            previous = current;
        }
    }

    budget
}

/// Whether the ROM at relative path `lower` (lower-cased) reports only on screen, so the
/// blargg reader must not be pointed at it.
///
/// Holy Mapperel says so in its own README ("All this information is displayed on the
/// screen"), and `AccuracyCoin` draws a table of 147 results.
#[must_use]
pub fn is_screen_rom(lower: &str) -> bool {
    lower.contains("holy-mapperel") || lower.contains("accuracycoin")
}

/// The frame budget of the screen ROM at relative path `lower` (lower-cased).
#[must_use]
pub fn frame_budget(lower: &str) -> u32 {
    if lower.contains("accuracycoin") {
        120_000 // the whole ROM runs 147 tests back to back
    } else if lower.contains("holy-mapperel") {
        10_800 // the CHR and WRAM passes buzz for a while before the result is drawn
    } else {
        900
    }
}

/// How a suite's documentation says its screen is to be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The ROM draws `PASSED`, or `FAILED` with its code.
    ///
    /// This is blargg's own wording, and the same text his `$6000` protocol carries: the
    /// readmes say "Tests generally print information on screen, but also report the final
    /// result audibly, and output text to memory", so the screen and `$6000` say the same
    /// thing. `sprite_hit_tests_2005.10.05` and `sprite_overflow_tests` predate `$6000` and
    /// have only the screen; `read_joy3/thorough_test` draws it without ever arming the
    /// protocol. A screen holding neither word is `manual`, so this reading never invents a
    /// verdict for a ROM that is not reporting one.
    Word,
    /// The ROM draws a hexadecimal result code, `$01` meaning every test passed.
    ///
    /// `blargg_ppu_tests_2005.09.15b`: "Each ROM runs several tests and reports a result code
    /// on screen and by beeping a number of times. A result code of 1 always indicates that
    /// all tests were passed."
    HexCode,
}

impl Kind {
    /// How the ROM at relative path `lower` (lower-cased) reports.
    #[must_use]
    pub fn of(lower: &str) -> Self {
        if lower.contains("blargg_ppu_tests") {
            Self::HexCode
        } else {
            Self::Word
        }
    }

    /// The verdict a settled screen states under this reading, and the code it names.
    #[must_use]
    pub fn verdict(self, text: &str) -> (Outcome, Option<u32>) {
        let upper = text.to_ascii_uppercase();
        match self {
            // `FAILED` wins over `PASSED`: a ROM that lists sub-tests can leave an earlier
            // `PASSED` on screen above the failure that stopped it.
            Self::Word if upper.contains("FAILED") => (Outcome::Fail, code_after_hash(&upper)),
            Self::Word if upper.contains("PASSED") => (Outcome::Pass, Some(0)),
            Self::HexCode => match hex_code(&upper) {
                Some(1) => (Outcome::Pass, Some(1)),
                Some(code) => (Outcome::Fail, Some(code)),
                None => (Outcome::Manual, None),
            },
            Self::Word => (Outcome::Manual, None),
        }
    }
}

/// Runs `core` for `budget` frames and reads the verdict off its screen the way `kind` says.
pub fn run(nes: &mut NesCore, budget: u32, kind: Kind) -> Verdict {
    let frames = run_until_settled(nes, budget);
    let (font, rows) = best(nes);
    let text = joined(&rows);
    let (outcome, code) = kind.verdict(&text);
    let message = match (outcome, text.is_empty()) {
        (_, true) => "nothing readable on screen; read the PNG of the final frame".to_owned(),
        (Outcome::Manual, false) => format!("screen (no documented verdict): {text}"),
        (_, false) => format!("screen: {text}"),
    };

    let mut verdict = Verdict::new("screen", outcome, message, frames);
    verdict.code = code;
    verdict.detail.insert("screen".into(), json!(text));
    verdict
        .detail
        .insert("font".into(), json!(format!("{font:?}")));
    verdict
}

/// The number after the first `#`, which is how the screen names a failure code (`FAILED #3`).
fn code_after_hash(upper: &str) -> Option<u32> {
    let digits: String = upper
        .split_once('#')?
        .1
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// The hexadecimal number after the first `$` of a screen (`$01`).
fn hex_code(upper: &str) -> Option<u32> {
    let digits: String = upper
        .split_once('$')?
        .1
        .chars()
        .take_while(char::is_ascii_hexdigit)
        .collect();
    u32::from_str_radix(&digits, 16).ok()
}
