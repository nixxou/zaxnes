// Board names (UxROM, AxROM) read as prose; the register structs carry the IRQ flags of the
// hardware, which are independent bits.
#![allow(clippy::doc_markdown, clippy::struct_excessive_bools)]
//! The mapper boards (port of `src/Zax.Nes/Cartridges/Mappers`).
//!
//! Every board is a small struct that implements [`Mapper`]; the [`super::Board`] enum
//! dispatches to it. The memories shared by all boards stay in the
//! [`CartridgeMemory`] the cartridge owns and are passed to each call.

/// Implements [`Mapper::banks`] for a board with a `banks: BankWindows` field.
macro_rules! banked {
    () => {
        fn banks(&self) -> Option<&$crate::cartridges::BankWindows> {
            Some(&self.banks)
        }

        fn banks_mut(&mut self) -> Option<&mut $crate::cartridges::BankWindows> {
            Some(&mut self.banks)
        }
    };
}

#[cfg(feature = "board-small-boards")]
mod bandai74161;
mod bandai_fcg;
#[cfg(feature = "board-small-boards")]
mod bf9096;
#[cfg(feature = "board-111")]
mod cheapocabra;
#[cfg(any(feature = "board-162", feature = "board-163"))]
mod chr_window;
#[cfg(feature = "board-small-boards")]
mod cnrom_protect;
#[cfg(feature = "board-cony")]
mod cony;
mod daou306;
#[cfg(feature = "board-decode-boards")]
mod decode_boards;
mod discrete;
#[cfg(feature = "board-eeprom")]
mod eeprom24c0x;
#[cfg(feature = "board-164")]
mod eeprom93c66;
mod fme7;
#[cfg(feature = "board-246")]
mod g0151;
#[cfg(feature = "board-huang1")]
mod huang1;
mod irem_h3001;
#[cfg(feature = "board-77")]
mod irem_lrog017;
#[cfg(feature = "board-small-boards")]
mod irem_tam_s1;
mod iremg101;
mod irq_boards;
#[cfg(feature = "board-small-boards")]
mod jaleco_jf11;
#[cfg(feature = "board-jaleco-72-92-86")]
mod jaleco_jf13;
#[cfg(feature = "board-jaleco-72-92-86")]
mod jaleco_jf17;
mod jaleco_ss88006;
#[cfg(feature = "board-jv001")]
mod jv001;
#[cfg(feature = "board-jy-asic")]
mod jy_company;
#[cfg(feature = "board-218")]
mod magic_floor;
#[cfg(feature = "board-91")]
mod mapper091;
mod mmc1;
mod mmc2;
mod mmc3;
#[cfg(feature = "board-219")]
mod mmc3_a9461;
#[cfg(feature = "board-mmc3-chrram")]
mod mmc3_chrram;
#[cfg(feature = "board-292")]
mod mmc3_dragonfighter;
#[cfg(feature = "board-208")]
mod mmc3_gouder208;
#[cfg(feature = "board-263")]
mod mmc3_kof97;
#[cfg(feature = "board-268")]
mod mmc3_outer;
#[cfg(feature = "board-mmc3-scramble")]
mod mmc3_scramble;
#[cfg(feature = "board-262")]
mod mmc3_shero;
#[cfg(feature = "board-189")]
mod mmc3_txc189;
#[cfg(feature = "board-215")]
mod mmc3_unl8237;
mod mmc5;
mod multicart;
mod namco163;
#[cfg(feature = "board-namcot108")]
mod namcot108;
#[cfg(feature = "board-105")]
mod nes_event;
#[cfg(feature = "board-nina03")]
mod nina03;
mod nrom;
#[cfg(feature = "board-96")]
mod oeka_kids;
#[cfg(feature = "board-164")]
mod pec9588;
mod rambo1;
#[cfg(feature = "board-133")]
mod sachen_72008;
#[cfg(feature = "board-sachen8259")]
mod sachen_8259;
#[cfg(feature = "board-sachen-8reg")]
mod sachen_8reg;
#[cfg(any(
    feature = "board-29",
    feature = "board-38",
    feature = "board-107",
    feature = "board-143",
    feature = "board-144",
    feature = "board-145",
    feature = "board-149",
    feature = "board-190"
))]
mod single_latch;
#[cfg(feature = "board-small-boards")]
mod small_boards;
#[cfg(feature = "board-116")]
mod somari;
#[cfg(feature = "board-small-boards")]
mod sunsoft1;
#[cfg(feature = "board-small-boards")]
mod sunsoft2;
mod sunsoft3;
#[cfg(feature = "board-68")]
mod sunsoft4;
#[cfg(feature = "board-small-boards")]
mod taito_tc0190;
mod taito_tc0690;
#[cfg(feature = "board-taito-x1")]
mod taito_x1005;
#[cfg(feature = "board-taito-x1")]
mod taito_x1017;
#[cfg(feature = "board-small-boards")]
mod vrc1;
mod vrc2_vrc4;
mod vrc3;
mod vrc6;
mod vrc7;
mod vrc_irq;
#[cfg(feature = "board-vs-system")]
mod vs_system;
#[cfg(feature = "board-178")]
mod waixing_fs305;

