using System.Numerics;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// The camera of SPEC-06 §3.2 to §3.6 and the test vectors of §5.1.
/// </summary>
public sealed class CameraTests
{
    /// <summary>Clip margins of vector C1, the defaults of REQ-SCN-003.</summary>
    private static Vec4F DefaultClipping => new(0.1f, 8.1f, 8.1f, 0.1f);

    /// <summary>Clip margins of vector C2, those of the Super Mario Bros profile.</summary>
    private static Vec4F MarioClipping => new(0f, 7.9f, 0f, 0.1f);

    [Theory]
    [InlineData(70f, 189.65f)]
    [InlineData(45f, 277.93f)]
    [InlineData(95f, 150.25f)]
    [InlineData(90f, 156.21f)]
    public void REQ_SCN_012_default_distance_is_104_over_atan_of_half_the_fov(float fov, float expected)
        => Assert.Equal(expected, SceneConstants.DefaultDistance(fov), 0.05);

    [Theory]
    [InlineData(70f, 109.42f)]
    [InlineData(90f, 90.12f)]
    public void REQ_SCN_045_hud_plane_distance_is_60_over_atan_of_half_the_fov(float fov, float expected)
        => Assert.Equal(expected, SceneConstants.HudDistance(fov), 0.05);

    [Fact]
    public void REQ_SCN_004_visible_height_in_the_nes_plane_matches_the_acceptance_criterion()
    {
        // 16:9, FOV 70, distance 189.65 from the NES plane: 265.6 high and 472.2 wide (±1 %).
        const float distance = 189.65f;
        float height = 2f * distance * MathF.Tan(70f * MathF.PI / 180f / 2f);
        float width = height * 16f / 9f;

        Assert.Equal(265.6f, height, 2.7);
        Assert.Equal(472.2f, width, 4.8);
    }

    [Fact]
    public void REQ_SCN_007_forward_is_cos_rx_sin_ry_minus_sin_rx_cos_rx_cos_ry()
    {
        Vector3 f = SceneConstants.Forward(new Vector3(30f, -20f, 0f), localCamera: false);

        Assert.Equal(-0.29620f, f.X, 1e-4);
        Assert.Equal(-0.5f, f.Y, 1e-4);
        Assert.Equal(0.81380f, f.Z, 1e-4);
    }

    [Fact]
    public void REQ_SCN_007_rz_has_no_effect_on_the_forward_direction()
    {
        Vector3 without = SceneConstants.Forward(new Vector3(12f, 34f, 0f), localCamera: false);
        Vector3 with = SceneConstants.Forward(new Vector3(12f, 34f, 57f), localCamera: false);

        Assert.Equal(without.X, with.X, 1e-5);
        Assert.Equal(without.Y, with.Y, 1e-5);
        Assert.Equal(without.Z, with.Z, 1e-5);
    }

    [Fact]
    public void REQ_SCN_007_effective_target_recentres_on_the_visible_rectangle()
    {
        Vector3 target = SceneConstants.EffectiveTarget(SceneConstants.Center, DefaultClipping);

        // Default margins: T' = (124, 124, 0), the centre of R (REQ-E3D-410).
        Assert.Equal(124f, target.X, 1e-4);
        Assert.Equal(124f, target.Y, 1e-4);
        Assert.Equal(0f, target.Z, 1e-4);
        Assert.Equal(VisibleRect.From(DefaultClipping).Center.X, target.X, 1e-4);
        Assert.Equal(VisibleRect.From(DefaultClipping).Center.Y, target.Y, 1e-4);
    }

    [Fact]
    public void Vector_C1_default_view_with_the_default_margins()
    {
        CameraView view = Steady(DefaultClipping);

        Assert.Equal(189.65f, view.Distance, 0.05);
        Assert.Equal(124f, view.Target.X, 0.5);
        Assert.Equal(124f, view.Target.Y, 0.5);
        Assert.Equal(121.32f, view.Eye.X, 0.5);
        Assert.Equal(126.68f, view.Eye.Y, 0.5);
        Assert.Equal(-189.61f, view.Eye.Z, 0.5);
        Assert.Equal(123.4f, view.AimPoint.X, 0.6);
        Assert.Equal(124.6f, view.AimPoint.Y, 0.6);
    }

    [Fact]
    public void Vector_C2_default_view_with_the_super_mario_bros_margins()
    {
        CameraView view = Steady(MarioClipping);

        Assert.Equal(128f, view.Target.X, 0.5);
        Assert.Equal(123.9f, view.Target.Y, 0.5);
        Assert.Equal(125.32f, view.Eye.X, 0.5);
        Assert.Equal(126.58f, view.Eye.Y, 0.5);
        Assert.Equal(-189.61f, view.Eye.Z, 0.5);
        Assert.Equal(127.3f, view.AimPoint.X, 0.6);
        Assert.Equal(124.5f, view.AimPoint.Y, 0.6);
    }

