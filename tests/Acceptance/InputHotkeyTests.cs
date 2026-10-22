using System.Numerics;
using System.Reflection;
using Silk.NET.Input;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// SPEC-08 §3.9 to §3.12: the camera mapping, the system hotkeys on the display clock, the
/// routing while the menu is open and the automatic masking of the cursor.
/// </summary>
public sealed class InputHotkeyTests
{
    private static readonly Vec4F DefaultClipping = new(0.1f, 8.1f, 8.1f, 0.1f);

    /// <summary>
    /// REQ-INP-080: the camera is driven by the OR of the masks of the players whose port is a
    /// Controller; a player in Zapper does not turn the camera with their actions.
    /// </summary>
    [Fact]
    public void REQ_INP_080_a_zapper_player_does_not_drive_the_camera()
    {
        Assert.Equal(
            InputAction.RotLeft,
            InputRouting.CameraSources(InputAction.None, InputAction.RotLeft));

        Assert.Equal(
            InputAction.None,
            InputRouting.CameraSources(
                InputAction.None,
                InputAction.RotLeft,
                ControllerPortType.Controller,
                ControllerPortType.Zapper));

        Assert.Equal(
            InputAction.RotLeft,
            InputRouting.CameraSources(
                InputAction.RotLeft,
                InputAction.RotRight,
                ControllerPortType.Controller,
                ControllerPortType.Zapper));

        // Only the six camera bits come out, whatever else is held.
        Assert.Equal(
            InputAction.ZoomIn,
            InputRouting.CameraSources(InputAction.ZoomIn | InputAction.A | InputAction.Menu, InputAction.None));
    }

    /// <summary>
    /// REQ-INP-081: 30 degrees per second, RotLeft decreasing the yaw and RotRight increasing it,
    /// RotUp decreasing the pitch and RotDown increasing it; RotLeft wins over RotRight and
    /// RotDown over RotUp.
    /// </summary>
    [Fact]
    public void REQ_INP_081_the_rotation_setpoint_moves_thirty_degrees_per_second()
    {
        Assert.Equal(30f, SceneConstants.RotationDegreesPerSecond);

        SceneCamera camera = Camera();
        CameraControls.Apply(camera, Actions(InputAction.RotRight), 3f);
        Assert.Equal(90f, camera.Current.Angles.Y, 3);

        camera = Camera();
        CameraControls.Apply(camera, Actions(InputAction.RotLeft), 1f);
        Assert.Equal(-30f, camera.Current.Angles.Y, 3);

        camera = Camera();
        CameraControls.Apply(camera, Actions(InputAction.RotDown), 1f);
        Assert.Equal(30f, camera.Current.Angles.X, 3);

        camera = Camera();
        CameraControls.Apply(camera, Actions(InputAction.RotUp), 1f);
        Assert.Equal(-30f, camera.Current.Angles.X, 3);

        // Both directions of one axis: RotLeft and RotDown win.
        camera = Camera();
        CameraControls.Apply(camera, Actions(InputAction.RotLeft | InputAction.RotRight), 1f);
        Assert.Equal(-30f, camera.Current.Angles.Y, 3);

        camera = Camera();
        CameraControls.Apply(camera, Actions(InputAction.RotUp | InputAction.RotDown), 1f);
        Assert.Equal(30f, camera.Current.Angles.X, 3);
    }

    /// <summary>
    /// REQ-INP-082: 70 scene units per second, ZoomIn shortening the distance and winning over
    /// ZoomOut.
    /// </summary>
    [Fact]
    public void REQ_INP_082_the_zoom_setpoint_moves_seventy_units_per_second()
    {
        Assert.Equal(70f, SceneConstants.ZoomUnitsPerSecond);

        SceneCamera camera = Camera();
        float start = camera.Current.Distance;
        CameraControls.Apply(camera, Actions(InputAction.ZoomOut), 1f);
        Assert.Equal(start + 70f, camera.Current.Distance, 3);

        camera = Camera();
        start = camera.Current.Distance;
        CameraControls.Apply(camera, Actions(InputAction.ZoomIn | InputAction.ZoomOut), 0.5f);
        Assert.Equal(start - 35f, camera.Current.Distance, 3);
    }

