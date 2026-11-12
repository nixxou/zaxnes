//! `movie-sync`: **where and why Zaxnes's own engine parts from a TAS movie**, and whether the
//! movie plays at all.
//!
//! A TAS movie is the hardest test an emulator gets: tens of thousands of frames of input that
//! only produce the run they were recorded from if every frame of emulation agrees with the
//! machine that recorded them. `.fm2` movies sync on FCEUX because FCEUX recorded them
//! (lot FCEUX-48); on Zaxnes they drift, and this command exists to say **where the drift starts
//! and what it is made of** — every such point being either an emulation error worth fixing or a
//! difference worth writing down.
//!
//! # The three things compared, and why those three
//!
//! With `--engine both` the two machines run **in lockstep**: one frame on each, the same input
//! word, then a comparison of
//!
//! * the **2 KiB of work RAM**, which is where a game keeps everything it has decided. A game's
//!   own variables part before the picture does — a position, a timer, a random seed — so RAM is
//!   the early warning and the picture is the confirmation;
//! * the **picture**, one NES colour index per pixel, which is what a player would see;
//! * whether the frame was **input-sensitive** (`--lag`), which is this instrument's operational
//!   stand-in for the lag/no-lag state of the frame. See [`probe_input_sensitivity`]: neither
//!   engine exposes a lag flag through its API, so the frame is *measured* rather than asked
//!   about, and it is measured the same way on both sides — which is the property that matters
//!   for a comparison.
//!
//! **Frame 0 is compared too**, and it is the first thing compared: the machine at power-on,
//! before an input word has been fed. There is no picture yet, so only the work RAM is looked at
//! — and a movie whose two machines start from a different 2 KiB has already lost, whatever the
//! emulation does afterwards.
//!
//! The first frame at which any of the three differs is the answer. The run then **keeps going**,
//! because "do they come back together" is a different question from "when did they part" and the
//! answer decides whether a difference is a timing wobble or a fork in the game's state.
//!
//! # What the movie's header decides, on both sides
//!
//! [`crate::movie_file`] reads the region, the power-on work RAM and the per-frame commands, and
//! `movie-sync` applies all three to **both** machines. That is not a nicety: lot EVAL-SYNC's own
//! reading found that an `.fm2` with no `RAMInitOption` asks for FCEUX's
//! `00 00 00 00 FF FF FF FF` pattern while `zax-nes` powers on all-zero, so a comparison that
//! skipped it would "find" a divergence at frame 0 on every real TAS and learn nothing.
//!
//! # One engine at a time
//!
//! `--engine zaxnes` or `--engine fceux` runs one machine and reports what can be told from it
//! alone: the longest stretch of frames whose picture never changes (a hang, a game-over screen,
//! a static title), the input-sensitive frame count, and where the picture last changed. None of
//! that is proof that a movie "worked" — only a human can say whether the run reached its ending
//! — and the report says so rather than implying otherwise.
//!
//! `--engine fceux --movie-mode` hands the file to **FCEUX's own movie mode**, which is what the
//! application does (lot FCEUX-48 step 3). That is the authority on "does this movie play on
//! FCEUX"; the lockstep run feeds FCEUX the inputs directly instead, because movie mode overrides
//! the pads every frame and the input-sensitivity probe could then never see anything. Whether
//! the two FCEUX paths agree is a measurement rather than an assumption, and it is the test
//! `the_two_fceux_paths_give_the_same_frames`.
//!
//! # What this command does not do
//!
//! It changes no emulation. The Zaxnes side writes the movie's power-on pattern over work RAM
//! through `NesBus::ram_mut` after the power-on that zeroed it, which is a memory write and not a
//! core behaviour, and reads everything else through accessors that already existed.

// With `fceux-core` off there is no second machine, so the lockstep half of this module has no
// caller — the comparison, its buffers and its progress line. It is still **compiled and
// clippy'd** in that build, which is the point of keeping the module out of the feature: only
// the warning that it is unreachable is silenced, and only there.
#![cfg_attr(not(feature = "fceux-core"), allow(dead_code))]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use zax_nes::input::ControllerPortType;
use zax_nes::rom::InesHeader;
use zax_nes::state::CoreState;
use zax_nes::{NesCoreOf, NesHawkCompat, NesRegion, RunFrame, Standard, TimingProfile};

use crate::golden_core::hex;
use crate::movie_file::{FrameCommand, FrameInput, MovieFile, RamPattern};
use crate::profile_choice::ProfileChoice;
use crate::ram_trace::{best_match, Comparison, RamCompare, RamTrace, Reference};
use crate::HarnessError;

pub const USAGE: &str = "zax-harness movie-sync <movie.fm2|.bk2> <rom.nes> [options]
    Plays a TAS movie and says where the two engines part, frame by frame. With `both` it runs
    them in lockstep and compares the work RAM, the picture and each frame's input sensitivity.
      --engine zaxnes|fceux|neshawk|both   which machine(s) (default: `neshawk` for a .bk2
                           whose `Core` header is NesHawk -- a movie replays exactly only on
                           the machine that recorded it -- and `both` for everything else)
                           `neshawk` is `zax-neshawk`, the Rust port of BizHawk's own core
      --frames <n>         stop after n frames (default: the whole movie)
      --from <n>           start comparing at frame n (both engines still run every frame)
      --ram default|ff|zero|random:<seed>
                           the power-on work RAM both machines get, overriding the header's
      --lag / --no-lag     probe each compared frame's input sensitivity (default: on; it costs
                           two extra frames and three state operations per frame per engine)
      --lag-from <n> --lag-to <n>   probe only inside that window
      --keep-going <n>     compare n more frames after the first divergence (default: all of them)
      --ram-bytes <n>      how many differing RAM addresses a frame's line names (default 16)
      --progress <n>       print a line every n frames (default 0, none)
      --neshawk-gamedb <dir>   BizHawk's gamedb directory (NesCarts.xml, gamedb_nes.txt),
                           which `--engine neshawk` needs to pick the board BizHawk picks;
                           defaults to $ZAXNES_BIZHAWK_GAMEDB
      --neshawk-event-log <file>   `--engine neshawk` only: the CYCLE LEDGER of every frame
                           run, in the very shape `scripts/bk2cyc*.lua` writes on BizHawk, so
                           that the two can be diffed event for event
      --neshawk-watch-pc <hex,hex,...>   the PCs the ledger prints a `pc$` line for, which is the
                           Lua scripts' `watched_pc` (e.g. C37C,C06E,80FC,804C for Kirby)
      --movie-mode         `--engine fceux` only: play through FCEUX's own movie mode
      --report <file.json> write the whole measurement as JSON
      --trace <file.txt>   write a work-RAM checksum every --trace-step frames, with the running
                           count of input-insensitive frames, in the reference traces' own shape
      --trace-step <n>     how often a trace line is written (default 100)
      --trace-ref <file>   a BizHawk reference trace to hold the run against, line for line; the
                           frame offset between the two numberings is measured, not assumed
      --ram-ref <file>     a reference trace whose lines carry `ram=<hex>`: the run's own 2 KiB
                           is held against it BYTE FOR BYTE at every frame it has, and the first
                           difference outside the stack page and $0000-$001F is named with its
                           addresses (one engine at a time)
      --ram-ref-table <file>  write EVERY --ram-ref frame's own line there, not only the first
                           twelve with a live difference: `<frame> total= live= stack= temp=`
                           and the live addresses. What a window of several hundred frames with
                           isolated one-frame differences needs
      --ram-ref-offset <n> the frame offset --ram-ref compares at (default 0, as --trace-ref
                           measures it on the same file)
      --png <dir> --png-frames a,b,c   write those frames' pictures, per engine, as PNG
      --dump <dir> --dump-frames a,b,c  write those frames' 2 KiB of work RAM, per engine, raw
      --resets on|off|early|late   what the movie's reset column does: honoured where its own
                           format puts it (default), ignored, or one frame either side of that.
                           An .fm2's commands are valid at the start of the flagged frame; a
                           .bk2's Reset happens AFTER it (both measured, see the module docs)
      --ppu-warmup         EXPERIMENT: undo the game's PPUCTRL/PPUMASK writes of the frame after
                           each reset, the ~29 658-cycle window nesdev says the hardware ignores
                           them in and `zax-nes` does not model (`--engine zaxnes` only)
      --ppu-ram default|ff|zero|random:<seed>
                           what OAM, the palette RAM and the nametable RAM hold at power-on,
                           which nesdev calls unspecified (`--engine zaxnes` only)
      --prg-ram default|ff|zero|random:<seed>
                           the power-on contents of the **cartridge's** PRG-RAM, which is a
                           different 8 KiB from --ram's 2 KiB (`--engine zaxnes` only)
      --builtin-patches    load a recognised commercial ROM with its 3D widescreen patch, as the
                           application does. OFF here: a movie comparison must be of the ROM the
                           movie was recorded on
      --warm-up zaxnes:<n> | fceux:<n>
                           blank frames on one engine before the movie starts; the experiment
                           that tells a difference of boot phase from one of emulation
      --profile standard|neshawk
                           which 2A03 Zaxnes runs. Default: the one the movie was recorded on —
                           the NesHawk-compatible chip for a `.bk2` whose Core is NesHawk, the
                           standard one for everything else (`--engine zaxnes`/`both`)";

/// Pixels in a picture (256 x 240, REQ-CORE-048).
const PICTURE_LEN: usize = 256 * 240;
/// Pixels in a picture's row.
const PICTURE_WIDTH: usize = 256;
/// Bytes of work RAM.
const RAM_LEN: usize = 2048;
/// How many consecutive identical pictures count as "the picture has stopped moving" in the
/// one-engine report. Four seconds: long enough that no animation is this still, short enough
/// that a game-over screen is caught.
const FROZEN_RUN: u32 = 240;
/// How many frames of detail after the first divergence the text report carries.
const DETAIL_FRAMES: usize = 8;

/// What the movie's reset column is allowed to do to the machine.
///
/// A diagnostic, and only a diagnostic: `On` is what the movie's own format asks for and
/// everything else exists to answer "is this divergence the reset, and is it the reset's
/// *timing*". The reset column of both formats is one frame wide and neither public description
/// says at what point of the frame the line is tapped, so a one-frame uncertainty is real and
/// worth being able to measure rather than argue about.
///
/// The three honouring values are **relative to the format's own placement**, which
/// [`MovieFile::reset_after_frame`] carries and which differs between the two formats: an
/// `.fm2`'s commands are valid at the start of the flagged frame, a `.bk2`'s `Reset` happens
/// after it. `On` is whichever of the two the file asks for, and `Early`/`Late` are the ±1 frame
/// either side of that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Resets {
    /// Honoured where the movie's own format puts them (default).
    #[default]
    On,
    /// Not replayed at all — what the movie is worth without them.
    Off,
    /// Honoured one frame earlier than the format's placement.
    Early,
    /// Honoured one frame later than the format's placement.
    Late,
}

impl Resets {
    /// The value a command line names, or `None` when it names none of the four.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "on" => Some(Self::On),
            "off" => Some(Self::Off),
            "early" => Some(Self::Early),
            "late" => Some(Self::Late),
            _ => None,
        }
    }

    /// How many frames the commands move **from the format's own placement**, and whether they
    /// happen at all.
    const fn shift(self) -> Option<i64> {
        match self {
            Self::On => Some(0),
            Self::Off => None,
            Self::Early => Some(-1),
            Self::Late => Some(1),
        }
    }
}

/// Which machine(s) to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engines {
    /// Zaxnes's own core alone.
    Zaxnes,
    /// FCEUX alone.
    Fceux,
    /// `zax-neshawk`, the Rust port of NesHawk, alone (lot NESHAWK-REWRITE).
    Neshawk,
    /// Both, in lockstep, compared every frame.
    Both,
}

/// The command line of `movie-sync`.
// A command line is a bag of flags, and six of this one's are booleans. Grouping them into a
// struct of their own would move the names one level further from the `match` that parses them
// and buy nothing.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Options {
    /// The movie file.
    pub movie: PathBuf,
    /// The ROM it is played on.
    pub rom: PathBuf,
    /// Which machine(s).
    pub engine: Engines,
    /// Whether `--engine` was given. Without it a `.bk2` recorded on NesHawk defaults to the port
    /// of that core rather than to `both`; see [`default_engine_for`].
    pub engine_was_chosen: bool,
    /// A frame cap, or the whole movie.
    pub frames: Option<u32>,
    /// The first frame compared.
    pub from: u32,
    /// The power-on work RAM, overriding the movie's.
    pub ram: Option<RamPattern>,
    /// Probe each compared frame's input sensitivity.
    pub lag: bool,
    /// The window the probe is limited to.
    pub lag_window: (u32, u32),
    /// How many frames to go on comparing after the first divergence.
    pub keep_going: Option<u32>,
    /// How many differing RAM addresses a frame's line names.
    pub ram_bytes: usize,
    /// Print a progress line every this many frames; 0 for none.
    pub progress: u32,
    /// BizHawk's `gamedb` directory, which `--engine neshawk` needs for board detection.
    pub neshawk_gamedb: Option<PathBuf>,
    /// Where `--engine neshawk` writes its cycle ledger, in the shape `bk2cyc*.lua` writes.
    pub neshawk_event_log: Option<PathBuf>,
    /// The PCs the ledger prints a `pc$` line for, as the Lua scripts' `watched_pc` does.
    pub neshawk_watch_pc: Vec<u16>,
    /// Play through FCEUX's own movie mode (`--engine fceux` only).
    pub movie_mode: bool,
    /// Where to write the JSON.
    pub report: Option<PathBuf>,
    /// Where to write the work-RAM trace. With two engines the name gains the engine's.
    pub trace: Option<PathBuf>,
    /// How often a trace line is written out.
    pub trace_step: u32,
    /// A reference trace to hold the run against.
    pub trace_ref: Option<PathBuf>,
    /// A reference trace carrying `ram=` fields, to hold the run's own 2 KiB against byte for
    /// byte.
    ///
    /// Separate from [`Self::trace_ref`] on purpose: that one is a checksum comparison done at
    /// the end, over a trace that keeps four bytes a frame, and it is what measures the offset.
    /// This one needs the whole 2 KiB **during** the run, so it is streamed, and it is what turns
    /// "the trace parts at frame *f*" into an address and two values.
    pub ram_ref: Option<PathBuf>,
    /// Where to write every [`Self::ram_ref`] frame's own line, rather than only the first few
    /// with a live difference that the printed report has room for.
    pub ram_ref_table: Option<PathBuf>,
    /// The offset [`Self::ram_ref`] compares at.
    pub ram_ref_offset: i64,
    /// What the movie's reset column is allowed to do.
    pub resets: Resets,
    /// Load a recognised commercial ROM with its 3D built-in patch, as the application does.
    ///
    /// **False by default**, unlike `NesCore`'s own power-on value: see [`ZaxnesMachine::new`].
    pub builtin_patches: bool,
    /// Imitate the PPU's write-inhibition window after a reset. See
    /// `ZaxnesMachine::apply_ppu_warmup`.
    pub ppu_warmup: bool,
    /// The power-on contents of the PPU's OAM, palette RAM and nametable RAM.
    pub ppu_ram: Option<RamPattern>,
    /// The power-on contents of the cartridge's PRG-RAM, when the command line names one.
    ///
    /// A **different** 8 KiB from [`Self::ram`]'s 2 KiB, and the distinction is not pedantry:
    /// `BizHawk`'s NES core calls the console's 2 KiB *RAM* and the cartridge's *WRAM*, and the
    /// one sync setting a `.bk2` carries about either of them —
    /// `InitialWRamStatePattern` — is named after the second. A battery game that boots
    /// differently out of its save RAM parts from its movie at frame 1 and nothing downstream
    /// can recover it.
    pub prg_ram: Option<RamPattern>,
    /// Where to write pictures.
    pub png: Option<PathBuf>,
    /// Which frames' pictures to write.
    pub png_frames: Vec<u32>,
    /// Where to write raw work-RAM dumps.
    ///
    /// A checksum says two machines differ; only the bytes say *how*. The reference traces carry
    /// a checksum and nothing else, so when a line of one does not match, the 2 KiB behind it is
    /// the only thing that can settle whether the run is out of step or the checksum is being
    /// taken over something else.
    pub dump: Option<PathBuf>,
    /// Which frames' work RAM to write.
    pub dump_frames: Vec<u32>,
    /// Blank frames run on (Zaxnes, FCEUX) **before** the movie starts, to test whether a
    /// divergence is a difference of boot phase rather than of emulation.
    ///
    /// The two engines do not start their first frame at the same place in the PPU's scan —
    /// `zax-nes` powers on at the start of vblank (REQ-CORE-020 notes, Q-003) and FCEUX does
    /// not — so a game that waits for vblank at boot can have its whole initialisation land one
    /// frame apart. Giving the engine that is behind one blank frame of head start is the
    /// experiment that says whether that is what happened: if the two runs then agree, the
    /// divergence was the phase and not the emulation.
    pub warm_up: (u32, u32),
    /// Which 2A03 Zaxnes runs, or `None` for "the one the movie was recorded on"
    /// (`MovieFile::recorded_profile`).
    pub profile: Option<ProfileChoice>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            movie: PathBuf::new(),
            rom: PathBuf::new(),
            engine: Engines::Both,
            engine_was_chosen: false,
            neshawk_gamedb: None,
            neshawk_event_log: None,
            neshawk_watch_pc: Vec::new(),
            frames: None,
            from: 0,
            ram: None,
            lag: true,
            lag_window: (0, u32::MAX),
            keep_going: None,
            ram_bytes: 16,
            progress: 0,
            movie_mode: false,
            report: None,
            trace: None,
            trace_step: 100,
            trace_ref: None,
            ram_ref: None,
            ram_ref_table: None,
            ram_ref_offset: 0,
            resets: Resets::On,
            builtin_patches: false,
            ppu_warmup: false,
            ppu_ram: None,
            prg_ram: None,
            png: None,
            png_frames: Vec::new(),
            dump: None,
            dump_frames: Vec::new(),
            warm_up: (0, 0),
            profile: None,
        }
    }
}

