//! `AccuracyCoin`, 100thCoin's collection of 147 accuracy tests on one NROM cartridge
//! (`AccuracyCoin/README.md` and the commented `AccuracyCoin.asm` it ships with).
//!
//! ## Driving it
//!
//! The ROM is a menu, and its README says what to press: "If the cursor is at the top of the
//! page (highlighting the current page index), pressing Start will run all tests on the ROM,
//! and then draw a table showing the results of every test." The cursor starts there — the
//! reset path sets `menuCursorYPos` to `$FF`, which the source comments as "the top of the
//! menu" — so one press of Start is the whole input a healthy run needs. Every press is held
//! for two frames and released for two, because the menu acts on `controller_New`, a button
//! that was not held on the previous frame.
//!
//! ## Reading it
//!
//! Each test owns one byte of RAM, named in the ROM's own `TestPages:` table as
//! `table "<name>", $FF, <result address>, <routine>`. The addresses run from `$0403` to
//! `$0498`, one per test, and the result-table drawing loop in the source reads them like this:
//!
//! ```text
//! LDA [suitePointerList,X]  ; read the result of test X of page Y
//! CMP #$FF                  ; If the result is $FF, we actually skipped this test.
//! ...
//! AND #01                   ; If the result isn't 1, print the error code
//! ...
//! LSR A / LSR A             ; shift error code into A
//! ```
//!
//! So a byte is `$FF` when the test was skipped, a pass when bit 0 is set, and otherwise a
//! failure whose error code is the byte shifted right twice. The README lists what each code
//! means, test by test. Five more entries of the table point at `result_DrawTest` (`$03FF`):
//! the source calls them tests that "don't actually test for anything; rather, they simply
//! print information on screen", and the ROM's own tally leaves them out, so they are not in
//! [`TESTS`] either. That is why 147 and not 152.
//!
//! A run is over when `RunningAllTests` (`$0035`) goes back to zero after having been one. The
//! ROM then recomputes `PostAllTestTally` (`$0037`) and `PostAllPassTally` (`$0038`) while it
//! draws its table, and those two are the score it shows on screen.
//!
//! ## When a test never returns
//!
//! A test the emulator cannot finish takes the whole ROM with it: the all-test loop is a
//! straight line and the 60-odd tests after the stuck one never run. The harness does what a
//! person would do, in the README's words — "Pressing B will mark the highlighted test to be
//! skipped. [...] This is useful if any tests are crashing the console or emulator and you
//! still want to see the table of results."
//!
//! So the drive is a loop. Run; if `PostAllTestTally` stops advancing, the first result byte
//! still at zero names the test that hung; power-cycle, walk the menu to that test, press B,
//! and run again. Up to [`MAX_SKIPS`] tests are retired this way, so one bad test costs the
//! campaign a few seconds instead of 60 results. The skipped tests are reported as such, never
//! as passes, and the row names each one.

use super::machine::TestMachine;
use serde_json::{json, Map, Value};

use super::screen::{self, Font};
use super::{Outcome, Verdict};

/// `$0035`: non-zero while the ROM is running every test.
const RUNNING_ALL: u16 = 0x0035;
/// `$0037`: how many tests the all-test loop has started, then how many its table counted.
const TOTAL_TALLY: u16 = 0x0037;
/// `$0038`: how many of them passed.
const PASS_TALLY: u16 = 0x0038;

/// Buttons of port 1 in the core's input word: A, B, Select, Start, Up, Down, Left, Right from
/// bit 0 (REQ-CORE-022). The ROM's own `AND #$10` for Start is the other way round because it
/// shifts `$4016` into a byte first; that is its business, not the core's.
const B: u16 = 1 << 1;
const START: u16 = 1 << 3;
const UP: u16 = 1 << 4;
const DOWN: u16 = 1 << 5;
const RIGHT: u16 = 1 << 7;

/// How many tests the drive will retire before giving up. Each retry re-runs the whole ROM
/// from a power cycle, so this is also the cost ceiling of one `AccuracyCoin` row.
pub const MAX_SKIPS: usize = 8;

/// How many retirements in a row may end by leaving the all-test loop before the drive calls
/// it a wall and stops.
///
/// One test that hangs is worth skipping: the 60 after it then run. A *run* of tests that each
/// take the ROM out of the loop is a different thing — it was measured, in this lot, to be one
/// cause blocking a whole stretch of the table, and retiring them one at a time buys a second
/// of emulation per test and no new information. The report says where the wall starts and how
/// many tests are behind it, which is the honest reading.
const WALL: usize = 3;

/// One test of the ROM, as its `TestPages:` table lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Test {
    /// The byte the test writes its result to.
    pub result: u16,
    /// Which page of the menu lists it, in `TableTable` order.
    pub page: u8,
    /// Its row on that page, counting the `DRAW` entries that share `$03FF`.
    pub index: u8,
    /// The page's title.
    pub suite: &'static str,
    /// The test's name.
    pub name: &'static str,
}

impl Test {
    /// `name (page "suite")`, how a row names a test.
    #[must_use]
    pub fn label(&self) -> String {
        format!("{} (page \"{}\")", self.name, self.suite)
    }
}

