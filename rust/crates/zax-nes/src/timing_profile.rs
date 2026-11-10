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
}
