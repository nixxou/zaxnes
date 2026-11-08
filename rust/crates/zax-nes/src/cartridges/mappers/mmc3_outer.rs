use super::{Mapper, Mmc3};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};

/// NES 2.0 mapper **268**: the 聚芯 **AA6023** ASIC, labelled **SMD132** and **SMD133** when
/// packaged — an MMC3 clone with **six outer bank registers** above the chip's own, which take
/// it from the MMC3's 512 KiB to 32 MiB of PRG-ROM (MF-UNL-6, REQ-CORE-102).
///
/// nesdev ("NES 2.0 Mapper 268"): the number "denotes cartridges using the AA6023 ASICs …
/// MMC3 clones that can address up to 32 MiB of PRG-ROM, 256 KiB of CHR-ROM or -RAM, and 8 KiB
/// of WRAM". The UNIF board names are **COOLBOY**, whose register window is at $6000-$6FFF, and
/// **MINDKIDS**, whose window is at $5000-$5FFF; bit 0 of the NES 2.0 submapper is which.
///
/// Twelve original titles, which is why `docs/review/UNL-census.md` §7.2 gives the number a lot
/// of its own: nine Chinese RPGs — *大话水浒*, *楚留香新传*, *魔道劫*, *口袋妖怪: 叶绿* (vols
/// 1-3), three *最终幻想* titles and *神奇宝贝* — and three modern licensed reissues, *Gravity
/// Trooper Metal Storm* and *Holy Diver* (Retro-Bit) and *Quest Forge* (Piko Interactive).
/// Behind them sit the multicarts the chipset was made for, *Super 402-in-1* and its kind.
///
/// # Which of the page's thirteen PCB codes this board serves, and why the rest are refused
///
/// §7.2's own warning is that "the page lists thirteen PCB codes … and its documentation is
/// **partial**. A single implementation covering all thirteen is a guess; the lot should
/// implement what the page documents, name in the code which PCB codes it is known to serve, and
/// leave the rest refusing."
///
/// The page indexes those codes by **submapper**, in six groups, and the groups do not share a
/// register layout: `$xxx0` has four distinct bit diagrams on that page and `$xxx1` three. So
/// this board is **submappers 0 and 1** — one layout, one port pair, and the six PCB codes that
/// carry every one of the twelve titles:
///
/// | submapper | PCB | the page's note |
/// | ---: | --- | --- |
/// | 0 / 1 | **COOLBOY**, **MINDKIDS**, **KT-008** | "CHR-RAM only, max. 32 MiB PRG-ROM" |
/// | 0 / 1 | **YH2018A** | "CHR-RAM only, max. 64 MiB PRG-ROM" — see [`Mmc3Outer::prg_a25`] |
/// | 0 / 1 | **JTH-813** | "CHR-ROM only, max. 8 MiB PRG-ROM, max. 1 MiB CHR-ROM" |
/// | 0 / 1 | **KT-???** | "CHR-ROM+CHR-RAM mixed functionality", which is `$xxx4`'s mixed mode |
///
/// Submappers **2 to 11** — the GH2009_V01 / SMD173C pair on the later AA6023B die, the
/// KP-6022 and LD622D, the J-852C with its two PRG chips, the SMD72A_V5S_V01 with its
/// write-protectable CHR-RAM, and the SMD172C-L1 with its single-screen mirroring — are
/// **refused at load**: [`Mmc3Outer::new`] answers `None` for them and `Cartridge::create` turns
/// that into REQ-CORE-011's `UnsupportedMapper`. Running them on this layout would put the PRG
/// offset bits on the wrong lines (submappers 2/3 reorder `$xxx1` and move the window to
/// $7000-$7FFF) rather than refuse a dump honestly. **Q-157** records the choice.
///
/// **Mapper 224 is not added**, as §7.2 says: "the same chipset per its own page … but 224 has
/// no title, so this lot does not add it."
///
/// # The register file
///
/// Six registers under **mask `$F007`**, so the whole of the 4 KiB window decodes them: every
/// address whose low three bits are 0-5 is a register, and `$xxx8`-`$xxxF` repeats `$xxx0`-`$xxx7`
/// through the page. The diagrams below are the page's own, for submappers 0 and 1.
///
/// ```text
/// $xxx0  ABCC DEEE   EEE  PRG offset (PRG A19, A18, A17)
///                    D    alternate CHR A17
///                    CC   PRG offset (PRG A24, A23); CHR offset (CHR A19, A18)
///                    B    PRG mask (PRG A17 from 0: MMC3; 1: offset)
///                    A    CHR mask (CHR A17 from 0: MMC3; 1: alternate)
/// $xxx1  GHIJ KKLS   S    SC0 bit
///                    L    GNROM mode bank PRG size (1: 32 KiB; 0: 16 KiB)
///                    JKK  PRG offset (in order: PRG A20, A22, A21)
///                    I    PRG mask (PRG A20 from 0: offset; 1: MMC3)
///                    H    PRG mask (PRG A19 from 0: offset; 1: MMC3)
///                    G    PRG mask (PRG A18 from 0: MMC3; 1: offset)
/// $xxx2  STTT MMMM   MMMM CHR offset for GNROM mode (CHR A16, A15, A14, A13)
///                    TTT  GNROM CHR bitmask (ANDed with CHR A16, A15, A14 in MMMM)
///                    S    GNROM CHR bitmask lock
/// $xxx3  N.ZP QQRS   S    SC1 bit
///                    QQR  PRG offset for GNROM mode (PRG A16, A15, A14)
///                    P    1: GNROM mode; 0: MMC3 mode
///                    Z    1: also enable PRG RAM in $5000-$5FFF
///                    bits 6 and 4 together: the banking mode
///                    N    lockout
/// $xxx4  UUUU UUUV   V    0: CHR-ROM only; 1: mixed CHR-RAM/CHR-ROM mode
///                    UUU  MMC3 bank register value D1-D7 that select CHR-RAM
/// $xxx5  xxxW XXYY   W    0: normal MMC3 oversize mode; 1: mega-UNROM mode
/// ```
///
/// **The two mask fields read in opposite directions and that is not a transcription slip**: the
/// page gives A17 and A18 "from 0: MMC3; 1: offset" and A19 and A20 "from 0: offset; 1: MMC3".
/// Out of reset every register is zero, so A17 and A18 come from the MMC3 and A19 and A20 from an
/// offset of zero — **the board powers on as a 512 KiB MMC3**, which is what makes a COOLBOY
/// multicart's menu work and why "a great number of Chinese cartridges use the ASIC as a simple
/// MMC3 clone and thus are usually found set to INES Mapper 004". A game above 512 KiB has to
/// open the lines first. [`Mmc3Outer::prg_mmc3_mask`] is that table.
///
/// # Where this board sits in `Board`
///
/// In [`super::IrqBoard`], **boxed**: it *is* the MMC3's board, counter included, so it answers
/// **yes** to [`Mapper::irq_asserted`], which is that enum's entry condition — and it holds an
/// [`Mmc3`] plus its own registers, which inline would set the enum's size, the reason
/// [`super::Mmc3Scramble`] is boxed (MF-6b step 1 priced one byte of displacement in that payload
/// at +0.009 % to +0.027 % on eleven of lot PR-1's thirteen ROMs).
///
/// # The IRQ is the MMC3A's, and the page says so in one line
///
/// > AA6023's IRQ operator follows MMC3 "NEC" behavior, i.e. a latch value of zero disables IRQ.
///
/// That is [`Mmc3`]'s `irq_revision_a`, which mapper 4 reaches through NES 2.0 submapper 4 and
/// which this board sets at load through `Mmc3Board::force_irq_revision_a` — the same call
/// mappers 114 and 182 make for the same sentence in their own page's vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc3Outer {
    /// The chip itself. Every `$8000-$FFFF` register reaches it unchanged; what leaves are the
    /// bank numbers its registers produce, which [`Mmc3Outer::refresh`] recomputes.
    mmc3: Mmc3,
    /// `$xxx0` to `$xxx5`, as written.
    outer: [u8; 6],
    /// `$6000` on a COOLBOY and `$5000` on a MINDKIDS: the high nibble of the register window,
    /// which bit 0 of the submapper chooses. See [`Mmc3Outer::new`].
    port: u16,
}

