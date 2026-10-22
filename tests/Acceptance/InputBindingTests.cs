using System.Numerics;
using Silk.NET.Input;
using Zax.App.Input;
using Zax.App.Persistence;
using Zax.Platform.Input;

namespace Zax.Acceptance.Tests;

/// <summary>
/// SPEC-08 §3.3, §3.5, §3.6 and §3.13: gamepad support and assignment, remapping and its capture
/// rules (AMD-015, Q-033), the persisted binding data and the libretro abstraction.
/// </summary>
public sealed class InputBindingTests
{
    /// <summary>
    /// REQ-INP-020 and §4.3: a recognised pad gets the Xbox-style defaults — the hat drives the
    /// directions, south is A and west is B.
    /// </summary>
    [Fact]
    public void REQ_INP_020_a_recognised_pad_gets_the_defaults_of_the_table()
    {
        InputBindings bindings = new();
        BindingSet set = bindings.Gamepad("Xbox Controller");

        (InputAction Action, PhysicalControl Control)[] table =
        [
            (InputAction.Left, PhysicalControl.Button(StandardGamepad.DPadLeft)),
            (InputAction.Right, PhysicalControl.Button(StandardGamepad.DPadRight)),
            (InputAction.Up, PhysicalControl.Button(StandardGamepad.DPadUp)),
            (InputAction.Down, PhysicalControl.Button(StandardGamepad.DPadDown)),
            (InputAction.A, PhysicalControl.Button(StandardGamepad.South)),
            (InputAction.B, PhysicalControl.Button(StandardGamepad.West)),
            (InputAction.RollBack, PhysicalControl.Button(StandardGamepad.North)),
            (InputAction.FastForward, PhysicalControl.Button(StandardGamepad.East)),
            (InputAction.Select, PhysicalControl.Button(StandardGamepad.Back)),
            (InputAction.Start, PhysicalControl.Button(StandardGamepad.Start)),
            (InputAction.RotLeft, PhysicalControl.AxisNegative(StandardGamepad.RightStickX)),
            (InputAction.RotRight, PhysicalControl.AxisPositive(StandardGamepad.RightStickX)),
            (InputAction.RotUp, PhysicalControl.AxisPositive(StandardGamepad.RightStickY)),
            (InputAction.RotDown, PhysicalControl.AxisNegative(StandardGamepad.RightStickY)),
            (InputAction.ZoomIn, PhysicalControl.Trigger(StandardGamepad.LeftTrigger)),
            (InputAction.ZoomOut, PhysicalControl.Trigger(StandardGamepad.RightTrigger)),
            (InputAction.Menu, PhysicalControl.Button(StandardGamepad.RightStick)),
        ];

        foreach ((InputAction action, PhysicalControl control) in table)
        {
            Assert.Equal(control, set.Get(action));
        }

        // The left stick, its click, both bumpers and Guide are deliberately unbound, and
        // QuickSave / QuickLoad have no default either.
        Assert.Equal(table.Length, set.Count);
        Assert.Equal(PhysicalControl.None, set.Get(InputAction.QuickSave));
        Assert.Equal(PhysicalControl.None, set.Get(InputAction.QuickLoad));

        PhysicalControl[] mustStayFree =
        [
            PhysicalControl.Button(StandardGamepad.LeftStick),
            PhysicalControl.Button(StandardGamepad.LeftBumper),
            PhysicalControl.Button(StandardGamepad.RightBumper),
            PhysicalControl.Button(StandardGamepad.Guide),
            PhysicalControl.AxisPositive(StandardGamepad.LeftStickX),
            PhysicalControl.AxisNegative(StandardGamepad.LeftStickX),
            PhysicalControl.AxisPositive(StandardGamepad.LeftStickY),
            PhysicalControl.AxisNegative(StandardGamepad.LeftStickY),
        ];

        foreach (PhysicalControl free in mustStayFree)
        {
            Assert.DoesNotContain(free, set.Bindings.Values);
        }
    }

