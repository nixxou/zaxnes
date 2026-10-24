//! The built-in frame profiler (Rust-only, not in the C# application): with
//! `ZAXNES_PROFILE=1` the loop times each stage of a displayed frame, and every two seconds the
//! mean and the maximum of each stage go to the log, with the frame rate achieved. The same
//! figures, shortened, are shown next to the FPS counter.
//!
//! The stages overlap when the frame is pipelined (the emulation and the conversion run on
//! threads of their own), so a second line gives how long each thread was busy per displayed
//! frame, and how many frames were pipelined.
//!
//! The per-frame path allocates nothing: the stages are summed in fixed arrays, and only the
//! report of each two-second window builds its strings.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use zax_scripting::HookTime;

/// The environment variable that turns the profiler on (`1`).
pub const ENVIRONMENT_VARIABLE: &str = "ZAXNES_PROFILE";

/// How long a report window lasts.
pub const REPORT_INTERVAL: Duration = Duration::from_secs(2);

/// Whether the environment asks for the profiler.
#[must_use]
pub fn requested_by_environment() -> bool {
    std::env::var(ENVIRONMENT_VARIABLE).is_ok_and(|value| value.trim() == "1")
}

/// A stage of the displayed frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Window events, input sampling, camera and cursor.
    Input,
    /// `NesCore::run_frame` of the emulated frames.
    Core,
    /// The copy of the core's tile records (the frame batch).
    Capture,
    /// The conversion of the batch (zax-engine3d), the Lua frame hooks excepted.
    Conversion,
    /// The Lua hooks: input process, the four frame hooks, camera tracking.
    Lua,
    /// The rollback snapshots (a savestate every 60 emulated frames, REQ-LOOP-030).
    Rollback,
    /// The rest of the host's tick: commands, pacing, settings, publication.
    Emulation,
    /// The engine's draw list of the frame (`GameSession::build_draw_list`).
    DrawList,
    /// The renderer's collection of the tiles into its buckets (meshes, textures, matrices).
    Collect,
    /// The scene: update and the renderer's passes (GL calls, CPU side).
    Scene,
    /// Pipelined frames: the main thread waiting for the conversion thread, which runs the
    /// conversion, the Lua and the draw list stages beside the drawing.
    Wait,
    /// The blit of the scene and the interface (Dear `ImGui` build and render).
    Ui,
    /// The audio pump.
    Audio,
    /// The buffer swap, where a vsync wait shows.
    Swap,
    /// The frame cap's sleep (only without vsync).
    Pacer,
    /// What no stage covers.
    Other,
}

impl Stage {
    /// Number of stages.
    pub const COUNT: usize = 16;

    /// Every stage, in frame order.
    pub const ALL: [Self; Self::COUNT] = [
        Self::Input,
        Self::Core,
        Self::Capture,
        Self::Conversion,
        Self::Lua,
        Self::Rollback,
        Self::Emulation,
        Self::DrawList,
        Self::Collect,
        Self::Scene,
        Self::Wait,
        Self::Ui,
        Self::Audio,
        Self::Swap,
        Self::Pacer,
        Self::Other,
    ];

    /// The short name the reports use.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Core => "core",
            Self::Capture => "capture",
            Self::Conversion => "conv",
            Self::Lua => "lua",
            Self::Rollback => "rollback",
            Self::Emulation => "host",
            Self::DrawList => "list",
            Self::Collect => "collect",
            Self::Scene => "scene",
            Self::Wait => "wait",
            Self::Ui => "ui",
            Self::Audio => "audio",
            Self::Swap => "swap",
            Self::Pacer => "sleep",
            Self::Other => "other",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// A thread of the frame loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thread {
    /// The main thread, from the start of the frame to the swap (its waits for the conversion
    /// included).
    Main,
    /// The conversion thread of a pipelined frame: conversion, Lua frame hooks, draw list.
    Conversion,
    /// The emulation thread ("emulation ahead"): the core's frames run there.
    Emulation,
}

