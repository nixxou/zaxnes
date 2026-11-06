use crate::state::{InvalidState, StateReader, StateWriter};

/// A serial I²C EEPROM on a Bandai LZ93D50 board: Xicor's **X24C01** (128 bytes) or **X24C02**
/// (256 bytes), clocked by the SCL and SDA lines the mapper's registers drive (MF-6c step 3).
///
/// REQ-CORE-100's mapper-16 row makes this **P3** in as many words ("IRQ ; EEPROM : P3"), and
/// MR-1's shared table records "no counterpart" in this build against Mesen 2's three files
/// (`BaseEeprom24C0X.h`, `Eeprom24C01.h`, `Eeprom24C02.h`). This is that counterpart, and it is
/// **one** type with a [`EepromKind`] parameter rather than three, for the reason [`super::VrcIrq`]
/// is one type: the two chips are the same bus with two different framings, and mapper 157 needs
/// *both* at once.
///
/// # Where the cost is, and is not
///
/// Nowhere on a hot path. An EEPROM moves only when a register is written, so the whole of it
/// lives inside `Mapper::write_register` and a save-state block — which is what §4.6 of
/// `docs/review/MF-6-inventory.md` predicted of it, and what the measurement in
/// `docs/progress/MF-6c.md` step 3 confirms.
///
/// # The protocol
///
/// Both chips are two-wire: the master holds SCL and SDA and the chip answers on SDA.
///
/// * a **START** is SDA falling while SCL is stable high — "START is identified by a high to low
///   transition of the SDA line while the clock SCL is *stable* in the high state";
/// * a **STOP** is SDA rising while SCL is stable high;
/// * otherwise a bit moves on each SCL edge, the rise clocking data and the fall advancing the
///   state machine.
///
/// The X24C02 opens with a **chip address** byte that must match `$A0` in its top three bits,
/// whose bit 0 is the R/W bit — "The last bit of the slave address defines the operation to be
/// performed. When set to one a read operation is selected, when set to zero a write operations
/// is selected" — and then an 8-bit word address. The X24C01 has no chip address: it opens with
/// **seven** address bits and an R/W bit in the same byte, and 128 bytes is all it has.
///
/// # The one behavioural claim of this step, quoted
///
/// nesdev ("INES Mapper 159"), verbatim:
///
/// > Bandai's developers consistently thought the X24C01 was a little-endian device, and their
/// > code clocks in address and data bytes assuming this. However, it's actually a big-endian
/// > device, and a multi-byte read or write from EEPROM will increment the address accordingly.
///
/// **So both chips are big-endian here**: every field is clocked most-significant bit first, the
/// X24C01 included. Mesen 2 implements the X24C01 the other way round — `1 << _counter`, LSB
/// first (`Eeprom24C01.h:15`), where its X24C02 uses `1 << (7 - _counter)` (`Eeprom24C02.h:15`) —
/// which is the model the page says Bandai's developers had and not the chip. AMD-061 §1 makes
/// the page prevail, and the consequence is exactly the one the page draws: a single byte still
/// round-trips (the game's bit order cancels on the way out as it did on the way in), and a
/// **multi-byte** operation increments the hardware's address rather than the game's.
/// `req_core_100_the_x24c01_is_big_endian_and_its_address_increments_accordingly` pins that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EepromKind {
    /// Xicor X24C01: 128 bytes, no chip-address byte, a 7-bit word address.
    X24C01,
    /// Xicor X24C02: 256 bytes, a chip-address byte of `$A0`, an 8-bit word address.
    X24C02,
}

impl EepromKind {
    /// The chip's capacity in bytes.
    pub(crate) const fn size(self) -> usize {
        match self {
            Self::X24C01 => 0x80,
            Self::X24C02 => 0x100,
        }
    }

    /// Mask for the word address: the chip wraps inside its own capacity.
    const fn address_mask(self) -> u8 {
        match self {
            Self::X24C01 => 0x7F,
            Self::X24C02 => 0xFF,
        }
    }
}

