//! The display pacing of SPEC-06 REQ-SCN-061: `video.vsync` (SPEC-09, 0 off, 1 every refresh,
//! 2 every other refresh) and `video.frame_rate` (60, 90 or 120) applied to a window, at
//! start-up and again whenever either setting changes.
//!
//! With vsync on the driver paces the swaps, so the cap is left off; with vsync off the cap is
//! what keeps the loop at the rate the user asked for. The rule is testable without a GL
//! context: [`crate::HostWindow`] is the real [`DisplaySurface`], the tests pass a recording
//! double.

use std::fmt;

/// Highest `video.vsync` value SPEC-09 defines.
pub const MAXIMUM_VSYNC: i32 = 2;

/// What the display pacing needs to drive on a window: the buffer-swap synchronisation and the
/// frame cap of the loop.
pub trait DisplaySurface {
    /// Turns the swap synchronisation on or off, which is a swap interval of 1 or 0.
    fn set_vsync(&mut self, enabled: bool);

    /// Caps the render loop at `frames_per_second`, or 0 for "as fast as the machine allows"
    /// (REQ-SCN-061).
    fn set_frame_cap(&mut self, frames_per_second: u32);

    /// Asks the GL context for an exact swap interval, which is the only way to express
    /// `video.vsync` = 2 — one refresh out of two. False means the backend cannot (Q-066) and
    /// the caller falls back.
    fn try_set_swap_interval(&mut self, interval: i32) -> bool;
}

/// What [`apply`] actually put in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayPacingState {
    /// The `video.vsync` value that was asked for: 0, 1 or 2.
    pub vsync: i32,
    /// The swap interval in force: refreshes between two swaps.
    pub swap_interval: i32,
    /// The frame cap in force, 0 when the driver paces the swaps.
    pub frame_cap: u32,
    /// Whether `video.vsync` = 2 had to fall back to every refresh because the backend has no
    /// swap-interval call (Q-066).
    pub fell_back_to_every_refresh: bool,
}

impl fmt::Display for DisplayPacingState {
    /// One log line describing what is in force.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "swap interval {} (video.vsync {}{}), frame cap ",
            self.swap_interval,
            self.vsync,
            if self.fell_back_to_every_refresh {
                ", every other refresh unavailable, Q-066"
            } else {
                ""
            }
        )?;
        if self.frame_cap > 0 {
            write!(f, "{} fps", self.frame_cap)
        } else {
            f.write_str("off")
        }
    }
}

