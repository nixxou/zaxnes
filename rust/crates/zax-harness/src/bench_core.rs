//! `bench-core`: runs a ROM headless and reports what each stage of an emulated frame costs on
//! the CPU, as the application chains them (core, tile-record capture, conversion, Lua hooks,
//! publication, draw list, picture copy). No display is needed, so it runs where the device
//! cannot be profiled (a cross build under `qemu-aarch64` gives the relative weights).

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use zax_engine3d::settings::RenderAlgorithm;
use zax_nes::FrameBatch;

use crate::render::input_script::InputScript;
use crate::render::profile;
use crate::render::rom::{Host, Startup};
use crate::render::timed_host::TimedHost;
use crate::HarnessError;

/// The options of `bench-core`.
pub const USAGE: &str = "zax-harness bench-core <rom.nes> [options]
  --frames <n>             emulated frames to time (default 1200)
  --warmup <n>             frames run first and not timed (default 120)
  --profile <title|file>   profile to play (default: none, the \"2D Mode\" path)
  --assets <dir>           folder holding profiles.json (profile lookup)
  --press <script>         port 1 buttons, as render --press (e.g. 60-70:start,200-900:right)
  --3d                     segment as a 3D profile does even without patterns";

/// What `bench-core` runs.
#[derive(Debug, Clone)]
pub struct Options {
    rom: PathBuf,
    frames: u32,
    warmup: u32,
    profile: Option<String>,
    assets: Option<PathBuf>,
    press: InputScript,
    three_dimensional: bool,
}

impl Options {
    /// Parses the arguments after `bench-core`.
    ///
    /// # Errors
    ///
    /// [`HarnessError::Usage`] when an option is unknown, lacks its value or a count is not a
    /// number.
    pub fn parse(arguments: &[String]) -> Result<Self, HarnessError> {
        let usage = || HarnessError::Usage(USAGE.to_owned());
        let (rom, mut rest) = arguments.split_first().ok_or_else(usage)?;
        let mut options = Self {
            rom: PathBuf::from(rom),
            frames: 1200,
            warmup: 120,
            profile: None,
            assets: None,
            press: InputScript::default(),
            three_dimensional: false,
        };
        while let Some((name, tail)) = rest.split_first() {
            if name == "--3d" {
                options.three_dimensional = true;
                rest = tail;
                continue;
            }
            let (value, tail) = tail.split_first().ok_or_else(usage)?;
            let count = || {
                value
                    .parse::<u32>()
                    .map_err(|_| HarnessError::Usage(format!("{name}: {value:?} is not a count")))
            };
            match name.as_str() {
                "--frames" => options.frames = count()?.max(1),
                "--warmup" => options.warmup = count()?,
                "--profile" => options.profile = Some(value.clone()),
                "--assets" => options.assets = Some(PathBuf::from(value)),
                "--press" => options.press = InputScript::parse(value)?,
                _ => return Err(usage()),
            }
            rest = tail;
        }
        Ok(options)
    }
}

/// The stages of an emulated frame, in the order they run.
#[derive(Debug, Clone, Copy)]
enum Stage {
    Rollback,
    Core,
    Capture,
    Conversion,
    Lua,
    Publish,
    DrawList,
    Picture,
}

impl Stage {
    const ALL: [Self; 8] = [
        Self::Rollback,
        Self::Core,
        Self::Capture,
        Self::Conversion,
        Self::Lua,
        Self::Publish,
        Self::DrawList,
        Self::Picture,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Rollback => "rollback snapshot",
            Self::Core => "core run_frame",
            Self::Capture => "tile-record capture",
            Self::Conversion => "conversion (no Lua)",
            Self::Lua => "Lua frame hooks",
            Self::Publish => "publish + settings sync",
            Self::DrawList => "draw list build",
            Self::Picture => "2D picture copy",
        }
    }
}

/// The sum and the maximum of one stage, in seconds.
#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    total: f64,
    max: f64,
}

impl Tally {
    fn add(&mut self, seconds: f64) {
        self.total += seconds;
        self.max = self.max.max(seconds);
    }
}

/// Snapshots the rollback history of the application keeps, one every
/// [`ROLLBACK_INTERVAL`] emulated frames (REQ-LOOP-030).
const ROLLBACK_CAPACITY: usize = 30;

/// Emulated frames between two rollback snapshots (REQ-LOOP-030).
const ROLLBACK_INTERVAL: i64 = 60;

/// The rollback snapshots as the application takes them, its oldest buffer recycled.
#[derive(Debug, Default)]
struct Rollback {
    history: VecDeque<Vec<u8>>,
    /// What a snapshot in a new allocation costs, for comparison (the loop before the reuse).
    fresh: Tally,
    snapshots: u32,
}

impl Rollback {
    fn capture(&mut self, host: &Host) -> Result<(), HarnessError> {
        let mut buffer = if self.history.len() == ROLLBACK_CAPACITY {
            self.history.pop_front().unwrap_or_default()
        } else {
            Vec::new()
        };
        host.core
            .save_state_to(false, &mut buffer)
            .map_err(|error| HarnessError::Rom(error.to_string()))?;
        self.history.push_back(buffer);
        Ok(())
    }

    fn time_fresh(&mut self, host: &Host) {
        let started = Instant::now();
        let state = host.core.save_state(false);
        drop(state);
        self.fresh.add(started.elapsed().as_secs_f64());
        self.snapshots += 1;
    }
}

