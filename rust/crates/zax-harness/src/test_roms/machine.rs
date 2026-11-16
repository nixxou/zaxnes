//! The five things a test-ROM reader asks of a machine, and the two machines that answer.
//!
//! The four readers of this module's siblings — `blargg`, `screen`, `coin` and `mapperel` — were
//! written against `NesCore` and between them use exactly five operations. Naming those
//! five is what lets `test-roms --engine neshawk` score the *same* ROMs by the *same* evidence on
//! the port of BizHawk's core, which is the only way a verdict comparison with NesHawk means
//! anything: a suite read by a different reader is a different measurement.
//!
//! Of the five, two deserve a word:
//!
//! * `nametable_byte` is a **peek**: it must not clock the board. An MMC3 counts PPU A12 edges
//!   and fires an IRQ off them, so a reader that walked the nametables through the real PPU bus
//!   would be writing the answer it then reads.
//! * `nametable_base` is PPUCTRL's low two bits, which is where the ROM told the PPU to draw
//!   from — not a scan of all four pages. `screen::rows` says why that distinction matters.

use zax_nes::NesCore;

/// One machine, as a test-ROM reader sees it.
pub trait TestMachine {
    /// `zaxnes` or `neshawk` — which machine the report's row was taken on.
    fn engine_name(&self) -> &'static str;

    /// One byte of the CPU address space, with no side effect.
    fn read_memory(&self, address: u16) -> u8;

    /// One frame with `input` on the two pads.
    fn run_frame(&mut self, input: u16);

    /// The RESET line, which keeps the memory.
    fn soft_reset(&mut self);

    /// A **power cycle**, which zeroes the memory. `AccuracyCoin` needs it between retries: the
    /// result bytes have to go back to zero and the menu back to the top of page 0.
    fn power(&mut self);

    /// `$2000` + `$400 *` PPUCTRL's low two bits: the nametable the PPU is showing.
    fn nametable_base(&self) -> u16;

    /// One nametable byte as the board serves it, with no side effect — **not** through the PPU
    /// bus, which would clock an MMC3's A12 filter.
    fn nametable_byte(&self, address: u16) -> u8;

    /// The frame just run, one NES colour index a pixel, for the report's screenshot.
    fn picture_into(&self, out: &mut Vec<u8>);
}

impl TestMachine for NesCore {
    fn engine_name(&self) -> &'static str {
        "zaxnes"
    }

    fn read_memory(&self, address: u16) -> u8 {
        NesCore::read_memory(self, address)
    }

    fn run_frame(&mut self, input: u16) {
        NesCore::run_frame(self, input);
    }

    fn soft_reset(&mut self) {
        NesCore::soft_reset(self);
    }

    fn power(&mut self) {
        NesCore::reset(self);
    }

    fn nametable_base(&self) -> u16 {
        0x2000u16 + 0x400 * u16::from(self.ppu().control() & 0x03)
    }

    fn nametable_byte(&self, address: u16) -> u8 {
        self.ppu().read_nametable_byte(address, self.cartridge())
    }

    fn picture_into(&self, out: &mut Vec<u8>) {
        self.pixel_frame_indices_into(out);
    }
}

/// `zax-neshawk`'s machine as a reader sees it.
///
/// `soft_reset` is a **pending flag** rather than an immediate action, because that is what
/// NesHawk's RESET is: `FrameAdvance` reads `controller.IsPressed("Reset")` at the end of a frame
/// and acts on it at the start of the next. A reader that calls `soft_reset()` then `run_frame()`
/// therefore gets exactly the frame BizHawk would run.
pub struct NeshawkTestMachine {
    pub nes: zax_neshawk::Nes,
    reset_pending: bool,
}

impl NeshawkTestMachine {
    #[must_use]
    pub fn new(nes: zax_neshawk::Nes) -> Self {
        Self {
            nes,
            reset_pending: false,
        }
    }
}

impl TestMachine for NeshawkTestMachine {
    fn engine_name(&self) -> &'static str {
        "neshawk"
    }

    fn read_memory(&self, address: u16) -> u8 {
        self.nes.peek_memory(address)
    }

    fn run_frame(&mut self, input: u16) {
        self.nes.controller_deck.pad = [(input & 0xFF) as u8, (input >> 8) as u8];
        let reset = std::mem::take(&mut self.reset_pending);
        self.nes.frame_advance(reset, false);
    }

    fn soft_reset(&mut self) {
        self.reset_pending = true;
    }

    fn power(&mut self) {
        self.reset_pending = false;
        self.nes.hard_reset();
    }

    fn nametable_base(&self) -> u16 {
        0x2000u16 + 0x400 * u16::from(self.nes.ppu.reg_2000_value() & 0x03)
    }

    fn nametable_byte(&self, address: u16) -> u8 {
        // `$2000-$2FFF` through the board's own mirroring, as `ppubus_peek` would.
        self.nes.peek_ppu(i32::from(address & 0x2FFF))
    }

    fn picture_into(&self, out: &mut Vec<u8>) {
        out.clear();
        out.extend(
            self.nes
                .ppu
                .xbuf
                .iter()
                .map(|&p| u8::try_from(p & 0x3F).unwrap_or(0)),
        );
    }
}
