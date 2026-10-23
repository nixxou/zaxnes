using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;
using Zax.Render.Assets;

namespace Zax.Acceptance.Tests;

/// <summary>
/// The items of <c>AMD-021</c> that need a GL 3.3 core context: the linear blending of the
/// transparencies (R2-009) and the absence of shadow on a transparent receiver (R2-010).
/// R2-014 covered the materials of the TV models, which AMD-031 removed from the product.
/// Captures are archived in <c>verify/round1/final/</c>.
/// </summary>
[Collection("opengl-acceptance")]
public sealed class Amd021SceneTests
{
    /// <summary>
    /// The lighting of a lit front face: the ambient term plus N·L = 0.577 (SPEC-06 §3.6 and
    /// the vectors of §5.2), which the criterion of R2-009 is stated for.
    /// </summary>
    private const float LitFrontFace = 0.1812f + 0.577f;

    [GlFact] // AMD-021 R2-009 (REQ-SCN-040) — first half of the criterion
    public void R2_009_a_half_opaque_motif_over_black_gives_sRGB_158_124_145()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        // NES $35 is F4C0E0, the colour the criterion names. Alpha 0.5, no shadow, over the
        // black universal backdrop ($0F).
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Palette(1, 0x35)
            .Pattern(SceneFixture.Solid(1), depth: 2, ProfileLayer.L1, alpha: 0.5f, columns: 16, rows: 14)
            .Rect(SceneFixture.Solid(1), column: 8, row: 8, columns: 16, rows: 14, palette: 0);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        scene.Settings.Shadows = false;

        Shot shot = target.Render(renderer, scene);
        (int r, int g, int b) = shot.At(shot.Width / 2, shot.Height / 2);

        // Blended on the linear colours the criterion gives (158, 124, 145); blended on the
        // sRGB ones it would be about (108, 85, 99), which is what this measurement rules out.
        Assert.InRange(r, 156, 160);
        Assert.InRange(g, 122, 126);
        Assert.InRange(b, 143, 147);

        // The same numbers derived from the amendment's own terms, independently of the
        // renderer: half of the lit motif in linear, re-encoded to sRGB.
        Assert.Equal(158, Blend(0x35, LitFrontFace, 0.5f, 0f).R);
        Assert.Equal(124, Blend(0x35, LitFrontFace, 0.5f, 0f).G);
        Assert.Equal(145, Blend(0x35, LitFrontFace, 0.5f, 0f).B);

