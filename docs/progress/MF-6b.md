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

**Stop-the-lot condition**: a figure above the reference that the step cannot fix — and from
step 2 on, the board is **parked, not reverted** (owner's rule, 2026-11-06, which stopped and
restarted this session after step 1 for that one change; step 1 stands). Parking means a new
Cargo feature **`parked-boards`** of `zax-nes`, **off by default**, carrying the board's module,
its `Board` variant, its arms in every dispatch, its registry entry and its tests, all
`#[cfg(feature = "parked-boards")]`, so that feature-off the build is the same as without the
board — shown, not asserted: the post-LTO check and the bar measured feature-off, and
`cargo test --release -p zax-nes --features parked-boards` green as well. The board then stays
listed here and in `docs/review/MF-6-inventory.md` as "parked: built and tested, off by default,
cost `<figures>`", which is the store the later lot of one specialised core per board picks it
up from. MF-6a2 step 3 is still the model for *what to record*; only the disposal changed.

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

## Step 2 — mappers 65 and 67, and the variant that paid for itself a second time

### The two boards, and the only thing an implementation can conflate

Both are a 16-bit counter stepped once per CPU cycle, with no prescaler, no scanline mode and
nothing on the PPU bus — which is why §5 puts them in one step — and they differ on exactly two
things. The step's own test, `mf_6b_mapper_65_fires_at_zero_and_mapper_67_one_cycle_past_it`,
drives both from the same count in the same assertion, which is what §5 asks for in as many
words:

| | **65**, Irem H3001 | **67**, Sunsoft-3 |
| --- | --- | --- |
| the firing | "when the counter reaches 0, an IRQ is fired" | "whenever the count wraps from $0000 to $FFFF" — one cycle later |
| afterwards | "the counter **stops at 0** -- it does not wrap and isn't automatically reloaded" | "the mapper asserts an IRQ and **pauses itself**", the count running on from $FFFF |
| the load | `$9004` copies a **reload register** (`$9005` high, `$9006` low) into the counter | `$C800`, written twice, high byte first, **straight into the live count** — "much like PPUADDR" |
| the enable | `$9003` bit 7, and a write there or to `$9004` acknowledges | `$D800` bit 4, and a write there explicitly does **not** acknowledge |
| the acknowledge | `$9003` and `$9004` | `$8000`, mask **`$8800`** |

**Mapper 67's decode is one line because of that mask.** The acknowledge is decoded on A15 and
A11 alone and the other seven registers on A12-A15 with A11 set, so "A11 clear above `$8000` is
the acknowledge, A11 set is one of the seven" is the whole of it —
`req_core_100_the_sunsoft3_acknowledges_on_every_a11_clear_mirror_of_8000` walks eight of those
mirrors.

Mapper 65's mask is `$F007`, so A3-A11 are undecoded and one register answers over its whole
4 KiB page — except at `$B000-$B007`, where A0-A2 split the page into the eight 1 KiB CHR
registers. `req_core_100_the_h3001_powers_on_with_prg_reg_0_at_zero_and_reg_1_at_one` writes
`$8007`, `$AFFF` and `$8123` to pin the collapsing half and
`req_core_100_the_h3001_banks_eight_1_kib_chr_regs_at_b000` writes `$B80C` to pin the splitting
one.

### Five divergences from Mesen 2, all the same way round, and AMD-061 §1 settles every one

The pages are newer than the code, and in four of the five cases they say so themselves:

1. **`$C000` is not a PRG register on the H3001.** The page's first line: "Disch's older
   documentation is known to be wrong, incorrectly believing in a changeable bank at $C000. It
   doesn't exist", with the forum post that settled it. `IremH3001.h:70` still has
   `case 0xC000: SelectPrgPage(2, value)`. Here `$C000-$FFFF` decodes to nothing and that window
   follows `$9000`'s layout bit alone.
2. **The H3001's mirroring has three outcomes, not two.** `$9001` is `[MM.. ....]`: "%00 = Vert",
   "%10 = Horz", "%01,%11 = 1scA". Mesen tests bit 7 alone (`IremH3001.h:57`) and never reaches
   the one-screen state, so bit 6 decides first here.
3. **The H3001's counter stops rather than its enable bit clearing.** Mesen clears `_irqEnabled`
   when the IRQ fires (`IremH3001.h:41-48`), which leaves a counter sitting at $0000 to be
   decremented to $FFFF by the next `$9003` enable and fire again 65 536 cycles later. The page's
   "stops at 0" is one test on the counter instead, and
   `req_core_100_the_h3001_counter_stops_at_zero_and_only_9004_reloads_it` is that sentence.
4. **`Sunsoft3.h` has no acknowledge at all.** Its `WriteRegister` switches on `addr & 0xF800`
   with no arm for the A11-clear pages, so the `$8000` register is unreachable there.
5. **`$D800` acknowledges in Mesen** (`Sunsoft3.h:57-61`), where the page says "despite previous
   notes, writes to this register do not acknowledge the IRQ".

Two narrower fields than the byte written into them, which Mesen passes whole and leaves to its
own bank wrap: the Sunsoft-3's `$F800` is `...X PPPP` with `X` "a 1 bit latch, present but
unused", and its CHR registers reach six bits because "the hardware only has six pins for CHR
banking, for a limit of 128KB of CHR".
`req_core_100_the_sunsoft3_prg_latch_bit_and_the_two_spare_chr_bits_are_dropped` is measured on
a 12-bank PRG and a 48 KiB CHR **on purpose**: at the fixture's default sizes the dropped bit is
a whole multiple of the bank count, so masking and not masking give the same bank and the test
would pass either way.

`$3E` and `$3F`, the H3001's two fixed banks, are `-2` and `-1` through
`BankWindows::map_prg_8k`: they are the last two 8 KiB banks of the 512 KiB its four bank lines
reach, and a board masks the bank number by its own ROM size, so $3E & 31 = 30 is the
second-to-last bank of a 256 KiB ROM and of every smaller one. Mesen reaches the same two banks
by the same arithmetic (`0xFE` and `-1`, `IremH3001.h:25-26`).

### The design: no new `Board` variant, and the measurement that says why that was right

Both boards are inner variants of [`IrqBoard`], so the lot still spends **one** body in
`Board::irq_asserted`, one in `Board::on_cpu_cycle`, one in `Board::on_ppu_address_bus` and one
in `Board::ppu_irq_horizon` — step 1's design, now carrying three boards instead of one. Each
board's own `on_cpu_cycle` is `#[inline(never)]` per §4.1 item 2, which §5 step 2 asks for by
name.

What LLVM did with the three-way inner match is worth the line. Step 1's leaf was three
instructions; the three-board leaf is **four**, and branchless:

    movl %edi, %eax ; cmpb $2, %sil ; cmovbl %esi, %eax ; retq