    [Fact]
    public void Vector_C3_orbited_view_places_the_eye_above_and_to_the_right()
    {
        SceneCamera camera = new();
        camera.Current.Angles = new Vector3(30f, -20f, 0f);
        camera.SnapToSetpoints();
        CameraView view = camera.BuildView(MarioClipping, 16f / 9f);

        Assert.Equal(184.17f, view.Eye.X, 0.5);
        Assert.Equal(218.73f, view.Eye.Y, 0.5);
        Assert.Equal(-154.34f, view.Eye.Z, 0.5);
        Assert.Equal(145.1f, view.AimPoint.X, 0.6);
        Assert.Equal(145.7f, view.AimPoint.Y, 0.6);
    }

    [Fact]
    public void Vector_C4_a_fov_change_resets_the_view_and_the_hud_distance()
    {
        SceneCamera camera = new();
        camera.Current.Distance = 42f;
        camera.Fov = 90f;

        Assert.Equal(156.21f, camera.Current.Distance, 0.05);
        Assert.Equal(90.12f, SceneConstants.HudDistance(camera.Fov), 0.05);
        Assert.Equal(SceneConstants.DefaultAngles, camera.Current.Angles);
    }

    [Fact]
    public void Vector_C5_zoom_in_held_three_seconds_bottoms_out_at_the_bound()
    {
        SceneCamera camera = new();
        for (int step = 0; step < 180; step++)
        {
            CameraControls.Apply(camera, CameraAction.ZoomIn, 1f / 60f);
        }

        Assert.Equal(0f, camera.Current.Distance);
    }

    [Fact]
    public void REQ_SCN_011_the_setpoint_distance_is_bounded_but_the_angles_are_not()
    {
        SceneCamera camera = new();

        camera.Current.Distance = 1000f;
        Assert.Equal(512f, camera.Current.Distance);
        camera.Current.Distance = -5f;
        Assert.Equal(0f, camera.Current.Distance);

        // Holding rotate down for 13 s takes rx past 360° (REQ-SCN-011).
        for (int step = 0; step < 13 * 60; step++)
        {
            CameraControls.Apply(camera, CameraAction.RotateDown, 1f / 60f);
        }

        Assert.True(camera.Current.Angles.X > 360f);
    }

    [Fact]
    public void REQ_SCN_014_rotate_right_held_three_seconds_adds_ninety_degrees_at_any_frame_rate()
    {
        foreach (float frameRate in new[] { 30f, 60f, 144f })
        {
            SceneCamera camera = new();
            float start = camera.Current.Angles.Y;
            int steps = (int)(3f * frameRate);
            for (int step = 0; step < steps; step++)
            {
                CameraControls.Apply(camera, CameraAction.RotateRight, 1f / frameRate);
            }

            Assert.Equal(start + 90f, camera.Current.Angles.Y, 90f * 0.02);
        }
    }

    [Fact]
    public void REQ_SCN_014_opposite_rotations_do_not_cancel_each_other()
    {
        SceneCamera camera = new();
        float startY = camera.Current.Angles.Y;
        float startX = camera.Current.Angles.X;

        CameraControls.Apply(
            camera,
            CameraAction.RotateLeft | CameraAction.RotateRight
                | CameraAction.RotateUp | CameraAction.RotateDown,
            1f);

        Assert.Equal(startY - 30f, camera.Current.Angles.Y, 1e-4);
        Assert.Equal(startX + 30f, camera.Current.Angles.X, 1e-4);
    }

    [Fact]
    public void REQ_SCN_015_zoom_in_held_one_second_from_the_default_view_reaches_119_65()
    {
        SceneCamera camera = new();
        for (int step = 0; step < 60; step++)
        {
            CameraControls.Apply(camera, CameraAction.ZoomIn, 1f / 60f);
        }

        Assert.Equal(119.65f, camera.Current.Distance, 119.65f * 0.02);
    }

    [Fact]
    public void REQ_SCN_016_ten_wheel_notches_forward_take_the_distance_to_139_65()
    {
        SceneCamera camera = new();
        CameraControls.ApplyWheel(camera, 10f, CameraAction.None);

        Assert.Equal(139.65f, camera.Current.Distance, 1.0);
    }

