//! Port of `SaveStateTests.cs`.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;
use rstest::rstest;
use time::OffsetDateTime;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::input::InputAction;
use zax_app::persistence::save_slots::{self, SaveSlotIndex, QUICK_SLOT_ID};
use zax_app::persistence::save_state_file::{
    self, CAMERA_SECTION_BYTES, CORRUPTED_MESSAGE, GAME_MISSING_MESSAGE, LEGACY_SIGNATURE,
    SIGNATURE,
};
use zax_nes::NesCore;
use zax_scripting::persistent;

#[test]
fn req_pers_056_the_file_has_the_signature_and_the_four_sections() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(3));
    let path = harness.paths.slot_state_file("3");
    assert!(path.is_file());
    let file = std::fs::read(path).unwrap();
    assert_eq!(&file[..8], SIGNATURE);
    let content = save_state_file::try_read(&file).expect("a readable state");
    assert_eq!(NesCore::CORE_NAME, content.header.core_id);
    assert_eq!("g1", content.header.game_id);
    assert_eq!(harness.now(), content.header.saved_at);
    assert_eq!("3", content.header.slot_id);
    assert_ne!(content.core, Vec::<u8>::new());
    assert_eq!(persistent::BLOCK_SIZE, content.lua.len());
    assert_eq!(CAMERA_SECTION_BYTES, content.camera.len());
    assert_ne!(content.conversion, Vec::<u8>::new());
    assert_eq!(40, content.header.rom_sha1.len());
}

#[test]
fn req_pers_056_two_saves_without_a_frame_are_identical() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.run(Command::Pause);
    let first = harness.host.capture_save_state("1").unwrap();
    let second = harness.host.capture_save_state("1").unwrap();
    assert_eq!(first.core, second.core);
    assert_eq!(first.lua, second.lua);
    assert_eq!(first.camera, second.camera);
    assert_eq!(first.conversion, second.conversion);
}

#[test]
fn req_arch_012_a_savestate_replays_identically() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(90);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(1));
    let sequence = [InputAction::NONE, InputAction::A, InputAction::RIGHT];
    harness.run_exact_frames(120, &sequence);
    let first_run = harness
        .host
        .session()
        .unwrap()
        .core()
        .save_state(false)
        .unwrap();
    harness.run_with(Command::LoadSlot, CommandArgs::of_number(1));
    assert_eq!(EmulationState::Running, harness.host.state());
    harness.run_exact_frames(120, &sequence);
    let second_run = harness
        .host
        .session()
        .unwrap()
        .core()
        .save_state(false)
        .unwrap();
    assert_eq!(first_run, second_run);
}

#[test]
fn req_arch_013_the_file_stays_under_the_announced_bound() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(60);
    let bound = harness.host.max_save_state_size();
    assert!(bound > 0);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(2));
    let written = std::fs::metadata(harness.paths.slot_state_file("2"))
        .unwrap()
        .len();
    assert!(
        written <= bound as u64,
        "{written} bytes written, bound announced {bound}"
    );
}

#[test]
fn req_loop_046_a_corrupt_savestate_is_refused() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(4));
    // Truncate the file in place; the index still says the slot is occupied.
    let path = harness.paths.slot_state_file("4");
    let file = std::fs::read(&path).unwrap();
    std::fs::write(&path, &file[..file.len() / 3]).unwrap();
    harness.run_with(Command::LoadSlot, CommandArgs::of_number(4));
    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert!(harness.host.current_game_id().is_none());
    assert!(!harness.host.is_loading());
    assert_eq!(
        CORRUPTED_MESSAGE,
        harness.host.notifications.current().unwrap().text
    );
}

#[rstest]
#[case("WRONGSIG")]
#[case("ZAXNESAV")]
#[case("SEN3DSAV")]
fn req_pers_056_the_signature_and_version_are_checked(#[case] signature: &str) {
    let mut file = vec![0_u8; 64];
    file[..8].copy_from_slice(signature.as_bytes());
    // Format version 99 for the right signature, so both rows are refused.
    file[8] = 99;
    assert!(save_state_file::try_read(&file).is_none());
}

fn replace_core_name(block: &[u8], name: &str) -> Vec<u8> {
    let stored_length =
        usize::try_from(i32::from_le_bytes(block[8..12].try_into().unwrap())).unwrap();
    let replacement = name.as_bytes();
    let mut result = block[..8].to_vec();
    result.extend_from_slice(&i32::try_from(replacement.len()).unwrap().to_le_bytes());
    result.extend_from_slice(replacement);
    result.extend_from_slice(&block[12 + stored_length..]);
    result
}

