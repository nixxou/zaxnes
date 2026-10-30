using System.Numerics;
using Zax.App.Loop;
using Zax.App.Persistence;
using Zax.Profiles;
using Zax.Render;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 verification (role R4) of lot V2-2, written from the amendments alone:
/// AMD-040 REQ-LUA-112, 113 and 115, and AMD-045 REQ-SCN-069 to 073 with the amended
/// REQ-SCN-007 step 4 and REQ-SCN-045.
/// </summary>
/// <remarks>
/// <para>
/// REQ-LUA-114 is out of this lot's scope and is not touched here.
/// </para>
/// <para>
/// Everything is asserted through what an outside observer reaches: the Lua results of a profile
/// runtime, the public camera of <c>Zax.Render</c> as the application drives it
/// (<see cref="CameraControls"/> then <see cref="SceneCamera.Advance"/>, the order of
/// <c>ZaxApplication.HandleCameraInput</c> and <c>RenderFrame</c>), the view the renderer reads,
/// and the savestate round trip of the real host.
/// </para>
/// </remarks>
public sealed class R4V2LuaCameraTests
{
    /// <summary>The default clip margins of REQ-SCN-003, as the scene starts with them.</summary>
    private static readonly Vec4F DefaultClipping = new(0.1f, 8.1f, 8.1f, 0.1f);

    /// <summary>A camera at <paramref name="fov"/> with its displayed values on the setpoints.</summary>
    private static SceneCamera Reset(float fov = 70f)
    {
        SceneCamera camera = new() { Fov = fov };
        camera.ResetView();
        camera.SnapToSetpoints();
        return camera;
    }

    /// <summary>Advances the camera by <paramref name="seconds"/> in 1/60 s steps, as a frame does.</summary>
    private static void Advance(SceneCamera camera, float seconds)
    {
        const float Step = 1f / 60f;
        for (float left = seconds; left > 0f; left -= Step)
        {
            camera.Advance(MathF.Min(Step, left));
        }
    }

    /// <summary>
    /// Holds <paramref name="held"/> for <paramref name="seconds"/>, one 1/60 s frame at a time:
    /// the controls first, then the smoothing, as the application does.
    /// </summary>
    private static void Hold(SceneCamera camera, CameraAction held, float seconds)
    {
        const float Step = 1f / 60f;
        for (float left = seconds; left > 0f; left -= Step)
        {
            float slice = MathF.Min(Step, left);
            CameraControls.Apply(camera, held, slice);
            camera.Advance(slice);
        }
    }

    /// <summary>The angle in degrees between two directions.</summary>
    private static float AngleBetween(Vector3 a, Vector3 b)
    {
        float cosine = Vector3.Dot(Vector3.Normalize(a), Vector3.Normalize(b));
        return MathF.Acos(Math.Clamp(cosine, -1f, 1f)) * 180f / MathF.PI;
    }

    /// <summary>
    /// The roll of a view in degrees: the angle between the camera's vertical and the world
    /// vertical projected on the plane across the aimed direction. It is 0 when the picture is
    /// upright and the roll angle itself when rz turns it (REQ-SCN-072).
    /// </summary>
    private static float Roll(CameraView view)
    {
        Vector3 forward = Vector3.Normalize(view.Forward);
        Vector3 reference = Vector3.UnitY - (forward * Vector3.Dot(Vector3.UnitY, forward));
        return AngleBetween(view.Up, reference);
    }

    /// <summary>A profile runtime on one empty script, started.</summary>
    private static LuaRun Lua() => new LuaProfile().Script("r4", string.Empty).Start();

    // ------------------------------------------------------------- AMD-040 REQ-LUA-112

    /// <summary>
    /// REQ-LUA-112 — the four automatic checks of the requirement, written exactly as its
    /// acceptance criterion words them: the leading integer of <c>Version</c> is at least 2,
    /// <c>Title</c> is not <c>"3dSenMaker"</c>, <c>Info.AppId</c> is nil and <c>Nes3D.Autre</c>
    /// is nil. Both members are strings, as the table of the requirement says.
    /// </summary>
    [Fact] // REQ-LUA-112
    public void R4_the_CS_info_table_carries_a_version_of_at_least_two_and_a_title()
    {
        using LuaRun run = Lua();

        Assert.Equal("table", run.TypeOf("CS"));
        Assert.Equal("table", run.TypeOf("CS.Nes3D"));
        Assert.Equal("table", run.TypeOf("CS.Nes3D.Info"));
        Assert.Equal("string", run.TypeOf("CS.Nes3D.Info.Version"));
        Assert.Equal("string", run.TypeOf("CS.Nes3D.Info.Title"));

        Assert.True(run.Flag("tonumber(CS.Nes3D.Info.Version:match('^%d+')) >= 2"));
        Assert.True(run.Flag("CS.Nes3D.Info.Title ~= '3dSenMaker'"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.Info.AppId"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.Autre"));
        Assert.Equal("nil", run.TypeOf("CS.Autre"));
        Assert.Empty(run.Log.Errors);
    }

    /// <summary>
    /// REQ-LUA-112 — "a write into these tables has no effect on the program": no error, nothing
    /// changed, and a fresh runtime still reads the specified values.
    /// </summary>
    [Fact] // REQ-LUA-112
    public void R4_a_write_into_the_CS_tables_changes_nothing()
    {
        using LuaRun run = Lua();

        Assert.Null(run.Error("CS.Nes3D.Info.Version = '1.0'"));
        Assert.Null(run.Error("CS.Nes3D.Info.Title = '3dSenMaker'"));
        Assert.Null(run.Error("CS.Nes3D.Info.Nouveau = 1"));
        Assert.Null(run.Error("CS.Nes3D.Nouveau = 1"));
        Assert.Null(run.Error("CS.Nes3D = nil"));
        Assert.Null(run.Error("CS.Nouveau = 1"));

        Assert.True(run.Flag("tonumber(CS.Nes3D.Info.Version:match('^%d+')) >= 2"));
        Assert.True(run.Flag("CS.Nes3D.Info.Title ~= '3dSenMaker'"));
        Assert.Equal("nil", run.TypeOf("CS.Nes3D.Info.Nouveau"));
        Assert.Empty(run.Log.Errors);

        using LuaRun second = Lua();
        Assert.True(second.Flag("tonumber(CS.Nes3D.Info.Version:match('^%d+')) >= 2"));
        Assert.True(second.Flag("CS.Nes3D.Info.Title ~= '3dSenMaker'"));
    }

