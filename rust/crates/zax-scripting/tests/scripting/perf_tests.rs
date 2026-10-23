//! Release benchmark of the hook binding layer. Run with
//! `cargo test -p zax-scripting --release --test scripting perf -- --ignored --nocapture`.

use std::time::Instant;

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::script::ScriptHost;
use zax_scripting::{
    CollectingScriptLog, DetachedScriptBackdrop, DetachedScriptCamera, DetachedScriptInput,
    DetachedScriptMemory, ProfileScriptRuntime, ScriptContext, ScriptEnvironment, ScriptLight,
    ScriptSettings,
};
use zax_settings::model::SettingsModel;

use super::support::{img, PatternSpec, ScriptProfileBuilder};

/// API accesses per loop turn of the frame hooks below.
const ACCESSES_PER_TURN: usize = 5;
const TURNS: usize = 20;
const FRAMES: usize = 2000;

const FRAME_HOOKS: &str = "
function Update()
  for i = 1, 20 do
    Camera.Distance = 10 + i
    local m = Script:ReadMem(16 + i)
    local s = Setting.SceneRatio
    Script:WriteMem(64, m)
    Backdrop.Enable = (i % 2) == 0
  end
end
function LateUpdate()
  for i = 1, 20 do
    Camera.Distance = 10 + i
    local m = Script:ReadMem(16 + i)
    local s = Setting.SceneRatio
    Script:WriteMem(64, m)
    Backdrop.Enable = (i % 2) == 0
  end
end
";

const BUILD_HOOK: &str = "
function BuildPattern()
  pattern.fillrate = 0.05
  local f = pattern.fillrate
  pattern.fillrate = f + 0.01
end
";

struct Rig {
    camera: DetachedScriptCamera,
    backdrop: DetachedScriptBackdrop,
    machine: Machine,
}

#[derive(Default)]
struct Machine {
    input: DetachedScriptInput,
    memory: DetachedScriptMemory,
}

impl zax_scripting::ScriptInput for Machine {
    fn player_state(&self, index: usize) -> i32 {
        self.input.player_state(index)
    }
    fn global_state(&self) -> i32 {
        self.input.global_state()
    }
    fn port_byte(&self, port: usize) -> i32 {
        self.input.port_byte(port)
    }
    fn port_type(&self, port: usize) -> i32 {
        self.input.port_type(port)
    }
    fn set_port_types(&mut self, first: i32, second: i32) {
        self.input.set_port_types(first, second);
    }
    fn zapper_offset(&self) -> zax_engine3d::vectors::Vec3 {
        self.input.zapper_offset()
    }
    fn set_zapper_offset(&mut self, offset: zax_engine3d::vectors::Vec3) {
        self.input.set_zapper_offset(offset);
    }
}

impl zax_scripting::ScriptMemory for Machine {
    fn read(&self, address: u16) -> u8 {
        self.memory.read(address)
    }
    fn write(&mut self, address: u16, value: u8) {
        self.memory.write(address, value);
    }
}

impl Rig {
    fn new() -> Self {
        Self {
            camera: DetachedScriptCamera::new(),
            backdrop: DetachedScriptBackdrop::new(),
            machine: Machine::default(),
        }
    }

    fn lend<R>(&mut self, bridge: &zax_scripting::ScriptBridge, operation: impl FnOnce() -> R) -> R {
        let mut result = None;
        bridge.lend(
            &mut ScriptContext {
                camera: &mut self.camera,
                backdrop: &mut self.backdrop,
                machine: &mut self.machine,
            },
            || result = Some(operation()),
        );
        result.unwrap()
    }
}

/// Seconds to load a profile of `instances` instances of one pattern, lent, with the script
/// matching (`tag` = `t`) or not.
fn load_seconds(instances: usize, matching: bool) -> f64 {
    let spec = PatternSpec {
        instance_count: instances,
        ..PatternSpec::tagged("t")
    };
    let builder = ScriptProfileBuilder::new()
        .pattern(&[img::a(), img::b()], spec)
        .script_with("build", BUILD_HOOK, if matching { "t" } else { "other" }, true);
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

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn perf_hooks() {
    for instances in [50, 500] {
        let with: Vec<f64> = (0..15).map(|_| load_seconds(instances, true)).collect();
        let without: Vec<f64> = (0..15).map(|_| load_seconds(instances, false)).collect();
        let per_instance = (median(with) - median(without)) / instances as f64;
        println!(
            "BuildPattern, {instances} instances: {:.2} us per hook call, {:.2} us per API access (4 per call)",
            per_instance * 1e6,
            per_instance * 1e6 / 4.0
        );
    }

    let mut session = ScriptProfileBuilder::new()
        .pattern(&[img::a(), img::b()], PatternSpec::tagged("t"))
        .script("frame", FRAME_HOOKS)
        .start();
    let mut rig = Rig::new();
    let bridge = session.runtime(|runtime, _| runtime.bridge());
    for (name, hook) in [
        ("Update", ProfileScriptRuntime::invoke_update as fn(&mut ProfileScriptRuntime, &mut _) -> _),
        ("LateUpdate", ProfileScriptRuntime::invoke_late_update),
    ] {
        let seconds = rig.lend(&bridge, || {
            session.runtime(|runtime, state| {
                let started = Instant::now();
                for _ in 0..FRAMES {
                    hook(runtime, state).unwrap();
                }
                started.elapsed().as_secs_f64()
            })
        });
        let per_call = seconds / FRAMES as f64;
        println!(
            "{name}: {:.2} us per hook call, {:.1} ns per API access ({} accesses)",
            per_call * 1e6,
            per_call * 1e9 / (ACCESSES_PER_TURN * TURNS) as f64,
            ACCESSES_PER_TURN * TURNS
        );
    }
    assert_eq!(session.log.errors(), [] as [String; 0]);
}
