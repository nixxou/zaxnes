use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **82** and NES 2.0 mapper **552**, the Taito X1-017 (MF-6c step 2).
///
/// nesdev ("Taito X1-017", "INES Mapper 082", "NES 2.0 Mapper 552"). The X1-005's bigger
/// brother: the same $7EFx register file at the same place, **5 KiB** of battery RAM instead of
/// 128 bytes, and a CHR A12 inversion the X1-005 has not got.
///
/// | register | what it does |
/// | --- | --- |
/// | `$7EF0`, `$7EF1` | 2 KiB CHR banks; bit 0 of the value is ignored |
/// | `$7EF2`-`$7EF5` | 1 KiB CHR banks |
/// | `$7EF6` | bit 0 nametable arrangement, **bit 1 CHR A12 inversion** |
/// | `$7EF7` | write **$CA** to enable the 2 KiB at $6000-$67FF |
/// | `$7EF8` | write **$69** to enable the 2 KiB at $6800-$6FFF |
/// | `$7EF9` | write **$84** to enable the 1 KiB at $7000-$73FF |
/// | `$7EFA`, `$7EFB`, `$7EFC` | 8 KiB PRG banks at $8000, $A000 and $C000 |
///
/// The CHR A12 inversion is the whole of the mode bit: clear, the two 2 KiB banks are at
/// $0000-$0FFF and the four 1 KiB ones at $1000-$1FFF; set, they swap halves. $E000-$FFFF is the
/// last 8 KiB bank, wired.
///
/// # The two bank orders, and why both exist
///
/// nesdev ("NES 2.0 Mapper 552"): **"NES 2.0 Mapper 552 represents the actual way the mask ROM is
/// connected and is thus the correct bank order, while iNES Mapper 082 represents the bank order
/// as it was understood before January 2020, when the mapper was reverse-engineered."** So the
/// same silicon has two numbers, and the number in the header decides which way the PRG register
/// is read:
///
/// | | bit 5 | bit 4 | bit 3 | bit 2 |
/// | --- | --- | --- | --- | --- |
/// | iNES **082** | PRG A16 | PRG A15 | PRG A14 | PRG A13 |
/// | NES 2.0 **552** | PRG A13 | PRG A14 | PRG A15 | PRG A16 |
///
/// — the same four bits in the opposite order, which is exactly what "the correct bank order"
/// means. 082 is therefore `value >> 2` and 552 is the bit reversal
/// [`TaitoX1017::reversed_bank`] computes. **Both orders are pinned in one test**
/// (`req_core_100_mapper_82_and_552_are_the_same_board_with_the_bank_order_reversed`) so that the
/// pair cannot be got backwards, which is what §5 MF-6c step 2 of the inventory asks for.
///
/// The reversal is carried **two bits further than the page's table**, to A17 and A18, because
/// the page gives the board a 512 KiB PRG capacity and four bits reach 128 KiB. Mesen 2 extends
/// it the same way and by the same pattern (`TaitoX1017.h:98-105`), and the four lines the page
/// does document are identical in the two implementations.
///
/// # The 5 KiB
///
/// Three windows with three magic values, and **the rest of $6000-$7FFF is not decoded**: so, as
/// on the X1-005, the board owns the whole window through [`Mapper::cpu_peek_override`] and
/// [`Mapper::intercept_cpu_write`] rather than through the generic PRG-RAM path. The bytes are
/// `CartridgeMemory`'s PRG-RAM, sized to exactly 5 KiB by `registry::board_memory_sizes`, in the
/// window order $6000 / $6800 / $7000 — which is Mesen's `.sav` layout too
/// (`GetSaveRamSize() = 0x1400` over five 1 KiB pages), so a save file crosses between the two
/// programs. §4.6's Namco 163 shape, with no header RAM in front of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaitoX1017 {
    banks: BankWindows,
    /// The six CHR registers of $7EF0-$7EF5, exactly as written.
    chr: [u8; 6],
    /// The three PRG registers of $7EFA-$7EFC, exactly as written: the bank they mean depends on
    /// the mapper number, so the raw byte is what is kept.
    prg: [u8; 3],
    /// The three RAM permission latches of $7EF7-$7EF9.
    ram_permission: [u8; 3],
    /// $7EF6 bit 1, the CHR A12 inversion.
    chr_inverted: bool,
    /// NES 2.0 mapper 552, decided at load: the corrected bank order. See the struct's note.
    reversed_prg: bool,
}

impl TaitoX1017 {
    /// The chip's RAM: 2 KiB + 2 KiB + 1 KiB, in the order the three windows appear.
    pub(crate) const RAM_SIZE: usize = 0x1400;

    /// The magic value each permission register wants, in register order ($7EF7, $7EF8, $7EF9).
    const RAM_MAGIC: [u8; 3] = [0xCA, 0x69, 0x84];

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Self {
        Self {
            banks: windows_for(memory),
            chr: [0; 6],
            prg: [0; 3],
            ram_permission: [0; 3],
            chr_inverted: false,
            reversed_prg: mapper == 552,
        }
    }

    /// The register `address` selects, or `None` when the address is not a register.
    ///
    /// **$7EF0-$7EFF and nothing else**, where the X1-005 also answers a $7E7x image: that chip's
    /// page says "All registers exist at $7EFx and mirrored at $7E7x because the CPU A7 line is
    /// ignored" and **this chip's page does not**, documenting $7EF0-$7EFF alone. Mesen 2 decodes
    /// the same range for both. Giving this board the mirror as well would be an inference from
    /// the other ASIC's wiring, and AMD-061 §1 makes the page the authority, not the family.
    const fn register_of(address: u16) -> Option<u16> {
        if matches!(address, 0x7EF0..=0x7EFF) {
            Some(address)
        } else {
            None
        }
    }

