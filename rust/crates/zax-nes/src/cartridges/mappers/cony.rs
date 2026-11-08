use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// Which of the page's five printed circuit boards a dump is (nesdev, "INES Mapper 083", the
/// variant list).
///
/// > iNES Mapper 83 and NES 2.0 Mapper 264 are used for games by Cony, also known as Yoko. The
/// > distinction between mappers 83 and 264 as well as NES 2.0 Submappers denote different printed
/// > circuit boards that all incorporate the **same chip-on-board ASIC**.
///
/// So one board with four parameters, on the [`super::Jv001`] and [`super::Namcot108`] precedent,
/// and the submapper picks the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConyVariant {
    /// **83.0**, "Standard single-game variant, 256 KiB of CHR-ROM maximum": *Street Fighter II
    /// Pro* and variants, *Fatal Fury 2*.
    Single,
    /// **83.1**, the **NC-24/82-07-17J** PCB, "Large CHR single-game variant, 512 KiB of CHR-ROM
    /// maximum": *World Heroes 2'*, *Super Blaster VII Turbo 28*.
    LargeChr,
    /// **83.3**, the **N-C44/82-08-24J** PCB, "4-in-1 128 KiB PRG+128 KiB+no PRG-RAM multicart
    /// variant": *1994 超強 20合1*.
    Multi4,
    /// **NES 2.0 mapper 264**, the **NC-14/82-05-26J** PCB, "Large CHR 2-in-1 multicart variant":
    /// *Mortal Kombat II/V Pro*, *Master Fighter VI*. UNIF MAPR `UNL-YOKO`.
    Yoko,
}

impl ConyVariant {
    /// The variant of `mapper_number` with `submapper`, or `None` for the one PCB this build
    /// refuses: **83.2** (Q-167).
    const fn resolve(mapper_number: i32, is_nes20: bool, submapper: i32) -> Option<Self> {
        if mapper_number == 264 {
            return Some(Self::Yoko);
        }

        match if is_nes20 { submapper } else { 0 } {
            0 => Some(Self::Single),
            1 => Some(Self::LargeChr),
            3 => Some(Self::Multi4),
            _ => None,
        }
    }

    /// Whether the PCB "connects CPU A11.A10 to the ASIC's A9..A8 inputs, so all addresses change
    /// accordingly" — mapper 264 alone.
    const fn remaps_addresses(self) -> bool {
        matches!(self, Self::Yoko)
    }

    /// Whether CHR-ROM is banked in **2 KiB** amounts through registers 0, 1, 6 and 7 rather than
    /// in 1 KiB amounts through all eight: "Mappers 83.1 and 264 increase the CHR-ROM size from
    /// 256 to 512 KiB by connecting CHR-ROM A10 to PPU A10 and CHR-ROM A11..A18 to the mapper
    /// ASIC's A10..A17 outputs".
    const fn banks_chr_in_2k(self) -> bool {
        matches!(self, Self::LargeChr | Self::Yoko)
    }

    /// The **inner PRG bank**, as a mask over 8 KiB bank numbers: the bits the banking mode and
    /// the four 8 KiB registers drive, with everything above them coming from the PRG base.
    ///
    /// "PRG Base A17 is only modified on mappers 83.0, 83.1 and 83.2, and not on 83.3 and 264. In
    /// other words, the former have a **256 KiB inner PRG bank**, and the latter a **128 KiB**
    /// one." Bit 0 of an 8 KiB bank number is PRG A13, so A13..A17 is `$1F` and A13..A16 is `$0F`.
    /// This mask is also what makes the page's "The `p` bit (PRG A17) has no effect on mappers
    /// 83.3 and 264" true without a second test.
    const fn inner_prg_mask(self) -> i32 {
        match self {
            Self::Single | Self::LargeChr => 0x1F,
            Self::Multi4 | Self::Yoko => 0x0F,
        }
    }
}

