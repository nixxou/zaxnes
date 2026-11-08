use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper 1, MMC1 (SxROM), SUROM and SOROM included (REQ-CORE-100).
///
/// Each write to $8000-$FFFF shifts one bit into a five-bit serial register and the fifth
/// write commits it to the register the address selects. Two writes on consecutive CPU
/// cycles only count once (read-modify-write instructions). On 512 KiB boards bit 4 of CHR
/// bank 0 supplies the top PRG address line, and its `S` bits bank the PRG-RAM of SOROM and
/// SXROM when the header announces more than 8 KiB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc1 {
    banks: BankWindows,
    shift: i32,
    shift_count: i32,
    control: u8,
    chr0: u8,
    chr1: u8,
    prg: u8,
    last_write_cycle: i64,
    /// NES 2.0 submapper 5 (MR-1-4), decided at load.
    fixed_prg: bool,
    /// How to read the `S` bits of CHR bank 0 for this board's PRG-RAM (MR-1-3), decided at
    /// load so that `prg_ram_window` keeps exactly one shift and one mask.
    ram_bank_shift: u32,
    ram_bank_mask: u8,
    /// Bit 4 of the PRG register, masked: `$10` on a board that wires the PRG-RAM disable and
    /// **`$00` on mapper 155**, which has no such wire.
    ///
    /// A mask and not a `bool` tested beside the register, for the reason `ram_bank_mask` above
    /// gives and which MF-6a2 step 2 then measured: MF-6a step 4 added mapper 155 as
    /// `!self.no_ram_disable && …`, which is free on `Mapper::prg_ram_enabled`'s path — that
    /// reads the cached answer — but not on the **commit** path, where
    /// [`Mmc1::refresh_prg_ram_disable`] runs. `Mmc1::update_banks` grew **+8 001 instructions
    /// over 600 level-2 frames of Mega Man 2**, +2 217 on Metroid and +4 311 on Zelda II, all
    /// three plain mapper-1 games paying for mapper 155's existence. Folded into the mask the
    /// test disappears and mapper 1 is back to one load and one `and` per register.
    prg_disable_mask: u8,
    /// The same bit of CHR bank 0, which only an SNROM wires (see [`Mmc1::is_snrom`]) as a
    /// second chip enable for the PRG-RAM: `$10` on an SNROM that is not a mapper 155, `$00`
    /// everywhere else.
    ///
    /// **Behind `mmc1-snrom-e-bit`, off in `default`** — the owner's decision of lot MF-LEG-1B,
    /// Q-176, written out at length under *the SNROM `E` bit* below. With the feature off this
    /// field does not exist, `Mmc1` is the base commit's layout minus one byte, and
    /// [`Mmc1::refresh_prg_ram_disable`] is one load and one `and`.
    #[cfg(feature = "mmc1-snrom-e-bit")]
    chr_disable_mask: u8,
    /// Bit 3 of the PRG register, masked: `$08` on an **MMC1A** — NES 2.0 mapper 1 submapper 3 —
    /// and `$00` on every other board (MF-LEG-1 step 5, report entry MMC1-4).
    ///
    /// nesdev ("MMC1") prints the `R` bit of `$E000` as the one place the two ASIC revisions
    /// differ:
    ///
    /// ```text
    /// 4bit0
    /// -----
    /// RPPPP
    /// |||||
    /// |++++- Select 16 KiB PRG-ROM bank (low bit ignored in 32 KiB mode)
    /// +----- MMC1A:
    ///            0: fixed bank affects A17..A14
    ///            1: fixed bank only affects A16..A14, bit 3 directly controls A17 across the
    ///               entire $8000-$FFFF address range
    ///        MMC1B:
    ///            0: PRG-RAM enabled
    ///            1: PRG-RAM disabled
    /// ```
    ///
    /// So on an MMC1A with `R` set, the **fixed** window of PRG mode 2 or 3 — the only window
    /// whose bank number does not already come from the register — keeps the low three bits its
    /// own logic gives it (`$0` or `$7`) and takes A17 from bit 3 of the register instead of from
    /// that logic. The switchable window and 32 KiB mode are unaffected, bit 3 being part of their
    /// bank number already, which is the other half of "across the entire `$8000-$FFFF` address
    /// range". And `R` is not a PRG-RAM disable on that revision, which is
    /// [`Mmc1::prg_disable_mask`] being zero: "PRG-RAM is always enabled at `$6000-$7FFF`".
    ///
    /// A **mask**, for the reason [`Mmc1::prg_disable_mask`] gives at length and MF-6a2 step 2
    /// measured: a `bool` tested beside the register on the commit path cost three plain mapper-1
    /// games up to +8 001 instructions over 600 frames for a board that was not theirs. Here the
    /// two fixed-bank numbers are one `and`, one `andn` and two `or`s whatever the revision.
    ///
    /// **Behind `board-mmc1a`**, so that with the feature off the field does not exist, this
    /// struct's layout is the base commit's to the byte — which MF-6b step 1 measured as worth
    /// +0.009 % to +0.027 % on eleven of PR-1's thirteen ROMs for one byte of displacement — and
    /// `update_banks` folds the two numbers back to the constants `outer` and `outer | $0F`.
    ///
    /// **Mapper 155 is deliberately not given this**, although the page makes it the same
    /// revision: it is outside this lot's scope ("no change to the emulation of any mapper outside
    /// the ones listed"), so a mapper-155 dump still gets the suppressed PRG-RAM disable it had
    /// and no A17. `docs/progress/MF-LEG-1.md` step 5 records it as the follow-up.
    #[cfg(feature = "board-mmc1a")]
    mmc1a_fixed_mask: u8,
    /// What [`Mapper::prg_ram_enabled`] answers, recomputed whenever `prg` or `chr0` commits.
    ///
    /// A field rather than an expression because on an SNROM the answer needs *both* registers:
    /// kept as `(self.prg | (self.chr0 & mask)) & 0x10 == 0` it would put a second load and a
    /// mask on the CPU's $6000-$7FFF decode for every MMC1 game. Derived state, so it is not
    /// written to the savestate — `load_registers` recomputes it (REQ-CORE-091).
    prg_ram_disabled: bool,
}

