//! `.fm2` playback in the **application**, on the engine that recorded it (lot FCEUX-48 step 4).
//!
//! The headless path is `LoopHarness` over the real `EmulationHost` — the real session, the real
//! conversion engine, the real settings and slots, no window — which is the path lot FCEUX-23
//! step 6 found and used. The 3D is exercised as far as the draw list, which is where the records
//! actually arrive.
//!
//! Every test is `#[ignore]`d: each needs a commercial ROM (`ZAXNES_ROMS`) and the profile
//! catalogue (`ZAXNES_PROFILES`), and the whole file needs `fceux-core`. They serialise
//! themselves, FCEUX keeping one machine per process.

#![cfg(feature = "fceux-core")]
// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_core_api::EngineKind;
use zax_nes::movie::MovieMode;
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

/// The game every test here uses, with the profile it is played on.
const ROM: &str = "Super Mario Bros. [World][!!].nes";
const TITLE: &str = "Super Mario Bros.";

/// The first folder of the list that holds `name`, or `None` after saying what is missing.
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

/// A harness with the game in its library under its real profile and the per-game engine set to
/// `engine`, launched and settled.
fn launched(engine: EngineKind) -> Option<LoopHarness> {
    let image = std::fs::read(rom(ROM)?).ok()?;
    let profile = profile(TITLE)?;
    let mut harness = LoopHarness::create_with(|options| {
        options.settings_store = Some(zax_app::persistence::settings_store::SettingsStore::new(
            &options.paths,
        ));
    });
    let path = harness.write_rom(&image, ROM);
    harness
        .catalog()
        .add_with("game", Some(&path), Some(profile), false, Some(TITLE));
    harness.host.set_game_options(
        &path,
        vec![(
            SettingId::EmuEngine,
            SettingValue::Enum(i32::from(engine as u8)),
        )],
    );
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("game"));
    harness.settle();
    Some(harness)
}

/// Writes `movie` into the harness's own folder and gives the path, so that nothing is left on
/// the machine.
fn write_movie(harness: &LoopHarness, name: &str, movie: &[u8]) -> String {
    let path = harness.root.path().join(name);
    std::fs::write(&path, movie).expect("the movie can be written");
    path.to_string_lossy().into_owned()
}

// ------------------------------------------------------------------------------------------------
// The `.fm2` builder, to the public format (TASVideos "FM2") and `src/movie.cpp`'s own parser
// ------------------------------------------------------------------------------------------------

/// One pad as a frame field: `RLDUTSBA`, `.` for a button that is up.
fn mnemonics(pad: u8) -> String {
    const BY_BIT: [char; 8] = ['A', 'B', 'S', 'T', 'U', 'D', 'L', 'R'];
    (0..8)
        .rev()
        .map(|bit| {
            if pad & (1 << bit) == 0 {
                '.'
            } else {
                BY_BIT[bit]
            }
        })
        .collect()
}

/// A version-3 `.fm2` of `frames` frames with two standard pads, the zero RAM fill and no ROM
/// checksum stated — a movie that claims nothing about the ROM is played rather than refused, so
/// the game's own MD5 never has to be computed here.
///
/// `extra` goes into the header as it stands, which is how the refusal tests ask for a Four Score
/// or FCEUX's new PPU.
fn fm2(pads: &[(u8, u8)], extra: &[&str]) -> Vec<u8> {
    let mut lines = vec![
        "version 3".to_owned(),
        "emuVersion 20000".to_owned(),
        "rerecordCount 11".to_owned(),
        "palFlag 0".to_owned(),
        "romFilename smb".to_owned(),
        "guid 00000000-0000-0000-0000-000000000000".to_owned(),
        "fourscore 0".to_owned(),
        "microphone 0".to_owned(),
        "port0 1".to_owned(),
        "port1 1".to_owned(),
        "port2 0".to_owned(),
        "FDS 0".to_owned(),
        "NewPPU 0".to_owned(),
        "RAMInitOption 2".to_owned(),
        "RAMInitSeed 0".to_owned(),
    ];
    lines.extend(extra.iter().map(|line| (*line).to_owned()));
    lines.extend(
        pads.iter()
            .map(|(one, two)| format!("|0|{}|{}||", mnemonics(*one), mnemonics(*two))),
    );
    let mut text = lines.join("\n");
    text.push('\n');
    text.into_bytes()
}

