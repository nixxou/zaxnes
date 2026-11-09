//! The `zax-fceux` wrapper against the library the fork builds.
//!
//! Every test here needs the `fceux-core` feature *and* a built fork, so the whole file is
//! `#[cfg(feature = "fceux-core")]`: with the feature off there is nothing to test and nothing is
//! compiled.
//!
//! **These tests serialise themselves.** FCEUX keeps its machine in file-scope globals, so there
//! is one emulator per process and two tests must never hold a handle at once; `lock()` below is
//! what makes that true whatever `--test-threads` says.

#![cfg(feature = "fceux-core")]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use zax_fceux::{Error, Fceux, RamInit, Region, FRAME_LEN};

/// Held for the length of every test, so only one holds a [`Fceux`] at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    // A poisoned lock means another test panicked while holding it; the core is then in an
    // unknown state but still has to be usable, so the guard is taken anyway.
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// `nestest.nes` of `rust/test-roms`, or `None` after saying what is missing. The ROMs are not in
/// the repository (`rust/.gitignore`), so a checkout without them skips rather than fails.
fn nestest() -> Option<PathBuf> {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms/nestest.nes");
    if rom.is_file() {
        return Some(rom);
    }

    eprintln!("skipped: rust/test-roms/nestest.nes is missing");
    None
}

/// FNV-1a, the same hash `zax/zaxselftest.c` uses, so the two can be compared digit for digit.
fn digest(data: &[u8], seed: u64) -> u64 {
    data.iter().fold(seed, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
    })
}

const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;

#[test]
fn the_version_names_the_fceux_core_it_forks() {
    let version = Fceux::version();
    assert!(
        version.contains("zaxfceux") && version.contains("FCEUX"),
        "unexpected version string: {version}"
    );
}

#[test]
fn there_is_one_instance_per_process_and_the_second_is_refused() {
    let _serial = lock();

    let first = Fceux::new().expect("the first instance");
    assert_eq!(
        Fceux::new().err(),
        Some(Error::AlreadyLive),
        "a second instance must be refused: FCEUX keeps its machine in file-scope globals"
    );

    // And once the first is dropped, a new one is allowed again.
    drop(first);
    let _second = Fceux::new().expect("an instance after the first was dropped");
}

#[test]
fn a_frame_before_any_rom_is_refused_rather_than_guessed_at() {
    let _serial = lock();

    let mut fceux = Fceux::new().expect("an instance");
    assert_eq!(fceux.run_frame().err(), Some(Error::NoRom));
    assert_eq!(fceux.reset().err(), Some(Error::NoRom));
    assert_eq!(fceux.read_memory(0).err(), Some(Error::NoRom));
    assert!(!fceux.has_rom());
}

#[test]
fn a_bad_image_is_refused_with_the_cores_own_explanation() {
    let _serial = lock();

    let mut fceux = Fceux::new().expect("an instance");
    let error = fceux
        .load_rom(b"this is not a ROM at all, not even close", "junk.nes")
        .expect_err("FCEUX must refuse this");

    match error {
        Error::LoadFailed(log) => assert!(
            log.iter().any(|line| !line.trim().is_empty()),
            "the refusal should carry what the core printed, got {log:?}"
        ),
        other => panic!("expected LoadFailed, got {other:?}"),
    }
    assert!(!fceux.has_rom());
}

#[test]
fn an_empty_image_and_a_name_with_a_nul_are_refused_before_the_ffi_call() {
    let _serial = lock();

    let mut fceux = Fceux::new().expect("an instance");
    assert!(matches!(
        fceux.load_rom(&[], "empty.nes"),
        Err(Error::BadArgument(_))
    ));
    assert!(matches!(
        fceux.load_rom(b"\x4eESX", "na\0me.nes"),
        Err(Error::BadArgument(_))
    ));
}

