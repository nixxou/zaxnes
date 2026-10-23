using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;

namespace Zax.App.Tests;

/// <summary>
/// The command model of AMD-010 REQ-UI-205 and the notification area of SPEC-10 §3.10.
/// </summary>
public sealed class CommandModelTests
{
    /// <summary>
    /// REQ-UI-205 and REQ-UI-200: every entry of the menu bar has a stable name in the model,
    /// and each one is invocable programmatically.
    /// </summary>
    [Fact]
    public void REQ_UI_205_the_model_enumerates_every_menu_command()
    {
        using LoopHarness harness = LoopHarness.Create();

        string[] expected =
        [
            ZaxCommands.OpenRom, ZaxCommands.OpenRecentRom, ZaxCommands.ChooseProfile,
            ZaxCommands.ImportProfile, ZaxCommands.ImportThreeDSen,
            ZaxCommands.LaunchGame, ZaxCommands.LoadSlot,
            ZaxCommands.SaveSlot, ZaxCommands.QuickSave, ZaxCommands.QuickLoad,
            ZaxCommands.DeleteSlot, ZaxCommands.DeleteGame, ZaxCommands.CreateGameArt,
            ZaxCommands.ImportCover, ZaxCommands.CancelModal, ZaxCommands.Quit,
            ZaxCommands.PauseResume, ZaxCommands.Pause, ZaxCommands.Resume, ZaxCommands.Reset,
            ZaxCommands.Stop, ZaxCommands.FastForward, ZaxCommands.Rollback,
            ZaxCommands.SetSpeed, ZaxCommands.SpeedUp, ZaxCommands.SpeedDown,
            ZaxCommands.SetRegion, ZaxCommands.ToggleAutoRegion, ZaxCommands.SetPortType,
            ZaxCommands.ToggleTwoDimensional, ZaxCommands.ResetView,
            ZaxCommands.SetCameraMode, ZaxCommands.ToggleLocalCamera, ZaxCommands.SetSkybox,
            ZaxCommands.ToggleShadows, ZaxCommands.ToggleFullscreen,
            ZaxCommands.ToggleFpsCounter,
            ZaxCommands.SetVolume, ZaxCommands.VolumeUp, ZaxCommands.VolumeDown,
            ZaxCommands.ToggleMute, ZaxCommands.ToggleMenuMusic,
            ZaxCommands.ShowOptions, ZaxCommands.ShowKeyConfig, ZaxCommands.ShowDataFolder,
            ZaxCommands.ShowAbout,
        ];

        foreach (string name in expected)
        {
            Assert.True(harness.Host.Commands.Contains(name), $"the command '{name}' is missing");
        }

        Assert.Equal(expected.Length, harness.Host.Commands.Commands.Count);
    }

    /// <summary>
    /// REQ-UI-205: every command name follows the stable-key shape SPEC-09 REQ-PERS-080 sets,
    /// and no two commands share a name.
    /// </summary>
    [Fact]
    public void REQ_UI_205_command_names_are_stable_keys()
    {
        using LoopHarness harness = LoopHarness.Create();
        HashSet<string> seen = [];

        foreach (ZaxCommand command in harness.Host.Commands.Commands)
        {
            Assert.True(seen.Add(command.Name), $"'{command.Name}' is registered twice");
            Assert.Matches("^[a-z0-9_]+(\\.[a-z0-9_]+)+$", command.Name);
            Assert.NotEmpty(command.Label);
        }
    }

    /// <summary>
    /// REQ-UI-204: the commands the amendment gives a shortcut to carry it, so the menu can
    /// show it next to the entry.
    /// </summary>
    [Fact]
    public void REQ_UI_204_the_default_shortcuts_are_carried_by_the_model()
    {
        using LoopHarness harness = LoopHarness.Create();

        Assert.Equal("Ctrl+O", harness.Host.Commands.Find(ZaxCommands.OpenRom)!.Shortcut);
        Assert.Equal("Ctrl+R", harness.Host.Commands.Find(ZaxCommands.Reset)!.Shortcut);
        Assert.Equal("Ctrl+Q", harness.Host.Commands.Find(ZaxCommands.Quit)!.Shortcut);
        Assert.Equal("P", harness.Host.Commands.Find(ZaxCommands.PauseResume)!.Shortcut);
        Assert.Equal("F11", harness.Host.Commands.Find(ZaxCommands.ToggleTwoDimensional)!.Shortcut);
        Assert.Equal("Alt+Enter", harness.Host.Commands.Find(ZaxCommands.ToggleFullscreen)!.Shortcut);
        Assert.Equal("Ctrl+,", harness.Host.Commands.Find(ZaxCommands.ShowOptions)!.Shortcut);
    }