#[cfg(feature = "board-small-boards")]
pub use bandai74161::Bandai74161;
pub use bandai_fcg::BandaiFcg;
#[cfg(feature = "board-small-boards")]
pub use bf9096::Bf9096;
#[cfg(feature = "board-111")]
pub use cheapocabra::Cheapocabra;
#[cfg(any(feature = "board-162", feature = "board-163"))]
pub use chr_window::{ChrWindowBoard, ChrWindowKind};
#[cfg(feature = "board-small-boards")]
pub use cnrom_protect::CnRomProtect;
#[cfg(feature = "board-cony")]
pub use cony::Cony;
pub use daou306::Daou306;
#[cfg(feature = "board-decode-boards")]
pub use decode_boards::DecodeBoard;
pub use discrete::{
    AxRom, CnRom, Codemasters, ColorDreams, CpRom, GxRom, Mapper078, UnRom512, UxRom,
};
#[cfg(feature = "board-eeprom")]
pub use eeprom24c0x::{Eeprom24C0X, EepromKind, EepromPort};
#[cfg(feature = "board-164")]
pub use eeprom93c66::Eeprom93C66;
pub use fme7::Fme7;
#[cfg(feature = "board-246")]
pub use g0151::G0151;
#[cfg(feature = "board-huang1")]
pub use huang1::Huang1;
pub use irem_h3001::IremH3001;
#[cfg(feature = "board-77")]
pub use irem_lrog017::IremLrog017;
#[cfg(feature = "board-small-boards")]
pub use irem_tam_s1::IremTamS1;
pub use iremg101::IremG101;
pub use irq_boards::IrqBoard;
#[cfg(feature = "board-small-boards")]
pub use jaleco_jf11::JalecoJf11;
#[cfg(feature = "board-jaleco-72-92-86")]
pub use jaleco_jf13::JalecoJf13;
#[cfg(feature = "board-jaleco-72-92-86")]
pub use jaleco_jf17::JalecoJf17;
pub use jaleco_ss88006::JalecoSs88006;
#[cfg(feature = "board-jv001")]
pub use jv001::Jv001;
#[cfg(feature = "board-jy-asic")]
pub use jy_company::JyCompany;
#[cfg(feature = "board-218")]
pub use magic_floor::MagicFloor;
#[cfg(feature = "board-91")]
pub use mapper091::Mapper091;
pub use mmc1::Mmc1;
pub use mmc2::Mmc2;
#[cfg(feature = "board-mmc3-multicart")]
pub use mmc3::Mmc3Multicart;
#[cfg(feature = "board-119")]
pub use mmc3::Tqrom;
pub use mmc3::{McAcc, Mmc3};
#[cfg(feature = "board-mmc3-chrram")]
pub use mmc3::{Mmc3ChrRam, Mmc3ChrRam192, Mmc3ChrRam194, Mmc3ChrRam74, WaixingFs303};
#[cfg(feature = "board-219")]
pub use mmc3_a9461::Mmc3A9461;
#[cfg(feature = "board-mmc3-chrram")]
pub use mmc3_chrram::Mmc3ChrRamBoard;
#[cfg(feature = "board-292")]
pub use mmc3_dragonfighter::Mmc3Dragonfighter;
#[cfg(feature = "board-208")]
pub use mmc3_gouder208::Mmc3Gouder208;
#[cfg(feature = "board-263")]
pub use mmc3_kof97::Mmc3Kof97;
#[cfg(feature = "board-268")]
pub use mmc3_outer::Mmc3Outer;
#[cfg(feature = "board-mmc3-scramble")]
pub use mmc3_scramble::Mmc3Scramble;
#[cfg(feature = "board-262")]
pub use mmc3_shero::Mmc3Shero;
#[cfg(feature = "board-189")]
pub use mmc3_txc189::Mmc3Txc189;
#[cfg(feature = "board-215")]
pub use mmc3_unl8237::Mmc3Unl8237;
pub use mmc5::Mmc5;
pub use multicart::{
    Action52, Action53, Caltron6In1, JalecoJ87, Mapper015, Mapper034, Multicart225,
};
pub use namco163::{Namco163, NamcoVariant};
#[cfg(feature = "board-namcot108")]
pub use namcot108::Namcot108;
#[cfg(feature = "board-105")]
pub use nes_event::NesEvent;
#[cfg(feature = "board-nina03")]
pub use nina03::Nina03;
pub use nrom::Nrom;
#[cfg(feature = "board-96")]
pub use oeka_kids::OekaKids;
#[cfg(feature = "board-164")]
pub use pec9588::Pec9588;
#[cfg(feature = "board-158")]
pub use rambo1::Tengen800037;
pub use rambo1::{Rambo1, Rambo1Board};
#[cfg(feature = "board-133")]
pub use sachen_72008::Sachen72008;
#[cfg(feature = "board-sachen8259")]
pub use sachen_8259::Sachen8259;
#[cfg(feature = "board-sachen-8reg")]
pub use sachen_8reg::Sachen8Reg;
#[cfg(any(
    feature = "board-29",
    feature = "board-38",
    feature = "board-107",
    feature = "board-143",
    feature = "board-144",
    feature = "board-145",
    feature = "board-149",
    feature = "board-190"
))]
pub use single_latch::{LatchBoard, SingleLatch};
#[cfg(feature = "board-small-boards")]
pub use small_boards::SmallBoard;
#[cfg(feature = "board-116")]
pub use somari::Somari;
#[cfg(feature = "board-small-boards")]
pub use sunsoft1::Sunsoft1;
#[cfg(feature = "board-small-boards")]
pub use sunsoft2::Sunsoft2;
pub use sunsoft3::Sunsoft3;
#[cfg(feature = "board-68")]
pub use sunsoft4::Sunsoft4;
#[cfg(feature = "board-small-boards")]
pub use taito_tc0190::TaitoTc0190;
pub use taito_tc0690::TaitoTc0690;
#[cfg(feature = "board-taito-x1")]
pub use taito_x1005::TaitoX1005;
#[cfg(feature = "board-taito-x1")]
pub use taito_x1017::TaitoX1017;
#[cfg(feature = "board-small-boards")]
pub use vrc1::Vrc1;
pub use vrc2_vrc4::Vrc2Vrc4;
pub use vrc3::Vrc3;
pub use vrc6::Vrc6;
pub use vrc7::Vrc7;
#[cfg(feature = "board-vs-system")]
pub use vs_system::VsBoard;
#[cfg(feature = "board-178")]
pub use waixing_fs305::WaixingFs305;