/// The argument after `index`, with `index` moved past it.
fn value_at(args: &[String], index: &mut usize) -> String {
    let text = args.get(*index).cloned().unwrap_or_default();
    *index += 1;
    text
}

impl Options {
    /// Parses the arguments after the subcommand name.
    ///
    /// # Errors
    /// [`HarnessError::Usage`] for a missing or unreadable argument.
    #[allow(clippy::too_many_lines)] // One arm per option, as `app_options.rs` has.
    pub fn parse(args: &[String]) -> Result<Self, HarnessError> {
        let mut options = Self::default();
        let mut positional = 0;
        let mut index = 0;
        while index < args.len() {
            let argument = args[index].clone();
            index += 1;
            match argument.as_str() {
                "--engine" => {
                    options.engine = parse_engine(&value_at(args, &mut index))?;
                    options.engine_was_chosen = true;
                }
                "--neshawk-gamedb" => {
                    options.neshawk_gamedb = Some(PathBuf::from(value_at(args, &mut index)));
                }
                "--neshawk-event-log" => {
                    options.neshawk_event_log = Some(PathBuf::from(value_at(args, &mut index)));
                }
                "--neshawk-watch-pc" => {
                    let value = value_at(args, &mut index);
                    options.neshawk_watch_pc = value
                        .split(',')
                        .filter(|text| !text.trim().is_empty())
                        .map(|text| {
                            let text = text.trim().trim_start_matches("0x").trim_start_matches('$');
                            u16::from_str_radix(text, 16).map_err(|_| {
                                HarnessError::Usage(format!(
                                    "--neshawk-watch-pc {text:?} is not a hexadecimal address"
                                ))
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                }
                "--frames" => {
                    options.frames = Some(number(&value_at(args, &mut index), "--frames")?);
                }
                "--from" => options.from = number(&value_at(args, &mut index), "--from")?,
                "--ram" => options.ram = Some(pattern(&value_at(args, &mut index))?),
                "--lag" => options.lag = true,
                "--no-lag" => options.lag = false,
                "--lag-from" => {
                    options.lag_window.0 = number(&value_at(args, &mut index), "--lag-from")?;
                }
                "--lag-to" => {
                    options.lag_window.1 = number(&value_at(args, &mut index), "--lag-to")?;
                }
                "--keep-going" => {
                    options.keep_going = Some(number(&value_at(args, &mut index), "--keep-going")?);
                }
                "--ram-bytes" => {
                    let count = number(&value_at(args, &mut index), "--ram-bytes")?;
                    options.ram_bytes = usize::try_from(count).unwrap_or(0);
                }
                "--progress" => {
                    options.progress = number(&value_at(args, &mut index), "--progress")?;
                }
                "--movie-mode" => options.movie_mode = true,
                "--report" => options.report = Some(PathBuf::from(value_at(args, &mut index))),
                "--trace" => options.trace = Some(PathBuf::from(value_at(args, &mut index))),
                "--trace-step" => {
                    let step = number(&value_at(args, &mut index), "--trace-step")?;
                    if step == 0 {
                        return Err(HarnessError::Usage(format!(
                            "--trace-step 0 would write a line at no frame\n{USAGE}"
                        )));
                    }
                    options.trace_step = step;
                }
                "--trace-ref" => {
                    options.trace_ref = Some(PathBuf::from(value_at(args, &mut index)));
                }
                "--ram-ref" => {
                    options.ram_ref = Some(PathBuf::from(value_at(args, &mut index)));
                }
                "--ram-ref-table" => {
                    options.ram_ref_table = Some(PathBuf::from(value_at(args, &mut index)));
                }
                "--ram-ref-offset" => {
                    let text = value_at(args, &mut index);
                    options.ram_ref_offset = text.parse().map_err(|_| {
                        HarnessError::Usage(format!(
                            "--ram-ref-offset {text:?} is not a whole number\n{USAGE}"
                        ))
                    })?;
                }
                "--builtin-patches" => options.builtin_patches = true,
                "--ppu-warmup" => options.ppu_warmup = true,
                "--ppu-ram" => options.ppu_ram = Some(pattern(&value_at(args, &mut index))?),
                "--prg-ram" => options.prg_ram = Some(pattern(&value_at(args, &mut index))?),
                "--resets" => {
                    let text = value_at(args, &mut index);
                    options.resets = Resets::parse(&text).ok_or_else(|| {
                        HarnessError::Usage(format!(
                            "--resets {text:?} is not on, off, early or late\n{USAGE}"
                        ))
                    })?;
                }
                "--png" => options.png = Some(PathBuf::from(value_at(args, &mut index))),
                "--png-frames" => options.png_frames = frame_list(&value_at(args, &mut index))?,
                "--dump" => options.dump = Some(PathBuf::from(value_at(args, &mut index))),
                "--dump-frames" => options.dump_frames = frame_list(&value_at(args, &mut index))?,
                "--warm-up" => {
                    options.warm_up = parse_warm_up(&value_at(args, &mut index), options.warm_up)?;
                }
                "--profile" => {
                    options.profile = Some(ProfileChoice::parse(&value_at(args, &mut index))?);
                }
                other if other.starts_with("--") => {
                    return Err(HarnessError::Usage(format!(
                        "unknown option {other:?}\n{USAGE}"
                    )));
                }
                other => {
                    match positional {
                        0 => options.movie = PathBuf::from(other),
                        1 => options.rom = PathBuf::from(other),
                        _ => return Err(HarnessError::Usage(USAGE.to_owned())),
                    }
                    positional += 1;
                }
            }
        }
        if positional == 2 {
            Ok(options)
        } else {
            Err(HarnessError::Usage(USAGE.to_owned()))
        }
    }
}

/// The engine to run, which is `--engine` when it was given and otherwise the one the **movie**
/// asks for.
///
/// A movie only replays exactly on the machine that recorded it, and a `.bk2` says in its own
/// header which core that was. So a `.bk2` whose `Core` is `NesHawk` defaults to `neshawk`, the
/// port of that core — which is the whole reason `zax-neshawk` exists — rather than to the `both`
/// an `.fm2` or a Zaxnes movie gets. `--engine` always wins, and the choice is printed so that
/// nobody reads a `neshawk` run as a `both` run.
fn default_engine_for(options: &Options, movie: &MovieFile) -> Engines {
    if options.engine_was_chosen {
        return options.engine;
    }
    if movie.recorded_engine() == Some(zax_core_api::EngineKind::NesHawk) {
        println!(
            "  note: the movie's own header says it was recorded on BizHawk's NesHawk, so it is \
             played on `--engine neshawk`, the port of that core -- a movie replays exactly only \
             on the machine that recorded it. Pass --engine both (or zaxnes) to override."
        );
        return Engines::Neshawk;
    }
    options.engine
}

fn parse_engine(text: &str) -> Result<Engines, HarnessError> {
    match text {
        "zaxnes" => Ok(Engines::Zaxnes),
        "fceux" => Ok(Engines::Fceux),
        "neshawk" => Ok(Engines::Neshawk),
        "both" => Ok(Engines::Both),
        _ => Err(HarnessError::Usage(format!(
            "--engine {text:?} is not zaxnes, fceux, neshawk or both\n{USAGE}"
        ))),
    }
}

fn pattern(text: &str) -> Result<RamPattern, HarnessError> {
    RamPattern::parse(text)
        .ok_or_else(|| HarnessError::Usage(format!("--ram {text:?} is not a pattern\n{USAGE}")))
}

fn number(text: &str, flag: &str) -> Result<u32, HarnessError> {
    text.parse()
        .map_err(|_| HarnessError::Usage(format!("{flag} {text:?} is not a number\n{USAGE}")))
}

/// `--warm-up zaxnes:<n>` or `--warm-up fceux:<n>`, applied over the pair it already holds so
/// that both engines can be named on one command line.
fn parse_warm_up(text: &str, mut warm_up: (u32, u32)) -> Result<(u32, u32), HarnessError> {
    let (engine, count) = text.split_once(':').unwrap_or((text, ""));
    let count = number(count, "--warm-up")?;
    match engine {
        "zaxnes" => warm_up.0 = count,
        "fceux" => warm_up.1 = count,
        _ => {
            return Err(HarnessError::Usage(format!(
                "--warm-up {text:?} is not zaxnes:<n> or fceux:<n>\n{USAGE}"
            )))
        }
    }
    Ok(warm_up)
}

fn frame_list(text: &str) -> Result<Vec<u32>, HarnessError> {
    text.split(',')
        .filter(|part| !part.is_empty())
        .map(|part| number(part, "--png-frames"))
        .collect()
}

// ------------------------------------------------------------------------------------------------
// The machines
// ------------------------------------------------------------------------------------------------

/// One emulated console, driven frame by frame with an input word.
///
/// A trait rather than an enum so that the lockstep loop is written once and reads the same for
/// both sides — which is the whole point of a comparison instrument. Nothing here is a capability
/// one engine has and the other has not.
pub(crate) trait Machine {
    /// `zaxnes` or `fceux`.
    fn name(&self) -> &'static str;
    /// Power-cycles and puts `ram` into the work RAM.
    fn power(&mut self, ram: RamPattern) -> Result<(), String>;
    /// Taps the reset line, leaving the memory alone.
    fn soft_reset(&mut self) -> Result<(), String>;
    /// Runs exactly one frame with `word` on the two pads.
    fn run(&mut self, word: u16) -> Result<(), String>;
    /// Runs exactly one frame, the movie's next command being a RESET.
    ///
    /// The same frame on every machine but one: the `NesHawkCompat` profile ends it at the PPU
    /// dot the frame boundary falls on, which is where NesHawk ends a frame and so where
    /// NesHawk's reset lands (item 12). Everything else — FCEUX, and this core's own `Standard`
    /// profile — runs the frame it always did, which is why this is a defaulted method and not a
    /// capability one engine has.
    fn run_before_soft_reset(&mut self, word: u16) -> Result<(), String> {
        self.run(word)
    }
    /// Copies the 2 KiB of work RAM into `out`.
    fn ram_into(&self, out: &mut Vec<u8>) -> Result<(), String>;
    /// Copies the picture into `out`, one NES colour index a pixel.
    fn picture_into(&self, out: &mut Vec<u8>) -> Result<(), String>;
    /// Captures the machine's state, for the input-sensitivity probe.
    fn capture(&mut self) -> Result<(), String>;
    /// Puts back what [`Machine::capture`] took.
    fn restore(&mut self) -> Result<(), String>;
}

pub(crate) struct ZaxnesMachine<P: TimingProfile = Standard> {
    pub(crate) core: Box<NesCoreOf<P>>,
    region: NesRegion,
    state: CoreState,
    /// What the cartridge's PRG-RAM is filled with after every power-on, when `--prg-ram` asks.
    prg_ram: Option<RamPattern>,
    /// Undo the game's PPUCTRL/PPUMASK writes of the frame after a reset (`--ppu-warmup`).
    ppu_warmup: bool,
    /// What OAM, the palette and the nametable RAM hold after every power-on (`--ppu-ram`).
    ppu_ram: Option<RamPattern>,
    /// Frames run since the last power-on or reset.
    since_reset: u32,
}

impl<P: TimingProfile> ZaxnesMachine<P> {
    /// A machine for `image`, with the **3D built-in patches off** unless `patches` asks for them.
    ///
    /// `NesCore` powers on with `builtin_patches_enabled` **true** (`nes_core.rs:134`), which is
    /// right for the application and wrong for every measurement: `load_rom` then hands a
    /// recognised commercial game its *widescreen hack* instead of the ROM the host passed in,
    /// and the hack unlocks the 3D extension and may ask for the **overclock** — extra CPU time
    /// each frame that no NES gives. `test_roms` has always turned both off
    /// (`test_roms/mod.rs:482`); `movie-sync` did not, and lot EVAL-SYNC-3 step 2 is where that
    /// cost a measurement: *Hoshi no Kirby* has a built-in patch, and its overclock is what parts
    /// the `.bk2` from `BizHawk`. `--builtin-patches` turns them back on, which is how the
    /// difference is shown rather than asserted.
    pub(crate) fn new(
        image: &[u8],
        name: &str,
        region: NesRegion,
        patches: bool,
    ) -> Result<Self, String> {
        let mut core = Box::new(NesCoreOf::<P>::new());
        core.set_builtin_patches(patches);
        core.set_advanced_patches(false);
        if !core.load_rom(image, name).success {
            return Err("Zaxnes refused the image".to_owned());
        }
        // REQ-CORE-013: `load_rom` reports the region, it does not apply it, so the movie's own
        // goes in here and the power-on below happens at the right timing.
        core.set_region(region);
        let state = CoreState::with_capacity(core.max_core_state_size());
        Ok(Self {
            core,
            region,
            state,
            prg_ram: None,
            ppu_warmup: false,
            ppu_ram: None,
            since_reset: 0,
        })
    }

    /// The **experiment** of `--ppu-warmup`, and the one place in this lot where the instrument
    /// imitates a hardware behaviour `zax-nes` does not have.
    ///
    /// nesdev, "PPU power up state": *writes to PPUCTRL, PPUMASK, PPUSCROLL and PPUADDR are
    /// ignored for about 29 658 CPU cycles (about 30 000) after reset*. `zax-nes` does not model
    /// that window — Q-078 chose not to, for the soft reset *and* for the power-on, and says so
    /// in `Ppu2C02::soft_reset`'s own comment. 29 658 cycles is **0.996 of an NTSC frame**, so
    /// to a very good approximation the window is "the first frame after a reset", and undoing
    /// those two registers at the end of that frame leaves the PPU where ignoring every write to
    /// them would have left it.
    ///
    /// What it does **not** imitate: `$2005` and `$2006`, whose effect is in `v`, `t` and
    /// `fine_x`, and the only way to zero those from outside the core is two `$2006` writes
    /// after a *side-effecting* read of `$2002` to clear the write latch — which would also
    /// clear the vblank flag the game is about to read, and so would break more than it fixes.
    /// And the undo lands at the end of the frame rather than at each write, so an NMI the game
    /// enabled inside the window still fires once.
    fn apply_ppu_warmup(&mut self) {
        if !self.ppu_warmup || self.since_reset != 1 {
            return;
        }
        // `NesCore::write_memory` is the debug write of REQ-CORE-082 and goes through the bus's
        // real write path, so these reach the PPU registers exactly as a CPU write would.
        self.core.write_memory(0x2000, 0);
        self.core.write_memory(0x2001, 0);
    }

    /// Plugs the movie's own devices into the two controller ports.
    ///
    /// A `.bk2` states per port whether a pad is in the socket at all, and an **empty** socket is
    /// not a pad with no button held: a pad answers 1 from the ninth read after a strobe and an
    /// empty socket answers 0 for ever, so the two part exactly when a DMA adds a clock to a
    /// game's own eight reads. That is the *Hoshi no Kirby* movie's tree frame 40 (lot
    /// NESHAWK-PROFILE-13 step 1), and it is a property of the recorded **machine** rather than
    /// of the 2A03's timing, which is why it is here and not an item of `NesHawkCompat`.
    ///
    /// The devices survive a power cycle and a reset — `InputPorts::reset` keeps them, as the
    /// reference does — so this is set once, before [`Machine::power`].
    pub(crate) fn plug_ports(&mut self, ports: [ControllerPortType; 2]) {
        for (port, kind) in ports.into_iter().enumerate() {
            // The only error is a port number out of range, and the array's length is the port
            // count; `InputPorts::PORT_COUNT` is 2 and so is `MovieFile::ports`.
            let _ = self.core.set_port_type(port, kind);
        }
    }

    /// Asks for the cartridge's PRG-RAM to hold `pattern` after every power-on.
    ///
    /// `NesCore::import_battery_ram` is the public way in — a host loading a save file — and it
    /// is also exactly the right one: it writes the bytes and marks them as not to be cleared,
    /// which is what a cartridge whose battery has been in place does.
    ///
    /// # Errors
    /// The cartridge has no battery-backed PRG-RAM to fill.
    fn want_prg_ram(&mut self, pattern: RamPattern) -> Result<(), String> {
        if self.core.export_battery_ram().is_empty() {
            return Err(
                "--prg-ram: this cartridge has no battery-backed PRG-RAM to fill".to_owned(),
            );
        }
        self.prg_ram = Some(pattern);
        self.fill_prg_ram()
    }

    /// Fills the PPU's own three memories — OAM, palette RAM and the 2 KiB of nametable RAM —
    /// with `pattern`, through CPU writes to `$2003`/`$2004` and `$2006`/`$2007`.
    ///
    /// nesdev, "PPU power up state", calls OAM, the palette and VRAM *unspecified* at power-on;
    /// `zax-nes` zeroes all three (`Ppu2C02::reset`) and what `NesHawk` does is not published,
    /// so this is the third thing a movie can be held against. The writes go through
    /// `NesCore::write_memory`, which is the bus's real write path, so they reach the PPU
    /// exactly as the CPU's would.
    ///
    /// `v`, `t` and the `$2005`/`$2006` write latch are put back to 0 afterwards — the two
    /// `$2006` writes that set the fill address left `v` elsewhere, and a machine that started
    /// its first frame with a different `v` would be a second change wearing this one's name.
    fn fill_ppu_ram(&mut self, pattern: RamPattern) {
        let mut oam = [0u8; 256];
        pattern.fill(&mut oam);
        self.core.write_memory(0x2003, 0);
        for byte in oam {
            self.core.write_memory(0x2004, byte);
        }
        let mut nametables = [0u8; 2048];
        pattern.fill(&mut nametables);
        let mut palette = [0u8; 32];
        pattern.fill(&mut palette);
        for (start, bytes) in [(0x2000u16, &nametables[..]), (0x3F00, &palette[..])] {
            self.core.write_memory(0x2006, (start >> 8) as u8);
            self.core.write_memory(0x2006, (start & 0xFF) as u8);
            for byte in bytes {
                self.core.write_memory(0x2007, *byte);
            }
        }
        self.core.write_memory(0x2006, 0);
        self.core.write_memory(0x2006, 0);
    }

    fn fill_prg_ram(&mut self) -> Result<(), String> {
        let Some(pattern) = self.prg_ram else {
            return Ok(());
        };
        let mut bytes = vec![0u8; self.core.export_battery_ram().len()];
        pattern.fill(&mut bytes);
        if self.core.import_battery_ram(&bytes) {
            Ok(())
        } else {
            Err("--prg-ram: the core would not take the PRG-RAM".to_owned())
        }
    }
}

impl<P: TimingProfile> Machine for ZaxnesMachine<P> {
    /// `zaxnes` on the standard profile, `zaxnes/neshawk` on the other, so that every report line
    /// and every trace file names the chip the figures were taken on.
    fn name(&self) -> &'static str {
        if P::NAME == Standard::NAME {
            "zaxnes"
        } else {
            "zaxnes/neshawk"
        }
    }

    fn power(&mut self, ram: RamPattern) -> Result<(), String> {
        self.core.set_region(self.region);
        self.core.reset();
        // The power-on zeroed the work RAM (REQ-CORE-004); the movie's pattern goes over it.
        // This writes memory, not emulation: no core behaviour changes, and with the movie asking
        // for `zero` the bytes written are the ones already there.
        ram.fill(self.core.bus_mut().ram_mut());
        self.since_reset = 0;
        if let Some(pattern) = self.ppu_ram {
            self.fill_ppu_ram(pattern);
        }
        // And the power-on zeroed the cartridge's PRG-RAM too, which is a second 8 KiB with its
        // own power-on contents.
        self.fill_prg_ram()
    }

    fn soft_reset(&mut self) -> Result<(), String> {
        self.core.soft_reset();
        self.since_reset = 0;
        Ok(())
    }

    fn run(&mut self, word: u16) -> Result<(), String> {
        RunFrame::run_frame(&mut *self.core, word);
        self.since_reset += 1;
        self.apply_ppu_warmup();
        Ok(())
    }

    fn run_before_soft_reset(&mut self, word: u16) -> Result<(), String> {
        self.core.run_frame_before_soft_reset(word);
        self.since_reset += 1;
        self.apply_ppu_warmup();
        Ok(())
    }

    fn ram_into(&self, out: &mut Vec<u8>) -> Result<(), String> {
        out.clear();
        out.reserve(RAM_LEN);
        out.extend_from_slice(self.core.bus().ram());
        Ok(())
    }

    fn picture_into(&self, out: &mut Vec<u8>) -> Result<(), String> {
        self.core.pixel_frame_indices_into(out);
        Ok(())
    }

    fn capture(&mut self) -> Result<(), String> {
        self.core
            .capture_core_state(&mut self.state)
            .map_err(|error| error.to_string())
    }

    fn restore(&mut self) -> Result<(), String> {
        self.core
            .restore_core_state(&self.state)
            .map_err(|error| error.to_string())
    }
}

#[cfg(feature = "fceux-core")]
struct FceuxMachine {
    core: zax_fceux::Fceux,
    region: zax_fceux::Region,
    state: Vec<u8>,
}

#[cfg(feature = "fceux-core")]
impl FceuxMachine {
    fn new(image: &[u8], name: &str, region: NesRegion) -> Result<Self, String> {
        // `FceuxEngine::region_of` maps the four the same way: a dual-compatible cartridge runs
        // NTSC timing (REQ-CORE-015).
        let region = match region {
            NesRegion::Ntsc | NesRegion::Multi => zax_fceux::Region::Ntsc,
            NesRegion::Pal => zax_fceux::Region::Pal,
            NesRegion::Dendy => zax_fceux::Region::Dendy,
        };
        let mut core = zax_fceux::Fceux::new().map_err(|error| error.to_string())?;
        core.load_rom(image, name)
            .map_err(|error| error.to_string())?;
        core.set_region(region).map_err(|error| error.to_string())?;
        Ok(Self {
            core,
            region,
            state: Vec::new(),
        })
    }

    /// The `RamInit` that produces `ram`.
    fn ram_init(ram: RamPattern) -> zax_fceux::RamInit {
        match ram {
            RamPattern::FceuxDefault => zax_fceux::RamInit::Default,
            RamPattern::Ff => zax_fceux::RamInit::Ff,
            RamPattern::Zero => zax_fceux::RamInit::Zero,
            RamPattern::Random(seed) => zax_fceux::RamInit::Random(seed),
        }
    }
}

#[cfg(feature = "fceux-core")]
impl Machine for FceuxMachine {
    fn name(&self) -> &'static str {
        "fceux"
    }

    /// Power-cycles FCEUX and then **writes the pattern into its work RAM itself**.
    ///
    /// The write is not belt and braces, it is the only way to get `RamPattern::Random` at all:
    /// `PowerNES` reseeds `RAMInitSeed` from `rand()` on every power-on **unless a movie is
    /// active** (`fceu.cpp:1031-1039`), so from outside movie mode FCEUX's option 3 is a
    /// different 2 KiB every time and could never be compared with anything. Writing the bytes
    /// from [`RamPattern::fill`] on both machines also makes "the same power-on RAM" true by
    /// construction rather than by two implementations agreeing.
    ///
    /// `set_ram_init` is still called, because `RAMInitOption` fills more than the work RAM: a
    /// cartridge's **CHR-RAM** gets it too (`ines.cpp:79`), and no API call reaches that.
    fn power(&mut self, ram: RamPattern) -> Result<(), String> {
        self.core
            .set_ram_init(Self::ram_init(ram))
            .map_err(|error| error.to_string())?;
        // FCEUX keeps its region in a process global a cartridge load leaves alone (lot FCEUX-23
        // step 8), so it is re-asserted here rather than trusted.
        self.core
            .set_region(self.region)
            .map_err(|error| error.to_string())?;
        self.core.power().map_err(|error| error.to_string())?;
        let mut bytes = [0u8; RAM_LEN];
        ram.fill(&mut bytes);
        for (address, byte) in bytes.iter().enumerate() {
            let address = u16::try_from(address).unwrap_or(u16::MAX);
            self.core
                .write_memory(address, *byte)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn soft_reset(&mut self) -> Result<(), String> {
        self.core.reset().map_err(|error| error.to_string())
    }

    fn run(&mut self, word: u16) -> Result<(), String> {
        self.core
            .set_input_word(word)
            .and_then(|()| self.core.run_frame())
            .map_err(|error| error.to_string())
    }

    fn ram_into(&self, out: &mut Vec<u8>) -> Result<(), String> {
        out.clear();
        out.reserve(RAM_LEN);
        for address in 0..RAM_LEN {
            // `zax_read_memory` is FCEUX's own `GetMem`, which the fork's own comment records as
            // side-effect free, so reading the whole page changes nothing.
            let address = u16::try_from(address).unwrap_or(u16::MAX);
            let byte = self
                .core
                .read_memory(address)
                .map_err(|error| error.to_string())?;
            out.push(byte);
        }
        Ok(())
    }

    fn picture_into(&self, out: &mut Vec<u8>) -> Result<(), String> {
        self.core
            .frame_indices_into(out)
            .map_err(|error| error.to_string())
    }

    fn capture(&mut self) -> Result<(), String> {
        self.core
            .save_state_into(&mut self.state)
            .map_err(|error| error.to_string())
    }

    fn restore(&mut self) -> Result<(), String> {
        // `load_state` needs `&mut self.core` while the bytes live in `self.state`, so the block
        // is moved out and put straight back.
        let state = std::mem::take(&mut self.state);
        let result = self
            .core
            .load_state(&state)
            .map_err(|error| error.to_string());
        self.state = state;
        result
    }
}

// ------------------------------------------------------------------------------------------------
// The comparison
// ------------------------------------------------------------------------------------------------

/// What differs between the two machines at one frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameDiff {
    /// The frame, counted from 1 (the picture after that many input words).
    pub frame: u32,
    /// How many of the 2 KiB differ.
    pub ram_count: u32,
    /// How many of those are **outside page 1**, the stack — memory the game is using rather
    /// than a leftover below the stack pointer. Counted whole, unlike [`Self::ram`].
    pub ram_outside_stack: u32,
    /// The first few differing addresses, as (address, Zaxnes, FCEUX).
    pub ram: Vec<(u16, u8, u8)>,
    /// How many pixels differ.
    pub pixels: u32,
    /// The first differing pixel, as (column, row).
    pub first_pixel: Option<(u16, u16)>,
    /// Whether the frame was input-sensitive on each side, when it was probed.
    pub sensitivity: Option<(bool, bool)>,
}

impl FrameDiff {
    /// Whether the two machines agreed about everything compared.
    #[must_use]
    pub fn agrees(&self) -> bool {
        self.ram_count == 0
            && self.pixels == 0
            && self.sensitivity.is_none_or(|(ours, theirs)| ours == theirs)
    }

    /// Whether what differs would be **seen**: the picture, or the two machines disagreeing about
    /// whether the frame reacted to the pad.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.pixels > 0
            || self
                .sensitivity
                .is_some_and(|(ours, theirs)| ours != theirs)
    }

    /// Whether the difference reaches memory the game is using, rather than only the leftovers
    /// of page 1 below the stack pointer.
    #[must_use]
    pub fn is_outside_the_stack(&self) -> bool {
        self.ram_outside_stack > 0 || self.is_visible()
    }

    /// The one-line description the report prints.
    #[must_use]
    pub fn line(&self) -> String {
        let mut text = format!("frame {}:", self.frame);
        if self.ram_count == 0 {
            text.push_str(" work RAM identical;");
        } else {
            let _ = write!(
                text,
                " {} RAM byte(s) differ, {} outside the stack page (",
                self.ram_count, self.ram_outside_stack
            );
            for (index, (address, ours, theirs)) in self.ram.iter().enumerate() {
                let separator = if index == 0 { "" } else { ", " };
                let _ = write!(text, "{separator}${address:04X} {ours:02X}/{theirs:02X}");
            }
            let named = u32::try_from(self.ram.len()).unwrap_or(u32::MAX);
            if named < self.ram_count {
                let _ = write!(text, ", +{} more", self.ram_count - named);
            }
            text.push_str(");");
        }
        match (self.pixels, self.first_pixel) {
            (0, _) => text.push_str(" picture identical;"),
            (count, Some((x, y))) => {
                let _ = write!(text, " {count} pixel(s) differ, first at ({x},{y});");
            }
            (count, None) => {
                let _ = write!(text, " {count} pixel(s) differ;");
            }
        }
        text.push_str(&self.sensitivity_text());
        text
    }

    fn sensitivity_text(&self) -> String {
        match self.sensitivity {
            None => " sensitivity not probed".to_owned(),
            Some((ours, theirs)) if ours == theirs => format!(
                " both {}",
                if ours {
                    "input-sensitive"
                } else {
                    "input-insensitive (a lag frame)"
                }
            ),
            Some((ours, theirs)) => {
                format!(" sensitivity differs: zaxnes {ours}, fceux {theirs}")
            }
        }
    }

    /// The JSON row.
    fn to_json(&self) -> Value {
        json!({
            "frame": self.frame,
            "ram_bytes_differing": self.ram_count,
            "ram_bytes_differing_outside_the_stack_page": self.ram_outside_stack,
            "ram": self.ram.iter().map(|(address, ours, theirs)| json!({
                "address": address, "zaxnes": ours, "fceux": theirs,
            })).collect::<Vec<_>>(),
            "pixels_differing": self.pixels,
            "first_pixel": self.first_pixel.map(|(x, y)| json!([x, y])),
            "input_sensitive": self.sensitivity.map(|(ours, theirs)| json!({
                "zaxnes": ours, "fceux": theirs,
            })),
        })
    }
}

/// Compares one frame of the two machines. `ram_bytes` caps how many differing addresses are
/// kept; the count is always the true one.
#[must_use]
pub fn compare_frame(
    frame: u32,
    ram: (&[u8], &[u8]),
    picture: (&[u8], &[u8]),
    sensitivity: Option<(bool, bool)>,
    ram_bytes: usize,
) -> FrameDiff {
    let mut diff = FrameDiff {
        frame,
        sensitivity,
        ..FrameDiff::default()
    };
    for (address, (ours, theirs)) in ram.0.iter().zip(ram.1.iter()).enumerate() {
        if ours != theirs {
            let address = u16::try_from(address).unwrap_or(u16::MAX);
            diff.ram_count += 1;
            if !STACK_PAGE.contains(&address) {
                diff.ram_outside_stack += 1;
            }
            if diff.ram.len() < ram_bytes {
                diff.ram.push((address, *ours, *theirs));
            }
        }
    }
    for (index, (ours, theirs)) in picture.0.iter().zip(picture.1.iter()).enumerate() {
        if ours != theirs {
            diff.pixels += 1;
            if diff.first_pixel.is_none() {
                let column = u16::try_from(index % PICTURE_WIDTH).unwrap_or(u16::MAX);
                let row = u16::try_from(index / PICTURE_WIDTH).unwrap_or(u16::MAX);
                diff.first_pixel = Some((column, row));
            }
        }
    }
    diff
}

/// Whether this frame's outcome depends on what the pads said — the instrument's stand-in for
/// "the game polled the controller".
///
/// Neither engine's API exposes a lag flag: FCEUX keeps one but does not publish it, and
/// `zax-nes` does not keep one at all (and this lot may not add one). So the frame is measured
/// instead, identically on both sides: the machine runs the frame **twice** from the same state,
/// once with the movie's word and once with its complement, and the frame is *input-sensitive*
/// when the two outcomes differ. A frame the game spent without reading `$4016` cannot react to
/// either word, so it comes out insensitive; a frame in which the game read the pad and stored it
/// comes out sensitive.
///
/// Two things this is **not**, said here so that no reader has to guess:
///
/// * it is not FCEUX's `lagFlag`. A game that polls the pad and then throws the value away
///   measures insensitive here and would be "not lagged" there. The direction of the error is
///   known, and for a *comparison* — which is what this is used for — it is harmless, because
///   both machines are measured by the same rule;
/// * it does not look past the frame. A read whose effect only shows later measures insensitive.
///
/// The probe runs **before** the real frame and the real frame is run last, so what the caller is
/// left holding is the movie's own frame, picture and all. That order matters:
/// `NesCore::restore_core_state` deliberately leaves the picture out of the block, so a probe run
/// after the real one would overwrite the picture it was supposed to be checking.
fn probe_input_sensitivity(
    machine: &mut dyn Machine,
    word: u16,
    resets_next: bool,
) -> Result<bool, String> {
    machine.capture()?;
    machine.run(!word)?;
    let alternate = outcome_digest(machine)?;
    machine.restore()?;
    // The real frame is the one the run goes on from, so it is the one that ends where a frame
    // before a RESET has to end (item 12); the alternate word's frame is thrown away.
    if resets_next {
        machine.run_before_soft_reset(word)?;
    } else {
        machine.run(word)?;
    }
    let real = outcome_digest(machine)?;
    Ok(alternate != real)
}

/// The work RAM and the picture of the frame just run, hashed together.
fn outcome_digest(machine: &mut dyn Machine) -> Result<[u8; 32], String> {
    let mut buffer = Vec::with_capacity(PICTURE_LEN);
    let mut hasher = Sha256::new();
    machine.ram_into(&mut buffer)?;
    hasher.update(&buffer);
    machine.picture_into(&mut buffer)?;
    hasher.update(&buffer);
    Ok(hasher.finalize().into())
}

// ------------------------------------------------------------------------------------------------
// What one engine alone can be told
// ------------------------------------------------------------------------------------------------

/// What a single engine's run says about itself.
#[derive(Debug, Clone, Default)]
pub struct EngineRun {
    /// `zaxnes` or `fceux`, with whatever the run has to add to the name.
    pub engine: String,
    /// Frames actually run.
    pub frames: u32,
    /// The frame the run stopped at and why, when it did not finish.
    pub stopped: Option<(u32, String)>,
    /// The last frame whose picture differed from the one before it.
    pub last_picture_change: u32,
    /// The longest run of consecutive identical pictures, and the frame it starts at.
    pub longest_frozen: (u32, u32),
    /// Frames probed, and how many of those were input-sensitive.
    pub probed: (u32, u32),
    /// The longest run of consecutive input-insensitive probed frames, and where it starts.
    pub longest_insensitive: (u32, u32),
    /// SHA-256 of the last picture.
    pub last_picture: String,
}

impl EngineRun {
    /// The paragraph the report prints for one engine.
    #[must_use]
    pub fn report(&self) -> String {
        let mut text = format!("  {}: {} frames run", self.engine, self.frames);
        if let Some((frame, why)) = &self.stopped {
            let _ = write!(text, ", STOPPED at {frame}: {why}");
        }
        let _ = writeln!(
            text,
            "\n    the picture last changed at frame {}",
            self.last_picture_change
        );
        if self.longest_frozen.0 >= FROZEN_RUN {
            let _ = writeln!(
                text,
                "    the longest stretch with no change at all is {} frames from {} \
                 -- a hang, or a screen that does not move",
                self.longest_frozen.0, self.longest_frozen.1
            );
        } else {
            let _ = writeln!(
                text,
                "    no stretch of {FROZEN_RUN} frames without a change (longest {} from {})",
                self.longest_frozen.0, self.longest_frozen.1
            );
        }
        if self.probed.0 > 0 {
            let _ = writeln!(
                text,
                "    {} of {} probed frames were input-sensitive; the longest insensitive \
                 stretch is {} frames from {}",
                self.probed.1,
                self.probed.0,
                self.longest_insensitive.0,
                self.longest_insensitive.1
            );
        }
        let _ = write!(text, "    last picture sha256 {}", self.last_picture);
        text
    }

    fn to_json(&self) -> Value {
        json!({
            "engine": self.engine,
            "frames": self.frames,
            "stopped_at": self.stopped.as_ref().map(|(frame, why)| json!({
                "frame": frame, "reason": why,
            })),
            "last_picture_change": self.last_picture_change,
            "longest_frozen_run": {
                "frames": self.longest_frozen.0, "from": self.longest_frozen.1,
            },
            "frames_probed": self.probed.0,
            "frames_input_sensitive": self.probed.1,
            "longest_insensitive_run": {
                "frames": self.longest_insensitive.0, "from": self.longest_insensitive.1,
            },
            "last_picture_sha256": self.last_picture,
        })
    }
}

/// Running totals of one engine's picture and sensitivity history.
#[derive(Debug, Default)]
struct RunTracker {
    previous: Vec<u8>,
    frozen_since: u32,
    insensitive_since: Option<u32>,
    run: EngineRun,
}

impl RunTracker {
    fn new(engine: &str) -> Self {
        Self {
            run: EngineRun {
                engine: engine.to_owned(),
                ..EngineRun::default()
            },
            ..Self::default()
        }
    }

    /// Takes one frame's picture and, when it was probed, its sensitivity.
    fn frame(&mut self, frame: u32, picture: &[u8], sensitive: Option<bool>) {
        if self.previous.is_empty() || self.previous != picture {
            self.run.last_picture_change = frame;
            self.frozen_since = frame;
            self.previous.clear();
            self.previous.extend_from_slice(picture);
        } else {
            let length = frame - self.frozen_since;
            if length > self.run.longest_frozen.0 {
                self.run.longest_frozen = (length, self.frozen_since);
            }
        }
        if let Some(sensitive) = sensitive {
            self.run.probed.0 += 1;
            if sensitive {
                self.run.probed.1 += 1;
                self.insensitive_since = None;
            } else {
                let since = *self.insensitive_since.get_or_insert(frame);
                let length = frame - since + 1;
                if length > self.run.longest_insensitive.0 {
                    self.run.longest_insensitive = (length, since);
                }
            }
        }
        self.run.frames = frame;
    }

    fn finish(mut self) -> EngineRun {
        self.run.last_picture = hex(&Sha256::digest(&self.previous));
        self.run
    }
}

// ------------------------------------------------------------------------------------------------
// The whole measurement
// ------------------------------------------------------------------------------------------------

/// What the movie turned out to be.
#[derive(Debug, Clone, Default)]
pub struct MovieSummary {
    /// `fm2` or `bk2`.
    pub kind: String,
    /// Its own file name.
    pub file: String,
    /// Frames in the file.
    pub length: u32,
    /// The region it asks for.
    pub pal: bool,
    /// The power-on work RAM used.
    pub ram: String,
    /// Whether the ROM is the one the movie names; `None` when the movie names none.
    pub rom_matches: Option<bool>,
    /// The ROM played.
    pub rom: String,
    /// Its mapper.
    pub mapper: Option<i32>,
    /// The frames carrying a command, with it.
    pub commands: Vec<(usize, String)>,
    /// What the reader had to decide.
    pub notes: Vec<String>,
}

/// What a `movie-sync` run found.
#[derive(Debug, Clone, Default)]
pub struct Measurement {
    /// The movie, as the reader understood it.
    pub movie: MovieSummary,
    /// One entry per engine run.
    pub runs: Vec<EngineRun>,
    /// The first frame at which the two machines differed, with what differed.
    pub first_divergence: Option<FrameDiff>,
    /// Frames compared before the divergence, all of them identical.
    pub agreed_frames: u32,
    /// After the divergence: frames compared, frames whose picture agreed again, and the last
    /// such frame.
    pub after: (u32, u32, Option<u32>),
    /// After the divergence: frames whose work RAM agreed again.
    pub ram_agreed_after: u32,
    /// A few frames' worth of detail after the divergence, for the report.
    pub detail: Vec<FrameDiff>,
    /// The first frame at which the difference is **visible**: pixels differ, or the two machines
    /// disagree about the frame's input sensitivity.
    ///
    /// Kept beside [`Self::first_divergence`] because on a real TAS the two are rarely the same
    /// frame and the distance between them is the interesting quantity. The strict first
    /// divergence is usually a byte of work RAM; whether and when it reaches the screen is a
    /// different fact.
    pub first_visible: Option<FrameDiff>,
    /// The first frame whose work RAM differs **outside page 1** (`$0100-$01FF`), or which is
    /// visible.
    ///
    /// Page 1 is the stack, and a byte of it below the stack pointer is a leftover no game reads
    /// — but it is *not* noise: the bytes an interrupt pushes are the program counter, so a
    /// difference there says the two machines took the interrupt at different points. Both are
    /// worth having separately: this one is the first difference in memory the game is using.
    pub first_outside_stack: Option<FrameDiff>,
    /// How many compared frames are visibly different, over the whole run.
    pub visible_count: u32,
    /// The first [`VISIBLE_FRAMES_LISTED`] of them, by frame. On a movie that nearly syncs this
    /// is the whole answer — "five frames of 47 454, and these are the five".
    pub visible_at: Vec<u32>,
    /// One work-RAM trace per engine run, when `--trace` or `--trace-ref` asked for one.
    pub traces: Vec<RamTrace>,
    /// What holding each trace against `--trace-ref` found: the engine, the best offset's
    /// comparison, and the runner-up offset's.
    pub against_reference: Vec<(String, Comparison, Option<Comparison>)>,
    /// What holding each run's own 2 KiB against `--ram-ref` found, byte for byte.
    pub ram_against_reference: Vec<(String, RamCompare)>,
}

/// How many visibly differing frames the report names before it only counts them.
const VISIBLE_FRAMES_LISTED: usize = 32;

/// The page the stack lives in, whose bytes below the stack pointer are leftovers.
const STACK_PAGE: std::ops::Range<u16> = 0x0100..0x0200;

impl Measurement {
    /// Takes one compared frame. Answers `true` when the caller should stop comparing.
    fn record(&mut self, diff: FrameDiff, keep_going: Option<u32>) -> bool {
        if diff.is_visible() {
            self.visible_count += 1;
            if self.visible_at.len() < VISIBLE_FRAMES_LISTED {
                self.visible_at.push(diff.frame);
            }
            if self.first_visible.is_none() {
                self.first_visible = Some(diff.clone());
            }
        }
        if self.first_outside_stack.is_none() && diff.is_outside_the_stack() {
            self.first_outside_stack = Some(diff.clone());
        }
        if self.first_divergence.is_none() {
            if diff.agrees() {
                self.agreed_frames += 1;
            } else {
                self.first_divergence = Some(diff);
            }
            return false;
        }
        self.after.0 += 1;
        if diff.pixels == 0 {
            self.after.1 += 1;
            self.after.2 = Some(diff.frame);
        }
        if diff.ram_count == 0 {
            self.ram_agreed_after += 1;
        }
        if self.detail.len() < DETAIL_FRAMES {
            self.detail.push(diff);
        }
        keep_going.is_some_and(|limit| self.after.0 >= limit)
    }

    /// The whole text report.
    #[must_use]
    pub fn report(&self) -> String {
        let mut text = String::new();
        let summary = &self.movie;
        let _ = writeln!(
            text,
            "movie-sync: {} ({}, {} frames, {}) on {}",
            summary.file,
            summary.kind,
            summary.length,
            if summary.pal { "PAL" } else { "NTSC" },
            summary.rom
        );
        let _ = writeln!(
            text,
            "  mapper {}, the ROM is {} the movie names",
            summary
                .mapper
                .map_or_else(|| "?".to_owned(), |mapper| mapper.to_string()),
            match summary.rom_matches {
                Some(true) => "the one",
                Some(false) => "NOT the one",
                None => "unverified, the movie naming no",
            }
        );
        let _ = writeln!(text, "  power-on work RAM: {}", summary.ram);
        if summary.commands.is_empty() {
            let _ = writeln!(text, "  no frame carries a command");
        } else {
            let _ = writeln!(text, "  commands: {:?}", summary.commands);
        }
        for note in &summary.notes {
            let _ = writeln!(text, "  note: {note}");
        }
        for run in &self.runs {
            let _ = writeln!(text, "{}", run.report());
        }
        for (engine, best, runner_up) in &self.against_reference {
            let _ = writeln!(text, "  {engine} against the reference trace:");
            let _ = writeln!(text, "{}", best.report());
            if let Some(runner_up) = runner_up {
                let _ = writeln!(
                    text,
                    "    (the next best offset, {}, agrees on {} line(s), {} of them leading — \
                     the winner is a measurement only if this is far behind it)",
                    runner_up.offset, runner_up.agreeing, runner_up.matched
                );
            }
        }
        for (engine, compare) in &self.ram_against_reference {
            let _ = writeln!(text, "  {engine} against the reference's own work RAM:");
            let _ = writeln!(text, "{}", compare.report());
        }
        let _ = write!(text, "{}", self.divergence_report());
        text
    }

    fn divergence_report(&self) -> String {
        if self.runs.len() < 2 {
            return String::new();
        }
        let mut text = String::new();
        let Some(first) = &self.first_divergence else {
            let _ = writeln!(
                text,
                "  the two machines agree at every one of the {} frames compared",
                self.agreed_frames
            );
            return text;
        };
        let _ = writeln!(
            text,
            "  FIRST DIVERGENCE after {} identical frames\n    {}",
            self.agreed_frames,
            first.line()
        );
        for diff in &self.detail {
            let _ = writeln!(text, "    {}", diff.line());
        }
        for (what, which) in [
            (
                "the first difference outside the stack page (memory the game is using)",
                self.first_outside_stack.as_ref(),
            ),
            (
                "the first difference that would be SEEN (pixels, or one machine reacting to the \
                 pad and the other not)",
                self.first_visible.as_ref(),
            ),
        ] {
            match which {
                None => {
                    let _ = writeln!(text, "  {what}: none in what was compared");
                }
                Some(diff) if diff.frame == first.frame => {
                    let _ = writeln!(text, "  {what}: the same frame");
                }
                Some(diff) => {
                    let _ = writeln!(text, "  {what}:\n    {}", diff.line());
                }
            }
        }
        let listed = self
            .visible_at
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            text,
            "  {} compared frame(s) differ visibly: {listed}{}",
            self.visible_count,
            if self.visible_count as usize > self.visible_at.len() {
                ", ..."
            } else {
                ""
            }
        );
        let (compared, same_picture, last) = self.after;
        let _ = writeln!(
            text,
            "  of the {compared} frames compared after it, {same_picture} had the same picture \
             and {} the same work RAM; the last frame with the same picture is {}",
            self.ram_agreed_after,
            last.map_or_else(|| "none".to_owned(), |frame| frame.to_string())
        );
        let _ = writeln!(
            text,
            "  so the two machines {}",
            if compared == 0 {
                "were not compared past the divergence"
            } else if same_picture == compared {
                "came back together and stayed together"
            } else if same_picture > 0 {
                "come back together at times but do not stay together"
            } else {
                "stay apart"
            }
        );
        text
    }

    fn to_json(&self) -> Value {
        let mut root = Map::new();
        root.insert(
            "movie".into(),
            json!({
                "kind": self.movie.kind,
                "file": self.movie.file,
                "frames": self.movie.length,
                "region": if self.movie.pal { "PAL" } else { "NTSC" },
                "power_on_work_ram": self.movie.ram,
                "rom": self.movie.rom,
                "rom_mapper": self.movie.mapper,
                "rom_is_the_one_named": self.movie.rom_matches,
                "commands": self.movie.commands.iter()
                    .map(|(frame, what)| json!({"frame": frame, "command": what}))
                    .collect::<Vec<_>>(),
                "notes": self.movie.notes,
            }),
        );
        root.insert(
            "engines".into(),
            Value::Array(self.runs.iter().map(EngineRun::to_json).collect()),
        );
        root.insert(
            "frames_identical_before_the_first".into(),
            json!(self.agreed_frames),
        );
        root.insert(
            "frames_visibly_different".into(),
            json!({ "count": self.visible_count, "first": self.visible_at }),
        );
        for (key, which) in [
            ("first_divergence", self.first_divergence.as_ref()),
            (
                "first_divergence_outside_the_stack",
                self.first_outside_stack.as_ref(),
            ),
            ("first_visible_divergence", self.first_visible.as_ref()),
        ] {
            root.insert(key.into(), which.map_or(Value::Null, FrameDiff::to_json));
        }
        root.insert(
            "against_the_reference_trace".into(),
            Value::Array(
                self.against_reference
                    .iter()
                    .map(|(engine, best, runner_up)| {
                        json!({
                            "engine": engine,
                            "offset": best.offset,
                            "lines_compared": best.compared,
                            "lines_agreeing": best.agreeing,
                            "leading_lines_agreeing": best.matched,
                            "last_agreeing_frame": best.last_match,
                            "first_differing_line": best.first_diff.map(|(frame, ours, theirs)| {
                                json!({"frame": frame, "ours": ours, "reference": theirs})
                            }),
                            "first_differing_lag": best.first_lag_diff.map(
                                |(frame, ours, theirs)| json!({
                                    "frame": frame, "ours": ours, "reference": theirs,
                                }),
                            ),
                            "runner_up_offset": runner_up.as_ref().map(|other| json!({
                                "offset": other.offset,
                                "leading_lines_agreeing": other.matched,
                            })),
                        })
                    })
                    .collect(),
            ),
        );
        root.insert(
            "after_the_divergence".into(),
            json!({
                "frames_compared": self.after.0,
                "frames_with_the_same_picture": self.after.1,
                "frames_with_the_same_work_ram": self.ram_agreed_after,
                "last_frame_with_the_same_picture": self.after.2,
                "detail": self.detail.iter().map(FrameDiff::to_json).collect::<Vec<_>>(),
            }),
        );
        Value::Object(root)
    }
}

/// Runs the command.
///
/// # Errors
/// [`HarnessError`] when a file cannot be read, an engine refuses the ROM, or the movie is not
/// one this instrument reads.
pub fn run(options: &Options) -> Result<Measurement, HarnessError> {
    // The two experiments touch one core only — no FCEUX API call reaches a cartridge's PRG-RAM,
    // and the PPU warm-up one is about this core's own PPU — so a two-engine run with either of
    // them would compare a machine that got the experiment with one that did not, which is worse
    // than not comparing. Both are refused here, before a file is read.
    for (asked, what, why) in [
        (
            options.prg_ram.is_some(),
            "--prg-ram",
            "no FCEUX API call reaches a cartridge's PRG-RAM, so the two machines could not be \
             given the same one",
        ),
        (
            options.ppu_ram.is_some(),
            "--ppu-ram",
            "no FCEUX API call reaches the PPU's own memories, so the two machines could not be              given the same ones",
        ),
        (
            options.ppu_warmup,
            "--ppu-warmup",
            "it is an experiment on this core's own PPU and there is nothing to give the other \
             machine",
        ),
    ] {
        if asked && options.engine != Engines::Zaxnes {
            return Err(HarnessError::Usage(format!(
                "{what} works with `--engine zaxnes` only: {why}"
            )));
        }
    }
    // The byte-for-byte comparison is a property of **one** run against a third machine's bytes,
    // and the lockstep loop is a comparison of two runs against each other: there is one work-RAM
    // buffer per machine there and no sensible place to put two verdicts.
    if options.ram_ref.is_some() && options.engine == Engines::Both {
        return Err(HarnessError::Usage(
            "--ram-ref holds one run against a reference's own bytes, so it takes one engine: \
             use `--engine zaxnes` or `--engine fceux`"
                .to_owned(),
        ));
    }
    let image = fs::read(&options.rom)?;
    let movie_bytes = fs::read(&options.movie)?;
    // A header that parses *is* the "this is a cartridge" answer the `.fm2` reader needs for the
    // `FDS 1` contradiction: an iNES image is a cartridge whatever the movie's header claims.
    let header = InesHeader::try_parse(&image).ok();
    let movie = relocate_resets(
        read_movie(&options.movie, &movie_bytes, header.is_some())?,
        options.resets,
    );

    let ram = options.ram.unwrap_or(movie.ram);
    // A `.bk2` states no region, so the ROM's own header decides — `InesHeader`'s rule, which is
    // REQ-CORE-013's "report, do not apply" read from the other end.
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

    let mut measurement = Measurement {
        movie: summarise(options, &movie, &image, header.as_ref(), ram),
        ..Measurement::default()
    };
    let machines = Machines {
        image: &image,
        name: &name,
        region,
        ram,
    };
    match default_engine_for(options, &movie) {
        Engines::Both => run_lockstep(options, &movie, &machines, &mut measurement)?,
        Engines::Zaxnes => {
            let mut machine = zaxnes_machine(options, &movie, &machines)?;
            let (run, trace, bytes) = run_alone(options, &movie, machine.as_mut(), ram)?;
            let engine = run.engine.clone();
            measurement.runs.push(run);
            measurement.traces.extend(trace);
            measurement
                .ram_against_reference
                .extend(bytes.map(|bytes| (engine, bytes)));
        }
        Engines::Neshawk => {
            let mut machine = neshawk_machine(options, &movie, &machines)?;
            let (run, trace, bytes) = run_alone(options, &movie, machine.as_mut(), ram)?;
            let engine = run.engine.clone();
            measurement.runs.push(run);
            measurement.traces.extend(trace);
            measurement
                .ram_against_reference
                .extend(bytes.map(|bytes| (engine, bytes)));
        }
        Engines::Fceux => run_fceux_alone(options, &movie, &machines, &mut measurement)?,
    }
    finish_traces(options, &mut measurement)?;
    if let Some(path) = &options.ram_ref_table {
        for (_, compare) in &measurement.ram_against_reference {
            fs::write(path, compare.table_text())?;
        }
    }

    if let Some(path) = &options.report {
        fs::write(path, format!("{:#}\n", measurement.to_json()))?;
    }
    Ok(measurement)
}

/// How far either side of zero the calibration looks for the offset between the two frame
/// numberings. Three frames is far more than a convention can plausibly differ by, and keeping it
/// small is what makes a win at one offset evidence rather than a search over possibilities.
const OFFSETS_TRIED: std::ops::RangeInclusive<i64> = -3..=3;

/// Writes `--trace` and holds the traces against `--trace-ref`.
fn finish_traces(options: &Options, measurement: &mut Measurement) -> Result<(), HarnessError> {
    if let Some(path) = &options.trace {
        for trace in &measurement.traces {
            // With two engines one path cannot hold two traces, so the engine's name goes into
            // the file name rather than one run quietly overwriting the other.
            let path = if measurement.traces.len() > 1 {
                let stem = path
                    .file_stem()
                    .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
                let extension = path.extension().map_or_else(
                    || "txt".to_owned(),
                    |ext| ext.to_string_lossy().into_owned(),
                );
                path.with_file_name(format!("{stem}-{}.{extension}", trace.engine))
            } else {
                path.clone()
            };
            fs::write(
                path,
                trace.text(&measurement.movie.file, &measurement.movie.rom),
            )?;
        }
    }
    let Some(path) = &options.trace_ref else {
        return Ok(());
    };
    let text = fs::read_to_string(path)?;
    let reference = Reference::parse(&text)
        .map_err(|why| HarnessError::Format(format!("{}: {why}", path.display())))?;
    for trace in &measurement.traces {
        let (best, runner_up) = best_match(trace, &reference, OFFSETS_TRIED);
        measurement
            .against_reference
            .push((trace.engine.clone(), best, runner_up));
    }
    Ok(())
}

/// A trace of this engine's run, when one was asked for.
fn tracer(options: &Options, engine: &str) -> Option<RamTrace> {
    (options.trace.is_some() || options.trace_ref.is_some())
        .then(|| RamTrace::new(engine, options.trace_step))
}

/// What a machine needs to be built.
struct Machines<'a> {
    image: &'a [u8],
    name: &'a str,
    region: NesRegion,
    ram: RamPattern,
}