/// A scripted sequence: Start at the title screen, then Right with a jump every so often, which
/// is a *Super Mario Bros.* run rather than an idle machine.
fn script(frames: usize) -> Vec<(u8, u8)> {
    (0..frames)
        .map(|frame| {
            let pad = match frame {
                0..=39 => 0x00,
                40..=43 => 0x08,                     // Start
                _ if frame % 64 < 10 => 0x80 | 0x01, // Right + A
                _ => 0x80,                           // Right
            };
            (pad, 0)
        })
        .collect()
}

/// The 3D draw list the renderer would be handed, as a count of tiles and of tiles with a mesh.
fn draw_tiles(harness: &mut LoopHarness) -> (usize, usize) {
    let session = harness.host.session_mut().expect("a session");
    let list = session.build_draw_list();
    let tiles = list.tiles().len();
    let meshed = list
        .tiles()
        .iter()
        .filter(|tile| tile.mesh.is_some())
        .count();
    (tiles, meshed)
}

/// §5 and the owner's reason for the whole engine: **opening an `.fm2` plays it on FCEUX**, for
/// that playback alone, and the game's own engine setting is left exactly as the user set it.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn opening_an_fm2_plays_it_on_fceux_without_changing_the_games_own_engine() {
    let _guard = lock();
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    // The game starts on Zaxnes, which is what the user chose.
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Zaxnes));
    assert_eq!(harness.host.engine_for_game("game"), EngineKind::Zaxnes);
    harness.run_frames(30);

    let movie = fm2(&script(600), &[]);
    let path = write_movie(&harness, "smb.fm2", &movie);
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();

    // The playback is on FCEUX…
    assert_eq!(
        harness.host.running_engine(),
        Some(EngineKind::Fceux),
        "an .fm2 must play on the engine it is in sync on"
    );
    assert!(harness.host.own_movie_playing());
    assert_eq!(harness.host.movie_mode(), Some(MovieMode::Playing));
    assert_eq!(harness.host.movie_position(), Some((0, 600)));
    // …and the user's own choice is untouched, both in the store and as the next load would read
    // it. That is the difference between "for this playback" and "changed the setting".
    assert_eq!(
        harness.host.engine_for_game("game"),
        EngineKind::Zaxnes,
        "the per-game setting must not have been written"
    );

    // 300 frames of it: the clock follows, and the 3D path is handed tiles with meshes.
    harness.run_frames(300);
    let (frame, length) = harness.host.movie_position().expect("a position");
    assert_eq!(length, 600);
    assert!(frame >= 290, "the movie's clock did not follow: {frame}");
    let (tiles, meshed) = draw_tiles(&mut harness);
    assert!(tiles > 0, "the 3D path received no tile");
    assert_eq!(tiles, meshed, "a tile reached the draw list without a mesh");

    // Notes are a Zaxnes movie's: `Ctrl+M` is not offered, and the command says so rather than
    // doing nothing.
    assert!(!harness.host.movie_takes_notes());
    assert!(!harness
        .host
        .can_execute(Command::AnnotateMovie, &CommandArgs::default()));
}

