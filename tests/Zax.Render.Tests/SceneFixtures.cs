using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// Builds the synthetic frames the render tests assert on: a profile, a batch of tile records and
/// the resulting scene state, all without a ROM.
/// </summary>
/// <remarks>
/// The same recipes the <c>Zax.Harness render</c> captures use; they are rebuilt here so that the
/// test project depends only on the libraries under test.
/// </remarks>
internal sealed class SceneFixture
{
    private readonly GameProfile _profile = GameProfile.CreateEmpty();
    private readonly List<TileRecord> _records = [];
    private readonly byte[] _palette = new byte[32];
    private int _spriteOrder;

    /// <summary>Sets the universal background colour, entry 0 of the palette.</summary>
    public SceneFixture Background(byte nesColor)
    {
        _palette[0] = nesColor;
        return this;
    }

    /// <summary>Sets one of the 32 PPU palette bytes.</summary>
    public SceneFixture Palette(int entry, byte nesColor)
    {
        if (entry is > 0 and < 32)
        {
            _palette[entry] = nesColor;
        }

        return this;
    }

    /// <summary>Adds a one-cell pattern carrying <paramref name="tile"/>.</summary>
    public SceneFixture Pattern(
        (ulong Low, ulong High) tile,
        int depth,
        ProfileLayer layer,
        float alpha = 1f,
        VariantRenderStyle style = VariantRenderStyle.Normal,
        bool fullWarp = false)
    {
        ProfilePattern pattern = new()
        {
            Grid = new PatternGrid(1, 1, [new ProfileTile(tile.Low, tile.High, TileFlip.None)]),
            TileSize = new Vec2I(1, 1),
            Permanent = true,
            SelfConnected = true,
            ConstraintSize = Vec2I.Unconstrained,
        };
        pattern.Variants.Add(new PatternVariant
        {
            Depth = depth,
            Layer = layer,
            Alpha = alpha,
            RenderStyle = style,
            FullWarp = fullWarp,
        });
        _profile.PermanentPatterns.Add(pattern);
        return this;
    }

    /// <summary>Adds a one-row pattern carrying <paramref name="cells"/> left to right.</summary>
    public SceneFixture PatternRow(
        int depth,
        ProfileLayer layer,
        float alpha,
        params (ulong Low, ulong High)[] cells)
    {
        ArgumentNullException.ThrowIfNull(cells);
        ProfilePattern pattern = new()
        {
            Grid = new PatternGrid(
                cells.Length,
                1,
                [.. cells.Select(cell => new ProfileTile(cell.Low, cell.High, TileFlip.None))]),
            TileSize = new Vec2I(cells.Length, 1),
            Permanent = true,
            SelfConnected = true,
            ConstraintSize = Vec2I.Unconstrained,
        };
        pattern.Variants.Add(new PatternVariant
        {
            Depth = depth,
            Layer = layer,
            Alpha = alpha,
            RenderStyle = VariantRenderStyle.Normal,
        });
        _profile.PermanentPatterns.Add(pattern);
        return this;
    }

    /// <summary>
    /// Adds a <paramref name="columns"/> × <paramref name="rows"/> pattern whose cells all carry
    /// <paramref name="tile"/>: one shape instance of several tiles, the "object" of AMD-012.
    /// </summary>
    public SceneFixture PatternBlock(
        (ulong Low, ulong High) tile,
        int columns,
        int rows,
        int depth,
        ProfileLayer layer,
        float alpha = 1f)
    {
        ProfileTile[] cells = [.. Enumerable
            .Range(0, columns * rows)
            .Select(_ => new ProfileTile(tile.Low, tile.High, TileFlip.None))];

        ProfilePattern pattern = new()
        {
            Grid = new PatternGrid(columns, rows, cells),
            TileSize = new Vec2I(columns, rows),
            Permanent = true,
            SelfConnected = true,
            ConstraintSize = Vec2I.Unconstrained,
        };
        pattern.Variants.Add(new PatternVariant
        {
            Depth = depth,
            Layer = layer,
            Alpha = alpha,
            RenderStyle = VariantRenderStyle.Normal,
        });
        _profile.PermanentPatterns.Add(pattern);
        return this;
    }

