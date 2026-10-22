namespace Zax.App.Loop;

/// <summary>
/// The three states the emulation is always in exactly one of (SPEC-07 REQ-LOOP-001).
/// </summary>
/// <remarks>
/// There is no "idle" state distinct from <see cref="Stopped"/>, and the loading indicator is
/// orthogonal to the state: see <see cref="EmulationHost.IsLoading"/>.
/// </remarks>
public enum EmulationState
{
    /// <summary>No game is running. This is the state the application starts in.</summary>
    Stopped,

    /// <summary>A game is loaded, its state is kept, no emulated frame runs.</summary>
    Paused,

    /// <summary>Emulated frames follow one another at the target rate.</summary>
    Running,
}
