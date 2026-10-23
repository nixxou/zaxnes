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
/// The texts are never written here: they come with the <see cref="ConfirmationRequest"/> the
/// loop raises, so the exact wording of SPEC-09 REQ-PERS-042 and REQ-PERS-054 has a single
/// source. That wording is the English entry of the table, which AMD-036 §1 has translated as
/// it is read — hence the three properties below rather than three stored strings.
/// </para>
/// </remarks>
internal sealed class ConfirmDialogModel
{
    private Action? _confirmed;
    private string _message = string.Empty;
    private string _confirmLabel = EmulationHost.YesLabel;
    private string _cancelLabel = EmulationHost.NoLabel;

    /// <summary>Whether the box is up.</summary>
    public bool IsOpen { get; private set; }

    /// <summary>Title of the box; the game's title for the deletions of SPEC-09.</summary>
    public string Title { get; private set; } = string.Empty;

    /// <summary>The message, in the language in force (AMD-036 §1).</summary>
    public string Message => UiNotifications.Translate(_message);

    /// <summary>Label of the button that acts; it has the focus (REQ-UI-110).</summary>
    public string ConfirmLabel => UiNotifications.Translate(_confirmLabel);

    /// <summary>Label of the button that changes nothing.</summary>
    public string CancelLabel => UiNotifications.Translate(_cancelLabel);

    /// <summary>Opens the box on <paramref name="request"/>; a second call replaces it.</summary>
    public void Open(ConfirmationRequest request)
    {
        ArgumentNullException.ThrowIfNull(request);

        Title = request.Title;
        _message = request.Message;
        _confirmLabel = request.ConfirmLabel;
        _cancelLabel = request.CancelLabel;
        _confirmed = request.Confirmed;
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
}