[`IrqBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/irq_boards.rs

### The bar, on both instruments

**PR-1's thirteen**, against `mf6b-0-9f9d9d41` — **every ROM below, set total −30 338 767
(−0.038 %)**, which is 1.27 M better than step 1 shipped:

| ROM | board | `9f9d9d41` | **step 2** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 310 470 388 | **3 308 287 961** | −2 182 427 | −0.066 % |
| `oamstr` | NROM | 3 319 466 313 | **3 318 033 026** | −1 433 287 | −0.043 % |
| `m1` | MMC1 | 4 886 651 514 | **4 885 266 985** | −1 384 529 | −0.028 % |
| `m0` | NROM | 5 521 219 795 | **5 519 813 048** | −1 406 747 | −0.025 % |
| `m4` | MMC3 | 5 473 783 790 | **5 469 380 223** | −4 403 567 | −0.080 % |
| `metroid` | MMC1 | 5 116 149 734 | **5 115 169 849** | −979 885 | −0.019 % |
| `smb1` | NROM | 5 996 882 321 | **5 996 345 177** | −537 144 | −0.009 % |
| `mm2` | MMC1 | 6 020 725 910 | **6 017 898 512** | −2 827 398 | −0.047 % |
| `z2` | MMC1 | 6 292 582 908 | **6 291 190 363** | −1 392 545 | −0.022 % |
| `readbuf` | CNROM | 6 836 830 137 | **6 834 513 940** | −2 316 197 | −0.034 % |
| `duelito` | NROM | 7 383 094 194 | **7 380 402 412** | −2 691 782 | −0.036 % |
| `smb3` | MMC3 | 7 950 795 166 | **7 944 884 594** | −5 910 572 | −0.074 % |
| `mmc5` | MMC5 | 11 289 605 855 | **11 286 733 168** | −2 872 687 | −0.025 % |
| **total** | | **79 398 258 025** | **79 367 919 258** | **−30 338 767** | **−0.038 %** |

`callgrind-diff.py`'s own verdict: **"no ROM slower, total lower -- keepable"**.

**Level 2**, `l2-diff.py --bar`: **"at or below the baseline on every game, total and core"**.

| game/stage | `9f9d9d41` | **step 2** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 898 492 | **11 145 612 461** | −2 286 031 | −0.021 % |
| `smb1/core` | 11 515 980 923 | **11 515 047 278** | −933 645 | −0.008 % |
| `zelda2/core` | 12 777 709 570 | **12 774 468 288** | −3 241 282 | −0.025 % |
| `mm2/core` | 12 043 306 623 | **12 036 875 886** | −6 430 737 | −0.053 % |
| `smb3/core` | 15 318 626 455 | **15 304 530 884** | −14 095 571 | −0.092 % |
| `metroid/total` | 13 736 176 888 | **13 733 892 748** | −2 284 140 | −0.017 % |
| `smb1/total` | 13 589 021 252 | **13 587 062 143** | −1 959 109 | −0.014 % |
| `zelda2/total` | 15 362 221 166 | **15 358 827 771** | −3 393 395 | −0.022 % |
| `mm2/total` | 18 982 867 263 | **18 976 390 259** | −6 477 004 | −0.034 % |
| `smb3/total` | 24 192 639 363 | **24 179 328 952** | −13 310 411 | −0.055 % |

`smb3/core`'s −0.092 % is the largest single-game core move of the lot so far, and it is not the
IRQ dispatch: it is `Board::ppu_peek`.

### Where the instructions actually went: two symbols, in opposite directions

Against **step 1's shipped V3** the set is −1 266 630 overall but **three ROMs are up** — `m0`
+15 122, `mm2` +53 484 and `mmc5` +7 388 068 — and the bar is `9f9d9d41`, where all three are
down. Both ends of that are one symbol each, and the `1 000 000`-threshold diff of V3 against
step 2 prints nothing else:

* **`Board::ppu_peek` −2 288 640 on `m4` and −4 473 536 on `smb3`.** The two MMC3 ROMs, on a
  board this step did not touch. Statically the function went **271 → 281**, +10 — so it is
  *larger* and *faster*, for the reason step 1 found in `Cpu6502::branch`: what the arms cost is
  not their instruction count but which of them sits on the fall-through. §4's fourth capped hook
  is therefore comfortable rather than strained — MF-6a2 step 3's TQROM arm took this function to
  583 and cost `m4` 3.4 M, and 281 costs it −2.3 M.
* **`Cartridge::default_cpu_read` +7 456 756 on `mmc5`**, and statically **305 → 322**, +17. That
  function inlines `Board::try_read_prg`, whose inner `IrqBoard` match gains a body per board, and
  the MMC5 is the one ROM of the thirteen whose reads go down that path in bulk. 305 at the
  baseline *and* at V3: the first `IrqBoard` variant was free there and the second and third were
  not.

**That is the figure step 4 has to read first.** `mmc5`'s remaining margin against the reference
is −2 872 687, and two more inner variants at ≈ +3.7 M each would spend it. Steps 3 and 5 do not
add any (105 is a parameter of `Mmc1`, 158 of `Rambo1`); steps 4 and 6 add one each. If `mmc5`
goes above `9f9d9d41` at step 4, the fix to try before parking the TC0690 is `#[inline(never)]`
on `IrqBoard::banks` — the same move that bought step 1 the whole set, one dispatch along.

### The post-LTO invariants

`shape.py`, the three dumps side by side:

| symbol | `9f9d9d41` | V3 (step 1) | **step 2** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 443 | 1 / 2 / 3 616 | 1 / 2 / 3 645 |
| `Cpu::resolve` | 2 / 6 / 1 854 | 2 / 6 / 1 944 | 2 / 6 / 1 958 |
| `Cpu::branch` | 1 / 1 / 418 | 1 / 1 / 456 | 1 / 1 / 468 |
| `NesBus::read_cycle` | 1 / 65 / 181 | 1 / 65 / 181 | 1 / 65 / 181 |
| `NesBus::tick` | 1 / 95 / 82 | 1 / 95 / 82 | 1 / 95 / 82 |
| `NesBus::read_internal` | 1 / 17 / 175 | 1 / 17 / 175 | 1 / 17 / 175 |
| `NesBus::write_internal` | 1 / 10 / 3 103 | 1 / 10 / 3 274 | 1 / 10 / **3 145** |
| `Ppu::run_dots` | 1 / 2 / 1 907 | 1 / 2 / 1 907 | 1 / 2 / 1 907 |

The three claimants of `Cpu6502::step`'s budget are still inlined into it, and `Ppu::run_dots`
has not moved by an instruction in either step — `IrqBoard::chr_view` answering `None` for the
whole variant is doing its job.

**`NesBus::write_internal` went *down*, 3 274 → 3 145**, with two boards' register decodes added.
V3 had the VRC3's decode inlined there (3 103 + 171); at three boards LLVM outlined all three —
`Vrc3::write_register` 151 instructions, `IremH3001::write_register` 51,
`Sunsoft3::write_register` 57, none of them marked — and `write_internal` kept only the +42 of the
dispatch. Nothing asked for that and nothing depends on it; it is recorded because the figure
would otherwise read as a board having gone missing.

`prologues.py`: **all twenty pinned prologues identical**, `Board::on_cpu_cycle`'s included —
still `pushq %r14 ; pushq %rbx ; pushq %rax ; movzbl 1368(%rdi), %eax ; subb $2, %al ; movl $5,
%ecx`, step 0's own reading. `callsites.py`: every watched callee unchanged, and
`Board::irq_asserted`, `NesBus::irq_asserted` and `Cartridge::irq_asserted` all **0 defs / 0
sites**.

The other watched figures: `Board::ppu_read` 354 → 364, `Board::on_cpu_cycle` 396 → 417,
`Board::on_ppu_address_bus` **156 → 156**, `NesBus::catch_up_ppu` **188 → 188**,
`Ppu2C02::read_nametable_inner` **114 → 114**, `Mmc1::update_banks` **215 → 215** (byte-identical).
The three out-of-line `on_cpu_cycle` leaves are 24 instructions (VRC3), 12 (H3001) and 7
(Sunsoft-3).

**`cmp4c.py` cried wolf at the same symbol `callsites.py` did in step 1, and for the same
reason.** It prints `Board::irq_asserted  defs 0 -> 4  DEFS`, and `4` is the four instructions of
`IrqBoard::irq_asserted`: its owner pattern is the plain substring `Board`, which `8IrqBoard`
contains. The direct check is the one step 0 recorded —
`grep -oE '^_RN[^:]*irq_asserted[A-Za-z0-9_]*:'` over the dump returns
`...10irq_boards...8IrqBoard...6Mapper12irq_asserted` and **nothing else**, in step 1's dump and
in this one, so `Board::irq_asserted` has no definition and no call site. `shape.py`, whose regex
anchors the owner on its v0 length prefix (`5Board`), agrees. That is the **fourth** instrument of
this project blind or loud at exactly the symbol a lot was watching (MF-3b rule 1, MF-5b step 1,
MF-6b step 1), and `rust/target-mf/asm/` is scratch, so the fix belongs with `callsites.py`'s:
anchor on `5Board`, not `Board`.

### Tests

**868 passed, 0 failed, 8 ignored**, which is step 1's 853 plus fifteen:

| test | what it pins |
| --- | --- |
| `req_core_100_the_h3001_counter_counts_down_one_per_cpu_cycle_and_fires_at_zero` | the unit (CPU cycle), the direction (down), the firing *at* zero, and `$9005` being the **high** byte — a swapped pair would read 61 441 cycles instead of 496 |
| `req_core_100_the_h3001_counter_stops_at_zero_and_only_9004_reloads_it` | "stops at 0 -- it does not wrap", and that `$9005`/`$9006` "do not have any effect on the actual counter" |
| `req_core_100_the_h3001_enable_without_a_reload_leaves_the_counter_armed` | §5's own case: `$9003` reloads nothing, and the cycles that passed while `E` was clear moved nothing |
| `req_core_100_the_h3001_powers_on_with_prg_reg_0_at_zero_and_reg_1_at_one` | the power-on the page says games crash without, the `$F007` mask collapsing a page, and `$C000-$FFFF` decoding to nothing |
| `req_core_100_the_h3001_layout_bit_moves_prg_reg_0_to_c000` | `$9000` bit 7, both ways round, with `$A000` and `$E000` unmoved |
| `req_core_100_the_h3001_banks_eight_1_kib_chr_regs_at_b000` | the eight CHR registers, and A0-A2 splitting the one page the mask does not collapse |
| `req_core_100_the_h3001_mirroring_is_decided_by_bit_6_before_bit_7` | all three outcomes of `[MM.. ....]`, the one-screen case Mesen cannot reach |
| `req_core_100_the_sunsoft3_count_is_written_high_byte_first_and_fires_on_the_wrap` | PPUADDR's shape, into the live count, and the wrap being one cycle past zero |
| `req_core_100_the_sunsoft3_pauses_itself_on_the_wrap_and_d800_does_not_acknowledge` | "pauses itself", the page's correction of Disch on `$D800`, and the count resuming from $FFFF |
| `req_core_100_the_sunsoft3_acknowledges_on_every_a11_clear_mirror_of_8000` | the `$8800` mask, over eight mirrors |
| `req_core_100_a_d800_write_puts_the_sunsoft3_c800_toggle_back_to_the_high_byte` | the toggle reset, through a `$C800` write deliberately left half-done |
| `req_core_100_the_sunsoft3_banks_16_kib_at_8000_and_four_2_kib_chr_windows` | the PRG pair and the four CHR registers |
| `req_core_100_the_sunsoft3_prg_latch_bit_and_the_two_spare_chr_bits_are_dropped` | `...X PPPP` and the six CHR pins, on a ROM where masking and not masking differ |
| `req_core_100_the_sunsoft3_mirroring_register_has_four_states` | `.... ..MM`, all four |
| `mf_6b_mapper_65_fires_at_zero_and_mapper_67_one_cycle_past_it` | §5's conflation test: the same count in both chips, one cycle apart, then stop against pause |

`req_core_100_every_required_mapper_is_registered`'s list of numbers beyond REQ-CORE-100's table
is extended with 65 and 67. The four sweeps needed nothing written:
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` passes because both numbers went into
`clocks_on_cpu_cycle` and would have failed otherwise,
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` because neither board touches
that bus, and
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` and
`req_core_090_the_mapper_state_survives_a_save_and_restore` because both boards reset and
serialise every field they have.

`cargo fmt --check` clean; `cargo clippy --release -p zax-nes --all-targets -- -D warnings`
clean, **no new `allow` of any kind**.

The commercial titles are *Daiku no Gen San 2*, *Kaiketsu Yanchamaru 3* and *Spartan X 2* (65),
and *Fantasy Zone II* (J) and *Mito Koumon II - Sekai Manyuu Ki* (67), for R0 to run. *Vs.
Platoon* is the Sunsoft-3 on a VS System board, which §6 of the inventory sets aside.

[PROGRESS] MF-6b step 2/8 done: mappers 65 and 67 in the same variant, all thirteen ROMs and all five level-2 cores below `9f9d9d41` (set −30 338 767, `smb3/core` −0.092 %), and `Cartridge::default_cpu_read` named as the figure step 4 reads first.

## Step 3 — mapper 105, and the answer to `const`-versus-field is neither

### The question §5 asks, and why the step does not answer it

§5 calls this "the step where `const`-vs-field matters most in the whole lot, because mapper 1 is
on five of R0's thirteen ROMs (`vblnmi`, `m1`, `metroid`, `mm2`, `z2`)", with the stop condition
"any of those five above `9f9d9d41`". Both of the shapes it offers cost mapper 1 something,
and both costs are already measured in this project:

* a **`const` parameter** — `Mmc1Board<const EVENT: bool>`, the shape MR-4-3 gave the MC-ACC —
  monomorphises the *code* but not the *struct*: the 30-bit counter and its three flags are
  fields of both instantiations, and **MF-6b step 1 priced exactly that move**. One byte of
  displacement on `VrcIrq::pending` re-laid out `Cpu6502::branch` and cost eleven of the thirteen
  ROMs +0.009 % to +0.027 %, ending +9 886 794 on the set, with no code added at all. `Mmc1`'s
  fields are read on more paths than `VrcIrq`'s.
* a **runtime `bool`** is the one MF-6a2 step 2 already charged: mapper 155 as
  `!self.no_ram_disable && …` cost `Mmc1::update_banks` **+8 001 instructions over 600 level-2
  frames of Mega Man 2**, +2 217 on Metroid and +4 311 on Zelda II, because the test is on the
  commit path.

So [`NesEvent`] **holds an `Mmc1` instead of being one**. The chip runs exactly as it does on a
mapper-1 cartridge — the five-bit shift register, the consecutive-write rule, the four registers,
the mirroring — and the board then writes its own PRG layout over the windows the MMC1 just
computed, which is what the "several 4000-series and 7400-series discrete ICs" of the page's parts
list physically do to the MMC1's outputs. The measurement that says this was the right shape is
one line of `cmp4c.py`:

    Mmc1::update_banks       1    215    215  IDENTICAL  IDENTICAL  IDENTICAL

**Byte-identical, baseline to step 3.** Mapper 1 carries no counter, no flag and no test. The
five accessors `Mmc1` gained (`control_register`, `chr0_register`, `prg_register`, `windows_mut`,
`force_chr0_register`) add no field and no branch, and nothing but `NesEvent` calls them.

What the NES-EVENT pays instead is one redundant MMC1 bank computation per write to
`$8000-$FFFF`, which §4.10's rule puts exactly where it belongs: there is one cartridge of this
board in the world.

`NesEvent` is the fourth inner variant of `IrqBoard`, so it adds **no** `Board` discriminant. A
probe of `size_of` before the step settled the one thing that could have made that expensive:
`Board` is **1 376 bytes and that is `Mmc5`'s size**, niche-filled, where `Mmc1` is 144 and
`IrqBoard` 136 — so an inner variant of ~152 bytes cannot grow the enum, and `Cartridge` is the
same size it was.

[`NesEvent`]: ../../rust/crates/zax-nes/src/cartridges/mappers/nes_event.rs

### The board

nesdev ("NES-EVENT", "INES Mapper 105") and Disch's notes, which agree register for register:
`$8000` is `[.... PSMM]`, "same as MMC1 (but CHR mode bit isn't used)"; `$A000` is
`[...I OAA.]`, the MMC1's CHR register repurposed as a timer control, a chip select and a PRG
register; `$C000` is unused; `$E000` is `[...W BBBB]`, the MMC1's `W` and the second chip's bank.
PRG is "2 PRG chips, each 128k", `O` choosing both the chip *and* which PRG register selects
inside it, and "if the mapper has not been 'unlocked', the first 32k of the first chip is always
selected regardless (as if $A000 contained $00)".

**Three readings the sources leave open, and what the board does with each:**

1. **The power-on `I` bit.** "The first 32 KiB is hardwired until the timer is started (write 0
   then 1 to bit 4) for the first time" only means something if `I` starts **set** — an MMC1
   powers its CHR register on at zero, and the documented "write 0" would then be a no-op that
   unlocks on the first `I=1` instead of the second edge. No page states the power-on value;
   Mesen 2 resolves it with `_chrReg0 |= 0x10; //Set I bit to 1` (`MMC1_105.h:24`) and this board
   does the same, so the documented two-step sequence is the sequence that unlocks.
   `req_core_100_the_nes_event_hardwires_the_first_32_kib_until_the_timer_is_unlocked` writes the
   half-done sequence on purpose: a board counting one edge swaps PRG a write too early.
2. **The DIP switches**, which have no host surface in this build. The page's own default is the
   one taken: "in the official tournament, `C` was closed, and the others were open, so the
   counter had to reach $2800000" — $28000000, a digit short, which its own `[1D CBAx …]` formula
   confirms (bit 29 set, `C` at bit 27). **The arithmetic corroborates it**: 0x2800_0000 M2 cycles
   at 1.789773 MHz is 6 min 15 s, and *Nintendo World Championships*' limit was 6 min 21 s. It
   also settles a conflict between the two sources — the wiki says the switches set "the counter's
   **initial** value", Disch that they set the value it must reach, and only Disch's reading gives
   the page's own "roughly 5 to 10 minutes" **and** the tournament's length. Mesen 2 implements
   Disch's (`0x20000000 | (GetDipSwitches() << 25)`).
3. **The locked state, exactly.** Disch's table makes it a flat first 32 KiB and Mesen 2
   implements that (`SelectPrgPage2x(0, 0)`); the wiki adds a parenthetical that "MMC1 PRG A14 is
   connected to both PRG ROMs (so writes to $8000 could select either 16+16 banking mode, and then
   writes to $E000 could cause either the first or second 16K of the first ROM to be mapped at
   both $8000 and $C000 before the timer was unlocked)". The two disagree, the note is the only
   statement of it anywhere, and nothing the one game does can show the difference — it unlocks
   before it banks. **The flat 32 KiB is implemented and the refinement is recorded here**, in the
   type's own note, as the thing a later lot would need a cartridge to settle. No Q ticket: this
   is a conflict between two public documents about an optional board, not a gap in the spec.

### The bar, and the stop condition that did not fire

**PR-1's thirteen**, against `mf6b-0-9f9d9d41` — every ROM below, **and the five MMC1 ROMs are
five of them**:

| ROM | board | `9f9d9d41` | **step 3** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | **MMC1** | 3 310 470 388 | **3 308 309 361** | −2 161 027 | −0.065 % |
| `oamstr` | NROM | 3 319 466 313 | **3 318 480 760** | −985 553 | −0.030 % |
| `m1` | **MMC1** | 4 886 651 514 | **4 885 520 719** | −1 130 795 | −0.023 % |
| `m0` | NROM | 5 521 219 795 | **5 519 962 642** | −1 257 153 | −0.023 % |
| `m4` | MMC3 | 5 473 783 790 | **5 471 994 836** | −1 788 954 | −0.033 % |
| `metroid` | **MMC1** | 5 116 149 734 | **5 115 868 405** | −281 329 | −0.005 % |
| `smb1` | NROM | 5 996 882 321 | **5 996 355 879** | −526 442 | −0.009 % |
| `mm2` | **MMC1** | 6 020 725 910 | **6 017 875 886** | −2 850 024 | −0.047 % |
| `z2` | **MMC1** | 6 292 582 908 | **6 291 433 990** | −1 148 918 | −0.018 % |
| `readbuf` | CNROM | 6 836 830 137 | **6 834 577 090** | −2 253 047 | −0.033 % |
| `duelito` | NROM | 7 383 094 194 | **7 380 483 839** | −2 610 355 | −0.035 % |
| `smb3` | MMC3 | 7 950 795 166 | **7 949 429 710** | −1 365 456 | −0.017 % |
| `mmc5` | MMC5 | 11 289 605 855 | **11 286 840 994** | −2 764 861 | −0.024 % |
| **total** | | **79 398 258 025** | **79 377 134 111** | **−21 123 914** | **−0.027 %** |

`callgrind-diff.py`: **"no ROM slower, total lower -- keepable"**.

**Level 2.** All five `core` figures below the baseline; **two `total` figures above it**, and
both are the layout noise the lot's own rule names:

| game/stage | `9f9d9d41` | **step 3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 898 492 | **11 147 223 960** | −674 532 | −0.006 % |
| `smb1/core` | 11 515 980 923 | **11 515 146 178** | −834 745 | −0.007 % |
| `zelda2/core` | 12 777 709 570 | **12 775 647 562** | −2 062 008 | −0.016 % |
| `mm2/core` | 12 043 306 623 | **12 036 872 563** | −6 434 060 | −0.053 % |
| `smb3/core` | 15 318 626 455 | **15 315 412 686** | −3 213 769 | −0.021 % |
| `metroid/total` | 13 736 176 888 | **13 735 588 274** | −588 614 | −0.004 % |
| `zelda2/total` | 15 362 221 166 | **15 361 403 682** | −817 484 | −0.005 % |
| `mm2/total` | 18 982 867 263 | **18 976 355 375** | −6 511 888 | −0.034 % |
| `smb1/total` | 13 589 021 252 | 13 592 170 876 | **+3 149 624** | +0.023 % |
| `smb3/total` | 24 192 639 363 | 24 195 011 633 | **+2 372 270** | +0.010 % |

**The diff, as the rule requires it.** `l2-diff.py`'s per-stage table puts the whole of both rises
in one stage: `smb1/convert` **+3 979 870** against `smb1/lua_hooks` **+3 979 870** — the same
figure to the instruction, `convert` being `lua_hooks`'s caller — and `smb3/convert` +5 129 101
against `smb3/lua_hooks` +5 579 822. Every other stage of both games is flat or down, `ppu`
−332 640 and −318 560 included. Per function, with `l2sym.py` over the two annotations:

| game | symbol | delta |
| --- | --- | ---: |
| `smb1` | `ltable.c:getgeneric` | **+2 200 700** |
| `smb1` | `ltable.c:luaH_getshortstr` | **+1 913 851** |
| `smb1` | `Cpu6502::branch` | −502 105 |
| `smb1` | `Ppu2C02::run_dots` | −332 640 |
| `smb3` | `ltable.c:getgeneric` | **+3 782 744** |
| `smb3` | `ltable.c:luaH_set` | **+1 223 386** |
| `smb3` | `ltable.c:luaH_getshortstr` | **+225 754** |
| `smb3` | `malloc.c:malloc_consolidate` | −793 516 |
| `smb3` | `malloc.c:unlink_chunk.isra.0` | −409 149 |
| `smb3` | `memmove-vec-unaligned-erms.S:__memcpy_avx_unaligned_erms` | +395 970 |
| `smb3` | `Cpu6502::branch` | **−2 931 497** |
| `smb3` | `Ppu2C02::run_dots` | −339 383 |

**Not one of the risen symbols is this project's code.** `getgeneric`, `luaH_set` and
`luaH_getshortstr` are Lua's hash table, `malloc_consolidate` and `unlink_chunk` are glibc's
allocator, `__memcpy_avx_unaligned_erms` is its `memcpy` — and the two symbols the step *does*
touch are **down** on both games. §5.11 measured that column's floor at ±57 k per ROM on
`luaH_getshortstr` alone and MF-6a2 step 2 measured ±2.7 M on a level-2 `total` against 0 on
`core`; this is that, on a 3D pack whose Lua tables rehash at a different point because the
binary's layout moved. `core` is the column the lot is read on and all five are below.

### The post-LTO invariants, and the hook that came back to the baseline

`shape.py`, the three dumps:

| symbol | `9f9d9d41` | step 2 | **step 3** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 443 | 1 / 2 / 3 645 | 1 / 2 / 3 626 |
| `Cpu::resolve` | 2 / 6 / 1 854 | 2 / 6 / 1 958 | 2 / 6 / 1 948 |
| `Cpu::branch` | 1 / 1 / 418 | 1 / 1 / 468 | 1 / 1 / 457 |
| `NesBus::read_cycle` | 1 / 65 / 181 | 1 / 65 / 181 | 1 / 65 / 181 |
| `NesBus::tick` | 1 / 95 / 82 | 1 / 95 / 82 | 1 / 95 / 82 |
| `NesBus::read_internal` | 1 / 17 / 175 | 1 / 17 / 175 | 1 / 17 / 175 |
| `NesBus::write_internal` | 1 / 10 / 3 103 | 1 / 10 / 3 145 | 1 / 10 / 3 465 |
| `Ppu::run_dots` | 1 / 2 / 1 907 | 1 / 2 / 1 907 | 1 / 2 / 1 907 |

`grep -oE '^_RN[^:]*irq_asserted[A-Za-z0-9_]*:'` over the dump still returns
`IrqBoard::irq_asserted` and nothing else; `callsites.py` has `Board::irq_asserted`,
`NesBus::irq_asserted` and `Cartridge::irq_asserted` all at **0 defs / 0 sites**;
`prologues.py` has **all twenty** pinned prologues identical.

**`Board::ppu_peek` is back to 272** — the baseline is 271 — because the fourth variant pushed
`IrqBoard::ppu_peek` out of line into a definition of its own, 66 instructions. That is why step
2's windfall on the two MMC3 ROMs (`ppu_peek` −2 288 640 on `m4`, −4 473 536 on `smb3`) is not in
this step's table: those ROMs are now carried by `branch` alone and are −1 788 954 and −1 365 456
instead. The capped hook came out of the step one instruction above where it started, which is
the better of the two outcomes to keep.

The rest: `Board::ppu_read` 354 → 356, `Board::on_cpu_cycle` 396 → 422 with
`NesEvent::on_cpu_cycle` 9 instructions out of line, `Board::on_ppu_address_bus` **156 → 156**,
`NesBus::catch_up_ppu` **188 → 188**, `Ppu2C02::read_nametable_inner` **114 → 114**,
`Mmc1::update_banks` **215 → 215, byte-identical**.

**`Cartridge::default_cpu_read` is 305 → 375**, +70, where step 2 left it at 322. `IrqBoard` now
forwards `prg_ram_enabled` — the NES-EVENT is the only board of the variant that needs it — and
that body inlines through `Board::prg_ram_enabled` into this function. **The dynamic figure did
not move**: `mmc5` is +7 456 756 on it in step 2 and +7 456 756 in step 3, to the instruction, and
`mmc5` overall is −2 764 861 against the reference. So step 2's warning stands but is no worse:
the margin step 4 has on that ROM is still about 2.8 M.

**Only one of the three PRG-RAM methods is forwarded**, deliberately:
`Mapper::prg_ram_writable`'s default calls `prg_ram_enabled`, and the NES-EVENT's 8 KiB of
unbanked PRG-RAM is exactly what `Mapper::prg_ram_offset`'s default already answers. One body on
the $6000-$7FFF decode instead of three.

### Tests

**877 passed, 0 failed, 8 ignored**, step 2's 868 plus nine:

| test | what it pins |
| --- | --- |
| `req_core_100_the_nes_event_counter_fires_at_its_dip_switch_target` | the 30-bit comparator at $28000000 **exactly** — two cycles short, the IRQ is two cycles away |
| `req_core_100_the_nes_event_timer_bit_resets_the_counter_and_acknowledges` | `I` set: "reset to 0 and stays there (does not count), and the pending IRQ is acknowledged", and `I` clear again restarting from zero rather than resuming |
| `req_core_100_only_the_nes_event_timer_bit_touches_its_counter` | the other three registers not resetting it: a hundred cycles short, two serial writes (ten ticks each) leave eighty |
| `req_core_100_the_nes_event_hardwires_the_first_32_kib_until_the_timer_is_unlocked` | the two-edge unlock, with the half-done sequence written first |
| `req_core_100_the_nes_event_chip_select_picks_both_the_rom_half_and_the_prg_register` | `O` choosing the chip **and** the register, all three MMC1 modes on the second chip, bit 0 unused, and `B` being three bits |
| `req_core_100_no_nes_event_register_moves_its_unbanked_chr` | the MMC1's CHR mode bit and both CHR registers moving nothing |
| `req_core_100_the_nes_event_carries_eight_kib_of_chr_ram` | the board's 8 KiB against REQ-CORE-012's 32 |
| `req_core_100_the_nes_event_wram_disable_is_e000_and_never_the_timer_bit` | **the trap**: a NWC dump is `Mmc1::is_snrom`'s exact signature, so a delegated `prg_ram_enabled` would read the timer bit as a RAM disable — and it powers on set, so the save RAM would never be readable at all |
| `req_core_100_the_nes_event_takes_its_mirroring_from_the_mmc1` | all four of the chip's own layouts |

**How the counter is tested at all.** Nothing on this board writes the counter — the switches set
the value it must *reach* and `I` only ever zeroes it — so reaching the comparator costs
671 088 640 bus ticks, which is minutes of wall clock. `nes_event_at_counter` runs the counter for
a distinctive number of cycles, **finds that number** in the savestate block, patches it and
reloads: the offset is not hard-coded, the helper asserts the marker appears exactly once, and it
therefore also pins that the counter is part of the state (REQ-CORE-090, REQ-CORE-091) — if it
were not serialised the search would fail rather than the test passing vacuously.

`req_core_100_every_required_mapper_is_registered`'s list is extended with 105, and the four
sweeps needed nothing: 105 went into `clocks_on_cpu_cycle`, it touches no PPU bus, and
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` and
`req_core_090_the_mapper_state_survives_a_save_and_restore` both pass over it.

`cargo fmt --check` clean; `cargo clippy --release -p zax-nes --all-targets -- -D warnings`
clean, **no new `allow` of any kind**.

The one title is *Nintendo World Championships 1990*, for R0 to run — and it is the one cartridge
in the lot whose whole point is a six-and-a-quarter-minute timer, so the thing to watch is that
the IRQ arrives at all rather than when.

[PROGRESS] MF-6b step 3/8 done: mapper 105 holding an `Mmc1` rather than parameterising one, `Mmc1::update_banks` byte-identical, all five MMC1 ROMs below `9f9d9d41` (set −21 123 914) and the two level-2 `total` rises entirely inside Lua's `ltable.c`.

## Step 4 — mapper 48, and the two instructions that cost `mmc5` nine million

### The board

nesdev ("INES Mapper 048"): the TC0690 is "a superset of TC0190+PAL16R4 abilities", so the
banking is mapper 33's with the register mask **widened from `$A003` to `$E003`** — and that one
change is the whole board, because it is what frees `$C000`-`$C003` for the IRQ and `$E000` for a
mirroring register of its own. On a TC0190 a write to `$C000` masks down to `$8000` and moves
PRG; here it is the reload value.

The IRQ is "exactly like MMC3's IRQs, with 2 exceptions":

1. **"The written reload value is inverted."** Stored inverted on the way in, so nothing else in
   the board has to remember. `req_core_100_the_tc0690_reload_value_is_inverted` writes `$14`
   both ways round: inverted it is a 21-line period, raw it is 236 lines.
2. **"The IRQ seems to trip a little later than it does on MMC3. It looks like about a 4 CPU
   cycle delay."** MR-64-4's shape, reused rather than re-invented: the counter **arms**
   `irq_delay` and `on_cpu_cycle` counts it down, which is `Rambo1::irq_delay` to the line.

The four prerequisites §5 step 4 lists were all already in the tree, so the step did not have to
build any of them: `A12Watcher::filter_dots` has been a construction parameter with
`min_rise_interval()` derived from it since MF-4c/MF-5a (MR-64-3), and MR-64-4's delayed
assertion since the same lot. The board is one `A12Watcher::new(A12Watcher::MMC3_FILTER_DOTS)`
and one `i32`.

**`irq_delay` is armed with the page's figure plus one, and the ordering inside `NesBus::tick` is
why.** That function runs `catch_up_ppu` — where the A12 rise, and so the arming, happens —
*before* `Cartridge::step_cpu_cycle`, so the very cycle the rise lands in decrements the counter
itself. An MMC3 asserting on that rise is observable at the end of that cycle; four cycles later
is four cycles after that. `Rambo1::irq_delay` encodes a different quantity the same way: its
page counts **M2 falls** from the rise and the arming cycle's own fall is the first of them, so
its 2 is literally two falls where this 4 is four cycles of difference. The measurement is
`mf_6b_the_tc0690_counts_the_same_lines_as_the_mmc3_and_trips_four_cycles_later`, which runs a
real mapper-4 MMC3 and a mapper-48 TC0690 with the same latch on the same instrument and asserts
the difference is exactly 4.

### Three divergences from Mesen 2, and one of them a game can see

1. **The delay is 4 and not 6 or 22.** `TaitoTc0690.h:32-34` picks between those two by submapper
   with a comment that says what they are: "6 removes the shaking from The Jetsons, 21-22 seems
   to work for the Flintstones+Captain Saver". Tuned against games; the page gives one number.
2. **`$C000` and `$C001` do not acknowledge.** Mesen clears the IRQ source in both
   (`TaitoTc0690.h:60`, `:67`): "Flintstones expects either $C000 or $C001 to clear the irq flag".
   The page's two exceptions do not include it and an MMC3's own latch and clear do not either.
   **This is the one divergence of this board that a game can see**, and it is named for R0: if
   *The Flintstones: The Rescue of Dino & Hoppy* shows a shaking or mistimed status bar, that arm
   is where to look. `req_core_100_the_tc0690_irq_registers_are_the_mmc3_s_at_c000_to_c003` pins
   the page's reading, so changing it later is one assertion.
3. **No submapper.** Mesen reads `_romInfo.SubMapperID == 1`; the page documents none.

And one place the page disagrees with **itself**: its prose says bit 4 of `$E000` selects the
mirroring and its own register diagram says `[.M.. ....]`, which is bit 6. The diagram and Mesen
(`value & 0x40`) agree, so bit 6 it is, and
`req_core_100_the_tc0690_mirroring_is_bit_6_of_e000` walks the whole byte — `$10` among the
values that must change nothing — rather than asserting the one bit.

The counter is **written out and not shared with `Mmc3`**: sharing would mean lifting the MMC3's
five IRQ fields into a struct of their own, and that is precisely what this step's stop condition
forbids. Step 1 measured what moving an IRQ board's fields by one byte costs (eleven of thirteen
ROMs, +0.009 % to +0.027 %, no code added). `Mmc3::update_banks` is **161 → 161** and
`Mmc1::update_banks` **215 → 215, byte-identical**, which is the receipt.

### V1 missed the bar, and the two instructions that did it

**V1 forwarded `ppu_irq_horizon` from `IrqBoard` with `#[inline(never)]`** — step 1's trick, and
the shape MF-6a2 step 2 used on `Rambo1::ppu_irq_horizon`. It was **above the reference on two
ROMs**:

| ROM | `9f9d9d41` | **V1** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `mmc5` | 11 289 605 855 | 11 296 018 331 | **+6 412 476** | +0.057 % |
| `z2` | 6 292 582 908 | 6 292 758 949 | **+176 041** | +0.003 % |

and the per-symbol diff named one function on **four** ROMs: `NesBus::catch_up_ppu`, +9 214 379
on `mmc5`, +2 433 068 on `duelito`, +2 245 658 on `readbuf`. Statically it had gone from **188
instructions to 190**. `metroid`, which was −281 329 at step 3, came out at −33 803.

`m4` and `smb3` — §5's own named stop condition — were **below** the reference the whole time
(−1 398 273 and −1 018 201). The board that paid was the MMC5, through a function the board has
nothing to do with.

**Why two instructions are worth nine million.** `NesBus::ppu_horizon` is inlined into
`catch_up_ppu`, and it folds in the whole `Board::ppu_irq_horizon` dispatch. *Any* forward from
`IrqBoard` is one more distinct body there — even one whose four other boards answer `Mapper`'s
default, even with the leaf out of line. This is MF-6a2 step 2's finding read backwards: that
step took the RAMBO-1's body *out* of this dispatch, took `catch_up_ppu` from 216 to 203, and
bought **every board in the library** 0.005 % to 0.082 %. Two instructions in that function are
worth more than thirty in most.

**V2 answers `None` for the whole variant, and `None` is already in the dispatch** — it is the
MMC5's arm — so it adds no body at all. `Mapper::ppu_irq_horizon`'s own contract makes it the
safe answer: "no bound is known (the PPU then runs in step with the CPU)". The price is that a
cartridge of this variant gives up the PPU's lag; the TC0690 could not have kept it anyway, and
the other four boards could but pay for the family, which is §4.10's rule and exactly the trade
`IrqBoard::chr_view` already makes one dispatch further on.

`NesBus::catch_up_ppu` is **188 → 188** in V2, and the function does not appear in the per-symbol
diff at all.

### The bar, V2 on both instruments

**PR-1's thirteen** — every ROM below, set total **−21 419 036 (−0.027 %)**:

| ROM | board | `9f9d9d41` | **V2** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 310 470 388 | **3 308 320 074** | −2 150 314 | −0.065 % |
| `oamstr` | NROM | 3 319 466 313 | **3 318 448 961** | −1 017 352 | −0.031 % |
| `m1` | MMC1 | 4 886 651 514 | **4 885 452 838** | −1 198 676 | −0.025 % |
| `m0` | NROM | 5 521 219 795 | **5 519 917 503** | −1 302 292 | −0.024 % |
| `m4` | **MMC3** | 5 473 783 790 | **5 471 936 909** | −1 846 881 | −0.034 % |
| `metroid` | MMC1 | 5 116 149 734 | **5 115 846 055** | −303 679 | −0.006 % |
| `smb1` | NROM | 5 996 882 321 | **5 996 373 588** | −508 733 | −0.008 % |
| `mm2` | MMC1 | 6 020 725 910 | **6 017 890 404** | −2 835 506 | −0.047 % |
| `z2` | MMC1 | 6 292 582 908 | **6 291 475 824** | −1 107 084 | −0.018 % |
| `readbuf` | CNROM | 6 836 830 137 | **6 834 540 841** | −2 289 296 | −0.033 % |
| `duelito` | NROM | 7 383 094 194 | **7 380 400 848** | −2 693 346 | −0.036 % |
| `smb3` | **MMC3** | 7 950 795 166 | **7 949 411 924** | −1 383 242 | −0.017 % |
| `mmc5` | MMC5 | 11 289 605 855 | **11 286 823 220** | −2 782 635 | −0.025 % |
| **total** | | **79 398 258 025** | **79 376 838 989** | **−21 419 036** | **−0.027 %** |

**Level 2** — `l2-diff.py --bar`: **"at or below the baseline on every game, total and core"**,
and step 3's two `total` rises are gone, which is the other half of the evidence that they were
Lua's layout and not this lot's code:

| game/stage | `9f9d9d41` | **V2** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 898 492 | **11 147 223 960** | −674 532 | −0.006 % |
| `smb1/core` | 11 515 980 923 | **11 515 146 178** | −834 745 | −0.007 % |
| `zelda2/core` | 12 777 709 570 | **12 775 647 562** | −2 062 008 | −0.016 % |
| `mm2/core` | 12 043 306 623 | **12 036 872 563** | −6 434 060 | −0.053 % |
| `smb3/core` | 15 318 626 455 | **15 315 412 686** | −3 213 769 | −0.021 % |
| `metroid/total` | 13 736 176 888 | **13 735 498 841** | −678 047 | −0.005 % |
| `smb1/total` | 13 589 021 252 | **13 586 873 570** | −2 147 682 | −0.016 % |
| `zelda2/total` | 15 362 221 166 | **15 360 551 856** | −1 669 310 | −0.011 % |
| `mm2/total` | 18 982 867 263 | **18 976 424 548** | −6 442 715 | −0.034 % |
| `smb3/total` | 24 192 639 363 | **24 189 903 784** | −2 735 579 | −0.011 % |

Every `core` figure is **identical to step 3's to the instruction** — the change between V1 and
V2 is in a function the five games' cores do not reach differently — which is also why the
level-2 run was only needed once for V2.

### The post-LTO invariants

| symbol | `9f9d9d41` | step 3 | **step 4 (V2)** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 443 | 1 / 2 / 3 626 | 1 / 2 / **3 626** |
| `Cpu::resolve` | 2 / 6 / 1 854 | 2 / 6 / 1 948 | 2 / 6 / **1 948** |
| `Cpu::branch` | 1 / 1 / 418 | 1 / 1 / 457 | 1 / 1 / **457** |
| `NesBus::read_cycle` | 1 / 65 / 181 | 1 / 65 / 181 | 1 / 65 / 181 |
| `NesBus::tick` | 1 / 95 / 82 | 1 / 95 / 82 | 1 / 95 / 82 |
| `NesBus::read_internal` | 1 / 17 / 175 | 1 / 17 / 175 | 1 / 17 / 175 |
| `NesBus::write_internal` | 1 / 10 / 3 103 | 1 / 10 / 3 465 | 1 / 10 / 3 577 |
| `Ppu::run_dots` | 1 / 2 / 1 907 | 1 / 2 / 1 907 | 1 / 2 / 1 907 |

The three claimants of `Cpu6502::step`'s budget are **unmoved to the instruction** from step 3,
the fifth variant having cost that chain nothing at all. `grep` still finds
`IrqBoard::irq_asserted` and no other `irq_asserted` definition; `callsites.py` has all three
links at 0 defs / 0 sites; **all twenty prologues identical**.

`NesBus::catch_up_ppu` **188 → 188**, `Ppu2C02::read_nametable_inner` **114 → 114**,
`Mmc1::update_banks` **215 → 215**, `Mmc3::update_banks` **161 → 161**, `Board::ppu_peek` **272**
against the baseline's 271 — the fourth capped hook a single instruction above where the lot
found it, with `IrqBoard::ppu_peek` out of line at 86. The two that grew are the ones this board
is: `Board::on_ppu_address_bus` 156 → 195, with `IrqBoard::on_ppu_address_bus` out of line at 38,
and `Board::on_cpu_cycle` 396 → 426 with `TaitoTc0690::on_cpu_cycle` 9.

`Cartridge::default_cpu_read` is 305 → 377, two instructions more than step 3's 375, and its
dynamic figure on `mmc5` is **+7 456 756 — the same number as step 2 and step 3, to the
instruction**. Three steps have now added an `IrqBoard` variant without moving it, so step 2's
warning can be narrowed: what cost that function its +53 was `prg_ram_enabled` becoming a
forwarded body in step 3, not the variant count, and `mmc5`'s margin against the reference is
still −2 782 635.

### Tests

**883 passed, 0 failed, 8 ignored**, step 3's 877 plus six:

| test | what it pins |
| --- | --- |
| `req_core_100_the_tc0690_reload_value_is_inverted` | the XOR, in both directions: `$14` inverted is 21 lines and raw is an order of magnitude more |
| `mf_6b_the_tc0690_counts_the_same_lines_as_the_mmc3_and_trips_four_cycles_later` | the counter against a real MMC3 on the same instrument, and the delay as **exactly** four cycles. §5 calls this the step's `ppu_irq_horizon` test, "the one test in the lot whose failure is a scheduling bug rather than a mapper bug": the PPU runs behind the CPU by whatever the horizon allows, so a wrong horizon shows here as a difference that is not four |
| `req_core_100_the_tc0690_irq_registers_are_the_mmc3_s_at_c000_to_c003` | `$C002` enabling, `$C003` disabling and acknowledging, and **neither `$C000` nor `$C001` acknowledging**, which is the divergence from Mesen a game can see |
| `req_core_100_an_a12_dip_shorter_than_the_filter_does_not_clock_the_tc0690` | §5's own case: three dots of low rejected, twenty-four accepted, with A12 driven high *before* the counter is armed so the rise under test is the first the watcher sees |
| `req_core_100_the_tc0690_banks_like_a_tc0190_through_the_wider_e003_mask` | the banking, six bits of PRG, the whole byte in the 2 KiB CHR registers, `$8004` collapsing to `$8000`, and the IRQ registers banking nothing |
| `req_core_100_the_tc0690_mirroring_is_bit_6_of_e000` | the whole byte, `$10` included, and `$E001`-`$E003` not being registers |

`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` **caught the new board by
itself** and failed until its own expected list was extended: `[4, 4, 5, 5, 48, 48, 64, 64, 118,
118]`. That is the sweep working as MF-3b intended — a board was added to
`watches_ppu_reads`/`watches_ppu_bus_without_read` and the guard against a vacuous pass is what
noticed. Its second assertion, that a board watching the PPU's reads must answer `None` to
`chr_view`, passed untouched: `IrqBoard::chr_view` has answered `None` for the whole variant
since step 1.

`req_core_100_every_required_mapper_is_registered` is extended with 48;
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched`,
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` and
`req_core_090_the_mapper_state_survives_a_save_and_restore` needed nothing.

`cargo fmt --check` clean; `cargo clippy --release -p zax-nes --all-targets -- -D warnings`
clean, **no new `allow` of any kind**.

The six titles for R0 are *Bakushou!! Jinsei Gekijou 3*, *Bubble Bobble 2*, *Don Doko Don 2*,
*Captain Saver*, *The Jetsons: Cogswell's Caper!* and — the one to run first, for the reason
above — *The Flintstones: The Rescue of Dino & Hoppy*.

[PROGRESS] MF-6b step 4/8 done: mapper 48, with `ppu_irq_horizon` answering `None` for the whole variant after a forward cost `catch_up_ppu` two instructions and `mmc5` 9.2 M; all thirteen ROMs and all five level-2 games below `9f9d9d41` on total and core.

## Step 5 — mapper 158, a `const` parameter, and a chain that had to be pinned

### The board, in one sentence of the page's own

nesdev ("INES Mapper 158"): the Tengen 800037 "modifies RAMBO-1 (mapper 64) in the exact same way
that TLSROM (118) differs from MMC3 (4), by which we mean the RAMBO-1's CHR A17 output is
connected to CIRAM A10". So bit 7 of the CHR bank registers picks the nametable, `$A000` is
bypassed, and **nothing else moves** — `mf_6b_tengen_800037_banks_exactly_as_a_rambo1_does` gives
a mapper-64 and a mapper-158 image the same seven registers and asserts the same windows.

Which registers drive it is the TxSROM's rule read onto this chip's CHR layout: it is the ones
mapped to the **first pattern table**, and bit 7 of `$8000` chooses them — clear, the two 2 KiB
registers R0 and R1 are at `$0000-$0FFF` and take two nametables each; set, the four 1 KiB
registers R2-R5 are there and take one each. That is the same bit, read the same way round, that
`update_banks` already uses for its `high`/`low` swap, and Mesen 2 reads it identically
(`Rambo1_158.h:10-32`). Registers 8 and 9, which fill the odd kilobytes of 1 KiB mode, reach no
nametable: there are four pages and the four 1 KiB registers already name them.

**`TLSROM` is a `const` parameter and not a field** — MR-4-3's rule, the one `Mmc3Board`'s
`MC_ACC` follows. A field would have been *two* fields on the RAMBO-1 (a flag and the four
pages), and step 1 priced that move exactly: one byte of displacement on `VrcIrq::pending`
re-laid out `Cpu6502::branch` and cost eleven of thirteen ROMs +0.009 % to +0.027 % with no code
added at all. The receipt is one line of `cmp4c.py`: **`Rambo1::update_banks` 139 → 139,
byte-identical**, which is `Rambo1Board<false>`'s monomorphisation.

And the pages are **computed, not cached**: `Mmc3Board` keeps a `nametable_pages: [i32; 4]` field
because it predates the cache `Cartridge` now holds, and with `Cartridge::nametable_pages_vary`
naming 158 the board is asked four times per register write and never on a fetch. A field would
have bought nothing and cost the RAMBO-1 its layout.

`Rambo1Board<true>` goes in [`IrqBoard`] as its sixth inner variant, so there is **no new
`Board` discriminant** — `McAcc`, the other monomorphisation of this kind, has one of its own
because it predates that variant.

### Four shapes, and the one that is below the bar

| shape | what changed | set total | `m1` | `mmc5` |
| --- | --- | ---: | ---: | ---: |
| **V1** | the board as designed | **−18 091 239** | −8 415 | −2 917 065 |
| **V2** | V1 + `#[inline(never)]` on `IrqBoard::nametable_page` | **−18 251 797** | −3 727 | −2 919 202 |
| **V3** (**shipped**) | V2 + `#[inline(always)]` on `Cartridge::cpu_write` | **−19 980 904** | **−784 152** | −2 814 125 |
| **V4** | V3 + `#[inline(never)]` on `IrqBoard::prg_ram_enabled` | **+43 399 080** | **+382 906** | **+56 855 937** |

**V1 was already below the reference on all thirteen**, but `m1` by 8 415 instructions — inside
the ±57 k-per-ROM floor §5.11 measured, so effectively *at* it. One function explains the whole
of that: `Cartridge::cpu_write` had gone **out of line**, 0 → 2 448 instructions, while
`NesBus::write_internal` fell from 3 577 to 1 102 as its body moved out. The call that appeared
between the two cost `m4` **+5 089 659** against −3 537 021 in its caller, `m1` +3 776 318 against
−2 658 017, and `m0` +3 543 400 — about 1.2 M net on three ROMs whose boards this step does not
touch. What tipped it is the second `Rambo1` monomorphisation: `Board::write_register` now holds
the RAMBO-1's register decode twice, and the two together are past what the cost model will
inline into one caller.

**V2 is the attempt that did not work, and it is recorded because it looked like the obvious
one.** `IrqBoard::nametable_page` out of line shrinks `Board::nametable_page`, which shrinks
`Cartridge::refresh_nametable_pages`, which is called from `cpu_write` — and it moved the set by
**+160 597**, thirteen ROMs' worth of floor. `cpu_write` outlined with exactly the same figures
after it. The attribute is kept because it keeps that cold dispatch out of a hot caller on
principle, but it is **not** what fixed this and the table says so.

**V3 pins the chain instead of shortening it**, which is PERFORMANCE-HISTORY §5.10's own
conclusion and the fourth time this project has reached for it: `#[inline(always)]` on
`Cartridge::cpu_write`, beside the four MF-5b and MF-6a left on `Board::irq_asserted`,
`Board::expansion_audio_output`, `Board::try_read_prg` and `Board::cpu_read_override`. This
function and `write_internal` are one body as far as every measurement of this project is
concerned, and which of the two holds it is LLVM's choice; the step takes the choice away.
`write_internal` is **3 580** instructions and `Cartridge::cpu_write` has no symbol at all.
That is the one new `allow(clippy::inline_always)` of the lot, on MF-5b-fix's terms, and it
carries its measurement in the comment.

**V4 is refuted, and its refutation is the useful part.** `Board::prg_ram_writable` had gone out
of line in V3 — 0 → 55 instructions, the sixth inner variant having tipped *that* dispatch — at a
cost of **+430 380** over 600 level-2 frames of *Super Mario Bros. 3*. Taking `IrqBoard`'s
`prg_ram_enabled` body out of line to shrink it put `Board::prg_ram_enabled` behind a `callq` on
the MMC5's read path instead: `Cartridge::default_cpu_read` **+67 110 804 instructions over 300
frames of `mmc5exram`**, +0.504 % on that ROM, to save 0.002 % on a game this board is not. **The
55 instructions stay**, and the comment on that method now says so with both numbers, so the
next lot does not try it again.

The shipped tree is V3's code with V4's refutation written into a doc comment, which cannot
change codegen.

### The bar

**PR-1's thirteen** — every ROM below, set total **−19 980 904 (−0.025 %)**:

| ROM | board | `9f9d9d41` | **V3** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 310 470 388 | **3 308 318 543** | −2 151 845 | −0.065 % |
| `oamstr` | NROM | 3 319 466 313 | **3 318 452 542** | −1 013 771 | −0.031 % |
| `m1` | MMC1 | 4 886 651 514 | **4 885 867 362** | −784 152 | −0.016 % |
| `m0` | NROM | 5 521 219 795 | **5 520 387 914** | −831 881 | −0.015 % |
| `m4` | MMC3 | 5 473 783 790 | **5 472 568 488** | −1 215 302 | −0.022 % |
| `metroid` | MMC1 | 5 116 149 734 | **5 115 903 585** | −246 149 | −0.005 % |
| `smb1` | NROM | 5 996 882 321 | **5 996 358 632** | −523 689 | −0.009 % |
| `mm2` | MMC1 | 6 020 725 910 | **6 017 842 189** | −2 883 721 | −0.048 % |
| `z2` | MMC1 | 6 292 582 908 | **6 291 432 170** | −1 150 738 | −0.018 % |
| `readbuf` | CNROM | 6 836 830 137 | **6 834 524 765** | −2 305 372 | −0.034 % |
| `duelito` | NROM | 7 383 094 194 | **7 380 398 198** | −2 695 996 | −0.037 % |
| `smb3` | MMC3 | 7 950 795 166 | **7 949 431 003** | −1 364 163 | −0.017 % |
| `mmc5` | MMC5 | 11 289 605 855 | **11 286 791 730** | −2 814 125 | −0.025 % |
| **total** | | **79 398 258 025** | **79 378 277 121** | **−19 980 904** | **−0.025 %** |

**Level 2** — all five `core` figures below, one `total` above:

| game/stage | `9f9d9d41` | **V3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 898 492 | **11 147 278 310** | −620 182 | −0.006 % |
| `smb1/core` | 11 515 980 923 | **11 515 125 238** | −855 685 | −0.007 % |
| `zelda2/core` | 12 777 709 570 | **12 775 641 366** | −2 068 204 | −0.016 % |
| `mm2/core` | 12 043 306 623 | **12 036 819 361** | −6 487 262 | −0.054 % |
| `smb3/core` | 15 318 626 455 | **15 315 558 069** | −3 068 386 | −0.020 % |
| `metroid/total` | 13 736 176 888 | **13 735 567 846** | −609 042 | −0.004 % |
| `smb1/total` | 13 589 021 252 | **13 588 385 656** | −635 596 | −0.005 % |
| `zelda2/total` | 15 362 221 166 | **15 359 169 158** | −3 052 008 | −0.020 % |
| `mm2/total` | 18 982 867 263 | **18 976 209 352** | −6 657 911 | −0.035 % |
| `smb3/total` | 24 192 639 363 | 24 193 872 632 | **+1 233 269** | +0.005 % |

**The diff for `smb3/total`, per function**, as the rule requires — and this one is *not* entirely
Lua, so the arithmetic is given rather than the label:

| symbol | delta |
| --- | ---: |
| `ltable.c:luaH_getshortstr` | **+3 442 500** |
| `Cpu6502::branch` | **−2 931 497** |
| `ltable.c:luaH_set` | +566 326 |
| `Board::prg_ram_writable` | +430 380 |
| `Ppu2C02::run_dots` | −339 383 |
| `NesBus::write_internal` | −264 172 |

Lua's hash table accounts for **+4 008 826** and this project's own symbols for **−3 104 672**,
which nets the +1 233 269. Without the `ltable.c` column the game is 3.1 M below the baseline;
`core`, the column the lot is read on, is −0.020 %. The one figure of ours that rose is
`Board::prg_ram_writable`'s 430 380, which V4 went after and which V4's own measurement says to
leave alone.

### The post-LTO invariants

| symbol | `9f9d9d41` | step 4 | **step 5 (V3)** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 443 | 1 / 2 / 3 626 | 1 / 2 / **3 626** |
| `Cpu::resolve` | 2 / 6 / 1 854 | 2 / 6 / 1 948 | 2 / 6 / **1 948** |
| `Cpu::branch` | 1 / 1 / 418 | 1 / 1 / 457 | 1 / 1 / **457** |
| `NesBus::read_cycle` | 1 / 65 / 181 | 1 / 65 / 181 | 1 / 65 / 181 |
| `NesBus::tick` | 1 / 95 / 82 | 1 / 95 / 82 | 1 / 95 / 82 |
| `NesBus::read_internal` | 1 / 17 / 175 | 1 / 17 / 175 | 1 / 17 / 175 |
| `NesBus::write_internal` | 1 / 10 / 3 103 | 1 / 10 / 3 577 | 1 / 10 / 3 580 |
| `Ppu::run_dots` | 1 / 2 / 1 907 | 1 / 2 / 1 907 | 1 / 2 / 1 907 |

The three claimants of `Cpu6502::step`'s budget are **unmoved to the instruction** across steps 4
and 5, and two new boards have now been added without touching them. `IrqBoard::irq_asserted` is
still the only `irq_asserted` definition in the dump; `callsites.py` has the three links at 0
defs / 0 sites.

`NesBus::catch_up_ppu` **188 → 188**, `Mmc1::update_banks` **215 → 215 byte-identical**,
`Rambo1::update_banks` **139 → 139 byte-identical**, `Mmc3::update_banks` 161 → 161,
`Board::ppu_peek` **272** against the baseline's 271 with `IrqBoard::ppu_peek` out of line at 133.
`Board::on_ppu_address_bus` 156 → 242 with `IrqBoard::on_ppu_address_bus` out of line at 85,
`Board::on_cpu_cycle` 396 → 425, `Cartridge::default_cpu_read` 305 → 379,
`Board::nametable_page` out of line at 44 with `IrqBoard::nametable_page` at 13, and
`Board::prg_ram_writable` out of line at 55.

**Nineteen of the twenty pinned prologues are identical, and the twentieth did not grow.**
`prologues.py` flags `Board::on_cpu_cycle`, and the diff is register allocation: the three
`pushq`es and the stack frame are the same instructions in the same order, and from the sixth
instruction on the same `movzbl`/`movl`/`cmov` trio uses `%ecx` and `%eax` the other way round
(`cmovael %eax, %ecx` against `cmovbl %eax, %ecx` with the operands swapped). MF-6a's criterion
for this instrument is "a `pushq` or a bigger `subq $N, %rsp` is a cost to every game", and
neither happened.

### Tests

**887 passed, 0 failed, 8 ignored**, step 4's 883 plus four:

| test | what it pins |
| --- | --- |
| `req_core_100_tengen_800037_takes_two_nametables_from_each_2_kib_register` | the four combinations of bit 7 of R0 and R1, which is `req_core_100_txsrom_takes_two_nametables_from_each_2_kib_register`'s own table on this chip |
| `req_core_100_tengen_800037_takes_one_nametable_from_each_1_kib_register` | the other arrangement, R2-R5 one page each, and registers 8 and 9 reaching none |
| `req_core_100_tengen_800037_ignores_the_mirroring_register` | `$A000` having no effect — **and a mapper-64 RAMBO-1 in the same test still honouring it**, with its pages still `FOLLOW_MIRRORING` whatever bit 7 of its CHR registers holds, which is what says the `const` parameter did not reach mapper 64 |
| `mf_6b_tengen_800037_banks_exactly_as_a_rambo1_does` | the seven registers of the 1 KiB mode, 8 and 9 included, giving the same windows on both numbers |

`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` **caught the missing latch**:
158 is a RAMBO-1 and watches A12, and the sweep failed with its own message
("Add the mapper number to the matches! in Cartridge::create") until `watches_ppu_reads` and
`watches_ppu_bus_without_read` had it. Its expected list is now
`[4, 4, 5, 5, 48, 48, 64, 64, 118, 118, 158, 158]`. That is the second step running in which
that sweep has found a real omission before any measurement did.

`req_core_100_every_required_mapper_is_registered` is extended with 158, and
`nametable_pages_vary` names it — §4.5's cheap level, a page override and not a
`read_nametable`.

`cargo fmt --check` clean; `cargo clippy --release -p zax-nes --all-targets -- -D warnings`
clean, with **one** new `allow`, `clippy::inline_always` on `Cartridge::cpu_write`, which the
lot's own terms permit and which carries its measurement.

The one title is *Alien Syndrome* (Tengen, 1989), for R0 to run.

[PROGRESS] MF-6b step 5/8 done: mapper 158 as a `const` parameter of `Rambo1`, `Rambo1::update_banks` byte-identical, `Cartridge::cpu_write` pinned after a second monomorphisation outlined it at 1.2 M on three ROMs, and all thirteen ROMs below `9f9d9d41` (set −19 980 904).