    /// <summary>
    /// REQ-LUA-112 — the 2.0 code path, as the requirement illustrates it, runs to its end
    /// inside <c>Start</c>: the script reads the leading integer of <c>Version</c>, takes the 2.0
    /// branch and frames the camera with <c>Camera.ResetDistance</c> (REQ-LUA-113). No Lua error
    /// is raised, so the script keeps its hooks (REQ-LUA-060 to 064).
    /// </summary>
    /// <remarks>
    /// The <c>punchout</c> profile of 3dSen 2.0 that the [R2] criterion names is not among the
    /// 101 profiles of <c>assets-neutral</c> (they are the 1.1.2 ones, AMD-040 §1), so the
    /// requirement's own illustrative snippet stands in for it.
    /// </remarks>
    [Fact] // REQ-LUA-112, REQ-LUA-113
    public void R4_the_two_point_zero_code_path_of_a_profile_script_runs_without_error()
    {
        using LuaRun run = new LuaProfile()
            .Script(
                "punchout_like",
                """
                path = 'none'
                function Start()
                  local version = tonumber(CS.Nes3D.Info.Version:match("^%d+"))
                  if version >= 2 then
                    path = 'two'
                    Camera.Distance = Camera.ResetDistance * 0.75
                  elseif CS.Nes3D.Info.Title == '3dSenMaker' then
                    path = 'maker'
                    Setting.Fov = 60
                  else
                    path = 'legacy'
                    Setting.Fov = 50
                  end
                  framed = Camera.Distance
                end
                """)
            .Start();

        Assert.Empty(run.Log.Errors);
        Assert.Equal("two", run.Text("path"));
        Assert.Equal(189.65 * 0.75, run.Num("framed"), 0.05);
        Assert.Equal(70f, run.Settings.Fov, 3);
    }

    // ------------------------------------------------------------- AMD-040 REQ-LUA-113

    /// <summary>
    /// REQ-LUA-113 — the whole acceptance criterion: 189.65 at FOV 70°, unchanged by a write to
    /// <c>Camera.Distance</c>, and 320.60 at FOV 45°. REQ-SCN-069 adds 121.69 at 95°.
    /// </summary>
    [Fact] // REQ-LUA-113, REQ-SCN-069
    public void R4_the_lua_reset_distance_is_the_framed_distance_of_the_current_fov()
    {
        using LuaRun run = Lua();

        Assert.Equal(70f, run.Settings.Fov, 3);
        Assert.Equal(189.65, run.Num("Camera.ResetDistance"), 0.05);

        run.Do("Camera.Distance = 50");
        Assert.Equal(50.0, run.Num("Camera.Distance"), 1e-3);
        Assert.Equal(189.65, run.Num("Camera.ResetDistance"), 0.05);

        run.Do("Setting.Fov = 45");
        Assert.Equal(320.60, run.Num("Camera.ResetDistance"), 0.05);
        run.Do("Setting.Fov = 95");
        Assert.Equal(121.69, run.Num("Camera.ResetDistance"), 0.05);
    }

    /// <summary>
    /// REQ-LUA-113 — the member is read only (access "L" in the table of the amendment): a write
    /// raises the error of REQ-LUA-042 and leaves the value alone.
    /// </summary>
    [Fact] // REQ-LUA-113
    public void R4_the_lua_reset_distance_is_read_only()
    {
        using LuaRun run = Lua();

        Assert.NotNull(run.Error("Camera.ResetDistance = 10"));
        Assert.Equal(189.65, run.Num("Camera.ResetDistance"), 0.05);
    }

    /// <summary>
    /// REQ-LUA-113 — what a script reads is what the real camera would write: the camera of
    /// <c>Zax.Render</c>, which the host bridges straight to <c>Camera.ResetDistance</c>, gives
    /// the same number as the formula, at every FOV of REQ-SCN-005.
    /// </summary>
    [Fact] // REQ-LUA-113, REQ-SCN-069
    public void R4_the_scene_camera_reset_distance_matches_the_formula_at_every_fov()
    {
        for (int fov = 45; fov <= 95; fov += 5)
        {
            SceneCamera camera = new() { Fov = fov };
            Assert.Equal(Expected(fov), camera.ResetDistance, 2e-3);
            Assert.Equal(camera.ResetDistance, ScriptCamera.FramedDistance(camera.Fov), 2e-3);
        }

        static double Expected(double fovDegrees)
            => 132.7964 / Math.Tan(fovDegrees * Math.PI / 180.0 / 2.0);
    }

    /// <summary>
    /// REQ-LUA-113 and Q-077 — which FOV feeds <c>d₀</c>. The camera keeps the [45, 95] of
    /// REQ-SCN-005, so what a script reads is the distance the camera really frames with: at a
    /// setting of 29° the camera still draws at 45° and <c>ResetDistance</c> reads 320.60, not
    /// the 513 the raw formula gives. The <c>[0, 512]</c> bound of REQ-SCN-069 therefore never
    /// bites, which is what Q-077 says it chose.
    /// </summary>
    /// <remarks>
    /// The second copy of the formula, <see cref="ScriptCamera.FramedDistance"/>, is fed by
    /// <c>DetachedScriptCamera.Fov</c>, which is <b>not</b> bounded: on that stand-in the same
    /// script would read 513. The application never goes through it.
    /// </remarks>
    [Fact] // REQ-LUA-113, REQ-SCN-005, Q-077
    public void R4_the_reset_distance_uses_the_bounded_fov_of_the_camera()
    {
        SceneCamera camera = new() { Fov = 29f };
        Assert.Equal(45f, camera.Fov, 1e-3f);
        Assert.Equal(320.60, camera.ResetDistance, 0.05);
        Assert.True(camera.ResetDistance <= SceneConstants.MaxDistance);

        camera.Fov = 200f;
        Assert.Equal(95f, camera.Fov, 1e-3f);
        Assert.Equal(121.69, camera.ResetDistance, 0.05);

        // The raw formula does pass 512 there, which is what the host-free copy would hand to a
        // script if it were ever fed an unbounded FOV.
        Assert.True(ScriptCamera.FramedDistance(29f) > SceneConstants.MaxDistance);
    }

