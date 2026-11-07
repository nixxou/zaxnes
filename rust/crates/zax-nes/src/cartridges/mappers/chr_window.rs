use super::{windows_for, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory};
use crate::state::{InvalidState, StateReader, StateWriter};

/// Which of the boards of [`ChrWindowBoard`] a cartridge is.
///
/// **A runtime field and not a `const` parameter**, on [`super::SingleLatch`]'s and
/// [`super::Jv001`]'s precedent: the boards of this variant share every port address, every mask
/// and the whole CHR mechanism, and what they differ by is reached on a CPU write to $5000-$53FF
/// and nowhere else — "the rarest bus event there is". One monomorphisation per number would have
/// bought one arm per number in every inner `match` of [`super::DecodeBoard`]; one field buys one
/// arm for all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChrWindowKind {
    /// Mapper 163, the 南晶 (Nánjīng) FC-001 (MF-UNL-5 step 1).
    #[cfg(feature = "board-163")]
    NanjingFc001,
    /// Mapper 162, the 外星 (Wàixīng) FS304 (MF-UNL-5 step 2).
    #[cfg(feature = "board-162")]
    WaixingFs304,
}

/// iNES mappers **163 and 162**: the 南晶 (Nánjīng) **FC-001** and the 外星 (Wàixīng) **FS304**,
/// the two boards that select a 4 KiB CHR-RAM page **from the PPU address lines** rather than from
/// a register (REQ-CORE-102, MF-UNL-5 steps 1 and 2).
///
/// Two pages, nesdev's **"INES Mapper 163"** and **"INES Mapper 162"**, and they describe one
/// register window with one mechanism in it. Both *Banks* sections are the same four lines, and
/// they are the whole of each board's memories:
///
/// * **"CPU $6000-$7FFF: 8 KiB unbanked PRG-RAM, battery-backed"**;
/// * **"CPU $8000-$FFFF: 32 KiB switchable PRG-ROM bank"**, whose bank number is composed out of
///   three write ports and two mode bits;
/// * **"PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM bank, 4 KiB can be automatically switched"**;
/// * **"Nametable mirroring: hard-wired"**, so the header decides it and no register does.
///
/// The two are different silicon from different publishers that share a register map because one
/// copied the other: 162's *Notes* say its 南晶 titles perform "a protection check that verifies
/// the behavior described above", i.e. the 外星 board's, and each page gives the same
/// one-register test for telling a dump of one from a dump of the other (below).
///
/// # The automatic CHR-RAM switch, which is what puts these boards in their lot
///
/// Bit 7 of the port at $5000 is the enable, and both pages write the same three lines against
/// it:
///
/// ```text
/// D~7654 3210
///   ---------
///   C... ....
///   +--------- Enable automatic 4 KiB CHR-RAM switch: when PPU A13=0 (pattern table) ...
///              0: CHR A12=PPU A12 (disable auto-switch)
///              1: CHR A12=PPU A9 latched on last rise of PPU A13 (enable auto-switch)
/// ```
///
/// With the switch off the 8 KiB chip is flat: CHR A12 is the fetch's own A12, which is what
/// [`BankWindows::map_chr_8k`] gives. With it on, **CHR A12 stops following the fetch** and becomes
/// a latch — PPU A9 as it stood on the last *rise* of PPU A13 — so both halves of the pattern-table
/// space show the same 4 KiB page. PPU A13 rises when the PPU address enters $2000-$3FFF, which is
/// a nametable or attribute fetch, and A9 of such an address is the half of the nametable being
/// read: $2000-$21FF is rows 0-15, $2200-$23FF rows 16-29 and the attributes. Hence the sentence
/// the page uses to describe the effect:
///
/// > Automatic 4 KiB CHR-RAM switch means that the left pattern table is used for the the top half
/// > of any nametable, and the right pattern table for the bottom half of any nametable,
/// > **regardless of the scroll position**.
///
/// So this is [`super::OekaKids`]'s mechanism with a different edge and a different bit, and these
/// boards go in **both** of §4.3's latches for the reason that one does: the rise of A13 is an
/// address change, and `Cartridge::on_ppu_address_bus` is the only hook that sees one. A bare
/// `$2006` write that takes the address from below $2000 to $2000 or above is such a rise and
/// latches too — this core cannot tell it apart from a fetch's, and neither can the board, which
/// has only the address lines to go on. [`Cartridge::watches_ppu_reads`] and
/// [`Cartridge::watches_ppu_bus_without_read`] therefore both carry 162 and 163.
///
/// [`Cartridge::watches_ppu_reads`]: super::super::Cartridge
/// [`Cartridge::watches_ppu_bus_without_read`]: super::super::Cartridge
///
/// The price is the `None` [`super::DecodeBoard`] already answers to `Mapper::chr_view` for every
/// board in it: `Ppu2C02::run_visible_span` takes a whole line of dots without telling the
/// cartridge anything, and a board that must hear every address cannot afford that path. Both
/// boards have **no IRQ**, so [`Mapper::ppu_irq_horizon`] keeps `Mapper`'s own `Some(i64::MAX)`
/// default — their /IRQ line cannot change through anything the PPU does, because they have none.
///
/// # The ports, under the pages' own masks
///
/// | port | mask | mapper 163 | mapper 162 |
/// | --- | --- | --- | --- |
/// | `$5000`, write | `$FF00` | `C... PPPP` — the CHR switch and PRG A18..A15 | `C... PPpq` — the CHR switch, PRG A18..A17, and `p`/`q` for A16/A15 behind the mode bits |
/// | `$5100`, write | `$FF01` on 163, `$FF00` on 162 | the **feedback** latch, `A0` picking the action | `.... ..p.` — "PRG A15 if $5300.0=0" |
/// | `$5200`, write | `$FF00` | `.... ..PP` — PRG A20..A19 | the same |
/// | `$5300`, write | `$FF00` | `.... .A?B` — `B` swaps D0 and D1, `A` sources PRG A15/A16 | `.... .A.B` — an A16 mode bit and an A15 mode bit |
/// | `$5500-$5501`, read | `$F300` | `.... .F..` — the inverted feedback bit | the board has no readable register |
///
/// Mask `$FF00` reads A15-A8 and nothing below, so each write port repeats through its own 256
/// bytes: `$5000-$50FF` is the first and `$5300-$53FF` the last. **Only 163's feedback write port
/// reads A0 as well**, which is its `$FF01`.
///
/// `Cartridge::cpu_write` hands these boards every write outside $6000-$7FFF, so the decode needs
/// no [`Mapper::intercept_cpu_write`] body; and because every port is below $8000 the PRG-ROM
/// never drives the bus during one, which is why neither page gives either board bus conflicts.
///
/// # Mapper 163's PRG composition, its mode register and its boot bank
///
/// ```text
/// $5000:  C... PPPP        $5200:  .... ..PP        $5300:  .... .A?B
///              ++++- A18..A15             ++- A20..A19          | +- 0: do not swap D0,D1 on
///                                                               |     writes to $5000-$5200
///                                                               |    1: swap D0,D1 on them
///                                                               +--- 0: PRG A15/A16=11b
///                                                                    1: PRG A15/A16 from $5000
/// ```
///
/// A15 is the least significant bit of a 32 KiB bank number, so the bank is
/// `A15 | A16<<1 | A17<<2 | A18<<3 | A19<<4 | A20<<5` — six bits, 2 MiB of addressable PRG-ROM.
/// "All registers are initialized to $00 on reset", and "**because reset clears the A bit, games
/// will boot in 32 KiB PRG-ROM bank #3**": with `A` clear the two low bits are forced to `11b` and
/// the four others are zero, which is bank 3. That sentence is what pins this reading, and the
/// page's *Notes* confirm it from the other side — "Mapper 163 cartridges should be dumped with
/// $5300=$04 to obtain the correct PRG bank order", `$04` being the `A` bit alone.
///
/// **The swap bit is applied to the two bank ports and not to the feedback port**, which is a
/// reading and therefore **Q-155**: the page's words are "swap D0,D1 on writes to `$5000-$5200`",
/// a range that textually contains `$5100`, but `$5100`'s two data bits are the feedback latch's
/// `F` and `E` and not bank lines, and the only purpose the page gives the bit is "to obtain the
/// correct PRG bank order".
///
/// # The feedback latch
///
/// ```text
/// write $5100-$5101, mask $FF01            read $5500-$5501, mask $F300
///   D~.... .F.E   A~.... .... .... ...A      D~.... .F..
///         | |                        A              +--- Inverted value of latched F bit
///         | +- A=0: Value latched
///         |    A=1: Flip the latched F bit when E=1
///         +--- A=0: Value latched
///              A=1: Ignored, previously-latched value either kept or flipped
/// ```
///
/// So a write to `$5100` latches both bits, a write to `$5101` flips the latched `F` when the
/// written `E` is set and ignores the written `F`, and a read answers with `F` **inverted** in bit
/// 2. What the games do with it, in the page's own words, is verify that inversion — which is the
/// whole of the protection.
///
/// The read mask is `$F300` and not `$FF00`: `$5500 & $F300` is `$5100`, so the port answers at
/// every address of $5000-$5FFF whose A9 is clear and whose A8 is set — `$51xx`, `$55xx`, `$59xx`
/// and `$5Dxx`. The feedback *write* port is one of those four, which is one hardware decode read
/// two ways. Bits other than 2 are not driven by the chip and this board answers them zero;
/// nothing in the page says what sits there.
///
/// # Mapper 162's PRG composition and its two mode bits
///
/// ```text
/// $5000:  C... PPpq      $5100:  .... ..p.      $5200:  .... ..PP      $5300:  .... .A.B
///              || |               +- A15 if            ++- A20..A19             | +- A15 mode
///              || +- A15 if          $5300.0=0                                  +--- A16 mode
///              ||    $5300.0=1 and $5300.2=1
///              |+--- A16 if $5300.2=1
///              ++--- A18..A17
/// ```
///
/// with the two mode bits spelled out by the page as
///
/// > `B` — PRG A15 mode: 0: PRG A15=$5100.1; 1: PRG A15=1 if `A`=0, PRG A15=$5000.0 if `A`=1
/// >
/// > `A` — PRG A16 mode: 0: PRG A16=1; 1: PRG A16=$5000.1
///
/// and the page's own check of that arithmetic: "Games boot in PRG bank #2 since reset initializes
/// all registers to $00" — `B`=0 gives A15 = `$5100.1` = 0, `A`=0 gives A16 = 1, so the bank is 2.
///
/// The board has **no swap bit and no feedback register**. `$5300` bit 0 means something else
/// entirely here, which is why `ChrWindowBoard::bank_port_value` tests the kind and not the bit
/// alone — without that test the FS304 would scramble every bank it writes — and `$5100` is a
/// bank line, which is why `Mapper::try_read_prg` is behind `board-163`.
///
/// # The one-register test that tells a dump of one number from a dump of the other
///
/// Each page's *Notes* give it, from its own side:
///
/// > (162) To ensure that a cartridge to be dumped is really INES Mapper 162, write $04 to $5300
/// > and verify that $5100.1 **changes** the PRG bank.
/// >
/// > (163) To verify that the cartridge is really mapper 163, you can rule out INES Mapper 162 by
/// > writing $04 to $5300 and verifying that $5100.1 does **not** change the PRG bank.
///
/// `$04` sets `A` and clears `B`. On 162 that is "PRG A15=$5100.1", so the bank moves; on 163
/// `$5100` carries no bank line at all, so it does not. This build reproduces both sides, and
/// `req_core_102_the_two_chr_window_boards_differ_by_the_dumpers_own_test` is that pair of
/// sentences as one test.
///
/// # Mapper 163's submapper 1 is deferred, and the page says what it is
///
/// "Submapper 1 denotes a PCB that mounts the same ADPCM chip used in NES 2.0 Mapper 419 with the
/// PCB code NJ-YUYIN0106 … used only on the 真 Samurai Spirits - 武士道列传: 侍魂 (NJ-033)
/// cartridge." §7.2 of the census scopes this lot to **submapper 0 only** and defers the chip to a
/// later expansion-audio lot; [`Mapper::expansion_audio_output`] therefore keeps its `None`, and
/// a submapper-1 image runs as the board without the chip. One cartridge is concerned, and it is
/// not one of the six category-A titles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrWindowBoard {
    banks: BankWindows,
    /// Which board, and so which PRG arithmetic and what `$5100` means.
    kind: ChrWindowKind,
    /// `$5000`, whole: bit 7 is the CHR switch and the low nibble the bank lines.
    port_5000: u8,
    /// `$5100`: on mapper 163 the **feedback** latch, `.... .F.E`.
    port_5100: u8,
    /// `$5200`: `.... ..PP`, PRG A20..A19.
    port_5200: u8,
    /// `$5300`, the mode register.
    port_5300: u8,
    /// PPU A9 as it stood on the last rise of PPU A13, which is CHR A12 while the switch is on.
    /// Zero at power-on, nothing having driven the bus yet.
    latched_a9: u8,
    /// The previous PPU address, the other side of the edge on A13. In the save state because
    /// nothing derives it, which is `OekaKids`'s own reason for keeping the same byte pair.
    last_address: u16,
}

