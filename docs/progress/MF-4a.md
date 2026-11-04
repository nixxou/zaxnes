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

## Step 2 — the halt cycle's repeated read, and the 2A03's own decode

### What the step found

Nothing left to fix. `dmc_dma_during_read4/dma_4016_read` **already passes** after step 1, and
it passes for the reason the suite is about rather than by accident: the halt cycle of step 1's
`run_dma` repeats the CPU's read, `$4016` is clocked once more, and the ROM prints
`08 08 07 08 08` — its own source's documented output, with the seven in the one iteration of
five where the DMA lands on the `lda $4016`.

The rest of the folder does not regress, and one row moved without changing its label:

| ROM | before MF-4a | after step 1 |
| --- | --- | --- |
| `dma_2007_read.nes` | manual — `11 22 / 11 22 / 11 22 / 11 22 / 11 22 / 498C5C5F` | manual — `11 22 / 11 22 / 44 55 / 11 22 / 11 22 / 5E3DF9C4` |
| `dma_2007_write.nes` | pass | pass |
| `dma_4016_read.nes` | **fail** — `08 08 08 08 08` | **pass** — `08 08 07 08 08` |
| `double_2007_read.nes` | manual — `22 33 44 55 66 / 33 44 55 66 77 / D84F6815` | manual, unchanged |
| `read_write_2007.nes` | pass | pass |

`dma_2007_read` is `manual` because its own source says the output "depends on CPU-PPU
synchronization at reset" and lists two acceptable results; before the lot it printed neither
(five identical lines is the "no extra `$2007` read" answer), and it now prints the second of
them, CRC included. `double_2007_read` is unchanged and is **not** a DMA row: its second line
comes from `lda $20F7,x` with X=$10 reading `$2007` twice inside one instruction, which is the
PPU's own read buffer on back-to-back reads — lot MF-4b's `PPU Register Mirroring` and open-bus
territory, not this one's.

### What the step adds

`crates/zax-nes/tests/dmc_dma_timing_tests.rs`, two tests, because the behaviour the ROM
measures deserves to be pinned where a bisect can see it:

* `req_core_113_the_halt_cycle_reads_4016_again_and_the_program_loses_a_bit` — the same
  experiment the ROM runs, in eight lines: with the DMC quiet, the controller answers on the
  eighth read of `$4016`; with a fetch pending, on the seventh.
* `req_core_113_only_the_halt_cycle_of_a_dma_reaches_the_controller` — the other half of the
  rule, and the one that would silently rot: the DMC's dummy cycle and its alignment cycle must
  **not** reach the port (/OE stays active), so the program loses one bit and not three. It also
  checks that the fetch cost three or four cycles, so the test cannot pass by the DMA not having
  happened.

Both arm the fetch with a `$4015` write and then burn its start delay on **write** cycles, which
is itself a check that a write cannot be halted.

### Hot path

No file under `src/` changed in this step, so the generated code is the step 1 commit's, byte
for byte: nothing to compare. `cargo fmt --check` clean, clippy clean,
`cargo test --release -p zax-nes -p zax-harness`: **683 passed, 0 failed, 7 ignored**.

No game's output moves: no behaviour changed.

## Step 3 — AccuracyCoin's DMA page

### What step 1 moved

`AccuracyCoin` is one ROM of 147 tests run in one pass, so the figure that matters is the
per-test table in the JSON, not the single row in the campaign. Against `b39eaba4`:

| test | before | after step 1 |
| --- | --- | --- |
| `APU Registers and DMA tests / DMA + $2002 Read` | fail 2 | **pass** |
| `APU Registers and DMA tests / DMA + $2007 Read` | fail 2 | **pass** |
| `APU Registers and DMA tests / DMA + $2007 Write` | fail 1 | **pass** |
| `APU Registers and DMA tests / DMA + $4015 Read` | fail 2 | **pass** |
| `APU Registers and DMA tests / DMA + $4016 Read` | fail 1 | **pass** |
| `APU Registers and DMA tests / DMC DMA + OAM DMA` | fail 2 | **pass** |
| `APU Tests / Delta Modulation Channel` | fail 15 | **pass** |
| `CPU Interrupts / Interrupt flag latency` | hung, took the run out of the loop | fail E — it returns now |
| **total** | **89 / 147** | **93 / 147** |

Seven of the eight rows that moved are the DMA page itself, and the eighth is the DMC. The set
of tests that block the run is the same before and after (32 never run, from
`Palette RAM Quirks` on), so nothing was traded for them.

### What still fails, and why

| test | code | diagnosis |
| --- | --- | --- |
| `DMC DMA Bus Conflicts` | 2 | the byte a DMA read returns when the 2A03's own decoder answers *and* the external bus does. `NesBus::dma_read` ANDs the two, which is the plain two-drivers-on-a-bus answer; Mesen 2 keeps the external value on the pins the `$4016`/`$4017` port leaves open and ANDs only the rest. Doing that needs an open-bus mask per controller port, which `InputPorts` does not have — a controller-port change, not a DMA one. |
| `Explicit DMA Abort` | 2 | a `$4015` write that takes a fetch back **after** its halt cycle has already run. The halt is dropped but the cycle is still stolen. This core cannot reach that state: `run_dma` owns every cycle from the halt onwards, and a `$4015` write is a write cycle, so the only abort it can see is one *before* the halt, which it does handle (`after_apu_write`). Reaching the other one means the two-or-three-cycle `$4015` disable delay plus an abort check inside the DMA loop. |
| `Implicit DMA Abort` | 2 | same mechanism, with the sample ending rather than a write. |
| `DMA Landing on Write` | hangs | it was `fail 5` before the lot and now does not return. The rule itself is implemented and tested (`req_core_111_the_oam_transfer_waits_for_a_read_cycle`, and the two `dmc_dma_timing_tests`); what the test measures beyond it is unknown without its source. |
| `DMC Reload Timing`, `DMC Playback Rate` | hang | both were **blocked** before the lot — they are reached now, and hang. Same family as the two aborts: the `$4015` delays. |
| `Open Bus`, the five `SH*` stores, `Internal Data Bus`, `Implied Dummy Reads` | 7, 3 | **out of scope**: the brief gives the `SH*` stores and PPU open bus to lot MF-4b. |
| `Interrupt flag latency` | E | **out of scope**, lot MF-4b. |
| `Frame Counter IRQ`, `APU Register Activation`, `Controller Strobing` | 7, 6, 4 | APU register edges, not DMA. |

