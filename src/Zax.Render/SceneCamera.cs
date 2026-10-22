using System.Numerics;
using Zax.Profiles;

namespace Zax.Render;

/// <summary>Camera mode, with the numeric values Lua sees (SPEC-06 REQ-SCN-006).</summary>
public enum CameraMode
{
    /// <summary>Orbital view around the NES screen, the default mode.</summary>
    Tpv = 0,

    /// <summary>View that follows a character.</summary>
    Fpv = 1,
}

/// <summary>
/// The setpoints of one camera mode: distance, angles in degrees and target point
/// (SPEC-06 REQ-SCN-006).
/// </summary>
public sealed class CameraSetpoint
{
    private float _distance;

    /// <summary>Setpoint distance, bounded to [0, 512] (REQ-SCN-011).</summary>
    public float Distance
    {
        get => _distance;
        set => _distance = Math.Clamp(value, 0f, SceneConstants.MaxDistance);
    }

    /// <summary>Setpoint angles <c>(rx, ry, rz)</c> in degrees; never bounded (REQ-SCN-011).</summary>
    public Vector3 Angles { get; set; }

    /// <summary>Setpoint target <c>T</c> in scene coordinates.</summary>
    public Vector3 Target { get; set; } = SceneConstants.Center;
}

/// <summary>
/// Everything a frame needs from the camera: the eye, the scene-frame basis, the aimed point,
/// the view-projection matrix and the HUD plane (SPEC-06 §3.2, §3.3, §3.12).
/// </summary>
/// <param name="Eye">The eye <c>E</c> in scene coordinates.</param>
/// <param name="Right">Camera right axis, scene frame.</param>
/// <param name="Up">Camera up axis, scene frame.</param>
/// <param name="Forward">Camera forward axis, scene frame.</param>
/// <param name="AimPoint">The aimed point <c>P*</c> (REQ-SCN-008), or the target in FPV.</param>
/// <param name="Target">The effective target <c>T′</c> (REQ-SCN-007 step 2).</param>
/// <param name="Distance">The displayed distance <c>d</c>.</param>
/// <param name="Fov">Vertical field of view in degrees.</param>
/// <param name="Aspect">Aspect ratio of the render target.</param>
/// <param name="View">View matrix, OpenGL frame, row-vector convention.</param>
/// <param name="Projection">Projection matrix, row-vector convention.</param>
/// <param name="HudDistance">Distance of the HUD plane, <c>d_HUD</c> (REQ-SCN-045).</param>
public readonly record struct CameraView(
    Vector3 Eye,
    Vector3 Right,
    Vector3 Up,
    Vector3 Forward,
    Vector3 AimPoint,
    Vector3 Target,
    float Distance,
    float Fov,
    float Aspect,
    Matrix4x4 View,
    Matrix4x4 Projection,
    float HudDistance)
{
    /// <summary>The combined view-projection matrix, OpenGL frame.</summary>
    public Matrix4x4 ViewProjection => Matrix4x4.Multiply(View, Projection);

    /// <summary>
    /// The scene point a HUD element placed at <paramref name="p"/> in the NES plane occupies:
    /// <c>E + d_HUD·f + 0.5 × ((p − C) along right, up, forward)</c> (REQ-SCN-045).
    /// </summary>
    /// <param name="p">Position of the element in the reference frame.</param>
    public Vector3 HudPoint(Vector3 p)
    {
        Vector3 offset = (p - SceneConstants.Center) * SceneConstants.HudScale;
        return Eye
            + (Forward * HudDistance)
            + (Right * offset.X)
            + (Up * offset.Y)
            + (Forward * offset.Z);
    }

    /// <summary>
    /// The light propagation direction of the HUD (REQ-SCN-029): <c>Light.Direction</c> read in
    /// the camera frame, so that orbiting does not change the HUD's shading.
    /// </summary>
    /// <param name="direction">The light direction setting.</param>
    public Vector3 HudLightDirection(Vector3 direction)
        => (Right * direction.X) + (Up * direction.Y) + (Forward * direction.Z);
}

