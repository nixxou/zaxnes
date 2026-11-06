# Performance work since the C# port — complete record

This document lists every performance change made to the Rust version of Zaxnes since it was
ported from the C# version 0.2.0 (commit `370216c`), the attempts that were dropped, and what was
learned. It exists for one job: when the C# version is updated for 3dSen 2.0 and that update is
carried over to Rust, nothing listed here may be lost or silently undone.

Read it with:
- `HANDHELD-PERFORMANCE.md`: the chronological log with every device measurement.
- `PROGRESS.md`, section "Rust-only changes": the behaviour changes that are not about speed.
- `ROADMAP.md`, `HANDHELD-RESEARCH.md`: plans and the platform survey.

Commits are on the branch `retroachievements-dev` (worktree `wt-exec-zone`), all local unless
stated otherwise.

## Contents

1. How to use this document during the 3dSen 2.0 update
2. Rules that every change below follows
3. Verification tools
4. Target hardware and what dominates where
5. Changes by subsystem
   - 5.1 NES core (`zax-nes`)
   - 5.2 3D conversion engine (`zax-engine3d`)
   - 5.3 Lua scripting (`zax-scripting`)
   - 5.4 Renderer (`zax-render`)
   - 5.5 Application loop, threads and pacing (`zax-app`)
   - 5.6 Build and packaging
   - 5.7 Instrumentation
6. Settings and environment variables
7. Attempts that were dropped, and why
8. Conclusions
9. Open problems
10. Porting checklist: C# areas and the Rust work they touch

---

## 1. How to use this document during the 3dSen 2.0 update

The Rust crates mirror the C# projects one to one:

| C# 0.2.0 project | Rust crate |
|---|---|
| `src/Zax.Nes` (Cpu, Ppu, Apu, Cartridges, Input, State) | `crates/zax-nes` |
| `src/Zax.Engine3D` | `crates/zax-engine3d` |
| `src/Zax.Scripting` | `crates/zax-scripting` |
| `src/Zax.Render` (Gl, Assets) | `crates/zax-render` |
| `src/Zax.App` (Loop, Ui, Persistence, Input, Audio) | `crates/zax-app` |
| `src/Zax.Platform` | `crates/zax-platform` |
| `src/Zax.Settings` | `crates/zax-settings` |
| `src/Zax.Profiles` | `crates/zax-profiles` |
| (none) | `crates/zax-harness` (tests, benches), `crates/zax-achievements` |

For each C# change of the update:
1. Find the subsystem in section 10 and read the Rust items it names.
2. Port the change so that the invariants listed for those items still hold.
3. Run the checks of section 3 that the subsystem calls for. An optimisation listed as
   "identical output" must still give identical output after the port.

None of the optimisations below changed the C# specification: they are internal to Rust, or
they are settings that are off on the desktop (the C# behaviour) and on by default on handhelds.

---

## 2. Rules that every change below follows

- **Identical output needs no switch.** A change that gives the same pixels, tiles, cycles,
  audio and script behaviour goes in unconditionally, but must pass the goldens.
- **Changed output is a setting.** Anything that can change what the player sees or what a script
  observes is a setting: off (C# behaviour) on the desktop, on by default on handhelds where it
  helps. Owner decision; it also keeps the desktop build a faithful reference.
- **Measure on the target.** x86 wall time misleads for the Cortex-A53 (section 4). Use
  instruction counts (callgrind) or the device profiler.
- **No catch-up set to 0 by default**, and no fidelity/latency trade without the owner's approval.

---

## 3. Verification tools

| Tool | What it checks | When |
|---|---|---|
| `cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored` | 291 ROMs x 1200 frames: pixels, tile records, audio, cycles, blargg results | any core change |
| `zax-harness golden-core <roms dir> <out.jsonl> <frames>` | same digests on any ROM folder; run at two commits and compare | any core change; the reference run used 98 commercial ROMs x 1800 frames |
| `zax-harness render --rom ... --out <dir>` with `--group-draws`, `--no-shadows`, `--size` | PNG captures; compare two builds byte for byte | any renderer or engine change |
| `zax-harness render ... --bench <n>` | GPU/CPU time of the scene passes, "GL draws" count | renderer work |
| `zax-harness bench-core <rom> [--profile] [--load-state] [--widescreen] [--run-ahead]` | CPU cost per emulated frame of each stage (core, run-ahead restore, capture, conversion, Lua, publish, draw list, picture), max and frames over 1 ms. **`--run-ahead` defaults to 0**, so a figure taken before AMD-041 and one taken after it measure the same work | CPU work; also runs under `qemu-aarch64` for relative weights |
| `ZAXNES_LUA_GC=smooth` with `bench-core` | measures Lua with the collector tuning the application uses | always, for Lua numbers (without it the spikes of the original tuning show) |
| `cargo test -p zax-scripting --release --test scripting perf -- --ignored --nocapture --test-threads=1` | Lua binding micro-benchmarks | binding work |
| Lua API dump (`goldens/lua-api.txt`) | the script-visible API is unchanged | binding work |
| In-app profiler (`ui.profiler` or `ZAXNES_PROFILE=1`) | per-stage mean/max per displayed frame, busy time per thread, Lua time per script hook, in `logs/zaxnes.log` | device runs |
| `valgrind --tool=callgrind` on a short `bench-core` | instruction counts, closer to an in-order A53 than x86 time | core and engine work |

**callgrind on the Windows host.** Lots MF-1 to MF-4a all recorded "no callgrind on this machine"
and fell back to reading generated assembly. There is one: the WSL2 Ubuntu it carries, whose
`rustc` is the *same* version with the same LLVM, so the Linux binary's hot code is the Windows
binary's. Lot MF-4a-bis used it to localise a regression to the instruction:

    # from a `git archive` of each compared commit, into its own fresh directory
    CARGO_PROFILE_RELEASE_STRIP=false CARGO_TARGET_DIR=<fresh> \
        cargo build --release -p zax-harness --bin zax-harness
    valgrind --tool=callgrind --dump-instr=no --cache-sim=no --branch-sim=no \
        ./zax-harness bench-core <rom> --frames 300 --warmup 0
    callgrind_annotate --threshold=100 callgrind.out.<pid>

`STRIP=false` is the whole trick — the shipped profile strips symbols, and without them callgrind
has nothing to name. Nothing else of the profile changes, so the codegen is the shipped one. The
counts are deterministic to the instruction: the same binary twice gives the same number, which is
what makes a one-instruction-per-cycle change measurable where wall clock has three times that as
noise. Build each commit in its **own** target directory (section 2).

**The procedure as a script: `rust/tools/perf/`** (lot PR-1). `make-tree.sh <ref|--worktree>` packs
a clean tree — on the host, because a git worktree's `.git` is a host path that WSL cannot follow;
`build-clean.sh <label> <tar>` builds it in a brand-new directory and deletes the 4 GB target as
soon as the binary is out; `callgrind-set.sh <label>` runs the whole ROM set of `roms.txt`;
`callgrind-diff.py` prints the per-ROM table, the symbols that moved and the verdict;
`callgrind-symbols.py` tabulates one run. The runs go in **parallel** — callgrind is a simulator,
so a count does not depend on machine load — which is 43 s of wall clock for thirteen ROMs instead
of nine minutes, and that is what makes measuring every variant of a change on a whole set
affordable. Two floors worth knowing: the whole-process figure carries about **10⁵ instructions**
(0.0001 %) of host noise from glibc's time handling and Lua's string table, measured by building
two trees that differ only in comments; the per-symbol counts of `zax_nes` have **no** floor, and
were identical to the instruction across that pair.

An app save state (`.ssN`) holds the core state block in its second section; extracting it gives a
`--load-state` file for the harness, which reproduces a device scene exactly (used for the Zelda II
world map, section 9).

---

## 4. Target hardware and what dominates where

Anbernic RG40XXV: Allwinner H700, 4x Cortex-A53 at 1.512 GHz (in-order, small caches), Mali-G31
(one core), 1 GB RAM, 720x480 (owner's unit), GLES 3.0 context on a GLES 3.2 driver.

Ratios observed between the device and an x86 desktop for the same work:

| Stage | Device / x86 |
|---|---|
| NES core | about 13x |
| Lua | about 9-10x |
| Conversion | about 14x |
| Draw list build (Zelda II world map, 2258 tiles) | about 60x (unexplained, section 9) |

Where the time went, in order of discovery:
1. The core alone did not fit in 16.6 ms (17.4 ms per NES frame), and the catch-up (up to 5
   emulated frames per displayed frame) turned every late frame into a collapse to 5-10 fps.
2. Once the core was faster and on its own thread, the scene's draw calls (one per tile, about
   17 us of CPU each on Mali) dominated.
3. After batching, the main thread (conversion + Lua + draw list + collection + GL submission)
   set the pace; Lua had spikes of 100-300 ms from the garbage collector.
4. Today: heavy scenes are CPU-bound on the main thread (Zelda II world map: draw list 12.6,
   GL submission 9.9, collection 7.1, Lua 5.2 ms per displayed frame), not GPU-bound.

---

## 5. Changes by subsystem

Each entry: what it does, where it lives, whether the output is identical, the switch if any, the
measured gain, and the invariant a port must keep.

### 5.1 NES core (`zax-nes`)

C# counterpart: `src/Zax.Nes/Ppu`, `Apu`, `Cartridges`, `Cpu` (the bus).

**PPU dot dispatch by range, sprite layer built once per line** (`537c0b2`)
- The per-dot `match` became a dispatch by dot range; the sprites of a line are evaluated into a
  per-line buffer once instead of per pixel.
- Identical output. About 3 % on the device (x86 gained more).

**PPU caught up in batches** (`91123ec`)
- The PPU no longer steps with every CPU cycle. It lags and is caught up at sync points: CPU access
  to $2000-$3FFF, OAM DMA ($4014), controller reads ($4016/$4017), any cartridge write,
  vblank/NMI edges, and an IRQ horizon given by the board (MMC3 A12 counter). Idle dots are
  skipped.
- Rambo1 and MMC5 still step every cycle (no IRQ horizon written for them).
- Identical output. x86 core 1.4-2x faster; device only about 11 % (17.4 -> 15.6 ms per frame).
- Invariant: any new register, mapper write path or IRQ source must be a sync point or provide a
  horizon, or the PPU will be observed late.

**PPU fast span on visible lines** (`a0c157a`, extended by `0541e02`)
- On a visible line with rendering on, dots 2 to 255 run in one loop: PPUMASK and fine-X decisions
  hoisted, and when the board's pattern fetches have no side effect (`Mapper::chr_view`, a
  `ChrView` over CHR banks) the fetches read CHR and console VRAM directly
  (`Ppu2C02::run_visible_span`).
- Boards that watch the PPU bus or latch on fetches (MMC2/MMC4, MMC3 and compatibles, Rambo1, MMC5)
  get `run_visible_span_through_bus`: one loop, but each fetch through the full bus path, so the
  board sees the same addresses at the same PPU clock.
- Identical output. FF1 x86 core 1.18 -> 0.87 ms/frame; SMB3 1.36 -> 1.24 ms/frame; device FF1
  15.6 -> 12 ms.
- Invariant: a board returns a `chr_view` only if its fetches are pure reads. A new mapper that
  latches on CHR reads must return `None` (the default) or the through-bus path.

**APU level mixed only when an input may have changed** (`226da76`)
- The mixer output used to be recomputed every CPU cycle. It is now recomputed only after a
  channel timer moved its sequencer, shift register or DMC bit, a frame-counter step, a register
  write, a reset, a loaded state or a region change.
- Identical output, audio included. SMB3 x86 core 1.24 -> 1.13 ms/frame.
- **Kept by RU-3a** (AMD-054, non-linear mixing): the two table lookups of the new law went into
  this same lazy path, where they replace four multiply-adds, an addition chain and a clamp. The
  mixing therefore costs *less* than it did; `nestest` did not move.
- **RU-3a-fix**: RU-3a had added the expansion level to the cache *test*
  (`key == level_key && expansion_level == level_expansion`). That is **not** a cold path — it runs
  about 15 000 times per emulated frame — and the second load, compare and branch measured +13.9 M
  instructions per 300 frames (+1.1 % of the APU) on a board with no expansion audio. `M` is folded
  into the one key instead: `set_expansion_level` stores `NO_LEVEL_KEY` when the value moves, which
  happens only for the MMC5 and only on a real change. The hot test is one comparison again.
- Invariant: **the cache test stays a single comparison of `level_key`.** Anything that can change
  `A`, `M` or the five channel outputs must either be packed into the key or invalidate it
  (`level_key = NO_LEVEL_KEY`) *and* set `level_dirty`; never add a second comparison here.
- Invariant: the mixing stays in `current_level`, never per CPU cycle, and `current_level` keeps
  **one** call site — the per-cycle step. A second one (a cold savestate rebuild was enough) makes
  the inliner leave the function out of line, which costs a call and a prologue on all ~15 000 of
  those cycles; `#[inline]` does not undo it. `Apu2A03::load_state` writes its mix out for that
  reason.

**APU quiet cycles skipped while the frame loop batches** (`0cf347d`)
- During the frame loop the bus lets the APU lag like the PPU and catches it up on $4000-$401F
  accesses (except $4014 and $4016), on the DMC fetch's byte, and at the end of the loop. Runs of
  cycles on which no timer, frame-counter step or level change falls are skipped in one go: the
  timers are counted down and the resampler advanced by the same cycle count
  (`apu/mod.rs: quiet_cycles`, `skip_quiet_cycles`; channels' `quiet_clocks` /
  `skip_quiet_clocks`; resampler `level()` / `skip_cycles`; bus `apu_lag`, `catch_up_apu`).
- Outside the frame loop the APU still runs cycle by cycle.
- Identical output (98 ROMs x 1800 frames, audio digests included). Core -5.5 to -8.5 %.
- Pitfall found: `pending_sequencer_reset` must be decremented in the skip too (audio differed on
  18 ROMs until it was).
- Invariant: any APU state that changes with time must report itself in `quiet_cycles` and be
  advanced in the skip.
- **RU-3a** (AMD-054, MMC5 audio): a cartridge with expansion audio keeps `apu_quiet = 0`, so the
  APU runs cycle by cycle on it and the `M` the bus pushes is never stale. **Only the MMC5 is
  concerned.** `tick` does now step the cartridge *before* the APU catch-up instead of after (a
  move, not a branch); the 291-ROM goldens are identical across that reorder.
- **Measured, and wrong: `catch_up_apu` does not run "rarely".** RU-3a assumed it did and put the
  `expansion_audio` test and the `M` push inside it. It runs on **14 894 of the 29 780 cycles of a
  frame** (nestest, counted) — one cycle in two, because a pulse timer is clocked on alternate
  cycles — so it is a per-cycle path, and one instruction in it is ~15 000 per frame against a
  ±0.1 % gate of ~27 000. RU-3a cost `NesBus::tick` two extra callee-saved registers and +15.7 %
  of its instructions, on every board.
- **RU-3a-fix**, how it is free now. Three things, all of them structure rather than less work:
  - `NesBus::apu_batching` = `deferred_ppu && !expansion_audio`, one field maintained in `attach`,
    `defer_ppu` and `end_deferred_ppu`. `catch_up_apu` tests it **instead of** `deferred_ppu`, not
    on top of it, so the fast path costs exactly what it cost before AMD-054 (12 instructions
    either way, read off the post-LTO assembly).
  - `!apu_batching` — a board with expansion audio, or any board outside the frame loop — falls
    through to `sync_apu`, which turned out to be the same function; there is one, not two.
  - `NesBus::push_expansion_audio` is `#[inline(never)]`. That is the one thing that must stay out
    of line: `sync_apu` is inlined into `tick` twice over (through `catch_up_apu` and through
    `perform_dmc_dma`), and the push's cartridge `Option`, devirtualised `expansion_audio_output`
    and `set_expansion_level` are what raise the register pressure of the whole function.
- Invariant: **nothing MMC5-specific may be inlined into `NesBus::tick`.** Check it by reading
  `tick` in the post-LTO assembly (`cargo rustc -p zax-harness --release --bin zax-harness --
  --emit=asm`, then the prologue between the label and `.seh_endproc`): three `pushq` and a 32-byte
  frame is right, four is the regression. Out-lining `sync_apu` as well was measured and is
  *worse*: a board with expansion audio runs it every cycle, so it pays a second call for nothing
  (`mmc5test` 1.87 -> 1.91 ms/frame).
- Measured, interleaved with the RU-2 and RU-3a binaries: `nestest` 0.87 ms/frame, within the
  machine's spread of both. `mmc5test` 1.83 (RU-2) -> 1.87, `exram` 2.09 -> 2.09: the MMC5 pays one
  out-of-line call per cycle, about +2 %, which is the price of every other board paying nothing.
  No built-in profile targets an MMC5-audio game.

**Plain PRG-ROM windows read without the board's decode** (`6a53e71`)
- The cartridge keeps, for each 8 KiB window of $8000-$FFFF, where it starts in PRG-ROM when a
  read of it is a plain ROM read (`Mapper::rom_windows`, `NO_ROM_WINDOW`,
  `BankWindows::rom_windows`, overrides for NROM and MMC5). Refreshed after every CPU write to the
  cartridge, after reset, an intercept and a loaded state. The bus `read_internal` tries this
  first for addresses at or above $8000.
- PRG-RAM windows, windows that would wrap, and MMC5 reads still go through the board.
- Debug builds check every fast read against the board's own (`debug_assert`).
- Identical output. Core -6 to -8.5 %.
- Invariant: a board whose ROM reads have side effects or depend on more than the bank registers
  must report `NO_ROM_WINDOW`.

**Rollback snapshots recycle their oldest buffer** (`13f8a08`)
- `RollbackHistory::capture_with`, `NesCore::save_state_to`: no allocation per snapshot, same
  bytes.

**The DMA's stolen cycles cost what they cost before MF-4a** (lot MF-4a-ter)
- MF-4a replaced the OAM transfer inside the `$4014` write, and the DMC fetch inlined into
  `NesBus::tick`, with one cycle-exact state machine (`run_dma`). It is right and it was dear:
  **61.4 instructions per stolen OAM cycle against the old loop's 23.8**, and 181 per DMC fetch
  against about 45.
- `NesBus::copy_oam_pairs` runs the byte pairs of a transfer no DMC fetch can interrupt as a tight
  get/put loop, `oam_fast_pairs` saying how many pairs are provably safe from
  `DmcChannel::cycles_until_fetch`; `dmc_fetch_cycles` (reached through `dmc_only_dma`, straight
  from `halted_read_cycle`) runs a lone fetch's three or four cycles straight. Anything else — a
  fetch inside a transfer, a halted `$4016` read, a source page of `$40` — is the state machine
  untouched.
- **23.6 instructions per stolen OAM cycle, 71 per DMC fetch.** `run_dma` + `dma_read` +
  `write_internal` against `b39eaba4`: `spritecans` (295 transfers, no DPCM) **−27 208**,
  `M0_P32K_C8K_V` −254, `M1_P128K` +1 537, `dpcmletterbox` (297 transfers and 19 051 fetches)
  +1 446 877. `tick`, `read_cycle`, `read_internal`, `write_internal`, `Cpu6502::step`/`branch`/
  `resolve`, `Apu2A03::run_cpu_cycles`/`quiet_cycles` are byte-identical post-LTO and read the same
  callgrind count to the instruction on all five ROMs.
- Invariant: the fast loop may run a cycle only if `promote_dmc_wait` would have done nothing on
  it. `oam_fast_pairs` returns `free / 2` and not `free`, so a batch's last cycle is always a put
  cycle; one cycle more would let that put consume the DMC's halt cycle and move the fetch.
  `req_core_113_the_fast_oam_loop_is_the_state_machine_cycle_for_cycle` asserts 128 rows of
  (stolen cycles, fetches inside, OAM hash) produced by the state machine alone at `dadb94c5`.
- **`spritecans-2011/spritecans.nes`** is the ROM to measure OAM DMA on: one full transfer a frame
  and no DPCM, which MF-4a-bis had to record as missing. `dpcmletterbox` is the same with DPCM
  played continuously. Counts obtained from `NesBus::oam_dma_count`/`dmc_dma_count` via a temporary
  print in `bench-core`, not by inference.

**The MMC5 listens to the PPU's reads, and only the MMC5 pays** (lot MF-4c)

- MR-5-4: the board's "in-frame" flag and its scanline counter now come off the PPU bus (three
  identical nametable reads, then the next read), with the idle clear on `on_cpu_cycle` and the
  `$FFFA`/`$FFFB` hook in `cpu_read_override`. `Mmc5::on_ppu_scanline` is gone.
