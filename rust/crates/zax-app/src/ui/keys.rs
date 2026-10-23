//! The key of every interface text of AMD-035. Both tables of [`super::strings`] hold exactly
//! these keys, which a test of `ui_language_tests` checks.

// ----------------------------------------------------------- menu bar
pub const MENU_FILE: &str = "menu.file";
pub const MENU_EMULATION: &str = "menu.emulation";
pub const MENU_VIDEO: &str = "menu.video";
pub const MENU_AUDIO: &str = "menu.audio";
pub const MENU_TOOLS: &str = "menu.tools";
pub const MENU_RECENT_ROMS: &str = "menu.recent_roms";
pub const MENU_LOAD_STATE: &str = "menu.load_state";
pub const MENU_SAVE_STATE: &str = "menu.save_state";
pub const MENU_DELETE_STATE: &str = "menu.delete_state";
pub const MENU_SPEED: &str = "menu.speed";
pub const MENU_REGION: &str = "menu.region";
pub const MENU_PORTS: &str = "menu.ports";
pub const MENU_CAMERA_MODE: &str = "menu.camera_mode";
pub const MENU_VOLUME: &str = "menu.volume";
pub const MENU_NOTHING_HERE: &str = "menu.nothing_here";
pub const MENU_REGION_AUTO: &str = "menu.region.auto";
pub const MENU_PORT: &str = "menu.port";
pub const MENU_PORT_CONTROLLER: &str = "menu.port.controller";
pub const MENU_PORT_ZAPPER: &str = "menu.port.zapper";
pub const MENU_CAMERA_ORBITAL: &str = "menu.camera.orbital";
pub const MENU_CAMERA_FIRST_PERSON: &str = "menu.camera.first_person";
pub const SLOT_EMPTY: &str = "slot.empty";
pub const SLOT_QUICK: &str = "slot.quick";
pub const SLOT_PREFIX: &str = "slot.prefix";

// -------------------------------------------------------- window titles
pub const WINDOW_OPTIONS: &str = "window.options";
pub const WINDOW_KEY_CONFIG: &str = "window.key_config";
pub const WINDOW_ABOUT: &str = "window.about";
pub const WINDOW_PROFILE_CHOOSER: &str = "window.profile_chooser";
pub const WINDOW_OPEN_ROM: &str = "window.open_rom";
pub const WINDOW_IMPORT_PROFILE: &str = "window.import_profile";
pub const WINDOW_IMPORT_COVER: &str = "window.import_cover";
pub const WINDOW_IMPORT_THREE_DSEN: &str = "window.import_3dsen";

// -------------------------------------------------------------- buttons
pub const BUTTON_CLOSE: &str = "button.close";
pub const BUTTON_CANCEL: &str = "button.cancel";
pub const BUTTON_OPEN: &str = "button.open";
pub const BUTTON_LAUNCH: &str = "button.launch";
pub const BUTTON_PARENT_FOLDER: &str = "button.parent_folder";
pub const BUTTON_RESET_DEFAULTS: &str = "button.reset_defaults";
pub const BUTTON_RESET_BINDINGS: &str = "button.reset_bindings";
pub const BUTTON_STOP_BINDING: &str = "button.stop_binding";
pub const BUTTON_YES: &str = "button.yes";
pub const BUTTON_NO: &str = "button.no";

// ------------------- notifications of SPEC-10 §3.10, whose English text the spec fixes
pub const NOTIFY_ROLL_BACK: &str = "notify.roll_back";
pub const NOTIFY_FAST_FORWARD: &str = "notify.fast_forward";
pub const NOTIFY_QUICK_SAVE: &str = "notify.quick_save";
pub const NOTIFY_QUICK_LOAD: &str = "notify.quick_load";
pub const NOTIFY_CORRUPTED_ZIP: &str = "notify.corrupted_zip";
pub const NOTIFY_UNCOMPRESSION_FAILED: &str = "notify.uncompression_failed";
pub const NOTIFY_INVALID_ROM: &str = "notify.invalid_rom";
pub const NOTIFY_GAME_MISSING: &str = "notify.game_missing";
pub const NOTIFY_FOREIGN_CORE: &str = "notify.foreign_core";
pub const NOTIFY_NOT_A3DN_FILE: &str = "notify.not_a_3dn_file";
pub const NOTIFY_UNSUPPORTED3DN_VERSION: &str = "notify.unsupported_3dn_version";
pub const NOTIFY_OBSOLETE_PROFILE_FORMAT: &str = "notify.obsolete_profile_format";

