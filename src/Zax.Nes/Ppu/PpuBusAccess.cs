namespace Zax.Nes.Ppu;

/// <summary>
/// Why the PPU bus is being accessed, which decides whether the cartridge may react to it
/// (REQ-CORE-033).
/// </summary>
public enum PpuBusAccess
{
    /// <summary>A real rendering fetch: the cartridge sees the address and may react.</summary>
    Render = 0,

    /// <summary>
    /// A $2006 or $2007 access by the CPU: the address reaches the cartridge too, which is
    /// how <c>mmc3_test</c> clocks an MMC3 counter with rendering switched off.
    /// </summary>
    Cpu = 1,

    /// <summary>
    /// A read made to build the tile records or to answer the host: no A12 counting, no
    /// latch toggling, no read buffer change (REQ-CORE-033).
    /// </summary>
    Inert = 2,
}
