// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// The geometry side of the engine (SPEC-04B §3.2 to §3.7): it voxelises every 3D object of a
/// pattern, meshes each of its distinct tile images and keeps the result until the pattern is
/// released.
/// </summary>
/// <remarks>
/// <para>
/// Pure .NET data: nothing here knows about OpenGL (REQ-ARCH-002). "Transferring" a mesh to the
/// renderer is therefore just marking it ready; <see cref="DeferTransfer"/> lets a host — or a
/// test — keep the two phases of REQ-E3D-600 apart, which is what the loading progress
/// measures.
/// </para>
/// <para>
/// Building is the only thing that touches a mesh. A change of palette, position, transform,
/// alpha, layer, shadow tag or deformation never rebuilds anything (REQ-E3D-602);
/// <see cref="BuildCount"/> is there so a test can prove it.
/// </para>
/// </remarks>
public sealed class GeometryBuilder : IPatternGeometryBuilder
{
    private readonly Dictionary<PatternInstance, ObjectGeometry> _geometry = [];
    private readonly List<PatternInstance> _pending = [];

    private int _built;
    private int _transferred;
    private int _toBuild;

    /// <summary>Creates a builder reading <paramref name="settings"/> for the global algorithm.</summary>
    /// <param name="settings">
    /// The pipeline settings, so that the global mesh algorithm stays the one the engine uses;
    /// a fresh set of factory settings when omitted.
    /// </param>
    public GeometryBuilder(ConversionSettings? settings = null)
    {
        Settings = settings ?? new ConversionSettings();
    }

    /// <summary>The settings the builder reads.</summary>
    public ConversionSettings Settings { get; }

    /// <summary>
    /// The global mesh algorithm an object with <c>rm = 0</c> takes (REQ-E3D-500). Changing it
    /// does not rebuild by itself: <see cref="ConversionEngine.SetRenderAlgorithm"/> does.
    /// </summary>
    public RenderAlgorithm GlobalAlgorithm => Settings.RenderAlgorithm;

    /// <summary>
    /// The universal background colour a direct texture gives to its value-0 texels. It is
    /// read when the texture is built and does not follow later palette changes
    /// (REQ-E3D-562, Q-017).
    /// </summary>
    public Rgb24 BackgroundColor { get; set; } = NesPalette.Color(0x0F);

    /// <summary>
    /// When true, <see cref="Build"/> only computes the meshes and <see cref="Commit"/> hands
    /// them over; when false (the default) a build does both at once.
    /// </summary>
    public bool DeferTransfer { get; set; }

    /// <summary>Number of 3D objects whose geometry was built since this builder was created.</summary>
    public int BuildCount { get; private set; }

    /// <summary>The 3D objects that currently hold geometry.</summary>
    public int ObjectCount => _geometry.Count;

    /// <inheritdoc/>
    public GeometryProgress Progress => new(_built, _transferred, _toBuild);

    /// <summary>
    /// Announces a mass build of <paramref name="objectCount"/> 3D objects, which resets the
    /// loading progress (REQ-E3D-600, REQ-E3D-362).
    /// </summary>
    public void BeginMassBuild(int objectCount)
    {
        _toBuild = Math.Max(0, objectCount);
        _built = 0;
        _transferred = 0;
    }

    /// <inheritdoc/>
    public void Build(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);

        foreach (PatternInstance instance in pattern.Instances)
        {
            BuildObject(pattern, instance);
        }

