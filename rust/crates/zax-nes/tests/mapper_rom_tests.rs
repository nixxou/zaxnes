//! Public test ROMs on mapper boards (REQ-CORE-100, REQ-CORE-110, REQ-CORE-111,
//! REQ-CORE-113, REQ-CORE-114, REQ-CORE-116). A missing ROM is skipped with a message.

mod common;

use common::{machine_with, run_legacy_rom, run_shell_rom, test_rom, RomRun};
use zax_nes::NesCore;

fn describe(run: &RomRun) -> String {
    format!(
        "code {}{} after {} frames, output: {}",
        run.code,
        if run.timed_out { " (timed out)" } else { "" },
        run.frames,
        run.text.replace('\n', "|")
    )
}

/// Runs each ROM of `folder` as a shell ROM and fails with the list of those that do not
/// pass.
fn assert_shell_pass(folder: &str, roms: &[&str], max_frames: i64) {
    let mut failures = Vec::new();
    for rom in roms {
        let path = format!("{folder}{rom}");
        let Some(image) = test_rom(&path) else {
            continue;
        };
        let result = run_shell_rom(&image, max_frames);
        if !result.passed {
            failures.push(format!("{path}: {}", describe(&result)));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn screen_text(core: &NesCore) -> String {
    let vram = core.ppu().vram();
    let mut text = String::new();
    for row in 0..30 {
        for column in 0..32 {
            let tile = vram[row * 32 + column];
            text.push(if (32..127).contains(&tile) {
                char::from(tile)
            } else {
                ' '
            });
        }

        text.push('\n');
    }

    text
}

// ---------------------------------------------------------------- MMC3, REQ-CORE-114

#[test]
fn req_core_114_mmc3_test_2_tests_1_to_5_pass() {
    assert_shell_pass(
        "mmc3_test_2/rom_singles/",
        &[
            "1-clocking.nes",
            "2-details.nes",
            "3-A12_clocking.nes",
            "4-scanline_timing.nes",
            "5-MMC3.nes",
        ],
        4000,
    );
}

#[test]
fn req_core_114_mmc3_test_2_test_6_reports_the_revision_difference() {
    // REQ-CORE-100 asks for the "revision B" counter and REQ-CORE-114 allows test 6, "MMC3
    // alt", to fail: it checks the revision A rule. Failing it with code 2 is therefore the
    // expected outcome, and is asserted so that a change of behaviour cannot pass unnoticed.
    let Some(image) = test_rom("mmc3_test_2/rom_singles/6-MMC3_alt.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 4000);
    assert_eq!(2, result.code, "{}", describe(&result));
    assert!(
        result.text.contains("reloading to 0"),
        "{}",
        describe(&result)
    );
}

#[test]
fn req_core_114_the_earlier_mmc3_irq_suite_passes() {
    // The 2005-era builds of the same tests, which report through the low-memory result
    // byte. Test 5 is the revision A counterpart of test 6 and is expected to fail.
    let mut failures = Vec::new();
    for rom in [
        "1.Clocking.nes",
        "2.Details.nes",
        "3.A12_clocking.nes",
        "4.Scanline_timing.nes",
        "6.MMC3_rev_B.nes",
    ] {
        let Some(image) = test_rom(&format!("mmc3_irq_tests/{rom}")) else {
            continue;
        };
        let result = run_legacy_rom(&image, 4000, common::LEGACY_RESULT);
        if !result.passed {
            failures.push(format!("{rom}: {}", describe(&result)));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn req_core_100_the_revision_a_mmc3_test_fails_as_the_revision_b_rule_requires() {
    let Some(image) = test_rom("mmc3_irq_tests/5.MMC3_rev_A.nes") else {
        return;
    };
    let result = run_legacy_rom(&image, 4000, common::LEGACY_RESULT);
    assert!(!result.passed, "{}", describe(&result));
}

#[test]
fn req_core_100_the_mmc6_build_is_read_as_an_mmc3() {
    // An iNES header cannot say whether a mapper 4 board is an MMC3 or an MMC6, so this build
    // runs as an MMC3 and fails on the revision rule, as REQ-CORE-114 allows for test 6. The
    // MMC6 RAM window itself is covered by a unit test on a NES 2.0 header.
    let Some(image) = test_rom("mmc3_test/6-MMC6.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 4000);
    assert!(!result.passed, "{}", describe(&result));
}

// ---------------------------------------------------------------- CPU, REQ-CORE-110/111

#[test]
fn req_core_110_instr_test_v5_all_instrs_passes() {
    // The combined build is an MMC1 cartridge: this is the ROM REQ-CORE-110 names.
    let Some(image) = test_rom("instr_test-v5/all_instrs.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 4000);
    assert!(result.passed, "{}", describe(&result));
    assert!(
        result.text.contains("All 16 tests passed"),
        "{}",
        describe(&result)
    );
}

#[test]
fn req_core_110_every_instr_test_v5_single_passes() {
    assert_shell_pass(
        "instr_test-v5/rom_singles/",
        &[
            "01-basics.nes",
            "02-implied.nes",
            "03-immediate.nes",
            "04-zero_page.nes",
            "05-zp_xy.nes",
            "06-absolute.nes",
            "07-abs_xy.nes",
            "08-ind_x.nes",
            "09-ind_y.nes",
            "10-branches.nes",
            "11-stack.nes",
            "12-jmp_jsr.nes",
            "13-rts.nes",
            "14-rti.nes",
            "15-brk.nes",
            "16-special.nes",
        ],
        4000,
    );
}

#[test]
fn req_core_111_instr_timing_passes() {
    let Some(image) = test_rom("instr_timing/instr_timing.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 3000);
    assert!(result.passed, "{}", describe(&result));
    assert!(
        result.text.contains("All 2 tests passed"),
        "{}",
        describe(&result)
    );
}

#[test]
fn req_core_110_instr_misc_passes() {
    let Some(image) = test_rom("instr_misc/instr_misc.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 4000);
    assert!(result.passed, "{}", describe(&result));
}

#[test]
fn req_core_111_cpu_timing_test6_passes_with_the_unofficial_instructions() {
    // This one only ever prints: its readme says to hold B at power-on to have the
    // unofficial instructions tested as well, and the verdict is read off the screen.
    let Some(image) = test_rom("cpu_timing_test6/cpu_timing_test.nes") else {
        return;
    };
    let mut core = machine_with(&image, "cpu_timing_test.nes");
    for _ in 0..1200 {
        core.run_frame(0x0002);
    }

    let screen = screen_text(&core);
    assert!(screen.contains("OFFICIAL + UNDOCUMENTED"), "{screen}");
    assert!(screen.contains("PASSED"), "{screen}");
}

// ---------------------------------------------------------------- PPU and APU

#[test]
fn req_core_116_the_combined_ppu_vbl_nmi_rom_passes() {
    let Some(image) = test_rom("ppu_vbl_nmi/ppu_vbl_nmi.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 3000);
    assert!(result.passed, "{}", describe(&result));
    assert!(
        result.text.contains("All 10 tests passed"),
        "{}",
        describe(&result)
    );
}

#[test]
fn req_core_116_test_ppu_read_buffer_passes() {
    let Some(image) = test_rom("ppu_read_buffer/test_ppu_read_buffer.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 3000);
    assert!(result.passed, "{}", describe(&result));
}

#[test]
fn req_core_113_the_combined_apu_test_rom_passes() {
    let Some(image) = test_rom("apu_test/apu_test.nes") else {
        return;
    };
    let result = run_shell_rom(&image, 4000);
    assert!(result.passed, "{}", describe(&result));
    assert!(
        result.text.contains("All 8 tests passed"),
        "{}",
        describe(&result)
    );
}

#[test]
fn req_core_100_cnrom_runs_cpu_dummy_reads() {
    // A CNROM cartridge; the suite itself is not required by SPEC-02, but it exercises the
    // board and the dummy reads of the CPU together.
    let Some(image) = test_rom("cpu_dummy_reads/cpu_dummy_reads.nes") else {
        return;
    };
    let mut core = machine_with(&image, "cpu_dummy_reads.nes");
    for _ in 0..600 {
        core.run_frame(0);
    }

    assert!(screen_text(&core).to_uppercase().contains("PASSED"));
}

// ---------------------------------------------------------------- boots, REQ-CORE-100

#[test]
fn req_core_100_each_public_rom_of_a_required_mapper_boots_and_draws() {
    // The acceptance criterion of REQ-CORE-100: every mapper of the table starts at least one
    // public test ROM or free homebrew. These are the ones available here; the mappers no
    // public ROM covers are pinned by the unit tests of `mapper_tests` instead.
    let roms: [(&str, i32); 15] = [
        ("MMC1_A12/mmc1_a12.nes", 1),
        ("240pee/240pee.nes", 2),
        ("240pee/240pee-bnrom.nes", 34),
        ("m22chrbankingtest/0-127.nes", 22),
        ("mmc5test/mmc5test.nes", 5),
        ("mmc5test_v2/mmc5test.nes", 5),
        ("exram/mmc5exram.nes", 5),
        ("other/firefly.nes", 5),
        ("other/BladeBuster.nes", 4),
        ("other/fceuxd.nes", 11),
        ("other/oam3.nes", 7),
        ("other/test28.nes", 28),
        ("other/Streemerz_bundle.nes", 28),
        ("other/quantum_disco_brothers_by_wAMMA.nes", 3),
        ("nes15-1.0.0/nes15-NTSC.nes", 0),
    ];
    for (rom, mapper) in roms {
        let Some(image) = test_rom(rom) else {
            continue;
        };
        let mut core = NesCore::new();
        let load = core.load_rom(&image, rom);
        assert!(load.success, "{rom}: {:?}", load.error);
        assert_eq!(mapper, core.cartridge().unwrap().mapper_number(), "{rom}");
        for _ in 0..180 {
            core.run_frame(0);
        }

        // Something was drawn: a ROM that crashed or hung before switching rendering on would
        // produce an empty batch every frame.
        assert!(!core.tile_records().is_empty(), "{rom} drew nothing");
    }
}

#[test]
fn req_core_100_action53_powers_on_with_the_last_bank_at_c000() {
    // The first thing the board's own test ROM checks; it prints the complaint below when the
    // power-on banks are wrong.
    let Some(image) = test_rom("other/test28.nes") else {
        return;
    };
    let mut core = machine_with(&image, "test28.nes");
    for _ in 0..200 {
        core.run_frame(0);
    }

    assert!(!screen_text(&core).contains("DOES NOT POWER ON"));
}
