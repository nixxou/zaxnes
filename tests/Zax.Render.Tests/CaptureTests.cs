using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// Offscreen captures of synthetic frames, checked by pixel statistics: default framing, shading
/// vectors, sprite nudge ordering, shadows per layer, clipping, HUD and background colour
/// (SPEC-06 §5, SPEC-01 REQ-ARCH-031).
/// </summary>
/// <remarks>
/// Every test renders through <see cref="GlFixture"/>, which hands the renderer a framebuffer it
/// did not create — the contract of REQ-SCN-057 and REQ-ARCH-003.
/// </remarks>
[Collection(GlCollection.Name)]
public sealed class CaptureTests
{
    private const int Blue22 = (104 << 16) | (136 << 8) | 252;
    private const int Black = 0;

    [GlFact]
    public void REQ_SCN_046_the_background_is_the_universal_nes_colour_of_the_frame()
    {
        SceneState scene = Flat(0x22, 0x30);
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);

        // The capture corners are outside the NES picture, so they show the clear colour.
        Assert.Equal(Blue22, capture.Packed(0, 0));
        Assert.Equal(Blue22, capture.Packed(capture.Width - 1, capture.Height - 1));
    }

    [GlFact]
    public void REQ_SCN_046_a_skybox_of_color_uses_the_custom_colour_and_a_stopped_game_is_black()
    {
        SceneState scene = Flat(0x22, 0x30);
        scene.Settings.Skybox = SkyboxKind.Color;
        scene.Settings.CustomBackgroundColor = new Rgb24(255, 0, 0);

        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture custom = gl.Render(renderer, scene);
        Assert.Equal((255 << 16) | 0, custom.Packed(0, 0));

        // REQ-SCN-047: outside a running game the 3D scene is not drawn at all.
        scene.Settings.Skybox = SkyboxKind.Default;
        scene.GameRunning = false;
        Capture stopped = gl.Render(renderer, scene);
        Assert.Equal(Black, stopped.Packed(0, 0));
        Assert.Single(stopped.DistinctColors());
    }

    [GlFact]
    public void REQ_SCN_012_the_default_view_frames_the_nes_picture_centred_and_about_87_percent_high()
    {
        SceneState scene = Flat(0x22, 0x30);
        using GlFixture gl = GlFixture.Create(640, 480);
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int firstRow, int lastRow) = capture.RowSpan(Blue22);
        (int firstColumn, int lastColumn) = capture.ColumnSpan(Blue22);

        double heightRatio = (lastRow - firstRow + 1) / (double)capture.Height;
        Assert.InRange(heightRatio, 0.85, 0.93);

        // 4:3 here, so the picture is wider on screen than the 52 % of the 16:9 reference view.
        double widthRatio = (lastColumn - firstColumn + 1) / (double)capture.Width;
        Assert.InRange(widthRatio, 0.65, 0.78);

        // Centred: the margins left and right, top and bottom, match within 2 % of the side.
        Assert.InRange(
            Math.Abs(firstColumn - (capture.Width - 1 - lastColumn)),
            0,
            (int)(capture.Width * 0.02));
        Assert.InRange(
            Math.Abs(firstRow - (capture.Height - 1 - lastRow)),
            0,
            (int)(capture.Height * 0.02));
    }

    [GlFact]
    public void Section_5_2_a_lit_white_front_face_is_sRGB_223()
    {
        SceneState scene = Flat(0x0F, 0x30);
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);

        // NES 0x30 is (252, 248, 252); lit by (1, -1, 1) at N.L = 0.577 it is (223, 219, 223).
        Assert.InRange(r, 220, 226);
        Assert.InRange(g, 216, 222);
        Assert.InRange(b, 220, 226);
    }

    [GlFact]
    public void Section_5_2_a_lit_blue_front_face_is_sRGB_91_120_223()
    {
        SceneState scene = Flat(0x0F, 0x22);
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);

        Assert.InRange(r, 88, 94);
        Assert.InRange(g, 117, 123);
        Assert.InRange(b, 220, 226);
    }

    [GlFact]
    public void REQ_SCN_026_the_intensity_setting_brightens_and_darkens_the_direct_light()
    {
        SceneState scene = Flat(0x0F, 0x30);
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        scene.Settings.LightIntensity = 1f;
        double normal = gl.Render(renderer, scene).Luma(gl.Width / 2, gl.Height / 2);

        scene.Settings.LightIntensity = 0.5f;
        double dim = gl.Render(renderer, scene).Luma(gl.Width / 2, gl.Height / 2);

        scene.Settings.LightIntensity = 1.5f;
        double bright = gl.Render(renderer, scene).Luma(gl.Width / 2, gl.Height / 2);

        Assert.True(dim < normal);
        Assert.True(bright > normal);
    }

    [GlFact]
    public void REQ_SCN_037_a_flat_sprite_stays_in_front_of_a_background_tile_of_the_same_depth()
    {
        SceneState scene = NudgeScene(behindBackground: false);
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture first = gl.Render(renderer, scene);
        Capture second = gl.Render(renderer, scene);

        int centreX = first.Width / 2;
        int centreY = first.Height / 2;
        (int r, int g, int b) = first.At(centreX, centreY);

        // The sprite is 0x16 (red, lit), the background tile 0x21 (blue, lit): red wins.
        Assert.True(r > g && r > b, $"expected the sprite's red in front, got ({r}, {g}, {b})");

        // No flicker: the same pixel comes out the same on two consecutive frames.
        Assert.Equal(first.Packed(centreX, centreY), second.Packed(centreX, centreY));
    }

    [GlFact]
    public void REQ_SCN_037_a_behind_background_sprite_is_pushed_behind_instead()
    {
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture front = gl.Render(renderer, NudgeScene(behindBackground: false));
        Capture behind = gl.Render(renderer, NudgeScene(behindBackground: true));

        int centreX = front.Width / 2;
        int centreY = front.Height / 2;
        Assert.NotEqual(front.Packed(centreX, centreY), behind.Packed(centreX, centreY));

        (int r, int g, int b) = behind.At(centreX, centreY);
        Assert.True(b > r, $"expected the background tile's blue in front, got ({r}, {g}, {b})");
    }

    [GlFact]
    public void REQ_SCN_030_the_cast_shadow_is_present_and_its_edge_is_sharp()
    {
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture lit = gl.Render(renderer, ShadowScene(shadows: false, layerCasts: true));
        Capture shadowed = gl.Render(renderer, ShadowScene(shadows: true, layerCasts: true));

        // The floor is NES 0x30 seen from the front: (223, 219, 223) lit, (146, 144, 146) in
        // shadow (SPEC-06 §5.2).
        int shadowPixels = CountNear(shadowed, ShadowedFloor);
        Assert.True(shadowPixels > 200, $"expected a cast shadow, found {shadowPixels} shadowed pixels");
        Assert.Equal(0, CountNear(lit, ShadowedFloor));

        // A hard edge: the lit floor turns into the shadowed floor within two pixels.
        Assert.InRange(EdgeWidth(shadowed), 0, 2);
    }

    [GlFact]
    public void REQ_SCN_031_a_slanted_lit_surface_shows_no_shadow_acne()
    {
        // The biases of REQ-SCN-031 are there "pour éviter l'acné d'ombre". Even with the
        // 4096-texel map of AMD-034 §2, one texel over a whole scene spans far more than
        // 0.02 unit, so a front face would shadow itself in a stipple pattern; only the back
        // faces fill the map.
        using GlFixture gl = GlFixture.Create(240, 240);
        using SceneRenderer renderer = new(gl.Gl);

        // One big slab of 3-voxel-deep tiles, every one of them a caster and a receiver, seen
        // from an angle so that its top face is lit and visible.
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 3, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), 4, 6, 24, 18);

        SceneState scene = fixture.Build();
        scene.Camera.Current.Angles = new Vector3(25f, -20f, 0f);
        scene.Camera.SnapToSetpoints();

        Capture lit = gl.Render(renderer, scene);
        Assert.True(renderer.Statistics.ShadowPass);

        scene.Settings.Shadows = false;
        Capture flat = gl.Render(renderer, scene);

        // Self-shadowing would double the number of distinct greys of the slab; without acne
        // the two captures hold the same handful of colours.
        Assert.True(
            lit.DistinctColors().Count <= flat.DistinctColors().Count + 2,
            $"shadow acne: {lit.DistinctColors().Count} colours with shadows, "
                + $"{flat.DistinctColors().Count} without");
    }

    [GlFact]
    public void REQ_SCN_030_turning_the_shadows_off_removes_every_shadow()
    {
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture off = gl.Render(renderer, ShadowScene(shadows: false, layerCasts: true));
        Capture on = gl.Render(renderer, ShadowScene(shadows: true, layerCasts: true));

        Assert.True(off.DistinctColors().Count < on.DistinctColors().Count);
    }

    [GlFact]
    public void REQ_SCN_032_clearing_the_casting_flag_of_a_layer_removes_its_shadows()
    {
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture casting = gl.Render(renderer, ShadowScene(shadows: true, layerCasts: true));
        Capture notCasting = gl.Render(renderer, ShadowScene(shadows: true, layerCasts: false));

        Assert.True(CountNear(casting, ShadowedFloor) > 200);
        Assert.Equal(0, CountNear(notCasting, ShadowedFloor));
    }

    [GlFact]
    public void REQ_SCN_036_a_fragment_outside_the_visible_rectangle_is_dropped()
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 8, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), column: 0, row: 8, columns: 6, rows: 14);

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        scene.Clipping = ConversionSettings.DefaultClipping;
        Capture wide = gl.Render(renderer, scene);
        (int wideFirst, _) = wide.ColumnSpan(Black);

        // REQ-SCN-039: the rectangle is recomputed every rendered frame from the margins.
        scene.Clipping = new Vec4F(24.1f, 8.1f, 8.1f, 0.1f);
        Capture narrow = gl.Render(renderer, scene);
        (int narrowFirst, _) = narrow.ColumnSpan(Black);

        Assert.True(
            narrowFirst > wideFirst + 20,
            $"the left edge of R did not move: {wideFirst} then {narrowFirst}");
        Assert.True(narrow.CountOther(Black) < wide.CountOther(Black));
    }

    [GlFact]
    public void REQ_SCN_044_a_hud_pattern_outside_the_visible_rectangle_stays_whole()
    {
        // Screen row 0 is scene y in [240, 248], above the top edge of R (REQ-E3D-410).
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 2, ProfileLayer.Ui)
            .Rect(SceneFixture.Solid(1), column: 4, row: 0, columns: 8, rows: 1);

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        int drawn = capture.CountOther(Black);

        Assert.True(drawn > 500, $"the HUD strip was clipped away, only {drawn} pixels drawn");
    }

    [GlFact]
    public void REQ_SCN_040_a_transparent_pattern_lets_the_wall_behind_it_show_through()
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x16)
            .Palette(2, 0x30)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L2)
            .Pattern(SceneFixture.Solid(2), depth: 6, ProfileLayer.L1, alpha: 0.5f)
            .Rect(SceneFixture.Solid(1), column: 8, row: 10, columns: 16, rows: 10)
            .Rect(SceneFixture.Solid(2), column: 13, row: 13, columns: 6, rows: 4);

        SceneState scene = fixture.Build();
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);

        // Half white over a lit red wall: still clearly reddish, but much lighter than the wall.
        (int wallR, int wallG, int wallB) = capture.At(capture.Width / 2, (capture.Height / 2) + 90);
        Assert.True(r > wallR - 40 && g > wallG + 20 && b > wallB + 20,
            $"blend looks wrong: pattern ({r}, {g}, {b}) over wall ({wallR}, {wallG}, {wallB})");
    }

    [GlFact]
    public void REQ_SCN_041_a_shadow_only_pattern_is_invisible_but_still_casts()
    {
        SceneState normal = ShadowScene(shadows: true, layerCasts: true);
        SceneState shadowOnly = ShadowScene(
            shadows: true,
            layerCasts: true,
            blockStyle: VariantRenderStyle.ShadowOnly);

        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture visible = gl.Render(renderer, normal);
        Capture invisible = gl.Render(renderer, shadowOnly);

        // The red block is gone...
        Assert.True(
            CountReddish(invisible) < CountReddish(visible) / 10,
            $"the shadow-only block is still drawn: {CountReddish(invisible)} red pixels");

        // ... but its shadow is still on the floor.
        Assert.True(CountNear(invisible, ShadowedFloor) > 200);
    }

    [GlFact]
    public void REQ_SCN_057_two_renderers_can_share_a_context_without_interfering()
    {
        SceneState scene = Flat(0x22, 0x30);
        using GlFixture gl = GlFixture.Create();
        using SceneRenderer first = new(gl.Gl);
        using SceneRenderer second = new(gl.Gl);

        Capture one = gl.Render(first, scene);
        Capture two = gl.Render(second, scene);

        Assert.Equal(one.Pixels, two.Pixels);
    }

    // ------------------------------------------------------------------ scenes

    private static SceneState Flat(byte background, byte tileColor)
        => new SceneFixture()
            .Background(background)
            .Palette(1, tileColor)
            .Rect(SceneFixture.Solid(1), column: 0, row: 1, columns: 32, rows: 29)
            .Build();

    private static SceneState NudgeScene(bool behindBackground)
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x21)
            .Palette(18, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(2), depth: 1, ProfileLayer.L1)
            .Rect(SceneFixture.Solid(1), column: 12, row: 12, columns: 8, rows: 6);

        for (int row = 13; row < 17; row++)
        {
            for (int column = 13; column < 19; column++)
            {
                fixture.Sprite(SceneFixture.Solid(2), column * 8, row * 8, palette: 0, behindBackground);
            }
        }

        return fixture.Build();
    }

    private static SceneState ShadowScene(
        bool shadows,
        bool layerCasts,
        VariantRenderStyle blockStyle = VariantRenderStyle.Normal)
    {
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(2, 0x16)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L2)
            .Pattern(SceneFixture.Solid(2), depth: 8, ProfileLayer.L1, style: blockStyle)
            .Rect(SceneFixture.Solid(1), column: 8, row: 10, columns: 16, rows: 10)
            .Rect(SceneFixture.Solid(2), column: 14, row: 13, columns: 2, rows: 2);

        SceneState scene = fixture.Build(settings =>
        {
            settings.Shadow = shadows;
            if (!layerCasts)
            {
                settings.Layer(ZLayer.L1).CastShadow = false;
            }
        });

        scene.Settings.Shadows = shadows;
        return scene;
    }

    // ------------------------------------------------------------------ AMD-021 R2-009/R2-010

    [GlFact] // AMD-021 R2-009, first half of the criterion
    public void AMD_021_R2_009_a_half_opaque_motif_on_black_is_sRGB_158_124_145()
    {
        // NES 0x35 is (244, 192, 224); its lit front face is linear (0.9047, 0.5273, 0.7455)
        // x (0.1812 + 0.577) = (0.6859, 0.3998, 0.5653). Half of that over a black backdrop is
        // (0.3430, 0.1999, 0.2826) in linear, that is sRGB (158, 124, 145). Blended in sRGB
        // instead it would come out (108, 85, 99).
        SceneState scene = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x35)
            .Pattern(SceneFixture.Solid(1), depth: 2, ProfileLayer.L1, alpha: 0.5f)
            .Rect(SceneFixture.Solid(1), column: 8, row: 8, columns: 16, rows: 14)
            .Build();

        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);
        (int r, int g, int b) = capture.At(capture.Width / 2, capture.Height / 2);

        Assert.InRange(r, 156, 160);
        Assert.InRange(g, 122, 126);
        Assert.InRange(b, 143, 147);
    }

    [GlFact] // AMD-021 R2-009, second half of the criterion
    public void AMD_021_R2_009_the_same_motif_over_sRGB_0_120_131_gives_g_147()
    {
        // The wall is NES 0x1C (0, 136, 148), whose lit front face is sRGB (0, 120, 131) —
        // the background colour the criterion names. Blended in linear, the green channel
        // comes out at 147; blended in sRGB it would be 134.
        //
        // The motif in front is laid down as sprite records: two background records on the
        // same cell would replace one another and leave a hole in the wall, which is exactly
        // what this measurement must not have.
        SceneFixture fixture = new SceneFixture()
            .Background(0x0F)
            .Palette(1, 0x1C)
            .Palette(18, 0x35)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L2)
            .Pattern(SceneFixture.Solid(2), depth: 6, ProfileLayer.L1, alpha: 0.5f)
            .Rect(SceneFixture.Solid(1), column: 6, row: 6, columns: 20, rows: 18);

        for (int row = 12; row < 18; row++)
        {
            for (int column = 12; column < 20; column++)
            {
                fixture.Sprite(SceneFixture.Solid(2), column * 8, row * 8);
            }
        }

        SceneState scene = fixture.Build();

        using GlFixture gl = GlFixture.Create();
        using SceneRenderer renderer = new(gl.Gl);

        Capture capture = gl.Render(renderer, scene);

        // The wall on its own, below the transparent motif: the named background colour.
        (int wallR, int wallG, int wallB) = capture.At(capture.Width / 2, (capture.Height / 2) + 110);
        Assert.InRange(wallR, 0, 2);
        Assert.InRange(wallG, 118, 122);
        Assert.InRange(wallB, 129, 133);

        (int _, int g, int _) = capture.At(capture.Width / 2, capture.Height / 2);
        Assert.InRange(g, 145, 149);
    }

    [GlFact] // AMD-021 R2-010
    public void AMD_021_R2_010_a_transparent_receiver_takes_no_shadow()
    {
        // The synthetic shadow scene of REQ-SCN-030 with the receiving floor at alpha 0.5: the
        // shadow of the block must not appear on it at all, so the whole floor is one colour.
        Capture Render(float alpha)
        {
            SceneState scene = new SceneFixture()
                .Background(0x0F)
                .Palette(1, 0x30)
                .Palette(2, 0x16)
                .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L2, alpha: alpha)
                .Pattern(SceneFixture.Solid(2), depth: 8, ProfileLayer.L1)
                .Rect(SceneFixture.Solid(1), column: 8, row: 10, columns: 16, rows: 10)
                .Rect(SceneFixture.Solid(2), column: 14, row: 13, columns: 2, rows: 2)
                .Build(settings => settings.Shadow = true);

            scene.Settings.Shadows = true;
            using GlFixture gl = GlFixture.Create();
            using SceneRenderer renderer = new(gl.Gl);
            return gl.Render(renderer, scene);
        }

        Capture opaque = Render(1f);
        Capture transparent = Render(0.5f);

        // The opaque floor does carry the shadow: that is REQ-SCN-030.
        Assert.True(CountNear(opaque, ShadowedFloor) > 100, "the opaque floor shows no shadow");

        // The transparent one does not: no pixel of the floor is darker than the unshadowed
        // colour of the same tile, which over a black backdrop is half of the lit floor in
        // linear space.
        (int litR, int litG, int litB) = transparent.At(
            transparent.Width / 2,
            (transparent.Height / 2) + 100);

        int darker = 0;
        for (int y = 0; y < transparent.Height; y++)
        {
            for (int x = 0; x < transparent.Width; x++)
            {
                (int r, int g, int b) = transparent.At(x, y);
                if (r == 0 && g == 0 && b == 0)
                {
                    continue; // The backdrop.
                }

                if (r < litR - 6 && g < litG - 6 && b < litB - 6 && r > 10)
                {
                    darker++;
                }
            }
        }

        Assert.Equal(0, darker);
        Assert.Equal(0, CountNear(transparent, ShadowedFloor));
    }

    // ------------------------------------------------------------------ measures

    /// <summary>The lit front face of the NES 0x30 floor: (223, 219, 223) (SPEC-06 §5.2).</summary>
    private static (int R, int G, int B) LitFloor { get; } = (223, 219, 223);

    /// <summary>The same face in shadow: (146, 144, 146) (SPEC-06 §5.2, REQ-SCN-031).</summary>
    private static (int R, int G, int B) ShadowedFloor { get; } = (146, 144, 146);

    /// <summary>Pixels within <paramref name="tolerance"/> of <paramref name="color"/>.</summary>
    private static int CountNear(Capture capture, (int R, int G, int B) color, int tolerance = 4)
    {
        int found = 0;
        for (int y = 0; y < capture.Height; y++)
        {
            for (int x = 0; x < capture.Width; x++)
            {
                if (Near(capture.At(x, y), color, tolerance))
                {
                    found++;
                }
            }
        }

        return found;
    }

    private static bool Near((int R, int G, int B) value, (int R, int G, int B) target, int tolerance)
        => Math.Abs(value.R - target.R) <= tolerance
        && Math.Abs(value.G - target.G) <= tolerance
        && Math.Abs(value.B - target.B) <= tolerance;

    private static int CountReddish(Capture capture)
    {
        int found = 0;
        for (int y = 0; y < capture.Height; y++)
        {
            for (int x = 0; x < capture.Width; x++)
            {
                (int r, int g, int b) = capture.At(x, y);
                if (r > 80 && r > g * 2 && r > b * 2)
                {
                    found++;
                }
            }
        }

        return found;
    }

    /// <summary>
    /// The widest run of pixels that are neither the lit floor nor the shadowed floor, between a
    /// lit pixel and a shadowed one on the same row: the width of the shadow edge
    /// (REQ-SCN-030 asks for at most 2 pixels on a 640 × 480 capture).
    /// </summary>
    private static int EdgeWidth(Capture capture)
    {
        int worst = 0;

        for (int y = 0; y < capture.Height; y++)
        {
            int run = -1;
            for (int x = 0; x < capture.Width; x++)
            {
                (int R, int G, int B) pixel = capture.At(x, y);
                bool isLit = Near(pixel, LitFloor, 4);
                bool isShadow = Near(pixel, ShadowedFloor, 4);

                if (isLit)
                {
                    run = 0;
                }
                else if (run >= 0 && isShadow)
                {
                    worst = Math.Max(worst, run);
                    run = 0;
                }
                else if (run >= 0)
                {
                    run++;
                    if (run > 8)
                    {
                        // Something else is in the way (the block itself); stop this run.
                        run = -1;
                    }
                }
            }
        }

        return worst;
    }
}