    /// <summary>
    /// REQ-UI-205: the availability of a command reflects the state, which is what a greyed-out
    /// menu entry shows. Invoking an unavailable command does nothing.
    /// </summary>
    [Fact]
    public void REQ_UI_205_availability_follows_the_state()
    {
        using LoopHarness harness = LoopHarness.Create();

        Assert.False(harness.Host.Commands.Find(ZaxCommands.Reset)!.CanExecute());
        Assert.False(harness.Host.Commands.Find(ZaxCommands.Stop)!.CanExecute());
        Assert.False(harness.Host.Commands.Find(ZaxCommands.QuickSave)!.CanExecute());
        Assert.False(harness.Host.Commands.Invoke(ZaxCommands.Reset));

        EmulationStateMachineTests.Launch(harness, "g1");

        Assert.True(harness.Host.Commands.Find(ZaxCommands.Reset)!.CanExecute());
        Assert.True(harness.Host.Commands.Find(ZaxCommands.Stop)!.CanExecute());
        Assert.True(harness.Host.Commands.Find(ZaxCommands.QuickSave)!.CanExecute());

        // The quick slot is empty, so quick load is unavailable (REQ-LOOP-045).
        Assert.False(harness.Host.Commands.Find(ZaxCommands.QuickLoad)!.CanExecute());
        harness.Run(ZaxCommands.QuickSave);
        Assert.True(harness.Host.Commands.Find(ZaxCommands.QuickLoad)!.CanExecute());
    }

    /// <summary>
    /// REQ-UI-205: the toggles report their tick state, so the menu can draw a check mark.
    /// </summary>
    [Fact]
    public void REQ_UI_205_the_toggles_report_their_state()
    {
        using LoopHarness harness = LoopHarness.Create();

        ZaxCommand shadows = harness.Host.Commands.Find(ZaxCommands.ToggleShadows)!;
        Assert.True(shadows.IsToggle);
        Assert.True(shadows.IsChecked);

        harness.Run(ZaxCommands.ToggleShadows);
        Assert.False(shadows.IsChecked);
        Assert.False(harness.Settings.Shadow);
        Assert.False(harness.Scene.Settings.Shadows);

        ZaxCommand music = harness.Host.Commands.Find(ZaxCommands.ToggleMenuMusic)!;
        Assert.True(music.IsChecked);
        harness.Run(ZaxCommands.ToggleMenuMusic);
        Assert.False(music.IsChecked);
        Assert.False(harness.Mixer.MenuMusicEnabled);
    }

    /// <summary>
    /// REQ-LOOP-015 through the command model: the speed commands move the setting and bound it
    /// at ±8.
    /// </summary>
    [Fact]
    public void REQ_LOOP_015_the_speed_commands_bound_the_setting()
    {
        using LoopHarness harness = LoopHarness.Create();

        harness.Run(ZaxCommands.SetSpeed, CommandArgs.Of(4));
        Assert.Equal(4, harness.Settings.GameSpeed);
        Assert.Equal(2.0, harness.Settings.SpeedFactor, 1e-9);

        for (int press = 0; press < 10; press++)
        {
            harness.Run(ZaxCommands.SpeedUp);
        }

        Assert.Equal(SpeedSetting.Maximum, harness.Settings.GameSpeed);

        for (int press = 0; press < 40; press++)
        {
            harness.Run(ZaxCommands.SpeedDown);
        }

        Assert.Equal(SpeedSetting.Minimum, harness.Settings.GameSpeed);
    }

