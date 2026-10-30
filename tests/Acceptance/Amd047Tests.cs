using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Profiles.Wire;
using Zax.Render;
using Zax.Settings;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-047 — negative clipping margins: the domain of REQ-PERS-087 (any finite real, NaN and
/// ±∞ falling back on the default), the three automatic criteria of the amendment — the Lua and
/// profile round trip, the camera vector <c>T′ / E / P* / R</c>, the tile at <c>(−8, 100)</c> —
/// and the empty visible rectangle. It opens Q-080 on the screen range the core really emits.
/// </summary>
public sealed class Amd047Tests
{
    private static readonly Vector2 Viewport = new(640f, 480f);

    /// <summary>The margins of the amendment's second criterion: (−128 ; −80 ; −128 ; 0).</summary>
    private static readonly Vec4F RingMargins = new(-128f, -80f, -128f, 0f);

    /// <summary>The margins of the third criterion: (−16 ; 8,1 ; 8,1 ; 0,1).</summary>
    private static readonly Vec4F WideLeftMargins = new(-16f, 8.1f, 8.1f, 0.1f);

    /// <summary>The defaults of §4.1.5, which the third criterion contrasts with.</summary>
    private static readonly Vec4F DefaultMargins = new(0.1f, 8.1f, 8.1f, 0.1f);

    /// <summary>The four keys, in the index order 0 left, 1 bottom, 2 right, 3 top.</summary>
    private static readonly string[] Margins =
    [
        SettingKeys.ClipLeft, SettingKeys.ClipBottom, SettingKeys.ClipRight, SettingKeys.ClipTop,
    ];

    // ------------------------------------------------- §3 the catalogue domain

    /// <summary>
    /// AMD-047 §3: the column "Domaine" of the four margins becomes "réel fini ; non fini
    /// (NaN, ±∞) → valeur par défaut". No bound is left, and neither the scope, the kind, the
    /// defaults nor the Lua and profile paths move.
    /// </summary>
    [Fact] // REQ-PERS-087, REQ-PERS-001
    public void REQ_PERS_087_the_domain_of_the_four_margins_is_the_finite_reals()
    {
        float[] defaults = [0.1f, 8.1f, 8.1f, 0.1f];
        string[] luaPaths = ["Clipping[0]", "Clipping[1]", "Clipping[2]", "Clipping[3]"];
        string[] fields = ["4.1", "4.2", "4.3", "4.4"];

        for (int i = 0; i < Margins.Length; i++)
        {
            SettingDescriptor descriptor = SettingsCatalogue.Get(Margins[i]);

            Assert.Equal(SettingKind.Float, descriptor.Kind);
            Assert.Equal(SettingScope.Profile, descriptor.Scope);
            Assert.Equal(SettingEffect.Immediate, descriptor.Effect);
            Assert.Equal(defaults[i], descriptor.DefaultValue.AsFloat, 4);
            Assert.Equal(luaPaths[i], descriptor.LuaPath);
            Assert.Equal(fields[i], descriptor.ProfileField);

            // No bound left: 256 and 248 are reference dimensions, not a domain.
            Assert.True(descriptor.Domain.FiniteOnly);
            Assert.Null(descriptor.Domain.Minimum);
            Assert.Null(descriptor.Domain.Maximum);
            Assert.Null(descriptor.Domain.Options);
            Assert.Equal(OutOfDomainRule.UseDefault, descriptor.Domain.Rule);
        }
    }

    // ------------------------------- §2 first criterion: the Lua and profile round trip

    /// <summary>
    /// REQ-PERS-087, first criterion: a script writes <c>Setting.Clipping[1] = -80</c> and reads
    /// −80 back; <c>Clipping.x = -128</c> leaves −128 in <c>Setting.Clipping[0]</c>. The two
    /// other write paths of SPEC-09 §4.2 — the indexer of the global and <c>Clipping.target</c>
    /// — behave the same, and the conversion engine sees the margins as written.
    /// </summary>
    [Fact] // REQ-PERS-087, REQ-PERS-007, REQ-LUA-089
    public void REQ_PERS_087_a_script_keeps_a_negative_margin_and_reads_it_back()
    {
        using LuaRun run = new LuaProfile().Script("probe", "-- no hook").Load();

        run.Do("Setting.Clipping[1] = -80");
        Assert.Equal(-80.0, run.Num("Setting.Clipping[1]"), 4);
        Assert.Equal(-80f, run.Settings.Clipping(1), 4);

        run.Do("Clipping.x = -128");
        Assert.Equal(-128.0, run.Num("Setting.Clipping[0]"), 4);
        Assert.Equal(-128.0, run.Num("Clipping[0]"), 4);

        // The indexer of the global and the vector member take negatives too.
        run.Do("Clipping[2] = -128");
        Assert.Equal(-128.0, run.Num("Setting.Clipping.z"), 4);

        run.Do("Clipping.target = Color(-128, -80, -128, 0)");
        Assert.Equal(new Vec4F(-128f, -80f, -128f, 0f), run.Settings.ClippingVector);
        Assert.Equal(-80.0, run.Num("Clipping.target.g"), 4);

        // A script write reaches the conversion engine (REQ-PERS-007).
        Assert.Equal(new Vec4F(-128f, -80f, -128f, 0f), run.Engine.Settings.Clipping);
    }

    /// <summary>
    /// REQ-PERS-087, first criterion, last sentence: a non-finite value is out of domain and no
    /// rule repairs it, so the margin takes its own default (REQ-PERS-003) — and the three other
    /// margins keep whatever they held.
    /// </summary>
    [Fact] // REQ-PERS-087, REQ-PERS-003
    public void REQ_PERS_087_a_script_writing_a_non_finite_margin_gets_the_default()
    {
        using LuaRun run = new LuaProfile().Script("probe", "-- no hook").Load();

        run.Do("Clipping.target = Color(-128, -80, -128, -4)");
        run.Do("Setting.Clipping[0] = 0/0");
        Assert.Equal(0.1, run.Num("Setting.Clipping[0]"), 4);
        Assert.Equal(-80.0, run.Num("Setting.Clipping[1]"), 4);

        run.Do("Setting.Clipping[1] = 1/0");
        Assert.Equal(8.1, run.Num("Setting.Clipping[1]"), 4);

        run.Do("Setting.Clipping[2] = -1/0");
        Assert.Equal(8.1, run.Num("Setting.Clipping[2]"), 4);

        // The top margin, never written with a non-finite value, kept its −4.
        Assert.Equal(-4.0, run.Num("Setting.Clipping[3]"), 4);
    }

    /// <summary>
    /// REQ-PERS-087, first criterion, profile half: field 4 of the settings message
    /// (SPEC-03 §4.6) carries (−128 ; −80 ; −128 ; 0) through the wire unchanged, and
    /// REQ-PROF-041 leaves those very values in the model.
    /// </summary>
    [Fact] // REQ-PERS-087, REQ-PROF-041
    public void REQ_PERS_087_a_profile_field_four_loads_its_negative_margins()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Settings!.Clipping = RingMargins;

        // Field 4 is a Vec4 of floats: −128 and −80 are exact, so the round trip is faithful.
        GameProfile reloaded = ProfileWireCodec
            .Decode(ProfileWireCodec.Encode(profile))
            .Profile;
        Assert.Equal(RingMargins, reloaded.Settings!.Clipping);

        SettingsModel model = new();
        model.ApplyProfile(ProfileSettingApplication.ToSettingValues(reloaded.Settings));

        Assert.Equal(-128f, model.GetFloat(SettingKeys.ClipLeft), 4);
        Assert.Equal(-80f, model.GetFloat(SettingKeys.ClipBottom), 4);
        Assert.Equal(-128f, model.GetFloat(SettingKeys.ClipRight), 4);
        Assert.Equal(0f, model.GetFloat(SettingKeys.ClipTop), 4);
    }

    // ------------------------------------- §2 second criterion: the camera vector

    /// <summary>
    /// REQ-PERS-087, second criterion: TPV, smoothing settled, setpoints rx = −60°, ry = 0,
    /// rz = 0, d = 265,51, target <c>C = (128, 120, 0)</c> and margins (−128 ; −80 ; −128 ; 0)
    /// give <c>T′ = (128 ; 80 ; 0)</c>, <c>E ≈ (128,0 ; −149,9 ; −132,8)</c> ±0,5,
    /// <c>P* ≈ (128 ; 41,5)</c> ±0,6 and <c>R = [−128, 384] × [−80, 240]</c>.
    /// </summary>
    [Fact] // REQ-PERS-087, REQ-SCN-007, REQ-SCN-008, REQ-SCN-039
    public void REQ_PERS_087_the_camera_vector_follows_the_negative_margins()
    {
        SceneCamera camera = new();
        camera.Current.Angles = new Vector3(-60f, 0f, 0f);
        camera.Current.Distance = 265.51f;
        camera.Current.Target = SceneConstants.Center;
        camera.SnapToSetpoints();

        CameraView view = camera.BuildView(RingMargins, Viewport.X / Viewport.Y);

        // Step 2 of REQ-SCN-007: T′ = T + ((c0 − c2)/2, (c1 − c3)/2, 0).
        Assert.Equal(128f, view.Target.X, 1e-3f);
        Assert.Equal(80f, view.Target.Y, 1e-3f);
        Assert.Equal(0f, view.Target.Z, 1e-3f);

        // Step 3: E = T′ − d·f.
        Assert.Equal(128f, view.Eye.X, 0.5f);
        Assert.Equal(-149.9f, view.Eye.Y, 0.5f);
        Assert.Equal(-132.8f, view.Eye.Z, 0.5f);

        // REQ-SCN-008: the aimed point, the observable the amendment is after.
        Assert.Equal(128f, view.AimPoint.X, 0.6f);
        Assert.Equal(41.5f, view.AimPoint.Y, 0.6f);

        // REQ-SCN-039: R = [c0, 256 − c2] × [c1, 240 − c3], which overflows the NES picture.
        VisibleRect rect = VisibleRect.From(RingMargins);
        Assert.Equal(-128f, rect.MinX, 1e-3f);
        Assert.Equal(384f, rect.MaxX, 1e-3f);
        Assert.Equal(-80f, rect.MinY, 1e-3f);
        Assert.Equal(240f, rect.MaxY, 1e-3f);

        // Without the amendment the margins would clamp to 0 and the target would sit 40 units
        // higher — the defect of §1.
        CameraView clamped = camera.BuildView(new Vec4F(0f, 0f, 0f, 0f), Viewport.X / Viewport.Y);
        Assert.Equal(120f, clamped.Target.Y, 1e-3f);
    }

    // --------------------------------------- §2 third criterion: the tile at (−8, 100)

    /// <summary>
    /// REQ-PERS-087, third criterion: with (−16 ; 8,1 ; 8,1 ; 0,1) a background tile emitted at
    /// <c>(x, y) = (−8, 100)</c> is kept by REQ-E3D-002 and none of its fragments is dropped by
    /// REQ-SCN-036 — its scene footprint <c>x ∈ [−8, 0]</c>, <c>y ∈ [140, 148]</c> lies inside
    /// <c>R = [−16 ; 247,9] × [8,1 ; 239,9]</c>. With the defaults the same tile is dropped by
    /// REQ-E3D-002.
    /// </summary>
    /// <remarks>
    /// Q-080: the criterion names REQ-E3D-002, the filter, and REQ-SCN-036, the fragment cut, so
    /// both are checked on their own formulas. The core never emits a background record at
    /// x = −8 — SPEC-02 REQ-CORE-037 gives x = 8k − fx with k ≤ 32 and fx ≤ 7, hence
    /// x ∈ [−7, 255] — and such a record would land in the margin column of the background grid,
    /// which the engine keeps free for the growth of the segmentation (SPEC-04 §2).
    /// </remarks>
    [Fact] // REQ-PERS-087, REQ-E3D-002, REQ-SCN-036
    public void REQ_PERS_087_a_negative_left_margin_keeps_the_tile_at_minus_eight()
    {
        Assert.True(ConversionEngine.IsInsideClipArea(-8, 100, WideLeftMargins));
        Assert.False(ConversionEngine.IsInsideClipArea(-8, 100, DefaultMargins));

        // The same filter still drops what lies beyond the widened area.
        Assert.False(ConversionEngine.IsInsideClipArea(-24, 100, WideLeftMargins));

        // REQ-SCN-036 on the four corners of the tile's scene footprint (REQ-E3D-410).
        VisibleRect rect = VisibleRect.From(WideLeftMargins);
        Assert.Equal(-16f, rect.MinX, 1e-3f);
        Assert.Equal(247.9f, rect.MaxX, 1e-3f);

        const float hs = ConversionGrid.SceneHeight;
        foreach (float x in new[] { -8f, 0f })
        {
            foreach (float y in new[] { hs - 108f, hs - 100f })
            {
                Assert.True(rect.Contains(x, y), $"({x}, {y}) must survive REQ-SCN-036.");
            }
        }
    }

    /// <summary>
    /// The same criterion end to end, on a row the core really emits (Q-080): a background tile
    /// at <c>(100, 240)</c> is dropped by the default bottom margin — <c>y ≤ 248 − 8,1</c> — and
    /// kept, whole, once that margin is −16. This is the border a negative margin uncovers.
    /// </summary>
    [Fact] // REQ-PERS-087, REQ-E3D-002, REQ-SCN-036
    public void REQ_PERS_087_a_negative_bottom_margin_uncovers_the_bottom_row()
    {
        Assert.Equal(0, Converted(DefaultMargins));
        Assert.Equal(1, Converted(new Vec4F(0.1f, -16f, 8.1f, 0.1f)));

        // Its scene footprint y ∈ [0, 8] is inside R = [−16 ; 239,9] and outside the default one.
        VisibleRect wide = VisibleRect.From(new Vec4F(0.1f, -16f, 8.1f, 0.1f));
        Assert.True(wide.Contains(100f, 0f));
        Assert.True(wide.Contains(108f, 8f));
        Assert.False(VisibleRect.From(DefaultMargins).Contains(100f, 0f));

        static int Converted(Vec4F margins)
        {
            ConversionEngine engine = new();
            engine.Settings.Clipping = margins;
            return new SyntheticBatch().Background('A', 100, 240).Convert(engine).BackgroundTileCount;
        }
    }

    // ------------------------------------------------ §2 the empty visible rectangle

    /// <summary>
    /// REQ-PERS-087, last bullet: margins for which <c>R</c> is empty — here
    /// <c>c0 + c2 = 300 ≥ 256</c> and <c>c1 + c3 = 300 ≥ 240</c> — raise no error. The rectangle
    /// contains nothing, the camera still builds a finite view and the scene still exports its
    /// HUD and its backdrop.
    /// </summary>
    [Fact] // REQ-PERS-087, REQ-SCN-036, REQ-SCN-039
    public void REQ_PERS_087_an_empty_visible_rectangle_raises_no_error()
    {
        Vec4F empty = new(200f, 200f, 100f, 100f);
        VisibleRect rect = VisibleRect.From(empty);

        Assert.True(rect.Width < 0f, $"R should be empty, width {rect.Width}.");
        Assert.True(rect.Height < 0f, $"R should be empty, height {rect.Height}.");
        Assert.False(rect.Contains(128f, 120f));
        Assert.False(rect.Contains(rect.Center.X, rect.Center.Y));

        SceneState scene = HudAndTileScene();
        scene.Clipping = empty;
        scene.Backdrop.Enabled = true;

        CameraView view = scene.Camera.BuildView(scene.Clipping, Viewport.X / Viewport.Y);
        foreach (Vector3 point in new[] { view.Eye, view.Target, view.AimPoint, view.Forward })
        {
            Assert.True(
                float.IsFinite(point.X) && float.IsFinite(point.Y) && float.IsFinite(point.Z),
                $"{point} must stay finite with an empty R.");
        }

        IReadOnlyList<ExportedObject> objects = SceneExport.Objects(scene, view);

        // The HUD is never cut by R (REQ-SCN-044) and the backdrop carries no clipping either.
        Assert.Contains(objects, o => o.Hud);
        Assert.Contains(objects, o => !o.Hud && !o.Clipped);

        // Every ordinary tile is outside R, which is what "the picture MAY be empty" means.
        foreach (ExportedObject exported in objects.Where(o => o.Clipped))
        {
            Assert.All(exported.Positions, p => Assert.False(rect.Contains(p.X, p.Y)));
        }
    }

    /// <summary>One L1 tile and one HUD tile, both well inside the default margins.</summary>
    private static SceneState HudAndTileScene()
    {
        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.Ui)
            .Tile(SceneFixture.Solid(1, 0), 64, 120)
            .Tile(SceneFixture.Solid(1, 1), 120, 120);

        return fixture.Build();
    }
}

