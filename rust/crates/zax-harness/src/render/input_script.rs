//! A scripted controller replay for `render --rom`: which port 1 buttons are held on which
//! emulated frames (port of `InputScript.cs`).
//!
//! The spec is a comma separated list of `<first>[-<last>]:<buttons>`, the frame numbers counting
//! from the reset and the buttons joined by `+`. Ranges may overlap: the held buttons are the
//! union.

use crate::HarnessError;

/// The buttons of the port 1 input word, SPEC-02 §4.4: A, B, Select, Start, Up, Down, Left,
/// Right in bits 0 to 7.
const BUTTONS: [(&str, u16); 8] = [
    ("a", 1 << 0),
    ("b", 1 << 1),
    ("select", 1 << 2),
    ("start", 1 << 3),
    ("up", 1 << 4),
    ("down", 1 << 5),
    ("left", 1 << 6),
    ("right", 1 << 7),
];

/// One entry of the script: the buttons held from `first` to `last`, both included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    first: u32,
    last: u32,
    buttons: u16,
}

/// The port 1 buttons held on each emulated frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputScript {
    spans: Vec<Span>,
}

impl InputScript {
    /// Parses the `--press` value; an empty or blank text holds nothing.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] when an entry is not `<frames>:<buttons>`, the frames are not a
    /// frame or a range, or a button is unknown.
    pub fn parse(text: &str) -> Result<Self, HarnessError> {
        let mut spans = Vec::new();
        for entry in text
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            let (frames, names) = entry
                .split_once(':')
                .filter(|(frames, _)| !frames.is_empty())
                .ok_or_else(|| {
                    HarnessError::Usage(format!("--press: '{entry}' is not <frames>:<buttons>."))
                })?;
            let (first, last) = parse_frames(frames)?;
            let buttons = names
                .split('+')
                .filter(|name| !name.is_empty())
                .try_fold(0, |held, name| {
                    Ok::<_, HarnessError>(held | button(name.trim())?)
                })?;
            spans.push(Span {
                first,
                last,
                buttons,
            });
        }

        Ok(Self { spans })
    }

    /// The port 1 input word of REQ-CORE-022 for emulated frame `frame`.
    #[must_use]
    pub fn at(&self, frame: u32) -> u16 {
        self.spans
            .iter()
            .filter(|span| (span.first..=span.last).contains(&frame))
            .fold(0, |word, span| word | span.buttons)
    }
}

/// `<first>` or `<first>-<last>`.
fn parse_frames(frames: &str) -> Result<(u32, u32), HarnessError> {
    let (first, last) = frames.split_once('-').unwrap_or((frames, frames));
    first
        .parse::<u32>()
        .ok()
        .zip(last.parse::<u32>().ok())
        .filter(|(first, last)| last >= first)
        .ok_or_else(|| {
            HarnessError::Usage(format!(
                "--press: '{frames}' is not a frame or a frame range."
            ))
        })
}

fn button(name: &str) -> Result<u16, HarnessError> {
    BUTTONS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|&(_, bit)| bit)
        .ok_or_else(|| {
            HarnessError::Usage(format!(
                "--press: '{name}' is not a button (a, b, select, start, up, down, left, right)."
            ))
        })
}
