//! `movie-sync` over **small synthetic movies**, written by the test itself to the `.fm2` format.
//!
//! The movies are built here rather than downloaded (the lot forbids downloads) and rather than
//! recorded through FCEUX's own recorder (which writes to a path, and this build has no
//! filesystem under it). Three public test ROMs carry the whole suite, and each is there for a
//! reason a measurement settled rather than a reading:
//!
//! * **`dmc_tests/buffer_retained.nes`** is a ROM the two engines agree on completely — picture
//!   *and* work RAM, at every frame, with inputs, with resets and with the probe on. It is the
//!   ROM that proves the instrument says "no divergence" when there is none. Finding it took two
//!   sweeps, and both are worth recording because they say how rare agreement is: of the **290**
//!   public ROMs run through `fceux-tiles` at frames 60 and 120, only **25** have no pixel
//!   difference at all; of those 25 run through `movie-sync` itself over 121 frames, only
//!   **five** also have no work-RAM difference. `nestest.nes` — the obvious candidate, and the
//!   one this file first used — is in neither set: the two engines part on it at frame **4**, on
//!   the power-on palette RAM, whatever the work-RAM pattern is. And
//!   `branch_timing_tests/1.Branch_Basics.nes`, which has no pixel difference, still parts on
//!   three bytes **below the stack pointer** ($01FA-$01FC) — dead leftovers of earlier pushes
//!   that no game reads, which the instrument reports because it reports what differs;
//! * **`oam3.nes`** is a ROM they do **not** agree on — lot FCEUX-48 step 1 measured
//!   `first_pixel_diff` 3 on it — so it is the ROM that proves the instrument can **fail**. A
//!   comparison tool that has never been seen to find a difference is worth nothing;
//! * **`nestest.nes`** is the ROM the one-engine tests use, because it polls the pad at its menu
//!   and so has input-sensitive frames to measure.
//!
//! The ROMs are not in the repository, so every test here returns at once unless
//! `ZAXNES_TEST_ROMS` points at a copy of `assets-free/test-roms/`.
//!
//! The tests that need two machines are `#[ignore]`d and `fceux-core`-gated, as every FCEUX test
//! of the workspace is: FCEUX keeps one machine per process, so they must not run beside each
//! other.

// A test: a missing ROM or a refused movie should fail it outright rather than be smoothed over.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use zax_harness::movie_file::{FrameCommand, MovieFile, RamPattern};
use zax_harness::movie_sync::{self, Engines, Options};

/// The directory `ZAXNES_TEST_ROMS` names, or `None` when it is unset.
fn roms_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("ZAXNES_TEST_ROMS")?);
    dir.is_dir().then_some(dir)
}

/// A ROM of `assets-free/test-roms/nes-test-roms`, or `None` with a printed reason.
fn rom(name: &str) -> Option<PathBuf> {
    let path = roms_dir()?.join("nes-test-roms").join(name);
    if !path.is_file() {
        println!("skipped: {} is not there", path.display());
        return None;
    }
    Some(path)
}

/// The ROM the two engines agree on, picture and work RAM, frame for frame.
/// Only the lockstep tests use it, and those need the second engine.
#[cfg(feature = "fceux-core")]
const AGREES: &str = "dmc_tests/buffer_retained.nes";
/// A ROM they part on early, so that the instrument is seen to be able to fail.
#[cfg(feature = "fceux-core")]
const PARTS: &str = "other/oam3.nes";
/// A ROM with a menu that polls the pad, for the one-engine measurements.
const POLLS: &str = "other/nestest.nes";

/// A scratch directory of this test's own, under the target directory.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    dir
}

/// Writes a one-player `.fm2` of `frames` frames and answers its path.
///
/// `extra` goes into the header, and `at` names frames that carry a command column other than 0.
/// The input word of frame `n` is `button(n)`, printed in FCEUX's own `RLDUTSBA` order with bit 7
/// first, which is `MovieRecord::mnemonics` indexed by bit.
fn write_movie(
    dir: &Path,
    name: &str,
    frames: u32,
    extra: &str,
    at: &[(u32, u8)],
    button: impl Fn(u32) -> u8,
) -> PathBuf {
    let mut text = format!(
        "version 3\nemuVersion 22020\nrerecordCount 1\npalFlag 0\n\
         romFilename synthetic\nfourscore 0\nmicrophone 0\nport0 1\nport1 0\nport2 0\n{extra}"
    );
    for frame in 0..frames {
        let command = at
            .iter()
            .find(|(which, _)| *which == frame)
            .map_or(0, |(_, command)| *command);
        let byte = button(frame);
        let pad: String = (0..8)
            .map(|index| {
                let bit = 7 - index;
                if byte & (1 << bit) == 0 {
                    '.'
                } else {
                    b"RLDUTSBA"[index] as char
                }
            })
            .collect();
        let _ = writeln!(text, "|{command}|{pad}|||");
    }
    let path = dir.join(name);
    std::fs::write(&path, text).expect("writing the movie");
    path
}

