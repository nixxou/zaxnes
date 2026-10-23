using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.App.Ui;
using Silk.NET.Input;

namespace Zax.App.Tests;

/// <summary>
/// The menu bar of AMD-010 REQ-UI-200 and the default shortcuts of REQ-UI-204, checked on the
/// models the interface draws (REQ-UI-205).
/// </summary>
public sealed class MenuBarTests
{
    /// <summary>
    /// REQ-UI-200 with AMD-035: the five menus exist, with the titles the amendment writes —
    /// in English by default, and in French once the setting says so.
    /// </summary>
    [Fact]
    public void REQ_UI_200_the_five_menus_exist_with_their_titles()
    {
        using UiLanguageScope language = new();
        using UiHarness harness = new("Arkanoid");

        Assert.Equal(
            ["File", "Emulation", "Video", "Audio", "Tools"],
            harness.Ui.MenuBar.Menus.Select(static menu => menu.Title));

        harness.Host.Settings.Language = "fr";

        Assert.Equal(
            ["Fichier", "Émulation", "Vidéo", "Audio", "Outils"],
            harness.Ui.MenuBar.Menus.Select(static menu => menu.Title));
    }

    /// <summary>
    /// REQ-UI-200: every entry of the amendment's table exists, in the right menu, and names a
    /// command of the model. Submenus are expanded, so the recent ROMs, the state slots, the
    /// speeds, the regions, the ports, the camera modes, the skyboxes, the TV boxes and the
    /// volumes are all covered.
    /// </summary>
    [Fact]
    public void REQ_UI_200_every_entry_of_the_table_exists_and_names_a_command()
    {
        using UiHarness harness = new("Arkanoid");
        MenuBarModel bar = harness.Ui.MenuBar;

        // One recent ROM, so the submenu of REQ-UI-200 has a row rather than its « vide » note.
        harness.Store.Games.NoteRomOpened("C:\\roms\\game.nes");

        // One expectation per row of the table of REQ-UI-200: the menu it belongs to and the
        // command it must invoke.
        (string Menu, string Command)[] expected =
        [
            (UiText.MenuFile, ZaxCommands.OpenRom),
            (UiText.MenuFile, ZaxCommands.OpenRecentRom),
            (UiText.MenuFile, ZaxCommands.ChooseProfile),
            (UiText.MenuFile, ZaxCommands.ImportProfile),
            (UiText.MenuFile, ZaxCommands.ImportThreeDSen), // AMD-030 §2
            (UiText.MenuFile, ZaxCommands.LoadSlot),
            (UiText.MenuFile, ZaxCommands.QuickLoad),
            (UiText.MenuFile, ZaxCommands.SaveSlot),
            (UiText.MenuFile, ZaxCommands.QuickSave),
            (UiText.MenuFile, ZaxCommands.Quit),
            (UiText.MenuEmulation, ZaxCommands.PauseResume),
            (UiText.MenuEmulation, ZaxCommands.Reset),
            (UiText.MenuEmulation, ZaxCommands.Stop),
            (UiText.MenuEmulation, ZaxCommands.FastForward),
            (UiText.MenuEmulation, ZaxCommands.Rollback),
            (UiText.MenuEmulation, ZaxCommands.SetSpeed),
            (UiText.MenuEmulation, ZaxCommands.SetRegion),
            (UiText.MenuEmulation, ZaxCommands.ToggleAutoRegion),
            (UiText.MenuEmulation, ZaxCommands.SetPortType),
            (UiText.MenuVideo, ZaxCommands.ToggleTwoDimensional),
            (UiText.MenuVideo, ZaxCommands.SetCameraMode),
            (UiText.MenuVideo, ZaxCommands.ResetView),
            (UiText.MenuVideo, ZaxCommands.SetSkybox),
            (UiText.MenuVideo, ZaxCommands.ToggleShadows),
            (UiText.MenuVideo, ZaxCommands.ToggleFullscreen),
            (UiText.MenuVideo, ZaxCommands.ToggleFpsCounter),
            (UiText.MenuAudio, ZaxCommands.SetVolume),
            (UiText.MenuAudio, ZaxCommands.ToggleMute),

            // AMD-024 REQ-PUB-005 (revised 2026-10-21): « Musique de menu » is gone from the
            // Audio menu with the menu music itself.

            (UiText.MenuTools, ZaxCommands.ShowOptions),
            (UiText.MenuTools, ZaxCommands.ShowKeyConfig),
            (UiText.MenuTools, ZaxCommands.ShowDataFolder),
            (UiText.MenuTools, ZaxCommands.ShowAbout),
        ];

        foreach ((string menu, string command) in expected)
        {
            Assert.Contains(command, CommandsOf(bar, menu));
        }
    }

