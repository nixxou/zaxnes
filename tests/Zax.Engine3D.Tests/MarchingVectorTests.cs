namespace Zax.Engine3D.Tests;

/// <summary>
/// The test vectors of SPEC-04B §5.4: the smoothed surfaces of MARCHING (REQ-E3D-520 to 524).
/// </summary>
public sealed class MarchingVectorTests
{
    [Fact]
    public void TV_M1_a_single_pixel_becomes_a_regular_octahedron()
    {
        // REQ-E3D-521, 522.
        TileMesh mesh = Geo.MeshOf(
            Geo.Tile("1......."),
            Geo.Object3D(depth: 1),
            RenderAlgorithm.Marching);

        Assert.Equal(8, mesh.TriangleCount);
        Assert.Equal(24, mesh.VertexCount);

        // Vertices of the octahedron centred on (−3.5, 3.5, 0), "radius" 1. The outward
        // orientation of the normals is the gradient rule of Q-016.
        Vec3[] expected =
        [
            new(-4.5f, 3.5f, 0f),
            new(-2.5f, 3.5f, 0f),
            new(-3.5f, 2.5f, 0f),
            new(-3.5f, 4.5f, 0f),
            new(-3.5f, 3.5f, -1f),
            new(-3.5f, 3.5f, 1f),
        ];

        foreach (MeshVertex vertex in mesh.Vertices)
        {
            Assert.Contains(expected, candidate => Geo.Close(candidate, vertex.Position));

            // uv (0.5, 0.5) for every vertex, and normals (±1, ±1, ±1)/sqrt(3).
            Assert.True(Geo.Close(new Vec2(0.5f, 0.5f), vertex.Uv));
            float root = 1f / MathF.Sqrt(3f);
            Assert.Equal(root, MathF.Abs(vertex.Normal.X), 3);
            Assert.Equal(root, MathF.Abs(vertex.Normal.Y), 3);
            Assert.Equal(root, MathF.Abs(vertex.Normal.Z), 3);
        }

        // One listed triangle, with its outward normal.
        AssertTriangle(
            mesh,
            new Vec3(-4.5f, 3.5f, 0f),
            new Vec3(-3.5f, 3.5f, -1f),
            new Vec3(-3.5f, 2.5f, 0f),
            new Vec3(-1f, -1f, -1f) * (1f / MathF.Sqrt(3f)));

        // Every normal looks away from the centre of the octahedron.
        Vec3 centre = new(-3.5f, 3.5f, 0f);
        foreach (MeshVertex vertex in mesh.Vertices)
        {
            Vec3 outward = vertex.Position - centre;
            float dot = (outward.X * vertex.Normal.X)
                + (outward.Y * vertex.Normal.Y)
                + (outward.Z * vertex.Normal.Z);
            Assert.True(dot > 0f, $"inward normal {vertex.Normal} at {vertex.Position}");
        }
    }

    [Fact]
    public void TV_M2_two_adjacent_pixels_of_one_colour_give_sixteen_triangles()
    {
        TileMesh mesh = Geo.MeshOf(
            Geo.Tile("11......"),
            Geo.Object3D(depth: 1),
            RenderAlgorithm.Marching);

        Assert.Equal(16, CountNonDegenerate(mesh));
        Assert.True(Geo.Close(new Vec3(-4.5f, 2.5f, -1f), mesh.BoundsMin));
        Assert.True(Geo.Close(new Vec3(-1.5f, 4.5f, 1f), mesh.BoundsMax));
    }

    [Fact]
    public void TV_M3_a_full_tile_gives_a_closed_surface_that_joins_its_neighbour()
    {
        // REQ-E3D-520: the neighbourhood is read in the next tile of the pattern.
        TileMesh alone = Geo.MeshOf(
            Geo.Solid(1),
            Geo.Object3D(depth: 2),
            RenderAlgorithm.Marching);

        Assert.Equal(380, CountNonDegenerate(alone));
        Assert.True(Geo.Close(new Vec3(-4.5f, -4.5f, -1.5f), alone.BoundsMin));
        Assert.True(Geo.Close(new Vec3(4.5f, 4.5f, 1.5f), alone.BoundsMax));

        // Two full tiles side by side, distinct as images but identical in content — a flip of
        // a symmetric tile is another image (SPEC-04 §2) — so each has its own mesh and its own
        // neighbourhood. Neither keeps a face in the junction plane.
        EnginePattern pair = Geo.Pattern(
            2,
            1,
            Geo.Solid(1),
            Geo.Solid(1) with { FlipX = true });
        Assert.Equal(2, pair.Images.Count);

        PatternInstance instance = Geo.Object3D(depth: 2);
        TileMesh left = Geo.Mesh(pair, instance, RenderAlgorithm.Marching, 0);
        TileMesh right = Geo.Mesh(pair, instance, RenderAlgorithm.Marching, 1);

        // A lone tile closes itself with a wall at x = 4.5; the left tile of the pair has none,
        // and neither has the right tile at x = −4.5. The caps at the front and the back still
        // reach those planes, so what counts is a triangle lying entirely in them.
        Assert.True(HasWallAt(alone, axis: 0, coordinate: 4.5f));
        Assert.False(HasWallAt(left, axis: 0, coordinate: 4.5f));
        Assert.False(HasWallAt(right, axis: 0, coordinate: -4.5f));

        // The left tile keeps its own outer walls.
        Assert.True(HasWallAt(left, axis: 0, coordinate: -4.5f));
        Assert.True(CountNonDegenerate(left) < CountNonDegenerate(alone));
    }

