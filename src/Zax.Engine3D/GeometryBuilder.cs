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
/// <para>
/// Nothing here throws because an object could not be housed. A texture that does not fit in
/// the budget of REQ-E3D-564 leaves its object un-drawn and the rest of the scene intact
/// (AMD-051 REQ-E3D-644); an object whose construction has not finished is drawn at the first
/// frame that follows, and a diagnostic is reported if it waits for 600 emulated frames
/// (REQ-E3D-646). Both report through <see cref="Diagnostics"/> under one line budget.
/// </para>
/// </remarks>
public sealed class GeometryBuilder : IPatternGeometryBuilder
{
    /// <summary>
    /// Emulated frames an object may stay waiting before a diagnostic is reported
    /// (AMD-051 REQ-E3D-646): 600 frames, 10 s in NTSC.
    /// </summary>
    public const int WaitingDiagnosticFrames = 600;

    /// <summary>
    /// Diagnostic lines one game allows before the periodic summary takes over
    /// (AMD-051 REQ-E3D-644). Patterns are also created <b>during</b> play (REQ-E3D-008,
    /// REQ-E3D-453) and nothing bounds their number, so the per-object rule alone could fill a
    /// log with thousands of lines.
    /// </summary>
    public const int DiagnosticLineBudget = 100;

    /// <summary>
    /// Emulated frames between two summary lines: one minute of play, at the nominal 60 frames
    /// per second of NTSC (SPEC-07 §2) the same way REQ-E3D-646 counts its 600 frames.
    /// </summary>
    public const int SummaryFrames = 3600;

    private readonly Dictionary<PatternInstance, ObjectGeometry> _geometry = [];
    private readonly List<PatternInstance> _pending = [];

    /// <summary>Objects requested but not drawable yet, and the frame they started waiting.</summary>
    private readonly Dictionary<PatternInstance, long> _waiting = [];

    /// <summary>Objects whose construction the host delayed, and the frame it is due.</summary>
    private readonly Dictionary<PatternInstance, (EnginePattern Pattern, long DueFrame)> _delayed = [];

    private readonly HashSet<PatternInstance> _reportedWait = [];
    private readonly HashSet<PatternInstance> _reportedTexture = [];
    private readonly HashSet<EnginePattern> _texturePatterns = [];
    private readonly List<PatternInstance> _scratch = [];
    private readonly List<string> _buffered = [];

    private int _built;
    private int _transferred;
    private int _toBuild;

    private long _frame;
    private long _texelsHoused;
    private int _diagnosticLines;
    private int _summaryLines;
    private long _lastSummaryFrame;

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

    /// <summary>
    /// Total texture occupancy allowed, in texels (SPEC-04B REQ-E3D-564). Unbounded by default:
    /// REQ-E3D-603 forbids a fixed cap on what a frame draws, and REQ-E3D-564 only asks for a
    /// floor of 64 × 512 × 512 texels. The harness lowers it (<c>--atlas-budget</c>) to exercise
    /// REQ-E3D-644, and a host may lower it to whatever its driver really grants.
    /// </summary>
    public long TextureTexelBudget { get; set; } = long.MaxValue;

    /// <summary>
    /// Emulated frames the construction of an object is held back by, 0 — the default — meaning
    /// it is built straight away. A test instrument for REQ-E3D-646: REQ-E3D-601 recommends
    /// meshes ready at most 2 emulated frames after a pattern is created, so a delay is a test
    /// case and never a behaviour to aim for.
    /// </summary>
    public int BuildDelayFrames { get; set; }

    /// <summary>
    /// Where a degradation is reported: a texture that could not be housed (REQ-E3D-644) and an
    /// object left waiting for <see cref="WaitingDiagnosticFrames"/> emulated frames
    /// (REQ-E3D-646).
    /// </summary>
    /// <remarks>
    /// A sink attached after the profile has been loaded still receives what the load produced:
    /// the lines wait here until there is somewhere to put them, and the line budget bounds the
    /// wait as it bounds everything else. Setting it back to null keeps the next lines waiting
    /// rather than dropping them.
    /// </remarks>
    public Action<string>? Diagnostics
    {
        get => field;
        set
        {
            field = value;
            if (value is null || _buffered.Count == 0)
            {
                return;
            }

            foreach (string line in _buffered)
            {
                value(line);
            }

            _buffered.Clear();
        }
    }

    /// <summary>Number of 3D objects whose geometry was built since this builder was created.</summary>
    public int BuildCount { get; private set; }

