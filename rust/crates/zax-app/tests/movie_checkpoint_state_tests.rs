//! LUA-MERGE step 3: the editor's checkpoints against `dev`'s save-states.
//!
//! `lua-editor` was branched before the mapper review, the MMC5 split and the core timing lots
//! (MF-4a..MF-4e). The movie explorer takes a checkpoint with `NesCore::save_state`, keeps it in
//! memory, and later restores it with `try_load_state` into a session that was **built again**
//! from the same ROM (`host/script_reload.rs`) — so a board whose state grew or is keyed
//! differently on `dev` would make the explorer restore a core that is not the one it saved.
//!
//! `fm2_movie_tests` already checks the invariant end to end, but on a synthetic NROM image.
//! These tests check it on the real boards of the demo ROMs, which is where the mapper review
//! actually moved things.
//!
//! **What is compared, and why not the digest straight after the restore.** The picture is
//! deliberately *not* part of `CoreState` — REQ-CORE-121 leaves the frame the host has not
//! finished reading where it is, and `state.rs` names no pixel buffer on either branch — while
//! [`machine_digest`] does read it. The digest is therefore only meaningful between two cores
//! that have each *run* the frame being compared, which is exactly how the explorer uses it: it
//! matches its own explored state against the session's checkpoint of the same frame, both
//! reached by running. So these tests compare the frame count, the cycle count and the work RAM
//! straight after the restore, and the digest only once both cores have run.
//!
//! The ROMs are the owner's and are not in the repository: without `ZAXNES_ROMS` each test
//! returns, as the other ROM-fed tests of this crate do.

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use zax_app::emulation::engine_handle::EngineHandle;
use zax_app::emulation::movie_timeline::{machine_digest, CHECKPOINT_INTERVAL};

/// The demo ROMs, or `None` when `ZAXNES_ROMS` is not set.
fn roms_folder() -> Option<PathBuf> {
    let folder = PathBuf::from(std::env::var_os("ZAXNES_ROMS")?);
    folder.is_dir().then_some(folder)
}

/// A core with `file` of the demo folder loaded, or `None` when that ROM is not there.
fn core_of(folder: &Path, file: &str) -> Option<EngineHandle> {
    let image = std::fs::read(folder.join(file)).ok()?;
    let mut core = EngineHandle::zaxnes_core();
    let load = core.load_rom(&image, file);
    assert!(load.success, "{file}: {:?}", load.error);
    Some(core)
}

/// The boards of the demo ROMs, one game each: the mapper number is what the review touched.
const GAMES: &[(&str, &str)] = &[
    ("Super Mario Bros. (World).nes", "NROM (0)"),
    ("Excitebike (Japan, USA) (En).nes", "NROM (0)"),
    ("Final Fantasy (USA).nes", "MMC1 (1)"),
    ("Legend of Zelda, The (USA).nes", "MMC1 (1), battery"),
    ("Zelda II - The Adventure of Link (USA).nes", "MMC1 (1)"),
    ("Metroid (USA).nes", "MMC1 (1)"),
    ("Castlevania [USA][!!].nes", "UNROM (2)"),
    ("Contra (USA).nes", "UNROM (2)"),
    ("Mega Man 2 (USA).nes", "MMC1 (1)"),
    ("Super Mario Bros. 3 (USA).nes", "MMC3 (4)"),
    ("Blaster Master (USA).nes", "MMC1 (1)"),
    ("Adventure Island (USA).nes", "MMC1 (1)"),
    ("Tetris (USA).nes", "NROM (0)"),
    ("Elevator Action (USA).nes", "NROM (0)"),
    ("Ninja Gaiden III-TASoD [USA][!].nes", "MMC3 (4)"),
];

/// A checkpoint restored into a core built again from the same ROM is the same machine, and
/// stays the same machine as it runs on.
///
/// This is the movie explorer's whole premise (`movie_explorer.rs`, `movie_timeline.rs`) and the
/// thing `dev`'s larger board state could have broken.
#[test]
fn a_checkpoint_restores_into_a_fresh_core_on_every_demo_board() {
    let Some(folder) = roms_folder() else {
        return;
    };
    let mut checked = 0;
    for (file, board) in GAMES {
        let Some(mut played) = core_of(&folder, file) else {
            continue;
        };
        // Past the warm-up, then to a frame count a checkpoint would really fall on.
        for _ in 0..CHECKPOINT_INTERVAL * 2 {
            played.run_frame(0);
        }
        let block = played.save_state(true).expect("a loaded ROM");

        // What the script reload does: a core built again from the ROM, then the block.
        let mut restored = core_of(&folder, file).unwrap();
        restored
            .try_load_state(&block, 0)
            .unwrap_or_else(|error| panic!("{file} ({board}): the checkpoint refused: {error}"));

        // Everything the state does carry: the counts and the work RAM. The picture is not in
        // it (see the module's note), so it is left out of this first comparison.
        assert_eq!(
            played.frame_count(),
            restored.frame_count(),
            "{file} ({board}): the restored checkpoint is at another frame"
        );
        assert_eq!(
            played.cycle_count(),
            restored.cycle_count(),
            "{file} ({board}): the restored checkpoint is at another cycle"
        );
        let ram_of = |core: &EngineHandle| {
            (0..0x0800u16)
                .map(|address| core.read_memory(address))
                .collect::<Vec<_>>()
        };
        assert!(
            ram_of(&played) == ram_of(&restored),
            "{file} ({board}): the restored checkpoint has other work RAM"
        );

        // A state that restored only partly agrees on all that and still diverges as it runs:
        // the mapper's banks, the PPU's pipeline and the APU are only visible in motion. Both
        // cores have run the frame by now, so the digest's picture is in the comparison.
        for frame in 0..CHECKPOINT_INTERVAL * 2 {
            played.run_frame(0);
            restored.run_frame(0);
            assert_eq!(
                machine_digest(&played),
                machine_digest(&restored),
                "{file} ({board}): the restored core diverged {} frames after the checkpoint",
                frame + 1
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 10,
        "only {checked} of the demo ROMs were found: the check is not worth its name"
    );
}

/// The digest tells two different frames apart, so that the test above cannot pass by comparing
/// a constant. `machine_digest` reads the picture through `pixel_frame_with_mask` on `dev`
/// (it was `pixel_frame` on `lua-editor`); this is what says the picture is still in it.
#[test]
fn the_digest_separates_frames_and_sees_the_picture() {
    let Some(folder) = roms_folder() else {
        return;
    };
    let Some(mut core) = core_of(&folder, "Super Mario Bros. (World).nes") else {
        return;
    };
    let mut digests = std::collections::HashSet::new();
    for _ in 0..90 {
        core.run_frame(0);
        digests.insert(machine_digest(&core));
    }
    assert!(
        digests.len() > 80,
        "the digest repeated too often over 90 frames ({} distinct): it is not reading the \
         machine",
        digests.len()
    );
}
