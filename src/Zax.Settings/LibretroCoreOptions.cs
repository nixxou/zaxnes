namespace Zax.Settings;

/// <summary>One libretro core option generated from the model (SPEC-09 §4.6, REQ-PERS-083).</summary>
/// <param name="Key">Core option key, <c>zaxnes_</c> and the stable key with separators replaced.</param>
/// <param name="SettingKey">The stable key of the setting behind it.</param>
/// <param name="Label">Human-readable name, the UI label when there is one.</param>
/// <param name="Category">Category of options v2, taken from the theme.</param>
/// <param name="Values">The finite ordered list of value labels (REQ-PERS-081).</param>
/// <param name="DefaultValue">The label of the default value, which is always in the list.</param>
public readonly record struct LibretroCoreOption(
    string Key,
    string SettingKey,
    string Label,
    string Category,
    IReadOnlyList<string> Values,
    string DefaultValue);

/// <summary>
/// The libretro core option table, generated from the settings catalogue (REQ-PERS-081 to
/// REQ-PERS-085). Data only: this lot builds and tests the table, the frontend that consumes it
/// is another one.
/// </summary>
/// <remarks>
/// <para>
/// Only user preferences carrying a discretisation are exposed. The P and S settings never are,
/// since they belong to the profile or to the script (REQ-PERS-085); nor are the settings §4.6
/// lists as host-owned — vsync, render rate, fast render, render method, resolution scale, the
/// interface keys, the menu music, the emulated menu button and the core index. SPEC-09 v1.1
/// also withdrew <c>video.graphic_quality</c> and <c>emu.tv_mode</c>, and both are
/// profile-scoped anyway (DEC-09-Q1).
/// </para>
/// <para>
/// A change coming from the host goes through <see cref="TryApply"/>, which writes into the
/// model exactly as the options window does, so the validation of REQ-PERS-003 and the
/// notification of REQ-PERS-009 happen on that path too (REQ-PERS-085).
/// </para>
/// </remarks>
public static class LibretroCoreOptions
{
    private static readonly LibretroCoreOption[] Options = Build();

    /// <summary>The core options, in the order of the catalogue.</summary>
    public static IReadOnlyList<LibretroCoreOption> All => Options;

    /// <summary>The option whose core key is <paramref name="coreOptionKey"/>, or null.</summary>
    public static LibretroCoreOption? Find(string coreOptionKey)
    {
        foreach (LibretroCoreOption option in Options)
        {
            if (string.Equals(option.Key, coreOptionKey, StringComparison.Ordinal))
            {
                return option;
            }
        }

        return null;
    }

    /// <summary>
    /// Applies the value <paramref name="label"/> of core option <paramref name="coreOptionKey"/>
    /// to <paramref name="model"/>, on the same path as a change from the UI (REQ-PERS-085).
    /// Returns false when the key or the label is unknown.
    /// </summary>
    public static bool TryApply(SettingsModel model, string coreOptionKey, string label)
    {
        ArgumentNullException.ThrowIfNull(model);

        if (Find(coreOptionKey) is not { } option)
        {
            return false;
        }

        SettingDescriptor descriptor = SettingsCatalogue.Get(option.SettingKey);
        if (descriptor.LibretroValues is not { } values)
        {
            return false;
        }

        foreach (SettingOption candidate in values)
        {
            if (!string.Equals(candidate.Label, label, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            SettingValue value = descriptor.Kind == SettingKind.Color

                // The colour options are the eight presets of DEC-09-Q5, held as packed RGB.
                ? SettingValue.OfColor(SettingColor.FromRgb((int)candidate.Value))
                : SettingValue.OfFloat((float)candidate.Value).CoerceTo(descriptor.Kind);

            model.Set(descriptor.Key, value, SettingLayer.User);
            return true;
        }

        return false;
    }

    /// <summary>
    /// The category name of a theme. REQ-PERS-083 only asks for a usable category; the names are
    /// the English form of the themes §4.6 shows.
    /// </summary>
    public static string CategoryOf(SettingTheme theme) => theme switch
    {
        SettingTheme.Video => "Video",
        SettingTheme.Render => "Render",
        SettingTheme.Camera => "Camera",
        SettingTheme.Light => "Light",
        SettingTheme.Scene => "Scene",
        SettingTheme.Clipping => "Clipping",
        SettingTheme.Layers => "Layers",
        SettingTheme.Emulation => "Emulation",
        SettingTheme.Conversion => "Conversion",
        SettingTheme.Audio => "Audio",
        SettingTheme.Input => "Input",
        SettingTheme.Interface => "Interface",
        _ => "Advanced",
    };

    private static LibretroCoreOption[] Build()
    {
        List<LibretroCoreOption> options = [];
        foreach (SettingDescriptor descriptor in SettingsCatalogue.All)
        {
            if (!descriptor.IsLibretroExposed || descriptor.Scope != SettingScope.User)
            {
                continue;
            }

            IReadOnlyList<SettingOption> values = descriptor.LibretroValues!;
            List<string> labels = new(values.Count);
            foreach (SettingOption value in values)
            {
                labels.Add(value.Label);
            }

            options.Add(new LibretroCoreOption(
                descriptor.LibretroKey,
                descriptor.Key,
                descriptor.UiLabel ?? descriptor.Key,
                CategoryOf(descriptor.Theme),
                labels,
                DefaultLabel(descriptor, values)));
        }

        return [.. options];
    }

    /// <summary>
    /// The label of the default value. REQ-PERS-081 requires the list to contain the default;
    /// for a colour that means the preset equal to the default colour.
    /// </summary>
    private static string DefaultLabel(SettingDescriptor descriptor, IReadOnlyList<SettingOption> values)
    {
        if (descriptor.Kind == SettingKind.Color)
        {
            SettingColor wanted = descriptor.DefaultValue.AsColor;
            foreach (SettingOption option in values)
            {
                if (SettingColor.FromRgb((int)option.Value) == wanted)
                {
                    return option.Label;
                }
            }

            return values[0].Label;
        }

        double number = descriptor.DefaultValue.Kind == SettingKind.Bool
            ? (descriptor.DefaultValue.AsBool ? 1d : 0d)
            : descriptor.DefaultValue.AsDouble;

        foreach (SettingOption option in values)
        {
            if (Math.Abs(option.Value - number) < 1e-6d)
            {
                return option.Label;
            }
        }

        return values[0].Label;
    }
}