impl Thread {
    /// Number of threads.
    pub const COUNT: usize = 3;

    /// Every thread.
    pub const ALL: [Self; Self::COUNT] = [Self::Main, Self::Conversion, Self::Emulation];

    /// The short name the reports use.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Conversion => "conversion",
            Self::Emulation => "emulation",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// Time spent in each stage, how long each thread was busy, and how many frames were pipelined.
/// The thread times are apart from the stages: they overlap them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StageTimes {
    stages: [Duration; Stage::COUNT],
    threads: [Duration; Thread::COUNT],
    pipelined: u32,
}

impl StageTimes {
    /// Adds `duration` to `stage`.
    pub fn add(&mut self, stage: Stage, duration: Duration) {
        self.stages[stage.index()] += duration;
    }

    /// The time of `stage`.
    #[must_use]
    pub const fn get(&self, stage: Stage) -> Duration {
        self.stages[stage.index()]
    }

    /// Adds `duration` to the busy time of `thread`.
    pub fn add_thread(&mut self, thread: Thread, duration: Duration) {
        self.threads[thread.index()] += duration;
    }

    /// The busy time of `thread`.
    #[must_use]
    pub const fn thread(&self, thread: Thread) -> Duration {
        self.threads[thread.index()]
    }

    /// Counts one pipelined frame.
    pub const fn count_pipelined(&mut self) {
        self.pipelined += 1;
    }

    /// Adds every stage, thread time and pipelined frame of `other`.
    pub fn merge(&mut self, other: &Self) {
        for (mine, theirs) in self.stages.iter_mut().zip(other.stages) {
            *mine += theirs;
        }
        for (mine, theirs) in self.threads.iter_mut().zip(other.threads) {
            *mine += theirs;
        }
        self.pipelined += other.pipelined;
    }

    /// The sum of every stage.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.stages.iter().sum()
    }
}

/// Times stages when it is on, and costs one branch per stage when it is off.
#[derive(Debug, Clone, Default)]
pub struct StageClock {
    enabled: bool,
    times: StageTimes,
}

impl StageClock {
    /// A clock that times only when `enabled`.
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            times: StageTimes::default(),
        }
    }

    /// Whether the clock times.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Turns the clock on or off.
    pub const fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// The start of a stage: `None` when the clock is off.
    #[must_use]
    pub fn start(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }

    /// Ends `stage` begun at `started`.
    pub fn stop(&mut self, stage: Stage, started: Option<Instant>) {
        if let Some(started) = started {
            self.times.add(stage, started.elapsed());
        }
    }

    /// Adds `duration` to `stage` (when it is on).
    pub fn add(&mut self, stage: Stage, duration: Duration) {
        if self.enabled {
            self.times.add(stage, duration);
        }
    }

    /// Adds `duration` to the busy time of `thread` (when it is on).
    pub fn add_thread(&mut self, thread: Thread, duration: Duration) {
        if self.enabled {
            self.times.add_thread(thread, duration);
        }
    }

    /// Counts the frame as pipelined (when it is on).
    pub const fn count_pipelined(&mut self) {
        if self.enabled {
            self.times.count_pipelined();
        }
    }

    /// Adds the stage times another clock took (when it is on).
    pub fn merge(&mut self, times: &StageTimes) {
        if self.enabled {
            self.times.merge(times);
        }
    }

    /// The times so far, and a fresh start.
    pub fn take(&mut self) -> StageTimes {
        std::mem::take(&mut self.times)
    }
}

/// The sum and the maximum of one stage over a window.
#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    sum: Duration,
    max: Duration,
}

impl Tally {
    fn add(&mut self, duration: Duration) {
        self.sum += duration;
        self.max = self.max.max(duration);
    }

    fn mean_ms(self, frames: u32) -> f64 {
        self.sum.as_secs_f64() * 1000.0 / f64::from(frames.max(1))
    }

    fn max_ms(self) -> f64 {
        self.max.as_secs_f64() * 1000.0
    }
}

