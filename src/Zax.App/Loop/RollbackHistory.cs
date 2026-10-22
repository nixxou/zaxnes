namespace Zax.App.Loop;

/// <summary>
/// The rollback history of SPEC-07 §3.4: a snapshot of the <b>core alone</b> every 60 emulated
/// frames, at most 30 of them, and a restore that goes back to the second-to-last one.
/// </summary>
/// <remarks>
/// <para>
/// A snapshot holds the core and nothing else — not the scripts' persistent data, not the
/// camera, not the 3D scene (REQ-LOOP-030, DEC-07-Q3). The depth is therefore about 30 s in
/// NTSC and 36 s in PAL/Dendy.
/// </para>
/// <para>
/// The history is never persisted (SPEC-09 REQ-PERS-058). It is cleared when a game is
/// launched, when a savestate is loaded and on every reset, and <b>not</b> by a pause, a quick
/// save or a rollback (REQ-LOOP-031).
/// </para>
/// </remarks>
public sealed class RollbackHistory
{
    /// <summary>Highest number of snapshots kept (REQ-LOOP-030).</summary>
    public const int Capacity = 30;

    /// <summary>A snapshot is taken on every frame counter that is a multiple of 60.</summary>
    public const int IntervalFrames = 60;

    private readonly List<byte[]> _snapshots = [];

    /// <summary>Snapshots currently held, oldest first.</summary>
    public int Count => _snapshots.Count;

    /// <summary>Whether a rollback would restore something.</summary>
    public bool IsEmpty => _snapshots.Count == 0;

    /// <summary>
    /// Whether the frame about to run deserves a snapshot: its core frame counter is a
    /// multiple of 60, so the snapshot is taken <b>before</b> the frame executes.
    /// </summary>
    public static bool IsSnapshotFrame(long coreFrameCount)
        => coreFrameCount >= 0 && coreFrameCount % IntervalFrames == 0;

    /// <summary>Appends a snapshot, dropping the oldest one when the history is full.</summary>
    public void Capture(byte[] coreState)
    {
        ArgumentNullException.ThrowIfNull(coreState);

        if (_snapshots.Count == Capacity)
        {
            _snapshots.RemoveAt(0);
        }

        _snapshots.Add(coreState);
    }

    /// <summary>
    /// Takes the snapshot a rollback restores and removes it from the history together with the
    /// last one (REQ-LOOP-032 step 2).
    /// </summary>
    /// <remarks>
    /// With a single snapshot, that snapshot is restored and removed; with none, nothing is
    /// restored, but the caller still applies step 3 (go to Running, reset the clock). The
    /// original misbehaves in both cases and is deliberately not reproduced
    /// (REQ-LOOP-032, notes).
    /// </remarks>
    /// <returns>The core state to restore, or null when the history is empty.</returns>
    public byte[]? TakeRollbackTarget()
    {
        if (_snapshots.Count == 0)
        {
            return null;
        }

        if (_snapshots.Count == 1)
        {
            byte[] only = _snapshots[0];
            _snapshots.Clear();
            return only;
        }

        byte[] target = _snapshots[^2];
        _snapshots.RemoveAt(_snapshots.Count - 1);
        _snapshots.RemoveAt(_snapshots.Count - 1);
        return target;
    }

    /// <summary>
    /// The optional history panel of REQ-LOOP-033: restoring entry <paramref name="index"/>
    /// (0 = oldest) keeps only the entries before it.
    /// </summary>
    /// <returns>The core state to restore, or null when the index is out of range.</returns>
    public byte[]? TakeAt(int index)
    {
        if (index < 0 || index >= _snapshots.Count)
        {
            return null;
        }

        byte[] target = _snapshots[index];
        _snapshots.RemoveRange(index, _snapshots.Count - index);
        return target;
    }

    /// <summary>Drops every snapshot.</summary>
    public void Clear() => _snapshots.Clear();
}
