# Lot MF-4d — the APU and DMA register edges, and `$2002`/`$2004`/`$2007` outside the dot loop

Baseline `64550944` (PR-1's end). `AccuracyCoin` **125 / 147**.

This lot takes the rows that do **not** need the sprite-evaluation or the shift-register pipeline
inside `run_dots`; those are lot MF-4e's.

## Steps

1. `APU Tests / Frame Counter IRQ` (code 7) and `APU Tests / Controller Strobing` (code 4): the
   get/put asymmetry of a `$4015` read and of a `$4016` bit-0 strobe.
2. `CPU Behavior 2 / DMC Reload Timing` (code 1).
3. `APU Registers and DMA tests / Explicit DMA Abort` and `Implicit DMA Abort` (code 2).
4. `APU Registers and DMA tests / DMC DMA Bus Conflicts` (code 2) and
   `APU Tests / APU Register Activation` (code 6).
5. `Sprite Evaluation / $2002 flag timing` (code 1).
6. `PPU Behavior / Address $2004 behavior` (code 6) and `$2007 read w/ rendering` (code 2), only if
   the fix is a read-time computation that does not touch the per-dot path.
7. The suites, `AccuracyCoin`, the whole campaign ROM by ROM, the campaign document and the guard.

## Instrument

PR-1's `rust/tools/perf/`, unchanged except for one bug fix that had to come first.

### The instrument was losing five of its thirteen ROMs

`callgrind-set.sh` fed its ROM list to `while read` through a **pipe**, which in POSIX sh puts the
loop in a subshell. The loop waits only when a batch of `jobs` is full; with 13 ROMs and the
default 8 jobs the last five jobs were started and then the subshell exited without waiting for
them, so the five `comm` ROMs — **R0's five commercial games**, the bar this lot is judged
against — were silently dropped. The outer `wait` cannot see them: they belong to the subshell.

The first run of this lot produced a `cg-` directory with eight `.ir` files and five empty `.log`
files, which is the only trace it leaves. Fixed by feeding the loop from a here-document (a
redirect, so the loop stays in the current shell); the reason is written on the code.

### The baseline, `64550944`

`git archive` into a fresh directory with its own `CARGO_TARGET_DIR`, 300 frames, no warm-up,
whole process:

| ROM | board | instructions |
| --- | --- | ---: |
| `vblnmi` | MMC1 | 3 426 057 297 |
| `oamstr` | NROM | 3 437 654 264 |
| `m1` | MMC1 | 5 013 859 247 |
| `metroid` | MMC1 | 5 252 761 496 |
| `m4` | MMC3 | 5 600 386 484 |
| `m0` | NROM | 5 643 153 415 |
| `smb1` | NROM | 6 130 483 561 |
| `mm2` | MMC1 | 6 136 296 065 |
| `z2` | MMC1 | 6 407 761 347 |
| `readbuf` | CNROM | 6 947 862 929 |
| `duelito` | NROM | 7 493 502 126 |
| `smb3` | MMC3 | 8 066 090 113 |
| `mmc5` | MMC5 | 11 402 181 836 |
| **total** | | **80 957 975 572** |

Against PR-1's own table for the same commit's code (its variant **G** column, built from a tree
that differs only in doc comments) every ROM agrees to within 60 000 instructions — PR-1 step 3
measured that instrument's host-noise floor at about 10⁵ on the whole-process figure, and these
are inside it. The per-symbol `zax_nes` counts, which are what this lot's arguments rest on, have
no such floor: they are exact.

## Step 1 — the get/put asymmetry of `$4015` and of the `$4016` strobe

Two rows, one mechanism on each side of the 2A03: a read of `$4015` whose clear of bit 6 waits
for a get cycle, and a `$4016` strobe whose reload waits for a put cycle. Both are **lazy**:
nothing per cycle has to ask, which is what keeps them off the hot path.

### `APU Tests / Frame Counter IRQ` — the failing message

> 7: The IRQ flag should not be cleared yet when the APU transitions from a "get" cycle to a
> "put" cycle.

and its other half, which the core already passed:

> 6: The IRQ flag should be cleared when the APU transitions from a "put" cycle to a "get" cycle.

### The hardware rule, and its source

The ROM's own commentary states it and even prescribes the implementation. A `$1F`
(`SLO Absolute,X`) over `$4015` reads the register twice, one CPU cycle apart:

```
; (get) [Read Opcode: $1F]        | (put) [Read Opcode: $1F]
; (put) [Read Operand: $15]       | (get) [Read Operand: $15]
; (get) [Read Operand: $40]       | (put) [Read Operand: $40]
; (put) [Read $4015]              | (get) [Read $4015]   <- test 7: no clear yet
; (get) [Read $4015]  <- cleared  | (put) [Read $4015]   <- still set
```

> However, bit 6 will not be cleared until the next "get" cycle. […] you probably want to clear
> bit 6 inside the APU cycle code of your emulator, and not in your "read $4015" code. I suggest
> making a flag for "we are clearing bit 6 on the next APU get cycle" to be set inside the
> "read $4015" code.

So the clear takes effect at the start of the first get cycle **strictly after** the read, which
is `(cycle + 2) & !1` in this core's numbering — a get cycle is an **even** one, the convention
`NesBus::run_dma` and the `$4017` write delay already use.

The **line** is a different grandeur and keeps falling on the read itself, which is what blargg's
`07.irq_flag_timing` and `08.irq_timing` pin. Neither source measures the other's quantity, so
the separation is a provisional choice: **Q-122**.

### `APU Tests / Controller Strobing` — the failing message

> 4: Controllers should not be strobed when the CPU transitions from a put cycle to a get cycle.

### The hardware rule, and the alignment that had to be read twice

`DEC $4016` writes `$41` on its fifth cycle and `$40` on its sixth, holding the line high for one
cycle. The ROM annotates both runs, and **`LDA <$00` is the key to the alignment**: it carries
three tokens for its three cycles, so each `DEC`'s seven tokens are its six cycles plus the cycle
after it.

| test | cycles 1-6, then the next | cycle 5 (raises) | cycle 6 (lowers) | result |
| --- | --- | --- | --- | --- |
| 3 | `(get) (put) (get) (put) (get) [PUT]` `[GET]` | **get** | **put** | strobed, reads `$00` |
| 4 | `(put) (get) (put) (get) (put) [GET]` `[PUT]` | **put** | **get** | not strobed, reads `$FF` |

Both rows are one rule: **the value a write puts on the bus latches at the end of its cycle**, so
the line raised on cycle 5 is high *during cycle 6*, and the reload fires on cycle 6 exactly when
cycle 6 is a put cycle. The cycle-6 write of `$40` only takes the line low from cycle 7 — which
is why it does not undo test 3's reload, and why in test 4, where the reload was waiting for
cycle 7, it does.

So a raising write at cycle `c` arms the reload for `(c + 1) | 1`, the first **odd** (put) cycle
after it, and a lowering write applies a reload armed at or before its own cycle and cancels a
later one. A line held high for more than one cycle necessarily covers a put cycle, so the
`LDA #1 / STA $4016 / LDA #0 / STA $4016` of every game is unchanged.

**This is the one place the first reading was wrong.** Taking the brackets as cycles 5 and 6 — the
two writes, which is the natural thing to highlight — gives the opposite parity, and the row went
*backwards*, from code 4 to code 3. `LDA <$00`'s three tokens are what settle it, and the ROM is
what caught it: the unit test written against the wrong reading passed, because it asserted the
model rather than the ROM's two byte values.

### The change

`apu/mod.rs`: two fields, `frame_irq_status` (bit 6 as a read sees it) and `frame_irq_clear_at`.
`read_status` applies a due clear and otherwise arms one; `compose_status` reads
`frame_irq_status` instead of the line; `peek_status` *looks at* a due clear without applying it
(REQ-CORE-081); a flag raised again by the frame counter cancels an armed clear; a `$4017` write
with bit 6 clears all three without delay (the ROM's test 9 reads `$4015` on the next instruction
and requires the bit already gone).

`input/mod.rs`: one field, `reload_at`, armed by `write` and consumed by `settle_strobe` from the
next read or write. `nes_bus.rs` passes `self.cycles` — the access's own cycle, the bus ticking
before it performs the access — to both.

### Hot path: the generated code after LTO

`cmp4c.py`, post-LTO `zax-harness`, `bin-pr1-v7.s` (the baseline's shipped variant) against
`bin-4d-s1.s`. `Apu2A03` grows 9 bytes and `InputPorts` 8, which moves field offsets across
`NesBus`, so bodies differ on displacements alone; the `offs+hash` column normalises the crate
disambiguator and the offsets at or above 2048.

| symbol | instructions | verdict |
| --- | ---: | --- |
| `Cpu6502::step` | 897 → 897 | **IDENTICAL** |
| `Cpu6502::branch` | 366 → 366 | **IDENTICAL** |
| `Cpu6502::resolve` (2 defs) | 1 725 → 1 725 | **IDENTICAL** |
| `NesBus::read_cycle` | 182 → 182 | **IDENTICAL** |
| `NesBus::tick` | 75 → 75 | **IDENTICAL** |
| `NesBus::read_internal` | 176 → 176 | **IDENTICAL** |
| `NesBus::catch_up_ppu` | 219 → 219 | **IDENTICAL** |
| `NesBus::run_dma`, `dma_read` | 511 → 511, 76 → 76 | **IDENTICAL** |
| `Ppu2C02::run_dots` (2 defs) | 1 936 → 1 936 | **IDENTICAL** |
| `Ppu2C02::background_fetch` | 199 → 199 | **IDENTICAL** |
| `Board::ppu_read`, `on_cpu_cycle`, `on_ppu_address_bus` | all equal | **IDENTICAL** |
| `Apu2A03::quiet_cycles` | 90 → 90 | displacements only (all below 2048), same opcodes in the same order |
| `Apu2A03::run_cpu_cycles` (4 defs) | 873 → **875** | **+2**, named below |
| `NesBus::write_internal` | 2 564 → **2 577** | **+13**, named below |

**The +2** is the whole of it, inside the `frame_irq_pending` branch. The baseline fused two
adjacent bools into one store; three fields now need three:

```
  cmpb $0, 978(%rdi)      ; if self.frame_irq_pending
  je   .LBB
- movw $1, 968(%rdi)      ; pending = false, flag = true
+ movb $0, 978(%rdi)      ; pending = false
+ movw $257, 976(%rdi)    ; flag = true, status = true
+ movq $0, 624(%rdi)      ; clear_at = 0
```

That branch is taken when the frame counter raises the IRQ — about 60 times a second in four-step
mode with the IRQ enabled, and **never** for a game that sets the inhibit bit or uses five-step
mode. The rule requires it: the flag a read sees must be re-raised and an armed clear cancelled.

**The +13** is the strobe arming, and it is **entirely inside the `$4016` write arm** — the hunk
sits after the arm's three address compares and ends in that arm's `jmp`, so no other write
touches it. A game writes `$4016` about twice a frame. The replaced code was
`testb $1, %dil / je .LBB`; the new code loads `cycles` and `reload_at`, tests whether a reload is
due, and computes `(cycle + 1) | 1` with an `incq` / `orq` / `cmovneq`.

**MF-3b rule 1, call sites.** Unchanged, every one: `Cpu6502::end_cycle` 0 → 0,
`Ppu2C02::read_nametable_as` 0 → 0, `read_nametable_inner` 14 → 14, `read_pattern_planes` 3 → 3,
`Board::ppu_read` 0 → 0, `ppu_peek` 4 → 4, `on_ppu_address_bus` 11 → 11, `irq_asserted` 0 → 0,
`on_cpu_cycle` 1 → 1. Nothing gained a first call site.

### Games whose output may move

* **`$4015`**: a game would need two reads of `$4015` **one or two CPU cycles apart** with the
  frame IRQ pending — i.e. a read-modify-write or a page-crossing indexed read over `$4015`.
  Ordinary code reads it once per frame at most, four cycles from anything else. No commercial
  game does this; the illegal `SLO Absolute,X` the ROM uses to make the pair is the only way in.
* **`$4016`**: a game would need a strobe held high for **exactly one cycle**, which needs a
  read-modify-write on `$4016` (`DEC`/`INC`/`ASL $4016`). Every game strobes with two separate
  writes at least two cycles apart, and those are unchanged by construction — the armed cycle is
  at most `c + 2`. No commercial game is expected to move.

### Tests

`apu_frame_counter_tests::req_core_111_a_4015_read_clears_bit_6_on_the_next_get_cycle` drives the
two reads one cycle apart on the bus itself and pins both parities: `(0x40, 0x00)` when the first
read is on a put cycle, `(0x40, 0x40)` when it is on a get cycle.
`req_core_111_the_frame_irq_line_falls_on_the_read_and_not_on_the_get_cycle` pins the other half
of Q-122, on both parities.
`input_port_tests::req_core_022_a_one_cycle_strobe_reloads_only_on_a_put_cycle` reproduces `DEC
$4016`'s last two cycles on the bus and requires the ROM's own two byte values, `$00` and `$FF`.

`cargo fmt --check` clean, `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings`
clean (**no new `allow`**), `cargo test --release -p zax-nes -p zax-harness`: **712 passed,
0 failed** — the baseline's 709 plus these three.

**Save states**: the format is unchanged (REQ-CORE-091). Both fields are derived on load and
nothing observable is lost; the reasoning is on `load_state` in each file. For the APU, the
nearest a second `$4015` read can be to the first across a save point is three cycles — a state
is written between instructions and an instruction's first cycle is its opcode fetch — so the
window can never still be open. For the ports, a strobe that is high reloads on every put cycle
it is high for, so arming the reload at cycle 1 is always right, and a low strobe can have none.

### `AccuracyCoin`: 125 → **126 / 147**

| row | baseline | step 1 |
| --- | --- | --- |
| `APU Tests / Controller Strobing` | fail, code 4 | **pass** |
| `APU Tests / Frame Counter IRQ` | fail, code 7 | fail, **code D** |

`Controller Strobing` passes. `Frame Counter IRQ` moves seven tests forward — 6 and 7, this
step's target, both pass, and so do 8 to 12 — and now stops on a **different rule**:

> D: The IRQ flag was enabled too late. (writing to $4017 on a "get" cycle.)

That is the `$4017` write's sequencer-reset delay (three CPU cycles or four, by APU half-cycle),
not the get/put asymmetry of the read. Measured on the bus, the core's two boundaries — the first
cycle at which a real read of `$4015` sees bit 6, counted from the enabling write — are:

| `$4017` write lands on | first read that sees bit 6 |
| --- | ---: |
| an **even** (get) cycle | write + 29 832 |
| an **odd** (put) cycle | write + 29 831 |

The difference is 1, in the direction the ROM asks (get one cycle later), and codes A, B and C
pass, so the defect is not the difference but a constant offset of one cycle that only the ROM's
own instruction stream exposes — a bus-level probe aligned on cycle parity reproduces neither.
`apu_cycle` is rigidly locked to cycle parity (`skip_quiet_cycles` advances both together), and an
OAM DMA before the writes changes nothing, so the next instrument has to be the ROM's exact
instruction stream rather than a parity-aligned probe. Left as named follow-up work; it is outside
this step's scope and no other row waits on it.

## Step 2 — `$4010` written as the DMC timer reloads

### The failing message

> `CPU Behavior 2 / DMC Reload Timing` 1: Writing to $4010 as the DMC timer resets should cause
> the timer to reload using the new value.

### The hardware rule, and its source

The ROM walks a `$4010` write across the reload one cycle at a time, six iterations, and keys the
six outcomes against hardware ("determined by running this on real hardware"). Its own table:

| iteration | the write lands | the timer reloads with |
| --- | --- | --- |
| 0, 1, 2 | 3, 2, 1 cycles **before** the reload | 428 cycles — the **new**, slow rate |
| 3 | **on the same cycle** as the reload | 428 cycles — still the **new** rate |
| 4, 5 | 1, 2 cycles **after** the reload | 54 cycles — the old, fast rate it already took |

> Amusingly, this test also confirms that if the write to `$4010` happens on the same cycle as the
> timer reloading, then the sample rate changes, and the DMC timer takes the new value.

Its answer key is `40 00 / 40 00 / 40 00 / 40 00 / 00 40 / 00 40` — four "early" then two "late",
so the boundary sits **after** iteration 3 and the same-cycle write counts as early.

### What was actually wrong — not MF-4b's diagnosis

MF-4b's table proposed "`DmcChannel::clock_timer` reloads from the period as it stood before the
write. Cheapest: read the period field at the moment of the reload rather than caching it — one
field read on the DMC's per-cycle path, so it needs measuring." **`clock_timer` already reads
`self.rates[self.rate_index]` at the instant it reloads**, so iterations 0 to 2 were right all
along and there is nothing to pay on the per-cycle path.

The one broken case is iteration 3, and the cause is the order inside a cycle: the bus ticks and
*then* performs the access, so a `$4010` write on cycle N is applied after cycle N's
`clock_timer` has already reloaded with the old period.

### The change

One register-write-time test in `DmcChannel::write`'s `$4010` arm. `timer` standing at exactly
`period - 1` is what says the reload was this cycle — the reload is the only thing that sets that
value and the timer leaves it on the next clock — so the arm re-reloads with the new period:

```rust
let reloaded_this_cycle = self.timer == self.rates[self.rate_index & 0x0F] - 1;
let index = usize::from(value & 0x0F);
self.rate_index = index;
if reloaded_this_cycle {
    self.timer = self.rates[index] - 1;
}
```

**Nothing per cycle, no new field, no save-state change.**

### Hot path: the generated code after LTO

`cmp4c.py`, `bin-4d-s1.s` (step 1) against `bin-4d-s2b.s`. **Every pinned symbol is IDENTICAL,
raw — displacements and all — except `NesBus::write_internal`, 2 577 → 2 587, +10.**

`Apu2A03::run_cpu_cycles` (875 → 875) and `Apu2A03::quiet_cycles` (90 → 90) are byte-identical,
which is the statement that matters: the DMC's whole per-cycle path is inlined into them and it
did not move. `write_internal`'s prologue, its 72-byte frame, its RAM fast path
(`movb %dil, 12040(%rsi,%r12)`) and all **23** of its jump tables are byte-identical too, so the
dispatch did not reshape (MF-3b rule 4).

The +10, by opcode shape, is exactly the rule and nothing else: `andl` (the mask), two
`movl (R,R,4)` (the two period loads), two `decl`, one `cmpl` against `timer`, one `jne`, one
`movl R, N(R)` (the timer store) and one `movq N(R), R` (the rates pointer). A game writes
`$4010` once per sample at most.

**The first shape of this cost a panic path.** Without the `& 0x0F`, LLVM cannot prove
`rate_index` is in range — the field's last writer is another call — and emitted a bounds check
plus a **`core::panicking::panic_bounds_check` call site inside `NesBus::write_internal`**, which
is MF-3b rule 1's stop-the-lot event: +15 instead of +10, with a cold panic block. `rates` is a
`&'static [i32; 16]`, so masking the index to four bits makes it checkless and the mask folds
away. The comment on the line says so, since it is not obvious from the source.
`panic_bounds_check` call sites across the binary: **1 260 → 1 260**. Every other watched call
site unchanged (`end_cycle` 0 → 0, `read_nametable_inner` 14 → 14, `ppu_peek` 4 → 4,
`on_ppu_address_bus` 11 → 11, `on_cpu_cycle` 1 → 1).

### Games whose output may move

A game needs a `$4010` write landing on the exact cycle its DMC timer reloads — a one-in-54 to
one-in-428 coincidence, and only for code that changes the sample rate *while* a sample plays.
Games that set `$4010` once before `$4015` and leave it alone cannot reach it at all. Where it is
reached, the effect is one bit of DPCM played at the new rate instead of the old, which moves the
next fetch by at most `428 - 54` cycles. `sprdma_and_dmc_dma` (`0F 526`) and
`sprdma_and_dmc_dma_512` (`0F 528`) are unchanged, as are all four `dmc_tests` screens and
`dma_4016_read`, `read_write_2007` and `double_2007_read`.

### Tests and ROMs

`dmc_dma_timing_tests::req_core_113_a_4010_write_on_the_reload_cycle_takes_the_new_period` reads
the rule through `cycles_until_dmc_fetch`, which is `timer + 1 + (bits_remaining - 1) * period`,
so `timer == 0` — the cycle before the reload — is `(horizon - 1) % period == 0` and the reload's
own cycle is the one after it. Writing the fast rate there gives `54 * 7`; writing it one cycle
later gives `427 + 54 * 6`, the slow period kept with one cycle of it already spent.

| suite | step 1 | step 2 |
| --- | --- | --- |
| `apu_test`, `pal_apu_tests`, `blargg_apu_2005.07.30`, `apu_reset`, `apu_mixer` (42 ROMs) | 42 / 42 | **42 / 42** |
| `dmc_dma_during_read4`, `dmc_tests`, `sprdma_and_dmc_dma` (14 ROMs) | 8 pass, 6 manual | **8 pass, 6 manual**, every screen identical |
| `AccuracyCoin` | 126 / 147 | **127 / 147** — `DMC Reload Timing` **pass** |

`cargo fmt --check` clean, clippy clean (**no new `allow`**), `cargo test --release -p zax-nes -p
zax-harness`: **713 passed, 0 failed**.

**Save states**: no field added, nothing moved.

## Step 3 — the `$4015` DMC stop, delayed like the start: **not committed**

**No code changed in this step.** What it produced is a measurement that localises the defect to
four bytes, the mechanism behind them, and a design that costs the hot path nothing — which is
what MF-4a asked a later lot to find and did not have.

### The failing message

> `Explicit DMA Abort` 2 / `Implicit DMA Abort` 2: The aborted DMAs did not spend the correct
> number of CPU cycles.

Both rows pass their test 1 (`CheckDMATiming`, `CPY #4`), so the core's DMA *duration* is already
right; what fails is the 16-entry answer key each compares against.

### The instrument this step added

The keys live in RAM — `$50`-`$5F` for the explicit row, `$500`-`$51F` for the implicit one — and
`NesCore::read_memory` can read them, which makes the rows measurable instead of a single error
code. A throwaway test loaded the ROM, ran the drive's own script (180 settle frames, then Start
for two) and snapshotted both ranges on the frame each result byte first went non-zero. To
reproduce it: `result_ExplicitDMAAbort` is `$479` and `result_ImplicitDMAAbort` is `$478`.

### What the measurement says

| X | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| hardware key | 04 | 04 | 04 | 04 | 04 | 04 | **03** | **04** | **01** | **01** | 00 | 00 | 00 | 00 | 00 | 00 |
| this core | 04 | 04 | 04 | 04 | 04 | 04 | **00** | **00** | **00** | **00** | 00 | 00 | 00 | 00 | 00 | 00 |

**Twelve of the sixteen already agree.** The DMA that runs in full costs four cycles (X = 0 to 5)
and the write that lands early enough cancels it outright (X = 10 to 15); only the four-cycle
window between them is wrong, and in one direction — this core cancels where the hardware does
not.

`Clockslide64_Minus_A` leaves `A = X` cycles before the DMA, so **a larger X is a write that
lands earlier**. Read that way the key is one rule:

* X = 0 to 5 — the write lands *after* the DMA has run: nothing to stop, four cycles.
* X = 6, 7 — the write lands on or just after the halt: too late, so the DMA still runs. Three
  cycles or four, by whether its alignment cycle was needed.
* X = 8, 9 — the write lands just *before* the halt. The stop has not reached the DMA unit yet, so
  the halt still happens and the DMA is then dropped: **one cycle spent**.
* X = 10 to 15 — the write lands two or more cycles before: the stop arrives first and no DMA
  happens at all.

So the stop reaches the DMA unit two or three cycles after the write, exactly the way the **start**
already does (`DmcChannel::set_enabled` returns `if apu_cycle { 2 } else { 3 }`), and a request
caught inside that window is dropped *after* its halt cycle rather than before it.

The implicit row's `$500`-`$51F` came back all zeroes, so its loop stored nothing: it is blocked
earlier than the key comparison and its own error code 3 names a mechanism this core does not have
at all — "The 1-cycle DMA should not get delayed by a write cycle, rather it just shouldn't occur
in that case", the 1-cycle DMA a reload makes when it lands just after a load on a one-byte sample
with looping off. That is a second mechanism, not a calibration of the first.

### The design, and why it costs nothing

MF-4a wrote the obvious shape — delay the **channel's stop** — measured it, and took it back out
because "it changes what `$4015` bit 4 reads back for two to three cycles after a disable, and
nothing in the suites pins the new behaviour". That objection is right and it is avoidable:
**delay the cancellation of the request, not the stop of the channel.**

* `DmcChannel::set_enabled(false)` keeps clearing `bytes_remaining` at once, so `$4015` bit 4
  reads back exactly as it does today and no unpinned behaviour is introduced. MF-4a's reason for
  reverting disappears.
* The bus gains one cycle stamp, `dmc_stop_cycle`, set by the `$4015` disable to
  `cycles + (2 or 3)`. `NesBus::after_apu_write` stops clearing `DMA_DMC | DMA_DMC_WAIT |
  DMA_DUMMY` immediately and stamps instead.
* `promote_dmc_wait` promotes a held request when `needs_dmc_dma() || halt_cycle <
  dmc_stop_cycle` — the second term is what lets X = 8 and 9 start at all.
* The DMA loop drops an in-flight `DMA_DMC` once `cycles >= dmc_stop_cycle`, which spends one
  cycle when the halt has just run.

Every consult site is inside `run_dma`, `dmc_only_dma` and `promote_dmc_wait` — all of them
`#[inline(never)]` or reached only from the DMA path, which `dma != 0` gates — so a game that
runs no DMA pays nothing, and a game that runs OAM transfers pays at most one test per
state-machine iteration (the fast loop `copy_oam_pairs` covers the bulk of a transfer and would
not be touched). `Apu2A03::write`'s return would become a three-way `None` / `Load(u32)` /
`Stop(u32)` rather than a bare `u32`, so the two delays cannot be confused.

### Why it is not in this commit

The remaining four bytes have to come out **exactly** — including the non-monotone `03` then `04`,
which is an alignment-cycle parity — and each calibration turn is a rebuild plus a ROM run. The
change lands in the one subsystem that `sprdma_and_dmc_dma`, `sprdma_and_dmc_dma_512`,
`dma_4016_read`, `read_write_2007`, `double_2007_read` and the whole golden core rest on, and the
implicit row needs a second mechanism on top. Weighed against the five rows of steps 4 to 6 and
the campaign of step 7, the honest call was to leave the two abort rows measured and designed
rather than half-changed: **this is a scope decision, not a hot-path verdict** — the design above
is free, and it is the next thing a follow-up lot should take, with the probe recipe above as its
instrument.

## Step 4 — what a DMA reads when the ports and the APU drive the bus: **not committed**

**No code shipped.** A merge was written and measured: it fixes all sixteen of the bytes the row
is *about*, moves no row, and perturbs the bus chain the ROM's later bytes ride on — so it came
back out, for the same reason MF-4a took its own attempt out. What this step leaves is the table,
the one-line diagnosis and the residual.

### The failing messages

> `APU Register Activation` 6: The OAM DMA should be able to read from the APU registers (and
> mirrors of them) if `$40` is written to `$4016`, and the CPU Address Bus is in the range of
> `$4000` to `$401F`.

> `DMC DMA Bus Conflicts` 2: The DMC DMA did not correctly emulate the bus conflict with the APU
> registers. (and its 4: "Reading from the controller port should have some bits that are open
> bus. These bits are different depending on the model of the console.")

### The instrument, and what it showed

`APU Register Activation` 6 copies OAM to `$500`-`$5FF` and checks it row by row, and
`AccuracyCoin.asm` prints the expected 256 bytes in its own comment. Reading those back with
`NesCore::read_memory` turns the row from one error code into a 256-byte diff. **Every byte agreed
with hardware except one per odd row** — offset `$x6`:

```
  00  50 50 40 50 50 50 40 50 50 50 40 50 50 50 40 50     (agrees)
* 10  50 50 40 50 50 44 40 40 ...        key  ... 44 41 40 ...
* 30  40 40 40 40 40 04 00 00 ...        key  ... 04 01 00 ...
* 50  00 00 00 00 00 04 00 00 ...        key  ... 04 01 00 ...   (and so on, every odd row)
```

The `$x5` bytes — `44` then `04` — agree, which is step 1's `$4015` clear timing measured a second
way: the frame IRQ flag the DMA's own read of `$4015` clears, seen 32 bytes later.

### The one-line diagnosis, which MF-4b had half-right

MF-4b proposed "the 2A03's own decode of a DMA read is gated on `internal`, which means the
**halted** address… and not the **source** address", and that is not it: the ROM says in so many
words that the gate *is* the 6502 bus — "Reading from the APU registers requires the 6502 address
bus to be in the range of `$4000` through `$401F`" — which is exactly what `internal` means, and
the core's tests 4 and 5 pass on it.

What the table actually turns on is that **the transferred page is `$50`, not `$40`**. The test
executes its `STA $4014` from `$3FFE` so that the high byte of its own operand comes out of open
bus, and `A` is `$50`:

```
[$3FFE = $8D  STA abs] [$3FFF = $14] [$4000 -> open bus $40] => STA $4014, A = $50
```

So the DMA reads `$5000`-`$50FF`, and **every** `$4016`/`$4017` byte of it is a *mirror* read —
`$5016`, `$5036`, `$5056`, … — never the register itself. All of them land in `dma_read`'s
`!same` arm, which is MF-4a's diagnosis exactly: it returns `self.read_internal(address) & value`,
and the AND loses the controller bit that the open-bus byte has clear.

### What was tried, and the residual

`(value & 0x01) | (self.read_internal(address) & 0xFE)` — the port register drives bit 0 and
leaves the rest to whatever else is on the bus. That is the rule the four distinct key bytes ask
for (`$41` at `$5016` over an open bus of `$40`; `$01` at `$5036` once the bus has gone to `$00`;
`$40` and `$00` for the port-2 mirrors), and **it produces all sixteen of them correctly.**

It still fails, one byte later. The bytes *after* each port read come back `44`/`04` where the key
says `40`/`00`: bit 2 — the triangle's bit, out of the `$4015` read five bytes earlier — survives
on the internal bus across the conflicted read, where on hardware the merged byte replaces it. So
the residual is **not** the merge mask but *which byte a conflicted mirror read leaves on the
internal data bus*, a second rule in the same arm. A first attempt with the mask at `0x1F`
instead of `0x01` was worse still: it zeroed bits 1 to 4 of the open-bus byte, which broke the
chain feeding the `$4000` fetch above, the `STA` stored somewhere other than `$4014`, and no
transfer happened at all.

`AccuracyCoin` with the merge in: **127 / 147, unchanged**, `APU Register Activation` still code 6
and `DMC DMA Bus Conflicts` still code 2. A change that moves no row while altering what a DMA
leaves on the bus — which `Internal Data Bus`, `Open Bus` and `Implied Dummy Reads` all read — is
not worth shipping unpinned, so it is out. The next lot wants the two rules together, with the
256-byte diff above as its instrument.

## Step 5 — the dot on which `$2002`'s sprite flags clear

### The failing message

> `Sprite Evaluation / $2002 flag timing` 1: Verify the timing in which the sprite zero and
> sprite overflow flags are cleared.

### The hardware rule, and its source

MF-4b guessed this one was about *which dot* clears the flags and that the dot "needs its `.asm`
read". The `.asm` says the opposite, in as many words, and it is not a dot at all:

> Just so you are aware, all the flags are cleared on dot 1 of the pre-render line. This is a
> fact. […] If you are failing this test, you might be inclined to scoot the sprite flags over,
> so they get cleared on dot 0, but that's not the proper solution. Here's what's going on:
> Reads from $2002 will latch the state of the vblank flag at the beginning of the read (when M2
> goes high), but the sprite flags are **not** latched, so the value you get back is the state of
> the sprite flags at the **end** of the read (when M2 goes low). On a revision G CPU, M2 has a
> duty cycle of 15/24 […] the sprite flags are read approximately **1.875 PPU cycles** after the
> vblank flag is read.

So the clear stays where it was — dot 1 of the pre-render line, for all three flags — and what
moves is the **sampling instant of a read**: bit 7 as of the read's own dot, bits 5 and 6 as of
about 1.9 dots later.

`ReadFrom2002WithExactTiming` walks four reads one dot apart, starting at scanline 260 dot 339,
and the key is `E0 E0 80 00` (`E0 80 80 00` also accepted, a console sampling a dot earlier):

| read lands on | sprite flags sampled at | key | why |
| --- | --- | --- | --- |
| 260 / 339 | 260 / ~340.9 | `E0` | before the clear |
| 260 / 340 | pre-render / ~0.9 | `E0` | **still** before dot 1 |
| pre-render / 0 | pre-render / ~1.9 | `80` | past the clear; bit 7 is the one dot 0 had |
| pre-render / 1 | pre-render / ~2.9 | `00` | the clear is behind both |

Flags latched at the read's own dot — what this core did — give `E0 E0 E0 00`: **one byte**, the
third.

### The change

One arm, no new state. `Ppu2C02::read`'s register-2 arm masks bits 5 and 6 off the byte it
returns when the read lands on **dot 0 of the pre-render line**, which is the only position
within 1.9 dots before the clear. The test on the dot is **shared** with the vblank-suppression
quirk that was already there — both live on dot 0 and differ only in the scanline — so the two
are now one `dot == 0` gate with two scanline arms instead of a separate compare each.

### Hot path: the generated code after LTO

`cmp4c.py`, `bin-4d-s2b.s` against `bin-4d-s5.s`. **Every pinned symbol IDENTICAL, raw** —
including `Ppu2C02::run_dots` (1 936 → 1 936), `NesBus::read_internal` (176 → 176, so the call
into the PPU is unchanged) and `NesBus::catch_up_ppu` (219 → 219). The one symbol that moves is
`Ppu2C02::read`, 257 → **263**, +6.

The six are in a basic block of their own, reached by the `dot == 0` branch and branchless inside
it:

```
movl   #(%rcx), %r8d     ; scanlines_per_frame
decl   %r8d              ;   - 1  = pre_render_scanline()
movzbl %sil, %r10d       ; the byte as composed
andb   $-97, %sil        ; status &= !0x60
cmpl   %r8d, %edx        ; scanline == pre-render?
cmovnel %r10d, %esi      ; and pick
```

**A `$2002` read on any other dot does not reach them**, and its own path did not grow: the
merged gate tests the dot once where the old code tested the scanline and the dot, so the common
read — a vblank spinner's, which is what PR-1's `duelito` and `vblnmi` are in the set for — pays
one compare as before. The rest of the function's diff is `apply_decay`'s unrolled loop being
re-laid-out around the new block (`-4` register moves, one branch flipped from `jge` to `jl`);
no instruction was added to it.

Call sites unchanged, every one of MF-3b rule 1's list.

### Games whose output may move

A game must read `$2002` on **dot 0 of the pre-render line** — one dot in 89 342 — and care about
bits 5 or 6 of that read. A vblank poller reads bit 7 and is unaffected even when it lands there;
sprite-0 timing code polls bit 6 during the visible lines, hundreds of scanlines away. **No
commercial game is expected to move**, and the suites agree: `sprite_hit_tests` 11 / 11,
`sprite_overflow_tests` 5 / 5, `ppu_vbl_nmi` 11 / 11, `vbl_nmi_timing` 7 / 7, every screen
identical.

### Tests and ROMs

`ppu_rendering_tests::req_core_112_a_2002_read_on_the_pre_render_dot_0_already_sees_the_sprite_flags_cleared`
sets both flags (sprite 0 over a solid background, nine sprites on the line), then aligns the bus
three dots ahead of the dot it wants — a read cycle covers exactly three — and **asserts the
position it achieved**, so a phase that cannot land on the dot fails loudly instead of measuring
a neighbour. Both of the ROM's middle readings are pinned: `$80` at pre-render dot 0, `$E0` at
dot 340 of the line before.

| suite | step 2 | step 5 |
| --- | --- | --- |
| `ppu_vbl_nmi`, `vbl_nmi_timing` (19 ROMs) | 19 / 19 | **19 / 19** |
| `sprite_hit_tests`, `sprite_overflow_tests` (18 ROMs) | 17 pass, 1 manual | **17 pass, 1 manual** |
| `oam_read`, `oam_stress` (4 ROMs) | 3 pass, 1 manual | **3 pass, 1 manual** |
| `ppu_*` (20 ROMs) | 19 / 20, `power_up_palette` required | **19 / 20**, same row |
| `AccuracyCoin` | 127 / 147 | **128 / 147** — `$2002 flag timing` **pass** |

`cargo fmt --check` clean, clippy clean (**no new `allow`**), `cargo test --release -p zax-nes -p
zax-harness`: **714 passed, 0 failed**.

**Save states**: no field added, nothing moved.

## Status

- [x] step 1 — the get/put asymmetry of `$4015` and of the `$4016` strobe
- [x] step 2 — `$4010` written as the DMC timer reloads
- [~] step 3 — the `$4015` DMC stop, delayed like the start: **measured and designed, not
  committed** (12 of 16 key bytes already agree; the four that do not are named above)
- [~] step 4 — what a DMA reads when the ports and the APU drive the bus: **the mirror merge is
  found and all sixteen key bytes come out, but a second rule on the same arm remains; not
  committed**
- [x] step 5 — the dot on which `$2002`'s sprite flags clear
- [ ] step 6 — `$2004` on a visible line, and `$2007` read with rendering on
- [ ] step 7 — the suites, the campaign and the guard
