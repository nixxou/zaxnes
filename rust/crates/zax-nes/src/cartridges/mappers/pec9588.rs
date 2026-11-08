use super::{windows_for, Eeprom93C66, Mapper};
use crate::cartridges::{BankWindows, CartridgeMemory, ChrFetch, ChrView};
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::NametableMirroring;

/// iNES mapper **164**, the 东达 (Dōngdá) **PEC-9588** *Pyramid Educational Computer* and the
/// 燕城 (Yànchéng) **cy2000-3** circuit board (REQ-CORE-102, MF-UNL-10 step 2).
///
/// nesdev, "INES Mapper 164", names the six games the census counts: *Final Fantasy 太空戰士 V*
/// (G-003), *大話西游* (G-004), *櫻桃小丸子* (G-005), *岳飛傳* (G-006), *Dark Seed - 黑暗之蛊*
/// (G-009) and *口袋精靈: 金* (Pokémon: Gold Edition, "not to be confused with several
/// similarly-named ports of the same name").
///
/// # The memories, and where the save lives
///
/// The *Banks* section, whole:
///
/// * "CPU $6000-$7FFF: 0-8 KiB unbanked PRG-RAM, **not battery-backed**";
/// * "CPU $8000-$BFFF: 16 KiB switchable PRG-ROM bank";
/// * "CPU $C000-$FFFF: 16 KiB semi-fixed PRG-ROM bank";
/// * "CPU $8000-$FFFF: *alternatively:* 32 KiB swichable PRG-ROM bank";
/// * "PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM".
///
/// and the sentence that makes this board an L in the census: "**PRG-RAM is never battery-backed;
/// all games save the game state to a 93C66 512-byte serial EEPROM.** The EEPROM's *ORG* pin is
/// tied to GND, selecting a word size of 8 bits. There is also a spot for a second EEPROM chip
/// that is unpopulated on all known games."
///
/// So the board has **two** memories at once where every other board here has one: a volatile RAM
/// chip the CPU addresses, and 512 bytes of non-volatile storage the CPU can only reach one bit at
/// a time. Both live in `CartridgeMemory::prg_ram`, the volatile part first and the EEPROM behind
/// it, and `registry::battery_offset` puts REQ-CORE-095's battery slice at the EEPROM — which is
/// what that function's second parameter has existed for since the MMC1's SOROM. The volatile part
/// is therefore **not** in the save file and the EEPROM **is**, which is what the page says of the
/// hardware. [`Eeprom93C66`] holds the bus and not the bytes, exactly as [`super::Eeprom24C0X`]
/// does for mappers 157 and 159, so the two EEPROM boards share one persistence path and neither
/// one's save format moves.
///
/// "The circuit board has address lines for 8 KiB of PRG-RAM, though all known carts mount either
/// no or a 2 KiB RAM chip which is then mirrored three times across the respective address range."
/// No header this build reads can say which of the two a dump is, so the volatile chip is **2 KiB
/// mirrored three times** — the behaviour of every known cartridge that mounts one — and
/// [`Mapper::prg_ram_window`]'s own wrap is the mirroring (Q-170; see
/// [`Pec9588::VOLATILE_RAM_SIZE`]).
///
/// # The five registers, under the page's own masks
///
/// Every port's mask is `$FF00`: A15-A8 and nothing below, so each repeats through its own 256
/// bytes and `$8000-$FFFF` reaches none of them. "All registers are initialized to $00 on reset."
///
/// ```text
/// $5000 write  D~CSQM PPPp   $5100 write  D~.... ..PP   $5200 write  D~.T.S .C.D
///                ||+|-++++- PRG A18..A14          ++- PRG A20..A19     | |  | +- DAT out
///                || | ++++-  (A18..A15 if M=1)                         | |  +--- CLK out
///                || +------ PRG banking mode                           | +------ #1 CS out
///                |+-------- fixed-bank select                          +-------- #2 CS out
///                +--------- 1 bpp video mode
///
/// $5300 write  D~M... ....   $5500 read   D~.... .A..
///                +--------- mirroring if $5000.4=1          +--- DAT in (inverted)
///                            0=Horizontal, 1=Vertical
/// ```
///
/// # The two PRG modes, and the bit order the page's own figures pin
///
/// The mode descriptions are written **A18 first**, which the diagram's `PRG A18..A14` agrees with
/// and which the page then proves twice over:
///
/// > 0: PRG A14..A18=QPPPp when CPU A14=0 (UxROM, 16 KiB switchable bank)
/// >    PRG A14..A18=11111 when CPU A14=1 and S=0 (fixed bank=1F)
/// >    PRG A14..A18=111p0 when CPU A14=1 and S=1 (fixed bank=1C or 1E)
/// > 1: PRG A14=CPU A14, PRG A15..A18=PPPp (BxROM, 32 KiB switchable bank)
///
/// `111p0` read A18-first is `0b111p0` — `$1C` with `p` clear and `$1E` with it set, which is the
/// pair the page names. Read A14-first it would be 7 or 15, which it does not. And `11111` is
/// bank `$1F`, which the page's other note confirms from the game's side: "Because this register is
/// initialized to $00 on reset, thus clearing bit 4 which causes UxROM mode, the game's reset
/// vectors will be at the end of 16 KiB bank **#31**."
///
/// So the 16 KiB bank number is `A14 | A15<<1 | … | A20<<6` — seven bits, 2 MiB — composed of
/// `PPPp` (A17..A14) and `Q` (A18) from `$5000` and `PP` (A20..A19) from `$5100`. In BxROM mode the
/// same `PPPp` nibble is A18..A15 instead, so it *is* the 32 KiB bank number and `$5100` sits two
/// bits above it.
///
/// # The 1 bpp video mode, which is the other half of the L
///
/// ```text
/// C... ....
/// +--------- 1 bpp video mode: when PPU A13=0 (pattern table) ...
///             0: CHR A3=PPU A3, CHR A12=PPU A12 (disable 1 bpp mode)
///             1: CHR A3=PPU A0, CHR A12=PPU A9, both latched on
///                last rise of PPU A13 (enable 1 bpp mode)
/// ```
///
/// > 1 bpp mode is an all points-addressable video mode that allows the 8 KiB CHR-RAM to hold just
/// > enough data for an entire screen.
///
/// **CHR A12 is [`super::ChrWindowBoard`]'s mechanism exactly** — "PPU A9 latched on last rise of
/// PPU A13", the same sentence mappers 162 and 163 carry — and **CHR A3 is a second line latched on
/// the same edge**. PPU A13 rises when the address enters $2000-$3FFF, i.e. at a nametable fetch,
/// so what the two latches hold is that address's A0 and A9: coarse X bit 0 and coarse Y bit 4 of
/// the tile being drawn. A pattern fetch then reads
/// `fine_y | latched_A0<<3 | tile<<4 | latched_A9<<12`, which gives each of the screen's
/// 32 × 30 tiles its own 8 bytes inside the 8 KiB chip — 960 of the 1 024 eight-byte blocks it
/// holds, which is the "just enough data for an entire screen" the page claims. And because CHR A3
/// no longer follows the fetch's own A3, **both bitplanes of a tile read the same eight bytes**:
/// one bit per pixel, hence the mode's name.
///
/// This board is therefore in **both** of §4.3's latches, for the reason mappers 96, 162 and 163
/// are: the rise of A13 is an address change and `Mapper::on_ppu_address_bus` is the only hook that
/// sees one, a bare `$2006` write that drives the address over $2000 being one this core cannot
/// tell from a fetch's — and neither can the board, which has only the address lines to go on. It
/// has **no IRQ**, so [`Mapper::ppu_irq_horizon`] keeps `Mapper`'s own `Some(i64::MAX)`.
///
/// **Unlike those three boards the latch moves no bank**, so `on_ppu_address_bus` stores two bits
/// and returns: A3 cannot be a bank line at all (the windows are 1 KiB) and A12 is not worth being
/// one when its neighbour is not. The substitution happens instead in [`Pec9588::chr_address`],
/// which [`Mapper::ppu_peek`] and [`Mapper::ppu_write`] both go through — the write side included,
/// because CHR A3 and A12 are pins on the RAM chip and a `$2007` write drives them through the same
/// board (Q-170 records that reading). With the mode off, `chr_address` is the identity and the
/// chip is the flat 8 KiB [`BankWindows::map_chr_8k`] gives.
///
/// # The Microwire port
///
/// `$5200` drives the three lines of [`Eeprom93C66`] — DI on bit 0, CLK on bit 2 and chip select on
/// bit 4 — and `$5500` reads DO back **inverted** in bit 2. Bit 6 is "93C66 EEPROM #2 CS output",
/// the second chip's select; the page says that spot "is unpopulated on all known games", so this
/// board holds one chip and bit 6 is decoded and drives nothing. Bits other than 2 of the read
/// register are not described by the page and answer zero, which is the choice
/// [`super::ChrWindowBoard`] makes for mapper 163's feedback port.
///
/// # NES 2.0 mapper 558 is not this number
///
/// The page's *See also*: "NES 2.0 Mapper 558 is a later version which drops UxROM mode and
/// connects the microwire interface to EEPROM differently." That is a different number with a
/// different register set and it is not in this lot or in the census's roster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pec9588 {
    banks: BankWindows,
    /// `$5000`: `CSQM PPPp` — the 1 bpp enable, the fixed-bank select, PRG A18, the mode bit and
    /// the four low bank lines.
    port_5000: u8,
    /// `$5100`: `.... ..PP`, PRG A20..A19.
    port_5100: u8,
    /// `$5300`: `M... ....`, the mirroring the mode bit may defer to.
    port_5300: u8,
    /// The 93C66 and its bus; its 512 bytes are in PRG-RAM behind [`Pec9588::eeprom_offset`].
    eeprom: Eeprom93C66,
    /// PPU A0 on the last rise of PPU A13, which is CHR A3 while 1 bpp mode is on.
    latched_a0: u8,
    /// PPU A9 on the same rise, which is CHR A12 while it is on.
    latched_a9: u8,
    /// The previous PPU address, the other side of the edge on A13. In the save state because
    /// nothing derives it, which is [`super::OekaKids`]'s own reason for keeping the same field.
    last_address: u16,
    /// Where the EEPROM's 512 bytes start inside PRG-RAM: the volatile chip's length, and the
    /// same figure [`registry::battery_offset`] answers for this number. Derived from the
    /// memories at load, so it is not in the save state.
    ///
    /// [`registry::battery_offset`]: crate::cartridges::registry::battery_offset
    eeprom_offset: usize,
}

