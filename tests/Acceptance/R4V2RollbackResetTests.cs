using Silk.NET.Input;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.Nes;
using Zax.Nes.Cpu;
using Zax.Platform.Input;
using Zax.Profiles;
using Zax.Render;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 verification (role R4) of lot <c>v2-5-rollback-reset</c>: AMD-042 (the rollback
/// restores the scripts' persistent data and the camera) and AMD-044 (the soft reset of the
/// console's RESET button).
/// </summary>
/// <remarks>
/// <para>
/// Requirements in scope: REQ-LOOP-069, REQ-LOOP-070, REQ-LOOP-071, REQ-CORE-122,
/// REQ-LOOP-072, REQ-INP-125, and the amended REQ-CORE-006 and REQ-SCN-024. The first-frame
/// camera snap of AMD-045 REQ-SCN-073 belongs to another lot and is not verified here; the
/// clause of REQ-LOOP-070 that asks for the restored camera to be <b>shown</b> straight at its
/// setpoints is in scope, because REQ-LOOP-070 states it itself.
/// </para>
/// <para>
/// Everything is checked from the outside: the loop through <see cref="HostDriver"/>, the
/// scripts through <c>PData</c> and Lua globals, the camera through the public
/// <see cref="SceneCamera"/>, the core through <see cref="NesCore"/> and the two public
/// <c>cpu_reset</c> ROMs, whose shell protocol is driven by this file's own runner.
/// </para>
/// </remarks>
public sealed class R4V2RollbackResetTests
{
    /// <summary>
    /// A test profile whose <c>Update</c> hook writes the frame number into <c>PData[0]</c>, as
    /// the acceptance criterion of REQ-LOOP-070 prescribes, and which counts every call of the
    /// four other hooks in Lua globals, so that "no <c>Save</c>, <c>Load</c>, <c>Start</c> or
    /// <c>End</c> hook was called" is an observable fact.
    /// </summary>
    private const string ProbeScript = """
        saves = 0
        loads = 0
        starts = 0
        ends = 0
        function Start() starts = starts + 1 end
        function End() ends = ends + 1 end
        function Save() saves = saves + 1 end
        function Load() loads = loads + 1 end
        function Update() SetPData(0, frame) end
        """;

    // ----------------------------------------------------------------- AMD-042 REQ-LOOP-069

    /// <summary>
    /// REQ-LOOP-069 — a snapshot of the history carries three things taken at the same instant:
    /// the complete core state, the 64 <c>PData</c> values as <c>GetPData(i)</c> reads them, and
    /// the camera state REQ-SCN-024 defines.
    /// </summary>
    /// <remarks>
    /// The snapshot captured at the core frame counter 540 is read back and compared with what
    /// the running game held at that very instant, recorded by the test one frame earlier.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-069
    public void REQ_LOOP_069_a_snapshot_carries_the_core_the_pdata_and_the_camera()
    {
        using HostDriver host = Probe("r4-v2-069");

        // 540 frames run, so the snapshot of the counter 540 is not taken yet: what the game
        // holds now is exactly what that snapshot must carry.
        Assert.Equal(540, host.RunFrames(540));
        Assert.Equal(540, host.Session!.Core.FrameCount);

        double pdataAtCapture = host.Session!.Scripts.ReadPersistentData(1);
        CameraStateData cameraAtCapture = host.Scene.Camera.Capture();
        Assert.True(pdataAtCapture > 0, "the probe profile never wrote PData[0].");

        // One more frame: the snapshot of the counter 540 is taken before it executes.
        Assert.Equal(1, host.RunFrames(1));
        Assert.Equal(541, host.Session!.Core.FrameCount);

        // Everything moves on afterwards, so a snapshot that read its values later would be
        // caught out.
        host.Session!.Scripts.Evaluate("PData[1] = -999");
        host.Scene.Camera.Setpoints(CameraMode.Tpv).Angles = new System.Numerics.Vector3(45f, 90f, 0f);

        RollbackSnapshot? snapshot = host.Rollback.TakeAt(host.Rollback.Count - 1);
        Assert.NotNull(snapshot);
        Assert.NotEmpty(snapshot.Core);

        float[] values = PersistentData.Read(snapshot.PersistentData);
        Assert.Equal(PersistentData.BlockSize, snapshot.PersistentData.Length);
        Assert.Equal(PersistentData.EntryCount, values.Length);
        Assert.Equal((float)pdataAtCapture, values[0]);
        Assert.Equal(cameraAtCapture, snapshot.Camera);
    }

    // ----------------------------------------------------------------- AMD-042 REQ-LOOP-070

    /// <summary>
    /// REQ-LOOP-070, acceptance criterion — no run-ahead, NTSC, 10.5 s of play then a rollback:
    /// the core goes back to the frame counter 540 (REQ-LOOP-032) and <c>PData[0]</c> goes back
    /// to the value it held when that snapshot was captured, instead of staying at about 630.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-070
    public void REQ_LOOP_070_a_rollback_restores_the_pdata_of_the_restored_snapshot()
    {
        using HostDriver host = Probe("r4-v2-070a");

        Assert.Equal(540, host.RunFrames(540));
        double pdataAtCapture = host.Session!.Scripts.ReadPersistentData(1);

        // 10.5 s of NTSC play: 630 frames, last snapshot 600, second-to-last 540.
        Assert.Equal(90, host.RunFrames(90));
        Assert.Equal(630, host.Session!.Core.FrameCount);
        double pdataBefore = host.Session!.Scripts.ReadPersistentData(1);
        Assert.True(pdataBefore > 600, $"PData[0] was {pdataBefore} before the rollback.");

        host.Run(ZaxCommands.Rollback);

        Assert.Equal(540, host.Session!.Core.FrameCount);
        Assert.Equal(
            pdataAtCapture,
            host.Session!.Scripts.ReadPersistentData(1),
            4);
    }

    /// <summary>
    /// REQ-LOOP-070 — a rollback calls none of the script hooks: not <c>Save</c>, not
    /// <c>Load</c> (they stay reserved for savestates and serialisation, REQ-LUA-075), and
    /// neither <c>Start</c> nor <c>End</c>. The 64 entries are written back the way
    /// <c>SetPData</c> writes them.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-070
    public void REQ_LOOP_070_a_rollback_calls_no_script_hook()
    {
        using HostDriver host = Probe("r4-v2-070b");

        host.RunFrames(630);

        // One Start at launch, nothing else so far.
        Assert.Equal(1, host.Session!.Scripts.EvaluateNumber("return starts"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return ends"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return saves"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return loads"));

        host.Run(ZaxCommands.Rollback);

        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return saves"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return loads"));
        Assert.Equal(1, host.Session!.Scripts.EvaluateNumber("return starts"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return ends"));
    }

    /// <summary>
    /// REQ-LOOP-070 — the setpoints of the camera go back to what they were when the restored
    /// snapshot was captured: the view turned by 90° after the capture is undone.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-070
    public void REQ_LOOP_070_a_rollback_restores_the_camera_setpoints()
    {
        using HostDriver host = Probe("r4-v2-070c");
        SceneCamera camera = host.Scene.Camera;

        host.RunFrames(541);
        float ryAtCapture = camera.Setpoints(CameraMode.Tpv).Angles.Y;

        // A 90° turn after the capture of the snapshot 540, settled by the smoothing the
        // frontend drives (REQ-SCN-019, REQ-SCN-059).
        Turn(camera, 90f);
        Assert.Equal(ryAtCapture + 90f, camera.Setpoints(CameraMode.Tpv).Angles.Y, 3);

        host.RunFrames(89);
        Assert.Equal(630, host.Session!.Core.FrameCount);

        host.Run(ZaxCommands.Rollback);

        Assert.Equal(ryAtCapture, camera.Setpoints(CameraMode.Tpv).Angles.Y, 3);
        Assert.Equal(CameraMode.Tpv, camera.Mode);
    }

    /// <summary>
    /// REQ-LOOP-070 — "la caméra rétablie DOIT être affichée <b>directement</b> à ses consignes,
    /// sans lissage depuis la vue d'avant": the first picture after the restoration shows the
    /// orientation of the moment of the capture, not the one the view had just before the
    /// rollback.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-070
    public void REQ_LOOP_070_the_restored_camera_is_shown_straight_at_its_setpoints()
    {
        using HostDriver host = Probe("r4-v2-070d");
        SceneCamera camera = host.Scene.Camera;

        host.RunFrames(541);
        float ryAtCapture = camera.Setpoints(CameraMode.Tpv).Angles.Y;

        Turn(camera, 90f);
        Assert.Equal(ryAtCapture + 90f, camera.Angles.Y, 1);

        host.RunFrames(89);
        host.Run(ZaxCommands.Rollback);

        // The frame the frontend draws right after the restoration (REQ-SCN-059: the camera is
        // advanced by real time, one rendered frame of 1/60 s here).
        camera.Advance(1f / 60f);

        Assert.Equal(ryAtCapture, camera.Angles.Y, 1);
    }

    /// <summary>
    /// REQ-LOOP-069 — what a snapshot still does <b>not</b> hold: the learned conversion state,
    /// the converted frames and the meshes. The image registry of REQ-E3D-007 keeps growing
    /// across a rollback instead of going back with the core.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-069
    public void REQ_LOOP_069_a_snapshot_holds_no_learned_conversion_state()
    {
        using HostDriver host = Probe("r4-v2-069b", rendering: true);

        host.RunFrames(630);
        int learned = host.Session!.Engine.Images.Count;
        Assert.True(learned > 0, "the conversion learned nothing to compare.");

        host.Run(ZaxCommands.Rollback);

        Assert.Equal(540, host.Session!.Core.FrameCount);
        Assert.Equal(learned, host.Session!.Engine.Images.Count);
    }

    // ----------------------------------------------------------------- AMD-042 REQ-SCN-024

    /// <summary>
    /// REQ-SCN-024 as amended — the camera state records the <b>remaining time</b> of the mode
    /// transition window, between 0 and 3 s, and not its deadline: whatever happens between the
    /// capture and the restore, the restored window holds what was left of it.
    /// </summary>
    [Fact] // REQ-SCN-024
    public void REQ_SCN_024_the_camera_state_records_the_remaining_transition_time()
    {
        SceneCamera camera = new();
        camera.ToggleMode();
        Assert.Equal(3f, camera.TransitionRemaining, 4);

        camera.Advance(1f);
        CameraStateData state = camera.Capture();
        Assert.Equal(2f, state.TransitionRemaining, 3);

        // Long after the window would have closed, the restored state still carries the two
        // seconds that were left: an absolute deadline would have gone stale.
        camera.Advance(5f);
        Assert.Equal(0f, camera.TransitionRemaining, 4);

        camera.Restore(state);
        Assert.Equal(2f, camera.TransitionRemaining, 3);
        Assert.InRange(camera.TransitionRemaining, 0f, 3f);
    }

    /// <summary>
    /// REQ-SCN-024 as amended, through a savestate — a quick save taken 1 s into a mode
    /// transition, then a quick load after the window would have closed, puts 2 s back.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-024
    public void REQ_SCN_024_a_savestate_round_trip_puts_the_remaining_time_back()
    {
        using HostDriver host = Probe("r4-v2-024");
        SceneCamera camera = host.Scene.Camera;

        host.RunFrames(120);
        camera.ToggleMode();
        camera.Advance(1f);
        Assert.Equal(2f, camera.TransitionRemaining, 3);

        host.Run(ZaxCommands.QuickSave);

        camera.Advance(5f);
        Assert.Equal(0f, camera.TransitionRemaining, 4);

        host.Run(ZaxCommands.QuickLoad);

        Assert.Equal(2f, camera.TransitionRemaining, 2);
        Assert.Equal(CameraMode.Fpv, camera.Mode);
    }

    // ----------------------------------------------------------------- AMD-042 REQ-LOOP-071

    /// <summary>
    /// REQ-LOOP-071, acceptance criterion — 10 s of play leave at least 9 snapshots, and the
    /// <b>end</b> of the game empties the history at once, before any new launch.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-071
    public void REQ_LOOP_071_the_end_of_a_game_empties_the_history()
    {
        using HostDriver host = Probe("r4-v2-071");

        host.RunFrames(600);
        Assert.True(host.Rollback.Count >= 9, $"only {host.Rollback.Count} snapshots after 10 s.");

        host.Run(ZaxCommands.Stop);

        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Equal(0, host.Rollback.Count);
        Assert.True(host.Rollback.IsEmpty);
        Assert.Null(host.CurrentGameId);
    }

    // ----------------------------------------------------------------- AMD-044 REQ-CORE-122

    /// <summary>
    /// REQ-CORE-122 — the CPU after a soft reset: the <c>I</c> flag is set, <c>S</c> drops by 3
    /// without anything being written to the stack, <c>PC</c> comes from the reset vector, and
    /// <c>A</c>, <c>X</c>, <c>Y</c> and every other flag are kept.
    /// </summary>
    [Fact] // REQ-CORE-122
    public void REQ_CORE_122_the_cpu_keeps_its_registers_but_for_I_PC_and_S()
    {
        NesCore core = Load(LoopRom.Counter());
        core.RunFrame(0);

        core.Cpu.A = 0x5A;
        core.Cpu.X = 0xC3;
        core.Cpu.Y = 0x1F;
        core.Cpu.S = 0xF0;
        core.Cpu.P = (byte)(Cpu6502.FlagC | Cpu6502.FlagN | Cpu6502.FlagD | Cpu6502.FlagU);

        // Three stack bytes the reset must not touch.
        core.WriteMemory(0x01F0, 0x11);
        core.WriteMemory(0x01EF, 0x22);
        core.WriteMemory(0x01EE, 0x33);

        core.SoftReset();

        Assert.Equal(0x5A, core.Cpu.A);
        Assert.Equal(0xC3, core.Cpu.X);
        Assert.Equal(0x1F, core.Cpu.Y);
        Assert.Equal(0xED, core.Cpu.S); // 0xF0 − 3
        Assert.True(core.Cpu.GetFlag(Cpu6502.FlagI), "the I flag must be set after a reset.");
        Assert.True(core.Cpu.GetFlag(Cpu6502.FlagC), "C must be kept.");
        Assert.True(core.Cpu.GetFlag(Cpu6502.FlagN), "N must be kept.");
        Assert.True(core.Cpu.GetFlag(Cpu6502.FlagD), "D must be kept.");
        Assert.False(core.Cpu.GetFlag(Cpu6502.FlagZ), "Z must be kept clear.");
        Assert.False(core.Cpu.GetFlag(Cpu6502.FlagV), "V must be kept clear.");

        // PC comes from $FFFC/$FFFD, which the minimal image points at $8000.
        Assert.Equal(0x8000, (int)core.Cpu.PC);
        Assert.False(core.Cpu.NmiPending, "no interrupt may be pending after a reset.");

        // Nothing written to the stack.
        Assert.Equal(0x11, core.ReadMemory(0x01F0));
        Assert.Equal(0x22, core.ReadMemory(0x01EF));
        Assert.Equal(0x33, core.ReadMemory(0x01EE));
    }

    /// <summary>
    /// REQ-CORE-122 — the internal RAM, the save RAM and the CHR RAM are kept, which is what
    /// distinguishes the soft reset from the power cycle of REQ-CORE-004.
    /// </summary>
    [Fact] // REQ-CORE-122
    public void REQ_CORE_122_the_memories_survive_a_soft_reset()
    {
        NesCore core = Load(BatteryRam());
        core.RunFrame(0);

        core.WriteMemory(0x0042, 0xA5);
        core.WriteMemory(0x0700, 0x3C);
        core.WriteMemory(0x6123, 0x77);
        WriteVideo(core, 0x0010, 0xE1);

        Assert.Equal(0xA5, core.ReadMemory(0x0042));
        Assert.Equal(0x77, core.ReadMemory(0x6123));
        Assert.Equal(0xE1, ReadVideo(core, 0x0010));

        core.SoftReset();

        Assert.Equal(0xA5, core.ReadMemory(0x0042));
        Assert.Equal(0x3C, core.ReadMemory(0x0700));
        Assert.Equal(0x77, core.ReadMemory(0x6123));
        Assert.Equal(0xE1, ReadVideo(core, 0x0010));

        // The power cycle of REQ-CORE-004 clears them, which is the contrast AMD-044 draws.
        core.Reset();
        Assert.Equal(0x00, core.ReadMemory(0x0042));
        Assert.Equal(0x00, core.ReadMemory(0x0700));
    }

    /// <summary>
    /// REQ-CORE-122 — the PPU and the APU follow their documented <i>after reset</i> state:
    /// PPUCTRL and PPUMASK go to 0 and the APU is silenced (<c>$4015</c> reads 0), while
    /// PPUSTATUS, OAMADDR and the video memories are untouched.
    /// </summary>
    [Fact] // REQ-CORE-122
    public void REQ_CORE_122_the_ppu_and_the_apu_go_to_their_after_reset_state()
    {
        NesCore core = Load(LoopRom.Counter());
        for (int frame = 0; frame < 3; frame++)
        {
            core.RunFrame(0);
        }

        // A byte of nametable RAM, written while the address increment is still 1.
        WriteVideo(core, 0x2100, 0x6B);

        core.Ppu.Write(0, 0x1E);
        core.Ppu.Write(1, 0x1E);

        // One pulse channel with a loaded length counter, so that $4015 reads non-zero.
        core.Apu.Write(0x4015, 0x01);
        core.Apu.Write(0x4000, 0x30);
        core.Apu.Write(0x4002, 0xFF);
        core.Apu.Write(0x4003, 0x08);
        core.RunFrame(0);

        // OAMADDR is set after the frame: sprite evaluation moves it while rendering is on.
        core.Ppu.Write(3, 0x40);

        bool vblank = core.Ppu.VBlankFlag;
        bool overflow = core.Ppu.SpriteOverflow;
        Assert.Equal(0x1E, core.Ppu.Control);
        Assert.Equal(0x1E, core.Ppu.Mask);
        Assert.NotEqual(0, core.Apu.PeekStatus() & 0x01);

        core.SoftReset();

        // PPUCTRL and PPUMASK to 0, the APU silenced ($4015 = 0).
        Assert.Equal(0, core.Ppu.Control);
        Assert.Equal(0, core.Ppu.Mask);
        Assert.Equal(0, core.Apu.PeekStatus());

        // PPUSTATUS, OAMADDR and the video memories are unchanged.
        Assert.Equal(vblank, core.Ppu.VBlankFlag);
        Assert.Equal(overflow, core.Ppu.SpriteOverflow);
        Assert.Equal(0x40, core.Ppu.OamAddress);
        Assert.Equal(0x6B, ReadVideo(core, 0x2100));
    }

    /// <summary>
    /// REQ-CORE-122 — the latch and the shift registers of the controller ports go back to
    /// zero, with no effect on the buttons the player holds, which are read again on the next
    /// frame.
    /// </summary>
    [Fact] // REQ-CORE-122
    public void REQ_CORE_122_the_controller_ports_are_cleared()
    {
        NesCore core = Load(LoopRom.Counter());
        core.RunFrame(0x00FF);

        // Strobe, then one read: the shift register holds the eight pressed buttons.
        core.Input.Write(1);
        core.Input.Write(0);
        Assert.Equal(1, core.Input.Read(0, 0) & 0x01);

        core.SoftReset();

        core.Input.Write(1);
        core.Input.Write(0);
        Assert.Equal(0, core.Input.Read(0, 0) & 0x01);
        Assert.Equal(0, core.Input.PortByte(0));

        // The buttons the player is still holding come back on the next frame.
        core.RunFrame(0x00FF);
        core.Input.Write(1);
        core.Input.Write(0);
        Assert.Equal(1, core.Input.Read(0, 0) & 0x01);
    }

    /// <summary>
    /// REQ-CORE-006 as amended by AMD-044 — the frame counter goes back to 0 when a ROM is
    /// loaded, on a hard reset <b>and</b> on a soft reset.
    /// </summary>
    [Fact] // REQ-CORE-006, REQ-CORE-122
    public void REQ_CORE_006_the_frame_counter_restarts_at_zero_on_a_soft_reset()
    {
        NesCore core = Load(LoopRom.Counter());
        Assert.Equal(0, core.FrameCount);

        for (int frame = 0; frame < 100; frame++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(100, core.FrameCount);

        core.SoftReset();
        Assert.Equal(0, core.FrameCount);

        for (int frame = 0; frame < 10; frame++)
        {
            core.RunFrame(0);
        }

        Assert.Equal(10, core.FrameCount);

        core.Reset();
        Assert.Equal(0, core.FrameCount);
    }

    /// <summary>
    /// REQ-CORE-122, complementary criterion — a home-made ROM that counts its resets in RAM
    /// reads 3 after three soft resets. The program stamps a magic byte at power-on, zeroes the
    /// counter, and increments it on every later entry through the reset vector.
    /// </summary>
    [Fact] // REQ-CORE-122
    public void REQ_CORE_122_a_rom_that_counts_its_resets_reads_three_after_three_presses()
    {
        NesCore core = Load(LoopRom.Build(ResetCounter));
        core.RunFrame(0);
        Assert.Equal(0x5A, core.ReadMemory(0x0011));
        Assert.Equal(0x00, core.ReadMemory(0x0010));

        for (int press = 1; press <= 3; press++)
        {
            core.SoftReset();
            core.RunFrame(0);
            Assert.Equal(press, core.ReadMemory(0x0010));
        }

        Assert.Equal(3, core.ReadMemory(0x0010));

        // The power cycle loses the count, which is why ram_after_reset fails under it.
        core.Reset();
        core.RunFrame(0);
        Assert.Equal(0x00, core.ReadMemory(0x0010));
    }

    /// <summary>
    /// REQ-CORE-122, acceptance criterion — the two public reset ROMs of blargg that ask for a
    /// press on RESET, <c>cpu_reset/ram_after_reset</c> and <c>cpu_reset/registers</c>, pass
    /// under the soft reset. SPEC-02 §3.12 as amended adds them to the required list.
    /// </summary>
    /// <remarks>
    /// The ROMs are not redistributed with the solution; the fact is skipped, not failed, when
    /// they are absent, like the rest of the public suite.
    /// </remarks>
    [Theory] // REQ-CORE-122, REQ-ACC-001
    [InlineData("cpu_reset/ram_after_reset.nes")]
    [InlineData("cpu_reset/registers.nes")]
    public void REQ_CORE_122_the_public_reset_roms_pass_under_a_soft_reset(string relative)
    {
        string? path = RomSuite.Find(relative);
        if (path is null)
        {
            return;
        }

        (int code, string text, long frames, int presses) = RunResetRom(path, soft: true);

        Assert.True(presses >= 1, $"{relative}: the ROM never asked for a reset ({frames} frames).");
        Assert.True(code == 0, $"{relative}: code {code} after {frames} frames, output: {text}");
    }

    // ----------------------------------------------------------------- AMD-044 REQ-LOOP-072

    /// <summary>
    /// REQ-LOOP-072, acceptance criterion — in game, a soft reset empties the rollback history,
    /// leaves <c>PData</c> alone and calls neither <c>End</c> nor <c>Start</c>.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-072
    public void REQ_LOOP_072_a_soft_reset_clears_the_history_and_keeps_the_scripts()
    {
        using HostDriver host = Probe("r4-v2-072a");

        host.RunFrames(300);
        Assert.True(host.Rollback.Count >= 5, $"only {host.Rollback.Count} snapshots.");

        host.Session!.Scripts.Evaluate("PData[12] = 1234");
        double pdata = host.Session!.Scripts.ReadPersistentData(1);
        Assert.True(pdata > 0, "the probe profile never wrote PData[0].");

        host.Run(ZaxCommands.SoftReset);

        Assert.Equal(0, host.Rollback.Count);
        Assert.Equal(pdata, host.Session!.Scripts.ReadPersistentData(1), 4);
        Assert.Equal(1234, host.Session!.Scripts.ReadPersistentData(12), 4);
        Assert.Equal(1, host.Session!.Scripts.EvaluateNumber("return starts"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return ends"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return saves"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return loads"));
    }

    /// <summary>
    /// REQ-LOOP-072 — the soft reset behaves like the hard reset of SPEC-07 for the rest of the
    /// program: the pause is lifted and the pacing clock is reset (no burst of catch-up frames).
    /// It does <b>not</b> relaunch the profile: the ROM is not read again and the learned
    /// conversion state stays.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-072
    public void REQ_LOOP_072_a_soft_reset_lifts_the_pause_without_relaunching_the_profile()
    {
        using HostDriver host = Probe("r4-v2-072b", rendering: true);

        host.RunFrames(300);
        int reads = host.RecentRoms.Count;
        int learned = host.Session!.Engine.Images.Count;
        Assert.True(learned > 0, "the conversion learned nothing to compare.");

        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);

        // A whole second goes by while the game is paused: an unreset pacing clock would owe
        // the loop a backlog of frames.
        host.Tick(1.0);

        host.Run(ZaxCommands.SoftReset);
        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(0, host.Session!.Core.FrameCount);

        // The pacing clock was reset, so the paused second is not owed: the next rendered frame
        // runs one or two emulated frames, not the five-frame backlog of REQ-LOOP-012.
        Assert.InRange(host.Tick(1.0 / 60.0), 1, 2);

        Assert.Equal(reads, host.RecentRoms.Count);
        Assert.Equal(learned, host.Session!.Engine.Images.Count);
        Assert.False(host.IsLoading);
    }

    /// <summary>
    /// REQ-CORE-122, assumed consequence — the Lua globals <c>frame</c> and <c>time</c> go back
    /// to 0 in the middle of a session, with no <c>End</c> and no <c>Start</c>: the probe's
    /// <c>Update</c> hook writes small frame numbers again after the soft reset.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-CORE-122, REQ-CORE-006
    public void REQ_CORE_122_the_lua_frame_global_restarts_without_End_or_Start()
    {
        using HostDriver host = Probe("r4-v2-122lua");

        host.RunFrames(300);
        Assert.True(host.Session!.Scripts.ReadPersistentData(1) > 200, "the frame global never grew.");

        host.Run(ZaxCommands.SoftReset);
        host.RunFrames(10);

        Assert.InRange(host.Session!.Scripts.ReadPersistentData(1), 0, 20);
        Assert.Equal(1, host.Session!.Scripts.EvaluateNumber("return starts"));
        Assert.Equal(0, host.Session!.Scripts.EvaluateNumber("return ends"));
    }

    /// <summary>
    /// REQ-LOOP-072 — the frontend offers the soft reset in the game menu, right next to the
    /// hard one, under the labels of AMD-044 §3; the row is active in game only.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-072
    public void REQ_LOOP_072_the_game_menu_offers_the_soft_reset_next_to_the_hard_one()
    {
        using HostDriver host = HostDriver.Create();
        UiDriver menu = UiDriver.MenuBar(host);

        List<string?> commands = [.. menu.Rows().Select(row => row.Command)];
        int hard = commands.IndexOf(ZaxCommands.Reset);
        int soft = commands.IndexOf(ZaxCommands.SoftReset);
        Assert.True(hard >= 0, "no menu row invokes the hard reset.");
        Assert.True(soft >= 0, "no menu row invokes the soft reset.");
        Assert.Equal(hard + 1, soft);

        // Nothing is running yet, so neither reset can be activated.
        Assert.False(menu.IsEnabled(ZaxCommands.SoftReset));

        host.Launch2DMode();
        UiDriver inGame = UiDriver.MenuBar(host);
        Assert.True(inGame.IsEnabled(ZaxCommands.SoftReset));

        // AMD-044 §3, with AMD-035's two languages.
        Assert.Equal("Soft Reset", UiLabel("English", "cmd.emulation.soft_reset"));
        Assert.Equal("Reset doux", UiLabel("French", "cmd.emulation.soft_reset"));
        Assert.Equal(
            UiDriver.CommandLabel(ZaxCommands.SoftReset),
            menu.Rows().First(row => row.Command == ZaxCommands.SoftReset).Label);
    }

    // ----------------------------------------------------------------- AMD-044 REQ-INP-125

    /// <summary>
    /// REQ-INP-125 — an extended action "Soft Reset" exists at the next free code of the table
    /// of SPEC-08 §4.1 (<c>0x80000</c>, after <c>QuickLoad</c> = <c>0x40000</c>), with no
    /// default binding on the keyboard or on a pad, and it never reaches the controller byte.
    /// </summary>
    [Fact] // REQ-INP-125
    public void REQ_INP_125_the_soft_reset_is_an_extended_action_with_the_next_free_code()
    {
        InputAction? action = InputActions.Parse("SoftReset");
        Assert.NotNull(action);
        Assert.Equal(0x80000, (int)action!.Value);
        Assert.Contains(InputAction.SoftReset, InputActions.All);
        Assert.Equal(0, InputActions.ControllerByte(InputAction.SoftReset));

        // The code is the next free one: no other action uses it, and it is twice QuickLoad's.
        Assert.Equal((int)InputAction.QuickLoad * 2, (int)InputAction.SoftReset);
        Assert.Single(InputActions.All, a => a == InputAction.SoftReset);

        // No default binding at all (SPEC-08 §4.2 and §4.3 as AMD-044 completes them).
        Assert.DoesNotContain(InputAction.SoftReset, DefaultBindings.PlayerOneKeyboard.Select(b => b.Action));
        Assert.DoesNotContain(InputAction.SoftReset, DefaultBindings.StandardGamepadSet.Select(b => b.Action));

        InputBindings bindings = new();
        Assert.Equal(PhysicalControl.None, bindings.Keyboard(0).Get(InputAction.SoftReset));
        Assert.Equal(PhysicalControl.None, bindings.Keyboard(1).Get(InputAction.SoftReset));
        Assert.Equal(PhysicalControl.None, bindings.Gamepad("R4 pad").Get(InputAction.SoftReset));
    }

    /// <summary>
    /// REQ-INP-125, acceptance criterion — the action can be bound in the controls screen, and
    /// a press on the bound control triggers REQ-LOOP-072: one soft reset per press, not one per
    /// frame the control is held.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-INP-125, REQ-LOOP-072
    public void REQ_INP_125_a_bound_soft_reset_triggers_the_command_once_per_press()
    {
        using HostDriver host = HostDriver.Create();

        // The controls screen lists the action, so a player can give it a control.
        Assert.Contains(
            UiDriver.KeyConfigRows(host),
            row => row.Target == InputActions.NameOf(InputAction.SoftReset));

        // Bound by hand, the binding layer reports the action like any other.
        InputBindings bindings = new();
        Assert.True(bindings.Keyboard(0).Set(InputAction.SoftReset, PhysicalControl.Key((int)Key.F4)));
        FakeKeyboard keyboard = new();
        FakeGamepadHub hub = new();
        DesktopInputSource source = InputSupport.Source(bindings, hub, keyboard);
        keyboard.Press(Key.F4);
        Assert.True(InputSupport.Sample(source, 0).HasFlag(InputAction.SoftReset));

        host.Launch2DMode();
        host.RunFrames(300);
        Assert.True(host.Rollback.Count >= 5, $"only {host.Rollback.Count} snapshots.");

        // One press: the reset runs in that rendered frame, before the emulated frames it
        // carries, so the history holds at most the snapshot of the brand-new frame 0.
        host.Tick(1.0 / 60.0, InputAction.SoftReset);
        Assert.InRange(host.Rollback.Count, 0, 1);
        Assert.InRange(host.Session!.Core.FrameCount, 0, 2);
        Assert.Equal(EmulationState.Running, host.State);

        // Held for five more rendered frames: one reset per press, not one per frame.
        host.RunFrames(5, InputAction.SoftReset);
        Assert.True(
            host.Session!.Core.FrameCount >= 5,
            $"the core was reset again while the control was held (frame {host.Session!.Core.FrameCount}).");
    }

    // ----------------------------------------------------------------------- helpers

    /// <summary>
    /// A machine-code program that counts the presses on RESET in RAM: at power-on it stamps
    /// <c>$5A</c> at <c>$0011</c> and zeroes the counter at <c>$0010</c>; every later entry
    /// through the reset vector finds the stamp and increments the counter.
    /// </summary>
    private static readonly byte[] ResetCounter =
    [
        0xA5, 0x11,             // $8000 LDA $11
        0xC9, 0x5A,             // $8002 CMP #$5A
        0xF0, 0x0B,             // $8004 BEQ $8011
        0xA9, 0x5A,             // $8006 LDA #$5A
        0x85, 0x11,             // $8008 STA $11
        0xA9, 0x00,             // $800A LDA #$00
        0x85, 0x10,             // $800C STA $10
        0x4C, 0x13, 0x80,       // $800E JMP $8013
        0xE6, 0x10,             // $8011 INC $10
        0x4C, 0x13, 0x80,       // $8013 JMP $8013
    ];

    /// <summary>
    /// A machine-code program that fills the first nametable with the tile numbers 0 to 255 and
    /// turns rendering on, over and over: the conversion then has something to learn, which is
    /// what the "learned state is kept" clause of REQ-LOOP-072 needs to be observable.
    /// </summary>
    private static readonly byte[] Renderer =
    [
        0xA9, 0x20,             // $8000 LDA #$20
        0x8D, 0x06, 0x20,       // $8002 STA $2006
        0xA9, 0x00,             // $8005 LDA #$00
        0x8D, 0x06, 0x20,       // $8007 STA $2006
        0xA2, 0x00,             // $800A LDX #$00
        0x8A,                   // $800C TXA
        0x8D, 0x07, 0x20,       // $800D STA $2007
        0xE8,                   // $8010 INX
        0xD0, 0xF9,             // $8011 BNE $800C
        0xA9, 0x1E,             // $8013 LDA #$1E
        0x8D, 0x01, 0x20,       // $8015 STA $2001
        0x4C, 0x00, 0x80,       // $8018 JMP $8000
    ];

    /// <summary>A driven host running the probe profile of this file.</summary>
    /// <param name="title">Title of the throw-away profile, also its library key.</param>
    /// <param name="rendering">
    /// Whether the ROM draws something, so that the conversion learns tile images
    /// (REQ-E3D-007). The default is the blank counter ROM of the loop tests.
    /// </param>
    private static HostDriver Probe(string title, bool rendering = false)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = title;
        profile.Scripts.Add(new ProfileScript("probe", true, ProbeScript, "*"));

        byte[] image = rendering
            ? SpecRom.Build(program: Renderer)
            : LoopRom.Counter();

        HostDriver host = HostDriver.Create(profile);
        host.Launch(host.EntryIdByTitle(title), host.WriteRom(image));
        return host;
    }

    /// <summary>
    /// Turns the orbital camera by <paramref name="degrees"/> around the vertical and lets the
    /// smoothing settle, as the frontend's rendered frames do (REQ-SCN-019, REQ-SCN-059).
    /// </summary>
    private static void Turn(SceneCamera camera, float degrees)
    {
        CameraSetpoint setpoint = camera.Setpoints(CameraMode.Tpv);
        setpoint.Angles = setpoint.Angles with { Y = setpoint.Angles.Y + degrees };
        for (int frame = 0; frame < 300; frame++)
        {
            camera.Advance(1f / 60f);
        }
    }

    /// <summary>A core with <paramref name="image"/> loaded, nothing run yet.</summary>
    private static NesCore Load(byte[] image)
    {
        NesCore core = new();
        Assert.True(core.LoadRom(image, "r4-v2.nes").Success);
        return core;
    }

    /// <summary>
    /// A minimal image with 8 KiB of battery-backed PRG-RAM at <c>$6000</c> and CHR <b>RAM</b>
    /// instead of CHR ROM, so that the three memories REQ-CORE-122 keeps can all be written.
    /// </summary>
    private static byte[] BatteryRam()
    {
        byte[] image = new byte[16 + (32 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2;
        image[5] = 0;        // no CHR ROM: the board carries 8 KiB of CHR RAM.
        image[6] = 0x03;     // vertical mirroring, battery-backed PRG-RAM.

        byte[] code = [0x4C, 0x00, 0x80];
        code.CopyTo(image, 16);
        for (int vector = 0; vector < 3; vector++)
        {
            image[16 + (32 * 1024) - 6 + (vector * 2)] = 0x00;
            image[16 + (32 * 1024) - 5 + (vector * 2)] = 0x80;
        }

        return image;
    }

    /// <summary>
    /// Writes one byte of video memory through the <c>$2006</c>/<c>$2007</c> registers. The
    /// host-side "write a memory byte" would do as well, but the host-side <b>read</b> is
    /// side-effect-free by REQ-CORE-081 and cannot drive the <c>$2007</c> buffer, so both ways
    /// go through the register path here.
    /// </summary>
    private static void WriteVideo(NesCore core, int address, byte value)
    {
        core.Ppu.Write(6, (byte)(address >> 8));
        core.Ppu.Write(6, (byte)(address & 0xFF));
        core.Ppu.Write(7, value);
    }

    /// <summary>
    /// Reads one byte of video memory. The <c>$2007</c> read buffer is one read behind, so the
    /// first read is discarded (REQ-CORE-040).
    /// </summary>
    private static byte ReadVideo(NesCore core, int address)
    {
        core.Ppu.Write(6, (byte)(address >> 8));
        core.Ppu.Write(6, (byte)(address & 0xFF));
        core.Ppu.Read(7);
        return core.Ppu.Read(7);
    }

    /// <summary>
    /// The label of <paramref name="key"/> in one of the two languages of AMD-035, read from
    /// that language's table so that the language in force is not changed. <c>UiStrings</c> and
    /// <c>UiLanguage</c> are internal to <c>Zax.App</c>, hence the reflection.
    /// </summary>
    private static string UiLabel(string language, string key)
    {
        System.Reflection.Assembly app = typeof(Zax.App.AppOptions).Assembly;
        Type strings = app.GetType("Zax.App.Ui.UiStrings", true)!;
        object value = Enum.Parse(app.GetType("Zax.App.Ui.UiLanguage", true)!, language);
        object table = strings.GetMethod(
            "TableOf",
            System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)!
            .Invoke(null, [value])!;
        return (string)table.GetType().GetProperty("Item")!.GetValue(table, [key])!;
    }

    /// <summary>
    /// Runs a ROM that uses blargg's shell protocol and presses RESET when it asks for it, as
    /// its <c>readme.txt</c> describes: <c>$6001-$6003</c> hold <c>DE B0 61</c>, <c>$6000</c>
    /// holds <c>$80</c> while the test runs, <c>$81</c> when it wants the reset button pressed
    /// at least 100 ms later, and the result code once it is done; 0 means passed. The text is a
    /// zero-terminated string at <c>$6004</c>.
    /// </summary>
    private static (int Code, string Text, long Frames, int Presses) RunResetRom(
        string path,
        bool soft,
        int maxFrames = 20_000)
    {
        NesCore core = Load(File.ReadAllBytes(path));
        long armed = -1;
        int presses = 0;
        bool waitingForRestart = false;

        for (long frame = 0; frame < maxFrames; frame++)
        {
            core.RunFrame(0);

            if (core.ReadMemory(0x6001) != 0xDE
                || core.ReadMemory(0x6002) != 0xB0
                || core.ReadMemory(0x6003) != 0x61)
            {
                continue;
            }

            byte status = core.ReadMemory(0x6000);
            if (status == 0x80)
            {
                // Running: a later $81 is a genuine new request.
                waitingForRestart = false;
                armed = -1;
                continue;
            }

            if (status == 0x81 && !waitingForRestart)
            {
                // At least 100 ms, which is six NTSC frames; ten is comfortable.
                if (armed < 0)
                {
                    armed = frame + 10;
                }
                else if (frame >= armed)
                {
                    if (soft)
                    {
                        core.SoftReset();
                    }
                    else
                    {
                        core.Reset();
                    }

                    presses++;
                    waitingForRestart = true;
                    armed = -1;
                }

                continue;
            }

            if (waitingForRestart)
            {
                // Between the press and the shell's own "$80", $6000 still holds the old byte.
                continue;
            }

            return (status, ShellText(core), frame, presses);
        }

        return (core.ReadMemory(0x6000), ShellText(core), maxFrames, presses);
    }

    /// <summary>The zero-terminated text a shell ROM leaves at <c>$6004</c>.</summary>
    private static string ShellText(NesCore core)
    {
        System.Text.StringBuilder text = new();
        for (int address = 0x6004; address < 0x6004 + 256; address++)
        {
            byte value = core.ReadMemory((ushort)address);
            if (value == 0)
            {
                break;
            }

            text.Append((char)value);
        }

        return text.ToString().Replace('\n', '|');
    }
}