/// The cross-check that matters: the wrapper must see **exactly** what `zax/zaxselftest.c` saw
/// running the same ROM for the same number of frames. The three digests are that program's
/// output, which was identical on Windows and Linux (lot FCEUX-1 step 3).
#[test]
fn sixty_frames_of_nestest_match_the_c_self_tests_digests() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).expect("nestest.nes");

    let mut fceux = Fceux::new().expect("an instance");
    fceux.set_sample_rate(48_000).expect("48 kHz");
    fceux
        .set_ram_init(RamInit::Default)
        .expect("the default RAM fill");
    // The basename, as the C self-test passes: FCEUX reports the name and uses it with the MD5
    // against its own database, so a digest must not depend on where the file sits.
    fceux
        .load_rom(&image, "nestest.nes")
        .expect("nestest must load");
    assert_eq!(fceux.region().expect("a region"), Region::Ntsc);
    fceux.reset().expect("a reset");

    let mut pixels = Vec::new();
    let mut emphasis = Vec::new();
    let mut audio = Vec::new();
    let mut pixel_hash = FNV_OFFSET;
    let mut emphasis_hash = FNV_OFFSET;
    let mut audio_hash = FNV_OFFSET;
    let mut samples = 0usize;

    for _ in 0..60 {
        fceux.set_input(0, 0).expect("no button pressed");
        fceux.run_frame().expect("a frame");

        fceux.frame_indices_into(&mut pixels).expect("the picture");
        fceux
            .frame_emphasis_into(&mut emphasis)
            .expect("the emphasis");
        assert_eq!(pixels.len(), FRAME_LEN);
        assert_eq!(emphasis.len(), FRAME_LEN);
        pixel_hash = digest(&pixels, pixel_hash);
        emphasis_hash = digest(&emphasis, emphasis_hash);

        audio.clear();
        let got = fceux.read_audio_into(&mut audio).expect("the audio");
        assert_eq!(got, audio.len());
        samples += got;
        let bytes: Vec<u8> = audio.iter().flat_map(|s| s.to_le_bytes()).collect();
        audio_hash = digest(&bytes, audio_hash);
    }

    assert_eq!(fceux.frame_count(), 60);
    assert_eq!(
        (pixel_hash, emphasis_hash, audio_hash, samples),
        (
            // The pixel digest moved once step 5 stopped FCEUX drawing its own "Power on" and
            // "Reset" captions into the frame (`vidGuiMsgEna`); the emphasis and audio digests
            // did not, which is exactly what an OSD-only change predicts.
            0xe455_081e_6a3e_efe9,
            0xd395_e456_e4e7_2325,
            0x7fc1_0a9c_e4ed_8fcd,
            47_917
        ),
        "the wrapper must see what zax/zaxselftest.c saw on the same 60 frames"
    );

    // 47917 samples over 60 frames at 48 kHz is 798.6 a frame: NTSC's 48000/60.0988, not a
    // nominal 60. A rate that drifted would show up here before it showed up in a comparison.
    assert_eq!(fceux.sample_rate(), 48_000);
}

#[test]
fn a_savestate_round_trip_brings_the_picture_back() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).expect("nestest.nes");

    let mut fceux = Fceux::new().expect("an instance");
    fceux
        .load_rom(&image, "nestest.nes")
        .expect("nestest must load");
    for _ in 0..30 {
        fceux.run_frame().expect("a frame");
    }

    let mut state = Vec::new();
    fceux.save_state_into(&mut state).expect("a savestate");
    assert_ne!(state.len(), 0, "a savestate must not be empty");
    assert_eq!(&state[..4], b"FCSX", "FCEUX's own savestate magic");

    let mut before = Vec::new();
    fceux.frame_indices_into(&mut before).expect("the picture");

    // A sentinel in work RAM, so this test cannot pass by the load doing nothing at all:
    // nestest's menu is a still picture, so comparing two frames of it would prove nothing.
    fceux.write_memory(0x0010, 0x5A).expect("the sentinel");
    for _ in 0..10 {
        fceux.run_frame().expect("a frame");
    }
    assert_eq!(fceux.read_memory(0x0010).expect("the sentinel"), 0x5A);

    fceux.load_state(&state).expect("the savestate must load");
    let mut after = Vec::new();
    fceux.frame_indices_into(&mut after).expect("the picture");
    assert_eq!(before, after, "the picture must come back");
    assert_ne!(
        fceux.read_memory(0x0010).expect("the sentinel"),
        0x5A,
        "loading the state must roll the work RAM back, not leave it as it was"
    );

    // The allocation is reused rather than reallocated, which is what a rollback history wants.
    let capacity = state.capacity();
    fceux
        .save_state_into(&mut state)
        .expect("a second savestate");
    assert!(state.capacity() <= capacity * 2);
}

