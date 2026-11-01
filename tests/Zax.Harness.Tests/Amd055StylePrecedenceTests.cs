using Zax.Profiles;
using Zax.Render;
using Zax.Settings;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-055 — the precedence the <c>render</c> command applies to the style: what the profile's
/// scripts left first (REQ-LUA-114), then the <c>--style</c> options, before the first rendered
/// frame. No GL context is needed: the run and the scripts happen before anything is drawn.
/// </summary>
/// <remarks>
/// The style and its twelve parameters are global user settings and never come from the profile
/// itself (REQ-SCN-078); the one thing a profile can do is write them from a script, which lands
/// in the session layer of the settings model (REQ-PERS-010). That is the layer the command line
/// has to be able to override.
/// </remarks>
public sealed class Amd055StylePrecedenceTests
{
    /// <summary>
    /// REQ-SCN-078 and REQ-LUA-114: a script that selects Enhanced and sets a parameter reaches
    /// the scene the capture renders, with no option on the command line.
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-SCN-079
    public void A_script_write_reaches_the_captured_scene()
    {
        SyntheticScene scene = CaptureHarness.Capture(CaptureHarness.WithScript("""
            function Start()
              Setting.VisualStyle = 4
              Setting:SetVisualStyleParameter(4, "spl.voxel_grid", 2)
              Setting:SetVisualStyleParameter(4, "ao.strength", 5)
            end
            """));

        StyleSettings style = scene.Scene.Settings.Style;
        Assert.True(style.Enhanced);
        Assert.Equal(2, style.VoxelGrid);
        Assert.Equal(4, style.EffectiveGridCell);
        Assert.Equal(5, style.AoStrength);
    }

    /// <summary>
    /// The alias of REQ-SCN-078: a script writing 1 selects Enhanced just as 4 does, which is
    /// what the scene then renders.
    /// </summary>
    [Fact] // REQ-SCN-078
    public void The_alias_one_selects_enhanced_from_a_script()
    {
        SyntheticScene scene = CaptureHarness.Capture(CaptureHarness.WithScript("""
            function Start() Setting.VisualStyle = 1 end
            """));

        Assert.True(scene.Scene.Settings.Style.Enhanced);
    }

    /// <summary>
    /// Without a script the style is Classic, the default of <c>render.visual_style</c>
    /// (REQ-SCN-078), and the twelve parameters are at their own defaults (REQ-SCN-079).
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-SCN-079
    public void A_bare_profile_captures_the_classic_style()
    {
        StyleSettings style = CaptureHarness.Capture(GameProfile.CreateEmpty())
            .Scene.Settings.Style;

        Assert.False(style.Enhanced);
        foreach ((string _, string key) in SettingsCatalogue.StyleParameterOrder)
        {
            Assert.Equal(SettingsCatalogue.Get(key).DefaultValue.AsInt, Read(style, key));
        }
    }

    /// <summary>
    /// The precedence of the lot: the command line overrides the value the profile's script
    /// wrote, and a parameter it does not name keeps what the script left.
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-SCN-079
    public void The_command_line_overrides_the_profile_value()
    {
        SyntheticScene scene = CaptureHarness.Capture(CaptureHarness.WithScript("""
            function Start()
              Setting.VisualStyle = 4
              Setting:SetVisualStyleParameter(4, "spl.voxel_grid", 1)
              Setting:SetVisualStyleParameter(4, "ao.strength", 5)
            end
            """));

        StyleOptions options = new();
        options.AddParameter("spl.voxel_grid=3");
        Overrides(options).ApplyTo(scene);

        StyleSettings style = scene.Scene.Settings.Style;
        Assert.Equal(3, style.VoxelGrid);

        // Named by nobody but the script: it survives the override.
        Assert.Equal(5, style.AoStrength);
        Assert.True(style.Enhanced);
    }

    /// <summary>
    /// <c>--style 0</c> wins over a script that selected Enhanced: the option is applied last.
    /// </summary>
    [Fact] // REQ-SCN-078
    public void The_style_option_overrides_the_profile_style()
    {
        SyntheticScene scene = CaptureHarness.Capture(CaptureHarness.WithScript("""
            function Start() Setting.VisualStyle = 4 end
            """));
        Assert.True(scene.Scene.Settings.Style.Enhanced);

        StyleOptions options = new();
        options.SetStyle("0");
        Overrides(options).ApplyTo(scene);

        Assert.False(scene.Scene.Settings.Style.Enhanced);
    }

