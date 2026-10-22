namespace Zax.App.Ui;

/// <summary>
/// The stable names of the commands the interface itself owns, registered in the same model as
/// <see cref="Loop.ZaxCommands"/> so that a menu entry, a shortcut and a gamepad binding reach
/// them the same way (AMD-010 REQ-UI-205, REQ-UI-203).
/// </summary>
/// <remarks>
/// These are additions, not requirements of REQ-UI-200: the amendment's menu table has no debug
/// overlay and no welcome screen entry. They live apart from <c>ZaxCommands</c> precisely so that
/// the test which enumerates the amendment's commands stays exact.
/// </remarks>
internal static class UiCommands
{
    /// <summary>Toggle the developer statistics overlay, off by default.</summary>
    public const string ToggleDebugOverlay = "ui.toggle_debug_overlay";

    /// <summary>Close every interface window that is open.</summary>
    public const string CloseWindows = "ui.close_windows";
}
