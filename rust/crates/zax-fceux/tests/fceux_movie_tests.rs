//! `.fm2` playback through FCEUX's own movie mode (lot FCEUX-48 step 3, §5 of
//! `docs/review/FCEUX-ENGINE.md`).
//!
//! The whole file needs the `fceux-core` feature *and* a built fork, so it is all behind the
//! `cfg`: with the feature off there is nothing to test and nothing is compiled.
//!
//! **The movies are built here**, by hand, to the `.fm2` format the public format page describes
//! and `src/movie.cpp`'s own parser accepts — a version-3 text header and one `|commands|pad|pad||`
//! line a frame. That is deliberate: a movie recorded by FCEUX's recorder would need a second
//! patch of the fork (its recorder writes to a *path*, and this build has no filesystem), and a
//! hand-built movie is the stronger test anyway — it pins the format rather than a round trip
//! through our own writer.
//!
//! Its own process, so the one-FCEUX-per-process rule needs only the mutex below.

#![cfg(feature = "fceux-core")]
// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use zax_fceux::{Error, Fceux, MovieMode, RamInit, Region, FRAME_LEN};

/// Held for the length of every test, so only one holds a [`Fceux`] at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// `nestest.nes` of `rust/test-roms`, or `None` after saying what is missing. The ROMs are not in
/// the repository, so a checkout without them skips rather than fails.
fn nestest() -> Option<PathBuf> {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms/nestest.nes");
    if rom.is_file() {
        return Some(rom);
    }
    eprintln!("skipped: rust/test-roms/nestest.nes is missing");
    None
}

/// FNV-1a over a frame, as the rest of this crate's tests hash one.
fn digest(data: &[u8]) -> u64 {
    data.iter()
        .fold(14_695_981_039_346_656_037_u64, |hash, &b| {
            (hash ^ u64::from(b)).wrapping_mul(1_099_511_628_211)
        })
}

/// What goes in the header of a built movie. `None` leaves the key out altogether, which is how
/// a movie that states no ROM checksum is built.
#[allow(clippy::struct_excessive_bools)] // the `.fm2` header's own flags, key by key
struct Fm2 {
    pal: bool,
    ram_init_option: i32,
    ram_init_seed: i32,
    new_ppu: bool,
    fds: i32,
    fourscore: bool,
    microphone: bool,
    ports: [i32; 3],
    rom_checksum: Option<String>,
    /// One `(pad1, pad2)` a frame, in the bit order `zax_set_input` takes: A, B, Select, Start,
    /// Up, Down, Left, Right from bit 0.
    frames: Vec<(u8, u8)>,
}

impl Default for Fm2 {
    fn default() -> Self {
        Self {
            pal: false,
            // All `$00`, so that the direct run can be set to the same fill and the two machines
            // are comparable at all (REQ-CORE-004 is Zaxnes's rule; FCEUX's own default fill is
            // `00 00 00 00 FF FF FF FF`).
            ram_init_option: 2,
            ram_init_seed: 0,
            new_ppu: false,
            fds: 0,
            fourscore: false,
            microphone: false,
            // SI_GAMEPAD, SI_GAMEPAD, SIFC_NONE (`src/git.h`).
            ports: [1, 1, 0],
            rom_checksum: None,
            frames: Vec::new(),
        }
    }
}

impl Fm2 {
    /// The bytes of the movie.
    fn build(&self) -> Vec<u8> {
        // `LoadFM2` refuses anything whose first nine bytes are not "version 3", so that line is
        // first and exactly that.
        let mut lines = vec![
            "version 3".to_owned(),
            "emuVersion 20000".to_owned(),
            "rerecordCount 7".to_owned(),
            format!("palFlag {}", i32::from(self.pal)),
            "romFilename nestest".to_owned(),
        ];
        if let Some(checksum) = &self.rom_checksum {
            lines.push(format!("romChecksum {checksum}"));
        }
        lines.extend([
            "guid 00000000-0000-0000-0000-000000000000".to_owned(),
            format!("fourscore {}", i32::from(self.fourscore)),
            format!("microphone {}", i32::from(self.microphone)),
            format!("port0 {}", self.ports[0]),
            format!("port1 {}", self.ports[1]),
            format!("port2 {}", self.ports[2]),
            format!("FDS {}", self.fds),
            format!("NewPPU {}", i32::from(self.new_ppu)),
            format!("RAMInitOption {}", self.ram_init_option),
            format!("RAMInitSeed {}", self.ram_init_seed),
            "comment author zax-fceux tests".to_owned(),
        ]);

        lines.extend(
            self.frames
                .iter()
                .map(|(pad1, pad2)| format!("|0|{}|{}||", mnemonics(*pad1), mnemonics(*pad2))),
        );

        let mut text = lines.join("\n");
        text.push('\n');
        text.into_bytes()
    }
}