    /// <summary>The 3D objects that currently hold geometry.</summary>
    public int ObjectCount => _geometry.Count;

    /// <summary>Texels the housed object textures occupy (REQ-E3D-564).</summary>
    public long HousedTexels => _texelsHoused;

    /// <summary>3D objects whose texture has no room in the budget (REQ-E3D-644).</summary>
    public int UnhousedObjects
    {
        get
        {
            int count = 0;
            foreach (ObjectGeometry geometry in _geometry.Values)
            {
                if (!geometry.TextureHoused)
                {
                    count++;
                }
            }

            return count;
        }
    }

    /// <summary>Diagnostic lines reported since this builder was created, summaries included.</summary>
    public int DiagnosticCount => _diagnosticLines + _summaryLines;

    /// <summary>3D objects for which a waiting diagnostic was reported (REQ-E3D-646).</summary>
    public int WaitingDiagnosticCount => _reportedWait.Count;

    /// <summary>Emulated frames this builder has been told about.</summary>
    public long FrameCount => _frame;

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
            // REQ-E3D-601: the object is known from here on and its shape stays recognised; it
            // is simply not drawn until its meshes and its texture are available. The frame it
            // started waiting is kept, so a rebuild does not restart the clock of REQ-E3D-646.
            _waiting.TryAdd(instance, _frame);

