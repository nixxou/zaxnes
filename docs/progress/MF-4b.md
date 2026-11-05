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

## Step 4 — a DMA on the dummy read drops the `SH*` stores' AND

### What the rule is

All five `SH*` rows failed with code **7**, which is the one sub-test of each that brings a DMA
in: "If the RDY line goes low 2 cycles before the write cycle, the target address of the
instruction was not correct." The basic behaviour — the value AND-ed with the un-indexed address's
high byte plus one, and that same value replacing the high byte on a page cross — was already
right, codes 1 to 6 passing.

The high byte reaches the data lines through the path a DMA unit takes over, so when the CPU was
halted on the dummy read cycle the AND does not happen and the register is stored whole; the
address still gets its corrupted high byte. Mesen 2 implements the same rule in `SyaSxaAxa`
("Dummy read took more than 1 cycle, so it was interrupted by a DMA"), and the file carries the
adaptation header.

### The change, and where the cost is

The CPU cannot see a DMA: `CpuBus::read` is the DMA's own gate, and asking it on the way *in*
would pay at every one of the hundred sites `read` is pasted into — MF-4a's +8 % lesson. So it is
asked **afterwards**, once, from the `SH*` tail:

* `CpuBus::dma_gave_back_at` (`cpu/cpu_bus.rs`), a trait method with a default of `i64::MIN`;
* `NesBus::dma_gave_back`, one store in `halted_read_cycle` at the point the CPU gets its read
  back — on a halted read cycle only, and **zero on every other cycle**;
* `Cpu6502::apply_write`: `bus.dma_gave_back_at() == bus.cycle_count()`. At that point the write
  has not run, so `cycle_count()` *is* the dummy read's cycle.

Post-LTO, `bin-4b-s3.s` against `bin-4b-s4.s` with `offsets.py`: `read_cycle`, `tick`,
`read_internal`, `write_internal`, `branch`, `resolve`, `run_dma`, `dma_read`,
`run_cpu_cycles`, `quiet_cycles` — all **IDENTICAL**. `Cpu6502::step` is 893 → **897**, and the
diff is one hunk of four instructions (`movq dma_gave_back` / `cmpq cycles` / `movzbl` / `cmovnel`)
inside the `SH*` write arm, which no official opcode reaches.

### Games whose output may move

Only a game that executes `SHA`, `SHX`, `SHY` or `TAS` **and** has a DMA land on that
instruction's dummy read. No commercial NES title uses these opcodes; blargg's `instr_test-v5`
`03-immediate` and `07-abs_xy` exercise them and both still pass, as do `cpu_dummy_writes` and
`instr_misc`.

### Tests and ROMs

`cpu_behaviour_tests::req_core_110_a_dma_on_the_dummy_read_drops_the_and_of_the_sh_stores` pins
*which* cycle it is instead of asserting a constant: a halt claimed on cycles 1, 2, 3 or 5 of a
five-cycle absolute-indexed write leaves the AND in place, and only cycle 4 — the one the write
follows — drops it. All four absolute forms, each with its own source register, plus the
page-crossing case where the address still takes the AND-ed high byte and only the byte written
changes. `TestBus` grows one field, `dma_gave_back`.

| | before step 4 | step 4 |
| --- | --- | --- |
| `Unofficial SH* / $93 SHA Indirect,Y` | fail 7 | **pass 1** |
| `Unofficial SH* / $9F SHA Absolute,Y` | fail 7 | **pass 1** |
| `Unofficial SH* / $9B SHS Absolute,Y` | fail 7 | **pass 1** |
| `Unofficial SH* / $9C SHY Absolute,X` | fail 7 | **pass** |
| `Unofficial SH* / $9E SHX Absolute,Y` | fail 7 | **pass** |
| `APU Registers and DMA tests / DMA + $2002 Read` | fail 2 | **pass 1** |
| `AccuracyCoin` | 118 / 147 | **124 / 147** |

Success code 1 on the three that have one is "The Address-Bus-High-Byte-Corruption performed a
bitwise AND upon ABH with both X and A", which is the documented behaviour and not the
magic-number variant. `DMA + $2002 Read` came back with them: its own error 1 is "did not pass the
SLO Absolute, X test" and its timing moved with the instruction stream of the page before it.

`cargo fmt --check` clean, clippy clean (**no new `allow`**), `cargo test --release -p zax-nes -p
zax-harness`: **700 passed, 0 failed**. `cpu_dummy_reads`, `cpu_dummy_writes`, `instr_test-v3`,
`instr_test-v5`, `nes_instr_test`, `instr_misc`, `cpu_exec_space`: all pass, unchanged.

