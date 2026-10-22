using Zax.App.Input;
using Zax.App.Loop;

namespace Zax.App.Tests;

/// <summary>
/// The cadence of SPEC-07 §3.2: the nominal rate, the catch-up limit, the clock reset, the
/// speed setting and the fast-forward model, plus the test vectors of SPEC-07 §5.
/// </summary>
public sealed class PacingTests
{
    /// <summary>
    /// REQ-LOOP-015: the truth table of the speed setting. The mapping is differential, so −8
    /// gives 1/3 and +8 gives 3.
    /// </summary>
    [Theory]
    [InlineData(-8, 1.0 / 3.0)]
    [InlineData(-4, 0.5)]
    [InlineData(-1, 0.8)]
    [InlineData(0, 1.0)]
    [InlineData(1, 1.25)]
    [InlineData(2, 1.5)]
    [InlineData(4, 2.0)]
    [InlineData(8, 3.0)]
    public void REQ_LOOP_015_the_speed_setting_maps_to_a_factor(int setting, double factor)
        => Assert.Equal(factor, SpeedSetting.Factor(setting), 1e-9);

    /// <summary>SPEC-09 REQ-PERS-003 rule 3: the setting is clamped to [−8 ; 8].</summary>
    [Fact]
    public void REQ_PERS_003_the_speed_setting_is_clamped()
    {
        Assert.Equal(-8, SpeedSetting.Clamp(-9));
        Assert.Equal(8, SpeedSetting.Clamp(12));
        Assert.Equal(1.5, SpeedSetting.FastForwardFactor, 1e-9);
    }

    /// <summary>
    /// REQ-LOOP-010 and vector T1: 10 s at <c>s = 0</c> in NTSC hand out 600 frames, and
    /// vector T2 gives 250 at <c>s = −4</c> in PAL.
    /// </summary>
    [Theory]
    [InlineData(60, 0, 10.0, 600)]
    [InlineData(50, -4, 10.0, 250)]
    [InlineData(60, 8, 1.0, 180)]
    public void REQ_LOOP_010_the_clock_hands_out_the_nominal_rate(
        double nominal,
        int setting,
        double seconds,
        int expected)
    {
        double now = 0.0;
        FramePacer pacer = new(nominal, () => now) { Speed = SpeedSetting.Factor(setting) };

        int frames = 0;
        const double step = 1.0 / 1000.0;
        while (now < seconds)
        {
            now += step;
            frames += pacer.DueFrames();
        }

        Assert.InRange(frames, expected - 1, expected + 1);
    }

    /// <summary>
    /// REQ-LOOP-012 and vector T5: a one-second stall catches up at most five frames and
    /// abandons the rest, so the game slows down instead of racing.
    /// </summary>
    [Fact]
    public void REQ_LOOP_012_the_backlog_is_limited_to_five_frames()
    {
        double now = 0.0;
        FramePacer pacer = new(60, () => now);

        Assert.Equal(1, pacer.DueFrames());

        now += 1.0;
        int due = pacer.DueFrames();

        Assert.Equal(FramePacer.MaxBacklog, due);
        Assert.InRange(pacer.DroppedFrames, 50, 60);
    }

    /// <summary>
    /// REQ-LOOP-013 and vector T7: resetting the clock drops the backlog, so a long pause is
    /// followed by the nominal rate with no burst.
    /// </summary>
    [Fact]
    public void REQ_LOOP_013_resetting_the_clock_drops_the_backlog()
    {
        double now = 0.0;
        FramePacer pacer = new(60, () => now);
        pacer.DueFrames();

        now += 30.0;
        pacer.Reset();

        // Frame 0 is due at t0 itself (REQ-LOOP-011), so one frame is handed out and the
        // thirty seconds of backlog are gone.
        Assert.Equal(1, pacer.DueFrames());

        int frames = 1;
        double limit = now + 1.0;
        while (now < limit)
        {
            now += 1.0 / 1000.0;
            frames += pacer.DueFrames();
        }

        Assert.InRange(frames, 59, 61);
    }

