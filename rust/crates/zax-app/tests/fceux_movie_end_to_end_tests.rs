//! The application with a **real movie**, end to end (lot FCEUX-48 step 5).
//!
//! A movie of over 5 000 frames on *Super Mario Bros.* with its own 3D profile, played through
//! `LoopHarness` over the real `EmulationHost` — the real session, the real conversion engine, the
//! real settings and slots, no window. The same headless path lot FCEUX-23 step 6 found.
//!
//! # How the movie is made, and why that is a recording
//!
//! The brief asks for a movie "built by recording a scripted input sequence on FCEUX — no
//! download", and that is literally what happens here: the scripted sequence is **first fed to
//! FCEUX frame by frame** through the application's own loop, every frame hashed; the `.fm2` is
//! then written from that very sequence and played back; and the two runs are compared frame for
//! frame. So the movie is not merely built to the format — it is checked to replay the run it was
//! recorded from, which is the only property a recording has that a hand-written file does not.
//!
//! FCEUX's own recorder was not used, and could not be: it writes to a *path*
//! (`openRecordingMovie`), and the fork's core-only build has no filesystem.
//!
//! Every test is `#[ignore]`d — each needs the commercial ROM (`ZAXNES_ROMS`) and the profile
//! catalogue (`ZAXNES_PROFILES`) — and the whole file needs `fceux-core`.

#![cfg(feature = "fceux-core")]
// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::hash::{Hash as _, Hasher as _};
use std::path::PathBuf;
use std::time::Instant;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_core_api::EngineKind;
use zax_profiles::catalogue::{ProfileCatalogue, FULL_EDITION};
use zax_profiles::profile::GameProfile;
use zax_settings::catalogue::SettingId;
use zax_settings::values::SettingValue;

/// The game, its profile, and the movie's length — over the 5 000 the brief asks for.
const ROM: &str = "Super Mario Bros. [World][!!].nes";
const TITLE: &str = "Super Mario Bros.";
const FRAMES: usize = 5_400;

/// One machine per process, so the tests of this file hold it in turn.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn rom(name: &str) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(root) = std::env::var("ZAXNES_ROMS") {
        roots.push(PathBuf::from(root));
    }
    roots.push(PathBuf::from("G:/3DSen/test-originals"));
    for root in &roots {
        let path = root.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    eprintln!("skipped: {name} is in none of {roots:?}");
    None
}

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

/// The game launched on `engine`, with its own profile and a settings store so that the per-game
/// option is the thing in force.
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

/// The scripted sequence: Start at the title screen, then run right with a jump every so often —
/// a *Super Mario Bros.* run rather than an idle machine, so that the records are a moving scene.
fn scripted(frames: usize) -> Vec<u8> {
    (0..frames)
        .map(|frame| match frame {
            0..=39 => 0x00,
            40..=43 => 0x08,              // Start
            _ if frame % 96 < 14 => 0x81, // Right + A
            _ => 0x80,                    // Right
        })
        .collect()
}

/// One pad as a `.fm2` frame field: `RLDUTSBA`, `.` for a button that is up.
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

/// The `.fm2` of `pads`: a version-3 text header with two standard pads, the zero RAM fill and no
/// ROM checksum stated, then one line a frame.
fn fm2_of(pads: &[u8]) -> Vec<u8> {
    let mut lines = vec![
        "version 3".to_owned(),
        "emuVersion 20000".to_owned(),
        "rerecordCount 0".to_owned(),
        "palFlag 0".to_owned(),
        format!("romFilename {TITLE}"),
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
        "comment author the zax-app end-to-end recording of lot FCEUX-48 step 5".to_owned(),
    ];
    lines.extend(
        pads.iter()
            .map(|pad| format!("|0|{}|........||", mnemonics(*pad))),
    );
    let mut text = lines.join("\n");
    text.push('\n');
    text.into_bytes()
}

