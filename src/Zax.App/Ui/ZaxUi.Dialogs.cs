using System.Numerics;
using ImGuiNET;
using Zax.App.Audio;

namespace Zax.App.Ui;

/// <summary>
/// The dialogs of AMD-010: the ROM and profile file dialog of REQ-UI-201, the profile chooser of
/// its step 2, and the about block of DEC-10-Q3.
/// </summary>
/// <remarks>
/// SPEC-10 REQ-UI-100 and REQ-UI-110 make these windows modal; AMD-010 §1 turns the presentation
/// and the navigation of SPEC-10 informative, so they are ordinary windows here, drawn over the
/// view and closable by their own buttons. The <b>content</b> — the title, the path trail, the
/// drives, the folders-then-files order, the filters and the remembered folder — follows
/// REQ-UI-100 to REQ-UI-105.
/// </remarks>
internal sealed partial class ZaxUi
{
    private void DrawFileBrowser()
    {
        ImGui.SetNextWindowSize(new Vector2(700f, 480f), ImGuiCond.FirstUseEver);
        ImGui.SetNextWindowPos(Centre(700f, 480f), ImGuiCond.Appearing);

        bool open = State.FileBrowser;
        if (!ImGui.Begin(_browser.Title, ref open, ImGuiWindowFlags.NoCollapse))
        {
            ImGui.End();
            State.FileBrowser = open;
            return;
        }

        DrawBrowserTrail();

        // AMD-030 §2: the 3dSen picker says what it expects, since a folder is an unusual answer
        // for this dialog.
        if (_browser.Hint.Length > 0)
        {
            ImGui.TextWrapped(_browser.Hint);
        }

        string search = _browser.Search;
        ImGui.SetNextItemWidth(220f);
        if (ImGui.InputText($"{UiText.Search}##search", ref search, 128))
        {
            _browser.Search = search;
        }

        ImGui.SameLine();
        if (ImGui.Button(UiText.ParentFolder) && _browser.Directory is not null)
        {
            _browser.GoUp();
        }

        ImGui.Separator();

        if (ImGui.BeginChild("##entries", new Vector2(0f, -2f * ImGui.GetFrameHeightWithSpacing())))
        {
            DrawBrowserEntries(ref open);
        }

        ImGui.EndChild();

        ImGui.Separator();
        DrawBrowserFooter(ref open);

        ImGui.End();
        State.FileBrowser = open;
    }

    private void DrawBrowserTrail()
    {
        IReadOnlyList<(string Label, string Path)> trail = _browser.Trail;
        for (int index = 0; index < trail.Count; index++)
        {
            if (index > 0)
            {
                ImGui.SameLine();
            }

            ImGui.PushID(index);
            try
            {
                if (ImGui.SmallButton(trail[index].Label))
                {
                    _browser.GoTo(trail[index].Path.Length == 0 ? null : trail[index].Path);
                }
            }
            finally
            {
                ImGui.PopID();
            }
        }
    }

    private void DrawBrowserEntries(ref bool open)
    {
        IReadOnlyList<FileEntry> entries = _browser.Entries;
        for (int index = 0; index < entries.Count; index++)
        {
            FileEntry entry = entries[index];
            bool selected = ReferenceEquals(_browser.Selected, entry);
            string label = entry.Kind == FileEntryKind.File ? entry.Name : $"[{entry.Name}]";

            ImGui.PushID(index);
            try
            {
                if (!ImGui.Selectable(label, selected))
                {
                    continue;
                }

                // REQ-UI-104: a click selects, a click on the selected row or a double click
                // opens it.
                if (selected || ImGui.IsMouseDoubleClicked(ImGuiMouseButton.Left))
                {
                    if (_browser.Activate(entry) && _browser.Result is { } path)
                    {
                        Finish(path, ref open);
                    }
                }
                else
                {
                    _browser.Selected = entry;
                    _options.Mixer?.PlaySound(UiSound.Move);
                }
            }
            finally
            {
                ImGui.PopID();
            }
        }
    }

    private void DrawBrowserFooter(ref bool open)
    {
        ImGui.TextUnformatted(_browser.FileName.Length > 0 ? _browser.FileName : " ");

        int filter = _browser.FilterIndex;
        string[] labels = new string[_browser.Filters.Count];
        for (int index = 0; index < labels.Length; index++)
        {
            labels[index] = _browser.Filters[index].Label;
        }

        if (labels.Length > 0)
        {
            ImGui.SetNextItemWidth(260f);
            if (ImGui.Combo("##filter", ref filter, labels, labels.Length))
            {
                _browser.ChooseFilter(filter);
            }

            ImGui.SameLine();
        }

        if (ImGui.Button(UiText.Cancel))
        {
            open = false;
        }

        ImGui.SameLine();
        ImGui.BeginDisabled(!_browser.CanConfirm);
        if (ImGui.Button(UiText.Open) && _browser.Confirm() && _browser.Result is { } path)
        {
            Finish(path, ref open);
        }

        ImGui.EndDisabled();
    }

    private void Finish(string path, ref bool open)
    {
        open = false;
        _options.Mixer?.PlaySound(UiSound.Confirm);
        OnFileChosen(path);
    }