/// Every test of the ROM, in the order the all-test loop runs them, taken from the
/// `TestPages:` table and the `TableTable` page order of `AccuracyCoin.asm`.
pub const TESTS: [Test; 147] = [
    Test {
        result: 0x0405,
        page: 0,
        index: 0,
        suite: "CPU Behavior",
        name: "ROM is not writable",
    },
    Test {
        result: 0x0403,
        page: 0,
        index: 1,
        suite: "CPU Behavior",
        name: "RAM Mirroring",
    },
    Test {
        result: 0x044D,
        page: 0,
        index: 2,
        suite: "CPU Behavior",
        name: "PC Wraparound",
    },
    Test {
        result: 0x0474,
        page: 0,
        index: 3,
        suite: "CPU Behavior",
        name: "The Decimal Flag",
    },
    Test {
        result: 0x0475,
        page: 0,
        index: 4,
        suite: "CPU Behavior",
        name: "The B Flag",
    },
    Test {
        result: 0x0406,
        page: 0,
        index: 5,
        suite: "CPU Behavior",
        name: "Dummy read cycles",
    },
    Test {
        result: 0x0407,
        page: 0,
        index: 6,
        suite: "CPU Behavior",
        name: "Dummy write cycles",
    },
    Test {
        result: 0x0408,
        page: 0,
        index: 7,
        suite: "CPU Behavior",
        name: "Open Bus",
    },
    Test {
        result: 0x047D,
        page: 0,
        index: 8,
        suite: "CPU Behavior",
        name: "All NOP instructions",
    },
    Test {
        result: 0x046E,
        page: 1,
        index: 0,
        suite: "Addressing mode wraparound",
        name: "Absolute Indexed",
    },
    Test {
        result: 0x046F,
        page: 1,
        index: 1,
        suite: "Addressing mode wraparound",
        name: "Zero Page Indexed",
    },
    Test {
        result: 0x0470,
        page: 1,
        index: 2,
        suite: "Addressing mode wraparound",
        name: "Indirect",
    },
    Test {
        result: 0x0471,
        page: 1,
        index: 3,
        suite: "Addressing mode wraparound",
        name: "Indirect, X",
    },
    Test {
        result: 0x0472,
        page: 1,
        index: 4,
        suite: "Addressing mode wraparound",
        name: "Indirect, Y",
    },
    Test {
        result: 0x0473,
        page: 1,
        index: 5,
        suite: "Addressing mode wraparound",
        name: "Relative",
    },
    Test {
        result: 0x0409,
        page: 2,
        index: 0,
        suite: "Unofficial SLO",
        name: "$03 SLO Indirect,X",
    },
    Test {
        result: 0x040A,
        page: 2,
        index: 1,
        suite: "Unofficial SLO",
        name: "$07 SLO Zero Page",
    },
    Test {
        result: 0x040B,
        page: 2,
        index: 2,
        suite: "Unofficial SLO",
        name: "$0F SLO Absolute",
    },
    Test {
        result: 0x040C,
        page: 2,
        index: 3,
        suite: "Unofficial SLO",
        name: "$13 SLO Indirect,Y",
    },
    Test {
        result: 0x040D,
        page: 2,
        index: 4,
        suite: "Unofficial SLO",
        name: "$17 SLO Zero Page,X",
    },
    Test {
        result: 0x040E,
        page: 2,
        index: 5,
        suite: "Unofficial SLO",
        name: "$1B SLO Absolute,Y",
    },
    Test {
        result: 0x040F,
        page: 2,
        index: 6,
        suite: "Unofficial SLO",
        name: "$1F SLO Absolute,X",
    },
    Test {
        result: 0x0419,
        page: 3,
        index: 0,
        suite: "Unofficial RLA",
        name: "$23 RLA Indirect,X",
    },
    Test {
        result: 0x041A,
        page: 3,
        index: 1,
        suite: "Unofficial RLA",
        name: "$27 RLA Zero Page",
    },
    Test {
        result: 0x041B,
        page: 3,
        index: 2,
        suite: "Unofficial RLA",
        name: "$2F RLA Absolute",
    },
    Test {
        result: 0x041C,
        page: 3,
        index: 3,
        suite: "Unofficial RLA",
        name: "$33 RLA Indirect,Y",
    },
    Test {
        result: 0x041D,
        page: 3,
        index: 4,
        suite: "Unofficial RLA",
        name: "$37 RLA Zero Page,X",
    },
    Test {
        result: 0x041E,
        page: 3,
        index: 5,
        suite: "Unofficial RLA",
        name: "$3B RLA Absolute,Y",
    },
    Test {
        result: 0x041F,
        page: 3,
        index: 6,
        suite: "Unofficial RLA",
        name: "$3F RLA Absolute,X",
    },
    Test {
        result: 0x0420,
        page: 4,
        index: 0,
        suite: "Unofficial SRE",
        name: "$43 SRE Indirect,X",
    },
    Test {
        result: 0x047F,
        page: 4,
        index: 1,
        suite: "Unofficial SRE",
        name: "$47 SRE Zero Page",
    },
    Test {
        result: 0x0422,
        page: 4,
        index: 2,
        suite: "Unofficial SRE",
        name: "$4F SRE Absolute",
    },
    Test {
        result: 0x0423,
        page: 4,
        index: 3,
        suite: "Unofficial SRE",
        name: "$53 SRE Indirect,Y",
    },
    Test {
        result: 0x0424,
        page: 4,
        index: 4,
        suite: "Unofficial SRE",
        name: "$57 SRE Zero Page,X",
    },
    Test {
        result: 0x0425,
        page: 4,
        index: 5,
        suite: "Unofficial SRE",
        name: "$5B SRE Absolute,Y",
    },
    Test {
        result: 0x0426,
        page: 4,
        index: 6,
        suite: "Unofficial SRE",
        name: "$5F SRE Absolute,X",
    },
    Test {
        result: 0x0427,
        page: 5,
        index: 0,
        suite: "Unofficial RRA",
        name: "$63 RRA Indirect,X",
    },
    Test {
        result: 0x0428,
        page: 5,
        index: 1,
        suite: "Unofficial RRA",
        name: "$67 RRA Zero Page",
    },
    Test {
        result: 0x0429,
        page: 5,
        index: 2,
        suite: "Unofficial RRA",
        name: "$6F RRA Absolute",
    },
    Test {
        result: 0x042A,
        page: 5,
        index: 3,
        suite: "Unofficial RRA",
        name: "$73 RRA Indirect,Y",
    },
    Test {
        result: 0x042B,
        page: 5,
        index: 4,
        suite: "Unofficial RRA",
        name: "$77 RRA Zero Page,X",
    },
    Test {
        result: 0x042C,
        page: 5,
        index: 5,
        suite: "Unofficial RRA",
        name: "$7B RRA Absolute,Y",
    },
    Test {
        result: 0x042D,
        page: 5,
        index: 6,
        suite: "Unofficial RRA",
        name: "$7F RRA Absolute,X",
    },
    Test {
        result: 0x042E,
        page: 6,
        index: 0,
        suite: "Unofficial *AX",
        name: "$83 SAX Indirect,X",
    },
    Test {
        result: 0x042F,
        page: 6,
        index: 1,
        suite: "Unofficial *AX",
        name: "$87 SAX Zero Page",
    },
    Test {
        result: 0x0430,
        page: 6,
        index: 2,
        suite: "Unofficial *AX",
        name: "$8F SAX Absolute",
    },
    Test {
        result: 0x0431,
        page: 6,
        index: 3,
        suite: "Unofficial *AX",
        name: "$97 SAX Zero Page,Y",
    },
    Test {
        result: 0x0432,
        page: 6,
        index: 4,
        suite: "Unofficial *AX",
        name: "$A3 LAX Indirect,X",
    },
    Test {
        result: 0x0433,
        page: 6,
        index: 5,
        suite: "Unofficial *AX",
        name: "$A7 LAX Zero Page",
    },
    Test {
        result: 0x0434,
        page: 6,
        index: 6,
        suite: "Unofficial *AX",
        name: "$AF LAX Absolute",
    },
    Test {
        result: 0x0435,
        page: 6,
        index: 7,
        suite: "Unofficial *AX",
        name: "$B3 LAX Indirect,Y",
    },
    Test {
        result: 0x0436,
        page: 6,
        index: 8,
        suite: "Unofficial *AX",
        name: "$B7 LAX Zero Page,Y",
    },
    Test {
        result: 0x0437,
        page: 6,
        index: 9,
        suite: "Unofficial *AX",
        name: "$BF LAX Absolute,Y",
    },
    Test {
        result: 0x0438,
        page: 7,
        index: 0,
        suite: "Unofficial DCP",
        name: "$C3 DCP Indirect,X",
    },
    Test {
        result: 0x0439,
        page: 7,
        index: 1,
        suite: "Unofficial DCP",
        name: "$C7 DCP Zero Page",
    },
    Test {
        result: 0x043A,
        page: 7,
        index: 2,
        suite: "Unofficial DCP",
        name: "$CF DCP Absolute",
    },
    Test {
        result: 0x043B,
        page: 7,
        index: 3,
        suite: "Unofficial DCP",
        name: "$D3 DCP Indirect,Y",
    },
    Test {
        result: 0x043C,
        page: 7,
        index: 4,
        suite: "Unofficial DCP",
        name: "$D7 DCP Zero Page,X",
    },
    Test {
        result: 0x043D,
        page: 7,
        index: 5,
        suite: "Unofficial DCP",
        name: "$DB DCP Absolute,Y",
    },
    Test {
        result: 0x043E,
        page: 7,
        index: 6,
        suite: "Unofficial DCP",
        name: "$DF DCP Absolute,X",
    },
    Test {
        result: 0x043F,
        page: 8,
        index: 0,
        suite: "Unofficial ISC",
        name: "$E3 ISC Indirect,X",
    },
    Test {
        result: 0x0440,
        page: 8,
        index: 1,
        suite: "Unofficial ISC",
        name: "$E7 ISC Zero Page",
    },
    Test {
        result: 0x0441,
        page: 8,
        index: 2,
        suite: "Unofficial ISC",
        name: "$EF ISC Absolute",
    },
    Test {
        result: 0x0442,
        page: 8,
        index: 3,
        suite: "Unofficial ISC",
        name: "$F3 ISC Indirect,Y",
    },
    Test {
        result: 0x0443,
        page: 8,
        index: 4,
        suite: "Unofficial ISC",
        name: "$F7 ISC Zero Page,X",
    },
    Test {
        result: 0x0444,
        page: 8,
        index: 5,
        suite: "Unofficial ISC",
        name: "$FB ISC Absolute,Y",
    },
    Test {
        result: 0x0445,
        page: 8,
        index: 6,
        suite: "Unofficial ISC",
        name: "$FF ISC Absolute,X",
    },
    Test {
        result: 0x0446,
        page: 9,
        index: 0,
        suite: "Unofficial SH*",
        name: "$93 SHA Indirect,Y",
    },
    Test {
        result: 0x0447,
        page: 9,
        index: 1,
        suite: "Unofficial SH*",
        name: "$9F SHA Absolute,Y",
    },
    Test {
        result: 0x0448,
        page: 9,
        index: 2,
        suite: "Unofficial SH*",
        name: "$9B SHS Absolute,Y",
    },
    Test {
        result: 0x0449,
        page: 9,
        index: 3,
        suite: "Unofficial SH*",
        name: "$9C SHY Absolute,X",
    },
    Test {
        result: 0x044A,
        page: 9,
        index: 4,
        suite: "Unofficial SH*",
        name: "$9E SHX Absolute,Y",
    },
    Test {
        result: 0x044B,
        page: 9,
        index: 5,
        suite: "Unofficial SH*",
        name: "$BB LAE Absolute,Y",
    },
    Test {
        result: 0x0410,
        page: 10,
        index: 0,
        suite: "Unofficial Immediates",
        name: "$0B ANC Immediate",
    },
    Test {
        result: 0x0411,
        page: 10,
        index: 1,
        suite: "Unofficial Immediates",
        name: "$2B ANC Immediate",
    },
    Test {
        result: 0x0412,
        page: 10,
        index: 2,
        suite: "Unofficial Immediates",
        name: "$4B ASR Immediate",
    },
    Test {
        result: 0x0413,
        page: 10,
        index: 3,
        suite: "Unofficial Immediates",
        name: "$6B ARR Immediate",
    },
    Test {
        result: 0x0414,
        page: 10,
        index: 4,
        suite: "Unofficial Immediates",
        name: "$8B ANE Immediate",
    },
    Test {
        result: 0x0415,
        page: 10,
        index: 5,
        suite: "Unofficial Immediates",
        name: "$AB LXA Immediate",
    },
    Test {
        result: 0x0416,
        page: 10,
        index: 6,
        suite: "Unofficial Immediates",
        name: "$CB AXS Immediate",
    },
    Test {
        result: 0x0417,
        page: 10,
        index: 7,
        suite: "Unofficial Immediates",
        name: "$EB SBC Immediate",
    },
    Test {
        result: 0x0461,
        page: 11,
        index: 0,
        suite: "CPU Interrupts",
        name: "Interrupt flag latency",
    },
    Test {
        result: 0x0462,
        page: 11,
        index: 1,
        suite: "CPU Interrupts",
        name: "NMI Overlap BRK",
    },
    Test {
        result: 0x0463,
        page: 11,
        index: 2,
        suite: "CPU Interrupts",
        name: "NMI Overlap IRQ",
    },
    Test {
        result: 0x046C,
        page: 12,
        index: 0,
        suite: "APU Registers and DMA tests",
        name: "DMA + Open Bus",
    },
    Test {
        result: 0x0488,
        page: 12,
        index: 1,
        suite: "APU Registers and DMA tests",
        name: "DMA + $2002 Read",
    },
    Test {
        result: 0x044C,
        page: 12,
        index: 2,
        suite: "APU Registers and DMA tests",
        name: "DMA + $2007 Read",
    },
    Test {
        result: 0x044F,
        page: 12,
        index: 3,
        suite: "APU Registers and DMA tests",
        name: "DMA + $2007 Write",
    },
    Test {
        result: 0x045D,
        page: 12,
        index: 4,
        suite: "APU Registers and DMA tests",
        name: "DMA + $4015 Read",
    },
    Test {
        result: 0x045E,
        page: 12,
        index: 5,
        suite: "APU Registers and DMA tests",
        name: "DMA + $4016 Read",
    },
    Test {
        result: 0x046B,
        page: 12,
        index: 6,
        suite: "APU Registers and DMA tests",
        name: "DMC DMA Bus Conflicts",
    },
    Test {
        result: 0x0477,
        page: 12,
        index: 7,
        suite: "APU Registers and DMA tests",
        name: "DMC DMA + OAM DMA",
    },
    Test {
        result: 0x0479,
        page: 12,
        index: 8,
        suite: "APU Registers and DMA tests",
        name: "Explicit DMA Abort",
    },
    Test {
        result: 0x0478,
        page: 12,
        index: 9,
        suite: "APU Registers and DMA tests",
        name: "Implicit DMA Abort",
    },
    Test {
        result: 0x0465,
        page: 13,
        index: 0,
        suite: "APU Tests",
        name: "Length Counter",
    },
    Test {
        result: 0x0466,
        page: 13,
        index: 1,
        suite: "APU Tests",
        name: "Length Table",
    },
    Test {
        result: 0x0467,
        page: 13,
        index: 2,
        suite: "APU Tests",
        name: "Frame Counter IRQ",
    },
    Test {
        result: 0x0468,
        page: 13,
        index: 3,
        suite: "APU Tests",
        name: "Frame Counter 4-step",
    },
    Test {
        result: 0x0469,
        page: 13,
        index: 4,
        suite: "APU Tests",
        name: "Frame Counter 5-step",
    },
    Test {
        result: 0x046A,
        page: 13,
        index: 5,
        suite: "APU Tests",
        name: "Delta Modulation Channel",
    },
    Test {
        result: 0x045C,
        page: 13,
        index: 6,
        suite: "APU Tests",
        name: "APU Register Activation",
    },
    Test {
        result: 0x045F,
        page: 13,
        index: 7,
        suite: "APU Tests",
        name: "Controller Strobing",
    },
    Test {
        result: 0x047A,
        page: 13,
        index: 8,
        suite: "APU Tests",
        name: "Controller Clocking",
    },
    Test {
        result: 0x0460,
        page: 14,
        index: 0,
        suite: "CPU Behavior 2",
        name: "Instruction Timing",
    },
    Test {
        result: 0x046D,
        page: 14,
        index: 1,
        suite: "CPU Behavior 2",
        name: "Implied Dummy Reads",
    },
    Test {
        result: 0x048B,
        page: 14,
        index: 2,
        suite: "CPU Behavior 2",
        name: "Branch Dummy Reads",
    },
    Test {
        result: 0x047C,
        page: 14,
        index: 3,
        suite: "CPU Behavior 2",
        name: "JSR Edge Cases",
    },
    Test {
        result: 0x0490,
        page: 14,
        index: 4,
        suite: "CPU Behavior 2",
        name: "Internal Data Bus",
    },
    Test {
        result: 0x0496,
        page: 14,
        index: 5,
        suite: "CPU Behavior 2",
        name: "DMA Landing on Write",
    },
    Test {
        result: 0x0497,
        page: 14,
        index: 6,
        suite: "CPU Behavior 2",
        name: "DMC Reload Timing",
    },
    Test {
        result: 0x0498,
        page: 14,
        index: 7,
        suite: "CPU Behavior 2",
        name: "DMC Playback Rate",
    },
    Test {
        result: 0x0485,
        page: 16,
        index: 0,
        suite: "PPU Behavior",
        name: "CHR ROM is not writable",
    },
    Test {
        result: 0x0404,
        page: 16,
        index: 1,
        suite: "PPU Behavior",
        name: "PPU Register Mirroring",
    },
    Test {
        result: 0x044E,
        page: 16,
        index: 2,
        suite: "PPU Behavior",
        name: "PPU Register Open Bus",
    },
    Test {
        result: 0x0476,
        page: 16,
        index: 3,
        suite: "PPU Behavior",
        name: "PPU Read Buffer",
    },
    Test {
        result: 0x047E,
        page: 16,
        index: 4,
        suite: "PPU Behavior",
        name: "Palette RAM Quirks",
    },
    Test {
        result: 0x0450,
        page: 17,
        index: 0,
        suite: "PPU VBlank Timing",
        name: "VBlank beginning",
    },
    Test {
        result: 0x0451,
        page: 17,
        index: 1,
        suite: "PPU VBlank Timing",
        name: "VBlank end",
    },
    Test {
        result: 0x0452,
        page: 17,
        index: 2,
        suite: "PPU VBlank Timing",
        name: "NMI Control",
    },
    Test {
        result: 0x0453,
        page: 17,
        index: 3,
        suite: "PPU VBlank Timing",
        name: "NMI Timing",
    },
    Test {
        result: 0x0454,
        page: 17,
        index: 4,
        suite: "PPU VBlank Timing",
        name: "NMI Suppression",
    },
    Test {
        result: 0x0455,
        page: 17,
        index: 5,
        suite: "PPU VBlank Timing",
        name: "NMI at VBlank end",
    },
    Test {
        result: 0x0456,
        page: 17,
        index: 6,
        suite: "PPU VBlank Timing",
        name: "NMI disabled at VBlank",
    },
    Test {
        result: 0x0459,
        page: 18,
        index: 0,
        suite: "Sprite Evaluation",
        name: "Sprite overflow behavior",
    },
    Test {
        result: 0x0457,
        page: 18,
        index: 1,
        suite: "Sprite Evaluation",
        name: "Sprite 0 Hit behavior",
    },
    Test {
        result: 0x048D,
        page: 18,
        index: 2,
        suite: "Sprite Evaluation",
        name: "$2002 flag timing",
    },
    Test {
        result: 0x0489,
        page: 18,
        index: 3,
        suite: "Sprite Evaluation",
        name: "Suddenly Resize Sprite",
    },
    Test {
        result: 0x0494,
        page: 18,
        index: 4,
        suite: "Sprite Evaluation",
        name: "Misaligned OAM DMA",
    },
    Test {
        result: 0x0458,
        page: 18,
        index: 5,
        suite: "Sprite Evaluation",
        name: "Arbitrary Sprite zero",
    },
    Test {
        result: 0x045A,
        page: 18,
        index: 6,
        suite: "Sprite Evaluation",
        name: "Misaligned OAM behavior",
    },
    Test {
        result: 0x047B,
        page: 18,
        index: 7,
        suite: "Sprite Evaluation",
        name: "OAM Corruption",
    },
    Test {
        result: 0x0482,
        page: 19,
        index: 0,
        suite: "PPU Misc.",
        name: "t Register Quirks",
    },
    Test {
        result: 0x045B,
        page: 19,
        index: 1,
        suite: "PPU Misc.",
        name: "Address $2004 behavior",
    },
    Test {
        result: 0x0480,
        page: 19,
        index: 2,
        suite: "PPU Misc.",
        name: "INC $4014",
    },
    Test {
        result: 0x0486,
        page: 19,
        index: 3,
        suite: "PPU Misc.",
        name: "Rendering Flag Behavior",
    },
    Test {
        result: 0x048A,
        page: 19,
        index: 4,
        suite: "PPU Misc.",
        name: "$2007 read w/ rendering",
    },
    Test {
        result: 0x048C,
        page: 19,
        index: 5,
        suite: "PPU Misc.",
        name: "$2004 Stress Test",
    },
    Test {
        result: 0x048E,
        page: 19,
        index: 6,
        suite: "PPU Misc.",
        name: "$2007 Stress Test",
    },
    Test {
        result: 0x0481,
        page: 20,
        index: 0,
        suite: "Advanced Background Evaluation",
        name: "Attributes As Tiles",
    },
    Test {
        result: 0x0483,
        page: 20,
        index: 1,
        suite: "Advanced Background Evaluation",
        name: "Stale BG Shift Registers",
    },
    Test {
        result: 0x0487,
        page: 20,
        index: 2,
        suite: "Advanced Background Evaluation",
        name: "BG Serial In",
    },
    Test {
        result: 0x0491,
        page: 20,
        index: 3,
        suite: "Advanced Background Evaluation",
        name: "ALE + Read",
    },
    Test {
        result: 0x0492,
        page: 20,
        index: 4,
        suite: "Advanced Background Evaluation",
        name: "Hybrid Addresses",
    },
    Test {
        result: 0x0484,
        page: 21,
        index: 0,
        suite: "Advanced Sprite Evaluation",
        name: "Sprites On Scanline 0",
    },
    Test {
        result: 0x048F,
        page: 21,
        index: 1,
        suite: "Advanced Sprite Evaluation",
        name: "Stale Sprite Shift Regs",
    },
    Test {
        result: 0x0493,
        page: 21,
        index: 2,
        suite: "Advanced Sprite Evaluation",
        name: "Frozen OAM2 Increment",
    },
    Test {
        result: 0x0495,
        page: 21,
        index: 3,
        suite: "Advanced Sprite Evaluation",
        name: "Misaligned OAM2 Address",
    },
];

