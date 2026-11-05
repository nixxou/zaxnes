# Lot MF-4b — the CPU and PPU register edges AccuracyCoin still reports

Baseline `3ac52b27` (MF-4a-ter's end). `AccuracyCoin` **93 / 147**: 14 failures, 8 tests the
drive had to retire, 32 never reached.

## What the ROM's own source says, and what the probe found

`assets-free/test-roms/AccuracyCoin/` ships `AccuracyCoin.asm` (739 KB, fully commented) and a
58 KB `README.md` with an error code list per test. Both are test-ROM documentation and were the
only sources used for the diagnoses below, with the NESdev wiki for the hardware and
`third_party/mesen2` where the two agree.

The first thing this lot did was run the eight retired tests **one at a time** — the ROM's own
README says "Pressing A will run the highlighted test" — instead of in the all-test loop. Seven
of the eight pass on their own:

| test | in the all-test loop | run alone |
| --- | --- | --- |
| `CPU Behavior 2 / Implied Dummy Reads` | took the run out of the loop | **pass** |
| `CPU Behavior 2 / DMA Landing on Write` | never returned | **pass** |
| `CPU Behavior 2 / DMC Reload Timing` | never returned | fail 1 |
| `CPU Behavior 2 / DMC Playback Rate` | never returned | **pass** |
| `PPU Behavior / CHR ROM is not writable` | never returned | **pass** |
| `PPU Behavior / PPU Register Mirroring` | never returned | **pass** |
| `PPU Behavior / PPU Register Open Bus` | never returned | **pass** |
| `PPU Behavior / PPU Read Buffer` | never returned | **pass** |
| `PPU Behavior / Palette RAM Quirks` | never reached | **pass** |

So the brief's step 2 is not a stretch of missing PPU behaviour: `$2000-$3FFF` mirroring, the
PPU's I/O latch with its decay and the `$2007` read buffer are all there and the ROM says so.
What takes the run down is **one** defect, and the ROM names it:

* `CPU Behavior / Open Bus` **fail 7** — "Reading from $4015 should not update the data bus."
* `CPU Behavior 2 / Internal Data Bus` **fail 3** — "Reads from $4015 only update the internal
  data bus and cannot affect the external data bus."

and `Implied Dummy Reads` is built on exactly that: it fetches opcodes out of `$4015` and takes
the operands of a `JSR` out of `$4016`/`$4017`, whose open-bus bits "leech off the value read
from the opcode (reading $4015 doesn't update the data bus)". With one data bus instead of two,
those operands come out wrong, the `JSR` lands in nowhere and the ROM leaves its loop — which
is what the eight retirements are, since the drive gives up after `MAX_SKIPS = 8`.

## Steps

1. `Interrupt flag latency`: the second poll of a page-crossing taken branch does not **clear**
   a decision the first poll already took.
2. The `$2007` read buffer is set up two PPU dots after the read, so two `$2007` reads inside
   one instruction both see the old buffer (`double_2007_read`).
3. The CPU's **two** data buses: the internal one every access drives, and the external one a
   read of `$4015` leaves alone and a DMC fetch drives on its own.
4. The `SH*` stores: a DMA on the dummy read cycle drops the AND with the address high byte.
5. The suites and `AccuracyCoin` end to end, the campaign document, the regression guard.

## Instruments

Baseline post-LTO dump `rust/target-mf/asm/bin-4b-base.s`, built at `3ac52b27` with
`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm`; `bodies.py` reports it
**IDENTICAL** to MF-4a-ter's `bin-ter.s` on all ten pinned symbols, so the baseline is that
lot's.

## Step 1 — the two polls of a page-crossing branch are a set, not an overwrite

### What the test does, and what was wrong

`Interrupt flag latency` ran all the way to its **last** sub-test and failed it, code E: "An
interrupt polled successfully on the first poll of a branch, cleared, and then polled again,
should still occur."

The sub-test is `JSR $4013` — the CPU fetches its opcode and operand out of open bus, where a
DMC fetch has left `$90`, so it runs `BCC $3FA5`: taken, and crossing from page `$40` to `$3F`,
hence four cycles. The NESdev "CPU interrupts" page says such a branch polls twice,
"before the second CPU cycle (the operand fetch)" and, "for taken branches that cross a page
boundary, before the PCH fixup cycle". The ROM times the APU frame counter so the IRQ is there
for the **first** poll, and makes the branch's third cycle a dummy read of `$4015`, which
acknowledges the frame IRQ — so the **second** poll finds nothing. On hardware the IRQ still
happens: polling raises an internal signal that stays raised for the rest of the instruction.

This core took the second poll as the decision and so lost the IRQ.

### The change

`cpu/cpu6502.rs` `branch`: on the page-crossing arm, the sample the first poll took is OR-ed
back into `poll_irq` instead of being dropped. `poll_nmi` needs nothing — it *is* `nmi_pending`,
which only `step` and `service_interrupt` clear, so an edge the first poll saw is still there.

The only way a program can tell the difference is to deassert the IRQ line inside the branch,
and a branch makes four **reads**: the one read that can do it is `$4015`. So nothing else moves.

### Hot path

Post-LTO `bodies.py`, `bin-4b-base.s` against `bin-4b-s1.s`: `Cpu6502::step` **893, IDENTICAL**,
and `read_cycle`, `tick`, `read_internal`, `write_internal`, `resolve`, `run_cpu_cycles`,
`quiet_cycles` all **IDENTICAL**. `Cpu6502::branch` is **366 instructions against 368** — two
*fewer* — and the diff is three hunks:

* the taken page-crossing epilogue gains `orb 39(%rsp), %r15b` / `movb %r15b, 12(%rsi)`
  (`12(%rsi)` is `poll_irq`), and one spill store `movb %dl, 39(%rsp)` on the way in;
* the taken in-page arm **loses** two reloads (`movl 44(%rsp), %r15d`, `movzbl 43(%rsp), %edx`)
  and gains one (`movzbl 39(%rsp), %eax`), because the two arms no longer need the NMI copy;
* one arm of the widescreen-zone dispatch loses four instructions
  (`movb %r13b, 8(%rsi)` … `je`) to tail merging.

So a taken page-crossing branch pays about **+2 instructions**; a not-taken or in-page branch
pays 0 or gains one. Measured end to end with callgrind in step 5.

### Games whose output may move

A game would have to take a page-crossing branch whose third cycle reads `$4015` while the frame
counter's IRQ is pending and `I` is clear. The third cycle of a branch reads the byte after the
operand, so that means a branch whose operand sits at `$4014` — i.e. code executing inside
`$4000`-`$401F`. **No commercial game does this**; `cpu_exec_space/test_cpu_exec_space_apu` is
the only ROM here that executes in that range at all, and it passes before and after.

### Tests

`cpu_behaviour_tests::req_core_111_a_page_crossing_branch_keeps_what_its_first_poll_found`:
`BEQ` at `$10FE`, four windows of the IRQ line — low on cycle 1 only (the new case), low on
cycles 1-3, low on cycle 3 only (`cpu_interrupts_v2` part 5's page-crossing column), and an
in-page taken branch with the line low from cycle 2 (the delay `5-branch_delays_irq` pins, which
the sticky poll must not undo). `TestBus` grows one field, `irq_window`, to place the line's
level on a chosen cycle.

`cargo fmt --check` clean, `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean (**no new `allow`**), `cargo test --release -p zax-nes -p zax-harness`: **696 passed,
0 failed**, MF-4a-ter's 695 plus this one.

### ROMs

| suite | `3ac52b27` | step 1 |
| --- | --- | --- |
| `cpu_interrupts_v2` | 6 / 6 | **6 / 6** |
| `instr_timing` | 3 / 3 | **3 / 3** |
| `branch_timing_tests` | 3 / 3 | **3 / 3** |
| `cpu_timing_test6`, `cpu_dummy_reads`, `cpu_dummy_writes`, `cpu_exec_space`, `instr_misc` | all pass | **all pass** |
| `AccuracyCoin` | 93 / 147 | **94 / 147** — `Interrupt flag latency` **pass** |

The eight retirements and the 32 tests behind them are unchanged, as expected: they are step 3's.

**Save states**: no field added, nothing moved.

## Step 2 — the `$2007` read buffer is refilled two dots late

### What was wrong, and it is not mirroring

`$2000-$3FFF` mirroring every eight bytes, the PPU's I/O latch with its 600 ms decay and the
`$2007` buffer were all already right: the probe above has `PPU Register Mirroring`,
`PPU Register Open Bus`, `PPU Read Buffer` and `Palette RAM Quirks` passing when run alone, and
`ppu_open_bus` and `ppu_read_buffer` pass in the campaign. What was wrong is the one thing in the
brief that no AccuracyCoin row covers: **`dmc_dma_during_read4/double_2007_read`**, which is
`manual` and was printing an output its own source does not list.

That ROM runs `lda $20F7,x` with X=`$10`: a page cross, so the dummy read hits `$2007` and the
real read `$2107` — two `$2007` reads one CPU cycle apart. Its header lists what hardware gives:

```
;22 33 44 55 66
;22 44 55 66 77 or
;22 33 44 55 66 or
;02 44 55 66 77 or
;32 44 55 66 77 or
;85CFD627 or F018C287 or 440EF923 or E52F41A5
```

Every one of the four has the **second** read returning the buffer as it stood *before* the
first. This core printed `33 44 55 66 77`, i.e. the first read's refill already visible to the
second — none of the four, and CRC `D84F6815`, none of the four either.

`AccuracyCoin`'s `$2007 Stress Test` error code 2 states the rule: "Reading from $2007 should
set up the PPU Read Buffer **two ppu cycles after the CPU Read ends**."

### The change

`ppu/mod.rs`: two fields, `read_buffer_prev` and `read_buffer_clock`, and a `$2007` read returns
`read_buffer_prev` when `0 < ppu_clock - read_buffer_clock < READ_BUFFER_READY_DOTS` (5). The
five is two terms: the refill lands two dots after the read cycle ends, and a read presents the
buffer at the *start* of its own cycle, three dots before the `ppu_clock` the bus hands
`Ppu2C02::read`. So a pair one CPU cycle apart (three dots) is stale and a pair two cycles apart
(six) is not — exactly the line the ROM draws.

The window is open **strictly after** the previous read's dot, because two reads on the same dot
are not two CPU reads: only `NesCore::ppu_read`, the time-free host accessor, can make them, and
there it means "read the register again". Its doc comment now says so.

`double_2007_read` prints **`22 33 44 55 66 / 22 44 55 66 77 / 85CFD627`** — the first output and
the first CRC of the four.

### Hot path, and the cost

Post-LTO, `bin-4b-s1.s` against `bin-4b-s2.s`. `Ppu2C02` grows 8 bytes, which moves every field
after it in `NesBus`, so every pinned body "DIFFERS" on **displacements alone**. Normalising
integers of 2048 or more — on these functions every such literal is a struct offset — gives:

| symbol | instructions | verdict |
| --- | ---: | --- |
| `Cpu6502::step` | 893 → 893 | **IDENTICAL**, offsets normalised |
| `Cpu6502::branch` | 366 → 366 | **IDENTICAL** |
| `Cpu6502::resolve` (2 defs) | 1 725 → 1 725 | **IDENTICAL** |
| `NesBus::read_cycle` | 3 → 3 | **IDENTICAL** |
| `NesBus::tick` | 75 → 75 | **IDENTICAL** |
| `NesBus::write_internal` | 2 565 → 2 565 | **IDENTICAL** |
| `Apu2A03::run_cpu_cycles` (4 defs) | 873 → 873 | **IDENTICAL** (raw, offsets and all) |
| `Apu2A03::quiet_cycles` | 90 → 90 | **IDENTICAL** (raw) |
| `NesBus::read_internal` | 729 → **739** | **DIFFERS**, +10 |

The +10 are **inside the register-7 arm**, after `cmpw $7, %ax ; jne`: nine instructions that
read `read_buffer_clock`, pick `read_buffer` or `read_buffer_prev` with a `cmovae` on the address
of the field, and store the two bookkeeping fields. **Nothing on any other read.** Every call-site
count in `report.py` is unchanged, and MF-3b rule 1 holds (`end_cycle` 0 → 0,
`read_nametable_as` 0 → 0, `read_nametable_inner` 15 → 15, `read_pattern_planes` 3 → 3,
`Board::ppu_read` 4 → 4, `ppu_peek` 4 → 4, `on_ppu_address_bus` 12 → 12, `on_cpu_cycle` 1 → 1).

**The first shape of this cost `read_internal` a callee-saved register** — `pushq %r15` /
`popq %r15` and eight more bytes of frame, *on every CPU read cycle of every game*, because
`visible` had to stay live across the `read_vram_inner` call. Writing the two bookkeeping fields
**before** that call removes it, and the comment on them says so, since it is not obvious from the
source.

### Games whose output may move

A game needs two `$2007` reads one CPU cycle apart: either an indexed read of a `$2007` mirror
that crosses a page (`lda $20F7,x` with X ≥ `$09`), a read-modify-write on a `$2007` mirror, or a
DMC DMA halting a `$2007` read. The last is the only one commercial code can stumble into, and
`dma_2007_read` — the ROM built for it — prints **exactly what it printed before**,
`11 22 / 11 22 / 44 55 / 11 22 / 11 22`, CRC `5E3DF9C4`, which is one of its two documented
outputs: the extra reads of a DMA halt read the same byte through the buffer either way. So no
game is expected to move, and `ppu_read_buffer`, `read_write_2007` and `dma_2007_write` confirm
the single-read path is untouched.

### Tests and ROMs

`ppu_rendering_tests::req_core_112_a_2007_read_one_cycle_after_another_still_sees_the_older_buffer`
drives the bus itself — `bus.read` is a real cycle — and pins both halves: the pair one cycle
apart both returning `$22` with the buffer left holding the *second* read's byte (`$44`, not
`$33`), and the reads two cycles apart lagging one byte as always.

| suite | `3ac52b27` | step 2 |
| --- | --- | --- |
| `dmc_dma_during_read4` | 3 pass, 2 manual; `double_2007_read` `22 33 44 55 66 / 33 44 55 66 77 / D84F6815` | 3 pass, 2 manual; **`double_2007_read` `22 33 44 55 66 / 22 44 55 66 77 / 85CFD627`** |
| `ppu_read_buffer`, `ppu_open_bus` | 2 / 2 | **2 / 2** |
| `blargg_ppu_tests_2005.09.15b` | 4 / 5 (`power_up_palette` required) | **4 / 5**, same row |
| `ppu_vbl_nmi`, `vbl_nmi_timing` | 11 / 11, 7 / 7 | **11 / 11, 7 / 7** |
| `sprite_hit_tests`, `sprite_overflow_tests` | 11 / 11, 5 / 5 | **11 / 11, 5 / 5** |
| `oam_read`, `oam_stress` | 2 / 2 | **2 / 2** |
| `AccuracyCoin` | 94 / 147 | **94 / 147** |

`cargo fmt --check` clean, clippy clean (**no new `allow`**), `cargo test --release -p zax-nes -p
zax-harness`: **697 passed, 0 failed**.

**Save states**: the format is unchanged. `read_buffer_clock` is cleared on load and
`read_buffer_prev` is only looked at inside the window, and nothing is lost — the window is
shorter than two CPU cycles and the only pair of `$2007` reads that close is the dummy read and
the read of one indexed instruction, which no save point can fall inside (a state is written
between instructions, and cycle 1 of an instruction is always its opcode fetch).

## Step 3 — two data buses, and the drive that was reading the ROM wrong

### What the ROM says

`AccuracyCoin`'s `Internal Data Bus` test states the rule in one line: "The internal data bus is
updated during every read/write, while the **external** data bus is updated on every read/write
**except for reads from address `$4015`**", and its test 2 states the other half, that a DMC
sample fetch "does not update the internal data bus. Only the external one." `$4015` is decoded
inside the 2A03 and answers on its internal bus; the sample byte reaches the DMC off the pins.

Three rows came straight out of that, and the first two were the lot's measurable target:

| test | code | what it says |
| --- | --- | --- |
| `CPU Behavior / Open Bus` | 7 | "Reading from $4015 should not update the data bus." |
| `CPU Behavior 2 / Internal Data Bus` | 3 | "Reads from $4015 only update the internal data bus and cannot affect the external data bus." |
| `CPU Behavior / Open Bus` | 9 | "Bit 5 of address $4015 is open bus" — and `Internal Data Bus` 2 says *which* bus: the internal one, which a DMC fetch does not touch. |

### The change

`nes_bus.rs`. The field that was `open_bus` is now `data_bus` and holds the **internal** bus:
every read, every write, every repeated read of a DMA halt cycle loads it, which is what the old
single bus already did, so the per-cycle path does not change at all. The external bus is a pair,
`ext_bus` and `ext_cycle`, written at the **two cold sites where the two diverge**:

* `hold_external_bus`, in `read_internal`'s `$4015` arm: the pins keep what they had;
* `drive_external_bus`, in `dmc_fetch_read`: the pins take the sample byte and the internal bus
  keeps what it had.

`external_bus()` returns `ext_bus` while `cycles - ext_cycle <= 1` and `data_bus` otherwise, and
the bound is exact: a CPU cycle *is* one bus access, so any cycle other than those two drives both
buses, and the cycle after a divergence is the last one that can tell them apart. Two cases need a
DMA **and** an address in `$4000`-`$401F` to fall outside it and read `data_bus` instead — a halt
cycle the controller cannot see directly after a `$4015` read, and an OAM transfer from page `$40`
interleaved with a DMC fetch — and the function's doc names them. No ROM here reaches either.

The three consumers that mean "nothing is driving the pins" now ask `external_bus()`:
`$4000`-`$4014` and `$4018`-`$401F`, an address no cartridge answers, and the extension's window.
`$4015`'s bit 5 and the three open-bus bits of `$4016`/`$4017` keep reading `data_bus` — which is
what `Implied Dummy Reads` needs when it takes the operands of a `JSR` out of the controller ports
after fetching the opcode from `$4015`: they "leech off the value read from the opcode".

### And then the real blocker, which was the harness

`Implied Dummy Reads` still took the run out of the all-test loop at 96/147 — although it passes
on its own, and although page 14 run as a page passes it, and although **every earlier page marked
as skipped** did not help. That last measurement is what gave it away: the cause was not state
left by another test, it was the drive.

`TEST_ImpliedDummyRead_BackupRAM` copies `$00`-`$FF` to `$700`-`$7FF`, **zeroes the zero page**,
and puts it back when the test is done — it needs every byte of it for the function it assembles
to run out of open bus. `RunningAllTests` is `$0035` and `PostAllTestTally` is `$0037`. So for the
whole of that test both read 0, and the drive's "the run is over when `RunningAllTests` drops to
zero" fired in the middle of it: it stopped, called the test "took the run out of the all-test
loop", retired it, and hit the same wall on every retry until `MAX_SKIPS` was spent. Eight tests
reported as hung, 32 behind them never reached, and the emulator had nothing to do with any of it.

`coin.rs` now measures progress with `run_count` — how many of the 147 result bytes are written —
which lives in page 4, which no test touches but its own. The run is over when all 147 are there
and `RunningAllTests` is zero; the stall watch is on that same count; and the two labels are still
told apart, by whether `RunningAllTests` stayed zero for the whole wait.

### What it moves

| | `3ac52b27` | step 3 |
| --- | --- | --- |
| `AccuracyCoin` | 93 / 147, 14 failed, **8 retired, 32 never run**, 9 attempts, 29 s | **118 / 147, 29 failed, 0 retired, 0 never run, 1 attempt, 2.6 s** |

The 25 rows that moved: `Open Bus`, `Interrupt flag latency` (step 1), `Implied Dummy Reads`,
`Internal Data Bus`, `DMA Landing on Write`, `DMC Playback Rate`, `CHR ROM is not writable`,
`PPU Register Mirroring`, `PPU Register Open Bus`, `PPU Read Buffer`, the seven
`PPU VBlank Timing` rows, `Sprite overflow behavior`, `Sprite 0 Hit behavior`,
`Suddenly Resize Sprite`, `Misaligned OAM DMA`, `t Register Quirks`, `INC $4014`,
`Rendering Flag Behavior`, `Attributes As Tiles` and `Sprites On Scanline 0` — all of which were
behind the wall.

One row reads worse and it is **not** a regression: `DMA + $2002 Read` was `pass 1` in the old
nine-attempt run, where the last attempt had eight tests marked skipped and so ran everything
with different timing, and is `fail 2` in the honest single run. It passes when run on its own.
Step 5 measures `3ac52b27` under the corrected drive so that the before/after is read the same way
on both sides.

### Hot path

`offsets.py` (`bodies.py` with struct displacements normalised, since `NesBus` grows 16 bytes and
every field after `ext_bus` moves), `bin-4b-s2.s` against `bin-4b-s3.s`:

| symbol | instructions | verdict |
| --- | ---: | --- |
| `Cpu6502::step` | 893 → 893 | **IDENTICAL** |
| `Cpu6502::branch`, `resolve` | 366, 1 725 | **IDENTICAL** |
| `NesBus::read_cycle`, `tick`, `write_internal` | 3, 75, 2 565 | **IDENTICAL** |
| `Apu2A03::run_cpu_cycles`, `quiet_cycles` | 873, 90 | **IDENTICAL**, raw |
| `NesBus::read_internal` | 739 → **757** | +18 |
| `NesBus::dma_read` | 98 → **109** | +11 |
| `NesBus::run_dma` | 506 → **511** | +5 |

Where the three go, read off the diffs:

* `read_internal`: **+9 in the `$4015` arm** (the select and the two stores), **+5 on the
  unmapped-cartridge tail** and +4 of tail duplication. The prologue is byte for byte the same —
  same five pushes, same `subq $32` — so no callee-saved register was added, and the RAM, PRG,
  PRG-RAM-answered and PPU-register paths are untouched. Two sites had to be made **lazy** to get
  there: `unwrap_or(self.external_bus())` evaluates the select before the `Option` is tested, which
  put it on every read a cartridge *does* answer, and the `$4000`-`$401F` test in `dma_read` had it
  hoisted in front of the compare, which put it on every byte of every OAM transfer. Both are now
  written as a `match` and as two returns, with the reason on them.
* `dma_read`: +6 in the `$4000`-`$401F` arm, +9 in the `internal` `$4015` sub-read, −2 elsewhere.
  The ordinary OAM source read — `read_internal` then two stores — is unchanged.
* `run_dma`: the five instructions of `dmc_fetch_read` inlined at the DMC-fetch arm, once per
  fetch.

So: **nothing per cycle**, nine instructions per `$4015` read, five per DMC fetch, five per read
of an address no cartridge answers.

### Games whose output may move

A game must read `$4015` and then read an unmapped address (or `$4000`-`$4014`/`$4018`-`$401F`) on
the very next cycle, or read `$4015` within a cycle of a DMC fetch. The first needs an indexed read
whose dummy cycle lands on `$4015`, i.e. `lda $40FF,x` with X ≥ `$16`; the second needs a DPCM
sample playing while `$4015` is polled, which **is** ordinary code — a music driver reading `$4015`
to see whether a channel has finished. What changes there is bit 5 only, the one bit of `$4015`
that is not a flag, and a driver that reads bit 5 reads a value no hardware guarantees. No
commercial title is expected to move; `apu_test`, `blargg_apu_2005.07.30`, `apu_reset`,
`apu_mixer`, `pal_apu_tests`, `dmc_dma_during_read4` and `sprdma_and_dmc_dma` are all unchanged,
ROM by ROM.

### Tests and ROMs

* `memory_access_tests::req_core_081_a_4015_read_answers_on_the_internal_bus_and_not_on_the_pins`
  — write `$A5`, read `$4015` (bit 5 set, every other bit 0), then read `$4018`: `$A5`, not the
  `$20` the APU just answered with.
* `dmc_dma_timing_tests::req_core_113_a_dmc_fetch_drives_the_external_bus_and_not_the_internal_one`
  — both halves, with `$20` on both buses and a sample byte of `$00`: a halted `$4015` read still
  reads bit 5 from the internal bus, and a halted read of an unmapped address returns the sample
  byte off the pins.

| suite | `3ac52b27` | step 3 |
| --- | --- | --- |
| `apu_test` 19/19, `blargg_apu` 11/11, `apu_reset` 6/6, `apu_mixer` 4/4, `pal_apu_tests` 10/10 | all pass | **all pass** |
| `sprdma_and_dmc_dma` 2/2, `dmc_dma_during_read4` 3 pass 2 manual, `dmc_tests` 4 manual | unchanged | **unchanged** |
| `cpu_interrupts_v2`, `instr_misc`, `instr_test-v3`, `instr_test-v5`, `nes_instr_test`, `cpu_dummy_reads`, `cpu_dummy_writes`, `cpu_exec_space`, `cpu_reset`, `instr_timing`, `branch_timing_tests`, `cpu_timing_test6` | all pass | **all pass** |
| `ppu_open_bus`, `ppu_read_buffer`, `read_joy3` | unchanged | **unchanged** |
| `AccuracyCoin` | 93 / 147 | **118 / 147** |

`cargo fmt --check` clean, clippy clean (**no new `allow`**), `cargo test --release -p zax-nes -p
zax-harness`: **699 passed, 0 failed**.

**Save states**: the format is unchanged — `data_bus` is the byte `open_bus` was. `ext_bus` and
`ext_cycle` are cleared on load and nothing is lost: the two buses differ for at most the cycle a
`$4015` read or a DMC fetch happened on and the one after it, a state is written between
instructions, and the first cycle of an instruction is its opcode fetch, which drives both buses
before anything reads the external one.

## Status

- [x] step 1 — the sticky poll of a taken branch
- [x] step 2 — the `$2007` read buffer's delay
- [x] step 3 — the internal and external data buses
- [ ] step 4 — the `SH*` stores and a DMA on the dummy read
- [ ] step 5 — the suites, the campaign document and the guard
