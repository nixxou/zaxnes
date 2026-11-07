use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, Cartridge, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mappers **150 and 243**: Sachen's eight-register ASIC, on the SA-015 / SA-630 and the
/// SA-020A (REQ-CORE-102, MF-UNL-1 step 2).
///
/// Mesen 2 builds one class, `Sachen74LS374N`, for both numbers, and §6.1 of
/// `docs/review/UNL-census.md` files them as one family for that reason. The difference is one
/// register: 243 drives CHR A13 from R2 where 150 has nothing there, and the CHR bits above it
/// move up to make room.
///
/// # The two ports, both under mask $C101
///
/// ```text
/// Register Index ($4100, write) - Mask: $C101     Register Data ($4101, read/write)
/// D~7654 3210                                     D~7654 3210
///   ---------                                       ---------
///   .... .RRR                                       .... .RRR
///         +++- Select register number (Rx)                +++- Register data
/// ```
///
/// Mask $C101 decodes A15, A14, A8 and A0 and nothing else, so **the two ports repeat through
/// the whole of $4100-$7FFF** — `$6100 & $C101` is `$4100`. That is why
/// `registry::board_memory_sizes` gives both numbers `prg_ram: Some(0)`: with no RAM behind
/// $6000-$7FFF, `Cartridge::cpu_write` walks past its PRG-RAM branch and
/// [`Mapper::write_register`] sees the address intact — the shape mappers 47, 37, 140, 184 and
/// the Jaleco JF-13 already use — and `Cartridge::default_cpu_read` falls through to
/// [`Mapper::try_read_prg`], where the data port answers.
///
/// # What each register drives, and the three that drive nothing
///
/// | register | 150 | 243 |
/// | --- | --- | --- |
/// | R0, R1, R3 | — | — |
/// | R2 | — | CHR A13 |
/// | R4 | CHR A15 | CHR A14 |
/// | R5 | PRG A16-A15 | PRG A16-A15 |
/// | R6 | CHR A14-A13 | CHR A16-A15 |
/// | R7 | nametable mirroring | nametable mirroring |
///
/// 150's page says "Registers 0-3 are completely unused" and 243's that R0, R1 and R3 have "no
/// external effect"; both say "All three bits in all eight registers are fully readable and
/// writable". So a write to one of them is **stored and does nothing** — a no-op and not an
/// `unreachable!`, which is what the census asks for by name: the bits come back out of the data
/// port, and a protection routine that writes and reads them back must see what it wrote.
///
/// # R7's four layouts, and the one this core cannot name
///
/// ```text
/// R7: ...  .MM.
///      0: S0-S0-S0-S1 (lower right unique, or vertically-flipped L)
///      1: Horizontal
///      2: Vertical
///      3: Single-screen, page 1
/// ```
///
/// Three of the four are a [`NametableMirroring`]. **Mode 0 is not**: three tables on CIRAM page
/// 0 and the fourth on page 1 is a per-table layout, so the board answers it through
/// [`Mapper::nametable_page`] — §4.5 of `docs/review/MF-6-inventory.md`'s cheap level, the
/// mechanism mappers 95, 118, 158 and 207 already use, and **not** a `read_nametable` body
/// (Q-123's dispatch is untouched by this board).
///
/// **Mode 3 is Q-141.** The page says "Single-screen, page 1" where its own mode-0 row names the
/// two pages `S0` and `S1`, so "page 1" is `S1` and this build selects
/// [`NametableMirroring::SingleScreenB`]. Mesen 2 reads the same line as
/// `MirroringType::ScreenAOnly` (`Sachen74LS374N.h`). The ticket holds both readings; the page
/// wins here because its own notation settles what "page 1" counts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sachen8Reg {
    banks: BankWindows,
    /// The eight three-bit registers. All of them are written and read back, including the
    /// three that drive nothing.
    registers: [u8; 8],
    /// The register the $4100 port last selected.
    index: u8,
    /// 150 or 243, for [`Mapper::mapper_number`].
    mapper_number: i32,
    /// Mapper 243: R2 is CHR A13 and the CHR field above it is one bit wider.
    chr_a13_from_r2: bool,
    /// What [`Mapper::nametable_page`] answers: `FOLLOW_MIRRORING` in three of R7's four modes
    /// and `[0, 0, 0, 1]` in the fourth. Derived state — `load_registers` recomputes it and no
    /// savestate byte carries it (REQ-CORE-091).
    nametable_pages: [i32; 4],
}

