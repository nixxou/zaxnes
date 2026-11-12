//! Two variants of the console chip: Zaxnes's own 2A03, and a NesHawk-compatible one.
//!
//! A `.bk2` movie is recorded against a particular emulator's idea of the 2A03's timing, and
//! BizHawk's NesHawk core makes a handful of choices Zaxnes does not — it halts the CPU on the
//! DMC's request cycle itself, it defers the DMC's length counter by three cycles, and it turns a
//! mid-sample `$4015` enable into a delayed load-style DMA. Those choices are not *more* right
//! than Zaxnes's (on each of them Zaxnes agrees with Mesen 2 and with the nesdev wiki, and the
//! differences are recorded item by item in `docs/progress/NESHAWK-PROFILE.md`); they are simply
//! what a NesHawk movie was recorded against, so replaying one in step needs them.
//!
//! **Why a type and not a flag.** The places that differ are on the CPU's per-cycle path — the
//! bus's read and write cycles, the APU's step, the DMC's timer — where this core counts
//! instructions (see the notes in [`crate::NesBus`] and `apu::DmcChannel`). A runtime `if profile
//! == NesHawk` there would cost every game a load and a branch on cycles that never run a DMA at
//! all. Instead the core is generic over a zero-sized profile type: the compiler monomorphises
//! [`crate::NesCore`] twice, every `P::SOMETHING` folds to a constant, and the [`Standard`] copy
//! is the code this core had before the profile existed, instruction for instruction.
//!
//! **How to add an item** is `docs/review/NESHAWK-PROFILE.md`; the short version is a new
//! associated constant here, a `false` (or "as today") value on [`Standard`], the NesHawk value on
//! [`NesHawkCompat`], and an `if P::ITEM` at the one site that reads it.

