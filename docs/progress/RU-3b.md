# RU-3b — run-ahead (AMD-041)

Branch `rustupdated`, on top of RU-3a-fix (`ce78abd`). The application is the Rust workspace
`rust/`; the .NET lot **V2-3** (`b599e6c`, AMD-041) of `dotnetupdated` is read as a reference
only — it has no `docs/progress/V2-*.md` file (those start at V2-21), so its diff is the whole
reference.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, design decisions, before benchmarks, spec copies | done — `0d312e2` |
| 2 | REQ-CORE-121: the in-memory state of the core, and its tests | done |
| 3 | REQ-LOOP-065: the frame sequence in the core, N = 0 untouched | todo |
| 4 | REQ-LOOP-068: the frames that are not displayed (no sound, the pixels measured) | todo |
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