/// What the chip is doing between two edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Idle,
    /// Receiving the X24C02's chip-address byte.
    ChipAddress,
    /// Receiving the word address (and, on the X24C01, the R/W bit with it).
    Address,
    /// Driving a byte out, one bit per clock rise.
    Read,
    /// Receiving a byte, one bit per clock rise.
    Write,
    /// Holding SDA low for one clock, the chip's acknowledge.
    SendAck,
    /// Waiting for the master's acknowledge after a byte read.
    WaitAck,
}

impl Mode {
    const fn to_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::ChipAddress => 1,
            Self::Address => 2,
            Self::Read => 3,
            Self::Write => 4,
            Self::SendAck => 5,
            Self::WaitAck => 6,
        }
    }

    const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::ChipAddress),
            2 => Some(Self::Address),
            3 => Some(Self::Read),
            4 => Some(Self::Write),
            5 => Some(Self::SendAck),
            6 => Some(Self::WaitAck),
            _ => None,
        }
    }
}

/// One serial EEPROM, with its bus state. **The bytes are not in here**: they live in the
/// cartridge's PRG-RAM so that REQ-CORE-095's single battery slice carries them, the layout
/// [`super::Namco163`] established for its own 128 bytes (§4.6). Every method therefore takes the
/// slice the board owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eeprom24C0X {
    kind: EepromKind,
    mode: Mode,
    /// Where [`Mode::SendAck`] goes on the next clock fall.
    next_mode: Mode,
    /// The X24C02's chip-address byte as received.
    chip_address: u8,
    /// The word address, which the chip itself increments across a multi-byte operation.
    address: u8,
    /// The byte being clocked in or out.
    data: u8,
    /// Bits moved so far in the current field.
    counter: u8,
    /// What the chip drives on SDA.
    output: u8,
    previous_scl: u8,
    previous_sda: u8,
}

impl Eeprom24C0X {
    pub(crate) fn new(kind: EepromKind) -> Self {
        Self {
            kind,
            mode: Mode::Idle,
            next_mode: Mode::Idle,
            chip_address: 0,
            address: 0,
            data: 0,
            counter: 0,
            output: 1,
            previous_scl: 0,
            previous_sda: 0,
        }
    }

    /// What the chip drives on SDA, for the board's read register.
    pub(crate) const fn output(&self) -> u8 {
        self.output
    }

    /// Drives SCL alone, keeping SDA where it was: mapper 157's external chip, whose clock comes
    /// from $8000-$8003 while its data comes from $800D.
    pub(crate) fn write_scl(&mut self, bytes: &mut [u8], scl: u8) {
        let sda = self.previous_sda;
        self.write(bytes, scl, sda);
    }

    /// Drives SDA alone, keeping SCL where it was.
    pub(crate) fn write_sda(&mut self, bytes: &mut [u8], sda: u8) {
        let scl = self.previous_scl;
        self.write(bytes, scl, sda);
    }

    /// One bus event: the master has put `scl` and `sda` on the two lines.
    pub(crate) fn write(&mut self, bytes: &mut [u8], scl: u8, sda: u8) {
        if self.previous_scl != 0 && scl != 0 && sda < self.previous_sda {
            // START: SDA falls while SCL is stable high.
            self.start();
        } else if self.previous_scl != 0 && scl != 0 && sda > self.previous_sda {
            // STOP: SDA rises while SCL is stable high.
            self.mode = Mode::Idle;
            self.output = 1;
        } else if scl > self.previous_scl {
            self.clock_rise(bytes, sda);
        } else if scl < self.previous_scl {
            self.clock_fall(bytes);
        }

        self.previous_scl = scl;
        self.previous_sda = sda;
    }

