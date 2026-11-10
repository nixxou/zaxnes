//! Holy Mapperel, Damian Yerrick's cartridge board test (`holy-mapperel/README.md`).
//!
//! The ROM detects the board it is running on through mirroring and bank switching, sizes the
//! PRG ROM, the work RAM and the CHR, checks every bank tag, writes and reads back the whole of
//! any CHR RAM and work RAM, and runs a detailed per-mapper pass. It has **no memory
//! protocol**: its README lists the variables that are valid at the end and then says
//!
//! > All this information is displayed on the screen.
//!
//! so the screen is what the harness reads, through [`super::screen`] and the six-bit ASCII
//! font of the ROM's own CHR. A settled screen looks like
//!
//! ```text
//! 001 SNROM (MMC1)
//! 128K PRG ROM
//! 8K PRG RAM OK
//! 8K CHR RAM OK
//! DETAILED TEST RESULT: 4000
//! ```
//!
//! ## What counts as a pass
//!
//! The README is explicit about the four-digit code: "The detailed code is 4 digits: WRAM, PRG
//! ROM, IRQ, and CHR ROM/RAM. Zero is normal; anything else reflects something unexpected." So
//! `0000` with the detected mapper matching the header is a pass, and any other digit is a
//! failure with the digit named.
//!
//! Two lines must **not** be read as failures on their own:
//!
//! * `PRG RAM MISSING` is the right answer for a board that has none — BNROM and the A53 both
//!   report it, and both are correct;
//! * the ROM's own README warns that a board may implement only a subset of a mapper, so the
//!   digits describe the board, not a defect, on real hardware. In an emulator they do describe
//!   the emulation, which is why they are reported in full.
//!
//! A board that fails before it can draw anything beeps Morse code instead (`WB`, `MIR`, `SU`,
//! `LB`, `RB`, `CBT`, `FON`, `DRV`). The harness cannot hear it, so the row is a timeout with a
//! blank screen and the PNG shows it.

use serde_json::json;
use zax_nes::{NesCoreOf, TimingProfile};

use super::screen::{self, Font};
use super::{Outcome, Verdict};

/// The line that carries the four detailed digits.
const DETAILED: &str = "DETAILED TEST RESULT:";

/// Runs `core` until its result screen settles and reads the board it detected.
///
/// `header_mapper` is the iNES mapper of the file, so a board the ROM detects as something else
/// is caught: that is the whole point of the suite.
pub fn run<P: TimingProfile>(
    core: &mut NesCoreOf<P>,
    header_mapper: Option<i32>,
    budget: u32,
) -> Verdict {
    let frames = screen::run_until_settled(core, budget);
    let rows = screen::rows(core, Font::Ascii6);
    let text = screen::joined(&rows);
    let report = Report::parse(&rows);

    let (outcome, message) = report.verdict(header_mapper, &text);
    let mut verdict = Verdict::new("mapperel", outcome, message, frames);
    verdict.code = report.detailed.as_ref().and_then(|code| code.value);
    verdict.detail.insert("screen".into(), json!(text));
    verdict.detail.insert("board".into(), json!(report.board));
    verdict
        .detail
        .insert("detected_mapper".into(), json!(report.mapper));
    verdict
        .detail
        .insert("prg_rom".into(), json!(report.line_with("PRG ROM")));
    verdict
        .detail
        .insert("prg_ram".into(), json!(report.line_with("PRG RAM")));
    verdict
        .detail
        .insert("chr".into(), json!(report.chr.clone()));
    if let Some(code) = &report.detailed {
        verdict.detail.insert("detailed".into(), json!(code.digits));
        verdict.detail.insert(
            "detailed_meaning".into(),
            json!(code.meanings(report.mapper)),
        );
    }
    verdict
}

/// What one settled Holy Mapperel screen says.
#[derive(Debug, Clone, Default)]
struct Report {
    /// The screen, row by row, trimmed.
    rows: Vec<String>,
    /// The mapper number the ROM detected, from the `NNN BOARD` line.
    mapper: Option<u32>,
    /// The board name on that same line.
    board: Option<String>,
    /// The CHR line, which says ROM or RAM, its size and `OK` or not.
    chr: Option<String>,
    /// The four detailed digits.
    detailed: Option<Detailed>,
}

impl Report {
    fn parse(rows: &[String]) -> Self {
        let mut report = Self {
            rows: rows.iter().map(|row| row.trim().to_owned()).collect(),
            ..Self::default()
        };

        for row in &report.rows {
            if let Some((number, board)) = detected_board(row) {
                if report.mapper.is_none() {
                    report.mapper = Some(number);
                    report.board = Some(board);
                }
            } else if row.contains("CHR ") && report.chr.is_none() {
                report.chr = Some(row.clone());
            } else if let Some(digits) = row.strip_prefix(DETAILED) {
                report.detailed = Some(Detailed::parse(digits.trim()));
            }
        }

        report
    }