- **No new notification was needed.** `Ppu2C02::read_vram_inner` has told the cartridge every
  address it reads since the first PPU commit, nametable and attribute bytes included; Q-116's
  cost estimate of ~33 000 new instructions a frame for every game was a misreading of the call
  chain. What the lot added is a second load-time latch so the two cold sites where the address
  lines move with **/RD high** — a `$2007` write and a `$2006` write — and the two per-tile
  notifications that drive a pattern address one dot *ahead* of its fetch can be kept off a board
  that waits on /RD. That alone is 229 M instructions of 300 frames of *Castlevania III*.
- **Two regressions that only callgrind could see, both of them MF-3b's rules:**
  - removing the MMC5's hook from `process_dot` left two `if self.dot == 1` tests side by side;
    LLVM merged them and made the second branchless, so the scanline compare ran on every one of
    the ~21 000 dots a frame that reach that function. **+30 to +40 M on every game, NROM
    included.** The capture must stay **nested** inside the one `dot == 1` block.
  - inlining the detection into the MMC5's arm of `Board::on_ppu_address_bus` added **one
    instruction to that dispatch's prologue** (the board tag kept alive across the jump-table
    index), paid by mappers 4, 64 and 118 on every fetch: **+16.6 M on *Super Mario Bros. 3***.
    `Mmc5::see_ppu_read` is `#[inline(never)]` for that reason and must stay so.
- After both: callgrind, 300 frames, against `a5994fd1` — `M0_P32K_C8K_V` **−0.115 %**,
  Mega Man 2 **−0.138 %**, Super Mario Bros. 3 **−0.138 %**, Zelda II **−0.531 %**, Castlevania
  III **+3.013 %**. On the four non-MMC5 ROMs every pinned symbol is **0 to the instruction** —
  `Board::on_ppu_address_bus` on SMB3 included — and the only movers are `run_dots` (−6.4 to
  −34.5 M, the hook), `read_internal` and `write_internal` (−131 k to −16).
- **An MMC3 game belongs in the measurement set.** `Board::on_ppu_address_bus` is reached only by
  mappers 4, 5, 64 and 118, so a regression in it is invisible on the MMC1 and NROM ROMs the
  baseline uses. *Super Mario Bros. 3* is the one that caught it.
- Invariant: a board in `Cartridge::watches_ppu_reads` must return `None` from
  `Mapper::chr_view`, or `Ppu2C02::run_visible_span` takes dots 2 to 255 of every visible line
  without a single notification. Asserted in
  `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`.

**The CPU read cycle reads the bus in its own copy of it** (lot PR-1, Q-119 point 3), `nes_bus.rs`
and `cartridges/board.rs`. `read_internal` is an `#[inline(always)]` body
(`read_internal_body`) with two out-of-line callers — `read_cycle`, which therefore ends its DMA
gate *in* the read instead of a `jmp` into it, and `read_internal` for the dozen other readers of
the bus. Output identical (attributes and wrappers only, no `match` arm reordered); no switch.
- **−5.90 instructions per CPU read cycle**, the `jmp` plus a second function's prologue and
  epilogue. Callgrind, 300 frames, thirteen ROMs against `2754a7e8`: **all thirteen negative**,
  −0.629 % (`Duelito`) to −1.463 % (`ppu_vbl_nmi`), total **−0.841 %**; Mega Man 2 −0.848 %,
  Zelda II −0.773 %, SMB3 −0.640 %, SMB1 −0.831 %, Metroid −0.974 %, an MMC5 −0.733 %.
- **It needs two companions, and neither is optional.** The rare arms of the body are out of line
  and `#[cold]` (`read_io_register` for `$4000`-`$401F`, `read_wide_window` for the widescreen
  window over `$5000`-`$7FFF`), so the two copies stay small — without them an MMC5 is +0.018 %.
  And `Board::cpu_read_override` carries `#[inline(always)]`, without which the same MMC5 is
  **+0.586 %**: it is the one board that reaches that dispatch on *every* read cycle, mapping
  PRG-RAM into `$8000`-`$DFFF` so `Cartridge::read_rom_window` cannot answer.
- **What it costs, and why it cannot lose.** LLVM moves `Ppu2C02::read` and
  `Cartridge::try_cpu_read` out of the read path rather than duplicate them — MF-3b rule 1's
  stop-the-lot signature, so it was priced: **+4.09 instructions per PPU register read**. A
  register read *is* a read cycle, so it saves 5.90 before paying 4.09. Measured on a ROM that
  spends a fifth of its read cycles on `$2002` (`Duelito`, 4 024 register reads a frame against
  Mega Man 2's 30): still −0.629 %.
- **Do not** add `#[inline(always)]` to `Ppu2C02::read`: its prologue then lands on every read
  cycle instead of on register reads alone, which measures +0.02 % overall and +0.89 % on an MMC5
  — the worst of the nine variants PR-1 tried.
- **The measurement set belongs to the change.** `rust/tools/perf/` holds the whole procedure
  (`make-tree.sh`, `build-clean.sh`, `callgrind-set.sh`, `callgrind-diff.py`,
  `callgrind-symbols.py`, `roms.txt`): thirteen ROMs chosen by *measured* register-read rate, run
  in parallel, 43 s of wall clock for a whole set. Section 3's procedure, as a script.

**Register-edge accuracy at a named cost, and a bug in the instrument** (lot MF-4d)
- `AccuracyCoin` 125 -> 129/147: the `$4016` strobe's put cycle, the `$4015` bit-6 clear's get
  cycle, a `$4010` write on the DMC timer's reload cycle, the `$2002` sprite flags' sampling
  instant, and a `$2007`/`$2004` access made while the PPU is fetching.
- **`callgrind-set.sh` was dropping five of its thirteen ROMs** — R0's five commercial games.
  Its `while read` was fed by a **pipe**, so the loop ran in a subshell that exited without
  waiting for the last batch of background jobs; the outer `wait` cannot see them. The only trace
  is a `cg-` directory with eight `.ir` files and five empty `.log`s. Fixed by feeding the loop
  from a here-document, which is a redirect and keeps the loop in the current shell. **Any
  measurement taken with the piped version covered eight ROMs, not thirteen.**
- Every per-cycle symbol is **below the 100 000 threshold on all thirteen ROMs**: `Cpu6502::step`,
  `branch`, `resolve`, `NesBus::tick`, `read_cycle`, `read_internal`, `run_dma`,
  `Apu2A03::run_cpu_cycles`, `quiet_cycles`, `Board::ppu_read`, `on_cpu_cycle`,
  `background_fetch`. What is paid is in the register arms: `Ppu2C02::read` +141 k to +2.12 M by
  `$2002`/`$2007` read rate, `NesBus::write_internal` +183 k to +228 k.
- **Two attributes are load-bearing and were measured, not guessed.** `#[inline]` on
  `Ppu2C02::increment_vram_address`: without it the new `fetching` branch pushes the body past the
  inliner's budget and it becomes a real function with **one call site on every `$2007` access**.
  And the `& 0x0F` on `DmcChannel::write`'s rate index: without it LLVM cannot prove the index in
  range and emits a `core::panicking::panic_bounds_check` **call site inside
  `NesBus::write_internal`**.
- **Three ROMs' totals move because their pictures move, not because the core got slower**:
  `duelito` −3.93 %, `smb3` +0.46 %, `z2` +0.02 %. `Ppu2C02::run_dots` is byte-identical after
  LTO at every step of the lot and still executes more dots on those three, while `duelito`'s
  −295 M is almost all `zax_engine3d`. The cause is the `$2007`-while-fetching rule (nesdev;
  `third_party/mesen2` agrees), and `golden-core` confirms CPU cycles and audio bit-identical on
  all six games measured. **Super Mario Bros. 3 and Zelda II need a widescreen-pack A/B.**
- Whole set: **−0.310 %** over the thirteen ROMs.

Combined effect of the core work: SMB1 -16 %, Metroid -15 %, MM2 -12 % on top of the
milestones, device core about 17.4 -> 10-12 ms per NES frame.

### 5.2 3D conversion engine (`zax-engine3d`)

C# counterpart: `src/Zax.Engine3D` (segmentation, frames, draw list, geometry).

**Fast hasher for the per-frame maps** (`5f056be`)
- `zax_engine3d::hashing::FastMap` / `FastSet` (rustc's Fx multiply-rotate) for instances,
  patterns, tile images, meshes, tracker, and the renderer's mesh/texture caches and draw-state
  groups. SipHash was about 18 % of the conversion + Lua + draw-list instructions.
- Identical output (no map relies on its iteration order).
- Zelda II world map x86: draw list 0.24 -> 0.14 ms, conversion 0.14 -> 0.126 ms.
- Invariant: never iterate a `FastMap` where the order reaches the output; sort first.

**Draw list placement matrix computed once per instance** (`c39e2dd`)
- `Pos . R . S` (REQ-E3D-406) is computed once per instance and multiplied by each tile's
  translation, in the same order as the full product, so the floats are identical
  (`InstanceLook::placement` in `draw_list.rs`). Draw list instructions -10 %.
- Invariant: keep the multiplication order, or the floats change.

**No draw list in 2D mode** (`13f8a08`)
- The renderer shows only the core's picture in 2D, so the list is not built (the C# loop built
  it every frame).

**Only the filled grid cells are emptied and scanned** (`608dde6`)
- The widescreen margins made the conversion grid 13x larger (163x93 cells instead of 35x33), and
  every batch emptied all of it, then the segmenter scanned every cell for germs. The frame now
  remembers the cells it fills, empties only those, and hands only those to the segmenter, still
  sorted by position, so the shapes are the same.
- Identical output. MM2 0.086 -> 0.040 ms, SMB1 0.071 -> 0.019 ms per frame (desktop).
- Invariant: the germination order is part of the output (greedy segmentation, REQ-E3D-090); any
  change to germ collection must keep the position sort.

**Wide 2D picture composed three times faster** (`fbdf979`)
- The wide picture was composed from every tile record pixel by pixel, recomputing ranges per
  pixel. The centre range of each row is computed once, tiles wholly in the centre are skipped,
  and the centre of each row is copied whole.
- Identical output (21 runs, seven games, wide/advanced/vertical margin). 0.15-0.24 -> 0.04-0.10 ms.

### 5.3 Lua scripting (`zax-scripting`)

C# counterpart: `src/Zax.Scripting` (the binder, host values, runtime). In C# the binder is
MoonSharp/NLua-style reflection; in Rust it is mlua. The per-call cost of mlua (about 1000
instructions per Lua-to-host call) is the main cost, not the interpreter.

**Binding rounds 1 and 2** (`2a76217` to `7446656`, `4795df3`)
- Frame handles keyed by integers (slot of group and member) and retired into per-epoch weak tables
  keyed by integers instead of 16-byte strings (retirement was 18 % of a frame).
- A frame handle keeps its identity after its epoch (REQ-LUA-043), as the C# binder does; costs
  about 45 us per frame but is required.
- Budget clock started at the first look of the count hook, not at each hook call.
- Member lookup by bytes (no UTF-8 check).
- Frame epochs read from the object cache (no borrow of the runtime data per access).
- The `UpdateS` function and `self` fetched once per script.
- Published hook globals and cached script handles; host writes queued behind the next request;
  the runtime lent once for all instances of a pattern.
- Result (synthetic frame: `Update` + `LateUpdate` + 60 `UpdateS`): 397 -> 203 us on x86.
- Invariant: the Lua API dump must stay byte-identical.

**Smooth incremental collector** (`e205ddd`, setting `emu.lua_smooth_gc`)
- Pause 120 %, step multiplier 400 %, 256-byte steps, instead of the original 400 / 400 / 8 KiB:
  collection is spread over the frames instead of stalling one.
- On by default on desktop and handheld. Scripts see no difference except
  `collectgarbage("count")` (no profile of the corpus calls it).
- Worst Lua frame on Zelda II x86: temple 136 -> 2.5 ms, world 126 -> 1.3 ms; mean equal or lower.
- History: first tried as the generational collector (`9f9136e`); its major collections stalled
  120 ms once the heap reached 49 MB, so it was replaced.
- Note for measurements: the harness starts with the original tuning; use `ZAXNES_LUA_GC=smooth`
  or `bench-core` shows spikes the application does not have.

**Optimised Lua** (setting `emu.lua_optimized`, "Optimized Lua (unsafe)", desktop off, handheld
on; `f0be2a5`, `bbe5584`, `7eea4fa`)
- Owner decision to allow `unsafe` in `zax-scripting`, in documented modules only (the crate is
  `deny(unsafe_code)` elsewhere).
- Step 0 (`binding/lend.rs`): the runtime data, engine state and instance being built are lent to
  Lua calls as pointers in a per-state slot instead of mlua scoped userdata; a busy bit per object
  keeps one mutable borrow at a time.
- Step A (`binding/raw.rs`): host values are bare userdata blocks (magic + payload) sharing one
  metatable whose `__index` / `__newindex` are C functions; getters, setters and method lookups
  skip mlua's userdata entry; other cases fall back to the safe binding.
- Step B: a returned host value is pushed by the metamethod itself, the shared metatable is an
  upvalue and a registry integer key, methods are C closures reading their receiver from the
  block, the state's `Lua` is an upvalue; coroutines go to the mlua function.
- Off runs no `unsafe` code and forces `emu.pipeline` off.
- x86 Lua per frame: Zelda 1 0.65 -> 0.36 ms, Zelda II 0.37 -> 0.20 ms; worst Zelda 1 frame
  36 -> 3 ms (less garbage per access). All tests pass in both modes (`ZAXNES_FAST_LUA=1` runs the
  test suite with it on).
- Invariant: any new host type or member must be reachable through both paths and give the same
  API dump.

**Experimental Lua** (setting `emu.lua_experimental`, off everywhere; `926fb34`)
- On top of the optimised Lua: `UpdateS` hooks that only use fields of `shape` (decided from the
  source when the profile loads) receive one light `shape` per list, pointed at the instance of
  each call, and no `shape2D` (`runtime/light_shapes.rs`).
- Off by default because the source analysis is a heuristic.

### 5.4 Renderer (`zax-render`)

C# counterpart: `src/Zax.Render/Gl` (scene passes, shaders, texture/mesh caches).

**Uniform value cache and binding cache** (`83aa286`)
- `ShaderProgram` keeps the bits last uploaded per uniform and skips identical uploads (FNV hash
  for the name lookup: SipHash was about 2 % of all instructions). Textures and vertex arrays
  are rebound only when they change.
- Identical output. RTX 3060 render 0.44 -> 0.33 ms; Zelda II temple scene on device 29 -> 17 ms.

**Instanced draws through a uniform block** (`cf00717`)
- The scene shader reads each draw's placements from a uniform block (`ZaxInstances`: model
  matrix, normal matrix columns, deformation tile position; 128 instances of 128 bytes per draw;
  one buffer upload per frame). Consecutive items with the same draw state (`DrawStateKey`:
  everything but the placement and the deformation tile position) go out as one
  `glDrawElementsInstanced`.
