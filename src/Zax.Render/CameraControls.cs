using System.Numerics;

namespace Zax.Render;

/// <summary>
/// The camera actions of SPEC-08 §4.1 the renderer-side controls consume.
/// </summary>
[Flags]
public enum CameraAction
{
    /// <summary>No action.</summary>
    None = 0,

    /// <summary>Rotate left: decreases <c>ry</c> (REQ-SCN-014).</summary>
    RotateLeft = 0x400,

    /// <summary>Rotate right: increases <c>ry</c>.</summary>
    RotateRight = 0x800,

    /// <summary>Rotate up: decreases <c>rx</c>.</summary>
    RotateUp = 0x1000,

    /// <summary>Rotate down: increases <c>rx</c>.</summary>
    RotateDown = 0x2000,

    /// <summary>Zoom in: decreases the distance (REQ-SCN-015).</summary>
    ZoomIn = 0x100,

    /// <summary>Zoom out: increases the distance.</summary>
    ZoomOut = 0x200,
}

/// <summary>
/// Applies the user controls of SPEC-06 §3.5 to the camera setpoints: the extended rotation and
/// zoom actions, the mouse wheel and the left-button drag.
/// </summary>
/// <remarks>
/// Everything here is driven by elapsed real time (REQ-SCN-014, REQ-SCN-015, REQ-SCN-059) and
/// only acts while the game is running (REQ-SCN-018); the caller decides that.
/// </remarks>
public static class CameraControls
{
    /// <summary>
    /// Applies the held rotation and zoom actions over <paramref name="realSeconds"/>
    /// (REQ-SCN-014, REQ-SCN-015). Opposite actions do not cancel out: RotLeft wins over
    /// RotRight, RotDown over RotUp and ZoomIn over ZoomOut.
    /// </summary>
    public static void Apply(SceneCamera camera, CameraAction held, float realSeconds)
    {
        ArgumentNullException.ThrowIfNull(camera);
        if (realSeconds <= 0f || held == CameraAction.None)
        {
            return;
        }

        CameraSetpoint setpoint = camera.Current;
        float rotation = SceneConstants.RotationDegreesPerSecond * realSeconds;
        Vector3 angles = setpoint.Angles;

        if (held.HasFlag(CameraAction.RotateLeft))
        {
            angles.Y -= rotation;
        }
        else if (held.HasFlag(CameraAction.RotateRight))
        {
            angles.Y += rotation;
        }

        if (held.HasFlag(CameraAction.RotateDown))
        {
            angles.X += rotation;
        }
        else if (held.HasFlag(CameraAction.RotateUp))
        {
            angles.X -= rotation;
        }

        setpoint.Angles = angles;

        float zoom = SceneConstants.ZoomUnitsPerSecond * realSeconds;
        if (held.HasFlag(CameraAction.ZoomIn))
        {
            setpoint.Distance -= zoom;
        }
        else if (held.HasFlag(CameraAction.ZoomOut))
        {
            setpoint.Distance += zoom;
        }
    }

    /// <summary>
    /// Applies <paramref name="notches"/> of mouse wheel, 5 units of distance each, positive
    /// forward (REQ-SCN-016, DEC-08-3). It only acts when no zoom action is held.
    /// </summary>
    public static void ApplyWheel(SceneCamera camera, float notches, CameraAction held)
    {
        ArgumentNullException.ThrowIfNull(camera);
        if (notches == 0f
            || held.HasFlag(CameraAction.ZoomIn)
            || held.HasFlag(CameraAction.ZoomOut))
        {
            return;
        }

        camera.Current.Distance -= notches * SceneConstants.WheelUnitsPerNotch;
    }

    /// <summary>
    /// Applies a left-button drag of <paramref name="pixelsRight"/> and
    /// <paramref name="pixelsUp"/> pixels, 0.1° per pixel (REQ-SCN-017).
    /// </summary>
    public static void ApplyDrag(SceneCamera camera, float pixelsRight, float pixelsUp)
    {
        ArgumentNullException.ThrowIfNull(camera);
        if (pixelsRight == 0f && pixelsUp == 0f)
        {
            return;
        }

        CameraSetpoint setpoint = camera.Current;
        Vector3 angles = setpoint.Angles;
        angles.Y += pixelsRight * SceneConstants.DragDegreesPerPixel;
        angles.X -= pixelsUp * SceneConstants.DragDegreesPerPixel;
        setpoint.Angles = angles;
    }
}
