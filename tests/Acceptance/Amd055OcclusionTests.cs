using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-055 §2.4 — the screen-space ambient occlusion of REQ-SCN-083, in both its modes: the
/// <b>Depth</b> hemisphere occlusion and the <b>MSVO</b> multi-scale one ported from the
/// MiniEngine, for which the <i>Conformité</i> clause makes the invariants of the section hold
/// too. They are all ratios of linear values between two captures of the <b>close-up A</b> that
/// differ by one setting only.
/// </summary>
/// <remarks>
/// <para>
/// Close-up A: capture 1920 × 1080, <c>video.anti_alias</c> = 0, native resolution, scene
/// scale 1, zero curvature, black <c>Color</c> skybox, <c>render.shadow</c> = false, every other
/// effect of §2.3 to §2.5 off, <c>spl.ao</c> = 2 and the four <c>ao.*</c> at their default
/// (strength 3, radius 3 so <c>r</c> = 20, thickness 3, quality 1); white light (1, −1, 1) of
/// intensity 1. A floor of 12 × 12 full tiles (96 × 96 units) turned by 90° about <c>x</c> and a
/// cube of 8 × 8 × 8 units standing on it, footprint centred on the centre of the floor; the
/// camera 80 units from that centre in the direction (−0.5, +0.5, −0.707), FOV 70°.
/// </para>
/// <para>
/// The engine's layer alignment puts an L1 shape of depth 8 on <c>z</c> ∈ [0, 8]
/// (REQ-E3D-205), so the fixtures pass <c>offset.z</c> = −4 to centre it on <c>C</c> as the
/// close-up describes; the floor's upper face is then the plane <c>y</c> = 124 of normal
/// (0, 1, 0), spanning <c>x</c> ∈ [80, 176] and <c>z</c> ∈ [−48, 48] about (128, 0).
/// </para>
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class Amd055OcclusionTests
{
    private const int Width = 1920;
    private const int Height = 1080;

    /// <summary>The plane of the floor's upper face.</summary>
    private const float Plane = 124f;

    /// <summary>
    /// The slack a ratio measured on an 8-bit sRGB capture needs before it can be compared with
    /// a bound stated in linear, which is what §2.4 does (<b>Q-097</b>, answered: « la tolérance
    /// d'un niveau est confirmée pour tous les invariants de §2.4 »). A pixel exactly at the
    /// ceiling reads a shade under the bound it attains — 0.69850 against 0.70000 under the
    /// ceiling of v1.3 — and one sRGB level covers that and nothing wider.
    /// </summary>
    private const double SrgbStep = 0.01;

    /// <summary>
    /// The darkening ceiling of REQ-SCN-083 as AMD-055 <b>v1.4</b> states it:
    /// <c>O ≤ min(0.95 ; 0.3 × ao.strength)</c>, so 0.3, 0.6, 0.9, 0.95 and 0.95 for the five
    /// settings and 0.9 at the default. The least linear ratio it permits is <c>1 −</c> this.
    /// </summary>
    /// <remarks>
    /// <b>Q-098</b>: the third invariant of §2.4 still restates the ceiling of v1.3 at the two
    /// extreme settings — « `ao.strength` = 5 : le rapport est partout ≥ 0,50 ; `ao.strength` =
    /// 1 : partout ≥ 0,90 », which is exactly <c>1 − 0.1 × ao.strength</c>. It cannot be read
    /// that way beside the second invariant, which v1.4 *did* move and which now brings the
    /// contact of close-up A to 0.10 at the default strength: under the proportionality of
    /// <i>Force</i> the same contact then reads 0.70 at strength 1, under the 0.90 the third
    /// invariant would demand. The tiret is read as what it is, the ceiling restated over the
    /// whole capture rather than at the contact alone, and it follows the ceiling v1.4 states.
    /// </remarks>
    // Q-098
    private static double Ceiling(int strength) => Math.Min(0.95, 0.3 * strength);

    /// <summary>The scene centre <c>C</c> of REQ-E3D-410.</summary>
    private static Vector3 Center => new(128f, 120f, 0f);

    /// <summary>The albedo colour of the close-ups: linear 0.503 ± 0.002.</summary>
    private static Rgb24 Grey => new(188, 188, 188);

    /// <summary>
    /// Two pixels of the floor at the contact of the base of the cube, on the two faces of it
    /// the camera sees: just outside the footprint on the <c>−x</c> side and just in front of it.
    /// </summary>
    private static Vector3[] Contact =>
    [
        new(123.4f, Plane, 0f),
        new(128f, Plane, -4.6f),
    ];

    /// <summary>
    /// A pixel of the floor more than <c>2 r</c> = 40 units from the cube, on the side away from
    /// the camera, and visible in the capture.
    /// </summary>
    private static Vector3 FarFromCube => new(174f, Plane, 0f);

    // =========================================================== the invariants of §2.4 (auto)

    /// <summary>
    /// REQ-SCN-083, first invariant: close-up A <b>without the cube</b> — a lone plane, with no
    /// other geometry — gives a ratio (occlusion On / Off) of 1.00 ± 0.02 at every pixel of the
    /// floor. Nothing occludes a flat surface, and the effect must not darken it by itself.
    /// </summary>
    [GlFact] // REQ-SCN-083
    public void REQ_SCN_083_a_lone_plane_is_not_darkened()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA(cube: false);
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        Enhance(scene, 2);
        Shot on = target.Render(renderer, scene);

        Assert.True(renderer.Statistics.OcclusionPass, "the occlusion pass must have run.");

        // Every pixel of the floor, sampled on a grid of its plane that stays inside the shape.
        int sampled = 0;
        for (float x = 84f; x <= 172f; x += 4f)
        {
            for (float z = -44f; z <= 44f; z += 4f)
            {
                (int px, int py) = Pixel(view, new Vector3(x, Plane, z));
                if (px < 0 || px >= Width || py < 0 || py >= Height || off.At(px, py).R == 0)
                {
                    continue;
                }

                sampled++;
                double ratio = Ratio(on, off, px, py);
                Assert.InRange(ratio, 0.98, 1.02);
            }
        }

        Assert.True(sampled > 100, $"only {sampled} pixels of the lone plane were measured.");
    }

    /// <summary>
    /// REQ-SCN-083, second invariant: close-up A — on the pixels of the floor at the contact of
    /// the base of the cube the ratio is <b>strictly below 1</b> and <b>at least 0.10</b> (the
    /// ceiling <c>O ≤ min(0.95 ; 0.3 × ao.strength)</c> = 0.9 at the default strength); at a
    /// pixel of the floor at least <c>2 r</c> = 40 units from the cube and visible, it is
    /// 1.00 ± 0.02.
    /// </summary>
    /// <remarks>
    /// <b>Q-097</b> (answered, AMD-055 v1.4) moved this bound from 0.70 to <b>0.10</b>: measured
    /// in black box, the reference darkens the darkest 1 % of its Punch-Out!! frame to a linear
    /// ratio of 0.13 at the default settings, which the ceiling of v1.3 did not even permit. The
    /// floor is read with the tolerance of one sRGB level, which the same answer confirms for
    /// every invariant of §2.4: a contact that <em>reaches</em> the ceiling — which the ceiling
    /// being a ceiling must permit — measures a shade under the bound it attains once the
    /// capture has been through the 8-bit sRGB §2.4 itself prescribes, and without the tolerance
    /// the invariant forbids attaining the very bound it states.
    /// </remarks>
    [GlFact] // REQ-SCN-083
    public void REQ_SCN_083_the_foot_of_the_cube_is_darkened_and_nothing_far_from_it_is()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        Enhance(scene, 2);
        Shot on = target.Render(renderer, scene);

        foreach (Vector3 point in Contact)
        {
            (int px, int py) = Pixel(view, point);
            double ratio = Ratio(on, off, px, py);
            Assert.True(ratio < 1.0, $"the contact pixel at {point} was not darkened: {ratio:F3}.");
            Assert.True(ratio >= 0.10 - SrgbStep, $"the contact pixel at {point} fell to {ratio:F5}.");
        }

        (int fx, int fy) = Pixel(view, FarFromCube);
        Assert.True(off.At(fx, fy).R > 0, "the far pixel of the floor must be visible.");
        Assert.InRange(Ratio(on, off, fx, fy), 0.98, 1.02);
    }

    /// <summary>
    /// REQ-SCN-083, third invariant: the ceiling <c>O ≤ min(0.95 ; 0.3 × ao.strength)</c> holds
    /// over the <b>whole capture</b> at every setting — so the ratio stays ≥ 0.70 at
    /// <c>ao.strength</c> = 1 and ≥ 0.05 at 5 — and the ratio at the contact pixel is monotone
    /// decreasing in <c>ao.strength</c>.
    /// </summary>
    /// <remarks>
    /// <b>Q-098</b> on both halves. The bounds follow the ceiling of v1.4, not the 0.90 and 0.50
    /// of v1.3 that the amendment's third tiret still quotes — see <see cref="Ceiling"/>. And
    /// « monotone décroissant » is read at large, the word « strictement » being absent, because
    /// the new ceiling is **constant** between the settings 4 and 5 (0.95 both): under the
    /// « plafond comme échelle » reading that the answer to Q-095 confirmed, those two settings
    /// give identical captures and an equal contact. The strict decrease is still asserted over
    /// the settings 1 to 4, where the ceiling does rise, so nothing of the criterion's
    /// discriminating power is given up by the weaker reading.
    /// </remarks>
    [GlFact] // REQ-SCN-083
    public void REQ_SCN_083_the_strength_is_a_ceiling_and_is_monotone()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        CameraView view = View(scene);
        Shot off = target.Render(renderer, scene);

        Enhance(scene, 2);
        (int cx, int cy) = Pixel(view, Contact[0]);

        double previous = double.MaxValue;
        for (int strength = 1; strength <= 5; strength++)
        {
            scene.Settings.Style.AoStrength = strength;
            Shot on = target.Render(renderer, scene);

            // The ceiling, over the whole capture and not only over the floor.
            double least = LeastRatio(on, off);
            Assert.True(
                least >= 1.0 - Ceiling(strength) - SrgbStep,
                $"strength {strength} darkened a pixel to {least:F3}.");

            double ratio = Ratio(on, off, cx, cy);
            Assert.True(
                strength <= 4 ? ratio < previous : ratio <= previous,
                $"strength {strength} gave {ratio:F3} at the contact, against {previous:F3}.");
            previous = ratio;
        }
    }

    /// <summary>
    /// REQ-SCN-083, fourth invariant, <b>with the MSVO port available</b>: in <c>spl.ao</c> = 1,
    /// changing <c>ao.thickness</c> from 1 to 5 changes at least one pixel by more than 2 sRGB
    /// levels, and the ratio at the contact pixel does <b>not grow</b> — a thicker occluder can
    /// only darken more. In <c>spl.ao</c> = 2 the thickness still changes nothing at all
    /// (REQ-SCN-079, third criterion).
    /// </summary>
    [GlFact] // REQ-SCN-083, REQ-SCN-079
    public void REQ_SCN_083_the_thickness_acts_in_msvo_and_nowhere_else()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        CameraView view = View(scene);
        Shot off = target.Render(renderer, scene);
        (int cx, int cy) = Pixel(view, Contact[0]);

        // MSVO: the thickness acts, and the darkening is monotone in it.
        Enhance(scene, 1);
        scene.Settings.Style.AoThickness = 1;
        Shot thin = target.Render(renderer, scene);
        Assert.True(renderer.Statistics.OcclusionPass, "the MSVO pass must have run.");

        scene.Settings.Style.AoThickness = 5;
        Shot thick = target.Render(renderer, scene);

        int moved = 0;
        for (int i = 0; i < thin.Pixels.Length; i += 4)
        {
            if (Math.Abs(thin.Pixels[i] - thick.Pixels[i]) > 2)
            {
                moved++;
            }
        }

        Assert.True(moved > 0, "ao.thickness changed no pixel by more than 2 sRGB levels.");

        double thinRatio = Ratio(thin, off, cx, cy);
        double thickRatio = Ratio(thick, off, cx, cy);
        Assert.True(
            thickRatio <= thinRatio + 1e-6,
            $"the contact pixel went from {thinRatio:F4} to {thickRatio:F4} as the thickness "
            + "grew: the darkening must not decrease.");

        // Depth: the thickness is not read at all, so the capture is identical to the pixel.
        scene.Settings.Style.AmbientOcclusion = 2;
        scene.Settings.Style.AoThickness = 1;
        Shot depthThin = target.Render(renderer, scene);
        scene.Settings.Style.AoThickness = 5;
        Assert.Equal(depthThin.Pixels, target.Render(renderer, scene).Pixels);
    }

    /// <summary>
    /// REQ-SCN-083, <i>Conformité</i>: the invariants of §2.4 hold for the MSVO mode too — a
    /// lone plane is not darkened, the foot of the cube is, nothing at <c>2 r</c> from it is,
    /// and the ceiling <c>O ≤ min(0.95 ; 0.3 × ao.strength)</c> holds with its monotony — read
    /// as <see cref="REQ_SCN_083_the_strength_is_a_ceiling_and_is_monotone"/> reads it
    /// (<b>Q-098</b>).
    /// </summary>
    [GlFact] // REQ-SCN-083
    public void REQ_SCN_083_the_msvo_mode_keeps_the_invariants_of_the_section()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        // A lone plane: nothing occludes it, so the occlusion must leave it alone.
        SceneState plane = CloseUpA(cube: false);
        CameraView planeView = View(plane);
        Shot planeOff = target.Render(renderer, plane);
        Enhance(plane, 1);
        Shot planeOn = target.Render(renderer, plane);

        int sampled = 0;
        for (float x = 84f; x <= 172f; x += 4f)
        {
            for (float z = -44f; z <= 44f; z += 4f)
            {
                (int px, int py) = Pixel(planeView, new Vector3(x, Plane, z));
                if (px < 0 || px >= Width || py < 0 || py >= Height || planeOff.At(px, py).R == 0)
                {
                    continue;
                }

                sampled++;
                Assert.InRange(Ratio(planeOn, planeOff, px, py), 0.98, 1.02);
            }
        }

        Assert.True(sampled > 100, $"only {sampled} pixels of the lone plane were measured.");

        // The cube: its foot is darkened without passing the ceiling, and the floor far from it
        // is untouched.
        SceneState scene = CloseUpA();
        CameraView view = View(scene);
        Shot off = target.Render(renderer, scene);
        Enhance(scene, 1);

        (int fx, int fy) = Pixel(view, FarFromCube);
        double previous = double.MaxValue;
        for (int strength = 1; strength <= 5; strength++)
        {
            scene.Settings.Style.AoStrength = strength;
            Shot on = target.Render(renderer, scene);

            double least = LeastRatio(on, off);
            Assert.True(
                least >= 1.0 - Ceiling(strength) - SrgbStep,
                $"strength {strength} darkened a pixel to {least:F3}.");
            Assert.InRange(Ratio(on, off, fx, fy), 0.98, 1.02);

            double contact = Contact
                .Select(point => Pixel(view, point))
                .Select(pixel => Ratio(on, off, pixel.X, pixel.Y))
                .Min();
            Assert.True(contact < 1.0, $"strength {strength} darkened no contact pixel.");
            Assert.True(
                contact >= 1.0 - Ceiling(strength) - SrgbStep,
                $"contact fell to {contact:F3}.");
            Assert.True(
                strength <= 4 ? contact < previous : contact <= previous,
                $"strength {strength} gave {contact:F3} at the contact, against {previous:F3}.");
            previous = contact;
        }
    }

    /// <summary>
    /// REQ-SCN-083, <i>Conformité</i>: in the MSVO mode too, <c>spl.ao</c> = 0 is a Classic
    /// capture and neither the skybox nor the HUD is touched.
    /// </summary>
    [GlFact] // REQ-SCN-083, REQ-SCN-029, REQ-SCN-044, REQ-SCN-049
    public void REQ_SCN_083_the_msvo_mode_keeps_the_scope_of_the_section()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA(hud: true, gradient: true);
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        Shot noHud = target.Render(renderer, CloseUpA(hud: false, gradient: true));
        SceneState empty = CloseUpA(cube: false, gradient: true);
        empty.GameRunning = false;
        Shot skyAlone = target.Render(renderer, empty);

        Enhance(scene, 1);
        Shot on = target.Render(renderer, scene);

        Assert.True(renderer.Statistics.OcclusionPass);
        (int cx, int cy) = Pixel(view, Contact[0]);
        Assert.True(Ratio(on, off, cx, cy) < 1.0, "the floor must still be darkened.");

        int hudPixels = 0;
        int skyPixels = 0;
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                if (off.Packed(x, y) != noHud.Packed(x, y))
                {
                    hudPixels++;
                    AssertClose(off, on, x, y, tolerance: 1);
                }
                else if (off.Packed(x, y) == skyAlone.Packed(x, y))
                {
                    skyPixels++;
                    AssertClose(off, on, x, y, tolerance: 1);
                }
            }
        }

        Assert.True(hudPixels > 200, $"only {hudPixels} HUD pixels were compared.");
        Assert.True(skyPixels > 10000, $"only {skyPixels} skybox pixels were compared.");
    }

    /// <summary>
    /// REQ-SCN-083: the three values of <c>ao.quality</c> give the same general aspect in the
    /// MSVO mode too, which is what makes the number of scales and of samples unobservable.
    /// </summary>
    [GlFact] // REQ-SCN-083
    public void REQ_SCN_083_the_three_qualities_agree_in_msvo()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        CameraView view = View(scene);
        Shot off = target.Render(renderer, scene);

        Enhance(scene, 1);
        (int cx, int cy) = Pixel(view, Contact[0]);
        (int fx, int fy) = Pixel(view, FarFromCube);

        List<double> contact = [];
        for (int quality = 0; quality <= 2; quality++)
        {
            scene.Settings.Style.AoQuality = quality;
            Shot on = target.Render(renderer, scene);

            contact.Add(Ratio(on, off, cx, cy));
            Assert.InRange(Ratio(on, off, fx, fy), 0.98, 1.02);
        }

        Assert.InRange(contact.Max() - contact.Min(), 0.0, 0.10);
        Assert.All(contact, ratio => Assert.True(ratio < 1.0, $"quality gave {ratio:F3}."));
    }

    /// <summary>
    /// REQ-SCN-083, fifth invariant: <c>spl.ao</c> = 0 gives a capture identical to a Classic
    /// capture of the same scene, to ±2 sRGB levels (REQ-SCN-080; every other effect is off in
    /// the close-up A).
    /// </summary>
    [GlFact] // REQ-SCN-083, REQ-SCN-080
    public void REQ_SCN_083_the_occlusion_off_is_classic()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        Assert.False(scene.Settings.Style.Enhanced);
        Shot classic = target.Render(renderer, scene);

        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.AmbientOcclusion = 0;
        Shot enhanced = target.Render(renderer, scene);
        Assert.False(renderer.Statistics.OcclusionPass, "spl.ao = 0 must not run the pass.");

        AssertSame(classic, enhanced, tolerance: 2);
    }

    /// <summary>
    /// REQ-SCN-083, sixth invariant — variant A-h: the pixels of the skybox and of the HUD are
    /// identical to ±1 sRGB level between <c>spl.ao</c> = 0 and <c>spl.ao</c> = 2. The skybox is
    /// drawn before the pass and is at infinity, the HUD after it (REQ-SCN-058 amended).
    /// </summary>
    [GlFact] // REQ-SCN-083, REQ-SCN-029, REQ-SCN-044, REQ-SCN-049
    public void REQ_SCN_083_the_skybox_and_the_hud_are_untouched()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA(hud: true);
        scene.Settings.Skybox = SkyboxKind.Gradient;
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);

        // Which pixels are the HUD's: the ones the same scene without the HUD tile does not
        // have. Which are the skybox's: the ones a capture of the skybox alone shares with the
        // full one — REQ-SCN-047 hides the 3D scene outside a game but keeps the skybox.
        Shot noHud = target.Render(renderer, CloseUpA(hud: false, gradient: true));
        SceneState empty = CloseUpA(cube: false, gradient: true);
        empty.GameRunning = false;
        Shot skyAlone = target.Render(renderer, empty);

        Enhance(scene, 2);
        Shot on = target.Render(renderer, scene);

        // The pass did run, and it did change the scene: the test is not vacuous.
        Assert.True(renderer.Statistics.OcclusionPass);
        (int cx, int cy) = Pixel(view, Contact[0]);
        Assert.True(Ratio(on, off, cx, cy) < 1.0, "the floor must still be darkened.");

        int hudPixels = 0;
        int skyPixels = 0;
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                if (off.Packed(x, y) != noHud.Packed(x, y))
                {
                    hudPixels++;
                    AssertClose(off, on, x, y, tolerance: 1);
                }
                else if (off.Packed(x, y) == skyAlone.Packed(x, y))
                {
                    skyPixels++;
                    AssertClose(off, on, x, y, tolerance: 1);
                }
            }
        }

        Assert.True(hudPixels > 200, $"only {hudPixels} HUD pixels were compared.");
        Assert.True(skyPixels > 10000, $"only {skyPixels} skybox pixels were compared.");
    }

    /// <summary>
    /// REQ-SCN-083: the three values of <c>ao.quality</c> give the same general aspect — the
    /// contact of the cube darkened in the same proportion — which is what makes their sample
    /// count and pass resolution unobservable.
    /// </summary>
    [GlFact] // REQ-SCN-083
    public void REQ_SCN_083_the_three_qualities_give_the_same_aspect()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        CameraView view = View(scene);
        Shot off = target.Render(renderer, scene);

        Enhance(scene, 2);
        (int cx, int cy) = Pixel(view, Contact[0]);
        (int fx, int fy) = Pixel(view, FarFromCube);

        List<double> contact = [];
        for (int quality = 0; quality <= 2; quality++)
        {
            scene.Settings.Style.AoQuality = quality;
            Shot on = target.Render(renderer, scene);

            contact.Add(Ratio(on, off, cx, cy));

            // Far from any occluder the three qualities agree on « nothing ».
            Assert.InRange(Ratio(on, off, fx, fy), 0.98, 1.02);
        }

        // The same aspect: the darkening at the contact is within a tenth from one quality to
        // the next. Low may be noisier, which is why the window is not tighter.
        Assert.InRange(contact.Max() - contact.Min(), 0.0, 0.10);
        Assert.All(contact, ratio => Assert.True(ratio < 1.0, $"quality gave {ratio:F3}."));
    }

    /// <summary>
    /// <b>Q-095</b>, <b>Q-097</b> — the magnitude of the darkening at the default settings.
    /// REQ-SCN-083 binds a ceiling and a proportionality, not a law, so none of the invariants
    /// above catches an occlusion that is merely far too weak: a thin outline satisfies every one
    /// of them. This is the regression test of lot V2-22, where the darkening was found four
    /// times weaker than the reference.
    /// </summary>
    /// <remarks>
    /// Measured on the floor of close-up A, away from the cube along <c>−x</c>, the side the
    /// camera is on. The bounds are deliberately loose — they say « a band, not an outline »,
    /// which is the review criterion of §2.4, and leave the law itself free. Lot V2-24 moved them
    /// down with the ceiling of AMD-055 v1.4: this contact reads 0.441 at the calibration of that
    /// lot, against 0.6985 while the ceiling was 0.3 (where it was pinned to the ceiling itself)
    /// and 0.81 at V2-22's constants.
    /// </remarks>
    [GlFact] // REQ-SCN-083
    public void Q095_the_darkening_at_the_default_settings_is_a_band_and_not_an_outline()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        Enhance(scene, 2);
        Shot on = target.Render(renderer, scene);

        // The ceiling is 0.9 at the default ao.strength = 3 (Q-097, AMD-055 v1.4); at the
        // contact the darkening must use a real share of it, not a twentieth. At V2-22's
        // constants this pixel sat at 0.81 — a tenth of the ceiling now in force.
        double contact = Ratio(on, off, view, Contact[0]);
        Assert.InRange(contact, 1.0 - Ceiling(3) - SrgbStep, 0.60);

        // And it must reach out over the radius r = 20, not die within a pixel or two of the
        // contact: at half the radius from the face of the cube the floor is still darkened,
        // and the profile never grows with the distance.
        double previous = contact;
        foreach ((float distance, double ceiling) in Profile)
        {
            double ratio = Ratio(on, off, view, new Vector3(124f - distance, Plane, 0f));
            Assert.True(
                ratio <= ceiling,
                $"{distance} units from the cube the ratio is {ratio:F4}, over {ceiling:F2}.");
            Assert.True(
                ratio >= previous - 0.02,
                $"{distance} units from the cube the ratio {ratio:F4} is below {previous:F4}.");
            previous = ratio;
        }
    }

    /// <summary>
    /// <b>Q-095</b> and <b>Q-097</b> (both answered) — the calibration of lot V2-24, in both
    /// modes. The answer to Q-095 measured the reference and found <em>both</em> modes « environ
    /// trois fois » too weak in mean darkening at comparable area; the answer to Q-097 found the
    /// cause — the reference was never under the ceiling the amendment then stated — and v1.4
    /// raised that ceiling from <c>0.1 × ao.strength</c> to
    /// <c>min(0.95 ; 0.3 × ao.strength)</c>. Neither the resulting magnitude nor the relation
    /// between the two modes is pinned by any invariant of §2.4 — a ceiling and a monotony leave
    /// both free — so this test holds the two constants that carry them.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Measured on the contact of close-up A and on the area the effect changes at all, the two
    /// quantities the answer's own method uses. The windows are wide: they fix the order of
    /// magnitude the reference shows, not a law, which REQ-SCN-083 leaves free.
    /// </para>
    /// <para>
    /// What fails here is a regression to either of the two calibrations this pair of tickets
    /// passed through: V2-22's constants left the Depth contact at 0.81, and V2-23's pinned both
    /// modes to the old ceiling at 0.6985 — both above the 0.50 asserted below. The *ratio*
    /// between the two modes' magnitudes, which is what `zaxMsvoShare` was finally calibrated on
    /// (1.355 against the reference's 1.37 in mean darkening), belongs to the ring of
    /// « Punch-Out!! » and is recorded in docs/progress/V2-24.md: this lone cube on a plane
    /// cannot carry it, and the area assertion below deliberately does not state which mode is
    /// the wider.
    /// </para>
    /// </remarks>
    [GlFact] // REQ-SCN-083
    public void Q095_both_modes_carry_the_magnitude_the_reference_shows()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpA();
        CameraView view = View(scene);
        Shot off = target.Render(renderer, scene);
        (int cx, int cy) = Pixel(view, Contact[0]);

        Enhance(scene, 2);
        Shot depth = target.Render(renderer, scene);
        Enhance(scene, 1);
        Shot msvo = target.Render(renderer, scene);

        double depthContact = Ratio(depth, off, cx, cy);
        double msvoContact = Ratio(msvo, off, cx, cy);
        int depthArea = Changed(depth, off);
        int msvoArea = Changed(msvo, off);

        // Both modes darken the deepest contact of the close-up by more than half in linear —
        // well past the 0.70 that was the whole of the ceiling until AMD-055 v1.4, and inside
        // the 0.10 that ceiling now permits (Q-097). At V2-22's constants the Depth contact sat
        // at 0.81, and at V2-23's it was pinned to the old ceiling at 0.6985.
        Assert.InRange(depthContact, 1.0 - Ceiling(3) - SrgbStep, 0.50);
        Assert.InRange(msvoContact, 1.0 - Ceiling(3) - SrgbStep, 0.50);

        // And both lay a band rather than an outline — the review criterion of §2.4 — over
        // comparable ground. Which of the two is the wider is not pinned here: the reference has
        // MSVO the wider on the ring (32.2 % against 23.3 % of the frame), where the port
        // measures the two level (32.20 % against 32.86 %), and on this lone cube on a plane it
        // is Depth that spreads further. What a regression of either gain would break is the
        // order of magnitude, which is what the bounds hold.
        Assert.True(depthArea > Width * Height / 100, $"Depth changed only {depthArea} pixels.");
        Assert.True(msvoArea > Width * Height / 100, $"MSVO changed only {msvoArea} pixels.");
        Assert.InRange(msvoArea / (double)depthArea, 0.4, 2.5);
    }

    /// <summary>How many pixels the effect changes at all, by at least one sRGB level.</summary>
    private static int Changed(Shot on, Shot off)
    {
        int changed = 0;
        for (int i = 0; i < off.Pixels.Length; i += 4)
        {
            if (Math.Abs(on.Pixels[i] - off.Pixels[i]) > 0
                || Math.Abs(on.Pixels[i + 1] - off.Pixels[i + 1]) > 0
                || Math.Abs(on.Pixels[i + 2] - off.Pixels[i + 2]) > 0)
            {
                changed++;
            }
        }

        return changed;
    }

    /// <summary>
    /// How far the darkening of close-up A must still reach along <c>−x</c>: the distance from
    /// the <c>−x</c> face of the cube in scene units, and the ratio the floor must stay under
    /// there. <c>r</c> = 20 at the default <c>ao.radius</c> = 3.
    /// </summary>
    /// <remarks>
    /// Tightened in lot V2-24 with the ceiling of AMD-055 v1.4: measured 0.646, 0.884 and 0.961
    /// at the three distances, against the 0.93 / 0.97 / 0.99 the bounds held while the ceiling
    /// was 0.3. They stay a factor clear of the measurement so that the test says « a band that
    /// reaches out over <c>r</c> » and not « this exact profile », the law being free.
    /// </remarks>
    private static (float Distance, double Ceiling)[] Profile =>
    [
        (4f, 0.78),
        (10f, 0.93),
        (16f, 0.98),
    ];

    /// <summary>
    /// REQ-SCN-083, scope: the transparent objects of REQ-SCN-040, drawn after the pass, are
    /// neither darkened by the occlusion nor occluders for it (REQ-SCN-058 amended).
    /// </summary>
    [GlFact] // REQ-SCN-083, REQ-SCN-040, REQ-SCN-058
    public void REQ_SCN_083_a_transparent_object_neither_occludes_nor_is_darkened()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        // The same close-up with the cube made translucent: it is drawn after the pass, so it
        // writes no depth the occlusion can see and the floor under it stays untouched.
        SceneState opaque = CloseUpA();
        CameraView view = View(opaque);
        (int cx, int cy) = Pixel(view, Contact[0]);

        SceneState scene = CloseUpA(cubeAlpha: 0.5f);
        Shot off = target.Render(renderer, scene);
        Enhance(scene, 2);
        Shot on = target.Render(renderer, scene);

        Assert.True(renderer.Statistics.OcclusionPass);
        Assert.InRange(Ratio(on, off, cx, cy), 0.98, 1.02);
    }

    // ================================================================================= fixtures

    /// <summary>
    /// The close-up A: the floor of 12 × 12 full tiles, the cube of 8 × 8 × 8 units standing on
    /// it, and the camera the amendment places.
    /// </summary>
    /// <param name="cube">Whether the cube is in the scene (the first invariant drops it).</param>
    /// <param name="hud">Whether a HUD tile is drawn at the centre (variant A-h).</param>
    /// <param name="cubeAlpha">Opacity of the cube, 1 by default.</param>
    /// <param name="gradient">Whether the Gradient skybox of the variant A-h is drawn.</param>
    private static SceneState CloseUpA(
        bool cube = true,
        bool hud = false,
        float cubeAlpha = 1f,
        bool gradient = false)
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)

            // The floor: 12 × 12 full tiles, turned by 90° about x, centred on C.
            .Pattern(
                SceneFixture.Solid(1),
                depth: 8,
                ProfileLayer.L1,
                columns: 12,
                rows: 12,
                texture: SceneFixture.FlatDirectTexture(Grey, 12, 12),
                rotation: new Vec3F(90f, 0f, 0f),
                offset: new Vec3F(0f, 0f, -4f))
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 10, 10, 12, 12, palette: 1);

        if (cube)
        {
            // The cube: one full tile of sizeZ = 8, its tile record placed clear of the floor
            // block so that the segmentation keeps two shapes, then brought by its offset onto
            // the centre of the floor with its lower face on the plane y = 124.
            fixture
                .Pattern(
                    SceneFixture.Solid(1, 1),
                    depth: 8,
                    ProfileLayer.L1,
                    alpha: cubeAlpha,
                    texture: SceneFixture.FlatDirectTexture(Grey),
                    offset: new Vec3F(-68f, 4f, -4f))
                .Palette(5, 0x30)
                .Tile(SceneFixture.Solid(1, 1), 192, 120, palette: 1);
        }

        if (hud)
        {
            // A HUD motif of the variant A-h, on the cell row 8 — above the floor block, whose
            // cells are the rows 10 to 21, so that no cell carries two tile records.
            fixture
                .Pattern(SceneFixture.Solid(1, 2), depth: 1, ProfileLayer.Ui, columns: 2, rows: 2)
                .Rect(SceneFixture.Solid(1, 2), 15, 8, 2, 2, palette: 1);
        }

        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        scene.Settings.AntiAliasSamples = 1;
        scene.Settings.Shadows = false;
        scene.Settings.Skybox = gradient ? SkyboxKind.Gradient : SkyboxKind.Color;
        scene.Settings.CustomBackgroundColor = default;
        scene.Settings.SceneScale = 1f;
        scene.Settings.Fov = 70f;
        scene.Camera.Fov = 70f;
        scene.Light.Direction = new Vector3(1f, -1f, 1f);
        scene.Light.Color = new Rgb24(255, 255, 255);
        scene.Settings.LightIntensity = 1f;

        // The camera 80 units from the centre of the floor in the direction
        // (−0.5, +0.5, −0.707): elevation 30°, azimuth 35.264°, so that
        // q·(0, 0, 1) = (0.5, −0.5, 0.707).
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Target = Center;
        scene.Camera.Current.Distance = 80f;
        scene.Camera.Current.Angles = new Vector3(30f, 35.26439f, 0f);
        scene.Camera.SnapToSetpoints();
        return scene;
    }

    // ================================================================================== measures

    /// <summary>
    /// Turns the Enhanced style on with the occlusion mode given. The reference capture of every
    /// invariant is taken in Classic, which REQ-SCN-080 makes identical to Enhanced with every
    /// effect off.
    /// </summary>
    private static void Enhance(SceneState scene, int mode)
    {
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.AmbientOcclusion = mode;
    }

    private static CameraView View(SceneState scene)
        => scene.Camera.BuildView(scene.Clipping, (float)Width / Height);

    private static (int X, int Y) Pixel(in CameraView view, Vector3 point)
    {
        Vector4 clip = Vector4.Transform(
            new Vector3(point.X, point.Y, -point.Z), view.ViewProjection);
        float x = clip.X / clip.W;
        float y = clip.Y / clip.W;
        return (
            (int)Math.Round((x + 1f) * 0.5f * Width),
            (int)Math.Round((1f - y) * 0.5f * Height));
    }

    /// <summary>The ratio of the linear green values of two captures at a scene point.</summary>
    private static double Ratio(Shot on, Shot off, in CameraView view, Vector3 point)
    {
        (int x, int y) = Pixel(view, point);
        return Ratio(on, off, x, y);
    }

    /// <summary>The ratio of the linear green values of two captures at one pixel.</summary>
    private static double Ratio(Shot on, Shot off, int x, int y)
    {
        Assert.InRange(x, 0, Width - 1);
        Assert.InRange(y, 0, Height - 1);

        double below = Linear(off.At(x, y).G);
        Assert.True(below > 0, $"({x}, {y}) is black without the occlusion.");
        return Linear(on.At(x, y).G) / below;
    }

    /// <summary>The smallest ratio of linear values over the whole capture.</summary>
    private static double LeastRatio(Shot on, Shot off)
    {
        double least = double.MaxValue;
        for (int i = 0; i < off.Pixels.Length; i += 4)
        {
            int below = off.Pixels[i + 1];
            if (below < 8)
            {
                // A pixel that is black or nearly black carries no measurable ratio.
                continue;
            }

            least = Math.Min(least, Linear(on.Pixels[i + 1]) / Linear(below));
        }

        return least == double.MaxValue ? 1.0 : least;
    }

    /// <summary>The linear value of an sRGB 8-bit level (SPEC-06 §2).</summary>
    private static double Linear(int level)
    {
        double c = level / 255.0;
        return c <= 0.04045 ? c / 12.92 : Math.Pow((c + 0.055) / 1.055, 2.4);
    }

    private static void AssertClose(Shot one, Shot two, int x, int y, int tolerance)
    {
        (int r1, int g1, int b1) = one.At(x, y);
        (int r2, int g2, int b2) = two.At(x, y);
        if (Math.Abs(r1 - r2) > tolerance
            || Math.Abs(g1 - g2) > tolerance
            || Math.Abs(b1 - b2) > tolerance)
        {
            Assert.Fail(
                $"({x}, {y}): ({r1}, {g1}, {b1}) against ({r2}, {g2}, {b2}), "
                + $"more than {tolerance} levels apart.");
        }
    }

    private static void AssertSame(Shot one, Shot two, int tolerance)
    {
        for (int y = 0; y < one.Height; y++)
        {
            for (int x = 0; x < one.Width; x++)
            {
                AssertClose(one, two, x, y, tolerance);
            }
        }
    }
}
