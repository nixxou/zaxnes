using Zax.App.Loop;
using Zax.App.Persistence;

namespace Zax.App.Tests;

/// <summary>
/// The state machine of SPEC-07 §3.1, the launch flow of §3.3 and the reset, stop, exit and
/// suspension of §3.6.
/// </summary>
public sealed class EmulationStateMachineTests
{
    /// <summary>
    /// REQ-LOOP-001 and REQ-LOOP-002: at start-up the state is Stopped, there is no current
    /// game, and the loading indicator is down.
    /// </summary>
    [Fact]
    public void REQ_LOOP_001_the_host_starts_stopped_with_no_current_game()
    {
        using LoopHarness harness = LoopHarness.Create();

        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.False(harness.Host.IsLoading);
        Assert.Null(harness.Host.CurrentGameId);
        Assert.Null(harness.Host.Session);
    }

    /// <summary>
    /// REQ-LOOP-021: launching a game raises the loading indicator, goes through Paused while
    /// the meshes finish, and only then reaches Running — with the core frame counter still at
    /// 0, so no emulated frame ran during the load.
    /// </summary>
    [Fact]
    public void REQ_LOOP_021_a_launch_runs_no_frame_before_running()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);

        harness.Host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        harness.Tick();

        Assert.True(harness.Host.IsLoading);

        List<EmulationState> seen = [];
        while (harness.Host.IsLoading)
        {
            harness.Tick(1.0 / 1000.0);
            seen.Add(harness.Host.State);
        }

        Assert.Contains(EmulationState.Paused, seen);
        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal("g1", harness.Host.CurrentGameId);
        Assert.Equal(0, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(0, harness.Host.EmulatedFrames);

        // REQ-PERS-033: the launch date is stamped on the entry.
        Assert.Equal([("g1", harness.Now.ToUnixTimeSeconds())], harness.Catalog.Played);
    }

    /// <summary>
    /// REQ-LOOP-004 and REQ-LOOP-005: Pause outside Running and Resume outside Paused do
    /// nothing; the Menu short press toggles the two.
    /// </summary>
    [Fact]
    public void REQ_LOOP_004_pause_and_resume_only_act_in_their_own_state()
    {
        using LoopHarness harness = LoopHarness.Create();

        // Stopped: both are inert.
        harness.Run(ZaxCommands.Resume);
        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        harness.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Stopped, harness.Host.State);

        Launch(harness, "g1");
        Assert.Equal(EmulationState.Running, harness.Host.State);

        harness.Run(ZaxCommands.PauseResume);
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        // A second Pause in Paused changes nothing and takes no new capture.
        harness.Thumbnail = [1, 2, 3];
        harness.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        harness.Run(ZaxCommands.PauseResume);
        Assert.Equal(EmulationState.Running, harness.Host.State);
    }

    /// <summary>
    /// REQ-LOOP-006: the capture of the game view is taken at the Running → Paused transition,
    /// before anything else is drawn.
    /// </summary>
    [Fact]
    public void REQ_LOOP_006_pausing_captures_the_game_view()
    {
        using LoopHarness harness = LoopHarness.Create();
        Launch(harness, "g1");

        byte[] capture = new byte[EmulationHost.ThumbnailWidth * EmulationHost.ThumbnailHeight * 3];
        capture[0] = 0x7F;
        harness.Thumbnail = capture;

        harness.Run(ZaxCommands.Pause);

        Assert.Equal(EmulationState.Paused, harness.Host.State);
        Assert.Equal(0x7F, harness.Host.PauseThumbnail[0]);
    }

    /// <summary>
    /// REQ-LOOP-007: commands sent while the loading indicator is up are deferred, not lost, and
    /// run in order once the load is over, on the post-load state.
    /// </summary>
    [Fact]
    public void REQ_LOOP_007_commands_are_deferred_during_a_load()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter());
        harness.Catalog.Add("g1", rom);

        harness.Host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of("g1"));
        harness.Tick();
        Assert.True(harness.Host.IsLoading);

        // A quick save asked for in the middle of the load.
        harness.Host.Enqueue(ZaxCommands.QuickSave);
        Assert.Equal(1, harness.Host.PendingCommands);

        harness.Tick(1.0 / 1000.0);
        Assert.Equal(1, harness.Host.PendingCommands);

        harness.Settle();
        Assert.False(harness.Host.IsLoading);

        harness.Tick(1.0 / 1000.0);
        Assert.Equal(0, harness.Host.PendingCommands);

        // The save happened, and it holds the post-load state.
        SaveSlotRecord quick = harness.Host.Slots.Find(SaveSlotIndex.QuickSlotId)!;
        Assert.True(quick.IsOccupied);
        Assert.Equal("g1", quick.GameId);
        Assert.Equal(EmulationState.Running, harness.Host.State);
    }

    /// <summary>
    /// REQ-LOOP-023: a file the core refuses shows <c>Invalid rom file.</c>, clears the current
    /// game and goes back to Stopped.
    /// </summary>
    [Fact]
    public void REQ_LOOP_023_an_invalid_rom_reports_the_exact_message()
    {
        using LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(System.Text.Encoding.ASCII.GetBytes("this is not a ROM"), "fake.nes");
        harness.Catalog.Add("bad", rom);

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("bad"));

        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.Null(harness.Host.CurrentGameId);
        Assert.False(harness.Host.IsLoading);
        Assert.Equal(RomFile.InvalidRomMessage, harness.Host.Notifications.Current!.Value.Text);
        Assert.Equal(NotificationSeverity.Error, harness.Host.Notifications.Current!.Value.Severity);
    }

    /// <summary>
    /// AMD-002 REQ-LOOP-018: opening a ROM from a running game validates the file before
    /// interrupting it. On failure the game carries on, with its RAM untouched.
    /// </summary>
    [Fact]
    public void REQ_LOOP_018_a_failed_open_keeps_the_running_game()
    {
        using LoopHarness harness = LoopHarness.Create();
        Launch(harness, "good");

        harness.RunFrames(30);
        long frames = harness.Host.Session!.Core.FrameCount;
        byte counter = harness.Host.Session.Core.ReadMemory(0x0010);
        Assert.True(frames > 0);

        string truncated = harness.WriteRom([0x4E, 0x45, 0x53, 0x1A, 0x02], "truncated.nes");
        harness.Catalog.Add("bad", truncated);

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("bad"));

        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal("good", harness.Host.CurrentGameId);
        Assert.Equal(frames, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(counter, harness.Host.Session.Core.ReadMemory(0x0010));
        Assert.Equal(RomFile.InvalidRomMessage, harness.Host.Notifications.Current!.Value.Text);
    }

    /// <summary>
    /// REQ-LOOP-050 with DEC-02-Q3: a reset is a full relaunch — the core cold-starts and its
    /// frame counter restarts from 0 — without reloading the 3D profile.
    /// </summary>
    [Fact]
    public void REQ_LOOP_050_a_reset_cold_starts_the_core()
    {
        using LoopHarness harness = LoopHarness.Create();
        Launch(harness, "g1");

        harness.RunFrames(40);
        Assert.True(harness.Host.Session!.Core.FrameCount > 0);
        object before = harness.Host.Session.Scripts;

        harness.Run(ZaxCommands.Reset);

        Assert.Equal(EmulationState.Running, harness.Host.State);
        Assert.Equal(0, harness.Host.Session!.Core.FrameCount);

        // The profile — and therefore the Lua state — was not reloaded (REQ-LOOP-021 note).
        Assert.Same(before, harness.Host.Session.Scripts);
    }

    /// <summary>
    /// REQ-LOOP-051: stopping runs the End hook, closes the ROM and goes back to Stopped with no
    /// current game; REQ-PERS-012 puts the speed setting back to 0.
    /// </summary>
    [Fact]
    public void REQ_LOOP_051_stopping_clears_the_current_game()
    {
        using LoopHarness harness = LoopHarness.Create();
        Launch(harness, "g1");

        harness.Settings.GameSpeed = 4;
        harness.Run(ZaxCommands.Stop);

        Assert.Equal(EmulationState.Stopped, harness.Host.State);
        Assert.Null(harness.Host.CurrentGameId);
        Assert.Null(harness.Host.Session);
        Assert.Equal(0, harness.Settings.GameSpeed);
    }

    /// <summary>
    /// REQ-LOOP-053: losing the focus must not pause, but an explicit suspension does and the
    /// resume goes back to Running only if it was Running.
    /// </summary>
    [Fact]
    public void REQ_LOOP_053_only_an_explicit_suspension_pauses()
    {
        using LoopHarness harness = LoopHarness.Create();
        Launch(harness, "g1");

        harness.Host.Suspend();
        Assert.Equal(EmulationState.Paused, harness.Host.State);

        harness.Host.ResumeFromSuspend();
        harness.Tick();
        Assert.Equal(EmulationState.Running, harness.Host.State);

        // Suspended while already paused: the resume must not start the game.
        harness.Run(ZaxCommands.Pause);
        harness.Host.Suspend();
        harness.Host.ResumeFromSuspend();
        Assert.Equal(EmulationState.Paused, harness.Host.State);
    }

    /// <summary>
    /// REQ-LOOP-052: the exit writes the settings and the slot metadata, stops the game and
    /// creates no savestate.
    /// </summary>
    [Fact]
    public void REQ_LOOP_052_the_exit_persists_settings_and_slots()
    {
        using LoopHarness harness = LoopHarness.Create();
        Launch(harness, "g1");

        harness.Settings.Volume = 0.25f;
        harness.Settings.AutoTvMode = true;

        SettingsStore store = new(harness.Paths);
        harness.Run(ZaxCommands.Quit);
        Assert.True(harness.Host.QuitRequested);

        harness.Host.Shutdown(store);

        Assert.True(File.Exists(harness.Paths.SettingsFile));
        Assert.True(File.Exists(harness.Paths.SaveIndexFile));
        Assert.False(File.Exists(harness.Paths.SlotStateFile(SaveSlotIndex.QuickSlotId)));

        AppSettings reread = new();
        new SettingsStore(harness.Paths).Load(reread, harness.Now);
        Assert.Equal(0.25f, reread.Volume, 1e-6f);
        Assert.True(reread.AutoTvMode);
    }

    /// <summary>
    /// REQ-LOOP-017 with DEC-03-Q01: with automatic region on, a PAL-marked ROM moves the
    /// setting to PAL and the cadence to 50; a region change in the middle of a game is applied
    /// to the core and to the cadence at once.
    /// </summary>
    [Fact]
    public void REQ_LOOP_017_automatic_region_follows_the_rom_header()
    {
        using LoopHarness harness = LoopHarness.Create();

        // Byte 9 bit 0 = 1 marks PAL in the iNES header.
        byte[] image = SyntheticRom.Counter();
        image[9] = 0x01;
        string rom = harness.WriteRom(image, "pal.nes");
        harness.Catalog.Add("pal", rom);
        harness.Settings.AutoTvMode = true;

        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("pal"));

        Assert.Equal(TvRegionSetting.Pal, harness.Settings.TvMode);
        Assert.Equal(Zax.Nes.NesRegion.Pal, harness.Host.Session!.Core.Region);
        Assert.Equal(50.0, harness.Host.Pacer.NominalRate);

        harness.Settings.TvMode = TvRegionSetting.Ntsc;
        Assert.Equal(Zax.Nes.NesRegion.Ntsc, harness.Host.Session!.Core.Region);
        harness.Tick();
        Assert.Equal(60.0, harness.Host.Pacer.NominalRate);
    }

    /// <summary>
    /// REQ-LOOP-024: changing the mesh algorithm during a session raises the loading
    /// indicator, regenerates, resets the frame clock and leaves both the state and the core
    /// untouched.
    /// </summary>
    [Fact]
    public void REQ_LOOP_024_a_mesh_algorithm_change_keeps_the_state_and_the_core()
    {
        using LoopHarness harness = LoopHarness.Create();
        Launch(harness, "g1");
        harness.RunFrames(60);

        harness.Run(ZaxCommands.Pause);
        long frames = harness.Host.Session!.Core.FrameCount;

        harness.Host.ChangeMeshAlgorithm(Zax.Engine3D.RenderAlgorithm.Marching);
        Assert.True(harness.Host.IsLoading);

        harness.Settle();

        Assert.False(harness.Host.IsLoading);
        Assert.Equal(EmulationState.Paused, harness.Host.State);
        Assert.Equal(frames, harness.Host.Session!.Core.FrameCount);
    }

    internal static void Launch(LoopHarness harness, string gameId)
    {
        string rom = harness.WriteRom(SyntheticRom.Counter(), gameId + ".nes");
        harness.Catalog.Add(gameId, rom);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of(gameId));
        Assert.Equal(EmulationState.Running, harness.Host.State);
    }
}
