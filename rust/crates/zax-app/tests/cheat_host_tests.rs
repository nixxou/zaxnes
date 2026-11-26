//! Lot CHEATS: a game's cheats through the host: in force on the core at the next tick, kept in
//! `cheats/<rom>.json` and read back when the game starts again, RetroArch's U toggling the one
//! selected, and the RAM search finding the counter the game increments.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::host::cheats::{typed_cheat, RamSearchAction};
use zax_cheats::code::encode_game_genie;
use zax_cheats::search::{Comparison, Operand, ValueSize};
use zax_cheats::CheatList;

fn memory(harness: &LoopHarness, address: u16) -> u8 {
    harness.host.read_game_memory(address).expect("a game, the core here")
}

#[test]
fn cheats_apply_are_kept_and_come_back() {
    let mut harness = LoopHarness::create();
    // `$8000: INC $10; JMP $8000`.
    harness.launch("g1");
    let genie = encode_game_genie(0x8001, 0x11, Some(0x10));
    let list = CheatList {
        cheats: vec![
            typed_cheat("Counter moved", &genie).unwrap(),
            typed_cheat("Seven", "0020:07").unwrap(),
        ],
    };
    harness.host.replace_cheats(list.clone());
    let before = memory(&harness, 0x0010);
    harness.run_frames(5);
    assert_eq!(memory(&harness, 0x0010), before, "the Game Genie code moved the counter");
    assert_ne!(memory(&harness, 0x0011), 0);
    assert_eq!(memory(&harness, 0x0020), 7);

    let file = harness.root.path().join("cheats").join("g1.json");
    assert!(file.is_file(), "{} written", file.display());

    // U turns the selected cheat (the first) off: the counter counts again.
    harness.run(Command::ToggleCheat);
    assert!(!harness.host.cheats().cheats[0].enabled);
    harness.run_frames(3);
    let counted = memory(&harness, 0x0010);
    harness.run_frames(3);
    assert_ne!(memory(&harness, 0x0010), counted);

    // The game started again finds its list.
    harness.run(Command::Stop);
    harness.run_with(Command::LaunchGame, CommandArgs::of_text("g1"));
    assert_eq!(harness.host.cheats().cheats.len(), 2);
    assert!(!harness.host.cheats().cheats[0].enabled);
    assert!(harness.host.cheats().cheats[1].enabled);
    harness.run_frames(2);
    assert_eq!(memory(&harness, 0x0020), 7);

    // An empty list removes the file.
    harness.host.replace_cheats(CheatList::default());
    assert!(!file.exists());
}

#[test]
fn the_ram_search_finds_the_counter() {
    let mut harness = LoopHarness::create();
    harness.launch("g1");
    harness.host.ram_search(RamSearchAction::Start {
        size: ValueSize::Byte,
        signed: false,
    });
    harness.run_frames(1);
    for _ in 0..4 {
        harness.run_frames(1);
        harness.host.ram_search(RamSearchAction::Filter(
            Comparison::NotEqual,
            Operand::Previous,
        ));
        harness.run_frames(1);
    }
    let view = harness.host.ram_search_view();
    assert!(view.active);
    assert!(
        view.rows.iter().any(|(address, _, _)| *address == 0x0010),
        "{:?}",
        view.rows
    );
    assert!(view.count < 64, "{} candidates left", view.count);
    assert!(view.can_undo);
}
