# RU-3a — soft reset, frame pacing and GPU queue, APU mixing and MMC5 audio (AMD-044, AMD-052, AMD-054)

Branch `rustupdated`. The application is the Rust workspace `rust/`; the .NET lots V2-5
(`b82d2d3`, AMD-042 + AMD-044), V2-14 (`1a148d7`, AMD-052) and V2-16 (`3d95190`, AMD-054) of
`dotnetupdated` are read as a reference only. Run-ahead (AMD-041) is **not** in this lot (RU-3b).

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, before benchmarks, amendments copied | done — `831d43f` |
| 2 | AMD-044: the soft reset of the core, the command, the input action and the menu | done — `cf75778` |
| 3 | AMD-054: the non-linear mixing, PPU goldens, audio digests refreshed | done — `a2d6d47` |
| 4 | AMD-054: the MMC5's two squares and its PCM channel, at no cost for other boards | done — `a4b4b34` |
| 5 | AMD-052: the catch-up limit and the clock reset on a speed change | done — `db9cac8` |
| 6 | AMD-052: the GPU frame queue and the anti-aliasing default | done — `8bdc37f` |
| 7 | Closing: fmt, clippy, crate tests, full PPU goldens, after benchmarks, journal | done |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Requirement map

### AMD-044 — soft reset

| Requirement | Rust crate / file |
|---|---|
| REQ-CORE-122 core soft reset | `zax-nes/src/nes_core.rs` (`soft_reset`), `cpu/mod.rs` (`soft_reset`), `ppu/mod.rs` (`soft_reset`), `apu/mod.rs` (`soft_reset`), `apu/triangle_channel.rs` (`reset_phase`), `apu/dmc_channel.rs` (`soft_reset`), `input/mod.rs` (`reset`) |
| REQ-CORE-006 amended (frame counter back to 0) | `zax-nes/src/nes_core.rs` |
| REQ-LOOP-072 command | `zax-app/src/emulation/commands.rs` (`Command::SoftReset`), `emulation/host/*` |
| REQ-INP-125 input action | `zax-app/src/input/` (extended actions), key config screen |
| Menu entry and labels | `zax-app/src/ui/` menu bar, `ui/strings` English / French |

### AMD-052 — pacing, GPU frame queue, anti-aliasing

| Requirement | Rust crate / file |
|---|---|
| REQ-LOOP-074 backlog of 2 | `zax-app/src/emulation/frame_pacer.rs` (`MAX_BACKLOG`) |
| REQ-LOOP-075 clock reset on `emu.game_speed` | `zax-app/src/emulation/host/`, `frame_pacer.rs` (`clock_resets`) |
| REQ-LOOP-077 `video.max_queued_frames` | `zax-app/src/graphics.rs` (new GPU frame queue), `zax-render` GL binding, `ui/options.rs` (`EXCLUDED`) |
| REQ-PERS-089 `video.anti_alias` default 1 | `zax-settings/src/table.rs`, `zax-app/src/ui/handheld/defaults.rs` |

### AMD-054 — non-linear APU mixing, MMC5 audio

| Requirement | Rust crate / file |
|---|---|
| REQ-CORE-073 amended, non-linear mixing | `zax-nes/src/apu/mixer.rs` (new), `apu/mod.rs` (`current_level`) |
| REQ-CORE-076 mixer test point | `zax-nes/src/apu/mod.rs` (`mixer_apu`, `mixer_expansion`, `mixer_output`), `zax-harness` |
| REQ-CORE-075 amended, MMC5 audio | `zax-nes/src/apu/mmc5_audio.rs` and `mmc5_pulse_channel.rs` (new), `cartridges/mappers/mmc5.rs`, `nes_bus.rs` |

## Before benchmarks (step 1)

`zax-harness bench-core`, release, x86 desktop, 1200 frames after 120 of warm-up, no profile.
The two MMC5 ROMs are the public test ROMs of `tests/roms` the lot names.

| ROM | core `run_frame` | total |
|---|---|---|
| `mmc5test/mmc5test.nes` (mapper 5) | 1.7909 ms/frame | 1.8442 ms/frame |
| `exram/mmc5exram.nes` (mapper 5) | 2.0429 ms/frame | 2.1991 ms/frame |
| `nestest.nes` (mapper 0) | 0.8695 ms/frame | 0.9371 ms/frame |

