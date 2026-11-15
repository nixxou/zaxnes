//! Lot MAPPERS-P: VRC2/VRC4 boards puNES runs and Zaxnes did not (`docs/progress/MAPPERS-P.md`).
//!
//! The shape of lot MAPPERS-N's `Ax5705` (mapper 530): a [`Vrc2Vrc4`] wired as the board wires
//! it and left untouched, the board narrowing its windows to an outer bank after every change
//! ([`BankWindows::remap_prg_banks`]/[`BankWindows::remap_chr_banks`]) or replacing its IRQ.
//! puNES 0.111 `mapper_308.c`, `mapper_384.c`, `mapper_447.c`, `mapper_524.c` (read, not copied);
//! `init_VRC2and4(VRC24_VRC2, 0x01, 0x02, …)` is the VRC2b wiring, `(VRC24_VRC4, 0x04, 0x08, …)`
//! the VRC4e. The chip maps its fixed banks to the image's last two, so the `& mask` of the
//! narrowing folds them into the outer bank's last two, as puNES's `value & mask` does.
use super::{Mapper, Vrc2Vrc4};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// The boards of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VrcPKind {
    /// 308, `mapper_308.c` (TH2131-1, *Batman* pirate): a VRC2b with a CPU-cycle IRQ of its own
    /// at `$F000-$F003`.
    M308,
    /// 384, `mapper_384.c` (L1A16): a VRC4e with a lockable outer register at `$6800-$6FFF`.
    M384,
    /// 447, `mapper_447.c` (KL-06): a VRC4e with an address-latched outer register at `$6000`
    /// and an NROM mode.
    M447,
    /// 524, `mapper_524.c` (BTL-900218, *Lord of King* pirate): a VRC2b with a CPU-cycle IRQ of
    /// its own at `$F008`/`$F00C`.
    M524,
}

impl VrcPKind {
    /// The kind of a number, or `None`.
    #[must_use]
    pub const fn of(mapper: i32) -> Option<Self> {
        Some(match mapper {
            308 => Self::M308,
            384 => Self::M384,
            447 => Self::M447,
            524 => Self::M524,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M308 => 308,
            Self::M384 => 384,
            Self::M447 => 447,
            Self::M524 => 524,
        }
    }
}

/// A VRC2/VRC4 board of this lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VrcP {
    vrc: Vrc2Vrc4,
    kind: VrcPKind,
    /// 384's and 447's outer register.
    outer: u8,
    /// 308's and 524's IRQ: enabled, the counter(s), pending.
    irq_enabled: bool,
    irq_low: u16,
    irq_high: u16,
    irq_pending: bool,
}

impl VrcP {
    pub(crate) fn new(memory: &CartridgeMemory, kind: VrcPKind) -> Self {
        let number = kind.number();
        let vrc = match kind {
            VrcPKind::M308 | VrcPKind::M524 => Vrc2Vrc4::new_vrc2b(memory, number),
            VrcPKind::M384 | VrcPKind::M447 => Vrc2Vrc4::new_vrc4e(memory, number),
        };
        Self {
            vrc,
            kind,
            outer: 0,
            irq_enabled: false,
            irq_low: 0,
            irq_high: 0,
            irq_pending: false,
        }
    }

    /// Narrows the chip's windows to the outer bank (384, 447), after the chip rebuilt them.
    fn apply(&mut self) {
        self.vrc.refresh_banks();
        let r = i32::from(self.outer);
        let (reg0, reg1) = self.vrc.prg_registers();
        let (reg0, reg1) = (i32::from(reg0), i32::from(reg1));
        let Some(b) = self.vrc.banks_mut() else {
            return;
        };
        match self.kind {
            // `prg_swap_vrc2and4_384`/`_447`: 16 KiB `outer << 4 | (v & $0F)` in 8 KiB units,
            // CHR `outer << 7 | (v & $7F)`.
            VrcPKind::M384 => {
                b.remap_prg_banks(0x0F, r << 4);
                b.remap_chr_banks(0x7F, r << 7);
            }
            VrcPKind::M447 => {
                // `prg_fix_vrc2and4_447`: D2 the NROM mode, the chip's two switchable registers
                // repeated, D1 clear making it NROM-256 (A14 from the window).
                if r & 0x04 != 0 {
                    let a14 = !r & 0x02;
                    b.map_prg_8k(0, (r << 4) | ((reg0 & !a14) & 0x0F));
                    b.map_prg_8k(1, (r << 4) | ((reg1 & !a14) & 0x0F));
                    b.map_prg_8k(2, (r << 4) | ((reg0 | a14) & 0x0F));
                    b.map_prg_8k(3, (r << 4) | ((reg1 | a14) & 0x0F));
                } else {
                    b.remap_prg_banks(0x0F, r << 4);
                }
                b.remap_chr_banks(0x7F, r << 7);
            }
            VrcPKind::M308 | VrcPKind::M524 => {}
        }
    }
}

