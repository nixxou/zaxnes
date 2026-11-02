//! `emu.pause_3d_in_2d` (Rust-only, owner request): while the 2D picture is shown, a profile whose
//! scripts leave the core alone is neither converted nor run through its frame hooks, and the 3D
//! comes back converted.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{LoopHarness, SyntheticRom};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::frame_profiler::Stage;
use zax_app::persistence::app_settings::AppSettings as _;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

/// A 3D game whose frame hook counts its calls into the field of view, launched with
/// `emu.pause_3d_in_2d` at `paused`, run a little, its stage times kept.
fn game(paused: bool) -> LoopHarness {
    let mut harness = LoopHarness::create();
    harness.host.update_settings(|model| {
        model.set(SettingId::EmuPause3dIn2d, SettingValue::Bool(paused));
    });
    harness.host.set_profiling(true);
    let mut profile = GameProfile::create_empty();
    // A profile with a pattern runs in 3D.
    profile
        .temporary_patterns
        .push(Arc::new(ProfilePattern::default()));
    profile.scripts.push(ProfileScript {
        name: Some("counter".to_owned()),
        enabled: true,
        source: Some(
            "local calls = 0
function LateUpdate()
  calls = calls + 1
  Setting.Fov = 45 + calls % 40
end
"
            .to_owned(),
        ),
        ..ProfileScript::default()
    });
    let rom = harness.write_rom(&SyntheticRom::counter(), "g.nes");
    harness
        .catalog()
        .add_with("g", Some(&rom), Some(profile), false, None);
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("g"));
    harness.run_frames(5);
    assert!(!harness.host.showing_two_dimensional());
    harness
}

/// The conversion time of `frames` frames run now, and the field of view after them.
fn run(harness: &mut LoopHarness, frames: u32) -> (Duration, u32) {
    harness.host.take_stage_times();
    assert_eq!(frames, harness.run_frames(frames));
    let conversion = harness.host.take_stage_times().get(Stage::Conversion);
    (conversion, harness.host.settings().fov().to_bits())
}

#[test]
fn nothing_of_the_3d_runs_in_2d_while_it_is_paused() {
    let mut paused = game(true);
    paused.run(Command::ToggleTwoDimensional);
    let (_, entered) = run(&mut paused, 1);
    let (conversion, fov) = run(&mut paused, 20);
    assert_eq!(Duration::ZERO, conversion);
    assert_eq!(entered, fov, "the frame hook ran");

    let mut running = game(false);
    running.run(Command::ToggleTwoDimensional);
    let (_, entered) = run(&mut running, 1);
    let (conversion, fov) = run(&mut running, 20);
    assert!(conversion > Duration::ZERO);
    assert_ne!(entered, fov);
}

#[test]
fn the_3d_comes_back_converted_and_its_scripts_go_on() {
    let mut harness = game(true);
    harness.run(Command::ToggleTwoDimensional);
    let (_, paused) = run(&mut harness, 10);
    harness.run(Command::ToggleTwoDimensional);
    assert!(!harness.host.showing_two_dimensional());
    let (conversion, fov) = run(&mut harness, 1);
    assert!(conversion > Duration::ZERO);
    assert_ne!(paused, fov);
}

/// The frame hooks run during one displayed frame that catches `due` emulated frames up, the
/// core on its own thread and ahead when `ahead`.
fn hooks_of_a_catch_up(last_3d: bool, due: u32, ahead: bool) -> u32 {
    let mut harness = game(false);
    harness.host.separate_core = ahead;
    harness.host.core_ahead = ahead;
    harness.host.update_settings(|model| {
        model.set(SettingId::EmuCatchUpLast3d, SettingValue::Bool(last_3d));
        // The catch-up of AMD-052 REQ-LOOP-074 stops at 2 frames; this test is about what a
        // catch-up of several frames converts, so it asks for the limit the handheld uses.
        model.set(SettingId::EmuCatchUp, SettingValue::Enum(5));
    });
    let before = harness.host.settings().fov();
    let period = 1.0 / harness.host.pacer.effective_rate().max(1.0);
    assert_eq!(due, harness.tick(period * f64::from(due)));
    let after = harness.host.settings().fov();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..40.
    let hooks = (after - before).rem_euclid(40.0).round() as u32;
    hooks
}

#[test]
fn a_catch_up_converts_the_frame_it_shows_only() {
    for ahead in [false, true] {
        let all = hooks_of_a_catch_up(false, 4, ahead);
        let shown = hooks_of_a_catch_up(true, 4, ahead);
        // Ahead, the last frame is still in flight: it is converted at the next tick.
        assert_eq!(if ahead { 3 } else { 4 }, all, "ahead {ahead}");
        assert!(
            (1..=2).contains(&shown),
            "ahead {ahead}: {shown} frame hooks ran"
        );
    }
}
