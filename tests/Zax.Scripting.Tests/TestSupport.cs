using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;

namespace Zax.Scripting.Tests;

/// <summary>The tile images the synthetic frames use, as in the SPEC-04 test vectors.</summary>
internal static class Img
{
    /// <summary>A full image whose identity is <paramref name="id"/> (0 to 63).</summary>
    public static TileImageKey Full(int id) => new(ulong.MaxValue, 1UL << id, false, false);

    public static TileImageKey A => Full(0);

    public static TileImageKey B => Full(1);

    public static TileImageKey C => Full(2);

    public static TileImageKey D => Full(3);
}

/// <summary>Builds a batch of the core and hands it to an engine (SPEC-02 REQ-CORE-030).</summary>
internal sealed class Batch
{
    private readonly List<TileRecord> _records = [];
    private readonly byte[] _palette = new byte[32];
    private int _spriteOrd;

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

    /// <summary>Adds a background tile in cell <c>(column + 1, row + 1)</c>.</summary>
    public Batch BgCell(TileImageKey image, int column, int row, int palette = 0)
        => Bg(image, 8 * column, 8 * row, palette);

    /// <summary>Adds a sprite tile at the screen position (x, y), in emission order.</summary>
    public Batch Sprite(TileImageKey image, int x, int y, int palette = 0)
    {
        _records.Add(new TileRecord(
            image.Low,
            image.High,
            (short)x,
            (short)y,
            _spriteOrd++,
            image.FlipX,
            image.FlipY,
            hidden: false,
            (byte)palette,
            background: false));
        return this;
    }

    /// <summary>Converts the batch with <paramref name="engine"/>.</summary>
    public ConversionFrame Convert(ConversionEngine engine, int counter = 0)
    {
        ArgumentNullException.ThrowIfNull(engine);
        FrameBatch batch = new([.. _records], _palette, 0, 0);
        return engine.Convert(batch, counter);
    }

    /// <summary>Converts and publishes the batch (REQ-E3D-280).</summary>
    public ConversionFrame ConvertAndPublish(ConversionEngine engine, int counter = 0)
    {
        ConversionFrame frame = Convert(engine, counter);
        engine.Publish();
        return frame;
    }
}

/// <summary>
/// Builds a profile with scripts and patterns and starts a runtime on it in the order of
/// REQ-LUA-015: chunks, then the profile settings and the patterns with their
/// <c>BuildPattern</c> hooks, then <c>Start</c>.
/// </summary>
internal sealed class ScriptProfileBuilder
{
    private readonly GameProfile _profile = GameProfile.CreateEmpty();

    /// <summary>The profile settings, so that a test can change one.</summary>
    public ProfileSettings Settings => _profile.Settings!;

    /// <summary>Adds a script, in file order.</summary>
    public ScriptProfileBuilder Script(string name, string source, string tags = "*", bool enabled = true)
    {
        _profile.Scripts.Add(new ProfileScript(name, enabled, source, tags));
        return this;
    }

    /// <summary>
    /// Adds a script that runs <b>first</b> and defines no hook, so that the others can share a
    /// global it sets up. Its name sorts before every letter, so it is also first in script
    /// order (REQ-LUA-004).
    /// </summary>
    public ScriptProfileBuilder Prelude(string source)
    {
        _profile.Scripts.Insert(0, new ProfileScript("!prelude", true, source, string.Empty));
        return this;
    }

    /// <summary>Adds a pattern of one row with <paramref name="instanceCount"/> instances.</summary>
    public ScriptProfileBuilder Pattern(
        TileImageKey?[] cells,
        string? tags = null,
        int instanceCount = 1,
        int depth = 3,
        bool permanent = true)
        => Grid(cells.Length, 1, cells, tags, instanceCount, depth, permanent);

