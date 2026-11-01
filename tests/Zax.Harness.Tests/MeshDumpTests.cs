using System.Text.Json;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;

namespace Zax.Harness.Tests;

/// <summary>
/// The <c>render --dump-meshes</c> export: one entry per drawn tile instance of the captured
/// frame, with the mesh sizes and the bounds of each tile in its own frame and in the scene
/// (SPEC-04B REQ-E3D-641, REQ-E3D-504, REQ-E3D-406; SPEC-01 REQ-ARCH-031).
/// </summary>
/// <remarks>
/// The frames are synthetic and need no ROM and no GL context: a profile of one-cell patterns, a
/// batch of tile records, the conversion, and the draw list the renderer would have consumed.
/// </remarks>
public sealed class MeshDumpTests
{
    /// <summary>A tile whose eight rows are all of colour 1: a full, opaque 8 × 8 image.</summary>
    private const ulong FullLow = ulong.MaxValue;

    [Fact] // REQ-E3D-641, REQ-ARCH-031
    public void Every_drawn_tile_of_the_frame_is_one_entry()
    {
        // Four tiles two cells apart, so each one is a shape of its own: four drawn tiles.
        (int X, int Y)[] places = [(96, 96), (112, 96), (96, 112), (112, 112)];
        Fixture fixture = new();
        fixture.Pattern(depth: 3, ProfileLayer.L1);
        foreach ((int x, int y) in places)
        {
            fixture.Tile(x, y);
        }

        JsonElement dump = Dump(fixture);

        Assert.Equal(4, dump.GetProperty("tileCount").GetInt32());
        JsonElement[] tiles = [.. dump.GetProperty("tiles").EnumerateArray()];
        Assert.Equal(4, tiles.Length);

        // The position is the one the core emitted, in the convention of '--dump-frame': the
        // screen pixel of the tile's top-left corner.
        Assert.Equal(
            [.. places.Select(place => (place.X, place.Y)).Order()],
            [.. tiles.Select(Position).Order()]);

        foreach (JsonElement tile in tiles)
        {
            Assert.False(tile.GetProperty("sprite").GetBoolean());
            Assert.False(tile.GetProperty("hud").GetBoolean());
            Assert.Equal("Greedy", tile.GetProperty("mesh").GetString());
            Assert.Equal(string.Empty, tile.GetProperty("tags").GetString());
        }
    }

    [Fact] // REQ-E3D-504, REQ-E3D-406, REQ-E3D-501
    public void A_full_tile_at_the_default_depth_spans_eight_units_and_its_depth()
    {
        // The default sizeZ of an instance is 3 (REQ-E3D-453), so the extruded box of a full
        // 8 × 8 tile is 8 x 8 x 3 voxels and GREEDY closes it with one rectangle per side.
        Fixture fixture = new();
        fixture.Pattern(depth: 3, ProfileLayer.L1);
        fixture.Tile(96, 96);

        JsonElement tile = Assert.Single(Dump(fixture).GetProperty("tiles").EnumerateArray());

        Assert.Equal(3, tile.GetProperty("depth").GetInt32());
        Assert.Equal(24, tile.GetProperty("vertices").GetInt32());
        Assert.Equal(12, tile.GetProperty("triangles").GetInt32());

        // REQ-E3D-504: the local frame is centred on the tile, (i − 4, j − 4, k − D/2).
        Assert.Equal([-4f, -4f, -1.5f], Vector(tile, "localMin"));
        Assert.Equal([4f, 4f, 1.5f], Vector(tile, "localMax"));

        // REQ-E3D-406 with no rotation and no scale is the translation to the tile centre T of
        // REQ-E3D-405: x keeps the screen column, y is counted from the bottom of the scene
        // (REQ-E3D-404), and z spans the extrusion depth around the instance's centre.
        float[] min = Vector(tile, "worldMin");
        float[] max = Vector(tile, "worldMax");
        Assert.Equal(96f, min[0]);
        Assert.Equal(104f, max[0]);
        Assert.Equal(ConversionGrid.SceneHeight - 104f, min[1]);
        Assert.Equal(ConversionGrid.SceneHeight - 96f, max[1]);
        Assert.Equal(3f, max[2] - min[2], 4);
    }

