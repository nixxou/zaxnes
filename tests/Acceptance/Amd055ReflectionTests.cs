using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-055 §2.5 — the screen-space floor reflections of REQ-SCN-084. The automatic invariants
/// each compare two captures of the <b>close-up R</b> (or one of its variants) that differ by one
/// setting only.
/// </summary>
/// <remarks>
/// <para>
/// Close-up R: capture 1920 × 1080, <c>video.anti_alias</c> = 0, native resolution, scene
/// scale 1, zero curvature, black <c>Color</c> skybox, <c>render.shadow</c> = false, every other
/// effect of §2.3 to §2.5 off, <c>spl.reflections</c> = 1 and the four <c>ssr.*</c> at their
/// default (shine 3 so an intensity of 0.45, clarity 3, length 3 so 200 units, quality 2);
/// white light (1, −1, 1) of intensity 1. A floor of 8 × 8 full tiles (64 × 64 units) turned by
/// 90° about <c>x</c>, of a dark RGB (124, 124, 124) — linear albedo 0.202 — and a red cube of
/// 8 × 8 × 8 units standing on it, footprint centred on the centre of the floor; the camera
/// 40 units from that centre in the direction (−0.5, +0.5, −0.707), FOV 70°.
/// </para>
/// <para>
/// The engine's layer alignment puts an L1 shape of depth 8 on <c>z</c> ∈ [0, 8]
/// (REQ-E3D-205), so the fixtures pass <c>offset.z</c> = −4 to centre it on <c>C</c>, exactly as
/// the close-up A of <see cref="Amd055OcclusionTests"/> does; the floor's upper face is then the
/// plane <c>y</c> = 124 of normal (0, 1, 0) and the cube stands on it.
/// </para>
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class Amd055ReflectionTests
{
    private const int Width = 1920;
    private const int Height = 1080;

    /// <summary>The plane of the floor's upper face.</summary>
    private const float Plane = 124f;

    /// <summary>The scene centre <c>C</c> of REQ-E3D-410.</summary>
    private static Vector3 Center => new(128f, 120f, 0f);

    /// <summary>The dark floor of the close-up R: linear albedo 0.202 ± 0.002.</summary>
    private static Rgb24 Dark => new(124, 124, 124);

    /// <summary>The pure red of the reflected cube: linear (1, 0, 0).</summary>
    private static Rgb24 Red => new(255, 0, 0);

    /// <summary>
    /// Two pixels of the floor visible at the contact of the base of the cube, just outside its
    /// footprint <c>x</c> ∈ [124, 132], <c>z</c> ∈ [−4, 4] on the two sides the camera sees. The
    /// reflected ray leaves them towards <c>+x</c>, <c>+y</c>, <c>+z</c>, so it meets the cube's
    /// (−1, 0, 0) face from the first and its (0, 0, −1) face from the second.
    /// </summary>
    private static Vector3[] Contact =>
    [
        new(123.4f, Plane, 0f),
        new(128f, Plane, -4.6f),
    ];

    // =========================================================== the invariants of §2.5 (auto)

    /// <summary>
    /// REQ-SCN-084, first invariant: close-up R — on the pixels of the floor visible at the
    /// contact of the base of the cube, the <b>linear</b> red channel grows by at least
    /// <b>10 %</b> against the same capture with <c>spl.reflections</c> = 0.
    /// </summary>
    [GlFact] // REQ-SCN-084
    public void REQ_SCN_084_the_floor_at_the_contact_takes_the_red_of_the_cube()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpR();
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        Reflect(scene);
        Shot on = target.Render(renderer, scene);

        Assert.True(renderer.Statistics.ReflectionPass, "the reflection pass must have run.");

        foreach (Vector3 point in Contact)
        {
            (int px, int py) = Pixel(view, point);
            double below = Linear(off.At(px, py).R);
            double above = Linear(on.At(px, py).R);
            Assert.True(
                above >= below * 1.10,
                $"the red channel at {point} went from {below:F4} to {above:F4}, "
                + $"a factor of {above / Math.Max(below, 1e-9):F3} and not 1.10 at least.");
        }
    }

    /// <summary>
    /// REQ-SCN-084, second invariant — variant R-m: the floor is replaced by the same shape
    /// <b>without rotation</b>, whose front face has the scene normal (0, 0, −1). No pixel of
    /// that face is modified (±1 sRGB level): it is not a floor surface, its normal having no
    /// <c>+Y</c> component at all.
    /// </summary>
    [GlFact] // REQ-SCN-084
    public void REQ_SCN_084_a_wall_is_not_a_floor()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = WallR();
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);
        Reflect(scene);
        Shot on = target.Render(renderer, scene);

        // The front face of the wall is the rectangle z = −4, x ∈ [96, 160], y ∈ [88, 152]. The
        // cube stands one unit in front of it, so its screen footprint is left out of the
        // measure with a margin: what the invariant is about is the wall, not what hides it.
        (int left, int top, int right, int bottom) = ScreenBox(view, CubeBoundsR());

        int measured = 0;
        for (float x = 98f; x <= 158f; x += 2f)
        {
            for (float y = 90f; y <= 150f; y += 2f)
            {
                (int px, int py) = Pixel(view, new Vector3(x, y, -4f));
                if (px < 0 || px >= Width || py < 0 || py >= Height)
                {
                    continue;
                }

                if (px >= left - 40 && px <= right + 40 && py >= top - 40 && py <= bottom + 40)
                {
                    continue;
                }

                measured++;
                AssertClose(off, on, px, py, tolerance: 1);
            }
        }

        Assert.True(measured > 400, $"only {measured} pixels of the wall were compared.");
    }

    /// <summary>
    /// REQ-SCN-084, third invariant — variant R-s: a lone block of 8 × 8 units in the <c>xz</c>
    /// plane <b>is</b> modified on at least one pixel of its upper face, and the same block at
    /// <c>scale = (1, 1, 0.5)</c> — 4 units in <c>z</c>, so under the <c>6 × s</c> of the
    /// requirement in one of its two dimensions — is modified on <b>no</b> pixel of it.
    /// </summary>
    [GlFact] // REQ-SCN-084
    public void REQ_SCN_084_the_extent_of_the_object_decides()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        foreach (bool shallow in (bool[])[false, true])
        {
            SceneState scene = BlockR(shallow);
            CameraView view = View(scene);

            Shot off = target.Render(renderer, scene);
            Reflect(scene);
            Shot on = target.Render(renderer, scene);

            // Which pixels are the block's: the grey ones. The variant holds nothing but the
            // block, the pure red cube and a black skybox, so a pixel with any green in it
            // belongs to the block and to nothing else — and of its faces only the upper one can
            // be a floor surface, the others having no +Y normal at all.
            int changed = 0;
            int measured = 0;
            for (int py = 0; py < Height; py++)
            {
                for (int px = 0; px < Width; px++)
                {
                    if (off.At(px, py).G <= 8)
                    {
                        continue;
                    }

                    measured++;
                    if (off.Packed(px, py) != on.Packed(px, py))
                    {
                        changed++;
                    }
                }
            }

            _ = view;
            Assert.True(measured > 1000, $"only {measured} pixels of the block were measured.");
            if (shallow)
            {
                Assert.Equal(0, changed);
            }
            else
            {
                Assert.True(changed > 0, "the 8 x 8 block must reflect on at least one pixel.");
            }
        }
    }

    /// <summary>
    /// REQ-SCN-084, fourth invariant, first part: <c>ssr.shine</c> growing from 1 to 5 makes the
    /// gap between the captures with and without reflections grow monotonically.
    /// </summary>
    [GlFact] // REQ-SCN-084
    public void REQ_SCN_084_the_shine_is_monotone()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpR();
        CameraView view = View(scene);
        Shot off = target.Render(renderer, scene);

        Reflect(scene);
        (int cx, int cy) = Pixel(view, Contact[0]);

        double previous = 0.0;
        for (int shine = 1; shine <= 5; shine++)
        {
            scene.Settings.Style.SsrShine = shine;
            Shot on = target.Render(renderer, scene);

            double gap = Linear(on.At(cx, cy).R) - Linear(off.At(cx, cy).R);
            Assert.True(
                gap > previous,
                $"shine {shine} gave a gap of {gap:F4}, not more than {previous:F4}.");
            previous = gap;
        }
    }

    /// <summary>
    /// REQ-SCN-084, fourth invariant, second part: <c>ssr.length</c> growing lengthens the
    /// modified zone.
    /// </summary>
    /// <remarks>
    /// The zone is measured as the number of pixels the reflection changes by at least half of
    /// what it changes the most: that is the area the reflection really covers, which the
    /// attenuation towards the maximum length shrinks and a longer ray therefore grows. Counting
    /// every pixel that changes at all instead would count the faintest fringe of the zone, where
    /// one sRGB level is the whole signal, and would not move at all.
    /// </remarks>
    [GlFact] // REQ-SCN-084
    public void REQ_SCN_084_the_length_lengthens_the_modified_zone()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpR();
        Shot off = target.Render(renderer, scene);

        Reflect(scene);

        int previous = 0;
        List<int> counts = [];
        for (int length = 1; length <= 5; length++)
        {
            scene.Settings.Style.SsrLength = length;
            Shot on = target.Render(renderer, scene);

            int covered = Covered(off, on);
            counts.Add(covered);
            Assert.True(
                covered > previous,
                $"length {length} covered {covered} pixels, not more than the {previous} of the "
                + $"previous one ({string.Join(", ", counts)}).");
            previous = covered;
        }
    }

    /// <summary>
    /// REQ-SCN-084, fourth invariant, third part: <c>ssr.clarity</c> growing reduces the width of
    /// the transition measured on an edge of the reflection.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Measured on the variant <b>R-c</b> — the close-up R with the red cube raised 12 units
    /// above the floor — and <b>not</b> on the close-up R itself (<b>Q-093</b>). The blur radius
    /// of REQ-SCN-084 is proportional to the distance the ray travelled, and in the close-up R
    /// the cube sits <em>on</em> the floor, so every ray that reaches it has travelled a few
    /// units out of the 200 the default <c>ssr.length</c> allows: the radius is under a third of
    /// a pixel everywhere, and exactly zero where the reflection is strongest. Raising the cube
    /// makes the rays travel about a sixth of their maximum length, which is where the law the
    /// requirement states becomes observable at all.
    /// </para>
    /// <para>
    /// The width is estimated as the height of the transition over its steepest slope: for a
    /// ramp of height <c>H</c> spread over <c>W</c> pixels the largest difference between two
    /// neighbours is <c>H / W</c>, so the largest value over the largest horizontal difference is
    /// the width of the sharpest edge the reflection has.
    /// </para>
    /// </remarks>
    [GlFact] // REQ-SCN-084
    public void REQ_SCN_084_the_clarity_sharpens_the_edge_of_the_reflection()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = RaisedR();
        Shot off = target.Render(renderer, scene);

        Reflect(scene);

        double previous = double.MaxValue;
        List<double> widths = [];
        for (int clarity = 1; clarity <= 5; clarity++)
        {
            scene.Settings.Style.SsrClarity = clarity;
            Shot on = target.Render(renderer, scene);

            double width = TransitionWidth(off, on);
            widths.Add(width);
            Assert.True(
                width < previous,
                $"clarity {clarity} gave a transition of {width:F3} pixels, not under "
                + $"{previous:F3} ({string.Join(", ", widths.Select(w => w.ToString("F3")))}).");
            previous = width;
        }

        Assert.True(widths[0] > 1.5, $"the blur was not measurable at all: {widths[0]:F3}.");
    }

    /// <summary>
    /// REQ-SCN-084, fifth invariant: <c>spl.reflections</c> = 0 gives a capture identical to a
    /// Classic capture of the same scene, to ±2 sRGB levels (REQ-SCN-080; every other effect is
    /// off in the close-up R).
    /// </summary>
    [GlFact] // REQ-SCN-084, REQ-SCN-080
    public void REQ_SCN_084_the_reflections_off_are_classic()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpR();
        Assert.False(scene.Settings.Style.Enhanced);
        Shot classic = target.Render(renderer, scene);

        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.Reflections = false;
        Shot enhanced = target.Render(renderer, scene);
        Assert.False(
            renderer.Statistics.ReflectionPass, "spl.reflections = 0 must not run the pass.");

        AssertSame(classic, enhanced, tolerance: 2);
    }

    /// <summary>
    /// REQ-SCN-084, sixth invariant — variant R-h: the pixels of the skybox and of the HUD are
    /// identical to ±1 sRGB level between <c>spl.reflections</c> = 0 and 1. The skybox is never a
    /// floor surface and the HUD is drawn after the pass (REQ-SCN-058 amended).
    /// </summary>
    [GlFact] // REQ-SCN-084, REQ-SCN-044, REQ-SCN-049
    public void REQ_SCN_084_the_skybox_and_the_hud_are_untouched()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = CloseUpR(hud: true, gradient: true);
        CameraView view = View(scene);

        Shot off = target.Render(renderer, scene);

        // Which pixels are the HUD's: the ones the same scene without the HUD tile does not
        // have. Which are the skybox's: the ones a capture of the skybox alone shares with the
        // full one — REQ-SCN-047 hides the 3D scene outside a game but keeps the skybox.
        Shot noHud = target.Render(renderer, CloseUpR(gradient: true));
        SceneState empty = CloseUpR(gradient: true);
        empty.GameRunning = false;
        Shot skyAlone = target.Render(renderer, empty);

        Reflect(scene);
        Shot on = target.Render(renderer, scene);

        // The pass did run, and it did change the floor: the test is not vacuous.
        Assert.True(renderer.Statistics.ReflectionPass);
        (int cx, int cy) = Pixel(view, Contact[0]);
        Assert.True(
            Linear(on.At(cx, cy).R) > Linear(off.At(cx, cy).R) * 1.10,
            "the floor must still take the red of the cube.");

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

    // ================================================================================= fixtures

    /// <summary>
    /// The close-up R: the dark floor of 8 × 8 full tiles, the red cube of 8 × 8 × 8 units
    /// standing on it, and the camera the amendment places.
    /// </summary>
    /// <param name="hud">Whether a HUD tile is drawn at the centre (variant R-h).</param>
    /// <param name="gradient">Whether the Gradient skybox of the variant R-h is drawn.</param>
    private static SceneState CloseUpR(bool hud = false, bool gradient = false)
        => Scene(Ground.Floor, hud, gradient, shallow: false);

    /// <summary>The variant R-m: the same shape without rotation, so a wall.</summary>
    private static SceneState WallR() => Scene(Ground.Wall, false, false, shallow: false);

    /// <summary>
    /// The variant R-c: the close-up R with the red cube raised 12 units above the floor, so that
    /// the reflected rays travel a sizeable fraction of their maximum length and the blur of
    /// REQ-SCN-084 — whose radius is proportional to that distance — is observable (Q-093).
    /// </summary>
    // Q-093
    private static SceneState RaisedR()
        => Scene(Ground.Floor, false, false, shallow: false, raise: 12f);

    /// <summary>
    /// The variant R-s: a lone full tile of 8 × 8 × 8 units, with the red cube beside it.
    /// </summary>
    /// <param name="shallow">
    /// Whether the block carries <c>scale = (1, 1, 0.5)</c>, which brings it to 4 units in
    /// <c>z</c> and therefore under the threshold of REQ-SCN-084.
    /// </param>
    private static SceneState BlockR(bool shallow) => Scene(Ground.Block, false, false, shallow);

    /// <summary>What stands in for the floor of the close-up R.</summary>
    private enum Ground
    {
        /// <summary>The 8 × 8 tile shape turned by 90° about <c>x</c>.</summary>
        Floor,

        /// <summary>The variant R-m: the same shape, unturned.</summary>
        Wall,

        /// <summary>The variant R-s: one full tile, unturned.</summary>
        Block,
    }

    private static SceneState Scene(
        Ground ground, bool hud, bool gradient, bool shallow, float raise = 0f)
    {
        SceneFixture fixture = new();
        fixture.Background(0x0F);

        if (ground == Ground.Block)
        {
            // One full tile of depth 8, centred on C: 8 × 8 × 8 units, upper face y = 124 of
            // normal (0, 1, 0), xz box of 8 × 8 units — or 8 × 4 with the scale.
            fixture
                .Pattern(
                    SceneFixture.Solid(1),
                    depth: 8,
                    ProfileLayer.L1,
                    texture: SceneFixture.FlatDirectTexture(Dark),
                    scale: shallow ? new Vec3F(1f, 1f, 0.5f) : null,
                    offset: new Vec3F(-4f, -4f, -4f))
                .Palette(1, 0x30)
                .Tile(SceneFixture.Solid(1), 128, 120, palette: 1);
        }
        else
        {
            // 8 × 8 full tiles (64 × 64 units) of depth 8, centred on C; turned by 90° about x
            // for the floor, left flat for the wall of the variant R-m.
            fixture
                .Pattern(
                    SceneFixture.Solid(1),
                    depth: 8,
                    ProfileLayer.L1,
                    columns: 8,
                    rows: 8,
                    texture: SceneFixture.FlatDirectTexture(Dark, 8, 8),
                    rotation: ground == Ground.Floor ? new Vec3F(90f, 0f, 0f) : null,
                    offset: new Vec3F(0f, 0f, -4f))
                .Palette(1, 0x30)
                .Rect(SceneFixture.Solid(1), 12, 12, 8, 8, palette: 1);
        }

        // The red cube: one full tile of sizeZ = 8, its tile record placed clear of the ground
        // block so that the segmentation keeps two shapes, then brought by its offset where the
        // close-up wants it.
        fixture
            .Pattern(
                SceneFixture.Solid(1, 1),
                depth: 8,
                ProfileLayer.L1,
                texture: SceneFixture.FlatDirectTexture(Red),
                offset: CubeOffset(ground, raise))
            .Palette(5, 0x30)
            .Tile(SceneFixture.Solid(1, 1), 192, 120, palette: 1);

        if (hud)
        {
            // A HUD motif of the variant R-h, on the cell row 2. The HUD plane of REQ-SCN-045
            // is 109 units in front of the eye and the camera of the close-up R is only 40 units
            // from the floor, so a motif at the centre of the reference frame lands *behind* the
            // floor and is hidden; the rows near the top of the frame are the ones whose HUD ray
            // clears the far edge of the floor and is visible at all. Row 2 is also clear of the
            // ground block, whose cells are the rows 12 to 19, so that no cell carries two tile
            // records.
            fixture
                .Pattern(SceneFixture.Solid(1, 2), depth: 1, ProfileLayer.Ui, columns: 2, rows: 2)
                .Rect(SceneFixture.Solid(1, 2), 15, 2, 2, 2, palette: 1);
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

        // The camera 40 units from the centre of the floor in the direction
        // (−0.5, +0.5, −0.707): elevation 30°, azimuth 35.264°.
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Target = Center;
        scene.Camera.Current.Distance = 40f;
        scene.Camera.Current.Angles = new Vector3(30f, 35.26439f, 0f);
        scene.Camera.SnapToSetpoints();
        return scene;
    }

    /// <summary>
    /// Where the red cube goes. The close-up A of <see cref="Amd055OcclusionTests"/> establishes
    /// that the record at the screen cell (24, 15) with the offset (−68, 4, −4) lands the cube on
    /// <c>x</c> ∈ [124, 132], <c>y</c> ∈ [124, 132], <c>z</c> ∈ [−4, 4] — standing on the plane
    /// <c>y</c> = 124 with its footprint centred on <c>C</c>.
    /// </summary>
    private static Vec3F CubeOffset(Ground ground, float raise) => ground switch
    {
        // Variant R-m: in front of the wall, whose front face is the plane z = −4, by one unit:
        // the cube then occupies z ∈ [−13, −5].
        Ground.Wall => new Vec3F(-68f, 4f, -13f),

        // Variant R-s: beside the block, its (−1, 0, 0) face two units from the block's +x edge
        // (x = 132), so x ∈ [134, 142], standing at the level of the block's upper face.
        Ground.Block => new Vec3F(-58f, 4f, -4f),

        _ => new Vec3F(-68f, 4f + raise, -4f),
    };

    /// <summary>The scene-space box the red cube of the variant R-m occupies.</summary>
    private static (Vector3 Min, Vector3 Max) CubeBoundsR()
        => (new Vector3(124f, 124f, -13f), new Vector3(132f, 132f, -5f));

    // ================================================================================== measures

    /// <summary>
    /// Turns the Enhanced style on with the reflections. The reference capture of every invariant
    /// is taken in Classic, which REQ-SCN-080 makes identical to Enhanced with every effect off.
    /// </summary>
    private static void Reflect(SceneState scene)
    {
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.Reflections = true;
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

    /// <summary>
    /// The area the reflection really covers: the pixels whose linear red channel the reflection
    /// raises by at least half of what it raises the most (see the remark of the length test).
    /// </summary>
    private static int Covered(Shot off, Shot on)
    {
        double peak = Peak(off, on);
        Assert.True(peak > 0.0, "the reflection changed nothing at all.");

        int count = 0;
        for (int y = 0; y < off.Height; y++)
        {
            for (int x = 0; x < off.Width; x++)
            {
                if (Linear(on.At(x, y).R) - Linear(off.At(x, y).R) > 0.5 * peak)
                {
                    count++;
                }
            }
        }

        return count;
    }

    /// <summary>
    /// The transition width of the sharpest edge of the reflection, in pixels: the height of the
    /// transition over its steepest slope (see the remark of the clarity test).
    /// </summary>
    private static double TransitionWidth(Shot off, Shot on)
    {
        double peak = Peak(off, on);
        Assert.True(peak > 0.0, "the reflection changed nothing at all.");

        double slope = 0.0;
        for (int y = 0; y < off.Height; y++)
        {
            double previous = 0.0;
            for (int x = 0; x < off.Width; x++)
            {
                double difference = Math.Max(
                    0.0, Linear(on.At(x, y).R) - Linear(off.At(x, y).R));
                slope = Math.Max(slope, Math.Abs(difference - previous));
                previous = difference;
            }
        }

        Assert.True(slope > 0.0, "the reflection has no edge at all.");
        return peak / slope;
    }

    /// <summary>The largest rise of the linear red channel the reflection causes.</summary>
    private static double Peak(Shot off, Shot on)
    {
        double peak = 0.0;
        for (int y = 0; y < off.Height; y++)
        {
            for (int x = 0; x < off.Width; x++)
            {
                peak = Math.Max(peak, Linear(on.At(x, y).R) - Linear(off.At(x, y).R));
            }
        }

        return peak;
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
