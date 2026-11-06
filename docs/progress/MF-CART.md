# Lot MF-CART — lift the limit on new boards, then un-park

The baseline **and** the performance reference is `44f77f72`, the end of MF-6c. Read first:
`docs/progress/MF-6c.md` (all of it — step 2's seven shapes, step 5's measurement of both shared
enums' limit, the plan revision `9367929e`, step 9's `DecodeBoard` limit),
`docs/progress/MF-6a3.md` step 1, `docs/review/BOARD-FEATURES.md`,
`rust/docs/PERFORMANCE-HISTORY.md` §5.10 onwards.

## The problem this lot exists for

Every new board now costs `mmc5` about **+7.46 M instructions (+0.066 %)** over 300 frames, above
the owner's **+0.02 %** tolerance, through the inlined `Board::prg_ram_enabled` dispatch inside
`Cartridge::default_cpu_read` — the MMC5 is the one board that maps PRG-RAM into $8000-$DFFF, so
`Cartridge::read_rom_window` cannot answer and **every** CPU read of an MMC5 game reaches that
function. MF-6c measured the figure **nine times**, in both directions, and found each shared
enum's own limit: `SmallBoard` ten variants, `IrqBoard` ten, `DecodeBoard` four. Two boards are
parked on it today — `board-taito-x1` (80, 207, 82, 552) and `board-111` — and twelve MF-UNL lots
(37 boards) plus MF-VS are planned after this one.

## Steps (n = 8)

0. The plan, the driver and the baseline at `44f77f72` on all three instruments.
1. **Reproduce** the limit: one more variant in each shared enum, priced on `mmc5`, and the symbol
   where it lands. The figure to beat.
2. **The structural fix**, built and green: the $6000-$7FFF read answer settled on `Cartridge`
   the way `rom_windows` and `nametable_pages` already are, so that `Cartridge::default_cpu_read`
   dispatches on `Board` for it no longer. No behaviour change.
3. **The fix measured**: the default build against the baseline, and step 1's experiment re-run.
   The target is that one more `Board` variant costs **no ROM more than +0.02 %**.
4. Other shapes, if step 3 does not reach the target (the jump tables of
   `Board::expansion_audio_output`, `ppu_peek`, `irq_asserted`).
5. **Un-park**: `board-taito-x1` and `board-111` re-measured on the new tree; each one inside the
   tolerance goes into `default`. `BOARD-FEATURES.md` updated.
6. **The new limit written down** in `PERFORMANCE-HISTORY.md`: how many more variants the shared
   enums, and `Board` itself, take before a ROM crosses +0.02 %, so MF-VS and the MF-UNL lots can
   plan for it.
7. The check: fmt, clippy, the suites in both configurations, the `--ignored` pair, golden-core and
   the campaign field by field.

## The bar, at every step

* Both runs at `44f77f72`'s figures or lower in the **default** build: PR-1's thirteen ROMs
  (`callgrind-set.sh` + `callgrind-diff.py`) and level 2's five games on `core` *and* `total`
  (`callgrind-l2.sh` + `l2-diff.py --bar`), against `mfct-0-44f77f72`, on clean `git archive`
  builds in their own directories.
* The three claimants of `Cpu6502::step` at **0 / 0 / 0**, `WideExtension::zone_offset` not dearer
  (the widescreen hack), and `Ppu2C02::read_pattern_planes` **unchanged**.
* `cargo fmt --check` clean, clippy `-D warnings` on the crates touched in `default` *and*
  `--features parked-boards` with no new `allow`, and the suites green in both.
* No behaviour change: the save-state format unchanged, golden-core and the campaign identical.

### The labels

`rust/target-mf/mfct.sh` is this lot's driver — scratch, never staged, as `mf6a.sh`, `mf62.sh`,
`mf6b.sh`, `mf63.sh` and `mf6c.sh` were, because `rust/target-*` is never committed. Every label
has the form **`mfct-<step>-<8 chars>`**, 15 characters, the length of every MF lot's since MF-6a
step 0 measured why: the same commit built at two paths of different lengths differs by up to
1.56 M instructions on a level-2 `total`, all of it in `load_profile` and `lua_hooks`, which open
files and run Lua whose paths are longer or shorter. `asmlinct.sh` and `symct.sh` are the same
two helpers under this lot's scratch root.

## Step 0 — the baseline at `44f77f72`, on three instruments

### The commands

Host (Git Bash):

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 44f77f72 rust/target-mf/tmp/mfct-0-44f77f72.tar

