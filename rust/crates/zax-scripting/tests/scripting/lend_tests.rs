//! The host context a host lends to the script API for the length of a call.

use zax_scripting::{
    DetachedScriptBackdrop, DetachedScriptCamera, DetachedScriptInput, DetachedScriptMemory,
    ScriptBackdrop, ScriptCamera, ScriptContext, ScriptInput, ScriptMemory,
};

use super::support::ScriptProfileBuilder;

/// The input and the memory of one machine.
#[derive(Default)]
struct Machine {
    input: DetachedScriptInput,
    memory: DetachedScriptMemory,
}

impl ScriptInput for Machine {
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

impl ScriptMemory for Machine {
    fn read(&self, address: u16) -> u8 {
        self.memory.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.memory.write(address, value);
    }
}

#[test]
fn a_lent_context_serves_the_script_api_for_the_length_of_the_lend() {
    let mut session = ScriptProfileBuilder::new().script("a", "-- nothing").load();
    let mut camera = DetachedScriptCamera::new();
    let mut backdrop = DetachedScriptBackdrop::new();
    let mut machine = Machine::default();
    machine.memory.write(0x10, 7);
    let bridge = session.runtime(|runtime, _| runtime.bridge());

    bridge.lend(
        &mut ScriptContext {
            camera: &mut camera,
            backdrop: &mut backdrop,
            machine: &mut machine,
        },
        || {
            session.evaluate("Camera.Distance = 77; Backdrop.Enable = true; Script:WriteMem(0x11, Script:ReadMem(0x10) + 1)");
        },
    );

    assert!((camera.distance() - 77.0).abs() < 1e-3);
    assert!(backdrop.enabled());
    assert_eq!(machine.memory.read(0x11), 8);
    // Outside the lend the script sees the headless bridges again, not the lent ones.
    assert_eq!(session.number("return Script:ReadMem(0x10)"), 0.0);
    assert_eq!(session.log.errors(), [] as [String; 0]);
}
