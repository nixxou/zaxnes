//! The two public reset ROMs SPEC-02 §3.12, as amended by AMD-044, adds to the required list:
//! `cpu_reset/ram_after_reset` and `cpu_reset/registers` (blargg), driven by the **soft** reset
//! of REQ-CORE-122. A missing ROM is skipped with a message.
//!
//! Both ROMs press RESET in the middle of the run ($6000 = $81) and then check what a real
//! console keeps across it: the RAM in the first, A, X, Y, S and the I flag in the second. They
//! therefore fail under the hard reset of REQ-CORE-004, which is a power cycle (DEC-02-Q3) —
//! the note of REQ-CORE-004 says so and stays true.

mod common;

use common::{run_shell_rom_soft, run_shell_rom_with, test_rom};

#[test]
fn req_core_122_the_cpu_reset_roms_pass_under_a_soft_reset() {
    let mut failures = Vec::new();
    for rom in ["ram_after_reset.nes", "registers.nes"] {
        let path = format!("cpu_reset/{rom}");
        let Some(image) = test_rom(&path) else {
            continue;
        };
        let result = run_shell_rom_soft(&image, 20_000);
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

/// The other half of the note of REQ-CORE-004, kept true: under the **hard** reset, which is a
/// power cycle, `ram_after_reset` does not pass. The run stops at the request ($6000 = $81)
/// rather than pressing, which is how the power-on half is read on such a core.
#[test]
fn req_core_004_ram_after_reset_still_fails_under_the_hard_reset() {
    let Some(image) = test_rom("cpu_reset/ram_after_reset.nes") else {
        return;
    };
    let result = run_shell_rom_with(&image, 20_000, false);
    assert!(
        !result.passed,
        "the power cycle of REQ-CORE-004 must not pass this ROM: {}",
        result.text.replace('\n', "|")
    );
}