/// What one test's result byte means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResult {
    /// Bit 0 set: the test passed. An error code may still be there when the test has more
    /// than one acceptable behaviour, which the ROM draws as a light blue number.
    Pass(u8),
    /// Bit 0 clear: the test failed with this error code.
    Fail(u8),
    /// `$FF`: the test was marked to be skipped.
    Skipped,
    /// The byte is still what the ROM left there before it ran anything.
    NotRun,
}

impl TestResult {
    /// Reads one result byte the way the ROM's own result-table loop does.
    #[must_use]
    pub const fn of(byte: u8) -> Self {
        match byte {
            0xFF => Self::Skipped,
            0x00 => Self::NotRun,
            _ if byte & 1 == 1 => Self::Pass(byte >> 2),
            _ => Self::Fail(byte >> 2),
        }
    }

    /// The word written in the JSON.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass(_) => "pass",
            Self::Fail(_) => "fail",
            Self::Skipped => "skipped",
            Self::NotRun => "not run",
        }
    }
}

/// Runs the ROM's "run every test" path, retiring any test it hangs on, and reads the 147
/// result bytes.
pub fn run<M: TestMachine>(nes: &mut M, budget: u32) -> Verdict {
    let mut retired: Vec<Retired> = Vec::new();
    let mut frames = 0u32;
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        let skips: Vec<Test> = retired.iter().map(|entry| entry.test).collect();
        let spent = drive(nes, budget.saturating_sub(frames), &skips);
        frames = frames.saturating_add(spent.frames);

        // A test the emulator cannot get through ends the run in one of two ways: it loops for
        // ever, which the stall watch catches, or it takes the CPU somewhere that leaves the
        // all-test loop and clears `RunningAllTests`. Both leave the same trace — the result
        // bytes from that test on are still zero — and both have the same remedy, so the retry
        // asks only whether a test is left unrun that has not already been retired. Which of
        // the two it was is kept, because it is worth a different line in the report.
        let stuck = first_not_run(nes);
        let wall =
            retired.len() >= WALL && retired.iter().rev().take(WALL).all(|entry| !entry.stalled);
        let can_retry = retired.len() < MAX_SKIPS
            && !wall
            && frames < budget
            && stuck.is_some_and(|test| !skips.contains(&test));
        if !can_retry {
            break;
        }
        if let Some(test) = stuck {
            retired.push(Retired {
                test,
                stalled: spent.stalled,
            });
        }
        // A power cycle, not the RESET button: the result bytes have to go back to zero, and
        // the menu has to be at the top of page 0 again for the walk to the next test.
        nes.power();
    }

    let mut passed = 0usize;
    let mut failed = Vec::new();
    let mut hung = Vec::new();
    let mut not_run = Vec::new();
    let mut rows = Map::new();
    for test in &TESTS {
        let result = TestResult::of(nes.read_memory(test.result));
        let retired_as = retired.iter().find(|entry| entry.test == *test);
        match (result, retired_as) {
            (TestResult::Pass(_), _) => passed += 1,
            (TestResult::Fail(code), _) => failed.push(format!("{} code {code:X}", test.label())),
            (TestResult::Skipped, Some(entry)) => {
                hung.push(format!("{} — {}", test.label(), entry.how()));
            }
            (TestResult::Skipped, None) => {}
            (TestResult::NotRun, _) => not_run.push(test.label()),
        }
        rows.insert(
            format!("{} / {}", test.suite, test.name),
            Value::String(match (result, retired_as) {
                (TestResult::Skipped, Some(entry)) => entry.how().to_owned(),
                (TestResult::Pass(0) | TestResult::Skipped | TestResult::NotRun, _) => {
                    result.label().to_owned()
                }
                (TestResult::Pass(code) | TestResult::Fail(code), _) => {
                    format!("{} {code:X}", result.label())
                }
            }),
        );
    }

    let total = TESTS.len();
    let (outcome, message) = verdict_of(passed, total, &failed, &hung, &not_run);
    let mut verdict = Verdict::new("accuracycoin", outcome, message, frames);
    verdict.code = u32::try_from(failed.len() + hung.len() + not_run.len()).ok();
    verdict.detail.insert("passed".into(), json!(passed));
    verdict.detail.insert("failed".into(), json!(failed.len()));
    verdict.detail.insert("hung".into(), json!(hung));
    verdict.detail.insert("not_run".into(), json!(not_run));
    verdict.detail.insert("total".into(), json!(total));
    verdict.detail.insert("attempts".into(), json!(attempts));
    // The ROM's own score, recomputed while it draws its table. It counts a retired test as
    // skipped, exactly as a person pressing B would see it.
    verdict.detail.insert(
        "rom_score".into(),
        json!(format!(
            "{}/{}",
            nes.read_memory(PASS_TALLY),
            nes.read_memory(TOTAL_TALLY)
        )),
    );
    verdict.detail.insert("tests".into(), Value::Object(rows));
    verdict.detail.insert(
        "screen".into(),
        json!(screen::joined(&screen::rows(nes, Font::Ascii))),
    );
    verdict
}