## Spec copies

`docs/spec/changes/`: AMD-044, AMD-052, AMD-054, and the three question tickets the .NET lots
wrote whose answers this lot reuses as they stand — Q-078 (soft reset: the PPU's write-inhibit
window is not modelled, the APU frame sequencer is reset), Q-087 (AMD-052: the harness's
anti-aliasing default, the latency criterion's bench, the two instruments), Q-091 (the DC blocker
runs after the resampler, not before). The SPEC chapter edits each amendment implies are applied
in the step that implements the requirement, as RU-2 did.

## Notes

### Reading of the three amendments against the Rust code

**AMD-054, the mixer.** `Apu2A03::current_level` is already lazy: it runs only when a channel
timer moved something or a register was written (`changed || level_dirty`), never per CPU cycle,
and it caches on `level_key`. The non-linear mixing therefore replaces four multiply-adds and a
clamp by two table indices and one addition **in the same place** — the owner's "two lookups per
sample, never per CPU cycle" with the lazy mixing of §5.1 kept exactly as it is. The amendment's
`A` and the float conversion `S / 32768` land there; the DC blocker stays in
`BandLimitedResampler` after the resampling (Q-091), which is also where the Rust port has always
had it.

**AMD-054, how `M` reaches the mixer.** The .NET gives the APU an `IExpansionAudio` reference and
reads it every time the level is recomputed. Rust cannot hand the APU a reference to the
cartridge, and a per-cycle `if board_has_expansion_audio` in `NesBus::tick` is **not** free: the
tick runs ~29 780 times per emulated frame, so three instructions there are ~90 k per frame
against a core of a few million — far over R0's 1 % gate. The design chosen instead:
- `NesBus` keeps a `bool` set once in `attach`, and the push happens inside `catch_up_apu` /
  `sync_apu`, which in the frame loop (`defer_ppu`, the only path gameplay takes) run **rarely**
  for a board without expansion audio. Nothing is added to `tick` itself.
- A board with expansion audio keeps `apu_quiet = 0`, so the APU never lags on it and the `M`
  pushed is always the current cycle's. The quiet-cycle skipping of §5.1 is untouched for every
  other board.
- `tick` steps the cartridge **before** the APU catch-up instead of after (the .NET made the same
  swap). That is a move, not a new branch. The two do not interact other than through the new
  `M`: the DMC DMA check stays after both. The goldens are the proof.

