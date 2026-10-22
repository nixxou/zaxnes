using Silk.NET.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// SPEC-10 as redefined by AMD-010: the new structure REQ-UI-200 to REQ-UI-209, and the functions
/// of SPEC-10 the amendment keeps mandatory (§3) — ROM and profile choice, profile import and
/// deletion, states, reset / resume / stop / quit, the visible settings, the key configuration,
/// the notifications with their exact texts and the about block.
/// </summary>
/// <remarks>
/// AMD-010 §1 makes the presentation and the navigation of SPEC-10 informative and classes any
/// presentation deviation as an accepted divergence <b>(D)</b> in advance, so the tabs, the tile
/// grids, the pagination, the help block, the colours and the reference layout are not judged
/// here (§5, "la présentation n'est pas jugée"). What is judged is the function.
/// </remarks>
public sealed class UiTests
{
    /// <summary>
    /// AMD-010 REQ-UI-200: the five menus, and every entry of the table with the command it
    /// invokes.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_200_the_menu_bar_holds_every_entry_of_the_table()
    {
        using HostDriver host = HostDriver.Create();
        UiDriver bar = UiDriver.MenuBar(host, ["D:\\roms\\a.nes", "D:\\roms\\b.nes"]);

        Assert.Equal<IEnumerable<string>>(
            ["Fichier", "Émulation", "Vidéo", "Audio", "Outils"],
            bar.MenuTitles);

        IReadOnlyList<UiMenuRow> rows = bar.Rows();
        HashSet<string> commands = [];
        foreach (UiMenuRow row in rows)
        {
            if (row.Command is { } name)
            {
                commands.Add(name);
            }
        }

        // Fichier
        Expect("Fichier", ZaxCommands.OpenRom);
        Expect("Fichier", ZaxCommands.OpenRecentRom);
        Expect("Fichier", ZaxCommands.ChooseProfile);
        Expect("Fichier", ZaxCommands.ImportProfile);
        Expect("Fichier", ZaxCommands.LoadSlot);
        Expect("Fichier", ZaxCommands.QuickLoad);
        Expect("Fichier", ZaxCommands.SaveSlot);
        Expect("Fichier", ZaxCommands.QuickSave);
        Expect("Fichier", ZaxCommands.Quit);

        // Émulation
        Expect("Émulation", ZaxCommands.PauseResume);
        Expect("Émulation", ZaxCommands.Reset);
        Expect("Émulation", ZaxCommands.Stop);
        Expect("Émulation", ZaxCommands.FastForward);
        Expect("Émulation", ZaxCommands.Rollback);
        Expect("Émulation", ZaxCommands.SetSpeed);
        Expect("Émulation", ZaxCommands.SetRegion);
        Expect("Émulation", ZaxCommands.ToggleAutoRegion);
        Expect("Émulation", ZaxCommands.SetPortType);

        // Vidéo
        Expect("Vidéo", ZaxCommands.ToggleTwoDimensional);
        Expect("Vidéo", ZaxCommands.SetCameraMode);
        Expect("Vidéo", ZaxCommands.ResetView);
        Expect("Vidéo", ZaxCommands.SetSkybox);
        Expect("Vidéo", ZaxCommands.SetTvBox);
        Expect("Vidéo", ZaxCommands.ToggleShadows);
        Expect("Vidéo", ZaxCommands.ToggleFullscreen);
        Expect("Vidéo", ZaxCommands.ToggleFpsCounter);

        // Audio
        Expect("Audio", ZaxCommands.SetVolume);
        Expect("Audio", ZaxCommands.ToggleMute);

        // « Musique de menu » (audio.toggle_menu_music) is no longer an entry of the Audio menu:
        // AMD-024 REQ-PUB-005, revised on 2026-10-21, removes the menu music. The command stays
        // registered and the setting stays in the model for the compatibility of settings.json,
        // but the menu bar does not offer it — see MenuBarModel.cs.
        Assert.DoesNotContain(ZaxCommands.ToggleMenuMusic, commands);

        // Outils
        Expect("Outils", ZaxCommands.ShowOptions);
        Expect("Outils", ZaxCommands.ShowKeyConfig);
        Expect("Outils", ZaxCommands.ShowDataFolder);
        Expect("Outils", ZaxCommands.ShowAbout);

        // The region submenu offers Auto, NTSC, PAL and Dendy.
        string[] regions = [.. rows
            .Where(static row => row.Command == ZaxCommands.SetRegion)
            .Select(static row => row.Label)];
        Assert.Equal<IEnumerable<string>>(["NTSC", "PAL", "Dendy"], regions);

        // The ports submenu offers Manette and Zapper for each of the two ports.
        Assert.Equal(
            4,
            rows.Count(static row => row.Command == ZaxCommands.SetPortType));

        // The ten recent ROMs are listed by file name.
        Assert.Contains(rows, row => row.Command == ZaxCommands.OpenRecentRom && row.Label == "a.nes");

        void Expect(string menu, string command)
        {
            Assert.Contains(command, commands);
            Assert.Contains(rows, row => row.Menu == menu && row.Command == command);
        }
    }

    /// <summary>
    /// AMD-010 REQ-UI-200, last clause: every entry shows its default shortcut, which it takes
    /// from the command model.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_200_each_entry_shows_its_default_shortcut()
    {
        using HostDriver host = HostDriver.Create();
        UiDriver bar = UiDriver.MenuBar(host);
        IReadOnlyList<UiMenuRow> rows = bar.Rows();

        Assert.Equal("Ctrl+O", Shortcut(rows, ZaxCommands.OpenRom));
        Assert.Equal("Ctrl+Q", Shortcut(rows, ZaxCommands.Quit));
        Assert.Equal("P", Shortcut(rows, ZaxCommands.PauseResume));
        Assert.Equal("Ctrl+R", Shortcut(rows, ZaxCommands.Reset));
        Assert.Equal("F11", Shortcut(rows, ZaxCommands.ToggleTwoDimensional));
        Assert.Equal("Alt+Entrée", Shortcut(rows, ZaxCommands.ToggleFullscreen));
        Assert.Equal("Ctrl+,", Shortcut(rows, ZaxCommands.ShowOptions));

        static string Shortcut(IReadOnlyList<UiMenuRow> rows, string command)
            => rows.First(row => row.Command == command).Shortcut;
    }

    /// <summary>
    /// AMD-010 REQ-UI-200: every command of the table really acts. Each one is invoked through
    /// the model and its effect is observed.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_200_every_command_of_the_table_has_an_effect()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        // Émulation
        host.Run(ZaxCommands.PauseResume);
        Assert.Equal(EmulationState.Paused, host.State);
        host.Run(ZaxCommands.PauseResume);
        Assert.Equal(EmulationState.Running, host.State);

        host.Run(ZaxCommands.SetSpeed, CommandArgs.Of(4));
        Assert.Equal(4, host.Settings.GameSpeed);
        host.Run(ZaxCommands.SpeedUp);
        Assert.Equal(5, host.Settings.GameSpeed);
        host.Run(ZaxCommands.SpeedDown);
        Assert.Equal(4, host.Settings.GameSpeed);
        host.Run(ZaxCommands.SetSpeed, CommandArgs.Of(0));