    [Fact] // AMD-057, REQ-E3D-501, REQ-SCN-065
    public void Two_full_tiles_of_one_object_that_touch_each_keep_the_whole_box()
    {
        // AMD-057 withdraws AMD-027 §1-2: the drawn mesh is the whole mesh of REQ-E3D-501, so
        // a full tile is a closed box of six rectangles whether or not a neighbour of the same
        // object covers one of its walls. The walls between the two are kept and the renderer
        // settles the depth tie they make (SPEC-06 REQ-SCN-065).
        Fixture fixture = new();
        fixture.Pattern(depth: 3, ProfileLayer.L1);

        // Four full tiles in a square, cell against cell: the pattern is self-connected, so the
        // four end up in one shape and the inner walls are all covered by a neighbour.
        foreach ((int x, int y) in new[] { (96, 96), (104, 96), (96, 104), (104, 104) })
        {
            fixture.Tile(x, y);
        }

        JsonElement[] tiles = [.. Dump(fixture).GetProperty("tiles").EnumerateArray()];

        Assert.Equal(4, tiles.Length);
        foreach (JsonElement tile in tiles)
        {
            Assert.Equal(24, tile.GetProperty("vertices").GetInt32());
            Assert.Equal(12, tile.GetProperty("triangles").GetInt32());
        }
    }

    [Fact] // REQ-E3D-642, REQ-SCN-044
    public void A_tile_of_the_interface_layer_is_marked_as_hud()
    {
        Fixture fixture = new();
        fixture.Pattern(depth: 3, ProfileLayer.Ui, tags: "hud,!cast");
        fixture.Tile(96, 96);

        JsonElement tile = Assert.Single(Dump(fixture).GetProperty("tiles").EnumerateArray());

        Assert.True(tile.GetProperty("hud").GetBoolean());
        Assert.False(tile.GetProperty("sprite").GetBoolean());
        Assert.Equal("hud,!cast", tile.GetProperty("tags").GetString());
    }

    [Fact] // REQ-E3D-641
    public void A_sprite_tile_is_marked_and_keeps_its_own_depth()
    {
        Fixture fixture = new();
        fixture.Pattern(depth: 8, ProfileLayer.L1);
        fixture.Sprite(96, 96);

        JsonElement tile = Assert.Single(Dump(fixture).GetProperty("tiles").EnumerateArray());

        Assert.True(tile.GetProperty("sprite").GetBoolean());
        Assert.False(tile.GetProperty("hud").GetBoolean());
        Assert.Equal(8, tile.GetProperty("depth").GetInt32());
        Assert.Equal(-4f, Vector(tile, "localMin")[2]);
        Assert.Equal(4f, Vector(tile, "localMax")[2]);
    }

    [Fact] // REQ-E3D-500, REQ-E3D-340
    public void The_entry_reports_the_algorithm_the_tile_really_used()
    {
        // rm = 2 on the instance overrides the global setting (REQ-E3D-500), and the header
        // still reports the global one.
        Fixture fixture = new();
        fixture.Pattern(depth: 3, ProfileLayer.L1, algorithm: MeshAlgorithm.Marching);
        fixture.Tile(96, 96);

        JsonElement dump = Dump(fixture);

        Assert.Equal("Greedy", dump.GetProperty("algorithm").GetString());
        JsonElement tile = Assert.Single(dump.GetProperty("tiles").EnumerateArray());
        Assert.Equal("Marching", tile.GetProperty("mesh").GetString());
    }

    [Fact] // REQ-ARCH-031
    public void Several_captures_each_get_their_own_file()
    {
        // One capture writes exactly the path the command line gave; several insert the scene
        // name, so that no dump overwrites another.
        Assert.Equal(
            Path.Combine("out", "meshes.json"),
            MeshDump.PathFor(Path.Combine("out", "meshes.json"), "framing", single: true));
        Assert.Equal(
            Path.Combine("out", "meshes.framing.json"),
            MeshDump.PathFor(Path.Combine("out", "meshes.json"), "framing", single: false));
    }