// ------------- notifications of the persisted files (SPEC-09 REQ-PERS-022, REQ-PERS-024)
pub const NOTIFY_SETTINGS_UNREADABLE: &str = "notify.settings_unreadable";
pub const NOTIFY_SETTINGS_NEWER_VERSION: &str = "notify.settings_newer_version";
pub const NOTIFY_GAME_LIST_UNREADABLE: &str = "notify.game_list_unreadable";
pub const NOTIFY_GAME_LIST_NEWER_VERSION: &str = "notify.game_list_newer_version";
pub const NOTIFY_BINDINGS_UNREADABLE: &str = "notify.bindings_unreadable";
pub const NOTIFY_BINDINGS_NEWER_VERSION: &str = "notify.bindings_newer_version";
pub const NOTIFY_THREE_DSEN_SUMMARY: &str = "notify.import_3dsen_summary";

// ------------------- confirmations of SPEC-09 REQ-PERS-042 and REQ-PERS-054 (AMD-036 §1)
pub const CONFIRM_DELETE_GAME: &str = "confirm.delete_game";
pub const CONFIRM_DELETE_SLOT: &str = "confirm.delete_slot";

// --------------------------------------------------------------- hints
pub const HINT_IMPORT_THREE_DSEN: &str = "hint.import_3dsen";
pub const HINT_RES_SCALE_DIRECT: &str = "hint.res_scale_direct";
pub const HINT_PRESS_ACONTROL: &str = "hint.press_a_control";
pub const HINT_CLICK_ACELL: &str = "hint.click_a_cell";
pub const HINT_AT_NEXT_START: &str = "hint.at_next_start";
pub const HINT_AT_NEXT_LOAD: &str = "hint.at_next_load";
pub const HINT_PROFILE_SETTING: &str = "hint.profile_setting";

// ------------------------------------------------------- fields, columns
pub const FIELD_SEARCH: &str = "field.search";
pub const COLUMN_ACTION: &str = "column.action";
pub const COLUMN_KEYBOARD: &str = "column.keyboard";
pub const COLUMN_GAMEPAD: &str = "column.gamepad";
pub const VALUE_NO_GAMEPAD: &str = "value.no_gamepad";

// ------------------------------------------------------- welcome screen
pub const WELCOME_HINT: &str = "welcome.hint";
pub const WELCOME_RECENT: &str = "welcome.recent";
pub const WELCOME_NO_RECENT: &str = "welcome.no_recent";

// ----------------------------------------------------------- about block
pub const ABOUT_LICENCE: &str = "about.licence";
pub const ABOUT_CREDIT_INSPIRED: &str = "about.credit.inspired";
pub const ABOUT_CREDIT_ADAPTED: &str = "about.credit.adapted";
pub const ABOUT_CREDIT_DISCLAIMER: &str = "about.credit.disclaimer";
pub const ABOUT_VERSION_PREFIX: &str = "about.version_prefix";

// ----------------------------------------------------- options window tabs
pub const TAB_VIDEO: &str = "options.tab.video";
pub const TAB_CAMERA: &str = "options.tab.camera";
pub const TAB_LIGHT: &str = "options.tab.light";
pub const TAB_AUDIO: &str = "options.tab.audio";
pub const TAB_EMULATION: &str = "options.tab.emulation";
pub const TAB_INTERFACE: &str = "options.tab.interface";

// ------------------------------------------------- key configuration sections
pub const KEYS_PLAYER_ONE: &str = "keys.section.player_one";
pub const KEYS_PLAYER_TWO: &str = "keys.section.player_two";
pub const KEYS_CAMERA: &str = "keys.section.camera";
pub const KEYS_HOTKEYS: &str = "keys.section.hotkeys";
pub const KEYS_COMMANDS: &str = "keys.section.commands";

