using Zax.App.Input;

namespace Zax.App.Loop;

/// <summary>What the system hotkeys produced during one rendered frame (SPEC-08 §3.11).</summary>
[Flags]
public enum HotkeyEvents
{
    /// <summary>Nothing happened.</summary>
    None = 0,

    /// <summary>Menu released in less than 0.3 s: pause / resume (REQ-INP-101).</summary>
    MenuShort = 1 << 0,

    /// <summary>Menu held for 0.3 s: reset the camera view, no state change.</summary>
    MenuLong = 1 << 1,

    /// <summary>Rising edge of RollBack (REQ-INP-102).</summary>
    Rollback = 1 << 2,

    /// <summary>One fast-forward request, emitted on every frame the key is held (REQ-INP-103).</summary>
    FastForward = 1 << 3,

    /// <summary>Quick save (REQ-INP-104).</summary>
    QuickSave = 1 << 4,

    /// <summary>Quick load (REQ-INP-104).</summary>
    QuickLoad = 1 << 5,
}

/// <summary>
/// Turns the per-frame action masks into the hotkey events of SPEC-08 §3.11.
/// </summary>
/// <remarks>
/// <para>
/// Hotkeys are evaluated on the <b>display</b> clock, not on the emulated-frame clock
/// (REQ-INP-100), which is why they live here and not in the session: only their consequences —
/// commands applied between two emulated frames — are part of the determinism guarantee of
/// REQ-ARCH-012.
/// </para>
/// <para>
/// Everything but the two combinations is read on the OR of both players' masks, whatever the
/// port types are (REQ-INP-100). The combinations of REQ-INP-104 are the exception: they are
/// evaluated <b>player by player</b>, so holding <c>Select</c> on J1 and flicking J2's right
/// stick saves nothing. <c>Select</c> keeps reaching the game and the stick keeps turning the
/// camera, which is the original behaviour.
/// </para>
/// </remarks>
public sealed class HotkeyTracker
{
    /// <summary>How long <c>Select</c> must be held before the combinations arm (REQ-INP-104).</summary>
    public const float SelectHoldSeconds = 0.1f;

    private readonly PressTracker _menu = new();
    private readonly PlayerCombination[] _combinations =
        [new PlayerCombination(), new PlayerCombination()];

    private EdgeDetector _rollback;
    private EdgeDetector _quickSave;
    private EdgeDetector _quickLoad;

    /// <summary>How long the Menu action has been held, for the UI.</summary>
    public float MenuHeldSeconds => _menu.HeldSeconds;

    /// <summary>
    /// Reads one displayed frame and returns the events it produced.
    /// </summary>
    /// <param name="playerOne">Action mask of J1.</param>
    /// <param name="playerTwo">Action mask of J2.</param>
    /// <param name="realSeconds">Real time since the previous call.</param>
    public HotkeyEvents Update(InputAction playerOne, InputAction playerTwo, float realSeconds)
    {
        InputAction mask = playerOne | playerTwo;
        HotkeyEvents events = HotkeyEvents.None;

        events |= _menu.Update(mask.HasFlag(InputAction.Menu), realSeconds) switch
        {
            PressKind.Short => HotkeyEvents.MenuShort,
            PressKind.Long => HotkeyEvents.MenuLong,
            _ => HotkeyEvents.None,
        };

        if (_rollback.Rising(mask.HasFlag(InputAction.RollBack)))
        {
            events |= HotkeyEvents.Rollback;
        }

        // REQ-INP-103: a request per displayed frame while the key is held.
        if (mask.HasFlag(InputAction.FastForward))
        {
            events |= HotkeyEvents.FastForward;
        }

        if (_quickSave.Rising(mask.HasFlag(InputAction.QuickSave)))
        {
            events |= HotkeyEvents.QuickSave;
        }

        if (_quickLoad.Rising(mask.HasFlag(InputAction.QuickLoad)))
        {
            events |= HotkeyEvents.QuickLoad;
        }

        events |= _combinations[0].Update(playerOne, realSeconds);
        events |= _combinations[1].Update(playerTwo, realSeconds);

        return events;
    }

    /// <summary>
    /// Reads one displayed frame with a single merged mask: the combinations are then evaluated
    /// as if every action belonged to J1.
    /// </summary>
    public HotkeyEvents Update(InputAction mask, float realSeconds)
        => Update(mask, InputAction.None, realSeconds);

    /// <summary>
    /// The two quick-slot combinations of REQ-INP-104 for one player: <c>Select</c> held for
    /// more than 0.1 s, then <c>RotDown</c> released saves and <c>RotUp</c> released loads.
    /// </summary>
    private sealed class PlayerCombination
    {
        private EdgeDetector _rotDown;
        private EdgeDetector _rotUp;
        private float _selectHeld;

        public HotkeyEvents Update(InputAction mask, float realSeconds)
        {
            bool select = mask.HasFlag(InputAction.Select);

            // The arming is tested against the hold measured *before* this frame, so a frame in
            // which Select and the stick appear together never fires.
            bool armed = select && _selectHeld >= SelectHoldSeconds;
            _selectHeld = select ? _selectHeld + realSeconds : 0f;

            HotkeyEvents events = HotkeyEvents.None;

            if (_rotDown.Falling(mask.HasFlag(InputAction.RotDown)) && armed)
            {
                events |= HotkeyEvents.QuickSave;
            }

            if (_rotUp.Falling(mask.HasFlag(InputAction.RotUp)) && armed)
            {
                events |= HotkeyEvents.QuickLoad;
            }

            return events;
        }
    }
}
