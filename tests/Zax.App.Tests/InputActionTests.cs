using Zax.App.Input;
using Zax.App.Loop;
using Zax.Nes;
using Zax.Platform.Input;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// The logical action layer of SPEC-08 §3.1, §3.9, §3.11 and §3.13: the bit values, the
/// combination rules, the input word, the camera sources and the hotkeys, all driven by injected
/// masks with no window in sight (REQ-INP-120).
/// </summary>
public sealed class InputActionTests
{
    [Theory]
    // SPEC-08 §4.1: the bit values the Lua scripts see.
    [InlineData(InputAction.A, 0x01)]
    [InlineData(InputAction.B, 0x02)]
    [InlineData(InputAction.Select, 0x04)]
    [InlineData(InputAction.Start, 0x08)]
    [InlineData(InputAction.Up, 0x10)]
    [InlineData(InputAction.Down, 0x20)]
    [InlineData(InputAction.Left, 0x40)]
    [InlineData(InputAction.Right, 0x80)]
    [InlineData(InputAction.ZoomIn, 0x100)]
    [InlineData(InputAction.ZoomOut, 0x200)]
    [InlineData(InputAction.RotLeft, 0x400)]
    [InlineData(InputAction.RotRight, 0x800)]
    [InlineData(InputAction.RotUp, 0x1000)]
    [InlineData(InputAction.RotDown, 0x2000)]
    [InlineData(InputAction.Menu, 0x4000)]
    [InlineData(InputAction.RollBack, 0x8000)]
    [InlineData(InputAction.FastForward, 0x10000)]
    [InlineData(InputAction.QuickSave, 0x20000)]
    [InlineData(InputAction.QuickLoad, 0x40000)]
    public void REQ_INP_003_the_action_bits_are_those_of_the_table(InputAction action, int bit)
        => Assert.Equal(bit, (int)action);

    /// <summary>
    /// REQ-INP-001 and REQ-INP-040: exactly 19 actions, in the display order of the table of
    /// §4.1, with its English labels, and exactly two players.
    /// </summary>
    [Fact]
    public void REQ_INP_001_there_are_nineteen_actions_in_the_order_of_the_table()
    {
        Assert.Equal(19, InputActions.Count);
        Assert.Equal(InputActions.Count, InputActions.All.Count);
        Assert.Equal(2, InputActions.PlayerCount);
        Assert.Equal(InputActions.All.Count, InputActions.All.Distinct().Count());

        Assert.Equal(
            [
                "Left", "Right", "Up", "Down", "A", "B", "Select", "Start",
                "RotLeft", "RotRight", "RotUp", "RotDown", "ZoomIn", "ZoomOut",
                "Menu", "RollBack", "FastForward", "QuickSave", "QuickLoad",
            ],
            InputActions.All.Select(InputActions.NameOf));

        foreach (InputAction action in InputActions.All)
        {
            Assert.Equal(action, InputActions.Parse(InputActions.NameOf(action)));
        }
    }

    /// <summary>
    /// REQ-INP-002: a player's mask is the OR of every source, and releasing one source leaves
    /// the other's bit set — the right arrow plus the pad's south button give <c>0x81</c>.
    /// </summary>
    [Fact]
    public void REQ_INP_002_sources_are_combined_with_a_bitwise_or()
    {
        InputAction keyboard = InputAction.Right;
        InputAction pad = InputAction.A;

        Assert.Equal(0x81, (int)(keyboard | pad));
        Assert.Equal(0x80, (int)(keyboard | InputAction.None));
        Assert.Equal(0x01, (int)(InputAction.None | pad));
    }

    /// <summary>
    /// REQ-INP-004: port 1 is the low byte and port 2 the next one, and only the eight low bits
    /// of a player's mask reach a controller port.
    /// </summary>
    [Fact]
    public void REQ_INP_004_the_word_carries_player_one_low_and_player_two_high()
    {
        ushort word = InputRouting.CoreWord(
            InputAction.Right | InputAction.A,
            InputAction.Up | InputAction.Start);

        Assert.Equal(0x81, word & 0xFF);
        Assert.Equal(0x18, (word >> 8) & 0xFF);
    }

