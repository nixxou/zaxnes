using System.Globalization;

namespace Zax.Engine3D.Tests;

/// <summary>
/// Builds the little patterns and 3D objects the test vectors of SPEC-04B §5 describe, and
/// reads the results back in the shape those vectors use.
/// </summary>
/// <remarks>
/// Conventions of §5: a tile image is given as eight rows, row 0 at the top, <c>.</c> for the
/// colour 0 and a digit for the colours 1 to 3. Unless said otherwise a vector uses a
/// one-tile pattern, <c>emptyColor = 0</c>, <c>GREEDY</c> and the pattern image as texture.
/// </remarks>
internal static class Geo
{
    /// <summary>A tile image given row by row, row 0 at the top (§5).</summary>
    public static TileImageKey Tile(params string[] rows)
    {
        ArgumentNullException.ThrowIfNull(rows);
        ulong low = 0;
        ulong high = 0;
        for (int y = 0; y < rows.Length && y < 8; y++)
        {
            string row = rows[y];
            for (int x = 0; x < 8; x++)
            {
                int color = x < row.Length && row[x] != '.'
                    ? row[x] - '0'
                    : 0;
                int bit = (y << 3) | (7 - x);
                if ((color & 1) != 0)
                {
                    low |= 1UL << bit;
                }

                if ((color & 2) != 0)
                {
                    high |= 1UL << bit;
                }
            }
        }

        return new TileImageKey(low, high, false, false);
    }

    /// <summary>A tile whose eight rows are all <paramref name="row"/>.</summary>
    public static TileImageKey Rows(string row) => Tile(row, row, row, row, row, row, row, row);

    /// <summary>A full tile of the colour <paramref name="color"/>.</summary>
    public static TileImageKey Solid(int color)
        => Rows(new string((char)('0' + color), 8));

    /// <summary>The empty tile.</summary>
    public static TileImageKey Blank => Tile();

    /// <summary>A pattern of <paramref name="width"/> × <paramref name="height"/> cells.</summary>
    /// <param name="width">Grid width.</param>
    /// <param name="height">Grid height.</param>
    /// <param name="cells">Cells row by row; a null entry is an empty cell.</param>
    public static EnginePattern Pattern(int width, int height, params TileImageKey?[] cells)
    {
        ArgumentNullException.ThrowIfNull(cells);
        ImageRegistry registry = new();
        TileImage?[] images = new TileImage?[width * height];
        for (int n = 0; n < images.Length && n < cells.Length; n++)
        {
            images[n] = cells[n] is { } key ? registry.Intern(key) : null;
        }

        return new EnginePattern(width, height, images, permanent: true);
    }

    /// <summary>A one-cell pattern carrying <paramref name="tile"/>.</summary>
    public static EnginePattern OneTile(TileImageKey tile) => Pattern(1, 1, tile);

    /// <summary>
    /// A 3D object with the defaults of REQ-E3D-453 and the given depth and geometry.
    /// </summary>
    public static PatternInstance Object3D(
        int depth,
        Geo3DType geometry = Geo3DType.Default,
        bool solid = false,
        bool fullWarp = false,
        bool flip = false,
        bool antiShadow = false,
        int emptyColor = 0)
        => new()
        {
            Depth = depth,
            Geometry = geometry,
            Solid = solid,
            FullWarp = fullWarp,
            Flip = flip,
            AntiShadow = antiShadow,
            EmptyColor = emptyColor,
        };

    /// <summary>The texture of a 3D object of <paramref name="pattern"/> (REQ-E3D-426).</summary>
    public static ObjectTexture TextureOf(EnginePattern pattern, PatternInstance instance)
        => ObjectTexture.For(pattern, instance.Texture, NesPalette.Color(0x0F));

    /// <summary>Builds the voxel fields of a 3D object and reports its effective depth.</summary>
    public static TileVoxels[] Fields(
        EnginePattern pattern,
        PatternInstance instance,
        out int effectiveDepth,
        RenderAlgorithm algorithm = RenderAlgorithm.Greedy)
        => VoxelConstruction.Build(
            pattern,
            TextureOf(pattern, instance),
            instance,
            algorithm,
            out effectiveDepth);

    /// <summary>Builds the mesh of one tile image of a 3D object.</summary>
    public static TileMesh Mesh(
        EnginePattern pattern,
        PatternInstance instance,
        RenderAlgorithm algorithm = RenderAlgorithm.Greedy,
        int imageIndex = 0)
    {
        ObjectTexture texture = TextureOf(pattern, instance);
        TileVoxels[] fields = VoxelConstruction.Build(pattern, texture, instance, algorithm, out _);
        if (algorithm == RenderAlgorithm.Marching)
        {
            PatternVoxels voxels = PatternVoxels.Create(pattern, texture, fields);
            return MarchingMesher.Build(voxels, imageIndex, ObjectTexture.EmptyColor(instance.EmptyColor));
        }

        return GreedyMesher.Build(
            fields[imageIndex],
            texture,
            algorithm == RenderAlgorithm.GreedyMulticolor);
    }

    /// <summary>The mesh of a one-tile pattern, the usual case of §5.</summary>
    public static TileMesh MeshOf(
        TileImageKey tile,
        PatternInstance instance,
        RenderAlgorithm algorithm = RenderAlgorithm.Greedy)
        => Mesh(OneTile(tile), instance, algorithm);

    /// <summary>Number of rectangles of a GREEDY mesh: each one is four vertices, two triangles.</summary>
    public static int Rectangles(TileMesh mesh) => mesh.TriangleCount / 2;

