//! The widescreen hack switched while its game plays (owner request): a game whose pack carries
//! its states both ways goes onto the patched ROM, or off it, at once, where it was.
//!
//! The game is `FlappyZaxNes`, the demo whose ROM is kept out of git in `demo/`; without it the
//! test says so and passes.

mod common;

use std::path::PathBuf;

use common::LoopHarness;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

fn demo_rom() -> Option<Vec<u8>> {
    std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demo/FlappyZaxNes.nes")).ok()
}

fn patched(harness: &LoopHarness) -> bool {
    harness
        .host
        .session()
        .is_some_and(|session| session.core().original_image().is_some())
}

fn set_widescreen(harness: &mut LoopHarness, choice: i32) {
    harness.host.update_settings(|model| {
        model.set(SettingId::VideoWidescreen, SettingValue::Enum(choice));
    });
}

#[test]
fn the_widescreen_of_a_portable_pack_switches_while_playing() {
    let Some(rom) = demo_rom() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let mut harness = LoopHarness::create();
    harness.host.update_settings(|model| {
        model.set(SettingId::EmuBuiltinPatches, SettingValue::Bool(true));
        model.set(SettingId::VideoWidescreen, SettingValue::Enum(0));
    });
    harness.launch_rom_with_script("flappy", &rom, None);
    harness.settle();
    harness.run_frames(120);
    assert!(!patched(&harness), "launched with the widescreen off");
    let frame = harness
        .host
        .session()
        .map(|session| session.core().frame_count());

    set_widescreen(&mut harness, 1);
    assert!(patched(&harness), "on: the patched ROM at once");
    assert_eq!(
        frame,
        harness
            .host
            .session()
            .map(|session| session.core().frame_count()),
        "where it was"
    );
    harness.run_frames(120);
    assert!(patched(&harness));

    set_widescreen(&mut harness, 0);
    assert!(!patched(&harness), "off: the original ROM at once");
    harness.run_frames(120);
    assert!(!patched(&harness));

    // The way the hacks screens go: an option of the game rather than the global setting.
    let rom_path = harness.write_rom(&rom, "flappy.nes");
    harness.host.set_game_options(
        &rom_path,
        vec![(SettingId::VideoWidescreen, SettingValue::Enum(1))],
    );
    assert!(patched(&harness), "the game's own widescreen on");
    harness.host.set_game_options(&rom_path, Vec::new());
    assert!(!patched(&harness), "the game's own widescreen gone");
}

/// Portrait (owner request): the widescreen held off while the screen is too narrow, the
/// settings untouched, and back on with the landscape.
#[test]
fn the_portrait_screen_holds_the_widescreen_off_without_the_settings() {
    let Some(rom) = demo_rom() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let mut harness = LoopHarness::create();
    harness.host.update_settings(|model| {
        model.set(SettingId::EmuBuiltinPatches, SettingValue::Bool(true));
        model.set(SettingId::VideoWidescreen, SettingValue::Enum(1));
    });
    harness.launch_rom_with_script("flappy", &rom, None);
    harness.settle();
    harness.run_frames(120);
    assert!(patched(&harness), "launched wide");

    harness.host.set_widescreen_held_off(true);
    assert!(!patched(&harness), "portrait: the original ROM at once");
    assert_eq!(
        1,
        harness
            .host
            .settings()
            .get(SettingId::VideoWidescreen)
            .as_int(),
        "the setting as it was"
    );
    harness.run_frames(120);
    assert!(!patched(&harness));

    harness.host.set_widescreen_held_off(false);
    assert!(patched(&harness), "landscape: wide again");
}
