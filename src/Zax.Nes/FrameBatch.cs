namespace Zax.Nes;

/// <summary>
/// The ordered tile records of one emulated frame, together with the frame palette and
/// the unreliable border widths, handed to the video callback (REQ-CORE-030).
/// </summary>
/// <remarks>
/// The spans are only valid for the duration of the callback: a host that keeps the data
/// must copy it (REQ-CORE-030).
/// </remarks>
public readonly ref struct FrameBatch
{
    /// <summary>Creates a frame batch over borrowed storage.</summary>
    public FrameBatch(ReadOnlySpan<TileRecord> records, ReadOnlySpan<byte> palette, int maskX, int maskY)
    {
        Records = records;
        Palette = palette;
        MaskX = maskX;
        MaskY = maskY;
    }

    /// <summary>
    /// Tile records in capture order (REQ-CORE-044): chronological by capture line, and
    /// for one line the sprites before the background row.
    /// </summary>
    public ReadOnlySpan<TileRecord> Records { get; }

    /// <summary>
    /// The 32 bytes of PPU palette RAM ($3F00-$3F1F) at the end of the frame
    /// (REQ-CORE-045). Each value is a NES colour index, 0 to 63.
    /// </summary>
    public ReadOnlySpan<byte> Palette { get; }

    /// <summary>
    /// Width in NES pixels of the unreliable bands on the left <em>and</em> right edges
    /// (REQ-CORE-046).
    /// </summary>
    public int MaskX { get; }

    /// <summary>
    /// Width in NES pixels of the unreliable bands on the top <em>and</em> bottom edges
    /// (REQ-CORE-046).
    /// </summary>
    public int MaskY { get; }
}

/// <summary>
/// Receives one <see cref="FrameBatch"/> per emulated frame, synchronously, at the end of
/// "run one frame" (REQ-CORE-020). It is called even when the batch is empty
/// (REQ-CORE-021).
/// </summary>
public delegate void VideoCallback(in FrameBatch batch);