    [Fact]
    public void REQ_SCN_016_the_wheel_is_ignored_while_a_zoom_action_is_held()
    {
        SceneCamera camera = new();
        float before = camera.Current.Distance;
        CameraControls.ApplyWheel(camera, 10f, CameraAction.ZoomIn);

        Assert.Equal(before, camera.Current.Distance);
    }

    [Fact]
    public void REQ_SCN_017_dragging_300_pixels_right_adds_thirty_degrees_to_ry()
    {
        SceneCamera camera = new();
        float start = camera.Current.Angles.Y;
        CameraControls.ApplyDrag(camera, 300f, 0f);

        Assert.Equal(start + 30f, camera.Current.Angles.Y, 1e-3);
    }

    [Fact]
    public void REQ_SCN_019_a_distance_step_of_one_hundred_decays_as_the_criterion_says()
    {
        SceneCamera camera = new();
        camera.SnapToSetpoints();
        float target = camera.Current.Distance;
        camera.Current.Distance = target + 100f;

        Advance(camera, 0.4f);
        Assert.Equal(40.6f, MathF.Abs(camera.Current.Distance - camera.Distance), 3.0);

        Advance(camera, 0.8f);
        Assert.True(MathF.Abs(camera.Current.Distance - camera.Distance) < 2f);
    }

    [Fact]
    public void REQ_SCN_019_the_filter_never_overshoots_its_setpoint()
    {
        CriticalSmoothing filter = default;
        float value = 0f;
        for (int step = 0; step < 600; step++)
        {
            value = filter.Step(value, 100f, 0.4f, 1f / 60f);
            Assert.InRange(value, 0f, 100f);
        }

        Assert.Equal(100f, value, 1e-3);
    }

    [Fact]
    public void REQ_SCN_006_each_mode_keeps_its_own_distance_setpoint()
    {
        SceneCamera camera = new();
        camera.Current.Distance = 150f;

        camera.SetMode(CameraMode.Fpv);
        camera.Current.Distance = 40f;
        Assert.Equal(40f, camera.Current.Distance);
        Assert.Equal(SceneConstants.ModeTransitionSeconds, camera.TransitionRemaining);

        camera.SetMode(CameraMode.Tpv);
        Assert.Equal(150f, camera.Current.Distance);
    }

    [Fact]
    public void REQ_SCN_012_a_view_reset_only_changes_the_distance_of_the_current_mode()
    {
        SceneCamera camera = new();
        camera.SetMode(CameraMode.Fpv);
        camera.Current.Distance = 40f;
        camera.SetMode(CameraMode.Tpv);
        camera.Current.Distance = 11f;

        camera.ResetView();

        Assert.Equal(SceneConstants.DefaultDistance(70f), camera.Setpoints(CameraMode.Tpv).Distance, 0.05);
        Assert.Equal(40f, camera.Setpoints(CameraMode.Fpv).Distance);
        Assert.Equal(SceneConstants.DefaultAngles, camera.Setpoints(CameraMode.Fpv).Angles);
        Assert.Equal(SceneConstants.Center, camera.Setpoints(CameraMode.Fpv).Target);
    }

    [Fact]
    public void REQ_SCN_012_the_fpv_distance_is_zero_at_application_launch()
        => Assert.Equal(0f, new SceneCamera().Setpoints(CameraMode.Fpv).Distance);

    [Fact]
    public void REQ_SCN_020_in_fpv_the_target_is_followed_instantly_once_the_window_is_over()
    {
        SceneCamera camera = new();
        camera.SetMode(CameraMode.Fpv);
        Advance(camera, 5f);

        camera.Current.Target = new Vector3(40f, 50f, 60f);
        camera.Advance(1f / 60f);

        Assert.Equal(40f, camera.Target.X, 1e-3);
        Assert.Equal(50f, camera.Target.Y, 1e-3);
        Assert.Equal(60f, camera.Target.Z, 1e-3);
    }

    [Fact]
    public void REQ_SCN_020_in_tpv_the_target_is_always_smoothed()
    {
        SceneCamera camera = new();
        Advance(camera, 5f);

        camera.Current.Target = new Vector3(128f, 180f, 0f);
        camera.Advance(1f / 60f);

        Assert.True(camera.Target.Y > 120f);
        Assert.True(camera.Target.Y < 180f);
    }

    [Fact]
    public void REQ_SCN_013_ending_a_game_returns_to_tpv_and_to_a_smooth_time_of_zero_point_four()
    {
        SceneCamera camera = new();
        camera.SmoothTime = 2f;
        camera.SetMode(CameraMode.Fpv);

        camera.EndGame();

        Assert.Equal(CameraMode.Tpv, camera.Mode);
        Assert.Equal(CriticalSmoothing.DefaultSmoothTime, camera.SmoothTime);
        Assert.Equal(SceneConstants.DefaultDistance(70f), camera.Current.Distance, 0.05);
    }

