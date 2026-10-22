using Zax.Platform.Input;

namespace Zax.App.Input;

/// <summary>
/// The gamepad assignment of SPEC-08 §3.3: at most one pad per player, reconnected at start-up,
/// assigned automatically on first use, assignable by hand, and released on unplug
/// (REQ-INP-023, REQ-INP-024, REQ-INP-025, DEC-08-1).
/// </summary>
/// <remarks>
/// <para>
/// A player's pad is tracked by its identity rather than by the device object, because the hub
/// re-reads the device list on every poll. Resolving the identity against the current list is
/// also what makes the unplug of REQ-INP-024 observable: the identity stays remembered as the
/// "last gamepad" of REQ-INP-050, but <see cref="Device"/> returns null, so the player's pad
/// bits fall to zero at the next sample.
/// </para>
/// <para>
/// Assigning a pad to a player removes it from the other one, which is the accepted divergence
/// DEC-08-1 (the original lets the same pad drive both).
/// </para>
/// </remarks>
public sealed class GamepadAssignment
{
    /// <summary>
    /// How long after launch the remembered pads are reconnected, about 0.5 s
    /// (REQ-INP-023, rule 1).
    /// </summary>
    public const float StartupDelaySeconds = 0.5f;

    private readonly IGamepadHub _hub;
    private readonly InputBindings _bindings;
    private readonly string?[] _assigned = new string?[InputActions.PlayerCount];
    private readonly List<PhysicalControl> _active = [];

    private float _sinceStart;
    private bool _startupDone;

    /// <summary>Creates the assignment over <paramref name="hub"/>.</summary>
    /// <param name="hub">The connected pads (VR devices already filtered out, REQ-INP-022).</param>
    /// <param name="bindings">The binding model, for the remembered identities and the sets.</param>
    public GamepadAssignment(IGamepadHub hub, InputBindings bindings)
    {
        ArgumentNullException.ThrowIfNull(hub);
        ArgumentNullException.ThrowIfNull(bindings);
        _hub = hub;
        _bindings = bindings;
    }

    /// <summary>
    /// The pads a player may be given: every connected non-VR pad. The chooser of REQ-INP-041
    /// shows "none" plus this list, and it updates on plug and unplug.
    /// </summary>
    public IReadOnlyList<IGamepadDevice> Available => _hub.Devices;

    /// <summary>Whether the start-up reconnection of REQ-INP-023 has already run.</summary>
    public bool StartupComplete => _startupDone;

    /// <summary>
    /// The pad currently driving <paramref name="player"/>, or null when they have none — never
    /// assigned, or assigned and then unplugged (REQ-INP-023, REQ-INP-024).
    /// </summary>
    public IGamepadDevice? Device(int player)
    {
        if ((uint)player >= InputActions.PlayerCount || _assigned[player] is not { } identity)
        {
            return null;
        }

        return Find(identity);
    }

    /// <summary>The identity assigned to <paramref name="player"/>, plugged in or not.</summary>
    public string? AssignedIdentity(int player)
        => (uint)player < InputActions.PlayerCount ? _assigned[player] : null;

    /// <summary>The binding set of <paramref name="player"/>'s pad, or null when they have none.</summary>
    public BindingSet? GamepadBindings(int player)
    {
        if (Device(player) is not { } device)
        {
            return null;
        }

        // REQ-INP-025: the set of this identity, created from the §4.3 defaults and remembered
        // at once the first time the pad is seen.
        return _bindings.Gamepad(device.Identity, device.IsStandard);
    }

    /// <summary>
    /// One frame of the assignment rules: the start-up reconnection once the delay has elapsed,
    /// then the automatic assignment of a pad that becomes active (REQ-INP-023).
    /// </summary>
    /// <param name="realSeconds">Real time since the previous call.</param>
    public void Update(float realSeconds)
    {
        if (!_startupDone)
        {
            _sinceStart += realSeconds;
            if (_sinceStart < StartupDelaySeconds)
            {
                return;
            }

            ReconnectRemembered();
            _startupDone = true;
        }

        AssignActiveDevices();
    }

