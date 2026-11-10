//! [`FceuxEngine`] against the `Engine` trait: what the application will call, called through
//! `dyn Engine` so the tests exercise the same vtable the application does.
//!
//! Like `fceux_core_tests.rs` the whole file needs the `fceux-core` feature and a built fork, and
//! **it serialises itself** with the same `SERIAL` discipline: FCEUX keeps its machine in
//! file-scope globals, so two tests must never hold an engine at once.

#![cfg(feature = "fceux-core")]
// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use zax_core_api::{
    state_magic, AudioError, Capability, ControllerPortType, Engine, EngineKind, NesRegion,
    StateLoadFailure, FOREIGN_CORE_MESSAGE,
};
use zax_fceux::{FceuxEngine, RamInit, FRAME_LEN};

static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// `nestest.nes` of `rust/test-roms`, or `None` after saying what is missing: the ROMs are not in
/// the repository, so a checkout without them skips rather than fails.
fn nestest() -> Option<PathBuf> {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms/nestest.nes");
    if rom.is_file() {
        return Some(rom);
    }
    eprintln!("skipped: rust/test-roms/nestest.nes is missing");
    None
}

/// An engine with `nestest` loaded and `frames` frames run, or `None` when the ROM is missing.
fn running(frames: u32) -> Option<FceuxEngine> {
    let rom = nestest()?;
    let image = std::fs::read(&rom).unwrap();
    let mut engine = FceuxEngine::new().unwrap();
    let loaded = engine.load_rom(&image, "nestest.nes");
    assert!(loaded.success, "{:?}", loaded.error);
    for _ in 0..frames {
        engine.run_frame(0);
    }
    Some(engine)
}

/// REQ-CORE-010, REQ-CORE-013: a load reports success and the detected region.
#[test]
fn a_load_reports_the_region_and_the_name_it_was_given() {
    let _guard = lock();
    let Some(engine) = running(0) else { return };

    assert!(engine.is_running());
    assert_eq!(engine.rom_name(), Some("nestest.nes"));
    // nestest is an NTSC image, and a fresh engine runs NTSC: REQ-CORE-013 has the load *report*
    // the region and the host apply it, which is `NesCore::load_rom`'s contract too.
    assert_eq!(engine.region(), NesRegion::Ntsc);
    assert_eq!(engine.frame_count(), 0);
}

/// REQ-CORE-013: the region a load reports comes from the **image**, not from the region the
/// previous game in this process was left running.
///
/// FCEUX keeps its region in a process global and a plain iNES load does not touch it, so an
/// engine that read the region back off the core reported the last game's. This test is the one
/// that would have caught it: it leaves the core at PAL exactly as a previous game would, then
/// loads an NTSC image with a new engine and asks both questions — what the load reported, and
/// what the machine is actually running.
#[test]
fn a_load_reports_the_images_region_whatever_the_previous_game_left_behind() {
    let _guard = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).unwrap();

    // A previous game, played in PAL, and dropped.
    {
        let mut played = FceuxEngine::new().unwrap();
        assert!(played.load_rom(&image, "nestest.nes").success);
        played.set_region(NesRegion::Pal);
        assert_eq!(played.run_frame(0), 33_248);
    }

    let mut engine = FceuxEngine::new().unwrap();
    let loaded = engine.load_rom(&image, "nestest.nes");
    assert!(loaded.success, "{:?}", loaded.error);
    assert_eq!(
        loaded.region,
        NesRegion::Ntsc,
        "the load reported the previous game's region instead of the image's"
    );
    assert_eq!(engine.region(), NesRegion::Ntsc);
    // And the core really is running it, not just the engine's own field.
    assert_eq!(engine.run_frame(0), 29_781);
}

