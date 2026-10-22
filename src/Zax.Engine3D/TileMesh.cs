namespace Zax.Engine3D;

/// <summary>
/// One vertex of a tile mesh (SPEC-04B §4.2). Colour is <b>not</b> carried by the vertices: it
/// is resolved at render time from the frame palette (REQ-E3D-560, REQ-E3D-565, REQ-E3D-640).
/// </summary>
/// <param name="Position">Position in the tile's local frame (REQ-E3D-504, REQ-E3D-522).</param>
/// <param name="Normal">Unit normal (REQ-E3D-505); <c>(0,0,0)</c> on a zero-area triangle.</param>
/// <param name="Uv">Texture coordinates in texels of the object's texture (REQ-E3D-506).</param>
public readonly record struct MeshVertex(Vec3 Position, Vec3 Normal, Vec2 Uv);

/// <summary>
/// The platform-independent description of the mesh of one tile (SPEC-04B REQ-E3D-640, §4.2):
/// a vertex list, a triangle index list, one single submesh, triangle topology and a local
/// bounding box.
/// </summary>
/// <remarks>
/// A mesh is immutable once built, which is what lets the same instance be shared by every
/// occurrence of a tile image in a shape (REQ-E3D-540) and reused across frames
/// (REQ-E3D-602).
/// </remarks>
public sealed class TileMesh
{
    private readonly MeshVertex[] _vertices;
    private readonly int[] _triangles;

    /// <summary>Creates a mesh from its vertices and triangle indices.</summary>
    /// <param name="vertices">The vertices.</param>
    /// <param name="triangles">Triangle indices, three per triangle.</param>
    /// <exception cref="ArgumentException">The index count is not a multiple of three.</exception>
    public TileMesh(IEnumerable<MeshVertex> vertices, IEnumerable<int> triangles)
    {
        ArgumentNullException.ThrowIfNull(vertices);
        ArgumentNullException.ThrowIfNull(triangles);

        _vertices = [.. vertices];
        _triangles = [.. triangles];
        if (_triangles.Length % 3 != 0)
        {
            throw new ArgumentException("A triangle list needs three indices per triangle.", nameof(triangles));
        }

        if (_vertices.Length == 0)
        {
            BoundsMin = Vec3.Zero;
            BoundsMax = Vec3.Zero;
            return;
        }

        Vec3 min = _vertices[0].Position;
        Vec3 max = min;
        foreach (MeshVertex vertex in _vertices)
        {
            Vec3 p = vertex.Position;
            min = new Vec3(MathF.Min(min.X, p.X), MathF.Min(min.Y, p.Y), MathF.Min(min.Z, p.Z));
            max = new Vec3(MathF.Max(max.X, p.X), MathF.Max(max.Y, p.Y), MathF.Max(max.Z, p.Z));
        }

        BoundsMin = min;
        BoundsMax = max;
    }

    /// <summary>The mesh of a tile with no geometry (<c>sizeZ = 0</c>, REQ-E3D-420).</summary>
    public static TileMesh Empty { get; } = new([], []);

    /// <summary>Number of submeshes; always one (REQ-E3D-640).</summary>
    public const int SubmeshCount = 1;

    /// <summary>The vertices.</summary>
    public ReadOnlySpan<MeshVertex> Vertices => _vertices;

    /// <summary>Triangle indices, three per triangle, oriented per REQ-E3D-505.</summary>
    public ReadOnlySpan<int> Triangles => _triangles;

    /// <summary>Number of vertices.</summary>
    public int VertexCount => _vertices.Length;

    /// <summary>Number of triangles.</summary>
    public int TriangleCount => _triangles.Length / 3;

    /// <summary>True when the mesh draws nothing.</summary>
    public bool IsEmpty => _triangles.Length == 0;

    /// <summary>Lowest corner of the local bounding box.</summary>
    public Vec3 BoundsMin { get; }

    /// <summary>Highest corner of the local bounding box.</summary>
    public Vec3 BoundsMax { get; }

