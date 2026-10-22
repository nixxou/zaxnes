namespace Zax.App.Ui;

/// <summary>Which interface window is open, and what full screen hides (AMD-010 REQ-UI-209).</summary>
/// <remarks>
/// <para>
/// REQ-UI-209 draws the line between two families: the <b>chrome</b> — the menu bar, the welcome
/// screen, the statistics overlay — which full screen hides altogether, and the <b>windows</b> —
/// the options, the key configuration, the about block and the two dialogs — which a shortcut may
/// open over a full-screen view and which therefore stay visible. The OSD of REQ-UI-206 is in
/// neither family: it is always drawn.
/// </para>
/// <para>
/// This is a plain state object on purpose: the test of REQ-UI-209 asks whether the menu bar is
/// visible, which is a question about this state and not about a rendered frame.
/// </para>
/// </remarks>
internal sealed class UiState
{
    /// <summary>Whether the window is in full screen (REQ-UI-209, setting <c>ui.fullscreen</c>).</summary>
    public bool Fullscreen { get; set; }

    /// <summary>Whether a game is loaded, which hides the welcome screen (REQ-UI-207).</summary>
    public bool HasCurrentGame { get; set; }

    /// <summary>Whether the options window is open (REQ-UI-202).</summary>
    public bool Options { get; set; }

    /// <summary>Whether the key configuration window is open (REQ-UI-203).</summary>
    public bool KeyConfig { get; set; }

    /// <summary>Whether the about window is open (DEC-10-Q3).</summary>
    public bool About { get; set; }

    /// <summary>Whether the profile chooser is open (REQ-UI-201).</summary>
    public bool ProfileChooser { get; set; }

    /// <summary>Whether the file dialog is open (REQ-UI-201).</summary>
    public bool FileBrowser { get; set; }

    /// <summary>Whether the confirmation box E8 is open (REQ-UI-110).</summary>
    public bool Dialog { get; set; }

    /// <summary>
    /// Whether the developer statistics panel is drawn. Off by default: SPEC-10 REQ-UI-095 and
    /// DEC-10-Q7 allow the panel as an option and require it to start hidden.
    /// </summary>
    public bool DebugOverlay { get; set; }

    /// <summary>The menu bar of REQ-UI-200, hidden in full screen (REQ-UI-209).</summary>
    public bool MenuBarVisible => !Fullscreen;

    /// <summary>The welcome screen of REQ-UI-207: no game loaded, and not in full screen.</summary>
    public bool WelcomeVisible => !HasCurrentGame && !Fullscreen;

    /// <summary>The statistics panel: only when asked for, and never in full screen.</summary>
    public bool DebugOverlayVisible => DebugOverlay && !Fullscreen;

    /// <summary>
    /// Whether a modal window is up, which stops the menu bar from being used and takes the
    /// Menu button before the emulation does (REQ-UI-004 cases 2 and 3). The loop reads it
    /// through <c>EmulationHostOptions.ModalWindowOpen</c>.
    /// </summary>
    public bool IsModal => FileBrowser || ProfileChooser || Dialog;

    /// <summary>
    /// Whether any window is open. A full-screen view with one of them open still draws it
    /// (REQ-UI-209).
    /// </summary>
    public bool AnyWindowOpen
        => Options || KeyConfig || About || ProfileChooser || FileBrowser || Dialog;

    /// <summary>Closes every window, which the Escape rule and <c>ui.close_windows</c> do.</summary>
    public void CloseAll()
    {
        Options = false;
        KeyConfig = false;
        About = false;
        ProfileChooser = false;
        FileBrowser = false;
        Dialog = false;
    }
}
