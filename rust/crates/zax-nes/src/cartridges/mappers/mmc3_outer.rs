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
    /// Which of the eight 1 KiB CHR windows read the **CHR-RAM** chip rather than the CHR-ROM,
    /// one bit per window — `$xxx4`'s mixed mode, which the page says "replicate[s] the
    /// functionality of INES Mapper 074 and INES Mapper 194"
    /// ([`Mmc3Outer::chr_ram_selected`]).
    ///
    /// Derived state, refreshed by [`Mmc3Outer::refresh`] after every write and brought back by
    /// `load_registers` through the same call, so no savestate byte carries it (REQ-CORE-091) —
    /// the shape `Mmc3Board::chr_ram_windows` already has for the same question. **Zero on every
    /// cartridge that never writes `$xxx4`**, which is what keeps the two fetch paths below off
    /// the CHR-RAM branch entirely.
    chr_ram_windows: u8,
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
        // Q-157
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
            chr_ram_windows: 0,
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
    // Q-156
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
    // Q-158
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

    /// The low seven bits of a 1 KiB CHR bank — CHR A10 to A16 — in the banking mode the two
    /// mode bits of `$xxx3` select.
    ///
    /// * **MMC3 mode**: the chip's own bank register, [`Mmc3Outer::chr_window_register`].
    /// * **GNROM mode** (`$10` and `$50`): "CHR A16, A15, A14, and A13 come from MMMM controls.
    ///   CHR A12, A11, and A10 come from PPU A12, A11, and A10" — so the window index *is* the
    ///   three low bits and `$xxx2`'s low nibble the four above, with `TTT` **ANDed** into its top
    ///   three. See [`Mmc3Outer::gnrom_chr_offset`].
    /// * **"Weird" MMC3 mode** (`$40`): "The second half of each of the 2 KiB CHR banks is
    ///   replaced with bank 0, and the LSB of MMC3 registers 0 and 1 work (instead of being
    ///   replaced by PPU A10)." So the even window of each 2 KiB pair shows the register's whole
    ///   byte, bit 0 included, and the odd one shows bank 0. The four 1 KiB registers are not
    ///   concerned: the page's sentence is about the 2 KiB ones.
    ///
    /// **The weird rule does not apply in GNROM mode**, which is the reading mode `$50` needs:
    /// "Combine the rules mentioned above for 'Normal' GNROM mode with 'Weird' MMC3 mode. (These
    /// descriptions should not be contradictory.)" — and they are not, because in GNROM mode the
    /// MMC3's CHR registers reach nothing at all, so a rule about their LSB has nothing to act on.
    fn chr_inner_bank(&self, window: usize) -> u8 {
        if Self::gnrom_mode(self.outer[3]) {
            // The window index is PPU A12-A10, which is what a GNROM-style board puts straight
            // through.
            return u8::try_from(window).unwrap_or(0) | (self.gnrom_chr_offset() << 3);
        }

        let (value, bank) = self.chr_window_register(window);
        if self.outer[3] & 0x40 != 0 {
            let high = usize::from(self.mmc3.select() & 0x80 != 0) * 4;
            if window >= high && window < high + 4 {
                return if (window - high) & 1 == 0 { value } else { 0 };
            }
        }

        bank
    }

    /// `$xxx2`'s GNROM CHR offset: `MMMM` (CHR A16, A15, A14, A13) with the `TTT` bitmask applied.
    ///
    /// nesdev gives `TTT` as the "GNROM CHR bitmask (**ANDed** with CHR A16, A15, A14 in MMMM)",
    /// so the mask covers the field's top three bits and A13 — `MMMM`'s own bit 0 — is never
    /// masked. The AND is the page's word and it is taken literally: the register can only ever
    /// clear a line of its own offset, which is what a multicart menu needs to keep a game inside
    /// its own CHR window.
    fn gnrom_chr_offset(&self) -> u8 {
        let register = self.outer[2];
        register & 0x0F & (((register >> 3) & 0x0E) | 0x01)
    }

    /// The low four bits of an 8 KiB PRG bank in **GNROM mode** — CHR A13 to A16 of the page's
    /// `PRG A16...13 from QQ, L, R, CPU A14, A13` — for window `slot`.
    ///
    /// nesdev, banking mode `$10`: "PRG A16 and A15 come from the `QQ` controls. When in 16 KiB
    /// PRG mode (`L` control), PRG A14 comes from the `R` control. Otherwise, it comes from CPU
    /// A14." PRG A13 is the CPU's own line in both, so the window index supplies it.
    ///
    /// Everything above A16 is still the mask-and-offset composition of MMC3 mode, which is the
    /// page's own note on this mode: "The CHR and PRG mask registers STILL AFFECT THIS MODE,
    /// allowing weird splicing of MMC3 banking (with 128 KiB granularity) overlaying GNROM-style
    /// banking."
    fn gnrom_prg_low(&self, slot: usize) -> i32 {
        let slot = i32::try_from(slot).unwrap_or(0);
        let r1 = i32::from(self.outer[1]);
        let r3 = i32::from(self.outer[3]);
        // `L`: 1 is a 32 KiB bank with PRG A14 = CPU A14, 0 a 16 KiB bank with A14 from `R`.
        let a14 = if r1 & 0x02 != 0 {
            (slot >> 1) & 0x01
        } else {
            (r3 >> 1) & 0x01
        };
        (slot & 0x01) | (a14 << 1) | (((r3 >> 2) & 0x03) << 2)
    }

    /// Whether `$xxx4` puts the MMC3 bank register value `value` on the **CHR-RAM** chip.
    ///
    /// nesdev: `$xxx4` is `UUUU UUUV`, bit 0 selecting "mixed CHR-RAM/CHR-ROM mode" and bits 7-1
    /// the "MMC3 bank register value **D1-D7** that select CHR-RAM rather than CHR-ROM", with the
    /// note that "by writing the appropriate value to D1-D7, the functionality of INES Mapper 074
    /// and INES Mapper 194 is replicated".
    ///
    /// That note is what pins the comparison to `value >> 1`: mapper 74's RAM is CHR banks
    /// `$08-$09` and mapper 194's `$00-$01`, each a 2 KiB pair whose two banks differ in D0 alone.
    /// So the register names a pair, and `$xxx4 = $09` *is* mapper 74.
    ///
    /// **Only in MMC3 mode**, because the register is specified against an MMC3 bank register
    /// value and GNROM mode has none; the page's own "only meaningful on cartridges that mount
    /// both CHR-ROM and CHR-RAM" is the other half of its scope.
    const fn chr_ram_selected(&self, value: u8) -> bool {
        self.outer[4] & 0x01 != 0 && value >> 1 == self.outer[4] >> 1
    }

    /// Whether `$xxx3` bit 5 has opened the second PRG-RAM window.
    ///
    /// nesdev gives the bit as "1: **Also** enable PRG RAM in `$5000-$5FFF`", and that is the
    /// whole of what the page says about it — no mapping for a 4 KiB window over an 8 KiB chip,
    /// and no game named that uses it. **Q-159** records both readings this build had to take:
    ///
    /// * **a COOLBOY only.** On a MINDKIDS the register file is at `$5000-$5FFF` under mask
    ///   `$F007`, which decodes the *whole* 4 KiB page, so there is no room for RAM there and the
    ///   bit cannot mean what it says. The page's *WRAM usage* section points the same way: the
    ///   window it worries about is the COOLBOY's.
    /// * **CPU A0-A12 straight through**, so `$5000-$5FFF` shows the chip's second 4 KiB — the
    ///   same bytes as `$7000-$7FFF`. That is what the ASIC asserting a chip select over an
    ///   unchanged address bus does, and it is the only reading that needs no extra wiring.
    ///
    /// The MMC3's `$A001` still gates it, which "also" implies: the bit adds a window, it does not
    /// add an enable.
    // Q-159
    const fn extra_ram_window(&self) -> bool {
        self.port == 0x6000 && self.outer[3] & 0x20 != 0
    }

    /// The index in PRG-RAM of an address in the second window, or `None` when there is no RAM.
    fn extra_ram_offset(memory: &CartridgeMemory, address: u16) -> Option<usize> {
        let length = memory.prg_ram().len();
        if length == 0 {
            None
        } else {
            Some((usize::from(address) & 0x1FFF) % length)
        }
    }

    /// A CHR fetch on a board whose `$xxx4` has put some windows on the CHR-RAM chip.
    ///
    /// `#[inline(never)]`, which is `Mmc3Board::chr_ram_read`'s own measured shape: that body
    /// inlined took `Ppu2C02::background_fetch` from 271 instructions to 583 and cost three games
    /// on other boards 3.4 M to 5.4 M over 300 frames (MF-6a2 step 3). The price is a `callq` per
    /// fetch for a cartridge in mixed mode and nothing for anybody else — on this board not even
    /// that, because [`Mmc3Outer::ppu_peek`] tests the flag byte first and a cartridge that never
    /// writes `$xxx4` never arrives here.
    #[inline(never)]
    fn chr_ram_read(&self, memory: &CartridgeMemory, address: u16) -> u8 {
        let index = usize::from(address) & 0x1FFF;
        let window = index / BankWindows::CHR_WINDOW_SIZE;
        let ram = memory.chr_ram();
        if ram.is_empty() {
            return 0;
        }

        let offset = self.chr_ram_offset(window) + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        ram[super::super::wrap_offset(offset, ram.len())]
    }

    /// The bit of [`Mmc3Outer::chr_ram_windows`] that `address`'s 1 KiB window carries.
    fn window_bit(address: u16) -> u8 {
        1u8 << ((usize::from(address) & 0x1FFF) / BankWindows::CHR_WINDOW_SIZE)
    }

    /// Where in the CHR-RAM chip window `window` starts: the bank number the ASIC drives, with
    /// the chip's own missing address lines standing in for the modulo, as
    /// `Mmc3Board::chr_ram_offset` does for the TQROM's family.
    fn chr_ram_offset(&self, window: usize) -> usize {
        let bank = usize::try_from(self.chr_bank(window)).unwrap_or(0);
        bank * BankWindows::CHR_WINDOW_SIZE
    }

    /// The 1 KiB CHR bank number window `window` shows, outer bits included.
    ///
    /// CHR A10-A16 are [`Mmc3Outer::chr_inner_bank`]. **CHR A17** is the mask bit `A` of
    /// `$xxx0`: clear and the line is the inner bank's own bit 7, set and it is the single `D` bit
    /// beside it — a *replacement* and not an OR, which is what "CHR mask (CHR A17 from 0: MMC3;
    /// 1: alternate)" says. **CHR A18 and A19** are the `CC` pair, the same two bits that are
    /// PRG A23 and A24; the page notes they are the CHR offset "on CHR-ROM-having boards", which
    /// is the JTH-813 and its 1 MiB.
    fn chr_bank(&self, window: usize) -> i32 {
        let inner = self.chr_inner_bank(window);
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
    /// The four 8 KiB PRG bank numbers the windows show, in the mode `$xxx3` selects.
    ///
    /// nesdev, banking mode `$40` ("weird" MMC3): "If MMC3 PRG ROM bank mode is 'normal'
    /// (8+8+16F), then the PRG banks at `0xC000` and `0xE000` are fixed to 8 KiB **bank 0**,
    /// instead of `0xFE` and `0xFF`. Supposedly this effect is **completely ignored** when MMC3
    /// PRG ROM bank mode is inverted (8F+8+8+8F)." Both sentences are here, and the second is why
    /// the rule is applied *before* the mask rather than to the windows: bank 0 is a bank of the
    /// chip's own numbering, so the outer offset still carries it.
    fn prg_banks(&self) -> [i32; BankWindows::PRG_WINDOWS] {
        let mask = self.prg_mmc3_mask();
        let bits = self.prg_offset() & !mask;
        let mut prg = self.mmc3_prg_banks();
        if self.outer[3] & 0x40 != 0 && self.mmc3.select() & 0x40 == 0 {
            prg[2] = 0;
            prg[3] = 0;
        }

        let gnrom = Self::gnrom_mode(self.outer[3]);
        std::array::from_fn(|slot| {
            let bank = (prg[slot] & mask) | bits;
            if gnrom {
                (bank & !0x0F) | self.gnrom_prg_low(slot)
            } else {
                bank
            }
        })
    }

    /// Recomputes all twelve windows from the MMC3's registers and this board's six, and with
    /// them the derived flag byte of `$xxx4`'s mixed mode.
    ///
    /// Called after **every** write either side takes, on [`super::Somari::refresh`]'s own
    /// idempotence argument — the chip keeps its registers and the composition is recomputed from
    /// them, so nothing can be counted twice and the board does not have to remember what it last
    /// applied. The MMC3's own `update_banks` runs first on its own account (a `$8000-$FFFF`
    /// write goes through [`Mmc3`] before this is called) and writes the plain MMC3 layout; every
    /// window it wrote is overwritten here.
    fn refresh(&mut self) {
        let prg = self.prg_banks();
        let mut chr_ram_windows = 0u8;
        let chr: [i32; BankWindows::CHR_WINDOWS] = std::array::from_fn(|window| {
            if !Self::gnrom_mode(self.outer[3])
                && self.chr_ram_selected(self.chr_window_register(window).0)
            {
                chr_ram_windows |= 1u8 << window;
            }
            self.chr_bank(window)
        });
        self.chr_ram_windows = chr_ram_windows;

        let Some(banks) = self.mmc3.banks_mut() else {
            return;
        };
        for (slot, bank) in prg.iter().enumerate() {
            banks.map_prg_8k(slot, *bank);
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
        // `$xxx3` bit 5's second window; see [`Mmc3Outer::extra_ram_window`] and Q-159. On a
        // MINDKIDS it is never open, so the port test below still sees $5000-$5FFF.
        if (0x5000..=0x5FFF).contains(&address) && self.extra_ram_window() {
            if self.prg_ram_writable() {
                if let Some(offset) = Self::extra_ram_offset(memory, address) {
                    memory.prg_ram_mut()[offset] = value;
                }
            }

            return true;
        }

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

    /// The second PRG-RAM window of `$xxx3` bit 5, and the MMC3's own decode for everything else.
    ///
    /// The window is read **here** and not through `Cartridge`'s cached base, which only covers
    /// $6000-$7FFF; see [`Mmc3Outer::extra_ram_window`] and Q-159.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if (0x5000..=0x5FFF).contains(&address) && self.extra_ram_window() && self.prg_ram_enabled()
        {
            if let Some(offset) = Self::extra_ram_offset(memory, address) {
                return Some(memory.prg_ram()[offset]);
            }
        }

        self.mmc3.try_read_prg(memory, address)
    }

    /// `None`, the body the MMC5 and the RAMBO-1 already give: the fetches clock the A12 counter.
    fn chr_view<'m>(&self, _memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        None
    }

    /// `None` in mixed mode, which is `Mmc3Board::chr_peek_view`'s own answer for the CHR-RAM
    /// family and for the same reason: "a `ChrView` is one slice plus eight offsets and has no way
    /// to say 'this window is the other chip'".
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if self.chr_ram_windows == 0 {
            self.mmc3.chr_peek_view(memory)
        } else {
            None
        }
    }

    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, fetch: ChrFetch) -> u8 {
        if self.chr_ram_windows & Self::window_bit(address) == 0 {
            return self.mmc3.ppu_peek(memory, address, fetch);
        }

        self.chr_ram_read(memory, address)
    }

    /// A PPU write: the CHR-RAM chip of `$xxx4`'s mixed mode, and the MMC3's own path otherwise —
    /// which on a CHR-RAM-only cartridge is the window table and on a CHR-ROM one drops the write.
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if self.chr_ram_windows & Self::window_bit(address) == 0 {
            self.mmc3.ppu_write(memory, address, value);
            return;
        }

        let index = usize::from(address) & 0x1FFF;
        let window = index / BankWindows::CHR_WINDOW_SIZE;
        let offset = self.chr_ram_offset(window) + (index & (BankWindows::CHR_WINDOW_SIZE - 1));
        let ram = memory.chr_ram_mut();
        if !ram.is_empty() {
            let length = ram.len();
            ram[super::super::wrap_offset(offset, length)] = value;
        }
    }

    fn on_ppu_address_bus(&mut self, memory: &mut CartridgeMemory, address: u16, clock: i64) {
        self.mmc3.on_ppu_address_bus(memory, address, clock);
    }
}
