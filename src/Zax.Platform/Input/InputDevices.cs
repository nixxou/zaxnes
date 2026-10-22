using System.Numerics;

namespace Zax.Platform.Input;

/// <summary>
/// The keyboard as the binding layer sees it: physical key positions, nothing else
/// (DEC-08-5, REQ-INP-012).
/// </summary>
public interface IKeyboardDevice
{
    /// <summary>Whether the key at physical position <paramref name="keyCode"/> is held.</summary>
    bool IsDown(int keyCode);

    /// <summary>
    /// Appends the physical positions of every key currently held to
    /// <paramref name="keys"/>, for the capture of REQ-INP-042.
    /// </summary>
    void CollectHeldKeys(ICollection<int> keys);
}

/// <summary>
/// The mouse as the input layer sees it (REQ-INP-120): a position in the render target, two
/// buttons and a wheel. Mouse buttons are deliberately <b>not</b> bindable (REQ-INP-012).
/// </summary>
public interface IPointerDevice
{
    /// <summary>Pointer position in the render target, in pixels, origin top left.</summary>
    Vector2 Position { get; }

    /// <summary>Whether the left button is held (camera orbit, REQ-INP-091).</summary>
    bool LeftButton { get; }

    /// <summary>Whether the right button is held (Zapper trigger, REQ-INP-062).</summary>
    bool RightButton { get; }

    /// <summary>
    /// The wheel notches accumulated since the previous call, positive forward, and resets the
    /// accumulator (REQ-INP-090).
    /// </summary>
    float TakeWheelNotches();

    /// <summary>Whether the system cursor is shown (REQ-INP-110, REQ-INP-111).</summary>
    bool CursorVisible { get; set; }
}

/// <summary>
/// One gamepad, reduced to the device-neutral controls of <see cref="PhysicalControl"/> with the
/// dead zones and the threshold of SPEC-08 §3.4 already applied.
/// </summary>
public interface IGamepadDevice
{
    /// <summary>
    /// Stable identity used as the persistence key of this pad's binding set
    /// (SPEC-08 §2, REQ-INP-025).
    /// </summary>
    string Identity { get; }

    /// <summary>Name shown to the user (REQ-INP-026).</summary>
    string DisplayName { get; }

    /// <summary>
    /// Whether the pad answers to the standard Xbox-style numbering of
    /// <see cref="StandardGamepad"/>, and therefore gets the defaults of SPEC-08 §4.3
    /// (REQ-INP-020). A pad without a mapping reports raw indices (REQ-INP-021).
    /// </summary>
    bool IsStandard { get; }

    /// <summary>
    /// The normalised 0-to-1 value of <paramref name="control"/>: 0 or 1 for a button, the
    /// calibrated travel for a half-axis or a trigger. A control this pad does not have reads 0.
    /// </summary>
    float Value(PhysicalControl control);

    /// <summary>
    /// Appends every control of this pad that is currently pressed — buttons down, half-axes and
    /// triggers at or above 0.34 — to <paramref name="controls"/>. This is what the capture of
    /// REQ-INP-042 listens to and what makes a pad "active" for the automatic assignment of
    /// REQ-INP-023.
    /// </summary>
    void CollectActiveControls(ICollection<PhysicalControl> controls);
}

/// <summary>
/// The set of gamepads the host can see, refreshed on demand. Hot-plug is observable through
/// <see cref="Devices"/> changing between two <see cref="Poll"/> calls (REQ-INP-024).
/// </summary>
public interface IGamepadHub
{
    /// <summary>
    /// The connected, non-VR pads, in a stable order (REQ-INP-022 keeps VR devices out).
    /// </summary>
    IReadOnlyList<IGamepadDevice> Devices { get; }

    /// <summary>Refreshes the device list and every pad's state. Called once per frame.</summary>
    void Poll();
}

/// <summary>Helpers about gamepad identities (REQ-INP-022, REQ-INP-026).</summary>
public static class GamepadIdentity
{
    /// <summary>
    /// The identity fragments REQ-INP-022 excludes: a VR controller is never offered and never
    /// assigned automatically.
    /// </summary>
    public static readonly string[] VirtualRealityMarkers = ["Oculus", "Vive", "Knuckles", "Spatial"];

    /// <summary>Whether <paramref name="identity"/> names a VR device (REQ-INP-022).</summary>
    public static bool IsVirtualReality(string? identity)
    {
        if (string.IsNullOrEmpty(identity))
        {
            return false;
        }

        foreach (string marker in VirtualRealityMarkers)
        {
            if (identity.Contains(marker, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }
}
