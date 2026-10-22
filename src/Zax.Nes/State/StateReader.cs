using System.Buffers.Binary;
using System.Text;

namespace Zax.Nes.State;

/// <summary>
/// Typed cause of a failed "restore state" (REQ-CORE-091, AREQ-CORE-120; AMD-002 answer to
/// Q-001).
/// </summary>
/// <remarks>
/// As for ROM loading, the French phrase « savestate corrompu » of REQ-CORE-091 is a
/// description of the cause: the core returns this code plus a technical message, and the
/// host picks the text it shows — which SPEC-07 REQ-LOOP-046 fixes to the literal
/// <c>Corrupted save state.</c>
/// </remarks>
public enum StateLoadFailure
{
    /// <summary>No failure.</summary>
    None = 0,

    /// <summary>The block is invalid or truncated ("savestate corrompu").</summary>
    CorruptState = 1,

    /// <summary>
    /// The block was written by another core, which REQ-CORE-120 refuses with the literal
    /// message <c>Save state used a different core.</c>
    /// </summary>
    ForeignCore = 2,
}

/// <summary>
/// Thrown when a savestate block is invalid or truncated. "Restore state" catches it and
/// reports the failure to the host (REQ-CORE-091).
/// </summary>
public sealed class InvalidStateException : Exception
{
    /// <summary>Creates the exception with <paramref name="message"/>.</summary>
    public InvalidStateException(string message)
        : this(StateLoadFailure.CorruptState, message)
    {
    }

    /// <summary>Creates the exception with a typed cause and <paramref name="message"/>.</summary>
    public InvalidStateException(StateLoadFailure failure, string message)
        : base(message)
        => Failure = failure;

    /// <summary>Typed cause of the failure (AMD-002, answer to Q-001).</summary>
    public StateLoadFailure Failure { get; }
}

/// <summary>
/// Little-endian reader over a savestate block, counterpart of <see cref="StateWriter"/>.
/// </summary>
/// <remarks>
/// It starts at an arbitrary offset inside a larger array and reports how many bytes it
/// consumed, so that the host can read its own data placed right after the core block
/// (REQ-CORE-091). Every read is bounds-checked and raises
/// <see cref="InvalidStateException"/> rather than corrupting the machine.
/// </remarks>
public sealed class StateReader
{
    private readonly ReadOnlyMemory<byte> _data;
    private readonly int _origin;
    private int _position;

    /// <summary>Creates a reader starting at <paramref name="offset"/> in <paramref name="data"/>.</summary>
    public StateReader(ReadOnlyMemory<byte> data, int offset)
    {
        if (offset < 0 || offset > data.Length)
        {
            throw new InvalidStateException(
                $"Corrupt savestate: start position {offset} is outside the {data.Length}-byte array.");
        }

        _data = data;
        _origin = offset;
        _position = offset;
    }

    /// <summary>Number of bytes consumed since the start position.</summary>
    public int Consumed => _position - _origin;

    /// <summary>Number of bytes left in the underlying array.</summary>
    public int Remaining => _data.Length - _position;

    /// <summary>Reads one byte.</summary>
    public byte ReadByte()
    {
        Require(1);
        return _data.Span[_position++];
    }

    /// <summary>Reads a boolean written by <see cref="StateWriter.WriteBool"/>.</summary>
    public bool ReadBool() => ReadByte() != 0;

    /// <summary>Reads a signed byte.</summary>
    public sbyte ReadSByte() => unchecked((sbyte)ReadByte());

    /// <summary>Reads a 16-bit unsigned integer.</summary>
    public ushort ReadUInt16()
    {
        Require(2);
        ushort value = BinaryPrimitives.ReadUInt16LittleEndian(_data.Span[_position..]);
        _position += 2;
        return value;
    }

    /// <summary>Reads a 32-bit signed integer.</summary>
    public int ReadInt32()
    {
        Require(4);
        int value = BinaryPrimitives.ReadInt32LittleEndian(_data.Span[_position..]);
        _position += 4;
        return value;
    }

    /// <summary>Reads a 64-bit signed integer.</summary>
    public long ReadInt64()
    {
        Require(8);
        long value = BinaryPrimitives.ReadInt64LittleEndian(_data.Span[_position..]);
        _position += 8;
        return value;
    }

    /// <summary>Reads a 64-bit unsigned integer.</summary>
    public ulong ReadUInt64()
    {
        Require(8);
        ulong value = BinaryPrimitives.ReadUInt64LittleEndian(_data.Span[_position..]);
        _position += 8;
        return value;
    }

    /// <summary>Reads a double.</summary>
    public double ReadDouble() => BitConverter.UInt64BitsToDouble(ReadUInt64());

    /// <summary>Reads <paramref name="count"/> raw bytes.</summary>
    public ReadOnlySpan<byte> ReadRaw(int count)
    {
        if (count < 0)
        {
            throw new InvalidStateException($"Corrupt savestate: negative byte count {count}.");
        }

        Require(count);
        ReadOnlySpan<byte> value = _data.Span.Slice(_position, count);
        _position += count;
        return value;
    }

    /// <summary>Reads exactly <paramref name="destination"/>.Length bytes into it.</summary>
    public void ReadInto(Span<byte> destination)
    {
        Require(destination.Length);
        _data.Span.Slice(_position, destination.Length).CopyTo(destination);
        _position += destination.Length;
    }

    /// <summary>
    /// Reads a length-prefixed blob into <paramref name="destination"/>, which must have
    /// exactly the stored length.
    /// </summary>
    public void ReadBlobInto(Span<byte> destination)
    {
        int count = ReadInt32();
        if (count != destination.Length)
        {
            throw new InvalidStateException(
                $"Corrupt savestate: a block of {destination.Length} bytes was expected, {count} were stored.");
        }

        ReadInto(destination);
    }

    /// <summary>Reads a length-prefixed blob as a new array.</summary>
    public byte[] ReadBlob()
    {
        int count = ReadInt32();
        return ReadRaw(count).ToArray();
    }

    /// <summary>Reads a string written by <see cref="StateWriter.WriteString"/>.</summary>
    public string ReadString()
    {
        int count = ReadInt32();
        return Encoding.UTF8.GetString(ReadRaw(count));
    }

    private void Require(int count)
    {
        if (count > Remaining)
        {
            throw new InvalidStateException(
                $"Corrupt savestate: {count} bytes were needed at offset {_position}, only {Remaining} are available.");
        }
    }
}
