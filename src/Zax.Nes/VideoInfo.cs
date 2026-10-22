namespace Zax.Nes;

/// <summary>
/// Dimensions of the tile grid the 3D engine allocates (REQ-CORE-031, SPEC-02 §4.3).
/// </summary>
/// <param name="Width">Background tile columns: 33 (32 plus one, because of fine X scroll).</param>
/// <param name="Height">Background tile rows: 31 (30 plus one).</param>
/// <param name="SpriteCount">Sprite record slots: 128 (64 sprites, two records each in 8x16 mode).</param>
public readonly record struct VideoInfo(int Width, int Height, int SpriteCount)
{
    /// <summary>The one and only value the core reports: (33, 31, 128).</summary>
    public static VideoInfo Standard => new(33, 31, 128);

    /// <summary>Maximum background records in a frame batch: 33 x 31 (REQ-CORE-032).</summary>
    public const int MaxBackgroundRecords = 1023;

    /// <summary>Maximum sprite records in a frame batch (REQ-CORE-032).</summary>
    public const int MaxSpriteRecords = 128;

    /// <summary>Maximum records of any kind in a frame batch (REQ-CORE-032).</summary>
    public const int MaxRecords = MaxBackgroundRecords + MaxSpriteRecords;
}