WSL2 Ubuntu-24.04 (valgrind-3.22.0, 24 cores), through the driver, every one in the
**foreground** with a long timeout: a headless session ends as soon as it stops answering, and a
process backgrounded inside a `wsl.exe` call is killed when the call returns (MF-6a step 0). The
`wsl.exe` invocations go through PowerShell and not Git Bash, which rewrites a `/mnt/g/...`
argument into `C:/Program Files/Git/mnt/g/...`.

    sh rust/target-mf/mfct.sh build mfct-0-44f77f72      # 45 s, then the tree is deleted
    sh rust/target-mf/mfct.sh set   mfct-0-44f77f72      # 13 ROMs, 8 jobs
    sh rust/target-mf/mfct.sh l2    mfct-0-44f77f72      # 5 games, 5 jobs
    sh rust/target-mf/mfct.sh asm   mfct-0-44f77f72      # post-LTO asm, into target-mf/asm/

### Instrument 1: the five level-2 games — **the bar for the whole lot**

`stages.txt` of `l2-mfct-0-44f77f72`, with MF-6c's own baseline column (`8496c503`) beside it so
that a reader can see the whole of MF-6c in the difference:

| game/stage | MF-6c step 0 (`8496c503`) | **`mfct-0-44f77f72`** | delta |
| --- | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 147 295 142** | **+0** |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** |
| `zelda2/core` | 12 775 651 048 | **12 775 651 048** | **+0** |
| `mm2/core` | 12 036 822 861 | **12 036 822 861** | **+0** |
| `smb3/core` | 15 315 558 322 | **15 315 537 499** | **−20 823** |
| `metroid/total` | 13 735 627 354 | 13 735 578 935 | −48 419 |
| `smb1/total` | 13 589 030 161 | 13 586 757 575 | −2 272 586 |
| `zelda2/total` | 15 360 284 960 | 15 360 318 280 | +33 320 |
| `mm2/total` | 18 976 349 017 | 18 976 430 445 | +81 428 |
| `smb3/total` | 24 187 154 526 | 24 188 986 419 | +1 831 893 |

**Four `core` figures identical to the instruction and the fifth is MF-6c's own −20 823**, the
figure its steps 6 to 9 all measured on `smb3/core`. So the reference is reproduced: this column
is `44f77f72`'s code and nothing else, and it is **the bar for the whole lot**.

### Instrument 2: PR-1's thirteen — **the bar, ROM by ROM**

| ROM | board | **`mfct-0-44f77f72`** | against `mf6c-9-gt111prk` |
| --- | --- | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 287 870 | −1 991 |
| `oamstr` | NROM | 3 318 485 575 | −33 716 |
| `m1` | MMC1 | 4 885 953 734 | +50 807 |
| `m4` | MMC3 | 5 472 522 428 | +54 688 |
| `m0` | NROM | 5 520 499 653 | −8 226 |
| `metroid` | MMC1 | 5 115 985 915 | +36 613 |
| `smb1` | NROM | 5 996 362 814 | −22 359 |
| `mm2` | MMC1 | 6 017 861 569 | +826 |
| `z2` | MMC1 | 6 291 442 776 | −45 179 |
| `readbuf` | CNROM | 6 834 534 036 | −62 362 |
| `duelito` | NROM | 7 380 421 165 | −14 602 |
| `smb3` | MMC3 | 7 949 489 726 | +34 859 |
| `mmc5` | MMC5 | **11 279 358 122** | **−6 196** |
| **total** | | **79 371 205 383** | **−16 838** |

The right-hand column is the **same code built under another label**, and every figure in it is
inside the ±57 k-per-ROM `luaH_getshortstr` floor §5.11 measured — which is what says the
instrument is the instrument. The one that matters is **`mmc5` −6 196**: `44f77f72` sits on the
*favourable* side of `Cartridge::default_cpu_read`'s two layouts, which is the side a successor
lot has to be able to stay on.

### Instrument 3: the post-LTO dump

`pins63.py lin-mfct-0-44f77f72.s`, with MF-6c step 9's shipped figures beside it:

| symbol | MF-6c step 9 | **`mfct-0-44f77f72`** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | **3 626** |
| `Board::ppu_peek` | 291 | **291** |
| `NesBus::write_internal` | 3 861 | **3 861** |
| `Ppu::run_dots` | 1 919 | **1 919** |
| `NesBus::catch_up_ppu` | 188 | **188** |

