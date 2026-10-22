using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Builds a profile with scripts and patterns, so that the verifier can exercise SPEC-05 from
/// the spec text alone, and starts a runtime on it in the load order of REQ-LUA-015.
/// </summary>
public sealed class LuaProfile
{
    private readonly GameProfile _profile = GameProfile.CreateEmpty();

    /// <summary>The profile's embedded settings block, so a test can change one value.</summary>
    public ProfileSettings Settings => _profile.Settings!;

    /// <summary>Appends one script, in <b>file</b> order (REQ-LUA-011).</summary>
    /// <param name="name">Script name (field 1).</param>
    /// <param name="source">Lua source (field 3).</param>
    /// <param name="tags">Tag string (field 5); <c>*</c> links to every instance.</param>
    /// <param name="enabled">The <c>enable</c> flag (field 2).</param>
    public LuaProfile Script(string name, string source, string tags = "*", bool enabled = true)
    {
        _profile.Scripts.Add(new ProfileScript(name, enabled, source, tags));
        return this;
    }

    /// <summary>
    /// Adds a permanent pattern over a <paramref name="width"/> × <paramref name="height"/>
    /// grid of letters (a space meaning an empty cell), with
    /// <paramref name="instanceCount"/> instances carrying <paramref name="tags"/>.
    /// </summary>
    public LuaProfile Pattern(
        string[] rows,
        string? tags = null,
        int instanceCount = 1,
        int depth = 3,
        bool permanent = true)
    {
        ArgumentNullException.ThrowIfNull(rows);
        int height = rows.Length;
        int width = rows.Max(static row => row.Length);

        List<ProfileTile?> tiles = [];
        for (int row = 0; row < height; row++)
        {
            for (int column = 0; column < width; column++)
            {
                char name = column < rows[row].Length ? rows[row][column] : ' ';
                if (name == ' ')
                {
                    tiles.Add(null);
                    continue;
                }

                (ulong low, ulong high) = SyntheticBatch.Full(name);
                tiles.Add(new ProfileTile(low, high, TileFlip.None));
            }
        }

        ProfilePattern pattern = new()
        {
            Grid = new PatternGrid(width, height, tiles),
            TileSize = new Vec2I(width, height),
            Permanent = permanent,
            SelfConnected = true,
            ConstraintSize = new Vec2I(99, 99),
        };

        for (int instance = 0; instance < instanceCount; instance++)
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

    /// <summary>The profile as built.</summary>
    public GameProfile Build() => _profile;

    /// <summary>Loads the profile without calling <c>Start</c> (REQ-LUA-015 steps 1 to 3).</summary>
    public LuaRun Load() => LuaRun.Create(_profile);

    /// <summary>Loads the profile and calls <c>Start</c> of every script (REQ-LUA-020).</summary>
    public LuaRun Start()
    {
        LuaRun run = Load();
        run.Runtime.InvokeStart();
        return run;
    }
}

/// <summary>
/// One conversion engine, one Lua runtime and the log, built in the order of REQ-LUA-015 — the
/// order the host follows. Everything the runtime needs from the host is a detached stand-in, so
/// a test needs no window, no ROM and no renderer.
/// </summary>
public sealed class LuaRun : IDisposable
{
    private LuaRun(
        ConversionEngine engine,
        ProfileScriptRuntime runtime,
        CollectingScriptLog log,
        DetachedScriptCamera camera,
        DetachedScriptBackdrop backdrop,
        DetachedScriptInput input,
        DetachedScriptMemory memory,
        ScriptSettings settings,
        List<ScriptNotification> notifications)
    {
        Engine = engine;
        Runtime = runtime;
        Log = log;
        Camera = camera;
        Backdrop = backdrop;
        Input = input;
        Memory = memory;
        Settings = settings;
        Notifications = notifications;
    }

    /// <summary>The conversion engine the frame queries read.</summary>
    public ConversionEngine Engine { get; }

    /// <summary>The Lua runtime under test.</summary>
    public ProfileScriptRuntime Runtime { get; }

    /// <summary>Everything the runtime logged (<c>print</c>, errors, warnings).</summary>
    public CollectingScriptLog Log { get; }

