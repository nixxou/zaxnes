//! The key configuration window: its rows, the capture through the rebinding session and the
//! frame it draws (`KeyConfigWindowTests.cs`).

mod common;
mod ui_harness;

use common::FakeGamepad;
use ui_harness::UiHarness;
use zax_platform::input::{standard_gamepad, GamepadHub as _, Key};
use zax_app::emulation::commands::Command;
use zax_app::input::input_action::{self, InputAction};
use zax_app::input::{BindingDeviceKind, BindingTarget};
use zax_app::ui::key_config::{self, KeyConfigRow, COMMANDS_NEEDING_ARGUMENTS};
use zax_app::ui::commands::UiCommand;

fn row_of(harness: &mut UiHarness, target: &BindingTarget, player: usize) -> KeyConfigRow {
    harness
        .ui
        .keys
        .all_rows()
        .find(|row| row.player == player && row.target == *target)
        .expect("the row exists")
        .clone()
}

/// Starts the capture of `row` on `kind` through the input layer, as a click on its cell does.
fn begin(harness: &mut UiHarness, row: &KeyConfigRow, kind: BindingDeviceKind) -> bool {
    let hub = &harness.pads;
    harness
        .input
        .edit(|bindings, pads, session| key_config::begin(session, bindings, pads, hub, row, kind))
}

#[test]
fn req_ui_203_the_window_lists_the_actions_of_both_players_and_the_commands() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let titles: Vec<String> = harness
        .ui
        .keys
        .sections()
        .iter()
        .map(|section| section.title.clone())
        .collect();
    assert_eq!(
        vec![
            "Player 1 — NES buttons",
            "Player 2 — NES buttons",
            "Camera",
            "System hotkeys",
            "Menu commands"
        ],
        titles
    );

    // The 19 actions of §4.1 are all there for player 1, and the eight NES buttons again for
    // player 2 (REQ-INP-001).
    for action in input_action::ALL {
        let target = BindingTarget::of(action);
        assert!(
            harness
                .ui
                .keys
                .all_rows()
                .any(|row| row.player == 0 && row.target == target),
            "{}",
            input_action::name_of(action)
        );
    }
    assert_eq!(8, harness.ui.keys.all_rows().filter(|row| row.player == 1).count());

    // Every argument-less command of the model is bindable (REQ-UI-203).
    for command in Command::ALL {
        let bindable = !COMMANDS_NEEDING_ARGUMENTS.contains(command);
        let target = BindingTarget::of_command(command.name());
        assert_eq!(
            bindable,
            harness.ui.keys.all_rows().any(|row| row.target == target),
            "{}",
            command.name()
        );
    }
    // The commands the interface owns join the same list.
    for command in UiCommand::ALL {
        let target = BindingTarget::of_command(command.name());
        assert!(harness.ui.keys.all_rows().any(|row| row.target == target));
    }
}

#[test]
fn req_ui_203_a_keyboard_capture_through_the_window_rebinds_the_action() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let row = row_of(&mut harness, &BindingTarget::of(InputAction::A), 0);
    assert!(begin(&mut harness, &row, BindingDeviceKind::Keyboard));

    // REQ-INP-042: the field was cleared the moment it was activated.
    assert!(!harness
        .input
        .bindings()
        .keyboard(0)
        .get_action(InputAction::A)
        .is_bound());
    assert!(key_config::is_listening_for(
        harness.input.rebinding(),
        &row,
        BindingDeviceKind::Keyboard
    ));

    // Q-033 / AMD-015: a key held when the field opened is ignored until it is released, so the
    // capture needs a real rising edge.
    harness.sample(1.0 / 60.0);
    harness.keyboard.press(Key::K as i32);
    harness.sample(1.0 / 60.0);

    assert!(!harness.input.rebinding().is_listening());
    assert_eq!(
        Key::K as i32,
        harness
            .input
            .bindings()
            .keyboard(0)
            .get_action(InputAction::A)
            .code
    );

    // The game answers to K from the next sample on (REQ-INP-047: not this one).
    harness.input.end_sample();
    let snapshot = harness.sample(1.0 / 60.0);
    assert!(snapshot.player_one.contains(InputAction::A));
}

