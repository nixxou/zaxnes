# Lot MF-5b-fix — giving back the inlining MF-5b lost on the pack path

MF-5b ended at `fcfa5cb3`. Its step 1 traded twenty-five *inlined* copies of the thirty-two-arm
`Board::irq_asserted` dispatch for twenty-five `callq`s to one shared copy, which let
`Cpu6502::execute_without_operand` fold back into `Cpu6502::step`, and measured the trade as a
gain on **all thirteen ROMs** of lot PR-1's set. R0 then measured the same commit with the
**level-2** run — the five commercial games, each with its 3D pack, 600 frames after the
harness's 120 of warm-up, with inputs pressed — and two of the five are *above* `a577f336`:

| ROM (with its 3D pack) | total | core stage |
| --- | --- | --- |
| Zelda II | **+21,282,232 (+0.138 %)** | +0.172 % |
| SMB1 | **+8,170,674 (+0.060 %)** | +0.058 % |
| SMB3 | −0.060 % | −0.093 % |
| Mega Man 2 | −0.256 % | −0.402 % |
| Metroid | −0.060 % | −0.072 % |

The lot's job: both helpers inlined into `step` again, no behaviour changed, **and** PR-1's
thirteen kept at `fcfa5cb3`'s counts or lower. A plain revert of MF-5b step 1's shape would meet
the first bar and fail the second, which is what makes this a measurement lot rather than a
one-line one.

## Steps

1. The instrument: a committed level-2 script, and both regressions reproduced from clean builds.
2. The diagnosis: what left `step`, what it cost, and why PR-1's thirteen could not see it.
3. The variants, each with its numbers, and the one that ships.
4. The bar: level-2's five games, PR-1's thirteen ROMs.
5. The checks: `fmt`, clippy, the suites, and the whole test-ROM campaign against MF-5b's.

## Step 1 — the instrument, and the two numbers reproduced

### Why PR-1's script is not enough, and what the level-2 one adds

`rust/tools/perf/callgrind-set.sh` is lot PR-1's instrument: thirteen ROMs, 300 frames, **no
pack, no input, no warm-up**. It measures the emulated core alone. The level-2 run differs in
three ways, and each of them is why this regression was invisible to the thirteen:

* a **3D pack** is loaded (`--profile`), so conversion, the Lua frame hooks and the draw-list
  build run — and the widescreen extension's execution zone is live, which is what puts
  `WideExtension::zone_offset` on the instruction-fetch path at all;
* **600 frames** after the harness's default 120 of warm-up, not 300 with none;
* **inputs are pressed**, so the games leave their title and file screens and run actual play.

Two files are added to `rust/tools/perf/`, in the shape of the four PR-1 committed:

| file | what it does |
| --- | --- |
| `roms-l2.txt` | the five games and their packs, `<key> <rom> \| <pack>` (a pipe, both paths having spaces) |
| `callgrind-l2.sh <label> [jobs]` | the level-2 callgrind run; writes `<key>.ir`, `<key>.ann`, `<key>.inc` and `stages.txt` |
| `l2-diff.py <dir-a> <dir-b> [--bar]` | the per-stage table; `--bar` prints only `total` and `core` and the verdict |

`<key>.ann` and `<key>.ir` are named as `callgrind-set.sh` names them, so **`callgrind-diff.py`
reads a level-2 directory unchanged** — the per-symbol diff below is that script, with no second
copy of its parser.

`stages.txt` is the one thing the level-2 run needs that PR-1's did not: the figures R0 publishes
are not the whole process but the **inclusive** count of one function per stage
(`total` = `bench_core::run`, `core` = `NesCore::run_frame`, `ppu` = `Ppu2C02::run_dots`, and so
on), taken from `callgrind_annotate --inclusive=yes`. Where a name has several entries — the tool
lists a function once per file its inlined code came from — the largest is the whole call tree,
and that is the one taken. `ZAXNES_LUA_GC=smooth` is exported by the script rather than left to
the default of the day, the collector's mode changing what the frame hooks cost.

### The commands

Host side (Git Bash), two clean trees from `git archive`:

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh a577f336 rust/target-mf/tmp/a577f336.tar
    sh rust/tools/perf/make-tree.sh 04a5d8c1 rust/target-mf/tmp/tip.tar