#[test]
fn an_empty_savestate_is_refused() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).expect("nestest.nes");

    let mut fceux = Fceux::new().expect("an instance");
    fceux
        .load_rom(&image, "nestest.nes")
        .expect("nestest must load");
    assert!(matches!(fceux.load_state(&[]), Err(Error::BadArgument(_))));
}

#[test]
fn work_ram_can_be_written_and_read_back_without_side_effects() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).expect("nestest.nes");

    let mut fceux = Fceux::new().expect("an instance");
    fceux
        .load_rom(&image, "nestest.nes")
        .expect("nestest must load");
    fceux.run_frame().expect("a frame");

    fceux.write_memory(0x0010, 0xA5).expect("a write");
    assert_eq!(fceux.read_memory(0x0010).expect("a read"), 0xA5);

    // Reading twice must give the same answer: the read is side-effect-free by contract, so it
    // cannot be clocking anything.
    let first = fceux.read_memory(0x2002).expect("a read of $2002");
    assert_eq!(fceux.read_memory(0x2002).expect("a read of $2002"), first);
}

#[test]
fn the_three_deterministic_ram_fills_differ_and_repeat() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).expect("nestest.nes");

    // $0010 is work RAM, so it shows the fill directly after a power-on and before the game has
    // had a chance to write there.
    let sample = |init: RamInit| -> Vec<u8> {
        let mut fceux = Fceux::new().expect("an instance");
        fceux.set_ram_init(init).expect("a RAM fill");
        fceux
            .load_rom(&image, "nestest.nes")
            .expect("nestest must load");
        (0x0000u16..0x0020)
            .map(|address| fceux.read_memory(address).unwrap_or(0))
            .collect()
    };

    let default = sample(RamInit::Default);
    let ones = sample(RamInit::Ff);
    let zeroes = sample(RamInit::Zero);

    assert!(ones.iter().all(|&byte| byte == 0xFF), "got {ones:?}");
    assert!(zeroes.iter().all(|&byte| byte == 0x00), "got {zeroes:?}");
    assert_ne!(default, ones);
    assert_ne!(default, zeroes);

    // Deterministic: the same option gives the same bytes again. This is what lets step 5's
    // comparison mean anything.
    assert_eq!(sample(RamInit::Default), default);
}

#[test]
fn the_region_can_be_overridden_after_a_load() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).expect("nestest.nes");

    let mut fceux = Fceux::new().expect("an instance");
    fceux
        .load_rom(&image, "nestest.nes")
        .expect("nestest must load");
    assert_eq!(fceux.region().expect("a region"), Region::Ntsc);

    for region in [Region::Pal, Region::Dendy, Region::Ntsc] {
        fceux.set_region(region).expect("a region");
        assert_eq!(fceux.region().expect("a region"), region);
    }
}

#[test]
fn only_the_rates_fceux_supports_are_accepted() {
    let _serial = lock();

    let mut fceux = Fceux::new().expect("an instance");
    for rate in [0, 44_100, 48_000, 96_000] {
        fceux.set_sample_rate(rate).expect("a supported rate");
        assert_eq!(fceux.sample_rate(), rate);
    }
    for rate in [22_050, 32_000, -1, 192_000] {
        assert!(
            matches!(fceux.set_sample_rate(rate), Err(Error::BadArgument(_))),
            "{rate} should be refused"
        );
    }
}