`shape.py` beside it: `Cpu::resolve` 2 / 6 / **1 948**, `Cpu::branch` 1 / 1 / **457**,
`NesBus::read_cycle` 1 / 65 / **181**, `tick` 1 / 95 / **82**, `read_internal` 1 / 17 / **175** —
every one MF-6c's figure to the instruction.

And the three functions this lot has to watch by name, with `fn.py`:

| symbol | MF-6c step 9 | **`mfct-0-44f77f72`** |
| --- | ---: | ---: |
| `Ppu2C02::read_pattern_planes` | 107 | **107** |
| `Ppu2C02::read_nametable_inner` | 75 | **75** |
| **`Cartridge::default_cpu_read`** | **429** | **429** |

The last row is the function this lot is about. `read_pattern_planes` at **107** is the figure
MF-2's shield has held for eight lots and the one no step here may move.

[PROGRESS] MF-CART step 0/8 done: the plan, the driver, and `44f77f72` reproduced on all three
instruments — four level-2 `core` figures identical to the instruction and `smb3/core` at MF-6c's
own −20 823, every ROM of PR-1's thirteen inside the ±57 k whole-process floor against MF-6c step
9's shipped build, `mmc5` on the **favourable** layout at −6 196, and all eleven pinned symbols
holding with `default_cpu_read` at **429**.

## Step 1 — the limit reproduced, and it is **three times** what MF-6c measured

### What was measured, and why these two experiments are the right ones

The brief asks for "one more variant in each shared enum, priced on `mmc5`". The two **parked
boards are exactly that experiment**, and they are also the two boards step 5 has to un-park, so
no synthetic variant was needed for either:

| experiment | label | what it adds | the enum |
| --- | --- | --- | --- |
| A | `mfct-1-x1small1` | `board-taito-x1` put into `default` | `SmallBoard`'s **eleventh and twelfth** variants |
| B | `mfct-1-gt111dec` | `board-111` put into `default` | `DecodeBoard`'s **fifth** variant |

`IrqBoard` is not covered here, for a reason: no parked board belongs to it, so pricing its
eleventh variant needs a synthetic one. MF-6c step 5 measured it (two more variants, two units)
and **step 6 of this lot re-prices all three enums on the fixed tree**, which is where the number
a successor needs lives.

Each experiment is one line of `rust/crates/zax-nes/Cargo.toml`'s `default` list, packed with
`make-tree.sh --worktree`; the file is back to `44f77f72`'s in the committed tree.

### Experiment A — `SmallBoard`'s eleventh and twelfth variants: `mmc5` **+0.198 %**

`callgrind-diff.py`, against `mfct-0-44f77f72`:

| ROM | `44f77f72` | **experiment A** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 287 870 | **3 308 284 219** | −3 651 | −0.000 % |
| `oamstr` | 3 318 485 575 | **3 318 480 156** | −5 419 | −0.000 % |
| `m1` | 4 885 953 734 | **4 885 941 716** | −12 018 | −0.000 % |
| `m4` | 5 472 522 428 | 5 472 580 799 | +58 371 | +0.001 % |
| `m0` | 5 520 499 653 | **5 520 471 147** | −28 506 | −0.001 % |
| `metroid` | 5 115 985 915 | **5 115 936 722** | −49 193 | −0.001 % |
| `smb1` | 5 996 362 814 | **5 996 355 331** | −7 483 | −0.000 % |
| `mm2` | 6 017 861 569 | **6 017 850 941** | −10 628 | −0.000 % |
| `z2` | 6 291 442 776 | 6 291 470 949 | +28 173 | +0.000 % |
| `readbuf` | 6 834 534 036 | 6 834 539 959 | +5 923 | +0.000 % |
| `duelito` | 7 380 421 165 | 7 380 476 374 | +55 209 | +0.001 % |
| `smb3` | 7 949 489 726 | **7 949 470 464** | −19 262 | −0.000 % |
| `mmc5` | 11 279 358 122 | 11 301 718 016 | **+22 359 894** | **+0.198 %** |
| **total** | **79 371 205 383** | 79 393 576 793 | +22 371 410 | +0.028 % |

One symbol, as in all nine of MF-6c's measurements: `Cartridge::default_cpu_read`,
**283 356 728 → 305 726 996, +22 370 268**. And that is **exactly 3 × 7 456 756**, the quantum
MF-6c measured — so the figure is the same quantum and the **same two variants now buy three
units of it instead of one**. MF-6c step 2 and step 3 recorded one unit for this pair against
`8496c503`; eight commits of boards later, the price has tripled. That is the whole case for this
lot: the wall is not stationary, it is moving away.

