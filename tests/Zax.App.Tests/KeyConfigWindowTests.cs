using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Ui;
using Zax.Platform.Input;
using Silk.NET.Input;

namespace Zax.App.Tests;

/// <summary>
/// The key configuration window of AMD-010 REQ-UI-203: the actions of SPEC-08 for both players
/// and the menu commands, bound to the keyboard and to a gamepad through the capture rules of
/// SPEC-08 (REQ-INP-042 to REQ-INP-047, Q-033).
/// </summary>
[Collection(ImGuiCollection.Name)]
public sealed class KeyConfigWindowTests
{
    /// <summary>
    /// REQ-UI-203: the window lists the NES buttons of both players, the camera actions, the
    /// system hotkeys and the menu commands.
    /// </summary>
    [Fact]
    public void REQ_UI_203_the_window_lists_the_actions_of_both_players_and_the_commands()
    {
        using UiHarness harness = new("Arkanoid");
        KeyConfigModel model = harness.Ui.Keys;

        Assert.Equal(
            ["Player 1 — NES buttons", "Player 2 — NES buttons", "Camera", "System hotkeys", "Menu commands"],
            model.Sections.Select(static section => section.Title));

        // The 19 actions of §4.1 are all there for player 1, and the eight NES buttons again for
        // player 2 (REQ-INP-001).
        foreach (InputAction action in InputActions.All)
        {
            Assert.Contains(
                model.AllRows(),
                row => row.Player == 0 && row.Target == BindingTarget.Of(action));
        }

        Assert.Equal(8, model.AllRows().Count(static row => row.Player == 1));

        // Every argument-less command of the model is bindable (REQ-UI-203).
        foreach (ZaxCommand command in harness.Host.Commands.Commands)
        {
            bool bindable = !KeyConfigModel.CommandsNeedingArguments.Contains(command.Name);
            Assert.Equal(
                bindable,
                model.AllRows().Any(row => row.Target == BindingTarget.OfCommand(command.Name)));
        }
    }

    /// <summary>
    /// REQ-UI-203 and REQ-INP-042: clicking a keyboard cell clears the binding and starts
    /// listening; the next key pressed becomes the binding, and the game answers to it.
    /// </summary>
    [Fact]
    public void REQ_UI_203_a_keyboard_capture_through_the_window_rebinds_the_action()
    {
        using UiHarness harness = new("Arkanoid");
        RebindingSession session = harness.Input.Rebinding;

        KeyConfigRow row = RowOf(harness, BindingTarget.Of(InputAction.A), player: 0);
        Assert.True(KeyConfigModel.Begin(session, row, BindingDeviceKind.Keyboard));

        // REQ-INP-042: the field was cleared the moment it was activated.
        Assert.False(harness.Bindings.Keyboard(0).Get(InputAction.A).IsBound);
        Assert.True(KeyConfigModel.IsListeningFor(session, row, BindingDeviceKind.Keyboard));

        // Q-033 / AMD-015: a key held when the field opened is ignored until it is released, so
        // the capture needs a real rising edge.
        harness.Input.Sample(1f / 60f);
        harness.Keyboard.Press(Key.K);
        harness.Input.Sample(1f / 60f);

        Assert.False(session.IsListening);
        Assert.Equal((int)Key.K, harness.Bindings.Keyboard(0).Get(InputAction.A).Code);

        // The game answers to K from the next sample on (REQ-INP-047: not this one).
        harness.Input.EndSample();
        InputSnapshot snapshot = harness.Input.Sample(1f / 60f);
        Assert.True(snapshot.PlayerOne.HasFlag(InputAction.A));
    }

