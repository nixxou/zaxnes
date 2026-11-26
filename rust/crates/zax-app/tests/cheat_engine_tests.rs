//! Lot CHEATS: a game's cheats through the engine handle the application runs: RAM codes after
//! every frame, Game Genie codes on the CPU's reads, both dropped by a new ROM.
#![allow(clippy::unwrap_used)]

use zax_app::emulation::engine_handle::EngineHandle;
use zax_cheats::code::encode_game_genie;
use zax_cheats::{Cheat, CheatCode, CheatList};

/// `$E000: LDA #$11; STA $10; JMP $E004` on NROM.
fn image() -> Vec<u8> {
    let prg = 32 * 1024;
    let mut image = vec![0u8; 16 + prg + 8 * 1024];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 2;
    image[5] = 1;
    let program = 16 + prg - 8 * 1024;
    image[program..program + 7].copy_from_slice(&[0xA9, 0x11, 0x85, 0x10, 0x4C, 0x04, 0xE0]);
    for vector in 0..3 {
        let at = 16 + prg - 6 + vector * 2;
        image[at..at + 2].copy_from_slice(&[0x00, 0xE0]);
    }
    image
}

fn list(codes: &str) -> CheatList {
    CheatList {
        cheats: vec![Cheat {
            description: codes.to_owned(),
            codes: CheatCode::parse_list(codes).unwrap(),
            enabled: true,
        }],
    }
}

fn handle() -> EngineHandle {
    let mut handle = EngineHandle::zaxnes_core();
    assert!(handle.load_rom(&image(), "cheats.nes").success);
    handle
}

#[test]
fn ram_and_game_genie_codes_apply() {
    let mut handle = handle();
    let genie = encode_game_genie(0xE001, 0x22, None);
    assert!(handle.set_cheats(&list(&format!("{genie}+0020:05"))));
    handle.run_frame(0);
    assert_eq!(handle.engine().read_memory(0x0010), 0x22);
    assert_eq!(handle.engine().read_memory(0x0020), 0x05);

    // a write waiting for its compare value
    assert!(handle.set_cheats(&list("0021?07:09")));
    handle.run_frame(0);
    assert_eq!(handle.engine().read_memory(0x0021), 0x00);
    handle.engine_mut().write_memory(0x0021, 0x07);
    handle.run_frame(0);
    assert_eq!(handle.engine().read_memory(0x0021), 0x09);
}

#[test]
fn a_new_rom_drops_the_cheats() {
    let mut handle = handle();
    let genie = encode_game_genie(0xE001, 0x22, None);
    assert!(handle.set_cheats(&list(&format!("{genie}+0020:05"))));
    assert!(handle.load_rom(&image(), "cheats.nes").success);
    handle.run_frame(0);
    assert_eq!(handle.engine().read_memory(0x0010), 0x11);
    assert_eq!(handle.engine().read_memory(0x0020), 0x00);
}