/// Stages whose mean stays under this many milliseconds are left out of the on-screen line.
const SHOWN_MINIMUM_MS: f64 = 0.05;

/// The two-second windows of the profiler.
#[derive(Debug)]
pub struct FrameProfiler {
    stages: [Tally; Stage::COUNT],
    threads: [Tally; Thread::COUNT],
    pipelined: u32,
    frame: Tally,
    frames: u32,
    emulated: u32,
    window_start: Instant,
    summary: String,
    thread_report: String,
    reported_frames: u32,
}

impl FrameProfiler {
    /// A profiler whose first window starts at `now`.
    #[must_use]
    pub fn new(now: Instant) -> Self {
        Self {
            stages: [Tally::default(); Stage::COUNT],
            threads: [Tally::default(); Thread::COUNT],
            pipelined: 0,
            frame: Tally::default(),
            frames: 0,
            emulated: 0,
            window_start: now,
            summary: String::new(),
            thread_report: String::new(),
            reported_frames: 0,
        }
    }

    /// The thread line of the last report: the busy time of each thread per displayed frame,
    /// and the share of pipelined frames (empty before the first report).
    #[must_use]
    pub fn thread_report(&self) -> &str {
        &self.thread_report
    }

    /// The displayed frames the last report covered.
    #[must_use]
    pub const fn reported_frames(&self) -> u32 {
        self.reported_frames
    }

    /// The short line shown next to the FPS counter (empty before the first report).
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// Adds one displayed frame that lasted `frame`, ran `emulated` emulated frames and whose
    /// stages took `times`; what no stage covers is counted as [`Stage::Other`]. Returns the
    /// report line when the window is over.
    ///
    /// The emulated frames per displayed frame show the catch-up of the frame clock: when a
    /// displayed frame takes longer than an emulated one, the next one runs several (up to five,
    /// REQ-LOOP-012), which makes it longer still.
    pub fn end_frame(
        &mut self,
        times: &StageTimes,
        frame: Duration,
        emulated: u32,
        now: Instant,
    ) -> Option<String> {
        self.emulated += emulated;
        let other = frame.saturating_sub(times.total());
        for stage in Stage::ALL {
            let duration = if stage == Stage::Other {
                times.get(stage) + other
            } else {
                times.get(stage)
            };
            self.stages[stage.index()].add(duration);
        }
        for thread in Thread::ALL {
            self.threads[thread.index()].add(times.thread(thread));
        }
        self.pipelined += times.pipelined;
        self.frame.add(frame);
        self.frames += 1;
        let elapsed = now.duration_since(self.window_start);
        if elapsed < REPORT_INTERVAL {
            return None;
        }
        let report = self.report(elapsed);
        self.stages = [Tally::default(); Stage::COUNT];
        self.threads = [Tally::default(); Thread::COUNT];
        self.pipelined = 0;
        self.frame = Tally::default();
        self.frames = 0;
        self.emulated = 0;
        self.window_start = now;
        Some(report)
    }

    fn report(&mut self, elapsed: Duration) -> String {
        let frames = self.frames;
        let fps = f64::from(frames) / elapsed.as_secs_f64().max(f64::EPSILON);
        let seconds = elapsed.as_secs_f64().max(f64::EPSILON);
        let emulated_rate = f64::from(self.emulated) / seconds;
        let per_frame = f64::from(self.emulated) / f64::from(frames.max(1));
        let mut line = format!(
            "profile: {fps:.1} fps over {frames} frames, emulated {emulated_rate:.1}/s \
             ({per_frame:.2} per displayed frame), frame {:.2}/{:.2} ms (mean/max)",
            self.frame.mean_ms(frames),
            self.frame.max_ms()
        );
        self.summary.clear();
        let _ = write!(self.summary, "{fps:.0} fps, emu {emulated_rate:.0}/s |");
        for stage in Stage::ALL {
            let tally = self.stages[stage.index()];
            let mean = tally.mean_ms(frames);
            let _ = write!(line, " | {} {mean:.2}/{:.2}", stage.label(), tally.max_ms());
            if mean >= SHOWN_MINIMUM_MS {
                let _ = write!(self.summary, " {} {mean:.1}", stage.label());
            }
        }
        self.summary.push_str(" ms");

        self.reported_frames = frames;
        self.thread_report.clear();
        let _ = write!(
            self.thread_report,
            "profile threads: pipelined {} of {frames} frames | busy per displayed frame \
             (mean/max ms)",
            self.pipelined
        );
        for thread in Thread::ALL {
            let tally = self.threads[thread.index()];
            let _ = write!(
                self.thread_report,
                " | {} {:.2}/{:.2}",
                thread.label(),
                tally.mean_ms(frames),
                tally.max_ms()
            );
        }
        let wait = self.stages[Stage::Wait.index()];
        let _ = write!(
            self.thread_report,
            " (main waits {:.2}/{:.2} of it)",
            wait.mean_ms(frames),
            wait.max_ms()
        );
        line
    }
}

