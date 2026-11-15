//! Lot MAPPERS-P: multicarts whose ASIC switches between two mapper chips at run time
//! (`docs/progress/MAPPERS-P.md`).
//!
//! Read from puNES 0.111 (`mapper_351.c`, `mapper_556.c`; read, not copied). The board holds
//! each chip whole ([`Mmc3`], [`Vrc2Vrc4`], [`Mmc1`]) and routes the CPU writes, the CPU-cycle and PPU-bus hooks and the
//! IRQ line to the chip its mode register selects; the chip's windows are then narrowed to the
//! board's outer bank with [`BankWindows::remap_prg_banks`]/[`BankWindows::remap_chr_banks`],
//! which is puNES's `(base & ~mask) | (value & mask)` applied after the chip's own swap. An
//! [`super::IrqBoard`] variant, boxed, because both chips' IRQs are that enum's business.
use super::{Mapper, Mmc1, Mmc3, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The boards of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipSwitchKind {
    /// 556, `mapper_556.c` (JY-215 relative): four outer registers written in turn at
    /// `$5000-$5FFF` (until `reg[3]` D7 locks them), `reg[2]` D7 selecting a VRC4 (A0 on CPU
    /// A0|A2, A1 on A1|A3, A2 and A3 swapped above A11) instead of the MMC3.
    M556,
    /// 351, `mapper_351.c` (Techline XB): `$5000` D0-D1 pick the MMC3, the MMC1 or a VRC4e
    /// (A2/A3, swapped above A11), `$5001` the outer PRG bank, `$5002` the NROM, CHR-ROM and
    /// mask modes; `$5000-$5FFF` reads the DIP switches.
    M351,
}

impl ChipSwitchKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            556 => Self::M556,
            351 => Self::M351,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M556 => 556,
            Self::M351 => 351,
        }
    }
}

/// A board of this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipSwitch {
    kind: ChipSwitchKind,
    /// The outer registers, their meaning per kind.
    regs: [u8; 4],
    /// 556's write cursor.
    index: u8,
    mmc3: Mmc3,
    vrc: Vrc2Vrc4,
    /// 351's third chip.
    mmc1: Mmc1,
}

/// The chip a board is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chip {
    Mmc3,
    Vrc4,
    Mmc1,
}

impl ChipSwitch {
    pub(crate) fn new(memory: &CartridgeMemory, kind: ChipSwitchKind) -> Self {
        let number = kind.number();
        let mut board = Self {
            kind,
            regs: Self::power_on_regs(kind),
            index: 0,
            mmc3: Mmc3::new(memory, number),
            // `init_VRC2and4(VRC24_VRC4, 0x05, 0x0A, …)`: index bit 0 from A0 or A2, bit 1 from
            // A1 or A3.
            vrc: if kind == ChipSwitchKind::M351 {
                // `init_VRC2and4(VRC24_VRC4, 0x04, 0x08, …)`: the VRC4e wiring.
                Vrc2Vrc4::new_vrc4_or_lines(memory, number, [2, 3], [2, 3])
            } else {
                Vrc2Vrc4::new_vrc4_or_lines(memory, number, [0, 1], [2, 3])
            },
            mmc1: Mmc1::new(memory),
        };
        board.apply(None);
        board
    }

    /// `map_init_556`: `reg[2] = $0F`, the CHR mask fully open.
    const fn power_on_regs(kind: ChipSwitchKind) -> [u8; 4] {
        match kind {
            ChipSwitchKind::M556 => [0, 0, 0x0F, 0],
            ChipSwitchKind::M351 => [0; 4],
        }
    }

    /// The chip in use: `reg[2]` D7 on 556 (VRC4 or MMC3), `switch_mode` on 351.
    const fn chip(&self) -> Chip {
        match self.kind {
            ChipSwitchKind::M556 if self.regs[2] & 0x80 != 0 => Chip::Vrc4,
            ChipSwitchKind::M556 => Chip::Mmc3,
            ChipSwitchKind::M351 => match self.regs[0] & 0x03 {
                2 => Chip::Mmc1,
                3 => Chip::Vrc4,
                _ => Chip::Mmc3,
            },
        }
    }

