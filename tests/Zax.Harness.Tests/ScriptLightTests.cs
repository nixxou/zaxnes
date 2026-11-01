using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Harness.Tests;

/// <summary>
/// A script's write on the directional light has to reach the rendered frame, over the light the
/// profile itself stores (SPEC-05 §4.1.3 <c>Light</c>, SPEC-06 REQ-SCN-025, SPEC-09 REQ-PERS-007,
/// REQ-PERS-009, DEC-06-Q5). Several built-in profiles — Mega Man 2 and Castlevania among them —
/// store one light and have a script put another one in force, so the stored value is exactly
/// what must <b>not</b> be rendered.
/// </summary>
public sealed class ScriptLightTests
{
    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-015
    public void A_light_written_in_start_reaches_the_scene_over_the_stored_one()
    {
        SyntheticScene capture = Capture("""
            function Start()
                Light.Color = Color(1, 1, 1)
                Light.Direction = Vector3(1, -1, 1)
            end
            """);

        Assert.Equal(new Rgb24(255, 255, 255), capture.Scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, -1f, 1f)),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-025
    public void A_light_written_by_an_update_hook_reaches_the_scene_over_the_stored_one()
    {
        // The shape of Mega Man 2's « stage » script: every frame picks the light, and the branch
        // the intro takes is the white one.
        SyntheticScene capture = Capture("""
            function Update()
                if frame == -1 then
                    Light.Color = Color(0.2, 0.8, 0)
                    Light.Direction = Vector3(-3, -3, 1)
                else
                    Light.Color = Color(1, 1, 1)
                    Light.Direction = Vector3(1, -1, 1)
                end
            end
            """);

        Assert.Equal(new Rgb24(255, 255, 255), capture.Scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, -1f, 1f)),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-025
    public void An_update_hook_can_also_write_a_light_the_profile_does_not_store()
    {
        // The other direction, so that the test cannot pass by the stored light being ignored
        // altogether: here the script asks for a light neither the profile nor the default holds.
        SyntheticScene capture = Capture("""
            function Update()
                Light.Color = Color(0.2, 0.4, 0.6)
                Light.Direction = Vector3(0, 0, 1)
            end
            """);

        Assert.Equal(new Rgb24(51, 102, 153), capture.Scene.Light.Color);
        Assert.Equal(
            new Vector3(0f, 0f, 1f),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-021, REQ-LUA-025
    public void The_end_of_game_hook_does_not_relight_the_captured_frame()
    {
        // Mega Man 2's « stage » script, whole: Start remembers the profile's light, Update picks
        // one every frame and End puts the remembered one back. The capture shows a frame of a
        // *running* game (GameRunning), so what End writes belongs to after it and must not reach
        // the image — otherwise the frame is lit by the very value Update replaced.
        SyntheticScene capture = Capture("""
            function Start()
                oldColor, oldDir = Light.Color, Light.Direction
            end

            function End()
                Light.Color, Light.Direction = oldColor, oldDir
            end

            function Update()
                Light.Color = Color(1, 1, 1)
                Light.Direction = Vector3(1, -1, 1)
            end
            """);

        Assert.Equal(new Rgb24(255, 255, 255), capture.Scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, -1f, 1f)),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-021
    public void The_end_of_game_hook_does_not_relight_a_frame_lit_by_start_either()
    {
        // Castlevania's « ui » script: Start dims the light for the game, End puts white back.
        SyntheticScene capture = Capture("""
            function Start()
                Light.Color = Color(0.8, 0.8, 0.8)
            end

            function End()
                Light.Color = Color(1, 1, 1)
            end
            """);

        Assert.Equal(new Rgb24(204, 204, 204), capture.Scene.Light.Color);
    }

    [Fact] // REQ-SCN-025, REQ-PROF-041
    public void Without_a_script_the_stored_light_is_the_one_rendered()
    {
        // The control: the stored light the three tests above override really is in force when no
        // script touches it, so they measure a script write and not a lost profile setting.
        SyntheticScene capture = Capture(string.Empty);

        Assert.Equal(new Rgb24(0x33, 0xCC, 0x00), capture.Scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(-3f, -3f, 1f)),
            Vector3.Normalize(capture.Scene.Light.Direction),
            Approximately);
    }

    private const float Tolerance = 1e-4f;

    private static readonly IEqualityComparer<Vector3> Approximately = new VectorComparer();

    /// <summary>
    /// Captures a profile that <b>stores</b> Mega Man 2's green light and carries
    /// <paramref name="source"/> as its only script.
    /// </summary>
    private static SyntheticScene Capture(string source)
    {
        GameProfile profile = CaptureHarness.WithScript(source);
        profile.Settings!.Light!.Color = new ArgbColor(unchecked((int)0xFF33CC00));
        profile.Settings.Light.Direction = new Vec3F(-3f, -3f, 1f);
        profile.Settings.Light.Intensity = 0.93f;
        return CaptureHarness.Capture(profile);
    }

    private sealed class VectorComparer : IEqualityComparer<Vector3>
    {
        public bool Equals(Vector3 x, Vector3 y) => (x - y).Length() <= Tolerance;

        public int GetHashCode(Vector3 value) => 0;
    }
}
