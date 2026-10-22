using Zax.Platform.Input;

namespace Zax.App.Input;

/// <summary>
/// The remapping model of SPEC-08 §3.5: which player and which device kind the key configuration
/// screen is showing, and the capture of one binding (REQ-INP-040 to REQ-INP-047).
/// </summary>
/// <remarks>
/// <para>
/// The screen itself is lot L9 (SPEC-10, AMD-010 REQ-UI-203); what lives here is the model it
/// drives and the rules the capture has to obey, so both the screen and the tests go through the
/// same code.
/// </para>
/// <para>
/// While a capture is listening, the host must stop producing action masks altogether: that is
/// what suspends the interface navigation of REQ-INP-047 and what prevents the double effect of
/// <c>Escape</c> — captured as a binding <b>and</b> seen as <c>Menu</c> — that DEC-08-2 asks to
/// avoid. <c>Escape</c> is not reserved for cancelling: it can be captured like any other key.
/// </para>
/// <para>
/// A control counts as "detected" when it goes from released to pressed <b>while</b> the session
/// is listening (see <c>spec/changes/Q-033.md</c>): a key still held from the click or the key
/// press that opened the field is not captured.
/// </para>
/// </remarks>
// Q-033
public sealed class RebindingSession
{
    private readonly InputBindings _bindings;
    private readonly GamepadAssignment? _pads;
    private readonly List<int> _keys = [];
    private readonly List<PhysicalControl> _controls = [];
    private readonly HashSet<PhysicalControl> _heldAtStart = [];

    private bool _primed;

    /// <summary>Creates the model over <paramref name="bindings"/>.</summary>
    /// <param name="bindings">The binding model being edited.</param>
    /// <param name="pads">
    /// The gamepad assignment, needed to know which pad's set the Gamepad mode edits
    /// (REQ-INP-042, REQ-INP-046). Null means "no pad is ever assigned".
    /// </param>
    public RebindingSession(InputBindings bindings, GamepadAssignment? pads = null)
    {
        ArgumentNullException.ThrowIfNull(bindings);
        _bindings = bindings;
        _pads = pads;
    }

    /// <summary>The player whose sets the screen shows, 0 = J1, 1 = J2 (REQ-INP-040).</summary>
    public int Player { get; private set; }

    /// <summary>The device kind the screen shows (REQ-INP-040).</summary>
    public BindingDeviceKind DeviceKind { get; private set; } = BindingDeviceKind.Keyboard;

    /// <summary>The target being captured, or an empty target when nothing is listening.</summary>
    public BindingTarget Target { get; private set; }

    /// <summary>Whether a capture is waiting for a physical command (REQ-INP-042).</summary>
    public bool IsListening { get; private set; }

    /// <summary>
    /// Whether a command was captured during the last <see cref="Capture"/> call. The host uses
    /// it to make sure that command triggers nothing else in that sample (REQ-INP-047).
    /// </summary>
    public bool CapturedThisSample { get; private set; }

    /// <summary>The control the last capture produced, or <c>None</c>.</summary>
    public PhysicalControl LastCaptured { get; private set; }

    /// <summary>
    /// The set the screen is editing: the player's keyboard set, or the set of the pad assigned
    /// to them. Null in Gamepad mode when the player has no pad, in which case the 19 fields are
    /// empty (REQ-INP-046).
    /// </summary>
    public BindingSet? ActiveSet => DeviceKind == BindingDeviceKind.Keyboard
        ? _bindings.Keyboard(Player)
        : _pads?.GamepadBindings(Player);

    /// <summary>
    /// Whether the interface must suspend its directional navigation and swallow the captured
    /// command for this sample (REQ-INP-047, DEC-08-2).
    /// </summary>
    public bool SuppressesActions => IsListening || CapturedThisSample;

    /// <summary>
    /// Shows another player's sets. Any capture in progress stops without creating a binding
    /// (REQ-INP-045).
    /// </summary>
    public void SelectPlayer(int player)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThanOrEqual(
            (uint)player,
            (uint)InputActions.PlayerCount,
            nameof(player));

