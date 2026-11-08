use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Which of lot MF-UNL-3's seven single-latch boards a [`SingleLatch`] is.
///
/// **A runtime field and not a `const` parameter**, for the reason [`super::Jv001`]'s and
/// [`super::Namcot108`]'s notes both give: every difference between the seven is read only by
/// [`SingleLatch::write_register`] and [`SingleLatch::update_banks`], i.e. only on a CPU write
/// to the board's own window — "the rarest bus event there is". Seven monomorphisations would
/// have bought seven arms in every inner `match` of [`super::DecodeBoard`]; one field with seven
/// values buys one.
///
/// Each variant carries exactly one mapper number, so [`SingleLatch::mapper_number`] is a
/// `match` and not a stored byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchBoard {
    /// Mapper 38, the Bit Corp. UNL-PCI556 of *Crime Busters*.
    #[cfg(feature = "board-38")]
    BitCorp38,
    /// Mapper 107, the Magicseries board of *Magic Dragon*.
    #[cfg(feature = "board-107")]
    Magic107,
    /// Mapper 143, the Sachen TCA-01 of *Dancing Blocks* and *Magical Mathematics* — an NROM
    /// board whose only logic is a protection **read**.
    #[cfg(feature = "board-143")]
    Tca143,
    /// Mapper 144, the AGCI 50282 of *Death Race*: the Color Dreams board with a 300 Ω resistor
    /// on CPU D0.
    #[cfg(feature = "board-144")]
    Agci144,
    /// Mapper 145, the Sachen SA-72007 of *Sidewinder*.
    #[cfg(feature = "board-145")]
    Sachen145,
    /// Mapper 149, the Sachen SA-0036 of *Taiwan Mahjong 16*.
    #[cfg(feature = "board-149")]
    Sachen149,
    /// Mapper 190, the Zemina board of *Magic Kid Googoo* — the one of the seven with more than
    /// one window.
    #[cfg(feature = "board-190")]
    Zemina190,
    /// Mapper 29, the Sealie Computing **RET-CUFROM revD** of *Glider* — the **eighth** board of
    /// this enum and the first of them with CHR-RAM (MF-UNL-7 step 6).
    #[cfg(feature = "board-29")]
    RetCufrom,
}