**AMD-052, the catch-up.** `MAX_BACKLOG` is used for two different things in Rust: the spec limit
of REQ-LOOP-012 and the clamp of the Rust-only `emu.catch_up` setting, whose domain is 0 to 5 and
whose handheld default must stay 5 (owner decision; perf history §7 "catch-up 2 on the handheld:
worse than both None and 5"). The two are split: the spec limit becomes 2 (REQ-LOOP-074), the
setting's domain stays 1 to 5.

**AMD-052, the GPU queue on handhelds.** Owner decision, recorded here as an assumed divergence
**(D)**: on PortMaster and Android the value 2 issues **no fence call at all** and keeps today's
behaviour, because the Mali driver keeps three frames of its own and bounding that queue could
cost frames. The value 1 works on every build.

### Test environment

The notes of RU-1 and RU-2 still apply: CMake on `PATH` and `--features zax-app/bundled` for
every `zax-app` or workspace command (SDL2 is linked statically), and the two demo binaries are
local stand-ins. One addition of this lot: **`rust/test-roms/` was empty but for `nestest.nes`**
in this checkout, while the 291-ROM corpus the goldens and the ROM-gated tests read lives in
`tests/roms/` (untracked, left by the .NET layout). It was copied over — `rust/test-roms/` is in
`rust/.gitignore`, so nothing of it is committed — and the goldens then report
`291 of 291 rows matched, 0 ROMs missing`.

### Step 2 — AMD-044, soft reset

Ported from V2-5 with no departure. Core: `NesCore::soft_reset` runs the order of `power_on`
minus everything that clears memory (PPU, APU, input ports, then the mapper, then the CPU's own
seven reset cycles against the already-reset machine), `Cpu6502::soft_reset`,
`Ppu2C02::soft_reset`, `Apu2A03::soft_reset`, `TriangleChannel::reset_phase`,
`DmcChannel::soft_reset`. The two question tickets the .NET lot wrote are reused as they stand
and referenced by `// Q-078` in the PPU's and the APU's methods: the PPU write-inhibit window is
not modelled, and the APU frame sequencer is reset (2A03G).

Two Rust-only points:
- `Ppu2C02::soft_reset` leaves `v_row_start` and `v_row_next` alone. They are the Rust port's
  own caches of `v` (performance history §5.1, the fast span), and `v` is not touched by a soft
  reset; only `t` and `fine_x` are.
- `Cartridge::reset` refreshes the fast ROM windows itself, so the board's after-reset banks
  reach the bus's `read_rom_window` path with nothing extra to do here.

Frontend: `Command::SoftReset` (`emulation.soft_reset`, no default shortcut), the menu entry next
to the hard one, `InputAction::SOFT_RESET` = `0x80000` with no default binding on keyboard or pad,
the `HotkeyEvents::SOFT_RESET` edge, the row in the controls screen among the hotkeys, and the
labels of AMD-044 §3 in both languages (the hard reset becomes `Hard Reset` / `Reset complet`).
`EmulationHost::soft_reset_game` empties the rollback history, the frame steps and the
fast-forward, re-applies the speed, forces the state back to Running and resets the pacing clock,
without ever going through `launch_game`.

Tests: `zax-nes/tests/soft_reset_tests.rs` (9), `zax-nes/tests/cpu_reset_rom_tests.rs` (2),
`req_core_122_the_apu_reset_tests_pass_under_a_soft_reset` in `apu_rom_tests.rs`,
`zax-app/tests/soft_reset_loop_tests.rs` (8). The shell-ROM runner of the test harness gained the
kind of press (`run_shell_rom_soft`): with a soft reset the RAM keeps `$6000 = $81` for a few
frames after the press, so the countdown is armed once per request and the delay is 12 frames
(the readme asks for 100 ms), exactly as the .NET runner does.

Measured, not asserted:
- `cpu_reset/ram_after_reset` and `cpu_reset/registers` **pass** under the soft reset — the
  acceptance criterion of REQ-CORE-122 — and `ram_after_reset` still fails under the hard one,
  so the note of REQ-CORE-004 stays true.
- The six public `apu_reset` ROMs also pass under it, which is what verifies Q-078 point 2.
- PPU goldens: `291 of 291 rows matched` (pixels, tiles, cycles, audio, blargg), unchanged.

Three app tests of the existing suites had to follow the new counts: the command enumeration
(57 → 58), the action table (19 → 20) and the controls screen's hotkey list (5 → 6). The stale
`#[ignore]` note of `req_core_004_apu_reset_needs_a_reset_that_keeps_ram` now points at the soft
reset that does keep both.

SPEC copies: REQ-CORE-006 amended and REQ-CORE-122 added in SPEC-02 §3.12, the last phrase of
REQ-LOOP-050 replaced and REQ-LOOP-072 added in SPEC-07 §3.6, the twentieth row added to the
action table of SPEC-08 §4.1.

### Step 3 — AMD-054, the non-linear mixing

`apu/mixer.rs` holds the two functions, `K`, `Km`, the three integer tables, the round-half-to-
even `r`, `mix`, `clip` and `to_float`. The tables are **`const`**: `const fn` float arithmetic
is stable on the pinned toolchain (1.99.1), so the compiler builds them from the published
formulas. That is better than the .NET's static constructor in two ways — no initialisation
check on the path that reads them, and no magic numbers in the source — and the test
`req_core_073_the_tables_follow_the_documented_formulas` recomputes all 234 entries at runtime
from `95.88 / (8128 / i + 100)` and `163.67 / (24329 / i + 100)` and compares, so the derivation
is checked rather than trusted.

Every figure of the amendment comes out exactly: 4890 for a lone pulse at volume 15, 8462 for
both (ratio 1.7304), 8363 for the triangle alone, `8462 + 24305 = 32767` at full scale,
`K = 32735.8776`, `T(196) = 23905` from the near-tie `23905.4997` (which is what pins the
round-to-even rule), and the 95.52 table of the same nesdev page would have given 4876 / 8438,
which is why the amendment sets it aside.

Where it runs: inside `Apu2A03::current_level`, which was already the lazy path (§5.1, "APU
level mixed only when an input may have changed") and is **not** per CPU cycle. The owner's
"two lookups per sample" lands there, and the change is a net *reduction* in work — four
multiply-adds, an addition chain and a clamp become two array reads and one addition. Q-091 is
reused as it stands and referenced by `// Q-091`: the DC blocker stays after the resampler,
where the Rust port has always had it, so the step synthesis of REQ-CORE-070 keeps its
piecewise-constant input.

REQ-CORE-076, the test point: `mixer_apu()`, `mixer_expansion()` and `mixer_output()` on the
APU. `M` arrives through `set_expansion_level`, which flags the level dirty only when the value
moves; `level_expansion` joins the cache comparison so that a move of `M` alone is noticed.

**The savestate layout is unchanged.** The .NET added three integers to the APU block; here the
three are derived instead — the board owns its own sound state, the bus pushes `M` again at
once, and `current_level` rebuilds all three from the channel outputs just read back. A block
written before AMD-054 therefore still loads (REQ-CORE-091) and `STATE_VERSION` stays 4. The
first attempt dropped `level_key` on load to force that rebuild and broke the
save → load → save byte comparison of `req_core_090_the_mapper_state_survives_a_save_and_restore`;
the fix is to park `level_expansion` at −1, a value no real expansion output can take, which
forces the same rebuild while leaving every saved field as the block gave it.

#### Golden files: what changed and why

`cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored`, 291 ROMs x 1200 frames,
before the digests were touched:

- **174 rows reported `audio differs`, and the classification of every difference line in the
  whole run is `audio differs` — nothing else.** Pixels (`pixel_sha256`), tile records
  (`tiles_sha256`), cycle counts (`cycles`), `blargg_status`, `blargg_text`, `mapper`, `region`,
  `load_ok` are untouched on all 291 rows.
- The 117 rows that matched are the ROMs that stay silent over 1200 frames.
- Cross-checked a second way: `zax-harness golden-core test-roms <out> 1200` was run into a
  fresh file and compared field by field against `goldens/core-goldens.jsonl`. The only key that
  differed anywhere was `audio_i16_sha256_samples`, on the same 174 rows.
- `goldens/core-goldens.jsonl` was then rewritten **in place, one field per line**: the
  `audio_i16_sha256_samples` object of those 174 lines was spliced in and nothing else on any
  line was re-serialised. The goldens then report `291 of 291 rows matched`.
- A note was added at the top of `goldens/README.md`: the audio column is no longer the C# 0.2.0
  value, every other column still is.

Tests ported from V2-16: `apu_channel_tests` and `apu_output_tests` lose the linear step
constants (`0.00752`, `0.00851`, `0.00494`, `0.00335`, gain `1.526`) and read their expectations
from `mixer` instead, since the law is no longer proportional — the triangle staircase, for
instance, is now checked step by step against `T(3t)` rather than for equal spacing. Two
criteria of the amendment replace their predecessors: the peak-to-peak of a lone pulse at the
output of REQ-CORE-071 is **0.149 ± 0.002** (measured in band: the test passes) where it was
0.172 ± 0.02, and the passband test's reference amplitude follows it. `REQ-CORE-076` has its own
test: silence reads `A = M = S = 0`.

`bench-core`, release, x86, same protocol as step 1 (the before figures are in the table above):

| ROM | core before | core after | repeats |
|---|---|---|---|
| `nestest.nes` | 0.8695 ms | 0.8803 ms | 0.8601 / 0.8869 / 0.8683 |
| `mmc5test/mmc5test.nes` | 1.7909 ms | 1.7623 ms | — |

The three repeats of `nestest` span 0.860 to 0.887 ms, so the "after" figure sits inside the
noise of the machine and `mmc5test` moved the other way. No measurable cost, which is what the
replacement of float work by two table reads predicts. R0's callgrind run is the real gate.

### Step 4 — AMD-054, the MMC5's audio

`apu/mmc5_audio.rs` and `apu/mmc5_pulse_channel.rs`, ported from V2-16 with the same reading of
the requirement: two square channels that borrow the APU pulse's waveform, duty, timer,
envelope, length counter and cut below period 8 but have **no sweep unit** (a separate type
rather than a flag, since an APU pulse with a zeroed sweep register mutes itself above period
$3FF and an MMC5 channel must not), a PCM channel in write mode, and $5010 / $5015. The board's
own divider gives the envelopes 7 458 CPU cycles (239.98 Hz) and the length counters one clock in
two (120 Hz), which is the **(D)** of the requirement against the 240 Hz of the public page. The
three other declared divergences are kept as the requirement writes them: no read mode of
$5010, the PCM IRQ flag never pulls /IRQ, and the period cut follows the APU.

#### How `M` reaches the mixer at no cost for other boards

The design the notes above describe, implemented:
- `Mapper::expansion_audio_output` returns `Option<i32>`, `None` by default; only the MMC5
  overrides it. The MMC5 also takes the `on_cpu_cycle` hook it had never used — it is one of the
  two boards that already step every cycle, so no board that did not step gained a step.
- `NesBus` keeps a `bool` set once in `attach` from `Cartridge::has_expansion_audio`. It is read
  in `catch_up_apu` and `sync_apu`, never in `tick`. In the frame loop — the only path gameplay
  takes — `catch_up_apu` runs **rarely** for a board without expansion audio, so the `bool` test
  is not on the per-cycle path at all. A `bool` test *in* `tick` was rejected: at about 29 780
  cycles per emulated frame it would add roughly 90 k instructions per frame, far over R0's 1 %.
- A board with expansion audio keeps `apu_quiet = 0`, so the APU never lags on it and the `M`
  pushed is always the current cycle's. For every other board the quiet-cycle skipping of §5.1 is
  exactly as it was.
- `tick` now steps the cartridge **before** the APU catch-up instead of after. That is a move of
  an existing block, not a new branch; the .NET lot made the same swap for the same reason. The
  two chips do not look at each other and the DMC DMA is still served after both. **The goldens
  prove it: 291 of 291 rows still match, audio included**, so the reorder changed nothing on any
  of the 291 ROMs.

#### The savestate format

The board's sound is part of the state (REQ-CORE-090), written at the end of the MMC5's own
registers, so `STATE_VERSION` had to move: **4 → 8**, and the widescreen variant **7 → 9**.
Versions 4, 5, 6 and 7 are still read (REQ-CORE-091) — a block of theirs simply carries no sound
state and the board starts silent, as it does after a reset — so no savestate a user already has
is refused. The version reaches the board through a new `StateReader::format_version`, set once
from the header in `NesCore::restore_state`: one additive field on the reader, no signature
change anywhere in the 26 mappers. `is_known_state_version` and
`state_version_has_mmc5_audio` hold the two decisions; the widescreen call's third argument
became `version >= WIDE_STATE_VERSION_7`, which keeps 7 and 9 on the same branch.

#### Measurements

Every acceptance figure of REQ-CORE-075 comes out exactly: one square at volume 15 → `M` = 4623,
both → 8000, `pcm` 128 → 3968, a 0 written to $5011 leaves the level and raises the flag, the
first read of $5010 gives $80 and the next $00, the envelope falls 15 → 0 in 15/240 s ± 5 % for
`$4017` = $00, $80 and $C0 alike, and the length counter runs out in 10/120 s. The clipping is
checked where it bites: `A` at 26 838 plus `M` at 7 905 is past full scale and `S` reads 32 767.

`bench-core`, release, x86, against the step-1 figures:

| ROM | core before | core after | repeats after |
|---|---|---|---|
| `mmc5test/mmc5test.nes` | 1.7909 ms | 1.8752 ms | 1.8316 / 1.8515 / 1.8215 |
| `exram/mmc5exram.nes` | 2.0429 ms | 2.0596 ms | — |
| `nestest.nes` (mapper 0) | 0.8695 ms | 0.8646 ms | 0.8544 / 0.8535 |

`nestest` did not move (it sits under the step-1 figure, inside the 0.853 to 0.887 span the
machine shows), which is the requirement that mattered: **no cost for a cartridge that is not an
MMC5**. The MMC5 ROM costs about 3 % more — the per-cycle sound step, plus the APU's quiet-cycle
skipping being off for that board so that `M` is never stale. That is the board that gained the
feature, and no built-in profile targets an MMC5-audio game (note of AMD-054).

Tests: `zax-nes/tests/mmc5_audio_tests.rs`, 14 of them, including a savestate round trip over
256 cycles of `M` and a block whose header is patched back to version 4 to show that an older
state still loads with the board silent. One test takes the other side: for mappers 0, 1, 4 and
7, `expansion_audio_output` is `None` and writes in the $5000-$5015 window reach no sound unit.

SPEC copies: REQ-CORE-073 replaced by its amended form and REQ-CORE-076 added in SPEC-02 §3.7,
REQ-CORE-075 replaced by its amended form, the mapper 5 row of REQ-CORE-100 now reads "audio
émulé", and Q5 is marked as superseded for the MMC5.

### Step 5 — AMD-052, the cadence

**The two limits are split**, which is the one place this lot departs from the .NET lot. In Rust
`MAX_BACKLOG` served both as the spec's limit and as the clamp of the Rust-only `emu.catch_up`
setting, whose handheld default must stay 5. So:
- `MAX_BACKLOG` = **2** (REQ-LOOP-074, replacing the 5 of REQ-LOOP-012), the pacer's default
  `max_backlog` and the figure the vectors measure;
- `MAX_CATCH_UP` = **5**, the largest value `emu.catch_up`, `--catch-up` and `ZAXNES_CATCH_UP`
  accept, unchanged.

Defaults: `emu.catch_up` becomes **2** on the desktop (`zax-settings/src/table.rs`) and the
handheld overrides it back to **5** (`ui/handheld/defaults.rs`, now ten entries), as the owner
decided. The comment there carries the reason from the performance history §5.5 and §7: a limit
of 2 measured *worse* on the device than both None and 5 once the core had its own thread,
because the second frame of a pair waits for the emulation thread in series.

REQ-LOOP-075: `FramePacer::reset` counts its calls (`clock_resets`, the instrument Q-087 point 3
asks for), and `on_setting_changed` resets the clock when `emu.game_speed` changes. The
fast-forward cycle of REQ-LOOP-016 reaches `apply_speed` without going through
`on_setting_changed`, so arming, expiring and re-arming a burst leaves the clock alone — which is
exactly the exclusion the requirement spells out, and the reason for it: a burst of 30 emulated
frames at x1.5 lasts about 0.33 s, so a hold produces about six state changes a second.

`FramePacer::new` deliberately does **not** go through `reset`: starting the clock is not one of
the resets REQ-LOOP-013 lists, so the counter begins at 0.

Tests: `zax-app/tests/amd052_cadence_tests.rs`, 6 of them, all passing.
- REQ-LOOP-074's amended criterion: a 500 ms block at 60 Hz runs **2** frames in a burst, no
  second burst follows, and the run ends 28 frames short of an unblocked one (measured 28).
- The amended vector T5: a 1 s block catches **2** frames up and drops 58.
- REQ-LOOP-075 first criterion: normal → x2 → back, on the clock alone and through the loop
  (options write, `SpeedDown` shortcut), each change bumping `clock_resets` by exactly one and
  no tick afterwards running a burst.
- REQ-LOOP-075 second criterion: the fast-forward hotkey held 5 s → `clock_resets` unchanged,
  about 450 emulated frames ±5 %, and 95 % of the gaps between consecutive emulated frames
  inside `1/90 ± 2 ms` during the hold and `1/60 ± 2 ms` after the release.
- REQ-LOOP-077's setting and its last criterion (600 ± 1 emulated frames over ten seconds of
  rendered frames at 30 Hz, at most one frame dropped — the reading Q-087 records).

Three existing tests had to follow the new default, each for a reason that is not about this
amendment: `catch_up_tests` now distinguishes the two constants, `pause_3d_tests` asks for a
catch-up of 5 because what it measures is what a *multi-frame* catch-up converts, and the
settings layering test writes 3 rather than 2 because 2 is now the default and writing a default
records nothing.

`AppSettings::max_queued_frames` / `set_max_queued_frames` were added here, the setting's
accessor pair; its effect is step 6.

SPEC copies: SPEC-07 §3.2 — REQ-LOOP-012's statement and criterion amended, the `emu.game_speed`
line added to REQ-LOOP-013, REQ-LOOP-074 and REQ-LOOP-075 added, and the T5 row of §5 rewritten.
SPEC-09 — the `video.anti_alias` row takes default **1**, REQ-PERS-089 is added after
REQ-PERS-024, and §4.6 gains the `zaxnes_video_anti_alias` line.

### Step 6 — AMD-052, the GPU frame queue and the anti-aliasing default

**Where the GL calls live.** `zax-app` holds **no `unsafe` at all**, and the workspace's rule is
that GL calls are confined to `zax-render/src/gl/api.rs` (its module header says so). The three
fence calls went there as safe methods with the module's `SAFETY:` contract — `fence_sync`,
`wait_for_fence`, `delete_fence` — and the queue itself is a new `zax_render::sync::GpuFrameQueue`
over them. So the app gained a type, not an `unsafe` block, and the lot's "safe Rust" rule holds
without an option or a second path.

`GpuFrameQueue` is the .NET `GpuFrameQueue` of V2-14: at most two outstanding fences, oldest
first; `begin_frame(depth)` retires until fewer than `depth` are left; `end_frame` places the
fence; a driver that refuses one leaves the queue unbounded, which is depth 2's behaviour anyway.
The wait is sliced at 100 ms, 20 times, so a lost fence cannot hang the program, and
`WAIT_FAILED` counts as signalled for the same reason. `Drop` flushes. `glClientWaitSync` is
called with `GL_SYNC_FLUSH_COMMANDS_BIT`, so the fence can never wait on an unflushed queue.

In the loop (`application.rs`): `wait_for_the_gpu_queue` runs **after** the emulated frames of
the tick and before the first drawing command, `close_the_gpu_queue` after the last one. That
placement is the requirement's: the frame clock is absolute, so the wait cannot shift the
emulated cadence (REQ-LOOP-010) and is not a delay in the sense of REQ-LOOP-074 — it only eats
slack. The wait is counted under the profiler's `Wait` stage. The queue exists only for the
OpenGL renderer; the wgpu backend presents through its own swap chain.

**Handheld (D), owner decision.** `bound_gpu_queue` returns `None` — **no fence call at all** —
on a handheld at the default depth of 2, so those builds keep exactly today's behaviour: that
depth is the driver's own queue, and the Mali keeps three frames of its own, so bounding it could
cost frames rather than latency. The depth of 1, which is what a player picks to lower the
latency, works on every build. Recorded here and in the notes of the spec copy.

**The anti-aliasing default** (REQ-PERS-089, a divergence **(D)**): `video.anti_alias` goes from
0 to **1**, which is MSAA 4× by AMD-032 §2. The handheld override to 0 was already in
`ui/handheld/defaults.rs` from an earlier lot, so handhelds are unaffected. Q-087 point 1 is
followed: the harness's `--anti-alias` default now **reads the setting's own default from the
catalogue**, so the two cannot diverge and a plain `zax-harness render` reports MSAA 4×. The
synthetic-capture golden test passes `--anti-alias 0` explicitly, since the C# goldens it
compares against were captured with the anti-aliasing off — without that it would be comparing
multi-sampled captures with single-sampled references.

`video.max_queued_frames` leaves `EXCLUDED` (37 entries now) and gets its options-window label in
both languages, `Max Queued Frames` / `Frames en file GPU`, placed after "Render rate" as
AMD-052 §3 point 10 asks.

Three tests joined `amd052_cadence_tests.rs` (9 in all): REQ-PERS-089 (the default is 1, 1 means
4 samples, a saved 0 wins over it, the handheld stays at 0), the two catch-up defaults (2 desktop
/ 5 handheld), and REQ-LOOP-077's interface (out of `EXCLUDED`, shown by `is_shown`, labelled in
both languages).

