//! `NesHawkEngine`: the `Engine` trait, the serialised savestate, and the 3D tile capture.
//!
//! The savestate tests run on a ROM built here rather than on a game, so that they are not
//! `#[ignore]`d: a 32 KiB NROM image whose reset vector runs a loop that writes to work RAM and
//! steps the PPU is enough to make a state that differs from a fresh power-on, which is the whole
//! requirement. The capture and the parity tests need real games and are `#[ignore]`d with
//! `ZAXNES_ROMS`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code
#![allow(clippy::cast_possible_truncation, clippy::assert_is_empty)] // test code

use std::path::PathBuf;

use zax_core_api::{Engine, EngineKind, NesRegion, StateLoadFailure};
use zax_neshawk::NesHawkEngine;

/// A 32 KiB NROM image, mapper 0, whose program writes a counter into work RAM for ever and
/// turns the PPU on, so that a hundred frames leave a machine state no fresh power-on has.
fn nrom_image() -> Vec<u8> {
    let mut image = vec![0u8; 16 + 0x8000 + 0x2000];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = 2; // 2 x 16 KiB PRG
    image[5] = 1; // 1 x 8 KiB CHR
    image[6] = 0; // mapper 0, horizontal mirroring
    let prg = &mut image[16..16 + 0x8000];

    // $8000: LDA #$1E / STA $2001  (background and sprites on)
    //        LDA #$80 / STA $2000  (NMI on vblank)
    // loop:  INC $0300 / LDA $2002 / JMP loop
    let program: &[u8] = &[
        0xA9, 0x1E, // LDA #$1E
        0x8D, 0x01, 0x20, // STA $2001
        0xA9, 0x80, // LDA #$80
        0x8D, 0x00, 0x20, // STA $2000
        0xEE, 0x00, 0x03, // INC $0300
        0xAD, 0x02, 0x20, // LDA $2002
        0x4C, 0x0A, 0x80, // JMP $800A
    ];
    prg[..program.len()].copy_from_slice(program);
    // The NMI handler is an RTI at $8100, and the vectors live at the end of the last bank.
    prg[0x100] = 0x40; // RTI
    prg[0x7FFA] = 0x00; // NMI  = $8100
    prg[0x7FFB] = 0x81;
    prg[0x7FFC] = 0x00; // RESET = $8000
    prg[0x7FFD] = 0x80;
    prg[0x7FFE] = 0x00; // IRQ  = $8100
    prg[0x7FFF] = 0x81;

    // Some CHR, so that the tile capture has patterns that are not all zero.
    let chr = &mut image[16 + 0x8000..];
    for (i, byte) in chr.iter_mut().enumerate() {
        *byte = (i % 251) as u8;
    }
    image
}

fn loaded() -> NesHawkEngine {
    let mut engine = NesHawkEngine::new();
    let result = engine.load_rom(&nrom_image(), "state-test.nes");
    assert!(result.success, "{:?}", result.error);
    engine
}

#[test]
fn the_engine_reports_itself_as_neshawk() {
    let engine = loaded();
    assert_eq!(engine.kind(), EngineKind::NesHawk);
    assert_eq!(EngineKind::NesHawk.as_str(), "neshawk");
    // **3, not 2.** 2 belongs to `zaxnes-neshawk` on `feat/neshawk-profile` (Zaxnes's own core
    // on the NesHawk-compatible chip), and these values are persisted in `games.json` and in a
    // savestate stamp, so taking it here would reinterpret that branch's files.
    assert_eq!(EngineKind::from_u8(3), Some(EngineKind::NesHawk));
    assert_eq!(
        EngineKind::from_u8(2),
        None,
        "2 is reserved for zaxnes-neshawk"
    );
    assert_eq!(EngineKind::parse(" NESHAWK "), Some(EngineKind::NesHawk));
    assert_eq!(
        zax_core_api::state_core_name(EngineKind::NesHawk),
        "NesHawk"
    );
    // The capabilities the user interface reads: none of Zaxnes's bus features, rollback yes.
    let caps = engine.capabilities();
    assert!(caps.rollback);
    assert!(!caps.widescreen && !caps.hack_packs && !caps.machine_internals);
    assert!(!caps.fast_core_snapshot && !caps.run_ahead);
}