    // ------------------------------------------------------------- AMD-040 REQ-LUA-115

    /// <summary>
    /// REQ-LUA-115 — <c>Instance</c> offers <c>BuildExportMeshes</c> and
    /// <c>ReleaseExportMeshes</c>; both are callable on a profile instance and on a frame shape,
    /// raise no error and change nothing visible, so a script that calls them keeps its hooks.
    /// </summary>
    [Fact] // REQ-LUA-115
    public void R4_the_export_mesh_methods_exist_and_do_nothing()
    {
        using LuaRun run = new LuaProfile()
            .Pattern(["A"], tags: "box")
            .Script(
                "box",
                """
                hooks = 0
                function BuildPattern()
                  kinds = type(pattern.BuildExportMeshes) .. ',' .. type(pattern.ReleaseExportMeshes)
                  pattern:BuildExportMeshes()
                  pattern:ReleaseExportMeshes()
                  hooks = hooks + 1
                end
                function UpdateS()
                  held = shape
                end
                """,
                tags: "box")
            .Start();

        Assert.Empty(run.Log.Errors);
        Assert.Equal("function,function", run.Text("kinds"));
        Assert.Equal(1.0, run.Num("hooks"), 1e-6);

        run.Frame(new SyntheticBatch().Background('A', 16, 32));
        Assert.Equal("userdata", run.TypeOf("held"));

        Zax.Engine3D.PatternInstance instance = run.Engine.Patterns.Permanent[0].Instances[0];
        Vector3 offset = new(instance.Offset.X, instance.Offset.Y, instance.Offset.Z);
        Vector3 scale = new(instance.Scale.X, instance.Scale.Y, instance.Scale.Z);
        float alpha = instance.Alpha;

        Assert.Null(run.Error("held:BuildExportMeshes()"));
        Assert.Null(run.Error("held:ReleaseExportMeshes()"));
        Assert.Equal(offset.X, instance.Offset.X, 5);
        Assert.Equal(offset.Y, instance.Offset.Y, 5);
        Assert.Equal(offset.Z, instance.Offset.Z, 5);
        Assert.Equal(scale.X, instance.Scale.X, 5);
        Assert.Equal(scale.Y, instance.Scale.Y, 5);
        Assert.Equal(scale.Z, instance.Scale.Z, 5);
        Assert.Equal(alpha, instance.Alpha, 5);
        Assert.Empty(run.Log.Errors);
    }

    // ------------------------------------------------------------- AMD-045 REQ-SCN-069

    /// <summary>
    /// REQ-SCN-069 — the framed distance is <c>d₀ = 132.7964 / tan(FOV / 2)</c>: 189.65 at 70°,
    /// 320.60 at 45°, 121.69 at 95° (±0.05); and at 70° it agrees with the old formula of
    /// REQ-SCN-012, <c>104 / atan(FOV_rad / 2)</c>, to 10⁻³.
    /// </summary>
    [Theory] // REQ-SCN-069
    [InlineData(70f, 189.65)]
    [InlineData(45f, 320.60)]
    [InlineData(95f, 121.69)]
    public void R4_the_framed_distance_follows_the_new_formula(float fov, double expected)
    {
        Assert.Equal(expected, SceneConstants.ResetDistance(fov), 0.05);
        Assert.Equal(expected, Reset(fov).ResetDistance, 0.05);
    }

    /// <summary>
    /// REQ-SCN-069 — the two formulas meet at the default FOV of 70° and part company elsewhere,
    /// which is the Note of the requirement: 277.93 against 320.60 at 45°, 150.25 against 121.69
    /// at 95°.
    /// </summary>
    [Fact] // REQ-SCN-069
    public void R4_the_old_and_the_new_framed_distance_agree_only_at_seventy_degrees()
    {
        Assert.Equal(Old(70.0), New(70.0), 1e-3);
        Assert.Equal(189.6529, Old(70.0), 1e-3);
        Assert.Equal(189.6529, New(70.0), 1e-3);

        Assert.Equal(277.93, Old(45.0), 0.05);
        Assert.Equal(320.60, New(45.0), 0.05);
        Assert.Equal(150.25, Old(95.0), 0.05);
        Assert.Equal(121.69, New(95.0), 0.05);

        static double Old(double fov) => 104.0 / Math.Atan(fov * Math.PI / 180.0 / 2.0);
        static double New(double fov) => 132.7964 / Math.Tan(fov * Math.PI / 180.0 / 2.0);
    }

    /// <summary>
    /// REQ-SCN-069 — a view reset writes <c>d₀</c> into the setpoint of the current mode only,
    /// through the <c>[0, 512]</c> bound of REQ-SCN-011, and <c>ResetDistance</c> follows a
    /// change of FOV even without a reset (REQ-LUA-113).
    /// </summary>
    [Fact] // REQ-SCN-069, REQ-SCN-011
    public void R4_a_view_reset_writes_the_framed_distance_through_the_bound()
    {
        SceneCamera camera = Reset();
        Assert.Equal(189.65, camera.Setpoints(CameraMode.Tpv).Distance, 0.05);

        // The other mode keeps its distance: 0 at launch (REQ-SCN-012).
        Assert.Equal(0f, camera.Setpoints(CameraMode.Fpv).Distance, 5);

        // The setpoint carries the bound of REQ-SCN-011, which REQ-SCN-069 submits d₀ to.
        camera.Current.Distance = 600f;
        Assert.Equal(512f, camera.Current.Distance, 5);
        camera.Current.Distance = -3f;
        Assert.Equal(0f, camera.Current.Distance, 5);

        // The threshold the requirement names: d₀ passes 512 just under 29.1°, a FOV the camera
        // of REQ-SCN-005 cannot take (Q-077) but the formula can. At 29.1° itself d₀ is still
        // under 512, so the figure of the amendment is the rounding of 29.07°.
        Assert.True(SceneConstants.ResetDistance(29.1f) < 512f);
        Assert.True(SceneConstants.ResetDistance(29.07f) >= 511.9f);
        Assert.True(SceneConstants.ResetDistance(29f) > 512f);

        // And it follows the FOV without any reset.
        camera.Current.Distance = 50f;
        camera.Fov = 45f;
        Assert.Equal(320.60, camera.ResetDistance, 0.05);
    }

