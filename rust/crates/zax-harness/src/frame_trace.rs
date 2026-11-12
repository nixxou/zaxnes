//! **One frame of a movie, instruction by instruction** — the diagnostic lot EVAL-SYNC-3 step 2
//! needed and the instrument had no equivalent of.
//!
//! `movie-sync` can say *that* two machines part at frame 247 and *which byte* parts
//! (`--ram-ref`); it cannot say which instruction wrote that byte or what the instruction read to
//! decide the value. This does: it plays a movie up to a chosen frame with the ordinary
//! machinery, then drives that one frame **by hand**, capturing the state before every
//! instruction and watching a list of addresses for the write that changes one.
//!
//! # It is outside `zax-nes`, and that is not a detail
//!
//! Nothing here is a change to the core, a `cfg` or a feature: every piece it uses already
//! existed and is public.
//!
//! | what it needs | what it uses |
//! |---|---|
//! | the state before an instruction, disassembled | `zax_nes::cpu::CpuTracer::capture`, which exists for `nestest.log` (REQ-CORE-110) and reads **only** through `CpuBus::peek` (REQ-CORE-081) |
//! | one instruction at a time | `NesCore::cpu_and_bus_mut`, whose own doc comment says "to step instructions by hand" |
//! | where the frame ends | `Ppu2C02::vblank_just_started`, which is what `NesCore::run_frame`'s own loop breaks on |
//! | where the PPU is | `Ppu2C02::scanline` and `dot` |
//!
//! So the traced frame is `run_frame`'s loop, written out, **minus** the three things `run_frame`
//! does around it that no emulation depends on: `defer_ppu` (a batching optimisation whose own
//! doc comment says "batching is exact because nothing outside the PPU can tell when its dots
//! ran"; leaving it off is what makes `scanline`/`dot` exact at every instruction), the margin
//! decoder and the smooth-scroll pass (3D presentation, and this core is loaded with neither),
//! and the frame counter.
//!
//! **That the traced frame is the same frame is checked rather than assumed**: the run takes the
//! work-RAM checksum after the traced frame and again after running the same frame the ordinary
//! way from the same state, and the report prints both. A tracer that perturbed the frame it
//! traced would be worse than none.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use zax_nes::cpu::{CpuBus, CpuTracer};
use zax_nes::rom::InesHeader;
use zax_nes::{NesCoreOf, NesHawkCompat, NesRegion, Standard, TimingProfile};

use crate::movie_file::{MovieFile, RamPattern};
use crate::movie_sync::{read_movie, relocate_resets, resets_next, Machine, Resets, ZaxnesMachine};
use crate::profile_choice::ProfileChoice;
use crate::ram_trace::{checksum, Reference};
use crate::HarnessError;

pub const USAGE: &str = "zax-harness frame-trace <movie.fm2|.bk2> <rom.nes> --frame <n> [options]
    Plays the movie to frame n on the Zaxnes engine, then traces that frame instruction by
    instruction: the state before each one, the PPU's scanline and dot, and every change of a
    watched address with the instruction that made it.
      --frame <n>          the frame to trace (1-based, as movie-sync counts them). Required.
      --frames <n>         trace n consecutive frames from --frame (default 1)
      --watch <addr>       a hex address to watch, repeatable (e.g. --watch 004B)
      --out <file>         write the trace there; without it only the summary is printed
      --context <n>        instructions of trace printed around each watched change (default 12)
      --ram default|ff|zero|random:<seed>   the power-on work RAM, overriding the movie's
      --resets on|off|early|late   as movie-sync: where the movie's reset column lands
      --converge <file>    a per-frame reference carrying `ram=<hex>`: at EVERY instruction of the
                           traced frame, how far this run's own 2 KiB is from the reference's for
                           that frame. A point with 0 differing bytes means the two machines ran
                           the same frame and only wrote it down at different moments
      --extra <n>          instructions past the frame boundary --converge carries on for
                           (default 256)
      --converge-window <n>   print --converge's live difference instruction by instruction for
                           n instructions either side of the frame boundary, with the addresses
                           named. A count says how far apart two machines are; the addresses say
                           in which direction
      --builtin-patches    load a recognised commercial ROM with its 3D widescreen patch, as the
                           application does; off by default, as in movie-sync
      --profile standard|neshawk
                           which 2A03 to trace on. Default: the one the movie was recorded on —
                           the NesHawk-compatible chip for a `.bk2` whose Core is NesHawk
      --dmc-census         every DMC sample fetch of the frame with its request cycle, its kind
                           (reload or started by a $4015 write) and its cost, every OAM transfer,
                           every APU register write and every $4015 and $2002 read, placed against
                           the instruction each happened inside. Needs the `dma-census` feature
                           (an instrument: `cargo build -p zax-harness --features dma-census`)
      --event-log <file>   the CYCLE LEDGER: one line per frame carrying the CPU cycle the frame
                           started on, its length, its instruction count and its DMA bill, then one
                           line per event with the cycle it happened on COUNTED FROM THE FRAME'S
                           START -- each NMI and IRQ handler entry, each --event-pc, and (with
                           --dmc-census) each $4015/$4016/$4017 and other register access. This is
                           the format a BizHawk Lua script can be made to write, so that the two
                           machines can be aligned event for event. It captures no instruction and
                           disassembles nothing, which is what makes a span of thousands of frames
                           affordable; --out and --watch are refused with it for that reason
      --event-pc <addr>    a hex PC whose every execution --event-log records, repeatable";

/// Instructions one traced frame is allowed, before the loop is called broken rather than slow.
///
/// A whole NTSC frame is about 29 780 CPU cycles and the shortest instruction is two, so about
/// 15 000 is the ceiling a real frame can reach; 200 000 is far above any of them and still
/// finite, which is what a hand-written loop on `vblank_just_started` needs.
const MAX_INSTRUCTIONS: usize = 200_000;

/// Whether an address is one of the two **artefact** classes [`crate::ram_trace`] sorts into: the
/// stack page, whose bytes below the stack pointer are dead, and `$0000-$001F`, the scratch of
/// whatever routine happens to be running. EVAL-SYNC-2 measured both on a movie that syncs
/// exactly, so neither can be read as a divergence.
const fn is_artefact(address: usize) -> bool {
    address < 0x0020 || (address >= 0x0100 && address < 0x0200)
}

/// How far two 2 KiB images are apart: differing bytes in total, and differing **live** bytes.
fn diff_counts(ours: &[u8], theirs: &[u8]) -> (u32, u32) {
    let (mut total, mut live) = (0u32, 0u32);
    for (address, (ours, theirs)) in ours.iter().zip(theirs.iter()).enumerate() {
        if ours == theirs {
            continue;
        }
        total += 1;
        if !is_artefact(address) {
            live += 1;
        }
    }
    (total, live)
}

/// How many live addresses one window line names before it only counts them.
const WINDOW_LISTED: usize = 8;

/// The addresses outside the two artefact ranges at which the two 2 KiB differ, up to
/// [`WINDOW_LISTED`] of them.
fn live_addresses(ours: &[u8], theirs: &[u8]) -> Vec<u16> {
    ours.iter()
        .zip(theirs.iter())
        .enumerate()
        .filter(|(address, (ours, theirs))| ours != theirs && !is_artefact(*address))
        .take(WINDOW_LISTED)
        .map(|(address, _)| u16::try_from(address).unwrap_or(u16::MAX))
        .collect()
}

/// The command line of `frame-trace`.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The movie.
    pub movie: PathBuf,
    /// The ROM.
    pub rom: PathBuf,
    /// The first frame traced, 1-based.
    pub frame: u32,
    /// How many consecutive frames to trace.
    pub frames: u32,
    /// Addresses watched for a change.
    pub watch: Vec<u16>,
    /// Where the trace goes.
    pub out: Option<PathBuf>,
    /// Instructions printed either side of a watched change.
    pub context: usize,
    /// The power-on work RAM, overriding the movie's.
    pub ram: Option<RamPattern>,
    /// Where the movie's reset column lands.
    pub resets: Resets,
    /// A per-frame reference carrying `ram=` fields, to find where inside the frame this run's
    /// own 2 KiB comes closest to it. See [`Converge`].
    pub converge: Option<PathBuf>,
    /// Instructions past the frame boundary that `--converge` carries on comparing.
    pub extra: usize,
    /// Instructions either side of the boundary whose live difference is printed in full.
    pub converge_window: usize,
    /// Load a recognised commercial ROM with its 3D built-in patch, as `movie-sync`'s own
    /// `--builtin-patches` does. Off by default, for the same reason.
    pub builtin_patches: bool,
    /// Which 2A03 to trace on, or `None` for the one the movie was recorded on.
    pub profile: Option<ProfileChoice>,
    /// Print the DMA census of every traced frame (`--dmc-census`), which needs the
    /// `dma-census` feature.
    pub dmc_census: bool,
    /// Where the cycle ledger goes (`--event-log`), and the mode it selects: with it the trace
    /// captures no instruction at all. See [`Event`].
    pub event_log: Option<PathBuf>,
    /// PCs whose every execution the cycle ledger records (`--event-pc`).
    pub event_pc: Vec<u16>,
}