**Save states**: `dma_gave_back` is transient and cleared on load; no save point can fall between
a halted read and the write of the instruction it belongs to, both being inside one `Cpu6502::step`.
The format is unchanged.

## Step 5 — the suites end to end, the campaign document and the guard

### The campaign, ROM by ROM

`zax-harness test-roms <roms> all.json --no-shots`, 331 ROMs, 2 min 51 s:
**214 pass, 5 fail, 0 timeout, 1 unsupported, 111 manual** — MF-4a-ter's four totals exactly.
Both runs' JSON compared row by row: **331 ROMs, the same set, and exactly two whose verdict or
message differs by a single character**, `AccuracyCoin` (93 → 124 of its 147 tests) and
`double_2007_read` (onto the first of the four outputs its own source names). The other 111
`manual` screens and their CRCs are character for character what they were.

The five failures left are the same five: `AccuracyCoin`; `power_up_palette`, whose own readme
says its reference values "are probably unique to my NES"; and the three MMC3 revision A ROMs
that are **required to fail** under REQ-CORE-100.

### The baseline, read the same way

The drive's fix means the before/after would otherwise be measured with two different
instruments. So `3ac52b27` was built from a `git archive` in WSL with **this lot's `coin.rs`**
copied over it, and run:

| | attempts | retired | never run | passed |
| --- | ---: | ---: | ---: | ---: |
| `3ac52b27`, the old drive | 9 | 8 | 32 | **93 / 147** |
| `3ac52b27`, the corrected drive | 1 | 0 | 0 | **116 / 147** |
| MF-4b | 1 | 0 | 0 | **124 / 147** |

So of the 31 rows between 93 and 124, **23 were the drive** and **8 are the core**: step 1's
`Interrupt flag latency`, step 3's `Open Bus` and `Internal Data Bus`, and step 4's five `SH*`
stores. Both numbers are worth stating: the 23 were always passing on this emulator and the
harness was hiding them, and the 8 are behaviour that was wrong.

### The per-test table

**124 / 147**, 23 failed, 0 retired, 0 never run, 1 attempt. The ROM's own tally reads `124/147`.