/// One pad as a `.fm2` frame field: `MovieRecord::dumpJoy` writes bit 7 first and
/// `MovieRecord::mnemonics` is `A B S T U D L R` indexed by bit, so the eight characters read
/// `RLDUTSBA` with `.` for a button that is up.
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

/// The input sequence both runs of the parity test use: nothing for a while, then Start, then
/// some of everything, so that the comparison is of a machine that was actually driven.
fn scripted_input(frames: usize) -> Vec<(u8, u8)> {
    (0..frames)
        .map(|frame| {
            let pad1 = match frame {
                0..=29 => 0x00,
                30..=33 => 0x08, // Start
                _ => u8::try_from(frame % 251).unwrap_or(0) & 0xF7,
            };
            // The second pad is driven too, so that a movie replaying only port 0 would fail.
            let pad2 = u8::from(frame % 7 == 0);
            (pad1, pad2)
        })
        .collect()
}

/// A loaded nestest with a known RAM fill, ready to be driven or to be given a movie.
fn loaded(fceux: &mut Fceux, rom: &Path, ram: RamInit) {
    let image = std::fs::read(rom).expect("the ROM file");
    fceux.set_ram_init(ram).expect("the RAM fill");
    fceux
        .load_rom(&image, "nestest.nes")
        .expect("nestest loads on FCEUX");
}

/// §5's whole point, and the brief's own check: **a movie plays back to the same frame digests as
/// feeding the same inputs directly**. If the two differed, the movie mode would be replaying a
/// different machine and every figure taken under it would be worthless.
#[test]
fn a_movie_plays_back_to_the_same_frames_as_the_same_inputs_fed_directly() {
    const FRAMES: usize = 240;
    const LENGTH: u32 = 240;

    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let script = scripted_input(FRAMES);

    // The direct run: the same RAM fill the movie's header asks for, a power-on, then the inputs.
    let direct = {
        let mut fceux = Fceux::new().expect("an instance");
        loaded(&mut fceux, &rom, RamInit::Zero);
        fceux.power().expect("power on");

        let mut picture = vec![0_u8; FRAME_LEN];
        let mut digests = Vec::with_capacity(FRAMES);
        for (pad1, pad2) in &script {
            fceux.set_input(*pad1, *pad2).expect("the pads");
            fceux.run_frame().expect("a frame");
            fceux.frame_indices_into(&mut picture).expect("the picture");
            digests.push(digest(&picture));
        }
        digests
    };

    // The movie run: the same inputs, through FCEUX's own movie mode. The pads are deliberately
    // left on a value that is in no frame of the script, so that a movie which failed to override
    // them could not accidentally agree.
    let movie = Fm2 {
        frames: script.clone(),
        ..Fm2::default()
    }
    .build();

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Ff);
    let header = fceux.play_movie(&movie).expect("the movie plays");
    assert_eq!(header.frames, LENGTH);
    assert_eq!(header.ram_init, RamInit::Zero);
    assert_eq!(fceux.movie_mode(), MovieMode::Playing);
    assert_eq!(fceux.movie_position(), Some((LENGTH, 0)));

    let mut picture = vec![0_u8; FRAME_LEN];
    let mut played = Vec::with_capacity(FRAMES);
    for _ in 0..FRAMES {
        fceux.set_input(0x55, 0xAA).expect("the pads");
        fceux.run_frame().expect("a frame");
        fceux.frame_indices_into(&mut picture).expect("the picture");
        played.push(digest(&picture));
    }

    assert_eq!(played.len(), direct.len());
    let first_difference = played
        .iter()
        .zip(&direct)
        .position(|(played, direct)| played != direct);
    assert_eq!(
        first_difference, None,
        "the movie and the same inputs fed directly part at frame {first_difference:?}"
    );

    // And the movie is spent: FCEUX's own clock reached the end. The *mode* still says Playing,
    // because FinishPlayback runs at the top of a frame rather than the bottom of the last one
    // (movie.cpp:1245-1250) -- which is why the clock, and not the mode, is the signal a host
    // should use. One more frame flips it.
    assert_eq!(fceux.movie_position(), Some((LENGTH, LENGTH)));
    assert_eq!(fceux.movie_mode(), MovieMode::Playing);
    fceux.run_frame().expect("a frame");
    assert_eq!(fceux.movie_mode(), MovieMode::Finished);
}

