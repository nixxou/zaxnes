//! Shay Green's ("blargg") result protocol, as every `readme.txt` of the suites in
//! `nes-test-roms/` states it under "Output at $6000":
//!
//! > All text output is written starting at `$6004`, with a zero-byte terminator at the end. [...]
//! > The test status is written to `$6000`. `$80` means the test is running, `$81` means the
//! > test needs the reset button pressed, but delayed by at least 100 msec from now.
//! > `$00-$7F` means the test has completed and given that result code.
//! >
//! > To allow an emulator to know when one of these tests is running and the data at `$6000+`
//! > is valid, as opposed to some other NES program, `$DE $B0 $61` is written to
//! > `$6001-$6003`.
//!
//! A result code of 0 is a pass; 1 is a plain failure and 2 or more is a specific reason named
//! in the text (`readme.txt`, "Audible output").
//!
//! Two details make the difference between a reader that works and one that invents results:
//!
//! * the signature is **not** enough on its own. Work RAM at power-on can hold anything, and a
//!   ROM that never implements the protocol can match by accident, so a code is only believed
//!   after `$80` has actually been seen at `$6000`;
//! * `$81` is a request, not a result. The harness waits the documented delay — at least
//!   100 ms, so seven frames — and then presses the RESET *button*
//!   ([`NesCore::soft_reset`], AMD-044 REQ-CORE-122), not a power cycle, which is what
//!   `cpu_reset/ram_after_reset` tells apart.

use serde_json::json;
use zax_nes::NesCore;

use super::{one_line, Outcome, Verdict};

/// The three bytes at `$6001-$6003` that say the protocol is in use.
const SIGNATURE: [u8; 3] = [0xDE, 0xB0, 0x61];

/// "The test is running."
const RUNNING: u8 = 0x80;

/// "Press the reset button, at least 100 ms from now."
const NEEDS_RESET: u8 = 0x81;

/// Frames waited before the RESET button is pressed: the readme asks for at least 100 ms, and
/// seven NTSC frames are about 117 ms.
const RESET_DELAY: u32 = 7;

/// Where the zero-terminated text starts, and the end of work RAM.
const TEXT: u16 = 0x6004;
const TEXT_END: u16 = 0x8000;

/// The frame budget of the blargg ROM at relative path `lower` (lower-cased).
///
/// A ROM stops the instant `$6000` leaves `$80`, so the budget only decides how long a hang is
/// given before it is called a timeout. The long budgets are the suites that genuinely stress
/// the PPU or the OAM for minutes of emulated time.
#[must_use]
pub fn frame_budget(lower: &str) -> u32 {
    const LONG: [&str; 6] = [
        "oam_stress",
        "ppu_read_buffer",
        "cpu_exec_space",
        "instr_timing",
        "stress",
        "ppu_vbl_nmi",
    ];
    if LONG.iter().any(|suite| lower.contains(suite)) {
        32_400 // nine minutes of emulated time
    } else {
        7_200 // two minutes
    }
}

/// Frames a ROM is given to arm the protocol before the reader gives up on it.
///
/// Half of the community collection predates `$6000`: the 2005 PPU, sprite-hit and
/// sprite-overflow suites report on screen only. A ROM that does implement the protocol writes
/// `$80` in its first frames, so ten seconds of emulated time is a generous bound and keeps the
/// screen-only ROMs from each burning a two-minute budget.
const PROBE: u32 = 600;

/// What the reader could make of a ROM.
#[derive(Debug, Clone)]
pub enum Read {
    /// The protocol spoke.
    Verdict(Verdict),
    /// The ROM never armed the protocol; `frames` is what the probe spent.
    NoProtocol {
        /// Frames run before the reader gave up.
        frames: u32,
    },
}