/// Pause, frame step and resume while the movie plays — the editor's own loop. The movie's clock
/// moves with the frames and stops with them, which is what a paused movie means.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn the_movie_pauses_steps_and_resumes() {
    let _guard = lock();
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let movie = fm2(&script(600), &[]);
    let path = write_movie(&harness, "smb.fm2", &movie);
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();
    harness.run_frames(120);

    let before = harness.host.movie_position().expect("a position").0;
    harness.run(Command::Pause);
    harness.run_frames(60);
    assert_eq!(
        harness.host.movie_position().expect("a position").0,
        before,
        "a paused movie must not advance"
    );

    // One frame at a time, which is `K`.
    harness.run(Command::FrameAdvance);
    harness.settle();
    let stepped = harness.host.movie_position().expect("a position").0;
    assert_eq!(stepped, before + 1, "a frame step must advance exactly one");
    assert!(harness.host.own_movie_playing());

    harness.run(Command::Resume);
    harness.run_frames(60);
    assert!(
        harness.host.movie_position().expect("a position").0 > stepped,
        "the movie must go on after a resume"
    );
}

/// Seeking **backwards** goes through the engine's own savestates taken during playback, and
/// what it replays is what an uninterrupted playback showed: the same work-RAM and frame digest
/// at the same movie frame. Forwards replays, which is the same machinery.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn a_backward_seek_replays_the_same_frames() {
    const TARGET: u32 = 90;
    const ON: u32 = 300;

    let _guard = lock();
    let movie = fm2(&script(600), &[]);

    // The straight run, for the digest to match.
    let straight = {
        let Some(mut harness) = launched(EngineKind::Zaxnes) else {
            return;
        };
        let path = write_movie(&harness, "smb.fm2", &movie);
        harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
        harness.settle();
        while harness.host.movie_position().expect("a position").0 < ON {
            harness.run_frames(1);
        }
        digest(&harness)
    };

    // The interrupted one: on to ON, back to TARGET through a checkpoint, and forward again.
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let path = write_movie(&harness, "smb.fm2", &movie);
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();
    while harness.host.movie_position().expect("a position").0 < ON {
        harness.run_frames(1);
    }

    harness.run_with(
        Command::SeekMovie,
        CommandArgs::of_number(i32::try_from(TARGET).unwrap_or(0)),
    );
    // The seek runs over the next ticks, within its own time budget.
    for _ in 0..600 {
        if harness.host.movie_seek_target().is_none() {
            break;
        }
        harness.tick_frame();
    }
    assert_eq!(
        harness.host.movie_seek_target(),
        None,
        "the seek never ended"
    );
    let after = harness.host.movie_position().expect("a position").0;
    assert_eq!(
        after, TARGET,
        "the seek did not land on the frame asked for"
    );
    assert!(
        harness.host.own_movie_playing(),
        "the movie must still be loaded"
    );

    while harness.host.movie_position().expect("a position").0 < ON {
        harness.run_frames(1);
    }
    assert_eq!(
        digest(&harness),
        straight,
        "a seek back and forward must replay the same machine"
    );
}

/// Reloading the profile while the movie plays — the editor mode's own F5 — keeps the movie and
/// the engine, which is the whole point of the lot: edit the 3dn and see it on the same frame.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn reloading_the_profile_mid_movie_keeps_the_movie_and_the_engine() {
    let _guard = lock();
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let movie = fm2(&script(600), &[]);
    let path = write_movie(&harness, "smb.fm2", &movie);
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();
    harness.run_frames(200);
    let before = harness.host.movie_position().expect("a position").0;

    harness.run(Command::ReloadScript);
    harness.settle();
    assert_eq!(
        harness.host.running_engine(),
        Some(EngineKind::Fceux),
        "a reload must not change engines under the author"
    );
    assert!(
        harness.host.own_movie_playing(),
        "the movie must survive a profile reload"
    );
    // The reload of a movie the engine plays is always **exact**: the movie is loaded again, which
    // puts its clock at 0, and the timeline then replays to the frame the author was on with the
    // new scripts. That takes ticks, within the seek's own budget, which is what is drained here.
    assert_eq!(
        harness.host.movie_seek_target(),
        Some(before),
        "the reload must seek back to the frame on screen"
    );
    for _ in 0..1_200 {
        if harness.host.movie_seek_target().is_none() {
            break;
        }
        harness.tick_frame();
    }
    assert_eq!(
        harness.host.movie_seek_target(),
        None,
        "the seek never ended"
    );
    let after = harness.host.movie_position().expect("a position").0;
    assert_eq!(
        after, before,
        "a reload must leave the movie on the frame it was on"
    );
    harness.run_frames(60);
    assert!(harness.host.movie_position().expect("a position").0 > after);
    let (tiles, meshed) = draw_tiles(&mut harness);
    assert!(tiles > 0 && tiles == meshed);
}

