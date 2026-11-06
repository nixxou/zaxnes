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

## Step 2 — the structural fix: the $6000-$7FFF read answer settled on `Cartridge`

### What was built

`Cartridge::default_cpu_read` asked the `Board` enum **two** questions about $6000-$7FFF —
`Mapper::prg_ram_enabled` and `Mapper::prg_ram_offset` — on a function the MMC5 reaches on every
CPU read. Both are now settled wherever `rom_windows` and `nametable_pages` already are, which is
the shape the plan revision of MF-6c step 5 named as shape 2 and nothing had tried.

**One trait method was replaced**, and that is the whole of the design:

| | before | after |
| --- | --- | --- |
| the trait | `fn prg_ram_offset(&self, memory, address) -> usize` | `fn prg_ram_window(&self, memory) -> (usize, usize)` |
| the default | `usize::from(address - 0x6000) % memory.prg_ram().len()` | `(0, memory.prg_ram().len())` |
| the index | the method, per read | `(base + (address - 0x6000)) % wrap`, from two cached `usize` |

The four boards that override it are the four that bank their PRG-RAM, and **every one of them
already had exactly this shape**, which is why the method could become a window with no change of
arithmetic anywhere:

| board | before | after |
| --- | --- | --- |
| `Mmc1` (SOROM/SXROM) | `(bank * 8192 + (a − 0x6000)) % len` | `(bank * 8192, len)` |
| `Mmc5` | `(ram_bank * 8192 + (a − 0x6000)) % len` | `(ram_bank * 8192, len)` |
| `Fme7` | `(window_6000 & 0x3F) * 8192 + …) % len` | `((window_6000 & 0x3F) * 8192, len)` |
| `Namco163` | `(a − 0x6000) % (len − 128).max(1)` | `(0, (len − 128).max(1))` |

No shared enum forwarded `prg_ram_offset` and none forwards `prg_ram_window`, so `Board` keeps a
four-body dispatch — now reached **once per CPU write** instead of once per CPU read.

### The two fields, and why the enable is folded into the wrap

    prg_ram_read_base: usize,   // the byte offset in PRG-RAM of $6000
    prg_ram_read_wrap: usize,   // the length the offset wraps at; 0 means "not RAM for a read"

