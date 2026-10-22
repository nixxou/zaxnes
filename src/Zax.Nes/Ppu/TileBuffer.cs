using Zax.Nes.State;

namespace Zax.Nes.Ppu;

/// <summary>
/// The tile records captured during one frame, with the capacity limits of REQ-CORE-032.
/// </summary>
/// <remarks>
/// At most 1 023 background records (33 x 31) and 128 sprite records, 1 151 in all. Records
/// that would exceed either limit are dropped without an error and the emulation carries on
/// (REQ-CORE-032); the reference stops emulating instead, which no embedded profile ever
/// triggers.
/// </remarks>
internal sealed class TileBuffer
{
    /// <summary>Maximum number of background records in one frame (33 x 31, REQ-CORE-032).</summary>
    public const int MaxBackground = 1023;

    /// <summary>Maximum number of sprite records in one frame (REQ-CORE-032).</summary>
    public const int MaxSprites = 128;

    /// <summary>Maximum number of records in one frame.</summary>
    public const int Capacity = MaxBackground + MaxSprites;

    private readonly TileRecord[] _records = new TileRecord[Capacity];

    private int _count;
    private int _backgroundCount;
    private int _spriteCount;

    /// <summary>Records in capture order (REQ-CORE-044).</summary>
    public ReadOnlySpan<TileRecord> Records => _records.AsSpan(0, _count);

    /// <summary>Number of background records dropped by the capacity limit, for the tests.</summary>
    public int DroppedBackground { get; private set; }

    /// <summary>Number of sprite records dropped by the capacity limit, for the tests.</summary>
    public int DroppedSprites { get; private set; }

    /// <summary>Empties the buffer at the start of a rendered frame.</summary>
    public void Clear()
    {
        _count = 0;
        _backgroundCount = 0;
        _spriteCount = 0;
        DroppedBackground = 0;
        DroppedSprites = 0;
    }

    /// <summary>Appends <paramref name="record"/>, dropping it when its kind is full.</summary>
    public void Add(in TileRecord record)
    {
        if (record.Background)
        {
            if (_backgroundCount >= MaxBackground)
            {
                DroppedBackground++;
                return;
            }

            _backgroundCount++;
        }
        else
        {
            if (_spriteCount >= MaxSprites)
            {
                DroppedSprites++;
                return;
            }

            _spriteCount++;
        }

        _records[_count++] = record;
    }

    /// <summary>Appends the buffer to <paramref name="writer"/> (REQ-CORE-090).</summary>
    /// <remarks>
    /// The records are part of the state because the Zapper's light detection reads them
    /// back during the vblank that follows the frame (REQ-CORE-062), so a restore has to
    /// reproduce them exactly (REQ-CORE-092).
    /// </remarks>
    public void SaveState(StateWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);

        writer.WriteInt32(_count);
        writer.WriteInt32(_backgroundCount);
        writer.WriteInt32(_spriteCount);

        for (int i = 0; i < _count; i++)
        {
            TileRecord record = _records[i];
            writer.WriteUInt64(record.Plane0);
            writer.WriteUInt64(record.Plane1);
            writer.WriteUInt16(unchecked((ushort)record.X));
            writer.WriteUInt16(unchecked((ushort)record.Y));
            writer.WriteInt32(record.Index);
            writer.WriteByte(record.Palette);
            writer.WriteByte((byte)(
                (record.MirrorH ? 1 : 0)
                | (record.MirrorV ? 2 : 0)
                | (record.Hidden ? 4 : 0)
                | (record.Background ? 8 : 0)));
        }
    }

    /// <summary>Restores the buffer from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);

        int count = reader.ReadInt32();
        if ((uint)count > Capacity)
        {
            throw new InvalidStateException("Corrupt savestate: impossible tile record count.");
        }

        _count = count;
        _backgroundCount = reader.ReadInt32();
        _spriteCount = reader.ReadInt32();
        DroppedBackground = 0;
        DroppedSprites = 0;

        for (int i = 0; i < count; i++)
        {
            ulong plane0 = reader.ReadUInt64();
            ulong plane1 = reader.ReadUInt64();
            var x = unchecked((short)reader.ReadUInt16());
            var y = unchecked((short)reader.ReadUInt16());
            int index = reader.ReadInt32();
            byte palette = reader.ReadByte();
            byte flags = reader.ReadByte();

            _records[i] = new TileRecord(
                plane0,
                plane1,
                x,
                y,
                index,
                (flags & 1) != 0,
                (flags & 2) != 0,
                (flags & 4) != 0,
                palette,
                (flags & 8) != 0);
        }
    }
}