/// The options of a run, with everything that makes a test slow turned off by default.
fn options(movie: &Path, rom: &Path, engine: Engines, frames: u32) -> Options {
    Options {
        movie: movie.to_path_buf(),
        rom: rom.to_path_buf(),
        engine,
        frames: Some(frames),
        lag: false,
        ..Options::default()
    }
}

// ------------------------------------------------------------------------------------------------
// What one machine can be measured with, in any build
// ------------------------------------------------------------------------------------------------

#[test]
fn a_movies_own_header_is_what_drives_the_machine() {
    let Some(rom) = rom(POLLS) else {
        return;
    };
    let dir = scratch("movie-sync-header");
    // `palFlag 1`, a seeded random fill, and a soft reset at frame 20.
    let movie = write_movie(
        &dir,
        "header.fm2",
        40,
        "palFlag 1\nRAMInitOption 3\nRAMInitSeed 4242\n",
        &[(0, 2), (20, 1)],
        |_| 0,
    );
    let bytes = std::fs::read(&movie).expect("the movie back");
    let read = MovieFile::read_fm2(&bytes, true).expect("a readable movie");
    assert!(read.pal, "palFlag 1 has to reach the machine");
    assert_eq!(read.ram, RamPattern::Random(4242));
    assert_eq!(read.frames.len(), 40);
    // The power-on command of frame 0 is not a reset; the one at frame 20 is.
    assert_eq!(read.commands(), [(20, FrameCommand::SoftReset)]);

    // And the run honours it: a PAL machine over 40 frames produces a picture, and the run is
    // not reported as stopped.
    let measurement =
        movie_sync::run(&options(&movie, &rom, Engines::Zaxnes, 40)).expect("the run");
    assert_eq!(measurement.runs.len(), 1);
    assert_eq!(measurement.runs[0].frames, 40);
    assert!(measurement.runs[0].stopped.is_none());
    assert!(measurement.movie.pal);
    assert_eq!(
        measurement.movie.ram,
        RamPattern::Random(4242).to_string(),
        "the report names the fill it used"
    );
}

#[test]
fn the_input_sensitivity_probe_leaves_the_run_exactly_where_it_was() {
    let Some(rom) = rom(POLLS) else {
        return;
    };
    let dir = scratch("movie-sync-probe");
    // Start pressed every 16th frame, so the run is not a still screen.
    let movie = write_movie(&dir, "probe.fm2", 90, "", &[], |frame| {
        if frame % 16 == 0 {
            0x08
        } else {
            0
        }
    });
    let plain = movie_sync::run(&options(&movie, &rom, Engines::Zaxnes, 90)).expect("the run");
    let probed = movie_sync::run(&Options {
        lag: true,
        ..options(&movie, &rom, Engines::Zaxnes, 90)
    })
    .expect("the probed run");

    // This is the probe's whole contract: it runs two extra frames per frame and must leave the
    // machine on the movie's own path, picture included.
    assert_eq!(
        plain.runs[0].last_picture, probed.runs[0].last_picture,
        "the probe moved the run"
    );
    assert_eq!(
        plain.runs[0].last_picture_change,
        probed.runs[0].last_picture_change
    );
    assert_eq!(probed.runs[0].probed.0, 90, "every frame probed");
    // `nestest` sits at a menu that polls the pad, so most frames are input-sensitive. The claim
    // tested is only that the probe answers both ways somewhere in the run rather than being
    // stuck on one of them, which a broken probe would be.
    assert!(
        probed.runs[0].probed.1 > 0,
        "no frame measured input-sensitive, so the probe never sees the input"
    );
}

