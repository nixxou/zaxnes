using System.Numerics;

namespace Zax.Profiles.Wire;

/// <summary>
/// The reversible masking of the tile bit planes (SPEC-03 REQ-PROF-030). The key is the
/// constant <c>tile_encoding_key</c> of <c>assets-neutral/format-data.json</c>.
/// </summary>
/// <remarks>
/// It applies to the two plane fields of <c>TilePlanes</c> only, wherever that message appears,
/// and to nothing else. The transformation is a bijection, so a profile survives a load and a
/// save unchanged.
/// </remarks>
public static class TilePlaneCipher
{
    /// <summary>The 64-bit key, <c>0x000708FACD884A5A</c>.</summary>
    public const ulong Key = 0x000708FACD884A5AUL;

    /// <summary>The rotation count, the low six bits of the key: 26.</summary>
    public const int Rotation = (int)(Key & 0x3F);

    /// <summary>Turns a stored value into the plane in clear: <c>rotl64(stored, 26) XOR key</c>.</summary>
    public static ulong Decode(long stored) => BitOperations.RotateLeft((ulong)stored, Rotation) ^ Key;

    /// <summary>Turns a plane in clear into its stored value: <c>rotr64(plane XOR key, 26)</c>.</summary>
    public static long Encode(ulong plane) => (long)BitOperations.RotateRight(plane ^ Key, Rotation);
}
