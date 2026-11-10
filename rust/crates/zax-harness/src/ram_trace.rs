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
