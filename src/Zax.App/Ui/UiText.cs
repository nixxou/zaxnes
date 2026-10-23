using System.Globalization;
using System.Reflection;
using Zax.Settings;

namespace Zax.App.Ui;

/// <summary>
/// The named view onto the interface texts of AMD-035: one property per text the code needs,
/// each reading the table of the language in force through <see cref="UiStrings.Get"/>.
/// </summary>
/// <remarks>
/// <para>
/// Every member here is a <b>property</b>, never a constant: AMD-035 §1 has the language change
/// apply immediately, and a constant would be baked into the caller at compile time. The models
/// that cache a list of labels watch <see cref="UiStrings.Generation"/> instead.
/// </para>
/// <para>
/// The <b>notification</b> texts have no property here: they are produced by the emulation loop,
/// by the profile reader and by the settings files, in the exact English SPEC-10 REQ-UI-082 to
/// REQ-UI-084 and SPEC-09 fix word for word. AMD-036 §1 makes that English the table entry and
/// has <see cref="UiNotifications"/> translate the message where it is shown.
/// </para>
/// <para>
/// The labels of the 19 logical actions are left in English: SPEC-08 §4.1 publishes them
/// (<c>RotLeft</c>, <c>QuickSave</c>…) as the names of the actions, the key configuration screen
/// shows them (REQ-UI-070) and the binding file persists them (REQ-PERS-065).
/// </para>
/// </remarks>
internal static class UiText
{
    /// <summary>Product name shown in the title bar and the about block (DEC-10-Q3).</summary>
    public const string ProductName = "Zaxnes";

    // ------------------------------------------------------------- menu titles

    /// <summary>Title of the File menu (REQ-UI-200).</summary>
    public static string MenuFile => UiStrings.Get(UiKeys.MenuFile);

    /// <summary>Title of the Emulation menu.</summary>
    public static string MenuEmulation => UiStrings.Get(UiKeys.MenuEmulation);

    /// <summary>Title of the Video menu.</summary>
    public static string MenuVideo => UiStrings.Get(UiKeys.MenuVideo);

    /// <summary>Title of the Audio menu.</summary>
    public static string MenuAudio => UiStrings.Get(UiKeys.MenuAudio);

    /// <summary>Title of the Tools menu.</summary>
    public static string MenuTools => UiStrings.Get(UiKeys.MenuTools);

    // ------------------------------------------------------------ menu entries

    /// <summary>Label of the recent ROM submenu (REQ-UI-200).</summary>
    public static string RecentRoms => UiStrings.Get(UiKeys.MenuRecentRoms);

    /// <summary>Label of the load-state submenu.</summary>
    public static string LoadState => UiStrings.Get(UiKeys.MenuLoadState);

    /// <summary>Label of the save-state submenu.</summary>
    public static string SaveState => UiStrings.Get(UiKeys.MenuSaveState);

    /// <summary>Label of the submenu that deletes a state (REQ-UI-052).</summary>
    public static string DeleteState => UiStrings.Get(UiKeys.MenuDeleteState);

    /// <summary>Label of the speed submenu.</summary>
    public static string Speed => UiStrings.Get(UiKeys.MenuSpeed);

    /// <summary>Label of the region submenu.</summary>
    public static string Region => UiStrings.Get(UiKeys.MenuRegion);

    /// <summary>Label of the port submenu.</summary>
    public static string Ports => UiStrings.Get(UiKeys.MenuPorts);

    /// <summary>Label of the camera mode submenu.</summary>
    public static string CameraMode => UiStrings.Get(UiKeys.MenuCameraMode);

    /// <summary>Label of the volume submenu.</summary>
    public static string Volume => UiStrings.Get(UiKeys.MenuVolume);

    /// <summary>Shown in place of an empty submenu.</summary>
    public static string NothingHere => UiStrings.Get(UiKeys.MenuNothingHere);

    /// <summary>The automatic region entry of the Region submenu.</summary>
    public static string RegionAuto => UiStrings.Get(UiKeys.MenuRegionAuto);

    /// <summary>The word a port submenu is titled with (REQ-INP-060).</summary>
    public static string Port => UiStrings.Get(UiKeys.MenuPort);

    /// <summary>The Controller entry of a port submenu.</summary>
    public static string PortController => UiStrings.Get(UiKeys.MenuPortController);

    /// <summary>The Zapper entry of a port submenu.</summary>
    public static string PortZapper => UiStrings.Get(UiKeys.MenuPortZapper);

    /// <summary>The orbital camera entry of the camera submenu (SPEC-06 REQ-SCN-006).</summary>
    public static string CameraOrbital => UiStrings.Get(UiKeys.MenuCameraOrbital);

    /// <summary>The first-person camera entry of the camera submenu.</summary>
    public static string CameraFirstPerson => UiStrings.Get(UiKeys.MenuCameraFirstPerson);