The three abort rows share one missing piece: a `$4015` write's effect on the DMC's **stop**
is delayed two or three cycles the way its effect on the start is, and a fetch caught inside
that window is dropped after its halt cycle rather than before it. That was written, measured,
and **taken back out**: it moved no AccuracyCoin row, it changes what `$4015` bit 4 reads back
for two to three cycles after a disable, and nothing in the suites pins the new behaviour — so
it would have been unverified behaviour on the APU's hottest register. It belongs with a lot
that can check it, and the diagnosis is recorded here so that lot does not have to find it
again.

### Hot path

No file under `src/` changed in this step. 683 tests, unchanged; the campaign unchanged.

## Step 4 — the frame sequencer on a PAL console

### The fix, which is three numbers

`apu/mod.rs:90-98`. The nesdev "APU Frame Counter" page gives each console's steps in **APU**
cycles, and a CPU cycle is half of one. For the 2A07's mode 0 those are 4156.5, 8313.5, 12469.5,
then 16626, 16626.5 and 16627 for the three cycles on which the IRQ flag is held — so the
sequence closes at CPU 33252, 33253, 33254. The table said 33250, 33251, 33252: **two cycles
early**, and nothing else in the four tables was wrong.

| table | before | after |
| --- | --- | --- |
| `NTSC_FOUR_STEP` | 7457, 14913, 22371, 29828, 29829, 29830 | unchanged |
| `NTSC_FIVE_STEP` | 7457, 14913, 22371, 29829, 37281, 37282 | unchanged |
| **`PAL_FOUR_STEP`** | 8313, 16627, 24939, **33250, 33251, 33252** | 8313, 16627, 24939, **33252, 33253, 33254** |
| `PAL_FIVE_STEP` | 8313, 16627, 24939, 33253, 41565, 41566 | unchanged |

That one row is all seven failures. Six of them — 04, 05, 07, 08, 10 and 11 — measure something
against the end of the mode 0 sequence directly. The seventh, `06.len_timing_mode1`, measures
mode 1, whose table was already right: it fails because it **synchronises on the mode 0 frame
IRQ** before switching, so a sequence that closes two cycles early moves its whole measurement.
That is why it was the odd one out in the campaign's diagnosis.

### The ROMs

| suite | before | after |
| --- | --- | --- |
| `nes-test-roms/pal_apu_tests` | 3 / 10 | **10 / 10** |
| `nes-test-roms/apu_test` | 9 / 9 | 9 / 9 |
| `nes-test-roms/blargg_apu_2005.07.30` | 11 / 11 | 11 / 11 |
| `nes-test-roms/apu_reset` | 6 / 6 | 6 / 6 |
| `nes-test-roms/apu_mixer` | 4 / 4 | 4 / 4 |
| whole campaign | 207 pass, 12 fail | **214 pass, 5 fail** |

The five failures left are `AccuracyCoin`, `power_up_palette` — whose own readme says its
reference values "are probably unique to my NES" — and the three MMC3 revision A ROMs the
campaign marks **required to fail** under REQ-CORE-100.

### The test

`apu_frame_counter_tests::req_core_015_each_region_closes_its_four_step_sequence_where_nesdev_says`
times the sequence **end to end**, on all four regions, rather than reading the table back: it
writes `$4017`, counts CPU cycles until the frame IRQ asserts, and requires 29830 for NTSC,
33254 for PAL and 29830 for Dendy and Multi, which use the 2A03's APU. A table edited by hand
cannot satisfy it without being right, and the two regions that borrow the NTSC sequence are
pinned too.

### Hot path, and the NTSC guarantee

`Apu2A03::run_cpu_cycles` — which is where `step_frame_counter` and the `sequence()` lookup are
inlined — is **byte for byte** the step 1 commit's, 880 instructions, in the post-LTO dump of
the whole binary; `NesBus::tick` likewise, 74; and the whole-binary call-site table is
unchanged, symbol for symbol. A constant in a `static` table moved and nothing else did.

**An NTSC game executes exactly the instructions it did**: the PAL table is only reached through
`sequence()`, behind `timing.uses_pal_noise_and_dmc_periods`, which is true for
`NesRegion::Pal` alone — Dendy and Multi take the NTSC tables, as REQ-CORE-015 says and as the
new test now checks.

### Games whose output may move

PAL games only, and all of them: the frame sequencer's last quarter and half frame, and its IRQ,
arrive two cycles later. That is two cycles per 33254 on every envelope, sweep and length
counter of a PAL title, and on the frame IRQ of a PAL game that uses it. No NTSC, Dendy or
Multi game moves by one cycle.

`cargo fmt --check` clean, clippy clean, `cargo test --release -p zax-nes -p zax-harness`:
**684 passed, 0 failed, 7 ignored**.