/// The RAM fill of a movie's header is honoured — the reason playback has to go through FCEUX's
/// movie mode at all (§5: `PowerNES` reseeds the random fill from `rand()` on every power-on
/// *unless a movie is active*). Two movies that differ only in `RAMInitOption` must leave
/// different work RAM, whatever the engine was set to before.
#[test]
fn the_ram_init_option_of_the_header_is_honoured() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    for (option, expected) in [(1_i32, 0xFF_u8), (2, 0x00)] {
        let movie = Fm2 {
            ram_init_option: option,
            frames: vec![(0, 0); 4],
            ..Fm2::default()
        }
        .build();

        let mut fceux = Fceux::new().expect("an instance");
        // Deliberately the *other* fill before the movie, so that a movie whose header were
        // ignored would read the wrong byte back.
        loaded(
            &mut fceux,
            &rom,
            if option == 1 {
                RamInit::Zero
            } else {
                RamInit::Ff
            },
        );
        let header = fceux.play_movie(&movie).expect("the movie plays");
        assert_eq!(
            header.ram_init,
            if option == 1 {
                RamInit::Ff
            } else {
                RamInit::Zero
            },
        );

        // Read before a single frame has run, so this is the fill and not the game's own writes.
        for address in [0x0010_u16, 0x0100, 0x0400, 0x07FF] {
            assert_eq!(
                fceux.read_memory(address),
                Ok(expected),
                "RAMInitOption {option} at ${address:04X}",
            );
        }
    }
}

/// A seeded random fill is the case §5 says can only work from inside movie mode. Two movies with
/// the same seed must give the same work RAM; two with different seeds must not.
#[test]
fn a_seeded_random_fill_is_reproducible_from_inside_a_movie() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let fill_of = |seed: i32| {
        let movie = Fm2 {
            ram_init_option: 3,
            ram_init_seed: seed,
            frames: vec![(0, 0); 2],
            ..Fm2::default()
        }
        .build();

        let mut fceux = Fceux::new().expect("an instance");
        loaded(&mut fceux, &rom, RamInit::Zero);
        let header = fceux.play_movie(&movie).expect("the movie plays");
        assert_eq!(header.ram_init, RamInit::Random(seed));
        (0_u16..0x0800)
            .map(|address| fceux.read_memory(address).expect("work RAM reads"))
            .collect::<Vec<u8>>()
    };

    let first = fill_of(12345);
    let again = fill_of(12345);
    let other = fill_of(999);

    assert_eq!(first, again, "the same seed must give the same work RAM");
    assert_ne!(first, other, "a different seed must give different RAM");
    assert!(
        first.iter().any(|byte| *byte != 0) && first.iter().any(|byte| *byte != 0xFF),
        "a random fill is neither all zero nor all $FF",
    );
}

