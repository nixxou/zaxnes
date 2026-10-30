using System.Numerics;
using Zax.App.Input;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-043 — space bending: the four settings of REQ-SCN-066, the scope of REQ-SCN-068 on the
/// export harness, and the Zapper aim of REQ-INP-124; with the style members of AMD-040
/// REQ-LUA-114, which share their storage with the settings.
/// </summary>
public sealed class Amd043Tests
{
    private static readonly Vector2 Viewport = new(640f, 480f);

    /// <summary>
    /// The vectors of REQ-SCN-067 the depth limit does not touch, which REQ-INP-124 takes as
    /// its own acceptance criterion.
    /// </summary>
    public static TheoryData<int, int, int, bool, float, float, float> UnclampedVectors => new()
    {
        { 60, 0, 0, false, 128f, 120f, 0f },
        { 60, 0, 0, false, 256f, 120f, 0f },
        { 60, 0, 0, false, 0f, 120f, 0f },
        { 60, 0, 0, false, 256f, 120f, 10f },
        { -60, 0, 0, false, 256f, 120f, 0f },
        { 0, 60, 0, false, 128f, 240f, 0f },
        { 0, 60, 0, false, 128f, 0f, 0f },
        { 0, 0, 60, false, 0f, 120f, 0f },
        { 0, 0, 60, false, 128f, 240f, 0f },
        { 0, 0, 60, true, 0f, 0f, 0f },
        { 0, 0, 60, true, 128f, 240f, 0f },
    };

    // -------------------------------------------------------------- §2 settings

    /// <summary>
    /// REQ-SCN-066 with AMD-040 §3: the four settings are user preferences, default 0, applied
    /// immediately, and a value off the ladder is brought onto it — 45 gives 60.
    /// </summary>
    [Fact] // REQ-SCN-066
    public void REQ_SCN_066_the_four_settings_are_user_preferences_on_a_ladder_of_30_degrees()
    {
        SettingsModel model = new();

        foreach (string key in Curvature)
        {
            SettingDescriptor descriptor = SettingsCatalogue.Get(key);
            Assert.Equal(SettingScope.User, descriptor.Scope);
            Assert.Equal(SettingEffect.Immediate, descriptor.Effect);
            Assert.Equal(0, descriptor.DefaultValue.AsInt);
            Assert.Equal(0, model.GetInt(key));
        }

        // The acceptance criterion of REQ-SCN-066 and the examples of AMD-040 §3.
        model.SetInt(SettingKeys.SceneBendLeftRight, 45);
        Assert.Equal(60, model.GetInt(SettingKeys.SceneBendLeftRight));

        model.SetInt(SettingKeys.SceneBendLeftRight, 40);
        Assert.Equal(30, model.GetInt(SettingKeys.SceneBendLeftRight));

        model.SetInt(SettingKeys.SceneBendLeftRight, 15);
        Assert.Equal(0, model.GetInt(SettingKeys.SceneBendLeftRight));

        model.SetInt(SettingKeys.SceneBendLeftRight, 500);
        Assert.Equal(180, model.GetInt(SettingKeys.SceneBendLeftRight));

        model.SetInt(SettingKeys.SceneBendLeftRight, -500);
        Assert.Equal(-180, model.GetInt(SettingKeys.SceneBendLeftRight));

        // Thirteen values on the libretro side (AMD-043 §6), two for the arc centre.
        Assert.Equal(13, SettingsCatalogue.Get(SettingKeys.SceneBendLeftRight).LibretroValues!.Count);
        Assert.Equal(13, SettingsCatalogue.Get(SettingKeys.SceneBendTopBottom).LibretroValues!.Count);
        Assert.Equal(13, SettingsCatalogue.Get(SettingKeys.SceneArcHorizontal).LibretroValues!.Count);
        Assert.Equal(2, SettingsCatalogue.Get(SettingKeys.SceneArcCenter).LibretroValues!.Count);

        model.SetInt(SettingKeys.SceneArcCenter, 7);
        Assert.Equal(1, model.GetInt(SettingKeys.SceneArcCenter));
    }

