use crate::apu::Apu2A03;
use crate::cartridges::Cartridge;
use crate::cpu::CpuBus;
use crate::input::InputPorts;
use crate::ppu::Ppu2C02;
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::wide::WideExtension;

/// Size of CPU work RAM.
const RAM_SIZE: usize = 2048;

/// The CPU bus: memory map, open bus, OAM DMA and the master cycle clock (REQ-CORE-080 to
/// REQ-CORE-082).
///
/// Map, after nesdev ("CPU memory map"): $0000-$1FFF the 2 KiB of work RAM, mirrored four
/// times; $2000-$3FFF the eight PPU registers, mirrored every 8 bytes; $4000-$4017 the APU
/// and I/O registers; $4018-$401F disabled; $4020-$FFFF the cartridge.
///
/// Every [`CpuBus::read`] and [`CpuBus::write`] advances the clock by exactly one CPU
/// cycle, during which the PPU gets the dots of the current region and the APU one cycle.
/// Addresses nothing drives return the **open bus**: the last value that travelled on the
/// data bus.
///
/// The bus owns the other chips and the cartridge; the PPU is handed the cartridge on each
/// call that needs it.
#[derive(Debug, Clone)]
pub struct NesBus {
    ram: [u8; RAM_SIZE],
    ppu: Ppu2C02,
    apu: Apu2A03,
    input: InputPorts,
    cartridge: Option<Cartridge>,
    open_bus: u8,
    cycles: i64,
    in_dmc_dma: bool,
    oam_dma_count: i64,
    dmc_dma_count: i64,
    /// CPU cycles the PPU still owes: while `deferred_ppu` is on, the PPU runs in batches,
    /// caught up before anything that could see or change it (see [`NesBus::sync_ppu`]).
    ppu_lag: u32,
    /// The cycle at which the PPU must be caught up whatever happens: the cycle where its
    /// NMI line or the cartridge's IRQ line may next change.
    ppu_deadline: i64,
    deferred_ppu: bool,
}

impl NesBus {
    /// Size of CPU work RAM.
    pub const RAM_SIZE: usize = RAM_SIZE;

    /// Cycles an OAM DMA steals from the CPU, on an even cycle (nesdev, "DMA").
    pub const OAM_DMA_CYCLES_EVEN: i64 = 513;

    /// Cycles an OAM DMA steals from the CPU, on an odd cycle.
    pub const OAM_DMA_CYCLES_ODD: i64 = 514;

    /// Cycles a DMC sample fetch steals from the CPU (nesdev, "DMA").
    pub const DMC_DMA_CYCLES: u32 = 4;

    /// Wires the bus to the other chips.
    #[must_use]
    pub const fn new(ppu: Ppu2C02, apu: Apu2A03, input: InputPorts) -> Self {
        Self {
            ram: [0; RAM_SIZE],
            ppu,
            apu,
            input,
            cartridge: None,
            open_bus: 0,
            cycles: 0,
            in_dmc_dma: false,
            oam_dma_count: 0,
            dmc_dma_count: 0,
            ppu_lag: 0,
            ppu_deadline: 0,
            deferred_ppu: false,
        }
    }

    /// Last value seen on the data bus, returned by unmapped reads.
    #[must_use]
    pub const fn open_bus(&self) -> u8 {
        self.open_bus
    }

    /// CPU work RAM.
    #[must_use]
    pub const fn ram(&self) -> &[u8; RAM_SIZE] {
        &self.ram
    }

    /// Number of OAM DMA transfers performed, for the DMA timing tests.
    #[must_use]
    pub const fn oam_dma_count(&self) -> i64 {
        self.oam_dma_count
    }

    /// Number of DMC sample fetches performed, for the DMA tests.
    #[must_use]
    pub const fn dmc_dma_count(&self) -> i64 {
        self.dmc_dma_count
    }

    /// The PPU.
    #[must_use]
    pub const fn ppu(&self) -> &Ppu2C02 {
        &self.ppu
    }

    /// The PPU, mutably.
    pub fn ppu_mut(&mut self) -> &mut Ppu2C02 {
        &mut self.ppu
    }

