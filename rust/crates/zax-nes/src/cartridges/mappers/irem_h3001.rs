use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 65, Irem's H3001 (REQ-CORE-100, REQ-CORE-102).
///
/// nesdev ("INES Mapper 065"), the whole board, mask `$F007` — so A3-A11 are not decoded and
/// every register answers over its whole 4 KiB page:
///
/// | range | what |
/// | --- | --- |
/// | `$8000-$8007` | PRG reg 0, 8 KiB, at `$8000` **or** at `$C000` |
/// | `$9000` | `[X... ....]` PRG bank layout |
/// | `$9001` | `[MM.. ....]` mirroring |
/// | `$9003` | `[E... ....]` IRQ enable |
/// | `$9004` | reload the IRQ counter |
/// | `$9005` | **high** 8 bits of the IRQ reload value |
/// | `$9006` | **low** 8 bits of the IRQ reload value |
/// | `$A000-$A007` | PRG reg 1, 8 KiB at `$A000` |
/// | `$B000-$B007` | the eight 1 KiB CHR regs |
///
/// The PRG layout bit is "very similar to VRC4 or MMC3": "%0 = bank at $8000 set by writes to
/// $8000, bank at $C000 always $3E", "%1 = bank at $C000 set by writes to $8000, bank at $8000
/// always $3E", and "bank at $E000 always $3F". `$3E` and `$3F` are the last two 8 KiB banks of
/// the 512 KiB the four bank lines reach, and a board masks the bank number by its own ROM size,
/// so they are **the ROM's last two** whatever its size — `-2` and `-1` through
/// [`BankWindows::map_prg_8k`], which is the same arithmetic ($3E & 31 = 30 on a 256 KiB ROM).
///
/// "On powerup, it appears as though PRG regs are inited to specific values: $8000 = $00,
/// $A000 = $01. Games do rely on this and will crash otherwise."
///
/// # `$C000` is not a register
///
/// The page opens on it: "Disch's older documentation is known to be wrong, incorrectly believing
/// in a changeable bank at $C000. It doesn't exist". **Mesen 2 still has it**
/// (`IremH3001.h:70`, `case 0xC000: SelectPrgPage(2, value)`), which is the older reading; AMD-061
/// §1 puts the page first, so `$C000-$FFFF` decodes to nothing here and the `$C000` window
/// follows `$9000` alone.
///
/// # Two more divergences from Mesen 2, both the same way round
///
/// * **Mirroring.** `$9001` is `[MM.. ....]` with three outcomes — "%00 = Vert", "%10 = Horz",
///   "%01,%11 = 1scA" — so bit 6 decides one-screen first and bit 7 only chooses between the two
///   mirrorings below it. Mesen tests bit 7 alone (`IremH3001.h:57`) and never produces the
///   one-screen case.
/// * **The counter's floor.** "The counter stops at 0 -- it does not wrap and isn't automatically
///   reloaded." Mesen implements that by clearing its enable flag when the IRQ fires
///   (`IremH3001.h:41-48`), which leaves a counter at $0000 to be decremented to $FFFF the next
///   time `$9003` enables it, and 65 536 cycles later to fire again. Here the counter itself
///   stops, so only a `$9004` reload can arm it again — the page's own sentence.
///
/// The games are *Daiku no Gen San 2*, *Kaiketsu Yanchamaru 3* and *Spartan X 2*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IremH3001 {
    banks: BankWindows,
    /// `$B000-$B007`, eight 1 KiB CHR banks "mapped sequentially from $0000".
    chr: [u8; 8],
    /// PRG reg 0, the window `$9000` bit 7 moves between `$8000` and `$C000`.
    prg0: u8,
    /// PRG reg 1, always the `$A000` window.
    prg1: u8,
    /// `$9000` bit 7, `X`: false puts PRG reg 0 at `$8000`, true at `$C000`.
    prg_at_c000: bool,
    /// `$9005` (high) and `$9006` (low). "$9006 and $9005 set the reload value, but do not have
    /// any effect on the actual counter."
    irq_reload: u16,
    /// The 16-bit down counter itself, which `$9004` alone loads.
    irq_counter: u16,
    /// `$9003` bit 7, `E`.
    irq_enabled: bool,
    irq_pending: bool,
}

