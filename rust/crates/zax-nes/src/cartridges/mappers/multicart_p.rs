//! Lot MAPPERS-P: the discrete boards puNES runs and Zaxnes did not (`docs/progress/MAPPERS-P.md`).
//!
//! Each kind is two functions, kept side by side: what a CPU write does to the registers
//! ([`MultiP::latch`]) and what the registers map ([`MultiP::map`]). The behaviour of every kind is
//! read from puNES 0.111 (`src/core/mappers/mapper_NNN.c`, read, not copied) and, where it exists,
//! nesdev's page for the number; each arm names its source. Where puNES leaves a part out (a
//! microphone, a DIP switch), the arm says what stands in for it, and FIDELITY-LEADS J37 lists it.
//!
//! puNES conventions the arms translate: `extcl_cpu_wr_mem` sees `$8000-$FFFF` unless the board sets
//! `extend_wr` (then `$4020-$7FFF` too); `memmap_auto_*` banks wrap on the chip's size, as
//! [`BankWindows`] does; `mirroring_H`/`mirroring_V` are horizontal and vertical.
use super::{windows_for, Mapper, NO_ROM_WINDOW};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// The boards of this module, by iNES / NES 2.0 number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiPKind {
    /// 55, `mapper_055.c` (BTL-MARIO1-MALEE2): NROM-256 with 2 KiB of PRG-ROM at `$6000` and
    /// 2 KiB of work RAM at `$7000`.
    M55,
    /// 81, `mapper_081.c` (NTDEC N715021, *Super Gun*): address latch, A2-A3 the 16 KiB bank, A0-A1
    /// the 8 KiB CHR bank.
    M81,
    /// 122, `mapper_184.c` (Sunsoft-1 wiring, the number puNES maps onto 184's code): `$6000-$7FFF`
    /// two 4 KiB CHR banks.
    M122,
    /// 237, `mapper_237.c` (Teletubbies 420-in-1): address and data latch, a lock bit, a DIP read.
    M237,
    /// 389, `mapper_389.c` (Caltron 9-in-1): three address registers at `$8000`, `$9000` and
    /// `$A000-$FFFF`.
    M389,
    /// 517, `mapper_517.c` (Kkachi-wa Norae Chingu, a karaoke with a microphone ADC at
    /// `$6000-$6FFF`).
    M517,
}

impl MultiPKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            55 => Self::M55,
            81 => Self::M81,
            122 => Self::M122,
            237 => Self::M237,
            389 => Self::M389,
            517 => Self::M517,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M55 => 55,
            Self::M81 => 81,
            Self::M122 => 122,
            Self::M237 => 237,
            Self::M389 => 389,
            Self::M517 => 517,
        }
    }

    /// Whether the board takes writes below `$8000` (puNES `extend_wr`), and where.
    const fn decodes_low(self, address: u16) -> bool {
        match self {
            Self::M122 => address >= 0x6000,
            _ => false,
        }
    }

    /// Whether `$6000-$7FFF` is something else than work RAM.
    const fn owns_6000(self) -> bool {
        matches!(self, Self::M55 | Self::M122 | Self::M517)
    }
}

/// A board of this lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiP {
    banks: BankWindows,
    kind: MultiPKind,
    /// The latched registers, their meaning per kind.
    regs: [u16; 4],
    /// Mapper 517's ADC: state, and the two counters it reads out.
    adc: [i16; 3],
}

