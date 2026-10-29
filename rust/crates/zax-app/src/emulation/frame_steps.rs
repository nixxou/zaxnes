//! The frame-by-frame history (Rust-only, owner request): the core's state before each of the
//! last emulated frames, with that frame's input, so that the player can step back one frame at
//! a time (J) and on again (K) to look at a frame closely. Unlike the rollback's snapshot every
//! 60 frames (`rollback_history`), one entry per frame, over a few seconds. Taken where the
//! rollback's are (not on a handheld), never persisted, cleared with the rollback's.

use std::collections::VecDeque;

use crate::input::InputAction;

/// Frames kept: five seconds in NTSC.
pub const CAPACITY: usize = 300;

/// One frame: the core's state before it ran, and the input it ran with.
#[derive(Debug, Clone, Default)]
struct Entry {
    state: Vec<u8>,
    player_one: InputAction,
    player_two: InputAction,
}

/// The last frames, oldest first; the newest is the frame on screen.
#[derive(Debug, Clone, Default)]
pub struct FrameSteps {
    entries: VecDeque<Entry>,
}

impl FrameSteps {
    /// An empty history.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Frames held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no frame is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Forgets every frame.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Keeps the state `write` puts in its buffer (the core before the frame about to run) with
    /// the frame's input; the oldest frame's buffer is reused once the history is full. A failure
    /// of `write` keeps nothing.
    ///
    /// # Errors
    ///
    /// What `write` returns.
    pub fn capture_with<E>(
        &mut self,
        write: impl FnOnce(&mut Vec<u8>) -> Result<(), E>,
        player_one: InputAction,
        player_two: InputAction,
    ) -> Result<(), E> {
        let mut entry = if self.entries.len() == CAPACITY {
            self.entries.pop_front().unwrap_or_default()
        } else {
            Entry::default()
        };
        entry.state.clear();
        write(&mut entry.state)?;
        entry.player_one = player_one;
        entry.player_two = player_two;
        self.entries.push_back(entry);
        Ok(())
    }

    /// One frame back: forgets the frame on screen and gives the state before the one that came
    /// before it, with that frame's input (running it shows it again); none when fewer than two
    /// frames are held.
    pub fn step_back(&mut self) -> Option<(Vec<u8>, InputAction, InputAction)> {
        if self.entries.len() < 2 {
            return None;
        }
        self.entries.pop_back();
        self.entries
            .back()
            .map(|entry| (entry.state.clone(), entry.player_one, entry.player_two))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_back_gives_the_frame_before_and_forgets_the_one_on_screen() {
        let mut steps = FrameSteps::new();
        for frame in 0u8..3 {
            let input = if frame == 1 {
                InputAction::A
            } else {
                InputAction::default()
            };
            steps
                .capture_with(
                    |buffer| {
                        buffer.push(frame);
                        Ok::<(), ()>(())
                    },
                    input,
                    InputAction::default(),
                )
                .unwrap();
        }
        // On screen: frame 2. Back: the state before frame 1, and its input.
        let (state, one, _) = steps.step_back().unwrap();
        assert_eq!(vec![1], state);
        assert_eq!(InputAction::A, one);
        assert_eq!(2, steps.len());
        steps.step_back().unwrap();
        assert_eq!(
            None,
            steps.step_back(),
            "the oldest frame has no frame before it"
        );
    }

    #[test]
    fn the_history_keeps_its_last_frames() {
        let mut steps = FrameSteps::new();
        for _ in 0..CAPACITY + 10 {
            steps
                .capture_with(
                    |_| Ok::<(), ()>(()),
                    InputAction::default(),
                    InputAction::default(),
                )
                .unwrap();
        }
        assert_eq!(CAPACITY, steps.len());
    }
}