impl Mmc1 {
    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let header = memory.rom().header();
        // MR-1-3, nesdev ("MMC1"): "For SXROM, the upper `S` (bit 3) selects the SRAM's A14,
        // and the lower `S` (bit 2) selects A13", and "The SOROM board only implements the
        // upper `S` bit". SOROM is the 16 KiB board, SXROM the 32 KiB one; our old
        // `(chr0 >> 2) & 0x03` reduced to bit 2 over a 16 KiB RAM, one bit off, which put the
        // SOROM titles' saves in the half that is not battery-backed.
        let (ram_bank_shift, ram_bank_mask) =
            match memory.prg_ram().len() / BankWindows::PRG_WINDOW_SIZE {
                0 | 1 => (0, 0x00),
                2 => (3, 0x01),
                _ => (2, 0x03),
            };
        // nesdev ("INES Mapper 155") gives the board as the MMC1 "with the WRAM disable bit
        // removed", so on mapper 155 neither wire exists and both masks are zero.
        // `cfg!` and not `#[cfg]`: with `board-155` off this folds to a plain `$10` and
        // the comparison is gone, so the MMC1 proper is the code it always was.
        let is_155 = cfg!(feature = "board-155") && memory.rom().mapper() == 155;
        // NES 2.0 mapper 1 submapper 3, the MMC1A (MF-LEG-1 step 5): `R` of `$E000` is not a
        // PRG-RAM disable on that revision -- "PRG-RAM is always enabled at $6000-$7FFF" -- so
        // `prg_disable_mask` is zero for it too. `chr_disable_mask`, where this build has it at
        // all, is **not**: that bit belongs to the SNROM *board*, which wires CHR A16 to the
        // RAM's second chip enable whichever revision of the ASIC is on it, and only mapper
        // 155's page removes a wire. No dump is known that is both an SNROM and a declared
        // MMC1A, so the two readings have nothing to disagree about; see
        // `Mmc1::mmc1a_fixed_mask`. With `mmc1-snrom-e-bit` off -- the default -- the question
        // does not arise, the MMC1A keeping its RAM through both bits.
        let is_mmc1a = cfg!(feature = "board-mmc1a") && header.is_nes20 && header.submapper == 3;
        #[cfg(feature = "mmc1-snrom-e-bit")]
        let chr_wired = u8::from(!is_155) << 4;
        let prg_wired = u8::from(!(is_155 || is_mmc1a)) << 4;
        Self {
            banks: windows_for(memory),
            shift: 0,
            shift_count: 0,
            control: 0x0C,
            chr0: 0,
            chr1: 0,
            prg: 0,
            last_write_cycle: -4,
            fixed_prg: header.is_nes20 && header.submapper == 5,
            ram_bank_shift,
            ram_bank_mask,
            prg_disable_mask: prg_wired,
            #[cfg(feature = "mmc1-snrom-e-bit")]
            chr_disable_mask: if Self::is_snrom(memory) { chr_wired } else { 0 },
            #[cfg(feature = "board-mmc1a")]
            mmc1a_fixed_mask: if is_mmc1a { 0x08 } else { 0 },
            prg_ram_disabled: false,
        }
    }

    // ------------------------------------------------------------------ the SNROM `E` bit
    //
    // **Behind `mmc1-snrom-e-bit`, which is OFF in `default`.** The owner's decision of lot
    // MF-LEG-1B, taken on 2026-11-08 in answer to Q-176: by default this build applies no `E`
    // bit at all, and `$E000` bit 4 -- the `R` bit, and never on an MMC1A -- is the only wire to
    // the PRG-RAM's chip enable. The feature restores the behaviour every build from MF-3 to
    // MF-LEG-1 shipped, the three-way size test included. What follows is why, because the
    // documentation and the implementations disagree and the decision goes against the page.
    //
    // **What the page says.** nesdev ("MMC1"), SNROM: the board, "which only supports 8 KiB of
    // CHR-ROM/-RAM, uses the upper CHR bank select line coming out of the mapper (CHR A16; bit 4
    // of bank number) as an additional chip enable for the PRG-RAM", and the `E` bit of both CHR
    // bank registers is "PRG-RAM disable (0: enable, 1: open bus)". The bit exists, it is an
    // SNROM-only wire, and the page is unambiguous about it. On every other MMC1 board that
    // line is something else: `P`, the 256 KiB outer PRG bank, on SUROM and SXROM, and CHR A16,
    // a real bank line, on SJROM, SKROM and SLROM.
    //
    // **What the test ROM says.** `holy-mapperel` takes the page's side and reserves detailed
    // digit `4xxx` for exactly this question -- "$A000 bit 4 does not disable WRAM (SNROM), or
    // $A000 bit 4 disables WRAM (all but SNROM)" (`holy-mapperel/README.md:201-203`). Two of its
    // twelve `M1_*` images reach the SNROM half: `M1_P128K.nes` and `M1_P128K_CR8K.nes` announce
    // themselves as "SNROM (MMC1)", report "8K PRG RAM OK" and probe that digit. Their headers
    // are NES 2.0 with 128 KiB of PRG, no CHR-ROM, 8 KiB of CHR-RAM in byte 11 and **byte 10 =
    // `$00`**, i.e. no work RAM declared at all -- the chip the ROM then tests is the one
    // REQ-CORE-012's 8 KiB default handed it. So the suite expects the bit to work, and these
    // two rows **fail in the default build**. They are marked as required failures rather than
    // left to read as a regression, and they are asserted to pass with the feature on
    // (`SNROM_ROW` in `zax-harness/tests/test_roms_tests.rs`, over [`Mmc1::SNROM_E_BIT`]).
    //
    // **What the implementations do.** Not one of the five cross-checked emulators applies it;
    // all five disable the PRG-RAM from `$E000` bit 4 alone. Read, not copied; paths relative to
    // `third_party/`, and `docs/review/LEGACY-XCHECK.md` entry MMC1-1 has the table:
    //
    // * FCEUX, `fceux/src/boards/mmc1.cpp:40-49` -- the work-RAM read and write handlers test
    //   `DRegs[3]` bit 4 and the mapper-155 flag, and nothing else;
    // * Nestopia UE, `nestopia/source/core/board/NstBoardMmc1.cpp:138-146` -- `Mmc1::UpdateWrk`
    //   takes the enable from the PRG register's disable bit, and only when the revision is
    //   not A;
    // * Nintendulator, `nintendulator-mappers/src/Hardware/h_MMC1.cpp:132-137` --
    //   `GetWRAMEnabled` returns true on an MMC1A and otherwise the complement of `Regs[3]`
    //   bit 4;
    // * Mesen 2, `mesen2/Core/NES/Mappers/Nintendo/MMC1.h:85-87` and `:131` -- only the `$E000`
    //   write sets the flag that the work RAM's access type is then read from;
    // * puNES, `punes/src/core/mappers/MMC1.c:124-154` -- and this is the interesting vote. It
    //   **has** the `E`-bit expression, written out in full with the page's own bit diagram
    //   quoted above it (`:125-143`), and it is **commented out** (`:147-151`) with a named
    //   regression immediately above (`:145-146`): with the check live, *Witch n' Wiz* shows
    //   corrupted screens after the save-slot selection. The shipped expression below it
    //   (`:152-154`) is `$E000` bit 4 on an MMC1B and nothing on an MMC1A. The same comment
    //   records the community's own reading -- that emulators tend not to implement the bit
    //   because it was not confirmed until October 2010.
    //
    // **Why no board test can save it.** The `E` bit only bites on a board that has it, and this
    // core cannot tell that board apart. The SNROM's row of the *SxROM* table -- 128/256 KiB of
    // PRG, 8 KiB of PRG-RAM, 8 KiB of CHR-RAM or -ROM -- is also the **SGROM's**, which has no
    // work-RAM chip at all: an iNES 1.0 header has no CHR-RAM field, and REQ-CORE-012 floors
    // every mapper-1 image at 8 KiB of PRG-RAM whether the cartridge carries the chip or not. So
    // the size test fires on every iNES 1.0 mapper-1 dump with CHR-RAM -- *The Legend of Zelda*,
    // *Metroid*, *Final Fantasy*, *Mega Man 2*, *Castlevania II*, SGROM dumps included -- and
    // hands each a second chip enable on a bit that is a meaningless CHR bank number on an 8 KiB
    // CHR-RAM board. MF-LEG-1 step 1 built the other rule, an NES 2.0 header declaring all three
    // memories; it is **removed** with its feature `mmc1-snrom-header`, because the two test ROMs
    // above are precisely the dumps it reads wrong, and because under this decision the default
    // build asks the question of no dump at all.
    //
    // **Why the owner chose compatibility.** AMD-061 makes the public documentation prevail, and
    // this is the one place in the mapper work where it is set aside deliberately rather than by
    // oversight -- so the reason is recorded here and in Q-176 rather than left to be inferred.
    // The bit's only observable effect is to take a work RAM **away**: dropping it can re-enable
    // RAM that was switched off and can never switch off RAM that was on, so no game that runs
    // today can break and one named game is fixed. Against that, a build that applies it stands
    // alone among six, on a popular set it cannot identify, for a wire no shipped game
    // exercises -- MF-LEG-1 step 1's extra check played *Zelda II* in both regions, *Metroid*,
    // *Mega Man 2* and *Final Fantasy* for 3600 frames each, 181 M instructions, sampling the
    // register after every CPU instruction, and not one of them commits bit 4 to `$A000`
    // (`zax-nes/tests/mf_leg_1_mmc1_e_bit_tests.rs`). The price is two conformance rows that say
    // so out loud, and the bit stays reachable behind one flag for whoever wants the page's
    // hardware rather than the convention.

    /// Whether this image is on an SNROM board, where bit 4 of the CHR bank registers disables
    /// the PRG-RAM — the three-way size test every build from MF-3 to MF-LEG-1 shipped, and what
    /// `mmc1-snrom-e-bit` restores exactly.
    ///
    /// The PRG-RAM size read is the **effective** one, so a dump that cannot be told from an
    /// SNROM is an SNROM here, as it is for the test ROM's own driver. See *the SNROM `E` bit*
    /// above for why that is the whole difficulty, and why the default build asks nothing.
    #[cfg(feature = "mmc1-snrom-e-bit")]
    fn is_snrom(memory: &CartridgeMemory) -> bool {
        let header = memory.rom().header();
        header.prg_rom_size <= 256 * 1024
            && header.chr_rom_size <= BankWindows::PRG_WINDOW_SIZE
            && memory.prg_ram().len() == BankWindows::PRG_WINDOW_SIZE
    }

    /// Whether this build applies the SNROM `E` bit at all (`mmc1-snrom-e-bit`, **off** in
    /// `default`), for the two `holy-mapperel` rows that are exactly that question. See *the
    /// SNROM `E` bit* above and Q-176.
    pub const SNROM_E_BIT: bool = cfg!(feature = "mmc1-snrom-e-bit");

    /// Whether the committed CHR bank 0 register carries the SNROM `E` bit — bit 4, the second
    /// PRG-RAM chip enable of *the SNROM `E` bit* above.
    ///
    /// **`pub`, and the one reason it is**: MF-LEG-1 step 1 owes the owner an extra check that
    /// taking the `E` bit away moves none of the five games whose dumps the old size test read as
    /// SNROMs, and the only exact way to ask that of a running game is to watch this bit after
    /// every CPU instruction — "the old rule would have disabled the RAM" is "this bit was set",
    /// on any board that test fired on. It is MF-LEG-1B's evidence too, the decision being the
    /// same change carried further: `mf_leg_1_mmc1_e_bit_tests.rs` is the only caller. A
    /// `const fn` over one field, on no path at all, and **not** behind `mmc1-snrom-e-bit` — the
    /// check has to run in the build that ships.
    #[must_use]
    pub const fn chr_bank0_has_the_e_bit(&self) -> bool {
        self.chr0 & 0x10 != 0
    }

    // ------------------------------------------------------------ the NES-EVENT's five lines
    //
    // Behind `board-105`, because mapper 105 is the only caller: with that feature off the
    // five accessors are dead code and rustc says so.
    //
    // Mapper 105 ([`super::NesEvent`]) is this chip with its PRG outputs taken through discrete
    // logic, so it drives the same shift register, the same four registers and the same
    // mirroring and then maps PRG itself. These five accessors are the whole of what it needs;
    // **they add no field and no branch to this struct**, which is MF-6b step 3's answer to §5's
    // `const`-versus-field question — mapper 1 carries neither the 30-bit counter nor a test for
    // it, and `cmp4c.py` reads `Mmc1::update_banks` byte-identical across the step.

    /// Recomputes the windows from the registers, for a board that holds this chip and writes
    /// its own PRG layout over them (mappers 105 and 116). Behind the two features that need it.
    #[cfg(feature = "board-116")]
    pub(crate) fn refresh_banks(&mut self, memory: &mut CartridgeMemory) {
        self.update_banks(memory);
    }

    /// The committed `$8000` register, `[.... PSMM]`.
    #[cfg(any(feature = "board-105", feature = "board-116"))]
    pub(crate) const fn control_register(&self) -> u8 {
        self.control
    }

    /// The committed `$A000` register, which on the NES-EVENT is `[...I OAA.]`.
    #[cfg(feature = "board-105")]
    pub(crate) const fn chr0_register(&self) -> u8 {
        self.chr0
    }

    /// The committed `$E000` register, `[...W BBBB]` on both boards.
    #[cfg(any(feature = "board-105", feature = "board-116"))]
    pub(crate) const fn prg_register(&self) -> u8 {
        self.prg
    }

    /// The bank windows, for a board that computes its own PRG layout from the registers above
    /// and writes it over the MMC1's — which is what the NES-EVENT's discrete ICs do.
    #[cfg(any(feature = "board-105", feature = "board-116"))]
    pub(crate) const fn windows_mut(&mut self) -> &mut BankWindows {
        &mut self.banks
    }

    /// Forces `$A000` to `value` without a commit, for the NES-EVENT's power-on `I` bit alone.
    #[cfg(feature = "board-105")]
    pub(crate) const fn force_chr0_register(&mut self, value: u8) {
        self.chr0 = value;
    }

    /// Recomputes [`Mmc1::prg_ram_disabled`] from the registers that drive it.
    ///
    /// Bit 4 of the PRG register — `R` of `$E000` — disables the RAM on every board that has the
    /// chip, and that is the whole of it in the **default** build: MF-LEG-1B's decision is that
    /// the SNROM `E` bit is not applied, and with `mmc1-snrom-e-bit` off the second term is a
    /// constant zero, so the expression is one load, one `and` and a test. See *the SNROM `E`
    /// bit* above for the five votes and the reason.
    ///
    /// With the feature on, bit 4 of CHR bank 0 joins it on an SNROM. The page gives CHR bank 1
    /// the same `E` bit, "ignored in 8 KiB mode", and then requires that "SNROM's `E` bit and
    /// SO/U/XROM's `P` and `S` bits in both CHR bank registers must be set to the same values" —
    /// so for any program the page admits, CHR bank 0 is the whole answer, and that is also the
    /// register Holy Mapperel names.
    ///
    /// Mapper 155 is this chip on a board that does not wire the disable: nesdev
    /// ("INES Mapper 155") gives it as MMC1 "with the WRAM disable bit removed", and Mesen 2
    /// implements it by holding the flag at false (`MMC1_155.h:9`). Here it is the masks being
    /// zero, so the shape of the expression is the same whatever the board — see
    /// [`Mmc1::prg_disable_mask`] for the measurement that made it a mask.
    fn refresh_prg_ram_disable(&mut self) {
        #[cfg(feature = "mmc1-snrom-e-bit")]
        let e_bit = self.chr0 & self.chr_disable_mask;
        #[cfg(not(feature = "mmc1-snrom-e-bit"))]
        let e_bit = 0u8;
        self.prg_ram_disabled = ((self.prg & self.prg_disable_mask) | e_bit) != 0;
    }

    fn update_banks(&mut self, memory: &mut CartridgeMemory) {
        self.refresh_prg_ram_disable();

        // A18 of a 512 KiB board, in 16 KiB units.
        let outer = if memory.rom().prg_rom().len() > 256 * 1024 {
            i32::from((self.chr0 >> 4) & 0x01) << 4
        } else {
            0
        };
        let bank = i32::from(self.prg & 0x0F) | outer;

        // The two fixed-bank numbers of PRG modes 2 and 3, `$0` and `$F` of the block on every
        // MMC1B — and on an **MMC1A** whose `R` bit is set, the same numbers with A17 taken from
        // bit 3 of the PRG register instead of from the fixed-bank logic: "fixed bank only affects
        // A16..A14, bit 3 directly controls A17" (MF-LEG-1 step 5; see
        // `Mmc1::mmc1a_fixed_mask`). `r` is `$08` when and only when the board is an MMC1A and
        // the bit is set, so `$08 & !r` is the logic's own A17 and `prg & r` the register's.
        #[cfg(feature = "board-mmc1a")]
        let (fixed_first, fixed_last) = {
            let r = (self.prg >> 1) & self.mmc1a_fixed_mask;
            (
                i32::from(self.prg & r),
                i32::from((0x08 & !r) | (self.prg & r)) | 0x07,
            )
        };
        // With the feature off the two are the constants they have always been, so the arms below
        // compile to `outer` and `outer | 0x0F` exactly as the base commit's did.
        #[cfg(not(feature = "board-mmc1a"))]
        let (fixed_first, fixed_last) = (0, 0x0F);

        // MR-1-4. NES 2.0 submapper 5 is SEROM, SHROM and SH1ROM, which "use a fixed 32k PRG
        // ROM with no banking support" (nesdev, "NES 2.0 submappers"); Dr. Mario is the dump
        // that names the board. The PRG register does nothing there.
        if self.fixed_prg {
            self.banks.map_prg_32k(0);
        } else {
            match (self.control >> 2) & 0x03 {
                // 32 KiB at $8000, the low bit of the bank number ignored. `R` changes nothing
                // here: bit 3 is part of this window's own bank number already.
                0 | 1 => self.banks.map_prg_32k(bank >> 1),
                // First bank of the block fixed at $8000, switchable at $C000.
                2 => {
                    self.banks.map_prg_16k(0, outer | fixed_first);
                    self.banks.map_prg_16k(2, bank);
                }
                // Switchable at $8000, last bank of the block fixed at $C000.
                _ => {
                    self.banks.map_prg_16k(0, bank);
                    self.banks.map_prg_16k(2, outer | fixed_last);
                }
            }
        }

        if self.control & 0x10 != 0 {
            self.banks.map_chr_4k(0, i32::from(self.chr0));
            self.banks.map_chr_4k(4, i32::from(self.chr1));
        } else {
            self.banks.map_chr_8k(i32::from(self.chr0 >> 1));
        }

        memory.set_mirroring(match self.control & 0x03 {
            0 => NametableMirroring::SingleScreenA,
            1 => NametableMirroring::SingleScreenB,
            2 => NametableMirroring::Vertical,
            _ => NametableMirroring::Horizontal,
        });
    }
}