impl Sachen8Reg {
    /// The decode of both ports: mask $C101, the index on A0 clear and the data on A0 set.
    const PORT_MASK: u16 = 0xC101;
    const INDEX_PORT: u16 = 0x4100;
    const DATA_PORT: u16 = 0x4101;

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            registers: [0; 8],
            index: 0,
            mapper_number,
            chr_a13_from_r2: mapper_number == 243,
            nametable_pages: [Cartridge::FOLLOW_MIRRORING; 4],
        };
        board.refresh_nametable_pages();
        board.update_banks();
        board
    }

    fn register(&self, index: usize) -> i32 {
        i32::from(self.registers[index])
    }

    fn update_banks(&mut self) {
        // "R5: .... ..PP (PRG A16..A15)", the same two bits on both numbers.
        self.banks.map_prg_32k(self.register(5) & 0x03);

        let chr = if self.chr_a13_from_r2 {
            // 243: "R2: CHR A13", "R4: CHR A14", "R6: CHR A16..A15".
            (self.register(2) & 0x01)
                | ((self.register(4) & 0x01) << 1)
                | ((self.register(6) & 0x03) << 2)
        } else {
            // 150: "R4: .... ...C (CHR A15)", "R6: .... ..BA (CHR A14..A13)".
            ((self.register(4) & 0x01) << 2) | (self.register(6) & 0x03)
        };
        self.banks.map_chr_8k(chr);
    }

    /// `MM`, bits 2-1 of R7: which of the four nametable layouts the board is in. Bit 0 of the
    /// three-bit register is not part of the field.
    const fn mirroring_mode(&self) -> u8 {
        (self.registers[7] >> 1) & 0x03
    }

    /// R7's mode 0 alone, `S0-S0-S0-S1`; every other mode leaves the four pages following
    /// [`Cartridge::mirroring`], which [`Sachen8Reg::update_mirroring`] has set.
    fn refresh_nametable_pages(&mut self) {
        self.nametable_pages = if self.mirroring_mode() == 0 {
            [0, 0, 0, 1]
        } else {
            [Cartridge::FOLLOW_MIRRORING; 4]
        };
    }

    fn update_mirroring(&mut self, memory: &mut CartridgeMemory) {
        match self.mirroring_mode() {
            // Mode 0 is the per-table layout above; the mirroring field is left as it is,
            // because `nametable_page` answers for all four tables and nothing consults it.
            0 => {}
            1 => memory.set_mirroring(NametableMirroring::Horizontal),
            2 => memory.set_mirroring(NametableMirroring::Vertical),
            // Q-141: "Single-screen, page 1", `S1` in the page's own notation.
            _ => memory.set_mirroring(NametableMirroring::SingleScreenB),
        }
    }
}

impl Mapper for Sachen8Reg {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        for register in self.registers {
            writer.write_u8(register);
        }
        writer.write_u8(self.index);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for register in &mut self.registers {
            *register = reader.read_u8()?;
        }
        self.index = reader.read_u8()?;
        self.refresh_nametable_pages();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.registers = [0; 8];
        self.index = 0;
        self.refresh_nametable_pages();
        self.update_mirroring(memory);
        self.update_banks();
    }

    /// PRG-ROM above $8000, and the data port below it.
    ///
    /// "All three bits in all eight registers are fully readable and writable": the port answers
    /// the selected register's three bits. The five pins the board does not drive read as **0**
    /// and not as open bus, for the reason [`super::Jv001`]'s own note gives — this hook returns
    /// a whole byte and cannot see the CPU data bus.
    ///
    /// Nothing else in $4020-$7FFF answers, which is what makes that window open bus: the board
    /// has no PRG-RAM (`registry::board_memory_sizes`) and no other register.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if address & Self::PORT_MASK == Self::DATA_PORT {
            return Some(self.registers[usize::from(self.index & 0x07)] & 0x07);
        }

        None
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & Self::PORT_MASK {
            Self::INDEX_PORT => self.index = value & 0x07,
            Self::DATA_PORT => {
                self.registers[usize::from(self.index & 0x07)] = value & 0x07;
                // R0, R1 and R3 (and R2 on mapper 150) land here as well and are **stored and
                // nothing else**: the page calls them unused, and every register is readable, so
                // a protection routine that writes one and reads it back must see what it wrote.
                self.refresh_nametable_pages();
                self.update_mirroring(memory);
                self.update_banks();
            }
            // Everything else in $4020-$FFFF: the mask decodes A15, A14, A8 and A0 only, so a
            // $8000-$FFFF write is `$8100`-shaped at most and never `$4100` or `$4101`.
            _ => {}
        }
    }

    fn nametable_page(&self, table: i32) -> i32 {
        self.nametable_pages[usize::try_from(table).unwrap_or(0) & 0x03]
    }
}
