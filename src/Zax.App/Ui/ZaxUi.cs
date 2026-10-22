using System.Numerics;
using ImGuiNET;
using Silk.NET.Input;
using Zax.App.Audio;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles.Catalogue;
using Zax.Render;
using Zax.Settings;

namespace Zax.App.Ui;

/// <summary>What the classic interface needs from the host.</summary>
internal sealed class ZaxUiOptions
{
    /// <summary>The emulation host, whose command model every control drives.</summary>
    public required EmulationHost Host { get; init; }

    /// <summary>The library, for the profile chooser and the profile import.</summary>
    public required LibraryGameCatalog Catalog { get; init; }

    /// <summary>The per-ROM choices and the recent ROMs of <c>games.json</c> (Q-036).</summary>
    public required GameConfigFile Games { get; init; }

    /// <summary>The user root, for « Dossier des données ».</summary>
    public required UserDataPaths Paths { get; init; }

    /// <summary>The scene, for the camera mode submenu.</summary>
    public required SceneState Scene { get; init; }

    /// <summary>The binding model the key configuration window edits.</summary>
    public required InputBindings Bindings { get; init; }

    /// <summary>The desktop input layer, for the capture and the pad list; null in a test.</summary>
    public DesktopInputSource? Input { get; set; }

    /// <summary>The mixer, for the interface sounds of SPEC-11.</summary>
    public AudioMixer? Mixer { get; init; }

    /// <summary>Measured frame rate, for the counter of REQ-UI-095.</summary>
    public Func<float>? FramesPerSecond { get; init; }

    /// <summary>Receives one line per notable event.</summary>
    public Action<string>? Log { get; init; }

    /// <summary>Opens a folder in the desktop's file manager; null disables the menu entry.</summary>
    public Action<string>? RevealFolder { get; init; }
}

/// <summary>
/// The classic emulator interface of AMD-010: the menu bar, the options window, the key
/// configuration, the ROM dialog and its profile chooser, the welcome screen, the about block and
/// the on-screen notifications.
/// </summary>
/// <remarks>
/// <para>
/// Every control goes through the command model of REQ-UI-205 — nothing here changes the
/// emulation by itself — and every window's content comes from a model
/// (<see cref="MenuBarModel"/>, <see cref="OptionsModel"/>, <see cref="KeyConfigModel"/>,
/// <see cref="ProfileChooserModel"/>, <see cref="FileBrowserModel"/>) that the tests drive without
/// an ImGui frame.
/// </para>
/// <para>
/// Full screen hides the chrome and keeps the windows (REQ-UI-209): <see cref="UiState"/> holds
/// that rule, and <see cref="Draw"/> only obeys it.
/// </para>
/// </remarks>
internal sealed partial class ZaxUi
{
    private readonly ZaxUiOptions _options;
    private readonly EmulationHost _host;
    private readonly AppSettings _settings;
    private readonly LibraryGameCatalog _catalog;
    private readonly GameConfigFile _games;
    private readonly Action<string> _log;

    private readonly OptionsModel _optionsModel = new();
    private readonly FileBrowserModel _browser = new();
    private readonly ConfirmDialogModel _confirm = new();
    private readonly KeyConfigModel _keys;
    private readonly List<(string Command, CommandArgs Args)> _shortcutHits = [];
    private ProfileChooserModel _chooser;
    private BrowserPurpose _purpose;
    private OptionsTab _tab = OptionsTab.Video;
    private float _fpsShown;
    private float _fpsCountdown;

    /// <summary>Creates the interface.</summary>
    public ZaxUi(ZaxUiOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);

        _options = options;
        _host = options.Host;
        _settings = options.Host.Settings;
        _catalog = options.Catalog;
        _games = options.Games;
        _log = options.Log ?? (static _ => { });

        _chooser = new ProfileChooserModel(options.Catalog.Library);

        // REQ-UI-205: the two commands the interface owns join the same model, so a key or a pad
        // button can reach them like any other. They are registered before the key configuration
        // is built, so that window offers them too (REQ-UI-203).
        _host.Commands.Register(
            UiCommands.ToggleDebugOverlay,
            "Panneau de statistiques",
            _ => State.DebugOverlay = !State.DebugOverlay,
            MenuShortcuts.TextOf(UiCommands.ToggleDebugOverlay),
            isChecked: () => State.DebugOverlay);

        _host.Commands.Register(
            UiCommands.CloseWindows,
            "Fermer les fenêtres",
            _ => State.CloseAll(),
            canExecute: _ => State.AnyWindowOpen);