    fn start(&mut self) {
        self.counter = 0;
        self.output = 1;
        match self.kind {
            // "To initiate a write operation, the master sends a start condition followed by a
            // seven bit word address and a write bit."
            EepromKind::X24C01 => {
                self.mode = Mode::Address;
                self.address = 0;
            }
            EepromKind::X24C02 => self.mode = Mode::ChipAddress,
        }
    }

    /// The bit position `counter` names inside a field `width` bits wide, **most significant
    /// first**: the big-endian reading of the page quoted on [`EepromKind`].
    const fn bit(width: u8, counter: u8) -> u8 {
        width - 1 - counter
    }

    /// Clocks one received bit into `destination`.
    fn receive_bit(destination: &mut u8, width: u8, counter: &mut u8, sda: u8) {
        if *counter < width {
            let bit = Self::bit(width, *counter);
            *destination = (*destination & !(1 << bit)) | ((sda & 1) << bit);
            *counter += 1;
        }
    }

    fn clock_rise(&mut self, bytes: &mut [u8], sda: u8) {
        match self.mode {
            Mode::ChipAddress => {
                Self::receive_bit(&mut self.chip_address, 8, &mut self.counter, sda);
            }
            Mode::Address => self.address_bit(bytes, sda),
            Mode::Read => {
                if self.counter < 8 {
                    let bit = Self::bit(8, self.counter);
                    self.output = (self.data >> bit) & 1;
                    self.counter += 1;
                }
            }
            Mode::Write => Self::receive_bit(&mut self.data, 8, &mut self.counter, sda),
            Mode::SendAck => self.output = 0,
            // A master that pulls SDA low has acknowledged and wants the next byte, so the chip
            // reads on from the address it has just incremented. **Both chips**, which is what
            // the page's "a multi-byte read or write from EEPROM will increment the address
            // accordingly" says of the X24C01 as much as of the X24C02. Mesen 2 sends its X24C01
            // to `Idle` here instead (`Eeprom24C01.h:77-82`), under a comment -- "We expected an
            // ack, but received something else" -- that its own condition contradicts.
            Mode::WaitAck => {
                if sda == 0 {
                    self.next_mode = Mode::Read;
                    self.data = self.byte_at(bytes);
                }
            }
            Mode::Idle => {}
        }
    }

    /// One address bit, and on the X24C01 the R/W bit that shares the byte with it.
    fn address_bit(&mut self, bytes: &[u8], sda: u8) {
        match self.kind {
            EepromKind::X24C01 => {
                if self.counter < 7 {
                    let bit = Self::bit(7, self.counter);
                    self.address = (self.address & !(1 << bit)) | ((sda & 1) << bit);
                    self.counter += 1;
                } else if self.counter == 7 {
                    // The eighth bit is the direction.
                    self.counter = 8;
                    if sda != 0 {
                        self.next_mode = Mode::Read;
                        self.data = self.byte_at(bytes);
                    } else {
                        self.next_mode = Mode::Write;
                    }
                }
            }
            EepromKind::X24C02 => {
                Self::receive_bit(&mut self.address, 8, &mut self.counter, sda);
            }
        }
    }

    fn clock_fall(&mut self, bytes: &mut [u8]) {
        match self.mode {
            Mode::ChipAddress => self.chip_address_done(bytes),
            Mode::Address => self.address_done(),
            Mode::Read => {
                if self.counter == 8 {
                    self.mode = Mode::WaitAck;
                    self.address = self.address.wrapping_add(1) & self.kind.address_mask();
                }
            }
            Mode::Write => {
                if self.counter == 8 {
                    self.counter = 0;
                    self.mode = Mode::SendAck;
                    // "a multi-byte read or write from EEPROM will increment the address
                    // accordingly": the chip stays in `Write` for the next byte, on both parts.
                    // Mesen 2 sends its X24C01 to `Idle` (`Eeprom24C01.h:116`), which forbids the
                    // sequential write the page describes.
                    self.next_mode = Mode::Write;
                    let index = usize::from(self.address & self.kind.address_mask());
                    if let Some(byte) = bytes.get_mut(index) {
                        *byte = self.data;
                    }
                    self.address = self.address.wrapping_add(1) & self.kind.address_mask();
                }
            }
            Mode::SendAck | Mode::WaitAck => {
                self.mode = self.next_mode;
                self.counter = 0;
                self.output = 1;
            }
            Mode::Idle => {}
        }
    }

