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
use std::hash::{Hash, Hasher};

use zax_nes::NesCore;

/// A checkpoint is taken on every core frame count that is a multiple of this.
pub const CHECKPOINT_INTERVAL: i64 = 60;
/// The frames before a seek's target that run with the scripts (and the conversion), so that
/// what they smooth or accumulate over a second or so is right where the seek stops.
pub const WARM_FRAMES: i64 = 60;

/// What two cores at the same frame of a movie must agree on to be the same machine as far as
/// the game is concerned: the frame and cycle counts, the work RAM and the last picture. The
/// state blocks themselves are not compared: they also hold the audio resampler's position and
/// filter history (REQ-CORE-090), which follow the host's drift correction and differ between
/// two runs of the same frames while nothing the emulation reads depends on them.
#[must_use]
pub fn machine_digest(core: &NesCore) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    core.frame_count().hash(&mut hasher);
    core.cycle_count().hash(&mut hasher);
    for address in 0..0x0800u16 {
        core.read_memory(address).hash(&mut hasher);
    }
    // LUA-MERGE: `pixel_frame` became `pixel_frame_with_mask` on `dev` (AMD-050 REQ-CORE-049
    // put the opacity bit in bit 6 of each byte). It is the same slice, so the digest keeps the
    // meaning it had — and the opacity bit only makes it finer.
    core.pixel_frame_with_mask().hash(&mut hasher);
    hasher.finish()
}

/// The checkpoints of the movie being played, and the seek under way.
#[derive(Debug, Clone)]
pub struct MovieTimeline {
    /// The core's frame count at the movie's start state.
    start_frame: i64,
    /// The movie's frames.
    length: u32,
    /// Core states (no ROM image) by core frame count, taken by the session; the start state
    /// is one of them.
    checkpoints: BTreeMap<i64, Vec<u8>>,
    /// Core states found by the explorer (`movie_explorer`), used once trusted.
    explored: BTreeMap<i64, Vec<u8>>,
    /// The [`machine_digest`] of each explored state, and of each checkpoint the session took.
    explored_digests: BTreeMap<i64, u64>,
    played_digests: BTreeMap<i64, u64>,
    /// Whether the explored states are trusted: unknown until one of them meets the session's
    /// own checkpoint of the same frame, then whether the two were equal.
    explored_trust: Option<bool>,
    /// Whether the frames before the warm ones may run with the core alone: not when a script
    /// changes the core or the inputs, since the core's past then depends on the scripts.
    core_only_allowed: bool,
    /// The core frame count a seek is heading for.
    seek_target: Option<i64>,
    /// Whether the seek under way is exact: from the movie's start, every frame with the
    /// scripts (a script reload whose new script must accumulate its own past).
    seek_exact: bool,
    /// Whether the exact seek under way went back to the start already: it then goes on from
    /// where it stands.
    seek_restarted: bool,
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
            explored: BTreeMap::new(),
            explored_digests: BTreeMap::new(),
            played_digests: BTreeMap::new(),
            explored_trust: None,
            core_only_allowed,
            seek_target: None,
            seek_exact: false,
            seek_restarted: false,
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

    /// Keeps `state`, taken by the session, as the checkpoint of `frame_count`, `digest` being
    /// its [`machine_digest`]. When the explored states are not judged yet and the explorer found
    /// the same frame, the two digests are compared: returns the verdict then (`Some(false)`
    /// drops every explored state).
    pub fn record(&mut self, frame_count: i64, state: Vec<u8>, digest: u64) -> Option<bool> {
        self.played_digests.insert(frame_count, digest);
        self.checkpoints.insert(frame_count, state);
        let explored = self.explored_digests.get(&frame_count).copied();
        self.judge(explored, Some(digest))
    }

