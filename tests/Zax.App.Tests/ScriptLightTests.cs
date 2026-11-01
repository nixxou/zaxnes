using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// A script's write on the directional light has to reach the scene the window renders, over the
/// light the profile itself stores (SPEC-05 §4.1.3 <c>Light</c>, SPEC-06 REQ-SCN-025, SPEC-09
/// REQ-PERS-007, REQ-PERS-009, REQ-PERS-010, DEC-06-Q5). Several built-in profiles — Mega Man 2
/// and Castlevania among them — store one light and have a script put another one in force, so
/// the stored value is exactly what the game must <b>not</b> be lit by.
/// </summary>
public sealed class ScriptLightTests : IDisposable
{
    private readonly string _romPath = Path.Combine(
        Path.GetTempPath(),
        $"zax-light-{Guid.NewGuid():N}.nes");

    public ScriptLightTests() => File.WriteAllBytes(_romPath, NromImage());

    /// <inheritdoc/>
    public void Dispose() => File.Delete(_romPath);

    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-015, REQ-LUA-020
    public void A_light_written_in_start_reaches_the_scene_over_the_stored_one()
    {
        using Session session = Open("""
            function Start()
                Light.Color = Color(1, 1, 1)
                Light.Direction = Vector3(1, -1, 1)
            end
            """);

        Assert.Equal(new Rgb24(255, 255, 255), session.Scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, -1f, 1f)),
            Vector3.Normalize(session.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-025
    public void A_light_written_by_an_update_hook_reaches_the_next_rendered_frame()
    {
        // Mega Man 2's « stage » script in miniature: Start remembers the stored light, every
        // frame picks one, End puts the remembered one back. The scene the window renders between
        // two frames must show what Update wrote, never what End will.
        using Session session = Open("""
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

        // Start ran but Update has not: the stored light is still the one in force.
        Assert.Equal(new Rgb24(0x33, 0xCC, 0x00), session.Scene.Light.Color);

        session.Game.RunFrame(0);

        Assert.Equal(new Rgb24(255, 255, 255), session.Scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(1f, -1f, 1f)),
            Vector3.Normalize(session.Scene.Light.Direction),
            Approximately);

        // And it keeps holding frame after frame, which is what a per-frame hook is for.
        session.Game.RunFrame(0);
        Assert.Equal(new Rgb24(255, 255, 255), session.Scene.Light.Color);
    }

    [Fact] // REQ-SCN-025, REQ-PERS-007, REQ-LUA-025
    public void An_update_hook_can_change_the_light_from_one_frame_to_the_next()
    {
        // The light follows a value the game writes in memory, which is how the real « stage »
        // script works: $2A decides, and the scene has to follow it at the next frame.
        using Session session = Open("""
            function Update()
                if Script:ReadMem(0x002A) == 6 then
                    Light.Color = Color(0.2, 0.8, 0)
                else
                    Light.Color = Color(1, 1, 1)
                end
            end
            """);

        session.Game.RunFrame(0);
        Assert.Equal(new Rgb24(255, 255, 255), session.Scene.Light.Color);

        session.Game.Scripts.Evaluate("Script:WriteMem(0x002A, 6)");
        session.Game.RunFrame(0);
        Assert.Equal(new Rgb24(51, 204, 0), session.Scene.Light.Color);

        session.Game.Scripts.Evaluate("Script:WriteMem(0x002A, 7)");
        session.Game.RunFrame(0);
        Assert.Equal(new Rgb24(255, 255, 255), session.Scene.Light.Color);
    }

    [Fact] // REQ-SCN-025, REQ-PROF-041, REQ-PERS-005
    public void Without_a_script_the_stored_light_is_the_one_in_force()
    {
        // The control of the three tests above: the stored light they override really is applied,
        // so what they measure is a script write and not a lost profile setting.
        using Session session = Open(string.Empty);

        Assert.Equal(new Rgb24(0x33, 0xCC, 0x00), session.Scene.Light.Color);
        Assert.Equal(
            Vector3.Normalize(new Vector3(-3f, -3f, 1f)),
            Vector3.Normalize(session.Scene.Light.Direction),
            Approximately);
    }

    [Fact] // REQ-SCN-025, REQ-LUA-021, REQ-PERS-005
    public void What_the_end_of_game_hook_writes_does_not_light_the_next_game()
    {
        // The counterpart of the harness's seal. In the application the End hook of REQ-LUA-021
        // runs when the game stops, so its write is in force only while nothing is displayed;
        // loading the next game puts the default light back (ApplyProfileScene, REQ-SCN-025)
        // before that profile's own is pushed, and the write cannot reach a rendered frame.
        SceneState scene = new();
        GameProfile first = Profile("""
            function End()
                Light.Color = Color(0.2, 0.8, 0)
                Light.Direction = Vector3(-3, -3, 1)
            end

            function Update()
                Light.Color = Color(1, 1, 1)
                Light.Direction = Vector3(1, -1, 1)
            end
            """);

        using (GameSession game = GameSession.Load(_romPath, first, "First", scene))
        {
            game.ApplyProfileSettings(scene, first.Settings);
            game.RunFrame(0);
            Assert.Equal(new Rgb24(255, 255, 255), scene.Light.Color);

            game.End();
            Assert.Equal(new Rgb24(51, 204, 0), scene.Light.Color);
        }

        // The next game, which stores no light of its own: the default of REQ-SCN-025 is what it
        // is lit by, not the green the previous game's End hook left behind.
        GameProfile second = GameProfile.CreateEmpty();
        using (GameSession game = GameSession.Load(_romPath, second, "Second", scene))
        {
            game.ApplyProfileSettings(scene, second.Settings);

            Assert.Equal(new Rgb24(255, 255, 255), scene.Light.Color);
            Assert.Equal(
                Vector3.Normalize(new Vector3(1f, -1f, 1f)),
                Vector3.Normalize(scene.Light.Direction),
                Approximately);
        }
    }

    private const float Tolerance = 1e-4f;

    private static readonly IEqualityComparer<Vector3> Approximately = new VectorComparer();

    /// <summary>
    /// A session over a profile that <b>stores</b> Mega Man 2's green light and carries
    /// <paramref name="source"/> as its only script; an empty source leaves it without one.
    /// </summary>
    private Session Open(string source)
    {
        GameProfile profile = Profile(source);
        SceneState scene = new();
        GameSession game = GameSession.Load(_romPath, profile, "Test", scene);
        game.ApplyProfileSettings(scene, profile.Settings);
        return new Session(game, scene);
    }

    /// <summary>
    /// A profile that <b>stores</b> Mega Man 2's green light and carries <paramref name="source"/>
    /// as its only script; an empty source leaves it without one.
    /// </summary>
    private static GameProfile Profile(string source)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        if (source.Length > 0)
        {
            profile.Scripts.Add(new ProfileScript("a", true, source, "*"));
        }

        profile.Settings!.Light!.Color = new ArgbColor(unchecked((int)0xFF33CC00));
        profile.Settings.Light.Direction = new Vec3F(-3f, -3f, 1f);
        profile.Settings.Light.Intensity = 0.93f;
        return profile;
    }

    /// <summary>A 32 KiB NROM image whose three vectors point at an endless loop.</summary>
    private static byte[] NromImage()
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2;
        image[5] = 1;
        image[6] = 0x01;

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

    private sealed class Session(GameSession game, SceneState scene) : IDisposable
    {
        public GameSession Game { get; } = game;

        public SceneState Scene { get; } = scene;

        public void Dispose() => Game.Dispose();
    }

    private sealed class VectorComparer : IEqualityComparer<Vector3>
    {
        public bool Equals(Vector3 x, Vector3 y) => (x - y).Length() <= Tolerance;

        public int GetHashCode(Vector3 value) => 0;
    }
}
