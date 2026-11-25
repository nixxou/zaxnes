//! Lot KEYS (owner request): the `RetroArch` keyboard layout, its current slot with the
//! notifications of the saves and loads the player asks for, and the bringing of a saved set of
//! keys from one layout to the other.

mod common;
mod ui_harness;

use common::LoopHarness;
use ui_harness::UiHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::input::default_bindings::{bring_to_layout, create_player_one_keyboard};
use zax_app::input::input_action::InputAction;
use zax_app::input::key_layout::{set_layout, KeyLayout};
use zax_app::ui::commands::UiCommand;
use zax_app::ui::menu_shortcuts::{self, ShortcutCommand, ShortcutModifiers};
use zax_app::ui::strings::{self, UiLanguage};
use zax_platform::input::{Key, PhysicalControl};

fn single(key: Key, modifiers: ShortcutModifiers) -> (ShortcutCommand, CommandArgs) {
    let mut hits = menu_shortcuts::resolve(&[key], modifiers);
    assert_eq!(1, hits.len(), "{}", key.name());
    hits.remove(0)
}

#[test]
fn the_retroarch_layout_has_retroarch_hotkeys() {
    use ShortcutCommand::{Host, Ui};
    set_layout(KeyLayout::RetroArch);
    let none = ShortcutModifiers::NONE;
    let shift = ShortcutModifiers::SHIFT;
    for (key, modifiers, expected) in [
        (Key::F1, none, Ui(UiCommand::ToggleConsole)),
        (Key::F2, none, Host(Command::SaveCurrentSlot)),
        (Key::F4, none, Host(Command::LoadCurrentSlot)),
        (Key::F6, none, Host(Command::PreviousSlot)),
        (Key::F7, none, Host(Command::NextSlot)),
        (Key::F2, shift, Ui(UiCommand::SaveStates)),
        (Key::F4, shift, Ui(UiCommand::LoadStates)),
        (Key::H, none, Host(Command::Reset)),
        (Key::F, none, Host(Command::ToggleFullscreen)),
        (Key::Home, none, Host(Command::ResetView)),
        (Key::Keypad5, none, Host(Command::ResetView)),
        (Key::F9, none, Host(Command::ToggleMute)),
        (Key::Escape, none, Ui(UiCommand::EscapeQuit)),
        (Key::O, none, Host(Command::ToggleMovieRecording)),
        (Key::F8, none, Host(Command::TakeScreenshot)),
        (Key::M, none, Host(Command::NextShader)),
        (Key::N, none, Host(Command::PreviousShader)),
        // Kept from Zaxnes.
        (Key::F11, none, Host(Command::ToggleTwoDimensional)),
        (Key::P, none, Host(Command::PauseResume)),
        (Key::K, none, Host(Command::FrameAdvance)),
    ] {
        assert_eq!(expected, single(key, modifiers).0, "{}", key.name());
    }
    // No key per slot in this layout.
    assert_eq!(
        0,
        menu_shortcuts::resolve(&[Key::F3], ShortcutModifiers::CONTROL).len()
    );
    assert_eq!(0, menu_shortcuts::resolve(&[Key::F5], shift).len());
}

#[test]
fn the_retroarch_keys_put_the_camera_on_the_keypad() {
    set_layout(KeyLayout::RetroArch);
    let set = create_player_one_keyboard();
    let key = |key: Key| PhysicalControl::key(key.code());
    for (action, expected) in [
        (InputAction::A, Key::X),
        (InputAction::SELECT, Key::Space),
        (InputAction::ROT_LEFT, Key::Keypad4),
        (InputAction::ROT_RIGHT, Key::Keypad6),
        (InputAction::ROT_UP, Key::Keypad8),
        (InputAction::ROT_DOWN, Key::Keypad2),
        (InputAction::ZOOM_IN, Key::Keypad9),
        (InputAction::ZOOM_OUT, Key::Keypad3),
        (InputAction::ROLL_BACK, Key::R),
        (InputAction::FAST_FORWARD, Key::L),
    ] {
        assert_eq!(key(expected), set.get_action(action), "{action:?}");
    }
    // Escape quits; F1 opens the menu.
    assert!(!set.get_action(InputAction::MENU).is_bound());
}

