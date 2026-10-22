namespace Zax.App.Input;

/// <summary>
/// The whole binding model of SPEC-08 §3.5 and §3.6: a keyboard set per player, a gamepad set per
/// device identity, and the identity of the last gamepad each player used.
/// </summary>
/// <remarks>
/// <para>
/// Gamepad sets belong to the <b>device</b>, not to the player (REQ-INP-025): a pad that moves
/// from J1 to J2 takes its bindings with it, and a pad of another model keeps its own. The three
/// pieces of data listed here are exactly what REQ-INP-050 asks to keep from one session to the
/// next; the file itself is SPEC-09 REQ-PERS-065.
/// </para>
/// <para>
/// Every mutation raises <see cref="Changed"/>, which is how the recommended "write after each
/// change" of REQ-INP-051 is implemented without the model knowing about a file.
/// </para>
/// </remarks>
public sealed class InputBindings
{
    private readonly BindingSet[] _keyboards;
    private readonly Dictionary<string, BindingSet> _gamepads = new(StringComparer.OrdinalIgnoreCase);
    private readonly string?[] _lastGamepads = new string?[InputActions.PlayerCount];

    private bool _muted;

    /// <summary>Creates the model with the defaults of §4.2 (J1) and an empty J2 keyboard.</summary>
    public InputBindings()
    {
        _keyboards = new BindingSet[InputActions.PlayerCount];
        for (int player = 0; player < _keyboards.Length; player++)
        {
            Attach(_keyboards[player] = DefaultBindings.CreateKeyboard(player));
        }
    }

    /// <summary>Raised after any binding or assignment memory changed (REQ-INP-051).</summary>
    public event Action? Changed;

    /// <summary>The identities that have a remembered gamepad set.</summary>
    public IReadOnlyCollection<string> GamepadIdentities => _gamepads.Keys;

    /// <summary>The keyboard set of <paramref name="player"/> (0 = J1, 1 = J2), REQ-INP-010.</summary>
    public BindingSet Keyboard(int player)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThanOrEqual(
            (uint)player,
            (uint)InputActions.PlayerCount,
            nameof(player));
        return _keyboards[player];
    }

    /// <summary>Whether a set is already remembered for <paramref name="identity"/>.</summary>
    public bool HasGamepad(string identity) => _gamepads.ContainsKey(identity);

    /// <summary>
    /// The gamepad set of <paramref name="identity"/>. When none exists yet, the defaults of
    /// §4.3 are applied and immediately remembered for that identity (REQ-INP-025).
    /// </summary>
    /// <param name="identity">The device identity (SPEC-08 §2).</param>
    /// <param name="standard">
    /// Whether the pad answers to the standard layout and therefore gets the §4.3 defaults; a
    /// pad without a mapping starts empty (REQ-INP-021).
    /// </param>
    public BindingSet Gamepad(string identity, bool standard = true)
    {
        ArgumentException.ThrowIfNullOrEmpty(identity);

        if (_gamepads.TryGetValue(identity, out BindingSet? existing))
        {
            return existing;
        }

        BindingSet created = DefaultBindings.CreateGamepad(standard);
        _gamepads[identity] = created;
        Attach(created);
        Raise();
        return created;
    }

    /// <summary>
    /// The gamepad set of <paramref name="identity"/> if one is remembered, without creating
    /// anything. Used when reading the persisted file.
    /// </summary>
    public BindingSet? FindGamepad(string identity)
        => _gamepads.GetValueOrDefault(identity);

    /// <summary>
    /// The identity of the last gamepad assigned to <paramref name="player"/>, or null for
    /// "none" (REQ-INP-050, item 3).
    /// </summary>
    public string? LastGamepad(int player)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThanOrEqual(
            (uint)player,
            (uint)InputActions.PlayerCount,
            nameof(player));
        return _lastGamepads[player];
    }

    /// <summary>Remembers the gamepad of <paramref name="player"/>; null means "none".</summary>
    public void SetLastGamepad(int player, string? identity)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThanOrEqual(
            (uint)player,
            (uint)InputActions.PlayerCount,
            nameof(player));

        if (string.Equals(_lastGamepads[player], identity, StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        _lastGamepads[player] = identity;
        Raise();
    }

    /// <summary>
    /// Puts the keyboard set of <paramref name="player"/> back to its default: the table of §4.2
    /// for J1, an empty set for J2 (REQ-INP-049).
    /// </summary>
    public void ResetKeyboard(int player)
    {
        BindingSet defaults = DefaultBindings.CreateKeyboard(player);
        Keyboard(player).Replace(defaults.Bindings);
    }

    /// <summary>
    /// Puts the gamepad set of <paramref name="identity"/> back to the table of §4.3
    /// (REQ-INP-049).
    /// </summary>
    public void ResetGamepad(string identity, bool standard = true)
    {
        BindingSet defaults = DefaultBindings.CreateGamepad(standard);
        Gamepad(identity, standard).Replace(defaults.Bindings);
    }

    /// <summary>
    /// Puts the whole model back to the factory state: default keyboards, no gamepad set, no
    /// remembered gamepad. This is also the state after an absent or unreadable file
    /// (REQ-INP-052).
    /// </summary>
    public void ResetAll()
    {
        using (Mute())
        {
            for (int player = 0; player < InputActions.PlayerCount; player++)
            {
                ResetKeyboard(player);
                _lastGamepads[player] = null;
            }

            _gamepads.Clear();
        }

        Raise();
    }

    /// <summary>
    /// Suspends <see cref="Changed"/> until the returned scope is disposed, then raises it once.
    /// The file reader uses it so that loading does not write the file back line by line.
    /// </summary>
    public IDisposable Mute() => new MuteScope(this);

    private void Attach(BindingSet set) => set.Changed += Raise;

    private void Raise()
    {
        if (!_muted)
        {
            Changed?.Invoke();
        }
    }

    private sealed class MuteScope : IDisposable
    {
        private readonly InputBindings _owner;
        private readonly bool _previous;

        public MuteScope(InputBindings owner)
        {
            _owner = owner;
            _previous = owner._muted;
            owner._muted = true;
        }

        public void Dispose() => _owner._muted = _previous;
    }
}
