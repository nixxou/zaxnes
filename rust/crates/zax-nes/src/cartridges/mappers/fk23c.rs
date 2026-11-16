//! iNES mapper 176, the 外星 (Wàixīng) / BMC **FK23C** and its kin (lot MAPPERS-N): an MMC3 with
//! an extended register set — four more bank registers in "extended MMC3 mode", PRG/CHR outer
//! banks at `$5000-$5003`, NROM-style PRG modes, a CNROM-style CHR mode, CHR-RAM selectable over
//! CHR-ROM, and up to 32 KiB of banked WRAM shown at `$4000-$7FFF`. It carries the large
//! Chinese multicarts and the Waixing ports that Mesen2's database files under 176.
//!
//! Read from Mesen2 `Core/NES/Mappers/Waixing/Fk23C.h` (read, not copied); the function names
//! below are that file's. Not taken: its soft-reset clear of the PRG base (this build's boards do
//! not see RESET unless a page documents it; FIDELITY-LEADS records it).

use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Mapper 176.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fk23c {
    banks: BankWindows,
    prg_mode: u8,
    outer_chr_size: bool,
    select_chr_ram: bool,
    mmc3_chr_mode: bool,
    cnrom_chr_mode: bool,
    prg_base: u16,
    chr_base: u8,
    extended: bool,
    wram_bank: u8,
    ram_in_first_chr_bank: bool,
    single_screen: bool,
    fk23_registers: bool,
    wram_config: bool,
    wram_enabled: bool,
    wram_protected: bool,
    invert_prg: bool,
    invert_chr: bool,
    current: u8,
    irq_reload_value: u8,
    irq_counter: u8,
    irq_reload: bool,
    irq_enabled: bool,
    irq_delay: u8,
    irq_pending: bool,
    mirroring: u8,
    cnrom_chr: u8,
    regs: [u8; 12],
    a12: A12Watcher,
    /// Derived: the CHR page of each 1 KiB window and which windows read CHR-RAM.
    chr_pages: [i32; 8],
    chr_ram_windows: u8,
    has_chr_rom: bool,
    /// Whether the cartridge carries CHR-RAM beside its CHR-ROM: without it the `$5xx0` CHR-RAM
    /// bit selects nothing (Mesen2 `Fk23C.h:160`, "_selectChrRam && _chrRamSize > 0").
    has_chr_ram: bool,
    /// "Subtype 1, 1024 KiB PRG-ROM, 1024 KiB CHR-ROM: boot in second 512 KiB of PRG-ROM."
    boot_high: bool,
    /// "Subtype 2, 16384 KiB PRG-ROM, no CHR-ROM: … MMC3 registers $46 and $47 swapped."
    swap_46_47: bool,
}

