//! Controller ports (port of `src/Zax.Nes/Input`).
//!
//! Stub for T01: no controller plugged in. T04 replaces it, keeping the bus-facing methods.

/// The two controller ports at $4016 and $4017.
#[derive(Debug, Clone, Default)]
pub struct InputPorts;

#[allow(clippy::unused_self)] // Stub until T04.
impl InputPorts {
    /// Creates the ports with nothing plugged in.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Reads $4016 (port 0) or $4017 (port 1). Bits 5 to 7 are the stale bus contents.
    pub fn read(&mut self, _port: usize, open_bus: u8) -> u8 {
        open_bus & 0xE0
    }

    /// Writes $4016: bit 0 is the strobe (REQ-CORE-022).
    pub fn write(&mut self, _value: u8) {}
}