#[test]
fn a_movie_recorded_on_another_rom_is_reported_and_still_played() {
    let Some(rom) = rom(POLLS) else {
        return;
    };
    let dir = scratch("movie-sync-checksum");
    // A checksum that is certainly not this ROM's.
    let movie = write_movie(
        &dir,
        "wrong-rom.fm2",
        10,
        "romChecksum base64:AAAAAAAAAAAAAAAAAAAAAA==\n",
        &[],
        |_| 0,
    );
    let measurement =
        movie_sync::run(&options(&movie, &rom, Engines::Zaxnes, 10)).expect("the run");
    assert_eq!(
        measurement.movie.rom_matches,
        Some(false),
        "the mismatch has to be stated"
    );
    assert_eq!(measurement.runs[0].frames, 10, "and the movie still played");
    assert!(measurement.report().contains("NOT the one"));

    // A movie that states no checksum claims nothing, which is not the same as claiming wrongly.
    let silent = write_movie(&dir, "no-checksum.fm2", 10, "", &[], |_| 0);
    let quiet = movie_sync::run(&options(&silent, &rom, Engines::Zaxnes, 10)).expect("the run");
    assert_eq!(quiet.movie.rom_matches, None);
    assert!(quiet.report().contains("unverified"));
}

// ------------------------------------------------------------------------------------------------
// The work-RAM trace, and the offset calibration it rests on (lot EVAL-SYNC-2 step 1)
// ------------------------------------------------------------------------------------------------

/// A run's own trace, written out and then read back as the reference for the same run, has to
/// come out at **offset 0 with every line and every lag count agreeing**.
///
/// It is a round trip rather than a tautology: the writer emits one line every `--trace-step`
/// frames from a per-frame array, and the comparison reads those line numbers back through a
/// different path (`Reference::parse`, then `RamTrace::at`). An off-by-one in either half shows
/// up here as a win at offset ±1 or as nothing matching at all — which is exactly the mistake the
/// calibration on the real `BizHawk` traces exists to rule out.
#[test]
fn a_runs_own_trace_read_back_as_its_reference_lands_on_offset_zero() {
    let Some(rom) = rom(POLLS) else {
        return;
    };
    let dir = scratch("movie-sync-trace");
    let movie = write_movie(&dir, "trace.fm2", 300, "", &[], |frame| {
        if frame % 16 == 0 {
            0x08
        } else {
            0
        }
    });
    let path = dir.join("own.txt");
    let written = movie_sync::run(&Options {
        lag: true,
        trace: Some(path.clone()),
        trace_step: 50,
        ..options(&movie, &rom, Engines::Zaxnes, 300)
    })
    .expect("the traced run");
    assert_eq!(written.traces.len(), 1);
    assert_eq!(written.traces[0].engine, "zaxnes");
    assert_eq!(written.traces[0].length(), 300);

    let text = std::fs::read_to_string(&path).expect("the trace back");
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with(|c: char| c.is_ascii_digit()))
        .collect();
    assert_eq!(lines.len(), 6, "one line every 50 frames of 300: {text}");
    assert!(lines[0].starts_with("50 "), "{text}");
    assert!(
        lines[0].contains(" lag="),
        "the probe ran, so the column is there: {text}"
    );
    // The trace's final lag count is the same quantity the one-engine report counts, arrived at
    // by a different route: frames probed minus frames input-sensitive.
    let insensitive = written.runs[0].probed.0 - written.runs[0].probed.1;
    let last = written.traces[0].at(300).expect("the last frame");
    assert_eq!(last.lag, Some(insensitive));

    let against = movie_sync::run(&Options {
        lag: true,
        trace_ref: Some(path.clone()),
        trace_step: 50,
        ..options(&movie, &rom, Engines::Zaxnes, 300)
    })
    .expect("the compared run");
    let (engine, best, _) = against
        .against_reference
        .first()
        .expect("a comparison was made");
    assert_eq!(engine, "zaxnes");
    assert_eq!(best.offset, 0, "the two numberings line up at 0");
    assert_eq!(best.compared, 6);
    assert_eq!(best.matched, 6, "every line agrees");
    assert_eq!(best.first_diff, None);
    assert_eq!(best.first_lag_diff, None);
    assert!(
        against.report().contains("every compared line agrees"),
        "{}",
        against.report()
    );
}