    /// <summary>The camera the scripts drive.</summary>
    public DetachedScriptCamera Camera { get; }

    /// <summary>The backdrop the scripts drive.</summary>
    public DetachedScriptBackdrop Backdrop { get; }

    /// <summary>The inputs the scripts read.</summary>
    public DetachedScriptInput Input { get; }

    /// <summary>The CPU address space <c>Script:ReadMem</c> reads.</summary>
    public DetachedScriptMemory Memory { get; }

    /// <summary>The settings model exposed as <c>Setting</c>.</summary>
    public ScriptSettings Settings { get; }

    /// <summary>The notifications <c>NotiCenter</c> raised.</summary>
    public List<ScriptNotification> Notifications { get; }

    /// <summary>Builds engine and runtime for <paramref name="profile"/>.</summary>
    public static LuaRun Create(GameProfile profile)
    {
        ArgumentNullException.ThrowIfNull(profile);

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

        // REQ-LUA-015, in the order the requirement fixes: (1) every chunk runs, (2) the
        // profile's embedded settings are applied — they overwrite what a chunk set at its main
        // level — and (3) the patterns are registered and built, which runs BuildPattern.
        // Start is the caller's business (step 4).
        ProfileScriptRuntime runtime = ProfileScriptRuntime.Create(profile, environment);
        settings.Model.ApplyProfile(
            profile.Settings is { } embedded
                ? ProfileSettingApplication.ToSettingValues(embedded)
                : []);
        engine.ScriptHost = runtime;
        engine.LoadProfile(profile, applySettings: false);

        return new LuaRun(engine, runtime, log, camera, backdrop, input, memory, settings, notifications);
    }

    /// <summary>Converts and publishes one batch, which runs the frame hooks of REQ-LUA-025.</summary>
    public ConversionFrame Frame(SyntheticBatch batch, int counter = 1)
    {
        ArgumentNullException.ThrowIfNull(batch);
        ConversionFrame frame = batch.Convert(Engine, counter);
        Engine.Publish();
        return frame;
    }

    /// <summary>Runs a chunk in the profile's own Lua state.</summary>
    public void Do(string chunk) => Runtime.Evaluate(chunk);

    /// <summary>Runs <c>return &lt;expression&gt;</c> and gives the result back as a double.</summary>
    public double Num(string expression) => Runtime.EvaluateNumber("return " + expression);

    /// <summary>Runs <c>return &lt;expression&gt;</c> and gives the result back as text.</summary>
    public string Text(string expression) => Runtime.EvaluateText("return " + expression);

    /// <summary>Runs <c>return &lt;expression&gt;</c> and reads the result as a truth value.</summary>
    public bool Flag(string expression) => Runtime.EvaluateNumber($"return ({expression}) and 1 or 0") != 0.0;

    /// <summary>The Lua type name of an expression.</summary>
    public string TypeOf(string expression) => Runtime.EvaluateText($"return type({expression})");

    /// <summary>The message of the Lua error an expression raises, or null when it raises none.</summary>
    public string? Error(string chunk)
    {
        try
        {
            Runtime.Evaluate(chunk);
            return null;
        }
        catch (LuaScriptException exception)
        {
            return exception.Message;
        }
    }

    /// <summary>Asserts that <paramref name="chunk"/> raises a Lua error and returns its message.</summary>
    public string MustFail(string chunk)
    {
        string? message = Error(chunk);
        Assert.NotNull(message);
        return message;
    }

    /// <inheritdoc/>
    public void Dispose() => Runtime.Dispose();
}

/// <summary>
/// Locates the 101 built-in profiles of <c>assets-neutral/profiles</c> and loads them, for the
/// corpus criteria of REQ-LUA-001, 004, 051 and 100 (DEC-R2-T7).
/// </summary>
public static class ProfileCorpus
{
    /// <summary>Absolute path of <c>assets-neutral/profiles</c>, or null.</summary>
    public static string? Directory { get; } = Locate();

    /// <summary>Whether the folder is available next to the solution.</summary>
    public static bool Available => Directory is not null;