    /// <summary>
    /// Manual assignment from the configuration screen (REQ-INP-041): gives
    /// <paramref name="device"/> to <paramref name="player"/>, or removes their pad when it is
    /// null. The pad is taken away from the other player (DEC-08-1).
    /// </summary>
    public void Assign(int player, IGamepadDevice? device)
    {
        ArgumentOutOfRangeException.ThrowIfGreaterThanOrEqual(
            (uint)player,
            (uint)InputActions.PlayerCount,
            nameof(player));

        if (device is null)
        {
            _assigned[player] = null;
            _bindings.SetLastGamepad(player, null);
            return;
        }

        for (int other = 0; other < InputActions.PlayerCount; other++)
        {
            if (other == player)
            {
                continue;
            }

            if (string.Equals(_assigned[other], device.Identity, StringComparison.OrdinalIgnoreCase))
            {
                _assigned[other] = null;
                _bindings.SetLastGamepad(other, null);
            }
        }

        _assigned[player] = device.Identity;

        // REQ-INP-025: whatever the means, the set of this identity is loaded — and created from
        // the defaults the first time — as soon as the pad is assigned.
        _bindings.Gamepad(device.Identity, device.IsStandard);
        _bindings.SetLastGamepad(player, device.Identity);
    }

    /// <summary>
    /// Forces the start-up reconnection to be considered done, which a test that does not want
    /// to wait 0.5 s uses.
    /// </summary>
    public void SkipStartupDelay()
    {
        if (_startupDone)
        {
            return;
        }

        ReconnectRemembered();
        _startupDone = true;
    }

    /// <summary>
    /// Rule 1 of REQ-INP-023: J1 takes back the pad whose identity equals its remembered one if
    /// it is plugged in, then J2 does the same, excluding the one J1 just took.
    /// </summary>
    private void ReconnectRemembered()
    {
        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            if (_bindings.LastGamepad(player) is not { } identity)
            {
                continue;
            }

            if (IsTaken(identity, player) || Find(identity) is not { } device)
            {
                continue;
            }

            _assigned[player] = device.Identity;
            _bindings.Gamepad(device.Identity, device.IsStandard);
        }
    }

    /// <summary>
    /// Rule 2 of REQ-INP-023: an unassigned non-VR pad that becomes active goes to J1 if J1 has
    /// none, otherwise to J2.
    /// </summary>
    private void AssignActiveDevices()
    {
        int free = FirstPlayerWithoutPad();
        if (free < 0)
        {
            return;
        }

        foreach (IGamepadDevice device in _hub.Devices)
        {
            if (IsTaken(device.Identity, player: -1))
            {
                continue;
            }

            _active.Clear();
            device.CollectActiveControls(_active);
            if (_active.Count == 0)
            {
                continue;
            }

            Assign(free, device);

            free = FirstPlayerWithoutPad();
            if (free < 0)
            {
                return;
            }
        }
    }

    private int FirstPlayerWithoutPad()
    {
        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            if (Device(player) is null)
            {
                return player;
            }
        }

        return -1;
    }

    /// <summary>
    /// Whether <paramref name="identity"/> is already assigned to a player other than
    /// <paramref name="player"/> (pass −1 to test every player).
    /// </summary>
    private bool IsTaken(string identity, int player)
    {
        for (int other = 0; other < InputActions.PlayerCount; other++)
        {
            if (other == player)
            {
                continue;
            }

            if (string.Equals(_assigned[other], identity, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }

    private IGamepadDevice? Find(string identity)
    {
        foreach (IGamepadDevice device in _hub.Devices)
        {
            if (string.Equals(device.Identity, identity, StringComparison.OrdinalIgnoreCase))
            {
                return device;
            }
        }

        return null;
    }
}