/// And a reference whose frames are numbered one higher is found at offset **-1** rather than
/// silently reported as a divergence. This is the calibration doing its job on real emulator
/// output instead of on a synthetic one.
#[test]
fn a_reference_numbered_one_frame_out_is_found_at_the_offset_it_is_out_by() {
    let Some(rom) = rom(POLLS) else {
        return;
    };
    let dir = scratch("movie-sync-trace-offset");
    let movie = write_movie(&dir, "offset.fm2", 300, "", &[], |frame| {
        if frame % 16 == 0 {
            0x08
        } else {
            0
        }
    });
    let own = dir.join("own.txt");
    movie_sync::run(&Options {
        trace: Some(own.clone()),
        trace_step: 50,
        ..options(&movie, &rom, Engines::Zaxnes, 300)
    })
    .expect("the traced run");

    // The same checksums, every frame number one higher.
    let shifted: String = std::fs::read_to_string(&own)
        .expect("the trace back")
        .lines()
        .filter_map(|line| {
            let (frame, rest) = line.split_once(' ')?;
            let frame: u32 = frame.parse().ok()?;
            Some(format!("{} {rest}\n", frame + 1))
        })
        .collect();
    let path = dir.join("shifted.txt");
    std::fs::write(&path, shifted).expect("writing the shifted reference");

    let against = movie_sync::run(&Options {
        trace_ref: Some(path),
        trace_step: 50,
        ..options(&movie, &rom, Engines::Zaxnes, 300)
    })
    .expect("the compared run");
    let (_, best, runner_up) = against.against_reference.first().expect("a comparison");
    assert_eq!(best.offset, -1);
    assert_eq!(best.matched, 6, "all six lines, once the shift is undone");
    let runner_up = runner_up.as_ref().expect("a second offset was tried");
    assert_eq!(
        runner_up.matched, 0,
        "no other offset matches a line, which is what makes the winner a measurement"
    );
}

#[test]
fn a_reference_that_is_not_a_trace_is_refused_by_name() {
    let Some(rom) = rom(POLLS) else {
        return;
    };
    let dir = scratch("movie-sync-trace-refusal");
    let movie = write_movie(&dir, "refusal.fm2", 10, "", &[], |_| 0);
    let path = dir.join("prose.txt");
    std::fs::write(&path, "this is not a trace at all\n").expect("writing the file");
    let error = movie_sync::run(&Options {
        trace_ref: Some(path),
        ..options(&movie, &rom, Engines::Zaxnes, 10)
    })
    .expect_err("a reference with no numeric line is refused");
    assert!(error.to_string().contains("prose.txt"), "{error}");
    assert!(
        error.to_string().contains("<frame> <checksum>"),
        "the refusal has to say what it wanted: {error}"
    );
}

#[test]
fn a_file_that_is_not_a_movie_is_refused_by_name() {
    let Some(rom) = rom(POLLS) else {
        return;
    };
    let dir = scratch("movie-sync-refusals");
    for (name, bytes) in [
        ("prose.fm2", b"this is not a movie at all\n".to_vec()),
        ("version2.fm2", b"version 2\n|0|........|||\n".to_vec()),
        (
            "newppu.fm2",
            b"version 3\nNewPPU 1\n|0|........|||\n".to_vec(),
        ),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("writing the file");
        let error = movie_sync::run(&options(&path, &rom, Engines::Zaxnes, 1))
            .expect_err(&format!("{name} should be refused"));
        assert!(
            error.to_string().contains(name),
            "the refusal has to name the file: {error}"
        );
    }
    // And a file whose extension is not one this instrument reads.
    let odd = dir.join("movie.zmv");
    std::fs::write(&odd, b"ZAXMOVIE").expect("writing the file");
    let error = movie_sync::run(&options(&odd, &rom, Engines::Zaxnes, 1)).expect_err("refused");
    assert!(
        error.to_string().contains("neither an .fm2 nor a .bk2"),
        "{error}"
    );
}

// ------------------------------------------------------------------------------------------------
// The lockstep comparison, which needs both engines
// ------------------------------------------------------------------------------------------------

#[cfg(feature = "fceux-core")]
mod both {
    use super::{options, rom, scratch, write_movie, AGREES, PARTS, POLLS};
    use zax_harness::movie_file::RamPattern;
    use zax_harness::movie_sync::{self, Engines, Options, Resets};

    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn on_a_rom_the_two_engines_agree_on_there_is_no_divergence() {
        let Some(rom) = rom(AGREES) else { return };
        let dir = scratch("movie-sync-agree");
        let movie = write_movie(&dir, "agree.fm2", 120, "", &[], |frame| {
            if frame % 16 == 0 {
                0x08
            } else {
                0
            }
        });
        let measurement =
            movie_sync::run(&options(&movie, &rom, Engines::Both, 120)).expect("the run");
        assert_eq!(measurement.runs.len(), 2);
        assert_eq!(measurement.runs[0].frames, 120);
        assert_eq!(measurement.runs[1].frames, 120);
        assert!(
            measurement.first_divergence.is_none(),
            "{AGREES} should not part: {}",
            measurement.report()
        );
        // 121, because frame 0 -- the machine at power-on -- is compared as well.
        assert_eq!(measurement.agreed_frames, 121);
        assert!(measurement
            .report()
            .contains("agree at every one of the 121 frames"));
    }

