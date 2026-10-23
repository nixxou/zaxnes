//! Rust-only fixes (owner requests): a key press consumed by an application shortcut does not
//! reach the game, and the options shortcut toggles its window.

mod common;
mod ui_harness;

use common::{FakeGamepadHub, FakeKeyboard, FakePointer};
use ui_harness::UiHarness;
use zax_app::input::input_action::InputAction;
use zax_app::input::{DesktopInputSource, DeviceRefs};
use zax_app::ui::menu_shortcuts::ShortcutModifiers;
use zax_app::ui::zax_ui::ZaxUi;
use zax_platform::input::{Key, KeyboardDevice};

fn start_held(source: &mut DesktopInputSource, keyboard: &FakeKeyboard) -> bool {
    let mut hub = FakeGamepadHub::default();
    let mut pointer = FakePointer::default();
    source
        .sample(
            0.016,
            DeviceRefs {
                keyboard: Some(keyboard),
                pointer: Some(&mut pointer),
                hub: &mut hub,
            },
        )
        .player_one
        .contains(InputAction::START)
}

/// One frame of the application loop: the shortcut keys first, then the sample.
fn frame(ui: &mut ZaxUi, source: &mut DesktopInputSource, keyboard: &FakeKeyboard) -> bool {
    source.consume_keys(
        ui.consumed_keys(|key| keyboard.is_down(key as i32))
            .into_iter()
            .map(|key| key as i32),
    );
    let start = start_held(source, keyboard);
    let _ = ui.rising_keys(|key| keyboard.is_down(key as i32));
    start
}

#[test]
fn alt_enter_does_not_press_start_until_enter_is_released_and_pressed_again() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let mut source = DesktopInputSource::unsaved();
    let mut keyboard = FakeKeyboard::default();

    keyboard.press(Key::AltLeft as i32);
    keyboard.press(Key::Enter as i32);
    assert!(!frame(&mut harness.ui, &mut source, &keyboard));
    // Alt comes up first: Enter is still the same press.
    keyboard.release(Key::AltLeft as i32);
    assert!(!frame(&mut harness.ui, &mut source, &keyboard));
    keyboard.release(Key::Enter as i32);
    assert!(!frame(&mut harness.ui, &mut source, &keyboard));
    keyboard.press(Key::Enter as i32);
    assert!(frame(&mut harness.ui, &mut source, &keyboard));
}

#[test]
fn a_plain_enter_still_presses_start() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let mut source = DesktopInputSource::unsaved();
    let mut keyboard = FakeKeyboard::default();
    keyboard.press(Key::Enter as i32);
    assert!(frame(&mut harness.ui, &mut source, &keyboard));
}

#[test]
fn the_options_shortcut_opens_then_closes_the_window() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    for expected in [true, false, true] {
        harness
            .ui
            .handle_shortcuts(&mut harness.host, &[Key::Comma], ShortcutModifiers::CONTROL);
        harness.host.apply_commands();
        harness.pump();
        assert_eq!(expected, harness.ui.state.options);
    }
}
