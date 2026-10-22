using ImGuiNET;
using Zax.App.Audio;
using Zax.App.Loop;

namespace Zax.App.Ui;

/// <summary>
/// The menu bar of AMD-010 REQ-UI-200, drawn from <see cref="MenuBarModel"/>: the rows, their
/// labels and their shortcut texts come from the model, and clicking one queues its command in
/// the model of REQ-UI-205 (REQ-UI-205's acceptance criterion).
/// </summary>
internal sealed partial class ZaxUi
{
    /// <summary>Draws the bar and returns the height it took, in pixels.</summary>
    private float DrawMenuBar()
    {
        float height = 0f;

        if (!ImGui.BeginMainMenuBar())
        {
            return height;
        }

        height = ImGui.GetWindowHeight();

        // The capture aid of --ui-shots: the popup identifier a menu-bar entry opens is the one
        // its label hashes to in this very window, so asking for it here is the same thing a
        // click would do.
        if (ForceOpenMenu is { } forced)
        {
            ImGui.OpenPopup(forced);
            ForceOpenMenu = null;
        }

        foreach (Menu menu in MenuBar.Menus)
        {
            if (!ImGui.BeginMenu(menu.Title))
            {
                continue;
            }

            DrawItems(menu.Items);
            ImGui.EndMenu();
        }

        ImGui.EndMainMenuBar();
        return height;
    }

    private void DrawItems(IReadOnlyList<MenuItem> items)
    {
        for (int index = 0; index < items.Count; index++)
        {
            MenuItem item = items[index];

            if (item.Kind == MenuItemKind.Separator)
            {
                ImGui.Separator();
                continue;
            }

            ImGui.PushID(index);
            try
            {
                if (item.Kind == MenuItemKind.Submenu)
                {
                    if (ImGui.BeginMenu(item.Label))
                    {
                        DrawItems(item.Children!());
                        ImGui.EndMenu();
                    }

                    continue;
                }

                bool enabled = MenuBar.IsEnabled(item);
                bool ticked = MenuBar.IsChecked(item);

                if (ImGui.MenuItem(item.Label, item.Shortcut, ticked, enabled) && item.Command is { } command)
                {
                    Activate(command, item.Args);
                }
            }
            finally
            {
                ImGui.PopID();
            }
        }
    }

    /// <summary>
    /// Activates an entry: the interface sound of SPEC-11 REQ-UI-140 ("validation"), then the
    /// command, queued so that it runs between two emulated frames (REQ-LOOP-007).
    /// </summary>
    private void Activate(string command, CommandArgs args)
    {
        _options.Mixer?.PlaySound(UiSound.Confirm);
        _host.Enqueue(command, args);
    }
}
