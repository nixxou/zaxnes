//! The keyboard layout (lot KEYS, owner request): the keys Zaxnes always had, or `RetroArch`'s
//! default keys wherever Zaxnes has the same function. The fixed shortcuts
//! ([`crate::ui::menu_shortcuts`]) and player one's default keys
//! ([`crate::input::default_bindings`]) both follow it.

use std::cell::Cell;

/// A keyboard layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyLayout {
    /// The keys Zaxnes always had.
    Zaxnes,
    /// `RetroArch`'s default keys (`retroarch.cfg`), Zaxnes' own where `RetroArch` has none.
    #[default]
    RetroArch,
}

impl KeyLayout {
    /// The layout of the value of `ui.key_layout`: 0 Zaxnes, anything else `RetroArch`.
    #[must_use]
    pub const fn from_value(value: i32) -> Self {
        if value == 0 {
            Self::Zaxnes
        } else {
            Self::RetroArch
        }
    }

    /// The value of `ui.key_layout` for this layout.
    #[must_use]
    pub const fn value(self) -> i32 {
        match self {
            Self::Zaxnes => 0,
            Self::RetroArch => 1,
        }
    }
}

thread_local! {
    /// The layout in force: a table the interface's thread reads, like the language of the
    /// strings. Per thread, so that tests running side by side each keep their own.
    static LAYOUT: Cell<KeyLayout> = const { Cell::new(KeyLayout::RetroArch) };
}

/// The layout in force.
#[must_use]
pub fn layout() -> KeyLayout {
    LAYOUT.with(Cell::get)
}

/// Puts `layout` in force.
pub fn set_layout(layout: KeyLayout) {
    LAYOUT.with(|cell| cell.set(layout));
}