#[test]
fn amd_029_a_savestate_of_the_old_name_is_still_read() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(45);
    let written = harness.host.capture_save_state("1").unwrap();
    let mut file = save_state_file::write(&written);
    // Both signatures are eight bytes, so nothing after them moves.
    assert_eq!(SIGNATURE.len(), LEGACY_SIGNATURE.len());
    file[..8].copy_from_slice(LEGACY_SIGNATURE);
    assert!(save_state_file::has_known_signature(&file[..8]));
    let content = save_state_file::try_read(&file).expect("a readable state");
    assert_eq!(written.core, content.core);
    assert_eq!(written.lua, content.lua);
    assert_eq!(written.camera, content.camera);
    assert_eq!(written.conversion, content.conversion);
    // And the core block itself: the legacy core name is accepted, a third one never is.
    let mut core = NesCore::new();
    let legacy = replace_core_name(&content.core, NesCore::LEGACY_CORE_NAME);
    assert!(core.try_load_state(&legacy, 0).is_ok());
    let mut foreign = NesCore::new();
    let error = foreign
        .try_load_state(&replace_core_name(&content.core, "Nestopia"), 0)
        .unwrap_err();
    assert_eq!(NesCore::FOREIGN_CORE_MESSAGE, error.message);
}

#[test]
fn req_pers_056_an_inconsistent_section_length_is_refused() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(10);
    let content = harness.host.capture_save_state("1").unwrap();
    let mut file = save_state_file::write(&content);
    // The core section's length prefix sits right after the signature, the version and the
    // metadata section; raising it past the end of the file must be refused.
    let metadata_length =
        usize::try_from(i32::from_le_bytes(file[12..16].try_into().unwrap())).unwrap();
    let offset = 8 + 4 + 4 + metadata_length;
    file[offset..offset + 4].copy_from_slice(&i32::MAX.to_le_bytes());
    assert!(save_state_file::try_read(&file).is_none());
}

#[test]
fn req_loop_043_the_quick_slot_has_no_thumbnail() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.set_thumbnail(Some(vec![0; 256 * 192 * 3]));
    harness.run_frames(60);
    harness.run(Command::QuickSave);
    let quick = harness.host.slots.find(QUICK_SLOT_ID).unwrap();
    assert!(quick.is_occupied());
    assert!(quick.thumbnail.is_none());
    assert!(!harness.paths.slot_thumbnail_file(QUICK_SLOT_ID).exists());
    let current = harness.host.notifications.current().unwrap();
    assert_eq!("Quick Save", current.text);
    assert!((current.seconds - 4.0).abs() < f64::EPSILON);
    harness.run_frames(60);
    let after = harness.host.session().unwrap().core().frame_count();
    harness.run(Command::QuickLoad);
    let frames = harness.host.session().unwrap().core().frame_count();
    assert_eq!(60, frames);
    assert_ne!(after, frames);
    assert_eq!(
        "Quick Load",
        harness.host.notifications.current().unwrap().text
    );
}

#[test]
fn req_loop_045_an_empty_quick_slot_does_nothing() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    let frames = harness.host.session().unwrap().core().frame_count();
    harness.run(Command::QuickLoad);
    assert_eq!(EmulationState::Running, harness.host.state());
    assert_eq!(frames, harness.host.session().unwrap().core().frame_count());
    assert!(harness.host.notifications.current().is_none());
}

#[test]
fn req_loop_041_a_save_does_not_change_the_state() {
    let mut harness = LoopHarness::create();
    // Stopped, no game: nothing happens.
    harness.run(Command::QuickSave);
    assert!(!harness
        .host
        .slots
        .find(QUICK_SLOT_ID)
        .unwrap()
        .is_occupied());
    harness.launch("g1");
    harness.run_frames(30);
    harness.run(Command::QuickSave);
    assert_eq!(EmulationState::Running, harness.host.state());
    harness.run(Command::Pause);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(1));
    assert_eq!(EmulationState::Paused, harness.host.state());
}

#[test]
fn req_loop_042_a_numbered_slot_carries_the_game_the_thumbnail_and_the_date() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    let mut capture = vec![0_u8; 256 * 192 * 3];
    capture[3] = 0x40;
    harness.set_thumbnail(Some(capture));
    harness.run_frames(60);
    harness.run(Command::Pause);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(5));
    let slot = harness.host.slots.find("5").unwrap();
    assert_eq!("g1", slot.game_id);
    assert_eq!(harness.now(), slot.saved_at);
    assert_eq!(Some("5.png"), slot.thumbnail.as_deref());
    assert!(harness.paths.slot_thumbnail_file("5").is_file());
    // The index was written straight away, before the process could die.
    assert!(harness.paths.save_index_file.is_file());
    let now = OffsetDateTime::from_unix_timestamp(harness.now()).unwrap();
    let reread = SaveSlotIndex::load(&harness.paths, now);
    assert_eq!("g1", reread.find("5").unwrap().game_id);
    // Overwriting is silent.
    harness.set_now(harness.now() + 300);
    harness.run(Command::Resume);
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(5));
    assert_eq!(
        harness.now(),
        harness.host.slots.find("5").unwrap().saved_at
    );
}

