# RU-3a-fix — give back the per-cycle cost RU-3a added to boards without expansion audio

Branch `rustupdated`, on top of RU-3a (`4fa0277`). Functional scope: **none**. Nothing of the three
amendments changes; only where the MMC5-specific code sits, so that a cartridge without expansion
audio pays nothing for it. No audio digest may move relative to `4fa0277`.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | `NesBus::tick`: the MMC5 push out of the inlined per-cycle body | done |
| 2 | `Apu2A03::current_level`: `M` folded into the single cache key | done |
| 3 | Closing: fmt, clippy, crate tests, PPU goldens, after measurements | done |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## What R0 measured (the reason for this lot)

Callgrind, `zax-harness bench-core`, 300 frames, Mega Man 2 and Zelda II (no expansion audio),
RU-2 (`30dc744`) vs RU-3a (`4fa0277`):

| stage | MM2 | Zelda II |
|---|---|---|
| core (`NesCore::run_frame`, inclusive) | **+1.05 %** | +0.93 % |
| apu (`Apu2A03::run_cpu_cycles`, inclusive) | +0.92 % | +0.79 % |
| ppu | 0 | 0 |

Almost all of it is exclusive to two functions: `NesBus::tick` +67.7 M on MM2 (432 M → 500 M,
+15.7 %, ~5 instructions per CPU cycle, from two extra callee-saved registers in the prologue) and
`Apu2A03::run_cpu_cycles` +13.9 M (the second comparison in the level cache test).

## Instrument used on this side

No callgrind and no `objdump`/`nm` on this machine, and a `bench-core` timing is far too noisy for
1 %. The instrument is therefore the **static code of `NesBus::tick`**, read out of the assembly
rustc itself emits:

```
cargo rustc -p zax-nes --release --lib -- --emit=asm
# then count, between the function's label and its .seh_endproc:
#   instructions (tab + lowercase mnemonic), pushq, the subq on %rsp, outgoing call/jmp
```

The RU-2 baseline is built the same way in a throwaway `git worktree` of `30dc744`
(`CARGO_TARGET_DIR` outside the repository; nothing of it is committed). This reproduces R0's
finding exactly, which is what validates the instrument — the absolute register names differ
because this machine builds for the MSVC ABI and R0 measures the SysV one, the phenomenon is the
same:

| build | `tick` instructions | `pushq` | stack | outgoing |
|---|---|---|---|---|
| RU-2 `30dc744` | **104** | **3** | 32 | 11 |
| RU-3a `4fa0277` | 144 (+38 %) | 4 | 40 | 13 |

A second, truer reading of the same instrument: the **whole program** after fat LTO, which is what
R0's callgrind actually runs (`cargo rustc -p zax-harness --release --bin zax-harness -- --emit=asm`
emits the post-LTO assembly of every function). The figures below are that build.

## Notes

### How often the APU catch-up really runs (the measurement that decided the design)

RU-3a's notes claim `catch_up_apu` "runs rarely for a board without expansion audio". **It does
not.** Measured with a throwaway `AtomicU64` in `catch_up_apu` and in `tick`, 300 frames after 120
of warm-up (the instrumentation was removed again before the commit):

| ROM | ticks/frame | `catch_up_apu`/frame |
|---|---|---|
| `nestest.nes` (mapper 0) | 29 780 | **14 894 (50.0 %)** |
| `mmc5test/mmc5test.nes` | 29 780 | 29 780 (100 %) |
| `exram/mmc5exram.nes` | 29 780 | 29 780 (100 %) |

Exactly half the cycles, because the APU's own quiet count is 0 on one cycle in two (the pulse
timers are clocked on alternate cycles). This is what rules out the obvious fix: `catch_up_apu` is
itself a per-cycle path, so **one** extra instruction in it costs ~15 000 per frame, and R0's gate
of ±0.1 % on a core of roughly 27 M instructions per frame is ~27 000 per frame. A single
`if self.expansion_audio` left on that path (load, compare, branch) would be ~0.17 % on its own —
over the gate. The test had to cost nothing, not merely less.

### Step 1 — the `tick` path

Two separate causes, both fixed by moving code out of line rather than by removing it.

**1. `catch_up_apu`: one test instead of two, not three.** The observation that makes it free:
`expansion_audio` implies the APU may never lag, and `deferred_ppu` was *already* tested at the end
of the function to decide the same thing. So the two conditions are folded into one field,
`apu_batching = deferred_ppu && !expansion_audio`, maintained in `attach`, `defer_ppu` and
`end_deferred_ppu` (the only three places either input changes), and the test is hoisted to the top:

