using System.Numerics;

namespace Zax.App.Input;

/// <summary>
/// The platform-independent input snapshot of REQ-INP-120, which the host hands to the input
/// logic once per frame.
/// </summary>
/// <remarks>
/// <para>
/// This is the whole contract between the devices and the logic: the two players' action masks
/// — <b>without</b> the script pad, which the logic adds itself (REQ-INP-071) — the pointer in
/// the render target with its two buttons, the wheel travel, and whether the pointer is over a
/// piece of interface. Nothing here mentions a window, a key or a gamepad, which is what lets a
/// libretro frontend produce snapshots of its own (REQ-INP-121) and a unit test inject them
/// without opening a window.
/// </para>
/// <para>
/// <see cref="Wheel"/> is counted in notches, positive forward, and is consumed by the frame it
/// is reported in (REQ-INP-090, DEC-08-3).
/// </para>
/// </remarks>
/// <param name="PlayerOne">Action mask of J1 (REQ-INP-002).</param>
/// <param name="PlayerTwo">Action mask of J2.</param>
/// <param name="Pointer">Pointer position in the render target, in pixels, origin top left.</param>
/// <param name="PointerSize">Size of the render target the pointer is expressed in, in pixels.</param>
/// <param name="LeftButton">Whether the pointer's left button is held (REQ-INP-091).</param>
/// <param name="RightButton">Whether the pointer's right button is held (REQ-INP-062).</param>
/// <param name="Wheel">Wheel notches since the previous snapshot, positive forward.</param>
/// <param name="PointerOverUi">Whether the pointer is over a piece of interface (REQ-INP-090).</param>
/// <param name="PointerMoved">Whether the pointer moved since the previous snapshot (REQ-INP-110).</param>
public readonly record struct InputSnapshot(
    InputAction PlayerOne,
    InputAction PlayerTwo,
    Vector2 Pointer,
    Vector2 PointerSize,
    bool LeftButton = false,
    bool RightButton = false,
    float Wheel = 0f,
    bool PointerOverUi = false,
    bool PointerMoved = false)
{
    /// <summary>A snapshot where nothing is pressed.</summary>
    public static InputSnapshot Empty => new(InputAction.None, InputAction.None, Vector2.Zero, Vector2.One);

    /// <summary>The action mask of player <paramref name="index"/> (0 = J1, 1 = J2).</summary>
    public InputAction Player(int index) => index == 0 ? PlayerOne : PlayerTwo;

    /// <summary>
    /// The OR of both players' masks, which is what the hotkeys of REQ-INP-100 are read on
    /// whatever the port types are.
    /// </summary>
    public InputAction Both => PlayerOne | PlayerTwo;

    /// <summary>A snapshot carrying only the two masks, for the tests of REQ-INP-120.</summary>
    public static InputSnapshot Of(InputAction playerOne, InputAction playerTwo = InputAction.None)
        => new(playerOne, playerTwo, Vector2.Zero, Vector2.One);
}