    /// <summary>What an empty save slot shows (REQ-UI-208).</summary>
    public static string EmptySlot => UiStrings.Get(UiKeys.SlotEmpty);

    /// <summary>Label of the quick slot in the state submenus (REQ-UI-208).</summary>
    public static string QuickSlot => UiStrings.Get(UiKeys.SlotQuick);

    /// <summary>Prefix of a numbered slot in the state submenus.</summary>
    public static string SlotPrefix => UiStrings.Get(UiKeys.SlotPrefix);

    // ---------------------------------------------------------- window titles

    /// <summary>Title of the options window (REQ-UI-202).</summary>
    public static string OptionsTitle => UiStrings.Get(UiKeys.WindowOptions);

    /// <summary>Title of the key configuration window (REQ-UI-203).</summary>
    public static string KeyConfigTitle => UiStrings.Get(UiKeys.WindowKeyConfig);

    /// <summary>Title of the about window (DEC-10-Q3).</summary>
    public static string AboutTitle => UiStrings.Get(UiKeys.WindowAbout);

    /// <summary>Title of the profile chooser (REQ-UI-201).</summary>
    public static string ProfileChooserTitle => UiStrings.Get(UiKeys.WindowProfileChooser);

    /// <summary>Title of the ROM file dialog (REQ-UI-201, SPEC-10 §4).</summary>
    public static string OpenRomTitle => UiStrings.Get(UiKeys.WindowOpenRom);

    /// <summary>Title of the profile import dialog (SPEC-10 §4).</summary>
    public static string ImportProfileTitle => UiStrings.Get(UiKeys.WindowImportProfile);

    /// <summary>Title of the cover-art import dialog (REQ-PERS-036).</summary>
    public static string ImportCoverTitle => UiStrings.Get(UiKeys.WindowImportCover);

    /// <summary>Title of the 3dSen installation picker (AMD-030 §2).</summary>
    public static string ImportThreeDSenTitle => UiStrings.Get(UiKeys.WindowImportThreeDSen);

    /// <summary>
    /// What the 3dSen picker asks for: the installation folder, or the bundle itself
    /// (AMD-030 §2, §3).
    /// </summary>
    public static string ImportThreeDSenHint => UiStrings.Get(UiKeys.HintImportThreeDSen);

    // ------------------------------------------------------------- misc labels

    /// <summary>The "reset this tab to its defaults" button of REQ-UI-202.</summary>
    public static string ResetDefaults => UiStrings.Get(UiKeys.ButtonResetDefaults);

    /// <summary>
    /// Why the resolution scale is greyed out under the direct render method (AMD-032 §3).
    /// </summary>
    public static string ResScaleNeedsRenderTexture => UiStrings.Get(UiKeys.HintResScaleDirect);

    /// <summary>Generic close button.</summary>
    public static string Close => UiStrings.Get(UiKeys.ButtonClose);

    /// <summary>Generic cancel button.</summary>
    public static string Cancel => UiStrings.Get(UiKeys.ButtonCancel);

    /// <summary>Validation button of the file dialog.</summary>
    public static string Open => UiStrings.Get(UiKeys.ButtonOpen);

    /// <summary>Validation button of the profile chooser.</summary>
    public static string Launch => UiStrings.Get(UiKeys.ButtonLaunch);

    /// <summary>The "go up one folder" button of the file dialog.</summary>
    public static string ParentFolder => UiStrings.Get(UiKeys.ButtonParentFolder);

    /// <summary>Search field hint.</summary>
    public static string Search => UiStrings.Get(UiKeys.FieldSearch);

    /// <summary>Column header of the bound action.</summary>
    public static string ActionColumn => UiStrings.Get(UiKeys.ColumnAction);

    /// <summary>Column header of the keyboard bindings.</summary>
    public static string KeyboardColumn => UiStrings.Get(UiKeys.ColumnKeyboard);

    /// <summary>Column header of the gamepad bindings.</summary>
    public static string GamepadColumn => UiStrings.Get(UiKeys.ColumnGamepad);

    /// <summary>Button that interrupts a binding capture (REQ-UI-072).</summary>
    public static string StopBinding => UiStrings.Get(UiKeys.ButtonStopBinding);

    /// <summary>Button that puts every binding back to its default (REQ-INP-049).</summary>
    public static string ResetBindings => UiStrings.Get(UiKeys.ButtonResetBindings);

    /// <summary>Hint shown while a capture is listening.</summary>
    public static string PressAControl => UiStrings.Get(UiKeys.HintPressAControl);

    /// <summary>Hint shown when no capture is running (REQ-INP-042).</summary>
    public static string ClickACell => UiStrings.Get(UiKeys.HintClickACell);

