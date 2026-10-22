using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-06 §3.9 to §3.17 that need no GL context: the light, the
/// ambient term, the shading model, the shadow rules, the materials, the background colour, the
/// backdrop, the skybox formulas, the TV box geometry and the pipeline settings.
/// </summary>
public sealed class SceneRenderTests
{
    private static readonly Vec4F DefaultClipping = new(0.1f, 8.1f, 8.1f, 0.1f);

    // ------------------------------------------------------------------ §3.9 light

    /// <summary>
    /// REQ-SCN-025 — one directional light; <c>Light.Direction</c> is the propagation direction,
    /// <c>(1, −1, 1)</c> by default, the colour white and the intensity 1. With that direction
    /// the faces turned towards −Z, +Y and −X all get <c>N·L = 0.577</c> and the three others
    /// get only the ambient term.
    /// </summary>
    [Fact] // REQ-SCN-025
    public void The_directional_light_has_the_documented_default_and_geometry()
    {
        LightState light = new();
        Assert.Equal(new Vector3(1f, -1f, 1f), light.Direction);
        Assert.Equal(new Rgb24(255, 255, 255), light.Color);

        RenderSettings settings = new();
        Assert.Equal(1f, settings.LightIntensity);

        Vector3 l = -Vector3.Normalize(light.Direction);
        foreach (Vector3 normal in new[] { -Vector3.UnitZ, Vector3.UnitY, -Vector3.UnitX })
        {
            Assert.Equal(0.577, Vector3.Dot(normal, l), 0.001);
        }

        foreach (Vector3 normal in new[] { Vector3.UnitX, -Vector3.UnitY, Vector3.UnitZ })
        {
            Assert.True(Vector3.Dot(normal, l) < 0f, $"{normal} must get the ambient term only.");
        }
    }

    /// <summary>
    /// REQ-SCN-026 — the direction and the colour are profile settings reloaded at the start of
    /// each game; the intensity is a user setting, kept from one profile to the next and bounded
    /// to <c>[0.50, 1.50]</c>.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-026
    public void The_light_direction_is_a_profile_setting_and_the_intensity_a_user_one()
    {
        RenderSettings bounds = new();
        bounds.LightIntensity = 1.2f;
        Assert.Equal(1.2f, bounds.LightIntensity, 1e-4f);
        bounds.LightIntensity = 2f;
        Assert.Equal(1.5f, bounds.LightIntensity, 1e-4f);
        bounds.LightIntensity = 0f;
        Assert.Equal(0.5f, bounds.LightIntensity, 1e-4f);

        // A profile that carries both a direction and an intensity: the direction is applied,
        // the intensity ignored, and the user's value is kept from one profile to the next.
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verifier-light";
        profile.Settings!.Light = new LightSettings
        {
            Direction = new Vec3F(1f, -1f, 2f),
            Intensity = 0.6f,
        };

        using HostDriver host = HostDriver.Create(profile);
        host.Settings.LightIntensity = 1.2f;
        host.ApplyCommands();

        host.Launch(host.EntryIdByTitle("verifier-light"), host.WriteRom(LoopRom.Counter()));

        Assert.Equal(new Vector3(1f, -1f, 2f), host.Scene.Light.Direction);
        Assert.Equal(1.2f, host.Settings.LightIntensity, 1e-4f);
        Assert.Equal(1.2f, host.Scene.Settings.LightIntensity, 1e-4f);
    }

    /// <summary>
    /// REQ-SCN-026, criterion — 45 of the 101 built-in profiles carry the light direction
    /// <c>(1, −1, 1)</c> and 14 carry <c>(1, −1, 2)</c>.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-SCN-026
    public void The_built_in_profiles_carry_the_documented_light_directions()
    {
        int ones = 0;
        int twos = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            GameProfile profile = ProfileCorpus.Load(path);
            if (profile.Settings?.Light?.Direction is not { } direction)
            {
                continue;
            }

            if (Near(direction, 1f, -1f, 1f))
            {
                ones++;
            }
            else if (Near(direction, 1f, -1f, 2f))
            {
                twos++;
            }
        }

        Assert.Equal(45, ones);
        Assert.Equal(14, twos);

