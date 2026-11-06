// Portions adapted from Mesen 2 (c) Sour, GPL-3.0
//
// The $C command's four arrangements are read the way `SunsoftFme7.h:103-110` reads them, which
// resolves the nesdev page's "Horizontal (A10)" wording; see `arrangement` below and
// THIRD-PARTY-NOTICES.md.

use super::{windows_for, Mapper};
use crate::apu::Sunsoft5bAudio;
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 69, Sunsoft FME-7 and the 5A / 5B that contain it (REQ-CORE-100's "comportement
/// documenté sur nesdev"; the number is outside REQ-CORE-100's table, which REQ-CORE-102 allows).
///
/// Everything goes through two addresses (nesdev, "Sunsoft FME-7"): a **command register** at
/// $8000-$9FFF, `.... CCCC`, whose four bits name one of sixteen commands, and a **parameter
/// register** at $A000-$BFFF, `PPPP PPPP`, where "Writing here invokes the command with the
/// specified parameter byte".
///
/// | command | effect |
/// | --- | --- |
/// | $0-$7 | 1 KiB CHR bank at PPU $0000-$03FF … $1C00-$1FFF, `BBBB BBBB` |
/// | $8 | the $6000-$7FFF window, `ERbB BBBB` — `E` enables the RAM chip, `R` picks RAM (1) or ROM (0), the rest is the 8 KiB bank |
/// | $9-$B | 8 KiB PRG bank at CPU $8000-$9FFF, $A000-$BFFF, $C000-$DFFF, `..bB BBBB` |
/// | $C | nametable arrangement, `.... ..MM` |
/// | $D | IRQ control, `C... ...T` |
/// | $E, $F | the low and high bytes of the IRQ counter |
///
/// $E000-$FFFF is not switchable: the last 8 KiB bank is wired there. $C000-$DFFF and
/// $E000-$FFFF are the 5B's sound registers, all of it in [`Sunsoft5bAudio`]; the output is the
/// `M` the APU subtracts (REQ-CORE-073, AMD-061 §6.2), which the bus reads through
/// [`Mapper::expansion_audio_output`].
///
/// Registers in `$4020-$5FFF`: **none**. The board decodes $6000-$FFFF only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fme7 {
    banks: BankWindows,
    /// The 5B's YM2149F. Carried by every mapper-69 image: whether a dump is a 5A (no sound) or
    /// a 5B is not in the header, and a 5A game simply never writes $C000-$FFFF, which leaves
    /// every volume register at 0 and the output at 0.
    audio: Sunsoft5bAudio,
    command: u8,
    /// Command $9-$B, the three switchable 8 KiB PRG banks.
    prg: [u8; 3],
    /// Command $8, the $6000-$7FFF window: `ERbB BBBB` exactly as written.
    window_6000: u8,
    /// Whether command $8 put **RAM** in $6000-$7FFF *and* enabled its chip, precomputed from
    /// `window_6000` so that `prg_ram_enabled` — on the CPU's $6000-$7FFF decode — is one load
    /// and one test, like every other board's.
    ram_readable: bool,
    /// First byte of PRG-ROM the $6000-$7FFF window shows when command $8 selected ROM, already
    /// wrapped to the image the way [`BankWindows`] wraps its own bank numbers. Meaningless
    /// while the window shows RAM.
    rom_offset_6000: usize,
    /// Command $D bit 0, `T`: "enables IRQ generation".
    irq_enabled: bool,
    /// Command $D bit 7, `C`: "enables counter decrement".
    irq_counter_enabled: bool,
    irq_counter: u16,
    irq_pending: bool,
}