impl Mapper for Mmc1 {
    banked!();

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_i32(self.shift);
        writer.write_i32(self.shift_count);
        writer.write_u8(self.control);
        writer.write_u8(self.chr0);
        writer.write_u8(self.chr1);
        writer.write_u8(self.prg);
        writer.write_i64(self.last_write_cycle);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.shift = reader.read_i32()?;
        self.shift_count = reader.read_i32()?;
        self.control = reader.read_u8()?;
        self.chr0 = reader.read_u8()?;
        self.chr1 = reader.read_u8()?;
        self.prg = reader.read_u8()?;
        self.last_write_cycle = reader.read_i64()?;
        self.refresh_prg_ram_disable();
        Ok(())
    }

    /// A zero [`Mmc1::prg_disable_mask`] is mapper 155 or a declared MMC1A, the two boards that
    /// do not wire bit 4 of the PRG register to the PRG-RAM's chip enable; a non-zero
    /// [`Mmc1::mmc1a_fixed_mask`] tells the second from the first, since mapper 155 does not get
    /// the `R` bit's A17 in this build (MF-LEG-1 step 5).
    ///
    /// With `board-mmc1a` off the second term does not exist and this is the one comparison it
    /// always was.
    fn mapper_number(&self) -> i32 {
        #[cfg(feature = "board-mmc1a")]
        let is_155 = self.prg_disable_mask == 0 && self.mmc1a_fixed_mask == 0;
        #[cfg(not(feature = "board-mmc1a"))]
        let is_155 = self.prg_disable_mask == 0;
        if is_155 {
            155
        } else {
            1
        }
    }

    /// Bit 4 of the PRG register disables the PRG-RAM chip (nesdev, "MMC1"), and with
    /// `mmc1-snrom-e-bit` on, bit 4 of CHR bank 0 on an SNROM as well. Folded into one bool by
    /// [`Mmc1::refresh_prg_ram_disable`].
    fn prg_ram_enabled(&self) -> bool {
        !self.prg_ram_disabled
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.shift = 0;
        self.shift_count = 0;
        // Power-on: the last bank is fixed at $C000, the only state the chip is known to come
        // up in (nesdev); everything else starts at 0.
        self.control = 0x0C;
        self.chr0 = 0;
        self.chr1 = 0;
        self.prg = 0;
        // Far enough back that the very first write is never taken for the second half of a
        // read-modify-write pair.
        self.last_write_cycle = -4;
        self.update_banks(memory);
    }

    /// The `S` bits of CHR bank 0 pick the 8 KiB PRG-RAM chip; which bits, and whether there
    /// are any, is `Mmc1::new`'s decision (MR-1-3).
    fn prg_ram_window(&self, memory: &CartridgeMemory) -> (usize, usize) {
        let bank = usize::from((self.chr0 >> self.ram_bank_shift) & self.ram_bank_mask);
        (bank * BankWindows::PRG_WINDOW_SIZE, memory.prg_ram().len())
    }

    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        let ignored = memory.cpu_cycle() - self.last_write_cycle < 2;
        self.last_write_cycle = memory.cpu_cycle();

        // MR-1-1, nesdev ("MMC1"): the consecutive-write restriction "only applies to the data
        // being written on bit 0; the bit 7 reset is never ignored" — the sentence that names
        // Shinsenden, whose reset arrives as the second half of a read-modify-write. The test
        // used to sit above this write, so the reset was swallowed with the data.
        if value & 0x80 != 0 {
            self.shift = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            self.update_banks(memory);
            return;
        }

        if ignored {
            return;
        }

        self.shift |= i32::from(value & 0x01) << self.shift_count;
        self.shift_count += 1;
        if self.shift_count < 5 {
            return;
        }

        let data = u8::try_from(self.shift & 0x1F).unwrap_or(0);
        self.shift = 0;
        self.shift_count = 0;
        match address & 0x6000 {
            0x0000 => self.control = data,
            0x2000 => self.chr0 = data,
            0x4000 => self.chr1 = data,
            _ => self.prg = data,
        }

        self.update_banks(memory);
    }
}