impl IremH3001 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            chr: [0; 8],
            prg0: 0,
            prg1: 1,
            prg_at_c000: false,
            irq_reload: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: false,
        };
        board.update_banks();
        board
    }

    fn update_banks(&mut self) {
        // The two halves of the layout bit, and the window the other half fixes to $3E.
        if self.prg_at_c000 {
            self.banks.map_prg_8k(0, -2);
            self.banks.map_prg_8k(2, i32::from(self.prg0));
        } else {
            self.banks.map_prg_8k(0, i32::from(self.prg0));
            self.banks.map_prg_8k(2, -2);
        }

        self.banks.map_prg_8k(1, i32::from(self.prg1));
        // "bank at $E000 always $3F".
        self.banks.map_prg_8k(3, -1);
        for slot in 0..8 {
            self.banks.map_chr_1k(slot, i32::from(self.chr[slot]));
        }
    }
}

impl Mapper for IremH3001 {
    banked!();

    fn mapper_number(&self) -> i32 {
        65
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_u8(self.prg0);
        writer.write_u8(self.prg1);
        writer.write_bool(self.prg_at_c000);
        writer.write_u16(self.irq_reload);
        writer.write_u16(self.irq_counter);
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        self.prg0 = reader.read_u8()?;
        self.prg1 = reader.read_u8()?;
        self.prg_at_c000 = reader.read_bool()?;
        self.irq_reload = reader.read_u16()?;
        self.irq_counter = reader.read_u16()?;
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.update_banks();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 8];
        // The power-on values the page says the games rely on.
        self.prg0 = 0;
        self.prg1 = 1;
        self.prg_at_c000 = false;
        self.irq_reload = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.update_banks();
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match address & 0xF000 {
            // "$8000-$8007: PRG Reg 0" and "$A000-$A007: PRG Reg 1": one register each, over
            // all eight addresses the $F007 mask leaves — and so over the whole page.
            0x8000 => self.prg0 = value,
            0xA000 => self.prg1 = value,
            // "$B000-$B007: CHR regs", one per low address.
            0xB000 => self.chr[usize::from(address & 0x07)] = value,
            0x9000 => match address & 0x07 {
                0x00 => self.prg_at_c000 = value & 0x80 != 0,
                0x01 => {
                    // "[MM.. ....] Mirroring: %00 = Vert, %10 = Horz, %01,%11 = 1scA" — bit 6
                    // decides the one-screen case whatever bit 7 says.
                    memory.set_mirroring(if value & 0x40 != 0 {
                        NametableMirroring::SingleScreenA
                    } else if value & 0x80 != 0 {
                        NametableMirroring::Horizontal
                    } else {
                        NametableMirroring::Vertical
                    });
                    return;
                }
                0x03 => {
                    // "[E... ....] IRQ Enable (0=disabled, 1=enabled)", and "any write to $9003
                    // or $9004 will acknowledge the pending IRQ".
                    self.irq_enabled = value & 0x80 != 0;
                    self.irq_pending = false;
                    return;
                }
                0x04 => {
                    // "Any write to $9004 will copy the 16-bit reload value into the counter."
                    self.irq_counter = self.irq_reload;
                    self.irq_pending = false;
                    return;
                }
                // "Note that $9005 is the HIGH bits, not the low bits."
                0x05 => {
                    self.irq_reload = (self.irq_reload & 0x00FF) | (u16::from(value) << 8);
                    return;
                }
                0x06 => {
                    self.irq_reload = (self.irq_reload & 0xFF00) | u16::from(value);
                    return;
                }
                // $9002 and $9007 are not registers.
                _ => return,
            },
            // $C000-$FFFF decodes to nothing: see the type's own note on Disch's $C000 bank.
            _ => return,
        }

        self.update_banks();
    }

    /// `#[inline(never)]`, for the reason [`super::Vrc3::on_cpu_cycle`] gives: §4.1 item 2 of
    /// `docs/review/MF-6-inventory.md`, and MF-5a's measurement of what one arm with a body of
    /// its own costs the *empty* arm of this dispatch.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        // "There's a 16-bit internal down counter which (when enabled), decrements by 1 every
        // CPU cycle. When the counter reaches 0, an IRQ is fired. The counter stops at 0 -- it
        // does not wrap and isn't automatically reloaded." Both halves are this one test: a
        // counter already at zero is a counter that has stopped.
        if !self.irq_enabled || self.irq_counter == 0 {
            return;
        }

        self.irq_counter -= 1;
        if self.irq_counter == 0 {
            self.irq_pending = true;
        }
    }
}
