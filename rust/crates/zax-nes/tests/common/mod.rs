//! Test helpers shared by the CPU and bus tests.

#![allow(dead_code)]
// Each test binary uses a different subset.
// Test helpers: a failed setup panics, and the narrowing casts are of in-range values.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation
)]

pub mod mapper_fixture;

use std::ops::{Index, IndexMut};

use zax_nes::apu::Apu2A03;
use zax_nes::cartridges::Cartridge;
use zax_nes::cpu::{Cpu6502, CpuBus};
use zax_nes::input::InputPorts;
use zax_nes::ppu::Ppu2C02;
use zax_nes::rom::RomImage;
use zax_nes::{NesBus, NesCore, TileRecord};

/// A flat 64 KiB memory with no device on it, used to measure instruction behaviour and
/// cycle counts in isolation. Every access costs exactly one cycle, like the real bus.
pub struct TestBus {
    memory: Vec<u8>,
    pub cycle_count: i64,
    pub nmi_asserted: bool,
    pub irq_asserted: bool,
    /// Every address read or written, in order, for dummy-cycle assertions.
    pub access_log: Vec<u16>,
    /// The addresses read as the instruction stream ([`CpuBus::read_code`]), in order.
    pub code_log: Vec<u16>,
}

impl TestBus {
    pub fn new() -> Self {
        Self {
            memory: vec![0; 0x10000],
            cycle_count: 0,
            nmi_asserted: false,
            irq_asserted: false,
            access_log: Vec::new(),
            code_log: Vec::new(),
        }
    }

    /// Writes `bytes` starting at `address`.
    pub fn load(&mut self, address: usize, bytes: &[u8]) {
        self.memory[address..address + bytes.len()].copy_from_slice(bytes);
    }

    /// Builds a CPU whose PC is at `origin`, past the reset sequence, with the cycle counter
    /// back at zero so that a single `step` can be measured.
    pub fn create_cpu(&mut self, origin: u16, stack_pointer: u8) -> Cpu6502 {
        let mut cpu = Cpu6502::new();
        cpu.pc = origin;
        cpu.s = stack_pointer;
        self.cycle_count = 0;
        self.access_log.clear();
        self.code_log.clear();
        cpu
    }

    /// `create_cpu` with the defaults of the C# helper: $1000 and SP $FD.
    pub fn cpu(&mut self) -> Cpu6502 {
        self.create_cpu(0x1000, 0xFD)
    }
}

impl Index<usize> for TestBus {
    type Output = u8;
    fn index(&self, address: usize) -> &u8 {
        &self.memory[address]
    }
}

impl IndexMut<usize> for TestBus {
    fn index_mut(&mut self, address: usize) -> &mut u8 {
        &mut self.memory[address]
    }
}

impl CpuBus for TestBus {
    fn read(&mut self, address: u16) -> u8 {
        self.cycle_count += 1;
        self.access_log.push(address);
        self.memory[usize::from(address)]
    }

    fn read_code(&mut self, address: u16) -> u8 {
        self.code_log.push(address);
        self.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.cycle_count += 1;
        self.access_log.push(address);
        self.memory[usize::from(address)] = value;
    }

