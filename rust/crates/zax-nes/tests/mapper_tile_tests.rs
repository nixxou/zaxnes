//! Tile records on boards with CHR side effects (REQ-CORE-033, REQ-CORE-039, REQ-CORE-044).
//! The two tests that compare saved states with and without a video callback belong to the
//! `NesCore` facade (T05).

// Test values are small: the narrowing casts cannot lose anything.
#![allow(clippy::cast_possible_truncation, clippy::unwrap_used)]

mod common;

use common::capture_batch;
use common::mapper_fixture::{MapperMachine, MapperRom};
use zax_nes::NesCore;

fn load_mmc2() -> NesCore {
    let mut core = MapperRom::new(9).prg(8).chr(64).load();
    for _ in 0..3 {
        core.run_frame();
    }

    // CHR for the $0000 window: 4 KiB bank 1 under latch $FD, bank 2 under latch $FE.
    core.mapper_write(0xB000, 0x01);
    core.mapper_write(0xC000, 0x02);
    core
}

fn write_vram(core: &mut NesCore, address: u16, values: &[u8]) {
    core.write_memory(0x2006, (address >> 8) as u8);
    core.write_memory(0x2006, address as u8);
    for &value in values {
        core.write_memory(0x2007, value);
    }
}

fn tile41_plane(bank: u64) -> u64 {
    let fill = 4 * bank + 1;
    (0..8).fold(0, |plane, row| plane | (fill << (8 * row)))
}

#[test]
fn req_core_039_an_mmc2_row_carries_the_pattern_each_latch_selected() {
    let mut core = load_mmc2();
    // The row of the acceptance criterion: $FE in column 2, $FD in column 8, $41 in columns 5
    // and 12.
    let mut row = [0u8; 32];
    row[2] = 0xFE;
    row[5] = 0x41;
    row[8] = 0xFD;
    row[12] = 0x41;
    write_vram(&mut core, 0x2000, &row);
    // The $2006 writes left the fine Y scroll at 2, as they always do for an address in
    // $2xxx; the pair of $2005 writes of the SPEC-02 §5.1 recipe puts it back to 0 so that the
    // row of line 0 is captured on line 1 (REQ-CORE-037).
    core.write_memory(0x2005, 0x00);
    core.write_memory(0x2005, 0x00);
    core.write_memory(0x2000, 0x00); // Background patterns at $0000.
    core.write_memory(0x2001, 0x0A); // Background on, left column shown.
    core.run_frame();
    let batch = capture_batch(&mut core);
    let first: Vec<_> = batch
        .background()
        .into_iter()
        .filter(|r| r.y == 0)
        .collect();
    assert_eq!(32, first.len());
    // Column 5 follows the $FE in column 2, column 12 the $FD in column 8.
    assert_eq!(tile41_plane(2), first[5].plane0);
    assert_eq!(tile41_plane(1), first[12].plane0);
}

#[test]
fn req_core_044_mappers_9_and_10_emit_the_sprites_of_a_capture_by_x() {
    for mapper in [9usize, 10] {
        let mut core = MapperRom::new(mapper).prg(8).chr(64).load();
        for _ in 0..3 {
            core.run_frame();
        }

        // Four sprites in the same eight-line band, in decreasing x: the records must come
        // out by increasing x, OAM number breaking ties.
        core.write_memory(0x2003, 0x00);
        let xs = [0x60u8, 0x40, 0x40, 0x20];
        for x in xs {
            core.write_memory(0x2004, 0x00); // y = 0, so the band of line 7.
            core.write_memory(0x2004, 0x41);
            core.write_memory(0x2004, 0x00);
            core.write_memory(0x2004, x);
        }

        for _ in xs.len()..64 {
            for value in [0xFF, 0x00, 0x00, 0x00] {
                core.write_memory(0x2004, value);
            }
        }

        core.write_memory(0x2001, 0x1E);
        core.run_frame();
        let batch = capture_batch(&mut core);
        let sprites = batch.sprites();
        assert_eq!(4, sprites.len(), "mapper {mapper}");
        let got_x: Vec<i32> = sprites.iter().map(|r| i32::from(r.x)).collect();
        assert_eq!(vec![0x20, 0x40, 0x40, 0x60], got_x, "mapper {mapper}");
        // Equal x keeps the OAM order: sprite 1 before sprite 2.
        let got_index: Vec<i32> = sprites.iter().map(|r| r.index).collect();
        assert_eq!(vec![3, 1, 2, 0], got_index, "mapper {mapper}");
    }
}

#[test]
fn req_core_044_other_mappers_keep_the_oam_order() {
    let mut core = MapperRom::new(4).prg(8).chr(64).load();
    for _ in 0..3 {
        core.run_frame();
    }

    core.write_memory(0x2003, 0x00);
    let xs = [0x60u8, 0x40, 0x20];
    for x in xs {
        for value in [0x00, 0x41, 0x00, x] {
            core.write_memory(0x2004, value);
        }
    }

    for _ in xs.len()..64 {
        for value in [0xFF, 0x00, 0x00, 0x00] {
            core.write_memory(0x2004, value);
        }
    }

    core.write_memory(0x2001, 0x1E);
    core.run_frame();
    let batch = capture_batch(&mut core);
    let got: Vec<i32> = batch.sprites().iter().map(|r| r.index).collect();
    assert_eq!(vec![0, 1, 2], got);
}
