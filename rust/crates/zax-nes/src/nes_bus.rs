use crate::apu::Apu2A03;
use crate::cartridges::Cartridge;
use crate::cpu::CpuBus;
use crate::input::InputPorts;
use crate::ppu::Ppu2C02;
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::wide::WideExtension;

/// Size of CPU work RAM.
const RAM_SIZE: usize = 2048;

/// [`NesBus::dma`]: a DMA unit is still owed its **halt** cycle. Outside [`NesBus::run_dma`] a
/// non-zero `dma` always has this bit, which is what lets [`CpuBus::read`] gate the whole DMA
/// path on `dma != 0` — one `cmpb` — rather than on a mask (REQ-CORE-113).
const DMA_HALT: u8 = 0x01;

/// [`NesBus::dma`]: the DMC's **dummy** cycle is still owed. The memory reader needs a halt and
/// a dummy cycle before it may read, which is what makes a lone DMC fetch three or four cycles
/// rather than two (nesdev, "DMA").
const DMA_DUMMY: u8 = 0x02;

/// [`NesBus::dma`]: a DMC sample fetch is pending or running.
const DMA_DMC: u8 = 0x04;

/// [`NesBus::dma`]: an OAM transfer is pending or running.
const DMA_OAM: u8 = 0x08;

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
#[allow(clippy::struct_excessive_bools)] // Independent flags of the chips and of the batching.
pub struct NesBus {
    ram: [u8; RAM_SIZE],
    ppu: Ppu2C02,
    apu: Apu2A03,
    input: InputPorts,
    cartridge: Option<Cartridge>,
    open_bus: u8,
    cycles: i64,
    /// What the two DMA units still owe: [`DMA_HALT`], [`DMA_DUMMY`], [`DMA_DMC`], [`DMA_OAM`].
    /// Zero while neither unit wants the bus, which is the state of every cycle of a game that
    /// plays no DPCM sample and writes no `$4014` (see [`NesBus::run_dma`]).
    dma: u8,
    /// Page the pending OAM transfer copies from: the byte written to `$4014`.
    dma_oam_page: u8,
    oam_dma_count: i64,
    dmc_dma_count: i64,
    /// CPU cycles the PPU still owes: while `deferred_ppu` is on, the PPU runs in batches,
    /// caught up before anything that could see or change it (see [`NesBus::sync_ppu`]).
    ppu_lag: u32,
    /// The cycle at which the PPU must be caught up whatever happens: the cycle where its
    /// NMI line or the cartridge's IRQ line may next change.
    ppu_deadline: i64,
    deferred_ppu: bool,
    /// CPU cycles the APU still owes: it is batched along with the PPU, and caught up before
    /// anything that could see or change it (see [`NesBus::catch_up_apu`]).
    apu_lag: u32,
    /// How far `apu_lag` may grow: the APU's quiet cycles when it was last caught up.
    apu_quiet: u32,
    /// Whether the cartridge has expansion audio, asked once in [`NesBus::attach`]. For such a
    /// board the APU is never let lag (`apu_quiet` stays 0) and its `M` is pushed in before the
    /// cycle it belongs to is run (REQ-CORE-073, REQ-CORE-075). Only the MMC5 sets it; it is
    /// never read on the per-cycle path, `apu_batching` standing in for it there.
    expansion_audio: bool,
    /// `deferred_ppu && !expansion_audio`: whether the APU may lag at all, kept as one flag so
    /// that the per-cycle catch-up tests it **instead of** `deferred_ppu` rather than on top of
    /// it. Everything the MMC5's audio needs sits on the other side of that single test, out of
    /// line, so a board without expansion audio pays exactly what it paid before AMD-054
    /// (see [`NesBus::catch_up_apu`]).
    apu_batching: bool,
    /// Whether the instruction being run was fetched from the extension's execution zone: its
    /// data accesses at `$3000`-`$3FFF` reach the extension when the zone holds its data there
    /// (`WideExtension::zone_data`), the game's still reach the PPU.
    zone_instruction: bool,
}

impl NesBus {
    /// Size of CPU work RAM.
    pub const RAM_SIZE: usize = RAM_SIZE;

    /// Cycles an OAM DMA steals from the CPU, on an even cycle (nesdev, "DMA").
    pub const OAM_DMA_CYCLES_EVEN: i64 = 513;