use super::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// What [`Mapper::cpu_read_windows`] settles for the CPU read path, cached on
/// [`Cartridge`](crate::cartridges::Cartridge) after every CPU write to the cartridge.
///
/// One struct from one dispatch, which is the point: lot MF-CART step 4 measured the three
/// questions asked separately at `mm2/core` +0.005 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuReadWindows {
    /// [`Mapper::rom_windows`]: where each 8 KiB window of $8000-$FFFF starts in PRG-ROM, or
    /// [`NO_ROM_WINDOW`].
    pub rom: [usize; 4],
    /// The byte offset in PRG-RAM of $6000 ([`Mapper::prg_ram_window`]'s base).
    pub prg_ram_base: usize,
    /// The length that offset wraps at, and **zero when a CPU read of $6000-$7FFF is not RAM**
    /// — the board has it disabled ([`Mapper::prg_ram_enabled`]), or the cartridge carries none.
    /// The enable is folded in here so that the read is one test; see
    /// `Cartridge::default_cpu_read`.
    pub prg_ram_wrap: usize,
}

/// The behaviour a board adds to the shared cartridge state (REQ-CORE-100). Every method has
/// the default of a board that does nothing special; banked boards expose their
/// [`BankWindows`] through [`Mapper::banks`] and get PRG and CHR access for free.
pub trait Mapper {
    /// iNES mapper number.
    fn mapper_number(&self) -> i32;

