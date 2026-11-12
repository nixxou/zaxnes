//! `nestest.nes` in automation mode, compared against Kevin Horton's reference log
//! instruction by instruction.
//!
//! This is the test that validates the microcode: 8 990 instructions covering every official
//! opcode and every unofficial one the 2A03 has, each checked for `PC`, the five registers and
//! the **cycle count**. A single wrong entry in the table, a missing page-cross cycle or a
//! flag set in the wrong order shows up within a few instructions of where it is.
//!
//! The ROM and the log are under `ZAXNES_TEST_ROMS`
//! (`nes-test-roms/other/nestest.{nes,log}`), so the test is `#[ignore]`d like the workspace's
//! other ROM tests and is run with that variable set.
//!
//! # The one column that is masked, and why
//!
//! **Bit 4 of `P` — the B ("break") flag — is excluded from the comparison.** On a real 6502
//! bit 4 is not a register bit at all: it exists only in the byte an interrupt or a `PHP`
//! *pushes*. NesHawk models it as real state instead, and `Execute.cs` is unambiguous about it
//! — all four of its `FlagB` assignments are in push micro-operations: `PushP` and `PushP_BRK`
//! set it, `PushP_IRQ` and `PushP_NMI` clear it, and none of them put it back. So after a `PHP`
//! NesHawk's `P` keeps bit 4 where silicon leaves it alone, and this port keeps NesHawk's
//! behaviour because that is what the lot is for: nestest's line 72 is the first place the two
//! part, and it is a deliberate difference in the reference, not a defect in the port.
//!
//! Masking a bit would be a way to hide a real bug, so it is worth saying exactly why it is not
//! one here. The bytes NesHawk pushes are still checked **exactly**: nestest pulls the pushed
//! `P` into `A` and operates on it, and `A` is compared unmasked on every one of the 8 990
//! lines — the `A:7F` on the line after that `PHP` is the pushed `0x7F` being verified. Every
//! other column, `PC`, `X`, `Y`, `SP` and the **cycle count**, is compared exactly too. A
//! microcode error cannot survive that; a one-bit flag convention can.

use zax_neshawk::{Mos6502x, Mos6502xLink};

/// nestest is NROM: 16 KiB of PRG mirrored into both halves of `$8000..=$FFFF`, 2 KiB of work
/// RAM mirrored through `$0000..=$1FFF`. The automation run touches nothing else — the
/// registers it writes to are absorbed, and a read of anything unmapped gives back the open-bus
/// value this port is not yet modelling, which the compared window never consumes.
struct NestestBus {
    ram: [u8; 0x800],
    prg: Vec<u8>,
    /// The address of the last opcode fetch, and whether one happened this cycle: the trace
    /// point, which is `Fetch1_Real`'s `OnExecFetch` call in the C# and is where BizHawk's own
    /// `TraceCallback` fires.
    fetch: Option<u16>,

    /// The 16-bit address bus, which lives on the machine side; see
    /// [`Mos6502xLink::set_address_bus`].
    address_bus: u16,
}

impl NestestBus {
    fn map(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => self.ram[usize::from(address) & 0x7FF],
            0x8000..=0xFFFF => {
                let offset = usize::from(address - 0x8000) % self.prg.len();
                self.prg[offset]
            }
            _ => 0xFF,
        }
    }
}

impl Mos6502xLink for NestestBus {
    fn read_memory(&mut self, address: u16) -> u8 {
        self.map(address)
    }

    fn dummy_read_memory(&mut self, address: u16) -> u8 {
        self.map(address)
    }

    fn peek_memory(&self, address: u16) -> u8 {
        self.map(address)
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        if let 0x0000..=0x1FFF = address {
            self.ram[usize::from(address) & 0x7FF] = value;
        }
    }

    fn on_exec_fetch(&mut self, address: u16) {
        self.fetch = Some(address);
    }

    fn set_address_bus(&mut self, address: u16) {
        self.address_bus = address;
    }
}

/// Bit 4, the B flag, is excluded from the `P` comparison: see the module documentation.
const NOT_B: u8 = !0x10;

/// One reference line, reduced to what is compared.
struct Expected {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    cyc: i64,
}