/// Which 2A03 a core runs: the console chip's timing, as one of two zero-sized types.
///
/// Implemented by [`Standard`] and [`NesHawkCompat`] and by nothing else. Every member is an
/// associated constant so that the generic code reading it folds away: a profile value is never
/// stored, passed or compared at run time.
pub trait TimingProfile: Copy + Clone + Default + core::fmt::Debug + Send + Sync + 'static {
    /// What the harness's `--profile` option, a savestate's report and the UI call this profile.
    const NAME: &'static str;

    /// The byte a savestate records so that restoring one into the other profile can be refused
    /// rather than silently drifting. Stable across versions: it is a file format.
    const STATE_ID: u8;

    /// The core name a savestate written by this profile carries at the head of its block, and
    /// the only one a core of this profile accepts.
    ///
    /// The two profiles are two machines — their DMC DMA differs by cycles — so a block of one
    /// restored into the other would drift silently from the frame after the restore. Refusing it
    /// through the **core name** is what makes that free: it is the field REQ-CORE-120 already has
    /// a message for, the check is the one `try_load_state` already makes, and
    /// `NesCore::STATE_VERSION` does not move, so every block written before this lot still loads
    /// into the standard core byte for byte. The first four bytes of the block stay
    /// `state_magic(EngineKind::Zaxnes)` either way: it is the same *engine*, and that is what the
    /// application reads them for.
    const STATE_CORE_NAME: &'static str;

    // ------------------------------------------------------- the DMC's sample-fetch DMA
    //
    // Item numbers are `docs/progress/NESHAWK-PROFILE.md` step 1's, where each is quoted against
    // the NesHawk function it was read from. Every one of them is `false` on `Standard`, and every
    // site that reads one writes the `Standard` arm as the code this core had before the profile
    // existed — so that arm is not merely equivalent, it is the same source.

    /// **Item 1a.** The halt of a DMC **reload** request — the sample buffer running dry — lands
    /// on the **request cycle itself** rather than on the first read cycle after it.
    ///
    /// NesHawk lowers `RDY` inside `DMCUnit.Run()`, which `APU.RunOneFirst()` calls before
    /// `cpu.ExecuteOne()`, so the cycle the memory reader ran dry on is already halted. Zaxnes —
    /// and Mesen 2, and the nesdev *DMA* page read the other way — raise the request inside the
    /// cycle and halt the next one.
    const DMC_HALT_ON_REQUEST_CYCLE: bool;

    /// **Item 1b.** A DMC fetch costs a **fixed four** cycles — the halt, two re-reads of the
    /// CPU's own address, the fetch — with no get/put alignment cycle and so no dependence on the
    /// parity of the cycle it started on.
    ///
    /// NesHawk's `dmc_dma_countdown = 3` and the two `RunDMCHaltFetch()` calls it spends; there is
    /// no `% 2` anywhere on that path. Zaxnes spends three cycles or four by parity
    /// (`NesBus::DMC_DMA_CYCLES_ALIGNED` and `NesBus::DMC_DMA_CYCLES`).
    const DMC_FETCH_HAS_TWO_DUMMIES: bool;

    /// **Item 2.** The sample length, the loop and the DMC IRQ are applied **three CPU cycles
    /// after** the fetch instead of at the fetch, so bit 4 of `$4015` stays set for three cycles
    /// after the last byte of a sample has been read.
    ///
    /// NesHawk's `DMCUnit.Fetch()` only sets `apu.dmc_reload_countdown = 3`; `RunOneLast()` spends
    /// it. Zaxnes does all of it in `DmcChannel::supply_byte`, as Mesen 2 does.
    const DMC_LENGTH_LAGS_THE_FETCH: bool;

    /// **Item 3.** A `$4015` write with bit 4 set starts a **write-style** fetch whenever the
    /// sample buffer is empty — *including while a sample is already playing* — and that fetch is
    /// displaced by two or three cycles and costs **three** cycles rather than four.
    ///
    /// NesHawk's `set_lenctr_en` tests `!sample_buffer_filled` as a **sibling** of the restart and
    /// not as a child of it, so `call_from_write` is set mid-sample; `Run()` then takes its `else`
    /// branch, which never lowers `RDY` and lets `RunCpuOne` start the halting a cycle later.
    /// Zaxnes's `DmcChannel::set_enabled` delays only a fetch that a write actually **started**
    /// (`bytes_remaining == 0`), and a mid-sample write leaves the plain four-cycle reload alone.
    const DMC_ENABLE_WRITES_MID_SAMPLE: bool;

    /// **Item 1c** (lot NESHAWK-PROFILE-2 step 3; retracted by NESHAWK-PROFILE-6 and **restored by
    /// NESHAWK-PROFILE-8, which measured it**). The halt lands on the request cycle **even when the
    /// CPU is writing on it**, so a request that falls on a run of an *odd* number of write cycles
    /// gives the CPU its cycle back **one cycle earlier** than waiting for the next read cycle does.
    ///
    /// This is item 1's third half, which NESHAWK-PROFILE named in its step 1 and did not
    /// implement. NesHawk has it in two pieces: `DMCUnit.Run()` lowers `RDY` whatever the CPU is
    /// doing on the request cycle, and `MOS6502X`'s write microstep runs unconditionally and never
    /// sets `rdy_freeze`; then `RunCpuOne`'s
    /// `if (!cpu.RDY && !cpu.rdy_freeze && dmc_dma_countdown == DMC_RDY_check) countdown += 2`
    /// gives the fetch two more cycles when no read microstep has stalled by then — that is, when
    /// the CPU has been writing throughout.
    ///
    /// **Why one bit of the slip is the whole of it.** Let the request fall on a run of `w`
    /// consecutive write cycles. NesHawk's stalled read then executes `w + 4` cycles after the
    /// request for `w` of 0 or 2, and `w + 3` for `w` of 1 or 3 (the `+2` fires for `w >= 2`, and
    /// the run ends before the countdown needs it for `w <= 1`). Zaxnes halts the first read cycle
    /// instead, `w` cycles on, and spends four — `w + 4` always. So the two agree on an even `w`
    /// and differ by exactly one cycle on an odd one, and **a 6502 never writes on more than three
    /// cycles in a row** (the three pushes of an interrupt or `BRK`; `JSR` and a read-modify-write
    /// make two), so `w <= 3` and the rule below is exact rather than an approximation.
    ///
    /// **The measurement, which is the reason the item is back** (lot NESHAWK-PROFILE-8 step 1).
    /// Over tree frames 11 to 46 of the *Kirby* `.bk2`, **705** sample fetches read off BizHawk's
    /// own bus stream with **no cross-machine calibration anywhere** — the stall is an
    /// instruction's duration minus the shortest duration that PC is ever seen to take, and the
    /// writes that passed through the DMA are the cycles carrying a `w` **and** an `r`, NesHawk's
    /// write microstep ignoring `RDY` being exactly what puts two accesses on one cycle:
    ///
    /// | slip | stall 3 | stall 4 |
    /// |---|---|---|
    /// | 0 | 0 | **648** |
    /// | 1 | **46** | 0 |
    /// | 2 | 0 | **11** |
    ///
    /// No exceptions, so `stall = 4 − (slip mod 2)`. Lot NESHAWK-PROFILE-6 retracted the item on a
    /// table that read the slip off **this core's** census and the stall off BizHawk's stream; the
    /// slip is a property of what the **CPU** is doing on the request cycle, and the two machines'
    /// CPUs are 2 cycles apart inside the instruction from the movie's reset onwards (the reset
    /// lands 3 cycles late here — NESHAWK-PROFILE-8 step 1), so a fetch this core calls slip 1 is
    /// one NesHawk calls slip 0 or 2. Lot 6's own table said so and was read the other way round:
    /// "slip 1 came out 4 six times and **3 ten times**", and the ten were the measurement.
    ///
    /// Read at `nes_bus.rs` `promote_dmc_wait` and `is_a_lone_dmc_fetch`. Only a **reload** is
    /// affected: a fetch item 3's `$4015` write started is not halted on its request cycle at all.
    ///
    /// **It shipped off for one lot, and the gate was the reset** (lot NESHAWK-PROFILE-8 step 2):
    /// on its own it took the *Kirby* `.bk2` from no live difference until frame **45 095** back to
    /// **925**, because the slip it fires on is read off **this core's** CPU position and this
    /// core's CPU was 2 cycles further into the instruction than NesHawk's from the movie's reset
    /// onwards — the right rule on the wrong fetches. **Lot NESHAWK-PROFILE-9 step 1 closed that
    /// gate** with item 12 ([`TimingProfile::RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT`]): the reset
    /// lands on NesHawk's own cycle, the four once-a-frame marks of the movie read the same
    /// residual either side of it, and the item is **on**.
    ///
    /// Measured, so that no later lot re-measures it: with the item on and the reset's phase moved
    /// instead by the one knob that can move it (item 10's dot count at the reset alone, `+9` dots)
    /// the movie reached 1 100 and parted at **9 353**, 35 742 short of 45 095. A dot bias is not
    /// the reset fix — it moves the PPU against the CPU and the post-reset boot's 7-cycle `$2002`
    /// polling loop re-quantises it, so the residual read +2, +2, +2, +1, −5 over biases of 0, 3,
    /// 6, 9 and 12 dots and never reached the −1 the frames before the reset read — where ending
    /// the frame where NesHawk ends it moves the whole machine and leaves the code's position
    /// against the PPU alone.
    const DMC_HALT_LANDS_ON_A_WRITE_CYCLE: bool;

    /// **Item 7** (lot NESHAWK-PROFILE-2 step 3), NesHawk's `dmc_realign`: a DMC fetch that falls
    /// **inside an OAM transfer** makes that transfer one cycle longer than the fetch alone does.
    ///
    /// `RunCpuOne`'s OAM block is gated on `oam_dma_exec && apu.dmc_dma_countdown != 1 &&
    /// !dmc_realign`, and the DMC block sets `dmc_realign = true` on the cycle its countdown
    /// reaches 1 — the fetch cycle. So the transfer does not advance on the fetch cycle (which it
    /// cannot anyway, the DMC has the bus) **nor on the cycle after it**, and NesHawk's transfer
    /// loses two cycles per inside-fetch where this core's loses one: Zaxnes's `run_dma` lets the
    /// DMC's halt and re-read cycles double as the transfer's own, and only the fetch cycle itself
    /// is taken from it.
    ///
    /// Read at `nes_bus.rs` `run_dma`, in the `dmc_may_read` arm.
    const DMC_FETCH_STALLS_AN_OAM_TRANSFER: bool;

    // ------------------------------------------------------- the PPU's start-up frame

    /// **Item 10** (lot NESHAWK-PROFILE-4 step 2). The PPU's **first frame after a power-on and
    /// after a soft reset is a start-up frame of `241 * 341 - 4 = 82 177` dots**, not a whole one
    /// of 89 342 — so the first frame boundary comes **2 389 CPU cycles earlier**, twice over a
    /// movie that presses RESET once.
    ///
    /// **Where it comes from.** NesHawk's `PPU.cs` `Reset()` sets `ppudead = 1` and
    /// `start_up_offset = 4`, and `PPU.run.cs` `NewDeadPPU()` then counts dots — running the CPU
    /// through `runppu()` all the while — until `ppur.status.cycle == 241 * 341 - start_up_offset`,
    /// where it raises the vblank flag, decrements `ppudead`, calls `ppu_init_frame()` and sets
    /// `nes.frame_is_done = true`. Its own comment says why it exists: *"not quite emulating all
    /// the NES power up behavior … it needs to wait for vblank"*. This core's PPU instead starts
    /// at the beginning of vblank (Q-003, REQ-CORE-020 notes) and runs a whole frame, and its
    /// `Ppu2C02::soft_reset` deliberately does not restart the frame at all (Q-078).
    ///
    /// **It is expressed as dots skipped, because that is the same thing from this side.** The
    /// frame this core would run is 89 343 dots long as the CPU meets it, NesHawk's is 82 176, so
    /// advancing the PPU by the difference puts the first frame boundary where NesHawk puts it and
    /// leaves every later frame alone.
    ///
    /// **What it is worth.** The whole constant drift between the two machines on the *Kirby*
    /// `.bk2`: lot NESHAWK-PROFILE-3 measured `zaxnes - bizhawk` at **4 774 to 4 783** cycles and
    /// could not say where it came from. It is two of these — one at the power-on and one at the
    /// reset the movie presses after frame 5 — and `2 * 2 389 = 4 778`. The cycle ledger of frames
    /// 2 to 1 201 shows it directly: BizHawk's frame 1 is **27 392** cycles and so is its frame 5,
    /// where this core's are 29 781 and 29 774 (lot NESHAWK-PROFILE-4 step 2).
    ///
    /// Read at `nes_core.rs` `power_on` and `soft_reset`, through
    /// `NesBusOf::run_ppu_start_up_dots`.
    const PPU_START_UP_DOTS_SKIPPED: i64;

    // ------------------------------------------------------- the DMC's power-on timer

    /// **Item 11** (lot NESHAWK-PROFILE-5 step 2). The DMC's timer powers on **1 020 CPU cycles**
    /// from its first bit rather than one period of the rate in force, so the output unit's whole
    /// phase — and with it the cycle every sample fetch of the game lands on — is `1020 - 428 =
    /// 592` cycles further on than this core's.
    ///
    /// **Where it comes from.** NesHawk's `APU.cs` `DMCUnit` constructor sets `timer_reload =
    /// DMC_RATE[0]` (428 on NTSC) and then `timer = 1020`, with `out_bits_remaining = 7`; `Run()`
    /// is `if (timer > 0) timer--; if (timer == 0) { timer = timer_reload; Clock(); }` and runs
    /// once a CPU cycle out of `RunOneFirst()`, so its first output clock is on CPU cycle 1 020.
    /// This core's `DmcChannel::reset` starts the timer one period out, at `rates[0] - 1`, so its
    /// first clock is on cycle 428. Neither machine's **soft** reset touches it: NesHawk's
    /// `APU.NESSoftReset()` clears the sequencer, writes `$4015 = 0` and clears the two fill
    /// glitches, and this core's `DmcChannel::soft_reset` moves the output level, the IRQ flag and
    /// item 2's countdown — the timer and the bit counter are left alone on both.
    ///
    /// **It is a count in this core's convention, which is NesHawk's minus one.** Both units take
    /// the sample buffer on every eighth output clock (NesHawk counts `7 → 0` and reloads to 7,
    /// this core `8 → 1` and reloads to 8), so the bit counter is the same function and the
    /// timer's start is the whole of the item; and NesHawk's timer is this core's plus one at every
    /// instant — the parity trap `docs/review/NESHAWK-PROFILE.md` records — so its 1 020 is this
    /// core's **1 019**. The value is region-independent, exactly as NesHawk's is: `timer = 1020`
    /// is set whatever `DMC_RATE` table the constructor picked.
    ///
    /// **What it is worth.** The first event of the *Kirby* `.bk2` whose cycle differs once item 10
    /// has removed the drift. Frames 1 to 10 of that movie — the power-on, the movie's soft reset
    /// and the first ten frames — are identical on both machines event for event, and frame 11, the
    /// first that plays a DPCM sample, has NesHawk fetching the sample's **second** byte 211 cycles
    /// earlier: four of the output unit's 54-cycle bit periods, which is this phase (lot
    /// NESHAWK-PROFILE-5 step 1).
    ///
    /// **Why `Standard` cannot simply be moved to it.** This core's phase is load-bearing and
    /// `DmcChannel::reset`'s own comment says so: it decides whether a reload fetch needs its
    /// alignment cycle, and blargg's `sync_dmc` — the thirty ROMs of `dmc_dma_during_read4` and
    /// `sprdma_and_dmc_dma` — has a fine-synchronisation loop written to be one cycle longer than
    /// the DMC's 3 424-cycle byte so that it *drifts into* alignment. 1 020 is **even**, so it
    /// moves the phase by an even number of cycles and leaves that alignment parity alone, which is
    /// what makes the item shippable at all.
    ///
    /// `0` means "as this core has always done it", which is one full period: a timer of 0 would
    /// clock on the first CPU cycle and is not a value either machine can power on with.
    ///
    /// Read at `apu/dmc_channel.rs` `reset`, reached from `nes_core.rs` `power_on` alone.
    const DMC_POWER_ON_TIMER: i32;

    // ------------------------------------------------------- the OAM transfer's wait cycle

    /// **Item 13** (lot NESHAWK-PROFILE-9 step 3, round 1). An OAM transfer's wait cycle is at the
    /// **end**: the transfer's 512 accesses start on the first read-phase cycle **at or after** the
    /// halt cycle — the halt cycle itself doing the first read when it is one — and the CPU loses
    /// **one more cycle after the last `$2004` write**.
    ///
    /// **Measured first, from BizHawk's own bus stream** (`q184.lua`, tree frames 11 to 17 of the
    /// *Kirby* `.bk2`, the two machines in step at a residual of −1). Every transfer of that window
    /// reads on an **even** reported cycle and ends with one cycle carrying no bus access at all:
    ///
    /// | the `$4014` write | leading dead cycles | the 512 accesses | trailing | total |
    /// |---|---|---|---|---|
    /// | reported **even** (293 092) | 1 | 293 094 … 293 605 | **1** (293 606) | **514** |
    /// | reported **odd** (352 651) | **0** | 352 652 … 353 165 | **1** (353 166) | **513** |
    ///
    /// This core's accesses land on exactly the same cycles — a reported cycle is this core's plus
    /// one, so both machines read on the odd cycles of this core's count — and it spends the halt
    /// cycle and then an alignment cycle when it needs one, with nothing at the end: **513 where
    /// NesHawk charges 514, and 514 where NesHawk charges 513**. The totals are not offset, they
    /// are swapped, which is why neither a cycle at the boot nor a parity flip reproduces it.
    ///
    /// **Where it comes from.** NesHawk's `$4014` handler sets `sprdma_countdown = 1`, and
    /// `NES.Core.cs` `RunCpuOne` spends it: `if (apu.dmc.timer % 2 == 0) cpu_deadcounter = 2; else
    /// cpu_deadcounter = 1;` with `oam_dma_exec = true` and `cpu.RDY = false`, then the OAM block
    /// reads or writes on each cycle `cpu_deadcounter` is 0 on. There is no halted re-read of the
    /// CPU's own address anywhere in it — the dead cycles carry no bus access in the stream, where
    /// this core repeats the read the hardware repeats (`dmc_dma_during_read4/dma_4016_read`) — and
    /// the CPU does not run again until the cycle after `oam_dma_exec` goes false.
    ///
    /// **It is not the more accurate chip, as usual**: 513 and 514 are the hardware's own two
    /// figures (nesdev, "DMA"), and `Standard`'s phase is the one blargg's `sprdma_and_dmc_dma`
    /// measures and passes. NesHawk takes the same two figures on the opposite phases.
    ///
    /// Read at `nes_bus.rs` `run_dma`, twice: the first read at the top and the trailing cycle at
    /// the bottom. A fetch owed on the halt cycle keeps the halt (the DMC wins the bus on both
    /// machines, `RunCpuOne`'s `dmc_dma_countdown != 1` gate), so the first half is the no-fetch
    /// case alone.
    const OAM_DMA_WAITS_AT_THE_END: bool;

    // ------------------------------------------------------- where a frame ends

    /// **Item 12** (lot NESHAWK-PROFILE-9 step 1). A frame the movie taps RESET after ends **at
    /// the PPU dot the frame boundary falls on**, before the instruction that straddles it — so
    /// the reset's seven cycles begin on that dot's own CPU cycle rather than after the
    /// straddling instruction has finished.
    ///
    /// **Where it comes from.** NesHawk interleaves the two clocks per cycle: `PPU.run.cs`
    /// `TickPPU_preVBL()` sets `nes.frame_is_done = true` at the dot where `ppur.status.sl`
    /// reaches `241 + preNMIlines`, and the CPU is clocked from inside that same dot loop, so a
    /// frame ends **mid-instruction** as a rule. `NES.Core.cs` `FrameAdvance` then opens the next
    /// frame with `if (resetSignal) { Board.NesSoftReset(); cpu.NESSoftReset(); apu.NESSoftReset();
    /// ppu.NESSoftReset(); }` — the `.bk2`'s RESET is read into `resetSignal` at the **end** of the
    /// frame it is pressed on and applied at the top of the next one — and `MOS 6502X`'s
    /// `NESSoftReset()` is `opcode = VOP_RESET; mi = 0; iflag_pending = true; FlagI = true;`, which
    /// **abandons the instruction in flight**: whatever cycles of it had run are spent, the rest
    /// never run. This core's `NesCoreOf::run_frame_profiled` loop breaks *after* the instruction
    /// `Ppu2C02::vblank_just_started` fired inside, so it runs that whole instruction before the
    /// reset.
    ///
    /// **What it is worth.** The measurement is lot NESHAWK-PROFILE-8 step 1: over frames 1 to 56
    /// of the *Kirby* `.bk2` the four once-a-frame marks read a constant `zaxnes − bizhawk` of
    /// **−1** before the movie's reset and a constant **+2** after it, because at the reset
    /// NesHawk's last cycle of frame 4 is 116 734 and its seven reset cycles are 116 735 to
    /// 116 741, where this core runs the straddling `$C070` (116 734 to 116 736 on its own clock,
    /// which reads one less) and only then resets — **3 CPU cycles late**. The PPU restarts there
    /// (item 10), the game is NMI-synced and the DMC's train is counted in CPU cycles, so those 3
    /// cycles put the game's code 3 cycles late against every DMC fetch for the rest of the movie,
    /// which is what lands NesHawk's halts on write cycles and this core's on read cycles — the
    /// gate on item 1c (`DMC_HALT_LANDS_ON_A_WRITE_CYCLE`).
    ///
    /// **A dot bias is not this item**, and that is measured too (NESHAWK-PROFILE-8 step 2): item
    /// 10's dot count at the reset moves the PPU against the CPU and the post-reset boot's 7-cycle
    /// `$2002` loop re-quantises it, so the residual reads +2, +2, +2, +1, −5 over biases of 0, 3,
    /// 6, 9 and 12 dots and never reaches −1, where ending the frame at the dot moves the whole
    /// machine and leaves the code's position against the PPU alone.
    ///
    /// **The mechanism, and what it does not do.** Read at `nes_core.rs`
    /// `run_frame_profiled_gen`'s loop through `NesBusOf::frame_boundary_is_the_next_cycle`, on
    /// the one frame the movie player announces a RESET after
    /// ([`NesCoreOf::run_frame_before_soft_reset`]). The loop asks, before each instruction,
    /// whether the boundary dot falls inside the **next** CPU cycle and ends the frame there, so
    /// the reset begins on exactly the cycle NesHawk's does whenever the boundary dot is the first
    /// cycle of an instruction — which is the measured case, `$C070` at this core's 116 734. Where
    /// the dot falls *inside* an instruction instead, NesHawk spends that instruction's first
    /// cycles and abandons the rest; this core cannot stop between an instruction's cycles, so it
    /// ends the frame before the instruction and the reset is then up to (its length − 1) cycles
    /// early. Nothing in the one movie there is exercises that, and the honest fix is a
    /// cycle-stepped CPU, not a constant.
    const RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT: bool;

    /// **Item 14** (lot NESHAWK-PROFILE-11 step 2, shipped by NESHAWK-PROFILE-12 step 1). PPU dots
    /// added to — or, negative, taken off — the start-up run of a **soft reset** only, which moves
    /// the PPU against the CPU by that many dots for the rest of the run. `0` on `Standard`, which
    /// is the line this core always had, and **−1** on [`NesHawkCompat`] — measured, costless, and
    /// shipped **on** with the source reading still open. See the note on that value.
    ///
    /// It is the one knob of its shape that is **reachable**.
    /// [`TimingProfile::PPU_START_UP_DOTS_SKIPPED`] is applied at the power-on **and** at the
    /// soft reset, so every value of it gives an **even** net offset — lot NESHAWK-PROFILE-10's
    /// finding, re-measured here at 7 164 and 7 166, both of which are *exactly inert*. A bias at
    /// the **power-on** alone is inert too, over −2 to +3, with −3 (one whole CPU cycle) as the
    /// live control: the post-reset boot's `$2002` wait re-quantises it, because moving the whole
    /// PPU moves the dot the flag rises on **and** the dot the loop sees it on. A dot dropped
    /// **after** that wait moves only the first, and nothing in the game can put it back.
    const SOFT_RESET_PPU_DOT_BIAS: i64;

    /// **Item 15** (lot NESHAWK-PROFILE-12 step 2), the **OAM/DMC interleave** — item 13's
    /// unmeasured half. A DMC sample fetch that falls **inside an OAM transfer** lands three
    /// cycles after the request on NesHawk, exactly as one outside a transfer does, because the
    /// transfer's own cycles stand in for the halt and both re-reads from the request cycle
    /// **inclusive**. This core's transfer loop can only date a halt on the cycle it is *about
    /// to* run, and a request is raised from inside one of the loop's own `tick`s, so its halt
    /// lands one cycle late and every in-transfer fetch is one cycle late with it.
    ///
    /// **Read from `NES.Core.cs` `RunCpuOne`**, whose OAM block and DMC block are two independent
    /// blocks of the *same* cycle: the OAM block runs on every cycle of the transfer except the
    /// fetch's (`oam_dma_exec && apu.dmc_dma_countdown != 1 && !dmc_realign`) and the realign's,
    /// while the DMC block counts `apu.dmc_dma_countdown` down from the 3 the request set and
    /// calls `apu.RunDMCHaltFetch()` — the re-read of the CPU's own address — on each cycle before
    /// the fetch. So the halt and the two re-reads cost the transfer **nothing** and the fetch and
    /// the realign cost it **two** cycles ([`TimingProfile::DMC_FETCH_STALLS_AN_OAM_TRANSFER`],
    /// item 7, is that pair).
    ///
    /// **Measured off BizHawk's own bus stream first** (`q191.lua`, BizHawk frames 35-37), and the
    /// measurement needs no cross-machine calibration at all: the DMC's requests are spaced by the
    /// sample period, so the interval between consecutive **fetch reads** says where each fetch
    /// sits relative to its own request, inside one machine. Over the five fetches that straddle
    /// tree frame 38's transfer NesHawk reads `$FE56`-`$FE5A` at a uniform **432** cycles apart —
    /// the two fetches inside the transfer at the same offset as the three outside it — where this
    /// core reads 432, **431**, 432, 432. Each of the two in-transfer fetches is one cycle late,
    /// and the two cycles are the whole of the movie's first difference at that frame.
    const DMC_FETCH_IN_AN_OAM_TRANSFER_STARTS_AT_THE_REQUEST: bool;

    /// **Item 16** (lot NESHAWK-PROFILE-12 step 3), the other end of item 15's interleave. A DMC
    /// fetch whose own cycle is the one **after an OAM transfer's last `$2004` write** costs that
    /// transfer **one** cycle and not two, because the cycle it takes is the cycle
    /// [`TimingProfile::OAM_DMA_WAITS_AT_THE_END`] already charges as the transfer's wait.
    ///
    /// **Why the two coincide, in `NES.Core.cs` `RunCpuOne`.** Item 7's second cycle is
    /// `dmc_realign`, and all `dmc_realign` does is skip the **OAM block** on the cycle after the
    /// fetch; once the 512th access has run, `oam_dma_exec` is already false and there is no block
    /// left to skip, so the realign costs nothing at all. What the CPU still loses is item 13's
    /// own cycle — `oam_dma_exec` goes false on the last write's cycle and the CPU runs again only
    /// on the cycle after the next — and the fetch is standing on it.
    ///
    /// **Measured off BizHawk's own stream** (`q191.lua`), both cases inside one machine, which is
    /// what makes it a measurement and not an arithmetic:
    ///
    /// | | last `w $2004` | then | then | the CPU runs |
    /// |---|---|---|---|---|
    /// | a fetch at the end (tree frame 38) | 1 097 681 | `r $FE57`, the fetch | one idle cycle | 1 097 684 |
    /// | no fetch at the end (tree frame 36) | 1 067 901 | one idle cycle | — | 1 067 903 |
    ///
    /// So the fetch adds **one** cycle there. Execute to execute across the whole transfer
    /// (`x $C360` to `x $C365`) NesHawk spends 523 cycles on tree frame 38 and **524** on tree
    /// frame 37, whose two inside fetches are both comfortably inside it — one cycle apart on the
    /// same instruction with the same two fetches, which is this item and nothing else.
    const DMC_FETCH_TAKES_THE_OAM_TRANSFER_WAIT_CYCLE: bool;
}

