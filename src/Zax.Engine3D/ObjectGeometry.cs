// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// The geometry of one 3D object: its resolved texture, the mesh of each distinct tile image of
/// its pattern, and the depths the construction used and produced (SPEC-04B §3.4, §3.7).
/// </summary>
/// <remarks>
/// Meshes depend only on the reference 3D object — its parameters at build time, its texture
/// and the tiles of its pattern (REQ-E3D-542). Nothing a script changes on a frame instance,
/// and no palette change, rebuilds them (REQ-E3D-565, REQ-E3D-602).
/// </remarks>
public sealed class ObjectGeometry
{
    private readonly TileMesh[] _meshes;
    private readonly int[] _meshOfImage;

    internal ObjectGeometry(
        EnginePattern pattern,
        ObjectTexture texture,
        RenderAlgorithm algorithm,
        int requestedDepth,
        int buildDepth,
        int effectiveDepth,
        TileMesh[] meshes,
        bool multicolor)
    {
        Pattern = pattern;
        Texture = texture;
        Algorithm = algorithm;
        RequestedDepth = requestedDepth;
        BuildDepth = buildDepth;
        EffectiveDepth = effectiveDepth;
        _meshes = meshes;
        Multicolor = multicolor;

        int highest = 0;
        foreach (TileImage image in pattern.Images)
        {
            highest = Math.Max(highest, image.Id);
        }

        _meshOfImage = new int[highest + 1];
        Array.Fill(_meshOfImage, -1);
        for (int n = 0; n < pattern.Images.Count; n++)
        {
            _meshOfImage[pattern.Images[n].Id] = n;
        }
    }

    /// <summary>The pattern the object belongs to.</summary>
    public EnginePattern Pattern { get; }

    /// <summary>The object's texture (REQ-E3D-426).</summary>
    public ObjectTexture Texture { get; }

    /// <summary>The mesh algorithm the meshes were built with (REQ-E3D-500).</summary>
    public RenderAlgorithm Algorithm { get; }

    /// <summary>
    /// The depth the build was <b>asked</b> for, before the cylinders replaced it by
    /// <see cref="EffectiveDepth"/>. A rebuild starts from this value again (Q-013).
    /// </summary>
    public int RequestedDepth { get; }

    /// <summary>
    /// The depth <c>D</c> the voxel field and the local centring of the meshes use
    /// (REQ-E3D-504, REQ-E3D-452).
    /// </summary>
    public int BuildDepth { get; }

    /// <summary>
    /// The depth <c>D'</c> placement, scale correction and deformation use: <c>D</c> for the
    /// non-cylindrical types, <c>max(B + 1)</c> over the segments otherwise (REQ-E3D-452).
    /// </summary>
    public int EffectiveDepth { get; }

    /// <summary>
    /// True once the meshes have reached the renderer; an object that is not ready is not
    /// drawn (REQ-E3D-543, REQ-E3D-601).
    /// </summary>
    public bool Ready { get; internal set; }

    /// <summary>
    /// Whether the texture of the object has room in the texture budget of REQ-E3D-564. False
    /// means the texture could not be allocated: the tiles of the object are not drawn until it
    /// can be (AMD-051 REQ-E3D-644), and nothing else about the object changes.
    /// </summary>
    public bool TextureHoused { get; internal set; } = true;

    /// <summary>Texels the object's texture occupies (REQ-E3D-564, expressed in texels).</summary>
    public long TextureTexels => (long)Texture.Width * Texture.Height;

    /// <summary>The meshes, in the order of <see cref="EnginePattern.Images"/>.</summary>
    public ReadOnlySpan<TileMesh> Meshes => _meshes;

    /// <summary>Total number of triangles over every tile mesh of the object.</summary>
    public int TriangleCount
    {
        get
        {
            int total = 0;
            foreach (TileMesh mesh in _meshes)
            {
                total += mesh.TriangleCount;
            }

            return total;
        }
    }

    /// <summary>True when the meshes were merged by colour (REQ-E3D-503).</summary>
    public bool Multicolor { get; }

    /// <summary>The mesh of <paramref name="image"/>, or null when the pattern has no such tile.</summary>
    public TileMesh? Mesh(TileImage image)
    {
        ArgumentNullException.ThrowIfNull(image);
        int index = Index(image);
        return index < 0 ? null : _meshes[index];
    }

    private int Index(TileImage image)
        => (uint)image.Id < (uint)_meshOfImage.Length ? _meshOfImage[image.Id] : -1;
}
