//! Zaxnes's widescreen extension on NesHawk's bus (lot NESHAWK-HACKS).
//!
//! **Not a port of anything**: NesHawk has no such thing. The extension is `zax_nes::wide`'s
//! [`WideExtension`], the very same register window, execution zone and overclock counter Zaxnes
//! runs; what lives here is only the wiring of it into this port's bus, which is shaped
//! differently from Zaxnes's (`Bus::read_memory`/`write_memory` and the CPU's
//! [`crate::cpu::Mos6502xLink::read_code`] instead of `NesBus::cpu_read_as` and
//! `cpu_read_code_as`).
//!
//! # The rule that keeps the timings
//!
//! The extension is an [`Option`] on the bus, and **`None` unless the host asks for it** — a
//! built-in patch applied, an aid of a hack pack on, or a widescreen margin offered. With it
//! `None`, every read and write takes the C#'s path exactly as before the lot: the tests on it
//! sit only in the `$2000`-`$7FFF` branches of the bus (never on the RAM or PRG-ROM path) and in
//! [`crate::cpu::Mos6502xLink::read_code`], and change no emulated state. A savestate carries no
//! extension block then, so its bytes are those of the build before the lot.
//!
//! With it present but locked (a game that never writes `WIDE` to `$5000`), the window only
//! listens to `$5000` writes, which still reach the board: the machine runs as without it.

use zax_nes::wide::WideExtension;

/// The extension and what the bus needs beside it.
#[derive(Debug, Clone, Default)]
pub struct WideHooks {
    /// The extension itself.
    pub ext: WideExtension,
    /// Whether the instruction being run was fetched from the execution zone: its data
    /// accesses at `$3000`-`$3FFF` reach the extension when the zone holds its data there
    /// (`zax_nes::nes_bus`'s `zone_instruction`, the same rule).
    pub zone_instruction: bool,
    /// The game's wait for the next frame (`BuiltinPatch::idle_loop`): an instruction fetched
    /// there ends the overclock, the cycles left would only run that loop. First and last
    /// address, as `NesCore` keeps it.
    pub idle_loop: Option<(u16, u16)>,
    /// What the frame being drawn latched of the extension for the tile capture (step 4).
    pub frame: WideFrame,
    /// Whether the game has a margin decoder: the capture then keeps each background row for it
    /// ([`Self::rows`]).
    pub capture_rows: bool,
    /// The background rows of the frame being drawn, by row number, for the margin decoder.
    pub rows: Vec<Option<CapturedRow>>,
    /// When each byte of CIRAM was last written, in rendered frames (0: never), and the frame
    /// count it is stamped with (`Ppu2C02`'s `vram_written` and `vram_clock`).
    pub vram_written: Vec<u32>,
    pub vram_clock: u32,
}

/// A background row kept for the margin decoder: where the PPU read it from, and the pattern
/// table and nametable map it was drawn with, which a game may change during the frame
/// (`zax_nes`'s `CapturedRow`, the CHR banks kept as the bytes they showed).
#[derive(Debug, Clone)]
pub struct CapturedRow {
    pub row: zax_nes::wide_decode::BackgroundRow,
    /// The picture line the row's tiles start at.
    pub y: i16,
    /// The 4 KiB pattern table the row's tiles came from, as it was then.
    pub chr: Box<[u8]>,
    /// The board's nametable map then (`BoardBase::apply_mirroring`'s four blocks).
    pub map: [i32; 4],
}

/// The widescreen part of a frame's tile capture, latched at the start of the frame's
/// rendering as `zax_nes`'s `Ppu2C02::begin_rendered_frame` latches it.
#[derive(Debug, Clone, Copy)]
pub struct WideFrame {
    /// The margin in use, in pixels on each side (0: a standard frame).
    pub margin: i32,
    /// The range of x each band of 8 lines showed tiles in.
    pub visible_rows: zax_core_api::VisibleRows,
    /// The sprites drawn in the "asleep" tint, one bit per OAM number.
    pub tinted: u64,
}

impl Default for WideFrame {
    fn default() -> Self {
        Self {
            margin: 0,
            visible_rows: zax_core_api::STANDARD_ROWS,
            tinted: 0,
        }
    }
}

impl WideHooks {
    /// A fresh, locked extension.
    #[must_use]
    pub fn new() -> Box<Self> {
        Box::new(Self {
            vram_written: vec![0; 0x800],
            ..Self::default()
        })
    }

    /// The extension's register a data access of `address` reaches from the zone's own code, as
    /// `$3xxx` standing for `$5xxx`; `None` for every other access (the game's among them).
    #[must_use]
    pub const fn zone_data_address(&self, address: u16) -> Option<u16> {
        if self.zone_instruction && address >= 0x3000 && address < 0x4000 && self.ext.zone_data() {
            Some(address + 0x2000)
        } else {
            None
        }
    }

    /// What a CPU read of `address` in `$4020`-`$7FFF` gets from the extension, `None` when it
    /// does not drive the bus there (`NesBus::read_window`'s order: the PRG window first, then
    /// the registers and RAM, then the window over `$6000`-`$7FFF`).
    #[must_use]
    pub fn read_window(&self, address: u16, prg: &[u8]) -> Option<u8> {
        if WideExtension::claims(address) && !self.ext.zone_data() {
            if let Some(value) = self.prg_window(address, prg) {
                return Some(value);
            }
            if let Some(value) = self.ext.read(address) {
                return Some(value);
            }
        }
        if (0x6000..0x8000).contains(&address) {
            return self.prg_window(address, prg);
        }
        None
    }

