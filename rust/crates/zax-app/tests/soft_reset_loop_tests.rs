//! The frontend side of the soft reset of AMD-044: the command of REQ-LOOP-072, its place in
//! the game menu, and the extended input action of REQ-INP-125. Port of `SoftResetLoopTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::input::binding_target::BindingTarget;
use zax_app::input::default_bindings;
use zax_app::input::input_action::{self, InputAction};
use zax_app::ui::key_config::KeyConfigModel;
use zax_app::ui::strings::{self, UiLanguage};
use zax_engine3d::image::TileImageKey;

/// A profile holding one script whose `Start` and `End` hooks print themselves and whose
/// `Update` hook writes the frame number into `PData[1]`.
const COUNTER_SCRIPT: &str = "function Start()\n  print('start')\nend\n\
                              function End()\n  print('end')\nend\n\
                              function Update()\n  SetPData(0, frame)\nend\n";

/// The value of `PData[index]`, as `GetPData(index)` reads it.
fn pdata(harness: &mut LoopHarness, index: i64) -> f64 {
    harness
        .host
        .session_mut()
        .unwrap()
        .read_persistent_data(index + 1)
        .unwrap()
}

fn launch_scripted() -> LoopHarness {
    let mut harness = LoopHarness::create();
    harness.launch_with_script("soft", COUNTER_SCRIPT);
    assert_eq!(EmulationState::Running, harness.host.state());
    harness
}

/// AMD-044 REQ-LOOP-072 acceptance: in game, a soft reset empties the rollback history, leaves
/// `PData` alone and calls neither `End` nor `Start`.
#[test]
fn req_loop_072_a_soft_reset_keeps_the_profile_and_empties_the_history() {
    let mut harness = launch_scripted();

    harness.run_frames(300);
    assert!(harness.host.rollback.count() > 0);
    let written = pdata(&mut harness, 0);
    assert!(written > 0.0, "the Update hook wrote nothing");
    assert_eq!(vec!["start".to_owned()], harness.script_lines());

    harness.run(Command::SoftReset);

    // The history is emptied (REQ-LOOP-031) and the core restarts from its reset vector.
    assert_eq!(0, harness.host.rollback.count());
    assert_eq!(0, harness.host.session().unwrap().core().frame_count());
    assert_eq!(EmulationState::Running, harness.host.state());

    // PData is untouched and no hook ran: the profile was not relaunched.
    assert_eq!(written, pdata(&mut harness, 0));
    assert_eq!(vec!["start".to_owned()], harness.script_lines());
}

/// AMD-044 REQ-LOOP-072: the soft reset keeps the RAM, where the hard reset of REQ-LOOP-050
/// relaunches the game and clears it. The synthetic ROM counts frames into `$0010`.
#[test]
fn req_loop_072_a_soft_reset_keeps_the_ram_where_a_hard_reset_clears_it() {
    let mut harness = launch_scripted();

    harness.run_frames(120);
    assert_ne!(
        0,
        harness.host.session().unwrap().core().read_memory(0x0010)
    );

    harness.run(Command::SoftReset);
    assert_ne!(
        0,
        harness.host.session().unwrap().core().read_memory(0x0010)
    );

    harness.run(Command::Reset);
    harness.settle();
    assert_eq!(
        0,
        harness.host.session().unwrap().core().read_memory(0x0010)
    );
}

/// AMD-044 REQ-LOOP-072: the soft reset lifts the pause and resets the pacing clock, as the
/// hard reset does for the rest of the program.
#[test]
fn req_loop_072_a_soft_reset_lifts_the_pause() {
    let mut harness = launch_scripted();

    harness.run_frames(120);
    harness.run(Command::Pause);
    assert_eq!(EmulationState::Paused, harness.host.state());

    harness.run(Command::SoftReset);

    assert_eq!(EmulationState::Running, harness.host.state());
}

/// AMD-044 REQ-LOOP-072: the learned conversion state survives a soft reset, which is one of
/// the two things that distinguish it from a relaunch. The synthetic ROM of the harness draws
/// nothing, so the tile image the conversion would have interned is interned by hand and must
/// still be there afterwards. What a relaunch does instead is shown by the two tests above: it
/// clears the RAM and calls `End` then `Start`.
#[test]
fn req_loop_072_a_soft_reset_keeps_the_learned_conversion_state() {
    let mut harness = launch_scripted();
    harness.run_frames(120);
    {
        let (_, images) = harness
            .host
            .session_mut()
            .unwrap()
            .engine_mut()
            .registries_mut();
        images.intern(TileImageKey::new(
            0x0123_4567_89AB_CDEF,
            0xFEDC,
            false,
            false,
        ));
    }
    let learned = harness.host.session().unwrap().engine().images().count();
    assert_eq!(1, learned);

    harness.run(Command::SoftReset);

    assert_eq!(
        learned,
        harness.host.session().unwrap().engine().images().count(),
        "a soft reset must not unlearn anything (REQ-LOOP-072)"
    );
}