    /// <summary>
    /// REQ-INP-006: the camera and system bits above <c>0x80</c> are never transmitted to the
    /// core — holding RollBack does not change the port byte.
    /// </summary>
    [Fact]
    public void REQ_INP_006_only_the_eight_low_bits_reach_the_core()
    {
        InputAction mask = InputAction.Right | InputAction.A | InputAction.RollBack
            | InputAction.RotRight | InputAction.Menu | InputAction.FastForward
            | InputAction.QuickSave | InputAction.QuickLoad | InputAction.ZoomIn;

        Assert.Equal(0x81, InputActions.ControllerByte(mask));
        Assert.Equal(0x81, InputRouting.CoreWord(mask, InputAction.None));
    }

    /// <summary>
    /// REQ-INP-004: port 2 in <c>Zapper</c> sees no controller button at all even while J2 holds
    /// A; back in <c>Controller</c> it sees it again.
    /// </summary>
    [Fact]
    public void REQ_INP_004_a_zapper_port_carries_no_controller_button()
    {
        ushort zapper = InputRouting.CoreWord(
            InputAction.None,
            InputAction.A,
            ControllerPortType.Controller,
            ControllerPortType.Zapper,
            ZapperState.ReleasedByte);

        Assert.Equal(0x10, (zapper >> 8) & 0xFF);

        ushort controller = InputRouting.CoreWord(
            InputAction.None,
            InputAction.A,
            ControllerPortType.Controller,
            ControllerPortType.Controller);

        Assert.Equal(0x01, (controller >> 8) & 0xFF);
    }

    /// <summary>
    /// REQ-INP-080: the camera is driven by the OR of the masks of the players whose port is a
    /// controller; a player in <c>Zapper</c> does not turn it.
    /// </summary>
    [Fact]
    public void REQ_INP_080_a_zapper_player_does_not_drive_the_camera()
    {
        Assert.Equal(
            InputAction.RotLeft,
            InputRouting.CameraSources(
                InputAction.None,
                InputAction.RotLeft,
                ControllerPortType.Controller,
                ControllerPortType.Controller));

        Assert.Equal(
            InputAction.None,
            InputRouting.CameraSources(
                InputAction.None,
                InputAction.RotLeft,
                ControllerPortType.Controller,
                ControllerPortType.Zapper));

        // J1 still turns it while port 2 is a Zapper.
        Assert.Equal(
            InputAction.RotLeft,
            InputRouting.CameraSources(
                InputAction.RotLeft,
                InputAction.RotRight,
                ControllerPortType.Controller,
                ControllerPortType.Zapper));
    }

    /// <summary>REQ-INP-006: the camera actions map onto the renderer's controls.</summary>
    [Fact]
    public void REQ_INP_080_the_camera_actions_map_onto_the_renderer_controls()
    {
        CameraAction actions = InputActions.ToCameraActions(
            InputAction.RotLeft | InputAction.ZoomOut | InputAction.A);

        Assert.Equal(CameraAction.RotateLeft | CameraAction.ZoomOut, actions);
    }

    /// <summary>
    /// SPEC-08 §5.1, the mask vectors of the default configuration with both ports on
    /// controllers.
    /// </summary>
    [Theory]
    [InlineData(InputAction.Up | InputAction.A, 0x11, 0x11)]
    [InlineData(InputAction.Start | InputAction.Select, 0x0C, 0x0C)]
    [InlineData(InputAction.RotLeft | InputAction.ZoomIn, 0x500, 0x00)]
    [InlineData(InputAction.Down | InputAction.A | InputAction.B, 0x23, 0x23)]
    [InlineData(InputAction.None, 0x00, 0x00)]
    [InlineData(InputAction.RotRight, 0x800, 0x00)]
    [InlineData(InputAction.Menu, 0x4000, 0x00)]
    public void Section_5_1_the_mask_vectors(InputAction mask, int expectedMask, int expectedPortByte)
    {
        Assert.Equal(expectedMask, (int)mask);
        Assert.Equal(expectedPortByte, InputRouting.CoreWord(mask, InputAction.None) & 0xFF);
    }