            if (BuildDelayFrames > 0)
            {
                _delayed[instance] = (pattern, _frame + BuildDelayFrames);
                continue;
            }

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
        DropDrawableWaiters();
        return count;
    }

    /// <inheritdoc/>
    public void Release(EnginePattern pattern)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        foreach (PatternInstance instance in pattern.Instances)
        {
            if (_geometry.Remove(instance, out ObjectGeometry? geometry) && geometry.TextureHoused)
            {
                // REQ-E3D-603: the texture space of a released pattern comes back, which is
                // what lets an object refused by REQ-E3D-644 be housed later on.
                _texelsHoused -= geometry.TextureTexels;
            }

            _pending.Remove(instance);
            _waiting.Remove(instance);
            _delayed.Remove(instance);
        }
    }

    /// <summary>Frees every mesh and texture (profile unloaded).</summary>
    public void Clear()
    {
        _geometry.Clear();
        _pending.Clear();
        _waiting.Clear();
        _delayed.Clear();

        // A game is over: the line budget of REQ-E3D-644 is per game and starts again.
        _reportedWait.Clear();
        _reportedTexture.Clear();
        _texturePatterns.Clear();
        _buffered.Clear();
        _texelsHoused = 0;
        _diagnosticLines = 0;
        _summaryLines = 0;
        _lastSummaryFrame = _frame;
        _built = 0;
        _transferred = 0;
        _toBuild = 0;
    }

    /// <inheritdoc/>
    public bool IsReady(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        return _geometry.TryGetValue(instance, out ObjectGeometry? geometry)
            && geometry.Ready

            // AMD-051 REQ-E3D-644: the tiles of an object whose texture could not be allocated
            // are not drawn until it can be. Nothing else about the object changes, so every
            // other pattern of the frame is drawn normally and the next frame is produced.
            && geometry.TextureHoused;
    }

    /// <summary>
    /// The builder's clock: one emulated frame has been converted. It runs the constructions
    /// <see cref="BuildDelayFrames"/> held back, retries the textures that had no room
    /// (REQ-E3D-644) and reports the objects that have been waiting for
    /// <see cref="WaitingDiagnosticFrames"/> frames (REQ-E3D-646).
    /// </summary>
    public void OnEmulatedFrame()
    {
        _frame++;
        RunDelayedBuilds();
        RetryUnhousedTextures();
        DropDrawableWaiters();
        ReportLongWaits();
        ReportSummary();
    }

    /// <summary>The geometry of a 3D object, or null when it has none yet.</summary>
    public ObjectGeometry? Of(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        return _geometry.GetValueOrDefault(instance);
    }

    /// <inheritdoc/>
    public TileMesh? MeshOf(PatternInstance instance, TileImage image)
    {
        ArgumentNullException.ThrowIfNull(instance);
        ArgumentNullException.ThrowIfNull(image);
        return _geometry.TryGetValue(instance, out ObjectGeometry? geometry)
            ? geometry.Mesh(image)
            : null;
    }

    /// <inheritdoc/>
    public TileMesh? MeshOf(PatternInstance instance, TileImage image, TileSides removedSides)
    {
        ArgumentNullException.ThrowIfNull(instance);
        ArgumentNullException.ThrowIfNull(image);
        return _geometry.TryGetValue(instance, out ObjectGeometry? geometry)
            ? geometry.Mesh(image, removedSides)
            : null;
    }

    /// <inheritdoc/>
    public bool IsFullTile(PatternInstance instance, TileImage image)
    {
        ArgumentNullException.ThrowIfNull(instance);
        ArgumentNullException.ThrowIfNull(image);
        return _geometry.TryGetValue(instance, out ObjectGeometry? geometry)
            && geometry.IsFullTile(image);
    }

    /// <inheritdoc/>
    public ObjectTexture? TextureOf(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        return _geometry.GetValueOrDefault(instance)?.Texture;
    }

    /// <inheritdoc/>
    public int MeshDepthOf(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        return _geometry.GetValueOrDefault(instance)?.BuildDepth ?? instance.Depth;
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

        // The texels the previous build held come back before the new texture asks for room,
        // so a rebuild of the same object never counts twice against the budget.
        if (previous is not null && previous.TextureHoused)
        {
            _texelsHoused -= previous.TextureTexels;
            previous.TextureHoused = false;
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
        bool[] full = new bool[fields.Length];
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

                // AMD-063 REQ-SCN-087: a variant whose effective algorithm is not MARCHING has
                // removable inner faces, whatever its opacity and its render style — the
                // opacity is read on the drawn instance and nowhere else.
                full[n] = IsFullTile(fields[n]);
            }
        }

        // REQ-E3D-452: from now on placement, the MARCHING scale correction, the deformation
        // and the scripts see D'.
        instance.Depth = effectiveDepth;

        ObjectGeometry built = new(
            pattern,
            texture,
            algorithm,
            requested,
            requested,
            effectiveDepth,
            meshes,
            algorithm == RenderAlgorithm.GreedyMulticolor,
            full);

        _geometry[instance] = built;
        House(instance, built, report: true);

        _pending.Add(instance);
        _built++;
        BuildCount++;
    }

    /// <summary>
    /// Whether <paramref name="field"/> is a full tile in the sense of AMD-063 REQ-SCN-087:
    /// <c>D > 0</c> and all three slots of each of its <c>8 × 8 × D</c> voxels occupied
    /// (REQ-E3D-422), <c>emptyColor</c> included (REQ-E3D-424).
    /// </summary>
    /// <remarks>
    /// A hollow geometry — <c>CUBE</c>, the cylinders — leaves the inside of the field empty and
    /// so yields no full tile, which the amendment wants.
    /// </remarks>
    private static bool IsFullTile(TileVoxels field)
    {
        if (field.Depth <= 0)
        {
            return false;
        }

        for (int k = 0; k < field.Depth; k++)
        {
            for (int j = 0; j < TileVoxels.SizeY; j++)
            {
                for (int i = 0; i < TileVoxels.SizeX; i++)
                {
                    if (!field.Occupied(i, j, k, TileVoxels.AxisX)
                        || !field.Occupied(i, j, k, TileVoxels.AxisY)
                        || !field.Occupied(i, j, k, TileVoxels.AxisZ))
                    {
                        return false;
                    }
                }
            }
        }

        return true;
    }

    // ----------------------------------------------- AMD-051 REQ-E3D-644 and REQ-E3D-646

    /// <summary>
    /// Reserves the texels of <paramref name="geometry"/> in the budget of REQ-E3D-564 and
    /// reports the refusal once per object when there is no room (REQ-E3D-644). Returns whether
    /// the object is housed.
    /// </summary>
    private bool House(PatternInstance instance, ObjectGeometry geometry, bool report)
    {
        long texels = geometry.TextureTexels;
        if (_texelsHoused + texels > TextureTexelBudget)
        {
            geometry.TextureHoused = false;
            if (report && _reportedTexture.Add(instance))
            {
                _texturePatterns.Add(geometry.Pattern);
                Report(
                    $"the texture of 3D object {instance.Index} of pattern "
                    + $"{geometry.Pattern.RegistrationIndex} needs {texels} texels and only "
                    + $"{Math.Max(0, TextureTexelBudget - _texelsHoused)} of the "
                    + $"{TextureTexelBudget} allowed are free: its tiles are not drawn until "
                    + "the texture can be allocated (REQ-E3D-644)");
            }

            return false;
        }

        _texelsHoused += texels;
        geometry.TextureHoused = true;
        return true;
    }

    /// <summary>Runs the constructions <see cref="BuildDelayFrames"/> held back until now.</summary>
    private void RunDelayedBuilds()
    {
        if (_delayed.Count == 0)
        {
            return;
        }

        _scratch.Clear();
        foreach ((PatternInstance instance, (EnginePattern _, long due)) in _delayed)
        {
            if (due <= _frame)
            {
                _scratch.Add(instance);
            }
        }

        if (_scratch.Count == 0)
        {
            return;
        }

        foreach (PatternInstance instance in _scratch)
        {
            EnginePattern pattern = _delayed[instance].Pattern;
            _delayed.Remove(instance);
            BuildObject(pattern, instance);
        }

        _scratch.Clear();
        if (!DeferTransfer)
        {
            Commit();
        }
    }

    /// <summary>
    /// Asks again for the texels of every object refused by REQ-E3D-644: a released pattern
    /// (REQ-E3D-603) may have freed the room it needs, and REQ-E3D-646 wants it drawn at the
    /// first frame that follows, never forgotten.
    /// </summary>
    private void RetryUnhousedTextures()
    {
        foreach ((PatternInstance instance, ObjectGeometry geometry) in _geometry)
        {
            if (!geometry.TextureHoused)
            {
                House(instance, geometry, report: false);
            }
        }
    }

    /// <summary>Takes out of the waiting list every object that can now be drawn.</summary>
    private void DropDrawableWaiters()
    {
        if (_waiting.Count == 0)
        {
            return;
        }

        _scratch.Clear();
        foreach (PatternInstance instance in _waiting.Keys)
        {
            if (IsReady(instance))
            {
                _scratch.Add(instance);
            }
        }

        foreach (PatternInstance instance in _scratch)
        {
            // REQ-E3D-646 allows one diagnostic per object for the whole game, so the object
            // stays in _reportedWait: it is never reported a second time.
            _waiting.Remove(instance);
        }

        _scratch.Clear();
    }

    /// <summary>
    /// REQ-E3D-646: an object that stays waiting for 600 consecutive emulated frames is
    /// reported once.
    /// </summary>
    private void ReportLongWaits()
    {
        foreach ((PatternInstance instance, long since) in _waiting)
        {
            if (_frame - since < WaitingDiagnosticFrames || _reportedWait.Contains(instance))
            {
                continue;
            }

            _reportedWait.Add(instance);
            Report(
                $"3D object {instance.Index} has not been drawable for "
                + $"{_frame - since} emulated frames: its meshes or its texture are still "
                + "missing (REQ-E3D-646)");
        }
    }

    /// <summary>
    /// The periodic summary REQ-E3D-644 asks for once the line budget is spent: at most one
    /// line per minute of play, giving the cumulative counts.
    /// </summary>
    private void ReportSummary()
    {
        if (_diagnosticLines < DiagnosticLineBudget
            || _reportedTexture.Count + _reportedWait.Count == 0
            || _frame - _lastSummaryFrame < SummaryFrames)
        {
            return;
        }

        _lastSummaryFrame = _frame;
        _summaryLines++;
        Emit(
            $"{_texturePatterns.Count} patterns and {_reportedTexture.Count} 3D objects have "
            + $"been degraded since this game started, {_reportedWait.Count} of them by waiting "
            + $"too long; right now {UnhousedObjects} objects have no room for their texture and "
            + $"{_waiting.Count} are not drawable (REQ-E3D-644, REQ-E3D-646)");
    }

    /// <summary>
    /// Reports one diagnostic line, under the budget of <see cref="DiagnosticLineBudget"/>
    /// lines per game; beyond it <see cref="ReportSummary"/> takes over.
    /// </summary>
    private void Report(string line)
    {
        if (_diagnosticLines >= DiagnosticLineBudget)
        {
            return;
        }

        _diagnosticLines++;
        Emit(line);
    }

    /// <summary>
    /// Hands <paramref name="line"/> to the sink, or holds it until there is one. The held lines
    /// are bounded by the same budget, so a builder nobody ever listens to keeps nothing.
    /// </summary>
    private void Emit(string line)
    {
        if (Diagnostics is { } sink)
        {
            sink(line);
        }
        else if (_buffered.Count < DiagnosticLineBudget)
        {
            _buffered.Add(line);
        }
    }
}