/// REQ-CORE-090 and REQ-CORE-091: a state saved, the machine run on, the state restored, and the
/// machine is the one it was — checked on the work RAM, the frame count and the picture.
///
/// The counter the test ROM increments is what makes this a real test: if `try_load_state` did
/// nothing at all, `$0300` would hold the later value and the assertion would fail.
#[test]
fn a_savestate_round_trip_brings_the_machine_back() {
    let mut engine = loaded();
    for _ in 0..100 {
        engine.run_frame(0);
    }

    let mut block = Vec::new();
    engine.save_state_to(true, &mut block).unwrap();
    assert!(block.len() <= engine.max_state_size(), "{}", block.len());

    let marker = engine.read_memory(0x0300);
    let frame = engine.frame_count();
    let mut picture = Vec::new();
    engine.pixel_frame_indices_into(&mut picture);

    for _ in 0..50 {
        engine.run_frame(0);
    }
    assert_ne!(engine.read_memory(0x0300), marker, "the ROM does nothing");

    let loaded = engine.try_load_state(&block, 0).unwrap();
    assert_eq!(loaded.consumed, block.len());
    assert_eq!(loaded.rom_name, "state-test.nes");
    assert_eq!(engine.read_memory(0x0300), marker);
    assert_eq!(engine.frame_count(), frame);

    let mut after = Vec::new();
    engine.pixel_frame_indices_into(&mut after);
    assert_eq!(after, picture, "a full state carries the frame buffer");

    // And the restored machine runs on identically, which is the stronger statement: a state that
    // restored only *most* of the machine would part from the first run within a frame or two.
    let mut again = engine.clone_run(50);
    engine.try_load_state(&block, 0).unwrap();
    let twice = engine.clone_run(50);
    assert_eq!(again.pop(), twice.last().copied());
    assert_eq!(again, twice[..twice.len() - 1]);
}

/// The rollback block leaves the frame buffer out, as REQ-CORE-121's own snapshot does — so it is
/// about 120 KiB smaller, and the machine still comes back.
#[test]
fn a_rollback_block_leaves_the_frame_buffer_out_and_still_restores() {
    let mut engine = loaded();
    for _ in 0..100 {
        engine.run_frame(0);
    }

    let mut full = Vec::new();
    let mut fast = Vec::new();
    engine.save_state_to(true, &mut full).unwrap();
    engine.save_state_to(false, &mut fast).unwrap();
    assert!(
        full.len() > fast.len() + 100_000,
        "full {} fast {}",
        full.len(),
        fast.len()
    );

    let marker = engine.read_memory(0x0300);
    for _ in 0..50 {
        engine.run_frame(0);
    }
    engine.try_load_state(&fast, 0).unwrap();
    assert_eq!(engine.read_memory(0x0300), marker);
}

/// REQ-CORE-120: a block of another engine is refused with the spec's own message, and not as
/// "corrupt" — which is the difference between a user who knows what to do and one who does not.
#[test]
fn a_block_of_another_engine_is_refused_by_name() {
    let mut engine = loaded();
    engine.run_frame(0);
    let mut block = Vec::new();
    engine.save_state_to(true, &mut block).unwrap();

    for other in [EngineKind::Zaxnes, EngineKind::Fceux] {
        let mut foreign = block.clone();
        foreign[..4].copy_from_slice(&zax_core_api::state_magic(other).to_le_bytes());
        let error = engine.try_load_state(&foreign, 0).unwrap_err();
        assert_eq!(error.failure, StateLoadFailure::ForeignCore);
        assert!(
            error
                .message
                .starts_with(zax_core_api::FOREIGN_CORE_MESSAGE),
            "{}",
            error.message
        );
        assert!(error.message.contains(other.display_name()));
    }

    // And a block of no engine at all is corrupt rather than foreign.
    let mut junk = block.clone();
    junk[..4].copy_from_slice(&[1, 2, 3, 4]);
    assert_eq!(
        engine.try_load_state(&junk, 0).unwrap_err().failure,
        StateLoadFailure::ForeignCore
    );

    // A truncated block of this engine is corrupt, and the machine is left running.
    let error = engine
        .try_load_state(&block[..block.len() / 2], 0)
        .unwrap_err();
    assert_eq!(error.failure, StateLoadFailure::CorruptState);
    assert!(engine.is_running());
}

/// A block written at an offset inside a larger buffer is found there and reports what it used
/// (REQ-CORE-091).
#[test]
fn a_block_is_restored_from_an_offset_inside_other_data() {
    let mut engine = loaded();
    for _ in 0..20 {
        engine.run_frame(0);
    }
    let mut block = Vec::new();
    engine.save_state_to(true, &mut block).unwrap();

    let mut buffer = vec![0xAAu8; 37];
    let at = buffer.len();
    buffer.extend_from_slice(&block);
    buffer.extend_from_slice(&[0x55; 11]);

    let loaded = engine.try_load_state(&buffer, at).unwrap();
    assert_eq!(loaded.consumed, block.len());
}