        host.Run(ZaxCommands.SetRegion, CommandArgs.Of(1));
        Assert.Equal(TvRegionSetting.Pal, host.Settings.TvMode);
        host.Run(ZaxCommands.SetRegion, CommandArgs.Of(0));

        host.Run(ZaxCommands.ToggleAutoRegion);
        Assert.True(host.Settings.AutoTvMode);
        host.Run(ZaxCommands.ToggleAutoRegion);

        host.Run(ZaxCommands.SetPortType, new CommandArgs(1, null, 1));
        Assert.Equal(Zax.Nes.ControllerPortType.Zapper, host.PortType(1));
        host.Run(ZaxCommands.SetPortType, new CommandArgs(1, null, 0));

        host.Run(ZaxCommands.FastForward);
        Assert.True(host.FastForward.IsActive);

        host.Run(ZaxCommands.Rollback);
        Assert.Equal(EmulationState.Running, host.State);

        // Vidéo
        host.Run(ZaxCommands.SetSkybox, CommandArgs.Of(2));
        Assert.Equal(2, host.Settings.Skybox);
        // AMD-025: one model, and the value 3 of an older shortcut still selects it.
        host.Run(ZaxCommands.SetTvBox, CommandArgs.Of(3));
        Assert.Equal(1, host.Settings.TvBox);
        host.Run(ZaxCommands.SetTvBox, CommandArgs.Of(0));
        Assert.Equal(0, host.Settings.TvBox);
        host.Run(ZaxCommands.ToggleShadows);
        Assert.False(host.Settings.Shadow);
        host.Run(ZaxCommands.ToggleShadows);
        host.Run(ZaxCommands.ToggleFpsCounter);
        Assert.True(host.Settings.ShowFps);
        host.Run(ZaxCommands.ToggleFullscreen);
        Assert.True(host.Settings.Fullscreen);
        host.Run(ZaxCommands.ToggleFullscreen);

        host.Run(ZaxCommands.SetCameraMode, CommandArgs.Of(1));
        Assert.Equal(Zax.Render.CameraMode.Fpv, host.Scene.Camera.Mode);
        host.Run(ZaxCommands.SetCameraMode, CommandArgs.Of(0));
        Assert.Equal(Zax.Render.CameraMode.Tpv, host.Scene.Camera.Mode);

        host.Scene.Camera.Current.Angles = new System.Numerics.Vector3(20f, 20f, 0f);
        host.Run(ZaxCommands.ResetView);
        Assert.NotEqual(new System.Numerics.Vector3(20f, 20f, 0f), host.Scene.Camera.Current.Angles);

        host.Run(ZaxCommands.ToggleLocalCamera);
        Assert.True(host.Settings.Model.GetBool(SettingKeys.CameraLocalCam));
        host.Run(ZaxCommands.ToggleLocalCamera);

        // Audio
        host.Run(ZaxCommands.SetVolume, CommandArgs.Of(40));
        Assert.Equal(0.4f, host.Settings.Volume, 3);
        host.Run(ZaxCommands.VolumeUp);
        Assert.Equal(0.45f, host.Settings.Volume, 3);
        host.Run(ZaxCommands.VolumeDown);
        Assert.Equal(0.4f, host.Settings.Volume, 3);
        host.Run(ZaxCommands.ToggleMute);
        Assert.Equal(0f, host.Settings.EffectiveVolume);
        host.Run(ZaxCommands.ToggleMute);
        host.Run(ZaxCommands.ToggleMenuMusic);
        Assert.False(host.Settings.MenuMusic);
        host.Run(ZaxCommands.ToggleMenuMusic);

