using System.Globalization;
using System.Numerics;
using ImGuiNET;
using Zax.App.Audio;
using Zax.Settings;

namespace Zax.App.Ui;

/// <summary>
/// The options window of AMD-010 REQ-UI-202, drawn from <see cref="OptionsModel"/>: one tab per
/// group, one row per setting, the widget chosen from the setting's shape and bounds, and a
/// « Valeurs par défaut » button per tab.
/// </summary>
/// <remarks>
/// Every change is written through <see cref="SettingsModel"/>, which applies it immediately,
/// notifies the loop and the renderer and schedules the file write (SPEC-09 REQ-PERS-009,
/// REQ-PERS-021). There is no Apply button, as SPEC-10 REQ-UI-060 requires.
/// </remarks>
internal sealed partial class ZaxUi
{
    private void DrawOptions()
    {
        SettingsModel model = _settings.Model;

        ImGui.SetNextWindowSize(new Vector2(560f, 520f), ImGuiCond.FirstUseEver);
        ImGui.SetNextWindowPos(Centre(560f, 520f), ImGuiCond.FirstUseEver);

        bool open = State.Options;
        if (ImGui.Begin(UiText.OptionsTitle, ref open, ImGuiWindowFlags.NoCollapse)
            && ImGui.BeginTabBar("##options"))
        {
            foreach (OptionsTab tab in OptionsModel.Tabs)
            {
                if (!ImGui.BeginTabItem(OptionsModel.TitleOf(tab)))
                {
                    continue;
                }

                _tab = tab;

                if (ImGui.Button(UiText.ResetDefaults))
                {
                    _optionsModel.ResetTab(model, tab);
                    _options.Mixer?.PlaySound(UiSound.Confirm);
                }

                ImGui.Separator();

                foreach (OptionsRow row in _optionsModel.Rows(tab))
                {
                    DrawRow(model, row);
                }

                ImGui.EndTabItem();
            }

            ImGui.EndTabBar();
        }

        ImGui.End();
        State.Options = open;
    }

    private void DrawRow(SettingsModel model, OptionsRow row)
    {
        SettingDescriptor descriptor = row.Descriptor;
        ImGui.PushID(descriptor.Key);

        // AMD-025: with no television model file there is nothing to show, so the row is
        // greyed out instead of offering a setting that would do nothing.
        bool disabled = descriptor.Key == SettingKeys.SceneTvBox
            && !_options.Host.TelevisionAvailable;
        if (disabled)
        {
            ImGui.BeginDisabled();
        }

        try
        {
            // A setting whose domain is a finite list gets a drop-down whatever its shape:
            // video.frame_rate is a whole number out of {60, 90, 120} (SPEC-09 §4.1.1), and
            // SPEC-10 REQ-UI-061 shows it as a list.
            bool enumerated = descriptor.Domain.Options is { Count: > 0 }
                && descriptor.Kind is SettingKind.Enum or SettingKind.Int;

            bool changed = enumerated
                ? DrawEnum(model, row)
                : descriptor.Kind switch
                {
                    SettingKind.Bool => DrawBool(model, row),
                    SettingKind.Enum => DrawEnum(model, row),
                    SettingKind.Color => DrawColor(model, row),
                    SettingKind.Int => DrawInt(model, row),
                    _ => DrawFloat(model, row),
                };

            if (changed && OptionsModel.PlaysValidationSound(descriptor.Key))
            {
                // AMD-021 R2-013 (replacing the note and the table of REQ-AUD-041): every
                // setting of the page plays the validation sound once per change of value,
                // except Speed and FOV, which are silent.
                _options.Mixer?.PlaySound(UiSound.Confirm);
            }

            DrawRowHint(row);
        }
        finally
        {
            if (disabled)
            {
                ImGui.EndDisabled();
            }

            ImGui.PopID();
        }
    }

    /// <summary>
    /// The one-line hint under a row: the key SPEC-09 persists it under, when its change needs a
    /// restart, and the warning that a profile parameter will be overwritten by the next profile
    /// load (REQ-PERS-005, REQ-PERS-084).
    /// </summary>
    private static void DrawRowHint(OptionsRow row)
    {
        if (row.Descriptor.Effect == SettingEffect.Restart)
        {
            ImGui.SameLine();
            ImGui.TextDisabled("(au prochain démarrage)");
        }
        else if (row.Descriptor.Effect is SettingEffect.NextGameLoad or SettingEffect.NextRomLoad)
        {
            ImGui.SameLine();
            ImGui.TextDisabled("(au prochain chargement)");
        }

        if (row.IsProfileScoped)
        {
            ImGui.SameLine();
            ImGui.TextDisabled("(réglage de profil)");
        }
    }