    /// <summary>
    /// AMD-040 §3: the normalisation is "into the bounds, onto the nearest rung, ties to the
    /// even multiple" — 7,5 gives 8 rungs and 6,5 gives 6.
    /// </summary>
    [Theory] // REQ-LUA-114
    [InlineData(45d, 60)]
    [InlineData(15d, 0)]
    [InlineData(40d, 30)]
    [InlineData(-45d, -60)]
    [InlineData(-15d, 0)]
    [InlineData(500d, 180)]
    [InlineData(-500d, -180)]
    [InlineData(0d, 0)]
    [InlineData(29.9d, 30)]
    public void REQ_LUA_114_the_normalisation_rounds_ties_to_the_even_rung(double written, int expected)
        => Assert.Equal(expected, (int)SettingsCatalogue.BendDomain.Snap(written));

    /// <summary>
    /// REQ-SCN-066: the value survives a restart, which for a user preference means it is
    /// written to <c>settings.json</c> and read back.
    /// </summary>
    [Fact] // REQ-SCN-066, REQ-PERS-020
    public void REQ_SCN_066_the_curvature_survives_a_restart()
    {
        MemorySettingsStorage storage = new();
        SettingsModel model = new();
        model.SetInt(SettingKeys.SceneBendLeftRight, 90);
        model.SetInt(SettingKeys.SceneArcCenter, 1);
        Assert.True(new SettingsFile(storage).Save(model));

        string json = System.Text.Encoding.UTF8.GetString(storage.Read(SettingsFile.DefaultName)!);
        Assert.Contains(SettingKeys.SceneBendLeftRight, json, StringComparison.Ordinal);

        SettingsModel restarted = new();
        new SettingsFile(storage).Load(restarted, DateTimeOffset.UnixEpoch);
        Assert.Equal(90, restarted.GetInt(SettingKeys.SceneBendLeftRight));
        Assert.Equal(1, restarted.GetInt(SettingKeys.SceneArcCenter));
        Assert.Equal(0, restarted.GetInt(SettingKeys.SceneBendTopBottom));
    }

    // ------------------------------------------------------- §2 Lua (AMD-040)