/// Puts the movie's reset commands where they are to be applied, as the format and `--resets` ask.
///
/// Two shifts, added together. Every `FrameInput::command` in a [`MovieFile`] means "applied
/// before this frame runs", which is [`step`]'s one invariant — so a format whose column means
/// *after* the flagged frame ([`MovieFile::reset_after_frame`], which a `.bk2` sets) is moved
/// `+1` here, and `--resets` then adds its own ±1 on top of that. `--resets on` is the format's
/// own placement and nothing else.
///
/// The input words are left exactly where they are: only the command column moves, so the machine
/// is still fed the movie's own inputs and the only thing under test is the reset.
pub(crate) fn relocate_resets(mut movie: MovieFile, resets: Resets) -> MovieFile {
    // What the file itself asks for: 0 when its column means "before this frame", 1 when it means
    // "after it".
    let native = i64::from(movie.reset_after_frame);
    if resets == Resets::On && native == 0 {
        return movie;
    }
    let was: Vec<FrameCommand> = movie.frames.iter().map(|frame| frame.command).collect();
    for frame in &mut movie.frames {
        frame.command = FrameCommand::None;
    }
    let Some(asked) = resets.shift() else {
        movie
            .notes
            .push("`--resets off`: the movie's reset commands are NOT replayed".to_owned());
        return movie;
    };
    let shift = native + asked;
    let mut dropped = 0u32;
    for (index, command) in was.iter().enumerate() {
        if *command == FrameCommand::None {
            continue;
        }
        let moved = usize::try_from(i64::try_from(index).unwrap_or(0) + shift);
        match moved.ok().and_then(|moved| movie.frames.get_mut(moved)) {
            Some(frame) => frame.command = *command,
            // A reset the shift pushes off either end of the movie. Off the *far* end is the
            // ordinary case — a `.bk2` whose last frame carries a Reset asks for a reset after a
            // movie that has ended — and it is counted and said rather than passed over.
            None => dropped += 1,
        }
    }
    if native != 0 {
        movie.notes.push(format!(
            "the {} column means \"after the flagged frame\", so each command is applied before \
             the frame {native} later",
            movie.kind
        ));
    }
    if asked != 0 {
        movie.notes.push(format!(
            "`--resets {}`: every reset command was moved {asked:+} frame from the placement the \
             movie's own format asks for",
            if asked < 0 { "early" } else { "late" }
        ));
    }
    if dropped > 0 {
        movie.notes.push(format!(
            "{dropped} reset command(s) fall outside the movie once moved, and are not replayed"
        ));
    }
    movie
}

