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
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use zax_core_api::TileRecord;
use zax_fceux::{Fceux, RamInit};
use zax_nes::rom::InesHeader;

use crate::golden_core::{collect_roms, hex, relative_name};
use crate::HarnessError;

pub const USAGE: &str =
    "zax-harness fceux-run <roms dir|rom.nes> <out.jsonl> [frames] [--in-process]
    Runs every ROM on the FCEUX engine and writes golden-core's digests, so the two engines
    can be compared row by row. Needs the `fceux-core` feature and a built fork.
    A directory run gives each ROM its own child process, because FCEUX can take the process
    down on some ROMs (see the module docs); --in-process turns that off.

zax-harness fceux-frame <rom.nes> <out.json> [frames] [--ram zero|default|ff]
    Runs a ROM on FCEUX and writes the tile records of the last frame in the format
    `render --frames <file.json>` reads, so FCEUX's records can go through Zaxnes's 3D path.
    --ram is what FCEUX's power-on writes into the uninitialised memory; it defaults to `zero`,
    which is what zax-nes does, so that the frame is the one `fceux-tiles` compares.";

/// Frames at which the digests are recorded — `golden_core::CHECKPOINTS`, repeated here because
/// that one is private to it and the two lists must never drift.
const CHECKPOINTS: [u32; 7] = [1, 60, 300, 600, 1200, 1800, 3600];

/// What an `fceux-run` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// ROMs that loaded and ran to the end.
    pub run: usize,
    /// `.nes` files found.
    pub files: usize,
    /// ROMs whose child process did not come back (always 0 for an in-process run, which has no
    /// way to survive one).
    pub crashed: usize,
}

/// The command line: `<roms dir|rom.nes> <out.jsonl> [frames]`, run and reported.
///
/// # Errors
///
/// [`HarnessError::Usage`] when the frame count is not a number, or whatever [`run`] returns.
pub fn command(roms: &str, out_path: &str, rest: &[String]) -> Result<(), HarnessError> {
    let mut frames = crate::golden_core::DEFAULT_FRAMES;
    let mut in_process = false;
    for argument in rest {
        if argument == "--in-process" {
            in_process = true;
        } else {
            frames = argument.parse().map_err(|_| {
                HarnessError::Usage(format!("{argument:?} is not a frame count\n{USAGE}"))
            })?;
        }
    }

    let roms = Path::new(roms);
    let summary = if roms.is_dir() && !in_process {
        run_isolated(roms, Path::new(out_path), frames)?
    } else {
        run(roms, Path::new(out_path), frames)?
    };

    println!(
        "fceux-run: {} ROMs run, {} files, {frames} frames each{} -> {out_path}",
        summary.run,
        summary.files,
        if summary.crashed == 0 {
            String::new()
        } else {
            format!(", {} took the engine down", summary.crashed)
        }
    );
    Ok(())
}

