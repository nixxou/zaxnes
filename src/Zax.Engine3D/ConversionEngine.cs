// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

using Zax.Nes;
using Zax.Profiles;

namespace Zax.Engine3D;

/// <summary>
/// The conversion pipeline of SPEC-04: it turns each batch of the core into a frame of
/// recognised shapes and instances, ready for the geometry of SPEC-04B.
/// </summary>
/// <remarks>
/// <para>
/// One instance per emulated game; it holds no static mutable state, so two engines in the
/// same process are independent (REQ-ARCH-011) and a run is reproducible (REQ-ARCH-012).
/// </para>
/// <para>
/// The host drives it: load a profile, then call <see cref="Convert"/> once per emulated frame
/// and <see cref="Publish"/> to hand the frame to the renderer (SPEC-07 REQ-LOOP-060).
/// </para>
/// </remarks>
public sealed class ConversionEngine
{
    private readonly List<ConversionFrame> _frames = [];
    private readonly ImageRegistry _images = new();
    private readonly PatternRegistry _patterns = new();
    private readonly Segmenter _segmenter;
    private readonly List<PatternInstance> _tagQuery = [];
    private readonly DrawListBuilder _drawListBuilder = new();

    private IScriptHost _host = NullScriptHost.Instance;
    private IPatternGeometryBuilder _geometry = NullPatternGeometryBuilder.Instance;
    private ConversionFrame _converting;
    private ConversionFrame? _published;
    private ConversionFrame? _pinned;

    /// <summary>Creates an engine with the factory settings of SPEC-04 §4.2.</summary>
    public ConversionEngine()
    {
        _converting = AcquireFrame();
        _segmenter = new Segmenter(_patterns, Settings, Learn);
    }

    /// <summary>The settings the pipeline consumes (SPEC-04 §4.2).</summary>
    public ConversionSettings Settings { get; } = new();

    /// <summary>The session's image registry (REQ-E3D-007).</summary>
    public ImageRegistry Images => _images;

    /// <summary>The pattern registry (REQ-E3D-060).</summary>
    public PatternRegistry Patterns => _patterns;

    /// <summary>
    /// True in 2D mode, the library entry that plays any ROM without a game profile
    /// (REQ-E3D-380, REQ-E3D-381).
    /// </summary>
    public bool TwoDimensionalMode { get; set; }

    /// <summary>The frame being converted; the one script queries see (SPEC-05 REQ-LUA-084).</summary>
    public ConversionFrame ConvertingFrame => _converting;

    /// <summary>
    /// The most recently published frame, the one the renderer shows (REQ-E3D-280), or null
    /// before the first publication.
    /// </summary>
    public ConversionFrame? PublishedFrame => _published;

    /// <summary>The pinned frame, or null when none is pinned (REQ-E3D-233).</summary>
    public ConversionFrame? PinnedFrame => _pinned;

    /// <summary>Loading progress of the geometry, 0 to 1 (REQ-E3D-362).</summary>
    public float LoadProgress => _geometry.Progress.Value;

    /// <summary>
    /// True when the loaded profile is in version 2 format, which moves the deformation pivot
    /// by one unit (SPEC-03, SPEC-04B REQ-E3D-568).
    /// </summary>
    public bool ProfileIsVersion2 { get; private set; }

    /// <summary>The Lua runtime the pipeline calls (REQ-E3D-320); a no-op host by default.</summary>
    public IScriptHost ScriptHost
    {
        get => _host;
        set => _host = value ?? NullScriptHost.Instance;
    }

    /// <summary>The geometry side (lot L4b); a builder that builds nothing by default.</summary>
    public IPatternGeometryBuilder Geometry
    {
        get => _geometry;
        set => _geometry = value ?? NullPatternGeometryBuilder.Instance;
    }

    /// <summary>
    /// Attaches a real geometry builder (SPEC-04B) that shares this engine's settings, so that
    /// a frame and a profile yield the full renderable scene description of REQ-E3D-641.
    /// </summary>
    /// <returns>The builder, so the host can drive its transfers and read its progress.</returns>
    public GeometryBuilder UseGeometry()
    {
        GeometryBuilder builder = new(Settings);
        Geometry = builder;
        return builder;
    }