    fn peek(&self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    fn nmi_asserted(&self) -> bool {
        self.nmi_asserted
    }

    fn irq_asserted(&self) -> bool {
        self.irq_asserted
    }

    fn cycle_count(&self) -> i64 {
        self.cycle_count
    }
}

/// The minimal ROM of SPEC-02 §5.1 (C# `TestRoms.Minimal`): 2 x 16 KiB of PRG, 1 x 8 KiB
/// of CHR, mapper 0, `program` at $8000 (default: an endless `JMP $8000`), every vector at
/// $8000 except NMI when `nmi_vector` is given. Tile n of the CHR has plane 0 = eight bytes
/// of n and plane 1 = eight bytes of 255 - n; tile 0 of table $0000 is entirely zero.
pub fn minimal_rom(header_byte6: u8, program: Option<&[u8]>, nmi_vector: Option<u16>) -> Vec<u8> {
    const PRG_SIZE: usize = 2 * 16 * 1024;
    const CHR_SIZE: usize = 8 * 1024;
    let mut image = vec![0u8; 16 + PRG_SIZE + CHR_SIZE];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 0x02;
    image[5] = 0x01;
    image[6] = header_byte6;

    let code: &[u8] = match program {
        Some(code) if !code.is_empty() => code,
        _ => &[0x4C, 0x00, 0x80],
    };
    image[16..16 + code.len()].copy_from_slice(code);

    let vectors = 16 + PRG_SIZE - 6;
    for i in 0..3 {
        image[vectors + 2 * i] = 0x00;
        image[vectors + 2 * i + 1] = 0x80;
    }
    if let Some(nmi) = nmi_vector {
        image[vectors..vectors + 2].copy_from_slice(&nmi.to_le_bytes());
    }

    let chr = 16 + PRG_SIZE;
    for table in 0..2 {
        for tile in 0..256usize {
            if table == 0 && tile == 0 {
                continue;
            }
            let offset = chr + table * 4096 + tile * 16;
            for row in 0..8 {
                image[offset + row] = tile as u8;
                image[offset + 8 + row] = (255 - tile) as u8;
            }
        }
    }
    image
}

/// [`minimal_rom`] with the defaults: vertical mirroring, endless loop.
pub fn minimal() -> Vec<u8> {
    minimal_rom(0x01, None, None)
}

/// The 16 CHR bytes of pattern `tile` of `table` (0 for $0000, 1 for $1000) of the minimal
/// ROM, as M(table, n) of §5.1.
pub fn pattern(table: usize, tile: usize) -> (u64, u64) {
    if table == 0 && tile == 0 {
        return (0, 0);
    }
    (
        u64::from_le_bytes([tile as u8; 8]),
        u64::from_le_bytes([(255 - tile) as u8; 8]),
    )
}

/// A 16-byte header from bytes 4 to 15, the form of the vector table of SPEC-02 §5.2.
pub fn from_header_vector(bytes4_to_15: [u8; 12]) -> Vec<u8> {
    let mut header = vec![0u8; 16];
    header[..4].copy_from_slice(b"NES\x1A");
    header[4..].copy_from_slice(&bytes4_to_15);
    header
}

/// Stands in for `NesCore` with the minimal ROM loaded: CPU and bus, powered on.
pub fn loaded_core(program: Option<&[u8]>) -> (Cpu6502, NesBus) {
    let rom = RomImage::try_load(&minimal_rom(0x01, program, None), "Minimal.nes").unwrap();
    let mut bus = NesBus::new(Ppu2C02::new(), Apu2A03::new(), InputPorts::new());
    bus.attach(Some(Cartridge::create(rom).unwrap()));
    bus.ppu_mut().reset();
    let mut cpu = Cpu6502::new();
    cpu.reset(&mut bus);
    (cpu, bus)
}

/// A machine with `image` loaded.
pub fn machine_with(image: &[u8], name: &str) -> NesCore {
    let mut machine = NesCore::new();
    let load = machine.load_rom(image, name);
    assert!(load.success, "{:?}", load.error);
    machine
}

/// A copy of one frame batch (C# `BatchCopy`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCopy {
    pub records: Vec<TileRecord>,
    pub palette: [u8; 32],
    pub mask_x: i32,
    pub mask_y: i32,
}

impl BatchCopy {
    /// Copies the batch of the frame `machine` just ran.
    pub fn of(machine: &NesCore) -> Self {
        let batch = machine.frame_batch();
        Self {
            records: batch.records.to_vec(),
            palette: batch.palette,
            mask_x: batch.mask_x,
            mask_y: batch.mask_y,
        }
    }

    /// The background records, in capture order.
    pub fn background(&self) -> Vec<TileRecord> {
        self.records
            .iter()
            .copied()
            .filter(|r| r.background)
            .collect()
    }