    /// **When an `.fm2`'s command column takes effect**, settled against FCEUX itself.
    ///
    /// The public format page says a frame's commands are "interpreted as a bit field
    /// corresponding to miscellaneous input states which are **valid at the start of the frame**",
    /// and that is a sentence rather than a measurement — so this test plays the *same* synthetic
    /// movie twice: once through `FCEUI_LoadMovie`, where FCEUX's own movie player decides when
    /// the reset happens, and once through this instrument's fed inputs at each of the three
    /// placements. Only one of the three can reproduce the reference, and the test asserts both
    /// halves: that `Resets::On` reproduces it **frame for frame over the whole 2 KiB**, and that
    /// the two neighbours do not — without which "they all match" would pass vacuously on a ROM
    /// whose RAM the reset does not reach.
    ///
    /// Lot EVAL-SYNC-3 step 1 ran it at three reset frames (5, 20 and 45) with the same answer;
    /// one of the three is kept here, because the slow part is FCEUX.
    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn an_fm2_resets_before_the_flagged_frame_and_fceuxs_own_movie_mode_says_so() {
        let Some(rom) = rom(POLLS) else { return };
        let dir = scratch("movie-sync-fm2-reset");
        // One soft reset (command 1) on frame 20, and an A press on frame 30 so that the movie
        // is not silent.
        let movie = write_movie(&dir, "reset20.fm2", 60, "", &[(20, 1)], |frame| {
            u8::from(frame == 30)
        });
        // The traces are compared through the files the run writes, which is also how the lot's
        // own command line did it.
        let numbers = |name: &str, resets, movie_mode| -> Vec<String> {
            let path = dir.join(name);
            movie_sync::run(&Options {
                trace: Some(path.clone()),
                trace_step: 1,
                movie_mode,
                resets,
                ..options(&movie, &rom, Engines::Fceux, 60)
            })
            .expect("the run");
            std::fs::read_to_string(&path)
                .expect("the trace")
                .lines()
                .filter(|line| line.starts_with(|first: char| first.is_ascii_digit()))
                .map(ToOwned::to_owned)
                .collect()
        };

        let reference = numbers("own.txt", Resets::On, true);
        assert_eq!(reference.len(), 60, "one line a frame");
        assert_eq!(
            numbers("fed-on.txt", Resets::On, false),
            reference,
            "FCEUX's own movie mode applies a frame's command BEFORE that frame, so the \
             instrument's default placement has to reproduce it exactly"
        );
        for (name, resets) in [
            ("fed-early.txt", Resets::Early),
            ("fed-late.txt", Resets::Late),
        ] {
            assert_ne!(
                numbers(name, resets, false),
                reference,
                "{resets:?} must NOT reproduce movie mode, or this test proves nothing"
            );
        }
    }

    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn on_a_rom_they_do_not_agree_on_the_divergence_is_found_and_described() {
        let Some(rom) = rom(PARTS) else { return };
        let dir = scratch("movie-sync-part");
        let movie = write_movie(&dir, "part.fm2", 30, "", &[], |_| 0);
        let measurement =
            movie_sync::run(&options(&movie, &rom, Engines::Both, 30)).expect("the run");
        let first = measurement
            .first_divergence
            .as_ref()
            .expect("oam3.nes parts the two engines -- lot FCEUX-48 step 1 measured frame 3");
        assert!(
            first.frame > 0,
            "the power-on work RAM is given to both machines, so frame 0 cannot be the first"
        );
        assert!(
            first.frame <= 10,
            "the two engines part early on this ROM, not at frame {}",
            first.frame
        );
        assert!(
            first.pixels > 0 || first.ram_count > 0,
            "a divergence has to be made of something: {}",
            first.line()
        );
        let report = measurement.report();
        assert!(report.contains("FIRST DIVERGENCE"), "{report}");
        // And the run went on past it, which is what answers "do they come back together".
        assert!(measurement.after.0 > 0, "{report}");
    }

    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn the_power_on_pattern_reaches_both_machines_and_not_just_one() {
        let Some(rom) = rom(AGREES) else { return };
        let dir = scratch("movie-sync-ram");
        // All four patterns, compared at **frame 0** -- the machine before a frame has run, which
        // is the only place a power-on fill can be measured without emulation in the way. If a
        // pattern reached one side only, frame 0's work RAM would differ by hundreds of bytes.
        for (option, seed, pattern) in [
            ("0", "0", RamPattern::FceuxDefault),
            ("1", "0", RamPattern::Ff),
            ("2", "0", RamPattern::Zero),
            ("3", "12345", RamPattern::Random(12345)),
        ] {
            let movie = write_movie(
                &dir,
                &format!("ram-{option}.fm2"),
                4,
                &format!("RAMInitOption {option}\nRAMInitSeed {seed}\n"),
                &[],
                |_| 0,
            );
            let measurement =
                movie_sync::run(&options(&movie, &rom, Engines::Both, 0)).expect("the run");
            assert_eq!(measurement.movie.ram, pattern.to_string());
            assert_eq!(
                measurement.agreed_frames, 1,
                "frame 0 is the one frame compared"
            );
            assert!(
                measurement.first_divergence.is_none(),
                "RAMInitOption {option} did not reach both machines: {}",
                measurement.report()
            );
        }
    }

    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn a_soft_reset_in_the_movie_reaches_both_machines() {
        let Some(rom) = rom(AGREES) else { return };
        let dir = scratch("movie-sync-reset");
        // A soft reset at frame 20 and a power cycle at frame 40, each of which parts the two
        // machines at once if one of them ignores it.
        let movie = write_movie(
            &dir,
            "resets.fm2",
            70,
            "",
            &[(0, 2), (20, 1), (40, 2)],
            |_| 0,
        );
        let measurement =
            movie_sync::run(&options(&movie, &rom, Engines::Both, 70)).expect("the run");
        assert_eq!(measurement.movie.commands.len(), 2, "frames 20 and 40");
        assert!(
            measurement.first_divergence.is_none(),
            "a command did not reach both machines: {}",
            measurement.report()
        );
    }

    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn the_probe_leaves_the_lockstep_run_where_it_was_on_both_machines() {
        let Some(rom) = rom(AGREES) else { return };
        let dir = scratch("movie-sync-probe-both");
        let movie = write_movie(&dir, "probe-both.fm2", 60, "", &[], |frame| {
            u8::from(frame % 8 == 0)
        });
        let plain = movie_sync::run(&options(&movie, &rom, Engines::Both, 60)).expect("the run");
        let probed = movie_sync::run(&Options {
            lag: true,
            ..options(&movie, &rom, Engines::Both, 60)
        })
        .expect("the probed run");
        for index in 0..2 {
            assert_eq!(
                plain.runs[index].last_picture, probed.runs[index].last_picture,
                "the probe moved {}",
                plain.runs[index].engine
            );
        }
        // FCEUX's state block is a different mechanism from Zaxnes's `CoreState`, so the probe
        // being sound on one engine is no evidence about the other; both are checked.
        assert_eq!(probed.runs[0].probed.0, 60);
        assert_eq!(probed.runs[1].probed.0, 60);
        assert!(probed.first_divergence.is_none(), "{}", probed.report());
    }

    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn the_two_fceux_paths_give_the_same_frames() {
        let Some(rom) = rom(POLLS) else { return };
        let dir = scratch("movie-sync-movie-mode");
        let movie = write_movie(&dir, "both-paths.fm2", 150, "", &[], |frame| {
            if frame % 12 == 0 {
                0x08
            } else {
                0
            }
        });
        // The lockstep run feeds FCEUX the inputs directly; the application plays the file
        // through FCEUX's own movie mode. The instrument is only worth its name if the two give
        // the same machine, so it is measured at three lengths rather than one.
        for frames in [30, 90, 150] {
            let direct = movie_sync::run(&options(&movie, &rom, Engines::Fceux, frames))
                .expect("the direct run");
            let mode = movie_sync::run(&Options {
                movie_mode: true,
                ..options(&movie, &rom, Engines::Fceux, frames)
            })
            .expect("the movie-mode run");
            assert_eq!(direct.runs[0].frames, frames);
            assert_eq!(mode.runs[0].frames, frames);
            assert_eq!(
                direct.runs[0].last_picture, mode.runs[0].last_picture,
                "the two FCEUX paths part at frame {frames}"
            );
            assert_eq!(
                direct.runs[0].last_picture_change,
                mode.runs[0].last_picture_change
            );
            assert!(
                mode.runs[0].engine.contains("clock"),
                "movie mode should report FCEUX's own clock: {}",
                mode.runs[0].engine
            );
        }
    }