/// <summary>
/// The camera of SPEC-06 §3.2 to §3.7: two modes with their own setpoints, critical smoothing,
/// the corrected aim point of REQ-SCN-008 and the HUD plane of REQ-SCN-045.
/// </summary>
/// <remarks>
/// <para>
/// The camera is driven by real time, never by the emulated frame rate (REQ-SCN-059), and keeps
/// being smoothed while the game is paused — only the scene stops being drawn (REQ-SCN-047).
/// </para>
/// <para>
/// FPV target tracking and the occlusion transparency of REQ-SCN-022 belong to the Lua layer
/// (lot L5) and the host; this class only holds the state and the geometry.
/// </para>
/// </remarks>
public sealed class SceneCamera
{
    private readonly CameraSetpoint[] _setpoints = [new(), new()];

    private float _fov = SceneConstants.DefaultFov;
    private float _distance;
    private Vector3 _angles;
    private Vector3 _target = SceneConstants.Center;

    private CriticalSmoothing _distanceFilter;
    private CriticalSmoothing _rxFilter;
    private CriticalSmoothing _ryFilter;
    private CriticalSmoothing _rzFilter;
    private CriticalSmoothing _targetXFilter;
    private CriticalSmoothing _targetYFilter;
    private CriticalSmoothing _targetZFilter;

    /// <summary>Creates the camera in its launch state (REQ-SCN-012, REQ-SCN-013).</summary>
    public SceneCamera()
    {
        // At application launch the FPV distance is 0 (REQ-SCN-012).
        _setpoints[(int)CameraMode.Fpv].Distance = 0f;
        ResetView();
        SnapToSetpoints();
    }

    /// <summary>Current mode (REQ-SCN-006).</summary>
    public CameraMode Mode { get; private set; } = CameraMode.Tpv;

    /// <summary>The setpoints of the current mode.</summary>
    public CameraSetpoint Current => _setpoints[(int)Mode];

    /// <summary>The setpoints of <paramref name="mode"/>.</summary>
    public CameraSetpoint Setpoints(CameraMode mode) => _setpoints[(int)mode];

    /// <summary>
    /// Field of view in degrees, a user setting of 45 to 95 by steps of 5 (REQ-SCN-005). Any
    /// change applies at once and triggers a view reset (REQ-SCN-012).
    /// </summary>
    public float Fov
    {
        get => _fov;
        set
        {
            float clamped = Math.Clamp(value, SceneConstants.MinFov, SceneConstants.MaxFov);
            if (clamped == _fov)
            {
                return;
            }

            _fov = clamped;
            ResetView();
        }
    }

    /// <summary>Characteristic smoothing time <c>Tₛ</c>, 0.4 s by default (REQ-SCN-019).</summary>
    public float SmoothTime { get; set; } = CriticalSmoothing.DefaultSmoothTime;

    /// <summary>
    /// The local camera profile setting (REQ-SCN-009): the rotation becomes
    /// <c>R_Z(ry) · R_X(rx)</c> and the camera no longer aims at <c>P*</c>.
    /// </summary>
    public bool LocalCamera { get; set; }

    /// <summary>Seconds left in the mode transition window (REQ-SCN-006, REQ-SCN-020).</summary>
    public float TransitionRemaining { get; private set; }

    /// <summary>Displayed distance, after smoothing.</summary>
    public float Distance => _distance;

    /// <summary>Displayed angles in degrees, after smoothing.</summary>
    public Vector3 Angles => _angles;

    /// <summary>Displayed target, after smoothing.</summary>
    public Vector3 Target => _target;

    /// <summary>
    /// The view reset of REQ-SCN-012: the current FOV is applied, both modes get the angles
    /// <c>(0.81°, 0.81°, 0)</c> and the target <c>C</c>, and the <b>current</b> mode gets
    /// <c>d₀ = 104 / atan(FOV_rad / 2)</c>. The other mode keeps its distance.
    /// </summary>
    public void ResetView()
    {
        foreach (CameraSetpoint setpoint in _setpoints)
        {
            setpoint.Angles = SceneConstants.DefaultAngles;
            setpoint.Target = SceneConstants.Center;
        }

        Current.Distance = SceneConstants.DefaultDistance(_fov);
    }

    /// <summary>
    /// Ends a game (REQ-SCN-013): back to TPV, smoothing time back to 0.4 s, then a view reset.
    /// The speed setting is the host's business (SPEC-07).
    /// </summary>
    public void EndGame()
    {
        Mode = CameraMode.Tpv;
        TransitionRemaining = 0f;
        SmoothTime = CriticalSmoothing.DefaultSmoothTime;
        ResetView();
    }

