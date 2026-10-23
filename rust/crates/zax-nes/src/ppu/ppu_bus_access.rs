/// Why the PPU bus is being accessed, which decides whether the cartridge may react to it
/// (REQ-CORE-033).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PpuBusAccess {
    /// A real rendering fetch: the cartridge sees the address and may react.
    Render = 0,
    /// A $2006 or $2007 access by the CPU: the address reaches the cartridge too, which is
    /// how `mmc3_test` clocks an MMC3 counter with rendering switched off.
    Cpu = 1,
    /// A read made to build the tile records or to answer the host: no A12 counting, no
    /// latch toggling, no read buffer change (REQ-CORE-033).
    Inert = 2,
}
