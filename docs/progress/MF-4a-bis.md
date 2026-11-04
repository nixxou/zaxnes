# Lot MF-4a-bis — giving back what MF-4a cost on games that run no DMA

R0 measured MF-4a's end (`c4577878`) against MF-3b's end (`b39eaba4`) with callgrind, 300 frames,
on clean builds made from `git archive` of each commit. Noise: zero instructions. Step 4 of MF-4a
only changed PAL constants, so the whole delta is its steps 1 to 3.

| game | stage | `b39eaba4` | `c4577878` | delta |
| --- | ---: | ---: | ---: | ---: |
| Mega Man 2 | core | 7 639 627 407 | 7 746 270 254 | **+106.6 M (+1.40 %)** |
| Mega Man 2 | PPU | 4 095 961 202 | 4 095 961 202 | 0 |
| Zelda II | core | 7 783 867 550 | 7 869 340 466 | **+85.4 M (+1.10 %)** |
| Zelda II | PPU | 4 295 001 664 | 4 295 001 664 | 0 |

R0's exclusive cost per function, Mega Man 2 (Zelda II in brackets):

| function | delta | this lot |
| --- | ---: | --- |
| `Apu2A03::quiet_cycles` | **+52.8 M** (+41.2 M) | step 1 |
| `Apu2A03::run_cpu_cycles` | **+35.1 M** (+28.7 M) | step 1 |
| `NesBus::read_cycle` (new) | +36.3 M (+36.4 M) | step 2 |
| `NesBus::tick` | −36.6 M (−39.8 M) | step 2 |
| `NesBus::run_dma` (new) | +7.3 M (+7.4 M) | the DMA cycles themselves |
| `NesBus::dma_read` (new) | +2.7 M (+2.7 M) | the DMA cycles themselves |
| `Cpu6502::step` + `branch` + `resolve` | +11.8 M (+11.7 M) | step 2 |
| `NesBus::write_internal` | −3.2 M (−3.2 M) | step 2 |

Q-119 estimated the CPU side right — `read_cycle` and `tick` cancel — and **did not measure the
APU side at all**: about +88 M (MM2) and +70 M (Zelda II), on every game, with or without a DMC
sample.

## Steps

1. The APU path back to `b39eaba4`: the DMC's load delay leaves the per-cycle APU code.
2. The CPU side: the address `movl` before each `read_cycle`.
3. The lot measured end to end (callgrind) and the suites.

## The instruments

Two, as MF-3b's rule asks (`docs/progress/MF-3b.md` step 5):

