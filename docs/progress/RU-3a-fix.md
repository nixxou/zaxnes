# RU-3a-fix — give back the per-cycle cost RU-3a added to boards without expansion audio

Branch `rustupdated`, on top of RU-3a (`4fa0277`). Functional scope: **none**. Nothing of the three
amendments changes; only where the MMC5-specific code sits, so that a cartridge without expansion
audio pays nothing for it. No audio digest may move relative to `4fa0277`.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | `NesBus::tick`: the MMC5 push out of the inlined per-cycle body | done |
| 2 | `Apu2A03::current_level`: `M` folded into the single cache key | todo |
| 3 | Closing: fmt, clippy, crate tests, PPU goldens, after measurements | todo |

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