    /// <summary>
    /// REQ-LOOP-016: one request arms exactly 30 emulated frames at ×1.5, and a request made
    /// while the burst runs has no effect.
    /// </summary>
    [Fact]
    public void REQ_LOOP_016_one_request_arms_thirty_frames()
    {
        FastForwardState fast = new();

        Assert.True(fast.Request());
        Assert.Equal(FastForwardState.BurstFrames, fast.Remaining);

        // A second request during the burst is ignored.
        Assert.False(fast.Request());
        Assert.Equal(FastForwardState.BurstFrames, fast.Remaining);

        for (int frame = 0; frame < FastForwardState.BurstFrames; frame++)
        {
            Assert.True(fast.IsActive);
            fast.OnFrameRun();
        }

        Assert.False(fast.IsActive);
        Assert.True(fast.Request());
    }

    /// <summary>
    /// REQ-LOOP-016 and vector T4: holding the hotkey — one request per rendered frame — keeps
    /// the factor at 1.5 almost continuously, so one second of NTSC gives about 90 frames.
    /// </summary>
    [Fact]
    public void REQ_LOOP_016_holding_the_hotkey_keeps_the_rate_at_one_and_a_half()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        long before = harness.Host.EmulatedFrames;
        int frames = 0;
        for (int tick = 0; tick < 90; tick++)
        {
            // 90 rendered frames of 1/90 s: one second of real time, the hotkey held.
            frames += harness.Tick(1.0 / 90.0, InputAction.FastForward);
        }

        long ran = harness.Host.EmulatedFrames - before;
        Assert.Equal(frames, (int)ran);
        Assert.InRange(ran, 81, 99);

        // REQ-LOOP-016: the notification is shown while the key is held.
        Assert.Equal("Fast Forward", harness.Host.Notifications.Current!.Value.Text);
        Assert.Equal(1.0, harness.Host.Notifications.Current!.Value.Seconds);

        // On release, between 0 and 30 frames of the burst remain.
        Assert.InRange(harness.Host.FastForward.Remaining, 0, FastForwardState.BurstFrames);
    }

    /// <summary>
    /// REQ-LOOP-015 and REQ-AUD-011: the speed setting multiplies the cadence and is handed to
    /// the core, which retimes its audio — pitch included — without the game state changing.
    /// </summary>
    [Fact]
    public void REQ_LOOP_015_a_speed_change_takes_effect_on_the_next_frame()
    {
        using LoopHarness harness = LoopHarness.Create();
        EmulationStateMachineTests.Launch(harness, "g1");

        long frames = harness.Host.Session!.Core.FrameCount;
        harness.Settings.GameSpeed = 4;
        harness.Tick();

        Assert.Equal(2.0, harness.Host.Pacer.Speed, 1e-9);
        Assert.Equal(2.0, harness.Host.Session!.Core.Speed, 1e-9);
        Assert.True(harness.Host.Session.Core.FrameCount >= frames);
    }

    /// <summary>
    /// REQ-LOOP-011: at <c>k = 1</c> in NTSC, the interval between two consecutive emulated
    /// frames stays inside 1/f ± 2 ms for the overwhelming majority of frames.
    /// </summary>
    [Fact]
    public void REQ_LOOP_011_the_interval_between_frames_stays_close_to_the_period()
    {
        double now = 0.0;
        FramePacer pacer = new(60, () => now);
        pacer.DueFrames();

        List<double> intervals = [];
        double last = now;
        while (intervals.Count < 300)
        {
            now += 1.0 / 1000.0;
            int due = pacer.DueFrames();
            for (int frame = 0; frame < due; frame++)
            {
                intervals.Add(now - last);
                last = now;
            }
        }

        int inside = intervals.Count(interval => interval is >= 0.0146 and <= 0.0186);
        Assert.True(inside >= intervals.Count * 95 / 100, $"only {inside}/{intervals.Count} inside");
    }
}