    /// Keeps `state`, found by the explorer, for the frame `frame_count`, `digest` being its
    /// [`machine_digest`] (nothing once the explored states were found wrong). When the session
    /// already took that frame and the explored states are not judged yet, the two digests are
    /// compared: returns the verdict then.
    pub fn add_explored(&mut self, frame_count: i64, state: Vec<u8>, digest: u64) -> Option<bool> {
        if self.explored_trust == Some(false)
            || frame_count <= self.start_frame
            || frame_count >= self.end_frame()
        {
            return None;
        }
        self.explored.insert(frame_count, state);
        self.explored_digests.insert(frame_count, digest);
        let played = self.played_digests.get(&frame_count).copied();
        self.judge(Some(digest), played)
    }

    /// The verdict on the explored states when both digests of a frame are known and none was
    /// given yet.
    fn judge(&mut self, explored: Option<u64>, played: Option<u64>) -> Option<bool> {
        if self.explored_trust.is_some() {
            return None;
        }
        let trusted = explored? == played?;
        self.explored_trust = Some(trusted);
        if !trusted {
            self.explored.clear();
            self.explored_digests.clear();
        }
        Some(trusted)
    }

    /// Whether the explored states are trusted (`None`: not judged yet).
    #[must_use]
    pub const fn explored_trust(&self) -> Option<bool> {
        self.explored_trust
    }

    /// The explored states, when they may be used.
    fn usable_explored(&self) -> Option<&BTreeMap<i64, Vec<u8>>> {
        (self.explored_trust == Some(true)).then_some(&self.explored)
    }

    /// The furthest core frame count that has a usable checkpoint.
    #[must_use]
    pub fn furthest_checkpoint(&self) -> i64 {
        let played = self.checkpoints.keys().next_back().copied();
        let explored = self
            .usable_explored()
            .and_then(|explored| explored.keys().next_back().copied());
        played.max(explored).unwrap_or(self.start_frame)
    }

    /// The number of checkpoints, and the bytes they hold, explored ones included.
    #[must_use]
    pub fn footprint(&self) -> (usize, usize) {
        (
            self.checkpoints.len() + self.explored.len(),
            self.checkpoints
                .values()
                .chain(self.explored.values())
                .map(Vec::len)
                .sum(),
        )
    }

    /// The usable checkpoint at or before the core frame count `frame_count`.
    #[must_use]
    pub fn checkpoint_before(&self, frame_count: i64) -> Option<(i64, &[u8])> {
        let played = self.checkpoints.range(..=frame_count).next_back();
        let explored = self
            .usable_explored()
            .and_then(|explored| explored.range(..=frame_count).next_back());
        let best = match (played, explored) {
            (Some(played), Some(explored)) => Some(if explored.0 > played.0 {
                explored
            } else {
                played
            }),
            (played, explored) => played.or(explored),
        };
        best.map(|(&frame, state)| (frame, state.as_slice()))
    }

    /// Whether the frames before the warm ones may run with the core alone.
    #[must_use]
    pub const fn core_only_allowed(&self) -> bool {
        self.core_only_allowed
    }

    /// Starts a seek to the movie frame `frame`.
    pub fn seek(&mut self, frame: u32) {
        self.seek_target = Some(self.core_frame(frame));
        self.seek_exact = false;
        self.seek_restarted = false;
    }

    /// Starts an exact seek to the core frame count `frame_count`: from the movie's start,
    /// every frame with the scripts.
    pub fn seek_exactly(&mut self, frame_count: i64) {
        self.seek_target = Some(frame_count.clamp(self.start_frame, self.end_frame()));
        self.seek_exact = true;
        self.seek_restarted = false;
    }

    /// Whether the seek under way is exact.
    #[must_use]
    pub const fn seek_is_exact(&self) -> bool {
        self.seek_exact
    }

    /// Forgets every checkpoint but the start state: they were taken with scripts that changed
    /// the core, which the next ones may change otherwise.
    pub fn forget_checkpoints(&mut self) {
        let start = self.checkpoints.remove(&self.start_frame);
        self.checkpoints.clear();
        self.played_digests.clear();
        self.explored.clear();
        self.explored_digests.clear();
        self.explored_trust = None;
        if let Some(start) = start {
            self.checkpoints.insert(self.start_frame, start);
        }
    }