    /// "Upon a correct compare the X24C02 outputs an acknowledge on the SDA line" — and goes back
    /// to idle when the chip address was somebody else's.
    fn chip_address_done(&mut self, bytes: &[u8]) {
        if self.counter != 8 {
            return;
        }

        self.counter = 0;
        self.output = 1;
        if self.chip_address & 0xA0 == 0xA0 {
            self.mode = Mode::SendAck;
            // Bit 0 is the R/W bit: "When set to one a read operation is selected, when set to
            // zero a write operations is selected". A read is a "Current Address Read" — "Upon
            // receipt of the slave address with the R/W bit set to one, the X24C02 issues an
            // acknowledge and transmits the eight bit word during the next eight clock cycles" —
            // so the byte is fetched **here**, from the address the chip already holds, and not
            // when the first bit leaves.
            if self.chip_address & 0x01 == 0 {
                self.next_mode = Mode::Address;
            } else {
                self.next_mode = Mode::Read;
                self.data = self.byte_at(bytes);
            }
        } else {
            self.mode = Mode::Idle;
        }
    }

    fn address_done(&mut self) {
        if self.counter != 8 {
            return;
        }

        self.counter = 0;
        self.output = 1;
        self.mode = Mode::SendAck;
        if matches!(self.kind, EepromKind::X24C02) {
            // "Finished receiving all 8 bits of the address": a word-address write is always
            // followed by data.
            self.next_mode = Mode::Write;
        }
        // On the X24C01 `next_mode` was settled by the R/W bit in `address_bit`.
    }

    /// The byte the word address names; 0 when the slice is shorter than the chip.
    fn byte_at(&self, bytes: &[u8]) -> u8 {
        bytes
            .get(usize::from(self.address & self.kind.address_mask()))
            .copied()
            .unwrap_or(0)
    }

    /// Puts the bus back to its power-on state. The **bytes are not touched**: they are the
    /// battery's, and a reset must not wipe a save (REQ-CORE-004, REQ-CORE-095).
    pub(crate) fn reset(&mut self) {
        let kind = self.kind;
        *self = Self::new(kind);
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.mode.to_u8());
        writer.write_u8(self.next_mode.to_u8());
        writer.write_u8(self.chip_address);
        writer.write_u8(self.address);
        writer.write_u8(self.data);
        writer.write_u8(self.counter);
        writer.write_u8(self.output);
        writer.write_u8(self.previous_scl);
        writer.write_u8(self.previous_sda);
    }

    /// # Errors
    /// [`InvalidState`] when the block is truncated or names a mode that does not exist.
    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        let mode = reader.read_u8()?;
        self.mode = Mode::from_u8(mode).ok_or_else(|| {
            InvalidState::corrupt(format!("Corrupt savestate: EEPROM mode {mode}."))
        })?;
        let next = reader.read_u8()?;
        self.next_mode = Mode::from_u8(next).ok_or_else(|| {
            InvalidState::corrupt(format!("Corrupt savestate: EEPROM mode {next}."))
        })?;
        self.chip_address = reader.read_u8()?;
        self.address = reader.read_u8()?;
        self.data = reader.read_u8()?;
        self.counter = reader.read_u8()?;
        self.output = reader.read_u8()?;
        self.previous_scl = reader.read_u8()?;
        self.previous_sda = reader.read_u8()?;
        Ok(())
    }
}

