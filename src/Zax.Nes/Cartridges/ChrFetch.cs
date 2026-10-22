namespace Zax.Nes.Cartridges;

/// <summary>
/// Which pattern fetch a CHR access belongs to.
/// </summary>
/// <remarks>
/// Only one required board cares: MMC5 holds two sets of CHR bank registers and, with 8x16
/// sprites selected, serves background fetches from one set and sprite fetches from the other
/// (nesdev, "MMC5"). Every other mapper ignores the distinction.
/// </remarks>
public enum ChrFetch
{
    /// <summary>A background pattern fetch, a $2007 access or a host peek.</summary>
    Background = 0,

    /// <summary>A sprite pattern fetch.</summary>
    Sprite = 1,
}
