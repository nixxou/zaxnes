//! Pipelined frames (Rust-only, owner request, setting `emu.pipeline`): the scripts and the
//! conversion of a frame, then its draw list, run on a thread of their own while the main thread
//! draws the frame before it. The picture is one displayed frame late; what is drawn is the same.
//!
//! It needs the optimised Lua (`emu.lua_optimized`): with it off, nothing is pipelined.
//!
//! "Auto" pipelines only while the frames are late, and goes back to the direct path once the
//! work fits again, with a margin between the two thresholds so that it does not flicker
//! between them. Each change shows once: entering draws the last frame again, leaving skips one.

use std::time::Duration;

use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

/// The choices of `emu.pipeline`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineMode {
    /// Never pipelined.
    Off,
    /// Pipelined while the frames are late.
    Auto,
    /// Always pipelined when the game allows it.
    Always,
}

impl PipelineMode {
    /// The choice in force: `ZAXNES_PIPELINE` (`0`, `1` or `2`) when set, else the one
    /// `settings` holds; always off without the optimised Lua (`emu.lua_optimized`), since a
    /// pipelined frame hands the Lua runtime to another thread.
    #[must_use]
    pub fn of(settings: &SettingsModel) -> Self {
        if !settings.get(SettingId::EmuLuaOptimized).as_bool() {
            return Self::Off;
        }
        let forced = std::env::var(ENVIRONMENT_VARIABLE)
            .ok()
            .and_then(|value| value.trim().parse::<i32>().ok());
        Self::from_index(forced.unwrap_or_else(|| settings.get(SettingId::EmuPipeline).as_int()))
    }

    const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Auto,
            2 => Self::Always,
            _ => Self::Off,
        }
    }
}

/// The variable that forces [`PipelineMode::of`].
const ENVIRONMENT_VARIABLE: &str = "ZAXNES_PIPELINE";

/// Share of the frame budget above which a direct frame counts as late.
const LATE_SHARE: f64 = 0.85;

/// Late direct frames in a row that start the pipeline.
const LATE_FRAMES: u32 = 3;

/// Share of the frame budget under which a pipelined frame would fit the direct path.
const LIGHT_SHARE: f64 = 0.6;

/// Light pipelined frames in a row that end the pipeline (two seconds at 60 Hz).
const LIGHT_FRAMES: u32 = 120;

/// Decides, frame after frame, whether the next one is pipelined.
#[derive(Debug, Clone, Default)]
pub struct PipelineGovernor {
    pipelined: bool,
    late: u32,
    light: u32,
}

impl PipelineGovernor {
    /// Whether the next frame is pipelined, under `mode` and when the game `allows` it (the
    /// scripts leave the core alone, the game runs in 3D and no window is modal).
    pub fn next_frame(&mut self, mode: PipelineMode, allows: bool) -> bool {
        let wanted = match mode {
            PipelineMode::Off => false,
            PipelineMode::Always => true,
            PipelineMode::Auto => self.pipelined,
        };
        if !allows || mode == PipelineMode::Off {
            self.pipelined = false;
            self.late = 0;
            self.light = 0;
        }
        wanted && allows
    }

    /// Records a frame of the main thread: `busy` before the swap, of which `overlap` would have
    /// been added to it on the direct path (the worker's time that ran beside the drawing), for
    /// a frame `budget`. Only "Auto" uses it.
    pub fn record(&mut self, pipelined: bool, busy: Duration, overlap: Duration, budget: Duration) {
        let budget = budget.as_secs_f64();
        if pipelined {
            let direct = (busy + overlap).as_secs_f64();
            self.light = if direct < budget * LIGHT_SHARE {
                self.light + 1
            } else {
                0
            };
            if self.light >= LIGHT_FRAMES {
                self.pipelined = false;
                self.light = 0;
            }
        } else {
            self.late = if busy.as_secs_f64() > budget * LATE_SHARE {
                self.late + 1
            } else {
                0
            };
            if self.late >= LATE_FRAMES {
                self.pipelined = true;
                self.late = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGET: Duration = Duration::from_micros(16_667);

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn off_and_always_ignore_the_timings() {
        let mut governor = PipelineGovernor::default();
        for _ in 0..10 {
            governor.record(false, ms(30), Duration::ZERO, BUDGET);
        }
        assert!(!governor.next_frame(PipelineMode::Off, true));
        assert!(governor.next_frame(PipelineMode::Always, true));
        assert!(!governor.next_frame(PipelineMode::Always, false));
    }

    #[test]
    fn auto_starts_after_three_late_frames_in_a_row() {
        let mut governor = PipelineGovernor::default();
        governor.record(false, ms(16), Duration::ZERO, BUDGET);
        governor.record(false, ms(16), Duration::ZERO, BUDGET);
        governor.record(false, ms(5), Duration::ZERO, BUDGET);
        governor.record(false, ms(16), Duration::ZERO, BUDGET);
        assert!(!governor.next_frame(PipelineMode::Auto, true));
        governor.record(false, ms(16), Duration::ZERO, BUDGET);
        governor.record(false, ms(16), Duration::ZERO, BUDGET);
        assert!(governor.next_frame(PipelineMode::Auto, true));
    }

    #[test]
    fn auto_stops_once_the_direct_path_fits_for_two_seconds() {
        let mut governor = PipelineGovernor::default();
        for _ in 0..LATE_FRAMES {
            governor.record(false, ms(20), Duration::ZERO, BUDGET);
        }
        assert!(governor.next_frame(PipelineMode::Auto, true));
        // Busy 6 ms plus 6 ms of overlap would be 12 ms on the direct path: too much.
        for _ in 0..LIGHT_FRAMES * 2 {
            governor.record(true, ms(6), ms(6), BUDGET);
        }
        assert!(governor.next_frame(PipelineMode::Auto, true));
        for _ in 0..LIGHT_FRAMES - 1 {
            governor.record(true, ms(5), ms(3), BUDGET);
        }
        assert!(governor.next_frame(PipelineMode::Auto, true));
        governor.record(true, ms(5), ms(3), BUDGET);
        assert!(!governor.next_frame(PipelineMode::Auto, true));
    }

    #[test]
    fn a_frame_the_game_does_not_allow_resets_auto() {
        let mut governor = PipelineGovernor::default();
        for _ in 0..LATE_FRAMES {
            governor.record(false, ms(20), Duration::ZERO, BUDGET);
        }
        assert!(!governor.next_frame(PipelineMode::Auto, false));
        assert!(!governor.next_frame(PipelineMode::Auto, true));
    }
}
