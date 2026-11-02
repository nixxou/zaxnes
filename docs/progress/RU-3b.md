# RU-3b — run-ahead (AMD-041)

Branch `rustupdated`, on top of RU-3a-fix (`ce78abd`). The application is the Rust workspace
`rust/`; the .NET lot **V2-3** (`b599e6c`, AMD-041) of `dotnetupdated` is read as a reference
only — it has no `docs/progress/V2-*.md` file (those start at V2-21), so its diff is the whole
reference.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, design decisions, before benchmarks, spec copies | done — `0d312e2` |
| 2 | REQ-CORE-121: the in-memory state of the core, and its tests | done — `08ebd15` |
| 3 | REQ-LOOP-065: the frame sequence in the core, N = 0 untouched | done — `c49033b` |
| 4 | REQ-LOOP-068: the frames that are not displayed (no sound, the pixels measured) | done |
| 5 | REQ-LOOP-066 / 067: the session, persistence, Lua and conversion rules + tests | todo |
| 6 | REQ-LOOP-064: desktop options row, handheld advanced option, harness `--run-ahead` | todo |
| 7 | Closing: benches, `PERFORMANCE-HISTORY.md`, fmt, clippy, full tests, goldens | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Requirement map — AMD-041 onto the Rust crates

| Requirement | Rust crate / file |
|---|---|
| REQ-CORE-121 in-memory state | `zax-nes/src/state.rs` (`CoreState`), `nes_core.rs` (`capture_core_state`, `restore_core_state`, `max_core_state_size`), `ppu/state.rs` (the machine / frame-output split), `apu/mod.rs` (a restore that keeps the queued samples) |
| REQ-LOOP-065 the five steps | `zax-nes/src/nes_core.rs` (`run_frame_ahead`, `restore_real_frame`) — in the **core**, so that the "emulation on a separate core" thread can run the sequence too (`zax-app/src/core_worker.rs`) |
| REQ-LOOP-065 step 1 and 3, no sound | `zax-nes/src/apu/mod.rs` (`SampleRing::muted`, `Apu2A03::set_output_enabled`) |
| REQ-LOOP-068 pixels / Zapper | `zax-nes/src/ppu/render.rs` — **measured, then not done**: every emulated frame renders its pixels (see the notes of step 4) |
| REQ-LOOP-060 amended, REQ-LOOP-067 | `zax-app/src/game_session.rs` (`set_run_ahead`, the restore sites), `emulation/host/mod.rs` (the setting read once per real frame) |
| REQ-LOOP-066 what follows the real frame | `zax-app/src/emulation/host/mod.rs` (rollback snapshot, frame steps), `emulation/host/save_states.rs`, `zax-nes/src/movie.rs` (input recording) — all of them already read `NesCore::frame_count`, which stays the real one |
| REQ-LOOP-064 the setting | `zax-settings/src/table.rs` + `catalogue.rs` (`emu.run_ahead`, already in by RU-2), `zax-app/src/ui/options.rs` (out of `EXCLUDED`), `ui/strings/*`, `ui/handheld/options.rs` |
| REQ-LUA-114 `Setting.RunAheadFrames` | `zax-scripting/src/api/settings.rs` — already in by RU-2, nothing to add |
| REQ-ARCH-012 / 013 amended | `zax-harness` (`--run-ahead` on `render` and `bench-core`, N in the exports), `golden-core` stays at 0 |
| REQ-ACC-030 precised | the bench protocol of step 7 reports N = 0, 1, 2 and 3 |

## Before benchmarks (step 1)

`zax-harness bench-core`, release, x86 desktop, 1200 frames after 120 of warm-up, no profile,
three runs each. No commercial ROM is present in this checkout (`rust/test-roms` holds the 69
public test folders and `nestest.nes` only), so `nestest` stands in for SMB1 / MM2 / Zelda II as
the lot allows, with the two MMC5 ROMs beside it because they are the heaviest boards available.

| ROM | core `run_frame` | total |
|---|---|---|
| `nestest.nes` (mapper 0) | 0.8410 / 0.8434 / 0.8519 ms | 0.9068 / 0.9084 / 0.9192 ms |
| `mmc5test/mmc5test.nes` (mapper 5) | 1.8366 / 1.8432 / 1.8505 ms | 1.8890 / 1.8963 / 1.9042 ms |
| `exram/mmc5exram.nes` (mapper 5) | 2.0499 / 2.0449 / 2.0523 ms | 2.2074 / 2.2042 / 2.2113 ms |