/// One child process per ROM, so that a ROM FCEUX cannot survive costs one row instead of the
/// whole run.
///
/// The child is this same executable with the single-ROM form of the command, which is why that
/// form exists. A child that dies gets a row of its own saying so, built from what the parent
/// knows (the name, the image's hash, the header's mapper) — enough for the comparison to show
/// the ROM and what happened to it.
///
/// # Errors
///
/// [`HarnessError`] when the ROM directory cannot be walked, or the output cannot be written.
pub fn run_isolated(roms: &Path, out_path: &Path, frames: u32) -> Result<Summary, HarnessError> {
    let mut files = Vec::new();
    collect_roms(roms, &mut files)?;
    files.sort_by_key(|file| relative_name(roms, file));

    let executable = std::env::current_exe()?;
    let scratch = out_path.with_extension("one.jsonl");

    let mut writer = BufWriter::new(fs::File::create(out_path)?);
    let mut ran = 0;
    let mut crashed = 0;
    // A child must not be able to stall the sweep. A healthy ROM takes well under a second for
    // 1200 frames, so this is generous by three orders of magnitude and only ever hits a child
    // that is not coming back.
    let budget = Duration::from_secs(60 + u64::from(frames) / 20);

    for file in &files {
        let name = relative_name(roms, file);
        let status = Command::new(&executable)
            .arg("fceux-run")
            .arg(file)
            .arg(&scratch)
            .arg(frames.to_string())
            .arg("--in-process")
            .stdout(std::process::Stdio::null())
            .spawn()
            .and_then(|child| wait_or_kill(child, budget));

        // The child wrote the row under the ROM's own file name; put the name the directory walk
        // gives it back, so both engines' files key on the same string.
        let row = match (status, fs::read_to_string(&scratch)) {
            (Ok(status), Ok(text)) if status.success() => text
                .lines()
                .find(|line| !line.trim().is_empty())
                .and_then(|line| serde_json::from_str::<Value>(line).ok())
                .map(|mut row| {
                    if let Value::Object(fields) = &mut row {
                        fields.insert("rom".into(), json!(name));
                    }
                    row
                }),
            _ => None,
        };

        let row = row.unwrap_or_else(|| {
            crashed += 1;
            crashed_row(&name, file, frames)
        });
        if row.get("load_ok") == Some(&Value::Bool(true)) && row.get("engine_crashed").is_none() {
            ran += 1;
        }

        serde_json::to_writer(&mut writer, &row).map_err(std::io::Error::from)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    let _ = fs::remove_file(&scratch);
    Ok(Summary {
        run: ran,
        files: files.len(),
        crashed,
    })
}

/// Waits for `child`, killing it if it outstays `budget`.
///
/// `std` has no wait-with-timeout, so this polls. It exists because a crashing child does **not**
/// reliably die: when FCEUX corrupted the heap under `other/oam3.nes` the child aborted cleanly
/// when started from a shell, but hung for ever as a child of this process — Windows' fault
/// reporting waiting on something that is never going to answer. Without this, one bad ROM stalls
/// a 291-ROM sweep instead of costing it one row.
pub(crate) fn wait_or_kill(
    mut child: std::process::Child,
    budget: Duration,
) -> std::io::Result<ExitStatus> {
    let deadline = Instant::now() + budget;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            child.kill()?;
            return child.wait();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The row of a ROM whose child process did not come back: everything the parent can know
/// without running it, plus the fact that the engine died.
fn crashed_row(name: &str, file: &Path, frames: u32) -> Value {
    let mut row = Map::new();
    row.insert("rom".into(), json!(name));
    match fs::read(file) {
        Ok(image) => {
            row.insert("rom_sha256".into(), json!(hex(&Sha256::digest(&image))));
            row.insert(
                "mapper".into(),
                InesHeader::try_parse(&image).map_or(Value::Null, |header| json!(header.mapper)),
            );
        }
        Err(error) => {
            row.insert("rom_read_error".into(), json!(error.to_string()));
        }
    }
    row.insert("load_ok".into(), json!(false));
    row.insert("frames".into(), json!(frames));
    row.insert("engine_crashed".into(), json!(true));
    row.insert(
        "run_error".into(),
        json!("the FCEUX engine took its process down or stopped answering; the row is what the               parent could record without running it"),
    );
    Value::Object(row)
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
        crashed: 0,
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

/// `fceux-frame <rom.nes> <out.json> [frames]`: runs a ROM on FCEUX and writes the **tile
/// records** of the last frame in the format `render --frames <file.json>` reads.
///
/// This is how step 6 of lot FCEUX-1 puts FCEUX's records through Zaxnes's own 3D path: the same
/// renderer, the same profile, the same code — only the records come from the other engine. The
/// comparison is then two PNGs of the same scene, which `zax-harness compare` can score.
///
/// # Errors
///
/// [`HarnessError::Usage`] on a bad frame count or if FCEUX refuses the ROM; [`HarnessError`] on
/// an I/O failure.
pub fn frame_command(rom: &str, out_path: &str, rest: &[String]) -> Result<(), HarnessError> {
    frame_command_inner(rom, out_path, rest)
}

/// `fceux-frame`'s trailing arguments: the frame to stop at, and the power-on fill.
///
/// **The fill defaults to `zero`, not to FCEUX's own `Default` pattern** — lot FCEUX-6 step 3's
/// finding, applied here in step 8. `FCEU_MemoryRand` fills the work RAM *and* the CHR-RAM of a
/// `chr = 0` cart with `00 00 00 00 FF FF FF FF` (`src/fceu.cpp:990-1022`, `src/ines.cpp:78-79`),
/// while `zax-nes` clears both (REQ-CORE-004). The two machines then start from different memory,
/// and a CHR-RAM cart reading a tile it never wrote reads a different tile on each.
///
/// This command exists to feed Zaxnes's 3D path with FCEUX's records and have the picture be
/// comparable to Zaxnes's own, which it cannot be unless the two are the same machine — so it
/// defaults to the fill `fceux-tiles` compares with, and step 3 measured what that is worth: 47
/// of 98 originals record-identical at every compared frame against 41 with the `Default` fill,
/// and 177 of 289 public ROMs against 162.
fn parse_frame_arguments(rest: &[String]) -> Result<(u32, RamInit), HarnessError> {
    let mut frames = 120;
    let mut ram = RamInit::Zero;
    let mut index = 0;
    while index < rest.len() {
        let argument = rest[index].as_str();
        if argument == "--ram" {
            let value = rest
                .get(index + 1)
                .ok_or_else(|| HarnessError::Usage(USAGE.to_owned()))?;
            ram = ram_init(value)?;
            index += 2;
            continue;
        }
        frames = argument.parse().map_err(|_| {
            HarnessError::Usage(format!("{argument:?} is not a frame count{}", usage_tail()))
        })?;
        index += 1;
    }
    Ok((frames, ram))
}

/// One `--ram` spelling.
fn ram_init(value: &str) -> Result<RamInit, HarnessError> {
    match value {
        "zero" => Ok(RamInit::Zero),
        "default" => Ok(RamInit::Default),
        "ff" => Ok(RamInit::Ff),
        other => Err(HarnessError::Usage(format!(
            "--ram: {other:?} is not zero, default or ff{}",
            usage_tail()
        ))),
    }
}

/// The usage, on its own line after a message.
fn usage_tail() -> String {
    format!("\n{USAGE}")
}

fn frame_command_inner(rom: &str, out_path: &str, rest: &[String]) -> Result<(), HarnessError> {
    let (frames, ram) = parse_frame_arguments(rest)?;

    let image = fs::read(rom)?;
    let name = Path::new(rom).file_name().map_or_else(
        || rom.to_owned(),
        |base| base.to_string_lossy().into_owned(),
    );

    let mut engine = Fceux::new().map_err(|error| HarnessError::Usage(error.to_string()))?;
    engine
        .set_ram_init(ram)
        .and_then(|()| engine.load_rom(&image, &name))
        .map_err(|error| HarnessError::Usage(error.to_string()))?;
    engine.set_tile_capture(true);
    engine
        .reset()
        .map_err(|error| HarnessError::Usage(error.to_string()))?;

    let mut capture = None;
    for _ in 0..frames.max(1) {
        engine
            .set_input(0, 0)
            .and_then(|()| engine.run_frame())
            .map_err(|error| HarnessError::Usage(error.to_string()))?;
        if let Some(got) = engine.take_tiles() {
            capture = Some(got);
        }
    }

    let Some(capture) = capture else {
        return Err(HarnessError::Usage(
            "FCEUX captured no tile record: the ROM never turned rendering on".to_owned(),
        ));
    };

    let records: Vec<TileRecord> = capture
        .records
        .iter()
        .map(|tile| {
            TileRecord::new(
                tile.plane0,
                tile.plane1,
                tile.x,
                tile.y,
                tile.index,
                tile.mirror_h,
                tile.mirror_v,
                tile.hidden,
                tile.palette,
                tile.background,
            )
        })
        .collect();

    fs::write(
        out_path,
        crate::render::frame_json::write(&records, &capture.palette)?,
    )?;
    println!(
        "fceux-frame: {} records of frame {frames} of {name} -> {out_path}",
        records.len()
    );
    Ok(())
}