fn parse(line: &str) -> Option<Expected> {
    // `C000  4C F5 C5  JMP $C5F5   … A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7`
    let field = |key: &str| -> Option<u8> {
        let at = line.find(key)? + key.len();
        u8::from_str_radix(line.get(at..at + 2)?, 16).ok()
    };
    Some(Expected {
        pc: u16::from_str_radix(line.get(0..4)?, 16).ok()?,
        a: field("A:")?,
        x: field("X:")?,
        y: field("Y:")?,
        p: field("P:")?,
        s: field("SP:")?,
        cyc: line.split("CYC:").nth(1)?.trim().parse().ok()?,
    })
}

#[test]
#[ignore = "needs ZAXNES_TEST_ROMS"]
fn nestest_matches_the_reference_log_instruction_for_instruction() {
    let Ok(root) = std::env::var("ZAXNES_TEST_ROMS") else {
        panic!("set ZAXNES_TEST_ROMS");
    };
    let dir = std::path::Path::new(&root).join("nes-test-roms/other");
    let image = std::fs::read(dir.join("nestest.nes")).expect("nestest.nes");
    let log = std::fs::read_to_string(dir.join("nestest.log")).expect("nestest.log");

    // iNES: a 16-byte header then the PRG banks.
    let prg = image[16..16 + 16 * 1024].to_vec();
    let mut bus = NestestBus {
        ram: [0; 0x800],
        prg,
        fetch: None,
        address_bus: 0,
    };

    // The power-on reset sequence is run rather than assumed, because it is the thing that
    // leaves `P` at `0x24` and `S` at `0xFD` — the two values nestest's first line states, and
    // they come out of `VOP_RESET`'s three dummy pushes and its `FlagI`, not out of a constant
    // written here. Then the automation entry point is forced, which is what the reference log
    // is a run of.
    let mut cpu = Mos6502x::new();
    cpu.bcd_enabled = false; // the 2A03 has no working decimal mode
    while bus.fetch.is_none() {
        cpu.execute_one(&mut bus);
    }
    assert_eq!(cpu.P, 0x24, "P after the reset sequence");
    assert_eq!(cpu.S, 0xFD, "S after the reset sequence");

    // Re-enter at `$C000` at an instruction boundary, with the cycle count the log starts from.
    cpu.PC = 0xC000;
    cpu.opcode = 256; // VOP_Fetch1
    cpu.mi = 0;
    cpu.total_executed_cycles = 7;
    bus.fetch = None;

    let mut checked = 0usize;
    let mut lines = log.lines();
    let mut expected = parse(lines.next().expect("a first log line")).expect("line 1 parses");

    for _ in 0..200_000_000u64 {
        let before = (cpu.A, cpu.X, cpu.Y, cpu.P, cpu.S);
        cpu.execute_one(&mut bus);

        if let Some(address) = bus.fetch.take() {
            // The fetch cycle runs only `End*` and `Fetch1*`, none of which touch a register, so
            // the registers seen here are the instruction's entry state. The log's `CYC` is the
            // count *before* the instruction's first cycle, and the fetch happens inside a cycle
            // that `execute_one` has already counted -- hence the one.
            let cyc = cpu.total_executed_cycles - 1;

            assert_eq!(
                (address, cpu.A, cpu.X, cpu.Y, cpu.P & NOT_B, cpu.S, cyc),
                (
                    expected.pc,
                    expected.a,
                    expected.x,
                    expected.y,
                    expected.p & NOT_B,
                    expected.s,
                    expected.cyc
                ),
                "instruction {} (log line {}): got PC:{:04X} A:{:02X} X:{:02X} Y:{:02X} \
                 P:{:02X} SP:{:02X} CYC:{} -- registers before the fetch cycle were {:02X?}",
                checked + 1,
                checked + 1,
                address,
                cpu.A,
                cpu.X,
                cpu.Y,
                cpu.P,
                cpu.S,
                cyc,
                before
            );
            checked += 1;

            match lines.next() {
                Some(next) => expected = parse(next).expect("a log line parses"),
                None => break,
            }
        }
    }

    assert_eq!(checked, log.lines().count(), "every log line was reached");
}