impl ChrWindowBoard {
    /// "PRG Bank Low/CHR-RAM Switch ($5000, write), Mask: $FF00", and the ports beside it. The
    /// mask reads A15-A8, so the four write ports are $50xx, $51xx, $52xx and $53xx.
    const PORT_MASK: u16 = 0xFF00;

    /// "Feedback Read ($5500-$5501, read), Mask: $F300" — A15-A12 and A9-A8, which `$5500 & $F300`
    /// makes `$5100`: every $5xxx address with A9 clear and A8 set.
    const FEEDBACK_READ_MASK: u16 = 0xF300;
    const FEEDBACK_READ: u16 = 0x5100;

    /// "8 KiB unbanked PRG-RAM, battery-backed", the page's *Banks* section. Whether a host
    /// persists it is the dump's own battery bit, as it is for every board here.
    pub(crate) const PRG_RAM_SIZE: usize = BankWindows::PRG_WINDOW_SIZE;

    /// "8 KiB unbanked CHR-RAM bank, 4 KiB can be automatically switched", likewise. The figure is
    /// the one REQ-CORE-012's default gives an image with no CHR-ROM; it is stated because the
    /// page states it and no iNES 1.0 header field can.
    pub(crate) const CHR_RAM_SIZE: usize = 8 * 1024;

