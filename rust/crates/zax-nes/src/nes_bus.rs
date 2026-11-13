use crate::apu::Apu2A03;
#[cfg(feature = "board-vs-system")]
use crate::cartridges::mappers::VsBoard;
use crate::cartridges::Cartridge;
use crate::cpu::CpuBus;
use crate::input::InputPorts;
use crate::ppu::Ppu2C02;
use crate::state::{InvalidState, StateReader, StateWriter};
use crate::wide::WideExtension;

/// Size of CPU work RAM.
const RAM_SIZE: usize = 2048;

/// [`NesBus::dma`]: a DMA unit is still owed its **halt** cycle. Outside [`NesBus::run_dma`] a
/// non-zero `dma` has this bit whenever a unit is actually asking for the bus, which is what lets
/// [`CpuBus::read`] gate the whole DMA path on `dma != 0` — one `cmpb` — rather than on a mask
/// (REQ-CORE-113). [`DMA_DMC_WAIT`] is the one state that carries no halt: there the DMC has not
/// asked yet, and the cold path it sends the read cycle to simply runs the cycle.
const DMA_HALT: u8 = 0x01;

/// [`NesBus::dma`]: the DMC's **dummy** cycle is still owed. The memory reader needs a halt and
/// a dummy cycle before it may read, which is what makes a lone DMC fetch three or four cycles
/// rather than two (nesdev, "DMA").
const DMA_DUMMY: u8 = 0x02;

/// [`NesBus::dma`]: a DMC sample fetch is pending or running.
const DMA_DMC: u8 = 0x04;

/// [`NesBus::dma`]: an OAM transfer is pending or running.
const DMA_OAM: u8 = 0x08;

/// [`NesBus::dma`]: the DMC's memory reader has asked for a byte, but no halt may land before
/// [`NesBus::dmc_ready_cycle`]. Every DMC request passes through this state:
///
/// * a **reload** — the sample buffer running dry — is asked for inside the cycle's
///   [`NesBus::tick`], so the earliest cycle it can halt is the next one;
/// * a **load** — a `$4015` write that starts a sample — waits two or three cycles more,
///   by the parity of the write's cycle (nesdev, "DMA"; see `DmcChannel::set_enabled`).
///
/// The load's delay lives here, as one absolute cycle compared only on the cycles that reach
/// [`NesBus::halted_read_cycle`], rather than as a counter inside the DMC. In the DMC it had to be
/// tested by `clock_timer`, `quiet_clocks` and `skip_quiet_clocks` — the per-CPU-cycle and
/// per-quiet-run paths of **every** game, with or without a sample playing — which R0 measured at
/// about +88 M instructions per 300 frames on Mega Man 2 (lot MF-4a-bis step 1). Here it costs a
/// game that runs no DMA nothing: `dma` is zero on every one of its cycles, which is the single
/// `cmpb` [`NesBus::read_cycle`] already made.
const DMA_DMC_WAIT: u8 = 0x10;