    /// <summary>
    /// REQ-INP-090: one wheel notch forward moves the camera 5 scene units closer; the wheel is
    /// ignored while a zoom action is held.
    /// </summary>
    [Fact]
    public void REQ_INP_090_one_wheel_notch_is_five_scene_units()
    {
        Assert.Equal(5f, SceneConstants.WheelUnitsPerNotch);

        SceneCamera camera = Camera();
        float start = camera.Current.Distance;
        CameraControls.ApplyWheel(camera, 4f, CameraAction.None);
        Assert.Equal(start - 20f, camera.Current.Distance, 3);

        CameraControls.ApplyWheel(camera, -4f, CameraAction.None);
        Assert.Equal(start, camera.Current.Distance, 3);

        CameraControls.ApplyWheel(camera, 4f, CameraAction.ZoomIn);
        Assert.Equal(start, camera.Current.Distance, 3);
    }

    /// <summary>
    /// REQ-INP-091: a left-button drag turns the camera 0.1 degree per pixel, to the right like
    /// RotRight and upwards like RotUp.
    /// </summary>
    [Fact]
    public void REQ_INP_091_a_drag_is_a_tenth_of_a_degree_per_pixel()
    {
        Assert.Equal(0.1f, SceneConstants.DragDegreesPerPixel);

        SceneCamera camera = Camera();
        CameraControls.ApplyDrag(camera, 300f, 0f);
        Assert.Equal(30f, camera.Current.Angles.Y, 3);

        // Upwards is the direction of RotUp, which decreases the pitch.
        camera = Camera();
        CameraControls.ApplyDrag(camera, 0f, 100f);
        Assert.Equal(-10f, camera.Current.Angles.X, 3);
    }

    /// <summary>
    /// REQ-INP-100: the hotkeys are read on the OR of both players' masks whatever the port
    /// types, and on the display clock — not on the emulated-frame clock.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_100_a_zapper_player_still_reaches_the_hotkeys()
    {
        using HostDriver host = HostDriver.Create(ZapperProfile());
        host.Launch(host.EntryIdByTitle("Zapper-probe"), host.WriteRom(LoopRom.Counter()));
        Assert.Equal(ControllerPortType.Zapper, host.PortType(1));
        Assert.Equal(EmulationState.Running, host.State);

        // J2's Menu, on a Zapper port, opens the menu all the same.
        host.Tick(0.05, InputAction.None, InputAction.Menu);
        host.Tick(0.05, InputAction.None, InputAction.None);
        host.Tick(1.0 / 60.0);

        Assert.Equal(EmulationState.Paused, host.State);
    }

    /// <summary>
    /// REQ-INP-101 and §5.2: a short press on Menu toggles between running and paused; a long
    /// press resets the camera view without changing the state.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_101_a_short_press_pauses_and_a_long_press_resets_the_view()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        Assert.Equal(EmulationState.Running, host.State);

        Assert.Equal(0.3f, PressTracker.LongPressSeconds);

        // 0.1 s then release: pause (§5.2 row 1).
        ShortPress(host);
        Assert.Equal(EmulationState.Paused, host.State);

        ShortPress(host);
        Assert.Equal(EmulationState.Running, host.State);

        // 0.5 s then release: the view is reset and the game keeps running (§5.2 row 2).
        Vector3 defaultAngles = host.Scene.Camera.Current.Angles;
        host.Scene.Camera.Current.Angles = new Vector3(40f, 40f, 0f);
        host.Scene.Camera.SnapToSetpoints();