impl Pec9588 {
    /// Every write port's "Mask: $FF00" — A15-A8 and nothing below.
    const PORT_MASK: u16 = 0xFF00;
    /// "Microwire Interface ($5500, read), Mask: $FF00".
    const MICROWIRE_READ: u16 = 0x5500;

    /// "PPU $0000-$1FFF: 8 KiB unbanked CHR-RAM". The figure REQ-CORE-012's default would give an
    /// image with no CHR-ROM, stated because the page states it and no iNES 1.0 field can.
    pub(crate) const CHR_RAM_SIZE: usize = 8 * 1024;

    /// The volatile RAM chip at $6000-$7FFF: **2 KiB**, "mirrored three times across the
    /// respective address range" by [`Mapper::prg_ram_window`]'s wrap.
    ///
    /// The page gives the board "address lines for 8 KiB of PRG-RAM, though all known carts mount
    /// either no or a 2 KiB RAM chip" — so 2 KiB mirrored is the behaviour of every known cartridge
    /// that has any RAM at all, and no header this build reads can say which of the two a dump is
    /// (`InesHeader`'s PRG-RAM field is floored at the 8 KiB default on both iNES 1.0 and NES 2.0,
    /// and NES 2.0's byte 10 adds the non-volatile nibble into the same figure). A cartridge that
    /// mounts no chip gets RAM it does not use, which is the divergence every iNES 1.0 board here
    /// already carries. **Q-170** records the choice.
    pub(crate) const VOLATILE_RAM_SIZE: usize = 2 * 1024;

