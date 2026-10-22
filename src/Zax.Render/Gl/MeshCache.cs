using Zax.Engine3D;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// Uploads the abstract tile meshes of SPEC-04B REQ-E3D-640 to the GPU and keeps them, keyed by
/// mesh identity.
/// </summary>
/// <remarks>
/// <para>
/// A mesh is immutable once built and a rebuild produces a new <see cref="TileMesh"/>
/// (REQ-E3D-542, REQ-E3D-602), so identity is the right key: nothing is re-uploaded while the
/// engine does not rebuild, whatever a script changes on an instance. Entries not used for
/// <see cref="RetentionFrames"/> rendered frames are released, which covers temporary patterns
/// (REQ-E3D-603).
/// </para>
/// <para>
/// Vertices are interleaved: position (3 floats), normal (3 floats), texel coordinates
/// (2 floats). Indices are 32 bits, as REQ-E3D-524 allows meshes over 65 535 vertices.
/// </para>
/// </remarks>
internal sealed class MeshCache : IDisposable
{
    /// <summary>Rendered frames an unused mesh is kept for.</summary>
    public const int RetentionFrames = 600;

    private const int FloatsPerVertex = 8;

    private readonly GL _gl;
    private readonly Dictionary<TileMesh, Entry> _entries = new(ReferenceComparer.Instance);
    private readonly List<TileMesh> _expired = [];
    private long _frame;

    /// <summary>Creates the cache for the current context.</summary>
    public MeshCache(GL gl)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
    }

    /// <summary>Number of meshes currently resident on the GPU.</summary>
    public int Count => _entries.Count;

    /// <summary>Number of uploads performed since the cache was created.</summary>
    public int Uploads { get; private set; }

    /// <summary>Starts a rendered frame, for the retention bookkeeping.</summary>
    public void BeginFrame() => _frame++;

    /// <summary>
    /// The vertex array of <paramref name="mesh"/>, uploading it on first use, or null when the
    /// mesh draws nothing.
    /// </summary>
    public GpuMesh? Get(TileMesh mesh)
    {
        ArgumentNullException.ThrowIfNull(mesh);
        if (mesh.IsEmpty)
        {
            return null;
        }

        if (_entries.TryGetValue(mesh, out Entry entry))
        {
            entry.LastUsed = _frame;
            _entries[mesh] = entry;
            return new GpuMesh(entry.VertexArray, entry.IndexCount);
        }

        Entry created = Upload(mesh);
        _entries[mesh] = created;
        Uploads++;
        return new GpuMesh(created.VertexArray, created.IndexCount);
    }

    /// <summary>Releases every mesh whose last use is older than the retention window.</summary>
    public void Collect()
    {
        _expired.Clear();
        foreach ((TileMesh mesh, Entry entry) in _entries)
        {
            if (_frame - entry.LastUsed > RetentionFrames)
            {
                _expired.Add(mesh);
            }
        }

        foreach (TileMesh mesh in _expired)
        {
            Release(_entries[mesh]);
            _entries.Remove(mesh);
        }

        _expired.Clear();
    }

    /// <summary>Releases everything, for instance when the global mesh algorithm changes.</summary>
    public void Clear()
    {
        foreach (Entry entry in _entries.Values)
        {
            Release(entry);
        }

        _entries.Clear();
    }

    /// <inheritdoc/>
    public void Dispose() => Clear();

    private unsafe Entry Upload(TileMesh mesh)
    {
        ReadOnlySpan<MeshVertex> vertices = mesh.Vertices;
        ReadOnlySpan<int> triangles = mesh.Triangles;

        float[] buffer = new float[vertices.Length * FloatsPerVertex];
        for (int i = 0; i < vertices.Length; i++)
        {
            MeshVertex vertex = vertices[i];
            int at = i * FloatsPerVertex;
            buffer[at + 0] = vertex.Position.X;
            buffer[at + 1] = vertex.Position.Y;
            buffer[at + 2] = vertex.Position.Z;
            buffer[at + 3] = vertex.Normal.X;
            buffer[at + 4] = vertex.Normal.Y;
            buffer[at + 5] = vertex.Normal.Z;
            buffer[at + 6] = vertex.Uv.X;
            buffer[at + 7] = vertex.Uv.Y;
        }

        uint vao = _gl.GenVertexArray();
        _gl.BindVertexArray(vao);

        uint vbo = _gl.GenBuffer();
        _gl.BindBuffer(BufferTargetARB.ArrayBuffer, vbo);
        fixed (float* data = buffer)
        {
            _gl.BufferData(
                BufferTargetARB.ArrayBuffer,
                (nuint)(buffer.Length * sizeof(float)),
                data,
                BufferUsageARB.StaticDraw);
        }

        uint ebo = _gl.GenBuffer();
        _gl.BindBuffer(BufferTargetARB.ElementArrayBuffer, ebo);
        fixed (int* data = triangles)
        {
            _gl.BufferData(
                BufferTargetARB.ElementArrayBuffer,
                (nuint)(triangles.Length * sizeof(int)),
                data,
                BufferUsageARB.StaticDraw);
        }

        uint stride = FloatsPerVertex * sizeof(float);
        _gl.EnableVertexAttribArray(0);
        _gl.VertexAttribPointer(0, 3, VertexAttribPointerType.Float, false, stride, (void*)0);
        _gl.EnableVertexAttribArray(1);
        _gl.VertexAttribPointer(1, 3, VertexAttribPointerType.Float, false, stride, (void*)(3 * sizeof(float)));
        _gl.EnableVertexAttribArray(2);
        _gl.VertexAttribPointer(2, 2, VertexAttribPointerType.Float, false, stride, (void*)(6 * sizeof(float)));

        _gl.BindVertexArray(0);
        _gl.BindBuffer(BufferTargetARB.ArrayBuffer, 0);
        _gl.BindBuffer(BufferTargetARB.ElementArrayBuffer, 0);

        return new Entry
        {
            VertexArray = vao,
            VertexBuffer = vbo,
            IndexBuffer = ebo,
            IndexCount = (uint)triangles.Length,
            LastUsed = _frame,
        };
    }

    private void Release(Entry entry)
    {
        _gl.DeleteVertexArray(entry.VertexArray);
        _gl.DeleteBuffer(entry.VertexBuffer);
        _gl.DeleteBuffer(entry.IndexBuffer);
    }

    private struct Entry
    {
        public uint VertexArray;
        public uint VertexBuffer;
        public uint IndexBuffer;
        public uint IndexCount;
        public long LastUsed;
    }

    private sealed class ReferenceComparer : IEqualityComparer<TileMesh>
    {
        public static ReferenceComparer Instance { get; } = new();

        public bool Equals(TileMesh? x, TileMesh? y) => ReferenceEquals(x, y);

        public int GetHashCode(TileMesh obj) => System.Runtime.CompilerServices.RuntimeHelpers.GetHashCode(obj);
    }
}

/// <summary>A mesh ready to draw: its vertex array and its index count.</summary>
/// <param name="VertexArray">The vertex array object.</param>
/// <param name="IndexCount">Number of indices, three per triangle.</param>
internal readonly record struct GpuMesh(uint VertexArray, uint IndexCount);
