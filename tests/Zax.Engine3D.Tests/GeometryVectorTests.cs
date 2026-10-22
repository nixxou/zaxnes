namespace Zax.Engine3D.Tests;

/// <summary>
/// The test vectors of SPEC-04B §5.1 and §5.2: extrusion and textured box.
/// </summary>
public sealed class ExtrusionAndCubeVectorTests
{
    [Fact]
    public void TV_G1_a_single_pixel_becomes_a_prism_of_six_rectangles()
    {
        // REQ-E3D-421, 440, 501, 502, 504, 505, 506.
        TileImageKey tile = Geo.Tile("1.......");
        TileMesh mesh = Geo.MeshOf(tile, Geo.Object3D(depth: 1));

        Assert.Equal(6, Geo.Rectangles(mesh));
        Assert.True(Geo.Close(new Vec3(-4f, 3f, -0.5f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(-3f, 4f, 0.5f), mesh.BoundsMax));

        Geo.AssertRectangle(
            mesh,
            Geo.MinusX,
            (-4f, 3f, -0.5f, 0f, 0f),
            (-4f, 3f, 0.5f, 1f, 0f),
            (-4f, 4f, -0.5f, 0f, 1f),
            (-4f, 4f, 0.5f, 1f, 1f));
        Geo.AssertRectangle(
            mesh,
            Geo.MinusY,
            (-4f, 3f, -0.5f, 0f, 0f),
            (-3f, 3f, -0.5f, 1f, 0f),
            (-4f, 3f, 0.5f, 0f, 1f),
            (-3f, 3f, 0.5f, 1f, 1f));
        Geo.AssertRectangle(
            mesh,
            Geo.MinusZ,
            (-4f, 3f, -0.5f, 0f, 0f),
            (-4f, 4f, -0.5f, 0f, 1f),
            (-3f, 3f, -0.5f, 1f, 0f),
            (-3f, 4f, -0.5f, 1f, 1f));
        Geo.AssertRectangle(
            mesh,
            Geo.PlusX,
            (-3f, 3f, -0.5f, 0f, 0f),
            (-3f, 3f, 0.5f, 1f, 0f),
            (-3f, 4f, -0.5f, 0f, 1f),
            (-3f, 4f, 0.5f, 1f, 1f));
        Geo.AssertRectangle(
            mesh,
            Geo.PlusY,
            (-4f, 4f, -0.5f, 0f, 0f),
            (-3f, 4f, -0.5f, 1f, 0f),
            (-4f, 4f, 0.5f, 0f, 1f),
            (-3f, 4f, 0.5f, 1f, 1f));
        Geo.AssertRectangle(
            mesh,
            Geo.PlusZ,
            (-4f, 3f, 0.5f, 0f, 0f),
            (-4f, 4f, 0.5f, 0f, 1f),
            (-3f, 3f, 0.5f, 1f, 0f),
            (-3f, 4f, 0.5f, 1f, 1f));

        // Same pixel with D = 4: the same six rectangles, z in [−2, 2].
        TileMesh deeper = Geo.MeshOf(tile, Geo.Object3D(depth: 4));
        Assert.Equal(6, Geo.Rectangles(deeper));
        Assert.True(Geo.Close(new Vec3(-4f, 3f, -2f), deeper.BoundsMin));
        Assert.True(Geo.Close(new Vec3(-3f, 4f, 2f), deeper.BoundsMax));
    }

    [Fact]
    public void TV_G2_greedy_ignores_colour_and_multicolour_does_not()
    {
        // REQ-E3D-502, 503, 506.
        TileImageKey tile = Geo.Tile("12......");

        TileMesh greedy = Geo.MeshOf(tile, Geo.Object3D(depth: 2));
        Assert.Equal(6, Geo.Rectangles(greedy));
        Assert.True(Geo.Close(new Vec3(-4f, 3f, -1f), greedy.BoundsMin));
        Assert.True(Geo.Close(new Vec3(-2f, 4f, 1f), greedy.BoundsMax));
        Geo.AssertRectangle(
            greedy,
            Geo.MinusZ,
            (-4f, 3f, -1f, 0f, 0f),
            (-4f, 4f, -1f, 0f, 1f),
            (-2f, 3f, -1f, 2f, 0f),
            (-2f, 4f, -1f, 2f, 1f));
        Geo.AssertRectangle(
            greedy,
            Geo.PlusX,
            (-2f, 3f, -1f, 1f, 0f),
            (-2f, 3f, 1f, 2f, 0f),
            (-2f, 4f, -1f, 1f, 1f),
            (-2f, 4f, 1f, 2f, 1f));

        TileMesh multicolor = Geo.MeshOf(
            tile,
            Geo.Object3D(depth: 2),
            RenderAlgorithm.GreedyMulticolor);
        Assert.Equal(10, Geo.Rectangles(multicolor));
        Assert.Equal(1, Geo.RectanglesWithNormal(multicolor, Geo.MinusX));
        Assert.Equal(1, Geo.RectanglesWithNormal(multicolor, Geo.PlusX));
        Assert.Equal(2, Geo.RectanglesWithNormal(multicolor, Geo.MinusY));
        Assert.Equal(2, Geo.RectanglesWithNormal(multicolor, Geo.PlusY));
        Assert.Equal(2, Geo.RectanglesWithNormal(multicolor, Geo.MinusZ));
        Assert.Equal(2, Geo.RectanglesWithNormal(multicolor, Geo.PlusZ));

        // The two halves of the −Z face, uv (0,0)–(1,1) then (1,0)–(2,1).
        List<(Vec3 Position, Vec2 Uv)> minusZ = Geo.WithNormal(multicolor, Geo.MinusZ);
        Assert.Contains((new Vec3(-4f, 3f, -1f), new Vec2(0f, 0f)), minusZ);
        Assert.Contains((new Vec3(-3f, 4f, -1f), new Vec2(1f, 1f)), minusZ);
        Assert.Contains((new Vec3(-3f, 3f, -1f), new Vec2(1f, 0f)), minusZ);
        Assert.Contains((new Vec3(-2f, 4f, -1f), new Vec2(2f, 1f)), minusZ);
    }

    [Fact]
    public void TV_G3_a_full_tile_is_six_rectangles_whatever_the_colours()
    {
        // REQ-E3D-502: GREEDY merges across colours.
        TileMesh mesh = Geo.MeshOf(Geo.Solid(1), Geo.Object3D(depth: 8));
        Assert.Equal(6, Geo.Rectangles(mesh));
        Assert.True(Geo.Close(new Vec3(-4f, -4f, -4f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(4f, 4f, 4f), mesh.BoundsMax));

        TileMesh striped = Geo.MeshOf(Geo.Rows("12121212"), Geo.Object3D(depth: 2));
        Assert.Equal(6, Geo.Rectangles(striped));
    }

    [Fact]
    public void TV_G4_an_L_shape_gives_ten_rectangles()
    {
        // REQ-E3D-502, 506.
        TileMesh mesh = Geo.MeshOf(Geo.Tile("11......", "1......."), Geo.Object3D(depth: 1));

        Assert.Equal(10, Geo.Rectangles(mesh));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.MinusX));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.MinusY));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.MinusZ));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.PlusX));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.PlusY));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.PlusZ));

        Geo.AssertRectangle(
            mesh,
            Geo.MinusX,
            (-4f, 2f, -0.5f, 0f, 2f),
            (-4f, 2f, 0.5f, 1f, 2f),
            (-4f, 4f, -0.5f, 0f, 0f),
            (-4f, 4f, 0.5f, 1f, 0f));
        Geo.AssertRectangle(
            mesh,
            Geo.PlusY,
            (-4f, 4f, -0.5f, 0f, 0f),
            (-2f, 4f, -0.5f, 2f, 0f),
            (-4f, 4f, 0.5f, 0f, 1f),
            (-2f, 4f, 0.5f, 2f, 1f));
    }

    [Fact]
    public void TV_G5_empty_colour_four_makes_every_pixel_opaque()
    {
        // REQ-E3D-424: e = 4 means none of the colours 0 to 3 is transparent.
        TileMesh mesh = Geo.MeshOf(
            Geo.Tile("10......"),
            Geo.Object3D(depth: 1, emptyColor: 4));

        Assert.Equal(6, Geo.Rectangles(mesh));
        Assert.True(Geo.Close(new Vec3(-4f, -4f, -0.5f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(4f, 4f, 0.5f), mesh.BoundsMax));
    }

    [Fact]
    public void TV_G6_full_warp_puts_the_side_faces_on_the_back_half()
    {
        // REQ-E3D-441.
        TileMesh mesh = Geo.MeshOf(
            Geo.Tile("1......."),
            Geo.Object3D(depth: 2, fullWarp: true));

        Assert.Equal(6, Geo.Rectangles(mesh));

        // ±Z cover z = ∓1 over the pixel.
        Geo.AssertRectangle(
            mesh,
            Geo.MinusZ,
            (-4f, 3f, -1f, 0f, 0f),
            (-4f, 4f, -1f, 0f, 1f),
            (-3f, 3f, -1f, 1f, 0f),
            (-3f, 4f, -1f, 1f, 1f));

        // ±X and ±Y only exist on the back half, z in [0, 1].
        Geo.AssertRectangle(
            mesh,
            Geo.MinusX,
            (-4f, 3f, 0f, 0f, 0f),
            (-4f, 3f, 1f, 1f, 0f),
            (-4f, 4f, 0f, 0f, 1f),
            (-4f, 4f, 1f, 1f, 1f));
    }

    [Fact]
    public void TV_G7_two_different_full_tiles_keep_the_wall_between_them()
    {
        // REQ-E3D-501 notes: each tile mesh is closed on itself.
        EnginePattern pattern = Geo.Pattern(2, 1, Geo.Solid(1), Geo.Solid(2));
        PatternInstance instance = Geo.Object3D(depth: 1);

        Assert.Equal(2, pattern.Images.Count);
        for (int image = 0; image < 2; image++)
        {
            TileMesh mesh = Geo.Mesh(pattern, instance, RenderAlgorithm.Greedy, image);
            Assert.Equal(6, Geo.Rectangles(mesh));
            Assert.True(Geo.Close(new Vec3(-4f, -4f, -0.5f), mesh.BoundsMin));
            Assert.True(Geo.Close(new Vec3(4f, 4f, 0.5f), mesh.BoundsMax));
            Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.PlusX));
            Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.MinusX));
        }
    }

    [Fact]
    public void TV_G8_multicolour_stripes_give_thirty_four_rectangles()
    {
        // REQ-E3D-503.
        TileMesh mesh = Geo.MeshOf(
            Geo.Rows("12121212"),
            Geo.Object3D(depth: 2),
            RenderAlgorithm.GreedyMulticolor);

        Assert.Equal(34, Geo.Rectangles(mesh));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.MinusX));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.PlusX));
        Assert.Equal(8, Geo.RectanglesWithNormal(mesh, Geo.MinusY));
        Assert.Equal(8, Geo.RectanglesWithNormal(mesh, Geo.PlusY));
        Assert.Equal(8, Geo.RectanglesWithNormal(mesh, Geo.MinusZ));
        Assert.Equal(8, Geo.RectanglesWithNormal(mesh, Geo.PlusZ));
    }

    [Fact]
    public void TV_C1_a_cube_of_a_full_tile_has_twelve_rectangles()
    {
        // REQ-E3D-443: six outer faces and six inner ones.
        TileMesh mesh = Geo.MeshOf(Geo.Solid(1), Geo.Object3D(depth: 4, geometry: Geo3DType.Cube));

        Assert.Equal(12, Geo.Rectangles(mesh));
        foreach (Vec3 normal in new[] { Geo.MinusX, Geo.PlusX, Geo.MinusY, Geo.PlusY, Geo.MinusZ, Geo.PlusZ })
        {
            Assert.Equal(2, Geo.RectanglesWithNormal(mesh, normal));
        }

        Assert.True(Geo.Close(new Vec3(-4f, -4f, -2f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(4f, 4f, 2f), mesh.BoundsMax));
    }

    [Fact]
    public void TV_C2_a_cube_of_a_ring_has_forty_rectangles()
    {
        // REQ-E3D-443 with transparent texels inside the box.
        TileImageKey ring = Geo.Tile(
            "11111111",
            "1......1",
            "1......1",
            "1......1",
            "1......1",
            "1......1",
            "1......1",
            "11111111");
        TileMesh mesh = Geo.MeshOf(ring, Geo.Object3D(depth: 4, geometry: Geo3DType.Cube));

        Assert.Equal(40, Geo.Rectangles(mesh));
        Assert.Equal(6, Geo.RectanglesWithNormal(mesh, Geo.MinusX));
        Assert.Equal(6, Geo.RectanglesWithNormal(mesh, Geo.PlusX));
        Assert.Equal(6, Geo.RectanglesWithNormal(mesh, Geo.MinusY));
        Assert.Equal(6, Geo.RectanglesWithNormal(mesh, Geo.PlusY));
        Assert.Equal(8, Geo.RectanglesWithNormal(mesh, Geo.MinusZ));
        Assert.Equal(8, Geo.RectanglesWithNormal(mesh, Geo.PlusZ));
    }

    [Fact]
    public void TV_C3_two_cells_of_the_same_tile_share_one_mesh_with_both_walls()
    {
        // REQ-E3D-425: the shared field receives the writes of both cells.
        EnginePattern shared = Geo.Pattern(2, 1, Geo.Solid(1), Geo.Solid(1));
        PatternInstance instance = Geo.Object3D(depth: 2, geometry: Geo3DType.Cube);

        Assert.Single(shared.Images);
        TileMesh mesh = Geo.Mesh(shared, instance, RenderAlgorithm.Greedy);
        Assert.Equal(10, Geo.Rectangles(mesh));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.MinusX));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.PlusX));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.MinusY));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.PlusY));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.MinusZ));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.PlusZ));

        // Both walls are present: x = −4 and x = +4.
        List<(Vec3 Position, Vec2 Uv)> left = Geo.WithNormal(mesh, Geo.MinusX);
        Assert.Contains(left, pair => MathF.Abs(pair.Position.X + 4f) < 1e-3f);
        Assert.Contains(left, pair => MathF.Abs(pair.Position.X - 3f) < 1e-3f);

        // Two different tiles instead: two meshes of eight rectangles each.
        EnginePattern distinct = Geo.Pattern(2, 1, Geo.Solid(1), Geo.Solid(2));
        Assert.Equal(2, distinct.Images.Count);
        for (int image = 0; image < 2; image++)
        {
            TileMesh one = Geo.Mesh(distinct, instance, RenderAlgorithm.Greedy, image);
            Assert.Equal(8, Geo.Rectangles(one));
            Assert.Equal(1, Geo.RectanglesWithNormal(one, Geo.MinusX));
            Assert.Equal(1, Geo.RectanglesWithNormal(one, Geo.PlusX));
            Assert.Equal(2, Geo.RectanglesWithNormal(one, Geo.MinusY));
            Assert.Equal(2, Geo.RectanglesWithNormal(one, Geo.PlusY));
        }
    }

    [Fact]
    public void TV_C4_a_cube_of_one_opaque_row_has_ten_rectangles()
    {
        // REQ-E3D-443, 506: the side faces show the columns D − 1 to 0.
        TileMesh mesh = Geo.MeshOf(
            Geo.Tile("11111111"),
            Geo.Object3D(depth: 2, geometry: Geo3DType.Cube));

        Assert.Equal(10, Geo.Rectangles(mesh));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.MinusX));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.PlusX));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.MinusY));
        Assert.Equal(2, Geo.RectanglesWithNormal(mesh, Geo.PlusY));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.MinusZ));
        Assert.Equal(1, Geo.RectanglesWithNormal(mesh, Geo.PlusZ));

        // −X at x = −4: y in [3, 4], z in [−1, 1], uv (2,0), (0,0), (2,1), (0,1).
        List<(Vec3 Position, Vec2 Uv)> minusX = Geo.WithNormal(mesh, Geo.MinusX);
        Assert.Contains((new Vec3(-4f, 3f, -1f), new Vec2(2f, 0f)), minusX);
        Assert.Contains((new Vec3(-4f, 3f, 1f), new Vec2(0f, 0f)), minusX);
        Assert.Contains((new Vec3(-4f, 4f, -1f), new Vec2(2f, 1f)), minusX);
        Assert.Contains((new Vec3(-4f, 4f, 1f), new Vec2(0f, 1f)), minusX);

        // The bottom face only shows row 0 of the texture: z in [−1, 0].
        List<(Vec3 Position, Vec2 Uv)> minusY = Geo.WithNormal(mesh, Geo.MinusY);
        Assert.Contains(minusY, pair => Geo.Close(pair.Position, new Vec3(-4f, -4f, -1f)));
        Assert.Contains(minusY, pair => Geo.Close(pair.Position, new Vec3(4f, -4f, 0f)));

        // The top face shows the same rows the other way round: z in [0, 1].
        Assert.Contains(minusY, pair => Geo.Close(pair.Position, new Vec3(-4f, 3f, 0f)));
        Assert.Contains(minusY, pair => Geo.Close(pair.Position, new Vec3(4f, 3f, 1f)));
    }

    [Fact]
    public void REQ_E3D_420_a_depth_of_zero_produces_no_geometry()
    {
        TileMesh mesh = Geo.MeshOf(Geo.Solid(1), Geo.Object3D(depth: 0));
        Assert.True(mesh.IsEmpty);
        Assert.Equal(0, mesh.TriangleCount);
    }

    [Fact]
    public void REQ_E3D_442_marching_full_warp_takes_the_lowest_opaque_texel()
    {
        // Column of three pixels of colours 1, 2, 3 top to bottom, D = 3.
        EnginePattern pattern = Geo.OneTile(Geo.Tile("1.......", "2.......", "3......."));
        PatternInstance instance = Geo.Object3D(depth: 3, fullWarp: true);
        TileVoxels[] fields = Geo.Fields(pattern, instance, out _, RenderAlgorithm.Marching);
        ObjectTexture texture = Geo.TextureOf(pattern, instance);

        // The slice k = 1 of the top pixel takes the colour 2, the slice k = 2 the colour 3.
        Assert.True(fields[0].TryTexel(0, 7, 1, 0, out int tx1, out int ty1));
        Assert.Equal(2, texture.Texel(tx1, ty1));
        Assert.True(fields[0].TryTexel(0, 7, 2, 0, out int tx2, out int ty2));
        Assert.Equal(3, texture.Texel(tx2, ty2));
    }

    [Fact]
    public void REQ_E3D_426_a_texture_smaller_than_the_pattern_repeats_over_it()
    {
        // An 8 x 8 texture on an 8 x 192 pattern of 24 distinct tiles: the 24 meshes carry the
        // same texture coordinates modulo 8, each block of 8 rows reading the same texture.
        TileImageKey?[] cells = new TileImageKey?[24];
        for (int n = 0; n < cells.Length; n++)
        {
            // Distinct images: a full tile of colour 1 with one marker pixel of colour 2.
            string[] rows = new string[8];
            for (int y = 0; y < 8; y++)
            {
                rows[y] = y == n % 8 ? "211111111"[..8] : "11111111";
            }

            cells[n] = Geo.Tile(rows) with { FlipX = n >= 8 && n < 16, FlipY = n >= 16 };
        }

        EnginePattern pattern = Geo.Pattern(1, 24, cells);
        Assert.Equal(24, pattern.Images.Count);

        PatternInstance instance = Geo.Object3D(depth: 1);
        instance.Texture = new Profiles.VariantTexture(
            8,
            8,
            new Profiles.TexturePixels(Profiles.TextureFormat.Indexed, BuildSolidIndexed(8, 8, 1)));

        ObjectTexture texture = Geo.TextureOf(pattern, instance);
        Assert.Equal(8, texture.Width);
        Assert.Equal(8, texture.Height);

        List<string>? reference = null;
        for (int image = 0; image < pattern.Images.Count; image++)
        {
            TileMesh mesh = Geo.Mesh(pattern, instance, RenderAlgorithm.Greedy, image);

            // The texture is uniform, so every pixel is opaque and the tile is a full prism.
            Assert.Equal(6, Geo.Rectangles(mesh));

            List<string> reduced = [];
            foreach (MeshVertex vertex in mesh.Vertices)
            {
                reduced.Add($"{vertex.Position} {Modulo(vertex.Uv.X, 8f)} {Modulo(vertex.Uv.Y, 8f)}");
            }

            reduced.Sort(StringComparer.Ordinal);
            if (reference is null)
            {
                reference = reduced;
            }
            else
            {
                Assert.Equal(reference, reduced);
            }
        }
    }

    private static float Modulo(float value, float size)
    {
        float result = value % size;
        return result < 0f ? result + size : result;
    }

    private static int[] BuildSolidIndexed(int width, int height, int value)
    {
        int words = ((width * height) + 15) / 16;
        int[] data = new int[words];
        for (int n = 0; n < width * height; n++)
        {
            data[n / 16] |= (value & 3) << (2 * (n % 16));
        }

        return data;
    }
}
