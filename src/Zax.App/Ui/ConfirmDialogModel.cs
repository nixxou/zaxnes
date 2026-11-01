using Zax.App.Loop;

namespace Zax.App.Ui;

/// <summary>
/// The confirmation box E8 of SPEC-10 REQ-UI-110, as a UI-independent model: a title, a message
/// and a row of buttons, the first of which acts.
/// </summary>
/// <remarks>
/// <para>
/// REQ-UI-110 makes the box modal and centred, gives the focus to the first button, has every
/// button run its action and then close, and offers no close cross — one leaves it by its
/// buttons only. While it is up nothing else is clickable and the Menu button has no effect
/// (REQ-UI-004 case 2), which <see cref="UiState.IsModal"/> carries to the loop.
/// </para>
/// <para>
/// The texts of a <see cref="ConfirmationRequest"/> are never written here: they come with the
/// request the loop raises, so the exact wording of SPEC-09 REQ-PERS-042 and REQ-PERS-054 has a
/// single source. That wording is the English entry of the table, which AMD-036 §1 has
/// translated as it is read — hence the properties below rather than stored strings.
/// </para>
/// <para>
/// AMD-056 REQ-UI-214 adds the import dialogs to the same box
/// (<see cref="OpenImportDialog"/>). Two things differ there: one of them has a <b>single</b>
/// button, and their texts arrive already in the language in force, built by
/// <see cref="ProfileImportDialogs.Decide"/> out of the text table. They are therefore stored
/// as they come and not translated a second time.
/// </para>
/// </remarks>
internal sealed class ConfirmDialogModel
{
    private Action? _confirmed;
    private string _message = string.Empty;
    private string _confirmLabel = EmulationHost.YesLabel;
    private string? _cancelLabel = EmulationHost.NoLabel;
    private bool _translate = true;

    /// <summary>Whether the box is up.</summary>
    public bool IsOpen { get; private set; }

    /// <summary>Title of the box; the game's title for the deletions of SPEC-09.</summary>
    public string Title { get; private set; } = string.Empty;

    /// <summary>The message, in the language in force (AMD-036 §1).</summary>
    public string Message => _translate ? UiNotifications.Translate(_message) : _message;

    /// <summary>Label of the button that acts; it has the focus (REQ-UI-110).</summary>
    public string ConfirmLabel => Say(_confirmLabel);

    /// <summary>
    /// Label of the button that changes nothing, or the empty string when the box has only one
    /// button — the two one-button rows of AMD-056 REQ-UI-214.
    /// </summary>
    public string CancelLabel => _cancelLabel is null ? string.Empty : Say(_cancelLabel);

    /// <summary>Whether a second button is drawn at all.</summary>
    public bool HasCancel => _cancelLabel is not null;

    /// <summary>
    /// Whether the first button destroys something. Only the replacement confirmation of
    /// REQ-UI-214 does; its presentation stays free (AMD-056 §3, AMD-010 §4).
    /// </summary>
    public bool Destructive { get; private set; }

    /// <summary>Opens the box on <paramref name="request"/>; a second call replaces it.</summary>
    public void Open(ConfirmationRequest request)
    {
        ArgumentNullException.ThrowIfNull(request);

        Title = request.Title;
        _message = request.Message;
        _confirmLabel = request.ConfirmLabel;
        _cancelLabel = request.CancelLabel;
        _confirmed = request.Confirmed;
        _translate = true;
        Destructive = false;
        IsOpen = true;
    }

    /// <summary>
    /// Opens the box on one of the import dialogs of AMD-056 REQ-UI-214, already worded in the
    /// language in force.
    /// </summary>
    /// <param name="dialog">The dialog <see cref="ProfileImportDialogs.Decide"/> chose.</param>
    /// <param name="confirmed">
    /// What the first button runs — the replacement, for the only row that has anything to run.
    /// The two one-button rows pass nothing: their button only closes.
    /// </param>
    public void OpenImportDialog(ProfileImportDialog dialog, Action? confirmed = null)
    {
        ArgumentNullException.ThrowIfNull(dialog);

        Title = dialog.Title;
        _message = dialog.Body;
        _confirmLabel = dialog.PrimaryLabel;
        _cancelLabel = dialog.CancelLabel;
        _confirmed = confirmed;
        _translate = false;
        Destructive = dialog.Destructive;
        IsOpen = true;
    }

    /// <summary>Runs the action and closes the box, which the first button does.</summary>
    public void Confirm()
    {
        Action? action = _confirmed;
        Close();
        action?.Invoke();
    }

    /// <summary>Closes the box without doing anything, which the second button does.</summary>
    public void Close()
    {
        IsOpen = false;
        _confirmed = null;
    }

    private string Say(string text) => _translate ? UiNotifications.Translate(text) : text;
}
