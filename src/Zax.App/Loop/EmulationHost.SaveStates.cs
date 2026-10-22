using Zax.App.Audio;
using Zax.App.Persistence;
using Zax.Engine3D;
using Zax.Nes;
using Zax.Render.Assets;

namespace Zax.App.Loop;

/// <summary>
/// The run-time save states of SPEC-07 §3.5: what a state holds (REQ-LOOP-040), writing to a
/// numbered slot (REQ-LOOP-042), the quick slot (REQ-LOOP-043, REQ-LOOP-045) and the size bound
/// of SPEC-01 REQ-ARCH-013.
/// </summary>
internal sealed partial class EmulationHost
{
    /// <summary>
    /// Upper bound on the size of a savestate file for the ROM and the profile currently
    /// loaded, announced before serialisation and valid for the whole session
    /// (REQ-ARCH-013, REQ-PERS-056).
    /// </summary>
    public int MaxSaveStateSize => _session is { HasRom: true } session
        ? SaveStateFile.MaxFileSize(session.Core.MaxStateSize())
        : 0;

    /// <summary>
    /// Captures the four sections of REQ-LOOP-040, in that logical order: the core, the Lua
    /// persistent data, the camera and the learned conversion state.
    /// </summary>
    /// <remarks>
    /// The <c>Save</c> hook of every active script runs <b>before</b> the capture
    /// (REQ-LOOP-041, REQ-LUA-071), so a script that writes a <c>PData</c> entry from its hook
    /// sees that value in the file.
    /// </remarks>
    public SaveStateContent? CaptureSaveState(string slotId)
    {
        if (_session is not { HasRom: true } session || _currentGame is not { } game)
        {
            return null;
        }

        byte[] lua = session.CaptureScriptState();
        byte[] core = session.Core.SaveState(includeRomImage: true);
        byte[] camera = SaveStateFile.WriteCamera(_scene.Camera.Capture());
        byte[] conversion = EngineStateCodec.Write(session.Engine);

        SaveStateHeader header = new(
            NesCore.CoreName,
            NesCore.StateFormatVersion,
            game.Id,
            CurrentRomFingerprint() ?? string.Empty,
            _now().ToUnixTimeSeconds(),
            slotId);

        return new SaveStateContent(header, core, lua, camera, conversion);
    }

    /// <summary>
    /// Writes a numbered slot (REQ-LOOP-042): the confirmation sound, the slot metadata — game,
    /// thumbnail, timestamp — persisted immediately, then the state file. An occupied slot is
    /// overwritten with no confirmation, and the emulation state does not change
    /// (REQ-LOOP-041).
    /// </summary>
    /// <param name="slotId">A numbered slot or the quick slot.</param>
    /// <param name="thumbnail">
    /// Whether a thumbnail is written. The quick save writes none (REQ-LOOP-043).
    /// </param>
    /// <returns>True when the state was written.</returns>
    public bool SaveToSlot(string slotId, bool thumbnail = true)
    {
        // REQ-LOOP-041: a save only happens outside Stopped and with a current game.
        if (State == EmulationState.Stopped || _session is not { HasRom: true } || IsLoading)
        {
            return false;
        }

        if (!SaveSlotIndex.IsValidSlotId(slotId) || Slots.Find(slotId) is null)
        {
            return false;
        }

        if (!_paths.IsWritable)
        {
            // REQ-PERS-073: a write action on a read-only root shows the same notification.
            Notifications.ShowError(_paths.UnwritableMessage ?? "User data folder is not writable.");
            return false;
        }

        _mixer?.PlaySound(UiSound.Confirm);

        if (CaptureSaveState(slotId) is not { } content)
        {
            return false;
        }

        byte[]? picture = thumbnail ? ThumbnailForSlot() : null;

        try
        {
            Slots.Occupy(slotId, content.Header.GameId, content.Header.SavedAt, picture is not null);
            Slots.Save(_paths);

            byte[] file = SaveStateFile.Write(content);
            if (file.Length > SaveStateFile.MaxFileSize(_session.Core.MaxStateSize()))
            {
                // REQ-ARCH-013: the announced bound must hold for the whole session. Going past
                // it is a bug in the bound, not a reason to lose the user's state, so it is
                // logged and the state is still written.
                _log($"savestate of slot {slotId} is larger than the announced bound.");
            }

            UserDataPaths.WriteAtomic(_paths.SlotStateFile(slotId), file);

            string thumbnailFile = _paths.SlotThumbnailFile(slotId);
            if (picture is not null)
            {
                PngImage.WriteRgb(thumbnailFile, ThumbnailWidth, ThumbnailHeight, picture);
            }
            else if (File.Exists(thumbnailFile))
            {
                // A quick save has no thumbnail (REQ-LOOP-043): the previous one must not stay
                // behind and be shown for a different state.
                File.Delete(thumbnailFile);
            }

            return true;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            Slots.Vacate(slotId);
            Slots.Save(_paths);
            Notifications.ShowError(exception.Message);
            return false;
        }
    }