/// iNES mapper **83** and NES 2.0 mapper **264**: the Cony / Yoko chip-on-board ASIC
/// (REQ-CORE-102, MF-UNL-8 step 3).
///
/// Four to six original titles depending on how the 2-in-1 is counted: *Street Fighter II Pro*
/// and its variants and *Fatal Fury 2* (83.0), *World Heroes 2'* and *Super Blaster VII Turbo 28*
/// (83.1), *Mortal Kombat II/V Pro* and *Master Fighter VI* (264).
///
/// # The register map, all of it on one page
///
/// Mapper 83's addresses; mapper 264's are the same ASIC reached through
/// [`ConyVariant::remaps_addresses`].
///
/// | window | mask | what it is |
/// | --- | --- | --- |
/// | `$5000` (read) | `$F100` | the solder pad, `.... ..PP` |
/// | `$5100`-`$5103` | `A~[0101 ..01 .... 00AA]` | four bytes of scratch RAM, read and write |
/// | `$8000` | `$8300` | the PRG base, `PPPP PPPP` / `CCPP PPPP` |
/// | `$8100` | `$8300` | the mode register, `CD6P P.NN` |
/// | `$8200`-`$8201` | `$8301` | the 16-bit IRQ counter, LSB and MSB |
/// | `$8300`-`$8303` | `$8310` | four 8 KiB PRG banks, `...p PPPP` |
/// | `$8310`-`$8317` | `$8318` | eight 1 KiB CHR banks, `CCCC CCCC` |
/// | `$8318` | `$8318` | the IRQ clock source |
///
/// # The one PCB this build refuses, and why
///
/// **Submapper 2**, the N-C88-1603/83-08-19J "4-in-1 256 KiB PRG+256 KiB CHR+32 KiB PRG-NVRAM
/// multicart variant" of *Dragon Ball Party*. It is the only one of the five whose `$6000`-`$7FFF`
/// window is **PRG-NVRAM** rather than PRG-ROM — "the circuit board hardware around the ASIC
/// forces CPU $6000-$7FFF to PRG-RAM instead" — with its bank in the PRG base register's top two
/// bits, which is [`Mapper::prg_ram_window`]: a method [`super::IrqBoard`] does not forward, so a
/// board needing it would add a body to a `Board` dispatch for a 4-in-1 multicart that carries no
/// original title. It is refused at load with REQ-CORE-011's message, on MF-UNL-6's Q-157
/// precedent, and **Q-167** records it.
///
/// # Where the board sits
///
/// In [`super::IrqBoard`]: the ASIC holds a counter that pulls /IRQ, which is that enum's entry
/// condition. Every method the board needs is one the variant **already forwards** —
/// `try_read_prg` for the solder pad, the scratch RAM and the `$6000` PRG-ROM window,
/// `on_cpu_cycle` and `on_ppu_address_bus` for the two clock sources, `irq_asserted` — so `Board`
/// does not grow an arm.
///
/// **There is no ROM of either number under `assets-free/test-roms`** (§5 of
/// `docs/review/UNL-census.md`), so nothing in the campaign exercises it and every test is written
/// from the register table above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cony {
    banks: BankWindows,
    variant: ConyVariant,
    /// 83 or 264.
    mapper_number: i32,

    /// `$8000`, the PRG base. Eight bits on 83.0, 83.1 and 264; on 83.3 the top two are the outer
    /// CHR bank instead.
    prg_base: u8,
    /// `$8100`, the mode register: `CD6P P.NN`.
    mode: u8,
    /// `$8300`-`$8303`, five bits each (`...p PPPP`). Index 3 is the `$6000`-`$7FFF` window's.
    prg: [u8; 4],
    /// `$8310`-`$8317`, eight bits each.
    chr: [u8; 8],
    /// `$8318`: `$00` is CPU M2 and `$FF` is PPU A12. "It's not known which of the eight data bits
    /// is decisive", so the whole byte is kept and any non-zero value is A12 (Q-166).
    irq_source: u8,
    /// `$8200`-`$8201`, sixteen bits. "Writing directly changes the current counter value."
    irq_counter: u16,
    irq_enabled: bool,
    irq_pending: bool,
    /// The previous PPU address, for the **unfiltered** A12 rise of the PPU source — kept whatever
    /// the source is, so that selecting A12 mid-frame does not read a stale edge ([`JyCompany`]'s
    /// own reason).
    ///
    /// [`JyCompany`]: super::JyCompany
    last_ppu_address: u16,
    /// `$5100`-`$5103`, four bytes the game may read back.
    scratch: [u8; 4],
}

