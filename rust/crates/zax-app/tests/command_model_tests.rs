//! Port of `CommandModelTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::collections::HashSet;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs, Shortcut};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::emulation::host::HostEvent;
use zax_app::emulation::hotkey_tracker::{HotkeyEvents, HotkeyTracker};
use zax_app::emulation::notification_center::{
    NotificationCenter, NotificationSeverity, DEFAULT_SECONDS, SHORT_SECONDS,
};
use zax_app::emulation::speed_setting;
use zax_app::input::InputAction;
use zax_app::persistence::app_settings::AppSettings;

fn can(harness: &LoopHarness, command: Command) -> bool {
    harness.host.can_execute(command, &CommandArgs::default())
}

#[test]
fn req_ui_205_the_model_enumerates_every_menu_command() {
    use Command::*;
    let expected = [
        OpenRom,
        OpenRecentRom,
        ChooseProfile,
        ImportProfile,
        ImportThreeDSen,
        LaunchGame,
        LoadSlot,
        SaveSlot,
        QuickSave,
        QuickLoad,
        DeleteSlot,
        DeleteGame,
        CreateGameArt,
        ImportCover,
        CancelModal,
        Quit,
        PauseResume,
        Pause,
        Resume,
        Reset,
        Stop,
        FastForward,
        Rollback,
        SetSpeed,
        SpeedUp,
        SpeedDown,
        SetRegion,
        ToggleAutoRegion,
        SetPortType,
        ToggleTwoDimensional,
        ResetView,
        SetCameraMode,
        ToggleLocalCamera,
        SetSkybox,
        ToggleShadows,
        // Rust-only (owner request): the options of the game's built-in patch.
        ToggleHackOption,
        ToggleFullscreen,
        ToggleFpsCounter,
        SetVolume,
        VolumeUp,
        VolumeDown,
        ToggleMute,
        ToggleMenuMusic,
        ShowOptions,
        ShowKeyConfig,
        ShowDataFolder,
        ShowAbout,
        // Rust-only (owner request): the movies.
        ToggleMovieRecording,
        PlayMovie,
        StopMovie,
        AnnotateMovie,
    ];
    for command in expected {
        assert!(
            Command::ALL.contains(&command),
            "the command '{}' is missing",
            command.name()
        );
    }
    assert_eq!(expected.len(), Command::ALL.len());
}

