using Zax.App.Persistence;

namespace Zax.App.Loop;

/// <summary>
/// A confirmation the loop asks the frontend for: the dialogue box E8 of SPEC-10 REQ-UI-110.
/// </summary>
/// <param name="Title">Title of the box; the game's title for the deletions of SPEC-09.</param>
/// <param name="Message">The exact message the requirement fixes.</param>
/// <param name="ConfirmLabel">Label of the button that acts, <c>Yes</c>.</param>
/// <param name="CancelLabel">Label of the button that changes nothing, <c>No</c>.</param>
/// <param name="Confirmed">What <paramref name="ConfirmLabel"/> runs.</param>
internal sealed record ConfirmationRequest(
    string Title,
    string Message,
    string ConfirmLabel,
    string CancelLabel,
    Action Confirmed);

/// <summary>
/// The library operations the loop owns: deleting a user game with its save states
/// (SPEC-09 REQ-PERS-042, SPEC-10 REQ-UI-037) and the cover art of REQ-PERS-036.
/// </summary>
internal sealed partial class EmulationHost
{
    /// <summary>
    /// The confirmation text of SPEC-09 REQ-PERS-042 and SPEC-10 REQ-UI-037, with its spacing
    /// before the question mark.
    /// </summary>
    public const string DeleteGameQuestion = "Do you want to delete the game and its save states ?";

    /// <summary>Labels of the two buttons of the deletions (REQ-UI-037, REQ-UI-052).</summary>
    public const string YesLabel = "Yes";

    /// <summary>Label of the button that changes nothing.</summary>
    public const string NoLabel = "No";

    /// <summary>The confirmation text of SPEC-09 REQ-PERS-054 and SPEC-10 REQ-UI-052.</summary>
    public static string DeleteSlotQuestion(string slotId) => $"Do you want to delete the slot {slotId} ?";

    /// <summary>
    /// Deletes a user game, after confirmation when the frontend offers one (REQ-PERS-042): the
    /// emulation stops if this is the current game, every slot of that game is emptied — the
    /// quick slot included — and the profile file, the cover art and the library entry go.
    /// </summary>
    /// <param name="gameId">The entry to delete; a built-in profile is refused.</param>
    /// <returns>
    /// True when the deletion happened or a confirmation was opened for it; false when the entry
    /// cannot be deleted at all.
    /// </returns>
    public bool RequestDeleteGame(string gameId)
    {
        if (string.IsNullOrEmpty(gameId) || IsLoading || !_games.CanDelete(gameId))
        {
            return false;
        }

        string title = _games.Title(gameId) ?? gameId;
        return Confirm(
            new ConfirmationRequest(
                title,
                DeleteGameQuestion,
                YesLabel,
                NoLabel,
                () => DeleteGame(gameId)));
    }

    /// <summary>
    /// The deletion itself, with no confirmation: the five erasures of REQ-PERS-042 in order.
    /// </summary>
    public bool DeleteGame(string gameId)
    {
        if (string.IsNullOrEmpty(gameId) || !_games.CanDelete(gameId))
        {
            return false;
        }

        // The title is read before the entry goes, so the notification can name the game.
        string title = _games.Title(gameId) ?? gameId;

        // 1. The emulation stops when the game being deleted is the one playing.
        if (string.Equals(_currentGame?.Id, gameId, StringComparison.Ordinal))
        {
            StopCurrentGame(clearCurrentGame: true);
        }

        // 2. Every slot of that game, the quick slot included.
        foreach (SaveSlotRecord slot in Slots.OccupiedFor(gameId))
        {
            DeleteSlot(slot.Id);
        }

        // 3 to 5. The profile file, the cover art and the library entry, then library.json.
        if (!_games.DeleteGame(gameId))
        {
            return false;
        }

        Notifications.Show($"« {title} » supprimé.");
        return true;
    }

    /// <summary>
    /// « Create game art » of REQ-PERS-036: the pause capture of the current game becomes its
    /// cover. Does nothing without a capture or without a current game.
    /// </summary>
    public bool CreateGameArt()
    {
        if (_currentGame is not { } game || ThumbnailForSlot() is not { } capture)
        {
            return false;
        }

        return StoreCover(
            game.Id,
            GameArt.SetCoverFromCapture(_paths, _games.Title(game.Id) ?? game.Title, game.Id, capture, out string? failure),
            failure);
    }

    /// <summary>
    /// Imports an image file as the cover of the current game (REQ-PERS-036): shrunk to fit
    /// 256 × 192 with its proportions kept, written without alpha, named deterministically
    /// (REQ-PERS-074).
    /// </summary>
    public bool ImportCover(string path)
    {
        if (_currentGame is not { } game || string.IsNullOrEmpty(path))
        {
            return false;
        }

        return StoreCover(
            game.Id,
            GameArt.ImportCover(_paths, _games.Title(game.Id) ?? game.Title, game.Id, path, out string? failure),
            failure);
    }

    /// <summary>
    /// The automatic refresh of REQ-PERS-036 for the "2D Mode" entry: its cover follows the
    /// pause capture every time the game is paused, which is when the menu appears.
    /// </summary>
    private void RefreshTwoDimensionalCover()
    {
        if (_currentGame is not { Is2DMode: true } game || _pauseThumbnail is not { } capture)
        {
            return;
        }

        StoreCover(
            game.Id,
            GameArt.SetCoverFromCapture(_paths, _games.Title(game.Id) ?? game.Title, game.Id, capture, out string? failure),
            failure,
            quiet: true);
    }

    private bool StoreCover(string gameId, string? fileName, string? failure, bool quiet = false)
    {
        if (fileName is null)
        {
            if (!quiet && failure is { Length: > 0 })
            {
                Notifications.ShowError(failure);
            }

            return false;
        }

        _games.SetCover(gameId, fileName);
        return true;
    }

    /// <summary>
    /// Hands <paramref name="request"/> to the frontend, or runs it straight away when there is
    /// no frontend to ask — a headless host, and the command-line paths.
    /// </summary>
    private bool Confirm(ConfirmationRequest request)
    {
        if (_options.Confirm is { } ask && ask(request))
        {
            return true;
        }

        request.Confirmed();
        return true;
    }
}
