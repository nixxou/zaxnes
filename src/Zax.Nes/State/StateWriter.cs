using System.Buffers.Binary;
using System.Text;

namespace Zax.Nes.State;

/// <summary>
/// Append-only little-endian writer used to build a savestate block (REQ-CORE-090).
/// </summary>
/// <remarks>
/// The encoding is deliberately plain and fixed-width so that a block is reproducible
/// byte for byte from the same machine state (REQ-ARCH-012).
/// </remarks>
public sealed class StateWriter
{
    private byte[] _buffer;
    private int _length;

    /// <summary>Creates a writer with an initial capacity.</summary>
    public StateWriter(int capacity = 4096)
    {
        ArgumentOutOfRangeException.ThrowIfNegative(capacity);
        _buffer = new byte[Math.Max(capacity, 16)];
    }

    /// <summary>Number of bytes written so far.</summary>
    public int Length => _length;

    /// <summary>Writes one byte.</summary>
    public void WriteByte(byte value)
    {
        Ensure(1);
        _buffer[_length++] = value;
    }

    /// <summary>Writes a boolean as one byte, 0 or 1.</summary>
    public void WriteBool(bool value) => WriteByte(value ? (byte)1 : (byte)0);

    /// <summary>Writes a signed byte.</summary>
    public void WriteSByte(sbyte value) => WriteByte(unchecked((byte)value));

    /// <summary>Writes a 16-bit unsigned integer.</summary>
    public void WriteUInt16(ushort value)
    {
        Ensure(2);
        BinaryPrimitives.WriteUInt16LittleEndian(_buffer.AsSpan(_length), value);
        _length += 2;
    }

    /// <summary>Writes a 32-bit signed integer.</summary>
    public void WriteInt32(int value)
    {
        Ensure(4);
        BinaryPrimitives.WriteInt32LittleEndian(_buffer.AsSpan(_length), value);
        _length += 4;
    }

    /// <summary>Writes a 64-bit signed integer.</summary>
    public void WriteInt64(long value)
    {
        Ensure(8);
        BinaryPrimitives.WriteInt64LittleEndian(_buffer.AsSpan(_length), value);
        _length += 8;
    }

    /// <summary>Writes a 64-bit unsigned integer.</summary>
    public void WriteUInt64(ulong value)
    {
        Ensure(8);
        BinaryPrimitives.WriteUInt64LittleEndian(_buffer.AsSpan(_length), value);
        _length += 8;
    }

    /// <summary>Writes a double.</summary>
    public void WriteDouble(double value) => WriteUInt64(BitConverter.DoubleToUInt64Bits(value));

    /// <summary>Writes raw bytes, without a length prefix.</summary>
    public void WriteRaw(ReadOnlySpan<byte> value)
    {
        Ensure(value.Length);
        value.CopyTo(_buffer.AsSpan(_length));
        _length += value.Length;
    }

    /// <summary>Writes a 32-bit length followed by the bytes.</summary>
    public void WriteBlob(ReadOnlySpan<byte> value)
    {
        WriteInt32(value.Length);
        WriteRaw(value);
    }

    /// <summary>Writes a 32-bit byte count followed by the UTF-8 bytes of <paramref name="value"/>.</summary>
    public void WriteString(string value)
    {
        int count = Encoding.UTF8.GetByteCount(value);
        WriteInt32(count);
        Ensure(count);
        Encoding.UTF8.GetBytes(value, _buffer.AsSpan(_length));
        _length += count;
    }

    /// <summary>Returns the bytes written so far as a new array.</summary>
    public byte[] ToArray() => _buffer.AsSpan(0, _length).ToArray();

    /// <summary>Returns the bytes written so far without copying.</summary>
    public ReadOnlySpan<byte> AsSpan() => _buffer.AsSpan(0, _length);

    private void Ensure(int extra)
    {
        if (_length + extra <= _buffer.Length)
        {
            return;
        }

        int capacity = _buffer.Length;
        while (capacity < _length + extra)
        {
            capacity *= 2;
        }

        Array.Resize(ref _buffer, capacity);
    }
}
