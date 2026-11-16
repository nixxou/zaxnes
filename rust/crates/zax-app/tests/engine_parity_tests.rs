//! The application's **Zaxnes** path, measured so that the two builds can be compared.
//!
//! The owner's rule for lot FCEUX-23 is that a build with `fceux-core` but `engine = zaxnes`
//! everywhere behaves exactly like a build without the feature. Most of that is argued by the
//! test suite — every test that does not need the feature passes in both builds — and by
//! golden-core being byte-identical. This file is the part that can be *measured*: it plays a real
//! game through the real host for 600 frames and prints a digest of the machine and of the 3D draw
//! list, so the same test run in the two builds either prints the same line or does not.
//!
//! The test is compiled in **both** builds, deliberately: that is what makes the comparison
//! possible. It is `#[ignore]`d because it needs a commercial ROM and the profile catalogue.

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_core_api::EngineKind;
use zax_profiles::catalogue::{ProfileCatalogue, FULL_EDITION};
use zax_profiles::profile::GameProfile;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

/// The game and the profile: *Super Mario Bros. 3*, which is the one the lot's other figures use.
const ROM: &str = "Super Mario Bros. 3 [USA][!!].nes";
const TITLE: &str = "Super Mario Bros. 3";
const FRAMES: u32 = 600;

fn rom() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(root) = std::env::var("ZAXNES_ROMS") {
        roots.push(PathBuf::from(root));
    }
    roots.push(PathBuf::from("G:/3DSen/test-originals"));

    for root in &roots {
        let path = root.join(ROM);
        if path.is_file() {
            return Some(path);
        }
    }
    eprintln!("skipped: {ROM} is in none of {roots:?}");
    None
}

fn profile() -> Option<GameProfile> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(root) = std::env::var("ZAXNES_PROFILES") {
        roots.push(PathBuf::from(root));
    }
    roots.push(PathBuf::from("G:/3dsen/cleanroom/assets-neutral"));

    let root = roots
        .iter()
        .find(|root| root.join("profiles.json").is_file())?;
    let files = zax_app::physical_file_system::PhysicalProfileFileSystem;
    let catalogue =
        ProfileCatalogue::load(&files, &root.join("profiles.json"), FULL_EDITION).ok()?;
    let entry = catalogue
        .entries()
        .iter()
        .find(|entry| entry.title.eq_ignore_ascii_case(TITLE))?
        .clone();
    Some(catalogue.load_profile(&files, &entry, None).ok()?.profile)
}

/// The same line in a build with `fceux-core` and in one without, or the owner's rule is broken.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn the_applications_zaxnes_path_prints_the_same_digest_in_both_builds() {
    let Some(path) = rom() else { return };
    let Some(profile) = profile() else { return };
    let image = std::fs::read(path).unwrap();

    let mut harness = LoopHarness::create_with(|options| {
        options.settings_store = Some(zax_app::persistence::settings_store::SettingsStore::new(
            &options.paths,
        ));
    });
    let rom_path = harness.write_rom(&image, ROM);
    harness
        .catalog()
        .add_with("game", Some(&rom_path), Some(profile), false, Some(TITLE));
    // Explicitly Zaxnes, which is also the default: the point is that the *stored* choice is the
    // one both builds act on.
    harness.host.set_game_options(
        &rom_path,
        vec![(SettingId::EmuEngine, SettingValue::Enum(0))],
    );
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("game"));
    harness.settle();
    harness.run_frames(FRAMES);

    assert_eq!(harness.host.running_engine(), Some(EngineKind::Zaxnes));

    let session = harness.host.session().expect("a session");
    let frames = session.core().frame_count();
    let cycles = session.core().cycle_count();
    let records = session.core().frame_batch().records.len();

    // The machine: the work RAM and the frame's own records and palette.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for address in 0..0x0800_u16 {
        session.core().read_memory(address).hash(&mut hasher);
    }
    let batch = session.core().frame_batch();
    for record in batch.records {
        record.hash(&mut hasher);
    }
    batch.palette.hash(&mut hasher);
    let machine = hasher.finish();

    // And the 3D path: the draw list the renderer would be handed.
    let session = harness.host.session_mut().expect("a session");
    let list = session.build_draw_list();
    let tiles = list.tiles().len();
    let meshes = list
        .tiles()
        .iter()
        .filter(|tile| tile.mesh.is_some())
        .count();

    println!(
        "zaxnes path, fceux-core {}: frames {frames} cycles {cycles} records {records} \
         machine {machine:016x} draw tiles {tiles} meshes {meshes}",
        if cfg!(feature = "fceux-core") {
            "on"
        } else {
            "off"
        }
    );

    assert_eq!(frames, i64::from(FRAMES));
    assert!(records > 0 && tiles > 0);
}

/// Lot FCEUX-48 step 4: **Zaxnes plays no movie of its own**, in either build.
///
/// That one fact is what keeps the default build's `.fm2` behaviour exactly as it was: the
/// application asks the engine, and Zaxnes's answer sends every `.fm2` through the clean-room
/// importer of `zax_nes::movie::fm2` as before. A build without `fceux-core` has only this
/// engine, so the question never arises there at all.
#[test]
fn zaxnes_plays_no_movie_of_its_own_so_the_fm2_path_is_the_one_it_always_was() {
    let core = zax_app::emulation::engine_handle::EngineHandle::zaxnes_core();
    assert_eq!(core.kind(), EngineKind::Zaxnes);
    assert!(
        !core.plays_its_own_movies(),
        "Zaxnes's own core must send an .fm2 to the importer, not to a movie mode"
    );
    assert_eq!(core.own_movie_position(), None);
    // The four calls answer "not me" rather than failing, which is what lets the host ask without
    // knowing which engine it holds.
    let movie = b"version 3\nport0 1\n|0|........|........||\n";
    assert!(core.inspect_own_movie(movie).is_none());
    let mut core = core;
    assert!(core.play_own_movie(movie).is_none());
    core.stop_own_movie();
    assert_eq!(core.own_movie_position(), None);
}

/// And the engine a build has decides it: with `fceux-core` there is an engine that does play its
/// own movies, without it there is not. Stated as a test so that the two builds disagree here on
/// purpose and nowhere else.
#[test]
fn only_a_build_with_the_feature_has_an_engine_that_plays_its_own_movies() {
    let fceux = zax_app::emulation::engine_handle::EngineHandle::of_kind(EngineKind::Fceux);
    if cfg!(feature = "fceux-core") {
        assert_eq!(fceux.kind(), EngineKind::Fceux);
        assert!(fceux.plays_its_own_movies());
    } else {
        // Without the feature `of_kind` falls back to Zaxnes's core with a line in the log.
        assert_eq!(fceux.kind(), EngineKind::Zaxnes);
        assert!(!fceux.plays_its_own_movies());
    }
}