    /// The first row containing `needle`.
    fn line_with(&self, needle: &str) -> Option<String> {
        self.rows.iter().find(|row| row.contains(needle)).cloned()
    }

    /// The verdict this screen states.
    fn verdict(&self, header_mapper: Option<i32>, text: &str) -> (Outcome, String) {
        let Some(detailed) = &self.detailed else {
            return (
                Outcome::Timeout,
                if text.is_empty() {
                    "no result screen; the ROM may be beeping a Morse failure code".to_owned()
                } else {
                    format!("no \"{DETAILED}\" line on the settled screen: {text}")
                },
            );
        };

        let board = self.board.clone().unwrap_or_else(|| "?".to_owned());
        let sizes = [
            self.line_with("PRG ROM"),
            self.line_with("PRG RAM"),
            self.chr.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");

        // The suite exists to catch a board detected as the wrong one, so a mismatch with the
        // header is a failure however clean the rest of the screen is.
        if let (Some(detected), Some(header)) = (self.mapper, header_mapper) {
            if i64::from(detected) != i64::from(header) {
                return (
                    Outcome::Fail,
                    format!(
                        "detected mapper {detected} ({board}) but the header says {header}; \
                         detailed {digits}",
                        digits = detailed.digits
                    ),
                );
            }
        }

        let meanings = detailed.meanings(self.mapper);
        if detailed.value == Some(0) && !self.says_bad() {
            (Outcome::Pass, format!("{board}: {sizes}, detailed 0000"))
        } else if detailed.value == Some(0) {
            (
                Outcome::Fail,
                format!("{board}: detailed 0000 but the screen reports {sizes}"),
            )
        } else {
            let explained = if meanings.is_empty() {
                String::new()
            } else {
                format!(" ({})", meanings.join("; "))
            };
            (
                Outcome::Fail,
                format!(
                    "{board}: detailed {digits}{explained}; {sizes}",
                    digits = detailed.digits
                ),
            )
        }
    }

    /// Whether a size line reports a memory that did not hold what was written to it.
    ///
    /// `PRG RAM MISSING` is **not** one: a BNROM and an A53 have no work RAM and say so
    /// correctly.
    fn says_bad(&self) -> bool {
        self.rows.iter().any(|row| row.contains("BAD"))
    }
}

/// `001 SNROM (MMC1)` split into its number and its board name.
fn detected_board(row: &str) -> Option<(u32, String)> {
    let (number, board) = row.split_once(' ')?;
    if number.len() != 3 || !number.chars().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let board = board.trim();
    if board.is_empty() {
        return None;
    }

    Some((number.parse().ok()?, board.to_owned()))
}

/// The `WPIC` digits: work RAM, PRG ROM, IRQ, CHR.
#[derive(Debug, Clone, Default)]
struct Detailed {
    /// The four characters as drawn.
    digits: String,
    /// Their value, `None` when the screen did not hold four hex digits.
    value: Option<u32>,
}

impl Detailed {
    fn parse(digits: &str) -> Self {
        let digits = digits.trim().to_owned();
        let value = u32::from_str_radix(&digits, 16).ok();
        Self { digits, value }
    }

    /// The meanings the README gives the non-zero digits of `mapper`.
    ///
    /// Only three are documented ("Displayed result"); the others are reported as the digit and
    /// the memory it belongs to, which is all the README commits to.
    fn meanings(&self, mapper: Option<u32>) -> Vec<String> {
        const FIELDS: [&str; 4] = ["WRAM", "PRG ROM", "IRQ", "CHR"];

        let mut meanings = Vec::new();
        for (index, digit) in self.digits.chars().take(4).enumerate() {
            if digit == '0' {
                continue;
            }
            let field = FIELDS.get(index).copied().unwrap_or("?");
            let documented = match (mapper, index, digit) {
                (Some(1), 0, '1') => Some("$E000 bit 4 does not disable WRAM"),
                (Some(1), 0, '4') => Some(
                    "$A000 bit 4 does not disable WRAM (SNROM), or disables it (all but SNROM)",
                ),
                (Some(4), 0, '2') => Some("read-only mode not present"),
                _ => None,
            };
            meanings.push(documented.map_or_else(
                || format!("{field} digit {digit}"),
                |text| format!("{field} {digit}: {text}"),
            ));
        }

        meanings
    }
}
