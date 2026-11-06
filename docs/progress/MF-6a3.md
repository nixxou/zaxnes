# Lot MF-6a3 — 119, 47 and 37 restored under the owner's tolerance, one Cargo feature per board, and mapper 116

The baseline and the performance reference are both **`07325f6e`**, the end of MF-6b.

**The owner's two decisions of 2026-11-06, which are what this lot is:**

1. **The tolerance.** MF-6a2 step 3 built 119, 47 and 37, measured them at **+0.004 % to
   +0.017 %** on six of PR-1's thirteen and **`mm2/core` +0.018 %** at level 2, and reverted them
   on the brief's stop-the-lot condition. A cost that small is now accepted: **at most +0.02 %
   per ROM** on both runs, against the base commit, for a step that adds boards with real titles,
   provided the three claimants of `Cpu6502::step` stay inlined (0 / 0 / 0) and `zone_offset`
   does not get dearer. So the three boards go in **enabled by default**.
2. **One Cargo feature per board that raises any figure.** Within the tolerance the feature is in
   `zax-nes`'s `default`; above it, it is left out and the umbrella `parked-boards` enables all
   the left-out ones. The lot applies this to 119, 47 and 37 **and retrofits it onto every board
   of MF-6a, MF-6a2 and MF-6b whose step measured a rise**, and ends with
   `docs/review/BOARD-FEATURES.md`.

Read first: `docs/progress/MF-6a2.md` **step 3** (the design, the page readings, the two Mesen
divergences, the six tests and the six measured shapes of 119, 47 and 37 — it is the whole of
what the rebuild needs), `docs/progress/MF-6b.md` **step 6** (mapper 116's full design and its
one Q-132 gap), `MF-6a.md`, and `rust/docs/PERFORMANCE-HISTORY.md` §5.10 to §5.13.

## Steps

0. The plan, both instruments and the post-LTO dump reproduced at `07325f6e`, and the
   `parked-boards` feature created, off by default.
1. **119 (TQROM), 47 and 37** rebuilt on `Mmc3Board` from MF-6a2 step 3's design, with its six
   tests, behind `board-119` and `board-mmc3-multicart`, both in `default`.
2. **Measured** on both instruments against `07325f6e`, ROM by ROM, against the tolerance.
3. **One feature per board for the boards of MF-6a, MF-6a2 and MF-6b that measured a rise**, at
   the granularity the measurements exist at.
4. **Mapper 116 (Somari)** from MF-6b step 6's design, behind `board-116`, with Q-132.
5. The checks: `fmt`, clippy both ways, the suites both ways, the `--ignored` pair, the campaign.
6. `docs/review/BOARD-FEATURES.md`, the inventory, and the journal.

## The bar, at every step

* **The tolerance**: at most **+0.02 % per ROM** on both runs — level 2 `core` *and* `total`, and
  PR-1's thirteen — against `07325f6e`.
* **The three claimants of `Cpu6502::step`** stay at 0 / 0 / 0, and `zone_offset` does not get
  dearer.
* After each step, the post-LTO check on the five symbols the brief names
  (`Cpu6502::step`, `Board::ppu_peek`, `NesBus::write_internal`, `Ppu2C02::run_dots`,
  `NesBus::catch_up_ppu`).

