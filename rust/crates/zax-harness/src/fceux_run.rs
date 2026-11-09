//! `fceux-run <roms dir|rom.nes> <out.jsonl> [frames]`: what [`crate::golden_core`] does, run on
//! **FCEUX** instead of Zaxnes's own core, so the two engines can be compared ROM by ROM.
//!
//! The rows carry the same keys with the same meaning wherever both engines can answer, which is
//! the whole point: `pixel_sha256` is taken over the identical 256×240 index image (0 to 63 a
//! pixel), `audio_i16_sha256_samples` over the identical little-endian `i16` serialisation, and
//! the checkpoint frames are `golden_core`'s own list. So a row of this file and a row of
//! `core-goldens.jsonl` for the same ROM are comparable key by key — `compare_engines` does
//! exactly that.
//!
//! Three keys are **absent by necessity** rather than by choice:
//!
//! * `tiles_sha256` — FCEUX produces no tile records at all. That is lot FCEUX-1's step 6, and
//!   until it lands there is nothing honest to put here.
//! * `cycles` — the C API exposes no CPU cycle count. FCEUX has `timestampbase + timestamp`
//!   internally, but the two cores count from different origins and reset them differently, so a
//!   number here would compare two unlike things. The frame count is published instead.
//! * `region` is reported, but FCEUX applies the region **itself** when it loads (the API asks it
//!   to), whereas `zax-nes` detects and returns one without applying it. The value is the region
//!   the frames were actually run at on each side, which is what matters for the comparison.
//!
//! Determinism: the RAM fill is pinned to FCEUX's deterministic default pattern
//! ([`RamInit::Default`]), so two runs of this command over the same ROMs produce the same file.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use zax_fceux::{Fceux, RamInit};
use zax_nes::rom::InesHeader;

use crate::golden_core::{collect_roms, hex, relative_name};
use crate::HarnessError;

pub const USAGE: &str = "zax-harness fceux-run <roms dir|rom.nes> <out.jsonl> [frames]
    Runs every ROM on the FCEUX engine and writes golden-core's digests, so the two engines
    can be compared row by row. Needs the `fceux-core` feature and a built fork.";

/// Frames at which the digests are recorded — `golden_core::CHECKPOINTS`, repeated here because
/// that one is private to it and the two lists must never drift.
const CHECKPOINTS: [u32; 7] = [1, 60, 300, 600, 1200, 1800, 3600];

/// What an `fceux-run` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// ROMs that loaded and ran.
    pub run: usize,
    /// `.nes` files found.
    pub files: usize,
}

/// The command line: `<roms dir|rom.nes> <out.jsonl> [frames]`, run and reported.
///
/// # Errors
///
/// [`HarnessError::Usage`] when the frame count is not a number, or whatever [`run`] returns.
pub fn command(roms: &str, out_path: &str, rest: &[String]) -> Result<(), HarnessError> {
    let frames = match rest {
        [] => crate::golden_core::DEFAULT_FRAMES,
        [count] => count
            .parse()
            .map_err(|_| HarnessError::Usage(format!("{count:?} is not a frame count\n{USAGE}")))?,
        _ => return Err(HarnessError::Usage(USAGE.to_owned())),
    };

    let summary = run(Path::new(roms), Path::new(out_path), frames)?;
    println!(
        "fceux-run: {} ROMs run, {} files, {frames} frames each -> {out_path}",
        summary.run, summary.files
    );
    Ok(())
}

