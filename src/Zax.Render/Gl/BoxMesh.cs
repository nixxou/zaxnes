using Zax.Engine3D;

namespace Zax.Render.Gl;

/// <summary>
/// The unit box the backdrop of SPEC-06 REQ-SCN-048 is drawn with: a <see cref="TileMesh"/> so
/// that it goes through the same upload path and the same shaders as the tiles.
/// </summary>
internal static class BoxMesh
{
    /// <summary>
    /// The box <c>[−0.5, 0.5]³</c> in the scene frame, six quads with outward normals, wound as
    /// REQ-E3D-505 requires.
    /// </summary>
    public static TileMesh Unit { get; } = Build();

    private static TileMesh Build()
    {
        List<MeshVertex> vertices = [];
        List<int> triangles = [];

        // Faces given as (normal, bottom-left, top-left, top-right, bottom-right) with
        // cross(TL − BL, TR − BL) along the normal.
        AddFace(vertices, triangles, new Vec3(0f, 0f, -1f), (-1, -1, -1), (-1, 1, -1), (1, 1, -1), (1, -1, -1));
        AddFace(vertices, triangles, new Vec3(0f, 0f, 1f), (1, -1, 1), (1, 1, 1), (-1, 1, 1), (-1, -1, 1));
        AddFace(vertices, triangles, new Vec3(-1f, 0f, 0f), (-1, -1, 1), (-1, 1, 1), (-1, 1, -1), (-1, -1, -1));
        AddFace(vertices, triangles, new Vec3(1f, 0f, 0f), (1, -1, -1), (1, 1, -1), (1, 1, 1), (1, -1, 1));
        AddFace(vertices, triangles, new Vec3(0f, 1f, 0f), (-1, 1, -1), (-1, 1, 1), (1, 1, 1), (1, 1, -1));
        AddFace(vertices, triangles, new Vec3(0f, -1f, 0f), (-1, -1, 1), (-1, -1, -1), (1, -1, -1), (1, -1, 1));

        return new TileMesh(vertices, triangles);
    }

    private static void AddFace(
        List<MeshVertex> vertices,
        List<int> triangles,
        Vec3 normal,
        (int X, int Y, int Z) bottomLeft,
        (int X, int Y, int Z) topLeft,
        (int X, int Y, int Z) topRight,
        (int X, int Y, int Z) bottomRight)
    {
        int first = vertices.Count;
        vertices.Add(new MeshVertex(Half(bottomLeft), normal, new Vec2(0f, 0f)));
        vertices.Add(new MeshVertex(Half(topLeft), normal, new Vec2(0f, 1f)));
        vertices.Add(new MeshVertex(Half(topRight), normal, new Vec2(1f, 1f)));
        vertices.Add(new MeshVertex(Half(bottomRight), normal, new Vec2(1f, 0f)));
        triangles.AddRange([first, first + 1, first + 2, first, first + 2, first + 3]);
    }

    private static Vec3 Half((int X, int Y, int Z) corner)
        => new(corner.X * 0.5f, corner.Y * 0.5f, corner.Z * 0.5f);
}
