using Zax.Engine3D;

namespace Zax.Render.Gl;

/// <summary>
/// The unit box the backdrop of SPEC-06 REQ-SCN-048 is drawn with: a <see cref="TileMesh"/> so
/// that it goes through the same upload path and the same shaders as the tiles.
/// </summary>
internal static class BoxMesh
{
    /// <summary>
    /// Cells per side of a face of <see cref="Subdivided"/>: the floor AMD-043 REQ-SCN-068 sets
    /// for the backdrop, so that a bent face is a curve rather than a flat quad tilted in space.
    /// </summary>
    public const int BackdropCells = 32;

    /// <summary>
    /// The box <c>[−0.5, 0.5]³</c> in the scene frame, six quads with outward normals, wound as
    /// REQ-E3D-505 requires.
    /// </summary>
    public static TileMesh Unit { get; } = Build(1);

    /// <summary>
    /// The same box with <see cref="BackdropCells"/> × <see cref="BackdropCells"/> cells per
    /// face (REQ-SCN-068). The backdrop is 256 × 240 units across, large enough that the
    /// curvature has to be sampled inside a face and not only at its corners.
    /// </summary>
    public static TileMesh Subdivided { get; } = Build(BackdropCells);

    private static TileMesh Build(int cells)
    {
        List<MeshVertex> vertices = [];
        List<int> triangles = [];

        // Faces given as (normal, bottom-left, top-left, top-right, bottom-right) with
        // cross(TL − BL, TR − BL) along the normal.
        AddFace(vertices, triangles, cells, new Vec3(0f, 0f, -1f), (-1, -1, -1), (-1, 1, -1), (1, 1, -1), (1, -1, -1));
        AddFace(vertices, triangles, cells, new Vec3(0f, 0f, 1f), (1, -1, 1), (1, 1, 1), (-1, 1, 1), (-1, -1, 1));
        AddFace(vertices, triangles, cells, new Vec3(-1f, 0f, 0f), (-1, -1, 1), (-1, 1, 1), (-1, 1, -1), (-1, -1, -1));
        AddFace(vertices, triangles, cells, new Vec3(1f, 0f, 0f), (1, -1, -1), (1, 1, -1), (1, 1, 1), (1, -1, 1));
        AddFace(vertices, triangles, cells, new Vec3(0f, 1f, 0f), (-1, 1, -1), (-1, 1, 1), (1, 1, 1), (1, 1, -1));
        AddFace(vertices, triangles, cells, new Vec3(0f, -1f, 0f), (-1, -1, 1), (-1, -1, -1), (1, -1, -1), (1, -1, 1));

        return new TileMesh(vertices, triangles);
    }

    /// <summary>
    /// Adds one face cut into <paramref name="cells"/> × <paramref name="cells"/> quads. The
    /// grid is bilinear in the face's own corners, which on a flat face is exactly the face.
    /// </summary>
    private static void AddFace(
        List<MeshVertex> vertices,
        List<int> triangles,
        int cells,
        Vec3 normal,
        (int X, int Y, int Z) bottomLeft,
        (int X, int Y, int Z) topLeft,
        (int X, int Y, int Z) topRight,
        (int X, int Y, int Z) bottomRight)
    {
        int steps = Math.Max(1, cells);
        int first = vertices.Count;
        Vec3 bl = Half(bottomLeft);
        Vec3 tl = Half(topLeft);
        Vec3 tr = Half(topRight);
        Vec3 br = Half(bottomRight);

        for (int row = 0; row <= steps; row++)
        {
            float v = row / (float)steps;
            for (int column = 0; column <= steps; column++)
            {
                float u = column / (float)steps;
                Vec3 bottom = bl + ((br - bl) * u);
                Vec3 top = tl + ((tr - tl) * u);
                vertices.Add(new MeshVertex(bottom + ((top - bottom) * v), normal, new Vec2(u, v)));
            }
        }

        int stride = steps + 1;
        for (int row = 0; row < steps; row++)
        {
            for (int column = 0; column < steps; column++)
            {
                int corner = first + (row * stride) + column;
                triangles.AddRange(
                [
                    corner,
                    corner + stride,
                    corner + stride + 1,
                    corner,
                    corner + stride + 1,
                    corner + 1,
                ]);
            }
        }
    }

    private static Vec3 Half((int X, int Y, int Z) corner)
        => new(corner.X * 0.5f, corner.Y * 0.5f, corner.Z * 0.5f);
}
