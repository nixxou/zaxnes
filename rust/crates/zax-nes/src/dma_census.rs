//! An instrument, not emulation: what the DMA units, the APU's registers and the two status
//! registers did, cycle by cycle, for one frame.
//!
//! This module exists **only** under the `dma-census` feature, and every one of its call sites in
//! [`crate::NesBusOf`] carries `#[cfg(feature = "dma-census")]` on the statement itself — so with
//! the feature off (the default, and what every other suite is run with) not one instruction of it
//! reaches the compiler, let alone the binary. That is the rule lot NESHAWK-PROFILE-2's brief sets
//! for a diagnostic hook, and it is also what keeps `golden-core` and `bench-core` answerable: the
//! shipped core cannot have paid for this.
//!
//! What it is for: lot NESHAWK-PROFILE closed frame 926 of the *Kirby* `.bk2` by reading NesHawk's
//! source and reasoning about *where* a DMC fetch's halt lands. Frame 4666 needed the next thing —
//! **which** of a frame's fifty-odd fetches cost what, and why — and no instrument said it. Every
//! site below is reached at most once per fetch, once per OAM transfer or once per register access,
//! never per cycle, so the census is cheap even when it is compiled in.

/// What one line of the census is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DmaEventKind {
    /// The DMC's memory reader ran dry, or a `$4015` write found its buffer empty: the request.
    DmcRequest,
    /// The request became a halt — the first cycle the CPU is stopped on.
    DmcHalt,
    /// The sample byte was read.
    DmcFetch,
    /// A `$4014` write asked for an OAM transfer.
    OamRequest,
    /// A write to `$4000`-`$4013`, `$4015` or `$4017`.
    ApuWrite,
    /// A write to `$4016`: the controller ports' strobe.
    ///
    /// Not an APU register and not a DMA, but on NesHawk it is on the same per-cycle path — its
    /// `APU.RunOneLast` applies a strobe on the first cycle with `dmc.timer % 2 == 1` rather than
    /// at the write — so a census that places the DMC's fetches against the cycle has to place
    /// this beside them. Lot NESHAWK-PROFILE-3 step 1 asks for it by name.
    PortStrobe,
    /// A read of `$4015`, whichever unit made it.
    ApuStatusRead,
    /// A read of `$2002`, whichever unit made it.
    PpuStatusRead,
}

impl DmaEventKind {
    /// The seven-character tag the report prints.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::DmcRequest => "request",
            Self::DmcHalt => "halt   ",
            Self::DmcFetch => "fetch  ",
            Self::OamRequest => "oam    ",
            Self::ApuWrite => "write  ",
            Self::PortStrobe => "strobe ",
            Self::ApuStatusRead => "r4015  ",
            Self::PpuStatusRead => "r2002  ",
        }
    }
}

/// One line of the census.
#[derive(Clone, Copy, Debug)]
pub struct DmaEvent {
    /// What it is about.
    pub kind: DmaEventKind,
    /// The bus cycle it happened on. For [`DmaEventKind::DmcHalt`] that is the cycle the halt
    /// lands on, which is the cycle the CPU would otherwise have read on.
    pub cycle: i64,
    /// For [`DmaEventKind::DmcRequest`] and [`DmaEventKind::DmcHalt`], the earliest cycle the halt
    /// is allowed to land on (`NesBus::dmc_ready_cycle`); 0 elsewhere. A reload of the `Standard`
    /// profile holds it to the cycle after the request, the `NesHawkCompat` profile to the request
    /// cycle itself (item 1a), and a `$4015` write to two or three cycles out (item 3).
    pub ready_cycle: i64,
    /// The register for a write or a status read, the sample address for a fetch, the page for an
    /// OAM request.
    pub address: u16,
    /// The byte written, read or fetched.
    pub value: u8,
    /// Whether the request this event belongs to came from a `$4015` write rather than from the
    /// memory reader running dry — the reload/write-started distinction the brief asks for.
    pub from_write: bool,
    /// The DMC's timer at the event.
    pub dmc_timer: i32,
    /// Bits the DMC's output unit still owes before it needs the next byte.
    pub dmc_bits: i32,
    /// Bytes left of the sample.
    pub dmc_bytes: i32,
}

impl DmaEvent {
    /// The cost of the fetch this `DmcFetch` ends, in cycles the CPU lost, given the `DmcHalt`
    /// that started it: the halt cycle, every cycle between, and the fetch cycle.
    #[must_use]
    pub const fn cost_from(&self, halt: &Self) -> i64 {
        self.cycle - halt.cycle + 1
    }
}

/// The census of one frame, in the order the events happened.
#[derive(Clone, Debug, Default)]
pub struct DmaCensus {
    events: Vec<DmaEvent>,
}

impl DmaCensus {
    /// An empty census. `const`, because `NesBusOf::new` is.
    #[must_use]
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Every event recorded since the last [`DmaCensus::clear`].
    #[must_use]
    pub fn events(&self) -> &[DmaEvent] {
        &self.events
    }

    /// Forgets everything, so that the next frame is measured on its own.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Records one event. Only ever reached from a `#[cfg(feature = "dma-census")]` statement.
    pub fn push(&mut self, event: DmaEvent) {
        self.events.push(event);
    }
}
