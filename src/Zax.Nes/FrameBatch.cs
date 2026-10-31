namespace Zax.Nes;

/// <summary>
/// The ordered tile records of one emulated frame, together with the frame palette, the
/// unreliable border widths, the pixel image and its opacity mask, handed to the video
/// callback (REQ-CORE-030, AMD-050).
/// </summary>
/// <remarks>
/// The spans are only valid for the duration of the callback: a host that keeps the data
/// must copy it (REQ-CORE-030).
/// </remarks>
public readonly ref struct FrameBatch
{
    /// <summary>
    /// Creates a frame batch that carries no pixel image, for a synthetic frame that was not
    /// produced by the PPU.
    /// </summary>
    public FrameBatch(ReadOnlySpan<TileRecord> records, ReadOnlySpan<byte> palette, int maskX, int maskY)
        : this(records, palette, maskX, maskY, default, default)
    {
    }

    /// <summary>Creates a frame batch over borrowed storage.</summary>
    public FrameBatch(
        ReadOnlySpan<TileRecord> records,
        ReadOnlySpan<byte> palette,
        int maskX,
        int maskY,
        ReadOnlySpan<byte> picture,
        ReadOnlySpan<byte> opacityMask)
    {
        Records = records;
        Palette = palette;
        MaskX = maskX;
        MaskY = maskY;
        Picture = picture;
        OpacityMask = opacityMask;
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

    /// <summary>
    /// The 256 × 240 pixel image of the frame, one NES colour index per pixel, row 0 first
    /// (point 4 of REQ-CORE-030, REQ-CORE-048 as AMD-050 amends it). Empty when the frame
    /// produced no image (<see cref="FrameOutput.Pixels"/> clear).
    /// </summary>
    public ReadOnlySpan<byte> Picture { get; }

    /// <summary>
    /// The 256 × 240 opacity mask of the same frame, one bit per pixel (point 5 of
    /// REQ-CORE-030, REQ-CORE-049): 32 bytes per image row, rows top to bottom, the least
    /// significant bit of a byte being its leftmost pixel. Empty when
    /// <see cref="Picture"/> is.
    /// </summary>
    public ReadOnlySpan<byte> OpacityMask { get; }
}

/// <summary>
/// Receives one <see cref="FrameBatch"/> per emulated frame, synchronously, at the end of
/// "run one frame" (REQ-CORE-020). It is called even when the batch is empty
/// (REQ-CORE-021).
/// </summary>
public delegate void VideoCallback(in FrameBatch batch);