/// The constants in `src/sys.rs` are copied from the fork's header rather than generated, so this
/// reads the header and checks every one of them. A drift is a test failure here instead of a
/// wrong answer at run time.
#[test]
fn the_error_codes_match_the_forks_header() {
    let header = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(|ancestor| {
            let candidate = ancestor.join("fceux-zax/zax/zaxfceux.h");
            candidate.is_file().then_some(candidate)
        });
    let Some(header) = header else {
        eprintln!("skipped: the fork's zax/zaxfceux.h was not found beside this worktree");
        return;
    };
    let text = std::fs::read_to_string(&header).expect("the fork's header");

    let expected: [(&str, i64); 19] = [
        ("ZAX_OK", 0),
        ("ZAX_ERR_ALREADY_LIVE", -1),
        ("ZAX_ERR_NO_INSTANCE", -2),
        ("ZAX_ERR_NO_ROM", -3),
        ("ZAX_ERR_BAD_ARGUMENT", -4),
        ("ZAX_ERR_LOAD_FAILED", -5),
        ("ZAX_ERR_BUFFER_TOO_SMALL", -6),
        ("ZAX_ERR_STATE_FAILED", -7),
        ("ZAX_REGION_NTSC", 0),
        ("ZAX_REGION_PAL", 1),
        ("ZAX_REGION_DENDY", 2),
        ("ZAX_RAM_INIT_DEFAULT", 0),
        ("ZAX_RAM_INIT_FF", 1),
        ("ZAX_RAM_INIT_00", 2),
        ("ZAX_RAM_INIT_RANDOM", 3),
        ("ZAX_SCREEN_WIDTH", 256),
        ("ZAX_SCREEN_HEIGHT", 240),
        // The two the Rust side derives rather than declares, checked against the header's own.
        (
            "ZAX_SCREEN_WIDTH",
            i64::try_from(zax_fceux::FRAME_WIDTH).unwrap_or(-1),
        ),
        (
            "ZAX_SCREEN_HEIGHT",
            i64::try_from(zax_fceux::FRAME_HEIGHT).unwrap_or(-1),
        ),
    ];

    for (name, value) in expected {
        let needle = format!("#define {name} ");
        let line = text
            .lines()
            .find(|line| line.trim_start().starts_with(&needle))
            .unwrap_or_else(|| panic!("{name} is not defined in {}", header.display()));
        let found: i64 = line
            .trim_start()
            .trim_start_matches(&needle)
            .split_whitespace()
            .next()
            .and_then(|token| token.parse().ok())
            .unwrap_or_else(|| panic!("cannot read the value of {name} from {line:?}"));
        assert_eq!(found, value, "{name} has drifted from the fork's header");
    }

    assert_eq!(FRAME_LEN, zax_fceux::FRAME_WIDTH * zax_fceux::FRAME_HEIGHT);
}

/// How many ROMs one process can load before FCEUX falls over.
///
/// Not a feature test: a **measurement** of a limit lot FCEUX-1 found the hard way, when
/// `fceux-run` segfaulted part-way through the 291 public ROMs. `#[ignore]`d because it is slow
/// and because its job is to be run deliberately and have its number read.
#[test]
#[ignore = "a measurement, run it deliberately: cargo test -- --ignored how_many"]
fn how_many_loads_one_process_survives() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).expect("nestest.nes");

    let mut fceux = Fceux::new().expect("an instance");
    for load in 1..=400u32 {
        if let Err(error) = fceux.load_rom(&image, "nestest.nes") {
            panic!("load {load} failed: {error}");
        }
        for _ in 0..2 {
            fceux.run_frame().expect("a frame");
        }
        if load % 50 == 0 {
            eprintln!("{load} loads of the same ROM survived");
        }
    }
    eprintln!("400 loads of the same ROM survived");
}