    /// What `board_memory_sizes` asks of PRG-RAM: the volatile chip and the EEPROM behind it.
    pub(crate) const PRG_RAM_SIZE: usize = Self::VOLATILE_RAM_SIZE + Eeprom93C66::SIZE;

    pub(crate) fn new(memory: &CartridgeMemory) -> Self {
        let mut board = Self {
            banks: windows_for(memory),
            port_5000: 0,
            port_5100: 0,
            port_5300: 0,
            eeprom: Eeprom93C66::new(),
            latched_a0: 0,
            latched_a9: 0,
            last_address: 0,
            eeprom_offset: memory.prg_ram().len().saturating_sub(Eeprom93C66::SIZE),
        };
        board.banks.map_chr_8k(0);
        board.update_banks();
        board
    }

    /// The two PRG windows, from `$5000`'s mode bit.
    fn update_banks(&mut self) {
        // "$5100: .... ..PP, PRG A20..A19" -- two bits above A18, i.e. bits 6-5 of a 16 KiB bank
        // number and bits 5-4 of a 32 KiB one.
        let high = i32::from(self.port_5100 & 0x03) << 5;
        // `PPPp`, A17..A14 in UxROM mode and A18..A15 in BxROM mode: the same nibble either way.
        let low = i32::from(self.port_5000 & 0x0F);
        if self.port_5000 & 0x10 == 0 {
            // "0: PRG A14..A18=QPPPp when CPU A14=0 (UxROM, 16 KiB switchable bank)" -- `Q` is bit
            // 5 of the port and A18 is bit 4 of the bank number.
            let a18 = i32::from(self.port_5000 & 0x20) >> 1;
            self.banks.map_prg_16k(0, high | a18 | low);
            // "=11111 when CPU A14=1 and S=0 (fixed bank=1F)"; "=111p0 when … S=1 (fixed
            // bank=1C or 1E)", `p` being bit 0 of the port.
            let fixed = if self.port_5000 & 0x40 == 0 {
                0x1F
            } else {
                0x1C | ((low & 0x01) << 1)
            };
            self.banks.map_prg_16k(2, high | fixed);
        } else {
            // "1: PRG A14=CPU A14, PRG A15..A18=PPPp (BxROM, 32 KiB switchable bank)" -- the
            // nibble is the 32 KiB bank number and `$5100` sits two bits above it.
            self.banks.map_prg_32k((high >> 1) | low);
        }
    }