```rust
fn catch_up_apu(&mut self) {
    if !self.apu_batching {
        self.catch_up_apu_in_step();   // #[inline(never)]
        return;
    }
    if self.apu_lag > 0 { self.apu.run_cpu_cycles(self.apu_lag); self.apu_lag = 0; }
    self.apu_quiet = self.apu.quiet_cycles();
}
```

`catch_up_apu_in_step` serves both of the cases that may not lag — a board with expansion audio
(where it pushes `M`) and any board outside `defer_ppu` — and always leaves `apu_quiet` at 0, which
is exactly what RU-3a's `if deferred_ppu && !expansion_audio { quiet_cycles() } else { 0 }`
computed. Behaviour is unchanged by construction; the goldens confirm it.

The hot path now costs what RU-2's cost, instruction for instruction: RU-2 spent
`xorl / cmpb deferred_ppu / jne` (3) after the APU call, this spends `cmpb apu_batching / je` (2)
before it plus the `jmp` out of the branch (3).

**2. `push_expansion_audio` out of line.** The second half of the regression was not in
`catch_up_apu` at all: `sync_apu` is inlined into `perform_dmc_dma`, which is inlined into `tick`,
and the push's cartridge `Option`, its devirtualised `expansion_audio_output` and its
`set_expansion_level` were therefore inlined into `tick` a second time. That is what cost the extra
callee-saved register for every cycle of every board. It is now `#[inline(never)]`, with the
`expansion_audio` test left at its two call sites (both of them rare).

**`tick` after step 1** (fat-LTO build, the same instrument as the table above):

| build | `tick` instructions | `pushq` | stack | outgoing |
|---|---|---|---|---|
| RU-2 `30dc744` | 104 | 3 | 32 | 11 |
| RU-3a `4fa0277` | 144 | 4 | 40 | 13 |
| **after step 1** | **112** | **3** | **32** | 13 |

The prologue and the stack frame are back to RU-2's. The eight static instructions that remain are
all in the DMC-DMA block — `cmpb expansion_audio / je` plus the call to the out-of-line push — which
runs `dmc_dma_count` times per frame, not per cycle. Diffed line by line, **the per-cycle path of
`tick` is RU-2's, with the same number of instructions on it.** The two extra outgoing calls are
`catch_up_apu_in_step` and `push_expansion_audio`, neither of them on it.

One incidental finding, worth recording because it would mislead the next measurement: in a
**non-LTO** per-crate build the fix leaves one extra instruction per cycle, a
`leaq <memory>(%rsi), %rdx` before the cartridge's `on_cpu_cycle`. It is the loss of LLVM's
dead-argument elimination on that trait method's unused `&mut CartridgeMemory`: AMD-054 gave the
MMC5 a real `on_cpu_cycle` body, which stops the dispatcher from inlining every arm, so the
argument stays live. **Fat LTO eliminates it again**, so it does not exist in the shipped binary or
in what R0 measures — but a per-crate asm dump will show it.

**The order of the cycle is kept as RU-3a set it**: the cartridge steps before the APU catch-up, so
a board with expansion audio has produced this cycle's `M` before the APU mixes it (REQ-CORE-075).
It is free — the diff of the two hot paths shows the same instructions, only in a different order —
and the 291 goldens matched both before and after it, so there was no reason to take the risk of
putting it back.

`bench-core`, release, 1200 frames after 120 of warm-up, three runs each, the RU-2 binary built
from a throwaway worktree of `30dc744` and run against the same `test-roms`:

| ROM | RU-2 core | after step 1 core |
|---|---|---|
| `nestest.nes` (mapper 0) | 0.8708 / 0.8793 / 0.8870 | 0.8726 / 0.8762 / 0.8811 |
| `mmc5test/mmc5test.nes` | 1.8307 / 1.8372 / 1.8386 | 1.8354 / 1.8639 / 1.8629 |
| `exram/mmc5exram.nes` | 2.0811 / 2.0967 / 2.0985 | 2.0637 / 2.0841 / 2.1008 |

`nestest` and `exram` are indistinguishable from RU-2; `mmc5test` is the board that gained the
feature. Timing at this scale cannot resolve 1 %, which is why the assembly above is the real
evidence.