    /// <summary>
    /// Loads a profile: applies the settings it carries, registers its patterns in the order of
    /// REQ-E3D-060, then runs <c>BuildPattern</c> and builds the geometry of each
    /// (REQ-E3D-322, SPEC-05 REQ-LUA-015 step 3).
    /// </summary>
    /// <remarks>
    /// Loading a profile starts from an empty registry, as a change of game does
    /// (REQ-E3D-008). Relaunching the same game without reloading the profile is
    /// <see cref="Restart"/>, which keeps what was learned.
    /// </remarks>
    /// <param name="profile">The profile to load.</param>
    /// <param name="applySettings">
    /// Whether to copy the profile's settings into <see cref="Settings"/>. A host that owns a
    /// settings model (SPEC-09) passes false and applies them through the model instead, which
    /// then mirrors them here: that keeps one source of truth, since the model also carries the
    /// values this slice does not (REQ-PERS-005, REQ-PERS-007).
    /// </param>
    public void LoadProfile(GameProfile profile, bool applySettings = true)
    {
        ArgumentNullException.ThrowIfNull(profile);

        _patterns.Clear();
        _images.Clear();
        ClearFrames();
        ProfileIsVersion2 = profile.IsV2Format;
        if (applySettings)
        {
            Settings.ApplyProfile(profile.Settings);
        }

        _patterns.LoadProfile(profile, _images);
        ProfileImageCount = _images.Count;
        RebuildAllGeometry();
    }

    /// <summary>
    /// How many images the profile's patterns interned. They keep their identity for the whole
    /// session; the images met while playing come after and belong to the learned state
    /// (REQ-E3D-008, DEC-R2-T5).
    /// </summary>
    public int ProfileImageCount { get; private set; }

    /// <summary>
    /// The list of tiles to draw for the published frame, or for the frame being converted
    /// when nothing has been published yet (SPEC-04B REQ-E3D-641).
    /// </summary>
    public FrameDrawList BuildDrawList()
        => _drawListBuilder.Build(_published ?? _converting, Settings, _geometry, ProfileIsVersion2);

    /// <summary>The list of tiles to draw for one frame (SPEC-04B REQ-E3D-641).</summary>
    public FrameDrawList BuildDrawList(ConversionFrame frame)
        => _drawListBuilder.Build(frame, Settings, _geometry, ProfileIsVersion2);

    /// <summary>
    /// Relaunches the same game without reloading the profile: the frames are emptied and the
    /// pinned frame released (REQ-E3D-281), while the image registry, the learned patterns and
    /// the temporary patterns stay active from the first frame (REQ-E3D-008).
    /// </summary>
    public void Restart() => ClearFrames();

    /// <summary>
    /// Forgets the registry, the learned patterns and the frames: what a change of game or a
    /// return from 2D mode does (REQ-E3D-008, REQ-E3D-380).
    /// </summary>
    public void Unload()
    {
        foreach (EnginePattern pattern in _patterns.Permanent)
        {
            _geometry.Release(pattern);
        }

        foreach (EnginePattern pattern in _patterns.Temporary)
        {
            _geometry.Release(pattern);
        }

        _patterns.Clear();
        _images.Clear();
        ProfileImageCount = 0;
        ClearFrames();
    }

    /// <summary>
    /// Rebuilds the geometry of one pattern, hooks included. Used when a savestate's learned
    /// patterns come back (REQ-E3D-008, REQ-E3D-322).
    /// </summary>
    internal void RebuildGeometryOf(EnginePattern pattern) => BuildGeometry(pattern);

    /// <summary>
    /// Empties every frame and releases the pinned one: what a game start, a reset or a state
    /// load does (REQ-E3D-281).
    /// </summary>
    public void ClearFrames()
    {
        _pinned = null;
        _published = null;
        foreach (ConversionFrame frame in _frames)
        {
            frame.Clear();
        }

        _converting = AcquireFrame();
    }

    /// <summary>
    /// Changes the global mesh algorithm and, when it really changes, rebuilds the geometry of
    /// every pattern of the registry (REQ-E3D-340).
    /// </summary>
    public void SetRenderAlgorithm(RenderAlgorithm algorithm)
    {
        if (Settings.RenderAlgorithm == algorithm)
        {
            return;
        }

        Settings.RenderAlgorithm = algorithm;
        RebuildAllGeometry();
    }