impl Mmc3Outer {
    /// The register window's own mask: A12-A15 and A0-A2, which is the page's "Masks: `$F007`".
    const PORT_MASK: u16 = 0xF000;

    /// How many of the eight addresses the mask decodes are registers.
    const REGISTERS: usize = 6;

    pub(crate) fn new(memory: &CartridgeMemory, mapper: i32) -> Option<Self> {
        let header = memory.rom().header();
        let submapper = if header.has_submapper() {
            header.submapper
        } else {
            0
        };
        // The page's own table: "right → $6000-$6FFF … NES 2.0 Submapper bit 0 = 0 … UNIF MAPR
        // COOLBOY", "left → $5000-$5FFF … bit 0 = 1 … MINDKIDS". Submappers 2 to 11 are refused;
        // the type's own note and Q-157 have the reason.
        let port = match submapper {
            0 => 0x6000,
            1 => 0x5000,
            _ => return None,
        };

        let mut board = Self {
            // 268 and not 4, which is the tightening `Mmc3Kof97::new` and `Mmc3Txc189::new`
            // already make: `Mmc3Board::new` reads a submapper only for mapper 4, so no image of
            // this number can be taken for an MMC6, an MMC3A or a hard-wired-mirroring MMC3C —
            // and this number's submapper means something else entirely.
            mmc3: Mmc3::new(memory, mapper),
            outer: [0; Self::REGISTERS],
            port,
        };
        // "AA6023's IRQ operator follows MMC3 'NEC' behavior, i.e. a latch value of zero
        // disables IRQ."
        board.mmc3.force_irq_revision_a();
        board.refresh();
        Some(board)
    }

