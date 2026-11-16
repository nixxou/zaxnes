//! Lot FDS: the Famicom Disk System as mapper 20 (Mesen2 `Fds.cpp`, `FdsAudio.h`,
//! `FdsLoader.h`), a `.fds` image packaged with the player's `disksys.rom`.
#![cfg(feature = "board-fds")]
#![allow(clippy::unwrap_used)]

use zax_nes::NesCore;

/// A side of `.fds` capacity: the disk header block (1), the file count block (2), then zeros.
fn side(name: [u8; 4]) -> Vec<u8> {
    let mut side = vec![0u8; 65_500];
    side[0] = 0x01;
    side[1..15].copy_from_slice(b"*NINTENDO-HVC*");
    // Header bytes 14-23: manufacturer, game name, type, version, side, disk, type, BIOS.
    side[15] = 0x01;
    side[16..20].copy_from_slice(&name);
    side[57] = 0x02;
    side
}

/// A fwNES image of the given sides.
fn disk(sides: &[Vec<u8>]) -> Vec<u8> {
    let mut image = b"FDS\x1A".to_vec();
    image.push(u8::try_from(sides.len()).unwrap());
    image.resize(16, 0);
    for side in sides {
        image.extend_from_slice(side);
    }
    image
}

/// An 8 KiB BIOS that loops at `$E000` (`JMP $E000`), every vector there, `$A5` elsewhere.
fn bios() -> Vec<u8> {
    let mut bios = vec![0xA5; 0x2000];
    bios[..3].copy_from_slice(&[0x4C, 0x00, 0xE0]);
    for vector in [0x1FFA, 0x1FFC, 0x1FFE] {
        bios[vector..vector + 2].copy_from_slice(&[0x00, 0xE0]);
    }
    bios
}

fn load(sides: &[Vec<u8>]) -> NesCore {
    let image = zax_nes::rom::package_fds(&disk(sides), &bios(), false).unwrap();
    let mut core = NesCore::new();
    let load = core.load_rom(&image, "game.fds");
    assert!(load.success, "{:?}", load.error);
    core
}

/// `FdsLoader::RebuildFdsFile`'s layout: a `.fds` packaged with the BIOS loads as mapper 20 with
/// 32 KiB of work RAM at `$6000-$DFFF` and the BIOS at `$E000`; a wrong BIOS or an image without
/// a side is refused with a message.
#[test]
fn fds_loads_with_its_bios() {
    assert!(zax_nes::rom::package_fds(&disk(&[side(*b"TEST")]), &[0; 0x1000], false).is_err());
    assert!(zax_nes::rom::package_fds(&[0; 100], &bios(), false).is_err());
    let mut core = load(&[side(*b"TEST")]);
    assert_eq!(20, core.cartridge().unwrap().mapper_number());
    assert_eq!(0x4C, core.read_memory(0xE000));
    assert_eq!(0xE0, core.read_memory(0xFFFD));
    for address in [0x6000, 0x8000, 0xDFFF] {
        core.write_memory(address, 0x42);
        assert_eq!(0x42, core.read_memory(address), "{address:04X}");
    }
    assert_eq!(1, core.disk_sides());
    assert_eq!(Some(0), core.disk_side());
}

/// `$4020-$4022`: the timer counts the reload value down once a cycle and raises its IRQ, which
/// a read of `$4030` (D0 set) acknowledges.
#[test]
fn fds_timer_raises_its_irq() {
    let mut core = load(&[side(*b"TEST")]);
    core.write_memory(0x4020, 10);
    core.write_memory(0x4021, 0);
    core.write_memory(0x4022, 0x02);
    let cartridge = core.cartridge_mut().unwrap();
    for _ in 0..10 {
        cartridge.step_cpu_cycle();
    }
    assert!(!cartridge.irq_asserted());
    cartridge.step_cpu_cycle();
    assert!(cartridge.irq_asserted());
    assert_eq!(Some(1), cartridge.try_cpu_read(0x4030).map(|v| v & 0x01));
    assert!(!cartridge.irq_asserted());
}