    // ------------------------------------------------------- REQ-SCN-045 (amended)

    /// <summary>
    /// REQ-SCN-045 as amended — the HUD plane sits at <c>d_HUD = 76.6133 / tan(FOV / 2)</c>:
    /// 109.42 at 70°, 184.96 at 45°, 70.20 at 95° (±0.05). The 0.5 scale factor and the rest of
    /// the requirement are unchanged.
    /// </summary>
    [Theory] // REQ-SCN-045
    [InlineData(70f, 109.42)]
    [InlineData(45f, 184.96)]
    [InlineData(95f, 70.20)]
    public void R4_the_hud_plane_distance_follows_the_new_formula(float fov, double expected)
    {
        Assert.Equal(expected, SceneConstants.HudDistance(fov), 0.05);

        SceneCamera camera = Reset(fov);
        CameraView view = camera.BuildView(DefaultClipping, 16f / 9f);
        Assert.Equal(expected, view.HudDistance, 0.05);

        // Unchanged: the element at the centre C sits on the axis at d_HUD, and the 0.5 factor
        // still halves the offsets.
        Assert.Equal(0.5f, SceneConstants.HudScale, 5);
        Vector3 centre = view.HudPoint(SceneConstants.Center);
        Assert.Equal(view.HudDistance, Vector3.Distance(view.Eye, centre), 0.01);

        Vector3 shifted = view.HudPoint(SceneConstants.Center + new Vector3(40f, 0f, 0f));
        Assert.Equal(20f, Vector3.Dot(shifted - centre, view.Right), 0.01);
    }

    /// <summary>
    /// REQ-SCN-045 — at 70° the amended value is the one the frozen requirement already gave,
    /// 109.42, so nothing moves at the default FOV; the old formula drifts elsewhere.
    /// </summary>
    [Fact] // REQ-SCN-045
    public void R4_the_hud_plane_distance_is_unchanged_at_seventy_degrees()
    {
        Assert.Equal(Old(70.0), New(70.0), 1e-3);
        Assert.Equal(109.415, New(70.0), 0.005);
        Assert.NotEqual(Math.Round(Old(45.0), 1), Math.Round(New(45.0), 1));

        static double Old(double fov) => 60.0 / Math.Atan(fov * Math.PI / 180.0 / 2.0);
        static double New(double fov) => 76.6133 / Math.Tan(fov * Math.PI / 180.0 / 2.0);
    }

    // ------------------------------------------------------------- AMD-045 REQ-SCN-070

    /// <summary>
    /// REQ-SCN-070 — the whole acceptance criterion: holding ZoomIn 5 s from the default view
    /// stops the setpoint at 16; from a distance of 5 posted by a script, ZoomIn does nothing;
    /// ZoomOut for 1 s from there gives 75 (±2 %).
    /// </summary>
    [Fact] // REQ-SCN-070
    public void R4_a_user_zoom_in_never_goes_under_sixteen()
    {
        SceneCamera camera = Reset();
        Assert.Equal(189.65, camera.Current.Distance, 0.05);

        Hold(camera, CameraAction.ZoomIn, 5f);
        Assert.Equal(16f, camera.Current.Distance, 3);

        // Already below the floor: no effect at all.
        camera.Current.Distance = 5f;
        Hold(camera, CameraAction.ZoomIn, 1f);
        Assert.Equal(5f, camera.Current.Distance, 3);

        // Zooming out is not concerned: 70 units per second (REQ-SCN-015).
        Hold(camera, CameraAction.ZoomOut, 1f);
        Assert.Equal(75f, camera.Current.Distance, 75f * 0.02f);
    }

    /// <summary>
    /// REQ-SCN-070 — the floor is a floor, not a clamp: a zoom in from just above it lands on
    /// 16 exactly, and the FPV distance of 0 at launch is left where it is.
    /// </summary>
    [Fact] // REQ-SCN-070, REQ-SCN-012
    public void R4_a_user_zoom_in_stops_on_sixteen_and_spares_the_fpv_zero()
    {
        SceneCamera camera = Reset();

        camera.Current.Distance = 20f;
        CameraControls.Apply(camera, CameraAction.ZoomIn, 1f);
        Assert.Equal(16f, camera.Current.Distance, 4);

        camera.Current.Distance = 16f;
        CameraControls.Apply(camera, CameraAction.ZoomIn, 1f);
        Assert.Equal(16f, camera.Current.Distance, 4);

        // The FPV distance is 0 at launch (REQ-SCN-012): a zoom in leaves it at 0, it does not
        // push it up to 16.
        camera.SetMode(CameraMode.Fpv);
        Assert.Equal(0f, camera.Current.Distance, 5);
        Hold(camera, CameraAction.ZoomIn, 1f);
        Assert.Equal(0f, camera.Current.Distance, 5);
    }

