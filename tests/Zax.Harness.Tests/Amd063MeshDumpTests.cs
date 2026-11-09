using System.Text.Json;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-063 REQ-SCN-087 — the three automatic criteria of the amendment, read where it says to
/// read them: on the per-drawn-tile export of <c>render --dump-meshes</c> (Q-081).
/// </summary>
/// <remarks>
/// Every scene here is the one the amendment fixes: a background object of 4 × 4 identical
/// tiles, <c>geo = Default</c>, <c>fullWarp = false</c>, <c>emptyColor = 0</c>,
/// <c>sizeZ = 4</c>, no pixel of colour 0 — so every tile is full — and GREEDY.
/// </remarks>
public sealed class Amd063MeshDumpTests
{
    /// <summary>
    /// A full 8 × 8 tile of colour 1 whose pixel (0, 0) is colour 3, so that two such tiles are
    /// two distinct images without either of them carrying a pixel of colour 0.
    /// </summary>
    private static (ulong Low, ulong High) Full(int id) => (ulong.MaxValue, 1UL << (id & 63));

    /// <summary>The same tile with the pixel (7, 0) emptied: 63 opaque pixels, not full.</summary>
    private static (ulong Low, ulong High) Holed => (ulong.MaxValue & ~1UL, 0UL);

    /// <summary>
    /// First criterion — the 4 × 4 block at opacity 0.7: the four corner tiles have 2 lateral
    /// faces removed (16 vertices), the eight other border tiles 3 (12 vertices) and the four
    /// inner ones 4 (8 vertices). At opacity 1 all sixteen have 24.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_transparent_block_of_sixteen_full_tiles_loses_its_inner_faces()
    {
        Dictionary<(int X, int Y), int> transparent = Block(alpha: 0.7f);
        Dictionary<(int X, int Y), int> opaque = Block(alpha: 1f);

        Assert.Equal(16, transparent.Count);
        Assert.Equal(16, opaque.Count);

        // The block sits on the cells (10, 10) to (13, 13), so the screen pixels run 80 to 104.
        foreach ((int x, int y) in transparent.Keys)
        {
            int borders = (x == 80 || x == 104 ? 1 : 0) + (y == 80 || y == 104 ? 1 : 0);
            int expected = borders switch
            {
                2 => 16,   // a corner: two neighbours outside the object
                1 => 12,   // an edge: one
                _ => 8,    // inside: none
            };

            Assert.Equal(expected, transparent[(x, y)]);
            Assert.Equal(24, opaque[(x, y)]);
        }

        Assert.Equal(4, transparent.Values.Count(static v => v == 16));
        Assert.Equal(8, transparent.Values.Count(static v => v == 12));
        Assert.Equal(4, transparent.Values.Count(static v => v == 8));
    }

    /// <summary>
    /// Second criterion — the same scene with one non-full tile (a single empty pixel) in the
    /// middle: that tile keeps all its faces, and its four neighbours keep the face that
    /// touches it.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_non_full_tile_keeps_its_faces_and_so_do_the_faces_that_touch_it
        ()
    {
        // The hole is the cell (1, 1) of the 4 x 4 grid, screen pixel (88, 88).
        Dictionary<(int X, int Y), int> transparent = HoledBlock(alpha: 0.7f);
        Dictionary<(int X, int Y), int> opaque = HoledBlock(alpha: 1f);

        Assert.Equal(16, transparent.Count);

        // The non-full tile is untouched: exactly the mesh the build produced, which is what the
        // same scene at opacity 1 draws.
        Assert.Equal(opaque[(88, 88)], transparent[(88, 88)]);

        // Its four neighbours keep the face that touches it, so each of them has one face more
        // than it would have had in a block of nothing but full tiles.
        Dictionary<(int X, int Y), int> whole = Block(alpha: 0.7f);
        foreach ((int X, int Y) neighbour in new (int, int)[] { (80, 88), (96, 88), (88, 80), (88, 96) })
        {
            Assert.Equal(whole[neighbour] + 4, transparent[neighbour]);
        }

        // Everything else is removed exactly as before: the four corners still lose two faces.
        foreach ((int X, int Y) corner in new (int, int)[] { (80, 80), (104, 80), (80, 104), (104, 104) })
        {
            Assert.Equal(16, transparent[corner]);
        }
    }

    /// <summary>
    /// Third criterion, first half — two <b>different</b> transparent background objects side by
    /// side lose the face they share, on both sides; a transparent object against an opaque one
    /// keeps it on both sides.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void Two_objects_side_by_side_share_the_removal_only_when_both_are_transparent()
    {
        Dictionary<(int X, int Y), int> both = Facing(leftAlpha: 0.7f, rightAlpha: 0.7f);
        Assert.Equal(20, both[(80, 80)]);
        Assert.Equal(20, both[(88, 80)]);

        Dictionary<(int X, int Y), int> mixed = Facing(leftAlpha: 0.7f, rightAlpha: 1f);
        Assert.Equal(24, mixed[(80, 80)]);
        Assert.Equal(24, mixed[(88, 80)]);
    }

