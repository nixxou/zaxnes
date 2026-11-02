//! The timeline of a movie being played (EDITOR-1): a core state every 60 frames over the
//! **whole** movie, kept as long as the movie plays, so that any frame of it can be reached from
//! the nearest checkpoint before it.
//!
//! The core at a frame of the movie depends on the movie only (the scripts of a profile that
//! change the core aside, see [`MovieTimeline::new`]); the scripts' state depends on their whole
//! past, so a seek replays the last [`WARM_FRAMES`] frames before its target with the scripts
//! running ("warm"), and the frames before those with the core alone when it may.
//!
//! Memory: one core state (no ROM image) per second of movie, about 20 KiB, so about 12 MiB for
//! ten minutes. Nothing here runs unless a movie plays.

use std::collections::BTreeMap;

/// A checkpoint is taken on every core frame count that is a multiple of this.
pub const CHECKPOINT_INTERVAL: i64 = 60;
/// The frames before a seek's target that run with the scripts (and the conversion), so that
/// what they smooth or accumulate over a second or so is right where the seek stops.
pub const WARM_FRAMES: i64 = 60;

/// The checkpoints of the movie being played, and the seek under way.
#[derive(Debug, Clone)]
pub struct MovieTimeline {
    /// The core's frame count at the movie's start state.
    start_frame: i64,
    /// The movie's frames.
    length: u32,
    /// Core states (no ROM image) by core frame count; the start state is one of them.
    checkpoints: BTreeMap<i64, Vec<u8>>,
    /// Whether the frames before the warm ones may run with the core alone: not when a script
    /// changes the core or the inputs, since the core's past then depends on the scripts.
    core_only_allowed: bool,
    /// The core frame count a seek is heading for.
    seek_target: Option<i64>,
}

impl MovieTimeline {
    /// The timeline of a movie of `length` frames whose start state, `start_state`, is at the
    /// core's frame count `start_frame`.
    #[must_use]
    pub fn new(
        start_frame: i64,
        length: u32,
        start_state: Vec<u8>,
        core_only_allowed: bool,
    ) -> Self {
        let mut checkpoints = BTreeMap::new();
        checkpoints.insert(start_frame, start_state);
        Self {
            start_frame,
            length,
            checkpoints,
            core_only_allowed,
            seek_target: None,
        }
    }

    /// The core's frame count at the movie's start.
    #[must_use]
    pub const fn start_frame(&self) -> i64 {
        self.start_frame
    }

    /// The movie's frames.
    #[must_use]
    pub const fn length(&self) -> u32 {
        self.length
    }

    /// The core frame count of the movie's last frame shown (after all its inputs).
    #[must_use]
    pub fn end_frame(&self) -> i64 {
        self.start_frame + i64::from(self.length)
    }

    /// The movie frame at the core frame count `frame_count`, clamped to the movie.
    #[must_use]
    pub fn movie_frame(&self, frame_count: i64) -> u32 {
        u32::try_from((frame_count - self.start_frame).clamp(0, i64::from(self.length)))
            .unwrap_or(0)
    }

    /// The core frame count of the movie frame `frame`, clamped to the movie.
    #[must_use]
    pub fn core_frame(&self, frame: u32) -> i64 {
        self.start_frame + i64::from(frame.min(self.length))
    }

    /// Whether a checkpoint is wanted at the core frame count `frame_count`: a multiple of
    /// [`CHECKPOINT_INTERVAL`] inside the movie that has none yet.
    #[must_use]
    pub fn wants(&self, frame_count: i64) -> bool {
        frame_count % CHECKPOINT_INTERVAL == 0
            && frame_count > self.start_frame
            && frame_count < self.end_frame()
            && !self.checkpoints.contains_key(&frame_count)
    }

    /// Keeps `state` as the checkpoint of `frame_count`.
    pub fn record(&mut self, frame_count: i64, state: Vec<u8>) {
        self.checkpoints.insert(frame_count, state);
    }

    /// The furthest core frame count that has a checkpoint.
    #[must_use]
    pub fn furthest_checkpoint(&self) -> i64 {
        self.checkpoints
            .keys()
            .next_back()
            .copied()
            .unwrap_or(self.start_frame)
    }