`refresh_prg_ram_read` is called from `refresh_rom_windows`, so the answer is settled at load, on
every CPU write to the cartridge, on a reset and on a loaded state — the same four moments
`rom_windows` is settled at, and **the only four at which any board can move it**: the eighteen
mapper numbers whose register can change it all move it from `write_register` or
`intercept_cpu_write`, and the one board whose state advances on a CPU cycle inside this question's
reach (the NES-EVENT's M2 counter) touches `counting`, `counter` and `irq_pending` and not
`Mmc1::prg_register`.

`prg_ram_enabled` and `!prg_ram.is_empty()` are folded into the wrap so that the read is **one
test and not three**, and the test is ordered *after* the range test rather than before it,
because the head of this function is where an MMC5 game's reads arrive: `movswl %si, %ecx` then
`cmpl $24575, %ecx; jle` is one *signed* comparison that sends both $0000-$5FFF and $8000-$FFFF
away in two instructions, a `u16` above $7FFF sign-extending negative.

`refresh_prg_ram_read` is **not** gated the way `nametable_pages_vary` gates
`refresh_nametable_pages`: eighteen numbers can move this answer against two that can move a
nametable page, which is too many for the gate to pay for itself.

### The write path is deliberately left alone

`Cartridge::cpu_write` goes on asking the dispatch, through `Cartridge::prg_ram_offset`, which is
now one `prg_ram_window` call and the arithmetic. It **cannot** share the read cache: the enable
folded into that cache is `Mapper::prg_ram_enabled` and what gates a write is
`Mapper::prg_ram_writable`, which is a different answer on four boards — and a Namcot 340 is a
case where a write may land while a read may not. A CPU write to the cartridge is the rarest bus
event there is (MF-6b step 5), so there is nothing to buy there and an implication to prove that
is not worth proving.

### **How the lot knows the cache is never stale, and it is not an argument**

`Cartridge::read_rom_window` has carried
`debug_assert_eq!(Some(value), self.board.try_read_prg(&self.memory, address))` since lot PR-1:
the cache checks itself against the dispatch it replaces, on every read, in every debug build.
`default_cpu_read` now carries the same idiom in three parts — the offset against
`self.prg_ram_offset(address)`, the taken branch against `prg_ram_enabled() && !is_empty()`, and
**the not-taken branch against its negation**, which is the half a one-sided assertion would miss.
`refresh_prg_ram_read` carries the fourth, the one thing the trait cannot state: a readable window
is a window of non-zero length, or the sentinel would read as "not RAM".

So the whole test suite is the checker, in all three configurations:

| configuration | `zax-nes`, debug (the asserts live) | release, with `zax-harness` |
| --- | ---: | ---: |
| `default` | **854 / 0 / 7** | **917 / 0 / 8** |
| `--features parked-boards` | **862 / 0 / 7** | **925 / 0 / 8** |
| `--no-default-features` | **783 / 0 / 7** | — |

Every figure is MF-6c step 10's and step 11's to the test, and the debug columns are **854 + 862 +
783 = 2 499 test bodies that drove the assertions and none that tripped**. No test was added,
changed or removed by this step: it is a refactor whose correctness claim is "the same answers",
and the way to show that is the existing suite with a checker wired into the path.

`cargo fmt --check` clean; clippy `-D warnings` clean on `zax-nes` in `default`,
`--features parked-boards` and `--no-default-features`, and on `zax-harness`, with **no new
`allow` of any kind**. The savestate is untouched — no board gained, lost or moved a field, and
`req_core_090_the_mapper_state_survives_a_save_and_restore` is in all three columns above.

The measurement is step 3; this step is the code.

[PROGRESS] MF-CART step 2/8 done: `Mapper::prg_ram_offset` becomes `Mapper::prg_ram_window`, asked
once per CPU write instead of once per CPU read, and its answer with the enable folded in is two
cached `usize` on `Cartridge` beside `rom_windows` — so `Cartridge::default_cpu_read` holds **no
`Board` dispatch for $6000-$7FFF at all**. 854 / 862 / 783 debug and 917 / 925 release, every
figure MF-6c's to the test, with the cache checking itself against the dispatch it replaces on
every read of every debug build.

## Step 3 — the fix measured: **−11.3 M on the set, and the target not reached**

### Instrument 2, the default build (`mfct-3-ramwindo`, then `mfct-3-pinoffst`)

Two shapes were measured for the one thing step 2 moved onto the write path, and both are in the
table below. The figures named in the text are the second, which is the shipped one.

| ROM | `44f77f72` | **step 3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 287 870 | **3 308 287 621** | −249 | −0.000 % |
| `oamstr` | 3 318 485 575 | **3 318 473 334** | −12 241 | −0.000 % |
| `m1` | 4 885 953 734 | **4 881 020 468** | **−4 933 266** | **−0.101 %** |
| `m4` | 5 472 522 428 | **5 467 781 912** | **−4 740 516** | **−0.087 %** |
| `m0` | 5 520 499 653 | **5 517 124 138** | **−3 375 515** | **−0.061 %** |
| `metroid` | 5 115 985 915 | **5 115 565 862** | −420 053 | −0.008 % |
| `smb1` | 5 996 362 814 | 5 996 365 756 | +2 942 | +0.000 % |
| `mm2` | 6 017 861 569 | 6 018 045 058 | **+183 489** | **+0.003 %** |
| `z2` | 6 291 442 776 | 6 291 535 363 | +92 587 | +0.001 % |
| `readbuf` | 6 834 534 036 | **6 834 511 350** | −22 686 | −0.000 % |
| `duelito` | 7 380 421 165 | 7 380 424 201 | +3 036 | +0.000 % |
| `smb3` | 7 949 489 726 | 7 949 548 566 | +58 840 | +0.001 % |
| `mmc5` | 11 279 358 122 | **11 279 349 260** | −8 862 | −0.000 % |
| **total** | **79 371 205 383** | **79 358 032 889** | **−13 172 494** | **−0.017 %** |

**The cache is worth −13.2 M on the set before it lifts any limit at all**, and the three
holy-mapperel ROMs say why: the dispatch it removed was **not free**.
`Board::prg_ram_offset` had a symbol of its own with **3 736 274 instructions on `m0`,
5 506 200 on `m1`, 3 736 350 on `m4` and 644 280 on `metroid`** — an out-of-line call per
$6000-$7FFF read, on four ROMs whose boards bank nothing. All four go to **0**.

**The two shapes, and the first is refuted.** What step 2 moved onto the write path is one
`Board::prg_ram_window` dispatch and one `prg_ram_enabled` per CPU write, and the boundary LLVM
had drawn moved with it: `Cartridge::refresh_rom_windows` **0 → 337 060** on `mm2` as it acquired
a symbol, with `Board::rom_windows` **177 400 → 0** as it was inlined into it.

| | shape | set total | worst ROM |
| --- | --- | ---: | --- |
| B1 | `#[inline(always)]` on `refresh_rom_windows`, to put the old boundary back | −10 963 934 | `mm2` **+223 498** — and six ROMs above, `readbuf`, `smb1` and `vblnmi` newly so |
| **B2** | `#[inline(always)]` on `Cartridge::prg_ram_offset`, the **write** path's own helper | **−13 172 494** | `mm2` +183 489, and `vblnmi` and `readbuf` back below |

B2 is the shipped one and the reason is in the symbol table: `cpu_write` reached PRG-RAM through
**two** calls (`Cartridge::prg_ram_offset`, 1 311 240 on `m0`, then `Board::prg_ram_window`) where
it used to make one, and pinning the outer one leaves the dispatch and drops the wrapper. That is
this project's own idiom for the fifth time — a chain the cost model keeps re-deciding is pinned
rather than shortened (PERFORMANCE-HISTORY §5.10).

### **The experiment re-run, and the target is not reached**

`board-taito-x1` put back into `default` on **this** tree (`mfct-3-x1fixed1`):

| | `mmc5` | `Cartridge::default_cpu_read` | |
| --- | ---: | ---: | --- |
| `44f77f72`, the board parked | 11 279 358 122 | 283 356 728 | |
| `44f77f72` + `board-taito-x1` (step 1) | 11 301 718 016 | 305 726 996 | +22 370 268 = **3 units** |
| **step 3** + `board-taito-x1` | 11 309 204 842 | **313 183 752** | **+29 827 024 = 4 units**, +0.265 % |

**Four units of exactly 7 456 756, and worse than step 1's three.** So the $6000-$7FFF cache does
not lift the limit, and step 1's dump said why it would not: the hoisted `leaq 240(%rdi), %rax`
that costs is the one in the **`Board::try_read_prg`** table's prologue, and that table is still
in this function.

### What the dump says, and it is the step's real finding

`fn.py` on the three trees, and one number that settles the diagnosis:

| | `default_cpu_read`, static | `default_cpu_read` on `mmc5`, dynamic |
| --- | ---: | ---: |
| `44f77f72` | 429 | **283 356 728** |
| step 3 (the cache) | 396 | **283 356 728**, identical to the instruction |
| step 3 + `board-taito-x1` | 340 | 313 183 752 |

**283 356 728 ÷ 7 456 756 is exactly 38.0**, so that function is entered **7 456 756 times** —
once per CPU read of `mmc5exram` — and the quantum MF-6c measured nine times **is one instruction
per read**, neither more nor less. Step 3 took 33 instructions out of the function statically and
changed its dynamic cost on the MMC5 by **zero**: the MMC5's reads are of $8000-$FFFF and never
enter the $6000-$7FFF half at all. The cache's −13.2 M is all on the other boards.

And the dear layout differs from the cheap one in **three instructions of the entry**, which
`grep leaq` finds in one look — `movl %esi, %ecx`, `movq %rdi, %r8`, and
**`leaq 240(%r8), %rdi`**. The fixed default build contains **no `leaq 240(…)` at all**. So LLVM
is setting up a call ABI: `%rdi = &self.board` for the cold arms of the `try_read_prg` table that
are out-of-line calls, with `self` and the address shuffled out of the way first.

> **The target of step 2 is not reached, and the reason is now exact: what has to leave
> `Cartridge::default_cpu_read` is not the $6000-$7FFF dispatch but the `Board::try_read_prg`
> jump table's *copy* in that function.** Step 4 is where that happens.

### The suites

The tree of this step is step 2's plus one `#[inline(always)]`, which cannot change behaviour;
the suites are therefore step 2's, and step 4 re-runs all five configurations on the tree that
carries both attributes: **854 / 862 / 783** debug and **917 / 925** release, `cargo fmt --check`
clean and clippy `-D warnings` clean on `zax-nes` in three configurations and on `zax-harness`.

[PROGRESS] MF-CART step 3/8 done: the cache is **−13 172 494 (−0.017 %)** on PR-1's thirteen with
`m1` −0.101 %, because the `Board::prg_ram_offset` dispatch it removed had a symbol worth 3.7 M to
5.5 M on four ROMs — **and the target is not reached**: `board-taito-x1` on this tree is **four**
units of the same 7 456 756, +0.265 %. The dump settles the diagnosis — `default_cpu_read` is
entered exactly 7 456 756 times on `mmc5`, the quantum is **one instruction per read**, and the
three that appear are `movq %rdi, %r8` and `leaq 240(%r8), %rdi` setting up `&self.board` for the
**`try_read_prg`** table's out-of-line arms.

## Step 4 — the shape that lifts the limit: **`default_cpu_read` inlined, and one dispatch per write**

Four shapes were measured, two of them refuted, and the one that works is one MF-6c had already
measured and had to reject.

### Shape 4a — `board` at offset 0 of `Cartridge`, `#[repr(C)]`: **refuted, and expensively**

Step 3's dump said the cost is LLVM materialising `&self.board` into `%rdi`. With `board` at
offset 0 that address *is* `%rdi` and there would be nothing to materialise, and `repr(Rust)`
sorts fields by alignment and had put `memory` first, so `#[repr(C)]` was the only way to ask.

**It costs every ROM of the set between +0.33 % and +1.12 %, and not in this function at all:**
`NesBus::read_cycle` **+35.3 M to +38.7 M on all thirteen** — about **4.3 instructions per CPU
read** — set total **+464 793 722 (+0.586 %)**, with `default_cpu_read` and `prg_ram_offset`
moving exactly as they did without it. `NesBus` holds the cartridge as an `Option<Cartridge>`, and
what the attribute moved is that enum's layout, not this struct's hot fields. Reverted, and worth
recording so that nobody tries it twice: **`#[repr(C)]` on `Cartridge` is not available as a
layout lever.**

### Shape 4i — `#[inline(always)]` on `Cartridge::default_cpu_read`: **this is the fix**

MF-6c step 2 measured this attribute as its shape c and rejected it: `mmc5` came out −0.065 %
but `m0` +0.022 % and `m1` +0.024 %, "`Board::prg_ram_enabled` out of line" — that dispatch
having left this function for one of its own as the body moved into `try_cpu_read`. **Step 2 of
this lot took that dispatch out of the function altogether**, so the shape's one drawback cannot
happen, and the two halves of the fix only work together.

On `mmc5`, with `board-taito-x1` **on**, the three symbols that move are:

| symbol | step 3 | **step 4** |
| --- | ---: | ---: |
| `Cartridge::default_cpu_read` | 283 356 728 | **0** |
| `Mmc5::try_read_prg` | 0 | **141 678 364** |
| `Cartridge::try_cpu_read` | 300 643 690 | **429 758 682** |

a net **−12.6 M**. The function stops existing and the **MMC5's own `try_read_prg` arm goes out
of line** — the big arm, on the one board that maps PRG-RAM into $8000-$DFFF — so the per-read
path is `try_cpu_read`'s body and a `callq` into one board's code.

> **And that is what lifts the limit.** The quantum was one instruction per read paid in this
> function's *copy* of the inlined `Board::try_read_prg` jump table. There is no such copy any
> more, and the proof is that **`Mmc5::try_read_prg` measures 141 678 364 and `try_cpu_read`
> 429 758 682 in all three configurations** — `default`, `+ board-taito-x1`, and
> `+ board-taito-x1 + board-111` — **identical to the instruction**, with `Board` at
> thirty-six, thirty-eight and thirty-nine variants. A board added to a shared enum no longer
> touches the MMC5's read path at all.

| configuration | `Board` variants | `mmc5` | set total |
| --- | ---: | ---: | ---: |
| `default` | 36 | **11 266 796 862** (−0.111 %) | −25 432 993 (−0.032 %) |
| `+ board-taito-x1` | 38 | **11 266 785 317** (−0.111 %) | −25 180 572 (−0.032 %) |
| `+ board-taito-x1 + board-111` | 39 | **11 266 985 480** (−0.110 %) | −26 066 297 (−0.033 %) |

The ±200 000 between the three is the ±57 k-per-ROM whole-process floor of §5.11 and the loader's
own work for two more boards; the symbol table is identical.

### Shape 4j — `#[inline(never)]` on `refresh_prg_ram_read`: **refuted**

An attempt at the write-path cost step 3 left: shrink `refresh_rom_windows` so that LLVM inlines
it into `cpu_write` again. Set total **−24 693 981** against 4i's −25 432 993, nine ROMs above the
baseline against four, and `mm2` **+233 731** against +177 444. Reverted.

### Shape 4k — **one dispatch per CPU write, and it halves what is left**

What stayed above the baseline after 4i was `mm2`, and the level-2 instrument priced it at
**`mm2/core` +612 352, +0.005 %**: Mega Man 2 is an MMC1, whose five-write shift register makes
`refresh_rom_windows` run five times per register, and that function was asking the `Board` enum
**three** questions — `rom_windows`, `prg_ram_window` and `prg_ram_enabled`, three jump tables.

`Mapper::cpu_read_windows` makes them **one**. It returns a `CpuReadWindows` — the four ROM
windows, the $6000-$7FFF base, and its wrap with the enable folded in — and **no board overrides
it**: the default body calls the three methods on `Self`, which is monomorphised per board, so
inside one arm of the one dispatch there is no dispatch left to pay.

| | set total | worst ROM of thirteen | `mm2/core` |
| --- | ---: | --- | ---: |
| 4i, three dispatches | −25 340 126 | `mm2` +171 056 (+0.003 %) | +612 352 (+0.005 %) |
| **4k, one dispatch** | **−25 551 352** | `mm2` **+62 226 (+0.001 %)** | **+287 536 (+0.002 %)** |

### The shipped build of this step, on both instruments

`mfct-4-onedisp1`, against `mfct-0-44f77f72`:

| ROM | `44f77f72` | **step 4** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 287 870 | 3 308 307 092 | +19 222 | +0.001 % |
| `oamstr` | 3 318 485 575 | **3 318 483 000** | −2 575 | −0.000 % |
| `m1` | 4 885 953 734 | **4 881 230 998** | **−4 722 736** | **−0.097 %** |
| `m4` | 5 472 522 428 | **5 467 984 307** | **−4 538 121** | **−0.083 %** |
| `m0` | 5 520 499 653 | **5 517 393 347** | **−3 106 306** | **−0.056 %** |
| `metroid` | 5 115 985 915 | **5 115 540 611** | −445 304 | −0.009 % |
| `smb1` | 5 996 362 814 | **5 996 331 285** | −31 529 | −0.001 % |
| `mm2` | 6 017 861 569 | 6 017 923 795 | **+62 226** | **+0.001 %** |
| `z2` | 6 291 442 776 | 6 291 477 214 | +34 438 | +0.001 % |
| `readbuf` | 6 834 534 036 | **6 834 492 375** | −41 661 | −0.001 % |
| `duelito` | 7 380 421 165 | **7 380 413 452** | −7 713 | −0.000 % |
| `smb3` | 7 949 489 726 | **7 949 308 385** | −181 341 | −0.002 % |
| `mmc5` | 11 279 358 122 | **11 266 768 170** | **−12 589 952** | **−0.112 %** |
| **total** | **79 371 205 383** | **79 345 654 031** | **−25 551 352** | **−0.032 %** |

**Ten of thirteen below the baseline**, the three above all at +0.001 %, and `mmc5` — the ROM
eleven measurements of MF-6c and MF-CART had been charged +7 456 756 a time for — **−0.112 %**.

Level 2, `l2-diff.py --bar`:

| game/stage | `44f77f72` | **step 4** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 146 858 287** | −436 855 | −0.004 % |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** | **equal** |
| `zelda2/core` | 12 775 651 048 | **12 774 921 154** | −729 894 | −0.006 % |
| `mm2/core` | 12 036 822 861 | 12 037 110 397 | **+287 536** | **+0.002 %** |
| `smb3/core` | 15 315 537 499 | **15 314 887 392** | −650 107 | −0.004 % |
| `metroid/total` | 13 735 578 935 | **13 735 098 459** | −480 476 | −0.003 % |
| `smb1/total` | 13 586 757 575 | **13 586 649 040** | −108 535 | −0.001 % |
| `zelda2/total` | 15 360 318 280 | **15 357 711 018** | −2 607 262 | −0.017 % |
| `mm2/total` | 18 976 430 445 | 18 976 510 095 | +79 650 | +0.000 % |
| `smb3/total` | 24 188 986 419 | **24 186 496 714** | −2 489 705 | −0.010 % |

**Eight of the ten figures are below the baseline, three of the five `core` figures by 0.004 % to
0.006 % and the fourth exactly equal.** The one that is above is **`mm2/core` +0.002 %**, a tenth
of the owner's +0.02 %, and it is reported rather than argued away: it is the per-CPU-write cost
of settling the answer the read path stopped asking for, on the one game of the five whose board
writes a register five times to set it. Four shapes were measured against it — B1, B2, 4j and 4k
— and 4k is the best of them; a fifth, a load-time latch on whether the window can move at all,
was considered and not built, because the MMC1's enable moves too (its $8000 bit 4) and the latch
would exempt neither Mega Man 2 nor Zelda II.

### Instrument 3: the post-LTO dump

`pins63.py lin-mfct-4-inlrdchk.s`, which is shape 4i's tree; 4k changes no symbol of this list.

| symbol | `44f77f72` | **step 4** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | **3 626** |
| `Board::ppu_peek` | 291 | **291** |
| `NesBus::write_internal` | 3 861 | 3 879 |
| `Ppu::run_dots` | 1 919 | **1 919** |
| `NesBus::catch_up_ppu` | 188 | **188** |
| `Ppu2C02::read_pattern_planes` | **107** | **107** |
| `Ppu2C02::read_nametable_inner` | 75 | **75** |
| `Cartridge::default_cpu_read` | 429 | **gone — it is `try_cpu_read`'s body** |

`shape.py`'s five others — `resolve` 1 948, `branch` 457, `read_cycle` 181, `tick` 82,
`read_internal` 175 — are identical to the instruction. **`read_pattern_planes` is 107 for the
ninth lot running** and `zone_offset` does not exist as a symbol, which is the owner's two named
figures holding. `write_internal` +18 is the write path's new arithmetic, and the dynamic column
prices it: `m4` −0.083 %, `smb3` −0.002 %.

**One note for a successor, because it cost this step twenty minutes.** The binary of shape 4k is
past the threshold §5.12 records: **valgrind 3.22 stopped reading its Rust symbols**, so
`callgrind-l2.sh`'s awk matched nothing and `stages.txt` came out empty. The counts are
unaffected. `rust/target-mf/l2fix.sh` (scratch) repairs a level-2 directory with
`resolvesyms.py` — which that lot wrote for a `callgrind-set.sh` directory, so the `.inc` files
have to be staged under `.ann` names first, and the stage patterns have to accept the **mangled**
v0 spelling the tool writes back. The bias derived itself, `0x108000`, with 986 addresses landing
exactly on a symbol start.

### The suites, every configuration

| configuration | `zax-nes`, debug | release, with `zax-harness` |
| --- | ---: | ---: |
| `default` | **854 / 0 / 7** | **917 / 0 / 8** |
| `--features parked-boards` | **862 / 0 / 7** | **925 / 0 / 8** |
| `--no-default-features` | **783 / 0 / 7** | — |

every figure MF-6c's to the test, with the debug columns running step 2's four assertions on
every read. `cargo fmt --check` clean; clippy `-D warnings` clean on `zax-nes` in all three
configurations and on `zax-harness`, **no new `allow` beyond the two `clippy::inline_always` the
two measured attributes carry**, each with its figure on the line.

[PROGRESS] MF-CART step 4/8 done: **the limit is lifted.** `#[inline(always)]` on
`Cartridge::default_cpu_read` — the shape MF-6c step 2 measured and had to reject, whose one
drawback step 2 of this lot removed — takes that function out of existence and the MMC5's own
`try_read_prg` arm out of line, and **`Mmc5::try_read_prg` and `try_cpu_read` then measure
141 678 364 and 429 758 682 whether `Board` has 36, 38 or 39 variants, identical to the
instruction.** `Mapper::cpu_read_windows` makes the write path one dispatch instead of three.
`mmc5` **−0.112 %**, set **−25 551 352 (−0.032 %)**, ten of thirteen below, four level-2 cores
below or equal and `mm2/core` +0.002 % the only figure above. `#[repr(C)]` on `Cartridge` refuted
at +0.586 %, and `#[inline(never)]` on the refresh refuted.

## Step 5 — **both boards un-parked**, and `parked-boards` is empty again

### The two measurements, each feature alone on the fixed tree

| | label | `mmc5` | worst ROM of thirteen | set total |
| --- | --- | ---: | --- | ---: |
| `board-taito-x1` (80, 207, 82, 552) | `mfct-5-x1onedis` | **11 266 781 495, −0.112 %** | `mm2` **+76 961, +0.001 %** | **−25 190 380, −0.032 %** |
| `board-111` (the GTROM) | `mfct-5-gt1onedi` | **11 266 793 832, −0.111 %** | `mm2` +94 724 / `oamstr` +76 127, **+0.002 %** | **−24 997 757, −0.031 %** |

**Both are inside the owner's +0.02 % by a factor of ten or twenty on every ROM, and both leave
`mmc5` a tenth of a per cent *below* the baseline.** What MF-6c measured for the same two features
— +7 456 756 and +7 423 140, and by MF-CART step 1 **+22 370 268** for the first — is gone, because
the function those figures lived in is gone. So both go into `default`, `parked-boards` is emptied,
and this project has **no parked board**.

    parked-boards = []

The switch stays wired, for the reason it was wired through MF-6b and MF-6a3 when it was also
empty: a later lot that does park a board adds it there and nowhere else, and
`--features parked-boards` is then still the one flag that reaches every left-out board at once.
With the list empty it is a build identical to `default`, and the suites run it anyway so that it
cannot rot.

### The shipped build: `default` with every board this project has

`mfct-5-unparked`, against `mfct-0-44f77f72`:

| ROM | board | `44f77f72` | **step 5** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 287 870 | 3 308 319 744 | +31 874 | +0.001 % |
| `oamstr` | NROM | 3 318 485 575 | **3 318 444 925** | −40 650 | −0.001 % |
| `m1` | MMC1 | 4 885 953 734 | **4 881 249 893** | **−4 703 841** | **−0.096 %** |
| `m4` | MMC3 | 5 472 522 428 | **5 468 239 595** | **−4 282 833** | **−0.078 %** |
| `m0` | NROM | 5 520 499 653 | **5 517 368 200** | **−3 131 453** | **−0.057 %** |
| `metroid` | MMC1 | 5 115 985 915 | **5 115 573 500** | −412 415 | −0.008 % |
| `smb1` | NROM | 5 996 362 814 | **5 996 328 257** | −34 557 | −0.001 % |
| `mm2` | MMC1 | 6 017 861 569 | 6 017 927 746 | **+66 177** | **+0.001 %** |
| `z2` | MMC1 | 6 291 442 776 | 6 291 510 615 | +67 839 | +0.001 % |
| `readbuf` | CNROM | 6 834 534 036 | **6 834 510 710** | −23 326 | −0.000 % |
| `duelito` | NROM | 7 380 421 165 | **7 380 410 905** | −10 260 | −0.000 % |
| `smb3` | MMC3 | 7 949 489 726 | **7 949 330 220** | −159 506 | −0.002 % |
| `mmc5` | MMC5 | 11 279 358 122 | **11 266 798 540** | **−12 559 582** | **−0.111 %** |
| **total** | | **79 371 205 383** | **79 346 012 850** | **−25 192 533** | **−0.032 %** |

**Ten of thirteen below the baseline with five mapper numbers added to the default build**, the
three above all at +0.001 %, and the set total is **−25 192 533** — a lot that *adds* boards and
ends 0.032 % faster than the tree it started from.

Level 2, `l2-diff.py --bar`:

| game/stage | `44f77f72` | **step 5** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 295 142 | **11 146 861 951** | −433 191 | −0.004 % |
| `smb1/core` | 11 515 128 803 | **11 515 128 803** | **+0** | **equal** |
| `zelda2/core` | 12 775 651 048 | **12 774 928 308** | −722 740 | −0.006 % |
| `mm2/core` | 12 036 822 861 | 12 037 123 709 | **+300 848** | **+0.002 %** |
| `smb3/core` | 15 315 537 499 | **15 314 945 307** | −592 192 | −0.004 % |
| `metroid/total` | 13 735 578 935 | **13 735 129 401** | −449 534 | −0.003 % |
| `smb1/total` | 13 586 757 575 | 13 588 084 193 | +1 326 618 | +0.010 % |
| `zelda2/total` | 15 360 318 280 | **15 358 364 558** | −1 953 722 | −0.013 % |
| `mm2/total` | 18 976 430 445 | 18 976 605 166 | +174 721 | +0.001 % |
| `smb3/total` | 24 188 986 419 | 24 190 119 762 | +1 133 343 | +0.005 % |

**Three `core` figures below by 0.004 % to 0.006 %, the fourth exactly equal, and the fifth
`mm2/core` at +0.002 %** — a tenth of the tolerance, and step 4 has the four shapes measured
against it. The two `total` rises are rule 2's layout-noise case **checked and not assumed**;
`symct.sh` at a 400 000 threshold names every symbol that moved on any of the five games:

* **`smb1`**: one symbol, `luaH_getshortstr` **+1 164 830**. Lua's `ltable.c`, nothing else at
  all, against a total delta of +1 326 618 and a `core` delta of **+0**.
* **`smb3`**: `luaH_getshortstr` +1 179 523 and `__memcpy_avx_unaligned_erms` +400 678, and this
  project's own five symbols sum to **−1 228 506** — `default_cpu_read` −1 705 442,
  `Board::rom_windows` −1 100 920, `Board::prg_ram_offset` −1 098 295 against
  `Board::cpu_read_windows` +1 623 857 and `try_cpu_read` +1 052 294. The rise is Lua and
  `memcpy`; the emulator is below, and `smb3/core` is −592 192.
* **`metroid`** and **`zelda2`** show the fix and nothing else: `Board::prg_ram_offset` −783 216
  and −849 968 and `default_cpu_read` −557 270 and −920 892 against `try_cpu_read` +425 285 and
  +702 786.
* **`mm2`** shows the one cost this lot has: `Board::cpu_read_windows` **+772 270** against
  `Board::rom_windows` **−532 600**, a net +239 670 which is `mm2/core`'s +300 848. That is the
  write path settling what the read path stopped asking for, named to the symbol.

### Instrument 3: the post-LTO dump

`pins63.py lin-mfct-0-44f77f72.s lin-mfct-5-unparked.s`:

| symbol | `44f77f72` | **step 5** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 626 | **1 / 2 / 3 626** |
| `Board::ppu_peek` | 1 / 4 / 291 | **1 / 4 / 291** |
| `NesBus::write_internal` | 1 / 10 / 3 861 | 1 / 10 / 3 879 |
| `Ppu::run_dots` | 1 / 5 / 1 919 | **1 / 5 / 1 919** |
| `NesBus::catch_up_ppu` | 1 / 12 / 188 | **1 / 12 / 188** |

and `shape.py`'s five others — `resolve` 2 / 6 / **1 948**, `branch` 1 / 1 / **457**,
`read_cycle` 1 / 65 / **181**, `tick` 1 / 95 / **82**, `read_internal` 1 / 17 / **175** —
identical to the instruction, def and call sites included. With `fn.py`:

| symbol | `44f77f72` | **step 5** |
| --- | ---: | ---: |
| `Ppu2C02::read_pattern_planes` | **107** | **107** |
| `Ppu2C02::read_nametable_inner` | **75** | **75** |
| `Cartridge::default_cpu_read` | 429 | **no definition** |

**`read_pattern_planes` is 107 for the ninth lot running**, `zone_offset` does not exist as a
symbol — the inlining holding, which is the widescreen hack's own figure — and the three
claimants of `Cpu6502::step` are 0 / 0 / 0 with `step` at 3 626. The only pinned symbol that moved
is `write_internal` **+18**, the write path's new arithmetic, and the dynamic column prices it at
`m4` −0.078 % and `smb3` −0.002 %.

### The suites, and what un-parking did to them

| configuration | `zax-nes`, debug | release, with `zax-harness` |
| --- | ---: | ---: |
| `default` | **862 / 0 / 7** | **925 / 0 / 8** |
| `--features parked-boards` | **862 / 0 / 7** | **925 / 0 / 8** |
| `--no-default-features` | **783 / 0 / 7** | — |

**`default` is now what `--features parked-boards` used to be** — MF-6c's 854 plus the eight tests
that were behind the two parked features, five of `board-taito-x1`'s and three of `board-111`'s —
and the two columns are equal because the umbrella is empty. `cargo fmt --check` clean; clippy
`-D warnings` clean on `zax-nes` in five configurations (`default`, `--features parked-boards`,
`--no-default-features`, and each un-parked feature alone on top of `--no-default-features`) and
on `zax-harness`, **no new `allow`**.

`docs/review/BOARD-FEATURES.md` is rewritten where it has to be: its header now says no board is
parked and why, the two rows carry their new figures beside the old ones, the umbrella row says
the switch is wired and empty, MF-6c's two diagnosis sections are kept and marked **superseded**
with a pointer to §5.15, and the how-to figures are the un-parked tree's (783 against **862**).

[PROGRESS] MF-CART step 5/8 done: **both boards un-parked and `parked-boards` is empty.**
`board-taito-x1` re-measured at `mmc5` **−0.112 %** with a worst ROM of +0.001 % and `board-111`
at **−0.111 %** with +0.002 %, against the +7 456 756 and +7 423 140 MF-6c parked them on. The
shipped `default` is **−25 192 533 (−0.032 %)** on PR-1's thirteen with ten of them below, three
level-2 cores below and one equal, `read_pattern_planes` **107** and every other pinned symbol
identical to the instruction — and five mapper numbers more than the tree it started from.

## Step 6 — the new limit, **measured with synthetic boards** and written down

### How it was measured

A limit is only worth writing down if it is a number, so this step added boards rather than
reasoning about them. `ProbeBoard<const N: u8>` is a synthetic register-decode board — a
`BankWindows`, a register, `write_register`, `prg_ram_enabled`, `reset` and a savestate block —
and each probe variant is a **distinct monomorphisation**, so N variants are N distinct bodies in
every dispatch they reach, which is the expensive case and the shape an MF-UNL board will have.

Two things make the probes honest:

* they are **constructed**, from `registry::create_board` under mapper numbers 10 000.., so no arm
  and no jump-table slot is dead code LLVM could prune. Those numbers are not in
  `SUPPORTED_MAPPERS`, so `Cartridge::create` refuses them and no cartridge is ever one;
* they are **scratch**. `rust/target-mf/probe.py` wrote them into the worktree, the tree was packed
  with `make-tree.sh --worktree`, and `git checkout --` put it back; nothing of them is committed,
  and the tree at the end of this step is step 5's byte for byte.

### What the two axes cost, against the shipped tree (`mfct-5-unparked`)

| probe | label | `mmc5` | worst ROM of the thirteen | set total |
| --- | --- | ---: | --- | ---: |
| **+12 variants in a shared enum** (`DecodeBoard` 5 → 17) | `mfct-6-dec12prb` | **+3 695** | `m4` **+136 589, +0.002 %** | +301 876 (+0.000 %) |
| **+12 new `Board` discriminants** (36 → 48) | `mfct-6-brd12prb` | **+2 391 493, +0.021 %** | `mmc5`; **twelve of thirteen are 0.015 % to 0.279 % below** | **−66 508 473 (−0.084 %)** |
| **+4 new `Board` discriminants** (36 → 40) | `mfct-6-brd4prbe` | **+20 396 376, +0.181 %** | `mmc5`, then `m4` +0.089 % | −16 609 321 (−0.021 %) |

> **The limit for a board that shares a variant is gone.** A shared enum takes **at least twelve
> more variants** — on top of `DecodeBoard`'s five and `SmallBoard`'s twelve — at `mmc5` **+3 695**
> and a worst ROM of **+0.002 %**, a tenth of the owner's tolerance. Where MF-6c had to measure
> `SmallBoard` at ten, `IrqBoard` at ten and `DecodeBoard` at four, there is now no number a lot
> will reach: the twelve MF-UNL lots and MF-VS can plan on **one shared enum variant per board**.
>
> **A `Board` discriminant of its own is still a lottery, and it is the only quantum left.** Both
> probes move `NesBus::push_expansion_audio` by **+8 934 160 on `mmc5`** — *to the instruction*
> the figure MF-6a3 step 1 measured for **two** discriminants and MF-6b step 1 recorded as
> **−8 934 160** for the thirty-fifth variant: one taken `jmp` per expansion-audio sample, 8.9
> million of them, as the jump table `push_expansion_audio` inlines whole re-lays out and the
> MMC5's arm loses the fall-through. What differs between four and twelve is everything *else*:
> `Ppu2C02::run_dots` is **+11 460 164 on `mmc5` at four** and **−4.6 M to −17.7 M on every other
> ROM at twelve**, which is `Board::chr_view`'s table doing the same thing with the opposite sign,
> and at four discriminants `Board::ppu_read` +3 082 184 and `ppu_peek` +2 288 640 take `m4` to
> +0.089 % as well.

So MF-6a3 step 1's rule survives this lot **unamended and now measured at four points**, and it is
the one a successor has to respect: a new discriminant is free in every dispatch except the one a
hot function inlines whole, and whether LLVM hands the MMC5 the fall-through is its choice.
**Measure it; do not predict it** (§5.14).

### Where it is written down

* **`rust/docs/PERFORMANCE-HISTORY.md` §5.15**, new, between §5.9 and §5.14 as the file orders
  its lots: the quantum's arithmetic (283 356 728 ÷ 7 456 756 = 38.0, so one instruction per
  read), the two halves of the fix and why neither works alone, the table above, the
  `push_expansion_audio` finding, the two refuted layout levers (`#[repr(C)]` at +0.586 % and
  `#[inline(never)]` on the refresh), the one cost the fix pays on the write path, and §5.12's
  valgrind failure with the two wrinkles `resolvesyms.py` has on a level-2 directory.