    /// <summary>
    /// Switches mode, which opens the 3-second transition window (REQ-SCN-006). Setting the
    /// same mode has no effect.
    /// </summary>
    public void SetMode(CameraMode mode)
    {
        if (mode == Mode)
        {
            return;
        }

        Mode = mode;
        TransitionRemaining = SceneConstants.ModeTransitionSeconds;
    }

    /// <summary>Switches to the other mode (REQ-SCN-021, <c>Camera:ToggleMode()</c>).</summary>
    public void ToggleMode() => SetMode(Mode == CameraMode.Tpv ? CameraMode.Fpv : CameraMode.Tpv);

    /// <summary>
    /// The camera state a savestate carries (REQ-SCN-024): the current mode, the setpoints of
    /// both modes and what is left of the transition window.
    /// </summary>
    public CameraStateData Capture() => new(
        Mode,
        Snapshot(CameraMode.Tpv),
        Snapshot(CameraMode.Fpv),
        TransitionRemaining);

    /// <summary>
    /// Puts back a state <see cref="Capture"/> produced (REQ-SCN-024). The displayed values are
    /// <b>not</b> snapped: they join the restored setpoints through the smoothing of
    /// REQ-SCN-019.
    /// </summary>
    public void Restore(in CameraStateData state)
    {
        Apply(CameraMode.Tpv, state.Tpv);
        Apply(CameraMode.Fpv, state.Fpv);
        Mode = state.Mode;
        TransitionRemaining = MathF.Max(0f, state.TransitionRemaining);
    }

    /// <summary>Puts the displayed values on the setpoints at once, without smoothing.</summary>
    public void SnapToSetpoints()
    {
        _distance = Current.Distance;
        _angles = Current.Angles;
        _target = Current.Target;
        _distanceFilter.Reset();
        _rxFilter.Reset();
        _ryFilter.Reset();
        _rzFilter.Reset();
        _targetXFilter.Reset();
        _targetYFilter.Reset();
        _targetZFilter.Reset();
    }

    /// <summary>
    /// Advances the smoothing by <paramref name="realSeconds"/> of real time (REQ-SCN-019,
    /// REQ-SCN-059). In TPV the target is always smoothed; in FPV it is smoothed only during
    /// the transition window, and followed instantly afterwards (REQ-SCN-020).
    /// </summary>
    public void Advance(float realSeconds)
    {
        if (realSeconds <= 0f)
        {
            return;
        }

        if (TransitionRemaining > 0f)
        {
            TransitionRemaining = MathF.Max(0f, TransitionRemaining - realSeconds);
        }

        CameraSetpoint setpoint = Current;
        _distance = _distanceFilter.Step(_distance, setpoint.Distance, SmoothTime, realSeconds);
        _angles = new Vector3(
            _rxFilter.Step(_angles.X, setpoint.Angles.X, SmoothTime, realSeconds),
            _ryFilter.Step(_angles.Y, setpoint.Angles.Y, SmoothTime, realSeconds),
            _rzFilter.Step(_angles.Z, setpoint.Angles.Z, SmoothTime, realSeconds));

        bool smoothTarget = Mode == CameraMode.Tpv || TransitionRemaining > 0f;
        if (smoothTarget)
        {
            _target = new Vector3(
                _targetXFilter.Step(_target.X, setpoint.Target.X, SmoothTime, realSeconds),
                _targetYFilter.Step(_target.Y, setpoint.Target.Y, SmoothTime, realSeconds),
                _targetZFilter.Step(_target.Z, setpoint.Target.Z, SmoothTime, realSeconds));
        }
        else
        {
            _target = setpoint.Target;
            _targetXFilter.Reset();
            _targetYFilter.Reset();
            _targetZFilter.Reset();
        }
    }

