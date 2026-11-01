using System.Numerics;
using Zax.App.Persistence;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-055 — the « Enhanced » visual style, the part that needs no GL context: the style
/// identifiers of REQ-SCN-078, the twelve parameters of REQ-SCN-079 with their normalisation,
/// their persistence and the session layer a script write lands in.
/// </summary>
public sealed class Amd055Tests
{
    /// <summary>
    /// REQ-SCN-078, first criterion, line by line: <c>Setting.VisualStyle = 4</c> reads back 4,
    /// <c>= 1</c> reads back 4, <c>SetVisualStyleParameter(4, "spl.bend_zx", 30)</c> is true,
    /// <c>SetVisualStyleParameter(1, "ao.strength", 5)</c> is true, <c>= 2</c> reads back 0.
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-LUA-114
    public void REQ_SCN_078_the_style_identifiers_are_4_and_1()
    {
        using LuaRun run = new LuaProfile().Script("probe", string.Empty).Start();

        run.Do("Setting.VisualStyle = 4");
        Assert.Equal(4d, run.Num("Setting.VisualStyle"));

        run.Do("Setting.VisualStyle = 0");
        run.Do("Setting.VisualStyle = 1");
        Assert.Equal(4d, run.Num("Setting.VisualStyle"));

        // Both identifiers reach the curvature parameters and the twelve new ones alike.
        Assert.True(run.Flag("""Setting:SetVisualStyleParameter(4, "spl.bend_zx", 30)"""));
        Assert.True(run.Flag("""Setting:SetVisualStyleParameter(1, "ao.strength", 5)"""));
        Assert.Equal(30d, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));
        Assert.Equal(5d, run.Num("""Setting:GetVisualStyleParameter(4, "ao.strength")"""));

        run.Do("Setting.VisualStyle = 2");
        Assert.Equal(0d, run.Num("Setting.VisualStyle"));

