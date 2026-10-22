namespace Zax.Nes.Tests;

/// <summary>
/// A copy of one <see cref="FrameBatch"/>, taken inside the video callback.
/// </summary>
/// <remarks>
/// <see cref="FrameBatch"/> is a <c>ref struct</c> over borrowed storage that is only valid
/// during the callback (REQ-CORE-030), so a test that wants to assert on it has to copy it
/// first — which is exactly the contract the host has to honour.
/// </remarks>
internal sealed class BatchCopy
{
    /// <summary>Records in capture order.</summary>
    public TileRecord[] Records { get; init; } = [];

    /// <summary>The 32 palette bytes.</summary>
    public byte[] Palette { get; init; } = [];

    /// <summary>maskX of the batch.</summary>
    public int MaskX { get; init; }

    /// <summary>maskY of the batch.</summary>
    public int MaskY { get; init; }

    /// <summary>The background records, in capture order.</summary>
    public TileRecord[] Background => [.. Records.Where(static record => record.Background)];

    /// <summary>The sprite records, in capture order.</summary>
    public TileRecord[] Sprites => [.. Records.Where(static record => !record.Background)];
}

/// <summary>
/// Builds the state of the test vectors of SPEC-02 §5.3 on the minimal ROM of §5.1.
/// </summary>
/// <remarks>
/// The recipe is the one the spec prescribes: three frames to pass the PPU warm-up, then the
/// wanted state programmed through <em>memory writes</em> to $2000-$2007 (REQ-CORE-082),
/// then one frame — the internal scroll register is reloaded from <c>t</c> on the pre-render
/// line — and the batch of the frame <em>after</em> that is the one to look at.
/// </remarks>
internal sealed class TileFixture
{
    private readonly NesCore _core;

    /// <summary>
    /// Loads the minimal ROM and passes the PPU warm-up.
    /// </summary>
    /// <param name="headerByte6">
    /// Byte 6 of the iNES header: <c>0x01</c> vertical mirroring (the default of §5.3),
    /// <c>0x00</c> horizontal, <c>0x08</c> four-screen.
    /// </param>
    /// <param name="program">Code to place at $8000 instead of the infinite loop.</param>
    public TileFixture(byte headerByte6 = 0x01, byte[]? program = null)
    {
        _core = new NesCore();
        Rom.RomLoadResult load = _core.LoadRom(TestRoms.Minimal(headerByte6, program), "Vector.nes");
        Assert.True(load.Success, load.Error);

        // Three frames, to get past the PPU warm-up of about 29 658 CPU cycles (§5.1).
        for (int i = 0; i < 3; i++)
        {
            _core.RunFrame(0);
        }

        FillOam(0xFF);
    }

    /// <summary>The core under test.</summary>
    public NesCore Core => _core;

    /// <summary>Fills all 256 OAM bytes with <paramref name="value"/> ($FF in §5.3).</summary>
    public void FillOam(byte value)
    {
        _core.WriteMemory(0x2003, 0x00);
        for (int i = 0; i < Ppu.Ppu2C02.OamSize; i++)
        {
            _core.WriteMemory(0x2004, value);
        }
    }

    /// <summary>Writes the four OAM bytes of sprite <paramref name="index"/>.</summary>
    public void SetSprite(int index, byte y, byte tile, byte attribute, byte x)
    {
        _core.WriteMemory(0x2003, (byte)(index * 4));
        _core.WriteMemory(0x2004, y);
        _core.WriteMemory(0x2004, tile);
        _core.WriteMemory(0x2004, attribute);
        _core.WriteMemory(0x2004, x);
    }

    /// <summary>
    /// Writes <paramref name="values"/> to PPU memory from <paramref name="address"/>, as a
    /// game would: two writes to $2006 then one write to $2007 per byte.
    /// </summary>
    public void WriteVram(ushort address, params byte[] values)
    {
        _core.WriteMemory(0x2006, (byte)(address >> 8));
        _core.WriteMemory(0x2006, (byte)address);
        foreach (byte value in values)
        {
            _core.WriteMemory(0x2007, value);
        }
    }

    /// <summary>Fills <paramref name="count"/> PPU bytes with <paramref name="value"/>.</summary>
    public void FillVram(ushort address, int count, byte value)
    {
        _core.WriteMemory(0x2006, (byte)(address >> 8));
        _core.WriteMemory(0x2006, (byte)address);
        for (int i = 0; i < count; i++)
        {
            _core.WriteMemory(0x2007, value);
        }
    }

    /// <summary>
    /// Sets the scroll with two writes to $2005, the write toggle being even since the last
    /// $2006 write (§5.1).
    /// </summary>
    public void SetScroll(byte x, byte y)
    {
        _core.WriteMemory(0x2005, x);
        _core.WriteMemory(0x2005, y);
    }

    /// <summary>Writes PPUCTRL.</summary>
    public void SetControl(byte value) => _core.WriteMemory(0x2000, value);

    /// <summary>Writes PPUMASK.</summary>
    public void SetMask(byte value) => _core.WriteMemory(0x2001, value);

    /// <summary>
    /// Runs one frame, then returns the batch of the next one, which is the first frame that
    /// fully reflects the programmed state (§5.1).
    /// </summary>
    public BatchCopy NextBatch()
    {
        _core.RunFrame(0);
        return CaptureBatch(_core);
    }

    /// <summary>Runs one frame and returns a copy of its batch.</summary>
    public static BatchCopy CaptureBatch(NesCore core, ushort input = 0)
    {
        BatchCopy? copy = null;
        VideoCallback? previous = core.VideoCallback;

        core.VideoCallback = (in FrameBatch batch) => copy = new BatchCopy
        {
            Records = batch.Records.ToArray(),
            Palette = batch.Palette.ToArray(),
            MaskX = batch.MaskX,
            MaskY = batch.MaskY,
        };

        core.RunFrame(input);
        core.VideoCallback = previous;

        Assert.NotNull(copy);
        return copy;
    }
}