    /// <summary>
    /// AMD-010 REQ-UI-202/203/209: the commands the UI lot owns raise a request carrying their
    /// name, so the model is complete before the screens exist.
    /// </summary>
    [Fact]
    public void REQ_UI_205_ui_only_commands_raise_a_request()
    {
        using LoopHarness harness = LoopHarness.Create();
        List<string> requested = [];
        harness.Host.UiRequested += (name, _) => requested.Add(name);

        harness.Run(ZaxCommands.ShowOptions);
        harness.Run(ZaxCommands.ShowKeyConfig);
        harness.Run(ZaxCommands.ShowAbout);
        harness.Run(ZaxCommands.ToggleFullscreen);

        Assert.Equal(
            [ZaxCommands.ShowOptions, ZaxCommands.ShowKeyConfig, ZaxCommands.ShowAbout, ZaxCommands.ToggleFullscreen],
            requested);
        Assert.True(harness.Settings.Fullscreen);
    }

    /// <summary>
    /// REQ-UI-080 and REQ-UI-081: one notification at a time, no stacking, and the timer runs on
    /// real time whatever the emulation does.
    /// </summary>
    [Fact]
    public void REQ_UI_081_notifications_do_not_stack()
    {
        NotificationCenter notifications = new();

        Assert.True(notifications.ShowError("Invalid rom file."));

        // A normal notification cannot replace a visible error.
        Assert.False(notifications.Show("Quick Save"));
        Assert.Equal("Invalid rom file.", notifications.Current!.Value.Text);

        notifications.Advance(4.0);
        Assert.Null(notifications.Current);

        // Once the error is gone, the normal one is accepted.
        Assert.True(notifications.Show("Quick Save"));
        Assert.Equal("Quick Save", notifications.Current!.Value.Text);

        // An equal or higher severity replaces and restarts the timer.
        notifications.Advance(3.0);
        Assert.True(notifications.Show("Roll Back", NotificationCenter.ShortSeconds));
        Assert.Equal(1.0, notifications.Remaining, 1e-9);
    }

    /// <summary>
    /// REQ-UI-080: the duration is measured in real time and is not affected by the pause.
    /// </summary>
    [Fact]
    public void REQ_UI_080_the_timer_runs_in_real_time()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.Run(ZaxCommands.QuickSave);
        Assert.Equal("Quick Save", harness.Host.Notifications.Current!.Value.Text);

        harness.Run(ZaxCommands.Pause);
        for (int tick = 0; tick < 50; tick++)
        {
            harness.Tick(0.1);
        }

