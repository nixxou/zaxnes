using Zax.Settings;

namespace Zax.App.Ui;

/// <summary>
/// The English table of AMD-035, the default language of a fresh user folder and the fallback of
/// every other language (AMD-035 §1, §3).
/// </summary>
/// <remarks>
/// The style is the short, plain one AMD-035 §3 asks for: no final full stop on a label, a final
/// full stop on a sentence, and an ellipsis on the entries that open a dialog, which is the
/// convention the menu bar of REQ-UI-200 already used.
/// </remarks>
internal static partial class UiStrings
{
    private static readonly Dictionary<string, string> English = new(StringComparer.Ordinal)
    {
        // --------------------------------------------------------- menu bar
        [UiKeys.MenuFile] = "File",
        [UiKeys.MenuEmulation] = "Emulation",
        [UiKeys.MenuVideo] = "Video",
        [UiKeys.MenuAudio] = "Audio",
        [UiKeys.MenuTools] = "Tools",

        [UiKeys.MenuRecentRoms] = "Recent ROMs",
        [UiKeys.MenuLoadState] = "Load a state",
        [UiKeys.MenuSaveState] = "Save the state",
        [UiKeys.MenuDeleteState] = "Delete a state",
        [UiKeys.MenuSpeed] = "Speed",
        [UiKeys.MenuRegion] = "Region",
        [UiKeys.MenuPorts] = "Ports",
        [UiKeys.MenuCameraMode] = "Camera mode",
        [UiKeys.MenuVolume] = "Volume",
        [UiKeys.MenuNothingHere] = "(empty)",

        [UiKeys.MenuRegionAuto] = "Auto",
        [UiKeys.MenuPort] = "Port",
        [UiKeys.MenuPortController] = "Controller",
        [UiKeys.MenuPortZapper] = "Zapper",
        [UiKeys.MenuCameraOrbital] = "Orbit (TPV)",
        [UiKeys.MenuCameraFirstPerson] = "First person (FPV)",

        [UiKeys.SlotEmpty] = "empty",
        [UiKeys.SlotQuick] = "Quick state",
        [UiKeys.SlotPrefix] = "Slot",

        // ----------------------------------------------------- window titles
        [UiKeys.WindowOptions] = "Options",
        [UiKeys.WindowKeyConfig] = "Key configuration",
        [UiKeys.WindowAbout] = "About",
        [UiKeys.WindowProfileChooser] = "Choose the 3D profile",
        [UiKeys.WindowOpenRom] = "Open a ROM",
        [UiKeys.WindowImportProfile] = "Import a 3D profile",
        [UiKeys.WindowImportCover] = "Import a cover",
        [UiKeys.WindowImportThreeDSen] = "Import from 3dSen",

        // ----------------------------------------------------------- buttons
        [UiKeys.ButtonClose] = "Close",
        [UiKeys.ButtonCancel] = "Cancel",
        [UiKeys.ButtonOpen] = "Open",
        [UiKeys.ButtonLaunch] = "Launch",
        [UiKeys.ButtonParentFolder] = "Parent folder",
        [UiKeys.ButtonResetDefaults] = "Defaults",
        [UiKeys.ButtonResetBindings] = "Reset every key",
        [UiKeys.ButtonStopBinding] = "Stop the capture",
        [UiKeys.ButtonYes] = "Yes",
        [UiKeys.ButtonNo] = "No",

        // ------------------------ notifications of SPEC-10 REQ-UI-082 to REQ-UI-084
        // Word for word what the requirements write, oddities included: the two spaces
        // before « is » and the missing space after the path (REQ-UI-084), and the product
        // name AMD-029 §3 put at the end. AMD-036 §1 makes these the English entries.
        [UiKeys.NotifyRollBack] = "Roll Back",
        [UiKeys.NotifyFastForward] = "Fast Forward",
        [UiKeys.NotifyQuickSave] = "Quick Save",
        [UiKeys.NotifyQuickLoad] = "Quick Load",

        [UiKeys.NotifyCorruptedZip] = "Corrupted zip file",
        [UiKeys.NotifyUncompressionFailed] = "Uncompression is not successful",
        [UiKeys.NotifyInvalidRom] = "Invalid rom file.",
        [UiKeys.NotifyGameMissing] = "The game profile doesn't exist anymore.",
        [UiKeys.NotifyForeignCore] = "Save state used a different core.",

        [UiKeys.NotifyNotA3dnFile] = "This file is not a 3dn file",
        [UiKeys.NotifyUnsupported3dnVersion] = "3dn format version {0}  is not supported",
        [UiKeys.NotifyObsoleteProfileFormat] =
            "{0}has an obsolate or wrong profile format. "
            + "This profile is not compatible with the current version of Zaxnes.",

        // ------------- notifications of the persisted files (REQ-PERS-022, REQ-PERS-024)
        [UiKeys.NotifySettingsUnreadable] = "Settings file was unreadable and has been reset.",
        [UiKeys.NotifySettingsNewerVersion] = "Settings were created by a newer version.",
        [UiKeys.NotifyGameListUnreadable] = "Game list was unreadable and has been reset.",
        [UiKeys.NotifyGameListNewerVersion] = "Game list was created by a newer version.",
        [UiKeys.NotifyBindingsUnreadable] = "Input bindings were unreadable and have been reset.",
        [UiKeys.NotifyBindingsNewerVersion] = "Input bindings were created by a newer version.",

        // The direct translation of the French text of AMD-030 §5, which AMD-036 §1 makes
        // the English entry and the one the command line always prints.
        [UiKeys.NotifyThreeDSenSummary] =
            "3dSen: {0} profile(s) imported, {1} already present, {2} failed.",

        // ------------------- confirmations of REQ-PERS-042 and REQ-PERS-054, with the
        //                     space before the question mark the requirements write
        [UiKeys.ConfirmDeleteGame] = "Do you want to delete the game and its save states ?",
        [UiKeys.ConfirmDeleteSlot] = "Do you want to delete the slot {0} ?",

        // ------------------------------------------------------------- hints
        [UiKeys.HintImportThreeDSen] =
            "Choose the installation folder of 3dSen (the one holding 3dSen.exe), "
            + "or the .bundle profile file itself.",
        [UiKeys.HintResScaleDirect] = "(no effect in direct rendering: native resolution)",
        [UiKeys.HintPressAControl] = "Press a key or a button…",
        [UiKeys.HintClickACell] = "Click a cell to reassign it.",
        [UiKeys.HintAtNextStart] = "(at the next start)",
        [UiKeys.HintAtNextLoad] = "(at the next load)",
        [UiKeys.HintProfileSetting] = "(profile setting)",

        // ---------------------------------------------------- fields, columns
        [UiKeys.FieldSearch] = "Search",
        [UiKeys.ColumnAction] = "Action",
        [UiKeys.ColumnKeyboard] = "Keyboard",
        [UiKeys.ColumnGamepad] = "Gamepad",
        [UiKeys.ValueNoGamepad] = "(no gamepad)",

        // ---------------------------------------------------- welcome screen
        [UiKeys.WelcomeHint] = "File ▸ Open a ROM",
        [UiKeys.WelcomeRecent] = "Recent ROMs",
        [UiKeys.WelcomeNoRecent] = "No ROM has been opened yet.",

        // -------------------------------------------------------- about block
        [UiKeys.AboutLicence] = "GPL v3",
        [UiKeys.AboutCreditInspired] = "Independent project inspired by 3dSen (Geod Studio);",
        [UiKeys.AboutCreditAdapted] = "contains code adapted from 3DNes_Engine (Geod Studio, GPL v3).",
        [UiKeys.AboutCreditDisclaimer] = "Not affiliated with Geod Studio or Nintendo.",
        [UiKeys.AboutVersionPrefix] = "Version",

        // -------------------------------------------------- options window tabs
        [UiKeys.TabVideo] = "Video",
        [UiKeys.TabCamera] = "Camera",
        [UiKeys.TabLight] = "Light and shadows",
        [UiKeys.TabAudio] = "Audio",
        [UiKeys.TabEmulation] = "Emulation",
        [UiKeys.TabInterface] = "Interface",

        // ---------------------------------------- key configuration sections
        [UiKeys.KeysPlayerOne] = "Player 1 — NES buttons",
        [UiKeys.KeysPlayerTwo] = "Player 2 — NES buttons",
        [UiKeys.KeysCamera] = "Camera",
        [UiKeys.KeysHotkeys] = "System hotkeys",
        [UiKeys.KeysCommands] = "Menu commands",

        // ------------------------------------------------------ file browser
        [UiKeys.BrowserComputer] = "This computer",
        [UiKeys.BrowserFilterNesRoms] = "NES ROMs (*.nes, *.zip)",
        [UiKeys.BrowserFilterNesFiles] = ".nes files",
        [UiKeys.BrowserFilterZip] = ".zip archives",
        [UiKeys.BrowserFilterProfiles] = "3D profiles (*.3dn)",
        [UiKeys.BrowserFilterBundle] = "3dSen profile bundle (*.bundle)",
        [UiKeys.BrowserFilterPng] = "PNG images (*.png)",
        [UiKeys.BrowserFilterAll] = "All files",

        // ---------------------------------------------------- profile chooser
        [UiKeys.ProfileTwoDimensional] = "2D Mode",
        [UiKeys.ProfileSourceTwoDimensional] = "no 3D conversion",
        [UiKeys.ProfileSourceDefaultThreeD] = "default 3D geometry",
        [UiKeys.ProfileSourceBuiltIn] = "built-in profile",
        [UiKeys.ProfileSourceProgramFolder] = "profile from the program folder",
        [UiKeys.ProfileSourceUser] = "user profile",

        // ---------------------------------------------------------- shortcuts
        [UiKeys.ShortcutAltEnter] = "Alt+Enter",
        [UiKeys.ShortcutShiftPrefix] = "Shift",
        [UiKeys.ShortcutQuickSave] = "QuickSave shortcut",
        [UiKeys.ShortcutQuickLoad] = "QuickLoad shortcut",
        [UiKeys.ShortcutFastForward] = "FastForward shortcut",
        [UiKeys.ShortcutRollback] = "RollBack shortcut",
        [UiKeys.ShortcutSaveSlots] = "Shift+F1…F10",
        [UiKeys.ShortcutLoadSlots] = "Ctrl+F1…F10",
        [UiKeys.ShortcutLongPressMenu] = "long press on Menu",

        // ------------------------------------------- commands of REQ-UI-205
        ["cmd.file.open_rom"] = "Open a ROM…",
        ["cmd.file.open_recent_rom"] = "Recent ROMs",
        ["cmd.file.choose_profile"] = "Choose the 3D profile…",
        ["cmd.file.import_profile"] = "Import a profile…",
        ["cmd.file.import_3dsen"] = "Import from 3dSen…",
        ["cmd.file.launch_game"] = "Launch the game",
        ["cmd.file.delete_game"] = "Delete the game…",
        ["cmd.file.create_game_art"] = "Create the cover",
        ["cmd.file.import_cover"] = "Import a cover…",
        ["cmd.app.quit"] = "Quit",
        ["cmd.state.save_slot"] = "Save the state",
        ["cmd.state.load_slot"] = "Load a state",
        ["cmd.state.quick_save"] = "Quick state: save",
        ["cmd.state.quick_load"] = "Quick state: load",
        ["cmd.state.delete_slot"] = "Delete the state",
        ["cmd.emulation.pause_resume"] = "Pause / Resume",
        ["cmd.emulation.pause"] = "Pause",
        ["cmd.emulation.resume"] = "Resume",
        ["cmd.emulation.reset"] = "Reset",
        ["cmd.emulation.stop"] = "Stop",
        ["cmd.emulation.fast_forward"] = "Fast forward",
        ["cmd.emulation.rollback"] = "Rollback",
        ["cmd.emulation.set_speed"] = "Speed",
        ["cmd.emulation.speed_up"] = "Faster",
        ["cmd.emulation.speed_down"] = "Slower",
        ["cmd.emulation.set_region"] = "Region",
        ["cmd.emulation.auto_region"] = "Region: Auto",
        ["cmd.emulation.set_port_type"] = "Port type",
        ["cmd.video.toggle_2d"] = "3D mode / 2D mode",
        ["cmd.video.reset_view"] = "Reset the view",
        ["cmd.video.set_camera_mode"] = "Camera mode",
        ["cmd.video.toggle_local_camera"] = "Top-down view",
        ["cmd.video.set_skybox"] = "Skybox",
        ["cmd.video.toggle_shadows"] = "Shadows",
        ["cmd.video.toggle_fullscreen"] = "Full screen",
        ["cmd.video.toggle_fps"] = "Show the FPS",
        ["cmd.audio.set_volume"] = "Volume",
        ["cmd.audio.volume_up"] = "Volume +",
        ["cmd.audio.volume_down"] = "Volume −",
        ["cmd.audio.toggle_mute"] = "Mute",
        ["cmd.audio.toggle_menu_music"] = "Menu music",
        ["cmd.tools.options"] = "Options…",
        ["cmd.tools.key_config"] = "Key configuration…",
        ["cmd.tools.data_folder"] = "Data folder",
        ["cmd.tools.about"] = "About",
        ["cmd.ui.cancel_modal"] = "Close the modal window",
        ["cmd.ui.toggle_debug_overlay"] = "Statistics panel",
        ["cmd.ui.close_windows"] = "Close the windows",

        // ------------------------------------------- labels of SPEC-09 settings
        ["setting." + SettingKeys.VideoFrameRate] = "Render rate",
        ["setting." + SettingKeys.VideoSkipFrame] = "Accelerated rendering (frame skip)",
        ["setting." + SettingKeys.VideoVSync] = "Vertical synchronisation",
        ["setting." + SettingKeys.VideoRenderMethod] = "Render method",
        ["setting." + SettingKeys.VideoResScale] = "Resolution scale",
        ["setting." + SettingKeys.VideoAntiAlias] = "Anti-aliasing",
        ["setting." + SettingKeys.RenderMeshAlgo] = "Mesh algorithm",
        ["setting." + SettingKeys.RenderShadow] = "Shadows",
        ["setting." + SettingKeys.CameraFov] = "Field of view (degrees)",
        ["setting." + SettingKeys.LightIntensity] = "Light intensity",
        ["setting." + SettingKeys.SceneSkybox] = "Background (skybox)",
        ["setting." + SettingKeys.SceneCustomBgColor] = "Background colour",
        ["setting." + SettingKeys.EmuGameSpeed] = "Game speed",
        ["setting." + SettingKeys.EmuAutoTvMode] = "Region detected automatically",
        ["setting." + SettingKeys.EmuTvMode] = "Video region",
        ["setting." + SettingKeys.EmuCore] = "Emulation core",
        ["setting." + SettingKeys.AudioVolume] = "Master volume",
        ["setting." + SettingKeys.AudioBgMusic] = "Menu music",
        ["setting." + SettingKeys.AudioMute] = "Mute",
        ["setting." + SettingKeys.InputEmulatedMenuButton] = "Menu = Select + Start",
        ["setting." + SettingKeys.UiFullscreen] = "Full screen",
        ["setting." + SettingKeys.UiShowFps] = "Show the FPS",
        ["setting." + SettingKeys.UiSortOption] = "Library sort order",
        ["setting." + SettingKeys.UiLanguage] = "Language",

        // ----------------------------------- labels of the enumerated values
        ["option.Off"] = "Off",
        ["option.Every refresh"] = "Every refresh",
        ["option.Every other refresh"] = "Every other refresh",
        ["option.Default"] = "Default",
        ["option.Color"] = "Colour",
        ["option.Gradient"] = "Gradient",
        ["option.Fantasy"] = "Fantasy",
        ["option.None"] = "None",
        ["option.Native"] = "Native",
        ["option.enabled"] = "enabled",
        ["option.disabled"] = "disabled",
        ["option.A-Z"] = "A-Z",
        ["option.Recent"] = "Recent",
        ["option.en"] = "English",
        ["option.fr"] = "Français",
    };
}