    /// <summary>
    /// Converts one batch of the core into a frame, in the order of REQ-E3D-001: palette, tile
    /// filtering and placement, background then sprite segmentation, Lua hooks, instance
    /// positions. The frame is left as <see cref="ConvertingFrame"/>; call
    /// <see cref="Publish"/> to hand it over.
    /// </summary>
    /// <param name="batch">The batch of the core (SPEC-02 REQ-CORE-030).</param>
    /// <param name="frameCounter">
    /// Number of emulated frames since the game started (REQ-E3D-043).
    /// </param>
    public ConversionFrame Convert(in FrameBatch batch, int frameCounter)
    {
        ConversionFrame frame = AcquireFrame();
        _converting = frame;
        frame.BeginBatch(frameCounter, new Cell(batch.MaskX, batch.MaskY));

        // (a) frame palette.
        frame.Palette.LoadFrom(batch.Palette);

        // (b) filtering and placement of the tiles.
        Intake(frame, batch.Records);

        // (c) segmentation.
        _segmenter.Run(frame, TwoDimensionalMode);

        // (d) Lua hooks.
        RunFrameHooks(frame);

        // (e) instance positions.
        CalculatePositions(frame);

        return frame;
    }

    /// <summary>
    /// Publishes the converted frame: from now on the renderer shows it (REQ-E3D-280). The
    /// next conversion goes to another buffer, so neither the published frame nor the pinned
    /// one is ever touched.
    /// </summary>
    public ConversionFrame Publish()
    {
        _published = _converting;
        return _published;
    }

    /// <summary>Recomputes <c>Pos</c> and <c>Center</c> of every instance (REQ-E3D-205).</summary>
    public void CalculatePositions(ConversionFrame frame)
    {
        ArgumentNullException.ThrowIfNull(frame);
        foreach (FrameInstance instance in frame.Instances)
        {
            instance.CalculatePos(Settings);
        }
    }

    /// <summary>
    /// The engine's tracker (REQ-E3D-300), the one the subjective camera uses and the one a
    /// savestate carries (DEC-R2-T5).
    /// </summary>
    public ShapeTracker Tracker { get; } = new();

    /// <summary>Starts the engine's tracker on <paramref name="shape"/> (REQ-E3D-300).</summary>
    public ShapeTracker StartTracker(FrameShape shape)
    {
        Tracker.Start(shape);
        return Tracker;
    }

    /// <summary>The scene background colour of the published frame (REQ-E3D-254).</summary>
    public Rgb24 BackgroundColor(bool skyboxActive)
        => _published is null
            ? Settings.CustomBackgroundColor
            : Settings.BackgroundColor(_published.Palette, skyboxActive);

    #region palette access for scripts (REQ-E3D-252)

    /// <summary>
    /// Reads a frame palette entry: the published frame when <paramref name="current"/> is
    /// false, the frame being converted when it is true (REQ-E3D-252).
    /// </summary>
    public int FrameColorIndex(int entry, bool current = false)
    {
        FramePalette? palette = current ? _converting.Palette : _published?.Palette;
        return palette is null ? 0 : palette[entry];
    }

    /// <summary>Writes a frame palette entry for this frame only (REQ-E3D-252).</summary>
    public void SetColorIndex(int entry, int value, bool current = false)
    {
        FramePalette? palette = current ? _converting.Palette : _published?.Palette;
        if (palette is not null)
        {
            palette[entry] = (byte)value;
        }
    }

    /// <summary>The RGB colour of a palette entry of the published frame (REQ-E3D-252).</summary>
    public Rgb24 FrameColor(int entry)
        => _published is null ? default : _published.Palette.Color(entry);

    #endregion

    #region shapes injected by scripts (REQ-E3D-230 to 232)

    /// <summary>
    /// <c>Frame:CreateNewShape(P, pos, pal)</c> with a pattern: adds a non-scriptable sprite
    /// shape whose tiles sit at <c>x = 8i + ⌊pos.x⌉₀</c> and
    /// <c>y = 8j + (Hs − (⌊pos.y⌉₀ + 8H) + 1)</c> (REQ-E3D-230).
    /// </summary>
    public FrameShape CreateNewShape(EnginePattern pattern, Vec3 position, int palette)
        => InjectShape(pattern, position, palette, centred: true, onlyInstance: null);

