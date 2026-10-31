namespace Zax.Harness;

/// <summary>
/// What a <c>render --rom</c> run does besides replaying frames: the two mid-run commands of the
/// application — the soft reset of AMD-044 and the rollback of AMD-042 — and the per-frame log.
/// </summary>
/// <remarks>
/// <para>
/// Both commands are placed "after real frame <c>n</c>": the frame numbered <c>n</c> runs and is
/// published, then the command is applied, before real frame <c>n + 1</c> starts. The numbering
/// is the harness's own count of real frames from the power-on, the very one <c>--press</c> uses
/// — never the core's frame counter, which both commands change (REQ-CORE-122 puts it back to 0,
/// a rollback winds it back with the rest of the core state).
/// </para>
/// <para>
/// A command therefore needs a real frame after it, otherwise the capture would have no
/// published frame left to show: the converted frames in flight are dropped by both, as a reset
/// and a state load do (REQ-E3D-281).
/// </para>
/// </remarks>
internal sealed record RomRunControl
{
    /// <summary><c>--soft-reset-at</c>, or null.</summary>
    public int? SoftResetAt { get; init; }

    /// <summary><c>--rollback-at</c>, or null.</summary>
    public int? RollbackAt { get; init; }

    /// <summary><c>--frame-log</c>, the file the log is written to, or null.</summary>
    public string? FrameLogPath { get; init; }

    /// <summary>Whether the run has to keep the rollback history of SPEC-07 §3.4.</summary>
    public bool KeepsHistory => RollbackAt is not null;

    /// <summary>Whether anything at all was asked for.</summary>
    public bool IsEmpty => SoftResetAt is null && RollbackAt is null && FrameLogPath is null;

    /// <summary>
    /// Checks the two frame numbers against the length of the run: a command has to name a real
    /// frame of the run and leave at least one after it.
    /// </summary>
    /// <param name="runFrames">The <c>--run</c> value, the number of real frames.</param>
    /// <exception cref="ArgumentException">A number is out of range.</exception>
    public void Validate(int runFrames)
    {
        Check(SoftResetAt, "--soft-reset-at");
        Check(RollbackAt, "--rollback-at");

        void Check(int? at, string option)
        {
            if (at is not { } frame)
            {
                return;
            }

            int last = Math.Max(1, runFrames) - 1;
            if (frame < 0 || frame >= last)
            {
                throw new ArgumentException(
                    $"{option} expects a real frame between 0 and {last - 1}: the frame it names "
                        + $"must run and at least one real frame must follow it, so that the "
                        + $"capture still has a converted frame to show.");
            }
        }
    }
}