/// A digest of the machine the frame on screen is of: the work RAM and the picture.
///
/// Not `movie_timeline::machine_digest`, which also hashes `cycle_count` — session-relative on an
/// engine without a counter of its own, so higher after a seek that replayed frames twice (lot
/// FCEUX-48 step 4 records why).
fn frame_digest(harness: &LoopHarness) -> u64 {
    let session = harness.host.session().expect("a session");
    let core = session.core();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for address in 0..0x0800_u16 {
        core.read_memory(address).hash(&mut hasher);
    }
    let mut picture = Vec::new();
    core.pixel_frame_indices_into(&mut picture);
    picture.hash(&mut hasher);
    hasher.finish()
}

/// Whether the picture of the frame on screen is a single colour — a frame with nothing on it,
/// which is what a record count of zero must mean.
fn picture_is_uniform(harness: &LoopHarness) -> bool {
    let session = harness.host.session().expect("a session");
    let mut picture = Vec::new();
    session.core().pixel_frame_indices_into(&mut picture);
    picture
        .first()
        .is_some_and(|first| picture.iter().all(|index| index == first))
}

/// Records in the frame just run, which is what the 3D path is handed.
fn records(harness: &LoopHarness) -> usize {
    harness
        .host
        .session()
        .expect("a session")
        .core()
        .frame_batch()
        .records
        .len()
}

/// The draw list the renderer would be given: tiles, and tiles with a mesh.
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

/// Writes the movie beside the harness's own files.
fn write_movie(harness: &LoopHarness, movie: &[u8]) -> String {
    let path = harness.root.path().join("smb-5400.fm2");
    std::fs::write(&path, movie).expect("the movie can be written");
    path.to_string_lossy().into_owned()
}

/// **The recording.** The scripted sequence is fed to FCEUX through the application's own loop,
/// every frame hashed; then the `.fm2` written from that sequence is played back and the two are
/// compared frame for frame. That is what makes the file a recording of this run rather than a
/// guess at the format.
#[test]
#[ignore = "needs the commercial ROM and the profile catalogue"]
fn the_recorded_sequence_and_the_movie_of_it_play_the_same_5400_frames() {
    let _guard = lock();
    let pads = scripted(FRAMES);

    // The recording: FCEUX driven by the sequence, one frame at a time.
    //
    // Two things make it comparable with the playback below, and both are needed. The machine is
    // **power-cycled first**, because a launch leaves the frame counter wherever the pacer left
    // it and a movie's load puts it at 0 — a couple of frames of attract mode is enough to part
    // the two runs. And the frames are run through `run_session_frame` rather than through the
    // pacer, so that one call is exactly one frame with exactly that input.
    let (recorded, blank_frames) = {
        let Some(mut harness) = launched(EngineKind::Fceux) else {
            return;
        };
        assert_eq!(harness.host.running_engine(), Some(EngineKind::Fceux));
        harness.run(Command::Reset);
        assert_eq!(
            harness
                .host
                .session()
                .expect("a session")
                .core()
                .frame_count(),
            0,
            "the recording must start at power-on, as the movie's own load does"
        );
        let mut digests = Vec::with_capacity(FRAMES);
        let mut blank = 0_usize;
        for pad in &pads {
            harness.run_exact_frames(
                1,
                &[zax_app::input::InputAction::from_bits(u32::from(*pad))],
            );
            digests.push(frame_digest(&harness));
            if records(&harness) == 0 {
                blank += 1;
            }
        }
        (digests, blank)
    };
    println!("recorded {FRAMES} frames on FCEUX, {blank_frames} of them with no record");

    // The playback of the movie written from it.
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let path = write_movie(&harness, &fm2_of(&pads));
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();
    assert_eq!(
        harness.host.running_engine(),
        Some(EngineKind::Fceux),
        "the movie must have forced the engine that replays it"
    );
    assert_eq!(
        harness.host.movie_position(),
        Some((0, u32::try_from(FRAMES).unwrap_or(0)))
    );

    let mut played = Vec::with_capacity(FRAMES);
    let mut without_records = Vec::new();
    for frame in 0..FRAMES {
        // The same one-call-one-frame path as the recording, and the input handed over is
        // deliberately *not* the movie's: FCEUX's movie mode loads each record over the pad state
        // itself, so a playback that needed our word would fail here rather than pass by luck.
        harness.run_exact_frames(1, &[zax_app::input::InputAction::NONE]);
        played.push(frame_digest(&harness));
        if records(&harness) == 0 {
            without_records.push((frame, picture_is_uniform(&harness)));
        }
    }

    // Every frame identical: the movie replays the run it was recorded from.
    let parted = played
        .iter()
        .zip(&recorded)
        .position(|(played, recorded)| played != recorded);
    assert_eq!(
        parted, None,
        "the movie and the sequence it was recorded from part at frame {parted:?}"
    );
    assert_eq!(played.len(), FRAMES);

    // **The 3D path is handed records on every frame that has anything to draw.** A frame with no
    // record is not a gap: *Super Mario Bros.* blanks the screen at a death and between levels,
    // the PPU's rendering being off, and this run dies — so the check is that a record-less frame
    // is exactly a frame whose picture is **one colour**, which is a frame with nothing on it.
    let blind: Vec<usize> = without_records
        .iter()
        .filter(|(_, uniform)| !*uniform)
        .map(|(frame, _)| *frame)
        .collect();
    println!(
        "{} frames of {FRAMES} had no record, every one of them a blank screen;          {} with something on screen and no record",
        without_records.len(),
        blind.len()
    );
    assert!(
        blind.is_empty(),
        "frames with a picture and no record: {blind:?}"
    );
    let (tiles, meshed) = draw_tiles(&mut harness);
    println!("the draw list at frame {FRAMES}: {tiles} tiles, {meshed} with a mesh");
    assert!(tiles > 0 && tiles == meshed);

    // And the movie is spent, on its last frame.
    assert_eq!(
        harness.host.movie_position(),
        Some((
            u32::try_from(FRAMES).unwrap_or(0),
            u32::try_from(FRAMES).unwrap_or(0)
        ))
    );
}

