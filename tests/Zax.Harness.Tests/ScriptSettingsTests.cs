using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Harness.Tests;

/// <summary>
/// The scene-owned settings a profile's scripts write have to reach the capture, whenever they
/// write them: in a chunk, in <c>Start</c> or in a frame hook (SPEC-05 REQ-LUA-015,
/// SPEC-09 REQ-PERS-007).
/// </summary>
public sealed class ScriptSettingsTests
{
    [Fact] // REQ-SCN-009, REQ-LUA-015, REQ-PERS-007
    public void Local_cam_set_in_start_reaches_the_captured_camera()
    {
        SyntheticScene capture = Capture("""
            function Start()
                Setting.LocalCam = true
            end
            """);

        Assert.True(capture.Scene.Camera.LocalCamera);
    }

    [Fact] // REQ-SCN-009, REQ-SCN-008
    public void A_local_camera_does_not_orient_towards_the_aim_point()
    {
        SyntheticScene capture = Capture("""
            function Start()
                Setting.LocalCam = true
            end
            """);

        SceneCamera camera = capture.Scene.Camera;
        CameraView view = camera.BuildView(capture.Scene.Clipping, 4f / 3f);

        // REQ-SCN-009: the camera looks exactly along q·(0, 0, 1), so the aim point is one unit
        // ahead of the eye and nothing is aimed at.
        Assert.Equal(
            SceneConstants.Forward(camera.Angles, localCamera: true),
            view.Forward,
            Approximately);
        Assert.Equal(view.Eye + view.Forward, view.AimPoint, Approximately);

        // And it is really a different orientation from the P* one of REQ-SCN-008, which is
        // what the harness captured before: same angles, other forward.
        camera.LocalCamera = false;
        Assert.NotEqual(
            view.Forward,
            camera.BuildView(capture.Scene.Clipping, 4f / 3f).Forward,
            Approximately);
    }

    [Fact] // REQ-SCN-009, REQ-LUA-025, REQ-PERS-007
    public void Local_cam_turned_on_by_a_frame_hook_reaches_the_captured_camera()
    {
        // Off at the start, on from the fourth frame: only a change pushed while the run goes on
        // can show up in the capture.
        SyntheticScene capture = Capture(
            """
            local frames = 0

            function Start()
                Setting.LocalCam = false
            end

            function Update()
                frames = frames + 1
                Setting.LocalCam = frames >= 4
            end
            """,
            runFrames: 8);

        Assert.True(capture.Scene.Camera.LocalCamera);
    }

    [Fact] // REQ-SCN-009, REQ-LUA-025
    public void A_frame_hook_can_also_turn_the_local_camera_back_off()
    {
        SyntheticScene capture = Capture(
            """
            local frames = 0

            function Start()
                Setting.LocalCam = true
            end

            function Update()
                frames = frames + 1
                Setting.LocalCam = frames < 3
            end
            """,
            runFrames: 8);

        Assert.False(capture.Scene.Camera.LocalCamera);
    }

    [Fact] // REQ-SCN-012, REQ-SCN-069, REQ-LUA-015
    public void A_field_of_view_set_in_start_reaches_the_scene_and_resets_the_view()
    {
        SyntheticScene capture = Capture("""
            function Start()
                Setting.Fov = 90
            end
            """);

        Assert.Equal(90f, capture.Scene.Settings.Fov);
        Assert.Equal(90f, capture.Scene.Camera.Fov);

        // AMD-045 REQ-SCN-069: the view reset the change triggers writes d₀ = 132.7964 / tan(45°).
        Assert.Equal(SceneConstants.ResetDistance(90f), capture.Scene.Camera.Distance, 3);
    }

