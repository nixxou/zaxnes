use super::vrc_irq::Vrc3Irq;
use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper 73, Konami VRC3 (REQ-CORE-100, REQ-CORE-102).
///
/// nesdev ("VRC3"), the whole board: "CPU $6000-$7FFF: optional 8 KiB PRG RAM bank",
/// "CPU $8000-$BFFF: 16 KiB switchable PRG ROM bank", "CPU $C000-$FFFF: 16 KiB PRG ROM bank,
/// fixed to the last bank", and 8 KiB of fixed CHR-RAM. The mirroring is hard-wired — there is
/// no register for it — so `$A000` is an IRQ latch here and not an arrangement register as it is
/// on every other Konami board.
///
/// Registers, each decoded on `address & $F000` alone (the page gives the ranges as
/// `$8000-$8FFF` and so on, i.e. A12-A15 and nothing below):
///
/// | range | what |
/// | --- | --- |
/// | `$8000-$8FFF` | IRQ latch bits 0-3 |
/// | `$9000-$9FFF` | IRQ latch bits 4-7 |
/// | `$A000-$AFFF` | IRQ latch bits 8-11 |
/// | `$B000-$BFFF` | IRQ latch bits 12-15 |
/// | `$C000-$CFFF` | IRQ control, `.... .MEA` |
/// | `$D000-$DFFF` | IRQ acknowledge |
/// | `$F000-$FFFF` | PRG select, bits 0-2 |
///
/// `$E000-$EFFF` is not decoded. The counter is [`Vrc3Irq`], the same type mappers 21-27, 24, 26
/// and 85 use with its width parameter the other way round; see [`super::vrc_irq`] for what the
/// VRC3 shares with them and what it does not.
///
/// The one game is *Salamander* (Konami, 1987).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vrc3 {
    banks: BankWindows,
    prg: u8,
    irq: Vrc3Irq,
}

impl Vrc3 {
    /// "PPU $0000-$1FFF: 8 KiB CHR RAM", the whole of the board's CHR and unbanked. Settled at
    /// load by `registry::board_memory_sizes`, because REQ-CORE-012's default for an image with
    /// no CHR-ROM is 32 KiB and this chip has no bank line to reach the rest of it.
    pub(crate) const CHR_RAM_SIZE: usize = 0x2000;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            prg: 0,
            irq: Vrc3Irq::new(),
        };
        board.update_banks();
        board
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_16k(0, i32::from(self.prg));
        self.banks.map_prg_16k(2, -1);
        self.banks.map_chr_8k(0);
    }
}

impl Mapper for Vrc3 {
    banked!();

    fn mapper_number(&self) -> i32 {
        73
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.prg);
        self.irq.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.prg = reader.read_u8()?;
        self.irq.load_state(reader)?;
        self.update_banks();
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg = 0;
        self.irq.reset();
        self.update_banks();
    }

    fn irq_asserted(&self) -> bool {
        self.irq.asserted()
    }

    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        match address & 0xF000 {
            0x8000 => self.irq.write_latch_nibble(value, 0),
            0x9000 => self.irq.write_latch_nibble(value, 1),
            0xA000 => self.irq.write_latch_nibble(value, 2),
            0xB000 => self.irq.write_latch_nibble(value, 3),
            0xC000 => self.irq.write_control(value),
            0xD000 => self.irq.acknowledge(),
            0xF000 => {
                // "Select 16 KB PRG ROM bank at $8000": three bits, the page's own width.
                self.prg = value & 0x07;
                self.update_banks();
            }
            _ => {}
        }
    }

    /// **`#[inline(never)]`, and §4.1 item 2 of `docs/review/MF-6-inventory.md` is why.** MF-5a
    /// measured what an arm with a body of its own costs the *empty* arm of this dispatch: the
    /// VRC6's inlined audio took the shared prologue from `pushq %rsi / subq $32, %rsp` to
    /// `pushq %rsi / pushq %rdi / subq $40, %rsp`, one more push and pop per CPU cycle for every
    /// board an NROM included. What worked there was the attribute on the board's own
    /// `on_cpu_cycle`, which is where it is here.
    #[inline(never)]
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        self.irq.step_cpu_cycle();
    }
}