/// The header is read **without touching the machine**: a movie can be inspected while another
/// one plays, and nothing moves.
#[test]
fn the_header_is_read_without_touching_the_machine() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Zero);
    fceux.power().expect("power on");
    for _ in 0..30 {
        fceux.run_frame().expect("a frame");
    }

    let mut before = vec![0_u8; FRAME_LEN];
    fceux.frame_indices_into(&mut before).expect("the picture");
    let frames_before = fceux.frame_count();

    let movie = Fm2 {
        pal: true,
        ram_init_option: 1,
        frames: vec![(0x10, 0), (0x20, 0), (0x30, 0)],
        rom_checksum: Some("0x000102030405060708090A0B0C0D0E0F".to_owned()),
        ..Fm2::default()
    }
    .build();

    let header = fceux.inspect_movie(&movie).expect("the header");
    assert_eq!(header.version, 3);
    assert_eq!(header.frames, 3);
    assert_eq!(header.region, Region::Pal);
    assert_eq!(header.ram_init, RamInit::Ff);
    assert_eq!(header.ports, [1, 1, 0]);
    assert!(!header.from_savestate && !header.from_saveram);
    assert_eq!(header.rerecord_count, 7);
    assert_eq!(
        header.rom_md5,
        Some([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
    );
    assert_eq!(
        header.rom_matches,
        Some(false),
        "that checksum is not nestest's",
    );

    // Nothing moved: no movie is loaded, the frame counter is where it was, and the picture is
    // the same one.
    assert_eq!(fceux.movie_mode(), MovieMode::None);
    assert_eq!(fceux.movie_position(), None);
    assert_eq!(fceux.frame_count(), frames_before);
    let mut after = vec![0_u8; FRAME_LEN];
    fceux.frame_indices_into(&mut after).expect("the picture");
    assert_eq!(digest(&before), digest(&after));
}

/// A movie whose header asks for something this build has not is **refused with the reason named,
/// not played wrong** — and the machine is left exactly as it was, which matters because
/// `FCEUI_LoadMovie` re-powers the machine as its third act.
#[test]
fn a_movie_the_build_cannot_play_is_refused_by_name_and_changes_nothing() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Zero);
    fceux.power().expect("power on");
    for _ in 0..20 {
        fceux.run_frame().expect("a frame");
    }
    let mut before = vec![0_u8; FRAME_LEN];
    fceux.frame_indices_into(&mut before).expect("the picture");
    let frames_before = fceux.frame_count();

    let cases: [(Fm2, &str); 6] = [
        (
            Fm2 {
                fds: 1,
                frames: vec![(0, 0); 2],
                ..Fm2::default()
            },
            "a Famicom Disk System disk",
        ),
        (
            Fm2 {
                new_ppu: true,
                frames: vec![(0, 0); 2],
                ..Fm2::default()
            },
            "FCEUX's new PPU",
        ),
        (
            Fm2 {
                fourscore: true,
                frames: vec![(0, 0); 2],
                ..Fm2::default()
            },
            "a Four Score (four pads)",
        ),
        (
            Fm2 {
                microphone: true,
                frames: vec![(0, 0); 2],
                ..Fm2::default()
            },
            "the Famicom microphone",
        ),
        (
            Fm2 {
                // SI_ZAPPER on port 1.
                ports: [1, 2, 0],
                frames: vec![(0, 0); 2],
                ..Fm2::default()
            },
            "an input device other than a standard pad",
        ),
        (
            Fm2 {
                // SIFC_FKB, the Family keyboard, on the expansion port.
                ports: [1, 1, 4],
                frames: vec![(0, 0); 2],
                ..Fm2::default()
            },
            "an expansion-port device",
        ),
    ];

    for (movie, why) in cases {
        let bytes = movie.build();
        // The header itself names the thing, which is what lets a host say which.
        let header = fceux.inspect_movie(&bytes).expect("the header");
        assert_eq!(
            header.refusal(),
            Some(Error::MovieUnsupported(why)),
            "the header should refuse: {why}",
        );
        // And asking for it anyway gives the same answer with the reason in the message.
        let refused = fceux.play_movie(&bytes).expect_err("it must be refused");
        assert_eq!(refused, Error::MovieUnsupported(why));
        assert!(refused.to_string().contains(why), "{refused}");
    }

    // Nothing was committed by any of the six: no movie, the same frame, the same picture.
    assert_eq!(fceux.movie_mode(), MovieMode::None);
    assert_eq!(fceux.frame_count(), frames_before);
    let mut after = vec![0_u8; FRAME_LEN];
    fceux.frame_indices_into(&mut after).expect("the picture");
    assert_eq!(digest(&before), digest(&after));
}

