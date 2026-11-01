using Zax.Render;
using Zax.Settings;

namespace Zax.Harness.Tests;

/// <summary>
/// AMD-055 — the style options of the <c>render</c> command: <c>--style</c>,
/// <c>--style-param</c> and <c>--style-params</c>, which select the visual style of
/// REQ-SCN-078 and the twelve parameters of REQ-SCN-079 from the command line.
/// </summary>
public sealed class Amd055StyleOptionTests
{
    // ------------------------------------------------------------------ --style

    /// <summary>
    /// REQ-SCN-078: the identifiers the setting accepts in writing — 0 Classic, 4 Enhanced and
    /// 1 as an alias of Enhanced, which is read back as 4.
    /// </summary>
    [Theory] // REQ-SCN-078
    [InlineData("0", SettingsCatalogue.ClassicStyle)]
    [InlineData("4", SettingsCatalogue.EnhancedStyle)]
    [InlineData("1", SettingsCatalogue.EnhancedStyle)]
    [InlineData("classic", SettingsCatalogue.ClassicStyle)]
    [InlineData("Enhanced", SettingsCatalogue.EnhancedStyle)]
    public void The_style_option_takes_the_identifiers_of_the_setting(string value, int expected)
    {
        StyleOptions options = new();
        options.SetStyle(value);

        Assert.Equal(expected, options.Style);
        Assert.Equal(expected == SettingsCatalogue.EnhancedStyle, options.Enhanced);
    }

    /// <summary>
    /// A value the domain does not list is refused rather than silently put back to Classic:
    /// a capture that asked for a style has to get it or fail (REQ-SCN-078 writes 0 for any
    /// other value, which would make a wrong capture look right).
    /// </summary>
    [Theory] // REQ-SCN-078
    [InlineData("2")]
    [InlineData("7")]
    [InlineData("-1")]
    [InlineData("enhancedd")]
    [InlineData("")]
    public void An_unknown_style_identifier_is_refused(string value)
    {
        StyleOptions options = new();

        ArgumentException error = Assert.Throws<ArgumentException>(() => options.SetStyle(value));
        Assert.Contains("--style", error.Message, StringComparison.Ordinal);
        Assert.Null(options.Style);
    }

    /// <summary>Without the option the style is left alone, and nothing is applied.</summary>
    [Fact] // REQ-SCN-078
    public void No_style_option_leaves_the_scene_alone()
    {
        StyleSettings style = new() { Enhanced = true, VoxelGrid = 2 };
        StyleOptions options = new();

        Assert.True(options.IsEmpty);
        options.ApplyTo(style);

        Assert.True(style.Enhanced);
        Assert.Equal(2, style.VoxelGrid);
    }

    // ------------------------------------------------------------ --style-param

    /// <summary>
    /// REQ-SCN-079: the twelve identifiers of the table are the keys the option takes, and each
    /// of them reaches the style block of the scene.
    /// </summary>
    [Fact] // REQ-SCN-079
    public void The_twelve_parameters_of_the_table_are_all_accepted()
    {
        foreach ((string id, string key) in SettingsCatalogue.StyleParameterOrder)
        {
            StyleOptions options = new();
            int value = (int)SettingsCatalogue.Get(key).Domain.Maximum!.Value;
            options.AddParameter($"{id}={value}");

            Assert.True(options.Parameters.ContainsKey(key), id);
            Assert.Equal(value, options.Parameters[key]);
        }

        Assert.Equal(12, SettingsCatalogue.StyleParameterOrder.Count);
    }

    /// <summary>
    /// The four effect switches of the amendment, each applied onto the style block the renderer
    /// reads (REQ-SCN-081 to REQ-SCN-084). The identifiers are the ones of the table of
    /// REQ-SCN-079: the reflections are <c>spl.reflections</c> and the grid is
    /// <c>spl.voxel_grid</c>.
    /// </summary>
    [Fact] // REQ-SCN-079, REQ-SCN-081, REQ-SCN-082, REQ-SCN-084
    public void The_four_effect_switches_reach_the_style_block()
    {
        StyleSettings style = new();
        StyleOptions options = new();
        options.SetStyle("4");
        options.AddParameter("spl.ao=2");
        options.AddParameter("spl.pixel_shadow=1");
        options.AddParameter("spl.reflections=1");
        options.AddParameter("spl.voxel_grid=3");
        options.ApplyTo(style);

        Assert.True(style.Enhanced);
        Assert.Equal(2, style.AmbientOcclusion);
        Assert.True(style.PixelShadow);
        Assert.True(style.Reflections);
        Assert.Equal(3, style.VoxelGrid);
        Assert.Equal(8, style.EffectiveGridCell);
    }