#[test]
fn req_ui_205_command_names_are_stable_keys() {
    let mut seen = HashSet::new();
    for command in Command::ALL {
        assert!(
            seen.insert(command.name()),
            "'{}' is registered twice",
            command.name()
        );
        let parts: Vec<&str> = command.name().split('.').collect();
        assert!(parts.len() >= 2, "{}", command.name());
        assert!(parts.iter().all(|part| !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')));
        assert_eq!(Some(*command), Command::from_name(command.name()));
        assert_ne!(command.label_key(), "");
    }
}

#[test]
fn req_ui_204_the_default_shortcuts_are_carried_by_the_model() {
    assert_eq!(Shortcut::Fixed("Ctrl+O"), Command::OpenRom.shortcut());
    assert_eq!(Shortcut::Fixed("Ctrl+R"), Command::Reset.shortcut());
    assert_eq!(Shortcut::Fixed("Ctrl+Q"), Command::Quit.shortcut());
    assert_eq!(Shortcut::Fixed("P"), Command::PauseResume.shortcut());
    assert_eq!(
        Shortcut::Fixed("F11"),
        Command::ToggleTwoDimensional.shortcut()
    );
    // "Alt+Enter" is the English text of the interface key this shortcut names.
    assert_eq!(
        Shortcut::Key("shortcut.alt_enter"),
        Command::ToggleFullscreen.shortcut()
    );
    assert_eq!(Shortcut::Fixed("Ctrl+,"), Command::ShowOptions.shortcut());
}

#[test]
fn req_ui_205_availability_follows_the_state() {
    let mut harness = LoopHarness::create();
    assert!(!can(&harness, Command::Reset));
    assert!(!can(&harness, Command::Stop));
    assert!(!can(&harness, Command::QuickSave));
    assert!(!harness.host.invoke(Command::Reset, &CommandArgs::default()));
    harness.launch("g1");
    assert!(can(&harness, Command::Reset));
    assert!(can(&harness, Command::Stop));
    assert!(can(&harness, Command::QuickSave));
    // The quick slot is empty, so quick load is unavailable (REQ-LOOP-045).
    assert!(!can(&harness, Command::QuickLoad));
    harness.run(Command::QuickSave);
    assert!(can(&harness, Command::QuickLoad));
}

#[test]
fn req_ui_205_the_toggles_report_their_state() {
    let mut harness = LoopHarness::create();
    assert!(Command::ToggleShadows.is_toggle());
    assert!(harness.host.is_checked(Command::ToggleShadows));
    harness.run(Command::ToggleShadows);
    assert!(!harness.host.is_checked(Command::ToggleShadows));
    assert!(!harness.host.settings().shadow());
    assert!(!harness.host.scene().settings.shadows);
    assert!(Command::ToggleMenuMusic.is_toggle());
    assert!(harness.host.is_checked(Command::ToggleMenuMusic));
    harness.run(Command::ToggleMenuMusic);
    assert!(!harness.host.is_checked(Command::ToggleMenuMusic));
    assert!(!harness.host.mixer().expect("a mixer").menu_music_enabled());
}

#[test]
fn req_loop_015_the_speed_commands_bound_the_setting() {
    let mut harness = LoopHarness::create();
    harness.run_with(Command::SetSpeed, CommandArgs::of_number(4));
    assert_eq!(4, harness.host.settings().game_speed());
    assert!((harness.host.settings().speed_factor() - 2.0).abs() < 1e-9);
    for _ in 0..10 {
        harness.run(Command::SpeedUp);
    }
    assert_eq!(speed_setting::MAXIMUM, harness.host.settings().game_speed());
    for _ in 0..40 {
        harness.run(Command::SpeedDown);
    }
    assert_eq!(speed_setting::MINIMUM, harness.host.settings().game_speed());
}

#[test]
fn req_ui_205_ui_only_commands_raise_a_request() {
    let mut harness = LoopHarness::create();
    harness.host.take_events();
    for command in [
        Command::ShowOptions,
        Command::ShowKeyConfig,
        Command::ShowAbout,
        Command::ToggleFullscreen,
    ] {
        harness.run(command);
    }
    let requested: Vec<Command> = harness
        .host
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            HostEvent::UiRequested { command, .. } => Some(command),
            _ => None,
        })
        .collect();
    assert_eq!(
        vec![
            Command::ShowOptions,
            Command::ShowKeyConfig,
            Command::ShowAbout,
            Command::ToggleFullscreen
        ],
        requested
    );
    assert!(harness.host.settings().fullscreen());
}

#[test]
fn req_ui_081_notifications_do_not_stack() {
    let mut notifications = NotificationCenter::new();
    assert!(notifications.show_error("Invalid rom file."));
    // A normal notification cannot replace a visible error.
    assert!(!notifications.show_normal("Quick Save"));
    assert_eq!("Invalid rom file.", notifications.current().unwrap().text);
    notifications.advance(4.0);
    assert!(notifications.current().is_none());
    // Once the error is gone, the normal one is accepted.
    assert!(notifications.show_normal("Quick Save"));
    assert_eq!("Quick Save", notifications.current().unwrap().text);
    // An equal or higher severity replaces and restarts the timer.
    notifications.advance(3.0);
    assert!(notifications.show("Roll Back", SHORT_SECONDS, NotificationSeverity::Normal));
    assert!((notifications.remaining() - 1.0).abs() < 1e-9);
}

#[test]
fn req_ui_080_the_timer_runs_in_real_time() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run(Command::QuickSave);
    assert_eq!(
        "Quick Save",
        harness.host.notifications.current().unwrap().text
    );
    harness.run(Command::Pause);
    for _ in 0..50 {
        harness.tick(0.1);
    }
    assert!(harness.host.notifications.current().is_none());
}

#[test]
fn req_ui_082_the_in_game_notifications_have_their_exact_texts() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(120);
    let latest = |harness: &LoopHarness| {
        let last = harness.host.notifications.log().last().unwrap();
        (last.text.clone(), last.seconds)
    };
    harness.run(Command::Rollback);
    assert_eq!(("Roll Back".to_owned(), 1.0), latest(&harness));
    harness.run(Command::FastForward);
    assert_eq!(("Fast Forward".to_owned(), 1.0), latest(&harness));
    harness.run(Command::QuickSave);
    assert_eq!(("Quick Save".to_owned(), DEFAULT_SECONDS), latest(&harness));
    harness.run(Command::QuickLoad);
    assert_eq!(("Quick Load".to_owned(), DEFAULT_SECONDS), latest(&harness));
}