    const fn vrc_active(&self) -> bool {
        matches!(self.chip(), Chip::Vrc4)
    }

    /// The outer banks and masks the chip's windows go through, in 8 KiB and 1 KiB units:
    /// `prg_base`, `prg_mask`, `chr_base`, `chr_mask` of each file (351's MMC1 halves them to
    /// its 16 KiB and 4 KiB banks, which in these units keep the low bits open).
    fn outer(&self) -> (i32, i32, i32, i32) {
        let r = self.regs.map(i32::from);
        match self.kind {
            ChipSwitchKind::M556 => (
                ((r[3] & 0x40) << 2) | r[1],
                !r[3] & 0x3F,
                ((r[3] & 0x40) << 6) | ((r[2] & 0xF0) << 4) | r[0],
                0xFF >> (!r[2] & 0x0F),
            ),
            ChipSwitchKind::M351 => {
                let (prg_base, prg_mask) = (r[1] >> 1, 0x1F >> ((r[2] & 0x04) >> 2));
                let chr_base = r[0] << 1;
                let chr_mask = if r[2] & 0x10 != 0 {
                    0x1F
                } else if r[2] & 0x20 != 0 {
                    0x7F
                } else {
                    0xFF
                };
                if self.chip() == Chip::Mmc1 {
                    (
                        ((prg_base >> 1) & !(prg_mask >> 1)) << 1,
                        ((prg_mask >> 1) << 1) | 0x01,
                        ((chr_base >> 2) & !(chr_mask >> 2)) << 2,
                        ((chr_mask >> 2) << 2) | 0x03,
                    )
                } else {
                    (prg_base, prg_mask, chr_base, chr_mask)
                }
            }
        }
    }

    /// The chip in use rebuilds its windows, then the board narrows them. The MMC1 needs the
    /// memory to rebuild (its mirroring and size); without it, as at power-on, its windows are
    /// only narrowed.
    fn apply(&mut self, memory: Option<&mut CartridgeMemory>) {
        let (prg_base, prg_mask, chr_base, chr_mask) = self.outer();
        let r = self.regs.map(i32::from);
        let banks = match self.chip() {
            Chip::Vrc4 => {
                self.vrc.refresh_banks();
                self.vrc.banks_mut()
            }
            Chip::Mmc3 => {
                self.mmc3.refresh_banks();
                self.mmc3.banks_mut()
            }
            Chip::Mmc1 => {
                if let Some(memory) = memory {
                    self.mmc1.refresh_banks(memory);
                }
                self.mmc1.banks_mut()
            }
        };
        let Some(b) = banks else {
            return;
        };
        b.remap_prg_banks(prg_mask, prg_base & !prg_mask);
        b.remap_chr_banks(chr_mask, chr_base & !chr_mask);
        if self.kind == ChipSwitchKind::M351 {
            // `prg_fix`: `$5002` D4 the NROM modes, D2 NROM-128.
            if r[2] & 0x10 != 0 {
                if r[2] & 0x04 != 0 {
                    b.map_prg_16k(0, r[1] >> 2);
                    b.map_prg_16k(2, r[1] >> 2);
                } else {
                    b.map_prg_32k(r[1] >> 3);
                }
            }
            // `chr_fix`: D0 the CHR-RAM (here the image's own CHR, J37), D6 an 8 KiB CHR bank.
            if r[2] & 0x01 != 0 {
                b.map_chr_8k(0);
            } else if r[2] & 0x40 != 0 {
                b.map_chr_8k(r[0] >> 2);
            }
        }
    }

    /// `extcl_cpu_wr_mem_556`: the four registers in turn, unless `reg[3]` D7 locks them;
    /// `extcl_cpu_wr_mem_351`: `$5000`-`$5002` by A0-A1.
    fn write_low(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) -> bool {
        if !(0x5000..=0x5FFF).contains(&address) {
            return false;
        }
        match self.kind {
            ChipSwitchKind::M556 => {
                if self.regs[3] & 0x80 == 0 {
                    self.regs[usize::from(self.index)] = value;
                    self.index = (self.index + 1) & 0x03;
                }
            }
            ChipSwitchKind::M351 => {
                let register = usize::from(address & 0x03);
                if register < 3 {
                    self.regs[register] = value;
                }
            }
        }
        self.apply(Some(memory));
        true
    }
}

