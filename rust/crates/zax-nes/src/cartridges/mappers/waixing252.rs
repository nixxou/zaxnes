//! iNES mapper 252, the 外星 (Wàixīng) VRC4 clone with 2 KiB of CHR-RAM beside its CHR-ROM
//! (lot MAPPERS-N), on the Chinese translation of *Sangokushi: Chūgen no Hasha*.
//!
//! Two sources, read and not copied, and where they part this board follows each where the
//! other is silent (FIDELITY-LEADS J19):
//!
//! * the register decode and the IRQ are Mesen2's `Waixing/Waixing252.h`: PRG bank 0 at
//!   `$8000-$8FFF`, PRG bank 1 at `$A000-$AFFF`, the last two 8 KiB banks fixed, eight CHR
//!   registers at `$B000-$EFFF` written a nibble at a time with A2 picking the nibble and
//!   A3/A12-A13 the register, and Konami's VRC IRQ at `$F000`/`$F004`/`$F008`/`$F00C`;
//! * the CHR-RAM is puNES's (`mapper_252.c`): a CHR register value whose bits under `mask` equal
//!   `compare` selects the 2 KiB of CHR-RAM instead of CHR-ROM, `mask`/`compare` starting at
//!   `$FE`/`$06` (values 6 and 7, as nesdev's "INES Mapper 252" gives them) and changing when the
//!   game writes CHR through a window whose register holds `$88`, `$C2` or `$C8` — the board's
//!   protection. Mesen2 instead lets the game write into CHR-ROM ("CHR needs to be writeable");
//!   puNES also decodes the VRC4's `$9000` mirroring, which Mesen2 leaves out, and this board
//!   takes it.
use super::vrc_irq::VrcIrq;
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Mapper 252.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waixing252 {
    banks: BankWindows,
    prg: [u8; 2],
    chr: [u8; 8],
    mask: u8,
    compare: u8,
    irq: VrcIrq,
    /// Which 1 KiB windows read the CHR-RAM, one bit each; derived by [`Waixing252::refresh`].
    ram_windows: u8,
    /// Whether the cartridge has CHR-RAM beside CHR-ROM, settled at load.
    has_ram: bool,
}

impl Waixing252 {
    /// 2 KiB beside the CHR-ROM (the cartridge database's entry `8B7EE49B`, CHR-RAM 2 KiB).
    pub const CHR_RAM_SIZE: usize = 0x800;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        Self {
            banks: windows_for(memory),
            prg: [0, 0],
            chr: [0; 8],
            mask: 0xFE,
            compare: 0x06,
            irq: VrcIrq::new(),
            ram_windows: 0,
            has_ram: !memory.chr_ram().is_empty() && !memory.rom().chr_rom().is_empty(),
        }
    }

    fn refresh(&mut self) {
        self.banks.map_prg_8k(0, i32::from(self.prg[0] & 0x1F));
        self.banks.map_prg_8k(1, i32::from(self.prg[1] & 0x1F));
        self.banks.map_prg_8k(2, -2);
        self.banks.map_prg_8k(3, -1);
        let has_ram = self.has_ram;
        self.ram_windows = 0;
        for (slot, value) in self.chr.iter().enumerate() {
            if has_ram && value & self.mask == self.compare {
                self.ram_windows |= 1 << slot;
            }
            self.banks.map_chr_1k(slot, i32::from(*value));
        }
    }

    fn ram_offset(&self, memory: &CartridgeMemory, address: u16) -> usize {
        let slot = usize::from(address >> 10) & 0x07;
        (usize::from(self.chr[slot]) * 0x400 + usize::from(address & 0x03FF))
            % memory.chr_ram().len().max(1)
    }

    const fn reads_ram(&self, address: u16) -> bool {
        self.ram_windows & (1 << ((address >> 10) & 0x07)) != 0
    }
}

impl Mapper for Waixing252 {
    banked!();

    fn mapper_number(&self) -> i32 {
        252
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
        writer.write_u8(self.mask);
        writer.write_u8(self.compare);
        self.irq.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.mask = reader.read_u8()?;
        self.compare = reader.read_u8()?;
        self.irq.load_state(reader)?;
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg = [0, 0];
        self.chr = [0; 8];
        self.mask = 0xFE;
        self.compare = 0x06;
        self.irq.reset();
        self.refresh();
    }

    fn irq_asserted(&self) -> bool {
        self.irq.asserted()
    }

    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        self.irq.step_cpu_cycle();
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address {
            0x8000..=0x8FFF => self.prg[0] = value,
            0x9000..=0x9FFF => memory.set_mirroring(match value & 0x03 {
                0 => NametableMirroring::Vertical,
                1 => NametableMirroring::Horizontal,
                2 => NametableMirroring::SingleScreenA,
                _ => NametableMirroring::SingleScreenB,
            }),
            0xA000..=0xAFFF => self.prg[1] = value,
            0xB000..=0xEFFF => {
                let shift = address & 0x04;
                let slot = usize::from(
                    ((((address - 0xB000) >> 1) & 0x1800) | ((address << 7) & 0x0400)) >> 10,
                );
                let keep = 0xF0u8 >> shift;
                self.chr[slot] = (self.chr[slot] & keep) | ((value & 0x0F) << shift);
            }
            _ => match address & 0xF00C {
                0xF000 => self.irq.write_latch_nibble(value, false),
                0xF004 => self.irq.write_latch_nibble(value, true),
                0xF008 => self.irq.write_control(value),
                0xF00C => self.irq.acknowledge(),
                _ => {}
            },
        }
        self.refresh();
    }

    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        (self.ram_windows == 0 && !memory.rom().chr_rom().is_empty())
            .then(|| ChrView::new(memory.rom().chr_rom(), &self.banks))
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        if self.reads_ram(address) {
            return memory
                .chr_ram()
                .get(self.ram_offset(memory, address))
                .copied()
                .unwrap_or(0);
        }
        let rom = memory.rom().chr_rom();
        if rom.is_empty() {
            return self.banks.read_chr(address, memory.chr_ram());
        }
        self.banks.read_chr(address, rom)
    }

    /// `extcl_wr_chr_252`: a CHR write through a window whose register holds `$88`, `$C2` or
    /// `$C8` moves the CHR-RAM selection, whatever memory the write itself reaches.
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let slot = usize::from(address >> 10) & 0x07;
        let (mask, compare) = match self.chr[slot] {
            0x88 => (0xFC, 0x4C),
            0xC2 => (0xFE, 0x7C),
            0xC8 => (0xFE, 0x04),
            _ => (self.mask, self.compare),
        };
        if self.reads_ram(address) {
            let offset = self.ram_offset(memory, address);
            if let Some(byte) = memory.chr_ram_mut().get_mut(offset) {
                *byte = value;
            }
        } else if memory.rom().chr_rom().is_empty() {
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
        if (mask, compare) != (self.mask, self.compare) {
            self.mask = mask;
            self.compare = compare;
            self.refresh();
        }
    }
}