    /// The number of checkpoints, and the bytes they hold.
    #[must_use]
    pub fn footprint(&self) -> (usize, usize) {
        (
            self.checkpoints.len(),
            self.checkpoints.values().map(Vec::len).sum(),
        )
    }

    /// The checkpoint at or before the core frame count `frame_count`.
    #[must_use]
    pub fn checkpoint_before(&self, frame_count: i64) -> Option<(i64, &[u8])> {
        self.checkpoints
            .range(..=frame_count)
            .next_back()
            .map(|(&frame, state)| (frame, state.as_slice()))
    }

    /// Whether the frames before the warm ones may run with the core alone.
    #[must_use]
    pub const fn core_only_allowed(&self) -> bool {
        self.core_only_allowed
    }

    /// Starts a seek to the movie frame `frame`.
    pub fn seek(&mut self, frame: u32) {
        self.seek_target = Some(self.core_frame(frame));
    }

    /// The core frame count the seek under way is heading for.
    #[must_use]
    pub const fn seek_target(&self) -> Option<i64> {
        self.seek_target
    }

    /// The seek has arrived (or was given up).
    pub fn end_seek(&mut self) {
        self.seek_target = None;
    }

    /// Where a seek to `target` starts from when the core stands at `current`: `None` to go on
    /// from there (it is on the way, and early enough to warm the scripts up, or the target is
    /// within the warm frames ahead), else the checkpoint to restore.
    #[must_use]
    pub fn seek_origin(&self, current: i64, target: i64) -> Option<(i64, &[u8])> {
        let wanted = (target - WARM_FRAMES).max(self.start_frame);
        let (frame, state) = self.checkpoint_before(wanted)?;
        // Going on from the current frame is as good as the checkpoint when it is between the
        // two: no further back than the checkpoint, not past the target.
        if current >= frame && current <= target {
            return None;
        }
        Some((frame, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timeline() -> MovieTimeline {
        MovieTimeline::new(3, 1000, vec![0], true)
    }

    #[test]
    fn frames_convert_both_ways_and_clamp() {
        let timeline = timeline();
        assert_eq!(timeline.end_frame(), 1003);
        assert_eq!(timeline.core_frame(10), 13);
        assert_eq!(timeline.core_frame(5000), 1003);
        assert_eq!(timeline.movie_frame(13), 10);
        assert_eq!(timeline.movie_frame(0), 0);
        assert_eq!(timeline.movie_frame(2000), 1000);
    }

    #[test]
    fn checkpoints_are_wanted_once_on_multiples_inside_the_movie() {
        let mut timeline = timeline();
        assert!(!timeline.wants(0), "before the movie");
        assert!(!timeline.wants(61));
        assert!(timeline.wants(60));
        timeline.record(60, vec![1]);
        assert!(!timeline.wants(60), "already there");
        assert!(!timeline.wants(1020), "after the movie");
        assert_eq!(timeline.furthest_checkpoint(), 60);
        assert_eq!(timeline.footprint(), (2, 2));
    }

    #[test]
    fn a_seek_starts_from_the_checkpoint_warm_frames_before_or_goes_on() {
        let mut timeline = timeline();
        timeline.record(60, vec![1]);
        timeline.record(120, vec![2]);
        timeline.record(180, vec![3]);
        // Target 200: wanted 140, so the checkpoint of 120.
        assert_eq!(timeline.seek_origin(500, 200).map(|(f, _)| f), Some(120));
        // Already between 120 and 200: go on.
        assert_eq!(timeline.seek_origin(150, 200), None);
        // Backwards: restore.
        assert_eq!(timeline.seek_origin(201, 200).map(|(f, _)| f), Some(120));
        // Near the start: the start state.
        assert_eq!(timeline.seek_origin(500, 30).map(|(f, _)| f), Some(3));
        // Far ahead of every checkpoint: the furthest one, or go on when past it.
        assert_eq!(timeline.seek_origin(10, 900).map(|(f, _)| f), Some(180));
        assert_eq!(timeline.seek_origin(400, 900), None);
    }
}
