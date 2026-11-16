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
    /// 527, `mapper_527.c` (AX-40G, *Fudou Myouou Den* pirate): a VRC2b whose CIRAM page is D7
    /// of CHR register 0 for `$2000`/`$2400` and of register 1 for `$2800`/`$2C00`.
    M527,
    /// 559, `mapper_559.c` (Subor-made *Contra* clone board): a VRC4 indexed by A10/A11, with
    /// `$9C00-$9FFF` holding the `$C000` bank (A2 clear) or four 1 KiB nametable pages (A2 set),
    /// and the high-nibble registers of `$B000-$FFFF` taking the data's high nibble.
    M559,
    /// 362, `mapper_362.c` (830506C, *Contra* / *Gradius* 2-in-1): a VRC4f whose outer PRG and
    /// CHR banks come from D7-D8 of the CHR register of the window the PPU last fetched from;
    /// on an image above 512 KiB a reset toggles to the second game, which has fixed outer banks.
    M362,
    /// 398, `mapper_398.c` (JY-048): a VRC4f; with D7 of the last write's low address byte set,
    /// a 32 KiB PRG and an 8 KiB CHR bank from that byte's D6-D7 and the CHR register of the
    /// window the PPU last fetched from.
    M398,
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
            527 => Self::M527,
            559 => Self::M559,
            362 => Self::M362,
            398 => Self::M398,
            _ => return None,
        })
    }

    const fn number(self) -> i32 {
        match self {
            Self::M308 => 308,
            Self::M384 => 384,
            Self::M447 => 447,
            Self::M524 => 524,
            Self::M527 => 527,
            Self::M559 => 559,
            Self::M362 => 362,
            Self::M398 => 398,
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
    /// 559's four nametable registers (`m559.mir`), saved for that kind only; its `$C000` bank
    /// (`m559.prg`) is [`VrcP::outer`].
    pages: [u8; 4],
    /// 362's and 398's last fetched CHR window (`m362.reg`, `m398.reg[1]`), saved for those two
    /// kinds only; 362's game and 398's address byte are [`VrcP::outer`].
    chr_window: u8,
}

impl VrcP {
    pub(crate) fn new(memory: &CartridgeMemory, kind: VrcPKind) -> Self {
        let number = kind.number();
        let vrc = match kind {
            VrcPKind::M308 | VrcPKind::M524 | VrcPKind::M527 => Vrc2Vrc4::new_vrc2b(memory, number),
            VrcPKind::M384 | VrcPKind::M447 => Vrc2Vrc4::new_vrc4e(memory, number),
            VrcPKind::M559 => Vrc2Vrc4::new_vrc4_lines(memory, number, [10, 11]),
            VrcPKind::M362 | VrcPKind::M398 => Vrc2Vrc4::new_vrc4f(memory, number),
        };
        Self {
            vrc,
            kind,
            outer: Self::outer_power_on(kind),
            irq_enabled: false,
            irq_low: 0,
            irq_high: 0,
            irq_pending: false,
            pages: Self::PAGES_559,
            chr_window: 0,
        }
    }

    /// Whether `$8000-$FFFF` follows the PPU's fetches now, so that the CPU's cached windows
    /// cannot stand for it.
    const fn prg_follows_ppu(&self) -> bool {
        match self.kind {
            VrcPKind::M362 => self.outer == 0,
            VrcPKind::M398 => self.outer & 0x80 != 0,
            _ => false,
        }
    }

    /// `map_init_559`: nametables 0, 0, 1, 1.
    const PAGES_559: [u8; 4] = [0xE0, 0xE0, 0xE1, 0xE1];

    /// `map_init_559`: the `$C000` bank powers on as `$FE`; the other kinds' outer register 0.
    const fn outer_power_on(kind: VrcPKind) -> u8 {
        match kind {
            VrcPKind::M559 => 0xFE,
            // `map_init_398`: `reg[0] = $C0`.
            VrcPKind::M398 => 0xC0,
            _ => 0,
        }
    }

    /// Narrows the chip's windows to the outer bank (384, 447), after the chip rebuilt them.
    fn apply(&mut self) {
        self.vrc.refresh_banks();
        let r = i32::from(self.outer);
        let (reg0, reg1) = self.vrc.prg_registers();
        let (reg0, reg1) = (i32::from(reg0), i32::from(reg1));
        let fetched = self.vrc.chr_register(usize::from(self.chr_window));
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
            // `prg_swap_vrc2and4_559`: every window masked to `$1F`, `$C000` the board's own
            // 8-bit register; `chr_swap_vrc2and4_559` masks the chip's CHR to `$1FF`, its width.
            VrcPKind::M559 => {
                b.remap_prg_banks(0x1F, 0);
                b.map_prg_8k(2, r);
            }
            // `prg_swap_vrc2and4_362`/`chr_swap_vrc2and4_362`: game 0 takes D7-D8 of the fetched
            // window's register as the outer bank, game 1 the fixed `$40`/`$200`.
            VrcPKind::M362 => {
                if r == 0 {
                    b.remap_prg_banks(0x0F, (fetched & 0x180) >> 3);
                    b.remap_chr_banks(0x7F, fetched & 0x180);
                } else {
                    b.remap_prg_banks(0x0F, 0x40);
                    b.remap_chr_banks(0x1FF, 0x200);
                }
            }
            // `prg_swap_vrc2and4_398`/`chr_swap_vrc2and4_398`.
            VrcPKind::M398 => {
                if r & 0x80 != 0 {
                    b.map_prg_32k(((r & 0xC0) >> 5) | ((fetched & 0x04) >> 2));
                    b.map_chr_8k(0x40 | ((r & 0x40) >> 3) | (fetched & 0x07));
                } else {
                    b.remap_prg_banks(0x0F, 0);
                }
            }
            VrcPKind::M308 | VrcPKind::M524 | VrcPKind::M527 => {}
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
        if self.kind == VrcPKind::M559 {
            writer.write_raw(&self.pages);
        }
        if matches!(self.kind, VrcPKind::M362 | VrcPKind::M398) {
            writer.write_u8(self.chr_window);
        }
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.vrc.load_registers(reader)?;
        self.outer = reader.read_u8()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_low = reader.read_u16()?;
        self.irq_high = reader.read_u16()?;
        self.irq_pending = reader.read_bool()?;
        if self.kind == VrcPKind::M559 {
            reader.read_into(&mut self.pages)?;
        }
        if matches!(self.kind, VrcPKind::M362 | VrcPKind::M398) {
            self.chr_window = reader.read_u8()? & 0x07;
        }
        self.apply();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.vrc.reset(memory);
        self.irq_enabled = false;
        self.irq_low = 0;
        self.irq_high = 0;
        self.irq_pending = false;
        self.pages = Self::PAGES_559;
        self.outer = Self::outer_power_on(self.kind);
        self.chr_window = 0;
        self.apply();
    }

    /// `map_init_362` on `RESET`: an image above 512 KiB toggles to the other game.
    fn soft_reset(&mut self, memory: &mut CartridgeMemory) {
        if self.kind == VrcPKind::M362 && memory.rom().prg_rom().len() > 0x8_0000 {
            self.outer ^= 1;
            self.apply();
        }
    }

    /// `extcl_rd_chr_362`/`_398`: the window the PPU fetches from picks the outer banks.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        if matches!(self.kind, VrcPKind::M362 | VrcPKind::M398) && address < 0x2000 {
            let window = address.to_be_bytes()[0] >> 2;
            if window != self.chr_window {
                self.chr_window = window;
                self.apply();
            }
        }
    }

    /// 362's game 0 and 398's latched mode move `$8000-$FFFF` on PPU fetches.
    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        if self.prg_follows_ppu() {
            return [super::NO_ROM_WINDOW; 4];
        }
        self.vrc.banks().map_or([super::NO_ROM_WINDOW; 4], |banks| {
            banks.rom_windows(memory.rom().prg_rom().len())
        })
    }

    fn irq_asserted(&self) -> bool {
        match self.kind {
            VrcPKind::M308 | VrcPKind::M524 => self.irq_pending,
            VrcPKind::M384
            | VrcPKind::M447
            | VrcPKind::M527
            | VrcPKind::M559
            | VrcPKind::M362
            | VrcPKind::M398 => self.vrc.irq_asserted(),
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
            VrcPKind::M384
            | VrcPKind::M447
            | VrcPKind::M527
            | VrcPKind::M559
            | VrcPKind::M362
            | VrcPKind::M398 => self.vrc.on_cpu_cycle(memory),
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

    /// `mirroring_fix_vrc2and4_527`: `chr[0] >> 7` for the first two tables, `chr[1] >> 7` for
    /// the other two.
    fn nametable_page(&self, table: i32) -> i32 {
        if self.kind == VrcPKind::M527 {
            let register = usize::try_from((table >> 1) & 0x01).unwrap_or(0);
            return (self.vrc.chr_register(register) >> 7) & 0x01;
        }
        // `mirroring_fix_vrc2and4_559`: one register per table (CIRAM page from D0).
        if self.kind == VrcPKind::M559 {
            return i32::from(self.pages[usize::try_from(table & 0x03).unwrap_or(0)] & 0x01);
        }
        self.vrc.nametable_page(table)
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
            // `misc_03_vrc2and4_559`: the chip's index 3 at `$9000` (A10 and A11 set).
            (VrcPKind::M559, _, _) if address & 0xFC00 == 0x9C00 => {
                if address & 0x0004 != 0 {
                    self.pages[usize::from(address & 0x03)] = value;
                } else {
                    self.outer = value;
                    self.apply();
                }
            }
            // `extcl_cpu_wr_mem_559`: `$B000-$FFFF` with A10 set take the high nibble.
            (VrcPKind::M559, _, _) if address >= 0xB000 && address & 0x0400 != 0 => {
                self.vrc.write_register(memory, address, value >> 4);
                self.apply();
            }
            _ => {
                // `extcl_cpu_wr_mem_398`: the low address byte of every write.
                if self.kind == VrcPKind::M398 {
                    self.outer = address.to_le_bytes()[0];
                }
                self.vrc.write_register(memory, address, value);
                self.apply();
            }
        }
    }
}