/// REQ-CORE-011: what FCEUX actually refuses, and the cause the engine reports for it.
///
/// FCEUX reports one failure for every reason, so the engine reads the typed cause off the
/// image's own header. What this test also records is **how lenient FCEUX is**, which is a real
/// difference from `zax-nes` and worth knowing before offering a game to it: an image whose
/// header announces more than the file holds, and one on a mapper number no board claims, are
/// both *loaded* — `zax-nes` refuses the first as `TruncatedFile` and the second as
/// `UnsupportedMapper`. In practice FCEUX refuses only an image it cannot recognise as any
/// format at all, which is the `CorruptHeader` branch below; the other two branches of the
/// classifier are there for a refusal of a well-formed image, which this ROM set has no example
/// of.
#[test]
fn what_fceux_refuses_is_reported_with_a_typed_cause_and_leaves_it_stopped() {
    use zax_core_api::RomLoadFailure;

    let _guard = lock();
    let mut engine = FceuxEngine::new().unwrap();

    // Not a ROM in any format FCEUX knows.
    let refused = engine.load_rom(b"this is not a ROM at all, not even close", "junk.nes");
    assert!(!refused.success);
    assert_eq!(refused.failure, RomLoadFailure::CorruptHeader);
    assert!(
        refused.error.is_some_and(|message| !message.is_empty()),
        "a refusal must carry the core's own explanation"
    );
    // A refusal leaves the engine stopped (REQ-CORE-011, Q-002).
    assert!(!engine.is_running());
    assert_eq!(engine.rom_name(), None);

    // A header announcing 2 x 16 KiB of PRG with nothing after it: accepted, padded out.
    let mut truncated = vec![0_u8; 16];
    truncated[..4].copy_from_slice(b"NES\x1a");
    truncated[4] = 2;
    assert!(
        engine.load_rom(&truncated, "short").success,
        "FCEUX's leniency on a short image has changed; the note above needs rewriting"
    );

    // A whole, well-formed image on mapper 255: accepted too.
    let mut exotic = vec![0_u8; 16 + 16 * 1024];
    exotic[..4].copy_from_slice(b"NES\x1a");
    exotic[4] = 1;
    exotic[6] = 0xF0;
    exotic[7] = 0xF0;
    assert!(
        engine.load_rom(&exotic, "exotic").success,
        "FCEUX's leniency on an unknown mapper has changed; the note above needs rewriting"
    );
}

/// REQ-CORE-030, REQ-CORE-045: a frame is records *and* palette, and the records are the ones
/// the 3D renderer reads — the capture is on from the start, not something a host must enable.
#[test]
fn a_frame_batch_carries_records_and_a_palette_without_being_asked_for_it() {
    let _guard = lock();
    let Some(engine) = running(60) else { return };

    let batch = engine.frame_batch();
    assert!(
        batch.records.len() > 100,
        "nestest draws a full screen of text by frame 60, got {} records",
        batch.records.len()
    );
    assert!(
        batch.palette.iter().any(|&entry| entry != 0),
        "the palette is all zero, so it was never captured"
    );
    assert!(batch.palette.iter().all(|&entry| entry < 64));

    // The widescreen fields stay standard, which is what `Capabilities::FCEUX` promises.
    assert_eq!(batch.margin, 0);
    assert_eq!(batch.margin_y, 0);
    assert_eq!(batch.margin_palette, None);
    assert_eq!(batch.visible_rows, zax_core_api::STANDARD_ROWS);
}

/// REQ-CORE-048: the 2D picture is one NES colour index a pixel, and it is there before any frame
/// has run rather than leaving the host with an empty buffer.
#[test]
fn the_picture_is_a_full_frame_of_indices_even_before_the_first_frame() {
    let _guard = lock();
    let mut picture = Vec::new();
    {
        // Its own scope: there is one FCEUX per process, so the first engine has to be dropped
        // before the second exists.
        let Some(engine) = running(0) else { return };
        engine.pixel_frame_indices_into(&mut picture);
        assert_eq!(picture.len(), FRAME_LEN);
    }

    let Some(engine) = running(30) else { return };
    engine.pixel_frame_indices_into(&mut picture);
    assert_eq!(picture.len(), FRAME_LEN);
    assert!(picture.iter().all(|&index| index < 64));
    assert!(
        picture.iter().any(|&index| index != picture[0]),
        "the picture is uniform after 30 frames, so nothing was drawn"
    );
}

/// REQ-CORE-071: entirely or not at all, and a stereo buffer of odd length is refused — the two
/// rules `zax-nes`'s ring follows, which a host switching engines must not have to notice.
#[test]
fn audio_comes_out_whole_or_not_at_all_and_an_odd_stereo_buffer_is_refused() {
    let _guard = lock();
    let Some(mut engine) = running(0) else { return };
    assert_eq!(engine.sample_rate(), 48_000);

    // Nothing has run, so nothing is available and the buffer is left alone.
    let mut out = vec![-7.0_f32; 64];
    assert_eq!(engine.read_audio(&mut out, false), Ok(0));
    assert!(out
        .iter()
        .all(|&sample| (sample - -7.0).abs() < f32::EPSILON));

    assert_eq!(
        engine.read_audio(&mut [0.0; 7], true),
        Err(AudioError::OddStereoBuffer(7))
    );
    assert_eq!(engine.read_audio(&mut [], false), Ok(0));

    // One frame is about 800 samples at 48 kHz, which is well over the 64 asked for.
    engine.run_frame(0);
    assert_eq!(engine.read_audio(&mut out, false), Ok(64));
    assert!(
        out.iter().all(|&sample| (-1.0..=1.0).contains(&sample)),
        "samples left the -1..1 scale the other engine produces"
    );

    // Stereo doubles each sample rather than halving the rate.
    let mut stereo = vec![0.0_f32; 32];
    assert_eq!(engine.read_audio(&mut stereo, true), Ok(32));
    for pair in stereo.chunks_exact(2) {
        assert!((pair[0] - pair[1]).abs() < f32::EPSILON);
    }
}