* **Three older rules now point at it**, so that a reader of the rule finds the measurement that
  changed it: §5.14's `push_expansion_audio` rule (narrowed, not amended), §5.11's "a `Board`
  variant is free", and §5.13's `Board::prg_ram_enabled` finding, whose dispatch *and* whose
  function are both gone.
* **`docs/review/MF-6-inventory.md` §4.9 item 4**, new: item 3 is the limit this lot removed, and
  it said in as many words that "the structural fix … is R0's own sub-lot after MF-6c". That
  sub-lot ran, so item 4 says what replaces the advice — add the board to a shared enum, and
  measure a `Board` discriminant if you want one.
* **`docs/review/BOARD-FEATURES.md`**, in step 5.

### Nothing of this step is in the tree

No code, no test and no feature changed: `git diff` against step 5 is `PERFORMANCE-HISTORY.md` and
`MF-6-inventory.md` and nothing else, so the suites and every measurement of step 5 stand
unaltered. The probe trees were deleted with their tars as soon as their figures were recorded.

[PROGRESS] MF-CART step 6/8 done: the new limit **measured, not assumed** — twelve more variants
in a shared enum cost `mmc5` **+3 695** and a worst ROM of **+0.002 %**, so a board that shares a
variant has no limit left to plan against; a new `Board` discriminant is still the
`push_expansion_audio` lottery, **+8 934 160 to the instruction** in both probes, at `mmc5`
+0.181 % for four and +0.021 % for twelve. Written into `PERFORMANCE-HISTORY.md` §5.15 with
pointers from the three older rules it changes, and into §4.9 of the inventory as item 4.

