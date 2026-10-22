using System.Text;

namespace Zax.Profiles.Wire;

/// <summary>Protobuf wire types (SPEC-03 §2, notation).</summary>
internal enum WireType
{
    Varint = 0,
    Fixed64 = 1,
    Length = 2,
    StartGroup = 3,
    EndGroup = 4,
    Fixed32 = 5,
}

/// <summary>
/// Minimal reader of the standard protobuf binary encoding, with the scalar conventions of
/// SPEC-03 REQ-PROF-002: varints are never zig-zagged, a negative integer is sign-extended to
/// 64 bits, and the bits of a float are taken as they are.
/// </summary>
/// <remarks>
/// Every failure is a <see cref="ProfileFormatException"/>, so that a corrupted file can be
/// refused as a whole without crashing (REQ-PROF-033, REQ-ACC-041).
/// </remarks>
internal sealed class ProtoReader
{
    private readonly byte[] _data;
    private int _position;

    internal ProtoReader(byte[] data)
    {
        _data = data;
    }

    /// <summary>Current offset.</summary>
    internal int Position => _position;

    /// <summary>Offset one past the last byte.</summary>
    internal int End => _data.Length;

    /// <summary>
    /// Reads the next tag while the position is below <paramref name="limit"/>. Returns false
    /// at the limit, which is the end of the enclosing message.
    /// </summary>
    internal bool TryReadTag(int limit, out int fieldNumber, out WireType wireType)
    {
        if (_position >= limit)
        {
            fieldNumber = 0;
            wireType = WireType.Varint;
            return false;
        }

        ulong tag = ReadVarint();
        fieldNumber = (int)(tag >> 3);
        wireType = (WireType)(int)(tag & 7);
        if (fieldNumber <= 0)
        {
            throw new ProfileFormatException($"Field number {fieldNumber} is not valid at offset {_position}.");
        }

        return true;
    }

    /// <summary>Reads a varint of at most ten bytes.</summary>
    internal ulong ReadVarint()
    {
        ulong result = 0;
        int shift = 0;
        for (int i = 0; i < 10; i++)
        {
            if (_position >= _data.Length)
            {
                throw new ProfileFormatException("The file ends in the middle of a varint.");
            }

            byte current = _data[_position++];
            result |= (ulong)(current & 0x7F) << shift;
            if ((current & 0x80) == 0)
            {
                return result;
            }

            shift += 7;
        }

        throw new ProfileFormatException($"A varint at offset {_position} is longer than ten bytes.");
    }

    /// <summary>Reads a 64-bit two's-complement integer.</summary>
    internal long ReadInt64() => (long)ReadVarint();

    /// <summary>Reads a 32-bit integer, stored sign-extended on 64 bits.</summary>
    internal int ReadInt32() => (int)(long)ReadVarint();

    /// <summary>Reads a boolean: zero is false, anything else true.</summary>
    internal bool ReadBool() => ReadVarint() != 0;

    /// <summary>Reads a little-endian IEEE-754 single, bits preserved.</summary>
    internal float ReadSingle()
    {
        Require(4);
        int bits = _data[_position]
            | (_data[_position + 1] << 8)
            | (_data[_position + 2] << 16)
            | (_data[_position + 3] << 24);
        _position += 4;
        return BitConverter.Int32BitsToSingle(bits);
    }

    /// <summary>Reads a length prefix and returns the offset of the end of the payload.</summary>
    internal int ReadLengthLimit()
    {
        ulong length = ReadVarint();
        if (length > (ulong)(_data.Length - _position))
        {
            throw new ProfileFormatException(
                $"A length-delimited field at offset {_position} claims {length} bytes, past the end of the file.");
        }

        return _position + (int)length;
    }

    /// <summary>Reads a UTF-8 string with its length prefix.</summary>
    internal string ReadString()
    {
        int limit = ReadLengthLimit();
        string value = Encoding.UTF8.GetString(_data, _position, limit - _position);
        _position = limit;
        return value;
    }

    /// <summary>Skips a field of the given wire type, groups included.</summary>
    internal void Skip(int fieldNumber, WireType wireType)
    {
        switch (wireType)
        {
            case WireType.Varint:
                ReadVarint();
                break;
            case WireType.Fixed64:
                Require(8);
                _position += 8;
                break;
            case WireType.Length:
                _position = ReadLengthLimit();
                break;
            case WireType.Fixed32:
                Require(4);
                _position += 4;
                break;
            case WireType.StartGroup:
                SkipGroup(fieldNumber);
                break;
            default:
                throw new ProfileFormatException(
                    $"Wire type {(int)wireType} of field {fieldNumber} at offset {_position} is not valid here.");
        }
    }

    /// <summary>
    /// Consumes the end-group tag of <paramref name="fieldNumber"/>, which must be the next tag.
    /// </summary>
    internal void ExpectGroupEnd(int fieldNumber)
    {
        ulong tag = ReadVarint();
        if ((int)(tag >> 3) != fieldNumber || (WireType)(int)(tag & 7) != WireType.EndGroup)
        {
            throw new ProfileFormatException(
                $"Expected the end of group {fieldNumber} at offset {_position}.");
        }
    }

    /// <summary>
    /// Reads the next tag inside a group, or returns false once the matching end-group tag has
    /// been consumed.
    /// </summary>
    internal bool TryReadGroupTag(int groupField, out int fieldNumber, out WireType wireType)
    {
        ulong tag = ReadVarint();
        fieldNumber = (int)(tag >> 3);
        wireType = (WireType)(int)(tag & 7);
        if (wireType == WireType.EndGroup)
        {
            if (fieldNumber != groupField)
            {
                throw new ProfileFormatException(
                    $"Group {groupField} ends with the tag of group {fieldNumber} at offset {_position}.");
            }

            return false;
        }

        if (fieldNumber <= 0)
        {
            throw new ProfileFormatException($"Field number {fieldNumber} is not valid at offset {_position}.");
        }

        return true;
    }

    private void SkipGroup(int fieldNumber)
    {
        while (TryReadGroupTag(fieldNumber, out int inner, out WireType type))
        {
            Skip(inner, type);
        }
    }

    private void Require(int count)
    {
        if (_data.Length - _position < count)
        {
            throw new ProfileFormatException($"The file ends {count} bytes too early at offset {_position}.");
        }
    }
}
