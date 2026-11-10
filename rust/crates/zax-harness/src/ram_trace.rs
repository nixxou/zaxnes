//! The **work-RAM trace**: one checksum of the 2 KiB per *N* frames, with a running count of
//! input-insensitive frames, in the shape a `BizHawk` Lua script writes it.
//!
//! Lot EVAL-SYNC could only ask "do the two engines agree with each other", and when the answer
//! was no it had nothing to say about which of them a real console would have agreed with. The
//! owner's `BizHawk` runs change that: `G:/3DSen/test-movies/bizhawk-runs/*.txt` is a third
//! machine's answer, written down every 100 frames, and a Zaxnes run can be held against it line
//! for line.
//!
//! # The checksum, and why this one
//!
//! It is not a digest this lot chose; it is the one the owner's script already took, so this
//! module reproduces it rather than improving on it:
//!
//! ```text
//! s = 0; for address in 0x000..=0x7FF: s = (s * 31 + ram[address]) mod 2^32
//! ```
//!
//! printed as eight lower-case hex digits. A rolling polynomial hash is a poor digest and an
//! excellent *tripwire*: one byte of 2 KiB changing changes it, which is all a divergence detector
//! needs.
//!
//! # Where a line sits in time, and why the offset is measured rather than assumed
//!
//! The reference's line `100 f6750d8e lag=16` is taken **at the start of the frame
//! `emu.framecount()` calls 100**, before that frame is emulated. `movie-sync` counts frames from
//! 1 and holds, after its frame *f*, a machine that has run *f* of the movie's input words. The
//! two conventions *ought* to line up at offset 0 — `framecount` is 0 before the first word is
//! fed, so at the start of `framecount` 100 exactly 100 words have been consumed — but "ought to"
//! is not a measurement, and a whole-frame offset is exactly the kind of mistake that would make
//! a perfectly good engine look broken.
//!
//! So [`RamTrace`] keeps **every** frame's checksum, not only the ones it writes out, and
//! [`compare`] is given a range of offsets to try. The offset is then a *result* of the
//! calibration run on the two *Faxanadu* references rather than an assumption built into the
//! instrument. [`best_match`] reports it, and reports the runner-up too, so that a reader can see
//! the agreement is not a coincidence of a nearly-static screen.
//!
//! # The lag column
//!
//! `lag=` in the reference is `BizHawk`'s own cumulative count of frames `emu.islagged()` reported
//! true. Neither engine here publishes a lag flag, so what is counted is `movie-sync`'s
//! input-sensitivity probe (`crate::movie_sync::probe_input_sensitivity`) coming out *insensitive*
//! — an operational stand-in whose one check against a real emulator's counter, on *Faxanadu*,
//! was exact to the frame (EVAL-SYNC step 4). It is written only when the probe actually ran.

use std::fmt::Write as _;

/// The reference script's checksum of a machine's work RAM.
///
/// `s = s * 31 + byte`, over `$0000`-`$07FF` in order, modulo 2^32.
#[must_use]
pub fn checksum(ram: &[u8]) -> u32 {
    ram.iter().fold(0u32, |sum, &byte| {
        sum.wrapping_mul(31).wrapping_add(u32::from(byte))
    })
}

/// One line of a trace: a frame, its checksum, and the lag count at that frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceLine {
    /// The frame the line is taken at, in the reference's own numbering.
    pub frame: u32,
    /// [`checksum`] of the 2 KiB.
    pub checksum: u32,
    /// Frames counted as lagging so far, when the run counted them.
    pub lag: Option<u32>,
}

impl TraceLine {
    /// The line as the reference writes it.
    #[must_use]
    pub fn text(&self) -> String {
        match self.lag {
            Some(lag) => format!("{} {:08x} lag={lag}", self.frame, self.checksum),
            None => format!("{} {:08x}", self.frame, self.checksum),
        }
    }

    /// Reads one numeric line of a reference trace, or `None` when it is not one.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let mut parts = line.split_whitespace();
        let frame = parts.next()?.parse().ok()?;
        let digits = parts.next()?;
        let checksum = u32::from_str_radix(digits, 16).ok()?;
        if digits.len() != 8 {
            return None;
        }
        let lag = parts
            .next()
            .and_then(|part| part.strip_prefix("lag="))
            .and_then(|value| value.parse().ok());
        Some(Self {
            frame,
            checksum,
            lag,
        })
    }
}

/// One engine's run, checksummed at every frame.
///
/// Every frame is kept even though only one in `step` is written out: the extra 8 bytes a frame
/// buy both the offset calibration and the ability to narrow a divergence without re-running
/// anything.
#[derive(Debug, Clone, Default)]
pub struct RamTrace {
    /// `zaxnes` or `fceux`.
    pub engine: String,
    /// How often a line is written out.
    pub step: u32,
    /// Index *f* is the machine after *f* frames of the movie; index 0 is power-on.
    frames: Vec<(u32, u32)>,
    /// Whether any frame was probed, which decides whether `lag=` is written at all.
    probed: bool,
}

