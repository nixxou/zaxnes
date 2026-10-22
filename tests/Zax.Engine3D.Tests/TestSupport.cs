using Zax.Nes;
using Zax.Profiles;

namespace Zax.Engine3D.Tests;

/// <summary>
/// The tile images the test vectors of SPEC-04 §5 use.
/// </summary>
/// <remarks>
/// The conventions of §5 call A, B, C… "distinct full images (64 pixels, colour 1)". A full
/// tile of nothing but colour 1 is unique, so <see cref="Full"/> keeps all 64 pixels opaque and
/// makes the identity unique with one marker pixel in colour 3. Every property the vectors rely
/// on is unchanged: 64 opaque pixels, pixel box (0,0)–(7,7), density SOLID and 2D shape
/// FULL_RECT.
/// </remarks>
internal static class Img
{
    /// <summary>A full image whose identity is <paramref name="id"/> (0 to 63).</summary>
    public static TileImageKey Full(int id)
    {
        Assert.InRange(id, 0, 63);
        return new TileImageKey(ulong.MaxValue, 1UL << id, false, false);
    }

    /// <summary>The empty image: both planes zero.</summary>
    public static TileImageKey Empty => new(0UL, 0UL, false, false);

    /// <summary>
    /// A single-pixel image: colour 1 at the source pixel
    /// (<paramref name="x"/>, <paramref name="y"/>), used by TV-E3D-10.
    /// </summary>
    public static TileImageKey Pixel(int x, int y, bool flipX = false, bool flipY = false)
        => new(1UL << ((y << 3) | (7 - x)), 0UL, flipX, flipY);

    /// <summary>The same image shown flipped, which is another image (SPEC-04 §2).</summary>
    public static TileImageKey Flipped(TileImageKey image, bool flipX = false, bool flipY = false)
        => image with { FlipX = flipX, FlipY = flipY };

    // Shorthands for the vectors of §5.
    public static TileImageKey A => Full(0);

    public static TileImageKey B => Full(1);

    public static TileImageKey C => Full(2);

    public static TileImageKey D => Full(3);

    public static TileImageKey U => Full(4);
}

/// <summary>
/// Builds a batch of the core (SPEC-02 REQ-CORE-030) and hands it to an engine. A batch is a
/// <c>ref struct</c>, so it is assembled here and built inline at conversion time.
/// </summary>
internal sealed class Batch
{
    private readonly List<TileRecord> _records = [];
    private readonly byte[] _palette = new byte[32];
    private int _spriteOrd;

    /// <summary>Unreliable border width on the left and right edges.</summary>
    public int MaskX { get; private set; }

    /// <summary>Unreliable border width on the top and bottom edges.</summary>
    public int MaskY { get; private set; }

    /// <summary>The records, in emission order.</summary>
    public IReadOnlyList<TileRecord> Records => _records;

    /// <summary>Sets the mask the core reports (REQ-CORE-046).</summary>
    public Batch Mask(int x, int y)
    {
        MaskX = x;
        MaskY = y;
        return this;
    }

    /// <summary>Sets one of the 32 PPU palette bytes.</summary>
    public Batch Palette(int index, byte value)
    {
        _palette[index] = value;
        return this;
    }

    /// <summary>Adds a background tile at the screen position (x, y).</summary>
    public Batch Bg(TileImageKey image, int x, int y, int palette = 0)
    {
        _records.Add(new TileRecord(
            image.Low,
            image.High,
            (short)x,
            (short)y,
            _records.Count,
            image.FlipX,
            image.FlipY,
            hidden: false,
            (byte)palette,
            background: true));
        return this;
    }

    /// <summary>
    /// Adds a background tile in cell <c>(column + 1, row + 1)</c>, that is at the screen
    /// position <c>(8·column, 8·row)</c> — the convention of SPEC-04 §5.
    /// </summary>
    public Batch BgCell(TileImageKey image, int column, int row, int palette = 0)
        => Bg(image, 8 * column, 8 * row, palette);

    /// <summary>Adds a whole row of background cells, left to right from <paramref name="column"/>.</summary>
    public Batch BgRow(int row, int column, params TileImageKey[] images)
    {
        for (int i = 0; i < images.Length; i++)
        {
            BgCell(images[i], column + i, row);
        }

        return this;
    }

