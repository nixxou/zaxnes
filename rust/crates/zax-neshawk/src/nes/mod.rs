//! The machine: `NES.Core.cs`.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/NES.Core.cs` (`RunCpuOne`, the DMA blocks,
//! `FrameAdvance`, `HardReset`, the memory map, `ReadReg`/`WriteReg`, the joypad ports) and
//! `NES.CpuLink.cs`. The PPU's and APU's own members are in [`crate::ppu`] and [`crate::apu`],
//! and the members of theirs that need the machine are `impl Nes` in sibling modules here.
//! MIT; see `LICENSE-BizHawk`.

pub(crate) mod apu;
mod load;

pub use load::{LoadError, LoadReport};

use crate::apu::Apu;
use crate::boards::{Board, BoardEnv, BoardKind};
use crate::cart::{CartInfo, DetectionOrigin};
use crate::controllers::ControllerDeck;
use crate::cpu::{Mos6502x, Mos6502xLink};
use crate::ppu::{Ppu, Region};

/// `DisplayType`, for the three regions NesHawk configures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayType {
    #[default]
    Ntsc,
    Pal,
    Dendy,
}

/// What a logged bus event is. The harness compares these against BizHawk's Lua
/// `event.onmemoryread`/`onmemorywrite`/`onmemoryexecuteany` streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusEventKind {
    Exec,
    Read,
    Write,
}

/// One entry of [`BusLog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusEvent {
    pub kind: BusEventKind,
    pub address: u16,
    pub value: u8,
    /// `cpu.TotalExecutedCycles` at the moment of the access, which is what
    /// `emu.totalexecutedcycles()` returns on the BizHawk side.
    pub cycle: i64,
}

/// `NES.MemoryCallbacks`, cut down to what the port's verification needs: an off-by-default
/// recording of every CPU bus access, in order.
#[derive(Debug, Default)]
pub struct BusLog {
    pub enabled: bool,
    pub events: Vec<BusEvent>,
    /// Stop recording once this many events are held, so a runaway trace cannot eat the heap.
    pub limit: usize,
}

impl BusLog {
    fn push(&mut self, kind: BusEventKind, address: u16, value: u8, cycle: i64) {
        if !self.enabled {
            return;
        }
        if self.limit != 0 && self.events.len() >= self.limit {
            return;
        }
        self.events.push(BusEvent {
            kind,
            address,
            value,
            cycle,
        });
    }
}

//PAL: sequence of ppu clocks per cpu clock: 3,3,3,3,4 — "at least it should be, but something is
//off with that (start up time?) so it is 3,3,3,4,3 for now". NTSC: 3.
const CPU_SEQUENCE_NTSC: [u8; 5] = [3, 3, 3, 3, 3];
const CPU_SEQUENCE_PAL: [u8; 5] = [3, 3, 3, 4, 3];

/// `NES.RomChecksums`.
mod rom_checksums {
    pub const CAMERICA_GOLDEN5: &str = "60FC5FA5B5ACCAF3AEFEBA73FC8BFFD3C4DAE558";
    pub const CAMERICA_GOLDEN5_OVERDUMP: &str = "BAD382331C30B22A908DA4BFF2759C25113CC26A";
    pub const CAMERICA_PEGASUS_4IN1: &str = "40409FEC8249EFDB772E6FFB2DCD41860C6CCA23";
    pub const DANCING_BLOCKS: &str = "68ABE1E49C9E9CCEA978A48232432C252E5912C0";
    pub const SILVA_SAGA: &str = "00C50062A2DECE99580063777590F26A253AAB6B";
    pub const FAM_JUMP_II: &str = "1D7417D31E19B590AFCEB6A8A6E7B9CAB9F9B475";
}

