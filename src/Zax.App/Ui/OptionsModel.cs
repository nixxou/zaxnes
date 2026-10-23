using Zax.Settings;

namespace Zax.App.Ui;

/// <summary>The tabs of the options window, exactly the six groups AMD-010 REQ-UI-202 names.</summary>
internal enum OptionsTab
{
    /// <summary>Vidéo: video output, rendering and the scene decor.</summary>
    Video,

    /// <summary>Caméra.</summary>
    Camera,

    /// <summary>Lumière et ombres.</summary>
    Light,

    /// <summary>Audio.</summary>
    Audio,

    /// <summary>Émulation, inputs included.</summary>
    Emulation,

    /// <summary>Interface.</summary>
    Interface,
}

/// <summary>
/// One row of the options window: a setting, its label and the bounds the widget uses.
/// </summary>
/// <param name="Descriptor">The setting, straight out of the catalogue.</param>
/// <param name="Minimum">Lowest value the widget offers, for a number.</param>
/// <param name="Maximum">Highest value the widget offers, for a number.</param>
/// <param name="Step">The step a number widget snaps to, or 0 for a free slider.</param>
internal sealed record OptionsRow(
    SettingDescriptor Descriptor,
    double Minimum,
    double Maximum,
    double Step)
{
    /// <summary>
    /// The label, read in the interface language every time it is asked for, so a change of
    /// language reaches the open window at once (AMD-035 §1).
    /// </summary>
    public string Label => UiText.LabelOf(Descriptor);

    /// <summary>Whether the setting is a profile parameter a profile load will overwrite.</summary>
    public bool IsProfileScoped => Descriptor.Scope == SettingScope.Profile;
}

/// <summary>
/// The options window of AMD-010 REQ-UI-202, generated from the settings catalogue of SPEC-09:
/// every user preference, grouped in the six tabs the amendment names, with its bounds, its
/// default and the effect SPEC-09 gives it.
/// </summary>
/// <remarks>
/// <para>
/// Generated, not written out: the rows come from <see cref="SettingsCatalogue"/>, so a setting
/// added to SPEC-09 appears in the window without a line of UI code, and the test of REQ-UI-202
/// can assert that no user preference is missing.
/// </para>
/// <para>
/// The bounds a widget offers are the ones of the libretro discretisation when the setting has
/// one, because that is where SPEC-09 §4.6 records the <b>interface</b> range rather than the
/// wider range a script may write: <c>camera.fov</c> accepts 10 to 150 from Lua and the slider
/// covers 45 to 95 (SPEC-10 REQ-UI-061), and <c>light.intensity</c> is unbounded for a script
/// while the slider covers 0,5 to 1,5.
/// </para>
/// </remarks>
internal sealed class OptionsModel
{
    /// <summary>
    /// The user preferences the window deliberately leaves out, with the reason.
    /// </summary>
    /// <remarks>
    /// <c>ui.last_browse_dir</c> and <c>ui.window_rect</c> are remembered state, not preferences:
    /// REQ-PERS-066 and REQ-PERS-067 have the program write them by itself, and there is nothing
    /// for the user to choose. The VR settings are out of scope altogether (REQ-PERS-011,
    /// SPEC-10 §1), which is why the <see cref="SettingTheme.Vr"/> theme maps to no tab.
    /// </remarks>
    /// <remarks>
    /// <c>audio.bg_music</c> is excluded since AMD-024 REQ-PUB-005 (revised 2026-10-21): the
    /// publishable version has no menu music, so the setting has no effect. It stays in the
    /// model so that an existing <c>settings.json</c> still reads.
    /// </remarks>
    public static IReadOnlyList<string> ExcludedKeys { get; } =
        [SettingKeys.UiLastBrowseDir, SettingKeys.UiWindowRect, SettingKeys.AudioBgMusic];

    /// <summary>
    /// The settings that are not user preferences but that SPEC-10 REQ-UI-061 showed, so the
    /// window keeps them (REQ-UI-202 allows more than the strict U-scope list).
    /// </summary>
    public static IReadOnlyList<string> IncludedProfileKeys { get; } = [SettingKeys.EmuTvMode];

    private readonly Dictionary<OptionsTab, List<OptionsRow>> _rows = [];

    /// <summary>Builds the window from the catalogue.</summary>
    public OptionsModel()
    {
        foreach (OptionsTab tab in Tabs)
        {
            _rows[tab] = [];
        }

        foreach (SettingDescriptor descriptor in SettingsCatalogue.All)
        {
            if (!IsShown(descriptor) || TabOf(descriptor) is not { } tab)
            {
                continue;
            }

            _rows[tab].Add(BuildRow(descriptor));
        }
    }

    /// <summary>The six tabs, in the order of REQ-UI-202.</summary>
    public static IReadOnlyList<OptionsTab> Tabs { get; } =
    [
        OptionsTab.Video,
        OptionsTab.Camera,
        OptionsTab.Light,
        OptionsTab.Audio,
        OptionsTab.Emulation,
        OptionsTab.Interface,
    ];