    /// <summary>
    /// REQ-SCN-070 — the mouse wheel is a user zoom too: forward notches stop at 16, backward
    /// notches keep only the upper bound of REQ-SCN-011.
    /// </summary>
    [Fact] // REQ-SCN-070, REQ-SCN-016
    public void R4_the_wheel_stops_at_sixteen_forward_and_is_free_backward()
    {
        SceneCamera camera = Reset();

        camera.Current.Distance = 25f;
        CameraControls.ApplyWheel(camera, 1f, CameraAction.None);
        Assert.Equal(20f, camera.Current.Distance, 4);
        CameraControls.ApplyWheel(camera, 1f, CameraAction.None);
        Assert.Equal(16f, camera.Current.Distance, 4);
        CameraControls.ApplyWheel(camera, 4f, CameraAction.None);
        Assert.Equal(16f, camera.Current.Distance, 4);

        // From below the floor a forward notch does nothing.
        camera.Current.Distance = 5f;
        CameraControls.ApplyWheel(camera, 1f, CameraAction.None);
        Assert.Equal(5f, camera.Current.Distance, 4);

        // Backward is untouched: 5 units a notch, up to the 512 of REQ-SCN-011.
        CameraControls.ApplyWheel(camera, -1f, CameraAction.None);
        Assert.Equal(10f, camera.Current.Distance, 4);
        CameraControls.ApplyWheel(camera, -200f, CameraAction.None);
        Assert.Equal(512f, camera.Current.Distance, 4);
    }

    /// <summary>
    /// REQ-SCN-070 — "the Lua write of <c>Camera.Distance</c> keeps its bounds
    /// <c>[0, 512 × s]</c>": the floor of 16 is a user-zoom rule, not a camera bound, so a
    /// script still posts 5 and reads 5 back.
    /// </summary>
    [Fact] // REQ-SCN-070, REQ-LUA-088
    public void R4_a_script_may_still_post_a_distance_under_sixteen()
    {
        using LuaRun run = Lua();

        run.Do("Camera.Distance = 5");
        Assert.Equal(5.0, run.Num("Camera.Distance"), 1e-3);
        run.Do("Camera.Distance = 0");
        Assert.Equal(0.0, run.Num("Camera.Distance"), 1e-3);
        run.Do("Camera.Distance = 1000");
        Assert.Equal(512.0, run.Num("Camera.Distance"), 1e-3);
    }

    // ------------------------------------------------------------- AMD-045 REQ-SCN-071

    /// <summary>
    /// REQ-SCN-071 — the acceptance criterion: holding RotDown for 13 s (390°) leaves
    /// <c>Camera.Rot.x</c> — the setpoint of the current mode, REQ-LUA-088 — inside
    /// <c>]−180, 180]</c>, around 30, and the camera orientation varies continuously: no frame
    /// turns the aimed direction by more than 2°.
    /// </summary>
    [Fact] // REQ-SCN-071
    public void R4_thirteen_seconds_of_rotation_stay_inside_one_turn_without_a_jump()
    {
        SceneCamera camera = Reset();
        Vector3 previous = camera.BuildView(DefaultClipping, 16f / 9f).Forward;
        float worst = 0f;
        float peak = 0f;

        const float Step = 1f / 60f;
        for (int frame = 0; frame < 13 * 60; frame++)
        {
            CameraControls.Apply(camera, CameraAction.RotateDown, Step);
            camera.Advance(Step);

            Vector3 forward = camera.BuildView(DefaultClipping, 16f / 9f).Forward;
            worst = MathF.Max(worst, AngleBetween(previous, forward));
            previous = forward;

            // Every frame, not only the last one, respects the interval.
            peak = MathF.Max(peak, MathF.Abs(camera.Current.Angles.X));
            Assert.InRange(camera.Current.Angles.X, -180f + 1e-3f, 180f);
        }

        Assert.Equal(30.81f, camera.Current.Angles.X, 0.2f);
        Assert.True(peak <= 180f, $"a setpoint angle left the interval: {peak}");
        Assert.True(worst <= 2f, $"the aimed direction jumped by {worst}° in one frame");
    }

    /// <summary>
    /// REQ-SCN-071 — the second half of the criterion: <c>Camera.Rot.y = 270</c> read at the next
    /// frame gives −90. The displayed angle is shifted by the very same turn, so the orientation
    /// does not jump and the smoothing keeps its lag.
    /// </summary>
    [Fact] // REQ-SCN-071
    public void R4_a_setpoint_angle_written_at_270_reads_minus_90_at_the_next_frame()
    {
        SceneCamera camera = Reset();
        camera.Current.Angles = new Vector3(camera.Current.Angles.X, 270f, 0f);

        float displayedBefore = camera.Angles.Y;
        Vector3 before = camera.BuildView(DefaultClipping, 16f / 9f).Forward;

        camera.Advance(1f / 60f);

        Assert.Equal(-90f, camera.Current.Angles.Y, 1e-3f);

        // The displayed angle took the same −360° turn, so the picture did not jump: it moved by
        // the fraction of the lag that one frame of smoothing allows, not by a turn.
        Vector3 after = camera.BuildView(DefaultClipping, 16f / 9f).Forward;
        Assert.True(AngleBetween(before, after) < 2f, $"the picture jumped by {AngleBetween(before, after)}°");
        Assert.True(
            camera.Angles.Y < displayedBefore - 300f,
            $"the displayed angle did not take the turn: {camera.Angles.Y}");
    }