/// A movie recorded on another ROM is refused: it states the checksum and FCEUX knows the
/// cartridge's, so playing it would be playing the wrong game's inputs.
#[test]
fn a_movie_recorded_on_another_rom_is_refused() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Zero);

    let wrong = Fm2 {
        rom_checksum: Some("0xFFEEDDCCBBAA99887766554433221100".to_owned()),
        frames: vec![(0, 0); 4],
        ..Fm2::default()
    }
    .build();
    assert_eq!(
        fceux.play_movie(&wrong).err(),
        Some(Error::MovieRom),
        "a movie of another ROM must be refused",
    );
    assert_eq!(fceux.movie_mode(), MovieMode::None);

    // The same movie without a stated checksum is played: a hand-written movie that claims
    // nothing is not claiming the wrong thing.
    let silent = Fm2 {
        frames: vec![(0, 0); 4],
        ..Fm2::default()
    }
    .build();
    let header = fceux.play_movie(&silent).expect("it plays");
    assert_eq!(header.rom_md5, None);
    assert_eq!(header.rom_matches, None);
    assert_eq!(fceux.movie_mode(), MovieMode::Playing);
}

/// What is not a `.fm2` at all, and what needs a cartridge.
#[test]
fn rubbish_is_refused_as_a_format_error_and_a_movie_needs_a_rom() {
    let _serial = lock();

    let mut fceux = Fceux::new().expect("an instance");
    let good = Fm2 {
        frames: vec![(0, 0); 2],
        ..Fm2::default()
    }
    .build();

    // No ROM: a movie plays a cartridge, not nothing — and the format is checked first, which is
    // why the empty case below is a format error rather than this one.
    assert_eq!(fceux.play_movie(&good).err(), Some(Error::NoRom));

    for rubbish in [
        b"".as_slice(),
        b"not a movie at all".as_slice(),
        b"version 2\nport0 1\n|0|........|........||\n".as_slice(),
        // The header of a real movie, truncated inside the magic.
        b"version ".as_slice(),
    ] {
        assert_eq!(
            fceux.inspect_movie(rubbish).err(),
            Some(Error::MovieFormat),
            "{:?} is not a version-3 .fm2",
            String::from_utf8_lossy(rubbish),
        );
    }

    let Some(rom) = nestest() else { return };
    loaded(&mut fceux, &rom, RamInit::Zero);
    assert_eq!(
        fceux.play_movie(b"version 2\n").err(),
        Some(Error::MovieFormat),
    );
    assert_eq!(fceux.movie_mode(), MovieMode::None);
}

/// Stopping: the movie is unloaded, the machine goes on from where it is, and stopping twice is
/// not an error. And once stopped, the host's own pads are live again.
#[test]
fn stopping_unloads_the_movie_and_gives_the_pads_back() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Zero);

    let movie = Fm2 {
        frames: scripted_input(60),
        ..Fm2::default()
    }
    .build();
    fceux.play_movie(&movie).expect("it plays");
    for _ in 0..30 {
        fceux.run_frame().expect("a frame");
    }
    assert_eq!(fceux.movie_position(), Some((60, 30)));

    fceux.stop_movie().expect("it stops");
    assert_eq!(fceux.movie_mode(), MovieMode::None);
    assert_eq!(fceux.movie_position(), None);
    fceux.stop_movie().expect("stopping twice is harmless");

    // The machine is still running where the movie left it, and it is ours again.
    let frames_before = fceux.frame_count();
    fceux.set_input(0x08, 0).expect("the pads");
    fceux.run_frame().expect("a frame");
    assert_eq!(fceux.frame_count(), frames_before + 1);
    assert_eq!(fceux.movie_mode(), MovieMode::None);
}

/// The region of the header is applied, which is the other thing §5 says the header decides: a
/// PAL movie must not be replayed at NTSC timing.
#[test]
fn the_region_of_the_header_is_applied() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Zero);
    fceux.set_region(Region::Ntsc).expect("NTSC");

    let movie = Fm2 {
        pal: true,
        frames: vec![(0, 0); 4],
        ..Fm2::default()
    }
    .build();
    let header = fceux.play_movie(&movie).expect("it plays");
    assert_eq!(header.region, Region::Pal);
    assert_eq!(
        fceux.region(),
        Ok(Region::Pal),
        "the movie's own region must be in force while it plays",
    );

    // And an NTSC movie after it brings the machine back, rather than leaving the last one's.
    let ntsc = Fm2 {
        pal: false,
        frames: vec![(0, 0); 4],
        ..Fm2::default()
    }
    .build();
    fceux.play_movie(&ntsc).expect("it plays");
    assert_eq!(fceux.region(), Ok(Region::Ntsc));
}

