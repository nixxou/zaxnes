// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// `clock_irq_counter`'s revision A condition is Mesen's `(count > 0 || _irqReload)`
// (`Core/NES/Mappers/Nintendo/MMC3.h`, `NotifyVramAddressChange`); see THIRD-PARTY-NOTICES.md.

use super::{windows_for, Mapper};
use crate::cartridges::{A12Watcher, BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 4, MMC3 and MMC6 (REQ-CORE-100, REQ-CORE-114).
///
/// Eight bank registers behind $8000/$8001, an A12-clocked scanline counter behind
/// $C000-$E001 (nesdev, "MMC3"), PRG-RAM protection at $A001. MMC6 (NES 2.0 submapper 1)
/// has 1 KiB of RAM in two halves with their own enable bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3 {
    banks: BankWindows,
    a12: A12Watcher,
    bank: [u8; 8],
    is_mmc6: bool,
    /// The nametable arrangement is wired and $A000 is ignored: a four-screen header, or
    /// NES 2.0 submapper 2, "MMC3C with hard-wired mirroring" (MR-4-9).
    four_screen: bool,
    /// NES 2.0 submapper 4, the NEC / Sharp MMC3A, whose IRQ counter is **revision A**
    /// (MR-4-9, AMD-061 §8 closing Q-114). Chosen at load; every other mapper-4 image, iNES
    /// 1.0 included, keeps revision B and with it REQ-CORE-114's `mmc3_test_2` test 6.
    irq_revision_a: bool,
    select: u8,
    protect: u8,
    irq_latch: u8,
    irq_counter: i32,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

impl Mmc3 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        let is_mmc6 = header.is_nes20 && header.submapper == 1;
        Self {
            banks: windows_for(memory),
            a12: A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS),
            bank: [0; 8],
            is_mmc6,
            // MR-4-9, nesdev ("NES 2.0 submappers"), mapper 4: "Submapper 2: MMC3C with
            // hard-wired mirroring" — the same condition as a four-screen header, so the
            // $A000 writes are ignored on both.
            four_screen: header.mirroring == NametableMirroring::FourScreen
                || (header.is_nes20 && header.submapper == 2),
            // AMD-061 §8: "le sous-mappeur NES 2.0 d'une image l'emporte sur la révision par
            // défaut". Submapper 4 is "NEC MMC3 (Loading latch with 0 disables IRQ)" (nesdev,
            // "NES 2.0 submappers"), which is the revision A counter.
            irq_revision_a: header.is_nes20 && header.submapper == 4,
            select: 0,
            protect: Self::power_on_protect(is_mmc6),
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    /// The power-on $A001.
    ///
    /// MR-4-5: the MMC3's is not documented, and enabled-and-writable is the only value that
    /// lets a game, or blargg's MMC3 test ROMs which never write $A001, use the RAM out of a
    /// cold start. The MMC6's *is* documented: $8000 powers on at 0, so the RAM is disabled,
    /// so the chip holds $A001 at $00 (MR-4-7) and no half is readable or writable yet.
    const fn power_on_protect(is_mmc6: bool) -> u8 {
        if is_mmc6 {
            0x00
        } else {
            0x80
        }
    }

    /// MR-4-6: the MMC6's own PRG-RAM, "1 KB PRG RAM, mirrored" over $7000-$7FFF. The size is
    /// settled at load by `registry::board_memory_sizes`, so the address mask alone is the
    /// offset and no division is left on the access path.
    pub(crate) const MMC6_PRG_RAM_SIZE: usize = 0x0400;

    fn mmc6_offset(address: u16) -> usize {
        usize::from(address & 0x03FF)
    }

    /// $A001 of an MMC6 is `HhLl xxxx` (nesdev, "MMC6"): bit 4 enables writing and bit 5
    /// reading the $7000-$71FF half, bit 6 writing and bit 7 reading the $7200-$73FF half
    /// (MR-4-1 — we had each pair the other way round).
    fn mmc6_half_readable(&self, address: u16) -> bool {
        self.protect & if address & 0x0200 != 0 { 0x80 } else { 0x20 } != 0
    }

    fn mmc6_half_writable(&self, address: u16) -> bool {
        self.protect & if address & 0x0200 != 0 { 0x40 } else { 0x10 } != 0
    }

    /// Whether either half answers a read, which is what decides between a value and open bus
    /// on this window (MR-4-8).
    const fn mmc6_any_half_readable(&self) -> bool {
        self.protect & 0xA0 != 0
    }

    fn update_banks(&mut self) {
        if self.select & 0x40 != 0 {
            self.banks.map_prg_8k(0, -2);
            self.banks.map_prg_8k(2, i32::from(self.bank[6]));
        } else {
            self.banks.map_prg_8k(0, i32::from(self.bank[6]));
            self.banks.map_prg_8k(2, -2);
        }

        self.banks.map_prg_8k(1, i32::from(self.bank[7]));
        self.banks.map_prg_8k(3, -1);

        let high = if self.select & 0x80 != 0 { 4 } else { 0 };
        let low = 4 - high;
        self.banks.map_chr_2k(high, i32::from(self.bank[0] >> 1));
        self.banks
            .map_chr_2k(high + 2, i32::from(self.bank[1] >> 1));
        for i in 0..4 {
            self.banks.map_chr_1k(low + i, i32::from(self.bank[2 + i]));
        }
    }

    /// Clocks the scanline counter on an A12 rise, and asserts the line when it lands on zero.
    ///
    /// **The two revisions (AMD-061 §8, Q-114).** Revision B — every mapper-4 image but a
    /// declared submapper 4 — asserts whenever the counter is zero after the clock, so a latch
    /// of $00 gives an IRQ on *every* rise; that is what REQ-CORE-114 and `mmc3_test_2` test 6
    /// require and it is unchanged. Revision A also demands that the counter was **moving**:
    /// non-zero before the clock, or a `$C001` reload pending. A latch of $00 then gives exactly
    /// one IRQ — the pending reload lets the first clock through and nothing lets the next one —
    /// which is §8's criterion.
    fn clock_irq_counter(&mut self) {
        let before = self.irq_counter;
        let reloading = self.irq_reload;
        if before == 0 || reloading {
            self.irq_counter = i32::from(self.irq_latch);
            self.irq_reload = false;
        } else {
            self.irq_counter -= 1;
        }

        let moving = !self.irq_revision_a || before > 0 || reloading;
        if moving && self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }
}