/// A test the drive had to mark as skipped, and how it had ended the run before that.
#[derive(Debug, Clone, Copy)]
struct Retired {
    test: Test,
    /// True when the ROM looped for ever in the test, false when the run left the all-test
    /// loop instead.
    stalled: bool,
}

impl Retired {
    /// How the run ended on this test, in a few words.
    const fn how(self) -> &'static str {
        if self.stalled {
            "never returned; skipped on a later attempt"
        } else {
            "took the run out of the all-test loop; skipped on a later attempt"
        }
    }
}

/// The verdict a finished set of result bytes states.
fn verdict_of(
    passed: usize,
    total: usize,
    failed: &[String],
    hung: &[String],
    not_run: &[String],
) -> (Outcome, String) {
    if not_run.len() == total {
        return (
            Outcome::Timeout,
            "the ROM never ran its tests: no result byte of $0403-$0498 was written".to_owned(),
        );
    }
    if failed.is_empty() && hung.is_empty() && not_run.is_empty() {
        return (Outcome::Pass, format!("{passed}/{total} tests passed"));
    }

    let mut parts = vec![format!("{passed}/{total} tests passed")];
    if !hung.is_empty() {
        parts.push(format!("stopped the run: {}", hung.join(", ")));
    }
    if let Some(first) = not_run.first() {
        parts.push(format!("{} never ran, from {first} on", not_run.len()));
    }
    if !failed.is_empty() {
        let mut sorted = failed.to_vec();
        sorted.sort_unstable();
        parts.push(format!("failed: {}", sorted.join(", ")));
    }

    (Outcome::Fail, parts.join("; "))
}

