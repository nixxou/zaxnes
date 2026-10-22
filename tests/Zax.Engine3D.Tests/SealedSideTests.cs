namespace Zax.Engine3D.Tests;

/// <summary>
/// Q-067 (SPEC-06 REQ-SCN-065): the wall that closes a tile against a neighbour of the same
/// object is left out of the <b>drawn</b> mesh when the neighbour covers it entirely, so the
/// joints of a flat surface show neither a line nor a fin.
/// </summary>
/// <remarks>
/// The mesh of REQ-E3D-501 is not touched: a mask of 0 — what every existing test vector asks
/// for — still gives the tile closed on itself, which is what
/// <see cref="A_sealed_side_changes_nothing_when_the_mask_is_empty"/> checks.
/// </remarks>
public sealed class SealedSideTests
{
    /// <summary>The same tile, as the image builders of the corpus tests take it.</summary>
    private static TileImageKey SolidImage => Solid;

    /// <summary>A solid 8 × 8 tile: every lateral side is a full plane.</summary>
    private static TileImageKey Solid => Geo.Tile(
        "11111111",
        "11111111",
        "11111111",
        "11111111",
        "11111111",
        "11111111",
        "11111111",
        "11111111");

    [Fact]
    public void A_sealed_side_changes_nothing_when_the_mask_is_empty()
    {
        // REQ-E3D-501, REQ-E3D-523: the default is the mesh closed on itself.
        EnginePattern pattern = Geo.OneTile(Solid);
        PatternInstance instance = Geo.Object3D(depth: 8);
        ObjectTexture texture = Geo.TextureOf(pattern, instance);
        TileVoxels field = Geo.Fields(pattern, instance, out _)[0];

        TileMesh closed = GreedyMesher.Build(field, texture, multicolor: false);
        TileMesh same = GreedyMesher.Build(field, texture, multicolor: false, sealedSides: 0);

        Assert.Equal(6, Geo.Rectangles(closed));
        Assert.Equal(closed.TriangleCount, same.TriangleCount);
        Assert.Equal(closed.VertexCount, same.VertexCount);
    }

    [Fact]
    public void A_sealed_side_drops_only_the_wall_it_names()
    {
        // Q-067: sealing +X removes the wall at x = 8 and nothing else; the five other faces
        // of the box keep the positions and the texture coordinates of REQ-E3D-504 and 506.
        EnginePattern pattern = Geo.OneTile(Solid);
        PatternInstance instance = Geo.Object3D(depth: 8);
        ObjectTexture texture = Geo.TextureOf(pattern, instance);
        TileVoxels field = Geo.Fields(pattern, instance, out _)[0];

        TileMesh sealedRight = GreedyMesher.Build(
            field,
            texture,
            multicolor: false,
            sealedSides: TileSides.PlusX);

        Assert.Equal(5, Geo.Rectangles(sealedRight));
        Assert.Equal(0, Geo.RectanglesWithNormal(sealedRight, Geo.PlusX));
        Assert.Equal(1, Geo.RectanglesWithNormal(sealedRight, Geo.MinusX));
        Assert.Equal(1, Geo.RectanglesWithNormal(sealedRight, Geo.MinusY));
        Assert.Equal(1, Geo.RectanglesWithNormal(sealedRight, Geo.PlusY));
        Assert.Equal(1, Geo.RectanglesWithNormal(sealedRight, Geo.MinusZ));
        Assert.Equal(1, Geo.RectanglesWithNormal(sealedRight, Geo.PlusZ));

        // The two tiles of a flat surface seal each other, so the pair keeps no wall at all.
        TileMesh sealedBoth = GreedyMesher.Build(
            field,
            texture,
            multicolor: false,
            sealedSides: TileSides.All);
        Assert.Equal(2, Geo.Rectangles(sealedBoth));
        Assert.Equal(1, Geo.RectanglesWithNormal(sealedBoth, Geo.MinusZ));
        Assert.Equal(1, Geo.RectanglesWithNormal(sealedBoth, Geo.PlusZ));
    }

    [Fact]
    public void A_side_that_is_not_a_full_plane_is_never_sealed()
    {
        // Safety rule of Q-067: only a side whose whole plane is occupied may be sealed, since
        // the two walls then coincide cell for cell and dropping them cannot open a hole.
        EnginePattern pattern = Geo.OneTile(Geo.Tile(
            "11111111",
            "11111111",
            "11111111",
            "11111111",
            "11111111",
            "11111111",
            "11111111",
            "1111111."));
        PatternInstance instance = Geo.Object3D(depth: 4);
        TileVoxels field = Geo.Fields(pattern, instance, out _)[0];

        int full = TileSides.FullSides(field);
        Assert.Equal(TileSides.MinusX | TileSides.PlusY, full & (TileSides.MinusX | TileSides.PlusY));
        Assert.Equal(0, full & TileSides.PlusX);
        Assert.Equal(0, full & TileSides.MinusY);
    }