/// NesHawk's `NES`.
///
/// Every field is `NES.Core.cs`'s, with the C#'s name. The exceptions, all named here so that no
/// reader has to find them: `blip` is not here (the host resamples [`Self::audio_deltas`]
/// instead), the cheat, VS. System, FDS, NSF, code-data-logger and memory-domain members are not
/// ported, and `address_bus` has moved from the CPU to the machine for the reason
/// [`Mos6502xLink::set_address_bus`] gives.
pub struct Nes {
    //hardware/state
    pub cpu: Mos6502x,
    pub ppu: Ppu,
    pub apu: Apu,
    pub ram: [u8; 0x800],
    /// AKA nametables
    pub ciram: [u8; 0x800],
    pub(crate) game_name: String,
    /// the current cart prototype. should be moved into the board, perhaps
    pub cart: CartInfo,
    /// the board hardware that is currently driving things
    pub board: Box<dyn Board>,
    pub(crate) board_kind: BoardKind,
    pub(crate) origin: DetectionOrigin,
    sprdma_countdown: i32,

    /// various irq signals that get merged to the cpu irq pin
    pub irq_apu: bool,

    /// Clock speed of the main cpu in hz. Used to time audio synthesis.
    pub cpuclockrate: i32,

    pub(crate) controller_deck: ControllerDeck,
    latched4016: u8,

    pub(crate) display_type: DisplayType,
    pub(crate) vsync_num: i32,
    pub(crate) vsync_den: i32,

    pub old_s: i32,

    pub double_controller_read: i64,
    pub double_controller_read_address: u16,
    pub previous_controller1_read: u8,
    pub previous_controller2_read: u8,
    pub dmc_dma_controller_conflict: bool,
    pub joypad_strobed: bool,
    pub joypad_strobe_value: u8,

    pub cpu_sequence: [u8; 5],
    pub cpu_deadcounter: i32,

    pub oam_dma_index: i32,
    pub oam_dma_exec: bool,
    pub oam_dma_addr: u16,
    pub oam_dma_byte: u8,
    pub dmc_dma_exec: bool,
    pub dmc_realign: bool,
    /// old data bus values from previous reads
    pub db: u8,

    /// The 16-bit address bus; see [`Mos6502xLink::set_address_bus`].
    pub address_bus: u16,

    pub frame: i32,
    pub(crate) lagged: bool,
    pub(crate) islag: bool,
    pub(crate) lagcount: i32,

    // these variables are for subframe input control
    pub controller_was_latched: bool,
    pub frame_is_done: bool,
    pub current_strobe: bool,
    pub new_strobe: bool,

    reset_signal: bool,
    hard_reset_signal: bool,

    /// `InitialWRamStatePattern`, which is empty by default and then the FCEUX pattern is used.
    pub initial_wram_state_pattern: Vec<u8>,

    /// Every `blip.AddDelta(sampleclock, delta)` of `RunCpuOne`, kept for the host to resample.
    pub audio_deltas: Vec<(u32, i32)>,

    pub bus_log: BusLog,

    /// What the load decided, and from where. `None` only in a machine built by hand in a test.
    pub load_report: Option<LoadReport>,
}

/// The CPU's view of the machine: `NES.CpuLink`, with the machine split off from the CPU so that
/// the borrow checker will allow `cpu.execute_one(&mut link)`.
struct NesLink<'a> {
    nes: &'a mut Nes,
}

impl Mos6502xLink for NesLink<'_> {
    fn read_memory(&mut self, address: u16) -> u8 {
        self.nes.read_memory(address)
    }
    fn dummy_read_memory(&mut self, address: u16) -> u8 {
        self.nes.read_memory(address)
    }
    fn peek_memory(&self, _address: u16) -> u8 {
        // `NES.PeekMemory` needs `&mut` here (the board's `ReadWram` is not `const` in the C#
        // either); the CPU only uses this for a disassembly, which this port does not do.
        0
    }
    fn write_memory(&mut self, address: u16, value: u8) {
        self.nes.write_memory(address, value);
    }
    fn on_exec_fetch(&mut self, address: u16) {
        self.nes.exec_fetch(address);
    }
    fn set_address_bus(&mut self, address: u16) {
        self.nes.address_bus = address;
    }
}