- Identical output. `RenderStatistics::draw_calls` still counts tile draws (goldens); `batches`
  counts GL calls.
- On its own only -5 to -8 % calls (the list interleaves meshes); the gain comes with grouping.

**Grouped opaque draws** (handheld only; `cf00717`, `5f40b61`, `0998e15`)
- Reorders the opaque bucket by draw state (states in first-appearance order, list order inside a
  state), and the tiles inside each transparent object (depth-only pass then equal-depth colour
  pass, so only exact ties matter; the HUD is never reordered).
- Output changes only on exact depth ties between different states: Zelda II world map 4 pixels
  of 86 400; 22 of 36 captures differ by at most 0.002 %. Hence a platform switch: on with
  `--handheld`, off on desktop, `ZAXNES_GROUP_DRAWS=0/1`, harness `--group-draws`.
- Temple 934 -> 138 GL draws, world map 1559 -> 72, Blaster Master 330 -> 206 (transparent).
- `0998e15`: the batching gives each item a draw-state label once and orders indices, instead of
  moving the draw items.
- Invariant: a new per-draw shader input belongs in `DrawStateKey` (or in the instance block),
  or two different states will be merged.

**Opaque draws without `discard`** (`04fc7d9`)
- A tile wholly inside the clip rectangle (its 8 placed bounding-box corners, with a 0.01 margin;
  `inside_clip` in `frame_plan.rs`) no longer needs the clip test. The clip flag is no longer
  part of `DrawStateKey`: a state is drawn clipped when any of its tiles needs it, so the flag
  neither splits runs nor changes the grouped order. States that never clip and read no colour
  texture use the `ZAX_SOLID` variant of the scene shader (`shaders::SCENE_SOLID`), which has no
  `discard` and lets a tiled GPU reject hidden fragments early.
- Identical output (56 captures, both draw modes). 8K bench RTX 3050: SMB3 2.26 -> 2.09 ms, MM2
  unchanged.
- History: a first version kept `clip` in the key and lost 7 % on MM2 (40 -> 101 GL draws) and
  changed one pixel through the grouped order; the per-state unification fixed both.
- Invariant: any new `discard` in `scene.frag` must be inside `#ifndef ZAX_SOLID`, and the
  condition for using the solid program (`!clip && color_mode != 1`) must exclude it.

**Palette linearised once per frame** (`e155539`)
- The 64 palette entries go through `zaxToLinear3` in a 64x1 pass into an `RGBA32F` texture
  (`gl/linear_palette.rs`, `shaders/linear_palette.frag`); the scene shader reads indexed albedo
  from it instead of three `pow` per fragment. Same GPU, same function, so identical values.
- CPU fallback (`color::rgb_to_linear`) when the float target is incomplete; last-bit differences
  possible there.
- Identical output (56 captures). No gain on the RTX 3050 (MM2 +2 % at 8K: `pow` is cheap there
  and the 128-bit fetch costs a little); aimed at the Mali's slow transcendentals, not measured on
  the device. `ZAXNES_SRGB_PALETTE=1` restores the per-fragment conversion (shader variants
  `SCENE_SRGB_PALETTE`, `SCENE_SOLID_SRGB_PALETTE`) for an A/B on the device.

**Shader program binary cache** (`8efd710`)
- `gl/program_cache.rs`: with `glGetProgramBinary`, each linked program is saved in
  `<userdata>/shader-cache/<hash>.bin` (hash of GL renderer and version strings, program name and
  both sources) and loaded at the next start. Damaged or refused binaries are recompiled.
- Start-up time on the handheld.

**Automatic resolution** (setting `video.dynamic_resolution`, off everywhere by default; `86b3bcd`)
- GL timer queries (`gl/gpu_timer.rs`) read a few frames late; the scale moves among native, 5/6,
  2/3, 1/2. Down after 3 measures over 85 % of the budget, up by probing after about 5 s without a
  late frame.
- Mali-G31 pitfall: the timer answered 4295 ms (the largest 32-bit result); results of 0 or of a
  second or more are ignored, and the timer is given up after 300 results without a measure.
- The existing `video.res_scale` (2/3, 1/2, 1/3, 1/4 through `video.render_method` = Render Tex)
  is the C# feature; only the automatic mode is Rust-only.

**wgpu backend batching** (`63c95cd`, wgpu feature only)
- Texture arrays and multi-draw indirect where the device allows; 80-byte draw blocks and 64-byte
  placements. Not used by the handheld (GL path). `ZAXNES_WGPU_BATCHING=0` disables it.

**The curvature is a shader variant, not a uniform** (RU-4a, AMD-043 REQ-SCN-067)
- `gl/shaders.rs` composes a variant in a `const fn`: `Stage` holds its chunks in **fixed slots**,
  empty ones skipped, and `scene(solid, srgb_palette, bend)` / `shadow(bend)` replace the four
  `SCENE*` constants and `SHADOW`. Eight scene variants and two shadow ones, ten distinct names
  so the on-disk program cache files them apart. `assemble` skips an empty slot without even a
  blank line, so leaving a define out produces the source the stage had before slots existed.
- **At curvature 0 — all four settings of REQ-SCN-066 default to 0 — nothing of the curvature is
  compiled, swapped or uploaded.** `select_bend_variant` is one `is_identity()` test per frame;
  the curvature pair is built the first time a setting is not 0, which on a handheld is seconds
  of shader compile a player who never bends anything never pays.
- Invariant, and the test that holds it:
  `zax-render/tests/bend_tests.rs` `req_scn_067_the_flat_variant_carries_nothing_of_the_curvature`
  preprocesses the assembled source the way the GLSL compiler would — `#ifdef ZAX_BEND` branches
  dropped, `#else` kept, line comments removed — and asserts that none of the nine identifiers
  the curvature brings in reaches the compiler, over all eight flat scene variants (desktop and
  GLES) and both flat shadow ones. **A uniform tested per vertex is the regression**; a per-axis
  `uBendOn` *inside* the variant is not, it is uniform over a whole draw and saves an idle axis a
  `sin` and a `cos`.
- Measured on the `framing` scene (`zax-harness render --frames synthetic:framing`): default 6
  distinct colours, **1798 draw calls, 7704 triangles**; `--bend 90,0,0` 416 colours and the
  **same 1798 draw calls and 7704 triangles**. The curvature adds no draw call and no triangle.
- The subdivisions exist only while something bends (`interop::subdivision_for`, the single home
  of the rule, read by the renderer and by `scene_export`): the backdrop box stays its six quads
  and the 2D rectangle its single quad at curvature 0. In 2D mode the rectangle goes 2 triangles
  → 2048 → 2 again when "Bend 2D Mode" is turned off, in one draw call throughout.
- Q-104: the curvature landed in the **OpenGL backend only** in this lot, and RU-7 ported it to
  the `wgpu` one — see the RU-7 section below. The known-gap test that pinned the gap is now its
  own opposite.

**The opacity mask of AMD-050 REQ-CORE-049 costs the PPU nothing** (RU-4a) — a core change, listed
here because it is the one measurement of the lot that had to come out at zero.
- The bit rides in **bit 6 of each byte of the picture**, beside the colour index of REQ-CORE-048
  in bits 0-5 (`ppu::OPAQUE_PIXEL`, `PIXEL_COLOR`). That is the internal layout the requirement's
  note allows, and the only free one: `entry`, the palette index the two pixel functions resolve,
  is non-zero **exactly** where the pixel carries an opaque pixel, so a derived
  `palette_marked[i] = palette[i] | 0x40` (`i ≥ 1`) and a colour mask widened to `0x7F` / `0x70`
  carry the bit with the same indexed load and the same `AND`.
- Measured on the post-LTO assembly, `Ppu2C02::run_dots` (both pixel functions are inlined into
  it): **2231 instructions before, 2231 after**, and with the label numbering, the anonymous
  symbol hashes and the `(%rbx)` displacements normalised the entire diff is five immediates
  (`$48 → $112` three times, `$63 → $127` twice) plus the palette load reading `palette_marked`
  at +32 instead of `palette`. No instruction added, removed or reordered.
- Invariant: **`palette_marked` is maintained where `palette` is written and nowhere else** —
  the one register write, the power-up fill, the state restore. A rebuild in a per-frame path, or
  a second store per pixel, is the regression. It is derived, so it is not in the state blob.
- On the renderer's side nothing new runs in a shader: `gl/picture.rs` already resolved the
  picture to RGBA once per frame **in 2D mode only** and already wrote alpha 0 for a pixel the
  overscan leaves out, and `scene.frag` already discarded on alpha 0 in the opaque pass.
- The 291 PPU golden hashes are unchanged: the two hashers mask with `PIXEL_COLOR` through
  `pixel_frame_indices_into`, and the accessor was **renamed** (`pixel_frame` →
  `pixel_frame_with_mask`) so the compiler visited all eleven call sites.

**The walls came back and the draw list got cheaper** (RU-4b, AMD-057)
- AMD-057 withdraws the Q-067 seal: every tile mesh is drawn whole (REQ-E3D-501), and REQ-SCN-065
  is held instead by a **depth bias on the inner walls**, in the scene vertex shader, from the
  sign of the **local** normal (`abs(aNormal.z) < 0.5`). Only `gl_Position.z` moves, by
  `uWallClipBias / gl_Position.w`, which is a fixed scene distance behind the vertex; `x`, `y` and
  `w` are untouched, so no silhouette and no texture coordinate moves with it.
- The per-instance neighbour map disappeared with the seal, and with it two hash operations per
  tile. `DrawListBuilder::build` (with `add_instance` and the former `sealed_sides` inlined) goes
  **1574 → 1095 instructions, −30 %**, and the wall clock of `bench-core … --3d` on 982 tile
  records a frame goes **0.0370 → 0.0314 ms/frame**.
- The GPU pays 0.76 % more vertices over the ten profiles AMD-057 §1 compares (worst case
  Castlevania, +5.6 %), and **saves nearly half the mesh cache**: the Q-067 variants were separate
  meshes, up to sixteen per tile image, and the resident mesh count over the eleven synthetic
  scenes falls **176 → 81**. On a handheld both GPU memory and mesh uploads are scarcer than
  vertices, so there is **no handheld exception** and no setting: the spec path runs everywhere.
- Invariant: `uWallClipBias` is **0 for a mesh whose normals are not axis aligned** (MARCHING
  merges its tiles, REQ-E3D-520, so it has no joint to hide). The flag rides on `GpuMesh`, read
  from `TileMesh::has_axis_aligned_normals`, computed **once when the mesh is built** — the draw
  path reads a bool off a handle it already has, and no field was added to the per-tile list.