        shot.SavePng(Path.Combine(FinalCaptures.Ready(), "amd021-r2-009-over-black.png"));
    }

    [GlFact] // AMD-021 R2-009 (REQ-SCN-040) — second half of the criterion
    public void R2_009_the_same_motif_over_sRGB_0_120_131_gives_green_147()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        // The wall is NES $1C (0, 136, 148); its lit front face is sRGB (0, 120, 131), the
        // background colour the criterion names. It is measured on its own first.
        SceneState wallOnly = Wall(motif: false);
        Shot plain = target.Render(renderer, wallOnly);
        (int wr, int wg, int wb) = plain.At(plain.Width / 2, plain.Height / 2);
        Assert.InRange(wr, 0, 2);
        Assert.InRange(wg, 118, 122);
        Assert.InRange(wb, 129, 133);

        // The half-opaque motif of NES $35 in front of it: the green channel comes out at 147.
        // Blended in sRGB it would be about 134.
        SceneState withMotif = Wall(motif: true);
        Shot shot = target.Render(renderer, withMotif);
        (int _, int g, int _) = shot.At(shot.Width / 2, shot.Height / 2);
        Assert.InRange(g, 145, 149);

        // And the same number from the amendment's terms: half the lit motif plus half the
        // wall, in linear.
        Assert.Equal(147, Blend(0x35, LitFrontFace, 0.5f, Linear(0x1C, LitFrontFace).G).G);

        plain.SavePng(Path.Combine(FinalCaptures.Ready(), "amd021-r2-009-wall.png"));
        shot.SavePng(Path.Combine(FinalCaptures.Ready(), "amd021-r2-009-over-wall.png"));
    }

    /// <summary>
    /// A wall of NES $1C tiles, optionally with a half-opaque motif of NES $35 in front of it.
    /// The motif is laid down as sprite records: two background records on the same cell would
    /// replace one another and open a hole in the wall, which the measurement must not have.
    /// </summary>
    private static SceneState Wall(bool motif)
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Palette(1, 0x1C)
            .Palette(18, 0x35)
            .Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L2, columns: 24, rows: 22)
            .Pattern(SceneFixture.Solid(2), depth: 6, ProfileLayer.L1, alpha: 0.5f)
            .Rect(SceneFixture.Solid(1), column: 4, row: 4, columns: 24, rows: 22, palette: 0);

        if (motif)
        {
            for (int row = 11; row < 19; row++)
            {
                for (int column = 11; column < 21; column++)
                {
                    fixture.Sprite(SceneFixture.Solid(2), column * 8, row * 8);
                }
            }
        }

        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        scene.Settings.Shadows = false;
        return scene;
    }

    [GlFact] // AMD-021 R2-010 (REQ-SCN-032) — the criterion
    public void R2_010_a_transparent_receiver_is_the_same_with_and_without_shadows()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        // The synthetic shadow scene of REQ-SCN-030, with the receiving floor at alpha 0.5.
        SceneState scene = ShadowScene(0.5f);
        Shot shadows = target.Render(renderer, scene);
        scene.Settings.Shadows = false;
        Shot none = target.Render(renderer, scene);

        // "une tile réceptrice d'alpha 0,5 a la couleur de la même tile non ombrée": nothing
        // at all changes when the shadows are switched off.
        int differing = Differing(shadows, none);
        Assert.Equal(0, differing);

        // The same scene with an opaque floor does carry the shadow, so the measurement above
        // is not vacuous.
        SceneState opaque = ShadowScene(1f);
        Shot opaqueShadows = target.Render(renderer, opaque);
        opaque.Settings.Shadows = false;
        Shot opaqueNone = target.Render(renderer, opaque);
        Assert.True(
            Differing(opaqueShadows, opaqueNone) > 500,
            "the opaque floor must show a shadow, else R2-010 is not being measured.");

        shadows.SavePng(Path.Combine(FinalCaptures.Ready(), "amd021-r2-010-transparent-floor.png"));
        opaqueShadows.SavePng(Path.Combine(FinalCaptures.Ready(), "amd021-r2-010-opaque-floor.png"));
    }

    /// <summary>The shadow scene of REQ-SCN-030, the floor at <paramref name="alpha"/>.</summary>
    private static SceneState ShadowScene(float alpha)
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Palette(1, 0x30)
            .Palette(2, 0x16)
            .Pattern(SceneFixture.Solid(1, 0), depth: 1, ProfileLayer.L2, alpha: alpha, columns: 16, rows: 10)
            .Pattern(SceneFixture.Solid(1, 1), depth: 8, ProfileLayer.L1, columns: 2, rows: 2)
            .Rect(SceneFixture.Solid(1, 0), column: 8, row: 10, columns: 16, rows: 10, palette: 0)
            .Rect(SceneFixture.Solid(1, 1), column: 14, row: 13, columns: 2, rows: 2, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        scene.Settings.Shadows = true;
        return scene;
    }

    /// <summary>Number of pixels that differ between two shots of the same size.</summary>
    private static int Differing(Shot one, Shot other)
    {
        Assert.Equal(one.Width, other.Width);
        Assert.Equal(one.Height, other.Height);

        int count = 0;
        for (int y = 0; y < one.Height; y++)
        {
            for (int x = 0; x < one.Width; x++)
            {
                if (one.Packed(x, y) != other.Packed(x, y))
                {
                    count++;
                }
            }
        }

        return count;
    }

    // ------------------------------------------------------- the amendment's arithmetic

    /// <summary>The linear colour of the NES index <paramref name="index"/> lit by <paramref name="light"/>.</summary>
    private static (double R, double G, double B) Linear(int index, double light)
    {
        Rgb24 colour = NesPalette.Color(index);
        return (ToLinear(colour.R) * light, ToLinear(colour.G) * light, ToLinear(colour.B) * light);
    }

    /// <summary>
    /// The sRGB result of <c>α × linéaire(objet) + (1 − α) × linéaire(fond)</c>, the formula
    /// R2-009 states, with a grey background of linear value <paramref name="behind"/>.
    /// </summary>
    private static (int R, int G, int B) Blend(int index, double light, double alpha, double behind)
    {
        (double r, double g, double b) = Linear(index, light);
        return (
            ToSrgb((alpha * r) + ((1 - alpha) * behind)),
            ToSrgb((alpha * g) + ((1 - alpha) * behind)),
            ToSrgb((alpha * b) + ((1 - alpha) * behind)));
    }

    private static double ToLinear(int channel)
    {
        double value = channel / 255.0;
        return value <= 0.04045 ? value / 12.92 : Math.Pow((value + 0.055) / 1.055, 2.4);
    }

    private static int ToSrgb(double linear)
    {
        double value = linear <= 0.0031308
            ? 12.92 * linear
            : (1.055 * Math.Pow(linear, 1.0 / 2.4)) - 0.055;
        return (int)Math.Round(Math.Clamp(value, 0, 1) * 255);
    }
}