impl RamTrace {
    /// A trace of one engine, writing a line every `step` frames.
    #[must_use]
    pub fn new(engine: &str, step: u32) -> Self {
        Self {
            engine: engine.to_owned(),
            step: step.max(1),
            frames: Vec::new(),
            probed: false,
        }
    }

    /// Takes one frame: its work RAM, and whether the probe found it input-sensitive.
    ///
    /// Frames must arrive in order from 0 (power-on, before any input word). The power-on entry
    /// never counts towards the lag total whatever it is handed: no frame has been emulated yet,
    /// and the reference's own counter is 0 at `framecount` 0 by construction.
    pub fn frame(&mut self, ram: &[u8], sensitive: Option<bool>) {
        let lag = match self.frames.last() {
            None => 0,
            Some((_, lag)) => lag + u32::from(matches!(sensitive, Some(false))),
        };
        self.probed |= sensitive.is_some();
        self.frames.push((checksum(ram), lag));
    }

    /// Frames taken, power-on not counted.
    #[must_use]
    pub fn length(&self) -> u32 {
        u32::try_from(self.frames.len().saturating_sub(1)).unwrap_or(u32::MAX)
    }

    /// The line for the machine after `frame` frames, or `None` when the run never reached it.
    #[must_use]
    pub fn at(&self, frame: u32) -> Option<TraceLine> {
        let (checksum, lag) = *self.frames.get(usize::try_from(frame).ok()?)?;
        Some(TraceLine {
            frame,
            checksum,
            lag: self.probed.then_some(lag),
        })
    }

    /// The lines that are written out: every `step` frames.
    #[must_use]
    pub fn lines(&self) -> Vec<TraceLine> {
        (1..=self.length())
            .filter(|frame| frame % self.step == 0)
            .filter_map(|frame| self.at(frame))
            .collect()
    }

    /// The file, header and all.
    ///
    /// The numeric lines are byte-for-byte the shape the reference uses, so that two traces can
    /// be compared with nothing more than `diff`.
    #[must_use]
    pub fn text(&self, movie: &str, rom: &str) -> String {
        let mut text = format!(
            "engine {}\nmovie {movie} length {}\nrom {rom}\n",
            self.engine,
            self.length()
        );
        for line in self.lines() {
            let _ = writeln!(text, "{}", line.text());
        }
        let _ = writeln!(text, "end frame {}", self.length());
        text
    }
}

/// A reference trace, as the owner's script wrote it.
#[derive(Debug, Clone, Default)]
pub struct Reference {
    /// The lines that are not numeric, kept for the report.
    pub header: Vec<String>,
    /// The numeric lines, in file order.
    pub lines: Vec<TraceLine>,
    /// The whole 2 KiB, for the frames whose line carries a `ram=` field.
    ///
    /// A checksum says two machines differ and only the bytes say **how**, so a reference that
    /// carries its RAM is worth far more than one that does not — it turns "the trace parts at
    /// frame *f*" into "`$004B` is `01` here and `00` there". The owner's finer script writes it;
    /// the 100-frame runs do not, and an absent field is simply an absent entry rather than an
    /// error.
    pub ram: std::collections::BTreeMap<u32, Vec<u8>>,
}

impl Reference {
    /// Reads one.
    ///
    /// # Errors
    /// The text holds no numeric line, which means it is not a trace.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut reference = Self::default();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if let Some(parsed) = TraceLine::parse(line) {
                if let Some(ram) = parse_ram(line) {
                    reference.ram.insert(parsed.frame, ram);
                }
                reference.lines.push(parsed);
            } else if !line.trim().is_empty() {
                reference.header.push(line.to_owned());
            }
        }
        if reference.lines.is_empty() {
            return Err("the file holds no `<frame> <checksum>` line".to_owned());
        }
        Ok(reference)
    }

    /// The last frame the reference reaches.
    #[must_use]
    pub fn last_frame(&self) -> u32 {
        self.lines.last().map_or(0, |line| line.frame)
    }
}

/// The `ram=<hex>` field of a reference line, as bytes, or `None` when the line carries none.
///
/// An odd number of digits, a non-hex digit or an empty field is `None` rather than a partial
/// read: a half-decoded 2 KiB would be compared against and would look like a divergence.
fn parse_ram(line: &str) -> Option<Vec<u8>> {
    let digits = line
        .split_whitespace()
        .find_map(|part| part.strip_prefix("ram="))?;
    if digits.is_empty() || digits.len() % 2 != 0 {
        return None;
    }
    let value = |byte: u8| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    };
    digits
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Some((value(pair[0])? << 4) | value(pair[1])?))
        .collect()
}

/// The stack page, whose bytes below the stack pointer are leftovers of pushes already popped.
const STACK: std::ops::Range<u16> = 0x0100..0x0200;
/// The low zero page, where a 6502 program keeps the scratch of whatever it is doing right now.
///
/// Both ranges are **artefact classes**, not excused differences: EVAL-SYNC-2 measured that the
/// two machines read the 2 KiB at a slightly different point inside the frame, so the bytes a
/// routine is in the middle of using differ on a run that is otherwise in step. *Faxanadu*, which
/// syncs exactly, shows the same signature. A difference anywhere else is a difference in what the
/// game has **decided**, and that is what [`RamCompare::first_live_diff`] is for.
const TEMPORARIES: std::ops::Range<u16> = 0x0000..0x0020;