- The test that bites is the translucent one (REQ-SCN-040), not the joint ones: on a surface of
  one object the coincident walls lose the fill rule anyway, but in the equal-depth colour pass of
  a translucent object `depth_func(EQUAL)` keeps a coincident wall and blends that pixel twice.

**The sprite depth by index costs one multiply-add** (RU-4b, AMD-057 REQ-SCN-037)
- The offset becomes `0.001 × (i mod 64) ∓ 0.1`, computed in `frame_plan::collect_tile` where the
  constant was, from the `FrameTile::ord()` the draw list already carried. **Nothing was added to
  any per-tile record**, no setting is consulted, and the `if nudge != 0.0` guard that skips the
  matrix multiply for a background tile is untouched.
- Worth knowing before trusting a capture: the span of the index term is **0.063 scene unit** over
  the 64 indices, which at the default setpoint (about 148 units, near 0.3 / far 5000) is **1.6
  least significant bits of a 24-bit depth buffer**. It resolves, barely. A 16-bit depth buffer
  would not resolve it at all; the context asks for 24 on every target.

**The centroid variant of AMD-058 does not exist until anti-aliasing does** (RU-4b)
- `vUv` is interpolated at the centroid of the covered samples, as a **shader variant** keyed on
  `video.anti_alias`, so with the setting at 0 — the handheld default — the code the GLSL compiler
  is given is the code of before the amendment, **line for line in all 40 assembled sources**
  (measured by dumping them before and after and diffing the `ZAX_CENTROID`-undefined
  preprocessing). No centroid program is compiled, and the eleven synthetic captures are identical
  at tolerance 0.
- The *source text* does change, because an `#ifdef` is text, so the eight pixel-centre programs
  get new **program-cache keys** and are compiled once more on the first start after the update
  (`gl/program_cache.rs` hashes the driver, the name and both sources). Their **names** are
  unchanged, so nothing else about the cache moves. Same one-time cost AMD-043 paid.
- `ScenePrograms` was rebuilt around a slot table for this: `built` is a flat `Vec<ShaderProgram>`
  and `slots[key]` holds the plain and solid indices of each `(bend, centroid, backdrop)` key, so
  `current()` — which **every draw run** calls — is two indexings and **no branch**, where it used
  to be a two-arm match plus an `Option` test. An unbuilt key points at the flat scene pair, so a
  failed compile degrades to the model of before rather than to no frame at all.
- Invariant to keep: the interpolation is chosen **before** the curvature
  (`select_centroid_variant` then `select_bend_variant`), so the two pairs the curvature builds
  are the ones the frame and its flat HUD actually use — two programs, not three.

**The backdrop lighting is a program of its own, off on handhelds** (RU-4b, AMD-049 REQ-SCN-075)
- The almost-matt model replaces the diffuse term on the backdrop alone. It is a **separate
  program** (`shaders/backdrop.frag`), not a fifth variant axis and not a per-fragment test on
  `uColorMode`: the backdrop is one opaque run a frame, carries a flat colour, and reads no texel
  coordinate — so it holds **no** atlas lookup, no palette, no `uColorMode` and no fragment
  rejection, and no tile fragment is given any of the model's identifiers (a test asserts both
  directions). Only the curvature axis is left, so the amendment costs at most two lazily built
  programs.
- Per frame, two new things and no more: `select_backdrop_variant`, one `&&` of two booleans; and
  `set_backdrop(item.color_mode == 2)` per **run**, riding on the `color_mode` the next line
  already reads for `use_solid`, and returning "nothing changed" — hence no GL call — whenever the
  variant was not built.
- Measured free on a desktop GPU: `render --frames synthetic:backdrop --bench 60` gives 0.19 /
  0.20 / 0.19 ms median with the model against 0.20 / 0.19 / 0.18 without it at 640 × 480, and
  0.22 / 0.23 against 0.24 / 0.26 at 1280 × 720. **The handheld default is still off**
  (`render.backdrop_lighting`): the cost is per fragment on a surface that can fill the picture,
  and a desktop measurement does not measure a Mali-G31. Offered as an advanced option, one pad
  step away. The figures are here for the owner to revisit on a real device.
- Invariant: the setting is honoured **per frame** (`backdrop_wanted`), not by whether the program
  exists — a program built for one frame outlives the setting that asked for it, and turning the
  setting off has to go back to REQ-SCN-048's diffuse term without throwing the program away. A
  test caught exactly this.

**The Enhanced style needs no GL feature the renderer did not already need** (RU-5, AMD-055
REQ-SCN-080 to REQ-SCN-084)
- The four effects of the Enhanced style use **three** formats, and all three were already
  mandatory for Zaxnes before the amendment, so there is nothing to probe and nothing to grey out
  for want of a capability:
  - `RGBA16F` as a colour target, for the reflection trace of REQ-SCN-084. It is what
    `LinearColorBuffer` has rendered **every** frame into since AMD-021 R2-009 — colour-renderable
    in every GL 3.0 and later implementation, and on GLES 3.0 through `EXT_color_buffer_float`,
    which the renderer already depends on without a fallback;
  - `R8` as a colour target, for the occlusion of REQ-SCN-083 and for the floor mask of
    REQ-SCN-084. Colour-renderable in OpenGL 3.3 core and **required** renderable in GLES 3.0;
  - `DEPTH_COMPONENT24` as a **sampled texture**, for the depth both screen passes read. Same
    format and same transfer type as the shadow map of REQ-SCN-030, which every target already
    allocates and samples.
  The one thing that did need care is not a capability: with anti-aliasing on, the depth the mask
  pass tests against is a **resolved** multisampled depth, and an exact `LEQUAL` against it marks
  nothing at all. A polygon offset of −16 units of the last place settles it at both sample counts
  (`gl/screen_effects.rs::begin_mask`).