    pub(crate) fn new(memory: &CartridgeMemory, kind: ChrWindowKind) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            kind,
            port_5000: 0,
            port_5100: 0,
            port_5200: 0,
            port_5300: 0,
            latched_a9: 0,
            last_address: 0,
        };
        board.update_banks();
        board
    }

    /// The 32 KiB PRG bank the ports compose, which is the one piece of arithmetic the boards of
    /// this variant do not share.
    fn prg_bank(&self) -> i32 {
        // `.... ..PP` is PRG A20..A19, two bits above A18, i.e. bits 5-4 of the bank number.
        let high = i32::from(self.port_5200 & 0x03) << 4;
        match self.kind {
            // `PPPP` is A18..A15 straight, unless the mode register's `A` ($5300 bit 2) is clear
            // -- then "PRG A15/A16=11b" and only A18..A17 come from the port.
            #[cfg(feature = "board-163")]
            ChrWindowKind::NanjingFc001 => {
                let low = if self.port_5300 & 0x04 == 0 {
                    (self.port_5000 & 0x0C) | 0x03
                } else {
                    self.port_5000 & 0x0F
                };
                high | i32::from(low)
            }
            // 162: A18..A17 are `$5000` bits 3-2 whatever the mode is, and A16 and A15 each have
            // a mode bit of their own.
            #[cfg(feature = "board-162")]
            ChrWindowKind::WaixingFs304 => {
                let a17_a18 = i32::from(self.port_5000 & 0x0C);
                // "PRG A16 mode: 0: PRG A16=1; 1: PRG A16=$5000.1" -- `A` is $5300 bit 2.
                let a16 = if self.port_5300 & 0x04 == 0 {
                    1
                } else {
                    i32::from((self.port_5000 >> 1) & 0x01)
                };
                // "PRG A15 mode: 0: PRG A15=$5100.1; 1: PRG A15=1 if A=0, PRG A15=$5000.0 if
                // A=1" -- `B` is $5300 bit 0.
                let a15 = if self.port_5300 & 0x01 == 0 {
                    i32::from((self.port_5100 >> 1) & 0x01)
                } else if self.port_5300 & 0x04 == 0 {
                    1
                } else {
                    i32::from(self.port_5000 & 0x01)
                };
                high | a17_a18 | (a16 << 1) | a15
            }
        }
    }

    /// The CHR windows, which is where the automatic switch lands.
    ///
    /// With bit 7 of `$5000` clear the chip is flat and [`BankWindows::map_chr_8k`] says so. With
    /// it set, **CHR A12 is the latch and not the fetch's own A12**, so both 4 KiB windows show the
    /// same page — which is exactly what "the left pattern table is used for the top half of any
    /// nametable, and the right pattern table for the bottom half" means once the fetch's own
    /// pattern-table select has been taken out of the picture.
    fn update_chr(&mut self) {
        if self.port_5000 & 0x80 == 0 {
            self.banks.map_chr_8k(0);
        } else {
            let page = i32::from(self.latched_a9);
            self.banks.map_chr_4k(0, page);
            self.banks.map_chr_4k(4, page);
        }
    }

    fn update_banks(&mut self) {
        self.banks.map_prg_32k(self.prg_bank());
        self.update_chr();
    }

    /// Mapper 163's `$5300` bit 0: "Swap D0,D1 on writes to $5000-$5200." Applied to the two
    /// **bank** ports and not to the feedback port, which is Q-155.
    ///
    /// The swap is on the **write** path and not on the composition, which is where the hardware
    /// puts it: the two data pins are exchanged on the way into the latch, so flipping `$5300` bit
    /// 0 afterwards does not re-swap a bank already written.
    ///
    /// **Mapper 162 has no swap bit at all** and its own `$5300` bit 0 is the A15 *mode*, so the
    /// kind test is not an optimisation: without it the FS304 would scramble every bank it writes.
    // With `board-162` alone no board here has a swap bit, so the body reads no field of `self`.
    #[cfg_attr(not(feature = "board-163"), allow(clippy::unused_self))]
    fn bank_port_value(&self, value: u8) -> u8 {
        #[cfg(feature = "board-163")]
        if self.kind == ChrWindowKind::NanjingFc001 && self.port_5300 & 0x01 != 0 {
            return (value & 0xFC) | ((value & 0x01) << 1) | ((value >> 1) & 0x01);
        }

        value
    }

    /// Mapper 163's feedback read: "Inverted value of latched F bit" in bit 2, and nothing else
    /// driven.
    #[cfg(feature = "board-163")]
    fn feedback_read(&self) -> u8 {
        !self.port_5100 & 0x04
    }
}

