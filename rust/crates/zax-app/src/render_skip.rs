//! The render-skip rule of SPEC-06 REQ-SCN-060, kept apart from the loop so that it can be
//! checked on its own.
//!
//! The camera is deliberately not part of the decision: the criterion of REQ-SCN-060 expects a
//! skipping host to redraw at the emulated rate and no more, which is also what makes the orbit
//! smoother once the setting is turned off.

/// Whether the scene may keep the previous image for this refresh: the user setting `enabled`
/// (true by default), a game is running (REQ-SCN-047), no emulated frame was produced since the
/// last refresh, and the render target is unchanged (same size, and something has been drawn into
/// it).
#[must_use]
pub const fn should_skip(
    enabled: bool,
    game_running: bool,
    emulated_frames: u32,
    target_unchanged: bool,
) -> bool {
    enabled && game_running && emulated_frames == 0 && target_unchanged
}