| page | test | `3ac52b27`, corrected drive | MF-4b |
| --- | --- | --- | --- |
| CPU Behavior | `ROM is not writable` | pass | pass |
|  | `RAM Mirroring` | pass | pass |
|  | `PC Wraparound` | pass | pass |
|  | `The Decimal Flag` | pass | pass |
|  | `The B Flag` | pass | pass |
|  | `Dummy read cycles` | pass | pass |
|  | `Dummy write cycles` | pass | pass |
|  | `Open Bus` | fail 7 | pass **(moved)** |
|  | `All NOP instructions` | pass | pass |
| Addressing mode wraparound | `Absolute Indexed` | pass | pass |
|  | `Zero Page Indexed` | pass | pass |
|  | `Indirect` | pass | pass |
|  | `Indirect, X` | pass | pass |
|  | `Indirect, Y` | pass | pass |
|  | `Relative` | pass | pass |
| Unofficial SLO | `$03 SLO Indirect,X` | pass | pass |
|  | `$07 SLO Zero Page` | pass | pass |
|  | `$0F SLO Absolute` | pass | pass |
|  | `$13 SLO Indirect,Y` | pass | pass |
|  | `$17 SLO Zero Page,X` | pass | pass |
|  | `$1B SLO Absolute,Y` | pass | pass |
|  | `$1F SLO Absolute,X` | pass | pass |
| Unofficial RLA | `$23 RLA Indirect,X` | pass | pass |
|  | `$27 RLA Zero Page` | pass | pass |
|  | `$2F RLA Absolute` | pass | pass |
|  | `$33 RLA Indirect,Y` | pass | pass |
|  | `$37 RLA Zero Page,X` | pass | pass |
|  | `$3B RLA Absolute,Y` | pass | pass |
|  | `$3F RLA Absolute,X` | pass | pass |
| Unofficial SRE | `$43 SRE Indirect,X` | pass | pass |
|  | `$47 SRE Zero Page` | pass | pass |
|  | `$4F SRE Absolute` | pass | pass |
|  | `$53 SRE Indirect,Y` | pass | pass |
|  | `$57 SRE Zero Page,X` | pass | pass |
|  | `$5B SRE Absolute,Y` | pass | pass |
|  | `$5F SRE Absolute,X` | pass | pass |
| Unofficial RRA | `$63 RRA Indirect,X` | pass | pass |
|  | `$67 RRA Zero Page` | pass | pass |
|  | `$6F RRA Absolute` | pass | pass |
|  | `$73 RRA Indirect,Y` | pass | pass |
|  | `$77 RRA Zero Page,X` | pass | pass |
|  | `$7B RRA Absolute,Y` | pass | pass |
|  | `$7F RRA Absolute,X` | pass | pass |
| Unofficial *AX | `$83 SAX Indirect,X` | pass | pass |
|  | `$87 SAX Zero Page` | pass | pass |
|  | `$8F SAX Absolute` | pass | pass |
|  | `$97 SAX Zero Page,Y` | pass | pass |
|  | `$A3 LAX Indirect,X` | pass | pass |
|  | `$A7 LAX Zero Page` | pass | pass |
|  | `$AF LAX Absolute` | pass | pass |
|  | `$B3 LAX Indirect,Y` | pass | pass |
|  | `$B7 LAX Zero Page,Y` | pass | pass |
|  | `$BF LAX Absolute,Y` | pass | pass |
| Unofficial DCP | `$C3 DCP Indirect,X` | pass | pass |
|  | `$C7 DCP Zero Page` | pass | pass |
|  | `$CF DCP Absolute` | pass | pass |
|  | `$D3 DCP Indirect,Y` | pass | pass |
|  | `$D7 DCP Zero Page,X` | pass | pass |
|  | `$DB DCP Absolute,Y` | pass | pass |
|  | `$DF DCP Absolute,X` | pass | pass |
| Unofficial ISC | `$E3 ISC Indirect,X` | pass | pass |
|  | `$E7 ISC Zero Page` | pass | pass |
|  | `$EF ISC Absolute` | pass | pass |
|  | `$F3 ISC Indirect,Y` | pass | pass |
|  | `$F7 ISC Zero Page,X` | pass | pass |
|  | `$FB ISC Absolute,Y` | pass | pass |
|  | `$FF ISC Absolute,X` | pass | pass |
| Unofficial SH* | `$93 SHA Indirect,Y` | fail 7 | pass 1 **(moved)** |
|  | `$9F SHA Absolute,Y` | fail 7 | pass 1 **(moved)** |
|  | `$9B SHS Absolute,Y` | fail 7 | pass 1 **(moved)** |
|  | `$9C SHY Absolute,X` | fail 7 | pass **(moved)** |
|  | `$9E SHX Absolute,Y` | fail 7 | pass **(moved)** |
|  | `$BB LAE Absolute,Y` | pass | pass |
| Unofficial Immediates | `$0B ANC Immediate` | pass | pass |
|  | `$2B ANC Immediate` | pass | pass |
|  | `$4B ASR Immediate` | pass | pass |
|  | `$6B ARR Immediate` | pass | pass |
|  | `$8B ANE Immediate` | pass | pass |
|  | `$AB LXA Immediate` | pass | pass |
|  | `$CB AXS Immediate` | pass | pass |
|  | `$EB SBC Immediate` | pass | pass |
| CPU Interrupts | `Interrupt flag latency` | fail E | pass **(moved)** |
|  | `NMI Overlap BRK` | pass | pass |
|  | `NMI Overlap IRQ` | pass | pass |
| APU Registers and DMA tests | `DMA + Open Bus` | pass | pass |
|  | `DMA + $2002 Read` | pass 1 | pass 1 |
|  | `DMA + $2007 Read` | pass | pass |
|  | `DMA + $2007 Write` | pass | pass |
|  | `DMA + $4015 Read` | pass | pass |
|  | `DMA + $4016 Read` | pass 1 | pass 1 |
|  | `DMC DMA Bus Conflicts` | fail 2 | fail 2 |
|  | `DMC DMA + OAM DMA` | pass | pass |
|  | `Explicit DMA Abort` | fail 2 | fail 2 |
|  | `Implicit DMA Abort` | fail 2 | fail 2 |
| APU Tests | `Length Counter` | pass | pass |
|  | `Length Table` | pass | pass |
|  | `Frame Counter IRQ` | fail 7 | fail 7 |
|  | `Frame Counter 4-step` | pass | pass |
|  | `Frame Counter 5-step` | pass | pass |
|  | `Delta Modulation Channel` | pass | pass |
|  | `APU Register Activation` | fail 6 | fail 6 |
|  | `Controller Strobing` | fail 4 | fail 4 |
|  | `Controller Clocking` | pass 2 | pass 2 |
| CPU Behavior 2 | `Instruction Timing` | pass | pass |
|  | `Implied Dummy Reads` | pass | pass |
|  | `Branch Dummy Reads` | pass | pass |
|  | `JSR Edge Cases` | pass | pass |
|  | `Internal Data Bus` | fail 3 | pass **(moved)** |
|  | `DMA Landing on Write` | pass 1 | pass 1 |
|  | `DMC Reload Timing` | fail 1 | fail 1 |
|  | `DMC Playback Rate` | pass | pass |
| PPU Behavior | `CHR ROM is not writable` | pass | pass |
|  | `PPU Register Mirroring` | pass | pass |
|  | `PPU Register Open Bus` | pass | pass |
|  | `PPU Read Buffer` | pass 10 | pass 10 |
|  | `Palette RAM Quirks` | fail 6 | fail 6 |
| PPU VBlank Timing | `VBlank beginning` | pass | pass |
|  | `VBlank end` | pass | pass |
|  | `NMI Control` | pass | pass |
|  | `NMI Timing` | pass | pass |
|  | `NMI Suppression` | pass | pass |
|  | `NMI at VBlank end` | pass | pass |
|  | `NMI disabled at VBlank` | pass | pass |
| Sprite Evaluation | `Sprite overflow behavior` | pass | pass |
|  | `Sprite 0 Hit behavior` | pass | pass |
|  | `$2002 flag timing` | fail 1 | fail 1 |
|  | `Suddenly Resize Sprite` | pass | pass |
|  | `Misaligned OAM DMA` | pass | pass |
|  | `Arbitrary Sprite zero` | fail 2 | fail 2 |
|  | `Misaligned OAM behavior` | fail 1 | fail 1 |
|  | `OAM Corruption` | fail 2 | fail 2 |
| PPU Misc. | `t Register Quirks` | pass | pass |
|  | `Address $2004 behavior` | fail 6 | fail 6 |
|  | `INC $4014` | pass | pass |
|  | `Rendering Flag Behavior` | pass | pass |
|  | `$2007 read w/ rendering` | fail 2 | fail 2 |
|  | `$2004 Stress Test` | fail 2 | fail 2 |
|  | `$2007 Stress Test` | fail 2 | fail 2 |
| Advanced Background Evaluation | `Attributes As Tiles` | pass | pass |
|  | `Stale BG Shift Registers` | fail 3 | fail 3 |
|  | `BG Serial In` | fail 2 | fail 2 |
|  | `ALE + Read` | fail 2 | fail 2 |
|  | `Hybrid Addresses` | fail 2 | fail 2 |
| Advanced Sprite Evaluation | `Sprites On Scanline 0` | pass 2 | pass 2 |
|  | `Stale Sprite Shift Regs` | fail 3 | fail 3 |
|  | `Frozen OAM2 Increment` | fail 2 | fail 2 |
|  | `Misaligned OAM2 Address` | fail 3 | fail 3 |

