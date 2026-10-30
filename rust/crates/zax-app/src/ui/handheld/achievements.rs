//! RetroAchievements in the console mode (Rust-only, owner request): its screen of switches and
//! account, opened from the options, and the keyboard on the screen that types the account's
//! name and password with the gamepad.

use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use super::pad::PadButton;

/// A row of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchievementsRow {
    /// `achievements.enabled`.
    Enabled,
    /// `achievements.encore`.
    Encore,
    /// `achievements.unofficial`.
    Unofficial,
    /// `achievements.sound`.
    Sound,
    /// The account's name, typed on the keyboard.
    Username,
    /// The password, typed on the keyboard.
    Password,
    /// Signs in, or out when signed in.
    Account,
    /// The game's achievements.
    List,
}

/// The rows, in order.
pub const ROWS: [AchievementsRow; 8] = [
    AchievementsRow::Enabled,
    AchievementsRow::Encore,
    AchievementsRow::Unofficial,
    AchievementsRow::Sound,
    AchievementsRow::Username,
    AchievementsRow::Password,
    AchievementsRow::Account,
    AchievementsRow::List,
];

impl AchievementsRow {
    /// The setting of a switch row.
    #[must_use]
    pub const fn setting(self) -> Option<SettingId> {
        match self {
            Self::Enabled => Some(SettingId::AchievementsEnabled),
            Self::Encore => Some(SettingId::AchievementsEncore),
            Self::Unofficial => Some(SettingId::AchievementsUnofficial),
            Self::Sound => Some(SettingId::AchievementsSound),
            _ => None,
        }
    }
}

/// The field the keyboard types into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The account's name.
    Username,
    /// The password, shown as dots.
    Password,
}

/// What the screen asks of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchievementsCommand {
    /// Back to the options.
    Close,
    /// Type into a field on the keyboard.
    Edit(Field),
    /// Sign in with the name and password typed.
    SignIn,
    /// Sign out.
    SignOut,
    /// Show the game's achievements.
    OpenList,
}

/// The screen: the selection and the fields typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AchievementsModel {
    selected: usize,
    /// The account's name typed.
    pub username: String,
    /// The password typed, emptied once sent.
    pub password: String,
}

impl AchievementsModel {
    /// The screen on its first row.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The selected row.
    #[must_use]
    pub const fn selected(&self) -> AchievementsRow {
        ROWS[self.selected]
    }

    /// The index of the selected row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// One press of a button: a switch row changes `settings` itself; `signed_in` says what the
    /// account row does.
    pub fn handle(
        &mut self,
        button: PadButton,
        settings: &mut SettingsModel,
        signed_in: bool,
    ) -> Option<AchievementsCommand> {
        let row = self.selected();
        match button {
            PadButton::Up => self.selected = (self.selected + ROWS.len() - 1) % ROWS.len(),
            PadButton::Down => self.selected = (self.selected + 1) % ROWS.len(),
            PadButton::B | PadButton::Menu => return Some(AchievementsCommand::Close),
            PadButton::Left | PadButton::Right | PadButton::A => {
                if let Some(id) = row.setting() {
                    let on = settings.get(id).as_bool();
                    settings.set(id, SettingValue::Bool(!on));
                    return None;
                }
                if button != PadButton::A {
                    return None;
                }
                return Some(match row {
                    AchievementsRow::Username => AchievementsCommand::Edit(Field::Username),
                    AchievementsRow::Password => AchievementsCommand::Edit(Field::Password),
                    AchievementsRow::Account if signed_in => AchievementsCommand::SignOut,
                    AchievementsRow::Account => AchievementsCommand::SignIn,
                    _ => AchievementsCommand::OpenList,
                });
            }
            _ => {}
        }
        None
    }
}

/// The longest text the keyboard takes.
pub const MAX_LENGTH: usize = 64;

/// The three layouts of the character rows, which Y cycles through.
pub const LAYOUTS: [[&str; 4]; 3] = [
    ["1234567890", "qwertyuiop", "asdfghjkl'", "zxcvbnm,.-"],
    ["1234567890", "QWERTYUIOP", "ASDFGHJKL\"", "ZXCVBNM;:_"],
    [
        "!@#$%^&*()",
        "+=?/\\|~`<>",
        "[]{}",
        "\u{a3}\u{20ac}\u{a7}\u{b0}",
    ],
];

/// A key of the bottom row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialKey {
    /// The next layout.
    Shift,
    /// A space.
    Space,
    /// Removes the last character.
    Delete,
    /// The text is done.
    Done,
}

/// The bottom row.
pub const SPECIAL_KEYS: [SpecialKey; 4] = [
    SpecialKey::Shift,
    SpecialKey::Space,
    SpecialKey::Delete,
    SpecialKey::Done,
];

/// What the keyboard answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyboardCommand {
    /// The text typed.
    Done(String),
    /// Given up; the field keeps its text.
    Cancel,
}

/// The keyboard on the screen: the text, the layout and the key under the cursor (the row 4 is
/// the bottom row of [`SPECIAL_KEYS`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardModel {
    field: Field,
    text: String,
    layout: usize,
    row: usize,
    column: usize,
}

impl KeyboardModel {
    /// The keyboard for `field`, starting from `text`.
    #[must_use]
    pub fn new(field: Field, text: &str) -> Self {
        Self {
            field,
            text: text.to_owned(),
            layout: 0,
            row: 1,
            column: 0,
        }
    }

    /// The field typed into.
    #[must_use]
    pub const fn field(&self) -> Field {
        self.field
    }

