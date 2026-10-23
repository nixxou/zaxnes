//! The two tables of interface texts of AMD-035: one key per text, one table per language, and
//! the English table as the fallback for a key a translation is missing.
//!
//! The language in force is a property of the running emulator, not of the machine: AMD-035 §1
//! forbids any detection of the system's language, so the only thing that sets it is the setting
//! `ui.language`, which the shell reads at start-up and writes here on every change, so the
//! change applies immediately (AMD-035 §4). The interface runs on one thread, so the language
//! is thread-local state: a test that switches it cannot disturb another.
//!
//! [`generation`] counts the changes. The models that build a list of labels once compare it
//! with the generation they were built for and rebuild themselves when it has moved, which is
//! what makes "the whole interface switches at once" true without a restart.

mod english;
mod french;

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::LazyLock;

use english::ENGLISH;
use french::FRENCH;

/// The code of the English language in `ui.language`.
pub const CODE_ENGLISH: &str = "en";
/// The code of the French language in `ui.language`.
pub const CODE_FRENCH: &str = "fr";

/// The two interface languages of AMD-035 §1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiLanguage {
    /// English, the default on a fresh user folder (AMD-035 §1).
    #[default]
    English,
    /// French.
    French,
}

impl UiLanguage {
    /// The `ui.language` text: `en` or `fr`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => CODE_ENGLISH,
            Self::French => CODE_FRENCH,
        }
    }

    /// The language a `ui.language` text designates. Anything that is not `fr` (an empty
    /// value, a misspelling, a language this build does not have) is English (AMD-035 §1).
    #[must_use]
    pub fn parse(code: &str) -> Self {
        if code.trim().eq_ignore_ascii_case(CODE_FRENCH) {
            Self::French
        } else {
            Self::English
        }
    }
}

thread_local! {
    static LANGUAGE: Cell<UiLanguage> = const { Cell::new(UiLanguage::English) };
    static GENERATION: Cell<u32> = const { Cell::new(0) };
}

static ENGLISH_INDEX: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| ENGLISH.iter().copied().collect());
static FRENCH_INDEX: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| FRENCH.iter().copied().collect());

/// The language the interface is drawn in; English until the setting says otherwise.
#[must_use]
pub fn language() -> UiLanguage {
    LANGUAGE.with(Cell::get)
}

/// Sets the language; a change moves [`generation`].
pub fn set_language(language: UiLanguage) {
    if LANGUAGE.with(Cell::get) == language {
        return;
    }
    LANGUAGE.with(|cell| cell.set(language));
    GENERATION.with(|cell| cell.set(cell.get().wrapping_add(1)));
}

/// How many times the language has changed since start-up. A model that caches labels keeps the
/// value it built with and rebuilds when it differs.
#[must_use]
pub fn generation() -> u32 {
    GENERATION.with(Cell::get)
}

/// The English table, which is also the fallback of every other language.
#[must_use]
pub const fn english_table() -> &'static [(&'static str, &'static str)] {
    ENGLISH
}

/// The French table.
#[must_use]
pub const fn french_table() -> &'static [(&'static str, &'static str)] {
    FRENCH
}

/// The text of `key` in `language` when that table holds it.
#[must_use]
pub fn lookup(language: UiLanguage, key: &str) -> Option<&'static str> {
    match language {
        UiLanguage::English => ENGLISH_INDEX.get(key).copied(),
        UiLanguage::French => FRENCH_INDEX.get(key).copied(),
    }
}

/// The text of `key` in the current language, the English one when the translation has no such
/// key, and the key itself when neither table has it: never an empty string and never a blank
/// (AMD-035 §3).
#[must_use]
pub fn get(key: &str) -> String {
    lookup_or_none(key).map_or_else(|| key.to_owned(), str::to_owned)
}

/// The same as [`get`] but `None` when neither table has the key.
#[must_use]
pub fn lookup_or_none(key: &str) -> Option<&'static str> {
    lookup(language(), key)
        .filter(|text| !text.is_empty())
        .or_else(|| lookup(UiLanguage::English, key).filter(|text| !text.is_empty()))
}

/// Puts `args` in the `{0}`, `{1}`... places of `template` (the .NET composite format the tables
/// keep).
#[must_use]
pub fn format(template: &str, args: &[&str]) -> String {
    let mut text = template.to_owned();
    for (index, arg) in args.iter().enumerate() {
        text = text.replace(&format!("{{{index}}}"), arg);
    }
    text
}
