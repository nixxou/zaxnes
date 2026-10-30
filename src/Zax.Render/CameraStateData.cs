using System.Numerics;

namespace Zax.Render;

/// <summary>The setpoints of one camera mode, as a savestate carries them (REQ-SCN-024).</summary>
/// <param name="Distance">Setpoint distance.</param>
/// <param name="Angles">Setpoint angles <c>(rx, ry, rz)</c> in degrees.</param>
/// <param name="Target">Setpoint target <c>T</c> in scene coordinates.</param>
public readonly record struct CameraModeState(float Distance, Vector3 Angles, Vector3 Target);

/// <summary>
/// The camera state a savestate holds (SPEC-06 REQ-SCN-024): the current mode, the setpoints of
/// <b>both</b> modes and what is left of the mode transition window.
/// </summary>
/// <remarks>
/// <para>
/// This is data only: writing it to a savestate file is SPEC-07's business (lot L6). On restore
/// the setpoints go back as they are and the displayed camera joins them through the smoothing
/// of REQ-SCN-019 — it is not snapped.
/// </para>
/// <para>
/// REQ-SCN-024 as amended by AMD-042 records a <b>remaining time</b> in [0, 3] s rather than a
/// deadline, which an absolute one would make stale the moment the state is read back. It is
/// also the single reference for what a rollback snapshot carries of the camera
/// (REQ-LOOP-069).
/// </para>
/// </remarks>
/// <param name="Mode">The current mode.</param>
/// <param name="Tpv">Setpoints of the orbital mode.</param>
/// <param name="Fpv">Setpoints of the subjective mode.</param>
/// <param name="TransitionRemaining">Seconds left in the 3-second transition window.</param>
public readonly record struct CameraStateData(
    CameraMode Mode,
    CameraModeState Tpv,
    CameraModeState Fpv,
    float TransitionRemaining);
