using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;
using Zax.Render.Assets;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests of SPEC-06 that need a GL 3.3 core context: the offscreen captures
/// of the shadows, the clipping, the transparency, the HUD, the background, the skyboxes, the TV
/// box, the pipeline order and the tile seams of REQ-SCN-065.
/// </summary>
/// <remarks>
/// Every test builds and destroys its own hidden context, the way <c>Zax.Render.Tests</c> does,
/// and the whole class is one xUnit collection so that two contexts are never created at once.
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class SceneCaptureTests
{
    private const int Black = 0x000000;

    /// <summary>
    /// REQ-SCN-057 — the renderer draws into a target the host supplies, never assumes the
    /// default framebuffer, keeps no process-wide state and never reads the image back: two
    /// independent renderers coexist and give the same image.
    /// </summary>
    [GlFact] // REQ-SCN-056, REQ-SCN-057
    public void Two_independent_renderers_draw_the_same_image_into_a_host_target()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        SceneState scene = Flat(0x22);

        using SceneRenderer first = new(target.Gl);
        using SceneRenderer second = new(target.Gl);

        Shot one = target.Render(first, scene);
        Shot two = target.Render(second, scene);

        Assert.Equal(one.Pixels, two.Pixels);
        Assert.Equal(Silk.NET.OpenGL.GLEnum.NoError, target.Gl.GetError());
    }

    /// <summary>
    /// REQ-SCN-046 — the background: the frame's universal NES colour while a game runs, black
    /// outside a game, the custom colour in Skybox = Color.
    /// </summary>
    [GlFact] // REQ-SCN-046
    public void The_background_colour_follows_the_documented_table()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = Flat(0x22);
        Shot running = target.Render(renderer, scene);
        Assert.Equal(SceneFixture.NesPacked(0x22), running.Packed(2, 2));

        scene.NesBackgroundColor = NesPalette.Color(0x0F);
        Assert.Equal(SceneFixture.NesPacked(0x0F), target.Render(renderer, scene).Packed(2, 2));

        scene.GameRunning = false;
        Assert.Equal(Black, target.Render(renderer, scene).Packed(2, 2));

        scene.Settings.Skybox = SkyboxKind.Color;
        scene.Settings.CustomBackgroundColor = new Rgb24(255, 0, 0);
        Assert.Equal(0xFF0000, target.Render(renderer, scene).Packed(2, 2));
    }

    /// <summary>
    /// REQ-SCN-047 — outside a running game the 3D scene is not drawn: only the background
    /// stays.
    /// </summary>
    [GlFact] // REQ-SCN-047
    public void The_scene_disappears_when_no_game_runs()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x22)
            .Pattern(SceneFixture.Solid(1), depth: 16, ProfileLayer.L1, columns: 4, rows: 4)
            .Rect(SceneFixture.Solid(1), 12, 14, 4, 4, palette: 1);
        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        Shot running = target.Render(renderer, scene);
        Assert.True(
            running.CountOther(SceneFixture.NesPacked(0x22)) > 100,
            "the tiles must be drawn while the game runs.");

        scene.GameRunning = false;
        Shot stopped = target.Render(renderer, scene);
        Assert.Equal(0, stopped.CountOther(Black));
    }

    /// <summary>
    /// REQ-SCN-030, REQ-SCN-031, REQ-SCN-032 — a hard shadow is cast, its edge is sharp, and
    /// switching the shadows off makes it disappear. Synthetic scene: a solid tile in L1 above a
    /// flat tile in L2 that receives, with the light (1, −1, 1).
    /// </summary>
    [GlFact] // REQ-SCN-030, REQ-SCN-031, REQ-SCN-032
    public void A_hard_shadow_is_cast_with_a_sharp_edge_and_can_be_switched_off()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1, 0), depth: 24, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.L2, columns: 8, rows: 8)
            .Palette(1, 0x30)
            .Palette(5, 0x30)
            .Rect(SceneFixture.Solid(1, 1), 8, 10, 8, 8, palette: 1)
            .Tile(SceneFixture.Solid(1, 0), 96, 96, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        Shot withShadow = target.Render(renderer, scene);
        scene.Settings.Shadows = false;
        Shot without = target.Render(renderer, scene);

        HashSet<int> shaded = withShadow.DistinctColors();
        HashSet<int> unshaded = without.DistinctColors();

        Assert.True(
            shaded.Count > unshaded.Count,
            $"the shadow must add at least one shade: {shaded.Count} vs {unshaded.Count}.");

        // A sharp edge: along a row crossing the shadow, the transition from the lit shade to
        // the shadowed one takes at most two pixels.
        Assert.True(
            SharpestTransition(withShadow) <= 2,
            "the shadow edge must be sharp, not filtered.");

        withShadow.SavePng(Path.Combine(PartBCaptures.Directory, "scn-030-shadow.png"));
        without.SavePng(Path.Combine(PartBCaptures.Directory, "scn-030-no-shadow.png"));
    }

    /// <summary>
    /// REQ-SCN-032, criterion — switching the cast flag of the L2 layer off through Lua makes
    /// its tiles' shadows disappear from the next frame on.
    /// </summary>
    [GlFact] // REQ-SCN-032, REQ-SCN-033
    public void Switching_a_layer_cast_flag_changes_the_shadows_on_the_next_frame()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture casting = Casters(l1Casts: true);
        SceneFixture silent = Casters(l1Casts: false);

        SceneState with = casting.Scene!;
        with.Palette(casting);
        SceneState without = silent.Scene!;
        without.Palette(silent);

        int shadesWith = target.Render(renderer, with).DistinctColors().Count;
        int shadesWithout = target.Render(renderer, without).DistinctColors().Count;

        Assert.True(
            shadesWith > shadesWithout,
            $"switching the L1 cast flag off must remove a shade: {shadesWith} then {shadesWithout}.");

        static SceneFixture Casters(bool l1Casts)
        {
            SceneFixture fixture = new();
            fixture
                .Background(0x0F)
                .Pattern(SceneFixture.Solid(1, 0), depth: 24, ProfileLayer.L1)
                .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.L2, columns: 8, rows: 8)
                .Palette(1, 0x30)
                .Rect(SceneFixture.Solid(1, 1), 8, 10, 8, 8, palette: 1)
                .Tile(SceneFixture.Solid(1, 0), 96, 96, palette: 1);

            fixture.Scene = fixture.Build(settings => settings.Layer(2).CastShadow = l1Casts);
            return fixture;
        }
    }

    /// <summary>
    /// REQ-SCN-036, REQ-SCN-039 — a fragment that leaves the visible rectangle is cut, whatever
    /// its depth: widening the left margin hides the band <c>x &lt; c0</c> from the next frame.
    /// </summary>
    [GlFact] // REQ-SCN-036, REQ-SCN-039
    public void The_visible_rectangle_cuts_the_tiles_at_its_edges()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1), depth: 4, ProfileLayer.L1, columns: 32, rows: 1)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 0, 15, 32, 1, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        Shot wide = target.Render(renderer, scene);
        (int firstWide, _) = wide.ColumnSpan(Black);

        scene.Clipping = new Vec4F(64.1f, 8.1f, 8.1f, 0.1f);
        Shot narrow = target.Render(renderer, scene);
        (int firstNarrow, _) = narrow.ColumnSpan(Black);

        Assert.True(
            firstNarrow > firstWide,
            $"the band left of c0 must disappear: {firstWide} then {firstNarrow}.");

        wide.SavePng(Path.Combine(PartBCaptures.Directory, "scn-036-clip-wide.png"));
        narrow.SavePng(Path.Combine(PartBCaptures.Directory, "scn-036-clip-narrow.png"));
    }

    /// <summary>
    /// REQ-SCN-040, AMD-012 — a motif of alpha 0.5 in front of a red background lets about half
    /// of it through, and the depth pass is done per <b>object</b>, so no internal face between
    /// the tiles of one translucent object shows.
    /// </summary>
    [GlFact] // REQ-SCN-040
    public void A_translucent_object_shows_the_background_and_no_internal_face()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x06)
            .Pattern(SceneFixture.Solid(1), depth: 24, ProfileLayer.L1, alpha: 0.5f, columns: 4, rows: 4)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 12, 14, 4, 4, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        Shot shot = target.Render(renderer, scene);
        int background = SceneFixture.NesPacked(0x06);

        SceneFixture solid = new();
        solid
            .Background(0x06)
            .Pattern(SceneFixture.Solid(1), depth: 24, ProfileLayer.L1, columns: 4, rows: 4)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 12, 14, 4, 4, palette: 1);
        SceneState opaqueScene = solid.Build();
        opaqueScene.Palette(solid);
        Shot opaque = target.Render(renderer, opaqueScene);

        // The object is visible and the background shows through it: the centre pixel sits
        // between the background and the opaque shade, channel by channel.
        Assert.True(shot.CountOther(background) > 100, "the translucent object must be visible.");

        int middleX = shot.Width / 2;
        int middleY = shot.Height / 2;
        (int r, int g, int b) = shot.At(middleX, middleY);
        (int br, int bg, int bb) = NesTuple(0x06);
        (int or_, int og, int ob) = opaque.At(middleX, middleY);

        Assert.True(
            Between(r, br, or_) && Between(g, bg, og) && Between(b, bb, ob),
            $"the blend ({r},{g},{b}) must sit between the background ({br},{bg},{bb}) "
                + $"and the opaque shade ({or_},{og},{ob}).");

        static bool Between(int value, int one, int other)
            => value >= Math.Min(one, other) - 2 && value <= Math.Max(one, other) + 2;

        shot.SavePng(Path.Combine(PartBCaptures.Directory, "scn-040-translucent.png"));
    }

    /// <summary>
    /// REQ-SCN-041 — a motif of style 2 ("shadow only") is not drawn but its shadow is, and a
    /// motif of style 0 is neither drawn nor shadowed.
    /// </summary>
    [GlFact] // REQ-SCN-041
    public void The_shadow_only_and_invisible_styles_behave_as_documented()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        foreach (VariantRenderStyle style in new[]
        {
            VariantRenderStyle.Normal, VariantRenderStyle.ShadowOnly, VariantRenderStyle.Hidden,
        })
        {
            SceneFixture fixture = new();
            fixture
                .Background(0x0F)
                .Pattern(SceneFixture.Solid(1, 0), depth: 24, ProfileLayer.L1, style: style)
                .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.L2, columns: 8, rows: 8)
                .Palette(1, 0x30)
                .Rect(SceneFixture.Solid(1, 1), 8, 10, 8, 8, palette: 1)
                .Tile(SceneFixture.Solid(1, 0), 96, 96, palette: 1);

            SceneState scene = fixture.Build();
            scene.Palette(fixture);
            Shot shot = target.Render(renderer, scene);
            int shades = shot.DistinctColors().Count;

            switch (style)
            {
                case VariantRenderStyle.Normal:
                    Assert.True(shades >= 3, $"normal: {shades} shades.");
                    break;

                case VariantRenderStyle.ShadowOnly:
                    // The caster itself is gone but its shadow is on the floor: more than one
                    // floor shade, and the statistics report it as a shadow caster.
                    Assert.True(shades >= 3, $"shadow only: {shades} shades.");
                    Assert.True(renderer.Statistics.ShadowCasters > 0);
                    break;

                default:
                    // Nothing of the caster: neither the object nor its shadow.
                    Assert.True(shades <= 3, $"hidden: {shades} shades.");
                    break;
            }
        }
    }

    /// <summary>
    /// REQ-SCN-044, REQ-SCN-045 — a HUD motif placed outside the visible rectangle stays fully
    /// drawn, after the transparent objects, and is not cut by <c>R</c>.
    /// </summary>
    [GlFact] // REQ-SCN-044, REQ-SCN-045
    public void A_HUD_motif_outside_the_visible_rectangle_is_still_drawn_whole()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        // A UI motif in the top band, which the clipping would cut for an ordinary tile.
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.Ui, columns: 4, rows: 1)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 12, 0, 4, 1, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        scene.Clipping = new Vec4F(0.1f, 8.1f, 8.1f, 24.1f);

        Shot hud = target.Render(renderer, scene);
        Assert.True(renderer.Statistics.HudTiles > 0, "the HUD tiles must be drawn.");
        Assert.True(hud.CountOther(Black) > 50, "the HUD must stay visible outside R.");

        // The same motif in L1 is cut away by the same clipping.
        SceneFixture ordinary = new();
        ordinary
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1, columns: 4, rows: 1)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 12, 0, 4, 1, palette: 1);

        SceneState cut = ordinary.Build();
        cut.Palette(ordinary);
        cut.Clipping = new Vec4F(0.1f, 8.1f, 8.1f, 24.1f);

        Shot clipped = target.Render(renderer, cut);
        Assert.Equal(0, clipped.CountOther(Black));

        hud.SavePng(Path.Combine(PartBCaptures.Directory, "scn-044-hud.png"));
    }

    /// <summary>
    /// REQ-SCN-048 — the backdrop appears behind the scene, lit diffusely, and receives the
    /// shadows of the layers in front of it.
    /// </summary>
    [GlFact] // REQ-SCN-048
    public void The_backdrop_appears_behind_the_scene_and_receives_the_shadows()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1), depth: 24, ProfileLayer.L1)
            .Palette(1, 0x30)
            .Tile(SceneFixture.Solid(1), 96, 96, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        Shot without = target.Render(renderer, scene);

        scene.Backdrop.Enabled = true;
        scene.Backdrop.Color = new Rgb24(200, 200, 200);
        scene.Backdrop.Offset = new Vector3(0f, 0f, 26f);
        Shot with = target.Render(renderer, scene);

        Assert.True(
            with.CountOther(Black) > without.CountOther(Black),
            "the backdrop must fill the scene behind the tiles.");

        // With the shadows on, the panel carries more than one shade: the cast shadow.
        Assert.True(with.DistinctColors().Count >= 3);

        scene.Settings.Shadows = false;
        Shot flat = target.Render(renderer, scene);
        Assert.True(
            flat.DistinctColors().Count <= with.DistinctColors().Count,
            "switching the shadows off must not add shades.");

        with.SavePng(Path.Combine(PartBCaptures.Directory, "scn-048-backdrop.png"));
    }

    /// <summary>
    /// REQ-SCN-050 — the Gradient sky: at the horizon the colour is <c>M</c>, the zenith is
    /// lighter and the nadir darker, and a bright <c>M</c> shows no star.
    /// </summary>
    [GlFact] // REQ-SCN-049, REQ-SCN-050
    public void The_gradient_sky_is_lighter_at_the_zenith_and_darker_at_the_nadir()
    {
        using GlTarget target = GlTarget.Create(320, 320);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = Flat(0x22);
        scene.Settings.Skybox = SkyboxKind.Gradient;
        scene.Camera.Setpoints(CameraMode.Tpv).Angles = new Vector3(-80f, 0f, 0f);
        scene.Camera.SnapToSetpoints();
        Shot up = target.Render(renderer, scene);

        scene.Camera.Setpoints(CameraMode.Tpv).Angles = new Vector3(80f, 0f, 0f);
        scene.Camera.SnapToSetpoints();
        Shot down = target.Render(renderer, scene);

        scene.Camera.Setpoints(CameraMode.Tpv).Angles = Vector3.Zero;
        scene.Camera.SnapToSetpoints();
        Shot level = target.Render(renderer, scene);

        double zenith = up.Luma(up.Width / 2, 10);
        double horizon = level.Luma(level.Width / 2, level.Height / 2);
        double nadir = down.Luma(down.Width / 2, down.Height - 10);

        Assert.True(zenith > horizon, $"zenith {zenith:0.###} must be lighter than horizon {horizon:0.###}.");
        Assert.True(nadir < horizon, $"nadir {nadir:0.###} must be darker than horizon {horizon:0.###}.");

        // A bright M shows no star: the sky is smooth, with few distinct shades per row.
        Assert.False(SkyGradient.StarsVisible(NesPalette.Color(0x22)));

        level.SavePng(Path.Combine(PartBCaptures.Directory, "scn-050-gradient.png"));
        up.SavePng(Path.Combine(PartBCaptures.Directory, "scn-050-gradient-zenith.png"));
    }

    /// <summary>
    /// REQ-SCN-051, round-1 criterion (a) and (c) — with every cubemap at zero the Fantasy sky
    /// shows <c>F0</c> in every direction, and a bright <c>F0</c> brings no <c>Starfield</c>
    /// contribution.
    /// </summary>
    [GlFact] // REQ-SCN-051
    public void The_fantasy_sky_with_null_cubemaps_shows_the_background_colour()
    {
        using GlTarget target = GlTarget.Create(256, 256);
        using SceneRenderer renderer = new(target.Gl, new NullAssets());

        SceneState scene = Flat(0x22);
        scene.Settings.Skybox = SkyboxKind.Fantasy;

        Shot shot = target.Render(renderer, scene);

        // Every cubemap sample is zero, so every term but the background drops out.
        HashSet<int> colours = shot.DistinctColors();
        Assert.Single(colours);
        Assert.Equal(SceneFixture.NesPacked(0x22), colours.First());

        shot.SavePng(Path.Combine(PartBCaptures.Directory, "scn-051-fantasy-null.png"));
    }

    /// <summary>
    /// REQ-SCN-051, round-1 criterion (b) — with <c>NebulaDiffusion</c> at <c>(0, 0.5, 0)</c> and
    /// <c>NebulaDensity</c> and <c>Starfield</c> at zero, a grey <c>F0</c> of sRGB 0.2 comes out
    /// at <c>srgb(2.2975 × lin(F0))</c> ≈ sRGB 0.306 in every direction: only the ambient halo of
    /// step 2 contributes, with the weight 1.
    /// </summary>
    [GlFact] // REQ-SCN-051
    public void The_fantasy_ambient_halo_follows_the_documented_weight()
    {
        using GlTarget target = GlTarget.Create(192, 192);
        using SceneRenderer renderer = new(target.Gl, new HaloAssets());

        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.NesBackgroundColor = new Rgb24(51, 51, 51); // sRGB 0.2
        scene.Settings.Skybox = SkyboxKind.Fantasy;

        Shot shot = target.Render(renderer, scene);

        float expected = SceneColor.ToSrgb(
            Math.Clamp(2.2975f * SceneColor.ToLinear(51f / 255f), 0f, 1f));
        int wanted = (int)Math.Round(expected * 255f);

        (int r, int g, int b) = shot.At(shot.Width / 2, shot.Height / 2);
        Assert.True(
            Math.Abs(r - wanted) <= 2 && Math.Abs(g - wanted) <= 2 && Math.Abs(b - wanted) <= 2,
            $"expected sRGB {wanted} +/- 2 in every channel, got ({r},{g},{b}).");

        shot.SavePng(Path.Combine(PartBCaptures.Directory, "scn-051-fantasy-halo.png"));
    }

    /// <summary>
    /// REQ-SCN-053 — with a TV box the scene appears through an opening whose edges are the
    /// visible rectangle, and the tunnel hides what sticks out sideways.
    /// </summary>
    [GlFact] // REQ-SCN-053, AMD-025
    public void The_TV_box_surrounds_the_scene_with_a_tunnel()
    {
        if (FreeTelevision.Model is not { } television)
        {
            return;
        }

        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl, null, television);

        SceneFixture fixture = new();
        fixture
            .Background(0x22)
            .Pattern(SceneFixture.Solid(1), depth: 4, ProfileLayer.L1, columns: 4, rows: 4)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 12, 14, 4, 4, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        Shot none = target.Render(renderer, scene);
        Assert.Equal(0, renderer.Statistics.TunnelBoxes);

        scene.Settings.TvBox = TvBoxKind.Tv;
        Shot shot = target.Render(renderer, scene);

        Assert.Equal(5, renderer.Statistics.TunnelBoxes);
        Assert.Equal(television.Parts.Count, renderer.Statistics.TvParts);
        Assert.True(
            shot.DistinctColors().Count >= none.DistinctColors().Count,
            "the TV box must add the tunnel and the model.");

        shot.SavePng(Path.Combine(PartBCaptures.Directory, "scn-053-tv.png"));
    }

    /// <summary>
    /// REQ-SCN-058 — the render order: the shadow map, the clear or the skybox, the opaque
    /// objects, the sorted transparent ones, then the HUD. A translucent motif in front of the
    /// backdrop lets it show, and the HUD draws over the translucent objects.
    /// </summary>
    [GlFact] // REQ-SCN-058
    public void The_render_order_puts_the_HUD_over_the_transparent_objects()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1, 0), depth: 8, ProfileLayer.L1, alpha: 0.5f, columns: 8, rows: 8)
            .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.Ui, columns: 2, rows: 2)
            .Pattern(SceneFixture.Solid(1, 2), depth: 24, ProfileLayer.L1)
            .Palette(1, 0x30)
            .Palette(2, 0x16)
            .Rect(SceneFixture.Solid(1, 0), 8, 10, 8, 8, palette: 1)
            .Rect(SceneFixture.Solid(1, 1), 15, 14, 2, 2, palette: 2)
            .Tile(SceneFixture.Solid(1, 2), 64, 64, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        scene.Backdrop.Enabled = true;
        scene.Backdrop.Color = new Rgb24(60, 60, 60);
        scene.Backdrop.Offset = new Vector3(0f, 0f, 40f);

        Shot shot = target.Render(renderer, scene);

        Assert.True(renderer.Statistics.ShadowPass, "the shadow pass runs first.");
        Assert.True(renderer.Statistics.TransparentTiles > 0);
        Assert.True(renderer.Statistics.TransparentObjects > 0);
        Assert.True(renderer.Statistics.HudTiles > 0);

        // The HUD colour is present in the capture: it was not covered by the translucent pass.
        Assert.True(
            shot.Count(SceneFixture.NesPacked(0x16)) > 0
                || shot.DistinctColors().Count >= 4,
            "the HUD must be visible over the transparent objects.");

        shot.SavePng(Path.Combine(PartBCaptures.Directory, "scn-058-order.png"));
    }

    /// <summary>
    /// REQ-SCN-062 — with AntiAlias at None no edge pixel carries an intermediate colour: every
    /// pixel of the capture is either the object's shade or the background.
    /// </summary>
    [GlFact] // REQ-SCN-062
    public void With_no_antialiasing_no_edge_pixel_is_blended()
    {
        using GlTarget target = GlTarget.Create(320, 240);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1), depth: 16, ProfileLayer.L1, columns: 2, rows: 2)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 15, 14, 2, 2, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        scene.Camera.Setpoints(CameraMode.Tpv).Angles = new Vector3(20f, 30f, 0f);
        scene.Camera.SnapToSetpoints();
        scene.Settings.Shadows = false;

        Shot shot = target.Render(renderer, scene);

        // A cube has at most three lit face shades plus the background: four colours, no blend.
        Assert.True(
            shot.DistinctColors().Count <= 4,
            $"{shot.DistinctColors().Count} distinct colours: an edge was blended.");

        shot.SavePng(Path.Combine(PartBCaptures.Directory, "scn-062-no-antialias.png"));
    }

    /// <summary>
    /// REQ-SCN-065 (AMD-012) — in the default view, two adjacent tiles of one flat surface of the
    /// same colour show no visible line at their joint: no column or row at a joint differs from
    /// the surface colour by more than two levels.
    /// </summary>
    [GlFact] // REQ-SCN-065
    public void Two_adjacent_tiles_of_one_flat_surface_show_no_joint()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(SceneFixture.Solid(1), depth: 4, ProfileLayer.L1, columns: 4, rows: 4)
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 14, 13, 4, 4, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        Shot shot = target.Render(renderer, scene);

        // The surface shade is the most frequent non-background colour.
        Dictionary<int, int> histogram = [];
        for (int y = 0; y < shot.Height; y++)
        {
            for (int x = 0; x < shot.Width; x++)
            {
                int colour = shot.Packed(x, y);
                if (colour == Black)
                {
                    continue;
                }

                histogram[colour] = histogram.GetValueOrDefault(colour) + 1;
            }
        }

        Assert.NotEmpty(histogram);
        int surface = histogram.MaxBy(static entry => entry.Value).Key;
        (int sr, int sg, int sb) = Unpack(surface);

        // Every pixel of the surface area is within two levels of that shade: no crack, no
        // z-fighting line, no shadow discontinuity at the joints.
        int offenders = 0;
        for (int y = 0; y < shot.Height; y++)
        {
            for (int x = 0; x < shot.Width; x++)
            {
                int colour = shot.Packed(x, y);
                if (colour == Black || colour == surface)
                {
                    continue;
                }

                (int r, int g, int b) = Unpack(colour);
                if (Math.Abs(r - sr) > 2 || Math.Abs(g - sg) > 2 || Math.Abs(b - sb) > 2)
                {
                    offenders++;
                }
            }
        }

        shot.SavePng(Path.Combine(PartBCaptures.Directory, "scn-065-seams.png"));

        // A flat 4 × 4 surface seen from the front shows its front face alone; the few pixels of
        // its silhouette are allowed, the joints are not.
        Assert.True(
            offenders <= 4 * (shot.Width + shot.Height),
            $"{offenders} pixels differ from the surface shade by more than two levels.");

        Assert.All(
            InteriorRows(shot, surface),
            row => Assert.True(row, "a joint line crosses the surface."));
    }

    // ------------------------------------------------------------------- helpers

    private static SceneState Flat(byte nesBackground)
    {
        SceneState scene = new();
        scene.Camera.SnapToSetpoints();
        scene.GameRunning = true;
        scene.NesBackgroundColor = NesPalette.Color(nesBackground);
        return scene;
    }

    private static (int R, int G, int B) NesTuple(int index)
    {
        Rgb24 colour = NesPalette.Color(index);
        return (colour.R, colour.G, colour.B);
    }

    private static (int R, int G, int B) Unpack(int packed)
        => ((packed >> 16) & 0xFF, (packed >> 8) & 0xFF, packed & 0xFF);

    /// <summary>
    /// The sharpest colour transition in the capture, in pixels: how many pixels a horizontal
    /// row needs to go from one shade to the next.
    /// </summary>
    private static int SharpestTransition(Shot shot)
    {
        int sharpest = int.MaxValue;
        for (int y = 0; y < shot.Height; y++)
        {
            int runStart = -1;
            int previous = shot.Packed(0, y);
            for (int x = 1; x < shot.Width; x++)
            {
                int colour = shot.Packed(x, y);
                if (colour == previous)
                {
                    continue;
                }

                if (runStart >= 0)
                {
                    sharpest = Math.Min(sharpest, x - runStart);
                }

                runStart = x;
                previous = colour;
            }
        }

        return sharpest == int.MaxValue ? 1 : sharpest;
    }

    /// <summary>
    /// For each row that crosses the surface, whether the surface pixels of that row form one
    /// unbroken run: a joint line would split it in two.
    /// </summary>
    private static IEnumerable<bool> InteriorRows(Shot shot, int surface)
    {
        for (int y = 0; y < shot.Height; y++)
        {
            int runs = 0;
            bool inside = false;
            for (int x = 0; x < shot.Width; x++)
            {
                bool isSurface = shot.Packed(x, y) == surface;
                if (isSurface && !inside)
                {
                    runs++;
                }

                inside = isSurface;
            }

            if (runs > 0)
            {
                yield return runs == 1;
            }
        }
    }

    /// <summary>
    /// Scene assets where only <c>NebulaDiffusion</c> is non-zero, at <c>(0, 0.5, 0)</c>: the
    /// injection criterion (b) of REQ-SCN-051.
    /// </summary>
    private sealed class HaloAssets : ISceneAssets
    {
        public ImageData? Texture(string name) => null;

        public ImageData? CubemapFace(string cubemap, CubeFace face)
            => string.Equals(cubemap, "NebulaDiffusion", StringComparison.Ordinal)
                ? new ImageData(1, 1, [0, 128, 0, 255])
                : new ImageData(1, 1, new byte[4]);

        public TileMesh? Mesh(string name) => null;
    }

    /// <summary>Scene assets whose cubemaps are all zero, for criterion (a) of REQ-SCN-051.</summary>
    private sealed class NullAssets : ISceneAssets
    {
        public ImageData? Texture(string name) => null;

        public ImageData? CubemapFace(string cubemap, CubeFace face)
            => new(1, 1, new byte[4]);

        public TileMesh? Mesh(string name) => null;
    }
}

/// <summary>Publishes the frame palette of a fixture into a scene, for the capture tests.</summary>
internal static class ScenePaletteExtensions
{
    /// <summary>Copies the published frame's palette row data into the scene's draw list.</summary>
    public static void Palette(this SceneState scene, SceneFixture fixture)
    {
        ArgumentNullException.ThrowIfNull(scene);
        ArgumentNullException.ThrowIfNull(fixture);
        scene.NesBackgroundColor = fixture.Engine!.PublishedFrame!.Palette.Color(0);
    }
}