    /// <inheritdoc/>
    public override string ToString()
        => $"mesh {VertexCount} vertices, {TriangleCount} triangles";
}

/// <summary>
/// Accumulates the vertices and triangles of a tile mesh while it is being built, with the
/// orientation rule of REQ-E3D-505 applied once per face.
/// </summary>
internal sealed class TileMeshBuilder
{
    private readonly List<MeshVertex> _vertices = [];
    private readonly List<int> _triangles = [];

    /// <summary>Number of triangles pushed so far.</summary>
    public int TriangleCount => _triangles.Count / 3;

    /// <summary>Forgets everything, so the builder can be reused.</summary>
    public void Clear()
    {
        _vertices.Clear();
        _triangles.Clear();
    }

    /// <summary>
    /// Pushes a rectangle given by its four corners in the order bottom-left, top-left,
    /// top-right, bottom-right, as two triangles oriented along <paramref name="normal"/>.
    /// </summary>
    public void AddQuad(
        Vec3 bottomLeft,
        Vec3 topLeft,
        Vec3 topRight,
        Vec3 bottomRight,
        Vec2 uvBottomLeft,
        Vec2 uvTopLeft,
        Vec2 uvTopRight,
        Vec2 uvBottomRight,
        Vec3 normal)
    {
        int first = _vertices.Count;
        _vertices.Add(new MeshVertex(bottomLeft, normal, uvBottomLeft));
        _vertices.Add(new MeshVertex(topLeft, normal, uvTopLeft));
        _vertices.Add(new MeshVertex(topRight, normal, uvTopRight));
        _vertices.Add(new MeshVertex(bottomRight, normal, uvBottomRight));

        // (B − A) x (C − A) must point the way the normal does (REQ-E3D-505).
        bool forward = Dot(Cross(topLeft - bottomLeft, topRight - bottomLeft), normal) >= 0f;
        if (forward)
        {
            PushTriangle(first, first + 1, first + 2);
            PushTriangle(first, first + 2, first + 3);
        }
        else
        {
            PushTriangle(first, first + 2, first + 1);
            PushTriangle(first, first + 3, first + 2);
        }
    }

    /// <summary>
    /// Pushes a triangle with three vertices of its own, carrying the flat normal of
    /// REQ-E3D-522, oriented so that the normal points along <paramref name="outward"/>. A
    /// zero-area triangle gets the normal <c>(0, 0, 0)</c>.
    /// </summary>
    public void AddTriangle(Vec3 a, Vec3 b, Vec3 c, Vec2 uv, Vec3 outward)
    {
        Vec3 cross = Cross(b - a, c - a);
        bool flip = Dot(cross, outward) < 0f;
        if (flip)
        {
            (b, c) = (c, b);
            cross = cross * -1f;
        }

        float length = MathF.Sqrt(cross.SquaredLength);
        Vec3 normal = length > 0f ? cross * (1f / length) : Vec3.Zero;

        int first = _vertices.Count;
        _vertices.Add(new MeshVertex(a, normal, uv));
        _vertices.Add(new MeshVertex(b, normal, uv));
        _vertices.Add(new MeshVertex(c, normal, uv));
        PushTriangle(first, first + 1, first + 2);
    }

    /// <summary>Freezes the accumulated data into a mesh.</summary>
    public TileMesh Build() => _triangles.Count == 0 ? TileMesh.Empty : new TileMesh(_vertices, _triangles);

    private void PushTriangle(int a, int b, int c)
    {
        _triangles.Add(a);
        _triangles.Add(b);
        _triangles.Add(c);
    }

    private static Vec3 Cross(Vec3 a, Vec3 b) => new(
        (a.Y * b.Z) - (a.Z * b.Y),
        (a.Z * b.X) - (a.X * b.Z),
        (a.X * b.Y) - (a.Y * b.X));

    private static float Dot(Vec3 a, Vec3 b) => (a.X * b.X) + (a.Y * b.Y) + (a.Z * b.Z);
}