    /// `NesBus::wide_window_read`: the PRG window shows the hack pack's code when one runs,
    /// PRG-ROM otherwise.
    fn prg_window(&self, address: u16, prg: &[u8]) -> Option<u8> {
        let offset = self.ext.prg_window(address)?;
        let code = self.ext.code();
        if !code.is_empty() {
            return Some(code.get(offset).copied().unwrap_or(0xFF));
        }
        prg.get(offset % prg.len().max(1)).copied()
    }

    /// A CPU write of `address` in `$4020`-`$5FFF`: the window listens, the board still gets it.
    pub fn listen(&mut self, address: u16, value: u8) {
        if WideExtension::claims(address) && !self.ext.zone_data() {
            self.ext.write(address, value);
        }
    }

    /// An instruction-stream read of `address`: the pack's code byte when `address` is in the
    /// execution zone, `None` otherwise (a plain read then). Records whether the instruction
    /// comes from the zone, for [`Self::zone_data_address`].
    pub fn read_code(&mut self, address: u16) -> Option<u8> {
        let offset = self.ext.zone_offset(address);
        self.zone_instruction = offset.is_some();
        let offset = offset?;
        Some(self.ext.code().get(offset).copied().unwrap_or(0xFF))
    }
}

impl crate::nes::Bus {
    /// [`crate::nes::Bus::read_memory`]'s `$2000`-`$7FFF` with the extension present: the zone's
    /// data at `$3000`-`$3FFF` for the zone's own code, the window and its RAM at `$5000`-`$5FFF`
    /// (and over `$6000`-`$7FFF` when asked), the C#'s path for everything else.
    pub(crate) fn read_memory_wide(&mut self, addr: u16) -> u8 {
        let iaddr = i32::from(addr);
        let db = self.db;
        let Some(wide) = self.wide.as_deref() else {
            unreachable!("only called with the extension present");
        };
        if addr < 0x4000 {
            if let Some(register) = wide.zone_data_address(addr) {
                return wide.ext.read(register).unwrap_or(db);
            }
            return self.ppu_read_reg(iaddr & 7);
        }
        if let Some(value) = wide.read_window(addr, &self.board.base().rom) {
            return value;
        }
        if addr < 0x6000 {
            self.board.read_exp(iaddr - 0x4000, db)
        } else {
            self.board.read_wram(iaddr - 0x6000, db)
        }
    }

    /// [`crate::nes::Bus::write_memory`]'s `$2000`-`$3FFF` and `$4020`-`$5FFF` with the
    /// extension present: the zone's data writes reach the extension, the window listens to the
    /// rest (which the board still gets).
    pub(crate) fn write_memory_wide(&mut self, addr: u16, value: u8) {
        let iaddr = i32::from(addr);
        let Some(wide) = self.wide.as_deref_mut() else {
            unreachable!("only called with the extension present");
        };
        if addr < 0x4000 {
            if let Some(register) = wide.zone_data_address(addr) {
                wide.ext.write(register, value);
                return;
            }
            self.ppu_write_reg(iaddr, value);
            return;
        }
        wide.listen(addr, value);
        self.board.write_exp(iaddr - 0x4000, value);
    }

    /// [`crate::cpu::Mos6502xLink::read_code`] with the extension present: a fetch in the
    /// execution zone reads the pack's code and touches nothing else (the PPU behind
    /// `$2000`-`$3FFF` least of all), as `NesBus::zone_read_cycle` does on Zaxnes; any other is a
    /// plain read.
    pub(crate) fn read_code_wide(&mut self, addr: u16) -> u8 {
        let Some(value) = self
            .wide
            .as_deref_mut()
            .and_then(|wide| wide.read_code(addr))
        else {
            return self.read_memory(addr);
        };
        if !self.oam_dma_exec && !self.dmc_dma_exec {
            self.address_bus = addr;
        }
        self.db = value;
        value
    }
}

impl crate::nes::Nes {
    /// Zaxnes's widescreen overclock (`$5006`), at the start of line 240 as on Zaxnes: that many
    /// scanlines' worth of CPU cycles with the PPU and the APU still and the board clocked,
    /// ended early once the CPU reaches the game's wait for the next frame.
    pub(crate) fn run_overclock(&mut self) {
        let Some(wide) = self.b.wide.as_deref_mut() else {
            return;
        };
        wide.ext.arm_overclock();
        let idle = wide.idle_loop.filter(|_| wide.ext.unlocked());
        while self
            .b
            .wide
            .as_deref_mut()
            .is_some_and(|wide| wide.ext.take_stall_cycle())
        {
            self.run_cpu_stalled();
            if let Some((first, last)) = idle {
                if (first..=last).contains(&self.cpu.PC) {
                    if let Some(wide) = self.b.wide.as_deref_mut() {
                        wide.ext.end_stall();
                    }
                }
            }
        }
    }
}

impl crate::nes::Nes {
    /// The start of a frame's rendering (the pre-render line): the margin the frame's capture
    /// uses is latched (`Ppu2C02::begin_rendered_frame`).
    pub(crate) fn begin_wide_frame(&mut self) {
        let Some(wide) = self.b.wide.as_deref_mut() else {
            return;
        };
        let margin = wide.ext.margin();
        wide.vram_clock = wide.vram_clock.wrapping_add(1);
        if wide.capture_rows {
            wide.rows.clear();
            wide.rows.resize(31, None);
        }
        #[allow(clippy::cast_possible_truncation)] // At most `MAX_MARGIN`.
        let half = margin as i16;
        wide.frame = WideFrame {
            margin,
            visible_rows: [(-half, 256 + half); 30],
            tinted: 0,
        };
    }
}