    /// <summary>
    /// True when at least one triangle lies entirely in the plane <c>axis = coordinate</c>,
    /// which is what a wall between two tiles looks like.
    /// </summary>
    private static bool HasWallAt(TileMesh mesh, int axis, float coordinate)
    {
        ReadOnlySpan<int> triangles = mesh.Triangles;
        for (int n = 0; n < triangles.Length; n += 3)
        {
            bool wall = true;
            for (int c = 0; c < 3 && wall; c++)
            {
                Vec3 position = mesh.Vertices[triangles[n + c]].Position;
                float value = axis switch
                {
                    0 => position.X,
                    1 => position.Y,
                    _ => position.Z,
                };
                wall = MathF.Abs(value - coordinate) < 1e-3f;
            }

            if (wall && mesh.Vertices[triangles[n]].Normal != Vec3.Zero)
            {
                return true;
            }
        }

        return false;
    }

    [Fact]
    public void TV_M4_an_opaque_empty_colour_adds_the_legacy_zero_surface()
    {
        // REQ-E3D-521 notes: with e != 0 the empty corners count as the value 0 and produce
        // their own surface; here eight of its triangles have zero area.
        TileMesh mesh = Geo.MeshOf(
            Geo.Tile("1......."),
            Geo.Object3D(depth: 1, emptyColor: 4),
            RenderAlgorithm.Marching);

        Assert.Equal(16, mesh.TriangleCount);
        Assert.Equal(8, CountNonDegenerate(mesh));
    }

    [Fact]
    public void REQ_E3D_524_the_mesher_accepts_more_than_sixty_five_thousand_vertices()
    {
        // A 3D checkerboard injected straight into the field: 18 432 rectangles,
        // 73 728 vertices.
        TileVoxels field = new(TileVoxels.MaxDepth, perAxisSlots: true);
        for (int k = 0; k < TileVoxels.MaxDepth; k++)
        {
            for (int j = 0; j < TileVoxels.SizeY; j++)
            {
                for (int i = 0; i < TileVoxels.SizeX; i++)
                {
                    if ((i + j + k) % 2 == 0)
                    {
                        field.Write(i, j, k, TileVoxels.AllAxes, 0, 0, transparent: false);
                    }
                }
            }
        }

        EnginePattern pattern = Geo.OneTile(Geo.Solid(1));
        TileMesh mesh = GreedyMesher.Build(
            field,
            ObjectTexture.FromPattern(pattern),
            multicolor: false);

        Assert.Equal(18432, Geo.Rectangles(mesh));
        Assert.Equal(73728, mesh.VertexCount);
        Assert.Equal(36864, mesh.TriangleCount);
    }

    private static int CountNonDegenerate(TileMesh mesh)
    {
        int count = 0;
        ReadOnlySpan<int> triangles = mesh.Triangles;
        for (int n = 0; n < triangles.Length; n += 3)
        {
            if (mesh.Vertices[triangles[n]].Normal != Vec3.Zero)
            {
                count++;
            }
        }

        return count;
    }

    private static void AssertTriangle(TileMesh mesh, Vec3 a, Vec3 b, Vec3 c, Vec3 normal)
    {
        ReadOnlySpan<int> triangles = mesh.Triangles;
        for (int n = 0; n < triangles.Length; n += 3)
        {
            Vec3 pa = mesh.Vertices[triangles[n]].Position;
            Vec3 pb = mesh.Vertices[triangles[n + 1]].Position;
            Vec3 pc = mesh.Vertices[triangles[n + 2]].Position;
            if (!SameCorners(pa, pb, pc, a, b, c))
            {
                continue;
            }

            Vec3 actual = mesh.Vertices[triangles[n]].Normal;
            Assert.Equal(normal.X, actual.X, 3);
            Assert.Equal(normal.Y, actual.Y, 3);
            Assert.Equal(normal.Z, actual.Z, 3);
            return;
        }

        Assert.Fail($"no triangle with the corners {a}, {b}, {c}");
    }

    private static bool SameCorners(Vec3 a0, Vec3 a1, Vec3 a2, Vec3 b0, Vec3 b1, Vec3 b2)
    {
        Vec3[] left = [a0, a1, a2];
        foreach (Vec3 corner in new[] { b0, b1, b2 })
        {
            if (!Array.Exists(left, candidate => Geo.Close(candidate, corner)))
            {
                return false;
            }
        }

        return true;
    }
}