/// iNES mappers **38, 107, 143, 144, 145, 149 and 190**: the seven boards of lot MF-UNL-3 that
/// are one latch each, **sharing one [`super::DecodeBoard`] variant** in the manner MF-6a step 2
/// established for [`super::SmallBoard`] (MF-UNL-3 steps 2 to 4).
///
/// One board per feature, one feature per `LatchBoard` variant, and the whole of what each
/// number's own nesdev page documents:
///
/// | # | board | window | the page's register |
/// | ---: | --- | --- | --- |
/// | 38 | Bit Corp. UNL-PCI556 | $7000-$7FFF (and $F000-$FFFF) | `[xxxx CCPP]` — a 32 KiB PRG bank and an 8 KiB CHR bank |
/// | 107 | Magicseries | $8000-$FFFF | `[BBBB BBBb]` — the 8 KiB CHR bank is **all eight** bits and the 32 KiB PRG bank the upper **seven** |
/// | 143 | Sachen TCA-01 | $4100, mask $E100, **read only** | no bank register at all: a read answers the inversion of the **address**'s low six bits |
/// | 144 | AGCI 50282 | $8000-$FFFF | mapper 11's `[CCCC PPPP]`, latched through a bus conflict in which **the ROM's bit 0 always wins** |
/// | 145 | Sachen SA-72007 | $4100, mask $E100 | `[C... ....]` — one 8 KiB CHR bank bit, at **bit 7** |
/// | 149 | Sachen SA-0036 | $8000, mask **$8000** | the same bit, moved onto the PRG window, so **with bus conflicts** |
/// | 190 | Zemina | $8000-$9FFF, $A000-$BFFF, $C000-$DFFF | three windows: a 16 KiB PRG bank whose top bit is **the address's A14**, four 2 KiB CHR banks picked by the address's A1-A0, and 16 KiB fixed to the **first** bank |
///
/// # Why one variant, and why a field
///
/// [`super::SmallBoard`] and [`super::IrqBoard`] are both full — MF-6c steps 2 to 5 priced one
/// more variant in either at `mmc5` +0.066 % — and [`super::DecodeBoard`] is the enum MF-6c step
/// 6 opened for exactly this shape. Seven numbers at one `DecodeBoard` arm each would still have
/// been free of the `Board` dispatch, but it would have put seven arms in each of that enum's ten
/// inner matches; one arm and a seven-valued field is what [`super::Jv001`] does for six numbers
/// and [`super::Namcot108`] for five, and it is what the census asks for in so many words ("one
/// `SmallBoard`-style variant holding the seven remaining latches").
///
/// # What the boards have in common, which is what makes the sharing honest
///
/// All seven answer *no* to [`Mapper::irq_asserted`] through `Mapper`'s own default (every page
/// says "IRQ: No", or documents no counter at all), none has a per-cycle or PPU-bus hook, and
/// none needs [`Mapper::intercept_cpu_write`]: the two whose window is $6000-$7FFF-adjacent are
/// mapper 38, whose page says "PRG RAM capacity: None" so that `registry::board_memory_sizes`
/// clears the window, and mapper 190, whose registers are all above $8000.
///
/// One method is forwarded for one board: [`Mapper::try_read_prg`], for mapper 143's protection
/// read at $4100. [`super::DecodeBoard`] has carried that forward since MF-UNL-1 step 1 and it
/// is `#[inline(never)]`, so the arm `Board::try_read_prg`'s `#[inline(always)]` folds into
/// `Cartridge::try_cpu_read` is one `callq`; its `cfg` gains `board-143`.
///
/// # The eighth board, added by MF-UNL-7 step 6
///
/// | # | board | window | the page's register |
/// | ---: | --- | --- | --- |
/// | 29 | Sealie Computing **RET-CUFROM revD** | $8000-$FFFF, mask **$8000** | `[...P PPCC]` — two bits of 8 KiB CHR-**RAM** bank and three of 16 KiB PRG bank, with $C000-$FFFF fixed to the last |
///
/// It meets the entry condition unchanged — the page documents no counter, so [`Mapper::irq_asserted`]
/// is `Mapper`'s own `false` — and it is the **first member of this enum with CHR-RAM**: "4x banks
/// worth of full 8KB CHR ram", which `registry::board_memory_sizes` gives it along with the
/// "8KB of WRAM mounted in the usual place" the same sentence names. It is also the only board here
/// that sets a mirroring: the page says it "is hard-wired for vertical mirroring", so
/// [`SingleLatch::reset`] says so too rather than leaving a dump's header to.
///
/// **The CopyNES flash-write path is not emulated**, which the census asks be recorded:
///
/// > If EXP0 and R/W go low, /WR reaches the flash ROM. When EXP0 is high, the internal
/// > bankswitching register gets accessed.
///
/// EXP0 is a cartridge **edge pin**, driven by a programmer and not by a game, so a running game
/// always takes the second branch — which is the branch this board implements. It is a programmer
/// feature rather than a game one, and the owner's rule 4 (development cost out of proportion)
/// settles the rest: the flash of [`super::UnRom512`] and [`super::Cheapocabra`] exists because
/// games on those boards *save* through it, and no mapper-29 title does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleLatch {
    banks: BankWindows,
    /// The one latch. Each board masks it to the field its own page draws, so the byte is the
    /// register and not the written value. On mapper 190 it is the four-bit 16 KiB PRG bank,
    /// the high bit of which comes from the address and not from the data.
    register: u8,
    /// **Mapper 190's four 2 KiB CHR registers**, and nothing on the other six.
    ///
    /// In the family's one block, which is what sharing a variant means: [`super::Jv001`] keeps
    /// mapper 36's CHR latch in all six of its numbers' savestates for the same reason, and
    /// four bytes is what a seven-board variant costs the six that do not use them. Every board
    /// here is new in this lot, so no existing savestate format moves (REQ-CORE-090).
    chr: [u8; 4],
    /// Which of the seven boards this is.
    board: LatchBoard,
}