/// Puts `vsync` and `target_frame_rate` in force on `surface`.
///
/// `vsync` is the `video.vsync` value, anything outside 0…2 is clamped; `target_frame_rate` is
/// the cap to use when vsync is off, 0 (or less) for uncapped.
pub fn apply(
    surface: &mut dyn DisplaySurface,
    vsync: i32,
    target_frame_rate: i32,
) -> DisplayPacingState {
    let requested = vsync.clamp(0, MAXIMUM_VSYNC);
    if requested == 0 {
        surface.set_vsync(false);

        let cap = u32::try_from(target_frame_rate).unwrap_or(0);
        surface.set_frame_cap(cap);
        return DisplayPacingState {
            vsync: requested,
            swap_interval: 0,
            frame_cap: cap,
            fell_back_to_every_refresh: false,
        };
    }

    // Every refresh first, through the windowing layer, so that its own idea of vsync stays
    // true — a window-state change re-applies it. Then the exact interval for the value 2.
    surface.set_vsync(true);

    // Q-066: without a swap-interval call there is no way to ask for one refresh out of two,
    // and the documented fallback is every refresh.
    let fell_back = requested == MAXIMUM_VSYNC && !surface.try_set_swap_interval(MAXIMUM_VSYNC);
    surface.set_frame_cap(0);

    DisplayPacingState {
        vsync: requested,
        swap_interval: if fell_back { 1 } else { requested },
        frame_cap: 0,
        fell_back_to_every_refresh: fell_back,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostWindowSettings;

    /// A window that records what the pacing asks of it.
    struct FakeDisplaySurface {
        swap_interval_supported: bool,
        swap_interval: i32,
        frame_cap: u32,
    }

    impl FakeDisplaySurface {
        fn new() -> Self {
            Self {
                swap_interval_supported: true,
                swap_interval: 0,
                frame_cap: 0,
            }
        }
    }

    impl DisplaySurface for FakeDisplaySurface {
        fn set_vsync(&mut self, enabled: bool) {
            self.swap_interval = i32::from(enabled);
        }

        fn set_frame_cap(&mut self, frames_per_second: u32) {
            self.frame_cap = frames_per_second;
        }

        fn try_set_swap_interval(&mut self, interval: i32) -> bool {
            if !self.swap_interval_supported {
                return false;
            }
            self.swap_interval = interval;
            true
        }
    }

    /// REQ-SCN-061 with SPEC-09: each value of `video.vsync` has its swap interval, and the
    /// frame cap only exists when the driver is not pacing the swaps.
    #[test]
    fn req_scn_061_each_vsync_value_sets_its_swap_interval() {
        let mut surface = FakeDisplaySurface::new();

        let off = apply(&mut surface, 0, 60);
        assert_eq!(surface.swap_interval, 0);
        assert_eq!(surface.frame_cap, 60);
        assert_eq!(
            off,
            DisplayPacingState {
                vsync: 0,
                swap_interval: 0,
                frame_cap: 60,
                fell_back_to_every_refresh: false
            }
        );

        let every = apply(&mut surface, 1, 60);
        assert_eq!(surface.swap_interval, 1);
        assert_eq!(surface.frame_cap, 0);
        assert_eq!(
            every,
            DisplayPacingState {
                vsync: 1,
                swap_interval: 1,
                frame_cap: 0,
                fell_back_to_every_refresh: false
            }
        );

        let other = apply(&mut surface, 2, 120);
        assert_eq!(surface.swap_interval, 2);
        assert_eq!(surface.frame_cap, 0);
        assert_eq!(
            other,
            DisplayPacingState {
                vsync: 2,
                swap_interval: 2,
                frame_cap: 0,
                fell_back_to_every_refresh: false
            }
        );

        // Out of domain is clamped, and an uncapped request stays uncapped.
        assert_eq!(apply(&mut surface, 9, 0).swap_interval, 2);
        assert_eq!(apply(&mut surface, 0, 0).frame_cap, 0);
    }

    /// Q-066: with no swap-interval call there is no way to ask for one refresh out of two, and
    /// the documented fallback is every refresh — never "no synchronisation at all".
    #[test]
    fn q_066_vsync_2_falls_back_to_every_refresh_without_a_swap_interval_call() {
        let mut surface = FakeDisplaySurface::new();
        surface.swap_interval_supported = false;

        let state = apply(&mut surface, 2, 90);

        assert!(state.fell_back_to_every_refresh);
        assert_eq!(state.swap_interval, 1);
        assert_eq!(surface.swap_interval, 1);
        assert_eq!(surface.frame_cap, 0);
        assert!(state.to_string().contains("Q-066"));
    }

    #[test]
    fn describe_names_the_cap() {
        let mut surface = FakeDisplaySurface::new();
        assert_eq!(
            apply(&mut surface, 0, 90).to_string(),
            "swap interval 0 (video.vsync 0), frame cap 90 fps"
        );
        assert_eq!(
            apply(&mut surface, 1, 90).to_string(),
            "swap interval 1 (video.vsync 1), frame cap off"
        );
    }

    /// REQ-SCN-061: the window is opened on the persisted values, not on the command-line
    /// defaults — the swap interval of the first frame is already the right one.
    #[test]
    fn req_scn_061_the_window_opens_on_the_settings_values() {
        let settings = HostWindowSettings {
            vsync: 2,
            target_frame_rate: 120,
            ..HostWindowSettings::default()
        };

        assert!(settings.is_vsync_enabled());
        assert_eq!(settings.vsync, 2);
        assert_eq!(settings.target_frame_rate, 120);
        let off = HostWindowSettings {
            vsync: 0,
            ..HostWindowSettings::default()
        };
        assert!(!off.is_vsync_enabled());
    }
}
