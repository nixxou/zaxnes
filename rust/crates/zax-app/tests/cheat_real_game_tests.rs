//! Lot CHEATS on a real game (ignored: needs `ZAXNES_ROMS` with `Super Mario Bros. (World).nes`):
//! the built-in database's sheet found by the data CRC, a Game Genie code of it and a RAM code
//! through the engine handle the application runs, on Zaxnes and (with `fceux-core`) on FCEUX.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use zax_app::cheat_database::CheatDatabase;
use zax_app::emulation::engine_handle::EngineHandle;
use zax_cheats::{Cheat, CheatCode, CheatList};
use zax_core_api::EngineKind;

const LIVES: u16 = 0x075A;
const START: u16 = 1 << 3;

fn rom() -> Option<Vec<u8>> {
    let folder = std::env::var_os("ZAXNES_ROMS")?;
    std::fs::read(std::path::Path::new(&folder).join("Super Mario Bros. (World).nes")).ok()
}

/// The lives (minus one) once a game has started: Start pressed on the title screen.
fn lives(kind: EngineKind, image: &[u8], cheats: &CheatList) -> u8 {
    let mut handle = EngineHandle::of_kind(kind);
    assert!(handle.load_rom(image, "Super Mario Bros. (World).nes").success);
    assert!(handle.set_cheats(cheats), "{kind:?} refused the Game Genie codes");
    for frame in 0..300 {
        let word = if (60..66).contains(&frame) { START } else { 0 };
        handle.run_frame(word);
    }
    handle.engine().read_memory(LIVES)
}

fn engines() -> Vec<EngineKind> {
    let mut kinds = vec![EngineKind::Zaxnes, EngineKind::NesHawk];
    if cfg!(feature = "fceux-core") {
        kinds.push(EngineKind::Fceux);
    }
    kinds
}

#[test]
#[ignore = "needs ZAXNES_ROMS with Super Mario Bros. (World).nes"]
fn smb_cheats_of_the_built_in_sheet_work_on_every_engine() {
    let Some(image) = rom() else {
        panic!("Super Mario Bros. (World).nes not found under ZAXNES_ROMS");
    };
    let crc = zax_nes::builtin_patches::data_crc(&image);
    let database = CheatDatabase::builtin().unwrap();
    let sheet = database.sheet_for(Some(crc), "smb").unwrap().expect("a sheet");
    eprintln!("sheet {} ({:?}), {} cheats", sheet.name, sheet.matched, sheet.imported.cheats.len());
    let mut one_life = sheet
        .imported
        .cheats
        .iter()
        .find(|cheat| cheat.description.starts_with("Both Players Start With One Life"))
        .cloned()
        .expect("the code in the sheet");
    one_life.enabled = true;
    let genie = CheatList { cheats: vec![one_life] };
    let ram = CheatList {
        cheats: vec![Cheat {
            description: "9 lives".to_owned(),
            codes: CheatCode::parse_list("075A:08").unwrap(),
            enabled: true,
        }],
    };
    for kind in engines() {
        let plain = lives(kind, &image, &CheatList::default());
        let with_genie = lives(kind, &image, &genie);
        let with_ram = lives(kind, &image, &ram);
        eprintln!("{kind:?}: $075A plain {plain}, Game Genie {with_genie}, RAM code {with_ram}");
        assert_eq!(plain, 2, "{kind:?}: three lives without a cheat");
        assert_eq!(with_genie, 0, "{kind:?}: one life with EATOLE");
        assert_eq!(with_ram, 8, "{kind:?}: nine lives with 075A:08");
    }
}