    /// <summary>
    /// The other way round: the command line turns Enhanced on for a profile whose scripts never
    /// mention the style, which is what capturing an Enhanced frame headless needs.
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-SCN-081
    public void The_command_line_turns_enhanced_on_for_a_bare_profile()
    {
        SyntheticScene scene = CaptureHarness.Capture(GameProfile.CreateEmpty());

        StyleOptions options = new();
        options.SetStyle("enhanced");
        options.AddParameters("spl.voxel_grid=2;spl.pixel_shadow=1");
        Overrides(options).ApplyTo(scene);

        StyleSettings style = scene.Scene.Settings.Style;
        Assert.True(style.Enhanced);
        Assert.Equal(4, style.EffectiveGridCell);
        Assert.True(style.EffectivePixelShadow);
    }

    /// <summary>
    /// A script write made during the run, not in <c>Start</c>, reaches the scene too: the style
    /// applies at the next rendered frame without the game being reloaded (REQ-SCN-078).
    /// </summary>
    [Fact] // REQ-SCN-078
    public void A_write_made_during_the_run_reaches_the_scene()
    {
        SyntheticScene scene = CaptureHarness.Capture(
            CaptureHarness.WithScript("""
                function Update()
                  if frame == 2 then
                    Setting.VisualStyle = 4
                    Setting:SetVisualStyleParameter(4, "spl.reflections", 1)
                  end
                end
                """),
            runFrames: 6);

        StyleSettings style = scene.Scene.Settings.Style;
        Assert.True(style.Enhanced);
        Assert.True(style.EffectiveReflections);
    }

    /// <summary>
    /// The style options alone leave the framing, the shadows and the curvature of the scene
    /// where they were, which is what makes a Classic capture and an Enhanced one comparable
    /// (REQ-SCN-080).
    /// </summary>
    [Fact] // REQ-SCN-080
    public void The_style_options_leave_the_rest_of_the_scene_alone()
    {
        SyntheticScene scene = SyntheticScenes.Build("framing", Zax.Engine3D.RenderAlgorithm.Greedy);
        int bend = scene.Scene.Settings.BendLeftRight;
        bool shadows = scene.Scene.Settings.Shadows;
        float fov = scene.Scene.Settings.Fov;

        StyleOptions options = new();
        options.SetStyle("4");
        options.AddParameter("spl.ao=2");
        Overrides(options).ApplyTo(scene);

        Assert.True(scene.Scene.Settings.Style.Enhanced);
        Assert.Equal(bend, scene.Scene.Settings.BendLeftRight);
        Assert.Equal(shadows, scene.Scene.Settings.Shadows);
        Assert.Equal(fov, scene.Scene.Settings.Fov);
    }

    /// <summary>The overrides of a command line that passed nothing but the style options.</summary>
    private static SceneOverrides Overrides(StyleOptions style) => new() { Style = style };

    private static int Read(StyleSettings style, string key) => key switch
    {
        SettingKeys.RenderStyleAo => style.AmbientOcclusion,
        SettingKeys.RenderStyleAoStrength => style.AoStrength,
        SettingKeys.RenderStyleAoRadius => style.AoRadius,
        SettingKeys.RenderStyleAoThickness => style.AoThickness,
        SettingKeys.RenderStyleAoQuality => style.AoQuality,
        SettingKeys.RenderStylePixelShadow => style.PixelShadow ? 1 : 0,
        SettingKeys.RenderStyleVoxelGrid => style.VoxelGrid,
        SettingKeys.RenderStyleReflections => style.Reflections ? 1 : 0,
        SettingKeys.RenderStyleSsrShine => style.SsrShine,
        SettingKeys.RenderStyleSsrClarity => style.SsrClarity,
        SettingKeys.RenderStyleSsrLength => style.SsrLength,
        SettingKeys.RenderStyleSsrQuality => style.SsrQuality,
        _ => throw new ArgumentException($"'{key}' is not a style parameter.", nameof(key)),
    };
}