## Step 7 — the check

### golden-core, in a form this project has not used before

The brief asks for golden-core unchanged, and MF-6c step 11 recorded that the test as it stands is
**nearly vacuous**: it reads its 291 ROMs from the gitignored `rust/test-roms/`, which in this
worktree holds **one**, and a ROM it cannot find counts as missing and not as a failure — so it
passes having examined 1 of 291 rows. That lot linked the real collection in with 64 junctions,
got **240 of 291 with 51 mismatches**, and showed the 51 were identical at its own baseline.

This step did that **and** something stronger, which cost nothing because the binaries already
existed. `zax-harness golden-core` is the same digests as a subcommand, and this lot has the
baseline's Linux binary and the tip's side by side in its scratch root:

    sh rust/target-mf/gcpair.sh mfct-0-44f77f72 mfct-5-unparked      # 1200 frames each

| | ROMs run | rows | the two files |
| --- | ---: | ---: | --- |
| `mfct-0-44f77f72` (the baseline) | **289** | 290 | |
| `mfct-5-unparked` (the tip) | **289** | 290 | **identical, byte for byte** |

**Every pixel digest, tile record, audio digest and cycle count of the whole `nes-test-roms`
collection, over 1 200 frames each, is unchanged between the baseline and the tip.** That is the
claim the lot needs — it rebuilt the CPU read path — and it is a `diff -q` rather than a count.
No build was needed: the two binaries are the ones instruments 1 and 2 were measured on.