    /// <summary>
    /// REQ-LUA-114: <c>Setting.VisualStyle</c> and the three style-parameter methods, with the
    /// acceptance criterion of the amendment, line by line.
    /// </summary>
    [Fact] // REQ-LUA-114
    public void REQ_LUA_114_the_style_members_answer_the_amendment_criterion()
    {
        using LuaRun run = new LuaProfile().Script("a", string.Empty).Start();

        Assert.Equal(0, run.Num("Setting.VisualStyle"));
        run.Do("Setting.VisualStyle = 1");
        Assert.Equal(1, run.Num("Setting.VisualStyle"));
        run.Do("Setting.VisualStyle = 5");
        Assert.Equal(0, run.Num("Setting.VisualStyle"));

        Assert.True(run.Flag("""Setting:SetVisualStyleParameter(1, "spl.bend_zx", 500)"""));
        Assert.Equal(180, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        run.Do("""Setting:SetVisualStyleParameter(1, "spl.bend_zx", 40)""");
        Assert.Equal(30, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        run.Do("""Setting:SetVisualStyleParameter(1, "spl.bend_zx", 45)""");
        Assert.Equal(60, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        run.Do("""Setting:SetVisualStyleParameter(1, "spl.bend_zx", 15)""");
        Assert.Equal(0, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        // An unknown style or an unknown identifier: false, and nothing recorded.
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(0, "spl.bend_zx", 30)"""));
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(7, "spl.bend_zx", 30)"""));
        Assert.False(run.Flag("""Setting:SetVisualStyleParameter(1, "inconnu", 30)"""));
        Assert.Equal(0, run.Num("""Setting:GetVisualStyleParameter(1, "inconnu")"""));
        Assert.Equal(0, run.Num("""Setting:GetVisualStyleParameter(0, "spl.bend_zx")"""));
        Assert.Equal(0, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zx")"""));

        // The four parameters of style 1 exist and go back to their default together.
        foreach (string id in (string[])["spl.bend_zx", "spl.bend_zy", "spl.arc_yx", "spl.arc_center"])
        {
            Assert.True(run.Flag($"""Setting:SetVisualStyleParameter(1, "{id}", 1000)"""));
            Assert.True(run.Num($"""Setting:GetVisualStyleParameter(1, "{id}")""") != 0d);
        }

        run.Do("Setting:ResetVisualStyleParameters(1)");
        foreach (string id in (string[])["spl.bend_zx", "spl.bend_zy", "spl.arc_yx", "spl.arc_center"])
        {
            Assert.Equal(0, run.Num($"""Setting:GetVisualStyleParameter(1, "{id}")"""));
        }

        // An unknown style changes nothing and raises nothing.
        run.Do("""Setting:SetVisualStyleParameter(1, "spl.arc_yx", 90)""");
        run.Do("Setting:ResetVisualStyleParameters(7)");
        Assert.Equal(90, run.Num("""Setting:GetVisualStyleParameter(1, "spl.arc_yx")"""));
    }

    /// <summary>
    /// AMD-040 §3: writing from Lua and writing from the interface come to the same thing —
    /// one storage, the settings model of SPEC-09.
    /// </summary>
    [Fact] // REQ-LUA-114, REQ-SCN-066
    public void REQ_LUA_114_lua_and_the_interface_share_one_storage()
    {
        using LuaRun run = new LuaProfile().Script("a", string.Empty).Start();
        SettingsModel model = run.Settings.Model;

        run.Do("""Setting:SetVisualStyleParameter(1, "spl.arc_center", 1)""");
        Assert.Equal(1, model.GetInt(SettingKeys.SceneArcCenter));

        model.SetInt(SettingKeys.SceneBendTopBottom, -90);
        Assert.Equal(-90, run.Num("""Setting:GetVisualStyleParameter(1, "spl.bend_zy")"""));
    }

    // ----------------------------------------------------------- §3 the scope

    /// <summary>
    /// REQ-SCN-068, harness criterion: with the left-right bend at 90°, the exported vertices
    /// of a tile standing at <c>(256, 120, 0)</c> are at <c>B(P)</c>, and the exported HUD is
    /// the one a null curvature gives.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-067
    public void REQ_SCN_068_the_exported_vertices_are_bent_and_the_hud_is_not()
    {
        // No margin, so the tile standing against the right edge of the frame survives the
        // conversion's own clipping and its corner really is (256, 120, 0).
        SceneState scene = CornerScene(new Vec4F(0f, 0f, 0f, 0f));
        CameraView view = View(scene);

        IReadOnlyList<ExportedObject> flat = SceneExport.Objects(scene, view);
        scene.Settings.BendLeftRight = 90;
        IReadOnlyList<ExportedObject> bentFrame = SceneExport.Objects(scene, view);
        SpaceBend bend = scene.Settings.Bend;

        Assert.Equal(flat.Count, bentFrame.Count);
        Assert.Contains(bentFrame, static o => o.Hud);
        Assert.Contains(bentFrame, static o => !o.Hud);

        for (int i = 0; i < bentFrame.Count; i++)
        {
            ExportedObject before = flat[i];
            ExportedObject after = bentFrame[i];

            if (after.Hud)
            {
                // REQ-SCN-068: the HUD plane stays flat, and exactly where it was.
                Assert.False(after.Bent);
                Assert.Equal(before.Positions, after.Positions);
                Assert.Equal(before.Normals, after.Normals);
                continue;
            }

            Assert.True(after.Bent);
            for (int v = 0; v < after.Positions.Count; v++)
            {
                Vec3 expected = bend.Apply(ToVec3(after.Unbent[v]));
                Assert.True(
                    Vector3.Distance(ToVector(expected), after.Positions[v]) < 1e-3f,
                    $"vertex {v} of object {i}: {after.Positions[v]} is not B({after.Unbent[v]}).");
            }
        }

        // The corner of the tile the criterion names is in the export, at B(P).
        Vector3 corner = new(256f, 120f, 0f);
        ExportedObject tile = Assert.Single(
            bentFrame,
            o => !o.Hud && o.Unbent.Any(p => Vector3.Distance(p, corner) < 1e-3f));
        int index = tile.Unbent.ToList().FindIndex(p => Vector3.Distance(p, corner) < 1e-3f);
        Assert.True(
            Vector3.Distance(tile.Positions[index], ToVector(bend.Apply(ToVec3(corner)))) < 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-068: the clipping of REQ-SCN-036 is judged on the positions <b>before</b>
    /// bending, so the corner tile is still rendered when its bent image has left the visible
    /// rectangle. With the horizontal arc at 60° on the bottom edge,
    /// <c>B((0, 8, 0)) = (0,32 ; −17,70 ; 0)</c>.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-036
    public void REQ_SCN_068_the_clipping_is_judged_before_bending()
    {
        SpaceBend arc = new(0f, 0f, 60f, arcAtEdge: true);
        Vec3 image = arc.Apply(new Vec3(0f, 8f, 0f));
        Assert.Equal(0.32f, image.X, 0.05f);
        Assert.Equal(-17.70f, image.Y, 0.05f);
        Assert.Equal(0f, image.Z, 0.05f);

        SceneState scene = CornerScene();
        scene.Settings.ArcHorizontal = 60;
        scene.Settings.ArcAtEdge = true;

        VisibleRect rect = scene.VisibleRectangle;
        ExportedObject corner = Assert.Single(
            SceneExport.Objects(scene, View(scene)),
            o => !o.Hud && o.Unbent.Any(p => Vector3.Distance(p, new Vector3(0f, 8f, 0f)) < 1e-3f));

        // Part of the tile survives the margins before bending …
        Assert.Contains(corner.Unbent, p => rect.Contains(p.X, p.Y));

        // … while its whole bent image is below the rectangle, which is exactly what would have
        // thrown it away had the test been made after the curvature.
        Assert.True(corner.Clipped);
        Assert.All(corner.Positions, p => Assert.False(rect.Contains(p.X, p.Y)));
    }

    /// <summary>
    /// REQ-SCN-068: the backdrop is subdivided into at least 32 × 32 cells per face, so its
    /// faces follow the curvature instead of staying flat quads.
    /// </summary>
    [Fact] // REQ-SCN-068, REQ-SCN-048
    public void REQ_SCN_068_the_backdrop_is_subdivided_and_bends()
    {
        SceneState scene = CornerScene();
        scene.Backdrop.Enabled = true;
        scene.Settings.BendLeftRight = 90;

        ExportedObject backdrop = SceneExport.Objects(scene, View(scene))[^1];
        Assert.True(backdrop.Bent);
        Assert.False(backdrop.Clipped);

        // Six faces of at least 33 × 33 vertices.
        Assert.True(
            backdrop.Positions.Count >= 6 * 33 * 33,
            $"the backdrop exported {backdrop.Positions.Count} vertices.");

        // A face is not a tilted quad: its middle leaves the plane of its corners.
        float straightness = 0f;
        for (int i = 0; i < backdrop.Positions.Count; i++)
        {
            straightness = MathF.Max(
                straightness,
                Vector3.Distance(backdrop.Positions[i], backdrop.Unbent[i]));
        }

        Assert.True(straightness > 1f, "the backdrop did not bend at all.");
    }

    // ------------------------------------------------------------- §4 aiming

    /// <summary>
    /// REQ-INP-124, acceptance criterion: for every vector of REQ-SCN-067 the depth limit does
    /// not touch, a ray through <c>B(P)</c> aims at a point less than 0,5 scene unit from
    /// <c>P</c>, with <c>z₀</c> the depth of <c>P</c>.
    /// </summary>
    [Theory] // REQ-INP-124
    [MemberData(nameof(UnclampedVectors))]
    public void REQ_INP_124_a_ray_through_the_image_aims_at_the_point(
        int bendLeftRight,
        int bendTopBottom,
        int arcHorizontal,
        bool arcAtEdge,
        float x,
        float y,
        float z)
    {
        SpaceBend bend = new(bendLeftRight, bendTopBottom, arcHorizontal, arcAtEdge);
        CameraView view = DefaultView();

        Vec3 point = new(x, y, z);
        Vector2 pointer = Project(view, ToVector(bend.Apply(point)));

        Assert.True(
            ZapperAim.TryIntersect(view, pointer, Viewport, new Vector3(0f, 0f, z), bend, 1f, out Vector2 aimed),
            "no candidate at all.");

        Assert.True(
            Vector2.Distance(aimed, new Vector2(x, y)) < 0.5f,
            $"aimed at {aimed} instead of ({x}, {y}).");
    }

    /// <summary>
    /// REQ-INP-124: with no curvature the aim is the one of REQ-INP-063, to the pixel.
    /// </summary>
    [Fact] // REQ-INP-124, REQ-INP-063
    public void REQ_INP_124_a_null_curvature_is_the_aim_of_REQ_INP_063()
    {
        CameraView view = DefaultView();
        Vector2 pointer = Project(view, new Vector3(200f, 60f, 0f));

        Assert.True(ZapperAim.TryIntersect(view, pointer, Viewport, Vector3.Zero, out Vector2 flat));
        Assert.True(ZapperAim.TryIntersect(
            view,
            pointer,
            Viewport,
            Vector3.Zero,
            SpaceBend.None,
            1f,
            out Vector2 bent));

        Assert.Equal(flat, bent);
    }

    /// <summary>
    /// REQ-INP-124 and REQ-INP-064: a candidate outside the aiming window is handed over as it
    /// is — an off-screen shot — and a ray with no candidate at all keeps the last valid
    /// position.
    /// </summary>
    [Fact] // REQ-INP-124, REQ-INP-064
    public void REQ_INP_124_off_screen_and_no_candidate_follow_REQ_INP_064()
    {
        SpaceBend bend = new(30f, 0f, 0f, arcAtEdge: false);
        CameraView view = DefaultView();

        // Well outside the window of ±144 × ±136 around (128, 128).
        Vector3 outside = ToVector(bend.Apply(new Vec3(-120f, 120f, 0f)));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(-120f, 120f)));
        Assert.True(ZapperAim.TryIntersect(
            view,
            Project(view, outside),
            Viewport,
            Vector3.Zero,
            bend,
            1f,
            out Vector2 aimed));
        Assert.True(
            Vector2.Distance(aimed, new Vector2(-120f, 120f)) < 0.5f,
            $"the off-screen candidate was changed: {aimed}.");

        // No candidate at all: the aim plane is far outside the part of the scene the
        // curvature touches, so nothing on the ray has a preimage there.
        ZapperState zapper = new();
        zapper.Update(true, view, Project(view, ToVector(bend.Apply(new Vec3(128f, 120f, 0f)))), Viewport,
            false, Vector3.Zero, 1f, bend);
        ZapperPosition valid = zapper.Position;
        Assert.True(zapper.HasPosition);

        Vector3 unreachable = new(0f, 0f, 100000f);
        Assert.False(ZapperAim.TryIntersect(
            view,
            new Vector2(320f, 240f),
            Viewport,
            unreachable,
            bend,
            1f,
            out _));

        zapper.Update(true, view, new Vector2(320f, 240f), Viewport, false, unreachable, 1f, bend);
        Assert.Equal(valid, zapper.Position);
    }

    /// <summary>The aiming window of REQ-INP-124: the NES image widened by 16 units.</summary>
    [Fact] // REQ-INP-124
    public void REQ_INP_124_the_aiming_window_is_the_nes_image_widened_by_sixteen_units()
    {
        Assert.Equal(144f, ZapperAim.WindowHalfWidth);
        Assert.Equal(136f, ZapperAim.WindowHalfHeight);

        Assert.True(ZapperAim.InAimingWindow(new Vector2(-16f, -8f)));
        Assert.True(ZapperAim.InAimingWindow(new Vector2(272f, 264f)));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(-17f, 128f)));
        Assert.False(ZapperAim.InAimingWindow(new Vector2(128f, 265f)));
    }

    // ------------------------------------------------------------- fixtures

    private static readonly string[] Curvature =
    [
        SettingKeys.SceneBendLeftRight,
        SettingKeys.SceneBendTopBottom,
        SettingKeys.SceneArcHorizontal,
        SettingKeys.SceneArcCenter,
    ];

    /// <summary>
    /// A frame with the corner tile of REQ-SCN-068 — screen <c>(0, 232)</c>, whose
    /// bottom-left-front corner is <c>(0, 8, 0)</c> — a tile whose bottom-right-front corner is
    /// <c>(256, 120, 0)</c>, and one HUD tile.
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
    /// The pointer a scene point projects to. The scene frame is left-handed, so the clip
    /// position is built from <c>(x, y, −z)</c> like the vertex shader's (REQ-SCN-001).
    /// </summary>
    private static Vector2 Project(in CameraView view, Vector3 point)
    {
        Vector4 clip = Vector4.Transform(new Vector3(point.X, point.Y, -point.Z), view.ViewProjection);
        float x = clip.X / clip.W;
        float y = clip.Y / clip.W;
        return new Vector2((x + 1f) * 0.5f * Viewport.X, (1f - y) * 0.5f * Viewport.Y);
    }

    private static Vec3 ToVec3(Vector3 v) => new(v.X, v.Y, v.Z);

    private static Vector3 ToVector(Vec3 v) => new(v.X, v.Y, v.Z);
}