/// REQ-CORE-070: outside `zax-nes`'s range the rate is refused exactly as `zax-nes` refuses it;
/// inside it, one FCEUX cannot produce is snapped and `sample_rate()` says so.
#[test]
fn a_rate_fceux_cannot_produce_is_snapped_and_reported_honestly() {
    let _guard = lock();
    let Some(mut engine) = running(0) else { return };

    assert_eq!(
        engine.set_sample_rate(8_000),
        Err(AudioError::SampleRateOutOfRange(8_000))
    );
    assert_eq!(
        engine.set_sample_rate(192_000),
        Err(AudioError::SampleRateOutOfRange(192_000))
    );

    // Accepted as they are.
    for rate in [44_100, 48_000, 96_000] {
        assert_eq!(engine.set_sample_rate(rate), Ok(()));
        assert_eq!(engine.sample_rate(), rate);
    }

    // 22 050 is inside `zax-nes`'s range and outside FCEUX's; the nearest it has is 44 100, and
    // that is what it reports rather than the rate it was handed.
    assert_eq!(engine.set_sample_rate(22_050), Ok(()));
    assert_eq!(engine.sample_rate(), 44_100);
    assert_eq!(engine.set_sample_rate(50_000), Ok(()));
    assert_eq!(engine.sample_rate(), 48_000);

    // Silence is passed through.
    assert_eq!(engine.set_sample_rate(0), Ok(()));
    assert_eq!(engine.sample_rate(), 0);
}

/// REQ-CORE-080, REQ-CORE-081: the two calls Lua and the achievements make.
#[test]
fn work_ram_can_be_written_and_read_back() {
    let _guard = lock();
    let Some(mut engine) = running(30) else {
        return;
    };

    engine.write_memory(0x0010, 0xA5);
    assert_eq!(engine.read_memory(0x0010), 0xA5);
    engine.write_memory(0x0010, 0x5A);
    assert_eq!(engine.read_memory(0x0010), 0x5A);
}

/// REQ-CORE-032: the counter counts frames, and a restored state's number survives even though
/// FCEUX's own counter does not carry it.
#[test]
fn the_frame_counter_counts_and_can_be_set() {
    let _guard = lock();
    let Some(mut engine) = running(10) else {
        return;
    };
    assert_eq!(engine.frame_count(), 10);

    engine.set_frame_count(5_000);
    assert_eq!(engine.frame_count(), 5_000);
    engine.run_frame(0);
    assert_eq!(engine.frame_count(), 5_001);

    // A power cycle restarts from 0, offset and all (REQ-CORE-006).
    engine.power();
    assert_eq!(engine.frame_count(), 0);
}

/// REQ-CORE-014: a hard reset is the power cycle, a soft one taps the RESET line and keeps the
/// RAM — the distinction AMD-044 REQ-CORE-122 draws, here through FCEUX's own two calls.
#[test]
fn a_soft_reset_keeps_the_work_ram_and_a_hard_one_does_not() {
    let _guard = lock();
    let Some(mut engine) = running(30) else {
        return;
    };

    engine.write_memory(0x0010, 0xA5);
    engine.soft_reset();
    assert_eq!(engine.frame_count(), 0);
    assert_eq!(
        engine.read_memory(0x0010),
        0xA5,
        "the RESET line must not clear the work RAM"
    );

    engine.reset();
    assert_eq!(
        engine.read_memory(0x0010),
        0x00,
        "the power cycle must refill the work RAM, and the engine's fill is zero"
    );
}

