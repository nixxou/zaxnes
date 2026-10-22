using System.Numerics;
using ImGuiNET;
using Zax.App.Audio;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.App.Ui;
using Zax.Profiles.Catalogue;
using Zax.Profiles.Library;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// The collection every test that lays out a Dear ImGui frame belongs to.
/// </summary>
/// <remarks>
/// Dear ImGui's current context is a single global, not a per-thread one, so two frames built at
/// the same time in two xUnit threads corrupt it and take the whole process down. Sharing one
/// collection serialises them; the tests that never touch ImGui keep running in parallel.
/// </remarks>
[CollectionDefinition(ImGuiCollection.Name, DisableParallelization = true)]
public sealed class ImGuiCollection
{
    /// <summary>Name of the collection.</summary>
    public const string Name = "dear-imgui";
}

/// <summary>
/// A Dear ImGui context with no graphics backend, so the drawing code of AMD-010 can be run in a
/// test: the frame is built, the widgets are laid out and the draw data is produced, and nothing
/// is ever uploaded to a GPU.
/// </summary>
/// <remarks>
/// It is what makes the interface itself testable rather than only its models: an unbalanced
/// <c>Begin</c>/<c>End</c>, a null dereference or a missing <c>PopID</c> in a window of lot L9
/// fails here. The font atlas is the built-in one; a real font is not needed to lay a frame out.
/// </remarks>
internal sealed class ImGuiHarness : IDisposable
{
    private IntPtr _context;

    /// <summary>Creates the context and makes it current.</summary>
    public ImGuiHarness(float width = 1280f, float height = 720f)
    {
        _context = ImGui.CreateContext();
        ImGui.SetCurrentContext(_context);

        ImGuiIOPtr io = ImGui.GetIO();
        io.DisplaySize = new Vector2(width, height);
        io.DeltaTime = 1f / 60f;
        io.Fonts.AddFontDefault();
        io.Fonts.GetTexDataAsRGBA32(out IntPtr _, out int _, out int _);
        io.Fonts.SetTexID(1);
    }

    /// <summary>Lays out one frame.</summary>
    public void Frame(Action draw)
    {
        ArgumentNullException.ThrowIfNull(draw);

        ImGui.NewFrame();
        try
        {
            draw();
        }
        finally
        {
            ImGui.Render();
        }
    }

    /// <summary>Lays out <paramref name="count"/> frames, which settles the window sizes.</summary>
    public void Frames(int count, Action draw)
    {
        for (int frame = 0; frame < count; frame++)
        {
            Frame(draw);
        }
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_context == IntPtr.Zero)
        {
            return;
        }

        ImGui.DestroyContext(_context);
        _context = IntPtr.Zero;
    }
}

/// <summary>
/// A whole classic interface driven headless: a temporary user root, a library built in memory, a
/// real <see cref="EmulationHost"/> and the <see cref="Zax.App.Ui.ZaxUi"/> of AMD-010 over them,
/// with a fake keyboard and a fake gamepad hub.
/// </summary>
internal sealed class UiHarness : IDisposable
{
    private readonly TemporaryRoot _root = new();
    private double _clock;
    private bool _disposed;

    /// <summary>Builds the harness.</summary>
    /// <param name="titles">Titles of the 3D profiles the library holds, beside « 2D Mode ».</param>
    public UiHarness(params string[] titles)
    {
        Paths = UserDataPaths.Resolve(_root.Path);
        Store = new SettingsStore(Paths);
        Settings = new AppSettings();
        Store.Load(Settings, DateTimeOffset.UnixEpoch);

        Library = GameLibrary.Build(BuildCatalogue(titles));
        Catalog = LibraryGameCatalog.FromLibrary(Library, Paths, Log.Add);

        Mixer = new AudioMixer(48_000, null, new Random(7));
        Host = new EmulationHost(new EmulationHostOptions
        {
            Paths = Paths,
            Settings = Settings,
            Scene = Scene,
            Games = Catalog,
            Mixer = Mixer,
            Clock = () => _clock,
            Now = () => Now,
            Log = Log.Add,
            CaptureThumbnail = () => Thumbnail,
            RomOpened = path => Store.Games.NoteRomOpened(path),
            ProfileForRom = path => Store.Games.ProfileFor(path),
            RomProfileChosen = (path, gameId) => Store.Games.SetProfileFor(path, gameId),

            // REQ-UI-004 and REQ-UI-110: the same wiring the application uses, so the Escape
            // priority order and the confirmation boxes are exercised through the real path.
            ModalWindowOpen = () => Ui?.State.IsModal ?? false,
            Confirm = request => Ui?.AskForConfirmation(request) ?? false,
        });

        Pads = new FakeGamepadHub();
        Input = new DesktopInputSource(Bindings, Pads) { Keyboard = Keyboard };

        Ui = new ZaxUi(new ZaxUiOptions
        {
            Host = Host,
            Catalog = Catalog,
            Games = Store.Games,
            Paths = Paths,
            Scene = Scene,
            Bindings = Bindings,
            Input = Input,
            Mixer = Mixer,
            FramesPerSecond = () => 60f,
            Log = Log.Add,
        });

        Host.UiRequested += Ui.HandleUiRequest;
    }