/// Zaxnes's own 2A03: the behaviour every lot before NESHAWK-PROFILE measured, and the one every
/// game, every test ROM and every `.fm2` movie runs on.
///
/// This is the default type parameter everywhere, so `NesCore`, `NesBus` and every existing
/// caller mean this profile without saying so.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Standard;

impl TimingProfile for Standard {
    const NAME: &'static str = "standard";
    const STATE_ID: u8 = 0;
    const STATE_CORE_NAME: &'static str =
        zax_core_api::state_core_name(zax_core_api::EngineKind::Zaxnes);

    const DMC_HALT_ON_REQUEST_CYCLE: bool = false;
    const DMC_FETCH_HAS_TWO_DUMMIES: bool = false;
    const DMC_LENGTH_LAGS_THE_FETCH: bool = false;
    const DMC_ENABLE_WRITES_MID_SAMPLE: bool = false;
    const DMC_HALT_LANDS_ON_A_WRITE_CYCLE: bool = false;
    const DMC_FETCH_STALLS_AN_OAM_TRANSFER: bool = false;
    const PPU_START_UP_DOTS_SKIPPED: i64 = 0;
    const DMC_POWER_ON_TIMER: i32 = 0;
    const OAM_DMA_WAITS_AT_THE_END: bool = false;
    const RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT: bool = false;
    const SOFT_RESET_PPU_DOT_BIAS: i64 = 0;
    const DMC_FETCH_IN_AN_OAM_TRANSFER_STARTS_AT_THE_REQUEST: bool = false;
    const DMC_FETCH_TAKES_THE_OAM_TRANSFER_WAIT_CYCLE: bool = false;
}