/// REQ-CORE-091: a round trip brings the machine back, and the sentinel written afterwards is
/// rolled back — so the load cannot pass by doing nothing.
#[test]
fn a_savestate_round_trip_brings_the_machine_back() {
    let _guard = lock();
    let Some(mut engine) = running(60) else {
        return;
    };

    let mut before = Vec::new();
    engine.pixel_frame_indices_into(&mut before);
    engine.set_frame_count(1_234);

    let mut block = Vec::new();
    engine.save_state_to(true, &mut block).unwrap();
    assert!(block.len() > 1_000);
    assert!(block.len() <= engine.max_state_size());
    // The block names its engine in its first four bytes, which is how a slot refuses the other
    // engine's (step 5 of the lot).
    assert_eq!(
        zax_core_api::engine_of_state(&block, 0),
        Some(EngineKind::Fceux)
    );

    engine.write_memory(0x0010, 0x5A);
    for _ in 0..10 {
        engine.run_frame(0);
    }

    let loaded = engine.try_load_state(&block, 0).unwrap();
    assert_eq!(loaded.consumed, block.len());
    assert_eq!(loaded.rom_name, "nestest.nes");
    assert_eq!(engine.frame_count(), 1_234);

    let mut after = Vec::new();
    engine.pixel_frame_indices_into(&mut after);
    assert_eq!(after, before, "the picture did not come back");
    assert_ne!(
        engine.read_memory(0x0010),
        0x5A,
        "the sentinel survived, so the load did nothing"
    );

    // The block may sit inside a larger buffer, and `consumed` says where the host's own data
    // starts (REQ-CORE-091).
    let mut padded = vec![0xCC_u8; 8];
    padded.extend_from_slice(&block);
    padded.extend_from_slice(b"host data");
    let loaded = engine.try_load_state(&padded, 8).unwrap();
    assert_eq!(loaded.consumed, block.len());
}

/// REQ-CORE-120: a block of the other engine is refused as a *foreign core*, with the message the
/// requirement fixes to the character — not as a corrupt one.
#[test]
fn a_zaxnes_state_is_refused_as_a_foreign_core() {
    let _guard = lock();
    let Some(mut engine) = running(10) else {
        return;
    };

    // Zaxnes's magic followed by nothing in particular: the magic alone must settle it, because a
    // real block of the other engine is never otherwise readable here.
    let mut foreign = state_magic(EngineKind::Zaxnes).to_le_bytes().to_vec();
    foreign.extend_from_slice(&[0_u8; 64]);

    let refused = engine.try_load_state(&foreign, 0).unwrap_err();
    assert_eq!(refused.failure, StateLoadFailure::ForeignCore);
    assert_eq!(refused.message, FOREIGN_CORE_MESSAGE);
    assert_eq!(refused.message, "Save state used a different core.");
}

/// REQ-CORE-091, REQ-CORE-005: a corrupt or truncated block is refused and leaves the engine
/// stopped rather than half restored.
#[test]
fn a_corrupt_block_is_refused_and_leaves_the_engine_stopped() {
    let _guard = lock();
    let Some(mut engine) = running(10) else {
        return;
    };

    for (label, block) in [
        ("empty", Vec::new()),
        ("wrong magic", vec![1, 2, 3, 4, 5, 6, 7, 8]),
        (
            "truncated after the magic",
            state_magic(EngineKind::Fceux).to_le_bytes().to_vec(),
        ),
    ] {
        let refused = engine.try_load_state(&block, 0).unwrap_err();
        assert_eq!(
            refused.failure,
            StateLoadFailure::CorruptState,
            "{label} should be corrupt, not {:?}",
            refused.failure
        );
    }
    assert!(!engine.is_running(), "a refused block left a ROM loaded");

    // An offset past the end of the buffer is a refusal, not a panic.
    let refused = engine.try_load_state(&[0_u8; 4], 99).unwrap_err();
    assert_eq!(refused.failure, StateLoadFailure::CorruptState);
}

/// REQ-CORE-015, REQ-CORE-016: the region can be chosen after a load, and `Multi` — which FCEUX
/// has no equivalent of — round-trips rather than silently becoming NTSC.
#[test]
fn the_region_can_be_chosen_and_multi_round_trips() {
    let _guard = lock();
    let Some(mut engine) = running(0) else { return };

    for region in [NesRegion::Pal, NesRegion::Dendy, NesRegion::Ntsc] {
        engine.set_region(region);
        assert_eq!(engine.region(), region);
    }

    engine.set_region(NesRegion::Multi);
    assert_eq!(
        engine.region(),
        NesRegion::Multi,
        "Multi became something else, so a round trip through the host's setting loses it"
    );
    // And a frame's reported cycles are NTSC's, because Multi *is* NTSC timing (REQ-CORE-015).
    assert_eq!(engine.run_frame(0), 29_781);
    engine.set_region(NesRegion::Pal);
    assert_eq!(engine.run_frame(0), 33_248);
}

