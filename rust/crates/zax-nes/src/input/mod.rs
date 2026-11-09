//! Controller ports (port of `src/Zax.Nes/Input`): the standard pad, the Zapper and the two
//! ports at $4016 and $4017 (REQ-CORE-022, REQ-CORE-024, REQ-CORE-060, REQ-CORE-061).

use crate::state::{InvalidState, StateReader, StateWriter};
use crate::RegionTiming;

// What is plugged into a port, and a port number out of range, are on the `Engine` trait
// (`port_type`/`set_port_type`), so they live in `zax-core-api` since lot FCEUX-23 step 2 beside
// the input word; re-exported here at the paths they were at. The ports *themselves* below — the
// shift registers, the strobe, the Zapper — are Zaxnes's machine and stay here.
pub use zax_core_api::{ControllerPortType, InvalidPort};

/// Where a Zapper is aimed, in image pixels; outside the image means "pointing off screen"
/// (REQ-CORE-060).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZapperPosition {
    /// Column, 0 to 255 when on screen.
    pub x: i32,
    /// Line, 0 to 239 when on screen.
    pub y: i32,
}

impl ZapperPosition {
    /// The aim of a gun pointing away from the screen.
    pub const OFF_SCREEN: Self = Self { x: -1, y: -1 };

    /// Creates a position.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Whether the aim is inside the image (REQ-CORE-062, condition 1).
    #[must_use]
    pub const fn is_on_screen(self) -> bool {
        self.x >= 0 && self.x < 256 && self.y >= 0 && self.y < RegionTiming::VISIBLE_SCANLINES
    }
}

/// The two controller ports at $4016 and $4017 (REQ-CORE-022).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPorts {
    types: [ControllerPortType; Self::PORT_COUNT],
    shift: [u8; Self::PORT_COUNT],
    shift_count: [u32; Self::PORT_COUNT],
    zapper: [ZapperPosition; Self::PORT_COUNT],
    input_word: u16,
    strobe: bool,
    /// The cycle from which a raised strobe's reload of the shift registers counts, 0 while none
    /// is waiting. See [`InputPorts::write`]; transient, derived, and outside the savestate.
    reload_at: i64,
    /// The VS. System of the image loaded, or `None` for a NES image — which is **every** image
    /// as far as the paths above are concerned, this field being read only by the `$4016` and
    /// `$4017` arms of `NesBus::read_io_register`, which are `#[cold]` and out of line.
    ///
    /// It lives here, and not beside the cartridge, because what it carries is the cabinet: two
    /// coin slots, a service button and eight DIP switches that answer on the two controller
    /// ports (lot MF-VS). Like [`Self::types`], it is **configuration** and survives
    /// [`Self::reset`]: a power cycle does not unplug the arcade board.
    #[cfg(feature = "board-vs-system")]
    vs: Option<Box<crate::vs::VsSystem>>,
}

impl Default for InputPorts {
    fn default() -> Self {
        Self::new()
    }
}

impl InputPorts {
    /// Number of ports.
    pub const PORT_COUNT: usize = 2;