    /// The APU.
    #[must_use]
    pub const fn apu(&self) -> &Apu2A03 {
        &self.apu
    }

    /// The APU, mutably.
    pub fn apu_mut(&mut self) -> &mut Apu2A03 {
        &mut self.apu
    }

    /// The controller ports.
    #[must_use]
    pub const fn input(&self) -> &InputPorts {
        &self.input
    }

    /// The controller ports, mutably.
    pub fn input_mut(&mut self) -> &mut InputPorts {
        &mut self.input
    }

    /// The attached cartridge, if any.
    #[must_use]
    pub const fn cartridge(&self) -> Option<&Cartridge> {
        self.cartridge.as_ref()
    }

    /// The attached cartridge, mutably.
    pub fn cartridge_mut(&mut self) -> Option<&mut Cartridge> {
        self.cartridge.as_mut()
    }

    /// The PPU and the cartridge together, for a register access made outside a CPU cycle.
    pub fn ppu_and_cartridge_mut(&mut self) -> (&mut Ppu2C02, Option<&mut Cartridge>) {
        (&mut self.ppu, self.cartridge.as_mut())
    }

    /// Attaches `cartridge`, or detaches it with `None`.
    pub fn attach(&mut self, cartridge: Option<Cartridge>) {
        self.cartridge = cartridge;
    }

    /// Detaches the cartridge and returns it.
    pub fn take_cartridge(&mut self) -> Option<Cartridge> {
        self.cartridge.take()
    }

    /// Clears work RAM and the clock (part of power-on, REQ-CORE-004).
    pub fn reset(&mut self) {
        self.ram.fill(0);
        self.open_bus = 0;
        self.cycles = 0;
        self.in_dmc_dma = false;
        self.oam_dma_count = 0;
        self.dmc_dma_count = 0;
        self.ppu_lag = 0;
        self.ppu_deadline = 0;
    }

    /// Advances the whole machine by one CPU cycle.
    pub fn tick(&mut self) {
        self.cycles += 1;
        self.ppu_lag += 1;
        if self.cycles >= self.ppu_deadline {
            self.catch_up_ppu();
        }

        self.apu.step_cpu_cycle();
        if let Some(cartridge) = self.cartridge.as_mut() {
            cartridge.step_cpu_cycle();
        }

        if self.apu.needs_dmc_dma() && !self.in_dmc_dma {
            self.perform_dmc_dma();
        }
    }

    /// Catches the PPU up before an access that may see or change it. The access itself
    /// may move the NMI line ($2000), the vblank flag ($2002) or the cartridge's IRQ, so
    /// the next cycle is run in step and recomputes the deadline.
    pub fn sync_ppu(&mut self) {
        self.catch_up_ppu();
        self.ppu_deadline = self.cycles + 1;
    }

    /// Runs the cycles the PPU owes, then sets the next deadline.
    ///
    /// Batching is exact because nothing outside the PPU can tell when its dots ran as long
    /// as it is caught up before: every CPU access to $2000-$3FFF, $4014 and the controller
    /// ports (the Zapper looks at the picture), every cartridge write (banks, mirroring),
    /// the cycle where /NMI may change (the vblank flag rises or falls), and every cycle
    /// where the cartridge's IRQ line could follow the PPU (see
    /// [`Cartridge::ppu_irq_horizon`]). Outside [`NesBus::defer_ppu`] the PPU runs on every
    /// cycle, as before.
    fn catch_up_ppu(&mut self) {
        if self.ppu_lag > 0 {
            self.ppu
                .run_cpu_cycles(self.ppu_lag, self.cartridge.as_mut());
            self.ppu_lag = 0;
        }

        self.ppu_deadline = if self.deferred_ppu {
            self.cycles + self.ppu_horizon()
        } else {
            self.cycles + 1
        };
    }

