//! `EngineHandle`, the one way the application holds an engine (lot FCEUX-23 step 4).
//!
//! What this pins is the façade's own contract: the trait's methods arrive through `Deref`, the
//! Zaxnes-only ones are inherent and each has a stated answer on another engine, and the
//! capability flags say which is which. The *cost* of the abstraction is the last test, which is
//! `#[ignore]`d because it needs a commercial ROM and takes a minute.

// A test: an unexpected error should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use zax_app::emulation::engine_handle::EngineHandle;
use zax_core_api::{Capabilities, Capability, EngineKind, NesRegion};

/// A minimal NROM image whose reset vector points at a `JMP` to itself.
fn image() -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + 16 * 1024 + 8 * 1024];
    rom[..4].copy_from_slice(b"NES\x1a");
    rom[4] = 1;
    rom[5] = 1;
    let prg = 16;
    rom[prg] = 0x4C;
    rom[prg + 1] = 0x00;
    rom[prg + 2] = 0xC0;
    rom[prg + 0x3FFC] = 0x00;
    rom[prg + 0x3FFD] = 0xC0;
    rom
}

fn loaded() -> EngineHandle {
    let mut handle = EngineHandle::zaxnes_core();
    let result = handle.load_rom(&image(), "handle.nes");
    assert!(result.success, "{:?}", result.error);
    handle
}

/// A ROM of `ZAXNES_ROMS`, or `None` after saying what is missing.
fn original(name: &str) -> Option<PathBuf> {
    let root = std::env::var("ZAXNES_ROMS").ok()?;
    let rom = PathBuf::from(root).join(name);
    if rom.is_file() {
        return Some(rom);
    }
    eprintln!("skipped: {} is missing", rom.display());
    None
}

/// With `fceux-core` off the only engine is Zaxnes's, and the handle says so — which is what
/// makes every fallback below dead at run time in the default build.
#[test]
fn the_default_build_holds_zaxnes_and_every_capability_is_on() {
    let handle = loaded();

    assert_eq!(handle.kind(), EngineKind::Zaxnes);
    assert_eq!(handle.capabilities(), Capabilities::ZAXNES);
    assert!(handle.zaxnes().is_some());
    for capability in [
        Capability::Widescreen,
        Capability::HackPacks,
        Capability::SpriteLimit,
        Capability::RunAhead,
        Capability::Rollback,
        Capability::FastCoreSnapshot,
        Capability::MachineInternals,
    ] {
        assert!(handle.has(capability));
        assert_eq!(handle.refusal(capability), None);
    }
}

/// The trait's methods reach the engine through `Deref`, so the call sites did not change.
#[test]
fn the_traits_methods_arrive_through_deref() {
    let mut handle = loaded();

    assert!(handle.is_running());
    assert_eq!(handle.rom_name(), Some("handle.nes"));
    assert_eq!(handle.region(), NesRegion::Ntsc);
    assert_eq!(handle.frame_count(), 0);
    handle.run_frame(0);
    assert_eq!(handle.frame_count(), 1);
    handle.write_memory(0x0010, 0xA5);
    assert_eq!(handle.read_memory(0x0010), 0xA5);
}

/// The Zaxnes-only ones are inherent, and each gives the core's own answer here.
#[test]
fn the_zaxnes_only_methods_give_the_cores_own_answers() {
    let mut handle = loaded();
    handle.run_frame_ahead(0, 1);

    assert!(handle.holds_a_real_frame());
    assert!(handle.restore_real_frame());

    let mut picture = Vec::new();
    assert_eq!(handle.wide_picture_into(&mut picture), 240);
    assert_eq!(picture.len(), 256 * 240);
    assert!(handle.pixel_frame_with_mask().is_some());
    assert!(handle.bus().is_some());
    assert!(handle.ppu().is_some());
    assert!(handle.cartridge().is_some());
    assert!(handle.apu().is_some());
    assert!(handle.audio_reader().is_some());
    assert_eq!(handle.widescreen_margin(), 0);
    assert_eq!(handle.hack_options().len(), 0);
    assert_eq!(handle.builtin_patch(), None);
    assert!(handle.max_core_state_size() > 0);
    assert!((handle.speed() - 1.0).abs() < f64::EPSILON);
}

/// The timing and the cycle count are derived rather than asked for, so both engines answer
/// them; the derivation must still agree with the core's own.
#[test]
fn the_timing_follows_the_region_and_the_cycles_follow_the_frames() {
    let mut handle = loaded();

    assert_eq!(handle.timing().nominal_frames_per_second, 60);
    handle.set_region(NesRegion::Pal);
    assert_eq!(handle.timing().nominal_frames_per_second, 50);
    handle.set_region(NesRegion::Ntsc);

    for _ in 0..4 {
        handle.run_frame(0);
    }
    let core = handle.zaxnes().expect("Zaxnes's core");
    assert_eq!(handle.cycle_count(), core.cycle_count());
    assert!(handle.cycle_count() > 0);
}

/// The work-RAM snapshot the scripts read from while the emulation thread has the engine: the
/// façade's copy must be the bus's own bytes.
#[test]
fn the_work_ram_snapshot_is_the_buses_own_bytes() {
    let mut handle = loaded();
    handle.write_memory(0x0010, 0xA5);
    handle.write_memory(0x07FF, 0x5A);

    let mut snapshot = [0_u8; 2048];
    handle.work_ram_into(&mut snapshot);
    assert_eq!(snapshot[0x0010], 0xA5);
    assert_eq!(snapshot[0x07FF], 0x5A);
    assert_eq!(
        &snapshot[..],
        handle.bus().expect("Zaxnes's bus").ram(),
        "the snapshot and the bus parted"
    );
}