The ignored test itself was then run the strong way as well, for continuity with MF-6c's figure:
**240 of 291 rows matched, 0 pending, 0 missing**, which is that lot's number. The junctions were
removed afterwards and `rust/test-roms` holds `nestest.nes` alone again — with them in place the
ignored test *fails*, and leaving a trap behind would be worse than leaving the weak check.

### The campaign: **331 rows, zero differing on any field**

    zax-harness test-roms G:/3dsen/cleanroom/assets-free/test-roms target-mf/camp-cart.json --no-shots
    python rust/target-mf/campfields.py rust/target-mf/camp-6c.json rust/target-mf/camp-cart.json

    base 331 rows, new 331 rows, same set: True
    summaries compare equal as objects: True
      base: {'total': 331, 'pass': 214, 'fail': 5, 'timeout': 0, 'unsupported': 1, 'manual': 111}
      new : {'total': 331, 'pass': 214, 'fail': 5, 'timeout': 0, 'unsupported': 1, 'manual': 111}

    rows differing on any field but ['ms', 'shot']: 0

    mapper numbers the campaign exercises: [0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180]
    of which this lot adds: []

**Zero rows differ**, `ms` and `shot` excluded, which is what the brief asks for rather than the
verdict alone; the suites are `nes-test-roms` (291 rows) and `holy-mapperel` (40, all forty
`pass`). The last line is the point: the eighteen numbers the campaign exercises include **none**
of this lot's, so what it proves is that a redesigned CPU read path, a new trait method, two
`#[inline(always)]` attributes and two un-parked boards changed **nothing** for the boards already
there.