/// Hooks a Lua line lists at most, the longest first.
const LUA_REPORT_HOOKS: usize = 8;

/// The Lua line of a report: the time of the longest hooks per displayed frame over the
/// `frames` of the window, with their number of calls, and the total of every hook.
#[must_use]
pub fn lua_report(hooks: &[HookTime], frames: u32) -> String {
    let per_frame = |time: Duration| time.as_secs_f64() * 1000.0 / f64::from(frames.max(1));
    let total: Duration = hooks.iter().map(|hook| hook.time).sum();
    let mut line = format!(
        "profile lua: {:.2} ms per displayed frame in the hooks (script.hook ms calls)",
        per_frame(total)
    );
    for hook in hooks.iter().take(LUA_REPORT_HOOKS) {
        let _ = write!(
            line,
            " | {}.{} {:.2} {}",
            hook.script,
            hook.hook.name(),
            per_frame(hook.time),
            hook.calls
        );
    }
    if hooks.len() > LUA_REPORT_HOOKS {
        let _ = write!(line, " | {} more", hooks.len() - LUA_REPORT_HOOKS);
    }
    line
}

/// The `target-cpu` the binary was built for, as the build script found it in the flags.
#[must_use]
pub const fn target_cpu() -> &'static str {
    match option_env!("ZAXNES_TARGET_CPU") {
        Some(cpu) => cpu,
        None => "generic",
    }
}

