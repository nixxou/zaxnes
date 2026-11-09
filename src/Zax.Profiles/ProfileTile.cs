namespace Zax.Profiles;

/// <summary>
/// How a tile of a « 3dh » profile names its content before it is resolved (AMD-062
/// REQ-PROF-074): the manifest string it refers to and the field it was written in.
/// </summary>
/// <param name="Text">
/// The manifest string: 32 hexadecimal digits for a tile identifier (REQ-PROF-075), 64 for a
/// legacy digest (REQ-PROF-076).
/// </param>
/// <param name="StoredAsIdentifier">
/// True when the tile carried field 5, the sixteen bytes of the identifier; false when it
/// carried field 4, the string. The writer puts it back in the field it came from, which is what
/// makes the rewrite of REQ-PROF-084 lossless whichever form the file used.
/// </param>
public readonly record struct TileHashReference(string Text, bool StoredAsIdentifier)
{
    /// <summary>True when the string is a tile identifier rather than a legacy digest.</summary>
    public bool IsIdentifier => Text.Length == Zax.Profiles.Hash.TileId.TextLength;
}

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
/// A tile of a « 3dh » profile carries a <see cref="Hash"/> instead of its planes and is
/// <see cref="IsPending">pending</see> until the tile is found in the ROM or in the running game
/// (AMD-062 REQ-PROF-078, REQ-PROF-080). A pending tile equals no ordinary tile and no tile of
/// the screen, and equals another pending tile only when both name the same manifest string with
/// the same flip (REQ-PROF-081). Resolution is monotone and happens between frames, which is
/// what lets the planes and therefore the equality of this object change in place; a container
/// keyed by tiles must be rebuilt afterwards, which is what REQ-PROF-080 requires anyway.
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

    /// <summary>
    /// Creates a tile of a « 3dh » profile, pending until its content is found (REQ-PROF-074).
    /// </summary>
    public ProfileTile(TileHashReference hash, TileFlip flip)
    {
        Hash = hash;
        Flip = flip;
        IsPending = true;
    }

    /// <summary>Low bit plane, in clear; zero while the tile is pending.</summary>
    public ulong LowPlane { get; private set; }

    /// <summary>High bit plane, in clear; zero while the tile is pending.</summary>
    public ulong HighPlane { get; private set; }

    /// <summary>Mirroring flags.</summary>
    public TileFlip Flip { get; }

    /// <summary>
    /// The manifest string this tile names, or <see langword="null"/> for an ordinary tile that
    /// carries its planes. It stays set once the tile is resolved, because the writer puts the
    /// identifier back rather than the planes (REQ-PROF-084).
    /// </summary>
    public TileHashReference? Hash { get; }

    /// <summary>
    /// True while the content of this tile is unknown (REQ-PROF-081). Always false for a tile
    /// that carries its planes.
    /// </summary>
    public bool IsPending { get; private set; }

    /// <summary>The manifest string, or <see langword="null"/> for an ordinary tile.</summary>
    public string? HashText => Hash?.Text;

    /// <summary>True when the horizontal mirror bit is set.</summary>
    public bool FlipX => (Flip & TileFlip.Horizontal) != 0;

    /// <summary>True when the vertical mirror bit is set.</summary>
    public bool FlipY => (Flip & TileFlip.Vertical) != 0;

    /// <summary>
    /// Gives a pending tile the content that was found for it (REQ-PROF-078, REQ-PROF-080).
    /// Resolving an already resolved tile does nothing, which is what makes the rule monotone.
    /// </summary>
    /// <returns>True when this call resolved the tile.</returns>
    public bool Resolve(ulong lowPlane, ulong highPlane)
    {
        if (!IsPending)
        {
            return false;
        }

        LowPlane = lowPlane;
        HighPlane = highPlane;
        IsPending = false;
        return true;
    }

    /// <summary>
    /// Colour index, 0 to 3, of the <b>displayed</b> pixel at (x, y), mirroring applied
    /// (REQ-PROF-031). Colour 0 is transparent, which is what every pixel of a pending tile is
    /// (REQ-PROF-081).
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
    {
        if (other is null || Flip != other.Flip)
        {
            return false;
        }

        // REQ-PROF-081: a pending tile is equal to no tile of the screen and to no resolved
        // tile; two pending tiles are equal exactly when they name the same manifest string.
        if (IsPending || other.IsPending)
        {
            return IsPending
                && other.IsPending
                && string.Equals(HashText, other.HashText, StringComparison.Ordinal);
        }

        return LowPlane == other.LowPlane && HighPlane == other.HighPlane;
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as ProfileTile);

    /// <inheritdoc/>
    public override int GetHashCode()
        => IsPending
            ? HashCode.Combine(HashText, (int)Flip, true)
            : HashCode.Combine(LowPlane, HighPlane, (int)Flip);

    /// <inheritdoc/>
    public override string ToString()
        => IsPending
            ? $"Tile(pending {HashText}, flip={Flip})"
            : $"Tile(low=0x{LowPlane:X16}, high=0x{HighPlane:X16}, flip={Flip})";
}