    /// Cycles an OAM DMA steals from the CPU, on an odd cycle.
    pub const OAM_DMA_CYCLES_ODD: i64 = 514;

    /// Cycles a lone DMC sample fetch steals from the CPU when it needs its alignment cycle
    /// (nesdev, "DMA": "DMC DMA normally takes 3 or 4 cycles, depending on whether alignment is
    /// needed").
    pub const DMC_DMA_CYCLES: u32 = 4;

    /// Cycles a lone DMC sample fetch steals when the halt already landed so that the cycle
    /// after the dummy read is a get cycle.
    pub const DMC_DMA_CYCLES_ALIGNED: u32 = 3;

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
            dma: 0,
            dma_oam_page: 0,
            oam_dma_count: 0,
            dmc_dma_count: 0,
            ppu_lag: 0,
            ppu_deadline: 0,
            deferred_ppu: false,
            apu_lag: 0,
            expansion_audio: false,
            apu_batching: false,
            apu_quiet: 0,
            zone_instruction: false,
        }
    }

    /// Last value seen on the data bus, returned by unmapped reads.
    #[must_use]
    pub const fn open_bus(&self) -> u8 {
        self.open_bus
    }

    /// CPU work RAM, for writing.
    pub fn ram_mut(&mut self) -> &mut [u8; RAM_SIZE] {
        &mut self.ram
    }

    /// CPU RAM.
    #[must_use]
    pub const fn ram(&self) -> &[u8; RAM_SIZE] {
        &self.ram
    }

    /// Number of OAM transfers asked for by a `$4014` write, for the DMA timing tests. The
    /// transfer itself starts on the next cycle the CPU reads on (see [`NesBus::run_dma`]).
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

    /// The APU, mutably, caught up first.
    pub fn apu_mut(&mut self) -> &mut Apu2A03 {
        self.sync_apu();
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
        self.expansion_audio = cartridge
            .as_ref()
            .is_some_and(Cartridge::has_expansion_audio);
        self.apu_batching = self.deferred_ppu && !self.expansion_audio;
        if !self.expansion_audio {
            // A board with no expansion audio leaves the APU's `M` at 0 for good.
            self.apu.set_expansion_level(0);
        }
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
        self.dma = 0;
        self.dma_oam_page = 0;
        self.oam_dma_count = 0;
        self.dmc_dma_count = 0;
        self.ppu_lag = 0;
        self.ppu_deadline = 0;
        self.apu_lag = 0;
        self.apu_quiet = 0;
    }

    /// Advances the whole machine by one CPU cycle.
    pub fn tick(&mut self) {
        // Widescreen extension overclock: the PPU and the APU stay still for this cycle. The
        // cartridge still sees it: MMC1 tells consecutive writes apart by their cycle, and a
        // bank switch is five writes in a row.
        if self.ppu.wide_mut().take_stall_cycle() {
            if let Some(cartridge) = self.cartridge.as_mut() {
                cartridge.step_cpu_cycle();
            }
            return;
        }

        self.cycles += 1;
        self.ppu_lag += 1;
        if self.cycles >= self.ppu_deadline {
            self.catch_up_ppu();
        }

        // The cartridge goes before the APU: a board with expansion audio (REQ-CORE-075) has
        // to have produced this cycle's `M` before the APU mixes it in (REQ-CORE-073). Nothing
        // else in the tick depends on the order — the two chips do not look at each other.
        //
        // A DMC fetch request is **not** noticed here: the halt may only land on a cycle the
        // CPU reads on, so the question is asked by [`NesBus::poll_dmc_dma`] where the APU
        // runs and answered by [`CpuBus::read`] (MF-4a step 1).
        if let Some(cartridge) = self.cartridge.as_mut() {
            cartridge.step_cpu_cycle();
        }

        self.apu_lag += 1;
        if self.apu_lag > self.apu_quiet {
            self.catch_up_apu();
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

    /// Runs the cycles the APU owes, then sets how many more it may owe.
    ///
    /// Batching is exact because the cycles it skips are quiet (see
    /// [`Apu2A03::quiet_cycles`]): on them the APU's IRQ line, its DMC fetch request and
    /// its output stay as they are, so the bus may look at those without catching it up.
    /// Everything else waits for [`NesBus::sync_apu`]: every access to $4000-$401F but
    /// $4014 and $4016, the DMC fetch's byte, and the end of the frame loop. Outside
    /// [`NesBus::defer_ppu`] the APU runs on every cycle, as before.
    ///
    /// This is a per-cycle path: it is inlined into [`NesBus::tick`] and runs on about half the
    /// CPU cycles of a frame (14 894 of 29 780, measured on `nestest`), so one instruction here
    /// is about 15 000 per emulated frame. Everything the MMC5's expansion audio needs therefore
    /// sits behind the **single** `apu_batching` test, which replaces the `deferred_ppu` test
    /// this function had before AMD-054 rather than adding to it, in a function of its own that
    /// is never inlined — so neither the test nor the register pressure of that code reaches
    /// `tick` for a board without expansion audio (REQ-CORE-075's "no cost for other boards").
    fn catch_up_apu(&mut self) {
        if !self.apu_batching {
            // Either the board has expansion audio, or the PPU is not being batched at all.
            // Both mean the APU may not lag past this cycle, which is what `sync_apu` does.
            self.sync_apu();
            return;
        }

        if self.apu_lag > 0 {
            self.apu.run_cpu_cycles(self.apu_lag);
            self.apu_lag = 0;
        }
        self.apu_quiet = self.apu.quiet_cycles();
        self.poll_dmc_dma();
    }

    /// Arms the DMC's sample fetch when its memory reader has run dry.
    ///
    /// This is the question [`NesBus::tick`] used to ask on every CPU cycle. It belongs here
    /// instead: a quiet cycle is one on which "the DMC's fetch request stays as it is" (see
    /// [`Apu2A03::quiet_cycles`]), so the request can only have risen on a cycle the APU just
    /// ran — the cycles this is reached on. The `DMA_DMC` test only runs once the first one has
    /// said yes, so a game with no sample playing pays one load and one not-taken branch, which
    /// is exactly what the test it replaces in `tick` cost (MF-4a step 1).
    fn poll_dmc_dma(&mut self) {
        if self.apu.needs_dmc_dma() && (self.dma & DMA_DMC) == 0 {
            // A halt and a dummy cycle come before the fetch itself (nesdev, "DMA").
            self.dma |= DMA_DMC | DMA_DUMMY | DMA_HALT;
        }
    }

    /// Catches the APU up before an access that may see or change it; the next cycle is
    /// run in step and recomputes how far it may lag. It is also the whole of the catch-up of a
    /// cycle the APU may not lag past, which is where [`NesBus::catch_up_apu`] sends a board
    /// with expansion audio and any board the PPU is not being batched for.
    ///
    /// It is inlined into [`NesBus::tick`] by two chains — through [`NesBus::catch_up_apu`] and
    /// through [`NesBus::perform_dmc_dma`] — which is why the `M` push below is the part kept out
    /// of line and not this one: a board with expansion audio runs this on every cycle and must
    /// not pay a call for it.
    fn sync_apu(&mut self) {
        if self.apu_lag > 0 {
            if self.expansion_audio {
                self.push_expansion_audio();
            }
            self.apu.run_cpu_cycles(self.apu_lag);
            self.apu_lag = 0;
        }
        self.apu_quiet = 0;
        self.poll_dmc_dma();
    }

    /// Hands the APU the board's `M` for the cycles it is about to run (REQ-CORE-073,
    /// REQ-CORE-075). The caller tests `expansion_audio` itself, so this body only ever runs
    /// for the MMC5.
    ///
    /// Kept **out of line**, which is what makes RU-3a's expansion audio free for every other
    /// board: inlined, its cartridge `Option`, its devirtualised `expansion_audio_output` and
    /// its `set_expansion_level` reach [`NesBus::tick`] through the two chains above, raise the
    /// register pressure of the whole function and cost every CPU cycle of every board one more
    /// saved register — RU-3a's +15.7 % on `tick`.
    #[inline(never)]
    fn push_expansion_audio(&mut self) {
        if let Some(level) = self
            .cartridge
            .as_ref()
            .and_then(Cartridge::expansion_audio_output)
        {
            self.apu.set_expansion_level(level);
        }
    }

    /// Lets the PPU run in batches until [`NesBus::end_deferred_ppu`] (the frame loop).
    pub fn defer_ppu(&mut self) {
        self.deferred_ppu = true;
        self.apu_batching = !self.expansion_audio;
        self.sync_ppu();
        self.sync_apu();
    }

    /// Catches the PPU up and returns to running it on every cycle.
    pub fn end_deferred_ppu(&mut self) {
        self.deferred_ppu = false;
        self.apu_batching = false;
        self.sync_ppu();
        self.sync_apu();
    }

    /// Runs the cycles the DMA units are owed, and the transfers they serve, in place of the
    /// cycle on which the CPU was about to read `read_address` (nesdev, "DMA"; REQ-CORE-113).
    ///
    /// Three rules of the page shape all of it:
    ///
    /// * **the halt lands on a read cycle only.** "DMA halts the CPU only on read cycles. If
    ///   the CPU is writing, it ignores the halt and the DMA unit waits until the next cycle to
    ///   try again." That is why this is reached from [`CpuBus::read`] and not from
    ///   [`NesBus::tick`]: a write simply does not call it, and the DMA waits.
    /// * **the halt cycle repeats the CPU's own read.** The CPU is stopped with its address on
    ///   the bus, so the access happens twice — the extra `$4016` read of
    ///   `dmc_dma_during_read4/dma_4016_read` and the extra `$2007` reads of `dma_2007_read`.
    /// * **get and put cycles.** "The CPU alternates between cycles on which DMA can get (read)
    ///   and cycles on which DMA can put (write). These are the first and second halves of APU
    ///   cycles." A unit that wants the wrong half wastes an alignment cycle. In this core's
    ///   numbering a get cycle is an **even** one, which is the convention the `$4017` write
    ///   delay already uses (`apu_reset/4017_timing`, `apu_test/4-jitter`) and which gives the
    ///   513 / 514 cycles of an OAM transfer unchanged.
    ///
    /// When both units want the bus the DMC wins: "When accesses collide, DMC DMA is allowed to
    /// run and OAM DMA is paused, trying again on the next cycle", and an OAM transfer's own
    /// cycles stand in for the DMC's halt and dummy cycles.
    ///
    /// Adapted from Mesen 2's `NesCpu::ProcessPendingDma`, which is the same reading of that
    /// page.
    ///
    /// **Out of line, measured.** Inlined into [`NesBus::read_cycle`] — a function entered on
    /// every read cycle of every game — this body makes the prologue save eight general
    /// registers and two XMM registers and reserve 88 bytes of stack, which cost **+5 % of wall
    /// clock** on an NROM and an MMC1 that never run a DMA at all. It is the same trap
    /// `push_expansion_audio` above carries its own note about.
    ///
    /// What the three shapes tried here cost, and the residual the measured one leaves, are in
    /// the ticket.
    // Q-119
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    #[inline(never)]
    fn run_dma(&mut self, read_address: u16) {
        // Only the **first** idle read of $4016/$4017 is a read as far as the controller is
        // concerned: /OE stays active for the cycles that follow, so the shift register is
        // clocked once however many idle cycles the DMA spends (nesdev, "DMA"; the original
        // Famicom differs and is not emulated here).
        let input_port = matches!(read_address, 0x4016 | 0x4017);
        // The CPU halted in $4000-$401F drives its own address decoder for the whole DMA, so
        // the unit's reads reach the 2A03's registers as well (see [`NesBus::dma_read`]).
        let internal = (read_address & 0xFFE0) == 0x4000;
        // When the DMC is about to read an address that decodes to the very register the CPU
        // was reading, /OE never goes inactive and the controller sees nothing of the halt.
        let halt_is_invisible = internal
            && input_port
            && (self.dma & DMA_DMC) != 0
            && (self.apu.dmc_dma_address() & 0x1F) == (read_address & 0x1F);

        // The register the 2A03's decoder last answered, for the "two reads in a row" rule.
        let mut decoded_before = read_address;

        self.dma &= !DMA_HALT;
        self.tick();
        if !halt_is_invisible {
            let value = self.read_internal(read_address);
            self.open_bus = value;
        }

        // Bytes read and written so far, counted the way the hardware's pair counter is: even
        // means the next get cycle reads, odd means the next put cycle writes.
        let mut oam_steps: u16 = 0;
        let mut oam_index: u8 = 0;
        let mut oam_byte: u8 = 0;

        while (self.dma & (DMA_DMC | DMA_OAM)) != 0 {
            let get_cycle = (self.cycles & 1) != 0;

            if get_cycle && (self.dma & (DMA_HALT | DMA_DUMMY | DMA_DMC)) == DMA_DMC {
                // The memory reader has had its halt and its dummy cycle: it reads.
                self.dma_cycle();
                self.dmc_dma_count += 1;
                let address = self.apu.dmc_dma_address();
                let value = self.dma_read(address, internal, &mut decoded_before);
                self.sync_apu();
                self.apu.supply_dmc_byte(value);
                // Cleared last: the `sync_apu` above polls the request, which is still raised
                // until the byte lands.
                self.dma &= !DMA_DMC;
            } else if get_cycle && (self.dma & DMA_OAM) != 0 {
                self.dma_cycle();
                let address = (u16::from(self.dma_oam_page) << 8) | u16::from(oam_index);
                oam_byte = self.dma_read(address, internal, &mut decoded_before);
                oam_index = oam_index.wrapping_add(1);
                oam_steps += 1;
            } else if !get_cycle && (self.dma & DMA_OAM) != 0 && (oam_steps & 1) != 0 {
                self.dma_cycle();
                self.sync_ppu();
                self.ppu.write_oam_dma(oam_byte);
                oam_steps += 1;
                if oam_steps == 0x200 {
                    self.dma &= !DMA_OAM;
                    self.ppu.wide_mut().latch_sprites();
                }
            } else {
                // A cycle no unit can use: the DMC's dummy read, or an alignment before a get.
                // The CPU's address is still on the bus, so its read happens again.
                self.dma_cycle();
                if !input_port {
                    let value = self.read_internal(read_address);
                    self.open_bus = value;
                }
            }
        }
    }

    /// Runs one DMA cycle, consuming the halt or the dummy cycle the DMC may still be owed: an
    /// OAM transfer's cycles count as those when both units run at once (nesdev, "DMA").
    fn dma_cycle(&mut self) {
        if (self.dma & DMA_HALT) != 0 {
            self.dma &= !DMA_HALT;
        } else if (self.dma & DMA_DUMMY) != 0 {
            self.dma &= !DMA_DUMMY;
        }
        self.tick();
    }

    /// One DMA unit's read of `address`.
    ///
    /// `internal` is "the CPU was halted while reading $4000-$401F": its address decoder is then
    /// still driven from the low five bits of whatever the DMA unit puts on the bus, so the read
    /// also reaches one of the 2A03's own registers — which can clear the frame IRQ flag behind
    /// the program's back, lose a controller bit, or replace the sample byte altogether. The
    /// 2A03 alone has the bug; `decoded_before` carries the "two reads of the same input
    /// register in a row" rule, on which the controller does not react at all.
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    fn dma_read(&mut self, address: u16, internal: bool, decoded_before: &mut u16) -> u8 {
        if !internal {
            // Nothing answers $4000-$401F on the external bus.
            let value = if (0x4000..0x4020).contains(&address) {
                self.open_bus
            } else {
                self.read_internal(address)
            };
            self.open_bus = value;
            *decoded_before = address;
            return value;
        }

        let decoded = 0x4000 | (address & 0x1F);
        let same = decoded == address;
        let value = match decoded {
            0x4015 => {
                let value = self.read_internal(decoded);
                if !same {
                    self.read_internal(address);
                }
                value
            }
            0x4016 | 0x4017 => {
                let value = if *decoded_before == decoded {
                    self.open_bus
                } else {
                    self.read_internal(decoded)
                };
                if same {
                    value
                } else {
                    // Two drivers on the data bus at once: the byte is the AND of both.
                    self.read_internal(address) & value
                }
            }
            _ => self.read_internal(address),
        };
        self.open_bus = value;
        *decoded_before = decoded;
        value
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
        if address >= 0x8000 {
            if let Some(value) = self
                .cartridge
                .as_ref()
                .and_then(|cartridge| cartridge.read_rom_window(address))
            {
                return value;
            }
        }

        if address < 0x2000 {
            return self.ram[usize::from(address) & (RAM_SIZE - 1)];
        }

        if address < 0x4000 {
            // The extension's registers, as a read of `$5xxx` reaches them (no PPU catch-up).
            if let Some(register) = self.zone_data_address(address) {
                return self.ppu.wide().read(register).unwrap_or(self.open_bus);
            }
            self.sync_ppu();
            // The PPU drives its own data-bus latch for the undefined bits, separate from
            // the CPU's open bus (nesdev, "PPU registers"; ppu_open_bus).
            return self.ppu.read(address & 0x07, self.cartridge.as_mut());
        }

        if address < 0x4020 {
            return match address {
                0x4015 => {
                    self.sync_apu();
                    self.apu.read_status(self.open_bus)
                }
                0x4016 => self.read_input_port(0, self.open_bus),
                0x4017 => self.read_input_port(1, self.open_bus),

                // $4000-$4014 are write-only and $4018-$401F are disabled: open bus.
                _ => self.open_bus,
            };
        }

        if WideExtension::claims(address) && !self.ppu.wide().zone_data() {
            if let Some(value) = self.wide_window_read(address) {
                return value;
            }
            if let Some(value) = self.ppu.wide().read(address) {
                return value;
            }
        }
        if (0x6000..0x8000).contains(&address) {
            if let Some(value) = self.wide_window_read(address) {
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
            if let Some(register) = self.zone_data_address(address) {
                self.ppu.wide_mut().write(register, value);
                return;
            }
            self.ppu
                .write(address & 0x07, value, self.cartridge.as_mut());
            return;
        }

        if address < 0x4020 {
            match address {
                0x4014 => self.queue_oam_dma(value),
                0x4016 => self.input.write(value),
                _ => {
                    self.sync_apu();
                    self.apu.write(address, value);
                    self.after_apu_write();
                }
            }
            return;
        }

        self.sync_ppu();
        if WideExtension::claims(address) && !self.ppu.wide().zone_data() {
            // The window only listens: the cartridge still gets the write.
            self.ppu.wide_mut().write(address, value);
        }

        if let Some(cartridge) = self.cartridge.as_mut() {
            cartridge.cpu_write(address, value);
        }
    }

    /// Asks for a 256-byte page to be copied into OAM, which halts the CPU for 513 cycles, 514
    /// when the alignment cycle is needed (nesdev, "DMA").
    ///
    /// The transfer does **not** run here. A `$4014` write is a write cycle, and the DMA unit
    /// may only halt the CPU on a read: it starts on the next cycle the CPU reads on, which for
    /// real code is the opcode fetch that follows the `STA $4014`. The 513 or 514 stolen cycles
    /// and their parity are unchanged by that — the halt cycle moved by one, and so did the
    /// parity that decides the alignment (see [`NesBus::run_dma`]).
    fn queue_oam_dma(&mut self, page: u8) {
        self.oam_dma_count += 1;
        self.dma_oam_page = page;
        self.dma |= DMA_OAM | DMA_HALT;
    }

    /// Notices what a write to an APU register did to the DMC's memory reader.
    ///
    /// A `$4015` write can both start a fetch and take one back. "If interrupted before the
    /// halt cycle starts, cancel DMA completely: this can happen when a write prevents the DMA
    /// from starting after being queued" — which is precisely the case a write creates, since
    /// it cannot be halted.
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    fn after_apu_write(&mut self) {
        if self.apu.needs_dmc_dma() {
            self.poll_dmc_dma();
        } else if (self.dma & (DMA_DMC | DMA_HALT)) == (DMA_DMC | DMA_HALT) {
            self.dma &= !(DMA_DMC | DMA_DUMMY);
            if (self.dma & DMA_OAM) == 0 {
                self.dma &= !DMA_HALT;
            }
        }
    }

    /// A read of the widescreen extension's PRG window: the byte of PRG-ROM it shows there.
    /// CPU RAM, the PPU and the cartridge at once, for a margin decoder.
    pub(crate) fn decoder_parts(&mut self) -> (&[u8], &mut Ppu2C02, Option<&Cartridge>) {
        (&self.ram, &mut self.ppu, self.cartridge.as_ref())
    }

    /// The extension's register (`$5000`-`$5FFF`) a data access of `address` reaches, when it is
    /// in `$3000`-`$3FFF`, made by an instruction of the execution zone, and the zone holds its
    /// data there; `None` for every other access, the game's among them.
    fn zone_data_address(&self, address: u16) -> Option<u16> {
        (self.zone_instruction && address >= 0x3000 && self.ppu.wide().zone_data())
            .then_some(address + 0x2000)
    }

    fn wide_window_read(&self, address: u16) -> Option<u8> {
        let offset = self.ppu.wide().prg_window(address)?;
        // A hack pack's code, when one runs; PRG-ROM otherwise.
        let code = self.ppu.wide().code();
        if !code.is_empty() {
            return Some(code.get(offset).copied().unwrap_or(0xFF));
        }
        let prg = self.cartridge.as_ref()?.rom().prg_rom();
        prg.get(offset % prg.len().max(1)).copied()
    }

    /// Writes a byte exactly as a CPU write would, without advancing emulated time
    /// (REQ-CORE-082).
    pub fn poke(&mut self, address: u16, value: u8) {
        self.open_bus = value;
        self.write_internal(address, value);
    }
}

impl NesBus {
    /// One **read** cycle of the CPU: the DMA units' only chance to halt it, then the cycle
    /// itself (see [`NesBus::run_dma`]). Outside `run_dma` a non-zero `dma` always carries
    /// `DMA_HALT`, so the gate needs no mask — one `cmpb` and one not-taken branch.
    ///
    /// **Out of line, and that is the point.** [`CpuBus::read`] is inlined into about a hundred
    /// places inside `Cpu6502::step`, `resolve`, `branch`, `push` and `pull`. The gate is two
    /// instructions to *execute* but five to *encode* (the test, the branch, and the two
    /// argument moves and the `callq` of the cold side), and pasting those hundred times cost a
    /// measured **+8 % of wall clock** on an NROM and an MMC1 — all of it instruction cache, as
    /// the executed-instruction count went *down*. Behind this call the gate is encoded once;
    /// the call sites keep the single `callq` they already had for [`NesBus::tick`].
    /// The two sides are written as an `if`/`else` of two tail calls on purpose: that way the
    /// function needs **no stack frame of its own** — no pushed register, no reserved shadow
    /// space — and the no-DMA side is a compare, a not-taken branch and a jump straight into
    /// [`NesBus::tick`], which is the call the read site already made.
    #[inline(never)]
    fn read_cycle(&mut self, address: u16) {
        if self.dma == 0 {
            self.tick();
        } else {
            self.halted_read_cycle(address);
        }
    }

    /// The cold half of [`NesBus::read_cycle`]: the DMA units take the bus, then the CPU gets
    /// its cycle. Separate so that its argument shuffling and its call never reach the other
    /// half's prologue.
    // Q-119
    #[inline(never)]
    fn halted_read_cycle(&mut self, address: u16) {
        self.run_dma(address);
        self.tick();
    }
}

impl CpuBus for NesBus {
    fn read(&mut self, address: u16) -> u8 {
        self.read_cycle(address);
        let value = self.read_internal(address);
        self.open_bus = value;
        value
    }

    /// An instruction fetch in the widescreen extension's execution zone (`$2000`-`$3FFF`, while
    /// a pack's code runs there) reads that code; any other is a plain read.
    ///
    /// Measured: the four instructions MF-4a's DMA gate adds to [`CpuBus::read`], multiplied by
    /// the call sites `read` is inlined into, grew this function past the inliner's budget and
    /// it became a `callq` on **every instruction fetch** — and, one level up, pushed
    /// `Cpu6502::execute` back out of `step`. Both are MF-3b's "a callee gaining its first call
    /// site is a stop-the-lot event"; the attribute puts them back.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn read_code(&mut self, address: u16) -> u8 {
        let zone = self.ppu.wide().zone_offset(address);
        self.zone_instruction = zone.is_some();
        if let Some(offset) = zone {
            self.read_cycle(address);
            let value = self.ppu.wide().code().get(offset).copied().unwrap_or(0xFF);
            self.open_bus = value;
            return value;
        }
        self.read(address)
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

        if WideExtension::claims(address) && !self.ppu.wide().zone_data() {
            if let Some(value) = self.wide_window_read(address) {
                return value;
            }
            if let Some(value) = self.ppu.wide().read(address) {
                return value;
            }
        }
        if (0x6000..0x8000).contains(&address) {
            if let Some(value) = self.wide_window_read(address) {
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
        // No DMA can be in flight at a save point: a state is written between instructions,
        // and the units own every cycle of theirs. Nothing of `dma` is in the format, so a
        // state written before MF-4a loads byte for byte (REQ-CORE-091).
        self.dma = 0;
        self.dma_oam_page = 0;
        self.ppu_lag = 0;
        self.ppu_deadline = 0;
        Ok(())
    }
}
