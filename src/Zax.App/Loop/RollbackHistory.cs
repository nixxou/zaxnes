using System.Numerics;
using Zax.Render;

namespace Zax.App.Loop;

/// <summary>
/// One entry of the rollback history (AMD-042 REQ-LOOP-069, with the fourth element AMD-063
/// REQ-LUA-119 adds): the core state, the 64 <c>PData</c> values, the camera state and the
/// forced camera rotation, all taken at the same instant.
/// </summary>
/// <param name="Core">
/// The complete core state; an in-memory one is enough (REQ-CORE-121), so the ROM image is
/// left out.
/// </param>
/// <param name="PersistentData">
/// The 256-byte block of the 64 <c>PData</c> entries, read the way <c>GetPData(i)</c> reads
/// them (SPEC-05 §4.6).
/// </param>
/// <param name="Camera">
/// The camera state a savestate carries, REQ-SCN-024 being its single reference.
/// </param>
/// <param name="RotOverride">
/// The value of <c>CS.Nes3D.CameraControl.RotOverride</c> (AMD-063 REQ-LUA-119). It is a session
/// value, so it is not in <paramref name="Camera"/> — REQ-SCN-024 is unchanged — and it is
/// carried here on its own.
/// </param>
public sealed record RollbackSnapshot(
    byte[] Core,
    byte[] PersistentData,
    CameraStateData Camera,
    Vector3? RotOverride = null);

/// <summary>
/// The rollback history of SPEC-07 §3.4: a snapshot every 60 emulated frames, at most 30 of
/// them, and a restore that goes back to the second-to-last one.
/// </summary>
/// <remarks>
/// <para>
/// A snapshot holds the core, the scripts' 64 <c>PData</c> values, the camera state and the
/// forced camera rotation (AMD-042 REQ-LOOP-069, which replaces the last sentence of
/// REQ-LOOP-030's notes, with the fourth element of AMD-063 REQ-LUA-119). It still holds neither
/// the learned conversion state, nor the converted frames, nor the meshes. The depth is
/// therefore about 30 s in NTSC and 36 s in PAL/Dendy.
/// </para>
/// <para>
/// The history is never persisted (SPEC-09 REQ-PERS-058). It is cleared when a game is
/// launched, when a savestate is loaded, on every reset — the soft one of AMD-044 included —
/// and at the <b>end</b> of a game (AMD-042 REQ-LOOP-071); it is <b>not</b> cleared by a
/// pause, a quick save or a rollback (REQ-LOOP-031).
/// </para>
/// </remarks>
public sealed class RollbackHistory
{
    /// <summary>Highest number of snapshots kept (REQ-LOOP-030).</summary>
    public const int Capacity = 30;

    /// <summary>A snapshot is taken on every frame counter that is a multiple of 60.</summary>
    public const int IntervalFrames = 60;

    private readonly List<RollbackSnapshot> _snapshots = [];

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
    public void Capture(RollbackSnapshot snapshot)
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
    /// restored, but the caller still applies step 3 (go to Running, reset the clock). The
    /// original misbehaves in both cases and is deliberately not reproduced
    /// (REQ-LOOP-032, notes).
    /// </remarks>
    /// <returns>The snapshot to restore, or null when the history is empty.</returns>
    public RollbackSnapshot? TakeRollbackTarget()
    {
        if (_snapshots.Count == 0)
        {
            return null;
        }

        if (_snapshots.Count == 1)
        {
            RollbackSnapshot only = _snapshots[0];
            _snapshots.Clear();
            return only;
        }

        RollbackSnapshot target = _snapshots[^2];
        _snapshots.RemoveAt(_snapshots.Count - 1);
        _snapshots.RemoveAt(_snapshots.Count - 1);
        return target;
    }

    /// <summary>
    /// The optional history panel of REQ-LOOP-033: restoring entry <paramref name="index"/>
    /// (0 = oldest) keeps only the entries before it.
    /// </summary>
    /// <returns>The snapshot to restore, or null when the index is out of range.</returns>
    public RollbackSnapshot? TakeAt(int index)
    {
        if (index < 0 || index >= _snapshots.Count)
        {
            return null;
        }

        RollbackSnapshot target = _snapshots[index];
        _snapshots.RemoveRange(index, _snapshots.Count - index);
        return target;
    }

    /// <summary>Drops every snapshot.</summary>
    public void Clear() => _snapshots.Clear();
}
