namespace Zax.App.Input;

/// <summary>What the players' actions asked the interface for during one displayed frame.</summary>
[Flags]
public enum MenuNavigationEvents
{
    /// <summary>Nothing happened.</summary>
    None = 0,

    /// <summary>Previous tab or previous page (REQ-INP-105).</summary>
    Previous = 1 << 0,

    /// <summary>Next tab or next page.</summary>
    Next = 1 << 1,

    /// <summary>Previous entry, for a control laid out vertically.</summary>
    PreviousVertical = 1 << 2,

    /// <summary>Next entry, for a control laid out vertically.</summary>
    NextVertical = 1 << 3,

    /// <summary>Secondary action of the focused element (REQ-INP-105).</summary>
    Secondary = 1 << 4,
}

/// <summary>
/// The routing of the players' actions to the interface while the game is paused
/// (REQ-INP-105): rising edges on the directional actions walk the tabs and the list pages, and
/// <c>Select</c> triggers the secondary action of the focused element.
/// </summary>
/// <remarks>
/// <para>
/// Only the edge detection lives here. What a tab change means, and which element has the
/// focus, belongs to the classic interface of AMD-010, which is lot L9; this model is what that
/// lot reads.
/// </para>
/// <para>
/// Navigation is suspended while a binding capture is listening (REQ-INP-047), which the caller
/// signals with <c>suspended</c>.
/// </para>
/// </remarks>
public sealed class MenuNavigation
{
    private EdgeDetector _left;
    private EdgeDetector _right;
    private EdgeDetector _up;
    private EdgeDetector _down;
    private EdgeDetector _select;

    /// <summary>
    /// Reads one displayed frame of the merged action mask and returns the navigation events.
    /// </summary>
    /// <param name="mask">The OR of both players' masks.</param>
    /// <param name="paused">Whether the menu is open; outside the menu nothing is produced.</param>
    /// <param name="suspended">Whether a binding capture is listening (REQ-INP-047).</param>
    public MenuNavigationEvents Update(InputAction mask, bool paused, bool suspended = false)
    {
        if (!paused || suspended)
        {
            // The edges are still consumed so that a key already held when the menu opens does
            // not count as a press.
            _left.Rising(mask.HasFlag(InputAction.Left));
            _right.Rising(mask.HasFlag(InputAction.Right));
            _up.Rising(mask.HasFlag(InputAction.Up));
            _down.Rising(mask.HasFlag(InputAction.Down));
            _select.Rising(mask.HasFlag(InputAction.Select));
            return MenuNavigationEvents.None;
        }

        MenuNavigationEvents events = MenuNavigationEvents.None;

        if (_left.Rising(mask.HasFlag(InputAction.Left)))
        {
            events |= MenuNavigationEvents.Previous;
        }

        if (_right.Rising(mask.HasFlag(InputAction.Right)))
        {
            events |= MenuNavigationEvents.Next;
        }

        if (_up.Rising(mask.HasFlag(InputAction.Up)))
        {
            events |= MenuNavigationEvents.PreviousVertical;
        }

        if (_down.Rising(mask.HasFlag(InputAction.Down)))
        {
            events |= MenuNavigationEvents.NextVertical;
        }

        if (_select.Rising(mask.HasFlag(InputAction.Select)))
        {
            events |= MenuNavigationEvents.Secondary;
        }

        return events;
    }
}