    /// <summary>
    /// REQ-INP-021: a pad with no standard mapping is listed and has no useful default, but its
    /// raw controls can be bound and then drive the player.
    /// </summary>
    [Fact]
    public void REQ_INP_021_a_generic_pad_has_no_defaults_but_can_be_remapped()
    {
        InputBindings bindings = new();
        FakeGamepad generic = new("Generic Joystick", standard: false);
        FakeGamepadHub hub = new FakeGamepadHub().Plug(generic);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        generic.Press(7);
        Assert.Equal(InputAction.None, InputSupport.Sample(source, 0));
        Assert.Contains(generic, source.Pads.Available);
        Assert.Equal(0, bindings.Gamepad("Generic Joystick", standard: false).Count);

        Assert.True(bindings.Gamepad("Generic Joystick", standard: false)
            .Set(InputAction.Start, PhysicalControl.Button(7)));
        Assert.Equal(InputAction.Start, InputSupport.Sample(source, 0));
    }

    /// <summary>REQ-INP-022: a VR controller is never offered and never assigned.</summary>
    [Theory]
    [InlineData("Oculus Touch Right")]
    [InlineData("HTC Vive Controller")]
    [InlineData("Valve Knuckles")]
    [InlineData("Spatial Controller")]
    public void REQ_INP_022_virtual_reality_devices_are_never_offered(string identity)
    {
        Assert.True(GamepadIdentity.IsVirtualReality(identity));

        InputBindings bindings = new();
        FakeGamepad vr = new(identity);
        FakeGamepadHub hub = new FakeGamepadHub().Plug(vr);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        Assert.Empty(source.Pads.Available);

        vr.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        Assert.Null(source.Pads.Device(0));
        Assert.Null(source.Pads.Device(1));
    }

    /// <summary>
    /// REQ-INP-023 rule 2 and its criterion: with no configuration and two pads plugged in,
    /// pressing a button of M2 then of M1 gives M2 to J1 and M1 to J2.
    /// </summary>
    [Fact]
    public void REQ_INP_023_the_first_pad_used_goes_to_player_one()
    {
        InputBindings bindings = new();
        FakeGamepad m1 = new("M1");
        FakeGamepad m2 = new("M2");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(m1).Plug(m2);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        m2.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        m2.ReleaseAll();

        Assert.Equal("M2", source.Pads.Device(0)?.Identity);
        Assert.Null(source.Pads.Device(1));

        m1.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        Assert.Equal("M2", source.Pads.Device(0)?.Identity);
        Assert.Equal("M1", source.Pads.Device(1)?.Identity);

        // "Relaunch the program": the identities were remembered, so rule 1 puts them back.
        Assert.Equal("M2", bindings.LastGamepad(0));
        Assert.Equal("M1", bindings.LastGamepad(1));

        GamepadAssignment again = new(hub, bindings);
        again.SkipStartupDelay();
        Assert.Equal("M2", again.Device(0)?.Identity);
        Assert.Equal("M1", again.Device(1)?.Identity);
    }

    /// <summary>REQ-INP-023 rule 1: the reconnection happens about 0.5 s after the launch.</summary>
    [Fact]
    public void REQ_INP_023_the_startup_reconnection_waits_half_a_second()
    {
        InputBindings bindings = new();
        bindings.SetLastGamepad(0, "M1");
        FakeGamepad m1 = new("M1");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(m1);

        GamepadAssignment pads = new(hub, bindings);
        Assert.Equal(0.5f, GamepadAssignment.StartupDelaySeconds);

        pads.Update(0.4f);
        Assert.False(pads.StartupComplete);
        Assert.Null(pads.Device(0));

        pads.Update(0.2f);
        Assert.True(pads.StartupComplete);
        Assert.Equal("M1", pads.Device(0)?.Identity);
    }

    /// <summary>
    /// REQ-INP-024: unplugging the assigned pad drops the player's pad bits at the next sample;
    /// plugging it back and pressing a button gives it back to them.
    /// </summary>
    [Fact]
    public void REQ_INP_024_unplugging_a_pad_drops_its_bits_at_the_next_sample()
    {
        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        pad.Press(StandardGamepad.DPadRight);
        Assert.Equal(InputAction.Right, InputSupport.Sample(source, 0));

        hub.Unplug(pad);
        Assert.Equal(InputAction.None, InputSupport.Sample(source, 0));
        Assert.Null(source.Pads.Device(0));

        // The identity is still remembered (REQ-INP-050 item 3).
        Assert.Equal("Pad", bindings.LastGamepad(0));

        hub.Plug(pad);
        Assert.Equal(InputAction.Right, InputSupport.Sample(source, 0));
    }