    /// <summary>
    /// REQ-UI-200: every entry shows the default shortcut its command carries, and the ones
    /// REQ-UI-204 gives a shortcut to really show it.
    /// </summary>
    [Fact]
    public void REQ_UI_200_each_entry_shows_its_default_shortcut()
    {
        using UiHarness harness = new("Arkanoid");

        Dictionary<string, string> shown = [];
        foreach (MenuItem item in harness.Ui.MenuBar.AllItems())
        {
            if (item.Command is { } command && item.Shortcut.Length > 0)
            {
                shown[command] = item.Shortcut;
            }
        }

        Assert.Equal("Ctrl+O", shown[ZaxCommands.OpenRom]);
        Assert.Equal("P", shown[ZaxCommands.PauseResume]);
        Assert.Equal("Ctrl+R", shown[ZaxCommands.Reset]);
        Assert.Equal("F11", shown[ZaxCommands.ToggleTwoDimensional]);
        Assert.Equal("Alt+Enter", shown[ZaxCommands.ToggleFullscreen]);
        Assert.Equal("Ctrl+,", shown[ZaxCommands.ShowOptions]);
        Assert.Equal("Ctrl+Q", shown[ZaxCommands.Quit]);
        Assert.Equal("Ctrl+F1…F10", shown[ZaxCommands.LoadSlot]);
        Assert.Equal("Shift+F1…F10", shown[ZaxCommands.SaveSlot]);
        Assert.Equal("FastForward shortcut", shown[ZaxCommands.FastForward]);
        Assert.Equal("RollBack shortcut", shown[ZaxCommands.Rollback]);
        Assert.Equal("long press on Menu", shown[ZaxCommands.ResetView]);
    }

    /// <summary>
    /// REQ-UI-200: an entry is greyed out when its command cannot run, and the toggles show
    /// their tick. With no game loaded Reset and Stop are unavailable; after a launch they are
    /// not.
    /// </summary>
    [Fact]
    public void REQ_UI_200_entries_follow_the_availability_of_their_command()
    {
        using UiHarness harness = new("Arkanoid");
        MenuBarModel bar = harness.Ui.MenuBar;

        MenuItem reset = Find(bar, ZaxCommands.Reset);
        MenuItem shadows = Find(bar, ZaxCommands.ToggleShadows);

        Assert.False(bar.IsEnabled(reset));
        Assert.True(bar.IsChecked(shadows));

        harness.Run(ZaxCommands.ToggleShadows);
        Assert.False(bar.IsChecked(shadows));

        harness.Launch("g0");
        Assert.True(bar.IsEnabled(reset));
    }