    /// Creates the ports with two standard pads and no button pressed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            types: [ControllerPortType::Controller; Self::PORT_COUNT],
            shift: [0; Self::PORT_COUNT],
            shift_count: [0; Self::PORT_COUNT],
            zapper: [ZapperPosition::OFF_SCREEN; Self::PORT_COUNT],
            input_word: 0,
            strobe: false,
            reload_at: 0,
            #[cfg(feature = "board-vs-system")]
            vs: None,
        }
    }

    /// Plugs in (or unplugs) the VS. System of the image loaded; see [`Self::vs`].
    #[cfg(feature = "board-vs-system")]
    pub fn set_vs_system(&mut self, vs: Option<crate::vs::VsSystem>) {
        self.vs = vs.map(Box::new);
    }

    /// The VS. System of the image loaded, or `None` for a NES image.
    #[cfg(feature = "board-vs-system")]
    #[must_use]
    pub fn vs(&self) -> Option<&crate::vs::VsSystem> {
        self.vs.as_deref()
    }

    /// The same, to drive the cabinet: insert a coin, hold the service button, set the DIP
    /// switches.
    #[cfg(feature = "board-vs-system")]
    pub fn vs_mut(&mut self) -> Option<&mut crate::vs::VsSystem> {
        self.vs.as_deref_mut()
    }

    /// The input word of the current frame: port 0 in the low byte, port 1 in the high byte.
    #[must_use]
    pub const fn input_word(&self) -> u16 {
        self.input_word
    }

    const fn check(port: usize) -> Result<(), InvalidPort> {
        if port < Self::PORT_COUNT {
            Ok(())
        } else {
            Err(InvalidPort(port))
        }
    }

    /// What is plugged into `port`.
    ///
    /// # Errors
    ///
    /// [`InvalidPort`] when `port` is not 0 or 1.
    pub const fn port_type(&self, port: usize) -> Result<ControllerPortType, InvalidPort> {
        match Self::check(port) {
            Ok(()) => Ok(self.types[port]),
            Err(error) => Err(error),
        }
    }

    /// Plugs `kind` into `port`.
    ///
    /// # Errors
    ///
    /// [`InvalidPort`] when `port` is not 0 or 1.
    pub fn set_port_type(
        &mut self,
        port: usize,
        kind: ControllerPortType,
    ) -> Result<(), InvalidPort> {
        Self::check(port)?;
        self.types[port] = kind;
        Ok(())
    }

    /// Latches the input word of a new frame (REQ-CORE-022). While the strobe is held high
    /// the shift registers reload continuously.
    pub fn begin_frame(&mut self, input_word: u16) {
        // A VS. System's cabinet counts its coin lines down and its wiring presents the word to
        // the two ports its own way: **once a frame, here, and never per read** (lot MF-VS
        // step 5, `crate::vs::VsSystem::input_word`).
        #[cfg(feature = "board-vs-system")]
        let input_word = match self.vs.as_deref_mut() {
            Some(vs) => {
                vs.begin_frame();
                vs.input_word(input_word)
            }
            None => input_word,
        };

        self.input_word = input_word;
        if self.strobe {
            self.reload();
            self.reload_at = 0;
        }
    }

    /// Stores where the Zapper of `port` is aimed this frame (REQ-CORE-060).
    ///
    /// # Errors
    ///
    /// [`InvalidPort`] when `port` is not 0 or 1.
    pub fn set_zapper_position(
        &mut self,
        port: usize,
        position: ZapperPosition,
    ) -> Result<(), InvalidPort> {
        Self::check(port)?;
        self.zapper[port] = position;
        Ok(())
    }

    /// Asks `source` for the aim of every port that has a Zapper plugged in (REQ-CORE-060).
    pub fn aim_zappers(&mut self, mut source: impl FnMut(usize) -> ZapperPosition) {
        for port in 0..Self::PORT_COUNT {
            if self.types[port] == ControllerPortType::Zapper {
                self.zapper[port] = source(port);
            }
        }
    }

    /// Where the Zapper of `port` is aimed.
    ///
    /// # Errors
    ///
    /// [`InvalidPort`] when `port` is not 0 or 1.
    pub const fn zapper_position(&self, port: usize) -> Result<ZapperPosition, InvalidPort> {
        match Self::check(port) {
            Ok(()) => Ok(self.zapper[port]),
            Err(error) => Err(error),
        }
    }

    /// The eight buttons of `port` as a byte: A, B, Select, Start, Up, Down, Left, Right
    /// from bit 0 to bit 7 (REQ-CORE-022). Zero for a port that does not exist.
    #[must_use]
    pub const fn port_byte(&self, port: usize) -> u8 {
        if port < Self::PORT_COUNT {
            self.input_word.to_le_bytes()[port]
        } else {
            0
        }
    }

    /// Reads $4016 (port 0) or $4017 (port 1) on CPU cycle `cycle`. Bits 5 to 7 are the stale bus
    /// contents. `sees_light` answers the Zapper's light detection for an aim (REQ-CORE-062); it
    /// is only called for a Zapper port.
    pub fn read(
        &mut self,
        port: usize,
        open_bus: u8,
        cycle: i64,
        sees_light: impl FnOnce(ZapperPosition) -> bool,
    ) -> u8 {
        if port >= Self::PORT_COUNT {
            return open_bus & 0xE0;
        }

        self.settle_strobe(cycle);

        if self.types[port] == ControllerPortType::Zapper {
            // Bit 4 is the trigger, copied straight from the input word; bit 3 is 0 when
            // light is seen; every other bit is zero (REQ-CORE-061).
            let mut value = self.port_byte(port) & 0x10;
            if !sees_light(self.zapper[port]) {
                value |= 0x08;
            }

            return value;
        }

        let bit = if self.strobe {
            // Reads taken while the strobe is high always show button A.
            self.shift[port] & 0x01
        } else if self.shift_count[port] >= 8 {
            // The ninth read and the ones after it return 1 on an official controller
            // (REQ-CORE-022, notes).
            1
        } else {
            let bit = self.shift[port] & 0x01;
            self.shift[port] >>= 1;
            self.shift_count[port] += 1;
            bit
        };

        // Bits 5 to 7 of a $4016/$4017 read are the stale bus contents (nesdev) — **except on a
        // VS. System**, where the cabinet drives bits 2 to 7 with the coin slots, the service
        // button and the eight DIP switches (`crate::vs::VsSystem::port_bits`). One test of an
        // `Option`, on a path `NesBus::read_io_register` already keeps `#[cold]` and out of line.
        #[cfg(feature = "board-vs-system")]
        if let Some(vs) = self.vs.as_deref() {
            return bit | vs.port_bits(port);
        }

        bit | (open_bus & 0xE0)
    }

    /// Writes $4016 on CPU cycle `cycle`: bit 0 is the strobe (REQ-CORE-022).
    ///
    /// **The shift registers reload on a put cycle the strobe line is high for, and the line a
    /// write raises is high from the cycle *after* it.** `AccuracyCoin`'s `Controller Strobing`
    /// 3 and 4 ask it as a question — "Do controller strobes only happen when the CPU
    /// transitions from a get cycle to a put cycle?" — and answer it with `DEC $4016`, which
    /// writes `$41` on its fifth cycle and `$40` on its sixth, holding the line high for exactly
    /// one cycle. The ROM annotates both runs cycle by cycle, and the `LDA <$00` before the
    /// second one carries three tokens for its three cycles, which fixes the alignment: each
    /// `DEC` carries seven tokens for six cycles, so the last one is the cycle *after* the
    /// instruction.
    ///
    /// * test 3 — `(get) (put) (get) (put) (get) [PUT]`: the raising write is on cycle 5, a
    ///   **get** cycle, and cycle 6 is a **put**. The controllers **are** strobed, and the test
    ///   reads the pad and requires `$00`, the unpressed buttons.
    /// * test 4 — `(put) (get) (put) (get) (put) [GET]`: the raising write is on a **put** cycle
    ///   and cycle 6 is a **get**. They are **not strobed at all**, and the test requires `$FF`,
    ///   a shift register left depleted by the reads before it.
    ///
    /// Both rows are the same rule: the value a write puts on the bus latches at the **end** of
    /// its cycle, so the line raised on cycle 5 is high *during cycle 6* and the reload fires on
    /// cycle 6 exactly when cycle 6 is a put cycle. The cycle-6 write of `$40` only takes the
    /// line low from cycle 7, which is why it does not undo test 3's reload — and why in test 4,
    /// where the reload was waiting for cycle 7, it does.
    ///
    /// So a write that raises the line arms the reload for the first put cycle **after** its own
    /// cycle — a put cycle is an **odd** one in this core's numbering, where a get cycle is even
    /// ([`NesBus::run_dma`](crate::NesBus)) — and a write that lowers it applies a reload armed
    /// for a cycle at or before its own and cancels any armed for a later one.
    ///
    /// A line held high for more than one cycle necessarily covers a put cycle, which is why the
    /// `LDA #1 / STA $4016 / LDA #0 / STA $4016` of every game reads the pad exactly as it did
    /// before: the two writes are at least two cycles apart, and the armed cycle is at most
    /// `cycle + 2`.
    ///
    /// The reload is applied **lazily**, by [`Self::settle_strobe`] from the next read or write:
    /// nothing per cycle has to ask.
    pub fn write(&mut self, value: u8, cycle: i64) {
        let strobe = value & 0x01 != 0;

        // A reload armed for this cycle or an earlier one has happened: this write's own level
        // only reaches the line from the next cycle, so it cannot take it back.
        self.settle_strobe(cycle);

        // The first put cycle this level covers, counted from the cycle after this one, or none
        // while the line is low.
        self.reload_at = if strobe { (cycle + 1) | 1 } else { 0 };
        self.strobe = strobe;
    }

    /// Reloads the shift registers when a strobe armed by [`Self::write`] has reached a put cycle
    /// at or before `through`. Called from every observer of the registers, so the deferral costs
    /// nothing per cycle; the reload itself is idempotent while `input_word` stands still.
    fn settle_strobe(&mut self, through: i64) {
        if self.reload_at != 0 && through >= self.reload_at {
            self.reload_at = 0;
            self.reload();
        }
    }

    /// Returns the ports to their power-on state; the plugged devices stay (as in the
    /// reference).
    pub fn reset(&mut self) {
        self.input_word = 0;
        self.strobe = false;
        self.reload_at = 0;
        self.shift = [0; Self::PORT_COUNT];
        self.shift_count = [0; Self::PORT_COUNT];
        self.zapper = [ZapperPosition::OFF_SCREEN; Self::PORT_COUNT];
    }

    fn reload(&mut self) {
        for port in 0..Self::PORT_COUNT {
            self.shift[port] = self.port_byte(port);
            self.shift_count[port] = 0;
        }
    }
}

