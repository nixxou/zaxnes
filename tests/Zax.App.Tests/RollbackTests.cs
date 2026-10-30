using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// The rollback history of SPEC-07 §3.4 and its edge cases, with the extension of AMD-042:
/// a snapshot also carries the 64 <c>PData</c> values and the camera state.
/// </summary>
public sealed class RollbackTests
{
    /// <summary>
    /// A snapshot whose core block is one recognisable byte, which is all the history's own
    /// bookkeeping needs. The <c>PData</c> block has the size SPEC-05 §4.6 fixes.
    /// </summary>
    private static RollbackSnapshot Snapshot(byte core)
        => new([core], new byte[Zax.Scripting.PersistentData.BlockSize], default);

    /// <summary>
    /// REQ-LOOP-030: the history keeps at most 30 snapshots and drops the oldest beyond that.
    /// </summary>
    [Fact]
    public void REQ_LOOP_030_the_history_keeps_thirty_snapshots()
    {
        RollbackHistory history = new();
        for (int index = 0; index < 40; index++)
        {
            history.Capture(Snapshot((byte)index));
        }

        Assert.Equal(RollbackHistory.Capacity, history.Count);

        // The newest two are 39 and 38; a rollback restores the second-to-last, that is 38.
        Assert.Equal([38], history.TakeRollbackTarget()!.Core);
        Assert.Equal(RollbackHistory.Capacity - 2, history.Count);
    }

    /// <summary>
    /// REQ-LOOP-030: a snapshot is only taken when the core frame counter is a multiple of 60.
    /// </summary>
    [Theory]
    [InlineData(0, true)]
    [InlineData(59, false)]
    [InlineData(60, true)]
    [InlineData(600, true)]
    [InlineData(601, false)]
    public void REQ_LOOP_030_snapshots_land_on_multiples_of_sixty(long frame, bool expected)
        => Assert.Equal(expected, RollbackHistory.IsSnapshotFrame(frame));

    /// <summary>
    /// REQ-LOOP-032, notes: a single snapshot is restored, and with none nothing is restored.
    /// </summary>
    [Fact]
    public void REQ_LOOP_032_the_edge_cases_are_fixed()
    {
        RollbackHistory history = new();
        Assert.Null(history.TakeRollbackTarget());

        history.Capture(Snapshot(7));
        Assert.Equal([7], history.TakeRollbackTarget()!.Core);
        Assert.True(history.IsEmpty);
    }

    /// <summary>
    /// REQ-LOOP-033: restoring entry <c>i</c> of the optional history panel keeps only the
    /// entries before it.
    /// </summary>
    [Fact]
    public void REQ_LOOP_033_the_history_panel_keeps_only_older_entries()
    {
        RollbackHistory history = new();
        for (int index = 0; index < 10; index++)
        {
            history.Capture(Snapshot((byte)index));
        }

        Assert.Equal([4], history.TakeAt(4)!.Core);
        Assert.Equal(4, history.Count);
        Assert.Null(history.TakeAt(9));
    }

    /// <summary>
    /// REQ-LOOP-032 and vector T6: after 100 emulated frames the history holds the snapshots of
    /// frames 0 and 60; a rollback restores frame 0, the history keeps nothing, and the
    /// snapshot is taken again on the next frame because the counter is a multiple of 60.
    /// </summary>
    [Fact]
    public void REQ_LOOP_032_a_rollback_restores_the_second_to_last_snapshot()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.RunFrames(100);
        Assert.Equal(100, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(2, harness.Host.Rollback.Count);

        harness.Run(ZaxCommands.Rollback);

        Assert.Equal(0, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal("Roll Back", harness.Host.Notifications.Current!.Value.Text);
        Assert.Equal(1.0, harness.Host.Notifications.Current!.Value.Seconds);

        // Frame 0 is a multiple of 60, so it is captured again straight away.
        harness.RunFrames(1);
        Assert.Equal(1, harness.Host.Rollback.Count);
    }

    /// <summary>
    /// REQ-LOOP-032 acceptance: 10.5 s of NTSC then a rollback puts the core back at frame 540,
    /// and a second rollback within the second at 480.
    /// </summary>
    [Fact]
    public void REQ_LOOP_032_each_press_goes_back_one_to_two_seconds()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.RunFrames(630);
        Assert.Equal(630, harness.Host.Session!.Core.FrameCount);

        harness.Run(ZaxCommands.Rollback);
        Assert.Equal(540, harness.Host.Session!.Core.FrameCount);

        // The restored snapshot sits on a multiple of 60, so it is captured again on the very
        // next frame (REQ-LOOP-032, last line); the next press therefore lands on 480.
        harness.RunFrames(1);
        harness.Run(ZaxCommands.Rollback);
        Assert.Equal(480, harness.Host.Session!.Core.FrameCount);
    }

    /// <summary>
    /// REQ-LOOP-031: the history is emptied by a launch, by a savestate load and by a reset, and
    /// <b>not</b> by a pause, a quick save or a rollback.
    /// </summary>
    [Fact]
    public void REQ_LOOP_031_only_a_launch_load_or_reset_clears_the_history()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.RunFrames(200);
        int snapshots = harness.Host.Rollback.Count;
        Assert.True(snapshots >= 3);

        harness.Run(ZaxCommands.Pause);
        Assert.Equal(snapshots, harness.Host.Rollback.Count);

        harness.Run(ZaxCommands.QuickSave);
        Assert.Equal(snapshots, harness.Host.Rollback.Count);

