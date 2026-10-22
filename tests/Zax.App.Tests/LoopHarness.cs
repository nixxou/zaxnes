using Zax.App.Input;
using Zax.App.Audio;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// Synthetic NROM images: a program whose vectors all point at it, which is enough for the core
/// to power on and run frames (SPEC-02 §5.1 style, but built here so the loop tests need no
/// external ROM).
/// </summary>
internal static class SyntheticRom
{
    /// <summary>
    /// A 32 KiB PRG + 8 KiB CHR NROM image.
    /// </summary>
    /// <param name="program">Code placed at <c>$8000</c>; an endless loop by default.</param>
    /// <param name="battery">Whether the header declares battery-backed PRG-RAM.</param>
    public static byte[] Build(byte[]? program = null, bool battery = false)
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2; // 2 × 16 KiB PRG.
        image[5] = 1; // 1 × 8 KiB CHR.
        image[6] = (byte)(0x01 | (battery ? 0x02 : 0x00));

        byte[] code = program is { Length: > 0 } ? program : [0x4C, 0x00, 0x80];
        code.CopyTo(image, 16);

        for (int vector = 0; vector < 3; vector++)
        {
            image[16 + (32 * 1024) - 6 + (vector * 2)] = 0x00;
            image[16 + (32 * 1024) - 5 + (vector * 2)] = 0x80;
        }

        return image;
    }

    /// <summary>
    /// A ROM that increments the RAM byte at <c>$0010</c> for ever, so that the core's state
    /// after N frames is a deterministic function of N.
    /// </summary>
    public static byte[] Counter() => Build([0xE6, 0x10, 0x4C, 0x00, 0x80]);

    /// <summary>
    /// A ROM with battery-backed RAM that writes <paramref name="value"/> to <c>$6000</c> once
    /// and then loops, so the cartridge RAM is non-empty after one frame.
    /// </summary>
    public static byte[] BatteryWriter(byte value = 0x5A)
        => Build([0xA9, value, 0x8D, 0x00, 0x60, 0x4C, 0x05, 0x80], battery: true);
}

/// <summary>A library the loop tests build in memory, behind <see cref="IGameCatalog"/>.</summary>
internal sealed class FakeCatalog : IGameCatalog
{
    private readonly Dictionary<string, Entry> _entries = new(StringComparer.Ordinal);

    /// <summary>Launch stamps recorded by <see cref="MarkPlayed"/>.</summary>
    public List<(string Id, long When)> Played { get; } = [];

    /// <inheritdoc/>
    public string? TwoDimensionalModeId { get; private set; }

    /// <inheritdoc/>
    public string? DefaultThreeDimensionalId { get; set; }

    /// <summary>Adds an entry.</summary>
    public FakeCatalog Add(
        string id,
        string? romPath,
        GameProfile? profile = null,
        bool is2DMode = false,
        string? title = null)
    {
        _entries[id] = new Entry(title ?? id, profile ?? GameProfile.CreateEmpty(), is2DMode, romPath);
        if (is2DMode)
        {
            TwoDimensionalModeId = id;
        }

        return this;
    }

    /// <summary>Removes an entry, as deleting a user profile would.</summary>
    public void Remove(string id) => _entries.Remove(id);

    /// <inheritdoc/>
    public ResolvedGame? Resolve(string gameId) => _entries.TryGetValue(gameId, out Entry? entry)
        ? new ResolvedGame(gameId, entry.Title, entry.Profile, entry.Is2DMode)
        : null;

    /// <inheritdoc/>
    public bool Exists(string gameId) => _entries.ContainsKey(gameId);

    /// <inheritdoc/>
    public string? RomPath(string gameId)
        => _entries.TryGetValue(gameId, out Entry? entry) ? entry.RomPath : null;

    /// <inheritdoc/>
    public string? GameForRom(string romPath)
        => _entries.FirstOrDefault(pair => string.Equals(pair.Value.RomPath, romPath, StringComparison.OrdinalIgnoreCase)).Key;

    /// <inheritdoc/>
    public void SetRomPath(string gameId, string romPath)
    {
        if (_entries.TryGetValue(gameId, out Entry? entry))
        {
            _entries[gameId] = entry with { RomPath = romPath };
        }
    }

    /// <inheritdoc/>
    public void MarkPlayed(string gameId, long unixSeconds) => Played.Add((gameId, unixSeconds));

    /// <inheritdoc/>
    public string? Title(string gameId)
        => _entries.TryGetValue(gameId, out Entry? entry) ? entry.Title : null;

    /// <summary>
    /// Whether an entry is deletable. Every entry a test adds stands for a user profile, except
    /// the "2D Mode" one, which REQ-UI-037 never allows to be deleted.
    /// </summary>
    public bool CanDelete(string gameId)
        => _entries.TryGetValue(gameId, out Entry? entry) && !entry.Is2DMode;

    /// <inheritdoc/>
    public bool DeleteGame(string gameId)
    {
        if (!CanDelete(gameId))
        {
            return false;
        }

        Deleted.Add(gameId);
        _entries.Remove(gameId);
        return true;
    }

    /// <summary>Identifiers <see cref="DeleteGame"/> removed (SPEC-09 REQ-PERS-042).</summary>
    public List<string> Deleted { get; } = [];

    /// <inheritdoc/>
    public bool SetCover(string gameId, string? fileName)
    {
        if (!_entries.TryGetValue(gameId, out Entry? entry))
        {
            return false;
        }

        _entries[gameId] = entry with { Cover = fileName };
        return true;
    }

    /// <inheritdoc/>
    public string? Cover(string gameId)
        => _entries.TryGetValue(gameId, out Entry? entry) ? entry.Cover : null;

