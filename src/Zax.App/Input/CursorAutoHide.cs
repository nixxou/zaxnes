namespace Zax.App.Input;

/// <summary>
/// The cursor masking of SPEC-08 §3.12: while a game runs, the system cursor disappears after
/// two seconds without mouse movement, and any movement brings it back (REQ-INP-110,
/// REQ-INP-111).
/// </summary>
/// <remarks>
/// The original checks every 0.1 s and hides the cursor at the twentieth consecutive check
/// without movement, which is the 2 s of the requirement with its ±0.15 s tolerance. The same
/// discrete clock is used here so that the timing is observable and testable, instead of a plain
/// "idle &gt;= 2 s" comparison.
/// </remarks>
public sealed class CursorAutoHide
{
    /// <summary>Interval between two checks, 0.1 s (REQ-INP-110).</summary>
    public const float CheckSeconds = 0.1f;

    /// <summary>How many consecutive still checks hide the cursor (REQ-INP-110).</summary>
    public const int ChecksBeforeHiding = 20;

    private float _sinceCheck;
    private bool _movedSinceCheck;

    /// <summary>How many consecutive checks found the mouse still.</summary>
    public int StillChecks { get; private set; }

    /// <summary>Whether the cursor should be visible right now.</summary>
    public bool CursorVisible { get; private set; } = true;

    /// <summary>
    /// Advances the timer by <paramref name="realSeconds"/> and returns whether the cursor
    /// should be visible.
    /// </summary>
    /// <param name="gameRunning">
    /// Whether the game is running. Outside a running game the cursor is always visible
    /// (REQ-INP-111).
    /// </param>
    /// <param name="pointerMoved">Whether the pointer moved since the previous call.</param>
    /// <param name="realSeconds">Real time since the previous call.</param>
    public bool Update(bool gameRunning, bool pointerMoved, float realSeconds)
    {
        if (!gameRunning)
        {
            // REQ-INP-111: visible at all times in the menu, in pause, in the library and while
            // loading, and the counter starts again from zero when the game resumes.
            Show();
            return CursorVisible;
        }

        if (pointerMoved)
        {
            // REQ-INP-110: any movement shows it again immediately and resets the counter.
            Show();
            _movedSinceCheck = true;
        }

        _sinceCheck += realSeconds;
        while (_sinceCheck >= CheckSeconds)
        {
            _sinceCheck -= CheckSeconds;

            if (_movedSinceCheck)
            {
                StillChecks = 0;
                _movedSinceCheck = false;
                continue;
            }

            StillChecks++;
            if (StillChecks >= ChecksBeforeHiding)
            {
                CursorVisible = false;
            }
        }

        return CursorVisible;
    }

    /// <summary>
    /// Shows the cursor and resets the counter, which closing the program or the window must
    /// also do (REQ-INP-111).
    /// </summary>
    public void Show()
    {
        CursorVisible = true;
        StillChecks = 0;
        _sinceCheck = 0f;
        _movedSinceCheck = false;
    }
}