    /// Whether the lockout bit is holding the registers.
    ///
    /// nesdev: `$xxx3` bit 7 is "Lockout (prevent further writes to all registers but the one at
    /// `$xxx2`, **only works in MMC3 mode**)", and the two GNROM banking modes each carry the
    /// line "Lockout DOES NOT work in this mode".
    ///
    /// **§7.2 step 3 asks for the lockout "implemented as a no-op with the reason in a comment"
    /// and the page refutes that**, so MF-UNL-2's rule applies and the page wins (**Q-156**).
    /// The page's *WRAM usage* section is the refutation: on a COOLBOY the register window is the
    /// lower half of the WRAM window, so a game that wants both sets the registers up with the
    /// MMC3's WRAM disabled, **sets the lockout**, and only then enables the WRAM —
    ///
    /// ```text
    /// LDA #$00
    /// STA $A001   ; Disable WRAM
    /// ; initialize PRG/CHR offsets and masks
    /// LDA #$80
    /// STA $6003   ; Enable Lockout
    /// STA $A001   ; Enable WRAM
    /// ```
    ///
    /// — and the page's own conclusion for the mode where the bit does nothing is that "it is
    /// impossible to have [a] GNROM game use WRAM in the `$6000-$6FFF` (submapper 0) setting, as
    /// any WRAM write would inadvertently change the outer bank registers". A no-op lockout
    /// would do exactly that to every one of the nine Chinese RPGs: the first byte a save routine
    /// wrote to `$6003` would re-bank the cartridge.
    const fn locked_out(&self) -> bool {
        self.outer[3] & 0x80 != 0 && !Self::gnrom_mode(self.outer[3])
    }

    /// Whether `$xxx3` selects **GNROM mode**: its bit 4, the page's "1: GNROM mode; 0: MMC3
    /// mode". Together with bit 6 ("weird" mode) it is the banking mode — `$00`, `$10`, `$40` or
    /// `$50` — which is why the page names the modes by the value of those two bits.
    const fn gnrom_mode(register3: u8) -> bool {
        register3 & 0x10 != 0
    }