/// A movie whose header asks for something this build has not is **refused with the reason**, and
/// the game is left running rather than half-switched.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn a_movie_the_build_cannot_play_is_refused_with_its_reason() {
    let _guard = lock();
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    harness.run_frames(30);

    for (extra, needle) in [
        (vec!["fourscore 1"], "Four Score"),
        (vec!["NewPPU 1"], "new PPU"),
        (vec!["FDS 1"], "Disk System"),
        (vec!["port1 2"], "standard pad"),
    ] {
        // The header's own keys come after the defaults, so these override them.
        let movie = fm2(&script(60), &extra);
        let path = write_movie(&harness, "bad.fm2", &movie);
        harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
        harness.settle();

        assert!(
            !harness.host.own_movie_playing(),
            "{extra:?} must not be played"
        );
        assert_eq!(harness.host.movie_mode(), None);
        let said = harness
            .host
            .notifications
            .log()
            .iter()
            .any(|line| line.text.contains(needle));
        assert!(said, "the refusal of {extra:?} did not name {needle}");
        // And the game is still there to be played.
        assert!(harness
            .host
            .session()
            .expect("a session")
            .core()
            .is_running());
        harness.run_frames(10);
    }
}

/// Stopping: the movie is unloaded, the game keeps running from the frame it is on, and the
/// engine stays the one it is until the next load — which `own_movies.rs` explains and the
/// message says. The next launch puts the game back on its own engine.
#[test]
#[ignore = "needs a commercial ROM and the profile catalogue"]
fn stopping_the_movie_leaves_the_game_running_and_the_next_load_restores_the_engine() {
    let _guard = lock();
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let movie = fm2(&script(600), &[]);
    let path = write_movie(&harness, "smb.fm2", &movie);
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();
    harness.run_frames(120);
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Fceux));

    harness.run(Command::StopMovie);
    harness.settle();
    assert!(!harness.host.own_movie_playing());
    assert_eq!(harness.host.movie_mode(), None);
    assert_eq!(harness.host.movie_position(), None);
    // The game goes on, on the engine the movie forced, until it is loaded again.
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Fceux));
    let frame = harness
        .host
        .session()
        .expect("a session")
        .core()
        .frame_count();
    harness.run_frames(30);
    assert!(
        harness
            .host
            .session()
            .expect("a session")
            .core()
            .frame_count()
            > frame
    );

    // And the user's own choice comes back at the next load, having never been written over.
    assert_eq!(harness.host.engine_for_game("game"), EngineKind::Zaxnes);
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("game"));
    harness.settle();
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Zaxnes));
}

/// A digest of the machine the frame on screen is of: the movie's own frame, the work RAM and the
/// picture.
///
/// Deliberately **not** `movie_timeline::machine_digest`, which also hashes
/// `EngineHandle::cycle_count` — and on an engine without a cycle counter of its own that figure
/// is the sum of the frames *this session has run*, not the cycles since power-on, so it is
/// higher after a seek that replayed 200 frames twice. That is honest for what it is used for
/// (the audio ratio and the frame profiler) and useless for comparing two paths to the same
/// frame, which is what this does.
fn digest(harness: &LoopHarness) -> u64 {
    use std::hash::{Hash as _, Hasher as _};

    let session = harness.host.session().expect("a session");
    let core = session.core();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    core.own_movie_position().hash(&mut hasher);
    for address in 0..0x0800_u16 {
        core.read_memory(address).hash(&mut hasher);
    }
    let mut picture = Vec::new();
    core.pixel_frame_indices_into(&mut picture);
    picture.hash(&mut hasher);
    hasher.finish()
}