        // Fichier and the states
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(1));
        Assert.True(host.Slots.Find("1")!.IsOccupied);
        host.Run(ZaxCommands.LoadSlot, CommandArgs.Of(1));
        Assert.Equal(EmulationState.Running, host.State);
        host.Run(ZaxCommands.QuickSave);
        Assert.True(host.Slots.Find("quick")!.IsOccupied);
        host.Run(ZaxCommands.QuickLoad);
        Assert.Equal(EmulationState.Running, host.State);

        host.Run(ZaxCommands.Reset);
        Assert.Equal(EmulationState.Running, host.State);
        host.Run(ZaxCommands.Stop);
        Assert.Equal(EmulationState.Stopped, host.State);
        host.Run(ZaxCommands.Quit);
        Assert.True(host.QuitRequested);
    }

    /// <summary>
    /// AMD-010 REQ-UI-200 « Vidéo ▸ Mode 3D / Mode 2D » as settled by AMD-017 (<c>Q-037</c>): a
    /// <b>hot</b> toggle of the rendering between the 3D scene and the core's pixel frame,
    /// « sans relance ni perte de partie ». The shortcut is <c>F11</c>.
    /// </summary>
    /// <remarks>
    /// New behaviour of the <c>R1-fixes</c> batch: the toggle used to relaunch the ROM on the
    /// "2D Mode" library entry. Verified here on a profile that <b>does</b> carry a pattern, so
    /// that the session half of <c>ShowingTwoDimensional</c> is off and the toggle is observable
    /// in both directions — which the implementers' own test, run on a pattern-less profile,
    /// cannot show. First verified 2026-10-21, <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // AMD-017 (Q-037), REQ-UI-200, REQ-SCN-055
    public void REQ_UI_200_the_two_dimensional_toggle_is_hot_and_loses_nothing()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verify-hot-toggle";
        profile.PermanentPatterns.Add(new ProfilePattern
        {
            TileSize = new Vec2I(1, 1),
            Permanent = true,
            Grid = new PatternGrid(1, 1, [new ProfileTile(0xFFFFFFFFFFFFFFFFUL, 0, TileFlip.None)]),
        });

        using HostDriver host = HostDriver.Create(profile);
        string id = host.EntryIdByTitle("verify-hot-toggle");
        host.Launch(id, host.WriteRom(LoopRom.Counter()));

        // A real 3D profile: the session is not in 2D mode by itself (REQ-SCN-055).
        Assert.False(host.Session!.Engine.TwoDimensionalMode);
        Assert.False(TwoDimensionalView(host));
        Assert.False(ShowingTwoDimensional(host));

        // State the toggle must not lose.
        host.RunFrames(12);
        long frames = host.EmulatedFrames;
        Assert.True(frames > 0);
        object session = host.HostObject.GetType().GetProperty("Session")!.GetValue(host.HostObject)!;
        int launches = host.RecentRoms.Count;

        host.Run(ZaxCommands.ToggleTwoDimensional);

        // On: the pixel frame is shown...
        Assert.True(TwoDimensionalView(host));
        Assert.True(ShowingTwoDimensional(host));

        // ...with no relaunch and nothing lost.
        Assert.False(host.IsLoading);
        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(id, host.CurrentGameId);
        Assert.Equal(frames, host.EmulatedFrames);
        Assert.Same(session, host.HostObject.GetType().GetProperty("Session")!.GetValue(host.HostObject));
        Assert.Equal(launches, host.RecentRoms.Count);

        // And the way back is the 3D rendering of the same profile, again without a reload.
        host.Run(ZaxCommands.ToggleTwoDimensional);
        Assert.False(TwoDimensionalView(host));
        Assert.False(ShowingTwoDimensional(host));
        Assert.Equal(id, host.CurrentGameId);
        Assert.Same(session, host.HostObject.GetType().GetProperty("Session")!.GetValue(host.HostObject));
        Assert.Equal(frames, host.EmulatedFrames);

        // The emulation keeps running across the toggle: the frames go on.
        host.RunFrames(6);
        Assert.True(host.EmulatedFrames > frames);
    }

    private static bool TwoDimensionalView(HostDriver host)
        => (bool)host.HostObject.GetType().GetProperty("TwoDimensionalView")!.GetValue(host.HostObject)!;

    private static bool ShowingTwoDimensional(HostDriver host)
        => (bool)host.HostObject.GetType().GetProperty("ShowingTwoDimensional")!.GetValue(host.HostObject)!;

    /// <summary>
    /// AMD-010 REQ-UI-201 and Q-036: the first opening of a ROM asks for the profile; the second
    /// one starts straight away with the remembered profile.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_201_the_profile_of_a_rom_is_remembered_per_file()
    {
        using TemporaryRoot root = new();
        UserDataSettingsStorage storage = new(root.Paths);
        GameConfigFile games = new(storage);
        games.Load(DateTimeOffset.FromUnixTimeSeconds(1_761_000_000));

        Assert.Null(games.ProfileFor(@"D:\roms\a.nes"));

        games.SetProfileFor(@"D:\roms\a.nes", "42");
        Assert.Equal("42", games.ProfileFor(@"D:\roms\a.nes"));

        games.Save();
        GameConfigFile reread = new(storage);
        reread.Load(DateTimeOffset.FromUnixTimeSeconds(1_761_000_000));
        Assert.Equal("42", reread.ProfileFor(@"D:\roms\a.nes"));

        // The same rules of version, corruption and atomic write as settings.json.
        Assert.Equal(1, GameConfigFile.CurrentSchemaVersion);
        File.WriteAllText(Path.Combine(root.Paths.Root, GameConfigFile.DefaultName), "{{{");
        GameConfigFile corrupt = new(storage);
        Assert.Equal(
            GameConfigFile.UnreadableMessage,
            corrupt.Load(DateTimeOffset.FromUnixTimeSeconds(1_761_000_000)).Message);
        Assert.NotEmpty(Directory.GetFiles(root.Paths.Root, "games.json.corrupt-*"));
    }

    /// <summary>
    /// AMD-010 REQ-UI-201: the ROM dialog filters on the extensions SPEC-07 supports and always
    /// offers "every file"; the profile chooser lists the catalogue, the user profiles and
    /// « 2D Mode », and searches by title.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_201_the_rom_dialog_and_the_profile_chooser()
    {
        using HostDriver host = HostDriver.Create();

        object browser = UiDriver.OpenBrowser(UiDriver.Text("OpenRomTitle"), host.Root);
        Assert.True(UiDriver.BrowserAccepts(browser, "game.nes"));
        Assert.True(UiDriver.BrowserAccepts(browser, "GAME.NES"));
        Assert.True(UiDriver.BrowserAccepts(browser, "archive.zip"));
        Assert.False(UiDriver.BrowserAccepts(browser, "state.sav"));
        Assert.False(UiDriver.BrowserAccepts(browser, "readme.txt"));

        object chooser = UiDriver.OpenChooser(host.Library, @"D:\roms\a.nes");
        IReadOnlyList<string> titles = UiDriver.ChooserTitles(chooser);
        Assert.Equal("2D Mode", titles[0]);
        Assert.True(titles.Count > 10);

        UiDriver.ChooserSearch(chooser, "  ARKA  ");
        IReadOnlyList<string> filtered = UiDriver.ChooserTitles(chooser, filtered: true);
        Assert.All(filtered, title => Assert.Contains("arka", title, StringComparison.OrdinalIgnoreCase));
        Assert.NotEmpty(filtered);
    }

    /// <summary>
    /// AMD-010 REQ-UI-202: the options window groups every user preference SPEC-10 made visible
    /// in the six named tabs, with its bounds, and a tab can be put back to its defaults.
    /// </summary>
    [Fact]
    public void REQ_UI_202_the_options_window_holds_every_visible_user_preference()
    {
        Assert.Equal<IEnumerable<string>>(
            ["Vidéo", "Caméra", "Lumière et ombres", "Audio", "Émulation", "Interface"],
            UiDriver.OptionTabs());

        IReadOnlyList<(string Tab, string Key, string Label, double Minimum, double Maximum)> rows =
            UiDriver.OptionRows();

        // The settings SPEC-10 REQ-UI-061 made visible, by stable key. « Musique de menu »
        // (audio.bg_music) is no longer among them: AMD-024 REQ-PUB-005, revised on
        // 2026-10-21, removes the menu music, so the setting has no effect and the window does
        // not offer it. It stays in the model for the compatibility of settings.json.
        string[] visible =
        [
            SettingKeys.LightIntensity, SettingKeys.AudioVolume,
            SettingKeys.RenderShadow, SettingKeys.VideoFrameRate, SettingKeys.VideoSkipFrame,
            SettingKeys.EmuGameSpeed, SettingKeys.CameraFov, SettingKeys.EmuTvMode,
            SettingKeys.SceneTvBox, SettingKeys.SceneSkybox, SettingKeys.SceneCustomBgColor,
            SettingKeys.UiFullscreen,
        ];

        foreach (string key in visible)
        {
            Assert.Contains(rows, row => row.Key == key);
        }

        // The bounds the interface offers are the ones of SPEC-10 REQ-UI-061.
        Assert.Equal((45d, 95d), Bounds(SettingKeys.CameraFov));
        Assert.Equal((0.5d, 1.5d), Bounds(SettingKeys.LightIntensity));
        Assert.Equal((-8d, 8d), Bounds(SettingKeys.EmuGameSpeed));

        // Every row is a user preference, apart from the region SPEC-10 showed (DEC-09-Q1).
        foreach ((string _, string key, string _, double _, double _) in rows)
        {
            SettingDescriptor descriptor = SettingsCatalogue.Get(key);
            Assert.True(
                descriptor.Scope == SettingScope.User || key == SettingKeys.EmuTvMode,
                $"{key} is of scope {descriptor.Scope}");
        }

        // The "default values" button of the tab.
        SettingsModel model = new();
        model.SetFloat(SettingKeys.CameraFov, 95f);
        UiDriver.ResetOptionTab(model, "Caméra");
        Assert.Equal(70f, model.GetFloat(SettingKeys.CameraFov));

        (double, double) Bounds(string key)
        {
            (string _, string _, string _, double minimum, double maximum) = rows.First(row => row.Key == key);
            return (minimum, maximum);
        }
    }

    /// <summary>
    /// AMD-010 REQ-UI-203: the key configuration offers the 19 actions of SPEC-08 for both
    /// players, the camera actions, the hotkeys <b>and</b> the menu commands, all bindable under
    /// the capture rules of SPEC-08.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_203_the_key_configuration_offers_the_actions_and_the_commands()
    {
        using HostDriver host = HostDriver.Create();
        IReadOnlyList<(string Section, string Label, string Target, int Player)> rows =
            UiDriver.KeyConfigRows(host);

        // The eight NES buttons for each player.
        foreach (int player in (int[])[0, 1])
        {
            foreach (string action in (string[])["Left", "Right", "Up", "Down", "A", "B", "Select", "Start"])
            {
                Assert.Contains(rows, row => row.Player == player && row.Target == action);
            }
        }

        // The six camera actions and the five hotkeys.
        foreach (string action in (string[])
            ["RotLeft", "RotRight", "RotUp", "RotDown", "ZoomIn", "ZoomOut",
             "Menu", "RollBack", "FastForward", "QuickSave", "QuickLoad"])
        {
            Assert.Contains(rows, row => row.Target == action);
        }

        // The menu commands, under a target of the form cmd:<name>.
        Assert.Contains(rows, row => row.Target == "cmd:" + ZaxCommands.PauseResume);
        Assert.Contains(rows, row => row.Target == "cmd:" + ZaxCommands.ToggleFullscreen);
        Assert.Contains(rows, row => row.Target == "cmd:" + ZaxCommands.Quit);

        // The commands that need an argument are not offered, since a binding carries none.
        Assert.DoesNotContain(rows, row => row.Target == "cmd:" + ZaxCommands.SaveSlot);
        Assert.DoesNotContain(rows, row => row.Target == "cmd:" + ZaxCommands.SetSpeed);
    }

    /// <summary>
    /// AMD-010 REQ-UI-203, criterion: a menu command bound to a pad button runs when the button
    /// is pressed.
    /// </summary>
    [Fact]
    public void REQ_UI_203_a_menu_command_bound_to_a_pad_button_runs()
    {
        Zax.App.Input.InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        Zax.App.Input.DesktopInputSource source = InputSupport.Source(bindings, hub);

        pad.Press(Zax.Platform.Input.StandardGamepad.South);
        source.Sample(1f / 60f);
        source.EndSample();
        pad.ReleaseAll();

        bindings.Gamepad("Pad").Set(
            Zax.App.Input.BindingTarget.OfCommand(ZaxCommands.PauseResume),
            Zax.Platform.Input.PhysicalControl.Button(Zax.Platform.Input.StandardGamepad.LeftBumper));

        pad.Press(Zax.Platform.Input.StandardGamepad.LeftBumper);
        source.Sample(1f / 60f);
        Assert.Contains(ZaxCommands.PauseResume, source.TriggeredCommands);
        source.EndSample();

        // Held down, it only fires once.
        source.Sample(1f / 60f);
        Assert.DoesNotContain(ZaxCommands.PauseResume, source.TriggeredCommands);
        source.EndSample();
    }

    /// <summary>
    /// AMD-010 REQ-UI-204 with AMD-013 (Q-030): the default shortcut table, and the resolution
    /// that requires the modifiers to match exactly.
    /// </summary>
    [Fact]
    public void REQ_UI_204_the_default_shortcuts_are_the_ones_of_the_table()
    {
        IReadOnlyList<UiShortcut> shortcuts = UiDriver.Shortcuts();

        Expect(Key.O, "Control", ZaxCommands.OpenRom, 0, "Ctrl+O");
        Expect(Key.P, "None", ZaxCommands.PauseResume, 0, "P");
        Expect(Key.R, "Control", ZaxCommands.Reset, 0, "Ctrl+R");
        Expect(Key.F11, "None", ZaxCommands.ToggleTwoDimensional, 0, "F11");
        Expect(Key.Enter, "Alt", ZaxCommands.ToggleFullscreen, 0, "Alt+Entrée");
        Expect(Key.Comma, "Control", ZaxCommands.ShowOptions, 0, "Ctrl+,");
        Expect(Key.Q, "Control", ZaxCommands.Quit, 0, "Ctrl+Q");

        // Q-030: Ctrl+Fn loads the state n, Maj+Fn saves it, for n = 1 to 10.
        Key[] functions =
        [
            Key.F1, Key.F2, Key.F3, Key.F4, Key.F5,
            Key.F6, Key.F7, Key.F8, Key.F9, Key.F10,
        ];

        for (int slot = 1; slot <= 10; slot++)
        {
            Expect(functions[slot - 1], "Control", ZaxCommands.LoadSlot, slot, $"Ctrl+F{slot}");
            Expect(functions[slot - 1], "Shift", ZaxCommands.SaveSlot, slot, $"Maj+F{slot}");
        }

        // A bare Fn stays with the camera actions of SPEC-08 §4.2.
        Assert.DoesNotContain(
            shortcuts,
            shortcut => shortcut.Modifiers == "None" && functions.Contains(shortcut.Key));

        void Expect(Key key, string modifiers, string command, int number, string text)
            => Assert.Contains(
                shortcuts,
                shortcut => shortcut.Key == key
                    && shortcut.Modifiers == modifiers
                    && shortcut.Command == command
                    && shortcut.Number == number
                    && shortcut.Text == text);
    }

    /// <summary>
    /// AMD-010 REQ-UI-205: the command model enumerates every command of REQ-UI-200 and each one
    /// is invocable by name, with its availability.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_205_every_command_is_enumerated_and_invocable_by_name()
    {
        using HostDriver host = HostDriver.Create();
        UiDriver bar = UiDriver.MenuBar(host);

        foreach (UiMenuRow row in bar.Rows())
        {
            if (row.Command is { } command)
            {
                Assert.True(host.Commands.Contains(command), command);
                Assert.NotNull(host.Commands.Find(command));
            }
        }

        // Availability: with no game loaded the emulation commands are greyed out.
        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.False(bar.IsEnabled(ZaxCommands.PauseResume));
        Assert.False(bar.IsEnabled(ZaxCommands.Stop));
        Assert.False(bar.IsEnabled(ZaxCommands.Reset));
        Assert.True(bar.IsEnabled(ZaxCommands.OpenRom));
        Assert.True(bar.IsEnabled(ZaxCommands.Quit));

        host.Launch2DMode();
        Assert.True(bar.IsEnabled(ZaxCommands.PauseResume));
        Assert.True(bar.IsEnabled(ZaxCommands.Stop));

        // Toggles report their tick.
        Assert.True(bar.IsChecked(ZaxCommands.ToggleShadows));
        host.Run(ZaxCommands.ToggleShadows);
        Assert.False(bar.IsChecked(ZaxCommands.ToggleShadows));
        host.Run(ZaxCommands.ToggleShadows);

        // A radio row reports which one is current.
        host.Run(ZaxCommands.SetRegion, CommandArgs.Of(1));
        Assert.True(bar.IsChecked(ZaxCommands.SetRegion, CommandArgs.Of(1)));
        Assert.False(bar.IsChecked(ZaxCommands.SetRegion, CommandArgs.Of(0)));

        // An unknown name is refused, not thrown.
        Assert.False(host.Commands.Invoke("no.such.command"));
    }

    /// <summary>
    /// AMD-010 REQ-UI-206 and SPEC-10 §3.10: the notification area — one notification at a time,
    /// a severity order, a real-time timer and the exact texts.
    /// </summary>
    [Fact]
    public void REQ_UI_206_the_notification_rules_and_the_vector_of_paragraph_5()
    {
        NotificationCenter center = new();
        Assert.Equal(4.0, NotificationCenter.DefaultSeconds);
        Assert.Equal(1.0, NotificationCenter.ShortSeconds);

        // V5 of SPEC-10 §5.
        Assert.True(center.ShowError("X"));
        Assert.Equal("X", center.Current?.Text);

        center.Advance(1.0);
        Assert.False(center.Show("Quick Save"));
        Assert.Equal("X", center.Current?.Text);

        center.Advance(3.1);
        Assert.Null(center.Current);

        center.Advance(0.1);
        Assert.True(center.Show("Roll Back", NotificationCenter.ShortSeconds));
        Assert.Equal("Roll Back", center.Current?.Text);
        center.Advance(1.0);
        Assert.Null(center.Current);

        // An equal severity replaces and restarts the timer.
        center.Show("Quick Save");
        center.Advance(2.0);
        center.Show("Quick Load");
        Assert.Equal("Quick Load", center.Current?.Text);
        Assert.Equal(4.0, center.Remaining, 3);

        // The three severities.
        Assert.Equal(NotificationSeverity.Error, new Notification("x", NotificationSeverity.Error, 1).Severity);
        Assert.Equal(3, Enum.GetValues<NotificationSeverity>().Length);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-082: the four in-game notifications, with their exact texts and durations.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_082_the_four_in_game_notifications()
    {
        using HostDriver host = HostDriver.Create();

        // With no current game a quick save shows nothing at all.
        host.Run(ZaxCommands.QuickSave);
        Assert.Null(host.Notifications.Current);

        host.Launch2DMode();

        host.Run(ZaxCommands.Rollback);
        Assert.Equal("Roll Back", host.Notifications.Current?.Text);
        Assert.InRange(host.Notifications.Remaining, 0.9, 1.0);

        host.Notifications.Clear();
        host.Run(ZaxCommands.FastForward);
        Assert.Equal("Fast Forward", host.Notifications.Current?.Text);
        Assert.InRange(host.Notifications.Remaining, 0.9, 1.0);

        host.Notifications.Clear();
        host.Run(ZaxCommands.QuickSave);
        Assert.Equal("Quick Save", host.Notifications.Current?.Text);
        Assert.InRange(host.Notifications.Remaining, 3.9, 4.0);

        host.Notifications.Clear();
        host.Run(ZaxCommands.QuickLoad);
        Assert.Equal("Quick Load", host.Notifications.Current?.Text);
        Assert.InRange(host.Notifications.Remaining, 3.9, 4.0);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-083: the five load and read failures are notified as errors, with the exact
    /// texts, and the menu stays usable.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_083_the_five_error_texts_are_the_documented_ones()
    {
        Assert.Equal("Corrupted zip file", RomFile.CorruptedZipMessage);
        Assert.Equal("Uncompression is not successful", RomFile.UncompressionFailedMessage);
        Assert.Equal("Invalid rom file.", RomFile.InvalidRomMessage);
        Assert.Equal("The game profile doesn't exist anymore.", SaveStateFile.GameMissingMessage);
        Assert.Equal("Save state used a different core.", Zax.Nes.NesCore.ForeignCoreMessage);
        Assert.Equal("Corrupted save state.", SaveStateFile.CorruptedMessage);

        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;

        // Ten random bytes named .nes.
        string bad = host.WriteRom(new byte[10], "broken.nes");
        host.SetRomPath(id, Path.GetFullPath(bad));
        host.Run(ZaxCommands.LaunchGame, CommandArgs.Of(id));

        Assert.Equal("Invalid rom file.", host.Notifications.Current?.Text);
        Assert.Equal(NotificationSeverity.Error, host.Notifications.Current?.Severity);
        Assert.Equal(EmulationState.Stopped, host.State);

        // The interface keeps working: another command still runs.
        Assert.True(host.Commands.Invoke(ZaxCommands.ToggleShadows));
    }

    /// <summary>
    /// SPEC-10 REQ-UI-084: the three profile-import failures are notified with the exact texts,
    /// oddities included, at <b>normal</b> severity.
    /// </summary>
    [Fact]
    public void REQ_UI_084_the_three_import_texts_keep_their_oddities()
    {
        Assert.Equal(
            "This file is not a 3dn file",
            Zax.Profiles.Catalogue.ProfileImportMessages.NotA3dnFile);

        Assert.Equal(
            "3dn format version 7  is not supported",
            Zax.Profiles.Catalogue.ProfileImportMessages.UnsupportedVersion("7"));

        Assert.Equal(
            @"C:\x\y.3dnhas an obsolate or wrong profile format. "
            + "This profile is not compatible with the current version of Zaxnes.",
            Zax.Profiles.Catalogue.ProfileImportMessages.ObsoleteFormat(@"C:\x\y.3dn"));
    }

    /// <summary>
    /// AMD-010 REQ-UI-207: with no ROM loaded the window shows a welcome screen naming
    /// « Fichier ▸ Ouvrir une ROM » and the recent ROMs.
    /// </summary>
    [Fact]
    public void REQ_UI_207_the_welcome_screen_appears_when_no_game_is_loaded()
    {
        object state = UiDriver.NewState();

        Assert.True(UiDriver.StateFlag(state, "WelcomeVisible"));
        UiDriver.SetStateFlag(state, "HasCurrentGame", true);
        Assert.False(UiDriver.StateFlag(state, "WelcomeVisible"));

        Assert.Equal("Fichier ▸ Ouvrir une ROM", UiDriver.Text("WelcomeHint"));
        Assert.Equal("ROMs récentes", UiDriver.Text("WelcomeRecent"));
        Assert.NotEmpty(UiDriver.Text("WelcomeNoRecent"));
    }

    /// <summary>
    /// AMD-010 REQ-UI-208: ten numbered slots plus the quick state are exposed in the menus, each
    /// showing its date or « vide »; the persistence model keeps SPEC-09's 36.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_208_ten_slots_and_the_quick_state_are_exposed()
    {
        using HostDriver host = HostDriver.Create();
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Launch2DMode();
        host.Now = DateTimeOffset.FromUnixTimeSeconds(1_761_000_000);
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));

        UiDriver bar = UiDriver.MenuBar(host);
        IReadOnlyList<UiMenuRow> rows = bar.Rows();

        Assert.Equal(10, SaveSlotIndex.ExposedSlotCount);
        Assert.Equal(10, rows.Count(static row => row.Command == ZaxCommands.LoadSlot));
        Assert.Equal(10, rows.Count(static row => row.Command == ZaxCommands.SaveSlot));
        Assert.Equal(1, rows.Count(static row => row.Command == ZaxCommands.QuickLoad));
        Assert.Equal(1, rows.Count(static row => row.Command == ZaxCommands.QuickSave));

        // The occupied slot shows a date, the others « vide ».
        Assert.Contains(
            rows,
            row => row.Command == ZaxCommands.LoadSlot && row.Label.Contains("3", StringComparison.Ordinal)
                && !row.Label.Contains("vide", StringComparison.Ordinal));
        Assert.Contains(
            rows,
            row => row.Command == ZaxCommands.LoadSlot && row.Label.Contains("vide", StringComparison.Ordinal));

        // Slot 20 is still loadable through the model, as (D) requires.
        Assert.Equal(36, SaveSlotIndex.NumberedSlotCount);
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(20));
        Assert.True(host.Slots.Find("20")!.IsOccupied);
    }

    /// <summary>
    /// AMD-010 REQ-UI-209: in full screen the menu bar and the rest of the chrome disappear, the
    /// windows opened by a shortcut stay visible, and leaving full screen brings the bar back.
    /// </summary>
    [Fact]
    public void REQ_UI_209_full_screen_hides_the_chrome_and_keeps_the_windows()
    {
        object state = UiDriver.NewState();
        Assert.True(UiDriver.StateFlag(state, "MenuBarVisible"));

        UiDriver.SetStateFlag(state, "Fullscreen", true);
        Assert.False(UiDriver.StateFlag(state, "MenuBarVisible"));
        Assert.False(UiDriver.StateFlag(state, "WelcomeVisible"));
        Assert.False(UiDriver.StateFlag(state, "DebugOverlayVisible"));

        // A window opened by a shortcut is still drawn over the full-screen view.
        UiDriver.SetStateFlag(state, "Options", true);
        Assert.True(UiDriver.StateFlag(state, "AnyWindowOpen"));
        Assert.True(UiDriver.StateFlag(state, "Options"));

        UiDriver.StateCall(state, "CloseAll");
        Assert.False(UiDriver.StateFlag(state, "AnyWindowOpen"));

        UiDriver.SetStateFlag(state, "Fullscreen", false);
        Assert.True(UiDriver.StateFlag(state, "MenuBarVisible"));
    }

    /// <summary>
    /// SPEC-10 REQ-UI-076 with DEC-10-Q3: the about block names the product, its version, the
    /// licence and the two credits.
    /// </summary>
    [Fact]
    public void REQ_UI_076_the_about_block_is_the_one_of_the_decision()
    {
        IReadOnlyList<string> lines = UiDriver.AboutLines();

        Assert.Equal("Zaxnes", lines[0]);
        Assert.StartsWith("Version ", lines[1], StringComparison.Ordinal);
        Assert.Contains("GPL v3", lines);
        Assert.Contains(lines, line => line.Contains("3dSen", StringComparison.Ordinal)
            && line.Contains("Geod Studio", StringComparison.Ordinal));
        Assert.Contains(lines, line => line.Contains("3DNes_Engine", StringComparison.Ordinal));

        // The original's identity is not reused as the product name.
        Assert.DoesNotContain("Version 1.1.2", lines);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-095 and DEC-10-Q7: the frame-rate counter is an option and is off by
    /// default.
    /// </summary>
    [Fact]
    public void REQ_UI_095_the_frame_rate_counter_is_off_by_default()
    {
        SettingsModel model = new();
        Assert.False(model.GetBool(SettingKeys.UiShowFps));

        object state = UiDriver.NewState();
        Assert.False(UiDriver.StateFlag(state, "DebugOverlay"));
        Assert.False(UiDriver.StateFlag(state, "DebugOverlayVisible"));
    }

    /// <summary>
    /// SPEC-10 §3.11, REQ-UI-100 to REQ-UI-105: the file browser — title, trail, folders before
    /// filtered files in alphabetical order, the search on file names only, the validation that
    /// needs a file, and the remembered folder.
    /// </summary>
    [Fact]
    public void REQ_UI_102_the_browser_lists_folders_then_filtered_files()
    {
        string folder = Path.Combine(Path.GetTempPath(), $"zax-browse-{Guid.NewGuid():N}");
        Directory.CreateDirectory(Path.Combine(folder, "x"));
        try
        {
            File.WriteAllText(Path.Combine(folder, "a.NES"), "x");
            File.WriteAllText(Path.Combine(folder, "b.zip"), "x");
            File.WriteAllText(Path.Combine(folder, "c.sav"), "x");
            File.WriteAllText(Path.Combine(folder, "super mario.nes"), "x");
            File.WriteAllText(Path.Combine(folder, "zelda.nes"), "x");

            object browser = UiDriver.OpenBrowser("Ouvrir une ROM", folder);

            IReadOnlyList<(string Name, string Kind)> entries = UiDriver.BrowserEntries(browser);
            Assert.Equal("x", entries[0].Name);
            Assert.Equal("Folder", entries[0].Kind);

            string[] files = [.. entries.Where(static e => e.Kind == "File").Select(static e => e.Name)];
            Assert.Equal<IEnumerable<string>>(["a.NES", "b.zip", "super mario.nes", "zelda.nes"], files);

            // The search filters the files, keeps the folders and ignores case and spaces.
            UiDriver.BrowserSearch(browser, "  MARIO ");
            entries = UiDriver.BrowserEntries(browser);
            Assert.Contains(entries, entry => entry.Name == "x");
            Assert.Contains(entries, entry => entry.Name == "super mario.nes");
            Assert.DoesNotContain(entries, entry => entry.Name == "zelda.nes");

            // The trail starts at the root and has one step per component.
            IReadOnlyList<string> trail = UiDriver.BrowserTrail(browser);
            Assert.True(trail.Count >= 2);
            Assert.Equal("Poste de travail", trail[0]);

            // Reopening clears the search (REQ-UI-103, last sentence).
            object reopened = UiDriver.OpenBrowser("Ouvrir une ROM", folder);
            Assert.Contains(
                UiDriver.BrowserEntries(reopened),
                entry => entry.Name == "zelda.nes");
        }
        finally
        {
            try
            {
                Directory.Delete(folder, recursive: true);
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A locked temporary folder must not fail a verification run.
            }
        }
    }

    /// <summary>
    /// SPEC-10 REQ-UI-037 with AMD-010 §3, which keeps "import <b>and deletion</b> of user
    /// profiles" among the mandatory functions: an interface route deletes a user profile, it
    /// opens the box E8 titled with the game, with the message
    /// <c>Do you want to delete the game and its save states ?</c> and the buttons <c>Yes</c> and
    /// <c>No</c>, and <c>No</c> changes nothing.
    /// </summary>
    /// <remarks>
    /// Round 1: no menu row and no command deleted a user profile
    /// (<c>verify/round1/R1-report-part-c.md</c>). The <c>R1-fixes</c> batch adds the row
    /// « Supprimer le jeu… » on <c>file.delete_game</c>. AMD-010 §1 makes the <i>presentation</i>
    /// informative — a menu row stands in for the small button on the selected tile — so what is
    /// checked here is the function, the exact texts and the two buttons. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-UI-037, REQ-UI-110
    public void REQ_UI_037_a_user_profile_can_be_deleted_from_the_interface()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verify-ui-delete";
        profile.Scripts.Add(new ProfileScript("s", true, "-- none", "*"));

        using HostDriver host = HostDriver.Create(profile);
        string id = host.EntryIdByTitle("verify-ui-delete");
        UiDriver bar = UiDriver.MenuBar(host);

        // The route exists in the menu, and it is the deletion command.
        UiMenuRow[] offered = [.. bar.Rows().Where(static candidate =>
            candidate.Command == ZaxCommands.DeleteGame)];
        UiMenuRow row = Assert.Single(offered);
        Assert.Contains("Supprim", row.Label, StringComparison.OrdinalIgnoreCase);

        // It opens the box E8 rather than deleting straight away, with the exact texts.
        host.HoldConfirmations = true;
        host.Run(ZaxCommands.DeleteGame, CommandArgs.Of(id));

        Confirmation box = Assert.Single(host.Confirmations);
        Assert.Equal("verify-ui-delete", box.Title);
        Assert.Equal("Do you want to delete the game and its save states ?", box.Message);
        Assert.Equal("Yes", box.ConfirmLabel);
        Assert.Equal("No", box.CancelLabel);

        // « No » changes nothing: the entry is still there as long as Yes has not run.
        Assert.NotNull(host.Library.Find(id));

        // « Yes » is what acts.
        box.Confirmed();
        Assert.Null(host.Library.Find(id));
    }

    /// <summary>
    /// SPEC-10 REQ-UI-052: a non-empty slot can be deleted from the interface, through the box E8
    /// titled with the game of the slot, with the message <c>Do you want to delete the slot
    /// &lt;n&gt; ?</c> and the buttons <c>Yes</c> and <c>No</c>; <c>Yes</c> empties the slot.
    /// </summary>
    /// <remarks>
    /// Round 1: PARTIEL — the command <c>state.delete_slot</c> emptied the slot but no menu row
    /// exposed it and no confirmation was asked for
    /// (<c>verify/round1/R1-report-part-c.md</c> §4.2). AMD-010 §1 makes the presentation
    /// informative, so the small button on the selected tile is stood in for by the submenu
    /// « Supprimer un état », which the <c>R1-fixes</c> batch adds together with the box.
    /// Re-verified 2026-10-21, <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-UI-052, REQ-PERS-054, REQ-UI-110
    public void REQ_UI_052_a_slot_can_be_deleted_from_the_interface_after_confirmation()
    {
        using HostDriver host = HostDriver.Create();
        host.Thumbnail = new byte[256 * 192 * 3];
        string id = host.Launch2DMode();
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(2));
        host.Run(ZaxCommands.QuickSave);
        Assert.True(host.Slots.Find("2")!.IsOccupied);

        // Only the occupied slots get a deletion row, the quick slot included.
        UiDriver bar = UiDriver.MenuBar(host);
        IReadOnlyList<UiMenuRow> rows = [.. bar.Rows()
            .Where(static row => row.Command == ZaxCommands.DeleteSlot)];
        Assert.Equal(2, rows.Count);
        Assert.Contains(rows, row => row.Label.Contains("2", StringComparison.Ordinal));
        Assert.True(bar.IsEnabled(ZaxCommands.DeleteSlot, CommandArgs.Of("2")));

        // The box E8 with the exact message of REQ-UI-052 and REQ-PERS-054.
        host.HoldConfirmations = true;
        host.Run(ZaxCommands.DeleteSlot, CommandArgs.Of("2"));

        Confirmation box = Assert.Single(host.Confirmations);
        Assert.Equal(host.Library.Find(id)!.DisplayTitle, box.Title);
        Assert.Equal("Do you want to delete the slot 2 ?", box.Message);
        Assert.Equal("Yes", box.ConfirmLabel);
        Assert.Equal("No", box.CancelLabel);

        // As long as « Yes » has not run, nothing changed.
        Assert.True(host.Slots.Find("2")!.IsOccupied);

        box.Confirmed();
        Assert.False(host.Slots.Find("2")!.IsOccupied);
        Assert.False(File.Exists(host.Paths.SlotStateFile("2")));

        // The row is gone with the slot, and the quick slot's is still there.
        IReadOnlyList<UiMenuRow> after = [.. UiDriver.MenuBar(host).Rows()
            .Where(static row => row.Command == ZaxCommands.DeleteSlot)];
        Assert.Single(after);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-003 as amended by AMD-013 (Q-029): the scene stays visible while the game
    /// is paused, frozen on the last frame, and is not drawn when the game is stopped.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_003_the_scene_follows_the_amended_state_table()
    {
        using HostDriver host = HostDriver.Create();
        Assert.False(host.Scene.GameRunning);

        host.Launch2DMode();
        Assert.True(host.Scene.GameRunning);

        host.Run(ZaxCommands.Pause);
        Assert.True(host.Scene.GameRunning);

        host.Run(ZaxCommands.Stop);
        Assert.False(host.Scene.GameRunning);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-004: the Menu button acts on a short press, in the priority order of the
    /// requirement — a load ignores it (1), a modal window takes it before the emulation does
    /// (2 and 3), then pause (4), resume (5), and nothing when stopped (6).
    /// </summary>
    /// <remarks>
    /// Round 1: cases 1 to 3 failed — a press during a load was deferred instead of ignored, and
    /// the loop never learned that a modal window was open (<c>UiState.IsModal</c> had no
    /// consumer), so Escape paused the game instead of closing the file browser
    /// (<c>verify/round1/R1-report-part-c.md</c>). The <c>R1-fixes</c> batch drops the press while
    /// <c>IsLoading</c> and adds the <c>ModalWindowOpen</c> predicate with the
    /// <c>ui.cancel_modal</c> command. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-UI-004
    public void REQ_UI_004_the_menu_button_follows_the_priority_order()
    {
        using HostDriver host = HostDriver.Create();

        // Case 6: with no game loaded, Escape does nothing.
        Press(host);
        Assert.Equal(EmulationState.Stopped, host.State);

        // Case 1: a press made while the loading indicator is up is ignored, not deferred — so
        // the game is Running, not Paused, once the load ends.
        string id = host.TwoDimensionalModeId!;
        host.SetRomPath(id, Path.GetFullPath(host.WriteRom(LoopRom.Counter())));
        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();
        Assert.True(host.IsLoading);
        Press(host, settle: false);
        Assert.True(host.IsLoading);
        host.Settle();
        host.Tick(1.0 / 60.0);
        Assert.Equal(EmulationState.Running, host.State);

        // Cases 2 and 3: a modal window takes the press before the emulation does. The loop
        // raises ui.cancel_modal and the emulation state does not move.
        host.ModalOpen = true;
        host.UiCalls.Clear();
        Press(host);
        Assert.Equal(EmulationState.Running, host.State);
        Assert.Contains(ZaxCommands.CancelModal, host.UiCalls.Select(static call => call.Command));
        Assert.True(host.Commands.Find(ZaxCommands.CancelModal)!.CanExecute(default));

        // Cases 4 and 5: with no modal window, pause then resume.
        host.ModalOpen = false;
        Press(host);
        Assert.Equal(EmulationState.Paused, host.State);

        Press(host);
        Assert.Equal(EmulationState.Running, host.State);

        // And the command is unavailable when no modal window is open, so no frontend can be
        // asked to close one that is not there.
        Assert.False(host.Commands.Find(ZaxCommands.CancelModal)!.CanExecute(default));

        // The three windows the loop must be told about: the confirmation box E8 (case 2), the
        // file browser E7 and the profile chooser (case 3). The frontend reports them through
        // UiState.IsModal, which is what ModalWindowOpen is wired to.
        foreach (string window in (string[])["Dialog", "FileBrowser", "ProfileChooser"])
        {
            object state = UiDriver.NewState();
            Assert.False(UiDriver.StateFlag(state, "IsModal"));
            UiDriver.SetStateFlag(state, window, true);
            Assert.True(UiDriver.StateFlag(state, "IsModal"));
        }
    }

    /// <summary>
    /// One short press on Menu: held under 0.3 s, then released, then one frame for the queued
    /// command to run (REQ-UI-004, REQ-INP-101).
    /// </summary>
    private static void Press(HostDriver host, bool settle = true)
    {
        host.Tick(0.05, Zax.App.Input.InputAction.Menu);
        host.Tick(0.05, Zax.App.Input.InputAction.None);
        host.Tick(1.0 / 60.0);
        if (settle)
        {
            host.Settle();
        }
    }

    /// <summary>
    /// SPEC-10 REQ-UI-025: no activation of a game, a slot or a save has any effect while a
    /// loading indicator is up — the menu rows are drawn unavailable.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_UI_025_nothing_can_be_activated_while_a_load_is_in_progress()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        host.SetRomPath(id, Path.GetFullPath(host.WriteRom(LoopRom.Counter())));

        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();
        Assert.True(host.IsLoading);

        UiDriver bar = UiDriver.MenuBar(host);
        Assert.False(bar.IsEnabled(ZaxCommands.OpenRom));
        Assert.False(host.Commands.Find(ZaxCommands.LaunchGame)!.CanExecute(CommandArgs.Of(id)));
        Assert.False(bar.IsEnabled(ZaxCommands.PauseResume));
        Assert.False(bar.IsEnabled(ZaxCommands.Reset));
        Assert.False(bar.IsEnabled(ZaxCommands.SaveSlot, CommandArgs.Of(1)));
        Assert.False(bar.IsEnabled(ZaxCommands.ImportProfile));
        Assert.False(bar.IsEnabled(ZaxCommands.ChooseProfile));

        host.Settle();
        Assert.True(bar.IsEnabled(ZaxCommands.PauseResume));
    }

    /// <summary>
    /// SPEC-10 REQ-UI-062: the settings the original hid stay in the model, with their
    /// persistence and their libretro exposure, and none of them is shown with its original
    /// label.
    /// </summary>
    [Fact]
    public void REQ_UI_062_the_hidden_settings_stay_in_the_model()
    {
        foreach (string key in (string[])
            [SettingKeys.VideoGraphicQuality, SettingKeys.VideoAntiAlias,
             SettingKeys.VideoRenderMethod, SettingKeys.VideoResScale])
        {
            Assert.NotNull(SettingsCatalogue.Find(key));
        }

        // The VR settings are in the model and in no tab of the options window.
        IReadOnlyList<(string Tab, string Key, string Label, double Minimum, double Maximum)> rows =
            UiDriver.OptionRows();

        foreach (string key in (string[])
            [SettingKeys.VrRuntime, SettingKeys.VrSceneRatio, SettingKeys.VrMirroring,
             SettingKeys.VrControlMethod])
        {
            Assert.NotNull(SettingsCatalogue.Find(key));
            Assert.DoesNotContain(rows, row => row.Key == key);
        }

        // Graphic quality is profile-scoped, so it is not a preference the window edits.
        Assert.Equal(SettingScope.Profile, SettingsCatalogue.Get(SettingKeys.VideoGraphicQuality).Scope);
        Assert.DoesNotContain(rows, row => row.Key == SettingKeys.VideoGraphicQuality);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-061 and V2: the conversions between the interface positions and the setting
    /// values, which AMD-010 keeps ("tous les réglages visibles").
    /// </summary>
    [Theory]
    [InlineData(0, 0.5f)]
    [InlineData(50, 1.0f)]
    [InlineData(100, 1.5f)]
    public void REQ_UI_061_the_light_slider_maps_to_the_intensity(int position, float intensity)
    {
        SettingsModel model = new();
        model.SetFloat(SettingKeys.LightIntensity, (position / 100f) + 0.5f);
        Assert.Equal(intensity, model.GetFloat(SettingKeys.LightIntensity), 4);
    }

    /// <summary>SPEC-10 REQ-UI-061 and V2: the FOV slider, the volume and the speed.</summary>
    [Fact]
    public void REQ_UI_061_the_other_sliders_map_as_the_vector_says()
    {
        SettingsModel model = new();

        foreach ((int position, float degrees) in (( int, float )[])[(-5, 45f), (0, 70f), (5, 95f), (2, 80f)])
        {
            model.SetFloat(SettingKeys.CameraFov, 70f + (5f * position));
            Assert.Equal(degrees, model.GetFloat(SettingKeys.CameraFov), 4);
        }

        model.SetFloat(SettingKeys.AudioVolume, 37 / 100f);
        Assert.Equal(0.37f, model.GetFloat(SettingKeys.AudioVolume), 4);

        model.SetFloat(SettingKeys.EmuGameSpeed, -8f);
        Assert.Equal(-8f, model.GetFloat(SettingKeys.EmuGameSpeed));
        model.SetFloat(SettingKeys.EmuGameSpeed, 8f);
        Assert.Equal(8f, model.GetFloat(SettingKeys.EmuGameSpeed));

        // The lists of REQ-UI-061, read off the domains.
        Assert.Equal<IEnumerable<string>>(
            ["60fps", "90fps", "120fps"],
            [.. Labels(SettingKeys.VideoFrameRate)]);
        // AMD-025 leaves one television model.
        Assert.Equal<IEnumerable<string>>(
            ["None", "TV"],
            [.. Labels(SettingKeys.SceneTvBox)]);
        Assert.Equal<IEnumerable<string>>(
            ["Default", "Color", "Gradient", "Fantasy"],
            [.. Labels(SettingKeys.SceneSkybox)]);

        static IEnumerable<string> Labels(string key)
            => SettingsCatalogue.Get(key).Domain.Options!.Select(static option => option.Label);
    }
}
