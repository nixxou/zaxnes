use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// iNES mapper **246**, the **G0151-1** board of *封神榜 (Fēngshénbǎng: Fúmó Sān Tàizǐ)*: eight bank
/// registers under one mask, 2 KiB of battery PRG-RAM at the unusual `$6800-$6FFF`, and **PRG A17
/// forced high on a read of four small ranges at the top of the address space** (MF-UNL-12 step 3,
/// REQ-CORE-102).
///
/// nesdev ("INES Mapper 246"): "iNES Mapper 246 denotes the **G0151-1** circuit board, used only
/// for the Taiwanese game 封神榜." Four 8 KiB PRG windows, four 2 KiB CHR windows, and
/// "Nametable mirroring is hard-wired and set by the iNES header" — so the board has no mirroring
/// register and **no counter at all**, which is what puts it in [`super::DecodeBoard`] rather than
/// beside the MMC3 clones of [`super::IrqBoard`].
///
/// # The two registers, and the one mask they share with the read
///
/// ```text
/// PRG-ROM bank select ($6000-$6003), write       CHR-ROM bank select ($6004-$6007), write
///  A~[0110 0000 000. .0rr] D~[..bb bbbb]          A~[0110 0000 000. .1rr] D~[bbbb bbbb]
///                      ||       ++-++++-                              ||     ++++-++++-
///                      ||       PRG A18..A13                          ||     CHR A18..A11
///                      ++- CPU A14..A13 to switch                     ++- PPU A12..A11 to switch
///
/// * "When reading from CPU address $FFE4-$FFE7, $FFEC-$FFEF, $FFF4-$FFF7, or $FFFC-$FFFF,
///    PRG A17 is forced high, as if register $6003 were OR'd with $10."
/// * "Register $6003 is set to $FF at power-on only."
/// ```
///
/// **The address mask and the read condition are the same number**, and that is not assumed here
/// but used: `A~[0110 0000 000. .0rr]` is `address & $FFE4 == $6000`, with A4, A3 and the two `r`
/// bits free; and the four read ranges are exactly `address & $FFE4 == $FFE4` — A15-A5 all set
/// with A2 set, A4, A3, A1 and A0 free. [`G0151::DECODE_MASK`] is that one constant, and
/// `req_core_102_mapper_246_forces_prg_a17_on_the_eight_addresses_of_its_own_mask` walks all 32
/// addresses of `$FFE0-$FFFF` to show which eight are forced and which twenty-four are not.
///
/// So the register window is `$6000-$601F` and nothing wider: A11-A5 must all be clear, which also
/// keeps it clear of the PRG-RAM at `$6800-$6FFF` (A11 set). §7.2's test bullet says "a write at
/// `$6000-$67FF`" instead, which is Mesen's decode and not this page's — **Q-172** records it, and
/// the test is written from the mask.
///
/// # The forced A17 is a read, so the window cannot be cached
///
/// `Cartridge::read_rom_window` answers a CPU read of `$8000-$FFFF` from a four-entry table
/// refreshed after each CPU write, which is the fast path every ordinary board takes. A board
/// whose bank depends on the **address within** a window cannot be in it — [`Mapper::rom_windows`]
/// says so itself: "a board whose windows move at another time, or whose reads of `$8000-$FFFF` go
/// through [`Mapper::cpu_read_override`], must return `NO_ROM_WINDOW` for them" — and
/// `read_rom_window`'s own `debug_assert_eq!` against [`Mapper::try_read_prg`] is what would catch
/// a board that tried.
///
/// So [`G0151::rom_windows`] answers [`super::NO_ROM_WINDOW`] for the **`$E000-$FFFF` window
/// alone** and every read of it goes through [`G0151::try_read_prg`], where the forcing lives. The
/// other three windows keep the fast path, which is three quarters of the game's code. The price
/// is one call per read of `$E000-$FFFF` **for this board and no other**, which is §4.10's rule.
///
/// # The 2 KiB chip's one window, and the three mirrors of it this build cannot refuse
///
/// The page gives the RAM one window, `$6800-$6FFF`, and that is A11 of the board's own decode.
/// [`Mapper::prg_ram_window`] cannot say "and nowhere else": it is a base and a wrap, asked once
/// per CPU write, so a 2 KiB chip appears four times across `$6000-$7FFF`. A short chip mirrored
/// inside the window is the reading mapper 164's `board_memory_sizes` row already takes for the
/// same shape, and this board takes it too — with the **register decode winning over it**, which
/// is what [`G0151::intercept_cpu_write`] is for: a write the page's mask names is a bank register
/// and never reaches the chip. Recorded in **Q-172**.
///
/// # *"Why does Fong Shen Bang only work in Famiclones?"*
///
/// The page's *See also* is a forum thread with that title, and `docs/review/UNL-census.md` §7.2
/// asks for it to be recorded. It is **not** a statement about this board: whether the game runs
/// is a console-timing matter, and this build targets NTSC, PAL and Dendy timing
/// (`crate::nes_region`). Nothing here is conditioned on it, and nothing here could be — the board
/// has no timing of its own at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct G0151 {
    banks: BankWindows,
    /// `$6000-$6003`, six bits live each: "PRG A18..A13 of bank to select".
    prg: [u8; 4],
    /// `$6004-$6007`, all eight bits: "CHR A18..A11 of bank to select".
    chr: [u8; 4],
}

