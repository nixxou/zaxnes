using System.Globalization;
using System.Numerics;
using ImGuiNET;
using Zax.App.Audio;
using Zax.App.Loop;

namespace Zax.App.Ui;

/// <summary>
/// The welcome screen of AMD-010 REQ-UI-207, the on-screen notifications of REQ-UI-206 and the
/// optional frame counter of SPEC-10 REQ-UI-095.
/// </summary>
internal sealed partial class ZaxUi
{
    /// <summary>
    /// REQ-UI-207: with no ROM loaded the window shows « Fichier ▸ Ouvrir une ROM » and the
    /// recent ROMs. The menu music of SPEC-11 plays by itself in every state but Running
    /// (REQ-AUD-020), so there is nothing to start here.
    /// </summary>
    private void DrawWelcome()
    {
        Vector2 display = ImGui.GetIO().DisplaySize;
        float width = MathF.Min(520f, MathF.Max(260f, display.X - 80f));
        float height = MathF.Min(360f, MathF.Max(200f, display.Y - 80f));

        ImGui.SetNextWindowPos(new Vector2(display.X * 0.5f, display.Y * 0.5f + ChromeHeight * 0.5f),
            ImGuiCond.Always,
            new Vector2(0.5f, 0.5f));
        ImGui.SetNextWindowSize(new Vector2(width, height), ImGuiCond.Always);
        ImGui.SetNextWindowBgAlpha(0.65f);

        const ImGuiWindowFlags flags = ImGuiWindowFlags.NoDecoration
            | ImGuiWindowFlags.NoSavedSettings
            | ImGuiWindowFlags.NoFocusOnAppearing
            | ImGuiWindowFlags.NoNav;

        if (ImGui.Begin("##welcome", flags))
        {
            if (UiFonts.Heading is { } heading)
            {
                ImGui.PushFont(heading);
                ImGui.TextUnformatted(UiText.ProductName);
                ImGui.PopFont();
            }
            else
            {
                ImGui.TextUnformatted(UiText.ProductName);
            }

            ImGui.TextDisabled(UiText.VersionLine);
            ImGui.Separator();
            ImGui.TextUnformatted(UiText.WelcomeHint);
            ImGui.Spacing();

            if (ImGui.Button(_host.Commands.Find(ZaxCommands.OpenRom)?.Label ?? UiText.OpenRomTitle))
            {
                Activate(ZaxCommands.OpenRom, default);
            }

            ImGui.Spacing();
            ImGui.Separator();
            ImGui.TextUnformatted(UiText.WelcomeRecent);

            IReadOnlyList<string> recent = _games.RecentRoms;
            if (recent.Count == 0)
            {
                ImGui.TextDisabled(UiText.WelcomeNoRecent);
            }
            else
            {
                for (int index = 0; index < recent.Count; index++)
                {
                    ImGui.PushID(index);
                    try
                    {
                        if (ImGui.Selectable(Path.GetFileName(recent[index])))
                        {
                            _options.Mixer?.PlaySound(UiSound.Confirm);
                            OpenRecent(index);
                        }
                    }
                    finally
                    {
                        ImGui.PopID();
                    }
                }
            }
        }

        ImGui.End();
    }

    /// <summary>
    /// The notification area of SPEC-10 REQ-UI-080, drawn as an OSD over the game view
    /// (REQ-UI-206): one line, centred at the bottom, italic in the original and here coloured by
    /// severity — white, yellow, red. It is the one layer full screen keeps (REQ-UI-209).
    /// </summary>
    private void DrawNotification()
    {
        if (_host.Notifications.Current is not { } notification)
        {
            return;
        }

        Vector2 display = ImGui.GetIO().DisplaySize;
        ImGui.SetNextWindowPos(
            new Vector2(display.X * 0.5f, display.Y - 48f),
            ImGuiCond.Always,
            new Vector2(0.5f, 1f));
        ImGui.SetNextWindowBgAlpha(0.5f);

        const ImGuiWindowFlags flags = ImGuiWindowFlags.NoDecoration
            | ImGuiWindowFlags.AlwaysAutoResize
            | ImGuiWindowFlags.NoSavedSettings
            | ImGuiWindowFlags.NoFocusOnAppearing
            | ImGuiWindowFlags.NoNav
            | ImGuiWindowFlags.NoInputs;

        if (ImGui.Begin("##notification", flags))
        {
            // AMD-036 §1: the text the loop produced is the English entry of the table, so it is
            // translated here, when it is drawn — and follows a language change while it is up.
            ImGui.TextColored(
                ColourOf(notification.Severity),
                UiNotifications.Translate(notification.Text));
        }

        ImGui.End();
    }

    /// <summary>The colour of a severity (REQ-UI-080).</summary>
    internal static Vector4 ColourOf(NotificationSeverity severity) => severity switch
    {
        NotificationSeverity.Error => new Vector4(1f, 0.3f, 0.3f, 1f),
        NotificationSeverity.Warning => new Vector4(1f, 0.9f, 0.3f, 1f),
        _ => new Vector4(1f, 1f, 1f, 1f),
    };

    /// <summary>
    /// The frame counter of SPEC-10 REQ-UI-095 and DEC-10-Q7: a small panel, off by default
    /// (<c>ui.show_fps</c>), one decimal, refreshed every half second.
    /// </summary>
    private void DrawFpsCounter(float realSeconds)
    {
        if (!_settings.ShowFps)
        {
            return;
        }

        _fpsCountdown -= realSeconds;
        if (_fpsCountdown <= 0f)
        {
            _fpsCountdown = RefreshSeconds;
            _fpsShown = _options.FramesPerSecond?.Invoke() ?? 0f;
        }

        ImGui.SetNextWindowPos(
            new Vector2(ImGui.GetIO().DisplaySize.X - 12f, ChromeHeight + 12f),
            ImGuiCond.Always,
            new Vector2(1f, 0f));
        ImGui.SetNextWindowBgAlpha(0.45f);

        const ImGuiWindowFlags flags = ImGuiWindowFlags.NoDecoration
            | ImGuiWindowFlags.AlwaysAutoResize
            | ImGuiWindowFlags.NoSavedSettings
            | ImGuiWindowFlags.NoFocusOnAppearing
            | ImGuiWindowFlags.NoNav
            | ImGuiWindowFlags.NoInputs;

        if (ImGui.Begin("FPS", flags))
        {
            ImGui.TextUnformatted(string.Create(CultureInfo.InvariantCulture, $"{_fpsShown:F1} FPS"));
        }

        ImGui.End();
    }

    /// <summary>How often the counter of REQ-UI-095 is refreshed.</summary>
    private const float RefreshSeconds = 0.5f;
}