impl Nes {
    /// `NES.HardReset`.
    pub fn hard_reset(&mut self) {
        self.cpu = Mos6502x::new();
        self.cpu.bcd_enabled = false;

        self.ppu = Ppu::new();
        self.ram = [0; 0x800];
        self.ciram = [0; 0x800];

        // set up region
        match self.display_type {
            DisplayType::Pal => {
                self.apu = Apu::new(Some(&self.apu), true);
                self.ppu.set_region(Region::Pal);
                self.cpuclockrate = 1_662_607;
                self.vsync_num = self.cpuclockrate * 2;
                self.vsync_den = 66495;
                self.cpu_sequence = CPU_SEQUENCE_PAL;
            }
            DisplayType::Ntsc => {
                self.apu = Apu::new(Some(&self.apu), false);
                self.ppu.set_region(Region::Ntsc);
                self.cpuclockrate = 1_789_773;
                self.vsync_num = self.cpuclockrate * 2;
                self.vsync_den = 59561;
                self.cpu_sequence = CPU_SEQUENCE_NTSC;
            }
            DisplayType::Dendy => {
                self.apu = Apu::new(Some(&self.apu), false);
                self.ppu.set_region(Region::Dendy);
                self.cpuclockrate = 1_773_448;
                self.vsync_num = self.cpuclockrate;
                self.vsync_den = 35464;
                self.cpu_sequence = CPU_SEQUENCE_NTSC;
            }
        }

        self.board_system_hard_reset();

        // apu has some specific power up bahaviour that we will emulate here
        self.apu.nes_hard_reset();

        if !self.initial_wram_state_pattern.is_empty() {
            let pattern = &self.initial_wram_state_pattern;
            for i in 0..0x800 {
                self.ram[i] = pattern[i % pattern.len()];
            }
        } else {
            // check fceux's PowerNES and FCEU_MemoryRand function for more information:
            // relevant games: Cybernoid; Minna no Taabou no Nakayoshi Daisakusen; Huang Di
            for i in 0..0x800 {
                self.ram[i] = if (i & 4) != 0 { 0xFF } else { 0x00 };
            }
        }

        // some boards cannot have specific values in RAM upon initialization
        // these are defined through the gameDB exclusively
        let hash = self.cart.game_info_hash.clone();
        if let Some(hash) = hash {
            if hash == rom_checksums::CAMERICA_GOLDEN5
                || hash == rom_checksums::CAMERICA_GOLDEN5_OVERDUMP
                || hash == rom_checksums::CAMERICA_PEGASUS_4IN1
            {
                self.ram[0x701] = 0xFF;
            } else if hash == rom_checksums::DANCING_BLOCKS {
                self.ram[0xEC] = 0;
                self.ram[0xED] = 0;
            } else if hash == rom_checksums::SILVA_SAGA || hash == rom_checksums::FAM_JUMP_II {
                self.board.base_mut().wram.fill(0xFF);
            }
        }
    }

    /// `NES.BoardSystem.cs`'s `BoardSystemHardReset`: a brand new board of the same class, with
    /// the ROM and the save RAM carried over and the WRAM and VRAM cleared.
    fn board_system_hard_reset(&mut self) {
        let mut newboard = crate::boards::create_board(self.board_kind);
        newboard.base_mut().cart = self.cart.clone();
        newboard.configure(self.origin);
        self.cart = newboard.base().cart.clone();

        {
            let old = self.board.base();
            let new = newboard.base_mut();
            new.rom.clone_from(&old.rom);
            new.vrom.clone_from(&old.vrom);
            if !old.wram.is_empty() {
                new.wram = vec![0; old.wram.len()];
            }
            if !old.vram.is_empty() {
                new.vram = vec![0; old.vram.len()];
            }
        }
        newboard.post_configure();

        // the old board's sram must be restored
        if let Some(save) = self.board.save_ram() {
            let save = save.to_vec();
            if let Some(dst) = newboard.save_ram_mut() {
                let n = save.len().min(dst.len());
                dst[..n].copy_from_slice(&save[..n]);
            }
        }

        self.ppu.has_clock_ppu = newboard.has_clock_ppu();
        self.board = newboard;
    }

    /// `NES.CycleCount`.
    #[must_use]
    pub fn cycle_count(&self) -> i64 {
        self.ppu.total_cycles()
    }