impl Fme7 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            audio: Sunsoft5bAudio::new(),
            command: 0,
            prg: [0; 3],
            window_6000: 0,
            ram_readable: false,
            rom_offset_6000: 0,
            irq_enabled: false,
            irq_counter_enabled: false,
            irq_counter: 0,
            irq_pending: false,
        };
        board.refresh_window_6000();
        board
    }

    /// Recomputes the two derived halves of command $8.
    ///
    /// nesdev gives the register as `ERbB BBBB`: `E` is the RAM chip's `+CE` and `R` picks
    /// "0=ROM, 1=RAM". So the window answers from RAM only when both are set; with `R` set and
    /// `E` clear the chip is deselected and the window is open bus; with `R` clear it is a plain
    /// 8 KiB PRG-ROM bank, whatever `E` says. The bank number is six bits, "FME-7 supports 6
    /// bits; 5A/5B support only 5 bits" — the sixth is kept, since a 5B image can never set it
    /// meaningfully and an FME-7 one needs it.
    ///
    /// The bank count comes from the window table rather than from the ROM, so that
    /// [`Mapper::load_registers`] — which is handed a reader and no memories — can call this and
    /// bring both derived halves back with the register (REQ-CORE-091).
    fn refresh_window_6000(&mut self) {
        self.ram_readable = self.window_6000 & 0xC0 == 0xC0;
        let banks = self.banks.prg_bank_count_8k();
        let bank = i32::from(self.window_6000 & 0x3F).rem_euclid(banks.max(1));
        self.rom_offset_6000 = usize::try_from(bank).unwrap_or(0) * BankWindows::PRG_WINDOW_SIZE;
    }

    fn update_banks(&mut self) {
        for (slot, bank) in self.prg.iter().enumerate() {
            self.banks.map_prg_8k(slot, i32::from(bank & 0x3F));
        }

        // "$E000-$FFFF" is absent from the page's PRG table: the last bank is wired there.
        self.banks.map_prg_8k(3, -1);
    }

    /// Command $C, `.... ..MM`.
    ///
    /// nesdev names the four values "0: Horizontal (A10), 1: Vertical (A11), 2: 1scA (GND),
    /// 3: 1scB (VCC)", which is the **arrangement** vocabulary of the iNES header — "(A10)"
    /// means CIRAM A10 is driven by PPU A10, the horizontal *arrangement*, which is what every
    /// other board page in this crate calls **vertical mirroring**. Mesen 2 reads the register
    /// the same way (`SunsoftFme7.h:103-110`: 0 → `Vertical`, 1 → `Horizontal`), so the two
    /// sources agree and the translation below is theirs.
    const fn arrangement(value: u8) -> NametableMirroring {
        match value & 0x03 {
            0 => NametableMirroring::Vertical,
            1 => NametableMirroring::Horizontal,
            2 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        }
    }
}

