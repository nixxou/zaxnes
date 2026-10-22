namespace Zax.Nes;

/// <summary>
/// Layout of the four logical nametables on cartridge/console VRAM (SPEC-02 §2,
/// "current mirroring"). Mappers may change it while the game runs.
/// </summary>
public enum NametableMirroring
{
    /// <summary>$2000 = $2400 and $2800 = $2C00.</summary>
    Horizontal = 0,

    /// <summary>$2000 = $2800 and $2400 = $2C00.</summary>
    Vertical = 1,

    /// <summary>All four logical nametables show the first physical one.</summary>
    SingleScreenA = 2,

    /// <summary>All four logical nametables show the second physical one.</summary>
    SingleScreenB = 3,

    /// <summary>Four distinct nametables, backed by 4 KiB of VRAM.</summary>
    FourScreen = 4,
}