/// The drive (`Fds::ClockDisk`): with the motor on, 50 000 cycles to reach the start of the
/// side, a byte every 150 cycles, the gap skipped until the `$80` start mark; each byte after it
/// raises the disk IRQ and reads at `$4031`, the header block's code then `*NINTENDO-HVC*`.
#[test]
fn fds_drive_reads_the_disk() {
    let mut core = load(&[side(*b"TEST")]);
    // Motor, read mode, ready, disk IRQ.
    core.write_memory(0x4025, 0xC5);
    let cartridge = core.cartridge_mut().unwrap();
    let mut bytes = Vec::new();
    let mut cycles = 0u32;
    while bytes.len() < 4 && cycles < 1_200_000 {
        cartridge.step_cpu_cycle();
        cycles += 1;
        if cartridge.irq_asserted() {
            bytes.push(cartridge.try_cpu_read(0x4031).unwrap());
        }
    }
    assert_eq!(vec![0x01, b'*', b'N', b'I'], bytes);
    // `$4032`: a disk in, scanning.
    assert_eq!(Some(0), cartridge.try_cpu_read(0x4032).map(|v| v & 0x07));
}

/// `FdsAudio`: the wave table written while `$4089` D7 holds it, a gain set directly by `$4080`,
/// and a frequency: the expansion audio plays.
#[test]
fn fds_audio_plays_its_wave() {
    let mut core = load(&[side(*b"TEST")]);
    core.write_memory(0x4089, 0x80);
    for index in 0..64u16 {
        core.write_memory(0x4040 + index, if index < 32 { 63 } else { 0 });
    }
    core.write_memory(0x4089, 0x00);
    core.write_memory(0x4080, 0x80 | 0x20);
    core.write_memory(0x4082, 0x00);
    core.write_memory(0x4083, 0x04);
    let cartridge = core.cartridge_mut().unwrap();
    let mut loudest = 0;
    for _ in 0..20_000 {
        cartridge.step_cpu_cycle();
        loudest = loudest.max(cartridge.expansion_audio_output().unwrap_or(0));
    }
    assert!(loudest > 0, "{loudest}");
}

/// What the drive writes goes to the save file (`ZXFD`, the changed blocks), which a new
/// session applies over the original image; a savestate keeps the written disk too.
#[test]
fn fds_writes_survive_a_new_session_and_a_savestate() {
    let mut core = load(&[side(*b"TEST")]);
    assert_eq!(Vec::<u8>::new(), core.export_flash_save());
    core.write_memory(0x4024, 0x5A);
    // Motor, write mode, ready.
    core.write_memory(0x4025, 0x41);
    let cartridge = core.cartridge_mut().unwrap();
    for _ in 0..60_000 {
        cartridge.step_cpu_cycle();
    }
    let file = core.export_flash_save();
    assert_eq!(b"ZXFD", &file[..4]);

    let state = core.save_state(true).unwrap();
    let mut again = load(&[side(*b"TEST")]);
    assert!(again.try_load_state(&state, 0).is_ok());
    assert_eq!(file, again.export_flash_save());

    let mut fresh = load(&[side(*b"TEST")]);
    assert!(!fresh.import_flash_save(b"not a save"));
    assert!(fresh.import_flash_save(&file));
    assert_eq!(file, fresh.export_flash_save());
}

/// Mesen's automatic insertion: when the BIOS reads `$E445`, the side whose header matches the
/// ten bytes `$00`/`$01` point at goes in, at the end of the frame; a hand change takes the disk
/// out and puts the new one in two seconds later.
#[test]
fn fds_inserts_the_side_the_game_asks_for() {
    let mut core = load(&[side(*b"ONE "), side(*b"TWO ")]);
    assert_eq!(2, core.disk_sides());
    assert_eq!(Some(0), core.disk_side());
    core.write_memory(0x0000, 0x00);
    core.write_memory(0x0001, 0x03);
    let mut wanted = [0xFF; 10];
    wanted[1..5].copy_from_slice(b"TWO ");
    for (index, byte) in wanted.iter().enumerate() {
        core.write_memory(0x0300 + u16::try_from(index).unwrap(), *byte);
    }
    let _ = core.cartridge_mut().unwrap().try_cpu_read(0xE445);
    core.run_frame(0);
    assert_eq!(Some(1), core.disk_side());

    core.set_disk_auto_insert(false);
    core.switch_disk_side();
    assert_eq!(None, core.disk_side());
    for _ in 0..120 {
        core.run_frame(0);
    }
    assert_eq!(Some(0), core.disk_side());
    core.eject_disk();
    assert_eq!(None, core.disk_side());
    core.insert_next_disk();
    assert_eq!(Some(0), core.disk_side());
}
