using System.Numerics;
using Zax.App.Loop;
using Zax.Profiles;
using Zax.Render;

namespace Zax.App.Tests;

/// <summary>
/// AMD-063 REQ-LUA-119 in the loop: <c>CS.Nes3D.CameraControl.RotOverride</c> is part of the
/// rollback snapshots — AMD-042 REQ-LOOP-069 gets a fourth element and REQ-LOOP-070 writes it
/// back — and it goes back to null when a game is launched.
/// </summary>
public sealed class Amd063RotOverrideRollbackTests
{
    private static readonly Vector3 Forced = new(-60f, 10f, 0f);

    /// <summary>
    /// A profile whose <c>Update</c> hook forces the rotation on the emulated frame
    /// <paramref name="at"/> and never touches it again.
    /// </summary>
    private static GameProfile ForcingProfile(int at)
    {
        GameProfile profile = GameProfile.CreateEmpty();
        profile.Scripts.Add(new ProfileScript(
            "rot",
            true,
            $"""
            function Update()
              if frame == {at} then
                CS.Nes3D.CameraControl.RotOverride = Vector3(-60, 10, 0)
              end
            end
            """,
            "*"));
        return profile;
    }

    /// <summary>A harness running <paramref name="profile"/> on the counter ROM.</summary>
    private static LoopHarness Running(GameProfile profile)
    {
        LoopHarness harness = LoopHarness.Create();
        string rom = harness.WriteRom(SyntheticRom.Counter(), "rot.nes");
        harness.Catalog.Add("rot", rom, profile);
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("rot"));
        Assert.Equal(EmulationState.Running, harness.Host.State);
        return harness;
    }

    /// <summary>
    /// AMD-042 REQ-LOOP-069 as AMD-063 amends it: a snapshot carries the forced rotation beside
    /// the core, the 64 <c>PData</c> values and the camera state. The camera state itself does
    /// not carry it — REQ-SCN-024 is unchanged.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, AMD-042 REQ-LOOP-069
    public void A_snapshot_carries_the_forced_rotation()
    {
        using LoopHarness harness = Running(ForcingProfile(at: 10));

        harness.RunFrames(180);
        Assert.Equal(Forced, harness.Scene.Camera.RotOverride);

        // Snapshots of the counters 0, 60 and 120; the rollback target is the one of 60, taken
        // well after the write.
        RollbackSnapshot snapshot = harness.Host.Rollback.TakeRollbackTarget()!;
        Assert.Equal(Forced, snapshot.RotOverride);
    }

    /// <summary>
    /// AMD-042 REQ-LOOP-070 as AMD-063 amends it — the criterion: the rotation is forced during
    /// the game and a rollback to a snapshot taken <b>before</b> that write puts it back to nil.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, AMD-042 REQ-LOOP-070
    public void A_rollback_before_the_write_puts_the_forced_rotation_back_to_nil()
    {
        using LoopHarness harness = Running(ForcingProfile(at: 50));

        // Snapshots of the counters 0 and 60; the write happens on frame 50, so the snapshot of
        // 0 is the only one that predates it, and that is the rollback target (REQ-LOOP-032).
        harness.RunFrames(100);
        Assert.Equal(Forced, harness.Scene.Camera.RotOverride);
        Assert.Equal(2, harness.Host.Rollback.Count);

        harness.Run(ZaxCommands.Rollback);

        Assert.Equal(0, harness.Host.Session!.Core.FrameCount);
        Assert.Null(harness.Scene.Camera.RotOverride);
    }

    /// <summary>
    /// AMD-042 REQ-LOOP-070: a rollback to a snapshot taken <b>after</b> the write restores the
    /// forced value, and the camera setpoint of that snapshot with it — the forced value having
    /// been written back first, the setpoints the camera ends on are the snapshot's.
    /// </summary>
    /// <remarks>
    /// The loop does not advance the camera — that is the application's frame, SPEC-06
    /// REQ-SCN-059 — so the setpoint here is the one the snapshot recorded and not the forced
    /// angle. That a camera update then writes the forced angle onto it is the camera's own
    /// business and is measured on <c>SceneCamera</c> (<c>Zax.Render.Tests</c>).
    /// </remarks>
    [Fact] // AMD-063, REQ-LUA-119, AMD-042 REQ-LOOP-070
    public void A_rollback_after_the_write_restores_the_forced_rotation_and_the_setpoint()
    {
        using LoopHarness harness = Running(ForcingProfile(at: 10));

        // The snapshot of counter 60 is taken with this distance in the setpoints.
        harness.Scene.Camera.Current.Distance = 123f;
        harness.RunFrames(180);

        harness.Scene.Camera.RotOverride = null;
        harness.Scene.Camera.Current.Distance = 321f;
        harness.Scene.Camera.Current.Angles = new Vector3(1f, 2f, 3f);

        harness.Run(ZaxCommands.Rollback);

        // The snapshot of counter 60 carried the forced value and its own setpoints.
        Assert.Equal(60, harness.Host.Session!.Core.FrameCount);
        Assert.Equal(Forced, harness.Scene.Camera.RotOverride);
        Assert.Equal(123f, harness.Scene.Camera.Current.Distance, 1e-3f);
        Assert.Equal(SceneConstants.DefaultAngles, harness.Scene.Camera.Current.Angles);
    }

    /// <summary>
    /// REQ-LUA-119: the value is a session value — a launch, and so a change of profile, puts it
    /// back to null, and nothing about it is written to disk.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-PERS-031, REQ-PERS-058
    public void A_launch_puts_the_forced_rotation_back_to_null()
    {
        using LoopHarness harness = Running(ForcingProfile(at: 10));

        harness.RunFrames(60);
        Assert.Equal(Forced, harness.Scene.Camera.RotOverride);

        // A second game, with a profile that forces nothing: the launch clears the value.
        string other = harness.WriteRom(SyntheticRom.Counter(), "plain.nes");
        harness.Catalog.Add("plain", other, GameProfile.CreateEmpty());
        harness.Run(ZaxCommands.LaunchGame, CommandArgs.Of("plain"));

        Assert.Null(harness.Scene.Camera.RotOverride);

        harness.Host.Shutdown();
        string[] files = Directory.GetFiles(harness.Root, "*", SearchOption.AllDirectories);
        foreach (string file in files)
        {
            Assert.DoesNotContain(
                "RotOverride",
                File.ReadAllText(file),
                StringComparison.OrdinalIgnoreCase);
        }
    }

    /// <summary>
    /// REQ-LUA-119: a savestate does not carry it (REQ-SCN-024 unchanged), so loading one back
    /// leaves the session value as it stands.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-SCN-024
    public void A_savestate_leaves_the_forced_rotation_alone()
    {
        using LoopHarness harness = Running(ForcingProfile(at: 10));

        harness.RunFrames(60);
        Assert.Equal(Forced, harness.Scene.Camera.RotOverride);
        harness.Run(ZaxCommands.QuickSave);

        harness.Scene.Camera.RotOverride = null;
        harness.RunFrames(60);
        harness.Run(ZaxCommands.QuickLoad);

        Assert.Null(harness.Scene.Camera.RotOverride);
    }
}
