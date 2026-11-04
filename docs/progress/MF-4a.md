# Lot MF-4a — DMA timing and the APU frame sequencer on PAL

The first lot on the NES core itself. Everything here runs on every cycle of every game, so
MF-3b's rule applies to every commit: the post-LTO generated code of the hot functions touched,
or of their callers, compared with the step before, with what changed and why it is free.

Baseline, measured before the first commit (`b39eaba4`), with
`zax-harness test-roms <roms> out.json --filter <suite> --no-shots`:

| suite | before |
| --- | --- |
| `nes-test-roms/sprdma_and_dmc_dma` | 0 / 2 — both `B8EA17D9 / Failed (code 1)` |
| `nes-test-roms/dmc_dma_during_read4` | 2 pass, 1 fail (`dma_4016_read`: `08 08 08 08 08`), 2 manual |
| `nes-test-roms/pal_apu_tests` | 3 / 10 — `04` `05` `06` `07` `08` `10` `11` fail |
| `nes-test-roms/apu_test` | 9 / 9 |
| `nes-test-roms/blargg_apu_2005.07.30` | 11 / 11 |
| `nes-test-roms/apu_reset` | 6 / 6 |
| `AccuracyCoin` | 89 / 147 |

## Steps

1. The DMA unit: the halt only on a read cycle, the get/put alignment, and the DMC fetch
   interleaved with an OAM transfer in progress (`sprdma_and_dmc_dma`, `sprdma_and_dmc_dma_512`).
2. The halt cycle's repeated read and the 2A03's own $4015/$4016/$4017 decode during a DMA
   (`dmc_dma_during_read4/dma_4016_read`).
3. AccuracyCoin's DMA page.
4. The frame sequencer on a PAL console (`pal_apu_tests` 04 to 08, 10, 11).
5. The suites, the campaign document and the regression guard.

## Step 1 — the DMA unit

### What was wrong

The old model did two things the hardware does not, both recorded as known in the comment on
`perform_dmc_dma`: it ran a DMC fetch **on the cycle the APU asked for the byte**, whatever the
CPU was doing on it, and it ran the whole OAM transfer **inside the `$4014` write**. The nesdev
"DMA" page says otherwise on three counts, and each one is a ROM:

* *"DMA halts the CPU only on read cycles. If the CPU is writing, it ignores the halt and the
  DMA unit waits until the next cycle to try again."*
* The halt cycle leaves the CPU's address on the bus, so **its read happens twice** — the extra
  `$4016` read `dmc_dma_during_read4/dma_4016_read` counts and the extra `$2007` reads
  `dma_2007_read` prints.
* *"The CPU alternates between cycles on which DMA can get (read) and cycles on which DMA can
  put (write)"*, and when both units want the bus *"DMC DMA is allowed to run and OAM DMA is
  paused, trying again on the next cycle"*.

### The change

| item | change (file:line) | test ROM / test | hot path, evidence, cost |
| --- | --- | --- | --- |
| the halt lands on a read cycle only | `nes_bus.rs:812-842` `read_cycle` / `halted_read_cycle`, called from `CpuBus::read` and `read_code`; the DMC poll leaves `tick` (`:259-262`) | `sprdma_and_dmc_dma`, `dma_4016_read` | see "What it costs" below |
| get/put alignment, halt, dummy, realignment, DMC inside an OAM transfer | `nes_bus.rs:432-540` `run_dma` / `dma_cycle`, replacing `perform_dmc_dma` and `perform_oam_dma` | `sprdma_and_dmc_dma`, `sprdma_and_dmc_dma_512` | `#[inline(never)]`, never entered unless a DMA runs |
| the 2A03's own `$4015`/`$4016`/`$4017` decode during a DMA | `nes_bus.rs:556-600` `dma_read` | `dma_4016_read` (step 2's target, reached here) | inside `run_dma` |
| the DMC fetch request noticed where the APU runs, not every cycle | `nes_bus.rs:333-347` `poll_dmc_dma`, called from `catch_up_apu`, `sync_apu` and `after_apu_write` | all of the above | **−2 instructions per CPU cycle** in `tick` |
| the DMC output unit's phase | `apu/dmc_channel.rs:224-243` `reset`: the timer starts a whole period from its first bit | `sync_dmc` converges at all | load-time only |
| the load fetch's start delay | `apu/dmc_channel.rs:139-152` `set_enabled`, 2 or 3 cycles by the write's parity | `sprdma_and_dmc_dma` ×2 | one `i32` in the DMC, counted down in `clock_timer` |

Two of those are phase, not machinery, and they are the two that actually decided the lot:

* **The DMC bit clock fired on odd CPU cycles.** Every entry of both rate tables is even, so the
  phase of the output unit — and with it the phase of every reload fetch — is fixed from
  power-on by the timer's initial value, which was `0`. On that phase a reload fetch aligns to
  **three** cycles. The hardware takes four, and blargg's `sync_dmc` is built on it: its
  fine-synchronisation loop is written one cycle longer than the DMC's 3424-cycle byte
  ("3421+4 clocks per iter") so that it drifts into alignment. At three the loop is exactly
  3424 cycles, locks, and never converges — which is why the first attempt hung **every** ROM
  of `dmc_dma_during_read4` on a blank screen. Starting the timer a whole period from its first
  bit, as Mesen 2 does at reset for the same stated reason, puts it back.
* **A fetch started by a `$4015` write is not asked for on the write's own cycle**: it waits two
  or three cycles by the parity of that cycle, which is what makes a *load* DMA "typically take
  3 cycles" against a *reload*'s four. Without it `sync_dmc` locks onto a phase three cycles
  from the hardware's and `sprdma_and_dmc_dma` measures the DMC fetch in the wrong place: its
  sixteen alignments came out `528 528 528 528 528 526 526 530 526 …`, the step from four
  stolen cycles to two in the right place but one alignment out of order. With it, both ROMs
  pass.

### Where the get cycle is

A get cycle is an **even** cycle in this core's numbering, which is not a new convention: it is
the one the `$4017` write delay already reads (three cycles on `apu_cycle`, four otherwise —
`apu_reset/4017_timing`, `apu_test/4-jitter`), and it is the one that gives the OAM transfer
its 513 and 514 cycles unchanged. Both halves of that are pinned by tests, so the DMC's own
phase was the only free variable left, and the two items above fix it.

### What it costs — and it is not free

MF-3b's instruments, post-LTO dump of the whole `zax-harness` binary, baseline `b39eaba4`:

| symbol | defs before | after | call sites before | after |
| --- | ---: | ---: | ---: | ---: |
| `Cpu6502::end_cycle` | 0 | 0 | 0 | 0 |
| `Ppu2C02::read_nametable_as` | 0 | 0 | 0 | 0 |
| `Ppu2C02::read_nametable_inner` | 1 | 1 | 15 | 15 |
| `Ppu2C02::read_pattern_planes` | 1 | 1 | 3 | 3 |
| `Board::ppu_read` | 1 | 1 | 4 | 4 |
| `Board::on_ppu_address_bus` | 1 | 1 | 12 | 12 |
| `Board::irq_asserted` | 0 | 0 | 0 | 0 |
| `Board::on_cpu_cycle` | 1 | 1 | 1 | 1 |
| `Cpu6502::execute` / `execute_without_operand` | folded / 1 | folded / 1 | — | — |
| `NesBus::tick` | 1 (**119 instructions**) | 1 (**74**) | 117 | 115 |
| `NesBus::read_cycle` | — | 1 (**3**) | — | ~100 |
| `NesBus::run_dma` | — | 1 | — | 1 |

`tick` is the one that matters. Its quiet path — the cycles on which the APU is allowed to lag,
which is most of them — was

    cmpl 14096(%rsi), %edx
    jbe  .LBB1426_12
    ...
    .LBB1426_12:
    cmpb $1, 11932(%rsi)      ; the DMC's fetch request, asked on every cycle
    jne  .LBB1426_23          ; -> epilogue

and is now `jbe` straight to the epilogue; and its prologue lost `pushq %rbx` / `popq %rbx`,
because the DMC fetch it used to inline is gone. **−4 instructions on every CPU cycle of every
game.** The DMC question now lives in `catch_up_apu`, where it costs `cmpb $1, 11720(%rsi)` and
a not-taken `je` on the cycles the APU actually runs — exact, because a quiet cycle is by
definition one on which "the DMC's fetch request stays as it is".

Against that, a **read** cycle gains, at the call site and inside the three-instruction
`read_cycle`:

    movl  %r15d, %edx                 ; the address the halt cycle would re-read
    callq read_cycle
      cmpb $0, 14113(%rcx)            ; a DMA is waiting?
      jne  halted_read_cycle
      jmp  tick

**+4 instructions on a read cycle; a write cycle is byte for byte what it was.** Reads being
about two thirds of the cycles, the executed-instruction count comes out roughly nil, slightly
negative.

Three shapes were written and measured before this one, and the two rejected ones are worth
recording because both are MF-3b's lessons repeating:

| shape | wall clock |
| --- | --- |
| the gate *inlined* at the ~100 sites `CpuBus::read` is pasted into | **+8 %** — five bytes of code per site inside `Cpu6502::step`; it also pushed `read_code` out of line and `Cpu6502::execute` back out of `step`, MF-3b's stop-the-lot signature |
| the gate out of line, but `run_dma` inlined into it | **+5 %** — the prologue of every read cycle saving eight general registers and two XMM, exactly the trap `push_expansion_audio` carries its own note about |
| the gate out of line with **no stack frame** (two tail calls), `run_dma` and its cold half `#[inline(never)]` | **+1 %** ← this step |