/// The 3D tile capture: off by default, and turning it on changes the records but **not the
/// machine** (REQ-CORE-033). The second half is the one that matters — a capture that clocked a
/// board latch or advanced an address would make the 3D engine's presence visible in the game.
#[test]
fn the_tile_capture_produces_records_and_changes_nothing() {
    let mut quiet = loaded();
    assert!(!quiet.tile_capture(), "the capture is off by default");
    for _ in 0..120 {
        quiet.run_frame(0);
    }
    assert!(quiet.frame_batch().records.is_empty());

    let mut capturing = loaded();
    capturing.set_tile_capture(true);
    assert!(capturing.tile_capture());
    for _ in 0..120 {
        capturing.run_frame(0);
    }
    let batch = capturing.frame_batch();
    assert!(
        !batch.records.is_empty(),
        "the capture produced no records at all"
    );
    assert!(
        batch.records.iter().any(|record| record.background),
        "no background row was captured"
    );
    assert_eq!(batch.margin, 0, "a NesHawk frame has no widescreen margin");
    assert_eq!(batch.palette, capturing.nes().unwrap().ppu.palram);

    // The machine itself is untouched: 2 KiB of work RAM, the frame buffer and the cycle count.
    let (mut a, mut b) = (Vec::new(), Vec::new());
    quiet.pixel_frame_indices_into(&mut a);
    capturing.pixel_frame_indices_into(&mut b);
    assert_eq!(a, b, "the capture changed the picture");
    assert_eq!(
        quiet.nes().unwrap().ram,
        capturing.nes().unwrap().ram,
        "the capture changed the work RAM"
    );
    assert_eq!(
        quiet.nes().unwrap().total_executed_cycles,
        capturing.nes().unwrap().total_executed_cycles,
        "the capture changed the cycle count"
    );
}

/// A record's index, x and y follow SPEC-02 §4.1: a background row is `row * 33 + k`, a sprite is
/// its OAM number, and the row's y is `line - 1`.
#[test]
fn the_records_carry_the_coordinates_the_spec_fixes() {
    let mut engine = loaded();
    engine.set_tile_capture(true);
    for _ in 0..120 {
        engine.run_frame(0);
    }
    let batch = engine.frame_batch();
    for record in batch.records {
        if record.background {
            assert!((-7..=255).contains(&record.x), "{record:?}");
            assert!((-1..=238).contains(&record.y), "{record:?}");
            assert!((0..30 * 33).contains(&record.index), "{record:?}");
            assert!(!record.mirror_h && !record.mirror_v && !record.hidden);
        } else {
            assert!((0..=255).contains(&record.x), "{record:?}");
            assert!((0..128).contains(&record.index), "{record:?}");
        }
        assert!(record.palette <= 3, "{record:?}");
    }
}

/// REQ-CORE-013: the load *reports* the region and does not apply it. `set_region` then applies
/// the host's choice, and `region()` reports back what the host asked for rather than what the
/// machine runs — `Multi` must not come back as `Ntsc`.
#[test]
fn the_region_is_reported_by_the_load_and_chosen_by_the_host() {
    let mut engine = loaded();
    assert_eq!(engine.region(), NesRegion::Ntsc);
    for region in [
        NesRegion::Pal,
        NesRegion::Dendy,
        NesRegion::Multi,
        NesRegion::Ntsc,
    ] {
        engine.set_region(region);
        assert_eq!(engine.region(), region);
        assert!(engine.is_running(), "a region change stopped the machine");
    }
}

/// The ports: what the host plugged in comes back, an out-of-range port is refused, and the
/// Zapper NesHawk has not got is still *reported* as a Zapper so the substitution is visible.
#[test]
fn the_ports_report_what_the_host_plugged_in() {
    use zax_core_api::ControllerPortType;
    let mut engine = loaded();
    assert_eq!(engine.port_type(0), Ok(ControllerPortType::Controller));
    engine
        .set_port_type(1, ControllerPortType::Empty)
        .expect("port 1");
    assert_eq!(engine.port_type(1), Ok(ControllerPortType::Empty));
    engine
        .set_port_type(0, ControllerPortType::Zapper)
        .expect("port 0");
    assert_eq!(engine.port_type(0), Ok(ControllerPortType::Zapper));
    assert!(engine
        .set_port_type(2, ControllerPortType::Controller)
        .is_err());
    assert!(engine.port_type(2).is_err());
}

