using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text;

namespace Zax.Profiles.Hash;

/// <summary>
/// The content identifier of a tile in a « 3dh » profile (AMD-062 REQ-PROF-075): a pair of
/// 64-bit integers derived from the sixteen raw bytes of the tile.
/// </summary>
/// <remarks>
/// <para>
/// <see cref="Low"/> carries the mixing prefix of REQ-PROF-077 in its low 32 bits and the first
/// four bytes of the digest in its high 32 bits; <see cref="High"/> carries bytes 4 to 11 of the
/// digest. Because the prefix is a cheap function of the two planes, a scan can compute it for
/// every window of a ROM and only hash the windows whose prefix is in the profile's manifest.
/// </para>
/// <para>
/// The <b>binary form</b> (envelope field 12, tile field 5) is <see cref="Low"/> then
/// <see cref="High"/>, each little-endian. The <b>text form</b>, which is what every comparison
/// to the manifest uses, is 32 lowercase hexadecimal digits, <see cref="High"/> first.
/// </para>
/// </remarks>
public readonly struct TileId : IEquatable<TileId>
{
    /// <summary>Number of bytes of the binary form.</summary>
    public const int ByteCount = 16;

    /// <summary>Number of hexadecimal digits of the text form.</summary>
    public const int TextLength = 32;

    /// <summary>The domain string the digest is prefixed with; a datum of the format.</summary>
    public const string DomainString = "3dSen.TileId.v1";

    private static readonly byte[] DomainBytes = Encoding.ASCII.GetBytes(DomainString);

    /// <summary>Builds an identifier from its two halves.</summary>
    public TileId(ulong low, ulong high)
    {
        Low = low;
        High = high;
    }

    /// <summary>First half, <c>L</c>: the prefix and the first four digest bytes.</summary>
    public ulong Low { get; }

    /// <summary>Second half, <c>H</c>: digest bytes 4 to 11.</summary>
    public ulong High { get; }

    /// <summary>The mixing prefix of REQ-PROF-077, which is the low 32 bits of <c>L</c>.</summary>
    public uint Prefix => (uint)Low;

    /// <summary>The identifier of a raw tile (REQ-PROF-075).</summary>
    /// <exception cref="ArgumentException"><paramref name="raw"/> is not sixteen bytes long.</exception>
    public static TileId Compute(ReadOnlySpan<byte> raw)
    {
        (ulong low, ulong high) = RawTile.ToPlanes(raw);
        return Compute(raw, ComputePrefix(low, high));
    }

    /// <summary>
    /// The identifier of a raw tile whose prefix has already been computed, which is what a scan
    /// does once the prefix has passed the manifest filter.
    /// </summary>
    public static TileId Compute(ReadOnlySpan<byte> raw, uint prefix)
    {
        if (raw.Length != RawTile.ByteCount)
        {
            throw new ArgumentException($"A raw tile is {RawTile.ByteCount} bytes long.", nameof(raw));
        }

        Span<byte> message = stackalloc byte[DomainString.Length + RawTile.ByteCount];
        DomainBytes.CopyTo(message);
        raw.CopyTo(message[DomainString.Length..]);

        Span<byte> digest = stackalloc byte[32];
        SHA256.HashData(message, digest);

        ulong low = prefix | ((ulong)BinaryPrimitives.ReadUInt32LittleEndian(digest) << 32);
        ulong high = BinaryPrimitives.ReadUInt64LittleEndian(digest[4..]);
        return new TileId(low, high);
    }

    /// <summary>
    /// The 32-bit mixing prefix <c>P</c> of REQ-PROF-077, a function of the two planes in clear.
    /// The six steps are a measured mixing function with no meaning of their own; they are
    /// interoperability constants and are reproduced bit for bit.
    /// </summary>
    public static uint ComputePrefix(ulong low, ulong high)
    {
        ulong mixedHigh = Math.BigMul(low ^ high ^ 0x59973F0033362349UL, 0x9E3779B185EBCA87UL, out ulong mixedLow);
        mixedLow += 0x03C0000000000000UL;

        ulong h2 = high ^ 0xC202797692D63D58UL;
        mixedHigh += h2 + ((h2 & 0xFFFFFFFFUL) * 0x85EBCA76UL);

        mixedLow ^= BinaryPrimitives.ReverseEndianness(mixedHigh);

        ulong v = mixedLow * 0xC2B2AE3D27D4EB4FUL;
        v ^= v >> 37;
        v *= 0x165667919E3779F9UL;
        v ^= v >> 32;
        return (uint)v;
    }

    /// <summary>The prefix of a raw tile.</summary>
    public static uint ComputePrefix(ReadOnlySpan<byte> raw)
    {
        (ulong low, ulong high) = RawTile.ToPlanes(raw);
        return ComputePrefix(low, high);
    }

    /// <summary>Reads the binary form: <c>L</c> then <c>H</c>, each little-endian.</summary>
    /// <exception cref="ArgumentException"><paramref name="bytes"/> is shorter than sixteen bytes.</exception>
    public static TileId FromBinary(ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length < ByteCount)
        {
            throw new ArgumentException($"An identifier is {ByteCount} bytes long.", nameof(bytes));
        }

        return new TileId(
            BinaryPrimitives.ReadUInt64LittleEndian(bytes),
            BinaryPrimitives.ReadUInt64LittleEndian(bytes[8..]));
    }

    /// <summary>Writes the binary form.</summary>
    /// <exception cref="ArgumentException"><paramref name="destination"/> is too short.</exception>
    public void WriteBinary(Span<byte> destination)
    {
        if (destination.Length < ByteCount)
        {
            throw new ArgumentException($"An identifier is {ByteCount} bytes long.", nameof(destination));
        }

        BinaryPrimitives.WriteUInt64LittleEndian(destination, Low);
        BinaryPrimitives.WriteUInt64LittleEndian(destination[8..], High);
    }

    /// <summary>The sixteen bytes of the binary form.</summary>
    public byte[] ToBinary()
    {
        byte[] bytes = new byte[ByteCount];
        WriteBinary(bytes);
        return bytes;
    }

    /// <summary>
    /// The text form: 32 lowercase hexadecimal digits, <c>H</c> on the first sixteen and
    /// <c>L</c> on the last sixteen. This is the form the manifest holds.
    /// </summary>
    public string ToText() => $"{High:x16}{Low:x16}";

    /// <summary>
    /// Reads a text form. Accepts exactly 32 lowercase hexadecimal digits, which is what the
    /// format writes; an entry in any other shape is not an identifier.
    /// </summary>
    public static bool TryParseText(string? text, out TileId id)
    {
        id = default;
        if (text is null || text.Length != TextLength || !HexText.IsLowercaseHex(text))
        {
            return false;
        }

        ulong high = ulong.Parse(text.AsSpan(0, 16), System.Globalization.NumberStyles.HexNumber, null);
        ulong low = ulong.Parse(text.AsSpan(16, 16), System.Globalization.NumberStyles.HexNumber, null);
        id = new TileId(low, high);
        return true;
    }

    /// <inheritdoc/>
    public bool Equals(TileId other) => Low == other.Low && High == other.High;

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is TileId other && Equals(other);

    /// <inheritdoc/>
    public override int GetHashCode() => HashCode.Combine(Low, High);

    /// <inheritdoc/>
    public override string ToString() => ToText();

    /// <summary>Equality.</summary>
    public static bool operator ==(TileId left, TileId right) => left.Equals(right);

    /// <summary>Inequality.</summary>
    public static bool operator !=(TileId left, TileId right) => !left.Equals(right);
}

/// <summary>Shape tests on the hexadecimal strings of the manifest (AMD-062 §3).</summary>
public static class HexText
{
    /// <summary>Length of a legacy digest (REQ-PROF-076).</summary>
    public const int LegacyDigestLength = 64;

    /// <summary>True when every character is a digit or a lowercase hexadecimal letter.</summary>
    public static bool IsLowercaseHex(ReadOnlySpan<char> text)
    {
        foreach (char character in text)
        {
            bool digit = character is >= '0' and <= '9';
            bool letter = character is >= 'a' and <= 'f';
            if (!digit && !letter)
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>True when <paramref name="text"/> is 64 lowercase hexadecimal digits.</summary>
    public static bool IsLegacyDigest(string? text)
        => text is { Length: LegacyDigestLength } && IsLowercaseHex(text);

    /// <summary>True when <paramref name="text"/> is 32 lowercase hexadecimal digits.</summary>
    public static bool IsIdentifierText(string? text)
        => text is { Length: TileId.TextLength } && IsLowercaseHex(text);
}