/// The savestate the slots write goes through the trait, and the handle's `save_state` is the
/// core's own bytes.
#[test]
fn a_savestate_through_the_handle_is_the_cores_own_block() {
    let mut handle = loaded();
    for _ in 0..3 {
        handle.run_frame(0);
    }

    let through_the_handle = handle.save_state(true).unwrap();
    let through_the_core = handle
        .zaxnes()
        .expect("Zaxnes's core")
        .save_state(true)
        .unwrap();
    assert_eq!(
        through_the_handle, through_the_core,
        "the façade changed the block"
    );
    assert_eq!(
        zax_core_api::engine_of_state(&through_the_handle, 0),
        Some(EngineKind::Zaxnes)
    );
}

/// What the abstraction costs, on the path it is actually on.
///
/// `bench-core` cannot answer this: the harness drives `NesCore` **directly**, so it measures
/// `zax-nes` and not the application's handle. This times the same frames both ways — through
/// `EngineHandle` (a `Box<dyn Engine>` with a downcast per Zaxnes-only call) and through
/// `Box<NesCore>` — and prints the two, which is the figure `docs/progress/FCEUX-23.md` step 4
/// records against the owner's +1 %.
///
/// `#[ignore]`d: it needs `ZAXNES_ROMS` and runs 24 000 frames.
#[test]
#[ignore = "needs ZAXNES_ROMS and a minute"]
fn what_the_abstraction_costs_on_the_applications_own_path() {
    use std::time::Instant;
    use zax_core_api::Engine;
    use zax_nes::NesCore;

    const WARMUP: u32 = 120;
    const FRAMES: u32 = 1_000;
    // Many short rounds rather than a few long ones, and the **median** of their ratios: a
    // machine hiccup inside one half of one round was seen to swing a long round by 20 to 40 %,
    // and the quantity being measured is far smaller than that. Twenty-one rounds put the median
    // out of reach of a handful of hiccups.
    const ROUNDS: u32 = 21;

    let Some(rom) = original("Super Mario Bros. 3 (USA).nes").or_else(|| {
        original("Super Mario Bros. 3 [USA][!!].nes").or_else(|| original("Super Mario Bros..nes"))
    }) else {
        return;
    };
    let image = std::fs::read(&rom).unwrap();

    // One round of the application's own per-frame sequence: the frame, then the four Zaxnes-only
    // reads `GameSession::capture_in_flight` makes of it.
    let mut handle_total = 0.0_f64;
    let mut direct_total = 0.0_f64;
    let mut per_round: Vec<f64> = Vec::new();
    for round in 0..ROUNDS {
        let mut picture = Vec::new();
        let mut overlay = Vec::new();

        let mut handle = EngineHandle::zaxnes_core();
        assert!(handle.load_rom(&image, "bench.nes").success);
        for _ in 0..WARMUP {
            handle.run_frame_ahead(0, 0);
        }
        let started = Instant::now();
        for _ in 0..FRAMES {
            handle.run_frame_ahead(0, 0);
            handle.wide_picture_into(&mut picture);
            let _ = handle.picture_shift();
            let _ = handle.picture_motion();
            handle.split_steady_sprites(&mut picture, &mut overlay);
        }
        let through_handle = started.elapsed().as_secs_f64();
        drop(handle);

        let mut core = Box::new(NesCore::new());
        assert!(Engine::load_rom(core.as_mut(), &image, "bench.nes").success);
        for _ in 0..WARMUP {
            core.run_frame_ahead(0, 0);
        }
        let started = Instant::now();
        for _ in 0..FRAMES {
            core.run_frame_ahead(0, 0);
            core.wide_picture_into(&mut picture);
            let _ = core.picture_shift();
            let _ = core.picture_motion();
            core.split_steady_sprites(&mut picture, &mut overlay);
        }
        let direct = started.elapsed().as_secs_f64();

        handle_total += through_handle;
        direct_total += direct;
        per_round.push(100.0 * (through_handle / direct - 1.0));
        println!(
            "round {round:>2}: EngineHandle {:.4} ms/frame, Box<NesCore> {:.4} ms/frame, {:+.3} %",
            1_000.0 * through_handle / f64::from(FRAMES),
            1_000.0 * direct / f64::from(FRAMES),
            100.0 * (through_handle / direct - 1.0)
        );
    }

    // The **median** of the rounds, not the total: one round in a handful lands on a machine
    // hiccup — a 20 % outlier was seen — and a mean carries it straight into the answer.
    per_round.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let median = per_round[per_round.len() / 2];
    let low = per_round[per_round.len() / 4];
    let high = per_round[3 * per_round.len() / 4];
    let total = 100.0 * (handle_total / direct_total - 1.0);
    println!(
        "EngineHandle against Box<NesCore> over {} frames each: median {median:+.3} %,          quartiles {low:+.3} % to {high:+.3} %, all rounds together {total:+.3} %",
        ROUNDS * FRAMES
    );
    // Not an assertion on the figure — a machine under load would fail it for the wrong reason.
    // The number is printed and recorded in the progress note, which is where the owner's +1 %
    // is judged.
}
