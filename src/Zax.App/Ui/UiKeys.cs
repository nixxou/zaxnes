namespace Zax.App.Ui;

/// <summary>
/// The key of every interface text of AMD-035. Both tables of <see cref="UiStrings"/> hold
/// exactly these keys, which a test of <c>UiLanguageTests</c> checks.
/// </summary>
/// <remarks>
/// The keys read like the place the text appears — <c>menu.</c>, <c>window.</c>, <c>button.</c>,
/// <c>cmd.</c> for a command of the model of REQ-UI-205, <c>setting.</c> for the label of a
/// setting of SPEC-09 §4.1 — so that a new screen names its texts without having to invent a
/// scheme. A <c>cmd.</c> key is the command's own stable name, which keeps the two in step.
/// </remarks>
internal static class UiKeys
{
    // ----------------------------------------------------------- menu bar

    public const string MenuFile = "menu.file";
    public const string MenuEmulation = "menu.emulation";
    public const string MenuVideo = "menu.video";
    public const string MenuAudio = "menu.audio";
    public const string MenuTools = "menu.tools";

    public const string MenuRecentRoms = "menu.recent_roms";
    public const string MenuLoadState = "menu.load_state";
    public const string MenuSaveState = "menu.save_state";
    public const string MenuDeleteState = "menu.delete_state";
    public const string MenuSpeed = "menu.speed";
    public const string MenuRegion = "menu.region";
    public const string MenuPorts = "menu.ports";
    public const string MenuCameraMode = "menu.camera_mode";
    public const string MenuVolume = "menu.volume";
    public const string MenuNothingHere = "menu.nothing_here";

    public const string MenuRegionAuto = "menu.region.auto";
    public const string MenuPort = "menu.port";
    public const string MenuPortController = "menu.port.controller";
    public const string MenuPortZapper = "menu.port.zapper";
    public const string MenuCameraOrbital = "menu.camera.orbital";
    public const string MenuCameraFirstPerson = "menu.camera.first_person";

    public const string SlotEmpty = "slot.empty";
    public const string SlotQuick = "slot.quick";
    public const string SlotPrefix = "slot.prefix";

    // -------------------------------------------------------- window titles

    public const string WindowOptions = "window.options";
    public const string WindowKeyConfig = "window.key_config";
    public const string WindowAbout = "window.about";
    public const string WindowProfileChooser = "window.profile_chooser";
    public const string WindowOpenRom = "window.open_rom";
    public const string WindowImportProfile = "window.import_profile";
    public const string WindowImportCover = "window.import_cover";
    public const string WindowImportThreeDSen = "window.import_3dsen";

    // -------------------------------------------------------------- buttons

    public const string ButtonClose = "button.close";
    public const string ButtonCancel = "button.cancel";
    public const string ButtonOpen = "button.open";
    public const string ButtonLaunch = "button.launch";
    public const string ButtonParentFolder = "button.parent_folder";
    public const string ButtonResetDefaults = "button.reset_defaults";
    public const string ButtonResetBindings = "button.reset_bindings";
    public const string ButtonStopBinding = "button.stop_binding";
    public const string ButtonYes = "button.yes";
    public const string ButtonNo = "button.no";

    // ------------------- notifications of SPEC-10 §3.10, whose English text the spec fixes
    //                     word for word (REQ-UI-082 to REQ-UI-084, AMD-036 §1)

    public const string NotifyRollBack = "notify.roll_back";
    public const string NotifyFastForward = "notify.fast_forward";
    public const string NotifyQuickSave = "notify.quick_save";
    public const string NotifyQuickLoad = "notify.quick_load";

    public const string NotifyCorruptedZip = "notify.corrupted_zip";
    public const string NotifyUncompressionFailed = "notify.uncompression_failed";
    public const string NotifyInvalidRom = "notify.invalid_rom";
    public const string NotifyGameMissing = "notify.game_missing";
    public const string NotifyForeignCore = "notify.foreign_core";

    public const string NotifyNotA3dnFile = "notify.not_a_3dn_file";
    public const string NotifyUnsupported3dnVersion = "notify.unsupported_3dn_version";
    public const string NotifyObsoleteProfileFormat = "notify.obsolete_profile_format";

    // ------------- notifications of the persisted files (SPEC-09 REQ-PERS-022, REQ-PERS-024)

    public const string NotifySettingsUnreadable = "notify.settings_unreadable";
    public const string NotifySettingsNewerVersion = "notify.settings_newer_version";
    public const string NotifyGameListUnreadable = "notify.game_list_unreadable";
    public const string NotifyGameListNewerVersion = "notify.game_list_newer_version";
    public const string NotifyBindingsUnreadable = "notify.bindings_unreadable";
    public const string NotifyBindingsNewerVersion = "notify.bindings_newer_version";

    /// <summary>The end-of-import summary of AMD-030 §5, translated by AMD-036 §1.</summary>
    public const string NotifyThreeDSenSummary = "notify.import_3dsen_summary";