    private sealed record Entry(
        string Title,
        GameProfile Profile,
        bool Is2DMode,
        string? RomPath,
        string? Cover = null);
}

/// <summary>
/// A whole emulation loop driven headless: a temporary user root, a manual monotonic clock and
/// an in-memory library. The host never touches OpenGL or the audio device, so the state
/// machine, the pacing, the save states and the audio mixing are all testable here.
/// </summary>
internal sealed class LoopHarness : IDisposable
{
    private double _clock;
    private bool _disposed;

    private LoopHarness(string root, FakeCatalog catalog, SceneState scene, AudioMixer mixer)
    {
        Root = root;
        Catalog = catalog;
        Scene = scene;
        Mixer = mixer;
        Paths = UserDataPaths.Resolve(root);
        Settings = new AppSettings();

        Host = new EmulationHost(new EmulationHostOptions
        {
            Paths = Paths,
            Settings = Settings,
            Scene = scene,
            Games = catalog,
            Mixer = mixer,
            Clock = () => _clock,
            Now = () => Now,
            CaptureThumbnail = () => Thumbnail,
            Log = message => Log.Add(message),
        });
    }

    /// <summary>The temporary user root.</summary>
    public string Root { get; }

    /// <summary>The library.</summary>
    public FakeCatalog Catalog { get; }

    /// <summary>The scene the camera and the scripts live in.</summary>
    public SceneState Scene { get; }

    /// <summary>The audio mixer.</summary>
    public AudioMixer Mixer { get; }

    /// <summary>The layout of the user root.</summary>
    public UserDataPaths Paths { get; }

    /// <summary>The settings the loop reads.</summary>
    public AppSettings Settings { get; }

    /// <summary>The host under test.</summary>
    public EmulationHost Host { get; }

    /// <summary>Lines the host logged.</summary>
    public List<string> Log { get; } = [];

    /// <summary>The wall clock the host stamps slots and launch dates with.</summary>
    public DateTimeOffset Now { get; set; } = DateTimeOffset.FromUnixTimeSeconds(1_760_950_000);

    /// <summary>What the thumbnail callback returns, or null for "cannot capture".</summary>
    public byte[]? Thumbnail { get; set; }

    /// <summary>Current value of the monotonic clock.</summary>
    public double Clock => _clock;

    /// <summary>Creates a harness with a fresh temporary user root.</summary>
    public static LoopHarness Create(FakeCatalog? catalog = null, AudioAssets? assets = null)
    {
        string root = Path.Combine(Path.GetTempPath(), $"zax-loop-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        return new LoopHarness(
            root,
            catalog ?? new FakeCatalog(),
            new SceneState(),
            new AudioMixer(48_000, assets, new Random(1234)));
    }

    /// <summary>Writes a synthetic ROM into the temporary root and returns its path.</summary>
    public string WriteRom(byte[] image, string name = "game.nes")
    {
        string path = Path.Combine(Root, name);
        File.WriteAllBytes(path, image);
        return path;
    }

    /// <summary>Advances the monotonic clock and runs one rendered frame.</summary>
    public int Tick(
        double seconds = 1.0 / 60.0,
        InputAction actions = InputAction.None,
        InputAction playerTwo = InputAction.None)
    {
        _clock += seconds;
        return Host.Tick(seconds, actions, playerTwo);
    }

    /// <summary>
    /// Advances the monotonic clock and runs one rendered frame driven by a full snapshot, which
    /// is what the Zapper and the pointer rules of SPEC-08 need (REQ-INP-120).
    /// </summary>
    public int Tick(double seconds, in InputSnapshot snapshot)
    {
        _clock += seconds;
        return Host.Tick(seconds, snapshot);
    }

    /// <summary>Runs rendered frames until no load is in progress.</summary>
    public void Settle(int maximumTicks = 64)
    {
        for (int tick = 0; tick < maximumTicks && Host.IsLoading; tick++)
        {
            Tick(1.0 / 1000.0);
        }
    }

    /// <summary>
    /// Queues a command, applies it between two emulated frames — so no frame runs because of
    /// it — then runs rendered frames until any load it started is over.
    /// </summary>
    public void Run(string command, CommandArgs args = default)
    {
        Host.Enqueue(command, args);
        Host.ApplyCommands();
        Settle();
    }

    /// <summary>
    /// Runs exactly <paramref name="frames"/> emulated frames through the kernel's
    /// "run one emulated frame with these inputs" operation (REQ-LOOP-062), cycling through
    /// <paramref name="sequence"/>. Bypassing the clock is what makes a replay frame-exact,
    /// which is the comparison REQ-LOOP-060 and REQ-ARCH-012 ask for.
    /// </summary>
    public void RunExactFrames(int frames, params InputAction[] sequence)
    {
        GameSession session = Host.Session
            ?? throw new InvalidOperationException("No session is loaded.");

        InputAction[] inputs = sequence.Length > 0 ? sequence : [InputAction.None];
        for (int frame = 0; frame < frames; frame++)
        {
            session.RunFrame((int)inputs[frame % inputs.Length]);
        }
    }

    /// <summary>Runs about <paramref name="frames"/> emulated frames and returns how many ran.</summary>
    public int RunFrames(int frames, InputAction actions = InputAction.None)
    {
        int ran = 0;
        double period = 1.0 / Math.Max(1.0, Host.Pacer.EffectiveRate);
        for (int step = 0; step < frames * 4 && ran < frames; step++)
        {
            ran += Tick(period, actions);
        }

        return ran;
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Host.Dispose();

        try
        {
            Directory.Delete(Root, recursive: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A locked temporary folder must not fail a test run.
        }
    }
}