Four existing tests had to follow the new default: the two `video_option_tests` anti-aliasing
ones (writing the value a setting already holds is not a change and dispatches nothing, so the
first write is now to 0), the catalogue reference table of SPEC-09 §4.1.1, and the
synthetic-render golden test above.

**Not done here, and why.** The fence criterion of REQ-LOOP-077 (a fence of frame *n* signalled
before the first command of frame *n + 1* at depth 1) and its latency bench need a live GL
context; Q-087 point 2 settles how the latency is to be measured — the bench has to be GPU-bound
or the comparison inverts — and the .NET lot put both in its acceptance suite rather than its
unit tests. They belong to R0's run on real hardware, as the `ZAXNES_3DSEN_INSTALL`-gated tests
do. The queue's own logic (depth, retire order, outstanding count, latency clock) is exercised by
the loop every frame of a desktop run.

### Step 7 — closing

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --release --features zax-app/bundled -- -D warnings` | clean |
| `cargo test --release -p zax-nes` | 482 passed |
| `cargo test --release -p zax-app --features bundled` | 628 passed |
| `cargo test --release -p zax-settings` | 221 passed |
| `cargo test --release -p zax-render` | 148 passed |
| `cargo test --release -p zax-scripting` | 155 passed |
| `cargo test --release -p zax-engine3d` | 194 passed |
| `cargo test --release -p zax-profiles` | 178 passed |
| `cargo test --release -p zax-platform --features bundled` | 35 passed |
| `cargo test --release -p zax-harness` | 40 passed |
| `cargo test --release -p zax-goldens` | 3 passed |
| `cargo test --release -p zax-achievements` | 3 passed |
| **total** | **2087 passed, 0 failed** |
| `cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored` | **291 of 291 rows matched**, 0 missing |

`zax-platform` needs `--features bundled` like `zax-app`: its test binary links SDL2 too.

#### After benchmarks

`bench-core`, release, x86, 1200 frames after 120 of warm-up, same protocol as step 1.

| ROM | core before | core after | total before | total after |
|---|---|---|---|---|
| `nestest.nes` (mapper 0) | 0.8695 ms | **0.8564 ms** | 0.9371 ms | 0.9239 ms |
| `mmc5test/mmc5test.nes` (mapper 5) | 1.7909 ms | **1.8000 ms** | 1.8442 ms | 1.8528 ms |
| `exram/mmc5exram.nes` (mapper 5) | 2.0429 ms | **2.0582 ms** | 2.1991 ms | 2.2206 ms |

`nestest` came out *below* its step-1 figure and inside the machine's own span (0.853 to 0.887 ms
over five readings), so nothing that is not an MMC5 pays for this lot — the requirement that
mattered. The MMC5 ROMs are 0.5 to 1 % up on these readings (1.80 to 1.88 over four), from the
board's per-cycle sound step and from the APU's quiet-cycle skipping being off for that board so
that `M` is never stale. R0's callgrind run on a profile that uses nothing new is the real gate.

#### What the next lot must not undo

`rust/docs/PERFORMANCE-HISTORY.md` was updated rather than left to drift, since its whole purpose
is that nothing in it is silently lost: §5.1 now records that the non-linear tables live in the
lazy mixing path and that only the MMC5 loses the quiet-cycle skipping, and §5.5 records the two
catch-up defaults (2 desktop / 5 handheld, with the measurement behind the 5), the clock reset on
a speed change, and the GPU frame queue with its handheld **(D)**.

#### Assumed divergences (D) this lot adds

1. **The GPU frame queue on handhelds** (owner decision): at the default depth of 2, a PortMaster
   or Android build issues no fence call at all and keeps today's behaviour. Depth 1 works
   everywhere.
2. **`video.anti_alias` shipped at 1** (REQ-PERS-089, the amendment's own **(D)**): the original
   forces its anti-aliasing off and defaults to 0. Handhelds keep 0.
3. The four divergences REQ-CORE-075 declares for the MMC5's sound are implemented as the
   requirement writes them (length counters at 120 Hz, no read mode of $5010, the PCM IRQ flag
   does not drive /IRQ, the period cut below 8 follows the APU).

#### Left to R0

The fence criterion of REQ-LOOP-077 (at depth 1 the fence of frame *n* is signalled before the
first command of frame *n + 1*) and its latency bench, which Q-087 point 2 settles must be
GPU-bound or the comparison inverts. Both need a live context on real hardware, which is where
the .NET lot put them too. Everything else in the three amendments is covered here.

No new Q ticket: the three ambiguities in this scope were already answered by Q-078, Q-087 and
Q-091, whose choices are taken as they stand and referenced by `// Q-078` and `// Q-091` in the
code.