    /// <summary>
    /// REQ-SCN-071 — "each angle (rx, ry, rz) of <b>each</b> mode": the setpoints of the mode
    /// that is not current are wrapped too, and the interval is the half-open
    /// <c>]−180, 180]</c>, so 180 stays 180 and −180 becomes 180.
    /// </summary>
    [Fact] // REQ-SCN-071
    public void R4_both_modes_and_the_three_angles_are_wrapped_into_the_half_open_interval()
    {
        SceneCamera camera = Reset();
        Assert.Equal(CameraMode.Tpv, camera.Mode);

        camera.Setpoints(CameraMode.Tpv).Angles = new Vector3(400f, -400f, 900f);
        camera.Setpoints(CameraMode.Fpv).Angles = new Vector3(270f, -270f, 540f);

        camera.Advance(1f / 60f);

        Vector3 tpv = camera.Setpoints(CameraMode.Tpv).Angles;
        Assert.Equal(40f, tpv.X, 1e-3f);
        Assert.Equal(-40f, tpv.Y, 1e-3f);
        Assert.Equal(180f, tpv.Z, 1e-3f);

        Vector3 fpv = camera.Setpoints(CameraMode.Fpv).Angles;
        Assert.Equal(-90f, fpv.X, 1e-3f);
        Assert.Equal(90f, fpv.Y, 1e-3f);
        Assert.Equal(180f, fpv.Z, 1e-3f);

        // The bounds themselves: 180 is kept, −180 is brought to 180.
        camera.Current.Angles = new Vector3(180f, -180f, 0f);
        camera.Advance(1f / 60f);
        Assert.Equal(180f, camera.Current.Angles.X, 1e-3f);
        Assert.Equal(180f, camera.Current.Angles.Y, 1e-3f);

        // A zero time step still wraps: it happens "before each camera update".
        camera.Current.Angles = new Vector3(361f, 0f, 0f);
        camera.Advance(0f);
        Assert.Equal(1f, camera.Current.Angles.X, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-071 — the user may still orbit without limit: 13 s of RotRight turns the camera
    /// by 390° of real rotation around the screen, which the eye position shows, even though the
    /// setpoint never leaves one turn.
    /// </summary>
    [Fact] // REQ-SCN-071, REQ-SCN-014
    public void R4_the_user_can_still_orbit_without_limit()
    {
        SceneCamera camera = Reset();
        float total = 0f;
        Vector3 previous = camera.BuildView(DefaultClipping, 16f / 9f).Forward;

        const float Step = 1f / 60f;
        for (int frame = 0; frame < 13 * 60; frame++)
        {
            CameraControls.Apply(camera, CameraAction.RotateRight, Step);
            camera.Advance(Step);
            Vector3 forward = camera.BuildView(DefaultClipping, 16f / 9f).Forward;
            total += AngleBetween(previous, forward);
            previous = forward;
        }

        // 13 s × 30°/s = 390° of travel, more than a whole turn.
        Assert.True(total > 360f, $"the camera only travelled {total}°");
        Assert.InRange(camera.Current.Angles.Y, -180f + 1e-3f, 180f);
    }

    // ------------------------------------------------------------- AMD-045 REQ-SCN-072

    /// <summary>
    /// REQ-SCN-072 — in TPV the camera still aims at <c>P*</c> but its vertical is its own,
    /// <c>q·(0, 1, 0)</c>, and no longer the world Y axis: the camera's right axis is across
    /// <c>q·(0, 1, 0)</c>, which the frozen step 4 would have put across the world Y instead.
    /// </summary>
    [Theory] // REQ-SCN-072, REQ-SCN-007
    [InlineData(0f, 0f, 30f)]
    [InlineData(20f, 15f, 30f)]
    [InlineData(45f, 0f, -60f)]
    [InlineData(89.5f, 0f, 0f)]
    public void R4_the_tpv_right_axis_is_across_the_camera_vertical(float rx, float ry, float rz)
    {
        CameraView view = AtAngles(new Vector3(rx, ry, rz));
        (_, Vector3 qUp, _) = SceneConstants.Rotation(new Vector3(rx, ry, rz), localCamera: false);

        // Right = normalize(q·(0,1,0) × forward), so it is across the camera's own vertical.
        Assert.True(
            MathF.Abs(Vector3.Dot(view.Right, Vector3.Normalize(qUp))) < 1e-3f,
            $"right is not across q·(0,1,0): {Vector3.Dot(view.Right, Vector3.Normalize(qUp))}");

        // Still aimed at P* (REQ-SCN-008), which the amendment keeps.
        Assert.True(AngleBetween(view.AimPoint - view.Eye, view.Forward) < 1e-2f);

        // The basis is orthonormal and left-handed, as REQ-SCN-001 fixes it.
        Assert.Equal(1f, view.Right.Length(), 1e-3f);
        Assert.Equal(1f, view.Up.Length(), 1e-3f);
        Assert.True(MathF.Abs(Vector3.Dot(view.Right, view.Forward)) < 1e-3f);
        Assert.True(MathF.Abs(Vector3.Dot(view.Right, view.Up)) < 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-072 — the world vertical of the frozen step 4 is really gone: with rz = 30° the
    /// right axis is well away from the horizontal plane the old rule forced it into.
    /// </summary>
    [Fact] // REQ-SCN-072, REQ-SCN-007
    public void R4_the_tpv_right_axis_is_no_longer_across_the_world_vertical()
    {
        CameraView view = AtAngles(new Vector3(0f, 0f, 30f));

        // The frozen step 4 (« l'axe Y du monde comme verticale, pas de roulis ») would have
        // given dot(right, worldY) = 0; with the camera's own vertical it is sin(30°).
        Assert.Equal(0.5f, MathF.Abs(Vector3.Dot(view.Right, Vector3.UnitY)), 0.01f);
    }

    /// <summary>
    /// REQ-SCN-072 — the first acceptance criterion: in TPV with rz = 0, between rx = 89.5° and
    /// rx = 90.5° the orientation varies by at most 2°, where the world Y axis would have turned
    /// the picture by about 180°.
    /// </summary>
    [Fact] // REQ-SCN-072, REQ-SCN-007
    public void R4_the_tpv_camera_crosses_the_zenith_smoothly()
    {
        CameraView low = AtAngles(new Vector3(89.5f, 0f, 0f));
        CameraView high = AtAngles(new Vector3(90.5f, 0f, 0f));

        Assert.True(AngleBetween(low.Up, high.Up) <= 2f, $"up turned by {AngleBetween(low.Up, high.Up)}°");
        Assert.True(
            AngleBetween(low.Right, high.Right) <= 2f,
            $"right turned by {AngleBetween(low.Right, high.Right)}°");
        Assert.True(AngleBetween(low.Forward, high.Forward) <= 2f);

        // What the frozen step 4 would have given, with the same cross-product rule but the
        // world Y axis: the picture flips.
        float worldFlip = AngleBetween(WorldUpRight(low.Forward), WorldUpRight(high.Forward));
        Assert.True(worldFlip > 150f, $"the world vertical would only have turned by {worldFlip}°");

        static Vector3 WorldUpRight(Vector3 forward)
        {
            Vector3 right = Vector3.Cross(Vector3.UnitY, forward);
            return right.LengthSquared() < 1e-8f ? Vector3.Cross(Vector3.UnitZ, forward) : right;
        }
    }

    /// <summary>
    /// REQ-SCN-072 — the second acceptance criterion and the consequence the requirement draws:
    /// with rx = ry = 0 and rz = 30°, the picture is turned by 30° around the aimed axis, which
    /// the frozen step 4 explicitly forbade ("no roll; rz therefore has no visible effect in
    /// TPV"). The aimed direction itself does not move.
    /// </summary>
    [Fact] // REQ-SCN-072, REQ-SCN-023
    public void R4_the_roll_is_visible_in_tpv()
    {
        CameraView flat = AtAngles(Vector3.Zero);
        CameraView rolled = AtAngles(new Vector3(0f, 0f, 30f));

        Assert.Equal(0f, Roll(flat), 0.1f);
        Assert.Equal(30f, Roll(rolled), 0.2f);
        Assert.True(AngleBetween(flat.Forward, rolled.Forward) < 0.1f);

        // −30° turns it the other way, by the same amount, so it really is rz that rolls.
        Assert.Equal(30f, Roll(AtAngles(new Vector3(0f, 0f, -30f))), 0.2f);
        Assert.True(
            Vector3.Dot(rolled.Right, Vector3.UnitY) * Vector3.Dot(AtAngles(new Vector3(0f, 0f, -30f)).Right, Vector3.UnitY) < 0f,
            "the two rolls turn the picture the same way");
    }

    /// <summary>
    /// REQ-SCN-072 — "TPV and the local camera then use the same vertical; the only difference
    /// left is the orientation towards P*": in both cases the vertical handed to the basis is
    /// <c>q·(0, 1, 0)</c> for that mode's own <c>q</c> (REQ-SCN-007 step 1, REQ-SCN-009), so the
    /// local camera's right axis is across its own <c>q·(0, 1, 0)</c> as well.
    /// </summary>
    [Fact] // REQ-SCN-072, REQ-SCN-009
    public void R4_tpv_and_the_local_camera_use_the_same_kind_of_vertical()
    {
        Vector3 angles = new(20f, 15f, 30f);

        SceneCamera local = Reset();
        local.LocalCamera = true;
        local.Current.Angles = angles;
        local.SnapToSetpoints();
        CameraView localView = local.BuildView(DefaultClipping, 16f / 9f);

        (Vector3 lRight, Vector3 lUp, Vector3 lForward) = SceneConstants.Rotation(angles, localCamera: true);
        Assert.True(AngleBetween(localView.Up, lUp) < 1e-2f);
        Assert.True(AngleBetween(localView.Right, lRight) < 1e-2f);
        Assert.True(AngleBetween(localView.Forward, lForward) < 1e-2f);

        // The plain TPV camera does the same with its own q, but keeps the aim at P*: the only
        // difference the requirement leaves between the two.
        CameraView tpvView = AtAngles(angles);
        (_, Vector3 qUp, Vector3 qForward) = SceneConstants.Rotation(angles, localCamera: false);
        Assert.True(MathF.Abs(Vector3.Dot(tpvView.Right, Vector3.Normalize(qUp))) < 1e-3f);
        Assert.True(
            AngleBetween(tpvView.Forward, qForward) > 1e-2f,
            "the TPV camera should still be corrected towards P*");
    }

    private static CameraView AtAngles(Vector3 angles)
    {
        SceneCamera camera = Reset();
        camera.Current.Angles = angles;
        camera.SnapToSetpoints();
        return camera.BuildView(DefaultClipping, 16f / 9f);
    }

    // ------------------------------------------------------------- AMD-045 REQ-SCN-073

    /// <summary>
    /// REQ-SCN-073 — the first criterion: save in the default view, turn 90° and zoom, reload;
    /// the first picture after the load already shows the saved view, with no slide.
    /// </summary>
    [Fact] // REQ-SCN-073, REQ-SCN-024
    public void R4_a_reloaded_camera_shows_the_saved_view_on_the_first_picture()
    {
        SceneCamera camera = Reset();
        CameraStateData saved = camera.Capture();
        Vector3 savedEye = camera.BuildView(DefaultClipping, 16f / 9f).Eye;

        // Turn 90° and zoom, and let the smoothing settle on the new view.
        Hold(camera, CameraAction.RotateRight, 3f);
        Hold(camera, CameraAction.ZoomIn, 1f);
        Advance(camera, 5f);
        Assert.Equal(90.81f, camera.Current.Angles.Y, 0.2f);
        Assert.True(MathF.Abs(camera.Angles.Y - 90.81f) < 0.5f);

        camera.Restore(saved);

        // No Advance in between: the displayed values are already the restored setpoints.
        Assert.Equal(saved.Tpv.Distance, camera.Distance, 1e-3f);
        Assert.Equal(saved.Tpv.Angles.X, camera.Angles.X, 1e-3f);
        Assert.Equal(saved.Tpv.Angles.Y, camera.Angles.Y, 1e-3f);
        Assert.Equal(saved.Tpv.Angles.Z, camera.Angles.Z, 1e-3f);
        Assert.Equal(saved.Tpv.Target.X, camera.Target.X, 1e-3f);
        Assert.Equal(saved.Tpv.Target.Y, camera.Target.Y, 1e-3f);

        Vector3 eye = camera.BuildView(DefaultClipping, 16f / 9f).Eye;
        Assert.Equal(savedEye.X, eye.X, 1e-2f);
        Assert.Equal(savedEye.Y, eye.Y, 1e-2f);
        Assert.Equal(savedEye.Z, eye.Z, 1e-2f);

        // The smoothing velocities were reset: the very next frame does not drift away.
        camera.Advance(1f / 60f);
        Assert.Equal(savedEye.Z, camera.BuildView(DefaultClipping, 16f / 9f).Eye.Z, 1e-2f);
    }

    /// <summary>
    /// REQ-SCN-073 — the transition window comes back as a <b>remaining time</b> bounded to
    /// <c>[0, 3]</c> s: saved 1 s after a switch to FPV, it is restored at 2 s, and a corrupt or
    /// stale value is brought back into the interval.
    /// </summary>
    [Fact] // REQ-SCN-073, REQ-SCN-024
    public void R4_the_transition_window_comes_back_as_a_bounded_remaining_time()
    {
        SceneCamera camera = Reset();
        camera.SetMode(CameraMode.Fpv);
        Assert.Equal(3f, camera.TransitionRemaining, 1e-3f);

        Advance(camera, 1f);
        Assert.Equal(2f, camera.TransitionRemaining, 0.05f);

        CameraStateData saved = camera.Capture();
        Advance(camera, 5f);
        Assert.Equal(0f, camera.TransitionRemaining, 1e-3f);

        camera.Restore(saved);
        Assert.Equal(CameraMode.Fpv, camera.Mode);
        Assert.Equal(2f, camera.TransitionRemaining, 0.05f);

        camera.Restore(saved with { TransitionRemaining = 7f });
        Assert.Equal(3f, camera.TransitionRemaining, 1e-3f);
        camera.Restore(saved with { TransitionRemaining = -1f });
        Assert.Equal(0f, camera.TransitionRemaining, 1e-3f);
        camera.Restore(saved with { TransitionRemaining = 1.25f });
        Assert.Equal(1.25f, camera.TransitionRemaining, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-073 — the second criterion: with 2 s of window left after the load, the target is
    /// still joined by the smoothing — about 40 units of the 100 are left after
    /// <c>Tₛ = 0.4 s</c> (REQ-SCN-019) — and once the window is over the camera follows the
    /// target inside the same frame (REQ-SCN-020).
    /// </summary>
    [Fact] // REQ-SCN-073, REQ-SCN-019, REQ-SCN-020
    public void R4_after_a_load_the_remaining_window_still_smooths_the_target()
    {
        SceneCamera camera = Reset();
        camera.SetMode(CameraMode.Fpv);
        Advance(camera, 1f);
        CameraStateData saved = camera.Capture();

        // Something else happened, then the state comes back.
        Advance(camera, 5f);
        camera.Restore(saved);
        Assert.Equal(2f, camera.TransitionRemaining, 0.05f);

        float start = camera.Current.Target.X;
        camera.Current.Target = new Vector3(start + 100f, camera.Current.Target.Y, camera.Current.Target.Z);

        Advance(camera, 0.4f);
        float left = MathF.Abs(camera.Current.Target.X - camera.Target.X);
        Assert.Equal(40f, left, 5f);

        // Past the window, the target is followed inside the frame.
        Advance(camera, 2f);
        Assert.Equal(0f, camera.TransitionRemaining, 1e-3f);
        camera.Current.Target = new Vector3(start + 300f, camera.Current.Target.Y, camera.Current.Target.Z);
        camera.Advance(1f / 60f);
        Assert.Equal(start + 300f, camera.Target.X, 1e-3f);
    }

    /// <summary>
    /// REQ-SCN-073 — the same, through the real savestate path of the host: the camera section
    /// of the file, then the quick load of SPEC-07. The displayed camera the renderer reads right
    /// after the load is the saved one, not the one that was on screen a moment before.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-SCN-073, REQ-SCN-024
    public void R4_the_host_quick_load_snaps_the_camera_onto_the_saved_setpoints()
    {
        using HostDriver host = HostDriver.Create();
        host.Launch2DMode();
        host.RunFrames(5);

        SceneCamera camera = host.Scene.Camera;
        camera.ResetView();
        camera.SnapToSetpoints();
        float savedDistance = camera.Current.Distance;
        Vector3 savedAngles = camera.Current.Angles;

        host.Run(ZaxCommands.QuickSave);

        // Move away and let the picture settle on the new view.
        Hold(camera, CameraAction.RotateRight, 3f);
        Hold(camera, CameraAction.ZoomIn, 1f);
        Advance(camera, 5f);
        Assert.True(MathF.Abs(camera.Angles.Y - savedAngles.Y) > 45f);
        Assert.True(MathF.Abs(camera.Distance - savedDistance) > 20f);

        host.Run(ZaxCommands.QuickLoad);
        host.Settle();

        Assert.Equal(savedDistance, camera.Current.Distance, 1e-2f);
        Assert.Equal(savedDistance, camera.Distance, 1e-2f);
        Assert.Equal(savedAngles.Y, camera.Angles.Y, 1e-2f);
        Assert.Equal(savedAngles.X, camera.Angles.X, 1e-2f);
    }

    /// <summary>
    /// REQ-SCN-073 — the camera section of the savestate file carries the remaining time, so the
    /// bound of <c>[0, 3]</c> s applies to what a file holds and not only to a live capture.
    /// </summary>
    [Fact] // REQ-SCN-073, REQ-SCN-024
    public void R4_the_camera_section_of_a_savestate_round_trips_the_remaining_time()
    {
        SceneCamera camera = Reset();
        camera.SetMode(CameraMode.Fpv);
        Advance(camera, 1f);
        camera.Current.Distance = 123f;

        byte[] block = SaveStateFile.WriteCamera(camera.Capture());
        Assert.Equal(SaveStateFile.CameraSectionBytes, block.Length);

        CameraStateData read = SaveStateFile.ReadCamera(block);
        Assert.Equal(CameraMode.Fpv, read.Mode);
        Assert.Equal(123f, read.Fpv.Distance, 1e-2f);
        Assert.Equal(2f, read.TransitionRemaining, 0.05f);

        SceneCamera fresh = Reset();
        fresh.Restore(read);
        Assert.Equal(CameraMode.Fpv, fresh.Mode);
        Assert.Equal(123f, fresh.Distance, 1e-2f);
        Assert.Equal(2f, fresh.TransitionRemaining, 0.05f);
    }
}
