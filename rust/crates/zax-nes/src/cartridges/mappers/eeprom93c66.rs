use crate::state::{InvalidState, StateReader, StateWriter};

/// The **93C66** serial EEPROM over a **Microwire** bus: 4 Kbit, and on this board 512 words of
/// eight bits (MF-UNL-10 step 1).
///
/// The one board that carries it is [`super::Pec9588`], the 东达 (Dōngdá) PEC-9588, whose page
/// makes the chip the game's only save medium: "PRG-RAM is never battery-backed; all games save
/// the game state to a 93C66 512-byte serial EEPROM. The EEPROM's *ORG* pin is tied to GND,
/// selecting a word size of 8 bits."
///
/// # Not a shape of [`super::Eeprom24C0X`], and that is **Q-170**
///
/// §7.2 of `docs/review/UNL-census.md` drafts this device as a third *kind* of the existing
/// `Eeprom24C0X` — "parameterised by word width … Shared with, or shared by, 157/159's X24C01 and
/// 24C02". It cannot be. Those two are **two-wire I²C** parts: one bidirectional data line, a
/// START condition that is SDA falling while SCL is high, a STOP that is SDA rising, and an
/// acknowledge bit the chip drives between bytes. This is **three-wire Microwire**: a chip select,
/// a clock, a *unidirectional* data-in line, a separate data-out line, and a start **bit** rather
/// than a start condition. There is no framing in common, and the lot brief forbids changing the
/// existing boards either way. So this is a second type beside that one, and the two share what
/// the brief does ask them to share: **the persistence path**. Its bytes are not in here — they
/// live in the cartridge's PRG-RAM behind [`battery_offset`], exactly as that type's do, so that
/// REQ-CORE-095's single battery slice carries them. Every method therefore takes the slice the
/// board owns.
///
/// [`battery_offset`]: crate::cartridges::registry::battery_offset
///
/// §7.2 also calls the part "512 bytes at **16-bit** words", which the page contradicts in the
/// sentence quoted above. The page wins (MF-UNL-2's rule), and the difference is not cosmetic: the
/// x8 organisation has **nine** address bits where x16 has eight, so every command is one clock
/// longer and a game's bit stream would desynchronise under the other reading.
///
/// # The protocol, from the datasheet
///
/// Microchip **DS20001795F**, *93AA66A/B/C, 93LC66A/B/C, 93C66A/B/C — 4-Kbit Microwire Compatible
/// Serial EEPROM*. "When the ORG pin is connected to V<sub>CC</sub>, the (x16) organization is
/// selected. When it is connected to ground, the (x8) organization is selected. Instructions,
/// addresses and write data are clocked into the DI pin on the rising edge of the clock (CLK)."
///
/// The start condition: "The Start bit is detected by the device if CS and DI are both high with
/// respect to the positive edge of CLK for the first time. Before a Start condition is detected,
/// CS, CLK and DI may change in any combination (except to that of a Start condition), without
/// resulting in any device operation … As soon as CS is high, the device is no longer in Standby
/// mode."
///
/// Table 1-4, *Instruction set for x8 organization*, verbatim in its seven rows — a start bit, a
/// two-bit opcode, nine address bits and, on the two that write, eight data bits:
///
/// ```text
/// instruction  SB  opcode  address              data in   data out    req. CLK cycles
/// ERASE         1    11     A8 A7 … A1 A0           --     (RDY/BSY)         12
/// ERAL          1    00     1 0 X X X X X X X       --     (RDY/BSY)         12
/// EWDS          1    00     0 0 X X X X X X X       --     High-Z            12
/// EWEN          1    00     1 1 X X X X X X X       --     High-Z            12
/// READ          1    10     A8 A7 … A1 A0           --     D7 … D0           20
/// WRITE         1    01     A8 A7 … A1 A0         D7 … D0  (RDY/BSY)         20
/// WRAL          1    00     0 1 X X X X X X X     D7 … D0  (RDY/BSY)         20
/// ```
///
/// Four sentences of §2 decide the rest:
///
/// * **the dummy zero and the sequential read** (§2.7) — "The READ instruction outputs the serial
///   data of the addressed memory location on the DO pin. A **dummy zero bit precedes** the 8-bit
///   … output string. The output data bits will toggle on the rising edge of the CLK … **Sequential
///   read is possible when CS is held high.** The memory data will automatically cycle to the next
///   register and output sequentially." Figure 2-7 shows `0 Dx…D0 Dx…D0 Dx…D0`: one dummy zero for
///   the whole burst and none between words;
/// * **the commit edge** (§2.8) — "For 93AA66A/B/C and 93LC66A/B/C devices, after the last data bit
///   is clocked into DI, the falling edge of CS initiates the self-timed auto-erase and programming
///   cycle. For **93C66A/B/C** devices, the self-timed auto-erase and programming cycle is
///   initiated by the **rising edge of CLK on the last data bit**." This board's part is a 93C66,
///   so the byte lands on that rise and a game that never lowers CS still saves;
/// * **the erase/write enable latch** (§2.6) — "The 93XX66A/B/C powers up in the Erase/Write
///   Disable (EWDS) state. All Programming modes must be preceded by an Erase/Write Enable (EWEN)
///   instruction. Once the EWEN instruction is executed, programming remains enabled until an EWDS
///   instruction is executed or V<sub>CC</sub> is removed from the device", and "Execution of a
///   READ instruction is independent of both the EWEN and EWDS instructions". So [`Self::reset`]
///   clears it and a bare CS cycle does not;
/// * **the data-out line** (§2.0) — "The DO pin is normally held in a **High-Z** state except when
///   reading data from the device, or when checking the Ready/Busy status during a programming
///   operation … DO low indicates that programming is still in progress, while DO high indicates
///   the device is ready. DO will enter the High-Z state on the falling edge of CS."
///
/// # The two things this model simplifies, both recorded in Q-170
///
/// * **Programming is instantaneous.** A real part holds DO low for up to 15 ms. Nothing in this
///   build can make a game wait, and a game that polls RDY/BSY sees *ready* at once — which is the
///   terminal state of the real poll, so the game proceeds. The intermediate *busy* is never seen.
/// * **High-Z reads as 1.** The board does not tell us which way its DO line floats, and this
///   choice is the one the protocol makes legible: with DO idling high, the dummy zero of §2.7 is
///   an edge a game can synchronise its read on, and the RDY/BSY poll above answers *ready*. It is
///   also the level [`super::Eeprom24C0X::output`] idles at, for the same reason.
///
/// ERASE and ERAL set a word, or the array, to "the logical `1' state" (§2.4, §2.5) — `$FF` — and
/// WRAL "will write the entire memory array with the data specified in the command" (§2.9), which
/// "does include an automatic ERAL cycle … but the chip must be in the EWEN status".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eeprom93C66 {
    mode: Mode,
    /// The two-bit opcode of the instruction being clocked in.
    opcode: u8,
    /// The nine-bit word address, which the chip itself increments across a sequential read.
    address: u16,
    /// The byte being clocked in or out.
    data: u8,
    /// Bits moved so far in the current field.
    counter: u8,
    /// What the chip drives on DO.
    output: u8,
    /// The EWEN/EWDS latch: cleared at power-on, and **not** by CS going low.
    write_enabled: bool,
    previous_cs: u8,
    previous_clk: u8,
}