These are the N = 0 figures the "N = 0 costs nothing" rule is measured against at the close.

## Spec copies

`docs/spec/changes/AMD-041-run-ahead.md`, plus the SPEC chapter edits it implies, applied in the
step that implements each requirement as the .NET lots did: SPEC-02 §3.12 (REQ-CORE-121),
SPEC-07 §3.4 and §3.5 (REQ-LOOP-060 amended, REQ-LOOP-064 to 068), SPEC-01 (REQ-ARCH-012/013),
SPEC-10 (REQ-ACC-030), SPEC-09 §4.1 and §4.2 (the `emu.run_ahead` row — already written by RU-2).

## Notes

### Reading of AMD-041 against the Rust application

**Two different things are called "ahead", and both must keep working.**

| | AMD-041 run-ahead (this lot) | "emulation ahead" (Rust-only, already in) |
|---|---|---|
| What it is | the displayed frame is N frames in the future of the real one | the next frame is computed while the current one is drawn |
| What it changes | input latency: the player sees their pad N frames earlier | throughput: the core's time leaves the main thread's budget |
| Cost | N extra emulated frames **plus** one state saved and restored, per real frame | none; it is pipelining |
| Where | `NesCore::run_frame_ahead` | `GameSession::prepare_frame_timed` / `launch_ahead`, `CoreWorker` |
| Setting | `emu.run_ahead` (0-3, desktop 1, handheld 0) | `ZAXNES_SEPARATE_CORE`, `ZAXNES_CORE_AHEAD`, `emu.pipeline` |

They compose rather than conflict: run-ahead is **inside** one "produce one frame of the core"
step, which is exactly the unit the emulation thread runs. That is why the sequence of
REQ-LOOP-065 lives in `NesCore` and not in the session — the session would have to reach into a
core that has been moved to another thread.

**Where the restore (step 5) goes.** REQ-LOOP-067 allows the restoration to be delayed past step
(d) — "l'implémentation garde l'état de la frame affichée pour servir ces lectures, ou retarde la
restauration après (d) ; l'ordre est libre tant que la frame réelle suivante repart de R". The
Rust application does not use the core's video callback for the conversion: `finish_frame_lent`
pulls `core.frame_batch()` *after* `run_frame` returns, and the frame hooks then read the live
core through `Script:ReadMem`. So delaying the restore is not merely allowed here, it is the only
placement that needs no copy at all: the core stays in the displayed frame's state for the whole
of (c) and (d), and `restore_real_frame` runs once they are over. Three sites, one per path the
core takes:

| Path | Restore site |
|---|---|
| single thread (`run_frame_timed`) | after `finish_frame_lent` (capture, conversion, hooks) |
| separate core, serial (`complete_frame_timed`) | after `finish_frame_lent`, same |
| separate core, emulation ahead (`capture_in_flight`) | after the batch, the RAM snapshot and the 2D picture are taken — the three things that path reads from the displayed frame — and before the next frame is launched |

### Design decisions of the lot, and what each one costs

**1. The in-memory state is written by the savestate's own per-component code** (`save_state` /
`load_state` of the bus, CPU, PPU, APU, input ports, cartridge, widescreen), into a reused
`Vec<u8>` held by the core. It is *not* the portable savestate: no magic number, no format
version, no core name, no ROM image, no ROM name, and the restore does not rebuild the cartridge
from the ROM image — it reads into the live objects. Reusing the per-component code is what keeps
the two from drifting apart, which is the trap the .NET lot's own comment names.

A `Clone` of the core was rejected: `NesCore` owns the `Cartridge`, which owns the PRG-ROM and
CHR-ROM images (128 KiB to 1 MiB), so a clone per real frame would copy megabytes of data that
cannot change. A hand-written per-component copy was rejected for the drift.

**2. The frame's output is left out of the state, which is both cheaper and what REQ-LOOP-067
asks for.** `Ppu2C02::save_state` is split in two in place — `save_machine_state` (the memories,
the registers, the rendering pipeline) and `save_frame_records` (`mask_x`, `mask_y` and the tile
records). `save_state` calls both in the same order, so **the savestate format is byte-identical
and every field still has exactly one definition**; the in-memory state calls the machine half
only. Consequences:

