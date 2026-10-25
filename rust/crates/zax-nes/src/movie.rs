//! Movies (Rust-only, owner request): a machine state to start from and the input word of every
//! frame after it, so that a game played once can be replayed exactly, by the application or the
//! harness, with notes the player wrote on given frames.
//!
//! File (`.zmv`): the signature `ZAXMOVIE`, a format version (u32), then four sections, each a
//! u32 length and its bytes: a JSON header (free form, for people and tools: game, ROM
//! fingerprint, built-in patch, its version and options, application version, date), the start
//! state (a core state block with its ROM image, see `NesCore::save_state`), the inputs (the
//! `run_frame` word of each frame, u16), and the annotations (a u32 count, then for each the frame
//! (u32), a byte 1 when the frame was later abandoned, and its UTF-8 text as a u32 length and its
//! bytes). Every number is little-endian.
//!
//! Frames are counted from the start state: frame `n` is the picture after `n` input words, and
//! an annotation on frame `n` was written while that picture was on screen.

/// The signature that opens a movie file.
pub const SIGNATURE: &[u8; 8] = b"ZAXMOVIE";
/// The format version this build writes and reads.
pub const VERSION: u32 = 1;

/// A note the player wrote on a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    /// The frame, counted from the start state.
    pub frame: u32,
    /// Whether the recording went back before this frame afterwards (a save state loaded): the
    /// note was written on frames the movie no longer holds.
    pub abandoned: bool,
    /// The note.
    pub text: String,
}

/// A movie.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Movie {
    /// The JSON header.
    pub header: String,
    /// The core state block the movie starts from.
    pub start_state: Vec<u8>,
    /// The input word of each frame from there.
    pub inputs: Vec<u16>,
    /// The notes, in the order they were written.
    pub annotations: Vec<Annotation>,
}

impl Movie {
    /// The movie as a file.
    #[must_use]
    pub fn write(&self) -> Vec<u8> {
        let mut file = Vec::with_capacity(
            SIGNATURE.len()
                + 20
                + self.header.len()
                + self.start_state.len()
                + (2 * self.inputs.len()),
        );
        file.extend_from_slice(SIGNATURE);
        file.extend_from_slice(&VERSION.to_le_bytes());
        put_section(&mut file, self.header.as_bytes());
        put_section(&mut file, &self.start_state);
        let inputs: Vec<u8> = self
            .inputs
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        put_section(&mut file, &inputs);
        let mut notes = Vec::new();
        notes.extend_from_slice(&length(self.annotations.len()).to_le_bytes());
        for note in &self.annotations {
            notes.extend_from_slice(&note.frame.to_le_bytes());
            notes.push(u8::from(note.abandoned));
            put_section(&mut notes, note.text.as_bytes());
        }
        put_section(&mut file, &notes);
        file
    }

    /// Reads a movie file, or `None` when it is not one this build reads.
    #[must_use]
    pub fn read(file: &[u8]) -> Option<Self> {
        let rest = file.strip_prefix(SIGNATURE.as_slice())?;
        let (version, mut rest) = take_u32(rest)?;
        if version != VERSION {
            return None;
        }
        let header = take_section(&mut rest)?;
        let start_state = take_section(&mut rest)?;
        let inputs = take_section(&mut rest)?;
        let mut notes = take_section(&mut rest)?;
        if !rest.is_empty() || inputs.len() % 2 != 0 {
            return None;
        }
        let (count, after) = take_u32(notes)?;
        notes = after;
        let mut annotations = Vec::new();
        for _ in 0..count {
            let (frame, after) = take_u32(notes)?;
            let (&abandoned, mut after) = after.split_first()?;
            let text = take_section(&mut after)?;
            notes = after;
            annotations.push(Annotation {
                frame,
                abandoned: abandoned != 0,
                text: String::from_utf8(text.to_vec()).ok()?,
            });
        }
        if !notes.is_empty() {
            return None;
        }
        Some(Self {
            header: String::from_utf8(header.to_vec()).ok()?,
            start_state: start_state.to_vec(),
            inputs: inputs
                .chunks_exact(2)
                .map(|word| u16::from_le_bytes([word[0], word[1]]))
                .collect(),
            annotations,
        })
    }
}

/// Whether a movie is being recorded or played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovieMode {
    /// The player's inputs are recorded.
    Recording,
    /// The movie's inputs replace the player's.
    Playing,
}

/// A movie being recorded or played, frame by frame. The clock is the core's frame count, which
/// save states hold: loading one (or rolling back) while recording takes the movie back to that
/// frame, the inputs after it are dropped and recording goes on from there; while playing, the
/// movie goes on from that frame.
#[derive(Debug, Clone)]
pub struct MovieRun {
    mode: MovieMode,
    /// The core's frame count at the start state.
    start_frame: i64,
    movie: Movie,
    /// Whether a played movie ran out of inputs.
    played_out: bool,
    /// Whether annotations were added since the run started.
    annotated: bool,
}

impl MovieRun {
    /// Records from the state `start_state`, taken at the core's frame count `start_frame`.
    #[must_use]
    pub fn record(start_frame: i64, start_state: Vec<u8>) -> Self {
        Self {
            mode: MovieMode::Recording,
            start_frame,
            movie: Movie {
                start_state,
                ..Movie::default()
            },
            played_out: false,
            annotated: false,
        }
    }

