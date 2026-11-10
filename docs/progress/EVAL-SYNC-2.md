# EVAL-SYNC-2 — where Zaxnes parts from NesHawk on an MMC3 game with resets

Branch `eval/bk2`, worktree `cleanroom/wt-bk2`, base `f1061097` (lot EVAL-SYNC).

EVAL-SYNC concluded that Zaxnes replays a NesHawk `.bk2` exactly, from **one** movie — *Faxanadu*,
MMC1, no resets — and said in so many words that an MMC3 game had not been tried. The owner then
checked a second `.bk2` that **does** sync to the end on BizHawk 2.11.1 and found that Zaxnes
desyncs on it before frame 1 000. This lot is that second movie.

| | |
|---|---|
| the movie | `G:/3DSen/test-movies/bk2/keyword-kirbysadventure-noabilities.bk2` |
| | TASVideos 6192M, BizHawk 2.8, NesHawk, 156 312 frames, **seven Reset presses**, the first at frame 3 |
| the ROM | `Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes`, MMC3/TxROM |
| the reference | `G:/3DSen/test-movies/bizhawk-runs/kirby-jp.txt` |
| the calibration | `fax-usa.txt`, `fax-rev1.txt` — the two runs EVAL-SYNC showed Zaxnes already matches |

This is an **evaluation**, like EVAL-SYNC: it measures and explains, and it changes no emulation in
`zax-nes`.

## Steps

- [x] 1. A RAM trace in `movie-sync`, calibrated on the two *Faxanadu* references.
- [ ] 2. The first divergence on Kirby, narrowed to the frame.
- [ ] 3. The cause, with a class for each finding.
- [ ] 4. Re-check: Kirby after any player fix, and *Faxanadu* to show nothing regressed.
- [ ] 5. The report.
- [ ] 6. The final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| the fork, MSVC | `cleanroom/fceux-zax/build-msvc`, the existing build — **not rebuilt** |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |

## What the inputs say, read before anything was run

### The movie

`Header.txt`, in full:

| key | value |
|---|---|
| `MovieVersion` | `BizHawk v2.0` |
| `emuVersion` / `OriginalEmuVersion` | `Version 2.8` |
| `Platform` | `NES` |
| `GameName` | `Hoshi no Kirby - Yume no Izumi no Monogatari` |
| `SHA1` | `63DCAB3395AB9D1CF98CC93BFC872C29FCA6E6F3` |
| `BoardName` | `TxROM` |
| `Core` | `NesHawk` |
| `rerecordCount` | `155556` |

`SyncSettings.json` carries `"BoardProperties":{}`, `"RegionOverride":0`,
`"NesLeftPort":"ControllerNES"`, `"NesRightPort":"UnpluggedNES"`, `"Famicom":false`,
`"FamicomExpPort":"UnpluggedFam"` and **`"InitialWRamStatePattern":null`** — every one of them a
value `crates/zax-harness/src/bk2.rs` already accepts, which is why the reader takes the file.

The `LogKey` is the one EVAL-SYNC's reader was written against:
`#Power|Reset|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|P1 B|P1 A|`. No `Power` press.
**Seven `Reset` presses**, each exactly one frame long, at the 0-based frame indices

    3, 22554, 25798, 29798, 62576, 87950, 92437

### The reference trace

`kirby-jp.txt`: 1 564 numeric lines, frames 100 to 156 300 every 100, then
`end frame 156312 mode FINISHED`. Its lag counter is **6 at frame 100** and only **24 at frame
156 300** — a TAS that lags 24 frames in 156 312. The two *Faxanadu* references are 833 lines each
and end on `lag=13398`, which is the figure EVAL-SYNC's probe reproduced exactly.

---

## Step 1 — the RAM trace, and the setup difference the calibration found

`crates/zax-harness/src/ram_trace.rs` (the trace, the reference reader and the comparison) with
four options in `movie-sync`:

```
  --trace <file.txt>   a work-RAM checksum every --trace-step frames with the running count of
                       input-insensitive frames, in the reference traces' own shape
  --trace-step <n>     how often a line is written (default 100)
  --trace-ref <file>   a BizHawk reference trace to hold the run against, line for line
  --dump <dir> --dump-frames a,b,c   those frames' 2 KiB of work RAM, per engine, raw
```

The checksum is the owner's, reproduced rather than improved on: `s = s*31 + byte` over
`$0000`-`$07FF`, modulo 2^32, eight lower-case hex digits. `--dump` is there because a checksum
says two machines differ and only the bytes say **how** — and it earned its place twice in this
step.

### The offset is measured, not assumed

`RamTrace` keeps **every** frame's checksum, not only the one line in a hundred it writes out, so
the comparison can try a range of offsets (±3) and *report* the one that fits. The answer, on both
*Faxanadu* references:

| | |
|---|---|
| the offset | **0** — the reference's frame *f* is this run's machine after *f* frames |
| the runner-up, at either ±1 | **0** lines of 832 agree |

So the two numberings line up with no shift, which is what `emu.framecount()` being 0 before the
first input word implies — but it is now a measurement, and the runner-up's 0 is the margin that
makes it one.

### What the calibration found on the way: NesHawk's power-on work RAM is **not** zeroes

The first calibration run matched **nothing**: 0 of 832 lines, first difference at frame 100.
Every one of the 832 **lag counts agreed exactly**, which is a run in step, so the 2 KiB was the
thing to look at. Three facts, in the order they came:

1. the difference between this run's checksum and the reference's was the **same constant**
   `0x29F49E` at frames 100, 200 and 300 — a fixed set of addresses holding fixed different
   values, not a drift;
2. over the whole 83 218 frames that constant takes only 336 distinct values on 832 lines, in long
   stretches (one of them 130 lines from frame 39 700 to the last) — the signature of a handful of
   bytes, not of a desynchronised machine;
3. of the three deterministic fills the instrument can write, **only one puts any line in step**:

| `--ram` | lines of 832 agreeing, `fax-usa` | `fax-rev1` |
|---|---|---|
| `zero` (what EVAL-SYNC's `.bk2` reader used) | **0** | 0 |
| `ff` | **0** | 0 |
| `default` — `00 00 00 00 FF FF FF FF` | **207** | **220** |

So `NesHawk` powers the work RAM on with the `00 00 00 00 FF FF FF FF` pattern, the same one
FCEUX's `RAMInitOption 0` writes. That is a **class (c) setup difference** in the harness's own
`.bk2` reader, and it is fixed there: `bk2::read` now asks for `RamPattern::FceuxDefault`, with
the measurement in the note it prints rather than a claim about a core nobody here may read.
`SyncSettings.json`'s `InitialWRamStatePattern` — the one field of the format that speaks about
the 2 KiB, and `null` in both movies in hand — is now read too, and **refused by name** when it
is anything else, because the public description does not say how a non-null value names a fill
and a wrong 2 KiB is not something emulation can recover from.

Neither *Faxanadu* run's picture changed: the last picture is still `03d7e6a1…` on `[USA][!!]`
and `13920624…` on `[Rev 1][!!]`, byte for byte what EVAL-SYNC measured with all-zero RAM. This
game does not read its uninitialised RAM, which is why EVAL-SYNC's six-screenshot match held
anyway.

### So do the two *Faxanadu* traces match line for line? No — and exactly why not

| | `fax-usa` | `fax-rev1` |
|---|---|---|
| lines compared | 832 | 832 |
| checksum lines agreeing | **207** | **220** |
| **lag counts agreeing** | **832 of 832** | **832 of 832** |
| the last agreeing checksum line | **83 200 — the last one** | **83 200** |
| the first differing line | 700 | 700 |

The differing lines are **the same frames on both dumps** (700, 800, 900, 1400, 1500, 1600,
1800, 2000, …), and they cluster on the reference's own lag frames: the reference's lag counter
steps at 700, 900, 1000, 1400, 1600 and 1700, which are differing lines, while the quiet stretches
between them agree.

What the residual is was then settled rather than argued. For each of the 625 differing lines the
delta was solved for a **single** differing byte — for each address *a*, whether
`delta · 31^-(2047-a)` is a value in `-255..255`:

| | `fax-usa` | `fax-rev1` |
|---|---|---|
| differing lines | 625 | 612 |
| of those, explained by **one byte** | **54** | **73** |
| expected by chance over that many lines | **0.15** | 0.15 |
| the page those bytes are in | **`$0100` — the stack — 54 of 54** | `$0100`, 71 of 73 |
| the addresses | `$01D7`-`$01FB` | `$01D7`-`$01FB`, and `$0006`/`$0007` twice |

54 single-byte explanations where chance predicts 0.15, every one of them in the top of the stack
page: the two machines are reading the 2 KiB at a **different point inside the frame**, so the
leftovers below the stack pointer — the bytes an interrupt pushed and the program has popped —
differ. That is a property of where each emulator ends a frame, not of what it emulated: 207
lines agree exactly, including the last; every one of the 832 lag counts agrees; and EVAL-SYNC's
six screenshots are identical.

For what follows, that fixes what a *real* divergence looks like in this instrument and what it
does not: a run in step scores a quarter of its lines, keeps its lag column exactly, and keeps
agreeing to the end. **The last agreeing line is the marker**, not the first differing one.

### Checked

| | |
|---|---|
| `cargo fmt --check` | clean |
| clippy `-D warnings --all-targets`, feature **off** and **on** | clean |
| `zax-harness` lib tests | **42**, up from 31: 9 new in `ram_trace`, 2 new in `bk2` |
| `movie_sync_tests`, feature off | **9**, 3 of them new |