### Experiment B — `DecodeBoard`'s fifth variant: `mmc5` **+0.066 %**, one unit

| ROM | `44f77f72` | **experiment B** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `mmc5` | 11 279 358 122 | 11 286 848 022 | **+7 489 900** | **+0.066 %** |
| `m4` | 5 472 522 428 | 5 472 542 909 | +20 481 | +0.000 % |
| every other ROM | | | within **±53 100** | ≤ +0.000 % |
| **total** | **79 371 205 383** | 79 378 523 753 | +7 318 370 | +0.009 % |

`Cartridge::default_cpu_read`, **283 356 728 → 290 813 484, +7 456 756** — MF-6c step 9's figure
to the instruction, for the tenth time in this project.

### **The instruction, and MF-6c's diagnosis sharpened**

MF-6c named `leaq 240(%rdi), %rax` hoisted above "the inlined `Board::prg_ram_enabled` dispatch".
On this tree the dump says something more precise, and it matters for the fix.
`fn.py lin-mfct-0-44f77f72.s default_cpu_read` is **429** instructions and
`lin-mfct-1-gt111dec.s` **445**, and the two bodies differ in **one instruction that is not new
code**:

| | `44f77f72` (cheap) | experiment B (dear) |
| --- | --- | --- |
| the head, for $6000-$7FFF | `cmpl $24575` / `jle` / **`leaq 240(%rdi), %rax`** / jump table | *identical* |
| the tail, for everything else | `movzbl 1608(%rdi), %eax` / … / `jmpq *%rax` — **11 instructions** | `movzbl 1608(%rdi), %edx` / `leal -2(%rdx), %r8d` / **`leaq 240(%rdi), %rax`** / … — **12** |

So the hoist is **already in the head at the baseline** and costs nothing there: `movswl %si, %ecx`
followed by `cmpl $24575, %ecx; jle` is one *signed* comparison that sends both $0000-$5FFF and
$8000-$FFFF away in two instructions, because a `u16` above $7FFF sign-extends negative. The head
is therefore never entered by a read of $8000-$FFFF, which is nearly every read an MMC5 game makes.

**What costs is the second copy of the same `leaq`, in the `Board::try_read_prg` jump table's
prologue** — the tail every $8000-$FFFF read goes through. `&self.board` is wanted there by the
*cold* arms of that table, among them mapper 116's boxed `IrqBoard::Somari(Box<Somari>)`, whose
arm is `movq (%rax), %r10` followed by a nested jump table; LLVM materialises the pointer above
the table rather than inside the arm, and it then runs on every call. One instruction × 7 456 756
reads of `mmc5exram` over 300 frames is **7 456 756**, to the instruction.

`default_cpu_read` holds the `try_read_prg` table **twice** — once as the fall-through of the
$6000-$7FFF head (PRG-RAM disabled, or none) and once as the tail — and the dear layout is the one
in which the *tail* copy gets the hoist. That is why no shape of a board's own code ever moved the
figure (MF-6c step 2 measured seven, step 9 one more): the trigger is how much pressure the whole
429-instruction function puts on that one block.

### The figure to beat

> **One more variant in a shared enum costs `mmc5` one or more units of exactly 7 456 756
> instructions over 300 frames (+0.066 % each), paid as one hoisted `leaq` per CPU read in
> `Cartridge::default_cpu_read`'s copy of the `Board::try_read_prg` jump table. At `44f77f72`
> `SmallBoard`'s eleventh and twelfth variants cost three units (+0.198 %) and `DecodeBoard`'s
> fifth costs one (+0.066 %), against a tolerance of +0.02 %.**

The target of step 2 is that both of those come inside +0.02 % on every ROM.

[PROGRESS] MF-CART step 1/8 done: the limit reproduced at the new baseline and **worse than
MF-6c's** — `board-taito-x1` is `mmc5` **+22 370 268, three units of the same 7 456 756** and
+0.198 %, `board-111` is one unit and +0.066 %, both of them entirely
`Cartridge::default_cpu_read`; and the dump sharpens the diagnosis — the hoisted
`leaq 240(%rdi), %rax` that costs is the one in the **`Board::try_read_prg`** table's prologue on
the $8000-$FFFF tail, not the one in the `prg_ram_enabled` head, which a single signed compare
already skips.
