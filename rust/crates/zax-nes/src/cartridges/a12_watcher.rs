use crate::state::{InvalidState, StateReader, StateWriter};

/// Detects the rises of PPU address line A12 that clock an MMC3-style scanline counter
/// (REQ-CORE-114).
///
/// The MMC3 does not see scanlines: it watches A12 through a low-pass filter and counts the
/// rising edges (nesdev, "MMC3"). With the background patterns at $0000 and the sprite
/// patterns at $1000 — or the other way round — A12 rises exactly once per rendered line,
/// which is what gives the board its scanline counter; a game that puts both layers in the
/// same half of CHR gets no IRQ at all, and one that writes $2006 by hand can clock the
/// counter outside rendering, which is how `mmc3_test` drives it.
///
/// The filter is what makes the difference: A12 also dips for the two-dot nametable and
/// attribute fetches of every tile, and those dips must *not* count. The board needs A12 to
/// stay low for some number of CPU cycles before it will accept a rise, and that number is
/// **not the same on every board**, so it is a field here and not a constant: see
/// [`A12Watcher::MMC3_FILTER_DOTS`] and [`A12Watcher::RAMBO1_FILTER_DOTS`].
///
/// One board counts the **other** edge and filters nothing at all: the Acclaim MC-ACC, whose
/// "scanline counter uses a ÷8 prescaler instead of filtering A12, clocked by falling edges"
/// (nesdev, "MMC3"; MR-4-3). [`A12Watcher::fell`] is its clock source, and it is a **second
/// method** rather than a mode inside [`A12Watcher::rose`] so that the two boards which count
/// rises keep the code they had: a field to compare `address & $1000` against, instead of the
/// test against zero the `and` itself performs, would have cost them two instructions on every
/// PPU fetch. Which method a board calls is a compile-time constant — see `Mmc3Board`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct A12Watcher {
    high: bool,
    low_since: i64,
    /// Dots A12 must have stayed low for a rise to count. Chosen once, when the board is
    /// built, so `rose` reads a field instead of a constant and no board pays for another's
    /// threshold. Not part of the savestate: it comes from the board, not from the game.
    filter_dots: i64,
}

impl A12Watcher {
    /// The MMC3's filter. Three CPU cycles are about nine dots; ten rejects the four-dot dips
    /// of the fetch pipeline and accepts the long lows of the sprite fetch phase and of the
    /// $2006 writes `mmc3_test` uses. MR-A12-1 re-checked this against the sources and found it
    /// equivalent in practice: the hardware counts three M2 falling edges, and only a low of 7
    /// to 9 dots could tell the two apart.
    pub const MMC3_FILTER_DOTS: i64 = 10;

    /// The RAMBO-1's filter, which is **five times** the MMC3's (MR-64-3). Its pseudo-code
    /// counts M2 falling edges while PA12 is low — "If 4-bit counter < 16: 4-bit counter++" —
    /// and only clocks the scanline counter on a rise when that counter reached 16, so A12 must
    /// have been low across sixteen M2 falls. At three dots to the CPU cycle that is 48 dots,
    /// where we required ten and Mesen requires 31 (nesdev, "RAMBO-1").
    pub const RAMBO1_FILTER_DOTS: i64 = 48;

    /// Creates a watcher in its power-on state, filtering lows shorter than `filter_dots`.
    #[must_use]
    pub const fn new(filter_dots: i64) -> Self {
        Self {
            high: false,
            low_since: 0,
            filter_dots,
        }
    }

    /// Fewest PPU dots between two rises that count: A12 must have been low for `filter_dots`
    /// after being high for at least one. What a board's `ppu_irq_horizon` multiplies by.
    #[must_use]
    pub const fn min_rise_interval(&self) -> i64 {
        self.filter_dots + 1
    }

    /// Feeds the address the PPU put on its bus and reports whether it counts as a rise.
    pub fn rose(&mut self, address: u16, ppu_clock: i64) -> bool {
        let high = (address & 0x1000) != 0;
        let rose = high && !self.high && ppu_clock - self.low_since >= self.filter_dots;
        if !high && self.high {
            self.low_since = ppu_clock;
        }

        self.high = high;
        rose
    }

    /// Feeds the address and reports whether A12 **fell**, with no filtering whatsoever: the
    /// MC-ACC divides its falls by eight instead of filtering them (MR-4-3). `low_since` is the
    /// filter's and is left alone — no board both filters and counts falls.
    pub fn fell(&mut self, address: u16) -> bool {
        let high = (address & 0x1000) != 0;
        let fell = !high && self.high;
        self.high = high;
        fell
    }

    /// Returns the watcher to its power-on state. `filter_dots` is the board's and survives:
    /// it is wiring, not state.
    pub fn reset(&mut self) {
        self.high = false;
        self.low_since = 0;
    }
}

impl A12Watcher {
    /// Appends the watcher state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_bool(self.high);
        writer.write_i64(self.low_since);
    }

    /// Restores the watcher state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.high = reader.read_bool()?;
        self.low_since = reader.read_i64()?;
        Ok(())
    }
}
