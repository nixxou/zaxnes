using Zax.App.Input;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// Lot V2-14 (AMD-052): the catch-up limit of REQ-LOOP-074, the clock reset on a speed change of
/// REQ-LOOP-075, and the part of the GPU frame queue of REQ-LOOP-077 that lives outside OpenGL —
/// the setting itself and its effect on the emulated cadence. The fence criterion and the latency
/// bench of REQ-LOOP-077 need a context and are in <c>Zax.Acceptance.Tests</c>.
/// </summary>
public sealed class Amd052CadenceTests
{
    /// <summary>
    /// REQ-LOOP-074, amended criterion of REQ-LOOP-012: a 500 ms block at 60 Hz runs at most two
    /// frames in a burst, then the normal cadence resumes, and the frame counter is about 28
    /// frames short of an unblocked run.
    /// </summary>
    [Fact] // REQ-LOOP-074, REQ-LOOP-012
    public void A_five_hundred_millisecond_block_costs_about_twenty_eight_frames()
    {
        // The reference run: one second of 1 ms steps, nothing in the way.
        double free = 0.0;
        FramePacer plain = new(60, () => free);
        int unblocked = 0;
        while (free < 1.0 - 1e-9)
        {
            free += 0.001;
            unblocked += plain.DueFrames();
        }

        // The same second, with the loop stuck for 500 ms in the middle of it.
        double now = 0.0;
        FramePacer pacer = new(60, () => now);
        int ran = 0;
        while (now < 0.25 - 1e-9)
        {
            now += 0.001;
            ran += pacer.DueFrames();
        }

        now += 0.5;
        int burst = pacer.DueFrames();
        ran += burst;

        // Then the normal cadence resumes: one frame per period, no second burst.
        while (now < 1.0 - 1e-9)
        {
            now += 0.001;
            int due = pacer.DueFrames();
            Assert.InRange(due, 0, 1);
            ran += due;
        }

        Assert.Equal(2, burst);
        Assert.InRange(unblocked - ran, 27, 29);
    }

    /// <summary>
    /// REQ-LOOP-075, first criterion: going from the normal speed to ×2 and back produces no
    /// burst of catch-up frames, because each change of <c>emu.game_speed</c> resets the cadence
    /// clock (REQ-LOOP-013 with the line AMD-052 §3 point 3 adds to it).
    /// </summary>
    [Fact] // REQ-LOOP-075, REQ-LOOP-013, REQ-LOOP-015
    public void A_speed_change_resets_the_clock_and_produces_no_burst()
    {
        // What the reset buys, on the clock alone: half a second of lateness is abandoned at the
        // change instead of being run as a burst at the new period. Without the reset the same
        // call would hand out the two frames of REQ-LOOP-074 and abandon 28.
        double now = 0.0;
        FramePacer pacer = new(60, () => now);
        pacer.DueFrames();
        now += 0.5;

        pacer.Speed = 2.0;
        pacer.Reset();

        Assert.Equal(1, pacer.DueFrames());
        Assert.Equal(0, pacer.DroppedFrames);

        // And in the loop: every change of emu.game_speed resets the clock — the ±1 shortcuts
        // of REQ-LOOP-015 included, since they write the same setting — and no tick that follows
        // runs a catch-up burst.
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.Tick(1.0 / 60.0);

        long resets = harness.Host.Pacer.ClockResets;

        harness.Settings.GameSpeed = 4;
        Assert.Equal(2.0, harness.Host.Pacer.Speed, 1e-9);
        Assert.Equal(resets + 1, harness.Host.Pacer.ClockResets);
        AssertNoBurst(harness, period: 1.0 / 120.0);

        harness.Run(ZaxCommands.SpeedDown);
        Assert.Equal(3, harness.Settings.GameSpeed);
        Assert.Equal(resets + 2, harness.Host.Pacer.ClockResets);

        harness.Settings.GameSpeed = 0;
        Assert.Equal(1.0, harness.Host.Pacer.Speed, 1e-9);
        Assert.Equal(resets + 3, harness.Host.Pacer.ClockResets);
        AssertNoBurst(harness, period: 1.0 / 60.0);
    }