/// The EEPROM side of a Bandai LZ93D50 board: iNES **157**, the Datach Joint ROM System, and
/// iNES **159** (MF-6c step 3).
///
/// One field of [`super::BandaiFcg`], `None` on mappers 16 and 153, so that those two numbers pay
/// a null test on a cold path and nothing else.
///
/// # The two numbers
///
/// nesdev ("INES Mapper 159"): "All registers function the same way as INES Mapper 016, submapper
/// 5, except that the serial EEPROM is not a 24C02 but Xicor's X24C01" — one 128-byte chip, and
/// nothing else of the board changes.
///
/// nesdev ("INES Mapper 157"): "The Datach main unit is a device for insertion into the Famicom's
/// cartridge port that provides a barcode scanner" and carries "an internal 256-byte serial
/// EEPROM (X24C02)"; one game, *Battle Rush: Build up Robot Tournament*, adds "an additional
/// external 128-byte serial EEPROM (X24C01)". The header says which: "The NES 2.0 header's
/// PRG-NVRAM field will only denote whether the game cartridge has an additional 128-byte serial
/// EEPROM (byte 10 value 0x10) or not (byte 10 value 0x00)", the internal chip being excluded
/// because it is "not part of the game cartridge, but instead shared among all games". The board
/// also has **no CHR banking** — "8 KiB of unbanked CHR-RAM".
///
/// # The three register bits, read off the page's own diagrams
///
/// | | diagram | what |
/// | --- | --- | --- |
/// | `$6000-$7FFF` read | `xxxE Bxxx` | bit 4 the EEPROMs' data out, bit 3 the barcode scanner's |
/// | `$8000-$8003` write | `xxxx Cxxx` | bit 3 the **external** chip's SCL |
/// | `$800D` write | `RDC. ....` | bit 7 the direction, bit 6 SDA for **both** chips, bit 5 the **internal** chip's SCL |
///
/// Mesen 2 reads the same three bits (`BandaiFcg.h:151-153`, `:176`, `:228-229`) and its open-bus
/// mask `$E7` corroborates the first diagram exactly: bits 3 and 4 driven, the other six not.
///
/// "When both internal and external EEPROMs exist, their outputs are ANDed together via
/// open-drain circuit", which is why [`EepromPort::read_register`] is an `&&`.
///
/// # Three things this build does not do, all three recorded rather than hidden
///
/// * **The barcode scanner is a gap.** It is an input device and this build has no host surface
///   for one — the same gap MR-30-4 records for the UNROM 512's flash. Bit 3 reads 0, so a Datach
///   game sees no scan and plays on.
/// * **$800D bit 7, the direction bit, is decoded and does not gate SDA.** On the hardware it
///   releases the master's SDA driver so the chip can answer; the state machine already knows
///   which way the byte is going, because the protocol told it, and Mesen 2 ignores the bit for
///   the same reason. Letting it force SDA high would break a master that sets it late. **Q-134**
///   holds the provisional choice.
/// * **The six open-bus bits of the read register read 0, not open bus.** `Mapper::try_read_prg`
///   answers a whole byte or nothing, and `Mapper::cpu_peek_override` can only say "all open
///   bus"; neither hook can drive two bits and release six. Routing the read through
///   `cpu_peek_override` to get `Some(None)` for the rest was not available either — MF-6c step 2
///   measured one more body in that dispatch at **+0.066 % on `mmc5`**.
///
/// # Where the bytes live
///
/// In the cartridge's PRG-RAM, the internal chip first and the external one behind it, so that
/// REQ-CORE-095's single battery slice carries both — §4.6's Namco 163 layout, with no header RAM
/// in front of it. **Mesen has no layout to match here**: it keeps each chip in a file of its own
/// (`.eeprom128`, `.eeprom256`) rather than in the `.sav`, so a cross-program save file is not
/// available for these two numbers whichever order this build chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EepromPort {
    /// 157 or 159.
    mapper: i32,
    /// The chip $800D drives: the X24C02 of the Datach main unit, or mapper 159's X24C01.
    internal: Eeprom24C0X,
    /// Mapper 157's subcartridge X24C01, whose SCL comes from $8000-$8003 instead.
    external: Option<Eeprom24C0X>,
}