impl Mapper for Mmc3 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.bank);
        writer.write_u8(self.select);
        writer.write_u8(self.protect);
        writer.write_u8(self.irq_latch);
        writer.write_i32(self.irq_counter);
        writer.write_bool(self.irq_reload);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        self.a12.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.bank)?;
        self.select = reader.read_u8()?;
        self.protect = reader.read_u8()?;
        self.irq_latch = reader.read_u8()?;
        self.irq_counter = reader.read_i32()?;
        self.irq_reload = reader.read_bool()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.a12.load_state(reader)?;
        // The windows again from the registers, over the PRG-ROM loaded: a state of the same
        // game made with a PRG-ROM of another size (a widescreen hack that added banks, its
        // fixed ones moved to the end) maps the fixed banks of this one.
        self.update_banks();
        Ok(())
    }

    fn chr_view<'m>(
        &self,
        _memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its fetches have side effects or another source: the PPU takes the full path.
        None
    }

    fn chr_peek_view<'m>(
        &self,
        memory: &'m crate::cartridges::CartridgeMemory,
    ) -> Option<crate::cartridges::ChrView<'m>> {
        // Its peek is the default one, through its bank windows: only the fetches clock the
        // IRQ counter.
        let chr_rom = memory.rom().chr_rom();
        let chr = if chr_rom.is_empty() {
            memory.chr_ram()
        } else {
            chr_rom
        };
        self.banks()
            .map(|banks| crate::cartridges::ChrView::new(chr, banks))
    }

    fn mapper_number(&self) -> i32 {
        4
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// The line only rises when the counter reaches 0 with IRQs on, and the counter only
    /// moves on an A12 rise, which come at least `min_rise_interval` dots apart.
    fn ppu_irq_horizon(&self) -> Option<i64> {
        if self.irq_pending || !self.irq_enabled {
            return Some(i64::MAX);
        }

        let clocks_to_zero = if self.irq_counter == 0 || self.irq_reload {
            1 + i64::from(self.irq_latch)
        } else {
            i64::from(self.irq_counter)
        };
        Some((clocks_to_zero - 1) * self.a12.min_rise_interval())
    }

    fn prg_ram_enabled(&self) -> bool {
        if self.is_mmc6 {
            self.select & 0x20 != 0
        } else {
            self.protect & 0x80 != 0
        }
    }

    /// The MMC3's reading of $A001: bit 7 enables the RAM, bit 6 write-protects it. An MMC6
    /// never gets here — it decodes the whole of $6000-$7FFF itself in `intercept_cpu_write`,
    /// half by half, so the generic PRG-RAM path is unreachable on that chip — which is why
    /// bit 6 keeps its MMC3 meaning here and not the MMC6's "enable writing the high half".
    fn prg_ram_writable(&self) -> bool {
        self.prg_ram_enabled() && self.protect & 0x40 == 0
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.bank = [0; 8];
        self.bank[7] = 1;
        self.select = 0;
        self.protect = Self::power_on_protect(self.is_mmc6);
        self.irq_latch = 0;
        self.irq_counter = 0;
        self.irq_reload = false;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.a12.reset();
        self.update_banks();
    }

    fn cpu_peek_override(&self, memory: &CartridgeMemory, address: u16) -> Option<Option<u8>> {
        if !self.is_mmc6 || !(0x6000..=0x7FFF).contains(&address) {
            return None;
        }

        // $6000-$6FFF is not decoded on this board, and MR-4-8, nesdev ("MMC6"): "If neither
        // bank is enabled for reading, the $7000-$7FFF area is open bus." The RAM being
        // disabled through $8000 holds $A001 at zero (MR-4-7), so it is the same case.
        if address < 0x7000 || !self.prg_ram_enabled() || !self.mmc6_any_half_readable() {
            return Some(None);
        }

        // "If only one bank is enabled for reading, the other reads back as zero."
        Some(Some(if self.mmc6_half_readable(address) {
            memory.prg_ram()[Self::mmc6_offset(address)]
        } else {
            0
        }))
    }

    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !self.is_mmc6 || !(0x6000..=0x7FFF).contains(&address) {
            return false;
        }

        if address >= 0x7000 && self.prg_ram_enabled() && self.mmc6_half_writable(address) {
            let offset = Self::mmc6_offset(address);
            memory.prg_ram_mut()[offset] = value;
        }

        true
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match address & 0xE001 {
            0x8000 => {
                self.select = value;
                self.update_banks();
            }
            0x8001 => {
                self.bank[usize::from(self.select & 0x07)] = value;
                self.update_banks();
            }
            0xA000 => {
                if !self.four_screen {
                    memory.set_mirroring(if value & 0x01 != 0 {
                        NametableMirroring::Horizontal
                    } else {
                        NametableMirroring::Vertical
                    });
                }
            }
            // MR-4-7, nesdev ("MMC6"): "When PRG RAM is disabled via $8000, the mapper
            // continuously sets $A001 to $00, and so all writes to $A001 are ignored." The
            // MMC3 proper has no such gate.
            0xA001 => {
                if !self.is_mmc6 || self.select & 0x20 != 0 {
                    self.protect = value;
                }
            }
            0xC000 => self.irq_latch = value,
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
    }

    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, clock: i64) {
        if self.a12.rose(address, clock) {
            self.clock_irq_counter();
        }
    }
}