        Stop();
        Player = player;
    }

    /// <summary>Switches between the Keyboard and Gamepad sets, stopping any capture.</summary>
    public void SelectDeviceKind(BindingDeviceKind kind)
    {
        Stop();
        DeviceKind = kind;
    }

    /// <summary>
    /// Tells the model that the pad assigned to the shown player changed, which also stops a
    /// capture in progress (REQ-INP-045).
    /// </summary>
    public void NotifyGamepadChanged() => Stop();

    /// <summary>
    /// Activates the field of <paramref name="target"/>: the existing binding is cleared at once
    /// and the model starts listening (REQ-INP-042). Activating another field while one is
    /// listening stops the first without binding it (REQ-INP-045).
    /// </summary>
    /// <returns>
    /// False when there is nothing to edit — an empty target, or Gamepad mode without an
    /// assigned pad (REQ-INP-046).
    /// </returns>
    public bool Begin(BindingTarget target)
    {
        Stop();

        if (target.IsEmpty || ActiveSet is not { } set)
        {
            return false;
        }

        // REQ-INP-042: activating the field erases the binding immediately, so stopping the
        // listening afterwards leaves the action empty (REQ-INP-045).
        set.Clear(target);

        Target = target;
        IsListening = true;
        _primed = false;
        _heldAtStart.Clear();
        return true;
    }

    /// <summary>Activates the field of <paramref name="action"/>.</summary>
    public bool Begin(InputAction action) => Begin(BindingTarget.Of(action));

    /// <summary>Activates the field of the command <paramref name="command"/> (REQ-UI-203).</summary>
    public bool BeginCommand(string command) => Begin(BindingTarget.OfCommand(command));

    /// <summary>
    /// Stops listening without creating a binding, which is what the "stop listening" button,
    /// a player, type or pad change, and closing the screen all do (REQ-INP-045).
    /// </summary>
    public void Stop()
    {
        IsListening = false;
        Target = default;
        _primed = false;
        _heldAtStart.Clear();
    }

    /// <summary>Closes the screen: the capture stops and the sample flag is cleared.</summary>
    public void Close()
    {
        Stop();
        CapturedThisSample = false;
        LastCaptured = PhysicalControl.None;
    }

    /// <summary>
    /// One sample of the capture (REQ-INP-042): the first eligible physical command that becomes
    /// pressed while listening becomes the binding, and the listening stops.
    /// </summary>
    /// <param name="keyboard">The keyboard, for a Keyboard capture.</param>
    /// <param name="pad">The player's pad, for a Gamepad capture.</param>
    /// <returns>The captured control, or <c>None</c> when nothing was captured this sample.</returns>
    public PhysicalControl Capture(IKeyboardDevice? keyboard, IGamepadDevice? pad)
    {
        CapturedThisSample = false;

        if (!IsListening || ActiveSet is not { } set)
        {
            return PhysicalControl.None;
        }

        _controls.Clear();
        if (DeviceKind == BindingDeviceKind.Keyboard)
        {
            // Keys only, modifiers included; mouse buttons are not part of the vocabulary
            // (REQ-INP-012, REQ-INP-042).
            if (keyboard is not null)
            {
                _keys.Clear();
                keyboard.CollectHeldKeys(_keys);
                _keys.Sort();
                foreach (int key in _keys)
                {
                    _controls.Add(PhysicalControl.Key(key));
                }
            }
        }
        else
        {
            // Buttons, hat, stick half-axes and triggers of the assigned pad, non-standard
            // controls included (REQ-INP-021, REQ-INP-042).
            pad?.CollectActiveControls(_controls);
        }

        if (!_primed)
        {
            // Q-033: whatever is already held when the field is activated is ignored, so the
            // click or key press that opened the field never becomes the binding.
            _heldAtStart.Clear();
            foreach (PhysicalControl held in _controls)
            {
                _heldAtStart.Add(held);
            }

            _primed = true;
            return PhysicalControl.None;
        }

        PhysicalControl captured = PhysicalControl.None;
        foreach (PhysicalControl control in _controls)
        {
            if (!_heldAtStart.Contains(control))
            {
                captured = control;
                break;
            }
        }

        // A control released since the field was activated becomes capturable again.
        _heldAtStart.RemoveWhere(held => !_controls.Contains(held));

        if (!captured.IsBound)
        {
            return PhysicalControl.None;
        }

        BindingTarget target = Target;
        Stop();
        set.Set(target, captured);

        CapturedThisSample = true;
        LastCaptured = captured;
        return captured;
    }

    /// <summary>
    /// Clears the "captured in this sample" flag, which the host does once the sample it was
    /// raised in is over.
    /// </summary>
    public void EndSample() => CapturedThisSample = false;
}
