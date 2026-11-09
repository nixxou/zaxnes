using System.Buffers.Binary;
using System.Security.Cryptography;

namespace Zax.Profiles.Hash;

/// <summary>
/// The sixteen bytes of a tile as they sit in a ROM — low plane in <c>b[0..7]</c>, high plane in
/// <c>b[8..15]</c> — and the operations a « 3dh » profile needs on them (AMD-062 §5, §6).
/// </summary>
/// <remarks>
/// Read as little-endian 64-bit integers the two halves are exactly the planes in clear of
/// SPEC-03 REQ-PROF-030: bit <c>y × 8 + (7 − x)</c> of a plane is the bit of pixel (x, y). The
/// flip of a profile tile is stored apart, so a raw tile is always a <b>non-flipped</b> tile.
/// </remarks>
public static class RawTile
{
    /// <summary>Number of bytes of a raw tile.</summary>
    public const int ByteCount = 16;

    /// <summary>Number of bytes of one bit plane.</summary>
    public const int PlaneByteCount = 8;

    private static readonly byte[] ReversedBits = BuildReversedBits();

    /// <summary>The two planes in clear of a raw tile.</summary>
    /// <exception cref="ArgumentException"><paramref name="raw"/> is not sixteen bytes long.</exception>
    public static (ulong Low, ulong High) ToPlanes(ReadOnlySpan<byte> raw)
    {
        RequireTile(raw);
        return (
            BinaryPrimitives.ReadUInt64LittleEndian(raw),
            BinaryPrimitives.ReadUInt64LittleEndian(raw[PlaneByteCount..]));
    }

    /// <summary>Writes the sixteen raw bytes of a tile given by its planes in clear.</summary>
    /// <exception cref="ArgumentException"><paramref name="destination"/> is too short.</exception>
    public static void FromPlanes(ulong low, ulong high, Span<byte> destination)
    {
        if (destination.Length < ByteCount)
        {
            throw new ArgumentException($"A raw tile needs {ByteCount} bytes.", nameof(destination));
        }

        BinaryPrimitives.WriteUInt64LittleEndian(destination, low);
        BinaryPrimitives.WriteUInt64LittleEndian(destination[PlaneByteCount..], high);
    }

    /// <summary>The sixteen raw bytes of a tile given by its planes in clear.</summary>
    public static byte[] FromPlanes(ulong low, ulong high)
    {
        byte[] raw = new byte[ByteCount];
        FromPlanes(low, high, raw);
        return raw;
    }

    /// <summary>
    /// Mirrors a raw tile (REQ-PROF-079). Bit 0 of <paramref name="flip"/> is the horizontal
    /// mirror, which reverses the eight bits of every byte; bit 1 is the vertical mirror, which
    /// reverses the order of the eight bytes <b>of each plane</b>.
    /// </summary>
    /// <exception cref="ArgumentException">A span has the wrong length.</exception>
    public static void Flip(ReadOnlySpan<byte> source, TileFlip flip, Span<byte> destination)
    {
        RequireTile(source);
        if (destination.Length < ByteCount)
        {
            throw new ArgumentException($"A raw tile needs {ByteCount} bytes.", nameof(destination));
        }

        bool horizontal = (flip & TileFlip.Horizontal) != 0;
        bool vertical = (flip & TileFlip.Vertical) != 0;
        for (int plane = 0; plane < 2; plane++)
        {
            int origin = plane * PlaneByteCount;
            for (int row = 0; row < PlaneByteCount; row++)
            {
                byte value = source[origin + (vertical ? PlaneByteCount - 1 - row : row)];
                destination[origin + row] = horizontal ? ReversedBits[value] : value;
            }
        }
    }

    /// <summary>The mirrored copy of a raw tile (REQ-PROF-079).</summary>
    public static byte[] Flipped(ReadOnlySpan<byte> source, TileFlip flip)
    {
        byte[] result = new byte[ByteCount];
        Flip(source, flip, result);
        return result;
    }

    /// <summary>
    /// The legacy digest of a raw tile (REQ-PROF-076): <c>SHA-256(b[0..15])</c> written as 64
    /// lowercase hexadecimal digits.
    /// </summary>
    public static string LegacyDigest(ReadOnlySpan<byte> raw)
    {
        RequireTile(raw);
        Span<byte> digest = stackalloc byte[32];
        SHA256.HashData(raw, digest);
        return Convert.ToHexStringLower(digest);
    }

    /// <summary>True when every one of the sixteen bytes is zero.</summary>
    public static bool IsBlank(ReadOnlySpan<byte> raw)
    {
        foreach (byte value in raw)
        {
            if (value != 0)
            {
                return false;
            }
        }

        return true;
    }

    private static void RequireTile(ReadOnlySpan<byte> raw)
    {
        if (raw.Length != ByteCount)
        {
            throw new ArgumentException($"A raw tile is {ByteCount} bytes long.", nameof(raw));
        }
    }

    private static byte[] BuildReversedBits()
    {
        byte[] table = new byte[256];
        for (int value = 0; value < 256; value++)
        {
            int reversed = 0;
            for (int bit = 0; bit < 8; bit++)
            {
                if ((value & (1 << bit)) != 0)
                {
                    reversed |= 1 << (7 - bit);
                }
            }

            table[value] = (byte)reversed;
        }

        return table;
    }
}