impl Fk23c {
    /// `GetChrRamSize() { return 0x40000; }` and `GetWorkRamSize() { return 0x8000; }`, "only used
    /// for iNES 1.0 files w/ no DB entry".
    pub const INES1_CHR_RAM: usize = 0x40000;
    pub const INES1_WORK_RAM: usize = 0x8000;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let prg = memory.rom().prg_rom().len();
        let chr = memory.rom().chr_rom().len();
        Self {
            banks: windows_for(memory),
            prg_mode: 0,
            outer_chr_size: false,
            select_chr_ram: false,
            mmc3_chr_mode: true,
            cnrom_chr_mode: false,
            prg_base: 0,
            chr_base: 0,
            extended: false,
            wram_bank: 0,
            ram_in_first_chr_bank: false,
            single_screen: false,
            fk23_registers: false,
            wram_config: false,
            wram_enabled: false,
            wram_protected: false,
            invert_prg: false,
            invert_chr: false,
            current: 0,
            irq_reload_value: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_delay: 0,
            irq_pending: false,
            mirroring: 0,
            cnrom_chr: 0,
            regs: [0; 12],
            a12: A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS),
            chr_pages: [0; 8],
            chr_ram_windows: 0,
            has_chr_rom: chr != 0,
            has_chr_ram: !memory.chr_ram().is_empty(),
            boot_high: prg == 1024 * 1024 && prg == chr,
            swap_46_47: prg == 16384 * 1024 && chr == 0,
        }
    }

    /// `InitMapper`.
    fn power_on(&mut self) {
        self.prg_mode = 0;
        self.outer_chr_size = false;
        self.select_chr_ram = false;
        self.mmc3_chr_mode = true;
        self.prg_base = if self.boot_high { 0x20 } else { 0 };
        self.chr_base = 0;
        self.extended = false;
        self.cnrom_chr_mode = false;
        self.wram_bank = 0;
        self.ram_in_first_chr_bank = false;
        self.single_screen = false;
        self.wram_config = false;
        self.fk23_registers = false;
        self.wram_enabled = false;
        self.wram_protected = false;
        self.current = 0;
        self.cnrom_chr = 0;
        self.regs = [0, 2, 4, 5, 6, 7, 0, 1, 0xFE, 0xFF, 0xFF, 0xFF];
        self.invert_prg = false;
        self.invert_chr = false;
        self.mirroring = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_reload = false;
        self.irq_reload_value = 0;
        self.irq_delay = 0;
        self.irq_pending = false;
        self.a12.reset();
    }

    /// `UpdatePrg`.
    fn update_prg(&mut self) {
        let r = |i: usize| i32::from(self.regs[i]);
        let base = i32::from(self.prg_base);
        let swap = if self.invert_prg { 2 } else { 0 };
        match self.prg_mode {
            0..=2 => {
                let (pages, outer) = if self.extended {
                    ([r(6), r(7), r(8), r(9)], base << 1)
                } else {
                    let inner = 0x3F >> self.prg_mode;
                    (
                        [r(6) & inner, r(7) & inner, 0xFE & inner, 0xFF & inner],
                        (base << 1) & !inner,
                    )
                };
                self.banks.map_prg_8k(swap, pages[0] | outer);
                self.banks.map_prg_8k(1, pages[1] | outer);
                self.banks.map_prg_8k(2 ^ swap, pages[2] | outer);
                self.banks.map_prg_8k(3, pages[3] | outer);
            }
            3 => {
                let page = base << 1;
                self.banks.map_prg_8k(0, page);
                self.banks.map_prg_8k(1, page + 1);
                self.banks.map_prg_8k(2, page);
                self.banks.map_prg_8k(3, page + 1);
            }
            4 => self.banks.map_prg_32k((base & 0xFFE) >> 1),
            _ => {}
        }
    }

    /// `UpdateChr` and `SelectChrPage`.
    fn update_chr(&mut self) {
        let r = |i: usize| i32::from(self.regs[i]);
        let base = i32::from(self.chr_base);
        let mut pages = [0i32; 8];
        if self.mmc3_chr_mode {
            let swap = if self.invert_chr { 4 } else { 0 };
            let values = if self.extended {
                let outer = base << 3;
                [r(0), r(10), r(1), r(11), r(2), r(3), r(4), r(5)].map(|page| page | outer)
            } else {
                let inner = if self.outer_chr_size { 0x7F } else { 0xFF };
                let outer = (base << 3) & !inner;
                [
                    r(0) & 0xFE,
                    r(0) | 0x01,
                    r(1) & 0xFE,
                    r(1) | 0x01,
                    r(2),
                    r(3),
                    r(4),
                    r(5),
                ]
                .map(|page| (page & inner) | outer)
            };
            for (slot, page) in values.iter().enumerate() {
                pages[slot ^ swap] = *page;
            }
        } else {
            let inner = if self.cnrom_chr_mode {
                if self.outer_chr_size {
                    1
                } else {
                    3
                }
            } else {
                0
            };
            let bank = (i32::from(self.cnrom_chr) & inner) | base;
            pages = std::array::from_fn(|slot| (bank << 3) + i32::try_from(slot).unwrap_or(0));
        }
        self.chr_ram_windows = 0;
        for (slot, page) in pages.iter().enumerate() {
            let ram = self.has_chr_rom
                && ((self.select_chr_ram && self.has_chr_ram)
                    || (self.wram_config && self.ram_in_first_chr_bank && *page <= 7));
            if ram {
                self.chr_ram_windows |= 1 << slot;
            }
            self.banks.map_chr_1k(slot, *page);
        }
        self.chr_pages = pages;
    }

    /// `UpdateState`, the mirroring half needing the memory.
    fn update(&mut self, memory: &mut CartridgeMemory) {
        let mask = if self.single_screen { 0x03 } else { 0x01 };
        memory.set_mirroring(match self.mirroring & mask {
            0 => NametableMirroring::Vertical,
            1 => NametableMirroring::Horizontal,
            2 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        });
        self.refresh();
    }

    fn refresh(&mut self) {
        self.update_prg();
        self.update_chr();
    }

    /// The offset in PRG-RAM of a CPU address of `$4000-$5FFF` in WRAM-config mode: the bank after
    /// `$6000`'s, "nextBank = (_wramBankSelect + 1) & 0x03".
    fn low_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> Option<usize> {
        let ram = memory.prg_ram().len();
        if !self.wram_config || ram == 0 {
            return None;
        }
        let bank = usize::from((self.wram_bank + 1) & 0x03);
        Some((bank * 0x2000 + usize::from(address & 0x1FFF)) % ram)
    }

    fn chr_ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        let slot = usize::from(address >> 10) & 0x07;
        let page = usize::try_from(self.chr_pages[slot]).unwrap_or(0);
        let len = memory.chr_ram().len().max(1);
        (page * 0x400 + usize::from(address & 0x3FF)) % len
    }

    /// A write to `$5000-$5FFF` while the FK23C registers are reachable.
    fn write_outer(&mut self, address: u16, value: u8) {
        if address & 0x5010 != 0x5010 {
            return;
        }
        match address & 0x03 {
            0 => {
                self.prg_mode = value & 0x07;
                self.outer_chr_size = value & 0x10 != 0;
                self.select_chr_ram = value & 0x20 != 0;
                self.mmc3_chr_mode = value & 0x40 == 0;
                self.prg_base = (self.prg_base & !0x180)
                    | ((u16::from(value) & 0x80) << 1)
                    | ((u16::from(value) & 0x08) << 4);
            }
            1 => self.prg_base = (self.prg_base & !0x7F) | (u16::from(value) & 0x7F),
            2 => {
                self.prg_base = (self.prg_base & !0x200) | ((u16::from(value) & 0x40) << 3);
                self.chr_base = value;
                self.cnrom_chr = 0;
            }
            _ => {
                self.extended = value & 0x02 != 0;
                self.cnrom_chr_mode = value & 0x44 != 0;
            }
        }
    }
}