    /// `RamPattern::Random` has to be `FCEU_MemoryRand` option 3, byte for byte, or the
    /// instrument would be comparing two different machines and calling the difference an
    /// emulation error.
    ///
    /// It can only be measured **from inside movie mode**, and that is the whole reason
    /// [`movie_sync`]'s FCEUX side writes the 2 KiB itself: `PowerNES` reseeds `RAMInitSeed` from
    /// `rand()` on every power-on unless a movie is active (`fceu.cpp:1031-1039`), so a plain
    /// `set_ram_init(Random(seed))` + `power()` gives a different 2 KiB every time. A movie
    /// carrying `RAMInitOption 3` pins it, which is exactly why the `.fm2` format has the field.
    #[test]
    #[ignore = "needs the FCEUX library; one machine per process, so run it alone"]
    fn the_random_pattern_is_the_one_fceux_writes() {
        let Some(rom_path) = rom(POLLS) else { return };
        let dir = scratch("movie-sync-random");
        let image = std::fs::read(&rom_path).expect("the ROM");
        let mut core = zax_fceux::Fceux::new().expect("one FCEUX");
        core.load_rom(&image, "rom.nes").expect("the image");
        for seed in [0, 1, 12345, 2_000_000_011] {
            let movie = write_movie(
                &dir,
                &format!("seed-{seed}.fm2"),
                2,
                &format!("RAMInitOption 3\nRAMInitSeed {seed}\n"),
                &[],
                |_| 0,
            );
            let bytes = std::fs::read(&movie).expect("the movie");
            core.play_movie(&bytes).expect("FCEUX plays it");
            let theirs: Vec<u8> = (0..2048)
                .map(|address| core.read_memory(address).expect("a byte"))
                .collect();
            let mut ours = vec![0u8; 2048];
            RamPattern::Random(seed).fill(&mut ours);
            assert_eq!(ours, theirs, "seed {seed}");
            core.stop_movie().expect("stopping");
        }
        // And the generator is not accidentally one of the other three patterns.
        let mut ours = vec![0u8; 2048];
        RamPattern::Random(12345).fill(&mut ours);
        assert!(ours.iter().any(|&byte| byte != 0 && byte != 0xFF));
    }
}