    /// The core frame count the seek under way is heading for.
    #[must_use]
    pub const fn seek_target(&self) -> Option<i64> {
        self.seek_target
    }

    /// The seek has arrived (or was given up).
    pub fn end_seek(&mut self) {
        self.seek_target = None;
        self.seek_exact = false;
        self.seek_restarted = false;
    }

    /// Where a seek to `target` starts from when the core stands at `current`: `None` to go on
    /// from there (it is on the way, and early enough to warm the scripts up, or the target is
    /// within the warm frames ahead), else the checkpoint to restore.
    #[must_use]
    pub fn seek_origin(&mut self, current: i64, target: i64) -> Option<(i64, &[u8])> {
        if self.seek_exact {
            if self.seek_restarted {
                return None;
            }
            self.seek_restarted = true;
            return self.checkpoint_before(self.start_frame);
        }
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
        timeline.record(60, vec![1], 1);
        assert!(!timeline.wants(60), "already there");
        assert!(!timeline.wants(1020), "after the movie");
        assert_eq!(timeline.furthest_checkpoint(), 60);
        assert_eq!(timeline.footprint(), (2, 2));
    }

    #[test]
    fn explored_states_count_once_one_meets_the_session_s_own() {
        let mut timeline = timeline();
        assert_eq!(timeline.add_explored(60, vec![1], 1), None);
        assert_eq!(timeline.add_explored(600, vec![6], 6), None);
        assert_eq!(
            timeline.checkpoint_before(700).map(|(f, _)| f),
            Some(3),
            "not judged yet"
        );
        assert_eq!(timeline.record(60, vec![1], 1), Some(true));
        assert_eq!(timeline.checkpoint_before(700).map(|(f, _)| f), Some(600));
        assert_eq!(timeline.furthest_checkpoint(), 600);
        assert_eq!(timeline.record(120, vec![2], 2), None, "judged once");

        // The other way round: the session took the frame before the explorer found it.
        let mut late = MovieTimeline::new(3, 1000, vec![0], true);
        late.record(60, vec![1], 1);
        assert_eq!(late.add_explored(60, vec![1], 1), Some(true));
        assert_eq!(late.explored_trust(), Some(true));

        let mut wrong = MovieTimeline::new(3, 1000, vec![0], true);
        assert_eq!(wrong.add_explored(60, vec![9], 9), None);
        assert_eq!(wrong.add_explored(600, vec![6], 6), None);
        assert_eq!(wrong.record(60, vec![1], 1), Some(false));
        assert_eq!(wrong.checkpoint_before(700).map(|(f, _)| f), Some(60));
        assert_eq!(wrong.add_explored(660, vec![7], 7), None);
        assert_eq!(
            wrong.footprint().0,
            2,
            "nothing explored is kept after a mismatch"
        );
    }

    #[test]
    fn an_exact_seek_starts_from_the_movie_s_start() {
        let mut timeline = timeline();
        timeline.record(60, vec![1], 1);
        timeline.record(120, vec![2], 2);
        timeline.seek_exactly(150);
        assert!(timeline.seek_is_exact());
        assert_eq!(timeline.seek_origin(140, 150).map(|(f, _)| f), Some(3));
        assert_eq!(timeline.seek_origin(20, 150), None, "it goes on from there");
        timeline.end_seek();
        assert!(!timeline.seek_is_exact());
        timeline.forget_checkpoints();
        assert_eq!(timeline.footprint().0, 1);
        assert_eq!(timeline.furthest_checkpoint(), 3);
    }

    #[test]
    fn a_seek_starts_from_the_checkpoint_warm_frames_before_or_goes_on() {
        let mut timeline = timeline();
        timeline.record(60, vec![1], 1);
        timeline.record(120, vec![2], 2);
        timeline.record(180, vec![3], 3);
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
