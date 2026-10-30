using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Harness.Tests;

/// <summary>
/// A profile's <b>embedded</b> settings of scope P (SPEC-03 §4.6, applied by REQ-PROF-041) have
/// to reach the capture, the way they reach the application's scene: through the settings model
/// (SPEC-09 REQ-PERS-005), after the chunks and before <c>Start</c> (REQ-LUA-015), with a
/// command-line override still coming last.
/// </summary>
public sealed class ProfileSettingsTests
{
    [Fact] // REQ-SCN-025, REQ-PROF-041, REQ-PERS-005
    public void An_embedded_light_direction_reaches_the_capture()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Settings!.Light!.Direction = new Vec3F(1f, 1f, 2f);

        SyntheticScene capture = CaptureHarness.Capture(profile);

        // REQ-SCN-025: the value is the propagation direction of the rays, normalised at use, so
        // the profile's vector and its normalised form designate the same light.
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, 1f, 2f)),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);

        // And it really is no longer the default (1, −1, 1) the capture showed before, which is
        // the gap this covers.
        Assert.NotEqual(
            Vector3.Normalize(new Vector3(1f, -1f, 1f)),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-025, REQ-PROF-041
    public void An_embedded_light_colour_reaches_the_capture()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Settings!.Light!.Color = new ArgbColor(unchecked((int)0xFF4080C0));

        SyntheticScene capture = CaptureHarness.Capture(profile);

        Assert.Equal(new Rgb24(0x40, 0x80, 0xC0), capture.Scene.Light.Color);
    }

    [Fact] // REQ-SCN-009, REQ-PROF-041, REQ-PERS-005
    public void An_embedded_local_camera_reaches_the_capture()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Settings!.LocalCamera = true;

        Assert.True(CaptureHarness.Capture(profile).Scene.Camera.LocalCamera);

        // The default profile leaves it off, so the flag really comes from the file.
        Assert.False(CaptureHarness.Capture(GameProfile.CreateEmpty()).Scene.Camera.LocalCamera);
    }

    [Fact] // REQ-SCN-003, REQ-PROF-041, AMD-047
    public void The_embedded_clip_margins_reach_the_capture_and_the_engine()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Settings!.Clipping = new Vec4F(2f, -3f, 4f, 5f);

        SyntheticScene capture = CaptureHarness.Capture(profile);

        Assert.Equal(new Vec4F(2f, -3f, 4f, 5f), capture.Scene.Clipping);
        Assert.Equal(new Vec4F(2f, -3f, 4f, 5f), capture.Engine.Settings.Clipping);
    }

    [Fact] // REQ-LUA-015, REQ-PROF-041
    public void The_embedded_settings_are_applied_after_the_chunks()
    {
        // REQ-LUA-015 step 1 runs the chunks, step 2 applies the profile's settings: an
        // assignment made at the main level of a chunk is therefore overwritten.
        GameProfile profile = CaptureHarness.WithScript("Setting.LocalCam = false");
        profile.Settings!.LocalCamera = true;

        Assert.True(CaptureHarness.Capture(profile).Scene.Camera.LocalCamera);
    }

    [Fact] // REQ-LUA-015, REQ-LUA-020, REQ-PROF-041
    public void A_start_hook_still_wins_over_the_embedded_settings()
    {
        // Step 4 comes after step 2, so what Start writes is what the capture shows.
        GameProfile profile = CaptureHarness.WithScript("""
            function Start()
                Setting.LocalCam = false
            end
            """);
        profile.Settings!.LocalCamera = true;

        Assert.False(CaptureHarness.Capture(profile).Scene.Camera.LocalCamera);
    }

    [Fact] // REQ-SCN-012, REQ-SCN-049, REQ-PROF-041
    public void A_command_line_override_wins_over_the_profile_and_its_scripts()
    {
        GameProfile profile = CaptureHarness.WithScript("""
            function Start()
                Setting.Fov = 50
                Setting.Skybox = 1
            end
            """);
        profile.Settings!.Light!.Direction = new Vec3F(1f, 1f, 2f);
        profile.Settings.LocalCamera = true;

        SyntheticScene capture = CaptureHarness.Capture(profile);
        Assert.Equal(50f, capture.Scene.Settings.Fov);
        Assert.Equal(SkyboxKind.Gradient, capture.Scene.Settings.Skybox);

        new SceneOverrides { Fov = 95f, Skybox = SkyboxKind.Fantasy }.ApplyTo(capture);

        Assert.Equal(95f, capture.Scene.Settings.Fov);
        Assert.Equal(95f, capture.Scene.Camera.Fov);
        Assert.Equal(SkyboxKind.Fantasy, capture.Scene.Settings.Skybox);

        // The override pass only writes what the command line gave: the profile's own P settings
        // are still what the capture renders with.
        Assert.True(capture.Scene.Camera.LocalCamera);
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, 1f, 2f)),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-049, REQ-SCN-046, REQ-SCN-005
    public void An_option_left_out_keeps_what_the_profile_and_its_scripts_set()
    {
        GameProfile profile = CaptureHarness.WithScript("""
            function Start()
                Setting.Fov = 50
                Setting.Skybox = 1
            end
            """);
        profile.Settings!.LocalCamera = true;

        SyntheticScene capture = CaptureHarness.Capture(profile);
        new SceneOverrides().ApplyTo(capture);

        Assert.Equal(50f, capture.Scene.Settings.Fov);
        Assert.Equal(SkyboxKind.Gradient, capture.Scene.Settings.Skybox);
        Assert.True(capture.Scene.Camera.LocalCamera);
    }

    [Fact] // REQ-E3D-340
    public void The_mesh_algorithm_the_command_line_asks_for_reaches_the_conversion()
    {
        // A user setting, so it travels through the model like any other: a value written
        // straight into the engine slice would be put back to the default by the first mirror.
        SyntheticScene capture = CaptureHarness.Capture(
            GameProfile.CreateEmpty(),
            mesh: RenderAlgorithm.Marching);

        Assert.Equal(RenderAlgorithm.Marching, capture.Engine.Settings.RenderAlgorithm);
    }

    private const float Tolerance = 1e-4f;

    private static readonly IEqualityComparer<Vector3> Approximately = new VectorComparer();

    private sealed class VectorComparer : IEqualityComparer<Vector3>
    {
        public bool Equals(Vector3 x, Vector3 y) => (x - y).Length() <= Tolerance;

        public int GetHashCode(Vector3 value) => 0;
    }
}
