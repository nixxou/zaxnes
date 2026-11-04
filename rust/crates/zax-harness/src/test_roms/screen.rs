//! ROMs that report on screen rather than through memory.
//!
//! Step 1 of the lot only routes them here and calls them `manual`, with the PNG of the final
//! frame as the evidence; step 2 adds the nametable reader that turns the documented pass/fail
//! text into a verdict.

use zax_nes::NesCore;

use super::{Outcome, Verdict};

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

/// Runs `core` for `budget` frames and leaves the verdict to a human.
pub fn run(core: &mut NesCore, budget: u32) -> Verdict {
    for _ in 0..budget {
        core.run_frame(0);
    }

    Verdict::new(
        "screen",
        Outcome::Manual,
        "reports on screen only; read the PNG of the final frame".to_owned(),
        budget,
    )
}