WSL2 Ubuntu side (`rustc 1.99.1`, `valgrind-3.22.0`), each tree extracted into a **brand-new**
directory with its **own** `CARGO_TARGET_DIR`, which `build-clean.sh` does and then deletes:

    export PATH="$HOME/.cargo/bin:$PATH"
    export PR1_WORK=$HOME/mf5bfix
    W=/mnt/g/3dsen/cleanroom/wt-mappers
    sh $W/rust/tools/perf/build-clean.sh base-a577f336 $W/rust/target-mf/tmp/a577f336.tar
    sh $W/rust/tools/perf/build-clean.sh tip-04a5d8c1  $W/rust/target-mf/tmp/tip.tar

    export ZAX_L2_ROMS=/mnt/g/3DSen/test-originals
    export ZAX_L2_PACKS=/mnt/g/3DSen/pm-test/profiles-stage
    sh $W/rust/tools/perf/callgrind-l2.sh base-a577f336 5
    sh $W/rust/tools/perf/callgrind-l2.sh tip-04a5d8c1  5

    export ZAX_TEST_ROMS=/mnt/g/3dsen/cleanroom/assets-free/test-roms
    export ZAX_COMM_ROMS=/mnt/g/3dsen/demo/roms
    sh $W/rust/tools/perf/callgrind-set.sh base-a577f336 8
    sh $W/rust/tools/perf/callgrind-set.sh tip-04a5d8c1  8

    python3 $W/rust/tools/perf/l2-diff.py       l2-base-a577f336 l2-tip-04a5d8c1 --bar
    python3 $W/rust/tools/perf/callgrind-diff.py l2-base-a577f336 l2-tip-04a5d8c1 2000000
    python3 $W/rust/tools/perf/callgrind-diff.py cg-base-a577f336 cg-tip-04a5d8c1

The tip is `04a5d8c1`, MF-6's end; MF-6 is a document-only lot, so its tree compiles to
`fcfa5cb3`'s code. Each level-2 run takes about eleven minutes of wall clock with five jobs, each
PR-1 set about a minute with eight; a clean build is 35 s.

### The level-2 bar, reproduced

`l2-diff.py --bar`, `a577f336` → `04a5d8c1`, both built here:

| game/stage | `a577f336` | `04a5d8c1` | delta | | R0's delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| `zelda2/total` | 15 402 925 330 | 15 424 039 498 | **+21 114 168** | **+0.137 %** | +21 282 232 |
| `zelda2/core` | 12 818 624 858 | 12 840 624 231 | **+21 999 373** | **+0.172 %** | +0.172 % |
| `smb1/total` | 13 680 903 406 | 13 689 047 460 | **+8 144 054** | **+0.060 %** | +8 170 674 |
| `smb1/core` | 11 608 529 144 | 11 615 319 550 | **+6 790 406** | **+0.058 %** | +0.058 % |
| `smb3/total` | 24 230 216 871 | 24 220 319 439 | −9 897 432 | −0.041 % | −0.060 % |
| `smb3/core` | 15 359 181 750 | 15 344 841 348 | −14 340 402 | −0.093 % | −0.093 % |
| `mm2/total` | 19 024 545 977 | 18 975 899 733 | −48 646 244 | −0.256 % | −0.256 % |
| `mm2/core` | 12 085 128 624 | 12 036 519 940 | −48 608 684 | −0.402 % | −0.402 % |
| `metroid/total` | 13 826 992 757 | 13 818 996 502 | −7 996 255 | −0.058 % | −0.060 % |
| `metroid/core` | 11 238 744 231 | 11 230 617 763 | −8 126 468 | −0.072 % | −0.072 % |

**The `core` column is R0's to the instruction** — `metroid/core` 11 238 744 231 → 11 230 617 763
is character for character the row in R0's `vs-a577f336.txt`, and so are the other four. That is
the check that the instrument has not drifted: the emulated core does not depend on where the
tree was built, so an identical core count on a different machine path means the binary's hot
code is the binary R0 measured.

The `total` column is within 0.07 % of R0's and its *deltas* are within 170 k of R0's
(`zelda2` +21 114 168 against +21 282 232). The small standing difference is in `load_profile`
and `lua_hooks`, the two stages that read files and run Lua and so depend on the build path's
length and on the host; neither is a stage this lot touches, and both are reported below.