/// [`NesBus::dma`]: a `$4015` write has just switched the DMC off while its memory reader had, or
/// was about to have, a fetch to make, and the switch-off has not reached the DMA unit yet.
///
/// "Disabling takes effect with a 1 APU cycle delay. If a DMA starts during this time, it gets
/// cancelled, but this will still cause the CPU to be halted" (Mesen 2,
/// `DeltaModulationChannel::SetEnabled`; `AccuracyCoin`, `Explicit DMA Abort`). The channel itself
/// is switched off at once — nothing but the DMA unit can tell — and this flag holds the fetch the
/// reader *would* have made, with the cycle its halt may land on in [`NesBus::dmc_ready_cycle`]
/// and the cycle the switch-off arrives in [`NesBus::dmc_abort_cycle`]. It lives at most three
/// cycles and costs the cycles of a game that never does this nothing: it is one more bit of the
/// `dma` byte [`NesBus::read_cycle`] already tests against zero.
const DMA_DMC_GHOST: u8 = 0x20;

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
    /// The 2A03's **internal** data bus: the byte of the last access the CPU or a DMA unit made,
    /// whatever answered it. Every read and every write loads it.
    data_bus: u8,
    /// The **external** data bus — the pins, and the capacitance that holds them — when it
    /// differs from [`NesBus::data_bus`], with the cycle it was last driven on its own in
    /// `ext_cycle`. See [`NesBus::external_bus`].
    ext_bus: u8,
    ext_cycle: i64,
    /// The cycle a DMA unit last gave a halted read back to the CPU on; `i64::MIN` while none
    /// ever has. Read by the `SH*` stores alone (see [`CpuBus::dma_gave_back_at`]).
    dma_gave_back: i64,
    cycles: i64,
    /// What the two DMA units still owe: [`DMA_HALT`], [`DMA_DUMMY`], [`DMA_DMC`], [`DMA_OAM`].
    /// Zero while neither unit wants the bus, which is the state of every cycle of a game that
    /// plays no DPCM sample and writes no `$4014` (see [`NesBus::run_dma`]).
    dma: u8,
    /// Page the pending OAM transfer copies from: the byte written to `$4014`.
    dma_oam_page: u8,
    /// Earliest cycle the halt of the DMC request held by [`DMA_DMC_WAIT`] may land on; only read
    /// while that bit is set.
    dmc_ready_cycle: i64,
    /// The cycle a `$4015` switch-off reaches the DMA unit, while [`DMA_DMC_GHOST`] is set.
    /// Transient like `dma`, and not in the savestate for the same reason.
    dmc_abort_cycle: i64,
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
    /// Whether the cartridge is a mapper-99 board, the one board whose register is a `$4016`
    /// write, asked once in [`NesBus::attach`] beside `expansion_audio`. See
    /// [`NesBus::write_vs_port`]; `false` for every other image, including every other VS one.
    #[cfg(feature = "board-vs-system")]
    vs_port: bool,
    /// Whether the cartridge is a board that reads something of the **console** when it answers a
    /// CPU read below `$8000`: mapper 143's protection read, whose two top bits the page calls open
    /// bus (Q-145), and mapper 292's data registers, which the page fills from "the desired values"
    /// the game "writes to zero page" (Q-172 §2). Both are MF-FIX.
    ///
    /// Asked once in [`NesBus::attach`], beside `vs_port` and for the same reason: the fill site
    /// then asks one `bool` and never the `Board` enum. `false` for every other image, so the only
    /// thing a game on any other board pays is this flag's test, in a function that is already
    /// `#[cold]` ([`NesBus::read_wide_window`]).
    ///
    /// **One flag for the two boards, and both snapshots taken together.** A cartridge is one
    /// board, so at most one of the two ever reads its own snapshot; filling both costs the board
    /// that does not two loads and two stores on its own window's reads, and saves a second flag
    /// test for everybody else.
    #[cfg(any(feature = "board-143-open-bus", feature = "board-292"))]
    samples_console: bool,
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
            data_bus: 0,
            ext_bus: 0,
            ext_cycle: 0,
            dma_gave_back: i64::MIN,
            cycles: 0,
            dma: 0,
            dma_oam_page: 0,
            dmc_ready_cycle: 0,
            dmc_abort_cycle: 0,
            oam_dma_count: 0,
            dmc_dma_count: 0,
            ppu_lag: 0,
            ppu_deadline: 0,
            deferred_ppu: false,
            apu_lag: 0,
            expansion_audio: false,
            #[cfg(feature = "board-vs-system")]
            vs_port: false,
            #[cfg(any(feature = "board-143-open-bus", feature = "board-292"))]
            samples_console: false,
            apu_batching: false,
            apu_quiet: 0,
            zone_instruction: false,
        }
    }

    /// Last value seen on the **external** data bus, which is what an unmapped read returns.
    #[must_use]
    pub const fn open_bus(&self) -> u8 {
        self.external_bus()
    }

    /// The byte the external data bus is holding.
    ///
    /// **Why there are two buses.** The 2A03's internal data bus and the cartridge-edge data bus
    /// are joined by a buffer the chip does not always enable, so they are not always the same
    /// byte. `AccuracyCoin`'s `Internal Data Bus` test states the rule it measures: "The internal
    /// data bus is updated during every read/write, while the external data bus is updated on
    /// every read/write **except for reads from address `$4015`**" — `$4015` is decoded inside
    /// the 2A03 and answers on the internal bus alone — and, the other way round, a DMC sample
    /// fetch "does not update the internal data bus. Only the external one".
    ///
    /// So the two differ for at most the cycle a `$4015` read or a DMC fetch happened on and the
    /// one after it: every other access drives both, and a CPU cycle is one access. That is what
    /// this reads — `data_bus` carries the internal bus for free on the per-cycle path, and the
    /// external one costs two stores on those two cold sites and nothing anywhere else.
    ///
    /// The bound is exact for every sequence a program can make. It is approximate in two cases
    /// that need a DMA *and* an address in `$4000`-`$401F`, and falls back to `data_bus` there:
    /// a halt cycle the controller cannot see (`halt_is_invisible`) directly after a `$4015`
    /// read, and an OAM transfer whose source page is `$40` interleaved with a DMC fetch. No ROM
    /// here reaches either.
    #[must_use]
    const fn external_bus(&self) -> u8 {
        if self.cycles - self.ext_cycle <= 1 {
            self.ext_bus
        } else {
            self.data_bus
        }
    }

    /// Notes that only the external data bus was driven, with `value`: a DMC sample fetch.
    fn drive_external_bus(&mut self, value: u8) {
        self.ext_bus = value;
        self.ext_cycle = self.cycles;
    }

    /// Notes that the external bus was **not** driven although the internal one was: a read of
    /// `$4015`. Called before `data_bus` takes the byte the APU answered with, so the value kept
    /// here is the one the pins are still holding.
    fn hold_external_bus(&mut self) {
        self.ext_bus = self.external_bus();
        self.ext_cycle = self.cycles;
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

    /// Plugs the VS. System of the image loaded into the machine, or takes it out with `None`
    /// (lot MF-VS).
    ///
    /// **One call settles every consumer**, which is the whole design of the lot: the cabinet's
    /// state goes to the controller ports, and the PPU is handed the two *answers* it needs —
    /// its palette remap and its RC2C05 register quirks — so that nothing downstream ever asks
    /// which machine this is. Called by `NesCore::load_rom` before the power-on, and the pieces
    /// it sets are configuration that no reset clears.
    #[cfg(feature = "board-vs-system")]
    pub fn set_vs_system(&mut self, vs: Option<crate::vs::VsSystem>) {
        self.input.set_vs_system(vs);
        self.ppu.set_vs_ppu(self.input.vs());
    }

    /// Hands a `$4016` write to the cartridge, for the **one** board whose register is a CPU
    /// register: mapper 99's PRG and CHR bank select is bit 2 of that write (nesdev, "INES
    /// Mapper 099" and "Vs. System"; lot MF-VS step 4).
    ///
    /// `$4016` is the controller strobe, so every game writes it a few times a frame and this is
    /// not a cold path — but what a non-VS game pays is **one test of a `bool`** settled in
    /// [`Self::attach`], beside the `expansion_audio` flag and in the same cache line. No `Board`
    /// dispatch is consulted, and `Cartridge::cpu_write` is the existing path: it reaches
    /// `Mapper::write_register` with the address unchanged and refreshes the ROM windows after,
    /// which is exactly what a bank switch needs.
    #[cfg(feature = "board-vs-system")]
    #[inline]
    fn write_vs_port(&mut self, value: u8) {
        if self.vs_port {
            if let Some(cartridge) = self.cartridge.as_mut() {
                cartridge.cpu_write(VsBoard::PORT, value);
            }
        }
    }

    /// Without the VS layer no board listens to `$4016`, and the call is nothing.
    #[cfg(not(feature = "board-vs-system"))]
    #[inline]
    #[allow(clippy::unused_self)]
    fn write_vs_port(&mut self, _value: u8) {}

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
        // Mapper 99 is the one board whose register is a **CPU** register: bit 2 of a `$4016`
        // write (lot MF-VS step 4). Settled here, once, so that the write path below asks one
        // `bool` and never the `Board` enum.
        #[cfg(feature = "board-vs-system")]
        {
            self.vs_port = cartridge
                .as_ref()
                .is_some_and(|cartridge| cartridge.mapper_number() == VsBoard::MAPPER);
        }
        // Q-145 and Q-172 section 2 / MF-FIX: the two boards that read something of the console
        // when they answer a read below $8000 -- mapper 143 the cartridge-edge data bus, mapper 292
        // the game's zero page. Settled here for `read_wide_window`'s reason, which the `vs_port`
        // note above gives in the same words.
        #[cfg(any(feature = "board-143-open-bus", feature = "board-292"))]
        {
            self.samples_console = cartridge
                .as_ref()
                .is_some_and(|cartridge| matches!(cartridge.mapper_number(), 143 | 292));
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
        self.data_bus = 0;
        self.ext_bus = 0;
        self.ext_cycle = 0;
        self.dma_gave_back = i64::MIN;
        self.cycles = 0;
        self.dma = 0;
        self.dma_oam_page = 0;
        self.dmc_ready_cycle = 0;
        self.dmc_abort_cycle = 0;
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
        if self.apu.needs_dmc_dma() && (self.dma & (DMA_DMC | DMA_DMC_WAIT)) == 0 {
            // The request is raised inside this cycle's `tick`, so the earliest cycle its halt
            // can land on is the next one — which is the whole reason the request is *held*
            // rather than armed (see [`DMA_DMC_WAIT`] and [`NesBus::read_cycle`]).
            self.hold_dmc_dma(0);
        }
    }

    /// Holds a DMC request whose halt may not land before `delay` cycles after the next one: 0 for
    /// a reload, the `$4015` load delay for a load.
    fn hold_dmc_dma(&mut self, delay: u32) {
        self.dmc_ready_cycle = self.cycles + 1 + i64::from(delay);
        self.dma |= DMA_DMC_WAIT;
    }

    /// Turns the held DMC request ([`DMA_DMC_WAIT`]) into a real one once `halt_cycle` — the cycle
    /// the halt would land on — has reached [`NesBus::dmc_ready_cycle`]. A halt and a dummy cycle
    /// come before the fetch itself (nesdev, "DMA").
    ///
    /// Reached only from the DMA path, which `dma != 0` already gates, so the cycles of a game
    /// that runs no DMA never see it.
    ///
    /// The request is raised only if the memory reader still wants a byte: a `$4015` disable, a
    /// soft reset or a sample that ended inside the window takes it back, which is the
    /// "cancel DMA completely" half of the nesdev page read from the other side.
    fn promote_dmc_wait(&mut self, halt_cycle: i64) {
        if (self.dma & DMA_DMC_WAIT) == 0 || halt_cycle < self.dmc_ready_cycle {
            return;
        }

        self.dma &= !DMA_DMC_WAIT;
        if self.apu.needs_dmc_dma() {
            self.dma |= DMA_DMC | DMA_DUMMY | DMA_HALT;
        } else if (self.dma & DMA_OAM) == 0 {
            self.dma &= !DMA_HALT;
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
    ///   `ZONE` is the one case where it does not: see [`NesBus::dma_repeat_read`].
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
    ///
    /// **`ZONE`** says the halted read is an instruction fetch from the widescreen extension's
    /// execution zone, and suppresses the re-reads of `read_address` alone — see
    /// [`NesBus::dma_repeat_read`] for the rule and [`NesBus::halted_zone_read_cycle`] for why it
    /// is a const parameter and not an argument. Everything else is the same machine: the same
    /// cycles, the same transfers, the same reads of the units' **own** addresses.
    // Q-119
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    #[inline(never)]
    fn run_dma<const ZONE: bool>(&mut self, read_address: u16) {
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

        // The halt cycle has already run: it is the cycle the caller ticked, the one the CPU was
        // about to read on (see [`NesBus::read_cycle`]). What is left of it is its read, which
        // happens with the CPU's address still on the bus.
        self.dma &= !DMA_HALT;
        if !halt_is_invisible {
            self.dma_repeat_read::<ZONE>(read_address);
        }

        // Bytes read and written so far, counted the way the hardware's pair counter is: even
        // means the next get cycle reads, odd means the next put cycle writes.
        let mut oam_steps: u16 = 0;
        let mut oam_index: u8 = 0;
        let mut oam_byte: u8 = 0;

        // Whether the fast loop below may be used at all for this transfer, which depends only on
        // the halted address and the page and so is asked once (see [`NesBus::copy_oam_pairs`]).
        let fast = !internal && self.dma_oam_page != 0x40;

        while (self.dma & (DMA_DMC | DMA_OAM)) != 0 {
            // **The fast path.** An OAM transfer with nothing else owed, standing on a byte
            // boundary one cycle before a get cycle, and with no DMC fetch able to fall inside
            // the next `pairs` byte pairs: those pairs are a tight get/put loop, and everything
            // the state machine below asks per cycle is answered once for all of them.
            if fast && self.dma == DMA_OAM && (self.cycles & 1) != 0 && (oam_steps & 1) == 0 {
                let pairs = self.oam_fast_pairs(0x100 - (oam_steps >> 1));
                if pairs > 0 {
                    oam_index = self.copy_oam_pairs(pairs, oam_index);
                    oam_steps += pairs * 2;
                    if oam_steps == 0x200 {
                        self.dma &= !DMA_OAM;
                        self.ppu.wide_mut().latch_sprites(self.dma_oam_page);
                    }
                    continue;
                }
            }

            // A held DMC request can come due in the middle of an OAM transfer — a load whose
            // `$4015` delay runs out, or a reload the memory reader asked for inside one of the
            // ticks below; the transfer's own cycles then stand in for its halt and dummy cycles.
            // The cycle this iteration is about to run is `cycles + 1`, which is where its halt
            // would land.
            self.promote_dmc_wait(self.cycles + 1);

            let get_cycle = (self.cycles & 1) != 0;

            if get_cycle && (self.dma & (DMA_HALT | DMA_DUMMY | DMA_DMC)) == DMA_DMC {
                // The memory reader has had its halt and its dummy cycle: it reads.
                self.dma_cycle();
                self.dmc_dma_count += 1;
                let address = self.apu.dmc_dma_address();
                let value = self.dmc_fetch_read(address, internal, &mut decoded_before);
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
                    self.ppu.wide_mut().latch_sprites(self.dma_oam_page);
                }
            } else {
                // A cycle no unit can use: the DMC's dummy read, or an alignment before a get.
                // The CPU's address is still on the bus, so its read happens again.
                self.dma_cycle();
                if !input_port {
                    self.dma_repeat_read::<ZONE>(read_address);
                }
            }
        }
    }

    /// A DMC sample fetch with no OAM transfer beside it, halting a read of an address outside
    /// `$4000`-`$401F`: the whole of [`NesBus::run_dma`] for that case, entered straight from
    /// [`NesBus::halted_read_cycle`] so that it pays neither `run_dma`'s prologue — eight general
    /// registers and two XMM registers, the price of the OAM state machine's working set — nor
    /// the three classifications of the halted address that only the `internal` case reads.
    ///
    /// A game that plays DPCM makes tens of these per frame against its one OAM transfer
    /// (`dpcmletterbox`: 19 051 fetches against 297 transfers in 300 frames), so this is the
    /// other half of the DMA bookkeeping lot MF-4a-ter set out to trim.
    /// `ZONE` carries [`NesBus::run_dma`]'s meaning, and suppresses the same re-reads.
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    #[inline(never)]
    fn dmc_only_dma<const ZONE: bool>(&mut self, read_address: u16) {
        // The halt cycle has already run; what is left of it is its read, with the CPU's address
        // still on the bus. `halt_is_invisible` needs `internal`, which the caller excluded.
        self.dma &= !DMA_HALT;
        self.dma_repeat_read::<ZONE>(read_address);
        self.dmc_fetch_cycles::<ZONE>(read_address);
    }

    /// The read a DMA unit makes of the **CPU's** address while it holds the bus: the halt cycle's
    /// own read, and the dummy and alignment cycles' repeats of it. One per idle DMA cycle.
    ///
    /// On real hardware it always happens — the CPU is stopped with its address on the bus and the
    /// access is simply made again, which is what `dmc_dma_during_read4` and `dma_2007_read`
    /// measure. **`ZONE` is the one case where it must not** (lot ZR-2): the address is then an
    /// instruction fetch from the widescreen extension's execution zone, `$2000`-`$3FFF`, and that
    /// zone does not exist on real hardware at all — no NES ever runs code from the PPU's register
    /// window, so there is no hardware behaviour to preserve by reading `$2002` or `$2007` here.
    /// What there is instead is the extension's own rule: **code running from the zone never
    /// reaches the PPU registers through the CPU's fetch path**, neither by the fetch (lot ZR-1)
    /// nor by a halt or dummy re-read of the fetch address (this lot). The hack's own explicit
    /// loads and stores to the PPU registers are data accesses, not fetches, and are out of the
    /// rule — they go on reaching the PPU as they always did.
    ///
    /// Measured at the tip of ZR-1, Super Mario Bros. 3 with `--advanced --widescreen 128`:
    /// 1 077 halted zone fetches in 1 500 frames made **3 166** of these reads, every one of them
    /// reaching `Ppu2C02::read` and none absorbed by the extension's own `$3000`-`$3FFF` window —
    /// 441 of `$2002`, each clearing the `w` latch, and 442 of `$2007`, each moving `v` and the
    /// read buffer. Since MF-4d step 6 a `$2007` read during rendering moves `v` by a coarse X and
    /// a Y at once, which is right for a real read and is what broke the row of ground above the
    /// status bar. Ninja Gaiden III: 3 810 of them in 700 frames.
    ///
    /// Not reading leaves [`NesBus::data_bus`] alone as well, which is the same statement: a read
    /// that does not happen drives nothing onto the bus. The byte the fetch takes comes from the
    /// pack's code buffer in [`CpuBus::read_code`], and loads it there. The alternative would be
    /// to load the bus with what the register *would* have answered, which needs the read the rule
    /// forbids — so the choice is barely open, but it is observable and it is written down.
    // Q-124
    ///
    /// The DMA units' reads of their **own** addresses — an OAM transfer's source bytes, the DMC's
    /// sample fetch — are not this read and are untouched: the transfer runs byte for byte, on the
    /// same cycles, whatever halted it.
    #[allow(clippy::inline_always)] // The whole point: `ZONE` false must leave no trace.
    #[inline(always)]
    fn dma_repeat_read<const ZONE: bool>(&mut self, read_address: u16) {
        if !ZONE {
            let value = self.read_internal(read_address);
            self.data_bus = value;
        }
    }

    /// The cycles of a DMC sample fetch with no OAM transfer beside it: the dummy cycle, an
    /// alignment cycle when the one after it is not a get cycle, and the fetch. Three cycles after
    /// the halt, or four — [`NesBus::DMC_DMA_CYCLES_ALIGNED`] and [`NesBus::DMC_DMA_CYCLES`].
    ///
    /// **Why the sequence is fixed before it starts**, so that the state machine's per-cycle
    /// questions can all be answered here in advance:
    ///
    /// * no second DMC request can appear inside it. `poll_dmc_dma` raises one only while
    ///   `(dma & (DMA_DMC | DMA_DMC_WAIT)) == 0`, and `DMA_DMC` stays set until the byte has been
    ///   supplied below — so `promote_dmc_wait` would have nothing to do on any of these cycles;
    /// * no OAM transfer can appear inside it: `$4014` is a write and the CPU is halted;
    /// * `DMA_HALT` is already consumed by the caller and `DMA_DUMMY` is consumed by the first
    ///   cycle, so `dma_cycle`'s two tests have one answer each;
    /// * the caller's `!internal` makes `input_port` false too (`$4016` and `$4017` are inside
    ///   `$4000`-`$401F`), so every idle cycle repeats the CPU's read unconditionally, and
    ///   `halt_is_invisible` — which needs `internal` — was false.
    ///
    /// The DMC's read keeps going through [`NesBus::dma_read`]: it is one call per fetch, it is
    /// where the 2A03's "two reads in a row" rule lives, and a sample address outside
    /// `$8000`-`$FFFF` (which only a loaded state can produce) must still decode as it did.
    fn dmc_fetch_cycles<const ZONE: bool>(&mut self, read_address: u16) {
        // The dummy cycle. The CPU's address is still on the bus, so its read happens again.
        self.dma &= !DMA_DUMMY;
        self.tick();
        self.dma_repeat_read::<ZONE>(read_address);

        // An alignment cycle when the cycle after this one would not be a get cycle.
        if (self.cycles & 1) == 0 {
            self.tick();
            self.dma_repeat_read::<ZONE>(read_address);
        }

        // The get cycle: the memory reader has had its halt and its dummy cycle, so it reads.
        self.tick();
        self.dmc_dma_count += 1;
        let address = self.apu.dmc_dma_address();
        let mut decoded_before = read_address;
        let value = self.dmc_fetch_read(address, false, &mut decoded_before);
        self.sync_apu();
        self.apu.supply_dmc_byte(value);
        // Cleared last: the `sync_apu` above polls the request, which is still raised until the
        // byte lands.
        self.dma &= !DMA_DMC;
    }

    /// How many of the `wanted` byte pairs an OAM transfer still owes may run in
    /// [`NesBus::copy_oam_pairs`] instead of in the state machine above: the pairs that end on or
    /// before the cycle the DMC's memory reader could ask for a byte on.
    ///
    /// **Why the bound closes.** The state machine's loop does one thing per cycle that the fast
    /// loop does not: `promote_dmc_wait`, which turns a held DMC request into a halt. A request
    /// can only appear from `poll_dmc_dma` — that is, from inside one of the `tick`s the loop
    /// itself runs, on the cycle the reader runs dry — and `cycles_until_dmc_fetch` names that
    /// cycle (lot MF-4a-ter step 1). Let it be `dry = cycles + free`. The request raised there is
    /// *held* until `dry + 1`, so the cycles `cycles + 1` to `dry` are cycles on which
    /// `promote_dmc_wait` would have done nothing at all, and skipping it on them is not an
    /// approximation.
    ///
    /// `free / 2` is exactly the pairs that fit in those cycles, and it also keeps `dry` from
    /// being a *get* cycle with a put still to come inside the batch: a pair is an even get cycle
    /// and the odd put after it, so the batch's last cycle is always a put, and when `free` is
    /// even that put *is* `dry`. Were the batch to run past a get cycle on which the request
    /// rose, the put after it would have consumed the DMC's halt cycle in the state machine
    /// (`dma_cycle`) and the fetch would land a cycle earlier than the fast loop puts it.
    ///
    /// `None` from the APU is "no sample is playing, and a halted CPU cannot start one", so the
    /// whole of the rest of the transfer is one batch — which is every OAM DMA of a game that
    /// plays no DPCM.
    fn oam_fast_pairs(&self, wanted: u16) -> u16 {
        let Some(free) = self.apu.cycles_until_dmc_fetch() else {
            return wanted;
        };
        // The APU's horizon is measured from where the APU stands: on a quiet cycle the DMC's
        // timer has not been counted down yet, so its own lag comes off the answer.
        let free = free.saturating_sub(self.apu_lag);
        u16::try_from(free / 2).unwrap_or(u16::MAX).min(wanted)
    }

    /// Copies `pairs` bytes of the pending OAM transfer, a get cycle and a put cycle each, and
    /// returns the index the next byte comes from.
    ///
    /// This is the state machine above with everything it asks per cycle hoisted out: `dma` is
    /// `DMA_OAM` alone, so no halt or dummy cycle is owed and `dma_cycle` is a plain `tick`; the
    /// caller has checked that the next cycle is a get one and that the transfer stands on a byte
    /// boundary, so the get/put alternation is the loop body rather than a dispatch; no DMC
    /// request can appear inside the batch, so `promote_dmc_wait` is not asked; and `dma_read`'s
    /// work is all constant here — the caller's `fast` excludes a halted address in `$4000`-`$401F`
    /// (so `internal` is false and `decoded_before` is dead) and a source page of `$40` (so no
    /// source address can be one of the open-bus ones), which leaves the plain
    /// [`NesBus::read_internal`] the old `$4014` loop made.
    ///
    /// What the state machine charged per stolen cycle for all of that is lot MF-4a-ter's subject;
    /// the cycles themselves, their count and their parity are untouched.
    fn copy_oam_pairs(&mut self, pairs: u16, mut oam_index: u8) -> u8 {
        let page = u16::from(self.dma_oam_page) << 8;
        for _ in 0..pairs {
            // The get cycle: the DMA unit reads the source byte.
            self.tick();
            let byte = self.read_internal(page | u16::from(oam_index));
            self.data_bus = byte;
            oam_index = oam_index.wrapping_add(1);

            // The put cycle: it writes it into OAM.
            self.tick();
            self.sync_ppu();
            self.ppu.write_oam_dma(byte);
        }
        oam_index
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
            // Nothing answers $4000-$401F on the external bus. Written as two returns rather
            // than one `if` expression so that the external bus — a compare and a select — stays
            // behind the test instead of being hoisted in front of it: an ordinary source
            // address, which is every byte of every OAM transfer, must pay nothing for it.
            if (0x4000..0x4020).contains(&address) {
                let value = self.external_bus();
                self.data_bus = value;
                *decoded_before = address;
                return value;
            }
            let value = self.read_internal(address);
            self.data_bus = value;
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
                    self.data_bus
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
        self.data_bus = value;
        *decoded_before = decoded;
        value
    }

    /// The **DMC's** fetch, which is [`NesBus::dma_read`] on the external bus alone.
    ///
    /// The sample byte reaches the DMC straight off the pins and never travels the internal bus,
    /// so the cycle after a fetch cannot see it in bit 5 of a `$4015` read — which is
    /// `AccuracyCoin`'s `Internal Data Bus` test 2 — while an open-bus read on that cycle does
    /// see it, which is `DMA + Open Bus`. Four instructions, once per fetch.
    fn dmc_fetch_read(&mut self, address: u16, internal: bool, decoded_before: &mut u16) -> u8 {
        let internal_bus = self.data_bus;
        let value = self.dma_read(address, internal, decoded_before);
        self.drive_external_bus(value);
        self.data_bus = internal_bus;
        value
    }

    /// Reads controller port `port` ($4016 or $4017) with its side effects, the Zapper's light
    /// detection included (REQ-CORE-061, REQ-CORE-062); this is the reference's
    /// `InputPorts.Read` with its light probe wired to the PPU.
    pub fn read_input_port(&mut self, port: usize, open_bus: u8) -> u8 {
        self.sync_ppu();
        let cycle = self.cycles;
        let ppu = &self.ppu;
        self.input
            .read(port, open_bus, cycle, |aim| ppu.sees_light_at(aim.x, aim.y))
    }

    /// The bus read itself, as a body each of the **two** out-of-line callers below gets its own
    /// copy of: [`NesBus::read_cycle`] and [`NesBus::read_internal`] (lot PR-1, Q-119 point 3).
    ///
    /// The point of the second copy is that the DMA gate then ends **in** the read instead of a
    /// `jmp` into it, so a read cycle no longer pays that `jmp` nor a second function's prologue
    /// and epilogue: **−5.9 instructions per CPU read cycle**, measured by callgrind over
    /// thirteen ROMs (lot PR-1 steps 2 and 3).
    ///
    /// What it costs is that LLVM, rather than duplicate them, moves the callees it can out of
    /// the read: `Ppu2C02::read` and `Cartridge::try_cpu_read` each become a tail call. That is
    /// **+4.1 instructions per PPU register read**, and it cannot lose, because a register read
    /// *is* a read cycle and so saves 5.9 first — which is the answer to the open question of
    /// Q-119 point 3, measured on a ROM that spends a fifth of its read cycles on `$2002`.
    ///
    /// The two things that make the arrangement hold, both measured and both easy to undo by
    /// accident:
    ///
    /// * the rare arms are out of line and `#[cold]` ([`NesBus::read_io_register`],
    ///   [`NesBus::read_wide_window`]), which keeps the body small enough that the two copies do
    ///   not cost more than they save;
    /// * `Board::cpu_read_override` carries `#[inline(always)]`, without which the MMC5 — the one
    ///   board that reaches it on *every* read — pays +0.59 % instead of gaining 0.73 %.
    ///
    /// **Do not** add `#[inline(always)]` to `Ppu2C02::read` to "keep the register arm inline":
    /// measured at +0.02 % overall and +0.89 % on the MMC5, because its prologue then lands on
    /// every read cycle instead of on the register reads alone (lot PR-1 step 3, variants C and
    /// D).
    #[allow(clippy::inline_always)] // Measured: lot PR-1 step 3, -0.84 % over thirteen ROMs.
    #[inline(always)]
    fn read_internal_body(&mut self, address: u16) -> u8 {
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
                return match self.ppu.wide().read(register) {
                    Some(value) => value,
                    None => self.external_bus(),
                };
            }
            self.sync_ppu();
            // The PPU drives its own data-bus latch for the undefined bits, separate from
            // the CPU's open bus (nesdev, "PPU registers"; ppu_open_bus).
            return self.ppu.read(address & 0x07, self.cartridge.as_mut());
        }

        if address < 0x4020 {
            return self.read_io_register(address);
        }

        if address < 0x8000 {
            if let Some(value) = self.read_wide_window(address) {
                return value;
            }
        }

        // `match` and not `unwrap_or`: the external bus is a compare and a select, and asking for
        // it eagerly puts those on every read the cartridge *does* answer — the PRG-RAM of every
        // game with a battery.
        match self
            .cartridge
            .as_mut()
            .and_then(|cartridge| cartridge.try_cpu_read(address))
        {
            Some(value) => value,
            None => self.external_bus(),
        }
    }

    /// The out-of-line copy of [`NesBus::read_internal_body`]: every reader of the bus that is
    /// not a CPU read cycle — the DMA units, the halted read, the OAM loop.
    ///
    /// `#[inline(never)]` because there are about a dozen such sites and the body is 170
    /// instructions: without it the copies multiply, which is the instruction-cache loss Q-119
    /// measured at +8 % of wall clock when the DMA gate itself was pasted at every call site.
    #[inline(never)]
    fn read_internal(&mut self, address: u16) -> u8 {
        self.read_internal_body(address)
    }

    /// `$4000`-`$401F`, the 2A03's own registers: `$4015` and the two controller ports, a few
    /// dozen reads a frame at most, so the arm is out of line and `#[cold]`.
    ///
    /// It is out of line to keep [`NesBus::read_internal_body`] small, that body now having two
    /// copies. Measured cost of the call: under 90 000 instructions per 300 frames on any ROM of
    /// lot PR-1's set.
    #[cold]
    #[inline(never)]
    fn read_io_register(&mut self, address: u16) -> u8 {
        match address {
            0x4015 => {
                self.sync_apu();
                // Bit 5 is the **internal** bus, and the read leaves the pins alone: the two
                // halves of `AccuracyCoin`'s `Internal Data Bus` test, and `Open Bus` 7 and 9
                // (see [`NesBus::external_bus`]).
                self.hold_external_bus();
                self.apu.read_status(self.data_bus)
            }
            // A controller port answers on the internal bus too, and its three open-bus bits
            // come from there — which is how `Implied Dummy Reads` can take the operands of a
            // `JSR` out of `$4016`/`$4017` and have them "leech off the value read from the
            // opcode" when that opcode came from `$4015`.
            0x4016 => self.read_input_port(0, self.data_bus),
            0x4017 => self.read_input_port(1, self.data_bus),

            // $4000-$4014 are write-only and $4018-$401F are disabled: open bus.
            _ => self.external_bus(),
        }
    }

    /// The widescreen extension's window at `$5000`-`$5FFF` and its optional one over
    /// `$6000`-`$7FFF`, both of which only a ROM made for this emulator ever drives.
    ///
    /// The caller's single `address < 0x8000` test stands for the two ranges: `claims` is
    /// `$5000`-`$5FFF` and the second range `$6000`-`$7FFF`, and at this point the address is
    /// already past `$401F`. So one compare replaces two range tests in each of the body's two
    /// copies, and the window's own decode moves here.
    ///
    /// `#[cold]` even though `$6000`-`$7FFF` is the PRG-RAM of every game with a battery: that
    /// range reaches [`NesBus::wide_window_read`], which answers `None` for every cartridge whose
    /// ROM has not written the unlock sequence, and the read then carries on to the cartridge as
    /// before. Measured: 1.6 M instructions per 300 frames on the Holy Mapperel ROMs, 10.6 M on
    /// an MMC5 reading its `ExRAM` through `$5C00`, under 0.15 M on any commercial game of lot
    /// PR-1's set.
    #[cold]
    #[inline(never)]
    fn read_wide_window(&mut self, address: u16) -> Option<u8> {
        // MF-FIX: what the two boards that read something of the console need, taken before the
        // cartridge is asked. **The fill is here and the reads are in the boards** because this is
        // the function that already runs on exactly the reads able to reach a window below $8000
        // ($4020-$7FFF), it is already `#[cold]` and `#[inline(never)]`, and `NesBus` is the only
        // thing that has the bus and the work RAM in hand: `Mapper::try_read_prg` and
        // `Mapper::cpu_read_override` are handed `CartridgeMemory` and an address and nothing else.
        //
        // * **Q-145**, mapper 143: the cartridge-edge data bus, for the `xx` of its `[xxii iiii]`
        //   that the page annotates "open bus". `external_bus()` and not `data_bus` -- the edge is
        //   the external bus, and `data_bus` is written *after* `read_internal` returns, so here it
        //   still holds the previous access, which is what open bus means.
        // * **Q-172 section 2**, mapper 292: zero page $6A and $FF, which is where the page says
        //   "the game writes the desired values" that fill its two Extra Data registers. This
        //   replaces a latch stored on **every CPU write of every game**, which MF-UNL-12 step 6
        //   measured at `mmc5exram` +1.098 %.
        //
        // No address test on either: each board decodes its own window, and a snapshot taken on a
        // read outside it is overwritten by the next read inside it before anybody looks, so the
        // value is always that of the access before the read that uses it. One `bool` and nothing
        // else for every other game, and nothing at all with both features off.
        #[cfg(any(feature = "board-143-open-bus", feature = "board-292"))]
        if self.samples_console {
            #[cfg(feature = "board-143-open-bus")]
            let edge = self.external_bus();
            #[cfg(feature = "board-292")]
            let zero_page = [self.ram[0x6A], self.ram[0xFF]];
            if let Some(cartridge) = self.cartridge.as_mut() {
                #[cfg(feature = "board-143-open-bus")]
                cartridge.memory_mut().set_cpu_edge_bus(edge);
                #[cfg(feature = "board-292")]
                cartridge.memory_mut().set_zero_page(zero_page);
            }
        }

        if WideExtension::claims(address) && !self.ppu.wide().zone_data() {
            if let Some(value) = self.wide_window_read(address) {
                return Some(value);
            }
            if let Some(value) = self.ppu.wide().read(address) {
                return Some(value);
            }
        }
        if (0x6000..0x8000).contains(&address) {
            if let Some(value) = self.wide_window_read(address) {
                return Some(value);
            }
        }
        None
    }

    /// **No cartridge latch here, since MF-FIX.** MF-UNL-12 step 2 stored every written byte into
    /// `CartridgeMemory` on this path, for the one board whose page describes a latch of "the data
    /// of the last CPU write to any address" (mapper 292) -- a store on every CPU write of every
    /// game, measured at `oamstr` +0.026 % and `mmc5exram` **+1.098 %**, which is what parked
    /// `board-292`. The page's own next sentence, "The game writes the desired values to zero
    /// page", is what made the latch avoidable: the board now reads zero page `$6A`/`$FF` through
    /// the snapshot [`NesBus::read_wide_window`] takes, on its own window's reads alone. Q-172
    /// section 2, `docs/review/Q-XCHECK.md` section CHANGE 8.
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
                // `self.cycles` is the write's own cycle: the bus ticks, then performs the
                // access. The strobe's reload waits for the next put cycle (see
                // [`InputPorts::write`]).
                0x4016 => {
                    self.input.write(value, self.cycles);
                    self.write_vs_port(value);
                }
                _ => {
                    self.sync_apu();
                    let ghost = if address == 0x4015 {
                        self.dmc_switch_off_window(value)
                    } else {
                        None
                    };
                    let dmc_load_delay = self.apu.write(address, value);
                    self.after_apu_write(dmc_load_delay);
                    if let Some((ready, abort)) = ghost {
                        self.dmc_ready_cycle = ready;
                        self.dmc_abort_cycle = abort;
                        self.dma |= DMA_DMC_GHOST;
                    }
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

    /// Notices what a write to an APU register did to the DMC's memory reader; `dmc_load_delay`
    /// is what [`Apu2A03::write`] returned.
    ///
    /// A `$4015` write can both start a fetch and take one back. "If interrupted before the
    /// halt cycle starts, cancel DMA completely: this can happen when a write prevents the DMA
    /// from starting after being queued" — which is precisely the case a write creates, since
    /// it cannot be halted.
    ///
    /// A write that **starts** a sample does not raise the request either: its load fetch waits
    /// two or three cycles, which this turns into the one absolute cycle [`DMA_DMC_WAIT`] holds.
    /// What a `$4015` write about to be made leaves the DMA unit to do when it switches a playing
    /// DMC off: the cycle the fetch its memory reader has asked for — or will ask for before the
    /// switch-off arrives — may halt on, and the cycle the switch-off arrives. `None` when the
    /// write does not switch the channel off, when nothing is playing, or when no fetch can halt
    /// before the switch-off (see [`DMA_DMC_GHOST`]). Called with the APU caught up.
    ///
    /// The switch-off arrives two cycles after a write on an even CPU cycle and three after one
    /// on an odd cycle, the same parity and the same figures as a load's start delay
    /// (`DmcChannel::set_enabled`).
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    #[cold]
    #[inline(never)]
    fn dmc_switch_off_window(&mut self, value: u8) -> Option<(i64, i64)> {
        // A write that switches the channel on, or leaves it on, ends any window still open.
        self.dma &= !DMA_DMC_GHOST;
        if (value & 0x10) != 0 {
            return None;
        }

        let until_fetch = self.apu.cycles_until_dmc_fetch()?;
        let abort = self.cycles + if self.apu.apu_cycle() { 2 } else { 3 };
        let ready = if (self.dma & DMA_DMC_WAIT) != 0 {
            self.dmc_ready_cycle
        } else if (self.dma & DMA_DMC) != 0 || until_fetch == 0 {
            self.cycles + 1
        } else {
            // The reader runs dry inside the tick of that cycle, and a request raised inside a
            // tick can halt on the next cycle at the earliest (see `poll_dmc_dma`).
            self.cycles + i64::from(until_fetch) + 1
        };
        (ready <= abort).then_some((ready, abort))
    }

    /// The cycles a DMC fetch caught by a `$4015` switch-off still takes from the CPU, `late`
    /// being how many cycles after the halt the switch-off arrives (see [`DMA_DMC_GHOST`]). The
    /// halt cycle has already run, as in [`NesBus::run_dma`].
    ///
    /// * on the halt cycle itself: the DMA is aborted after it, one cycle;
    /// * on the dummy cycle: the cycle after it still runs — the fetch if it is a get cycle, the
    ///   alignment cycle otherwise, after which the DMA is gone — three cycles;
    /// * later: the DMA runs to its fetch, three or four cycles.
    ///
    /// The byte a fetch reads is dropped in every case: the channel has no bytes left to play.
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    #[cold]
    #[inline(never)]
    fn ghost_dmc_dma(&mut self, read_address: u16, late: i64) {
        let input_port = matches!(read_address, 0x4016 | 0x4017);
        let internal = (read_address & 0xFFE0) == 0x4000;
        if !(input_port && late == 0) {
            self.dma_repeat_read::<false>(read_address);
        }
        if late == 0 {
            return;
        }

        self.tick();
        if !input_port {
            self.dma_repeat_read::<false>(read_address);
        }
        let aligns = (self.cycles & 1) == 0;
        if aligns && late >= 2 {
            self.tick();
            if !input_port {
                self.dma_repeat_read::<false>(read_address);
            }
        }

        self.tick();
        if aligns && late == 1 {
            if !input_port {
                self.dma_repeat_read::<false>(read_address);
            }
            return;
        }
        self.dmc_dma_count += 1;
        let address = self.apu.dmc_dma_address();
        let mut decoded_before = read_address;
        self.dmc_fetch_read(address, internal, &mut decoded_before);
    }

    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    fn after_apu_write(&mut self, dmc_load_delay: u32) {
        if dmc_load_delay > 0 {
            // `self.cycles` is the write's own cycle: the bus ticks, then performs the access.
            self.hold_dmc_dma(dmc_load_delay);
            return;
        }

        if self.apu.needs_dmc_dma() {
            self.poll_dmc_dma();
        } else if (self.dma & (DMA_DMC | DMA_DMC_WAIT)) != 0 {
            self.dma &= !(DMA_DMC | DMA_DMC_WAIT | DMA_DUMMY);
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
        self.data_bus = value;
        self.write_internal(address, value);
    }
}

impl NesBus {
    /// The **read** of a CPU read cycle whose cycle has just run, and the DMA units' only chance
    /// to halt it (see [`NesBus::run_dma`]).
    ///
    /// **Where the gate sits, and why there.** The DMA question has to be asked on the read cycle
    /// itself — the halt repeats the read the CPU was making — but it does not have to be asked
    /// *before* it: the cycle the CPU was about to read on **is** the halt cycle, so running it
    /// first and asking afterwards is the same machine. Asking afterwards is what makes it free:
    /// the gate then stands where the read site already materialised the address for
    /// `read_internal`, so a read cycle passes the address to **one** call, as it did before
    /// MF-4a, instead of two. The `movl` of the second materialisation is the +9.0 M of
    /// `Cpu6502::step`/`branch`/`resolve` this lot's step 2 gives back, measured on a mapper 1
    /// that never runs a DMA.
    ///
    /// What the ordering costs is that `tick` can raise a DMC request *inside* the cycle the gate
    /// is then asked about. It may not halt that cycle: the hardware samples the request before
    /// the cycle. So every DMC request is **held** — [`DMA_DMC_WAIT`] with an earliest halt cycle
    /// in [`NesBus::dmc_ready_cycle`] — and only the cold half below turns it into a halt.
    ///
    /// **Out of line, and that is the point.** [`CpuBus::read`] is inlined into about a hundred
    /// places inside `Cpu6502::step`, `resolve`, `branch`, `push` and `pull`. The gate is two
    /// instructions to *execute* but five to *encode* (the test, the branch, and the two
    /// argument moves and the `callq` of the cold side), and pasting those hundred times cost a
    /// measured **+8 % of wall clock** on an NROM and an MMC1 — all of it instruction cache, as
    /// the executed-instruction count went *down*. Behind this call the gate is encoded once;
    /// the call sites keep the single `callq` they already had for [`NesBus::read_internal`].
    /// The two sides are written as an `if`/`else` of two tail calls on purpose: that way the
    /// function needs **no stack frame of its own** — no pushed register, no reserved shadow
    /// space — and the no-DMA side is a compare, a not-taken branch and a jump straight into
    /// [`NesBus::read_internal`], which is the call the read site already made.
    // Q-119
    #[inline(never)]
    fn read_cycle(&mut self, address: u16) -> u8 {
        if self.dma == 0 {
            // Its **own copy** of the read (lot PR-1): the gate then ends in the read instead of
            // a `jmp` into it, which is −5.9 instructions a read cycle. See
            // [`NesBus::read_internal_body`] for what that copy costs and why it still wins.
            self.read_internal_body(address)
        } else {
            self.halted_read_cycle(address)
        }
    }

    /// The cold half of [`NesBus::read_cycle`]: the cycle that has just run becomes the DMA's
    /// halt cycle, the units take the bus, and the CPU then gets the cycle it was denied and
    /// reads again. Separate so that its argument shuffling and its frame never reach the other
    /// half.
    ///
    /// A held DMC request whose cycle has not come is the one case where `dma` is non-zero and no
    /// unit may halt: the read then happens as on any other cycle.
    // Q-119
    #[inline(never)]
    fn halted_read_cycle(&mut self, address: u16) -> u8 {
        if (self.dma & DMA_DMC_GHOST) != 0 {
            // A fetch a `$4015` switch-off caught. With an OAM transfer beside it the window is
            // simply dropped: the transfer's cycles would have stood in for the fetch's.
            let late = self.dmc_abort_cycle - self.cycles;
            if late < 0 || self.dma != DMA_DMC_GHOST {
                self.dma &= !DMA_DMC_GHOST;
            } else if self.cycles >= self.dmc_ready_cycle {
                self.dma = 0;
                self.ghost_dmc_dma(address, late);
                self.tick();
                self.dma_gave_back = self.cycles;
                return self.read_internal(address);
            } else {
                return self.read_internal(address);
            }
        }

        // The cycle that has just run is the one a halt would land on.
        self.promote_dmc_wait(self.cycles);
        if (self.dma & (DMA_DMC | DMA_OAM)) == 0 {
            return self.read_internal(address);
        }
        if self.dma == (DMA_DMC | DMA_DUMMY | DMA_HALT) && (address & 0xFFE0) != 0x4000 {
            // A lone DMC fetch, which is most of what a game playing DPCM comes here for, and a
            // fixed three or four cycles rather than a state machine (lot MF-4a-ter step 3).
            self.dmc_only_dma::<false>(address);
        } else {
            self.run_dma::<false>(address);
        }
        self.tick();
        // The cycle the CPU finally reads on, for the five `SH*` stores: they drop the AND with
        // the address high byte when the RDY line went low on their dummy read (see
        // [`CpuBus::dma_gave_back_at`]). One store per halted read cycle, zero on every other.
        self.dma_gave_back = self.cycles;
        self.read_internal(address)
    }

    /// The DMA units' chance to halt a read cycle whose byte does **not** come from the bus: an
    /// instruction fetch inside the widescreen extension's execution zone, which reads a hack
    /// pack's code. It runs the cycle itself, so the fetch site keeps the single call it had.
    #[inline(never)]
    fn zone_read_cycle(&mut self, address: u16) {
        self.tick();
        if self.dma != 0 {
            self.halted_zone_read_cycle(address);
        }
    }

    /// The cold half of [`NesBus::zone_read_cycle`]: [`NesBus::halted_read_cycle`] for a fetch
    /// whose byte does not come from the bus.
    ///
    /// **The one thing it must not do is read the address** (lot ZR-1). The zone is `$2000`-`$3FFF`
    /// — the PPU's registers — and the byte the CPU fetches there comes from the pack's code
    /// buffer, so the fetch itself never drives a read of `$2002` or `$2007`. Sharing
    /// `halted_read_cycle`, which since MF-4a-bis ends in `read_internal` on both its paths, gave
    /// every halted zone fetch one: it cleared `w`, or moved `v` and the read buffer — and since
    /// MF-4d step 6 a `$2007` read during rendering moves `v` by a coarse X and a Y at once.
    /// Measured on Super Mario Bros. 3 with `--advanced --widescreen 128`: 3 301 halted zone
    /// fetches in 1 500 frames, **every one of them** a register read the hardware does not make,
    /// 420 of `$2002` and 411 of `$2007`. It broke the row of ground above the status bar from
    /// frame 1 500, and Ninja Gaiden III from frame 370.
    ///
    /// **Nor may the DMA read it on the fetch's behalf** (lot ZR-2). ZR-1 took away the CPU's own
    /// read and left the halt's and the dummy cycles' re-reads of the same address inside
    /// [`NesBus::run_dma`] and [`NesBus::dmc_only_dma`], on the grounds that the CPU is stopped
    /// with `$2xxx` on the address bus whatever the fetched byte came from. That reasoning does not
    /// hold, because the premise does not: **the zone does not exist on real hardware**, so there
    /// is no halt of a fetch from `$2000`-`$3FFF` whose behaviour could be preserved. What is left
    /// is the extension's rule, and it covers the whole fetch path — see
    /// [`NesBus::dma_repeat_read`], which is where the suppression lives and what it measured.
    /// 3 166 of those reads survived ZR-1 on Super Mario Bros. 3's 1 500 frames and the row of
    /// ground stayed broken.
    ///
    /// **Why `ZONE` is a const parameter and not an argument.** `run_dma` and `dmc_only_dma` are
    /// on the DMA path of **every** game, and the bar for this lot is zero cost outside the zone.
    /// A `bool` argument would put a `movl` at each of their ordinary call sites and a test in
    /// each body; a const parameter is monomorphised, so `run_dma::<false>` is the old `run_dma`
    /// instruction for instruction and its call site is the old call site. What it costs instead
    /// is a second, cold copy of each — step 2's table measures it — reached only from here.
    ///
    /// [`NesBus::dma_gave_back`] is not stamped either, for the same reason the read is not made:
    /// it is read by the five `SH*` stores about **their own dummy read**, which is a data read
    /// and never a fetch.
    ///
    /// A zone address is in `$2000`-`$3FFF` by construction (`Wide::zone_offset`), so
    /// `halted_read_cycle`'s `$4000`-`$401F` test is answered here once and for all: the halted
    /// address is never one of the 2A03's own registers.
    #[inline(never)]
    fn halted_zone_read_cycle(&mut self, address: u16) {
        // The zone has no part in the switch-off window (see `halted_read_cycle`).
        self.dma &= !DMA_DMC_GHOST;
        // The cycle that has just run is the one a halt would land on.
        self.promote_dmc_wait(self.cycles);
        if (self.dma & (DMA_DMC | DMA_OAM)) == 0 {
            return;
        }
        if self.dma == (DMA_DMC | DMA_DUMMY | DMA_HALT) {
            self.dmc_only_dma::<true>(address);
        } else {
            self.run_dma::<true>(address);
        }
        // The cycle the CPU finally fetches on; its byte comes from the code buffer.
        self.tick();
    }
}

impl CpuBus for NesBus {
    /// One **read** cycle: the cycle, then the read it carries — which the DMA units may turn
    /// into a halt (see [`NesBus::read_cycle`]). Two calls and one materialisation of the
    /// address, which is what this was before MF-4a.
    fn read(&mut self, address: u16) -> u8 {
        self.tick();
        let value = self.read_cycle(address);
        self.data_bus = value;
        value
    }

    /// An instruction fetch in the widescreen extension's execution zone (`$2000`-`$3FFF`, while
    /// a pack's code runs there) reads that code; any other is a plain read.
    ///
    /// Measured: the four instructions MF-4a's DMA gate added to [`CpuBus::read`], multiplied by
    /// the call sites `read` is inlined into, grew this function past the inliner's budget and
    /// it became a `callq` on **every instruction fetch** — and, one level up, pushed
    /// `Cpu6502::execute` back out of `step`. Both are MF-3b's "a callee gaining its first call
    /// site is a stop-the-lot event"; the attribute puts them back. MF-4a-bis took those four
    /// instructions back out again, and the attribute stays: it is the shape the whole MF-3b/MF-4a
    /// history says to pin rather than rediscover.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn read_code(&mut self, address: u16) -> u8 {
        let zone = self.ppu.wide().zone_offset(address);
        self.zone_instruction = zone.is_some();
        if let Some(offset) = zone {
            self.zone_read_cycle(address);
            let value = self.ppu.wide().code().get(offset).copied().unwrap_or(0xFF);
            self.data_bus = value;
            return value;
        }
        self.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.tick();
        self.data_bus = value;
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

    /// **`#[inline(always)]`, and it has to stay: a `callq` here costs about 170 M instructions
    /// over 300 frames, on every board.** This is MF-3b step 1's regression one level up, and
    /// lot MF-5b met it by adding a thirty-second `Board` variant.
    ///
    /// `Cpu6502::end_cycle` samples /IRQ once per CPU cycle and reaches this function, which
    /// inlines `Cartridge::irq_asserted` and with it the whole `Board::irq_asserted` dispatch —
    /// thirty-two arms, twenty of which only answer "no". MF-3b pinned `end_cycle` itself so the
    /// *sampling* would stay inlined; nothing pinned this, so when the VRC7's arm grew the
    /// dispatch past the inliner's budget LLVM outlined **this** function instead and called it
    /// every cycle. Callgrind priced it at `+169 739 749` on `m1` and `m4`, `+169 641 994` on
    /// `m0`, `+164 225 879` on `metroid` — about **19 instructions per CPU cycle, on thirteen
    /// ROMs of which twelve are on a board that cannot assert an IRQ at all** (MF-5b step 1).
    ///
    /// The attribute, and not the load-time latch MF-3b step 1 also costed: the latch would take
    /// an MMC1 below parity, but the dispatch has to *leave* this function for the `bool` to
    /// shrink it, which puts the `callq` on the boards that do have an IRQ — mapper 4 among them,
    /// and golden-core's. Restoring parity is what the lot's bar asks for.
    #[allow(clippy::inline_always)] // Measured: 170 M instructions, MF-5b step 1.
    #[inline(always)]
    fn irq_asserted(&self) -> bool {
        self.apu.irq_asserted() || self.cartridge.as_ref().is_some_and(Cartridge::irq_asserted)
    }

    fn cycle_count(&self) -> i64 {
        self.cycles
    }

    fn dma_gave_back_at(&self) -> i64 {
        self.dma_gave_back
    }
}

impl NesBus {
    /// Appends the bus state (work RAM, open bus, clock and DMA counters) to `writer`
    /// (REQ-CORE-090). The chips and the cartridge the bus owns save themselves.
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_blob(&self.ram);
        writer.write_u8(self.data_bus);
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
        self.data_bus = reader.read_u8()?;
        // The external bus is in the format only through `data_bus`: the two differ for at most
        // the cycle a `$4015` read or a DMC fetch happened on and the one after it, and neither
        // can be the cycle a state is written on. A `$4015` read **can** be an instruction's last
        // cycle, but the next instruction's first cycle is its opcode fetch, which drives both
        // buses before anything reads the external one.
        self.ext_bus = 0;
        self.ext_cycle = 0;
        // Transient like `dma`: no save point can fall between a halted read and the write of the
        // instruction it belongs to, both being inside one `Cpu6502::step`.
        self.dma_gave_back = i64::MIN;
        self.cycles = reader.read_i64()?;
        self.oam_dma_count = reader.read_i64()?;
        self.dmc_dma_count = reader.read_i64()?;
        // Nothing of `dma` is in the format, so a state written before MF-4a loads byte for byte
        // (REQ-CORE-091).
        //
        // A DMA that is *running* cannot be at a save point, because all 513 or 514 cycles of a
        // transfer happen inside one `CpuBus::read` and so inside one instruction. A **pending**
        // one can: a `$4014` write is a write cycle, which no unit may halt, so the transfer waits
        // for the next read cycle — the opcode fetch of the instruction after it — and a held DMC
        // request waits for an absolute cycle by construction. Clearing the three fields below
        // therefore drops a transfer a `STA $4014` had just asked for, and the state reloads as if
        // that write had not happened.
        //
        // That is a divergence and not a save-format question this lot settled: see Q-120 for the
        // measurement, its scope (one cycle per `STA $4014`, crossed every frame by run-ahead) and
        // the three fields a fix would add behind a block version. It is pinned as it stands by
        // `req_core_091_a_pending_oam_transfer_is_dropped_by_a_state_load`.
        // Q-120
        self.dma = 0;
        self.dma_oam_page = 0;
        self.dmc_ready_cycle = 0;
        self.ppu_lag = 0;
        self.ppu_deadline = 0;
        Ok(())
    }
}
