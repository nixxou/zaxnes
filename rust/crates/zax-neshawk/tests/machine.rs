//! The machine runs: a power-on smoke test, and the blargg suites' own verdicts.
//!
//! These are `#[ignore]`d because they need ROMs that are not in the repository:
//! `ZAXNES_TEST_ROMS` for the public test ROMs and `ZAXNES_BIZHAWK_GAMEDB` for BizHawk's
//! `gamedb` directory (`gamedb_nes.txt` and `NesCarts.xml`), without which board detection falls
//! back to the iNES header and is not NesHawk-equivalent for every cartridge.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::path::PathBuf;

use zax_neshawk::controllers::PortDevice;
use zax_neshawk::gamedb::GameDb;
use zax_neshawk::Nes;

fn test_roms_dir() -> Option<PathBuf> {
    std::env::var_os("ZAXNES_TEST_ROMS").map(PathBuf::from)
}

fn gamedb() -> GameDb {
    match std::env::var_os("ZAXNES_BIZHAWK_GAMEDB") {
        Some(dir) => GameDb::load_from_dir(&PathBuf::from(dir)),
        None => GameDb::empty(),
    }
}

fn load(path: &std::path::Path, db: &GameDb) -> Nes {
    let image = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Nes::load(
        &image,
        db,
        PortDevice::ControllerNes,
        PortDevice::ControllerNes,
    )
    .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// blargg's report protocol: `$6001-$6003` hold `DE B0 61`, `$6000` is `0x80` while the test
/// runs and the result code afterwards, and `$6004` begins a NUL-terminated message.
fn blargg_verdict(nes: &mut Nes, max_frames: i32) -> (u8, String) {
    let mut status = 0xFFu8;
    let mut started = false;
    for _ in 0..max_frames {
        nes.frame_advance(false, false);

        let magic = [
            nes.peek_memory(0x6001),
            nes.peek_memory(0x6002),
            nes.peek_memory(0x6003),
        ];
        if magic != [0xDE, 0xB0, 0x61] {
            continue;
        }
        status = nes.peek_memory(0x6000);
        if status == 0x80 {
            started = true;
        } else if started || status < 0x80 {
            break;
        }
    }

    let mut text = String::new();
    for i in 0..256u16 {
        let b = nes.peek_memory(0x6004 + i);
        if b == 0 {
            break;
        }
        if b.is_ascii_graphic() || b == b' ' || b == b'\n' {
            text.push(char::from(b));
        }
    }
    (status, text.trim().to_string())
}

#[test]
#[ignore = "needs ZAXNES_TEST_ROMS"]
fn the_machine_powers_on_and_runs_sixty_frames() {
    let Some(dir) = test_roms_dir() else { return };
    let db = gamedb();
    let mut nes = load(&dir.join("nes-test-roms/other/nestest.nes"), &db);

    // The start-up frame comes first: `Reset` leaves `ppudead = 1`, so frame 0 is NewDeadPPU's
    // 241*341 - start_up_offset dots and nothing else.
    nes.frame_advance(false, false);
    assert_eq!(nes.frame, 1);
    assert_eq!(nes.ppu.ppudead, 0, "the start-up frame is spent");

    for _ in 0..59 {
        nes.frame_advance(false, false);
    }
    assert_eq!(nes.frame, 60);

    // the PPU drives the CPU three dots to the cycle, so after 60 frames both counters must be
    // in step to within one instruction
    let ppu_cycles = nes.cycle_count();
    assert!(ppu_cycles > 60 * 89_000, "{ppu_cycles} dots in 60 frames");
    assert!(
        nes.total_executed_cycles > 60 * 29_000,
        "{} cpu cycles",
        nes.total_executed_cycles
    );
}

#[test]
#[ignore = "needs ZAXNES_TEST_ROMS"]
fn the_blargg_cpu_suites_report_their_verdicts() {
    let Some(dir) = test_roms_dir() else { return };
    let db = gamedb();

    // The ROMs that report through $6000 and that NesHawk itself passes. The assertion is
    // deliberately "the same verdict as NesHawk", which for these is 0 (passed).
    let roms = [
        "nes-test-roms/instr_test-v5/rom_singles/01-basics.nes",
        "nes-test-roms/instr_test-v5/rom_singles/02-implied.nes",
        "nes-test-roms/instr_test-v5/rom_singles/03-immediate.nes",
        "nes-test-roms/instr_test-v5/rom_singles/04-zero_page.nes",
        "nes-test-roms/instr_test-v5/rom_singles/05-zp_xy.nes",
        "nes-test-roms/instr_test-v5/rom_singles/06-absolute.nes",
        "nes-test-roms/instr_test-v5/rom_singles/07-abs_xy.nes",
        "nes-test-roms/instr_test-v5/rom_singles/08-ind_x.nes",
        "nes-test-roms/instr_test-v5/rom_singles/09-ind_y.nes",
        "nes-test-roms/instr_test-v5/rom_singles/10-branches.nes",
        "nes-test-roms/instr_test-v5/rom_singles/11-stack.nes",
        "nes-test-roms/instr_test-v5/rom_singles/12-jmp_jsr.nes",
        "nes-test-roms/instr_test-v5/rom_singles/13-rts.nes",
        "nes-test-roms/instr_test-v5/rom_singles/14-rti.nes",
        "nes-test-roms/instr_test-v5/rom_singles/15-brk.nes",
        "nes-test-roms/instr_test-v5/rom_singles/16-special.nes",
    ];

    let mut report = Vec::new();
    for rom in roms {
        let path = dir.join(rom);
        if !path.exists() {
            continue;
        }
        let mut nes = load(&path, &db);
        let (status, text) = blargg_verdict(&mut nes, 1200);
        report.push(format!("{rom}: {status:#04X} {text}"));
    }

    assert!(!report.is_empty(), "no blargg ROM was found under {dir:?}");
    for line in &report {
        println!("{line}");
    }
    let failures: Vec<String> = report.into_iter().filter(|l| !l.contains("0x00")).collect();
    assert!(
        failures.is_empty(),
        "these did not report pass:\n{}",
        failures.join("\n")
    );
}