    /// <summary>
    /// The same with a pattern instance: only instance <c>P.Index</c> is enabled and it is the
    /// returned value (REQ-E3D-230, SPEC-05 REQ-LUA-086).
    /// </summary>
    public FrameInstance CreateNewShape(PatternInstance instance, Vec3 position, int palette)
    {
        ArgumentNullException.ThrowIfNull(instance);
        EnginePattern pattern = OwnerOf(instance);
        FrameShape shape = InjectShape(pattern, position, palette, centred: true, instance.Index);
        return shape.Instances[Math.Clamp(instance.Index, 0, shape.Instances.Count - 1)];
    }

    /// <summary>
    /// <c>Frame:AddNewShape(P, pos, pal)</c>: the tiles sit at <c>x = 8i</c>, <c>y = 8j</c> and
    /// every instance offset receives <c>(pos.x, pos.y − Hs + 8H, pos.z)</c> (REQ-E3D-231).
    /// </summary>
    public FrameShape AddNewShape(EnginePattern pattern, Vec3 position, int palette)
        => InjectShape(pattern, position, palette, centred: false, onlyInstance: null);

    /// <summary>The same with a pattern instance (REQ-E3D-231, SPEC-05 REQ-LUA-086).</summary>
    public FrameInstance AddNewShape(PatternInstance instance, Vec3 position, int palette)
    {
        ArgumentNullException.ThrowIfNull(instance);
        EnginePattern pattern = OwnerOf(instance);
        FrameShape shape = InjectShape(pattern, position, palette, centred: false, instance.Index);
        return shape.Instances[Math.Clamp(instance.Index, 0, shape.Instances.Count - 1)];
    }

    /// <summary>
    /// <c>Frame:InsertShape(P, pos, pal)</c>, which is
    /// <c>CreateNewShape(P, pos − BottomLeft(P), pal)</c> (REQ-E3D-232).
    /// </summary>
    public FrameShape InsertShape(EnginePattern pattern, Vec3 position, int palette)
    {
        ArgumentNullException.ThrowIfNull(pattern);
        Vec2 bottomLeft = pattern.BottomLeft;
        Vec3 shifted = new(position.X - bottomLeft.X, position.Y - bottomLeft.Y, position.Z);
        return CreateNewShape(pattern, shifted, palette);
    }

    private FrameShape InjectShape(
        EnginePattern pattern,
        Vec3 position,
        int palette,
        bool centred,
        int? onlyInstance)
    {
        ArgumentNullException.ThrowIfNull(pattern);

        ConversionFrame frame = _converting;
        FrameShape shape = frame.TakeShape(background: false);
        shape.Scriptable = false;

        int truncatedX = (int)position.X;
        int truncatedY = (int)position.Y;
        int flip = ConversionGrid.SceneHeight - (truncatedY + (8 * pattern.Height)) + 1;

        for (int j = 0; j < pattern.Height; j++)
        {
            for (int i = 0; i < pattern.Width; i++)
            {
                TileImage? image = pattern.ImageAt(i, j);
                if (image is null)
                {
                    continue;
                }

                int x = centred ? (8 * i) + truncatedX : 8 * i;
                int y = centred ? (8 * j) + flip : 8 * j;
                shape.AddTile(frame.AddInjectedTile(image, x, y, palette - 4));
            }
        }

        shape.Bind(pattern);

        Vec3 extra = centred
            ? new Vec3(position.X - truncatedX, -(position.Y - truncatedY), position.Z)
            : new Vec3(position.X, position.Y - ConversionGrid.SceneHeight + (8 * pattern.Height), position.Z);

        foreach (FrameInstance instance in shape.Instances)
        {
            instance.Offset += extra;
            if (onlyInstance is { } only && instance.Index != only)
            {
                instance.Enable = false;
            }
        }

        frame.AddShape(shape);
        return shape;
    }