- **Consequence of REQ-SCN-080's own order, worth stating**: with anti-aliasing **and** a screen
  effect on, the colour and the depth are resolved before the passes run ("après résolution du
  MSAA éventuel"), so the transparent objects and the HUD — drawn after the passes — are drawn into
  the **resolved single-sample** target and are therefore not multisampled. That is the
  requirement's order, not a shortcut.
- **A refusal is survivable.** If an implementation nevertheless refuses one of the three, the
  frame is drawn **without** the effects — the Classic picture, cheaper and never more expensive —
  one diagnostic line says so, and the effects are given up for the session rather than retried
  every frame (`SceneRenderer::prepare_screen_effects`). There is no probe at context creation,
  deliberately: a probe would have to allocate, and point 1 of the RU-5 brief forbids Classic
  allocating anything the amendment needs. The first Enhanced frame is the probe.
- `wgpu` (Vulkan, DirectX 12): in this lot the style was read back as set and the scene drawn in
  **Classic**, with one log line — the same gap the curvature had there, recorded in `Q-104`.
  **RU-7 closed it**, log line and all; see its section below.

**What each Enhanced effect costs when it is on** (RU-5, AMD-055; `--bench 60 --anti-alias 0`,
median of 60 frames, each ended by a `finish()`)
- Enhanced with **every effect off is Classic**, measured and not only asserted: 0.451 against
  0.460 ms at 1280 × 720 and 0.529 against 0.572 at 1920 × 1080 on `framing`, 1.026 against 1.064
  and 1.108 against 1.067 on `heavy` — inside the ±0.05 ms run-to-run spread, in three of the four
  rows on the faster side — with **0 bytes** of targets and 0 passes submitted.
- `framing` at 1280 × 720 → 1920 × 1080, against a Classic baseline of 0.460 → 0.572 ms:

  | Effect | 720p | 1080p | target bytes, 720p / 1080p |
  |---|---|---|---|
  | voxel grid `N` = 4 | +0.03 ms | +0.03 ms | 0 / 0 |
  | pixel shadows | +0.00 ms | −0.01 ms | 0 / 0 |
  | occlusion Depth, `ao.quality` Low | +0.13 ms | +0.30 ms | 460 800 / 1 036 800 |
  | occlusion Depth, `ao.quality` Medium | +0.15 ms | +0.30 ms | 460 800 / 1 036 800 |
  | occlusion Depth, `ao.quality` High | +0.53 ms | +1.11 ms | 1 843 200 / 4 147 200 |

- `heavy` at **45° of camera elevation**, against a Classic baseline of 1.064 → 1.067 ms. The
  elevation is not decoration: a floor surface needs a face of pre-curvature scene normal
  `+Y ≥ 0,9` on an instance whose `xz` box reaches 6 units in both dimensions, and at the default
  camera the only candidates — the top faces of the deeper tile boxes — are edge-on, so **no pixel
  is marked** and `ssr.frag` returns at its mask test. Measured without the elevation, the
  reflections read +0.08 ms, which is the cost of one texture fetch per pixel and not a figure.

  | Effect | 720p | 1080p | target bytes, 720p / 1080p |
  |---|---|---|---|
  | reflections, `ssr.quality` Low | +0.75 ms | +0.87 ms | 2 764 800 / 6 220 800 |
  | reflections, `ssr.quality` Medium | +0.71 ms | +0.89 ms | 2 764 800 / 6 220 800 |
  | reflections, `ssr.quality` High | +0.80 ms | +0.89 ms | 2 764 800 / 6 220 800 |
  | reflections, `ssr.quality` Ultra | +0.91 ms | +1.02 ms | 8 294 400 / 18 662 400 |
  | occlusion Depth, Medium (cross-check) | +0.11 ms | +0.30 ms | 460 800 / 1 036 800 |
  | all four effects on | +0.89 ms | +1.15 ms | 3 225 600 / 7 257 600 |

- Reading of the quality rungs, which is what the handheld options need: **Low and Medium are the
  half-resolution rungs** of both effects and High/Ultra the full-resolution ones, and that — not
  the sample or step count — is what moves the figures. The occlusion goes ×3.5 in time and ×4 in
  memory from Medium to High at 1080p; the reflections barely move across Low, Medium and High
  (the march is paid only on the marked pixels) and pay their Ultra in **memory**: 17.8 MiB at
  1080p against 5.9.
- The targets, exactly: the occlusion is two `R8` targets at half of each dimension (full at
  `ao.quality` High); the reflections are one `R8` mask at the frame's own size plus one
  `RGBA16F` trace at half of each dimension (full at `ssr.quality` Ultra).
- The effects **add up and do not multiply**: all four on costs 1.957 ms against the 1.99 the sum
  of the deltas predicts.
- **Handheld**: Classic stays the default (`render.visual_style` = 0 is the setting's own default,
  and at Classic the amendment costs nothing anywhere). The order to offer them in, on these
  figures: the voxel grid and the pixel shadows first (no target, no pass, cost inside the
  measurement spread **on a desktop GPU**), then the occlusion at Low, then the reflections —
  whose mask pass is geometry re-drawn and whose march is dear on exactly the GPUs whose screens
  would have made the fill cheap. `ao.quality` and `ssr.quality` default to **Low** on a handheld
  (`ui/handheld/defaults.rs`) and to the amendment's 1 and 2 on the desktop.

**The MSVO occlusion adds one format, and it is one the renderer already had** (RU-6, AMD-055
REQ-SCN-083 *Mode MSVO*)
- The multi-scale estimator (`gl/msvo.rs`, ported from the MIT-licensed MiniEngine) needs a fourth
  format beside the three above: **`R16F` as a colour target**, for the four levels of its linear
  depth pyramid. It is colour-renderable in OpenGL 3.3 core, and on GLES 3.0 through the very
  `EXT_color_buffer_float` / `EXT_color_buffer_half_float` that already makes the `RGBA16F` of
  `LinearColorBuffer` renderable on every frame — a driver that grants one grants the other. So
  there is still nothing to probe and nothing to grey out, and still no probe at context creation.
- Sixteen bits are **enough**, but only because the access of each level is stored with a factor-2
  headroom (`style::MSVO_ACCESS_HEADROOM`). Without it, clamping the access at 1 before the
  smoothing turns the estimator's zero-mean sampling noise into a one-sided bias that four pyramid
  levels multiply: measured, a lone plane came out at 0.949 against the 1.00 ± 0.02 §2.4 requires,
  and `R32F` did **not** fix it. With the headroom in, `R32F` changes no figure of §2.4 and `R16F`
  stays.
- **A refusal falls back one level, not all the way.** Where the three formats above are given up
  for the whole screen-effect module, a refused pyramid gives up only the **mode**: `spl.ao` = 1
  then renders exactly as `spl.ao` = 2, which is REQ-SCN-083's declared fallback **(D)**, with one
  diagnostic line and no retry (`ScreenEffects::msvo_refused`).
- **One estimator or the other is built, never both.** `spl.ao` = 2 compiles the hemisphere pass,
  its blur and the shared composite (3 programs); `spl.ao` = 1 compiles the five of the port and the
  same composite (6). Switching the mode releases the one that leaves, pyramid and all.

**What the MSVO occlusion costs when it is on** (RU-6, AMD-055; `--bench 120 --anti-alias 0`,
median of 120 frames, each ended by a `finish()`, `framing`)
- The Classic baseline of this session reads 0.47 → 0.65 ms and the Depth rows re-measure RU-5's
  within the session spread (0.86 / 0.89 / 1.71 ms at 1080p against its 0.867 / 0.871 / 1.682),
  which is what makes the two tables comparable. Every delta below is against the baseline of
  **this** run.

  | Effect | 720p | 1080p | passes | target bytes, 720p / 1080p |
  |---|---|---|---|---|
  | occlusion Depth, `ao.quality` Low | +0.15 ms | +0.21 ms | 3 | 460 800 / 1 036 800 |
  | occlusion Depth, `ao.quality` Medium | +0.12 ms | +0.24 ms | 3 | 460 800 / 1 036 800 |
  | occlusion Depth, `ao.quality` High | +0.45 ms | +1.06 ms | 3 | 1 843 200 / 4 147 200 |
  | occlusion **MSVO**, `ao.quality` Low | +0.20 ms | **+0.49 ms** | 20 | 1 526 400 / **3 434 160** |
  | occlusion **MSVO**, `ao.quality` Medium | +0.41 ms | **+0.64 ms** | 20 | 1 526 400 / **3 434 160** |
  | occlusion **MSVO**, `ao.quality` High | +1.00 ms | **+2.74 ms** | 20 | 6 105 600 / **13 737 600** |

- **MSVO costs 2.3 to 2.7 times Depth** at the same rung, and the pass column says why: twenty
  full-screen passes against three — one linearise, three decimations, four occlusions, eight
  smoothings and three upsamples. Only the four occlusion passes sample; the other sixteen are
  cheap, and together they are the factor.
- **The memory is exactly 3.31 times Depth's at every rung**, and derivable: four levels, each one
  `R16F` depth plus two or three `R8` occlusion targets, at `finest >> level`. At 1080p that is
  3.28 MiB at Low and Medium and **13.1 MiB** at High, against 0.99 and 3.96. On a 1 GB handheld
  only the first figure is in reach, and `spl.ao` = 0 is the console default.
- The rungs: Low and Medium are both at **half** resolution and differ only by the sample pattern
  (four checkerboard pairs against seven), so they are 1.3 apart at 1080p; High is the same chain at
  full resolution and costs ×4.3 of Medium, the ratio the pixel count predicts. **Medium is the
  rung to offer**, as it is for Depth. The number of *scales* stays 4 at every rung (`Q-097`): the
  coarsest level carries the wide half of a multi-scale estimate, and dropping it at Low gives a
  lighter effect, not a cheaper one.
- Worth knowing when reading a capture: on `framing` — 1798 separate tile boxes with the lateral
  walls of REQ-E3D-501 at every joint — the **Depth** estimator changes not one pixel while MSVO
  darkens 10.7 % of the frame by at least one sRGB level and 22 pixels by four, with a frame mean of
  0.22 levels and no banding. The hemisphere estimator tests the rise above a reconstructed tangent
  plane, which a one-unit joint step between coplanar faces does not pass; the sphere test of the
  multi-scale one registers it at its finest level.

**The `wgpu` backend draws the whole picture now, and a backend that is never executed accumulates
faults no review finds** (RU-7, AMD-043 / AMD-046 / AMD-049 / AMD-055 / AMD-058, `Q-104` §6)
- Q-104 recorded that the curvature of AMD-043/AMD-046 and the Enhanced style of AMD-055 were
  rendered by the OpenGL backend alone, and gave one reason: WGSL is validated by naga at module
  creation, so it could not be verified on the machines of RU-4a and RU-5, where
  `render_wgpu_tests.rs` skipped for want of a driver. The owner's answer was **port it**.
- **The premise was false on a machine that has a driver**, and the skip it rested on was hiding a
  defect in its own test. Both adapters answer here (Vulkan and DirectX 12, RTX 3050), and
  `render_wgpu_tests.rs` did not skip — it **failed**, at 93.125 % of the pixels on `heavy`,
  identically to the thousandth on both APIs. It was the test: its OpenGL twin passes
  `--anti-alias 0` and `--no-backdrop-lighting` and says in eight lines why, and the `wgpu` one
  passed neither. Rendered with those options the **OpenGL** backend fails the same scene by the
  same margin. So the `wgpu` golden check had **never once been executed**. A registry probe is not
  evidence either: `HKLM\SOFTWARE\Khronos\Vulkan\Drivers` does not exist on this installation,
  and the loader finds the ICD through the display adapter's PnP registry instead.
- **Three defects, none visible in either shader, all found by measuring on a device**:
  1. **`fwidth` is not `fwidth`.** Its precision is implementation-chosen in all three languages:
     naga emits `OpFwidth` for Vulkan and HLSL `fwidth` for DirectX 12, and on this NVIDIA driver
     the second is quad-**coarse**. The voxel grid uses the derivative only for its 3.5 % band's
     smoothing width, so **DX12 alone** read 99.0124 % against Vulkan's 99.9795 %. Asking for
     `fwidthFine` by name takes both to 99.983 %. Vulkan alone would have shipped it.
  2. **`glPolygonOffset`'s two arguments cross over into `DepthBiasState`.** GL offsets by
     `m × factor + r × units`, so GL's *first* argument is wgpu's `slope_scale` and its *second* is
     wgpu's `constant`. Written the obvious way round the slope term is sixteen times too large and
     a steeply-seen face wins `LessEqual` against what is in front of it: the floor mask of
     REQ-SCN-084 marked the top of the cube and 1 709 pixels came out reflected. The bug is in a
     pipeline field, not in any WGSL.
  3. **Two row indices had to be mirrored, not transcribed.** The MSVO chain reads a row index
     twice — which texel of each 2 × 2 block the decimation keeps, and which pixel coordinate the
     per-pixel rotation hashes — and OpenGL counts rows from the bottom where this API counts from
     the top. Either choice is admissible (the requirement fixes neither), but transcribing builds
     the whole pyramid on a half-texel shift.
- **And two gaps that only the absence of execution had kept alive**: the backdrop model of AMD-049
  REQ-SCN-075 had no WGSL counterpart at all — **on by default on the desktop** — and the two
  backends agreed on only **58.930 %** of the `backdrop` capture; and the golden-test defect above.
  Hence Q-104 §6.3's answer to its own question 3: **the second backend's parity is a requirement
  of every lot**, as long as a development machine has a driver.
- What `wgpu` needed that OpenGL did not, in full: `preprocess.rs`, a `#ifdef` for WGSL sources so
  that a variant is a marker and not a branch on a frame field; **one render pass per screen-effect
  step**, where the OpenGL path rebinds a framebuffer inside one; and **a depth-resolve pass of its
  own**, because `RenderPassDepthStencilAttachment` has no `resolve_target` and there is no
  texture-to-texture blit for a depth format, so sample 0 goes through a full-screen triangle into
  `@builtin(frag_depth)` (which is what the OpenGL blit gives, and what the floor mask's bias is
  there for). That pass is counted, so `screen_effect_passes` reads one higher under anti-aliasing.
- No format is new: `R8Unorm`, `Rgba16Float` and `R16Float` are the three the OpenGL module already
  depends on. The one refusal REQ-SCN-083 names — the `R16Float` pyramid, the fallback **(D)** — is
  asked of the **adapter** in `WgpuSceneRenderer::new` rather than caught, because `wgpu` reports a
  refused allocation through the device's error scope and hands back a usable handle. Every adapter
  WebGPU admits grants it, so the fallback is reached through the same test seam its OpenGL twin
  has.
- **A Classic `wgpu` frame does no new work**, measured: `style_programs`, `screen_effect_bytes`,
  `screen_effect_passes` and `occlusion_levels` all 0 in Classic *and* in Enhanced with the four
  switches at the defaults of REQ-SCN-079, the same picture to the pixel, and **ten pipeline
  bindings** either way on both APIs. What it does carry from the lot: **80 bytes** added to a
  `write_buffer` it was already making once a frame (64 for the curvature, 16 for the backdrop's
  eye), four `is_some` tests on `Option`s that are `None`, one `color_mode == 2` comparison per
  opaque run whose two arms are the same value, and one loop over an empty `floor_runs`.
- **The OpenGL backend did not move**: its renders are **byte-identical to `faced2a`** — eleven
  captures at each of the three anti-aliasing levels, and eleven captures plus `render.json` for
  each of six Enhanced, lit and bent configurations — and the four instruction counts are
  **2231 / 1095 / 3401 / 159** throughout. The lot's whole diff in an OpenGL file is one word
  (`projection_terms` is `pub(crate)`, so the four projection terms the screen passes read are not
  written twice) and three comments that said the style was not rendered on the other backend.

**What the parity costs and what it reads** (RU-7; `--bench 60 --anti-alias 0`, median of 60
frames, `framing` at 1920 × 1080, one session, RTX 3050 — **each backend against its own Classic
baseline**, which is the only way the columns mean anything)

| | OpenGL | Vulkan | DX12 | passes | target bytes |
|---|---|---|---|---|---|
| **Classic baseline** | **0.549 ms** | **0.519 ms** | **0.780 ms** | — | 0 |
| voxel grid `N` = 4 | +0.06 | +0.07 | +0.07 | 0 | 0 |
| pixel shadows | +0.02 | +0.14 | +0.14 | 0 | 0 |
| occlusion Depth, Medium | +0.42 | +0.58 | +0.51 | 3 | 1 036 800 |
| occlusion Depth, High | +1.13 | +1.59 | +1.45 | 3 | 4 147 200 |
| occlusion **MSVO**, Medium | +0.61 | +0.82 | +0.98 | 20 | 3 434 160 |
| occlusion **MSVO**, High | +2.16 | +2.63 | +2.67 | 20 | 13 737 600 |
| all four effects on | +0.41 | +0.77 | +0.63 | 6 | 7 257 600 |

- The reflections do not appear there and RU-5 recorded why: on `framing` at the default camera no
  pixel is marked, so the trace returns at its mask test. On **`heavy` at 45° of elevation**,
  against baselines of 1.119 / 1.270 / 1.635 ms: `ssr.quality` High **+0.85 / +0.85 / +1.01 ms**,
  Ultra **+1.04 / +0.86 / +1.36 ms**. And the backdrop model of AMD-049, on `backdrop` against
  0.251 / 0.289 / 0.475 ms: **+0.050 / +0.006 / +0.032 ms** — the cheapest effect of the lot
  anywhere.
- **The pass and byte columns are the same numbers on all three backends**, which is the stronger
  statement than the milliseconds: the three renderers submit the same passes and allocate the same
  targets, so the time differences are the drivers' and not the ports'. The screen passes run 15 to
  40 % slower on `wgpu`, and the cause is the architecture rather than the shaders — one render
  pass per step, plus the depth resolve.
- DX12's Classic baseline is 50 % above Vulkan's **on this machine** (0.780 against 0.519) and its
  deltas are then comparable; that is this driver's command-submission cost and has nothing to do
  with any amendment. RU-5's and RU-6's tables were measured on their own machines and are
  comparable in *shape* — the ordering of the rungs, the MSVO-to-Depth ratio, the memory, the pass
  counts, all reproduced — and not in milliseconds.
- **Agreement, per effect** (640 × 480, worst scene, share of pixels within 2 sRGB levels of the
  OpenGL capture). The Classic control reads **99.6423 %** on `heavy` — 1 926 overlapping tile
  boxes, so silhouette edge everywhere — and that figure, not 100 %, is what every row is read
  against:

  | | within 2 levels |
  |---|---|
  | Classic control | 99.6423 % |
  | curvature (three configurations) | 99.59 to 99.87 % |
  | voxel grid, pixel shadows, occlusion Depth and MSVO | **the control's own figure** |
  | reflections, `ssr.quality` High / Ultra | 99.5999 % / 99.8493 % |
  | **backdrop lighting** | **99.9990 %**, from 58.930 % |
  | `video.anti_alias` 1 / 2 | 99.1084 % / 98.9489 % |

- The two multisampled rows are **MSAA sample positions**, which are implementation-defined in all
  three APIs: under MSAA 4× on `heavy`, Vulkan and DX12 agree with each other on **100.0000 %** of
  the pixels with max channel **0**, and both differ from OpenGL by exactly the same amount. There
  is nothing in the renderer to choose between the two patterns, and more samples let them diverge
  further.
- The two `wgpu` APIs are byte-identical to each other everywhere **except** the reflections
  (99.9993 %, two or three pixels of the march) and the MSVO contact ratio (0.1430 against 0.1348,
  the minimum sitting one pixel over). Both are the same cause: an estimator that is discontinuous
  in the depth buffer, read at its minimum.
- **The three invariants of AMD-055 §2.4 hold on this backend**, in both estimator modes: a plane
  alone at **0.9827** (Depth) and **0.99565** (MSVO) against the 1,00 ± 0,02 the requirement allows
  — 33 of 144 094 pixels darkened by at most 1.73 %, all on the plane's own silhouette, because the
  two backends compute the same NDC through two expressions that differ in the last bits of `f32`
  — the contact at 0.4337 / 0.1430, and the five `ao.strength` rungs strictly decreasing from 1 to
  4 and flat from 4 to 5. MSVO is the better-behaved of the two on the first invariant, because
  four `SmartBlur` passes and three bilateral upsamples turn a boundary case into a fraction of a
  level.

**The depth-detecting SMAA costs a third of MSAA 4×'s memory and one sample of geometry, and it is
not the cheap option in milliseconds** (SM-1, AMD-060 REQ-SCN-085 / REQ-SCN-086)
- `video.anti_alias` grows a fourth rung: the scene is rendered **single-sample** and the three
  passes of SMAA 1x — depth edge detection, blending weights through the reference's two
  precomputed tables, neighbourhood blending — run over it in linear space, after the screen
  passes of REQ-SCN-080 and the transparent objects and **before the HUD**. Both backends.
- **Nothing of it exists at the other three levels** (REQ-SCN-086), and the whole cost of the mode
  to such a frame is the `is_none` test that decides not to run it: `SceneRenderer::render_scene`
  goes from 3401 instructions at `1a1086c` to **3409** (+0.24 %, the cause identified and
  documented — one store in `begin_pass` and one `unwrap_or` in `resolve_to`), and the synthetic
  set is **byte-identical to `1a1086c` at levels 0, 1 and 2 on OpenGL, Vulkan and DX12** —
  eleven PNGs *and* `render.json`, over seven configurations including three Enhanced ones and a
  bent one.
- **What it costs when it is on** (`--bench 60`, median of 60 frames, 1920 × 1080, one session,
  RTX 3050, each backend against its own level-0 baseline):

  | `framing` | OpenGL | Vulkan | DX12 | extra bytes |
  |---|---|---|---|---|
  | **level 0, baseline** | **0.539 ms** | **0.605 ms** | **0.798 ms** | 0 |
  | level 1, MSAA 4× | +0.221 | +0.202 | +0.174 | 99.5 MB |
  | level 2, MSAA 8× | +0.423 | +0.395 | +0.433 | 199.1 MB |
  | **level 3, SMAA** | **+0.382** | **+0.404** | **+0.620** | **29.0 MB** |

  and on `heavy`, against 1.140 / 1.255 / 1.616 ms: MSAA 4× **+0.215 / +0.218 / +0.223**, MSAA 8×
  **+0.421 / +0.403 / +0.397**, SMAA **+0.241 / +0.351 / +0.474**.
- Read plainly: **in time the mode sits between MSAA 4× and MSAA 8× on OpenGL and Vulkan and above
  8× on DX12**, and three full-screen passes at 1080p are what they are. **In memory it is a third
  of MSAA 4× and a seventh of MSAA 8×** — 29 030 400 bytes of targets (`RG8` + `RGBA8` + `RGBA16F`
  at the frame's size, 14 bytes a pixel) against 99.5 and 199.1 MB of extra attachments — **and it
  rasterises the scene once** where 4× and 8× shade every covered sample of every silhouette
  pixel. On the handhelds of §4, whose limit is bandwidth and fill rate, those are the two terms
  that decide; this desktop table cannot show them.
- The filter's own cost follows the frame's **contour count**, not its complexity: it is *cheaper*
  on `heavy` (+0.241) than on `framing` (+0.382) although `heavy` is the slower frame, because
  `framing`'s regular grid of boxes at a shallow angle marks more pixels. One scene's figure is not
  the mode's price.
- **The two backends submit the same three passes, allocate the same 4 300 800 bytes and build the
  same three pipelines** at 640 × 480 — an equality, not an allowance, because REQ-SCN-085 renders
  single-sample and there is nothing for the depth resolve of REQ-SCN-080 to do. The agreement is
  the control's own: on `heavy` the filtered pictures differ on 0.3776 % of the pixels where the
  unfiltered ones already differ on 0.3577 %, and the `hud` scene is **byte-identical** at level 3
  on all three backends.
- **A mirrored axis is safe through multiply-adds and unsafe through comparisons**, and that is the
  lesson of the lot. `SMAA_RT_METRICS.y` is negative on OpenGL so that the reference's own
  `± offset · metrics.y` arithmetic means "north is up on screen" in a texture whose rows run
  upward. It works for every offset and it does **not** work for the two vertical searches of the
  second pass, whose bound flips with the sign while the operator `>` between them does not: with
  the reference's condition those loops exited on the first test and a vertical contour was blended
  over one pixel instead of over the length of its run. Both are now written
  `(texcoord.y − end) · metrics.y > 0`, identically in the GLSL and the WGSL.
  - It was found by **putting the same algorithm beside itself with the other sign** — the `wgpu`
    port, where the reference's condition is right — and by nothing else: the filter passed every
    static assertion, both acceptance criteria of AMD-060, the whole lifecycle, and the usual
    "99.5 % of pixels within 2 levels" parity bar at **99.6458 %**. What exposed it was comparing
    the level-3 disagreement with the **level-0 control's**: 0.4271 % of a frame whose unfiltered
    captures were identical. That comparison is now the bar the parity test asserts.
  - RU-7's conclusion was "a device finds what reading cannot". This is its sequel: **a second
    backend finds what one device cannot.**

**The curved Zapper aim runs on a change, not per frame** (RU-4a, AMD-043 REQ-INP-124)
- REQ-INP-061 refreshes the Zapper once per displayed frame and must keep doing so, so the march
  of the inverse is guarded by a memo of its inputs (`AimInputs`: the pointer, the target size,
  the aim plane, the scene scale, the curvature, the camera view). A frame on which none of them
  moved costs **one comparison**.
- At curvature 0 `ZapperState::update` is the two lines it was before the amendment — one
  ray-plane intersection — and the memo is cleared rather than consulted.

### 5.5 Application loop, threads and pacing (`zax-app`)

C# counterpart: `src/Zax.App/Loop` (frame clock, emulation host, game session).

**Emulation on a separate core** (handheld only; `6b5ad4f`; `ZAXNES_SEPARATE_CORE=0/1`)
- `core_worker.rs` runs each emulated frame on a "zaxnes-emulation" thread. The core is moved there
  (never shared; the session holds a ROM-less stand-in) and taken back at the start of the next
  tick, before anything reads or changes it.
- Order per tick: finish frame N (tile capture, conversion, frame hooks), hotkeys and commands,
  prepare N+1 (input hook, input word), battery / audio / settings / publication, then launch N+1
  while the main thread draws N.
- Scripts and core see each other exactly as on one thread (`tests/separate_core_tests.rs`). The
  frame is shown one displayed frame later (input latency + 1 frame), which is why it is a
  platform choice.

**Emulation ahead** (with the separate core, automatic; `e312c87`, `22ec35a`;
`ZAXNES_CORE_AHEAD=0`)
- When no script of the profile names a member that reads or drives the core (`CORE_MEMBERS` in
  `game_session.rs`: `InputProcess`, `WriteMem`, `GetInput`, `GetInputType`, `SetInputType`,
  `ZapperEnable`), frame N+1 runs while frame N is converted and its hooks run.
- `ReadMem` is allowed: the console RAM is copied when the frame comes back (`MemorySnapshot`) and
  scripts read that copy. A read of cartridge space turns ahead off for the session (logged).
- 95 of the 101 3dSen profiles qualify. Same frames as serial (tests).
- Invariant: **every new Lua API member that reads or writes the core must be added to
  `CORE_MEMBERS`**, or the scripts will see the next frame's state. This matters for 3dSen 2.0's
  new API.
- The flag `pipeline.ahead_allowed` it sets also gates `pause_3d_in_2d`, `catch_up_last_3d` and
  `half_rate_3d` (below).
- **Not to be confused with run-ahead** (next entry). "Emulation ahead" is pipelining and changes
  no latency; AMD-041's run-ahead re-simulates frames to cut input latency. They compose.

**Run-ahead** (setting `emu.run_ahead`: 0 to 3, **desktop 1**, handheld 0; AMD-041 REQ-LOOP-064
to 068; RU-3b)
- The displayed frame is N frames in the future of the **real** frame, computed with the current
  inputs; the real state then comes back. The player sees their pad N frames earlier. A real frame
  therefore emulates **N + 1** frames: measured 2.02x at N = 1, 3.04x at N = 2 and 4.09x at N = 3
  of the core's time on `nestest`.
- The sequence of REQ-LOOP-065 lives in **`NesCore::run_frame_ahead`**, not in the session,
  because the "emulation on a separate core" thread *moves* the core for the duration of one frame
  (`core_worker.rs`): the sequence is exactly the unit that thread already ran. `CoreWorker::start`
  takes N as a third argument.
- **N = 0 costs nothing, and that is a rule, not an aspiration.** `run_frame_ahead` is
  `if ahead == 0 { return self.run_frame(word) }` with the sequence `#[inline(never)]` behind it.
  Verified on the post-LTO assembly against RU-3a-fix (`ce78abd`): `NesBus::tick` **119
  instructions / 3 `pushq` / 32-byte frame, identical**, and `NesCore::run_frame` **2 096,
  identical**. The only change anywhere hot is +18 static instructions in
  `Apu2A03::run_cpu_cycles`, three inlined copies of the mute test, each **behind the ring's
  mutex** — so once per output sample (~735 per frame), not per CPU cycle: about 0.008 % of a
  frame. A lot that touches this must keep those three figures.
- The in-memory state of REQ-CORE-121 (`zax_nes::state::CoreState`) is a **reused** buffer, 19.5
  KiB, written by the savestate's own per-component code. `Ppu2C02::save_state` is split into
  `save_machine_state` + `save_frame_records` and the in-memory state takes the machine half only:
  the frame's tile records are cleared by `begin_rendered_frame` at every frame's pre-render line,
  so the emulation never reads them back. Save + restore measured at **0.9 to 1.0 µs per real
  frame** (0.11 % of one emulated frame); keeping the records out is most of why.
- That split is also what makes REQ-LOOP-067 free: after the restore the core still holds the
  **displayed** frame's records, masks and pixels, so `frame_batch()`, the 2D picture and the
  Zapper answer for the displayed frame with nothing copied. The restore (step 5) is **delayed**
  past the conversion and the frame hooks, which REQ-LOOP-067 allows in so many words.
- **Invariant**: every entry point of the session that runs a frame leaves the core in the
  **real** state (`GameSession::put_the_real_frame_back`, four sites). That is what makes
  REQ-LOOP-066 true by construction for the rollback snapshots, the frame steps, the savestates,
  the input recording and the achievements. A lot that adds a frame-running path must add the
  restore to it.
- Frame stepping (J / K) forces N = 0, so it shows the frame it steps to.
- The pixels of a hidden frame **are** rendered. Skipping them was measured at 4.8 % of an
  emulated frame on `nestest`, and the only zero-cost way to get it duplicates the five hottest
  PPU loops through a `const` generic — the wrong trade against the ±0.1 % gate at N = 0 and
  against a Cortex-A53's instruction cache, on handhelds that default to N = 0 anyway.
  REQ-LOOP-068 only permits the omission; the Zapper is unaffected either way, its light being
  composed from the tile records (`sees_light_at`) and not from the pixel buffer.
- The harness runs at **N = 0** unless `--run-ahead` says otherwise (`render`, `bench-core`), and
  `golden-core` has no option at all: the digests and captures are only comparable at N equal
  (REQ-ARCH-012 amended). `bench-core` reports N in its header and the restore as its own stage.
- Q-103, answered by R0 after this lot and implemented in RU-3b-fix: a frame hook's
  `Script:WriteMem` writes into the displayed state and is lost, and the write is not replayed, so
  a profile whose scripts name a member that **changes** the core (`WriteMem`, `SetInputType`; the
  read-only ones and `InputProcess`, which runs before the real frame, are not concerned) runs at
  an **effective** N of 0, the user's setting kept for the next game. One text scan of the sources
  at profile load (`scripts_change_the_core`, beside `scripts_leave_the_core_alone`) and one
  `bool` test in `GameSession::apply_run_ahead_setting`, where the loop already read the setting:
  nothing in the core, the conversion, the draw list or the frame hooks. `set_run_ahead` stays the
  raw setter, so the frame steps and the harness are untouched.

**Pipelined frames** (setting `emu.pipeline`: Off / Auto / Always; desktop Off, handheld Auto;
`ZAXNES_PIPELINE`; `8bc7374`)
- With emulation ahead, the conversion of a frame (Lua hooks, conversion, draw list) runs on a third
  thread while the main thread draws the previous frame's list with a copy of the scene. The UI is
  built before the hand-off.
- Auto (`pipeline.rs`, `PipelineGovernor`): on after 3 frames over 85 % of the budget, off after 2 s
  where the direct path fits in 60 %.
- The Lua runtime changes thread through a documented `unsafe impl Send`
  (`zax-scripting/src/runtime/handoff.rs`); mlua's `send` feature was measured at +16 % Lua time and
  not used. Requires `emu.lua_optimized`.

**Catch-up limit** (setting `emu.catch_up`: None, 1-5; **default 2 on the desktop, 5 on a
handheld** since RU-3a; `9fc03f5`, `893986e`; `--catch-up`, `ZAXNES_CATCH_UP`)
- REQ-LOOP-012 ran up to 5 emulated frames per displayed frame; **AMD-052 REQ-LOOP-074 takes the
  spec's limit to 2**. History on the handheld: None (game slows instead of collapsing), then 2
  (worse: the second frame waits for the emulation thread in series), then back to 5 once the core
  was on its own thread (heavy scenes then keep game speed by showing fewer frames). The owner
  does not want 0 as a default.
- **RU-3a** therefore splits the two figures: `MAX_BACKLOG` = 2 is the spec's limit and the pacer's
  default, `MAX_CATCH_UP` = 5 is the largest the setting offers, and the handheld default stays 5
  (`ui/handheld/defaults.rs`). A lot that touches the pacer must keep them apart: lowering the
  handheld to 2 would undo the measurement above.

**Clock reset on a speed change** (AMD-052 REQ-LOOP-075; RU-3a)
- A change of `emu.game_speed` resets the cadence clock; arming, expiring and re-arming a
  fast-forward burst (about six state changes a second) must **not**. `FramePacer::clock_resets`
  counts the resets, which is what the criterion measures.

**GPU frame queue** (setting `video.max_queued_frames`: 1 or 2, default 2; AMD-052 REQ-LOOP-077;
RU-3a)
- `zax_render::sync::GpuFrameQueue`: a `glFenceSync` after the last draw of a frame, waited on
  before the first draw of the next, so that at depth 1 the GPU holds no finished frame.
  `Running::wait_for_the_gpu_queue` / `close_the_gpu_queue` place it after the tick's emulated
  frames and after the last draw, so it cannot move the emulated cadence.
- **Handheld (D), owner decision**: at the default depth of 2 a handheld issues **no fence call at
  all** and keeps its current behaviour — that depth is the driver's own queue, and the Mali keeps
  three frames of its own. Depth 1 works everywhere. A lot that touches this must not start
  fencing handheld frames at depth 2.

**3D paused in 2D** (setting `emu.pause_3d_in_2d`, desktop off, handheld on; `246bab8`)
- While the 2D picture is shown, a profile whose scripts leave the core alone is neither converted
  nor run through its frame hooks; its input hook still runs. The last skipped frame
  (`skipped_batch`) is converted when the 3D comes back, before it is drawn.
- Changes script timing (hooks do not run in 2D), hence a setting. Measured: SMB3 widescreen in 2D
  main thread 19.8 ms (Lua 6.3, conversion 4.0, collect 2.65) went to near nothing.

**Catch-up converts the shown frame only** (setting `emu.catch_up_last_3d`, desktop off, handheld
on; `d9e5680`)
- When a displayed frame runs several emulated frames, those finished before the last round are
  neither converted nor run through the frame hooks (scripts leaving the core alone only); the input
  hook runs every frame.
- Breaks the catch-up spiral seen in a Zelda II temple: 5 emulated frames per displayed frame, each
  about 11 ms core + 8 ms conversion + 9 ms Lua (`pivot.UpdateS`, about 400 calls per frame), 8 fps.

**3D at 30 fps** (setting `video.half_rate_3d`, handheld options only, off by default; `31c7485`,
`3152ece`)
- While a running game shows its 3D, the window swaps every other refresh and a catch-up converts
  only its shown frame; the 2D picture and the paused menus keep every refresh
  (`EmulationHost::half_rate_3d`, `VideoPacing::half_rate_3d`). Hidden from the desktop options
  window (`ui/options.rs` `EXCLUDED`).
- Owner is not convinced of its value: kept off by default.

**Curvature of space** (settings `scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`,
`scene.arc_center`, `scene.bend_2d_mode`; AMD-043 REQ-SCN-066 and AMD-046 REQ-SCN-074; RU-4a)
- The four angles are **0 on both platforms**, which is their own default, and `bend_2d_mode` is
  on both. Nothing is overridden for a handheld, because at 0 there is nothing to turn off: no
  program is compiled, no surface is subdivided, and the per-frame cost is one `is_identity()`
  test (see 5.4). That is also why the five are offered among the handheld's advanced options.
- The five are read **when they change** — `scene_script_bridge::bend_of` in `apply_setting` and
  at a profile load — never per frame, per tile or per pixel: `SpaceBend::new` evaluates three
  sines, three cosines and three divisions, which belongs nowhere near a frame path.

**No rewind on a handheld**: `EmulationHost::rollback_enabled` false with `--handheld` (the
snapshot every 60 frames caused a spike).

**Handheld options page "Optimizations"** (`3152ece`): frame cap, 3D at 30 fps, catch-up, catch-up
last 3D, pause 3D in 2D, parallel scripts, optimised and experimental Lua.

### 5.6 Build and packaging

- Release profile: `lto = "fat"`, `codegen-units = 1`, `strip = true` (`9b89c15`).
- Windows: `+crt-static` (standalone exe).
- aarch64: `-C target-cpu=cortex-a53` (`13f8a08`; `ZAXNES_TARGET_CPU` reported by the build
  script); golden-core under qemu-aarch64 is identical to the x86 goldens.
- PortMaster launcher sets the `performance` CPU governor for the run and restores it.

### 5.8 Lot MF-5a: two boards added, and the whole set came out faster

The lot that added the VRC6 (mappers 24, 26) and the Namco 163 (19, 210) is the first to add
boards *and* end below its own baseline on every ROM of PR-1's set. Four findings are reusable and
none of them is about the boards:

**A per-cycle `expansion_audio_output` is a recomputation, not a read.** `Mmc5Audio::output`
summed two channel outputs through two tables every time the bus asked for `M`, which is every CPU
cycle of an MMC5 — **178.7 M instructions over 300 frames of `mmc5exram`, about twenty per cycle**,
for a value that moves only when a duty sequencer advances, an envelope or length counter is
clocked, or a register is written. Caching it in a `level` field refreshed at those four points is
what gave the MMC5 back the `on_cpu_cycle` latch's two instructions and 44 M more. Every
expansion-audio board in the build now has that shape (`Vrc6Audio` had it from the start,
`Namco163Audio` was written with it). **A board whose `M` is computed on demand should be assumed
to be paying twenty instructions a cycle until measured.**

**Shrink-wrapping is a property worth checking by name.** `Ppu2C02::read`'s +11 instructions per
register read (Q-125) was neither a new call site nor a bigger function — `read` was *smaller* —
but LLVM had stopped shrink-wrapping: the five `pushq` the `$2007` arm needs had moved from inside
that arm to the head of the block every register read arrives in. `cmp4c.py` and `prologues.py`
both report the entry prologue and neither sees it, because the prologue had moved *into* a basic
block. `target-mf/asm/blocks.py` (new) prints a body with its `.LBB` labels kept, which is what
made it visible in one look. **Taking the one arm that calls anything out of line
(`#[inline(never)]`, a tail call) is the fix, and it is worth trying whenever a dispatch's arms
have unequal register needs.**

**Argument promotion is transitive through `noinline`.** The Namco 163 is the first
`Mapper::on_cpu_cycle` arm that reads `CartridgeMemory` (its chip RAM is PRG-RAM), and LLVM
promoted the `&mut CartridgeMemory` argument to the (pointer, length) it uses, propagated the new
convention up through `Board::on_cpu_cycle` — both are internal — and hoisted two loads into
`NesBus::tick`. An extra `#[inline(never)]` boundary between the arm and the audio was built and
**changed nothing**: promotion runs bottom-up and simply promoted the new function too. What stops
it is a *store* through the pointer. The two loads sit after `Cartridge::clocks_on_cpu_cycle`, so
the twenty-six unlatched mapper numbers pay nothing and the twelve latched ones pay two
instructions per CPU cycle.

**MF-3b rule 3 cost two attempts in one step, on a new path.** One `refresh_level()` call inside
`Mmc5Audio::feed_pcm_read` is a call inside an arm of `Board::cpu_read_override`, which LLVM
inlines into `Cartridge::try_cpu_read` — entered on every cartridge-space read of every game. The
byte read must stay live across the call, so that dispatch's prologue grew from `pushq %rax` to
three pushes: **exactly +524 336 instructions on each of `M0_P32K_C8K_V`, `M1_P128K` and
`M4_P128K`**, none of them an MMC5, and the three figures being equal to the instruction is what
identified it as a per-call entry cost. Out-of-lining the arm moved the same two instructions into
argument shuffling instead of removing them. **The fix was to have nothing to call**: the squares'
half of `M` and the PCM's half share no input, so they are two cached fields and the PCM refresh
is a load, an add and a store.

Numbers, callgrind, thirteen ROMs, 300 frames, against MF-4d's end `ad1ad765`: twelve of thirteen
lower by −1.4 % to −2.9 %, `mmc5` −0.621 %, total **−1.226 %**; and against the branch tip the lot
started from, **all thirteen lower**, total −0.159 %. `duelito`'s +2.78 % against `ad1ad765` is
MF-4d-fix's behaviour change, not code.

### 5.9 Lot MF-5b: the thirty-second `Board` variant, and the chain nobody had pinned

The lot that added the VRC7 (mapper 85) and its FM sound ended below its baseline on all thirteen
ROMs of PR-1's set, total **−0.239 %** — but it got there by first measuring a **+0.542 %
regression on every one of them**, caused by twenty-two lines of bank switching. Three findings
are reusable and none of them is about the board.

**Pin the whole chain of a per-cycle question, not its last link.** `Cpu6502::end_cycle` samples
/IRQ once per CPU cycle through `NesBus::irq_asserted` → `Cartridge::irq_asserted` →
`Board::irq_asserted`, a thirty-two-arm dispatch of which twenty arms only answer "no". MF-3b
step 1 pinned `end_cycle` with `#[inline(always)]` so the *sampling* would stay inlined, and
nothing pinned the two links below it. Adding the thirty-second arm grew the callee past the
inliner's budget, so LLVM outlined **`NesBus::irq_asserted`** instead and called it every cycle:
`0 → 169 739 749` instructions over 300 frames, about **19 per CPU cycle, on twelve ROMs of
thirteen whose board cannot assert an IRQ at all**. Both links now carry the attribute with the
figure on them. This is the third time in these lots that one more enum variant has tipped an
inliner budget on a per-cycle path (MF-3b step 1, MF-5a step 1's `Board::mapper_number`, here).

**And widen the instrument when it misses one.** `callsites.py` reported `Board::irq_asserted`
**0 → 0** throughout, because MF-3b rule 1's watched list held only the last link. It now watches
`NesBus::irq_asserted` and `Cartridge::irq_asserted` too. That is the second time the watched list
has been blind to a real outlining; both times the fix was to widen it rather than to trust it.

**The load-time latch for `irq_asserted` was built and refuted, and the numbers are the record.**
MF-3b step 1 costed a `Cartridge::irq_capable` latch on paper and declined it; this lot built it —
the latch plus `#[inline(never)]` on the dispatch — because the paper argument could not say how
big either side was. A board that cannot assert an IRQ stops paying the dispatch entirely (`m1`
**−1.526 %**, `metroid` −1.428 %, `m0` −1.300 %, `z2` −1.069 %) and a board that can pays a
`callq` per CPU cycle instead of an inlined bitset (`m4` **+1.805 %**, `mmc5` +1.449 %, `smb3`
+1.340 %). Three of the thirteen, two of them MMC3s, so it is out — and nobody has to build it
again.

**A savestate round-trip test can catch an audio design error.** `Apu2A03::load_state` re-derives
its cached mixer level with `M` forced to 0, deliberately and with its reason on the line. So an
expansion chip whose `M` is non-zero **while silent** makes
`req_core_090_the_mapper_state_survives_a_save_and_restore` fail on byte identity, which is how
the VRC7's first mixer level was caught: six bipolar FM carriers need a DC offset for `M` to stay
positive (AMD-061 §6.2), and a *standing* offset of 14 670 showed up as three bytes of an `f64` at
−0.4476. Carrying the offset **per sounding channel** fixes it and is better sound: a chip with
nothing playing has an `M` of exactly 0, like the other four expansion chips. The asymmetry it
exposed is general — a state saved while any expansion chip is *sounding* is not byte-identical on
an immediate re-save, on the MMC5, the VRC6, the Namco 163 and the 5B alike — and
`mf_5b_a_sounding_expansion_chip_is_not_byte_identical_on_any_board` records it on two boards so
nobody reads the silence requirement as a stronger invariant than it is.

**Scale a new sound chip's work with its own clock, not the CPU's.** The VRC7 runs at the board
clock divided by 36, so `Vrc7Audio::step_cpu_cycle` is a decrement, a test and a call to an
`#[inline(never)]` sample — the shape MF-5a wanted for the VRC6 and could not have, its three
oscillators being on the CPU clock itself. The whole chip costs the set nothing measurable: six
ROMs move up by 424 to 19 213 instructions against the board-only step, one to two orders of
magnitude inside the ~10⁵ whole-process floor.

### 5.11 Lot MF-6a: a `Board` variant is free, a `Board` arm *body* is not

Seventeen mapper numbers added in two `Board` variants, set total **−50 984 instructions** below
`240a2718` with every ROM of lot PR-1's thirteen inside the instrument's own noise. Four
findings, and the first one corrects the unit §4.9 of `docs/review/MF-6-inventory.md` costed in.

**A new `Board` variant costs the existing boards nothing; a new *distinct arm body* costs
whatever inlines that dispatch.** The experiment is step 1's X: the 33rd variant, with its
struct, its `banked!()`, its `nametable_page`, its `reset` and its save-state block but **no
`write_register` of its own**, left `NesBus::write_internal` identical **to the instruction on
all thirteen ROMs**. Give the same variant a register decode and the two MMC3 ROMs pay +66 921
and +16 561 there, whatever attribute the arm carries — inlined, `#[inline(never)]` and
`#[cold] #[inline(never)]` give the *same delta to the instruction*. And §4.2's per-arm yardstick
(+11 in `Cpu6502::branch`, +47 in `resolve`) is **not** paid by a board that inherits
`irq_asserted`'s empty default: `branch` stayed at 418 and `resolve` at 1 854 through all five
measured trees, because twenty-one boards answering `false` share one arm and a twenty-second
joins it. The lesson for a board-adding lot: count bodies, not variants, and keep the new board's
methods either absent or identical to a body that already exists.

**`Ppu2C02::run_dots` is the function a board-adding lot has to defend, and `chr_view` is the
line that defends it.** Five boards as five variants cost **+3.3 M to +6.9 M over 300 frames on
eight of the thirteen** — 0.067 % to 0.131 % — with `run_dots` **not gaining one instruction**:
it was all spill and reload churn inside the loop, because `Board::chr_view`'s arms are inlined
into `Ppu2C02::run_visible_span` and that into `run_dots`. Three refutations came with it, and
all three are worth not repeating: **merging the five variants into one changes nothing** (one
variant forwarding `chr_view` to five boards costs the same as five variants, because the trait
default is monomorphised per board and five `banks` fields sit at five offsets);
**`#[inline(never)]` on `Board::chr_view` is far worse** (+0.297 % on the set, +0.849 % on
`Duelito`, because that dispatch is reached once per *span* and some ROMs have very many);
and what works is **answering `None`** — the body MMC2, MMC3, MMC5 and the RAMBO-1 already
answer — which costs the dozen games of those boards the span fast path and the rest of the
library nothing. Q-123's rule ("the trigger is the third distinct body, not its size") holds one
hook along, on `chr_view` as on `read_nametable`.

**`expansion_audio_output` was the next dispatch in the queue behind `irq_asserted`.** Five more
variants pushed it past the inliner's budget and LLVM gave it a symbol: **0 → 116 144 093
instructions over 300 frames of `mmc5exram`**, with `NesBus::push_expansion_audio` shedding
44 670 800 as its body left — net **+75 937 647, +0.672 %** on a board the lot does not touch.
`#[inline(always)]` on it, and the ROM ends at −0.000 %. That is MF-5b step 1's story one
dispatch along, so the list of pinned links is now five: `NesBus::irq_asserted`,
`Cartridge::irq_asserted`, `<Board as Mapper>::irq_asserted`, `Board::try_read_prg`,
`Board::cpu_read_override` and `Board::expansion_audio_output`.

**The whole-process callgrind count is not deterministic on this machine; the emulated symbols
are.** Three runs of the *same binary at the same path* spread by up to **57 097 instructions**
per ROM (0.0017 %), and `callgrind-diff.py` names all of it in one symbol: **`luaH_getshortstr`**,
Lua's short-string hash lookup, 85 722 → 187 789 between two runs, whose collision pattern
follows the heap addresses. `getgeneric`, `__memcpy_avx_unaligned_erms`, `__memcmp_avx2_movbe`
and `dragon::format_exact` move the same way. Every emulated-core symbol and every level-2 `core`
stage is **exactly** reproducible, so a bar of the form "every ROM at the baseline's count or
lower" has to be read on the **symbol table** — a 2 000-instruction move there is real where a
50 000-instruction move in the total may be none. The build path's *length* matters too: the same
commit built at two paths differs by up to 1.56 M on a level-2 `total`, all of it in
`load_profile` and `lua_hooks`, which is why MF-6a gave every build of the lot a label of exactly
fifteen characters.

### 5.10 Lot MF-5b-fix: the set is not the games, and `step` holds one inlining budget

MF-5b ended **−0.240 %** on all thirteen ROMs of lot PR-1's set and **+0.138 % on Zelda II** and
**+0.060 % on Super Mario Bros.** once each game was run with its 3D pack. Both numbers were right.
Four findings, none of them about a board.

**A 300-frame no-pack run is not the application.** PR-1's thirteen ROMs measure the emulated core
alone: no pack, so the widescreen extension's execution zone is off and `WideExtension::zone_offset`
answers `None` from a constant; no input, so the games sit on a title screen; 300 frames, so every
per-cycle cost is 2.4 × smaller than in R0's 600-frames-after-120 level-2 run. `rust/tools/perf/`
now carries the level-2 instrument as well (`roms-l2.txt`, `callgrind-l2.sh`, `l2-diff.py`), so
**a lot measures the games with their packs before R0 does**, and `<key>.ann`/`<key>.ir` are named
so `callgrind-diff.py` reads a level-2 directory unchanged.

**`Cpu6502::step` holds one inlining budget with three claimants**, and a lot that frees one of
them hands the room to another. The three are `<Board as Mapper>::irq_asserted` (the per-cycle
/IRQ dispatch), `WideExtension::zone_offset` (every instruction fetch, through
`NesBus::read_code`) and `Cpu6502::execute_without_operand` (the interpreter's out-of-line
boundary). At `a577f336` the cost model kept the first two and outlined the third; MF-5b's
thirty-second `Board` variant flipped it; **the budget fits all three only if all three are
pinned**, and that is what ships — `step` 932 → 2 529 → **3 443** instructions, with `read_cycle`
(181), `tick` (82), `read_internal` (175) and `run_dots` (1 906) unmoved to the instruction and
their call-site counts unchanged. Set total −0.376 % against `a577f336`, and all five games
0.143 % to 0.663 % below it, core and total.

**Pinning a dispatch and shortening it are different changes with opposite beneficiaries.** An
eleven-arm `irq_asserted` with the twenty-one boards that cannot assert /IRQ folded into one arm is
inlined by the cost model with no attribute at all, and makes `Cpu6502::branch` 330 instructions
(395 at `a577f336`) and `resolve` 1 552 (1 800). LLVM compiles it to a bit mask over the
discriminant plus a **table of field offsets**, which is three instructions cheaper for a board
that answers "no" and three dearer for one that answers: `metroid` **−1.107 %**, `m1` −0.745 %,
and `m4` **+0.698 %**, `smb3` +0.464 %, `mmc5` +0.278 %. That is MF-5b's refuted load-time latch
in a new disguise, with the same verdict — the thirty-two-arm `dispatch!` is what the boards with
an IRQ want — and `docs/progress/MF-5b-fix.md` step 3 has the form written out so nobody builds it
a third time. The same step refutes a `zone_on()` guard in front of `zone_offset`, in both
positions, and outlining the zone-hit body.

**A per-cycle question's cost depends on the APU, not only on the board.** `NesBus::irq_asserted`
is `apu.irq_asserted() || cartridge…`, so a game that leaves the APU's frame IRQ asserted never
reaches the cartridge at all. Over 300 frames the dispatch is entered **37.9 M times on Metroid
and 51 324 times on Mega Man 2 — both MMC1 games, same binary**. Four of PR-1's thirteen
(`duelito`, `oamstr`, `readbuf`, `mm2`) are in that regime: they pay for the dispatch's code,
never run it, and end **+0.046 % to +0.065 %** above `fcfa5cb3` while the other nine are 0.055 %
to 0.672 % below it. All thirteen are below `a577f336`. When reading a per-cycle figure, check
which half of the `||` answered.

### 5.7 Instrumentation

- Frame profiler (`frame_profiler.rs`, setting `ui.profiler`, `ZAXNES_PROFILE=1`): stages input,
  core, capture, conversion, Lua, rollback, host, list, collect, scene, wait, UI, audio, swap,
  sleep; busy time per thread; Lua time per script hook; start-up line with the render and pacing
  settings. Off, each stage clock is one branch.
- `bench-core` (`zax-harness`): same stages headless, max and frames over 1 ms per stage.
- File log `logs/zaxnes.log`, launcher `log.txt` (firmware, GPU, governor).

---

## 6. Settings and environment variables

Rust-only performance settings (none are exposed to Lua; the Lua API dump is unchanged) — with
one exception, `emu.run_ahead`, which is AMD-041's own setting and **is** exposed to Lua
(`Setting.RunAheadFrames`). Handheld values are environment defaults: they apply only while the
user has no saved value.

| Setting | Desktop default | Handheld default | Output identical? |
|---|---|---|---|
| `emu.run_ahead` (AMD-041) | **1** | **0** | no, and by design: the displayed frame is N frames on. Costs N extra emulated frames per real frame |
| `emu.catch_up` | 5 | 5 | n/a (C# behaviour is 5) |
| `emu.lua_smooth_gc` | on | on | yes (except `collectgarbage("count")`) |
| `emu.lua_optimized` | off | on | yes |
| `emu.lua_experimental` | off | off | intended yes (heuristic) |
| `emu.pipeline` | Off | Auto | yes, one frame of latency |
| `emu.pause_3d_in_2d` | off | on | no: hooks do not run in 2D |
| `emu.catch_up_last_3d` | off | on | no: hooks skip caught-up frames |
| `video.half_rate_3d` | off (hidden) | off | no: 30 fps 3D |
| `video.dynamic_resolution` | off | off | no |
| `render.backdrop_lighting` (AMD-049) | **on** (the spec default, REQ-SCN-075) | **off** (REQ-SCN-048's diffuse term, which the amendment's T-1 clause keeps valid) | no, on a profile whose script enables the backdrop: a zero albedo goes from black to sRGB 26 and the surface takes the light's tint. Free on a desktop GPU at 640 × 480 and 1280 × 720; a per-fragment cost on a large surface, hence the handheld default |
| `ui.profiler` | off | off | n/a |

Platform choices without a setting:

| Behaviour | Desktop | Handheld | Override |
|---|---|---|---|
| Grouped draws | off | on | `ZAXNES_GROUP_DRAWS=0/1` |
| Separate core thread | off | on | `ZAXNES_SEPARATE_CORE=0/1` |
| Emulation ahead | with separate core | with separate core | `ZAXNES_CORE_AHEAD=0` |
| Rewind snapshots | on | off | none |
| Linearised palette | on | on | `ZAXNES_SRGB_PALETTE=1` |

Other variables: `ZAXNES_LUA_GC` (harness collector), `ZAXNES_FAST_LUA`, `ZAXNES_EXPERIMENTAL_LUA`
(tests), `ZAXNES_WGPU_BATCHING`, `ZAXNES_GLES`, `ZAXNES_HANDHELD`, `ZAXNES_PROFILE`.

---

## 7. Attempts that were dropped, and why

Core:
- Removing the PRG/CHR `% len` in the board reads: no x86 gain.
- Restricting the cartridge cycle-hook dispatch: no gain.
- Drawing visible spans one tile (8 dots) at a time: identical output but 5-9 % slower.
- **Giving `NesBus::read_cycle` its own copy of the bus read** (`read_internal` split into an
  `#[inline(always)]` body plus two out-of-line callers), so that the DMA gate ends *in* the read
  instead of a `jmp` into it. Measured with callgrind, 300 frames: **−0.86 %** on `M1_P128K` and
  **−0.79 %** on `M0_P32K_C8K_V` — real, reproducible, and not shipped by lot MF-4a-bis (step 2),
  because only one of the saved instructions is the `jmp`: the rest was LLVM outlining
  `Ppu2C02::read` and a `Board` read out of the CPU read path to avoid duplicating them, which is
  MF-3b rule 1's stop-the-lot signature (a hot callee gaining its first call site) pointing the
  other way. Every `$2002` or `$2007` read then pays a prologue, and the two holy-mapperel ROMs
  that show the win barely read PPU registers, so neither could price it.

  **Resolved and shipped by lot PR-1** (`docs/progress/PR-1.md`), which measured it on thirteen
  ROMs spanning the register-read rate — from Mega Man 2's 30 PPU-register reads a frame to
  `Duelito`'s 4 024, about a fifth of its read cycles, with the rate measured per ROM rather than
  guessed. Three findings:

  1. **The `$2002` spinner cannot pay for it.** The split saves **5.90 instructions per CPU read
     cycle** and the outlined `Ppu2C02::read` costs **4.09 per PPU register read** (least squares
     over twelve ROMs, residuals under 0.05 % of a run). A register read *is* a read cycle, so it
     saves 5.90 before paying 4.09: net −1.81, and no mix of addresses can reverse that. `Duelito`
     gains 0.63 %, Zelda II 0.77 %, `oam_stress` 1.43 %.
  2. **The real cost was a board, not the PPU.** The split alone is +0.586 % on an MMC5, which maps
     PRG-RAM into `$8000`-`$DFFF` so that `Cartridge::read_rom_window` cannot answer and *every*
     read reaches `Board::cpu_read_override` — about one call per read cycle, against a few
     thousand in 8.8 million on an MMC1.
  3. **What shipped** is the split plus two things that make it hold: the rare arms of the body
     (`$4000`-`$401F`, and the widescreen window over `$5000`-`$7FFF`) moved into
     `#[cold] #[inline(never)]` helpers so the two copies stay small, and `#[inline(always)]` on
     `Board::cpu_read_override`. **Thirteen ROMs, thirteen negative: −0.63 % to −1.46 %, total
     −0.84 %**, the MMC5 going from the only loser to the largest gain (−0.73 %).

  Two traps PR-1 measured and that a later lot should not re-try: `#[inline(always)]` on
  `Ppu2C02::read` to "keep the register arm inline" is the **worst** of the nine variants
  (+0.02 % overall, +0.89 % on the MMC5 — its prologue then lands on every read cycle instead of
  on register reads alone), and the cold arms **without** the split are slower on eleven ROMs of
  thirteen. The two ideas are complementary, not alternative.

Lua:
- Lazy `shape2D` (made only when a script reads it, `763a0ff`): reverted. It needed a protected
  metatable on the globals, which scripts can see; C# puts none on `_G`.
- Typed answers instead of boxed ones for host reads: about 0 ns, the cost is the Lua round trip.
- A cache of setting reads: the model read is already cheap.
- Generational collector: majors of the whole heap, 120 ms stalls.
- mlua's `send` feature for the pipelined runtime: +16 % Lua time.

Renderer:
- Clip flag kept in the draw-state key for the solid shader: more runs (MM2 -7 %) and one changed
  pixel through the grouped order (replaced by the per-state unification).
- Front-to-back order of the opaque states: no gain on the RTX 3050; costs CPU for a sort.

Engine and loop:
- Disk cache of meshes: draw-list spikes are 1 frame in 4000 (SMB3, x86); not worth it.
- Catch-up 2 on the handheld: worse than both None and 5 once the core had its own thread.
- Reusing the draw list when nothing changed: not done, too many mutation sources (Lua, settings,
  meshes finishing in the background, state loads); a missed one would show a stale picture.

Measurement pitfall:
- Lua spikes of 35 ms seen in `bench-core` on the SMB3 world map came from the harness's original
  collector tuning; with `ZAXNES_LUA_GC=smooth` (the application's default) the worst is 0.67 ms.

---

## 8. Conclusions

1. **The A53 is not an x86.** An optimisation worth 1.4-2x on x86 gave 11 % on the device (batched
   PPU). Count instructions and cache misses; prefer work that removes memory traffic and branches.
   Memory-heavy stages (draw list, collection) are 30-60x slower than on x86, compute-heavy ones
   about 10-14x.
2. **The catch-up spiral is the first thing to break.** Once a frame overruns, running up to 5
   emulated frames per displayed frame makes everything worse. The fixes that mattered moved work
   off the critical path (core thread, emulation ahead, pipelining) or skipped work nobody sees
   (pause in 2D, convert only the shown frame).
3. **Draw calls were the GPU-side CPU cost.** About 17 us of CPU per draw on the Mali driver;
   instancing alone barely helped because the list interleaves meshes; grouping by state is what
   brought 1559 draws down to 72.
4. **Lua's cost is per call, not per instruction.** mlua's callback entry (about 1000
   instructions) dominated; raw userdata with C metamethods halved Lua time. GC tuning removed the
   worst stalls entirely.
5. **Identical output is achievable for most work.** Every core, engine and most renderer
   changes are bit-identical, verified by goldens and captures. Only grouping, threading latency
   and skipped hooks change output, and they are platform settings.
6. **Script-visible timing is the boundary.** Every trick that skips or reorders frame hooks is
   gated on `scripts_leave_the_core_alone` (the `CORE_MEMBERS` scan).
7. **Today the device is CPU-bound on the main thread** in heavy 3D scenes, not GPU-bound;
   lowering the render scale does not help there.

---

## 9. Open problems

- **Zelda II world map** (owner savestate, logs 21-22): 18 fps, main thread about 47 ms per
  displayed frame: draw list 12.6, GL submission 9.9, collection 7.1, Lua 5.2, conversion 2.7 ms.
  The same draw list costs 0.2 ms on x86 (2258 tiles; `DrawTile` is 432 bytes, of which 164 are an
  inline `Option<DeformParameters>`, so about 1 MB written per list). Unexplained ratio of 60;
  memory traffic is the main suspect. Leads: sub-stage timers inside list/collect/scene in the
  profiler; a smaller `DrawTile` (deformation out of line); reuse of the list when nothing changed
  (risky, section 7).
- **Paused game in 3D**: the list, collection and passes are still rebuilt every displayed frame
  (about 30 ms on the Zelda II map), although nothing changes.
- **Linearised palette** and **solid shader**: gains not measured on the Mali (no device access).
- **Segmentation reuse on scroll** (idea "piste 5"): order-dependent greedy segmentation, hard to
  keep identical; only as a setting with a debug comparison.

---

## 10. Porting checklist: C# areas and the Rust work they touch

| C# area changed by the update | Rust items to re-check | Checks to run |
|---|---|---|
| `Zax.Nes/Ppu` (rendering, registers, timing) | 5.1 batched PPU sync points, fast span (`run_visible_span*`), per-line sprite buffer, sprite limit; the `save_machine_state` / `save_frame_records` split of the savestate (5.5 run-ahead) — a new field goes in one of the two, never in both | PPU goldens, golden-core |
| `Zax.Nes/Apu` | 5.1 lazy level mixing (the non-linear tables of AMD-054 live there; the cache test stays **one** comparison of `level_key` and `current_level` keeps **one** call site), quiet-cycle skipping (`quiet_cycles` must account for any new timed state; a board with expansion audio keeps it off) | golden-core with audio digests |
| `Zax.Nes/Cartridges` (new or changed mapper) | 5.1 IRQ horizon or per-cycle stepping; `chr_view` (only for pure fetches); `rom_windows` (`NO_ROM_WINDOW` when reads have side effects); debug assert in debug build | golden-core, a debug run of the mapper's games |
| `Zax.Nes` bus / CPU | 5.1 sync points ($2000-$3FFF, $4014, $4016/7, cartridge writes), APU catch-up on $4000-$401F, and **`NesBus::tick`'s prologue in the post-LTO assembly** (since MF-4a-bis: 75 instructions, 2 `pushq`, 40-byte frame — it was 119 and 3 `pushq` while it inlined the DMC fetch; nothing board-specific and nothing of run-ahead may be inlined into it). A **read** cycle must stay two calls with the address materialised **once** — `callq tick` then `callq read_cycle` — and a per-cycle test added to `DmcChannel` or to `Apu2A03::quiet_cycles` costs about 10 instructions on every CPU cycle of every game (MF-4a-bis step 1) | goldens, callgrind (section 3) |
| `Zax.Engine3D` segmentation / frames | 5.2 filled-cell lists and germ order; `FastMap` iteration order never reaching output | render captures (identical) |
| `Zax.Engine3D` draw list / `DrawTile` fields | 5.2 placement matrix order; 5.4 `DrawStateKey` must include any new per-draw input; renderer `DrawItem` | captures in both draw modes |
| `Zax.Scripting` new API members | 5.3 both binding paths (safe and `raw.rs`), API dump; 5.5 **`CORE_MEMBERS`** if the member touches the core; `light_shapes` analysis if it touches `shape` | scripting tests with and without `ZAXNES_FAST_LUA=1`, API dump, `separate_core_tests` |
| `Zax.Scripting` hook order or new hooks | 5.5 emulation ahead, pipelining, `pause_3d_in_2d`, `catch_up_last_3d` assumptions on when hooks run | `separate_core_tests`, `pause_3d_tests` |
| `Zax.Render` shaders | 5.4 `ZAX_SOLID` (no `discard` outside the guard), **`ZAX_BEND`** (the curvature is a variant; at curvature 0 nothing of it may reach the compiler, and nothing of it may become a per-vertex uniform), **`ZAX_CENTROID`** (same rule for `video.anti_alias` = 0), **the backdrop program** (the model of REQ-SCN-075 may not reach a tile fragment, and the backdrop fragment may not gain an atlas lookup), linear palette (indexed albedo path), uniform cache, instance block layout, **the three SMAA passes of AMD-060** (they exist only at `video.anti_alias` = 3, and their `uRtMetrics.y` is negative) | captures, `bend_tests.rs`, `amd058_centroid_tests.rs`, `amd049_backdrop_tests.rs`, `amd060_smaa_tests.rs`, `ZAXNES_SRGB_PALETTE=1` comparison |
| `Zax.Render` **wgpu** shaders or a new effect | 5.4 **the second backend is not optional**: a new variant is a `//#ifdef` of `preprocess.rs` and never a branch on a frame field; a new screen pass takes a `Slot` of its own because `write_buffer` applies before the command buffers; anything that reads a **row index** or `gl_FragCoord.y` has to be mirrored into OpenGL's order, not transcribed; `DepthBiasState`'s two fields are the **reverse** of `glPolygonOffset`'s arguments; and where a mirrored `y` metric carries the convention (SM-1), **every comparison against a coordinate has to be multiplied through by it** — an offset flips with the sign, an inequality does not | `parity_wgpu_tests.rs` and `amd055_parity_tests.rs` (both figures *and* the effect's own "it moves something" row), `amd055_style_tests.rs`, and the §2.4 invariants on both backends |
| `Zax.Render` scene programs (a new variant axis) | 5.4 `ScenePrograms`: the new axis goes in the **slot table**, so `current()` stays two indexings and no branch; the axis is chosen **once a frame**, an unbuilt key falls back to the flat scene pair, and the handheld defaults must leave it unbuilt | `amd058_centroid_tests.rs`, `amd049_backdrop_setting_tests.rs`, a start-up with every default |
| `Zax.Engine3D` tile meshes / walls | 5.4 the wall depth bias of AMD-057 (every tile mesh is drawn **whole**; REQ-SCN-065 is held by depth, and `uWallClipBias` is 0 for a mesh whose normals are not axis aligned) | `amd057_wall_tests.rs` (both crates), `seam_tests.rs`, the `--dump-meshes` counts |
| `Zax.Nes/Ppu` pixel composition | 5.4 **the opacity mask of REQ-CORE-049 rides in bit 6 of the picture byte**: `palette_marked` is maintained only where `palette` is written, and `run_dots` must stay at 2231 instructions in the post-LTO assembly | PPU goldens, the `run_dots` instruction count |
| `Zax.Render` passes / new draw states | 5.4 grouping, labels and runs; clip unification; transparent object grouping | captures with and without `--group-draws` |
| `Zax.App/Loop` (frame clock, catch-up, pacing) | 5.5 separate core order per tick, ahead, pipeline governor, catch-up limit, half-rate pacing, **run-ahead** (the restore must be added to any new frame-running path, and N = 0 must stay `run_frame`) | app tests (`catch_up_tests`, `separate_core_tests`, `run_ahead_tests`), device profiler |
| `Zax.Settings` (new settings) | handheld defaults (`ui/handheld/defaults.rs`), handheld option rows, desktop `EXCLUDED` | settings catalogue test |
| Lua runtime / GC | 5.3 smooth GC tuning, `unsafe impl Send` handoff | bench-core with `ZAXNES_LUA_GC=smooth` |

Before closing the update: run the full golden suites, the 56-capture comparison in both draw
modes, `bench-core` on SMB1/SMB3/MM2/Metroid/Zelda II against the numbers above, and one device
run with the profiler on a light and a heavy profile.