    /// <summary>
    /// Third criterion, second half — a transparent <c>geo = CUBE</c> object loses no face (its
    /// tiles are not full), and neither does a transparent sprite object of full tiles.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_cube_object_and_a_sprite_object_lose_no_face()
    {
        Fixture cube = new();
        cube.Pattern(depth: 4, ProfileLayer.L1, Full(0), alpha: 0.7f, geometry: VariantGeometry.Cube);
        Rect(cube, Full(0), 10, 10, 4);

        Dictionary<(int X, int Y), int> cubes = Counts(Dump(cube));
        Assert.Equal(16, cubes.Count);
        Assert.Single(cubes.Values.Distinct());

        Fixture sprites = new();
        sprites.Pattern(depth: 4, ProfileLayer.L1, Full(0), alpha: 0.7f);
        for (int y = 0; y < 2; y++)
        {
            for (int x = 0; x < 2; x++)
            {
                sprites.Sprite(Full(0), (10 + x) * 8, (10 + y) * 8);
            }
        }

        Dictionary<(int X, int Y), int> drawn = Counts(Dump(sprites));
        Assert.Equal(4, drawn.Count);
        foreach (int vertices in drawn.Values)
        {
            Assert.Equal(24, vertices);
        }
    }

    /// <summary>
    /// Third criterion, last half — the scene of the first criterion with a variant of opacity 1
    /// whose instance a script sets to 0.7: the inner faces go exactly as they do for a variant
    /// of opacity 0.7. Only the drawn instance decides.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087, SPEC-05 §4.2
    public void An_alpha_a_script_lowers_removes_the_same_faces()
    {
        Fixture fixture = new();
        fixture.Pattern(depth: 4, ProfileLayer.L1, Full(0), alpha: 1f);
        Rect(fixture, Full(0), 10, 10, 4);

        // SPEC-05 §4.2 Instance.Alpha, the member 43 of the 101 profiles write (AMD-063 §1).
        fixture.BeforeDrawing = frame =>
        {
            foreach (FrameInstance instance in frame.Instances)
            {
                instance.Alpha = 0.7f;
            }
        };

        Assert.Equal(Block(alpha: 0.7f), Counts(Dump(fixture)));
    }

    /// <summary>
    /// The dump the criteria read is the ordinary one: every drawn tile is still one entry, with
    /// its triangle count halved along with its vertices.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087, REQ-E3D-641
    public void The_dump_keeps_two_triangles_per_remaining_quad()
    {
        Fixture fixture = new();
        fixture.Pattern(depth: 4, ProfileLayer.L1, Full(0), alpha: 0.7f);
        Rect(fixture, Full(0), 10, 10, 4);

        JsonElement dump = Dump(fixture);
        Assert.Equal(16, dump.GetProperty("tileCount").GetInt32());
        foreach (JsonElement tile in dump.GetProperty("tiles").EnumerateArray())
        {
            int vertices = tile.GetProperty("vertices").GetInt32();
            Assert.Equal(vertices / 2, tile.GetProperty("triangles").GetInt32());
            Assert.Equal("Greedy", tile.GetProperty("mesh").GetString());
            Assert.False(tile.GetProperty("sprite").GetBoolean());
        }
    }

    /// <summary>The vertex count per drawn tile of the 4 × 4 block of the first criterion.</summary>
    private static Dictionary<(int X, int Y), int> Block(float alpha)
    {
        Fixture fixture = new();
        fixture.Pattern(depth: 4, ProfileLayer.L1, Full(0), alpha: alpha);
        Rect(fixture, Full(0), 10, 10, 4);
        return Counts(Dump(fixture));
    }

    /// <summary>The same block with the cell (1, 1) carrying a tile of 63 opaque pixels.</summary>
    private static Dictionary<(int X, int Y), int> HoledBlock(float alpha)
    {
        (ulong Low, ulong High)[] cells = [.. Enumerable.Repeat(Full(0), 16)];
        cells[(1 * 4) + 1] = Holed;

        Fixture fixture = new();
        fixture.Grid(4, 4, cells, depth: 4, ProfileLayer.L1, alpha: alpha);
        for (int y = 0; y < 4; y++)
        {
            for (int x = 0; x < 4; x++)
            {
                fixture.Tile(cells[(y * 4) + x], (10 + x) * 8, (10 + y) * 8);
            }
        }

        return Counts(Dump(fixture));
    }