- the ~1 000 tile records of a frame (26 bytes each) are not serialised twice per real frame;
- after the restore the core still holds the **displayed** frame's records, masks, margins and
  pixels, so `frame_batch()`, `wide_picture_into()` (the 2D picture) and the Zapper's colour all
  answer for the displayed frame with nothing copied anywhere;
- it is sound because `Ppu2C02::begin_rendered_frame` clears the records unconditionally at dot 1
  of the pre-render line of **every** frame, rendering on or off (`ppu/mod.rs`), and a frame runs
  from vblank to vblank — so the clear always precedes the first capture of the next real frame.

The PPU's pixel buffer was already outside the savestate before this lot, so the picture of the
displayed frame survives the restore for free (the .NET lot records the same property).

**3. No sound from a frame that is not displayed** is one early return in `SampleRing::enqueue`,
behind the mutex the enqueue already takes. That runs per **output sample** — about 735 per
emulated frame at 44.1 kHz — not per CPU cycle, so it is about 0.003 % of a frame's instructions.
The alternative, a flag captured by the `|sample| lock(ring).enqueue(sample)` closure of
`step_cpu_cycle`, was rejected: that closure *is* built per CPU cycle and widening it risks the
register pressure of the hottest loop in the APU (the lesson of RU-3a-fix).

**4. The pixels of a hidden frame are rendered** — see step 4 for the measurement. REQ-LOOP-068
only *permits* skipping them ("PEUVENT omettre"), and in Rust the saving is small while the only
zero-cost way to get it (a `const` generic over the two span loops) doubles the hottest PPU loops
in the instruction cache, which is the wrong trade on a Cortex-A53.

### Test environment

The notes of RU-1, RU-2 and RU-3a still apply: CMake must be on `PATH` and
`--features zax-app/bundled` is needed for every `zax-app` or workspace command (SDL2 is linked
statically), the two demo binaries are local stand-ins, and `rust/test-roms/` is a copy of the
291-ROM corpus of `tests/roms/` (git-ignored, nothing of it committed).

The untracked `tests/` folder of this checkout (343 MB) is the .NET solution's build output left
by the former branch layout. It is not touched by this lot.

### Step 2 — REQ-CORE-121, the in-memory state

`zax_nes::state::CoreState` is a reused `Vec<u8>`, with `len`, `is_empty`, `as_slice` and a
`with_capacity` whose documented argument is `NesCore::max_core_state_size()` — the value that
makes it grow exactly never. `NesCore::capture_core_state` / `restore_core_state` are the pair;
both refuse a core with no ROM, and the restore also refuses a buffer nothing was captured into
and, unlike a corrupt savestate, **leaves the machine running** (nothing was written yet when the
header checks fail).

What is in the block, in order: the frame counter, the region, then `NesBus`, `Cpu6502`,
`Ppu2C02::save_machine_state`, `Apu2A03`, `InputPorts`, `Cartridge` and `WideExtension` — the
savestate's own per-component code, called in the savestate's own order. What is not in it:

| Left out | Why it is sound |
|---|---|
| the magic number, the format version, the core name | the block is read back by the instance that wrote it, in the same session |
| the ROM image and the ROM name | the ROM never leaves; the restore reads into the **attached** cartridge instead of rebuilding one, and `Cartridge::load_state` refreshes the fast ROM windows itself |
| the PPU's pixel buffer | it was never in the savestate either, so the displayed frame's picture survives for free |
| the PPU's tile records and edge bands (`save_frame_records`) | `begin_rendered_frame` clears them at dot 1 of the pre-render line of **every** frame, rendering on or off, and a frame runs vblank to vblank — so the clear always precedes the next frame's first capture |

Two departures from the .NET lot, both deliberate:
1. **The widescreen block is always written**, where a savestate writes it only once a ROM has
   unlocked the extension. Within one real frame a game may unlock it, so the in-memory state has
   to be able to put "locked" back; writing the block unconditionally does that exactly (the
   block carries `unlocked` itself) and needs no `reset()` branch.
2. **The tile records are out** (above). The .NET keeps them in, and then has to say that the
   picture survives anyway; here both the picture *and* the records survive, which is what makes
   REQ-LOOP-067 true by construction in step 5.

`Apu2A03::load_state_keeping_samples` is the restore that leaves the queue alone: one private
`load_state_inner(reader, keep_pending_samples)` with two public wrappers, so the two paths
cannot read a different layout. REQ-CORE-074 and Q-006 are unchanged for savestates and
rollbacks — a test asserts both sides.

