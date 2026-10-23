//! The public test ROMs of SPEC-02 §3.12 that the PPU and the CPU branch timing answer to
//! (REQ-CORE-111, REQ-CORE-112, REQ-CORE-116).
//!
//! Each ROM is run through the runner of `common`, which reads back the result with the
//! protocol that ROM documents, and nothing else: no emulator hook, no screen scraping.
//! A missing ROM is skipped with a message.

mod common;

use common::{run_legacy_rom, run_shell_rom, test_rom, LEGACY_RESULT};

/// Result variable of the 2005-era `blargg_ppu_tests` runtime: all four park with 1 in
/// $00F0 and print "$01" on screen, the "tests passed" code of that console.
const BLARGG_PPU_RESULT: u16 = 0x00F0;

fn assert_legacy_pass(folder: &str, roms: &[&str], result_address: u16) {
    let mut failures = Vec::new();
    for rom in roms {
        let path = format!("{folder}/{rom}");
        let Some(image) = test_rom(&path) else {
            continue;
        };
        let result = run_legacy_rom(&image, 2000, result_address);
        if !result.passed {
            failures.push(format!(
                "{path}: code {}{} after {} frames",
                result.code,
                if result.timed_out { " (timed out)" } else { "" },
                result.frames
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn assert_shell_pass(folder: &str, roms: &[&str]) {
    let mut failures = Vec::new();
    for rom in roms {
        let path = format!("{folder}/{rom}");
        let Some(image) = test_rom(&path) else {
            continue;
        };
        let result = run_shell_rom(&image, 4000);
        if !result.passed {
            failures.push(format!(
                "{path}: code {}{} after {} frames, output: {}",
                result.code,
                if result.timed_out { " (timed out)" } else { "" },
                result.frames,
                result.text.replace('\n', "|")
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------- REQ-CORE-111

#[test]
fn req_core_111_branch_timing_tests_pass() {
    assert_legacy_pass(
        "branch_timing_tests",
        &[
            "1.Branch_Basics.nes",
            "2.Backward_Branch.nes",
            "3.Forward_Branch.nes",
        ],
        LEGACY_RESULT,
    );
}

// ---------------------------------------------------------------- REQ-CORE-112

#[test]
fn req_core_112_sprite_hit_tests_pass() {
    assert_legacy_pass(
        "sprite_hit_tests_2005.10.05",
        &[
            "01.basics.nes",
            "02.alignment.nes",
            "03.corners.nes",
            "04.flip.nes",
            "05.left_clip.nes",
            "06.right_edge.nes",
            "07.screen_bottom.nes",
            "08.double_height.nes",
            "09.timing_basics.nes",
            "10.timing_order.nes",
            "11.edge_timing.nes",
        ],
        LEGACY_RESULT,
    );
}

#[test]
fn req_core_112_blargg_ppu_tests_pass() {
    assert_legacy_pass(
        "blargg_ppu_tests_2005.09.15b",
        &[
            "palette_ram.nes",
            "sprite_ram.nes",
            "vram_access.nes",
            "vbl_clear_time.nes",
        ],
        BLARGG_PPU_RESULT,
    );
}

#[test]
fn req_core_112_ppu_vbl_nmi_tests_1_to_3_pass() {
    assert_shell_pass(
        "ppu_vbl_nmi/rom_singles",
        &[
            "01-vbl_basics.nes",
            "02-vbl_set_time.nes",
            "03-vbl_clear_time.nes",
        ],
    );
}

// ---------------------------------------------------------------- REQ-CORE-116

#[test]
fn req_core_116_ppu_vbl_nmi_tests_4_to_10_pass() {
    assert_shell_pass(
        "ppu_vbl_nmi/rom_singles",
        &[
            "04-nmi_control.nes",
            "05-nmi_timing.nes",
            "06-suppression.nes",
            "07-nmi_on_timing.nes",
            "08-nmi_off_timing.nes",
            "09-even_odd_frames.nes",
            "10-even_odd_timing.nes",
        ],
    );
}

#[test]
fn req_core_116_vbl_nmi_timing_tests_pass() {
    assert_legacy_pass(
        "vbl_nmi_timing",
        &[
            "1.frame_basics.nes",
            "2.vbl_timing.nes",
            "3.even_odd_frames.nes",
            "4.vbl_clear_timing.nes",
            "5.nmi_suppression.nes",
            "6.nmi_disable.nes",
            "7.nmi_timing.nes",
        ],
        LEGACY_RESULT,
    );
}

#[test]
fn req_core_116_oam_read_passes() {
    assert_shell_pass("oam_read", &["oam_read.nes"]);
}

#[test]
fn req_core_116_ppu_open_bus_passes() {
    assert_shell_pass("ppu_open_bus", &["ppu_open_bus.nes"]);
}

#[test]
fn req_core_116_sprite_overflow_tests_pass() {
    assert_legacy_pass(
        "sprite_overflow_tests",
        &[
            "1.Basics.nes",
            "2.Details.nes",
            "3.Timing.nes",
            "4.Obscure.nes",
            "5.Emulator.nes",
        ],
        LEGACY_RESULT,
    );
}