A level-2 `total` rise whose per-function diff lands entirely in Lua's `ltable.c` or in `malloc`
is layout noise — MF-6a2 step 2 measured that column's run-to-run floor at **±2.7 M** against
**0** on `core`, and MF-6b read every one of its own `total` rises that way and was right twice
over (its step 4 re-measured step 3's two rises away). The `core` column decides.

### The labels are MF-6a's shape, deliberately

`rust/target-mf/mf63.sh` is this lot's driver — scratch, not committed, as `mf6a.sh`, `mf62.sh`
and `mf6b.sh` were, because `rust/target-*` is never staged. Every label it is given has the form
**`mf63-<step>-<8 chars>`**, 15 characters, the length of MF-6a's `mf6a-<step>-<hash>`, MF-6a2's
`mf62-<step>-<hash>` and MF-6b's `mf6b-<step>-<hash>`. MF-6a step 0 measured why: the same commit
built at two paths of different lengths differs by up to 1.56 M instructions on a level-2
`total`, all of it in `load_profile` and `lua_hooks`, which open files and run Lua whose paths are
longer or shorter. Keeping the length identical makes every figure below directly comparable with
the tables in `MF-6a.md`, `MF-6a2.md` and `MF-6b.md`.

## Step 0 — the baseline, on three instruments, and the feature switch

### The commands

Host (Git Bash):

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 07325f6e rust/target-mf/tmp/mf63-0-07325f6e.tar

WSL2 Ubuntu-24.04, through the driver, which is four lines of environment around
`rust/tools/perf/`'s own scripts — `PR1_WORK=$HOME/mf63` so this lot's scratch is its own:

    sh rust/target-mf/mf63.sh build mf63-0-07325f6e      # 39 s, then the tree is deleted
    sh rust/target-mf/mf63.sh set   mf63-0-07325f6e      # 13 ROMs, 8 jobs
    sh rust/target-mf/mf63.sh asm   mf63-0-07325f6e      # post-LTO asm, into target-mf/asm/
    sh rust/target-mf/mf63.sh l2    mf63-0-07325f6e      # 5 games, 5 jobs, about six minutes

Every one in the **foreground** with a long timeout, never backgrounded: a headless session ends
as soon as it stops answering, and a process backgrounded inside a `wsl.exe` call is killed when
the call returns (MF-6a step 0). The `wsl.exe` invocations go through PowerShell and not Git
Bash, which rewrites a `/mnt/g/...` argument into `C:/Program Files/Git/mnt/g/...`.

### Instrument 1: the five level-2 games — **all five `core` figures identical to the instruction**

`stages.txt` of `l2-mf63-0-07325f6e`, beside MF-6b step 5's shipped V3 column, which is
`07325f6e`'s code:

| game/stage | MF-6b step 5 (V3) | **`mf63-0-07325f6e`** | delta |
| --- | ---: | ---: | ---: |
| `metroid/core` | 11 147 278 310 | **11 147 278 310** | **0** |
| `smb1/core` | 11 515 125 238 | **11 515 125 238** | **0** |
| `zelda2/core` | 12 775 641 366 | **12 775 641 366** | **0** |
| `mm2/core` | 12 036 819 361 | **12 036 819 361** | **0** |
| `smb3/core` | 15 315 558 069 | **15 315 558 069** | **0** |
| `metroid/total` | 13 735 567 846 | 13 735 514 951 | −52 895 |
| `smb1/total` | 13 588 385 656 | 13 587 898 562 | −487 094 |
| `zelda2/total` | 15 359 169 158 | 15 358 834 193 | −334 965 |
| `mm2/total` | 18 976 209 352 | 18 976 300 081 | +90 729 |
| `smb3/total` | 24 193 872 632 | 24 187 868 326 | −6 004 306 |

**Five measurement sessions, four scratch roots and four label prefixes apart, and every
level-2 `core` figure is the same to the instruction.** That is MF-5a step 1's drift check at its
strongest, and it is now the fifth lot in a row to get that line. The `total` column moves by up
to 6.0 M — above the ±2.7 M floor MF-6a2 step 2 measured, and in the favourable direction on four
of the five — which is the column's own behaviour and not a code change: there is no code change,
the tar is `git archive 07325f6e`.

The right-hand `core` column is **the bar for the whole lot**.

### Instrument 2: PR-1's thirteen

| ROM | board | MF-6b step 5 (V3) | **`mf63-0-07325f6e`** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 318 543 | 3 308 378 921 | +60 378 | +0.002 % |
| `oamstr` | NROM | 3 318 452 542 | 3 318 459 130 | +6 588 | +0.000 % |
| `m1` | MMC1 | 4 885 867 362 | **4 885 841 284** | −26 078 | −0.001 % |
| `m0` | NROM | 5 520 387 914 | 5 520 389 483 | +1 569 | +0.000 % |
| `m4` | MMC3 | 5 472 568 488 | **5 472 545 224** | −23 264 | −0.000 % |
| `metroid` | MMC1 | 5 115 903 585 | 5 115 944 243 | +40 658 | +0.001 % |
| `smb1` | NROM | 5 996 358 632 | **5 996 346 457** | −12 175 | −0.000 % |
| `mm2` | MMC1 | 6 017 842 189 | 6 017 889 238 | +47 049 | +0.001 % |
| `z2` | MMC1 | 6 291 432 170 | **6 291 431 663** | −507 | −0.000 % |
| `readbuf` | CNROM | 6 834 524 765 | 6 834 547 836 | +23 071 | +0.000 % |
| `duelito` | NROM | 7 380 398 198 | **7 380 393 405** | −4 793 | −0.000 % |
| `smb3` | MMC3 | 7 949 431 003 | 7 949 452 274 | +21 271 | +0.000 % |
| `mmc5` | MMC5 | 11 286 791 730 | 11 286 830 608 | +38 878 | +0.000 % |
| **total** | | **79 378 277 121** | **79 378 449 766** | **+172 645** | +0.0002 % |

Every ROM is within **±60 378** of MF-6b's own column, which is the ±57 k `luaH_getshortstr`
floor §5.11 measured, per ROM; the set total is +0.0002 % over thirteen ROMs. The right-hand
column is **the bar for the whole lot**, ROM by ROM.

### Instrument 3: the post-LTO dump

`pins63.py lin-mf63-0-07325f6e.s` — the three claimants plus the five symbols the brief names:

| symbol | def / sites / instructions |
| --- | --- |
| `Board::irq_asserted` | **0 / 0 / 0** |
| `Wide::zone_offset` | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / **3 626** |
| `Board::ppu_peek` | 1 / 4 / **272** |
| `NesBus::write_internal` | 1 / 10 / **3 580** |
| `Ppu::run_dots` | 1 / 5 / **1 907** |
| `NesBus::catch_up_ppu` | 1 / 12 / **188** |

and `shape.py` beside it, for the figures MF-6b's tables carry: `Cpu::resolve` 2 / 6 / **1 948**,
`Cpu::branch` 1 / 1 / **457**, `NesBus::read_cycle` 1 / 65 / 181, `tick` 1 / 95 / 82,
`read_internal` 1 / 17 / 175. **Every figure is MF-6b step 5's, to the instruction.**

`pins63.py` is new, and it is written the way MF-6b step 1 and step 2 had to fix `callsites.py`
and `cmp4c.py`: its owner patterns are anchored on the **v0 length prefix** (`5Board`, not
`Board`), because `Board` is a substring of `IrqBoard` and both of those instruments cried wolf
at `Board::irq_asserted` for exactly that reason. `rust/target-mf/asm/` is scratch and not
committed, so the script is recorded here.

### The feature switch

`rust/crates/zax-nes/Cargo.toml` gains a `[features]` section with `default = []` and
**`parked-boards = []`, off by default**. MF-6b step 8 records that R0 added that feature for its
step 2 onwards and that it "was never needed, because every board that missed the bar missed it
on a shape that was fixable" — so it was never written, and this step writes it. It is empty
while no board is parked, which is the state this lot also expects to end in; it exists so that
the switch is wired for a later lot that does park one, and so that `--features parked-boards`
is a configuration this lot checks either way.

An empty feature section changes no codegen, and the step is committed with no source file
touched, which is what makes step 1's measurement readable.

[PROGRESS] MF-6a3 step 0/6 done: the plan, all five level-2 `core` figures and all eight pinned post-LTO symbols reproduced at `07325f6e` to the instruction, and `parked-boards` created off by default.