        // Any other style identifier stays unknown, as AMD-040 §3 has it.
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(2, "ao.strength", 5)"""));
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(0, "ao.strength", 5)"""));
    }

    /// <summary>
    /// REQ-SCN-078, first criterion, last line: a settings file holding
    /// <c>render.visual_style = 1</c> reads back 4, and <c>= 7</c> reads back 0 — the rule of
    /// the « Domain » column of §4 applies to the file as it does to a write
    /// (SPEC-09 REQ-PERS-003).
    /// </summary>
    [Fact] // REQ-SCN-078, REQ-PERS-003
    public void REQ_SCN_078_a_settings_file_normalises_the_style_like_a_write()
    {
        Assert.Equal(4, Reloaded("""{ "schemaVersion": 1, "values": { "render.visual_style": 1 } }"""));
        Assert.Equal(0, Reloaded("""{ "schemaVersion": 1, "values": { "render.visual_style": 7 } }"""));
        Assert.Equal(4, Reloaded("""{ "schemaVersion": 1, "values": { "render.visual_style": 4 } }"""));
        Assert.Equal(0, Reloaded("""{ "schemaVersion": 1, "values": { "render.visual_style": 0 } }"""));
    }

    /// <summary>
    /// REQ-SCN-079, first criterion: every one of the twelve parameters, written from the
    /// settings model the interface writes through, survives a restart; and the normalisation of
    /// AMD-040 §3 — bounded, then rounded to the step of 1 — gives <c>ao.strength</c> 9 → 5,
    /// 1.4 → 1 and 0 → 1, <c>spl.voxel_grid</c> 4 → 3 and <c>spl.ao</c> 1.4 → 1.
    /// </summary>
    [Fact] // REQ-SCN-079, REQ-PERS-003
    public void REQ_SCN_079_the_twelve_parameters_are_normalised_and_persisted()
    {
        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);

        // A value inside the domain of each parameter, different from its default, so that a
        // default read back would fail the test.
        Dictionary<string, int> notDefault = new(StringComparer.Ordinal)
        {
            ["spl.ao"] = 2,
            ["ao.strength"] = 5,
            ["ao.radius"] = 1,
            ["ao.thickness"] = 5,
            ["ao.quality"] = 2,
            ["spl.pixel_shadow"] = 1,
            ["spl.voxel_grid"] = 3,
            ["spl.reflections"] = 1,
            ["ssr.shine"] = 1,
            ["ssr.clarity"] = 5,
            ["ssr.length"] = 1,
            ["ssr.quality"] = 0,
        };

        Dictionary<string, int> written = [];
        foreach ((string id, string key) in SettingsCatalogue.StyleParameterOrder)
        {
            int wanted = notDefault[id];
            Assert.NotEqual(wanted, settings.StyleParameter(key));
            settings.Model.SetInt(key, wanted);
            Assert.Equal(wanted, settings.StyleParameter(key));
            written[key] = wanted;
        }

        Assert.Equal(notDefault.Count, written.Count);

        store.Save(settings);

        AppSettings restarted = new();
        new SettingsStore(root.Paths).Load(restarted, DateTimeOffset.UnixEpoch);
        foreach ((string key, int wanted) in written)
        {
            Assert.Equal(wanted, restarted.StyleParameter(key));
        }

        // The normalisation lines of the criterion, read on a fresh model.
        SettingsModel model = new();
        Assert.Equal(5, Normalised(model, SettingKeys.RenderStyleAoStrength, 9d));
        Assert.Equal(1, Normalised(model, SettingKeys.RenderStyleAoStrength, 1.4d));
        Assert.Equal(1, Normalised(model, SettingKeys.RenderStyleAoStrength, 0d));
        Assert.Equal(3, Normalised(model, SettingKeys.RenderStyleVoxelGrid, 4d));
        Assert.Equal(1, Normalised(model, SettingKeys.RenderStyleAo, 1.4d));
    }

    /// <summary>
    /// REQ-SCN-079, second criterion: a write <b>by Lua</b> follows SPEC-09 REQ-PERS-010 — it
    /// lands in the session layer, so the user's own preference comes back at the end of the
    /// game and was never persisted.
    /// </summary>
    [Fact] // REQ-SCN-079, REQ-PERS-010
    public void REQ_SCN_079_a_lua_write_is_a_session_value()
    {
        using LuaRun run = new LuaProfile().Script("probe", string.Empty).Start();
        SettingsModel model = run.Settings.Model;

        // The user's preference, written the way the interface writes it.
        model.SetInt(SettingKeys.RenderStyleAoStrength, 2);

        Assert.True(run.Flag("""Setting:SetVisualStyleParameter(4, "ao.strength", 5)"""));
        Assert.Equal(5, model.GetInt(SettingKeys.RenderStyleAoStrength));
        Assert.True(model.HasSessionValue(SettingKeys.RenderStyleAoStrength));
        Assert.Contains(
            model.UserPreferences(),
            pair => pair.Key == SettingKeys.RenderStyleAoStrength && (int)pair.Value.AsFloat == 2);

        model.EndOfGame();
        Assert.Equal(2, model.GetInt(SettingKeys.RenderStyleAoStrength));
        Assert.False(model.HasSessionValue(SettingKeys.RenderStyleAoStrength));
    }

    /// <summary>
    /// REQ-SCN-079: the twelve identifiers and their defaults, which are the arguments of the
    /// Lua API and so facts of interoperability. Eight of the twelve have a non-zero default,
    /// which is why AMD-055 §3 point 2 restricts the « a script does not tell unknown from zero »
    /// note of AMD-040 §2 to the four curvature parameters.
    /// </summary>
    [Fact] // REQ-SCN-079, REQ-LUA-114
    public void REQ_SCN_079_the_twelve_identifiers_carry_the_defaults_of_the_table()
    {
        using LuaRun run = new LuaProfile().Script("probe", string.Empty).Start();

        (string Id, int Default)[] expected =
        [
            ("spl.ao", 0),
            ("ao.strength", 3),
            ("ao.radius", 3),
            ("ao.thickness", 3),
            ("ao.quality", 1),
            ("spl.pixel_shadow", 0),
            ("spl.voxel_grid", 0),
            ("spl.reflections", 0),
            ("ssr.shine", 3),
            ("ssr.clarity", 3),
            ("ssr.length", 3),
            ("ssr.quality", 2),
        ];

        foreach ((string id, int value) in expected)
        {
            Assert.Equal(value, run.Num($"""Setting:GetVisualStyleParameter(4, "{id}")"""));
        }

        Assert.Equal(8, expected.Count(static pair => pair.Default != 0));

        // The four curvature parameters of AMD-043 are reached by the same style identifier and
        // keep their zero default, so the note of AMD-040 §2 still holds for them.
        foreach (string id in (string[])["spl.bend_zx", "spl.bend_zy", "spl.arc_yx", "spl.arc_center"])
        {
            Assert.Equal(0d, run.Num($"""Setting:GetVisualStyleParameter(4, "{id}")"""));
        }
    }

    /// <summary>
    /// What a setting reads back after a raw value has been written to it. The return of
    /// <see cref="SettingsModel.SetFloat"/> says whether the value <b>changed</b>, not whether
    /// it was accepted, so only the read-back is asserted on.
    /// </summary>
    private static int Normalised(SettingsModel model, string key, double value)
    {
        model.SetFloat(key, (float)value);
        return model.GetInt(key);
    }

    private static int Reloaded(string json)
    {
        using TemporaryRoot root = new();
        File.WriteAllText(root.Paths.SettingsFile, json);

        AppSettings settings = new();
        new SettingsStore(root.Paths).Load(settings, DateTimeOffset.UnixEpoch);
        return settings.VisualStyle;
    }
}