        harness.Run(ZaxCommands.Reset);
        Assert.Equal(0, harness.Host.Rollback.Count);
    }

    /// <summary>
    /// REQ-LOOP-031 acceptance: loading a savestate and asking for a rollback at once restores
    /// nothing — the history is empty — and the emulation carries on.
    /// </summary>
    [Fact]
    public void REQ_LOOP_031_a_rollback_right_after_a_load_restores_nothing()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.RunFrames(120);
        harness.Run(ZaxCommands.QuickSave);

        harness.RunFrames(120);
        harness.Run(ZaxCommands.QuickLoad);

        Assert.Equal(0, harness.Host.Rollback.Count);
        long frames = harness.Host.Session!.Core.FrameCount;

        harness.Run(ZaxCommands.Rollback);

        Assert.Equal(frames, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(EmulationState.Running, harness.Host.State);
    }

    /// <summary>
    /// AMD-042 REQ-LOOP-069 and REQ-LOOP-070, which complete DEC-07-Q3: a rollback also puts
    /// the camera back to the setpoints of the moment the snapshot was captured. The learned
    /// conversion state is still outside the snapshot.
    /// </summary>
    [Fact]
    public void REQ_LOOP_070_a_rollback_puts_the_camera_back()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        // The snapshot of frame 60 is taken with this distance in the setpoints.
        harness.Scene.Camera.Current.Distance = 123f;
        harness.RunFrames(120);

        harness.Scene.Camera.Current.Distance = 321f;
        harness.Scene.Camera.SnapToSetpoints();

        harness.Run(ZaxCommands.Rollback);

        Assert.Equal(123f, harness.Scene.Camera.Current.Distance, 1e-3f);
    }

    /// <summary>
    /// AMD-042 REQ-LOOP-069: a snapshot carries the 64 <c>PData</c> values and the camera state
    /// beside the core state, and nothing else — no learned conversion state, no frames.
    /// </summary>
    [Fact]
    public void REQ_LOOP_069_a_snapshot_carries_the_core_the_pdata_and_the_camera()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.Scene.Camera.SetMode(CameraMode.Fpv);
        harness.Scene.Camera.Current.Distance = 77f;
        harness.RunFrames(61);

        RollbackSnapshot snapshot = harness.Host.Rollback.TakeRollbackTarget()!;

        Assert.NotEmpty(snapshot.Core);
        Assert.Equal(Zax.Scripting.PersistentData.BlockSize, snapshot.PersistentData.Length);
        Assert.Equal(CameraMode.Fpv, snapshot.Camera.Mode);
        Assert.Equal(77f, snapshot.Camera.Fpv.Distance, 1e-3f);

        // The remaining transition time is a time, bounded to [0, 3] s (REQ-SCN-024 amended).
        Assert.InRange(snapshot.Camera.TransitionRemaining, 0f, SceneConstants.ModeTransitionSeconds);
    }

    /// <summary>
    /// AMD-042 REQ-LOOP-070 acceptance: a profile whose <c>Update</c> hook writes
    /// <c>PData[1] = frame</c>; 10.5 s of NTSC then a rollback puts the entry back to the value
    /// it had when the restored snapshot was captured — the one of counter 540 — instead of
    /// leaving it near 630. No <c>Save</c> and no <c>Load</c> hook runs.
    /// </summary>
    [Fact]
    public void REQ_LOOP_070_a_rollback_puts_pdata_back_without_the_hooks()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Scripts.Add(new ProfileScript("!prelude", true, "hooks = {}", string.Empty));
        profile.Scripts.Add(new ProfileScript(
            "counter",
            true,
            """
            function Update() SetPData(1, frame) end
            function Save() hooks[#hooks + 1] = 'Save' end
            function Load() hooks[#hooks + 1] = 'Load' end
            """,
            "*"));

        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter(), "pdata.nes");
        harness.Catalog.Add("pdata", rom, profile);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("pdata"));
        Assert.Equal(EmulationState.Running, harness.Host.State);

        harness.RunFrames(630);
        Assert.Equal(630, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(629.0, harness.Host.Session!.Scripts.ReadPersistentData(2), 0);

        harness.Run(ZaxCommands.Rollback);

        // REQ-LOOP-032 restores the snapshot of counter 540, taken before that frame ran, so
        // the last value Update had written was the one of frame 539.
        Assert.Equal(540, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(539.0, harness.Host.Session!.Scripts.ReadPersistentData(2), 0);

        // REQ-LOOP-070: the rollback never calls the savestate hooks.
        Assert.Equal(0.0, harness.Host.Session!.Scripts.EvaluateNumber("return #hooks"));
    }

    /// <summary>
    /// AMD-042 REQ-LOOP-071 acceptance: 10 s of play gives at least nine snapshots, and the end
    /// of the game empties the history at once — before any new launch.
    /// </summary>
    [Fact]
    public void REQ_LOOP_071_the_end_of_a_game_empties_the_history()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        harness.RunFrames(600);
        Assert.True(harness.Host.Rollback.Count >= 9, $"only {harness.Host.Rollback.Count} snapshots.");

        harness.Run(ZaxCommands.Stop);

        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.Equal(0, harness.Host.Rollback.Count);
    }

    /// <summary>SPEC-09 REQ-PERS-058: the history is never written to disk.</summary>
    [Fact]
    public void REQ_PERS_058_the_history_is_not_persisted()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.RunFrames(200);
        harness.Host.Shutdown();

        string[] files = Directory.GetFiles(harness.Root, "*", SearchOption.AllDirectories);
        Assert.DoesNotContain(files, file => Path.GetFileName(file).Contains("rollback", StringComparison.OrdinalIgnoreCase));
    }
}
