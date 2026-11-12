//! The application playing on FCEUX, end to end (lot FCEUX-23 step 6).
//!
//! The headless path is the one the application's own integration tests use: `LoopHarness` drives
//! `EmulationHost` — the real host, the real session, the real conversion engine, the real
//! settings and slots — without a window, which is what the brief means by "whatever headless or
//! scripted path the app and harness offer". Nothing here needs a human and nothing opens a GPU
//! context: the 3D path is exercised as far as the draw list, which is where FCEUX's records
//! actually arrive.
//!
//! Every test is `#[ignore]`d because each needs a commercial ROM (`ZAXNES_ROMS`, or
//! `G:/3DSen/test-originals` style folders) and the profile catalogue (`ZAXNES_PROFILES`), and
//! **the whole file needs the `fceux-core` feature**: with it off there is one engine and nothing
//! here would mean anything.
//!
//! They also serialise themselves: FCEUX keeps one machine in process globals, so two harnesses
//! must never hold an engine at once.

#![cfg(feature = "fceux-core")]
// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_core_api::EngineKind;
use zax_profiles::catalogue::{ProfileCatalogue, FULL_EDITION};
use zax_profiles::profile::GameProfile;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The three games of the brief, with the profile each one is played on.
const GAMES: [(&str, &str); 3] = [
    ("Super Mario Bros. [World][!!].nes", "Super Mario Bros."),
    ("Mega Man 3 [USA][!!].nes", "Mega Man 3"),
    // The catalogue's title has no hyphen, the ROM's name has one.
    ("Punch-Out!! (USA).nes", "Punch Out!!"),
];

/// Frames every game runs, as the brief asks.
const FRAMES: u32 = 1_200;

/// The first folder of the list that holds `name`, or `None` after saying what is missing.
///
/// `ZAXNES_ROMS` first, then the two folders the lot's own notes use, so that a run on this
/// machine needs no environment at all and a run elsewhere needs only the one variable.
fn rom(name: &str) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(root) = std::env::var("ZAXNES_ROMS") {
        roots.push(PathBuf::from(root));
    }
    roots.push(PathBuf::from("G:/3DSen/test-originals"));
    roots.push(PathBuf::from("G:/3DSen/test-originals-extra"));

    for root in &roots {
        let path = root.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    eprintln!("skipped: {name} is in none of {roots:?}");
    None
}

/// The folder holding `profiles.json`, from `ZAXNES_PROFILES` or the clean room's own.
fn assets() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(root) = std::env::var("ZAXNES_PROFILES") {
        roots.push(PathBuf::from(root));
    }
    roots.push(PathBuf::from("G:/3dsen/cleanroom/assets-neutral"));

    for root in &roots {
        if root.join("profiles.json").is_file() {
            return Some(root.clone());
        }
    }
    eprintln!("skipped: profiles.json is in none of {roots:?}");
    None
}

/// The built-in profile titled `title`, as the application resolves it.
fn profile(title: &str) -> Option<GameProfile> {
    let root = assets()?;
    let files = zax_app::physical_file_system::PhysicalProfileFileSystem;
    let catalogue =
        ProfileCatalogue::load(&files, &root.join("profiles.json"), FULL_EDITION).ok()?;
    let entry = catalogue
        .entries()
        .iter()
        .find(|entry| entry.title.eq_ignore_ascii_case(title))?
        .clone();
    Some(catalogue.load_profile(&files, &entry, None).ok()?.profile)
}