/// Reads the movie by its extension.
pub(crate) fn read_movie(
    path: &Path,
    bytes: &[u8],
    rom_is_cartridge: bool,
) -> Result<MovieFile, HarnessError> {
    let extension = path
        .extension()
        .map(|text| text.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let read = match extension.as_str() {
        "fm2" => MovieFile::read_fm2(bytes, rom_is_cartridge),
        "bk2" => crate::bk2::read(bytes),
        _ => {
            return Err(HarnessError::Usage(format!(
                "{} is neither an .fm2 nor a .bk2 movie\n{USAGE}",
                path.display()
            )))
        }
    };
    read.map_err(|why| HarnessError::Format(format!("{}: {why}", path.display())))
}

fn summarise(
    options: &Options,
    movie: &MovieFile,
    image: &[u8],
    header: Option<&InesHeader>,
    ram: RamPattern,
) -> MovieSummary {
    let mut notes = movie.notes.clone();
    if movie.rom_sha1.is_some() && rom_matches(movie, image).is_none() {
        notes.push(
            "the movie states a SHA-1 that is neither this image's nor its data's after the iNES \
             header. What BizHawk takes that digest over is not in the public description of the \
             format, so this is reported as unverified rather than as the wrong ROM"
                .to_owned(),
        );
    }
    if options.ram.is_some() {
        notes.push(format!(
            "the power-on work RAM was overridden on the command line; the movie asks for {}",
            movie.ram
        ));
    }
    MovieSummary {
        kind: movie.kind.to_owned(),
        file: file_name(&options.movie),
        length: u32::try_from(movie.frames.len()).unwrap_or(u32::MAX),
        pal: movie.pal,
        ram: ram.to_string(),
        rom_matches: rom_matches(movie, image),
        rom: file_name(&options.rom),
        mapper: header.map(|header| header.mapper),
        commands: movie
            .commands()
            .into_iter()
            .map(|(frame, command)| (frame, format!("{command:?}")))
            .collect(),
        notes,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// Whether the image is the one the movie names, by whichever fingerprint the movie carries.
///
/// The `.fm2` rule is exact: the MD5 covers the data after the 16-byte iNES header. `BizHawk`'s
/// is not published — the format description says the header carries "SHA1 (also CRC32 or MD5)"
/// and does not say what it is taken over — so **both** the whole file and the data after the
/// header are tried, and a movie that matches neither is *unverified* rather than wrong.
fn rom_matches(movie: &MovieFile, image: &[u8]) -> Option<bool> {
    let data = image.get(16..)?;
    if let Some(expected) = movie.rom_md5 {
        return Some(zax_nes::movie::fm2::md5(data) == expected);
    }
    let expected = movie.rom_sha1?;
    // A match is a match; a non-match is **not** evidence of the wrong ROM, because what the
    // digest is taken over is not published. The caller says so in a note instead.
    (crate::bk2::sha1(image) == expected || crate::bk2::sha1(data) == expected).then_some(true)
}

/// The window of frames the input-sensitivity probe covers.
fn probes(options: &Options, frame: u32) -> bool {
    options.lag && frame >= options.lag_window.0 && frame <= options.lag_window.1
}

/// One frame on one machine, with the probe when this frame is probed.
///
/// `resets_next` is the next frame's command: a movie's RESET is applied before the frame it is
/// recorded on, so the frame **before** it is the one that has to end where the machine being
/// replayed ends it (item 12, [`Machine::run_before_soft_reset`]).
fn step(
    machine: &mut dyn Machine,
    input: FrameInput,
    ram: RamPattern,
    probe: bool,
    resets_next: bool,
) -> Result<Option<bool>, String> {
    match input.command {
        FrameCommand::None => {}
        FrameCommand::SoftReset => machine.soft_reset()?,
        FrameCommand::HardReset => machine.power(ram)?,
    }
    if probe {
        probe_input_sensitivity(machine, input.word, resets_next).map(Some)
    } else if resets_next {
        machine.run_before_soft_reset(input.word).map(|()| None)
    } else {
        machine.run(input.word).map(|()| None)
    }
}

/// Whether the movie taps RESET on the frame after `frame`, which is the one thing the frame
/// before a reset has to know (item 12). Out of range — the last frame — is false.
pub(crate) fn resets_next(movie: &MovieFile, frame: u32) -> bool {
    movie
        .frames
        .get(usize::try_from(frame).unwrap_or(usize::MAX))
        .is_some_and(|next| matches!(next.command, FrameCommand::SoftReset))
}

/// How many frames to run: the movie's length, capped by `--frames`.
fn frame_count(options: &Options, movie: &MovieFile) -> u32 {
    let length = u32::try_from(movie.frames.len()).unwrap_or(u32::MAX);
    options.frames.map_or(length, |cap| cap.min(length))
}

/// The four buffers the lockstep loop reuses, so that no frame allocates.
#[derive(Debug, Default)]
struct Buffers {
    ram: (Vec<u8>, Vec<u8>),
    picture: (Vec<u8>, Vec<u8>),
}

impl Buffers {
    fn fill(&mut self, ours: &dyn Machine, theirs: &dyn Machine) -> Result<(), String> {
        ours.ram_into(&mut self.ram.0)?;
        theirs.ram_into(&mut self.ram.1)?;
        ours.picture_into(&mut self.picture.0)?;
        theirs.picture_into(&mut self.picture.1)?;
        Ok(())
    }
}

#[cfg(feature = "fceux-core")]
fn run_lockstep(
    options: &Options,
    movie: &MovieFile,
    machines: &Machines<'_>,
    measurement: &mut Measurement,
) -> Result<(), HarnessError> {
    let ram = machines.ram;
    let mut ours = zaxnes_machine(options, movie, machines)?;
    let mut theirs = FceuxMachine::new(machines.image, machines.name, machines.region)
        .map_err(HarnessError::Rom)?;
    ours.power(ram).map_err(HarnessError::Rom)?;
    theirs.power(ram).map_err(HarnessError::Rom)?;
    // The head start of `--warm-up`, before frame 0 is compared: blank frames, no input.
    for _ in 0..options.warm_up.0 {
        ours.run(0).map_err(HarnessError::Rom)?;
    }
    for _ in 0..options.warm_up.1 {
        theirs.run(0).map_err(HarnessError::Rom)?;
    }

    let total = frame_count(options, movie);
    let mut trackers = (RunTracker::new("zaxnes"), RunTracker::new("fceux"));
    let mut traces = (tracer(options, "zaxnes"), tracer(options, "fceux"));
    let mut buffers = Buffers::default();
    if traces.0.is_some() || traces.1.is_some() {
        buffers
            .fill(ours.as_ref(), &theirs)
            .map_err(HarnessError::Rom)?;
        if let Some(trace) = traces.0.as_mut() {
            trace.frame(&buffers.ram.0, None);
        }
        if let Some(trace) = traces.1.as_mut() {
            trace.frame(&buffers.ram.1, None);
        }
    }

    // Frame 0 is the machine at power-on, before an input word has been fed: the work RAM the
    // movie's header asked for, on both sides. It is compared first because a movie that starts
    // from two different 2 KiB has already lost, and there is no picture yet to compare.
    if options.from == 0 {
        buffers
            .fill(ours.as_ref(), &theirs)
            .map_err(HarnessError::Rom)?;
        let diff = compare_frame(
            0,
            (&buffers.ram.0, &buffers.ram.1),
            (&[], &[]),
            None,
            options.ram_bytes,
        );
        let _ = measurement.record(diff, options.keep_going);
    }

    for frame in 1..=total {
        let input = movie.frames[usize::try_from(frame).unwrap_or(1) - 1];
        let probe = probes(options, frame);
        let resets = resets_next(movie, frame);
        let sensitivity = match (
            step(ours.as_mut(), input, ram, probe, resets),
            step(&mut theirs, input, ram, probe, resets),
        ) {
            (Ok(our_lag), Ok(their_lag)) => our_lag.zip(their_lag),
            (Err(why), _) => {
                trackers.0.run.stopped = Some((frame, why));
                break;
            }
            (_, Err(why)) => {
                trackers.1.run.stopped = Some((frame, why));
                break;
            }
        };
        if let Err(why) = buffers.fill(ours.as_ref(), &theirs) {
            trackers.0.run.stopped = Some((frame, why));
            break;
        }
        trackers
            .0
            .frame(frame, &buffers.picture.0, sensitivity.map(|pair| pair.0));
        trackers
            .1
            .frame(frame, &buffers.picture.1, sensitivity.map(|pair| pair.1));
        if let Some(trace) = traces.0.as_mut() {
            trace.frame(&buffers.ram.0, sensitivity.map(|pair| pair.0));
        }
        if let Some(trace) = traces.1.as_mut() {
            trace.frame(&buffers.ram.1, sensitivity.map(|pair| pair.1));
        }
        write_picture(options, "zaxnes", frame, &buffers.picture.0)?;
        write_picture(options, "fceux", frame, &buffers.picture.1)?;
        write_ram(options, "zaxnes", frame, &buffers.ram.0)?;
        write_ram(options, "fceux", frame, &buffers.ram.1)?;

        if frame >= options.from {
            let diff = compare_frame(
                frame,
                (&buffers.ram.0, &buffers.ram.1),
                (&buffers.picture.0, &buffers.picture.1),
                sensitivity,
                options.ram_bytes,
            );
            if measurement.record(diff, options.keep_going) {
                break;
            }
        }
        progress(options, frame, total, measurement);
    }

    measurement.runs.push(trackers.0.finish());
    measurement.runs.push(trackers.1.finish());
    measurement.traces.extend(traces.0);
    measurement.traces.extend(traces.1);
    Ok(())
}

#[cfg(not(feature = "fceux-core"))]
fn run_lockstep(
    _options: &Options,
    _movie: &MovieFile,
    _machines: &Machines<'_>,
    _measurement: &mut Measurement,
) -> Result<(), HarnessError> {
    Err(HarnessError::Usage(
        "--engine both needs the `fceux-core` feature and a built fork".to_owned(),
    ))
}

#[cfg(feature = "fceux-core")]
fn run_fceux_alone(
    options: &Options,
    movie: &MovieFile,
    machines: &Machines<'_>,
    measurement: &mut Measurement,
) -> Result<(), HarnessError> {
    let mut machine = FceuxMachine::new(machines.image, machines.name, machines.region)
        .map_err(HarnessError::Rom)?;
    if options.movie_mode {
        let (run, trace) = run_fceux_movie_mode(options, movie, &mut machine)?;
        measurement.runs.push(run);
        measurement.traces.extend(trace);
    } else {
        let (run, trace, bytes) = run_alone(options, movie, &mut machine, machines.ram)?;
        let engine = run.engine.clone();
        measurement.runs.push(run);
        measurement.traces.extend(trace);
        measurement
            .ram_against_reference
            .extend(bytes.map(|bytes| (engine, bytes)));
    }
    Ok(())
}

#[cfg(not(feature = "fceux-core"))]
fn run_fceux_alone(
    _options: &Options,
    _movie: &MovieFile,
    _machines: &Machines<'_>,
    _measurement: &mut Measurement,
) -> Result<(), HarnessError> {
    Err(HarnessError::Usage(
        "--engine fceux needs the `fceux-core` feature and a built fork".to_owned(),
    ))
}

/// FCEUX's own movie mode, which is what the application uses: the file goes to
/// `FCEUI_LoadMovie` and the loop only turns the crank.
///
/// The input-sensitivity probe is **not** run here, and that is not laziness: movie mode loads
/// each record over the pad state inside the frame, so the probe's alternate word would be
/// overwritten before the frame ran and every frame would measure insensitive.
///
/// The **work-RAM trace is** written, and it is what makes this mode a reference rather than a
/// demonstration: FCEUX's own movie player decides for itself when a frame's command column
/// takes effect, so holding a fed-inputs run against this one settles the question of `.fm2`
/// reset timing by measurement (lot EVAL-SYNC-3 step 1) instead of by reading a sentence.
#[cfg(feature = "fceux-core")]
fn run_fceux_movie_mode(
    options: &Options,
    movie: &MovieFile,
    machine: &mut FceuxMachine,
) -> Result<(EngineRun, Option<RamTrace>), HarnessError> {
    let bytes = fs::read(&options.movie)?;
    let header = machine
        .core
        .play_movie(&bytes)
        .map_err(|error| HarnessError::Format(format!("FCEUX refused the movie: {error}")))?;
    let engine = "fceux-movie-mode";
    let mut tracker = RunTracker::new("fceux (movie mode)");
    let mut trace = tracer(options, engine);
    let total = frame_count(options, movie).min(header.frames);
    let mut picture = Vec::with_capacity(PICTURE_LEN);
    let mut work_ram = Vec::with_capacity(RAM_LEN);
    if let Some(trace) = trace.as_mut() {
        machine.ram_into(&mut work_ram).map_err(HarnessError::Rom)?;
        trace.frame(&work_ram, None);
    }
    for frame in 1..=total {
        if let Err(why) = machine.core.run_frame() {
            tracker.run.stopped = Some((frame, why.to_string()));
            break;
        }
        if let Err(why) = machine.core.frame_indices_into(&mut picture) {
            tracker.run.stopped = Some((frame, why.to_string()));
            break;
        }
        if trace.is_some() || options.dump.is_some() {
            if let Err(why) = machine.ram_into(&mut work_ram) {
                tracker.run.stopped = Some((frame, why));
                break;
            }
            if let Some(trace) = trace.as_mut() {
                trace.frame(&work_ram, None);
            }
            write_ram(options, engine, frame, &work_ram)?;
        }
        tracker.frame(frame, &picture, None);
        write_picture(options, engine, frame, &picture)?;
        if options.progress > 0 && frame % options.progress == 0 {
            println!("  movie mode: frame {frame} of {total}");
        }
    }
    let mut run = tracker.finish();
    // FCEUX's movie clock counts past the end and the mode lags it by a frame (lot FCEUX-48
    // step 3), so the clock is the signal and it is reported as it stands.
    if let Some((frame, length)) = machine.core.movie_position() {
        run.engine = format!("fceux (movie mode, clock {frame}/{length})");
    }
    Ok((run, trace))
}

/// The Zaxnes machine `options` and `movie` ask for, on the timing profile they ask for.
///
/// **This is the one place the profile is chosen** (lot NESHAWK-PROFILE step 4): `--profile` if it
/// was given, otherwise `MovieFile::recorded_profile`, which is the NesHawk-compatible chip for a
/// `.bk2` whose `Core` header says `NesHawk` and the standard one for everything else. The two
/// arms are identical but for the type, and the result is boxed because the rest of the command
/// drives a `dyn Machine` and has no business knowing which chip it holds.
fn zaxnes_machine(
    options: &Options,
    movie: &MovieFile,
    machines: &Machines<'_>,
) -> Result<Box<dyn Machine>, HarnessError> {
    fn build<P: TimingProfile>(
        options: &Options,
        movie: &MovieFile,
        machines: &Machines<'_>,
    ) -> Result<Box<dyn Machine>, HarnessError> {
        let mut machine = ZaxnesMachine::<P>::new(
            machines.image,
            machines.name,
            machines.region,
            options.builtin_patches,
        )
        .map_err(HarnessError::Rom)?;
        if let Some(pattern) = options.prg_ram {
            machine.want_prg_ram(pattern).map_err(HarnessError::Rom)?;
        }
        machine.ppu_warmup = options.ppu_warmup;
        machine.ppu_ram = options.ppu_ram;
        machine.plug_ports(movie.ports);
        Ok(Box::new(machine))
    }

    match options.profile.unwrap_or_else(|| movie.recorded_profile()) {
        ProfileChoice::Standard => build::<Standard>(options, movie, machines),
        ProfileChoice::NesHawk => build::<NesHawkCompat>(options, movie, machines),
    }
}

/// `zax-neshawk`, the Rust port of NesHawk, as a [`Machine`].
///
/// A `.bk2` replayed on a port of the core that recorded it is the whole point of lot
/// NESHAWK-REWRITE: `--engine neshawk --ram-ref <BizHawk per-frame reference>` is the instrument
/// that says whether the port and BizHawk are the same machine.
fn neshawk_machine(
    options: &Options,
    movie: &MovieFile,
    machines: &Machines<'_>,
) -> Result<Box<dyn Machine>, HarnessError> {
    let machine = NeshawkMachine::new(machines.image, machines.region, movie.ports, options)
        .map_err(HarnessError::Rom)?;
    Ok(Box::new(machine))
}

/// `zax-neshawk`'s machine behind the harness's [`Machine`] trait.
pub(crate) struct NeshawkMachine {
    nes: zax_neshawk::Nes,
    /// Kept so that a mid-movie hard reset can rebuild the machine from the image, which is what
    /// `NES.HardReset` does when it rebuilds the board.
    image: Vec<u8>,
    db: zax_neshawk::gamedb::GameDb,
    region: NesRegion,
    ports: [ControllerPortType; 2],
    /// What [`Machine::capture`] took.
    state: Option<zax_neshawk::Nes>,
    /// The cycle ledger, when `--neshawk-event-log` asked for one.
    ledger: Option<Ledger>,
}

/// The cycle ledger of `--neshawk-event-log`: the same lines `scripts/bk2cyc*.lua` writes on
/// BizHawk, so that a diff of the two files is a diff of the two machines event for event.
///
/// The filter is the scripts' own: every write to `$4000-$401F` (with `$4014` printed as `oam`,
/// which is what the Lua calls it), reads of `$2002` and `$4015`, the first instruction of the
/// NMI handler, and whatever PCs `--neshawk-watch-pc` names.
struct Ledger {
    path: PathBuf,
    text: String,
    watch_pc: Vec<u16>,
    /// The NMI vector, read off the cartridge once the machine is up.
    nmi_vector: u16,
    /// The IRQ vector, which the ledgers tag `irq`. `$FFFE` is `BRK`'s vector too, so a `BRK`
    /// would be tagged `irq` here — exactly as BizHawk's own `event.onmemoryexecute` on that
    /// address does, since neither can tell them apart from the address alone.
    irq_vector: u16,
}

impl Ledger {
    /// One frame's worth of lines, from the events the machine recorded for it.
    fn frame(&mut self, frame: u32, start: i64, lag: i32, events: &[zax_neshawk::BusEvent]) {
        use std::fmt::Write as _;
        use zax_neshawk::BusEventKind;

        let _ = writeln!(self.text, "F {frame} start={start} lag={lag}");
        for event in events {
            let at = event.cycle - start;
            match event.kind {
                BusEventKind::Exec => {
                    let tag = if event.address == self.nmi_vector {
                        "nmi".to_owned()
                    } else if event.address == self.irq_vector {
                        "irq".to_owned()
                    } else if self.watch_pc.contains(&event.address) {
                        format!("pc${:04X}", event.address)
                    } else {
                        continue;
                    };
                    let _ = writeln!(
                        self.text,
                        "  {frame} {at:6} {tag:<8} A:{:02X} X:{:02X} Y:{:02X} SP:{:02X}",
                        event.a, event.x, event.y, event.s
                    );
                }
                BusEventKind::Write => {
                    if !(0x4000..=0x401F).contains(&event.address) {
                        continue;
                    }
                    let tag = if event.address == 0x4014 {
                        "oam".to_owned()
                    } else {
                        format!("w${:04X}", event.address)
                    };
                    let _ = writeln!(self.text, "  {frame} {at:6} {tag:<8} ={:02X}", event.value);
                }
                BusEventKind::Read => {
                    if event.address != 0x2002 && event.address != 0x4015 {
                        continue;
                    }
                    let _ = writeln!(
                        self.text,
                        "  {frame} {at:6} {:<8} ={:02X}",
                        format!("r${:04X}", event.address),
                        event.value
                    );
                }
            }
        }
    }
}

impl NeshawkMachine {
    /// Where BizHawk's `gamedb` directory is, which board detection needs to be
    /// NesHawk-equivalent: `--neshawk-gamedb`, else `ZAXNES_BIZHAWK_GAMEDB`, else nothing — and
    /// then detection falls back to the iNES header, which `zax-neshawk::gamedb` explains is a
    /// *different machine* for some cartridges.
    fn gamedb_dir(options: &Options) -> Option<PathBuf> {
        options
            .neshawk_gamedb
            .clone()
            .or_else(|| std::env::var_os("ZAXNES_BIZHAWK_GAMEDB").map(PathBuf::from))
    }

    fn new(
        image: &[u8],
        region: NesRegion,
        ports: [ControllerPortType; 2],
        options: &Options,
    ) -> Result<Self, String> {
        let db = match Self::gamedb_dir(options) {
            Some(dir) => zax_neshawk::gamedb::GameDb::load_from_dir(&dir),
            None => zax_neshawk::gamedb::GameDb::empty(),
        };
        let nes = Self::load(image, &db, ports)?;
        let mut machine = Self {
            nes,
            image: image.to_vec(),
            db,
            region,
            ports,
            state: None,
            ledger: None,
        };
        machine.apply_region();
        // The owner's marker of 2026-11-12: a mapper with no ported NesHawk board runs on
        // Zaxnes's board, which is not NesHawk-exact — so a run of it must not be read as
        // NesHawk's. Printed here rather than buried in the JSON, because this is the tool whose
        // whole point is "the port and BizHawk are the same machine".
        if let Some(report) = machine.nes.load_report.as_ref() {
            if report.board_from_zaxnes {
                println!(
                    "  note: no ported NesHawk board implements this cartridge, so it runs on                      Zaxnes's own {} -- BOARD FROM ZAXNES, NOT NESHAWK-EXACT. The CPU, the PPU                      and the APU are still the port's; the board's IRQ timing is not.",
                    report.board_name
                );
            }
        }
        if let Some(path) = options.neshawk_event_log.clone() {
            let lo = machine.nes.peek_memory(0xFFFA);
            let hi = machine.nes.peek_memory(0xFFFB);
            machine.ledger = Some(Ledger {
                path,
                text: format!("romhash {}\n", "zax-neshawk"),
                watch_pc: options.neshawk_watch_pc.clone(),
                nmi_vector: u16::from(lo) | (u16::from(hi) << 8),
                irq_vector: u16::from(machine.nes.peek_memory(0xFFFE))
                    | (u16::from(machine.nes.peek_memory(0xFFFF)) << 8),
            });
            machine.nes.bus_log.arm(0);
        }
        Ok(machine)
    }

    fn load(
        image: &[u8],
        db: &zax_neshawk::gamedb::GameDb,
        ports: [ControllerPortType; 2],
    ) -> Result<zax_neshawk::Nes, String> {
        let port = |kind: ControllerPortType| match kind {
            ControllerPortType::Empty => zax_neshawk::controllers::PortDevice::Unplugged,
            // NesHawk's Zapper is not ported; a movie that names one plays a pad here, and the
            // report says which device each port got so that the substitution is visible.
            _ => zax_neshawk::controllers::PortDevice::ControllerNes,
        };
        zax_neshawk::Nes::load(image, db, port(ports[0]), port(ports[1]))
            .map_err(|why| why.to_string())
    }

    /// One frame, with the cycle ledger taken around it when one was asked for.
    fn advance(&mut self, reset: bool) {
        if self.ledger.is_none() {
            self.nes.frame_advance(reset, false);
            return;
        }
        let start = self.nes.total_executed_cycles;
        self.nes.bus_log.events.clear();
        self.nes.frame_advance(reset, false);
        let frame = u32::try_from(self.nes.frame).unwrap_or(0);
        let lag = self.nes.lag_count();
        // The borrow has to be split: the ledger formats the events the machine owns.
        let events = std::mem::take(&mut self.nes.bus_log.events);
        if let Some(ledger) = self.ledger.as_mut() {
            ledger.frame(frame, start, lag, &events);
        }
        self.nes.bus_log.events = events;
        self.nes.bus_log.events.clear();
    }

    /// The movie's region. `NES.HardReset` is what reads `_display_type`, so it is set before the
    /// power-on that [`Machine::power`] does.
    fn apply_region(&mut self) {
        self.nes.set_display_type(match self.region {
            NesRegion::Pal => zax_neshawk::DisplayType::Pal,
            NesRegion::Dendy => zax_neshawk::DisplayType::Dendy,
            _ => zax_neshawk::DisplayType::Ntsc,
        });
    }
}

impl Drop for NeshawkMachine {
    fn drop(&mut self) {
        if let Some(ledger) = self.ledger.take() {
            let _ = fs::write(&ledger.path, &ledger.text);
        }
    }
}

impl Machine for NeshawkMachine {
    fn name(&self) -> &'static str {
        "neshawk"
    }

    fn power(&mut self, ram: RamPattern) -> Result<(), String> {
        // `Nes::load` already powered the machine on once; a `FrameCommand::HardReset` mid-movie
        // starts from the image again, which is what `NES.HardReset` does by rebuilding the board.
        self.nes = Self::load(&self.image, &self.db, self.ports)?;
        self.apply_region();
        if self.ledger.is_some() {
            // `Self::load` built a new machine, whose log is off by default.
            self.nes.bus_log.arm(0);
        }
        // The movie's power-on pattern is NesHawk's own `InitialWRamStatePattern`, which
        // `HardReset` reads *before* it fills the RAM — so it is set and the machine re-powered,
        // rather than the bytes being written over the top afterwards.
        let mut pattern = vec![0u8; RAM_LEN];
        ram.fill(&mut pattern);
        self.nes.initial_wram_state_pattern = pattern;
        self.nes.hard_reset();
        Ok(())
    }

    fn soft_reset(&mut self) -> Result<(), String> {
        // Nothing to do: NesHawk applies RESET at the *start* of the frame after the one that
        // pressed it (`FrameAdvance` acts on the previous call's `resetSignal`), so the movie's
        // RESET is carried by `run_before_soft_reset` below and taking the line here as well
        // would reset twice.
        Ok(())
    }

    fn run(&mut self, word: u16) -> Result<(), String> {
        self.nes.controller_deck.pad = [(word & 0xFF) as u8, (word >> 8) as u8];
        self.advance(false);
        Ok(())
    }

    fn run_before_soft_reset(&mut self, word: u16) -> Result<(), String> {
        // `FrameAdvance` reads `controller.IsPressed("Reset")` at the end of the frame and acts on
        // it at the start of the next, so pressing it on *this* frame is exactly what the movie's
        // Reset column means.
        self.nes.controller_deck.pad = [(word & 0xFF) as u8, (word >> 8) as u8];
        self.advance(true);
        Ok(())
    }

    fn ram_into(&self, out: &mut Vec<u8>) -> Result<(), String> {
        out.clear();
        out.reserve(RAM_LEN);
        out.extend_from_slice(&self.nes.ram);
        Ok(())
    }

    fn picture_into(&self, out: &mut Vec<u8>) -> Result<(), String> {
        out.clear();
        out.reserve(PICTURE_LEN);
        // `xbuf` holds `pixel | intensity << 6`, with bit 15 marking a pixel the backdrop shows
        // through; the harness's picture is one NES colour index a pixel, which is the low six.
        out.extend(
            self.nes
                .ppu
                .xbuf
                .iter()
                .map(|&p| u8::try_from(p & 0x3F).unwrap_or(0)),
        );
        Ok(())
    }

    /// A whole second machine rather than a serialised block: `zax-neshawk` has no `SyncState`
    /// yet, and a clone is exact where a half-written serialiser would quietly not be.
    fn capture(&mut self) -> Result<(), String> {
        self.state = Some(self.nes.clone());
        Ok(())
    }

    fn restore(&mut self) -> Result<(), String> {
        match self.state.take() {
            Some(state) => {
                self.nes = state;
                Ok(())
            }
            None => Err("nothing was captured to restore".to_owned()),
        }
    }
}

/// One machine, run alone over the movie's inputs.
fn run_alone(
    options: &Options,
    movie: &MovieFile,
    machine: &mut dyn Machine,
    ram: RamPattern,
) -> Result<(EngineRun, Option<RamTrace>, Option<RamCompare>), HarnessError> {
    machine.power(ram).map_err(HarnessError::Rom)?;
    let total = frame_count(options, movie);
    let mut tracker = RunTracker::new(machine.name());
    let mut trace = tracer(options, machine.name());
    let mut bytes = ram_comparer(options)?;
    let mut picture = Vec::with_capacity(PICTURE_LEN);
    let mut work_ram = Vec::with_capacity(RAM_LEN);
    // Frame 0 of a trace is the machine at power-on, before an input word: the reference's own
    // numbering starts from a `framecount` of 0, so there must be an entry for it to line up with.
    if trace.is_some() || bytes.is_some() {
        machine.ram_into(&mut work_ram).map_err(HarnessError::Rom)?;
        if let Some(trace) = trace.as_mut() {
            trace.frame(&work_ram, None);
        }
        if let Some(bytes) = bytes.as_mut() {
            bytes.frame(0, &work_ram);
        }
    }
    for frame in 1..=total {
        let input = movie.frames[usize::try_from(frame).unwrap_or(1) - 1];
        let probe = probes(options, frame);
        let sensitive = match step(machine, input, ram, probe, resets_next(movie, frame)) {
            Ok(sensitive) => sensitive,
            Err(why) => {
                tracker.run.stopped = Some((frame, why));
                break;
            }
        };
        if let Err(why) = machine.picture_into(&mut picture) {
            tracker.run.stopped = Some((frame, why));
            break;
        }
        if trace.is_some() || bytes.is_some() || options.dump.is_some() {
            if let Err(why) = machine.ram_into(&mut work_ram) {
                tracker.run.stopped = Some((frame, why));
                break;
            }
            if let Some(trace) = trace.as_mut() {
                trace.frame(&work_ram, sensitive);
            }
            if let Some(bytes) = bytes.as_mut() {
                bytes.frame(frame, &work_ram);
            }
            write_ram(options, machine.name(), frame, &work_ram)?;
        }
        tracker.frame(frame, &picture, sensitive);
        write_picture(options, machine.name(), frame, &picture)?;
        if options.progress > 0 && frame % options.progress == 0 {
            println!("  {}: frame {frame} of {total}", machine.name());
        }
    }
    Ok((tracker.finish(), trace, bytes))
}

/// The byte-for-byte comparison `--ram-ref` asks for, or `None` when it was not asked for.
///
/// # Errors
/// The file cannot be read, is not a trace, or is a trace whose lines carry no `ram=` field — the
/// last one being a refusal rather than an empty comparison, because a reference without RAM
/// silently answering "nothing differs" is exactly the failure mode this instrument is built to
/// avoid.
fn ram_comparer(options: &Options) -> Result<Option<RamCompare>, HarnessError> {
    let Some(path) = &options.ram_ref else {
        return Ok(None);
    };
    let text = fs::read_to_string(path)?;
    let reference = Reference::parse(&text)
        .map_err(|why| HarnessError::Format(format!("{}: {why}", path.display())))?;
    RamCompare::new(&reference, options.ram_ref_offset)
        .ok_or_else(|| {
            HarnessError::Format(format!(
                "{}: no line of this reference carries a `ram=<hex digits>` field, so there is \
                 nothing to compare the work RAM against",
                path.display()
            ))
        })
        .map(|mut compare| {
            if options.ram_ref_table.is_some() {
                compare.keep_table();
            }
            Some(compare)
        })
}

fn progress(options: &Options, frame: u32, total: u32, measurement: &Measurement) {
    if options.progress == 0 || frame % options.progress != 0 {
        return;
    }
    match &measurement.first_divergence {
        None => println!("  frame {frame} of {total}: the two machines still agree"),
        Some(first) => println!(
            "  frame {frame} of {total}: parted at {}, {} of {} frames since with the same picture",
            first.frame, measurement.after.1, measurement.after.0
        ),
    }
}

/// Writes one machine's 2 KiB of work RAM when `--dump` and `--dump-frames` ask for this frame.
fn write_ram(options: &Options, engine: &str, frame: u32, ram: &[u8]) -> Result<(), HarnessError> {
    let Some(directory) = &options.dump else {
        return Ok(());
    };
    if !options.dump_frames.contains(&frame) {
        return Ok(());
    }
    fs::create_dir_all(directory)?;
    let engine = engine.replace([' ', '(', ')', '/', ','], "-");
    fs::write(directory.join(format!("{engine}-ram-{frame:06}.bin")), ram)?;
    Ok(())
}

/// Writes one picture as a PNG when `--png` and `--png-frames` ask for this frame.
fn write_picture(
    options: &Options,
    engine: &str,
    frame: u32,
    picture: &[u8],
) -> Result<(), HarnessError> {
    let Some(directory) = &options.png else {
        return Ok(());
    };
    if !options.png_frames.contains(&frame) {
        return Ok(());
    }
    fs::create_dir_all(directory)?;
    let pixels: Vec<u8> = picture
        .iter()
        .flat_map(|&index| {
            let colour = zax_engine3d::palette::nes_color(usize::from(index));
            [colour.r, colour.g, colour.b, 255]
        })
        .collect();
    let rows = u32::try_from(picture.len() / PICTURE_WIDTH).unwrap_or(0);
    let width = u32::try_from(PICTURE_WIDTH).unwrap_or(0);
    let encoded = zax_render::assets::png_image::encode_rgba(width, rows, &pixels)?;
    let engine = engine.replace([' ', '(', ')', '/', ','], "-");
    fs::write(directory.join(format!("{engine}-{frame:06}.png")), encoded)?;
    Ok(())
}

/// The subcommand.
///
/// # Errors
/// As [`run`].
pub fn command(args: &[String]) -> Result<(), HarnessError> {
    let options = Options::parse(args)?;
    print!("{}", run(&options)?.report());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_identical_frames_agree() {
        let ram = [0u8; RAM_LEN];
        let picture = vec![0u8; PICTURE_LEN];
        let diff = compare_frame(
            7,
            (&ram, &ram),
            (&picture, &picture),
            Some((true, true)),
            16,
        );
        assert!(diff.agrees());
        let line = diff.line();
        assert!(line.contains("work RAM identical"), "{line}");
        assert!(line.contains("picture identical"), "{line}");
        assert!(line.contains("both input-sensitive"), "{line}");
    }

    #[test]
    fn a_ram_difference_is_counted_whole_and_named_up_to_the_cap() {
        let ours = [0u8; RAM_LEN];
        let mut theirs = [0u8; RAM_LEN];
        for address in [0x0012, 0x0100, 0x0345, 0x07FF] {
            theirs[address] = 0xAB;
        }
        let picture = vec![0u8; PICTURE_LEN];
        let diff = compare_frame(3, (&ours, &theirs), (&picture, &picture), None, 2);
        assert!(!diff.agrees());
        assert_eq!(diff.ram_count, 4);
        assert_eq!(diff.ram, [(0x0012, 0x00, 0xAB), (0x0100, 0x00, 0xAB)]);
        assert_eq!(diff.pixels, 0);
        let line = diff.line();
        assert!(line.contains("4 RAM byte(s) differ"), "{line}");
        assert!(line.contains("$0012 00/AB"), "{line}");
        assert!(line.contains("+2 more"), "{line}");
        assert!(line.contains("sensitivity not probed"), "{line}");
    }

    #[test]
    fn a_picture_difference_names_the_first_pixel_by_column_and_row() {
        let ram = [0u8; RAM_LEN];
        let ours = vec![0u8; PICTURE_LEN];
        let mut theirs = ours.clone();
        theirs[2 * PICTURE_WIDTH + 5] = 0x21;
        theirs[100 * PICTURE_WIDTH + 200] = 0x21;
        let diff = compare_frame(1, (&ram, &ram), (&ours, &theirs), None, 16);
        assert_eq!(diff.pixels, 2);
        assert_eq!(diff.first_pixel, Some((5, 2)));
        assert!(diff.line().contains("first at (5,2)"), "{}", diff.line());
    }

    #[test]
    fn a_sensitivity_difference_alone_is_a_divergence() {
        let ram = [0u8; RAM_LEN];
        let picture = vec![0u8; PICTURE_LEN];
        let one_sided = compare_frame(
            9,
            (&ram, &ram),
            (&picture, &picture),
            Some((true, false)),
            16,
        );
        assert!(
            !one_sided.agrees(),
            "a lag frame on one side only is a divergence"
        );
        assert!(
            one_sided.line().contains("sensitivity differs"),
            "{}",
            one_sided.line()
        );
        let both = compare_frame(
            9,
            (&ram, &ram),
            (&picture, &picture),
            Some((false, false)),
            16,
        );
        assert!(both.agrees());
        assert!(both.line().contains("a lag frame"), "{}", both.line());
    }

    #[test]
    fn the_tracker_finds_the_longest_frozen_stretch_and_the_last_change() {
        let mut tracker = RunTracker::new("zaxnes");
        let still = vec![1u8; 16];
        let moved = vec![2u8; 16];
        for frame in 1..=5 {
            tracker.frame(frame, &still, None);
        }
        tracker.frame(6, &moved, None);
        for frame in 7..=20 {
            tracker.frame(frame, &still, None);
        }
        let run = tracker.finish();
        assert_eq!(run.frames, 20);
        assert_eq!(run.last_picture_change, 7);
        assert_eq!(run.longest_frozen, (13, 7));
        assert_eq!(run.last_picture.len(), 64, "a sha256 in hex");
    }

    #[test]
    fn the_tracker_finds_the_longest_insensitive_stretch() {
        let mut tracker = RunTracker::new("fceux");
        let picture = vec![0u8; 16];
        for (frame, sensitive) in [
            (1, true),
            (2, false),
            (3, false),
            (4, false),
            (5, true),
            (6, false),
            (7, false),
        ] {
            tracker.frame(frame, &picture, Some(sensitive));
        }
        let run = tracker.finish();
        assert_eq!(run.probed, (7, 2));
        assert_eq!(run.longest_insensitive, (3, 2));
        let report = run.report();
        assert!(report.contains("2 of 7 probed frames"), "{report}");
    }

    #[test]
    fn the_measurement_keeps_the_first_divergence_and_what_came_after() {
        let ram = [0u8; RAM_LEN];
        let mut other = [0u8; RAM_LEN];
        other[1] = 1;
        let picture = vec![0u8; PICTURE_LEN];
        // Two runs, because `divergence_report` only speaks when there are two machines.
        let mut measurement = Measurement {
            runs: vec![EngineRun::default(), EngineRun::default()],
            ..Measurement::default()
        };
        for frame in 1..=3 {
            let diff = compare_frame(frame, (&ram, &ram), (&picture, &picture), None, 16);
            assert!(!measurement.record(diff, None));
        }
        assert_eq!(measurement.agreed_frames, 3);
        // Frame 4 parts them, frames 5 and 6 agree again on the picture.
        let parted = compare_frame(4, (&ram, &other), (&picture, &picture), None, 16);
        assert!(!measurement.record(parted, None));
        assert_eq!(
            measurement.first_divergence.as_ref().map(|diff| diff.frame),
            Some(4)
        );
        for frame in 5..=6 {
            let diff = compare_frame(frame, (&ram, &other), (&picture, &picture), None, 16);
            assert!(!measurement.record(diff, None));
        }
        assert_eq!(measurement.after, (2, 2, Some(6)));
        assert_eq!(measurement.ram_agreed_after, 0);
        let report = measurement.report();
        assert!(
            report.contains("FIRST DIVERGENCE after 3 identical frames"),
            "{report}"
        );
        assert!(
            report.contains("come back together at times but do not stay together")
                || report.contains("came back together and stayed together"),
            "{report}"
        );
        // And `--keep-going` stops the caller.
        let diff = compare_frame(7, (&ram, &other), (&picture, &picture), None, 16);
        assert!(measurement.record(diff, Some(3)));
    }

    /// The stack page is told apart from the rest, and the two extra markers follow it: a
    /// leftover below the stack pointer is a divergence but not one the game can see.
    #[test]
    fn a_difference_only_in_the_stack_page_is_not_counted_as_one_the_game_can_see() {
        let ours = [0u8; RAM_LEN];
        let picture = vec![0u8; PICTURE_LEN];
        let stack_only = {
            let mut theirs = ours;
            theirs[0x01FB] = 0x56;
            theirs[0x0100] = 0x01;
            compare_frame(2, (&ours, &theirs), (&picture, &picture), None, 16)
        };
        assert_eq!(stack_only.ram_count, 2);
        assert_eq!(stack_only.ram_outside_stack, 0);
        assert!(!stack_only.agrees(), "it is still a divergence");
        assert!(!stack_only.is_outside_the_stack());
        assert!(!stack_only.is_visible());

        let elsewhere = {
            let mut theirs = ours;
            theirs[0x01FB] = 0x56;
            theirs[0x0345] = 0x01;
            compare_frame(9, (&ours, &theirs), (&picture, &picture), None, 16)
        };
        assert_eq!(elsewhere.ram_count, 2);
        assert_eq!(elsewhere.ram_outside_stack, 1);
        assert!(elsewhere.is_outside_the_stack());
        assert!(!elsewhere.is_visible(), "no pixel differs");

        // And the cap on named addresses must not be able to hide one: 20 stack bytes differ
        // before the one that is not in the stack, and the count still finds it.
        let hidden = {
            let mut theirs = ours;
            for byte in &mut theirs[0x0100..0x0114] {
                *byte = 0xAA;
            }
            theirs[0x0700] = 0xAA;
            compare_frame(9, (&ours, &theirs), (&picture, &picture), None, 4)
        };
        assert_eq!(hidden.ram.len(), 4, "only four are named");
        assert_eq!(hidden.ram_count, 21);
        assert_eq!(hidden.ram_outside_stack, 1);
        assert!(hidden.is_outside_the_stack());

        // The three markers in a measurement: all three move independently.
        let mut measurement = Measurement {
            runs: vec![EngineRun::default(), EngineRun::default()],
            ..Measurement::default()
        };
        let _ = measurement.record(stack_only, None);
        let _ = measurement.record(elsewhere, None);
        let mut lit = picture.clone();
        lit[0] = 1;
        let visible = compare_frame(
            30,
            (&ours, &ours),
            (&picture, &lit),
            Some((true, false)),
            16,
        );
        let _ = measurement.record(visible, None);
        assert_eq!(
            measurement.first_divergence.as_ref().map(|diff| diff.frame),
            Some(2)
        );
        assert_eq!(
            measurement.first_outside_stack.as_ref().map(|d| d.frame),
            Some(9)
        );
        assert_eq!(
            measurement.first_visible.as_ref().map(|d| d.frame),
            Some(30)
        );
        assert_eq!(measurement.visible_count, 1);
        assert_eq!(measurement.visible_at, [30]);
        let report = measurement.report();
        assert!(report.contains("would be SEEN"), "{report}");
        assert!(
            report.contains("1 compared frame(s) differ visibly: 30"),
            "{report}"
        );
    }

    #[test]
    fn the_command_line_is_read_and_a_bad_one_refused() {
        let args: Vec<String> = [
            "movie.fm2",
            "rom.nes",
            "--engine",
            "zaxnes",
            "--frames",
            "500",
            "--ram",
            "zero",
            "--no-lag",
            "--keep-going",
            "40",
            "--png-frames",
            "10,20,30",
        ]
        .iter()
        .map(|text| (*text).to_owned())
        .collect();
        let options = Options::parse(&args).unwrap_or_default();
        assert_eq!(options.movie, PathBuf::from("movie.fm2"));
        assert_eq!(options.rom, PathBuf::from("rom.nes"));
        assert_eq!(options.engine, Engines::Zaxnes);
        assert_eq!(options.frames, Some(500));
        assert_eq!(options.ram, Some(RamPattern::Zero));
        assert!(!options.lag);
        assert_eq!(options.keep_going, Some(40));
        assert_eq!(options.png_frames, [10, 20, 30]);
        // The warm-up names its engine, and only that engine moves.
        let warmed = Options::parse(
            &["a.fm2", "b.nes", "--warm-up", "fceux:3"]
                .iter()
                .map(|text| (*text).to_owned())
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();
        assert_eq!(warmed.warm_up, (0, 3));
        assert!(Options::parse(
            &["a.fm2", "b.nes", "--warm-up", "neither:1"]
                .iter()
                .map(|text| (*text).to_owned())
                .collect::<Vec<_>>()
        )
        .is_err());

        let plain = Options::parse(&["a.fm2".to_owned(), "b.nes".to_owned()]).unwrap_or_default();
        assert_eq!(plain.engine, Engines::Both);
        assert!(plain.lag);
        for bad in [
            vec!["only-one.fm2".to_owned()],
            vec![
                "a.fm2".to_owned(),
                "b.nes".to_owned(),
                "--engine".to_owned(),
                "nope".to_owned(),
            ],
            vec![
                "a.fm2".to_owned(),
                "b.nes".to_owned(),
                "--nonsense".to_owned(),
            ],
            vec![
                "a.fm2".to_owned(),
                "b.nes".to_owned(),
                "--ram".to_owned(),
                "wrong".to_owned(),
            ],
        ] {
            assert!(Options::parse(&bad).is_err(), "{bad:?} should be refused");
        }
    }

    /// `--resets` moves or drops the command column and **leaves the input words alone**, which
    /// is what makes it an experiment about the reset rather than about the movie.
    #[test]
    fn the_reset_column_can_be_moved_or_dropped_without_touching_the_inputs() {
        let movie = MovieFile {
            frames: (0..6u16)
                .map(|index| FrameInput {
                    word: index + 1,
                    command: if index == 3 {
                        FrameCommand::SoftReset
                    } else {
                        FrameCommand::None
                    },
                })
                .collect(),
            ..MovieFile::default()
        };
        let words = |file: &MovieFile| file.frames.iter().map(|f| f.word).collect::<Vec<_>>();
        let untouched = relocate_resets(movie.clone(), Resets::On);
        assert_eq!(untouched.commands(), [(3, FrameCommand::SoftReset)]);
        assert_eq!(words(&untouched), [1, 2, 3, 4, 5, 6]);

        for (resets, at) in [
            (Resets::Off, None),
            (Resets::Early, Some(2)),
            (Resets::Late, Some(4)),
        ] {
            let moved = relocate_resets(movie.clone(), resets);
            assert_eq!(
                moved.commands(),
                at.map_or_else(Vec::new, |frame| vec![(frame, FrameCommand::SoftReset)]),
                "{resets:?}"
            );
            assert_eq!(words(&moved), [1, 2, 3, 4, 5, 6], "{resets:?}");
            assert_eq!(moved.notes.len(), 1, "the report has to say it: {resets:?}");
        }

        // A reset on frame 0 moved earlier falls off the front rather than wrapping round.
        let first = MovieFile {
            frames: vec![
                FrameInput {
                    word: 0,
                    command: FrameCommand::SoftReset,
                },
                FrameInput::default(),
            ],
            ..MovieFile::default()
        };
        assert_eq!(relocate_resets(first, Resets::Early).commands(), Vec::new());
    }

    /// A format whose column means "**after** the flagged frame" — a `.bk2` — has its commands
    /// applied one frame later, and `--resets` is then relative to *that*.
    ///
    /// This is the measurement of step 1 of lot EVAL-SYNC-3 turned into a contract: `on` must be
    /// `BizHawk`'s own reading, `early` must be the one lot EVAL-SYNC-2 ran with, and the input
    /// words must not move whichever is chosen.
    #[test]
    fn a_bk2_resets_after_the_flagged_frame_and_the_option_is_relative_to_that() {
        let movie = MovieFile {
            kind: "bk2",
            reset_after_frame: true,
            frames: (0..6u16)
                .map(|index| FrameInput {
                    word: index + 1,
                    command: if index == 3 {
                        FrameCommand::SoftReset
                    } else {
                        FrameCommand::None
                    },
                })
                .collect(),
            ..MovieFile::default()
        };
        let words = |file: &MovieFile| file.frames.iter().map(|f| f.word).collect::<Vec<_>>();
        for (resets, at) in [
            (Resets::On, Some(4)),
            (Resets::Early, Some(3)),
            (Resets::Late, Some(5)),
            (Resets::Off, None),
        ] {
            let moved = relocate_resets(movie.clone(), resets);
            assert_eq!(
                moved.commands(),
                at.map_or_else(Vec::new, |frame| vec![(frame, FrameCommand::SoftReset)]),
                "{resets:?}"
            );
            assert_eq!(words(&moved), [1, 2, 3, 4, 5, 6], "{resets:?}");
            assert!(
                !moved.notes.is_empty(),
                "the report has to say it: {resets:?}"
            );
        }

        // The ordinary edge case of the format: a Reset on the movie's last frame asks for a
        // reset after a movie that has ended, which is said rather than passed over.
        let last = MovieFile {
            kind: "bk2",
            reset_after_frame: true,
            frames: vec![
                FrameInput::default(),
                FrameInput {
                    word: 0,
                    command: FrameCommand::SoftReset,
                },
            ],
            ..MovieFile::default()
        };
        let moved = relocate_resets(last, Resets::On);
        assert_eq!(moved.commands(), Vec::new());
        assert!(
            moved.notes.iter().any(|note| note.contains("outside")),
            "{:?}",
            moved.notes
        );
    }

    #[test]
    fn the_experiments_are_refused_on_the_engines_they_cannot_be_run_on() {
        for argument in ["--ppu-warmup", "--prg-ram"] {
            let mut args = vec!["a.bk2".to_owned(), "b.nes".to_owned(), argument.to_owned()];
            if argument == "--prg-ram" {
                args.push("ff".to_owned());
            }
            let options = Options::parse(&args).unwrap_or_default();
            assert_eq!(options.engine, Engines::Both, "the default is still both");
            // `run` refuses before it loads anything, so the paths need not exist.
            let error = run(&Options {
                engine: Engines::Both,
                engine_was_chosen: false,
                ..options
            })
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(
                error.contains("`--engine zaxnes` only"),
                "{argument}: {error}"
            );
        }
        assert_eq!(Resets::parse("off"), Some(Resets::Off));
        assert_eq!(Resets::parse("sideways"), None);
        assert!(Options::parse(&[
            "a.bk2".to_owned(),
            "b.nes".to_owned(),
            "--resets".to_owned(),
            "sideways".to_owned()
        ])
        .is_err());
    }

    #[test]
    fn the_probe_window_bounds_which_frames_are_probed() {
        let options = Options {
            lag_window: (100, 200),
            ..Options::default()
        };
        assert!(!probes(&options, 99));
        assert!(probes(&options, 100));
        assert!(probes(&options, 200));
        assert!(!probes(&options, 201));
        let off = Options {
            lag: false,
            ..options
        };
        assert!(!probes(&off, 150));
    }
}