    /// The sprite records, in capture order.
    pub fn sprites(&self) -> Vec<TileRecord> {
        self.records
            .iter()
            .copied()
            .filter(|r| !r.background)
            .collect()
    }
}

/// Runs one frame and returns a copy of its batch.
pub fn capture_batch(machine: &mut NesCore) -> BatchCopy {
    machine.run_frame(0);
    BatchCopy::of(machine)
}

/// Builds the state of the test vectors of SPEC-02 §5.3 on the minimal ROM of §5.1 (C#
/// `TileFixture`): three frames to pass the PPU warm-up, then the wanted state programmed
/// through memory writes to $2000-$2007 (REQ-CORE-082), then one frame, and the batch of
/// the frame after that is the one to look at.
pub struct TileFixture {
    pub core: NesCore,
}

impl TileFixture {
    /// Loads the minimal ROM with header byte 6 `header_byte6` and passes the warm-up.
    pub fn with(header_byte6: u8, program: Option<&[u8]>) -> Self {
        let mut core = machine_with(&minimal_rom(header_byte6, program, None), "Vector.nes");
        for _ in 0..3 {
            core.run_frame(0);
        }
        let mut fixture = Self { core };
        fixture.fill_oam(0xFF);
        fixture
    }

    /// The default fixture: vertical mirroring.
    pub fn new() -> Self {
        Self::with(0x01, None)
    }

    pub fn fill_oam(&mut self, value: u8) {
        self.core.write_memory(0x2003, 0x00);
        for _ in 0..256 {
            self.core.write_memory(0x2004, value);
        }
    }

    pub fn set_sprite(&mut self, index: u8, y: u8, tile: u8, attribute: u8, x: u8) {
        self.core.write_memory(0x2003, index * 4);
        for value in [y, tile, attribute, x] {
            self.core.write_memory(0x2004, value);
        }
    }

    pub fn write_vram(&mut self, address: u16, values: &[u8]) {
        self.core.write_memory(0x2006, (address >> 8) as u8);
        self.core.write_memory(0x2006, address as u8);
        for &value in values {
            self.core.write_memory(0x2007, value);
        }
    }

    pub fn fill_vram(&mut self, address: u16, count: usize, value: u8) {
        self.write_vram(address, &vec![value; count]);
    }

    pub fn set_scroll(&mut self, x: u8, y: u8) {
        self.core.write_memory(0x2005, x);
        self.core.write_memory(0x2005, y);
    }

    pub fn set_control(&mut self, value: u8) {
        self.core.write_memory(0x2000, value);
    }

    pub fn set_mask(&mut self, value: u8) {
        self.core.write_memory(0x2001, value);
    }

    /// Runs one frame, then returns the batch of the next one.
    pub fn next_batch(&mut self) -> BatchCopy {
        self.core.run_frame(0);
        capture_batch(&mut self.core)
    }
}

/// Finds a public test ROM in `rust/test-roms/`; `None` (with a message) when missing.
pub fn test_rom(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test-roms")
        .join(name);
    let bytes = std::fs::read(&path).ok();
    if bytes.is_none() {
        eprintln!("skipped: {} not found", path.display());
    }
    bytes
}

/// Outcome of a test ROM run (C# `TestRomRunner.Result`).
#[derive(Debug, Clone)]
pub struct RomRun {
    pub code: u8,
    pub passed: bool,
    pub text: String,
    pub frames: i64,
    pub timed_out: bool,
}

/// Address the later blargg suites report their status at.
pub const SHELL_STATUS: u16 = 0x6000;

/// Address the 2005-era suites keep their result code at (`validation.a`).
pub const LEGACY_RESULT: u16 = 0x00F8;

/// Frames the CPU has to stay parked at the same address before a legacy run is over.
const HALT_CONFIRMATION_FRAMES: i32 = 20;

/// Runs a ROM that uses the shell protocol: $6001-$6003 = DE B0 61, $6000 holds $80 while
/// running, $81 when it wants a reset, and the final code afterwards, 0 meaning "passed".
pub fn run_shell_rom(image: &[u8], max_frames: i64) -> RomRun {
    run_shell_rom_with(image, max_frames, true)
}