    /// <summary>Adds a sprite tile at the screen position (x, y), in emission order.</summary>
    public Batch Sprite(TileImageKey image, int x, int y, int palette = 0, bool hidden = false)
    {
        _records.Add(new TileRecord(
            image.Low,
            image.High,
            (short)x,
            (short)y,
            _spriteOrd++,
            image.FlipX,
            image.FlipY,
            hidden,
            (byte)palette,
            background: false));
        return this;
    }

    /// <summary>Converts the batch with <paramref name="engine"/> and publishes nothing.</summary>
    public ConversionFrame Convert(ConversionEngine engine, int counter = 0)
    {
        ArgumentNullException.ThrowIfNull(engine);
        FrameBatch batch = new(_records.ToArray(), _palette, MaskX, MaskY);
        return engine.Convert(batch, counter);
    }

    /// <summary>Converts the batch and publishes the frame (REQ-E3D-280).</summary>
    public ConversionFrame ConvertAndPublish(ConversionEngine engine, int counter = 0)
    {
        ConversionFrame frame = Convert(engine, counter);
        engine.Publish();
        return frame;
    }

    /// <summary>A full frame: 33 × 31 background tiles and 64 sprites (REQ-E3D-360).</summary>
    public static Batch FullFrame(int distinctImages = 16)
    {
        Batch batch = new();
        int n = 0;
        for (int row = 0; row < 31; row++)
        {
            for (int column = 0; column < 33; column++)
            {
                batch.BgCell(Img.Full(n++ % distinctImages), column, row, (row + column) & 3);
            }
        }

        for (int i = 0; i < 64; i++)
        {
            batch.Sprite(Img.Full(i % distinctImages), (i * 3) % 248, 8 + ((i * 5) % 224), i & 3);
        }

        return batch;
    }
}

/// <summary>Builds a profile whose patterns the registry can load (REQ-E3D-060).</summary>
internal sealed class ProfileBuilder
{
    private readonly GameProfile _profile = GameProfile.CreateEmpty();

    /// <summary>The profile settings, so that a test can change one.</summary>
    public ProfileSettings Settings => _profile.Settings!;

    /// <summary>Adds a pattern of one row: the <c>[A B]</c> of the test vectors.</summary>
    public ProfileBuilder Row(
        TileImageKey?[] cells,
        bool selfConnected = true,
        bool skipGeometry = false,
        int constraintWidth = 99,
        int constraintHeight = 99,
        bool permanent = true,
        string? tags = null,
        int instanceCount = 1)
    {
        ArgumentNullException.ThrowIfNull(cells);
        return Pattern(
            cells.Length,
            1,
            cells,
            selfConnected,
            skipGeometry,
            constraintWidth,
            constraintHeight,
            permanent,
            tags,
            instanceCount: instanceCount);
    }

    /// <summary>Adds a pattern.</summary>
    public ProfileBuilder Pattern(
        int width,
        int height,
        TileImageKey?[] cells,
        bool selfConnected = true,
        bool skipGeometry = false,
        int constraintWidth = 99,
        int constraintHeight = 99,
        bool permanent = true,
        string? tags = null,
        int depth = 3,
        ProfileLayer layer = ProfileLayer.L1,
        int instanceCount = 1)
    {
        List<ProfileTile?> tiles = [];
        foreach (TileImageKey? cell in cells)
        {
            tiles.Add(cell is { } key
                ? new ProfileTile(
                    key.Low,
                    key.High,
                    (key.FlipX ? TileFlip.Horizontal : TileFlip.None)
                        | (key.FlipY ? TileFlip.Vertical : TileFlip.None))
                : null);
        }

        ProfilePattern pattern = new()
        {
            Grid = new PatternGrid(width, height, tiles),
            TileSize = new Vec2I(width, height),
            Permanent = permanent,
            SelfConnected = selfConnected,
            SkipGeometry = skipGeometry,
            ConstraintSize = new Vec2I(constraintWidth, constraintHeight),
        };

        for (int k = 0; k < instanceCount; k++)
        {
            pattern.Variants.Add(new PatternVariant
            {
                Depth = depth,
                Layer = layer,
                Alpha = 1f,
                RenderStyle = VariantRenderStyle.Normal,
                TagString = tags,
            });
        }

        if (permanent)
        {
            _profile.PermanentPatterns.Add(pattern);
        }
        else
        {
            _profile.TemporaryPatterns.Add(pattern);
        }

        return this;
    }