impl Cony {
    /// 8 KiB, the unit of a PRG window.
    const PRG_BANK_SIZE: usize = 0x2000;

    /// The solder pad `$5000` reads, `.... ..PP`.
    ///
    /// **A constant and a recorded gap** (Q-166). The page: "Several games display different names
    /// (*Street Fighter* versus *Street Blaster*) depending on the solder pad value. *Master
    /// Fighter VI* uses it to select NTSC or PAL timing routines. Yet others will just display a
    /// garbled title screen at some pad values, and the 'good' value is not always 0 or 3." A pad
    /// is cut on the board and **no field of any header records how**, so there is nothing for a
    /// dump to configure it from; census §7.2's "returns the configured value" has no
    /// configuration to read. Zero is the answer, which is the same choice [`JyCompany`]'s own
    /// `JJ.. ....` pad takes.
    ///
    /// [`JyCompany`]: super::JyCompany
    const SOLDER_PAD: u8 = 0x00;

    /// `$8100` bit 7, `C`: "1: IRQ counting enabled on next reload".
    const MODE_IRQ_ARM: u8 = 0x80;
    /// `$8100` bit 6, `D`: "0: IRQ counter increments, 1: IRQ counter decrements".
    const MODE_IRQ_DOWN: u8 = 0x40;
    /// `$8100` bit 5, `6`: "1: Map 8 KiB PRG-ROM bank to CPU $6000".
    const MODE_PRG_AT_6000: u8 = 0x20;

    pub(crate) fn new(memory: &CartridgeMemory, mapper_number: i32) -> Option<Self> {
        let header = memory.rom().header();
        let variant = ConyVariant::resolve(mapper_number, header.is_nes20, header.submapper)?;
        let mut board = Self {
            banks: windows_for(memory),
            variant,
            mapper_number,
            prg_base: 0,
            mode: 0,
            prg: [0; 4],
            chr: [0; 8],
            irq_source: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_pending: false,
            last_ppu_address: 0,
            scratch: [0; 4],
        };
        board.update_banks();
        Some(board)
    }

    /// The address the ASIC sees.
    ///
    /// "Only mapper 83's addresses given. Mapper 264 connects **CPU A11.A10 to the ASIC's A9..A8
    /// inputs**, so all addresses change accordingly", with the page's own five-row table:
    /// `$5100`→`$5400`, `$8100`→`$8400`, `$8200`→`$8800`, `$8300`→`$8C00`, `$8318`→`$8C18`.
    ///
    /// One expression reproduces all five: drop the CPU's own A11-A8 and put A11-A10 back two
    /// places down. The ASIC's A11-A10 are then zero, which every mask on the page leaves
    /// undecoded anyway.
    const fn asic_address(&self, address: u16) -> u16 {
        if self.variant.remaps_addresses() {
            (address & 0xF0FF) | ((address & 0x0C00) >> 2)
        } else {
            address
        }
    }

    /// The PRG base in 8 KiB units: the bank number the inner bank's bits sit inside.
    ///
    /// * 83.0, 83.1, 264: `PPPP PPPP` is "PRG A21..A14", so the byte shifted up one place.
    /// * 83.3: `CCPP PPPP` is "PRG A19..A14" in the low six bits, the top two being CHR.
    fn prg_base_8k(&self) -> i32 {
        let bits = match self.variant {
            ConyVariant::Multi4 => i32::from(self.prg_base & 0x3F),
            _ => i32::from(self.prg_base),
        };
        bits << 1
    }

    /// The outer CHR bank in 1 KiB units, from the PRG base register.
    ///
    /// 83.3 alone among the four this build serves: `CCPP PPPP`'s top two bits are "CHR-ROM
    /// A19..A18", and bit 8 of a 1 KiB bank number is CHR A18.
    fn outer_chr(&self) -> i32 {
        match self.variant {
            ConyVariant::Multi4 => i32::from(self.prg_base >> 6) << 8,
            _ => 0,
        }
    }

    /// `$8100` bits 4-3, the PRG banking mode — "`PRG Base` bits modification".
    const fn prg_mode(&self) -> u8 {
        (self.mode >> 3) & 0x03
    }

