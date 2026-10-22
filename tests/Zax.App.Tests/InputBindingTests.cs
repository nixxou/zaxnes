using Zax.App.Input;
using Zax.App.Persistence;
using Zax.Platform.Input;
using Silk.NET.Input;

namespace Zax.App.Tests;

/// <summary>
/// The desktop host layer of SPEC-08 §3.2 to §3.6: the default tables, the dead zones and the
/// 0.34 threshold, the gamepad assignment, the remapping rules and the persistence of the
/// bindings (REQ-INP-121).
/// </summary>
public sealed class InputBindingTests
{
    /// <summary>
    /// REQ-INP-011 and §4.2: the default keyboard of player 1 is the table of §4.2 and the one
    /// of player 2 is empty.
    /// </summary>
    [Fact]
    public void REQ_INP_011_the_default_keyboards_are_the_table_and_an_empty_set()
    {
        InputBindings bindings = new();
        BindingSet first = bindings.Keyboard(0);

        Assert.Equal(PhysicalControl.Key((int)Key.Left), first.Get(InputAction.Left));
        Assert.Equal(PhysicalControl.Key((int)Key.Right), first.Get(InputAction.Right));
        Assert.Equal(PhysicalControl.Key((int)Key.Up), first.Get(InputAction.Up));
        Assert.Equal(PhysicalControl.Key((int)Key.Down), first.Get(InputAction.Down));
        Assert.Equal(PhysicalControl.Key((int)Key.X), first.Get(InputAction.A));
        Assert.Equal(PhysicalControl.Key((int)Key.Z), first.Get(InputAction.B));
        Assert.Equal(PhysicalControl.Key((int)Key.Space), first.Get(InputAction.Select));
        Assert.Equal(PhysicalControl.Key((int)Key.Enter), first.Get(InputAction.Start));
        Assert.Equal(PhysicalControl.Key((int)Key.F5), first.Get(InputAction.RotLeft));
        Assert.Equal(PhysicalControl.Key((int)Key.F6), first.Get(InputAction.RotRight));
        Assert.Equal(PhysicalControl.Key((int)Key.F7), first.Get(InputAction.RotUp));
        Assert.Equal(PhysicalControl.Key((int)Key.F8), first.Get(InputAction.RotDown));
        Assert.Equal(PhysicalControl.Key((int)Key.F9), first.Get(InputAction.ZoomIn));
        Assert.Equal(PhysicalControl.Key((int)Key.F10), first.Get(InputAction.ZoomOut));
        Assert.Equal(PhysicalControl.Key((int)Key.Escape), first.Get(InputAction.Menu));
        Assert.Equal(PhysicalControl.Key((int)Key.Comma), first.Get(InputAction.RollBack));
        Assert.Equal(PhysicalControl.Key((int)Key.Period), first.Get(InputAction.FastForward));

        // SPEC-08 §4.2: QuickSave and QuickLoad have no default key.
        Assert.False(first.Get(InputAction.QuickSave).IsBound);
        Assert.False(first.Get(InputAction.QuickLoad).IsBound);

        Assert.Equal(0, bindings.Keyboard(1).Count);
    }