    /// <summary>
    /// The registry pattern <paramref name="instance"/> belongs to, or null when it belongs to
    /// none. Scripts read it as <c>refPattern</c> (SPEC-05 §4.2).
    /// </summary>
    public EnginePattern? FindPattern(PatternInstance instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        PatternInstance reference = instance.Reference ?? instance;
        foreach (EnginePattern pattern in _patterns.Permanent)
        {
            if (pattern.Instances.Contains(reference))
            {
                return pattern;
            }
        }

        foreach (EnginePattern pattern in _patterns.Temporary)
        {
            if (pattern.Instances.Contains(reference))
            {
                return pattern;
            }
        }

        return null;
    }

    private EnginePattern OwnerOf(PatternInstance instance)
    {
        PatternInstance reference = instance.Reference ?? instance;
        foreach (EnginePattern pattern in _patterns.Permanent)
        {
            if (pattern.Instances.Contains(reference))
            {
                return pattern;
            }
        }

        foreach (EnginePattern pattern in _patterns.Temporary)
        {
            if (pattern.Instances.Contains(reference))
            {
                return pattern;
            }
        }

        throw new ArgumentException("The instance does not belong to a registry pattern.", nameof(instance));
    }

    #endregion

    #region pinned frame (REQ-E3D-233)

    /// <summary>Freezes the last published frame until it is released (REQ-E3D-233).</summary>
    public void PinNewFrame() => _pinned = _published;

    /// <summary>Releases the pinned frame (REQ-E3D-233).</summary>
    public void ReleasePinnedFrame() => _pinned = null;

    /// <summary>
    /// Adds a rectangle, in scene coordinates, to the list of the frame being converted. No
    /// effect without a pinned frame (REQ-E3D-233).
    /// </summary>
    public void AddRectFromPinnedFrame(float x, float y, float width, float height)
    {
        if (_pinned is null)
        {
            return;
        }

        _converting.AddPinnedRect(new Rect(x, y, width, height));
    }

    /// <summary>
    /// Adds, as non-scriptable shapes of the frame being converted, every shape of the pinned
    /// frame whose box overlaps one of the frame's rectangles (REQ-E3D-233).
    /// </summary>
    public int RestoreFromPinnedFrame()
    {
        if (_pinned is null)
        {
            return 0;
        }

        int restored = 0;
        foreach (FrameShape shape in _pinned.Shapes)
        {
            if (!OverlapsPinnedRects(shape, _converting))
            {
                continue;
            }

            if (_converting.AddForeignShape(shape))
            {
                restored++;
            }
        }

        return restored;
    }

    /// <summary>Adds the rectangle, then restores the shapes that overlap it (REQ-E3D-233).</summary>
    public int RestoreFromPinnedFrame(Rect rectangle)
    {
        if (_pinned is null)
        {
            return 0;
        }

        _converting.AddPinnedRect(rectangle);
        return RestoreFromPinnedFrame();
    }

    /// <summary>
    /// The shapes of the pinned frame that overlap <paramref name="rectangle"/>
    /// (SPEC-05 <c>Frame:GetShapesFromPinnedFrame</c>).
    /// </summary>
    public void CollectShapesFromPinnedFrame(Rect rectangle, List<FrameShape> destination)
    {
        ArgumentNullException.ThrowIfNull(destination);
        destination.Clear();
        if (_pinned is null)
        {
            return;
        }

        foreach (FrameShape shape in _pinned.Shapes)
        {
            if (ShapeBox(shape).Overlaps(rectangle))
            {
                destination.Add(shape);
            }
        }
    }

