//! Release benchmarks of the binding layer between the hooks and the host. Run with
//! `cargo test -p zax-scripting --release --test scripting perf -- --ignored --nocapture
//! --test-threads=1`. They print timings and assert only that the scripts ran without error.

use std::time::Instant;

use zax_engine3d::engine::{ConversionEngine, ConversionState};
use zax_engine3d::script::{ScriptError, ScriptHost};
use zax_scripting::{
    CollectingScriptLog, DetachedScriptBackdrop, DetachedScriptCamera, ProfileScriptRuntime,
    ScriptBridge, ScriptContext, ScriptEnvironment, ScriptLight, ScriptSettings,
};
use zax_settings::model::SettingsModel;

use super::lend_tests::Machine;
use super::support::{img, Batch, PatternSpec, ScriptProfileBuilder};

/// API accesses per turn of the loops of `HOOK_BODY`.
const ACCESSES_PER_TURN: u32 = 5;
const TURNS: u32 = 20;
/// Hook calls per timed round of the hook benchmark.
const CALLS: u32 = 500;
const ROUNDS: usize = 10;
const SHAPES: u32 = 60;
const SHAPE_FRAMES: i32 = 100;
const ITERATIONS: u32 = 200_000;

/// Typical accesses of a frame hook: camera and backdrop writes, a memory read and write, a
/// setting read.
const HOOK_BODY: &str = "
  for i = 1, 20 do
    Camera.Distance = 10 + i
    local m = Script:ReadMem(16 + i)
    local s = Setting.SceneRatio
    Script:WriteMem(64, m)
    Backdrop.Enable = (i % 2) == 0
  end
";

const BUILD_HOOK: &str = "
function BuildPattern()
  pattern.fillrate = 0.05
  local f = pattern.fillrate
  pattern.fillrate = f + 0.01
end
";

const UPDATE_S: &str = "
function UpdateS()
  if shape.TagStr ~= 's' then return end
  shape.Alpha = shape.Alpha * 0.5
  local layer = shape.Layer
  shape.Enable = true
  local x = shape.Pos.x
end
";

/// A frame hook of the runtime.
type Hook = fn(&mut ProfileScriptRuntime, &mut ConversionState) -> Result<(), ScriptError>;

/// The host side of a lend: what the application would lend for one frame.
struct Rig {
    camera: DetachedScriptCamera,
    backdrop: DetachedScriptBackdrop,
    machine: Machine,
}

impl Rig {
    fn new() -> Self {
        Self {
            camera: DetachedScriptCamera::new(),
            backdrop: DetachedScriptBackdrop::new(),
            machine: Machine::default(),
        }
    }

    fn lend<R>(&mut self, bridge: &ScriptBridge, operation: impl FnOnce() -> R) -> R {
        let mut result = None;
        bridge.lend(
            &mut ScriptContext {
                camera: &mut self.camera,
                backdrop: &mut self.backdrop,
                machine: &mut self.machine,
            },
            || result = Some(operation()),
        );
        result.expect("the lend runs its operation")
    }
}

/// The fastest of the runs: the least disturbed by the machine.
fn best(values: impl IntoIterator<Item = f64>) -> f64 {
    values.into_iter().fold(f64::INFINITY, f64::min)
}

/// Seconds to load a profile of `instances` instances of one pattern with the lend a host makes,
/// the `BuildPattern` script being linked to them or not.
fn load_seconds(instances: u32, linked: bool, hook: &str) -> f64 {
    let spec = PatternSpec {
        instance_count: usize::try_from(instances).unwrap(),
        ..PatternSpec::tagged("t")
    };
    let builder = ScriptProfileBuilder::new()
        .pattern(&[img::a(), img::b()], spec)
        .script_with("build", hook, if linked { "t" } else { "other" }, true);
    let profile = builder.profile();

    let mut engine = ConversionEngine::new();
    let log = CollectingScriptLog::new();
    let environment = ScriptEnvironment {
        settings: ScriptSettings::new(SettingsModel::new(), ScriptLight::detached()),
        log: Box::new(log.clone()),
        ..ScriptEnvironment::default()
    };
    let runtime = ProfileScriptRuntime::create(profile, environment, engine.state_mut()).unwrap();
    let bridge = runtime.bridge();
    engine.set_script_host(Box::new(runtime));
    let mut rig = Rig::new();
    let started = Instant::now();
    rig.lend(&bridge, || engine.load_profile(profile, true).unwrap());
    let elapsed = started.elapsed().as_secs_f64();
    assert_eq!(log.errors(), [] as [String; 0]);
    elapsed
}

/// Microseconds one `BuildPattern` call costs: the load with the hook minus the load without.
fn build_microseconds(instances: u32, hook: &str) -> f64 {
    let with = best((0..30).map(|_| load_seconds(instances, true, hook)));
    let without = best((0..30).map(|_| load_seconds(instances, false, hook)));
    (with - without) / f64::from(instances) * 1e6
}

