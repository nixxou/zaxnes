//! The bindings file `input.json` (SPEC-09 REQ-PERS-065 to REQ-PERS-075).

use serde_json::{Map, Value};
use time::OffsetDateTime;
use zax_platform::input::PhysicalControl;

use crate::input::binding_target::BindingTarget;
use crate::input::input_action::PLAYER_COUNT;
use crate::input::{BindingSet, InputBindings};
use crate::user_data_paths::{set_aside_corrupt_file, write_atomic, UserDataPaths};

/// Schema version of the file.
pub const SCHEMA_VERSION: i64 = 1;
/// Notification text of an unreadable file.
pub const UNREADABLE_MESSAGE: &str = "Input bindings were unreadable and have been reset.";
/// Notification text of a file written by a newer version.
pub const NEWER_VERSION_MESSAGE: &str = "Input bindings were created by a newer version.";

/// Where the bindings are read from and written to.
pub trait InputBindingStore {
    /// Replaces `bindings` with what the store holds.
    fn load(&mut self, bindings: &mut InputBindings);

    /// Writes `bindings`.
    fn save(&mut self, bindings: &InputBindings);

    /// The notification to show after the last load or save, if any.
    fn message(&self) -> Option<&str>;
}

/// A store that keeps nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullInputBindingStore;

impl InputBindingStore for NullInputBindingStore {
    fn load(&mut self, _bindings: &mut InputBindings) {}

    fn save(&mut self, _bindings: &InputBindings) {}

    fn message(&self) -> Option<&str> {
        None
    }
}

/// The store backed by `input.json` in the user data folder.
#[derive(Debug)]
pub struct FileInputBindingStore {
    paths: UserDataPaths,
    now: fn() -> OffsetDateTime,
    message: Option<String>,
    is_read_only_session: bool,
}

impl FileInputBindingStore {
    /// Creates a store over `paths`, taking the time of a quarantined file from `now`.
    #[must_use]
    pub fn new(paths: UserDataPaths, now: fn() -> OffsetDateTime) -> Self {
        Self {
            paths,
            now,
            message: None,
            is_read_only_session: false,
        }
    }

    /// Whether the file came from a newer version and is therefore never overwritten.
    #[must_use]
    pub const fn is_read_only_session(&self) -> bool {
        self.is_read_only_session
    }
}

impl InputBindingStore for FileInputBindingStore {
    fn load(&mut self, bindings: &mut InputBindings) {
        self.message = None;
        self.is_read_only_session = false;
        if !self.paths.input_file.is_file() {
            // REQ-PERS-075: first launch starts on the default bindings, no dialogue required.
            return;
        }
        let root = std::fs::read(&self.paths.input_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|value| match value {
                Value::Object(map) => Some(map),
                _ => None,
            });
        let Some(root) = root else {
            set_aside_corrupt_file(&self.paths.input_file, (self.now)());
            self.message = Some(UNREADABLE_MESSAGE.to_owned());
            return;
        };
        if try_int(root.get("schemaVersion")).is_some_and(|version| version > SCHEMA_VERSION) {
            self.is_read_only_session = true;
            self.message = Some(NEWER_VERSION_MESSAGE.to_owned());
        }
        bindings.reset_all();
        read_keyboards(&root, bindings);
        read_gamepads(&root, bindings);
        read_last_gamepads(&root, bindings);
    }

    fn save(&mut self, bindings: &InputBindings) {
        if !self.paths.is_writable || self.is_read_only_session {
            return;
        }
        let mut keyboards = Map::new();
        for player in 0..PLAYER_COUNT {
            keyboards.insert(player.to_string(), write_set(bindings.keyboard(player)));
        }
        let mut gamepads = Map::new();
        for identity in bindings.gamepad_identities() {
            if let Some(set) = bindings.find_gamepad(identity) {
                gamepads.insert(identity.to_owned(), write_set(set));
            }
        }
        let mut last = Map::new();
        for player in 0..PLAYER_COUNT {
            let identity = bindings
                .last_gamepad(player)
                .map_or(Value::Null, |identity| Value::String(identity.to_owned()));
            last.insert(player.to_string(), identity);
        }
        let mut root = Map::new();
        root.insert("schemaVersion".to_owned(), Value::from(SCHEMA_VERSION));
        root.insert("keyboard".to_owned(), Value::Object(keyboards));
        root.insert("gamepads".to_owned(), Value::Object(gamepads));
        root.insert("lastGamepad".to_owned(), Value::Object(last));
        let text = format!("{:#}", Value::Object(root));
        if let Err(error) = write_atomic(&self.paths.input_file, text.as_bytes()) {
            self.message = Some(
                self.paths
                    .unwritable_message
                    .clone()
                    .unwrap_or_else(|| error.to_string()),
            );
        }
    }

    fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

fn read_keyboards(root: &Map<String, Value>, bindings: &mut InputBindings) {
    let Some(Value::Object(keyboards)) = root.get("keyboard") else {
        return;
    };
    for player in 0..PLAYER_COUNT {
        if let Some(Value::Object(entries)) = keyboards.get(&player.to_string()) {
            bindings.keyboard_mut(player).replace(read_set(entries));
        }
    }
}

fn read_gamepads(root: &Map<String, Value>, bindings: &mut InputBindings) {
    let Some(Value::Object(gamepads)) = root.get("gamepads") else {
        return;
    };
    for (identity, node) in gamepads {
        let Value::Object(entries) = node else {
            continue;
        };
        if identity.trim().is_empty() {
            continue;
        }
        // The set is created empty and then filled: a remembered set is authoritative and must
        // not be mixed with the §4.3 defaults (REQ-INP-025).
        bindings.gamepad_mut(identity, false).replace(read_set(entries));
    }
}

fn read_last_gamepads(root: &Map<String, Value>, bindings: &mut InputBindings) {
    let Some(Value::Object(last)) = root.get("lastGamepad") else {
        return;
    };
    for player in 0..PLAYER_COUNT {
        let identity = match last.get(&player.to_string()) {
            Some(Value::String(identity)) if !identity.trim().is_empty() => Some(identity.as_str()),
            _ => None,
        };
        bindings.set_last_gamepad(player, identity);
    }
}

fn write_set(set: &BindingSet) -> Value {
    let mut entries = Map::new();
    for (target, control) in set.bindings() {
        entries.insert(target.key(), Value::String(control.to_string()));
    }
    Value::Object(entries)
}

fn read_set(entries: &Map<String, Value>) -> Vec<(BindingTarget, PhysicalControl)> {
    entries
        .iter()
        .filter_map(|(key, node)| {
            let target = BindingTarget::parse(key);
            let Value::String(text) = node else {
                return None;
            };
            let control = PhysicalControl::parse(text);
            (!target.is_empty() && control.is_bound()).then_some((target, control))
        })
        .collect()
}

fn try_int(node: Option<&Value>) -> Option<i64> {
    match node? {
        Value::Number(number) => number.as_i64().or_else(|| {
            // Rounded like the C# `(int)Math.Round(double)`.
            #[allow(clippy::cast_possible_truncation)]
            number.as_f64().map(|value| value.round() as i64)
        }),
        _ => None,
    }
}