    /// <summary>
    /// The vertices carrying the normal <paramref name="normal"/>, as (position, uv) pairs.
    /// Vertices are never shared, so a GREEDY rectangle contributes exactly four of them.
    /// </summary>
    public static List<(Vec3 Position, Vec2 Uv)> WithNormal(TileMesh mesh, Vec3 normal)
    {
        ArgumentNullException.ThrowIfNull(mesh);
        List<(Vec3, Vec2)> found = [];
        foreach (MeshVertex vertex in mesh.Vertices)
        {
            if (vertex.Normal == normal)
            {
                found.Add((vertex.Position, vertex.Uv));
            }
        }

        return found;
    }

    /// <summary>Number of rectangles whose normal is <paramref name="normal"/>.</summary>
    public static int RectanglesWithNormal(TileMesh mesh, Vec3 normal)
        => WithNormal(mesh, normal).Count / 4;

    /// <summary>The six unit normals, in the listing order of §5.</summary>
    public static Vec3 MinusX => new(-1f, 0f, 0f);

    /// <summary>The +X normal.</summary>
    public static Vec3 PlusX => new(1f, 0f, 0f);

    /// <summary>The −Y normal.</summary>
    public static Vec3 MinusY => new(0f, -1f, 0f);

    /// <summary>The +Y normal.</summary>
    public static Vec3 PlusY => new(0f, 1f, 0f);

    /// <summary>The −Z normal.</summary>
    public static Vec3 MinusZ => new(0f, 0f, -1f);

    /// <summary>The +Z normal.</summary>
    public static Vec3 PlusZ => new(0f, 0f, 1f);

    /// <summary>
    /// Asserts that the rectangle of normal <paramref name="normal"/> is exactly the four
    /// (position, uv) pairs given, in the order the vectors of §5 use: BL, TL, BR, TR. The
    /// comparison ignores the order of the vertices (REQ-E3D-523).
    /// </summary>
    public static void AssertRectangle(
        TileMesh mesh,
        Vec3 normal,
        params (float X, float Y, float Z, float U, float V)[] corners)
    {
        ArgumentNullException.ThrowIfNull(corners);
        List<(Vec3 Position, Vec2 Uv)> actual = WithNormal(mesh, normal);
        Assert.Equal(4, actual.Count);

        foreach ((float x, float y, float z, float u, float v) in corners)
        {
            Vec3 position = new(x, y, z);
            Vec2 uv = new(u, v);
            Assert.True(
                actual.Exists(pair => Close(pair.Position, position) && Close(pair.Uv, uv)),
                string.Create(
                    CultureInfo.InvariantCulture,
                    $"no vertex {position} uv {uv} with normal {normal}; got {Describe(actual)}"));
        }
    }

    /// <summary>
    /// The voxels of one line of a cylindrical section, as the vectors of §5.3 list them:
    /// <c>(i, k)</c> and the texel column for the V types, <c>(j, k)</c> and the texel row for
    /// the H types.
    /// </summary>
    public static List<(int Line, int Depth, int Texel)> Section(TileVoxels field, int line, bool vertical)
    {
        ArgumentNullException.ThrowIfNull(field);
        List<(int, int, int)> found = [];
        for (int i = 0; i < TileVoxels.SizeX; i++)
        {
            for (int j = 0; j < TileVoxels.SizeY; j++)
            {
                for (int k = 0; k < field.Depth; k++)
                {
                    if (!field.TryTexel(i, j, k, TileVoxels.AxisX, out int tx, out int ty))
                    {
                        continue;
                    }

                    if (vertical)
                    {
                        // V types keep the whole section in the row of the line.
                        Assert.Equal(7 - (line & 7), j);
                        found.Add((i, k, tx));
                    }
                    else
                    {
                        Assert.Equal(line & 7, i);
                        found.Add((j, k, ty));
                    }
                }
            }
        }

        found.Sort();
        return found;
    }

    /// <summary>Formats the expected voxel list of a §5.3 vector.</summary>
    public static List<(int Line, int Depth, int Texel)> Voxels(
        params (int Line, int Depth, int Texel)[] voxels)
    {
        ArgumentNullException.ThrowIfNull(voxels);
        List<(int, int, int)> sorted = [.. voxels];
        sorted.Sort();
        return sorted;
    }

    /// <summary>Asserts that a reference is not null and returns it.</summary>
    public static T Present<T>(T? value)
        where T : class
    {
        Assert.NotNull(value);
        return value;
    }

    /// <summary>Asserts that a nullable value is set and returns it.</summary>
    public static T Present<T>(T? value)
        where T : struct
    {
        Assert.NotNull(value);
        return value.Value;
    }

    /// <summary>True when the two positions agree to 10⁻³ (REQ-E3D-523).</summary>
    public static bool Close(Vec3 a, Vec3 b)
        => MathF.Abs(a.X - b.X) < 1e-3f && MathF.Abs(a.Y - b.Y) < 1e-3f && MathF.Abs(a.Z - b.Z) < 1e-3f;

    /// <summary>True when the two texture coordinates agree to 10⁻³ texel (REQ-E3D-523).</summary>
    public static bool Close(Vec2 a, Vec2 b)
        => MathF.Abs(a.X - b.X) < 1e-3f && MathF.Abs(a.Y - b.Y) < 1e-3f;

    private static string Describe(List<(Vec3 Position, Vec2 Uv)> vertices)
        => string.Join(", ", vertices.ConvertAll(pair => $"{pair.Position} {pair.Uv}"));
}
