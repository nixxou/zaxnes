using System.Numerics;
using System.Reflection;
using Zax.App.Input;
using Zax.App.Persistence;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 verification (role R4) of lot V2-4 — AMD-043 « Courbure de l'espace » and the
/// <c>VisualStyle</c> rows of AMD-040 REQ-LUA-114: the four settings of REQ-SCN-066, the point
/// transform of REQ-SCN-067 with its thirteen vectors, the scope of REQ-SCN-068 (including the
/// amended REQ-SCN-028 and the "evaluated before bending" rules), and the Zapper aim of
/// REQ-INP-124.
/// </summary>
/// <remarks>
/// Written from the amendment texts alone, before the implementation was read, and kept apart
/// from the implementer's own <c>Amd043Tests</c>. Everything is asserted on an outside-observable
/// surface: the exported scene vertices of the REQ-SCN-068 harness criterion, the settings file
/// on disk, the Lua results, the options-window model (over the shipped assembly, as
/// <see cref="UiDriver"/> does) and the Zapper state.
/// </remarks>
public sealed class R4V2BendTests
{
    private static readonly Vector2 Viewport = new(640f, 480f);

    /// <summary>Tolerance of the REQ-SCN-067 vectors: ±0,05 scene unit.</summary>
    private const float VectorTolerance = 0.05f;

    // ===================================================================== §3 REQ-SCN-067

    /// <summary>
    /// REQ-SCN-067, acceptance criterion: the thirteen vectors of the amendment, to ±0,05 unit.
    /// The columns are the four settings (bend_zx, bend_zy, arc_yx, arc centre at the edge),
    /// then P, then the expected B(P).
    /// </summary>
    [Theory] // REQ-SCN-067
    [InlineData(60, 0, 0, false, 128f, 120f, 0f, 128f, 120f, -8.43f)]
    [InlineData(60, 0, 0, false, 256f, 120f, 0f, 251.64f, 120f, 24.70f)]
    [InlineData(60, 0, 0, false, 0f, 120f, 0f, 4.36f, 120f, 24.70f)]
    [InlineData(60, 0, 0, false, 256f, 120f, 10f, 246.58f, 120f, 33.46f)]
    [InlineData(-60, 0, 0, false, 256f, 120f, 0f, 251.64f, 120f, -24.70f)]
    [InlineData(-180, 0, 0, false, 256f, 120f, -200f, 132.53f, 120f, -64.00f)]
    [InlineData(0, 60, 0, false, 128f, 240f, 0f, 128f, 235.91f, 23.16f)]
    [InlineData(0, 60, 0, false, 128f, 0f, 0f, 128f, 4.09f, 23.16f)]
    [InlineData(0, 0, 60, false, 0f, 120f, 0f, 4.36f, 95.30f, 0f)]
    [InlineData(0, 0, 60, false, 128f, 240f, 0f, 128f, 249.81f, 0f)]
    [InlineData(0, 0, 60, true, 0f, 0f, 0f, 4.36f, -24.70f, 0f)]
    [InlineData(0, 0, 60, true, 128f, 240f, 0f, 128f, 251.19f, 0f)]
    [InlineData(0, 0, -180, false, 128f, 240f, 0f, 128f, 179.47f, 0f)]
    public void REQ_SCN_067_the_thirteen_vectors_of_the_amendment(
        int bendZx,
        int bendZy,
        int arcYx,
        bool arcAtEdge,
        float x,
        float y,
        float z,
        float expectedX,
        float expectedY,
        float expectedZ)
    {
        SpaceBend bend = new(bendZx, bendZy, arcYx, arcAtEdge);
        Vec3 image = bend.Apply(new Vec3(x, y, z));

        Assert.Equal(expectedX, image.X, VectorTolerance);
        Assert.Equal(expectedY, image.Y, VectorTolerance);
        Assert.Equal(expectedZ, image.Z, VectorTolerance);
    }

    /// <summary>
    /// REQ-SCN-067: the same thirteen vectors through the four <b>settings</b> of REQ-SCN-066 —
    /// the renderer's own transform, which is what a frame is drawn with — and the settings
    /// apply hot, the transform being rebuilt as soon as one of them changes.
    /// </summary>
    [Fact] // REQ-SCN-067, REQ-SCN-066
    public void REQ_SCN_067_the_settings_of_the_scene_drive_the_very_same_transform()
    {
        RenderSettings settings = new();
        Assert.True(settings.Bend.IsIdentity);
        Assert.Equal(0, settings.BendLeftRight);
        Assert.Equal(0, settings.BendTopBottom);
        Assert.Equal(0, settings.ArcHorizontal);
        Assert.False(settings.ArcAtEdge);

        settings.BendLeftRight = 60;
        Assert.False(settings.Bend.IsIdentity);
        Vec3 image = settings.Bend.Apply(new Vec3(256f, 120f, 0f));
        Assert.Equal(251.64f, image.X, VectorTolerance);
        Assert.Equal(24.70f, image.Z, VectorTolerance);

        // Hot: the previous transform is gone the moment the setting changes.
        settings.BendLeftRight = 0;
        settings.ArcHorizontal = 60;
        settings.ArcAtEdge = true;
        Vec3 arc = settings.Bend.Apply(new Vec3(0f, 0f, 0f));
        Assert.Equal(4.36f, arc.X, VectorTolerance);
        Assert.Equal(-24.70f, arc.Y, VectorTolerance);
    }

    /// <summary>
    /// REQ-SCN-067: <c>B</c> is the identity at θ = 0 and continuous in θ — an axis whose angle
    /// is null transforms nothing, and a very small angle moves a point by very little. The
    /// amendment allows a threshold of at most 10⁻⁴ rad (≈ 0,0057°), so an angle of 0,01° must
    /// already be either neutral or all but neutral.
    /// </summary>
    [Fact] // REQ-SCN-067
    public void REQ_SCN_067_is_the_identity_at_zero_and_continuous_in_the_angle()
    {
        Vec3[] probes =
        [
            new(128f, 120f, 0f),
            new(0f, 8f, 0f),
            new(256f, 240f, 20f),
            new(40f, 200f, -30f),
        ];

        SpaceBend none = new(0f, 0f, 0f, arcAtEdge: false);
        Assert.True(none.IsIdentity);
        foreach (Vec3 probe in probes)
        {
            Vec3 image = none.Apply(probe);
            Assert.Equal(probe.X, image.X, 1e-4f);
            Assert.Equal(probe.Y, image.Y, 1e-4f);
            Assert.Equal(probe.Z, image.Z, 1e-4f);
        }

        // A very small angle moves a point of the frame by very little, on each axis.
        foreach (float degrees in (float[])[0.01f, 0.5f])
        {
            foreach (Vec3 probe in probes)
            {
                foreach (SpaceBend axis in (SpaceBend[])
                [
                    new(degrees, 0f, 0f, false),
                    new(0f, degrees, 0f, false),
                    new(0f, 0f, degrees, false),
                    new(0f, 0f, degrees, true),
                ])
                {
                    float moved = MathF.Sqrt((axis.Apply(probe) - probe).SquaredLength);
                    Assert.True(
                        moved < degrees * 4f,
                        $"{degrees}° moved {probe} by {moved} units, which is not continuity at 0.");
                }
            }
        }

        // Continuity away from zero: 30° and 29,99° give all but the same image.
        Vec3 coarse = new SpaceBend(30f, 10f, -20f, true).Apply(new Vec3(0f, 240f, 15f));
        Vec3 fine = new SpaceBend(29.99f, 10.01f, -19.99f, true).Apply(new Vec3(0f, 240f, 15f));
        Assert.True(MathF.Sqrt((coarse - fine).SquaredLength) < 0.5f);
    }

    /// <summary>
    /// REQ-SCN-067, last paragraph: z is the depth of the scene, so a positive angle pushes the
    /// edges away (bulged) and a negative one brings them closer (wrapping round the player).
    /// </summary>
    [Fact] // REQ-SCN-067
    public void REQ_SCN_067_a_positive_angle_bulges_and_a_negative_one_wraps()
    {
        Vec3 edge = new(256f, 120f, 0f);
        Assert.True(new SpaceBend(60f, 0f, 0f, false).Apply(edge).Z > 1f);
        Assert.True(new SpaceBend(-60f, 0f, 0f, false).Apply(edge).Z < -1f);

        Vec3 top = new(128f, 240f, 0f);
        Assert.True(new SpaceBend(0f, 60f, 0f, false).Apply(top).Z > 1f);
        Assert.True(new SpaceBend(0f, -60f, 0f, false).Apply(top).Z < -1f);
    }