    /// <summary>The temporary user root.</summary>
    public UserDataPaths Paths { get; }

    /// <summary>The settings store, which owns <c>games.json</c>.</summary>
    public SettingsStore Store { get; }

    /// <summary>The settings the interface reads and writes.</summary>
    public AppSettings Settings { get; }

    /// <summary>The scene the camera lives in.</summary>
    public SceneState Scene { get; } = new();

    /// <summary>The library.</summary>
    public GameLibrary Library { get; }

    /// <summary>The library behind the host.</summary>
    public LibraryGameCatalog Catalog { get; }

    /// <summary>The audio mixer, which plays nothing without a device.</summary>
    public AudioMixer Mixer { get; }

    /// <summary>The emulation host.</summary>
    public EmulationHost Host { get; }

    /// <summary>The binding model the key configuration window edits.</summary>
    public InputBindings Bindings { get; } = new();

    /// <summary>The connected pads.</summary>
    public FakeGamepadHub Pads { get; }

    /// <summary>The keyboard the capture reads.</summary>
    public FakeKeyboard Keyboard { get; } = new();

    /// <summary>The desktop input layer.</summary>
    public DesktopInputSource Input { get; }

    /// <summary>The interface under test.</summary>
    public ZaxUi Ui { get; }

    /// <summary>Lines the host and the interface logged.</summary>
    public List<string> Log { get; } = [];

    /// <summary>The wall clock the host stamps slots with.</summary>
    public DateTimeOffset Now { get; set; } = DateTimeOffset.FromUnixTimeSeconds(1_760_950_000);

    /// <summary>
    /// The 256 × 192 RGB capture the host takes at a pause (REQ-PERS-051); a black one, which
    /// is all the slot and cover-art tests need.
    /// </summary>
    public byte[]? Thumbnail { get; set; } = new byte[256 * 192 * 3];

    /// <summary>The identifier of the « 2D Mode » entry.</summary>
    public string TwoDimensionalModeId => Catalog.TwoDimensionalModeId!;

    /// <summary>Writes a synthetic ROM into the temporary root and returns its path.</summary>
    public string WriteRom(string name = "game.nes")
    {
        string path = Path.Combine(_root.Path, name);
        File.WriteAllBytes(path, SyntheticRom.Counter());
        return path;
    }

    /// <summary>Advances the clock and runs one rendered frame of the loop.</summary>
    public int Tick(double seconds = 1.0 / 60.0)
    {
        _clock += seconds;
        return Host.Tick(seconds, Input.Sample((float)seconds));
    }

    /// <summary>Runs rendered frames until no load is in progress.</summary>
    public void Settle(int maximumTicks = 128)
    {
        for (int tick = 0; tick < maximumTicks && (Host.IsLoading || Host.PendingCommands > 0); tick++)
        {
            Tick(1.0 / 1000.0);
        }
    }

    /// <summary>
    /// Starts <paramref name="gameId"/> on a synthetic ROM and runs until it is Running, which
    /// is what most interface tests need before they can look at a slot or at a toggle.
    /// </summary>
    public void Launch(string gameId, string romName = "game.nes")
    {
        string rom = WriteRom(romName);
        Catalog.SetRomPath(gameId, rom);
        Run(ZaxCommands.LaunchGame, CommandArgs.Of(gameId));
    }

    /// <summary>Queues a command, applies it, then settles any load it started.</summary>
    public void Run(string command, CommandArgs args = default)
    {
        Host.Enqueue(command, args);
        Host.ApplyCommands();
        Settle();
    }

    /// <summary>
    /// The catalogue a test library is built from: the « 2D Mode » entry plus one entry per
    /// title. The profile files do not exist, so each entry resolves to the empty profile, which
    /// is all the interface tests need (REQ-PROF-033).
    /// </summary>
    private static ProfileCatalogue BuildCatalogue(IReadOnlyList<string> titles)
    {
        System.Text.StringBuilder json = new();
        json.Append("{\"editions\":{\"full\":[");
        json.Append(
            $"{{\"id\":\"2d\",\"title\":\"{ProfileCatalogue.TwoDimensionalModeTitle}\",\"file\":\"profiles/2d.bytes\"}}");

        for (int index = 0; index < titles.Count; index++)
        {
            json.Append(
                $",{{\"id\":\"g{index}\",\"title\":\"{titles[index]}\",\"file\":\"profiles/g{index}.bytes\"}}");
        }

        json.Append("]}}");
        return ProfileCatalogue.Parse(json.ToString(), ".", ProfileCatalogue.FullEdition);
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Host.UiRequested -= Ui.HandleUiRequest;
        Host.Dispose();
        Input.Detach();
        _root.Dispose();
    }
}
