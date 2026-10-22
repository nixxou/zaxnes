namespace Zax.App.Ui;

/// <summary>
/// The capture plan behind <c>--ui-shots</c>: a list of interface states, each held for a few
/// frames so Dear ImGui can settle its auto-sized windows, then written out as a PNG.
/// </summary>
/// <remarks>
/// It exists for the verification artefacts of the round-1 report: the screenshots of the welcome
/// screen, of an open menu, of the options window, of the key configuration and of the profile
/// chooser. It drives nothing a user cannot reach from the menus — it only puts the interface
/// into each state without a mouse.
/// </remarks>
internal sealed class UiShots
{
    /// <summary>How many frames a state is held before it is captured.</summary>
    public const int SettleFrames = 4;

    private readonly List<(string Name, Action<ZaxUi> Arrange)> _plan =
    [
        ("accueil", static ui =>
        {
            ui.State.CloseAll();
            ui.ForceOpenMenu = null;
        }),
        ("menu-fichier", static ui =>
        {
            ui.State.CloseAll();
            ui.ForceOpenMenu = UiText.MenuFile;
        }),
        ("options", static ui =>
        {
            ui.State.CloseAll();
            ui.ForceOpenMenu = null;
            ui.State.Options = true;
        }),
        ("touches", static ui =>
        {
            ui.State.CloseAll();
            ui.State.KeyConfig = true;
        }),
        ("profil", static ui =>
        {
            ui.State.CloseAll();
            ui.OpenProfileChooserForCapture();
        }),
        ("a-propos", static ui =>
        {
            ui.State.CloseAll();
            ui.State.About = true;
        }),
    ];

    private readonly string _directory;
    private int _index;
    private int _frame;

    /// <summary>Creates the plan and the folder it writes into.</summary>
    public UiShots(string directory)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(directory);
        _directory = directory;
        Directory.CreateDirectory(directory);
    }

    /// <summary>Whether every state has been captured.</summary>
    public bool Finished => _index >= _plan.Count;

    /// <summary>
    /// One frame of the plan: arranges the state on its first frame, and asks
    /// <paramref name="capture"/> for the picture once it has settled.
    /// </summary>
    /// <param name="ui">The interface to arrange.</param>
    /// <param name="capture">Writes the window's picture to the path it is given.</param>
    public void Advance(ZaxUi ui, Action<string> capture)
    {
        ArgumentNullException.ThrowIfNull(ui);
        ArgumentNullException.ThrowIfNull(capture);

        if (Finished)
        {
            return;
        }

        (string name, Action<ZaxUi> arrange) = _plan[_index];

        if (_frame == 0)
        {
            arrange(ui);
        }

        _frame++;
        if (_frame <= SettleFrames)
        {
            return;
        }

        capture(Path.Combine(_directory, $"{name}.png"));
        _index++;
        _frame = 0;
    }

    /// <summary>How many frames the whole plan needs.</summary>
    public int TotalFrames => _plan.Count * (SettleFrames + 1);
}