impl EepromPort {
    /// The port of `mapper`, or `None` on a number without one. `external_declared` follows the
    /// header: byte 10's PRG-NVRAM field is 128 bytes exactly when the cartridge carries the
    /// second chip.
    pub(crate) fn new(mapper: i32, external_declared: bool) -> Option<Self> {
        match mapper {
            157 => Some(Self {
                mapper,
                internal: Eeprom24C0X::new(EepromKind::X24C02),
                external: external_declared.then(|| Eeprom24C0X::new(EepromKind::X24C01)),
            }),
            159 => Some(Self {
                mapper,
                internal: Eeprom24C0X::new(EepromKind::X24C01),
                external: None,
            }),
            _ => None,
        }
    }

    /// The PRG-RAM this port needs: the internal chip, plus the external one when it is there.
    pub(crate) fn ram_size(mapper: i32, external_declared: bool) -> Option<usize> {
        Self::new(mapper, external_declared).map(|port| {
            port.internal.kind.size() + port.external.as_ref().map_or(0, |chip| chip.kind.size())
        })
    }

    pub(crate) const fn mapper_number(&self) -> i32 {
        self.mapper
    }

    /// What the board drives for a read of $6000-$7FFF: bit 4 the EEPROMs' output, bit 3 the
    /// barcode scanner's — which is always 0, the gap this type's note records.
    pub(crate) fn read_register(&self) -> u8 {
        let output = self.internal.output() != 0
            && self.external.as_ref().is_none_or(|chip| chip.output() != 0);
        u8::from(output) << 4
    }

    /// $800D: bit 5 the internal chip's SCL, bit 6 the SDA **both** chips see.
    ///
    /// Bit 7, the page's direction bit, is **not** consulted: Q-134 holds the choice and the
    /// reasons, and `q_134_the_eeprom_direction_bit_is_decoded_and_does_not_gate_sda` pins it.
    // Q-134
    pub(crate) fn write_control(&mut self, bytes: &mut [u8], value: u8) {
        let split = self.internal.kind.size().min(bytes.len());
        let (internal, external) = bytes.split_at_mut(split);
        let scl = (value >> 5) & 1;
        let sda = (value >> 6) & 1;
        self.internal.write(internal, scl, sda);
        if let Some(chip) = self.external.as_mut() {
            chip.write_sda(external, sda);
        }
    }

    /// $8000-$8003 bit 3: the external chip's SCL, and nothing on a cartridge without one.
    ///
    /// The page explains the odd placement — the line "uses CHR A13 output on the subcartridge
    /// header", so "PPU rendering must be disabled during external EEPROM access, or identical
    /// values must be written to all four register addresses ($8000-$8003)". That is the game's
    /// problem and not the board's: every write to one of the four addresses clocks the chip.
    pub(crate) fn write_external_clock(&mut self, bytes: &mut [u8], value: u8) {
        let split = self.internal.kind.size().min(bytes.len());
        if let Some(chip) = self.external.as_mut() {
            let (_, external) = bytes.split_at_mut(split);
            chip.write_scl(external, (value >> 3) & 1);
        }
    }

    /// Puts both buses back to power-on, leaving the bytes alone (REQ-CORE-004).
    pub(crate) fn reset(&mut self) {
        self.internal.reset();
        if let Some(chip) = self.external.as_mut() {
            chip.reset();
        }
    }

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        self.internal.save_state(writer);
        if let Some(chip) = self.external.as_ref() {
            chip.save_state(writer);
        }
    }

    /// # Errors
    /// [`InvalidState`] when the block is truncated or names a mode that does not exist.
    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.internal.load_state(reader)?;
        if let Some(chip) = self.external.as_mut() {
            chip.load_state(reader)?;
        }
        Ok(())
    }
}
