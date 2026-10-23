//! The notifications of the emulation loop, the profile reader and the settings files are
//! produced in the exact English SPEC-10 REQ-UI-082 to REQ-UI-084 and SPEC-09 fix word for word
//! (AMD-036 §1). They are translated where they are shown: the English message is looked up in
//! the English table, and the entry of the language in force is shown instead.

use super::keys;
use super::strings::{self, UiLanguage};

/// The texts that are the same on every occurrence: the table entry replaces the whole message.
const FIXED_KEYS: &[&str] = &[
    keys::NOTIFY_ROLL_BACK,
    keys::NOTIFY_FAST_FORWARD,
    keys::NOTIFY_QUICK_SAVE,
    keys::NOTIFY_QUICK_LOAD,
    keys::NOTIFY_CORRUPTED_ZIP,
    keys::NOTIFY_UNCOMPRESSION_FAILED,
    keys::NOTIFY_INVALID_ROM,
    keys::NOTIFY_GAME_MISSING,
    keys::NOTIFY_FOREIGN_CORE,
    keys::NOTIFY_NOT_A3DN_FILE,
    keys::NOTIFY_SETTINGS_UNREADABLE,
    keys::NOTIFY_SETTINGS_NEWER_VERSION,
    keys::NOTIFY_GAME_LIST_UNREADABLE,
    keys::NOTIFY_GAME_LIST_NEWER_VERSION,
    keys::NOTIFY_BINDINGS_UNREADABLE,
    keys::NOTIFY_BINDINGS_NEWER_VERSION,
    keys::CONFIRM_DELETE_GAME,
    keys::BUTTON_YES,
    keys::BUTTON_NO,
];

/// The texts with one value in them (`{0}`): the value is read out of the English message and put
/// back into the entry of the language in force.
const PATTERN_KEYS: &[&str] = &[
    keys::NOTIFY_UNSUPPORTED3DN_VERSION,
    keys::NOTIFY_OBSOLETE_PROFILE_FORMAT,
    keys::CONFIRM_DELETE_SLOT,
];

/// The text of `text` in the language of the interface: unchanged in English, and for a message
/// a requirement fixes the entry of the French table. Any other text (a script notification, a
/// file system message) is shown as it comes.
#[must_use]
pub fn translate(text: &str) -> String {
    if text.is_empty() || strings::language() == UiLanguage::English {
        return text.to_owned();
    }
    for key in FIXED_KEYS {
        if strings::lookup(UiLanguage::English, key) == Some(text) {
            return strings::get(key);
        }
    }
    for key in PATTERN_KEYS {
        let pattern = strings::lookup(UiLanguage::English, key).unwrap_or_default();
        if let Some(value) = read_value(pattern, text) {
            return strings::format(&strings::get(key), &[value]);
        }
    }
    text.to_owned()
}

/// The summary of a 3dSen import in the language of the interface (AMD-030 §5).
#[must_use]
pub fn three_d_sen_summary(imported: usize, already_present: usize, failed: usize) -> String {
    strings::format(
        &strings::get(keys::NOTIFY_THREE_DSEN_SUMMARY),
        &[
            &imported.to_string(),
            &already_present.to_string(),
            &failed.to_string(),
        ],
    )
}

/// The value `text` holds in the place of the `{0}` of `pattern`, when `text` is that pattern.
fn read_value<'a>(pattern: &str, text: &'a str) -> Option<&'a str> {
    let (head, tail) = pattern.split_once("{0}")?;
    let value = text.strip_prefix(head)?.strip_suffix(tail)?;
    Some(value)
}