    /// PRG **A25**, the one line that takes the chipset past 32 MiB.
    ///
    /// nesdev's *64 MiB mapping*: "The **YH2018A** PCB uses the SC0 (`$6001` bit 0) and SC1
    /// (`$6003` bit 0) bits to address two halves of 32 MiB for a total of 64 MiB. To do so, the
    /// ASIC's SL0 input pin is connected to GND, and the ASIC's OA0 output is connected to the
    /// ROM chip's PRG A25 input."
    ///
    /// | `$xxx1.0` | `$xxx3.0` | the page's meaning |
    /// | :-: | :-: | --- |
    /// | 0 | 0 | "PRG A25=PRG A0, i.e. bytes at even addresses come from the first 32 MiB and bytes at odd addresses come from the second 32 MiB half" |
    /// | 0 | 1 | "PRG A25=1, i.e. second 32 MiB half" |
    /// | 1 | x | "PRG A25=0, i.e. first 32 MiB half" |
    ///
    /// **The first row is not a bank and this core has no way to say it**: a [`BankWindows`]
    /// window is a byte offset plus a length, and "A25 follows A0" interleaves two chips byte by
    /// byte. It is also the **reset** state, and the page says what the hardware's users do about
    /// that: "either the code at the reset vector must be carefully placed with bytes alternating
    /// between the two 32 MiB halves, or (as the existing multicarts do) **must be duplicated in
    /// both 32 MiB halves**." So this board answers the **first** half for that row — the half
    /// every known dump duplicates its boot code into — and **Q-158** records it with the
    /// observation that would close it.
    ///
    /// The line is driven on every image and not only on a declared YH2018A, because no header
    /// field distinguishes one: on an image of 32 MiB or less the bit is above the PRG-ROM's own
    /// lines and [`BankWindows::map_prg_8k`] wraps it away, which is the same thing the hardware's
    /// missing address line does.
    const fn prg_a25(&self) -> i32 {
        if self.outer[1] & 0x01 == 0 && self.outer[3] & 0x01 != 0 {
            1
        } else {
            0
        }
    }

    /// Which bits of an 8 KiB PRG bank number the **MMC3** still drives, as a mask.
    ///
    /// A13-A16 always; A17 and A18 unless their mask bit is set; A19 and A20 only *when* theirs
    /// is. A21 to A25 are never the MMC3's — the chip has eight bank bits and they stop at A20 —
    /// so they are left out of the mask and [`Mmc3Outer::prg_offset`] drives them.
    ///
    /// See the type's own note on why the four mask bits do not all read the same way round.
    const fn prg_mmc3_mask(&self) -> i32 {
        let mut mask = 0x0F;
        if self.outer[0] & 0x40 == 0 {
            mask |= 0x10;
        }
        if self.outer[1] & 0x80 == 0 {
            mask |= 0x20;
        }
        if self.outer[1] & 0x40 != 0 {
            mask |= 0x40;
        }
        if self.outer[1] & 0x20 != 0 {
            mask |= 0x80;
        }
        mask
    }

    /// The outer PRG offset, as an 8 KiB bank number: PRG A17 to A25 in their own bit positions,
    /// gathered from the two registers that scatter them.
    ///
    /// The page's own reading of the whole field is "The combined register of CCKKJEEE could be
    /// thought of as a way to select one of 256 different 128 KiB PRG MMC3 games, or CCKK
    /// selecting one of 16 different 2 MiB MMC3 games", which is this number written the other
    /// way round.
    fn prg_offset(&self) -> i32 {
        let r0 = i32::from(self.outer[0]);
        let r1 = i32::from(self.outer[1]);
        // EEE: A19, A18, A17 — three bits in their own order, so the field lands whole.
        ((r0 & 0x07) << 4)
            // J: A20.
            | (((r1 >> 4) & 0x01) << 7)
            // KK: "in order: PRG A20, A22, A21", so bit 3 is A22 and bit 2 is A21.
            | (((r1 >> 2) & 0x01) << 8)
            | (((r1 >> 3) & 0x01) << 9)
            // CC: A24, A23.
            | (((r0 >> 4) & 0x01) << 10)
            | (((r0 >> 5) & 0x01) << 11)
            | (self.prg_a25() << 12)
    }

