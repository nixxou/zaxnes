//! Lot MAPPERS-P: multicarts whose ASIC switches between two mapper chips at run time
//! (`docs/progress/MAPPERS-P.md`).
//!
//! Read from puNES 0.111 (`mapper_556.c`; read, not copied). The board holds each chip whole
//! ([`Mmc3`], [`Vrc2Vrc4`]) and routes the CPU writes, the CPU-cycle and PPU-bus hooks and the
//! IRQ line to the chip its mode register selects; the chip's windows are then narrowed to the
//! board's outer bank with [`BankWindows::remap_prg_banks`]/[`BankWindows::remap_chr_banks`],
//! which is puNES's `(base & ~mask) | (value & mask)` applied after the chip's own swap. An
//! [`super::IrqBoard`] variant, boxed, because both chips' IRQs are that enum's business.
use super::{Mapper, Mmc3, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The boards of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipSwitchKind {
    /// 556, `mapper_556.c` (JY-215 relative): four outer registers written in turn at
    /// `$5000-$5FFF` (until `reg[3]` D7 locks them), `reg[2]` D7 selecting a VRC4 (A0 on CPU
    /// A0|A2, A1 on A1|A3, A2 and A3 swapped above A11) instead of the MMC3.
    M556,
}

impl ChipSwitchKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            556 => Self::M556,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M556 => 556,
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
            vrc: Vrc2Vrc4::new_vrc4_or_lines(memory, number, [0, 1], [2, 3]),
        };
        board.apply();
        board
    }

    /// `map_init_556`: `reg[2] = $0F`, the CHR mask fully open.
    const fn power_on_regs(kind: ChipSwitchKind) -> [u8; 4] {
        match kind {
            ChipSwitchKind::M556 => [0, 0, 0x0F, 0],
        }
    }

    /// Whether the VRC4 is the chip in use (`reg[2]` D7 on 556).
    const fn vrc_active(&self) -> bool {
        match self.kind {
            ChipSwitchKind::M556 => self.regs[2] & 0x80 != 0,
        }
    }

    /// The chip in use rebuilds its windows, then the board narrows them (`prg_base`,
    /// `prg_mask`, `chr_base`, `chr_mask` of `mapper_556.c`).
    fn apply(&mut self) {
        let r = self.regs.map(i32::from);
        let (prg_base, prg_mask, chr_base, chr_mask) = match self.kind {
            ChipSwitchKind::M556 => (
                ((r[3] & 0x40) << 2) | r[1],
                !r[3] & 0x3F,
                ((r[3] & 0x40) << 6) | ((r[2] & 0xF0) << 4) | r[0],
                0xFF >> (!r[2] & 0x0F),
            ),
        };
        let banks = if self.vrc_active() {
            self.vrc.refresh_banks();
            self.vrc.banks_mut()
        } else {
            self.mmc3.refresh_banks();
            self.mmc3.banks_mut()
        };
        if let Some(b) = banks {
            b.remap_prg_banks(prg_mask, prg_base & !prg_mask);
            b.remap_chr_banks(chr_mask, chr_base & !chr_mask);
        }
    }

    /// `extcl_cpu_wr_mem_556`: the four registers in turn, unless `reg[3]` D7 locks them.
    fn write_low(&mut self, address: u16, value: u8) -> bool {
        if !(0x5000..=0x5FFF).contains(&address) {
            return false;
        }
        if self.regs[3] & 0x80 == 0 {
            self.regs[usize::from(self.index)] = value;
            self.index = (self.index + 1) & 0x03;
        }
        self.apply();
        true
    }
}

impl Mapper for ChipSwitch {
    fn banks(&self) -> Option<&BankWindows> {
        if self.vrc_active() {
            self.vrc.banks()
        } else {
            self.mmc3.banks()
        }
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        if self.vrc_active() {
            self.vrc.banks_mut()
        } else {
            self.mmc3.banks_mut()
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
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.regs)?;
        self.index = reader.read_u8()? & 0x03;
        self.mmc3.load_registers(reader)?;
        self.vrc.load_registers(reader)?;
        self.apply();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.regs = Self::power_on_regs(self.kind);
        self.index = 0;
        self.mmc3.reset(memory);
        self.vrc.reset(memory);
        self.apply();
    }

    fn irq_asserted(&self) -> bool {
        if self.vrc_active() {
            self.vrc.irq_asserted()
        } else {
            self.mmc3.irq_asserted()
        }
    }

    fn prg_ram_enabled(&self) -> bool {
        self.vrc_active() || self.mmc3.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.vrc_active() || self.mmc3.prg_ram_writable()
    }

    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        address < 0x6000 && self.write_low(address, value)
    }

    /// `extcl_cpu_wr_mem_556`: the chip in use; the VRC4 sees A2 and A3 swapped above A11.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        if self.vrc_active() {
            let address = if address & 0x0800 != 0 {
                (address & 0xFFF3) | ((address & 0x0004) << 1) | ((address & 0x0008) >> 1)
            } else {
                address
            };
            self.vrc.write_register(memory, address, value);
        } else {
            self.mmc3.write_register(memory, address, value);
        }
        self.apply();
    }

    /// `extcl_cpu_every_cycle_556`: the VRC4's counter while it is in use.
    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        if self.vrc_active() {
            self.vrc.on_cpu_cycle(memory);
        }
    }

    /// `extcl_ppu_*_556`: the MMC3's A12 counter while it is in use.
    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if !self.vrc_active() {
            self.mmc3.on_ppu_address_bus(memory, address, clock);
        }
    }
}