    /// <summary>
    /// REQ-UI-203, acceptance criterion: a menu command bound to a gamepad button runs when the
    /// button is pressed.
    /// </summary>
    [Fact]
    public void REQ_UI_203_a_menu_command_bound_to_a_pad_button_runs()
    {
        using UiHarness harness = new("Arkanoid");
        FakeGamepad pad = new("pad-1");
        harness.Pads.Plug(pad);
        harness.Input.Pads.SkipStartupDelay();
        harness.Input.Pads.Assign(0, pad);

        RebindingSession session = harness.Input.Rebinding;
        KeyConfigRow row = RowOf(harness, BindingTarget.OfCommand(ZaxCommands.ShowAbout), player: 0);
        Assert.True(KeyConfigModel.Begin(session, row, BindingDeviceKind.Gamepad));

        harness.Input.Sample(1f / 60f);
        pad.Press(StandardGamepad.North);
        harness.Input.Sample(1f / 60f);

        Assert.False(session.IsListening);
        Assert.Equal(
            StandardGamepad.North,
            harness.Input.Pads.GamepadBindings(0)!.Get(BindingTarget.OfCommand(ZaxCommands.ShowAbout)).Code);

        // Pressing it now runs the command, which opens the about window.
        harness.Input.EndSample();
        harness.Input.Sample(1f / 60f);
        Assert.Contains(ZaxCommands.ShowAbout, harness.Input.TriggeredCommands);

        foreach (string command in harness.Input.TriggeredCommands)
        {
            harness.Host.Enqueue(command);
        }

        harness.Host.ApplyCommands();
        Assert.True(harness.Ui.State.About);
    }

    /// <summary>
    /// REQ-UI-072 and REQ-INP-045: stopping the capture leaves the action without a binding,
    /// since activating the cell erased it.
    /// </summary>
    [Fact]
    public void REQ_UI_072_stopping_the_capture_leaves_the_action_unbound()
    {
        using UiHarness harness = new("Arkanoid");
        RebindingSession session = harness.Input.Rebinding;

        KeyConfigRow row = RowOf(harness, BindingTarget.Of(InputAction.B), player: 0);
        KeyConfigModel.Begin(session, row, BindingDeviceKind.Keyboard);
        session.Stop();

        Assert.False(session.IsListening);
        Assert.False(harness.Bindings.Keyboard(0).Get(InputAction.B).IsBound);
    }

    /// <summary>
    /// REQ-INP-046: in the gamepad column, a player with no pad has no set, so no capture can
    /// start.
    /// </summary>
    [Fact]
    public void REQ_INP_046_a_player_without_a_pad_cannot_be_captured()
    {
        using UiHarness harness = new("Arkanoid");
        RebindingSession session = harness.Input.Rebinding;

        Assert.Null(KeyConfigModel.SetOf(
            harness.Bindings,
            harness.Input.Pads,
            player: 0,
            BindingDeviceKind.Gamepad));

        KeyConfigRow row = RowOf(harness, BindingTarget.Of(InputAction.Left), player: 0);
        Assert.False(KeyConfigModel.Begin(session, row, BindingDeviceKind.Gamepad));
        Assert.False(session.IsListening);
    }

    /// <summary>REQ-INP-049: the window's reset puts the defaults of §4.2 back.</summary>
    [Fact]
    public void REQ_INP_049_the_reset_restores_the_default_bindings()
    {
        using UiHarness harness = new("Arkanoid");

        harness.Bindings.Keyboard(0).Clear(BindingTarget.Of(InputAction.A));
        Assert.False(harness.Bindings.Keyboard(0).Get(InputAction.A).IsBound);

        harness.Bindings.ResetAll();
        Assert.True(harness.Bindings.Keyboard(0).Get(InputAction.A).IsBound);
    }

    /// <summary>REQ-UI-203: the window draws, with and without a pad assigned.</summary>
    [Fact]
    public void REQ_UI_203_the_window_draws()
    {
        using UiHarness harness = new("Arkanoid");
        using ImGuiHarness imgui = new();

        harness.Ui.State.KeyConfig = true;
        imgui.Frames(3, () => harness.Ui.Draw(1f / 60f));

        FakeGamepad pad = new("pad-1");
        harness.Pads.Plug(pad);
        harness.Input.Pads.SkipStartupDelay();
        harness.Input.Pads.Assign(0, pad);

        // A capture in progress greys the other cells out (REQ-UI-071); the frame must still
        // lay out.
        KeyConfigModel.Begin(
            harness.Input.Rebinding,
            RowOf(harness, BindingTarget.Of(InputAction.Start), player: 0),
            BindingDeviceKind.Keyboard);

        imgui.Frames(2, () => harness.Ui.Draw(1f / 60f));
        Assert.True(harness.Ui.State.KeyConfig);
    }

    private static KeyConfigRow RowOf(UiHarness harness, BindingTarget target, int player)
        => harness.Ui.Keys.AllRows().First(row => row.Player == player && row.Target == target);
}