/// A harness with `rom_name` in its library under its real profile, the per-game engine set to
/// `engine`, launched and run for `frames`.
fn play(
    rom_name: &str,
    title: &str,
    engine: EngineKind,
    frames: u32,
) -> Option<(LoopHarness, String)> {
    let image = std::fs::read(rom(rom_name)?).ok()?;
    let profile = profile(title)?;

    // The harness builds its host with a settings **store**, which is what keeps a per-game
    // option: without one `set_game_options` has nowhere to write and the launch would see the
    // global default. `LoopHarness::create` leaves it out, so this asks for it.
    let mut harness = LoopHarness::create_with(|options| {
        options.settings_store = Some(zax_app::persistence::settings_store::SettingsStore::new(
            &options.paths,
        ));
    });
    let path = harness.write_rom(&image, rom_name);
    harness
        .catalog()
        .add_with("game", Some(&path), Some(profile), false, Some(title));

    // The per-game option, exactly where the user's own would be, **before** the launch: the
    // choice takes effect on load.
    harness.host.set_game_options(
        &path,
        vec![(
            SettingId::EmuEngine,
            SettingValue::Enum(i32::from(engine as u8)),
        )],
    );
    assert_eq!(
        harness.host.engine_for_game("game"),
        engine,
        "the per-game option did not reach the launch"
    );

    harness.run_with(Command::LaunchGame, CommandArgs::of_text("game"));
    harness.settle();
    harness.run_frames(frames);
    Some((harness, path))
}

/// The engine chosen for a game is the engine that plays it, and switching back gives the other
/// one — the whole point of §4 of the design note.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn the_engine_chosen_for_a_game_is_the_engine_that_plays_it() {
    let _guard = lock();
    let (rom_name, title) = GAMES[0];

    {
        let Some((harness, _)) = play(rom_name, title, EngineKind::Fceux, 60) else {
            return;
        };
        assert_eq!(harness.host.running_engine(), Some(EngineKind::Fceux));
        assert_eq!(harness.host.chosen_engine(), EngineKind::Fceux);
        let session = harness.host.session().expect("a session");
        assert_eq!(session.core().kind(), EngineKind::Fceux);
        assert!(session.core().is_running());
        // And the capability flags are FCEUX's, which is what greys the four groups of rows.
        assert_eq!(
            session.core().capabilities(),
            zax_core_api::Capabilities::FCEUX
        );
    }

    // Its own scope above: one FCEUX per process, so the first harness has to be gone.
    let Some((harness, _)) = play(rom_name, title, EngineKind::Zaxnes, 60) else {
        return;
    };
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Zaxnes));
    let session = harness.host.session().expect("a session");
    assert_eq!(session.core().kind(), EngineKind::Zaxnes);
    assert_eq!(
        session.core().capabilities(),
        zax_core_api::Capabilities::ZAXNES
    );
}

/// 1 200 frames of each of the three games on FCEUX, with its own 3D profile: the records reach
/// the conversion engine, meshes come out of them, and the machine is still running at the end.
#[test]
#[ignore = "needs three commercial ROMs and the profile catalogue"]
fn the_3d_path_receives_fceuxs_records_on_three_games() {
    let _guard = lock();

    for (rom_name, title) in GAMES {
        let Some((mut harness, _)) = play(rom_name, title, EngineKind::Fceux, FRAMES) else {
            continue;
        };
        let session = harness.host.session().expect("a session");
        assert_eq!(session.core().kind(), EngineKind::Fceux);
        assert!(session.core().is_running(), "{title}: the machine stopped");
        assert!(
            session.core().frame_count() >= i64::from(FRAMES),
            "{title}: {} frames ran, {FRAMES} were asked for",
            session.core().frame_count()
        );

        // The frame the conversion engine was handed: FCEUX's own records and palette.
        let batch = session.core().frame_batch();
        assert!(
            batch.records.len() > 200,
            "{title}: only {} records in the last frame",
            batch.records.len()
        );
        assert!(
            batch.palette.iter().any(|&entry| entry != 0),
            "{title}: the palette never reached the batch"
        );
        // The widescreen fields stay standard, which is what `Capabilities::FCEUX` promises.
        assert_eq!(batch.margin, 0, "{title}");
        assert_eq!(batch.margin_y, 0, "{title}");

        let records = batch.records.len();

        // And the conversion engine made a scene out of them: the draw list is what the renderer
        // would be handed, so counting its tiles and the ones that have a mesh is the 3D path's
        // own answer rather than a guess about it.
        let session = harness.host.session_mut().expect("a session");
        let list = session.build_draw_list();
        let tiles = list.tiles().len();
        let meshes = list
            .tiles()
            .iter()
            .filter(|tile| tile.mesh.is_some())
            .count();
        println!("{title}: {records} records -> {tiles} draw tiles, {meshes} with a mesh");
        assert!(tiles > 0, "{title}: the 3D path converted nothing");
    }
}