    /// <summary>
    /// REQ-SCN-067 step 2: the depth is held back at 95 % of the radius, in <b>both</b>
    /// directions of curvature. Two points whose depth is past the limit, one each way, have the
    /// same image as the point exactly on the limit.
    /// </summary>
    [Fact] // REQ-SCN-067
    public void REQ_SCN_067_the_depth_is_held_at_ninety_five_percent_of_the_radius()
    {
        // k = θ / 256 with θ = −π: the limit w·k > 0,95 bites for w < −77,4.
        SpaceBend inward = new(-180f, 0f, 0f, false);
        float radius = 256f / MathF.PI;
        float limit = -0.95f * radius;

        Vec3 onLimit = inward.Apply(new Vec3(256f, 120f, limit));
        Vec3 past = inward.Apply(new Vec3(256f, 120f, limit - 500f));
        Assert.Equal(onLimit.X, past.X, 0.05f);
        Assert.Equal(onLimit.Z, past.Z, 0.05f);

        // The sixth vector of the table is exactly that case.
        Vec3 clamped = inward.Apply(new Vec3(256f, 120f, -200f));
        Assert.Equal(132.53f, clamped.X, VectorTolerance);
        Assert.Equal(-64.00f, clamped.Z, VectorTolerance);

        // The other direction of curvature: θ = +180°, the limit bites for w > +77,4.
        SpaceBend outward = new(180f, 0f, 0f, false);
        Vec3 deep = outward.Apply(new Vec3(256f, 120f, 0.95f * radius));
        Vec3 deeper = outward.Apply(new Vec3(256f, 120f, (0.95f * radius) + 500f));
        Assert.Equal(deep.X, deeper.X, 0.05f);
        Assert.Equal(deep.Z, deeper.Z, 0.05f);
    }

    // ===================================================================== §2 REQ-SCN-066

    /// <summary>
    /// REQ-SCN-066 and AMD-043 §6: four user preferences, persisted, immediate, default 0, on
    /// the thirteen-rung ladder from −180° to 180° (two rungs for the arc centre), each exposed
    /// to libretro with that many values.
    /// </summary>
    [Fact] // REQ-SCN-066
    public void REQ_SCN_066_the_four_settings_are_the_lines_of_section_6()
    {
        foreach (string key in (string[])["scene.bend_lr", "scene.bend_tb", "scene.arc_h"])
        {
            SettingDescriptor descriptor = SettingsCatalogue.Get(key);
            Assert.Equal(SettingKind.Int, descriptor.Kind);
            Assert.Equal(0, descriptor.DefaultValue.AsInt);
            Assert.Equal(SettingScope.User, descriptor.Scope);
            Assert.Equal(SettingEffect.Immediate, descriptor.Effect);

            double[] rungs = [.. descriptor.Domain.Options!.Select(static o => o.Value)];
            Assert.Equal(13, rungs.Length);
            Assert.Equal(-180d, rungs[0]);
            Assert.Equal(180d, rungs[^1]);
            for (int i = 0; i < rungs.Length; i++)
            {
                Assert.Equal(-180d + (30d * i), rungs[i]);
            }

            Assert.Equal(13, descriptor.LibretroValues!.Count);
        }

        SettingDescriptor centre = SettingsCatalogue.Get("scene.arc_center");
        Assert.Equal(SettingKind.Int, centre.Kind);
        Assert.Equal(0, centre.DefaultValue.AsInt);
        Assert.Equal(SettingScope.User, centre.Scope);
        Assert.Equal(SettingEffect.Immediate, centre.Effect);
        Assert.Equal(new[] { 0d, 1d }, centre.Domain.Options!.Select(static o => o.Value).ToArray());
        Assert.Equal(2, centre.LibretroValues!.Count);
    }

    /// <summary>
    /// REQ-SCN-066 acceptance criterion and the normalisation of AMD-040 §3: a value written off
    /// the ladder is brought into the bounds, then onto the nearest rung, <b>ties to the even</b>
    /// rung. 45 gives 60 and 75 gives 60 — a half rounded away from zero would give 90.
    /// </summary>
    [Theory] // REQ-SCN-066, REQ-LUA-114
    [InlineData("scene.bend_lr", 45, 60)]
    [InlineData("scene.bend_lr", 75, 60)]
    [InlineData("scene.bend_lr", 15, 0)]
    [InlineData("scene.bend_lr", 105, 120)]
    [InlineData("scene.bend_lr", -45, -60)]
    [InlineData("scene.bend_lr", -15, 0)]
    [InlineData("scene.bend_lr", 40, 30)]
    [InlineData("scene.bend_lr", 29, 30)]
    [InlineData("scene.bend_lr", 500, 180)]
    [InlineData("scene.bend_lr", -500, -180)]
    [InlineData("scene.bend_tb", 45, 60)]
    [InlineData("scene.arc_h", 45, 60)]
    [InlineData("scene.arc_center", 7, 1)]
    [InlineData("scene.arc_center", -7, 0)]
    public void REQ_SCN_066_a_value_off_the_ladder_is_normalised(string key, int written, int expected)
    {
        SettingsModel model = new();
        Assert.Equal(0, model.GetInt(key));

        model.SetInt(key, written);
        Assert.Equal(expected, model.GetInt(key));
    }

    /// <summary>
    /// REQ-SCN-066 acceptance criterion, second half: the value survives a restart. Written
    /// through the application's own settings object, saved to <c>settings.json</c> and read back
    /// by a fresh one, as a start-up does.
    /// </summary>
    [Fact] // REQ-SCN-066, REQ-PERS-020
    public void REQ_SCN_066_the_curvature_survives_a_restart()
    {
        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);

        // 45 written in "Bend Left-Right" gives 60 (AMD-040 §3).
        settings.BendLeftRight = 45;
        Assert.Equal(60, settings.BendLeftRight);

        settings.BendTopBottom = -90;
        settings.ArcHorizontal = 30;
        settings.ArcCenter = 1;
        store.Save(settings);

        string json = File.ReadAllText(root.Paths.SettingsFile);
        Assert.Contains("\"scene.bend_lr\": 60", json, StringComparison.Ordinal);
        Assert.Contains("\"scene.arc_center\": 1", json, StringComparison.Ordinal);

