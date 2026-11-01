using System.Globalization;
using System.Numerics;
using Xunit.Abstractions;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-055 §2.4 and §2.5 — the two figures the black-box verification of lot V2-25 measured
/// against 3dSen 2.0, which neither the automatic invariants of
/// <see cref="Amd055ReflectionTests"/> nor those of <see cref="Amd055OcclusionTests"/> constrain:
/// the <b>magnitude</b> the floor reflections really add at the default settings, and the share
/// of the MSVO darkening that comes from the <b>large scales</b> of its pyramid.
/// </summary>
/// <remarks>
/// <para>
/// Both requirements are P3 and judged by review (SPEC-12), so neither figure has a normative
/// value; what the two tests below hold is a <b>regression bound</b> around the behaviour lot
/// V2-25 established, anchored at each end so that it can be broken in either direction:
/// </para>
/// <list type="bullet">
/// <item>the reflection must stay <em>visible</em> — the first invariant of §2.5, the red of the
/// cube at the contact — and must stay <em>within scale</em>, the magnitude ceiling the
/// verification's reference figure implies;</item>
/// <item>the MSVO darkening must keep a <em>wide</em> component — a floor band a radius away from
/// the only occluder of the scene, which the coarsest levels of the pyramid alone can reach — and
/// must keep its far field <em>clean</em>, the <c>2 r</c> invariant of §2.4.</item>
/// </list>
/// <para>
/// The geometry is that of the <b>close-up R</b> and of the <b>close-up A</b> the amendment
/// describes, rebuilt here rather than shared, so that the two files stay readable on their own.
/// </para>
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class Amd055StyleStrengthTests
{
    private const int Width = 1920;
    private const int Height = 1080;

    /// <summary>The plane of the upper face of both floors.</summary>
    private const float Plane = 124f;

    /// <summary>
    /// The occlusion radius <c>r</c> of REQ-SCN-083 at the default <c>ao.radius</c> = 3:
    /// <c>8 + 6 × 2</c> scene units, the scene scale being 1 in both close-ups.
    /// </summary>
    private const float Radius = 20f;

    private readonly ITestOutputHelper _output;

    public Amd055StyleStrengthTests(ITestOutputHelper output) => _output = output;

    /// <summary>The scene centre <c>C</c> of REQ-E3D-410.</summary>
    private static Vector3 Center => new(128f, 120f, 0f);

    /// <summary>The dark floor of the close-up R: linear albedo 0.202.</summary>
    private static Rgb24 Dark => new(124, 124, 124);

    /// <summary>The grey of the close-up A: linear albedo 0.503.</summary>
    private static Rgb24 Grey => new(188, 188, 188);

    /// <summary>The pure red of the reflected cube of the close-up R.</summary>
    private static Rgb24 Red => new(255, 0, 0);

    /// <summary>
    /// Two pixels of the floor of the close-up R at the contact of the base of the cube, just
    /// outside its footprint <c>x</c> ∈ [124, 132], <c>z</c> ∈ [−4, 4] on the two sides the
    /// camera sees (the points <see cref="Amd055ReflectionTests"/> reads the first invariant at).
    /// </summary>
    private static Vector3[] Contact =>
    [
        new(123.4f, Plane, 0f),
        new(128f, Plane, -4.6f),
    ];

    /// <summary>
    /// REQ-SCN-084 — <b>close-up R at the default settings</b>: the magnitude of what the
    /// reflections change.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Measured the way the verification of V2-25 measures it: the <b>zone</b> is the set of
    /// pixels whose sum over the three channels of the absolute change in sRGB levels exceeds
    /// <b>12</b>, and the figure is the <b>mean of that sum over the zone</b> — 63 for the
    /// reference against 122 for this reimplementation on the scene the verification used.
    /// Q-099 traced the factor of two to the intensity constant itself, which AMD-055 v1.5
    /// halves to <c>0,045 × n(ssr.shine)</c>.
    /// </para>
    /// <para>
    /// The bound is set on the close-up R, whose geometry is normative and third-party
    /// reproducible, and not on that scene: a pure red cube one unit from a dark floor is a far
    /// harsher case than a game frame, so the figure is larger here and only the fixture's own
    /// history gives it a scale. It is held at both ends — a reflection that disappeared would
    /// pass the ceiling and fail the floor, which is the first invariant of §2.5 restated.
    /// </para>
    /// </remarks>
    [GlFact] // REQ-SCN-084
    public void REQ_SCN_084_the_reflection_magnitude_at_the_defaults_stays_in_scale()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpR();
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.Reflections = true;
        Shot on = target.Render(renderer, scene);

        Assert.True(renderer.Statistics.ReflectionPass, "the reflection pass must have run.");

        (double share, double mean, double signed) = Zone(off, on);

        // The realised mix weight. The cube is pure red, so the reflected colour has no green at
        // all and a floor pixel's green comes out at exactly (1 − i) of what it was: the mix
        // « (1 − i) · surface + i · reflet » of REQ-SCN-084 read straight off the capture, with
        // no need to know what the ray hit. Applied twice it would read 1 − (1 − i)², 0.40.
        double[] realised = new double[Contact.Length];
        for (int i = 0; i < Contact.Length; i++)
        {
            (int px, int py) = Pixel(view, Contact[i]);
            realised[i] = 1.0 - (Linear(on.At(px, py).G) / Linear(off.At(px, py).G));
        }

        Report(
            "close-up R, reflections at the defaults",
            ("zone, share of the frame", share * 100.0, "%"),
            ("in-zone mean of the summed absolute change", mean, "levels"),
            ("in-zone signed mean", signed, "levels"),
            ("realised mix weight at the first contact pixel", realised[0], string.Empty),
            ("realised mix weight at the second", realised[1], string.Empty));

        // The floor: the effect is still there at all, and still carries the cube's red at the
        // contact (§2.5, first invariant), so the ceiling cannot be met by removing it.
        Assert.True(share > 0.005, $"the reflection only covers {share * 100.0:F2} % of the frame.");
        (int cx, int cy) = Pixel(view, Contact[0]);
        Assert.True(
            Linear(on.At(cx, cy).R) >= Linear(off.At(cx, cy).R) * 1.10,
            "the floor must still take the red of the cube at the contact.");

        // The intensity itself, which is the one normative figure of the three: `i` must be
        // « 0,045 × n(ssr.shine) » = 0.225 at the default shine (AMD-055 v1.5, Q-099), and this
        // reads it off the capture with no reference to what the ray hit. Half an sRGB level at
        // these values is under 0.004, so 0.015 is four times the quantisation and still far
        // from any of the ways of getting it wrong — 0.40 for a double application of the v1.4
        // weight, 0.45 for the v1.4 constant itself, 0.135 for `0,045 × ssr.shine`.
        foreach (double weight in realised)
        {
            Assert.InRange(weight, 0.210, 0.240);
        }

        // The magnitude. V2-25 measured 116.1 in-zone on this fixture at the v1.4 intensity,
        // where R5's reference implied about half of that; v1.5 halves the constant and V2-26
        // measures 59.9 here. The bound is a regression guard, not the reference's own figure
        // (this is a pure red cube one unit from a dark floor, not a lit ring on a grey mat):
        // 70 catches a return to the v1.4 constant and leaves a fifth of margin.
        Assert.True(
            mean < 70.0,
            $"the in-zone mean change is {mean:F1} summed sRGB levels, where V2-26 measured "
            + "59.9 — something is adding reflection beyond REQ-SCN-084's intensity (Q-099).");

        // The change is a brightening on balance. On this fixture the two halves of the mix very
        // nearly cancel in the sum — the red channel rises by what the cube reflects while the
        // green and the blue fall by the (1 − i) the mix leaves the surface — so the figure is
        // small and only its sign is held; the verification's own scene, a lit ring on a grey
        // mat, reads it as the +38 against +73 of the lot's statement.
        Assert.True(signed > 0.0, $"the reflection darkens the frame on balance ({signed:F1}).");
    }

    /// <summary>
    /// REQ-SCN-083, <b>mode MSVO</b> — the <b>large-scale</b> component of the multi-scale
    /// occlusion: the floor of <see cref="KerbOnFloor"/> is darkened well away from the kerb's
    /// contact, not in a band along it alone.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The scene holds exactly one occluder, so the distance from it selects which scales of the
    /// pyramid can have produced a darkening at all: within a quarter of <c>r</c> every level
    /// contributes, while the band from <c>0.25 r</c> to <c>0.75 r</c> is out of reach of every
    /// level but the two coarsest. That the mid-field is theirs is measured, not assumed: with
    /// the two coarse levels alone the band 8 to 12 units reads 5.54 against the full pyramid's
    /// 5.47 (<b>Q-100</b>).
    /// </para>
    /// <para>
    /// The figure held is the <b>ratio of the mid band's darkening to the contact band's</b>,
    /// which is free of the absolute calibration of the mode (the gain and the ceiling of
    /// <b>Q-097</b>) and of the albedo of the scene, and the same ratio for the single-scale
    /// <b>Depth</b> mode beside it — the one anchor of the verification that no framing can
    /// disturb, since both modes are read off the same capture. The far field is held too: beyond
    /// <c>2 r</c> the second invariant of §2.4 wants nothing at all, so a wide band cannot be had
    /// by letting the darkening run on for ever.
    /// </para>
    /// <para>
    /// What these bounds are <em>not</em>: a statement that the mode is as wide as the reference.
    /// It is not — <b>Q-100</b> measures the gap and establishes that its cause is the
    /// sensitivity of the ported estimator, which compares a sample's depth to the centre pixel's
    /// rather than to the tangent plane of the surface, and therefore cannot be closed without
    /// changing the estimator REQ-SCN-083 names. These bounds guard the four departures from the
    /// original that lot V2-25 did fix.
    /// </para>
    /// </remarks>
    [GlFact] // REQ-SCN-083
    public void REQ_SCN_083_the_msvo_mode_darkens_at_the_large_scale()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = KerbOnFloor();
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.AmbientOcclusion = 2;
        Shot hemisphere = target.Render(renderer, scene);
        scene.Settings.Style.AmbientOcclusion = 1;
        Shot multiScale = target.Render(renderer, scene);

        Assert.True(renderer.Statistics.OcclusionPass, "the MSVO pass must have run.");

        double[] depth = FloorProfile(off, hemisphere, view);
        double[] msvo = FloorProfile(off, multiScale, view);
        ReportProfile("kerb on floor, Depth — darkening by distance in front of the kerb", depth);
        ReportProfile("kerb on floor, MSVO — darkening by distance in front of the kerb", msvo);

        double contact = Band(msvo, 0f, 0.25f * Radius);
        double depthMid = Band(depth, 0.25f * Radius, 0.75f * Radius)
            / Band(depth, 0f, 0.25f * Radius);
        double msvoMid = Band(msvo, 0.25f * Radius, 0.75f * Radius) / contact;
        double far = Band(msvo, 2f * Radius, 2.2f * Radius);

        Report(
            "kerb on floor, at the default ao.*",
            ("MSVO: contact band (< 0.25 r)", contact, "levels"),
            ("MSVO: mid band (0.25 r to 0.75 r) / contact", msvoMid, string.Empty),
            ("Depth: mid band / contact", depthMid, string.Empty),
            ("MSVO / Depth", msvoMid / depthMid, string.Empty),
            ("MSVO beyond 2 r", far, "levels"));

        // The contact is still there: the mid band is held as a *share* of it below, so a mode
        // that had stopped darkening at all would otherwise pass.
        Assert.True(contact > 20.0, $"the MSVO contact band darkens only {contact:F1} levels.");

        // The mid field, which is the coarse levels' alone (Q-100 measures that: the two coarse
        // levels on their own give the band 8 to 12 units 5.54 against the full pyramid's 5.47).
        // V2-25 measures 0.191 here; 0.12 leaves a third of margin and is what a pyramid that
        // had lost its coarse scales — the state lot V2-24 found for the Low quality, where
        // dropping the coarsest level halved the darkening — would fall under.
        Assert.True(
            msvoMid > 0.12,
            $"the MSVO mid band is {msvoMid:F3} of its contact band, where V2-25 measured 0.191: "
            + "the coarse levels of the pyramid have stopped contributing, so REQ-SCN-083's "
            + "multi-scale estimate is one in name only.");

        // ... and it does not narrow further against the single-scale estimator read off the same
        // capture. V2-25 measures 0.466 on this band (0.191 against the Depth mode's 0.410).
        // This is deliberately a *lower* bound and not the reference's 1.38: Q-100 establishes
        // that the figure cannot reach 1.38 without changing the estimator REQ-SCN-083 names, and
        // 0.30 is what guards against going back.
        Assert.True(
            msvoMid / depthMid > 0.30,
            $"the MSVO profile falls off at {msvoMid:F3} of its contact against the Depth mode's "
            + $"{depthMid:F3} on the same capture, a factor of {msvoMid / depthMid:F2} where "
            + "V2-25 measured 0.47 (Q-100).");

        // The far field stays clean: 2 r away from the only occluder, nothing (§2.4, second
        // invariant, which is what bounds the coarsest scale from above).
        Assert.True(
            far < 1.0,
            $"the floor beyond 2 r is darkened by {far:F2} levels, which REQ-SCN-083 forbids.");
    }

    // ================================================================================== measures

    /// <summary>
    /// The zone of a pair of captures and its two means: the share of the frame whose summed
    /// absolute change over the three channels exceeds 12 sRGB levels, the mean of that sum over
    /// the zone, and the mean of the signed sum over the same pixels.
    /// </summary>
    private static (double Share, double Mean, double Signed) Zone(Shot off, Shot on)
    {
        long inside = 0;
        double absolute = 0.0;
        double signed = 0.0;
        for (int y = 0; y < off.Height; y++)
        {
            for (int x = 0; x < off.Width; x++)
            {
                (int r1, int g1, int b1) = off.At(x, y);
                (int r2, int g2, int b2) = on.At(x, y);
                int sum = Math.Abs(r2 - r1) + Math.Abs(g2 - g1) + Math.Abs(b2 - b1);
                if (sum <= 12)
                {
                    continue;
                }

                inside++;
                absolute += sum;
                signed += (r2 - r1) + (g2 - g1) + (b2 - b1);
            }
        }

        Assert.True(inside > 0, "the effect changed nothing at all.");
        double pixels = (double)off.Width * off.Height;
        return (inside / pixels, absolute / inside, signed / inside);
    }

    /// <summary>Width of one band of <see cref="FloorProfile"/>, in scene units.</summary>
    private const float BandWidth = 4f;

    /// <summary>Bands of <see cref="FloorProfile"/>: 0 to 44 scene units from the kerb.</summary>
    private const int Bands = 11;

    /// <summary>
    /// The mean darkening of the floor of the close-up A, in sRGB levels of the green channel,
    /// band by band of <see cref="BandWidth"/> scene units of distance from the cube's footprint
    /// in the <c>xz</c> plane — the profile of the occlusion around the scene's only occluder,
    /// and therefore the shape the scales of an estimator give it.
    /// </summary>
    /// <remarks>
    /// The floor is sampled on a grid of scene points rather than by walking the capture, so that
    /// the distance to the occluder is known exactly; the cube's own screen box is left out, with
    /// a margin, because the floor behind it is hidden and its pixels are the cube's.
    /// </remarks>
    private static double[] FloorProfile(Shot off, Shot on, in CameraView view)
    {
        double[] total = new double[Bands];
        int[] count = new int[Bands];

        // Across the middle of the kerb, so that the two ends of it are far away, and only on
        // the camera's side of it, where every floor point is visible — the floor behind it is
        // hidden by it and those pixels would be the kerb's.
        for (float x = 108.5f; x <= 147.5f; x += 0.5f)
        {
            for (float z = -4.5f; z >= -47.5f; z -= 0.5f)
            {
                int band = (int)((-4f - z) / BandWidth);
                if (band >= Bands)
                {
                    continue;
                }

                (int px, int py) = Pixel(view, new Vector3(x, Plane, z));
                if (px < 0 || px >= Width || py < 0 || py >= Height)
                {
                    continue;
                }

                total[band] += off.At(px, py).G - on.At(px, py).G;
                count[band]++;
            }
        }

        double[] profile = new double[Bands];
        for (int band = 0; band < Bands; band++)
        {
            Assert.True(count[band] > 100, $"band {band} holds only {count[band]} floor points.");
            profile[band] = total[band] / count[band];
        }

        return profile;
    }

    /// <summary>
    /// The mean of the bands of a profile that fall in <c>[near, far)</c> scene units, each band
    /// weighted equally — the bounds are multiples of <see cref="BandWidth"/> by construction.
    /// </summary>
    private static double Band(double[] profile, float near, float far)
    {
        double total = 0.0;
        int count = 0;
        for (int band = (int)(near / BandWidth); band < (int)(far / BandWidth); band++)
        {
            total += profile[band];
            count++;
        }

        return total / count;
    }

    /// <summary>Writes a profile band by band, under its title.</summary>
    private void ReportProfile(string title, double[] profile)
    {
        _output.WriteLine(title);
        for (int band = 0; band < profile.Length; band++)
        {
            _output.WriteLine(string.Create(
                CultureInfo.InvariantCulture,
                $"  {band * BandWidth,3}-{(band + 1) * BandWidth,-3} units: {profile[band]:F3}"));
        }
    }

    /// <summary>Writes one measured figure per line, for the progress file of the lot.</summary>
    private void Report(string title, params (string Name, double Value, string Unit)[] figures)
    {
        _output.WriteLine(title);
        foreach ((string name, double value, string unit) in figures)
        {
            _output.WriteLine(string.Create(
                CultureInfo.InvariantCulture, $"  {name}: {value:F3} {unit}"));
        }
    }

    // ================================================================================= fixtures

    /// <summary>
    /// The close-up R of §2.5: the dark floor of 8 × 8 full tiles turned by 90° about <c>x</c>,
    /// and the red cube of 8 × 8 × 8 units standing on it.
    /// </summary>
    private static SceneState CloseUpR()
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(
                SceneFixture.Solid(1),
                depth: 8,
                ProfileLayer.L1,
                columns: 8,
                rows: 8,
                texture: SceneFixture.FlatDirectTexture(Dark, 8, 8),
                rotation: new Vec3F(90f, 0f, 0f),
                offset: new Vec3F(0f, 0f, -4f))
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 12, 12, 8, 8, palette: 1)
            .Pattern(
                SceneFixture.Solid(1, 1),
                depth: 8,
                ProfileLayer.L1,
                texture: SceneFixture.FlatDirectTexture(Red),
                offset: new Vec3F(-68f, 4f, -4f))
            .Palette(5, 0x30)
            .Tile(SceneFixture.Solid(1, 1), 192, 120, palette: 1);

        return Finish(fixture, distance: 40f);
    }

    /// <summary>
    /// The close-up A of §2.4: the grey floor of 12 × 12 full tiles (96 × 96 units) turned by 90°
    /// about <c>x</c> and centred on <c>C</c>, upper face the plane <c>y</c> = 124, and the grey
    /// cube of 8 × 8 × 8 units standing on its centre.
    /// </summary>
    private static SceneState CloseUpA()
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
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
            .Rect(SceneFixture.Solid(1), 10, 10, 12, 12, palette: 1)

            // The cube: one full tile of sizeZ = 8, its record placed clear of the floor block so
            // that the segmentation keeps two shapes, then brought by its offset onto the centre
            // of the floor with its lower face on the plane y = 124.
            .Pattern(
                SceneFixture.Solid(1, 1),
                depth: 8,
                ProfileLayer.L1,
                texture: SceneFixture.FlatDirectTexture(Grey),
                offset: new Vec3F(-68f, 4f, -4f))
            .Palette(5, 0x30)
            .Tile(SceneFixture.Solid(1, 1), 192, 120, palette: 1);

        return Finish(fixture, distance: 80f);
    }

    /// <summary>
    /// The close-up A with a <b>laterally large</b> occluder: the same floor and the same camera,
    /// and in place of the single cube a <b>kerb</b> of 8 × 1 of the close-up's own cube tile —
    /// 64 × 8 × 8 units, <c>x</c> ∈ [96, 160], <c>y</c> ∈ [124, 132], <c>z</c> ∈ [−4, 4] —
    /// standing on the floor right across the middle of it.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The wide half of a multi-scale estimate needs something wide to see. The pattern of the
    /// MiniEngine port is a seven-entry checkerboard scaled by the sphere radius of its level, so
    /// a coarse level samples sparsely in absolute terms and a <em>small</em> occluder falls
    /// between its samples: on the close-up A, whose only occluder is an 8-unit cube, a coarse
    /// scale can barely register it however the pyramid is built, and the mode's large scale is
    /// not measurable there at all.
    /// </para>
    /// <para>
    /// The kerb is <b>64 units long and 8 tall</b>: long enough that every sample of a coarse
    /// level taken along it meets it, and no taller than the close-up's own cube, so the camera —
    /// 36 units above the floor — keeps its view of the floor on both sides and the darkening's
    /// reach can be read off it. It is built from eight records of the <em>same</em> 1 × 1
    /// pattern the cube uses, which the segmentation joins into one shape, so the geometry is the
    /// close-up's own, eight cells long.
    /// </para>
    /// </remarks>
    private static SceneState KerbOnFloor()
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
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
            .Rect(SceneFixture.Solid(1), 10, 10, 12, 12, palette: 1)
            .Pattern(
                SceneFixture.Solid(1, 1),
                depth: 8,
                ProfileLayer.L1,
                texture: SceneFixture.FlatDirectTexture(Grey),
                offset: new Vec3F(-80f, 4f, -4f))
            .Palette(5, 0x30)
            .Rect(SceneFixture.Solid(1, 1), 22, 15, 8, 1, palette: 1);

        return Finish(fixture, distance: 80f);
    }

    /// <summary>The settings, the light and the camera the two close-ups share.</summary>
    private static SceneState Finish(SceneFixture fixture, float distance)
    {
        SceneState scene = fixture.Build();
        scene.Palette(fixture);

        scene.Settings.AntiAliasSamples = 1;
        scene.Settings.Shadows = false;
        scene.Settings.Skybox = SkyboxKind.Color;
        scene.Settings.CustomBackgroundColor = default;
        scene.Settings.SceneScale = 1f;
        scene.Settings.Fov = 70f;
        scene.Camera.Fov = 70f;
        scene.Light.Direction = new Vector3(1f, -1f, 1f);
        scene.Light.Color = new Rgb24(255, 255, 255);
        scene.Settings.LightIntensity = 1f;

        // The direction (−0.5, +0.5, −0.707) from the centre of the floor: elevation 30°,
        // azimuth 35.264°.
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Target = Center;
        scene.Camera.Current.Distance = distance;
        scene.Camera.Current.Angles = new Vector3(30f, 35.26439f, 0f);
        scene.Camera.SnapToSetpoints();
        return scene;
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

    /// <summary>The screen bounding box of the eight corners of a scene-space box.</summary>
    private static (int Left, int Top, int Right, int Bottom) ScreenBox(
        in CameraView view, (Vector3 Min, Vector3 Max) box)
    {
        int left = int.MaxValue;
        int top = int.MaxValue;
        int right = int.MinValue;
        int bottom = int.MinValue;
        for (int corner = 0; corner < 8; corner++)
        {
            Vector3 point = new(
                (corner & 1) == 0 ? box.Min.X : box.Max.X,
                (corner & 2) == 0 ? box.Min.Y : box.Max.Y,
                (corner & 4) == 0 ? box.Min.Z : box.Max.Z);
            (int px, int py) = Pixel(view, point);
            left = Math.Min(left, px);
            top = Math.Min(top, py);
            right = Math.Max(right, px);
            bottom = Math.Max(bottom, py);
        }

        return (left, top, right, bottom);
    }

    /// <summary>The linear value of an sRGB 8-bit level (SPEC-06 §2).</summary>
    private static double Linear(int level)
    {
        double c = level / 255.0;
        return c <= 0.04045 ? c / 12.92 : Math.Pow((c + 0.055) / 1.055, 2.4);
    }
}
