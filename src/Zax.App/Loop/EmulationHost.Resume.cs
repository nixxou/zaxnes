using Zax.App.Persistence;

namespace Zax.App.Loop;

/// <summary>
/// The resume point of AMD-053 REQ-LOOP-076: the savestate the normal exit writes into the
/// <c>resume</c> slot, the entry the next launch offers, and the three erasure rules.
/// </summary>
/// <remarks>
/// <para>
/// This is an accepted divergence <b>(D)</b>: the original program writes no savestate at all
/// when it closes. REQ-LOOP-052 keeps its promise all the same — no numbered slot and no quick
/// slot is ever written behind the user's back; the resume slot is a thirty-eighth location
/// that only this path writes and only <c>resume.last</c> reads (REQ-PERS-052 as §3 point 5
/// extends it).
/// </para>
/// <para>
/// <c>-noresume</c> — which <c>-slot</c> implies — suspends the whole of it for one run: the
/// slot is neither read, nor written, nor erased. The amendment states the first two and leaves
/// its first erasure rule in contradiction with the third (R2 point 53-N2); see
/// <c>spec/changes/Q-088.md</c>.
/// </para>
/// </remarks>
internal sealed partial class EmulationHost
{
    private bool _resumeWritten;

    /// <summary>
    /// What the start-up does with the resume slot (REQ-LOOP-076, « Au lancement suivant » and
    /// the three erasure rules).
    /// </summary>
    /// <param name="wasOccupied">
    /// Whether the slot was occupied in the file, before the consistency pass of REQ-PERS-055
    /// had a chance to empty it.
    /// </param>
    private void PrepareResumeSlot(bool wasOccupied)
    {
        if (_options.SuspendResume)
        {
            // Q-088: nothing is read and nothing is touched this run.
            ResumeAvailable = false;
            return;
        }

        // Rule 3: the file is validated whole, so a savestate truncated in its body is caught
        // here rather than at the moment the user activates the entry. An unreadable one is
        // erased, the start-up is normal, no notification is shown, and the fact is journalled.
        if (Slots.ValidateResumeSlot(_paths) is { } failure)
        {
            _log($"the resume point was discarded: {failure} (REQ-LOOP-076).");
            Slots.Save(_paths);
        }
        else if (wasOccupied && Slots.Find(SaveSlotIndex.ResumeSlotId) is { IsOccupied: false })
        {
            // The consistency pass emptied it: its game left the library, or its file is gone
            // (REQ-PERS-055 as §3 point 7 extends it).
            _log("the resume point was discarded: its game is no longer in the library (REQ-PERS-055).");
        }

        if (Slots.MigratedFromVersion is { } version)
        {
            _log($"saves/index.json was migrated from schema {version} to {SaveSlotIndex.SchemaVersion}.");
            Slots.Save(_paths);
        }

        ResumeAvailable = Slots.Find(SaveSlotIndex.ResumeSlotId) is { IsOccupied: true } slot
            && _games.Exists(slot.GameId);
    }

    /// <summary>
    /// The title the <c>Resume</c> entry carries: the game the resume slot holds, or null when
    /// the entry is not offered.
    /// </summary>
    public string? ResumeGameTitle
        => ResumeAvailable && Slots.Find(SaveSlotIndex.ResumeSlotId) is { IsOccupied: true } slot
            ? _games.Title(slot.GameId) ?? slot.GameId
            : null;

    /// <summary>
    /// The thumbnail file of the resume slot, or null when there is none to show
    /// (REQ-LOOP-076, « portant sa vignette »).
    /// </summary>
    public string? ResumeThumbnailFile
        => ResumeAvailable
            && Slots.Find(SaveSlotIndex.ResumeSlotId) is { IsOccupied: true, Thumbnail: not null }
            && File.Exists(_paths.SlotThumbnailFile(SaveSlotIndex.ResumeSlotId))
                ? _paths.SlotThumbnailFile(SaveSlotIndex.ResumeSlotId)
                : null;

    /// <summary>
    /// Loads the resume slot, exactly as a slot load does (REQ-LOOP-044). It is what the
    /// <c>resume.last</c> command runs, and the only path that ever reads that slot. A file that
    /// passed the start-up check and still fails here falls under REQ-LOOP-046 — the
    /// <c>Corrupted save state.</c> notification and the Stopped state — like any other slot.
    /// </summary>
    public void ResumeLastGame()
    {
        if (!ResumeAvailable)
        {
            return;
        }

        LoadSlot(SaveSlotIndex.ResumeSlotId);
    }

    /// <summary>
    /// Steps 1 and 2 of the exit (REQ-LOOP-076): the <c>Save</c> hook of every active script,
    /// then the capture and the write of the resume savestate — or the erasure of the slot when
    /// the state is Stopped.
    /// </summary>
    /// <returns>True when a savestate was written.</returns>
    /// <remarks>
    /// The clause of REQ-LOOP-041 asking for the write to happen « between two emulated frames »
    /// is satisfied by construction: no emulated frame runs any more at this point.
    /// </remarks>
    private bool WriteOrClearResumeSlot()
    {
        _resumeWritten = false;

        if (_options.SuspendResume)
        {
            // Q-088: -noresume suspends the erasure rule as well, otherwise launching a slot,
            // quitting from Stopped and losing the resume point would be the common case.
            return false;
        }

        // Rule 1, which wins over rule 2: quitting from Stopped erases the slot, so ignoring the
        // Resume entry and then quitting without playing does clear the resume point.
        if (State == EmulationState.Stopped)
        {
            ClearResumeSlot();
            return false;
        }

        // The condition of the requirement, in the vocabulary of SPEC-07: Running or Paused with
        // the loading indicator down. A loading exit writes nothing and erases nothing.
        if (IsLoading || _session is not { HasRom: true } || _currentGame is null)
        {
            return false;
        }

        // The capture taken at the last pause when there is one — it always belongs to the
        // current game, being cleared with it — and a capture of the game view otherwise, which
        // is the usual case of an exit made from Running.
        byte[]? picture = _pauseThumbnail ?? _options.CaptureThumbnail?.Invoke();

        _resumeWritten = WriteSlot(
            SaveSlotIndex.ResumeSlotId,
            thumbnail: picture is not null,
            picture: picture,
            sound: false);

        if (_resumeWritten)
        {
            _log($"resume point written for '{_currentGame.Id}' (REQ-LOOP-076).");
        }

        return _resumeWritten;
    }

    /// <summary>Empties the resume slot and deletes its files.</summary>
    private void ClearResumeSlot()
    {
        if (Slots.Find(SaveSlotIndex.ResumeSlotId) is not { IsOccupied: true })
        {
            // Nothing to erase; the files are removed all the same, in case one was left over.
            SaveSlotIndex.DeleteFiles(_paths, SaveSlotIndex.ResumeSlotId);
            return;
        }

        DeleteSlot(SaveSlotIndex.ResumeSlotId);
        _log("the resume point was cleared: the program exited with no game running (REQ-LOOP-076).");
    }
}