/// AMD-044 REQ-LOOP-072: the command does nothing outside a running game.
#[test]
fn req_loop_072_the_command_is_unavailable_without_a_game() {
    let mut harness = LoopHarness::create();
    assert!(!harness
        .host
        .can_execute(Command::SoftReset, &CommandArgs::default()));

    harness.run(Command::SoftReset);
    assert_eq!(EmulationState::Stopped, harness.host.state());

    harness.launch("g1");
    assert!(harness
        .host
        .can_execute(Command::SoftReset, &CommandArgs::default()));
}

/// AMD-044 §3: the two resets sit next to each other in the game menu, with the labels of the
/// table — `Hard Reset` / `Soft Reset` in English, `Reset complet` / `Reset doux` in French.
#[test]
fn req_loop_072_the_game_menu_offers_both_resets_with_the_amd_044_labels() {
    strings::set_language(UiLanguage::English);
    assert_eq!("Hard Reset", strings::get(&Command::Reset.label_key()));
    assert_eq!("Soft Reset", strings::get(&Command::SoftReset.label_key()));

    strings::set_language(UiLanguage::French);
    assert_eq!("Reset complet", strings::get(&Command::Reset.label_key()));
    assert_eq!("Reset doux", strings::get(&Command::SoftReset.label_key()));
    strings::set_language(UiLanguage::English);
}

/// AMD-044 REQ-INP-125: the action exists in the extended-action table with the next free code,
/// `0x80000`, and has no default binding at all.
#[test]
fn req_inp_125_the_soft_reset_action_has_the_next_free_code_and_no_default_binding() {
    assert_eq!(0x8_0000, InputAction::SOFT_RESET.bits());
    assert_eq!("SoftReset", input_action::name_of(InputAction::SOFT_RESET));
    assert_eq!(
        Some(InputAction::SOFT_RESET),
        input_action::parse("SoftReset")
    );
    assert!(input_action::ALL.contains(&InputAction::SOFT_RESET));
    assert!(InputAction::HOTKEY_MASK.contains(InputAction::SOFT_RESET));

    // No default binding, on the keyboard or on a pad: it goes through the game menu.
    assert!(!default_bindings::PLAYER_ONE_KEYBOARD
        .iter()
        .any(|(action, _)| *action == InputAction::SOFT_RESET));
    assert!(!default_bindings::standard_gamepad_set()
        .iter()
        .any(|(action, _)| *action == InputAction::SOFT_RESET));
    assert!(!default_bindings::create_player_one_keyboard()
        .get(&BindingTarget::of(InputAction::SOFT_RESET))
        .is_bound());
    assert!(!default_bindings::create_gamepad(true)
        .get(&BindingTarget::of(InputAction::SOFT_RESET))
        .is_bound());
}

/// AMD-044 REQ-INP-125 acceptance: the action is bindable in the controls screen and, once
/// bound, triggers the command of REQ-LOOP-072.
#[test]
fn req_inp_125_the_bound_action_triggers_the_soft_reset() {
    // The controls screen of REQ-UI-203 offers the action among the system hotkeys.
    let mut keys = KeyConfigModel::new();
    assert!(keys
        .all_rows()
        .any(|row| row.target.action() == InputAction::SOFT_RESET));

    let mut harness = launch_scripted();
    harness.run_frames(300);
    assert!(harness.host.rollback.count() > 0);
    let before = harness.host.session().unwrap().core().frame_count();
    assert!(before > 0);

    // One displayed frame with the action held: the rising edge queues the command, which the
    // same tick applies between two emulated frames, before the frames it then runs.
    harness.tick_actions(1.0 / 60.0, InputAction::SOFT_RESET, InputAction::NONE);

    let after = harness.host.session().unwrap().core().frame_count();
    assert!(after < before, "the core is still at frame {after}");
    // The history was emptied; the snapshots left are the recaptures taken by the frames that
    // followed the reset (REQ-LOOP-030).
    assert!(harness.host.rollback.count() <= 1);

    // Holding the action does not reset the console again and again: it is an edge.
    harness.run_frames_with(300, InputAction::SOFT_RESET);
    assert!(harness.host.session().unwrap().core().frame_count() >= 300);
}