/// One traced instruction.
#[derive(Debug, Clone)]
pub struct Step {
    /// The instruction's index inside the frame, from 0.
    pub index: usize,
    /// The PPU's scanline before the instruction.
    pub scanline: i32,
    /// The PPU's dot before the instruction.
    pub dot: i32,
    /// The bus cycle the instruction began on, which is what places a core event — a DMA census
    /// line, say — against the instruction it happened inside.
    pub cycle: i64,
    /// The line, in `nestest.log`'s own shape.
    pub line: String,
    /// `NMI` or `IRQ` when this instruction is the first of an interrupt's handler.
    pub interrupt: Option<&'static str>,
}

impl Step {
    /// The line as the trace file writes it.
    #[must_use]
    pub fn text(&self) -> String {
        format!(
            "{:6} SL:{:3} DOT:{:3} {}{}",
            self.index,
            self.scanline,
            self.dot,
            self.line,
            match self.interrupt {
                Some(which) => format!("   <-- {which} handler"),
                None => String::new(),
            }
        )
    }
}

/// One line of the **cycle ledger** (`--event-log`): something that happened at a known cycle of
/// a frame, named the same way on both machines.
///
/// This is the whole instrument lot NESHAWK-PROFILE-3 step 1 is built on, and its shape is chosen
/// for one purpose: a BizHawk Lua script can write exactly these lines
/// (`emu.totalexecutedcycles()` minus the frame's own start, `event.onmemoryexecute` for a PC,
/// `event.onmemorywrite` for a register), so that two machines can be compared **event for
/// event** instead of only frame for frame. A count of differing bytes says two machines parted;
/// the first event whose cycle differs says *where*.
///
/// The cycle is counted **from the frame's first cycle**, not absolutely: the two machines start
/// their runs at different absolute cycles (a power-on offset, and NesHawk's own counter is not
/// this core's), and nothing about that offset is interesting.
#[derive(Debug, Clone)]
pub struct Event {
    /// Cycles from the frame's first.
    pub offset: i64,
    /// The PPU scanline when it happened, or -1 when the event carries none.
    pub scanline: i32,
    /// The PPU dot when it happened, or -1.
    pub dot: i32,
    /// What it is: `nmi`, `irq`, `pc$XXXX`, or a census tag such as `w$4017`.
    pub tag: String,
    /// Whatever the kind carries beside the cycle: the registers for a PC, the byte for a write.
    pub detail: String,
}

impl Event {
    /// The line the ledger file writes.
    #[must_use]
    pub fn text(&self, frame: u32) -> String {
        format!(
            "  {frame} {:>6} {:<8} {:>3}:{:<4} {}",
            self.offset, self.tag, self.scanline, self.dot, self.detail
        )
    }
}

/// A watched address changing, with the instruction that changed it.
#[derive(Debug, Clone)]
pub struct Change {
    /// The address.
    pub address: u16,
    /// The instruction's index inside the frame.
    pub index: usize,
    /// What it held before.
    pub was: u8,
    /// What it holds now.
    pub now: u8,
}

/// One frame's trace.
#[derive(Debug, Clone, Default)]
pub struct FrameTrace {
    /// The frame's number, as `movie-sync` counts them.
    pub frame: u32,
    /// Every instruction of the frame. **Empty in `--event-log` mode**, which captures none.
    pub steps: Vec<Step>,
    /// The cycle ledger's events, in the order they happened. Empty without `--event-log`.
    pub events: Vec<Event>,
    /// The absolute bus cycle the frame started on, which the ledger's header line carries so
    /// that a frame of a different *length* shows up as a difference of its own.
    pub start_cycle: i64,
    /// Instructions the frame ran, which the ledger counts even when it captures none of them.
    pub instructions: usize,
    /// Cycles the frame's DMC fetches charged the CPU in all; only under the `dma-census`
    /// feature, and 0 without it.
    pub dmc_cost: i64,
    /// Every change of a watched address.
    pub changes: Vec<Change>,
    /// The work-RAM checksum after the traced frame.
    pub checksum: u32,
    /// The input word the frame was fed.
    pub word: u16,
    /// Whether the loop hit [`MAX_INSTRUCTIONS`] instead of the frame boundary.
    pub overran: bool,
    /// CPU cycles the frame took.
    pub cycles: i64,
    /// Instructions that cost the machine **no CPU cycle at all**.
    ///
    /// On a NES there are none: every instruction is at least two cycles. The one thing in this
    /// core that can produce one is the 3D extension's **overclock**
    /// (`WideExtension::take_stall_cycle`), which freezes the PPU, the APU and the clock while
    /// the CPU runs — so a non-zero count here is the extension taking part in a measurement
    /// that is supposed to be of a plain NES, and it is counted and reported for that reason.
    pub stalled: usize,
    /// Whether the 3D extension is unlocked on this machine (`$5000` has been written `WIDE`,
    /// or a built-in patch booted it).
    pub unlocked: bool,
    /// DMC sample fetches during the frame, and OAM transfers, from `NesBus`'s own counters.
    ///
    /// A DMC fetch **halts the CPU** for three or four cycles (nesdev, "DMA"), so a frame's count
    /// is the size of the one mechanism that moves the program's position inside the frame without
    /// any read or branch deciding it. EVAL-SYNC-4 step 3 needed the number; both getters are
    /// public and exist for the DMA timing tests.
    pub dmc_fetches: i64,
    /// OAM transfers asked for by a `$4014` write during the frame.
    pub oam_transfers: i64,
    /// What `--converge` found: where inside this frame the run's own 2 KiB comes closest to the
    /// reference's recorded 2 KiB for the same frame.
    pub converge: Option<Converge>,
    /// What `--dmc-census` found: every DMC sample fetch of the frame with its request cycle, its
    /// kind and its cost, every OAM transfer, every APU register write and every `$4015` and
    /// `$2002` read. Only under the `dma-census` feature.
    #[cfg(feature = "dma-census")]
    pub census: Vec<CensusLine>,
}

/// One line of `--dmc-census`: a core event placed against the instruction it happened inside.
///
/// The core's own event ([`zax_nes::dma_census::DmaEvent`]) carries absolute bus cycles, which say
/// nothing to a reader. This is the same event with the frame's own cycle, the instruction index
/// and the PPU position put beside it, which is what makes two runs comparable line for line.
#[cfg(feature = "dma-census")]
#[derive(Debug, Clone)]
pub struct CensusLine {
    /// The core's event.
    pub event: zax_nes::dma_census::DmaEvent,
    /// The event's cycle counted from the frame's first.
    pub cycle: i64,
    /// The earliest cycle the halt was allowed on, counted the same way; `None` when the event
    /// does not carry one.
    pub ready: Option<i64>,
    /// The instruction the event happened inside, and its PPU position.
    pub index: usize,
    /// The PPU scanline at the start of that instruction.
    pub scanline: i32,
    /// The PPU dot at the start of that instruction.
    pub dot: i32,
    /// For a `DmcFetch`, the cycles the CPU lost to the fetch it ends — the halt cycle, every
    /// cycle between and the fetch cycle. `None` for every other kind, and for a fetch whose halt
    /// is not in this frame's census.
    pub cost: Option<i64>,
    /// The PC of the instruction the event happened inside, and the cycles from that instruction's
    /// first to the next instruction's first — its **opcode-to-opcode gap**, cost included.
    ///
    /// This is here for one reason (lot NESHAWK-PROFILE-6 step 1): a BizHawk Lua script can read
    /// the same two numbers off its own bus stream, and `gap - cost` is the instruction's
    /// undisturbed length, which both machines must agree on. So `gap_bizhawk - (gap - cost)` is
    /// **what NesHawk charged the same fetch**, with no opcode cycle table and no calibration of
    /// the cycle a DMA read is reported on — the question Q-182 could not answer. `None` for the
    /// last instruction of a frame, which has no next.
    pub gap: Option<(u16, i64)>,
}

