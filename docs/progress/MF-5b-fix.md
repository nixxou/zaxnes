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

## Step 2 — the diagnosis: one budget, three candidates, and why PR-1 could not see it

### What the post-LTO code says

`cargo rustc --release -p zax-harness --bin zax-harness -- --emit=asm` on the same two clean
trees, read with `target-mf/asm/shape.py` (new, untracked as the lot's rules require of anything
under `rust/target-mf/`): for each symbol, *definitions / call sites / instructions*.

| symbol | `a577f336` | `04a5d8c1` |
| --- | ---: | ---: |
| `<Board as Mapper>::irq_asserted` | **0 / 0 / 0** | 1 / 25 / 32 |
| `WideExtension::zone_offset` | **0 / 0 / 0** | 1 / 1 / 33 |
| `Cpu6502::execute_without_operand` | 1 / 1 / 2 402 | **0 / 0 / 0** |
| `Cpu6502::step` | 1 / 2 / **932** | 1 / 2 / **2 529** |
| `Cpu6502::resolve` | 2 / 6 / 1 800 | 2 / 6 / 1 847 |
| `Cpu6502::branch` | 1 / 1 / 395 | 1 / 1 / 406 |
| `NesBus::read_cycle`, `tick`, `read_internal` | 181, 82, 175 | **same** |
| `NesBus::write_internal` | 2 874 | 3 020 (the VRC7's register decode) |
| `Ppu2C02::run_dots` | 1 907 | 1 906 |

`step` 932 → 2 529 and `execute_without_operand` 2 402 → *no symbol* are the same event: MF-5b
step 5's table, reproduced to the instruction. **It is one budget with three claimants**, and at
either commit `step` holds two of the three:

* at `a577f336`: the dispatch and the zone test are pasted into `step`, and
  `execute_without_operand` is a `callq`;
* at `04a5d8c1`: `execute_without_operand` is folded in, and the other two are `callq`s.

### Why the fold is a gain on thirteen ROMs and a loss on two games

The fold is worth, per ROM, the difference between `execute_without_operand`'s exclusive count and
what `step` grows by. The calls it pays for cost what the table of step 1 says. On `m1`,
300 frames: `execute_without_operand` −92.7 M against `step` +58.4 M and `irq_asserted` +18.8 M, a
net −15.5 M. On Zelda II with its pack, 600 frames: −145.5 M against `step` +94.0 M,
`irq_asserted` +56.8 M and `zone_offset` +16.6 M, a net **+21.9 M**. Same trade, opposite sign.

Three measurements of step 1's symbol table are the reason the sign flips, and each of them is a
thing PR-1's set cannot see:

| what | on PR-1's set | at level 2 |
| --- | --- | --- |
| `zone_offset` is on the fetch path | it answers `None` from `code.is_empty()` — `duelito` 68 537, `oamstr` 319 802 | **Zelda II 16.6 M**, SMB1 2.6 M: the pack's code runs in the execution zone |
| the cartridge is actually asked for /IRQ | `NesBus::irq_asserted` is `apu.irq_asserted() \|\| cartridge…`, and the APU's frame IRQ short-circuits it on most ROMs — `mm2` **51 324**, `duelito` 51 156, `oamstr` 44 472, `readbuf` 44 160 | Metroid 90.0 M, SMB1 97.1 M, Zelda II 56.8 M, SMB3 47.9 M |
| how long a run is | 300 frames | 600 after 120 of warm-up, so every per-cycle cost is 2.4 × larger |

The second row is the one that matters most and is the least obvious: **`mm2` and `metroid` are
both MMC1 games and the same binary asks the cartridge 700 times more often on one than on the
other**, because Mega Man 2 leaves the APU's frame IRQ asserted and Metroid inhibits it. A ROM
whose APU short-circuits the sample pays for the dispatch's *code* and never runs it, so for
`duelito`, `oamstr`, `readbuf` and `mm2` there is nothing to win here and only size to lose. They
are four of PR-1's thirteen, and they are exactly the four that resist in step 3.

### The three candidates, and the one the brief rules out in advance

The brief lists four. One is refuted before anything is built: **a different place for the VRC7's
audio-silence test cannot help**, because `Vrc7::irq_asserted` is `self.irq.asserted()` — one load
of a `VrcIrq`'s `pending` — and `silenced` is read by `expansion_audio_output`, which is not on
this path at all. What the VRC7 added to the dispatch is an *arm*: the jump table's thirty-second
slot and an eleventh distinct arm body (`movzbl 676(%rdi)`, its `VrcIrq` sitting at an offset no
other board's does). The dispatch at `04a5d8c1` is

    movzbl 1368(%rdi), %eax ; subb $2 ; movzbl ; movl $5 ; cmovael ; movzbl
    leaq .LJTI1784_0(%rip) ; movslq (%rcx,%rax,4) ; addq ; jmpq *%rax
    ... 11 blocks of `movzbl <offset>(%rdi), %eax ; retq`, one of them `xorl %eax, %eax`

— 32 instructions, of which 21 of the 32 table slots land on the `xorl`. Nothing in it is dear;
what is dear is its *inline cost*, which LLVM charges per table slot and per block, and which one
more variant tipped over the threshold. So the three live candidates are the dispatch's **shape**,
the **attribute**, and **splitting the VRC7 arm out** — step 3 measures all three, and the shape
turns out to have a second, opposite cost that only the boards with an IRQ pay.

[PROGRESS] MF-5b-fix step 2/5 done: the diagnosis.

## Step 3 — eleven variants, and the three attributes that ship

Every variant was built from a `git ls-files` tar of the working tree into its own fresh
directory with its own `CARGO_TARGET_DIR`, dumped with `--emit=asm`, and run over **all thirteen
ROMs of PR-1's set** — which is the discriminating bar, being the one `fcfa5cb3` already won. The
labels are the build labels, so that the `target-mf/asm/lin-<label>.s` dumps and the
`cg-<label>/` directories can be read back; **there is no variant 8** (a tree superseded before it
was measured).

### What each variant is

| | `Board::irq_asserted` | `zone_offset` | `execute_without_operand` | other |
| --- | --- | --- | --- | --- |
| **v1** | `inline(always)` | — | — | |
| **v2** | `inline(always)` | `inline(always)` | — | |
| **v3** | `inline(always)` | `inline(always)` | `inline(always)` | **shipped** |
| **v4** | — | — | `inline(always)` | |
| **v5** | `inline(always)` | — | `inline(always)` | |
| **v6** | — | `inline(always)` | `inline(always)` | |
| **v7** | eleven-arm match, `_ => false`, not pinned | `inline(always)` | `inline(always)` | |
| **v9** | `inline(always)` | `inline(always)` | `inline(always)` | `zone_on()` guard inlined in `read_code` |
| **v10** | `inline(always)` | `inline(always)` | `inline(always)` | the zone-hit body outlined (`zone_code_fetch`, `inline(never)`) |
| **v11** | `inline(always)` | **not** pinned | `inline(always)` | `zone_on()` guard inlined in `read_code` |
| **v12** | `inline(always)` | `inline(always)` | `inline(always)` | `zone_offset`'s guard chain split into three early returns |

### PR-1's thirteen, every variant

| variant | set total | vs `a577f336` | vs `fcfa5cb3` | ROMs above `fcfa5cb3` | worst excess |
| --- | ---: | ---: | ---: | ---: | ---: |
| `a577f336` | 79 717 215 557 | — | +0.240 % | | |
| `fcfa5cb3` | 79 526 223 046 | −0.240 % | — | | |
| v2 | 79 841 286 836 | +0.156 % | +0.396 % | **13** | +39 880 188 `smb1` |
| v4 | 79 714 479 571 | −0.003 % | +0.237 % | **13** | +45 526 550 `smb1` |
| v5 | 79 537 433 123 | −0.226 % | +0.014 % | 6 | +39 979 902 `smb1` |
| v6 | 79 579 347 630 | −0.173 % | +0.067 % | 11 | +9 214 163 `m0` |
| v7 | 79 478 337 364 | −0.300 % | −0.060 % | 7 | +38 271 871 `m4` |
| v9 | 79 462 652 389 | −0.319 % | −0.080 % | 5 | +7 470 184 `duelito` |
| v10 | 79 429 436 783 | −0.361 % | −0.122 % | 4 | +6 052 523 `duelito` |
| v11 | 79 503 666 045 | −0.268 % | −0.028 % | 5 | +11 299 368 `duelito` |
| v12 | 79 423 100 603 | −0.369 % | −0.130 % | 4 | +4 792 336 `duelito` |
| **v3** | **79 417 939 646** | **−0.375 %** | **−0.136 %** | **4** | **+4 802 854 `duelito`** |
| **shipped** | **79 417 861 775** | **−0.376 %** | **−0.136 %** | **4** | **+4 794 923 `duelito`** |

(The shipped tree is v3 with the doc comments written; the 78 k between them is the build
directory's name inside the binary, not code — the post-LTO shapes are identical, see below.)

### What each variant taught, in the order it was measured

**v1 — the attribute alone is not enough, and it is a step backwards.** Pinning the dispatch puts
`Board::irq_asserted` back to *0 definitions, 0 call sites* at once, and `step` drops straight back
to **933** instructions with `execute_without_operand` out of line at 2 472: `a577f336`'s
arrangement, recovered by one attribute. But `zone_offset` stays out of line (1 / 1 / 33) where
`a577f336` had it inlined, so this is `a577f336` *minus* something. Not measured on the set — the
shape alone says it cannot beat `fcfa5cb3`, which beats `a577f336` by 0.240 %.

**v2 — two attributes without the third lose on all thirteen, and to both baselines.** +0.156 %
over `a577f336`, +0.396 % over `fcfa5cb3`, every ROM worse. The two helpers are inlined and
`execute_without_operand` is out of line again, and the fold is worth more than they are. This is
the variant that says **the fold has to be kept**, not merely tolerated.

**v4 — the fold alone, pinned, is worth nothing without the other two.** +0.237 % over
`fcfa5cb3`, all thirteen worse, `smb1` +45.5 M. The reason is in the asm: with
`execute_without_operand` folded in, `step` holds **fifteen** instruction-fetch sites, and
`zone_offset` goes from one call site to **fifteen**. A `callq` per fetch, for every game, pack or
no pack. Same story in **v5** (`smb1` +40.0 M, for the same fifteen).

**v6 — the fold plus the zone, without the dispatch.** +0.067 %, eleven ROMs worse. The dispatch
is the single biggest of the three on the ROMs that reach it.

**v7 — the eleven-arm dispatch: the shape works, and it has an opposite cost.** With the
twenty-one boards that cannot assert /IRQ folded into `_ => false`, the cost model inlines the
dispatch **on its own**, no attribute, and the generated code gets *better everywhere else*:
`Cpu6502::branch` **330** instructions (395 at `a577f336`, 406 at `fcfa5cb3`, 418 in v3),
`resolve` **1 552** (1 800 / 1 847 / 1 854). On the ROMs whose board cannot assert /IRQ it is the
best variant of the lot — `metroid` **−1.107 %**, `m1` −0.745 %, `z2` −0.654 %, `m0` −0.639 %
against `fcfa5cb3`. And LLVM compiles it to

    leal -6(%rax) ; cmpb $2 ; movzbl ; movl $1 ; cmovael ; xorl %eax, %eax
    cmpb $27, %cl ; ja <false>            ; the range of the eleven
    movzbl %cl, %ecx ; movl $255886211, %edx ; btl %ecx, %edx ; jae <false>   ; their bit mask
    leaq .Lswitch.table…(%rip) ; movzwl (%rdx,%rcx,2), %ecx ; …               ; a table of field offsets

— a bit mask and a table of **field offsets** instead of a jump table. Three instructions cheaper
for a board that answers "no" and three dearer for one that answers at all: **`m4` +0.698 %,
`smb3` +0.464 %, `mmc5` +0.278 %**, two of them MMC3s. That is MF-5b's refuted load-time latch
again, in a different disguise and with the same verdict, and it is why the thirty-two-arm
`dispatch!` stays. (The form is also safe to write — `_ => false` can be spelled as the twenty-one
variants named explicitly, so the compiler refuses a board that is in neither group — and it is
recorded here in case a future lot has a set where the MMC3s do not matter.)

**v9, v11 — a `zone_on()` guard in `read_code`, and why it is not free.** v3's residual cost is
`branch`: its *body* grows 406 → 418 and its *executed* path grows with it, by about two
instructions a fetch, because the pinned `zone_offset` materialises and spills the
`WideExtension` pointer (`leaq 3348(%r14) ; movq %rax, 24(%rsp)`) before the range test. The
obvious answer is a three-load guard in front of it, and it is wrong both ways round: in front of
the pinned form (**v9**, +7.5 M `duelito`, five ROMs above) and in front of an out-of-line
`zone_offset` (**v11**, +11.3 M `duelito`, five above, and `smb1` back to +4.8 M). The guard's own
three loads land on every fetch and buy less than they cost.

**v10 — outlining the zone-hit body does not remove the spill.** `zone_read_cycle` is already
`#[inline(never)]`; moving the `code().get(offset)` with it into a `zone_code_fetch` shrinks `step`
to 3 350 and `resolve` to 1 816, and `branch` keeps the `leaq`/`movq` pair anyway — the pointer is
spilled under register pressure, not for `code()`. Within 0.014 % of v3 on the set and on four of
the five games; kept out because it adds a function and buys nothing.

**v12 — splitting `zone_offset`'s guard chain into three early returns is a wash.** Set total
−0.369 % against v3's −0.375 %; `oamstr` +1.67 M against +2.10 M, `z2` −17.4 M against −18.2 M; at
level 2, better on Metroid (−0.663 % against −0.653 %) and worse on Zelda II (−0.256 % against
−0.263 %), SMB1 and SMB3. v3 wins three of the five games and the set total, and does it **with no
change to any function body**, so v12 is out and its numbers stay here.

### What ships

Three `#[inline(always)]`, each with its measurement on the function, and **nothing else** — no
body moved, no arm reordered, no new function, no new state:

| attribute | what it buys |
| --- | --- |
| `<Board as Mapper>::irq_asserted` | the 56.8 M of Zelda II's pack and the 97.1 M of SMB1's. The last link of the per-cycle /IRQ sample, and the one MF-5b left to the cost model |
| `WideExtension::zone_offset` | 16.6 M on Zelda II's pack; and, once the fold is pinned, the fifteen `callq`s per instruction that v4 and v5 measured |
| `Cpu6502::execute_without_operand` | the fold MF-5b got for free and pinning the dispatch takes back: +11.6 M to +13.1 M on every ROM of the set without it (v2) |

### The post-LTO code: MF-3b's rule, both ways

`shape.py`, the shipped tree against both baselines:

| symbol | `a577f336` | `04a5d8c1` | **shipped** |
| --- | ---: | ---: | ---: |
| `<Board as Mapper>::irq_asserted` | 0 / 0 / 0 | 1 / 25 / 32 | **0 / 0 / 0** |
| `WideExtension::zone_offset` | 0 / 0 / 0 | 1 / 1 / 33 | **0 / 0 / 0** |
| `Cpu6502::execute_without_operand` | 1 / 1 / 2 402 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu6502::step` | 932 | 2 529 | **3 443** |
| `Cpu6502::resolve` | 1 800 | 1 847 | 1 854 |
| `Cpu6502::branch` | 395 | 406 | 418 |
| `NesBus::read_cycle` | 181 | 181 | **181** |
| `NesBus::tick` | 82 | 82 | **82** |
| `NesBus::read_internal` | 175 | 175 | **175** |
| `NesBus::write_internal` | 2 874 | 3 020 | 3 020 |
| `Ppu2C02::run_dots` | 1 907 | 1 906 | 1 906 |

**Both symbols are inlined into `step` again — no definition and no call site anywhere** — and
`execute_without_operand` stays folded in, which is the arrangement neither baseline had. `step`
holds all three at 3 443 instructions, and the four symbols a per-cycle path cannot afford to lose
(`read_cycle`, `tick`, `read_internal`, `run_dots`) are **unmoved to the instruction**, with their
call-site counts unchanged (65, 95, 17, 2). `write_internal` and `run_dots` are MF-5b's numbers,
untouched by this lot.

`target-mf/asm/callsites.py` (MF-3b rule 1's own instrument, with MF-5b's widened list) reports
**no entry moved at all** between `a577f336` and the shipped tree: `Board::irq_asserted` 0 → 0
call sites, `NesBus::irq_asserted` and `Cartridge::irq_asserted` 0 → 0, `end_cycle` 0 → 0,
`Board::ppu_read` 4 → 4, `on_ppu_address_bus` 14 → 14, `read_nametable_inner` 14 → 14,
`panic_bounds_check` 911 → 911. The `FIRST CALL SITE -- stop the lot` that MF-5b's own run printed
and overruled is gone.

### Behaviour, and what was not touched

No body of any function changed: the diff is three attributes, their `allow(clippy::inline_always)`
and their doc comments, plus an addendum on `Cpu6502::execute`'s existing note saying that both
halves of the pair now carry the attribute. No `unsafe`. Nothing under
`G:/3dsen/cleanroom/spec/`. The save-state format is untouched — no field, no order, no block.
`cargo test --release -p zax-nes -p zax-harness -- --test-threads=1`: **816 passed, 0 failed,
8 ignored**, which is MF-6's count on the nose.

[PROGRESS] MF-5b-fix step 3/5 done: the three attributes, and eleven variants' numbers.

## Status

Steps 1-3 done. Step 4 next.