    /// <summary>
    /// REQ-INP-071 and REQ-INP-072: the script pad is OR-merged into the mask of the player it
    /// is attached to and can never take a bit away.
    /// </summary>
    [Fact]
    public void REQ_INP_071_the_script_pad_is_merged_into_its_player_only()
    {
        (InputAction first, InputAction second) = InputRouting.MergeScriptPad(
            InputAction.Right,
            InputAction.Left,
            InputAction.Start,
            padIndex: 0);

        Assert.Equal(InputAction.Right | InputAction.Start, first);
        Assert.Equal(InputAction.Left, second);

        (first, second) = InputRouting.MergeScriptPad(
            InputAction.Right,
            InputAction.Left,
            InputAction.Start,
            padIndex: 1);

        Assert.Equal(InputAction.Right, first);
        Assert.Equal(InputAction.Left | InputAction.Start, second);

        // REQ-INP-072: a pad that returns 0 leaves the physical input untouched.
        (first, second) = InputRouting.MergeScriptPad(
            InputAction.Right,
            InputAction.Left,
            InputAction.None,
            padIndex: 0);

        Assert.Equal(InputAction.Right, first);
        Assert.Equal(InputAction.Left, second);
    }

    /// <summary>
    /// REQ-INP-100 and REQ-INP-105: the plain hotkeys are read on the OR of both players'
    /// masks, whatever the port types are — J2's Menu key opens the menu.
    /// </summary>
    [Fact]
    public void REQ_INP_100_hotkeys_are_read_on_both_players()
    {
        HotkeyTracker tracker = new();

        Assert.Equal(HotkeyEvents.None, tracker.Update(InputAction.None, InputAction.Menu, 0.1f));
        Assert.Equal(
            HotkeyEvents.MenuShort,
            tracker.Update(InputAction.None, InputAction.None, 0.0f));
    }

    /// <summary>
    /// REQ-INP-104: the quick-slot combinations are evaluated player by player, so Select held
    /// by J1 and the stick flicked by J2 trigger nothing.
    /// </summary>
    [Fact]
    public void REQ_INP_104_the_combinations_are_evaluated_player_by_player()
    {
        HotkeyTracker split = new();
        split.Update(InputAction.Select, InputAction.None, 0.2f);
        split.Update(InputAction.Select, InputAction.RotDown, 0.02f);
        Assert.Equal(
            HotkeyEvents.None,
            split.Update(InputAction.Select, InputAction.None, 0.02f));

        // The same player holding Select and releasing RotDown does save.
        HotkeyTracker same = new();
        same.Update(InputAction.None, InputAction.Select, 0.2f);
        same.Update(InputAction.None, InputAction.Select | InputAction.RotDown, 0.02f);
        Assert.True(same
            .Update(InputAction.None, InputAction.Select, 0.02f)
            .HasFlag(HotkeyEvents.QuickSave));
    }

    /// <summary>
    /// REQ-INP-104: Select must have been held for more than 0.1 s before the combination arms,
    /// so pressing both together does nothing.
    /// </summary>
    [Fact]
    public void REQ_INP_104_the_combination_needs_select_held_for_a_tenth_of_a_second()
    {
        HotkeyTracker tracker = new();

        tracker.Update(InputAction.Select | InputAction.RotDown, 0.02f);
        Assert.Equal(
            HotkeyEvents.None,
            tracker.Update(InputAction.Select, 0.02f));
    }

