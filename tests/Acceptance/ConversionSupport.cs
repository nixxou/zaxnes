using Zax.Engine3D;
using Zax.Nes;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Builders for the synthetic frames of SPEC-04 §5: named full images A, B, C …, background
/// rows and sprites placed by the verifier.
/// </summary>
public sealed class SyntheticBatch
{
    private readonly List<TileRecord> _records = [];
    private readonly byte[] _palette = new byte[32];

    /// <summary>Unreliable left/right band width.</summary>
    public int MaskX { get; set; }

    /// <summary>Unreliable top/bottom band width.</summary>
    public int MaskY { get; set; }

    /// <summary>Palette byte <paramref name="entry"/>.</summary>
    public SyntheticBatch Palette(int entry, byte value)
    {
        _palette[entry] = value;
        return this;
    }

    /// <summary>
    /// A distinct full image: 64 pixels of colour 1, identified by <paramref name="name"/>.
    /// Images differ by their low plane, so each letter is its own image.
    /// </summary>
    public static (ulong Low, ulong High) Full(char name)
        => (0xFFFFFFFFFFFFFFFFUL, 0x0101010101010101UL * (ulong)(name - 'A'));

    /// <summary>An image with a single non-transparent pixel at (px, py), colour 1.</summary>
    public static (ulong Low, ulong High) SinglePixel(int px, int py)
        => (1UL << ((8 * py) + 7 - px), 0UL);

    /// <summary>Adds one background tile at a screen position.</summary>
    public SyntheticBatch Background(char image, int x, int y, int palette = 0)
    {
        (ulong low, ulong high) = Full(image);
        _records.Add(new TileRecord(low, high, (short)x, (short)y, _records.Count, false, false, false, (byte)palette, true));
        return this;
    }

    /// <summary>Adds one background tile from explicit planes.</summary>
    public SyntheticBatch BackgroundPlanes(ulong low, ulong high, int x, int y, int palette = 0)
    {
        _records.Add(new TileRecord(low, high, (short)x, (short)y, _records.Count, false, false, false, (byte)palette, true));
        return this;
    }

    /// <summary>Adds a row of background tiles, one per character of <paramref name="row"/>.</summary>
    public SyntheticBatch Row(string row, int column, int line, int palette = 0)
    {
        for (int i = 0; i < row.Length; i++)
        {
            if (row[i] != ' ')
            {
                Background(row[i], 8 * (column + i), 8 * line, palette);
            }
        }

        return this;
    }

    /// <summary>Adds one sprite tile.</summary>
    public SyntheticBatch Sprite(
        char image, int x, int y, int palette = 0, bool hidden = false, bool flipX = false, bool flipY = false)
    {
        (ulong low, ulong high) = Full(image);
        _records.Add(new TileRecord(low, high, (short)x, (short)y, _records.Count, flipX, flipY, hidden, (byte)palette, false));
        return this;
    }

    /// <summary>Adds one sprite tile from explicit planes.</summary>
    public SyntheticBatch SpritePlanes(
        ulong low, ulong high, int x, int y, int palette = 0, bool flipX = false, bool flipY = false)
    {
        _records.Add(new TileRecord(low, high, (short)x, (short)y, _records.Count, flipX, flipY, false, (byte)palette, false));
        return this;
    }

    /// <summary>Runs the batch through an engine and returns the converted frame.</summary>
    public ConversionFrame Convert(ConversionEngine engine, int counter = 1)
    {
        TileRecord[] records = [.. _records];
        FrameBatch batch = new(records, _palette, MaskX, MaskY);
        return engine.Convert(batch, counter);
    }
}

/// <summary>Helpers that build engine patterns the way SPEC-04 §5 describes them.</summary>
public static class SyntheticPatterns
{
    /// <summary>The image of a letter, interned in <paramref name="images"/>.</summary>
    public static TileImage Image(ImageRegistry images, char name)
    {
        (ulong low, ulong high) = SyntheticBatch.Full(name);
        return images.Intern(low, high, false, false);
    }

    /// <summary>
    /// Builds a pattern from rows of letters, a space meaning an empty cell, and registers
    /// it with one default instance.
    /// </summary>
    public static EnginePattern Build(
        ConversionEngine engine,
        string[] rows,
        bool selfConnected = true,
        bool skipGeometry = false,
        Cell? constraint = null,
        bool permanent = true,
        string tags = "")
    {
        int height = rows.Length;
        int width = rows.Max(r => r.Length);
        List<TileImage?> cells = [];
        for (int j = 0; j < height; j++)
        {
            for (int i = 0; i < width; i++)
            {
                char c = i < rows[j].Length ? rows[j][i] : ' ';
                cells.Add(c == ' ' ? null : Image(engine.Images, c));
            }
        }

        EnginePattern pattern = new(width, height, cells, permanent);
        pattern.SetSelfConnected(selfConnected);
        pattern.SkipGeometry = skipGeometry;
        pattern.ConstraintSize = constraint ?? new Cell(99, 99);
        pattern.AddInstance(new PatternInstance
        {
            Depth = 3,
            Layer = ZLayer.L1,
            Scale = new Vec3(1, 1, 1),
            Pivot = new Vec3(0.5f, 0.5f, 0.5f),
            DeformPivot = new Vec3(0.5f, 0.5f, 0.5f),
            Alpha = 1,
            Style = RenderStyle.Normal,
            CustomPalette = -1,
            TexTilting = new Vec2(1, 1),
            FillRate = 1,
            TagString = tags,
        });

        engine.Patterns.Register(pattern);
        return pattern;
    }
}