    /// The bank windows of a banked board.
    fn banks(&self) -> Option<&BankWindows> {
        None
    }

    /// The bank windows of a banked board, mutably.
    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        None
    }

    /// Appends the registers of the board, beyond its bank windows, to `writer`
    /// (REQ-CORE-090).
    fn save_registers(&self, _writer: &mut StateWriter) {}

    /// Restores the registers written by [`Mapper::save_registers`] (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    fn load_registers(&mut self, _reader: &mut StateReader) -> Result<(), InvalidState> {
        Ok(())
    }

    /// Appends the bank windows and the registers of the board to `writer`
    /// (REQ-CORE-090).
    fn save_state(&self, writer: &mut StateWriter) {
        if let Some(banks) = self.banks() {
            banks.save_state(writer);
        }

        self.save_registers(writer);
    }

    /// Restores what [`Mapper::save_state`] wrote (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        if let Some(banks) = self.banks_mut() {
            banks.load_state(reader)?;
        }

        self.load_registers(reader)
    }

    /// Puts the registers back to their power-on values and recomputes the windows.
    fn reset(&mut self, _memory: &mut CartridgeMemory) {}

    /// Whether the board asserts the CPU IRQ line.
    fn irq_asserted(&self) -> bool {
        false
    }

    /// PPU dots during which the IRQ line cannot change through what the PPU does (address
    /// bus, scanline signals), so that the PPU may run behind the CPU for that long:
    /// `Some(i64::MAX)` for a board that does not follow the PPU, `None` when no bound is
    /// known (the PPU then runs in step with the CPU). A board that overrides
    /// [`Mapper::on_ppu_address_bus`] or [`Mapper::on_ppu_scanline`] overrides this too.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        Some(i64::MAX)
    }

    /// True when $6000-$7FFF is readable and writable.
    fn prg_ram_enabled(&self) -> bool {
        true
    }

    /// True when $6000-$7FFF accepts writes.
    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_enabled()
    }

    /// Where $6000-$7FFF lands in PRG-RAM: the byte offset of $6000, and the length the offset
    /// wraps at — the chip, or the part of it the window reaches. The index of `address` is
    /// `(base + (address - 0x6000)) % wrap`, which is what every board that banks its PRG-RAM
    /// computes and the only shape any of the four overrides needs.
    ///
    /// **Asked once per CPU write and not once per CPU read**, which is the whole reason it is a
    /// window and not an offset: [`Cartridge`](crate::cartridges::Cartridge) caches the answer
    /// beside `rom_windows`, so `Cartridge::default_cpu_read` — on the MMC5's every CPU read —
    /// holds no `Board` dispatch for $6000-$7FFF at all. Lot MF-CART is why; its step 1 priced
    /// the dispatch this replaces at one hoisted `leaq` per read.
    ///
    /// A board whose window moves at some other time than a CPU write, a reset or a loaded state
    /// has nowhere to say so, exactly as [`Mapper::rom_windows`] already requires; none does.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        (0, memory.prg_ram().len())
    }

    /// Where each 8 KiB window of $8000-$FFFF starts in PRG-ROM when a CPU read of it is a plain
    /// ROM read, with no side effect and no wrap inside the window; [`NO_ROM_WINDOW`] for the
    /// others. Asked again after every CPU write to the cartridge, a reset and a loaded state:
    /// a board whose windows move at another time, or whose reads of $8000-$FFFF go through
    /// [`Mapper::cpu_read_override`], must return `NO_ROM_WINDOW` for them.
    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        let rom = memory.rom().prg_rom().len();
        self.banks()
            .map_or([NO_ROM_WINDOW; 4], |banks| banks.rom_windows(rom))
    }

    /// Everything the CPU **read** path needs settled, in **one** call: the four windows of
    /// [`Mapper::rom_windows`], and [`Mapper::prg_ram_window`] with
    /// [`Mapper::prg_ram_enabled`] folded into its wrap (see [`CpuReadWindows`]).
    ///
    /// **It exists to be one `Board` dispatch and not three** (lot MF-CART step 4). This runs
    /// after every CPU write to the cartridge, and the three questions it answers used to be
    /// three dispatches on that path — the shape MF-CART step 3 measured at `mm2/core`
    /// **+0.005 %**, Mega Man 2 being an MMC1 that writes its registers often. **No board
    /// overrides this method**: the default body calls the three on `Self`, which is
    /// monomorphised per board, so inside one arm there is no dispatch left to pay.
    fn cpu_read_windows(&self, memory: &CartridgeMemory) -> CpuReadWindows {
        let (prg_ram_base, wrap) = self.prg_ram_window(memory);
        CpuReadWindows {
            rom: self.rom_windows(memory),
            prg_ram_base,
            prg_ram_wrap: if self.prg_ram_enabled() && !memory.prg_ram().is_empty() {
                debug_assert!(wrap != 0, "a readable PRG-RAM window of length zero");
                wrap
            } else {
                0
            },
        }
    }

    /// Reads PRG-ROM, or a readable register, at `address`.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        self.banks()
            .and_then(|banks| banks.read_prg(address, memory.rom().prg_rom(), memory.prg_ram()))
    }

    /// Lets a board answer a CPU read of $4020-$FFFF before the default decode (MMC5
    /// registers, MMC6 RAM halves). `Some(value)` means the board handled it, `value` being
    /// what it drives. Boards with read side effects override this and
    /// [`Mapper::cpu_peek_override`] separately.
    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        self.cpu_peek_override(memory, address)
    }

    /// Side-effect-free counterpart of [`Mapper::cpu_read_override`].
    fn cpu_peek_override(&self, _memory: &CartridgeMemory, _address: u16) -> Option<Option<u8>> {
        None
    }

    /// Lets a board take a CPU write before the PRG-RAM and the register decode see it
    /// (boards whose registers sit below $8000). Returns true when the write is consumed.
    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        _address: u16,
        _value: u8,
    ) -> bool {
        false
    }

    /// Handles a write that is not PRG-RAM.
    fn write_register(&mut self, _memory: &mut CartridgeMemory, _address: u16, _value: u8) {}

    /// Reads CHR as the PPU would, side effects included.
    fn ppu_read(&mut self, memory: &mut CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.ppu_peek(memory, address, fetch)
    }

    /// The CHR the rendering fetches read, for the PPU's fast path: `None` when a fetch has
    /// a side effect or another source (a board that overrides [`Mapper::ppu_read`],
    /// [`Mapper::ppu_peek`] or [`Mapper::on_ppu_address_bus`] must override this one too).
    /// The default matches the default [`Mapper::ppu_peek`].
    fn chr_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        let chr_rom = memory.rom().chr_rom();
        let chr = if chr_rom.is_empty() {
            memory.chr_ram()
        } else {
            chr_rom
        };
        Some(
            self.banks()
                .map_or(ChrView::linear(&[]), |banks| ChrView::new(chr, banks)),
        )
    }

    /// The CHR [`Mapper::ppu_peek`] reads, as a view: what the patterns look like with the
    /// banks in place now, whether or not the rendering fetches have side effects (a board
    /// whose fetches only clock an IRQ counter, MMC3, has one even though [`Mapper::chr_view`]
    /// gives none). `None` when the peek reads another source. Defaults to
    /// [`Mapper::chr_view`].
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.chr_view(memory)
    }

    /// Reads CHR without any side effect.
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        self.banks().map_or(0, |banks| {
            let chr_rom = memory.rom().chr_rom();
            banks.read_chr(
                address,
                if chr_rom.is_empty() {
                    memory.chr_ram()
                } else {
                    chr_rom
                },
            )
        })
    }

    /// Writes CHR-RAM; ignored on a CHR-ROM board.
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if memory.rom().chr_rom().is_empty() {
            if let Some(banks) = self.banks() {
                banks.write_chr(address, value, memory.chr_ram_mut());
            }
        }
    }

    /// Opaque CHR latch state (MMC2/MMC4).
    fn chr_latch_state(&self) -> i32 {
        0
    }

    /// Restores the CHR latch state.
    fn set_chr_latch_state(&mut self, _state: i32) {}

    /// Moves the CHR latches on as the fetch of `tile` from `pattern_table` would.
    fn advance_chr_latch(&mut self, _pattern_table: i32, _tile: i32) {}

    /// The PPU put `address` on its bus (A12 counters).
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, _address: u16, _clock: i64) {}

    /// A write to PPUCTRL.
    fn on_ppu_control(&mut self, _control: u8) {}

    /// The start of a scanline.
    fn on_ppu_scanline(&mut self, _memory: &mut CartridgeMemory, _scanline: i32, _rendering: bool) {
    }

    /// Physical nametable page of logical table `table`, or a negative `FOLLOW_MIRRORING` /
    /// `SERVED_BY_CARTRIDGE`.
    fn nametable_page(&self, _table: i32) -> i32 {
        -1
    }

    /// Reads a nametable byte the board serves itself.
    ///
    /// `memory` is passed because a board may serve its nametables out of the memories the
    /// cartridge owns rather than out of its own registers: the UNROM 512's four-screen mode
    /// takes them from the last 8 KiB of CHR-RAM (MR-30-6), where the MMC5's come from its
    /// `ExRAM` and its fill registers. Only a board whose [`Mapper::nametable_page`] answers
    /// `SERVED_BY_CARTRIDGE` is ever asked, so this is not on any other game's fetch path.
    fn read_nametable(&self, _memory: &CartridgeMemory, _address: u16) -> u8 {
        0
    }

    /// [`Mapper::read_nametable`] with the console's VRAM, for the one board that answers every
    /// nametable fetch while some of the four tables are still CIRAM's: the MMC5 in extended
    /// attribute mode or with the vertical split on (lot MF-LEG-2).
    ///
    /// It exists so that the two features cost **no other board anything**, feature on or off.
    /// `Ppu2C02::read_nametable_as` already branches on `page == SERVED_BY_CARTRIDGE`, so a
    /// board that answers it is asked for the byte and the rest of the PPU's path is untouched;
    /// the only thing the MMC5 lacked there was a way to read the CIRAM page `$5105` names, and
    /// that is `vram`. The default body forwards, so no board but the MMC5 overrides it, and
    /// **the `read_nametable` dispatch keeps its two arms** — the bitset `Board`'s own doc
    /// comment says a third body turns into a jump table.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn read_nametable_from_vram(&self, memory: &CartridgeMemory, address: u16, _vram: &[u8]) -> u8 {
        self.read_nametable(memory, address)
    }

    /// Where a **write** to nametable-space `address` lands in console VRAM on a board that
    /// answers `SERVED_BY_CARTRIDGE` for a table CIRAM still backs, or `None` when the board
    /// keeps or drops the byte itself — which is every board but the MMC5 with
    /// `board-mmc5-exattr-split` on, and the MMC5 itself whenever `$5105` picks `ExRAM` or
    /// fill mode. See [`Mapper::read_nametable_from_vram`].
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn nametable_write_offset(&self, _address: u16) -> Option<usize> {
        None
    }

    /// Whether the board substitutes a background tile's nametable byte, its attribute or its
    /// pattern bank — the MMC5 with `board-mmc5-exattr-split` and either mode armed, and no
    /// other board ever.
    ///
    /// The tile capture asks **once per row**, before its column loop, so that the loop of every
    /// other board holds no `&mut Cartridge` reborrow at all: carrying one cost `Ppu2C02::run_dots`
    /// up to **+9.5 M instructions over 300 frames (+0.128 % on `duelito`)**, measured, and
    /// `docs/progress/MF-LEG-2.md` step 4 has the attribution.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn substitutes_background_tiles(&self) -> bool {
        false
    }

    /// Opaque per-tile state of the two MMC5 substituting modes, for the tile capture to hold
    /// while it walks a row out of display order and put back afterwards — the same
    /// save/walk/restore REQ-CORE-033 and REQ-CORE-039 already make the capture do for the
    /// MMC2 and MMC4 latches. Always 0 on every other board.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn background_capture_state(&self) -> u64 {
        0
    }

    /// Restores what [`Mapper::background_capture_state`] took.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn set_background_capture_state(&mut self, _state: u64) {}

    /// Positions that per-tile state on the tile the capture is about to read: its nametable
    /// `address`, its screen tile `column` (which may be negative or above 31 under the
    /// widescreen extension) and the screen `line` its top row is drawn on.
    #[cfg(feature = "board-mmc5-exattr-split")]
    fn position_captured_background_tile(&mut self, _address: u16, _column: i32, _line: i32) {}

    /// Writes a nametable byte the board serves itself.
    fn write_nametable(&mut self, _memory: &mut CartridgeMemory, _address: u16, _value: u8) {}

    /// One CPU cycle elapsed (`memory.cpu_cycle()` already incremented).
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {}

    /// `M` of REQ-CORE-073: the board's expansion audio in the mixer's integer units
    /// (REQ-CORE-075, AMD-054), or `None` for a board without any — which is every board but
    /// the MMC5, so the bus never looks at the expansion audio for them.
    ///
    /// Only the Famicom's cartridge connector carries an audio input pin, so no NES cartridge
    /// can make such a channel sound. REQ-CORE-075 as amended requires the MMC5's own sound
    /// and leaves the others (VRC6, VRC7, Namco 163, Sunsoft 5B, FDS) optional.
    fn expansion_audio_output(&self) -> Option<i32> {
        None
    }
}

/// A window of [`Mapper::rom_windows`] that is not a plain PRG-ROM read.
pub const NO_ROM_WINDOW: usize = usize::MAX;

/// A fresh window table sized for the memories of `memory`.
pub(crate) fn windows_for(memory: &CartridgeMemory) -> BankWindows {
    let chr_rom = memory.rom().chr_rom().len();
    let chr_len = if chr_rom == 0 {
        memory.chr_ram().len()
    } else {
        chr_rom
    };
    BankWindows::new(
        memory.rom().prg_rom().len(),
        chr_len,
        memory.prg_ram().len(),
    )
}