    /// <summary>Two one-cell background objects on the cells (10, 10) and (11, 10).</summary>
    private static Dictionary<(int X, int Y), int> Facing(float leftAlpha, float rightAlpha)
    {
        Fixture fixture = new();
        fixture.Pattern(depth: 4, ProfileLayer.L1, Full(0), alpha: leftAlpha);
        fixture.Pattern(depth: 4, ProfileLayer.L1, Full(1), alpha: rightAlpha);
        fixture.Tile(Full(0), 80, 80);
        fixture.Tile(Full(1), 88, 80);
        return Counts(Dump(fixture));
    }

    /// <summary>Adds a square of background tile records, in cells.</summary>
    private static void Rect(Fixture fixture, (ulong Low, ulong High) tile, int column, int row, int size)
    {
        for (int y = 0; y < size; y++)
        {
            for (int x = 0; x < size; x++)
            {
                fixture.Tile(tile, (column + x) * 8, (row + y) * 8);
            }
        }
    }

    /// <summary>The vertex count of each entry of a dump, by the tile's screen position.</summary>
    private static Dictionary<(int X, int Y), int> Counts(JsonElement dump)
    {
        Dictionary<(int X, int Y), int> counts = [];
        foreach (JsonElement tile in dump.GetProperty("tiles").EnumerateArray())
        {
            JsonElement at = tile.GetProperty("tile");
            counts[(at[0].GetInt32(), at[1].GetInt32())] = tile.GetProperty("vertices").GetInt32();
        }

        return counts;
    }

    /// <summary>Converts the fixture's frame and parses the dump of its draw list.</summary>
    private static JsonElement Dump(Fixture fixture)
    {
        string json = MeshDump.Write(fixture.Build(), RenderAlgorithm.Greedy, "amd063");
        using JsonDocument document = JsonDocument.Parse(json);
        return document.RootElement.Clone();
    }

    /// <summary>
    /// A profile and a batch of tile records converted into the draw list the renderer consumes,
    /// the <c>render --frames &lt;file.json&gt;</c> path of the harness without the file and
    /// without the GL context.
    /// </summary>
    private sealed class Fixture
    {
        private readonly GameProfile _profile = GameProfile.CreateEmpty();
        private readonly List<TileRecord> _records = [];
        private readonly byte[] _palette = new byte[32];
        private int _spriteOrder;

        public Fixture()
        {
            _palette[0] = 0x0F;
            for (int entry = 1; entry < 32; entry++)
            {
                _palette[entry] = 0x30;
            }
        }

        /// <summary>What a script would do between the conversion and the draw list.</summary>
        public Action<ConversionFrame>? BeforeDrawing { get; set; }

        /// <summary>Adds a one-cell permanent pattern with one 3D variant.</summary>
        public Fixture Pattern(
            int depth,
            ProfileLayer layer,
            (ulong Low, ulong High) tile,
            float alpha = 1f,
            VariantGeometry geometry = VariantGeometry.Default)
            => Grid(1, 1, [tile], depth, layer, alpha, geometry);

        /// <summary>Adds a permanent pattern of an explicit grid of cells, row by row.</summary>
        public Fixture Grid(
            int columns,
            int rows,
            (ulong Low, ulong High)[] cells,
            int depth,
            ProfileLayer layer,
            float alpha = 1f,
            VariantGeometry geometry = VariantGeometry.Default)
        {
            ArgumentNullException.ThrowIfNull(cells);
            ProfilePattern pattern = new()
            {
                Grid = new PatternGrid(
                    columns,
                    rows,
                    [.. cells.Select(static cell => new ProfileTile(cell.Low, cell.High, TileFlip.None))]),
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
                Geometry = geometry,
                StoredEmptyColor = 0,
                FullWarp = false,
            });
            _profile.PermanentPatterns.Add(pattern);
            return this;
        }

        /// <summary>Adds a background tile record at the screen pixel (x, y).</summary>
        public Fixture Tile((ulong Low, ulong High) tile, int x, int y)
        {
            _records.Add(new TileRecord(
                tile.Low, tile.High, (short)x, (short)y, _records.Count,
                mirrorH: false, mirrorV: false, hidden: false, palette: 0, background: true));
            return this;
        }

        /// <summary>Adds a sprite tile record at the screen pixel (x, y).</summary>
        public Fixture Sprite((ulong Low, ulong High) tile, int x, int y)
        {
            _records.Add(new TileRecord(
                tile.Low, tile.High, (short)x, (short)y, _spriteOrder++,
                mirrorH: false, mirrorV: false, hidden: false, palette: 0, background: false));
            return this;
        }

        /// <summary>Converts the frame and builds the draw list of REQ-E3D-641.</summary>
        public FrameDrawList Build()
        {
            ConversionEngine engine = new();
            engine.UseGeometry();
            engine.LoadProfile(_profile);
            engine.Convert(new FrameBatch([.. _records], _palette, 0, 0), frameCounter: 0);
            engine.Publish();
            BeforeDrawing?.Invoke(engine.PublishedFrame!);
            return engine.BuildDrawList();
        }
    }
}