    /// <summary>The profile.</summary>
    public GameProfile Build() => _profile;

    /// <summary>An engine with this profile loaded.</summary>
    public ConversionEngine Load()
    {
        ConversionEngine engine = new();
        engine.LoadProfile(Build());
        return engine;
    }

    /// <summary>An engine with no pattern at all, the "empty profile" of the vectors.</summary>
    public static ConversionEngine EmptyEngine()
    {
        ConversionEngine engine = new();
        engine.LoadProfile(GameProfile.CreateEmpty());
        return engine;
    }
}

/// <summary>
/// A script host that records what the pipeline calls and lets a test run code at each call
/// point (REQ-E3D-320).
/// </summary>
internal sealed class RecordingScriptHost : IScriptHost
{
    /// <summary>The call log, in order.</summary>
    public List<string> Calls { get; } = [];

    /// <summary>The frame counter of the last <c>SetFrameGlobals</c>.</summary>
    public int LastFrame { get; private set; } = -1;

    /// <summary>The frame time of the last <c>SetFrameGlobals</c>.</summary>
    public float LastTime { get; private set; } = -1f;

    /// <summary>The instances the last <c>UpdateS</c> round received, in order.</summary>
    public List<FrameInstance> UpdateShapesArgument { get; } = [];

    /// <summary>The pattern instances <c>BuildPattern</c> was called with, in order.</summary>
    public List<PatternInstance> BuiltPatterns { get; } = [];

    /// <summary>Runs during <c>Update</c>.</summary>
    public Action? OnUpdate { get; set; }

    /// <summary>Runs once per scriptable instance, in the order of REQ-E3D-220.</summary>
    public Action<FrameInstance>? OnUpdateShape { get; set; }

    /// <summary>Runs during <c>LateUpdate</c>.</summary>
    public Action? OnLateUpdate { get; set; }

    /// <summary>Runs during <c>BuildPattern</c>.</summary>
    public Action<PatternInstance>? OnBuildPattern { get; set; }

    /// <inheritdoc/>
    public bool HasFrameHooks => true;

    /// <inheritdoc/>
    public void SetFrameGlobals(int frameCounter, float time)
    {
        LastFrame = frameCounter;
        LastTime = time;
        Calls.Add($"globals({frameCounter})");
    }

    /// <inheritdoc/>
    public void InvokeUpdate()
    {
        Calls.Add("Update");
        OnUpdate?.Invoke();
    }

    /// <inheritdoc/>
    public void InvokeUpdateShapes(IReadOnlyList<FrameInstance> scriptableInstances)
    {
        Calls.Add($"UpdateS[{scriptableInstances.Count}]");
        UpdateShapesArgument.Clear();
        UpdateShapesArgument.AddRange(scriptableInstances);
        if (OnUpdateShape is null)
        {
            return;
        }

        foreach (FrameInstance instance in scriptableInstances)
        {
            OnUpdateShape(instance);
        }
    }

    /// <inheritdoc/>
    public void InvokeLateUpdate()
    {
        Calls.Add("LateUpdate");
        OnLateUpdate?.Invoke();
    }

    /// <inheritdoc/>
    public void InvokeBuildPattern(PatternInstance patternInstance)
    {
        Calls.Add("BuildPattern");
        BuiltPatterns.Add(patternInstance);
        OnBuildPattern?.Invoke(patternInstance);
    }
}

/// <summary>
/// A geometry builder that records what the pipeline asks of it and lets a test drive the
/// readiness and the progress (REQ-E3D-340, REQ-E3D-361, REQ-E3D-362).
/// </summary>
internal sealed class RecordingGeometryBuilder : IPatternGeometryBuilder
{
    /// <summary>The patterns built, in order; a pattern rebuilt appears twice.</summary>
    public List<EnginePattern> Built { get; } = [];

    /// <summary>The patterns released, in order.</summary>
    public List<EnginePattern> Released { get; } = [];

    /// <summary>Instances declared not ready (REQ-E3D-361).</summary>
    public HashSet<PatternInstance> NotReady { get; } = [];