impl SingleLatch {
    /// Mapper 29's CHR chip: "4x banks worth of full 8KB CHR ram" (nesdev, "INES Mapper 029").
    #[cfg(feature = "board-29")]
    pub(crate) const RET_CUFROM_CHR_RAM_SIZE: usize = 4 * 8 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory, board: LatchBoard) -> Self {
        let mut latch = Self {
            banks: windows_for(memory),
            register: 0,
            chr: [0; 4],
            board,
        };
        latch.update_banks();
        latch
    }

    /// The 32 KiB PRG bank and the 8 KiB CHR bank the latch selects, one arm per page.
    fn update_banks(&mut self) {
        // Each arm reads `self.register` itself rather than a binding above the `match`: with
        // `--features board-143` alone the only arm is a board with no register at all, and a
        // binding no arm reads is a warning (`-D warnings` is the gate every configuration of
        // this lot passes).
        match self.board {
            // "xxxx CCPP: ||++- Select 32 KB PRG ROM bank for CPU $8000-$FFFF, ++--- Select
            // 8 KB CHR ROM bank for PPU $0000-$1FFF" (nesdev, "INES Mapper 038"): four banks of
            // each, which is the page's 128 KiB of PRG and 32 KiB of CHR.
            #[cfg(feature = "board-38")]
            LatchBoard::BitCorp38 => {
                self.banks.map_prg_32k(i32::from(self.register & 0x03));
                self.banks
                    .map_chr_8k(i32::from((self.register >> 2) & 0x03));
            }
            // "BBBB BBBb: ++++ ++++- Select 8 KiB CHR bank at PPU $0000-$1FFF, ++++ +++--
            // Select 32 KiB PRG ROM bank at CPU $8000-$FFFF" (nesdev, "INES Mapper 107"). The
            // CHR field is the whole byte and the PRG field is it shifted one place right, which
            // is what the page's "bits 1-7 is used to switch both CHR and PRG banks
            // simultaneously" says — and **all eight bits are honoured**: "FCEUX implements only
            // the bottom 3 bits of the unified register, however MesenCE supports the full 8
            // bits", and a board with 32 KiB windows over the page's PRG-ROM needs more than
            // three.
            #[cfg(feature = "board-107")] // Q-147
            LatchBoard::Magic107 => {
                self.banks.map_prg_32k(i32::from(self.register >> 1));
                self.banks.map_chr_8k(i32::from(self.register));
            }
            // "An NROM circuit board with a simple form of protection" (nesdev, "INES Mapper
            // 143"): no bank register at all, so the whole board is one 32 KiB PRG window —
            // which an NROM-128 image shows twice, as `Nrom` does — and 8 KiB of unbanked CHR.
            // The latch this enum is named for is, on this one number, a *read*: see
            // [`SingleLatch::try_read_prg`].
            #[cfg(feature = "board-143")]
            LatchBoard::Tca143 => {
                self.banks.map_prg_32k(0);
                self.banks.map_chr_8k(0);
            }
            // Mapper 11's own register, which this board is "almost identical to the revision B
            // Color Dreams boards" of: `[CCCC PPPP]`, the low bits the 32 KiB PRG bank and the
            // high nibble the 8 KiB CHR bank. The two fields are read exactly as
            // [`super::ColorDreams`] reads them, because they are the same latch on the same
            // board; what mapper 144 adds is the resistor, and that is in
            // [`SingleLatch::write_register`].
            #[cfg(feature = "board-144")]
            LatchBoard::Agci144 => {
                self.banks.map_prg_32k(i32::from(self.register & 0x03));
                self.banks.map_chr_8k(i32::from(self.register >> 4));
            }
            // "$4100: [C... ....] - select 8 KiB CHR bank" (nesdev, "INES Mapper 145") and
            // "$8000: [C... ....]" (nesdev, "INES Mapper 149"). **The bit is bit 7**, which is
            // the one thing these two diagrams say and a sentence about "a CNROM clone" does
            // not. Neither page gives the board a PRG bank line: both are an NROM with a CHR
            // switch, so $8000-$FFFF is bank 0 of 32 KiB (which an NROM-128 image shows twice).
            #[cfg(feature = "board-145")]
            LatchBoard::Sachen145 => {
                self.banks.map_prg_32k(0);
                self.banks.map_chr_8k(i32::from(self.register >> 7));
            }
            #[cfg(feature = "board-149")]
            LatchBoard::Sachen149 => {
                self.banks.map_prg_32k(0);
                self.banks.map_chr_8k(i32::from(self.register >> 7));
            }
            // nesdev ("INES Mapper 190"), the three windows of its *Banks* section:
            //
            //     CPU $8000-$BFFF: 16 KiB switchable PRG ROM bank
            //     CPU $C000-$FFFF: 16 KiB PRG ROM, fixed to the FIRST bank
            //     PPU $0000-$1FFF: four 2 KiB switchable CHR ROM banks
            //
            // The fixed window is **bank 0** and not the last bank, which is what separates this
            // board from every UxROM-like one in this build; the four CHR registers hold whole
            // bytes, the page's `D~[CCCC CCCC]`.
            #[cfg(feature = "board-190")]
            LatchBoard::Zemina190 => {
                self.banks.map_prg_16k(0, i32::from(self.register & 0x0F));
                self.banks.map_prg_16k(2, 0);
                for slot in 0..4 {
                    self.banks.map_chr_2k(slot * 2, i32::from(self.chr[slot]));
                }
            }
            // nesdev ("INES Mapper 029"), whose *Banks* rows are
            //
            //     CPU $8000-$BFFF: 16 KiB switchable PRG ROM bank
            //     CPU $C000-$FFFF: 16 KiB PRG ROM, fixed to the last bank
            //     PPU $0000-$1FFF: 8 KiB switchable CHR RAM bank
            //
            // against the register `[...P PPCC]`: `CC` is the CHR-RAM bank over the page's
            // "4x banks worth of full 8KB CHR ram" and `PPP` the 16 KiB PRG bank over its
            // "8x 16KB PRG banks".
            #[cfg(feature = "board-29")]
            LatchBoard::RetCufrom => {
                self.banks
                    .map_prg_16k(0, i32::from((self.register >> 2) & 0x07));
                self.banks.map_prg_16k(2, -1);
                self.banks.map_chr_8k(i32::from(self.register & 0x03));
            }
        }
    }
}