        Assert.Null(harness.Host.Notifications.Current);
    }

    /// <summary>
    /// REQ-UI-082: the four in-game notifications carry their exact texts and durations.
    /// </summary>
    [Fact]
    public void REQ_UI_082_the_in_game_notifications_have_their_exact_texts()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.RunFrames(120);

        harness.Run(ZaxCommands.Rollback);
        Assert.Equal(("Roll Back", 1.0), Latest(harness));

        harness.Run(ZaxCommands.FastForward);
        Assert.Equal(("Fast Forward", 1.0), Latest(harness));

        harness.Run(ZaxCommands.QuickSave);
        Assert.Equal(("Quick Save", 4.0), Latest(harness));

        harness.Run(ZaxCommands.QuickLoad);
        Assert.Equal(("Quick Load", 4.0), Latest(harness));

        static (string, double) Latest(LoopHarness harness)
        {
            Notification last = harness.Host.Notifications.Log[^1];
            return (last.Text, last.Seconds);
        }
    }

    /// <summary>
    /// REQ-UI-082: without a current game the quick save shows nothing, and with an empty quick
    /// slot the quick load shows nothing.
    /// </summary>
    [Fact]
    public void REQ_UI_082_the_quick_actions_are_silent_when_they_do_nothing()
    {
        using LoopHarness harness = LoopHarness.Create();

        harness.Run(ZaxCommands.QuickSave);
        harness.Run(ZaxCommands.QuickLoad);

        Assert.Empty(harness.Host.Notifications.Log);
    }

    /// <summary>
    /// REQ-LOOP-007: the queue is FIFO. A pause followed by a resume leaves the game running,
    /// and the reverse order leaves it paused.
    /// </summary>
    [Fact]
    public void REQ_LOOP_007_the_queue_is_first_in_first_out()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.Host.Enqueue(ZaxCommands.Pause);
        harness.Host.Enqueue(ZaxCommands.Resume);
        harness.Tick();
        Assert.Equal(EmulationState.Running, harness.Host.State);

        harness.Host.Enqueue(ZaxCommands.Resume);
        harness.Host.Enqueue(ZaxCommands.Pause);
        harness.Tick();
        Assert.Equal(EmulationState.Paused, harness.Host.State);
    }

    /// <summary>
    /// SPEC-08 REQ-INP-101 to REQ-INP-104: the Menu short press toggles the pause, the long
    /// press does not, and the combinations reach the quick slot.
    /// </summary>
    [Fact]
    public void REQ_INP_101_the_menu_press_length_decides()
    {
        HotkeyTracker tracker = new();

        // A 0.1 s press is short.
        Assert.Equal(HotkeyEvents.None, tracker.Update(InputAction.Menu, 0.1f));
        Assert.Equal(HotkeyEvents.MenuShort, tracker.Update(InputAction.None, 0.0f));

        // A 0.5 s press is long and reports as soon as the threshold is crossed.
        Assert.Equal(HotkeyEvents.None, tracker.Update(InputAction.Menu, 0.0f));
        Assert.Equal(HotkeyEvents.MenuLong, tracker.Update(InputAction.Menu, 0.4f));
        Assert.Equal(HotkeyEvents.None, tracker.Update(InputAction.None, 0.0f));
    }

    /// <summary>
    /// REQ-INP-103: the fast-forward hotkey emits a request on every rendered frame it is held,
    /// and REQ-INP-104's combinations reach the quick slot.
    /// </summary>
    [Fact]
    public void REQ_INP_103_the_hotkeys_produce_their_events()
    {
        HotkeyTracker tracker = new();

        Assert.True(tracker.Update(InputAction.FastForward, 0.01f).HasFlag(HotkeyEvents.FastForward));
        Assert.True(tracker.Update(InputAction.FastForward, 0.01f).HasFlag(HotkeyEvents.FastForward));

        // A rising edge only for RollBack.
        Assert.True(tracker.Update(InputAction.RollBack, 0.01f).HasFlag(HotkeyEvents.Rollback));
        Assert.False(tracker.Update(InputAction.RollBack, 0.01f).HasFlag(HotkeyEvents.Rollback));

        // Select held more than 0.1 s, then RotDown released: quick save.
        tracker.Update(InputAction.Select, 0.2f);
        tracker.Update(InputAction.Select | InputAction.RotDown, 0.02f);
        Assert.True(tracker.Update(InputAction.Select, 0.02f).HasFlag(HotkeyEvents.QuickSave));

        // The same with RotUp: quick load.
        tracker.Update(InputAction.Select | InputAction.RotUp, 0.02f);
        Assert.True(tracker.Update(InputAction.Select, 0.02f).HasFlag(HotkeyEvents.QuickLoad));
    }

    /// <summary>
    /// REQ-INP-101 and REQ-LOOP-005: the Menu hotkey drives the pause through the loop, and a
    /// long press resets the view without changing the state.
    /// </summary>
    [Fact]
    public void REQ_LOOP_005_the_menu_hotkey_toggles_the_pause()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.Tick(0.1, InputAction.Menu);
        harness.Tick(0.01, InputAction.None);
        harness.Tick();
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        harness.Tick(0.1, InputAction.Menu);
        harness.Tick(0.01, InputAction.None);
        harness.Tick();
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // A long press leaves the state alone. The press is held over several rendered
        // frames, as it is in the real loop.
        harness.Tick(0.1, InputAction.Menu);
        harness.Tick(0.2, InputAction.Menu);
        harness.Tick(0.2, InputAction.Menu);
        harness.Tick(0.01, InputAction.None);
        harness.Tick();
        Assert.Equal(EmulationState.Running, harness.Host.State);
    }
}