#[test]
fn req_ui_082_the_quick_actions_are_silent_when_they_do_nothing() {
    let mut harness = LoopHarness::create();
    harness.run(Command::QuickSave);
    harness.run(Command::QuickLoad);
    assert_eq!(harness.host.notifications.log(), []);
}

#[test]
fn req_loop_007_the_queue_is_first_in_first_out() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.host.enqueue(Command::Pause, CommandArgs::default());
    harness
        .host
        .enqueue(Command::Resume, CommandArgs::default());
    harness.tick_frame();
    assert_eq!(EmulationState::Running, harness.host.state());
    harness
        .host
        .enqueue(Command::Resume, CommandArgs::default());
    harness.host.enqueue(Command::Pause, CommandArgs::default());
    harness.tick_frame();
    assert_eq!(EmulationState::Paused, harness.host.state());
}

#[test]
fn req_inp_101_the_menu_press_length_decides() {
    let mut tracker = HotkeyTracker::new();
    // A 0.1 s press is short.
    assert_eq!(
        HotkeyEvents::NONE,
        tracker.update_mask(InputAction::MENU, 0.1)
    );
    assert_eq!(
        HotkeyEvents::MENU_SHORT,
        tracker.update_mask(InputAction::NONE, 0.0)
    );
    // A 0.5 s press is long and reports as soon as the threshold is crossed.
    assert_eq!(
        HotkeyEvents::NONE,
        tracker.update_mask(InputAction::MENU, 0.0)
    );
    assert_eq!(
        HotkeyEvents::MENU_LONG,
        tracker.update_mask(InputAction::MENU, 0.4)
    );
    assert_eq!(
        HotkeyEvents::NONE,
        tracker.update_mask(InputAction::NONE, 0.0)
    );
}

#[test]
fn req_inp_103_the_hotkeys_produce_their_events() {
    let mut tracker = HotkeyTracker::new();
    assert!(tracker
        .update_mask(InputAction::FAST_FORWARD, 0.01)
        .contains(HotkeyEvents::FAST_FORWARD));
    assert!(tracker
        .update_mask(InputAction::FAST_FORWARD, 0.01)
        .contains(HotkeyEvents::FAST_FORWARD));
    // A rising edge only for RollBack.
    assert!(tracker
        .update_mask(InputAction::ROLL_BACK, 0.01)
        .contains(HotkeyEvents::ROLLBACK));
    assert!(!tracker
        .update_mask(InputAction::ROLL_BACK, 0.01)
        .contains(HotkeyEvents::ROLLBACK));
    // Select held more than 0.1 s, then RotDown released: quick save.
    tracker.update_mask(InputAction::SELECT, 0.2);
    tracker.update_mask(InputAction::SELECT | InputAction::ROT_DOWN, 0.02);
    assert!(tracker
        .update_mask(InputAction::SELECT, 0.02)
        .contains(HotkeyEvents::QUICK_SAVE));
    // The same with RotUp: quick load.
    tracker.update_mask(InputAction::SELECT | InputAction::ROT_UP, 0.02);
    assert!(tracker
        .update_mask(InputAction::SELECT, 0.02)
        .contains(HotkeyEvents::QUICK_LOAD));
}

#[test]
fn req_loop_005_the_menu_hotkey_toggles_the_pause() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    let none = InputAction::NONE;
    harness.tick_actions(0.1, InputAction::MENU, none);
    harness.tick_actions(0.01, none, none);
    harness.tick_frame();
    assert_eq!(EmulationState::Paused, harness.host.state());
    harness.tick_actions(0.1, InputAction::MENU, none);
    harness.tick_actions(0.01, none, none);
    harness.tick_frame();
    assert_eq!(EmulationState::Running, harness.host.state());
    // A long press leaves the state alone. The press is held over several rendered frames, as
    // it is in the real loop.
    harness.tick_actions(0.1, InputAction::MENU, none);
    harness.tick_actions(0.2, InputAction::MENU, none);
    harness.tick_actions(0.2, InputAction::MENU, none);
    harness.tick_actions(0.01, none, none);
    harness.tick_frame();
    assert_eq!(EmulationState::Running, harness.host.state());
}
