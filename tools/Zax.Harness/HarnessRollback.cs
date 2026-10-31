using Zax.Render;

namespace Zax.Harness;

/// <summary>
/// One entry of the rollback history a <c>render --rom</c> run keeps (AMD-042 REQ-LOOP-069): the
/// core state, the 64 <c>PData</c> values and the camera state, all taken at the same instant.
/// </summary>
/// <param name="Core">
/// The complete core state; an in-memory one is enough (REQ-CORE-121), so the ROM image is left
/// out.
/// </param>
/// <param name="PersistentData">
/// The 256-byte block of the 64 <c>PData</c> entries, read the way <c>GetPData(i)</c> reads them
/// (SPEC-05 §4.6).
/// </param>
/// <param name="Camera">
/// The camera state a savestate carries, REQ-SCN-024 being its single reference.
/// </param>
internal sealed record HarnessRollbackSnapshot(
    byte[] Core,
    byte[] PersistentData,
    CameraStateData Camera);

/// <summary>
/// The rollback history of SPEC-07 §3.4 for the headless harness: a snapshot every 60 emulated
/// frames, at most 30 of them, and a restore that goes back to the second-to-last one.
/// </summary>
/// <remarks>
/// <para>
/// It is a copy of the host's <c>Zax.App.Loop.RollbackHistory</c>, down to the two constants and
/// to the step of REQ-LOOP-032: the harness deliberately does not reference Zax.App
/// (SPEC-01 REQ-ARCH-002), so the few views it needs on the loop are built here rather than
/// reused — the same arrangement <see cref="ScriptBridge"/> makes for the scene. Any change to
/// the depth, the spacing or the step belongs to both.
/// </para>
/// <para>
/// A run only builds the history when <c>--rollback-at</c> asks for it: nothing observable
/// depends on it otherwise, and a savestate every 60 frames is not free.
/// </para>
/// </remarks>
internal sealed class HarnessRollbackHistory
{
    /// <summary>Highest number of snapshots kept (REQ-LOOP-030).</summary>
    public const int Capacity = 30;

    /// <summary>A snapshot is taken on every frame counter that is a multiple of 60.</summary>
    public const int IntervalFrames = 60;

    private readonly List<HarnessRollbackSnapshot> _snapshots = [];

    /// <summary>Snapshots currently held, oldest first.</summary>
    public int Count => _snapshots.Count;

    /// <summary>
    /// Whether the frame about to run deserves a snapshot: its core frame counter is a multiple
    /// of 60, so the snapshot is taken <b>before</b> the frame executes.
    /// </summary>
    public static bool IsSnapshotFrame(long coreFrameCount)
        => coreFrameCount >= 0 && coreFrameCount % IntervalFrames == 0;

    /// <summary>Appends a snapshot, dropping the oldest one when the history is full.</summary>
    public void Capture(HarnessRollbackSnapshot snapshot)
    {
        ArgumentNullException.ThrowIfNull(snapshot);

        if (_snapshots.Count == Capacity)
        {
            _snapshots.RemoveAt(0);
        }

        _snapshots.Add(snapshot);
    }

    /// <summary>
    /// Takes the snapshot a rollback restores and removes it from the history together with the
    /// last one (REQ-LOOP-032 step 2).
    /// </summary>
    /// <remarks>
    /// With a single snapshot, that snapshot is restored and removed; with none, nothing is
    /// restored. AMD-042 §3 keeps this step as the reference: 3dSen 2.0 goes back one snapshot
    /// further on the first press, and that change is not taken up.
    /// </remarks>
    /// <returns>The snapshot to restore, or null when the history is empty.</returns>
    public HarnessRollbackSnapshot? TakeRollbackTarget()
    {
        if (_snapshots.Count == 0)
        {
            return null;
        }

        if (_snapshots.Count == 1)
        {
            HarnessRollbackSnapshot only = _snapshots[0];
            _snapshots.Clear();
            return only;
        }

        HarnessRollbackSnapshot target = _snapshots[^2];
        _snapshots.RemoveAt(_snapshots.Count - 1);
        _snapshots.RemoveAt(_snapshots.Count - 1);
        return target;
    }

    /// <summary>Drops every snapshot, which a reset does (REQ-LOOP-031).</summary>
    public void Clear() => _snapshots.Clear();
}