    /// CPU cycles the PPU may lag from now on, at least 1.
    fn ppu_horizon(&self) -> i64 {
        // The PPU's sample of /NMI lags its line by a dot or two: when they differ, the
        // next cycle must be run in step.
        if self.ppu.nmi_line_sample() != self.ppu.nmi_asserted() {
            return 1;
        }

        // A cycle is at most four dots (PAL averages 3.2); a margin of two cycles covers
        // the dot or two by which the sample trails and the odd frame's skipped dot.
        let nmi = self.ppu.dots_until_nmi_change() / 4 - 2;
        let irq = self.cartridge.as_ref().map_or(i64::MAX, |cartridge| {
            match cartridge.ppu_irq_horizon() {
                Some(dots) => dots / 4 - 2,
                None => 0,
            }
        });
        nmi.min(irq).max(1)
    }

    /// Lets the PPU run in batches until [`NesBus::end_deferred_ppu`] (the frame loop).
    pub fn defer_ppu(&mut self) {
        self.deferred_ppu = true;
        self.sync_ppu();
    }

    /// Catches the PPU up and returns to running it on every cycle.
    pub fn end_deferred_ppu(&mut self) {
        self.deferred_ppu = false;
        self.sync_ppu();
    }

    /// Fetches one DMC sample byte, halting the CPU for [`NesBus::DMC_DMA_CYCLES`] cycles
    /// (nesdev, "DMA": the DMC's memory reader takes the bus for up to four cycles, the
    /// last of which is the read).
    ///
    /// The halt is placed on the cycle the APU asked for the byte rather than on the next
    /// cycle the CPU could actually be stopped at, so a fetch that collides with a read of
    /// $2007 or $4016 does not reproduce the hardware's double read. Those collisions are
    /// what `dmc_dma_during_read4` and `sprdma_and_dmc_dma` measure; REQ-CORE-113 makes
    /// them P3.
    fn perform_dmc_dma(&mut self) {
        self.in_dmc_dma = true;
        self.dmc_dma_count += 1;

        for _ in 1..Self::DMC_DMA_CYCLES {
            self.tick();
        }

        self.tick();
        let value = self.read_internal(self.apu.dmc_dma_address());
        self.open_bus = value;
        self.apu.supply_dmc_byte(value);

        self.in_dmc_dma = false;
    }

    /// Reads controller port `port` ($4016 or $4017) with its side effects, the Zapper's light
    /// detection included (REQ-CORE-061, REQ-CORE-062); this is the reference's
    /// `InputPorts.Read` with its light probe wired to the PPU.
    pub fn read_input_port(&mut self, port: usize, open_bus: u8) -> u8 {
        self.sync_ppu();
        let ppu = &self.ppu;
        self.input
            .read(port, open_bus, |aim| ppu.sees_light_at(aim.x, aim.y))
    }

    fn read_internal(&mut self, address: u16) -> u8 {
        if address < 0x2000 {
            return self.ram[usize::from(address) & (RAM_SIZE - 1)];
        }

        if address < 0x4000 {
            self.sync_ppu();
            // The PPU drives its own data-bus latch for the undefined bits, separate from
            // the CPU's open bus (nesdev, "PPU registers"; ppu_open_bus).
            return self.ppu.read(address & 0x07, self.cartridge.as_mut());
        }

        if address < 0x4020 {
            return match address {
                0x4015 => self.apu.read_status(self.open_bus),
                0x4016 => self.read_input_port(0, self.open_bus),
                0x4017 => self.read_input_port(1, self.open_bus),

                // $4000-$4014 are write-only and $4018-$401F are disabled: open bus.
                _ => self.open_bus,
            };
        }

        if WideExtension::claims(address) {
            if let Some(value) = self.ppu.wide().read(address) {
                return value;
            }
        }

        self.cartridge
            .as_mut()
            .and_then(|cartridge| cartridge.try_cpu_read(address))
            .unwrap_or(self.open_bus)
    }

    fn write_internal(&mut self, address: u16, value: u8) {
        if address < 0x2000 {
            self.ram[usize::from(address) & (RAM_SIZE - 1)] = value;
            return;
        }

        if address < 0x4000 {
            self.sync_ppu();
            self.ppu
                .write(address & 0x07, value, self.cartridge.as_mut());
            return;
        }

        if address < 0x4020 {
            match address {
                0x4014 => self.perform_oam_dma(value),
                0x4016 => self.input.write(value),
                _ => self.apu.write(address, value),
            }
            return;
        }

        self.sync_ppu();
        if WideExtension::claims(address) {
            // The window only listens: the cartridge still gets the write.
            self.ppu.wide_mut().write(address, value);
        }

        if let Some(cartridge) = self.cartridge.as_mut() {
            cartridge.cpu_write(address, value);
        }
    }