impl G0151 {
    /// The mask both register diagrams carry, `A~[0110 0000 000. .?rr]`: A15-A12 and A11-A5 fixed,
    /// A2 selecting PRG from CHR, A4, A3 and A1-A0 free. **And** the mask of the forced-A17 read;
    /// see the type's own note.
    const DECODE_MASK: u16 = 0xFFE4;
    /// `address & DECODE_MASK` for the four PRG bank registers.
    const PRG_PORT: u16 = 0x6000;
    /// … and for the four CHR bank registers.
    const CHR_PORT: u16 = 0x6004;
    /// … and for the reads the page forces PRG A17 on: `$FFE4-$FFE7`, `$FFEC-$FFEF`,
    /// `$FFF4-$FFF7` and `$FFFC-$FFFF`, which is this mask over `$FFE0-$FFFF`.
    const FORCED_A17_PORT: u16 = 0xFFE4;

    /// "PRG A18..A13 of bank to select": six bits of 8 KiB bank, 512 KiB.
    const PRG_BANK_MASK: u8 = 0x3F;

    /// "as if register `$6003` were OR'd with `$10`" — bit 4 of the six-bit PRG bank field, which
    /// is **PRG A17**.
    const FORCED_A17: u8 = 0x10;

    /// The slot `$6003` drives, which is the only register the forcing touches.
    const LAST_PRG_SLOT: usize = 3;

    /// "Register `$6003` is set to `$FF` at power-on **only**" — so it is not a reset value, which
    /// [`G0151::reset`]'s own note is about. Masked to the field's six bits it is bank `$3F`, the
    /// last 8 KiB of a 512 KiB image, which is where the vectors are.
    const PRG3_POWER_ON: u8 = 0xFF;

    /// 2 KiB of PRG-RAM, "battery-banked", at CPU `$6800-$6FFF`. Settled by
    /// `registry::board_memory_sizes`, because no header field describes a 2 KiB chip.
    pub(crate) const PRG_RAM_SIZE: usize = 2 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            prg: [0, 0, 0, Self::PRG3_POWER_ON],
            chr: [0; 4],
        };
        board.update_banks();
        board
    }

    /// The eight registers laid into the window tables: four 8 KiB PRG banks and four 2 KiB CHR
    /// banks, each the register's own field width.
    fn update_banks(&mut self) {
        for (slot, bank) in self.prg.into_iter().enumerate() {
            self.banks
                .map_prg_8k(slot, i32::from(bank & Self::PRG_BANK_MASK));
        }

        for (pair, bank) in self.chr.into_iter().enumerate() {
            self.banks.map_chr_2k(pair * 2, i32::from(bank));
        }
    }
}

