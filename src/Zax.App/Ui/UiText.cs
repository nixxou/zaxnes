using System.Globalization;
using System.Reflection;
using Zax.Settings;

namespace Zax.App.Ui;

/// <summary>
/// The visible French text of the classic interface (AMD-010): menu titles, window titles,
/// buttons and the French label of each setting the options window shows.
/// </summary>
/// <remarks>
/// <para>
/// The <b>notification</b> texts are deliberately absent: SPEC-10 REQ-UI-082, REQ-UI-083 and
/// REQ-UI-084 fix them word for word, in English, and AMD-010 §3 keeps them normative. They are
/// produced by the emulation loop and by the profile reader, and are shown verbatim.
/// </para>
/// <para>
/// The labels of the 19 logical actions are likewise left in English: SPEC-08 §4.1 publishes them
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
    public const string MenuFile = "Fichier";

    /// <summary>Title of the Emulation menu.</summary>
    public const string MenuEmulation = "Émulation";

    /// <summary>Title of the Video menu.</summary>
    public const string MenuVideo = "Vidéo";

    /// <summary>Title of the Audio menu.</summary>
    public const string MenuAudio = "Audio";

    /// <summary>Title of the Tools menu.</summary>
    public const string MenuTools = "Outils";

    // ------------------------------------------------------------ menu entries

    /// <summary>Label of the recent ROM submenu (REQ-UI-200).</summary>
    public const string RecentRoms = "ROMs récentes";

    /// <summary>Label of the load-state submenu.</summary>
    public const string LoadState = "Charger un état";

    /// <summary>Label of the save-state submenu.</summary>
    public const string SaveState = "Sauvegarder l'état";

    /// <summary>Label of the submenu that deletes a state (REQ-UI-052).</summary>
    public const string DeleteState = "Supprimer un état";

    /// <summary>Label of the speed submenu.</summary>
    public const string Speed = "Vitesse";

    /// <summary>Label of the region submenu.</summary>
    public const string Region = "Région";

    /// <summary>Label of the port submenu.</summary>
    public const string Ports = "Ports";

    /// <summary>Label of the camera mode submenu.</summary>
    public const string CameraMode = "Mode caméra";

    /// <summary>Label of the volume submenu.</summary>
    public const string Volume = "Volume";

    /// <summary>Shown in place of an empty submenu.</summary>
    public const string NothingHere = "(vide)";

    /// <summary>What an empty save slot shows (REQ-UI-208).</summary>
    public const string EmptySlot = "vide";

    /// <summary>Label of the quick slot in the state submenus (REQ-UI-208).</summary>
    public const string QuickSlot = "État rapide";

    /// <summary>Prefix of a numbered slot in the state submenus.</summary>
    public const string SlotPrefix = "Emplacement";

    // ---------------------------------------------------------- window titles

    /// <summary>Title of the options window (REQ-UI-202).</summary>
    public const string OptionsTitle = "Options";

    /// <summary>Title of the key configuration window (REQ-UI-203).</summary>
    public const string KeyConfigTitle = "Configuration des touches";

    /// <summary>Title of the about window (DEC-10-Q3).</summary>
    public const string AboutTitle = "À propos";

    /// <summary>Title of the profile chooser (REQ-UI-201).</summary>
    public const string ProfileChooserTitle = "Choisir le profil 3D";

    /// <summary>Title of the ROM file dialog (REQ-UI-201, SPEC-10 §4).</summary>
    public const string OpenRomTitle = "Ouvrir une ROM";

    /// <summary>Title of the profile import dialog (SPEC-10 §4).</summary>
    public const string ImportProfileTitle = "Importer un profil 3D";

    /// <summary>Title of the cover-art import dialog (REQ-PERS-036).</summary>
    public const string ImportCoverTitle = "Importer une jaquette";

    // ------------------------------------------------------------- misc labels

    /// <summary>The "reset this tab to its defaults" button of REQ-UI-202.</summary>
    public const string ResetDefaults = "Valeurs par défaut";

    /// <summary>Generic close button.</summary>
    public const string Close = "Fermer";

    /// <summary>Generic cancel button.</summary>
    public const string Cancel = "Annuler";

    /// <summary>Validation button of the file dialog.</summary>
    public const string Open = "Ouvrir";

    /// <summary>Search field hint.</summary>
    public const string Search = "Rechercher";

    /// <summary>Column header of the keyboard bindings.</summary>
    public const string KeyboardColumn = "Clavier";

    /// <summary>Column header of the gamepad bindings.</summary>
    public const string GamepadColumn = "Manette";

    /// <summary>Button that interrupts a binding capture (REQ-UI-072).</summary>
    public const string StopBinding = "Arrêter la capture";

    /// <summary>Hint shown while a capture is listening.</summary>
    public const string PressAControl = "Appuyez sur une touche ou un bouton…";

    /// <summary>The "no gamepad" entry of the pad chooser (REQ-UI-070).</summary>
    public const string NoGamepad = "(aucune manette)";

    /// <summary>The 2D Mode entry of the profile chooser (SPEC-10 §2).</summary>
    public const string TwoDimensionalMode = "2D Mode";

    // ---------------------------------------------------------- welcome screen

    /// <summary>The call to action of REQ-UI-207.</summary>
    public const string WelcomeHint = "Fichier ▸ Ouvrir une ROM";

    /// <summary>Heading above the recent ROM list of the welcome screen.</summary>
    public const string WelcomeRecent = "ROMs récentes";

    /// <summary>Shown on the welcome screen when no ROM has ever been opened.</summary>
    public const string WelcomeNoRecent = "Aucune ROM n'a encore été ouverte.";

    /// <summary>
    /// The version the about block shows. The build metadata the compiler appends to the
    /// informational version — a plus sign and the commit hash — is left out.
    /// </summary>
    /// <remarks>
    /// Declared before <see cref="AboutLines"/> on purpose: static initialisers run in the order
    /// they are written, so a version read afterwards would reach the about block empty.
    /// </remarks>
    public static string Version { get; } = Shorten(
        typeof(UiText).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()
            ?.InformationalVersion
        ?? typeof(UiText).Assembly.GetName().Version?.ToString(3)
        ?? "0.0.0");

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
    /// sources themselves (checked by <c>ArchitectureTests</c>).
    /// </remarks>
    public static IReadOnlyList<string> AboutLines { get; } =
    [
        ProductName,
        string.Create(CultureInfo.InvariantCulture, $"Version {Version}"),
        "GPL v3",
        "Projet indépendant inspiré de 3dSen (Geod Studio) ;",
        "contient du code adapté de 3DNes_Engine (Geod Studio, GPL v3).",
        "Sans lien avec Geod Studio ni Nintendo.",
    ];

    private static string Shorten(string version)
    {
        int plus = version.IndexOf('+', StringComparison.Ordinal);
        return plus < 0 ? version : version[..plus];
    }

    /// <summary>
    /// The French label of <paramref name="descriptor"/> for the options window. The settings
    /// SPEC-10 REQ-UI-061 made visible keep the meaning of their original label; the ones the
    /// original hid (P3 in REQ-UI-202) get a descriptive one.
    /// </summary>
    public static string LabelOf(SettingDescriptor descriptor)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        return descriptor.Key switch
        {
            SettingKeys.VideoFrameRate => "Fréquence de rendu",
            SettingKeys.VideoSkipFrame => "Rendu accéléré (saut d'images)",
            SettingKeys.VideoVSync => "Synchronisation verticale",
            SettingKeys.VideoRenderMethod => "Méthode de rendu",
            SettingKeys.VideoResScale => "Échelle de résolution",
            SettingKeys.VideoAntiAlias => "Anticrénelage",
            SettingKeys.RenderMeshAlgo => "Algorithme de maillage",
            SettingKeys.RenderShadow => "Ombres",
            SettingKeys.CameraFov => "Champ de vision (degrés)",
            SettingKeys.LightIntensity => "Intensité lumineuse",
            SettingKeys.SceneSkybox => "Fond (skybox)",
            SettingKeys.SceneCustomBgColor => "Couleur de fond",
            SettingKeys.SceneTvBox => "Boîtier TV",
            SettingKeys.EmuGameSpeed => "Vitesse de jeu",
            SettingKeys.EmuAutoTvMode => "Région détectée automatiquement",
            SettingKeys.EmuTvMode => "Région vidéo",
            SettingKeys.EmuCore => "Cœur d'émulation",
            SettingKeys.AudioVolume => "Volume général",
            SettingKeys.AudioBgMusic => "Musique de menu",
            SettingKeys.AudioMute => "Muet",
            SettingKeys.InputEmulatedMenuButton => "Menu = Select + Start",
            SettingKeys.UiFullscreen => "Plein écran",
            SettingKeys.UiShowFps => "Afficher les FPS",
            SettingKeys.UiSortOption => "Tri de la bibliothèque",
            _ => descriptor.UiLabel ?? descriptor.Key,
        };
    }

    /// <summary>The French label of an enumerated value, or the English one when there is none.</summary>
    public static string LabelOf(SettingDescriptor descriptor, string option)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        return option switch
        {
            "Off" => "Désactivée",
            "Every refresh" => "À chaque rafraîchissement",
            "Every other refresh" => "Un rafraîchissement sur deux",
            "Default" => "Par défaut",
            "Color" => "Couleur",
            "Gradient" => "Dégradé",
            "Fantasy" => "Fantaisie",
            "None" when descriptor.Key == SettingKeys.SceneTvBox => "Aucun",
            "None" => "Aucun",
            "Native" => "Native",
            "enabled" => "activées",
            "disabled" => "désactivées",
            "A-Z" => "A-Z",
            "Recent" => "Récent",
            "NTSC" or "PAL" or "DENDY" or "MULTI" => option,
            _ => option,
        };
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