// ------------------------------------------------------------------------------------------------
// `.bk2`, hand-built by the test to the documented layout
// ------------------------------------------------------------------------------------------------

/// The same inputs written as an `.fm2` and as a `.bk2` must drive the machine to the same place.
///
/// This is the test the two formats deserve, because **their pad orders are different**: `.fm2`
/// writes `RLDUTSBA` with bit 7 first, and a `.bk2`'s `LogKey` names
/// `Up Down Left Right Start Select B A`. A reader that mapped either onto the other would be
/// wrong in a way no amount of reading the file would show — but it would send the machine
/// somewhere else, and 400 frames of a scripted sequence is enough for that to be visible.
#[test]
fn a_bk2_and_an_fm2_of_the_same_inputs_drive_the_machine_to_the_same_place() {
    const FRAMES: u32 = 400;
    let Some(rom) = rom(POLLS) else { return };
    let dir = scratch("movie-sync-bk2");
    // A sequence that presses every button, so that a mis-mapped bit cannot hide.
    let button = |frame: u32| -> u8 { (1u8 << (frame % 8)) | u8::from(frame % 5 == 0) };

    let fm2 = write_movie(&dir, "same.fm2", FRAMES, "RAMInitOption 2\n", &[], button);
    let bk2 = dir.join("same.bk2");
    std::fs::write(&bk2, bk2_of(FRAMES, button)).expect("writing the .bk2");

    let from_fm2 = movie_sync::run(&options(&fm2, &rom, Engines::Zaxnes, FRAMES)).expect("the fm2");
    let from_bk2 = movie_sync::run(&options(&bk2, &rom, Engines::Zaxnes, FRAMES)).expect("the bk2");

    assert_eq!(from_bk2.movie.kind, "bk2");
    assert_eq!(from_bk2.movie.length, FRAMES);
    assert_eq!(
        from_fm2.runs[0].last_picture, from_bk2.runs[0].last_picture,
        "the two readers do not agree about what the inputs are"
    );
    assert_eq!(
        from_fm2.runs[0].last_picture_change,
        from_bk2.runs[0].last_picture_change
    );
    // The control, without which the agreement above would mean nothing: a `.bk2` of the same
    // length whose pads are never pressed goes somewhere else. So this ROM's picture at frame
    // 400 *does* depend on the inputs, and the two readers agreeing about it is a statement.
    let silent = dir.join("silent.bk2");
    std::fs::write(&silent, bk2_of(FRAMES, |_| 0)).expect("writing it");
    let nothing_pressed =
        movie_sync::run(&options(&silent, &rom, Engines::Zaxnes, FRAMES)).expect("the silent bk2");
    assert_ne!(
        from_bk2.runs[0].last_picture, nothing_pressed.runs[0].last_picture,
        "the picture does not depend on the inputs, so this test would prove nothing"
    );
}

