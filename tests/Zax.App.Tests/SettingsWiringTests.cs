using Zax.App;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// The settings model as the host actually uses it: the ad-hoc keys are gone, the profile's P
/// settings reach the engine and the scene, and the file keeps the preferences
/// (REQ-PERS-001, REQ-PERS-005, REQ-PERS-007, REQ-PERS-009, REQ-PERS-010, REQ-PERS-012,
/// REQ-PERS-020, REQ-PERS-021).
/// </summary>
public sealed class SettingsWiringTests
{
    /// <summary>
    /// REQ-PERS-001: the host's view reads and writes the one model, so nothing is held twice.
    /// </summary>
    [Fact]
    public void The_host_view_reads_and_writes_the_model()
    {
        SettingsModel model = new();
        AppSettings settings = new(model);

        settings.Fov = 85f;
        Assert.Equal(85f, model.GetFloat(SettingKeys.CameraFov));

        model.SetFloat(SettingKeys.AudioVolume, 0.25f);
        Assert.Equal(0.25f, settings.Volume, 4);
    }

    /// <summary>REQ-PERS-009: the host's change event carries the stable key.</summary>
    [Fact]
    public void A_change_reaches_the_host_with_its_stable_key()
    {
        AppSettings settings = new();
        List<string> seen = [];
        settings.Changed += seen.Add;

        settings.Volume = 0.5f;
        settings.Volume = 0.5f;

        Assert.Equal([SettingKeys.AudioVolume], seen);
    }

    /// <summary>
    /// REQ-PERS-003: the host's view is validated by the model, so a bad value from a menu is
    /// brought back into the domain.
    /// </summary>
    [Fact]
    public void The_host_view_validates_through_the_model()
    {
        AppSettings settings = new()
        {
            FrameRate = 75,
            GameSpeed = 12,
            Skybox = 7,
        };

        Assert.Equal(90, settings.FrameRate);
        Assert.Equal(8, settings.GameSpeed);
        Assert.Equal(-1, settings.Skybox);
    }

    /// <summary>
    /// REQ-PERS-012: the end of a game takes the speed back to 0 and drops the session layer.
    /// </summary>
    [Fact]
    public void The_end_of_a_game_resets_the_speed_and_the_session_layer()
    {
        AppSettings settings = new();
        settings.GameSpeed = 4;
        settings.Model.SetFloat(SettingKeys.CameraSmoothTime, 0.9f, SettingLayer.Session);

        settings.EndOfGameResets();

        Assert.Equal(0, settings.GameSpeed);
        Assert.Equal(0.4f, settings.Model.GetFloat(SettingKeys.CameraSmoothTime), 4);
    }

    /// <summary>
    /// REQ-PERS-020 and REQ-PERS-002: the preferences survive a restart and an absent file
    /// leaves the defaults.
    /// </summary>
    [Fact]
    public void The_preferences_survive_a_restart()
    {
        using TempRoot root = new();

        AppSettings first = new();
        SettingsStore store = new(root.Paths);
        store.Load(first, DateTimeOffset.UnixEpoch);
        Assert.Equal(70f, first.Fov);

        first.Fov = 55f;
        first.Volume = 0.4f;
        first.Fullscreen = true;
        store.Save(first);

        AppSettings second = new();
        new SettingsStore(root.Paths).Load(second, DateTimeOffset.UnixEpoch);

        Assert.Equal(55f, second.Fov);
        Assert.Equal(0.4f, second.Volume, 4);
        Assert.True(second.Fullscreen);
    }

    /// <summary>
    /// REQ-PERS-010: a script's write on a preference never reaches the file, and the
    /// preference is back once the game ends.
    /// </summary>
    [Fact]
    public void A_script_write_on_a_preference_is_never_persisted()
    {
        using TempRoot root = new();

        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);

        settings.Volume = 0.8f;
        settings.Model.SetFloat(SettingKeys.AudioVolume, 0.1f, SettingLayer.Session);
        Assert.Equal(0.1f, settings.Volume, 4);

        store.Save(settings);
        settings.EndOfGameResets();
        Assert.Equal(0.8f, settings.Volume, 4);

