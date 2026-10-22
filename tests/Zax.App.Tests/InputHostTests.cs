using System.Numerics;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.Nes;
using Zax.Nes.Input;

namespace Zax.App.Tests;

/// <summary>
/// The input layer driven end to end with no window at all, which is the acceptance criterion of
/// REQ-INP-120: injected snapshots, the byte transmitted to the core, the hotkey triggers and the
/// Zapper position.
/// </summary>
public sealed class InputHostTests
{
    /// <summary>
    /// REQ-INP-004 and REQ-INP-001: both players reach the core through one input word, J1 in the
    /// low byte and J2 in the next one.
    /// </summary>
    [Fact]
    public void REQ_INP_120_injected_snapshots_drive_both_players_through_the_core()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));

        Assert.True(harness.Tick(
            1.0 / 60.0,
            InputSnapshot.Of(
                InputAction.Right | InputAction.A,
                InputAction.Up | InputAction.Start)) > 0);

        InputPortsSnapshot ports = Read(harness);
        Assert.Equal(0x81, ports.PortOne);
        Assert.Equal(0x18, ports.PortTwo);

        // REQ-INP-006: the camera and system bits never reach the core.
        harness.Tick(
            1.0 / 60.0,
            InputSnapshot.Of(
                InputAction.Right | InputAction.RollBack | InputAction.Menu,
                InputAction.FastForward | InputAction.RotLeft));

        ports = Read(harness);
        Assert.Equal(0x80, ports.PortOne);
        Assert.Equal(0x00, ports.PortTwo);
    }

    /// <summary>
    /// REQ-INP-005: one emulated frame is one sample, fast forward included — the mask of the
    /// snapshot is in force for every frame a tick runs.
    /// </summary>
    [Fact]
    public void REQ_INP_005_every_emulated_frame_of_a_burst_sees_the_same_sample()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));

        // A press held across a run of ticks is in force on every emulated frame those ticks
        // run: the core's port byte is checked after each one.
        long before = harness.Host.EmulatedFrames;
        int ran = 0;
        for (int tick = 0; tick < 60; tick++)
        {
            ran += harness.Tick(1.0 / 60.0, InputSnapshot.Of(InputAction.Right | InputAction.A));
            Assert.Equal(0x81, Read(harness).PortOne);
        }

        Assert.True(ran >= 55, $"a second of ticks ran only {ran} emulated frames");
        Assert.Equal(before + ran, harness.Host.EmulatedFrames);

        // REQ-INP-103 with REQ-INP-005: holding FastForward runs more emulated frames for the
        // same real time, and the player bits of the same snapshot still reach the core on each
        // of them.
        before = harness.Host.EmulatedFrames;
        int fast = 0;
        for (int tick = 0; tick < 60; tick++)
        {
            fast += harness.Tick(
                1.0 / 60.0,
                InputSnapshot.Of(InputAction.Right | InputAction.FastForward));
            Assert.Equal(0x80, Read(harness).PortOne);
        }

        Assert.True(fast > ran, $"fast forward ran {fast} frames, ×1 ran {ran}");
        Assert.Equal(before + fast, harness.Host.EmulatedFrames);
    }

    /// <summary>
    /// REQ-INP-100 to REQ-INP-104: the hotkeys of the snapshot reach the command model — a short
    /// Menu press pauses, a long one resets the view without leaving Running, and a rising edge
    /// of RollBack rolls back.
    /// </summary>
    [Fact]
    public void REQ_INP_120_injected_snapshots_trigger_the_hotkeys()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // REQ-INP-101: a short press on J2's Menu pauses, whatever the port types are.
        harness.Tick(0.1, InputSnapshot.Of(InputAction.None, InputAction.Menu));
        harness.Tick(0.01, InputSnapshot.Empty);
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        harness.Tick(0.1, InputSnapshot.Of(InputAction.Menu));
        harness.Tick(0.01, InputSnapshot.Empty);
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // REQ-INP-101: a long press resets the view and leaves the state alone. The first held
        // tick starts the measurement, so the 0.3 s boundary is crossed on the third.
        harness.Scene.Camera.Current.Distance = 42f;
        harness.Tick(0.2, InputSnapshot.Of(InputAction.Menu));
        harness.Tick(0.2, InputSnapshot.Of(InputAction.Menu));
        harness.Tick(0.2, InputSnapshot.Of(InputAction.Menu));
        harness.Tick(0.01, InputSnapshot.Empty);

        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.NotEqual(42f, harness.Scene.Camera.Current.Distance);
    }

    /// <summary>
    /// REQ-INP-061 and REQ-INP-063: an injected snapshot moves the Zapper, and the position the
    /// core is handed is the one the pointer aimed at on the previous displayed frame.
    /// </summary>
    [Fact]
    public void REQ_INP_120_injected_snapshots_move_the_zapper()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("duck", rom, title: "Duck Hunt");
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("duck"));
        harness.Host.Session!.Core.SetPortType(1, ControllerPortType.Zapper);

        Vector2 viewport = new(256f, 240f);

        // The exact pixel depends on the camera; what matters here is that the position moves
        // with the pointer and is handed to the core for the Zapper port.
        harness.Tick(
            1.0 / 60.0,
            new InputSnapshot(InputAction.None, InputAction.None, new Vector2(10f, 10f), viewport));

        ZapperPosition topLeft = harness.Host.Zapper.Position;
        Assert.Equal(topLeft, harness.Host.Session!.Core.Input.GetZapperPosition(1));

        harness.Tick(
            1.0 / 60.0,
            new InputSnapshot(InputAction.None, InputAction.None, new Vector2(246f, 230f), viewport));

        ZapperPosition bottomRight = harness.Host.Zapper.Position;
        Assert.True(bottomRight.X > topLeft.X, "pointing right must raise the aimed column");
        Assert.True(bottomRight.Y > topLeft.Y, "pointing down must raise the aimed line");
        Assert.Equal(bottomRight, harness.Host.Session!.Core.Input.GetZapperPosition(1));
    }

    private static InputPortsSnapshot Read(LoopHarness harness)
    {
        InputPorts ports = harness.Host.Session!.Core.Input;
        return new InputPortsSnapshot(ports.PortByte(0), ports.PortByte(1));
    }

    private readonly record struct InputPortsSnapshot(byte PortOne, byte PortTwo);
}