impl Mapper for G0151 {
    banked!();

    fn mapper_number(&self) -> i32 {
        246
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.update_banks();
        Ok(())
    }

    /// **`$6003` keeps its value**, which is the page's own "set to `$FF` at power-on **only**"
    /// (AMD-061, REQ-CORE-004). The other seven registers go back to zero: the page gives them no
    /// power-up value, which is this build's default for a register no page states.
    ///
    /// A reset here is therefore *not* a power-on, and that is the point of the word "only" — the
    /// board has no reset line to the register at all, so the bank the vectors are fetched through
    /// survives. `amd_061_mapper_246_keeps_its_last_prg_bank_across_a_reset` is the gate.
    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.prg[0] = 0;
        self.prg[1] = 0;
        self.prg[2] = 0;
        self.chr = [0; 4];
        self.update_banks();
    }

    /// `NO_ROM_WINDOW` for `$E000-$FFFF` and the cached offset for the other three: the forced
    /// A17 is a property of the **address read**, so that window has to reach
    /// [`G0151::try_read_prg`] on every read. See the type's own note.
    fn rom_windows(&self, memory: &CartridgeMemory) -> [usize; 4] {
        let mut windows = self.banks.rom_windows(memory.rom().prg_rom().len());
        windows[Self::LAST_PRG_SLOT] = super::NO_ROM_WINDOW;
        windows
    }

    /// "When reading from CPU address `$FFE4-$FFE7`, `$FFEC-$FFEF`, `$FFF4-$FFF7`, or
    /// `$FFFC-$FFFF`, PRG A17 is forced high, as if register `$6003` were OR'd with `$10`."
    ///
    /// Taken literally: the bank of the `$E000-$FFFF` window is recomputed for those addresses
    /// with bit 4 of the register set, and the byte read from it. Everything else goes through the
    /// window table as usual.
    ///
    /// `#[inline(never)]` is **not** here and that is deliberate: this is the body of the one
    /// `$E000-$FFFF` read path, reached through `Board::try_read_prg`'s own `#[inline(always)]`
    /// jump table from `Cartridge::try_cpu_read`, and [`super::DecodeBoard::try_read_prg`]'s
    /// forward already carries the attribute — so the arm this board contributes there is one
    /// `callq` and the body inside it is this board's alone.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address & Self::DECODE_MASK == Self::FORCED_A17_PORT {
            let bank = (self.prg[Self::LAST_PRG_SLOT] | Self::FORCED_A17) & Self::PRG_BANK_MASK;
            let rom = memory.rom().prg_rom();
            let offset = usize::from(bank) * BankWindows::PRG_WINDOW_SIZE
                + (usize::from(address) & (BankWindows::PRG_WINDOW_SIZE - 1));
            return Some(rom[super::super::wrap_offset(offset, rom.len())]);
        }

        self.banks
            .read_prg(address, memory.rom().prg_rom(), memory.prg_ram())
    }

    /// The eight bank registers, **before** `Cartridge::cpu_write` can mistake them for the
    /// PRG-RAM behind the same `$6000-$7FFF` window.
    ///
    /// That is what this hook is for, and it is why the board needs it at all: the chip is at
    /// `$6800-$6FFF` and the registers at `$6000-$601F`, both inside the range that function
    /// routes to PRG-RAM. `true` consumes the write; `false` lets it fall through to the RAM, and
    /// `$6800-$6FFF` is exactly the half that does.
    ///
    /// `#[inline(never)]`: MF-6a step 1 measured what an inlined register decode does to
    /// `NesBus::write_internal`'s layout, and every board of this shape has carried the attribute
    /// since.
    #[inline(never)]
    fn intercept_cpu_write(
        &mut self,
        _memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        let slot = usize::from(address & 0x03);
        match address & Self::DECODE_MASK {
            Self::PRG_PORT => self.prg[slot] = value,
            Self::CHR_PORT => self.chr[slot] = value,
            _ => return false,
        }

        self.update_banks();
        true
    }
}
