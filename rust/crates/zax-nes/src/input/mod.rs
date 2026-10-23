//! Controller ports (port of `src/Zax.Nes/Input`): the standard pad, the Zapper and the two
//! ports at $4016 and $4017 (REQ-CORE-022, REQ-CORE-024, REQ-CORE-060, REQ-CORE-061).

use crate::state::{InvalidState, StateReader, StateWriter};
use crate::RegionTiming;

/// What is plugged into a controller port (REQ-CORE-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControllerPortType {
    /// A standard pad.
    #[default]
    Controller = 0,
    /// A Zapper light gun.
    Zapper = 1,
}

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

/// A port number outside 0 to 1 (REQ-CORE-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPort(pub usize);

impl std::fmt::Display for InvalidPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "port {} is out of range (0 or 1)", self.0)
    }
}

impl std::error::Error for InvalidPort {}

/// The two controller ports at $4016 and $4017 (REQ-CORE-022).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPorts {
    types: [ControllerPortType; Self::PORT_COUNT],
    shift: [u8; Self::PORT_COUNT],
    shift_count: [u32; Self::PORT_COUNT],
    zapper: [ZapperPosition; Self::PORT_COUNT],
    input_word: u16,
    strobe: bool,
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
        }
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
        self.input_word = input_word;
        if self.strobe {
            self.reload();
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

    /// Reads $4016 (port 0) or $4017 (port 1). Bits 5 to 7 are the stale bus contents.
    /// `sees_light` answers the Zapper's light detection for an aim (REQ-CORE-062); it is
    /// only called for a Zapper port.
    pub fn read(
        &mut self,
        port: usize,
        open_bus: u8,
        sees_light: impl FnOnce(ZapperPosition) -> bool,
    ) -> u8 {
        if port >= Self::PORT_COUNT {
            return open_bus & 0xE0;
        }

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

        // Bits 5 to 7 of a $4016/$4017 read are the stale bus contents (nesdev).
        bit | (open_bus & 0xE0)
    }

    /// Writes $4016: bit 0 is the strobe (REQ-CORE-022).
    pub fn write(&mut self, value: u8) {
        let strobe = value & 0x01 != 0;
        if strobe {
            self.reload();
        }

        self.strobe = strobe;
    }

    /// Returns the ports to their power-on state; the plugged devices stay (as in the
    /// reference).
    pub fn reset(&mut self) {
        self.input_word = 0;
        self.strobe = false;
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
    }

    /// Restores the port state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or names an unknown device.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.input_word = reader.read_u16()?;
        self.strobe = reader.read_bool()?;
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

        Ok(())
    }
}