    /// <summary>
    /// REQ-LOOP-075, second criterion: NTSC, <c>s = 0</c>, the fast-forward hotkey held for 5 s.
    /// The clock-reset counter stays at 0 for the whole hold, the criterion of REQ-LOOP-016
    /// still holds (≈ 450 emulated frames ±5 %), and the gap between two consecutive emulated
    /// frames stays inside <c>1/f ± 2 ms</c> with <c>f = 90</c> while the key is held and with
    /// <c>f = 60</c> after it is released (statement of REQ-LOOP-011).
    /// </summary>
    /// <remarks>
    /// This is the criterion the second R2 pass rejected in v1.1 (point 52-N1): it used to cite
    /// the acceptance criterion of REQ-LOOP-011, which is posed at <c>k = 1</c>, against a hold
    /// that runs at <c>k = 1.5</c>. The statement of REQ-LOOP-011 is parameterised by <c>f</c>,
    /// so it is the one measured here. Arming, expiring and re-arming a burst happens about six
    /// times a second — a burst of 30 emulated frames at ×1.5 lasts ≈ 0.33 s — which is why
    /// REQ-LOOP-075 excludes the cycle of REQ-LOOP-016 from the resets of REQ-LOOP-013.
    /// </remarks>
    [Fact] // REQ-LOOP-075, REQ-LOOP-016, REQ-LOOP-011
    public void Holding_the_fast_forward_hotkey_never_resets_the_clock()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        long resets = harness.Host.Pacer.ClockResets;
        List<double> held = Intervals(harness, seconds: 5.0, InputAction.FastForward, out int frames);

        // The hold itself: no clock reset, and the ≈ 450 frames of REQ-LOOP-016 ±5 %.
        Assert.Equal(resets, harness.Host.Pacer.ClockResets);
        Assert.InRange(frames, 427, 473);
        AssertIntervalsNear(held, 1.0 / 90.0);

        // On release, between 0 and 30 emulated frames of the last burst remain (REQ-LOOP-016);
        // they are drained before the cadence at f = 60 is measured.
        Intervals(harness, seconds: 0.5, InputAction.None, out _);
        List<double> released = Intervals(harness, seconds: 2.0, InputAction.None, out _);