    /// <summary>
    /// Step 2 of REQ-UI-201: the embedded catalogue, the user profiles and « 2D Mode », with a
    /// search by title. The choice is remembered for the ROM file.
    /// </summary>
    private void DrawProfileChooser()
    {
        ImGui.SetNextWindowSize(new Vector2(520f, 460f), ImGuiCond.FirstUseEver);
        ImGui.SetNextWindowPos(Centre(520f, 460f), ImGuiCond.Appearing);

        bool open = State.ProfileChooser;
        if (!ImGui.Begin(UiText.ProfileChooserTitle, ref open, ImGuiWindowFlags.NoCollapse))
        {
            ImGui.End();
            State.ProfileChooser = open;
            return;
        }

        if (_chooser.RomPath is { } rom)
        {
            ImGui.TextWrapped(Path.GetFileName(rom));
        }

        string search = _chooser.Search;
        ImGui.SetNextItemWidth(-1f);
        if (ImGui.InputText($"##{UiText.Search}", ref search, 128))
        {
            _chooser.Search = search;
        }

        ImGui.Separator();

        if (ImGui.BeginChild("##profiles", new Vector2(0f, -ImGui.GetFrameHeightWithSpacing())))
        {
            IReadOnlyList<ProfileChoice> choices = _chooser.Filtered;
            for (int index = 0; index < choices.Count; index++)
            {
                ProfileChoice choice = choices[index];
                bool selected = string.Equals(_chooser.SelectedId, choice.Id, StringComparison.Ordinal);

                ImGui.PushID(index);
                try
                {
                    if (ImGui.Selectable($"{choice.Title}", selected))
                    {
                        if (selected)
                        {
                            Choose(choice, ref open);
                        }
                        else
                        {
                            _chooser.SelectedId = choice.Id;
                            _options.Mixer?.PlaySound(UiSound.Move);
                        }
                    }

                    ImGui.SameLine();
                    ImGui.TextDisabled(choice.SourceLabel);
                }
                finally
                {
                    ImGui.PopID();
                }
            }
        }

        ImGui.EndChild();

        ImGui.BeginDisabled(_chooser.SelectedId is null);
        if (ImGui.Button(UiText.Launch))
        {
            ProfileChoice? picked = null;
            foreach (ProfileChoice choice in _chooser.All)
            {
                if (string.Equals(choice.Id, _chooser.SelectedId, StringComparison.Ordinal))
                {
                    picked = choice;
                    break;
                }
            }

            if (picked is { } ready)
            {
                Choose(ready, ref open);
            }
        }

        ImGui.EndDisabled();

        ImGui.SameLine();
        if (ImGui.Button(UiText.Cancel))
        {
            open = false;
        }

        ImGui.End();
        State.ProfileChooser = open;
    }

    private void Choose(ProfileChoice choice, ref bool open)
    {
        open = false;
        _options.Mixer?.PlaySound(UiSound.Confirm);

        if (_chooser.RomPath is { Length: > 0 } rom)
        {
            // REQ-UI-201: the pair is remembered, so the next opening of this file is direct.
            _host.LaunchRomWithProfile(rom, choice.Id);
        }
        else
        {
            _host.Enqueue(Loop.ZaxCommands.LaunchGame, Loop.CommandArgs.Of(choice.Id));
        }
    }

    /// <summary>
    /// The confirmation box E8 of SPEC-10 REQ-UI-110: modal, centred, a title, a message and a
    /// row of buttons, the first of which has the focus. There is no close cross — the box is
    /// left by its buttons only — and each button runs its action and then closes.
    /// </summary>
    private void DrawConfirmDialog()
    {
        ImGui.SetNextWindowPos(Centre(380f, 0f), ImGuiCond.Always);
        ImGui.SetNextWindowSize(new Vector2(380f, 0f), ImGuiCond.Always);

        const ImGuiWindowFlags flags = ImGuiWindowFlags.NoCollapse
            | ImGuiWindowFlags.NoResize
            | ImGuiWindowFlags.NoSavedSettings
            | ImGuiWindowFlags.AlwaysAutoResize;

        if (ImGui.Begin($"{_confirm.Title}##confirm", flags))
        {
            ImGui.TextWrapped(_confirm.Message);
            ImGui.Separator();

            // REQ-UI-110: the first button has the focus by default.
            if (ImGui.IsWindowAppearing())
            {
                ImGui.SetKeyboardFocusHere();
            }

            if (ImGui.Button(_confirm.ConfirmLabel, new Vector2(110f, 0f)))
            {
                _options.Mixer?.PlaySound(UiSound.Confirm);
                _confirm.Confirm();
            }

            ImGui.SameLine();
            if (ImGui.Button(_confirm.CancelLabel, new Vector2(110f, 0f)))
            {
                _confirm.Close();
            }
        }

        ImGui.End();
        State.Dialog = _confirm.IsOpen;
    }

    /// <summary>The about block of DEC-10-Q3 / SPEC-10 REQ-UI-076.</summary>
    private void DrawAbout()
    {
        ImGui.SetNextWindowSize(new Vector2(460f, 220f), ImGuiCond.FirstUseEver);
        ImGui.SetNextWindowPos(Centre(460f, 220f), ImGuiCond.Appearing);

        bool open = State.About;
        if (ImGui.Begin(UiText.AboutTitle, ref open, ImGuiWindowFlags.NoCollapse))
        {
            IReadOnlyList<string> lines = UiText.AboutLines;
            for (int index = 0; index < lines.Count; index++)
            {
                if (index == 0 && UiFonts.Heading is { } heading)
                {
                    ImGui.PushFont(heading);
                    Centred(lines[index]);
                    ImGui.PopFont();
                    continue;
                }

                Centred(lines[index]);
            }

            ImGui.Separator();
            if (ImGui.Button(UiText.Close))
            {
                open = false;
            }
        }

        ImGui.End();
        State.About = open;

        static void Centred(string text)
        {
            float width = ImGui.CalcTextSize(text).X;
            ImGui.SetCursorPosX(MathF.Max(ImGui.GetCursorPosX(), (ImGui.GetWindowWidth() - width) * 0.5f));
            ImGui.TextUnformatted(text);
        }
    }
}