impl MultiP {
    pub(crate) fn new(memory: &CartridgeMemory, kind: MultiPKind) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            kind,
            regs: [0; 4],
            adc: [0; 3],
        };
        let _ = board.map();
        board
    }

    /// A CPU write the board decodes, `$4020-$FFFF`.
    fn latch(&mut self, address: u16, value: u8) {
        let r = &mut self.regs;
        match self.kind {
            MultiPKind::M55 => {}
            // `extcl_cpu_wr_mem_081`: the whole address is the register.
            MultiPKind::M81 => r[0] = address,
            MultiPKind::M122 => r[0] = u16::from(value),
            // `extcl_cpu_wr_mem_237`: with bit 1 of the address register set (the lock), only the
            // inner three bits of the data register move.
            MultiPKind::M237 => {
                if r[0] & 0x02 != 0 {
                    r[1] = (r[1] & 0xF8) | u16::from(value & 0x07);
                } else {
                    r[0] = address;
                    r[1] = u16::from(value);
                }
            }
            // `extcl_cpu_wr_mem_389`: by the top address nibble.
            MultiPKind::M389 => match address & 0xF000 {
                0x8000 => r[0] = address & 0xFF,
                0x9000 => r[1] = address & 0xFF,
                _ => r[2] = address & 0x0F,
            },
            // `extcl_cpu_wr_mem_517`: a write to `$8000-$8FFF` samples the microphone (silent
            // here: puNES's own TODO, FIDELITY-LEADS J37), every write is the bank.
            MultiPKind::M517 => {
                if (0x8000..=0x8FFF).contains(&address) {
                    let data = 0i16;
                    self.adc[1] = data >> 2;
                    self.adc[2] = 0x40 - self.adc[1] - ((data & 0x03) << 2);
                    self.adc[0] = 0;
                }
                r[0] = u16::from(value);
            }
        }
    }

    /// The banks the registers select, and the mirroring when the board drives it.
    fn map(&mut self) -> Option<NametableMirroring> {
        let r = self.regs.map(i32::from);
        let b = &mut self.banks;
        let mirror = |vertical: bool| {
            Some(if vertical {
                NametableMirroring::Vertical
            } else {
                NametableMirroring::Horizontal
            })
        };
        match self.kind {
            MultiPKind::M55 => {
                b.map_prg_32k(0);
                None
            }
            MultiPKind::M81 => {
                b.map_prg_16k(0, r[0] >> 2);
                b.map_prg_16k(2, -1);
                b.map_chr_8k(r[0] & 0x03);
                None
            }
            MultiPKind::M122 => {
                b.map_prg_32k(0);
                b.map_chr_4k(0, r[0] & 0x0F);
                b.map_chr_4k(4, r[0] >> 4);
                None
            }
            // `prg_fix_237`: an outer 128 KiB block from A2 and D3-D4, four inner modes in D6-D7.
            MultiPKind::M237 => {
                let base = ((r[0] & 0x04) << 3) | (r[1] & 0x18);
                let (first, second) = match r[1] & 0xC0 {
                    0x00 => (base | (r[1] & 0x07), base | 0x07),
                    0x40 => (base | (r[1] & 0x06), base | 0x07),
                    0x80 => (base | (r[1] & 0x07), base | (r[1] & 0x07)),
                    _ => (base | (r[1] & 0x06), base | (r[1] & 0x06) | 0x01),
                };
                b.map_prg_16k(0, first);
                b.map_prg_16k(2, second);
                b.map_chr_8k(0);
                mirror(r[1] & 0x20 == 0)
            }
            // `prg_fix_389`/`chr_fix_389`: UNROM-like mode with `$9000` bit 1, NROM-256 otherwise.
            MultiPKind::M389 => {
                if r[1] & 0x02 != 0 {
                    b.map_prg_16k(0, (r[0] >> 2) | ((r[2] & 0x0C) >> 2));
                    b.map_prg_16k(2, (r[0] >> 2) | 0x03);
                } else {
                    b.map_prg_32k(r[0] >> 3);
                }
                b.map_chr_8k(((r[1] & 0x38) >> 1) | (r[2] & 0x03));
                mirror(r[0] & 0x01 == 0)
            }
            MultiPKind::M517 => {
                b.map_prg_16k(0, r[0]);
                b.map_prg_16k(2, -1);
                None
            }
        }
    }

    /// Whether reads of `$8000-$FFFF` cannot come from the cached windows now.
    const fn reads_are_special(&self) -> bool {
        matches!(self.kind, MultiPKind::M237) && self.regs[0] & 0x01 != 0
    }

    /// A read of `$6000-$7FFF` the board answers itself, with its side effect: `Some(None)` is
    /// open bus (the `Mapper::cpu_read_override` shape).
    #[allow(clippy::option_option)]
    fn read_6000(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        match self.kind {
            // `wram_fix_055`: `$6000-$6FFF` PRG-ROM 4 KiB bank 8 (2 KiB mirrored on the board),
            // `$7000-$7FFF` work RAM.
            MultiPKind::M55 if address < 0x7000 => {
                let rom = memory.rom().prg_rom();
                if rom.is_empty() {
                    return Some(None);
                }
                Some(
                    rom.get((0x8000 + usize::from(address & 0x0FFF)) % rom.len())
                        .copied(),
                )
            }
            MultiPKind::M55 => {
                let ram = memory.prg_ram();
                (!ram.is_empty()).then(|| Some(ram[usize::from(address & 0x0FFF) % ram.len()]))
            }
            MultiPKind::M122 => Some(None),
            // `extcl_cpu_rd_mem_517`: the ADC's serial read-out.
            MultiPKind::M517 if address < 0x7000 => {
                let bit = if address == 0x6000 {
                    match self.adc[0] {
                        0 => {
                            self.adc[0] = 1;
                            0
                        }
                        1 => {
                            self.adc[0] = 2;
                            1
                        }
                        _ => {
                            if self.adc[2] > 0 {
                                self.adc[2] -= 1;
                                1
                            } else {
                                self.adc[0] = 0;
                                0
                            }
                        }
                    }
                } else {
                    let high = self.adc[1];
                    self.adc[1] = high.saturating_sub(1);
                    u8::from(high <= 0)
                };
                Some(Some(bit))
            }
            _ => None,
        }
    }
}