        if (!DeferTransfer)
        {
            Commit();
        }
    }

    /// <summary>
    /// Hands the meshes built since the last call over to the renderer and returns how many
    /// objects that was (REQ-E3D-600, second half of the progress).
    /// </summary>
    public int Commit()
    {
        int count = 0;
        foreach (PatternInstance instance in _pending)
        {
            if (_geometry.TryGetValue(instance, out ObjectGeometry? geometry) && !geometry.Ready)
            {
                geometry.Ready = true;
                _transferred++;
                count++;
            }
        }

        _pending.Clear();
        return count;
    }

    /// <inheritdoc/>
    public void Release(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        foreach (PatternInstance instance in pattern.Instances)
        {
            _geometry.Remove(instance);
            _pending.Remove(instance);
        }
    }

    /// <summary>Frees every mesh and texture (profile unloaded).</summary>
    public void Clear()
    {
        _geometry.Clear();
        _pending.Clear();
        _built = 0;
        _transferred = 0;
        _toBuild = 0;
    }

    /// <inheritdoc/>
    public bool IsReady(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        return _geometry.TryGetValue(instance, out ObjectGeometry? geometry) && geometry.Ready;
    }

    /// <summary>The geometry of a 3D object, or null when it has none yet.</summary>
    public ObjectGeometry? Of(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        return _geometry.GetValueOrDefault(instance);
    }

    /// <inheritdoc/>
    public TileMesh? MeshOf(PatternInstance instance, TileImage image)
        => MeshOf(instance, image, sealedSides: 0);

    /// <inheritdoc/>
    public TileMesh? MeshOf(PatternInstance instance, TileImage image, int sealedSides)
    {
        ArgumentNullException.ThrowIfNull(instance);
        ArgumentNullException.ThrowIfNull(image);
        return _geometry.TryGetValue(instance, out ObjectGeometry? geometry)
            ? geometry.Mesh(image, sealedSides)
            : null;
    }

    /// <inheritdoc/>
    public int FullSidesOf(PatternInstance instance, TileImage image)
    {
        ArgumentNullException.ThrowIfNull(instance);
        ArgumentNullException.ThrowIfNull(image);
        return _geometry.TryGetValue(instance, out ObjectGeometry? geometry)
            ? geometry.FullSides(image)
            : 0;
    }

    /// <inheritdoc/>
    public ObjectTexture? TextureOf(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        return _geometry.GetValueOrDefault(instance)?.Texture;
    }

    private void BuildObject(EnginePattern pattern, PatternInstance instance)
    {
        RenderAlgorithm algorithm = instance.EffectiveAlgorithm(GlobalAlgorithm);

        // A rebuild starts from the depth the previous build was asked for, unless something
        // (a BuildPattern hook, REQ-E3D-322) has moved sizeZ since. Without this a cylinder
        // would shrink a little at every rebuild, because REQ-E3D-452 writes D' back into
        // sizeZ. See Q-013.
        int depth = instance.Depth;
        if (_geometry.TryGetValue(instance, out ObjectGeometry? previous)
            && depth == previous.EffectiveDepth)
        {
            depth = previous.RequestedDepth;
        }

        int requested = Math.Clamp(depth, 0, TileVoxels.MaxDepth);
        instance.Depth = requested;

        ObjectTexture texture = ObjectTexture.For(pattern, instance.Texture, BackgroundColor);
        TileVoxels[] fields = VoxelConstruction.Build(
            pattern,
            texture,
            instance,
            algorithm,
            out int effectiveDepth);

        TileMesh[] meshes = new TileMesh[fields.Length];
        if (algorithm == RenderAlgorithm.Marching)
        {
            PatternVoxels voxels = PatternVoxels.Create(pattern, texture, fields);
            int empty = ObjectTexture.EmptyColor(instance.EmptyColor);
            for (int n = 0; n < meshes.Length; n++)
            {
                meshes[n] = MarchingMesher.Build(voxels, n, empty);
            }
        }
        else
        {
            bool multicolor = algorithm == RenderAlgorithm.GreedyMulticolor;
            for (int n = 0; n < meshes.Length; n++)
            {
                meshes[n] = GreedyMesher.Build(fields[n], texture, multicolor);
            }
        }

        // REQ-E3D-452: from now on placement, the MARCHING scale correction, the deformation
        // and the scripts see D'.
        instance.Depth = effectiveDepth;

        _geometry[instance] = new ObjectGeometry(
            pattern,
            texture,
            algorithm,
            requested,
            requested,
            effectiveDepth,
            meshes,
            fields,
            algorithm == RenderAlgorithm.GreedyMulticolor);

        _pending.Add(instance);
        _built++;
        BuildCount++;
    }
}