/// A 2A03 that makes BizHawk NesHawk's timing choices where they differ from Zaxnes's, so that a
/// `.bk2` recorded on that core replays in step.
///
/// Selected automatically by the `.bk2` player when the movie's header names `NesHawk`, and never
/// for a `.fm2`, for a `QuickNes` `.bk2` or for ordinary play. It is **not** the more accurate
/// chip: see the module note.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct NesHawkCompat;

impl TimingProfile for NesHawkCompat {
    const NAME: &'static str = "neshawk";
    const STATE_ID: u8 = 1;
    const STATE_CORE_NAME: &'static str = "Zaxnes-NesHawk";

    const DMC_HALT_ON_REQUEST_CYCLE: bool = true;
    const DMC_FETCH_HAS_TWO_DUMMIES: bool = true;
    const DMC_LENGTH_LAGS_THE_FETCH: bool = true;
    const DMC_ENABLE_WRITES_MID_SAMPLE: bool = true;
    /// **ON since lot NESHAWK-PROFILE-10**, with item 12 and item 13 beside it. Lot
    /// NESHAWK-PROFILE-9's step 1 opened the gate the note below asks for — item 12 puts the
    /// movie's reset on NesHawk's own cycle, so this item's slip is read off a CPU in the same
    /// place inside the instruction as NesHawk's — and lot 10 ships the three **measured** items
    /// together rather than the pair of cancelling errors that reached a further frame. The three
    /// part the *Kirby* `.bk2` at **9 353** where the all-off profile reaches 45 095 only because
    /// two errors cancel; every one of the three is measured right on its own stream, and the
    /// residual at 9 353 is a further difference and not a reason to retract one of them. It is
    /// **inert** on both *Faxanadu* dumps (247 and 314 either way).
    const DMC_HALT_LANDS_ON_A_WRITE_CYCLE: bool = true;
    const DMC_FETCH_STALLS_AN_OAM_TRANSFER: bool = true;
    const PPU_START_UP_DOTS_SKIPPED: i64 = 7165;