impl Mapper for MultiP {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        for reg in self.regs {
            writer.write_u16(reg);
        }
        for value in self.adc {
            writer.write_i16(value);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for reg in &mut self.regs {
            *reg = reader.read_u16()?;
        }
        for value in &mut self.adc {
            *value = reader.read_i16()?;
        }
        let _ = self.map();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        let kind = self.kind;
        *self = Self::new(memory, kind);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
        }
    }

    fn prg_ram_enabled(&self) -> bool {
        !self.kind.owns_6000()
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address >= 0x8000 || !self.kind.decodes_low(address) {
            // Mapper 55's `$7000-$7FFF` is RAM the board serves itself.
            if self.kind == MultiPKind::M55 && (0x7000..=0x7FFF).contains(&address) {
                let ram = memory.prg_ram_mut();
                if !ram.is_empty() {
                    let len = ram.len();
                    ram[usize::from(address & 0x0FFF) % len] = value;
                }
                return true;
            }
            return false;
        }
        self.latch(address, value);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
        }
        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        self.latch(address, value);
        if let Some(mirroring) = self.map() {
            memory.set_mirroring(mirroring);
        }
    }

    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        if self.reads_are_special() {
            return [NO_ROM_WINDOW; 4];
        }
        self.banks.rom_windows(memory.rom().prg_rom().len())
    }

    fn cpu_read_override(&mut self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if (0x6000..=0x7FFF).contains(&address) && self.kind.owns_6000() {
            return self.read_6000(memory, address);
        }
        self.cpu_peek_override(memory, address)
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if (0x6000..=0x7FFF).contains(&address) && self.kind.owns_6000() {
            if self.kind == MultiPKind::M517 && address < 0x7000 {
                return Some(Some(0));
            }
            return self.clone().read_6000(memory, address);
        }
        // `extcl_cpu_rd_mem_237`: with A0 of the address register set, `$8000-$FFFF` reads the
        // DIP switches (none set here: J37).
        if address >= 0x8000 && self.reads_are_special() {
            return Some(Some(0));
        }
        None
    }
}