### The `--ignored` suite, with both ROM folders

`ZAXNES_ROMS=G:/3dsen/demo/roms ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`,
`cargo test -p zax-nes -p zax-harness --release --no-fail-fast -- --ignored`: **7 passed, 1
failed**, MF-6c step 11's figure.

| test | |
| --- | --- |
| `zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack` | **ok** — the brief names it |
| `golden_pixels_tiles_audio_cycles_and_blargg_match_up_to_frame_1200` | **ok** — see above |
| `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line` | ok |
| `req_core_004_apu_reset_needs_a_reset_that_keeps_ram` | ok |
| `req_core_113_dmc_dma_during_read4_is_p3` | ok |
| `the_sprite_dma_and_dmc_dma_collision_is_not_cycle_exact` | ok |
| `the_cost_of_one_capture_and_one_restore` | ok |
| `builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom` | **FAILED — the known one, out of scope** |

The failure is the one the brief names as a known failure and MF-6c step 11 reproduced at *its*
baseline: the same assertion on the same line, `builtin_patches.rs:2525`. No commit of this lot
touches that file — `git log 44f77f72..HEAD -- crates/zax-nes/src/builtin_patches.rs` is empty —
and it belongs to the Zelda II patch lot.

### fmt, clippy and the suites

| configuration | `zax-nes`, debug | release, with `zax-harness` |
| --- | ---: | ---: |
| `default` | **862 / 0 / 7** | **925 / 0 / 8** |
| `--features parked-boards` | **862 / 0 / 7** | **925 / 0 / 8** |
| `--no-default-features` | **783 / 0 / 7** | — |

