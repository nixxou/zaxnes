using Zax.App;
using Zax.Profiles;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// The Lua runtime seen from the session: the load order of REQ-LUA-015, the game start and end
/// of REQ-LUA-020/021, the savestate sequence of REQ-LUA-071/072 (DEC-R2-T6), the libretro
/// sequence of REQ-LUA-075 and the reset of REQ-LUA-013.
/// </summary>
public sealed class GameSessionScriptTests : IDisposable
{
    private readonly string _romPath = Path.Combine(
        Path.GetTempPath(),
        $"zax-session-{Guid.NewGuid():N}.nes");

    public GameSessionScriptTests() => File.WriteAllBytes(_romPath, NromImage());

    /// <inheritdoc/>
    public void Dispose() => File.Delete(_romPath);

    /// <summary>
    /// REQ-LUA-020: starting the game calls <c>Start</c> of every script in script order, and a
    /// script's write to a setting reaches the scene (DEC-06-Q5).
    /// </summary>
    [Fact]
    public void Starting_the_game_calls_Start_in_script_order_REQ_LUA_020()
    {
        using Session session = Session.Open(
            _romPath,
            Script("b", "function Start() log[#log + 1] = 'b' end"),
            Script("a", "function Start() log[#log + 1] = 'a' end"),
            Prelude("log = {}"));

        Assert.Equal(["a", "b"], session.Log());
        Assert.Equal(["!prelude", "a", "b"], session.Game.ScriptNames);
    }

    /// <summary>
    /// REQ-LUA-021 and REQ-LUA-090: ending the game calls <c>End</c>, then the end-of-game
    /// resets put the backdrop back to its default state.
    /// </summary>
    [Fact]
    public void Ending_the_game_calls_End_then_resets_the_backdrop_REQ_LUA_021()
    {
        using Session session = Session.Open(
            _romPath,
            Script("a", """
                function Start() Backdrop.Enable = true; Backdrop.OfsZ = 17.1 end
                function End() log[#log + 1] = 'End'; log[#log + 1] = Backdrop.Enable end
                """),
            Prelude("log = {}"));

        Assert.True(session.Scene.Backdrop.Enabled);
        Assert.Equal(17.1f, session.Scene.Backdrop.Offset.Z, 4);

        session.Game.End();

        // End saw the state it had set; the reset of REQ-LUA-090 runs after it.
        Assert.Equal(["End", "true"], session.Log());
        Assert.False(session.Scene.Backdrop.Enabled);
        Assert.Equal(0f, session.Scene.Backdrop.Offset.Z);
    }

    /// <summary>
    /// REQ-LUA-071 and REQ-LUA-072 with the order of DEC-R2-T6: <c>Save</c> then the block, then
    /// on the way back <c>End</c>, the core, <c>PData</c>, <c>Start</c> and <c>Load</c>.
    /// </summary>
    [Fact]
    public void The_savestate_sequence_follows_REQ_LUA_072_and_DEC_R2_T6()
    {
        using Session session = Session.Open(
            _romPath,
            Script("_scene", """
                scene = 0
                function Save() PData[6] = scene; log[#log + 1] = 'Save' end
                function Start() log[#log + 1] = 'Start(' .. tostring(PData[6]) .. ')' end
                function Load() scene = PData[6]; log[#log + 1] = 'Load' end
                function End() log[#log + 1] = 'End' end
                """),
            Prelude("log = {}"));

        session.Game.Scripts.Evaluate("scene = 2");
        byte[] block = session.Game.CaptureScriptState();

        // The savestate kept scene 2; the running game moves on to something else.
        session.Game.Scripts.Evaluate("scene = 0; PData[6] = 0");
        session.Game.RunFrame(0);

        session.Game.BeginScriptRestore();
        session.Game.CompleteScriptRestore(block);

        Assert.Equal(["Start(0)", "Save", "End", "Start(2)", "Load"], session.Log());

        // Start already saw the restored entry, and Load pushed it into the script's own state.
        Assert.Equal(2.0, session.Game.Scripts.EvaluateNumber("return scene"));
        Assert.Equal(0, session.Game.FrameCounter);
    }

