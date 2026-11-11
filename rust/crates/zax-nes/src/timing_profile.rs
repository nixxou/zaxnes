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

    /// **Item 1c** (lot NESHAWK-PROFILE-2 step 3). The halt lands on the request cycle **even when
    /// the CPU is writing on it**, so a request that falls on a run of an *odd* number of write
    /// cycles gives the CPU its cycle back **one cycle earlier** than waiting for the next read
    /// cycle does.
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
    /// Read at `nes_bus.rs` `promote_dmc_wait` and `is_a_lone_dmc_fetch`. Only a **reload** is
    /// affected: a fetch item 3's `$4015` write started is not halted on its request cycle at all.
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
    const DMC_HALT_LANDS_ON_A_WRITE_CYCLE: bool = true;
    const DMC_FETCH_STALLS_AN_OAM_TRANSFER: bool = true;
    const PPU_START_UP_DOTS_SKIPPED: i64 = 7165;
    const DMC_POWER_ON_TIMER: i32 = 1019;
}