/// Where inside a frame a run's work RAM comes closest to a reference's recorded 2 KiB.
///
/// This is the measurement that tells a **divergence** from a **read-point artefact**, and it is
/// the one thing a checksum trace can never do. A reference line says "at the start of frame *f*
/// the 2 KiB was this"; it does not say at which dot of the previous frame the reading was taken.
/// If some instruction of our own frame *f* holds the reference's bytes **exactly**, then the two
/// machines ran the same frame and only wrote it down at different moments; if none does, the two
/// machines genuinely parted.
#[derive(Debug, Clone, Default)]
pub struct Converge {
    /// Instructions compared: the frame's own, plus the ones past its boundary.
    pub compared: usize,
    /// How many past the frame boundary were compared.
    pub extra: usize,
    /// The fewest differing bytes seen, and the instruction index it was seen at.
    pub best: (u32, usize),
    /// The same for the **live** bytes alone — outside the stack page and `$0000-$001F`.
    ///
    /// This is the figure that decides, and the total above is not. The bytes of the stack below
    /// the stack pointer are dead: they are leftovers of pushes already popped, and two machines
    /// that have taken their interrupts at even slightly different points will never agree on
    /// them again however in step they are. So "identical to the byte" is unreachable by
    /// construction, while "no live byte differs" is exactly the question.
    pub best_live: (u32, usize),
    /// The instruction index of the frame's own boundary, which is where the ordinary run reads
    /// the 2 KiB.
    pub boundary: usize,
    /// Differing bytes at the boundary itself.
    pub at_boundary: u32,
    /// Every instruction index at which the 2 KiB is identical to the reference's.
    pub exact: Vec<usize>,
    /// Every instruction index at which no **live** byte differs.
    pub exact_live: Vec<usize>,
    /// Live bytes differing at the frame boundary.
    pub live_at_boundary: u32,
    /// How many instructions either side of the boundary [`Self::window`] keeps.
    pub window_half: usize,
    /// `(index, total, live, the live addresses)` for the instants near the boundary.
    ///
    /// The counts alone say *how far* two machines are apart and never *in which direction*. A
    /// frame whose three differing bytes turn out to be exactly what the next seventeen
    /// instructions write is a frame whose reference was read **later** than our boundary, and
    /// that cannot be read off a count. EVAL-SYNC-4 step 3 is the frame it was built for.
    pub window: Vec<(usize, u32, u32, Vec<u16>)>,
    /// Whether [`Self::take`] has passed the frame boundary, so that the window stops sliding.
    past_boundary: bool,
    /// Instants kept past the boundary.
    window_after: usize,
    /// Whether the window has all it asked for.
    window_done: bool,
}

/// What a run found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// One entry per traced frame.
    pub frames: Vec<FrameTrace>,
    /// The checksum the **ordinary** path gives for the first traced frame, from the same state.
    ///
    /// The tracer's own answer beside it is the check that tracing changed nothing.
    pub control: u32,
    /// What the reader had to decide about the movie.
    pub notes: Vec<String>,
}

impl Converge {
    /// Takes one instant: instruction `index`, with the run's work RAM and the reference's.
    ///
    /// Answers the pair [`diff_counts`] gives, so that the caller can report the boundary's own.
    fn take(&mut self, index: usize, ram: &[u8], wanted: &[u8]) -> (u32, u32) {
        let (total, live) = diff_counts(ram, wanted);
        self.compared += 1;
        // The window wanted is [boundary - half, boundary + half], and the boundary is not known
        // until it arrives: so before it, the last `half + 1` instants are kept as a sliding
        // tail, and after it the window fills up once and stops.
        if self.window_half > 0 && !self.window_done {
            self.window
                .push((index, total, live, live_addresses(ram, wanted)));
            if self.past_boundary {
                self.window_after += 1;
                self.window_done = self.window_after >= self.window_half;
            } else if self.window.len() > self.window_half + 1 {
                self.window.drain(..1);
            }
        }
        if total < self.best.0 {
            self.best = (total, index);
        }
        if live < self.best_live.0 {
            self.best_live = (live, index);
        }
        if total == 0 {
            self.exact.push(index);
        }
        if live == 0 {
            self.exact_live.push(index);
        }
        (total, live)
    }

    /// The paragraph the report prints for one frame.
    #[must_use]
    pub fn report(&self) -> String {
        let found = self;
        let mut text = String::new();
        let _ = writeln!(
            text,
            "    against the reference's own 2 KiB for this frame, over {} instructions \
                 ({} of them past the frame boundary at instruction {}):",
            found.compared, found.extra, found.boundary
        );
        let _ = writeln!(
            text,
            "      at the frame boundary -- where the ordinary run reads the 2 KiB -- \
                 {} byte(s) differ",
            found.at_boundary
        );
        let _ = writeln!(
            text,
            "      the closest it ever comes is {} byte(s) at instruction {}, and {} \
                 LIVE byte(s) at instruction {}",
            found.best.0, found.best.1, found.best_live.0, found.best_live.1
        );
        let _ = writeln!(
            text,
            "      {} live byte(s) differ at the boundary",
            found.live_at_boundary
        );
        let away = |index: usize| {
            i64::try_from(index).unwrap_or(0) - i64::try_from(found.boundary).unwrap_or(0)
        };
        match (found.exact_live.first(), found.exact_live.last()) {
            (Some(first), Some(last)) => {
                let _ = writeln!(
                    text,
                    "      NO live byte differs at {} instruction(s) of this frame, from \
                         {first} ({:+} from the boundary) to {last} ({:+}): the two machines \
                         ran the same frame and read the 2 KiB at different moments",
                    found.exact_live.len(),
                    away(*first),
                    away(*last)
                );
            }
            _ => {
                let _ = writeln!(
                    text,
                    "      at NO instant of this frame does the live memory agree, so \
                         the two machines did not run the same frame"
                );
            }
        }
        if let Some(first) = found.exact.first() {
            let _ = writeln!(
                text,
                "      and the whole 2 KiB, dead stack bytes included, agrees at {} \
                     instruction(s), the first at {first} ({:+})",
                found.exact.len(),
                away(*first)
            );
        }
        if !found.window.is_empty() {
            let _ = writeln!(
                text,
                "      the live difference instruction by instruction across the boundary \
                     (`<` before it, `>` after):"
            );
            for (index, total, live, addresses) in &found.window {
                let marker = match index.cmp(&found.boundary) {
                    std::cmp::Ordering::Less => '<',
                    std::cmp::Ordering::Equal => '=',
                    std::cmp::Ordering::Greater => '>',
                };
                let named = addresses.iter().fold(String::new(), |mut text, address| {
                    let _ = write!(text, " ${address:04X}");
                    text
                });
                let _ = writeln!(
                    text,
                    "      {marker} {index} ({:+}): {total} byte(s), {live} live{named}",
                    away(*index)
                );
            }
        }
        text
    }
}