    /// "Also selects nametable mirroring: 0: Forced vertical mirroring; 1: Mirroring selected by
    /// $5300" — `$5000` bit 4, the same bit that chooses the banking mode, and `$5300` bit 7 is
    /// "0=Horizontal, 1=Vertical".
    fn update_mirroring(&self, memory: &mut CartridgeMemory) {
        let vertical = self.port_5000 & 0x10 == 0 || self.port_5300 & 0x80 != 0;
        memory.set_mirroring(if vertical {
            NametableMirroring::Vertical
        } else {
            NametableMirroring::Horizontal
        });
    }

    /// CHR A3 and CHR A12 of a pattern fetch: the fetch's own lines, or — in 1 bpp mode — the two
    /// bits latched on the last rise of PPU A13.
    ///
    /// `address & 0x0FF7` is the fetch with A12 and A3 taken out of it; the two latches put theirs
    /// back. With the mode off the whole function is the identity, which is what keeps the flat
    /// 8 KiB chip flat.
    #[inline]
    fn chr_address(&self, address: u16) -> u16 {
        if self.port_5000 & 0x80 == 0 {
            return address;
        }

        (address & 0x0FF7) | (u16::from(self.latched_a9) << 12) | (u16::from(self.latched_a0) << 3)
    }

    /// The CHR the board shows, as a slice: CHR-RAM, or the CHR-ROM of a dump that declares some.
    fn chr(memory: &CartridgeMemory) -> &[u8] {
        let chr_rom = memory.rom().chr_rom();
        if chr_rom.is_empty() {
            memory.chr_ram()
        } else {
            chr_rom
        }
    }

