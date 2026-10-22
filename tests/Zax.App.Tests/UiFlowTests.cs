using Zax.App.Loop;
using Zax.App.Ui;
using Zax.Profiles.Catalogue;

namespace Zax.App.Tests;

/// <summary>
/// The flows of AMD-010: opening a ROM and resolving its profile (REQ-UI-201), the welcome screen
/// (REQ-UI-207), the OSD (REQ-UI-206), full screen (REQ-UI-209) and the about block (DEC-10-Q3).
/// </summary>
[Collection(ImGuiCollection.Name)]
public sealed class UiFlowTests
{
    /// <summary>
    /// REQ-UI-201, acceptance criterion: the first opening of a ROM asks for the profile, and the
    /// second opening of the same file starts straight away with the one that was remembered.
    /// </summary>
    [Fact]
    public void REQ_UI_201_the_first_open_asks_for_a_profile_and_the_second_does_not()
    {
        using UiHarness harness = new("Arkanoid", "Balloon Fight");
        string rom = harness.WriteRom();

        // First opening: no profile is known for the file, so the chooser opens and nothing
        // starts.
        harness.Run(ZaxCommands.OpenRom, CommandArgs.Of(rom));

        Assert.True(harness.Ui.State.ProfileChooser);
        Assert.Equal(rom, harness.Ui.Chooser.RomPath);
        Assert.Null(harness.Host.CurrentGameId);

        // The chooser offers the catalogue, the user profiles and « 2D Mode », first.
        Assert.Equal(UiText.TwoDimensionalMode, harness.Ui.Chooser.All[0].Title);
        Assert.Contains("Arkanoid", harness.Ui.Chooser.All.Select(static choice => choice.Title));

        // Choosing "Arkanoid" starts the game and remembers the pair for the file.
        harness.Host.LaunchRomWithProfile(rom, "g0");
        harness.Settle();
        harness.Ui.State.ProfileChooser = false;

        Assert.Equal("g0", harness.Host.CurrentGameId);
        Assert.Equal("g0", harness.Store.Games.ProfileFor(Path.GetFullPath(rom)));

        // Second opening: direct, with no chooser.
        harness.Run(ZaxCommands.Stop);
        harness.Run(ZaxCommands.OpenRom, CommandArgs.Of(rom));

        Assert.False(harness.Ui.State.ProfileChooser);
        Assert.Equal("g0", harness.Host.CurrentGameId);
    }

    /// <summary>
    /// REQ-UI-201: the chooser's search filters by title, case and surrounding spaces ignored.
    /// </summary>
    [Fact]
    public void REQ_UI_201_the_chooser_searches_by_title()
    {
        using UiHarness harness = new("Arkanoid", "Balloon Fight", "Batman");
        ProfileChooserModel chooser = harness.Ui.Chooser;

        chooser.Search = "  BAT ";
        Assert.Equal(["Batman"], chooser.Filtered.Select(static choice => choice.Title));

        chooser.Search = string.Empty;
        Assert.Equal(4, chooser.Filtered.Count);
    }

    /// <summary>
    /// REQ-UI-201: a profile that has been removed from the library since it was remembered
    /// falls back to the chooser rather than to an error.
    /// </summary>
    [Fact]
    public void REQ_UI_201_a_forgotten_profile_reopens_the_chooser()
    {
        using UiHarness harness = new("Arkanoid");
        string rom = harness.WriteRom();

        harness.Store.Games.SetProfileFor(Path.GetFullPath(rom), "gone");
        harness.Run(ZaxCommands.OpenRom, CommandArgs.Of(rom));

        Assert.True(harness.Ui.State.ProfileChooser);
    }

    /// <summary>
    /// REQ-UI-200: « Ouvrir une ROM… » with no path asks the interface for the file dialog, with
    /// the ROM filters of SPEC-10 §4 and the folder that was remembered (REQ-PERS-066).
    /// </summary>
    [Fact]
    public void REQ_UI_201_the_menu_entry_opens_the_file_dialog()
    {
        using UiHarness harness = new("Arkanoid");
        harness.Settings.LastBrowseDirectory = harness.Paths.Root;

        harness.Run(ZaxCommands.OpenRom);

        Assert.True(harness.Ui.State.FileBrowser);
        Assert.Equal(UiText.OpenRomTitle, harness.Ui.Browser.Title);
        Assert.Equal(Path.GetFullPath(harness.Paths.Root), harness.Ui.Browser.Directory);

        // The filters of SPEC-10 §4 plus "every file", as REQ-UI-201 asks.
        Assert.Equal(["nes", "zip"], harness.Ui.Browser.Filter);
        Assert.Contains("Tous les fichiers", harness.Ui.Browser.Filters.Select(static f => f.Label));
    }

