//! Golden parity of the PPU with the C# reference: for every row of `core-goldens.jsonl`
//! whose ROM is mapper 0, `pixel_sha256` and `tiles_sha256` must match at every checkpoint
//! (run protocol and tile serialisation: `rust/goldens/README.md`).
//!
//! The default test checks the checkpoints up to frame 60; the full run up to frame 1200 is
//! `cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored`.

// A harness: a poisoned lock or unreadable goldens file should fail the test outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use sha2::{Digest, Sha256};
use zax_goldens::{goldens_dir, read_core_goldens, CoreGolden};
use zax_nes::{Machine, TileRecord};

/// Rows that still differ because the APU is a stub until T03: every one of them is an APU
/// test, or reads the frame or DMC IRQ, the $4015 status or the DMC DMA stalls to decide
/// what it draws (`stars_se` and `dpcmletterbox` time their raster effects on the DMC
/// IRQ, `other/SPRITE.NES` polls the DMC bit of $4015). They are reported, not failed.
const PENDING_T03: &[&str] = &[
    "apu_reset/4017_timing.nes",
    "apu_reset/4017_written.nes",
    "apu_reset/works_immediately.nes",
    "apu_test/rom_singles/1-len_ctr.nes",
    "apu_test/rom_singles/2-len_table.nes",
    "apu_test/rom_singles/3-irq_flag.nes",
    "apu_test/rom_singles/4-jitter.nes",
    "apu_test/rom_singles/5-len_timing.nes",
    "apu_test/rom_singles/6-irq_flag_timing.nes",
    "apu_test/rom_singles/7-dmc_basics.nes",
    "apu_test/rom_singles/8-dmc_rates.nes",
    "blargg_apu_2005.07.30/01.len_ctr.nes",
    "blargg_apu_2005.07.30/02.len_table.nes",
    "blargg_apu_2005.07.30/03.irq_flag.nes",
    "blargg_apu_2005.07.30/04.clock_jitter.nes",
    "blargg_apu_2005.07.30/05.len_timing_mode0.nes",
    "blargg_apu_2005.07.30/06.len_timing_mode1.nes",
    "blargg_apu_2005.07.30/07.irq_flag_timing.nes",
    "blargg_apu_2005.07.30/08.irq_timing.nes",
    "blargg_apu_2005.07.30/09.reset_timing.nes",
    "blargg_apu_2005.07.30/10.len_halt_timing.nes",
    "blargg_apu_2005.07.30/11.len_reload_timing.nes",
    "cpu_interrupts_v2/rom_singles/1-cli_latency.nes",
    "cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes",
    "cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes",
    "cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes",
    "dmc_dma_during_read4/dma_4016_read.nes",
    "dpcmletterbox/dpcmletterbox.nes",
    "instr_misc/rom_singles/04-dummy_reads_apu.nes",
    "instr_timing/rom_singles/1-instr_timing.nes",
    "instr_timing/rom_singles/2-branch_timing.nes",
    "other/SPRITE.NES",
    "pal_apu_tests/01.len_ctr.nes",
    "pal_apu_tests/02.len_table.nes",
    "pal_apu_tests/03.irq_flag.nes",
    "pal_apu_tests/04.clock_jitter.nes",
    "pal_apu_tests/07.irq_flag_timing.nes",
    "pal_apu_tests/08.irq_timing.nes",
    "sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes",
    "sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes",
    "stars_se/StarsSE.NES",
];

fn hex(digest: &[u8]) -> String {
    use std::fmt::Write as _;
    digest.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn tiles_hash(records: &[TileRecord]) -> String {
    let mut hasher = Sha256::new();
    for r in records {
        hasher.update(r.plane0.to_le_bytes());
        hasher.update(r.plane1.to_le_bytes());
        hasher.update(r.x.to_le_bytes());
        hasher.update(r.y.to_le_bytes());
        hasher.update(r.index.to_le_bytes());
        let flags = u8::from(r.mirror_h)
            | (u8::from(r.mirror_v) << 1)
            | (u8::from(r.hidden) << 2)
            | (u8::from(r.background) << 3);
        hasher.update([flags, r.palette]);
    }
    format!("{}:{}", records.len(), hex(&hasher.finalize()))
}

/// Runs one golden row up to `last_frame`; `Err` describes the first mismatch.
fn check_row(row: &CoreGolden, image: &[u8], last_frame: u32) -> Result<(), String> {
    let digest = hex(&Sha256::digest(image));
    if digest != row.rom_sha256 {
        return Err(format!("rom_sha256 {digest} differs from the golden"));
    }

    let name = Path::new(&row.rom)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut machine = Machine::new();
    let load = machine.load_rom(image, &name);
    if load.success != row.load_ok {
        return Err(format!(
            "load_ok {} vs golden {}",
            load.success, row.load_ok
        ));
    }
    if !load.success {
        return Ok(());
    }

    machine.reset();
    for frame in 1..=last_frame {
        machine.run_frame();
        if let Some(expected) = row.pixel_sha256.get(&frame) {
            let actual = hex(&Sha256::digest(machine.pixel_frame()));
            if &actual != expected {
                return Err(format!("pixels differ at frame {frame}"));
            }
        }
        if let Some(expected) = row.tiles_sha256.get(&frame) {
            let actual = tiles_hash(machine.tile_records());
            if &actual != expected {
                return Err(format!(
                    "tiles differ at frame {frame}: {actual} vs {expected}"
                ));
            }
        }
    }
    Ok(())
}

fn run_goldens(last_frame: u32) {
    let rows = read_core_goldens(&goldens_dir().join("core-goldens.jsonl")).expect("goldens load");
    let rows: Vec<CoreGolden> = rows
        .into_iter()
        .filter(|row| row.mapper == Some(0))
        .collect();
    let rom_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms");
    if !rom_dir.is_dir() {
        eprintln!("skipped: {} not found", rom_dir.display());
        return;
    }

    let next = AtomicUsize::new(0);
    let matched = AtomicUsize::new(0);
    let missing = Mutex::new(Vec::new());
    let mismatches = Mutex::new(Vec::new());
    let pending = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(1, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(row) = rows.get(i) else { break };
                let Ok(image) = std::fs::read(rom_dir.join(&row.rom)) else {
                    missing.lock().unwrap().push(row.rom.clone());
                    continue;
                };
                match check_row(row, &image, last_frame) {
                    Ok(()) => {
                        matched.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(why) if PENDING_T03.contains(&row.rom.as_str()) => {
                        pending.lock().unwrap().push(format!("{}: {why}", row.rom));
                    }
                    Err(why) => mismatches
                        .lock()
                        .unwrap()
                        .push(format!("{}: {why}", row.rom)),
                }
            });
        }
    });

    let mut mismatches = mismatches.into_inner().unwrap();
    mismatches.sort();
    let missing = missing.into_inner().unwrap();
    let pending = pending.into_inner().unwrap();
    eprintln!(
        "mapper-0 goldens up to frame {last_frame}: {} of {} rows matched, {} pending T03 (APU stub), {} ROMs missing",
        matched.load(Ordering::Relaxed),
        rows.len(),
        pending.len(),
        missing.len()
    );
    assert!(
        mismatches.is_empty(),
        "{} mismatching ROMs:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[test]
fn golden_mapper_0_pixels_and_tiles_match_up_to_frame_60() {
    run_goldens(60);
}

#[test]
#[ignore = "slow: 222 ROMs x 1200 frames; run with --release -- --ignored"]
fn golden_mapper_0_pixels_and_tiles_match_up_to_frame_1200() {
    run_goldens(1200);
}
