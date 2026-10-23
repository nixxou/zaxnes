use crate::apu::Apu2A03;
use crate::cartridges::Cartridge;
use crate::cpu::{Cpu6502, CpuBus};
use crate::input::InputPorts;
use crate::ppu::Ppu2C02;
use crate::rom::{RomImage, RomLoadResult};
use crate::{FrameBatch, NametableMirroring, NesBus, NesRegion, RegionTiming, TileRecord};

/// The machine without its host-facing services: CPU, bus, ROM loading, power-on and "run
/// one frame" (the part of `NesCore` the PPU goldens need, REQ-CORE-003, REQ-CORE-004,
/// REQ-CORE-020). The `NesCore` facade wraps it.
#[derive(Debug, Clone)]
pub struct Machine {
    cpu: Cpu6502,
    bus: NesBus,
    region: NesRegion,
    battery_ram_imported: bool,
    frame_count: i64,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    /// Creates a machine with no ROM loaded, in NTSC.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu: Cpu6502::new(),
            bus: NesBus::new(Ppu2C02::new(), Apu2A03::new(), InputPorts::new()),
            region: NesRegion::Ntsc,
            battery_ram_imported: false,
            frame_count: 0,
        }
    }

    /// Loads a ROM image (REQ-CORE-010) and powers the machine on. On failure the instance
    /// is left stopped (REQ-CORE-011, Q-002). The detected region is returned, not applied
    /// (REQ-CORE-013).
    pub fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult {
        let cartridge = match RomImage::try_load(image, name).and_then(Cartridge::create) {
            Ok(cartridge) => cartridge,
            Err(error) => {
                self.close_rom();
                return RomLoadResult::fail(error.failure, error.message);
            }
        };

        let region = cartridge.rom().announced_region();
        self.battery_ram_imported = false;
        self.bus.attach(Some(cartridge));
        self.power_on();
        RomLoadResult::ok(region)
    }

    /// Stops the emulation (REQ-CORE-005).
    pub fn close_rom(&mut self) {
        self.battery_ram_imported = false;
        self.bus.attach(None);
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.bus.cartridge().is_some()
    }

    /// Name the ROM was loaded under (REQ-CORE-014).
    #[must_use]
    pub fn rom_name(&self) -> Option<&str> {
        self.bus.cartridge().map(|cartridge| cartridge.rom().name())
    }

    /// Resets the machine, which is equivalent to a power cycle (REQ-CORE-004, DEC-02-Q3).
    pub fn reset(&mut self) {
        if self.bus.cartridge().is_some() {
            self.power_on();
        }
    }

    /// Marks PRG-RAM as imported battery RAM, so that a reset keeps it (REQ-CORE-095).
    pub fn set_battery_ram_imported(&mut self, imported: bool) {
        self.battery_ram_imported = imported;
    }

    fn power_on(&mut self) {
        // Q-003: the region goes in first, so the PPU's power-on position is the start of
        // vblank of the region in force (REQ-CORE-015, REQ-CORE-020 notes).
        self.apply_region();

        self.bus.reset();
        self.bus.ppu_mut().reset();
        let clear_prg_ram = !self.battery_ram_imported;
        if let Some(cartridge) = self.bus.cartridge_mut() {
            cartridge.reset();
            cartridge.clear_volatile_ram(clear_prg_ram);
        }

        self.cpu.reset(&mut self.bus);
        self.frame_count = 0;
    }

    /// TV region in force (REQ-CORE-016).
    #[must_use]
    pub const fn region(&self) -> NesRegion {
        self.region
    }

    /// Changes the region; takes effect on the next frame, without a reset (REQ-CORE-016).
    pub fn set_region(&mut self, region: NesRegion) {
        self.region = region;
        self.apply_region();
    }

    fn apply_region(&mut self) {
        self.bus
            .ppu_mut()
            .set_timing(RegionTiming::for_region(self.region));
    }

    /// Runs one frame: from where the previous one stopped to the start of the next vblank
    /// (REQ-CORE-020). Returns the CPU cycles it took, 0 when no ROM is loaded
    /// (REQ-CORE-005).
    pub fn run_frame(&mut self) -> i64 {
        if self.bus.cartridge().is_none() {
            return 0;
        }

        let start_cycle = self.bus.cycle_count();

        self.bus.ppu_mut().clear_frame_boundary();
        loop {
            self.cpu.step(&mut self.bus);
            if self.bus.ppu().vblank_just_started() {
                break;
            }
        }

        self.frame_count += 1;
        self.bus.cycle_count() - start_cycle
    }

    /// The batch of the frame just run (REQ-CORE-030).
    #[must_use]
    pub fn frame_batch(&self) -> FrameBatch<'_> {
        let ppu = self.bus.ppu();
        FrameBatch {
            records: ppu.tile_records(),
            palette: ppu.snapshot_palette(),
            mask_x: ppu.mask_x(),
            mask_y: ppu.mask_y(),
        }
    }

    /// The 256 x 240 indexed image of the frame just run (REQ-CORE-048).
    #[must_use]
    pub fn pixel_frame(&self) -> &[u8] {
        self.bus.ppu().pixel_frame()
    }

    /// The tile records of the frame just run.
    #[must_use]
    pub fn tile_records(&self) -> &[TileRecord] {
        self.bus.ppu().tile_records()
    }

    /// Frames run since the last power-on.
    #[must_use]
    pub const fn frame_count(&self) -> i64 {
        self.frame_count
    }

    /// CPU cycles since the last power-on.
    #[must_use]
    pub fn cycle_count(&self) -> i64 {
        self.bus.cycle_count()
    }

    /// Reads CPU memory without side effects (REQ-CORE-080, REQ-CORE-081).
    #[must_use]
    pub fn read_memory(&self, address: u16) -> u8 {
        self.bus.peek(address)
    }

    /// Writes CPU memory as a CPU write would, without advancing time (REQ-CORE-082).
    pub fn write_memory(&mut self, address: u16, value: u8) {
        self.bus.poke(address, value);
    }

    /// The nametable layout the cartridge currently selects.
    #[must_use]
    pub fn mirroring(&self) -> NametableMirroring {
        Ppu2C02::mirroring_of(self.bus.cartridge())
    }

    /// Offset in VRAM of nametable address `address` under the current layout.
    #[must_use]
    pub fn map_nametable(&self, address: u16) -> usize {
        Ppu2C02::map_nametable(address, self.mirroring())
    }

    /// Reads a PPU register with its side effects, as the CPU would, without advancing
    /// time.
    pub fn ppu_read(&mut self, register: u16) -> u8 {
        let (ppu, cartridge) = self.bus.ppu_and_cartridge_mut();
        ppu.read(register, cartridge)
    }

    /// The CPU.
    #[must_use]
    pub const fn cpu(&self) -> &Cpu6502 {
        &self.cpu
    }

    /// The CPU and the bus, mutably, to step instructions by hand.
    pub fn cpu_and_bus_mut(&mut self) -> (&mut Cpu6502, &mut NesBus) {
        (&mut self.cpu, &mut self.bus)
    }

    /// The bus.
    #[must_use]
    pub const fn bus(&self) -> &NesBus {
        &self.bus
    }

    /// The bus, mutably.
    pub fn bus_mut(&mut self) -> &mut NesBus {
        &mut self.bus
    }

    /// The PPU.
    #[must_use]
    pub const fn ppu(&self) -> &Ppu2C02 {
        self.bus.ppu()
    }

    /// The cartridge, if a ROM is loaded.
    #[must_use]
    pub const fn cartridge(&self) -> Option<&Cartridge> {
        self.bus.cartridge()
    }

    /// The cartridge, mutably.
    pub fn cartridge_mut(&mut self) -> Option<&mut Cartridge> {
        self.bus.cartridge_mut()
    }
}