/// Which field of an instruction the chip is clocking, between two edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// CS is low, or an instruction has run to its end: DO is undriven and nothing is clocked.
    Idle,
    /// CS has gone high and the start bit has not arrived yet.
    Start,
    /// Receiving the two opcode bits.
    Opcode,
    /// Receiving the nine address bits (of which the top two are the sub-opcode when the opcode
    /// is `00`).
    Address,
    /// Driving a byte out, one bit per clock rise, and cycling to the next word after the eighth.
    Read,
    /// Receiving the eight data bits of a WRITE.
    Write,
    /// Receiving the eight data bits of a WRAL.
    WriteAll,
}

impl Mode {
    const fn to_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Start => 1,
            Self::Opcode => 2,
            Self::Address => 3,
            Self::Read => 4,
            Self::Write => 5,
            Self::WriteAll => 6,
        }
    }

    const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::Start),
            2 => Some(Self::Opcode),
            3 => Some(Self::Address),
            4 => Some(Self::Read),
            5 => Some(Self::Write),
            6 => Some(Self::WriteAll),
            _ => None,
        }
    }
}

impl Eeprom93C66 {
    /// "a 93C66 **512-byte** serial EEPROM" with "ORG … tied to GND, selecting a word size of
    /// 8 bits": 512 words of one byte.
    pub(crate) const SIZE: usize = 512;

