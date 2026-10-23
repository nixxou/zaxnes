//! The 2A03 APU (port of `src/Zax.Nes/Apu`).
//!
//! Stub for T01: silent, no IRQ, no DMC fetch. T03 replaces it, keeping the bus-facing
//! methods.

/// The audio processing unit.
#[derive(Debug, Clone, Default)]
pub struct Apu2A03;

#[allow(clippy::unused_self)] // Stub until T03.
impl Apu2A03 {
    /// Creates an APU in its power-on state.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Whether the frame counter or the DMC holds /IRQ low.
    #[must_use]
    pub const fn irq_asserted(&self) -> bool {
        false
    }

    /// Whether the DMC wants a sample byte fetched by DMA.
    #[must_use]
    pub const fn needs_dmc_dma(&self) -> bool {
        false
    }

    /// Address of the next DMC sample byte.
    #[must_use]
    pub const fn dmc_dma_address(&self) -> u16 {
        0
    }

    /// Hands the DMC the byte its DMA fetched.
    pub fn supply_dmc_byte(&mut self, _value: u8) {}

    /// Advances the APU by one CPU cycle.
    pub fn step_cpu_cycle(&mut self) {}

    /// Reads $4015.
    pub fn read_status(&mut self, _open_bus: u8) -> u8 {
        0
    }

    /// Writes an APU register ($4000-$4017, except $4014 and $4016).
    pub fn write(&mut self, _address: u16, _value: u8) {}
}