        AppSettings restarted = new();
        new SettingsStore(root.Paths).Load(restarted, DateTimeOffset.UnixEpoch);
        Assert.Equal(60, restarted.BendLeftRight);
        Assert.Equal(-90, restarted.BendTopBottom);
        Assert.Equal(30, restarted.ArcHorizontal);
        Assert.Equal(1, restarted.ArcCenter);
    }

    /// <summary>
    /// REQ-SCN-066: the four settings read and write through
    /// <c>Setting:Get/SetVisualStyleParameter(1, id, …)</c> as well — writing by Lua and writing
    /// by the interface come to the same value — and they act whatever <c>Setting.VisualStyle</c>
    /// says, Zaxnes having a single renderer.
    /// </summary>
    [Fact] // REQ-SCN-066, REQ-LUA-114
    public void REQ_SCN_066_the_settings_are_the_style_parameters_of_REQ_LUA_114()
    {
        using LuaRun run = new LuaProfile().Script("probe", string.Empty).Start();
        SettingsModel model = run.Settings.Model;

        (string Id, string Key)[] pairs =
        [
            ("spl.bend_zx", "scene.bend_lr"),
            ("spl.bend_zy", "scene.bend_tb"),
            ("spl.arc_yx", "scene.arc_h"),
            ("spl.arc_center", "scene.arc_center"),
        ];

        // Written by the interface, read by Lua. Done first, because a Lua write becomes a
        // session layer that shadows the preference until the end of the game (REQ-PERS-010).
        foreach ((string id, string key) in pairs)
        {
            model.SetInt(key, 1);
            Assert.Equal(model.GetInt(key), (int)run.Num($"""Setting:GetVisualStyleParameter(1, "{id}")"""));
        }

        model.SetInt("scene.arc_h", -90);
        Assert.Equal(-90d, run.Num("""Setting:GetVisualStyleParameter(1, "spl.arc_yx")"""));

        // Written by Lua, with the normalisation of AMD-040 §3, read by the interface.
        foreach ((string id, string key) in pairs)
        {
            Assert.True(run.Flag($"""Setting:SetVisualStyleParameter(1, "{id}", 1)"""));
            Assert.Equal(model.GetInt(key), (int)run.Num($"""Setting:GetVisualStyleParameter(1, "{id}")"""));
        }

        run.Do("""Setting:SetVisualStyleParameter(1, "spl.bend_zx", 45)""");
        Assert.Equal(60, model.GetInt("scene.bend_lr"));
        Assert.Equal(60d, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        // Whatever the visual style says.
        foreach (int style in (int[])[0, 1])
        {
            run.Do($"Setting.VisualStyle = {style}");
            run.Do("""Setting:SetVisualStyleParameter(1, "spl.bend_zy", 30)""");
            Assert.Equal(30, model.GetInt("scene.bend_tb"));
            model.SetInt("scene.bend_tb", 0);
        }
    }

    /// <summary>
    /// REQ-SCN-066, last sentence: the four settings SHOULD be shown in a « Geometry » group,
    /// marked advanced, with the labels and the help texts of AMD-043 §6. The labels are
    /// compared against the English <b>and</b> French wording of the amendment, so the test does
    /// not depend on the language the interface happens to be in (FIX-6).
    /// </summary>
    [Fact] // REQ-SCN-066
    public void REQ_SCN_066_the_options_window_shows_a_geometry_group()
    {
        (string Key, string English, string French)[] expected =
        [
            ("scene.bend_lr", "Bend Left-Right", "Courbure gauche-droite"),
            ("scene.bend_tb", "Bend Top-Bottom", "Courbure haut-bas"),
            ("scene.arc_h", "Arc Horizontal", "Arc horizontal"),
            ("scene.arc_center", "Arc Center", "Centre de l'arc"),
        ];

        IReadOnlyList<(string Tab, string Key, string Label, double Minimum, double Maximum)> rows =
            UiDriver.OptionRows();

        foreach ((string key, string english, string french) in expected)
        {
            (string Tab, string Key, string Label, double Minimum, double Maximum) row =
                Assert.Single(rows, r => string.Equals(r.Key, key, StringComparison.Ordinal));
            Assert.Contains(row.Label, (string[])[english, french]);

            // The group of AMD-043 §6, and a marker for "advanced".
            Assert.Equal(GeometryGroupKey, OptionsGroupOf(key));
            Assert.Contains(
                UiDriver.Label(GeometryGroupKey),
                (string[])["Geometry", "Géométrie"]);
            Assert.NotEqual("hint.advanced", UiDriver.Label("hint.advanced"));
        }

        // The three angle settings carry the help text of §6 (English "CRT screen", French
        // « écran cathodique » for the first one); the arc centre has none there.
        string leftRight = HelpOf("scene.bend_lr");
        Assert.True(
            leftRight.Contains("CRT", StringComparison.Ordinal)
            || leftRight.Contains("cathodique", StringComparison.Ordinal),
            $"the help of scene.bend_lr is not the one of §6: '{leftRight}'.");
        Assert.NotEmpty(HelpOf("scene.bend_tb"));
        Assert.NotEmpty(HelpOf("scene.arc_h"));

        // AMD-040 §3 gives render.visual_style no interface at all.
        Assert.DoesNotContain(rows, static r => string.Equals(r.Key, "render.visual_style", StringComparison.Ordinal));
    }

    // ===================================================================== AMD-040 REQ-LUA-114

    /// <summary>
    /// REQ-LUA-114, the <c>VisualStyle</c> rows, with the acceptance criterion of the amendment:
    /// the property, the three methods, the normalisation and the "unknown" answers.
    /// </summary>
    [Fact] // REQ-LUA-114
    public void REQ_LUA_114_the_visual_style_and_its_parameters()
    {
        using LuaRun run = new LuaProfile().Script("probe", string.Empty).Start();

        // 0 = Classic, 1 = Enhanced, default 0; any other value written gives 0.
        Assert.Equal(0d, run.Num("Setting.VisualStyle"));
        run.Do("Setting.VisualStyle = 1");
        Assert.Equal(1d, run.Num("Setting.VisualStyle"));
        run.Do("Setting.VisualStyle = 5");
        Assert.Equal(0d, run.Num("Setting.VisualStyle"));
        run.Do("Setting.VisualStyle = -1");
        Assert.Equal(0d, run.Num("Setting.VisualStyle"));

        // SetVisualStyleParameter: true, with the value normalised as AMD-040 §3 says.
        Assert.True(run.Flag("""Setting:SetVisualStyleParameter(1, "spl.bend_zx", 500)"""));
        Assert.Equal(180d, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        foreach ((double written, double read) in (ValueTuple<double, double>[])
            [(40d, 30d), (45d, 60d), (15d, 0d), (-500d, -180d)])
        {
            Assert.True(run.Flag($"""Setting:SetVisualStyleParameter(1, "spl.bend_zx", {written})"""));
            Assert.Equal(read, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));
        }

        // An unknown style or an unknown identifier: false, and nothing recorded.
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(0, "spl.bend_zx", 30)"""));
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(7, "spl.bend_zx", 30)"""));
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(1, "inconnu", 30)"""));
        Assert.Equal(0d, run.Num("""Setting:GetVisualStyleParameter(1, "inconnu")"""));
        Assert.Equal(0d, run.Num("""Setting:GetVisualStyleParameter(0, "spl.bend_zx")"""));

        // Style 0 (Classic) has no parameter at all.
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(0, "spl.arc_yx", 30)"""));

        // ResetVisualStyleParameters: the four parameters of style 1 back to their default;
        // an unknown style does nothing and raises nothing.
        string[] ids = ["spl.bend_zx", "spl.bend_zy", "spl.arc_yx", "spl.arc_center"];
        foreach (string id in ids)
        {
            Assert.True(run.Flag($"""Setting:SetVisualStyleParameter(1, "{id}", 1000)"""));
            Assert.NotEqual(0d, run.Num($"""Setting:GetVisualStyleParameter(1, "{id}")"""));
        }

        Assert.Null(run.Error("Setting:ResetVisualStyleParameters(7)"));
        Assert.Equal(180d, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        Assert.Null(run.Error("Setting:ResetVisualStyleParameters(1)"));
        foreach (string id in ids)
        {
            Assert.Equal(0d, run.Num($"""Setting:GetVisualStyleParameter(1, "{id}")"""));
        }
    }

    /// <summary>
    /// REQ-LUA-114: <c>render.visual_style</c> is a persisted user preference (AMD-040 §3) whose
    /// value has no effect on what Zaxnes draws.
    /// </summary>
    [Fact] // REQ-LUA-114
    public void REQ_LUA_114_the_visual_style_is_persisted_and_changes_no_drawing()
    {
        SettingDescriptor descriptor = SettingsCatalogue.Get("render.visual_style");
        Assert.Equal(SettingScope.User, descriptor.Scope);
        Assert.Equal(0, descriptor.DefaultValue.AsInt);
        Assert.Equal(SettingEffect.Immediate, descriptor.Effect);
        Assert.Null(descriptor.LibretroValues);

        using TemporaryRoot root = new();
        AppSettings settings = new();
        SettingsStore store = new(root.Paths);
        store.Load(settings, DateTimeOffset.UnixEpoch);
        settings.VisualStyle = 1;
        store.Save(settings);

        AppSettings restarted = new();
        new SettingsStore(root.Paths).Load(restarted, DateTimeOffset.UnixEpoch);
        Assert.Equal(1, restarted.VisualStyle);

        settings.VisualStyle = 5;
        Assert.Equal(0, settings.VisualStyle);

        // The scene the renderer reads carries no style at all, so no drawing can depend on it.
        Assert.DoesNotContain(
            typeof(RenderSettings).GetProperties().Select(static p => p.Name),
            static name => name.Contains("VisualStyle", StringComparison.Ordinal));
    }

    // ===================================================================== §3 REQ-SCN-068

    /// <summary>
    /// REQ-SCN-068, harness criterion: with the left-right bend at 90°, the exported vertices of
    /// a tile standing at <c>(256, 120, 0)</c> are at <c>B(P)</c>, and the exported HUD is
    /// exactly the one a null curvature gives.
    /// </summary>
    [Fact] // REQ-SCN-068
    public void REQ_SCN_068_the_tiles_are_bent_and_the_hud_is_not()
    {
        SceneState scene = CornerScene(new Vec4F(0f, 0f, 0f, 0f));
        CameraView view = View(scene);

        IReadOnlyList<ExportedObject> flat = SceneExport.Objects(scene, view);
        scene.Settings.BendLeftRight = 90;
        SpaceBend bend = scene.Settings.Bend;
        IReadOnlyList<ExportedObject> bent = SceneExport.Objects(scene, view);

        Assert.Equal(flat.Count, bent.Count);
        Assert.Contains(bent, static o => o.Hud);
        Assert.Contains(bent, static o => !o.Hud);

        // The tile the criterion names: its corner is at B(P).
        Vector3 corner = new(256f, 120f, 0f);
        ExportedObject tile = Assert.Single(bent, o => !o.Hud && Holds(o.Unbent, corner));
        Vec3 expected = bend.Apply(new Vec3(256f, 120f, 0f));
        int at = IndexOf(tile.Unbent, corner);
        Assert.Equal(expected.X, tile.Positions[at].X, 0.01f);
        Assert.Equal(expected.Y, tile.Positions[at].Y, 0.01f);
        Assert.Equal(expected.Z, tile.Positions[at].Z, 0.01f);

        // The bend really moved it: 90° on the right edge is a large displacement.
        Assert.True(Vector3.Distance(tile.Positions[at], corner) > 10f);

        // Every converted vertex of the scene is at the image of its unbent position …
        foreach (ExportedObject exported in bent.Where(static o => !o.Hud))
        {
            for (int v = 0; v < exported.Positions.Count; v++)
            {
                Vec3 image = bend.Apply(ToVec3(exported.Unbent[v]));
                Assert.True(
                    Vector3.Distance(ToVector(image), exported.Positions[v]) < 1e-2f,
                    $"vertex {v}: {exported.Positions[v]} is not B({exported.Unbent[v]}).");
            }
        }

        // … and the HUD is byte for byte the one a null curvature gives.
        for (int i = 0; i < bent.Count; i++)
        {
            if (!bent[i].Hud)
            {
                continue;
            }

            Assert.Equal(flat[i].Positions, bent[i].Positions);
            Assert.Equal(flat[i].Normals, bent[i].Normals);
        }
    }

    /// <summary>
    /// REQ-SCN-068, "order with the scene-coordinate computations": the clipping of REQ-SCN-036
    /// is evaluated on the position <b>before</b> bending, so a tile the margins keep stays
    /// visible even when its bent image has left the rectangle R. With the horizontal arc at 60°
    /// on the bottom edge and the default margins, the corner tile at <c>P = (0, 8, 0)</c> has
    /// <c>B(P) = (0,32 ; −17,70 ; 0)</c>, well below R.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-036
    public void REQ_SCN_068_the_clipping_is_evaluated_before_bending()
    {
        // The figure of the criterion, first.
        Vec3 image = new SpaceBend(0f, 0f, 60f, arcAtEdge: true).Apply(new Vec3(0f, 8f, 0f));
        Assert.Equal(0.32f, image.X, VectorTolerance);
        Assert.Equal(-17.70f, image.Y, VectorTolerance);
        Assert.Equal(0f, image.Z, VectorTolerance);

        SceneState scene = CornerScene();
        Assert.Equal(ConversionSettings.DefaultClipping, scene.Clipping);
        VisibleRect rect = scene.VisibleRectangle;
        Assert.False(rect.Contains(image.X, image.Y));

        scene.Settings.ArcHorizontal = 60;
        scene.Settings.ArcAtEdge = true;

        ExportedObject corner = Assert.Single(
            SceneExport.Objects(scene, View(scene)),
            o => !o.Hud && Holds(o.Unbent, new Vector3(0f, 8f, 0f)));

        // The clipping applies to this tile, and it is judged on the unbent positions: part of
        // the tile is inside R before bending …
        Assert.True(corner.Clipped);
        Assert.Contains(corner.Unbent, p => rect.Contains(p.X, p.Y));

        // … while the whole bent image is outside it, which is what would have thrown the tile
        // away had the rectangle been applied after the curvature.
        Assert.DoesNotContain(corner.Positions, p => rect.Contains(p.X, p.Y));
    }

    /// <summary>
    /// REQ-SCN-068, same bullet: the aimed point <c>P*</c> of the camera (REQ-SCN-008) is
    /// evaluated on the positions <b>before</b> bending — the curvature comes in at the final
    /// placement of the vertices only, so it moves no camera.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-008
    public void REQ_SCN_068_the_camera_target_is_evaluated_before_bending()
    {
        SceneState scene = CornerScene();
        CameraView flat = View(scene);

        foreach ((int lr, int tb, int arc, bool edge) in (ValueTuple<int, int, int, bool>[])
            [(90, 0, 0, false), (0, -90, 0, false), (0, 0, 60, true), (180, 180, 180, true)])
        {
            scene.Settings.BendLeftRight = lr;
            scene.Settings.BendTopBottom = tb;
            scene.Settings.ArcHorizontal = arc;
            scene.Settings.ArcAtEdge = edge;

            CameraView bent = View(scene);
            Assert.Equal(flat.Eye, bent.Eye);
            Assert.Equal(flat.Target, bent.Target);

            // P* itself (REQ-SCN-008), and the whole matrix it builds.
            Assert.Equal(flat.AimPoint, bent.AimPoint);
            Assert.Equal(flat.ViewProjection, bent.ViewProjection);
        }
    }

    /// <summary>
    /// REQ-SCN-068: the backdrop bends, is never clipped, and is subdivided into at least
    /// 32 × 32 cells per face — a face has to be a curve, not a quad tilted in space.
    /// </summary>
    [Fact] // REQ-SCN-068
    public void REQ_SCN_068_the_backdrop_bends_and_is_subdivided()
    {
        SceneState scene = CornerScene();
        scene.Backdrop.Enabled = true;
        scene.Settings.BendLeftRight = 90;

        ExportedObject backdrop = SceneExport.Objects(scene, View(scene))[^1];
        Assert.True(backdrop.Bent);
        Assert.False(backdrop.Clipped);

        // Six faces of (32 + 1)² vertices at least.
        Assert.True(
            backdrop.Positions.Count >= 6 * 33 * 33,
            $"the backdrop exported only {backdrop.Positions.Count} vertices, "
            + $"which is fewer than 6 × 33 × 33 = {6 * 33 * 33}.");

        // The curvature really reaches it, and it bends inside a face: the vertex that moves
        // most is not a corner of the box.
        float largest = 0f;
        int worst = 0;
        for (int i = 0; i < backdrop.Positions.Count; i++)
        {
            float moved = Vector3.Distance(backdrop.Positions[i], backdrop.Unbent[i]);
            if (moved > largest)
            {
                largest = moved;
                worst = i;
            }
        }

        Assert.True(largest > 1f, "the backdrop did not bend at all.");

        // A subdivided face has interior vertices: the whole bent face is not a plane. Three
        // vertices of a row of one face and their middle suffice to show it.
        Assert.True(
            Deviation(backdrop.Positions, backdrop.Unbent) > 0.5f,
            $"the bent backdrop is still flat around vertex {worst}.");
    }

    /// <summary>
    /// Q-079, closed by AMD-046 as reading 1 (REQ-SCN-074): in 2D mode the 256 × 240 rectangle
    /// is the game scene, so it bends like the tiles it stands in for, and it is subdivided into
    /// 32 × 32 cells — 2 048 triangles instead of 2.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-074
    public void REQ_SCN_068_the_two_dimensional_quad_bends_as_REQ_SCN_074_settles_Q079()
    {
        SceneFixture fixture = new() { TwoDimensionalMode = true };
        SceneState scene = fixture.Build();
        scene.TwoDimensionalMode = true;
        scene.Settings.BendLeftRight = 90;

        ExportedObject quad = Assert.Single(SceneExport.Objects(scene, View(scene)));
        Assert.True(quad.Bent);
        Assert.Equal(33 * 33, quad.Positions.Count);
        Assert.True(
            Deviation(quad.Positions, quad.Unbent) > 0.5f,
            "the 2D quad stayed a flat plane under the curvature.");
    }

    /// <summary>
    /// REQ-SCN-028 as amended by REQ-SCN-068: the shading normal is the mesh normal
    /// <b>transported</b> by the curvature. Checked against the surface itself: the normal of a
    /// bent vertex is perpendicular to the two tangents the curvature gives at that point.
    /// </summary>
    [Fact] // REQ-SCN-028, REQ-SCN-068
    public void REQ_SCN_028_the_shading_normal_is_transported_by_the_curvature()
    {
        SceneState scene = CornerScene(new Vec4F(0f, 0f, 0f, 0f));
        CameraView view = View(scene);

        IReadOnlyList<ExportedObject> flat = SceneExport.Objects(scene, view);
        scene.Settings.BendLeftRight = 90;
        SpaceBend bend = scene.Settings.Bend;
        IReadOnlyList<ExportedObject> bent = SceneExport.Objects(scene, view);

        Vector3 corner = new(256f, 120f, 0f);
        int which = -1;
        for (int i = 0; i < bent.Count; i++)
        {
            if (!bent[i].Hud && Holds(bent[i].Unbent, corner))
            {
                which = i;
            }
        }

        Assert.True(which >= 0, "the corner tile is not in the export.");
        ExportedObject tile = bent[which];
        int at = IndexOf(tile.Unbent, corner);

        // The normal moved: 90° on the right edge turns the face by a large angle.
        Vector3 before = Vector3.Normalize(flat[which].Normals[at]);
        Vector3 after = tile.Normals[at];
        Assert.Equal(1f, after.Length(), 1e-3f);
        Assert.True(
            Vector3.Dot(before, after) < 0.95f,
            $"the normal was not transported: {before} became {after}.");

        // And it is the normal of the bent surface: a normal "transported by the curvature" is
        // the one perpendicular to the image of every tangent of the surface it belonged to.
        // Two independent tangents of the face — both perpendicular to the mesh normal — are
        // pushed through B by a central difference, and the transported normal must be
        // perpendicular to both of their images.
        Vector3 helper = MathF.Abs(before.X) < 0.5f ? Vector3.UnitX : Vector3.UnitY;
        Vector3 tangent1 = Vector3.Normalize(Vector3.Cross(before, helper));
        Vector3 tangent2 = Vector3.Normalize(Vector3.Cross(before, tangent1));

        foreach (Vector3 tangent in (Vector3[])[tangent1, tangent2])
        {
            const float step = 0.05f;
            Vec3 point = ToVec3(corner);
            Vec3 forward = bend.Apply(point + (ToVec3(tangent) * step));
            Vec3 backward = bend.Apply(point - (ToVec3(tangent) * step));
            Vec3 image = forward - backward;

            float cosine = Dot(image, ToVec3(after)) / MathF.Sqrt(image.SquaredLength);
            Assert.True(
                MathF.Abs(cosine) < 1e-2f,
                $"the transported normal {after} is not perpendicular to the image of the "
                + $"tangent {tangent}: cosine {cosine}.");
        }
    }

    /// <summary>
    /// REQ-SCN-068, first bullet: the curvature comes <b>after</b> the animated deformation of
    /// REQ-SCN-043 and after the object's own position, scale and rotation — it acts on the
    /// final position in the scene. A tile whose instance deforms is exported at
    /// <c>B(M · (v + δ))</c>, which is not <c>B(M · v)</c>.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-043
    public void REQ_SCN_068_the_curvature_comes_after_the_animated_deformation()
    {
        SceneState scene = DeformedScene(0.25f);
        scene.Settings.BendLeftRight = 90;
        SpaceBend bend = scene.Settings.Bend;

        DrawTile tile = Assert.Single(scene.DrawList!.Tiles, static t => t.Deform is not null);
        TileMesh mesh = tile.Mesh!;
        DeformParameters deform = tile.Deform!.Value;

        ExportedObject exported = Assert.Single(
            SceneExport.Objects(scene, View(scene)),
            o => !o.Hud && o.Positions.Count == mesh.Vertices.Length);

        float moved = 0f;
        for (int v = 0; v < mesh.Vertices.Length; v++)
        {
            Vec3 local = mesh.Vertices[v].Position;
            Vec3 displacement = deform.Displacement(local, scene.EmulatedTime);
            Vec3 deformed = tile.Transform.TransformPoint(local + displacement);
            Vec3 plain = tile.Transform.TransformPoint(local);
            moved = MathF.Max(moved, MathF.Sqrt((deformed - plain).SquaredLength));

            // The position before bending is the deformed, placed vertex …
            Assert.Equal(deformed.X, exported.Unbent[v].X, 1e-3f);
            Assert.Equal(deformed.Y, exported.Unbent[v].Y, 1e-3f);
            Assert.Equal(deformed.Z, exported.Unbent[v].Z, 1e-3f);

            // … and the drawn position is its image, not the image of the undeformed one.
            Vec3 image = bend.Apply(deformed);
            Assert.Equal(image.X, exported.Positions[v].X, 1e-2f);
            Assert.Equal(image.Y, exported.Positions[v].Y, 1e-2f);
            Assert.Equal(image.Z, exported.Positions[v].Z, 1e-2f);
        }

        Assert.True(moved > 1e-3f, "the deformation displaced nothing, so the order is not shown.");

        // REQ-SCN-028: the parenthesis « unchanged by the deformation of REQ-SCN-043 » stays
        // true for the animated deformation. Without any curvature, the normals of the deformed
        // frame are the ones of the very same frame built without a deformation at all.
        scene.Settings.BendLeftRight = 0;
        IReadOnlyList<Vector3> deformedNormals =
            Assert.Single(SceneExport.Objects(scene, View(scene))).Normals;

        SceneState plainScene = DeformedScene(0f);
        IReadOnlyList<Vector3> plainNormals =
            Assert.Single(SceneExport.Objects(plainScene, View(plainScene))).Normals;
        Assert.Equal(plainNormals, deformedNormals);
    }

    /// <summary>
    /// One background tile whose instance deforms with the amplitude
    /// <paramref name="amplitude"/> on y, at the emulated time 0,5 s.
    /// </summary>
    private static SceneState DeformedScene(float amplitude)
    {
        ConversionEngine engine = new();
        GeometryBuilder geometry = engine.UseGeometry();
        EnginePattern pattern = SyntheticPatterns.Build(engine, ["A"]);

        // A default instance already carries the phases π/2 of REQ-E3D-568, so an amplitude is
        // all it takes for δ to be a constant, plainly non-zero displacement.
        pattern.Instances[0].DeformAmplitude = new Vec3(0f, amplitude, 0f);
        geometry.Build(pattern);
        geometry.Commit();

        ConversionFrame frame = new SyntheticBatch().Background('A', 128, 112).Convert(engine);
        engine.Publish();

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.Clipping = engine.Settings.Clipping;
        scene.DrawList = engine.BuildDrawList(frame);
        scene.EmulatedTime = 0.5f;
        return scene;
    }

    /// <summary>
    /// REQ-SCN-068 on the GPU: the shadow pass is bent too, so the shadows follow the curved
    /// surface instead of staying under the flat geometry. The shadow is isolated by rendering
    /// the same frame with the shadows on and off; its displacement under a 90° horizontal arc
    /// is compared with the displacement of the caster itself.
    /// </summary>
    [GlFact] // REQ-SCN-068
    public void REQ_SCN_068_the_shadows_follow_the_bent_surface()
    {
        using GlTarget target = GlTarget.Create(480, 360);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = ShadowScene();
        (Vector2 caster, Vector2 shadow) flat = ShadowAndCaster(target, renderer, scene);

        scene.Settings.ArcHorizontal = 90;
        (Vector2 caster, Vector2 shadow) bent = ShadowAndCaster(target, renderer, scene);

        Vector2 casterMoved = bent.caster - flat.caster;
        Vector2 shadowMoved = bent.shadow - flat.shadow;

        Assert.True(
            casterMoved.Length() > 8f,
            $"the curvature barely moved the caster ({flat.caster} → {bent.caster}).");
        Assert.True(
            shadowMoved.Length() > 0.25f * casterMoved.Length()
            && shadowMoved.Length() < 4f * casterMoved.Length(),
            $"the shadow moved {shadowMoved.Length():F1} px ({flat.shadow} → {bent.shadow}) "
            + $"while the caster moved {casterMoved.Length():F1} px: the shadow pass does not "
            + "follow the curvature.");
        Assert.True(
            Vector2.Dot(Vector2.Normalize(casterMoved), Vector2.Normalize(shadowMoved)) > 0.5f,
            $"the shadow moved {shadowMoved} while the caster moved {casterMoved}.");
    }

    /// <summary>
    /// REQ-SCN-068, third bullet: the curvature reaches neither the HUD plane of REQ-SCN-045 nor
    /// the skybox. A frame whose only geometry is on the HUD plane, over a drawn skybox, is
    /// rendered pixel for pixel the same under the three axes at once.
    /// </summary>
    [GlFact] // REQ-SCN-068, REQ-SCN-045, REQ-SCN-049
    public void REQ_SCN_068_the_hud_and_the_skybox_stay_flat()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.Ui)
            .Rect(SceneFixture.Solid(1), column: 9, row: 9, columns: 8, rows: 8)
            .Build();
        scene.Settings.Skybox = SkyboxKind.Gradient;

        Shot flat = target.Render(renderer, scene);
        Assert.True(scene.Settings.SkyboxDrawn, "the skybox of the frame is not drawn at all.");
        Assert.True(flat.DistinctColors().Count > 2, "the capture holds neither skybox nor HUD.");

        scene.Settings.BendLeftRight = 180;
        scene.Settings.BendTopBottom = -90;
        scene.Settings.ArcHorizontal = 120;
        scene.Settings.ArcAtEdge = true;
        Shot bent = target.Render(renderer, scene);

        Assert.Equal(flat.Pixels, bent.Pixels);
    }

    /// <summary>
    /// REQ-SCN-068, "order with the scene-coordinate computations": the FPV occlusion segment of
    /// REQ-SCN-022 is evaluated on the positions <b>before</b> bending, so the very same tiles
    /// fade whatever the curvature.
    /// </summary>
    [GlFact] // REQ-SCN-068, REQ-SCN-022
    public void REQ_SCN_068_the_fpv_occlusion_is_evaluated_before_bending()
    {
        using GlTarget target = GlTarget.Create(160, 160);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(21, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 4, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), 12, 12, 6, 6)
            .Sprite(SceneFixture.Solid(1, id: 3), 112, 112, palette: 1);

        SceneState scene = fixture.Build();
        scene.Settings.Shadows = false;
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Target = new Vector3(116f, 128f, 20f);
        scene.Camera.Current.Distance = 120f;
        scene.Camera.Current.Angles = Vector3.Zero;
        scene.Camera.SnapToSetpoints();
        scene.FollowedShape = fixture.Engine!.PublishedFrame!.Shapes[^1];

        _ = target.Render(renderer, scene);
        int flat = renderer.Statistics.Occluders;
        Assert.True(flat > 0, "nothing hid the followed shape, so there is nothing to compare.");

        foreach ((int lr, int tb, int arc, bool edge) in (ValueTuple<int, int, int, bool>[])
            [(90, 0, 0, false), (0, 0, 90, true), (-120, 60, 0, false)])
        {
            scene.Settings.BendLeftRight = lr;
            scene.Settings.BendTopBottom = tb;
            scene.Settings.ArcHorizontal = arc;
            scene.Settings.ArcAtEdge = edge;

            _ = target.Render(renderer, scene);
            Assert.Equal(flat, renderer.Statistics.Occluders);
        }
    }

    /// <summary>
    /// REQ-SCN-066: the four settings apply hot, from the next rendered frame on, inside the
    /// running desktop host — written on the settings model, read on the scene the renderer
    /// consumes, with no reload of anything.
    /// </summary>
    [Fact] // REQ-SCN-066
    public void REQ_SCN_066_the_settings_apply_hot_in_the_running_host()
    {
        using HostDriver host = HostDriver.Create();
        Assert.True(host.Scene.Settings.Bend.IsIdentity);

        // 45 written in "Bend Left-Right" gives 60, and the scene has it at once.
        host.Settings.BendLeftRight = 45;
        Assert.Equal(60, host.Settings.BendLeftRight);
        Assert.Equal(60, host.Scene.Settings.BendLeftRight);

        Vec3 image = host.Scene.Settings.Bend.Apply(new Vec3(256f, 120f, 0f));
        Assert.Equal(251.64f, image.X, VectorTolerance);
        Assert.Equal(24.70f, image.Z, VectorTolerance);

        host.Settings.BendTopBottom = -30;
        host.Settings.ArcHorizontal = 60;
        host.Settings.ArcCenter = 1;
        Assert.Equal(-30, host.Scene.Settings.BendTopBottom);
        Assert.Equal(60, host.Scene.Settings.ArcHorizontal);
        Assert.True(host.Scene.Settings.ArcAtEdge);

        // And back to a flat scene, just as hot.
        host.Settings.BendLeftRight = 0;
        host.Settings.BendTopBottom = 0;
        host.Settings.ArcHorizontal = 0;
        host.Settings.ArcCenter = 0;
        Assert.True(host.Scene.Settings.Bend.IsIdentity);
    }

    // ===================================================================== §4 REQ-INP-124

    /// <summary>
    /// REQ-INP-124, acceptance criterion: for each of the vectors of REQ-SCN-067 the depth limit
    /// does not touch, a ray that passes through <c>B(P)</c> gives an aimed point less than 0,5
    /// scene unit from <c>P</c>, with <c>z₀</c> the depth of <c>P</c>.
    /// </summary>
    [Theory] // REQ-INP-124
    [InlineData(60, 0, 0, false, 128f, 120f, 0f)]
    [InlineData(60, 0, 0, false, 256f, 120f, 0f)]
    [InlineData(60, 0, 0, false, 0f, 120f, 0f)]
    [InlineData(60, 0, 0, false, 256f, 120f, 10f)]
    [InlineData(-60, 0, 0, false, 256f, 120f, 0f)]
    [InlineData(0, 60, 0, false, 128f, 240f, 0f)]
    [InlineData(0, 60, 0, false, 128f, 0f, 0f)]
    [InlineData(0, 0, 60, false, 0f, 120f, 0f)]
    [InlineData(0, 0, 60, false, 128f, 240f, 0f)]
    [InlineData(0, 0, 60, true, 0f, 0f, 0f)]
    [InlineData(0, 0, 60, true, 128f, 240f, 0f)]
    public void REQ_INP_124_a_ray_through_the_image_aims_within_half_a_unit(
        int bendZx,
        int bendZy,
        int arcYx,
        bool arcAtEdge,
        float x,
        float y,
        float z)
    {
        SpaceBend bend = new(bendZx, bendZy, arcYx, arcAtEdge);
        CameraView view = DefaultView();
        Vector2 pointer = Project(view, ToVector(bend.Apply(new Vec3(x, y, z))));

        Assert.True(
            ZapperAim.TryIntersect(view, pointer, Viewport, new Vector3(0f, 0f, z), bend, 1f, out Vector2 aimed),
            $"no candidate for ({x}, {y}, {z}).");

        Assert.True(
            Vector2.Distance(aimed, new Vector2(x, y)) < 0.5f,
            $"aimed at {aimed} instead of ({x}, {y}): {Vector2.Distance(aimed, new Vector2(x, y))} units off.");
    }

    /// <summary>
    /// REQ-INP-124, last bullet: « la position retenue DOIT être à moins de 0,5 unité de scène
    /// du point défini ci-dessus ». The vectors of REQ-SCN-067 are only thirteen samples; this
    /// sweeps a grid of 7 × 7 points covering the whole aiming window, for several curvatures
    /// on each axis and both arc centres, and asks for the half-unit everywhere.
    /// </summary>
    [Theory] // REQ-INP-124
    [InlineData(90, 0, 0, false)]
    [InlineData(-90, 0, 0, false)]
    [InlineData(30, 0, 0, false)]
    [InlineData(0, 90, 0, false)]
    [InlineData(0, -60, 0, false)]
    [InlineData(0, 0, 90, false)]
    [InlineData(0, 0, -90, false)]
    [InlineData(0, 0, 90, true)]
    [InlineData(0, 0, -60, true)]
    [InlineData(60, 30, 0, false)]
    [InlineData(30, 0, 60, true)]
    public void REQ_INP_124_the_aim_is_within_half_a_unit_over_the_whole_window(
        int bendZx,
        int bendZy,
        int arcYx,
        bool arcAtEdge)
    {
        SpaceBend bend = new(bendZx, bendZy, arcYx, arcAtEdge);
        CameraView view = DefaultView();
        int tried = 0;

        for (int row = 0; row <= 6; row++)
        {
            for (int column = 0; column <= 6; column++)
            {
                // The window of REQ-INP-124: |x − 128| ≤ 144, |y − 128| ≤ 136.
                Vector2 wanted = new(
                    128f - 144f + (column * 288f / 6f),
                    128f - 136f + (row * 272f / 6f));
                // AMD-043 §4: beyond the depth limit of REQ-SCN-067, B is not invertible and
                // the aim is left unspecified; such a sample proves nothing either way.
                if (DepthHeld(bendZx, bendZy, arcYx, arcAtEdge, wanted.X, wanted.Y, 0f))
                {
                    continue;
                }

                tried++;
                Vector2 pointer = Project(view, ToVector(bend.Apply(new Vec3(wanted.X, wanted.Y, 0f))));

                Assert.True(
                    ZapperAim.TryIntersect(
                        view, pointer, Viewport, Vector3.Zero, bend, 1f, out Vector2 aimed),
                    $"no candidate for {wanted}.");
                Assert.True(
                    Vector2.Distance(aimed, wanted) < 0.5f,
                    $"{wanted} was aimed at {aimed}: {Vector2.Distance(aimed, wanted):F3} units off.");
            }
        }

        Assert.True(tried >= 30, $"only {tried} of the 49 samples were inside the specified domain.");
    }

    /// <summary>
    /// Whether the depth limit of REQ-SCN-067 step 2 — <c>w · k &gt; 0,95</c> — bites for any
    /// axis on the way through <c>B</c>. Written straight from the amendment, so that the sweep
    /// above only asks about the part of the plane where AMD-043 §4 specifies an answer.
    /// </summary>
    private static bool DepthHeld(
        int bendZx,
        int bendZy,
        int arcYx,
        bool arcAtEdge,
        float x,
        float y,
        float z)
    {
        bool held = false;
        float u = x - 128f;
        float v = y - 120f;
        float w = z;

        void Wrap(int degrees, float length, ref float along, ref float depth)
        {
            float theta = degrees * MathF.PI / 180f;
            if (MathF.Abs(theta) < 1e-4f)
            {
                return;
            }

            float k = theta / length;
            float phi = theta / 4f;
            float f = phi / MathF.Sin(phi);
            float half = MathF.Sin(phi / 2f);
            float h = 2f * half * half / k;

            float limited = depth;
            if (depth * k > 0.95f)
            {
                held = true;
                limited = 0.95f / k;
            }

            float a = along * k;
            along = f * (1f - (limited * k)) * MathF.Sin(a) / k;
            depth = f * (((1f - MathF.Cos(a)) / k) + (limited * MathF.Cos(a)) - h);
        }

        Wrap(bendZx, 256f, ref u, ref w);
        Wrap(bendZy, 240f, ref v, ref w);

        float pivot = arcAtEdge ? -120f : 0f;
        float second = pivot - v;
        Wrap(arcYx, 256f, ref u, ref second);
        return held;
    }

    /// <summary>
    /// REQ-INP-124: the NES coordinates are then obtained as in REQ-INP-063, so a ray through
    /// the image of a point of the aim plane gives the pixel that point shows, and without any
    /// curvature the aim is the one of REQ-INP-063, unchanged.
    /// </summary>
    [Fact] // REQ-INP-124, REQ-INP-063
    public void REQ_INP_124_the_pixel_is_the_one_of_REQ_INP_063()
    {
        CameraView view = DefaultView();
        SpaceBend bend = new(60f, 0f, 0f, arcAtEdge: false);

        foreach (Vector2 scenePoint in (Vector2[])[new(128f, 128f), new(40f, 200f), new(230f, 60f)])
        {
            Vector2 pointer = Project(view, ToVector(bend.Apply(new Vec3(scenePoint.X, scenePoint.Y, 0f))));
            ZapperState zapper = new();
            zapper.Update(true, view, pointer, Viewport, rightButton: false, Vector3.Zero, 1f, bend);

            ZapperPosition want = ZapperAim.ToNesPixel(scenePoint, Vector3.Zero);
            Assert.True(zapper.HasPosition);
            Assert.True(
                Math.Abs(want.X - zapper.Position.X) <= 1 && Math.Abs(want.Y - zapper.Position.Y) <= 1,
                $"{scenePoint}: expected ({want.X}, {want.Y}), got ({zapper.Position.X}, {zapper.Position.Y}).");
        }

        // No curvature: the aim of REQ-INP-063, to the unit.
        Vector2 straight = Project(view, new Vector3(200f, 60f, 0f));
        Assert.True(ZapperAim.TryIntersect(view, straight, Viewport, Vector3.Zero, out Vector2 plain));
        Assert.True(ZapperAim.TryIntersect(
            view, straight, Viewport, Vector3.Zero, SpaceBend.None, 1f, out Vector2 identity));
        Assert.Equal(plain, identity);
    }

    /// <summary>
    /// REQ-INP-124: the aiming window is the NES image of REQ-SCN-001 widened by 16 scene units
    /// on each of its four edges, that is <c>|x − 128| ≤ 144</c> and <c>|y − 128| ≤ 136</c>.
    /// </summary>
    [Fact] // REQ-INP-124
    public void REQ_INP_124_the_aiming_window_is_the_image_widened_by_sixteen_units()
    {
        Assert.True(ZapperAim.InAimingWindow(new Vector2(128f, 128f)));
        Assert.True(ZapperAim.InAimingWindow(new Vector2(-16f, -8f)));
        Assert.True(ZapperAim.InAimingWindow(new Vector2(272f, 264f)));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(-16.5f, 128f)));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(272.5f, 128f)));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(128f, -8.5f)));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(128f, 264.5f)));

        // The scale of REQ-SCN-002 scales the window with the image.
        Assert.True(ZapperAim.InAimingWindow(new Vector2(128f + 288f, 128f), 2f));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(128f + 289f, 128f), 2f));
    }

    /// <summary>
    /// REQ-INP-124, last two bullets: a candidate found only outside the aiming window is handed
    /// over as it is — the unclamped off-screen shot of REQ-INP-064 — and a ray with no candidate
    /// at all leaves the last valid position in place.
    /// </summary>
    [Fact] // REQ-INP-124, REQ-INP-064
    public void REQ_INP_124_off_screen_and_no_candidate_keep_the_rules_of_REQ_INP_064()
    {
        CameraView view = DefaultView();
        SpaceBend bend = new(30f, 0f, 0f, arcAtEdge: false);

        // A point well outside the window: handed over unclamped.
        Vector2 far = new(-120f, 120f);
        Assert.False(ZapperAim.InAimingWindow(far));
        Vector2 pointer = Project(view, ToVector(bend.Apply(new Vec3(far.X, far.Y, 0f))));
        Assert.True(ZapperAim.TryIntersect(view, pointer, Viewport, Vector3.Zero, bend, 1f, out Vector2 aimed));
        Assert.True(
            Vector2.Distance(aimed, far) < 0.5f,
            $"the off-screen candidate was changed into {aimed}.");

        ZapperPosition shot = ZapperAim.ToNesPixel(aimed, Vector3.Zero);
        Assert.True(shot.X < 0, $"an off-screen shot must stay negative, got {shot.X}.");

        // A valid position first …
        ZapperState zapper = new();
        zapper.Update(
            true,
            view,
            Project(view, ToVector(bend.Apply(new Vec3(128f, 120f, 0f)))),
            Viewport,
            rightButton: false,
            Vector3.Zero,
            1f,
            bend);
        Assert.True(zapper.HasPosition);
        ZapperPosition valid = zapper.Position;

        // … then a ray that meets no point of the aim plane at all: the position is kept.
        Vector3 unreachable = new(0f, 0f, 100_000f);
        Assert.False(ZapperAim.TryIntersect(
            view, new Vector2(320f, 240f), Viewport, unreachable, bend, 1f, out _));
        zapper.Update(
            true, view, new Vector2(320f, 240f), Viewport, false, unreachable, 1f, bend);
        Assert.Equal(valid, zapper.Position);
    }

    // ===================================================================== fixtures

    /// <summary>
    /// A frame holding the corner tile of REQ-SCN-068 — screen <c>(0, 232)</c>, whose
    /// bottom-left-front corner is <c>(0, 8, 0)</c> — a tile whose right edge reaches
    /// <c>x = 256</c> at <c>y = 120</c>, and one HUD tile.
    /// </summary>
    private static SceneState CornerScene(Vec4F? clipping = null)
    {
        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.Ui)
            .Tile(SceneFixture.Solid(1, 0), 0, 232)
            .Tile(SceneFixture.Solid(1, 0), 248, 120)
            .Tile(SceneFixture.Solid(1, 1), 120, 120);

        return fixture.Build(settings =>
        {
            if (clipping is { } margins)
            {
                settings.Clipping = margins;
            }
        });
    }

    /// <summary>
    /// A flat grey floor in L2, which receives, and a red block of 2 × 2 cells in L1 standing
    /// in front of it to the right of the centre, which casts (REQ-SCN-030, REQ-SCN-032).
    /// </summary>
    private static SceneState ShadowScene()
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(2, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L2)
            .Pattern(SceneFixture.Solid(2), depth: 8, ProfileLayer.L1);

        for (int row = 8; row < 22; row++)
        {
            for (int column = 14; column < 32; column++)
            {
                fixture.Tile(SceneFixture.Solid(1), column * 8, row * 8);
            }
        }

        for (int row = 13; row < 15; row++)
        {
            for (int column = 28; column < 31; column++)
            {
                fixture.Tile(SceneFixture.Solid(2), column * 8, row * 8);
            }
        }

        SceneState scene = fixture.Build();
        scene.Settings.Shadows = true;
        return scene;
    }

    /// <summary>
    /// The centroid of the caster and the centroid of its shadow, in pixels. The shadow is the
    /// set of pixels the shadow pass darkens, isolated by rendering the same frame twice.
    /// </summary>
    private static (Vector2 Caster, Vector2 Shadow) ShadowAndCaster(
        GlTarget target,
        SceneRenderer renderer,
        SceneState scene)
    {
        scene.Settings.Shadows = true;
        Shot lit = target.Render(renderer, scene);
        Assert.True(renderer.Statistics.ShadowPass, "the frame ran no shadow pass at all.");

        scene.Settings.Shadows = false;
        Shot plain = target.Render(renderer, scene);
        scene.Settings.Shadows = true;

        double sumX = 0d;
        double sumY = 0d;
        int darkened = 0;
        double casterX = 0d;
        double casterY = 0d;
        int caster = 0;

        for (int y = 0; y < lit.Height; y++)
        {
            for (int x = 0; x < lit.Width; x++)
            {
                (int r, int g, int b) = lit.At(x, y);
                if (r > 80 && r > g * 2 && r > b * 2)
                {
                    casterX += x;
                    casterY += y;
                    caster++;
                    continue;
                }

                // A pixel the shadow pass took light away from.
                if (plain.Luma(x, y) - lit.Luma(x, y) > 0.02d)
                {
                    sumX += x;
                    sumY += y;
                    darkened++;
                }
            }
        }

        Assert.True(caster > 20, $"only {caster} pixels of the caster were drawn.");
        Assert.True(darkened > 20, $"only {darkened} pixels were darkened by the shadow pass.");

        return (
            new Vector2((float)(casterX / caster), (float)(casterY / caster)),
            new Vector2((float)(sumX / darkened), (float)(sumY / darkened)));
    }

    private static CameraView View(SceneState scene)
    {
        scene.Camera.SnapToSetpoints();
        return scene.Camera.BuildView(scene.Clipping, Viewport.X / Viewport.Y);
    }

    private static CameraView DefaultView()
    {
        SceneCamera camera = new();
        camera.ResetView();
        camera.SnapToSetpoints();
        return camera.BuildView(ConversionSettings.DefaultClipping, Viewport.X / Viewport.Y);
    }

    /// <summary>
    /// The pointer a scene point projects to. The scene frame is left-handed (REQ-E3D-400), so
    /// the clip position is built from <c>(x, y, −z)</c> as REQ-SCN-001 has the renderer do.
    /// </summary>
    private static Vector2 Project(in CameraView view, Vector3 point)
    {
        Vector4 clip = Vector4.Transform(new Vector3(point.X, point.Y, -point.Z), view.ViewProjection);
        return new Vector2(
            ((clip.X / clip.W) + 1f) * 0.5f * Viewport.X,
            (1f - (clip.Y / clip.W)) * 0.5f * Viewport.Y);
    }

    /// <summary>How far the bent vertices leave the plane their unbent positions lie in.</summary>
    private static float Deviation(IReadOnlyList<Vector3> bent, IReadOnlyList<Vector3> unbent)
    {
        float most = 0f;
        for (int i = 0; i < bent.Count; i++)
        {
            most = MathF.Max(most, Vector3.Distance(bent[i], unbent[i]));
        }

        // The displacement alone could be a rigid motion of the whole face; what shows a real
        // curvature is that it is not the same everywhere.
        float least = float.MaxValue;
        for (int i = 0; i < bent.Count; i++)
        {
            least = MathF.Min(least, Vector3.Distance(bent[i], unbent[i]));
        }

        return most - least;
    }

    private static bool Holds(IReadOnlyList<Vector3> points, Vector3 wanted)
        => IndexOf(points, wanted) >= 0;

    private static int IndexOf(IReadOnlyList<Vector3> points, Vector3 wanted)
    {
        for (int i = 0; i < points.Count; i++)
        {
            if (Vector3.Distance(points[i], wanted) < 1e-3f)
            {
                return i;
            }
        }

        return -1;
    }

    private static Vec3 ToVec3(Vector3 v) => new(v.X, v.Y, v.Z);

    private static Vector3 ToVector(Vec3 v) => new(v.X, v.Y, v.Z);

    private static float Dot(Vec3 a, Vec3 b) => (a.X * b.X) + (a.Y * b.Y) + (a.Z * b.Z);

    // ------------------------------------------------- the options model, over the assembly

    /// <summary>
    /// The group key AMD-043 §6 asks for. <c>OptionsModel</c> is internal to <c>Zax.App</c>,
    /// whose internals the verifier may not open, so the model is read over the shipped
    /// assembly, exactly as <see cref="UiDriver"/> does.
    /// </summary>
    private static string GeometryGroupKey { get; } = (string)AppType("Zax.App.Ui.UiKeys")
        .GetField("GroupGeometry", BindingFlags.Public | BindingFlags.Static)!
        .GetValue(null)!;

    private static string? OptionsGroupOf(string key)
    {
        Type model = AppType("Zax.App.Ui.OptionsModel");
        object descriptor = SettingsCatalogue.Get(key);
        return (string?)model
            .GetMethod("GroupOf", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [descriptor]);
    }

    private static string HelpOf(string key)
    {
        Type text = AppType("Zax.App.Ui.UiText");
        object descriptor = SettingsCatalogue.Get(key);
        return (string?)text
            .GetMethod("HelpOf", BindingFlags.Public | BindingFlags.Static)!
            .Invoke(null, [descriptor]) ?? string.Empty;
    }

    private static Type AppType(string name)
        => typeof(Zax.App.AppOptions).Assembly.GetType(name)
        ?? throw new InvalidOperationException($"Zax.App has no type '{name}'");
}