    /// <summary>
    /// REQ-INP-025: a gamepad set belongs to the device identity, not to the player: it survives
    /// an unplug and follows the pad from J1 to J2, and another model keeps its own.
    /// </summary>
    [Fact]
    public void REQ_INP_025_the_gamepad_set_belongs_to_the_device()
    {
        InputBindings bindings = new();
        FakeGamepad modelX = new("Model X");
        FakeGamepad modelY = new("Model Y");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(modelX).Plug(modelY);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        modelX.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        modelX.ReleaseAll();

        bindings.Gamepad("Model X").Set(InputAction.QuickSave, PhysicalControl.Button(StandardGamepad.North));

        hub.Unplug(modelX);
        InputSupport.Sample(source, 0);
        hub.Plug(modelX);
        modelX.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);

        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            bindings.Gamepad("Model X").Get(InputAction.QuickSave));
        Assert.Equal(PhysicalControl.None, bindings.Gamepad("Model Y").Get(InputAction.QuickSave));
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            bindings.Gamepad("Model Y").Get(InputAction.RollBack));

        // The same pad moved to J2 takes its bindings with it.
        source.Pads.Assign(1, modelX);
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            source.Pads.GamepadBindings(1)!.Get(InputAction.QuickSave));
    }

    /// <summary>REQ-INP-026: the displayed identity is the readable name of the device.</summary>
    [Fact]
    public void REQ_INP_026_the_displayed_identity_is_the_readable_name()
    {
        FakeGamepad xbox = new("XInput Controller");
        FakeGamepad generic = new("Unknown HID 0x1234", standard: false);

        Assert.Equal("XInput Controller", xbox.DisplayName);
        Assert.Equal("Unknown HID 0x1234", generic.DisplayName);
        Assert.Equal(xbox.DisplayName, xbox.Identity);
    }

    /// <summary>
    /// REQ-INP-041 and DEC-08-1: assigning a pad to a player takes it away from the other one.
    /// Choosing "none" removes the player's pad, and the list follows plug and unplug.
    /// </summary>
    [Fact]
    public void REQ_INP_041_the_manual_assignment_removes_the_pad_from_the_other_player()
    {
        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        GamepadAssignment pads = new(hub, bindings);
        pads.SkipStartupDelay();

        pads.Assign(0, pad);
        Assert.Equal("Pad", pads.Device(0)?.Identity);

        pads.Assign(1, pad);
        Assert.Null(pads.Device(0));
        Assert.Equal("Pad", pads.Device(1)?.Identity);
        Assert.Null(bindings.LastGamepad(0));

        pads.Assign(1, null);
        Assert.Null(pads.Device(1));
        Assert.Null(bindings.LastGamepad(1));

        FakeGamepad second = new("Pad 2");
        Assert.Single(pads.Available);
        hub.Plug(second);
        Assert.Equal(2, pads.Available.Count);
        hub.Unplug(second);
        Assert.Single(pads.Available);
    }

    /// <summary>
    /// REQ-INP-042 and REQ-INP-045: activating a field erases the binding at once and starts
    /// listening; the first eligible command detected becomes the binding; stopping the listening
    /// leaves the action empty.
    /// </summary>
    [Fact]
    public void REQ_INP_042_the_capture_clears_then_binds_the_first_command()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        RebindingSession session = new(bindings);

        Assert.True(session.Begin(InputAction.B));
        Assert.True(session.IsListening);
        Assert.Equal(PhysicalControl.None, bindings.Keyboard(0).Get(InputAction.B));

        // AMD-015 (Q-033): the first sample only records what is already held.
        session.Capture(keyboard, null);
        keyboard.Press(Key.C);
        Assert.Equal(PhysicalControl.Key((int)Key.C), session.Capture(keyboard, null));
        Assert.False(session.IsListening);
        Assert.Equal(PhysicalControl.Key((int)Key.C), bindings.Keyboard(0).Get(InputAction.B));

        // REQ-INP-045: stopping the listening leaves the action empty.
        Assert.True(session.Begin(InputAction.Start));
        session.Stop();
        Assert.Equal(PhysicalControl.None, bindings.Keyboard(0).Get(InputAction.Start));
    }

    /// <summary>
    /// AMD-015 (Q-033): a command already held when the field is activated is ignored until it is
    /// released, so the click or key press that opened the field never becomes the binding.
    /// </summary>
    [Fact]
    public void REQ_INP_042_a_command_held_at_the_activation_is_ignored()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        RebindingSession session = new(bindings);

        keyboard.Press(Key.Enter);
        session.Begin(InputAction.A);

        Assert.Equal(PhysicalControl.None, session.Capture(keyboard, null));
        Assert.Equal(PhysicalControl.None, session.Capture(keyboard, null));
        Assert.True(session.IsListening);

        keyboard.Release(Key.Enter);
        session.Capture(keyboard, null);
        keyboard.Press(Key.Enter);
        Assert.Equal(PhysicalControl.Key((int)Key.Enter), session.Capture(keyboard, null));
    }

    /// <summary>
    /// REQ-INP-042, Gamepad mode: a left-stick half-axis is an eligible command, and no key is.
    /// </summary>
    [Fact]
    public void REQ_INP_042_a_gamepad_capture_takes_a_half_axis_and_no_key()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        GamepadAssignment pads = new(hub, bindings);
        pads.SkipStartupDelay();
        pads.Assign(0, pad);

        RebindingSession session = new(bindings, pads);
        session.SelectDeviceKind(BindingDeviceKind.Gamepad);
        Assert.True(session.Begin(InputAction.A));

        session.Capture(keyboard, pad);
        keyboard.Press(Key.K);
        Assert.Equal(PhysicalControl.None, session.Capture(keyboard, pad));

        pad.Axis(StandardGamepad.LeftStickY, 1f);
        Assert.Equal(
            PhysicalControl.AxisPositive(StandardGamepad.LeftStickY),
            session.Capture(keyboard, pad));
    }

    /// <summary>REQ-INP-043: at most one binding per action per device kind.</summary>
    [Fact]
    public void REQ_INP_043_one_binding_per_action_and_per_device_kind()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        RebindingSession session = new(bindings);

        Capture(Key.K);
        Capture(Key.L);

        Assert.Equal(PhysicalControl.Key((int)Key.L), bindings.Keyboard(0).Get(InputAction.A));

        DesktopInputSource source = InputSupport.Source(bindings, new FakeGamepadHub(), keyboard);
        keyboard.ReleaseAll().Press(Key.K);
        Assert.Equal(InputAction.None, InputSupport.Sample(source, 0));
        keyboard.ReleaseAll().Press(Key.L);
        Assert.Equal(InputAction.A, InputSupport.Sample(source, 0));

        void Capture(Key key)
        {
            keyboard.ReleaseAll();
            session.Begin(InputAction.A);
            session.Capture(keyboard, null);
            keyboard.Press(key);
            session.Capture(keyboard, null);
        }
    }

    /// <summary>
    /// REQ-INP-044: no conflict detection — one key bound to two actions triggers both, and
    /// binding a command does not remove it from the other actions.
    /// </summary>
    [Fact]
    public void REQ_INP_044_one_command_may_drive_several_actions()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Key((int)Key.X));
        bindings.Keyboard(0).Set(InputAction.B, PhysicalControl.Key((int)Key.X));

        DesktopInputSource source = InputSupport.Source(bindings, new FakeGamepadHub(), keyboard);
        keyboard.Press(Key.X);

        Assert.Equal((InputAction)0x03, InputSupport.Sample(source, 0));
    }

    /// <summary>
    /// REQ-INP-045: the listening also stops on a player change, a device-kind change, a pad
    /// change and on activating another field, each time without creating a binding.
    /// </summary>
    [Fact]
    public void REQ_INP_045_every_documented_event_stops_the_listening()
    {
        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        GamepadAssignment pads = new(hub, bindings);
        pads.SkipStartupDelay();
        pads.Assign(0, pad);
        RebindingSession session = new(bindings, pads);

        session.Begin(InputAction.Start);
        session.SelectPlayer(1);
        Assert.False(session.IsListening);

        session.SelectPlayer(0);
        session.Begin(InputAction.Start);
        session.SelectDeviceKind(BindingDeviceKind.Gamepad);
        Assert.False(session.IsListening);

        session.SelectDeviceKind(BindingDeviceKind.Keyboard);
        session.Begin(InputAction.Start);
        session.NotifyGamepadChanged();
        Assert.False(session.IsListening);

        session.Begin(InputAction.Start);
        session.Begin(InputAction.Select);
        Assert.True(session.IsListening);
        Assert.Equal(BindingTarget.Of(InputAction.Select), session.Target);
        Assert.Equal(PhysicalControl.None, bindings.Keyboard(0).Get(InputAction.Start));

        session.Close();
        Assert.False(session.IsListening);
    }

    /// <summary>
    /// REQ-INP-046: in Gamepad mode a player without a pad shows 19 empty fields, and activating
    /// one starts no listening.
    /// </summary>
    [Fact]
    public void REQ_INP_046_gamepad_mode_without_a_pad_shows_nothing()
    {
        InputBindings bindings = new();
        GamepadAssignment pads = new(new FakeGamepadHub(), bindings);
        pads.SkipStartupDelay();
        RebindingSession session = new(bindings, pads);

        session.SelectPlayer(1);
        session.SelectDeviceKind(BindingDeviceKind.Gamepad);

        Assert.Null(session.ActiveSet);
        foreach (InputAction action in InputActions.All)
        {
            Assert.False(session.Begin(action));
            Assert.False(session.IsListening);
        }
    }

    /// <summary>
    /// REQ-INP-047 and DEC-08-2: while a capture is listening no action mask is produced, so the
    /// interface navigation is suspended and the captured command triggers nothing else.
    /// </summary>
    [Fact]
    public void REQ_INP_047_a_capture_suspends_the_actions_and_the_navigation()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        DesktopInputSource source = InputSupport.Source(bindings, new FakeGamepadHub(), keyboard);
        MenuNavigation navigation = new();

        source.Rebinding.Begin(InputAction.Start);

        // The first sample after the activation only records what is already held (Q-033).
        Zax.App.Input.InputSnapshot first = source.Sample(1f / 60f);
        Assert.Equal(InputAction.None, first.PlayerOne);
        source.EndSample();

        keyboard.Press(Key.Right);
        Zax.App.Input.InputSnapshot second = source.Sample(1f / 60f);

        Assert.Equal(PhysicalControl.Key((int)Key.Right), bindings.Keyboard(0).Get(InputAction.Start));
        Assert.Equal(InputAction.None, second.PlayerOne);
        Assert.True(source.Rebinding.SuppressesActions);
        Assert.Equal(
            MenuNavigationEvents.None,
            navigation.Update(second.Both, paused: true, suspended: source.Rebinding.SuppressesActions));
        source.EndSample();

        // Once the sample is over the interface answers again. The right arrow now raises both
        // Start — just captured — and Right, which REQ-INP-044 allows (no conflict detection).
        Zax.App.Input.InputSnapshot third = source.Sample(1f / 60f);
        Assert.Equal(InputAction.Start | InputAction.Right, third.PlayerOne);
        source.EndSample();
    }

    /// <summary>
    /// REQ-INP-048: bindings are global. The model holds one keyboard set per player and one set
    /// per gamepad identity — nothing is keyed by game.
    /// </summary>
    [Fact]
    public void REQ_INP_048_the_bindings_are_global()
    {
        InputBindings bindings = new();
        bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Key((int)Key.K));

        // The model holds exactly one keyboard set per player and one set per gamepad identity:
        // there is no third dimension a game could index, and the same instance is handed back.
        Assert.Same(bindings.Keyboard(0), bindings.Keyboard(0));
        Assert.Equal(PhysicalControl.Key((int)Key.K), bindings.Keyboard(0).Get(InputAction.A));

        bindings.Gamepad("Pad");
        Assert.Single(bindings.GamepadIdentities);
        Assert.Same(bindings.Gamepad("Pad"), bindings.Gamepad("Pad"));

        // Only the port type varies per game (REQ-INP-060), and it is not part of this model.
        Assert.Null(typeof(InputBindings).GetProperty("PortType"));
        Assert.Null(typeof(InputBindings).GetMethod("PortType"));
    }

    /// <summary>REQ-INP-049: the recommended reset puts a set back to its default table.</summary>
    [Fact]
    public void REQ_INP_049_a_set_can_be_reset_to_its_defaults()
    {
        InputBindings bindings = new();
        foreach (InputAction action in InputActions.All)
        {
            bindings.Keyboard(0).Set(action, PhysicalControl.Key((int)Key.K));
        }

        bindings.ResetKeyboard(0);
        Assert.Equal(PhysicalControl.Key((int)Key.Left), bindings.Keyboard(0).Get(InputAction.Left));
        Assert.Equal(17, bindings.Keyboard(0).Count);

        bindings.Gamepad("Pad").Set(InputAction.A, PhysicalControl.Button(StandardGamepad.North));
        bindings.ResetGamepad("Pad");
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.South),
            bindings.Gamepad("Pad").Get(InputAction.A));

        bindings.ResetKeyboard(1);
        Assert.Equal(0, bindings.Keyboard(1).Count);
    }

    /// <summary>
    /// REQ-INP-050, REQ-INP-051 and SPEC-09 REQ-PERS-065: the three pieces of data survive a
    /// session, are written after each change, and the file carries a schema version.
    /// </summary>
    [Fact]
    public void REQ_INP_050_the_three_pieces_of_data_survive_a_session()
    {
        using TemporaryRoot root = new();
        InputBindingStore store = new(root.Paths);

        InputBindings first = new();
        DesktopInputSource source = new(first, new FakeGamepadHub(), store);

        first.Keyboard(1).Set(InputAction.Up, PhysicalControl.Key((int)Key.W));
        first.Gamepad("Pad X").Set(InputAction.QuickSave, PhysicalControl.Button(StandardGamepad.North));
        first.SetLastGamepad(1, "Pad X");

        // REQ-INP-051: each change wrote the file; no explicit save was needed.
        Assert.True(File.Exists(root.Paths.InputFile));
        Assert.Contains("\"schemaVersion\"", File.ReadAllText(root.Paths.InputFile), StringComparison.Ordinal);
        source.Detach();

        InputBindings second = new();
        new InputBindingStore(root.Paths).Load(second);

        Assert.Equal(PhysicalControl.Key((int)Key.W), second.Keyboard(1).Get(InputAction.Up));
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            second.Gamepad("Pad X").Get(InputAction.QuickSave));
        Assert.Equal("Pad X", second.LastGamepad(1));
        Assert.Null(second.LastGamepad(0));
    }

    /// <summary>
    /// REQ-INP-052: an absent or unreadable binding file starts the program on the defaults
    /// without a blocking error, and the unreadable one is set aside.
    /// </summary>
    [Fact]
    public void REQ_INP_052_a_corrupt_binding_file_falls_back_on_the_defaults()
    {
        using TemporaryRoot root = new();

        InputBindings absent = new();
        InputBindingStore store = new(root.Paths);
        store.Load(absent);
        Assert.Null(store.Message);
        Assert.Equal(PhysicalControl.Key((int)Key.X), absent.Keyboard(0).Get(InputAction.A));

        File.WriteAllText(root.Paths.InputFile, "{{{ not json");
        InputBindings corrupt = new();
        InputBindingStore second = new(root.Paths);
        second.Load(corrupt);

        Assert.Equal(InputBindingStore.UnreadableMessage, second.Message);
        Assert.Equal(PhysicalControl.Key((int)Key.X), corrupt.Keyboard(0).Get(InputAction.A));
        Assert.NotEmpty(Directory.GetFiles(root.Paths.Root, "input.json.corrupt-*"));
    }

    /// <summary>
    /// REQ-INP-052, single malformed entry: it degrades to "unbound" instead of losing the file.
    /// </summary>
    [Fact]
    public void REQ_INP_052_one_malformed_entry_does_not_lose_the_file()
    {
        using TemporaryRoot root = new();
        File.WriteAllText(
            root.Paths.InputFile,
            """
            { "schemaVersion": 1,
              "keyboard": { "0": { "A": "key:zzz", "B": "key:67" } },
              "gamepads": {}, "lastGamepad": { "0": null, "1": null } }
            """);

        InputBindings bindings = new();
        InputBindingStore store = new(root.Paths);
        store.Load(bindings);

        Assert.Equal(PhysicalControl.None, bindings.Keyboard(0).Get(InputAction.A));
        Assert.Equal(PhysicalControl.Key(67), bindings.Keyboard(0).Get(InputAction.B));
    }

    /// <summary>
    /// REQ-INP-120: the snapshot is platform independent and carries everything the logic needs,
    /// and a test with no window can inject one and read the byte the core gets.
    /// </summary>
    [Fact]
    public void REQ_INP_120_the_snapshot_is_the_whole_contract()
    {
        Zax.App.Input.InputSnapshot snapshot = new(
            InputAction.A | InputAction.Right,
            InputAction.Start,
            new Vector2(100f, 50f),
            new Vector2(640f, 480f),
            LeftButton: true,
            RightButton: true,
            Wheel: -2f,
            PointerOverUi: true,
            PointerMoved: true);

        Assert.Equal(InputAction.A | InputAction.Right, snapshot.Player(0));
        Assert.Equal(InputAction.Start, snapshot.Player(1));
        Assert.Equal(InputAction.A | InputAction.Right | InputAction.Start, snapshot.Both);
        Assert.Equal(0x81, InputRouting.CoreWord(snapshot.PlayerOne, snapshot.PlayerTwo) & 0xFF);
        Assert.Equal(0x08, (InputRouting.CoreWord(snapshot.PlayerOne, snapshot.PlayerTwo) >> 8) & 0xFF);

        // No windowing or gamepad type appears in the snapshot's own surface.
        foreach (var property in typeof(Zax.App.Input.InputSnapshot).GetProperties())
        {
            string ns = property.PropertyType.Namespace ?? string.Empty;
            Assert.True(
                ns.StartsWith("System", StringComparison.Ordinal)
                || ns.StartsWith("Zax.App.Input", StringComparison.Ordinal),
                $"{property.Name} exposes {property.PropertyType.FullName}");
        }
    }

    /// <summary>
    /// REQ-INP-121: the logic above the snapshot depends on no binding, no dead zone and no pad
    /// assignment — <c>Zax.App.Input.InputRouting</c> and the hotkeys work on masks alone.
    /// </summary>
    [Fact]
    public void REQ_INP_121_the_device_conversion_lives_in_the_host_layer()
    {
        // The two halves are distinct types: the host layer is DesktopInputSource, the logic is
        // InputRouting plus the trackers, and the logic never mentions the host layer.
        Type routing = typeof(InputRouting);
        foreach (var method in routing.GetMethods())
        {
            foreach (var parameter in method.GetParameters())
            {
                Assert.NotEqual(typeof(DesktopInputSource), parameter.ParameterType);
                Assert.NotEqual(typeof(InputBindings), parameter.ParameterType);
                Assert.NotEqual(typeof(GamepadAssignment), parameter.ParameterType);
            }
        }

        // The threshold and the dead zones live in the platform layer, not in the logic.
        Assert.Equal("Zax.Platform.Input", typeof(AnalogCalibration).Namespace);
        Assert.Equal("Zax.App.Input", routing.Namespace);
    }

    /// <summary>
    /// REQ-INP-122 and §4.4: the default libretro correspondence, with A on east and B on south,
    /// the right stick on the four rotations, L2 and R2 on the zoom, and no system hotkey.
    /// </summary>
    [Fact]
    public void REQ_INP_122_the_libretro_table_is_the_one_of_paragraph_4_4()
    {
        Assert.Equal(InputAction.Left, LibretroInputMap.ActionOf(RetroPadButton.Left));
        Assert.Equal(InputAction.Right, LibretroInputMap.ActionOf(RetroPadButton.Right));
        Assert.Equal(InputAction.Up, LibretroInputMap.ActionOf(RetroPadButton.Up));
        Assert.Equal(InputAction.Down, LibretroInputMap.ActionOf(RetroPadButton.Down));
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

        // A = east and B = south, deliberately different from the desktop layout of §4.3.
        Assert.Equal(8, (int)RetroPadButton.A);
        Assert.Equal(0, (int)RetroPadButton.B);

        foreach ((RetroPadButton _, InputAction action) in LibretroInputMap.Buttons)
        {
            Assert.False(InputAction.HotkeyMask.HasFlag(action), $"{action} must stay with the frontend");
        }

        Assert.Equal(InputAction.HotkeyMask, LibretroInputMap.HostReservedActions);
    }
}

/// <summary>A throw-away user root for the persistence tests of SPEC-08 and SPEC-09.</summary>
public sealed class TemporaryRoot : IDisposable
{
    /// <summary>Creates the root and its folders.</summary>
    public TemporaryRoot()
    {
        Root = Path.Combine(Path.GetTempPath(), $"zax-verify-c-{Guid.NewGuid():N}");
        Directory.CreateDirectory(Root);
        Paths = UserDataPaths.Resolve(Root);
    }

    /// <summary>The folder.</summary>
    public string Root { get; }

    /// <summary>The layout of REQ-PERS-072.</summary>
    public UserDataPaths Paths { get; }

    /// <inheritdoc/>
    public void Dispose()
    {
        try
        {
            Directory.Delete(Root, recursive: true);
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            // A locked temporary folder must not fail a verification run.
        }
    }
}