    /// `NES.FrameAdvance`.
    ///
    /// `reset` and `power` are the two console buttons; the controller state itself is set with
    /// [`crate::controllers::ControllerDeck`] before the call.
    pub fn frame_advance(&mut self, reset: bool, power: bool) {
        self.lagged = true;
        if self.reset_signal {
            self.board.nes_soft_reset();
            self.cpu.nes_soft_reset();
            self.apu.nes_soft_reset();
            self.ppu.nes_soft_reset();
        } else if self.hard_reset_signal {
            self.hard_reset();
        }

        self.reset_signal = reset;
        self.hard_reset_signal = power;

        self.cpu.ext_ppu_cycle = 0; // Reset this value at the beginning of each frame

        if self.ppu.ppudead > 0 {
            while self.ppu.ppudead > 0 {
                self.new_dead_ppu();
            }
        } else {
            // do the vbl ticks seperate, that will save us a few checks that don't happen in
            // active region
            while self.ppu.do_vbl {
                self.tick_ppu_vbl();
            }

            // now do the rest of the frame
            while self.ppu.do_active_sl {
                self.tick_ppu_active();
            }

            // now do the pre-NMI lines
            while self.ppu.do_pre_vbl {
                self.tick_ppu_pre_vbl();
            }
        }

        if self.lagged {
            self.lagcount += 1;
            self.islag = true;
        } else {
            self.islag = false;
        }

        self.frame += 1;
    }

    /// `NES.do_single_step`, the subframe core's one PPU step.
    pub fn do_single_step(&mut self) -> (bool, bool) {
        self.controller_was_latched = false;
        self.frame_is_done = false;

        self.current_strobe = self.new_strobe;
        if self.ppu.ppudead > 0 {
            self.new_dead_ppu();
        } else if self.ppu.do_vbl {
            self.tick_ppu_vbl();
        } else if self.ppu.do_active_sl {
            self.tick_ppu_active();
        } else if self.ppu.do_pre_vbl {
            self.tick_ppu_pre_vbl();
        }

        (self.controller_was_latched, self.frame_is_done)
    }

    /// `NES.RunCpuOne`: one CPU cycle, in the C#'s order, which is the whole point of the port.
    pub fn run_cpu_one(&mut self) {
        ///////////////////////////
        // OAM DMA start
        ///////////////////////////

        if self.oam_dma_exec && self.apu.dmc_dma_countdown != 1 && !self.dmc_realign {
            if self.cpu_deadcounter == 0 {
                if self.oam_dma_index % 2 == 0 {
                    self.oam_dma_byte = self.read_memory(self.oam_dma_addr);
                    self.oam_dma_addr = self.oam_dma_addr.wrapping_add(1);
                } else {
                    let b = self.oam_dma_byte;
                    self.write_memory(0x2004, b);
                }
                self.oam_dma_index += 1;
                if self.oam_dma_index == 512 {
                    self.oam_dma_exec = false;
                }
            } else {
                self.cpu_deadcounter -= 1;
            }
        }

        self.dmc_realign = false;

        /////////////////////////////
        // OAM DMA end
        /////////////////////////////

        /////////////////////////////
        // dmc dma start
        /////////////////////////////

        if self.apu.dmc_dma_countdown > 0 {
            if self.apu.dmc_dma_countdown == 1 {
                self.dmc_realign = true;
            }

            // By this point the cpu should be frozen, if it is not, then we are in a multi-write
            // opcode, add another cycle delay
            if !self.cpu.rdy
                && !self.cpu.rdy_freeze
                && (self.apu.dmc_dma_countdown == self.apu.dmc_rdy_check)
            {
                self.apu.dmc_dma_countdown += 2;
            }

            self.cpu.rdy = false;
            self.dmc_dma_exec = true;
            self.apu.dmc_dma_countdown -= 1;
            if self.apu.dmc_dma_countdown == 0 {
                self.run_dmc_fetch();

                self.dmc_dma_exec = false;
                self.apu.dmc_dma_countdown = -1;

                if (self.apu.dmc.timer == 2) && (self.apu.dmc.out_bits_remaining == 0) {
                    if self.apu.dmc.sample_length != 0 {
                        self.apu.dmc.fill_glitch = true;
                    }
                }

                if (self.apu.dmc.timer == 4)
                    && (self.apu.dmc.out_bits_remaining == 0)
                    && (self.apu.dmc.sample_length == 1)
                {
                    self.apu.dmc.fill_glitch_2 = true;
                }
            } else {
                // the DMC DMA Halt, Put cycles
                self.run_dmc_halt_fetch();
            }
        }

        /////////////////////////////
        // dmc dma end
        /////////////////////////////
        self.apu_run_one_first();

        self.cpu.irq = self.irq_apu || self.board.irq_signal();

        // DMC was started in the APU, but in this case it only lasts 1 cycle and is then
        // aborted, so put this here
        if self.apu.dmc.fill_glitch_2_end {
            self.apu.dmc_dma_countdown = -1;
            self.dmc_dma_exec = false;
            self.apu.dmc.fill_glitch_2 = false;
            self.apu.dmc.fill_glitch_2_end = false;
        }

        self.cpu_execute_one();
        self.board.clock_cpu();

        let s = self.apu.emit_sample();

        if s != self.old_s {
            self.audio_deltas
                .push((self.apu.sampleclock, s - self.old_s));
            self.old_s = s;
        }
        self.apu.sampleclock += 1;

        self.apu_run_one_last();

        if !self.cpu.rdy && !self.dmc_dma_exec && !self.oam_dma_exec {
            self.cpu.rdy = true;
        }
    }