// --------------------------------------------------------- file browser
pub const BROWSER_COMPUTER: &str = "browser.computer";
pub const BROWSER_FILTER_NES_ROMS: &str = "browser.filter.nes_roms";
pub const BROWSER_FILTER_NES_FILES: &str = "browser.filter.nes_files";
pub const BROWSER_FILTER_ZIP: &str = "browser.filter.zip";
pub const BROWSER_FILTER_PROFILES: &str = "browser.filter.profiles";
pub const BROWSER_FILTER_BUNDLE: &str = "browser.filter.bundle";
pub const BROWSER_FILTER_PNG: &str = "browser.filter.png";
pub const BROWSER_FILTER_ALL: &str = "browser.filter.all";

// ------------------------------------------------------- profile chooser
pub const PROFILE_TWO_DIMENSIONAL: &str = "profile.two_dimensional";
pub const PROFILE_SOURCE_TWO_DIMENSIONAL: &str = "profile.source.two_dimensional";
pub const PROFILE_SOURCE_DEFAULT_THREE_D: &str = "profile.source.default_3d";
pub const PROFILE_SOURCE_BUILT_IN: &str = "profile.source.built_in";
pub const PROFILE_SOURCE_PROGRAM_FOLDER: &str = "profile.source.program_folder";
pub const PROFILE_SOURCE_USER: &str = "profile.source.user";

// ------------------------------------------------------------- shortcuts
pub const SHORTCUT_ALT_ENTER: &str = "shortcut.alt_enter";
pub const SHORTCUT_SHIFT_PREFIX: &str = "shortcut.shift_prefix";
pub const SHORTCUT_QUICK_SAVE: &str = "shortcut.quick_save";
pub const SHORTCUT_QUICK_LOAD: &str = "shortcut.quick_load";
pub const SHORTCUT_FAST_FORWARD: &str = "shortcut.fast_forward";
pub const SHORTCUT_ROLLBACK: &str = "shortcut.rollback";
pub const SHORTCUT_SAVE_SLOTS: &str = "shortcut.save_slots";
pub const SHORTCUT_LOAD_SLOTS: &str = "shortcut.load_slots";
pub const SHORTCUT_LONG_PRESS_MENU: &str = "shortcut.long_press_menu";

// ------------------------------------------- handheld interface (Rust-only)
pub const HANDHELD_GAMES_TITLE: &str = "handheld.games_title";
pub const HANDHELD_ROMS_EMPTY: &str = "handheld.roms_empty";
pub const HANDHELD_TAG_2D: &str = "handheld.tag_2d";
pub const HANDHELD_PROFILE_FOR: &str = "handheld.profile_for";
pub const HANDHELD_HINT_PLAY: &str = "handheld.hint.play";
pub const HANDHELD_HINT_PROFILE: &str = "handheld.hint.profile";
pub const HANDHELD_HINT_OPTIONS: &str = "handheld.hint.options";
pub const HANDHELD_HINT_BACK: &str = "handheld.hint.back";
pub const HANDHELD_HINT_JUMP: &str = "handheld.hint.jump";
pub const HANDHELD_HINT_CHOOSE: &str = "handheld.hint.choose";
pub const HANDHELD_HINT_CHANGE: &str = "handheld.hint.change";
pub const HANDHELD_HINT_REBIND: &str = "handheld.hint.rebind";
pub const HANDHELD_HINT_CANCEL: &str = "handheld.hint.cancel";
pub const HANDHELD_EXIT_HOLD: &str = "handheld.exit_hold";
pub const HANDHELD_MENU_TITLE: &str = "handheld.menu.title";
pub const HANDHELD_MENU_RESUME: &str = "handheld.menu.resume";
pub const HANDHELD_MENU_SAVE_STATE: &str = "handheld.menu.save_state";
pub const HANDHELD_MENU_LOAD_STATE: &str = "handheld.menu.load_state";
pub const HANDHELD_MENU_FAST_FORWARD: &str = "handheld.menu.fast_forward";
pub const HANDHELD_MENU_QUIT_TO_LIST: &str = "handheld.menu.quit_to_list";
pub const HANDHELD_MENU_EXIT: &str = "handheld.menu.exit";
pub const HANDHELD_OPTION_RENDER_SCALE: &str = "handheld.option.render_scale";
pub const HANDHELD_OPTION_FRAME_CAP: &str = "handheld.option.frame_cap";
pub const HANDHELD_OPTION_CAMERA: &str = "handheld.option.camera";
pub const HANDHELD_OPTION_CONTROLS: &str = "handheld.option.controls";
pub const HANDHELD_OPTION_REGION: &str = "handheld.option.region";
pub const HANDHELD_REGION_AUTO: &str = "handheld.region.auto";
pub const HANDHELD_CAMERA_RESET: &str = "handheld.camera.reset";
pub const HANDHELD_CONTROLS_PLAYER: &str = "handheld.controls.player";