    /// <summary>
    /// Builds the view of the frame from the smoothed values, the clip margins and the aspect
    /// ratio of the render target (REQ-SCN-004, REQ-SCN-007 to REQ-SCN-010).
    /// </summary>
    /// <param name="clipping">The four clip margins (DEC-R2-T1).</param>
    /// <param name="aspect">Width divided by height of the render target.</param>
    public CameraView BuildView(Vec4F clipping, float aspect)
    {
        VisibleRect rect = VisibleRect.From(clipping);
        (Vector3 qRight, Vector3 qUp, Vector3 qForward) = SceneConstants.Rotation(_angles, LocalCamera);

        // REQ-SCN-007 step 2: TPV recentres the target on the visible rectangle, FPV does not
        // (REQ-SCN-010).
        Vector3 target = Mode == CameraMode.Tpv
            ? SceneConstants.EffectiveTarget(_target, clipping)
            : _target;
        Vector3 eye = target - (qForward * _distance);

        Vector3 right;
        Vector3 up;
        Vector3 forward;
        Vector3 aim;

        if (Mode == CameraMode.Tpv && !LocalCamera)
        {
            // REQ-SCN-008: the camera is oriented towards P*, with world Y as the vertical.
            aim = SceneConstants.AimPoint(eye, rect, target);
            forward = aim - eye;
            forward = forward.LengthSquared() > 0f ? Vector3.Normalize(forward) : qForward;
            (right, up) = BasisFromForward(forward);
        }
        else
        {
            // REQ-SCN-009 and REQ-SCN-010: the camera looks exactly along q·(0,0,1), with the
            // vertical q·(0,1,0). In FPV without the local camera that includes the roll rz.
            forward = qForward;
            right = qRight;
            up = qUp;
            aim = eye + forward;
        }

        Matrix4x4 view = LookAt(eye, forward, up);
        Matrix4x4 projection = Matrix4x4.CreatePerspectiveFieldOfView(
            _fov * MathF.PI / 180f,
            MathF.Max(aspect, 1e-3f),
            SceneConstants.NearPlane,
            SceneConstants.FarPlane);

        return new CameraView(
            eye,
            right,
            up,
            forward,
            aim,
            target,
            _distance,
            _fov,
            aspect,
            view,
            projection,
            SceneConstants.HudDistance(_fov));
    }

    private CameraModeState Snapshot(CameraMode mode)
    {
        CameraSetpoint setpoint = _setpoints[(int)mode];
        return new CameraModeState(setpoint.Distance, setpoint.Angles, setpoint.Target);
    }

    private void Apply(CameraMode mode, in CameraModeState state)
    {
        CameraSetpoint setpoint = _setpoints[(int)mode];
        setpoint.Distance = state.Distance;
        setpoint.Angles = state.Angles;
        setpoint.Target = state.Target;
    }

    /// <summary>
    /// The camera basis that completes <paramref name="forward"/> with world Y as the vertical,
    /// in the left-handed scene frame (REQ-SCN-007 step 4: no roll).
    /// </summary>
    internal static (Vector3 Right, Vector3 Up) BasisFromForward(Vector3 forward)
    {
        Vector3 worldUp = Vector3.UnitY;
        Vector3 right = Vector3.Cross(worldUp, forward);
        if (right.LengthSquared() < 1e-8f)
        {
            // Looking straight up or down: any horizontal axis will do.
            right = Vector3.Cross(Vector3.UnitZ, forward);
            if (right.LengthSquared() < 1e-8f)
            {
                right = Vector3.UnitX;
            }
        }

        right = Vector3.Normalize(right);
        return (right, Vector3.Cross(forward, right));
    }

    /// <summary>
    /// The OpenGL view matrix of a scene-frame camera. The scene frame is left-handed, so the
    /// conversion <c>(x, y, z) → (x, y, −z)</c> of REQ-SCN-001 is applied to the eye and to the
    /// basis before the right-handed look-at is built.
    /// </summary>
    internal static Matrix4x4 LookAt(Vector3 eye, Vector3 forward, Vector3 up)
    {
        Vector3 eyeGl = ToGl(eye);
        Vector3 forwardGl = ToGl(forward);
        Vector3 upGl = ToGl(up);
        if (forwardGl.LengthSquared() < 1e-8f)
        {
            forwardGl = -Vector3.UnitZ;
        }

        if (upGl.LengthSquared() < 1e-8f)
        {
            upGl = Vector3.UnitY;
        }

        return Matrix4x4.CreateLookAt(eyeGl, eyeGl + forwardGl, upGl);
    }

    /// <summary>Converts a scene-frame point or vector to the OpenGL frame (REQ-SCN-001).</summary>
    public static Vector3 ToGl(Vector3 scene) => new(scene.X, scene.Y, -scene.Z);
}
