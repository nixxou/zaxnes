//! `NesCore` as an `Engine`: that the trait is a faithful view of the core, and that what the
//! design note keeps off the trait is still reachable — safely — by the downcast.
//!
//! The core's *behaviour* is pinned by the forty-odd other files here; what this one pins is the
//! abstraction lot FCEUX-23 put in front of it, called through `dyn Engine` so the tests exercise
//! the same vtable the application does.

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use zax_core_api::{
    engine_of_state, AudioError, Capabilities, Capability, ControllerPortType, Engine, EngineKind,
    NesRegion, StateLoadFailure,
};
use zax_nes::NesCore;

/// A minimal NROM image: 16 KiB of PRG whose reset vector points at a `JMP` to itself, so the
/// machine runs for ever without needing a real game.
fn image() -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + 16 * 1024 + 8 * 1024];
    rom[..4].copy_from_slice(b"NES\x1a");
    rom[4] = 1; // 1 x 16 KiB PRG
    rom[5] = 1; // 1 x 8 KiB CHR
    let prg = 16;
    // $C000: JMP $C000 — 4C 00 C0
    rom[prg] = 0x4C;
    rom[prg + 1] = 0x00;
    rom[prg + 2] = 0xC0;
    // The reset vector at $FFFC, which is the last 16 KiB bank's offset $3FFC.
    rom[prg + 0x3FFC] = 0x00;
    rom[prg + 0x3FFD] = 0xC0;
    rom
}

/// A boxed engine with the image above loaded, as the application's worker thread holds one.
fn engine() -> Box<dyn Engine> {
    let mut core = Box::new(NesCore::new());
    let loaded = Engine::load_rom(core.as_mut(), &image(), "trait.nes");
    assert!(loaded.success, "{:?}", loaded.error);
    core
}

/// REQ-CORE-010, REQ-CORE-013, REQ-CORE-005: the lifecycle, through the trait.
#[test]
fn the_lifecycle_is_the_cores_own() {
    let mut engine = engine();

    assert!(engine.is_running());
    assert_eq!(engine.rom_name(), Some("trait.nes"));
    assert_eq!(engine.region(), NesRegion::Ntsc);
    assert_eq!(engine.frame_count(), 0);

    engine.run_frame(0);
    assert_eq!(engine.frame_count(), 1);
    engine.set_frame_count(99);
    assert_eq!(engine.frame_count(), 99);

    engine.close_rom();
    assert!(!engine.is_running());
    assert_eq!(engine.rom_name(), None);
    assert_eq!(engine.run_frame(0), 0);
}

/// `NesCore::reset` is the power cycle (DEC-02-Q3), so `power` and `reset` are the same call
/// here; AMD-044 REQ-CORE-122's soft reset is the other one, and it keeps the RAM.
#[test]
fn power_and_reset_are_one_call_on_this_core_and_the_soft_reset_is_not() {
    let mut engine = engine();

    engine.write_memory(0x0010, 0xA5);
    for _ in 0..3 {
        engine.run_frame(0);
    }

    engine.soft_reset();
    assert_eq!(engine.frame_count(), 0);
    assert_eq!(engine.read_memory(0x0010), 0xA5);

    engine.power();
    assert_eq!(engine.frame_count(), 0);
    assert_eq!(
        engine.read_memory(0x0010),
        0x00,
        "a power cycle clears the work RAM (REQ-CORE-004)"
    );
}

/// REQ-CORE-030, REQ-CORE-045, REQ-CORE-048: one frame's three outputs.
#[test]
fn a_frame_gives_a_picture_a_batch_and_audio() {
    let mut engine = engine();
    engine.run_frame(0);

    let mut picture = Vec::new();
    engine.pixel_frame_indices_into(&mut picture);
    assert_eq!(picture.len(), 256 * 240);
    assert!(picture.iter().all(|&index| index < 64));

    let batch = engine.frame_batch();
    assert_eq!(batch.palette.len(), 32);
    assert_eq!(batch.visible_rows, zax_core_api::STANDARD_ROWS);

    assert_eq!(
        engine.read_audio(&mut [0.0; 7], true),
        Err(AudioError::OddStereoBuffer(7))
    );
    assert_eq!(engine.sample_rate(), 48_000);
    assert_eq!(engine.set_sample_rate(44_100), Ok(()));
    assert_eq!(engine.sample_rate(), 44_100);
    assert_eq!(
        engine.set_sample_rate(8_000),
        Err(AudioError::SampleRateOutOfRange(8_000))
    );
}

/// REQ-CORE-016, REQ-CORE-024: the two machine settings the trait carries.
#[test]
fn the_region_and_the_ports_are_the_cores_own() {
    let mut engine = engine();

    for region in [
        NesRegion::Pal,
        NesRegion::Dendy,
        NesRegion::Multi,
        NesRegion::Ntsc,
    ] {
        engine.set_region(region);
        assert_eq!(engine.region(), region);
    }

    assert_eq!(engine.port_type(0), Ok(ControllerPortType::Controller));
    assert_eq!(
        engine.set_port_type(1, ControllerPortType::Zapper),
        Ok(()),
        "this core does have a Zapper, unlike the other one"
    );
    assert_eq!(engine.port_type(1), Ok(ControllerPortType::Zapper));
    assert!(engine.port_type(2).is_err());
}