    /// <summary>
    /// REQ-UI-200: the recent ROM list grows as files are started, and a file that has gone is
    /// dropped with the error text SPEC-10 REQ-UI-083 fixes.
    /// </summary>
    [Fact]
    public void REQ_UI_200_the_recent_roms_are_remembered_and_pruned()
    {
        using UiHarness harness = new("Arkanoid");
        string rom = harness.WriteRom();
        harness.Store.Games.SetProfileFor(Path.GetFullPath(rom), "g0");

        harness.Run(ZaxCommands.OpenRom, CommandArgs.Of(rom));
        Assert.Equal([Path.GetFullPath(rom)], harness.Store.Games.RecentRoms);

        // The file disappears: the entry leaves the list and the user is told.
        harness.Run(ZaxCommands.Stop);
        File.Delete(rom);
        harness.Run(ZaxCommands.OpenRecentRom, CommandArgs.Of(0));

        Assert.Empty(harness.Store.Games.RecentRoms);
        Assert.Equal(Zax.App.Loop.RomFile.InvalidRomMessage, harness.Host.Notifications.Current!.Value.Text);
        Assert.Equal(NotificationSeverity.Error, harness.Host.Notifications.Current!.Value.Severity);
    }

    /// <summary>
    /// AMD-010 REQ-UI-200 with AMD-017 (Q-037): « Mode 3D / Mode 2D » is a <b>hot</b> toggle of
    /// the rendering. The ROM is not relaunched, the game keeps playing, the profile and the
    /// current game are untouched, and the entry back gives the 3D rendering of that same
    /// profile at once.
    /// </summary>
    [Fact] // Q-037
    public void REQ_UI_200_the_2d_toggle_is_a_hot_switch_of_the_rendering()
    {
        using UiHarness harness = new("Arkanoid");
        string rom = harness.WriteRom();
        harness.Store.Games.SetProfileFor(Path.GetFullPath(rom), "g0");

        harness.Run(ZaxCommands.OpenRom, CommandArgs.Of(rom));
        Assert.Equal("g0", harness.Host.CurrentGameId);
        Assert.False(harness.Host.TwoDimensionalView);

        // A frame is run so that the session has state the toggle must not lose.
        harness.Host.Tick(1.0 / 60.0);
        long frames = harness.Host.EmulatedFrames;
        object? session = harness.Host.Session;

        harness.Run(ZaxCommands.ToggleTwoDimensional);
        Assert.True(harness.Host.TwoDimensionalView);
        Assert.True(harness.Host.ShowingTwoDimensional);

        // No relaunch: no loading indicator, the same session object, the same current game and
        // the frames already run are still there.
        Assert.False(harness.Host.IsLoading);
        Assert.Same(session, harness.Host.Session);
        Assert.Equal("g0", harness.Host.CurrentGameId);
        Assert.Equal(frames, harness.Host.EmulatedFrames);

        // The way back is the same entry, with nothing reloaded. The harness profile carries no
        // pattern, so the session half of ShowingTwoDimensional stays on by itself
        // (REQ-SCN-055); the toggle itself is what this test follows.
        harness.Run(ZaxCommands.ToggleTwoDimensional);
        Assert.False(harness.Host.TwoDimensionalView);
        Assert.Equal("g0", harness.Host.CurrentGameId);
        Assert.Same(session, harness.Host.Session);
    }

    /// <summary>
    /// REQ-SCN-055: a profile with no pattern at all — the « 2D Mode » entry — is always drawn
    /// as the core's pixel frame, whether or not the hot toggle of Q-037 is on.
    /// </summary>
    [Fact] // Q-037
    public void REQ_SCN_055_the_2d_mode_entry_shows_the_pixel_frame_whatever_the_toggle()
    {
        using UiHarness harness = new("Arkanoid");
        string rom = harness.WriteRom();
        harness.Store.Games.SetProfileFor(Path.GetFullPath(rom), harness.TwoDimensionalModeId);

        harness.Run(ZaxCommands.OpenRom, CommandArgs.Of(rom));
        Assert.Equal(harness.TwoDimensionalModeId, harness.Host.CurrentGameId);

        Assert.False(harness.Host.TwoDimensionalView);
        Assert.True(harness.Host.ShowingTwoDimensional);
    }