Behaviour: `cargo test --release -p zax-nes` 482 passed, and the full goldens report
**`291 of 291 rows matched, 0 ROMs missing`** — no audio digest moved.

(`catch_up_apu_in_step`, the function this step introduced, is `sync_apu` to the line; step 2
collapses the two and moves the `#[inline(never)]` to the push alone, which is where it belongs.
The shape described above is kept for the record of what each measurement was taken on.)

### Step 2 — the level cache key

`M` is folded into the one key the cache already compares, exactly as the lot asks:
`set_expansion_level` stores `NO_LEVEL_KEY` (`-1`, a value the five packed nibbles can never
produce) when the value moves, and the hot test is back to `key == self.level_key` alone. The
invalidation runs only for the MMC5 and only on a real change; the comparison runs on every cycle
that reaches the mixer. `level_expansion` stays, but only as the record of the `M` the cached level
was mixed with — which is what `mixer_expansion()` reports for REQ-CORE-076.

Every mutation of `expansion_level` is now paired with an invalidation (`new`, `reset`,
`set_expansion_level`, `load_state`), which is what makes the single comparison sound.

What the assembly says, inside `Apu2A03::run_cpu_cycles` where the mixer is inlined — the cached
path, the one taken when nothing moved:

| build | the cache test |
|---|---|
| RU-3a | `movl 920(%rdi),%ecx` … `cmpl 916(%rdi),%r9d` / `jne` / `cmpl 924(%rdi),%ecx` / `je` — **5** |
| after step 2 | `cmpl 916(%rdi),%r10d` / `je` — **2** |

Three instructions saved on a path `nestest` reaches about 15 000 times per frame: 3 × 15 000 × 300
frames = 13.5 M, against the **+13.9 M** R0 measured. The regression is accounted for to within
3 %, which is as close as a static reading gets to a callgrind count.

The `movl 920(%rdi)` (the load of `expansion_level`) did not disappear, it moved to the *recompute*
branch, where RU-2 had its own work; `run_cpu_cycles` comes out at 855 instructions against RU-2's
856 and RU-3a's 857.

**Two traps on the way, both recorded because they would be easy to fall into again.**

1. **`current_level` must keep a single call site.** The first version of the savestate rebuild
   called `current_level` from `load_state`. That second call site — cold, run once — was enough for
   the inliner to leave the whole function **out of line**: `run_cpu_cycles` dropped to 730
   instructions and a separate `current_level` of 132 appeared, which would have cost a call and a
   prologue on each of those ~15 000 cycles per frame (worse than the bug being fixed). `#[inline]`
   was not enough to undo it. `load_state` therefore writes the four `mixer::` lines out itself,
   and the note above `current_level` says why it must stay alone.
2. **The rebuild cannot touch `level_key`.** Step 2 removes the field RU-3a parked at an impossible
   value (`level_expansion = -1`) to force the first cycle after a load to rebuild the test point,
   so the rebuild now happens *in* `load_state` — from the channel outputs just read back, with
   `level_key` left exactly as the block gave it. Setting `level_key` to `NO_LEVEL_KEY` there would
   break the save → load → save byte comparison of REQ-CORE-092, which is the same trap RU-3a
   reports hitting from the other side. The test point is now right immediately after a load rather
   than one cycle later, which is strictly what RU-3a intended.

New test, `req_core_076_a_move_of_m_alone_reaches_the_mixer` in `apu_output_tests.rs`: with every
channel silent — so the packed key is 0 on every pass and the cache would hold — five successive
values of `M` each reach `mixer_expansion`, `mixer_output` and `mixed_level`. It fails if the
invalidation is dropped, which is what pins the mechanism this step relies on.

### The MMC5 board, and what it costs

The cost does not vanish, it moves onto the board that gained the feature. Three shapes were
measured, interleaved run by run with the RU-2 and RU-3a binaries so that machine drift cancels
(`bench-core`, core `run_frame`, 1200 frames after 120 of warm-up, three runs each):

| shape | `tick` static | `nestest` | `mmc5test` |
|---|---|---|---|
| RU-2 `30dc744` | 104 / 3 push | 0.870 / 0.883 / 0.894 | 1.832 / 1.832 / 1.838 |
| RU-3a `4fa0277` | 140 / 4 push | 0.861 / 0.863 / 0.866 | 1.795 / 1.809 / 1.825 |
| `sync_apu` out of line too | 101 / 3 push | 0.849 / 0.868 / 0.863 | 1.904 / 1.908 / 1.916 |
| **kept: only the push out of line** | 119 / 3 push | 0.868 / 0.874 / 0.875 | 1.868 / 1.871 / 1.874 |