/// The first test, in run order, whose result byte is still zero.
///
/// In the all-test mode the ROM writes nothing before a test and its result afterwards, so
/// this is the test it never came back from.
fn first_not_run<M: TestMachine>(core: &M) -> Option<Test> {
    TESTS
        .iter()
        .find(|test| TestResult::of(core.read_memory(test.result)) == TestResult::NotRun)
        .copied()
}

/// How one drive of the ROM ended.
#[derive(Debug, Clone, Copy)]
struct Run {
    /// Frames spent.
    frames: u32,
    /// Whether the ROM stopped advancing its own test counter: it never came back from a test.
    stalled: bool,
}

/// How many of the 147 tests have a result byte: the drive's own measure of progress.
///
/// **Not** `RunningAllTests` or `PostAllTestTally`, and that is the whole point. Both live in the
/// zero page, and `Implied Dummy Reads` — which needs every byte of it for the function it builds
/// to run out of open bus — copies `$00`-`$FF` to `$700`-`$7FF`, **zeroes it**, and puts it back
/// when it is done (`TEST_ImpliedDummyRead_BackupRAM` / `_RestoreRAM`). So for the whole of that
/// test `RunningAllTests` reads 0 and `PostAllTestTally` reads 0, which the earlier drive took for
/// the end of the run: it stopped there, called the test "took the run out of the all-test loop",
/// retired it, and then met the same wall on the next retry until `MAX_SKIPS` was spent — eight
/// tests reported as hung and the 32 behind them never reached, none of which the emulator had
/// anything to do with. Lot MF-4b found it by running those eight tests one at a time, the way the
/// README says to ("Pressing A will run the highlighted test"), and watching seven of them pass.
///
/// The result bytes are at `$0403`-`$0498`, in page 4, which no test touches but its own.
fn run_count<M: TestMachine>(core: &M) -> usize {
    TESTS
        .iter()
        .filter(|test| TestResult::of(core.read_memory(test.result)) != TestResult::NotRun)
        .count()
}