/// REQ-CORE-090, REQ-CORE-091: the round trip, and the first four bytes that name the engine —
/// which is how the application's slots tell the two apart (step 5 of the lot).
#[test]
fn a_savestate_round_trip_goes_through_the_trait_and_names_its_engine() {
    let mut engine = engine();
    for _ in 0..5 {
        engine.run_frame(0);
    }

    let mut block = Vec::new();
    engine.save_state_to(true, &mut block).unwrap();
    assert!(block.len() > 1_000);
    assert!(block.len() <= engine.max_state_size());
    assert_eq!(
        engine_of_state(&block, 0),
        Some(EngineKind::Zaxnes),
        "a block must say which engine wrote it"
    );

    engine.write_memory(0x0010, 0x5A);
    let loaded = engine.try_load_state(&block, 0).unwrap();
    assert_eq!(loaded.consumed, block.len());
    assert_eq!(loaded.rom_name, "trait.nes");
    assert_eq!(engine.frame_count(), 5);
    assert_ne!(engine.read_memory(0x0010), 0x5A);

    // A block whose first four bytes name the *other* engine is refused, and not as corrupt.
    let mut foreign = zax_core_api::state_magic(EngineKind::Fceux)
        .to_le_bytes()
        .to_vec();
    foreign.extend_from_slice(&[0_u8; 64]);
    let refused = engine.try_load_state(&foreign, 0).unwrap_err();
    assert_eq!(refused.failure, StateLoadFailure::CorruptState);
    // And an honest-to-goodness foreign *core* name still gets the spec's own message, which is
    // what `NesCore::FOREIGN_CORE_MESSAGE` is; `core_state_tests.rs` pins that path.
    assert_eq!(
        NesCore::FOREIGN_CORE_MESSAGE,
        zax_core_api::FOREIGN_CORE_MESSAGE
    );
}

/// REQ-CORE-095: battery RAM, which the trait copies rather than lends — the one method that is
/// not a bare forward.
#[test]
fn battery_ram_is_copied_out_and_matches_what_the_core_lends() {
    let mut engine = engine();

    let mut battery = vec![0xAA_u8; 16];
    engine.export_battery_ram(&mut battery);
    assert_eq!(
        battery.len(),
        0,
        "the test image has no battery, so there is nothing to export"
    );
    assert!(!engine.import_battery_ram(&[0_u8; 8192]));

    // And on the concrete core the borrow is still there, with the same bytes.
    let core = engine
        .as_any()
        .downcast_ref::<NesCore>()
        .expect("the concrete core");
    assert_eq!(NesCore::export_battery_ram(core), battery.as_slice());
}

/// §2 of the design note: everything is on, and nothing has a reason to be disabled.
#[test]
fn every_capability_is_on_for_this_engine() {
    let engine = engine();

    assert_eq!(engine.capabilities(), Capabilities::ZAXNES);
    assert_eq!(engine.kind(), EngineKind::Zaxnes);
    for capability in [
        Capability::Widescreen,
        Capability::HackPacks,
        Capability::SpriteLimit,
        Capability::RunAhead,
        Capability::Rollback,
        Capability::FastCoreSnapshot,
        Capability::MachineInternals,
    ] {
        assert!(engine.capabilities().has(capability));
        assert_eq!(engine.capabilities().refusal(capability), None);
    }
}

/// The point of the downcast: the Zaxnes-only features are off the trait and still reachable,
/// safely, with the capability flag as the gate — and the **fast** rollback path is among them,
/// which is the whole reason it is a downcast and not a trait method that FCEUX would have to
/// fake.
#[test]
fn the_downcast_reaches_what_the_trait_deliberately_leaves_out() {
    let mut engine = engine();
    for _ in 0..3 {
        engine.run_frame(0);
    }

    assert!(engine.capabilities().has(Capability::FastCoreSnapshot));
    let core = engine
        .as_any_mut()
        .downcast_mut::<NesCore>()
        .expect("the concrete core");

    // The in-memory snapshot of AMD-041 REQ-CORE-121, which rollback must keep using here.
    let mut snapshot = zax_nes::state::CoreState::with_capacity(core.max_core_state_size());
    core.capture_core_state(&mut snapshot).unwrap();
    assert!(!snapshot.is_empty());
    core.write_memory(0x0010, 0x5A);
    core.restore_core_state(&snapshot).unwrap();
    assert_ne!(core.read_memory(0x0010), 0x5A);

    // The exact run-ahead, the widescreen picture, the sprite limit and the machine's internals:
    // each a `NesCore` method the trait does not have.
    core.run_frame_ahead(0, 1);
    assert!(core.restore_real_frame());
    let mut wide = Vec::new();
    assert_eq!(core.wide_picture_into(&mut wide), 240);
    core.set_sprite_limit(zax_nes::ppu::SpriteLimit::Removed);
    assert!(core.cartridge().is_some());
    assert_eq!(core.hack_options().len(), 0);

    // And the downcast answers honestly for the wrong type rather than panicking.
    assert!(engine.as_any().downcast_ref::<String>().is_none());
}

/// Lot FCEUX-1 found this and the trait must not undo it: an engine moves between threads but is
/// never shared, because FCEUX keeps one machine in process globals.
#[test]
fn an_engine_can_move_to_another_thread() {
    fn moves<T: Send>(_: &T) {}

    let engine = engine();
    moves(&engine);

    let frames = std::thread::spawn(move || {
        let mut engine = engine;
        for _ in 0..4 {
            engine.run_frame(0);
        }
        engine.frame_count()
    })
    .join()
    .expect("the thread");
    assert_eq!(frames, 4);
}