    /// **Item 11 is ON since lot NESHAWK-PROFILE-6**, which answered Q-182 and retracted item 1c:
    /// with `1019` here the *Kirby* `.bk2` went from parting at frame 8 005 to **45 095**. The note
    /// below is the lot that shipped it off and what it was waiting for, kept because the trade it
    /// records is the one every later lot has had to re-read.
    ///
    /// The item is not in doubt: NesHawk's `DMCUnit` constructor sets `timer = 1020`, and with
    /// `1019` here every DMC sample fetch of *Kirby*'s frame 11 lands at a constant offset from
    /// BizHawk's own per-byte fetch ledger where this value of `0` drifts 211 cycles out of it
    /// (lot NESHAWK-PROFILE-5 steps 1 to 3).
    ///
    /// **It is off because half of it is worse than none of it.** Enabled on its own it takes the
    /// *Kirby* `.bk2` from no live difference until frame **8 005** to no live difference until
    /// frame **925** — which is where the movie parted before this profile existed at all — while
    /// moving no other measurement on either profile by one ROM or one line: the thirty DMC/DMA
    /// ROMs, `ppu_vbl_nmi`, `vbl_nmi_timing`, `cpu_interrupts_v2`, `AccuracyCoin` and both
    /// *Faxanadu* dumps are all identical with it and without it. The profile exists to replay a
    /// NesHawk movie, so a change that costs 7 080 frames of the one movie there is and buys
    /// nothing anywhere else is not shippable, however exact it is in isolation.
    ///
    /// **What it is waiting for is one cycle, and `Q-182` is the gate.** With item 11 on, frame 11
    /// is exact on every tag except a single cycle lost at the one reload fetch whose request falls
    /// on a write cycle; charging that fetch four cycles instead of item 1c's three makes frame 11
    /// exact and takes the movie back to 1 100 — but the source says three is right, and the number
    /// that would settle it (the cycle BizHawk's Lua reports a *DMA* read on) is not pinned. Once
    /// Q-182 is answered, this constant and item 1c move together or not at all.
    ///
    /// Setting this to `1019` restores the measured item; leaving it `0` keeps lot
    /// NESHAWK-PROFILE-4's behaviour exactly. `tests/neshawk_profile_tests.rs` measures the
    /// mechanism at `1019` through a profile of its own, so the item stays covered while it is off.
    const DMC_POWER_ON_TIMER: i32 = 1019;