impl Mapper for VrcP {
    fn banks(&self) -> Option<&BankWindows> {
        self.vrc.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.vrc.banks_mut()
    }

    fn mapper_number(&self) -> i32 {
        self.kind.number()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.vrc.save_registers(writer);
        writer.write_u8(self.outer);
        writer.write_bool(self.irq_enabled);
        writer.write_u16(self.irq_low);
        writer.write_u16(self.irq_high);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.vrc.load_registers(reader)?;
        self.outer = reader.read_u8()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_low = reader.read_u16()?;
        self.irq_high = reader.read_u16()?;
        self.irq_pending = reader.read_bool()?;
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.vrc.reset(memory);
        self.outer = 0;
        self.irq_enabled = false;
        self.irq_low = 0;
        self.irq_high = 0;
        self.irq_pending = false;
        self.apply();
    }

    fn irq_asserted(&self) -> bool {
        match self.kind {
            VrcPKind::M308 | VrcPKind::M524 => self.irq_pending,
            VrcPKind::M384 | VrcPKind::M447 => self.vrc.irq_asserted(),
        }
    }

    /// `extcl_cpu_every_cycle_308`: a 12-bit low counter; at `$800` it decrements the high one,
    /// and the IRQ asserts while the high one is 0 in the lower half. `_524`: a 16-bit counter
    /// whose bit 10 asserts it. The VRC4 boards keep the chip's own.
    fn on_cpu_cycle(&mut self, memory: &mut CartridgeMemory) {
        match self.kind {
            VrcPKind::M308 => {
                if self.irq_enabled {
                    self.irq_low = (self.irq_low + 1) & 0x0FFF;
                    if self.irq_low == 0x800 {
                        self.irq_high = self.irq_high.wrapping_sub(1);
                    }
                    if self.irq_high == 0 && self.irq_low < 0x800 {
                        self.irq_pending = true;
                    }
                }
            }
            VrcPKind::M524 => {
                if self.irq_enabled {
                    self.irq_low = self.irq_low.wrapping_add(1);
                    if self.irq_low & 0x400 != 0 {
                        self.irq_pending = true;
                    }
                }
            }
            VrcPKind::M384 | VrcPKind::M447 => self.vrc.on_cpu_cycle(memory),
        }
    }

    /// 384's `$6800-$6FFF` until its D3 locks it, and 447's `$6000-$7FFF` (the address, until
    /// A0 locks it); puNES writes the RAM first (`wram_wr`), so the RAM takes the byte too.
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        let writable = !memory.prg_ram().is_empty();
        match self.kind {
            VrcPKind::M384 if (0x6800..=0x6FFF).contains(&address) => {
                if self.outer & 0x08 == 0 && writable {
                    self.outer = value;
                }
            }
            VrcPKind::M447 if (0x6000..=0x7FFF).contains(&address) => {
                if self.outer & 0x01 == 0 && writable {
                    self.outer = address.to_le_bytes()[0];
                }
            }
            _ => return false,
        }
        if writable {
            let ram = memory.prg_ram_mut();
            let len = ram.len();
            ram[usize::from(address & 0x1FFF) % len] = value;
        }
        self.apply();
        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }
        match (self.kind, address & 0xF00C, address & 0xF003) {
            // `extcl_cpu_wr_mem_524`: `$F008` enables, `$F00C` disables and acknowledges;
            // `extcl_cpu_wr_mem_308`: `$F001` enables, `$F000` disables and acknowledges,
            // `$F003` D4-D7 the high counter.
            (VrcPKind::M524, 0xF008, _) | (VrcPKind::M308, _, 0xF001) => self.irq_enabled = true,
            (VrcPKind::M524, 0xF00C, _) | (VrcPKind::M308, _, 0xF000) => {
                self.irq_enabled = false;
                self.irq_low = 0;
                self.irq_pending = false;
            }
            (VrcPKind::M308, _, 0xF003) => self.irq_high = u16::from(value >> 4),
            _ => {
                self.vrc.write_register(memory, address, value);
                self.apply();
            }
        }
    }
}
