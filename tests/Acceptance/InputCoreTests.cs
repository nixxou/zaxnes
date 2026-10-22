using Silk.NET.Input;
using Zax.App.Input;
using Zax.Nes;
using Zax.Platform.Input;

namespace Zax.Acceptance.Tests;

/// <summary>
/// SPEC-08 §3.1, §3.2 and §3.4: the 19 actions and their bit values, the combination of the
/// sources, the byte the core receives, the keyboard defaults and the analog thresholds.
/// </summary>
public sealed class InputCoreTests
{
    /// <summary>REQ-INP-001: exactly two players and 19 actions each, and no J3.</summary>
    [Fact]
    public void REQ_INP_001_two_players_and_nineteen_actions()
    {
        Assert.Equal(2, InputActions.PlayerCount);
        Assert.Equal(19, InputActions.Count);
        Assert.Equal(19, InputActions.All.Count);
        Assert.Equal(19, InputActions.All.Distinct().Count());

        InputBindings bindings = new();
        Assert.NotNull(bindings.Keyboard(0));
        Assert.NotNull(bindings.Keyboard(1));
        Assert.Throws<ArgumentOutOfRangeException>(() => bindings.Keyboard(2));
    }

    /// <summary>
    /// REQ-INP-003 and §4.1: the bit values are an interoperability fact and must be exact.
    /// </summary>
    [Theory]
    [InlineData("Left", 0x40)]
    [InlineData("Right", 0x80)]
    [InlineData("Up", 0x10)]
    [InlineData("Down", 0x20)]
    [InlineData("A", 0x01)]
    [InlineData("B", 0x02)]
    [InlineData("Select", 0x04)]
    [InlineData("Start", 0x08)]
    [InlineData("RotLeft", 0x400)]
    [InlineData("RotRight", 0x800)]
    [InlineData("RotUp", 0x1000)]
    [InlineData("RotDown", 0x2000)]
    [InlineData("ZoomIn", 0x100)]
    [InlineData("ZoomOut", 0x200)]
    [InlineData("Menu", 0x4000)]
    [InlineData("RollBack", 0x8000)]
    [InlineData("FastForward", 0x10000)]
    [InlineData("QuickSave", 0x20000)]
    [InlineData("QuickLoad", 0x40000)]
    public void REQ_INP_003_the_bit_of_each_action_is_the_one_of_the_table(string name, int bit)
    {
        InputAction? action = InputActions.Parse(name);
        Assert.NotNull(action);
        Assert.Equal(bit, (int)action!.Value);
    }

    /// <summary>REQ-INP-040: the 19 actions in the display order of §4.1, with their names.</summary>
    [Fact]
    public void REQ_INP_040_the_actions_are_listed_in_the_order_of_the_table()
    {
        string[] expected =
        [
            "Left", "Right", "Up", "Down", "A", "B", "Select", "Start",
            "RotLeft", "RotRight", "RotUp", "RotDown", "ZoomIn", "ZoomOut",
            "Menu", "RollBack", "FastForward", "QuickSave", "QuickLoad",
        ];

        List<string> actual = [.. InputActions.All.Select(InputActions.NameOf)];
        Assert.Equal<IEnumerable<string>>(expected, actual);
    }

    /// <summary>
    /// REQ-INP-002: the mask of a player is the bitwise OR of its sources, and no source can
    /// take away a bit another one raised. The criterion's case: right arrow plus the pad's south
    /// button gives <c>0x81</c>.
    /// </summary>
    [Fact]
    public void REQ_INP_002_the_sources_are_combined_by_bitwise_or()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        FakeGamepad pad = new("Pad A");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = InputSupport.Source(bindings, hub, keyboard);

