namespace Zax.Nes;

/// <summary>
/// P0 scaffolding marker for the NES core library.
/// </summary>
/// <remarks>
/// <para>
/// This type carries no emulation behaviour on purpose. The core is written in lot L1
/// against SPEC-02 (core contract: tile emission, regions and timing, extended buttons,
/// audio, savestates) and public documentation only.
/// </para>
/// <para>
/// Architectural constraints that apply to every type added to this assembly
/// (PLAN.md, libretro portability section): no platform dependency, no window, no clock
/// of its own, no mutable global state, and one emulation instance per object.
/// </para>
/// </remarks>
public static class NesModule
{
    /// <summary>Stable identifier of this module, used by the headless harness and tests.</summary>
    public const string Name = "Zax.Nes";

    /// <summary>Short human-readable role of this module.</summary>
    public const string Role = "NES core (CPU, PPU, APU, mappers)";
}