impl Report {
    /// The summary printed on the terminal.
    #[must_use]
    pub fn text(&self, options: &Options) -> String {
        let mut text = String::new();
        for note in &self.notes {
            let _ = writeln!(text, "  note: {note}");
        }
        let Some(first) = self.frames.first() else {
            return text;
        };
        let _ = writeln!(
            text,
            "  the traced frame and the same frame run the ordinary way: {:08x} and {:08x} -- {}",
            first.checksum,
            self.control,
            if first.checksum == self.control {
                "IDENTICAL, so tracing changed nothing"
            } else {
                "THEY DIFFER, so this trace is not of the frame the movie plays"
            }
        );
        for frame in &self.frames {
            let _ = writeln!(
                text,
                "  frame {}: {} instructions, input word {:04X}{}",
                frame.frame,
                frame.instructions,
                frame.word,
                if frame.overran {
                    " -- OVERRAN: the frame boundary was never reached"
                } else {
                    ""
                }
            );
            let interrupts = frame
                .steps
                .iter()
                .filter(|step| step.interrupt.is_some())
                .count();
            let _ = writeln!(
                text,
                "    {} CPU cycles, {interrupts} interrupt handler entries, the 3D extension {}",
                frame.cycles,
                if frame.unlocked { "UNLOCKED" } else { "locked" }
            );
            let _ = writeln!(
                text,
                "    {} DMC sample fetch(es) and {} OAM transfer(s): a fetch halts the CPU for \
                 3 or 4 cycles and a transfer for 513 or 514",
                frame.dmc_fetches, frame.oam_transfers
            );
            if frame.stalled > 0 {
                let _ = writeln!(
                    text,
                    "    {} instruction(s) cost NO cpu cycle: the extension's overclock ran,                      which a plain NES has no equivalent of",
                    frame.stalled
                );
            }
            if let Some(found) = &frame.converge {
                text.push_str(&found.report());
            }
            #[cfg(feature = "dma-census")]
            if options.dmc_census {
                text.push_str(&census_report(&frame.census));
            }
            if frame.changes.is_empty() && !options.watch.is_empty() {
                let _ = writeln!(text, "    no watched address changed in this frame");
            }
            for change in &frame.changes {
                let _ = writeln!(
                    text,
                    "    ${:04X}: {:02X} -> {:02X} at instruction {}",
                    change.address, change.was, change.now, change.index
                );
                let first = change.index.saturating_sub(options.context);
                let last = (change.index + 1).min(frame.steps.len());
                for step in &frame.steps[first..last] {
                    let marker = if step.index == change.index { '>' } else { ' ' };
                    let _ = writeln!(text, "    {marker} {}", step.text());
                }
            }
        }
        text
    }

    /// The **cycle ledger** (`--event-log`): a header line per frame and a line per event.
    ///
    /// The format is the contract between the two machines, so it is written down here rather
    /// than left implicit in a `writeln!`:
    ///
    /// ```text
    /// F <frame> start=<abs cycle> len=<cycles> instr=<n> dmc=<fetches>/<cycles> oam=<n>
    ///   <frame> <cycle from the frame's start> <tag> <SL>:<dot> <detail>
    /// ```
    ///
    /// `start` is absolute and the two machines' absolute counters have no reason to agree; it is
    /// carried because its **difference between consecutive frames** is the frame's length, and a
    /// frame of a different length on one machine is an immediate pointer. Every event's own
    /// cycle is counted from the frame's start, which is what makes the two comparable.
    #[must_use]
    pub fn ledger_text(&self) -> String {
        let mut text = String::new();
        for frame in &self.frames {
            let _ = writeln!(
                text,
                "F {} start={} len={} instr={} dmc={}/{} oam={}",
                frame.frame,
                frame.start_cycle,
                frame.cycles,
                frame.instructions,
                frame.dmc_fetches,
                frame.dmc_cost,
                frame.oam_transfers
            );
            for event in &frame.events {
                let _ = writeln!(text, "{}", event.text(frame.frame));
            }
        }
        text
    }

    /// The trace file: every instruction of every traced frame.
    #[must_use]
    pub fn trace_text(&self) -> String {
        let mut text = String::new();
        for frame in &self.frames {
            let _ = writeln!(
                text,
                "=== frame {} ({} instructions, word {:04X}, checksum {:08x})",
                frame.frame,
                frame.steps.len(),
                frame.word,
                frame.checksum
            );
            for step in &frame.steps {
                let _ = writeln!(text, "{}", step.text());
            }
            for change in &frame.changes {
                let _ = writeln!(
                    text,
                    "--- ${:04X}: {:02X} -> {:02X} at instruction {}",
                    change.address, change.was, change.now, change.index
                );
            }
        }
        text
    }
}

/// Plays the movie to the chosen frame and traces it.
///
/// # Errors
/// [`HarnessError`] when a file cannot be read, the engine refuses the ROM, the movie is not one
/// this instrument reads, or the movie is shorter than the frame asked for.
pub fn run(options: &Options) -> Result<Report, HarnessError> {
    if options.frame == 0 {
        return Err(HarnessError::Usage(format!(
            "--frame counts from 1, as movie-sync does\n{USAGE}"
        )));
    }
    let image = fs::read(&options.rom)?;
    let movie_bytes = fs::read(&options.movie)?;
    let header = InesHeader::try_parse(&image).ok();
    let movie = relocate_resets(
        read_movie(&options.movie, &movie_bytes, header.is_some())?,
        options.resets,
    );
    let last = options.frame + options.frames.max(1) - 1;
    if usize::try_from(last).unwrap_or(usize::MAX) > movie.frames.len() {
        return Err(HarnessError::Usage(format!(
            "the movie has {} frames, so frame {last} cannot be traced",
            movie.frames.len()
        )));
    }
    let ram = options.ram.unwrap_or(movie.ram);
    let region = if movie.pal_stated {
        if movie.pal {
            NesRegion::Pal
        } else {
            NesRegion::Ntsc
        }
    } else {
        header
            .as_ref()
            .map_or(NesRegion::Ntsc, |header| header.region)
    };
    let name = options
        .rom
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());

    match options.profile.unwrap_or_else(|| movie.recorded_profile()) {
        ProfileChoice::Standard => {
            trace_on::<Standard>(options, &movie, &image, &name, region, ram, last)
        }
        ProfileChoice::NesHawk => {
            trace_on::<NesHawkCompat>(options, &movie, &image, &name, region, ram, last)
        }
    }
}

/// The trace itself, on the timing profile the caller chose.
///
/// Generic over the profile because the trace reaches into the core — `cpu_and_bus_mut`, the PPU's
/// scan position, the bus's DMA counters — rather than driving it through `Machine`, so there is no
/// `dyn` to hide the chip behind. `run` above makes the one choice and this runs on it.
#[allow(clippy::too_many_arguments)] // Seven, and every one of them is read by the loop below.
fn trace_on<P: TimingProfile>(
    options: &Options,
    movie: &MovieFile,
    image: &[u8],
    name: &str,
    region: NesRegion,
    ram: RamPattern,
    last: u32,
) -> Result<Report, HarnessError> {
    let mut machine = ZaxnesMachine::<P>::new(image, name, region, options.builtin_patches)
        .map_err(HarnessError::Rom)?;
    // The movie's own sockets, before the power-on: a `.bk2` can say a port is empty, and an
    // empty port answers a ninth read differently from a pad (`ZaxnesMachine::plug_ports`).
    // Without this the ledger this tool writes would be of a machine the movie did not record.
    machine.plug_ports(movie.ports);
    machine.power(ram).map_err(HarnessError::Rom)?;
    // Everything before the first traced frame is the ordinary path, so the state the trace
    // starts from is the state the movie puts the machine in and nothing else.
    for frame in 1..options.frame {
        let input = movie.frames[usize::try_from(frame).unwrap_or(1) - 1];
        play(&mut machine, input, resets_next(movie, frame))?;
    }

    // The control: the first traced frame, run the ordinary way, from this exact state. It is
    // taken first and undone by `restore`, so the trace itself starts from the same bytes.
    let input = movie.frames[usize::try_from(options.frame).unwrap_or(1) - 1];
    machine.capture().map_err(HarnessError::Rom)?;
    play(&mut machine, input, resets_next(movie, options.frame))?;
    let control = checksum(machine.core.bus().ram());
    machine.restore().map_err(HarnessError::Rom)?;

    let mut report = Report {
        control,
        notes: movie.notes.clone(),
        ..Report::default()
    };
    let wanted = match &options.converge {
        None => std::collections::BTreeMap::new(),
        Some(path) => {
            let text = fs::read_to_string(path)?;
            let reference = Reference::parse(&text)
                .map_err(|why| HarnessError::Format(format!("{}: {why}", path.display())))?;
            if reference.ram.is_empty() {
                return Err(HarnessError::Format(format!(
                    "{}: no line of this reference carries a `ram=<hex digits>` field",
                    path.display()
                )));
            }
            reference.ram
        }
    };

    for frame in options.frame..=last {
        let input = movie.frames[usize::try_from(frame).unwrap_or(1) - 1];
        match input.command {
            crate::movie_file::FrameCommand::None => {}
            crate::movie_file::FrameCommand::SoftReset => {
                machine.soft_reset().map_err(HarnessError::Rom)?;
            }
            crate::movie_file::FrameCommand::HardReset => {
                machine.power(ram).map_err(HarnessError::Rom)?;
            }
        }
        // The convergence probe steps **past** the frame boundary, so the machine it leaves
        // behind is not the one the next traced frame must start from. The state before the
        // frame is kept, put back, and the frame then run the ordinary way — which the control
        // above has already shown lands in the same place as the trace.
        let probing = wanted.get(&frame);
        if probing.is_some() {
            machine.capture().map_err(HarnessError::Rom)?;
        }
        let mut traced = trace_frame(
            &mut machine.core,
            input.word,
            &options.watch,
            probing.map(|bytes| (bytes.as_slice(), options.extra)),
            options.converge_window,
            options.dmc_census,
            options
                .event_log
                .as_ref()
                .map(|_| options.event_pc.as_slice()),
            resets_next(movie, frame),
        );
        traced.frame = frame;
        report.frames.push(traced);
        if probing.is_some() {
            machine.restore().map_err(HarnessError::Rom)?;
            if resets_next(movie, frame) {
                machine
                    .run_before_soft_reset(input.word)
                    .map_err(HarnessError::Rom)?;
            } else {
                machine.run(input.word).map_err(HarnessError::Rom)?;
            }
        }
    }

    if let Some(path) = &options.out {
        fs::write(path, report.trace_text())?;
    }
    if let Some(path) = &options.event_log {
        fs::write(path, report.ledger_text())?;
    }
    Ok(report)
}