#[test]
fn a_saved_set_moves_to_the_layout_but_keeps_the_keys_of_the_player() {
    set_layout(KeyLayout::Zaxnes);
    let mut set = create_player_one_keyboard();
    let key = |key: Key| PhysicalControl::key(key.code());
    set.set_action(InputAction::ROT_LEFT, key(Key::Q));

    assert!(bring_to_layout(
        &mut set,
        KeyLayout::Zaxnes,
        KeyLayout::RetroArch
    ));
    assert_eq!(key(Key::Q), set.get_action(InputAction::ROT_LEFT));
    assert_eq!(key(Key::Keypad6), set.get_action(InputAction::ROT_RIGHT));
    assert_eq!(key(Key::L), set.get_action(InputAction::FAST_FORWARD));
    assert!(!set.get_action(InputAction::MENU).is_bound());
    assert_eq!(key(Key::X), set.get_action(InputAction::A));

    bring_to_layout(&mut set, KeyLayout::RetroArch, KeyLayout::Zaxnes);
    assert_eq!(key(Key::F6), set.get_action(InputAction::ROT_RIGHT));
    assert_eq!(key(Key::Escape), set.get_action(InputAction::MENU));
    assert_eq!(key(Key::Q), set.get_action(InputAction::ROT_LEFT));
}

fn notification(harness: &LoopHarness) -> Option<String> {
    harness
        .host
        .notifications
        .current()
        .map(|notification| notification.text.clone())
}

#[test]
fn the_current_slot_goes_round_and_says_which() {
    strings::set_language(UiLanguage::English);
    let mut harness = LoopHarness::create();
    harness.launch("castle");
    assert_eq!(1, harness.host.current_slot());
    harness.run(Command::PreviousSlot);
    assert_eq!(10, harness.host.current_slot());
    assert_eq!(Some("Slot 10 (empty)".to_owned()), notification(&harness));
    harness.run(Command::NextSlot);
    harness.run(Command::NextSlot);
    assert_eq!(2, harness.host.current_slot());
}

#[test]
fn a_save_and_a_load_the_player_asks_for_say_so() {
    strings::set_language(UiLanguage::English);
    let mut harness = LoopHarness::create();
    harness.launch("castle");
    harness.run(Command::LoadCurrentSlot);
    assert_eq!(Some("Slot 1 is empty".to_owned()), notification(&harness));
    harness.run(Command::SaveCurrentSlot);
    assert_eq!(Some("Saved to slot 1".to_owned()), notification(&harness));
    harness.run(Command::LoadCurrentSlot);
    assert_eq!(Some("Slot 1 loaded".to_owned()), notification(&harness));
}

#[test]
fn escape_twice_quits_and_once_only_warns() {
    set_layout(KeyLayout::RetroArch);
    strings::set_language(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let none = ShortcutModifiers::NONE;
    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::Escape], none);
    harness.host.apply_commands();
    assert!(!harness.host.quit_requested());
    assert_eq!(
        Some("Press Esc again to quit".to_owned()),
        harness
            .host
            .notifications
            .current()
            .map(|notification| notification.text.clone())
    );
    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::Escape], none);
    harness.host.apply_commands();
    assert!(harness.host.quit_requested());
}

#[test]
fn escape_closes_a_modal_window_first() {
    set_layout(KeyLayout::RetroArch);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    harness.run(Command::OpenRom);
    assert!(harness.ui.state.file_browser);
    harness
        .host
        .set_modal_window_open(harness.ui.state.is_modal());
    harness
        .ui
        .handle_shortcuts(&mut harness.host, &[Key::Escape], ShortcutModifiers::NONE);
    harness.host.apply_commands();
    harness.pump();
    assert!(!harness.ui.state.file_browser);
    assert!(!harness.host.quit_requested());
}

/// Owner request: M and N go round the shader presets of the display mode, no shader among
/// them, and say which.
#[test]
fn the_shader_keys_go_round_the_presets_and_say_which() {
    strings::set_language(UiLanguage::English);
    let mut harness = LoopHarness::create();
    harness.launch("castle");
    harness.run(Command::NextShader);
    let first = notification(&harness).expect("a notification");
    assert!(first.starts_with("Shader: "), "{first}");
    // Back where it was: no shader.
    harness.run(Command::PreviousShader);
    assert_eq!(Some("Shader: none".to_owned()), notification(&harness));
}