/// Runs the command: one row per `.nes` file under `roms`, in the same ordinal path order
/// `golden-core` uses, to `out_path`. A single `.nes` file is accepted too.
///
/// # Errors
///
/// [`HarnessError`] when the ROM directory cannot be walked, a file cannot be read or written, or
/// the FCEUX engine cannot be created.
pub fn run(roms: &Path, out_path: &Path, frames: u32) -> Result<Summary, HarnessError> {
    let (root, mut files) = if roms.is_dir() {
        let mut found = Vec::new();
        collect_roms(roms, &mut found)?;
        (roms.to_path_buf(), found)
    } else {
        let root = roms.parent().unwrap_or(Path::new(".")).to_path_buf();
        (root, vec![roms.to_path_buf()])
    };
    files.sort_by_key(|file| relative_name(&root, file));

    // One engine for the whole run, reused ROM after ROM: FCEUX keeps its machine in file-scope
    // globals, so there can only ever be one, and closing a game and loading the next is what a
    // fresh instance would amount to anyway.
    let mut engine = Fceux::new().map_err(|error| HarnessError::Usage(error.to_string()))?;

    let mut writer = BufWriter::new(fs::File::create(out_path)?);
    let mut ran = 0;
    for file in &files {
        let image = fs::read(file)?;
        let row = row_for(&mut engine, &relative_name(&root, file), &image, frames);
        if row.get("load_ok") == Some(&Value::Bool(true)) {
            ran += 1;
        }

        serde_json::to_writer(&mut writer, &row).map_err(std::io::Error::from)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(Summary {
        run: ran,
        files: files.len(),
    })
}

/// The row of one ROM image, named `rel` in the output.
fn row_for(engine: &mut Fceux, rel: &str, image: &[u8], frames: u32) -> Value {
    let mut row = Map::new();
    row.insert("rom".into(), json!(rel));
    row.insert("rom_sha256".into(), json!(hex(&Sha256::digest(image))));
    row.insert(
        "mapper".into(),
        InesHeader::try_parse(image).map_or(Value::Null, |header| json!(header.mapper)),
    );

    // The basename, as the C self-test and `zax-fceux`'s tests pass: FCEUX uses the name with the
    // image's MD5 against its own cartridge database, so a digest must not depend on the path.
    let name = Path::new(rel).file_name().map_or_else(
        || rel.to_owned(),
        |base| base.to_string_lossy().into_owned(),
    );

    engine.close_rom();
    if let Err(error) = engine.set_ram_init(RamInit::Default) {
        row.insert("load_ok".into(), json!(false));
        row.insert("load_error".into(), json!(error.to_string()));
        return Value::Object(row);
    }
    // 48 kHz, which is `zax-nes`'s own default (`apu::DEFAULT_SAMPLE_RATE`), so the two engines'
    // audio digests are taken over the same number of samples a frame.
    let _ = engine.set_sample_rate(48_000);

    if let Err(error) = engine.load_rom(image, &name) {
        row.insert("load_ok".into(), json!(false));
        row.insert("load_error".into(), json!(error.to_string()));
        return Value::Object(row);
    }

    row.insert("load_ok".into(), json!(true));
    row.insert(
        "region".into(),
        json!(engine
            .region()
            .map_or_else(|error| error.to_string(), |region| format!("{region:?}"))),
    );
    row.insert("sample_rate".into(), json!(engine.sample_rate()));

    // `golden-core` resets after loading; the same here, so both engines start from the same
    // place.
    if let Err(error) = engine.reset() {
        row.insert("load_ok".into(), json!(false));
        row.insert("load_error".into(), json!(error.to_string()));
        return Value::Object(row);
    }

    let mut pixels = Map::new();
    let mut emphasis = Map::new();
    let mut audio = Map::new();
    let mut audio_hash = Sha256::new();
    let mut samples: u64 = 0;
    let mut frame_buffer = Vec::new();
    let mut emphasis_buffer = Vec::new();
    let mut sample_buffer = Vec::new();
    let mut failure: Option<String> = None;

    for frame in 1..=frames {
        if let Err(error) = engine.set_input(0, 0).and_then(|()| engine.run_frame()) {
            failure = Some(format!("frame {frame}: {error}"));
            break;
        }

        sample_buffer.clear();
        match engine.read_audio_into(&mut sample_buffer) {
            Ok(got) => {
                for sample in &sample_buffer[..got] {
                    audio_hash.update(sample.to_le_bytes());
                }
                samples += got as u64;
            }
            Err(error) => {
                failure = Some(format!("frame {frame}: {error}"));
                break;
            }
        }

        if CHECKPOINTS.contains(&frame) {
            let key = frame.to_string();
            if let Err(error) = engine
                .frame_indices_into(&mut frame_buffer)
                .and_then(|()| engine.frame_emphasis_into(&mut emphasis_buffer))
            {
                failure = Some(format!("frame {frame}: {error}"));
                break;
            }

            pixels.insert(key.clone(), json!(hex(&Sha256::digest(&frame_buffer))));
            emphasis.insert(key.clone(), json!(hex(&Sha256::digest(&emphasis_buffer))));
            audio.insert(
                key,
                json!(format!("{}:{samples}", hex(&audio_hash.clone().finalize()))),
            );
        }
    }

    row.insert("frames".into(), json!(frames));
    row.insert("frames_run".into(), json!(engine.frame_count()));
    row.insert("pixel_sha256".into(), Value::Object(pixels));
    row.insert("emphasis_sha256".into(), Value::Object(emphasis));
    row.insert("audio_i16_sha256_samples".into(), Value::Object(audio));
    if let Some(failure) = failure {
        row.insert("run_error".into(), json!(failure));
    }

    // The blargg protocol, exactly as `golden_core` reads it: $6001-$6003 = DE B0 61, the status
    // in $6000, the text from $6004. `zax_read_memory` is side-effect-free, so reading a
    // cartridge's RAM here cannot disturb the result it is reporting.
    let byte = |address: u16| engine.read_memory(address).unwrap_or(0);
    if byte(0x6001) == 0xDE && byte(0x6002) == 0xB0 && byte(0x6003) == 0x61 {
        row.insert("blargg_status".into(), json!(byte(0x6000)));
        let text: String = (0x6004u16..0x7000)
            .map(byte)
            .take_while(|&byte| byte != 0)
            .map(char::from)
            .collect();
        row.insert("blargg_text".into(), json!(text));
    }

    Value::Object(row)
}
