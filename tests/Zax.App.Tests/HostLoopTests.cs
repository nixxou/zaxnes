using System.Diagnostics;
using Zax.App.Input;
using Zax.App;
using Zax.Render;
using Silk.NET.Input;

namespace Zax.App.Tests;

/// <summary>
/// The host-side pieces of the playable slice: the emulated frame clock of SPEC-07 §3.2 and the
/// player 1 keyboard of SPEC-08 §4.1 and §4.2.
/// </summary>
public sealed class HostLoopTests
{
    [Fact]
    public void REQ_LOOP_012_the_backlog_never_exceeds_five_emulated_frames()
    {
        FramePacer pacer = new(60);

        // Frame 0 is due at t0 itself (REQ-LOOP-011), so the first call hands out one frame.
        Assert.Equal(1, pacer.DueFrames());

        // Block the loop for half a second: at most five frames are run in a burst and the
        // rest of the backlog is abandoned.
        Block(0.5);
        int due = pacer.DueFrames();

        Assert.Equal(FramePacer.MaxBacklog, due);
        Assert.True(pacer.DroppedFrames > 20, $"only {pacer.DroppedFrames} frames were dropped");
    }

    [Fact]
    public void REQ_LOOP_013_resetting_the_clock_drops_the_backlog()
    {
        FramePacer pacer = new(60);
        Block(0.2);
        pacer.Reset();

        // The twelve frames the block made late are gone: only the frame due at t0 is handed
        // out, so the resume shows no burst.
        Assert.Equal(1, pacer.DueFrames());
    }

    [Fact]
    public void REQ_LOOP_015_the_speed_factor_multiplies_the_nominal_rate()
    {
        FramePacer pacer = new(60) { Speed = 1.5 };
        Assert.Equal(90.0, pacer.EffectiveRate, 1e-9);

        pacer.Speed = 0.5;
        Assert.Equal(30.0, pacer.EffectiveRate, 1e-9);
    }

    [Fact]
    public void REQ_LOOP_010_the_clock_hands_out_about_the_nominal_rate()
    {
        FramePacer pacer = new(60);
        Stopwatch watch = Stopwatch.StartNew();
        int total = 0;
        while (watch.Elapsed.TotalSeconds < 0.5)
        {
            total += pacer.DueFrames();
            Thread.Sleep(1);
        }

        // 0.5 s at 60 frames/s, plus the frame due at t0, with a margin for a loaded machine.
        Assert.InRange(total, 25, 34);
    }

    [Fact]
    public void Section_2_a_press_released_under_0_3_s_is_short_and_a_longer_one_is_long()
    {
        PressTracker tracker = new();

        Assert.Equal(PressKind.None, tracker.Update(down: true, 0.1f));
        Assert.Equal(PressKind.Short, tracker.Update(down: false, 0.0f));

        Assert.Equal(PressKind.None, tracker.Update(down: true, 0.1f));
        Assert.Equal(PressKind.None, tracker.Update(down: true, 0.1f));
        Assert.Equal(PressKind.Long, tracker.Update(down: true, 0.2f));

        // A long press is reported once, and the release that follows is not a short press.
        Assert.Equal(PressKind.None, tracker.Update(down: true, 0.2f));
        Assert.Equal(PressKind.None, tracker.Update(down: false, 0.0f));
    }

    [Fact]
    public void AppOptions_parses_the_options_of_the_slice()
    {
        AppOptions options = AppOptions.Parse(
            ["--rom", "game.nes", "--profile", "Balloon Fight", "--size", "1600x900",
             "--fov", "85", "--no-shadows", "--no-audio"]);

        Assert.Equal("game.nes", options.RomPath);
        Assert.Equal("Balloon Fight", options.Profile);
        Assert.Equal(1600, options.Width);
        Assert.Equal(900, options.Height);
        Assert.Equal(85f, options.Fov);
        Assert.False(options.Shadows);
        Assert.False(options.Audio);
    }