/// One frame's byte-level difference from the reference.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameRamDiff {
    /// The reference's frame number.
    pub frame: u32,
    /// Addresses differing, in total.
    pub total: u32,
    /// Of those, in the stack page.
    pub stack: u32,
    /// Of those, in the low zero page.
    pub temporaries: u32,
    /// Of those, anywhere else — memory the game is using.
    pub live: u32,
    /// The first few differing addresses outside the two artefact classes, as
    /// (address, ours, theirs). Empty when every difference is an artefact.
    pub first_live: Vec<(u16, u8, u8)>,
}

impl FrameRamDiff {
    /// The report's one line for this frame.
    #[must_use]
    pub fn line(&self) -> String {
        let mut text = format!(
            "    frame {}: {} byte(s) differ -- {} live, {} in the stack page, {} below $0020",
            self.frame, self.total, self.live, self.stack, self.temporaries
        );
        for (address, ours, theirs) in &self.first_live {
            let _ = write!(
                text,
                "\n      ${address:04X} = {ours:02X} here, {theirs:02X} there"
            );
        }
        text
    }
}

/// How many live differences a frame's line names before it only counts them.
const LIVE_LISTED: usize = 16;
/// How many frames with a live difference the report lists.
///
/// The *report* is a paragraph a reader reads, so it stops; the **table** of
/// [`RamCompare::keep_table`] does not, which is what a window of several hundred frames with
/// isolated one-frame differences needs (EVAL-SYNC-4 step 1).
const FRAMES_LISTED: usize = 12;

/// Holds a run's work RAM against a reference that carries its own, **byte for byte**.
///
/// Fed one frame at a time during the run so that nothing has to keep 2 KiB a frame: the
/// reference's bytes are already in memory and ours are the buffer the trace is reading anyway.
#[derive(Debug, Clone, Default)]
pub struct RamCompare {
    /// The reference's frame *f* is held against our machine after *f + offset* frames, the same
    /// convention as [`compare`].
    pub offset: i64,
    /// Frames the reference carries RAM for and the run reached.
    pub compared: u32,
    /// Of those, frames whose 2 KiB is identical to the byte.
    pub identical: u32,
    /// Of those, frames with no difference outside the two artefact classes.
    pub live_agreeing: u32,
    /// The last frame with no live difference, which is the marker of sync.
    pub last_live_agreeing: Option<u32>,
    /// The first frame differing anywhere at all.
    pub first_diff: Option<FrameRamDiff>,
    /// The first frame differing **outside** the stack page and the low zero page.
    pub first_live_diff: Option<FrameRamDiff>,
    /// The first [`FRAMES_LISTED`] frames with a live difference, for the report.
    pub live_frames: Vec<FrameRamDiff>,
    /// **Every** compared frame's difference, when [`RamCompare::keep_table`] asked for it.
    ///
    /// Off by default because it is one entry per frame and a movie is 156 312 of them; on, it is
    /// what turns "the first live difference is at frame 645" into the per-frame table a window of
    /// seven hundred frames needs.
    table: Option<Vec<FrameRamDiff>>,
    /// What the reference holds: frame to 2 KiB.
    want: std::collections::BTreeMap<u32, Vec<u8>>,
}

impl RamCompare {
    /// A comparison against whatever RAM `reference` carries, or `None` when it carries none.
    #[must_use]
    pub fn new(reference: &Reference, offset: i64) -> Option<Self> {
        if reference.ram.is_empty() {
            return None;
        }
        Some(Self {
            offset,
            want: reference.ram.clone(),
            ..Self::default()
        })
    }

    /// Frames the reference carries RAM for at all, whether or not the run reached them.
    #[must_use]
    pub fn available(&self) -> u32 {
        u32::try_from(self.want.len()).unwrap_or(u32::MAX)
    }

    /// Asks for every compared frame's difference to be kept, not only the first
    /// [`FRAMES_LISTED`] with a live one.
    pub fn keep_table(&mut self) {
        self.table = Some(Vec::new());
    }

    /// Every compared frame's difference, in frame order, when [`Self::keep_table`] asked for it.
    #[must_use]
    pub fn table(&self) -> &[FrameRamDiff] {
        self.table.as_deref().unwrap_or_default()
    }