    /// Plays `movie`, its start state loaded in the core at the frame count `start_frame`.
    #[must_use]
    pub const fn play(start_frame: i64, movie: Movie) -> Self {
        Self {
            mode: MovieMode::Playing,
            start_frame,
            movie,
            played_out: false,
            annotated: false,
        }
    }

    /// Recording or playing.
    #[must_use]
    pub const fn mode(&self) -> MovieMode {
        self.mode
    }

    /// The movie so far.
    #[must_use]
    pub const fn movie(&self) -> &Movie {
        &self.movie
    }

    /// The movie, the run ending.
    #[must_use]
    pub fn into_movie(self) -> Movie {
        self.movie
    }

    /// Whether a played movie ran out of inputs.
    #[must_use]
    pub const fn played_out(&self) -> bool {
        self.played_out
    }

    /// Whether annotations were added since the run started.
    #[must_use]
    pub const fn annotated(&self) -> bool {
        self.annotated
    }

    /// The frame of the movie at the core's frame count `frame_count` (before the start state:
    /// none).
    #[must_use]
    pub fn frame_at(&self, frame_count: i64) -> Option<u32> {
        u32::try_from(frame_count - self.start_frame).ok()
    }

    /// The input word of the frame about to run at the core's frame count `frame_count`, the
    /// player's being `word`: recorded, or replaced by the movie's.
    pub fn input(&mut self, frame_count: i64, word: u16) -> u16 {
        let Some(frame) = self.frame_at(frame_count) else {
            // Before the start state (an older save state): the movie has no say.
            if self.mode == MovieMode::Playing {
                self.played_out = true;
            }
            return word;
        };
        let index = frame as usize;
        match self.mode {
            MovieMode::Recording => {
                if index < self.movie.inputs.len() {
                    self.movie.inputs.truncate(index);
                    for note in &mut self.movie.annotations {
                        note.abandoned |= note.frame > frame;
                    }
                }
                // A state from after the end (another movie's): its frames are blank here.
                self.movie.inputs.resize(index, 0);
                self.movie.inputs.push(word);
                word
            }
            MovieMode::Playing => {
                if let Some(&recorded) = self.movie.inputs.get(index) {
                    recorded
                } else {
                    self.played_out = true;
                    word
                }
            }
        }
    }

    /// Writes `text` on the frame on screen, the core's frame count being `frame_count`.
    /// Returns the frame, or `None` before the start state.
    pub fn annotate(&mut self, frame_count: i64, text: &str) -> Option<u32> {
        let frame = self.frame_at(frame_count)?;
        self.movie.annotations.push(Annotation {
            frame,
            abandoned: false,
            text: text.to_owned(),
        });
        self.annotated = true;
        Some(frame)
    }
}

fn length(length: usize) -> u32 {
    u32::try_from(length).unwrap_or(u32::MAX)
}

fn put_section(file: &mut Vec<u8>, section: &[u8]) {
    file.extend_from_slice(&length(section.len()).to_le_bytes());
    file.extend_from_slice(section);
}

fn take_u32(bytes: &[u8]) -> Option<(u32, &[u8])> {
    let (head, rest) = bytes.split_at_checked(4)?;
    Some((u32::from_le_bytes(head.try_into().ok()?), rest))
}

fn take_section<'a>(bytes: &mut &'a [u8]) -> Option<&'a [u8]> {
    let (length, rest) = take_u32(bytes)?;
    let (section, rest) = rest.split_at_checked(usize::try_from(length).ok()?)?;
    *bytes = rest;
    Some(section)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_movie_reads_back() {
        let movie = Movie {
            header: "{\"game\":\"x\"}".to_owned(),
            start_state: vec![1, 2, 3],
            inputs: vec![0, 0x0108, 0xFFFF],
            annotations: vec![Annotation {
                frame: 2,
                abandoned: true,
                text: "ici, une épine".to_owned(),
            }],
        };
        assert_eq!(Movie::read(&movie.write()), Some(movie.clone()));
        let mut truncated = movie.write();
        truncated.pop();
        assert_eq!(Movie::read(&truncated), None);
        assert_eq!(Movie::read(b"ZAXMOVIX"), None);
    }

    #[test]
    fn loading_an_older_state_while_recording_takes_the_movie_back() {
        let mut run = MovieRun::record(100, Vec::new());
        for (frame, word) in (100..110).zip(1..) {
            assert_eq!(run.input(frame, word), word);
        }
        assert_eq!(run.annotate(108, "note"), Some(8));
        // A state saved at frame count 105 is loaded: frames 5.. are played again.
        run.input(105, 50);
        assert_eq!(run.movie().inputs, [1, 2, 3, 4, 5, 50]);
        assert!(run.movie().annotations[0].abandoned);

        let mut play = MovieRun::play(100, run.into_movie());
        assert_eq!(play.input(100, 0), 1);
        assert_eq!(play.input(105, 0), 50);
        assert!(!play.played_out());
        assert_eq!(play.input(106, 7), 7);
        assert!(play.played_out());
    }
}
