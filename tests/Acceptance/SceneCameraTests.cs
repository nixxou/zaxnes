using System.Numerics;
using Zax.App.Loop;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for SPEC-06 §3.1 to §3.8: the scene frame, the projection, the two
/// camera modes, the corrected aim point, the view reset, the user controls, the smoothing, the
/// subjective mode and the saved camera state.
/// </summary>
public sealed class SceneCameraTests
{
    private static readonly Vec4F DefaultClipping = new(0.1f, 8.1f, 8.1f, 0.1f);
    private static readonly Vec4F MarioClipping = new(0f, 7.9f, 0f, 0.1f);

    private static SceneCamera Settled(float fov = 70f, Vec4F? clipping = null)
    {
        SceneCamera camera = new() { Fov = fov };
        camera.ResetView();
        camera.SnapToSetpoints();
        _ = clipping;
        return camera;
    }

    /// <summary>Advances the camera by <paramref name="seconds"/> in 600 Hz steps.</summary>
    private static void Advance(SceneCamera camera, float seconds)
    {
        int ticks = (int)MathF.Round(seconds * 600f);
        for (int tick = 0; tick < ticks; tick++)
        {
            camera.Advance(1f / 600f);
        }
    }

    // ------------------------------------------------- §3.1 frame, scale, visible rectangle

    /// <summary>
    /// REQ-SCN-001 — the NES pixel of column <c>px</c> and line <c>py</c> occupies
    /// <c>[px, px + 1] × [247 − py, 248 − py]</c> in the plane <c>z = 0</c>: the reference frame
    /// of SPEC-04B REQ-E3D-410, with the NES image in <c>y ∈ [8, 248]</c>.
    /// </summary>
    [Fact] // REQ-SCN-001
    public void The_scene_frame_places_the_NES_image_in_y_from_8_to_248()
    {
        Assert.Equal(248f, SceneConstants.SceneHeight);
        Assert.Equal(256f, SceneConstants.FrameWidth);
        Assert.Equal(240f, SceneConstants.FrameHeight);
        Assert.Equal(new Vector3(128f, 120f, 0f), SceneConstants.Center);

        // A background tile posted at NES (0, 8) covers [0, 8] × [232, 240].
        SceneFixture fixture = new();
        fixture.Pattern(SceneFixture.Solid(1), depth: 1, ProfileLayer.L1).Tile(SceneFixture.Solid(1), 0, 8);
        SceneState scene = fixture.Build();

        DrawTile tile = Assert.Single(scene.DrawList!.Tiles);

        // The square the tile covers runs from the lower corner BL to the upper corner TR + 1
        // (SPEC-04 REQ-E3D-205, TV-LUA-05): x from 0 to 8 and y from 232 to 240.
        Assert.Equal(0f, tile.Instance.BL.X, 3);
        Assert.Equal(232f, tile.Instance.BL.Y, 3);
        Assert.Equal(8f, tile.Instance.TR.X + 1f, 3);
        Assert.Equal(240f, tile.Instance.TR.Y + 1f, 3);
    }

    /// <summary>
    /// REQ-SCN-002 — on desktop the scene scale is 1: <c>Video.SceneScale</c> reads 1 and a
    /// write has no geometric effect. The view-reset half is verified in
    /// <see cref="LuaApiTests"/>.
    /// </summary>
    [Fact] // REQ-SCN-002
    public void The_scene_scale_is_one_on_desktop()
    {
        using LuaRun run = new LuaProfile().Script("a", string.Empty).Start();
        run.Do("Video.SceneScale = 0.02");
        Assert.Equal(1, run.Num("Video.SceneScale"));
        Assert.Equal(1, run.Num("Setting.SceneScale"));
        Assert.Equal(1, run.Num("Setting.SceneRatio"));
    }

    /// <summary>
    /// REQ-SCN-003, REQ-SCN-039 — the visible rectangle is
    /// <c>R = [c0, 256 − c2] × [c1, 240 − c3]</c>, recomputed from the current margins, and the
    /// default margins outside a profile are <c>(0.1 ; 8.1 ; 8.1 ; 0.1)</c>.
    /// </summary>
    [Fact] // REQ-SCN-003, REQ-SCN-039
    public void The_visible_rectangle_comes_from_the_four_clip_margins()
    {
        Assert.Equal(new Vec4F(0.1f, 8.1f, 8.1f, 0.1f), ConversionSettings.DefaultClipping);

        VisibleRect mario = VisibleRect.From(MarioClipping);
        Assert.Equal(0f, mario.MinX, 4);
        Assert.Equal(7.9f, mario.MinY, 4);
        Assert.Equal(256f, mario.MaxX, 4);
        Assert.Equal(239.9f, mario.MaxY, 4);

        VisibleRect defaults = VisibleRect.From(DefaultClipping);
        Assert.Equal(0.1f, defaults.MinX, 4);
        Assert.Equal(247.9f, defaults.MaxX, 4);

        // A scene recomputes R from the current margins at every rendered frame.
        SceneState scene = new() { Clipping = MarioClipping };
        Assert.Equal(7.9f, scene.VisibleRectangle.MinY, 4);
        scene.Clipping = new Vec4F(16.1f, 8.1f, 8.1f, 0.1f);
        Assert.Equal(16.1f, scene.VisibleRectangle.MinX, 4);
    }

    /// <summary>
    /// REQ-SCN-003 — the margins come from the profile at game load and Lua can change them
    /// during the game.
    /// </summary>
    [Fact] // REQ-SCN-003
    public void The_clip_margins_come_from_the_profile_and_Lua_can_change_them()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Settings!.Clipping = new Vec4F(0f, 7.9f, 0f, 0.1f);
        profile.Scripts.Add(new ProfileScript(
            "a", true, "function Start() Setting.Clipping[0] = 16.1 end", "*"));

        SceneState scene = new();
        using SessionDriver session = SessionDriver.CreateOnProfile(profile, scene);

        // At load, before Start: the profile's margins are in the scene.
        Assert.Equal(0f, scene.Clipping.X, 4);
        Assert.Equal(7.9f, scene.Clipping.Y, 4);