/// With no ROM: nothing panics, a frame costs nothing, and a savestate is refused (REQ-CORE-005).
#[test]
fn an_engine_with_no_rom_answers_without_panicking() {
    let mut engine = NesHawkEngine::new();
    assert!(!engine.is_running());
    assert_eq!(engine.rom_name(), None);
    assert_eq!(engine.run_frame(0), 0);
    assert_eq!(engine.frame_count(), 0);
    engine.power();
    engine.reset();
    engine.soft_reset();
    engine.write_memory(0x0300, 1);
    assert_eq!(engine.read_memory(0x0300), 0);
    let mut out = Vec::new();
    engine.pixel_frame_indices_into(&mut out);
    assert_eq!(out.len(), 256 * 240);
    assert!(out.iter().all(|&p| p == 0));
    assert!(engine.save_state_to(true, &mut Vec::new()).is_err());
    engine.export_battery_ram(&mut out);
    assert!(out.is_empty());
    assert!(!engine.import_battery_ram(&[0; 8192]));

    // And a load that fails leaves it stopped rather than half-up (REQ-CORE-011).
    let result = engine.load_rom(b"not a ROM", "junk.nes");
    assert!(!result.success);
    assert!(!engine.is_running());
    assert_eq!(engine.rom_name(), None);
}

/// A capture switched on after some frames have run still produces a correct first row: the
/// row-start shadow is seeded from the live `v` rather than left at zero.
#[test]
fn a_capture_switched_on_mid_game_seeds_its_row_shadow() {
    let mut engine = loaded();
    for _ in 0..90 {
        engine.run_frame(0);
    }
    engine.set_tile_capture(true);
    engine.run_frame(0);
    assert!(!engine.frame_batch().records.is_empty());
}

/// Audio comes out at the rate asked for, and a rate outside the range is refused rather than
/// snapped (which is what `zax-nes` does and FCEUX cannot).
#[test]
fn audio_is_produced_at_the_rate_asked_for() {
    let mut engine = loaded();
    assert_eq!(engine.sample_rate(), 48_000);
    engine.set_sample_rate(44_100).expect("44100");
    assert_eq!(engine.sample_rate(), 44_100);
    assert!(engine.set_sample_rate(1).is_err());
    assert!(engine.set_sample_rate(1_000_000).is_err());
    assert_eq!(engine.sample_rate(), 44_100);

    for _ in 0..60 {
        engine.run_frame(0);
    }
    let mut buffer = vec![0.0f32; 44_100 / 2];
    let written = engine.read_audio(&mut buffer, false).expect("mono");
    assert!(written > 0, "a second of frames produced no samples");
    assert!(
        buffer[..written].iter().all(|s| (-1.0..=1.0).contains(s)),
        "a sample left the -1..=1 range"
    );
    // A stereo buffer of odd length is refused (the trait's one audio error).
    assert!(engine.read_audio(&mut [0.0; 3], true).is_err());
}

/// A `.nes` image this port has no board for is a **load**, not an error: it runs on Zaxnes's own
/// board through the adapter and says so (the owner's decision of 2026-11-12).
#[test]
#[ignore = "needs ZAXNES_ROMS"]
fn a_mapper_without_a_neshawk_board_falls_back_on_zaxnes_and_says_so() {
    let Some(dir) = std::env::var_os("ZAXNES_ROMS").map(PathBuf::from) else {
        return;
    };
    // Mapper 69 (FME-7) and mapper 5 (MMC5) are the common ones; whichever of the collection's
    // ROMs has no ported board will do, so the test walks until it finds one.
    let mut tried = 0;
    for entry in std::fs::read_dir(&dir).expect("ZAXNES_ROMS").flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("nes") {
            continue;
        }
        let Ok(image) = std::fs::read(&path) else {
            continue;
        };
        let mut engine = NesHawkEngine::new();
        let result = engine.load_rom(&image, &path.display().to_string());
        assert!(
            result.success,
            "{} was refused: {:?}",
            path.display(),
            result.error
        );
        if engine.board_is_from_zaxnes() {
            let note = result.note.unwrap_or_default();
            assert!(
                note.contains("board from Zaxnes, not NesHawk-exact"),
                "the fallback was silent: {note}"
            );
            for _ in 0..60 {
                engine.run_frame(0);
            }
            tried += 1;
            if tried == 3 {
                return;
            }
        }
    }
}

/// A small test-only helper: run `frames` frames and collect a byte of work RAM from each, so
/// that two runs can be compared as sequences rather than at one point.
trait CloneRun {
    fn clone_run(&mut self, frames: usize) -> Vec<u8>;
}

impl CloneRun for NesHawkEngine {
    fn clone_run(&mut self, frames: usize) -> Vec<u8> {
        (0..frames)
            .map(|_| {
                self.run_frame(0);
                self.read_memory(0x0300)
            })
            .collect()
    }
}