    /// `$5200`: "`.T.S .C.D`" — DI on bit 0, CLK on bit 2, the populated chip's CS on bit 4.
    ///
    /// Bit 6 is the unpopulated second chip's select and drives nothing; see this type's note.
    fn write_microwire(&mut self, memory: &mut CartridgeMemory, value: u8) {
        let offset = self.eeprom_offset;
        let bytes = memory.prg_ram_mut().get_mut(offset..).unwrap_or_default();
        self.eeprom
            .write(bytes, (value >> 4) & 1, (value >> 2) & 1, value & 1);
    }

    /// `$5500`: "93C66 EEPROM DAT input (**inverted**)" in bit 2, and nothing else driven.
    fn read_microwire(&self) -> u8 {
        (!self.eeprom.output() & 1) << 2
    }
}

impl Mapper for Pec9588 {
    banked!();

    fn mapper_number(&self) -> i32 {
        164
    }

    fn save_registers(&self, writer: &mut StateWriter) {
        writer.write_u8(self.port_5000);
        writer.write_u8(self.port_5100);
        writer.write_u8(self.port_5300);
        writer.write_u8(self.latched_a0);
        writer.write_u8(self.latched_a9);
        writer.write_u16(self.last_address);
        self.eeprom.save_state(writer);
    }

    fn load_registers(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        // The windows themselves are in the block `Mapper::save_state` wrote before this one.
        self.port_5000 = reader.read_u8()?;
        self.port_5100 = reader.read_u8()?;
        self.port_5300 = reader.read_u8()?;
        self.latched_a0 = reader.read_u8()? & 1;
        self.latched_a9 = reader.read_u8()? & 1;
        self.last_address = reader.read_u16()?;
        self.eeprom.load_state(reader)
    }

    /// "All registers are initialized to $00 on reset", which is UxROM mode with the fixed bank at
    /// `$1F`, 1 bpp mode off and forced vertical mirroring. The EEPROM's **bytes are not touched**
    /// — they are the save (REQ-CORE-004) — and its bus goes back to power-on, which is the
    /// `EepromPort::reset` precedent.
    fn reset(&mut self, memory: &mut CartridgeMemory) {
        self.port_5000 = 0;
        self.port_5100 = 0;
        self.port_5300 = 0;
        self.eeprom.reset();
        self.latched_a0 = 0;
        self.latched_a9 = 0;
        self.last_address = 0;
        self.banks.map_chr_8k(0);
        self.update_banks();
        self.update_mirroring(memory);
    }

    /// Reads the Microwire port at `$5500`, below $8000 where `Cartridge::default_cpu_read` falls
    /// through to this dispatch; $8000-$FFFF takes the answer out of the windows, as `Mapper`'s own
    /// default would.
    fn try_read_prg(&self, memory: &CartridgeMemory, address: u16) -> Option<u8> {
        if address >= 0x8000 {
            return self
                .banks
                .read_prg(address, memory.rom().prg_rom(), memory.prg_ram());
        }

        if address & Self::PORT_MASK == Self::MICROWIRE_READ {
            return Some(self.read_microwire());
        }

        None
    }