        host.Tick(0.1, InputAction.Menu);
        host.Tick(0.25, InputAction.Menu);
        host.Tick(0.1, InputAction.Menu);
        host.Tick(1.0 / 60.0, InputAction.None);
        host.Tick(1.0 / 60.0);

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(defaultAngles, host.Scene.Camera.Current.Angles);
    }

    /// <summary>
    /// REQ-INP-101 and SPEC-10 REQ-UI-004 case 1: a Menu press is <b>ignored</b> while a game is
    /// loading.
    /// </summary>
    /// <remarks>
    /// Round 1: the press was deferred instead, so the game paused as soon as the load finished
    /// (<c>verify/round1/R1-report-part-c.md</c>). Corrected by the <c>R1-fixes</c> batch —
    /// <c>ReadHotkeys</c> drops <c>MenuShort</c> while <c>IsLoading</c>. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_101_a_menu_press_during_a_load_must_be_ignored()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        string rom = host.WriteRom(LoopRom.Counter());
        host.SetRomPath(id, Path.GetFullPath(rom));

        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();
        Assert.True(host.IsLoading);

        // Pressed and released while the loading indicator is up.
        host.Tick(0.05, InputAction.Menu);
        host.Tick(0.05, InputAction.None);
        Assert.True(host.IsLoading);

        host.Settle();
        host.Tick(1.0 / 60.0);
        host.Tick(1.0 / 60.0);

        Assert.Equal(EmulationState.Running, host.State);
    }

    /// <summary>
    /// REQ-INP-102 and §5.2: a rising edge on RollBack rolls the game back, resumes it and shows
    /// <c>Roll Back</c> for one second.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_102_rollback_resumes_the_game_and_shows_its_notification()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);

        host.Tick(1.0 / 60.0, InputAction.RollBack);
        host.Tick(1.0 / 60.0, InputAction.RollBack);

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal("Roll Back", host.Notifications.Current?.Text);
        Assert.Equal(1.0, NotificationCenter.ShortSeconds);
        Assert.True(host.Notifications.Remaining <= 1.0);

        // Held down, it only fires once: the trigger is the rising edge.
        int before = host.Notifications.Log.Count;
        host.Tick(1.0 / 60.0, InputAction.RollBack);
        host.Tick(1.0 / 60.0, InputAction.RollBack);
        Assert.Equal(before, host.Notifications.Log.Count);
    }

    /// <summary>
    /// REQ-INP-103: holding FastForward asks for an advance on every displayed frame, which gives
    /// about 450 emulated frames over five seconds at NTSC (±10 %); a brief tap gives 30 frames
    /// at ×1.5.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_103_holding_fast_forward_runs_at_one_and_a_half_times()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        long start = host.EmulatedFrames;
        for (int tick = 0; tick < 300; tick++)
        {
            host.Tick(1.0 / 60.0, InputAction.FastForward);
        }

        long ran = host.EmulatedFrames - start;
        Assert.InRange(ran, 405, 495);
        Assert.Equal("Fast Forward", host.Notifications.Current?.Text);

        // A brief tap arms exactly 30 emulated frames at ×1.5 (REQ-LOOP-016).
        using HostDriver tap = HostDriver.Create();
        tap.Launch2DMode();
        tap.Tick(1.0 / 60.0, InputAction.FastForward);
        tap.Tick(1.0 / 60.0, InputAction.None);

        Assert.True(tap.FastForward.IsActive);
        Assert.Equal(1.5, tap.Pacer.Speed, 3);
        Assert.InRange(tap.FastForward.Remaining, 1, 30);
    }

    /// <summary>
    /// REQ-INP-104 and §5.2: QuickSave on the rising edge of its action, or Select held for more
    /// than 0.1 s followed by the release of RotDown; QuickLoad likewise with RotUp. The
    /// combinations are evaluated player by player.
    /// </summary>
    [Fact]
    public void REQ_INP_104_the_quick_slot_combinations_are_per_player()
    {
        Assert.Equal(0.1f, HotkeyTracker.SelectHoldSeconds);

        HotkeyTracker tracker = new();

        // Select held for more than 0.1 s, then RotDown pressed and released.
        Assert.Equal(HotkeyEvents.None, tracker.Update(InputAction.Select, InputAction.None, 0.2f));
        Assert.Equal(
            HotkeyEvents.None,
            tracker.Update(InputAction.Select | InputAction.RotDown, InputAction.None, 1f / 60f));
        Assert.True(tracker
            .Update(InputAction.Select, InputAction.None, 1f / 60f)
            .HasFlag(HotkeyEvents.QuickSave));

        // RotUp gives a quick load.
        Assert.Equal(
            HotkeyEvents.None,
            tracker.Update(InputAction.Select | InputAction.RotUp, InputAction.None, 1f / 60f));
        Assert.True(tracker
            .Update(InputAction.Select, InputAction.None, 1f / 60f)
            .HasFlag(HotkeyEvents.QuickLoad));

        // One player holding Select and the other flicking the stick saves nothing.
        HotkeyTracker split = new();
        Assert.Equal(HotkeyEvents.None, split.Update(InputAction.Select, InputAction.None, 0.2f));
        Assert.Equal(
            HotkeyEvents.None,
            split.Update(InputAction.Select, InputAction.RotDown, 1f / 60f));
        Assert.Equal(
            HotkeyEvents.None,
            split.Update(InputAction.Select, InputAction.None, 1f / 60f));

        // Select appearing in the same frame as the release never fires.
        HotkeyTracker late = new();
        Assert.Equal(
            HotkeyEvents.None,
            late.Update(InputAction.Select | InputAction.RotDown, InputAction.None, 1f / 60f));
        Assert.Equal(HotkeyEvents.None, late.Update(InputAction.Select, InputAction.None, 1f / 60f));
    }

    /// <summary>
    /// REQ-INP-104, end to end: the combination writes the quick slot and shows
    /// <c>Quick Save</c> for four seconds, and an empty quick slot makes QuickLoad do nothing.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_104_the_combination_writes_the_quick_slot()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        // An empty quick slot: nothing at all, not even a notification.
        host.Tick(1.0 / 60.0, InputAction.QuickLoad);
        host.Tick(1.0 / 60.0, InputAction.None);
        host.Tick(1.0 / 60.0);
        Assert.Null(host.Notifications.Current);

        host.Tick(0.2, InputAction.Select);
        host.Tick(1.0 / 60.0, InputAction.Select | InputAction.RotDown);
        host.Tick(1.0 / 60.0, InputAction.Select);
        host.Tick(1.0 / 60.0);

        Assert.Equal("Quick Save", host.Notifications.Current?.Text);
        Assert.Equal(4.0, NotificationCenter.DefaultSeconds);
        Assert.True(host.Slots.Find("quick")!.IsOccupied);
    }

    /// <summary>
    /// REQ-INP-105: while the menu is open the emulation is stopped, so no input reaches the
    /// core; the directional actions walk the interface on their rising edges and Select triggers
    /// the secondary action.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact]
    public void REQ_INP_105_in_pause_the_actions_drive_the_interface_and_not_the_core()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Run(ZaxCommands.Pause);

        long before = host.EmulatedFrames;
        for (int tick = 0; tick < 30; tick++)
        {
            host.Tick(1.0 / 60.0, InputAction.A | InputAction.Right);
        }

        Assert.Equal(before, host.EmulatedFrames);

        MenuNavigation navigation = new();
        Assert.Equal(
            MenuNavigationEvents.Next,
            navigation.Update(InputAction.Right, paused: true));

        // A held action is one event, not one per frame.
        Assert.Equal(
            MenuNavigationEvents.None,
            navigation.Update(InputAction.Right, paused: true));

        Assert.Equal(
            MenuNavigationEvents.Previous,
            navigation.Update(InputAction.Left, paused: true));
        Assert.Equal(
            MenuNavigationEvents.NextVertical,
            navigation.Update(InputAction.Down, paused: true));
        Assert.Equal(
            MenuNavigationEvents.PreviousVertical,
            navigation.Update(InputAction.Up, paused: true));
        Assert.Equal(
            MenuNavigationEvents.Secondary,
            navigation.Update(InputAction.Select, paused: true));

        // Outside the menu nothing is produced.
        MenuNavigation running = new();
        Assert.Equal(
            MenuNavigationEvents.None,
            running.Update(InputAction.Right, paused: false));
    }

    /// <summary>
    /// REQ-INP-106: a fixed shortcut added by the implementation must use no key of the default
    /// table of §4.2. Only the shortcuts that act with <b>no modifier</b> are concerned; the
    /// modified ones of AMD-010 REQ-UI-204 deliberately reuse those positions (Q-049).
    /// </summary>
    [Fact]
    public void REQ_INP_106_no_bare_fixed_shortcut_uses_a_default_game_key()
    {
        Key[] defaults =
        [
            Key.Left, Key.Right, Key.Up, Key.Down, Key.X, Key.Z, Key.Space, Key.Enter,
            Key.F5, Key.F6, Key.F7, Key.F8, Key.F9, Key.F10, Key.Escape, Key.Comma, Key.Period,
        ];

        Type shortcuts = typeof(Zax.App.AppOptions).Assembly.GetType("Zax.App.Ui.MenuShortcuts", true)!;
        object bare = shortcuts.GetProperty("BareKeys", BindingFlags.Public | BindingFlags.Static)!
            .GetValue(null)!;

        List<Key> keys = [.. ((System.Collections.IEnumerable)bare).Cast<Key>()];
        Assert.NotEmpty(keys);

        foreach (Key key in keys)
        {
            Assert.DoesNotContain(key, defaults);
        }
    }

    /// <summary>
    /// REQ-INP-110: the cursor is masked after two still seconds while a game runs — the
    /// twentieth consecutive check without movement — and any movement shows it again.
    /// </summary>
    [Fact]
    public void REQ_INP_110_the_cursor_hides_after_two_still_seconds()
    {
        Assert.Equal(0.1f, CursorAutoHide.CheckSeconds);
        Assert.Equal(20, CursorAutoHide.ChecksBeforeHiding);

        CursorAutoHide cursor = new();
        for (int check = 0; check < 19; check++)
        {
            Assert.True(cursor.Update(gameRunning: true, pointerMoved: false, 0.1f));
        }

        Assert.Equal(19, cursor.StillChecks);
        Assert.False(cursor.Update(gameRunning: true, pointerMoved: false, 0.1f));

        // One pixel of movement shows it again and restarts the count.
        Assert.True(cursor.Update(gameRunning: true, pointerMoved: true, 0.1f));
        Assert.Equal(0, cursor.StillChecks);
    }

    /// <summary>
    /// REQ-INP-111: outside a running game the cursor is visible at all times, and it comes back
    /// within one check of the pause.
    /// </summary>
    [Fact]
    public void REQ_INP_111_the_cursor_is_always_visible_outside_a_running_game()
    {
        CursorAutoHide cursor = new();
        for (int check = 0; check < CursorAutoHide.ChecksBeforeHiding; check++)
        {
            cursor.Update(gameRunning: true, pointerMoved: false, 0.1f);
        }

        Assert.False(cursor.CursorVisible);

        // The pause brings it back at the next check at the latest, which is 0.1 s.
        Assert.True(cursor.Update(gameRunning: false, pointerMoved: false, 0.1f));
        Assert.Equal(0, cursor.StillChecks);

        // It stays visible for as long as the game is not running, however still the mouse is.
        for (int check = 0; check < 40; check++)
        {
            Assert.True(cursor.Update(gameRunning: false, pointerMoved: false, 0.1f));
        }

        // Closing the program shows it again (REQ-INP-111, last clause).
        for (int check = 0; check < CursorAutoHide.ChecksBeforeHiding; check++)
        {
            cursor.Update(gameRunning: true, pointerMoved: false, 0.1f);
        }

        Assert.False(cursor.CursorVisible);
        cursor.Show();
        Assert.True(cursor.CursorVisible);
    }

    private static SceneCamera Camera()
    {
        SceneCamera camera = new();
        camera.ResetView();
        camera.Current.Angles = Vector3.Zero;
        camera.SnapToSetpoints();
        return camera;
    }

    private static CameraAction Actions(InputAction mask) => InputActions.ToCameraActions(mask);

    private static void ShortPress(HostDriver host)
    {
        host.Tick(0.1, InputAction.Menu);
        host.Tick(1.0 / 60.0, InputAction.None);
        host.Tick(1.0 / 60.0);
    }

    private static GameProfile ZapperProfile()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "Zapper-probe";
        profile.Scripts.Add(new ProfileScript(
            "zap",
            true,
            "function Start() Setting:SetInputType(0, 1) end",
            "*"));
        return profile;
    }
}