    /// Which RAM window `address` falls in and where it starts in the 5 KiB, or `None` for the
    /// $7400-$7FFF the board does not decode.
    fn ram_window(address: u16) -> Option<(usize, usize)> {
        match address {
            0x6000..=0x67FF => Some((0, usize::from(address - 0x6000))),
            0x6800..=0x6FFF => Some((1, 0x0800 + usize::from(address - 0x6800))),
            0x7000..=0x73FF => Some((2, 0x1000 + usize::from(address - 0x7000))),
            _ => None,
        }
    }

    /// NES 2.0 mapper 552's bank number: the register's bits 2-7 taken in the opposite order,
    /// bit 5 being PRG A13 where iNES 082 makes bit 2 PRG A13.
    fn reversed_bank(value: u8) -> i32 {
        let value = i32::from(value);
        ((value & 0x20) >> 5)
            | ((value & 0x10) >> 3)
            | ((value & 0x08) >> 1)
            | ((value & 0x04) << 1)
            | ((value & 0x02) << 3)
            | ((value & 0x01) << 5)
    }

    /// The 8 KiB bank one PRG register names, under the order this cartridge's number gives.
    fn prg_bank(&self, value: u8) -> i32 {
        if self.reversed_prg {
            Self::reversed_bank(value)
        } else {
            // iNES 082: "bits 2-6 contain the bank number (PRG A13-A17)".
            i32::from(value >> 2)
        }
    }

    fn update_banks(&mut self) {
        for slot in 0..3 {
            let bank = self.prg_bank(self.prg[slot]);
            self.banks.map_prg_8k(slot, bank);
        }
        self.banks.map_prg_8k(3, -1);

        // "Bit 0 is ignored for 2KB banks": the register is a 2 KiB bank number.
        let two_k = [i32::from(self.chr[0] >> 1), i32::from(self.chr[1] >> 1)];
        // $7EF6 bit 1 is "CHR A12 inversion (determines whether 2KB banks occupy $0000-$0FFF or
        // $1000-$1FFF)", so the mode decides which half each group of windows gets.
        let (two_k_base, one_k_base) = if self.chr_inverted { (4, 0) } else { (0, 4) };
        self.banks.map_chr_2k(two_k_base, two_k[0]);
        self.banks.map_chr_2k(two_k_base + 2, two_k[1]);
        for window in 0..4 {
            self.banks
                .map_chr_1k(one_k_base + window, i32::from(self.chr[2 + window]));
        }
    }
}

impl Mapper for TaitoX1017 {
    banked!();

    fn mapper_number(&self) -> i32 {
        if self.reversed_prg {
            552
        } else {
            82
        }
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.chr);
        writer.write_raw(&self.prg);
        writer.write_raw(&self.ram_permission);
        writer.write_bool(self.chr_inverted);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.chr)?;
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.ram_permission)?;
        self.chr_inverted = reader.read_bool()?;
        Ok(())
    }

    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.chr = [0; 6];
        self.prg = [0; 3];
        self.ram_permission = [0; 3];
        self.chr_inverted = false;
        self.update_banks();
    }

    /// **`false`**, for the reason [`TaitoX1005::prg_ram_enabled`] gives: three windows with
    /// three separate permission latches, and $7400-$7FFF decoded by nothing.
    ///
    /// [`TaitoX1005::prg_ram_enabled`]: super::TaitoX1005
    fn prg_ram_enabled(&self) -> bool {
        false
    }

    /// PRG-ROM above $8000, and each of the three RAM windows while its own latch holds its own
    /// magic value. Open bus everywhere else in $6000-$7FFF: $7400-$7FFF, which no window covers,
    /// and the write-only registers inside it. See [`super::SmallBoard::try_read_prg`].
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        let (window, offset) = Self::ram_window(address)?;
        if self.ram_permission[window] != Self::RAM_MAGIC[window] {
            return None;
        }

        memory.prg_ram().get(offset).copied()
    }

    /// `#[inline(never)]`, as every `SmallBoard` register decode is since MF-6a step 1.
    #[inline(never)]
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if !(0x6000..=0x7FFF).contains(&address) {
            return false;
        }

        if let Some(register) = Self::register_of(address) {
            self.write_x1_register(memory, register, value);
            return true;
        }

        if let Some((window, offset)) = Self::ram_window(address) {
            if self.ram_permission[window] == Self::RAM_MAGIC[window] {
                if let Some(byte) = memory.prg_ram_mut().get_mut(offset) {
                    *byte = value;
                }
            }
        }

        true
    }
}

impl TaitoX1017 {
    /// One register write, `register` already folded onto $7EF0-$7EFF.
    fn write_x1_register(&mut self, memory: &mut CartridgeMemory, register: u16, value: u8) {
        match register {
            0x7EF0..=0x7EF5 => {
                self.chr[usize::from(register - 0x7EF0)] = value;
                self.update_banks();
            }
            0x7EF6 => {
                memory.set_mirroring(if value & 0x01 == 0 {
                    NametableMirroring::Horizontal
                } else {
                    NametableMirroring::Vertical
                });
                self.chr_inverted = value & 0x02 != 0;
                self.update_banks();
            }
            0x7EF7..=0x7EF9 => self.ram_permission[usize::from(register - 0x7EF7)] = value,
            0x7EFA..=0x7EFC => {
                self.prg[usize::from(register - 0x7EFA)] = value;
                self.update_banks();
            }
            _ => {}
        }
    }
}