    /// <summary>
    /// REQ-INP-122: the default libretro correspondence of §4.4, with A on the east button and B
    /// on the south one, and no hotkey exposed as a button.
    /// </summary>
    [Fact]
    public void REQ_INP_122_the_libretro_table_is_the_one_of_section_4_4()
    {
        Assert.Equal(InputAction.Left, LibretroInputMap.ActionOf(RetroPadButton.Left));
        Assert.Equal(InputAction.Right, LibretroInputMap.ActionOf(RetroPadButton.Right));
        Assert.Equal(InputAction.Up, LibretroInputMap.ActionOf(RetroPadButton.Up));
        Assert.Equal(InputAction.Down, LibretroInputMap.ActionOf(RetroPadButton.Down));

        // A = east, B = south, the usual libretro NES convention, different from §4.3 on purpose.
        Assert.Equal(InputAction.A, LibretroInputMap.ActionOf(RetroPadButton.A));
        Assert.Equal(InputAction.B, LibretroInputMap.ActionOf(RetroPadButton.B));
        Assert.Equal(InputAction.Select, LibretroInputMap.ActionOf(RetroPadButton.Select));
        Assert.Equal(InputAction.Start, LibretroInputMap.ActionOf(RetroPadButton.Start));
        Assert.Equal(InputAction.ZoomIn, LibretroInputMap.ActionOf(RetroPadButton.L2));
        Assert.Equal(InputAction.ZoomOut, LibretroInputMap.ActionOf(RetroPadButton.R2));

        Assert.Equal(InputAction.RotLeft, LibretroInputMap.ActionOf(RetroAnalogAxis.RightStickLeft));
        Assert.Equal(InputAction.RotRight, LibretroInputMap.ActionOf(RetroAnalogAxis.RightStickRight));
        Assert.Equal(InputAction.RotUp, LibretroInputMap.ActionOf(RetroAnalogAxis.RightStickUp));
        Assert.Equal(InputAction.RotDown, LibretroInputMap.ActionOf(RetroAnalogAxis.RightStickDown));

        // The hotkeys belong to the frontend host and are not mapped.
        foreach ((RetroPadButton _, InputAction action) in LibretroInputMap.Buttons)
        {
            Assert.Equal(InputAction.None, action & LibretroInputMap.HostReservedActions);
        }
    }

    /// <summary>
    /// REQ-INP-105: the directional actions walk the tabs on their rising edge while the menu is
    /// open, and are suspended while a binding capture is listening (REQ-INP-047).
    /// </summary>
    [Fact]
    public void REQ_INP_105_menu_navigation_reads_rising_edges_only_while_paused()
    {
        MenuNavigation navigation = new();

        // Not paused: nothing is produced, and the held key is consumed.
        Assert.Equal(
            MenuNavigationEvents.None,
            navigation.Update(InputAction.Right, paused: false));

        Assert.Equal(
            MenuNavigationEvents.None,
            navigation.Update(InputAction.Right, paused: true));

        navigation.Update(InputAction.None, paused: true);
        Assert.Equal(
            MenuNavigationEvents.Next,
            navigation.Update(InputAction.Right, paused: true));

        // Held down: no repeat.
        Assert.Equal(
            MenuNavigationEvents.None,
            navigation.Update(InputAction.Right, paused: true));

        // REQ-INP-047: suspended while a capture listens.
        navigation.Update(InputAction.None, paused: true);
        Assert.Equal(
            MenuNavigationEvents.None,
            navigation.Update(InputAction.Right, paused: true, suspended: true));
    }

    /// <summary>
    /// REQ-INP-110 and REQ-INP-111: the cursor is hidden at the twentieth consecutive 0.1 s
    /// check without movement, comes back on any movement, and is always visible outside a
    /// running game.
    /// </summary>
    [Fact]
    public void REQ_INP_110_the_cursor_hides_after_two_still_seconds()
    {
        CursorAutoHide cursor = new();

        // 1.9 s still: still visible.
        for (int tick = 0; tick < 19; tick++)
        {
            Assert.True(cursor.Update(gameRunning: true, pointerMoved: false, 0.1f));
        }

        // The twentieth check hides it (2.0 s).
        Assert.False(cursor.Update(gameRunning: true, pointerMoved: false, 0.1f));

        // Any movement shows it again immediately and resets the counter.
        Assert.True(cursor.Update(gameRunning: true, pointerMoved: true, 0.1f));
        Assert.Equal(0, cursor.StillChecks);

        // REQ-INP-111: outside a running game it is visible whatever the counter says.
        for (int tick = 0; tick < 25; tick++)
        {
            cursor.Update(gameRunning: true, pointerMoved: false, 0.1f);
        }

        Assert.False(cursor.CursorVisible);
        Assert.True(cursor.Update(gameRunning: false, pointerMoved: false, 0.1f));
    }