    private static bool OverlapsPinnedRects(FrameShape shape, ConversionFrame frame)
    {
        Rect box = ShapeBox(shape);
        foreach (Rect rectangle in frame.PinnedRects)
        {
            if (box.Overlaps(rectangle))
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>
    /// The box <c>(pStart, pEnd − pStart + 1)</c> a restore test uses. It is in screen pixels
    /// while the rectangles are in scene coordinates; the mismatch is the original's and is
    /// kept (DEC-04-Q05, Q-05).
    /// </summary>
    private static Rect ShapeBox(FrameShape shape) => new(
        shape.PixelStart.X,
        shape.PixelStart.Y,
        shape.PixelEnd.X - shape.PixelStart.X + 1,
        shape.PixelEnd.Y - shape.PixelStart.Y + 1);

    #endregion

    #region queries for scripts (SPEC-05 REQ-LUA-084, REQ-LUA-085)

    /// <summary>The scriptable instance of index <paramref name="index"/> (REQ-E3D-220).</summary>
    public FrameInstance GetShape(int index) => _converting.ScriptableInstances[index];

    /// <summary>Number of scriptable instances of the frame being converted.</summary>
    public int ScriptableCount => _converting.ScriptableInstances.Count;

    /// <summary>
    /// The scriptable instances carrying <paramref name="tag"/>, in the order of REQ-E3D-220
    /// (SPEC-05 REQ-LUA-084).
    /// </summary>
    public void CollectShapesWithTag(string tag, List<FrameInstance> destination)
    {
        ArgumentNullException.ThrowIfNull(tag);
        ArgumentNullException.ThrowIfNull(destination);
        destination.Clear();
        foreach (FrameInstance instance in _converting.ScriptableInstances)
        {
            if (instance.ContainsTag(tag))
            {
                destination.Add(instance);
            }
        }
    }

    /// <summary>The first scriptable instance carrying <paramref name="tag"/>, or null.</summary>
    public FrameInstance? GetShapeWithTag(string tag)
    {
        ArgumentNullException.ThrowIfNull(tag);
        foreach (FrameInstance instance in _converting.ScriptableInstances)
        {
            if (instance.ContainsTag(tag))
            {
                return instance;
            }
        }

        return null;
    }

    /// <summary>
    /// The first scriptable instance whose box contains the scene point
    /// (<paramref name="x"/>, <paramref name="y"/>) and that carries <paramref name="tag"/>
    /// (SPEC-05 REQ-LUA-084 point 5).
    /// </summary>
    public FrameInstance? GetShapeWithPos(float x, float y, string tag = "")
    {
        ArgumentNullException.ThrowIfNull(tag);
        foreach (FrameInstance instance in _converting.ScriptableInstances)
        {
            Vec3 bottomLeft = instance.BottomLeft;
            Vec3 topRight = instance.TopRight;
            if (bottomLeft.X <= x
                && x <= topRight.X
                && bottomLeft.Y <= y
                && y <= topRight.Y
                && instance.ContainsTag(tag))
            {
                return instance;
            }
        }

        return null;
    }

    /// <summary>The first pattern instance carrying <paramref name="tag"/>, or null.</summary>
    public PatternInstance? GetPatternWithTag(string tag) => _patterns.FindInstanceWithTag(tag);

    /// <summary>Every pattern instance carrying <paramref name="tag"/>, in registry order.</summary>
    public IReadOnlyList<PatternInstance> GetPatternsWithTag(string tag)
    {
        _patterns.CollectInstancesWithTag(tag, _tagQuery);
        return _tagQuery;
    }

    #endregion

    /// <summary>
    /// Learns a pattern from a shape (REQ-E3D-161): a grid the size of its tile box, the
    /// recognition defaults, one instance with the 3D defaults of SPEC-04 §4.3, registration,
    /// the <c>BuildPattern</c> hook and then its geometry.
    /// </summary>
    private EnginePattern Learn(FrameShape shape, bool permanent)
    {
        Cell size = shape.TileSize;
        int width = Math.Max(size.X, 1);
        int height = Math.Max(size.Y, 1);
        TileImage?[] cells = new TileImage?[width * height];
        for (int j = 0; j < height; j++)
        {
            for (int i = 0; i < width; i++)
            {
                int index = ConversionGrid.Index(shape.TileStart.X + i, shape.TileStart.Y + j);
                cells[i + (j * width)] = shape.TileAt(index)?.Image;
            }
        }

        EnginePattern pattern = new(width, height, cells, permanent);
        pattern.AddInstance(new PatternInstance());
        _patterns.Add(pattern);
        BuildGeometry(pattern);
        return pattern;
    }

    private void RebuildAllGeometry()
    {
        // The loading progress of SPEC-04B REQ-E3D-600 counts 3D objects, so the builder is
        // told the total before the first one.
        int objects = 0;
        foreach (EnginePattern pattern in _patterns.Permanent)
        {
            objects += pattern.Instances.Count;
        }

        foreach (EnginePattern pattern in _patterns.Temporary)
        {
            objects += pattern.Instances.Count;
        }

        _geometry.BeginMassBuild(objects);

        foreach (EnginePattern pattern in _patterns.Permanent)
        {
            BuildGeometry(pattern);
        }

        foreach (EnginePattern pattern in _patterns.Temporary)
        {
            BuildGeometry(pattern);
        }
    }

    /// <summary>
    /// Runs <c>BuildPattern</c> for every instance of the pattern, then builds its geometry
    /// (REQ-E3D-322: the hook always runs before the construction).
    /// </summary>
    private void BuildGeometry(EnginePattern pattern)
    {
        foreach (PatternInstance instance in pattern.Instances)
        {
            try
            {
                _host.InvokeBuildPattern(instance);
            }
            catch (Exception)
            {
                // A Lua error must not stop the conversion (REQ-E3D-324); the runtime
                // disables the faulty script (SPEC-05 §3.8).
            }
        }

        _geometry.Build(pattern);
    }

    /// <summary>
    /// Filters and places the tiles of a batch: the clip area of REQ-E3D-002, the sprite rules
    /// of REQ-E3D-003, the empty background tiles of REQ-E3D-004 and the grid placement of
    /// REQ-E3D-005.
    /// </summary>
    private void Intake(ConversionFrame frame, ReadOnlySpan<TileRecord> records)
    {
        Vec4F clip = Settings.Clipping;
        foreach (TileRecord record in records)
        {
            if (!IsInsideClipArea(record.X, record.Y, clip))
            {
                continue;
            }

            TileImage image = _images.Intern(
                record.Plane0,
                record.Plane1,
                record.MirrorH,
                record.MirrorV);

            if (record.Background)
            {
                if (image.IsEmpty && !Settings.EmptyTile)
                {
                    continue;
                }

                Cell cell = ConversionGrid.BackgroundCell(record.X, record.Y);
                if (cell.X <= 0
                    || cell.Y <= 0
                    || cell.X >= ConversionGrid.Width - 1
                    || cell.Y >= ConversionGrid.Height - 1)
                {
                    // The grid has one cell of margin on each side precisely so that growth
                    // never needs a bounds test (SPEC-04 §2). With the clip margins of §4.2 no
                    // tile can land there; one that would is dropped.
                    continue;
                }

                frame.PlaceBackground(image, record.X, record.Y, record.Palette, record.Hidden);
                continue;
            }

            if (record.Y >= 240 || image.Key.IsEmpty)
            {
                continue;
            }

            frame.AddSprite(image, record.X, record.Y, record.Palette, record.Hidden, record.Index);
        }
    }

    /// <summary>
    /// The clip test of REQ-E3D-002: kept when <c>x + 8 &gt; g</c>, <c>x ≤ 256 − d</c>,
    /// <c>y + 8 &gt; h</c> and <c>y ≤ 248 − b</c>.
    /// </summary>
    internal static bool IsInsideClipArea(int x, int y, Vec4F clipping)
        => x + 8 > clipping.X
        && x <= 256 - clipping.Z
        && y + 8 > clipping.W
        && y <= 248 - clipping.Y;

    /// <summary>
    /// The Lua hooks of the pipeline, in the order of REQ-E3D-320. An error in a hook is
    /// counted and the conversion carries on (REQ-E3D-324).
    /// </summary>
    private void RunFrameHooks(ConversionFrame frame)
    {
        try
        {
            _host.SetFrameGlobals(frame.Counter, frame.Time);
        }
        catch (Exception)
        {
            frame.ScriptErrorCount++;
        }

        try
        {
            _host.InvokeUpdate();
        }
        catch (Exception)
        {
            frame.ScriptErrorCount++;
        }

        try
        {
            _host.InvokeUpdateShapes(frame.ScriptableInstances);
        }
        catch (Exception)
        {
            frame.ScriptErrorCount++;
        }

        try
        {
            _host.InvokeLateUpdate();
        }
        catch (Exception)
        {
            frame.ScriptErrorCount++;
        }
    }

    /// <summary>
    /// A frame buffer that is neither displayed nor pinned, so that converting a new frame
    /// never touches either (REQ-E3D-280).
    /// </summary>
    private ConversionFrame AcquireFrame()
    {
        foreach (ConversionFrame frame in _frames)
        {
            if (!ReferenceEquals(frame, _published) && !ReferenceEquals(frame, _pinned))
            {
                return frame;
            }
        }

        ConversionFrame fresh = new();
        _frames.Add(fresh);
        return fresh;
    }
}