    /// <summary>
    /// REQ-LUA-075 and DEC-R2-D5: the libretro path restores the entries and calls <c>Load</c>
    /// only — the profile stays loaded, so no <c>End</c> and no <c>Start</c>.
    /// </summary>
    [Fact]
    public void The_libretro_sequence_calls_Load_alone_REQ_LUA_075()
    {
        using Session session = Session.Open(
            _romPath,
            Script("a", """
                function Save() PData[1] = 42; log[#log + 1] = 'Save' end
                function Load() log[#log + 1] = 'Load(' .. tostring(PData[1]) .. ')' end
                function Start() log[#log + 1] = 'Start' end
                function End() log[#log + 1] = 'End' end
                """),
            Prelude("log = {}"));

        byte[] block = session.Game.CaptureScriptState();
        session.Game.Scripts.Evaluate("PData[1] = 0");
        session.Game.RestoreScriptStateInPlace(block);

        Assert.Equal(["Start", "Save", "Load(42)"], session.Log());
    }

    /// <summary>
    /// REQ-LUA-013 and REQ-LUA-062 (DEC-05-Q5): a reset of the same game calls <c>End</c> then
    /// <c>Start</c> without re-running the chunks, and keeps the error flags.
    /// </summary>
    [Fact]
    public void A_reset_keeps_the_lua_state_REQ_LUA_013()
    {
        using Session session = Session.Open(
            _romPath,
            Script("a", """
                n = (n or 0) + 1
                function Start() log[#log + 1] = 'Start(' .. tostring(n) .. ')' end
                function End() log[#log + 1] = 'End' end
                """),
            Script("bad", "function Update() local x = nil; return x.y end"),
            Prelude("log = {}"));

        session.Game.RunFrame(0);
        Assert.True(session.Game.Scripts.FindScript("bad")!.Failed);

        session.Game.Reset();

        // The chunk did not run again (n stays 1) and the failed script got neither hook
        // (REQ-LUA-062).
        Assert.Equal(["Start(1)", "End", "Start(1)"], session.Log());
        Assert.True(session.Game.Scripts.FindScript("bad")!.Failed);
        Assert.Equal(0, session.Game.FrameCounter);
    }

    /// <summary>
    /// REQ-LUA-032 with SPEC-08 REQ-INP-071: the script pad adds its bits to the player's mask
    /// before the core runs the frame, and never masks a physical input.
    /// </summary>
    [Fact]
    public void The_script_pad_is_additive_REQ_LUA_032()
    {
        using Session session = Session.Open(
            _romPath,
            Script("pad", """
                function Start() ScriptPad:Enable(true) end
                function InputProcess(index) log[#log + 1] = index; return 8.9 end
                """),
            Prelude("log = {}"));

        session.Game.RunFrame(0x01);

        Assert.Equal(["0"], session.Log());

        // 8.9 truncates to 8 (REQ-LUA-040) and joins the A button of the physical mask.
        Assert.Equal(8, session.Game.Scripts.ScriptPadState);
    }

    private static ProfileScript Script(string name, string source)
        => new(name, true, source, "*");

    private static ProfileScript Prelude(string source)
        => new("!prelude", true, source, string.Empty);

    /// <summary>
    /// A 32 KiB NROM image whose three vectors point at an endless loop, enough for the core to
    /// power on and run frames.
    /// </summary>
    private static byte[] NromImage()
    {
        byte[] image = new byte[16 + (32 * 1024) + (8 * 1024)];
        image[0] = 0x4E;
        image[1] = 0x45;
        image[2] = 0x53;
        image[3] = 0x1A;
        image[4] = 2; // 2 × 16 KiB PRG.
        image[5] = 1; // 1 × 8 KiB CHR.
        image[6] = 0x01; // Vertical mirroring.

        // JMP $8000 at $8000, and the three vectors pointing there.
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

    /// <summary>A session over the synthetic ROM and a profile built from the given scripts.</summary>
    private sealed class Session : IDisposable
    {
        private Session(GameSession game, SceneState scene)
        {
            Game = game;
            Scene = scene;
        }

        public GameSession Game { get; }

        public SceneState Scene { get; }

        public static Session Open(string romPath, params ProfileScript[] scripts)
        {
            GameProfile profile = GameProfile.CreateEmpty();
            foreach (ProfileScript script in scripts)
            {
                profile.Scripts.Add(script);
            }

            SceneState scene = new();
            GameSession game = GameSession.Load(romPath, profile, "Test", scene);
            game.ApplyProfileSettings(scene, profile.Settings);
            return new Session(game, scene);
        }

        /// <summary>The global <c>log</c> table the scripts fill, as strings.</summary>
        public string[] Log()
        {
            int count = (int)Game.Scripts.EvaluateNumber("return #log");
            string[] entries = new string[count];
            for (int i = 0; i < count; i++)
            {
                entries[i] = Game.Scripts.EvaluateText($"return tostring(log[{i + 1}])");
            }

            return entries;
        }

        public void Dispose() => Game.Dispose();
    }
}
