//! `golden-core <roms dir> <out.jsonl> [frames]`: dumps the golden data of the core over a
//! directory of test ROMs, one JSON object per line, in the format of
//! `rust/goldens/core-goldens.jsonl` (protocol in `rust/goldens/README.md`).
//!
//! Every ROM is loaded, reset, and run for `frames` frames with an input word of 0. At the
//! checkpoint frames the hash of the pixel frame, the hash of the tile records, the CPU
//! cycle count and the running hash of the audio quantised to little-endian 16-bit are
//! recorded.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use zax_nes::rom::InesHeader;
use zax_nes::{NesCore, TileRecord};

use crate::HarnessError;

/// Frames run when the command line gives no count.
pub const DEFAULT_FRAMES: u32 = 1200;

/// Frames at which the hashes are recorded.
const CHECKPOINTS: [u32; 7] = [1, 60, 300, 600, 1200, 1800, 3600];

/// What a `golden-core` run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// ROMs that loaded and ran.
    pub run: usize,
    /// `.nes` files found.
    pub files: usize,
}

/// Runs the command: writes one row per `.nes` file under `roms_dir`, in ordinal path
/// order, to `out_path`.
///
/// # Errors
/// [`HarnessError`] when the ROM directory cannot be walked or a file read or written.
pub fn run(roms_dir: &Path, out_path: &Path, frames: u32) -> Result<Summary, HarnessError> {
    let mut files = Vec::new();
    collect_roms(roms_dir, &mut files)?;
    files.sort_by_key(|file| relative_name(roms_dir, file));

    let mut writer = BufWriter::new(fs::File::create(out_path)?);
    let mut run = 0;
    for file in &files {
        let image = fs::read(file)?;
        let row = golden_row(&relative_name(roms_dir, file), &image, frames)?;
        if row.get("load_ok") == Some(&Value::Bool(true)) {
            run += 1;
        }

        serde_json::to_writer(&mut writer, &row).map_err(std::io::Error::from)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(Summary {
        run,
        files: files.len(),
    })
}

fn collect_roms(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), HarnessError> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_roms(&path, found)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("nes"))
        {
            found.push(path);
        }
    }

    Ok(())
}

/// The path of `file` below `root`, with `/` separators.
fn relative_name(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The golden row of one ROM image, named `rel` in the output.
///
/// # Errors
/// [`HarnessError::Audio`] if the core refuses an audio read (it cannot for a mono buffer).
pub fn golden_row(rel: &str, image: &[u8], frames: u32) -> Result<Value, HarnessError> {
    let mut row = Map::new();
    row.insert("rom".into(), json!(rel));
    row.insert("rom_sha256".into(), json!(hex(&Sha256::digest(image))));
    row.insert(
        "mapper".into(),
        InesHeader::try_parse(image).map_or(Value::Null, |header| json!(header.mapper)),
    );

    let name = Path::new(rel)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut core = NesCore::new();
    let load = core.load_rom(image, &name);
    row.insert("load_ok".into(), json!(load.success));
    if !load.success {
        row.insert("load_failure".into(), json!(format!("{:?}", load.failure)));
        row.insert("load_error".into(), json!(load.error));
        return Ok(Value::Object(row));
    }

    row.insert("region".into(), json!(format!("{:?}", load.region)));
    row.insert("sample_rate".into(), json!(core.sample_rate()));
    core.reset();

    let mut recorder = Checkpoints::default();
    let mut audio = AudioHash::new();
    for frame in 1..=frames {
        core.run_frame(0);
        audio.drain(&core)?;

        if CHECKPOINTS.contains(&frame) {
            recorder.record(frame, &core, &audio);
        }
    }

    row.insert("frames".into(), json!(frames));
    row.insert("pixel_sha256".into(), Value::Object(recorder.pixels));
    row.insert("tiles_sha256".into(), Value::Object(recorder.tiles));
    row.insert("cycles".into(), Value::Object(recorder.cycles));
    row.insert(
        "audio_i16_sha256_samples".into(),
        Value::Object(recorder.audio),
    );

    // blargg protocol: $6001-$6003 = DE B0 61, $6000 = status, text from $6004.
    if core.read_memory(0x6001) == 0xDE
        && core.read_memory(0x6002) == 0xB0
        && core.read_memory(0x6003) == 0x61
    {
        row.insert("blargg_status".into(), json!(core.read_memory(0x6000)));
        let text: String = (0x6004u16..0x7000)
            .map(|address| core.read_memory(address))
            .take_while(|&byte| byte != 0)
            .map(char::from)
            .collect();
        row.insert("blargg_text".into(), json!(text));
    }

    Ok(Value::Object(row))
}

/// The values recorded at the checkpoint frames, keyed by the frame number.
#[derive(Default)]
struct Checkpoints {
    pixels: Map<String, Value>,
    tiles: Map<String, Value>,
    cycles: Map<String, Value>,
    audio: Map<String, Value>,
}

impl Checkpoints {
    fn record(&mut self, frame: u32, core: &NesCore, audio: &AudioHash) {
        let key = frame.to_string();
        self.pixels
            .insert(key.clone(), json!(hex(&Sha256::digest(core.pixel_frame()))));
        self.tiles
            .insert(key.clone(), json!(tiles_hash(core.tile_records())));
        self.cycles.insert(key.clone(), json!(core.cycle_count()));
        self.audio.insert(key, json!(audio.snapshot()));
    }
}

/// The running hash of the audio read so far, quantised to 16 bits.
struct AudioHash {
    hasher: Sha256,
    samples: u64,
    buffer: Vec<f32>,
}

impl AudioHash {
    fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            samples: 0,
            buffer: vec![0.0; 8192],
        }
    }

    /// Reads everything the core has produced and folds it in.
    fn drain(&mut self, core: &NesCore) -> Result<(), HarnessError> {
        loop {
            let got = core.read_audio(&mut self.buffer, false)?;
            for &sample in &self.buffer[..got] {
                self.hasher.update(quantise(sample).to_le_bytes());
            }

            self.samples += got as u64;
            if got < self.buffer.len() {
                return Ok(());
            }
        }
    }

    /// `<hex of the hash so far>:<samples so far>`.
    fn snapshot(&self) -> String {
        format!("{}:{}", hex(&self.hasher.clone().finalize()), self.samples)
    }
}

/// A sample as the signed 16-bit value the goldens hash: clamped, scaled in single
/// precision, rounded half to even.
#[allow(clippy::cast_possible_truncation)] // The value is within -32767..=32767.
fn quantise(sample: f32) -> i16 {
    f64::from(sample.clamp(-1.0, 1.0) * 32767.0).round_ties_even() as i16
}

/// Canonical little-endian serialisation of every field of every record, in order, as
/// `<count>:<hex of the SHA-256>`.
fn tiles_hash(records: &[TileRecord]) -> String {
    let mut hasher = Sha256::new();
    for record in records {
        hasher.update(record.plane0.to_le_bytes());
        hasher.update(record.plane1.to_le_bytes());
        hasher.update(record.x.to_le_bytes());
        hasher.update(record.y.to_le_bytes());
        hasher.update(record.index.to_le_bytes());
        let flags = u8::from(record.mirror_h)
            | (u8::from(record.mirror_v) << 1)
            | (u8::from(record.hidden) << 2)
            | (u8::from(record.background) << 3);
        hasher.update([flags, record.palette]);
    }

    format!("{}:{}", records.len(), hex(&hasher.finalize()))
}

fn hex(digest: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    digest
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 0x0F)]),
            ]
        })
        .collect()
}