    [Fact] // REQ-SCN-030, REQ-SCN-046, REQ-SCN-049
    public void The_other_scene_owned_settings_of_a_start_hook_reach_the_scene()
    {
        SyntheticScene capture = Capture("""
            function Start()
                Setting.Shadow = false
                Setting.Skybox = 1
                Setting.CustomBgColor = Color(0.2, 0.4, 0.6)
                Setting.BgColor = false
                Setting.SmoothTime = 0.25
            end
            """);

        Assert.False(capture.Scene.Settings.Shadows);
        Assert.Equal(SkyboxKind.Gradient, capture.Scene.Settings.Skybox);
        Assert.False(capture.Scene.UseNesBackgroundColor);
        Assert.Equal(0.25f, capture.Scene.Camera.SmoothTime, 4);

        Rgb24 background = capture.Scene.Settings.CustomBackgroundColor;
        Assert.Equal(51, background.R);
        Assert.Equal(102, background.G);
        Assert.Equal(153, background.B);
    }

    [Fact] // REQ-ARCH-031, REQ-PROF-022
    public void The_capture_description_names_the_profile_that_is_active()
    {
        SyntheticScene capture = Capture("", title: "Jackal");
        Assert.Contains("profile 'Jackal'", capture.Description, StringComparison.Ordinal);
        Assert.DoesNotContain("2D Mode", capture.Description, StringComparison.Ordinal);
    }

    [Fact] // REQ-ARCH-031, REQ-PROF-022
    public void Only_the_absent_profile_is_labelled_two_dimensional_mode()
    {
        _ = RenderExport.LoadProfile(null, null, out string none);
        Assert.Equal("2D Mode", none);

        GameProfile titled = GameProfile.CreateEmpty();
        titled.Title = "Jackal";
        Assert.Equal("Jackal", LabelOfFile(titled, "whatever.bytes"));

        // A profile file with no title of its own: the capture names the file, never « 2D Mode ».
        Assert.Equal("Castlevania (U)", LabelOfFile(GameProfile.CreateEmpty(), "Castlevania (U).bytes"));
    }

    private static string LabelOfFile(GameProfile profile, string fileName)
    {
        string directory = Path.Combine(Path.GetTempPath(), $"zax-harness-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        string path = Path.Combine(directory, fileName);
        try
        {
            File.WriteAllBytes(path, ProfileFormat.Save(profile));
            _ = RenderExport.LoadProfile(path, null, out string label);
            return label;
        }
        finally
        {
            Directory.Delete(directory, recursive: true);
        }
    }

    private const float Tolerance = 1e-4f;

    private static readonly IEqualityComparer<Vector3> Approximately = new VectorComparer();

    /// <summary>
    /// Runs the harness's own <c>render --rom</c> path on a blank NROM image and a profile
    /// carrying <paramref name="source"/> as its only script.
    /// </summary>
    private static SyntheticScene Capture(string source, int runFrames = 4, string? title = null)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        if (source.Length > 0)
        {
            profile.Scripts.Add(new ProfileScript("a.lua", true, source, "*"));
        }

        string rom = Path.Combine(
            Path.GetTempPath(),
            $"zax-harness-{Guid.NewGuid():N}.nes");
        File.WriteAllBytes(rom, BlankRom());
        try
        {
            return RenderExport.FromRom(
                rom,
                runFrames,
                profile,
                title ?? "2D Mode",
                RenderAlgorithm.Greedy,
                InputScript.Parse(null));
        }
        finally
        {
            File.Delete(rom);
        }
    }

    /// <summary>A 32 KiB PRG + 8 KiB CHR NROM image whose program is an endless loop.</summary>
    private static byte[] BlankRom()
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2;
        image[5] = 1;
        image[6] = 0x01;

        // JMP $8000 at the reset vector, which the three vectors all point to.
        image[16] = 0x4C;
        image[17] = 0x00;
        image[18] = 0x80;
        for (int vector = 0; vector < 3; vector++)
        {
            image[16 + (32 * 1024) - 6 + (vector * 2)] = 0x00;
            image[16 + (32 * 1024) - 5 + (vector * 2)] = 0x80;
        }

        return image;
    }

    private sealed class VectorComparer : IEqualityComparer<Vector3>
    {
        public bool Equals(Vector3 x, Vector3 y) => (x - y).Length() <= Tolerance;

        public int GetHashCode(Vector3 value) => 0;
    }
}
