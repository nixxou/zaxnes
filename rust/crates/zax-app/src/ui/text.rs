//! The named view onto the interface texts of AMD-035: one function per text the code needs,
//! each reading the table of the language in force through [`strings::get`].
//!
//! Every member is a function, never a constant: AMD-035 §1 has the language change apply
//! immediately. The models that cache a list of labels watch [`strings::generation`] instead.
//!
//! The notification texts have no function here: they are produced by the emulation loop, by the
//! profile reader and by the settings files, in the exact English SPEC-10 REQ-UI-082 to
//! REQ-UI-084 and SPEC-09 fix word for word. AMD-036 §1 makes that English the table entry and
//! has [`super::notifications`] translate the message where it is shown.
//!
//! The labels of the 19 logical actions are left in English: SPEC-08 §4.1 publishes them as the
//! names of the actions, the key configuration screen shows them (REQ-UI-070) and the binding
//! file persists them (REQ-PERS-065).

use zax_settings::meta::SettingDescriptor;

use super::keys;
use super::strings;

/// Product name shown in the title bar and the about block (DEC-10-Q3).
pub const PRODUCT_NAME: &str = "Zaxnes";

/// Title of the File menu (REQ-UI-200).
#[must_use]
pub fn menu_file() -> String {
    strings::get(keys::MENU_FILE)
}

/// Title of the Emulation menu.
#[must_use]
pub fn menu_emulation() -> String {
    strings::get(keys::MENU_EMULATION)
}

/// Title of the Video menu.
#[must_use]
pub fn menu_video() -> String {
    strings::get(keys::MENU_VIDEO)
}

/// Title of the Audio menu.
#[must_use]
pub fn menu_audio() -> String {
    strings::get(keys::MENU_AUDIO)
}

/// Title of the Tools menu.
#[must_use]
pub fn menu_tools() -> String {
    strings::get(keys::MENU_TOOLS)
}

/// Label of the recent ROM submenu (REQ-UI-200).
#[must_use]
pub fn recent_roms() -> String {
    strings::get(keys::MENU_RECENT_ROMS)
}

/// Label of the load-state submenu.
#[must_use]
pub fn load_state() -> String {
    strings::get(keys::MENU_LOAD_STATE)
}

/// Label of the save-state submenu.
#[must_use]
pub fn save_state() -> String {
    strings::get(keys::MENU_SAVE_STATE)
}

/// Label of the submenu that deletes a state (REQ-UI-052).
#[must_use]
pub fn delete_state() -> String {
    strings::get(keys::MENU_DELETE_STATE)
}

/// Label of the speed submenu.
#[must_use]
pub fn speed() -> String {
    strings::get(keys::MENU_SPEED)
}

/// Label of the region submenu.
#[must_use]
pub fn region() -> String {
    strings::get(keys::MENU_REGION)
}

/// Label of the port submenu.
#[must_use]
pub fn ports() -> String {
    strings::get(keys::MENU_PORTS)
}

/// Label of the camera mode submenu.
#[must_use]
pub fn camera_mode() -> String {
    strings::get(keys::MENU_CAMERA_MODE)
}

/// Label of the volume submenu.
#[must_use]
pub fn volume() -> String {
    strings::get(keys::MENU_VOLUME)
}

/// Shown in place of an empty submenu.
#[must_use]
pub fn nothing_here() -> String {
    strings::get(keys::MENU_NOTHING_HERE)
}

/// The automatic region entry of the Region submenu.
#[must_use]
pub fn region_auto() -> String {
    strings::get(keys::MENU_REGION_AUTO)
}

/// The word a port submenu is titled with (REQ-INP-060).
#[must_use]
pub fn port() -> String {
    strings::get(keys::MENU_PORT)
}

/// The Controller entry of a port submenu.
#[must_use]
pub fn port_controller() -> String {
    strings::get(keys::MENU_PORT_CONTROLLER)
}

/// The Zapper entry of a port submenu.
#[must_use]
pub fn port_zapper() -> String {
    strings::get(keys::MENU_PORT_ZAPPER)
}

/// The orbital camera entry of the camera submenu (SPEC-06 REQ-SCN-006).
#[must_use]
pub fn camera_orbital() -> String {
    strings::get(keys::MENU_CAMERA_ORBITAL)
}

