namespace Zax.Nes;

/// <summary>
/// TV region of the emulated console (SPEC-02 §4.5).
/// </summary>
/// <remarks>
/// The numeric values are observable through the persisted settings (SPEC-09) and
/// must not change: NTSC = 0, PAL = 1, MULTI = 2, DENDY = 3.
/// </remarks>
public enum NesRegion
{
    /// <summary>NTSC timing, 60 fps nominal host rate.</summary>
    Ntsc = 0,

    /// <summary>PAL timing, 50 fps nominal host rate.</summary>
    Pal = 1,

    /// <summary>Dual-compatible cartridge; the core runs NTSC timing (REQ-CORE-015).</summary>
    Multi = 2,

    /// <summary>Dendy timing: PAL-length frames with an NTSC-like CPU clock.</summary>
    Dendy = 3,
}