    /// <summary>Adds a pattern over a <paramref name="width"/> × <paramref name="height"/> grid.</summary>
    public ScriptProfileBuilder Grid(
        int width,
        int height,
        TileImageKey?[] cells,
        string? tags = null,
        int instanceCount = 1,
        int depth = 3,
        bool permanent = true)
    {
        ArgumentNullException.ThrowIfNull(cells);
        List<ProfileTile?> tiles = [];
        foreach (TileImageKey? cell in cells)
        {
            tiles.Add(cell is { } key ? new ProfileTile(key.Low, key.High, TileFlip.None) : null);
        }

        ProfilePattern pattern = new()
        {
            Grid = new PatternGrid(width, height, tiles),
            TileSize = new Vec2I(width, height),
            Permanent = permanent,
            SelfConnected = true,
            ConstraintSize = new Vec2I(99, 99),
        };

        for (int k = 0; k < instanceCount; k++)
        {
            pattern.Variants.Add(new PatternVariant
            {
                Depth = depth,
                Layer = ProfileLayer.L1,
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

    /// <summary>Starts a session on this profile without calling <c>Start</c>.</summary>
    public ScriptSession Load() => ScriptSession.Create(_profile);

    /// <summary>Starts a session and calls <c>Start</c> of every script (REQ-LUA-020).</summary>
    public ScriptSession Start()
    {
        ScriptSession session = Load();
        session.Runtime.InvokeStart();
        return session;
    }
}

/// <summary>
/// One engine, one runtime and the collected log: what a test drives. It follows the load order
/// of REQ-LUA-015, which is what the host does.
/// </summary>
internal sealed class ScriptSession : IDisposable
{
    private ScriptSession(
        ConversionEngine engine,
        ProfileScriptRuntime runtime,
        CollectingScriptLog log,
        DetachedScriptCamera camera,
        DetachedScriptBackdrop backdrop,
        DetachedScriptInput input,
        DetachedScriptMemory memory,
        List<ScriptNotification> notifications)
    {
        Engine = engine;
        Runtime = runtime;
        Log = log;
        Camera = camera;
        Backdrop = backdrop;
        Input = input;
        Memory = memory;
        Notifications = notifications;
    }

    /// <summary>The conversion engine.</summary>
    public ConversionEngine Engine { get; }

    /// <summary>The Lua runtime.</summary>
    public ProfileScriptRuntime Runtime { get; }

    /// <summary>Everything the runtime logged.</summary>
    public CollectingScriptLog Log { get; }

    /// <summary>The camera the scripts drive.</summary>
    public DetachedScriptCamera Camera { get; }

    /// <summary>The backdrop the scripts drive.</summary>
    public DetachedScriptBackdrop Backdrop { get; }

    /// <summary>The inputs the scripts read.</summary>
    public DetachedScriptInput Input { get; }

    /// <summary>The memory <c>Script:ReadMem</c> reads.</summary>
    public DetachedScriptMemory Memory { get; }

    /// <summary>The notifications the scripts raised.</summary>
    public List<ScriptNotification> Notifications { get; }

    /// <summary>Builds engine and runtime for <paramref name="profile"/> (REQ-LUA-015).</summary>
    public static ScriptSession Create(GameProfile profile)
    {
        ConversionEngine engine = new();
        engine.TwoDimensionalMode = profile.PermanentPatterns.Count == 0
            && profile.TemporaryPatterns.Count == 0;

        CollectingScriptLog log = new();
        DetachedScriptCamera camera = new();
        DetachedScriptBackdrop backdrop = new();
        DetachedScriptInput input = new();
        DetachedScriptMemory memory = new();
        List<ScriptNotification> notifications = [];
        ScriptSettings settings = new(engine.Settings, new DetachedScriptLight());

        ScriptEnvironment environment = new()
        {
            Engine = engine,
            Settings = settings,
            Camera = camera,
            Backdrop = backdrop,
            Input = input,
            Memory = memory,
            Log = log,
            Notify = notifications.Add,
            Profile = new ScriptProfileInfo
            {
                Title = profile.Title ?? string.Empty,
                Maker = profile.Author ?? string.Empty,
                GameDev = profile.Publisher ?? string.Empty,
                Description = profile.Description ?? string.Empty,
                Id = profile.Id ?? string.Empty,
                Version = profile.VersionText ?? string.Empty,
            },
        };

        settings.ViewReset = camera.ResetView;

        // 1. the chunks, 2. the profile settings and 3. the patterns with BuildPattern.
        ProfileScriptRuntime runtime = ProfileScriptRuntime.Create(profile, environment);
        engine.ScriptHost = runtime;
        engine.LoadProfile(profile);

        return new ScriptSession(engine, runtime, log, camera, backdrop, input, memory, notifications);
    }

    /// <summary>Converts one batch, which runs the frame hooks of REQ-LUA-025.</summary>
    public ConversionFrame Frame(Batch batch, int counter = 0)
    {
        ArgumentNullException.ThrowIfNull(batch);
        return batch.ConvertAndPublish(Engine, counter);
    }

    /// <inheritdoc/>
    public void Dispose() => Runtime.Dispose();
}

/// <summary>
/// Locates <c>cleanroom/assets-neutral/profiles</c>, which holds the 101 built-in profiles.
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

    /// <summary>Loads one built-in profile from its full path.</summary>
    public static GameProfile LoadPath(string path)
    {
        ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
        Assert.True(result.IsSuccess, $"{path}: {result.Message}");
        return result.Profile!;
    }

    /// <summary>Loads the built-in profile whose file name starts with <paramref name="prefix"/>.</summary>
    public static GameProfile? LoadNamed(string prefix)
    {
        foreach (string path in ProfileFiles())
        {
            if (Path.GetFileNameWithoutExtension(path).StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return LoadPath(path);
            }
        }

        return null;
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

/// <summary>Keeps the heavy corpus tests from running at the same time.</summary>
[CollectionDefinition(Name)]
public sealed class AssetCollection
{
    /// <summary>The collection name.</summary>
    public const string Name = "assets-neutral";
}
