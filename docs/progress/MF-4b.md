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

## Status

- [x] step 1 — the sticky poll of a taken branch
- [x] step 2 — the `$2007` read buffer's delay
- [ ] step 3 — the internal and external data buses
- [ ] step 4 — the `SH*` stores and a DMA on the dummy read
- [ ] step 5 — the suites, the campaign document and the guard
