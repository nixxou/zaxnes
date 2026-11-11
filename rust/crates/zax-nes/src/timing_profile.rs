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
}