    /// <summary>Width of a slot thumbnail (REQ-PERS-051).</summary>
    public const int ThumbnailWidth = 256;

    /// <summary>Height of a slot thumbnail (REQ-PERS-051).</summary>
    public const int ThumbnailHeight = 192;

    /// <summary>
    /// The quick save of REQ-LOOP-043: the quick slot, no thumbnail, and the notification
    /// <c>Quick Save</c> for 4 s. Nothing happens without a current game.
    /// </summary>
    public void QuickSave()
    {
        if (_currentGame is null || State == EmulationState.Stopped)
        {
            return;
        }

        if (SaveToSlot(SaveSlotIndex.QuickSlotId, thumbnail: false))
        {
            Notifications.Show("Quick Save", NotificationCenter.DefaultSeconds);
        }
    }

    /// <summary>
    /// The quick load of REQ-LOOP-045: loads the quick slot and shows <c>Quick Load</c> for
    /// 4 s. An empty quick slot does nothing at all — no notification, no sound.
    /// </summary>
    public void QuickLoad()
    {
        if (Slots.Find(SaveSlotIndex.QuickSlotId) is not { IsOccupied: true })
        {
            return;
        }

        LoadSlot(SaveSlotIndex.QuickSlotId);
        if (IsLoading)
        {
            Notifications.Show("Quick Load", NotificationCenter.DefaultSeconds);
        }
    }

    /// <summary>
    /// Asks for the confirmation of SPEC-09 REQ-PERS-054 and SPEC-10 REQ-UI-052 —
    /// <c>Do you want to delete the slot &lt;n&gt; ?</c>, titled with the game of the slot — then
    /// empties the slot. With no frontend to ask, the slot is emptied at once.
    /// </summary>
    /// <returns>False when the slot is empty or its identifier invalid.</returns>
    public bool RequestDeleteSlot(string slotId)
    {
        if (!SaveSlotIndex.IsValidSlotId(slotId) || Slots.Find(slotId) is not { IsOccupied: true } slot)
        {
            return false;
        }

        return Confirm(
            new ConfirmationRequest(
                _games.Title(slot.GameId) ?? slot.GameId,
                DeleteSlotQuestion(slotId),
                YesLabel,
                NoLabel,
                () => DeleteSlot(slotId)));
    }

    /// <summary>Empties a slot and deletes its files (SPEC-09 REQ-PERS-054).</summary>
    public void DeleteSlot(string slotId)
    {
        if (!SaveSlotIndex.IsValidSlotId(slotId))
        {
            return;
        }

        SaveSlotIndex.DeleteFiles(_paths, slotId);
        Slots.Vacate(slotId);
        Slots.Save(_paths);
    }

    /// <summary>
    /// The thumbnail a slot save uses: the capture taken when the game was paused
    /// (REQ-LOOP-006). While the game is running there is no pause capture, so the current view
    /// is captured instead, which DEC-07-Q5 allows — see <c>spec/changes/Q-027.md</c>.
    /// </summary>
    // Q-027
    private byte[]? ThumbnailForSlot()
    {
        if (State == EmulationState.Paused && _pauseThumbnail is { } paused)
        {
            return paused;
        }

        return _options.CaptureThumbnail?.Invoke() ?? _pauseThumbnail;
    }

    /// <summary>
    /// The rollback of REQ-LOOP-032: the notification, the second-to-last snapshot restored
    /// into the core, and the state forced to Running with the clock reset.
    /// </summary>
    private void PerformRollback()
    {
        if (_session is not { HasRom: true } session || State == EmulationState.Stopped)
        {
            return;
        }

        Notifications.Show("Roll Back", NotificationCenter.ShortSeconds);

        if (Rollback.TakeRollbackTarget() is { } snapshot)
        {
            // DEC-07-Q3: the core alone. Neither the camera nor the scripts' persistent data
            // are restored, and no Save or Load hook runs (REQ-LUA-074).
            if (!session.RestoreCore(snapshot, out string? error))
            {
                _log($"rollback snapshot refused: {error}");
            }
        }

        // Step 3 applies even when nothing was restored (REQ-LOOP-032, notes).
        SetState(EmulationState.Running);
        Pacer.Reset();
    }
}