    /// <summary>The stable <c>render.style.*</c> key of AMD-055 §4 is accepted as well.</summary>
    [Fact] // REQ-SCN-079
    public void A_stable_key_is_accepted_beside_the_lua_identifier()
    {
        StyleOptions options = new();
        options.AddParameter($"{SettingKeys.RenderStyleAoStrength}=5");

        Assert.Equal(5, options.Parameters[SettingKeys.RenderStyleAoStrength]);
    }

    /// <summary>An identifier that is not a style parameter is refused, with the list.</summary>
    [Theory] // REQ-SCN-079
    [InlineData("nope")]
    [InlineData("ao.force")]
    [InlineData("spl.ambient_occlusion")]
    public void An_unknown_parameter_is_refused(string id)
    {
        StyleOptions options = new();

        ArgumentException error = Assert.Throws<ArgumentException>(
            () => options.AddParameter($"{id}=1"));
        Assert.Contains(id, error.Message, StringComparison.Ordinal);
        Assert.Contains("spl.voxel_grid", error.Message, StringComparison.Ordinal);
        Assert.Empty(options.Parameters);
    }

    /// <summary>
    /// The curvature parameters of AMD-043 go through the same Lua method (REQ-LUA-114) but are
    /// not effects of the style (REQ-SCN-078): the option refuses them and names <c>--bend</c>.
    /// </summary>
    [Theory] // REQ-SCN-078
    [InlineData("spl.bend_zx")]
    [InlineData("spl.arc_yx")]
    public void A_curvature_parameter_is_refused_and_points_at_bend(string id)
    {
        StyleOptions options = new();

        ArgumentException error = Assert.Throws<ArgumentException>(
            () => options.AddParameter($"{id}=30"));
        Assert.Contains("--bend", error.Message, StringComparison.Ordinal);
    }

    /// <summary>
    /// A value the parameter's domain does not hold is refused rather than rounded: REQ-SCN-079
    /// normalises a write, which would turn a mistyped capture into a silently different one.
    /// </summary>
    [Theory] // REQ-SCN-079
    [InlineData("spl.voxel_grid=4")]
    [InlineData("spl.voxel_grid=-1")]
    [InlineData("ao.strength=0")]
    [InlineData("ao.strength=9")]
    [InlineData("ao.strength=1.4")]
    [InlineData("spl.ao=3")]
    [InlineData("ssr.quality=4")]
    [InlineData("spl.pixel_shadow=2")]
    public void An_out_of_range_value_is_refused(string assignment)
    {
        StyleOptions options = new();

        ArgumentException error = Assert.Throws<ArgumentException>(
            () => options.AddParameter(assignment));
        Assert.Contains("--style-param", error.Message, StringComparison.Ordinal);
        Assert.Empty(options.Parameters);
    }

    /// <summary>A malformed assignment is refused.</summary>
    [Theory] // REQ-SCN-079
    [InlineData("spl.voxel_grid")]
    [InlineData("=2")]
    [InlineData("spl.voxel_grid=")]
    [InlineData("")]
    public void A_malformed_assignment_is_refused(string assignment)
        => Assert.Throws<ArgumentException>(() => new StyleOptions().AddParameter(assignment));

    /// <summary>Repeating the option for one key keeps the last value.</summary>
    [Fact] // REQ-SCN-079
    public void A_repeated_parameter_keeps_the_last_value()
    {
        StyleOptions options = new();
        options.AddParameter("ao.strength=2");
        options.AddParameter("ao.radius=4");
        options.AddParameter("ao.strength=5");

        Assert.Equal(5, options.Parameters[SettingKeys.RenderStyleAoStrength]);
        Assert.Equal(4, options.Parameters[SettingKeys.RenderStyleAoRadius]);
        Assert.Equal(2, options.Parameters.Count);
    }