#[test]
fn req_loop_044_a_slot_of_a_deleted_game_reports_the_exact_message() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(2));
    harness.catalog().remove("g1");
    harness.run_with(Command::LoadSlot, CommandArgs::of_number(2));
    assert_eq!(EmulationState::Stopped, harness.host.state());
    assert_eq!(
        GAME_MISSING_MESSAGE,
        harness.host.notifications.current().unwrap().text
    );
}

#[test]
fn req_pers_053_a_slot_loads_without_the_rom_file() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(120);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(7));
    harness.run(Command::Stop);
    assert_eq!(EmulationState::Stopped, harness.host.state());
    // The ROM file disappears and the association is cleared.
    std::fs::remove_file(harness.root.path().join("g1.nes")).unwrap();
    harness.catalog().add("g1", None);
    harness.run_with(Command::LoadSlot, CommandArgs::of_number(7));
    assert_eq!(EmulationState::Running, harness.host.state());
    assert_eq!(Some("g1"), harness.host.current_game_id());
    assert_eq!(120, harness.host.session().unwrap().core().frame_count());
}

#[test]
fn req_loop_040_the_camera_comes_back_with_the_state() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    let set_distance = |harness: &mut LoopHarness, distance: f32| {
        let scene = harness.host.scene_mut();
        scene.camera.current_mut().set_distance(distance);
        scene.camera.snap_to_setpoints();
    };
    set_distance(&mut harness, 200.0);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(6));
    set_distance(&mut harness, 40.0);
    harness.run_with(Command::LoadSlot, CommandArgs::of_number(6));
    assert!((harness.host.scene().camera.current().distance() - 200.0).abs() < 1e-3);
}

#[test]
fn req_pers_054_deleting_a_slot_removes_its_file() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(8));
    assert!(harness.paths.slot_state_file("8").is_file());
    harness.run_with(Command::DeleteSlot, CommandArgs::of_number(8));
    assert!(!harness.host.slots.find("8").unwrap().is_occupied());
    assert!(!harness.paths.slot_state_file("8").exists());
}

#[test]
fn req_pers_050_the_model_holds_thirty_seven_slots() {
    let index = SaveSlotIndex::new();
    assert_eq!(save_slots::NUMBERED_SLOT_COUNT + 1, index.slots().len());
    assert!(index.find("1").is_some());
    assert!(index.find("36").is_some());
    assert!(index.find(QUICK_SLOT_ID).is_some());
    assert!(index.find("37").is_none());
    assert_eq!(10, save_slots::EXPOSED_SLOT_COUNT);
    assert!(!save_slots::is_valid_slot_id("0"));
    assert!(!save_slots::is_valid_slot_id("37"));
}

#[test]
fn the_header_carries_the_hack_the_version_and_a_thumbnail() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(3));
    let file = std::fs::read(harness.paths.slot_state_file("3")).unwrap();
    let content = save_state_file::try_read(&file).expect("a readable state");
    // The test ROM has no built-in patch.
    assert_eq!(Some(false), content.header.is_hack);
    assert_eq!(None, content.header.hack_version);
    assert_eq!(
        Some(env!("CARGO_PKG_VERSION")),
        content.header.app_version.as_deref()
    );
    assert!(content.header.saved_at_local.is_some());
    assert_eq!(&[0xFF, 0xD8], &content.thumbnail[..2], "a JPEG");
}

#[test]
fn a_file_without_a_thumbnail_still_reads() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.run_frames(30);
    let mut content = harness.host.capture_save_state("1").unwrap();
    content.thumbnail.clear();
    let read = save_state_file::try_read(&save_state_file::write(&content)).unwrap();
    assert_eq!(content, read);
}

#[test]
fn a_wide_thumbnail_keeps_the_centre_in_4_3() {
    // 432 x 240: the 320 columns of the centre are kept.
    let picture = vec![0x21u8; 432 * 240];
    let jpeg = save_state_file::thumbnail_jpeg(&picture).unwrap();
    // The SOF0 frame header holds height then width.
    let at = jpeg.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
    let height = u16::from_be_bytes([jpeg[at + 5], jpeg[at + 6]]);
    let width = u16::from_be_bytes([jpeg[at + 7], jpeg[at + 8]]);
    assert_eq!((320, 240), (width, height));
    assert!(save_state_file::thumbnail_jpeg(&[0; 100]).is_none());
}