/// The start-up line of the profiler: the machine, the build and the render settings in force.
#[must_use]
pub fn startup_report(render: &str, pacing: &str) -> String {
    let cpus = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    format!(
        "profile: {cpus} CPUs ({}), {} build, {} {}, target-cpu {}; {render}; pacing: {pacing}",
        cpu_frequency_report(),
        build_profile(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        target_cpu()
    )
}

/// The first CPU's frequency governor and its current and maximum frequencies, as Linux reports
/// them (a handheld left on a power-saving governor runs at a fraction of its speed).
fn cpu_frequency_report() -> String {
    let folder = std::path::Path::new("/sys/devices/system/cpu/cpu0/cpufreq");
    let read = |name: &str| {
        std::fs::read_to_string(folder.join(name))
            .map_or_else(|_| "?".to_owned(), |text| text.trim().to_owned())
    };
    if !folder.is_dir() {
        return "no cpufreq".to_owned();
    }
    format!(
        "governor {}, {} kHz now, {} kHz max of {} kHz",
        read("scaling_governor"),
        read("scaling_cur_freq"),
        read("scaling_max_freq"),
        read("cpuinfo_max_freq")
    )
}

/// The build profile of the binary.
#[must_use]
pub const fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn a_stopped_clock_records_nothing() {
        let mut clock = StageClock::new(false);
        let started = clock.start();
        assert!(started.is_none());
        clock.stop(Stage::Core, started);
        clock.add(Stage::Lua, ms(3));
        assert_eq!(StageTimes::default(), clock.take());
    }

    #[test]
    fn a_running_clock_sums_its_stages_and_starts_afresh_when_taken() {
        let mut clock = StageClock::new(true);
        clock.add(Stage::Core, ms(2));
        clock.add(Stage::Core, ms(3));
        let started = clock.start();
        clock.stop(Stage::Scene, started);
        let times = clock.take();
        assert_eq!(ms(5), times.get(Stage::Core));
        assert_eq!(Duration::ZERO, clock.take().get(Stage::Core));
    }

    #[test]
    fn the_report_comes_after_two_seconds_with_means_maxima_and_the_rate() {
        let start = Instant::now();
        let mut profiler = FrameProfiler::new(start);
        let mut times = StageTimes::default();
        times.add(Stage::Core, ms(10));
        times.add(Stage::Swap, ms(4));
        let mut now = start;
        let mut report = None;
        for frame in 0..120 {
            now += ms(20);
            let mut this = times;
            if frame == 7 {
                this.add(Stage::Core, ms(6));
            }
            report = profiler.end_frame(&this, ms(20), 1 + u32::from(frame % 5 == 0), now);
            if frame < 99 {
                assert!(report.is_none(), "frame {frame} reported early");
            }
            if report.is_some() {
                break;
            }
        }
        let report = report.expect("a report after two seconds");
        assert!(report.contains("50.0 fps over 100 frames"), "{report}");
        assert!(
            report.contains("emulated 60.0/s (1.20 per displayed frame)"),
            "{report}"
        );
        assert!(report.contains("core 10.06/16.00"), "{report}");
        assert!(report.contains("other 5.94/6.00"), "{report}");
        assert!(report.contains("frame 20.00/20.00"), "{report}");
        assert_eq!(
            "50 fps, emu 60/s | core 10.1 swap 4.0 other 5.9 ms",
            profiler.summary()
        );
    }

    #[test]
    fn the_thread_line_gives_the_busy_times_and_the_pipelined_frames() {
        let start = Instant::now();
        let mut profiler = FrameProfiler::new(start);
        let mut now = start;
        for frame in 0..100 {
            now += ms(20);
            let mut times = StageTimes::default();
            times.add_thread(Thread::Main, ms(12));
            times.add_thread(Thread::Conversion, ms(9));
            times.add(Stage::Wait, ms(3));
            if frame % 2 == 0 {
                times.count_pipelined();
                times.add_thread(Thread::Emulation, ms(10));
            }
            if profiler.end_frame(&times, ms(20), 1, now).is_some() {
                break;
            }
        }
        let line = profiler.thread_report();
        assert!(line.contains("pipelined 50 of 100 frames"), "{line}");
        assert!(line.contains("main 12.00/12.00"), "{line}");
        assert!(line.contains("conversion 9.00/9.00"), "{line}");
        assert!(line.contains("emulation 5.00/10.00"), "{line}");
        assert!(line.contains("main waits 3.00/3.00"), "{line}");
        assert_eq!(100, profiler.reported_frames());
    }

    #[test]
    fn the_lua_line_lists_the_longest_hooks_per_displayed_frame() {
        let hook = |script: &str, hook, time, calls| HookTime {
            script: script.to_owned(),
            hook,
            time: ms(time),
            calls,
        };
        let hooks = [
            hook("Enemies", zax_scripting::ScriptHook::UpdateS, 600, 4000),
            hook("Camera", zax_scripting::ScriptHook::LateUpdate, 100, 100),
        ];
        let line = lua_report(&hooks, 100);
        assert_eq!(
            "profile lua: 7.00 ms per displayed frame in the hooks (script.hook ms calls) \
             | Enemies.UpdateS 6.00 4000 | Camera.LateUpdate 1.00 100",
            line
        );
    }

    #[test]
    fn the_stage_times_merge_and_add_up() {
        let mut first = StageTimes::default();
        first.add(Stage::Input, ms(1));
        let mut second = StageTimes::default();
        second.add(Stage::Input, ms(2));
        second.add(Stage::Ui, ms(3));
        first.merge(&second);
        assert_eq!(ms(3), first.get(Stage::Input));
        assert_eq!(ms(6), first.total());
    }
}