and the other crates that build in this worktree — `zax-goldens`, `zax-profiles`, `zax-settings`,
`zax-scripting` — are **615 / 0 / 68**, MF-6c step 10's figure unchanged. (`zax-app` cannot build
here, which the brief states.)

`cargo fmt --check` clean. clippy `-D warnings` clean on **`zax-nes` in five configurations** —
`default`, `--features parked-boards`, `--no-default-features`, and each un-parked feature alone on
top of `--no-default-features` — and on `zax-harness`. **No new `allow` of any kind** beyond the
two `clippy::inline_always` the two measured attributes carry, each with its figure on the line,
which is the convention the four such attributes already in `board.rs` and `cartridges/mod.rs`
follow.

### No behaviour changed, and four things say so independently

| | |
| --- | --- |
| the **save-state format** | untouched: no board gained, lost or moved a field, and `req_core_090_the_mapper_state_survives_a_save_and_restore` is in all three test columns |
| **golden-core** | 289 ROMs × 1 200 frames, the two digest files identical **byte for byte** |
| the **campaign** | 331 rows, **0** differing on any field |
| the **cache's own checker** | four `debug_assert!` in `default_cpu_read` — the offset against the dispatch replaced, the taken branch, **and its negation** — driven by **2 499 test bodies** across the three debug configurations, none tripping |