    /// `cpu.ExecuteOne()`, with the CPU lifted out of the machine for the duration so that the
    /// link may hold the rest of it.
    fn cpu_execute_one(&mut self) {
        let mut cpu = std::mem::take(&mut self.cpu);
        cpu.execute_one(&mut NesLink { nes: self });
        self.cpu = cpu;
    }

    // ------------------------------------------------------------------ $4000-$401F

    /// `NES.ReadReg`.
    pub fn read_reg(&mut self, addr: i32) -> u8 {
        match addr {
            0x4000..=0x4013 => self.db,
            0x4014 => self.db, /*OAM DMA*/
            0x4015 => {
                let a = self.apu.read_reg(addr) & 0xDF;
                a.wrapping_add(self.db & 0x20)
            }
            0x4016 => {
                // special hardware glitch case
                if self.dmc_dma_exec && self.ppu.region != Region::Ntsc {
                    return self.db;
                }
                self.read_joyport(addr)
            }
            0x4017 => self.read_joyport(addr),
            _ => self.db,
        }
    }

    /// `NES.PeekReg`.
    #[must_use]
    pub fn peek_reg(&self, addr: i32) -> u8 {
        match addr {
            0x4000..=0x4013 => self.apu.peek_reg(addr),
            0x4014 => 0xFF, /*OAM DMA*/
            0x4015 => self.apu.peek_reg(addr),
            // "at the moment, the new system doesn't support peeks"
            0x4016 | 0x4017 => 0,
            _ => 0xFF,
        }
    }

    /// `NES.WriteReg`.
    fn write_reg(&mut self, addr: i32, val: u8) {
        match addr {
            0x4000..=0x4013 => self.apu.write_reg(addr, val),
            0x4014 => {
                //schedule a sprite dma event for beginning 1 cycle in the future.
                //this receives 2 because that's just the way it works out.
                self.oam_dma_addr = u16::from(val) << 8;
                self.sprdma_countdown = 1;

                if self.sprdma_countdown > 0 {
                    self.sprdma_countdown -= 1;
                    if self.sprdma_countdown == 0 {
                        if self.apu.dmc.timer % 2 == 0 {
                            self.cpu_deadcounter = 2;
                        } else {
                            self.cpu_deadcounter = 1;
                        }
                        self.oam_dma_exec = true;
                        self.cpu.rdy = false;
                        self.oam_dma_index = 0;
                    }
                }
            }
            0x4015 => self.apu.write_reg(addr, val),
            0x4016 => self.write_joyport(val),
            0x4017 => self.apu.write_reg(addr, val),
            _ => {}
        }
    }

    fn write_joyport(&mut self, value: u8) {
        self.joypad_strobe_value = value;
        self.joypad_strobed = true;
    }

    /// `NES.strobe_joyport`. The controllers only get strobed when transitioning from a get cycle
    /// to a put cycle.
    pub fn strobe_joyport(&mut self) {
        let old = self.latched4016;
        let new = self.joypad_strobe_value;
        self.controller_deck.strobe(old, new);
        self.latched4016 = new;
        self.new_strobe = (new & 1) != 0;
        if self.current_strobe && !self.new_strobe {
            self.controller_was_latched = true;
            self.lagged = false;
        }
        self.current_strobe = self.new_strobe;
    }