/// REQ-CORE-024: two ports, a pad on each, and no Zapper on this engine — refused rather than
/// accepted and ignored.
#[test]
fn both_ports_hold_a_pad_and_a_zapper_is_refused() {
    let _guard = lock();
    let Some(mut engine) = running(0) else { return };

    for port in 0..2 {
        assert_eq!(engine.port_type(port), Ok(ControllerPortType::Controller));
        assert_eq!(
            engine.set_port_type(port, ControllerPortType::Controller),
            Ok(())
        );
        assert!(engine
            .set_port_type(port, ControllerPortType::Zapper)
            .is_err());
    }
    assert!(engine.port_type(2).is_err());
    assert!(engine
        .set_port_type(2, ControllerPortType::Controller)
        .is_err());
}

/// §2 and §4 of the design note: the flags the interface reads, and the reason it shows.
#[test]
fn the_capabilities_say_what_fceux_cannot_do_and_why() {
    let _guard = lock();
    let Some(engine) = running(0) else { return };

    let capabilities = engine.capabilities();
    assert_eq!(capabilities.kind, EngineKind::Fceux);
    assert_eq!(engine.kind(), EngineKind::Fceux);

    for capability in [
        Capability::Widescreen,
        Capability::HackPacks,
        Capability::SpriteLimit,
        Capability::RunAhead,
        Capability::FastCoreSnapshot,
        Capability::MachineInternals,
    ] {
        assert!(
            !capabilities.has(capability),
            "{capability:?} should be off"
        );
        let reason = capabilities
            .refusal(capability)
            .expect("a capability that is off must say why");
        assert!(reason.len() > 20, "{capability:?}: {reason:?}");
    }

    // Rollback is the one that works, over the serialised block rather than a fast snapshot.
    assert!(capabilities.has(Capability::Rollback));
    assert_eq!(capabilities.refusal(Capability::Rollback), None);
}

/// The downcast is how a caller reaches what is off the trait, and it must answer honestly for
/// the wrong engine rather than panicking.
#[test]
fn the_downcast_finds_the_concrete_engine_and_nothing_else() {
    let _guard = lock();
    let Some(mut engine) = running(0) else { return };

    let handle: &mut dyn Engine = &mut engine;
    assert!(handle.as_any().downcast_ref::<FceuxEngine>().is_some());
    assert!(handle.as_any().downcast_ref::<String>().is_none());
    assert!(handle.as_any_mut().downcast_mut::<FceuxEngine>().is_some());
}

/// Step 5's requirement, and the reason it is a requirement: the engine's own default fill is
/// `zax-nes`'s zero, which `.fm2` playback must be able to override (lot FCEUX-4).
#[test]
fn the_default_ram_fill_is_zero_and_stays_settable() {
    let _guard = lock();
    let Some(rom) = nestest() else { return };
    let image = std::fs::read(&rom).unwrap();

    let mut engine = FceuxEngine::new().unwrap();
    assert!(engine.load_rom(&image, "nestest.nes").success);
    // `$0010` is untouched by nestest's first frames, so it still holds the power-on fill.
    assert_eq!(engine.read_memory(0x0010), 0x00);

    engine.set_ram_init(RamInit::Ff).unwrap();
    engine.power();
    assert_eq!(engine.read_memory(0x0010), 0xFF);

    engine.set_ram_init(RamInit::Zero).unwrap();
    engine.power();
    assert_eq!(engine.read_memory(0x0010), 0x00);
}

/// REQ-CORE-095: the battery RAM the fork's three new C calls reach. `nestest` has no battery, so
/// what this pins is the honest answer for a cart without one — and that an import of the wrong
/// length is refused rather than half applied.
#[test]
fn a_cart_without_a_battery_exports_nothing_and_refuses_an_import() {
    let _guard = lock();
    let Some(mut engine) = running(10) else {
        return;
    };

    let mut battery = vec![0xAA_u8; 16];
    engine.export_battery_ram(&mut battery);
    assert_eq!(
        battery.len(),
        0,
        "nestest has no battery, so there is nothing to export"
    );
    assert!(!engine.import_battery_ram(&[0_u8; 8192]));
    assert!(!engine.import_battery_ram(&[]));
}

/// REQ-CORE-005: with no ROM, nothing advances and nothing is invented.
#[test]
fn with_no_rom_a_frame_does_nothing() {
    let _guard = lock();
    let mut engine = FceuxEngine::new().unwrap();

    assert!(!engine.is_running());
    assert_eq!(engine.run_frame(0), 0);
    assert_eq!(engine.frame_count(), 0);
    assert_eq!(engine.frame_batch().records.len(), 0);
    assert!(engine.save_state_to(true, &mut Vec::new()).is_err());
}