        static bool Near(Vec3F value, float x, float y, float z)
            => MathF.Abs(value.X - x) < 1e-4f
            && MathF.Abs(value.Y - y) < 1e-4f
            && MathF.Abs(value.Z - z) < 1e-4f;
    }

    /// <summary>
    /// REQ-SCN-027 — the ambient light is uniform, sRGB (118, 118, 118), 0.1812 in linear space;
    /// a white face (NES 0x30) lit by the ambient term alone shows sRGB (117, 115, 117) ± 2.
    /// </summary>
    [Fact] // REQ-SCN-027
    public void The_ambient_term_is_a_uniform_sRGB_118()
    {
        Assert.Equal(new Rgb24(118, 118, 118), SceneColor.Ambient);
        Assert.Equal(0.1812f, SceneColor.AmbientLinear, 1e-4f);

        Rgb24 white = NesPalette.Color(0x30);
        Rgb24 ambientOnly = SceneColor.Shade(
            white, -Vector3.UnitX, new Vector3(-1f, 0f, 0f), new Rgb24(255, 255, 255), 1f, 1f);

        // A face turned away from the light: only the ambient term.
        Near(117, ambientOnly.R, 2);
        Near(115, ambientOnly.G, 2);
        Near(117, ambientOnly.B, 2);
    }

    /// <summary>
    /// REQ-SCN-028, REQ-SCN-031, REQ-SCN-035, §5.2 — the shading table: a lit face
    /// (<c>N·L = 0.577</c>), the same face in shadow and a face with the ambient term alone, for
    /// five colours, computed in linear space and re-encoded to sRGB. Tolerance ±3 per channel.
    /// </summary>
    [Theory] // REQ-SCN-028, REQ-SCN-031, REQ-SCN-035
    [InlineData(0x30, 252, 248, 252, 223, 219, 223, 146, 144, 146, 117, 115, 117)]
    [InlineData(0x22, 104, 136, 252, 91, 120, 223, 57, 76, 146, 44, 60, 117)]
    [InlineData(0x17, 228, 96, 24, 202, 84, 20, 132, 52, 9, 105, 40, 5)]
    [InlineData(0x29, 188, 248, 24, 166, 219, 20, 108, 144, 9, 85, 115, 5)]
    public void The_shading_vectors_of_section_5_2(
        int nes,
        int inR, int inG, int inB,
        int litR, int litG, int litB,
        int shadowR, int shadowG, int shadowB,
        int ambientR, int ambientG, int ambientB)
    {
        Rgb24 albedo = NesPalette.Color(nes);
        Assert.Equal(inR, albedo.R);
        Assert.Equal(inG, albedo.G);
        Assert.Equal(inB, albedo.B);

        Rgb24 white = new(255, 255, 255);
        Vector3 direction = new(1f, -1f, 1f);

        Rgb24 lit = SceneColor.Shade(albedo, -Vector3.UnitZ, direction, white, 1f, 1f);
        Rgb24 shadowed = SceneColor.Shade(
            albedo, -Vector3.UnitZ, direction, white, 1f, SceneColor.ShadowFactor);
        Rgb24 ambient = SceneColor.Shade(albedo, Vector3.UnitZ, direction, white, 1f, 1f);

        Near(litR, lit.R, 3);
        Near(litG, lit.G, 3);
        Near(litB, lit.B, 3);
        Near(shadowR, shadowed.R, 3);
        Near(shadowG, shadowed.G, 3);
        Near(shadowB, shadowed.B, 3);
        Near(ambientR, ambient.R, 3);
        Near(ambientG, ambient.G, 3);
        Near(ambientB, ambient.B, 3);
    }

    /// <summary>§5.2, last row — pure white: 226 lit, 148 in shadow, 118 with the ambient only.</summary>
    [Fact] // REQ-SCN-028, REQ-SCN-031
    public void Pure_white_follows_the_last_row_of_section_5_2()
    {
        Rgb24 white = new(255, 255, 255);
        Vector3 direction = new(1f, -1f, 1f);

        Near(226, SceneColor.Shade(white, -Vector3.UnitZ, direction, white, 1f, 1f).R, 3);
        Near(148, SceneColor.Shade(white, -Vector3.UnitZ, direction, white, 1f, SceneColor.ShadowFactor).R, 3);
        Near(118, SceneColor.Shade(white, Vector3.UnitZ, direction, white, 1f, 1f).R, 3);
    }

    /// <summary>
    /// REQ-SCN-028 — the shading has no specular and no emissive term and is not tone mapped:
    /// the result saturates at 1. The intensity goes through the same sRGB → linear curve as the
    /// colours (DEC-06-Q2).
    /// </summary>
    [Fact] // REQ-SCN-028
    public void The_shading_saturates_and_the_intensity_uses_the_same_curve()
    {
        Rgb24 white = new(255, 255, 255);
        Vector3 direction = new(0f, 0f, 1f);

        // A face straight at the light with a bright intensity saturates at 255, never above.
        Rgb24 saturated = SceneColor.Shade(white, -Vector3.UnitZ, direction, white, 1.5f, 1f);
        Assert.Equal(255, saturated.R);

        // The intensity curve: 0.5 of intensity becomes lin(0.5) of linear light.
        Rgb24 half = SceneColor.Shade(white, -Vector3.UnitZ, direction, white, 0.5f, 1f);
        float expected = SceneColor.AmbientLinear + SceneColor.ToLinear(0.5f);
        Near(
            (int)Math.Round(SceneColor.ToSrgb(Math.Clamp(expected, 0f, 1f)) * 255f),
            half.R,
            2);
    }

    /// <summary>
    /// REQ-SCN-029 — the HUD light direction is read in the camera frame, so orbiting does not
    /// change the HUD's shading.
    /// </summary>
    [Fact] // REQ-SCN-029
    public void The_HUD_light_is_expressed_in_the_camera_frame()
    {
        SceneCamera camera = new();
        camera.SnapToSetpoints();
        Vector3 direction = new(1f, -1f, 1f);

        CameraView front = camera.BuildView(DefaultClipping, 16f / 9f);
        Vector3 before = front.HudLightDirection(direction);

        camera.Current.Angles = new Vector3(20f, 90f, 0f);
        camera.SnapToSetpoints();
        CameraView orbited = camera.BuildView(DefaultClipping, 16f / 9f);
        Vector3 after = orbited.HudLightDirection(direction);

        // The HUD plane turns with the camera, so the light turns with it: the angle between the
        // light and the camera basis is unchanged.
        Assert.Equal(
            Vector3.Dot(Vector3.Normalize(before), Vector3.Normalize(front.Forward)),
            Vector3.Dot(Vector3.Normalize(after), Vector3.Normalize(orbited.Forward)),
            1e-4);
        Assert.Equal(before.Length(), after.Length(), 1e-3);
    }

    // ----------------------------------------------------------------- §3.10 shadows

    /// <summary>
    /// REQ-SCN-030 — the shadow map is a single map of 1024 texels by default covering the part
    /// of the scene within 512 units of the camera (DEC-R2-D4, Q-019).
    /// </summary>
    [Fact] // REQ-SCN-030
    public void The_shadow_map_is_one_1024_texel_map_over_512_units()
    {
        Assert.Equal(1024, SceneConstants.ShadowMapSize);
        Assert.Equal(512f, SceneConstants.ShadowDistance);
        Assert.Equal(1024, new RenderSettings().ShadowMapSize);
    }

    /// <summary>
    /// REQ-SCN-031, Q-032 — the direct term is multiplied by 0.2 inside a shadow, the ambient is
    /// untouched, and the depth bias is <c>max(0.02 ; 1.5 × texel)</c>.
    /// </summary>
    [Fact] // REQ-SCN-031
    public void The_shadow_strength_is_zero_point_eight_and_the_bias_follows_the_texel()
    {
        Assert.Equal(0.2f, SceneColor.ShadowFactor);
        Assert.Equal(0.02f, SceneConstants.ShadowDepthBias);
        Assert.Equal(0.02f, SceneConstants.ShadowNormalOffset);
        Assert.Equal(1.5f, SceneConstants.ShadowBiasTexels);

        // A white frontal face goes from 223 in full light to 146 in shadow (±3).
        Rgb24 white = NesPalette.Color(0x30);
        Vector3 direction = new(1f, -1f, 1f);
        Near(223, SceneColor.Shade(white, -Vector3.UnitZ, direction, new Rgb24(255, 255, 255), 1f, 1f).R, 3);
        Near(
            146,
            SceneColor.Shade(white, -Vector3.UnitZ, direction, new Rgb24(255, 255, 255), 1f, 0.2f).R,
            3);
    }

    /// <summary>
    /// REQ-SCN-032 — a tile casts a shadow if and only if the shadows are on, it is not HUD, it
    /// is rendered opaque, and the cast flags of its motif and of its Z layer are both true.
    /// The "shadow only" style always casts. A transparent tile never casts.
    /// </summary>
    [Fact] // REQ-SCN-032, REQ-SCN-041
    public void The_cast_and_receive_rules_follow_the_motif_and_the_layer()
    {
        // L1 casts and receives; L2 receives without casting (REQ-SCN-033).
        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 8, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 8, ProfileLayer.L2)
            .Pattern(SceneFixture.Solid(1, 2), depth: 8, ProfileLayer.Ui)
            .Pattern(SceneFixture.Solid(1, 3), depth: 8, ProfileLayer.L1, alpha: 0.5f)
            .Pattern(
                SceneFixture.Solid(1, 4), depth: 8, ProfileLayer.L1,
                style: VariantRenderStyle.ShadowOnly)
            .Pattern(SceneFixture.Solid(1, 5), depth: 8, ProfileLayer.L1, tags: "!cast")
            .Tile(SceneFixture.Solid(1, 0), 0, 96)
            .Tile(SceneFixture.Solid(1, 1), 16, 96)
            .Tile(SceneFixture.Solid(1, 2), 32, 96)
            .Tile(SceneFixture.Solid(1, 3), 48, 96)
            .Tile(SceneFixture.Solid(1, 4), 64, 96)
            .Tile(SceneFixture.Solid(1, 5), 80, 96);

        SceneState scene = fixture.Build();
        Dictionary<int, DrawTile> byColumn = scene.DrawList!.Tiles
            .ToDictionary(static tile => tile.Tile.Position.X);

        Assert.True(At(0).CastShadow, "an L1 motif casts.");
        Assert.True(At(0).ReceiveShadow, "an L1 motif receives.");
        Assert.False(At(16).CastShadow, "an L2 motif does not cast.");
        Assert.True(At(16).ReceiveShadow, "an L2 motif receives.");
        Assert.False(At(32).CastShadow, "a UI motif never casts.");
        Assert.False(At(32).ReceiveShadow, "a UI motif never receives.");
        Assert.False(At(48).CastShadow, "a transparent motif does not cast.");
        Assert.True(At(64).CastShadow, "a shadow-only motif always casts.");
        Assert.Equal(MaterialClass.ShadowOnly, At(64).Material);
        Assert.False(At(80).CastShadow, "the motif's own cast flag is false.");

        DrawTile At(int pixelX)
        {
            int cell = (pixelX / 8) + 1;
            Assert.True(byColumn.ContainsKey(cell), $"no tile in cell column {cell}.");
            return byColumn[cell];
        }
    }

    /// <summary>
    /// REQ-SCN-032, REQ-SCN-033 — switching a layer's cast flag off through Lua removes its
    /// tiles' shadows from the next frame on, and the shadow flags are reset at every profile
    /// load: L1 casts and receives, L2 to L4 receive without casting, UI and UNIDENTIFIED
    /// neither. Values stored in the profile are ignored.
    /// </summary>
    [Fact] // REQ-SCN-033
    public void The_layer_shadow_flags_are_reset_at_every_profile_load()
    {
        GameProfile profile = GameProfile.CreateEmpty();

        // Whatever the profile stores for the layers must be ignored.
        profile.Settings!.Layers!.Layers[2].StoredCastShadow = false;
        profile.Settings.Layers.Layers[3].StoredCastShadow = true;
        profile.Settings.Layers.Layers[1].StoredCastShadow = true;

        using LuaRun run = LuaRun.Create(profile);

        Assert.False(run.Settings.Layer(0).CastShadow);
        Assert.False(run.Settings.Layer(0).RecvShadow);
        Assert.False(run.Settings.Layer(1).CastShadow);
        Assert.False(run.Settings.Layer(1).RecvShadow);
        Assert.True(run.Settings.Layer(2).CastShadow);
        Assert.True(run.Settings.Layer(2).RecvShadow);
        for (int layer = 3; layer <= 5; layer++)
        {
            Assert.False(run.Settings.Layer(layer).CastShadow, $"layer {layer} must not cast.");
            Assert.True(run.Settings.Layer(layer).RecvShadow, $"layer {layer} must receive.");
        }

        // Lua may change them during the game.
        run.Do("Setting.Layer[3].CastShadow = true");
        Assert.True(run.Settings.Layer(3).CastShadow);
        run.Do("Setting.Layer[2].CastShadow = false");
        Assert.False(run.Settings.Layer(2).CastShadow);
    }

    /// <summary>
    /// REQ-SCN-033, criterion — 13 built-in scripts switch <c>Setting.Layer[3].CastShadow</c> on.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-SCN-033
    public void Thirteen_built_in_scripts_switch_the_L2_cast_flag_on()
    {
        int scripts = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            foreach (ProfileScript script in ProfileCorpus.Load(path).Scripts)
            {
                string source = script.Source ?? string.Empty;
                if (source.Contains("Layer[3].CastShadow", StringComparison.Ordinal))
                {
                    scripts++;
                }
            }
        }

        Assert.Equal(13, scripts);
    }

    // --------------------------------------------- §3.11 materials and tile shading

    /// <summary>
    /// REQ-SCN-034 — the albedo comes from the palette row of the forced palette of the motif
    /// when it has one, otherwise the tile's palette for a background shape and the tile's
    /// palette + 4 for a sprite shape.
    /// </summary>
    [Fact] // REQ-SCN-034
    public void The_palette_row_follows_the_motif_the_shape_and_the_sprite_flag()
    {
        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.L1)
            .Tile(SceneFixture.Solid(1, 0), 0, 96, palette: 2)
            .Sprite(SceneFixture.Solid(1, 1), 64, 96, palette: 1);

        SceneState scene = fixture.Build();

        DrawTile background = scene.DrawList!.Tiles.First(static tile => !tile.Sprite);
        DrawTile sprite = scene.DrawList.Tiles.First(static tile => tile.Sprite);

        Assert.Equal(2, background.PaletteTable);
        Assert.Equal(1 + 4, sprite.PaletteTable);

        // The frame palette table is the one of SPEC-04 REQ-E3D-250: 64 entries.
        Assert.Equal(64, scene.DrawList.Palette.Length);
    }

    /// <summary>
    /// REQ-SCN-035 — the sRGB and linear transfer functions are the standard ones.
    /// </summary>
    [Fact] // REQ-SCN-035
    public void The_transfer_functions_are_the_standard_sRGB_curves()
    {
        Assert.Equal(0f, SceneColor.ToLinear(0f), 1e-6f);
        Assert.Equal(1f, SceneColor.ToLinear(1f), 1e-6f);
        Assert.Equal(0.21404f, SceneColor.ToLinear(0.5f), 1e-4f);
        Assert.Equal(0.5f, SceneColor.ToSrgb(0.21404f), 1e-3f);
        Assert.Equal(0.04f / 12.92f, SceneColor.ToLinear(0.04f), 1e-6f);

        for (float value = 0f; value <= 1f; value += 0.05f)
        {
            Assert.Equal(value, SceneColor.ToSrgb(SceneColor.ToLinear(value)), 1e-4f);
        }
    }

    /// <summary>
    /// REQ-SCN-036 — a fragment whose scene position leaves <c>R</c> is cut, whatever its depth;
    /// the rectangle is the clipping one.
    /// </summary>
    [Fact] // REQ-SCN-036
    public void The_visible_rectangle_cuts_on_the_x_and_y_of_the_scene_position()
    {
        VisibleRect rect = VisibleRect.From(DefaultClipping);

        Assert.True(rect.Contains(128f, 120f));
        Assert.False(rect.Contains(0f, 120f));
        Assert.False(rect.Contains(250f, 120f));
        Assert.False(rect.Contains(128f, 0f));
        Assert.False(rect.Contains(128f, 245f));
        Assert.True(rect.Contains(0.1f, 8.1f));
        Assert.True(rect.Contains(247.9f, 239.9f));
    }

    /// <summary>
    /// REQ-SCN-037 — the renderer alone owns the ±0.1 depth nudge of the sprite tiles: the
    /// transform the engine delivers does not include it, and the engine only reports the sprite
    /// and priority flags (DEC-R2-T4).
    /// </summary>
    [Fact] // REQ-SCN-037
    public void The_sprite_depth_nudge_is_not_in_the_delivered_transform()
    {
        Assert.Equal(0.1f, SceneConstants.SpriteNudge);

        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 1, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 1, ProfileLayer.L1)
            .Tile(SceneFixture.Solid(1, 0), 0, 96)
            .Sprite(SceneFixture.Solid(1, 1), 0, 96)
            .Sprite(SceneFixture.Solid(1, 1), 64, 96, behindBackground: true);

        SceneState scene = fixture.Build();
        DrawTile background = scene.DrawList!.Tiles.First(static tile => !tile.Sprite);
        List<DrawTile> sprites = [.. scene.DrawList.Tiles.Where(static tile => tile.Sprite)];

        Assert.Equal(2, sprites.Count);
        Assert.Contains(sprites, static tile => tile.BehindBackground);
        Assert.Contains(sprites, static tile => !tile.BehindBackground);

        // The sprite in front of the background was placed at exactly the same depth as the
        // background tile: the nudge is the renderer's business.
        DrawTile inFront = sprites.First(static tile => !tile.BehindBackground);
        Assert.Equal(background.Anchor.Z, inFront.Anchor.Z, 1e-4f);
    }

    /// <summary>
    /// REQ-SCN-040, REQ-SCN-041 — the render style and the alpha decide the material class:
    /// style 0 is invisible and draws nothing, style 1 with alpha 1 is opaque, alpha below 1 is
    /// transparent and casts no shadow, and style 2 is shadow-only and always opaque.
    /// </summary>
    [Fact] // REQ-SCN-040, REQ-SCN-041
    public void The_render_style_and_the_alpha_decide_the_material()
    {
        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 2, ProfileLayer.L1)
            .Pattern(SceneFixture.Solid(1, 1), depth: 2, ProfileLayer.L1, alpha: 0.5f)
            .Pattern(
                SceneFixture.Solid(1, 2), depth: 2, ProfileLayer.L1,
                style: VariantRenderStyle.ShadowOnly)
            .Pattern(
                SceneFixture.Solid(1, 3), depth: 2, ProfileLayer.L1,
                style: VariantRenderStyle.Hidden)
            .Pattern(SceneFixture.Solid(1, 4), depth: 2, ProfileLayer.Ui)
            .Tile(SceneFixture.Solid(1, 0), 0, 96)
            .Tile(SceneFixture.Solid(1, 1), 16, 96)
            .Tile(SceneFixture.Solid(1, 2), 32, 96)
            .Tile(SceneFixture.Solid(1, 3), 48, 96)
            .Tile(SceneFixture.Solid(1, 4), 64, 96);

        SceneState scene = fixture.Build();
        Dictionary<int, DrawTile> byColumn = scene.DrawList!.Tiles
            .ToDictionary(static tile => tile.Tile.Position.X);

        Assert.Equal(MaterialClass.Opaque, byColumn[1].Material);
        Assert.Equal(MaterialClass.Transparent, byColumn[3].Material);
        Assert.Equal(0.5f, byColumn[3].Alpha, 4);
        Assert.Equal(MaterialClass.ShadowOnly, byColumn[5].Material);
        Assert.Equal(1f, byColumn[5].Alpha, 4);
        Assert.Equal(MaterialClass.Interface, byColumn[9].Material);

        // Style 0 draws nothing at all.
        Assert.False(byColumn.ContainsKey(7), "an invisible motif must produce no tile.");
    }

    /// <summary>
    /// REQ-SCN-043 — the animated deformation follows SPEC-04B REQ-E3D-568 and the <b>emulated</b>
    /// time, so it freezes while the game is paused, and the parameters are delivered with each
    /// tile (REQ-E3D-641).
    /// </summary>
    [Fact] // REQ-SCN-043
    public void The_animated_deformation_uses_the_emulated_time()
    {
        SceneFixture fixture = new();
        fixture.Pattern(SceneFixture.Solid(1), depth: 8, ProfileLayer.L1).Tile(SceneFixture.Solid(1), 64, 96);
        SceneState scene = fixture.Build();

        DrawTile flat = Assert.Single(scene.DrawList!.Tiles);
        Assert.Null(flat.Deform);

        // With a non-zero amplitude the parameters travel with the tile and the displacement
        // depends on the emulated time.
        ConversionEngine engine = fixture.Engine!;
        PatternInstance instance = engine.Patterns.Permanent[0].Instances[0];
        instance.DeformAmplitude = new Vec3(0f, 0f, 1f);
        Mat4 speed = default;
        speed[2, 3] = 2f;
        instance.DeformSpeed = speed;

        engine.Convert(
            new Zax.Nes.FrameBatch(
                [new Zax.Nes.TileRecord(
                    SceneFixture.Solid(1).Low, SceneFixture.Solid(1).High, 64, 96, 0,
                    false, false, false, 0, true)],
                new byte[32], 0, 0),
            frameCounter: 60);
        engine.Publish();

        DrawTile animated = Assert.Single(engine.BuildDrawList().Tiles);
        Assert.NotNull(animated.Deform);

        DeformParameters parameters = animated.Deform!.Value;
        Vec3 atZero = parameters.Displacement(default, 0f);
        Vec3 atQuarter = parameters.Displacement(default, 0.25f);
        Assert.NotEqual(atZero.Z, atQuarter.Z);

        // The time the scene hands the renderer is the emulated one: frames / 60.
        Assert.Equal(1f, engine.PublishedFrame!.Time, 1e-4f);
    }

    // -------------------------------------------------------------------- §3.12 HUD

    /// <summary>
    /// REQ-SCN-045 — the HUD plane is attached to the camera at
    /// <c>E + d_HUD · f</c> with <c>d_HUD = 60 / atan(FOV_rad / 2)</c> and the scale 0.5; an
    /// element at <c>p</c> in the reference frame appears at <c>(p − C) × 0.5</c> along the
    /// camera axes.
    /// </summary>
    [Fact] // REQ-SCN-045
    public void The_HUD_plane_hangs_at_the_documented_distance_and_scale()
    {
        Assert.Equal(109.42f, SceneConstants.HudDistance(70f), 0.02f);
        Assert.Equal(90.12f, SceneConstants.HudDistance(90f), 0.02f);
        Assert.Equal(0.5f, SceneConstants.HudScale);

        SceneCamera camera = new();
        camera.SnapToSetpoints();
        CameraView view = camera.BuildView(DefaultClipping, 16f / 9f);

        Assert.Equal(SceneConstants.HudDistance(70f), view.HudDistance, 0.02f);

        // The centre C of the reference frame lands on the axis of the HUD plane.
        Vector3 centre = view.HudPoint(SceneConstants.Center);
        Assert.Equal(view.Eye + (view.Forward * view.HudDistance), centre);

        // An element at the centre of the NES image (128, 128) appears 4 units above it.
        Vector3 nesCentre = view.HudPoint(new Vector3(128f, 128f, 0f));
        Assert.Equal(4f, Vector3.Dot(nesCentre - centre, view.Up), 1e-3);
        Assert.Equal(0f, Vector3.Dot(nesCentre - centre, view.Right), 1e-3);

        // At FOV 70 the 256 × 240 frame covers about 78 % of the image height.
        double visibleHeight = 2.0 * view.HudDistance * Math.Tan(70.0 * Math.PI / 180.0 / 2.0);
        Assert.Equal(0.78, 240.0 * 0.5 / visibleHeight, 0.01);
    }

    // ----------------------------------------------- §3.13 background and backdrop

    /// <summary>
    /// REQ-SCN-046 — the background colour table: black outside a game, the frame's universal
    /// NES colour while a game runs with <c>Setting.BgColor</c> true, the custom colour when it
    /// is false, and the custom colour in Skybox = Color whatever the state.
    /// </summary>
    [Theory] // REQ-SCN-046
    [InlineData(SkyboxKind.Default, false, true, 0x22, 0x0F, 0x0F)]
    [InlineData(SkyboxKind.Default, true, true, 0x22, 0x0F, 0x22)]
    [InlineData(SkyboxKind.Default, true, false, 0x22, 0x16, 0x16)]
    [InlineData(SkyboxKind.Color, false, true, 0x22, 0x16, 0x16)]
    [InlineData(SkyboxKind.Color, true, true, 0x22, 0x16, 0x16)]
    public void The_background_colour_table(
        SkyboxKind skybox, bool running, bool useNes, int nes, int custom, int expected)
    {
        Rgb24 colour = SceneState.ResolveBackground(
            skybox,
            running,
            useNes,
            NesPalette.Color(nes),
            NesPalette.Color(custom));

        Assert.Equal(NesPalette.Color(expected), colour);
    }

    /// <summary>
    /// REQ-SCN-046 — <c>Setting.BgColor</c> is forced to true at every profile load and Lua may
    /// change it afterwards; the custom colour is a user setting Lua may also change.
    /// </summary>
    [Fact] // REQ-SCN-046
    public void BgColor_is_forced_true_at_every_profile_load()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        SceneState scene = new();
        scene.UseNesBackgroundColor = false;

        using SessionDriver session = SessionDriver.CreateOnProfile(profile, scene);
        session.ApplyProfileSettings(scene, profile.Settings);

        Assert.True(scene.UseNesBackgroundColor);

        session.Scripts.Evaluate("Setting.BgColor = false");
        Assert.False(scene.UseNesBackgroundColor);

        session.Scripts.Evaluate("Setting.CustomBgColor = Color(1, 0, 0)");
        Assert.Equal(new Rgb24(255, 0, 0), scene.Settings.CustomBackgroundColor);
    }

    /// <summary>
    /// REQ-SCN-047 — outside a running game the 3D scene is not displayed: only the background
    /// or the skybox stays behind the interface.
    /// </summary>
    /// <remarks>
    /// Q-029, answered by AMD-013: at a <b>pause</b> the scene stays visible, frozen on the last
    /// emulated frame, because AMD-010 replaces the opaque menu with a menu bar. In the Stopped
    /// state the scene is not drawn, which is what the requirement still asks.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-047
    public void The_scene_is_not_displayed_in_the_stopped_state()
    {
        using HostDriver host = HostDriver.Create();
        Assert.False(host.Scene.GameRunning);

        host.Launch2DMode();
        Assert.True(host.Scene.GameRunning);

        host.Run(Zax.App.Loop.ZaxCommands.Pause);
        Assert.True(host.Scene.GameRunning, "AMD-013 keeps the frozen scene visible in pause.");

        host.Run(Zax.App.Loop.ZaxCommands.Stop);
        Assert.False(host.Scene.GameRunning);
    }

    /// <summary>
    /// REQ-SCN-048 — the backdrop defaults, its geometry and the reset at the end of each game.
    /// </summary>
    [Fact] // REQ-SCN-048
    public void The_backdrop_has_the_documented_defaults_and_geometry()
    {
        BackdropState backdrop = new();
        Assert.False(backdrop.Enabled);
        Assert.Equal(default, backdrop.Color);
        Assert.Equal(new Vector3(256f, 240f, 1f), backdrop.Size);
        Assert.Equal(Vector3.Zero, backdrop.Offset);
        Assert.True(backdrop.ReceiveShadow);

        // Centred at C + offset, sides (SizeX, SizeY, SizeZ): with OfsZ = 26 and SizeZ = 1 the
        // front face is at z = 25.5.
        backdrop.Offset = new Vector3(0f, 0f, 26f);
        Vector3 centre = SceneConstants.Center + backdrop.Offset;
        Assert.Equal(25.5f, centre.Z - (backdrop.Size.Z / 2f), 1e-4f);
        Assert.Equal(128f, centre.X, 1e-4f);
        Assert.Equal(120f, centre.Y, 1e-4f);

        // By default the box covers the reference frame, y from 0 to 240.
        backdrop.Reset();
        Assert.Equal(0f, SceneConstants.Center.Y - (backdrop.Size.Y / 2f), 1e-4f);
        Assert.Equal(240f, SceneConstants.Center.Y + (backdrop.Size.Y / 2f), 1e-4f);
    }

    // ---------------------------------------------------------------- §3.14 skyboxes

    /// <summary>
    /// REQ-SCN-049 — the four skybox values are stored as integers: −1 Default, 0 Color,
    /// 1 Gradient, 2 Fantasy, and only Gradient and Fantasy draw a sky.
    /// </summary>
    [Fact] // REQ-SCN-049
    public void The_skybox_setting_has_four_integer_values()
    {
        Assert.Equal(-1, (int)SkyboxKind.Default);
        Assert.Equal(0, (int)SkyboxKind.Color);
        Assert.Equal(1, (int)SkyboxKind.Gradient);
        Assert.Equal(2, (int)SkyboxKind.Fantasy);

        RenderSettings settings = new();
        Assert.Equal(SkyboxKind.Default, settings.Skybox);
        Assert.False(settings.SkyboxDrawn);

        settings.Skybox = SkyboxKind.Color;
        Assert.False(settings.SkyboxDrawn);
        settings.Skybox = SkyboxKind.Gradient;
        Assert.True(settings.SkyboxDrawn);
        settings.Skybox = SkyboxKind.Fantasy;
        Assert.True(settings.SkyboxDrawn);
    }

    /// <summary>
    /// REQ-SCN-050 — the Gradient sky: <c>Hc = 1.4 M</c>, <c>Bc = 0.6 M</c>,
    /// <c>c = M + (Hc − M) · max(y, 0)^2.5</c> then <c>g = c + (Bc − c) · max(−y, 0)^2.5</c>, and
    /// stars appear only when the grey level of <c>M</c> is at most 0.4.
    /// </summary>
    [Fact] // REQ-SCN-050
    public void The_gradient_sky_formula_and_its_star_threshold()
    {
        Assert.Equal(1.4f, SkyGradient.ZenithFactor);
        Assert.Equal(0.6f, SkyGradient.NadirFactor);
        Assert.Equal(0.4f, SkyGradient.StarGreyThreshold);
        Assert.Equal(3f, SkyGradient.StarDegreesPerSecond);

        Rgb24 m = NesPalette.Color(0x22);
        Assert.Equal(104, m.R);
        Assert.Equal(136, m.G);
        Assert.Equal(252, m.B);

        // The horizon is M itself.
        (float r, float g, float b) = SkyGradient.Sky(m, 0f);
        Assert.Equal(m.R / 255f, r, 1e-5f);
        Assert.Equal(m.G / 255f, g, 1e-5f);
        Assert.Equal(m.B / 255f, b, 1e-5f);

        // The zenith is 1.4 M, not saturated before the conversion.
        (r, g, b) = SkyGradient.Sky(m, 1f);
        Assert.Equal(1.4f * m.R / 255f, r, 1e-5f);
        Assert.Equal(1.4f * m.B / 255f, b, 1e-5f);

        // The nadir is 0.6 M.
        (r, g, b) = SkyGradient.Sky(m, -1f);
        Assert.Equal(0.6f * m.R / 255f, r, 1e-5f);
        Assert.Equal(0.6f * m.G / 255f, g, 1e-5f);

        // The 2.5 exponent between.
        (r, _, _) = SkyGradient.Sky(m, 0.5f);
        float up = MathF.Pow(0.5f, 2.5f);
        Assert.Equal((m.R / 255f) + ((1.4f - 1f) * (m.R / 255f) * up), r, 1e-5f);

        // Stars: not on 0x22 (grey 0.566), yes on 0x0F (black).
        Assert.False(SkyGradient.StarsVisible(m));
        Assert.True(SkyGradient.StarsVisible(NesPalette.Color(0x0F)));
        Assert.Equal(0.5477f, SkyGradient.Grey(m), 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-051 — the "×2" of the Fantasy sky means multiplying the linearised tint by what
    /// doubling an sRGB value does to it, under the sRGB curve SPEC-06 §2 declares normative:
    /// <c>lin(1) / lin(0.5) ≈ 4.672</c>, within the ±2 % AMD-019 grants.
    /// </summary>
    /// <remarks>
    /// REQ-SCN-051 writes 4.595 — which is 2^2.2, the pure power law — in the same sentence as
    /// « doublement en sRGB ». The round-1 test pinned that written number to 1e-3; AMD-019
    /// (answer to <c>Q-045</c>, see also <c>Q-055</c>) reverses the choice: the curve of §2 is the
    /// authority, « ×2 » was an approximation, and the criterion carries a ±2 % tolerance. The
    /// expected value is therefore recomputed from the curve itself rather than written down.
    /// </remarks>
    [Fact] // REQ-SCN-051 — AMD-019 (Q-045), Q-055
    public void The_fantasy_sky_doubling_factor_is_the_documented_one()
    {
        // The ratio the piecewise curve of SPEC-06 §2 gives, computed and not transcribed.
        float expected = SceneColor.ToLinear(1f) / SceneColor.ToLinear(0.5f);
        Assert.Equal(4.672f, expected, 1e-3f);

        // The ±2 % of AMD-019, on the value the amendment makes exact.
        Assert.True(
            MathF.Abs(SkyGradient.SrgbDouble - expected) <= 0.02f * expected,
            $"SrgbDouble = {SkyGradient.SrgbDouble}, expected {expected} ±2 %.");

        // And it is that value that is implemented, not the 2^2.2 of the requirement's text,
        // which AMD-019 downgrades to an approximation.
        Assert.Equal(4.672f, SkyGradient.SrgbDouble, 1e-3f);
        Assert.True(MathF.Abs(SkyGradient.SrgbDouble - MathF.Pow(2f, 2.2f)) > 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-052 — the background and the skyboxes follow the colour of the <b>displayed</b>
    /// frame, so the background, the sky and the tiles change on the same image.
    /// </summary>
    [Fact] // REQ-SCN-052
    public void The_background_follows_the_displayed_frame()
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x22)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1)
            .Tile(SceneFixture.Solid(1), 64, 96);

        SceneState scene = fixture.Build();
        Assert.Equal(NesPalette.Color(0x22), scene.NesBackgroundColor);
        Assert.Equal(NesPalette.Color(0x22), scene.BackgroundColor);

        // The engine's own accessor reads the published frame, never the one being converted.
        Assert.Equal(
            fixture.Engine!.PublishedFrame!.Palette.Color(0),
            scene.NesBackgroundColor);
    }

    // ------------------------------------------------------------- §3.15 TV box

    /// <summary>
    /// REQ-SCN-053, §5.3 — the five tunnel boxes with the clipping (0.1 ; 8.1 ; 8.1 ; 0.1).
    /// </summary>
    [Fact] // REQ-SCN-053
    public void The_tunnel_boxes_of_section_5_3()
    {
        Assert.Equal(5, TvBox.TunnelBoxCount);

        TunnelBox[] tunnel = TvBox.Tunnel(DefaultClipping);
        Assert.Equal(5, tunnel.Length);

        // left: x ∈ [−8 ; 0.1], y ∈ [4.1 ; 243.9], z ∈ [−16 ; 112].
        Box(tunnel[0], -8f, 0.1f, 4.1f, 243.9f, -16f, 112f);

        // right: x ∈ [247.9 ; 264].
        Box(tunnel[1], 247.9f, 264f, 4.1f, 243.9f, -16f, 112f);

        // bottom: y ∈ [−4 ; 8.1].
        Box(tunnel[2], -8f, 264f, -4f, 8.1f, -16f, 112f);

        // top: y ∈ [239.9 ; 244].
        Box(tunnel[3], -8f, 264f, 239.9f, 244f, -16f, 112f);

        // back: x ∈ [0 ; 256], y ∈ [0 ; 240], z ∈ [112 ; 120].
        Box(tunnel[4], 0f, 256f, 0f, 240f, 112f, 120f);

        static void Box(TunnelBox box, float x0, float x1, float y0, float y1, float z0, float z1)
        {
            Assert.Equal(x0, box.Min.X, 1e-3f);
            Assert.Equal(x1, box.Max.X, 1e-3f);
            Assert.Equal(y0, box.Min.Y, 1e-3f);
            Assert.Equal(y1, box.Max.Y, 1e-3f);
            Assert.Equal(z0, box.Min.Z, 1e-3f);
            Assert.Equal(z1, box.Max.Z, 1e-3f);
        }
    }

    /// <summary>
    /// REQ-SCN-054, §5.3 — the TV frame: the origin
    /// <c>C + ((c0 − c2) / 256, (c1 − c3) / 240, −16)</c> and the scales
    /// <c>(256 − c0 − c2) / 256</c>, <c>(240 − c1 − c3) / 240</c>, 1.
    /// </summary>
    [Fact] // REQ-SCN-054
    public void The_TV_frame_origin_and_scale_of_section_5_3()
    {
        Vector3 origin = TvBox.FrameOrigin(DefaultClipping);
        Vector3 scale = TvBox.FrameScale(DefaultClipping);

        Assert.Equal(SceneConstants.Center.X + (-0.03125f), origin.X, 1e-4f);
        Assert.Equal(SceneConstants.Center.Y + 0.03333f, origin.Y, 1e-4f);
        Assert.Equal(SceneConstants.Center.Z - 16f, origin.Z, 1e-4f);

        Assert.Equal(0.96797f, scale.X, 1e-4f);
        Assert.Equal(0.96583f, scale.Y, 1e-4f);
        Assert.Equal(1f, scale.Z, 1e-4f);
    }

    /// <summary>
    /// REQ-SCN-053 with AMD-025 — the TV box setting has two values now, and a stored 2 or 3,
    /// the TV 2 and TV 3 of REQ-SCN-054, comes back as the one model.
    /// </summary>
    [Fact] // REQ-SCN-053, AMD-025
    public void The_TV_box_setting_has_one_model_and_migrates_the_old_values()
    {
        Assert.Equal(TvBoxKind.None, TvBoxSetting.From(0));
        Assert.Equal(TvBoxKind.Tv, TvBoxSetting.From(1));
        Assert.Equal(TvBoxKind.Tv, TvBoxSetting.From(2));
        Assert.Equal(TvBoxKind.Tv, TvBoxSetting.From(3));

        // Out-of-range values settle on one side or the other rather than on a value the
        // renderer could not read.
        Assert.Equal(TvBoxKind.None, TvBoxSetting.From(-4));
        Assert.Equal(TvBoxKind.Tv, TvBoxSetting.From(9));

        Assert.Equal(0, TvBoxSetting.Normalise(0));
        Assert.Equal(1, TvBoxSetting.Normalise(3));
        Assert.Equal(1, TvBoxSetting.Max);
    }

    /// <summary>
    /// REQ-SCN-054, notes — the textures and the cubemap the three TV models need are present in
    /// <c>assets-neutral</c>.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-SCN-054
    public void The_TV_textures_and_the_cubemap_are_in_the_neutral_assets()
    {
        string textures = Path.Combine(NeutralAssets.Root, "textures");
        string cubemaps = Path.Combine(NeutralAssets.Root, "cubemaps");
        string meshes = Path.Combine(NeutralAssets.Root, "meshes");

        foreach (string name in new[]
        {
            "Tv_3_4_diff", "Tv_3_4_normal", "Tv_1_2_diff", "Tv_1_2_normal", "Tv_1_2_gloss",
        })
        {
            Assert.True(
                Directory.GetFiles(textures, name + ".*").Length > 0,
                $"the texture {name} is missing.");
        }

        Assert.True(
            Directory.GetFiles(textures, "2000s_CRT_TV_black_*").Length > 0,
            "the TV 3 textures are missing.");
        Assert.True(Directory.GetFiles(cubemaps, "Cubemap*").Length > 0, "the cubemap is missing.");

        foreach (string mesh in new[] { "Object003.obj", "Object004.obj", "2000s_CRT_TV_LOD0.obj" })
        {
            Assert.True(File.Exists(Path.Combine(meshes, mesh)), $"the mesh {mesh} is missing.");
        }
    }

    // ----------------------------------------------------------------- §3.16 2D mode

    /// <summary>
    /// REQ-SCN-055 — "2D Mode" is not a separate render path: it is the embedded profile with no
    /// motif, rendered by the same 3D pipeline with the same camera and the same scene settings.
    /// AMD-009 (Q-022) makes it show the core's pixel frame on the NES plane.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-SCN-055
    public void The_two_dimensional_mode_is_the_empty_profile_in_the_same_pipeline()
    {
        GameProfile empty = ProfileCorpus.Named("empty")
            ?? throw new InvalidOperationException("the profile 'empty' is missing.");

        Assert.Empty(empty.PermanentPatterns);
        Assert.Empty(empty.TemporaryPatterns);

        SceneState scene = new();
        using SessionDriver session = SessionDriver.CreateOnProfile(empty, scene);
        Assert.True(session.Engine.TwoDimensionalMode);

        // The same camera and the same settings: nothing about the camera depends on the mode.
        Assert.Equal(CameraMode.Tpv, scene.Camera.Mode);
        Assert.Equal(SceneConstants.DefaultDistance(70f), scene.Camera.Current.Distance, 2);
    }

    /// <summary>
    /// REQ-SCN-055, notes — the "TV Mode" setting chooses the emulated region and has no effect
    /// on the rendering.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-055
    public void The_TV_Mode_setting_only_changes_the_region()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        SkyboxKind skybox = host.Scene.Settings.Skybox;
        float fov = host.Scene.Settings.Fov;

        host.Settings.TvMode = Zax.App.Persistence.TvRegionSetting.Pal;
        host.ApplyCommands();

        Assert.Equal(Zax.Nes.NesRegion.Pal, host.Session!.Core.Region);
        Assert.Equal(skybox, host.Scene.Settings.Skybox);
        Assert.Equal(fov, host.Scene.Settings.Fov, 4);
    }

    // ------------------------------------------------------------- §3.17 pipeline

    /// <summary>
    /// REQ-SCN-060, REQ-SCN-061, REQ-SCN-062, REQ-SCN-063 — the pipeline settings of §4.1: the
    /// defaults and the bounds.
    /// </summary>
    [Fact] // REQ-SCN-060, REQ-SCN-061
    public void The_pipeline_settings_have_the_documented_defaults_and_bounds()
    {
        RenderSettings settings = new();

        Assert.True(settings.SkipRender);
        Assert.Equal(0, settings.VSync);
        Assert.Equal(90, settings.TargetFrameRate);
        Assert.True(settings.Shadows);
        Assert.Equal(TvBoxKind.None, settings.TvBox);
        Assert.Equal(default, settings.CustomBackgroundColor);
        Assert.Equal(70f, settings.Fov);

        settings.VSync = 7;
        Assert.Equal(2, settings.VSync);
        settings.VSync = -1;
        Assert.Equal(0, settings.VSync);

        foreach (int rate in new[] { 60, 90, 120 })
        {
            settings.TargetFrameRate = rate;
            Assert.Equal(rate, settings.TargetFrameRate);
        }

        settings.TargetFrameRate = 144;
        Assert.Equal(90, settings.TargetFrameRate);
    }

    /// <summary>
    /// REQ-SCN-059 — the camera is driven by real time, never by the emulated rate: the same
    /// rotation over 3 s covers the same angle at 60, 90 and 120 frames per second, and the
    /// camera keeps being smoothed while the game is paused.
    /// </summary>
    [Fact] // REQ-SCN-059
    public void The_camera_is_driven_by_real_time_at_any_frame_rate()
    {
        foreach (int rate in new[] { 60, 90, 120 })
        {
            SceneCamera camera = new();
            camera.Current.Angles = Vector3.Zero;
            camera.SnapToSetpoints();

            for (int tick = 0; tick < 3 * rate; tick++)
            {
                CameraControls.Apply(camera, CameraAction.RotateRight, 1f / rate);
                camera.Advance(1f / rate);
            }

            Assert.Equal(90f, camera.Current.Angles.Y, 90f * 0.02f);

            // The displayed angle lags the setpoint by the smoothing, and that lag is itself
            // rate-independent: let the filter settle and it reaches the same angle.
            for (int tick = 0; tick < 3 * rate; tick++)
            {
                camera.Advance(1f / rate);
            }

            Assert.Equal(90f, camera.Angles.Y, 90f * 0.02f);
        }
    }

    /// <summary>
    /// REQ-SCN-060 — with render skip on, a rendered frame for which no new emulated frame is
    /// available may be omitted while the game is running; with it off the scene is drawn at
    /// every refresh.
    /// </summary>
    [Fact] // REQ-SCN-060
    public void Render_skip_only_omits_a_frame_while_the_game_runs()
    {
        System.Reflection.MethodInfo skip = typeof(Zax.App.AppOptions).Assembly
            .GetType("Zax.App.RenderSkip", true)!
            .GetMethod("ShouldSkip", System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)!;

        // The rule is a pure decision: (setting, running, new frames, target unchanged) → skip.
        Assert.False(Skip(true, true, 1, true));
        Assert.True(Skip(true, true, 0, true));
        Assert.False(Skip(false, true, 0, true));
        Assert.False(Skip(true, false, 0, true));
        Assert.False(Skip(true, true, 0, false));

        bool Skip(bool enabled, bool running, int newFrames, bool unchanged)
            => (bool)skip.Invoke(null, [enabled, running, newFrames, unchanged])!;
    }

    /// <summary>Asserts that two channel values are within <paramref name="tolerance"/>.</summary>
    private static void Near(int expected, int actual, int tolerance)
        => Assert.True(
            Math.Abs(expected - actual) <= tolerance,
            $"expected {expected} ± {tolerance}, got {actual}.");
}