// ------------------------------------------------- commands of REQ-UI-205
pub const COMMAND_PREFIX: &str = "cmd.";

// --------------------------------------------- labels of SPEC-09 settings
pub const SETTING_PREFIX: &str = "setting.";

// ------------------------------------------- labels of enumerated values
pub const OPTION_PREFIX: &str = "option.";

/// The label key of the command named `command`.
#[must_use]
pub fn command(command: &str) -> String {
    format!("{COMMAND_PREFIX}{command}")
}

/// The label key of the setting whose stable key is `key`.
#[must_use]
pub fn setting(key: &str) -> String {
    format!("{SETTING_PREFIX}{key}")
}

/// The label key of the domain value `label` of SPEC-09 §4.1.
#[must_use]
pub fn option(label: &str) -> String {
    format!("{OPTION_PREFIX}{label}")
}

/// Every key published above, for the parity tests (the prefixes are not keys).
pub const PUBLISHED: &[&str] = &[
    MENU_FILE,
    MENU_EMULATION,
    MENU_VIDEO,
    MENU_AUDIO,
    MENU_TOOLS,
    MENU_RECENT_ROMS,
    MENU_LOAD_STATE,
    MENU_SAVE_STATE,
    MENU_DELETE_STATE,
    MENU_SPEED,
    MENU_REGION,
    MENU_PORTS,
    MENU_CAMERA_MODE,
    MENU_VOLUME,
    MENU_NOTHING_HERE,
    MENU_REGION_AUTO,
    MENU_PORT,
    MENU_PORT_CONTROLLER,
    MENU_PORT_ZAPPER,
    MENU_CAMERA_ORBITAL,
    MENU_CAMERA_FIRST_PERSON,
    SLOT_EMPTY,
    SLOT_QUICK,
    SLOT_PREFIX,
    WINDOW_OPTIONS,
    WINDOW_KEY_CONFIG,
    WINDOW_ABOUT,
    WINDOW_PROFILE_CHOOSER,
    WINDOW_OPEN_ROM,
    WINDOW_IMPORT_PROFILE,
    WINDOW_IMPORT_COVER,
    WINDOW_IMPORT_THREE_DSEN,
    BUTTON_CLOSE,
    BUTTON_CANCEL,
    BUTTON_OPEN,
    BUTTON_LAUNCH,
    BUTTON_PARENT_FOLDER,
    BUTTON_RESET_DEFAULTS,
    BUTTON_RESET_BINDINGS,
    BUTTON_STOP_BINDING,
    BUTTON_YES,
    BUTTON_NO,
    NOTIFY_ROLL_BACK,
    NOTIFY_FAST_FORWARD,
    NOTIFY_QUICK_SAVE,
    NOTIFY_QUICK_LOAD,
    NOTIFY_CORRUPTED_ZIP,
    NOTIFY_UNCOMPRESSION_FAILED,
    NOTIFY_INVALID_ROM,
    NOTIFY_GAME_MISSING,
    NOTIFY_FOREIGN_CORE,
    NOTIFY_NOT_A3DN_FILE,
    NOTIFY_UNSUPPORTED3DN_VERSION,
    NOTIFY_OBSOLETE_PROFILE_FORMAT,
    NOTIFY_SETTINGS_UNREADABLE,
    NOTIFY_SETTINGS_NEWER_VERSION,
    NOTIFY_GAME_LIST_UNREADABLE,
    NOTIFY_GAME_LIST_NEWER_VERSION,
    NOTIFY_BINDINGS_UNREADABLE,
    NOTIFY_BINDINGS_NEWER_VERSION,
    NOTIFY_THREE_DSEN_SUMMARY,
    CONFIRM_DELETE_GAME,
    CONFIRM_DELETE_SLOT,
    HINT_IMPORT_THREE_DSEN,
    HINT_RES_SCALE_DIRECT,
    HINT_PRESS_ACONTROL,
    HINT_CLICK_ACELL,
    HINT_AT_NEXT_START,
    HINT_AT_NEXT_LOAD,
    HINT_PROFILE_SETTING,
    FIELD_SEARCH,
    COLUMN_ACTION,
    COLUMN_KEYBOARD,
    COLUMN_GAMEPAD,
    VALUE_NO_GAMEPAD,
    WELCOME_HINT,
    WELCOME_RECENT,
    WELCOME_NO_RECENT,
    ABOUT_LICENCE,
    ABOUT_CREDIT_INSPIRED,
    ABOUT_CREDIT_ADAPTED,
    ABOUT_CREDIT_DISCLAIMER,
    ABOUT_VERSION_PREFIX,
    TAB_VIDEO,
    TAB_CAMERA,
    TAB_LIGHT,
    TAB_AUDIO,
    TAB_EMULATION,
    TAB_INTERFACE,
    KEYS_PLAYER_ONE,
    KEYS_PLAYER_TWO,
    KEYS_CAMERA,
    KEYS_HOTKEYS,
    KEYS_COMMANDS,
    BROWSER_COMPUTER,
    BROWSER_FILTER_NES_ROMS,
    BROWSER_FILTER_NES_FILES,
    BROWSER_FILTER_ZIP,
    BROWSER_FILTER_PROFILES,
    BROWSER_FILTER_BUNDLE,
    BROWSER_FILTER_PNG,
    BROWSER_FILTER_ALL,
    PROFILE_TWO_DIMENSIONAL,
    PROFILE_SOURCE_TWO_DIMENSIONAL,
    PROFILE_SOURCE_DEFAULT_THREE_D,
    PROFILE_SOURCE_BUILT_IN,
    PROFILE_SOURCE_PROGRAM_FOLDER,
    PROFILE_SOURCE_USER,
    SHORTCUT_ALT_ENTER,
    SHORTCUT_SHIFT_PREFIX,
    SHORTCUT_QUICK_SAVE,
    SHORTCUT_QUICK_LOAD,
    SHORTCUT_FAST_FORWARD,
    SHORTCUT_ROLLBACK,
    SHORTCUT_SAVE_SLOTS,
    SHORTCUT_LOAD_SLOTS,
    SHORTCUT_LONG_PRESS_MENU,
    HANDHELD_GAMES_TITLE,
    HANDHELD_ROMS_EMPTY,
    HANDHELD_TAG_2D,
    HANDHELD_PROFILE_FOR,
    HANDHELD_HINT_PLAY,
    HANDHELD_HINT_PROFILE,
    HANDHELD_HINT_OPTIONS,
    HANDHELD_HINT_BACK,
    HANDHELD_HINT_JUMP,
    HANDHELD_HINT_CHOOSE,
    HANDHELD_HINT_CHANGE,
    HANDHELD_HINT_REBIND,
    HANDHELD_HINT_CANCEL,
    HANDHELD_EXIT_HOLD,
    HANDHELD_MENU_TITLE,
    HANDHELD_MENU_RESUME,
    HANDHELD_MENU_SAVE_STATE,
    HANDHELD_MENU_LOAD_STATE,
    HANDHELD_MENU_FAST_FORWARD,
    HANDHELD_MENU_QUIT_TO_LIST,
    HANDHELD_MENU_EXIT,
    HANDHELD_OPTION_RENDER_SCALE,
    HANDHELD_OPTION_FRAME_CAP,
    HANDHELD_OPTION_CAMERA,
    HANDHELD_OPTION_CONTROLS,
    HANDHELD_OPTION_REGION,
    HANDHELD_REGION_AUTO,
    HANDHELD_CAMERA_RESET,
    HANDHELD_CONTROLS_PLAYER,
];
