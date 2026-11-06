# Lot MF-6b — the boards with a CPU-cycle or scanline IRQ, and the SOMARI-P family

The baseline and the performance reference are both **`9f9d9d41`**, the end of MF-6a2. The plan
is `docs/review/MF-6-inventory.md` §5 "MF-6b" (boards, steps 0-7, tests per board, titles) and
§4 (the hooks and the hot paths), with the brief's points taking precedence where they differ —
and one of them adds a step: **mappers 116, 14 and 238**, the Gouder SOMARI-P board, which §6 of
the inventory left out pending Q-130 question 1 and which R0 has since decided.

Read first: `docs/progress/MF-6a.md`, `MF-6a2.md` and `MF-5b-fix.md`;
`rust/docs/PERFORMANCE-HISTORY.md` §5.10 to §5.12.

## Steps

0. The plan, the baseline on both instruments, and `NesBus::irq_asserted`'s **absence** from the
   post-LTO dump recorded explicitly — the figure every later step is read against.
1. **73 (VRC3)**, extending `VrcIrq`.
2. **65 and 67**, the two plain CPU-cycle counters.
3. **105 (NES-EVENT)**, as a parameter of `Mmc1`.
4. **48 (TC0690)** and its A12 watcher parameter.
5. **158**, as a parameter of `Rambo1`.
6. **116, 14 and 238**, the SOMARI-P board and its Huang ASIC (the brief's added step).
7. Save states; the four sweeps; `req_core_100_every_required_mapper_is_registered`.
8. The suites, the campaign, the documents.

## The bar, at every step

Both instruments, at **every** step, each on a clean `git archive`-equivalent build in its own
directory with its own target directory (`make-tree.sh` + `build-clean.sh`):

* **level 2** — `callgrind-l2.sh` + `l2-diff.py --bar`, the five games with their 3D packs, 600
  frames after 120 of warm-up, inputs pressed. Every game at `9f9d9d41`'s **total and core** or
  lower.
* **PR-1's set** — `callgrind-set.sh` + `callgrind-diff.py`, thirteen ROMs, 300 frames, no pack.
  Every ROM at `9f9d9d41`'s count or lower.

Total-only rises at level 2 whose per-function diff lands entirely in Lua's `ltable.c` or in
`malloc` are layout noise — MF-6a2 step 2 measured that column's run-to-run floor at **±2.7 M**
against **0** on `core` — and the `core` figure decides. After **each** new board, the post-LTO
check (`cmp4c.py`, `prologues.py`, `blocks.py`, `callsites.py`) that `irq_asserted`,
`zone_offset` and `execute_without_operand` are still inlined into `Cpu6502::step` (0 / 0 / 0).

**Stop-the-lot condition**: a figure above the reference that the step cannot fix — revert that
board, record why and its design, and go on with the next one. MF-6a2 step 3 is the model.

### The labels are MF-6a's shape, deliberately

`rust/target-mf/mf6b.sh` is this lot's driver — scratch, not committed, as `mf6a.sh` and
`mf62.sh` were, because `rust/target-*` is never staged. Every label it is given has the form
**`mf6b-<step>-<8 chars>`**, 15 characters, the length of MF-6a's `mf6a-<step>-<hash>` and
MF-6a2's `mf62-<step>-<hash>`. MF-6a step 0 measured why: the same commit built at two paths of
different lengths differs by up to 1.56 M instructions on a level-2 `total`, all of it in
`load_profile` and `lua_hooks`, which open files and run Lua whose paths are longer or shorter.
Keeping the length identical makes every figure below directly comparable with the tables in
`MF-6a.md` and `MF-6a2.md`.

## Step 0 — the baseline, and the symbol that is not there

### The commands

Host (Git Bash):

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 9f9d9d41 rust/target-mf/tmp/mf6b-0-9f9d9d41.tar

WSL2 Ubuntu-24.04 (`rustc` 1.99.1, `valgrind-3.22.0`, 24 cores), through the driver, which is
four lines of environment around `rust/tools/perf/`'s own scripts — `PR1_WORK=$HOME/mf6b` so
this lot's scratch is its own:

    sh rust/target-mf/mf6b.sh build mf6b-0-9f9d9d41      # 39 s, then the tree is deleted
    sh rust/target-mf/mf6b.sh set   mf6b-0-9f9d9d41      # 13 ROMs, 8 jobs
    sh rust/target-mf/mf6b.sh asm   mf6b-0-9f9d9d41      # post-LTO asm, into target-mf/asm/
    sh rust/target-mf/mf6b.sh l2    mf6b-0-9f9d9d41      # 5 games, 5 jobs, about six minutes

Every one in the **foreground** with a long timeout, never backgrounded: a headless session ends
as soon as it stops answering, and a process backgrounded inside a `wsl.exe` call is killed when
the call returns (MF-6a step 0). The `wsl.exe` invocations go through PowerShell and not Git
Bash, which rewrites a `/mnt/g/...` argument into `C:/Program Files/Git/mnt/g/...`.

### Instrument 1: the five level-2 games — character for character MF-6a2's shipped table

`stages.txt` of `l2-mf6b-0-9f9d9d41`, beside MF-6a2 step 2's V3 column, which is `9f9d9d41`'s
code:

| game/stage | MF-6a2 step 2 (V3) | **`mf6b-0-9f9d9d41`** | delta |
| --- | ---: | ---: | ---: |
| `metroid/core` | 11 147 898 492 | **11 147 898 492** | **0** |
| `smb1/core` | 11 515 980 923 | **11 515 980 923** | **0** |
| `zelda2/core` | 12 777 709 570 | **12 777 709 570** | **0** |
| `mm2/core` | 12 043 306 623 | **12 043 306 623** | **0** |
| `smb3/core` | 15 318 626 455 | **15 318 626 455** | **0** |
| `metroid/total` | 13 736 160 637 | 13 736 176 888 | +16 251 |
| `mm2/total` | 18 982 779 228 | 18 982 867 263 | +88 035 |
| `smb1/total` | 13 587 306 153 | 13 589 021 252 | +1 715 099 |
| `zelda2/total` | 15 362 494 367 | 15 362 221 166 | −273 201 |
| `smb3/total` | 24 190 909 323 | 24 192 639 363 | +1 730 040 |

**All five `core` figures are identical to the instruction**, four measurement sessions and three
scratch roots apart. The `total` column moves by up to 1.73 M, which is inside the ±2.7 M
run-to-run floor MF-6a2 step 2 measured on that column with the same binary at the same path.
`core` is the column this lot is read on, and the five figures above are the bar.

### Instrument 2: PR-1's thirteen

| ROM | board | MF-6a2 step 2 (V3) | **`mf6b-0-9f9d9d41`** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 310 526 412 | **3 310 470 388** | −56 024 | −0.002 % |
| `oamstr` | NROM | 3 319 502 016 | **3 319 466 313** | −35 703 | −0.001 % |
| `m1` | MMC1 | 4 886 656 025 | **4 886 651 514** | −4 511 | −0.000 % |
| `m0` | NROM | 5 521 263 906 | **5 521 219 795** | −44 111 | −0.001 % |
| `m4` | MMC3 | 5 473 792 342 | **5 473 783 790** | −8 552 | −0.000 % |
| `metroid` | MMC1 | 5 116 127 287 | 5 116 149 734 | +22 447 | +0.000 % |
| `smb1` | NROM | 5 996 876 627 | 5 996 882 321 | +5 694 | +0.000 % |
| `mm2` | MMC1 | 6 020 784 916 | **6 020 725 910** | −59 006 | −0.001 % |
| `z2` | MMC1 | 6 292 636 422 | **6 292 582 908** | −53 514 | −0.001 % |
| `readbuf` | CNROM | 6 836 835 668 | **6 836 830 137** | −5 531 | −0.000 % |
| `duelito` | NROM | 7 383 061 072 | 7 383 094 194 | +33 122 | +0.000 % |
| `smb3` | MMC3 | 7 950 803 409 | **7 950 795 166** | −8 243 | −0.000 % |
| `mmc5` | MMC5 | 11 289 616 358 | **11 289 605 855** | −10 503 | −0.000 % |
| **total** | | **79 398 482 460** | **79 398 258 025** | **−224 435** | −0.000 % |

Every ROM is within ±59 006 of MF-6a2's own column, which is **inside the ±57 k
`luaH_getshortstr` floor §5.11 measured**, per ROM. The right-hand column is **the bar for the
whole lot**: every step at that number or lower, ROM by ROM.

### Instrument 3: the post-LTO dump, and the symbol MF-5b taught this project to look for

`shape.py lin-mf6b-0-9f9d9d41.s`:

| symbol | def / sites / instructions |
| --- | --- |
| `Board::irq_asserted` | **0 / 0 / 0** |
| `Wide::zone_offset` | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | **0 / 0 / 0** |
| `Cpu::execute` | 0 / 0 / 0 |
| `Cpu::step` | 1 / 2 / **3 443** |
| `Cpu::resolve` | 2 / 6 / **1 854** |
| `Cpu::branch` | 1 / 1 / **418** |
| `NesBus::read_cycle` | 1 / 65 / 181 |
| `NesBus::tick` | 1 / 95 / 82 |
| `NesBus::read_internal` | 1 / 17 / 175 |
| `NesBus::write_internal` | 1 / 10 / **3 103** |
| `Ppu::run_dots` | 1 / 2 / **1 907** |

Every figure is MF-6a2's, to the instruction.

**The absence is the measurement.** §5 step 0 asks for "the baseline's `NesBus::irq_asserted`
figure recorded **before** anything is added", and MF-5b's lesson is that this symbol's absence
from the dump is the signal rather than its value. So it is recorded as a count and not as a
reading:

    grep -c irq_asserted lin-mf6b-0-9f9d9d41.s   ->  0

**Zero occurrences in 35 736 933 bytes of assembly.** Not one definition, not one call site, not
one relocation: `<NesBus as CpuBus>::irq_asserted`, `Cartridge::irq_asserted` and
`Board::irq_asserted` are all three folded into `Cpu6502::step` by the four `#[inline(always)]`
MF-5b put on the chain, and `callsites.py` reports `0 → 0` for all three. **The first step of
this lot whose dump contains the string at all has an outlining to explain**, and that is the
whole of §4.2's risk in one `grep`.

Beside it, the figures a new board's arm can move, measured on the same dump:

| symbol | instructions at `9f9d9d41` | why it is watched |
| --- | ---: | --- |
| `Board::ppu_peek` | **271** | the fourth capped hook (MF-6a2 step 3: a TQROM arm took it to 583 and cost `m4` 3.4 M) |
| `Board::ppu_read` | 354 | one arm per board, unlatched (§4.4) |
| `Board::on_cpu_cycle` | 396 | §4.1: the arm that matters is the empty one |
| `Board::on_ppu_address_bus` | 156 | 57 600 calls a frame, latched (§4.3) |
| `NesBus::catch_up_ppu` | **188** | inlines the whole `Board::ppu_irq_horizon` dispatch (MF-6a2 step 2) |
| `Ppu2C02::read_nametable_inner` | 114 | the two-body cap of Q-123 |
| `NesBus::write_internal` | 3 103 | every board's register decode |
| `Mmc1::update_banks` | 215 | five of thirteen ROMs are MMC1s (step 3's stop condition) |

`Board::on_cpu_cycle`'s prologue, which §4.1 item 2 makes the thing to read after steps 1-4:

    pushq %r14 ; pushq %rbx ; pushq %rax ; movzbl 1368(%rdi), %eax ; subb $2, %al ; movl $5, %ecx

the literal `$5` being the MMC5's niche tag (§4.9).

### The design the lot starts from, and the one measurement it rests on

§5 gives six boards and §4.2 prices each of them at one more arm of `NesBus::irq_asserted`:
"five new `irq_asserted` arms that answer *yes*, on a dispatch of thirty-two arms that outlined
at the thirty-second". **This lot does not spend five.** MF-6a step 2 built `SmallBoard` — nine
boards behind **one** `Board` discriminant — and measured the alternative at 0.067 % to 0.131 %
on eight of thirteen ROMs; its own note says why the IRQ methods are *not* forwarded there:
"forwarding them would give each of those dispatches a *distinct body* — an inner match returning
`false` is not the same code as `false`".

The mirror of that sentence is this lot's design. A second shared variant, whose boards all
answer **yes** to `irq_asserted` and all have something to do on a CPU cycle, pays one body in
each of those dispatches **for the whole lot** instead of one per board. The boards that need it
pay an inner match; no other board pays anything. So:

* **one new `Board` variant** for 65, 67, 73, 48 and 105 (and 116 in step 6), each an inner
  variant of an `IrqBoard` enum that forwards `irq_asserted`, `on_cpu_cycle`,
  `on_ppu_address_bus` and `ppu_irq_horizon`;
* `chr_view` answers `None` for the whole variant, which is the body the MMC3, the MMC5, the
  RAMBO-1 and `SmallBoard` already give, so `Ppu2C02::run_dots` gains nothing to lay out;
* 158 is a parameter of `Rambo1` and 105 reuses `Mmc1`, so neither adds a variant at all.

Two of §5's prerequisites turn out to be **already built**, which step 4 therefore does not have
to invent:

* `A12Watcher::filter_dots` is already a construction parameter with `min_rise_interval()`
  derived from it (`a12_watcher.rs:33`, `:64`) — MR-64-3's fix landed in MF-4c/MF-5a, so §5 step
  4's "`FILTER_DOTS` becomes a construction parameter" is done and `Mmc3::dots_per_clock` already
  moves with it;
* MR-64-4's delayed assertion is in the tree as `Rambo1::irq_delay`, counted down in
  `on_cpu_cycle` (`rambo1.rs:21-26`, `:233-242`). The TC0690's "+≈4 CPU cycle delay" is the same
  shape and reuses it rather than recording a gap.

[PROGRESS] MF-6b step 0/8 done: the plan, both instruments reproduced at `9f9d9d41` — all five level-2 cores identical to the instruction, set total −224 435 — and `irq_asserted` nowhere in the dump.

## Step 1 — mapper 73, the VRC3, and the one body that pays for the whole lot

### The design: one `Board` variant for the lot, not one per board

§5 step 1 asks whether "a width parameter on `VrcIrq` serves it", so that "mapper 73 reuses
mappers 21-27's and 24/26's existing `irq_asserted` arms and §4.2's risk does not arise for it".
**It does not, and the step measured why** — twice, because the answer has two halves and only the
second one is about the counter.

**The arm cannot be reused at all, by any board.** `Vrc2Vrc4`'s `irq_asserted` arm is
`self.irq.asserted()`, which compiles to one `cmpb $0, <offset>(%r14)` at the offset of that
board's `pending` byte. A second board's arm is the same instruction at a *different* offset, so
it is a different block whatever the source says. The post-LTO dump shows the dispatch as
**twelve such blocks**, one per board that can assert, plus the shared "no" arm — and what MF-6a
step 1 calls "the twenty-odd boards that answer no share one arm precisely because their bodies
are identical" is the same sentence read forwards: a body is shared while it is the same
*instruction*, not while it is the same *source*. A new IRQ board is a thirteenth block however
it is written.

So the lot stops paying per board and pays once. `IrqBoard` (`mappers/irq_boards.rs`) is
`SmallBoard` read the other way round — every board in that enum answers *no* to `irq_asserted`
and therefore shares the empty arm; every board in this one answers *yes* and can only share the
others' — so one inner match is one block in `Board::irq_asserted`, one in `on_cpu_cycle`, one in
`on_ppu_address_bus` and one in `ppu_irq_horizon` **for 73, 65, 67, 48, 105 and 116 together**.
`chr_view` answers `None` for the whole variant, which is the body the MMC2, MMC3, MMC5, RAMBO-1
and `SmallBoard` already give, so `Ppu2C02::run_dots` gains nothing to lay out and stays at
**1 907 instructions** at every variant below.

### The board

nesdev ("VRC3"), the whole chip, and every claim in the code quotes it: 16 KiB switchable PRG at
`$8000` from `$F000-$FFFF` bits 0-2, 16 KiB fixed to the last bank at `$C000`, 8 KiB of unbanked
CHR-RAM, hard-wired mirroring — which is what frees `$A000` to be an IRQ latch here where every
other Konami board has an arrangement register — and `$E000-$EFFF` not decoded at all. The
counter is "16 bits instead of 8 bits, and there is no scanline mode, only CPU cycle mode", its
latch arriving as **four** nibbles at `$8000`, `$9000`, `$A000` and `$B000`; it "is incremented
each CPU cycle", "when it overflows from $FFFF, an IRQ is generated, and the counter is reloaded
with the 16-bit latch value", and in 8-bit mode "IRQs are generated when the lower 8 bits
overflow from $FF". The control register is the family's `.... .MEA` and `$D000` the family's
acknowledge.

**The 8 KiB of CHR-RAM is the board's and goes in `registry::board_memory_sizes`**, for the reason
MF-6a2 gave the TQROM's: REQ-CORE-012 hands an iNES 1.0 image with no CHR-ROM **32 KiB**, three
quarters of which this chip has no bank line to reach. Mesen 2 agrees (`VRC3.h:17`).

**One divergence from Mesen 2, and AMD-061 §1 settles it for the page.** `VRC3.h:41-48` writes

    uint8_t smallCounter = _irqCounter & 0xFF;
    smallCounter++;
    if(_smallCounter == 0) { ... }

testing `_smallCounter`, the mode *flag*, where it means `smallCounter`, the byte it has just
incremented. The flag is true inside that branch, so Mesen's 8-bit mode never fires at all. The
page's sentence is implemented instead, and the comment in `vrc_irq.rs` says so.

### Why `Vrc3Irq` stands beside `VrcIrq` instead of being `VrcIrqCounter<true>`

The width parameter was built first, as **V1**: `VrcIrqCounter<const WIDE: bool>` with
`VrcIrq = <false>` and `Vrc3Irq = <true>`, every `if WIDE` compile-time dead on one of the two —
the shape MR-4-3 gave the MC-ACC and MF-6a step 4 gave mapper 155. The 16-bit latch needs one
byte more, which lands in the padding after `latch` and so costs the struct nothing: 16 bytes
with and without it. What it does cost is **one byte of displacement on `pending`** inside
`Vrc2Vrc4`, `Vrc6` and `Vrc7` — and those three displacements are *operands* in three of the
twelve blocks above. The dump shows the whole chain re-laid-out: offsets that read 9723, 9748,
9761 and 9772 at the baseline read 9717, 9723, 9761 and 9793 in V1, and the block that holds the
fall-through changes with them.

**V2 is V1 with `VrcIrq` byte-identical to `9f9d9d41` and a separate `Vrc3Irq` struct**, which
isolates the two causes — and it leaves the regression exactly where it was, ROM for ROM
(`vblnmi` +881 934 → +874 045, `m1` +789 853 → +789 160, `m4` +800 987 → +795 877). So the field
churn is **not** the cost; the thirteenth block is. The width parameter is out because it buys
nothing and moves three boards' layouts for it, and
`mf_6b_the_vrc3_and_the_vrc4_fire_on_the_same_cycle` pins what the two chips really do share: the
control register, the reload on `E`, the acknowledge that copies `A` into `E`, and an up-counter
that fires on the step out of its top value.

### The measurement: three shapes, and the one that is below the bar on both instruments

| shape | `Cpu::step` | `resolve` | `branch` | set total | `m4` | `mm2` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `9f9d9d41` | 3 443 | 1 854 | 418 | — | — | — |
| **V1** the width parameter | 3 443 | 1 854 | 418 | **+9 886 794** | +800 987 | +1 276 020 |
| **V2** a separate `Vrc3Irq` | 3 535 | 1 901 | 429 | **+958 143** | +795 877 | +1 228 441 |
| **V3** V2 + `#[inline(never)]` on `IrqBoard::irq_asserted` (**shipped**) | 3 616 | 1 944 | 456 | **−29 072 137** | **−1 956 515** | **−2 880 882** |

V2's static figures are §4.2's yardstick to the instruction — "`Cpu6502::branch` **+11** and
`Cpu6502::resolve` **+47** are the per-arm cost of one more `irq_asserted` variant" — and the
yardstick turns out to be **the wrong unit for this dispatch**. V1 has the *same* static sizes as
the baseline (3 443 / 1 854 / 418, unmoved to the instruction) and the *worst* dynamic figure of
the three; V3 is the **largest** of the three statically and the only one below the bar. What
moves is not how many instructions `branch` holds but which of its blocks sit on the
fall-through, and the dynamic cost per ROM then depends on that ROM's own branch mix — which is
why nine ROMs are up by +0.013 % to +0.026 % in V1 and V2 while `metroid` is **down** 0.014 % and
`oamstr` down 0.002 %.

**V3 is MF-6a2 step 2's fix applied one dispatch along.** That step took the RAMBO-1's body out
of `Board::ppu_irq_horizon` with `#[inline(never)]`, which took `NesBus::catch_up_ppu` from 216
instructions to 203 and bought −0.005 % to −0.082 % on **every** board in the library. Here the
body comes out of `Board::irq_asserted`, and `Cpu6502::branch` falls on **twelve of the thirteen
ROMs**:

| ROM | board | `branch` at `9f9d9d41` | V3 | delta |
| --- | --- | ---: | ---: | ---: |
| `mm2` | MMC1 | 141 362 311 | 138 634 831 | **−2 727 480** |
| `duelito` | **NROM** | 129 167 584 | 126 670 564 | **−2 497 020** |
| `readbuf` | **CNROM** | 112 504 708 | 110 359 851 | −2 144 857 |
| `vblnmi` | MMC1 | 109 931 748 | 107 876 455 | −2 055 293 |
| `m4` | MMC3 | 177 091 455 | 175 204 063 | −1 887 392 |
| `m0` | **NROM** | 181 707 977 | 180 404 199 | −1 303 778 |
| `smb3` | MMC3 | 173 228 963 | 171 955 860 | −1 273 103 |
| `m1` | MMC1 | 171 152 167 | 169 998 510 | −1 153 657 |
| `z2` | MMC1 | 150 273 887 | 149 262 765 | −1 011 122 |
| `oamstr` | **NROM** | 72 774 245 | 71 836 955 | −937 290 |

An NROM has no IRQ counter and takes the shared "no" arm, and it still gains 2.5 M instructions
over 300 frames, because the dispatch's *shape* is what `branch` is laid out around. The price is
named rather than hidden: **a cartridge of the `IrqBoard` family pays a `callq` per /IRQ sample**,
where it would have had one inlined `cmpb`. No ROM of either instrument is one of these boards,
so the cost is not measurable here; it is real, and it is the same trade MF-6a2 step 2 charged
the RAMBO-1. LLVM then rewrites that call's own ABI — `IrqBoard::irq_asserted` is three
instructions, `movl %edi, %eax ; andb $1, %al ; retq`, the flag arriving in a register — so the
load stays in `branch` and only the test and the join leave it.

### The bar, on both instruments

**Level 2**, `l2-diff.py --bar`, V3 against `mf6b-0-9f9d9d41`:

| game/stage | `9f9d9d41` | **V3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 898 492 | **11 147 219 247** | −679 245 | −0.006 % |
| `smb1/core` | 11 515 980 923 | **11 515 146 178** | −834 745 | −0.007 % |
| `zelda2/core` | 12 777 709 570 | **12 775 623 328** | −2 086 242 | −0.016 % |
| `mm2/core` | 12 043 306 623 | **12 036 891 507** | −6 415 116 | −0.053 % |
| `smb3/core` | 15 318 626 455 | **15 315 376 398** | −3 250 057 | −0.021 % |
| `metroid/total` | 13 736 176 888 | **13 735 529 421** | −647 467 | −0.005 % |
| `smb1/total` | 13 589 021 252 | **13 586 861 699** | −2 159 553 | −0.016 % |
| `zelda2/total` | 15 362 221 166 | **15 358 914 988** | −3 306 178 | −0.022 % |
| `mm2/total` | 18 982 867 263 | **18 976 420 628** | −6 446 635 | −0.034 % |
| `smb3/total` | 24 192 639 363 | **24 189 117 265** | −3 522 098 | −0.015 % |

`l2-diff.py`'s own verdict: **"at or below the baseline on every game, total and core"** — the
first step of any MF lot to get that line with a board added rather than removed.

**PR-1's thirteen**, all thirteen below, set total **−29 072 137 (−0.037 %)**:

| ROM | `9f9d9d41` | **V3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 310 470 388 | **3 308 346 431** | −2 123 957 | −0.064 % |
| `oamstr` | 3 319 466 313 | **3 318 458 850** | −1 007 463 | −0.030 % |
| `m1` | 4 886 651 514 | **4 885 426 439** | −1 225 075 | −0.025 % |
| `m0` | 5 521 219 795 | **5 519 797 926** | −1 421 869 | −0.026 % |
| `m4` | 5 473 783 790 | **5 471 827 275** | −1 956 515 | −0.036 % |
| `metroid` | 5 116 149 734 | **5 115 922 897** | −226 837 | −0.004 % |
| `smb1` | 5 996 882 321 | **5 996 357 010** | −525 311 | −0.009 % |
| `mm2` | 6 020 725 910 | **6 017 845 028** | −2 880 882 | −0.048 % |
| `z2` | 6 292 582 908 | **6 291 454 769** | −1 128 139 | −0.018 % |
| `readbuf` | 6 836 830 137 | **6 834 577 385** | −2 252 752 | −0.033 % |
| `duelito` | 7 383 094 194 | **7 380 410 705** | −2 683 489 | −0.036 % |
| `smb3` | 7 950 795 166 | **7 949 416 073** | −1 379 093 | −0.017 % |
| `mmc5` | 11 289 605 855 | **11 279 345 100** | −10 260 755 | −0.091 % |
| **total** | **79 398 258 025** | **79 369 185 888** | **−29 072 137** | **−0.037 %** |

`mmc5`'s −10.3 M is two symbols and only one of them is the attribute's: `branch` −1 174 873 and
**`NesBus::push_expansion_audio` −8 934 160**, which the thirty-fifth `Board` variant re-laid out
on its own — it is −8 934 160 in V2 as well, before the attribute. MF-6a step 2 measured the
mirror of that (the thirty-seventh variant cost that function +75.9 M by outlining
`Board::expansion_audio_output`); here a variant bought it back, and the `#[inline(always)]` that
step added is what keeps the dispatch in place while its layout improves.

### The post-LTO invariants, and an instrument that reported a stop-the-lot that was not one

`shape.py`, V3 against the baseline: `Board::irq_asserted` **0 / 0 / 0**, `Wide::zone_offset`
**0 / 0 / 0**, `Cpu::execute_without_operand` **0 / 0 / 0** — the three claimants of
`Cpu6502::step`'s budget still inlined. `NesBus::read_cycle` 1/65/181, `tick` 1/95/82,
`read_internal` 1/17/175 and `Ppu::run_dots` 1/2/1907 **unmoved to the instruction**;
`write_internal` 3 103 → 3 274, which is the VRC3's register decode in its own arm.
`prologues.py`: **all twenty pinned prologues identical**, `Board::on_cpu_cycle`'s included.
`Board::ppu_peek` 271 → 271, `ppu_read` 354 → 354, `on_ppu_address_bus` 156 → 156,
`catch_up_ppu` 188 → 188, `read_nametable_inner` 114 → 114, `Mmc1::update_banks` 215 → 215.

**`callsites.py` printed `Board::irq_asserted 0 → 54 FIRST CALL SITE -- stop the lot`, and it is a
false positive.** Its watched owners are matched as plain substrings, and `Board` is a substring
of **`IrqBoard`**: the 54 sites are calls to the new leaf, not to the dispatch, which `shape.py`'s
own regex (`5Board.*6Mapper12irq_asserted`) and `grep -o '^_RNv.*irq_asserted:'` both put at zero
definitions and zero sites. The same looseness was hiding the definitions column, because the
pattern required a digit, `$` or a capital after the method name and a definition line ends in
`:`. The local copy now anchors the owner on its v0 length prefix (`5Board`, not `Board`) and
allows the colon, which puts `read_nametable_inner`, `run_dots`, `ppu_read` and the rest at 1
definition each where they read 0 before. **This is the third time an instrument of this project
has been blind at exactly the symbol a lot was watching** — MF-3b rule 1 and MF-5b step 1 are the
other two — and the first time one has cried wolf rather than stayed quiet. `rust/target-mf/asm/`
is scratch and not committed, so the fix is recorded here and in PERFORMANCE-HISTORY §5.13.

### Tests

**853 passed, 0 failed, 8 ignored**, which is MF-6a2's 847 plus six:

| test | what it pins |
| --- | --- |
| `req_core_100_the_vrc3_counter_is_sixteen_bits_wide_and_counts_cpu_cycles_up` | the unit (CPU cycle), the direction (up), the 16-bit wrap at $10000 − latch, and the acknowledge stopping a counter whose `A` is clear |
| `req_core_100_the_vrc3_eight_bit_mode_wraps_at_ff_and_leaves_bits_8_to_15_alone` | `M` as a **width**: the same latch that takes 65 040 cycles takes 16; the reload is the latch's low byte; and switching back to sixteen bits with `E` clear shows bits 8-15 still hold the $01 the first reload put there |
| `req_core_100_the_vrc3_enable_without_a_reload_leaves_the_counter_armed` | §5's own case, the one `req_core_100_the_vrc6_counter_*` pins for the VRC6 |
| `req_core_100_the_vrc3_banks_16_kib_at_8000_and_fixes_the_last_at_c000` | the three bits of `$F000`, the fixed last bank, `$E000` not decoded, and no register of the board moving CHR |
| `req_core_100_the_vrc3_carries_eight_kib_of_chr_ram_and_not_the_default_thirty_two` | the board's CHR-RAM size against REQ-CORE-012's default |
| `mf_6b_the_vrc3_and_the_vrc4_fire_on_the_same_cycle` | what the two chips share, through two decodes with nothing in common |

`req_core_100_every_required_mapper_is_registered`'s list of numbers beyond REQ-CORE-100's table
is extended with 73, and `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` needed nothing
once 73 was in `clocks_on_cpu_cycle` — it sweeps `SUPPORTED_MAPPERS` and would have failed
otherwise. `cargo fmt --check` clean; `cargo clippy --release -p zax-nes --all-targets -D
warnings` clean, **no new `allow` of any kind**.

The commercial title is *Salamander* (Konami, 1987), for R0 to run.

[PROGRESS] MF-6b step 1/8 done: mapper 73, one `Board` variant for the whole lot, and the body out of line buys −29 072 137 on the set and −0.006 % to −0.053 % on all five level-2 cores.