/// Marks `retired` to be skipped, presses Start at the menu's top and waits for the run.
///
/// The run is over once every test has a result byte ([`run_count`]) and the ROM has cleared
/// `RunningAllTests`; it draws its table in the same call, so a few more frames are run before the
/// tallies are read. A test that never returns would otherwise cost the whole budget — the ROM is
/// looping, not working — so the drive watches that same count: if it has not moved for `STALL`
/// frames the run will not finish. Which of the two ways it ended is told apart by
/// `RunningAllTests`: a test that took the CPU out of the loop leaves it at zero for the whole
/// wait, where a test that is merely looping inside the loop does not.
fn drive<M: TestMachine>(core: &mut M, budget: u32, retired: &[Test]) -> Run {
    /// Frames run after the ROM clears `RunningAllTests`, so that it finishes drawing its
    /// result table and recomputing the two tallies.
    const AFTER: u32 = 30;
    /// Frames the test count may stand still before the run is called stuck. A minute of
    /// emulated time, where the ROM's longest tests are seconds.
    const STALL: u32 = 3_600;
    /// Frames spent on the title screen before the first button, so that the menu is drawn.
    const SETTLE: u32 = 180;

    let mut script = Script::new(SETTLE);
    script.mark_to_skip(retired);
    script.press(START);

    let mut done_at = None;
    let mut counted = run_count(core);
    let mut moved_at = script.frames();
    let mut running_zero_since = None;
    for frame in 1..=budget {
        core.run_frame(script.input(frame));
        if frame <= script.frames() {
            continue;
        }

        if core.read_memory(RUNNING_ALL) == 0 {
            if running_zero_since.is_none() {
                running_zero_since = Some(frame);
            }
        } else {
            running_zero_since = None;
        }

        let now = run_count(core);
        if now != counted {
            counted = now;
            moved_at = frame;
        }

        if counted == TESTS.len() {
            if done_at.is_none() && running_zero_since.is_some() {
                done_at = Some(frame + AFTER);
            }
        } else if frame - moved_at >= STALL {
            let left_the_loop = running_zero_since.is_some_and(|since| frame - since >= STALL);
            return Run {
                frames: frame,
                stalled: !left_the_loop,
            };
        }

        if done_at.is_some_and(|due| frame >= due) {
            return Run {
                frames: frame,
                stalled: false,
            };
        }
    }

    Run {
        frames: budget,
        stalled: counted > 0,
    }
}