/// One frame of the ordinary path, command and all.
fn play<P: TimingProfile>(
    machine: &mut ZaxnesMachine<P>,
    input: crate::movie_file::FrameInput,
    resets_next: bool,
) -> Result<(), HarnessError> {
    match input.command {
        crate::movie_file::FrameCommand::None => {}
        crate::movie_file::FrameCommand::SoftReset => {
            machine.soft_reset().map_err(HarnessError::Rom)?;
        }
        // A power cycle inside the run-up needs the pattern the caller chose, which `Machine`'s
        // own `power` takes; the frames before a traced one are played by `run` for that reason.
        crate::movie_file::FrameCommand::HardReset => {
            return Err(HarnessError::Usage(
                "frame-trace: a Power command inside the run-up is not supported; trace a frame \
                 before it or use --resets off"
                    .to_owned(),
            ))
        }
    }
    if resets_next {
        machine
            .run_before_soft_reset(input.word)
            .map_err(HarnessError::Rom)?;
    } else {
        machine.run(input.word).map_err(HarnessError::Rom)?;
    }
    Ok(())
}

/// What one instruction of the loop below records, which is the one thing the two modes differ in.
///
/// Without `ledger`: a full [`Step`], disassembled as `nestest.log` writes it. With it: a bare
/// `Step` carrying only the cycle and the PPU position — kept so that a census event can still be
/// placed against the instruction it happened inside, dropped with the frame — plus an [`Event`]
/// when this instruction is an interrupt handler's first or one of the watched PCs. No
/// `CpuTracer::capture` and no disassembly in that mode, which is what lets it run over thousands
/// of frames.
fn note_instruction<P: TimingProfile>(
    out: &mut FrameTrace,
    cpu: &zax_nes::cpu::Cpu6502,
    bus: &zax_nes::NesBusOf<P>,
    index: usize,
    interrupt: Option<&'static str>,
    ledger: Option<&[u16]>,
    first_cycle: i64,
) {
    let (scanline, dot, cycle) = (bus.ppu().scanline(), bus.ppu().dot(), bus.cycle_count());
    out.steps.push(Step {
        index,
        scanline,
        dot,
        cycle,
        line: match ledger {
            None => CpuTracer::capture(cpu, bus).format(),
            Some(_) => String::new(),
        },
        interrupt,
    });
    let Some(watched) = ledger else { return };
    let pc = cpu.pc;
    if interrupt.is_none() && !watched.contains(&pc) {
        return;
    }
    out.events.push(Event {
        offset: cycle - first_cycle,
        scanline,
        dot,
        tag: match interrupt {
            Some(which) => which.to_ascii_lowercase(),
            None => format!("pc${pc:04X}"),
        },
        detail: format!(
            "A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} i={index}",
            cpu.a,
            cpu.x,
            cpu.y,
            cpu.p(),
            cpu.s
        ),
    });
}

/// **Item 12** on the frame the movie taps RESET after: the `NesHawkCompat` profile's frame ends
/// at the boundary dot, before the instruction that straddles it, because that is where NesHawk
/// ends a frame and so where its reset's seven cycles begin.
///
/// `NesCoreOf::run_frame_profiled_gen`'s own test, and it has to be here too: the marks this
/// instrument writes are what the item is measured on. With `--converge` the window does not
/// slide past this one frame's boundary — a probe of the frame before a reset would be comparing
/// a frame that ends three cycles from here.
fn ends_at_the_boundary_dot<P: TimingProfile>(
    out: &mut FrameTrace,
    bus: &mut zax_nes::NesBusOf<P>,
    at: (usize, (i64, i64, i64), bool),
    before_reset: bool,
) -> bool {
    let (index, first, census) = at;
    if !(before_reset
        && P::RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT
        && bus.frame_boundary_is_the_next_cycle())
    {
        return false;
    }
    bus.ppu_mut().capture_frame_snapshot();
    out.instructions = index;
    close_frame(out, bus, first, census);
    true
}

/// Drives one frame instruction by instruction, which is `NesCore::run_frame`'s own loop.
///
/// `probe` is the reference's own 2 KiB for this frame and how many instructions to carry on
/// past the frame boundary; see [`Converge`].
#[allow(clippy::too_many_arguments)] // Each is one of the modes the one loop below has to serve.
fn trace_frame<P: TimingProfile>(
    core: &mut NesCoreOf<P>,
    word: u16,
    watch: &[u16],
    probe: Option<(&[u8], usize)>,
    window_half: usize,
    census: bool,
    ledger: Option<&[u16]>,
    before_reset: bool,
) -> FrameTrace {
    let mut out = FrameTrace {
        word,
        ..FrameTrace::default()
    };
    let nmi_vector = u16::from_le_bytes([core.read_memory(0xFFFA), core.read_memory(0xFFFB)]);
    let irq_vector = u16::from_le_bytes([core.read_memory(0xFFFE), core.read_memory(0xFFFF)]);
    let (cpu, bus) = core.cpu_and_bus_mut();
    bus.input_mut().begin_frame(word);
    bus.ppu_mut().clear_frame_boundary();
    out.unlocked = bus.ppu().wide().unlocked();
    let first = (bus.cycle_count(), bus.dmc_dma_count(), bus.oam_dma_count());
    out.start_cycle = first.0;
    // The census is a per-frame record, so it starts empty here and is read at the boundary.
    #[cfg(feature = "dma-census")]
    if census {
        bus.clear_dma_census();
    }
    let mut held: Vec<u8> = watch.iter().map(|address| bus.peek(*address)).collect();
    let mut converge = probe.map(|(wanted, extra)| {
        (
            wanted,
            extra,
            Converge {
                best: (u32::MAX, 0),
                best_live: (u32::MAX, 0),
                window_half,
                ..Converge::default()
            },
        )
    });
    let mut boundary: Option<usize> = None;
    for index in 0..MAX_INSTRUCTIONS {
        let at = (index, first, census);
        if boundary.is_none() && ends_at_the_boundary_dot(&mut out, bus, at, before_reset) {
            boundary = Some(index);
            break;
        }
        let pc = cpu.pc;
        let interrupt = match pc {
            // The handler's first instruction: the vector was just taken. Reported rather than
            // inferred from the stack, because the two vectors are what the ROM itself names.
            _ if pc == nmi_vector => Some("NMI"),
            _ if pc == irq_vector && irq_vector != nmi_vector => Some("IRQ"),
            _ => None,
        };
        if boundary.is_none() {
            note_instruction(&mut out, cpu, bus, index, interrupt, ledger, first.0);
        }
        if cpu.step(bus) == 0 {
            out.stalled += 1;
        }
        for (slot, address) in watch.iter().enumerate() {
            let now = bus.peek(*address);
            let was = held.get(slot).copied().unwrap_or(now);
            if now == was {
                continue;
            }
            if boundary.is_none() {
                out.changes.push(Change {
                    address: *address,
                    index,
                    was,
                    now,
                });
            }
            if let Some(slot) = held.get_mut(slot) {
                *slot = now;
            }
        }
        // How far this instant is from the reference's recorded 2 KiB, every instruction.
        let differing = converge
            .as_mut()
            .map(|(wanted, _, found)| found.take(index, bus.ram(), wanted));

        if boundary.is_none() && bus.ppu().vblank_just_started() {
            out.instructions = index + 1;
            close_frame(&mut out, bus, first, census);
            boundary = Some(index);
            if let Some((_, _, found)) = converge.as_mut() {
                found.boundary = index;
                let (total, live) = differing.unwrap_or((0, 0));
                found.at_boundary = total;
                found.live_at_boundary = live;
                // From here the window stops sliding and fills forward instead.
                found.past_boundary = true;
            }
        }
        match (boundary, converge.as_ref()) {
            // No probe: the frame ends at its own boundary, as `run_frame` does.
            (Some(_), None) => break,
            // A probe: carry on for the instructions it asked for.
            (Some(at), Some((_, extra, _))) if index >= at + *extra => break,
            _ => {}
        }
    }
    if boundary.is_none() {
        out.overran = true;
        out.instructions = MAX_INSTRUCTIONS;
        close_frame(&mut out, bus, first, census);
    }
    out.converge = converge.map(|(_, _, mut found)| {
        found.extra = found.compared.saturating_sub(found.boundary + 1);
        found
    });
    // A ledger run keeps thousands of frames at once, and nothing past this point reads the bare
    // steps a ledger frame pushed: they are the frame's own scaffolding and go with it.
    if ledger.is_some() {
        out.steps = Vec::new();
    }
    out
}