    /// The 8 KiB bank one of the four windows of `$8000`-`$FFFF` shows.
    ///
    /// The page's three modes, in its own words:
    ///
    /// * **0**: "PRG A17/A16..A14=1 if CPU A14=1 (UxROM)" — so `$8000`-`$BFFF` is the base's own
    ///   16 KiB and `$C000`-`$FFFF` the **last** 16 KiB of the inner bank, with CPU A13 supplying
    ///   PRG A13 throughout.
    /// * **1**: "None (NROM-128)" — nothing modifies the base, so CPU A14 reaches no PRG line and
    ///   the one 16 KiB bank appears at `$8000` and again at `$C000`.
    /// * **2** and **3** ("Same as 2"): "PRG A17/A16..A13 from 4x8 KiB PRG registers", with
    ///   "the $E000-$FFFF window … hard-wired to the last (inner) bank".
    fn prg_window(&self, slot: usize) -> i32 {
        let base = self.prg_base_8k();
        let mask = self.variant.inner_prg_mask();
        let low = i32::from(slot & 1 != 0);
        match self.prg_mode() {
            0 if slot < 2 => base | low,
            // A14 up to the top of the inner bank forced to 1, A13 still the CPU's.
            0 => base | (mask & !1) | low,
            1 => base | low,
            _ if slot == 3 => (base & !mask) | mask,
            _ => (base & !mask) | (i32::from(self.prg[slot]) & mask),
        }
    }

    /// The 8 KiB bank `$6000`-`$7FFF` shows when `$8100` bit 5 maps PRG-ROM there.
    ///
    /// "Register `$8303` applies to the `$6000`-`$7FFF` window, if enabled in the Mode register."
    /// The page conditions that on the mode register's bit 5 and **not** on the PRG banking mode,
    /// so the register's own composition is used in every mode; **Q-166** records that the page
    /// says nothing about what the window shows in modes 0 and 1, where the other three registers
    /// drive nothing.
    fn prg_window_6000(&self) -> i32 {
        let mask = self.variant.inner_prg_mask();
        (self.prg_base_8k() & !mask) | (i32::from(self.prg[3]) & mask)
    }

    fn update_banks(&mut self) {
        for slot in 0..BankWindows::PRG_WINDOWS {
            self.banks.map_prg_8k(slot, self.prg_window(slot));
        }

        if self.variant.banks_chr_in_2k() {
            // "CHR-ROM is banked in 2 KiB rather than 1 KiB amounts, using on the the four CHR
            // bank registers 0, 1, 6 and 7" -- the page's own address table, which is what the
            // rewiring of PPU A11 and A12 onto the ASIC's PA10-PA12 makes the chip see.
            for (window, register) in [0usize, 1, 6, 7].into_iter().enumerate() {
                self.banks
                    .map_chr_2k(window * 2, i32::from(self.chr[register]));
            }
        } else {
            let outer = self.outer_chr();
            for slot in 0..BankWindows::CHR_WINDOWS {
                self.banks
                    .map_chr_1k(slot, i32::from(self.chr[slot]) | outer);
            }
        }
    }

    /// `$8100` bits 1-0, `NN`, the **nametable arrangement**.
    ///
    /// The page gives both namings and they are each other's opposites, as [`JyCompany`]'s own note
    /// records: "0: Horizontal (**Vertical mirroring**)", "1: Vertical (**Horizontal mirroring**)",
    /// "2: Single screen 0", "3: Single screen 1". This core's [`NametableMirroring`] is the
    /// mirroring, which is the name in the page's own parentheses.
    ///
    /// [`JyCompany`]: super::JyCompany
    const fn mirroring(&self) -> NametableMirroring {
        match self.mode & 0x03 {
            0 => NametableMirroring::Vertical,
            1 => NametableMirroring::Horizontal,
            2 => NametableMirroring::SingleScreenA,
            _ => NametableMirroring::SingleScreenB,
        }
    }