/// Playback runs out rather than running on: past the last record FCEUX reports `Finished`, the
/// frame counter stops at the length, and running further frames is allowed (the machine keeps
/// going, which is FCEUX's own behaviour) without the clock moving.
#[test]
fn playback_finishes_at_the_last_record_and_says_so() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Zero);

    let movie = Fm2 {
        frames: vec![(0, 0); 5],
        ..Fm2::default()
    }
    .build();
    fceux.play_movie(&movie).expect("it plays");

    for frame in 0..5 {
        assert_eq!(fceux.movie_position(), Some((5, frame)));
        assert_eq!(fceux.movie_mode(), MovieMode::Playing);
        fceux.run_frame().expect("a frame");
    }

    // The clock is the signal that does not lag: it is at the length, and the movie is over.
    assert_eq!(fceux.movie_position(), Some((5, 5)));
    // The *mode* lags by one frame, FCEUX deciding at the top of a frame rather than the bottom
    // of the last one (movie.cpp:1245-1250). Both halves of that are pinned here, because a host
    // that used the mode alone would show a movie as playing after it had ended.
    assert_eq!(fceux.movie_mode(), MovieMode::Playing);

    // Two more frames: the machine runs, the mode catches up, and the movie's clock goes **on
    // counting past the end** -- `currFrameCounter++` at movie.cpp:1329 is unconditional while a
    // movie is loaded. So "the movie is over" is `frame >= length`, not `frame == length`, and a
    // host that subtracted them would get a negative remaining count.
    let frames_before = fceux.frame_count();
    fceux.run_frame().expect("a frame");
    assert_eq!(fceux.movie_mode(), MovieMode::Finished);
    fceux.run_frame().expect("a frame");
    assert_eq!(fceux.frame_count(), frames_before + 2);
    assert_eq!(fceux.movie_position(), Some((5, 7)));
    assert_eq!(fceux.movie_mode(), MovieMode::Finished);
}

/// A savestate taken during playback and restored brings the machine back **and** the movie's own
/// clock with it — which is what makes seeking backwards by checkpoint possible at all
/// (step 4's requirement, measured here where the seam is).
#[test]
fn a_savestate_during_playback_carries_the_movies_clock() {
    let _serial = lock();
    let Some(rom) = nestest() else { return };

    let mut fceux = Fceux::new().expect("an instance");
    loaded(&mut fceux, &rom, RamInit::Zero);

    let script = scripted_input(120);
    let movie = Fm2 {
        frames: script.clone(),
        ..Fm2::default()
    }
    .build();
    fceux.play_movie(&movie).expect("it plays");

    for _ in 0..40 {
        fceux.run_frame().expect("a frame");
    }
    let mut state = Vec::new();
    fceux.save_state_into(&mut state).expect("a state");
    assert_eq!(fceux.movie_position(), Some((120, 40)));

    // On to frame 100, hashing what the picture becomes.
    let mut picture = vec![0_u8; FRAME_LEN];
    let mut expected = Vec::new();
    for _ in 40..100 {
        fceux.run_frame().expect("a frame");
        fceux.frame_indices_into(&mut picture).expect("the picture");
        expected.push(digest(&picture));
    }
    assert_eq!(fceux.movie_position(), Some((120, 100)));

    // Back to the checkpoint, and forward again: the same pictures, frame for frame.
    fceux.load_state(&state).expect("the state loads");
    assert_eq!(
        fceux.movie_position(),
        Some((120, 40)),
        "the movie's clock must come back with the state",
    );
    let mut again = Vec::new();
    for _ in 40..100 {
        fceux.run_frame().expect("a frame");
        fceux.frame_indices_into(&mut picture).expect("the picture");
        again.push(digest(&picture));
    }
    assert_eq!(again, expected, "a seek back must replay identically");
    assert_eq!(fceux.movie_mode(), MovieMode::Playing);
}
