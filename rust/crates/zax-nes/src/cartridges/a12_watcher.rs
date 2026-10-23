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
/// stay low for roughly three CPU cycles before it will accept a rise, so the window below
/// is expressed in PPU dots and rejects anything shorter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct A12Watcher {
    high: bool,
    low_since: i64,
}

impl A12Watcher {
    /// Dots A12 must have stayed low for a rise to count. Three CPU cycles are about nine
    /// dots; ten rejects the four-dot dips of the fetch pipeline and accepts the long lows
    /// of the sprite fetch phase and of the $2006 writes `mmc3_test` uses.
    const FILTER_DOTS: i64 = 10;

    /// Creates a watcher in its power-on state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            high: false,
            low_since: 0,
        }
    }

    /// Feeds the address the PPU put on its bus and reports whether it counts as a rise.
    pub fn rose(&mut self, address: u16, ppu_clock: i64) -> bool {
        let high = (address & 0x1000) != 0;
        let rose = high && !self.high && ppu_clock - self.low_since >= Self::FILTER_DOTS;
        if !high && self.high {
            self.low_since = ppu_clock;
        }

        self.high = high;
        rose
    }

    /// Returns the watcher to its power-on state.
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