    /// Nine address bits, which is what the x8 organisation of table 1-4 clocks in.
    const ADDRESS_BITS: u8 = 9;
    const ADDRESS_MASK: u16 = 0x01FF;

    /// `10`, table 1-4.
    const READ: u8 = 0b10;
    /// `01`.
    const WRITE: u8 = 0b01;
    /// `11`.
    const ERASE: u8 = 0b11;
    /// `00`, whose action is the top two address bits: `00` EWDS, `01` WRAL, `10` ERAL, `11` EWEN.
    const EXTENDED: u8 = 0b00;

    pub(crate) const fn new() -> Self {
        Self {
            mode: Mode::Idle,
            opcode: 0,
            address: 0,
            data: 0,
            counter: 0,
            output: 1,
            write_enabled: false,
            previous_cs: 0,
            previous_clk: 0,
        }
    }

    /// What the chip drives on DO, for the board's read register. 1 while undriven, which is the
    /// choice Q-170 records.
    pub(crate) const fn output(&self) -> u8 {
        self.output
    }

    /// One bus event: the master has put `cs`, `clk` and `di` on the three lines. The board drives
    /// all three from one register, so a single event may move all of them; CS is settled first,
    /// then the clock edge, which is the order the hardware's own gating implies.
    pub(crate) fn write(&mut self, bytes: &mut [u8], cs: u8, clk: u8, di: u8) {
        if cs & 1 == 0 {
            if self.previous_cs != 0 {
                // "DO will enter the High-Z state on the falling edge of CS", and the device is
                // back in standby.
                self.mode = Mode::Idle;
                self.counter = 0;
                self.output = 1;
            }
        } else {
            if self.previous_cs == 0 {
                // "As soon as CS is high, the device is no longer in Standby mode" -- and the
                // start bit has not been seen yet.
                self.mode = Mode::Start;
                self.counter = 0;
            }

            if clk & 1 > self.previous_clk {
                self.clock_rise(bytes, di & 1);
            }
        }

        self.previous_cs = cs & 1;
        self.previous_clk = clk & 1;
    }

    /// Clocks one received bit into a byte-wide field, **most significant bit of the field
    /// first**: the datasheet's `D7 … D0` order, and the two opcode bits.
    fn receive_bit(destination: &mut u8, width: u8, counter: &mut u8, di: u8) {
        if *counter < width {
            let bit = width - 1 - *counter;
            *destination = (*destination & !(1 << bit)) | (di << bit);
            *counter += 1;
        }
    }

    /// The same for the nine-bit address field, whose `A8` does not fit in a byte.
    fn receive_address_bit(destination: &mut u16, counter: &mut u8, di: u8) {
        if *counter < Self::ADDRESS_BITS {
            let bit = Self::ADDRESS_BITS - 1 - *counter;
            *destination = (*destination & !(1 << bit)) | (u16::from(di) << bit);
            *counter += 1;
        }
    }

    fn clock_rise(&mut self, bytes: &mut [u8], di: u8) {
        match self.mode {
            // "The Start bit is detected by the device if CS and DI are both high with respect to
            // the positive edge of CLK for the first time" -- a rise with DI low is one of the
            // combinations that "may change in any combination … without resulting in any device
            // operation".
            Mode::Start => {
                if di != 0 {
                    self.mode = Mode::Opcode;
                    self.opcode = 0;
                    self.counter = 0;
                }
            }
            Mode::Opcode => {
                Self::receive_bit(&mut self.opcode, 2, &mut self.counter, di);
                if self.counter == 2 {
                    self.mode = Mode::Address;
                    self.address = 0;
                    self.counter = 0;
                }
            }
            Mode::Address => {
                Self::receive_address_bit(&mut self.address, &mut self.counter, di);
                if self.counter == Self::ADDRESS_BITS {
                    self.address_done(bytes);
                }
            }
            // The eight output bits, then "the memory data will automatically cycle to the next
            // register and output sequentially" with no second dummy zero.
            Mode::Read => {
                self.output = (self.data >> (7 - self.counter)) & 1;
                self.counter += 1;
                if self.counter == 8 {
                    self.address = (self.address + 1) & Self::ADDRESS_MASK;
                    self.data = self.byte_at(bytes);
                    self.counter = 0;
                }
            }
            Mode::Write | Mode::WriteAll => {
                Self::receive_bit(&mut self.data, 8, &mut self.counter, di);
                if self.counter == 8 {
                    // "For 93C66A/B/C devices, the self-timed auto-erase and programming cycle is
                    // initiated by the rising edge of CLK on the last data bit" -- this rise.
                    if self.write_enabled {
                        if self.mode == Mode::WriteAll {
                            bytes.fill(self.data);
                        } else if let Some(byte) = bytes.get_mut(self.index()) {
                            *byte = self.data;
                        }
                    }

                    self.finish();
                }
            }
            Mode::Idle => {}
        }
    }