    /// <summary>
    /// REQ-INP-007: there is no turbo and no auto-repeat action in the default list.
    /// </summary>
    [Fact]
    public void REQ_INP_007_there_is_no_turbo_action()
    {
        Assert.DoesNotContain(
            InputActions.All.Select(InputActions.NameOf),
            name => name.Contains("Turbo", StringComparison.OrdinalIgnoreCase)
                || name.Contains("Repeat", StringComparison.OrdinalIgnoreCase));

        // Every action of the enumeration is one of the 19 of §4.1, bar the three masks.
        foreach (InputAction action in Enum.GetValues<InputAction>())
        {
            if (action is InputAction.None
                or InputAction.ControllerByte
                or InputAction.CameraMask
                or InputAction.HotkeyMask)
            {
                continue;
            }

            Assert.Contains(action, InputActions.All);
        }
    }

    /// <summary>
    /// REQ-INP-033 and REQ-INP-081: a camera action coming from a stick is digital — the
    /// rotation runs at 30°/s once the threshold is crossed, so 50 % and 100 % of travel turn
    /// the camera by the same amount.
    /// </summary>
    [Fact]
    public void REQ_INP_033_a_stick_turns_the_camera_at_a_constant_speed()
    {
        float half = Rotate(0.5f);
        float full = Rotate(1.0f);

        Assert.Equal(half, full, 4);

        // REQ-INP-081: 3 s of RotRight add 90° ± 3° to the yaw setpoint.
        Assert.InRange(Rotate(1.0f, seconds: 3f), 87f, 93f);

        // Below the threshold nothing moves at all (REQ-INP-030).
        Assert.Equal(0f, Rotate(0.2f), 4);

        static float Rotate(float travel, float seconds = 1f)
        {
            FakeGamepad pad = new("Pad");
            pad.Axis(StandardGamepad.RightStickX, travel);

            BindingSet set = DefaultBindings.CreateGamepad();
            InputAction mask = set.Sample(
                control => AnalogCalibration.IsPressed(pad.Value(control)));

            SceneCamera camera = new();
            camera.ResetView();
            float before = camera.Current.Angles.Y;
            CameraControls.Apply(camera, InputActions.ToCameraActions(mask), seconds);
            return camera.Current.Angles.Y - before;
        }
    }

    /// <summary>
    /// REQ-INP-082: one second of ZoomOut adds 70 scene units to the camera distance, and
    /// ZoomIn wins over ZoomOut.
    /// </summary>
    [Fact]
    public void REQ_INP_082_zoom_moves_seventy_units_per_second()
    {
        SceneCamera camera = new();
        camera.ResetView();
        float start = camera.Current.Distance;

        CameraControls.Apply(camera, CameraAction.ZoomOut, 1f);
        Assert.InRange(camera.Current.Distance - start, 67f, 73f);

        // ZoomIn wins when both are held.
        float held = camera.Current.Distance;
        CameraControls.Apply(camera, CameraAction.ZoomIn | CameraAction.ZoomOut, 1f);
        Assert.InRange(held - camera.Current.Distance, 67f, 73f);
    }

    /// <summary>
    /// REQ-INP-106 with Q-030: the fixed shortcuts that act without a modifier use no key of the
    /// default table of §4.2, so the default game keys keep working in game.
    /// </summary>
    // Q-030
    [Fact]
    public void REQ_INP_106_no_bare_fixed_shortcut_uses_a_default_game_key()
    {
        HashSet<Silk.NET.Input.Key> gameKeys =
            [.. DefaultBindings.PlayerOneKeyboard.Select(static entry => entry.Key)];

        foreach (Silk.NET.Input.Key key in ZaxApplication.BareShortcutKeys)
        {
            Assert.DoesNotContain(key, gameKeys);
        }
    }

    /// <summary>
    /// REQ-INP-061: the Zapper is active only while a game runs and a port is in
    /// <c>Zapper</c>.
    /// </summary>
    [Fact]
    public void REQ_INP_061_the_zapper_is_active_only_with_a_zapper_port_in_a_running_game()
    {
        Assert.False(InputRouting.ZapperActive(
            gameRunning: true,
            ControllerPortType.Controller,
            ControllerPortType.Controller));

        Assert.True(InputRouting.ZapperActive(
            gameRunning: true,
            ControllerPortType.Controller,
            ControllerPortType.Zapper));

        Assert.False(InputRouting.ZapperActive(
            gameRunning: false,
            ControllerPortType.Controller,
            ControllerPortType.Zapper));
    }
}
