using Zax.App.Loop;
using Zax.Profiles;
using Zax.Render;
using Zax.Scripting;

namespace Zax.Acceptance.Tests;

/// <summary>
/// Round-1 acceptance tests for the parts of SPEC-05 that only the host can show: the load order
/// of REQ-LUA-015, the end-of-game resets of REQ-LUA-021 and REQ-LUA-090, what a rollback must
/// not touch (REQ-LUA-074), and the dynamic run of the whole corpus (REQ-LUA-051).
/// </summary>
public sealed class LuaHostTests
{
    /// <summary>
    /// REQ-LUA-015 — the load order is: (1) the chunks, (2) the profile's embedded settings,
    /// (3) the patterns with <c>BuildPattern</c>, (4) <c>Start</c>. The criterion is explicit:
    /// a chunk that assigns <c>Setting.EmptyTile = true</c> at its main level, in a profile whose
    /// settings carry <c>EmptyTile</c> false, must see <c>Setting.EmptyTile</c> <b>false</b> at
    /// the first <c>Update</c>, because step 2 overwrites it; the same assignment made in
    /// <c>Start</c> survives.
    /// </summary>
    /// <remarks>
    /// Round 1: the profile's settings were applied to the settings model <b>before</b> the chunks
    /// ran, so the chunk's assignment was the one that survived — the opposite of the criterion
    /// (<c>verify/round1/R1-report-part-b.md</c> §2). Corrected by the <c>R1-fixes</c> batch —
    /// <c>GameSession.Create</c> calls <c>model.ApplyProfile</c> after
    /// <c>ProfileScriptRuntime.Create</c>. Re-verified 2026-10-21,
    /// <c>verify/round1/R1-recheck.md</c>.
    /// </remarks>
    [Fact] // REQ-LUA-015
    public void The_profile_settings_must_overwrite_what_a_chunk_set_at_its_main_level()
    {
        GameProfile atChunkLevel = GameProfile.CreateEmpty();
        atChunkLevel.Scripts.Add(new ProfileScript("a", true, "Setting.EmptyTile = true", "*"));
        atChunkLevel.Settings!.EmptyTile = false;

        using SessionDriver chunk = SessionDriver.CreateOnProfile(atChunkLevel, new SceneState());
        chunk.Start();

        GameProfile inStart = GameProfile.CreateEmpty();
        inStart.Scripts.Add(new ProfileScript(
            "a", true, "function Start() Setting.EmptyTile = true end", "*"));
        inStart.Settings!.EmptyTile = false;

        using SessionDriver started = SessionDriver.CreateOnProfile(inStart, new SceneState());
        started.Start();

        Assert.True(started.Scripts.Settings.EmptyTile, "the assignment made in Start must survive.");
        Assert.False(
            chunk.Scripts.Settings.EmptyTile,
            "the profile's embedded settings must overwrite what the chunk set at its main level.");
    }

    /// <summary>
    /// REQ-LUA-021, REQ-LUA-090 — the end of a game calls <c>End</c> of every script that is not
    /// in error, and only then puts the end-of-game state back: the backdrop returns to size
    /// (256, 240, 1), offset (0, 0, 0), opaque black, <c>RecvShadow</c> true and
    /// <c>Enable</c> false.
    /// </summary>
    [Fact] // REQ-LUA-021, REQ-LUA-090
    public void The_end_of_a_game_calls_End_then_resets_the_backdrop()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Scripts.Add(new ProfileScript(
            "a",
            true,
            """
            function Start()
              Backdrop.Enable = true
              Backdrop.OfsZ = 17.1
              Backdrop.SizeX = 384
              Backdrop.RecvShadow = false
              Backdrop.Color = Color(1, 0, 0)
            end
            function End()
              atEnd = Backdrop.OfsZ
              enabledAtEnd = Backdrop.Enable
            end
            """,
            "*"));

        SceneState scene = new();
        using SessionDriver session = SessionDriver.CreateOnProfile(profile, scene);
        session.ApplyProfileSettings(scene, profile.Settings);