/// Runs `core` until the protocol gives a result or `budget` frames have been run.
///
/// Returns [`Read::NoProtocol`] — after only [`PROBE`] frames — when the ROM never writes `$80`
/// at `$6000` behind the signature, so the caller can read its screen instead.
pub fn run(core: &mut NesCore, budget: u32) -> Read {
    let mut seen_running = false;
    let mut resets = 0u32;
    let mut reset_at: Option<u32> = None;

    for frame in 1..=budget {
        core.run_frame(0);

        if let Some(due) = reset_at {
            if frame >= due {
                core.soft_reset();
                reset_at = None;
                resets += 1;
                // A soft reset keeps the work RAM, so `$6000` still holds the `$81` that asked
                // for it. Forgetting that `$80` was seen makes the reader wait for the ROM to
                // write `$80` again before it believes anything at `$6000`; without this a
                // single request turns into a reset every seven frames for ever.
                seen_running = false;
                continue;
            }
        }

        if signed(core) {
            match core.read_memory(0x6000) {
                RUNNING => seen_running = true,
                NEEDS_RESET if seen_running && reset_at.is_none() => {
                    reset_at = Some(frame + RESET_DELAY);
                }
                NEEDS_RESET => {}
                status if seen_running => {
                    return Read::Verdict(finish(core, status, frame, resets))
                }
                _ => {}
            }
        }

        if frame >= PROBE && !seen_running && resets == 0 {
            return Read::NoProtocol { frames: frame };
        }
    }

    let text = one_line(&text_of(core));
    let mut verdict = Verdict::new(
        "blargg",
        Outcome::Timeout,
        if text.is_empty() {
            "no result after the frame budget".to_owned()
        } else {
            format!(
                "no result after the frame budget; last text: {}",
                tail(&text)
            )
        },
        budget,
    );
    verdict.detail.insert("text".into(), json!(text));
    verdict.detail.insert("resets".into(), json!(resets));
    Read::Verdict(verdict)
}

/// Whether a ROM uses the protocol at all, used to decide between this reader and the screen
/// one without running a second time.
#[must_use]
pub fn signed(core: &NesCore) -> bool {
    (0..3).all(|offset| core.read_memory(0x6001 + offset) == SIGNATURE[usize::from(offset)])
}

/// Whether a finished ROM said "Done" rather than "Passed".
///
/// `tests_done` in blargg's own `common/testing.s` prints "Done" and exits with code 0 when the
/// ROM never called `set_test`, and the readme says of exactly those ROMs: "Only a test which
/// prints 'done' at the end requires that you watch/listen while it runs in order to determine
/// whether it passed. Such tests involve things which the CPU cannot directly test." Code 0
/// therefore does **not** mean the ROM passed; it means the ROM finished and has nothing to
/// say about it, so the row is `manual` and the PNG is the evidence.
fn says_done(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("done") && !lower.contains("passed")
}

/// The zero-terminated text the ROM wrote from `$6004`.
#[must_use]
pub fn text_of(core: &NesCore) -> String {
    (TEXT..TEXT_END)
        .map(|address| core.read_memory(address))
        .take_while(|&byte| byte != 0)
        .map(char::from)
        .collect()
}

fn finish(nes: &NesCore, status: u8, frame: u32, resets: u32) -> Verdict {
    let text = one_line(&text_of(nes));
    let outcome = match status {
        0 if says_done(&text) => Outcome::Manual,
        0 => Outcome::Pass,
        _ => Outcome::Fail,
    };
    let short = tail(&text);
    let message = match (outcome, short.is_empty()) {
        (Outcome::Manual, _) => {
            format!("{short} — the readme says a \"done\" test has to be watched or heard")
        }
        (_, true) if status == 0 => "passed".to_owned(),
        (_, false) if status == 0 => short,
        (_, true) => format!("failed with code {status}"),
        (_, false) => format!("{short} (code {status})"),
    };

    let mut verdict = Verdict::new("blargg", outcome, message, frame);
    verdict.code = Some(u32::from(status));
    verdict.detail.insert("text".into(), json!(text));
    verdict.detail.insert("resets".into(), json!(resets));
    verdict
}

/// The end of a ROM's output: its last three lines.
///
/// A blargg ROM prints everything it has to say and finishes with the name the failing
/// `set_test` gave, the ROM's own filename and `Passed` or `Failed #<code>`. The whole text is
/// kept in `detail.text`; a row of the report wants the conclusion, which is that tail. The
/// multi-test builds put the failing sub-test's name there too, so nothing is lost.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text
        .split(" / ")
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let start = lines.len().saturating_sub(3);
    lines[start..].join(" / ")
}