    [Fact]
    public void REQ_SCN_009_the_local_camera_rotates_the_picture_around_the_view_axis()
    {
        SceneCamera camera = new();
        camera.LocalCamera = true;
        camera.Current.Angles = new Vector3(0f, 90f, 0f);
        camera.SnapToSetpoints();

        CameraView view = camera.BuildView(DefaultClipping, 16f / 9f);

        // q = R_Z(90°) · R_X(0): the camera's right axis is scene +Y, so the top of the NES
        // picture appears on the right of the screen, a quarter turn (REQ-SCN-009 criterion).
        Assert.Equal(0f, view.Forward.X, 1e-4);
        Assert.Equal(0f, view.Forward.Y, 1e-4);
        Assert.Equal(1f, view.Forward.Z, 1e-4);
        Assert.Equal(1f, view.Right.Y, 1e-4);
        Assert.Equal(-1f, view.Up.X, 1e-4);

        // The view is still centred on the effective target (REQ-SCN-009).
        Vector3 expected = SceneConstants.EffectiveTarget(SceneConstants.Center, DefaultClipping);
        Vector3 eye = expected - (view.Forward * view.Distance);
        Assert.Equal(eye.X, view.Eye.X, 1e-3);
        Assert.Equal(eye.Y, view.Eye.Y, 1e-3);
    }

    [Fact]
    public void REQ_SCN_010_in_fpv_the_target_is_exactly_at_the_centre_of_the_picture()
    {
        SceneCamera camera = new();
        camera.SetMode(CameraMode.Fpv);
        camera.Current.Angles = Vector3.Zero;
        camera.Current.Distance = 100f;
        camera.Current.Target = new Vector3(60f, 70f, 80f);
        camera.SnapToSetpoints();

        CameraView view = camera.BuildView(DefaultClipping, 16f / 9f);

        // No recentring on the visible rectangle in FPV (REQ-SCN-010).
        Assert.Equal(60f, view.Target.X, 1e-3);
        Assert.Equal(70f, view.Target.Y, 1e-3);
        Assert.Equal(60f, view.Eye.X, 1e-3);
        Assert.Equal(70f, view.Eye.Y, 1e-3);
        Assert.Equal(-20f, view.Eye.Z, 1e-3);
    }

    /// <summary>
    /// REQ-SCN-008 as amended by AMD-019 (Q-046): the degenerate case is the eye on <b>one</b>
    /// of the two vertices — at least one distance zero — and the point is then the midpoint of
    /// the segment, not the vertex the general formula would give.
    /// </summary>
    [Fact] // REQ-SCN-008, Q-046
    public void REQ_SCN_008_a_degenerate_segment_falls_back_to_its_midpoint()
    {
        Vector3 p = new(10f, 20f, 0f);
        Vector3 q = new(30f, 20f, 0f);

        Assert.Equal(20f, SceneConstants.Bisector(p, q, p).X, 1e-4);
        Assert.Equal(20f, SceneConstants.Bisector(p, q, q).X, 1e-4);

        // The segment reduced to a point stays that point.
        Assert.Equal(10f, SceneConstants.Bisector(p, p, p).X, 1e-4);

        // Away from the vertices the general formula still holds: an eye equidistant from both
        // ends lands on the middle, and an eye twice as far from q lands a third of the way.
        Vector3 middle = SceneConstants.Bisector(p, q, new Vector3(20f, 0f, 0f));
        Assert.Equal(20f, middle.X, 1e-4);
    }

    [Fact]
    public void REQ_SCN_045_a_hud_element_at_the_nes_centre_sits_four_units_above_the_screen_centre()
    {
        CameraView view = Steady(DefaultClipping);

        Vector3 centre = view.HudPoint(SceneConstants.Center);
        Vector3 element = view.HudPoint(new Vector3(128f, 128f, 0f));

        Assert.Equal(4f, Vector3.Dot(element - centre, view.Up), 1e-3);
        Assert.Equal(0f, Vector3.Dot(element - centre, view.Right), 1e-3);
        Assert.Equal(view.HudDistance, Vector3.Dot(centre - view.Eye, view.Forward), 1e-3);
    }

    [Fact]
    public void REQ_SCN_045_the_hud_frame_fills_about_78_percent_of_the_picture_height()
    {
        CameraView view = Steady(DefaultClipping);

        float visible = 2f * view.HudDistance * MathF.Tan(view.Fov * MathF.PI / 180f / 2f);
        float hudHeight = SceneConstants.FrameHeight * SceneConstants.HudScale;

        Assert.Equal(0.78f, hudHeight / visible, 0.01);
    }

