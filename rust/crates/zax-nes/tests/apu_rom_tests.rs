//! The public test ROMs of SPEC-02 §3.12 that the APU and the CPU interrupts answer to
//! (REQ-CORE-113, REQ-CORE-111, REQ-CORE-004). A missing ROM is skipped with a message.

mod common;

use common::{run_legacy_rom, run_shell_rom, run_shell_rom_with, test_rom};

/// Result variable of the 2005-era `blargg_apu` runtime.
const BLARGG_APU_RESULT: u16 = 0x00F0;

fn assert_shell_pass(folder: &str, roms: &[&str]) {
    let mut failures = Vec::new();
    for rom in roms {
        let path = format!("{folder}{rom}");
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

// ---------------------------------------------------------------- REQ-CORE-113

#[test]
fn req_core_113_apu_test_tests_1_to_8_pass() {
    assert_shell_pass(
        "apu_test/rom_singles/",
        &[
            "1-len_ctr.nes",
            "2-len_table.nes",
            "3-irq_flag.nes",
            "4-jitter.nes",
            "5-len_timing.nes",
            "6-irq_flag_timing.nes",
            "7-dmc_basics.nes",
            "8-dmc_rates.nes",
        ],
    );
}

#[test]
fn req_core_113_blargg_apu_2005_tests_pass() {
    let mut failures = Vec::new();
    for rom in [
        "01.len_ctr.nes",
        "02.len_table.nes",
        "03.irq_flag.nes",
        "04.clock_jitter.nes",
        "05.len_timing_mode0.nes",
        "06.len_timing_mode1.nes",
        "07.irq_flag_timing.nes",
        "08.irq_timing.nes",
        "09.reset_timing.nes",
        "10.len_halt_timing.nes",
        "11.len_reload_timing.nes",
    ] {
        let path = format!("blargg_apu_2005.07.30/{rom}");
        let Some(image) = test_rom(&path) else {
            continue;
        };
        let result = run_legacy_rom(&image, 2000, BLARGG_APU_RESULT);
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

// ---------------------------------------------------------------- REQ-CORE-111

#[test]
fn req_core_111_the_five_cpu_interrupts_parts_pass() {
    assert_shell_pass(
        "cpu_interrupts_v2/rom_singles/",
        &[
            "1-cli_latency.nes",
            "2-nmi_and_brk.nes",
            "3-nmi_and_irq.nes",
            "4-irq_and_dma.nes",
            "5-branch_delays_irq.nes",
        ],
    );
}

#[test]
fn req_core_111_the_combined_cpu_interrupts_rom_passes() {
    assert_shell_pass("cpu_interrupts_v2/", &["cpu_interrupts.nes"]);
}

// ---------------------------------------------------------------- APU reset state

#[test]
fn req_core_004_the_power_on_half_of_the_apu_reset_tests_passes() {
    // Each of these ROMs checks its power-on condition, then asks for the reset button
    // ($6000 = $81) and checks the same thing again. The reset half cannot run here (see
    // `req_core_004_apu_reset_needs_a_reset_that_keeps_ram`), but reaching the prompt proves
    // the power-on half passed: a failed check would have exited with its own code.
    for rom in [
        "4015_cleared.nes",
        "4017_timing.nes",
        "4017_written.nes",
        "irq_flag_cleared.nes",
        "len_ctrs_enabled.nes",
        "works_immediately.nes",
    ] {
        let Some(image) = test_rom(&format!("apu_reset/{rom}")) else {
            continue;
        };
        let result = run_shell_rom_with(&image, 4000, false);

        assert_eq!(0x81, result.code, "{rom}: {}", result.text);
        assert!(
            result.text.contains("Press RESET"),
            "{rom}: {}",
            result.text
        );
    }
}

#[test]
fn req_core_004_the_frame_sequencer_starts_nine_to_twelve_cycles_before_the_first_instruction() {
    // blargg: "After reset or power-up, APU acts as if $4017 were written with $00 from 9 to
    // 12 clocks before first instruction begins." The ROM prints the delay it measured.
    let Some(image) = test_rom("apu_reset/4017_timing.nes") else {
        return;
    };
    let result = run_shell_rom_with(&image, 4000, false);

    assert!(
        result
            .text
            .contains("Delay after effective $4017 write: 10"),
        "{}",
        result.text
    );
}

// ---------------------------------------------------------------- pending (C# ApuRomPendingTests)

#[test]
#[ignore = "Reset is a power cycle here (REQ-CORE-004, DEC-02-Q3), so it clears work RAM and \
            rewrites $4017 with $00; the apu_reset ROMs keep their reset counter in RAM across \
            the reset, so their reset half cannot run. The power-on half is checked above."]
fn req_core_004_apu_reset_needs_a_reset_that_keeps_ram() {}

#[test]
#[ignore = "P3 in REQ-CORE-113: a DMC fetch halts the CPU on the cycle the APU asks for the \
            byte, so the hardware's extra reads of $2007 or $4016 are not reproduced; these \
            ROMs also report only on screen (dmc_dma_during_read4)."]
fn req_core_113_dmc_dma_during_read4_is_p3() {}

#[test]
#[ignore = "Not required by SPEC-02 §3.12: this core halts the CPU for a full four cycles per \
            DMC fetch during an OAM DMA, so sprdma_and_dmc_dma reports 528 or 529 clocks \
            instead of the hardware's 513 to 518."]
fn the_sprite_dma_and_dmc_dma_collision_is_not_cycle_exact() {}