    /// <summary>
    /// REQ-INP-020 and §4.3: the default gamepad table, with the hat on the directions, A on the
    /// south button, B on the west one, RollBack on north, FastForward on east and the right
    /// stick on the rotations.
    /// </summary>
    [Fact]
    public void REQ_INP_020_the_default_gamepad_set_is_the_table_of_section_4_3()
    {
        BindingSet pad = DefaultBindings.CreateGamepad();

        Assert.Equal(PhysicalControl.Button(StandardGamepad.DPadLeft), pad.Get(InputAction.Left));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.DPadRight), pad.Get(InputAction.Right));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.DPadUp), pad.Get(InputAction.Up));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.DPadDown), pad.Get(InputAction.Down));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.South), pad.Get(InputAction.A));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.West), pad.Get(InputAction.B));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.North), pad.Get(InputAction.RollBack));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.East), pad.Get(InputAction.FastForward));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.Back), pad.Get(InputAction.Select));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.Start), pad.Get(InputAction.Start));
        Assert.Equal(PhysicalControl.Button(StandardGamepad.RightStick), pad.Get(InputAction.Menu));
        Assert.Equal(
            PhysicalControl.AxisNegative(StandardGamepad.RightStickX),
            pad.Get(InputAction.RotLeft));
        Assert.Equal(
            PhysicalControl.AxisPositive(StandardGamepad.RightStickX),
            pad.Get(InputAction.RotRight));
        Assert.Equal(
            PhysicalControl.AxisPositive(StandardGamepad.RightStickY),
            pad.Get(InputAction.RotUp));
        Assert.Equal(
            PhysicalControl.AxisNegative(StandardGamepad.RightStickY),
            pad.Get(InputAction.RotDown));
        Assert.Equal(PhysicalControl.Trigger(StandardGamepad.LeftTrigger), pad.Get(InputAction.ZoomIn));
        Assert.Equal(PhysicalControl.Trigger(StandardGamepad.RightTrigger), pad.Get(InputAction.ZoomOut));

        // REQ-INP-032 and §4.3: the left stick, both clicks but R3, the bumpers and Guide are
        // unbound, and QuickSave / QuickLoad have no default binding either.
        foreach (PhysicalControl control in (PhysicalControl[])
            [
                PhysicalControl.AxisPositive(StandardGamepad.LeftStickX),
                PhysicalControl.AxisNegative(StandardGamepad.LeftStickX),
                PhysicalControl.AxisPositive(StandardGamepad.LeftStickY),
                PhysicalControl.AxisNegative(StandardGamepad.LeftStickY),
            ])
        {
            Assert.DoesNotContain(control, pad.Bindings.Values);
        }

        Assert.DoesNotContain(PhysicalControl.Button(StandardGamepad.LeftStick), pad.Bindings.Values);
        Assert.DoesNotContain(PhysicalControl.Button(StandardGamepad.LeftBumper), pad.Bindings.Values);
        Assert.DoesNotContain(PhysicalControl.Button(StandardGamepad.RightBumper), pad.Bindings.Values);
        Assert.DoesNotContain(PhysicalControl.Button(StandardGamepad.Guide), pad.Bindings.Values);
        Assert.False(pad.Get(InputAction.QuickSave).IsBound);
        Assert.False(pad.Get(InputAction.QuickLoad).IsBound);
    }

    /// <summary>
    /// REQ-INP-021: a pad with no standard mapping starts with no useful default and works once
    /// its raw controls are remapped.
    /// </summary>
    [Fact]
    public void REQ_INP_021_an_unrecognised_pad_has_no_default_binding()
    {
        FakeGamepad generic = new("Generic USB Joystick", standard: false);
        FakeGamepadHub hub = new FakeGamepadHub().Plug(generic);
        InputBindings bindings = new();
        GamepadAssignment pads = new(hub, bindings);

        pads.Assign(0, generic);
        BindingSet set = pads.GamepadBindings(0)!;
        Assert.Equal(0, set.Count);

        set.Set(InputAction.A, PhysicalControl.Button(7));
        generic.Press(7);
        Assert.Equal(
            InputAction.A,
            set.Sample(control => AnalogCalibration.IsPressed(generic.Value(control))));
    }

    /// <summary>
    /// REQ-INP-030 and REQ-INP-031, as amended by AMD-021 R2-011 (which replaces AMD-020 for
    /// <c>Q-048</c>): the 0.34 threshold is measured on the <b>shaped</b> value — radial dead
    /// zone 0.2 and saturation 0.9 for a stick, 0.2 and 0.9 for a trigger.
    /// </summary>
    [Fact]
    public void REQ_INP_030_the_digital_threshold_is_zero_point_three_four()
    {
        FakeGamepad pad = new("Pad");
        PhysicalControl right = PhysicalControl.AxisPositive(StandardGamepad.RightStickX);
        PhysicalControl left = PhysicalControl.AxisNegative(StandardGamepad.RightStickX);

        // REQ-INP-031: a resting stick drifting by 0.15 produces nothing.
        pad.Axis(StandardGamepad.RightStickX, 0.15f);
        Assert.Equal(0f, pad.Value(right));

        // §5.1: 20 % to the right is released, 60 % is pressed.
        pad.Axis(StandardGamepad.RightStickX, 0.2f);
        Assert.False(AnalogCalibration.IsPressed(pad.Value(right)));

        pad.Axis(StandardGamepad.RightStickX, 0.6f);
        Assert.True(AnalogCalibration.IsPressed(pad.Value(right)));

        // R2-011: 0.40 of travel shapes to 0.286, below the threshold; 0.45 shapes to 0.357,
        // above it. The 0.30 / 0.40 pair of the original criterion is superseded.
        pad.Axis(StandardGamepad.RightStickX, -0.40f);
        Assert.Equal(0.2857f, pad.Value(left), 3);
        Assert.False(AnalogCalibration.IsPressed(pad.Value(left)));

        pad.Axis(StandardGamepad.RightStickX, -0.45f);
        Assert.Equal(0.3571f, pad.Value(left), 3);
        Assert.True(AnalogCalibration.IsPressed(pad.Value(left)));

        // REQ-INP-031: the shaped value of a stick at 0.9 is exactly 1, and that is now the
        // value the press decision reads (R2-011 closes Q-056).
        pad.Axis(StandardGamepad.RightStickX, 0.9f);
        Assert.Equal(1f, pad.Value(right), 3);

        // REQ-INP-030: a half-pressed trigger is pressed — 0.5 shapes to 0.429.
        pad.Trigger(StandardGamepad.LeftTrigger, 0.5f);
        PhysicalControl trigger = PhysicalControl.Trigger(StandardGamepad.LeftTrigger);
        Assert.Equal(0.4286f, pad.Value(trigger), 3);
        Assert.True(AnalogCalibration.IsPressed(pad.Value(trigger)));

        // R2-011: the trigger dead zone is 0.2, not the 0.1 of REQ-INP-031, and it saturates
        // at 0.9.
        pad.Trigger(StandardGamepad.LeftTrigger, 0.15f);
        Assert.Equal(0f, pad.Value(trigger));

        pad.Trigger(StandardGamepad.LeftTrigger, 0.9f);
        Assert.Equal(1f, pad.Value(trigger), 4);
    }

    [Fact] // AMD-021 R2-011 criterion
    public void AMD_021_R2_011_the_threshold_applies_to_the_shaped_value()
    {
        FakeGamepad pad = new("Pad");
        PhysicalControl up = PhysicalControl.AxisNegative(StandardGamepad.RightStickY);
        PhysicalControl rightHalf = PhysicalControl.AxisPositive(StandardGamepad.RightStickX);

        // Straight up at 0.40 of travel: the shaped value is (0.40 - 0.2) / 0.7 = 0.286, no
        // press.
        pad.Axis(StandardGamepad.RightStickY, -0.40f).Axis(StandardGamepad.RightStickX, 0f);
        Assert.Equal(0.2857f, pad.Value(up), 3);
        Assert.False(AnalogCalibration.IsPressed(pad.Value(up)));

        // At 0.45 the shaped value is 0.357: pressed.
        pad.Axis(StandardGamepad.RightStickY, -0.45f);
        Assert.Equal(0.3571f, pad.Value(up), 3);
        Assert.True(AnalogCalibration.IsPressed(pad.Value(up)));

        // At 45 degrees with a travel of 0.6: the shaped magnitude is 0.571 and each component
        // 0.404, so Up and Right are both pressed.
        float diagonal = 0.6f / MathF.Sqrt(2f);
        pad.Axis(StandardGamepad.RightStickY, -diagonal).Axis(StandardGamepad.RightStickX, diagonal);

        Assert.Equal(0.404f, pad.Value(up), 3);
        Assert.Equal(0.404f, pad.Value(rightHalf), 3);
        Assert.True(AnalogCalibration.IsPressed(pad.Value(up)));
        Assert.True(AnalogCalibration.IsPressed(pad.Value(rightHalf)));
    }

    [Fact] // AMD-021 R2-011 (the shaping constants)
    public void AMD_021_R2_011_the_trigger_dead_zone_is_zero_point_two()
    {
        Assert.Equal(0.34f, AnalogCalibration.PressThreshold);
        Assert.Equal(0.2f, AnalogCalibration.StickDeadZone);
        Assert.Equal(0.9f, AnalogCalibration.StickSaturation);
        Assert.Equal(0.2f, AnalogCalibration.TriggerDeadZone);
        Assert.Equal(0.9f, AnalogCalibration.TriggerSaturation);

        // The press decision and an analog consumer read the same number (Q-056 is moot).
        Assert.Equal(AnalogCalibration.ShapedTrigger(0.5f), AnalogCalibration.Trigger(0.5f), 6);
        Assert.Equal(
            AnalogCalibration.ShapedHalfAxis(0.5f, 0f, 1),
            AnalogCalibration.HalfAxis(0.5f, 0f, 1),
            6);
    }

    /// <summary>
    /// REQ-INP-031: the stick dead zone is radial, so a diagonal push past the zone produces a
    /// value on both halves.
    /// </summary>
    [Fact]
    public void REQ_INP_031_the_stick_dead_zone_is_radial()
    {
        FakeGamepad pad = new("Pad");

        // Each axis alone is inside the dead zone, but the magnitude (0.212) is just past it.
        pad.Axis(StandardGamepad.RightStickX, 0.15f).Axis(StandardGamepad.RightStickY, 0.15f);
        Assert.True(pad.Value(PhysicalControl.AxisPositive(StandardGamepad.RightStickX)) > 0f);

        // Both at 0.19 keeps the magnitude at 0.269, still past the zone; at 0.1 each the
        // magnitude is 0.141 and nothing is produced.
        pad.Axis(StandardGamepad.RightStickX, 0.1f).Axis(StandardGamepad.RightStickY, 0.1f);
        Assert.Equal(0f, pad.Value(PhysicalControl.AxisPositive(StandardGamepad.RightStickX)));
    }

    /// <summary>
    /// REQ-INP-032: with the default gamepad set, pushing the left stick fully produces neither
    /// a NES direction nor a camera action.
    /// </summary>
    [Fact]
    public void REQ_INP_032_the_left_stick_is_unbound_by_default()
    {
        FakeGamepad pad = new("Pad");
        pad.Axis(StandardGamepad.LeftStickX, 1f).Axis(StandardGamepad.LeftStickY, 1f);

        BindingSet set = DefaultBindings.CreateGamepad();
        Assert.Equal(
            InputAction.None,
            set.Sample(control => AnalogCalibration.IsPressed(pad.Value(control))));
    }

    /// <summary>
    /// REQ-INP-010: the same key may drive an action of J1 and another of J2; binding W to Up
    /// for J2 leaves J1 untouched.
    /// </summary>
    [Fact]
    public void REQ_INP_010_a_key_can_drive_both_players()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        DesktopInputSource source = Source(bindings, new FakeGamepadHub(), keyboard);

        bindings.Keyboard(1).Set(InputAction.Up, PhysicalControl.Key((int)Key.W));
        keyboard.Press(Key.W);

        InputSnapshot snapshot = source.Sample(0.016f);
        Assert.Equal(InputAction.None, snapshot.PlayerOne);
        Assert.Equal(InputAction.Up, snapshot.PlayerTwo);

        // The right arrow is J1's: both players are then driven by one keyboard.
        keyboard.Press(Key.Right);
        snapshot = source.Sample(0.016f);
        Assert.Equal(InputAction.Right, snapshot.PlayerOne);
        Assert.Equal(InputAction.Up, snapshot.PlayerTwo);
    }

    /// <summary>
    /// REQ-INP-002 and REQ-INP-044: a key bound to two actions raises both bits, and the
    /// keyboard and the pad are OR-merged.
    /// </summary>
    [Fact]
    public void REQ_INP_044_one_control_can_drive_several_actions()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = Source(bindings, hub, keyboard);

        source.Pads.SkipStartupDelay();
        source.Pads.Assign(0, pad);

        // Bind X to both A and B: pressing it gives 0x03 (REQ-INP-044).
        bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Key((int)Key.X));
        bindings.Keyboard(0).Set(InputAction.B, PhysicalControl.Key((int)Key.X));
        keyboard.Press(Key.X);

        Assert.Equal(InputAction.A | InputAction.B, source.Sample(0.016f).PlayerOne);

        // REQ-INP-002: the pad's hat adds its bit; releasing the key leaves it set.
        pad.Press(StandardGamepad.DPadRight);
        Assert.Equal(
            InputAction.A | InputAction.B | InputAction.Right,
            source.Sample(0.016f).PlayerOne);

        keyboard.Release(Key.X);
        Assert.Equal(InputAction.Right, source.Sample(0.016f).PlayerOne);
    }

    /// <summary>
    /// REQ-INP-023 rule 2: the first unassigned pad to become active goes to J1, the next one to
    /// J2 — press M2 then M1 and M2 drives J1.
    /// </summary>
    [Fact]
    public void REQ_INP_023_the_first_pad_used_goes_to_player_one()
    {
        FakeGamepad m1 = new("M1");
        FakeGamepad m2 = new("M2");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(m1).Plug(m2);
        InputBindings bindings = new();
        GamepadAssignment pads = new(hub, bindings);

        pads.SkipStartupDelay();
        Assert.Null(pads.Device(0));
        Assert.Null(pads.Device(1));

        m2.Press(StandardGamepad.South);
        pads.Update(0.016f);
        Assert.Equal("M2", pads.Device(0)?.Identity);
        Assert.Null(pads.Device(1));

        m2.ReleaseAll();
        m1.Press(StandardGamepad.South);
        pads.Update(0.016f);
        Assert.Equal("M2", pads.Device(0)?.Identity);
        Assert.Equal("M1", pads.Device(1)?.Identity);

        // REQ-INP-050: the assignment is remembered, so the same pads come back next launch.
        Assert.Equal("M2", bindings.LastGamepad(0));
        Assert.Equal("M1", bindings.LastGamepad(1));

        InputBindings reloaded = new();
        reloaded.SetLastGamepad(0, "M2");
        reloaded.SetLastGamepad(1, "M1");
        GamepadAssignment next = new(hub, reloaded);
        next.Update(GamepadAssignment.StartupDelaySeconds);
        Assert.Equal("M2", next.Device(0)?.Identity);
        Assert.Equal("M1", next.Device(1)?.Identity);
    }

    /// <summary>
    /// REQ-INP-023 rule 1: the reconnection waits about 0.5 s after launch before taking the
    /// remembered pads back.
    /// </summary>
    [Fact]
    public void REQ_INP_023_the_remembered_pads_come_back_after_half_a_second()
    {
        FakeGamepad pad = new("M1");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        InputBindings bindings = new();
        bindings.SetLastGamepad(0, "M1");

        GamepadAssignment pads = new(hub, bindings);

        pads.Update(0.2f);
        Assert.False(pads.StartupComplete);
        Assert.Null(pads.Device(0));

        pads.Update(0.4f);
        Assert.True(pads.StartupComplete);
        Assert.Equal("M1", pads.Device(0)?.Identity);
    }

    /// <summary>
    /// REQ-INP-024: unplugging an assigned pad leaves the player with none and its bits fall to
    /// zero at the next sample; plugging it back and pressing a button gives it back.
    /// </summary>
    [Fact]
    public void REQ_INP_024_unplugging_drops_the_pad_bits_at_the_next_sample()
    {
        FakeGamepad pad = new("M1");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        InputBindings bindings = new();
        DesktopInputSource source = Source(bindings, hub, keyboard: null);

        source.Pads.SkipStartupDelay();
        pad.Press(StandardGamepad.DPadRight);
        Assert.Equal(InputAction.Right, source.Sample(0.016f).PlayerOne);

        hub.Unplug("M1");
        Assert.Equal(InputAction.None, source.Sample(0.016f).PlayerOne);
        Assert.Null(source.Pads.Device(0));

        // Plugged back in: not assigned until it is used again.
        hub.Plug(pad);
        pad.ReleaseAll();
        Assert.Equal(InputAction.None, source.Sample(0.016f).PlayerOne);

        pad.Press(StandardGamepad.DPadRight);
        Assert.Equal(InputAction.Right, source.Sample(0.016f).PlayerOne);
    }

    /// <summary>
    /// REQ-INP-022: a VR controller never appears in the device list and is never assigned
    /// automatically.
    /// </summary>
    [Theory]
    [InlineData("Oculus Touch Controller")]
    [InlineData("HTC Vive Controller")]
    [InlineData("Valve Index Knuckles")]
    [InlineData("Spatial Controller")]
    public void REQ_INP_022_vr_devices_are_never_offered(string identity)
    {
        Assert.True(GamepadIdentity.IsVirtualReality(identity));

        FakeGamepadHub hub = new FakeGamepadHub().Plug(new FakeGamepad(identity));
        Assert.Empty(hub.Devices);

        GamepadAssignment pads = new(hub, new InputBindings());
        pads.Update(1f);
        Assert.Null(pads.Device(0));
        Assert.Empty(pads.Available);
    }

    /// <summary>
    /// DEC-08-1 (divergence D) and REQ-INP-041: assigning a pad to a player removes it from the
    /// other one.
    /// </summary>
    [Fact]
    public void DEC_08_1_a_pad_assigned_to_one_player_is_removed_from_the_other()
    {
        FakeGamepad pad = new("M1");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        InputBindings bindings = new();
        GamepadAssignment pads = new(hub, bindings);

        pads.SkipStartupDelay();
        pads.Assign(0, pad);
        Assert.Equal("M1", pads.Device(0)?.Identity);

        pads.Assign(1, pad);
        Assert.Null(pads.Device(0));
        Assert.Equal("M1", pads.Device(1)?.Identity);
        Assert.Null(bindings.LastGamepad(0));

        // "none" takes the pad away (REQ-INP-041).
        pads.Assign(1, null);
        Assert.Null(pads.Device(1));
    }

    /// <summary>
    /// REQ-INP-025: a gamepad set belongs to the device identity, survives an unplug and follows
    /// the pad from one player to the other; another model keeps its own set.
    /// </summary>
    [Fact]
    public void REQ_INP_025_gamepad_bindings_belong_to_the_device_identity()
    {
        FakeGamepad modelX = new("Model X");
        FakeGamepad modelY = new("Model Y");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(modelX).Plug(modelY);
        InputBindings bindings = new();
        GamepadAssignment pads = new(hub, bindings);

        pads.SkipStartupDelay();
        pads.Assign(0, modelX);

        // Remap the north button of Model X onto QuickSave.
        pads.GamepadBindings(0)!.Set(
            InputAction.QuickSave,
            PhysicalControl.Button(StandardGamepad.North));

        // Unplug and plug back: the remapping is still there.
        hub.Unplug("Model X");
        hub.Plug(modelX);
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            bindings.Gamepad("Model X").Get(InputAction.QuickSave));

        // Model Y keeps the §4.3 defaults.
        pads.Assign(1, modelY);
        Assert.False(pads.GamepadBindings(1)!.Get(InputAction.QuickSave).IsBound);
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            pads.GamepadBindings(1)!.Get(InputAction.RollBack));

        // The set follows the pad from J1 to J2.
        pads.Assign(1, modelX);
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            pads.GamepadBindings(1)!.Get(InputAction.QuickSave));
    }

    /// <summary>
    /// REQ-INP-042 and REQ-INP-043: activating a field clears the binding and starts listening,
    /// the first newly pressed eligible key becomes the binding, and a second capture replaces
    /// the first.
    /// </summary>
    [Fact]
    public void REQ_INP_042_a_capture_clears_then_takes_the_first_eligible_key()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        RebindingSession session = new(bindings);

        Assert.True(session.Begin(InputAction.B));
        Assert.True(session.IsListening);

        // REQ-INP-042: the old binding is already gone while the listening is on.
        Assert.False(bindings.Keyboard(0).Get(InputAction.B).IsBound);

        // Q-033: the first sample only records what is already held.
        session.Capture(keyboard, null);

        keyboard.Press(Key.C);
        Assert.Equal(PhysicalControl.Key((int)Key.C), session.Capture(keyboard, null));
        Assert.False(session.IsListening);
        Assert.Equal(PhysicalControl.Key((int)Key.C), bindings.Keyboard(0).Get(InputAction.B));

        // REQ-INP-043: one binding per target per device kind; the second capture replaces it.
        keyboard.Release(Key.C);
        session.Begin(InputAction.B);
        session.Capture(keyboard, null);
        keyboard.Press(Key.K);
        session.Capture(keyboard, null);

        Assert.Equal(PhysicalControl.Key((int)Key.K), bindings.Keyboard(0).Get(InputAction.B));
        Assert.Equal(1, bindings.Keyboard(0).Bindings.Count(
            entry => entry.Key == BindingTarget.Of(InputAction.B)));
    }

    /// <summary>
    /// REQ-INP-042 and REQ-INP-012: a keyboard capture only accepts keys, a gamepad capture only
    /// pad controls, and a mouse button can never be bound — the vocabulary has no such control.
    /// </summary>
    [Fact]
    public void REQ_INP_042_only_the_right_device_kind_is_eligible()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        GamepadAssignment pads = new(hub, bindings);
        pads.SkipStartupDelay();
        pads.Assign(0, pad);

        RebindingSession session = new(bindings, pads);

        // A keyboard capture ignores a pad button.
        session.Begin(InputAction.A);
        session.Capture(keyboard, pad);
        pad.Press(StandardGamepad.South);
        Assert.False(session.Capture(keyboard, pad).IsBound);
        Assert.True(session.IsListening);

        // A gamepad capture ignores a key and takes the stick half-axis.
        session.SelectDeviceKind(BindingDeviceKind.Gamepad);
        session.Begin(InputAction.A);
        session.Capture(keyboard, pad);

        keyboard.Press(Key.J);
        Assert.False(session.Capture(keyboard, pad).IsBound);

        pad.Axis(StandardGamepad.LeftStickY, 1f);
        Assert.Equal(
            PhysicalControl.AxisPositive(StandardGamepad.LeftStickY),
            session.Capture(keyboard, pad));
        Assert.Equal(
            PhysicalControl.AxisPositive(StandardGamepad.LeftStickY),
            pads.GamepadBindings(0)!.Get(InputAction.A));

        // REQ-INP-012: a keyboard set refuses anything that is not a key.
        Assert.False(bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Button(0)));
        Assert.False(
            bindings.Gamepad("Pad").Set(InputAction.A, PhysicalControl.Key((int)Key.A)));
    }

    /// <summary>
    /// REQ-INP-045: stopping the listening, changing player, changing device kind or a pad change
    /// all leave the action empty, because the field was cleared when it was activated.
    /// </summary>
    [Fact]
    public void REQ_INP_045_stopping_the_listening_leaves_the_action_empty()
    {
        InputBindings bindings = new();
        RebindingSession session = new(bindings);

        session.Begin(InputAction.Start);
        session.Stop();
        Assert.False(session.IsListening);
        Assert.False(bindings.Keyboard(0).Get(InputAction.Start).IsBound);

        session.Begin(InputAction.A);
        session.SelectPlayer(1);
        Assert.False(session.IsListening);
        Assert.False(bindings.Keyboard(0).Get(InputAction.A).IsBound);

        session.SelectPlayer(0);
        session.Begin(InputAction.Up);
        session.SelectDeviceKind(BindingDeviceKind.Gamepad);
        Assert.False(session.IsListening);
        Assert.False(bindings.Keyboard(0).Get(InputAction.Up).IsBound);

        session.SelectDeviceKind(BindingDeviceKind.Keyboard);
        session.Begin(InputAction.Down);
        session.NotifyGamepadChanged();
        Assert.False(session.IsListening);

        // Activating another field stops the first one.
        session.Begin(InputAction.Left);
        session.Begin(InputAction.Right);
        Assert.Equal(BindingTarget.Of(InputAction.Right), session.Target);
        Assert.False(bindings.Keyboard(0).Get(InputAction.Left).IsBound);

        session.Close();
        Assert.False(session.IsListening);
    }

    /// <summary>
    /// REQ-INP-046: in Gamepad mode with no pad assigned, there is nothing to show and
    /// activating a field starts no capture.
    /// </summary>
    [Fact]
    public void REQ_INP_046_gamepad_mode_without_a_pad_shows_nothing()
    {
        InputBindings bindings = new();
        GamepadAssignment pads = new(new FakeGamepadHub(), bindings);
        RebindingSession session = new(bindings, pads);

        session.SelectPlayer(1);
        session.SelectDeviceKind(BindingDeviceKind.Gamepad);

        Assert.Null(session.ActiveSet);
        Assert.False(session.Begin(InputAction.A));
        Assert.False(session.IsListening);
    }

    /// <summary>
    /// DEC-08-2 and REQ-INP-047: Escape can be captured like any other key, and while a capture
    /// is listening — and in the sample it completes in — the host produces no action at all, so
    /// the same press cannot also be seen as <c>Menu</c>.
    /// </summary>
    [Fact]
    public void DEC_08_2_capturing_escape_does_not_also_open_the_menu()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        DesktopInputSource source = Source(bindings, new FakeGamepadHub(), keyboard);

        source.Rebinding.Begin(InputAction.B);
        Assert.True(source.Rebinding.IsListening);

        // The sample that starts the listening produces nothing.
        Assert.Equal(InputAction.None, source.Sample(0.016f).PlayerOne);
        source.EndSample();

        keyboard.Press(Key.Escape);
        InputSnapshot captured = source.Sample(0.016f);

        // Escape became the binding of B, and the mask stays empty: no Menu this sample.
        Assert.Equal(PhysicalControl.Key((int)Key.Escape), bindings.Keyboard(0).Get(InputAction.B));
        Assert.Equal(InputAction.None, captured.PlayerOne);
        Assert.True(source.Rebinding.SuppressesActions);

        source.EndSample();

        // From the next sample on, Escape is B — and it is no longer Menu, since the capture
        // replaced nothing else: Menu kept its own Escape binding until it was overwritten.
        Assert.True(source.Sample(0.016f).PlayerOne.HasFlag(InputAction.B));
    }

    /// <summary>
    /// REQ-INP-049: resetting a set puts the table of §4.2 back for J1, an empty set for J2 and
    /// the table of §4.3 for a pad.
    /// </summary>
    [Fact]
    public void REQ_INP_049_a_set_can_be_reset_to_its_defaults()
    {
        InputBindings bindings = new();

        bindings.Keyboard(0).ClearAll();
        bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Key((int)Key.Q));
        bindings.ResetKeyboard(0);
        Assert.Equal(PhysicalControl.Key((int)Key.X), bindings.Keyboard(0).Get(InputAction.A));
        Assert.Equal(17, bindings.Keyboard(0).Count);

        bindings.Keyboard(1).Set(InputAction.A, PhysicalControl.Key((int)Key.Q));
        bindings.ResetKeyboard(1);
        Assert.Equal(0, bindings.Keyboard(1).Count);

        BindingSet pad = bindings.Gamepad("Pad");
        pad.Set(InputAction.A, PhysicalControl.Button(StandardGamepad.North));
        bindings.ResetGamepad("Pad");
        Assert.Equal(PhysicalControl.Button(StandardGamepad.South), pad.Get(InputAction.A));
    }

    /// <summary>
    /// AMD-010 REQ-UI-203: a menu command is bindable like an action and runs when its control is
    /// pressed, once per press.
    /// </summary>
    [Fact]
    public void REQ_UI_203_a_menu_command_can_be_bound_to_a_pad_button()
    {
        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = Source(bindings, hub, keyboard: null);

        source.Pads.SkipStartupDelay();
        source.Pads.Assign(0, pad);

        // A command is a target like any other: in Keyboard mode the field edits the keyboard
        // set of the shown player.
        Assert.True(source.Rebinding.BeginCommand(Zax.App.Loop.ZaxCommands.Reset));
        Assert.Same(bindings.Keyboard(0), source.Rebinding.ActiveSet);

        source.Rebinding.SelectDeviceKind(BindingDeviceKind.Gamepad);
        Assert.True(source.Rebinding.BeginCommand(Zax.App.Loop.ZaxCommands.Reset));
        source.Rebinding.Capture(null, pad);

        pad.Press(StandardGamepad.LeftBumper);
        Assert.Equal(PhysicalControl.Button(StandardGamepad.LeftBumper), source.Rebinding.Capture(null, pad));
        source.Rebinding.EndSample();

        // The command fires once on the rising edge, not on every frame it is held.
        pad.ReleaseAll();
        source.Sample(0.016f);
        source.EndSample();

        pad.Press(StandardGamepad.LeftBumper);
        source.Sample(0.016f);
        Assert.Equal([Zax.App.Loop.ZaxCommands.Reset], source.TriggeredCommands);

        source.EndSample();
        source.Sample(0.016f);
        Assert.Empty(source.TriggeredCommands);
    }

    /// <summary>
    /// REQ-INP-050, REQ-INP-052 and SPEC-09 REQ-PERS-065: the keyboard sets, the per-identity
    /// gamepad sets and the last pad of each player survive a round trip through
    /// <c>input.json</c>, and an unreadable file falls back to the defaults.
    /// </summary>
    [Fact]
    public void REQ_PERS_065_the_bindings_survive_a_round_trip_through_input_json()
    {
        using TemporaryRoot root = new();
        UserDataPaths paths = UserDataPaths.Resolve(root.Path);
        InputBindingStore store = new(paths);

        InputBindings written = new();
        written.Keyboard(0).Set(InputAction.A, PhysicalControl.Key((int)Key.K));
        written.Keyboard(1).Set(InputAction.Up, PhysicalControl.Key((int)Key.W));
        written.Gamepad("Model X").Set(
            InputAction.QuickSave,
            PhysicalControl.Button(StandardGamepad.North));
        written.Gamepad("Model X").Set(
            InputAction.RotLeft,
            PhysicalControl.AxisNegative(StandardGamepad.LeftStickX));
        written.Gamepad("Model X").Set(
            BindingTarget.OfCommand(Zax.App.Loop.ZaxCommands.Reset),
            PhysicalControl.Trigger(StandardGamepad.RightTrigger));
        written.SetLastGamepad(1, "Model X");
        store.Save(written);

        Assert.True(File.Exists(paths.InputFile));

        InputBindings read = new();
        new InputBindingStore(paths).Load(read);

        Assert.Equal(PhysicalControl.Key((int)Key.K), read.Keyboard(0).Get(InputAction.A));
        Assert.Equal(PhysicalControl.Key((int)Key.W), read.Keyboard(1).Get(InputAction.Up));
        Assert.Equal(
            PhysicalControl.Button(StandardGamepad.North),
            read.Gamepad("Model X").Get(InputAction.QuickSave));
        Assert.Equal(
            PhysicalControl.AxisNegative(StandardGamepad.LeftStickX),
            read.Gamepad("Model X").Get(InputAction.RotLeft));
        Assert.Equal(
            PhysicalControl.Trigger(StandardGamepad.RightTrigger),
            read.Gamepad("Model X").Get(BindingTarget.OfCommand(Zax.App.Loop.ZaxCommands.Reset)));
        Assert.Equal("Model X", read.LastGamepad(1));
        Assert.Null(read.LastGamepad(0));

        // REQ-INP-052: a corrupted file starts on the defaults, without a blocking error, and is
        // set aside.
        File.WriteAllText(paths.InputFile, "{ not json");
        InputBindings recovered = new();
        InputBindingStore recovery = new(paths);
        recovery.Load(recovered);

        Assert.Equal(InputBindingStore.UnreadableMessage, recovery.Message);
        Assert.Equal(PhysicalControl.Key((int)Key.X), recovered.Keyboard(0).Get(InputAction.A));
        Assert.Equal(0, recovered.Keyboard(1).Count);
        Assert.NotEmpty(Directory.GetFiles(paths.Root, "input.json.corrupt-*"));
    }

    /// <summary>
    /// REQ-INP-051: a binding change is written at once, so nothing is lost on a hard kill.
    /// </summary>
    [Fact]
    public void REQ_INP_051_a_change_is_written_immediately()
    {
        using TemporaryRoot root = new();
        UserDataPaths paths = UserDataPaths.Resolve(root.Path);
        InputBindingStore store = new(paths);

        InputBindings bindings = new();
        store.Load(bindings);
        DesktopInputSource source = new(bindings, new FakeGamepadHub(), store);

        Assert.False(File.Exists(paths.InputFile));

        bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Key((int)Key.K));
        Assert.True(File.Exists(paths.InputFile));

        InputBindings reread = new();
        new InputBindingStore(paths).Load(reread);
        Assert.Equal(PhysicalControl.Key((int)Key.K), reread.Keyboard(0).Get(InputAction.A));

        source.Detach();
    }

    /// <summary>
    /// SPEC-09 REQ-PERS-065: the persisted form of a physical command is stable and round-trips,
    /// and a malformed entry degrades to "unbound".
    /// </summary>
    [Theory]
    [InlineData("key:88")]
    [InlineData("button:3")]
    [InlineData("axis:2+")]
    [InlineData("axis:3-")]
    [InlineData("trigger:1")]
    public void REQ_PERS_065_a_control_round_trips_through_its_text(string text)
    {
        PhysicalControl control = PhysicalControl.Parse(text);
        Assert.True(control.IsBound);
        Assert.Equal(text, control.ToString());
    }

    [Theory]
    [InlineData("")]
    [InlineData("axis:2")]
    [InlineData("mouse:0")]
    [InlineData("key:")]
    [InlineData("key:abc")]
    [InlineData("nonsense")]
    public void REQ_INP_052_a_malformed_control_is_unbound(string text)
        => Assert.False(PhysicalControl.Parse(text).IsBound);

    /// <summary>
    /// REQ-INP-048: the bindings are global; nothing in the model is keyed by game.
    /// </summary>
    [Fact]
    public void REQ_INP_048_bindings_are_global()
    {
        InputBindings bindings = new();
        bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Key((int)Key.K));

        // There is one keyboard set per player and one per pad identity; no per-game dimension
        // exists in the model, so a remapping made in one game is active in every other.
        Assert.Equal(PhysicalControl.Key((int)Key.K), bindings.Keyboard(0).Get(InputAction.A));
        Assert.Equal(2, InputActions.PlayerCount);
    }

    private static DesktopInputSource Source(
        InputBindings bindings,
        IGamepadHub hub,
        FakeKeyboard? keyboard)
        => new(bindings, hub)
        {
            Keyboard = keyboard,
            Pointer = new FakePointer(),
        };
}