### What still fails, and the cheapest correct design for each

23 rows. Four of them are the APU register edges the brief holds back for a later lot, and they
are listed first because three of the others wait on them.

| test | code | what it is, and the cheapest design |
| --- | --- | --- |
| `APU Tests / Frame Counter IRQ` | 7 | "The IRQ flag should not be cleared yet when the APU transitions from a **get** cycle to a **put** cycle" (and code 6: it *is* cleared on put→get). A `$4015` read clears the frame IRQ flag here at once; the hardware clears it on the next put→get boundary, half a CPU cycle later. Cheapest: `Apu2A03::read_status` sets a pending-clear flag instead of clearing, and the frame counter's per-cycle step consumes it on the put→get cycle it already distinguishes. That is a test on a path the APU walks every cycle, so it has to be measured; the brief defers it. |
| `APU Tests / Controller Strobing` | 4 | "Controllers should not be strobed when the CPU transitions from a put cycle to a get cycle" — the same get/put asymmetry on `$4016` bit 0. Cheapest: `InputPorts::write` records the cycle and the strobe takes effect on the next get→put, which the bus already knows from `cycles & 1`. |
| `APU Tests / APU Register Activation` | 6 | "The OAM DMA should be able to read from the APU registers (and mirrors of them) if `$40` is written to `$4016`, and the CPU Address Bus is in the range of `$4000` to `$401F`." The source page `$40` *is* handled (`run_dma`'s `fast` excludes it), but the 2A03's own decode of a DMA read is gated on `internal`, which means "the **halted** address is in `$4000`-`$401F`" and not "the **source** address is". Cheapest: give `dma_read` the source address's own decode as well. Small, but it changes bus-conflict rules this lot did not measure. |
| `APU Registers and DMA tests / DMC DMA Bus Conflicts` | 2 | MF-4a step 3's diagnosis stands: `dma_read` ANDs the whole byte of two drivers, where the hardware keeps the external value on the bits the `$4016`/`$4017` port leaves open. Needs an open-bus mask per controller port, which `InputPorts` does not have — a controller-port change, not a DMA one. |
| `APU Registers and DMA tests / Explicit DMA Abort` | 2 | MF-4a step 3's diagnosis stands: a `$4015` write that takes a fetch back **after** its halt cycle. Needs the `$4015` **stop** delayed two or three cycles the way the start already is, plus an abort check inside the DMA loop. MF-4a wrote it, measured it and took it back out because it moved no row and changed what `$4015` bit 4 reads back for two cycles with nothing pinning the new value. The piece it is missing is `Frame Counter IRQ`'s get/put machinery above. |
| `APU Registers and DMA tests / Implicit DMA Abort` | 2 | The same mechanism with the sample ending rather than a write. |
| `CPU Behavior 2 / DMC Reload Timing` | 1 | "Writing to `$4010` as the DMC timer resets should cause the timer to reload using the new value." `DmcChannel::clock_timer` reloads from the period as it stood before the write. Cheapest: read the period field at the moment of the reload rather than caching it — one field read on the DMC's per-cycle path, so it needs measuring. |
| `PPU Behavior / Palette RAM Quirks` | 6 | "With Greyscale Mode enabled, the lower four bits of the value read should all be zero" (and code 7: the value *written* is unaffected). A `$2007` read of palette RAM returns six bits and ignores PPUMASK's greyscale bit. Cheapest and free: one `value &= 0x30` behind `self.mask & 0x01` in the palette arm of `Ppu2C02::read`, the arm step 2 already touched. **This one is a one-line fix** and is the first thing a follow-up lot should take. |
| `Sprite Evaluation / $2002 flag timing` | 1 | "The flags were not cleared on the correct ppu cycle." The sprite-0 and overflow flags are cleared at dot 1 of the pre-render line here and the test measures the exact dot; which dot it wants needs its `.asm` read. |
| `Sprite Evaluation / Arbitrary Sprite zero` | 2 | "The first processed sprite of a scanline should be treated as sprite zero" — with a non-zero OAMADDR the flag follows the first sprite *evaluated*, not OAM index 0. Cheapest: `sprite_zero_found` becomes "the first sprite copied to secondary OAM came from the address evaluation started at". |
| `Sprite Evaluation / Misaligned OAM behavior` | 1 | The rest of that family: evaluation starting at an OAMADDR that is not a multiple of 4, with the `+4 & $FC` and `+5` rules of its codes 2 to 7. A rewrite of `run_sprite_evaluation`'s address stepping. |
| `Sprite Evaluation / OAM Corruption` | 2 | "OAM Corruption should corrupt a row in OAM by copying the 8 values from row 0 to another row" when rendering is switched off during evaluation. Not modelled at all; a documented 2C02 behaviour, and an addition to the evaluation state machine. |
| `PPU Misc. / Address $2004 behavior` | 6 | "Writing to `$2004` on a visible scanline should increment the OAM address by 4" (code A: by 4 then AND `$FC`; code 7: and not write to OAM). `write_oam` always writes and always increments by 1. Cheapest and nearly free: the visible-and-rendering case inside `Ppu2C02::write`'s arm 4, a register write and not a per-dot path. |
| `PPU Misc. / $2007 read w/ rendering` | 2 | "Reading from `$2007` while rendering is enabled should result in a vertical increment of v." `increment_vram_address` adds 1 or 32; with rendering on the hardware does the coarse-X and fine-Y increments instead. Cheapest: call the two increment helpers `run_dots` already has. Free for a game that does not read `$2007` mid-frame. |
| `PPU Misc. / $2004 Stress Test` | 2 | Reads of `$2004` during evaluation must come from the OAM **buffer** the evaluation holds, not from OAM. Part of the `run_sprite_evaluation` rewrite above. |
| `PPU Misc. / $2007 Stress Test` | 2 | The other half of step 2's rule: with rendering on, the buffer takes "the same value as the resulting read from the background or sprite fetch that occurred on the same ppu cycle". Needs the `$2007` access placed on the PPU's own fetch cadence — the same dot-exact PPU bus `ALE + Read` and `Hybrid Addresses` want. |
| `Advanced Background Evaluation / Stale BG Shift Registers` | 3 | "The background shift registers should not be clocked during H-Blank or F-Blank." They are reloaded at the start of every line here. A change inside `run_dots`' background pipeline — the hottest function in the emulator, and the one family where a fix is **not** obviously free. It should be designed against callgrind from the first line. |
| `Advanced Background Evaluation / BG Serial In` | 2 | "Background shift registers should bring in a 1 into bit 0 when shifted." Same pipeline, same caveat. |
| `Advanced Background Evaluation / ALE + Read` | 2 | "A well timed read from `$2007` should be able to affect the PPU Address Bus during the background read cadence." Needs the CPU's `$2007` access to land on the PPU's current fetch slot. |
| `Advanced Background Evaluation / Hybrid Addresses` | 2 | The same with a `$2006` write. |
| `Advanced Sprite Evaluation / Stale Sprite Shift Regs` | 3 | "The sprite shift registers should not be clocked during F-Blank or H-Blank", the sprite twin of the background family. |
| `Advanced Sprite Evaluation / Frozen OAM2 Increment` | 2 | The secondary-OAM overflow flag gating the OAM2 address increment; part of the evaluation state machine. |
| `Advanced Sprite Evaluation / Misaligned OAM2 Address` | 3 | Reads of `$2004` between dots 321 and 340 must see the OAM2 address the fetch left; same family. |

Read as a whole, the 23 are **four lots and not one**: the APU's get/put edges (4 rows, the
brief's next lot), one free PPU fix (`Palette RAM Quirks`), a group of register-write edges that
are cheap because they are not per-dot (`Address $2004 behavior`, `$2007 read w/ rendering`), and
the sprite-evaluation and shift-register pipelines (13 rows), which live inside `run_dots` and
want a lot of their own with callgrind in the loop.

### No performance regression: measured, not argued

`PERFORMANCE-HISTORY.md` §3's procedure: `git archive` of `3ac52b27` and of this lot's end into
their own fresh directories in WSL2, each with its own `CARGO_TARGET_DIR` and
`CARGO_PROFILE_RELEASE_STRIP=false`, then `valgrind --tool=callgrind --dump-instr=no
--cache-sim=no --branch-sim=no ./zax-harness bench-core <rom> --frames 300 --warmup 0`. The five
ROMs are MF-4a-ter's.

**Whole process:**

| ROM | `3ac52b27` | MF-4b | delta | |
| --- | ---: | ---: | ---: | ---: |
| `spritecans` (295 OAM transfers, no DPCM) | 6 345 020 441 | 6 343 865 227 | **−1 155 214** | −0.018 % |
| `dpcmletterbox` (297 transfers, 19 051 DMC fetches) | 6 310 835 408 | 6 308 290 193 | **−2 545 215** | −0.040 % |
| `apu_test/8-dmc_rates` | 5 810 464 106 | 5 810 635 186 | +171 080 | +0.003 % |
| `M1_P128K` (MMC1) | 5 075 609 367 | 5 074 620 058 | **−989 309** | −0.020 % |
| `M0_P32K_C8K_V` (NROM) | 5 703 590 876 | 5 701 113 096 | **−2 477 780** | −0.043 % |

Four of the five are **faster**, and the fifth is +0.003 %.

**Per symbol, exclusive**, which is the statement that matters. On **all five** ROMs:

| symbol | delta |
| --- | --- |
| `Cpu6502::step` | **0, to the instruction** |
| `Cpu6502::resolve`, `apply_read`, `execute_without_operand` | **0** |
| `NesBus::tick`, `read_cycle`, `write_internal` | **0** |
| `Apu2A03::run_cpu_cycles`, `quiet_cycles` | **0** |
| `Ppu2C02::read_nametable_inner`, `read_pattern_planes` | **0** |
| `Board::ppu_read`, `Board::on_cpu_cycle` | **0** |

and the only `zax_nes` symbols that moved at all are:

| symbol | spritecans | dpcm | 8-dmc_rates | M1 | M0 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `Cpu6502::branch` | +1 379 306 | −82 714 | +2 539 305 | −625 710 | −1 038 587 |
| `NesBus::read_internal` | −1 516 | −42 589 | −35 741 | +912 247 | +390 718 |
| `NesBus::halted_read_cycle` | +590 | +38 696 | +578 | +2 | — |
| `NesBus::dmc_only_dma` | — | +76 204 | +1 144 | — | — |
| `NesBus::run_dma` | 0 | 0 | +15 | 0 | — |
| `Ppu2C02::run_dots` | −2 265 895 | −2 281 257 | −2 142 999 | −1 144 469 | −1 697 501 |
| `Ppu2C02::background_fetch` | −222 430 | −223 938 | −210 366 | −112 346 | −166 634 |

Read row by row:

* **`branch`** is step 1's two instructions on a taken page-crossing branch plus the register
  allocation that moved with them, and **its sign changes with the ROM** — negative on three of
  the five — so most of the ±1 to 2.5 M is placement rather than work. The worst case is +1.0 %
  of `branch` on `8-dmc_rates`, a ROM whose main loop *is* a branch, and that ROM's whole-process
  figure is +0.003 %.
* **`read_internal`** is step 3's five instructions on a read no cartridge answers. It is
  *negative* on the three nes-test-roms and positive on the two **Holy Mapperel** boards, which
  are memory-test ROMs that deliberately read across unmapped space; +912 k on `M1_P128K` is
  +0.36 % of that function, and that ROM's whole-process figure is still **−989 309**.
* **`halted_read_cycle`**, **`dmc_only_dma`** and **`run_dma`** are step 3's and step 4's stores:
  four to five instructions per DMC fetch and one per halted read cycle. +76 204 across
  `dpcmletterbox`'s 19 051 fetches is exactly the four predicted.
* **`run_dots` and `background_fetch` went down on every ROM**, by 1.1 to 2.3 M. Neither was
  touched. `Ppu2C02` grew 16 bytes across steps 2 and 3, which moved every field offset after
  them and let LLVM re-lay-out the dot loop; the gain is layout, and it is reported because it is
  what makes four of five totals negative, not because the lot earned it.

**Golden core.** The DMA timing, the APU and the whole dot loop are cycle-identical to
`3ac52b27`: the campaign's 111 `manual` screens and their CRCs say so, `double_2007_read`
excepted. The games whose output may move are listed per step above and the list is short — a
page-crossing branch whose third cycle reads `$4015` (no commercial game), two `$2007` reads one
CPU cycle apart (no commercial game, and `dma_2007_read`, the ROM built for the one case ordinary
code can reach, prints exactly what it printed before), a `$4015` read within a cycle of a DMC
fetch (bit 5 only, the one bit of `$4015` no hardware guarantees), and the `SH*` stores (no
commercial game).

### The suites

| suite | `3ac52b27` | MF-4b |
| --- | --- | --- |
| `AccuracyCoin` | 93 / 147 (116 read correctly) | **124 / 147** |
| `cpu_interrupts_v2` | 6 / 6 | **6 / 6** |
| `instr_timing`, `branch_timing_tests` | 3 / 3, 3 / 3 | **3 / 3, 3 / 3** |
| `cpu_timing_test6`, `cpu_dummy_reads`, `cpu_dummy_writes`, `cpu_exec_space`, `cpu_reset` | all pass | **all pass** |
| `instr_misc`, `instr_test-v3`, `instr_test-v5`, `nes_instr_test` | 5 / 5, 17 / 17, 18 / 18, 11 / 11 | **unchanged** |
| `ppu_open_bus`, `ppu_read_buffer` | 2 / 2 | **2 / 2** |
| `ppu_vbl_nmi`, `vbl_nmi_timing`, `sprite_hit_tests`, `sprite_overflow_tests` | 11 / 11, 7 / 7, 11 / 11, 5 / 5 | **unchanged** |
| `blargg_ppu_tests_2005.09.15b` | 4 / 5 | **4 / 5**, the same required row |
| `oam_read`, `oam_stress` | 2 / 2 | **2 / 2** |
| `apu_test`, `blargg_apu_2005.07.30`, `apu_reset`, `apu_mixer`, `pal_apu_tests` | 19 / 19, 11 / 11, 6 / 6, 4 / 4, 10 / 10 | **unchanged** |
| `sprdma_and_dmc_dma`, `dmc_dma_during_read4`, `dmc_tests` | 2 / 2, 3 pass 2 manual, 4 manual | **unchanged but `double_2007_read`'s text** |
| `holy-mapperel` | 40 / 40 | **40 / 40** |
| the three MMC3 suites | 5 / 6 each | **5 / 6 each**, the same required failures |
| **whole campaign** | 214 pass, 5 fail, 1 unsupported, 111 manual | **identical; 2 of 331 messages changed** |

### The regression guard

`crates/zax-harness/tests/test_roms_tests.rs` grows from 9 to **13** groups:

| test | what it pins |
| --- | --- |
| `req_core_110_accuracy_coin_keeps_the_rows_mf_4b_turned_green` | the tally as a floor of 124, **no test left unrun or retired** — so the drive cannot silently start hiding tests again — and twelve rows by name |
| `req_core_111_the_cpu_interrupt_and_timing_suites_pass` | `cpu_interrupts_v2` 6, `branch_timing_tests` 3, `instr_timing` 3 |
| `req_core_112_the_ppu_bus_latch_and_the_read_buffer_pass` | `ppu_open_bus`, `ppu_read_buffer` |
| `req_core_110_the_dummy_cycle_suites_pass` | `cpu_dummy_reads`, `cpu_dummy_writes` 2 |

`ZAXNES_TEST_ROMS=… cargo test --release -p zax-harness --test test_roms_tests`: **13 passed**.

### Hot path, this step

Nothing under `crates/zax-nes/src/` changed in step 5, so the generated code is step 4's byte for
byte and there is nothing to compare: the step touches `zax-harness`'s drive (`coin.rs`), the
regression guard and the two documents. `run_count` runs 147 `read_memory` calls once per emulated
frame of an `AccuracyCoin` run and nowhere else; the whole row now takes 2.6 s against 29 s,
because it is one attempt instead of nine.

### The lot, end to end

`cargo fmt --check` clean. `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean, **no new `allow` anywhere**. `cargo test --release -p zax-nes -p zax-harness`:
**704 passed, 0 failed, 7 ignored**, against MF-4a-ter's 695 — nine added, none removed or
`#[ignore]`d.

**No `unsafe`.** **No savestate byte moved**: `data_bus` is the byte `open_bus` was, and every
field the lot adds (`Ppu2C02::read_buffer_prev` and `read_buffer_clock`, `NesBus::ext_bus`,
`ext_cycle` and `dma_gave_back`) is transient, outside the format, and cleared on load — each
with the argument for why nothing observable is lost written next to it. A state written before
this lot, or before MF-4a, loads byte for byte.

**No Q ticket.** Nothing here was ambiguous: every rule has a sentence of `AccuracyCoin`'s own
documentation or of the NESdev wiki behind it, quoted at the code. Q-121 is still free.

| brief item | state |
| --- | --- |
| 1. `Interrupt flag latency` returns and passes; `cpu_interrupts_v2` 6/6, `instr_timing`, `branch_timing_tests` keep passing | **done**, step 1 |
| 2. `$2000`-`$3FFF` mirroring, the PPU I/O latch with its decay, the `$2007` read buffer including `double_2007_read`; `ppu_open_bus`, `ppu_read_buffer` keep passing | **done**, step 2 — the first three were already right and the ROM says so once it is allowed to run; `double_2007_read` was the defect and it now prints a documented output |
| 3. the CPU data bus: `Open Bus`, `Internal Data Bus`, `Implied Dummy Reads`, the five `SH*`; `cpu_dummy_reads`, `cpu_dummy_writes`, `instr_test-v5`, `instr_misc` keep passing | **done**, steps 3 and 4 — `Implied Dummy Reads` was the drive, not the core |
| 4. re-run everything, before/after in the campaign document, the per-test table here, the guard, and what still fails with the cheapest design for each | **done**, this step |
| no performance regression, none at all | **done**: `Cpu6502::step`, `tick`, `read_cycle`, `write_internal`, `resolve`, `run_cpu_cycles`, `quiet_cycles` are **0 to the instruction** on five ROMs, and four of the five whole-process totals are negative |
| safe Rust, no `unsafe` | **done** |
| save states written before this lot keep loading | **done**, format unchanged |
| the APU register edges held back | **held back**: `Frame Counter IRQ`, `APU Register Activation`, `Controller Strobing` and the `$4015` stop delay are listed above with their designs and were not touched |

## Status

- [x] step 1 — the sticky poll of a taken branch
- [x] step 2 — the `$2007` read buffer's delay
- [x] step 3 — the internal and external data buses
- [x] step 4 — the `SH*` stores and a DMA on the dummy read
- [x] step 5 — the suites, the campaign document and the guard
