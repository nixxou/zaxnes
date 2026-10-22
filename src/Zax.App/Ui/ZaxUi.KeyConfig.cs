using System.Numerics;
using ImGuiNET;
using Zax.App.Audio;
using Zax.App.Input;
using Zax.Platform.Input;

namespace Zax.App.Ui;

/// <summary>
/// The key configuration window of AMD-010 REQ-UI-203: every logical action of SPEC-08 §4.1 for
/// both players and every argument-less menu command, with a keyboard column and a gamepad
/// column, captured through the <see cref="RebindingSession"/> of lot L8.
/// </summary>
/// <remarks>
/// Clicking a cell clears its binding and starts listening (REQ-INP-042); the next control
/// pressed becomes the binding on its rising edge (Q-033, AMD-015); « Arrêter la capture » leaves
/// the action unbound (REQ-INP-045). The capture itself runs in the input layer, once per
/// sample — this window only opens and closes it, which is what keeps the rules in one place.
/// </remarks>
internal sealed partial class ZaxUi
{
    private void DrawKeyConfig()
    {
        ImGui.SetNextWindowSize(new Vector2(640f, 560f), ImGuiCond.FirstUseEver);
        ImGui.SetNextWindowPos(Centre(640f, 560f), ImGuiCond.FirstUseEver);

        bool open = State.KeyConfig;
        if (ImGui.Begin(UiText.KeyConfigTitle, ref open, ImGuiWindowFlags.NoCollapse))
        {
            DrawKeyConfigHeader();
            ImGui.Separator();

            if (ImGui.BeginChild("##bindings", new Vector2(0f, -ImGui.GetFrameHeightWithSpacing())))
            {
                foreach (KeyConfigSection section in _keys.Sections)
                {
                    if (ImGui.CollapsingHeader(section.Title, ImGuiTreeNodeFlags.DefaultOpen))
                    {
                        DrawBindingTable(section);
                    }
                }
            }

            ImGui.EndChild();

            if (ImGui.Button(UiText.Close))
            {
                open = false;
            }
        }

        ImGui.End();

        if (!open)
        {
            // REQ-UI-067: closing the screen stops a capture in progress.
            CloseCapture();
        }

        State.KeyConfig = open;
    }

    private void DrawKeyConfigHeader()
    {
        RebindingSession? session = _options.Input?.Rebinding;

        if (session is { IsListening: true })
        {
            ImGui.TextUnformatted(UiText.PressAControl);
            ImGui.SameLine();
            if (ImGui.Button(UiText.StopBinding))
            {
                session.Stop();
            }
        }
        else
        {
            ImGui.TextDisabled("Cliquez sur une case pour réassigner.");
        }

        DrawGamepadChoosers();

        if (ImGui.Button("Réinitialiser toutes les touches"))
        {
            // REQ-INP-049: back to the defaults of §4.2 and §4.3, both players, both devices.
            _options.Input?.Rebinding.Stop();
            _options.Bindings.ResetAll();
            _options.Mixer?.PlaySound(UiSound.Confirm);
        }
    }

    /// <summary>
    /// The pad chooser of SPEC-10 REQ-UI-070, one per player: « aucune manette » and the
    /// connected pads, refreshed on plug and unplug (REQ-INP-023, REQ-INP-041).
    /// </summary>
    private void DrawGamepadChoosers()
    {
        if (_options.Input is not { } input)
        {
            return;
        }

        GamepadAssignment pads = input.Pads;
        IReadOnlyList<IGamepadDevice> available = pads.Available;

        for (int player = 0; player < InputActions.PlayerCount; player++)
        {
            string[] labels = new string[available.Count + 1];
            labels[0] = UiText.NoGamepad;
            int current = 0;

            for (int index = 0; index < available.Count; index++)
            {
                labels[index + 1] = available[index].DisplayName;
                if (ReferenceEquals(available[index], pads.Device(player)))
                {
                    current = index + 1;
                }
            }

            ImGui.PushID(player);
            try
            {
                string label = $"{UiText.GamepadColumn} J{player + 1}";
                if (ImGui.Combo(label, ref current, labels, labels.Length))
                {
                    pads.Assign(player, current == 0 ? null : available[current - 1]);
                    input.Rebinding.NotifyGamepadChanged();
                    _options.Mixer?.PlaySound(UiSound.Confirm);
                }
            }
            finally
            {
                ImGui.PopID();
            }
        }
    }

    private void DrawBindingTable(KeyConfigSection section)
    {
        if (!ImGui.BeginTable(
            $"##table-{section.Title}",
            3,
            ImGuiTableFlags.SizingStretchProp | ImGuiTableFlags.RowBg | ImGuiTableFlags.BordersInnerV))
        {
            return;
        }

        ImGui.TableSetupColumn("Action", ImGuiTableColumnFlags.WidthStretch, 2f);
        ImGui.TableSetupColumn(UiText.KeyboardColumn, ImGuiTableColumnFlags.WidthStretch, 1.5f);
        ImGui.TableSetupColumn(UiText.GamepadColumn, ImGuiTableColumnFlags.WidthStretch, 1.5f);
        ImGui.TableHeadersRow();

        for (int index = 0; index < section.Rows.Count; index++)
        {
            KeyConfigRow row = section.Rows[index];

            ImGui.TableNextRow();
            ImGui.TableNextColumn();
            ImGui.TextUnformatted(row.Label);

            ImGui.PushID(index);
            try
            {
                ImGui.TableNextColumn();
                DrawBindingCell(row, BindingDeviceKind.Keyboard);

                ImGui.TableNextColumn();
                DrawBindingCell(row, BindingDeviceKind.Gamepad);
            }
            finally
            {
                ImGui.PopID();
            }
        }

        ImGui.EndTable();
    }

    private void DrawBindingCell(KeyConfigRow row, BindingDeviceKind kind)
    {
        RebindingSession? session = _options.Input?.Rebinding;
        BindingSet? set = KeyConfigModel.SetOf(
            _options.Bindings,
            _options.Input?.Pads,
            row.Player,
            kind);

        bool listening = session is not null && KeyConfigModel.IsListeningFor(session, row, kind);
        string label = listening
            ? "…"
            : set is null
                ? UiText.NoGamepad
                : ControlLabels.Of(set.Get(row.Target));

        ImGui.PushID((int)kind);
        try
        {
            // REQ-UI-071: while a capture is listening the other cells are inactive.
            bool otherListening = session is { IsListening: true } && !listening;
            ImGui.BeginDisabled(otherListening || set is null);

            if (ImGui.Button($"{label}##cell", new Vector2(-1f, 0f)) && session is not null)
            {
                KeyConfigModel.Begin(session, row, kind);
            }

            ImGui.EndDisabled();
        }
        finally
        {
            ImGui.PopID();
        }
    }
}