    /// `NES.read_joyport`.
    fn read_joyport(&mut self, addr: i32) -> u8 {
        let mut ret;
        if self.cpu.total_executed_cycles == self.double_controller_read
            && addr == i32::from(self.double_controller_read_address)
        {
            if addr == 0x4016 {
                ret = self.previous_controller1_read;
            } else {
                ret = self.previous_controller2_read;
            }
        } else if addr == 0x4016 {
            ret = self.controller_deck.read_a();
            // If the following CPU cycle is also reading from this controller port, read the
            // same value without clocking the controller.
            self.previous_controller1_read = ret;
        } else {
            ret = self.controller_deck.read_b();
            self.previous_controller2_read = ret;
        }
        self.double_controller_read_address = addr as u16;
        // The shift register in the controller is only updated if the previous CPU cycle did not
        // read from the controller port.
        self.double_controller_read = self.cpu.total_executed_cycles + 1;
        if self.dmc_dma_controller_conflict {
            // since the DMC DMA fetch routine occurs before cpu.ExecuteOne() which updates
            // TotalExecutedCycles, we need to increment this value.
            self.double_controller_read += 1;
        }

        ret &= 0x1F;
        ret |= 0xE0 & self.db;
        ret
    }

    // ------------------------------------------------------------------ the memory map

    /// The two fields a board reaches back for, as disjoint borrows of `self`.
    fn board_env(&mut self) -> (&mut dyn Board, BoardEnv<'_>) {
        (
            &mut *self.board,
            BoardEnv {
                ciram: &mut self.ciram,
                db: self.db,
            },
        )
    }

    /// `NES.ExecFetch`.
    fn exec_fetch(&mut self, addr: u16) {
        let cyc = self.cpu.total_executed_cycles;
        self.bus_log.push(BusEventKind::Exec, addr, 0, cyc);
    }

    /// `NES.ReadMemory`.
    pub fn read_memory(&mut self, addr: u16) -> u8 {
        if !self.oam_dma_exec && !self.dmc_dma_exec {
            self.address_bus = addr;
        }
        let ret;

        let iaddr = i32::from(addr);
        if addr >= 0x8000 {
            // easy optimization, since rom reads are so common, move this up
            ret = self.board.read_prg(iaddr - 0x8000);
        } else if addr < 0x0800 {
            ret = self.ram[addr as usize];
        } else if addr < 0x2000 {
            ret = self.ram[(addr & 0x7FF) as usize];
        } else if addr < 0x4000 {
            // Board.ReadReg2xxx; no board in scope overrides it
            ret = self.ppu_read_reg(iaddr & 7);
        } else if addr < 0x6000 {
            // Famicom Disk System Registers, or open bus. APU Registers need to be activated
            // based on the position of the 6502 address bus, see below.
            let db = self.db;
            ret = self.board.read_exp(iaddr - 0x4000, db);
        } else {
            let db = self.db;
            ret = self.board.read_wram(iaddr - 0x6000, db);
        }

        let mut ret = ret;
        if self.address_bus >= 0x4000 && self.address_bus <= 0x401F {
            // Regardless of the address the CPU is intending to read, if the 6502 address bus is
            // pointing to the APU registers, then the APU registers are active.
            if (addr & 0x1F) == 0x15 {
                if !self.dmc_dma_exec {
                    ret &= 0x20; // only bit 5 of $4015 is open bus
                    ret |= self.read_reg(0x4000 | (iaddr & 0x1F)) & 0xDF;
                } else {
                    // Poke OAMSTATUS to potentially clear the APU Frame Counter IRQ Flag.
                    self.read_reg(0x4000 | (iaddr & 0x1F));
                }
            }
            if (addr & 0x1F) == 0x16 || (addr & 0x1F) == 0x17 {
                ret &= 0xE0; // only bits 5, 6, and 7 of the controller ports are open bus
                ret |= self.read_reg(0x4000 | (iaddr & 0x1F)) & 0x1F;
            }
        }

        let cyc = self.cpu.total_executed_cycles;
        self.bus_log.push(BusEventKind::Read, addr, ret, cyc);

        if !(self.address_bus >= 0x4000 && self.address_bus < 0x4020 && (addr & 0x1F) == 0x15) {
            // This register is internal to the CPU and so the external CPU data bus is
            // disconnected when reading it.
            self.db = ret;
        }

        ret
    }

