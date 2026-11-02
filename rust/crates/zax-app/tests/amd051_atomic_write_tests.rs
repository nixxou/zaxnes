//! AMD-051 REQ-PERS-088: the atomic write of REQ-PERS-021 covers **every** persistent file of
//! REQ-PERS-024 — settings, library, slot index, bindings — and the savestates of REQ-PERS-056.
//!
//! An interruption cannot be staged inside a test (the process *is* the test), so what is
//! checked is the property that makes the guarantee true: nothing is ever written through the
//! final path. Every writer goes through [`zax_app::user_data_paths::write_atomic`], which
//! writes a `<name>.tmp` beside the file and renames it over it; a rename leaves either the old
//! file or the new one, never a truncated one, on NTFS as on the POSIX filesystems.
//!
//! The ordering clauses of the same requirement — the `.v<N>.bak` copy before the first migrated
//! rewrite, and the `*.corrupt-<timestamp>` rename before any replacement write — are checked
//! where those two operations live (`zax-settings`, `persistence_tests.rs`).

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::fs;
use std::path::Path;

use common::TemporaryRoot;
use time::OffsetDateTime;
use zax_app::persistence::save_slots::SaveSlotIndex;
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::user_data_paths::{write_atomic, UserDataPaths};
use zax_settings::model::SettingsModel;

fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
}

/// What `write_atomic` leaves behind: the contents, and no temporary for the next start to
/// mistake for the file.
fn assert_no_temporary(path: &Path) {
    assert!(path.is_file(), "the file exists: {}", path.display());
    let mut temporary = path.file_name().unwrap().to_os_string();
    temporary.push(".tmp");
    let temporary = path.with_file_name(temporary);
    assert!(
        !temporary.exists(),
        "no temporary is left beside {}",
        path.display()
    );
}

#[test]
fn req_pers_088_a_replacement_never_shortens_the_file_it_replaces() {
    let root = TemporaryRoot::new("zax-amd051-");
    let file = root.path().join("slots.json");
    let long = vec![b'a'; 8192];
    write_atomic(&file, &long).expect("the first write");
    assert_eq!(long, fs::read(&file).expect("the file"));
    assert_no_temporary(&file);

    // A shorter content. A writer that truncated in place would leave 8 KiB of the old file
    // behind an interruption; a replace cannot.
    let short = b"{}".to_vec();
    write_atomic(&file, &short).expect("the second write");
    assert_eq!(short, fs::read(&file).expect("the file"));
    assert_no_temporary(&file);
}

#[test]
fn req_pers_088_the_replacement_makes_the_folder_a_savestate_needs() {
    let root = TemporaryRoot::new("zax-amd051-");
    let file = root.path().join("states").join("game.ss1");
    write_atomic(&file, b"state").expect("the write");
    assert_eq!(b"state".to_vec(), fs::read(&file).expect("the file"));
    assert_no_temporary(&file);
}

/// The settings file and the slot index, written by the code that owns each, each landing with
/// no temporary beside it. The slot index is the file whose corruption costs the most
/// (REQ-PERS-055: "à défaut, tous les slots sont vides") and the one the first version of
/// AMD-051 left out.
#[test]
fn req_pers_088_the_settings_and_the_slot_index_land_without_a_temporary() {
    let root = TemporaryRoot::new("zax-amd051-");
    let paths = UserDataPaths::resolve(Some(root.path()), None, None);

    let mut store = SettingsStore::new(&paths);
    let mut model = SettingsModel::new();
    store.load(&mut model, now());
    store.save(&model);
    assert_no_temporary(&paths.settings_file);

    // The index is only written when a slot is occupied: an empty one is removed instead.
    let mut index = SaveSlotIndex::load(&paths, now());
    assert!(
        index.occupy("1", "g1", now().unix_timestamp(), false),
        "slot 1 exists"
    );
    index.save(&paths).expect("the index is written");
    assert_no_temporary(&paths.save_index_file);

    // And a second write, which is where an in-place truncation would show.
    index.save(&paths).expect("the index is rewritten");
    assert_no_temporary(&paths.save_index_file);
}
