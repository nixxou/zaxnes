using System.Numerics;
using Zax.App.Persistence;
using Zax.Platform.Input;

namespace Zax.App.Input;

/// <summary>
/// The desktop host layer of SPEC-08 §3.13: it turns the keyboard, the assigned gamepads and the
/// mouse into the platform-independent snapshot of REQ-INP-120, through the bindings, the dead
/// zones and the 0.34 threshold REQ-INP-121 puts on this side of the seam.
/// </summary>
/// <remarks>
/// <para>
/// Replace this class and the logic above it keeps working: that is what makes the libretro
/// frontend of REQ-INP-121 possible, since it uses neither the binding system nor
/// <c>input.json</c> and still drives both players.
/// </para>
/// <para>
/// Commands bound to a key or a pad control (AMD-010 REQ-UI-203) are reported on their rising
/// edge in <see cref="TriggeredCommands"/>, so a held key invokes the command once.
/// </para>
/// </remarks>
public sealed class DesktopInputSource
{
    private readonly IGamepadHub _hub;
    private readonly IInputBindingStore _store;
    private readonly List<string> _pressedCommands = [];
    private readonly List<string> _triggered = [];
    private readonly HashSet<string> _heldCommands = new(StringComparer.Ordinal);

    private Vector2 _lastPointer;
    private bool _hasPointer;

    /// <summary>Creates the source.</summary>
    /// <param name="bindings">The binding model (loaded from <c>input.json</c> by the caller).</param>
    /// <param name="hub">The connected pads.</param>
    /// <param name="store">Where a binding change is written at once (REQ-INP-051).</param>
    public DesktopInputSource(InputBindings bindings, IGamepadHub hub, IInputBindingStore? store = null)
    {
        ArgumentNullException.ThrowIfNull(bindings);
        ArgumentNullException.ThrowIfNull(hub);

        Bindings = bindings;
        _hub = hub;
        _store = store ?? NullInputBindingStore.Instance;

        Pads = new GamepadAssignment(hub, bindings);
        Rebinding = new RebindingSession(bindings, Pads);

        // REQ-INP-051: the original only writes at exit; writing after each change as well is
        // the recommended behaviour, so nothing is lost on a hard kill.
        Bindings.Changed += OnBindingsChanged;
    }

    /// <summary>The binding model (REQ-INP-048: global, common to every game).</summary>
    public InputBindings Bindings { get; }

    /// <summary>The gamepad assignment of §3.3.</summary>
    public GamepadAssignment Pads { get; }

    /// <summary>The remapping model of §3.5.</summary>
    public RebindingSession Rebinding { get; }

    /// <summary>The keyboard, or null when the host has none.</summary>
    public IKeyboardDevice? Keyboard { get; set; }

    /// <summary>The mouse, or null when the host has none.</summary>
    public IPointerDevice? Pointer { get; set; }

    /// <summary>
    /// Whether a piece of interface is reading the keyboard — a text field, for instance. The
    /// keyboard then drives nothing in the game.
    /// </summary>
    public bool KeyboardCapturedByUi { get; set; }

    /// <summary>
    /// Whether the pointer is over a piece of interface, which stops the wheel from zooming the
    /// camera (REQ-INP-090).
    /// </summary>
    public bool PointerOverUi { get; set; }

    /// <summary>Size of the render target the pointer is expressed in, in pixels.</summary>
    public Vector2 RenderTargetSize { get; set; } = Vector2.One;

    /// <summary>
    /// The commands whose binding went from released to pressed during the last
    /// <see cref="Sample"/> (AMD-010 REQ-UI-203). The host queues them in the command model.
    /// </summary>
    public IReadOnlyList<string> TriggeredCommands => _triggered;

    /// <summary>
    /// Samples every device and produces one snapshot (REQ-INP-120).
    /// </summary>
    /// <param name="realSeconds">Real time since the previous call.</param>
    public InputSnapshot Sample(float realSeconds)
    {
        _hub.Poll();
        Pads.Update(realSeconds);

        // A capture consumes the sample: no action and no command is produced from the command
        // that is being captured (REQ-INP-047, DEC-08-2).
        Rebinding.Capture(Keyboard, Pads.Device(Rebinding.Player));
        bool suppressed = Rebinding.SuppressesActions;

        InputAction playerOne = suppressed ? InputAction.None : SamplePlayer(0);
        InputAction playerTwo = suppressed ? InputAction.None : SamplePlayer(1);

        CollectCommands(suppressed);

        Vector2 pointer = Pointer?.Position ?? Vector2.Zero;
        bool moved = _hasPointer && pointer != _lastPointer;
        _lastPointer = pointer;
        _hasPointer = Pointer is not null;

        return new InputSnapshot(
            playerOne,
            playerTwo,
            pointer,
            RenderTargetSize,
            Pointer?.LeftButton ?? false,
            Pointer?.RightButton ?? false,
            Pointer?.TakeWheelNotches() ?? 0f,
            PointerOverUi,
            moved);
    }

    /// <summary>
    /// Ends the sample: the "captured in this sample" flag of REQ-INP-047 is cleared so the next
    /// one behaves normally.
    /// </summary>
    public void EndSample() => Rebinding.EndSample();

    /// <summary>Detaches from the binding model.</summary>
    public void Detach() => Bindings.Changed -= OnBindingsChanged;

    /// <summary>
    /// The mask of one player: the OR of their keyboard set and of the set of the pad assigned
    /// to them (REQ-INP-002). No source can take away a bit another one raised.
    /// </summary>
    private InputAction SamplePlayer(int player)
    {
        InputAction mask = InputAction.None;

        if (Keyboard is { } keyboard && !KeyboardCapturedByUi)
        {
            mask |= Bindings.Keyboard(player).Sample(control => IsPressed(keyboard, control));
        }

        if (Pads.Device(player) is { } pad && Pads.GamepadBindings(player) is { } set)
        {
            mask |= set.Sample(control => IsPressed(pad, control));
        }

        return mask;
    }

    private void CollectCommands(bool suppressed)
    {
        _triggered.Clear();
        _pressedCommands.Clear();

        if (suppressed)
        {
            // Nothing is pressed as far as the commands are concerned, so the next real press is
            // a rising edge again.
            _heldCommands.Clear();
            return;
        }

        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            if (Keyboard is { } keyboard && !KeyboardCapturedByUi)
            {
                Bindings.Keyboard(player)
                    .SampleCommands(control => IsPressed(keyboard, control), _pressedCommands);
            }

            if (Pads.Device(player) is { } pad && Pads.GamepadBindings(player) is { } set)
            {
                set.SampleCommands(control => IsPressed(pad, control), _pressedCommands);
            }
        }

        foreach (string command in _pressedCommands)
        {
            if (_heldCommands.Add(command))
            {
                _triggered.Add(command);
            }
        }

        _heldCommands.RemoveWhere(held => !_pressedCommands.Contains(held));
    }

    private static bool IsPressed(IKeyboardDevice keyboard, PhysicalControl control)
        => control.IsKeyboard && keyboard.IsDown(control.Code);

    /// <summary>
    /// Whether a pad control counts as pressed: a button down, or an analog command at or above
    /// 0.34 of its travel after its dead zone (REQ-INP-030, REQ-INP-031).
    /// </summary>
    private static bool IsPressed(IGamepadDevice pad, PhysicalControl control)
        => control.IsGamepad && AnalogCalibration.IsPressed(pad.Value(control));

    private void OnBindingsChanged() => _store.Save(Bindings);
}