impl Mapper for SingleLatch {
    banked!();

    fn mapper_number(&self) -> i32 {
        match self.board {
            #[cfg(feature = "board-38")]
            LatchBoard::BitCorp38 => 38,
            #[cfg(feature = "board-107")]
            LatchBoard::Magic107 => 107,
            #[cfg(feature = "board-143")]
            LatchBoard::Tca143 => 143,
            #[cfg(feature = "board-144")]
            LatchBoard::Agci144 => 144,
            #[cfg(feature = "board-145")]
            LatchBoard::Sachen145 => 145,
            #[cfg(feature = "board-149")]
            LatchBoard::Sachen149 => 149,
            #[cfg(feature = "board-190")]
            LatchBoard::Zemina190 => 190,
            #[cfg(feature = "board-29")]
            LatchBoard::RetCufrom => 29,
        }
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.register);
        writer.write_raw(&self.chr);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        // The windows themselves are in the block `Mapper::save_state` wrote before this one.
        self.register = reader.read_u8()?;
        reader.read_into(&mut self.chr)?;
        Ok(())
    }

    /// `memory` is written by the mapper-29 arm alone — the one board of the eight whose page
    /// states a mirroring — so with `board-29` off the parameter is unused and says so.
    #[cfg_attr(not(feature = "board-29"), allow(unused_variables))]
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.register = 0;
        self.chr = [0; 4];
        // "It is hard-wired for vertical mirroring" (nesdev, "INES Mapper 029"). Stated here
        // rather than left to a dump's header, because the page states it and byte 6 of an iNES
        // 1.0 header is what a dumper happened to set; the other seven boards have no such line
        // and keep whatever the header said.
        #[cfg(feature = "board-29")]
        if matches!(self.board, LatchBoard::RetCufrom) {
            memory.set_mirroring(crate::NametableMirroring::Vertical);
        }

        self.update_banks();
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    ///
    /// `memory` is read by the mapper-144 arm alone — the only board here that computes its own
    /// bus conflict — so with `board-144` off the parameter is unused and says so.
    #[cfg_attr(not(feature = "board-144"), allow(unused_variables))]
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        // **Mapper 190 is the one board here with two registers**, so it is decoded before the
        // single-latch `match` rather than inside it (nesdev, "INES Mapper 190"):
        //
        //     CHR-ROM bank select ($A000-$BFFF):  A~[101. .... .... ..bb]  D~[CCCC CCCC]
        //     PRG-ROM bank select ($8000-$9FFF, $C000-$DFFF):
        //                                         A~[1P0. .... .... ....]  D~[.... .PPP]
        //
        // `bb` picks which 2 KiB window the byte lands in, and **`P` is an address line**: A14,
        // so $8000-$9FFF writes the low eight 16 KiB banks and $C000-$DFFF the high eight. A13
        // set is neither register, which is why $E000-$FFFF reaches nothing.
        #[cfg(feature = "board-190")]
        {
            if matches!(self.board, LatchBoard::Zemina190) {
                if address & 0xE000 == 0xA000 {
                    self.chr[usize::from(address & 0x0003)] = value;
                    self.update_banks();
                } else if address & 0xA000 == 0x8000 {
                    let high = if address & 0x4000 == 0 { 0x00 } else { 0x08 };
                    self.register = high | (value & 0x07);
                    self.update_banks();
                }

                return;
            }
        }

        let latched = match self.board {
            // "Bank select ($7000-$7FFF)", plus the page's *Hardware* note: "Writes to
            // $F000-$FFFF are **very likely** to also trigger a bankswitch, but well behaved
            // code has no reason to" -- the 74138 "uses /ROMSEL and R/W on its enable inputs,
            // but /ROMSEL is 1 while M2 is low, not just while M2 is high and A15 is low". So
            // the decode is A14, A13 and A12 set and **A15 ignored**, which is the census's
            // "implemented as the same decode" and is recorded in Q-147 because the page says
            // "very likely" rather than asserting it.
            #[cfg(feature = "board-38")] // Q-147
            LatchBoard::BitCorp38 if address & 0x7000 == 0x7000 => Some(value & 0x0F),
            // The whole of $8000-$FFFF is the register, and the whole byte is the register.
            #[cfg(feature = "board-107")]
            LatchBoard::Magic107 if address >= 0x8000 => Some(value),
            // **Mapper 144's resistor, and the one of the page's three candidate formulas this
            // build takes.** "A 300 Ω resistor was added between CPU D0 and the combination of
            // mapper hardware and ROM. The effect of this is that only the ROM's least
            // significant bit always wins bus conflicts" (nesdev, "INES Mapper 144"), with
            //
            //     EffectiveData = (ROM[address] & (WrittenData | 1))
            //
            // which the page calls "most succinct" and which is the **same function** as its
            // first candidate, Nestopia 1.4.0's `(WrittenData & ROM[address]) | (ROM[address] &
            // 1)`: bit 0 comes from the ROM whatever the CPU drove, and bits 7-1 are the
            // ordinary Color Dreams AND. The third candidate, FCEUX 2.1.5's, "ignores writes to
            // 0x8000" -- a workaround for the one address *Death Race* writes, not a rule -- and
            // is deliberately not taken. Q-146 records all three and the census's own figure,
            // which none of them produces.
            //
            // **This board is therefore not on `registry::has_bus_conflicts`**: that function's
            // generic path ANDs the written value with the ROM byte in `Cartridge::cpu_write`,
            // which destroys bit 0 before the board could put the ROM's back. The byte is read
            // here instead, through the windows as they stand -- i.e. before this write moves
            // them, which is the order the hardware latches in.
            #[cfg(feature = "board-144")] // Q-146
            LatchBoard::Agci144 if address >= 0x8000 => {
                let rom = self
                    .banks
                    .read_prg(address, memory.rom().prg_rom(), memory.prg_ram())
                    .unwrap_or(0xFF);
                Some(rom & (value | 0x01))
            }
            // "Mask: $E100", so $4100-$41FF and its mirrors up to $5Fxx, and nothing in
            // $6000-$7FFF or above $8000 (nesdev, "INES Mapper 145").
            #[cfg(feature = "board-145")]
            LatchBoard::Sachen145 if address & 0xE100 == 0x4100 => Some(value & 0x80),
            // "Mask: $8000": **A15 alone**, so every address in $8000-$FFFF is the register --
            // which is the page's own account of the difference from 145, "moves the register to
            // overlap with PRG ROM and so has bus conflicts". The AND with the ROM byte is
            // `registry::has_bus_conflicts`'s, applied by `Cartridge::cpu_write` before this
            // function sees the value.
            #[cfg(feature = "board-149")]
            LatchBoard::Sachen149 if address & 0x8000 == 0x8000 => Some(value & 0x80),
            // "Range,Mask: $8000-FFFF, $8000": A15 alone again, so every address in $8000-$FFFF
            // is the register, and the five bits of `[...P PPCC]` are what it keeps. The page
            // gives this board no bus-conflict line, so it is not on
            // `registry::has_bus_conflicts` -- which is also why the whole written value arrives
            // here and not an AND of it with the ROM byte (nesdev, "INES Mapper 029").
            #[cfg(feature = "board-29")]
            LatchBoard::RetCufrom if address & 0x8000 == 0x8000 => Some(value & 0x1F),
            // Any address that is not this board's window -- and **every** address on mapper
            // 143, whose board has no write side at all. `Option` and not a `return` because
            // `--features board-143` alone leaves this `match` with that one arm, and a `match`
            // all of whose arms diverge makes everything after it unreachable (which is a
            // warning, and `-D warnings` is the gate every configuration of this lot passes).
            _ => None,
        };

        if let Some(latched) = latched {
            self.register = latched;
            self.update_banks();
        }
    }

    /// PRG-ROM above $8000, and **mapper 143's protection read** below it.
    ///
    /// nesdev ("INES Mapper 143") gives the board no write side at all and one read:
    ///
    /// ```text
    /// A:[010. ...1 ..DD DDDD] -> [xxii iiii]
    ///                ++-++++------||++-++++-- reads provide the logical inversion of DDDDDD
    ///                             ++--------- open bus
    /// ```
    ///
    /// **`DDDDDD` is the address, bits A5-A0** — not a value the game wrote, which it could not
    /// be on a board whose only chips are "a 74138 and a 74368". The census's own test asks for
    /// "the logical inversion of the *written* bits"; the page wins, and **Q-145** records it.
    /// The address pattern is mask $E100 against $4100 again, the window three other boards of
    /// this lot and [`super::Sachen72008`] share.
    ///
    /// The two bits the page calls open bus read as **0** and not as the last value on the bus,
    /// for the reason [`super::Jv001`]'s own note gives: this hook returns a whole byte and
    /// cannot see the CPU data bus, and no page asks a game to read past the bits its board
    /// drives.
    ///
    /// *Dancing Blocks* "relies extensively on the values of uninitialized memory", which the
    /// page mentions and which is a property of the console's 2 KiB of work RAM and not of this
    /// board — recorded here because the census asks that it be, and deliberately not a test.
    #[cfg(feature = "board-143")] // Q-145
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if matches!(self.board, LatchBoard::Tca143) && address & 0xE100 == 0x4100 {
            // The mask makes the truncation: `address & $3F` is six bits.
            #[allow(clippy::cast_possible_truncation)]
            let low = (address & 0x003F) as u8;
            return Some(!low & 0x3F);
        }

        None
    }
}