    /// One clock of the selected source.
    ///
    /// "When counting is enabled and **the counter is not zero**, the counter is incremented or
    /// decremented according to the Mode register's `D` bit. If the counter is then zero,
    /// **counting is disabled**, and an IRQ is raised."
    ///
    /// All three clauses are here and each one is load-bearing: a counter sitting at zero does not
    /// move, the direction is the mode register's and not a reload's, and the firing turns the
    /// counter off so that a game must write the MSB again to re-arm it.
    fn tick_irq(&mut self) {
        if !self.irq_enabled || self.irq_counter == 0 {
            return;
        }

        self.irq_counter = if self.mode & Self::MODE_IRQ_DOWN == 0 {
            self.irq_counter.wrapping_add(1)
        } else {
            self.irq_counter.wrapping_sub(1)
        };
        if self.irq_counter == 0 {
            self.irq_enabled = false;
            self.irq_pending = true;
        }
    }

    /// Whether the counter is clocked by the PPU's A12 rather than by the CPU's M2.
    ///
    /// "The default value `$00` selects CPU M2 cycles as the source of the IRQ counter, and a value
    /// of `$FF` selects unfiltered PPU A12 rising edges as the source, clocking the counter eight
    /// times per rendered scanline. **It's not known which of the eight data bits is decisive.**"
    /// So any non-zero value is A12, which is the only reading that gives both documented values
    /// their documented meaning; **Q-166** records the gap.
    const fn clocks_on_ppu_a12(&self) -> bool {
        self.irq_source != 0
    }
}

impl Mapper for Cony {
    banked!();