        _keys = new KeyConfigModel(options.Host.Commands);
        MenuBar = MenuBarModel.Build(_host, () => _games.RecentRoms, options.Scene.Camera);
    }

    /// <summary>What the file dialog was opened for.</summary>
    private enum BrowserPurpose
    {
        /// <summary>Choosing a ROM (REQ-UI-201).</summary>
        Rom,

        /// <summary>Choosing a <c>.3dn</c> profile to import.</summary>
        Profile,

        /// <summary>Choosing an image to use as cover art (REQ-PERS-036).</summary>
        Cover,
    }

    /// <summary>Which windows are open and what full screen hides (REQ-UI-209).</summary>
    public UiState State { get; } = new();

    /// <summary>The menu bar of REQ-UI-200.</summary>
    public MenuBarModel MenuBar { get; }

    /// <summary>The options window of REQ-UI-202.</summary>
    public OptionsModel Options => _optionsModel;

    /// <summary>The key configuration of REQ-UI-203.</summary>
    public KeyConfigModel Keys => _keys;

    /// <summary>The profile chooser of REQ-UI-201.</summary>
    public ProfileChooserModel Chooser => _chooser;

    /// <summary>The file dialog of REQ-UI-201.</summary>
    public FileBrowserModel Browser => _browser;

    /// <summary>The confirmation box E8 of REQ-UI-110.</summary>
    public ConfirmDialogModel Confirm => _confirm;

    /// <summary>
    /// Opens the confirmation box on a request the loop raised (REQ-UI-110). This is what
    /// <c>EmulationHostOptions.Confirm</c> is wired to, and returning true is what tells the
    /// loop not to act straight away.
    /// </summary>
    public bool AskForConfirmation(ConfirmationRequest request)
    {
        _confirm.Open(request);
        State.Dialog = true;
        return true;
    }

    /// <summary>
    /// Height in pixels the chrome takes at the top of the window, so the host can put the game
    /// view underneath it. Zero in full screen (REQ-UI-209).
    /// </summary>
    public float ChromeHeight { get; private set; }

    /// <summary>
    /// Title of a menu to drop open on the next frame, or null. It is how <c>--ui-shots</c>
    /// photographs an open menu without a mouse; nothing else sets it.
    /// </summary>
    public string? ForceOpenMenu { get; set; }

    /// <summary>Opens the profile chooser on no particular ROM, for <c>--ui-shots</c>.</summary>
    public void OpenProfileChooserForCapture() => OpenProfileChooser(null);

    /// <summary>
    /// Draws one frame of the interface. Called with an ImGui frame already begun.
    /// </summary>
    /// <param name="realSeconds">Real time since the previous frame.</param>
    public void Draw(float realSeconds)
    {
        // The window state is the setting's (REQ-PERS-067); a change made by a shortcut, by the
        // options window or by the menu reaches the interface here.
        State.Fullscreen = _settings.Fullscreen;
        State.HasCurrentGame = _host.CurrentGameId is not null;

        ChromeHeight = State.MenuBarVisible ? DrawMenuBar() : 0f;

        if (State.WelcomeVisible)
        {
            DrawWelcome();
        }

        if (State.Options)
        {
            DrawOptions();
        }

        if (State.KeyConfig)
        {
            DrawKeyConfig();
        }

        if (State.About)
        {
            DrawAbout();
        }

        if (State.FileBrowser)
        {
            DrawFileBrowser();
        }

        if (State.ProfileChooser)
        {
            DrawProfileChooser();
        }

        // REQ-UI-110: the confirmation box is drawn last, so it sits over everything else.
        if (State.Dialog)
        {
            DrawConfirmDialog();
        }

        // REQ-UI-206: the OSD is the one layer full screen keeps.
        DrawNotification();
        DrawFpsCounter(realSeconds);
    }

    /// <summary>
    /// Answers a command the model hands to the interface (<see cref="EmulationHost.UiRequested"/>).
    /// </summary>
    public void HandleUiRequest(string command, CommandArgs args)
    {
        switch (command)
        {
            case ZaxCommands.OpenRom:
                OpenBrowser(BrowserPurpose.Rom);
                break;

            case ZaxCommands.OpenRecentRom:
                OpenRecent(args.Number);
                break;

            case ZaxCommands.ChooseProfile:
                OpenProfileChooser(args.Text ?? CurrentRomPath());
                break;

            case ZaxCommands.ImportProfile:
                OpenBrowser(BrowserPurpose.Profile);
                break;

            case ZaxCommands.ImportCover:
                OpenBrowser(BrowserPurpose.Cover);
                break;

            case ZaxCommands.CancelModal:
                CancelTopmostModal();
                break;

            case ZaxCommands.ShowOptions:
                State.Options = true;
                break;

            case ZaxCommands.ShowKeyConfig:
                State.KeyConfig = true;
                break;

            case ZaxCommands.ShowAbout:
                State.About = true;
                break;

            case ZaxCommands.ShowDataFolder:
                ShowDataFolder();
                break;

            case ZaxCommands.ToggleFullscreen:
                // The host already flipped the setting; the window itself is the caller's job.
                State.Fullscreen = _settings.Fullscreen;
                break;

            default:
                _log($"the interface has nothing to do for '{command}'.");
                break;
        }
    }

    /// <summary>
    /// The default shortcuts of REQ-UI-204: the commands the keys pressed in this frame invoke
    /// are queued in the model, so a shortcut and its menu entry behave identically.
    /// </summary>
    /// <param name="pressed">Keys whose rising edge was seen this frame.</param>
    /// <param name="modifiers">Modifiers held right now.</param>
    public void HandleShortcuts(IReadOnlyCollection<Key> pressed, ShortcutModifiers modifiers)
    {
        _shortcutHits.Clear();
        MenuShortcuts.Resolve(pressed, modifiers, _shortcutHits);

        foreach ((string command, CommandArgs args) in _shortcutHits)
        {
            _host.Enqueue(command, args);
        }
    }

    /// <summary>Closes a capture in progress, which closing the key configuration does.</summary>
    public void CloseCapture() => _options.Input?.Rebinding.Close();

    private string? CurrentRomPath()
        => _host.CurrentGameId is { } id ? _catalog.RomPath(id) : null;

    private void OpenBrowser(BrowserPurpose purpose)
    {
        _purpose = purpose;
        _browser.Open(
            purpose switch
            {
                BrowserPurpose.Rom => UiText.OpenRomTitle,
                BrowserPurpose.Cover => UiText.ImportCoverTitle,
                _ => UiText.ImportProfileTitle,
            },
            purpose switch
            {
                BrowserPurpose.Rom => FileBrowserModel.RomFilters,
                BrowserPurpose.Cover => FileBrowserModel.CoverFilters,
                _ => FileBrowserModel.ProfileFilters,
            },
            _settings.LastBrowseDirectory);
        State.FileBrowser = true;
    }

    private void OpenProfileChooser(string? romPath)
    {
        _chooser.Reset(romPath);
        State.ProfileChooser = true;
    }

    private void OpenRecent(int index)
    {
        IReadOnlyList<string> recent = _games.RecentRoms;
        if ((uint)index >= (uint)recent.Count)
        {
            return;
        }

        string path = recent[index];
        if (!File.Exists(path))
        {
            // The file has gone: it leaves the list and the user is told, with the text
            // SPEC-10 REQ-UI-083 fixes for an unusable ROM.
            _games.ForgetRom(path);
            _host.Notifications.ShowError(RomFile.InvalidRomMessage);
            return;
        }

        _host.Enqueue(ZaxCommands.OpenRom, CommandArgs.Of(path));
    }

    /// <summary>
    /// The result of the file dialog: a ROM goes through the open command, a profile through the
    /// library's import (REQ-UI-084 for the message).
    /// </summary>
    private void OnFileChosen(string path)
    {
        // REQ-PERS-066: the folder the dialog closed on is the next one it opens on.
        if (Path.GetDirectoryName(path) is { Length: > 0 } directory)
        {
            _settings.LastBrowseDirectory = directory;
        }

        switch (_purpose)
        {
            case BrowserPurpose.Rom:
                _host.Enqueue(ZaxCommands.OpenRom, CommandArgs.Of(path));
                break;

            case BrowserPurpose.Cover:
                _host.Enqueue(ZaxCommands.ImportCover, CommandArgs.Of(path));
                break;

            default:
                ImportProfile(path);
                break;
        }
    }

    private void ImportProfile(string path)
    {
        ProfileImportResult result = _catalog.ImportProfile(path);

        if (result.Message.Length > 0)
        {
            // REQ-UI-084: an import failure is a notification of <b>normal</b> severity.
            _host.Notifications.Show(result.Message);
            return;
        }

        if (result.Outcome is ProfileImportOutcome.Added or ProfileImportOutcome.Replaced
            && result.Profile is { } profile)
        {
            _log($"profile '{profile.Title}' imported.");
            _options.Mixer?.PlaySound(UiSound.Confirm);

            // The chooser is rebuilt from the library, so a new profile shows up at once.
            RebuildChooser();
        }
    }

    /// <summary>
    /// Rebuilds the chooser's list from the library, which an import has just changed.
    /// </summary>
    private void RebuildChooser()
    {
        string? rom = _chooser.RomPath;
        _chooser = new ProfileChooserModel(_catalog.Library);
        _chooser.Reset(rom);
    }

    /// <summary>
    /// What a short press on Menu does to a modal window (REQ-UI-004): the file browser and the
    /// profile chooser close as with « Cancel » (case 3), and the confirmation box E8 swallows
    /// the press and stays up (case 2, REQ-UI-110). The topmost window is the one that answers,
    /// which is the drawing order of <see cref="Draw"/>.
    /// </summary>
    private void CancelTopmostModal()
    {
        if (State.Dialog)
        {
            // Case 2: « rien ». One leaves the box by its buttons only.
            return;
        }

        if (State.ProfileChooser)
        {
            State.ProfileChooser = false;
            return;
        }

        if (State.FileBrowser)
        {
            State.FileBrowser = false;
        }
    }

    private void ShowDataFolder()
    {
        string root = _options.Paths.Root;
        _log($"user data folder is {root}");

        if (_options.RevealFolder is { } reveal)
        {
            reveal(root);
        }
    }

    private static Vector2 Centre(float width, float height)
    {
        Vector2 display = ImGui.GetIO().DisplaySize;
        return new Vector2((display.X - width) * 0.5f, (display.Y - height) * 0.5f);
    }
}