impl Mapper for ChrWindowBoard {
    banked!();

    fn mapper_number(&self) -> i32 {
        match self.kind {
            #[cfg(feature = "board-163")]
            ChrWindowKind::NanjingFc001 => 163,
            #[cfg(feature = "board-162")]
            ChrWindowKind::WaixingFs304 => 162,
        }
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.port_5000);
        writer.write_u8(self.port_5100);
        writer.write_u8(self.port_5200);
        writer.write_u8(self.port_5300);
        writer.write_u8(self.latched_a9);
        writer.write_u16(self.last_address);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        // The windows themselves are in the block `Mapper::save_state` wrote before this one.
        self.port_5000 = reader.read_u8()?;
        self.port_5100 = reader.read_u8()?;
        self.port_5200 = reader.read_u8()?;
        self.port_5300 = reader.read_u8()?;
        self.latched_a9 = reader.read_u8()?;
        self.last_address = reader.read_u16()?;
        Ok(())
    }

    /// "All registers are initialized to $00 on reset", which with `A` clear is PRG bank 3.
    fn reset(&mut self, _memory: &mut CartridgeMemory) {
        self.port_5000 = 0;
        self.port_5100 = 0;
        self.port_5200 = 0;
        self.port_5300 = 0;
        self.latched_a9 = 0;
        self.last_address = 0;
        self.update_banks();
    }

    /// Reads mapper 163's feedback port, the one readable register either board has. Below $8000,
    /// where `Cartridge::default_cpu_read` falls through to this dispatch; $8000-$FFFF takes the
    /// answer out of the windows, as `Mapper`'s own default would.
    ///
    /// **Behind `board-163`**, because mapper 162 has no readable register: with that feature off
    /// the whole method is absent and the board takes `Mapper`'s default, which is this body's
    /// `$8000-$FFFF` half and nothing else.
    #[cfg(feature = "board-163")]
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if self.kind == ChrWindowKind::NanjingFc001
            && address & Self::FEEDBACK_READ_MASK == Self::FEEDBACK_READ
        {
            return Some(self.feedback_read());
        }

        None
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    #[inline(never)]
    fn write_register(&mut self, _memory: &mut CartridgeMemory, address: u16, value: u8) {
        // Mask $FF00 on the four write ports: A15-A8 and nothing below, except the feedback port,
        // whose $FF01 reads A0 as well. $8000-$FFFF reaches none of them -- `$8000 & $FF00` is
        // `$8000` -- so the board has no register above the PRG-ROM window.
        match address & Self::PORT_MASK {
            0x5000 => {
                self.port_5000 = self.bank_port_value(value);
                self.update_banks();
            }
            // The one port whose *meaning* differs between the two boards rather than its
            // arithmetic, and the one thing a dumper's "write $04 to $5300 and see whether
            // $5100.1 moves the bank" check tells the two numbers apart by.
            0x5100 => match self.kind {
                // 163: the feedback latch, with **no bank line at all**.
                #[cfg(feature = "board-163")]
                ChrWindowKind::NanjingFc001 => {
                    if address & 0x01 == 0 {
                        // "A=0: Value latched", for `F` (bit 2) and `E` (bit 0) both.
                        self.port_5100 = value & 0x05;
                    } else if value & 0x01 != 0 {
                        // "A=1: Flip the latched F bit when E=1", the written `F` ignored.
                        self.port_5100 ^= 0x04;
                    }
                }
                // 162: `.... ..p.`, "PRG A15 if $5300.0=0".
                #[cfg(feature = "board-162")]
                ChrWindowKind::WaixingFs304 => {
                    self.port_5100 = value;
                    self.update_banks();
                }
            },
            0x5200 => {
                self.port_5200 = self.bank_port_value(value);
                self.update_banks();
            }
            // The mode register takes the value as it came: the swap bit's own words are "on
            // writes to $5000-$5200", and this port is not one of those.
            0x5300 => {
                self.port_5300 = value;
                self.update_banks();
            }
            _ => {}
        }
    }

    /// The latch: **CHR A12 becomes PPU A9 on the rise of PPU A13**, which is the entry of the PPU
    /// address into $2000-$3FFF, whether a fetch or a bare `$2006` write drove it there.
    ///
    /// **`#[inline(never)]`, and the reason is `OekaKids::on_ppu_address_bus`'s**: this dispatch is
    /// behind two latches so no board outside them reaches it, but the boards that do are the three
    /// A12 counters, the J.Y. ASIC and the MMC5's in-frame detection, whose arms run on 57 600
    /// fetches a frame. Out of line, this body is one `callq` per PPU address change of a cartridge
    /// of this board and nothing at all for them.
    ///
    /// While the switch is off — the board's power-on state — the latch is still kept, because the
    /// page makes it a property of the address lines and not of the enable; only the window update
    /// is skipped, the flat chip being already mapped.
    #[inline(never)]
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        if self.last_address & 0x2000 == 0 && address & 0x2000 != 0 {
            self.latched_a9 = ((address >> 9) & 0x01) as u8;
            if self.port_5000 & 0x80 != 0 {
                self.update_chr();
            }
        }

        self.last_address = address;
    }
}