    /// `#[inline(never)]`, as every board of this shape since MF-6a step 1.
    #[inline(never)]
    fn write_register(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        match address & Self::PORT_MASK {
            0x5000 => {
                self.port_5000 = value;
                self.update_banks();
                self.update_mirroring(memory);
            }
            0x5100 => {
                self.port_5100 = value;
                self.update_banks();
            }
            0x5200 => self.write_microwire(memory, value),
            0x5300 => {
                self.port_5300 = value;
                self.update_mirroring(memory);
            }
            _ => {}
        }
    }

    /// The pattern fetch, through [`Pec9588::chr_address`].
    ///
    /// **`#[inline(never)]`**: `DecodeBoard::ppu_peek` forwards to every board of that enum and is
    /// reached per fetch for all of them (the variant answers `None` to `Mapper::chr_view`), so one
    /// more *body* in it is one more thing for that dispatch to lay out. Out of line the arm this
    /// board contributes is a `callq`, which is the reason `OekaKids::on_ppu_address_bus` carries
    /// the attribute and the reason MF-6a step 1 established it.
    #[inline(never)]
    fn ppu_peek(&self, memory: &CartridgeMemory, address: u16, _fetch: ChrFetch) -> u8 {
        self.banks
            .read_chr(self.chr_address(address), Self::chr(memory))
    }

    /// The CHR-RAM write side of the same substitution: CHR A3 and A12 are pins on the RAM chip, so
    /// a `$2007` write in 1 bpp mode lands where a fetch of the same address would read (Q-170).
    #[inline(never)]
    fn ppu_write(&mut self, memory: &mut CartridgeMemory, address: u16, value: u8) {
        if memory.rom().chr_rom().is_empty() {
            let address = self.chr_address(address);
            self.banks.write_chr(address, value, memory.chr_ram_mut());
        }
    }

    /// `None` while 1 bpp mode is on, and the flat chip's view otherwise.
    ///
    /// `Ppu2C02::read_row_planes` captures a view when a background row is drawn and replays the
    /// row through it later (REQ-CORE-037). In 1 bpp mode there is no view to capture: CHR A3 and
    /// A12 come from a latch that moves once per tile, which no set of 1 KiB window offsets can
    /// express. `None` sends the replay through [`Mapper::ppu_peek`] instead, where
    /// [`Pec9588::chr_address`] is.
    fn chr_peek_view<'m>(&self, memory: &'m CartridgeMemory) -> Option<ChrView<'m>> {
        if self.port_5000 & 0x80 != 0 {
            return None;
        }

        Some(ChrView::new(Self::chr(memory), &self.banks))
    }

    /// "0-8 KiB unbanked PRG-RAM, not battery-backed": the volatile chip, which is PRG-RAM up to
    /// [`Pec9588::eeprom_offset`] and not the EEPROM behind it. A 2 KiB chip "is then mirrored
    /// three times across the respective address range", which is this wrap.
    fn prg_ram_window(&self, _memory: &CartridgeMemory) -> (usize, usize) {
        (0, self.eeprom_offset)
    }

    /// The two latches, on the rise of PPU A13 — "both latched on last rise of PPU A13", whether a
    /// fetch or a bare `$2006` write drove the address there.
    ///
    /// **`#[inline(never)]`, for `ChrWindowBoard::on_ppu_address_bus`'s reason**: this dispatch is
    /// behind two latches so no board outside them reaches it, but the boards that do are the A12
    /// counters and the MMC5's in-frame detection, whose arms run on 57 600 fetches a frame.
    ///
    /// The latches are kept whether or not 1 bpp mode is on, the page making them a property of the
    /// address lines and not of the enable — and **no window is recomputed here**, which is what
    /// separates this board from the three before it in that latch.
    #[inline(never)]
    fn on_ppu_address_bus(&mut self, _memory: &mut CartridgeMemory, address: u16, _clock: i64) {
        if self.last_address & 0x2000 == 0 && address & 0x2000 != 0 {
            self.latched_a0 = (address & 0x01) as u8;
            self.latched_a9 = ((address >> 9) & 0x01) as u8;
        }

        self.last_address = address;
    }
}