impl Mapper for Fk23c {
    banked!();

    fn mapper_number(&self) -> i32 {
        176
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        for flag in [
            self.outer_chr_size,
            self.select_chr_ram,
            self.mmc3_chr_mode,
            self.cnrom_chr_mode,
            self.extended,
            self.ram_in_first_chr_bank,
            self.single_screen,
            self.fk23_registers,
            self.wram_config,
            self.wram_enabled,
            self.wram_protected,
            self.invert_prg,
            self.invert_chr,
            self.irq_reload,
            self.irq_enabled,
            self.irq_pending,
        ] {
            writer.write_bool(flag);
        }
        writer.write_u8(self.prg_mode);
        writer.write_u16(self.prg_base);
        writer.write_u8(self.chr_base);
        writer.write_u8(self.wram_bank);
        writer.write_u8(self.current);
        writer.write_u8(self.irq_reload_value);
        writer.write_u8(self.irq_counter);
        writer.write_u8(self.irq_delay);
        writer.write_u8(self.mirroring);
        writer.write_u8(self.cnrom_chr);
        writer.write_raw(&self.regs);
        self.a12.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let mut flags = [false; 16];
        for flag in &mut flags {
            *flag = reader.read_bool()?;
        }
        [
            self.outer_chr_size,
            self.select_chr_ram,
            self.mmc3_chr_mode,
            self.cnrom_chr_mode,
            self.extended,
            self.ram_in_first_chr_bank,
            self.single_screen,
            self.fk23_registers,
            self.wram_config,
            self.wram_enabled,
            self.wram_protected,
            self.invert_prg,
            self.invert_chr,
            self.irq_reload,
            self.irq_enabled,
            self.irq_pending,
        ] = flags;
        self.prg_mode = reader.read_u8()?;
        self.prg_base = reader.read_u16()?;
        self.chr_base = reader.read_u8()?;
        self.wram_bank = reader.read_u8()? & 0x03;
        self.current = reader.read_u8()?;
        self.irq_reload_value = reader.read_u8()?;
        self.irq_counter = reader.read_u8()?;
        self.irq_delay = reader.read_u8()?;
        self.mirroring = reader.read_u8()?;
        self.cnrom_chr = reader.read_u8()?;
        reader.read_into(&mut self.regs)?;
        self.a12.load_state(reader)?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.power_on();
        self.update(memory);
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `UpdateState`: with the WRAM config on, `$6000` is always RAM; otherwise `$A001` bit 7.
    fn prg_ram_enabled(&self) -> bool {
        self.wram_config || self.wram_enabled
    }

    fn prg_ram_writable(&self) -> bool {
        self.wram_config || (self.wram_enabled && !self.wram_protected)
    }

    /// `SetCpuMemoryMapping(0x6000, 0x7FFF, _wramBankSelect, …)` in WRAM-config mode, bank 0
    /// otherwise.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        let len = memory.prg_ram().len();
        let bank = if self.wram_config {
            usize::from(self.wram_bank)
        } else {
            0
        };
        if len == 0 {
            (0, 0)
        } else {
            ((bank * 0x2000) % len, len)
        }
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !(0x4020..=0x5FFF).contains(&address) {
            return false;
        }
        if address >= 0x5000 && (self.fk23_registers || !self.wram_config) {
            self.write_outer(address, value);
            self.update(memory);
            return true;
        }
        // "FK23C Registers disabled, $5000-$5FFF maps to the second 4 KiB of the 8 KiB WRAM
        // bank", and `$4000-$4FFF` is the first half of the same bank in WRAM-config mode.
        if let Some(offset) = self.low_ram_offset(memory, address) {
            memory.prg_ram_mut()[offset] = value;
            return true;
        }
        address >= 0x5000
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, mut value: u8) {
        if address < 0x8000 {
            return;
        }
        if self.cnrom_chr_mode && (address <= 0x9FFF || address >= 0xC000) {
            self.cnrom_chr = value & 0x03;
        }
        match address & 0xE001 {
            0x8000 => {
                if self.swap_46_47 && (value == 0x46 || value == 0x47) {
                    value ^= 1;
                }
                self.invert_prg = value & 0x40 != 0;
                self.invert_chr = value & 0x80 != 0;
                self.current = value & 0x0F;
            }
            0x8001 => {
                let index = usize::from(self.current & if self.extended { 0x0F } else { 0x07 });
                if index < 12 {
                    self.regs[index] = value;
                }
            }
            0xA000 => self.mirroring = value & 0x03,
            0xA001 => {
                if value & 0x20 == 0 {
                    value &= 0xC0;
                }
                self.wram_bank = value & 0x03;
                self.ram_in_first_chr_bank = value & 0x04 != 0;
                self.single_screen = value & 0x08 != 0;
                self.wram_config = value & 0x20 != 0;
                self.fk23_registers = value & 0x40 != 0;
                self.wram_protected = value & 0x40 != 0;
                self.wram_enabled = value & 0x80 != 0;
            }
            0xC000 => self.irq_reload_value = value,
            0xC001 => {
                self.irq_counter = 0;
                self.irq_reload = true;
            }
            0xE000 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            _ => self.irq_enabled = true,
        }
        self.update(memory);
    }

    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if (0x4020..=0x5FFF).contains(&address) {
            return self
                .low_ram_offset(memory, address)
                .map(|offset| memory.prg_ram()[offset]);
        }
        self.banks
            .read_prg(address, memory.rom().prg_rom(), memory.prg_ram())
    }

    /// `ProcessCpuClock`: the line rises two cycles after the counter reached zero.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if self.irq_delay > 0 {
            self.irq_delay -= 1;
            if self.irq_delay == 0 {
                self.irq_pending = true;
            }
        }
    }

    /// `NotifyVramAddressChange`, the MMC3 counter on Mesen2's filtered A12 rise.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if !self.a12.rose(address, clock) {
            return;
        }
        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = self.irq_reload_value;
        } else {
            self.irq_counter -= 1;
        }
        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_delay = 2;
        }
        self.irq_reload = false;
    }

    fn ppu_irq_horizon(&self) -> Option<i64> {
        if self.irq_enabled || self.irq_delay != 0 {
            None
        } else {
            Some(i64::MAX)
        }
    }

    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if self.chr_ram_windows != 0 {
            return None;
        }
        let rom = memory.rom().chr_rom();
        Some(ChrView::new(
            if rom.is_empty() {
                memory.chr_ram()
            } else {
                rom
            },
            &self.banks,
        ))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        let window = 1u8 << ((address >> 10) & 0x07);
        if self.chr_ram_windows & window != 0 {
            return memory
                .chr_ram()
                .get(self.chr_ram_offset(memory, address))
                .copied()
                .unwrap_or(0);
        }
        let rom = memory.rom().chr_rom();
        self.banks.read_chr(
            address,
            if rom.is_empty() {
                memory.chr_ram()
            } else {
                rom
            },
        )
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let window = 1u8 << ((address >> 10) & 0x07);
        if self.chr_ram_windows & window != 0 {
            let offset = self.chr_ram_offset(memory, address);
            if let Some(byte) = memory.chr_ram_mut().get_mut(offset) {
                *byte = value;
            }
        } else if !self.has_chr_rom {
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
    }
}
