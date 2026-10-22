using System.Numerics;
using Zax.Nes;
using Zax.Render;

namespace Zax.App.Input;

/// <summary>
/// The mouse Zapper of SPEC-08 §3.7: the trigger on the right mouse button and the aimed NES
/// pixel, refreshed once per displayed frame while the Zapper is active.
/// </summary>
/// <remarks>
/// <para>
/// The Zapper is active if and only if a game is running and at least one port is of type
/// <c>Zapper</c> (REQ-INP-061); while it is not, moving the mouse changes nothing of what the
/// core sees. The light detection itself is the core's business (SPEC-02 REQ-CORE-062); this
/// class supplies the trigger bit and the position (REQ-INP-065).
/// </para>
/// <para>
/// The trigger is the <b>right</b> button (REQ-INP-062); the left one orbits the camera
/// (REQ-INP-091), so there is no conflict (DEC-06-Q6). The polarity is the one of DEC-02-Q1 and
/// of §4.1: bit 4 is 1 when the trigger is released and 0 when it is held.
/// </para>
/// </remarks>
public sealed class ZapperState
{
    /// <summary>The port byte when the trigger is released (§4.1, DEC-02-Q1).</summary>
    public const byte ReleasedByte = 0x10;

    /// <summary>The port byte while the trigger is held (§4.1, DEC-02-Q1).</summary>
    public const byte PressedByte = 0x00;

    /// <summary>Whether the trigger is held, that is whether the right button is down.</summary>
    public bool TriggerHeld { get; private set; }

    /// <summary>
    /// The last position the aim produced. It survives a frame in which the ray missed the
    /// plane, which is the "last valid position" of REQ-INP-064.
    /// </summary>
    public ZapperPosition Position { get; private set; } = ZapperPosition.OffScreen;

    /// <summary>Whether a valid position was ever computed.</summary>
    public bool HasPosition { get; private set; }

    /// <summary>
    /// The byte of a Zapper port inside the input word (§4.1): no controller button at all, and
    /// bit 4 carrying the trigger with the polarity of DEC-02-Q1.
    /// </summary>
    public byte PortByte => TriggerHeld ? PressedByte : ReleasedByte;

    /// <summary>
    /// Updates the Zapper for one displayed frame (REQ-INP-061).
    /// </summary>
    /// <param name="active">
    /// Whether the Zapper is active: a game is running and a port is of type Zapper. When it is
    /// not, neither the trigger nor the position changes.
    /// </param>
    /// <param name="view">The camera view of the frame.</param>
    /// <param name="pointer">Pointer position in the render target.</param>
    /// <param name="size">Size of the render target, in pixels.</param>
    /// <param name="rightButton">Whether the right mouse button is held (REQ-INP-062).</param>
    /// <param name="offset">The scripts' <c>ZapperOffset</c>, in scene units.</param>
    /// <param name="sceneScale">The scene scale <c>e</c>; 1 on desktop (DEC-08-4).</param>
    public void Update(
        bool active,
        in CameraView view,
        Vector2 pointer,
        Vector2 size,
        bool rightButton,
        Vector3 offset,
        float sceneScale = 1f)
    {
        if (!active)
        {
            return;
        }

        TriggerHeld = rightButton;

        if (ZapperAim.TryAim(view, pointer, size, offset, sceneScale, out ZapperPosition aimed))
        {
            Position = aimed;
            HasPosition = true;
        }
    }

    /// <summary>
    /// Puts the Zapper back to its start-of-game state: trigger released, no position yet. A
    /// game change does this, along with the port types going back to Controller
    /// (REQ-INP-060).
    /// </summary>
    public void Reset()
    {
        TriggerHeld = false;
        Position = ZapperPosition.OffScreen;
        HasPosition = false;
    }
}