    /// <summary>The title of a tab, in the interface language (REQ-UI-202, AMD-035).</summary>
    public static string TitleOf(OptionsTab tab) => UiStrings.Get(tab switch
    {
        OptionsTab.Video => UiKeys.TabVideo,
        OptionsTab.Camera => UiKeys.TabCamera,
        OptionsTab.Light => UiKeys.TabLight,
        OptionsTab.Audio => UiKeys.TabAudio,
        OptionsTab.Emulation => UiKeys.TabEmulation,
        _ => UiKeys.TabInterface,
    });

    /// <summary>The rows of a tab, in catalogue order.</summary>
    public IReadOnlyList<OptionsRow> Rows(OptionsTab tab) => _rows[tab];

    /// <summary>Every row of every tab.</summary>
    public IEnumerable<OptionsRow> AllRows()
    {
        foreach (OptionsTab tab in Tabs)
        {
            foreach (OptionsRow row in _rows[tab])
            {
                yield return row;
            }
        }
    }

    /// <summary>Whether the window shows <paramref name="descriptor"/>.</summary>
    public static bool IsShown(SettingDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);

        if (ExcludedKeys.Contains(descriptor.Key))
        {
            return false;
        }

        if (IncludedProfileKeys.Contains(descriptor.Key))
        {
            return true;
        }

        return descriptor.Scope == SettingScope.User && IsEditable(descriptor);
    }

    /// <summary>
    /// The tab a setting belongs to, or null when the theme is out of scope — VR, the profile
    /// editor, movie recording and the derived internals (REQ-PERS-011, REQ-PERS-016).
    /// </summary>
    public static OptionsTab? TabOf(SettingDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);

        // The global shadow switch belongs with the light, which is what the tab "Lumière et
        // ombres" of REQ-UI-202 is for; the rest of the Render theme is video.
        if (descriptor.Key == SettingKeys.RenderShadow)
        {
            return OptionsTab.Light;
        }

        return descriptor.Theme switch
        {
            SettingTheme.Video or SettingTheme.Render or SettingTheme.Scene => OptionsTab.Video,
            SettingTheme.Camera => OptionsTab.Camera,
            SettingTheme.Light => OptionsTab.Light,
            SettingTheme.Audio => OptionsTab.Audio,
            SettingTheme.Emulation or SettingTheme.Input or SettingTheme.Conversion
                or SettingTheme.Clipping or SettingTheme.Layers => OptionsTab.Emulation,
            SettingTheme.Interface => OptionsTab.Interface,
            _ => null,
        };
    }

    /// <summary>
    /// The settings of the Réglages page that change value without any interface sound
    /// (AMD-021 R2-013, which replaces the note and the table of SPEC-11 REQ-AUD-041).
    /// </summary>
    public static IReadOnlyList<string> SilentKeys { get; } =
        [SettingKeys.EmuGameSpeed, SettingKeys.CameraFov];

    /// <summary>
    /// Whether changing the value of <paramref name="key"/> plays the validation sound
    /// (AMD-021 R2-013). Speed and FOV are silent; every other setting of the page plays it,
    /// exactly once per change of value.
    /// </summary>
    public static bool PlaysValidationSound(string key)
    {
        ArgumentNullException.ThrowIfNull(key);
        return !SilentKeys.Contains(key);
    }

    /// <summary>
    /// The "Valeurs par défaut" button of REQ-UI-202: every row of the tab goes back to the
    /// default of SPEC-09 REQ-PERS-002, through the model so the change is persisted and
    /// notified like any other.
    /// </summary>
    public void ResetTab(SettingsModel model, OptionsTab tab)
    {
        ArgumentNullException.ThrowIfNull(model);

        foreach (OptionsRow row in _rows[tab])
        {
            model.Reset(row.Descriptor.Key);
        }
    }

    /// <summary>
    /// Whether the window has a widget for <paramref name="descriptor"/>. A text setting only
    /// qualifies when its values are a finite list, which is the drop-down <c>ui.language</c>
    /// of AMD-035 needs; a free text field — <c>ui.last_browse_dir</c> — stays out.
    /// </summary>
    private static bool IsEditable(SettingDescriptor descriptor) => descriptor.Kind switch
    {
        SettingKind.Bool or SettingKind.Int or SettingKind.Float
            or SettingKind.Enum or SettingKind.Color => true,
        SettingKind.String => descriptor.TextOptions is { Count: > 0 },
        _ => false,
    };

    private static OptionsRow BuildRow(SettingDescriptor descriptor)
    {
        (double minimum, double maximum, double step) = Bounds(descriptor);
        return new OptionsRow(descriptor, minimum, maximum, step);
    }

    private static (double Minimum, double Maximum, double Step) Bounds(SettingDescriptor descriptor)
    {
        if (descriptor.Kind is not (SettingKind.Int or SettingKind.Float))
        {
            return (0d, 0d, 0d);
        }

        if (descriptor.LibretroValues is { Count: > 1 } values)
        {
            double first = values[0].Value;
            double last = values[^1].Value;
            double step = Math.Abs(values[1].Value - first);
            return (Math.Min(first, last), Math.Max(first, last), step);
        }

        if (descriptor.Domain is { Minimum: { } low, Maximum: { } high })
        {
            return (low, high, 0d);
        }

        // No bounds at all: the widget becomes a free numeric field.
        return (0d, 0d, 0d);
    }
}
