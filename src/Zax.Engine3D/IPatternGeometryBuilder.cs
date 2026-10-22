namespace Zax.Engine3D;

/// <summary>
/// How far a mass build of geometry has got (REQ-E3D-362, SPEC-04B REQ-E3D-600).
/// </summary>
/// <param name="Built">3D objects whose voxel field and meshes are computed.</param>
/// <param name="Transferred">3D objects whose meshes have reached the renderer.</param>
/// <param name="ToBuild">3D objects the current build has to produce; 0 outside a build.</param>
public readonly record struct GeometryProgress(int Built, int Transferred, int ToBuild)
{
    /// <summary>
    /// The loading progress <c>(built + transferred) / (2 × toBuild)</c>, and 1 outside a
    /// build or when there is nothing to build (REQ-E3D-362).
    /// </summary>
    public float Value => ToBuild <= 0
        ? 1f
        : Math.Clamp((Built + Transferred) / (2f * ToBuild), 0f, 1f);
}

/// <summary>
/// The geometry side of the engine, which lot L4b implements against SPEC-04B: the conversion
/// pipeline only decides <b>when</b> a pattern's geometry is (re)built and reads back whether
/// it is ready.
/// </summary>
/// <remarks>
/// The pipeline calls <see cref="Build"/> at profile load, when a pattern is learned
/// (REQ-E3D-161) and for every pattern when the global mesh algorithm changes
/// (REQ-E3D-340), always <b>after</b> the <c>BuildPattern</c> hooks (REQ-E3D-322). A shape
/// whose geometry is not ready stays in the frame and is seen by scripts but is not drawn
/// (REQ-E3D-361, SPEC-04B REQ-E3D-601).
/// </remarks>
public interface IPatternGeometryBuilder
{
    /// <summary>Builds, or rebuilds, the geometry of every instance of a pattern.</summary>
    void Build(EnginePattern pattern);

    /// <summary>Frees the meshes and the texture space of a pattern (SPEC-04B REQ-E3D-603).</summary>
    void Release(EnginePattern pattern);

    /// <summary>Whether the meshes of a pattern instance can be drawn (REQ-E3D-361).</summary>
    bool IsReady(PatternInstance instance);

    /// <summary>
    /// The mesh of one tile image of a pattern instance (SPEC-04B REQ-E3D-640), or null when
    /// it is not available.
    /// </summary>
    TileMesh? MeshOf(PatternInstance instance, TileImage image);

    /// <summary>
    /// The mesh of <paramref name="image"/> for <paramref name="instance"/> with the walls of
    /// <paramref name="sealedSides"/> left out, the rendering variant of Q-067. A mask of 0
    /// gives the same mesh as <see cref="MeshOf(PatternInstance, TileImage)"/>.
    /// </summary>
    /// <param name="instance">The 3D object.</param>
    /// <param name="image">The tile image.</param>
    /// <param name="sealedSides">A mask of <see cref="TileSides"/>.</param>
    TileMesh? MeshOf(PatternInstance instance, TileImage image, int sealedSides);

    /// <summary>
    /// The lateral sides of <paramref name="image"/> whose plane is entirely occupied: the
    /// only sides a neighbouring tile may seal (Q-067).
    /// </summary>
    /// <param name="instance">The 3D object.</param>
    /// <param name="image">The tile image.</param>
    int FullSidesOf(PatternInstance instance, TileImage image);

    /// <summary>
    /// The resolved texture of a pattern instance (SPEC-04B REQ-E3D-426, REQ-E3D-641), or null
    /// when its geometry has not been built.
    /// </summary>
    ObjectTexture? TextureOf(PatternInstance instance);

    /// <summary>How far the current mass build has got (REQ-E3D-362).</summary>
    GeometryProgress Progress { get; }

    /// <summary>
    /// Announces a mass build of <paramref name="objectCount"/> 3D objects, so that the
    /// loading progress of REQ-E3D-600 can be reported. A builder that reports no progress may
    /// ignore it.
    /// </summary>
    void BeginMassBuild(int objectCount)
    {
    }
}

/// <summary>
/// A geometry builder that builds nothing and declares everything ready. It lets the
/// conversion pipeline run and be tested on its own, before lot L4b.
/// </summary>
public sealed class NullPatternGeometryBuilder : IPatternGeometryBuilder
{
    /// <summary>The single instance.</summary>
    public static NullPatternGeometryBuilder Instance { get; } = new();

    private NullPatternGeometryBuilder()
    {
    }

    /// <inheritdoc/>
    public void Build(EnginePattern pattern)
    {
    }

    /// <inheritdoc/>
    public void Release(EnginePattern pattern)
    {
    }

    /// <inheritdoc/>
    public bool IsReady(PatternInstance instance) => true;

    /// <inheritdoc/>
    public TileMesh? MeshOf(PatternInstance instance, TileImage image) => null;

    /// <inheritdoc/>
    public TileMesh? MeshOf(PatternInstance instance, TileImage image, int sealedSides) => null;

    /// <inheritdoc/>
    public int FullSidesOf(PatternInstance instance, TileImage image) => 0;

    /// <inheritdoc/>
    public ObjectTexture? TextureOf(PatternInstance instance) => null;

    /// <inheritdoc/>
    public GeometryProgress Progress => new(0, 0, 0);
}