/// The hook time of the frame the timed runtime last ran, in milliseconds.
fn lua_milliseconds(host: &Host) -> f64 {
    host.engine
        .script_host_as::<TimedHost>()
        .map_or(0.0, TimedHost::current_frame)
}

/// Runs the benchmark and returns the report.
///
/// # Errors
///
/// What loading the profile or the ROM can return (see [`profile::load`] and the ROM mode of
/// `render`).
pub fn run(options: &Options) -> Result<String, HarnessError> {
    let profile = profile::load(options.profile.as_deref(), options.assets.as_deref())?;
    let (mut host, mut scene) = Host::start(&Startup {
        rom: &options.rom,
        battery: None,
        profile: &profile,
        mesh: RenderAlgorithm::Greedy,
        force_three_dimensional: options.three_dimensional,
    })?;
    let mut records = Vec::new();
    let mut picture = Vec::with_capacity(256 * 240);
    let mut tallies = [Tally::default(); Stage::ALL.len()];
    let mut record_count = 0.0;
    let mut whole = Tally::default();
    let mut rollback = Rollback::default();
    let total = options.warmup + options.frames;
    for counter in 0..total {
        let timed = counter >= options.warmup;
        let frame_start = Instant::now();
        let mut mark = frame_start;
        let mut lap = |stage: Stage, extra: f64| {
            let now = Instant::now();
            if timed {
                tallies[stage as usize].add(now.duration_since(mark).as_secs_f64() + extra);
            }
            mark = now;
        };

        let snapshot = host.core.frame_count() % ROLLBACK_INTERVAL == 0;
        if snapshot {
            rollback.capture(&host)?;
        }
        lap(Stage::Rollback, 0.0);

        host.core.run_frame(options.press.at(counter));
        lap(Stage::Core, 0.0);

        let batch = host.core.frame_batch();
        records.clear();
        records.extend_from_slice(batch.records);
        let (palette, mask_x, mask_y) = (batch.palette, batch.mask_x, batch.mask_y);
        lap(Stage::Capture, 0.0);

        let batch = FrameBatch {
            records: &records,
            palette,
            mask_x,
            mask_y,
        };
        let frame_counter = i32::try_from(counter)
            .map_err(|_| HarnessError::Usage("--frames: too many frames".to_owned()))?;
        let convert_start = Instant::now();
        host.scripted(&mut scene, |engine| {
            engine.convert(&batch, frame_counter);
        });
        let convert = convert_start.elapsed().as_secs_f64();
        let lua = lua_milliseconds(&host) / 1000.0;
        // The conversion lap excludes the hooks the runtime timed inside it.
        lap(Stage::Conversion, -lua.min(convert));

        host.engine.publish();
        host.sync_settings(&mut scene);
        lap(Stage::Publish, 0.0);

        host.engine.build_draw_list();
        lap(Stage::DrawList, 0.0);

        picture.clear();
        picture.extend_from_slice(host.core.pixel_frame());
        lap(Stage::Picture, 0.0);

        if timed {
            tallies[Stage::Lua as usize].add(lua);
            record_count += f64::from(u32::try_from(records.len()).unwrap_or(u32::MAX));
            whole.add(frame_start.elapsed().as_secs_f64());
            if snapshot {
                rollback.time_fresh(&host);
            }
        }
    }
    let mut text = report(options, &tallies, whole, record_count);
    if rollback.snapshots > 0 {
        let snapshots = f64::from(rollback.snapshots);
        let recycled = tallies[Stage::Rollback as usize];
        let _ = write!(
            text,
            "\n  rollback: {} snapshots, {:.4} ms each recycled (max {:.3}), {:.4} ms each in a \
             new allocation (max {:.3}), {} bytes",
            rollback.snapshots,
            recycled.total * 1000.0 / snapshots,
            recycled.max * 1000.0,
            rollback.fresh.total * 1000.0 / snapshots,
            rollback.fresh.max * 1000.0,
            rollback.history.back().map_or(0, Vec::len)
        );
    }
    Ok(text)
}

fn report(options: &Options, tallies: &[Tally], whole: Tally, record_count: f64) -> String {
    let frames = f64::from(options.frames);
    let mut lines = vec![format!(
        "bench-core: {} ({} frames after {} of warm-up, profile {}, {} segmentation)",
        options.rom.display(),
        options.frames,
        options.warmup,
        options.profile.as_deref().unwrap_or("none"),
        if options.three_dimensional {
            "3D"
        } else {
            "profile's"
        }
    )];
    for (stage, tally) in Stage::ALL.iter().zip(tallies) {
        lines.push(format!(
            "  {:<24} {:>8.4} ms/frame  max {:>8.3} ms",
            stage.name(),
            tally.total * 1000.0 / frames,
            tally.max * 1000.0
        ));
    }
    lines.push(format!(
        "  {:<24} {:>8.4} ms/frame  max {:>8.3} ms  ({:.0} frames/s)",
        "total",
        whole.total * 1000.0 / frames,
        whole.max * 1000.0,
        frames / whole.total.max(f64::EPSILON)
    ));
    let records = record_count / frames;
    lines.push(format!("  tile records per frame   {records:.1}"));
    lines.join("\n")
}