    /// The table as a file: one line per compared frame, the frames with a live difference naming
    /// their addresses.
    ///
    /// Blank-separated columns in the reference's own spirit, so that `grep` and `awk` read it:
    /// `<frame> total=<n> live=<n> stack=<n> temp=<n>` and then, for a frame with live
    /// differences, ` live:$ADDR=<ours>/<theirs>` for each one named.
    #[must_use]
    pub fn table_text(&self) -> String {
        let mut text = format!(
            "# the run's own 2 KiB against the reference's, byte for byte, at offset {}\n\
             # <frame> total=<bytes differing> live=<outside $0100-$01FF and $0000-$001F> \
             stack=<in $0100-$01FF> temp=<below $0020>\n",
            self.offset
        );
        for diff in self.table() {
            let _ = write!(
                text,
                "{} total={} live={} stack={} temp={}",
                diff.frame, diff.total, diff.live, diff.stack, diff.temporaries
            );
            for (address, ours, theirs) in &diff.first_live {
                let _ = write!(text, " live:${address:04X}={ours:02X}/{theirs:02X}");
            }
            text.push('\n');
        }
        text
    }

    /// Takes our machine after `frame` frames.
    pub fn frame(&mut self, frame: u32, ram: &[u8]) {
        // Our frame *f + offset* answers for the reference's frame *f*.
        let Ok(theirs_frame) = u32::try_from(i64::from(frame) - self.offset) else {
            return;
        };
        let Some(theirs) = self.want.get(&theirs_frame) else {
            return;
        };
        self.compared += 1;
        let mut diff = FrameRamDiff {
            frame: theirs_frame,
            ..FrameRamDiff::default()
        };
        for (index, (&ours, &them)) in ram.iter().zip(theirs.iter()).enumerate() {
            if ours == them {
                continue;
            }
            let address = u16::try_from(index).unwrap_or(u16::MAX);
            diff.total += 1;
            if STACK.contains(&address) {
                diff.stack += 1;
            } else if TEMPORARIES.contains(&address) {
                diff.temporaries += 1;
            } else {
                diff.live += 1;
                if diff.first_live.len() < LIVE_LISTED {
                    diff.first_live.push((address, ours, them));
                }
            }
        }
        if diff.total == 0 {
            self.identical += 1;
        }
        if diff.live == 0 {
            self.live_agreeing += 1;
            self.last_live_agreeing = Some(theirs_frame);
        } else {
            if self.first_live_diff.is_none() {
                self.first_live_diff = Some(diff.clone());
            }
            if self.live_frames.len() < FRAMES_LISTED {
                self.live_frames.push(diff.clone());
            }
        }
        if diff.total > 0 && self.first_diff.is_none() {
            self.first_diff = Some(diff.clone());
        }
        if let Some(table) = self.table.as_mut() {
            table.push(diff);
        }
    }

    /// The paragraph the report prints.
    #[must_use]
    pub fn report(&self) -> String {
        let mut text = format!(
            "  the reference's own 2 KiB, byte for byte, at offset {}: {} of {} frames compared, \
             {} identical to the byte, {} with no difference outside the stack page and $0000-$001F",
            self.offset,
            self.compared,
            self.available(),
            self.identical,
            self.live_agreeing
        );
        let _ = write!(
            text,
            "\n    the last frame with no live difference is {}",
            self.last_live_agreeing
                .map_or_else(|| "none".to_owned(), |frame| frame.to_string())
        );
        match &self.first_live_diff {
            None => text.push_str(
                "\n    NO live difference at any compared frame: every byte outside those two \
                 ranges agrees",
            ),
            Some(first) => {
                let _ = write!(
                    text,
                    "\n    the FIRST LIVE difference is at frame {}:\n{}",
                    first.frame,
                    first.line()
                );
                for frame in self.live_frames.iter().skip(1) {
                    let _ = write!(text, "\n{}", frame.line());
                }
            }
        }
        text
    }
}

/// What holding a trace against a reference at one offset found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Comparison {
    /// The offset tried: the reference's frame *f* is held against our machine after
    /// *f + offset* frames.
    pub offset: i64,
    /// Reference lines the run was long enough to answer for.
    pub compared: u32,
    /// Leading lines, from the first, whose checksum agrees.
    pub matched: u32,
    /// The reference frame of the last agreeing line.
    pub last_match: Option<u32>,
    /// The first line that differs: the reference frame, ours, theirs.
    pub first_diff: Option<(u32, u32, u32)>,
    /// The first line whose `lag=` differs: the reference frame, ours, theirs.
    pub first_lag_diff: Option<(u32, u32, u32)>,
    /// Lines that agree on the checksum anywhere in the run, not only in the leading stretch.
    pub agreeing: u32,
}

impl Comparison {
    /// The paragraph the report prints.
    #[must_use]
    pub fn report(&self) -> String {
        let mut text = format!(
            "  against the reference at offset {}: {} of {} lines compared agree, \
             {} of them a leading run",
            self.offset, self.agreeing, self.compared, self.matched
        );
        match (self.last_match, self.first_diff) {
            (_, None) => {
                let _ = write!(
                    text,
                    "\n    every compared line agrees, to frame {}",
                    self.last_match.unwrap_or(0)
                );
            }
            (last, Some((frame, ours, theirs))) => {
                let _ = write!(
                    text,
                    "\n    the last agreeing line is frame {}\
                     \n    the FIRST DIFFERING line is frame {frame}: {ours:08x} here, \
                     {theirs:08x} in the reference",
                    last.map_or_else(|| "none".to_owned(), |frame| frame.to_string())
                );
            }
        }
        if let Some((frame, ours, theirs)) = self.first_lag_diff {
            let _ = write!(
                text,
                "\n    the first differing lag count is at frame {frame}: {ours} here, \
                 {theirs} in the reference"
            );
        } else {
            text.push_str("\n    every compared lag count agrees");
        }
        text
    }
}