/// Audio is produced: the samples FCEUX hands over reach the shared ring the audio thread reads.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn audio_is_produced_on_fceux() {
    let _guard = lock();
    let (rom_name, title) = GAMES[0];
    let Some((harness, _)) = play(rom_name, title, EngineKind::Fceux, 120) else {
        return;
    };

    let session = harness.host.session().expect("a session");
    let available = session.core().audio_available();
    println!("{title}: {available} samples waiting after 120 frames");
    assert!(
        available > 0,
        "{title}: no sample reached the host; a frame is about 800 at 48 kHz"
    );
    // The engine runs a rate the host can feed a device from, and says which.
    assert_eq!(session.core().sample_rate(), 48_000);
    // And the samples read back inside the scale the other engine produces.
    let mut out = vec![0.0_f32; 64];
    let read = session.core().console_samples().expect("a sample source");
    let mut read = read;
    assert!(read.read(&mut out) > 0);
    assert!(out.iter().all(|&sample| (-1.0..=1.0).contains(&sample)));
}

/// Lua's `ReadMem` and the achievements' memory both read **FCEUX's** RAM. The script writes a
/// sentinel through `WriteMem` and prints what it reads back, which is the only way to see the
/// whole path from the script host to the engine and back.
#[test]
#[ignore = "needs a commercial ROM"]
fn lua_and_the_achievements_read_fceuxs_ram() {
    use zax_achievements::Memory;
    use zax_app::achievements::CoreMemory;

    let _guard = lock();
    let Some(path) = rom(GAMES[0].0) else { return };
    let image = std::fs::read(path).unwrap();

    let mut harness = LoopHarness::create_with(|options| {
        options.settings_store = Some(zax_app::persistence::settings_store::SettingsStore::new(
            &options.paths,
        ));
    });
    let rom_path = harness.write_rom(&image, GAMES[0].0);
    let mut profile = GameProfile::create_empty();
    profile.scripts.push(zax_profiles::script::ProfileScript {
        name: Some("test".to_owned()),
        enabled: true,
        // `Update` is the frame hook and the two memory members are on `Script`, the shape
        // `run_ahead_tests.rs` uses for the same purpose.
        source: Some(
            r"
                function Update()
                    Script:WriteMem(0x0010, 0xA5)
                    print('ram ' .. Script:ReadMem(0x0010))
                end
            "
            .to_owned(),
        ),
        ..zax_profiles::script::ProfileScript::default()
    });
    harness
        .catalog()
        .add_with("game", Some(&rom_path), Some(profile), false, None);
    harness.host.set_game_options(
        &rom_path,
        vec![(SettingId::EmuEngine, SettingValue::Enum(1))],
    );
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("game"));
    harness.settle();
    harness.run_frames(10);

    let session = harness.host.session().expect("a session");
    assert_eq!(session.core().kind(), EngineKind::Fceux);

    // The script's own read, through the script host.
    let lines = harness.script_lines();
    assert!(
        lines.iter().any(|line| line.contains("ram 165")),
        "the script did not read back what it wrote through FCEUX: {lines:?}"
    );

    // And the achievements' view of the same address, which is the engine's own memory.
    let session = harness.host.session().expect("a session");
    let memory = CoreMemory(session.core());
    assert_eq!(memory.read(0x0010), Some(0xA5));
    assert_eq!(session.core().read_memory(0x0010), 0xA5);
    // The achievements' ROM hash also answers on this engine, from the image the handle kept.
    assert!(zax_app::achievements::hash_image(session.core()).is_some());
}