    /// <summary>The built-in profile files, ordered by file name.</summary>
    public static IReadOnlyList<string> Files()
    {
        if (Directory is null)
        {
            return [];
        }

        string[] files = System.IO.Directory.GetFiles(Directory, "*.bytes");
        Array.Sort(files, StringComparer.Ordinal);
        return files;
    }

    /// <summary>Loads one profile from its full path.</summary>
    public static GameProfile Load(string path)
    {
        ProfileLoadResult result = ProfileFormat.Load(File.ReadAllBytes(path), sourceName: path);
        Assert.True(result.IsSuccess, $"{path}: {result.Message}");
        return result.Profile!;
    }

    /// <summary>Loads the profile whose file name starts with <paramref name="prefix"/>, or null.</summary>
    public static GameProfile? Named(string prefix)
    {
        foreach (string path in Files())
        {
            if (Path.GetFileNameWithoutExtension(path).StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return Load(path);
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
            if (System.IO.Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        return null;
    }
}

/// <summary>A fact skipped, not failed, when <c>assets-neutral/profiles</c> is missing.</summary>
/// <remarks>
/// AMD-024 REQ-PUB-001 makes this the gate of the <b>opt-in</b> half of the suite: the 101
/// embedded profiles are not part of the publishable project any more, so every test that needs
/// one is skipped when they are not there and carries
/// <c>[Trait("Requires", "OriginalAssets")]</c> so that it can be filtered out explicitly —
/// <c>dotnet test --filter "Requires!=OriginalAssets"</c> is the public suite.
/// </remarks>
public sealed class CorpusFactAttribute : FactAttribute
{
    /// <summary>The message a skipped test reports.</summary>
    public const string MissingMessage =
        "assets-neutral/profiles is not available next to the solution.";

    /// <summary>Creates the attribute.</summary>
    public CorpusFactAttribute()
    {
        if (!ProfileCorpus.Available)
        {
            Skip = MissingMessage;
        }
    }
}

/// <summary>The theory form of <see cref="CorpusFactAttribute"/>.</summary>
public sealed class CorpusTheoryAttribute : TheoryAttribute
{
    /// <summary>Creates the attribute.</summary>
    public CorpusTheoryAttribute()
    {
        if (!ProfileCorpus.Available)
        {
            Skip = CorpusFactAttribute.MissingMessage;
        }
    }
}

/// <summary>A fact that needs the embedded profiles <b>and</b> a GL 3.3 core context.</summary>
[Xunit.Sdk.XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class CorpusGlFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public CorpusGlFactAttribute()
    {
        if (GlAvailability.Reason is { } reason)
        {
            Skip = reason;
        }
        else if (!ProfileCorpus.Available)
        {
            Skip = CorpusFactAttribute.MissingMessage;
        }
    }
}

/// <summary>A fact that needs the embedded profiles <b>and</b> one named dump of the user.</summary>
[Xunit.Sdk.XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class CorpusRomFactAttribute : FactAttribute
{
    /// <summary>Requires <paramref name="rom"/> and the embedded profiles.</summary>
    public CorpusRomFactAttribute(string rom)
    {
        if (!UserRoms.Has(rom))
        {
            Skip = UserRoms.MissingReason ?? $"user ROM not available: {UserRoms.Path(rom)}.";
        }
        else if (!ProfileCorpus.Available)
        {
            Skip = CorpusFactAttribute.MissingMessage;
        }
    }
}

/// <summary>
/// A fact that needs the embedded profiles, a GL context and at least one dump of the user.
/// </summary>
[Xunit.Sdk.XunitTestCaseDiscoverer("Xunit.Sdk.FactDiscoverer", "xunit.execution.dotnet")]
[AttributeUsage(AttributeTargets.Method, AllowMultiple = false)]
public sealed class CorpusAnyRomGlFactAttribute : FactAttribute
{
    /// <summary>Creates the attribute.</summary>
    public CorpusAnyRomGlFactAttribute()
    {
        if (GlAvailability.Reason is { } reason)
        {
            Skip = reason;
        }
        else if (UserRoms.MissingReason is { } missing)
        {
            Skip = missing;
        }
        else if (!ProfileCorpus.Available)
        {
            Skip = CorpusFactAttribute.MissingMessage;
        }
    }
}