    /// The text typed.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The text as shown: dots for a password.
    #[must_use]
    pub fn shown_text(&self) -> String {
        match self.field {
            Field::Username => self.text.clone(),
            Field::Password => "\u{2022}".repeat(self.text.chars().count()),
        }
    }

    /// The character rows of the layout in force.
    #[must_use]
    pub const fn rows(&self) -> [&'static str; 4] {
        LAYOUTS[self.layout]
    }

    /// The index of the layout in force.
    #[must_use]
    pub const fn layout(&self) -> usize {
        self.layout
    }

    /// The key under the cursor: its row (4 for the bottom row) and its column.
    #[must_use]
    pub const fn cursor(&self) -> (usize, usize) {
        (self.row, self.column)
    }

    fn row_length(&self, row: usize) -> usize {
        self.rows()
            .get(row)
            .map_or(SPECIAL_KEYS.len(), |keys| keys.chars().count())
    }

    /// Keeps the cursor on a key of its row.
    fn clamp(&mut self) {
        let length = self.row_length(self.row);
        if self.column >= length {
            self.column = length - 1;
        }
    }

    /// One press of a button.
    pub fn handle(&mut self, button: PadButton) -> Option<KeyboardCommand> {
        match button {
            PadButton::Left => {
                let length = self.row_length(self.row);
                self.column = (self.column + length - 1) % length;
            }
            PadButton::Right => {
                let length = self.row_length(self.row);
                self.column = (self.column + 1) % length;
            }
            PadButton::Up | PadButton::Down => {
                let from = self.row_length(self.row);
                self.row = if button == PadButton::Up {
                    (self.row + 4) % 5
                } else {
                    (self.row + 1) % 5
                };
                // Moving between rows of different lengths keeps the place along the row.
                let to = self.row_length(self.row);
                self.column = (self.column * to + from / 2) / from;
                self.clamp();
            }
            PadButton::A => return self.press_key(),
            PadButton::X => {
                self.text.pop();
            }
            PadButton::Y => self.next_layout(),
            PadButton::Start => return Some(KeyboardCommand::Done(self.text.clone())),
            PadButton::B | PadButton::Menu => return Some(KeyboardCommand::Cancel),
            _ => {}
        }
        None
    }

    fn next_layout(&mut self) {
        self.layout = (self.layout + 1) % LAYOUTS.len();
        self.clamp();
    }

    fn press_key(&mut self) -> Option<KeyboardCommand> {
        if let Some(keys) = self.rows().get(self.row) {
            if let Some(character) = keys.chars().nth(self.column) {
                self.push(character);
            }
            return None;
        }
        match SPECIAL_KEYS[self.column] {
            SpecialKey::Shift => self.next_layout(),
            SpecialKey::Space => self.push(' '),
            SpecialKey::Delete => {
                self.text.pop();
            }
            SpecialKey::Done => return Some(KeyboardCommand::Done(self.text.clone())),
        }
        None
    }

    fn push(&mut self, character: char) {
        if self.text.chars().count() < MAX_LENGTH {
            self.text.push(character);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switches_change_their_settings_and_a_opens_the_keyboard() {
        let mut settings = SettingsModel::new();
        let mut model = AchievementsModel::new();
        assert_eq!(None, model.handle(PadButton::A, &mut settings, false));
        assert!(settings.get(SettingId::AchievementsEnabled).as_bool());
        for _ in 0..4 {
            model.handle(PadButton::Down, &mut settings, false);
        }
        assert_eq!(
            Some(AchievementsCommand::Edit(Field::Username)),
            model.handle(PadButton::A, &mut settings, false)
        );
        model.handle(PadButton::Down, &mut settings, false);
        model.handle(PadButton::Down, &mut settings, false);
        assert_eq!(
            Some(AchievementsCommand::SignIn),
            model.handle(PadButton::A, &mut settings, false)
        );
        assert_eq!(
            Some(AchievementsCommand::SignOut),
            model.handle(PadButton::A, &mut settings, true)
        );
        assert_eq!(
            Some(AchievementsCommand::Close),
            model.handle(PadButton::B, &mut settings, true)
        );
    }

    #[test]
    fn the_keyboard_types_deletes_shifts_and_finishes() {
        let mut keyboard = KeyboardModel::new(Field::Password, "");
        // The cursor starts on the q.
        keyboard.handle(PadButton::A);
        keyboard.handle(PadButton::Y);
        keyboard.handle(PadButton::Right);
        keyboard.handle(PadButton::A);
        assert_eq!("qW", keyboard.text());
        assert_eq!("\u{2022}\u{2022}", keyboard.shown_text());
        keyboard.handle(PadButton::X);
        assert_eq!("q", keyboard.text());
        // Down to the bottom row, then the space and OK.
        for _ in 0..3 {
            keyboard.handle(PadButton::Down);
        }
        assert_eq!(4, keyboard.cursor().0);
        while keyboard.cursor().1 != 1 {
            keyboard.handle(PadButton::Left);
        }
        keyboard.handle(PadButton::A);
        assert_eq!(
            Some(KeyboardCommand::Done("q ".to_owned())),
            keyboard.handle(PadButton::Start)
        );
        assert_eq!(Some(KeyboardCommand::Cancel), keyboard.handle(PadButton::B));
    }

    #[test]
    fn the_cursor_stays_on_a_key_of_a_shorter_row() {
        let mut keyboard = KeyboardModel::new(Field::Username, "");
        keyboard.handle(PadButton::Y);
        keyboard.handle(PadButton::Y);
        for _ in 0..9 {
            keyboard.handle(PadButton::Right);
        }
        keyboard.handle(PadButton::Down);
        let (row, column) = keyboard.cursor();
        assert_eq!(2, row);
        assert!(column < keyboard.rows()[2].chars().count());
    }
}
