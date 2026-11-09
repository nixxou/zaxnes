using System.Numerics;
using Zax.Render;

namespace Zax.Render.Tests;

/// <summary>
/// AMD-063 REQ-LUA-119 on the camera side: while <c>RotOverride</c> is set, every camera update
/// writes it into the rotation setpoint of the current mode and shows it at once, without the
/// smoothing of REQ-SCN-019.
/// </summary>
public sealed class Amd063RotOverrideTests
{
    private static readonly Vector3 Forced = new(-60f, 10f, 0f);

    /// <summary>It is null by default: the camera behaves as it always has.</summary>
    [Fact] // AMD-063, REQ-LUA-119
    public void The_forced_rotation_is_null_by_default()
    {
        SceneCamera camera = new();
        Assert.Null(camera.RotOverride);
    }

    /// <summary>
    /// REQ-LUA-119 — the criterion, first half: one camera update after the write, the setpoint
    /// of the current mode and the displayed rotation are both the forced value, at once.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-SCN-007, REQ-SCN-019
    public void One_update_puts_the_forced_rotation_on_the_setpoint_and_on_the_picture()
    {
        SceneCamera camera = new();
        Assert.NotEqual(Forced, camera.Current.Angles);

        camera.RotOverride = Forced;
        camera.Advance(1f / 60f);

        // What Camera.Rot reads (SPEC-05 CameraAPI) and what the picture is built from.
        Assert.Equal(Forced, camera.Current.Angles);
        Assert.Equal(Forced, camera.Angles);

        // The smoothing of REQ-SCN-019 is not involved: a second update changes nothing, and a
        // single update was enough — a smoothed camera would still be on its way.
        camera.Advance(1f / 60f);
        Assert.Equal(Forced, camera.Angles);
    }

    /// <summary>
    /// REQ-LUA-119 — a camera update with no elapsed time still applies it: the requirement says
    /// "every camera update", not "every advanced second".
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-SCN-007
    public void An_update_of_zero_seconds_applies_it_too()
    {
        SceneCamera camera = new();
        camera.RotOverride = Forced;
        camera.Advance(0f);

        Assert.Equal(Forced, camera.Current.Angles);
        Assert.Equal(Forced, camera.Angles);
    }

    /// <summary>
    /// REQ-LUA-119 — it overrides whatever else writes the setpoint: a script, the mouse drag or
    /// a hotkey may move <c>Camera.Rot</c>, and the next update puts the forced value back.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119
    public void It_wins_over_any_other_write_of_the_setpoint()
    {
        SceneCamera camera = new();
        camera.RotOverride = Forced;
        camera.Advance(1f / 60f);

        camera.Current.Angles = new Vector3(12f, 34f, 56f);
        camera.Advance(1f / 60f);

        Assert.Equal(Forced, camera.Current.Angles);
        Assert.Equal(Forced, camera.Angles);
    }

    /// <summary>
    /// REQ-LUA-119 — the criterion, second half: put back to null, the camera carries on from
    /// the current setpoint, which is the last forced value, and stays there until a new one.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119
    public void Back_to_null_the_camera_stays_where_it_was_forced()
    {
        SceneCamera camera = new();
        camera.RotOverride = Forced;
        camera.Advance(1f / 60f);

        camera.RotOverride = null;
        for (int frame = 0; frame < 120; frame++)
        {
            camera.Advance(1f / 60f);
        }

        Assert.Null(camera.RotOverride);
        Assert.Equal(Forced, camera.Current.Angles);
        Assert.Equal(Forced, camera.Angles);

        // A new setpoint is followed again, through the smoothing: after two seconds of a 0.4 s
        // characteristic time the camera has all but arrived — a tenth of a degree out of the
        // eighty it had to cross.
        camera.Current.Angles = new Vector3(20f, 30f, 0f);
        for (int frame = 0; frame < 120; frame++)
        {
            camera.Advance(1f / 60f);
        }

        Assert.Equal(20f, camera.Angles.X, 0.1f);
        Assert.Equal(30f, camera.Angles.Y, 0.1f);
    }

    /// <summary>
    /// REQ-LUA-119 — the forced value is written before the wrap of AMD-045 REQ-SCN-071, so an
    /// angle outside <c>]−180°, 180°]</c> is brought back by whole turns like any other
    /// setpoint, and the orientation shown is the one asked for.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, AMD-045 REQ-SCN-071
    public void A_forced_angle_outside_the_turn_is_wrapped_like_any_setpoint()
    {
        SceneCamera camera = new();
        camera.RotOverride = new Vector3(400f, 0f, 0f);
        camera.Advance(1f / 60f);

        Assert.Equal(40f, camera.Current.Angles.X, 1e-3f);
        Assert.Equal(40f, camera.Angles.X, 1e-3f);
    }

    /// <summary>
    /// REQ-LUA-119 — it follows the <b>current</b> mode: only that mode's setpoint is written,
    /// the other one is left alone, and a mode change moves the forced rotation along.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-SCN-006
    public void It_writes_the_setpoint_of_the_current_mode_only()
    {
        SceneCamera camera = new();
        Vector3 before = camera.Setpoints(CameraMode.Fpv).Angles;

        camera.RotOverride = Forced;
        camera.Advance(1f / 60f);

        Assert.Equal(Forced, camera.Setpoints(CameraMode.Tpv).Angles);
        Assert.Equal(before, camera.Setpoints(CameraMode.Fpv).Angles);

        camera.SetMode(CameraMode.Fpv);
        camera.Advance(1f / 60f);
        Assert.Equal(Forced, camera.Setpoints(CameraMode.Fpv).Angles);
    }

    /// <summary>
    /// REQ-LUA-119 — it is not part of the camera state a savestate carries (REQ-SCN-024
    /// unchanged): neither <c>Capture</c> nor <c>Restore</c> touches it.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-SCN-024
    public void The_savestate_camera_state_does_not_carry_it()
    {
        SceneCamera camera = new();
        camera.RotOverride = Forced;
        camera.Advance(1f / 60f);
        CameraStateData saved = camera.Capture();

        // A savestate written while a rotation was forced restores the angles it was holding —
        // they are in the setpoints — but says nothing about the forcing itself.
        SceneCamera other = new();
        Assert.Null(other.RotOverride);
        other.Restore(saved);
        Assert.Null(other.RotOverride);
        Assert.Equal(Forced, other.Current.Angles);

        // And a restore does not clear a forcing the session already had either.
        other.RotOverride = new Vector3(1f, 2f, 3f);
        other.Restore(saved);
        Assert.Equal(new Vector3(1f, 2f, 3f), other.RotOverride);
    }

    /// <summary>
    /// REQ-LUA-119 — the picture really turns: the view built after the update is the one the
    /// forced angles give, basis for basis.
    /// </summary>
    [Fact] // AMD-063, REQ-LUA-119, REQ-SCN-007
    public void The_rendered_view_is_the_one_of_the_forced_rotation()
    {
        SceneCamera forced = new();
        forced.RotOverride = Forced;
        forced.Advance(1f / 60f);

        // The same camera brought to the same angles without any forcing, shown at its
        // setpoints: the two views have to agree.
        SceneCamera plain = new();
        plain.Current.Angles = Forced;
        plain.SnapToSetpoints();

        CameraView a = forced.BuildView(default, 4f / 3f);
        CameraView b = plain.BuildView(default, 4f / 3f);

        Assert.Equal(b.Eye, a.Eye);
        Assert.Equal(b.Right, a.Right);
        Assert.Equal(b.Up, a.Up);
        Assert.Equal(b.Forward, a.Forward);
        Assert.Equal(b.View, a.View);
    }
}