/// **The capture must change nothing** (REQ-CORE-033): the same ROM run with the tile capture on
/// and off must produce the same pictures, frame for frame.
///
/// This is the test that guards the save-and-restore contract of lot FCEUX-6. Two of the capture's
/// steps *move emulator state* while they walk a line — the MMC2/MMC4 `$FD`/`$FE` latch
/// (REQ-CORE-039), which also remaps CHR through `setchr4`, and the MMC5's per-tile state — and
/// both put it back afterwards. If either restore were wrong, the picture of a mapper 9, 10 or 5
/// game would drift the moment a host asked for records, and nothing else in the suite would
/// notice.
///
/// `holy-mapperel`'s mapper 9 and 10 ROMs are the ones that matter for the latch and they live in
/// `assets-free/test-roms`, which `ZAXNES_TEST_ROMS` points at; the MMC5 ones are
/// `mmc5test_v2/mmc5test.nes` and `exram/mmc5exram.nes` of `rust/test-roms`; and `nestest` is
/// always checked as the control.
#[test]
fn the_tile_capture_does_not_change_what_fceux_draws() {
    let _serial = lock();

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(rom) = nestest() {
        candidates.push(rom);
    }
    // The MMC2, MMC4 and MMC5 ROMs, whose boards are the ones the capture moves state on.
    if let Ok(root) = std::env::var("ZAXNES_TEST_ROMS") {
        let root = Path::new(&root).join("holy-mapperel/testroms");
        for name in [
            "M9_P128K_C64K.nes",
            "M10_P128K_C64K_S8K.nes",
            "M10_P128K_C64K_W8K.nes",
        ] {
            let rom = root.join(name);
            if rom.is_file() {
                candidates.push(rom);
            }
        }
    }
    // The MMC5 ones are in `rust/test-roms`, where `nestest` is.
    let public = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms");
    for name in ["mmc5test_v2/mmc5test.nes", "exram/mmc5exram.nes"] {
        let rom = public.join(name);
        if rom.is_file() {
            candidates.push(rom);
        }
    }
    if candidates.is_empty() {
        eprintln!("skipped: no ROM to run");
        return;
    }

    let mut fceux = Fceux::new().expect("an instance");
    // **Silent on purpose, and not only because the audio is not what is being tested.**
    // `SexyFilter` (`src/filter.cpp:53`) keeps its two DC-blocker accumulators in `static` locals
    // that nothing resets — not `FCEUSND_Power`, not `MakeFilters`, not a game load — so a test
    // that runs a thousand frames over six ROMs leaves a tail behind that
    // `sixty_frames_of_nestest_match_the_c_self_tests_digests` then measures as a different audio
    // digest. With the rate at 0, `FlushEmulateSound` returns before the filter
    // (`src/sound.cpp:1043-1048`), so this test cannot perturb it. The carry-over itself is
    // upstream FCEUX's and is recorded in `docs/progress/FCEUX-6.md`, step 4.
    let rate = fceux.sample_rate();
    let _ = fceux.set_sample_rate(0);
    for rom in &candidates {
        let image = std::fs::read(rom).expect("the ROM");
        let name = rom
            .file_name()
            .map_or_else(String::new, |base| base.to_string_lossy().into_owned());

        let mut digests = Vec::new();
        for capture in [false, true] {
            fceux.set_tile_capture(capture);
            fceux.set_ram_init(RamInit::Zero).expect("the RAM fill");
            fceux.load_rom(&image, &name).expect("the load");
            fceux.reset().expect("a reset");

            let mut hash = FNV_OFFSET;
            let mut frame = Vec::new();
            for _ in 0..180 {
                fceux.set_input(0, 0).expect("the pads");
                fceux.run_frame().expect("a frame");
                fceux.frame_indices_into(&mut frame).expect("the picture");
                hash = digest(&frame, hash);
            }
            digests.push(hash);
        }
        fceux.set_tile_capture(false);

        assert_eq!(
            digests[0], digests[1],
            "{name}: 180 frames differ with the tile capture on ({:016x}) and off ({:016x}) — \
             something the capture moves is not being put back (REQ-CORE-033)",
            digests[1], digests[0]
        );
        eprintln!(
            "{name}: capture on and off agree, pixels {:016x}",
            digests[0]
        );
    }
    let _ = fceux.set_sample_rate(rate);
}
