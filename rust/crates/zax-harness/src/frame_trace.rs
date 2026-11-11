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
use crate::movie_sync::{read_movie, relocate_resets, Machine, Resets, ZaxnesMachine};
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
                           the NesHawk-compatible chip for a `.bk2` whose Core is NesHawk";

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
    /// Every instruction of the frame.
    pub steps: Vec<Step>,
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
                frame.steps.len(),
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
    machine.power(ram).map_err(HarnessError::Rom)?;
    // Everything before the first traced frame is the ordinary path, so the state the trace
    // starts from is the state the movie puts the machine in and nothing else.
    for frame in 1..options.frame {
        let input = movie.frames[usize::try_from(frame).unwrap_or(1) - 1];
        play(&mut machine, input)?;
    }

    // The control: the first traced frame, run the ordinary way, from this exact state. It is
    // taken first and undone by `restore`, so the trace itself starts from the same bytes.
    let input = movie.frames[usize::try_from(options.frame).unwrap_or(1) - 1];
    machine.capture().map_err(HarnessError::Rom)?;
    play(&mut machine, input)?;
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
        );
        traced.frame = frame;
        report.frames.push(traced);
        if probing.is_some() {
            machine.restore().map_err(HarnessError::Rom)?;
            machine.run(input.word).map_err(HarnessError::Rom)?;
        }
    }

    if let Some(path) = &options.out {
        fs::write(path, report.trace_text())?;
    }
    Ok(report)
}

/// One frame of the ordinary path, command and all.
fn play<P: TimingProfile>(
    machine: &mut ZaxnesMachine<P>,
    input: crate::movie_file::FrameInput,
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
    machine.run(input.word).map_err(HarnessError::Rom)?;
    Ok(())
}

/// Drives one frame instruction by instruction, which is `NesCore::run_frame`'s own loop.
///
/// `probe` is the reference's own 2 KiB for this frame and how many instructions to carry on
/// past the frame boundary; see [`Converge`].
fn trace_frame<P: TimingProfile>(
    core: &mut NesCoreOf<P>,
    word: u16,
    watch: &[u16],
    probe: Option<(&[u8], usize)>,
    window_half: usize,
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
    let first_cycle = bus.cycle_count();
    let (first_dmc, first_oam) = (bus.dmc_dma_count(), bus.oam_dma_count());

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
        let captured = CpuTracer::capture(cpu, bus);
        let interrupt = match captured.pc {
            // The handler's first instruction: the vector was just taken. Reported rather than
            // inferred from the stack, because the two vectors are what the ROM itself names.
            pc if pc == nmi_vector => Some("NMI"),
            pc if pc == irq_vector && irq_vector != nmi_vector => Some("IRQ"),
            _ => None,
        };
        if boundary.is_none() {
            out.steps.push(Step {
                index,
                scanline: bus.ppu().scanline(),
                dot: bus.ppu().dot(),
                line: captured.format(),
                interrupt,
            });
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
            out.checksum = checksum(bus.ram());
            out.cycles = bus.cycle_count() - first_cycle;
            out.dmc_fetches = bus.dmc_dma_count() - first_dmc;
            out.oam_transfers = bus.oam_dma_count() - first_oam;
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
            (Some(_), None) => return out,
            // A probe: carry on for the instructions it asked for.
            (Some(at), Some((_, extra, _))) if index >= at + *extra => break,
            _ => {}
        }
    }
    if boundary.is_none() {
        out.overran = true;
        out.checksum = checksum(bus.ram());
        out.cycles = bus.cycle_count() - first_cycle;
        out.dmc_fetches = bus.dmc_dma_count() - first_dmc;
        out.oam_transfers = bus.oam_dma_count() - first_oam;
    }
    out.converge = converge.map(|(_, _, mut found)| {
        found.extra = found.compared.saturating_sub(found.boundary + 1);
        found
    });
    out
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
            "--watch" => {
                let text = value(&mut index);
                let address =
                    u16::from_str_radix(text.trim_start_matches("0x"), 16).map_err(|_| {
                        HarnessError::Usage(format!(
                            "--watch {text:?} is not a hex address\n{USAGE}"
                        ))
                    })?;
                options.watch.push(address);
            }
            "--out" => options.out = Some(PathBuf::from(value(&mut index))),
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
    Ok(options)
}

fn number(text: &str, flag: &str) -> Result<u32, HarnessError> {
    text.parse()
        .map_err(|_| HarnessError::Usage(format!("{flag} {text:?} is not a number\n{USAGE}")))
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
            line: format!("C{index:03X}  A9 01     LDA #$01"),
            interrupt: None,
        };
        let mut report = Report {
            control: 0x1234_5678,
            frames: vec![FrameTrace {
                frame: 246,
                steps: (0..5).map(step).collect(),
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