/// Holds `trace` against `reference`, reading the reference's frame *f* as our machine after
/// *f + offset* frames.
#[must_use]
pub fn compare(trace: &RamTrace, reference: &Reference, offset: i64) -> Comparison {
    let mut out = Comparison {
        offset,
        ..Comparison::default()
    };
    let mut leading = true;
    for line in &reference.lines {
        let Ok(frame) = u32::try_from(i64::from(line.frame) + offset) else {
            continue;
        };
        let Some(ours) = trace.at(frame) else {
            continue;
        };
        out.compared += 1;
        if ours.checksum == line.checksum {
            out.agreeing += 1;
            out.last_match = Some(line.frame);
            if leading {
                out.matched += 1;
            }
        } else {
            leading = false;
            if out.first_diff.is_none() {
                out.first_diff = Some((line.frame, ours.checksum, line.checksum));
            }
        }
        if let (Some(mine), Some(theirs)) = (ours.lag, line.lag) {
            if mine != theirs && out.first_lag_diff.is_none() {
                out.first_lag_diff = Some((line.frame, mine, theirs));
            }
        }
    }
    out
}

/// The offset in `offsets` that agrees with the reference for the longest leading run, with the
/// runner-up beside it.
///
/// Both are returned because the calibration's answer is only worth having if the winner is
/// *clearly* the winner: an offset that matches 833 lines beside one that matches 1 is a
/// measurement, and two offsets that both match 1 line is a coincidence of a static screen.
#[must_use]
pub fn best_match(
    trace: &RamTrace,
    reference: &Reference,
    offsets: std::ops::RangeInclusive<i64>,
) -> (Comparison, Option<Comparison>) {
    let mut all: Vec<Comparison> = offsets
        .map(|offset| compare(trace, reference, offset))
        .collect();
    // The longest leading run wins; the count of agreeing lines anywhere breaks a tie, and the
    // offset nearest zero breaks that.
    all.sort_by_key(|one| {
        (
            std::cmp::Reverse(one.matched),
            std::cmp::Reverse(one.agreeing),
            one.offset.abs(),
        )
    });
    let mut all = all.into_iter();
    let best = all.next().unwrap_or_default();
    (best, all.next())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The checksum is the reference script's, worked by hand on a few short inputs.
    #[test]
    fn the_checksum_is_the_reference_scripts() {
        assert_eq!(checksum(&[]), 0);
        assert_eq!(checksum(&[0]), 0);
        assert_eq!(checksum(&[1]), 1);
        assert_eq!(checksum(&[1, 2]), 31 + 2);
        assert_eq!(checksum(&[1, 2, 3]), (31 + 2) * 31 + 3);
        // It wraps rather than panicking, which 2 KiB of $FF reaches long before the end.
        let big = [0xFFu8; 2048];
        let by_hand = big.iter().fold(0u32, |sum, &byte| {
            sum.wrapping_mul(31).wrapping_add(u32::from(byte))
        });
        assert_eq!(checksum(&big), by_hand);
        // One byte of 2 KiB changing changes it, which is the whole requirement.
        let mut one = [0u8; 2048];
        one[1234] = 1;
        assert_ne!(checksum(&one), checksum(&[0u8; 2048]));
    }

    #[test]
    fn a_line_is_written_and_read_back() {
        let line = TraceLine {
            frame: 100,
            checksum: 0xf675_0d8e,
            lag: Some(16),
        };
        assert_eq!(line.text(), "100 f6750d8e lag=16");
        assert_eq!(TraceLine::parse("100 f6750d8e lag=16"), Some(line));
        assert_eq!(
            TraceLine::parse("100 f6750d8e"),
            Some(TraceLine { lag: None, ..line })
        );
        // The header lines of a real reference are not numeric lines.
        for not_a_line in [
            "romname Faxanadu",
            "romhash 6FE59381270DA5BE666E22C2DA34337C0D4B06BB",
            "moviemode PLAY length 83218",
            "end frame 83218 mode FINISHED",
            "",
            "100 f675 lag=1",
        ] {
            assert_eq!(TraceLine::parse(not_a_line), None, "{not_a_line:?}");
        }
    }

    #[test]
    fn a_reference_is_read_and_its_header_kept() {
        let text = "romname Faxanadu\nromhash ABCD\nmoviemode PLAY length 300\n\
                    100 00000001 lag=16\n200 00000002 lag=16\n300 00000003 lag=35\n\
                    end frame 318 mode FINISHED\n";
        let reference = Reference::parse(text).unwrap_or_default();
        assert_eq!(reference.lines.len(), 3);
        assert_eq!(reference.last_frame(), 300);
        assert_eq!(reference.header.len(), 4, "{:?}", reference.header);
        assert!(Reference::parse("nothing numeric here\n").is_err());
    }

    /// A trace of `length` frames whose checksum is the frame number, parting from `break_at`.
    fn trace_of(length: u32, break_at: u32) -> RamTrace {
        let mut trace = RamTrace::new("zaxnes", 100);
        for frame in 0..=length {
            let value = if frame >= break_at {
                frame + 7777
            } else {
                frame
            };
            // Four bytes whose checksum is injective enough for a test.
            trace.frame(&value.to_be_bytes(), Some(frame % 3 != 0));
        }
        trace
    }

    #[test]
    fn the_trace_writes_one_line_every_step_frames_and_counts_the_lag() {
        let mut trace = RamTrace::new("zaxnes", 100);
        for frame in 0..=250u32 {
            trace.frame(&frame.to_be_bytes(), Some(frame % 10 != 0));
        }
        assert_eq!(trace.length(), 250);
        let lines = trace.lines();
        assert_eq!(
            lines.iter().map(|line| line.frame).collect::<Vec<_>>(),
            [100, 200]
        );
        // Frames 1..=100 hold ten multiples of ten, so ten of them are insensitive. Frame 0 is a
        // multiple of ten too and must *not* be counted: it is the power-on entry.
        assert_eq!(lines[0].lag, Some(10));
        assert_eq!(lines[1].lag, Some(20));
        assert_eq!(trace.at(0).and_then(|line| line.lag), Some(0));
        assert_eq!(trace.at(251), None, "past the end of the run");
        let text = trace.text("a.bk2", "b.nes");
        assert!(
            text.starts_with("engine zaxnes\nmovie a.bk2 length 250\n"),
            "{text}"
        );
        assert!(text.contains("\n100 "), "{text}");
        assert!(text.ends_with("end frame 250\n"), "{text}");
    }

    /// With no probe there is no `lag=` column, because an absent count must not read as zero.
    #[test]
    fn an_unprobed_trace_writes_no_lag_column() {
        let mut trace = RamTrace::new("fceux", 2);
        for frame in 0..=4u32 {
            trace.frame(&frame.to_be_bytes(), None);
        }
        assert_eq!(trace.at(4).and_then(|line| line.lag), None);
        assert!(!trace.text("a", "b").contains("lag="));
    }

    #[test]
    fn a_comparison_finds_the_last_agreeing_line_and_the_first_differing_one() {
        let trace = trace_of(1000, 450);
        let reference = {
            let whole = trace_of(1000, u32::MAX);
            let mut text = String::from("romname X\n");
            for line in whole.lines() {
                let _ = writeln!(text, "{}", line.text());
            }
            Reference::parse(&text).unwrap_or_default()
        };
        let same = compare(&trace, &reference, 0);
        assert_eq!(same.compared, 10);
        assert_eq!(same.matched, 4, "frames 100 to 400");
        assert_eq!(same.agreeing, 4);
        assert_eq!(same.last_match, Some(400));
        assert_eq!(same.first_diff.map(|(frame, _, _)| frame), Some(500));
        assert_eq!(same.first_lag_diff, None, "the lag column is untouched");
        let report = same.report();
        assert!(
            report.contains("the FIRST DIFFERING line is frame 500"),
            "{report}"
        );
        assert!(
            report.contains("every compared lag count agrees"),
            "{report}"
        );

        // A run that never parts says so.
        let whole = compare(&trace_of(1000, u32::MAX), &reference, 0);
        assert_eq!(whole.first_diff, None);
        assert_eq!(whole.matched, 10);
        assert!(
            whole.report().contains("every compared line agrees"),
            "{}",
            whole.report()
        );
    }

    /// The calibration: a trace deliberately shifted by one frame is matched at offset -1, and
    /// the winner wins by a distance no coincidence could cover.
    #[test]
    fn the_offset_is_found_rather_than_assumed() {
        let reference = {
            let whole = trace_of(1000, u32::MAX);
            let mut text = String::new();
            for line in whole.lines() {
                let _ = writeln!(text, "{}", line.text());
            }
            Reference::parse(&text).unwrap_or_default()
        };
        // A run one frame ahead of the reference: our frame f + 1 is the reference's frame f.
        let mut shifted = RamTrace::new("zaxnes", 100);
        shifted.frame(&0u32.to_be_bytes(), Some(true));
        for frame in 0..=1000u32 {
            shifted.frame(&frame.to_be_bytes(), Some(frame % 3 != 0));
        }
        let (best, runner_up) = best_match(&shifted, &reference, -3..=3);
        assert_eq!(best.offset, 1);
        assert_eq!(best.matched, 10);
        assert_eq!(best.first_diff, None);
        let runner_up = runner_up.unwrap_or_default();
        assert_eq!(
            runner_up.matched, 0,
            "no other offset matches even one line"
        );

        // And at offset 0 the same trace matches nothing, which is what makes the winner a result.
        assert_eq!(compare(&shifted, &reference, 0).matched, 0);
    }

    /// An offset that runs the reference off the end of a short run compares nothing rather than
    /// claiming agreement.
    #[test]
    fn a_run_shorter_than_the_reference_compares_only_what_it_reached() {
        let reference = {
            let whole = trace_of(1000, u32::MAX);
            let mut text = String::new();
            for line in whole.lines() {
                let _ = writeln!(text, "{}", line.text());
            }
            Reference::parse(&text).unwrap_or_default()
        };
        let short = trace_of(250, u32::MAX);
        let out = compare(&short, &reference, 0);
        assert_eq!(out.compared, 2, "frames 100 and 200 only");
        assert_eq!(out.matched, 2);
        assert_eq!(out.first_diff, None);
    }

    /// The `ram=` field of the finer reference, and the refusals that keep a half-read 2 KiB from
    /// being compared against.
    #[test]
    fn the_references_own_bytes_are_read_when_it_carries_them() {
        let line = "7 0000002a lag=1 ram=00ff10";
        assert_eq!(parse_ram(line), Some(vec![0x00, 0xFF, 0x10]));
        // Upper case too: the owner's script writes lower case, but the field is hex either way.
        assert_eq!(
            parse_ram("7 0000002a ram=00FF10"),
            Some(vec![0, 0xFF, 0x10])
        );
        for not_bytes in [
            "7 0000002a lag=1",      // the 100-frame references carry no RAM at all
            "7 0000002a ram=",       // an empty field
            "7 0000002a ram=00f",    // an odd number of digits
            "7 0000002a ram=00gg10", // not hex
        ] {
            assert_eq!(parse_ram(not_bytes), None, "{not_bytes:?}");
        }
        // And the line itself still parses as a trace line, so a reference with RAM is also a
        // reference the checksum comparison can use -- which is what lets one file measure the
        // offset and supply the bytes.
        let parsed = TraceLine::parse(line).unwrap_or(TraceLine {
            frame: 0,
            checksum: 0,
            lag: None,
        });
        assert_eq!(
            (parsed.frame, parsed.checksum, parsed.lag),
            (7, 42, Some(1))
        );

        let reference = Reference::parse(&format!("romhash ABCD\n{line}\n8 0000002b lag=1\n"))
            .unwrap_or_default();
        assert_eq!(reference.lines.len(), 2);
        assert_eq!(reference.ram.len(), 1, "only frame 7 carries bytes");
        assert_eq!(reference.ram.get(&7), Some(&vec![0x00, 0xFF, 0x10]));
    }

    /// The byte-for-byte comparison: what it counts, where it draws the artefact line, and that a
    /// reference with no RAM is refused rather than answering "nothing differs".
    #[test]
    fn the_byte_comparison_separates_the_artefact_pages_from_the_game_s_own_memory() {
        let hex = |bytes: &[u8]| -> String {
            bytes.iter().fold(String::new(), |mut text, byte| {
                let _ = write!(text, "{byte:02x}");
                text
            })
        };
        // Four frames of a 2 KiB machine, the reference's own.
        let theirs = |frame: u32| -> Vec<u8> {
            let mut ram = vec![0u8; 2048];
            ram[0x0400] = u8::try_from(frame).unwrap_or(0);
            ram
        };
        let mut text = String::new();
        for frame in 1..=4u32 {
            let ram = theirs(frame);
            let _ = writeln!(
                text,
                "{frame} {:08x} lag=0 ram={}",
                checksum(&ram),
                hex(&ram)
            );
        }
        let reference = Reference::parse(&text).unwrap_or_default();
        let mut compare = RamCompare::new(&reference, 0).unwrap_or_default();
        assert_eq!(compare.available(), 4);

        for frame in 1..=4u32 {
            let mut ram = theirs(frame);
            // Frame 2 differs only in the two artefact classes; frame 3 differs in one live byte
            // as well; frame 4 is identical.
            if frame == 2 || frame == 3 {
                ram[0x01F0] ^= 0xFF; // the stack page
                ram[0x0005] ^= 0xFF; // below $0020
            }
            if frame == 3 {
                ram[0x004B] = 0x01; // the one that matters
            }
            compare.frame(frame, &ram);
        }

        assert_eq!(compare.compared, 4);
        assert_eq!(compare.identical, 2, "frames 1 and 4");
        assert_eq!(compare.live_agreeing, 3, "every frame but 3");
        assert_eq!(compare.last_live_agreeing, Some(4));
        assert_eq!(
            compare.first_diff.as_ref().map(|diff| diff.frame),
            Some(2),
            "the first difference of any kind is the artefact one"
        );
        let live = compare.first_live_diff.clone().unwrap_or_default();
        assert_eq!(live.frame, 3);
        assert_eq!(
            (live.total, live.live, live.stack, live.temporaries),
            (3, 1, 1, 1)
        );
        assert_eq!(live.first_live, [(0x004B, 0x01, 0x00)]);
        let report = compare.report();
        assert!(
            report.contains("the FIRST LIVE difference is at frame 3"),
            "{report}"
        );
        assert!(report.contains("$004B = 01 here, 00 there"), "{report}");

        // A reference whose lines carry no RAM gives no comparison at all, which the caller turns
        // into a refusal: a comparison against nothing would report perfect agreement.
        let plain = Reference::parse("1 00000001 lag=0\n").unwrap_or_default();
        assert!(RamCompare::new(&plain, 0).is_none());
    }

    /// The per-frame table keeps **every** compared frame, where the report keeps the first
    /// [`FRAMES_LISTED`]. EVAL-SYNC-4 step 1: a window of several hundred frames holding isolated
    /// one-frame differences cannot be read off a paragraph that stops at twelve.
    #[test]
    fn the_table_keeps_every_compared_frame_and_the_report_still_stops() {
        let hex = |bytes: &[u8]| -> String {
            bytes.iter().fold(String::new(), |mut text, byte| {
                let _ = write!(text, "{byte:02x}");
                text
            })
        };
        // Twenty frames whose live byte differs on every one of them, which is more than the
        // report lists.
        let frames = 20u32;
        let mut text = String::new();
        for frame in 1..=frames {
            let ram = vec![0u8; 2048];
            let _ = writeln!(
                text,
                "{frame} {:08x} lag=0 ram={}",
                checksum(&ram),
                hex(&ram)
            );
        }
        let reference = Reference::parse(&text).unwrap_or_default();

        let mut plain = RamCompare::new(&reference, 0).unwrap_or_default();
        let mut tabled = RamCompare::new(&reference, 0).unwrap_or_default();
        tabled.keep_table();
        for frame in 1..=frames {
            let mut ram = vec![0u8; 2048];
            ram[0x0400] = u8::try_from(frame).unwrap_or(0);
            plain.frame(frame, &ram);
            tabled.frame(frame, &ram);
        }

        // The two agree about what they measured; they differ only in how much they kept.
        assert_eq!(plain.compared, frames);
        assert_eq!(tabled.compared, frames);
        assert_eq!(plain.live_frames.len(), FRAMES_LISTED);
        assert_eq!(plain.table().len(), 0, "off by default");
        assert_eq!(tabled.table().len(), usize::try_from(frames).unwrap_or(0));
        assert_eq!(plain.report(), tabled.report());

        let table = tabled.table_text();
        assert!(table.starts_with("# the run's own 2 KiB"), "{table}");
        assert!(table.contains("\n1 total=1 live=1 stack=0 temp=0 live:$0400=01/00\n"));
        assert!(table.contains("\n20 total=1 live=1 stack=0 temp=0 live:$0400=14/00\n"));
        assert_eq!(
            table.lines().filter(|line| !line.starts_with('#')).count(),
            usize::try_from(frames).unwrap_or(0)
        );

        // A frame with no difference at all is still a line, which is what makes the table a
        // table rather than a list of complaints.
        let mut quiet = RamCompare::new(&reference, 0).unwrap_or_default();
        quiet.keep_table();
        quiet.frame(1, &vec![0u8; 2048]);
        assert!(
            quiet
                .table_text()
                .contains("\n1 total=0 live=0 stack=0 temp=0\n"),
            "{}",
            quiet.table_text()
        );
    }

    /// The offset is honoured the same way [`compare`] honours it: the reference's frame *f* is
    /// our machine after *f + offset* frames.
    #[test]
    fn the_byte_comparison_honours_the_offset() {
        let mut ram = vec![0u8; 2048];
        ram[0x0400] = 0x42;
        let digits = ram.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        });
        let reference = Reference::parse(&format!("5 {:08x} ram={digits}\n", checksum(&ram)))
            .unwrap_or_default();

        // At offset 1 our frame 6 answers for their frame 5.
        let mut compare = RamCompare::new(&reference, 1).unwrap_or_default();
        compare.frame(5, &ram);
        assert_eq!(compare.compared, 0, "our frame 5 answers for their frame 4");
        compare.frame(6, &ram);
        assert_eq!(compare.compared, 1);
        assert_eq!(compare.identical, 1);
        assert_eq!(compare.first_live_diff, None);
        assert!(compare
            .report()
            .contains("NO live difference at any compared frame"));
    }

    #[test]
    fn a_differing_lag_count_is_reported_even_when_the_checksums_agree() {
        let mut ours = RamTrace::new("zaxnes", 100);
        for frame in 0..=200u32 {
            // One lag frame of our own, at frame 50, that the reference does not have.
            ours.frame(&frame.to_be_bytes(), Some(frame != 50));
        }
        let mut text = String::new();
        for frame in [100u32, 200] {
            let _ = writeln!(text, "{frame} {:08x} lag=0", checksum(&frame.to_be_bytes()));
        }
        let reference = Reference::parse(&text).unwrap_or_default();
        let out = compare(&ours, &reference, 0);
        assert_eq!(out.matched, 2, "the work RAM agrees at both lines");
        assert_eq!(out.first_lag_diff, Some((100, 1, 0)));
        assert!(
            out.report()
                .contains("the first differing lag count is at frame 100"),
            "{}",
            out.report()
        );
    }
}