    // ------------------- confirmations of SPEC-09 REQ-PERS-042 and REQ-PERS-054 (AMD-036 §1)

    public const string ConfirmDeleteGame = "confirm.delete_game";
    public const string ConfirmDeleteSlot = "confirm.delete_slot";

    // --------------------------------------------------------------- hints

    public const string HintImportThreeDSen = "hint.import_3dsen";
    public const string HintResScaleDirect = "hint.res_scale_direct";
    public const string HintPressAControl = "hint.press_a_control";
    public const string HintClickACell = "hint.click_a_cell";
    public const string HintAtNextStart = "hint.at_next_start";
    public const string HintAtNextLoad = "hint.at_next_load";
    public const string HintProfileSetting = "hint.profile_setting";

    // ------------------------------------------------------- fields, columns

    public const string FieldSearch = "field.search";
    public const string ColumnAction = "column.action";
    public const string ColumnKeyboard = "column.keyboard";
    public const string ColumnGamepad = "column.gamepad";
    public const string ValueNoGamepad = "value.no_gamepad";

    // ------------------------------------------------------- welcome screen

    public const string WelcomeHint = "welcome.hint";
    public const string WelcomeRecent = "welcome.recent";
    public const string WelcomeNoRecent = "welcome.no_recent";

    // ----------------------------------------------------------- about block

    public const string AboutLicence = "about.licence";
    public const string AboutCreditInspired = "about.credit.inspired";
    public const string AboutCreditAdapted = "about.credit.adapted";
    public const string AboutCreditDisclaimer = "about.credit.disclaimer";
    public const string AboutVersionPrefix = "about.version_prefix";

    // ----------------------------------------------------- options window tabs

    public const string TabVideo = "options.tab.video";
    public const string TabCamera = "options.tab.camera";
    public const string TabLight = "options.tab.light";
    public const string TabAudio = "options.tab.audio";
    public const string TabEmulation = "options.tab.emulation";
    public const string TabInterface = "options.tab.interface";

    // ------------------------------------------------- key configuration sections

    public const string KeysPlayerOne = "keys.section.player_one";
    public const string KeysPlayerTwo = "keys.section.player_two";
    public const string KeysCamera = "keys.section.camera";
    public const string KeysHotkeys = "keys.section.hotkeys";
    public const string KeysCommands = "keys.section.commands";

    // --------------------------------------------------------- file browser

    public const string BrowserComputer = "browser.computer";
    public const string BrowserFilterNesRoms = "browser.filter.nes_roms";
    public const string BrowserFilterNesFiles = "browser.filter.nes_files";
    public const string BrowserFilterZip = "browser.filter.zip";
    public const string BrowserFilterProfiles = "browser.filter.profiles";
    public const string BrowserFilterBundle = "browser.filter.bundle";
    public const string BrowserFilterPng = "browser.filter.png";
    public const string BrowserFilterAll = "browser.filter.all";

    // ------------------------------------------------------- profile chooser

    public const string ProfileTwoDimensional = "profile.two_dimensional";
    public const string ProfileSourceTwoDimensional = "profile.source.two_dimensional";
    public const string ProfileSourceDefaultThreeD = "profile.source.default_3d";
    public const string ProfileSourceBuiltIn = "profile.source.built_in";
    public const string ProfileSourceProgramFolder = "profile.source.program_folder";
    public const string ProfileSourceUser = "profile.source.user";

    // ------------------------------------------------------------- shortcuts

    public const string ShortcutAltEnter = "shortcut.alt_enter";
    public const string ShortcutShiftPrefix = "shortcut.shift_prefix";
    public const string ShortcutQuickSave = "shortcut.quick_save";
    public const string ShortcutQuickLoad = "shortcut.quick_load";
    public const string ShortcutFastForward = "shortcut.fast_forward";
    public const string ShortcutRollback = "shortcut.rollback";
    public const string ShortcutSaveSlots = "shortcut.save_slots";
    public const string ShortcutLoadSlots = "shortcut.load_slots";
    public const string ShortcutLongPressMenu = "shortcut.long_press_menu";

    // ------------------------------------------------- commands of REQ-UI-205

    public const string CommandPrefix = "cmd.";

    /// <summary>The label key of the command named <paramref name="command"/>.</summary>
    public static string Command(string command) => CommandPrefix + command;

    // --------------------------------------------- labels of SPEC-09 settings

    public const string SettingPrefix = "setting.";

    /// <summary>The label key of the setting whose stable key is <paramref name="key"/>.</summary>
    public static string Setting(string key) => SettingPrefix + key;

    // ------------------------------------------- labels of enumerated values

    public const string OptionPrefix = "option.";

    /// <summary>The label key of the domain value <paramref name="label"/> of SPEC-09 §4.1.</summary>
    public static string Option(string label) => OptionPrefix + label;
}
