//! The in-memory state of the core (AMD-041 REQ-CORE-121): a complete save and restore that
//! allocates nothing per frame, leaves the audio already produced alone, and leaves the frame
//! the host has not finished reading where it is.

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use common::{machine_with, minimal_rom, test_rom, TestRandom};
use zax_nes::state::CoreState;
use zax_nes::NesCore;

/// A ROM whose loop stirs work RAM, so that a state difference shows.
const COUNTER_PROGRAM: [u8; 10] = [
    0xE6, 0x20, // INC $20
    0xA5, 0x20, // LDA $20
    0x8D, 0x00, 0x03, // STA $0300
    0x4C, 0x00, 0x80, // JMP $8000
];

fn counter_core() -> NesCore {
    machine_with(
        &minimal_rom(0x01, Some(&COUNTER_PROGRAM), None),
        "State.nes",
    )
}

/// The input word of frame `frame` of the acceptance run: the same sequence both times, and not
/// a constant, so that the inputs really are part of what is compared.
fn input_of(frame: usize) -> u16 {
    #[allow(clippy::cast_possible_truncation)] // A deliberate 16-bit pattern.
    {
        ((frame * 2_654_435_761) & 0xFFFF) as u16
    }
}

/// A 64-bit digest of `bytes` (FNV-1a), to compare 100 pictures without keeping them.
fn digest(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The picture, the tile records and the work RAM of the frame just run, as one digest.
fn frame_digest(core: &NesCore) -> u64 {
    let mut blob = core.pixel_frame_with_mask().to_vec();
    for record in core.frame_batch().records {
        blob.extend_from_slice(&record.plane0.to_le_bytes());
        blob.extend_from_slice(&record.plane1.to_le_bytes());
        blob.extend_from_slice(&record.x.to_le_bytes());
        blob.extend_from_slice(&record.y.to_le_bytes());
        blob.extend_from_slice(&record.index.to_le_bytes());
        blob.push(record.palette);
        blob.push(u8::from(record.background));
    }
    blob.extend_from_slice(core.bus().ram());
    digest(&blob)
}

/// Runs `frames` frames with the inputs of [`input_of`], returning the digest of each one.
fn run_and_digest(core: &mut NesCore, frames: usize) -> Vec<u64> {
    (0..frames)
        .map(|frame| {
            core.run_frame(input_of(frame));
            frame_digest(core)
        })
        .collect()
}

/// REQ-CORE-121, the acceptance criterion: save, emulate 100 frames, restore, emulate the same
/// 100 frames with the same inputs — the pictures and the RAM are identical byte for byte.
#[test]
fn req_core_121_a_hundred_frames_run_the_same_way_after_a_restore() {
    let mut core = counter_core();
    // Past the PPU warm-up, so the hundred frames are ordinary ones.
    for _ in 0..4 {
        core.run_frame(0);
    }

    let mut state = CoreState::default();
    core.capture_core_state(&mut state).unwrap();
    let first = run_and_digest(&mut core, 100);

    core.restore_core_state(&state).unwrap();
    let second = run_and_digest(&mut core, 100);

    assert_eq!(100, first.len());
    assert_eq!(first, second, "the hundred frames did not repeat");
}

/// The same criterion on a real ROM that draws and makes noise, so that the PPU's pipeline, the
/// mapper and the APU are all in the comparison and not just work RAM.
#[test]
fn req_core_121_the_criterion_holds_on_a_public_test_rom() {
    let Some(image) = test_rom("spritecans-2011/spritecans.nes") else {
        return;
    };
    let mut core = machine_with(&image, "spritecans.nes");
    for _ in 0..60 {
        core.run_frame(0);
    }

    let mut state = CoreState::default();
    core.capture_core_state(&mut state).unwrap();
    let first = run_and_digest(&mut core, 100);
    core.restore_core_state(&state).unwrap();
    let second = run_and_digest(&mut core, 100);

    assert_eq!(first, second, "the hundred frames did not repeat");
}

/// REQ-CORE-121: a state restored and captured again gives the very same bytes, which is what
/// "restaurables à l'identique" means for the block itself.
#[test]
fn req_core_121_a_restored_state_captures_to_the_same_bytes() {
    let mut core = counter_core();
    for _ in 0..10 {
        core.run_frame(0);
    }

    let mut first = CoreState::default();
    core.capture_core_state(&mut first).unwrap();
    let bytes = first.as_slice().to_vec();

    for _ in 0..7 {
        core.run_frame(0x0101);
    }
    let mut moved = CoreState::default();
    core.capture_core_state(&mut moved).unwrap();
    assert_ne!(
        bytes,
        moved.as_slice(),
        "seven frames left the state unchanged, so the test proves nothing"
    );

    core.restore_core_state(&first).unwrap();
    let mut again = CoreState::default();
    core.capture_core_state(&mut again).unwrap();
    assert_eq!(bytes, again.as_slice());
}

/// REQ-CORE-121: the frame counter (REQ-CORE-006) is part of the state.
#[test]
fn req_core_121_the_frame_counter_comes_back() {
    let mut core = counter_core();
    for _ in 0..25 {
        core.run_frame(0);
    }
    let mut state = CoreState::default();
    core.capture_core_state(&mut state).unwrap();
    assert_eq!(25, core.frame_count());

    for _ in 0..3 {
        core.run_frame(0);
    }
    assert_eq!(28, core.frame_count());

    core.restore_core_state(&state).unwrap();
    assert_eq!(25, core.frame_count());
}

/// REQ-CORE-121: work RAM, the save RAM and the nametable RAM all come back.
#[test]
fn req_core_121_the_three_memories_come_back() {
    let mut core = machine_with(&minimal_rom(0x03, None, None), "Battery.nes");
    core.run_frame(0);
    core.bus_mut().ram_mut()[0x0123] = 0xA5;
    core.write_memory(0x6000, 0x5A);
    core.write_memory(0x2006, 0x20);
    core.write_memory(0x2006, 0x40);
    core.write_memory(0x2007, 0x3C);

    let mut state = CoreState::default();
    core.capture_core_state(&mut state).unwrap();

    core.bus_mut().ram_mut()[0x0123] = 0x00;
    core.write_memory(0x6000, 0x00);
    core.write_memory(0x2006, 0x20);
    core.write_memory(0x2006, 0x40);
    core.write_memory(0x2007, 0x00);

    core.restore_core_state(&state).unwrap();
    assert_eq!(0xA5, core.bus().ram()[0x0123]);
    assert_eq!(0x5A, core.read_memory(0x6000));
    core.write_memory(0x2006, 0x20);
    core.write_memory(0x2006, 0x40);
    core.ppu_read(7);
    assert_eq!(0x3C, core.ppu_read(7));
}

/// REQ-CORE-121: the mapper's own registers are part of the state — a bank switched after the
/// capture is switched back.
#[test]
fn req_core_121_the_mapper_registers_come_back() {
    let Some(image) = test_rom("mmc3_test_2/rom_singles/1-clocking.nes") else {
        return;
    };
    let mut core = machine_with(&image, "1-clocking.nes");
    for _ in 0..30 {
        core.run_frame(0);
    }

    let mut state = CoreState::default();
    core.capture_core_state(&mut state).unwrap();
    let banked = (0x8000..0x8010)
        .map(|a| core.read_memory(a))
        .collect::<Vec<_>>();

    // A bank switch through the board's own registers.
    core.write_memory(0x8000, 0x06);
    core.write_memory(0x8001, 0x02);
    core.write_memory(0x8000, 0x07);
    core.write_memory(0x8001, 0x03);

    core.restore_core_state(&state).unwrap();
    let restored = (0x8000..0x8010)
        .map(|a| core.read_memory(a))
        .collect::<Vec<_>>();
    assert_eq!(banked, restored, "the PRG window did not come back");
}

/// REQ-CORE-121, last clause: the restore leaves the audio already produced intact, where a
/// savestate restore drops it (REQ-CORE-074, Q-006).
#[test]
fn req_core_121_the_restore_keeps_the_audio_already_produced() {
    let mut core = counter_core();
    // A square at full volume, so that the frames really queue samples.
    core.write_memory(0x4015, 0x01);
    core.write_memory(0x4000, 0xBF);
    core.write_memory(0x4002, 0x40);
    core.write_memory(0x4003, 0x08);
    for _ in 0..3 {
        core.run_frame(0);
    }

    let mut state = CoreState::default();
    core.capture_core_state(&mut state).unwrap();
    core.run_frame(0);
    let queued = core.apu().available_samples();
    assert!(queued > 0, "the fixture produced no sound");

    core.restore_core_state(&state).unwrap();
    assert_eq!(
        queued,
        core.apu().available_samples(),
        "the in-memory restore dropped the samples the player is about to hear"
    );

    // The savestate path still drops them, which is the requirement this one departs from.
    let block = core.save_state(true).unwrap();
    core.run_frame(0);
    assert!(core.apu().available_samples() > 0);
    core.try_load_state(&block, 0).unwrap();
    assert_eq!(0, core.apu().available_samples());
}

/// REQ-CORE-121: the picture and the tile records of the frame the host has not finished reading
/// survive the restore. This is what makes the displayed frame of AMD-041 REQ-LOOP-067 the one
/// the conversion and the 2D picture see, with nothing copied anywhere.
#[test]
fn req_core_121_the_frame_the_host_is_reading_survives_the_restore() {
    let Some(image) = test_rom("spritecans-2011/spritecans.nes") else {
        return;
    };
    let mut core = machine_with(&image, "spritecans.nes");
    for _ in 0..60 {
        core.run_frame(0);
    }

    let mut state = CoreState::default();
    core.capture_core_state(&mut state).unwrap();
    // One more frame: its picture and records are what a host would be reading.
    core.run_frame(0);
    let pixels = core.pixel_frame_with_mask().to_vec();
    let records = core.frame_batch().records.to_vec();
    assert!(!records.is_empty(), "the fixture emitted no tile record");

    core.restore_core_state(&state).unwrap();
    assert_eq!(
        pixels,
        core.pixel_frame_with_mask(),
        "the picture was overwritten"
    );
    assert_eq!(
        records,
        core.frame_batch().records.to_vec(),
        "the tile records were overwritten"
    );
    // And the next frame rewrites them from scratch all the same: `begin_rendered_frame` clears
    // the records at the pre-render line of every frame.
    core.run_frame(0);
    assert_ne!(0, core.frame_batch().records.len());
}

/// REQ-CORE-121 and REQ-ARCH-013: the size is bounded, the ROM image is not in it, and the
/// buffer stops growing after the first capture — run-ahead allocates nothing per frame.
#[test]
fn req_core_121_the_size_is_bounded_and_the_buffer_is_reused() {
    let mut core = counter_core();
    core.run_frame(0);

    let bound = core.max_core_state_size();
    let mut state = CoreState::with_capacity(bound);
    assert!(state.is_empty());

    core.capture_core_state(&mut state).unwrap();
    let length = state.len();
    assert!(length > 0);
    assert!(
        length <= bound,
        "the state is {length} bytes, the announced bound is {bound}"
    );
    // The ROM image is 32 KiB here and is not part of the block.
    let savestate = core.save_state(true).unwrap().len();
    assert!(
        length + 32 * 1024 <= savestate,
        "the in-memory state ({length}) is not smaller than the savestate ({savestate}) by the \
         ROM image"
    );

    // Sixty captures interleaved with frames: the length stays the same and nothing grows.
    for frame in 0..60 {
        core.run_frame(input_of(frame));
        core.capture_core_state(&mut state).unwrap();
        assert_eq!(
            length,
            state.len(),
            "the state's size moved at frame {frame}"
        );
    }
}

/// A state captured and restored over and over leaves the machine on exactly the course it would
/// have taken with no run-ahead at all: the sequence of real states does not depend on N
/// (REQ-ARCH-012 amended).
#[test]
fn req_arch_012_the_real_states_do_not_depend_on_the_captures() {
    let mut plain = counter_core();
    let mut captured = counter_core();
    let mut state = CoreState::default();
    let mut random = TestRandom::new(0x5EED_1234);

    for frame in 0..120 {
        let word = input_of(frame);
        plain.run_frame(word);

        captured.run_frame(word);
        captured.capture_core_state(&mut state).unwrap();
        // Two extra frames with the same input, as run-ahead would run, then back.
        captured.run_frame(word);
        captured.run_frame(word);
        captured.restore_core_state(&state).unwrap();
        // And a read of the random source, so the two runs are not accidentally in step.
        let _ = random.next_u64();

        assert_eq!(
            plain.frame_count(),
            captured.frame_count(),
            "frame {frame}: the counters parted"
        );
        assert_eq!(
            plain.bus().ram(),
            captured.bus().ram(),
            "frame {frame}: the work RAM parted"
        );
    }
}

/// The cost REQ-LOOP-065's note warns about: "le coût de sa sauvegarde et de sa restauration
/// s'ajoute à chaque frame réelle". Reported, not asserted — a wall clock cannot gate anything
/// here, and R0's callgrind run is the real measurement. Run with
/// `--ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "a measurement, reported rather than asserted"]
fn the_cost_of_one_capture_and_one_restore() {
    const ROUNDS: u32 = 20_000;

    let fixtures: [(&str, Option<Vec<u8>>); 3] = [
        ("minimal (NROM, no CHR-ROM)", None),
        (
            "mmc3_test_2/1-clocking.nes (MMC3)",
            test_rom("mmc3_test_2/rom_singles/1-clocking.nes"),
        ),
        (
            "spritecans.nes (a drawn frame)",
            test_rom("spritecans-2011/spritecans.nes"),
        ),
    ];

    for (name, image) in fixtures {
        let mut core = match &image {
            Some(bytes) => machine_with(bytes, name),
            None if name.starts_with("minimal") => counter_core(),
            None => continue,
        };
        for _ in 0..60 {
            core.run_frame(0);
        }

        let mut state = CoreState::with_capacity(core.max_core_state_size());
        core.capture_core_state(&mut state).unwrap();

        let started = std::time::Instant::now();
        for _ in 0..ROUNDS {
            core.capture_core_state(&mut state).unwrap();
        }
        let capture = started.elapsed().as_secs_f64() / f64::from(ROUNDS);

        let started = std::time::Instant::now();
        for _ in 0..ROUNDS {
            core.restore_core_state(&state).unwrap();
        }
        let restore = started.elapsed().as_secs_f64() / f64::from(ROUNDS);

        println!(
            "{name}: state {} bytes, capture {:.3} us, restore {:.3} us, both {:.3} us per real \
             frame",
            state.len(),
            capture * 1.0e6,
            restore * 1.0e6,
            (capture + restore) * 1.0e6
        );
    }
}

/// The two refusals: no ROM, and a buffer nothing was captured into.
#[test]
fn req_core_121_an_empty_state_and_a_stopped_core_are_refused() {
    let empty = CoreState::default();
    let mut stopped = NesCore::new();
    assert!(stopped
        .capture_core_state(&mut CoreState::default())
        .is_err());
    assert!(stopped.restore_core_state(&empty).is_err());

    let mut core = counter_core();
    core.run_frame(0);
    assert!(core.restore_core_state(&empty).is_err());
    // And the refusal left the machine running, unlike a corrupt savestate (REQ-CORE-005).
    assert!(core.is_running());
    core.run_frame(0);
    assert_eq!(2, core.frame_count());
}