    /// <summary>The single-argument form takes a list separated by semicolons or commas.</summary>
    [Theory] // REQ-SCN-079
    [InlineData("spl.ao=2;ao.strength=5")]
    [InlineData("spl.ao=2,ao.strength=5")]
    [InlineData(" spl.ao = 2 ; ; ao.strength = 5 ")]
    public void The_list_form_sets_several_parameters(string list)
    {
        StyleOptions options = new();
        options.AddParameters(list);

        Assert.Equal(2, options.Parameters[SettingKeys.RenderStyleAo]);
        Assert.Equal(5, options.Parameters[SettingKeys.RenderStyleAoStrength]);
    }

    /// <summary>One bad entry of the list refuses the whole option.</summary>
    [Fact] // REQ-SCN-079
    public void A_bad_entry_of_the_list_is_refused()
        => Assert.Throws<ArgumentException>(
            () => new StyleOptions().AddParameters("spl.ao=2;ao.strength=9"));

    // ---------------------------------------------------------------- precedence

    /// <summary>
    /// The precedence of the command: a parameter the command line names overrides what the
    /// scene already carries — what the profile's scripts wrote (REQ-LUA-114) — and a parameter
    /// it does not name is left exactly as it was.
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-SCN-079
    public void A_named_parameter_overrides_the_scene_and_the_others_survive()
    {
        StyleSettings style = new()
        {
            Enhanced = true,
            VoxelGrid = 1,
            AoStrength = 2,
            PixelShadow = true,
            SsrQuality = 0,
        };

        StyleOptions options = new();
        options.AddParameter("spl.voxel_grid=3");
        options.ApplyTo(style);

        Assert.Equal(3, style.VoxelGrid);

        // Untouched: the three the command line never named, and the style itself.
        Assert.True(style.Enhanced);
        Assert.Equal(2, style.AoStrength);
        Assert.True(style.PixelShadow);
        Assert.Equal(0, style.SsrQuality);
    }

    /// <summary>
    /// <c>--style 0</c> goes back to Classic even on a scene a script put into Enhanced, the
    /// parameters staying where they were (they are persisted and exposed without effect,
    /// REQ-SCN-079).
    /// </summary>
    [Fact] // REQ-SCN-078
    public void The_style_option_can_go_back_to_classic()
    {
        StyleSettings style = new() { Enhanced = true, VoxelGrid = 3 };
        StyleOptions options = new();
        options.SetStyle("0");
        options.ApplyTo(style);

        Assert.False(style.Enhanced);
        Assert.Equal(3, style.VoxelGrid);
        Assert.Equal(0, style.EffectiveGridCell);
    }

    // --------------------------------------------------------------- the report

    /// <summary>What the console line of the capture says, in the order of the table.</summary>
    [Fact] // REQ-SCN-079
    public void The_description_lists_the_style_then_the_parameters_in_order()
    {
        StyleOptions options = new();
        options.AddParameter("ssr.shine=5");
        options.AddParameter("spl.ao=2");
        options.SetStyle("1");

        Assert.Equal("Enhanced (4), spl.ao=2, ssr.shine=5", options.Describe());
        Assert.Equal(string.Empty, new StyleOptions().Describe());
    }

    /// <summary>
    /// The style block <c>render.json</c> carries: the style in force and the twelve parameters
    /// by their Lua identifier, so a headless capture states what produced it.
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-SCN-079
    public void The_json_block_states_the_style_and_its_twelve_parameters()
    {
        StyleSettings style = new() { Enhanced = true, VoxelGrid = 2, Reflections = true };
        string json = StyleOptions.Json(style);

        Assert.Contains("\"visualStyle\": 4", json, StringComparison.Ordinal);
        Assert.Contains("\"spl.voxel_grid\": 2", json, StringComparison.Ordinal);
        Assert.Contains("\"spl.reflections\": 1", json, StringComparison.Ordinal);
        Assert.Contains("\"ao.quality\": 1", json, StringComparison.Ordinal);
        foreach (string id in StyleOptions.AcceptedParameters)
        {
            Assert.Contains($"\"{id}\"", json, StringComparison.Ordinal);
        }

        Assert.Contains(
            "\"visualStyle\": 0",
            StyleOptions.Json(new StyleSettings()),
            StringComparison.Ordinal);
    }
}