/// The `--dmc-census` section: one line per event, then the totals that say where a frame's
/// cycles went.
///
/// The totals are the point. A frame is a fixed 29 780 or 29 781 CPU cycles, so two machines that
/// run the same frame and get different amounts of work done differ by **what the DMA charged
/// them**, and nothing else — which is exactly the quantity lot NESHAWK-PROFILE-2 step 2 is after.
#[cfg(feature = "dma-census")]
fn census_report(lines: &[CensusLine]) -> String {
    use core::fmt::Write as _;
    use zax_nes::dma_census::DmaEventKind;

    let mut text = String::new();
    let _ = writeln!(
        text,
        "    the DMA census of this frame: {} event(s)\n      \
         cyc        kind     instr  SL:DOT     addr val  ready  cost  dmc(timer,bits,bytes)",
        lines.len()
    );
    for line in lines {
        let ready = match line.ready {
            Some(cycle) => format!("{cycle:>6}"),
            None if line.event.kind == DmaEventKind::ApuWrite && line.event.ready_cycle != 0 => {
                format!("d={:<4}", line.event.ready_cycle)
            }
            None => "     -".to_owned(),
        };
        let cost = match line.cost {
            Some(cost) => format!("{cost:>4}"),
            None => "   -".to_owned(),
        };
        // The gap is printed on a fetch alone: it is there to be compared with BizHawk's own
        // opcode-to-opcode gap for the same instruction, and only a fetch carries the cost that
        // comparison subtracts.
        let gap = match (line.event.kind, line.gap) {
            (DmaEventKind::DmcFetch, Some((pc, gap))) => format!("  pc${pc:04X} gap={gap}"),
            _ => String::new(),
        };
        let _ = writeln!(
            text,
            "      {:<9} {} {:>6} {:>3}:{:<4} ${:04X} {:02X}  {ready} {cost}  ({},{},{}){}{gap}",
            line.cycle,
            line.event.kind.tag(),
            line.index,
            line.scanline,
            line.dot,
            line.event.address,
            line.event.value,
            line.event.dmc_timer,
            line.event.dmc_bits,
            line.event.dmc_bytes,
            if line.event.from_write { "  write" } else { "" }
        );
    }

    let fetches: Vec<&CensusLine> = lines
        .iter()
        .filter(|line| line.event.kind == DmaEventKind::DmcFetch)
        .collect();
    let charged: i64 = fetches.iter().filter_map(|line| line.cost).sum();
    let from_write = fetches.iter().filter(|line| line.event.from_write).count();
    let mut histogram = [0_usize; 8];
    for cost in fetches.iter().filter_map(|line| line.cost) {
        if let Ok(slot) = usize::try_from(cost) {
            if let Some(count) = histogram.get_mut(slot) {
                *count += 1;
            }
        }
    }
    let _ = writeln!(
        text,
        "      {} DMC fetch(es) ({from_write} started by a $4015 write) costing {charged} CPU \
         cycle(s) in all",
        fetches.len()
    );
    let spread: Vec<String> = histogram
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(cost, count)| format!("{count}x{cost}"))
        .collect();
    let _ = writeln!(text, "      the costs: {}", spread.join(" "));
    text
}

/// What the end of a frame records: the checksum, the cycles, the DMA counts and, when the
/// instrument is compiled in and asked for, the census.
///
/// One function because the frame can end two ways — at its own boundary, or by overrunning
/// [`MAX_INSTRUCTIONS`] — and both must record the same things.
///
/// `first` is the bus cycle, the DMC fetch count and the OAM transfer count the frame started at.
fn close_frame<P: TimingProfile>(
    out: &mut FrameTrace,
    bus: &zax_nes::NesBusOf<P>,
    first: (i64, i64, i64),
    census: bool,
) {
    out.checksum = checksum(bus.ram());
    out.cycles = bus.cycle_count() - first.0;
    out.dmc_fetches = bus.dmc_dma_count() - first.1;
    out.oam_transfers = bus.oam_dma_count() - first.2;
    // Taken here and not after `--converge`'s extra instructions: the census is of the frame, and
    // those instructions belong to the next one.
    #[cfg(feature = "dma-census")]
    if census {
        out.census = census_lines(bus.dma_census(), &out.steps, first.0);
        out.dmc_cost = out.census.iter().filter_map(|line| line.cost).sum();
        // The ledger's own events are in cycle order and so are the census's, so a *stable* sort
        // on the cycle merges the two without reordering either.
        out.events.extend(ledger_events(&out.census));
        out.events.sort_by_key(|event| event.offset);
    }
    #[cfg(not(feature = "dma-census"))]
    let _ = census;
}

/// The census events the cycle ledger carries, which are exactly the ones a BizHawk Lua script
/// can also see: a register access and an OAM request.
///
/// The DMC's own request, halt and fetch are **left out on purpose**. There are two hundred of
/// them in a *Kirby* frame and no documented Lua hook reports them, so a ledger line for one could
/// never be compared with anything; the frame's header carries their count and their total cost
/// instead, which is the comparable quantity (a frame is a fixed 29 780 cycles, so two machines
/// that get different amounts of work done differ by what the DMA charged them).
#[cfg(feature = "dma-census")]
fn ledger_events(lines: &[CensusLine]) -> Vec<Event> {
    use zax_nes::dma_census::DmaEventKind;

    lines
        .iter()
        .filter_map(|line| {
            let tag = match line.event.kind {
                DmaEventKind::ApuWrite | DmaEventKind::PortStrobe => {
                    format!("w${:04X}", line.event.address)
                }
                DmaEventKind::ApuStatusRead => "r$4015".to_owned(),
                DmaEventKind::PpuStatusRead => format!("r${:04X}", line.event.address),
                DmaEventKind::OamRequest => "oam".to_owned(),
                DmaEventKind::DmcRequest | DmaEventKind::DmcHalt | DmaEventKind::DmcFetch => {
                    return None
                }
                // Left out of the **ledger** on purpose, though it is a register access a Lua
                // script can see: `bk2cyc1.lua` wrote the references this side is held against
                // and it logs `$4016` writes alone, so a ledger line for a port *read* would
                // read as an event the other machine does not have on every frame of every
                // window. It is in the `--dmc-census` section, which is where lot
                // NESHAWK-PROFILE-13 compares it against `q192.lua`'s own stream.
                DmaEventKind::InputPortRead => return None,
            };
            Some(Event {
                offset: line.cycle,
                scanline: line.scanline,
                dot: line.dot,
                tag,
                detail: format!("={:02X} i={}", line.event.value, line.index),
            })
        })
        .collect()
}

