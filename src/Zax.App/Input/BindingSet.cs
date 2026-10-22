using Zax.Platform.Input;

namespace Zax.App.Input;

/// <summary>
/// One binding set: the bindings of one device kind, for one player's keyboard or for one
/// gamepad identity (SPEC-08 §2).
/// </summary>
/// <remarks>
/// <para>
/// Each target has <b>at most one</b> binding in the set, and capturing a new one replaces the
/// old (REQ-INP-043). There is no conflict detection: the same physical command may be bound to
/// several targets of the same set and then triggers all of them (REQ-INP-044).
/// </para>
/// <para>
/// Bindings are global, common to every game (REQ-INP-048); only the port type varies per game,
/// at a profile script's request (REQ-INP-060).
/// </para>
/// </remarks>
public sealed class BindingSet
{
    private readonly Dictionary<BindingTarget, PhysicalControl> _bindings = [];

    /// <summary>The device kind this set belongs to.</summary>
    public BindingDeviceKind Kind { get; }

    /// <summary>Creates an empty set of <paramref name="kind"/> (REQ-INP-011 for J2's keyboard).</summary>
    public BindingSet(BindingDeviceKind kind)
    {
        Kind = kind;
    }

    /// <summary>Raised whenever a binding is set or cleared, for the write of REQ-INP-051.</summary>
    public event Action? Changed;

    /// <summary>How many targets are bound.</summary>
    public int Count => _bindings.Count;

    /// <summary>Every binding of the set.</summary>
    public IReadOnlyDictionary<BindingTarget, PhysicalControl> Bindings => _bindings;

    /// <summary>The control bound to <paramref name="target"/>, or <c>None</c> (REQ-INP-045).</summary>
    public PhysicalControl Get(BindingTarget target)
        => _bindings.GetValueOrDefault(target, PhysicalControl.None);

    /// <summary>The control bound to <paramref name="action"/>, or <c>None</c>.</summary>
    public PhysicalControl Get(InputAction action) => Get(BindingTarget.Of(action));

    /// <summary>
    /// Binds <paramref name="control"/> to <paramref name="target"/>, replacing whatever was
    /// there (REQ-INP-043). An unbound control clears the entry.
    /// </summary>
    /// <returns>False when the control does not belong to this set's device kind.</returns>
    public bool Set(BindingTarget target, PhysicalControl control)
    {
        if (target.IsEmpty)
        {
            return false;
        }

        if (!control.IsBound)
        {
            Clear(target);
            return true;
        }

        // REQ-INP-042: a keyboard set only accepts keys, a gamepad set only accepts pad
        // controls. Mouse buttons are not bindable at all (REQ-INP-012), which the
        // PhysicalControl vocabulary already enforces: it has no mouse-button kind.
        bool eligible = Kind == BindingDeviceKind.Keyboard ? control.IsKeyboard : control.IsGamepad;
        if (!eligible)
        {
            return false;
        }

        _bindings[target] = control;
        Changed?.Invoke();
        return true;
    }

    /// <summary>Binds <paramref name="control"/> to <paramref name="action"/>.</summary>
    public bool Set(InputAction action, PhysicalControl control)
        => Set(BindingTarget.Of(action), control);

    /// <summary>
    /// Removes the binding of <paramref name="target"/>, which is what activating a field does
    /// before the listening starts (REQ-INP-042).
    /// </summary>
    public void Clear(BindingTarget target)
    {
        if (_bindings.Remove(target))
        {
            Changed?.Invoke();
        }
    }

    /// <summary>Removes every binding (REQ-INP-049, the empty J2 keyboard of REQ-INP-011).</summary>
    public void ClearAll()
    {
        if (_bindings.Count == 0)
        {
            return;
        }

        _bindings.Clear();
        Changed?.Invoke();
    }

    /// <summary>Replaces the content of the set with <paramref name="bindings"/>, in one event.</summary>
    public void Replace(IEnumerable<KeyValuePair<BindingTarget, PhysicalControl>> bindings)
    {
        ArgumentNullException.ThrowIfNull(bindings);

        _bindings.Clear();
        foreach ((BindingTarget target, PhysicalControl control) in bindings)
        {
            bool eligible = Kind == BindingDeviceKind.Keyboard ? control.IsKeyboard : control.IsGamepad;
            if (!target.IsEmpty && control.IsBound && eligible)
            {
                _bindings[target] = control;
            }
        }

        Changed?.Invoke();
    }

    /// <summary>
    /// The action mask every bound action whose control is pressed contributes
    /// (REQ-INP-002, REQ-INP-044: one control may raise several bits).
    /// </summary>
    /// <param name="isPressed">Tells whether a physical control is pressed right now.</param>
    public InputAction Sample(Func<PhysicalControl, bool> isPressed)
    {
        ArgumentNullException.ThrowIfNull(isPressed);

        InputAction mask = InputAction.None;
        foreach ((BindingTarget target, PhysicalControl control) in _bindings)
        {
            if (target.IsAction && isPressed(control))
            {
                mask |= target.Action;
            }
        }

        return mask;
    }

    /// <summary>
    /// Appends the commands of this set whose control is pressed to <paramref name="commands"/>
    /// (AMD-010 REQ-UI-203). The caller turns rising edges into invocations.
    /// </summary>
    public void SampleCommands(Func<PhysicalControl, bool> isPressed, ICollection<string> commands)
    {
        ArgumentNullException.ThrowIfNull(isPressed);
        ArgumentNullException.ThrowIfNull(commands);

        foreach ((BindingTarget target, PhysicalControl control) in _bindings)
        {
            if (target.Command is { } command && isPressed(control))
            {
                commands.Add(command);
            }
        }
    }

    /// <summary>A copy of this set, used by the defaults and by the tests.</summary>
    public BindingSet Clone()
    {
        BindingSet copy = new(Kind);
        foreach ((BindingTarget target, PhysicalControl control) in _bindings)
        {
            copy._bindings[target] = control;
        }

        return copy;
    }
}