The third row is the tempting one — the smallest `tick` of the four — and it is the wrong choice: a
board with expansion audio runs `sync_apu` on **every** cycle, so out-lining it too costs the MMC5 a
second call per cycle for nothing. The kept shape out-lines the push alone, which is the part whose
register pressure reaches `tick`, and `sync_apu` stays inlined where it is reached.

So the MMC5 pays one out-of-line call per cycle, about **+2 % against RU-2** on `mmc5test` and
`exram` (and about +3 % against RU-3a, which was itself *below* RU-2 on those ROMs because
AMD-054's table lookups are cheaper than the float mixing they replaced). That is the price of
every other board paying nothing, and AMD-054's own note already puts it where it belongs: no
built-in profile targets a game with MMC5 audio.

Two ways to take it back were examined and rejected:
- inlining the push after all, by making `Mmc5::expansion_audio_output` inlinable — its
  `Mmc5Audio::output()` is two channel outputs, a table index and an addition, not a field read, so
  inlining it puts the register pressure straight back on every board;
- caching `M` in a plain field the MMC5's `on_cpu_cycle` writes, so the push needs neither the
  cartridge `Option` nor the dispatch — this would work, but it is a new path from the board to the
  bus and a new field, which is more than this lot is allowed to change.

Both are noted here for whoever measures next; neither is needed to meet the lot's target.

Behaviour after step 2: `cargo test --release -p zax-nes` **483 passed** (the 482 of RU-3a plus the
new one), and the full goldens again report **`291 of 291 rows matched, 0 ROMs missing`** — no audio
digest moved.

### Step 3 — closing

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --release --features zax-app/bundled -- -D warnings` | clean |
| `cargo test --release -p zax-nes` | 483 passed |
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
| **total** | **2088 passed, 0 failed** (RU-3a's 2087 plus the new mixer test) |
| `cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored` | **291 of 291 rows matched**, 0 missing |

#### The final state of the per-cycle path

`NesBus::tick`, post-LTO, against the two references:

| build | instructions | `pushq` | stack | hot-path APU block |
|---|---|---|---|---|
| RU-2 `30dc744` | 104 | **3** | **32** | 12 instructions |
| RU-3a `4fa0277` | 140 | 4 | 40 | 14, plus the prologue on every cycle |
| **RU-3a-fix** | 119 | **3** | **32** | **12 instructions** |

Read instruction by instruction, the catch-up block is the same length as RU-2's — RU-2 spent
`xorl %eax,%eax` / `cmpb deferred_ppu` / `jne`, this spends `cmpb apu_batching` / `je` and the `jmp`
out of the branch. The cartridge block is byte-identical and only moved earlier in the function.
The 15 static instructions above RU-2 are the `!apu_batching` branch and the DMC-DMA block, neither
on the per-cycle path of a board without expansion audio.

`Apu2A03::run_cpu_cycles`: **855** instructions against RU-2's 856 and RU-3a's 857, same prologue
(8 saved registers), stack 184 against RU-2's 232.

#### What this lot did *not* change

No requirement, no amendment, no setting, no savestate format, no spec copy, no Q ticket: the
behaviour of the three amendments of RU-3a is identical, which is what the 291 unchanged golden rows
(audio digests included) and the 2 088 passing tests say. `STATE_VERSION` stays 8 / 9, the APU block
keeps its layout, and `rust/docs/PERFORMANCE-HISTORY.md` §5.1 was corrected rather than left to
drift — RU-3a's entry there claimed `catch_up_apu` "runs rarely", which the counting in these notes
disproves, and the two new invariants (a single comparison in the mixer's cache, nothing
board-specific inlined into `tick`) are written down with the way to check them.

#### Left to R0

The callgrind re-run that is the real gate, on Mega Man 2 and Zelda II. The expectation this side's
evidence supports: `NesBus::tick` back to its RU-2 exclusive count (same prologue, same per-cycle
instructions) and `Apu2A03::run_cpu_cycles` three instructions per mixer call lighter than RU-3a,
which is the 13.9 M R0 measured. The MMC5 is about 2 % above RU-2 on `mmc5test` and `exram`; the two
ways to take that back are listed above and both are outside this lot's scope.