    /// **ON since lot NESHAWK-PROFILE-10** (implemented by lot NESHAWK-PROFILE-9 step 3). The
    /// measurement has never been in doubt — BizHawk's own bus stream puts the hardware's 513 and
    /// 514 cycles on the opposite phases of the `$4014` write, and the table above is read off that
    /// stream with no cross-machine calibration — and it is the movie's **first difference of any
    /// kind**, which is why lot 10 ships it rather than the configuration that cancels it.
    ///
    /// **What it costs, recorded rather than hidden**: on its own it takes *Faxanadu* `[USA]` from
    /// **247** of 832 checksum lines to **202** and the `[Rev 1]` dump from 314 to 210 (item 1c is
    /// inert on *Faxanadu*, so those 45 and 104 lines are this item's alone). The half the
    /// measurement could not reach is the likely cause: NesHawk's behaviour when a DMC fetch is
    /// owed on the transfer's **own halt cycle**, which `run_dma` leaves as this core's halt plus
    /// the trailing cycle because no transfer of the measured window does it. A game with DPCM
    /// under a transfer — which *Faxanadu* has and *Kirby*'s measured frames do not — meets that
    /// case every frame.
    const OAM_DMA_WAITS_AT_THE_END: bool = true;

    /// **ON since lot NESHAWK-PROFILE-10** (implemented by lot NESHAWK-PROFILE-9 step 1). It is the
    /// one of the three whose *own* measurement is a clean pass — the four once-a-frame marks read
    /// the same −1 either side of the movie's reset where they read −1 and then +2 — and it is
    /// worth something only with item 1c beside it, which is why the three move together. On its
    /// own it takes the movie from 45 095 to **4 665**, which is the two-errors-cancelling
    /// arithmetic and not a fault of the item.
    const RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT: bool = true;