    /// <summary>
    /// REQ-SCN-060 and REQ-SCN-061 with Q-066: the display options parse, and an option that was
    /// not given stays null — "use the persisted setting" — instead of carrying a default that
    /// would overwrite it at every launch.
    /// </summary>
    [Fact]
    public void REQ_SCN_060_061_the_display_options_parse_and_default_to_the_setting()
    {
        AppOptions defaults = AppOptions.Parse([]);
        Assert.Null(defaults.FrameRate);
        Assert.Null(defaults.SkipRender);
        Assert.Null(defaults.Skybox);
        Assert.Null(defaults.TvBox);
        Assert.Null(defaults.VSync);
        Assert.Null(defaults.Fov);
        Assert.Null(defaults.Shadows);

        // SPEC-09 gives video.vsync three values; the bare flag is still "every refresh".
        Assert.Equal(1, AppOptions.Parse(["--vsync"]).VSync);
        Assert.Equal(0, AppOptions.Parse(["--vsync", "0"]).VSync);
        Assert.Equal(2, AppOptions.Parse(["--vsync", "2"]).VSync);
        Assert.Equal(1, AppOptions.Parse(["--vsync", "--no-shadows"]).VSync);
        Assert.Throws<ArgumentException>(static () => AppOptions.Parse(["--vsync", "3"]));

        AppOptions options = AppOptions.Parse(
            ["--fps", "120", "--no-render-skip", "--skybox", "fantasy", "--tv", "1"]);

        Assert.Equal(120, options.FrameRate);
        Assert.False(options.SkipRender);
        Assert.Equal(2, options.Skybox);
        Assert.Equal(1, options.TvBox);

        // AMD-025: a command line that still asks for TV 2 or TV 3 gets the one model.
        Assert.Equal(1, AppOptions.Parse(["--tv", "3"]).TvBox);
        Assert.Equal(0, AppOptions.Parse(["--tv", "0"]).TvBox);
        Assert.Throws<ArgumentException>(static () => AppOptions.Parse(["--skybox", "nebula"]));
    }

    [Fact]
    public void REQ_SCN_061_any_frame_rate_other_than_60_90_or_120_comes_back_to_90()
    {
        RenderSettings settings = new();
        Assert.Equal(90, settings.TargetFrameRate);

        settings.TargetFrameRate = 60;
        Assert.Equal(60, settings.TargetFrameRate);
        settings.TargetFrameRate = 120;
        Assert.Equal(120, settings.TargetFrameRate);

        settings.TargetFrameRate = 144;
        Assert.Equal(90, settings.TargetFrameRate);
        settings.TargetFrameRate = 0;
        Assert.Equal(90, settings.TargetFrameRate);

        // VSync keeps the three values of SPEC-09.
        settings.VSync = 5;
        Assert.Equal(2, settings.VSync);
    }

    [Fact]
    public void REQ_SCN_060_the_scene_is_only_skipped_when_no_emulated_frame_arrived()
    {
        // Enabled, running, nothing new, same target: the previous image is kept.
        Assert.True(RenderSkip.ShouldSkip(true, gameRunning: true, 0, targetUnchanged: true));

        // A new emulated frame, a stopped game, a resize or the setting off all force a draw.
        Assert.False(RenderSkip.ShouldSkip(true, gameRunning: true, 1, targetUnchanged: true));
        Assert.False(RenderSkip.ShouldSkip(true, gameRunning: false, 0, targetUnchanged: true));
        Assert.False(RenderSkip.ShouldSkip(true, gameRunning: true, 0, targetUnchanged: false));
        Assert.False(RenderSkip.ShouldSkip(false, gameRunning: true, 0, targetUnchanged: true));
    }

    [Fact]
    public void AppOptions_rejects_what_it_cannot_parse()
    {
        Assert.Throws<ArgumentException>(static () => AppOptions.Parse(["--nope"]));
        Assert.Throws<ArgumentException>(static () => AppOptions.Parse(["--rom"]));
        Assert.Throws<ArgumentException>(static () => AppOptions.Parse(["--size", "big"]));
    }

    private static void Block(double seconds)
    {
        Stopwatch watch = Stopwatch.StartNew();
        while (watch.Elapsed.TotalSeconds < seconds)
        {
            Thread.Sleep(1);
        }
    }
}