    /// <summary>What <see cref="Progress"/> returns.</summary>
    public GeometryProgress ReportedProgress { get; set; }

    /// <inheritdoc/>
    public void Build(EnginePattern pattern) => Built.Add(pattern);

    /// <inheritdoc/>
    public void Release(EnginePattern pattern) => Released.Add(pattern);

    /// <inheritdoc/>
    public bool IsReady(PatternInstance instance) => !NotReady.Contains(instance);

    /// <summary>The object counts <see cref="BeginMassBuild"/> was told, in order.</summary>
    public List<int> MassBuilds { get; } = [];

    /// <inheritdoc/>
    public TileMesh? MeshOf(PatternInstance instance, TileImage image) => Stub;

    public TileMesh? MeshOf(PatternInstance instance, TileImage image, int sealedSides) => Stub;

    public int FullSidesOf(PatternInstance instance, TileImage image) => 0;

    /// <inheritdoc/>
    public ObjectTexture? TextureOf(PatternInstance instance) => null;

    /// <inheritdoc/>
    public void BeginMassBuild(int objectCount) => MassBuilds.Add(objectCount);

    /// <inheritdoc/>
    public GeometryProgress Progress => ReportedProgress;

    /// <summary>A mesh that stands in for a real one: one triangle, so it is not empty.</summary>
    private static TileMesh Stub { get; } = new(
        [
            new MeshVertex(Vec3.Zero, new Vec3(0f, 0f, -1f), Vec2.Zero),
            new MeshVertex(new Vec3(1f, 0f, 0f), new Vec3(0f, 0f, -1f), Vec2.Zero),
            new MeshVertex(new Vec3(0f, 1f, 0f), new Vec3(0f, 0f, -1f), Vec2.Zero),
        ],
        [0, 1, 2]);
}

/// <summary>
/// Groups the test classes that work on the whole built-in catalogue. They are the only heavy
/// ones; keeping them in one collection stops them from running at the same time, which the
/// timing report of REQ-E3D-360 needs in order to mean anything.
/// </summary>
[CollectionDefinition(Name)]
public sealed class AssetCollection
{
    /// <summary>The collection name.</summary>
    public const string Name = "assets-neutral";
}

/// <summary>
/// Locates <c>cleanroom/assets-neutral</c>, which holds the 101 built-in profiles, by walking
/// up from the test binary.
/// </summary>
internal static class NeutralAssets
{
    /// <summary>Absolute path of <c>assets-neutral/profiles</c>, or null.</summary>
    public static string? ProfilesDirectory { get; } = Locate();

    /// <summary>True when the profiles folder is available.</summary>
    public static bool Available => ProfilesDirectory is not null;

    /// <summary>The message a skipped test reports.</summary>
    public const string MissingMessage =
        "The extracted assets of cleanroom/assets-neutral are not available next to the solution.";

    /// <summary>The built-in profile files, ordered by file name.</summary>
    public static IReadOnlyList<string> ProfileFiles()
    {
        if (ProfilesDirectory is null)
        {
            return [];
        }

        string[] files = Directory.GetFiles(ProfilesDirectory, "*.bytes");
        Array.Sort(files, StringComparer.Ordinal);
        return files;
    }

    /// <summary>Loads one built-in profile by file name.</summary>
    public static GameProfile Load(string fileName)
    {
        ArgumentNullException.ThrowIfNull(ProfilesDirectory);
        byte[] bytes = File.ReadAllBytes(Path.Combine(ProfilesDirectory, fileName));
        ProfileLoadResult result = ProfileFormat.Load(bytes, sourceName: fileName);
        Assert.True(result.IsSuccess, $"{fileName}: {result.Message}");
        return result.Profile!;
    }

    /// <summary>Loads one built-in profile from its full path.</summary>
    public static GameProfile LoadPath(string path)
    {
        ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
        Assert.True(result.IsSuccess, $"{path}: {result.Message}");
        return result.Profile!;
    }

    private static string? Locate()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName, "assets-neutral", "profiles");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}

/// <summary>A fact skipped, rather than failed, when <c>assets-neutral</c> is missing.</summary>
internal sealed class AssetFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public AssetFactAttribute()
    {
        if (!NeutralAssets.Available)
        {
            Skip = NeutralAssets.MissingMessage;
        }
    }
}