/// A savestate round trip and a rollback step, both on FCEUX, and a state of the **other** engine
/// refused with the message that says what to do.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn a_savestate_round_trip_and_a_rollback_step_work_on_fceux() {
    let _guard = lock();
    let (rom_name, title) = GAMES[0];
    let Some((mut harness, _)) = play(rom_name, title, EngineKind::Fceux, 180) else {
        return;
    };

    // The engine's own round trip, through the trait the application drives.
    let (frame, block) = {
        let session = harness.host.session().expect("a session");
        let block = session.core().save_state(true).expect("a state");
        assert_eq!(
            zax_core_api::engine_of_state(&block, 0),
            Some(EngineKind::Fceux),
            "the block must name its engine"
        );
        (session.core().frame_count(), block)
    };
    harness.run_frames(30);
    {
        let session = harness.host.session_mut().expect("a session");
        session.core_mut().write_memory(0x0010, 0x5A);
        session.restore_core(&block).expect("the state loads back");
        assert_eq!(session.core().frame_count(), frame);
        assert_ne!(
            session.core().read_memory(0x0010),
            0x5A,
            "the sentinel survived, so the restore did nothing"
        );
    }

    // The rollback history, which has always used the serialised state with one reused buffer a
    // slot, so it works here with no change of its own.
    harness.run_frames(240);
    let before = harness.host.rollback.count();
    assert!(
        before > 0,
        "no rollback snapshot was taken in 240 frames (one every 60)"
    );
    harness.run(Command::Rollback);
    harness.settle();
    let session = harness.host.session().expect("a session");
    assert!(
        session.core().is_running(),
        "the machine stopped on a rollback"
    );
    println!(
        "{title}: {before} snapshots, rollback left frame {}",
        session.core().frame_count()
    );

    // A block whose first four bytes name Zaxnes is refused, and as a foreign core.
    let mut foreign = zax_core_api::state_magic(EngineKind::Zaxnes)
        .to_le_bytes()
        .to_vec();
    foreign.extend_from_slice(&[0_u8; 128]);
    let session = harness.host.session_mut().expect("a session");
    let refused = session.restore_core(&foreign).expect_err("it must refuse");
    assert_eq!(refused, zax_core_api::FOREIGN_CORE_MESSAGE);
}

/// What the owner has to be able to compare: the application's core-plus-3D rate on one game with
/// each engine, headless and with no GPU campaign.
#[test]
#[ignore = "needs a commercial ROM, the profile catalogue and a minute"]
fn the_frame_rate_of_the_applications_own_path_with_each_engine() {
    use std::time::Instant;

    const WARMUP: u32 = 120;
    const TIMED: u32 = 1_200;

    let _guard = lock();
    let (rom_name, title) = ("Super Mario Bros. 3 [USA][!!].nes", "Super Mario Bros. 3");

    for engine in [EngineKind::Zaxnes, EngineKind::Fceux] {
        let Some((mut harness, _)) = play(rom_name, title, engine, WARMUP) else {
            return;
        };
        assert_eq!(harness.host.running_engine(), Some(engine));

        let started = Instant::now();
        harness.run_frames(TIMED);
        let elapsed = started.elapsed().as_secs_f64();
        #[allow(clippy::cast_precision_loss)] // 1 200 is exact in an f64.
        let rate = f64::from(TIMED) / elapsed;
        println!(
            "{title} on {engine}: {rate:.0} frames/s of the application's core + 3D path \
             ({:.4} ms a frame)",
            1_000.0 * elapsed / f64::from(TIMED)
        );
        assert!(rate > 1.0, "{engine}: nothing ran");
    }
}