/// Places every core event against the instruction it happened inside, and gives each `DmcFetch`
/// the cost of the halt that started it.
///
/// The instruction an event belongs to is the last one that began on or before its cycle, found by
/// [`slice::partition_point`] over the frame's own steps rather than by a scan: a frame has nine
/// thousand instructions and a hundred events.
#[cfg(feature = "dma-census")]
fn census_lines(
    events: &[zax_nes::dma_census::DmaEvent],
    steps: &[Step],
    first_cycle: i64,
) -> Vec<CensusLine> {
    use zax_nes::dma_census::DmaEventKind;

    let mut lines: Vec<CensusLine> = Vec::with_capacity(events.len());
    // The halt of the fetch now in flight, so a fetch can be charged for it.
    let mut halt: Option<i64> = None;
    for event in events {
        let at = steps.partition_point(|step| step.cycle <= event.cycle);
        let inside = steps.get(at.saturating_sub(1));
        let (index, scanline, dot) =
            inside.map_or((0, -1, -1), |step| (step.index, step.scanline, step.dot));
        // The instruction's PC is the first field of `nestest.log`'s own shape, which is what
        // `Step::line` carries.
        let gap = inside.zip(steps.get(at)).and_then(|(step, next)| {
            let pc = u16::from_str_radix(step.line.get(..4)?, 16).ok()?;
            Some((pc, next.cycle - step.cycle))
        });
        let cost = match event.kind {
            DmaEventKind::DmcHalt => {
                halt = Some(event.cycle);
                None
            }
            DmaEventKind::DmcFetch => halt.take().map(|from| event.cycle - from + 1),
            _ => None,
        };
        lines.push(CensusLine {
            event: *event,
            cycle: event.cycle - first_cycle,
            // Only a request and a halt carry an absolute ready cycle; on an `ApuWrite` the same
            // field is the `$4015` load delay the write returned, which the printer reads raw.
            ready: matches!(event.kind, DmaEventKind::DmcRequest | DmaEventKind::DmcHalt)
                .then(|| event.ready_cycle - first_cycle),
            index,
            scanline,
            dot,
            cost,
            gap,
        });
    }
    lines
}

/// What `--dmc-census` means in this build.
///
/// A build without the `dma-census` feature must **say so** rather than print an empty section: a
/// silent nothing would read as "this frame has no DMA", which is the opposite of what it means.
///
/// # Errors
/// Never in this configuration; the `Result` is the other one's, which always errors.
#[cfg(feature = "dma-census")]
#[allow(clippy::unnecessary_wraps)] // The signature belongs to the configuration below.
const fn census_asked() -> Result<bool, HarnessError> {
    Ok(true)
}

/// As above, for a build the instrument is not in.
///
/// # Errors
/// Always.
#[cfg(not(feature = "dma-census"))]
fn census_asked() -> Result<bool, HarnessError> {
    Err(HarnessError::Usage(
        "--dmc-census needs this binary built with the `dma-census` feature: \
         cargo build -p zax-harness --release --features dma-census"
            .to_owned(),
    ))
}

/// The subcommand.
///
/// # Errors
/// As [`run`].
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let options = parse(args)?;
    print!("{}", run(&options)?.text(&options));
    Ok(())
}

fn parse(args: &[String]) -> Result<Options, HarnessError> {
    let mut options = Options {
        frames: 1,
        context: 12,
        extra: 256,
        ..Options::default()
    };
    let mut positional = 0;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].clone();
        index += 1;
        let value = |index: &mut usize| -> String {
            let text = args.get(*index).cloned().unwrap_or_default();
            *index += 1;
            text
        };
        match argument.as_str() {
            "--frame" => options.frame = number(&value(&mut index), "--frame")?,
            "--frames" => options.frames = number(&value(&mut index), "--frames")?,
            "--context" => {
                options.context =
                    usize::try_from(number(&value(&mut index), "--context")?).unwrap_or(0);
            }
            "--watch" => options.watch.push(address(&value(&mut index), "--watch")?),
            "--out" => options.out = Some(PathBuf::from(value(&mut index))),
            "--event-log" => options.event_log = Some(PathBuf::from(value(&mut index))),
            "--event-pc" => options
                .event_pc
                .push(address(&value(&mut index), "--event-pc")?),
            "--converge" => options.converge = Some(PathBuf::from(value(&mut index))),
            "--converge-window" => {
                let text = value(&mut index);
                options.converge_window = text.parse().map_err(|_| {
                    HarnessError::Usage(format!(
                        "--converge-window {text:?} is not a whole number
{USAGE}"
                    ))
                })?;
            }
            "--builtin-patches" => options.builtin_patches = true,
            "--dmc-census" => options.dmc_census = census_asked()?,
            "--profile" => {
                options.profile = Some(ProfileChoice::parse(&value(&mut index))?);
            }
            "--extra" => {
                options.extra =
                    usize::try_from(number(&value(&mut index), "--extra")?).unwrap_or(0);
            }
            "--ram" => {
                let text = value(&mut index);
                options.ram = Some(RamPattern::parse(&text).ok_or_else(|| {
                    HarnessError::Usage(format!("--ram {text:?} is not a pattern\n{USAGE}"))
                })?);
            }
            "--resets" => {
                let text = value(&mut index);
                options.resets = Resets::parse(&text).ok_or_else(|| {
                    HarnessError::Usage(format!(
                        "--resets {text:?} is not on, off, early or late\n{USAGE}"
                    ))
                })?;
            }
            other if other.starts_with("--") => {
                return Err(HarnessError::Usage(format!(
                    "unknown option {other:?}\n{USAGE}"
                )))
            }
            other => {
                match positional {
                    0 => options.movie = PathBuf::from(other),
                    1 => options.rom = PathBuf::from(other),
                    _ => {
                        return Err(HarnessError::Usage(format!(
                            "unexpected argument {other:?}\n{USAGE}"
                        )))
                    }
                }
                positional += 1;
            }
        }
    }
    if positional < 2 || options.frame == 0 {
        return Err(HarnessError::Usage(format!(
            "frame-trace needs a movie, a ROM and --frame\n{USAGE}"
        )));
    }
    // The ledger mode captures no instruction, so neither the trace file nor a watched
    // address's context can be produced from it. Refusing the combination is the honest answer:
    // an empty `--out` file or a change with no context around it would read as a result.
    if options.event_log.is_some() && (options.out.is_some() || !options.watch.is_empty()) {
        return Err(HarnessError::Usage(format!(
            "--event-log captures no instruction, so --out and --watch cannot be served from it; \
             run them as a second command\n{USAGE}"
        )));
    }
    if options.event_log.is_none() && !options.event_pc.is_empty() {
        return Err(HarnessError::Usage(format!(
            "--event-pc is recorded by --event-log, which is not asked for\n{USAGE}"
        )));
    }
    Ok(options)
}

fn number(text: &str, flag: &str) -> Result<u32, HarnessError> {
    text.parse()
        .map_err(|_| HarnessError::Usage(format!("{flag} {text:?} is not a number\n{USAGE}")))
}