#### What it costs

`the_cost_of_one_capture_and_one_restore`, release, x86, 20 000 rounds each, reported by the
`#[ignore]` measurement in `core_state_tests.rs`:

| Fixture | state size | capture | restore | both, per real frame |
|---|---|---|---|---|
| minimal (NROM, CHR-RAM) | 19 474 B | 0.249 µs | 0.646 µs | **0.895 µs** |
| `mmc3_test_2/1-clocking.nes` (MMC3) | 19 553 B | 0.255 µs | 0.713 µs | **0.967 µs** |
| `spritecans.nes` (a drawn frame) | 19 474 B | 0.255 µs | 0.655 µs | **0.911 µs** |

About **1 µs against a core frame of 850 µs: 0.11 %** of one emulated frame, where N = 1 doubles
the frame. Keeping the ~1 000 tile records of a frame out of the block is most of why: the state
is 19.5 KiB instead of about 46 KiB, and it is written and read with no per-record loop.

Tests: `zax-nes/tests/core_state_tests.rs`, 11 of them plus the measurement — the acceptance
criterion of REQ-CORE-121 (100 frames, a varying input sequence, the picture, the tile records
and the work RAM digested per frame) on the counter ROM **and** on `spritecans.nes` so that the
PPU pipeline, the mapper and the APU are all in the comparison; a restored state that captures to
the same bytes; the frame counter; the three memories; an MMC3 bank window; the audio queue kept
where a savestate drops it; the frame the host is reading surviving the restore; the bound and
the buffer that stops growing over 60 captures; the amended REQ-ARCH-012 ("the sequence of real
states does not depend on N") over 120 frames; and the two refusals.

`cargo test --release -p zax-nes` 494 passed, and the full goldens report
**`291 of 291 rows matched, 0 ROMs missing`** — the savestate format is byte-identical, which is
what the `save_state` = `save_machine_state` + `save_frame_records` split was for.

SPEC copy: REQ-CORE-121 added to SPEC-02 §3.12, after REQ-CORE-122.

### Step 3 — REQ-LOOP-065, the frame sequence

`NesCore::run_frame_ahead(word, n)` and `NesCore::restore_real_frame()` are the pair, with
`NesCore::MAX_RUN_AHEAD` = 3 and `holds_a_real_frame()` for a host that wants to know. Two new
fields on the core: `real_frame: CoreState` (a plain field, not a `Box` — `CoreState::default()`
allocates nothing, so a core that never runs ahead carries three null words) and
`real_frame_held: bool`.

**Why the sequence is in the core and not in the session.** "Emulation on a separate core" *moves*
the core to the `zaxnes-emulation` thread for the duration of one frame (`core_worker.rs`), so a
sequence driven from `GameSession` could not reach it. Putting it in the core makes run-ahead and
emulation ahead orthogonal: the thread runs one real frame of the sequence exactly as it ran one
ordinary frame.

**The N = 0 path.** `run_frame_ahead` is `if ahead == 0 { return self.run_frame(word); }` and the
sequence is `#[inline(never)]` behind it, so nothing of run-ahead reaches `run_frame`. The host
side costs one `bool` test per real frame (`restore_real_frame` returning false at once).

**Step 5 is deferred**, which REQ-LOOP-067 spells out as allowed. `run_frame_ahead` leaves the
core in the **displayed** frame's state; the host calls `restore_real_frame` once it has finished
reading that frame. A host that forgets is made good: the next sequence restores first, so the
real frames stay contiguous whatever the host does (a test holds that).

**What the hidden frames do not do.** The video callback is taken out for steps 1 and 3 and put
back for step 4, so REQ-CORE-020's "once per emulated frame" becomes "once per **real** frame"
exactly as REQ-LOOP-067 asks — and the batch the callback carries is the displayed frame's.
`Apu2A03::set_output_enabled(false)` silences them; the switch is owned by the sequence and put
back before step 4, so a host that never asks for run-ahead never sees it move.

**Lifecycle (REQ-LOOP-066).** `power_on`, `soft_reset`, `close_rom` and the savestate restore all
call `forget_real_frame`: the state of a machine that is gone is dropped, never put back, and the
next real frame starts a sequence of its own with the same N. `close_rom` also puts the audio
switch back on.

#### The measurement that gates the lot: N = 0 costs nothing

Wall-clock `bench-core` cannot resolve 1 %, so the instrument is RU-3a-fix's: the **post-LTO
assembly** of the hot functions (`cargo rustc -p zax-harness --release --bin zax-harness --
--emit=asm`), compared against a throwaway worktree of `ce78abd` built with `CARGO_TARGET_DIR`
outside the repository.

| function | RU-3a-fix `ce78abd` | after step 3 |
|---|---|---|
| `NesBus::tick` | 119 instr / 3 `pushq` / 32 B frame | **119 / 3 / 32 — identical** |
| `NesCore::run_frame` | 2 096 instr / 8 `pushq` / 904 B | **2 096 / 8 / 904 — identical** |
| `Apu2A03::run_cpu_cycles` | 855 instr / 8 `pushq` / 184 B | 873 / 8 / 184 (**+18**) |

The +18 are three inlined copies of the mute test, and the diff of the two function bodies shows
**where** each one sits: right after the `lock cmpxchgb` that takes the ring's mutex, i.e. inside
the enqueue, which runs once per **output sample** and not once per CPU cycle.

    movq  GLOBAL_PANIC_COUNT(%rip), %rax     ; the lock is already taken here
    ...
    cmpb  $0, 72(%r14)                       ; <- SampleRing::muted
    jne   .LBB1318_30

So the dynamic cost at N = 0 is about 735 samples × 3 instructions = **~2 200 instructions per
emulated frame** against a core of roughly 27 M: **0.008 %**, well inside R0's ±0.1 % gate. The
per-cycle path of `tick` and the body of `run_frame` are byte-identical.

Interleaved wall clock, the two binaries alternated run by run so that machine drift cancels
(`bench-core`, `nestest`, 1200 frames after 120 of warm-up, core `run_frame`):

| round | `ce78abd` | after step 3 |
|---|---|---|
| 1 | 0.8406 ms | 0.8515 ms |
| 2 | 0.8586 ms | 0.8516 ms |
| 3 | 0.8586 ms | 0.8653 ms |
| 4 | 0.8482 ms | 0.8510 ms |

The baseline's own four readings span 0.841 to 0.859 (2.1 %), which is why the assembly above is
the evidence and this table is only a sanity check.

#### The test vector ROM

`zax-nes/tests/common/run_ahead_rom.rs`, built in source from the amendment's text: NROM, the CHR
of SPEC-02 §5.1 (tile *n* = eight bytes of *n* and eight of 255 − *n*), an NMI handler that prints
the counter it had **on entry** as the tile index of the top-left cell and the A bit it read as
the second cell, then increments. `displayed()` reads the eight pixels of the top-left tile back
out of the picture; `shows_button_a()` reads the second cell's last pixel. The module is shared
with the `zax-app` tests by `#[path]`, as `ui_harness/bundle_support.rs` already shares the
`zax-profiles` fixtures, so there is one definition of the vector.

One departure from the .NET vector, found by a failing test and worth recording: the .NET handler
resets the scroll with `$2006 = $20, $00`, which leaves **fine Y = 2** (bits 14-12 of `t`), so the
picture is two rows up and the top-left cell is not captured as a tile record at all. The handler
here writes `$2006 = $00, $00` (`v` = `t` = `$0000`: nametable 0, no scroll), which is what "scroll
back to the top left" was meant to do. `displayed()` worked either way — the pixels were right —
but the batch did not carry the cell.

Tests: `zax-nes/tests/run_ahead_tests.rs`, 12 of them. The acceptance criterion of REQ-LOOP-065
word for word (N = 2, counter k, A pressed, picture k + 2 with A, counter k + 1 after step 5); the
picture at every N, including N = 0 where the displayed frame **is** the real one; N = 0 identical
to `run_frame` over 30 frames (counter, RAM and pixels); one core frame counted per real frame at
every N; only the displayed frame heard (four emulated frames queue one frame's samples, ±12 %)
and the restore keeping them; the video callback firing once per real frame and carrying the
displayed frame's records; the RAM read before the restore being the displayed frame's; the real
frames being those of a run without run-ahead over 60 frames at N = 1, 2 and 3; a forgotten
restore made good; the four lifecycle events dropping the held frame; a caller's out-of-range N
clamped to 3; and a stopped core running nothing.

`cargo test --release -p zax-nes` **506 passed**, goldens **`291 of 291 rows matched`** — the mute
flag is off by default, so no audio digest moved.

SPEC copies: SPEC-07 §3.7 — REQ-LOOP-060 replaced by its amended form and REQ-LOOP-064 to 068
added after it. SPEC-01 — REQ-ARCH-012 amended (N is part of the determinism contract) and
REQ-ARCH-013 precised (the in-memory state has its own bound). SPEC-12 — REQ-ACC-030 precised
(the measure is taken at Run-ahead = 1).

### Step 4 — REQ-LOOP-068, the frames that are not displayed

The "no sound" half went in with step 3 (the sequence owns the switch). This step is the pixels,
and the answer is **the pixels of every emulated frame are rendered** — the optimisation
REQ-LOOP-068 permits is declined, measured rather than guessed, as the lot's rule 3 allows
("otherwise render the pixels and say so").

**What it would buy.** Upper bound, measured by building a throwaway binary from the `ce78abd`
worktree with the palette read, the greyscale mask and the pixel store removed from
`span_pixel`, `render_pixel` and the two span loops — everything the emulation reads back kept
(`bench-core`, 1200 frames after 120 of warm-up, core `run_frame`, interleaved run by run):

| round | `nestest`, pixels | `nestest`, none | `spritecans`, pixels | `spritecans`, none |
|---|---|---|---|---|
| 1 | 0.8477 ms | 0.8298 ms | 0.8541 ms | 0.8451 ms |
| 2 | 0.8622 ms | 0.7933 ms | 0.8636 ms | 0.8123 ms |
| 3 | 0.8421 ms | 0.8056 ms | 0.8623 ms | 0.8224 ms |

So about **4.8 %** of one emulated frame on `nestest` and **3.9 %** on `spritecans` — which at the
default N = 1 is 2.4 % of a real frame, and at N = 3 about 3.6 %.

**Why it is not taken.** The only way to get it at zero cost when unused is a `const` generic over
`run_visible_span`, `run_visible_span_through_bus`, `span_pixel`, `render_pixel` and the backdrop
fill — the five hottest pieces of the PPU, monomorphised twice. Against that:
1. It is zero *instruction* cost at N = 0 but not zero *layout* cost: doubling those loops moves
   inlining decisions and the instruction cache, and R0's gate is ±0.1 % on a profile that uses
   nothing new. The rule that N = 0 costs nothing outranks a 2.4 % gain at N = 1.
2. Handhelds default to N = 0, so a Cortex-A53 (32 KiB of instruction cache) would pay the
   duplication and get none of the benefit.
3. The desktop, where N = 1 is the default, is not where the frame budget is tight
   (`PERFORMANCE-HISTORY.md` §4).

**Nothing of accuracy is given up either way**, and in Zaxnes the Zapper is unaffected by the
choice: `Ppu2C02::sees_light_at` composes the colour from the **tile records** of the frame being
drawn (REQ-CORE-062, AMD-021 R2-006), not from the pixel buffer, and every emulated frame emits
its records unconditionally. So REQ-LOOP-068's "each emulated frame detects the light on its own
pixels" holds by construction, with no `has_zapper` test anywhere — one branch the .NET
implementation needs and this one does not.

The normative half of REQ-LOOP-065 is still met: no hidden frame's image ever reaches the
display, because the host reads the picture only after step 4 and the displayed frame has
overwritten every visible dot by then.

Tests, two more in `run_ahead_tests.rs` (14 in all):
- REQ-LOOP-068's acceptance criterion: port 2 is a Zapper, the aim alternates every 7 real frames
  between a lit target (the top-left cell, whose every pixel is sub-palette colour 1 or 2 — `$21`
  and `$11`, both light) and a dark one (a pattern-0 cell, where the `$0F` backdrop shows), and the
  ROM's per-frame light log over 40 real frames is **byte-identical at N = 1, 2 and 3 to N = 0**.
  The log is asserted to hold both values first, so the comparison cannot pass vacuously.
- The mechanism: a hidden real frame's own log entry records the light its own frame showed.

One fault in the .NET vector, found by that non-vacuity assertion and fixed here: bit 3 of a
Zapper port read is **0 when light is seen** (REQ-CORE-061, active-low as on the hardware), so
`LDA $4017 / AND #$08 / ORA $14` accumulates "no light", which reads `$08` on every frame whatever
the aim — the log is a constant and the comparison proves nothing. The main loop here inverts the
bit (`EOR #$08`) before accumulating.

`cargo test --release -p zax-nes` **508 passed**.
