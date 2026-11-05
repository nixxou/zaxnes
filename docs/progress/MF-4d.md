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

## Status

- [x] step 1 — the get/put asymmetry of `$4015` and of the `$4016` strobe
- [ ] step 2 — `$4010` written as the DMC timer reloads
- [ ] step 3 — the `$4015` DMC stop, delayed like the start
- [ ] step 4 — what a DMA reads when the ports and the APU drive the bus
- [ ] step 5 — the dot on which `$2002`'s sprite flags clear
- [ ] step 6 — `$2004` on a visible line, and `$2007` read with rendering on
- [ ] step 7 — the suites, the campaign and the guard
