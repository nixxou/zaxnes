namespace Zax.App.Loop;

/// <summary>
/// The fast-forward model of SPEC-07 REQ-LOOP-016: every request arms 30 emulated frames at
/// <c>k = 1.5</c>, and a request received while one is already running has no effect.
/// </summary>
/// <remarks>
/// The hotkey emits one request per <b>rendered</b> frame while it is held (SPEC-08
/// REQ-INP-103), so holding it re-arms the burst as soon as the previous one ends: the speed is
/// ×1.5 almost continuously. On release, 0 to 30 emulated frames remain, then <c>k</c> goes back
/// to the speed setting.
/// </remarks>
public sealed class FastForwardState
{
    /// <summary>Emulated frames one request arms (REQ-LOOP-016).</summary>
    public const int BurstFrames = 30;

    private int _remaining;

    /// <summary>Whether a burst is running, that is whether <c>k</c> is forced to 1.5.</summary>
    public bool IsActive => _remaining > 0;

    /// <summary>Emulated frames left in the current burst, 0 when none runs.</summary>
    public int Remaining => _remaining;

    /// <summary>
    /// One request. Arms a burst when none is running; does nothing otherwise.
    /// </summary>
    /// <returns>True when this request armed a new burst.</returns>
    public bool Request()
    {
        if (_remaining > 0)
        {
            return false;
        }

        _remaining = BurstFrames;
        return true;
    }

    /// <summary>Accounts for one emulated frame that has just run.</summary>
    public void OnFrameRun()
    {
        if (_remaining > 0)
        {
            _remaining--;
        }
    }

    /// <summary>Drops the burst: what stopping a game or loading a state does.</summary>
    public void Clear() => _remaining = 0;
}