    [Fact]
    public void An_empty_tile_seals_nothing()
    {
        EnginePattern pattern = Geo.OneTile(Geo.Tile(
            "........",
            "........",
            "........",
            "........",
            "........",
            "........",
            "........",
            "........"));
        PatternInstance instance = Geo.Object3D(depth: 4);
        TileVoxels field = Geo.Fields(pattern, instance, out _)[0];
        Assert.Equal(0, TileSides.FullSides(field));
    }

    [Fact]
    public void Two_full_tiles_of_one_object_drop_the_wall_between_them()
    {
        // REQ-SCN-065 at the source: a 2 × 1 surface of identical full tiles. The draw list
        // asks for the variant whose shared wall is gone, so the pair draws 5 + 5 rectangles
        // instead of 6 + 6.
        EnginePattern pattern = Geo.Pattern(2, 1, Solid, Solid);
        PatternInstance instance = Geo.Object3D(depth: 8);
        ObjectTexture texture = Geo.TextureOf(pattern, instance);
        TileVoxels field = Geo.Fields(pattern, instance, out _)[0];

        Assert.Equal(TileSides.All, TileSides.FullSides(field));

        TileMesh left = GreedyMesher.Build(field, texture, multicolor: false, sealedSides: TileSides.PlusX);
        TileMesh right = GreedyMesher.Build(field, texture, multicolor: false, sealedSides: TileSides.MinusX);
        Assert.Equal(5, Geo.Rectangles(left));
        Assert.Equal(5, Geo.Rectangles(right));

        // No face is left on the plane the two tiles share.
        Assert.Equal(0, Geo.RectanglesWithNormal(left, Geo.PlusX));
        Assert.Equal(0, Geo.RectanglesWithNormal(right, Geo.MinusX));
    }

    [Fact]
    public void The_draw_list_seals_the_joints_of_a_surface_and_keeps_its_border()
    {
        // End to end: a 2 x 2 surface of one full image. Each tile loses the two walls it
        // shares with its neighbours and keeps the two that are on the border of the surface,
        // so every tile draws four rectangles instead of six.
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.LoadProfile(new ProfileBuilder()
            .Pattern(2, 2, [SolidImage, SolidImage, SolidImage, SolidImage], depth: 8)
            .Build());

        Batch batch = new();
        batch.Bg(SolidImage, 0, 0).Bg(SolidImage, 8, 0).Bg(SolidImage, 0, 8).Bg(SolidImage, 8, 8);
        FrameDrawList list = engine.BuildDrawList(batch.Convert(engine));

        Assert.Equal(4, list.Tiles.Count);
        foreach (DrawTile tile in list.Tiles)
        {
            TileMesh mesh = Geo.Present(tile.Mesh);
            Assert.Equal(4, Geo.Rectangles(mesh));
        }

        // One tile on its own keeps the six faces of REQ-E3D-501.
        ConversionEngine alone = new();
        alone.UseGeometry();
        alone.LoadProfile(new ProfileBuilder().Pattern(1, 1, [SolidImage], depth: 8).Build());
        Batch single = new();
        single.Bg(SolidImage, 0, 0);
        DrawTile only = Assert.Single(alone.BuildDrawList(single.Convert(alone)).Tiles);
        Assert.Equal(6, Geo.Rectangles(Geo.Present(only.Mesh)));
    }

    [Fact]
    public void Sealing_is_symmetric_between_the_two_sides_of_a_joint()
    {
        Assert.Equal(TileSides.PlusX, TileSides.Opposite(TileSides.MinusX));
        Assert.Equal(TileSides.MinusX, TileSides.Opposite(TileSides.PlusX));
        Assert.Equal(TileSides.PlusY, TileSides.Opposite(TileSides.MinusY));
        Assert.Equal(TileSides.MinusY, TileSides.Opposite(TileSides.PlusY));

        // The Z sides are never sealed: nothing is ever against them inside one object.
        Assert.False(TileSides.IsSealed(TileSides.All, axis: 2, index: -1, size: 8));
        Assert.True(TileSides.IsSealed(TileSides.MinusX, axis: 0, index: -1, size: 8));
        Assert.False(TileSides.IsSealed(TileSides.MinusX, axis: 0, index: 8, size: 8));
        Assert.True(TileSides.IsSealed(TileSides.PlusY, axis: 1, index: 8, size: 8));
    }
}