/// The first-person camera entry of the camera submenu.
#[must_use]
pub fn camera_first_person() -> String {
    strings::get(keys::MENU_CAMERA_FIRST_PERSON)
}

/// What an empty save slot shows (REQ-UI-208).
#[must_use]
pub fn empty_slot() -> String {
    strings::get(keys::SLOT_EMPTY)
}

/// Label of the quick slot in the state submenus (REQ-UI-208).
#[must_use]
pub fn quick_slot() -> String {
    strings::get(keys::SLOT_QUICK)
}

/// Prefix of a numbered slot in the state submenus.
#[must_use]
pub fn slot_prefix() -> String {
    strings::get(keys::SLOT_PREFIX)
}

/// Title of the options window (REQ-UI-202).
#[must_use]
pub fn options_title() -> String {
    strings::get(keys::WINDOW_OPTIONS)
}

/// Title of the key configuration window (REQ-UI-203).
#[must_use]
pub fn key_config_title() -> String {
    strings::get(keys::WINDOW_KEY_CONFIG)
}

/// Title of the about window (DEC-10-Q3).
#[must_use]
pub fn about_title() -> String {
    strings::get(keys::WINDOW_ABOUT)
}

/// Title of the profile chooser (REQ-UI-201).
#[must_use]
pub fn profile_chooser_title() -> String {
    strings::get(keys::WINDOW_PROFILE_CHOOSER)
}

/// Title of the ROM file dialog (REQ-UI-201, SPEC-10 §4).
#[must_use]
pub fn open_rom_title() -> String {
    strings::get(keys::WINDOW_OPEN_ROM)
}

/// Title of the profile import dialog (SPEC-10 §4).
#[must_use]
pub fn import_profile_title() -> String {
    strings::get(keys::WINDOW_IMPORT_PROFILE)
}

/// Title of the cover-art import dialog (REQ-PERS-036).
#[must_use]
pub fn import_cover_title() -> String {
    strings::get(keys::WINDOW_IMPORT_COVER)
}

/// Title of the 3dSen installation picker (AMD-030 §2).
#[must_use]
pub fn import_three_d_sen_title() -> String {
    strings::get(keys::WINDOW_IMPORT_THREE_DSEN)
}

/// What the 3dSen picker asks for: the installation folder, or the bundle itself (AMD-030 §2, §3).
#[must_use]
pub fn import_three_d_sen_hint() -> String {
    strings::get(keys::HINT_IMPORT_THREE_DSEN)
}

/// The "reset this tab to its defaults" button of REQ-UI-202.
#[must_use]
pub fn reset_defaults() -> String {
    strings::get(keys::BUTTON_RESET_DEFAULTS)
}

/// Why the resolution scale is greyed out under the direct render method (AMD-032 §3).
#[must_use]
pub fn res_scale_needs_render_texture() -> String {
    strings::get(keys::HINT_RES_SCALE_DIRECT)
}

/// Generic close button.
#[must_use]
pub fn close() -> String {
    strings::get(keys::BUTTON_CLOSE)
}

/// Generic cancel button.
#[must_use]
pub fn cancel() -> String {
    strings::get(keys::BUTTON_CANCEL)
}

/// Validation button of the file dialog.
#[must_use]
pub fn open() -> String {
    strings::get(keys::BUTTON_OPEN)
}

/// Validation button of the profile chooser.
#[must_use]
pub fn launch() -> String {
    strings::get(keys::BUTTON_LAUNCH)
}

/// The "go up one folder" button of the file dialog.
#[must_use]
pub fn parent_folder() -> String {
    strings::get(keys::BUTTON_PARENT_FOLDER)
}

/// Search field hint.
#[must_use]
pub fn search() -> String {
    strings::get(keys::FIELD_SEARCH)
}

/// Column header of the bound action.
#[must_use]
pub fn action_column() -> String {
    strings::get(keys::COLUMN_ACTION)
}

/// Column header of the keyboard bindings.
#[must_use]
pub fn keyboard_column() -> String {
    strings::get(keys::COLUMN_KEYBOARD)
}

/// Column header of the gamepad bindings.
#[must_use]
pub fn gamepad_column() -> String {
    strings::get(keys::COLUMN_GAMEPAD)
}

/// Button that interrupts a binding capture (REQ-UI-072).
#[must_use]
pub fn stop_binding() -> String {
    strings::get(keys::BUTTON_STOP_BINDING)
}

