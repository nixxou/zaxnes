using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// AMD-063 REQ-SCN-087 — the inner lateral faces of a transparent background object made of
/// full tiles are taken off the drawn mesh, and nothing else changes.
/// </summary>
/// <remarks>
/// The unit of measure is the drawn mesh of one tile of the draw list, which is where the
/// requirement puts the removal: REQ-E3D-425 still governs the voxel field and the mesh that is
/// <b>built</b>.
/// </remarks>
public sealed class Amd063TransparentFacesTests
{
    /// <summary>
    /// A full tile of a non-MARCHING variant: <c>D &gt; 0</c> and all three slots of every voxel
    /// occupied. The greedy mesh of such a tile is the closed box of 24 vertices AMD-057 draws.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087, REQ-E3D-420, REQ-E3D-422
    public void A_tile_with_no_empty_pixel_and_a_positive_depth_is_full()
    {
        ConversionEngine engine = Engine(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 4));
        Batch batch = new Batch().BgCell(Img.A, 10, 10);
        ConversionFrame frame = batch.Convert(engine);

        (PatternInstance reference, TileImage image) = FirstTile(frame);
        Assert.True(engine.Geometry.IsFullTile(reference, image));
        Assert.Equal(24, engine.Geometry.MeshOf(reference, image)!.VertexCount);
    }

    /// <summary>A single empty pixel is enough to make the tile not full.</summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_tile_with_one_empty_pixel_is_not_full()
    {
        TileImageKey holed = new(ulong.MaxValue & ~1UL, 0UL, false, false);
        ConversionEngine engine = Engine(new ProfileBuilder().Pattern(1, 1, [holed], depth: 4));
        ConversionFrame frame = new Batch().BgCell(holed, 10, 10).Convert(engine);

        (PatternInstance reference, TileImage image) = FirstTile(frame);
        Assert.False(engine.Geometry.IsFullTile(reference, image));
    }

    /// <summary>
    /// <c>D = 0</c> leaves no voxel at all (REQ-E3D-420), so the tile is not full and the
    /// removal never reaches it.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087, REQ-E3D-420
    public void A_tile_of_a_depthless_object_is_not_full()
    {
        ConversionEngine engine = Engine(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 0));
        ConversionFrame frame = new Batch().BgCell(Img.A, 10, 10).Convert(engine);

        (PatternInstance reference, TileImage image) = FirstTile(frame);
        Assert.False(engine.Geometry.IsFullTile(reference, image));
    }

    /// <summary>
    /// The hollow geometries do not produce a full tile, which the requirement wants: a
    /// <c>CUBE</c> writes the six outer shells of the object and leaves the inside empty.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_cube_tile_is_not_full()
    {
        ConversionEngine engine = Engine(new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 4, geometry: VariantGeometry.Cube));
        ConversionFrame frame = new Batch().BgCell(Img.A, 10, 10).Convert(engine);

        (PatternInstance reference, TileImage image) = FirstTile(frame);
        Assert.False(engine.Geometry.IsFullTile(reference, image));
    }

    /// <summary>
    /// A variant whose effective algorithm is MARCHING has no removable inner face, so no tile
    /// of it is ever full.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void No_tile_of_a_marching_object_is_full()
    {
        ConversionEngine engine = Engine(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 4));
        engine.SetRenderAlgorithm(RenderAlgorithm.Marching);
        ConversionFrame frame = new Batch().BgCell(Img.A, 10, 10).Convert(engine);

        (PatternInstance reference, TileImage image) = FirstTile(frame);
        Assert.False(engine.Geometry.IsFullTile(reference, image));

        // The whole mesh is still handed over, sides asked for or not.
        TileMesh? whole = engine.Geometry.MeshOf(reference, image);
        Assert.Same(whole, engine.Geometry.MeshOf(reference, image, TileSides.All));
    }

    /// <summary>
    /// The first automatic criterion of REQ-SCN-087: a 4 × 4 block of identical full tiles at
    /// opacity 0.7 loses 2 lateral faces on each corner tile, 3 on each edge tile and 4 on each
    /// of the four inner ones — 16, 12 and 8 vertices.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_transparent_block_of_full_tiles_loses_its_inner_faces()
    {
        Assert.Equal([16, 12, 8], VertexCountsOfBlock(alpha: 0.7f));
    }

    /// <summary>The same block at opacity 1 keeps all 24 vertices of every tile (AMD-057).</summary>
    [Fact] // AMD-063, REQ-SCN-087, AMD-057
    public void An_opaque_block_of_full_tiles_keeps_every_face()
    {
        Assert.Equal([24, 24, 24], VertexCountsOfBlock(alpha: 1f));
    }

    /// <summary>
    /// An opacity above 1 is opaque everywhere in REQ-SCN-087 (§4, assumed divergence), and so
    /// is render style 2, whatever the opacity.
    /// </summary>
    [Theory] // AMD-063, REQ-SCN-087
    [InlineData(1.5f, VariantRenderStyle.Normal)]
    [InlineData(0.7f, VariantRenderStyle.ShadowOnly)]
    public void An_instance_that_is_not_transparent_keeps_every_face(float alpha, VariantRenderStyle style)
        => Assert.Equal([24, 24, 24], VertexCountsOfBlock(alpha, style));

    /// <summary>
    /// A script that lowers <c>shape.Alpha</c> on a variant of opacity 1 gets the removal of a
    /// variant of opacity 0.7: only the drawn instance decides.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087, SPEC-05 §4.2
    public void Lowering_the_alpha_of_the_drawn_instance_is_enough()
    {
        ConversionEngine engine = Engine(new ProfileBuilder().Pattern(1, 1, [Img.A], depth: 4));
        ConversionFrame frame = Block(engine, Img.A, column: 10, row: 10, size: 4);

        foreach (FrameInstance instance in frame.Instances)
        {
            instance.Alpha = 0.7f;
        }

        Assert.Equal([16, 12, 8], Counts(engine.BuildDrawList(frame)));
    }

    /// <summary>
    /// The third criterion, first half: two <b>different</b> transparent background objects side
    /// by side lose the face they share, on both sides.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void Two_transparent_objects_side_by_side_lose_the_face_they_share()
    {
        (int left, int right) = FacingTiles(leftAlpha: 0.7f, rightAlpha: 0.7f);

        // Each object is one tile wide: its left and right faces are the only lateral ones that
        // can go, and only the one that touches the other object does.
        Assert.Equal(20, left);
        Assert.Equal(20, right);
    }

    /// <summary>The second half: a transparent object against an opaque one keeps the face on both sides.</summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_transparent_object_against_an_opaque_one_keeps_the_face()
    {
        (int left, int right) = FacingTiles(leftAlpha: 0.7f, rightAlpha: 1f);
        Assert.Equal(24, left);
        Assert.Equal(24, right);
    }

    /// <summary>
    /// Sprites are left out of the removal (§4, assumed divergence): a transparent sprite object
    /// of full tiles keeps every face, and it does not take any face off a background neighbour
    /// either.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void A_transparent_sprite_object_of_full_tiles_loses_nothing()
    {
        ConversionEngine engine = Engine(new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 4, alpha: 0.7f)
            .Pattern(1, 1, [Img.B], depth: 4, alpha: 0.7f));

        Batch batch = new();
        for (int y = 0; y < 2; y++)
        {
            for (int x = 0; x < 2; x++)
            {
                batch.Sprite(Img.A, (10 + x) * 8, (10 + y) * 8);
            }
        }

        // One background tile right against the sprite block, so the sprite map cannot help it.
        batch.BgCell(Img.B, 12, 10);

        ConversionFrame frame = batch.Convert(engine);
        FrameDrawList list = engine.BuildDrawList(frame);
        Assert.NotEmpty(list.Tiles);
        foreach (DrawTile tile in list.Tiles)
        {
            Assert.Equal(24, tile.Mesh!.VertexCount);
        }
    }

    /// <summary>
    /// The mesh of a removed combination is memoised: the renderer sees the very same object
    /// from frame to frame, as REQ-E3D-602 wants.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087, REQ-E3D-602
    public void The_mesh_of_a_removed_combination_keeps_its_identity()
    {
        ConversionEngine engine = Engine(new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 4, alpha: 0.7f));
        ConversionFrame frame = Block(engine, Img.A, column: 10, row: 10, size: 4);

        (PatternInstance reference, TileImage image) = FirstTile(frame);
        TileMesh first = engine.Geometry.MeshOf(reference, image, TileSides.Left | TileSides.Top)!;
        TileMesh again = engine.Geometry.MeshOf(reference, image, TileSides.Left | TileSides.Top)!;
        Assert.Same(first, again);
        Assert.Equal(16, first.VertexCount);

        // A different combination is a different mesh, and the whole mesh is still the one the
        // build produced.
        Assert.NotSame(first, engine.Geometry.MeshOf(reference, image, TileSides.Left));
        Assert.Same(
            engine.Geometry.MeshOf(reference, image),
            engine.Geometry.MeshOf(reference, image, TileSides.None));
    }

    /// <summary>
    /// The front and the back faces are never removable: the mesh of an inner tile is exactly
    /// the two faces of normal <c>±z</c>, 8 vertices, and their normals say so.
    /// </summary>
    [Fact] // AMD-063, REQ-SCN-087
    public void Only_the_lateral_faces_are_ever_removed()
    {
        ConversionEngine engine = Engine(new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 4, alpha: 0.7f));
        ConversionFrame frame = Block(engine, Img.A, column: 10, row: 10, size: 4);

        (PatternInstance reference, TileImage image) = FirstTile(frame);
        TileMesh inner = engine.Geometry.MeshOf(reference, image, TileSides.All)!;
        Assert.Equal(8, inner.VertexCount);
        foreach (MeshVertex vertex in inner.Vertices)
        {
            Assert.Equal(0f, vertex.Normal.X);
            Assert.Equal(0f, vertex.Normal.Y);
            Assert.Equal(1f, MathF.Abs(vertex.Normal.Z));
        }
    }

    /// <summary>
    /// The 4 × 4 block of the first criterion, read as the three distinct vertex counts in
    /// decreasing order: corners, edges, inner tiles.
    /// </summary>
    private static int[] VertexCountsOfBlock(
        float alpha, VariantRenderStyle style = VariantRenderStyle.Normal)
    {
        ConversionEngine engine = Engine(new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 4, alpha: alpha, style: style));
        return Counts(engine.BuildDrawList(Block(engine, Img.A, column: 10, row: 10, size: 4)));
    }

    /// <summary>
    /// The distinct vertex counts of a draw list, decreasing: a 4 × 4 block has exactly three of
    /// them, the corners, the edges and the inner tiles — or one when nothing is removed.
    /// </summary>
    private static int[] Counts(FrameDrawList list)
    {
        Assert.Equal(16, list.Tiles.Count);
        int[] counts = [.. list.Tiles.Select(static tile => tile.Mesh!.VertexCount)];

        // 4 corners, 8 edges, 4 inner tiles when the three counts differ.
        int[] distinct = [.. counts.Distinct().OrderDescending()];
        if (distinct.Length == 1)
        {
            return [distinct[0], distinct[0], distinct[0]];
        }

        Assert.Equal(3, distinct.Length);
        Assert.Equal(4, counts.Count(c => c == distinct[0]));
        Assert.Equal(8, counts.Count(c => c == distinct[1]));
        Assert.Equal(4, counts.Count(c => c == distinct[2]));
        return distinct;
    }

    /// <summary>
    /// Two one-cell background objects on neighbouring cells, and the vertex count of each
    /// drawn tile — the left one first.
    /// </summary>
    private static (int Left, int Right) FacingTiles(float leftAlpha, float rightAlpha)
    {
        ConversionEngine engine = Engine(new ProfileBuilder()
            .Pattern(1, 1, [Img.A], depth: 4, alpha: leftAlpha)
            .Pattern(1, 1, [Img.B], depth: 4, alpha: rightAlpha));

        ConversionFrame frame = new Batch()
            .BgCell(Img.A, 10, 10)
            .BgCell(Img.B, 11, 10)
            .Convert(engine);

        FrameDrawList list = engine.BuildDrawList(frame);
        Assert.Equal(2, list.Tiles.Count);
        DrawTile left = list.Tiles.Single(static tile => tile.Tile.X == 80);
        DrawTile right = list.Tiles.Single(static tile => tile.Tile.X == 88);
        return (left.Mesh!.VertexCount, right.Mesh!.VertexCount);
    }

    /// <summary>An engine with geometry, carrying <paramref name="profile"/>.</summary>
    private static ConversionEngine Engine(ProfileBuilder profile)
    {
        ConversionEngine engine = new();
        engine.UseGeometry();
        engine.LoadProfile(profile.Build());
        return engine;
    }

    /// <summary>Converts a square block of identical background tiles.</summary>
    private static ConversionFrame Block(
        ConversionEngine engine, TileImageKey image, int column, int row, int size)
    {
        Batch batch = new();
        for (int y = 0; y < size; y++)
        {
            for (int x = 0; x < size; x++)
            {
                batch.BgCell(image, column + x, row + y);
            }
        }

        return batch.Convert(engine);
    }

    /// <summary>The reference 3D object and the tile image of the first tile of the frame.</summary>
    private static (PatternInstance Reference, TileImage Image) FirstTile(ConversionFrame frame)
    {
        FrameInstance instance = frame.Instances[0];
        return (instance.Reference, instance.Shape.Tiles[0].Image);
    }
}