impl Mapper for Fme7 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.command);
        writer.write_raw(&self.prg);
        writer.write_u8(self.window_6000);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_counter_enabled);
        writer.write_u16(self.irq_counter);
        writer.write_bool(self.irq_pending);
        // Written last, so a mapper-69 state from before the sound was added loads with a silent
        // chip — which is the state of a 5A, and of a 5B before its first register write
        // (REQ-CORE-091).
        self.audio.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.command = reader.read_u8()?;
        reader.read_into(&mut self.prg)?;
        self.window_6000 = reader.read_u8()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_counter_enabled = reader.read_bool()?;
        self.irq_counter = reader.read_u16()?;
        self.irq_pending = reader.read_bool()?;
        if reader.remaining() > 0 {
            self.audio.load_state(reader)?;
        } else {
            self.audio.reset();
        }

        self.refresh_window_6000();
        Ok(())
    }

    fn mapper_number(&self) -> i32 {
        69
    }

    /// The header's nametable arrangement is left in place: nothing on the page gives the
    /// power-on value of command $C, and the board's own games set it before they draw.
    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.command = 0;
        self.prg = [0; 3];
        self.window_6000 = 0;
        self.irq_enabled = false;
        self.irq_counter_enabled = false;
        self.irq_counter = 0;
        self.irq_pending = false;
        self.audio.reset();
        self.refresh_window_6000();
        self.update_banks();
        self.banks.map_chr_8k(0);
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `M` of REQ-CORE-073, which the APU subtracts (AMD-061 §6.2, Q-118). One load: the level
    /// is recomputed where the chip's generators move, not here.
    fn expansion_audio_output(&self) -> Option<i32> {
        Some(self.audio.output())
    }

    // No `ppu_irq_horizon`: the counter is clocked by the CPU and nothing the PPU does can move
    // the line, which is the trait's default `Some(i64::MAX)`. Overriding it with the same value
    // would only add an arm to a dispatch `NesBus::catch_up_ppu` consults.

    fn prg_ram_enabled(&self) -> bool {
        self.ram_readable
    }

    /// The six bank bits of command $8 over the PRG-RAM the board carries, which on JSROM is one
    /// 8 KiB chip: the modulo then makes every bank the same chip, as the hardware's missing
    /// address lines do.
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        let bank = usize::from(self.window_6000 & 0x3F) * BankWindows::PRG_WINDOW_SIZE;
        (bank, memory.prg_ram().len())
    }

    /// $6000-$7FFF when it is not the RAM: a PRG-ROM bank, or open bus if command $8 selected the
    /// RAM and then deselected its chip.
    ///
    /// `Cartridge::default_cpu_read` only reaches this for $6000-$7FFF once `prg_ram_enabled`
    /// has said no, so no other board and no $8000-$FFFF read pays for the extra range test:
    /// the fast path for $8000-$FFFF is `Cartridge::read_rom_window`, which this board feeds
    /// through the default `rom_windows`.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if (0x6000..0x8000).contains(&address) {
            if self.window_6000 & 0x40 != 0 {
                return None;
            }

            let offset = self.rom_offset_6000 + usize::from(address - 0x6000);
            return memory.rom().prg_rom().get(offset).copied();
        }

        self.banks
            .read_prg(address, memory.rom().prg_rom(), memory.prg_ram())
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & 0xE000 {
            0x8000 => self.command = value & 0x0F,
            0xA000 => match self.command {
                bank @ 0..=7 => self.banks.map_chr_1k(usize::from(bank), i32::from(value)),
                8 => {
                    self.window_6000 = value;
                    self.refresh_window_6000();
                }
                slot @ 9..=0x0B => {
                    self.prg[usize::from(slot) - 9] = value;
                    self.update_banks();
                }
                0x0C => memory.set_mirroring(Self::arrangement(value)),
                // "All writes to this register acknowledge an active IRQ."
                0x0D => {
                    self.irq_enabled = value & 0x01 != 0;
                    self.irq_counter_enabled = value & 0x80 != 0;
                    self.irq_pending = false;
                }
                // "Writes to the IRQ counter registers directly set the lower or upper eight
                // bits of the counter" — they do not reload a latch, and they do not
                // acknowledge.
                0x0E => self.irq_counter = (self.irq_counter & 0xFF00) | u16::from(value),
                _ => self.irq_counter = (self.irq_counter & 0x00FF) | (u16::from(value) << 8),
            },
            // $C000-$DFFF selects one of the chip's sixteen registers, $E000-$FFFF writes it.
            _ => self.audio.write(address, value),
        }
    }

    /// "When the IRQ counter is decremented from $0000 to $FFFF an IRQ is generated" — the
    /// counter running off M2, one step per CPU cycle, while command $D's `C` bit allows it.
    ///
    /// The line is only pulled if `T` was set *at that moment*: the wrap is not latched into a
    /// flag the enable bit can reveal later, which is how Mesen reads the page too
    /// (`SunsoftFme7.h:59-66`).
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        self.audio.step_cpu_cycle();

        if !self.irq_counter_enabled {
            return;
        }

        self.irq_counter = self.irq_counter.wrapping_sub(1);
        if self.irq_counter == 0xFFFF && self.irq_enabled {
            self.irq_pending = true;
        }
    }
}