    /// The four 8 KiB PRG bank numbers the MMC3 underneath selects, in window order.
    ///
    /// `$8000` bit 6 swaps the switchable `R6` window with the fixed one, which is the chip's own
    /// rule; what is **not** the chip's own rule is that the two fixed windows are `$FE` and
    /// `$FF` here rather than "the last two banks of the image". On this board they have to be:
    /// the MMC3 drives A13-A20 and the outer register drives A21 upwards, so the fixed banks are
    /// the last two of the **selected** 2 MiB window and not of a 32 MiB image. It is also the
    /// page's own vocabulary — banking mode `$40` says the two become "8 KiB bank 0, instead of
    /// `0xFE` and `0xFF`".
    fn mmc3_prg_banks(&self) -> [i32; BankWindows::PRG_WINDOWS] {
        let r6 = i32::from(self.mmc3.bank_register(6));
        let r7 = i32::from(self.mmc3.bank_register(7));
        if self.mmc3.select() & 0x40 == 0 {
            [r6, r7, 0xFE, 0xFF]
        } else {
            [0xFE, r7, r6, 0xFF]
        }
    }

    /// The MMC3 bank **register value** that drives 1 KiB CHR window `window`, and the 8-bit
    /// bank number that window shows — which are the same byte except on the two 2 KiB
    /// registers, where the page's own "ignores bit 0" makes the pair `v & $FE` and `v | $01`.
    ///
    /// `$8000` bit 7 is which half of the pattern tables the 2 KiB registers drive, exactly as
    /// `Mmc3Board::chr_bank_of_window` reads it. The register value is returned beside the bank
    /// because `$xxx4`'s mixed mode is specified against the **register value** and not against
    /// the window.
    fn chr_window_register(&self, window: usize) -> (u8, u8) {
        let high = usize::from(self.mmc3.select() & 0x80 != 0) * 4;
        if window >= high && window < high + 4 {
            let value = self.mmc3.bank_register((window - high) >> 1);
            (value, (value & 0xFE) | u8::from((window - high) & 1 != 0))
        } else {
            let value = self.mmc3.bank_register(window + high - 2);
            (value, value)
        }
    }

    /// The 1 KiB CHR bank number window `window` shows, outer bits included.
    ///
    /// CHR A10-A16 are the MMC3's seven low bank bits. **CHR A17** is the mask bit `A` of
    /// `$xxx0`: clear and the line is the MMC3's own bit 7, set and it is the single `D` bit
    /// beside it — a *replacement* and not an OR, which is what "CHR mask (CHR A17 from 0: MMC3;
    /// 1: alternate)" says. **CHR A18 and A19** are the `CC` pair, the same two bits that are
    /// PRG A23 and A24; the page notes they are the CHR offset "on CHR-ROM-having boards", which
    /// is the JTH-813 and its 1 MiB.
    fn chr_bank(&self, window: usize) -> i32 {
        let (_, inner) = self.chr_window_register(window);
        let r0 = self.outer[0];
        let a17 = if r0 & 0x80 == 0 {
            i32::from(inner) & 0x80
        } else {
            (i32::from(r0 >> 3) & 0x01) << 7
        };
        (i32::from(inner) & 0x7F)
            | a17
            | ((i32::from(r0 >> 4) & 0x01) << 8)
            | ((i32::from(r0 >> 5) & 0x01) << 9)
    }

    /// Recomputes all twelve windows from the MMC3's registers and this board's six.
    ///
    /// Called after **every** write either side takes, on [`super::Somari::refresh`]'s own
    /// idempotence argument — the chip keeps its registers and the composition is recomputed from
    /// them, so nothing can be counted twice and the board does not have to remember what it last
    /// applied. The MMC3's own `update_banks` runs first on its own account (a `$8000-$FFFF`
    /// write goes through [`Mmc3`] before this is called) and writes the plain MMC3 layout; every
    /// window it wrote is overwritten here.
    fn refresh(&mut self) {
        let mask = self.prg_mmc3_mask();
        let bits = self.prg_offset() & !mask;
        let prg = self.mmc3_prg_banks();
        let chr: [i32; BankWindows::CHR_WINDOWS] =
            std::array::from_fn(|window| self.chr_bank(window));

        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };
        for (slot, bank) in prg.iter().enumerate() {
            banks.map_prg_8k(slot, (bank & mask) | bits);
        }
        for (slot, bank) in chr.iter().enumerate() {
            banks.map_chr_1k(slot, *bank);
        }
    }

    /// A write to one of the six outer registers.
    ///
    /// `#[inline(never)]`, the shape MF-6a2 step 3 measured and [`super::Somari`]'s
    /// `write_supervisor` carries: this is one arm of a `Board::intercept_cpu_write` dispatch
    /// `NesBus::write_internal` reaches, and a cartridge of this board pays a `callq` per register
    /// write where nobody else pays anything.
    #[inline(never)]
    fn write_outer(&mut self, index: usize, value: u8) {
        // "Lockout … prevent further writes to all registers but the one at $xxx2."
        if self.locked_out() && index != 2 {
            return;
        }

        // "GNROM CHR bitmask lock (1: prevents subsequent updates to S or T bits)": once bit 7 of
        // `$xxx2` is set, a write to that register still moves `MMMM` and no longer `S` or `TTT`.
        if index == 2 && self.outer[2] & 0x80 != 0 {
            self.outer[2] = (self.outer[2] & 0xF0) | (value & 0x0F);
        } else {
            self.outer[index] = value;
        }

        self.refresh();
    }
}

