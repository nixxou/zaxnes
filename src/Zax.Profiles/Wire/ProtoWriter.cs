using System.Text;

namespace Zax.Profiles.Wire;

/// <summary>
/// Minimal writer of the standard protobuf binary encoding, following the canonical writer of
/// SPEC-03 REQ-PROF-051: minimal varints, no packing of repeated fields, negative integers
/// sign-extended to 64 bits, floats copied bit for bit.
/// </summary>
/// <remarks>
/// Nested messages are written in place: the body goes straight into the buffer and its length
/// prefix is inserted afterwards, which shifts the body by one or two bytes. That keeps a
/// single pass and a single buffer, and the shift cost stays proportional to the nesting depth
/// of the format (six levels).
/// </remarks>
internal sealed class ProtoWriter
{
    private byte[] _buffer;
    private int _length;

    internal ProtoWriter(int initialCapacity = 1024)
    {
        _buffer = new byte[Math.Max(16, initialCapacity)];
    }

    /// <summary>Number of bytes written so far.</summary>
    internal int Length => _length;

    /// <summary>Writes a field tag.</summary>
    internal void WriteTag(int fieldNumber, WireType wireType)
        => WriteVarint(((ulong)fieldNumber << 3) | (uint)wireType);

    /// <summary>Writes a minimal varint.</summary>
    internal void WriteVarint(ulong value)
    {
        Ensure(10);
        while (value >= 0x80UL)
        {
            _buffer[_length++] = (byte)(value | 0x80UL);
            value >>= 7;
        }

        _buffer[_length++] = (byte)value;
    }

    /// <summary>Writes a 32-bit integer field, sign-extended to 64 bits when negative.</summary>
    internal void WriteInt32(int fieldNumber, int value)
    {
        WriteTag(fieldNumber, WireType.Varint);
        WriteVarint((ulong)(long)value);
    }

    /// <summary>Writes a 64-bit integer field.</summary>
    internal void WriteInt64(int fieldNumber, long value)
    {
        WriteTag(fieldNumber, WireType.Varint);
        WriteVarint((ulong)value);
    }

    /// <summary>Writes a boolean field as 0 or 1.</summary>
    internal void WriteBool(int fieldNumber, bool value)
    {
        WriteTag(fieldNumber, WireType.Varint);
        WriteVarint(value ? 1UL : 0UL);
    }

    /// <summary>Writes a float field, bits preserved.</summary>
    internal void WriteSingle(int fieldNumber, float value)
    {
        WriteTag(fieldNumber, WireType.Fixed32);
        int bits = BitConverter.SingleToInt32Bits(value);
        Ensure(4);
        _buffer[_length++] = (byte)bits;
        _buffer[_length++] = (byte)(bits >> 8);
        _buffer[_length++] = (byte)(bits >> 16);
        _buffer[_length++] = (byte)(bits >> 24);
    }

    /// <summary>Writes a UTF-8 string field, empty strings included.</summary>
    internal void WriteString(int fieldNumber, string value)
    {
        WriteTag(fieldNumber, WireType.Length);
        int byteCount = Encoding.UTF8.GetByteCount(value);
        WriteVarint((ulong)byteCount);
        Ensure(byteCount);
        Encoding.UTF8.GetBytes(value, _buffer.AsSpan(_length, byteCount));
        _length += byteCount;
    }

    /// <summary>
    /// Writes the tag of a nested message and returns the token to pass to
    /// <see cref="EndNested"/> once its body has been written.
    /// </summary>
    internal int BeginNested(int fieldNumber)
    {
        WriteTag(fieldNumber, WireType.Length);
        return _length;
    }

    /// <summary>Inserts the length prefix of the nested message opened at <paramref name="start"/>.</summary>
    internal void EndNested(int start)
    {
        int bodyLength = _length - start;
        int prefixSize = VarintSize((ulong)bodyLength);
        Ensure(prefixSize);
        Array.Copy(_buffer, start, _buffer, start + prefixSize, bodyLength);
        _length += prefixSize;

        int position = start;
        ulong value = (ulong)bodyLength;
        while (value >= 0x80UL)
        {
            _buffer[position++] = (byte)(value | 0x80UL);
            value >>= 7;
        }

        _buffer[position] = (byte)value;
    }

    /// <summary>Writes a start-group tag.</summary>
    internal void WriteGroupStart(int fieldNumber) => WriteTag(fieldNumber, WireType.StartGroup);

    /// <summary>Writes an end-group tag.</summary>
    internal void WriteGroupEnd(int fieldNumber) => WriteTag(fieldNumber, WireType.EndGroup);

    /// <summary>Returns the bytes written.</summary>
    internal byte[] ToArray() => _buffer.AsSpan(0, _length).ToArray();

    private static int VarintSize(ulong value)
    {
        int size = 1;
        while (value >= 0x80UL)
        {
            value >>= 7;
            size++;
        }

        return size;
    }

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