/// A hex address, for the two options that take one (`--watch` and `--event-pc`).
fn address(text: &str, flag: &str) -> Result<u16, HarnessError> {
    u16::from_str_radix(text.trim_start_matches("0x"), 16)
        .map_err(|_| HarnessError::Usage(format!("{flag} {text:?} is not a hex address\n{USAGE}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_is_read_and_a_bad_one_refused() {
        let options = parse(&[
            "a.bk2".to_owned(),
            "b.nes".to_owned(),
            "--frame".to_owned(),
            "246".to_owned(),
            "--watch".to_owned(),
            "004B".to_owned(),
            "--watch".to_owned(),
            "0x07FF".to_owned(),
            "--frames".to_owned(),
            "2".to_owned(),
            "--resets".to_owned(),
            "early".to_owned(),
        ])
        .unwrap_or_default();
        assert_eq!(options.frame, 246);
        assert_eq!(options.frames, 2);
        assert_eq!(options.watch, [0x004B, 0x07FF]);
        assert_eq!(options.resets, Resets::Early);
        // The defaults are the ones the summary depends on.
        let plain = parse(&[
            "a.bk2".to_owned(),
            "b.nes".to_owned(),
            "--frame".to_owned(),
            "1".to_owned(),
        ])
        .unwrap_or_default();
        assert_eq!((plain.frames, plain.context), (1, 12));

        for bad in [
            vec!["a.bk2".to_owned()],
            vec!["a.bk2".to_owned(), "b.nes".to_owned()],
            vec![
                "a.bk2".to_owned(),
                "b.nes".to_owned(),
                "--frame".to_owned(),
                "0".to_owned(),
            ],
            vec![
                "a.bk2".to_owned(),
                "b.nes".to_owned(),
                "--frame".to_owned(),
                "1".to_owned(),
                "--watch".to_owned(),
                "nonsense".to_owned(),
            ],
            vec![
                "a.bk2".to_owned(),
                "b.nes".to_owned(),
                "--frame".to_owned(),
                "1".to_owned(),
                "--nonsense".to_owned(),
            ],
        ] {
            assert!(parse(&bad).is_err(), "{bad:?} should be refused");
        }
    }

    /// The summary says whether the traced frame is the frame the movie plays, and says it
    /// loudly when it is not: a tracer whose own frame differs is worthless and must not be
    /// quietly reported as a result.
    #[test]
    fn the_summary_names_the_control_and_whether_it_agrees() {
        let options = Options {
            watch: vec![0x004B],
            context: 2,
            ..Options::default()
        };
        let step = |index: usize| Step {
            index,
            scanline: 10,
            dot: 20,
            cycle: i64::try_from(index).unwrap_or(0),
            line: format!("C{index:03X}  A9 01     LDA #$01"),
            interrupt: None,
        };
        let mut report = Report {
            control: 0x1234_5678,
            frames: vec![FrameTrace {
                frame: 246,
                steps: (0..5).map(step).collect(),
                // Not `steps.len()`: the ledger mode captures no step and still counts them,
                // which is why the report reads this field and not the vector.
                instructions: 5,
                changes: vec![Change {
                    address: 0x004B,
                    index: 3,
                    was: 0,
                    now: 1,
                }],
                checksum: 0x1234_5678,
                cycles: 29_780,
                ..FrameTrace::default()
            }],
            notes: vec!["a note".to_owned()],
        };
        let text = report.text(&options);
        assert!(text.contains("note: a note"), "{text}");
        assert!(
            text.contains("IDENTICAL, so tracing changed nothing"),
            "{text}"
        );
        assert!(text.contains("frame 246: 5 instructions"), "{text}");
        assert!(text.contains("$004B: 00 -> 01 at instruction 3"), "{text}");
        // The context is the instructions up to and including the one that wrote.
        assert!(text.contains("> ") && text.contains("C003"), "{text}");

        report.frames[0].checksum = 0;
        assert!(
            report.text(&options).contains("THEY DIFFER"),
            "a perturbed frame has to say so"
        );

        // And the trace file carries every instruction plus the change.
        let file = report.trace_text();
        assert_eq!(file.lines().filter(|line| line.contains("LDA")).count(), 5);
        assert!(file.contains("=== frame 246"), "{file}");
        assert!(file.contains("--- $004B: 00 -> 01"), "{file}");
    }

    /// The cycle ledger's own two halves: the file format the BizHawk script has to match, and
    /// the two option combinations it must refuse.
    ///
    /// The format is a contract between two machines, so a change to it is a change to the Lua
    /// script in `docs/review/` as well; pinning it here is what makes that visible.
    #[test]
    fn the_cycle_ledger_writes_the_format_the_other_machine_is_read_in() {
        let report = Report {
            frames: vec![FrameTrace {
                frame: 1100,
                start_cycle: 32_728_781,
                cycles: 29_779,
                instructions: 9335,
                dmc_fetches: 3,
                dmc_cost: 11,
                oam_transfers: 1,
                events: vec![
                    Event {
                        offset: 7,
                        scanline: 241,
                        dot: 27,
                        tag: "nmi".to_owned(),
                        detail: "A:00 X:00 Y:00 P:27 SP:F9 i=1".to_owned(),
                    },
                    Event {
                        offset: 1715,
                        scanline: 256,
                        dot: 36,
                        tag: "w$4017".to_owned(),
                        detail: "=C0 i=393".to_owned(),
                    },
                ],
                ..FrameTrace::default()
            }],
            ..Report::default()
        };
        let text = report.ledger_text();
        assert_eq!(text.lines().count(), 3, "{text}");
        assert_eq!(
            text.lines().next(),
            Some("F 1100 start=32728781 len=29779 instr=9335 dmc=3/11 oam=1"),
            "{text}"
        );
        // The frame number is repeated on every event line, so that `grep` on one frame of a
        // ledger of nine thousand gives something readable on its own.
        assert!(
            text.contains("  1100      7 nmi      241:27   A:00 X:00 Y:00 P:27 SP:F9 i=1"),
            "{text}"
        );
        assert!(
            text.contains("  1100   1715 w$4017   256:36   =C0 i=393"),
            "{text}"
        );

        let base = || {
            vec![
                "a.bk2".to_owned(),
                "b.nes".to_owned(),
                "--frame".to_owned(),
                "1100".to_owned(),
            ]
        };
        let mut good = base();
        good.extend(["--event-log".to_owned(), "l.txt".to_owned()]);
        good.extend(["--event-pc".to_owned(), "C06E".to_owned()]);
        let options = parse(&good).unwrap_or_default();
        assert_eq!(options.event_pc, [0xC06E]);
        assert!(options.event_log.is_some());

        for (bad, why) in [
            (
                vec!["--event-pc".to_owned(), "C06E".to_owned()],
                "no --event-log",
            ),
            (
                vec![
                    "--event-log".to_owned(),
                    "l.txt".to_owned(),
                    "--out".to_owned(),
                    "t.txt".to_owned(),
                ],
                "--out captures instructions the ledger has none of",
            ),
            (
                vec![
                    "--event-log".to_owned(),
                    "l.txt".to_owned(),
                    "--watch".to_owned(),
                    "0039".to_owned(),
                ],
                "--watch needs the context the ledger has none of",
            ),
            (
                vec![
                    "--event-log".to_owned(),
                    "l.txt".to_owned(),
                    "--event-pc".to_owned(),
                    "zz".to_owned(),
                ],
                "not a hex address",
            ),
        ] {
            let mut args = base();
            args.extend(bad);
            assert!(parse(&args).is_err(), "should be refused: {why}");
        }
    }

    /// `--converge-window`'s two halves: which addresses are *live*, and the line that says on
    /// which side of the frame boundary an instant sits.
    ///
    /// EVAL-SYNC-4 step 3: a count says how far two machines are apart and never in which
    /// direction, and the direction is the whole question when three bytes could as easily be
    /// writes our own frame had not reached yet.
    #[test]
    fn the_converge_window_names_the_live_addresses_and_which_side_of_the_boundary() {
        let ours = {
            let mut ram = vec![0u8; 2048];
            ram[0x0005] = 1; // below $0020: an artefact, never named
            ram[0x0038] = 1; // live
            ram[0x01F0] = 1; // the stack page: an artefact, never named
            ram[0x0493] = 1; // live
            ram
        };
        let theirs = vec![0u8; 2048];
        assert_eq!(diff_counts(&ours, &theirs), (4, 2));
        assert_eq!(live_addresses(&ours, &theirs), [0x0038, 0x0493]);
        assert_eq!(live_addresses(&theirs, &theirs), [] as [u16; 0]);

        let found = Converge {
            compared: 3,
            boundary: 11,
            at_boundary: 4,
            live_at_boundary: 2,
            window_half: 1,
            window: vec![
                (10, 4, 2, vec![0x0038, 0x0493]),
                (11, 4, 2, vec![0x0038, 0x0493]),
                (12, 2, 1, vec![0x0038]),
            ],
            best: (2, 12),
            best_live: (1, 12),
            ..Converge::default()
        };
        let report = found.report();
        assert!(
            report.contains("< 10 (-1): 4 byte(s), 2 live $0038 $0493"),
            "{report}"
        );
        assert!(
            report.contains("= 11 (+0): 4 byte(s), 2 live $0038 $0493"),
            "{report}"
        );
        assert!(
            report.contains("> 12 (+1): 2 byte(s), 1 live $0038"),
            "{report}"
        );
        // With no window asked for, the report does not grow the block at all.
        let plain = Converge {
            window: Vec::new(),
            window_half: 0,
            ..found
        };
        assert!(
            !plain.report().contains("instruction by instruction"),
            "{}",
            plain.report()
        );
    }
}