    /// Copies a 256-byte page into OAM and halts the CPU for 513 cycles, 514 when the
    /// write landed on an odd CPU cycle (nesdev, "DMA").
    ///
    /// The write cycle itself is already counted by [`CpuBus::write`]; the cycles below
    /// are the ones stolen from the CPU on top of it.
    fn perform_oam_dma(&mut self, page: u8) {
        self.oam_dma_count += 1;

        // Parity is read at the $4014 write, before the wait state.
        let odd_cycle = (self.cycles & 1) != 0;

        self.tick(); // Wait state while the DMA unit takes the bus.

        if odd_cycle {
            self.tick(); // Alignment cycle: the transfer only starts on a "get" cycle.
        }

        let base_address = u16::from(page) << 8;
        for i in 0..256u16 {
            self.tick();
            let value = self.read_internal(base_address.wrapping_add(i));
            self.open_bus = value;

            self.tick();
            self.sync_ppu();
            self.ppu.write_oam_dma(value);
        }

        self.ppu.wide_mut().latch_sprites();
    }

    /// Writes a byte exactly as a CPU write would, without advancing emulated time
    /// (REQ-CORE-082).
    pub fn poke(&mut self, address: u16, value: u8) {
        self.open_bus = value;
        self.write_internal(address, value);
    }
}

impl CpuBus for NesBus {
    fn read(&mut self, address: u16) -> u8 {
        self.tick();
        let value = self.read_internal(address);
        self.open_bus = value;
        value
    }

    fn write(&mut self, address: u16, value: u8) {
        self.tick();
        self.open_bus = value;
        self.write_internal(address, value);
    }

    /// Reads a byte with no side effect and without advancing time (REQ-CORE-080,
    /// REQ-CORE-081).
    ///
    /// $2000-$401F reads back as 0, as REQ-CORE-081 asks, so that a host or a script
    /// peeking at $2002 cannot clear the `VBlank` flag under the game's feet.
    fn peek(&self, address: u16) -> u8 {
        if address < 0x2000 {
            return self.ram[usize::from(address) & (RAM_SIZE - 1)];
        }

        if address < 0x4020 {
            return 0;
        }

        if WideExtension::claims(address) {
            if let Some(value) = self.ppu.wide().read(address) {
                return value;
            }
        }

        self.cartridge
            .as_ref()
            .and_then(|cartridge| cartridge.try_cpu_peek(address))
            .unwrap_or(0)
    }

    /// The CPU sees the /NMI line as it stood near the start of the cycle, not after the
    /// cycle's own register access (see `Ppu2C02::nmi_line_sample`).
    fn nmi_asserted(&self) -> bool {
        self.ppu.nmi_line_sample()
    }

    fn irq_asserted(&self) -> bool {
        self.apu.irq_asserted() || self.cartridge.as_ref().is_some_and(Cartridge::irq_asserted)
    }

    fn cycle_count(&self) -> i64 {
        self.cycles
    }
}

impl NesBus {
    /// Appends the bus state (work RAM, open bus, clock and DMA counters) to `writer`
    /// (REQ-CORE-090). The chips and the cartridge the bus owns save themselves.
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_blob(&self.ram);
        writer.write_u8(self.open_bus);
        writer.write_i64(self.cycles);
        writer.write_i64(self.oam_dma_count);
        writer.write_i64(self.dmc_dma_count);
    }

    /// Restores the bus state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or its RAM block has the wrong size.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        reader.read_blob_into(&mut self.ram)?;
        self.open_bus = reader.read_u8()?;
        self.cycles = reader.read_i64()?;
        self.oam_dma_count = reader.read_i64()?;
        self.dmc_dma_count = reader.read_i64()?;
        self.in_dmc_dma = false;
        self.ppu_lag = 0;
        self.ppu_deadline = 0;
        Ok(())
    }
}