    /// <summary>The "no gamepad" entry of the pad chooser (REQ-UI-070).</summary>
    public static string NoGamepad => UiStrings.Get(UiKeys.ValueNoGamepad);

    /// <summary>The 2D Mode entry of the profile chooser (SPEC-10 §2).</summary>
    public static string TwoDimensionalMode => UiStrings.Get(UiKeys.ProfileTwoDimensional);

    /// <summary>Why a setting only takes effect at the next start (REQ-PERS-084).</summary>
    public static string AtNextStart => UiStrings.Get(UiKeys.HintAtNextStart);

    /// <summary>Why a setting only takes effect at the next load (REQ-PERS-084).</summary>
    public static string AtNextLoad => UiStrings.Get(UiKeys.HintAtNextLoad);

    /// <summary>That a setting is a profile parameter a profile load overwrites (REQ-PERS-005).</summary>
    public static string ProfileSetting => UiStrings.Get(UiKeys.HintProfileSetting);

    // ---------------------------------------------------------- welcome screen

    /// <summary>The call to action of REQ-UI-207.</summary>
    public static string WelcomeHint => UiStrings.Get(UiKeys.WelcomeHint);

    /// <summary>Heading above the recent ROM list of the welcome screen.</summary>
    public static string WelcomeRecent => UiStrings.Get(UiKeys.WelcomeRecent);

    /// <summary>Shown on the welcome screen when no ROM has ever been opened.</summary>
    public static string WelcomeNoRecent => UiStrings.Get(UiKeys.WelcomeNoRecent);

    /// <summary>
    /// The version the about block shows. The build metadata the compiler appends to the
    /// informational version — a plus sign and the commit hash — is left out.
    /// </summary>
    public static string Version { get; } = Shorten(
        typeof(UiText).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion
        ?? typeof(UiText).Assembly.GetName().Version?.ToString(3)
        ?? "0.0.0");

    /// <summary>The version line of the about block and of the welcome screen.</summary>
    public static string VersionLine => string.Create(
        CultureInfo.InvariantCulture,
        $"{UiStrings.Get(UiKeys.AboutVersionPrefix)} {Version}");

    /// <summary>
    /// The about block of DEC-10-Q3 / SPEC-10 REQ-UI-076, one line per entry: the product name,
    /// the version, the licence and the factual attribution.
    /// </summary>
    /// <remarks>
    /// The three attribution lines are the sentence AMD-029 §3 gives word for word, which
    /// closes Q-068: the block presents Zaxnes and credits what it owes — « 3dSen », the
    /// program the specification describes, and the upstream engine part of
    /// <c>Zax.Engine3D</c> is adapted from — then the disclaimer that ends it. The credit is
    /// UI text, not a claim about this file: the attribution header belongs on the adapted
    /// sources themselves (checked by <c>ArchitectureTests</c>). AMD-029 writes the sentence in
    /// French, so the French table is the normative one and the English table translates it.
    /// </remarks>
    public static IReadOnlyList<string> AboutLines =>
    [
        ProductName,
        VersionLine,
        UiStrings.Get(UiKeys.AboutLicence),
        UiStrings.Get(UiKeys.AboutCreditInspired),
        UiStrings.Get(UiKeys.AboutCreditAdapted),
        UiStrings.Get(UiKeys.AboutCreditDisclaimer),
    ];

    private static string Shorten(string version)
    {
        int plus = version.IndexOf('+', StringComparison.Ordinal);
        return plus < 0 ? version : version[..plus];
    }

    /// <summary>
    /// The label of <paramref name="descriptor"/> for the options window, in the language in
    /// force. The settings SPEC-10 REQ-UI-061 made visible keep the meaning of their original
    /// label; the ones the original hid (P3 in REQ-UI-202) get a descriptive one.
    /// </summary>
    public static string LabelOf(SettingDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);

        string key = UiKeys.Setting(descriptor.Key);
        string text = UiStrings.Get(key);
        return ReferenceEquals(text, key) || string.Equals(text, key, StringComparison.Ordinal)
            ? descriptor.UiLabel ?? descriptor.Key
            : text;
    }

    /// <summary>The label of an enumerated value, or the raw domain label when there is none.</summary>
    public static string LabelOf(SettingDescriptor descriptor, string option)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        ArgumentNullException.ThrowIfNull(option);

        string key = UiKeys.Option(option);
        string text = UiStrings.Get(key);
        return string.Equals(text, key, StringComparison.Ordinal) ? option : text;
    }

    /// <summary>The label of the speed setting <paramref name="setting"/> and its factor.</summary>
    public static string SpeedLabel(int setting)
    {
        double factor = Loop.SpeedSetting.Factor(setting);
        string sign = setting > 0 ? "+" : string.Empty;
        return string.Format(
            CultureInfo.InvariantCulture,
            "{0}{1}  (×{2:0.00})",
            sign,
            setting,
            factor);
    }
}