    [Fact]
    public void REQ_SCN_029_the_hud_light_direction_does_not_move_when_the_camera_orbits()
    {
        Vector3 direction = new(1f, -1f, 1f);

        SceneCamera front = new();
        front.SnapToSetpoints();
        Vector3 frontLight = front.BuildView(DefaultClipping, 1.6f).HudLightDirection(direction);

        SceneCamera orbited = new();
        orbited.Current.Angles = new Vector3(0f, 90f, 0f);
        orbited.SnapToSetpoints();
        CameraView orbitedView = orbited.BuildView(DefaultClipping, 1.6f);
        Vector3 orbitedLight = orbitedView.HudLightDirection(direction);

        // Expressed in the camera frame the direction is the same, so N·L on a HUD quad facing
        // the camera is unchanged.
        Vector3 frontNormal = -front.BuildView(DefaultClipping, 1.6f).Forward;
        Vector3 orbitedNormal = -orbitedView.Forward;
        float frontNdl = Vector3.Dot(Vector3.Normalize(frontNormal), Vector3.Normalize(-frontLight));
        float orbitedNdl = Vector3.Dot(Vector3.Normalize(orbitedNormal), Vector3.Normalize(-orbitedLight));

        Assert.Equal(frontNdl, orbitedNdl, 1e-4);
    }

    [Fact]
    public void REQ_SCN_024_the_captured_state_holds_the_mode_both_setpoints_and_the_window()
    {
        SceneCamera camera = new();
        camera.Setpoints(CameraMode.Tpv).Distance = 150f;
        camera.SetMode(CameraMode.Fpv);
        camera.Current.Distance = 40f;
        camera.Current.Angles = new Vector3(12f, -34f, 5f);
        camera.Current.Target = new Vector3(10f, 20f, 30f);
        camera.Advance(1f);

        CameraStateData state = camera.Capture();

        Assert.Equal(CameraMode.Fpv, state.Mode);
        Assert.Equal(150f, state.Tpv.Distance, 3);
        Assert.Equal(40f, state.Fpv.Distance, 3);
        Assert.Equal(new Vector3(12f, -34f, 5f), state.Fpv.Angles);
        Assert.Equal(new Vector3(10f, 20f, 30f), state.Fpv.Target);
        Assert.Equal(2f, state.TransitionRemaining, 2);
    }

    [Fact]
    public void REQ_SCN_024_restoring_brings_back_fpv_with_the_distance_of_the_save()
    {
        SceneCamera saved = new();
        saved.SetMode(CameraMode.Fpv);
        saved.Current.Distance = 40f;
        CameraStateData state = saved.Capture();

        // The criterion of REQ-SCN-024: save in FPV, go back to TPV and zoom, reload.
        SceneCamera camera = new();
        camera.SetMode(CameraMode.Tpv);
        camera.Current.Distance = 300f;
        camera.SnapToSetpoints();

        camera.Restore(state);

        Assert.Equal(CameraMode.Fpv, camera.Mode);
        Assert.Equal(40f, camera.Current.Distance, 3);

        // The displayed values are not snapped: they join the setpoint by the smoothing of
        // REQ-SCN-019.
        Assert.Equal(300f, camera.Distance, 3);
        Advance(camera, 3f);
        Assert.Equal(40f, camera.Distance, 1);
    }

    [Fact]
    public void REQ_SCN_024_the_restored_distances_stay_inside_their_bounds()
    {
        SceneCamera camera = new();
        camera.Restore(new CameraStateData(
            CameraMode.Tpv,
            new CameraModeState(9000f, Vector3.Zero, Vector3.Zero),
            new CameraModeState(-20f, Vector3.Zero, Vector3.Zero),
            -5f));

        Assert.Equal(SceneConstants.MaxDistance, camera.Setpoints(CameraMode.Tpv).Distance, 3);
        Assert.Equal(0f, camera.Setpoints(CameraMode.Fpv).Distance, 3);
        Assert.Equal(0f, camera.TransitionRemaining, 3);
    }

    private static CameraView Steady(Vec4F clipping)
    {
        SceneCamera camera = new();
        camera.SnapToSetpoints();
        return camera.BuildView(clipping, 16f / 9f);
    }

    private static void Advance(SceneCamera camera, float seconds)
    {
        int steps = (int)MathF.Round(seconds * 600f);
        for (int step = 0; step < steps; step++)
        {
            camera.Advance(1f / 600f);
        }
    }
}