* the **post-LTO** assembly of the whole binary,
  `cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm`, with instruction counts
  and call sites per symbol, and the bodies diffed instruction for instruction after normalising
  basic-block labels and anonymous-constant hashes. The dumps are
  `rust/target-mf/asm/bin-base.s` (`b39eaba4`, built during the MF-4a session),
  `bin-step4.s` (MF-4a's code) and `bin-bis1.s`/`bin-bis2.s` (this lot). The three carry the same
  `rustc 1.99.1 (58cf8f9c3)`, LLVM 23.1.1, `x86_64-pc-windows-msvc`;
* **callgrind**, in step 3. It is available on this host after all — not under Windows, but in the
  WSL2 Ubuntu-24.04 the host carries, whose `rustc` is the *same* 1.99.1 with the same LLVM. Every
  earlier MF lot recorded "no callgrind on this host" and fell back to generated code alone; this
  lot measures instructions itself. What it cannot do is reproduce R0's two games: Mega Man 2 and
  Zelda II are commercial ROMs and are not in `assets-free/`. It measures the public ROMs that are
  — including `M1_P128K`, the same mapper 1 as both of R0's games, which is the board MF-3b used as
  its own stand-in.

## Step 1 — the APU path back to `b39eaba4`

### What MF-4a put on the per-cycle path, exactly

`DmcChannel::start_delay`, an `i32` counted down in `clock_timer` and in `skip_quiet_clocks` and
clamped in `quiet_clocks`. All three are per-cycle or per-quiet-run code on **every** game. In the
post-LTO dump of `run_cpu_cycles`, which is where `step_cpu_cycle`, `clock_timer`,
`skip_quiet_cycles` and `skip_quiet_clocks` are all inlined, MF-4a added two blocks.

`clock_timer`, `b39eaba4` on the left and MF-4a on the right (field offsets normalised to `N`):

    movl  N(%rdi), %ecx          testl %eax, %eax
    testl %ecx, %ecx             sete  %al
    jle   .L_78                  movl  N(%rdi), %ecx   ; start_delay
    testl %eax, %eax             testl %ecx, %ecx      ; tested on every CPU cycle
    sete  %al                    jle   .L_81           ; taken on every cycle of every game
    orb   %al, %bl               decl  %ecx
    ...                          movl  %ecx, N(%rdi)

**+3 executed instructions on every CPU cycle the APU steps.** And `skip_quiet_clocks` gained a
block of its own, +3 on every quiet run:

    movl  N(%rdi), %eax          ; start_delay
    testl %eax, %eax
    jle   .L_11
    subl  %ebx, %eax
    movl  %eax, N(%rdi)

`quiet_cycles` is worse, because LLVM made the clamp **branchless**: the DMC's `min` went from six
instructions to twelve, every one of them executed on every call —

    ; b39eaba4 (6)                 ; MF-4a (12)
    movl   840(%rcx), %edx         movl   656(%rcx), %eax     ; start_delay
    movl   872(%rcx), %ecx         movl   676(%rcx), %ecx     ; timer
    xorl   %eax, %eax              xorl   %edx, %edx
    testl  %ecx, %ecx              testl  %eax, %eax
    cmovgl %ecx, %eax              cmovgl %eax, %edx
    cmpl   %r8d, %eax              leal   -1(%rcx), %eax
    cmovael %r8d, %eax             cmpl   %edx, %eax
                                   cmovael %edx, %eax
                                   testl  %ecx, %ecx
                                   cmovlel %edx, %eax
                                   cmpl   %r8d, %eax
                                   cmovael %r8d, %eax

**+6 on every call.** So about **+9 instructions per CPU cycle** on a game whose APU is caught up
every cycle, plus +3 per quiet run — which is R0's +88 M over the ~8.8 M CPU cycles of 300 frames,
to the instruction. It is also why Q-119's reasoning missed it: the ticket counted the cost of the
*bus* gate, and this cost is in the APU, on cycles where no DMA is involved at all.

### The change

The delay is no longer a counter the APU owns. It is **one absolute cycle the bus owns**, computed
at the `$4015` write and compared only on the cycles that already reach the DMA path.

| item | change (file:line) | why it is free |
| --- | --- | --- |
| `start_delay` deleted; `clock_timer`, `quiet_clocks`, `skip_quiet_clocks`, `needs_sample_fetch` back to `b39eaba4` | `apu/dmc_channel.rs:39`, `:89`, `:166-187` | nothing per-cycle looks at the delay any more |
| `DmcChannel::set_enabled` **returns** the delay (0, 2 or 3) instead of storing it | `apu/dmc_channel.rs:126-161` | runs on a `$4015` write only |
| `Apu2A03::write` returns it to the bus | `apu/mod.rs:923-953` | likewise |
| `DMA_DMC_WAIT` and `NesBus::dmc_load_cycle`: the request held until an absolute cycle | `nes_bus.rs:28-43`, `:75-77` | one more bit of the `dma` byte `read_cycle` already tests; `dma == 0` on every cycle of a game with no DMA |
| `NesBus::promote_dmc_wait`: the held request becomes a real one when its cycle comes | `nes_bus.rs:400-418` | inlined into the two **cold** functions that call it; no symbol of its own |
| `halted_read_cycle` runs the cycle plainly while the load is still held | `nes_bus.rs:869-885` | only entered when `dma != 0` |
| `run_dma`'s loop promotes too, so a load coming due inside an OAM transfer still interleaves | `nes_bus.rs:548-553` | inside `run_dma`, which no game without a DMA enters |
| `after_apu_write` takes the delay, and a `$4015` disable cancels a held load as it cancels a queued one | `nes_bus.rs:783-809` | on an APU register write only |

The mechanism in one line: a `$4015` write that starts a sample sets `dmc_load_cycle = cycles + 2`
or `+ 3` by the parity of the write's cycle and raises `DMA_DMC_WAIT`; `read_cycle`'s existing
`dma == 0` gate then sends every read cycle to the cold half, which either runs the cycle (the
load's cycle has not come) or turns the held request into a real `DMA_DMC | DMA_DUMMY | DMA_HALT`
and runs the DMA.

### Why the timing is the same cycle, not one near it

`read_cycle` is entered *before* the cycle's `tick`, so `self.cycles` there is the previous
cycle's number. A `$4015` write is handled *after* its own cycle's `tick`, so `self.cycles` at the
write is the write's cycle `N`, and `dmc_load_cycle = N + D`. The DMA therefore runs on the first
read cycle `M` with `M − 1 ≥ N + D`, i.e. from `N + D + 1` on.

MF-4a: the counter reached 0 when the APU had stepped `N + D`, `poll_dmc_dma` raised the request
inside that cycle's `tick` — which is *after* `read_cycle` already tested `dma` for that cycle — so
the halt landed on the first read cycle after `N + D`, i.e. from `N + D + 1` on. **The same
cycle**, and the ROMs confirm it: `sprdma_and_dmc_dma` still prints its sixteen alignments as
`526` and `528`.

It is also exact where MF-4a was not. MF-4a needed the clamp in `quiet_clocks` *because* the APU
may lag: without it the counter would reach 0 late and the request would be noticed late. Here the
comparison is on bus cycles, which never lag, so the APU is free to lag as far as its channels
allow and the fetch still lands on the cycle the hardware puts it on.

A *reload* fetch is unchanged: `poll_dmc_dma` raises it from `catch_up_apu`/`sync_apu` as before,
with no delay, and `quiet_clocks` already returns the DMC timer — so the APU is always caught up on
the cycle the memory reader runs dry.

### What it gives back, in the generated code

Post-LTO, whole binary:

| symbol | `b39eaba4` | MF-4a | step 1 |
| --- | ---: | ---: | ---: |
| `Apu2A03::quiet_cycles` | 90 | 96 | **90** |
| `Apu2A03::run_cpu_cycles` (4 defs) | 949 | 956 | **949** |
| `DmcChannel::clock_timer` | *inlined* | *inlined* | *inlined* |
| `NesBus::tick` | 119 | 74 | **72** |
| `NesBus::read_cycle` | — | 3 | 3 |
| `NesBus::run_dma` | — | 289 | 309 |
| `NesBus::halted_read_cycle` | — | 8 | 32 |
| `NesBus::dma_read` | — | 97 | 95 |
| `NesBus::promote_dmc_wait` | — | — | *inlined, no symbol* |
| `NesBus::write_internal` | 2 629 | 2 550 | 2 553 |

and the two that matter are not merely the same size, they are **the same instructions**:

    $ asmtool.py body bin-base.s Apu2A0312quiet_cycles   | norm > a
    $ asmtool.py body bin-bis1.s Apu2A0312quiet_cycles   | norm > b
    $ diff a b && echo IDENTICAL
    IDENTICAL
    $ ... Apu2A0314run_cpu_cycles ...
    IDENTICAL

Identical **including every field offset** (`movl 872(%rcx)` for the DMC timer, `cmpl $2, 664(%rcx)`
for the triangle, …), because deleting the field put `DmcChannel`'s layout back where it was. That
is the strongest form the check can take: the per-cycle APU code of a game that plays no sample is
`b39eaba4`'s, byte for byte, and no test of any kind was left behind on it. `clock_timer`,
`skip_quiet_clocks`, `step_cpu_cycle` and `skip_quiet_cycles` have no symbols of their own in
either dump — they are inlined into `run_cpu_cycles`, which is identical, so they are too.

### `poll_dmc_dma` moving into `catch_up_apu`: the brief asked, and the answer is nothing

`catch_up_apu` and `sync_apu` are inlined into `NesBus::tick`, so the question is answered by
`tick`'s body. At `b39eaba4` the DMC's fetch question lived in the **shared tail** of `tick`:

    movl  14092(%rsi), %edx        ; apu_lag += 1
    incl  %edx
    movl  %edx, 14092(%rsi)
    cmpl  14096(%rsi), %edx        ; > apu_quiet?
    jbe   .L_12                    ; no -> .L_12
    ...                            ; the catch-up branches
    .L_12:
    cmpb  $1, 11932(%rsi)          ; the DMC's fetch request, on EVERY cycle
    jne   .L_23                    ; -> epilogue

and `tick`'s prologue was `pushq %rsi` / `pushq %rdi` / **`pushq %rbx`** / `subq $32`, the `%rbx`
being there for the DMC fetch MF-4a took out of this function altogether (at `b39eaba4` `tick`
inlined the whole fetch: four `callq tick`, a `read_internal`, `supply_byte` — 45 of its 119
instructions).

Per CPU cycle, step 1's `tick` against `b39eaba4`'s:

| cycle | `b39eaba4` | step 1 | delta |
| --- | --- | --- | ---: |
| the APU may lag (quiet) | prologue 4 + `cmpb`/`jne` 2 + epilogue 5 | prologue 3 + `jbe` straight to the epilogue + epilogue 4 | **−4** |
| the APU runs | prologue 4 + `jmp`/`movl`/`cmpb`/`jne` 4 + epilogue 5 | prologue 3 + `movl`/`cmpb`/`je` 3 + epilogue 4 | **−3** |

So the move **costs nothing**: the question is asked in the same place it always was for a cycle
the APU runs, it is gone entirely for a cycle the APU may lag, and `tick` lost a saved register
either way. `tick` is 72 instructions against `b39eaba4`'s 119 and MF-4a's 74 — the two it gives
back on MF-4a being the `start_delay` test in the cold tail of the inlined `poll_dmc_dma`.

### What the step costs, stated plainly

`halted_read_cycle` grows from 8 instructions to 32 and gains a stack frame
(`pushq %rsi` / `subq $32`), and `run_dma`'s loop runs three more instructions per DMA cycle
(`movzbl` / `testb $16` / `je`) to ask whether a held load has come due. Both are **inside the DMA
path**: a cycle that reaches them is a cycle on which a DMA is pending, which for a game that plays
no sample and writes no `$4014` never happens, and for one that does is the cycles R0's `run_dma`
and `dma_read` rows already account for. The frameless two-tail-call shape of `read_cycle` itself —
the thing Q-119 measured three times to get right — is untouched: still `cmpb` / `jne` / `jmp tick`,
three instructions, 101 call sites, no prologue.

### Savestates

Nothing moved, and one field left the format's neighbourhood rather than entering it.
`DmcChannel::start_delay` was never written (MF-4a kept it out and cleared it on load); it no longer
exists, so `load_state` has nothing to clear. `NesBus::dmc_load_cycle` is transient like `dma` and
`dma_oam_page`, is not written by `save_state`, and is cleared by `load_state` and by `reset`. A
state written before MF-4a loads byte for byte.

### Tests

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean. `cargo test --release -p zax-nes -p zax-harness`: **690 passed, 0 failed, 7 ignored** —
exactly MF-4a's count, no test added, removed or `#[ignore]`d. **No `unsafe`.**

The ROMs MF-4a made pass, re-run end to end:

| suite | MF-4a | step 1 |
| --- | --- | --- |
| `nes-test-roms/sprdma_and_dmc_dma` | 2 / 2 (`526`, `528`) | **2 / 2 (`526`, `528`)** |
| `nes-test-roms/dmc_dma_during_read4` | 3 pass, 0 fail, 2 manual | **3 pass, 0 fail, 2 manual** |
| `nes-test-roms/pal_apu_tests` | 10 / 10 | **10 / 10** |
| `nes-test-roms/apu_test` | 9 / 9 (19 with the PAL folder) | **19 / 19** |
| `nes-test-roms/blargg_apu_2005.07.30` | 11 / 11 | **11 / 11** |
| `AccuracyCoin` | 93 / 147 | **93 / 147**, the same fourteen failures and the same eight that take the run out of the loop |

`dma_4016_read` prints `08 08 07 08 08`, `dma_2007_read` prints
`11 22 / 11 22 / 44 55 / 11 22 / 11 22` with CRC `5E3DF9C4` — both MF-4a's output, character for
character. `ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`:
**9 passed**, the six groups MF-4a added included.

### Games whose output may move

**None.** Every ROM above prints what it printed, and the one behavioural difference the step can
have — the load fetch being noticed on time even when the APU is lagging, where MF-4a needed the
`quiet_clocks` clamp to get the same answer — makes the two models agree rather than differ. A
`$4015` disable, a soft reset or a sample that ends inside the two-or-three-cycle window now takes
the held load back, which MF-4a's `needs_sample_fetch` did too (the request simply never rose).