### The documents and the journal

* `rust/docs/PERFORMANCE-HISTORY.md` **§5.15**, with pointers from §5.14, §5.11 and §5.13 (step 6).
* `docs/review/MF-6-inventory.md` **§4.9 item 4** (step 6).
* `docs/review/BOARD-FEATURES.md`: the header, the two rows, the umbrella row, the how-to figures,
  and MF-6c's two diagnosis sections kept and marked superseded (step 5).
* `journal/JOURNAL.md`, the entry of 2026-11-07: the diagnosis, the two halves of the fix, the two
  refuted levers, the new limit, the verification figures, the disk, and the `resolvesyms.py`
  wrinkles.
* **No Q ticket.** Nothing in the spec was ambiguous: the lot adds no board, reads no new page and
  changes no behaviour, and nothing under `spec/` was touched.

### What the lot delivered

| | |
| --- | ---: |
| the limit on a board that shares a variant | **gone** — twelve more variants cost `mmc5` **+3 695**, worst ROM **+0.002 %** |
| boards un-parked | **5 mapper numbers** in two features (80, 207, 82, 552 and 111); `parked-boards` is **empty** |
| PR-1's thirteen ROMs | **−25 192 533 (−0.032 %)**, ten of thirteen below |
| `mmc5` | **−12 559 582 (−0.111 %)** |
| level-2 `core` | three below by 0.004 %–0.006 %, one **equal**, `mm2` **+0.002 %** |
| `Ppu2C02::read_pattern_planes` | **107**, unchanged |
| the three `Cpu6502::step` inlinings | **0 / 0 / 0**, and `zone_offset` still has no symbol |
| trait change | `prg_ram_offset` → `prg_ram_window`, and `cpu_read_windows` added; **no board overrides either** |
| shapes measured | **eight** (the cache, B1, B2, `repr(C)`, 4i, 4j, 4k, and the three-enum probes), **four refuted and written down** |
| tests added | **0** — it is a refactor, and its correctness claim is the existing suite with a checker wired into the path |
| Q tickets | **0** |

[PROGRESS] MF-CART step 7/8 done: the check. **golden-core is 289 ROMs over 1 200 frames each with
the baseline's and the tip's digest files identical byte for byte** — a stronger form than the
240-of-291 count, and free, because the two measured binaries already existed; the campaign is
**331 rows with zero differing on any field**; the `--ignored` suite is **7 of 8** with the one
failure the brief names, reproduced on a file no commit of this lot touches; **862 / 925 / 783 /
615** green with fmt and clippy `-D warnings` clean in five configurations and no new `allow`; and
the journal entry of 2026-11-07 is written. Lot complete: **the limit lifted, five mapper numbers
un-parked, `parked-boards` empty, and the tree 0.032 % faster than the one it started from.**