    /// Item 14 — **measured at −1 by lot NESHAWK-PROFILE-11 and shipped at −1 by lot
    /// NESHAWK-PROFILE-12, on the measurement and on its costing nothing.** It is *not* item 11's
    /// shape: that item is right in isolation and expensive, where this one is right in isolation
    /// and **free**, so the only thing against it is that the reading is still open — which is a
    /// reason to keep looking, not a reason to run a clock that is known to be one dot out.
    ///
    /// **What it is worth.** At −1 the movie's first difference of any kind moves from tree frame
    /// **14** to tree frame **38**: frames 3 to 37 go in step on *every* event of the ledger where
    /// the shipped profile parts at 14, and `--ram-ref` over frames 1 100-10 000 goes from 237
    /// frames identical to the byte to **459**. The sweep is sharply peaked and signed — the
    /// window is exactly `{-1, -2}`, one cycle-grid position, with `0` parting at frame 14, `+1`
    /// and `+2` parting at frame **11** and `-3`, `-4`, `-5` parting at frame **8**.
    ///
    /// **It costs nothing, and that is structural.** The site is the soft reset, which only a
    /// movie's RESET column reaches, so every ROM of every suite runs the code it ran before —
    /// `test-roms --filter nmi --profile neshawk` stays 20 pass / 0 fail with it on. It is also
    /// the one knob of this shape that `ppu_vbl_nmi` `05-nmi_timing` cannot refuse either way:
    /// that ROM synchronises on `$2002` and times in CPU cycles, so a **uniform** PPU/CPU phase
    /// cancels in both of its terms. Every *rule*-shaped candidate for the same dot has been
    /// refused by it — item 8 by lot NESHAWK-PROFILE-4, and the dot-grained version of the same
    /// question by step 1 of this lot, which fails six ROMs NesHawk itself passes.
    ///
    /// **The reading is still open, and the best candidate is named.** Three were tried and none
    /// predicts a one-dot accumulating offset. `PPU.cs` `Reset()`'s **`idleSynch = true`** is the
    /// best of them and the one to re-read: NesHawk's reset leaves the odd-frame skip **armed**
    /// where `Ppu2C02::soft_reset` sets `odd_frame = false` and disarms it, which is a real
    /// difference at this site and exactly one dot in size. It was implemented faithfully
    /// (`arm_odd_frame_skip`, under the constant, after `soft_reset`) and **its effect
    /// alternates** 0/−1 rather than accumulating, because both machines then skip once every two
    /// frames — so what is missing is whatever makes the first skip after the reset land on only
    /// one of them. `PPU.run.cs` `NewDeadPPU()` reads `241 * 341 - start_up_offset` = 82 177 dots
    /// verbatim, which is the shipped 7 165 and not 7 164, and a change to *that* constant is
    /// applied twice and so cannot give an odd offset at all. And `evenOddDestiny = PPUON` is
    /// sampled at `ppur.status.cycle == 339` against this core's dot 338, which is one dot but only
    /// moves anything when a `$2001` write lands on it.
    ///
    /// **What it does not buy: the wall at 9 353 does not move.** What it buys is the clock, which
    /// is what the next item is measured against — an item hunt whose window opens on a machine
    /// known to be one dot out measures that dot and not the item.
    const SOFT_RESET_PPU_DOT_BIAS: i64 = -1;

    /// Item 15 — the **OAM/DMC interleave**, measured off BizHawk's own stream by lot
    /// NESHAWK-PROFILE-12 step 2 and read in `RunCpuOne` second. The trait's note carries both,
    /// and the measurement is calibration-free: the fetch-read spacing inside one machine.
    ///
    /// It is what the movie's first difference at tree frame **38** is made of, and its size
    /// there is exactly its two in-transfer fetches: the frame's `nmi` and its `$4014` write are
    /// at residual 0, every event from `pc$C37C` to `r$4015` is at **+2**, and the frame's own
    /// `pc$C06E` is back at 0 — because what the transfer's middle loses its end gives back, the
    /// held request's halt and re-reads landing on transfer cycles that were going to run anyway.
    const DMC_FETCH_IN_AN_OAM_TRANSFER_STARTS_AT_THE_REQUEST: bool = true;

    /// Item 16 — item 15's other end, measured off the same stream: a fetch standing on the
    /// transfer's own wait cycle costs it one cycle, not two. The trait's note carries the table.
    const DMC_FETCH_TAKES_THE_OAM_TRANSFER_WAIT_CYCLE: bool = true;
}
