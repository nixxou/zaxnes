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