#[test]
fn req_ui_203_a_menu_command_bound_to_a_pad_button_runs() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let pad = FakeGamepad::new("pad-1");
    harness.pads.plug(&pad);
    {
        let hub = &harness.pads;
        harness.input.edit(|bindings, pads, _| {
            pads.skip_startup_delay(hub, bindings);
            pads.assign(bindings, 0, hub.devices().first().map(|device| &**device));
        });
    }

    let target = BindingTarget::of_command(Command::ShowAbout.name());
    let row = row_of(&mut harness, &target, 0);
    assert!(begin(&mut harness, &row, BindingDeviceKind::Gamepad));

    harness.sample(1.0 / 60.0);
    pad.press(standard_gamepad::NORTH);
    harness.sample(1.0 / 60.0);

    assert!(!harness.input.rebinding().is_listening());
    let identity = harness
        .input
        .pads()
        .assigned_identity(0)
        .expect("a pad is assigned")
        .to_owned();
    assert_eq!(
        standard_gamepad::NORTH,
        harness
            .input
            .bindings()
            .find_gamepad(&identity)
            .expect("the pad has a set")
            .get(&target)
            .code
    );

    // Pressing it now runs the command, which opens the about window.
    harness.input.end_sample();
    harness.sample(1.0 / 60.0);
    assert!(harness
        .input
        .triggered_commands()
        .iter()
        .any(|command| command == Command::ShowAbout.name()));

    let triggered: Vec<String> = harness.input.triggered_commands().to_vec();
    for name in triggered {
        if let Some(command) = Command::from_name(&name) {
            harness
                .host
                .enqueue(command, zax_app::emulation::commands::CommandArgs::default());
        }
    }
    harness.host.apply_commands();
    harness.pump();
    assert!(harness.ui.state.about);
}

#[test]
fn req_ui_072_stopping_the_capture_leaves_the_action_unbound() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    let row = row_of(&mut harness, &BindingTarget::of(InputAction::B), 0);
    begin(&mut harness, &row, BindingDeviceKind::Keyboard);
    harness.input.edit(|_, _, session| session.stop());

    assert!(!harness.input.rebinding().is_listening());
    assert!(!harness
        .input
        .bindings()
        .keyboard(0)
        .get_action(InputAction::B)
        .is_bound());
}

#[test]
fn req_inp_046_a_player_without_a_pad_cannot_be_captured() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    assert!(key_config::set_of(
        harness.input.bindings(),
        harness.input.pads(),
        0,
        BindingDeviceKind::Gamepad
    )
    .is_none());

    let row = row_of(&mut harness, &BindingTarget::of(InputAction::LEFT), 0);
    assert!(!begin(&mut harness, &row, BindingDeviceKind::Gamepad));
    assert!(!harness.input.rebinding().is_listening());
}

#[test]
fn req_inp_049_the_reset_restores_the_default_bindings() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.input.edit(|bindings, _, _| {
        bindings
            .keyboard_mut(0)
            .clear(&BindingTarget::of(InputAction::A));
    });
    assert!(!harness
        .input
        .bindings()
        .keyboard(0)
        .get_action(InputAction::A)
        .is_bound());

    harness.input.edit(|bindings, _, _| bindings.reset_all());
    assert!(harness
        .input
        .bindings()
        .keyboard(0)
        .get_action(InputAction::A)
        .is_bound());
}

#[test]
fn req_ui_203_the_window_draws() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.ui.state.key_config = true;
    harness.draw_frames(3);

    let pad = FakeGamepad::new("pad-1");
    harness.pads.plug(&pad);
    {
        let hub = &harness.pads;
        harness.input.edit(|bindings, pads, _| {
            pads.skip_startup_delay(hub, bindings);
            pads.assign(bindings, 0, hub.devices().first().map(|device| &**device));
        });
    }

    // A capture in progress greys the other cells out (REQ-UI-071); the frame must still lay out.
    let row = row_of(&mut harness, &BindingTarget::of(InputAction::START), 0);
    begin(&mut harness, &row, BindingDeviceKind::Keyboard);

    harness.draw_frames(2);
    assert!(harness.ui.state.key_config);
}