    private static bool DrawBool(SettingsModel model, OptionsRow row)
    {
        bool value = model.GetBool(row.Descriptor.Key);
        if (!ImGui.Checkbox(row.Label, ref value))
        {
            return false;
        }

        model.SetBool(row.Descriptor.Key, value);
        return true;
    }

    private bool DrawEnum(SettingsModel model, OptionsRow row)
    {
        IReadOnlyList<SettingOption> options = row.Descriptor.Domain.Options ?? [];
        if (options.Count == 0)
        {
            return DrawInt(model, row);
        }

        double current = model.Get(row.Descriptor.Key).AsDouble;
        int index = 0;
        string[] labels = new string[options.Count];
        for (int i = 0; i < options.Count; i++)
        {
            labels[i] = UiText.LabelOf(row.Descriptor, options[i].Label);
            if (Math.Abs(options[i].Value - current) < 1e-9)
            {
                index = i;
            }
        }

        if (!ImGui.Combo(row.Label, ref index, labels, labels.Length))
        {
            return false;
        }

        int picked = (int)options[index].Value;
        model.Set(
            row.Descriptor.Key,
            row.Descriptor.Kind == SettingKind.Int ? SettingValue.OfInt(picked) : SettingValue.OfEnum(picked));
        return true;
    }

    private static bool DrawInt(SettingsModel model, OptionsRow row)
    {
        int value = model.GetInt(row.Descriptor.Key);

        if (row.Maximum > row.Minimum)
        {
            int low = (int)Math.Round(row.Minimum);
            int high = (int)Math.Round(row.Maximum);
            if (!ImGui.SliderInt(row.Label, ref value, low, high))
            {
                return false;
            }
        }
        else if (!ImGui.InputInt(row.Label, ref value))
        {
            return false;
        }

        model.SetInt(row.Descriptor.Key, value);
        return true;
    }

    private static bool DrawFloat(SettingsModel model, OptionsRow row)
    {
        float value = model.GetFloat(row.Descriptor.Key);

        if (row.Maximum <= row.Minimum)
        {
            if (!ImGui.InputFloat(row.Label, ref value))
            {
                return false;
            }

            model.SetFloat(row.Descriptor.Key, value);
            return true;
        }

        // A step of one or more means the setting is whole in the interface — the speed of
        // REQ-LOOP-015 and the field of view of SPEC-10 REQ-UI-061 — so the slider is an integer
        // one and the value lands exactly on a step.
        if (row.Step >= 1d)
        {
            int whole = (int)MathF.Round(value / (float)row.Step) * (int)row.Step;
            int low = (int)Math.Round(row.Minimum);
            int high = (int)Math.Round(row.Maximum);
            if (!ImGui.SliderInt(row.Label, ref whole, low, high))
            {
                return false;
            }

            model.SetFloat(row.Descriptor.Key, whole);
            return true;
        }

        if (!ImGui.SliderFloat(row.Label, ref value, (float)row.Minimum, (float)row.Maximum, "%.2f"))
        {
            return false;
        }

        model.SetFloat(row.Descriptor.Key, value);
        return true;
    }

    /// <summary>
    /// The background colour of SPEC-10 REQ-UI-063 and REQ-UI-090: a standard ImGui colour
    /// button, which REQ-UI-090 accepts as the picker.
    /// </summary>
    private static bool DrawColor(SettingsModel model, OptionsRow row)
    {
        SettingColor stored = model.GetColor(row.Descriptor.Key);
        Vector4 value = new(stored.R, stored.G, stored.B, stored.A);

        if (!ImGui.ColorEdit4(row.Label, ref value, ImGuiColorEditFlags.NoInputs | ImGuiColorEditFlags.AlphaPreview))
        {
            return false;
        }

        model.SetColor(row.Descriptor.Key, new SettingColor(value.X, value.Y, value.Z, value.W));
        return true;
    }

    /// <summary>The percentage a volume-like setting shows, for the menu and the tooltips.</summary>
    internal static string Percent(float value)
        => string.Create(CultureInfo.InvariantCulture, $"{MathF.Round(value * 100f):0} %");
}