    /// <summary>Adds an opaque one-row pattern carrying <paramref name="cells"/> left to right.</summary>
    public SceneFixture PatternRow(int depth, ProfileLayer layer, params (ulong Low, ulong High)[] cells)
        => PatternRow(depth, layer, 1f, cells);

    /// <summary>Adds a background tile record at the screen position (x, y).</summary>
    public SceneFixture Tile((ulong Low, ulong High) tile, int x, int y, int palette = 0)
    {
        _records.Add(new TileRecord(
            tile.Low, tile.High, (short)x, (short)y, _records.Count,
            mirrorH: false, mirrorV: false, hidden: false, (byte)palette, background: true));
        return this;
    }

    /// <summary>Adds a rectangle of background tile records, in tile cells.</summary>
    public SceneFixture Rect(
        (ulong Low, ulong High) tile,
        int column,
        int row,
        int columns,
        int rows,
        int palette = 0)
    {
        for (int y = 0; y < rows; y++)
        {
            for (int x = 0; x < columns; x++)
            {
                Tile(tile, (column + x) * 8, (row + y) * 8, palette);
            }
        }

        return this;
    }

    /// <summary>Adds a sprite tile record.</summary>
    public SceneFixture Sprite(
        (ulong Low, ulong High) tile,
        int x,
        int y,
        int palette = 0,
        bool behindBackground = false)
    {
        _records.Add(new TileRecord(
            tile.Low, tile.High, (short)x, (short)y, _spriteOrder++,
            mirrorH: false, mirrorV: false, hidden: behindBackground, (byte)palette, background: false));
        return this;
    }

    /// <summary>
    /// Whether the frame is converted in "2D Mode": the empty profile, where every tile image is
    /// learned as it appears (SPEC-06 REQ-SCN-055, SPEC-04 REQ-E3D-381).
    /// </summary>
    public bool TwoDimensionalMode { get; set; }

    /// <summary>Converts the frame and returns a scene state ready to render.</summary>
    public SceneState Build(Action<ConversionSettings>? configure = null)
    {
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.TwoDimensionalMode = TwoDimensionalMode;
        engine.LoadProfile(_profile);

        // After the load: loading a profile resets the layer shadow permissions (REQ-SCN-033).
        configure?.Invoke(engine.Settings);

        engine.Convert(new FrameBatch(_records.ToArray(), _palette, 0, 0), frameCounter: 0);
        engine.Publish();

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Clipping = engine.Settings.Clipping;
        scene.DrawList = engine.BuildDrawList();
        scene.NesBackgroundColor = engine.PublishedFrame!.Palette.Color(0);
        scene.EmulatedTime = engine.PublishedFrame.Time;
        Engine = engine;
        return scene;
    }

    /// <summary>The engine of the last <see cref="Build"/>, kept alive for the draw list.</summary>
    public ConversionEngine? Engine { get; private set; }

    /// <summary>A full tile of the colour <paramref name="color"/> with a unique identity.</summary>
    public static (ulong Low, ulong High) Solid(int color, int id = 0)
    {
        ulong low = (color & 1) != 0 ? ulong.MaxValue : 0UL;
        ulong high = (color & 2) != 0 ? ulong.MaxValue : 0UL;
        if ((color & 2) == 0)
        {
            high = 1UL << (id & 63);
        }
        else if ((color & 1) == 0)
        {
            low = 1UL << (id & 63);
        }

        return (low, high);
    }

    /// <summary>The packed RGB of a NES colour index.</summary>
    public static int NesPacked(int index) => NesPalette.Color(index).Packed;
}