    /// `NES.WriteMemory`.
    pub fn write_memory(&mut self, addr: u16, value: u8) {
        if !self.oam_dma_exec {
            self.address_bus = addr;
        }
        let iaddr = i32::from(addr);
        if addr < 0x0800 {
            self.ram[addr as usize] = value;
        } else if addr < 0x2000 {
            self.ram[(addr & 0x7FF) as usize] = value;
        } else if addr < 0x4000 {
            // Board.WriteReg2xxx; no board in scope overrides it
            self.ppu_write_reg(iaddr, value);
        } else if addr < 0x4020 {
            self.write_reg(iaddr, value);
        } else if addr < 0x6000 {
            self.board.write_exp(iaddr - 0x4000, value);
        } else if addr < 0x8000 {
            self.board.write_wram(iaddr - 0x6000, value);
        } else {
            self.board.write_prg(iaddr - 0x8000, value);
        }

        let cyc = self.cpu.total_executed_cycles;
        self.bus_log.push(BusEventKind::Write, addr, value, cyc);

        self.db = value;
    }

    /// `NES.PeekMemory`, for a debugger or the harness. `&mut` because a board's `ReadWram` is
    /// not `const` in the C# either; nothing here changes machine state.
    #[must_use]
    pub fn peek_memory(&mut self, addr: u16) -> u8 {
        let iaddr = i32::from(addr);
        if addr >= 0x4020 {
            let db = self.db;
            self.board.peek_cart(iaddr, db)
        } else if addr < 0x0800 {
            self.ram[addr as usize]
        } else if addr < 0x2000 {
            self.ram[(addr & 0x7FF) as usize]
        } else if addr < 0x4000 {
            self.ppu_peek_reg(iaddr & 7)
        } else {
            self.peek_reg(iaddr)
        }
    }

    /// `NES.ApplySystemBusPoke`, which is how a cheat and the harness write through the bus.
    pub fn apply_system_bus_poke(&mut self, addr: i32, value: u8) {
        if addr < 0x2000 {
            self.ram[(addr & 0x7FF) as usize] = value;
        } else if addr < 0x4000 {
            self.ppu_write_reg(addr, value);
        } else if addr < 0x4020 {
            self.write_reg(addr, value);
        } else if addr < 0x6000 {
            // "let's ignore pokes to EXP until someone asks for it"
        } else if addr < 0x8000 {
            self.board.write_wram(addr - 0x6000, value);
        }
        // a cheat on non-writable memory is not ported
    }

    /// `NES.ppubus_read`'s board side, with the PPU's `PPUON` guard.
    pub(crate) fn ppubus_read(&mut self, addr: i32, ppu: bool, addr_ppu: bool) -> u8 {
        //hardware doesnt touch the bus when the PPU is disabled
        if !self.ppu.ppuon() && ppu {
            return 0xFF;
        }

        if addr_ppu {
            self.board.address_ppu(addr);
        }

        let (board, mut env) = self.board_env();
        board.read_ppu(addr, &mut env)
    }

    /// `NES.ppubus_write`.
    pub(crate) fn ppubus_write(&mut self, addr: i32, value: u8) {
        if self.ppu.ppur.status.sl >= 241 || !self.ppu.ppuon() {
            self.board.address_ppu(addr);
        }

        let (board, mut env) = self.board_env();
        board.write_ppu(addr, value, &mut env);
    }

    /// `NES.GameName`.
    #[must_use]
    pub fn game_name(&self) -> &str {
        &self.game_name
    }

    /// `NES.IsLagFrame`.
    #[must_use]
    pub fn is_lag_frame(&self) -> bool {
        self.islag
    }

    /// `NES.LagCount`.
    #[must_use]
    pub fn lag_count(&self) -> i32 {
        self.lagcount
    }

    /// `NES.Region`.
    #[must_use]
    pub fn region(&self) -> DisplayType {
        self.display_type
    }

    #[must_use]
    pub fn vsync_rate(&self) -> (i32, i32) {
        (self.vsync_num, self.vsync_den)
    }
}