    fn mapper_number(&self) -> i32 {
        self.mapper_number
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.prg_base);
        writer.write_u8(self.mode);
        writer.write_raw(&self.prg);
        writer.write_raw(&self.chr);
        writer.write_u8(self.irq_source);
        writer.write_i32(i32::from(self.irq_counter));
        writer.write_bool(self.irq_enabled);
        writer.write_bool(self.irq_pending);
        writer.write_i32(i32::from(self.last_ppu_address));
        writer.write_raw(&self.scratch);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.prg_base = reader.read_u8()?;
        self.mode = reader.read_u8()?;
        reader.read_into(&mut self.prg)?;
        reader.read_into(&mut self.chr)?;
        self.irq_source = reader.read_u8()?;
        self.irq_counter = u16::try_from(reader.read_i32()? & 0xFFFF).unwrap_or(0);
        self.irq_enabled = reader.read_bool()?;
        self.irq_pending = reader.read_bool()?;
        self.last_ppu_address = u16::try_from(reader.read_i32()? & 0xFFFF).unwrap_or(0);
        reader.read_into(&mut self.scratch)?;
        // The windows again from the registers, over the PRG-ROM loaded, which is the rule every
        // board of this library follows (REQ-CORE-091). The mirroring is the machine's and comes
        // back with its own block.
        self.update_banks();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.prg_base = 0;
        self.mode = 0;
        self.prg = [0; 4];
        self.chr = [0; 8];
        self.irq_source = 0;
        self.irq_counter = 0;
        self.irq_enabled = false;
        self.irq_pending = false;
        self.scratch = [0; 4];
        self.update_banks();
        memory.set_mirroring(self.mirroring());
    }

    fn irq_asserted(&self) -> bool {
        self.irq_pending
    }

    /// `$6000`-`$7FFF` is **PRG-ROM or open bus** on all four of this build's variants, never RAM:
    /// the page's *Banks* section gives the window "into 32 KiB PRG-NVRAM (Mapper 83.2)/into
    /// PRG-ROM (all others)", and 83.2 is refused. So `registry::board_memory_sizes` gives both
    /// numbers `prg_ram: Some(0)` and [`Cony::try_read_prg`] answers the window itself.
    fn prg_ram_enabled(&self) -> bool {
        false
    }

    /// `$8000`-`$FFFF` through the window table, the solder pad, the scratch RAM, and
    /// `$6000`-`$7FFF` when `$8100` bit 5 maps PRG-ROM there.
    ///
    /// **This and not [`Mapper::cpu_read_override`]**, which `IrqBoard` does not forward, for the
    /// reason [`JyCompany::try_read_prg`](super::JyCompany) gives at length: that dispatch is on
    /// every CPU read of an MMC5 game and this board has no read side effect at all.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if (0x6000..=0x7FFF).contains(&address) {
            if self.mode & Self::MODE_PRG_AT_6000 == 0 {
                return None;
            }

            let rom = memory.rom().prg_rom();
            if rom.is_empty() {
                return None;
            }

            let count = i32::try_from(rom.len() / Self::PRG_BANK_SIZE)
                .unwrap_or(1)
                .max(1);
            let bank = usize::try_from(self.prg_window_6000().rem_euclid(count)).unwrap_or(0);
            return Some(
                rom[(bank * Self::PRG_BANK_SIZE + usize::from(address - 0x6000)) % rom.len()],
            );
        }

        let asic = self.asic_address(address);
        // The solder pad, `(Address AND $F100)=$5000`.
        if asic & 0xF100 == 0x5000 {
            return Some(Self::SOLDER_PAD);
        }

        // The scratch RAM, from the page's own address pattern `A~[0101 ..01 .... 00AA]` --
        // A15-A12, A9-A8 and A3-A2 decoded, A1-A0 the index. **Its printed mask is a copy of the
        // PRG bank register's and cannot be right**: `$5100 AND $8310` is `$0100`, not `$8300`
        // (Q-166).
        if asic & 0xF30C == 0x5100 {
            return Some(self.scratch[usize::from(asic & 0x03)]);
        }

        None
    }

    /// Every register of the board. The write decode is the ASIC's, so mapper 264's addresses go
    /// through [`Cony::asic_address`] first and there is one decode for the two numbers.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        let asic = self.asic_address(address);
        if asic & 0xF30C == 0x5100 {
            self.scratch[usize::from(asic & 0x03)] = value;
            return;
        }

        if address < 0x8000 {
            return;
        }

        match asic & 0x8300 {
            // The PRG base, `(Address AND $8300)=$8000`.
            0x8000 => {
                self.prg_base = value;
                self.update_banks();
            }
            // The mode register, `(Address AND $8300)=$8100`.
            0x8100 => {
                self.mode = value;
                self.update_banks();
                memory.set_mirroring(self.mirroring());
            }
            // The 16-bit counter, `(Address AND $8301)=$8200`/`$8201`. "Writing directly changes
            // the current counter value. Writing to the **LSB acknowledges a pending IRQ**;
            // writing to the **MSB enables counting if bit C in the Mode register was previously
            // set**."
            0x8200 => {
                if asic & 0x01 == 0 {
                    self.irq_counter = (self.irq_counter & 0xFF00) | u16::from(value);
                    self.irq_pending = false;
                } else {
                    self.irq_counter = (self.irq_counter & 0x00FF) | (u16::from(value) << 8);
                    if self.mode & Self::MODE_IRQ_ARM != 0 {
                        self.irq_enabled = true;
                    }
                }
            }
            // `$8300`-`$830F` under `(Address AND $8310)=$8300`, `$8310`-`$8317` under
            // `(Address AND $8318)=$8310`, `$8318`-`$831F` under `(Address AND $8318)=$8318`:
            // three masks that differ in A4 and A3 alone, which is what the two tests below are.
            _ if asic & 0x10 == 0 => {
                self.prg[usize::from(asic & 0x03)] = value & 0x1F;
                self.update_banks();
            }
            _ if asic & 0x08 == 0 => {
                self.chr[usize::from(asic & 0x07)] = value;
                self.update_banks();
            }
            _ => self.irq_source = value,
        }
    }

    /// The CPU M2 source, which is `$8318`'s default value `$00`.
    fn on_cpu_cycle(&mut self, _memory: &mut CartridgeMemory) {
        if !self.clocks_on_ppu_a12() {
            self.tick_irq();
        }
    }

    /// The PPU A12 source: "**unfiltered** PPU A12 rising edges … clocking the counter eight times
    /// per rendered scanline".
    ///
    /// Unfiltered is the one word that keeps this board away from
    /// [`A12Watcher`](crate::cartridges::A12Watcher), exactly as it does the J.Y. ASIC: the MMC3's
    /// counter needs the filter to tell a real rise from the dips of the fetch pipeline, and this
    /// one counts every rise the bus shows.
    ///
    /// `last_ppu_address` is kept whatever the source is, so that a game selecting A12 mid-frame
    /// does not read a stale edge — which is why both numbers are in the two PPU watch lists of
    /// `cartridges::mod` whatever `$8318` holds.
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        if self.clocks_on_ppu_a12() && address & 0x1000 != 0 && self.last_ppu_address & 0x1000 == 0
        {
            self.tick_irq();
        }

        self.last_ppu_address = address;
    }
}