    /// <summary>
    /// REQ-UI-208: each of the ten slot rows shows the date and time of its state, or the word
    /// for « vide » in the interface language (AMD-035), and the quick slot is there too.
    /// </summary>
    [Fact]
    public void REQ_UI_208_the_slot_submenus_show_a_date_or_vide()
    {
        using UiHarness harness = new("Arkanoid");

        IReadOnlyList<MenuItem> slots = SubmenuOf(harness.Ui.MenuBar, UiText.MenuFile, UiText.SaveState);

        // Ten numbered rows, a rule, and the quick slot (REQ-UI-208).
        Assert.Equal(SaveSlotIndex.ExposedSlotCount + 2, slots.Count);
        Assert.All(
            slots.Where(static item => item.Kind != MenuItemKind.Separator),
            item => Assert.Contains(UiText.EmptySlot, item.Label));

        harness.Launch("g0");
        harness.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));

        slots = SubmenuOf(harness.Ui.MenuBar, UiText.MenuFile, UiText.LoadState);
        Assert.Contains(
            SaveStateFile.FormatTimestamp(harness.Now.ToUnixTimeSeconds()),
            slots[2].Label);
        Assert.StartsWith($"{UiText.SlotPrefix} 3 ", slots[2].Label);

        // The slots beyond ten are never exposed (REQ-UI-208, accepted divergence D).
        Assert.Equal(10, slots.Count(static item => item.Label.StartsWith(UiText.SlotPrefix, StringComparison.Ordinal)));
    }

    /// <summary>
    /// REQ-UI-200: the recent ROM submenu lists at most ten files and says so when it is empty.
    /// </summary>
    [Fact]
    public void REQ_UI_200_the_recent_rom_submenu_lists_the_ten_last_files()
    {
        using UiHarness harness = new("Arkanoid");

        IReadOnlyList<MenuItem> recent = SubmenuOf(harness.Ui.MenuBar, UiText.MenuFile, UiText.RecentRoms);
        Assert.Equal(UiText.NothingHere, Assert.Single(recent).Label);

        for (int index = 0; index < 12; index++)
        {
            harness.Store.Games.NoteRomOpened($"C:\\roms\\rom{index}.nes");
        }

        recent = SubmenuOf(harness.Ui.MenuBar, UiText.MenuFile, UiText.RecentRoms);
        Assert.Equal(10, recent.Count);
        Assert.Equal("rom11.nes", recent[0].Label);
    }

    /// <summary>
    /// REQ-UI-204: the shortcut table resolves each combination to the command the amendment
    /// gives it, and the modifiers have to match exactly — <c>Ctrl+P</c> is not <c>P</c>.
    /// </summary>
    [Fact]
    public void REQ_UI_204_the_shortcut_table_resolves_each_combination()
    {
        Assert.Equal(ZaxCommands.PauseResume, Single(Key.P, ShortcutModifiers.None).Command);
        Assert.Equal(ZaxCommands.Reset, Single(Key.R, ShortcutModifiers.Control).Command);
        Assert.Equal(ZaxCommands.Quit, Single(Key.Q, ShortcutModifiers.Control).Command);
        Assert.Equal(ZaxCommands.OpenRom, Single(Key.O, ShortcutModifiers.Control).Command);
        Assert.Equal(ZaxCommands.ShowOptions, Single(Key.Comma, ShortcutModifiers.Control).Command);
        Assert.Equal(ZaxCommands.ToggleFullscreen, Single(Key.Enter, ShortcutModifiers.Alt).Command);
        Assert.Equal(ZaxCommands.ToggleTwoDimensional, Single(Key.F11, ShortcutModifiers.None).Command);

        // AMD-013 (Q-030): Ctrl+Fn loads slot n, Maj+Fn saves it.
        (string Command, CommandArgs Args) load = Single(Key.F3, ShortcutModifiers.Control);
        Assert.Equal(ZaxCommands.LoadSlot, load.Command);
        Assert.Equal(3, load.Args.Number);

        (string Command, CommandArgs Args) save = Single(Key.F3, ShortcutModifiers.Shift);
        Assert.Equal(ZaxCommands.SaveSlot, save.Command);
        Assert.Equal(3, save.Args.Number);

        // An exact modifier match: P under Ctrl is nothing at all.
        List<(string Command, CommandArgs Args)> hits = [];
        MenuShortcuts.Resolve([Key.P], ShortcutModifiers.Control, hits);
        Assert.Empty(hits);
    }

    /// <summary>
    /// REQ-UI-204: no two shortcuts share the same key and modifier set, so no combination is
    /// ambiguous.
    /// </summary>
    [Fact]
    public void REQ_UI_204_the_shortcut_table_has_no_conflict()
    {
        HashSet<(Key, ShortcutModifiers)> seen = [];
        foreach (MenuShortcut shortcut in MenuShortcuts.All)
        {
            Assert.True(
                seen.Add((shortcut.Key, shortcut.Modifiers)),
                $"{shortcut.Text} is used twice");
            Assert.NotEmpty(shortcut.Text);
        }
    }

    /// <summary>
    /// REQ-UI-204 and REQ-INP-106: the keys that act without a modifier use none of the default
    /// game keys of SPEC-08 §4.2, for either player.
    /// </summary>
    [Fact]
    public void REQ_UI_204_no_bare_shortcut_collides_with_a_default_game_key()
    {
        // Player 2 has no default keyboard binding (REQ-INP-011), so player 1's table is the
        // whole of §4.2.
        HashSet<Key> gameKeys = [.. DefaultBindings.PlayerOneKeyboard.Select(static entry => entry.Key)];

        foreach (Key key in MenuShortcuts.BareKeys)
        {
            Assert.DoesNotContain(key, gameKeys);
        }
    }

    /// <summary>
    /// REQ-UI-204 and REQ-UI-205: a shortcut goes through the command model, so pressing it has
    /// exactly the effect of its menu entry. Ctrl+, opens the options window.
    /// </summary>
    [Fact]
    public void REQ_UI_205_a_shortcut_drives_the_command_model()
    {
        using UiHarness harness = new("Arkanoid");

        Assert.False(harness.Ui.State.Options);

        harness.Ui.HandleShortcuts([Key.Comma], ShortcutModifiers.Control);
        harness.Host.ApplyCommands();

        Assert.True(harness.Ui.State.Options);
    }

    private static (string Command, CommandArgs Args) Single(Key key, ShortcutModifiers modifiers)
    {
        List<(string Command, CommandArgs Args)> hits = [];
        MenuShortcuts.Resolve([key], modifiers, hits);
        return Assert.Single(hits);
    }

    private static IEnumerable<string> CommandsOf(MenuBarModel bar, string title)
    {
        Menu menu = bar.Menus.Single(m => string.Equals(m.Title, title, StringComparison.Ordinal));
        return Expand(menu.Items).Select(static item => item.Command).OfType<string>();
    }

    internal static IReadOnlyList<MenuItem> SubmenuOf(MenuBarModel bar, string title, string submenu)
    {
        Menu menu = bar.Menus.Single(m => string.Equals(m.Title, title, StringComparison.Ordinal));
        MenuItem item = menu.Items.Single(i =>
            i.Kind == MenuItemKind.Submenu && string.Equals(i.Label, submenu, StringComparison.Ordinal));
        return item.Children!();
    }

    internal static MenuItem Find(MenuBarModel bar, string command)
        => bar.AllItems().First(item => string.Equals(item.Command, command, StringComparison.Ordinal));

    private static IEnumerable<MenuItem> Expand(IReadOnlyList<MenuItem> items)
    {
        foreach (MenuItem item in items)
        {
            yield return item;

            if (item.Children is { } children)
            {
                foreach (MenuItem child in Expand(children()))
                {
                    yield return child;
                }
            }
        }
    }
}