/// [`run_shell_rom`] with the choice of pressing reset when the ROM asks for it ($6000 =
/// $81). `false` stops the run there and reports $81, which is how the power-on half of a
/// reset test is read on a core whose reset is a power cycle (REQ-CORE-004).
pub fn run_shell_rom_with(image: &[u8], max_frames: i64, press_reset: bool) -> RomRun {
    let mut core = machine_with(image, "shell");
    let mut frames = 0;
    let mut started = false;
    let mut reset_delay = 0;
    while frames < max_frames {
        core.run_frame(0);
        frames += 1;

        if reset_delay > 0 {
            reset_delay -= 1;
            if reset_delay == 0 {
                core.reset();
                continue;
            }
        }

        let status = core.read_memory(SHELL_STATUS);
        if !started {
            if core.read_memory(0x6001) == 0xDE
                && core.read_memory(0x6002) == 0xB0
                && core.read_memory(0x6003) == 0x61
                && status == 0x80
            {
                started = true;
            }
            continue;
        }

        if status == 0x81 {
            if !press_reset {
                return RomRun {
                    code: status,
                    passed: false,
                    text: shell_text(&core),
                    frames,
                    timed_out: false,
                };
            }

            // "Emulator should wait a couple of frames after seeing $81."
            reset_delay = 3;
            continue;
        }

        if status != 0x80 {
            return RomRun {
                code: status,
                passed: status == 0,
                text: shell_text(&core),
                frames,
                timed_out: false,
            };
        }
    }

    RomRun {
        code: core.read_memory(SHELL_STATUS),
        passed: false,
        text: shell_text(&core),
        frames,
        timed_out: true,
    }
}

/// Runs a ROM of the 2005-era suites: it parks in a one-instruction endless loop once it has
/// reported, and the result code, 1 meaning "passed", stays in `result_address`.
pub fn run_legacy_rom(image: &[u8], max_frames: i64, result_address: u16) -> RomRun {
    let mut core = machine_with(image, "legacy");
    let mut frames = 0;
    let mut parked_at = 0u16;
    let mut parked_frames = 0;
    while frames < max_frames {
        core.run_frame(0);
        frames += 1;
        let pc = core.cpu().pc;
        if is_halted(&core) && (parked_frames == 0 || pc == parked_at) {
            parked_at = pc;
            parked_frames += 1;
            if parked_frames >= HALT_CONFIRMATION_FRAMES {
                let result = core.read_memory(result_address);
                return RomRun {
                    code: result,
                    passed: result == 1,
                    text: String::new(),
                    frames,
                    timed_out: false,
                };
            }
        } else {
            parked_frames = 0;
        }
    }

    RomRun {
        code: core.read_memory(result_address),
        passed: false,
        text: String::new(),
        frames,
        timed_out: true,
    }
}

/// Whether the CPU sits in a `jmp` to itself.
fn is_halted(core: &NesCore) -> bool {
    let pc = core.cpu().pc;
    if core.read_memory(pc) != 0x4C {
        return false;
    }
    let target = u16::from_le_bytes([
        core.read_memory(pc.wrapping_add(1)),
        core.read_memory(pc.wrapping_add(2)),
    ]);
    target == pc
}

fn shell_text(core: &NesCore) -> String {
    let mut text = String::new();
    for address in 0x6004u16..0x7FFF {
        let value = core.read_memory(address);
        if value == 0 {
            break;
        }
        text.push(char::from(value));
    }
    text.trim().to_owned()
}

/// A small deterministic generator (xorshift64*), standing in for the seeded `System.Random`
/// of the C# tests: the tests need reproducible noise, not a particular sequence.
pub struct TestRandom(u64);

impl TestRandom {
    pub const fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 32) as u16
    }

    /// A value in `0..bound`.
    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() >> 16) as usize % bound
    }

    pub fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = (self.next_u64() >> 40) as u8;
        }
    }
}