/// Button that puts every binding back to its default (REQ-INP-049).
#[must_use]
pub fn reset_bindings() -> String {
    strings::get(keys::BUTTON_RESET_BINDINGS)
}

/// Hint shown while a capture is listening.
#[must_use]
pub fn press_a_control() -> String {
    strings::get(keys::HINT_PRESS_ACONTROL)
}

/// Hint shown when no capture is running (REQ-INP-042).
#[must_use]
pub fn click_a_cell() -> String {
    strings::get(keys::HINT_CLICK_ACELL)
}

/// The "no gamepad" entry of the pad chooser (REQ-UI-070).
#[must_use]
pub fn no_gamepad() -> String {
    strings::get(keys::VALUE_NO_GAMEPAD)
}

/// The 2D Mode entry of the profile chooser (SPEC-10 §2).
#[must_use]
pub fn two_dimensional_mode() -> String {
    strings::get(keys::PROFILE_TWO_DIMENSIONAL)
}

/// Why a setting only takes effect at the next start (REQ-PERS-084).
#[must_use]
pub fn at_next_start() -> String {
    strings::get(keys::HINT_AT_NEXT_START)
}

/// Why a setting only takes effect at the next load (REQ-PERS-084).
#[must_use]
pub fn at_next_load() -> String {
    strings::get(keys::HINT_AT_NEXT_LOAD)
}

/// That a setting is a profile parameter a profile load overwrites (REQ-PERS-005).
#[must_use]
pub fn profile_setting() -> String {
    strings::get(keys::HINT_PROFILE_SETTING)
}

/// The call to action of REQ-UI-207.
#[must_use]
pub fn welcome_hint() -> String {
    strings::get(keys::WELCOME_HINT)
}

/// Heading above the recent ROM list of the welcome screen.
#[must_use]
pub fn welcome_recent() -> String {
    strings::get(keys::WELCOME_RECENT)
}

/// Shown on the welcome screen when no ROM has ever been opened.
#[must_use]
pub fn welcome_no_recent() -> String {
    strings::get(keys::WELCOME_NO_RECENT)
}

/// The version the about block shows.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The version line of the about block and of the welcome screen.
#[must_use]
pub fn version_line() -> String {
    format!("{} {}", strings::get(keys::ABOUT_VERSION_PREFIX), version())
}

/// The about block of DEC-10-Q3 / SPEC-10 REQ-UI-076, one line per entry: the product name, the
/// version, the licence and the factual attribution.
///
/// The three attribution lines are the sentence AMD-029 §3 gives word for word, which closes
/// Q-068: the block presents Zaxnes and credits what it owes (3dSen, the program the
/// specification describes, and the upstream engine part of `zax-engine3d` is adapted from),
/// then the disclaimer that ends it. AMD-029 writes the sentence in French, so the French table
/// is the normative one and the English table translates it.
#[must_use]
pub fn about_lines() -> Vec<String> {
    vec![
        PRODUCT_NAME.to_owned(),
        version_line(),
        strings::get(keys::ABOUT_LICENCE),
        strings::get(keys::ABOUT_CREDIT_INSPIRED),
        strings::get(keys::ABOUT_CREDIT_ADAPTED),
        strings::get(keys::ABOUT_CREDIT_DISCLAIMER),
    ]
}

/// The label of `descriptor` for the options window, in the language in force. The settings
/// SPEC-10 REQ-UI-061 made visible keep the meaning of their original label; the ones the
/// original hid (P3 in REQ-UI-202) get a descriptive one.
#[must_use]
pub fn label_of(descriptor: &SettingDescriptor) -> String {
    let key = descriptor.key();
    strings::lookup_or_none(&keys::setting(&key)).map_or_else(
        || {
            descriptor
                .ui_label()
                .map_or_else(|| key.into_owned(), str::to_owned)
        },
        str::to_owned,
    )
}

/// The label of an enumerated value, or the raw domain label when there is none.
#[must_use]
pub fn label_of_option(option: &str) -> String {
    strings::lookup_or_none(&keys::option(option))
        .unwrap_or(option)
        .to_owned()
}

/// The label of the speed setting `setting` and its factor.
#[must_use]
pub fn speed_label(setting: i32) -> String {
    let factor = crate::emulation::speed_setting::factor(setting);
    let sign = if setting > 0 { "+" } else { "" };
    format!("{sign}{setting}  (×{factor:.2})")
}