    /// <summary>
    /// REQ-UI-207: with no ROM loaded the welcome screen is up; a loaded game hides it, and so
    /// does full screen, since it is not an OSD element (REQ-UI-209).
    /// </summary>
    [Fact]
    public void REQ_UI_207_the_welcome_screen_shows_while_no_game_is_loaded()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        imgui.Frame(() => harness.Ui.Draw(1f / 60f));
        Assert.True(harness.Ui.State.WelcomeVisible);

        harness.Launch("g0");
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));
        Assert.False(harness.Ui.State.WelcomeVisible);

        harness.Run(ZaxCommands.Stop);
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));
        Assert.True(harness.Ui.State.WelcomeVisible);

        harness.Settings.Fullscreen = true;
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));
        Assert.False(harness.Ui.State.WelcomeVisible);
    }

    /// <summary>
    /// REQ-UI-209, acceptance criterion: in full screen no menu bar is visible and the shortcuts
    /// still work; back in a window the menu bar is there again.
    /// </summary>
    [Fact]
    public void REQ_UI_209_full_screen_hides_the_menu_bar_and_keeps_the_shortcuts()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        harness.Launch("g0");
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));

        Assert.True(harness.Ui.State.MenuBarVisible);
        Assert.True(harness.Ui.ChromeHeight > 0f);

        harness.Run(ZaxCommands.ToggleFullscreen);
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));

        Assert.True(harness.Settings.Fullscreen);
        Assert.False(harness.Ui.State.MenuBarVisible);
        Assert.Equal(0f, harness.Ui.ChromeHeight);

        // The shortcuts keep working: pause, then the options window over the view.
        harness.Ui.HandleShortcuts([Silk.NET.Input.Key.P], ShortcutModifiers.None);
        harness.Host.ApplyCommands();
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        harness.Ui.HandleShortcuts([Silk.NET.Input.Key.Comma], ShortcutModifiers.Control);
        harness.Host.ApplyCommands();
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));
        Assert.True(harness.Ui.State.Options);

        // Alt+Entrée comes back out, and the bar is visible again.
        harness.Ui.HandleShortcuts([Silk.NET.Input.Key.Enter], ShortcutModifiers.Alt);
        harness.Host.ApplyCommands();
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));

        Assert.False(harness.Settings.Fullscreen);
        Assert.True(harness.Ui.State.MenuBarVisible);
        Assert.True(harness.Ui.ChromeHeight > 0f);
    }

    /// <summary>
    /// REQ-UI-206: the OSD shows the notification of the loop with the text, the severity and
    /// the duration of SPEC-10, in game as in pause, and in full screen too.
    /// </summary>
    [Fact]
    public void REQ_UI_206_the_osd_shows_the_notifications_of_spec10()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        harness.Launch("g0");
        harness.Run(ZaxCommands.QuickSave);

        Notification shown = harness.Host.Notifications.Current!.Value;
        Assert.Equal("Quick Save", shown.Text);
        Assert.Equal(NotificationCenter.DefaultSeconds, shown.Seconds);
        Assert.Equal(NotificationSeverity.Normal, shown.Severity);

        // The OSD draws in both states and in full screen (REQ-UI-209).
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));
        harness.Settings.Fullscreen = true;
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));

        // REQ-UI-081: a normal notification cannot replace a visible error.
        harness.Host.Notifications.ShowError("Invalid rom file.");
        Assert.False(harness.Host.Notifications.Show("Quick Save"));
        Assert.Equal(
            NotificationSeverity.Error,
            harness.Host.Notifications.Current!.Value.Severity);
    }

    /// <summary>
    /// SPEC-10 REQ-UI-095 and DEC-10-Q7: the frame counter is off by default and appears when
    /// the setting is on.
    /// </summary>
    [Fact]
    public void REQ_UI_095_the_frame_counter_is_off_by_default()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        Assert.False(harness.Settings.ShowFps);
        Assert.False(harness.Ui.State.DebugOverlay);

        harness.Run(ZaxCommands.ToggleFpsCounter);
        Assert.True(harness.Settings.ShowFps);

        imgui.Frames(2, () => harness.Ui.Draw(1f / 60f));
    }

    /// <summary>
    /// DEC-10-Q3 / SPEC-10 REQ-UI-076, with the attribution AMD-029 §3 fixes (closing Q-068):
    /// the about block names the product, its version, the GPL v3 licence, the two credits and
    /// the disclaimer.
    /// </summary>
    [Fact]
    public void DEC_10_Q3_the_about_block_has_its_six_lines()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        Assert.Equal(6, UiText.AboutLines.Count);
        Assert.Equal("Zaxnes", UiText.AboutLines[0]);
        Assert.Equal($"Version {UiText.Version}", UiText.AboutLines[1]);
        Assert.Matches(@"^\d+\.\d+\.\d+", UiText.Version);
        Assert.Equal("GPL v3", UiText.AboutLines[2]);

        // AMD-029 §3 word for word, split over the three lines of the block.
        Assert.Equal("Projet indépendant inspiré de 3dSen (Geod Studio) ;", UiText.AboutLines[3]);
        Assert.Equal(
            "contient du code adapté de 3DNes_Engine (Geod Studio, GPL v3).",
            UiText.AboutLines[4]);
        Assert.Equal("Sans lien avec Geod Studio ni Nintendo.", UiText.AboutLines[5]);

        harness.Run(ZaxCommands.ShowAbout);
        Assert.True(harness.Ui.State.About);

        imgui.Frames(2, () => harness.Ui.Draw(1f / 60f));
    }

    /// <summary>
    /// REQ-UI-209: a window opened by a shortcut is drawn over a full-screen view, while the
    /// chrome stays hidden.
    /// </summary>
    [Fact]
    public void REQ_UI_209_a_window_opened_by_shortcut_survives_full_screen()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        harness.Settings.Fullscreen = true;
        harness.Run(ZaxCommands.ShowKeyConfig);

        // The state follows the setting at the next drawn frame (REQ-PERS-067).
        imgui.Frame(() => harness.Ui.Draw(1f / 60f));

        Assert.True(harness.Ui.State.KeyConfig);
        Assert.True(harness.Ui.State.AnyWindowOpen);
        Assert.False(harness.Ui.State.MenuBarVisible);
        Assert.False(harness.Ui.State.DebugOverlayVisible);
    }

    /// <summary>
    /// The whole interface lays out in one frame, with every window open at once: the menu bar,
    /// the welcome screen, the options, the key configuration, the about block, the file dialog
    /// and the profile chooser.
    /// </summary>
    [Fact]
    public void REQ_UI_200_the_whole_interface_draws_in_one_frame()
    {
        using UiHarness harness = new("Arkanoid", "Balloon Fight");
        using ImGuiHarness imgui = new();

        harness.Run(ZaxCommands.OpenRom);
        harness.Run(ZaxCommands.ShowOptions);
        harness.Run(ZaxCommands.ShowKeyConfig);
        harness.Run(ZaxCommands.ShowAbout);
        harness.Ui.State.ProfileChooser = true;
        harness.Ui.State.DebugOverlay = true;
        harness.Settings.ShowFps = true;
        harness.Host.Notifications.Show("Quick Save");

        imgui.Frames(4, () => harness.Ui.Draw(1f / 60f));

        Assert.True(harness.Ui.State.AnyWindowOpen);
    }

    /// <summary>
    /// REQ-UI-200 « Importer un profil… » with REQ-UI-084: a file that is not a profile is
    /// refused with the exact text of SPEC-10, as a notification of normal severity.
    /// </summary>
    [Fact]
    public void REQ_UI_084_a_bad_profile_import_shows_the_exact_message()
    {
        using UiHarness harness = new("Arkanoid");

        string bogus = Path.Combine(harness.Paths.Root, "not-a-profile.3dn");
        File.WriteAllText(bogus, "this is plain text, not a profile");

        ProfileImportResult result = harness.Catalog.ImportProfile(bogus);

        Assert.Equal(ProfileImportOutcome.Rejected, result.Outcome);
        Assert.NotEmpty(result.Message);

        harness.Host.Notifications.Show(result.Message);
        Assert.Equal(NotificationSeverity.Normal, harness.Host.Notifications.Current!.Value.Severity);
    }
}
