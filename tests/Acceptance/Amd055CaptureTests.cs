using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// AMD-055 — the captures of §2.2 and §2.3: the identity of Classic and Enhanced with every
/// effect off (REQ-SCN-080), the voxel grid of REQ-SCN-081 with its close-up G and the wide
/// framing B of the clause of order, and the pixel shadows of REQ-SCN-082.
/// </summary>
/// <remarks>
/// <para>
/// Every test builds and destroys its own hidden context, and the whole class is one xUnit
/// collection so that two contexts are never created at once.
/// </para>
/// <para>
/// The cameras are the ones each close-up names, placed exactly: the first-person camera of
/// REQ-SCN-009 puts the eye at <c>target − q·(0, 0, 1) · d</c> with no re-centring on the
/// visible rectangle, which is what « camera at (x, y, z), aiming at the centre » asks for.
/// Measured points are then found by projecting through the view the renderer itself used, so
/// no pixel coordinate is hard-coded.
/// </para>
/// </remarks>
[Collection("opengl-acceptance")]
public sealed class Amd055CaptureTests
{
    /// <summary>The capture width every close-up of the amendment asks for.</summary>
    private const int Width = 1920;

    /// <summary>The capture height.</summary>
    private const int Height = 1080;

    /// <summary>The scene centre <c>C</c> of REQ-E3D-410.</summary>
    private static Vector3 Center => new(128f, 120f, 0f);

    /// <summary>The albedo colour of the close-ups: linear 0.503 ± 0.002.</summary>
    private static Rgb24 Grey => new(188, 188, 188);

    // =========================================================== §2.2 REQ-SCN-080, base lighting

    /// <summary>
    /// REQ-SCN-080, first criterion: the same scene in Classic and in Enhanced with every effect
    /// off gives captures identical to ±2 sRGB levels per channel. The scene carries lit tiles, a
    /// backdrop and a HUD tile, so the three lighting models the requirement names are all in the
    /// comparison.
    /// </summary>
    [GlFact] // REQ-SCN-080
    public void REQ_SCN_080_enhanced_with_every_effect_off_is_classic()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = BuildCloseUp(CloseUpFixture(hud: true));
        scene.Backdrop.Enabled = true;
        scene.Backdrop.Color = Grey;

        Shot classic = target.Render(renderer, scene);
        Assert.False(scene.Settings.Style.Enhanced);

        scene.Settings.Style.Enhanced = true;
        Shot enhanced = target.Render(renderer, scene);