### The two symbols, reproduced

`callgrind-diff.py` on the level-2 directories, floor 2 M. Every figure the brief names comes out:

| game | symbol | `a577f336` | `04a5d8c1` | delta |
| --- | --- | ---: | ---: | ---: |
| `zelda2` | `execute_without_operand` | 145 455 592 | **0** | −145 455 592 |
| `zelda2` | `Cpu6502::step` | 802 289 583 | 896 278 848 | +93 989 265 |
| `zelda2` | **`<Board as Mapper>::irq_asserted`** | **0** | **56 834 892** | **+56 834 892** |
| `zelda2` | **`WideExtension::zone_offset`** | **0** | **16 642 696** | **+16 642 696** |
| `smb1` | `execute_without_operand` | 589 121 611 | **0** | −589 121 611 |
| `smb1` | `Cpu6502::step` | 577 554 085 | 1 073 794 512 | +496 240 427 |
| `smb1` | **`<Board as Mapper>::irq_asserted`** | **0** | **97 104 372** | **+97 104 372** |
| `smb1` | **`WideExtension::zone_offset`** | **0** | **2 554 167** | **+2 554 167** |
| `smb3` | `irq_asserted` | 0 | 47 885 076 | +47 885 076 |
| `metroid` | `irq_asserted` | 0 | 90 013 236 | +90 013 236 |
| `mm2` | `irq_asserted` | 0 | *below the 2 M floor* | — |

Two things in that table are worth keeping, because they shape step 3:

* **`zone_offset` has a count on `smb1` and `zelda2` only.** The other three packs do not run code
  in the execution zone, so `read_code`'s zone test answers "no" from a constant there and the
  call never happens. The two games that *do* are exactly the two that are above the baseline.
* **`irq_asserted` costs `metroid` 90 M and still leaves it 8 M below the baseline**, because the
  `execute_without_operand` fold is worth 429.7 M → 330.4 M = 99.3 M there. The regression is not
  "the calls cost something"; it is that on `zelda2` and `smb1` the fold is worth *less* than the
  calls, and PR-1's 300-frame no-pack runs are the regime where it is worth more.

### PR-1's thirteen, reproduced, and the count this lot must not lose

`callgrind-diff.py cg-base-a577f336 cg-tip-04a5d8c1`. Every ROM is below `a577f336`, which is
MF-5b step 5's own table reproduced (`z2` −0.013 %, `oamstr` −0.632 %, `smb3` −0.085 %):

| ROM | `a577f336` | `04a5d8c1` — **the second bar** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `oamstr` | 3 339 440 632 | **3 318 333 668** | −21 106 964 | −0.632 % |
| `smb1` | 6 032 398 559 | **6 000 514 816** | −31 883 743 | −0.529 % |
| `mm2` | 6 038 204 350 | **6 017 251 716** | −20 952 634 | −0.347 % |
| `m4` | 5 500 905 288 | **5 482 298 196** | −18 607 092 | −0.338 % |
| `m1` | 4 914 885 719 | **4 900 185 476** | −14 700 243 | −0.299 % |
| `readbuf` | 6 852 874 003 | **6 835 936 948** | −16 937 055 | −0.247 % |
| `duelito` | 7 398 932 523 | **7 380 729 013** | −18 203 510 | −0.246 % |
| `m0` | 5 544 079 149 | **5 533 124 867** | −10 954 282 | −0.198 % |
| `vblnmi` | 3 328 802 922 | **3 322 241 995** | −6 560 927 | −0.197 % |
| `mmc5` | 11 331 604 501 | **11 311 610 019** | −19 994 482 | −0.176 % |
| `smb3` | 7 967 708 299 | **7 960 902 506** | −6 805 793 | −0.085 % |
| `metroid` | 5 154 485 933 | **5 150 997 694** | −3 488 239 | −0.068 % |
| `z2` | 6 312 893 679 | **6 312 096 132** | −797 547 | −0.013 % |
| **total** | **79 717 215 557** | **79 526 223 046** | −190 992 511 | **−0.240 %** |

The right-hand column is the second bar: **every one of the thirteen at that number or lower.**

[PROGRESS] MF-5b-fix step 1/5 done: the instrument and both numbers reproduced.

## Status

Step 1 done. Step 2 next.