    /// The nine address bits are in: the instruction either starts moving a byte or is already
    /// over.
    fn address_done(&mut self, bytes: &mut [u8]) {
        self.counter = 0;
        match self.opcode {
            Self::READ => {
                // The rise that completed the address drives the dummy zero; the next eight drive
                // D7..D0.
                self.mode = Mode::Read;
                self.data = self.byte_at(bytes);
                self.output = 0;
            }
            Self::WRITE => {
                self.mode = Mode::Write;
                self.data = 0;
            }
            // "The ERASE instruction forces all data bits of the specified address to the logical
            // `1' state." The datasheet's 93C-device sentence puts the commit on "the rising edge
            // of CLK before the last address bit", which cannot be meant literally -- the address
            // is not complete there; the write cycle starts at the end of the address field, which
            // is this rise, and a game clocks all twelve cycles either way.
            Self::ERASE => {
                if self.write_enabled {
                    if let Some(byte) = bytes.get_mut(self.index()) {
                        *byte = 0xFF;
                    }
                }
                self.finish();
            }
            // Opcode `00`: the action is the top two bits of the address field.
            Self::EXTENDED => match (self.address >> (Self::ADDRESS_BITS - 2)) & 0x03 {
                // EWDS.
                0b00 => {
                    self.write_enabled = false;
                    self.finish();
                }
                // WRAL, the one extended instruction with data behind it.
                0b01 => {
                    self.mode = Mode::WriteAll;
                    self.data = 0;
                }
                // ERAL: "will erase the entire memory array to the logical `1' state".
                0b10 => {
                    if self.write_enabled {
                        bytes.fill(0xFF);
                    }
                    self.finish();
                }
                // EWEN.
                _ => {
                    self.write_enabled = true;
                    self.finish();
                }
            },
            // Two bits have no fifth value; the arm is here so that the four opcodes above are
            // read as the constants of table 1-4 and not as a catch-all.
            _ => self.finish(),
        }
    }

    /// The instruction is over: the chip waits for CS to fall and answers *ready* until it does.
    fn finish(&mut self) {
        self.mode = Mode::Idle;
        self.counter = 0;
        self.output = 1;
    }

    /// The index the word address names inside the slice the board owns.
    fn index(&self) -> usize {
        usize::from(self.address & Self::ADDRESS_MASK)
    }

    /// The byte the word address names; 0 when the slice is shorter than the chip.
    fn byte_at(&self, bytes: &[u8]) -> u8 {
        bytes.get(self.index()).copied().unwrap_or(0)
    }

    /// Puts the bus and the EWEN latch back to their power-on state — "After power-up, the device
    /// is automatically in the EWDS mode". The **bytes are not touched**: they are the save, and a
    /// reset must not wipe one (REQ-CORE-004, REQ-CORE-095).
    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.mode.to_u8());
        writer.write_u8(self.opcode);
        writer.write_u16(self.address);
        writer.write_u8(self.data);
        writer.write_u8(self.counter);
        writer.write_u8(self.output);
        writer.write_u8(u8::from(self.write_enabled));
        writer.write_u8(self.previous_cs);
        writer.write_u8(self.previous_clk);
    }

    /// # Errors
    /// [`InvalidState`] when the block is truncated or names a mode that does not exist.
    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let mode = reader.read_u8()?;
        self.mode = Mode::from_u8(mode).ok_or_else(|| {
            InvalidState::corrupt(format!("Corrupt savestate: 93C66 mode {mode}."))
        })?;
        self.opcode = reader.read_u8()? & 0x03;
        self.address = reader.read_u16()? & Self::ADDRESS_MASK;
        self.data = reader.read_u8()?;
        self.counter = reader.read_u8()?;
        self.output = reader.read_u8()? & 1;
        self.write_enabled = reader.read_u8()? != 0;
        self.previous_cs = reader.read_u8()? & 1;
        self.previous_clk = reader.read_u8()? & 1;
        Ok(())
    }
}