impl InputPorts {
    /// Appends the port state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u16(self.input_word);
        writer.write_bool(self.strobe);
        for port in 0..Self::PORT_COUNT {
            writer.write_u8(self.types[port] as u8);
            writer.write_u8(self.shift[port]);
            writer.write_u32(self.shift_count[port]);
            writer.write_i32(self.zapper[port].x);
            writer.write_i32(self.zapper[port].y);
        }

        // The cabinet, **for a VS machine only**, so that a NES block is byte for byte the block
        // it was: see `crate::vs::VsSystem::save_state` for why that leaves the format version
        // alone.
        #[cfg(feature = "board-vs-system")]
        if let Some(vs) = self.vs.as_deref() {
            vs.save_state(writer);
        }
    }

    /// Restores the port state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or names an unknown device.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.input_word = reader.read_u16()?;
        self.strobe = reader.read_bool()?;
        // Not in the block, and nothing is lost (REQ-CORE-091, the format is unchanged). A strobe
        // that is **high** reloads on every put cycle it is high for, so a reload is always due:
        // 1 is before any cycle a loaded state can be at, and the first observer settles it. A
        // strobe that is low can have no reload armed, [`Self::write`] having taken it back.
        //
        // The reload is armed rather than performed here so that the bytes just read stay exactly
        // as the block gave them and saving again writes the same ones (REQ-CORE-092). In the
        // ordinary case — a `$4016` write of 1 some cycles ago — it settles to a no-op anyway,
        // the registers already holding `port_byte` with a count of 0.
        self.reload_at = i64::from(self.strobe);
        for port in 0..Self::PORT_COUNT {
            self.types[port] = match reader.read_u8()? {
                0 => ControllerPortType::Controller,
                1 => ControllerPortType::Zapper,
                other => {
                    return Err(InvalidState::corrupt(format!(
                        "Corrupt savestate: unknown controller port type {other}."
                    )))
                }
            };
            self.shift[port] = reader.read_u8()?;
            self.shift_count[port] = reader.read_u32()?;
            let x = reader.read_i32()?;
            let y = reader.read_i32()?;
            self.zapper[port] = ZapperPosition::new(x, y);
        }

        // The machine the block is being loaded into is the one the block was written from — the
        // state carries the ROM — so the reader knows whether these bytes are there.
        #[cfg(feature = "board-vs-system")]
        if let Some(vs) = self.vs.as_deref_mut() {
            vs.load_state(reader)?;
        }

        Ok(())
    }
}
