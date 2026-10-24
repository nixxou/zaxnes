# Goldens of the C# reference (Zaxnes 0.2.0)

Produced on Windows x64 from the C# implementation at tag `v0.2.0`. They contain only outputs of this project's own code, computed over **public** test ROMs and built-in synthetic scenes: no ROM, no game profile, no commercial data.

## core-goldens.jsonl

One JSON object per line, one line per `.nes` file of the public test-ROM set (`rust/test-roms/`, 291 files), sorted by path (ordinal).

| Key | Meaning |
|---|---|
| `rom` | path relative to the test-ROM folder, `/` separators |
| `rom_sha256` | SHA-256 of the file, to check you run the same ROM |
| `mapper` | iNES mapper number read from the header (`null` if the header is not iNES) |
| `load_ok` | `NesCore.LoadRom` result; when false: `load_failure`, `load_error` |
| `region`, `sample_rate` | after loading (`sample_rate` is the default output rate) |
| `frames` | frames run (1200) |
| `pixel_sha256` | `{frame: sha256}` of `NesCore.PixelFrame` (256×240 bytes, PPU colour indices as the C# stores them) after that frame |
| `tiles_sha256` | `{frame: "<count>:<sha256>"}` of the tile records of that frame (serialisation below) |
| `cycles` | `{frame: NesCore.CycleCount}` after that frame |
| `audio_i16_sha256_samples` | `{frame: "<sha256>:<samples>"}`, running hash of all audio since reset (rule below) |
| `blargg_status`, `blargg_text` | when `$6001-$6003` = `DE B0 61` after the last frame: `$6000` and the zero-terminated text from `$6004` |
| `exception` | only if the C# threw (none did) |

Checkpoints: frames 1, 60, 300, 600, 1200 (counted from 1 after `Reset()`).

**Run protocol**: `new NesCore()`, `LoadRom(bytes, file name without extension)`, `Reset()`, then for each frame `RunFrame(0)` (no button pressed) followed by draining `ReadAudio(buffer of 8192 floats, mono)` until it returns fewer samples than the buffer size (or 0).

**Audio quantisation**: each float sample `s` → `v = round_half_to_even(clamp(s, -1, 1) × 32767)` as `i32`, appended as 2 bytes little-endian (`v & 0xFF`, `(v >> 8) & 0xFF`) to one running SHA-256. C# `Math.Round(float)` rounds half to even; the multiplication is done in `float`, then `Math.Round` on the `float` promoted to `double`: use `((s.clamp(-1.0, 1.0) * 32767.0_f32) as f64).round_ties_even() as i32` in Rust.

**Tile record serialisation** (per record, in order, little-endian): `plane0` u64, `plane1` u64, `x` i16, `y` i16, `index` i32, one flag byte (`mirror_h`=1 | `mirror_v`=2 | `hidden`=4 | `background`=8), `palette` u8. Hash the concatenation of all records of the frame; prefix with the record count.

Blargg statuses are a snapshot, not a pass/fail list: `0` = passed, `0x80` = still running at frame 1200, `0x81` = needs a reset press; other values are the test's failure codes. Parity means the Rust core gives the same values.

## render-synthetic/

`Zax.Harness render --frames synthetic` at 640×480: one PNG per built-in synthetic scene and `render.json` (scene list, sizes, counts). GPU output: compare with a per-channel tolerance (2) rather than bit for bit.

## lua-api.txt

`Zax.Harness lua-api`: the script-visible API (globals, constants, members of each type), one line each. The Rust dump must be identical.

## tool/

Source of the C# program that produced `core-goldens.jsonl` (`dotnet run -c Release --project rust/goldens/tool -- <test-roms dir> <out.jsonl> 1200`; the `mapper` key was added afterwards from the iNES header). It references `src/Zax.Nes`, which is on the `archive-dotnetversion` branch.
