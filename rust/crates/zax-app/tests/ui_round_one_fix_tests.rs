//! The interface parts of `RoundOneFixTests.cs` (the rest is in `round_one_fix_tests.rs`).

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use ui_harness::{ImGuiHarness, UiHarness, UiLanguageScope};
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::persistence::save_slots::QUICK_SLOT_ID;
use zax_app::ui::fonts::UiFonts;
use zax_app::ui::strings::UiLanguage;
use zax_app::ui::text;
use zax_platform::imgui_glyphs;
use zax_platform::input::Key;

#[test] // REQ-INP-101, REQ-UI-004
fn req_inp_101_a_menu_press_during_a_load_is_dropped() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let rom = harness.write_rom("game.nes");
    harness
        .host
        .settings_store_mut()
        .expect("a settings store")
        .games_mut()
        .set_profile_for(&std::path::absolute(&rom).unwrap().to_string_lossy(), "g0");

    harness
        .host
        .enqueue(Command::OpenRom, CommandArgs::of_text(&rom));
    harness.host.apply_commands();
    assert!(harness.host.is_loading());

    // Pressed and released while the loading indicator is up.
    harness.keyboard.press(Key::Escape as i32);
    harness.tick(0.05);
    harness.keyboard.release(Key::Escape as i32);
    harness.tick(0.05);

    assert_eq!(0, harness.host.pending_commands());

    harness.settle();
    harness.tick(1.0 / 60.0);
    harness.tick(1.0 / 60.0);

    assert_eq!(EmulationState::Running, harness.host.state());
}

/// SPEC-10 REQ-UI-004 cases 2 and 3: a modal window takes the Menu press before the emulation
/// does. The file browser closes as with "Cancel" and the emulation keeps running; the
/// confirmation box E8 swallows the press and stays up (REQ-UI-110).
#[test] // REQ-UI-004, REQ-UI-110
fn req_ui_004_a_modal_window_takes_the_menu_press_before_the_emulation() {
    fn escape(harness: &mut UiHarness) {
        harness.keyboard.press(Key::Escape as i32);
        harness.tick(0.05);
        harness.keyboard.release(Key::Escape as i32);
        harness.tick(0.05);
        harness.host.apply_commands();
        harness.pump();
        harness.tick(1.0 / 60.0);
    }

    let mut harness = UiHarness::new(&["Arkanoid"]);
    harness.launch("g0");
    assert_eq!(EmulationState::Running, harness.host.state());

    // Case 3: the browser is open, Escape closes it and the game keeps running.
    harness
        .host
        .enqueue(Command::OpenRom, CommandArgs::default());
    harness.host.apply_commands();
    harness.pump();
    assert!(harness.ui.state.file_browser);
    harness
        .host
        .set_modal_window_open(harness.ui.state.is_modal());
    assert!(harness.host.modal_window_open());

    escape(&mut harness);
    assert!(!harness.ui.state.file_browser);
    assert_eq!(EmulationState::Running, harness.host.state());

    // Case 2: a confirmation box is open, Escape does nothing at all.
    harness
        .host
        .enqueue(Command::QuickSave, CommandArgs::default());
    harness.host.apply_commands();
    harness
        .host
        .enqueue(Command::DeleteSlot, CommandArgs::of_text(QUICK_SLOT_ID));
    harness.host.apply_commands();
    harness.pump();
    assert!(harness.ui.state.dialog);

    escape(&mut harness);
    assert!(harness.ui.state.dialog);
    assert_eq!(EmulationState::Running, harness.host.state());

    // Case 4, for contrast: with no modal window the press pauses the game.
    harness.host.cancel_pending();
    harness.pump();
    assert!(!harness.ui.state.dialog);
    escape(&mut harness);
    assert_eq!(EmulationState::Paused, harness.host.state());
}

#[test] // REQ-UI-037, REQ-UI-052
fn req_ui_037_the_menu_bar_offers_both_deletions() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    harness.launch("g0");
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(2));

    let host = &harness.host;
    let rows = harness.ui.menu_bar.all_items(host);

    assert!(rows
        .iter()
        .any(|row| row.command == Some(Command::DeleteGame)));
    assert!(rows
        .iter()
        .any(|row| row.command == Some(Command::DeleteSlot) && row.label.contains('2')));
    for row in rows.iter().filter(|row| row.command.is_some()) {
        assert_ne!(row.label, "");
    }
}

#[test] // REQ-UI-207, REQ-PUB-006
fn req_ui_207_the_triangle_of_the_welcome_hint_is_in_the_font_atlas() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    assert!(text::welcome_hint().contains('▸'));
    assert_eq!("File ▸ Open a ROM", text::welcome_hint());

    // AMD-035: the French hint keeps the amendment's wording, and is what the atlas has to carry
    // too: the triangle plus the accents.
    let french_hint = {
        let _french = UiLanguageScope::new(UiLanguage::French);
        let hint = text::welcome_hint();
        assert_eq!("Fichier ▸ Ouvrir une ROM", hint);
        hint
    };

    let mut imgui = ImGuiHarness::bare();
    let fonts = UiFonts::load(imgui.context_mut().fonts(), None, false);
    if fonts.body_file.is_none() {
        // No candidate face is installed; REQ-PUB-006 then allows the built-in font and its ASCII
        // substitutes, which carry no accent to check.
        return;
    }

    // Building the atlas is what fills the glyph index tables the remap works on.
    let atlas = imgui.context_mut().fonts();
    atlas.build_rgba32_texture();
    fonts.remap_missing_glyphs(atlas);

    for index in 0..2 {
        // Every character of the hint resolves to a real glyph rather than to the missing-glyph
        // box: the accents of AMD-010 and the triangle of REQ-UI-207, whether the face carries
        // U+25B8 itself or one of the substitutes.
        let hint = text::welcome_hint();
        for letter in hint.chars().chain(french_hint.chars()) {
            assert!(
                imgui_glyphs::carries(atlas, index, letter as u32),
                "{letter}"
            );
        }
        for accented in "ÉÈÊËÀÂÇÎÏÔÙÛÜæœ«»…".chars() {
            assert!(
                imgui_glyphs::carries(atlas, index, accented as u32),
                "{accented}"
            );
        }
    }
}
