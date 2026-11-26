//! Lot CHEATS: a ROM finds its sheet in a database through the game database's No-Intro name.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;

use zax_app::cheat_database::CheatDatabase;
use zax_cheats::sheet::SheetMatch;

const MESEN: &str = r#"{"Cheats":[{"Description":"Infinite lives","Type":"NesCustom","Enabled":false,"Codes":"00A9:09"}]}"#;
const LIBRETRO: &str =
    "cheats = 1\ncheat0_desc = \"Big jump\"\ncheat0_code = \"GOSSIP\"\ncheat0_enable = false\n";

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("zaxnes-cheatdb-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_rom_finds_its_sheet_by_no_intro_name_in_a_zip() {
    let dir = folder("zip");
    let path = dir.join("cheats.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    for (name, text) in [
        ("Mega Man 3 (USA).json", MESEN),
        ("Mega Man 3 (Europe).json", MESEN),
        ("Super Mario Bros. (World).cht", LIBRETRO),
        ("Mario Adventure 3.json", MESEN),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
    let database = CheatDatabase::open(&path).unwrap();
    assert_eq!(database.len(), 4);

    // Mega Man 3 (USA), by its data CRC
    let found = database
        .sheet_for(Some(0x603A_AA57), "whatever")
        .unwrap()
        .unwrap();
    assert_eq!(
        (found.name.as_str(), found.matched),
        ("Mega Man 3 (USA)", SheetMatch::Exact)
    );
    assert_eq!(found.imported.cheats[0].description, "Infinite lives");

    // the USA Virtual Console image: same title, same region, another version
    let (name, matched) = database.sheet_name_for(Some(0x16B0_6537), "").unwrap();
    assert_eq!(
        (name.as_str(), matched),
        ("Mega Man 3 (USA)", SheetMatch::SameRegion)
    );

    // libretro's format
    let found = database.sheet_for(Some(0xD445_F698), "").unwrap().unwrap();
    assert_eq!(found.imported.cheats[0].description, "Big jump");

    // a hack the database does not know: its file name
    let (name, matched) = database.sheet_name_for(None, "Mario Adventure 3").unwrap();
    assert_eq!(
        (name.as_str(), matched),
        ("Mario Adventure 3", SheetMatch::FileName)
    );
    assert!(database.sheet_name_for(None, "Unknown Game").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_works_too() {
    let dir = folder("dir");
    std::fs::write(dir.join("Mega Man 3 (USA).json"), MESEN).unwrap();
    std::fs::write(dir.join("readme.txt"), "not a sheet").unwrap();
    let database = CheatDatabase::open(&dir).unwrap();
    assert_eq!(database.len(), 1);
    assert!(database.sheet_for(Some(0x603A_AA57), "").unwrap().is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every ROM of the game database against a real database:
/// `ZAXNES_CHEAT_DB=<Cheats.zip or folder> cargo test --test cheat_database_tests -- --ignored`.
#[test]
#[ignore = "needs ZAXNES_CHEAT_DB, a cheat database (zip or folder)"]
fn coverage_of_a_real_database() {
    let path = std::env::var_os("ZAXNES_CHEAT_DB").expect("set ZAXNES_CHEAT_DB");
    let database = CheatDatabase::open(std::path::Path::new(&path)).unwrap();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for rom in zax_app::gamedb::roms() {
        let key = database
            .sheet_name_for(Some(rom.data_crc), "")
            .map_or_else(|| "none".to_owned(), |(_, matched)| format!("{matched:?}"));
        *counts.entry(key).or_default() += 1;
    }
    println!("{} sheets: {counts:?}", database.len());
    assert!(counts.get("Exact").copied().unwrap_or(0) > 0);
}

/// The database built into the program (owner request, 2026-11-26) reads, and finds Mega Man 3
/// (USA) by its data CRC.
#[test]
fn the_built_in_database_finds_a_game_by_its_crc() {
    let database = zax_app::cheat_database::CheatDatabase::builtin().unwrap();
    assert!(database.len() > 1900, "{} sheets", database.len());
    let sheet = database.sheet_for(Some(0x603A_AA57), "mm3").unwrap().unwrap();
    assert_eq!(sheet.name, "Mega Man 3 (USA)");
    assert_eq!(sheet.matched, zax_cheats::sheet::SheetMatch::Exact);
    assert!(sheet.imported.cheats.len() > 1, "{} cheats", sheet.imported.cheats.len());
}