`bench-core --frames 3600 --warmup 300`, three runs each, the two binaries built one after the
other on this host and measured in both orders:

| ROM | `b39eaba4` | MF-4a step 1 |
| --- | --- | --- |
| `M0_P32K_C8K_V` (NROM) | 0.797 / 0.797 / 0.806 | 0.808 / 0.811 / 0.820 |
| `M1_P128K` (MMC1) | 0.808 / 0.809 / 0.817 | 0.815 / 0.818 / 0.822 |
| `M4_P128K` (MMC3) | 1.003 / 1.006 / 1.011 | 1.010 / 1.012 / 1.024 |

about **+1 %**, consistent across the three ROMs and both orders. For scale, changing one DMC
field from `i32` to `u8` — which cannot change a single executed instruction — moved these same
figures by ±3 % in both directions on this host, so +1 % is of the order of the placement noise;
it is reported as a cost anyway because it does not change sign. **Q-119** states the cost, the
three shapes and why no cheaper one was found, and asks R0 to rule; the code carries `// Q-119`
at both places. It was committed rather than held back because holding it back means shipping no
DMA at all, and the three ROMs the lot is asked for pass with it.

### Games whose output may move

Every game that plays a DPCM sample, which is most commercial titles with speech or drums
(*Super Mario Bros. 3*, *Mega Man 3*-*6*, *Batman*, *Journey to Silius*, *Castlevania III*…):
a fetch now lands one to three cycles later than before and steals three or four cycles instead
of always four. Specifically:

* **a DMC fetch that collides with a controller read** — the halt re-reads `$4016`, so a game
  that reads the pad while a sample plays can lose a bit, which is the glitch the
  "read the controller twice and compare" idiom exists for (*Super Mario Bros. 3*, *Mega Man 3*,
  *Battletoads*). Games that already read twice see no change; games that do not may see a
  dropped input frame exactly where hardware drops one.
* **a DMC fetch inside an OAM transfer** — the transfer now takes two more cycles, so a game
  that writes `$4014` with a sample playing and counts cycles to the end of vblank
  (*Battletoads*, *Micro Machines*) moves by two cycles per collision.
* **`STA $4014` itself** — the stolen cycles are now charged to the instruction *after* it,
  because that is where the CPU's next read cycle is. The total is unchanged.

### Tests

`cargo fmt --check` clean; `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean (one `allow(clippy::inline_always)` on `CpuBus::read_code`, with the measured reason on
it, as MF-3b's `end_cycle` precedent); `cargo test --release -p zax-nes -p zax-harness`:
**681 passed, 0 failed, 7 ignored** (680 at the lot's start plus one new test).

Two test files changed, both because they drove the bus in a way a CPU never does:

* `apu_channel_tests.rs` advanced time with `bus.tick()`, which is not a read cycle, so no DMC
  fetch could ever be served. It now advances with `bus.read(0x8000)` — one cycle, as before,
  but the kind of cycle a DMA may halt. `req_core_113_a_dmc_sample_fetch_steals_four_cpu_cycles`
  becomes `…_three_or_four_…` and pins which one is which: the load fetch the `$4015` write
  starts is the short one, every reload after it costs four.
* `oam_dma_timing_tests.rs` expected the whole transfer inside the `$4014` write. It now writes
  and then makes the read the halt lands on, and a new test
  `req_core_111_the_oam_transfer_waits_for_a_read_cycle` pins the rule itself: three writes
  after the `$4014` write cost three cycles and nothing else, and the 513 or 514 arrive on the
  first read after them.

**Save states**: nothing moved. `NesBus::dma` and `dma_oam_page` are not in the format and are
cleared on load, `DmcChannel::start_delay` likewise — a state is written between instructions
and the DMA units own every cycle of theirs, so none can be in flight at a save point; and if
one were pending, `poll_dmc_dma` re-arms it on the next cycle the APU runs. A state written
before this lot loads byte for byte.

### Suites

| suite | before | after |
| --- | --- | --- |
| `nes-test-roms/sprdma_and_dmc_dma` | 0 / 2 | **2 / 2** |
| `nes-test-roms/dmc_dma_during_read4` | 2 pass, 1 fail, 2 manual | **3 pass, 0 fail, 2 manual** |
| whole campaign | 204 pass, 15 fail | **207 pass, 12 fail** |
| `AccuracyCoin` | 89 / 147 | **93 / 147** |

`dma_4016_read` prints `08 08 07 08 08`, which is its own source's documented output, and
`dma_2007_read` — `manual`, because its result depends on CPU/PPU synchronisation and its source
lists two acceptable outputs — now prints `11 22 / 11 22 / 44 55 / 11 22 / 11 22` with CRC
`5E3DF9C4`, one of the two its source names. It printed `11 22` five times before, which is the
"no extra `$2007` reads" answer and is not among them.