/// Pausing, stepping, seeking back to a checkpoint and resuming give **the same frame digests as
/// an uninterrupted playback**, which is the brief's own check and the property the profile
/// editor rests on.
#[test]
#[ignore = "needs the commercial ROM and the profile catalogue"]
fn interrupting_the_playback_changes_nothing_about_the_frames_it_shows() {
    const MARKS: [u32; 4] = [1_200, 2_400, 3_600, 5_000];

    let _guard = lock();
    let pads = scripted(FRAMES);
    let movie = fm2_of(&pads);

    // The uninterrupted playback, hashed at four marks.
    let straight = {
        let Some(mut harness) = launched(EngineKind::Zaxnes) else {
            return;
        };
        let path = write_movie(&harness, &movie);
        harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
        harness.settle();
        let mut at = Vec::new();
        for mark in MARKS {
            while harness.host.movie_position().expect("a position").0 < mark {
                harness.run_frames(1);
            }
            at.push(frame_digest(&harness));
        }
        at
    };

    // The interrupted one: the same marks, with a pause, two frame steps, a seek back over a
    // checkpoint and a resume between each pair.
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let path = write_movie(&harness, &movie);
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();

    let mut at = Vec::new();
    for (index, mark) in MARKS.into_iter().enumerate() {
        while harness.host.movie_position().expect("a position").0 < mark {
            harness.run_frames(1);
        }
        at.push(frame_digest(&harness));

        if index + 1 == MARKS.len() {
            break;
        }
        // Pause, two single frames forward, then back behind the mark and on to it again.
        harness.run(Command::Pause);
        harness.run_frames(30);
        harness.run(Command::FrameAdvance);
        harness.settle();
        harness.run(Command::FrameAdvance);
        harness.settle();
        let back = mark - 300;
        harness.run_with(
            Command::SeekMovie,
            CommandArgs::of_number(i32::try_from(back).unwrap_or(0)),
        );
        for _ in 0..4_000 {
            if harness.host.movie_seek_target().is_none() {
                break;
            }
            harness.tick_frame();
        }
        assert_eq!(harness.host.movie_seek_target(), None, "a seek never ended");
        assert_eq!(
            harness.host.movie_position().expect("a position").0,
            back,
            "the seek did not land where it was asked to"
        );
        harness.run(Command::Resume);
        while harness.host.movie_position().expect("a position").0 < mark {
            harness.run_frames(1);
        }
        assert_eq!(
            frame_digest(&harness),
            at[index],
            "coming back to frame {mark} after a seek showed another machine"
        );
    }

    assert_eq!(
        at, straight,
        "an interrupted playback must show the same frames as an uninterrupted one"
    );
    println!(
        "interrupted and uninterrupted playbacks agree at frames {MARKS:?} \
         (pause, two frame steps, a 300-frame seek back and a resume between each)"
    );
}