impl Mapper for Mmc3Outer {
    fn mapper_number(&self) -> i32 {
        268
    }

    fn banks(&self) -> Option<&BankWindows> {
        self.mmc3.banks()
    }

    fn banks_mut(&mut self) -> Option<&mut BankWindows> {
        self.mmc3.banks_mut()
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        self.mmc3.save_registers(writer);
        writer.write_raw(&self.outer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.mmc3.load_registers(reader)?;
        reader.read_into(&mut self.outer)?;
        // `chr_ram_windows` comes back here: it is derived from the two register files.
        self.refresh();
        Ok(())
    }

    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.mmc3.reset(memory);
        // The page gives the six registers no power-on value but does say of the two bits it
        // names that "after reset, both bits are cleared", so zero is what the whole file comes
        // back to — and zero is the state in which this board is a 512 KiB MMC3.
        self.outer = [0; Self::REGISTERS];
        self.refresh();
    }

    fn irq_asserted(&self) -> bool {
        self.mmc3.irq_asserted()
    }

    fn prg_ram_enabled(&self) -> bool {
        self.mmc3.prg_ram_enabled()
    }

    fn prg_ram_writable(&self) -> bool {
        self.mmc3.prg_ram_writable()
    }

    /// The register window, and the WRAM behind it.
    ///
    /// **The write is consumed and the RAM written here rather than left to
    /// `Cartridge::cpu_write`'s own PRG-RAM branch**, which is the MMC6's shape and this board
    /// needs it for the same reason: on a COOLBOY the window *is* the lower half of
    /// $6000-$7FFF, so both the register and the RAM see the write, and that branch returns
    /// without re-reading the ROM windows a register write has just moved.
    ///
    /// The RAM's own gate is the MMC3's `$A001`, untouched — which is exactly what makes the
    /// page's *WRAM usage* sequence work: with the WRAM disabled only the registers latch, and
    /// after the lockout only the RAM does. See [`Mmc3Outer::locked_out`].
    fn intercept_cpu_write(
        &mut self,
        memory: &mut CartridgeMemory,
        address: u16,
        value: u8,
    ) -> bool {
        if address & Self::PORT_MASK != self.port {
            return false;
        }

        let index = usize::from(address & 0x07);
        if index < Self::REGISTERS {
            self.write_outer(index, value);
        }

        // $6000-$6FFF of a COOLBOY is the register window *and* the first half of the WRAM.
        if (0x6000..=0x7FFF).contains(&address) && self.prg_ram_writable() {
            let ram = memory.prg_ram_mut();
            if !ram.is_empty() {
                let length = ram.len();
                ram[usize::from(address - 0x6000) % length] = value;
            }
        }

        true
    }

    /// **The `address < 0x8000` guard is deliberately not the first line**, which is
    /// [`super::Mmc3Txc189::write_register`]'s own note — except that here the guard is all there
    /// is below `$8000`: the six registers are decoded by
    /// [`Mmc3Outer::intercept_cpu_write`] before this is reached.
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if address < 0x8000 {
            return;
        }

        self.mmc3.write_register(memory, address, value);
        self.refresh();
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give: the fetches clock the A12 counter.
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        self.mmc3.chr_peek_view(memory)
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        self.mmc3.ppu_peek(memory, address, fetch)
    }

    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        self.mmc3.ppu_write(memory, address, value);
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }
}
