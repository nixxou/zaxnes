namespace Zax.Profiles;

/// <summary>
/// One 8×8 NES pattern stored in a profile: two bit planes and the mirroring flags
/// (SPEC-03 REQ-PROF-030, REQ-PROF-031, REQ-PROF-032).
/// </summary>
/// <remarks>
/// <para>
/// The planes are held <b>in clear</b>: the reversible masking of the file
/// (<c>rotr64(plane XOR key, 26)</c>) belongs to the wire codec and never reaches this type.
/// Bit <c>y × 8 + (7 − x)</c> of a plane is the bit of pixel (x, y), with (0, 0) at the top
/// left and without mirroring applied — the same packing as the tiles produced by the core
/// (SPEC-02, <c>Zax.Nes.TileRecord</c>).
/// </para>
/// <para>
/// Two tiles are <b>the same tile</b> exactly when their planes and their flip are equal
/// (REQ-PROF-032), which is what <see cref="Equals(ProfileTile?)"/> implements. Sharing inside
/// a file is a separate matter: a profile may carry one tile object in several grid cells, and
/// the reader preserves that object identity (REQ-PROF-011), which is what the writer uses to
/// rebuild the shared references.
/// </para>
/// <para>
/// Everything else about a tile — pixel count, bounding box, colours used, density,
/// connectivity — is derived and must be recomputed by the conversion engine (SPEC-04); none
/// of it is stored.
/// </para>
/// </remarks>
public sealed class ProfileTile : IEquatable<ProfileTile>
{
    /// <summary>Creates a tile from its planes in clear.</summary>
    public ProfileTile(ulong lowPlane, ulong highPlane, TileFlip flip)
    {
        LowPlane = lowPlane;
        HighPlane = highPlane;
        Flip = flip;
    }

    /// <summary>Low bit plane, in clear.</summary>
    public ulong LowPlane { get; }

    /// <summary>High bit plane, in clear.</summary>
    public ulong HighPlane { get; }

    /// <summary>Mirroring flags.</summary>
    public TileFlip Flip { get; }

    /// <summary>True when the horizontal mirror bit is set.</summary>
    public bool FlipX => (Flip & TileFlip.Horizontal) != 0;

    /// <summary>True when the vertical mirror bit is set.</summary>
    public bool FlipY => (Flip & TileFlip.Vertical) != 0;

    /// <summary>
    /// Colour index, 0 to 3, of the <b>displayed</b> pixel at (x, y), mirroring applied
    /// (REQ-PROF-031). Colour 0 is transparent.
    /// </summary>
    /// <exception cref="ArgumentOutOfRangeException">A coordinate is outside 0..7.</exception>
    public int ColorAt(int x, int y)
    {
        if ((uint)x > 7)
        {
            throw new ArgumentOutOfRangeException(nameof(x));
        }

        if ((uint)y > 7)
        {
            throw new ArgumentOutOfRangeException(nameof(y));
        }

        int sourceX = FlipX ? 7 - x : x;
        int sourceY = FlipY ? 7 - y : y;
        int bit = (sourceY * 8) + 7 - sourceX;
        int low = (int)((LowPlane >> bit) & 1);
        int high = (int)((HighPlane >> bit) & 1);
        return (2 * high) + low;
    }

    /// <inheritdoc/>
    public bool Equals(ProfileTile? other)
        => other is not null
        && LowPlane == other.LowPlane
        && HighPlane == other.HighPlane
        && Flip == other.Flip;

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as ProfileTile);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(LowPlane, HighPlane, (int)Flip);

    /// <inheritdoc/>
    public override string ToString()
        => $"Tile(low=0x{LowPlane:X16}, high=0x{HighPlane:X16}, flip={Flip})";
}