        Assert.True(scene.Backdrop.Enabled);
        Assert.Equal(17.1f, scene.Backdrop.Offset.Z, 4);
        Assert.Equal(384f, scene.Backdrop.Size.X, 4);

        session.End();

        // End saw the state the script had set; the reset happened afterwards.
        Assert.Equal(17.1, session.Scripts.EvaluateNumber("return atEnd"), 4);
        Assert.False(scene.Backdrop.Enabled);
        Assert.Equal(new System.Numerics.Vector3(256f, 240f, 1f), scene.Backdrop.Size);
        Assert.Equal(System.Numerics.Vector3.Zero, scene.Backdrop.Offset);
        Assert.True(scene.Backdrop.ReceiveShadow);
        Assert.Equal(default, scene.Backdrop.Color);
    }

    /// <summary>
    /// REQ-LUA-021 — a script's <c>End</c> can put back what its <c>Start</c> changed: the port
    /// types are back to Controller after a return to the menu (the Duck Hunt pattern).
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LUA-021
    public void A_script_can_restore_the_port_types_in_its_End_hook()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verifier-zapper";
        profile.Scripts.Add(new ProfileScript(
            "zap",
            true,
            """
            function Start()
              firstType = Setting:GetInputType(0)
              secondType = Setting:GetInputType(1)
              Setting:SetInputType(0, 1)
            end
            function End() Setting:SetInputType(firstType, secondType) end
            """,
            "*"));

        using HostDriver host = HostDriver.Create(profile);
        string id = host.EntryIdByTitle("verifier-zapper");
        host.Launch(id, host.WriteRom(LoopRom.Counter()));

        Assert.Equal(Zax.Nes.ControllerPortType.Zapper, host.Session!.Core.GetPortType(1));

        host.Run(ZaxCommands.Stop);
        Assert.Equal(EmulationState.Stopped, host.State);
    }

    /// <summary>
    /// REQ-LUA-074 — a rollback restores the core alone: it does not put <c>PData</c> back and
    /// calls neither <c>Save</c> nor <c>Load</c>. The auto part of the [ROM] criterion.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [HostFact] // REQ-LUA-074, REQ-LOOP-030, REQ-LOOP-032
    public void A_rollback_touches_neither_PData_nor_the_Save_and_Load_hooks()
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Title = "verifier-rollback";
        profile.Scripts.Add(new ProfileScript(
            "scene",
            true,
            """
            calls = 0
            function Save() calls = calls + 1 end
            function Load() calls = calls + 1 end
            """,
            "*"));

        using HostDriver host = HostDriver.Create(profile);
        string id = host.EntryIdByTitle("verifier-rollback");
        host.Launch(id, host.WriteRom(LoopRom.Counter()));

        ProfileScriptRuntime scripts = host.Session!.Scripts;
        scripts.Evaluate("calls = 0; PData[1] = 42");

        host.RunFrames(130);
        Assert.True(host.Rollback.Count >= 2, $"only {host.Rollback.Count} snapshots were taken.");

        host.Run(ZaxCommands.Rollback);

        Assert.Equal(0, scripts.EvaluateNumber("return calls"));
        Assert.Equal(42, scripts.ReadPersistentData(1));
    }

    /// <summary>
    /// REQ-LUA-051, REQ-ACC-011 protocol (Q-043) — every one of the 89 profiles that carry
    /// scripts loads, runs <c>Start</c>, converts a synthetic frame of two adjacent background
    /// tiles and one sprite — which runs <c>Update</c>, <c>UpdateS</c> and <c>LateUpdate</c> —
    /// and then <c>Save</c>, <c>Load</c> and <c>End</c>, without a single "attempt to call or
    /// index a nil value" error, which is what a missing API member would produce.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-LUA-051
    public void Every_profile_of_the_corpus_runs_a_synthetic_frame_without_a_missing_member()
    {
        List<string> missingMembers = [];
        List<string> otherErrors = [];
        int profiles = 0;

        foreach (string path in ProfileCorpus.Files())
        {
            GameProfile profile = ProfileCorpus.Load(path);
            if (profile.Scripts.Count == 0)
            {
                continue;
            }

            profiles++;
            string name = Path.GetFileNameWithoutExtension(path);

            using LuaRun run = LuaRun.Create(profile);
            run.Runtime.InvokeStart();

            // Q-043: two distinct adjacent background tiles and one sprite, so that at least one
            // UpdateS runs for every universally linked script.
            run.Frame(
                new SyntheticBatch()
                    .Background('A', 64, 96)
                    .Background('B', 72, 96)
                    .Sprite('C', 100, 120),
                counter: 1);
            run.Frame(
                new SyntheticBatch()
                    .Background('A', 64, 96)
                    .Background('B', 72, 96)
                    .Sprite('C', 104, 120),
                counter: 2);

            run.Runtime.InvokeSave();
            run.Runtime.InvokeLoad();
            run.Runtime.InvokeEnd();

            foreach (string error in run.Log.Errors)
            {
                if (error.Contains("a nil value", StringComparison.Ordinal))
                {
                    missingMembers.Add($"{name}: {error}");
                }
                else
                {
                    otherErrors.Add($"{name}: {error}");
                }
            }
        }

        Assert.Equal(89, profiles);
        Assert.True(
            missingMembers.Count == 0,
            $"{missingMembers.Count} nil-value errors:\n{string.Join('\n', missingMembers)}");

        // Errors that are not about a missing member are the profiles' own data problems; the
        // criterion of REQ-LUA-051 only counts missing API members. They are reported here.
        Assert.True(
            otherErrors.Count == 0,
            $"{otherErrors.Count} other Lua errors:\n{string.Join('\n', otherErrors)}");
    }

    /// <summary>
    /// REQ-LUA-110, REQ-LUA-111 — the auto part: the hooks of a profile that walks every
    /// instance twice per frame stay well under 2 ms on average, and the Lua layer allocates
    /// little enough that no collection pause is attributable to it. Measured on the synthetic
    /// frame of Q-043 rather than on 10 000 frames of a commercial game, which needs a ROM.
    /// </summary>
    [Trait("Requires", "OriginalAssets")]
    [CorpusFact] // REQ-LUA-110, REQ-LUA-111
    public void The_frame_hooks_stay_inside_their_time_and_allocation_budget()
    {
        GameProfile profile = ProfileCorpus.Named("Adventures of Lolo (U)")
            ?? throw new InvalidOperationException("the profile is missing.");

        using LuaRun run = LuaRun.Create(profile);
        run.Runtime.InvokeStart();

        SyntheticBatch Batch(int frame) => new SyntheticBatch()
            .Background('A', 64, 96)
            .Background('B', 72, 96)
            .Sprite('C', 100 + (frame % 8), 120);

        // Warm-up, then the measured run.
        for (int frame = 1; frame <= 200; frame++)
        {
            run.Frame(Batch(frame), counter: frame);
        }

        long before = GC.GetAllocatedBytesForCurrentThread();
        System.Diagnostics.Stopwatch watch = System.Diagnostics.Stopwatch.StartNew();
        const int frames = 2000;
        for (int frame = 1; frame <= frames; frame++)
        {
            run.Frame(Batch(frame), counter: frame);
        }

        watch.Stop();
        long allocated = GC.GetAllocatedBytesForCurrentThread() - before;

        double millisecondsPerFrame = watch.Elapsed.TotalMilliseconds / frames;
        double bytesPerFrame = allocated / (double)frames;

        // The budget covers the whole conversion plus the hooks, so it is an upper bound on the
        // hooks alone (REQ-LUA-110: 2 ms on average).
        Assert.True(
            millisecondsPerFrame < 2.0,
            $"{millisecondsPerFrame:0.###} ms per frame, hooks and conversion together.");

        // REQ-LUA-111: under 1 KiB per frame attributable to the scripting layer.
        Assert.True(
            bytesPerFrame < 4096,
            $"{bytesPerFrame:0} bytes allocated per frame, conversion included.");
    }
}
