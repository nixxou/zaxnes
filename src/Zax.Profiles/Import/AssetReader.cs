using System.Buffers.Binary;
using System.Text;

namespace Zax.Profiles.Import;

/// <summary>
/// A bounds-checked cursor over the bytes of a Unity bundle or serialized file, with the byte
/// order as a property: the bundle header and its blocks info are big-endian, while a serialized
/// file announces its own order (AMD-030 §4.1, §4.3).
/// </summary>
/// <remarks>
/// Every read checks what is left and raises <see cref="UnityAssetFormatException"/> rather than
/// letting an <see cref="IndexOutOfRangeException"/> escape, so a truncated file always comes out
/// as a message (AMD-030 §5).
/// </remarks>
internal ref struct AssetReader
{
    /// <summary>
    /// Longest accepted null-terminated string. Generous for a version string or a
    /// <c>CAB-…</c> node path, and small enough that a corrupt file cannot make the reader walk
    /// a whole bundle looking for a zero byte.
    /// </summary>
    public const int MaximumStringLength = 4096;

    private readonly ReadOnlySpan<byte> _bytes;
    private int _position;

    /// <summary>Creates a cursor at the start of <paramref name="bytes"/>.</summary>
    public AssetReader(ReadOnlySpan<byte> bytes, bool bigEndian)
    {
        _bytes = bytes;
        BigEndian = bigEndian;
    }

    /// <summary>Whether the multi-byte numbers read big-endian.</summary>
    public bool BigEndian { get; set; }

    /// <summary>The cursor, in bytes from the start.</summary>
    public int Position
    {
        get => _position;
        set
        {
            if (value < 0 || value > _bytes.Length)
            {
                throw new UnityAssetFormatException(
                    $"a position of {value} falls outside the {_bytes.Length} bytes available.");
            }

            _position = value;
        }
    }

    /// <summary>How many bytes are left.</summary>
    public int Remaining => _bytes.Length - _position;

    /// <summary>Total length of the view.</summary>
    public int Length => _bytes.Length;

    /// <summary>Reads one byte.</summary>
    public byte ReadByte() => Take(1)[0];

    /// <summary>Reads <paramref name="count"/> bytes without copying them.</summary>
    public ReadOnlySpan<byte> ReadBytes(int count)
    {
        if (count < 0)
        {
            throw new UnityAssetFormatException($"a length of {count} bytes is not valid.");
        }

        return Take(count);
    }

    /// <summary>Skips <paramref name="count"/> bytes.</summary>
    public void Skip(int count)
    {
        if (count < 0)
        {
            throw new UnityAssetFormatException($"a skip of {count} bytes is not valid.");
        }

        Take(count);
    }

    /// <summary>Reads a signed 16-bit number.</summary>
    public short ReadInt16()
    {
        ReadOnlySpan<byte> bytes = Take(2);
        return BigEndian
            ? BinaryPrimitives.ReadInt16BigEndian(bytes)
            : BinaryPrimitives.ReadInt16LittleEndian(bytes);
    }

    /// <summary>Reads an unsigned 16-bit number.</summary>
    public ushort ReadUInt16()
    {
        ReadOnlySpan<byte> bytes = Take(2);
        return BigEndian
            ? BinaryPrimitives.ReadUInt16BigEndian(bytes)
            : BinaryPrimitives.ReadUInt16LittleEndian(bytes);
    }

    /// <summary>Reads a signed 32-bit number.</summary>
    public int ReadInt32()
    {
        ReadOnlySpan<byte> bytes = Take(4);
        return BigEndian
            ? BinaryPrimitives.ReadInt32BigEndian(bytes)
            : BinaryPrimitives.ReadInt32LittleEndian(bytes);
    }

    /// <summary>Reads an unsigned 32-bit number.</summary>
    public uint ReadUInt32()
    {
        ReadOnlySpan<byte> bytes = Take(4);
        return BigEndian
            ? BinaryPrimitives.ReadUInt32BigEndian(bytes)
            : BinaryPrimitives.ReadUInt32LittleEndian(bytes);
    }

    /// <summary>Reads a signed 64-bit number.</summary>
    public long ReadInt64()
    {
        ReadOnlySpan<byte> bytes = Take(8);
        return BigEndian
            ? BinaryPrimitives.ReadInt64BigEndian(bytes)
            : BinaryPrimitives.ReadInt64LittleEndian(bytes);
    }

    /// <summary>
    /// Reads a UTF-8 string up to its terminating zero byte, which is consumed.
    /// </summary>
    public string ReadNullTerminatedString()
    {
        int start = _position;
        while (true)
        {
            if (_position >= _bytes.Length)
            {
                throw new UnityAssetFormatException(
                    "a text field is not terminated before the end of the file.");
            }

            if (_bytes[_position] == 0)
            {
                string text = Encoding.UTF8.GetString(_bytes[start.._position]);
                _position++;
                return text;
            }

            if (_position - start >= MaximumStringLength)
            {
                throw new UnityAssetFormatException(
                    $"a text field is longer than the accepted {MaximumStringLength} bytes.");
            }

            _position++;
        }
    }

    /// <summary>
    /// Moves the cursor forward to the next multiple of <paramref name="alignment"/>, counted
    /// from <paramref name="origin"/>.
    /// </summary>
    /// <param name="alignment">The alignment, 16 for the bundle header, 4 inside objects.</param>
    /// <param name="origin">
    /// Where the alignment is counted from. Unity aligns relative to the start of the stream
    /// being read, which for an object is the start of its own data (AMD-030 §4.3, §4.4).
    /// </param>
    public void Align(int alignment, int origin = 0)
    {
        int offset = _position - origin;
        int padding = (alignment - (offset % alignment)) % alignment;
        if (padding == 0)
        {
            return;
        }

        if (_position + padding > _bytes.Length)
        {
            // Padding that runs past the end is how a truncated file shows up here; it is the
            // same refusal as a short read.
            throw new UnityAssetFormatException(
                $"the file ends inside the padding that aligns position {_position} on {alignment} bytes.");
        }

        _position += padding;
    }

    private ReadOnlySpan<byte> Take(int count)
    {
        if (count > Remaining)
        {
            throw new UnityAssetFormatException(
                $"the file ends at byte {_bytes.Length}: {count} byte(s) were expected at {_position}, "
                + $"{Remaining} left.");
        }

        ReadOnlySpan<byte> slice = _bytes.Slice(_position, count);
        _position += count;
        return slice;
    }
}