        AssertSame(classic, enhanced, tolerance: 2);
    }

    /// <summary>
    /// REQ-SCN-078: a change of style, or of one of its parameters, is in force at the <b>next
    /// rendered frame</b>, with no reload of the game — the same renderer answers Classic, then
    /// Enhanced with the grid, then Classic again.
    /// </summary>
    [GlFact] // REQ-SCN-078, REQ-SCN-080
    public void REQ_SCN_078_a_style_change_is_in_force_at_the_next_frame()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = BuildCloseUp(CloseUpFixture());
        Shot classic = target.Render(renderer, scene);

        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.VoxelGrid = 2;
        Shot grooved = target.Render(renderer, scene);
        Assert.True(Different(classic, grooved) > 1000, "the grid must change the image.");

        scene.Settings.Style.Enhanced = false;
        AssertSame(classic, target.Render(renderer, scene), tolerance: 0);

        // And the same for the parameter alone, the style staying on.
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.VoxelGrid = 0;
        AssertSame(classic, target.Render(renderer, scene), tolerance: 2);
    }

    // ============================================================= §2.3 REQ-SCN-081, voxel grid

    /// <summary>
    /// REQ-SCN-081, first criterion — close-up G, <c>spl.voxel_grid</c> = 2 (so <c>N</c> = 4),
    /// the front face of normal (0, 0, −1), no shadow, no pixel shadows. The four points are
    /// given in the local frame of the tile whose centre is (132, 132) in the scene frame, and
    /// the expected ratios of linear values are 0.62, 1.00, 1.048 and 0.384.
    /// </summary>
    /// <remarks>
    /// Q-092: the amendment calls that tile « the one adjacent to the centre of the face on the
    /// <c>+x</c>, <c>+y</c> side », which with its own <c>y</c> ∈ [104, 136] is the tile centred
    /// on (132, 124); (132, 132) is the next one up. Either reading gives the same numbers,
    /// because neighbouring tile centres are 8 units apart and 8 is a multiple of every cell
    /// side <c>N</c> ∈ {2, 4, 8}, so the groove phase is the same in both local frames. The
    /// coordinates the amendment writes are the ones used here.
    /// </remarks>
    [GlFact] // REQ-SCN-081
    public void REQ_SCN_081_the_four_points_of_the_close_up_hold_their_ratios()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = BuildCloseUp(CloseUpFixture());
        CameraView view = View(scene);
        Assert.Equal(new Vector3(128f, 120f, -34f), view.Eye);
        Mat4 tile = TileAt(scene, 132f, 132f);

        Shot off = target.Render(renderer, scene);

        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.VoxelGrid = 2;
        Shot on = target.Render(renderer, scene);

        (float X, float Y, double Ratio, double Window, int Srgb)[] points =
        [
            (0f, 2f, 0.62, 0.03, 133),      // centre of a groove, one axis only
            (2f, 2f, 1.00, 0.01, 166),      // centre of a cell
            (0.4f, 2f, 1.048, 0.02, 169),   // bright edge, d = 10 %
            (0f, 0f, 0.384, 0.03, 107),     // crossing of two grooves
        ];

        foreach ((float x, float y, double ratio, double window, int srgb) in points)
        {
            (int px, int py) = Pixel(view, Local(tile, x, y));
            int onLevel = on.At(px, py).R;
            int offLevel = off.At(px, py).R;

            // The « off » column of the table: the four points read 166 ± 3 without the grid.
            Assert.InRange(offLevel, 163, 169);

            // The « on » column, and the ratio of the linear values the criterion gives.
            Assert.InRange(onLevel, srgb - 3, srgb + 3);
            Assert.InRange(Linear(onLevel) / Linear(offLevel), ratio - window, ratio + window);
        }

        // In Enhanced with spl.voxel_grid = 0 the four points read 166 ± 3 again: the criterion
        // is discriminant.
        scene.Settings.Style.VoxelGrid = 0;
        Shot enhancedOff = target.Render(renderer, scene);
        foreach ((float x, float y, double _, double _, int _) in points)
        {
            (int px, int py) = Pixel(view, Local(tile, x, y));
            Assert.InRange(enhancedOff.At(px, py).R, 163, 169);
        }
    }

    /// <summary>
    /// REQ-SCN-081, second criterion — close-up G with <c>spl.voxel_grid</c> = 1, 2 then 3: the
    /// spacing of the groove lines measured on the screen is 2, 4 and 8 scene units (±0.1), a
    /// line goes through the centre of every tile in all three cases, and at 1 and 2 a line also
    /// goes through the junction of two neighbouring tiles.
    /// </summary>
    [GlFact] // REQ-SCN-081
    public void REQ_SCN_081_the_groove_spacing_follows_the_cell_side()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = BuildCloseUp(CloseUpFixture());
        CameraView view = View(scene);
        Mat4 tile = TileAt(scene, 132f, 132f);
        scene.Settings.Style.Enhanced = true;

        foreach ((int setting, double cell) in (ValueTuple<int, double>[])[(1, 2d), (2, 4d), (3, 8d)])
        {
            scene.Settings.Style.VoxelGrid = setting;
            Shot shot = target.Render(renderer, scene);

            // The vertical groove lines crossed by the row of pixels that passes through the
            // centre of the measured tile, given back in scene units.
            List<double> grooves = VerticalGrooves(shot, view, tile);
            Assert.True(grooves.Count >= 3, $"N = {cell}: only {grooves.Count} grooves found.");

            for (int i = 1; i < grooves.Count; i++)
            {
                Assert.InRange(grooves[i] - grooves[i - 1], cell - 0.1, cell + 0.1);
            }

            // A line through the centre of every tile: the measured tile is centred on scene
            // x = 132, its neighbours on 124 and 140.
            foreach (double centre in (double[])[124d, 132d, 140d])
            {
                Assert.Contains(grooves, x => Math.Abs(x - centre) < 0.2);
            }

            // The junction of two neighbouring tiles is at scene x = 128 and 136: a line goes
            // through it exactly when the cell side divides 4, so for N = 2 and N = 4.
            bool junction = grooves.Exists(static x => Math.Abs(x - 128d) < 0.2);
            Assert.Equal(setting <= 2, junction);
        }
    }

    /// <summary>
    /// REQ-SCN-081, third criterion — the grid follows the object: with <c>scale = (2, 1, 1)</c>
    /// the spacing of the grooves measured on the screen <b>doubles</b> on the <c>x</c> axis, and
    /// with <c>rot = (0, 0, 30)</c> the grooves turn by 30° with the face.
    /// </summary>
    [GlFact] // REQ-SCN-081
    public void REQ_SCN_081_the_grid_follows_the_object_transform()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        // The grid is evaluated in the local mesh frame, before the object transform, so a cell
        // of N = 4 local units covers 8 scene units on x once the object is stretched twice.
        double plain = GrooveSpacing(target, renderer, scale: null, rotation: null);
        double stretched = GrooveSpacing(target, renderer, new Vec3F(2f, 1f, 1f), null);
        Assert.InRange(stretched / plain, 1.9, 2.1);

        // The rotation: the angle of a groove line measured on the screen. The camera looks
        // straight down the normal of the face, so the screen angle is the angle in the face.
        double flat = GrooveAngle(target, renderer, rotation: null);
        double turned = GrooveAngle(target, renderer, new Vec3F(0f, 0f, 30f));
        Assert.InRange(Math.Abs(turned - flat), 27.0, 33.0);
    }

    /// <summary>
    /// REQ-SCN-080, clause of order against the curvature — wide framing B, <c>spl.bend_zx</c> at
    /// 90°: each horizontal groove line of the front face of the backdrop between 8 and 56 units
    /// of the centre <c>C</c> is <b>curved</b>, its greatest departure from its chord exceeding
    /// 15 pixels and equal to <c>2.661 × |y − 120|</c> pixels (tolerance 3 pixels or 3 %,
    /// whichever is larger). At zero curvature the same line is straight to under a pixel.
    /// </summary>
    /// <remarks>
    /// The note of REQ-SCN-081 says what this criterion settles: the grid is <b>painted on the
    /// geometry</b>. A grid evaluated on the projected position, or in screen space, would give
    /// straight and evenly spaced lines in both captures and would fail here. It does not tell
    /// an evaluation before the wrapping of REQ-SCN-067 from one after it; that part of the
    /// clause is declared rather than measured.
    /// </remarks>
    [GlFact] // REQ-SCN-080, REQ-SCN-081
    public void REQ_SCN_080_the_grooves_of_the_backdrop_follow_the_curvature()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        // Wide framing B: no shape at all, the backdrop alone at its default size, N = 8, the
        // camera 110 units in front of its front face.
        SceneFixture fixture = new();
        fixture.Background(0x0F);
        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        Configure(scene);
        scene.Backdrop.Enabled = true;
        scene.Backdrop.Color = Grey;
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.VoxelGrid = 3;
        Aim(scene, Center, 110.5f, Vector3.Zero);

        Assert.Equal(new Vector3(128f, 120f, -110.5f), View(scene).Eye);

        Shot straight = target.Render(renderer, scene);
        CameraView flatView = View(scene);

        scene.Settings.BendLeftRight = 90;
        Shot bent = target.Render(renderer, scene);
        CameraView bentView = View(scene);
        SpaceBend bend = scene.Settings.Bend;

        foreach (int y in (int[])[64, 72, 80, 88, 96, 104, 112, 128, 136, 144, 152, 160, 168, 176])
        {
            double expected = 2.661 * Math.Abs(y - 120);
            double tolerance = Math.Max(3.0, 0.03 * expected);

            double curved = ChordDeparture(bent, bentView, bend, y);
            Assert.True(
                curved > 15.0,
                $"the groove line y = {y} departs from its chord by only {curved:F1} px.");
            Assert.InRange(curved, expected - tolerance, expected + tolerance);

            double level = ChordDeparture(straight, flatView, SpaceBend.None, y);
            Assert.True(level < 1.0, $"at zero curvature the line y = {y} bends by {level:F2} px.");
        }
    }

    // =========================================================== §2.3 REQ-SCN-082, pixel shadows

    /// <summary>
    /// REQ-SCN-082, the automatic criterion: a floor of 4 × 4 full tiles with
    /// <c>rot = (90, 0, 0)</c>, so of scene normal (0, 1, 0), and a plate of <c>sizeZ</c> = 1
    /// whose lower face is 29 units above it, its vertical footprint centred on the centre of the
    /// floor, under the light (1, −3, 1). The shadow of the plate is then a square of 8 × 8 units
    /// whose edges fall 5.667 and 13.667 units from the centre of the floor — in the middle of a
    /// voxel.
    /// <list type="bullet">
    /// <item>with <c>spl.pixel_shadow</c> = 1, every pixel of a completely visible one-voxel cell
    /// carries the same value to ±2 sRGB levels, and the boundary between shadowed and lit cells
    /// follows cell edges exactly, at the whole coordinates 6 and 14;</item>
    /// <item>with <c>spl.pixel_shadow</c> = 0, at least one cell of each edge of the shadow holds
    /// both shadowed and lit pixels — the clause that makes the criterion discriminant, the
    /// shadows of REQ-SCN-030 being hard already.</item>
    /// </list>
    /// </summary>
    [GlFact] // REQ-SCN-082
    public void REQ_SCN_082_the_shadow_changes_by_squares_of_one_voxel()
    {
        using GlTarget target = GlTarget.Create(Width, Height);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = BuildPixelShadow();
        CameraView view = View(scene);

        scene.Settings.Style.Enhanced = true;
        Shot soft = target.Render(renderer, scene);

        scene.Settings.Style.PixelShadow = true;
        Shot hard = target.Render(renderer, scene);

        // The cells of the floor are the voxels of REQ-E3D-504: the local frame of each tile
        // runs over [−4, 4] about a whole tile centre, so a cell edge of the floor is a whole
        // scene coordinate on both axes of its plane. The row measured is z ∈ [9, 10], which
        // crosses the shadow (z ∈ [5.667, 13.667]), and the column x ∈ [137, 138], which
        // crosses it too.
        AssertCellsAreOfOnePiece(hard, soft, view, FloorCellsAlongX(9), "x");
        AssertCellsAreOfOnePiece(hard, soft, view, FloorCellsAlongZ(137), "z");
    }

    /// <summary>
    /// REQ-SCN-082: the effect is without object on the backdrop. A scene whose only receiver is
    /// the backdrop — a shape of 4 × 4 tiles casting its shadow on the box behind it — gives
    /// captures identical to the pixel whether <c>spl.pixel_shadow</c> is 0 or 1.
    /// </summary>
    [GlFact] // REQ-SCN-082
    public void REQ_SCN_082_the_backdrop_is_untouched()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        // The block is 24 units in front of the box, so its shadow falls on the box and not on
        // itself; the camera is far enough back to see both.
        SceneState scene = BuildCloseUp(CloseUpFixture());
        scene.Settings.Shadows = true;
        scene.Backdrop.Enabled = true;
        scene.Backdrop.Color = Grey;
        scene.Backdrop.Offset = new Vector3(0f, 0f, 24f);
        scene.Settings.Style.Enhanced = true;
        Aim(scene, Center, 120f, Vector3.Zero);

        Shot shadowed = target.Render(renderer, scene);

        // The test is not vacuous: there is a shadow on the box to be found.
        scene.Settings.Shadows = false;
        Assert.True(
            Different(shadowed, target.Render(renderer, scene)) > 500,
            "the block must cast a visible shadow on the box.");

        // And the pixel shadows leave every pixel of it alone.
        scene.Settings.Shadows = true;
        scene.Settings.Style.PixelShadow = true;
        Assert.Equal(shadowed.Pixels, target.Render(renderer, scene).Pixels);
    }

    /// <summary>
    /// REQ-SCN-079, third criterion: changing <c>ao.thickness</c> from 1 to 5 with
    /// <c>spl.ao</c> = 2 (Depth) and every other setting fixed leaves the capture identical to
    /// the pixel — the Depth occlusion does not use it (REQ-SCN-083).
    /// </summary>
    [GlFact] // REQ-SCN-079, REQ-SCN-083
    public void REQ_SCN_079_the_thickness_has_no_effect_with_the_depth_occlusion()
    {
        using GlTarget target = GlTarget.Create(640, 480);
        using SceneRenderer renderer = new(target.Gl);

        SceneState scene = BuildPixelShadow();
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.AmbientOcclusion = 2;

        scene.Settings.Style.AoThickness = 1;
        Shot thin = target.Render(renderer, scene);

        scene.Settings.Style.AoThickness = 5;
        Shot thick = target.Render(renderer, scene);

        Assert.Equal(thin.Pixels, thick.Pixels);
    }

    // ================================================================================= fixtures

    /// <summary>
    /// The shape of the close-up G: 4 × 4 full tiles (32 × 32 units) of construction depth 8,
    /// centred on <c>C</c> — so <c>x</c> ∈ [112, 144], <c>y</c> ∈ [104, 136], <c>z</c> ∈ [−4, 4]
    /// — with the custom direct texture of colour (188, 188, 188). The <c>z</c> offset of −4
    /// cancels the front alignment of layer L1 (REQ-E3D-205), which would otherwise put the
    /// block at <c>z</c> ∈ [0, 8].
    /// </summary>
    private static SceneFixture CloseUpFixture(
        Vec3F? scale = null,
        Vec3F? rotation = null,
        bool hud = false)
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(
                SceneFixture.Solid(1),
                depth: 8,
                ProfileLayer.L1,
                columns: 4,
                rows: 4,
                texture: SceneFixture.FlatDirectTexture(Grey, 4, 4),
                scale: scale,
                rotation: rotation,
                offset: new Vec3F(0f, 0f, -4f))
            .Palette(1, 0x30)
            .Rect(SceneFixture.Solid(1), 14, 14, 4, 4, palette: 1);

        if (hud)
        {
            // A HUD tile, so that the lighting of REQ-SCN-029 and the rendering of REQ-SCN-044
            // are in the comparison of REQ-SCN-080 too.
            fixture
                .Pattern(SceneFixture.Solid(1, 2), depth: 1, ProfileLayer.Ui)
                .Palette(2, 0x21)
                .Tile(SceneFixture.Solid(1, 2), 32, 24, palette: 0);
        }

        return fixture;
    }

    /// <summary>
    /// The close-up G: the camera at (128, 120, −34), 30 units from the centre of the measured
    /// front face, on its normal, aiming at that centre, FOV 70°.
    /// </summary>
    private static SceneState BuildCloseUp(SceneFixture fixture)
    {
        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        Configure(scene);
        Aim(scene, new Vector3(128f, 120f, -4f), 30f, Vector3.Zero);
        return scene;
    }

    /// <summary>
    /// The floor and the plate of the criterion of REQ-SCN-082. The floor is a shape of 4 × 4
    /// full tiles turned by 90° about <c>x</c>, centred on <c>C</c>: its upper face is then the
    /// plane <c>y</c> = 124 of normal (0, 1, 0), spanning <c>x</c> ∈ [112, 144] and
    /// <c>z</c> ∈ [−16, 16] about the centre (128, 0). The plate is a full tile of
    /// <c>sizeZ</c> = 1 turned the same way, its footprint centred on (128, 0) and its lower face
    /// 29 units above the floor. Both are in L1, the only layer that casts (REQ-E3D-567).
    /// </summary>
    private static SceneState BuildPixelShadow()
    {
        SceneFixture fixture = new();
        fixture
            .Background(0x0F)
            .Pattern(
                SceneFixture.Solid(1),
                depth: 8,
                ProfileLayer.L1,
                columns: 4,
                rows: 4,
                texture: SceneFixture.FlatDirectTexture(Grey, 4, 4),
                rotation: new Vec3F(90f, 0f, 0f),
                offset: new Vec3F(0f, 0f, -4f))
            .Pattern(
                SceneFixture.Solid(1, 1),
                depth: 1,
                ProfileLayer.L1,
                texture: SceneFixture.FlatDirectTexture(Grey),
                rotation: new Vec3F(90f, 0f, 0f),

                // The tile record of the plate is placed well clear of the floor block so that
                // the segmentation keeps two shapes; the offset brings it over the centre of the
                // floor, 29 units up.
                offset: new Vec3F(-36f, 29.5f, -0.5f))
            .Palette(1, 0x30)
            .Palette(5, 0x30)
            .Rect(SceneFixture.Solid(1), 14, 14, 4, 4, palette: 1)
            .Tile(SceneFixture.Solid(1, 1), 160, 120, palette: 1);

        SceneState scene = fixture.Build();
        scene.Palette(fixture);
        Configure(scene);

        scene.Settings.Shadows = true;
        scene.Light.Direction = new Vector3(1f, -3f, 1f);

        // The camera 30 units from the centre of the shadow, elevation 45°, azimuth 0 (from −z).
        // The shadow square is centred 29/3 units past the centre of the floor on both axes of
        // its plane.
        Aim(
            scene,
            new Vector3(128f + (29f / 3f), 124f, 29f / 3f),
            30f,
            new Vector3(45f, 0f, 0f));
        return scene;
    }

    /// <summary>
    /// The settings every close-up of the amendment shares: capture at the native resolution, no
    /// anti-aliasing, scene scale 1, zero curvature, black <c>Color</c> skybox, no shadow, and
    /// the white light of REQ-SCN-025 at intensity 1.
    /// </summary>
    private static void Configure(SceneState scene)
    {
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
    }

    /// <summary>
    /// Puts the camera exactly where a close-up asks for it: first-person, so that the eye is
    /// <c>target − q·(0, 0, 1) · d</c> and the aim is <c>q·(0, 0, 1)</c>, with no re-centring on
    /// the visible rectangle (REQ-SCN-009, REQ-SCN-010).
    /// </summary>
    private static void Aim(SceneState scene, Vector3 target, float distance, Vector3 angles)
    {
        scene.Camera.SetMode(CameraMode.Fpv);
        scene.Camera.Current.Target = target;
        scene.Camera.Current.Distance = distance;
        scene.Camera.Current.Angles = angles;
        scene.Camera.SnapToSetpoints();
    }

    // ================================================================================== measures

    private static CameraView View(SceneState scene)
        => scene.Camera.BuildView(scene.Clipping, (float)Width / Height);

    /// <summary>
    /// The transform of the tile whose centre <c>T</c> is at (<paramref name="x"/>,
    /// <paramref name="y"/>) in the scene frame (REQ-E3D-405): it maps a point of the local mesh
    /// frame of REQ-E3D-504 to the scene.
    /// </summary>
    private static Mat4 TileAt(SceneState scene, float x, float y)
        => Single(scene, tile
            => Math.Abs(tile.TileCenter.X - x) < 0.01f && Math.Abs(tile.TileCenter.Y - y) < 0.01f);

    private static Mat4 Single(SceneState scene, Func<DrawTile, bool> wanted)
    {
        List<Mat4> found = [];
        foreach (DrawTile tile in scene.DrawList!.Tiles)
        {
            if (wanted(tile))
            {
                found.Add(tile.Transform);
            }
        }

        Assert.True(
            found.Count == 1,
            $"{found.Count} tiles matched among "
            + string.Join(", ", scene.DrawList!.Tiles.Select(static t => t.TileCenter)));
        return found[0];
    }

    /// <summary>The scene position of a point of the front face, in the local mesh frame.</summary>
    private static Vector3 Local(Mat4 transform, float x, float y) => Local(transform, x, y, -4f);

    private static Vector3 Local(Mat4 transform, float x, float y, float z)
    {
        Vec3 scene = transform.TransformPoint(new Vec3(x, y, z));
        return new Vector3(scene.X, scene.Y, scene.Z);
    }

    /// <summary>
    /// The pixel a scene point projects to. The scene frame is left-handed, so the clip position
    /// is built from <c>(x, y, −z)</c> like the vertex shader's (REQ-SCN-001).
    /// </summary>
    private static (int X, int Y) Pixel(in CameraView view, Vector3 point)
    {
        Vector2 at = Project(view, point);
        return ((int)Math.Round(at.X), (int)Math.Round(at.Y));
    }

    private static Vector2 Project(in CameraView view, Vector3 point)
    {
        Vector4 clip = Vector4.Transform(new Vector3(point.X, point.Y, -point.Z), view.ViewProjection);
        float x = clip.X / clip.W;
        float y = clip.Y / clip.W;
        return new Vector2((x + 1f) * 0.5f * Width, (1f - y) * 0.5f * Height);
    }

    /// <summary>The linear value of an sRGB 8-bit level (SPEC-06 §2).</summary>
    private static double Linear(int level)
    {
        double c = level / 255.0;
        return c <= 0.04045 ? c / 12.92 : Math.Pow((c + 0.055) / 1.055, 2.4);
    }

    /// <summary>
    /// The scene <c>x</c> of every vertical groove line crossed by the row of pixels that passes
    /// through the centre of the measured tile. A pixel belongs to a groove when its luminance is
    /// below the midpoint between the darkest and the brightest of the row; the centre of each
    /// run of such pixels is turned back into scene units by the affine mapping two known points
    /// of the row give.
    /// </summary>
    private static List<double> VerticalGrooves(Shot shot, in CameraView view, Mat4 tile)
    {
        // The row, and the span measured on it: 11 local units each side of the tile centre, so
        // the measure stays on the block of 4 × 4 tiles (16 units each side).
        (int _, int row) = Pixel(view, Local(tile, 0f, 2f));
        (int left, int _) = Pixel(view, Local(tile, -11f, 2f));
        (int right, int _) = Pixel(view, Local(tile, 11f, 2f));

        List<double> centres = [];
        foreach ((int from, int to) in Runs(shot, row, left, right))
        {
            centres.Add(SceneX(view, tile, (from + to) * 0.5));
        }

        centres.Sort();
        return centres;
    }

    /// <summary>The runs of groove pixels along one row, between two columns.</summary>
    private static List<(int From, int To)> Runs(Shot shot, int row, int left, int right)
    {
        double darkest = 1d;
        double brightest = 0d;
        for (int x = left; x <= right; x++)
        {
            darkest = Math.Min(darkest, shot.Luma(x, row));
            brightest = Math.Max(brightest, shot.Luma(x, row));
        }

        double threshold = (darkest + brightest) * 0.5;
        List<(int From, int To)> runs = [];
        int start = -1;
        for (int x = left; x <= right; x++)
        {
            bool dark = shot.Luma(x, row) <= threshold;
            if (dark && start < 0)
            {
                start = x;
            }
            else if (!dark && start >= 0)
            {
                runs.Add((start, x - 1));
                start = -1;
            }
        }

        if (start >= 0)
        {
            runs.Add((start, right));
        }

        // A run touching either end of the span may be clipped, so its centre is not its
        // groove's centre: those two are dropped.
        return [.. runs.Where(run => run.From > left && run.To < right)];
    }

    /// <summary>
    /// The scene <c>x</c> a pixel column of the measured row stands for, by inverting the
    /// projection on the plane of the face: two known points give the affine mapping.
    /// </summary>
    private static double SceneX(in CameraView view, Mat4 tile, double column)
    {
        Vector2 a = Project(view, Local(tile, -8f, 2f));
        Vector2 b = Project(view, Local(tile, 8f, 2f));
        Vector3 left = Local(tile, -8f, 2f);
        Vector3 right = Local(tile, 8f, 2f);

        double t = (column - a.X) / (b.X - a.X);
        return left.X + (t * (right.X - left.X));
    }

    /// <summary>
    /// The mean spacing of the groove lines along the middle row of a close-up, in pixels, with
    /// <c>N</c> = 4 on the object given.
    /// </summary>
    private static double GrooveSpacing(
        GlTarget target,
        SceneRenderer renderer,
        Vec3F? scale,
        Vec3F? rotation)
    {
        List<int> columns = MiddleRowGrooves(target, renderer, scale, rotation, out _);
        Assert.True(columns.Count >= 3, $"only {columns.Count} grooves on the middle row.");
        return (double)(columns[^1] - columns[0]) / (columns.Count - 1);
    }

    /// <summary>
    /// The angle in degrees of the groove line nearest the centre of the capture, measured over
    /// 120 rows of pixels. The line is followed ten rows at a time: a groove of <c>N</c> = 4 is
    /// 103 pixels from the next one, and ten rows move it by at most 10 pixels even at 45°, so
    /// the nearest run of the next row is always the same groove.
    /// </summary>
    private static double GrooveAngle(GlTarget target, SceneRenderer renderer, Vec3F? rotation)
    {
        const int Step = 10;
        const int Rows = 120;

        List<int> upper = MiddleRowGrooves(target, renderer, null, rotation, out Shot shot);
        int near = upper.OrderBy(static c => Math.Abs(c - (Width / 2))).First();

        int tracked = near;
        for (int row = (Height / 2) + Step; row <= (Height / 2) + Rows; row += Step)
        {
            List<(int From, int To)> runs = Runs(shot, row, tracked - 40, tracked + 40);
            Assert.NotEmpty(runs);
            tracked = runs
                .Select(static run => (run.From + run.To) / 2)
                .OrderBy(c => Math.Abs(c - tracked))
                .First();
        }

        return Math.Atan2(tracked - near, Rows) * 180.0 / Math.PI;
    }

    private static List<int> MiddleRowGrooves(
        GlTarget target,
        SceneRenderer renderer,
        Vec3F? scale,
        Vec3F? rotation,
        out Shot shot)
    {
        SceneState scene = BuildCloseUp(CloseUpFixture(scale, rotation));
        scene.Settings.Style.Enhanced = true;
        scene.Settings.Style.VoxelGrid = 2;
        shot = target.Render(renderer, scene);

        // The middle of the capture, which the camera aims at, is inside the block in every
        // variant of the criterion; a third of the width each side of it stays on the block too.
        int row = Height / 2;
        return
        [
            .. Runs(shot, row, (Width / 2) - 300, (Width / 2) + 300)
                .Select(static run => (run.From + run.To) / 2),
        ];
    }

    /// <summary>
    /// The greatest departure from its chord, in pixels, of the horizontal groove line of the
    /// front face of the backdrop at the scene coordinate <paramref name="y"/>: the line is
    /// followed column of pixels by column of pixels at the local minimum of luminance, and its
    /// chord joins its two ends in the capture (REQ-SCN-080, clause of order).
    /// </summary>
    private static double ChordDeparture(Shot shot, in CameraView view, SpaceBend bend, int y)
    {
        // The line is seeded at the middle column, where the image of the scene point
        // (128, y, −0.5) of the front face lands.
        Vec3 seed = bend.Apply(new Vec3(128f, y, -0.5f));
        (int column, int row) = Pixel(view, new Vector3(seed.X, seed.Y, seed.Z));
        Assert.InRange(column, 1, Width - 2);
        Assert.InRange(row, 1, Height - 2);

        // The two ends of the line in the capture are the two ends of the front face. Beyond
        // them the box shows its side faces, which carry grooves of their own, so the follow is
        // bounded by the image of the face's own edges — taken a unit inside, so that no column
        // of the boundary itself is read.
        int left = FaceColumn(view, bend, y, -127f);
        int right = FaceColumn(view, bend, y, 127f);
        Assert.True(left < column && column < right, $"the face of y = {y} spans [{left}, {right}].");

        List<(double Column, double Row)> line =
            [(column, Centroid(shot, column, Darkest(shot, column, row, 20)))];
        Follow(shot, line, column, -2, left, right);
        Follow(shot, line, column, 2, left, right);
        line.Sort(static (a, b) => a.Column.CompareTo(b.Column));

        Assert.True(line.Count > 20, $"the line y = {y} was followed over {line.Count} columns.");

        (double x0, double y0) = line[0];
        (double x1, double y1) = line[^1];
        double most = 0;
        foreach ((double cx, double cy) in line)
        {
            double chord = y0 + ((cx - x0) / (x1 - x0) * (y1 - y0));
            most = Math.Max(most, Math.Abs(cy - chord));
        }

        return most;
    }

    /// <summary>
    /// Follows a groove line away from its seed, one step of pixels at a time, taking at each
    /// step the darkest row inside a window of ±8 pixels around the previous one — narrower than
    /// half the 56-pixel spacing of the lines, so the follower cannot jump to a neighbour.
    /// </summary>
    private static void Follow(
        Shot shot,
        List<(double Column, double Row)> line,
        int seedColumn,
        int step,
        int left,
        int right)
    {
        // The window is centred on the row the previous slope predicts, not on the previous row:
        // near the edges of a face wrapped at 90° a groove line climbs by more than a pixel per
        // column, and a window centred on the previous row would lag behind it.
        int row = (int)Math.Round(line[0].Row);
        int previous = row;
        for (int column = seedColumn + step; column >= left && column <= right; column += step)
        {
            int predicted = row + (row - previous);
            previous = row;
            row = Darkest(shot, column, Math.Clamp(predicted, 0, shot.Height - 1), 8);
            line.Add((column, Centroid(shot, column, row)));
        }
    }

    /// <summary>
    /// The pixel column the point of the front face of the backdrop at <paramref name="u"/> of
    /// the centre and at the scene height <paramref name="y"/> lands on.
    /// </summary>
    private static int FaceColumn(in CameraView view, SpaceBend bend, int y, float u)
    {
        Vec3 edge = bend.Apply(new Vec3(128f + u, y, -0.5f));
        return Pixel(view, new Vector3(edge.X, edge.Y, edge.Z)).X;
    }

    /// <summary>The darkest row of a column inside a window about <paramref name="around"/>.</summary>
    private static int Darkest(Shot shot, int column, int around, int window)
    {
        int best = around;
        double darkest = double.MaxValue;
        for (int row = around - window; row <= around + window; row++)
        {
            if (row < 0 || row >= shot.Height)
            {
                continue;
            }

            double here = shot.Luma(column, row);
            if (here < darkest)
            {
                darkest = here;
                best = row;
            }
        }

        return best;
    }

    /// <summary>
    /// The centre of the groove to the sub-pixel, as the darkness-weighted mean of the rows
    /// around the darkest one. A groove of <c>N</c> = 8 is about four pixels wide at this
    /// framing, so the darkest <i>whole</i> row wobbles by a pixel or two from column to column
    /// and would not answer the « straight to less than one pixel » clause of the criterion.
    /// </summary>
    private static double Centroid(Shot shot, int column, int around)
    {
        const int Window = 4;
        double reference = 0;
        for (int row = around - (2 * Window); row <= around + (2 * Window); row++)
        {
            if (row >= 0 && row < shot.Height)
            {
                reference = Math.Max(reference, shot.Luma(column, row));
            }
        }

        double weighted = 0;
        double total = 0;
        for (int row = around - Window; row <= around + Window; row++)
        {
            if (row < 0 || row >= shot.Height)
            {
                continue;
            }

            double weight = Math.Max(0, reference - shot.Luma(column, row));
            weighted += weight * row;
            total += weight;
        }

        return total > 0 ? weighted / total : around;
    }

    /// <summary>
    /// One one-voxel cell of the upper face of the floor: the whole-coordinate square
    /// <c>[x, x + 1] × [z, z + 1]</c> of the plane <c>y</c> = 124, with the scene coordinate of
    /// its lower corner along the axis the run is measured on.
    /// </summary>
    private readonly record struct FloorCell(float X, float Z, int At);

    /// <summary>The cells of the floor along <c>x</c> on the row <c>[z, z + 1]</c>.</summary>
    private static List<FloorCell> FloorCellsAlongX(int z)
        => [.. Enumerable.Range(128, 16).Select(x => new FloorCell(x, z, x))];

    /// <summary>The cells of the floor along <c>z</c> on the column <c>[x, x + 1]</c>.</summary>
    private static List<FloorCell> FloorCellsAlongZ(int x)
        => [.. Enumerable.Range(0, 16).Select(z => new FloorCell(x, z, z))];

    /// <summary>
    /// The clauses of the criterion of REQ-SCN-082 on one run of cells: with the pixel shadows
    /// on, every cell is of one piece and carries the lit or the shadowed value of the
    /// criterion; the shadowed cells are one run whose ends are 6 and 14 units from the centre of
    /// the floor; and without the pixel shadows at least one cell of each end of that run holds
    /// both shadowed and lit pixels.
    /// </summary>
    private static void AssertCellsAreOfOnePiece(
        Shot hard,
        Shot soft,
        in CameraView view,
        List<FloorCell> cells,
        string axis)
    {
        // The centre of the floor is (128, 0) in the plane of its upper face.
        const int Centre = 128;
        int first = -1;
        int last = -1;
        int lit = 0;
        List<int> cut = [];

        foreach (FloorCell cell in cells)
        {
            List<(int X, int Y)> pixels = CellPixels(view, cell);
            Assert.True(pixels.Count >= 16, $"{axis} cell {cell.At} covers {pixels.Count} pixels.");

            // The cell is of one piece: the shadow was read at the centre of its voxel, the same
            // point for every one of its fragments.
            int spread = Spread(hard, pixels);
            Assert.True(spread <= 2, $"{axis} cell {cell.At} spreads over {spread} sRGB levels.");

            int level = hard.At(pixels[0].X, pixels[0].Y).R;
            if (level > 150)
            {
                lit++;
                Assert.InRange(level, 192, 198);
            }
            else
            {
                Assert.InRange(level, 115, 121);
                first = first < 0 ? cell.At : first;
                last = cell.At;
            }

            if (Spread(soft, pixels) > 2)
            {
                cut.Add(cell.At);
            }
        }

        // The boundary follows cell edges exactly, at the whole coordinates 6 and 14 units from
        // the centre of the floor: the first shadowed cell starts at +6 and the last one ends
        // at +14.
        int origin = axis == "x" ? Centre : 0;
        Assert.Equal(origin + 6, first);
        Assert.Equal(origin + 14, last + 1);
        Assert.True(lit >= 4, $"only {lit} lit cells on the {axis} run.");

        // Without the pixel shadows, the near edge of the shadow — at +5.667 of the centre of
        // the floor, in the middle of the voxel [+5, +6] — cuts that one cell in half, and it
        // alone. That is the clause that makes the criterion discriminant: the shadows of
        // REQ-SCN-030 are hard already, so the sharpness of the edge tells the two styles apart
        // on nothing, but a cell holding two values does.
        // Q-092 b: only the two **near** edges of the shadow fall in the middle of a voxel. The
        // plate is one unit thick, so its upper face casts 10 units instead of 29/3 and puts the
        // far edges at exactly +14 — on a cell edge, where no cell can be cut. The boundary at 6
        // and 14 that the criterion also gives is the one this reading produces.
        Assert.Equal<int[]>([origin + 5], [.. cut]);
    }

    /// <summary>
    /// The pixels of the capture that fall inside a one-voxel cell of the upper face of the
    /// floor, taken a fifth of a voxel inside the cell so that no border pixel is read.
    /// </summary>
    private static List<(int X, int Y)> CellPixels(in CameraView view, FloorCell cell)
    {
        const float Inset = 0.2f;
        const float Plane = 124f;

        Vector2 a = Project(view, new Vector3(cell.X + Inset, Plane, cell.Z + Inset));
        Vector2 b = Project(view, new Vector3(cell.X + 1f - Inset, Plane, cell.Z + 1f - Inset));
        Vector2 c = Project(view, new Vector3(cell.X + Inset, Plane, cell.Z + 1f - Inset));
        Vector2 d = Project(view, new Vector3(cell.X + 1f - Inset, Plane, cell.Z + Inset));

        int left = (int)Math.Ceiling(Math.Min(Math.Min(a.X, b.X), Math.Min(c.X, d.X)));
        int right = (int)Math.Floor(Math.Max(Math.Max(a.X, b.X), Math.Max(c.X, d.X)));
        int top = (int)Math.Ceiling(Math.Min(Math.Min(a.Y, b.Y), Math.Min(c.Y, d.Y)));
        int bottom = (int)Math.Floor(Math.Max(Math.Max(a.Y, b.Y), Math.Max(c.Y, d.Y)));

        List<(int X, int Y)> inside = [];
        for (int y = top; y <= bottom; y++)
        {
            for (int x = left; x <= right; x++)
            {
                if (x >= 0 && x < Width && y >= 0 && y < Height)
                {
                    inside.Add((x, y));
                }
            }
        }

        return inside;
    }

    /// <summary>The spread of the red channel over a set of pixels, in sRGB levels.</summary>
    private static int Spread(Shot shot, List<(int X, int Y)> pixels)
    {
        int least = 255;
        int most = 0;
        foreach ((int x, int y) in pixels)
        {
            int level = shot.At(x, y).R;
            least = Math.Min(least, level);
            most = Math.Max(most, level);
        }

        return most - least;
    }

    private static int Different(Shot one, Shot two)
    {
        int count = 0;
        for (int i = 0; i < one.Pixels.Length; i += 4)
        {
            if (one.Pixels[i] != two.Pixels[i]
                || one.Pixels[i + 1] != two.Pixels[i + 1]
                || one.Pixels[i + 2] != two.Pixels[i + 2])
            {
                count++;
            }
        }

        return count;
    }

    private static void AssertSame(Shot one, Shot two, int tolerance)
    {
        for (int y = 0; y < one.Height; y++)
        {
            for (int x = 0; x < one.Width; x++)
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
        }
    }
}