/// Reloading the profile mid-movie takes effect **without disturbing the movie**: the engine, the
/// movie and the frame on screen are the ones they were, and the scene is built from the profile
/// read again.
#[test]
#[ignore = "needs the commercial ROM and the profile catalogue"]
fn reloading_the_profile_mid_movie_takes_effect_without_disturbing_the_movie() {
    const AT: u32 = 2_000;

    let _guard = lock();
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let path = write_movie(&harness, &fm2_of(&scripted(FRAMES)));
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();
    while harness.host.movie_position().expect("a position").0 < AT {
        harness.run_frames(1);
    }
    let before = frame_digest(&harness);
    let (tiles_before, _) = draw_tiles(&mut harness);

    harness.run(Command::ReloadScript);
    harness.settle();
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Fceux));
    assert!(harness.host.own_movie_playing(), "the movie must survive");
    // The reload of a movie the engine plays is exact: the movie is loaded again, its clock put
    // at 0, and the timeline replays to the frame the author was on with the profile read again.
    assert_eq!(harness.host.movie_seek_target(), Some(AT));
    for _ in 0..20_000 {
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

    assert_eq!(
        harness.host.movie_position().expect("a position").0,
        AT,
        "the reload must come back to the frame on screen"
    );
    assert_eq!(
        frame_digest(&harness),
        before,
        "the machine at frame {AT} must be the one it was"
    );
    let (tiles_after, meshed_after) = draw_tiles(&mut harness);
    assert!(tiles_after > 0 && tiles_after == meshed_after);
    println!(
        "reloaded the profile at movie frame {AT}: {tiles_before} draw tiles before, \
         {tiles_after} after, the machine identical"
    );

    // And the movie goes on from there.
    harness.run_frames(120);
    assert!(harness.host.movie_position().expect("a position").0 > AT);
}

/// The frames per second of movie playback with the 3D on FCEUX, which is the figure the brief
/// asks to be recorded. Measured after a warm-up, in the application's own loop, so it carries
/// the core, the capture, the conversion and the draw-list build — everything but the renderer.
#[test]
#[ignore = "needs the commercial ROM and the profile catalogue"]
fn the_frame_rate_of_movie_playback_with_3d_on_fceux() {
    const WARMUP: u32 = 240;
    const TIMED: u32 = 3_000;

    let _guard = lock();
    let Some(mut harness) = launched(EngineKind::Zaxnes) else {
        return;
    };
    let path = write_movie(&harness, &fm2_of(&scripted(FRAMES)));
    harness.run_with(Command::OpenMovie, CommandArgs::of_text(&path));
    harness.settle();
    harness.run_frames(WARMUP);
    assert_eq!(harness.host.running_engine(), Some(EngineKind::Fceux));

    let started = Instant::now();
    harness.run_frames(TIMED);
    let elapsed = started.elapsed().as_secs_f64();
    #[allow(clippy::cast_precision_loss)] // 3 000 is exact in an f64.
    let rate = f64::from(TIMED) / elapsed;
    println!(
        "movie playback with 3D on FCEUX: {rate:.0} frames/s ({:.4} ms a frame) over {TIMED} \
         frames after {WARMUP} of warm-up",
        1_000.0 * elapsed / f64::from(TIMED)
    );
    assert!(rate > 1.0, "nothing ran");
    assert!(
        harness.host.movie_position().expect("a position").0 >= WARMUP + TIMED,
        "the movie did not play the frames that were timed"
    );
}