#[test]
fn a_bk2_that_cannot_be_honoured_is_refused_with_its_reason() {
    let Some(rom) = rom(POLLS) else { return };
    let dir = scratch("movie-sync-bk2-refused");
    let anchored = dir.join("anchored.bk2");
    let mut members = bk2_members(4, |_| 0);
    members.push(("CoreState".to_owned(), "a state of another core".to_owned()));
    std::fs::write(&anchored, zip_of(&members)).expect("writing it");
    let error = movie_sync::run(&options(&anchored, &rom, Engines::Zaxnes, 4))
        .expect_err("a savestate-anchored movie is refused");
    assert!(error.to_string().contains("CoreState"), "{error}");
    assert!(error.to_string().contains("anchored.bk2"), "{error}");
}

/// A `.bk2` of `frames` frames whose pad-1 byte is `button(frame)` in `NesCore`'s bit order.
fn bk2_of(frames: u32, button: impl Fn(u32) -> u8) -> Vec<u8> {
    zip_of(&bk2_members(frames, button))
}

/// The members of such a movie, as the format describes them.
fn bk2_members(frames: u32, button: impl Fn(u32) -> u8) -> Vec<(String, String)> {
    // The LogKey of a real NES `.bk2`, read off `invariel-faxanadu.bk2`.
    const KEY: &str = "#Power|Reset|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|P1 B|P1 A|\
                       #P2 Up|P2 Down|P2 Left|P2 Right|P2 Start|P2 Select|P2 B|P2 A|";
    // The bit of `NesCore::run_frame`'s word each character of a pad field stands for, in the
    // key's own order.
    const BITS: [u8; 8] = [0x10, 0x20, 0x40, 0x80, 0x08, 0x04, 0x02, 0x01];
    const LETTERS: [char; 8] = ['U', 'D', 'L', 'R', 'S', 's', 'B', 'A'];

    let mut log = format!("[Input]\r\nLogKey:{KEY}\r\n");
    for frame in 0..frames {
        let byte = button(frame);
        let pad: String = BITS
            .iter()
            .zip(LETTERS)
            .map(|(bit, letter)| if byte & bit == 0 { '.' } else { letter })
            .collect();
        let _ = writeln!(log, "|..|{pad}|........|");
    }
    log.push_str("[/Input]\r\n");
    vec![
        (
            "Header.txt".to_owned(),
            "MovieVersion BizHawk v2.0 Tasproj v1.0\r\nPlatform NES\r\nGameName Test\r\n\
             Core NesHawk\r\nrerecordCount 1\r\n"
                .to_owned(),
        ),
        (
            "SyncSettings.json".to_owned(),
            r#"{"o":{"BoardProperties":{},"RegionOverride":0,"Controls":{"Famicom":false,
               "NesLeftPort":"ControllerNES","NesRightPort":"ControllerNES",
               "FamicomExpPort":"UnpluggedFam"}}}"#
                .to_owned(),
        ),
        ("Input Log.txt".to_owned(), log),
        ("Comments.txt".to_owned(), String::new()),
        ("Subtitles.txt".to_owned(), String::new()),
    ]
}

/// A zip of those members, which is what a `.bk2` is.
fn zip_of(members: &[(String, String)]) -> Vec<u8> {
    use std::io::{Cursor, Write as _};
    let mut out = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, body) in members {
            writer.start_file(name.as_str(), options).expect("a member");
            writer.write_all(body.as_bytes()).expect("its body");
        }
        writer.finish().expect("the archive");
    }
    out
}