        AppSettings reread = new();
        new SettingsStore(root.Paths).Load(reread, DateTimeOffset.UnixEpoch);
        Assert.Equal(0.8f, reread.Volume, 4);
    }

    /// <summary>
    /// REQ-PERS-021: a change made from the UI is written at the latest two seconds later, not
    /// only at the exit.
    /// </summary>
    [Fact]
    public void A_change_from_the_ui_is_written_within_two_seconds()
    {
        using TempRoot root = new();

        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);
        Assert.False(store.HasPendingWrite);

        settings.Fov = 45f;
        Assert.True(store.HasPendingWrite);

        store.Tick(settings, TimeSpan.FromSeconds(10));
        store.Tick(settings, TimeSpan.FromSeconds(12.1));
        Assert.False(store.HasPendingWrite);

        AppSettings reread = new();
        new SettingsStore(root.Paths).Load(reread, DateTimeOffset.UnixEpoch);
        Assert.Equal(45f, reread.Fov);
    }

    /// <summary>
    /// REQ-PERS-022: an unreadable file starts on the defaults, is set aside and reports its
    /// notification to the host.
    /// </summary>
    [Fact]
    public void An_unreadable_file_reports_its_notification_to_the_host()
    {
        using TempRoot root = new();
        File.WriteAllText(Path.Combine(root.Paths.Root, "settings.json"), "{{{");

        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.FromUnixTimeSeconds(1_760_950_000));

        Assert.Equal("Settings file was unreadable and has been reset.", store.Message);
        Assert.Equal(70f, settings.Fov);
        Assert.False(File.Exists(Path.Combine(root.Paths.Root, "settings.json")));
        Assert.NotEmpty(Directory.GetFiles(root.Paths.Root, "settings.json.corrupt-*"));
    }

    /// <summary>
    /// AMD-010 REQ-UI-200 and REQ-UI-201: the recent list and the per-ROM profile choice live
    /// beside the library and survive a restart.
    /// </summary>
    [Fact]
    public void The_recent_roms_and_the_rom_profile_choices_survive_a_restart()
    {
        using TempRoot root = new();

        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);

        store.Games.NoteRomOpened(@"D:\roms\a.nes");
        store.Games.NoteRomOpened(@"D:\roms\b.nes");
        store.Games.SetProfileFor(@"D:\roms\b.nes", "42");
        store.Save(settings);

        Assert.True(File.Exists(Path.Combine(root.Paths.Root, "games.json")));

        SettingsStore reread = new(root.Paths);
        reread.Load(new AppSettings(), DateTimeOffset.UnixEpoch);

        Assert.Equal([@"D:\roms\b.nes", @"D:\roms\a.nes"], reread.Games.RecentRoms);
        Assert.Equal("42", reread.Games.ProfileFor(@"D:\roms\b.nes"));
    }

    /// <summary>
    /// REQ-PERS-072: <c>games.json</c> sits beside <c>library.json</c> and neither disturbs the
    /// other.
    /// </summary>
    [Fact]
    public void The_game_list_coexists_with_the_library()
    {
        using TempRoot root = new();
        File.WriteAllText(root.Paths.LibraryFile, """{ "schemaVersion": 1, "games": [] }""");

        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);
        store.Games.NoteRomOpened(@"D:\roms\a.nes");
        store.Save(settings);

        Assert.Contains("\"games\": []", File.ReadAllText(root.Paths.LibraryFile));
        Assert.Contains("recentRoms", File.ReadAllText(Path.Combine(root.Paths.Root, "games.json")));
    }

    /// <summary>
    /// REQ-PERS-073: a root that cannot be written to does not stop the program and does not
    /// throw.
    /// </summary>
    [Fact]
    public void A_read_only_root_does_not_stop_the_program()
    {
        using TempRoot root = new();

        // A plain file where the root folder should be: creating the layout fails, so the
        // program keeps its settings in memory and the writes are refused (REQ-PERS-073).
        string blocked = Path.Combine(root.Paths.Root, "blocked");
        File.WriteAllText(blocked, "not a folder");
        UserDataPaths paths = UserDataPaths.Resolve(blocked);
        Assert.False(paths.IsWritable);
        Assert.Equal($"User data folder is not writable: {blocked}", paths.UnwritableMessage);

        AppSettings settings = new();
        SettingsStore store = new(paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);
        settings.Fov = 50f;
        store.Save(settings);

        Assert.Equal(50f, settings.Fov);
    }

    /// <summary>
    /// REQ-PERS-005 and REQ-PERS-006 through the loop: launching a different game replaces the
    /// profile's P settings and relaunching the same one does not.
    /// </summary>
    [Fact]
    public void Launching_a_different_game_reapplies_the_profile_settings()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("a", rom);
        harness.Catalog.Add("b", rom);

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("a"));
        SettingsModel model = harness.Settings.Model;

        // A script changes a profile setting during the game.
        model.SetBool(SettingKeys.ConvEmptyTile, true, SettingLayer.Session);
        Assert.True(model.GetBool(SettingKeys.ConvEmptyTile));
        Assert.True(harness.Host.Session!.Engine.Settings.EmptyTile);

        // Relaunching the same game keeps it (REQ-PERS-006).
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("a"));
        Assert.True(model.GetBool(SettingKeys.ConvEmptyTile));

        // A different game takes the profile's value back (REQ-PERS-005).
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("b"));
        Assert.False(model.GetBool(SettingKeys.ConvEmptyTile));
        Assert.False(harness.Host.Session!.Engine.Settings.EmptyTile);
    }

    /// <summary>
    /// REQ-PERS-005 points 3 and 4 through the loop: the NES background flag is forced true and
    /// the layer shadow flags are reset at every load of a different game.
    /// </summary>
    [Fact]
    public void A_game_load_forces_the_background_flag_and_resets_the_layer_shadows()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Catalog.Add("a", harness.WriteRom(SyntheticRom.Counter()));

        SettingsModel model = harness.Settings.Model;
        model.SetBool(SettingKeys.SceneUseNesBgColor, false, SettingLayer.Profile);
        for (int i = 0; i < SettingKeys.LayerCount; i++)
        {
            model.SetBool(SettingKeys.LayerCastShadow(i), true, SettingLayer.Profile);
        }

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("a"));

        Assert.True(model.GetBool(SettingKeys.SceneUseNesBgColor));
        Assert.Equal(
            [false, false, true, false, false, false],
            Enumerable.Range(0, SettingKeys.LayerCount)
                .Select(i => model.GetBool(SettingKeys.LayerCastShadow(i))));
    }

    /// <summary>
    /// REQ-PERS-007: a write reaches the renderer's working copies — the scene and the
    /// conversion slice — without anyone pushing them by hand.
    /// </summary>
    [Fact]
    public void A_write_reaches_the_scene_and_the_conversion_slice()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Catalog.Add("a", harness.WriteRom(SyntheticRom.Counter()));
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("a"));

        SettingsModel model = harness.Settings.Model;

        model.SetFloat(SettingKeys.ClipBottom, 16.1f, SettingLayer.Profile);
        Assert.Equal(16.1f, harness.Host.Session!.Engine.Settings.Clipping.Y, 4);
        Assert.Equal(16.1f, harness.Scene.Clipping.Y, 4);

        model.SetFloat(SettingKeys.LayerOffset(3), 24f, SettingLayer.Profile);
        Assert.Equal(24f, harness.Host.Session!.Engine.Settings.Layer(3).Offset);

        model.SetFloat(SettingKeys.CameraFov, 50f);
        Assert.Equal(50f, harness.Scene.Settings.Fov);

        model.SetFloat(SettingKeys.LightIntensity, 1.4f);
        Assert.Equal(1.4f, harness.Scene.Settings.LightIntensity, 4);
    }

    /// <summary>
    /// REQ-PERS-012 and DEC-09-Q6: both ports are back to Gamepad after a load of a different
    /// game.
    /// </summary>
    [Fact]
    public void A_game_load_puts_both_ports_back_to_gamepad()
    {
        using LoopHarness harness = LoopHarness.Create();
        harness.Catalog.Add("a", harness.WriteRom(SyntheticRom.Counter()));

        SettingsModel model = harness.Settings.Model;
        model.SetInt(SettingKeys.InputPortType(0), 1);

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("a"));

        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(0)));
        Assert.Equal(0, model.GetInt(SettingKeys.InputPortType(1)));
    }

    /// <summary>A throw-away user root.</summary>
    private sealed class TempRoot : IDisposable
    {
        private readonly string _root = Path.Combine(
            Path.GetTempPath(),
            "zax-settings-" + Guid.NewGuid().ToString("N"));

        public TempRoot()
        {
            Directory.CreateDirectory(_root);
            Paths = UserDataPaths.Resolve(_root);
        }

        public UserDataPaths Paths { get; }

        public void Dispose()
        {
            try
            {
                Directory.Delete(_root, recursive: true);
            }
            catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
            {
                // A leftover temporary folder is not a test failure.
            }
        }
    }
}