    [Fact] // REQ-ARCH-031
    public void The_dump_is_written_as_a_utf8_json_file()
    {
        Fixture fixture = new();
        fixture.Pattern(depth: 3, ProfileLayer.L1);
        fixture.Tile(96, 96);

        string path = Path.Combine(Path.GetTempPath(), $"zax-meshes-{Guid.NewGuid():N}.json");
        try
        {
            MeshDump.WriteFile(path, fixture.Build(), RenderAlgorithm.Greedy, "unit");

            byte[] bytes = File.ReadAllBytes(path);
            Assert.NotEqual<byte>(0xEF, bytes[0]);

            using JsonDocument document = JsonDocument.Parse(bytes);
            Assert.Equal("unit", document.RootElement.GetProperty("scene").GetString());
            Assert.Equal(1, document.RootElement.GetProperty("tileCount").GetInt32());
        }
        finally
        {
            File.Delete(path);
        }
    }

    /// <summary>Converts the fixture's frame and parses the dump of its draw list.</summary>
    private static JsonElement Dump(Fixture fixture)
    {
        string json = MeshDump.Write(fixture.Build(), RenderAlgorithm.Greedy, "unit");
        using JsonDocument document = JsonDocument.Parse(json);
        return document.RootElement.Clone();
    }

    private static (int X, int Y) Position(JsonElement tile)
    {
        JsonElement at = tile.GetProperty("tile");
        return (at[0].GetInt32(), at[1].GetInt32());
    }

    private static float[] Vector(JsonElement tile, string name)
        => [.. tile.GetProperty(name).EnumerateArray().Select(value => value.GetSingle())];

    /// <summary>
    /// A profile of one-cell patterns and a batch of tile records, converted into the draw list
    /// the renderer would consume. It is the <c>render --frames &lt;file.json&gt;</c> path of the
    /// harness without the file and without the GL context.
    /// </summary>
    private sealed class Fixture
    {
        private readonly GameProfile _profile = GameProfile.CreateEmpty();
        private readonly List<TileRecord> _records = [];
        private readonly byte[] _palette = new byte[32];
        private int _spriteOrder;

        public Fixture()
        {
            // A readable palette: the universal background plus one visible colour per table.
            _palette[0] = 0x0F;
            for (int entry = 1; entry < 32; entry++)
            {
                _palette[entry] = 0x30;
            }
        }

        /// <summary>Adds a one-cell permanent pattern of the full tile, with one 3D variant.</summary>
        public Fixture Pattern(
            int depth,
            ProfileLayer layer,
            string? tags = null,
            MeshAlgorithm algorithm = MeshAlgorithm.FromSettings)
        {
            ProfilePattern pattern = new()
            {
                Grid = new PatternGrid(1, 1, [new ProfileTile(FullLow, 0UL, TileFlip.None)]),
                TileSize = new Vec2I(1, 1),
                Permanent = true,
                SelfConnected = true,
                ConstraintSize = Vec2I.Unconstrained,
            };
            pattern.Variants.Add(new PatternVariant
            {
                Depth = depth,
                Layer = layer,
                Alpha = 1f,
                RenderStyle = VariantRenderStyle.Normal,
                MeshAlgorithm = algorithm,
                TagString = tags,
            });
            _profile.PermanentPatterns.Add(pattern);
            return this;
        }

        /// <summary>Adds a background tile record at the screen pixel (x, y).</summary>
        public Fixture Tile(int x, int y)
        {
            _records.Add(new TileRecord(
                FullLow, 0UL, (short)x, (short)y, _records.Count,
                mirrorH: false, mirrorV: false, hidden: false, palette: 0, background: true));
            return this;
        }

        /// <summary>Adds a sprite tile record at the screen pixel (x, y).</summary>
        public Fixture Sprite(int x, int y)
        {
            _records.Add(new TileRecord(
                FullLow, 0UL, (short)x, (short)y, _spriteOrder++,
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
            return engine.BuildDrawList();
        }
    }
}
