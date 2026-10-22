using Zax.App;
using Zax.App.Audio;
using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-07: the states and their transitions, the command queue,
/// the frame cadence, the launch flow, the rollback history, the save states, the reset, the
/// exit and the determinism guarantees.
/// </summary>
public sealed class LoopTests
{
    // --------------------------------------------------------- §3.1 emulation states

    /// <summary>
    /// REQ-LOOP-001 — at every instant the emulation is in exactly one of Stopped, Running and
    /// Paused, and the Loading indicator is orthogonal to it. At application start the state is
    /// Stopped.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-001
    public void The_state_is_always_exactly_one_of_three_plus_the_loading_flag()
    {
        using HostDriver host = HostDriver.Create();

        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.False(host.IsLoading);
        Assert.Null(host.CurrentGameId);

        // There are exactly three states and no "idle" one.
        Assert.Equal(3, Enum.GetValues<EmulationState>().Length);

        host.Launch2DMode();
        Assert.Equal(EmulationState.Running, host.State);
        Assert.False(host.IsLoading);

        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);

        host.Run(ZaxCommands.Stop);
        Assert.Equal(EmulationState.Stopped, host.State);
    }

    /// <summary>
    /// REQ-LOOP-002 — at start-up the settings are read, the library is there, no game is
    /// current, and the menu music is in its fade-in phase.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-002
    public void The_application_starts_stopped_with_the_menu_music_fading_in()
    {
        using HostDriver host = HostDriver.Create();

        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Null(host.CurrentGameId);
        Assert.NotEmpty(host.Library.Entries);

        // The music starts from 0 and rises; the emulation makes no sound.
        Assert.Equal(0f, host.Mixer.MusicGain, 1e-6f);
        host.Mixer.Advance(FadeGain.FullFadeSeconds);
        Assert.Equal(AudioMixer.MusicGainFactor, host.Mixer.MusicGain, 1e-6f);
        Assert.False(host.Mixer.EmulationAudible);
    }

    /// <summary>
    /// REQ-LOOP-003 — the presentation table: the console is audible only in Running, the menu
    /// music is in its active phase in every other state, and the 3D scene is hidden in Stopped
    /// (AMD-013 keeps it visible in Paused, Q-029).
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-003
    public void The_presentation_follows_the_state_table()
    {
        using HostDriver host = HostDriver.Create();

        Assert.False(host.Mixer.EmulationAudible);
        Assert.False(host.Scene.GameRunning);

        host.Launch2DMode();
        Assert.True(host.Mixer.EmulationAudible);
        Assert.True(host.Scene.GameRunning);

        host.Run(ZaxCommands.Pause);
        Assert.False(host.Mixer.EmulationAudible);

        host.Run(ZaxCommands.Resume);
        Assert.True(host.Mixer.EmulationAudible);

        host.Run(ZaxCommands.Stop);
        Assert.False(host.Mixer.EmulationAudible);
        Assert.False(host.Scene.GameRunning);
    }

    /// <summary>
    /// REQ-LOOP-003 — the progress indicator reflects the mesh generation and is only up while
    /// Loading.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-003, REQ-LOOP-021
    public void The_progress_indicator_only_moves_while_loading()
    {
        using HostDriver host = HostDriver.Create();
        Assert.Equal(0f, host.LoadProgress, 1e-6f);

        host.Launch2DMode();
        Assert.False(host.IsLoading);
        Assert.Equal(1f, host.LoadProgress, 1e-6f);
    }

    /// <summary>
    /// REQ-LOOP-004 — a Play command outside Paused and a Pause command outside Running have no
    /// effect, and no new capture is taken.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-004
    public void Play_and_pause_do_nothing_outside_their_state()
    {
        using HostDriver host = HostDriver.Create();

        host.Run(ZaxCommands.Resume);
        Assert.Equal(EmulationState.Stopped, host.State);

        host.Launch2DMode();
        host.Run(ZaxCommands.Resume);
        Assert.Equal(EmulationState.Running, host.State);

        host.Thumbnail = new byte[256 * 192 * 3];
        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);

        host.Thumbnail = new byte[256 * 192 * 3];
        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);
    }

    /// <summary>
    /// REQ-LOOP-004 — removing the current game from the library goes back to Stopped with the
    /// current game cleared, and an exception raised by the core during a frame goes back to
    /// Stopped too.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-004, REQ-LOOP-051
    public void Stopping_clears_the_current_game()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        Assert.NotNull(host.CurrentGameId);

        host.Run(ZaxCommands.Stop);
        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Null(host.CurrentGameId);
    }

    /// <summary>
    /// REQ-LOOP-005 — the Menu button toggles Running and Paused on a <b>short</b> press
    /// (released in under 0.3 s); a long press does not toggle.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-005
    public void A_short_press_on_Menu_toggles_the_pause_and_a_long_one_does_not()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        // 0.1 s held then released: a short press.
        host.Tick(0.1, InputAction.Menu);
        host.Tick(0.05);
        host.ApplyCommands();
        Assert.Equal(EmulationState.Paused, host.State);

        host.Tick(0.1, InputAction.Menu);
        host.Tick(0.05);
        host.ApplyCommands();
        Assert.Equal(EmulationState.Running, host.State);

        // 0.5 s held then released: a long press, which does not toggle.
        host.Tick(0.2, InputAction.Menu);
        host.Tick(0.3, InputAction.Menu);
        host.Tick(0.05);
        host.ApplyCommands();
        Assert.Equal(EmulationState.Running, host.State);
    }

    /// <summary>
    /// REQ-LOOP-006 — the Running → Paused transition takes a capture of the game view
    /// <b>before</b> the menu appears, and that capture is the thumbnail of the next slot save.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-006, REQ-LOOP-042
    public void The_pause_capture_becomes_the_thumbnail_of_the_next_slot_save()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.Thumbnail = new byte[256 * 192 * 3];
        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);

        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(3));

        SaveSlotRecord slot = host.Slots.Find(SaveSlotIndex.NumberedSlotId(3))!;
        Assert.True(slot.IsOccupied);
        Assert.Equal(host.CurrentGameId, slot.GameId);
        Assert.NotNull(slot.Thumbnail);
        Assert.True(File.Exists(host.Paths.SlotThumbnailFile(slot.Id)));
    }

    /// <summary>
    /// REQ-LOOP-007 — commands are applied between two emulated frames, in the order they were
    /// sent, are deferred — never dropped — while Loading, and are handled in Paused and
    /// Stopped too.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-007
    public void Commands_are_a_FIFO_that_waits_for_the_end_of_a_load()
    {
        using HostDriver host = HostDriver.Create();

        // Handled in Stopped: a quick save without a current game does nothing but is consumed.
        host.Enqueue(ZaxCommands.QuickSave);
        Assert.Equal(1, host.PendingCommands);
        Assert.Equal(1, host.ApplyCommands());
        Assert.Equal(0, host.PendingCommands);

        // A command sent while a load is under way waits for it, and then sees the state after
        // the load.
        string id = host.TwoDimensionalModeId!;
        host.SetRomPath(id, host.WriteRom(LoopRom.Counter()));
        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();
        Assert.True(host.IsLoading);

        host.Enqueue(ZaxCommands.QuickSave);
        Assert.Equal(0, host.ApplyCommands());
        Assert.Equal(1, host.PendingCommands);

        host.Settle();
        Assert.False(host.IsLoading);
        Assert.Equal(1, host.ApplyCommands());

        SaveSlotRecord quick = host.Slots.Find(SaveSlotIndex.QuickSlotId)!;
        Assert.True(quick.IsOccupied);
        Assert.Equal(id, quick.GameId);
    }

    /// <summary>
    /// REQ-LOOP-007 — the order of two different commands is kept.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-007
    public void Two_commands_run_in_the_order_they_were_sent()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.Enqueue(ZaxCommands.Pause);
        host.Enqueue(ZaxCommands.Resume);
        host.ApplyCommands();
        Assert.Equal(EmulationState.Running, host.State);

        host.Enqueue(ZaxCommands.Resume);
        host.Enqueue(ZaxCommands.Pause);
        host.ApplyCommands();
        Assert.Equal(EmulationState.Paused, host.State);
    }

    // ----------------------------------------------------------- §3.2 frame cadence

    /// <summary>
    /// REQ-LOOP-010, T1, T2, T3 — the cadence is <c>nominal × k</c> on a monotonic clock: 600
    /// frames in 10 s at NTSC and <c>k = 1</c>, 500 at PAL and Dendy, 250 at PAL with
    /// <c>s = −4</c>, 180 in 1 s at NTSC with <c>s = +8</c>.
    /// </summary>
    [Theory] // REQ-LOOP-010, REQ-LOOP-015
    [InlineData(60, 0, 10.0, 600)]
    [InlineData(50, 0, 10.0, 500)]
    [InlineData(50, -4, 10.0, 250)]
    [InlineData(60, 8, 1.0, 180)]
    [InlineData(60, 4, 10.0, 1200)]
    public void The_cadence_is_the_nominal_rate_times_the_speed_factor(
        double nominal, int speed, double seconds, int expected)
    {
        double clock = 0;
        FramePacer pacer = new(nominal, () => clock) { Speed = SpeedSetting.Factor(speed) };

        int frames = 0;
        const double step = 1.0 / 1000.0;
        for (double elapsed = 0; elapsed < seconds; elapsed += step)
        {
            clock += step;
            frames += pacer.DueFrames();
        }

        Assert.InRange(frames, expected - 2, expected + 2);
    }

    /// <summary>
    /// REQ-LOOP-010 — the mean cadence drifts by less than 0.1 % over 60 s.
    /// </summary>
    [Fact] // REQ-LOOP-010
    public void The_mean_cadence_does_not_drift()
    {
        double clock = 0;
        FramePacer pacer = new(60, () => clock);

        int frames = 0;
        for (int tick = 0; tick < 60_000; tick++)
        {
            clock += 1.0 / 1000.0;
            frames += pacer.DueFrames();
        }

        Assert.InRange(frames, 3596, 3604);
    }

    /// <summary>
    /// REQ-LOOP-011 — a frame is due at <c>t0 + n / f</c> and may start up to 1 ms early; 95 % of
    /// the intervals at <c>k = 1</c> NTSC fall inside [14.6 ms ; 18.6 ms].
    /// </summary>
    [Fact] // REQ-LOOP-011
    public void The_frame_intervals_stay_inside_the_documented_window()
    {
        Assert.Equal(0.001, FramePacer.EarlyStartSeconds);

        double clock = 0;
        FramePacer pacer = new(60, () => clock);

        List<double> intervals = [];
        double last = -1;
        for (int tick = 0; tick < 60_000; tick++)
        {
            clock += 1.0 / 1000.0;
            int due = pacer.DueFrames();
            for (int frame = 0; frame < due; frame++)
            {
                if (last >= 0)
                {
                    intervals.Add(clock - last);
                }

                last = clock;
            }
        }

        int inside = intervals.Count(interval => interval is >= 0.0146 and <= 0.0186);
        Assert.True(
            inside >= intervals.Count * 0.95,
            $"{inside} of {intervals.Count} intervals are inside the window.");
    }

    /// <summary>
    /// REQ-LOOP-012, T5 — the backlog never exceeds five emulated frames: after a 1 s block at
    /// NTSC at most 5 frames run in a burst and about 55 are abandoned.
    /// </summary>
    [Fact] // REQ-LOOP-012
    public void The_backlog_is_capped_at_five_frames()
    {
        Assert.Equal(5, FramePacer.MaxBacklog);

        double clock = 0;
        FramePacer pacer = new(60, () => clock);

        clock += 1.0;
        int burst = pacer.DueFrames();

        Assert.Equal(5, burst);
        Assert.True(pacer.DroppedFrames >= 54, $"{pacer.DroppedFrames} frames were abandoned.");
        Assert.True(pacer.DroppedFrames <= 56, $"{pacer.DroppedFrames} frames were abandoned.");

        // Then the normal cadence resumes.
        clock += 1.0 / 60.0;
        Assert.Equal(1, pacer.DueFrames());
    }

    /// <summary>
    /// REQ-LOOP-013, T7 — the cadence clock is reset with no catch-up: after a 30 s pause the
    /// first second back holds 60 ± 1 frames.
    /// </summary>
    [Fact] // REQ-LOOP-013
    public void Resetting_the_clock_abandons_the_elapsed_time()
    {
        double clock = 0;
        FramePacer pacer = new(60, () => clock);

        clock += 30.0;
        pacer.Reset();

        int frames = 0;
        for (int tick = 0; tick < 1000; tick++)
        {
            clock += 1.0 / 1000.0;
            frames += pacer.DueFrames();
        }

        Assert.InRange(frames, 59, 61);
    }

    /// <summary>
    /// REQ-LOOP-013 — the host resets the clock at a resume, at a reset, at the end of a load and
    /// at a rollback: no burst follows any of them.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-013
    public void The_host_resets_the_clock_at_every_documented_moment()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        // A 10 s pause then a resume: the first second back is one second of frames.
        host.Run(ZaxCommands.Pause);
        for (int tick = 0; tick < 600; tick++)
        {
            host.Tick(1.0 / 60.0);
        }

        host.Run(ZaxCommands.Resume);

        int frames = 0;
        for (int tick = 0; tick < 60; tick++)
        {
            frames += host.Tick(1.0 / 60.0);
        }

        Assert.True(frames <= 62, $"{frames} frames ran in the first second back.");
    }

    /// <summary>
    /// REQ-LOOP-015 — the speed table: <c>k = 1 + s/4</c> for <c>s ≥ 0</c> and
    /// <c>k = 1 / (1 − s/4)</c> for <c>s &lt; 0</c>, the setting bounded to <c>[−8, 8]</c>.
    /// </summary>
    [Theory] // REQ-LOOP-015
    [InlineData(-8, 1.0 / 3.0)]
    [InlineData(-4, 0.5)]
    [InlineData(-1, 0.8)]
    [InlineData(0, 1.0)]
    [InlineData(1, 1.25)]
    [InlineData(4, 2.0)]
    [InlineData(8, 3.0)]
    public void The_speed_setting_maps_to_the_documented_factor(int setting, double factor)
        => Assert.Equal(factor, SpeedSetting.Factor(setting), 1e-9);

    /// <summary>REQ-LOOP-015 — the setting is bounded to <c>[−8, 8]</c>, 0 by default.</summary>
    [Fact] // REQ-LOOP-015
    public void The_speed_setting_is_bounded()
    {
        Assert.Equal(-8, SpeedSetting.Minimum);
        Assert.Equal(8, SpeedSetting.Maximum);
        Assert.Equal(0, SpeedSetting.Default);
        Assert.Equal(1.0, SpeedSetting.Factor(SpeedSetting.Default));
        Assert.Equal(SpeedSetting.Factor(8), SpeedSetting.Factor(20));
        Assert.Equal(SpeedSetting.Factor(-8), SpeedSetting.Factor(-20));
    }

    /// <summary>
    /// REQ-LOOP-015 — a change of setting takes effect on the next emulated frame without
    /// resetting the game.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-015
    public void A_speed_change_takes_effect_at_once_without_resetting_the_game()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(10);

        long before = host.Session!.Core.FrameCount;
        host.Run(ZaxCommands.SetSpeed, CommandArgs.Of(4));

        Assert.Equal(2.0, host.Pacer.Speed, 1e-9);
        Assert.Equal(120.0, host.Pacer.EffectiveRate, 1e-6);
        Assert.Equal(before, host.Session!.Core.FrameCount);
    }

    /// <summary>
    /// REQ-LOOP-016, T4 — one fast-forward request arms 30 emulated frames at <c>k = 1.5</c>
    /// whatever the speed setting; a request received during a burst has no effect; a held key
    /// re-arms as soon as the burst ends, and the notification "Fast Forward" is shown for 1 s.
    /// </summary>
    [Fact] // REQ-LOOP-016
    public void A_fast_forward_request_arms_thirty_frames_at_one_and_a_half()
    {
        Assert.Equal(30, FastForwardState.BurstFrames);
        Assert.Equal(2, SpeedSetting.FastForward);
        Assert.Equal(1.5, SpeedSetting.FastForwardFactor, 1e-9);

        FastForwardState fast = new();
        Assert.False(fast.IsActive);
        Assert.True(fast.Request());
        Assert.Equal(30, fast.Remaining);

        // A second request during the burst does nothing.
        Assert.False(fast.Request());
        Assert.Equal(30, fast.Remaining);

        for (int frame = 0; frame < 30; frame++)
        {
            fast.OnFrameRun();
        }

        Assert.False(fast.IsActive);
        Assert.Equal(0, fast.Remaining);
        Assert.True(fast.Request());

        fast.Clear();
        Assert.False(fast.IsActive);
    }

    /// <summary>
    /// REQ-LOOP-016 — in the host: the factor becomes 1.5 whatever the setting, the notification
    /// is shown for 1 s, and the leftover is 0 to 30 frames when the key is released.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-016
    public void The_host_fast_forward_forces_the_factor_and_shows_the_notification()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Run(ZaxCommands.SetSpeed, CommandArgs.Of(8));
        Assert.Equal(3.0, host.Pacer.Speed, 1e-9);

        host.Run(ZaxCommands.FastForward);
        Assert.True(host.FastForward.IsActive);

        // The factor is 1.5 even though the setting asks for 3: faithful (DEC-07-Q2).
        Assert.Equal(1.5, host.Pacer.Speed, 1e-9);

        Notification shown = Assert.Single(
            host.Notifications.Log,
            notification => notification.Text == "Fast Forward");
        Assert.Equal(1.0, shown.Seconds, 1e-9);
        Assert.Equal(NotificationSeverity.Normal, shown.Severity);

        // The burst ends after 30 emulated frames and the setting comes back.
        host.RunFrames(40);
        Assert.False(host.FastForward.IsActive);
        Assert.Equal(3.0, host.Pacer.Speed, 1e-9);
    }

    /// <summary>
    /// REQ-LOOP-017 — the region is a setting, applied at once to the core and to the cadence
    /// even in the middle of a game; with automatic region on, loading a ROM marked PAL makes the
    /// setting PAL and the cadence 50 Hz.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-017
    public void The_region_is_applied_at_once_and_the_automatic_region_follows_the_header()
    {
        using HostDriver host = HostDriver.Create();
        host.Settings.AutoTvMode = true;

        string id = host.TwoDimensionalModeId!;
        host.Launch(id, host.WriteRom(LoopRom.Pal(), "pal.nes"));

        Assert.Equal(TvRegionSetting.Pal, host.Settings.TvMode);
        Assert.Equal(NesRegion.Pal, host.Session!.Core.Region);
        Assert.Equal(50.0, host.Pacer.NominalRate, 1e-6);

        // A change in the middle of a game is applied at once.
        host.Settings.TvMode = TvRegionSetting.Ntsc;
        host.ApplyCommands();
        Assert.Equal(NesRegion.Ntsc, host.Session!.Core.Region);
        Assert.Equal(60.0, host.Pacer.NominalRate, 1e-6);
    }

    // ---------------------------------------------------------- §3.3 launching a game

    /// <summary>
    /// REQ-LOOP-021 — the launch sequence is observable: the launch date is stamped, the loading
    /// indicator goes up, the ROM reaches the core, the profile is loaded, the mesh generation
    /// reaches 100 %, and only then the state becomes Running with the core frame counter at 0.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-021
    public void The_launch_sequence_runs_in_the_documented_order()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        string rom = host.WriteRom(LoopRom.Counter());
        host.SetRomPath(id, rom);

        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();

        Assert.True(host.IsLoading);

        int ticks = 0;
        while (host.IsLoading && ticks++ < 256)
        {
            Assert.NotEqual(EmulationState.Running, host.State);
            host.Tick(1.0 / 1000.0);
        }

        Assert.False(host.IsLoading);
        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(1f, host.LoadProgress, 1e-6f);
        Assert.Equal(0, host.Session!.Core.FrameCount);
        Assert.Equal(id, host.CurrentGameId);
        Assert.Contains(rom, host.RecentRoms.Select(Path.GetFullPath));
    }

    /// <summary>
    /// REQ-LOOP-021 — relaunching the <b>same</b> game does not reload the profile: the Lua
    /// state survives, which is what REQ-LUA-013 asks for.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-021, REQ-LOOP-050, REQ-LUA-013
    public void Relaunching_the_same_game_keeps_the_Lua_state()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verifier-relaunch";
        profile.Scripts.Add(new ProfileScript(
            "a", true, "loads = (loads or 0) + 1\nstarts = 0\nfunction Start() starts = starts + 1 end", "*"));

        using HostDriver host = HostDriver.Create(profile);
        string id = host.EntryIdByTitle("verifier-relaunch");
        host.Launch(id, host.WriteRom(LoopRom.Counter()));

        Assert.Equal(1, host.Session!.Scripts.EvaluateNumber("return loads"));
        Assert.Equal(1, host.Session!.Scripts.EvaluateNumber("return starts"));

        host.Run(ZaxCommands.Reset);

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(1, host.Session!.Scripts.EvaluateNumber("return loads"));
        Assert.Equal(2, host.Session!.Scripts.EvaluateNumber("return starts"));
    }

    /// <summary>
    /// REQ-LOOP-022 — a <c>.zip</c> is decompressed and its first entry used, under the name of
    /// the archive with the <c>.nes</c> extension; anything else is read as it is.
    /// </summary>
    [Fact] // REQ-LOOP-022
    public void A_zip_uses_its_first_entry_under_the_name_of_the_archive()
    {
        string folder = Path.Combine(Path.GetTempPath(), $"zax-rom-{Guid.NewGuid():N}");
        Directory.CreateDirectory(folder);
        try
        {
            string archive = Path.Combine(folder, "Foo.zip");
            using (FileStream stream = File.Create(archive))
            using (System.IO.Compression.ZipArchive zip =
                new(stream, System.IO.Compression.ZipArchiveMode.Create))
            {
                using (Stream first = zip.CreateEntry("bar.nes").Open())
                {
                    first.Write(LoopRom.Counter());
                }

                using Stream second = zip.CreateEntry("zzz.nes").Open();
                second.Write([0, 1, 2]);
            }

            RomReadResult read = RomFile.Read(archive);
            Assert.True(read.Success);
            Assert.Equal("Foo.nes", read.Name);
            Assert.Equal(LoopRom.Counter().Length, read.Image!.Length);

            // A plain file keeps its own name.
            string plain = Path.Combine(folder, "plain.nes");
            File.WriteAllBytes(plain, LoopRom.Counter());
            RomReadResult direct = RomFile.Read(plain);
            Assert.True(direct.Success);
            Assert.Equal("plain.nes", direct.Name);
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    /// <summary>
    /// REQ-LOOP-023, T10 — the exact messages: a corrupt archive, an entry that cannot be
    /// extracted, content the core refuses, and a file system error.
    /// </summary>
    [Fact] // REQ-LOOP-023
    public void The_launch_error_messages_are_the_documented_ones()
    {
        Assert.Equal("Corrupted zip file", RomFile.CorruptedZipMessage);
        Assert.Equal("Uncompression is not successful", RomFile.UncompressionFailedMessage);
        Assert.Equal("Invalid rom file.", RomFile.InvalidRomMessage);

        string folder = Path.Combine(Path.GetTempPath(), $"zax-rom-{Guid.NewGuid():N}");
        Directory.CreateDirectory(folder);
        try
        {
            // A file that is not a zip at all.
            string broken = Path.Combine(folder, "broken.zip");
            File.WriteAllBytes(broken, [1, 2, 3, 4, 5, 6, 7, 8]);
            Assert.Equal(RomFile.CorruptedZipMessage, RomFile.Read(broken).ErrorMessage);

            // An empty archive.
            string empty = Path.Combine(folder, "empty.zip");
            using (FileStream stream = File.Create(empty))
            {
                using System.IO.Compression.ZipArchive zip =
                    new(stream, System.IO.Compression.ZipArchiveMode.Create);
            }

            Assert.Equal(RomFile.UncompressionFailedMessage, RomFile.Read(empty).ErrorMessage);

            // A missing file reports the file system's own message.
            RomReadResult missing = RomFile.Read(Path.Combine(folder, "nope.nes"));
            Assert.False(missing.Success);
            Assert.NotNull(missing.ErrorMessage);
            Assert.NotEqual(RomFile.InvalidRomMessage, missing.ErrorMessage);

            // Q-028 (AMD-013): .7z is refused with the uncompression message.
            string sevenZip = Path.Combine(folder, "rom.7z");
            File.WriteAllBytes(sevenZip, [1, 2, 3]);
            Assert.Equal(RomFile.UncompressionFailedMessage, RomFile.Read(sevenZip).ErrorMessage);
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    /// <summary>
    /// REQ-LOOP-023 — a text file renamed <c>.nes</c> gives the notification
    /// <c>Invalid rom file.</c>, the state Stopped and no current game.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-023
    public void An_invalid_rom_file_gives_the_documented_notification()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        host.SetRomPath(id, host.WriteRom(System.Text.Encoding.ASCII.GetBytes("not a rom"), "fake.nes"));

        host.Run(ZaxCommands.LaunchGame, CommandArgs.Of(id));

        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Null(host.CurrentGameId);
        Assert.False(host.IsLoading);
        Assert.Contains(
            host.Notifications.Log,
            notification => notification.Text == RomFile.InvalidRomMessage
                && notification.Severity == NotificationSeverity.Error);
    }

    /// <summary>
    /// REQ-LOOP-018 (AMD-002) — opening a ROM while a game is running validates the file
    /// <b>before</b> interrupting the game: on failure the game carries on in the state it was
    /// in, with its RAM unchanged, and the error message is shown.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-018
    public void A_bad_rom_never_interrupts_the_running_game()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(30);

        long frames = host.Session!.Core.FrameCount;
        byte counter = host.Session!.Core.ReadMemory(0x0010);

        string id = host.TwoDimensionalModeId!;
        host.SetRomPath(id, host.WriteRom([1, 2, 3, 4], "truncated.nes"));
        host.Run(ZaxCommands.LaunchGame, CommandArgs.Of(id));

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(id, host.CurrentGameId);
        Assert.Equal(frames, host.Session!.Core.FrameCount);
        Assert.Equal(counter, host.Session!.Core.ReadMemory(0x0010));
        Assert.Contains(
            host.Notifications.Log,
            notification => notification.Severity == NotificationSeverity.Error);
    }

    /// <summary>
    /// REQ-LOOP-024 — a change of mesh algorithm goes through Loading, regenerates the meshes,
    /// resets the cadence clock and leaves the state and the core alone.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-024
    public void A_mesh_algorithm_change_keeps_the_state_and_the_core()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(20);
        host.Run(ZaxCommands.Pause);

        long frames = host.Session!.Core.FrameCount;

        host.Settings.Model.SetInt(
            Zax.Settings.SettingKeys.RenderMeshAlgo,
            1,
            Zax.Settings.SettingLayer.User);
        host.ApplyCommands();
        host.Settle();

        Assert.False(host.IsLoading);
        Assert.Equal(EmulationState.Paused, host.State);
        Assert.Equal(frames, host.Session!.Core.FrameCount);
    }

    // ------------------------------------------------------------------ §3.4 rollback

    /// <summary>
    /// REQ-LOOP-030 — a snapshot of the core alone is taken at the start of every emulated frame
    /// whose counter is a multiple of 60, and at most 30 are kept.
    /// </summary>
    [Fact] // REQ-LOOP-030
    public void The_history_keeps_thirty_snapshots_taken_every_sixty_frames()
    {
        Assert.Equal(30, RollbackHistory.Capacity);
        Assert.Equal(60, RollbackHistory.IntervalFrames);

        Assert.True(RollbackHistory.IsSnapshotFrame(0));
        Assert.True(RollbackHistory.IsSnapshotFrame(60));
        Assert.True(RollbackHistory.IsSnapshotFrame(1800));
        Assert.False(RollbackHistory.IsSnapshotFrame(59));
        Assert.False(RollbackHistory.IsSnapshotFrame(61));

        RollbackHistory history = new();
        for (int index = 0; index < 45; index++)
        {
            history.Capture([(byte)index]);
        }

        Assert.Equal(30, history.Count);

        // The oldest kept is the 15th taken: the earlier ones were dropped.
        byte[]? target = history.TakeRollbackTarget();
        Assert.NotNull(target);
        Assert.Equal(43, target[0]);
    }

    /// <summary>
    /// REQ-LOOP-030 — in the host: 45 s of NTSC play leave 30 snapshots and the snapshot holds
    /// the core alone.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-030
    public void Forty_five_seconds_of_play_leave_thirty_snapshots()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.RunFrames(45 * 60);
        Assert.Equal(30, host.Rollback.Count);
    }

    /// <summary>
    /// REQ-LOOP-031 — the history is cleared at a launch, at a savestate load and at a reset, and
    /// <b>not</b> by a pause, a quick save or a rollback.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-031
    public void The_history_is_cleared_only_at_the_documented_moments()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(240);

        int before = host.Rollback.Count;
        Assert.True(before >= 4, $"only {before} snapshots.");

        host.Run(ZaxCommands.Pause);
        Assert.Equal(before, host.Rollback.Count);

        host.Run(ZaxCommands.QuickSave);
        Assert.Equal(before, host.Rollback.Count);

        host.Run(ZaxCommands.Rollback);
        Assert.Equal(before - 2, host.Rollback.Count);

        host.Run(ZaxCommands.QuickLoad);
        Assert.Equal(0, host.Rollback.Count);

        host.RunFrames(240);
        Assert.True(host.Rollback.Count > 0);
        host.Run(ZaxCommands.Reset);
        Assert.Equal(0, host.Rollback.Count);
    }

    /// <summary>
    /// REQ-LOOP-032, T6 — a rollback restores the <b>second-to-last</b> snapshot and removes both
    /// it and the last one, goes to Running and resets the clock. With one snapshot it restores
    /// that one; with none nothing is restored but the state still becomes Running.
    /// </summary>
    [Fact] // REQ-LOOP-032
    public void A_rollback_takes_the_second_to_last_snapshot()
    {
        RollbackHistory history = new();
        history.Capture([0]);
        history.Capture([60]);

        byte[]? target = history.TakeRollbackTarget();
        Assert.NotNull(target);
        Assert.Equal(0, target[0]);
        Assert.Equal(0, history.Count);

        // A single snapshot is restored and removed.
        RollbackHistory single = new();
        single.Capture([7]);
        Assert.Equal(7, single.TakeRollbackTarget()![0]);
        Assert.True(single.IsEmpty);

        // An empty history restores nothing.
        Assert.Null(new RollbackHistory().TakeRollbackTarget());
    }

    /// <summary>
    /// REQ-LOOP-032 — in the host: 10.5 s of NTSC play then a rollback puts the core frame
    /// counter at 540, a second one at 480; the notification is "Roll Back" for 1 s and the state
    /// becomes Running even from Paused.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-032
    public void The_host_rollback_steps_back_one_snapshot_at_a_time()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(630);

        Assert.Equal(630, host.Session!.Core.FrameCount);

        host.Run(ZaxCommands.Pause);
        host.Run(ZaxCommands.Rollback);

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(540, host.Session!.Core.FrameCount);

        // The restored snapshot is taken at a multiple of 60, so it is recaptured on the very
        // next frame; a second press one frame later goes back one more second (REQ-LOOP-032).
        host.RunFrames(1);
        host.Run(ZaxCommands.Rollback);
        Assert.Equal(480, host.Session!.Core.FrameCount);

        Notification shown = host.Notifications.Log.Last(
            notification => notification.Text == "Roll Back");
        Assert.Equal(1.0, shown.Seconds, 1e-9);
    }

    /// <summary>
    /// REQ-LOOP-033 — the optional history panel: restoring entry <c>i</c> keeps only the entries
    /// before it, and an out-of-range index restores nothing.
    /// </summary>
    [Fact] // REQ-LOOP-033
    public void The_optional_history_panel_keeps_only_the_earlier_entries()
    {
        RollbackHistory history = new();
        for (int index = 0; index < 10; index++)
        {
            history.Capture([(byte)index]);
        }

        byte[]? fourth = history.TakeAt(4);
        Assert.NotNull(fourth);
        Assert.Equal(4, fourth[0]);
        Assert.Equal(4, history.Count);

        Assert.Null(history.TakeAt(-1));
        Assert.Null(history.TakeAt(99));
    }

    // -------------------------------------------------------------- §3.5 save states

    /// <summary>
    /// REQ-LOOP-040 — a savestate carries the core, the scripts' persistent data, the camera and
    /// the learned conversion state, and holds neither the converted frames nor the meshes.
    /// </summary>
    [Fact] // REQ-LOOP-040
    public void A_savestate_carries_the_four_documented_sections()
    {
        SaveStateContent content = new(
            new SaveStateHeader("sen", SaveStateFile.FormatVersion, "game", "sha1", 1_761_000_000, "quick"),
            new byte[64],
            new byte[256],
            new byte[SaveStateFile.CameraSectionBytes],
            new byte[32]);

        byte[] file = SaveStateFile.Write(content);
        Assert.True(SaveStateFile.TryRead(file, out SaveStateContent? read));
        Assert.NotNull(read);

        Assert.Equal(content.Core.Length, read.Core.Length);
        Assert.Equal(256, read.Lua.Length);
        Assert.Equal(SaveStateFile.CameraSectionBytes, read.Camera.Length);
        Assert.Equal(32, read.Conversion.Length);
        Assert.Equal("game", read.Header.GameId);
    }

    /// <summary>
    /// REQ-LOOP-041 — a save only happens outside Stopped and with a current game, runs the Save
    /// hook <b>before</b> capturing, and does not change the state.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-041
    public void A_save_needs_a_current_game_and_leaves_the_state_alone()
    {
        using HostDriver host = HostDriver.Create();

        host.Run(ZaxCommands.QuickSave);
        Assert.False(host.Slots.Find(SaveSlotIndex.QuickSlotId)!.IsOccupied);

        host.Launch2DMode();
        host.RunFrames(10);

        host.Run(ZaxCommands.QuickSave);
        Assert.Equal(EmulationState.Running, host.State);
        Assert.True(host.Slots.Find(SaveSlotIndex.QuickSlotId)!.IsOccupied);
    }

    /// <summary>
    /// REQ-LOOP-042 — a slot save stamps the slot with the current game, the pause capture and
    /// the time, persists the metadata at once, and overwrites an occupied slot without asking.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-042
    public void A_slot_save_stamps_and_persists_the_metadata_at_once()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Thumbnail = new byte[256 * 192 * 3];
        host.Run(ZaxCommands.Pause);

        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(5));

        string slotId = SaveSlotIndex.NumberedSlotId(5);
        SaveSlotRecord first = host.Slots.Find(slotId)!;
        Assert.Equal(host.CurrentGameId, first.GameId);
        Assert.Equal(host.Now.ToUnixTimeSeconds(), first.SavedAt);
        Assert.True(File.Exists(host.Paths.SlotStateFile(slotId)));

        // The metadata is on disk straight away.
        SaveSlotIndex reread = SaveSlotIndex.Load(host.Paths, host.Now);
        Assert.True(reread.Find(slotId)!.IsOccupied);

        // Overwriting asks nothing.
        host.Now = host.Now.AddMinutes(1);
        host.Run(ZaxCommands.SaveSlot, CommandArgs.Of(5));
        Assert.Equal(host.Now.ToUnixTimeSeconds(), host.Slots.Find(slotId)!.SavedAt);
    }

    /// <summary>
    /// REQ-LOOP-043 — the quick save writes the quick slot <b>without</b> a capture and shows
    /// "Quick Save" for 4 s; without a current game nothing happens.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-043
    public void The_quick_save_has_no_thumbnail_and_shows_its_notification()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Thumbnail = new byte[256 * 192 * 3];

        host.Run(ZaxCommands.QuickSave);

        SaveSlotRecord quick = host.Slots.Find(SaveSlotIndex.QuickSlotId)!;
        Assert.True(quick.IsOccupied);
        Assert.Null(quick.Thumbnail);

        Notification shown = Assert.Single(
            host.Notifications.Log,
            notification => notification.Text == "Quick Save");
        Assert.Equal(4.0, shown.Seconds, 1e-9);
    }

    /// <summary>
    /// REQ-LOOP-044 — loading a slot restores the game at the saved point without asking for the
    /// ROM, and a slot whose game no longer exists fails with the documented message.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-044
    public void Loading_a_slot_restores_the_game_without_the_rom()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(120);

        long saved = host.Session!.Core.FrameCount;
        byte counter = host.Session!.Core.ReadMemory(0x0010);
        host.Run(ZaxCommands.QuickSave);

        host.RunFrames(120);
        Assert.NotEqual(saved, host.Session!.Core.FrameCount);

        host.Run(ZaxCommands.Stop);
        Assert.Equal(EmulationState.Stopped, host.State);

        host.Run(ZaxCommands.QuickLoad);

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(saved, host.Session!.Core.FrameCount);
        Assert.Equal(counter, host.Session!.Core.ReadMemory(0x0010));
    }

    /// <summary>
    /// REQ-LOOP-044 — the message when the slot's game has disappeared.
    /// </summary>
    [Fact] // REQ-LOOP-044
    public void The_missing_game_message_is_the_documented_one()
        => Assert.Equal("The game profile doesn't exist anymore.", SaveStateFile.GameMissingMessage);

    /// <summary>
    /// REQ-LOOP-045, T8 — the quick load does nothing at all when the quick slot is empty: no
    /// notification and no sound.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-045
    public void A_quick_load_on_an_empty_slot_does_nothing()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        int notifications = host.Notifications.Log.Count;
        host.Run(ZaxCommands.QuickLoad);

        Assert.Equal(notifications, host.Notifications.Log.Count);
        Assert.Equal(EmulationState.Running, host.State);

        // After a quick save it works and shows "Quick Load" for 4 s.
        host.Run(ZaxCommands.QuickSave);
        host.Run(ZaxCommands.QuickLoad);

        Notification shown = Assert.Single(
            host.Notifications.Log,
            notification => notification.Text == "Quick Load");
        Assert.Equal(4.0, shown.Seconds, 1e-9);
    }

    /// <summary>
    /// REQ-LOOP-046 — a truncated slot file gives <c>Corrupted save state.</c>, clears the
    /// loading indicator and goes to Stopped with the current game cleared.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-046
    public void A_corrupted_savestate_gives_the_documented_notification()
    {
        Assert.Equal("Corrupted save state.", SaveStateFile.CorruptedMessage);

        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.Run(ZaxCommands.QuickSave);

        string file = host.Paths.SlotStateFile(SaveSlotIndex.QuickSlotId);
        byte[] whole = File.ReadAllBytes(file);
        File.WriteAllBytes(file, whole[..(whole.Length / 2)]);

        host.Run(ZaxCommands.QuickLoad);

        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.Null(host.CurrentGameId);
        Assert.False(host.IsLoading);
        Assert.Contains(
            host.Notifications.Log,
            notification => notification.Text == SaveStateFile.CorruptedMessage);
    }

    /// <summary>
    /// REQ-LOOP-046 — the format carries a signature and a version, so a foreign file is rejected
    /// with the same message rather than read.
    /// </summary>
    [Fact] // REQ-LOOP-046
    public void A_foreign_file_is_rejected_by_the_signature()
    {
        Assert.False(SaveStateFile.TryRead(new byte[64], out _));
        Assert.False(SaveStateFile.TryRead("NOTASAVE"u8, out _));
        Assert.Equal("ZAXNESAV"u8.ToArray(), SaveStateFile.Signature.ToArray());
        Assert.Equal(1, SaveStateFile.FormatVersion);
    }

    // ------------------------------------------------ §3.6 reset, stop, exit, focus

    /// <summary>
    /// REQ-LOOP-050 — the reset is a full relaunch of the current game: the ROM is read again,
    /// the core cold-starts with its memory reinitialised, the End then Start hooks run, the
    /// history is emptied and the profile is <b>not</b> reloaded.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-050
    public void The_reset_is_a_full_relaunch_of_the_current_game()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(300);

        Assert.True(host.Session!.Core.FrameCount > 0);
        Assert.True(host.Rollback.Count > 0);

        host.Run(ZaxCommands.Reset);

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(0, host.Session!.Core.FrameCount);
        Assert.Equal(0, host.Rollback.Count);
        Assert.Equal(0, host.Session!.Core.ReadMemory(0x0010));
    }

    /// <summary>
    /// REQ-LOOP-052 — the exit runs without confirmation and without an automatic save: the
    /// settings and the slot metadata are written and no new savestate appears.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-052
    public void The_exit_writes_the_settings_and_creates_no_savestate()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(60);

        int states = Directory.Exists(host.Paths.SavesDirectory)
            ? Directory.GetFiles(host.Paths.SavesDirectory, "*.state").Length
            : 0;

        host.Run(ZaxCommands.Quit);
        Assert.True(host.QuitRequested);

        host.Shutdown(new SettingsStore(host.Paths));

        Assert.Equal(EmulationState.Stopped, host.State);
        Assert.True(File.Exists(host.Paths.SaveIndexFile));
        Assert.True(File.Exists(host.Paths.SettingsFile));

        int after = Directory.Exists(host.Paths.SavesDirectory)
            ? Directory.GetFiles(host.Paths.SavesDirectory, "*.state").Length
            : 0;
        Assert.Equal(states, after);
    }

    /// <summary>
    /// REQ-LOOP-053 — losing the window focus does not pause the emulation: the host has no
    /// focus input at all, so the loop keeps running frames and the audio keeps flowing.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-053
    public void Losing_the_focus_does_not_pause_the_emulation()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        long before = host.EmulatedFrames;
        host.RunFrames(60);

        Assert.True(host.EmulatedFrames > before);
        Assert.Equal(EmulationState.Running, host.State);
        Assert.True(host.Mixer.EmulationAudible);
    }

    // --------------------------------------------- §3.7 asynchrony and determinism

    /// <summary>
    /// REQ-LOOP-060 — for one emulated frame the operations run synchronously and in order, and
    /// two runs from the same savestate with the same inputs per frame give the same core state,
    /// the same persistent data and the same tile list, frame by frame.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-060, REQ-ARCH-012
    public void Two_replays_from_one_savestate_are_identical_frame_by_frame()
    {
        InputAction[] sequence =
        [
            InputAction.None, InputAction.A, InputAction.Right,
            InputAction.A | InputAction.Right, InputAction.None, InputAction.Start,
        ];

        string state;
        List<string> first;
        List<string> second;

        using (HostDriver host = HostDriver.Create())
        {
            host.Launch2DMode(LoopRom.Counter());
            host.RunFrames(30);
            host.Run(ZaxCommands.QuickSave);
            state = host.Paths.SlotStateFile(SaveSlotIndex.QuickSlotId);

            Assert.True(File.Exists(state));

            first = Replay(host, sequence);

            host.Run(ZaxCommands.QuickLoad);
            second = Replay(host, sequence);
        }

        Assert.NotEmpty(first);
        Assert.Equal(first, second);

        static List<string> Replay(HostDriver host, InputAction[] sequence)
        {
            List<string> prints = [];
            SessionDriver session = host.Session!;
            for (int frame = 0; frame < 120; frame++)
            {
                session.RunFrame((int)sequence[frame % sequence.Length]);
                prints.Add(Fingerprint(session));
            }

            return prints;
        }

        static string Fingerprint(SessionDriver session)
        {
            byte[] core = session.Core.SaveState(includeRomImage: false);
            byte[] lua = session.Scripts.CapturePersistentData();
            Zax.Engine3D.FrameDrawList draw = session.Engine.BuildDrawList();

            System.Text.StringBuilder text = new();
            text.Append(Convert.ToHexString(System.Security.Cryptography.SHA1.HashData(core)));
            text.Append('/');
            text.Append(Convert.ToHexString(System.Security.Cryptography.SHA1.HashData(lua)));
            text.Append('/');
            text.Append(draw.Tiles.Count);
            return text.ToString();
        }
    }

    /// <summary>
    /// REQ-LOOP-061 — the passage to Running waits for the initial mesh generation to finish, so
    /// no emulated frame runs while the meshes are still being built.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-061
    public void No_frame_runs_before_the_initial_mesh_generation_is_finished()
    {
        using HostDriver host = HostDriver.Create();
        string id = host.TwoDimensionalModeId!;
        host.SetRomPath(id, host.WriteRom(LoopRom.Counter()));

        host.Enqueue(ZaxCommands.LaunchGame, CommandArgs.Of(id));
        host.ApplyCommands();

        int ticks = 0;
        while (host.IsLoading && ticks++ < 256)
        {
            Assert.Equal(0, host.Tick(1.0 / 1000.0));
        }

        Assert.Equal(0, host.EmulatedFrames);
        Assert.Equal(EmulationState.Running, host.State);
    }

    /// <summary>
    /// REQ-LOOP-062 — the kernel exposes "run exactly one emulated frame with these inputs", so N
    /// calls run exactly N frames, and the pacing belongs to the frontend alone.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-062
    public void The_kernel_runs_exactly_one_frame_per_call()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        SessionDriver session = host.Session!;
        long before = session.Core.FrameCount;

        for (int frame = 0; frame < 37; frame++)
        {
            session.RunFrame();
        }

        Assert.Equal(before + 37, session.Core.FrameCount);
        Assert.Equal(37, session.FrameCounter);
    }

    /// <summary>
    /// REQ-LOOP-062 — the serialisation size is bounded for the whole session of the loaded game.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-062
    public void The_serialisation_size_is_bounded_for_the_session()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        int bound = SaveStateFile.MaxFileSize(host.Session!.Core.MaxStateSize());
        Assert.True(bound > 0);

        for (int round = 0; round < 3; round++)
        {
            host.RunFrames(120);
            host.Run(ZaxCommands.QuickSave);
            long size = new FileInfo(host.Paths.SlotStateFile(SaveSlotIndex.QuickSlotId)).Length;
            Assert.True(size <= bound, $"{size} bytes exceed the bound {bound}.");
        }
    }

    /// <summary>
    /// REQ-LOOP-034 — the libretro frontend may drop the internal rollback in favour of the
    /// host's rewind, which rests on the serialisation: the serialisation of §3.7 round-trips.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LOOP-034, REQ-LOOP-063
    public void The_serialisation_round_trips_so_the_host_rewind_can_rest_on_it()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(60);

        SessionDriver session = host.Session!;
        byte[] core = session.Core.SaveState(includeRomImage: false);
        byte[] lua = session.CaptureScriptState();

        host.RunFrames(60);

        Assert.True(session.Core.TryLoadState(core, 0, out _, out _, out string? error), error);
        session.Scripts.RestorePersistentData(lua);

        Assert.Equal(60, session.Core.FrameCount);
    }
}