        // The pad becomes J1's as soon as it is used (REQ-INP-023 rule 2).
        pad.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);

        keyboard.Press(Key.Right);
        Assert.Equal((InputAction)0x81, InputSupport.Sample(source, 0));

        keyboard.ReleaseAll();
        Assert.Equal(InputAction.A, InputSupport.Sample(source, 0));

        pad.ReleaseAll();
        keyboard.Press(Key.Right);
        Assert.Equal(InputAction.Right, InputSupport.Sample(source, 0));
    }

    /// <summary>
    /// REQ-INP-004: the word the core receives — port 1 in the low byte, port 2 in the next —
    /// carries the eight low bits of a Controller port and the Zapper byte of a Zapper port,
    /// without any controller button.
    /// </summary>
    [Fact]
    public void REQ_INP_004_the_core_word_follows_the_port_types()
    {
        ushort both = InputRouting.CoreWord(InputAction.A, InputAction.B);
        Assert.Equal(0x01, both & 0xFF);
        Assert.Equal(0x02, (both >> 8) & 0xFF);

        ushort zapper = InputRouting.CoreWord(
            InputAction.A,
            InputAction.A | InputAction.Right,
            ControllerPortType.Controller,
            ControllerPortType.Zapper,
            ZapperState.ReleasedByte);

        Assert.Equal(0x01, zapper & 0xFF);
        Assert.Equal(ZapperState.ReleasedByte, (zapper >> 8) & 0xFF);
    }

    /// <summary>REQ-INP-006: the bits above 0x80 never reach the core.</summary>
    [Fact]
    public void REQ_INP_006_the_camera_and_system_bits_never_reach_the_core()
    {
        InputAction held = InputAction.RollBack | InputAction.RotLeft | InputAction.FastForward
            | InputAction.Menu | InputAction.QuickSave | InputAction.QuickLoad | InputAction.ZoomIn;

        Assert.Equal(0, InputActions.ControllerByte(held));
        Assert.Equal(0, InputRouting.CoreWord(held, held));

        Assert.Equal(0xFF, (int)InputAction.ControllerByte);
    }

    /// <summary>REQ-INP-007: no turbo action in the default list.</summary>
    [Fact]
    public void REQ_INP_007_there_is_no_turbo_action()
    {
        foreach (InputAction action in InputActions.All)
        {
            Assert.DoesNotContain("turbo", InputActions.NameOf(action), StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("repeat", InputActions.NameOf(action), StringComparison.OrdinalIgnoreCase);
        }

        Assert.Equal(19, InputActions.All.Count);
    }

    /// <summary>
    /// REQ-INP-010: the physical keyboard feeds the keyboard set of <b>each</b> player, so the
    /// same key may drive an action of J1 and another of J2.
    /// </summary>
    [Fact]
    public void REQ_INP_010_one_key_can_drive_both_players()
    {
        InputBindings bindings = new();
        bindings.Keyboard(1).Set(InputAction.Up, PhysicalControl.Key((int)Key.W));

        FakeKeyboard keyboard = new();
        DesktopInputSource source = InputSupport.Source(bindings, new FakeGamepadHub(), keyboard);

        keyboard.Press(Key.W);
        Zax.App.Input.InputSnapshot snapshot = source.Sample(1f / 60f);
        source.EndSample();

        Assert.Equal(InputAction.Up, snapshot.PlayerTwo);
        Assert.Equal(InputAction.None, snapshot.PlayerOne);
    }

    /// <summary>
    /// REQ-INP-011 and §4.2: the default keyboard set of J1 is the table, J2's is empty.
    /// </summary>
    [Fact]
    public void REQ_INP_011_the_default_keyboard_is_the_table_and_J2_is_empty()
    {
        InputBindings bindings = new();

        (InputAction Action, Key Key)[] table =
        [
            (InputAction.Left, Key.Left),
            (InputAction.Right, Key.Right),
            (InputAction.Up, Key.Up),
            (InputAction.Down, Key.Down),
            (InputAction.A, Key.X),
            (InputAction.B, Key.Z),
            (InputAction.Select, Key.Space),
            (InputAction.Start, Key.Enter),
            (InputAction.RotLeft, Key.F5),
            (InputAction.RotRight, Key.F6),
            (InputAction.RotUp, Key.F7),
            (InputAction.RotDown, Key.F8),
            (InputAction.ZoomIn, Key.F9),
            (InputAction.ZoomOut, Key.F10),
            (InputAction.Menu, Key.Escape),
            (InputAction.RollBack, Key.Comma),
            (InputAction.FastForward, Key.Period),
        ];

        foreach ((InputAction action, Key key) in table)
        {
            Assert.Equal(PhysicalControl.Key((int)key), bindings.Keyboard(0).Get(action));
        }

        Assert.Equal(PhysicalControl.None, bindings.Keyboard(0).Get(InputAction.QuickSave));
        Assert.Equal(PhysicalControl.None, bindings.Keyboard(0).Get(InputAction.QuickLoad));
        Assert.Equal(table.Length, bindings.Keyboard(0).Count);
        Assert.Equal(0, bindings.Keyboard(1).Count);
    }

    /// <summary>
    /// REQ-INP-012: modifier keys are bindable like ordinary keys; mouse buttons are not
    /// bindable at all — the vocabulary of a binding has no mouse kind.
    /// </summary>
    [Fact]
    public void REQ_INP_012_modifiers_are_bindable_and_mouse_buttons_are_not()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        DesktopInputSource source = InputSupport.Source(bindings, new FakeGamepadHub(), keyboard, new FakePointer());

        Assert.True(bindings.Keyboard(0).Set(InputAction.B, PhysicalControl.Key((int)Key.ShiftLeft)));
        keyboard.Press(Key.ShiftLeft);
        Assert.Equal(InputAction.B, InputSupport.Sample(source, 0));

        // No PhysicalControlKind stands for a mouse button, and a set only accepts the controls
        // of its own device kind.
        Assert.DoesNotContain(
            "mouse",
            string.Join(',', Enum.GetNames<PhysicalControlKind>()),
            StringComparison.OrdinalIgnoreCase);
        Assert.False(bindings.Keyboard(0).Set(InputAction.A, PhysicalControl.Button(0)));
        Assert.Equal(PhysicalControl.Key((int)Key.X), bindings.Keyboard(0).Get(InputAction.A));
    }

    /// <summary>
    /// REQ-INP-030: an analog command bound to an action is pressed from 0.34 of the
    /// <b>shaped</b> value of REQ-INP-031 — a stick pushed to 0.40 of its travel is released,
    /// 0.45 is pressed.
    /// </summary>
    /// <remarks>
    /// AMD-021 R2-011 <b>replaces</b> AMD-020 for <c>Q-048</c> (§4.1): the 0.34 applies to the
    /// value shaped by the dead zone and the saturation, not to the raw travel. The round-1
    /// reading of this test was the calibrated one, AMD-020 reversed it to the raw travel, and
    /// R2-011 settles it on the shaped value with its own vectors. <c>Q-058</c> listed the
    /// assertions this made obsolete; they are the ones rewritten here. With a dead zone of 0.2
    /// and a saturation of 0.9 the shaped value is (r − 0.2) / 0.7, so the bound of 0.34 sits at
    /// a raw travel of 0.438 and the 30 % / 40 % of the old reading are both released.
    /// </remarks>
    [Fact] // REQ-INP-030 — AMD-021 R2-011 (Q-048, Q-058)
    public void REQ_INP_030_the_digital_threshold_is_zero_point_thirty_four()
    {
        Assert.Equal(0.34f, AnalogCalibration.PressThreshold);
        Assert.False(AnalogCalibration.IsPressed(0.33999f));
        Assert.True(AnalogCalibration.IsPressed(0.34f));

        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        pad.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        pad.ReleaseAll();

        // R2-011: the threshold is applied to the shaped value, so the half-axis the press
        // decision reads is re-spread between the dead zone and the saturation.
        Assert.Equal(0.142857f, AnalogCalibration.HalfAxis(-0.30f, 0f, -1), 1e-5f);
        Assert.Equal(0.285714f, AnalogCalibration.HalfAxis(-0.40f, 0f, -1), 1e-5f);

        // The two vectors of the criterion, on the device: 0.40 of travel released, 0.45
        // pressed.
        pad.Axis(StandardGamepad.RightStickX, -0.40f);
        Assert.False(InputSupport.Sample(source, 0).HasFlag(InputAction.RotLeft));

        pad.Axis(StandardGamepad.RightStickX, -0.45f);
        Assert.True(InputSupport.Sample(source, 0).HasFlag(InputAction.RotLeft));

        // The old 30 % and 34 % of AMD-020 are now released, and the exact bound is 0.438 of
        // raw travel (0.2 + 0.34 × 0.7).
        foreach (float travel in new[] { 0.30f, 0.34f, 0.40f, 0.4379f })
        {
            pad.Axis(StandardGamepad.RightStickX, -travel);
            Assert.False(
                InputSupport.Sample(source, 0).HasFlag(InputAction.RotLeft),
                $"{travel} of travel must stay released");
        }

        pad.Axis(StandardGamepad.RightStickX, -0.4381f);
        Assert.True(InputSupport.Sample(source, 0).HasFlag(InputAction.RotLeft));

        pad.Axis(StandardGamepad.RightStickX, -0.60f);
        Assert.True(InputSupport.Sample(source, 0).HasFlag(InputAction.RotLeft));

        // A trigger at 0.5 of travel shapes to 0.429, so the action stays pressed.
        pad.ReleaseAll().Trigger(StandardGamepad.LeftTrigger, 0.5f);
        Assert.True(InputSupport.Sample(source, 0).HasFlag(InputAction.ZoomIn));
    }

    /// <summary>
    /// AMD-021 R2-011, third vector: a stick held at 45° with a travel of 0.6 shapes to
    /// components of 0.404, so both Up and Right are pressed.
    /// </summary>
    [Fact] // REQ-INP-030, REQ-INP-031 — AMD-021 R2-011
    public void REQ_INP_030_a_diagonal_push_of_zero_point_six_presses_both_directions()
    {
        float component = 0.6f / MathF.Sqrt(2f);
        Assert.Equal(0.404f, AnalogCalibration.HalfAxis(component, component, 1), 1e-3f);

        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        pad.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        pad.ReleaseAll();

        // The right stick is the bound one (REQ-INP-032); its X and Y both at 0.424 of travel
        // is the 45° push of the vector, and the shaped components clear 0.34 together. Up is
        // the positive half of the Y axis (SPEC-08 §4.3).
        pad.Axis(StandardGamepad.RightStickX, component).Axis(StandardGamepad.RightStickY, component);
        InputAction actions = InputSupport.Sample(source, 0);
        Assert.True(actions.HasFlag(InputAction.RotRight), $"got {actions}");
        Assert.True(actions.HasFlag(InputAction.RotUp), $"got {actions}");
    }

    /// <summary>
    /// REQ-INP-031 as amended by AMD-021 R2-011: radial dead zone 0.2, saturation 0.9, linear
    /// re-spread in between, and a trigger dead zone of <b>0.2</b> (§4.1 replaces the 0.1 the
    /// requirement carried; <c>Q-058</c>).
    /// </summary>
    [Fact] // REQ-INP-031 — AMD-021 R2-011 (Q-058)
    public void REQ_INP_031_the_dead_zones_are_the_documented_ones()
    {
        Assert.Equal(0.2f, AnalogCalibration.StickDeadZone);
        Assert.Equal(0.9f, AnalogCalibration.StickSaturation);
        Assert.Equal(0.2f, AnalogCalibration.TriggerDeadZone);
        Assert.Equal(0.9f, AnalogCalibration.TriggerSaturation);

        // The trigger is re-spread between the same two bounds as a stick.
        Assert.Equal(0f, AnalogCalibration.Trigger(0.2f));
        Assert.Equal(0.428571f, AnalogCalibration.Trigger(0.5f), 1e-5f);
        Assert.Equal(0.5f, AnalogCalibration.Trigger(0.55f), 1e-4f);

        // A drift of 0.15 is inside the dead zone: no action at all.
        Assert.Equal(0f, AnalogCalibration.Stick(0.15f, 0f));
        Assert.Equal(0f, AnalogCalibration.HalfAxis(0.15f, 0f, 1));

        // At saturation the value is exactly 1.
        Assert.Equal(1f, AnalogCalibration.Stick(0.9f, 0f), 5);
        Assert.Equal(1f, AnalogCalibration.Stick(1f, 0f), 5);

        // Half-way between the two bounds the re-spread gives one half.
        Assert.Equal(0.5f, AnalogCalibration.Stick(0.55f, 0f), 4);

        // The dead zone is radial: a diagonal push of the same magnitude behaves the same.
        float diagonal = 0.55f / MathF.Sqrt(2f);
        Assert.Equal(0.5f, MathF.Sqrt(
            (AnalogCalibration.Stick(diagonal, diagonal) * AnalogCalibration.Stick(diagonal, diagonal))
            + (AnalogCalibration.Stick(diagonal, diagonal) * AnalogCalibration.Stick(diagonal, diagonal))), 3);

        Assert.Equal(0f, AnalogCalibration.Trigger(0.1f));
        Assert.Equal(1f, AnalogCalibration.Trigger(1f), 5);
    }

    /// <summary>
    /// REQ-INP-032: by default the left stick is bound to nothing — pushing it to the limit
    /// produces no NES direction and no camera action.
    /// </summary>
    [Fact]
    public void REQ_INP_032_the_left_stick_is_unbound_by_default()
    {
        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        pad.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        pad.ReleaseAll();

        pad.Axis(StandardGamepad.LeftStickX, 1f).Axis(StandardGamepad.LeftStickY, 1f);
        Assert.Equal(InputAction.None, InputSupport.Sample(source, 0));

        // The hat alone drives the directions.
        pad.ReleaseAll().Press(StandardGamepad.DPadLeft);
        Assert.Equal(InputAction.Left, InputSupport.Sample(source, 0));

        // The user may bind a left-stick half-axis by remapping.
        BindingSet set = bindings.Gamepad("Pad");
        Assert.True(set.Set(InputAction.Up, PhysicalControl.AxisPositive(StandardGamepad.LeftStickY)));
        pad.ReleaseAll().Axis(StandardGamepad.LeftStickY, 1f);
        Assert.Equal(InputAction.Up, InputSupport.Sample(source, 0));
    }

    /// <summary>
    /// REQ-INP-033: a camera action produced by a stick is digital — 50 % and 100 % of travel
    /// give the same bit and therefore the same rotation speed.
    /// </summary>
    [Fact]
    public void REQ_INP_033_a_camera_action_from_a_stick_is_digital()
    {
        InputBindings bindings = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = InputSupport.Source(bindings, hub);

        pad.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);
        pad.ReleaseAll();

        pad.Axis(StandardGamepad.RightStickX, 0.5f);
        InputAction half = InputSupport.Sample(source, 0);
        pad.Axis(StandardGamepad.RightStickX, 1f);
        InputAction full = InputSupport.Sample(source, 0);

        Assert.Equal(InputAction.RotRight, half);
        Assert.Equal(half, full);
    }

    /// <summary>
    /// SPEC-08 §5.1: the seven vectors of the mask table, with the default configuration and both
    /// ports in Controller.
    /// </summary>
    [Fact]
    public void The_mask_vectors_of_paragraph_5_1_are_reproduced()
    {
        InputBindings bindings = new();
        FakeKeyboard keyboard = new();
        FakeGamepad pad = new("Pad");
        FakeGamepadHub hub = new FakeGamepadHub().Plug(pad);
        DesktopInputSource source = InputSupport.Source(bindings, hub, keyboard);

        Check(() => keyboard.Press(Key.Up).Press(Key.X), 0x11, 0x11);
        Check(() => keyboard.Press(Key.Enter).Press(Key.Space), 0x0C, 0x0C);
        Check(() => keyboard.Press(Key.F5).Press(Key.F9), 0x500, 0x00);
        Check(() => keyboard.Press(Key.Escape), 0x4000, 0x00);

        // The pad becomes J1's on its first use.
        pad.Press(StandardGamepad.South);
        InputSupport.Sample(source, 0);

        Check(
            () => pad.Press(StandardGamepad.South)
                .Press(StandardGamepad.West)
                .Press(StandardGamepad.DPadDown),
            0x23,
            0x23,
            padCase: true);

        Check(() => pad.Axis(StandardGamepad.RightStickX, 0.20f), 0x00, 0x00, padCase: true);
        Check(() => pad.Axis(StandardGamepad.RightStickX, 0.60f), 0x800, 0x00, padCase: true);

        void Check(Action press, int mask, int portByte, bool padCase = false)
        {
            keyboard.ReleaseAll();
            pad.ReleaseAll();
            InputSupport.Sample(source, 0);

            press();
            InputAction actual = InputSupport.Sample(source, 0);
            string label = padCase ? "pad" : "keyboard";
            Assert.Equal((InputAction)mask, actual);
            Assert.Equal(
                portByte,
                InputRouting.CoreWord(actual, InputAction.None) & 0xFF);
            Assert.True(mask == (int)actual, $"{label} vector gave 0x{(int)actual:X}");
        }
    }
}