/// A list of button presses, one entry per frame.
///
/// Each press is held for `HOLD` frames and followed by `GAP` idle ones, because the menu only
/// acts on `controller_New`: a button that was not held on the previous frame.
struct Script {
    frames: Vec<u16>,
}

impl Script {
    /// Frames a button is held.
    const HOLD: u32 = 2;
    /// Idle frames after a press, so the next one is seen as new.
    const GAP: u32 = 2;

    fn new(settle: u32) -> Self {
        Self {
            frames: vec![0; settle as usize],
        }
    }

    fn press(&mut self, button: u16) {
        for _ in 0..Self::HOLD {
            self.frames.push(button);
        }
        for _ in 0..Self::GAP {
            self.frames.push(0);
        }
    }

    fn repeat(&mut self, button: u16, times: u32) {
        for _ in 0..times {
            self.press(button);
        }
    }

    /// Walks the menu to each test of `retired` and presses B on it, leaving the cursor back
    /// at the top of the page it ended on.
    ///
    /// From the top of a page, Right moves to the next page and Down puts the cursor on row 0;
    /// from row *n*, Down moves to *n+1* and B marks the row; from row 0, Up goes back to the
    /// top (`DEC menuCursorYPos` turns 0 into `$FF`). Start works from the top of any page, so
    /// the walk never has to come back to page 0.
    fn mark_to_skip(&mut self, retired: &[Test]) {
        let mut tests: Vec<Test> = retired.to_vec();
        tests.sort_by_key(|test| (test.page, test.index));

        let mut page = 0u8;
        for test in tests {
            self.repeat(RIGHT, u32::from(test.page.wrapping_sub(page)));
            page = test.page;
            self.press(DOWN);
            self.repeat(DOWN, u32::from(test.index));
            self.press(B);
            self.repeat(UP, u32::from(test.index) + 1);
        }
    }

