using Zax.App;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Render;
using Zax.Settings;

namespace Zax.App.Tests;

/// <summary>
/// Lot FIX-4 (AMD-032): the three video options the application used to store without acting
/// on — the anti-aliasing of SPEC-09 REQ-PERS-017, the resolution scale of SPEC-06 REQ-SCN-063
/// and the render method of SPEC-09 §4.1.1 — and the two start-up decisions the same lot fixes.
/// </summary>
public sealed class VideoOptionTests
{
    // ------------------------------------------------------- anti-aliasing (AMD-032 §2)

    /// <summary>
    /// REQ-PERS-017 with AMD-032 §2: the stored option reaches the scene as a sample count,
    /// on the next rendered frame, with no restart.
    /// </summary>
    [Fact]
    public void REQ_PERS_017_a_change_of_anti_alias_reaches_the_scene_at_once()
    {
        using LoopHarness harness = LoopHarness.Create();
        Assert.Equal(1, harness.Scene.Settings.AntiAliasSamples);

        harness.Host.Settings.AntiAlias = 1;
        Assert.Equal(4, harness.Scene.Settings.AntiAliasSamples);

        harness.Host.Settings.AntiAlias = 2;
        Assert.Equal(SceneConstants.MaxAntiAliasSamples, harness.Scene.Settings.AntiAliasSamples);

        harness.Host.Settings.AntiAlias = 0;
        Assert.Equal(1, harness.Scene.Settings.AntiAliasSamples);
    }

    /// <summary>The option keeps the domain of SPEC-09 §4.1.1, whatever is written to it.</summary>
    [Fact]
    public void REQ_PERS_017_the_anti_alias_option_stays_in_its_domain()
    {
        AppSettings settings = new();
        Assert.Equal(0, settings.AntiAlias);

        settings.AntiAlias = 2;
        Assert.Equal(2, settings.AntiAlias);

        settings.AntiAlias = 7;
        Assert.Equal(2, settings.AntiAlias);
    }

    // ------------------------------------------------------- resolution scale (AMD-032 §3)

    /// <summary>
    /// REQ-SCN-063: with the render-texture method the scene buffer is the view scaled by 1,
    /// 2/3, 1/2, 1/3 or 1/4, the factors SPEC-09 REQ-PERS-013 point 6 derives from the index.
    /// </summary>
    [Theory]
    // index, expected width, expected height for a 1920 x 1080 view
    [InlineData(0, 1920, 1080)]
    [InlineData(1, 1280, 720)]
    [InlineData(2, 960, 540)]
    [InlineData(3, 640, 360)]
    [InlineData(4, 480, 270)]
    public void REQ_SCN_063_the_render_texture_method_scales_the_scene_buffer(
        int index,
        int expectedWidth,
        int expectedHeight)
    {
        SettingsModel model = new();
        AppSettings settings = new(model);
        settings.RenderMethod = RenderScaling.RenderTextureMethod;
        model.Set(SettingKeys.VideoResScale, SettingValue.OfEnum(index));

        (int width, int height) = RenderScaling.SceneSize(
            1920, 1080, settings.RenderMethod, settings.ResScaleFactor);

        Assert.Equal(expectedWidth, width);
        Assert.Equal(expectedHeight, height);
    }

    /// <summary>
    /// AMD-032 §3: the direct method draws at the native resolution, so the index has no effect
    /// at all — which is why the options window greys the row out.
    /// </summary>
    [Theory]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(2)]
    [InlineData(3)]
    [InlineData(4)]
    public void The_direct_method_ignores_the_resolution_scale(int index)
    {
        SettingsModel model = new();
        AppSettings settings = new(model);
        settings.RenderMethod = RenderScaling.DirectMethod;
        model.Set(SettingKeys.VideoResScale, SettingValue.OfEnum(index));

        Assert.False(RenderScaling.ScaleApplies(settings.RenderMethod));
        Assert.Equal(
            (1920, 1080),
            RenderScaling.SceneSize(1920, 1080, settings.RenderMethod, settings.ResScaleFactor));
    }

    /// <summary>A view too small to divide still yields a buffer that can be rendered into.</summary>
    [Fact]
    public void The_scaled_buffer_never_collapses_to_nothing()
    {
        Assert.Equal(
            (1, 1),
            RenderScaling.SceneSize(2, 2, RenderScaling.RenderTextureMethod, 0.25f));
        Assert.Equal(
            (1, 1),
            RenderScaling.SceneSize(0, -4, RenderScaling.RenderTextureMethod, 1f));
    }

    // ------------------------------------------------------- console attach (AMD-032 §5)

    /// <summary>
    /// AMD-032 §5: only the three options that print and exit borrow the terminal's console. A
    /// normal launch must not, or the window shares the shell it was started from.
    /// </summary>
    [Theory]
    [InlineData("--help")]
    [InlineData("-h")]
    [InlineData("--list-profiles")]
    public void Only_the_options_that_print_and_exit_attach_to_the_parent_console(string argument)
        => Assert.True(Program.PrintsAndExits(AppOptions.Parse([argument])));

    /// <summary>The import of AMD-030 §2 prints and exits too.</summary>
    [Fact]
    public void The_3dsen_import_attaches_to_the_parent_console()
        => Assert.True(Program.PrintsAndExits(AppOptions.Parse(["--import-3dsen", "C:/3dSen"])));

    /// <summary>Every launch that opens a window leaves the terminal alone.</summary>
    [Theory]
    [InlineData("")]
    [InlineData("--no-audio")]
    [InlineData("--smoke-test")]
    [InlineData("--vsync 0")]
    public void A_launch_that_opens_a_window_does_not_attach(string commandLine)
        => Assert.False(Program.PrintsAndExits(AppOptions.Parse(Split(commandLine))));

    // ------------------------------------------------------- full screen at start-up (§6)

    /// <summary>
    /// SPEC-09 REQ-PERS-067 with AMD-032 §6: the window opens in the state the settings hold,
    /// so the window and the interface never disagree at start-up.
    /// </summary>
    [Theory]
    [InlineData(false, false)]
    [InlineData(true, true)]
    public void REQ_PERS_067_the_window_opens_in_the_persisted_state(bool persisted, bool expected)
        => Assert.Equal(expected, ZaxApplication.ShouldOpenFullscreen(AppOptions.Parse([]), persisted));

    /// <summary>
    /// The two capture runs stay windowed whatever the settings say: their PNGs must keep the
    /// size the command line asked for.
    /// </summary>
    [Theory]
    [InlineData("--smoke-test")]
    [InlineData("--ui-shots shots")]
    public void A_capture_run_stays_windowed(string commandLine)
        => Assert.False(
            ZaxApplication.ShouldOpenFullscreen(AppOptions.Parse(Split(commandLine)), persisted: true));

    /// <summary>
    /// AMD-032 §6: the platform layer carries the state into the window options, which is what
    /// used to be missing — the setting reached the interface and never the window.
    /// </summary>
    [Fact]
    public void REQ_PERS_067_the_window_settings_carry_the_full_screen_state()
    {
        Zax.Platform.HostWindowSettings windowed = new("Zaxnes", 1280, 720, VSync: 1);
        Assert.False(windowed.Fullscreen);

        Zax.Platform.HostWindowSettings full = windowed with { Fullscreen = true };
        Assert.True(full.Fullscreen);
        Assert.Equal(1280, full.Width);
    }

    /// <summary>A command line written as one string, since xUnit splats an array argument.</summary>
    private static string[] Split(string commandLine)
        => commandLine.Split(' ', StringSplitOptions.RemoveEmptyEntries);
}
