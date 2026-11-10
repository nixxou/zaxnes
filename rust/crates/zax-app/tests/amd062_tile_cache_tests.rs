//! The resolved-tile cache of the "3dh" profiles as the application keeps it (AMD-062
//! REQ-PROF-082): one file per profile identifier in the user data folder, and the
//! `--no-tile-cache` option that keeps nothing.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use std::fs;

use zax_app::app_options::AppOptions;
use zax_app::persistence::tile_cache_store::TileCacheStore;
use zax_app::user_data_paths::UserDataPaths;
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::tile_cache;
use zax_profiles::hash::tile_id::TileId;

mod common;

use common::TemporaryRoot;

fn raw(seed: u8) -> RawTile {
    let mut bytes = [0_u8; raw_tile::BYTE_COUNT];
    for (index, byte) in bytes.iter_mut().enumerate() {
        #[allow(clippy::cast_possible_truncation)] // A test pattern.
        {
            *byte = seed.wrapping_add(index as u8);
        }
    }
    bytes
}

/// A cache text holding the two tiles, as the resolver writes it.
fn cache_of(tiles: &[RawTile]) -> String {
    let keys: Vec<String> = tiles
        .iter()
        .map(|raw| TileId::compute(raw).to_text())
        .collect();
    tile_cache::write(keys.iter().map(String::as_str).zip(tiles))
}

fn paths_in(directory: &std::path::Path) -> UserDataPaths {
    UserDataPaths::resolve(Some(directory), Some(directory), None)
}

// REQ-PROF-082: the cache of a profile is a file of the user data folder, read at the launch and
// written when the game stops; every entry recomputes from its bytes.
#[test]
fn req_prof_082_the_cache_is_kept_per_profile_identifier() {
    let directory = TemporaryRoot::new("tile-cache");
    let paths = paths_in(directory.path());
    let store = TileCacheStore::new(true);
    let tiles = [raw(1), raw(100)];

    assert_eq!("", store.read(&paths, "Game1"));
    assert!(store.write(&paths, "Game1", &cache_of(&tiles)));

    let file = paths.tile_cache_file("Game1");
    assert!(file.is_file());
    assert_eq!(
        paths.tiles_directory.join("Game1.tiles"),
        file,
        "one file per profile identifier"
    );

    let kept = tile_cache::read(&store.read(&paths, "Game1"));
    assert_eq!(2, kept.len());
    for (key, raw) in &kept {
        assert_eq!(TileId::compute(raw).to_text(), *key);
    }

    // Another profile has its own file, and an identifier that is not a usable file name is made
    // into one (the derived identifier of AMD-037 is base64).
    assert!(store.write(&paths, "ab/cd+ef", &cache_of(&[raw(7)])));
    assert_eq!(1, tile_cache::read(&store.read(&paths, "ab/cd+ef")).len());
    assert_eq!(2, tile_cache::read(&store.read(&paths, "Game1")).len());
}

// REQ-PROF-082: an unreadable, truncated or arbitrary cache is ignored without an error, and a
// profile that resolved nothing leaves no file behind.
#[test]
fn req_prof_082_a_damaged_cache_is_ignored_and_an_empty_one_is_not_written() {
    let directory = TemporaryRoot::new("tile-cache-damaged");
    let paths = paths_in(directory.path());
    let store = TileCacheStore::new(true);

    // Nothing to keep: no file.
    assert!(!store.write(&paths, "Game1", &tile_cache::write(Vec::new())));
    assert!(!paths.tile_cache_file("Game1").is_file());

    // Arbitrary content, and a truncated file: both read as nothing kept.
    fs::create_dir_all(&paths.tiles_directory).unwrap();
    fs::write(paths.tile_cache_file("Game2"), b"not a cache at all").unwrap();
    assert_eq!(0, tile_cache::read(&store.read(&paths, "Game2")).len());

    let text = cache_of(&[raw(1), raw(100)]);
    let truncated = &text[..text.len() / 3];
    fs::write(paths.tile_cache_file("Game3"), truncated).unwrap();
    assert!(tile_cache::read(&store.read(&paths, "Game3")).len() < 2);

    // An entry whose string does not recompute from its bytes is dropped.
    let wrong = format!(
        "{}\n{} {}\n",
        tile_cache::write(Vec::new()).trim(),
        TileId::compute(&raw(1)).to_text(),
        raw_tile::hex_lowercase(&raw(2))
    );
    fs::write(paths.tile_cache_file("Game4"), &wrong).unwrap();
    assert_eq!(0, tile_cache::read(&store.read(&paths, "Game4")).len());
}

// REQ-PROF-082 and REQ-PROF-080: `--no-tile-cache` reads nothing and writes nothing, so every
// run starts from an empty resolved set — which is what determinism is stated at.
#[test]
fn req_prof_082_no_tile_cache_keeps_nothing() {
    let directory = TemporaryRoot::new("tile-cache-off");
    let paths = paths_in(directory.path());
    let store = TileCacheStore::new(false);
    assert!(!store.enabled());

    assert!(!store.write(&paths, "Game1", &cache_of(&[raw(1)])));
    assert!(!paths.tile_cache_file("Game1").is_file());

    // A file left by an earlier run is not read either.
    fs::create_dir_all(&paths.tiles_directory).unwrap();
    fs::write(paths.tile_cache_file("Game1"), cache_of(&[raw(1)])).unwrap();
    assert_eq!("", store.read(&paths, "Game1"));

    let options = AppOptions::parse(&["--no-tile-cache"]).unwrap();
    assert!(options.no_tile_cache);
    assert!(!AppOptions::parse(&[]).unwrap().no_tile_cache);
}
