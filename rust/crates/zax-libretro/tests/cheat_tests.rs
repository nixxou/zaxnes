//! Lot CHEATS: the frontend's cheats (`retro_cheat_set`) on a program of its own: a Game Genie
//! code patches what the CPU reads, a raw code writes RAM every frame, a reset drops both.

use zax_libretro::core::{Core, Options};

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

#[test]
fn the_frontend_cheats_apply_and_reset() {
    let Ok(mut core) = Core::load(&image(), "cheats.nes", Options::default()) else {
        panic!("the program loads");
    };
    let genie = zax_cheats::code::encode_game_genie(0xE001, 0x22, None);
    assert!(core.set_cheat(0, true, &genie));
    assert!(core.set_cheat(1, true, "0020:05"));
    assert!(!core.set_cheat(2, true, "not a code"));
    core.run(0);
    core.run(0);
    assert_eq!(core.read_memory(0x0010), 0x22);
    assert_eq!(core.read_memory(0x0020), 0x05);

    assert_eq!(core.read_memory(0xE001), 0x22);

    // Off, the ROM reads as it is again; the RAM code still writes.
    assert!(core.set_cheat(0, false, &genie));
    assert_eq!(core.read_memory(0xE001), 0x11);
    core.write_memory_for_test(0x0020, 0x00);
    core.run(0);
    assert_eq!(core.read_memory(0x0020), 0x05);

    // A reset drops the RAM code too.
    core.reset_cheats();
    core.write_memory_for_test(0x0020, 0x00);
    core.run(0);
    assert_eq!(core.read_memory(0x0020), 0x00);
}