        session.ApplyProfileSettings(scene, profile.Settings);
        Assert.Equal(16.1f, scene.Clipping.X, 4);
    }

    // ------------------------------------------------------------- §3.2 projection

    /// <summary>
    /// REQ-SCN-004 — the projection is perspective with the FOV as its <b>vertical</b> angle, a
    /// near plane at 0.3 and a far plane at 5000, and the aspect ratio of the render target
    /// without letterboxing: in 16:9 at FOV 70 and distance 189.65, the visible height in the
    /// NES plane is 265.6 units and the width 472.2 (±1 %).
    /// </summary>
    [Fact] // REQ-SCN-004
    public void The_projection_uses_the_vertical_field_of_view_and_the_target_aspect()
    {
        Assert.Equal(0.3f, SceneConstants.NearPlane);
        Assert.Equal(5000f, SceneConstants.FarPlane);

        const float fov = 70f;
        const float aspect = 16f / 9f;
        const float distance = 189.65f;

        double height = 2.0 * distance * Math.Tan(fov * Math.PI / 180.0 / 2.0);
        double width = height * aspect;

        Assert.Equal(265.6, height, 265.6 * 0.01);
        Assert.Equal(472.2, width, 472.2 * 0.01);

        // The matrix the camera builds agrees with that: the half-height at z = 1 is tan(FOV/2).
        SceneCamera camera = Settled(fov);
        CameraView view = camera.BuildView(DefaultClipping, aspect);
        Matrix4x4 projection = view.Projection;

        Assert.Equal(1.0 / Math.Tan(fov * Math.PI / 180.0 / 2.0), projection.M22, 1e-4);
        Assert.Equal(projection.M22 / aspect, projection.M11, 1e-4);
        Assert.Equal(fov, view.Fov);
        Assert.Equal(aspect, view.Aspect);
    }

    /// <summary>
    /// REQ-SCN-005 — the FOV is 70° by default, takes the values 45 to 95 by steps of 5, applies
    /// at once and triggers a view reset.
    /// </summary>
    [Fact] // REQ-SCN-005, REQ-SCN-013
    public void The_field_of_view_defaults_to_70_is_bounded_and_resets_the_view()
    {
        SceneCamera camera = new();
        Assert.Equal(70f, camera.Fov);
        Assert.Equal(70f, SceneConstants.DefaultFov);
        Assert.Equal(45f, SceneConstants.MinFov);
        Assert.Equal(95f, SceneConstants.MaxFov);

        for (int step = -5; step <= 5; step++)
        {
            float wanted = 70f + (5f * step);
            camera.Fov = wanted;
            Assert.Equal(wanted, camera.Fov);
            Assert.Equal(SceneConstants.ResetDistance(wanted), camera.Current.Distance, 2);
        }

        camera.Fov = 10f;
        Assert.Equal(45f, camera.Fov);
        camera.Fov = 200f;
        Assert.Equal(95f, camera.Fov);

        // Changing the FOV resets the view: the angles go back to the default.
        camera.Fov = 70f;
        camera.Current.Angles = new Vector3(45f, 45f, 0f);
        camera.Fov = 90f;
        Assert.Equal(SceneConstants.DefaultAngles, camera.Current.Angles);
        Assert.Equal(132.80f, camera.Current.Distance, 0.05f);
    }

    // ----------------------------------------------- §3.3 camera modes and placement

    /// <summary>
    /// REQ-SCN-006 — two modes with the numeric values Lua sees, each with its own setpoints, and
    /// a change of mode opens a 3-second transition window.
    /// </summary>
    [Fact] // REQ-SCN-006, REQ-SCN-020
    public void Each_mode_keeps_its_own_setpoints_and_a_change_opens_the_transition_window()
    {
        Assert.Equal(0, (int)CameraMode.Tpv);
        Assert.Equal(1, (int)CameraMode.Fpv);
        Assert.Equal(3f, SceneConstants.ModeTransitionSeconds);

        SceneCamera camera = new();
        Assert.Equal(CameraMode.Tpv, camera.Mode);

        camera.Current.Distance = 150f;
        camera.SetMode(CameraMode.Fpv);
        Assert.Equal(3f, camera.TransitionRemaining, 4);

        camera.Current.Distance = 40f;
        camera.SetMode(CameraMode.Tpv);
        Assert.Equal(150f, camera.Current.Distance, 3);
        Assert.Equal(40f, camera.Setpoints(CameraMode.Fpv).Distance, 3);

        // Setting the same mode again changes nothing.
        camera.Advance(5f);
        camera.SetMode(CameraMode.Tpv);
        Assert.Equal(0f, camera.TransitionRemaining, 4);
    }

    /// <summary>
    /// REQ-SCN-007, REQ-SCN-008, §5.1 — the vectors C1, C2 and C3: the eye, the effective target
    /// and the corrected aim point.
    /// </summary>
    [Theory] // REQ-SCN-007, REQ-SCN-008
    [InlineData("C1", 0.1f, 8.1f, 8.1f, 0.1f, 0.81f, 0.81f, 124f, 124f, 121.32f, 126.68f, -189.61f, 123.4f, 124.6f)]
    [InlineData("C2", 0f, 7.9f, 0f, 0.1f, 0.81f, 0.81f, 128f, 123.9f, 125.32f, 126.58f, -189.61f, 127.3f, 124.5f)]
    [InlineData("C3", 0f, 7.9f, 0f, 0.1f, 30f, -20f, 128f, 123.9f, 184.17f, 218.73f, -154.34f, 145.1f, 145.7f)]
    public void The_camera_vectors_of_section_5_1(
        string name,
        float c0, float c1, float c2, float c3,
        float rx, float ry,
        float targetX, float targetY,
        float eyeX, float eyeY, float eyeZ,
        float aimX, float aimY)
    {
        Vec4F clipping = new(c0, c1, c2, c3);
        SceneCamera camera = new() { Fov = 70f };
        camera.ResetView();
        camera.Current.Angles = new Vector3(rx, ry, 0f);
        camera.SnapToSetpoints();

        Assert.Equal(189.65f, camera.Distance, 0.05f);

        CameraView view = camera.BuildView(clipping, 16f / 9f);

        Assert.Equal(targetX, view.Target.X, 0.5f);
        Assert.Equal(targetY, view.Target.Y, 0.5f);
        Assert.Equal(eyeX, view.Eye.X, 0.5f);
        Assert.Equal(eyeY, view.Eye.Y, 0.5f);
        Assert.Equal(eyeZ, view.Eye.Z, 0.5f);
        Assert.Equal(aimX, view.AimPoint.X, 0.6f);
        Assert.Equal(aimY, view.AimPoint.Y, 0.6f);
        Assert.Equal(0f, view.AimPoint.Z, 1e-4f);
        _ = name;
    }

    /// <summary>
    /// REQ-SCN-007 — <c>f = q · (0, 0, 1)</c> is
    /// <c>(cos rx · sin ry, −sin rx, cos rx · cos ry)</c> when <c>rz = 0</c>, so <c>rx &gt; 0</c>
    /// puts the camera above the screen and <c>ry &gt; 0</c> to its left.
    /// </summary>
    [Fact] // REQ-SCN-007
    public void The_forward_vector_follows_the_documented_formula()
    {
        foreach ((float rx, float ry) in new[] { (0f, 0f), (30f, -20f), (-15f, 40f), (60f, 120f) })
        {
            Vector3 forward = SceneConstants.Forward(new Vector3(rx, ry, 0f), localCamera: false);
            double x = Math.Cos(rx * Math.PI / 180.0) * Math.Sin(ry * Math.PI / 180.0);
            double y = -Math.Sin(rx * Math.PI / 180.0);
            double z = Math.Cos(rx * Math.PI / 180.0) * Math.Cos(ry * Math.PI / 180.0);

            Assert.Equal(x, forward.X, 1e-4);
            Assert.Equal(y, forward.Y, 1e-4);
            Assert.Equal(z, forward.Z, 1e-4);
        }

        // rx > 0 raises the eye, ry > 0 moves it to the camera's left (positive x).
        SceneCamera camera = Settled();
        camera.Current.Angles = new Vector3(30f, 20f, 0f);
        camera.SnapToSetpoints();
        CameraView view = camera.BuildView(MarioClipping, 16f / 9f);
        Assert.True(view.Eye.Y > SceneConstants.Center.Y, "rx > 0 must raise the eye.");
        Assert.True(view.Eye.X < SceneConstants.Center.X, "ry > 0 must move the eye left of the screen.");
    }

    /// <summary>
    /// REQ-SCN-008 — the bisector of a segment seen from the eye lands at
    /// <c>P + (Q − P) · |P − E| / (|P − E| + |Q − E|)</c>, and the degenerate case gives the
    /// midpoint.
    /// </summary>
    [Fact] // REQ-SCN-008
    public void The_bisector_formula_and_its_degenerate_case()
    {
        Vector3 p = new(0f, 0f, 0f);
        Vector3 q = new(10f, 0f, 0f);
        Vector3 eye = new(0f, 0f, -10f);

        Vector3 m = SceneConstants.Bisector(p, q, eye);
        double dp = Vector3.Distance(p, eye);
        double dq = Vector3.Distance(q, eye);
        Assert.Equal(dp / (dp + dq) * 10.0, m.X, 1e-3);

    }

    /// <summary>
    /// REQ-SCN-008, last sentence of the criterion — "for the degenerate cases (the eye
    /// coinciding with a vertex) the point of the segment is its midpoint".
    /// </summary>
    /// <remarks>
    /// Round 1: the implementation fell back to the midpoint only when <b>both</b> distances were
    /// zero, that is when the segment itself was a point; with the eye on one vertex the ratio
    /// <c>|P − E| / (|P − E| + |Q − E|)</c> was 0 and the bisector landed on that vertex
    /// (<c>verify/round1/R1-report-part-b.md</c> §2, <c>Q-046</c>). AMD-019 confirms the reading —
    /// « au moins une des deux distances nulle » — and the <c>R1-fixes</c> batch corrects
    /// <c>SceneConstants.Bisector</c>. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Fact] // REQ-SCN-008 — AMD-019 (Q-046)
    public void The_degenerate_bisector_must_be_the_midpoint_of_the_segment()
    {
        Vector3 p = new(0f, 0f, 0f);
        Vector3 q = new(10f, 0f, 0f);

        Assert.Equal(5f, SceneConstants.Bisector(p, q, p).X, 1e-4f);
        Assert.Equal(5f, SceneConstants.Bisector(p, q, q).X, 1e-4f);

        // The degenerate case is "at least one distance zero", so it holds off the segment's own
        // line as well, and the general formula is untouched elsewhere (the eye at equal
        // distance from both ends gives the midpoint too, for a different reason).
        Assert.Equal(5f, SceneConstants.Bisector(p, q, new Vector3(0f, 0f, 0f)).X, 1e-4f);
        Assert.Equal(5f, SceneConstants.Bisector(p, q, new Vector3(5f, 7f, 0f)).X, 1e-4f);
        Assert.Equal(10f * 2f / 14f, SceneConstants.Bisector(p, q, new Vector3(-2f, 0f, 0f)).X, 1e-4f);
    }

    /// <summary>
    /// REQ-SCN-009 — the local camera replaces the rotation by <c>R_Z(ry) · R_X(rx)</c>, drops
    /// the aim at <c>P*</c> and keeps the recentred target in TPV.
    /// </summary>
    [Fact] // REQ-SCN-009
    public void The_local_camera_rotates_the_image_around_the_line_of_sight()
    {
        SceneCamera camera = Settled();
        camera.LocalCamera = true;
        camera.Current.Angles = new Vector3(0f, 90f, 0f);
        camera.SnapToSetpoints();

        CameraView view = camera.BuildView(DefaultClipping, 16f / 9f);

        // Looking exactly along q·(0,0,1) = +Z, with the vertical q·(0,1,0) turned a quarter turn.
        Assert.Equal(0f, view.Forward.X, 1e-4f);
        Assert.Equal(0f, view.Forward.Y, 1e-4f);
        Assert.Equal(1f, view.Forward.Z, 1e-4f);
        Assert.Equal(-1f, view.Up.X, 1e-4f);
        Assert.Equal(0f, view.Up.Y, 1e-4f);

        // The target is still recentred on the visible rectangle in TPV.
        Assert.Equal(124f, view.Target.X, 0.5f);
        Assert.Equal(124f, view.Target.Y, 0.5f);

        // The default rotation is R_Y(ry)·R_X(rx)·R_Z(rz), the local one R_Z(ry)·R_X(rx).
        (Vector3 right, Vector3 up, Vector3 forward) =
            SceneConstants.Rotation(new Vector3(0f, 90f, 0f), localCamera: true);
        Assert.Equal(new Vector3(0f, 0f, 1f), Round(forward));
        Assert.Equal(new Vector3(-1f, 0f, 0f), Round(up));
        Assert.Equal(new Vector3(0f, 1f, 0f), Round(right));
    }

    /// <summary>
    /// REQ-SCN-072 — in TPV the vertical is the camera's own, <c>q·(0, 1, 0)</c>, and no longer
    /// the world's Y axis: the basis stays continuous across <c>rx = 90°</c> and <c>rz</c> rolls
    /// the picture as it does in FPV.
    /// </summary>
    [Fact] // REQ-SCN-072
    public void In_TPV_the_vertical_is_the_camera_own_so_rz_rolls_the_picture()
    {
        // Criterion 1: between rx = 89.5° and rx = 90.5° the orientation varies by at most 2°
        // — with the world vertical the basis flipped by about 180° across the pole.
        CameraView below = Aimed(new Vector3(89.5f, 0f, 0f));
        CameraView above = Aimed(new Vector3(90.5f, 0f, 0f));

        Assert.InRange(Between(below.Up, above.Up), 0f, 2f);
        Assert.InRange(Between(below.Right, above.Right), 0f, 2f);
        Assert.InRange(Between(below.Forward, above.Forward), 0f, 2f);

        // Criterion 2: rx = ry = 0 and rz = 30° turn the picture by 30° about the line of sight.
        CameraView straight = Aimed(Vector3.Zero);
        CameraView rolled = Aimed(new Vector3(0f, 0f, 30f));

        Assert.Equal(30f, Between(straight.Up, rolled.Up), 0.5f);
        Assert.Equal(30f, Between(straight.Right, rolled.Right), 0.5f);
        Assert.InRange(Between(straight.Forward, rolled.Forward), 0f, 0.5f);

        // The vertical really is q·(0, 1, 0), bar the orthogonalisation against the aim at P*:
        // the two agree to a degree.
        (_, Vector3 qUp, _) = SceneConstants.Rotation(new Vector3(0f, 0f, 30f), localCamera: false);
        Assert.InRange(Between(qUp, rolled.Up), 0f, 1f);

        static CameraView Aimed(Vector3 angles)
        {
            SceneCamera camera = Settled();
            camera.Current.Angles = angles;
            camera.SnapToSetpoints();
            return camera.BuildView(DefaultClipping, 16f / 9f);
        }
    }

    /// <summary>The angle between two vectors, in degrees.</summary>
    private static float Between(Vector3 first, Vector3 second)
    {
        float cosine = Vector3.Dot(Vector3.Normalize(first), Vector3.Normalize(second));
        return MathF.Acos(Math.Clamp(cosine, -1f, 1f)) * 180f / MathF.PI;
    }

    /// <summary>
    /// REQ-SCN-010 — in FPV the eye is <c>E = T − d · f</c> with no recentring, the orientation
    /// is exactly <c>q</c> — roll included — and there is no aim at <c>P*</c>.
    /// </summary>
    [Fact] // REQ-SCN-010
    public void In_FPV_the_camera_looks_exactly_along_q_with_the_roll()
    {
        SceneCamera camera = new() { Fov = 70f };
        camera.ResetView();
        camera.SetMode(CameraMode.Fpv);
        camera.Current.Distance = 100f;
        camera.Current.Angles = Vector3.Zero;
        camera.Current.Target = new Vector3(128f, 120f, 0f);
        camera.SnapToSetpoints();

        CameraView straight = camera.BuildView(DefaultClipping, 16f / 9f);
        Assert.Equal(128f, straight.Target.X, 1e-3f);
        Assert.Equal(120f, straight.Target.Y, 1e-3f);
        Assert.Equal(128f, straight.Eye.X, 1e-3f);
        Assert.Equal(-100f, straight.Eye.Z, 1e-3f);
        Assert.Equal(straight.Eye + straight.Forward, straight.AimPoint);

        // A roll of 90 degrees: q = R_Z(90) sends the vertical (0, 1, 0) onto (-1, 0, 0), that is
        // a quarter turn of the image (REQ-SCN-010).
        camera.Current.Angles = new Vector3(0f, 0f, 90f);
        camera.SnapToSetpoints();
        CameraView rolled = camera.BuildView(DefaultClipping, 16f / 9f);
        Assert.Equal(-1f, rolled.Up.X, 1e-4f);
        Assert.Equal(0f, rolled.Up.Y, 1e-4f);
        Assert.Equal(0f, rolled.Forward.X, 1e-4f);
        Assert.Equal(1f, rolled.Forward.Z, 1e-4f);
    }

    /// <summary>
    /// REQ-SCN-011 — the setpoint distance is bounded to <c>[0, 512]</c>. The user may still
    /// orbit without limit, but AMD-045 REQ-SCN-071 replaces the criterion « rx past 360° after
    /// 13 s »: the angle comes back into <c>]−180°, 180°]</c> at every update, so 390° reads
    /// about 30.
    /// </summary>
    [Fact] // REQ-SCN-011, REQ-SCN-071
    public void The_distance_is_bounded_and_the_angles_stay_inside_one_turn()
    {
        Assert.Equal(512f, SceneConstants.MaxDistance);

        SceneCamera camera = new();
        camera.Current.Distance = 1000f;
        Assert.Equal(512f, camera.Current.Distance);
        camera.Current.Distance = -5f;
        Assert.Equal(0f, camera.Current.Distance);

        // Holding the rotation down for 13 s is 390° of orbit.
        camera.Current.Angles = Vector3.Zero;
        camera.SnapToSetpoints();
        for (int tick = 0; tick < 13 * 60; tick++)
        {
            CameraControls.Apply(camera, CameraAction.RotateDown, 1f / 60f);
            camera.Advance(1f / 60f);
        }

        Assert.InRange(camera.Current.Angles.X, -180f, 180f);
        Assert.Equal(30f, camera.Current.Angles.X, 0.5f);
    }

    // ------------------------------------------- §3.4 view reset and default values

    /// <summary>
    /// REQ-SCN-012 — the view reset applies the current FOV, puts the angles of <b>both</b> modes
    /// at (0.81°, 0.81°, 0) and their target at <c>C</c>, and the distance of the <b>current</b>
    /// mode only at <c>d₀</c>, which AMD-045 REQ-SCN-069 now computes as
    /// <c>132.7964 / tan(FOV / 2) × s</c>.
    /// </summary>
    [Theory] // REQ-SCN-012, REQ-SCN-069
    [InlineData(70f, 189.65f)]
    [InlineData(45f, 320.60f)]
    [InlineData(95f, 121.69f)]
    public void The_reset_distance_formula(float fov, float expected)
        => Assert.Equal(expected, SceneConstants.ResetDistance(fov), 0.05f);

    /// <summary>
    /// REQ-SCN-069 — at the default FOV of 70° the new formula still agrees with the old
    /// <c>104 / atan(FOV_rad / 2)</c> to 10⁻³, which is why the default framing does not move:
    /// 189.65291 against 189.65285. The reset writes it through the <c>[0, 512]</c> bound of
    /// REQ-SCN-011.
    /// </summary>
    [Fact] // REQ-SCN-069
    public void The_reset_distance_matches_the_old_formula_at_seventy_degrees_and_is_bounded()
    {
        Assert.Equal(132.7964f, SceneConstants.ResetHalfHeight);

        double old = 104.0 / Math.Atan(70.0 * Math.PI / 180.0 / 2.0);
        double now = 132.7964 / Math.Tan(70.0 * Math.PI / 180.0 / 2.0);
        Assert.Equal(189.65285, old, 1e-4);
        Assert.Equal(189.65291, now, 1e-4);
        Assert.True(Math.Abs(now - old) < 1e-3, $"{now} against {old}.");

        // At d₀ the framed half-height fills exactly half the vertical field of view.
        foreach (float fov in new[] { 45f, 70f, 95f })
        {
            float half = SceneConstants.ResetDistance(fov)
                * MathF.Tan(fov * MathF.PI / 180f / 2f);
            Assert.Equal(SceneConstants.ResetHalfHeight, half, 1e-2f);
        }

        // d₀ passes 512 under a FOV of about 29.1° — a FOV only a script can post, the UI
        // stopping at 45 (SPEC-09 camera.fov) — and the setpoint brings it back in.
        Assert.Equal(SceneConstants.MaxDistance, SceneConstants.ResetDistance(29.07f), 0.5f);
        Assert.True(
            SceneConstants.ResetDistance(29f) > SceneConstants.MaxDistance,
            $"d₀ at 29° is {SceneConstants.ResetDistance(29f)}.");

        CameraSetpoint setpoint = new() { Distance = SceneConstants.ResetDistance(29f) };
        Assert.Equal(SceneConstants.MaxDistance, setpoint.Distance);
    }

    /// <summary>
    /// REQ-SCN-012 — the reset touches both modes' angles and target but only the current mode's
    /// distance, and the FPV distance is 0 at application launch.
    /// </summary>
    [Fact] // REQ-SCN-012
    public void The_view_reset_leaves_the_other_mode_distance_alone()
    {
        SceneCamera camera = new();

        // At launch: TPV is the current mode, the FPV distance is 0.
        Assert.Equal(CameraMode.Tpv, camera.Mode);
        Assert.Equal(0f, camera.Setpoints(CameraMode.Fpv).Distance);
        Assert.Equal(SceneConstants.ResetDistance(70f), camera.Setpoints(CameraMode.Tpv).Distance, 2);

        camera.Setpoints(CameraMode.Fpv).Distance = 42f;
        camera.Setpoints(CameraMode.Fpv).Angles = new Vector3(10f, 10f, 10f);
        camera.Setpoints(CameraMode.Tpv).Angles = new Vector3(20f, 20f, 20f);
        camera.Setpoints(CameraMode.Tpv).Target = new Vector3(1f, 2f, 3f);

        camera.ResetView();

        Assert.Equal(SceneConstants.DefaultAngles, camera.Setpoints(CameraMode.Tpv).Angles);
        Assert.Equal(SceneConstants.DefaultAngles, camera.Setpoints(CameraMode.Fpv).Angles);
        Assert.Equal(SceneConstants.Center, camera.Setpoints(CameraMode.Tpv).Target);
        Assert.Equal(SceneConstants.Center, camera.Setpoints(CameraMode.Fpv).Target);
        Assert.Equal(SceneConstants.ResetDistance(70f), camera.Setpoints(CameraMode.Tpv).Distance, 2);
        Assert.Equal(42f, camera.Setpoints(CameraMode.Fpv).Distance, 3);
    }

    /// <summary>
    /// REQ-SCN-013 — the end of a game must reset the view and <b>in addition</b> go back to
    /// TPV, put the smoothing time back to 0.4 s and the game speed back to normal.
    /// </summary>
    /// <remarks>
    /// Round 1: the host put the smoothing time and the speed back but neither reset the view nor
    /// returned to TPV when a game stopped — the camera kept its mode, angles and distance
    /// (<c>verify/round1/R1-report-part-b.md</c> §2). Corrected by the <c>R1-fixes</c> batch —
    /// <c>EmulationHost</c> calls <c>SceneCamera.EndGame()</c> when the game stops. Re-verified
    /// 2026-10-21, <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-013
    public void The_end_of_a_game_must_reset_the_view_and_go_back_to_TPV()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.Scene.Camera.SmoothTime = 2f;
        host.Settings.GameSpeed = 4;
        host.Scene.Camera.SetMode(CameraMode.Fpv);
        host.Scene.Camera.Current.Distance = 42f;
        host.Scene.Camera.Current.Angles = new Vector3(45f, 45f, 0f);

        host.Run(ZaxCommands.Stop);

        Assert.Equal(EmulationState.Stopped, host.State);

        // The smoothing time and the speed.
        Assert.Equal(0.4f, host.Scene.Camera.SmoothTime, 4);
        Assert.Equal(0, host.Settings.GameSpeed);

        // The view reset and the return to TPV.
        Assert.Equal(CameraMode.Tpv, host.Scene.Camera.Mode);
        Assert.Equal(SceneConstants.DefaultAngles, host.Scene.Camera.Current.Angles);
        Assert.Equal(
            SceneConstants.ResetDistance(host.Settings.Fov),
            host.Scene.Camera.Current.Distance,
            2);
    }

    /// <summary>
    /// REQ-SCN-013 — the triggers the camera itself owns: a view reset puts the default view
    /// back, and <c>EndGame</c> adds the return to TPV and the 0.4 s smoothing time.
    /// </summary>
    [Fact] // REQ-SCN-013
    public void The_camera_end_of_game_operation_does_what_the_requirement_asks()
    {
        SceneCamera camera = new();
        camera.SetMode(CameraMode.Fpv);
        camera.SmoothTime = 2f;
        camera.Current.Angles = new Vector3(45f, 45f, 0f);
        camera.Current.Distance = 42f;

        camera.EndGame();

        Assert.Equal(CameraMode.Tpv, camera.Mode);
        Assert.Equal(0.4f, camera.SmoothTime, 4);
        Assert.Equal(SceneConstants.DefaultAngles, camera.Current.Angles);
        Assert.Equal(SceneConstants.ResetDistance(70f), camera.Current.Distance, 2);
        Assert.Equal(0f, camera.TransitionRemaining, 4);
    }

    /// <summary>
    /// REQ-SCN-013 — a long press on Menu resets the view without changing the pause state, and
    /// a short press toggles it (REQ-LOOP-005).
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-013, REQ-LOOP-005
    public void A_long_press_on_Menu_resets_the_view_without_pausing()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.Scene.Camera.Current.Angles = new Vector3(45f, 45f, 0f);

        // Held 0.5 s, then released: a long press (REQ-LOOP-005).
        host.Tick(0.1, Zax.App.Input.InputAction.Menu);
        host.Tick(0.4, Zax.App.Input.InputAction.Menu);
        host.Tick(0.05);
        host.ApplyCommands();

        Assert.Equal(EmulationState.Running, host.State);
        Assert.Equal(SceneConstants.DefaultAngles, host.Scene.Camera.Current.Angles);
    }

    // --------------------------------------------------------- §3.5 user controls

    /// <summary>
    /// REQ-SCN-014 — the rotation actions move the setpoint angles by 30° per second of real
    /// time, and opposite actions do not cancel out: RotLeft wins over RotRight, RotDown over
    /// RotUp.
    /// </summary>
    [Fact] // REQ-SCN-014
    public void The_rotation_actions_turn_at_thirty_degrees_a_second()
    {
        Assert.Equal(30f, SceneConstants.RotationDegreesPerSecond);

        foreach (int rate in new[] { 30, 60, 144 })
        {
            SceneCamera camera = new();
            camera.Current.Angles = Vector3.Zero;
            for (int tick = 0; tick < 3 * rate; tick++)
            {
                CameraControls.Apply(camera, CameraAction.RotateRight, 1f / rate);
            }

            Assert.Equal(90f, camera.Current.Angles.Y, 90f * 0.02f);
        }

        SceneCamera both = new();
        both.Current.Angles = Vector3.Zero;
        CameraControls.Apply(both, CameraAction.RotateLeft | CameraAction.RotateRight, 1f);
        Assert.Equal(-30f, both.Current.Angles.Y, 1e-3f);

        CameraControls.Apply(both, CameraAction.RotateUp | CameraAction.RotateDown, 1f);
        Assert.Equal(30f, both.Current.Angles.X, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-015 — ZoomIn and ZoomOut move the setpoint distance by 70 units per second, and
    /// ZoomIn wins when both are held.
    /// </summary>
    [Fact] // REQ-SCN-015
    public void The_zoom_actions_move_seventy_units_a_second()
    {
        Assert.Equal(70f, SceneConstants.ZoomUnitsPerSecond);

        SceneCamera camera = Settled();
        float before = camera.Current.Distance;
        for (int tick = 0; tick < 60; tick++)
        {
            CameraControls.Apply(camera, CameraAction.ZoomIn, 1f / 60f);
        }

        Assert.Equal(before - 70f, camera.Current.Distance, 70f * 0.02f);
        Assert.Equal(119.65f, camera.Current.Distance, 119.65f * 0.02f);

        CameraControls.Apply(camera, CameraAction.ZoomIn | CameraAction.ZoomOut, 1f);
        Assert.Equal(119.65f - 70f, camera.Current.Distance, 2f);
    }

    /// <summary>
    /// REQ-SCN-016 — with no zoom action held, one wheel notch forward lowers the setpoint
    /// distance by 5 units, independently of the frame rate.
    /// </summary>
    [Fact] // REQ-SCN-016
    public void The_mouse_wheel_moves_five_units_a_notch()
    {
        Assert.Equal(5f, SceneConstants.WheelUnitsPerNotch);

        SceneCamera camera = Settled();
        CameraControls.ApplyWheel(camera, 10f, CameraAction.None);
        Assert.Equal(139.65f, camera.Current.Distance, 1f);

        // A held zoom action takes precedence: the wheel is ignored.
        float frozen = camera.Current.Distance;
        CameraControls.ApplyWheel(camera, 10f, CameraAction.ZoomIn);
        Assert.Equal(frozen, camera.Current.Distance, 1e-4f);
    }

    /// <summary>
    /// REQ-SCN-070 — a user zoom in never takes the setpoint distance under <c>16 × s</c>:
    /// holding ZoomIn 5 s from the default view stops at 16, a distance already below is left
    /// alone, and zooming out is not concerned.
    /// </summary>
    [Fact] // REQ-SCN-070
    public void A_user_zoom_in_never_goes_under_sixteen_units()
    {
        Assert.Equal(16f, SceneConstants.MinUserDistance);

        // Criterion 1: ZoomIn held 5 s from the default view ⇒ 16.
        SceneCamera camera = Settled();
        for (int tick = 0; tick < 5 * 60; tick++)
        {
            CameraControls.Apply(camera, CameraAction.ZoomIn, 1f / 60f);
        }

        Assert.Equal(16f, camera.Current.Distance, 1e-3f);

        // Criterion 2: a distance a script posted below the floor is left exactly as it is.
        camera.Current.Distance = 5f;
        CameraControls.Apply(camera, CameraAction.ZoomIn, 1f / 60f);
        for (int tick = 0; tick < 60; tick++)
        {
            CameraControls.Apply(camera, CameraAction.ZoomIn, 1f / 60f);
        }

        Assert.Equal(5f, camera.Current.Distance, 1e-3f);

        // Criterion 3: ZoomOut for 1 s from there ⇒ 75, the floor has nothing to say about it.
        for (int tick = 0; tick < 60; tick++)
        {
            CameraControls.Apply(camera, CameraAction.ZoomOut, 1f / 60f);
        }

        Assert.Equal(75f, camera.Current.Distance, 75f * 0.02f);

        // The wheel is a user zoom too: forward notches stop at the floor, backward ones do not.
        camera.Current.Distance = 20f;
        CameraControls.ApplyWheel(camera, 10f, CameraAction.None);
        Assert.Equal(16f, camera.Current.Distance, 1e-3f);
        CameraControls.ApplyWheel(camera, 2f, CameraAction.None);
        Assert.Equal(16f, camera.Current.Distance, 1e-3f);
        CameraControls.ApplyWheel(camera, -2f, CameraAction.None);
        Assert.Equal(26f, camera.Current.Distance, 1e-3f);

        // The FPV distance of 0 at launch is untouched by a zoom in (REQ-SCN-012).
        SceneCamera launch = new();
        launch.SetMode(CameraMode.Fpv);
        Assert.Equal(0f, launch.Current.Distance);
        CameraControls.Apply(launch, CameraAction.ZoomIn, 1f);
        Assert.Equal(0f, launch.Current.Distance);
    }

    /// <summary>
    /// REQ-SCN-070 — a Lua write to <c>Camera.Distance</c> keeps its own bounds
    /// <c>[0, 512 × s]</c>: the zoom floor is only for the user commands (REQ-LUA-088).
    /// </summary>
    [Fact] // REQ-SCN-070, REQ-LUA-088
    public void The_zoom_floor_does_not_apply_to_a_lua_write()
    {
        SceneCamera camera = Settled();
        camera.Current.Distance = 5f;
        Assert.Equal(5f, camera.Current.Distance);
        camera.Current.Distance = 0f;
        Assert.Equal(0f, camera.Current.Distance);
    }

    /// <summary>
    /// REQ-SCN-071 — every setpoint angle of <b>every</b> mode is brought back into
    /// <c>]−180°, 180°]</c> at each update; the displayed angle of the current mode takes the
    /// same whole turns, so the orientation never jumps, and the smoothing velocity is left
    /// alone.
    /// </summary>
    [Fact] // REQ-SCN-071
    public void The_setpoint_angles_are_wrapped_into_one_turn_without_a_visible_jump()
    {
        SceneCamera camera = Settled();
        camera.Current.Angles = Vector3.Zero;
        camera.SnapToSetpoints();

        // 390° of orbit, one 60 Hz tick at a time: the orientation varies continuously.
        Vector3 previous = camera.BuildView(DefaultClipping, 16f / 9f).Forward;
        float worst = 0f;
        for (int tick = 0; tick < 13 * 60; tick++)
        {
            CameraControls.Apply(camera, CameraAction.RotateDown, 1f / 60f);
            camera.Advance(1f / 60f);
            Vector3 forward = camera.BuildView(DefaultClipping, 16f / 9f).Forward;
            worst = MathF.Max(worst, Vector3.Distance(forward, previous));
            previous = forward;
        }

        Assert.Equal(30f, camera.Current.Angles.X, 0.5f);
        Assert.InRange(camera.Current.Angles.X, -180f, 180f);

        // One tick of 30°/s turns the camera by half a degree, about 0.009 of a unit vector;
        // a jump of a whole turn would show up as a change of order 1.
        Assert.True(worst < 0.05f, $"the orientation jumped by {worst}.");

        // Criterion 2: a write of 270 reads −90 at the next frame.
        camera.Current.Angles = new Vector3(0f, 270f, 0f);
        camera.Advance(1f / 60f);
        Assert.Equal(-90f, camera.Current.Angles.Y, 1e-3f);

        // ]−180, 180]: 180 stays 180 and −180 becomes 180.
        Assert.Equal(180f, SceneConstants.WrapAngle(180f).Angle);
        Assert.Equal(180f, SceneConstants.WrapAngle(-180f).Angle);
        Assert.Equal(-170f, SceneConstants.WrapAngle(190f).Angle, 1e-3f);
        Assert.Equal(170f, SceneConstants.WrapAngle(-190f).Angle, 1e-3f);

        // The angles of the mode that is not current are wrapped as well.
        SceneCamera both = new();
        both.Setpoints(CameraMode.Fpv).Angles = new Vector3(400f, -400f, 900f);
        both.Advance(1f / 60f);
        Assert.Equal(40f, both.Setpoints(CameraMode.Fpv).Angles.X, 1e-2f);
        Assert.Equal(-40f, both.Setpoints(CameraMode.Fpv).Angles.Y, 1e-2f);
        Assert.Equal(180f, both.Setpoints(CameraMode.Fpv).Angles.Z, 1e-2f);
    }

    /// <summary>
    /// REQ-SCN-071 — the shift does not change the internal velocity of the smoothing: a
    /// setpoint written 460° away is reached along exactly the path the filter of REQ-SCN-019
    /// would follow without any wrap, to whole turns.
    /// </summary>
    [Fact] // REQ-SCN-071, REQ-SCN-019
    public void Wrapping_an_angle_leaves_the_smoothing_untouched()
    {
        // The reference: the bare filter of REQ-SCN-019 taking a scalar from 0 to 460.
        CriticalSmoothing filter = default;
        float reference = 0f;

        SceneCamera camera = new();
        camera.Current.Angles = Vector3.Zero;
        camera.SnapToSetpoints();
        camera.Current.Angles = new Vector3(460f, 0f, 0f);

        for (int tick = 0; tick < 600; tick++)
        {
            camera.Advance(1f / 60f);
            reference = filter.Step(reference, 460f, camera.SmoothTime, 1f / 60f);

            // The displayed angle and the reference differ by whole turns only — both ends took
            // the same multiple of 360° — so the camera shows the very same orientation.
            float gap = reference - camera.Angles.X;
            Assert.Equal(0f, gap - (360f * MathF.Round(gap / 360f)), 1e-2f);
        }

        Assert.Equal(100f, camera.Current.Angles.X, 1e-3f);
        Assert.Equal(460f, reference, 1e-2f);
    }

    /// <summary>
    /// REQ-SCN-017 — a left-button drag turns the camera by 0.1° per pixel: 300 pixels to the
    /// right add 30° to <c>ry</c>, and a drag upwards lowers <c>rx</c>.
    /// </summary>
    [Fact] // REQ-SCN-017
    public void A_mouse_drag_turns_a_tenth_of_a_degree_a_pixel()
    {
        Assert.Equal(0.1f, SceneConstants.DragDegreesPerPixel);

        SceneCamera camera = new();
        camera.Current.Angles = Vector3.Zero;
        CameraControls.ApplyDrag(camera, 300f, 0f);
        Assert.Equal(30f, camera.Current.Angles.Y, 1e-3f);

        CameraControls.ApplyDrag(camera, 0f, 100f);
        Assert.Equal(-10f, camera.Current.Angles.X, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-018 — the rotations and zooms only act while the game is running: in pause the
    /// view the player comes back to is the one they left.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-018
    public void The_camera_controls_do_nothing_outside_a_running_game()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        Vector3 angles = host.Scene.Camera.Current.Angles;
        float distance = host.Scene.Camera.Current.Distance;

        host.Run(ZaxCommands.Pause);
        Assert.Equal(EmulationState.Paused, host.State);

        for (int tick = 0; tick < 60; tick++)
        {
            host.Tick(
                1.0 / 60.0,
                Zax.App.Input.InputAction.RotRight | Zax.App.Input.InputAction.ZoomIn);
        }

        Assert.Equal(angles, host.Scene.Camera.Current.Angles);
        Assert.Equal(distance, host.Scene.Camera.Current.Distance, 1e-3f);
    }

    // ---------------------------------------------------------------- §3.6 smoothing

    /// <summary>
    /// REQ-SCN-019 — the critical smoothing of characteristic time <c>Tₛ</c> never overshoots: a
    /// 100-unit step with <c>Tₛ = 0.4 s</c> leaves 40.6 ± 3 after 0.4 s and under 2 after 1.2 s.
    /// </summary>
    [Fact] // REQ-SCN-019
    public void The_critical_smoothing_follows_the_documented_filter()
    {
        Assert.Equal(0.4f, CriticalSmoothing.DefaultSmoothTime);

        SceneCamera camera = new();
        camera.Current.Distance = 0f;
        camera.SnapToSetpoints();
        camera.Current.Distance = 100f;

        Step(camera, 0.4f);
        Assert.Equal(100f - 40.6f, camera.Distance, 3f);

        Step(camera, 0.8f);
        Assert.True(Math.Abs(100f - camera.Distance) < 2f, $"{100f - camera.Distance} left after 1.2 s.");

        // No overshoot, whatever the step size.
        SceneCamera fine = new();
        fine.Current.Distance = 0f;
        fine.SnapToSetpoints();
        fine.Current.Distance = 100f;
        for (int tick = 0; tick < 600; tick++)
        {
            fine.Advance(1f / 60f);
            Assert.True(fine.Distance <= 100f + 1e-3f, $"overshoot at tick {tick}: {fine.Distance}.");
        }

        static void Step(SceneCamera camera, float seconds)
        {
            int ticks = (int)Math.Round(seconds * 600f);
            for (int tick = 0; tick < ticks; tick++)
            {
                camera.Advance(1f / 600f);
            }
        }
    }

    /// <summary>
    /// REQ-SCN-019 — the angles are smoothed component by component. Its parenthesis « without
    /// folding modulo 360° » is replaced by AMD-045 REQ-SCN-071: the setpoint is folded, but so
    /// is the displayed angle, by the same whole turn, so the <b>rotation travelled</b> is still
    /// 400° and not 40°.
    /// </summary>
    [Fact] // REQ-SCN-019, REQ-SCN-071
    public void The_angles_are_smoothed_component_by_component_over_the_whole_rotation()
    {
        SceneCamera camera = new();
        camera.Current.Angles = Vector3.Zero;
        camera.SnapToSetpoints();
        camera.Current.Angles = new Vector3(400f, 0f, 0f);

        float travelled = 0f;
        float previous = camera.Angles.X;
        for (int tick = 0; tick < 600; tick++)
        {
            camera.Advance(1f / 60f);

            // The fold of the first tick shifts both ends alike, so it adds nothing to the
            // rotation the camera really performs.
            float step = camera.Angles.X - previous;
            travelled += step - (360f * MathF.Round(step / 360f));
            previous = camera.Angles.X;
        }

        Assert.Equal(40f, camera.Current.Angles.X, 0.5f);
        Assert.Equal(40f, camera.Angles.X, 0.5f);
        Assert.Equal(400f, travelled, 0.5f);
        Assert.Equal(0f, camera.Angles.Y, 1e-3f);
        Assert.Equal(0f, camera.Angles.Z, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-020 — in TPV the target is always smoothed; in FPV it is smoothed during the
    /// 3-second transition window and followed instantly afterwards.
    /// </summary>
    [Fact] // REQ-SCN-020
    public void The_FPV_target_is_followed_instantly_once_the_transition_is_over()
    {
        SceneCamera camera = new();
        camera.SnapToSetpoints();

        // TPV: a jump of the target is smoothed.
        camera.Current.Target = new Vector3(200f, 120f, 0f);
        camera.Advance(1f / 60f);
        Assert.True(camera.Target.X < 200f, "the TPV target must be smoothed.");

        camera.SetMode(CameraMode.Fpv);
        camera.Current.Target = new Vector3(10f, 10f, 0f);
        camera.Advance(1f / 60f);
        Assert.True(
            Math.Abs(camera.Target.X - 10f) > 1f,
            "inside the transition window the FPV target is smoothed.");

        // Five seconds later the window is closed: a brusque move is followed in the same frame.
        camera.Advance(5f);
        camera.Current.Target = new Vector3(200f, 200f, 0f);
        camera.Advance(1f / 60f);
        Assert.Equal(200f, camera.Target.X, 1e-3f);
        Assert.Equal(200f, camera.Target.Y, 1e-3f);
    }

    // ------------------------------------------------- §3.7 FPV follow and occlusion

    /// <summary>
    /// REQ-SCN-021 — FPV can be entered from Lua by <c>SetFPV(shape)</c>, <c>SetFPV()</c>,
    /// <c>Camera.Mode = 1</c> and <c>ToggleMode()</c>; with a shape the target takes the position
    /// of its first tile and the tracker starts following it.
    /// </summary>
    [Fact] // REQ-SCN-021, REQ-LUA-088
    public void Entering_FPV_on_a_shape_moves_the_target_and_starts_the_tracker()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "hero")
            .Script("s", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 64, 96), counter: 1);

        run.Do("""
            hero = Frame:GetShapeWithTag("hero")
            Camera:SetFPV(hero.Shape2D)
            target = Camera.LookAt
            """);

        Assert.Equal(1, run.Camera.Mode);
        Assert.NotNull(run.Runtime.FpvShape);

        Vec3 expected = run.Engine.PublishedFrame!.ScriptableInstances[0].GetPos(run.Engine.Settings);
        Assert.Equal(expected.X, run.Num("target.x"), 3);
        Assert.Equal(expected.Y, run.Num("target.y"), 3);

        // SetFPV() without a shape only changes mode.
        run.Do("Camera:SetTPV(); Camera:SetFPV()");
        Assert.Equal(1, run.Camera.Mode);

        run.Do("Camera.Mode = 0; Camera.Mode = 1");
        Assert.Equal(1, run.Camera.Mode);
    }

    /// <summary>
    /// REQ-SCN-021 — when the tracker loses the followed shape the follow stops and the target
    /// stays where it was.
    /// </summary>
    [Fact] // REQ-SCN-021
    public void When_the_tracker_loses_the_shape_the_follow_stops_and_the_target_stays()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "hero")
            .Script("s", string.Empty, tags: "*")
            .Start();

        run.Frame(new SyntheticBatch().Background('A', 64, 96), counter: 1);
        run.Do("Camera:SetFPV(Frame:GetShapeWithTag(\"hero\").Shape2D)");

        Vec3 before = run.Camera.LookAt;

        // The shape is gone from the next frame: the tracker loses it.
        run.Frame(new SyntheticBatch(), counter: 2);
        run.Runtime.AdvanceCameraTracking();

        Assert.Null(run.Runtime.FpvShape);
        Assert.Equal(before.X, run.Camera.LookAt.X, 3);
        Assert.Equal(before.Y, run.Camera.LookAt.Y, 3);
    }

    /// <summary>
    /// REQ-SCN-022 — the occlusion test is only enabled in FPV with a followed shape, it never
    /// fades the followed shape itself, and the opacity it imposes is 0.5.
    /// </summary>
    [Fact] // REQ-SCN-022
    public void The_occlusion_test_only_runs_in_FPV_with_a_followed_shape()
    {
        Assert.Equal(0.5f, SceneConstants.OccluderAlpha);

        SceneFixture fixture = new();
        fixture
            .Pattern(SceneFixture.Solid(1, 0), depth: 16, ProfileLayer.L1)
            .Tile(SceneFixture.Solid(1, 0), 64, 96);
        SceneState scene = fixture.Build();

        Assert.False(Occlusion.For(scene, scene.Camera.BuildView(scene.Clipping, 4f / 3f)).Enabled);

        scene.FollowedShape = fixture.Engine!.PublishedFrame!.Shapes[0];
        Assert.False(
            Occlusion.For(scene, scene.Camera.BuildView(scene.Clipping, 4f / 3f)).Enabled,
            "TPV must never fade a tile.");

        scene.Camera.SetMode(CameraMode.Fpv);
        Occlusion occlusion = Occlusion.For(scene, scene.Camera.BuildView(scene.Clipping, 4f / 3f));
        Assert.True(occlusion.Enabled);

        DrawTile tile = scene.DrawList!.Tiles[0];
        Assert.False(
            occlusion.Hides(tile, tile.Mesh!, tile.Transform),
            "a tile of the followed shape is never an occluder.");

        // The slab test itself: a segment that crosses the box, and one that misses it.
        Assert.True(Occlusion.SegmentHitsBox(
            new Vector3(0f, 0f, -10f), new Vector3(0f, 0f, 10f),
            new Vector3(-1f, -1f, -1f), new Vector3(1f, 1f, 1f)));
        Assert.False(Occlusion.SegmentHitsBox(
            new Vector3(10f, 10f, -10f), new Vector3(10f, 10f, 10f),
            new Vector3(-1f, -1f, -1f), new Vector3(1f, 1f, 1f)));
    }

    // --------------------------------------- §3.8 profile control and saved state

    /// <summary>
    /// REQ-SCN-023 — <c>Camera:SetTarget</c> acts on the setpoint of the current mode and the
    /// effective target in TPV is <c>T + ((c0 − c2)/2, (c1 − c3)/2, 0)</c>.
    /// </summary>
    [Fact] // REQ-SCN-023, REQ-SCN-007
    public void SetTarget_moves_the_setpoint_and_TPV_recentres_it()
    {
        SceneCamera camera = Settled();
        camera.Current.Target = new Vector3(128f, 180f, 0f);
        camera.SnapToSetpoints();

        CameraView view = camera.BuildView(DefaultClipping, 16f / 9f);

        Assert.Equal(128f + ((0.1f - 8.1f) / 2f), view.Target.X, 1e-3f);
        Assert.Equal(180f + ((8.1f - 0.1f) / 2f), view.Target.Y, 1e-3f);

        Vector3 effective = SceneConstants.EffectiveTarget(new Vector3(128f, 180f, 0f), DefaultClipping);
        Assert.Equal(view.Target, effective);
    }

    /// <summary>
    /// REQ-SCN-024 — the camera state a savestate carries: the current mode, and for each mode
    /// the distance, the angles and the target, plus what is left of the transition window.
    /// AMD-045 REQ-SCN-073 replaces the last clause: the displayed values are snapped onto the
    /// restored setpoints instead of rejoining them through the smoothing.
    /// </summary>
    [Fact] // REQ-SCN-024, REQ-SCN-073
    public void The_camera_state_round_trips_through_a_savestate()
    {
        SceneCamera camera = new();
        camera.SetMode(CameraMode.Fpv);
        camera.Current.Distance = 40f;
        camera.Current.Angles = new Vector3(10f, 20f, 30f);
        camera.Current.Target = new Vector3(1f, 2f, 3f);
        camera.Setpoints(CameraMode.Tpv).Distance = 150f;
        camera.Advance(0.5f);

        CameraStateData saved = camera.Capture();
        byte[] block = Zax.App.Persistence.SaveStateFile.WriteCamera(saved);
        CameraStateData read = Zax.App.Persistence.SaveStateFile.ReadCamera(block);

        SceneCamera other = new();
        other.SetMode(CameraMode.Tpv);
        other.Current.Distance = 500f;
        other.SnapToSetpoints();
        other.Restore(read);

        Assert.Equal(CameraMode.Fpv, other.Mode);
        Assert.Equal(40f, other.Setpoints(CameraMode.Fpv).Distance, 3);
        Assert.Equal(new Vector3(10f, 20f, 30f), other.Setpoints(CameraMode.Fpv).Angles);
        Assert.Equal(new Vector3(1f, 2f, 3f), other.Setpoints(CameraMode.Fpv).Target);
        Assert.Equal(150f, other.Setpoints(CameraMode.Tpv).Distance, 3);
        Assert.Equal(saved.TransitionRemaining, other.TransitionRemaining, 3);

        // Snapped: the displayed camera is already on the restored setpoints (REQ-SCN-073).
        Assert.Equal(40f, other.Distance, 3);
        Assert.Equal(new Vector3(10f, 20f, 30f), other.Angles);
        Assert.Equal(new Vector3(1f, 2f, 3f), other.Target);
    }

    /// <summary>
    /// REQ-SCN-073 — the first picture after a load already shows the saved view: save the
    /// default view, orbit 90° and zoom, reload.
    /// </summary>
    [Fact] // REQ-SCN-073
    public void A_reloaded_camera_shows_the_saved_view_on_the_very_first_frame()
    {
        SceneCamera saved = Settled();
        CameraStateData state = saved.Capture();
        CameraView wanted = saved.BuildView(DefaultClipping, 16f / 9f);

        SceneCamera camera = Settled();
        camera.Current.Angles = new Vector3(0.81f, 90.81f, 0f);
        camera.Current.Distance = 60f;
        Advance(camera, 2f);

        camera.Restore(state);

        // No Advance in between: the frame drawn right after the load is already the saved one.
        CameraView first = camera.BuildView(DefaultClipping, 16f / 9f);
        Assert.Equal(wanted.Distance, first.Distance, 1e-3f);
        Assert.Equal(wanted.Eye.X, first.Eye.X, 1e-3f);
        Assert.Equal(wanted.Eye.Y, first.Eye.Y, 1e-3f);
        Assert.Equal(wanted.Eye.Z, first.Eye.Z, 1e-3f);

        // And the smoothing velocities were reset, so the next frames do not drift either.
        Advance(camera, 1f);
        CameraView later = camera.BuildView(DefaultClipping, 16f / 9f);
        Assert.Equal(wanted.Eye.X, later.Eye.X, 1e-3f);
        Assert.Equal(wanted.Eye.Y, later.Eye.Y, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-073 — the transition window is restored as a <b>remaining time</b> bounded to
    /// <c>[0, 3]</c> s (AMD-042): saving 1 s after a switch to FPV leaves 2 s, during which the
    /// FPV target is still smoothed; after them the camera follows it in the same frame
    /// (REQ-SCN-020).
    /// </summary>
    [Fact] // REQ-SCN-073, REQ-SCN-020
    public void The_restored_transition_window_keeps_its_remaining_time()
    {
        SceneCamera saved = Settled();
        saved.SetMode(CameraMode.Fpv);
        Advance(saved, 1f);
        Assert.Equal(2f, saved.TransitionRemaining, 0.05f);

        CameraStateData state = saved.Capture();
        SceneCamera camera = Settled();
        camera.Restore(state);
        Assert.Equal(2f, camera.TransitionRemaining, 0.05f);

        // Inside the window the target is smoothed: 100 units away, about 40 are left after
        // Tₛ = 0.4 s (REQ-SCN-019).
        Vector3 start = camera.Current.Target;
        camera.Current.Target = start + new Vector3(100f, 0f, 0f);
        Advance(camera, 0.4f);
        Assert.Equal(40f, camera.Current.Target.X - camera.Target.X, 3f);

        // Past the window the camera follows the target in the same frame (REQ-SCN-020).
        Advance(camera, 2f);
        camera.Current.Target = start + new Vector3(500f, 0f, 0f);
        camera.Advance(1f / 60f);
        Assert.Equal(camera.Current.Target.X, camera.Target.X, 1e-3f);

        // The bound: a window of 90 s in the block comes back as 3 s, a negative one as 0.
        camera.Restore(new CameraStateData(state.Mode, state.Tpv, state.Fpv, 90f));
        Assert.Equal(SceneConstants.ModeTransitionSeconds, camera.TransitionRemaining, 1e-3f);
        camera.Restore(new CameraStateData(state.Mode, state.Tpv, state.Fpv, -1f));
        Assert.Equal(0f, camera.TransitionRemaining, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-024, REQ-SCN-073 — the camera state really travels in a savestate written and
    /// read by the host, and the displayed camera is already there when the load returns.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-024, REQ-SCN-073, REQ-LOOP-040
    public void A_savestate_of_the_host_carries_the_camera()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();

        host.Scene.Camera.SetMode(CameraMode.Fpv);
        host.Scene.Camera.Current.Distance = 40f;
        host.Run(ZaxCommands.QuickSave);

        host.Scene.Camera.SetMode(CameraMode.Tpv);
        host.Scene.Camera.Current.Distance = 300f;

        host.Run(ZaxCommands.QuickLoad);

        Assert.Equal(CameraMode.Fpv, host.Scene.Camera.Mode);
        Assert.Equal(40f, host.Scene.Camera.Current.Distance, 1f);
        Assert.Equal(40f, host.Scene.Camera.Distance, 1f);
    }

    private static Vector3 Round(Vector3 value) => new(
        MathF.Round(value.X, 4),
        MathF.Round(value.Y, 4),
        MathF.Round(value.Z, 4));
}