        Assert.Equal(resets, harness.Host.Pacer.ClockResets);
        AssertIntervalsNear(released, 1.0 / 60.0);
    }

    /// <summary>
    /// REQ-LOOP-077: the setting of SPEC-09 §4.1.1 AMD-052 §3 point 7 adds — a whole number, 1
    /// or 2, default 2, scope U, immediate — and nothing outside the domain gets through.
    /// </summary>
    [Fact] // REQ-LOOP-077
    public void The_queued_frame_setting_is_a_user_preference_of_one_or_two()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get(SettingKeys.VideoMaxQueuedFrames);

        Assert.Equal(SettingKind.Int, descriptor.Kind);
        Assert.Equal(SettingScope.User, descriptor.Scope);
        Assert.Equal(SettingEffect.Immediate, descriptor.Effect);
        Assert.Equal(2, descriptor.DefaultValue.AsInt);
        Assert.Equal([1d, 2d], descriptor.Domain.Options!.Select(static option => option.Value));

        // The presentation chain belongs to the host in libretro (REQ-PERS-086), so the key is
        // not a core option — AMD-052 §3 point 8 lists it beside video.vsync.
        Assert.False(descriptor.IsLibretroExposed);
        Assert.Null(descriptor.LuaPath);
        Assert.Null(descriptor.ProfileField);

        AppSettings settings = new();
        Assert.Equal(2, settings.MaxQueuedFrames);

        settings.MaxQueuedFrames = 1;
        Assert.Equal(1, settings.MaxQueuedFrames);

        // Out of domain falls back to the default, as REQ-PERS-003 asks of an enumerated domain.
        settings.MaxQueuedFrames = 7;
        Assert.Equal(2, settings.MaxQueuedFrames);
    }

    /// <summary>
    /// REQ-LOOP-077, first criterion: the setting is persisted and survives a restart.
    /// </summary>
    [Fact] // REQ-LOOP-077, REQ-PERS-020
    public void The_queued_frame_setting_survives_a_restart()
    {
        string root = Path.Combine(Path.GetTempPath(), $"zax-amd052-{Guid.NewGuid():N}");
        try
        {
            UserDataPaths paths = UserDataPaths.Resolve(root);
            Assert.True(SettingsCatalogue.Get(SettingKeys.VideoMaxQueuedFrames).IsPersisted);

            AppSettings first = new();
            SettingsStore writer = new(paths);
            writer.Load(first, DateTimeOffset.UnixEpoch);
            Assert.Equal(2, first.MaxQueuedFrames);

            first.MaxQueuedFrames = 1;
            writer.Save(first);

            AppSettings second = new();
            SettingsStore reader = new(paths);
            reader.Load(second, DateTimeOffset.UnixEpoch);

            Assert.Null(reader.Message);
            Assert.Equal(1, second.MaxQueuedFrames);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }

    /// <summary>
    /// REQ-LOOP-077, last criterion: at <c>video.max_queued_frames</c> = 1, NTSC, <c>k = 1</c>,
    /// ten seconds still hand out 600 ± 1 emulated frames — the criterion of REQ-LOOP-010 is
    /// unchanged — including when the rendering is deliberately slowed down, here to 30 rendered
    /// frames a second. The wait on the GPU is not a delay in the sense of REQ-LOOP-074: the
    /// frame clock is absolute, so no frame is abandoned as long as the backlog stays inside the
    /// limit of two.
    /// </summary>
    [Fact] // REQ-LOOP-077, REQ-LOOP-010, REQ-LOOP-074
    public void A_slowed_render_does_not_move_the_emulated_cadence()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");
        harness.Settings.MaxQueuedFrames = 1;

        long before = harness.Host.EmulatedFrames;
        long resets = harness.Host.Pacer.ClockResets;

        // 300 rendered frames of 1/30 s: ten seconds of real time with the display path half as
        // fast as the emulation.
        for (int tick = 0; tick < 300; tick++)
        {
            harness.Tick(1.0 / 30.0);
        }

        long ran = harness.Host.EmulatedFrames - before;

        Assert.InRange(ran, 599, 601);
        Assert.Equal(resets, harness.Host.Pacer.ClockResets);

        // The 1 ms early start of REQ-LOOP-011 can round one frame of the ten seconds onto the
        // wrong side of a rendered frame; the limit of REQ-LOOP-074 is never reached.
        // Q-087
        Assert.InRange(harness.Host.Pacer.DroppedFrames, 0, 1);
    }

    /// <summary>
    /// Asserts that half a second of ticks at half <paramref name="period"/> never runs more
    /// than one emulated frame at a time and abandons nothing: no catch-up burst.
    /// </summary>
    private static void AssertNoBurst(LoopHarness harness, double period)
    {
        long dropped = harness.Host.Pacer.DroppedFrames;
        for (int tick = 0; tick < (int)Math.Round(1.0 / period); tick++)
        {
            Assert.InRange(harness.Tick(period / 2.0), 0, 1);
        }

        Assert.Equal(dropped, harness.Host.Pacer.DroppedFrames);
    }

    /// <summary>
    /// Runs <paramref name="seconds"/> of real time as 1 ms rendered frames — fine enough to
    /// time the emulated frames one by one — and returns the gaps between consecutive emulated
    /// frames.
    /// </summary>
    private static List<double> Intervals(
        LoopHarness harness,
        double seconds,
        InputAction actions,
        out int frames)
    {
        const double step = 1.0 / 1000.0;
        List<double> intervals = [];
        double last = harness.Clock;
        frames = 0;

        for (int tick = 0; tick < (int)Math.Round(seconds / step); tick++)
        {
            int due = harness.Tick(step, actions);
            frames += due;
            for (int frame = 0; frame < due; frame++)
            {
                intervals.Add(harness.Clock - last);
                last = harness.Clock;
            }
        }

        return intervals;
    }

    /// <summary>
    /// Asserts that the overwhelming majority of <paramref name="intervals"/> sit inside
    /// <c>period ± 2 ms</c>, which is what the statement of REQ-LOOP-011 asks of the gap between
    /// two consecutive emulated frames.
    /// </summary>
    private static void AssertIntervalsNear(List<double> intervals, double period)
    {
        Assert.NotEmpty(intervals);
        int inside = intervals.Count(interval => Math.Abs(interval - period) <= 0.002);
        Assert.True(
            inside >= intervals.Count * 95 / 100,
            $"only {inside} of {intervals.Count} intervals are inside {period * 1000:F2} ms ± 2 ms.");
    }
}