    /// The input word of frame `frame`, counting from 1.
    fn input(&self, frame: u32) -> u16 {
        usize::try_from(frame)
            .ok()
            .and_then(|frame| self.frames.get(frame.wrapping_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    /// How many frames the script lasts.
    fn frames(&self) -> u32 {
        u32::try_from(self.frames.len()).unwrap_or(u32::MAX)
    }
}

/// `AccuracyCoin` driven **exactly as `ext-tools/scripts/coin_bizhawk.lua` drives BizHawk**, so
/// that a verdict can be held against `bizhawk-runs/coin-neshawk.txt` line for line.
///
/// This is not how [`run`] above drives it, and the difference is the whole point of having both.
/// `run` walks the menu and runs each test from a freshly powered machine, which reaches all 147;
/// the Lua presses Start once at frame 120 to start the ROM's own **run-all**, waits for every
/// result byte to go non-zero, and gives up at frame 100 000. A test that passes from a clean
/// machine can fail inside a long run-all that left state behind, and a run-all that stalls
/// leaves every later test at `NOT RUN` — so the two instruments give different numbers on the
/// same machine, and only the same instrument on both machines compares anything.
///
/// Returns `(lines, passed, frames)` where each line is the Lua's own
/// `<suite> | <name> | PASS|FAIL <n>|NOT RUN`.
pub fn run_all<M: TestMachine>(core: &mut M, budget: u32) -> (Vec<String>, usize, u32) {
    let mut frame = 0u32;
    let mut done: Option<u32> = None;
    let cap = budget.max(1);
    loop {
        frame += 1;
        // `if n >= 120 and n < 124 then joypad.set({Start = true}, 1) end`
        let input = if (120..124).contains(&frame) {
            START
        } else {
            0
        };
        core.run_frame(input);

        let filled = TESTS
            .iter()
            .filter(|test| core.read_memory(test.result) != 0)
            .count();
        if frame > 130 && filled == TESTS.len() && done.is_none() {
            done = Some(frame);
        }
        if done.is_some_and(|at| frame > at + 120) || frame > cap {
            break;
        }
    }

    let mut lines = Vec::with_capacity(TESTS.len());
    let mut passed = 0usize;
    for test in &TESTS {
        let byte = core.read_memory(test.result);
        let verdict = if byte == 0 {
            "NOT RUN".to_owned()
        } else if byte % 2 == 1 {
            passed += 1;
            "PASS".to_owned()
        } else {
            format!("FAIL {}", byte / 4)
        };
        lines.push(format!("{} | {} | {verdict}", test.suite, test.name));
    }
    (lines, passed, frame)
}