/// <summary>
/// The one AMD-047 case that needs a GL 3.3 core context: an empty visible rectangle must not
/// stop the HUD, the background or the skybox from being drawn.
/// </summary>
/// <remarks>Its own collection, so that two contexts are never created at once.</remarks>
[Collection("opengl-acceptance")]
public sealed class Amd047CaptureTests
{
    /// <summary>
    /// REQ-PERS-087, last bullet: with <c>R</c> empty the renderer draws the frame without
    /// error, the HUD tiles are still drawn and the background — the NES colour, then the
    /// colour skybox of REQ-SCN-046 — still fills the picture.
    /// </summary>
    [GlFact] // REQ-PERS-087, REQ-SCN-036, REQ-SCN-044, REQ-SCN-046
    public void REQ_PERS_087_an_empty_rectangle_keeps_the_hud_the_background_and_the_skybox()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x22)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.Ui, columns: 4, rows: 1)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 12, 10, 4, 1, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        // R = [200, 156] × [200, 140]: empty on both axes.
        scene.Clipping = new Vec4F(200f, 200f, 100f, 100f);
        Assert.True(scene.VisibleRectangle.Width < 0f);

        Shot shot = target.Render(renderer, scene);
        Assert.Equal(Silk.NET.OpenGL.GLEnum.NoError, target.Gl.GetError());
        Assert.True(renderer.Statistics.HudTiles > 0, "the HUD must still be drawn.");
        Assert.Equal(SceneFixture.NesPacked(0x22), shot.Packed(2, 2));

        // And the colour skybox still paints the background (REQ-SCN-046).
        scene.Settings.Skybox = SkyboxKind.Color;
        scene.Settings.CustomBackgroundColor = new Rgb24(255, 0, 0);
        Shot sky = target.Render(renderer, scene);
        Assert.Equal(Silk.NET.OpenGL.GLEnum.NoError, target.Gl.GetError());
        Assert.Equal(0xFF0000, sky.Packed(2, 2));
    }
}