#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn perf_build_pattern() {
    for instances in [50, 500] {
        let per_call = build_microseconds(instances, BUILD_HOOK);
        println!(
            "BuildPattern, {instances} instances: {per_call:.2} us per hook call \
             (3 accesses), {:.2} us of it without the accesses",
            build_microseconds(instances, "function BuildPattern() end")
        );
    }
}

#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn perf_frame_hooks() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a(), img::b()], PatternSpec::tagged("t"))
        .script(
            "frame",
            &format!("function Update() {HOOK_BODY} end function LateUpdate() {HOOK_BODY} end"),
        )
        .start();
    let mut rig = Rig::new();
    let bridge = session.runtime(|runtime, _| runtime.bridge());
    let hooks: [(&str, Hook); 2] = [
        ("Update", ProfileScriptRuntime::invoke_update),
        ("LateUpdate", ProfileScriptRuntime::invoke_late_update),
    ];
    for (name, hook) in hooks {
        let seconds = rig.lend(&bridge, || {
            session.runtime(|runtime, state| {
                best((0..ROUNDS).map(|_| {
                    let started = Instant::now();
                    for _ in 0..CALLS {
                        hook(runtime, state).unwrap();
                    }
                    started.elapsed().as_secs_f64()
                }))
            })
        });
        let per_call = seconds / f64::from(CALLS);
        let accesses = ACCESSES_PER_TURN * TURNS;
        println!(
            "{name}: {:.2} us per hook call, {:.1} ns per API access ({accesses} accesses)",
            per_call * 1e6,
            per_call * 1e9 / f64::from(accesses)
        );
    }

    assert_eq!(session.log.errors(), [] as [String; 0]);
}

/// Seconds per converted frame of `SHAPES` linked shapes, the frame hooks being `Update` and
/// `LateUpdate` with `extra` added to the script.
fn frame_seconds(extra: &str) -> f64 {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::c()], PatternSpec::tagged("s"))
        .script(
            "frame",
            &format!(
                "function Update() {HOOK_BODY} end function LateUpdate() {HOOK_BODY} end {extra}"
            ),
        )
        .start();
    let mut rig = Rig::new();
    let bridge = session.runtime(|runtime, _| runtime.bridge());
    let mut batch = Batch::new();
    for index in 0..i32::try_from(SHAPES).unwrap() {
        batch = batch.sprite(img::c(), 24 * (index % 10), 32 * (index / 10));
    }

    let seconds = best((0..5).map(|round| {
        let started = Instant::now();
        for counter in 1..=SHAPE_FRAMES {
            rig.lend(&bridge, || {
                session.frame(&batch, round * SHAPE_FRAMES + counter);
            });
        }
        started.elapsed().as_secs_f64() / f64::from(SHAPE_FRAMES)
    }));
    assert_eq!(session.log.errors(), [] as [String; 0]);
    seconds
}

#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn perf_frames() {
    let hooks = frame_seconds("");
    let empty = frame_seconds("function UpdateS() end");
    println!(
        "empty UpdateS: {:.2} us per call",
        (empty - hooks) * 1e6 / f64::from(SHAPES)
    );
    let full = frame_seconds(UPDATE_S);
    println!(
        "frame with Update and LateUpdate: {:.1} us; with {SHAPES} UpdateS calls on top: {:.1} us, \
         {:.2} us per UpdateS call (7 accesses)",
        hooks * 1e6,
        full * 1e6,
        (full - hooks) * 1e6 / f64::from(SHAPES)
    );
}

#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn perf_accesses() {
    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::c()], PatternSpec::tagged("s"))
        .script("frame", "x = 0")
        .start();
    let mut rig = Rig::new();
    let bridge = session.runtime(|runtime, _| runtime.bridge());
    let loops = [
        ("empty loop", ""),
        ("Camera.Distance read", "local a = Camera.Distance"),
        ("Camera.Distance write", "Camera.Distance = 5"),
        ("Setting.SceneRatio read", "local a = Setting.SceneRatio"),
        ("Script:ReadMem", "local a = Script:ReadMem(16)"),
        ("unknown member read", "local a = Camera.Nope"),
        ("Vector3(1, 2, 3)", "local a = Vector3(1, 2, 3)"),
        ("vector component read", "local a = v.x"),
        ("vector component write", "v.x = 4"),
    ];
    for (name, body) in loops {
        let chunk = format!("local v = Vector3(1, 2, 3) for i = 1, {ITERATIONS} do {body} end");
        let seconds = best((0..5).map(|_| {
            rig.lend(&bridge, || {
                let started = Instant::now();
                session.evaluate(&chunk);
                started.elapsed().as_secs_f64()
            })
        }));
        println!("{name}: {:.0} ns", seconds / f64::from(ITERATIONS) * 1e9);
    }

    assert_eq!(session.log.errors(), [] as [String; 0]);
}