impl Mapper for ChipSwitch {
    fn banks(&self) -> Option<&BankWindows> {
        match self.chip() {
            Chip::Vrc4 => self.vrc.banks(),
            Chip::Mmc3 => self.mmc3.banks(),
            Chip::Mmc1 => self.mmc1.banks(),
        }
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        match self.chip() {
            Chip::Vrc4 => self.vrc.banks_mut(),
            Chip::Mmc3 => self.mmc3.banks_mut(),
            Chip::Mmc1 => self.mmc1.banks_mut(),
        }
    }

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.regs);
        writer.write_u8(self.index);
        self.mmc3.save_registers(writer);
        self.vrc.save_registers(writer);
        if self.kind == ChipSwitchKind::M351 {
            self.mmc1.save_registers(writer);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        self.index = reader.read_u8()? & 0x03;
        self.mmc3.load_registers(reader)?;
        self.vrc.load_registers(reader)?;
        if self.kind == ChipSwitchKind::M351 {
            self.mmc1.load_registers(reader)?;
        }
        // The windows themselves come back with the block `Mapper::save_state` wrote first.
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = Self::power_on_regs(self.kind);
        self.index = 0;
        self.mmc3.reset(memory);
        self.vrc.reset(memory);
        self.mmc1.reset(memory);
        self.apply(Some(memory));
    }

    fn irq_asserted(&self) -> bool {
        match self.chip() {
            Chip::Vrc4 => self.vrc.irq_asserted(),
            Chip::Mmc3 => self.mmc3.irq_asserted(),
            Chip::Mmc1 => false,
        }
    }

    fn prg_ram_enabled(&self) -> bool {
        match self.chip() {
            Chip::Vrc4 => true,
            Chip::Mmc3 => self.mmc3.prg_ram_enabled(),
            Chip::Mmc1 => self.mmc1.prg_ram_enabled(),
        }
    }

    fn prg_ram_writable(&self) -> bool {
        match self.chip() {
            Chip::Vrc4 => true,
            Chip::Mmc3 => self.mmc3.prg_ram_writable(),
            Chip::Mmc1 => self.mmc1.prg_ram_writable(),
        }
    }

    /// `extcl_cpu_rd_mem_351`: `$5000-$5FFF` reads `(open bus & $F8) | DIP`; the DIP is 0 and the
    /// high address byte stands in for the bus (FIDELITY-LEADS J33).
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if self.kind == ChipSwitchKind::M351 && (0x5000..=0x5FFF).contains(&address) {
            return Some(address.to_be_bytes()[0] & 0xF8);
        }
        self.banks()
            .and_then(|banks| banks.read_prg(address, memory.rom().prg_rom(), memory.prg_ram()))
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        address < 0x6000 && self.write_low(memory, address, value)
    }

    /// `extcl_cpu_wr_mem_556`: the chip in use; the VRC4 sees A2 and A3 swapped above A11.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        match self.chip() {
            Chip::Vrc4 => {
                let address = if address & 0x0800 != 0 {
                    (address & 0xFFF3) | ((address & 0x0004) << 1) | ((address & 0x0008) >> 1)
                } else {
                    address
                };
                self.vrc.write_register(memory, address, value);
            }
            Chip::Mmc3 => self.mmc3.write_register(memory, address, value),
            Chip::Mmc1 => self.mmc1.write_register(memory, address, value),
        }
        self.apply(Some(memory));
    }

    /// `extcl_cpu_every_cycle_556`: the VRC4's counter while it is in use.
    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        if self.vrc_active() {
            self.vrc.on_cpu_cycle(memory);
        }
    }

    /// `extcl_ppu_*_556`: the MMC3's A12 counter while it is in use.
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.chip() == Chip::Mmc3 {
            self.mmc3.on_ppu_address_bus(memory, address, clock);
        }
    }
}
