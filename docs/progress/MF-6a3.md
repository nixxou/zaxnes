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

## Step 1 — 119, 47 and 37 rebuilt, and the first measured price of a `Board` **variant**

### What was built: MF-6a2 step 3's shape C, with two departures it measured the reason for

The design is `docs/progress/MF-6a2.md` step 3's, which is the whole of what the rebuild needed —
the page readings, the two Mesen 2 divergences, the six tests and the six measured shapes. The
code is that step's **shape C**, the best of its six: two `const` parameters on `Mmc3Board` so
that mapper 4 and 118 keep their own monomorphisation, the TQROM's `ppu_peek`/`ppu_write` bodies
`#[inline(never)]`, one `update_banks` body whose outer-bank arithmetic is compile-time dead off
the multicart, and the outer register's decode out of line.

`Mmc3Board` is now `Mmc3Board<const MC_ACC: bool, const TQROM: bool, const MULTICART: bool>` with
four monomorphisations instantiated and no others: `Mmc3` (`<false, false, false>`, mappers 4 and
118), `McAcc` (`<true, false, false>`), `Tqrom` (`<false, true, false>`) and `Mmc3Multicart`
(`<false, false, true>`).

**Departure 1: one `u8` of bit flags, where the design says `[u8; 8]`.** MF-6a2 step 3 records the
TQROM's chip select as "the eight windows' source bytes in a `[u8; 8]` refreshed when a register
commits". One `u8` carries the same eight answers, and the reason to prefer it postdates that
step: **MF-6b step 1 measured that a field offset is a cost even when no code changes** —
+9 886 794 on PR-1's set for one byte of displacement on `VrcIrq::pending`, with `step`, `resolve`
and `branch` unmoved to the instruction. The receipt is two numbers:

    size_of::<Mmc3>()   192 with both new fields, 192 with neither
    size_of::<Board>() 1376 with both new fields, 1376 with neither

**Both new `u8`s landed in the padding this struct already had**, so not one existing field moved
and MF-6b step 1's cost does not apply to this step at all. Eight bytes could not have done that.

**Departure 2 is the step's whole performance story**, and it is below.

### The page readings, and where the code says so

Every claim of MF-6a2 step 3's write-up is in the code with its page quoted, so the next reader of
either finds the other:

| what | where |
| --- | --- |
| 119's chip select being a line of the **bank number**, so a 2 KiB register puts both of its 1 KiB windows on the same chip | `Mmc3Board::refresh_chr_source` |
| a ROM window needing **no new arithmetic** (`BankWindows` already holds the wrapped register) | `Mmc3Board::tqrom_read` |
| the 8 KiB CHR-RAM chip's three bank lines | `Mmc3Board::tqrom_ram_offset` |
| `chr_peek_view` answering `None`, because a `ChrView` cannot say "this window is the other chip" | `Mmc3Board::chr_peek_view` |
| the 8 KiB being the **board's**, because byte 5 of an iNES header is the CHR-*ROM* size and REQ-CORE-012's default only covers an image with no CHR-ROM | `registry::board_memory_sizes`, arm 119 |
| 47's block bit as PRG A17 and CHR A17, the fixed windows becoming the block's last two | `Mmc3Board::multicart_prg`, `multicart_chr` |
| 37's `A16 = (Q0 AND Q1) OR (Q2 AND M16)`, `A17 = Q2`, and the page's whole four-row table | `Mmc3Board::multicart_prg`, with the table in the doc comment |
| 37's **two divergences from Mesen 2**, settled for the page by AMD-061 section 1 — block 7 at $30000-$3FFFF, and the MMC3's CHR bit 7 never reaching the CHR-ROM | the same two methods |
| the outer register writable **only while MMC3 PRG-RAM is enabled and writable** | `Mmc3Board::multicart_outer_write` |
| the window reading open bus, out of `prg_ram: Some(0)` | `registry::board_memory_sizes`, arm 37 \| 47 |

**One reading was tightened, exactly as MF-6a2 step 3 said to.** `is_mmc6`, `irq_revision_a` and
`ignore_a000`'s submapper-2 clause were `!is_txsrom && header.is_nes20 && submapper == N`, which
is mapper 4's condition expressed as "not 118". nesdev's *NES 2.0 submappers* defines submappers
1-4 for mapper 4 and **none** for 118, 119, 47 or 37, so a NES 2.0 dump of a TQROM carrying a
submapper byte would have been read as an MMC6 or an MMC3A. The three are now
`mapper_number == 4 && …`, which is identical for mapper 4 and 118.

**Both A12 latches needed the three numbers**, and the test that exists for it caught the
omission by itself: `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` failed with
`left: [4, 4, 5, 5, 37, 37, 47, 47, 48, 48, 64, 64, 118, 118, 119, 119, 158, 158]` against its own
expected list until `watches_ppu_reads` and `watches_ppu_bus_without_read` had them. That is the
third lot running in which that sweep has found a real omission before any measurement did.

**Save states**: one byte appended behind the existing versioning *and* behind the `const` that is
the only reason the field exists, so no byte of mapper 4's, 118's or the MC-ACC's block moves
(REQ-CORE-090). The TQROM appends nothing — `chr_ram_windows` is derived and `load_registers`
brings it back through `update_banks`, which is MF-5a step 3's `level` lesson.

### Departure 2 — **two `Board` variants cost `mmc5` +0.080 %, and the unit is the variant**

Shape C as MF-6a2 step 3 left it gives the two boards **two new `Board` discriminants**, 35 to 37.
Built that way (label `mf63-1-tqrom47a`) it is **+8 994 278 on `mmc5`, +0.080 %** — four times the
owner's tolerance — and every pinned symbol holds while it happens:

| symbol | `07325f6e` | **`tqrom47a`** (two variants) | **`tqrom47b`** (shipped) |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** | **0 / 0 / 0** |
| `Cpu::step` | 3 626 | 3 590 | **3 626** |
| `Board::ppu_peek` | 272 | 273 | **272** |
| `NesBus::write_internal` | 3 580 | 3 681 | 3 743 |
| `Ppu::run_dots` | 1 907 | 1 906 | **1 907** |
| `NesBus::catch_up_ppu` | **188** | **188** | **188** |

`catch_up_ppu` unmoved, `ppu_peek` within one instruction, `run_dots` within one — and `mmc5`
+8 994 278. The per-symbol diff names **one** function, `NesBus::push_expansion_audio`,
**+8 934 160**, and that function is **31 instructions in both dumps** with
`Board::expansion_audio_output` still inlined into it whole (0 / 0 / 0, its `#[inline(always)]`
holding). So nothing grew. What changed is in its jump table, and two lines of the dump say it:

| | `07325f6e` | `tqrom47a` |
| --- | --- | --- |
| the MMC5's table entry | `.LBB_8` | `.LBB_3` |
| that block | `movl 10776(%rdi), %eax` *(falls through)* | `movl 10776(%rdi), %eax` ; **`jmp .LBB_10`** |

**The MMC5 lost the fall-through into the shared tail.** The table grew from 35 entries to 37 and
LLVM placed a different one of the five expansion-audio arms last. The arithmetic closes exactly:
`push_expansion_audio` is 142 946 560 exclusive instructions over about 8.9 M samples, and
8 934 160 over 8.9 M is **one instruction per sample**.

**This is MF-6a2 step 1's residual one dispatch along, and MF-6b step 1's gain read backwards.**
That step recorded "`NesBus::push_expansion_audio` **−8 934 160**, which the thirty-fifth `Board`
variant re-laid out on its own" — **the same number to the instruction**, in the other direction.
The two measurements close on each other and give a rule narrower than section 4.9's "a `Board`
variant is free":

> **A `Board` variant is free in every dispatch except one whose jump table a hot function
> inlines whole, and `NesBus::push_expansion_audio` is that function.** Its unit is the
> *discriminant count*, not the body count — the four MF-6a/MF-6a2/MF-6b findings before it all
> priced a *body*.

### The fix: no new `Board` variant at all

The three numbers go into **[`IrqBoard`]**, which leaves `Board` at thirty-five discriminants and
that jump table byte-identical. They belong there by the enum's own entry condition: all three
*are* the MMC3's board, counter included, so all three answer **yes** to `Mapper::irq_asserted`.

Two methods had to be forwarded, and **both are behaviour-neutral for the six boards MF-6b put
there** — none of the six overrides either, so each one's own default gives the answer the
un-forwarded dispatch gave it:

* `prg_ram_writable`, for the TQROM alone: it is an MMC3 board, so $A001 bit 6 write-protects its
  RAM, and without the forward `Mapper::prg_ram_writable`'s default would answer
  `IrqBoard::prg_ram_enabled` and bit 6 would do nothing;
* `intercept_cpu_write`, for the two multicarts alone, whose outer register *is* the board's
  decode of $6000-$7FFF.

**The price the three boards pay** is `IrqBoard::ppu_irq_horizon`'s `None`: an MMC3 could bound
the PPU's lag and these three give it up, so their games run the PPU in step with the CPU. That is
exactly the trade MF-6b step 4 charged the TC0690 and for the same reason — any forward there is
one more body in a dispatch `NesBus::catch_up_ppu` inlines, worth +9 214 379 on `mmc5` for two
instructions — and it is section 4.10's rule: the cost lands on the boards that wanted the
variant, not on the library.

[`IrqBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/irq_boards.rs

### The six tests, 887 to **893**

| test | what it pins |
| --- | --- |
| `req_core_100_tqrom_bit_6_of_a_chr_bank_selects_the_chr_ram_chip` | bit 6 in **both** CHR modes, on a 1 KiB register and on both halves of a 2 KiB one, and `chr_peek_view` answering `None` |
| `req_core_100_tqrom_a_write_to_a_ram_selected_window_is_not_swallowed` | section 5's own words: the write lands in RAM, is dropped on a ROM window, is visible through a second window on the same RAM bank and not through another, the CHR-ROM is untouched, and the board's 8 KiB is 8 KiB |
| `req_core_100_mapper_47_switches_128_kib_blocks_of_prg_and_chr` | the two blocks, PRG and CHR, the fixed windows being the block's last two, a bank number wrapping inside the block, and CHR A17 coming from the outer register |
| `req_core_100_mapper_37_reproduces_its_outer_bank_table` | all eight values against the MMC3's A16 clear and set, i.e. the page's whole table, plus bits 3-7 not being latched and the MMC3's CHR bit 7 being dropped rather than ORed in |
| `req_core_100_the_mmc3_multicart_outer_register_needs_prg_ram_enabled_and_writable` | $A001 = $80 / $C0 / $00 / $80 on **both** numbers, and the window reading open bus |
| `req_core_100_an_ordinary_mapper_4_image_is_unaffected_by_119_47_and_37` | a mapper-4 write to $6000 moving no bank and keeping its PRG-RAM, CHR bank $A0 being bank $A0 and not bank $20 of a block, the four windows spanning the whole image, and mapper 4 still offering a `chr_peek_view` |

The first of them needed `txsrom_bank_in_chr_mode_1`, the helper mapper 118's tests already carry,
for the trap its own note describes: the register index and the CHR mode share $8000, so
`mmc3_bank` — which writes the index alone — puts the board back in CHR mode 0 on every call. A
first draft that used `mmc3_bank` in CHR mode 1 read the wrong window and failed, which is that
helper earning its existence a second time.

**Five configurations build, pass clippy `-D warnings` and pass their tests**, which is what says
the gating is real rather than decorative:

| configuration | `zax-nes` tests |
| --- | ---: |
| `--no-default-features` | 825 / 0 / 7 |
| `--no-default-features --features board-119` | 827 / 0 / 7 |
| `--no-default-features --features board-mmc3-multicart` | 828 / 0 / 7 |
| default | 830 / 0 / 7 |
| default `--features parked-boards` | 830 / 0 / 7 |

825 + 2 + 3 = 830, the sixth test being the mapper-4 non-regression, which is in no feature. With
`zax-harness` the default figure is **893 passed, 0 failed, 8 ignored** — MF-6b's 887 plus six.
`cargo fmt --check` clean; clippy `-D warnings` on `zax-nes` and `zax-harness`, with and without
`parked-boards` and with `--no-default-features`, clean, **no new `allow` of any kind**.

[PROGRESS] MF-6a3 step 1/6 done: 119, 47 and 37 on `Mmc3Board`, 893 / 0 / 8, and two `Board` variants priced at +0.080 % on `mmc5` before the three numbers went into `IrqBoard` instead.

## Step 2 — the measurement, against the owner's tolerance

The tree measured is step 1's, at label `mf63-1-tqrom47b`, on a clean `make-tree.sh --worktree`
build in its own directory with its own target directory. **The default build is what is
measured**: `board-119` and `board-mmc3-multicart` are both in `zax-nes`'s `default`, so this is
what a `cargo build` of the shipped configuration produces.

### PR-1's thirteen: every ROM inside ±0.0011 %, set total **−57 265**

`callgrind-diff.py`, against `mf63-0-07325f6e`:

| ROM | board | `07325f6e` | **step 1** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 378 921 | **3 308 293 511** | −85 410 | −0.0026 % |
| `oamstr` | NROM | 3 318 459 130 | **3 318 454 343** | −4 787 | −0.0001 % |
| `m1` | MMC1 | 4 885 841 284 | 4 885 841 629 | **+345** | **+0.0000 %** |
| `m0` | NROM | 5 520 389 483 | 5 520 447 790 | **+58 307** | **+0.0011 %** |
| `m4` | **MMC3** | 5 472 545 224 | 5 472 559 790 | **+14 566** | **+0.0003 %** |
| `metroid` | MMC1 | 5 115 944 243 | **5 115 924 968** | −19 275 | −0.0004 % |
| `smb1` | NROM | 5 996 346 457 | **5 996 326 465** | −19 992 | −0.0003 % |
| `mm2` | MMC1 | 6 017 889 238 | **6 017 844 725** | −44 513 | −0.0007 % |
| `z2` | MMC1 | 6 291 431 663 | 6 291 486 006 | **+54 343** | **+0.0009 %** |
| `readbuf` | CNROM | 6 834 547 836 | **6 834 546 677** | −1 159 | −0.0000 % |
| `duelito` | NROM | 7 380 393 405 | 7 380 419 534 | **+26 129** | **+0.0004 %** |
| `smb3` | **MMC3** | 7 949 452 274 | **7 949 441 696** | −10 578 | −0.0001 % |
| `mmc5` | MMC5 | 11 286 830 608 | **11 286 805 367** | −25 241 | −0.0002 % |
| **total** | | **79 378 449 766** | **79 378 392 501** | **−57 265** | **−0.0001 %** |

**The worst ROM of the thirteen is `m0` at +58 307, which is +0.0011 %** — one eighteenth of the
+0.02 % the owner allows, and inside the ±57 k-per-ROM `luaH_getshortstr` floor §5.11 measured.
The two MMC3 ROMs — `m4` and `smb3`, which every MF lot since MF-6a has made the ones to watch
because the three new boards *are* the MMC3's board — are **+14 566 and −10 578**. `mmc5`, which
the two-variant shape cost +8 994 278, is **−25 241**.

For comparison, MF-6a2 step 3's best shape (C) left `m4` +258 640, `smb3` +390 724, `mm2`
+992 528 and `m0` +786 360 on the same instrument. Every one of those is between five and
eighteen times this step's worst figure, and the difference is the one change of step 1: no new
`Board` discriminant.

The only symbol that moves by 300 000 or more on any ROM of the set is `Mmc3Board::update_banks`
**being renamed by its new monomorphisation signature**:

    smb3:  +817,035  0 -> 817,035  <...mmc3::Mmc3Board<false, false, false>>::update_banks
           -817,035  817,035 -> 0  <...mmc3::Mmc3Board<false>>::update_banks

**817 035 to 817 035, the same number to the instruction** — mapper 4's banking is the same code
doing the same work under a new name, which is what the `const` parameters exist to guarantee and
what MF-6b step 5 took as its receipt for `Rambo1::update_banks`.

### Level 2: **all five `core` figures within ±6 699**, and smb1's at exactly 0

`l2-diff.py --bar`, against `mf63-0-07325f6e`:

| game/stage | `07325f6e` | **step 1** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 147 278 310 | 11 147 281 665 | **+3 355** | **+0.00003 %** |
| `smb1/core` | 11 515 125 238 | 11 515 125 238 | **0** | **equal** |
| `zelda2/core` | 12 775 641 366 | 12 775 647 488 | **+6 122** | **+0.00005 %** |
| `mm2/core` | 12 036 819 361 | **12 036 800 417** | −18 944 | −0.0002 % |
| `smb3/core` | 15 315 558 069 | **15 315 551 370** | −6 699 | −0.0000 % |
| `mm2/total` | 18 976 300 081 | **18 976 261 589** | −38 492 | −0.0002 % |
| `smb1/total` | 13 587 898 562 | **13 587 226 944** | −671 618 | −0.0049 % |
| `metroid/total` | 13 735 514 951 | 13 735 608 782 | +93 831 | +0.0007 % |
| `zelda2/total` | 15 358 834 193 | 15 361 709 526 | **+2 875 333** | **+0.019 %** |
| `smb3/total` | 24 187 868 326 | 24 194 302 228 | **+6 433 902** | **+0.027 %** |

**The `core` column — the one with no run-to-run spread at all — is unmoved on all five games**,
the largest move being `zelda2` at +6 122 instructions in 12.8 billion. `mm2/core`, which is the
figure that ended MF-6a2 step 3 at **+2 136 243 (+0.018 %)**, is **−18 944** here: that step's
stop-the-lot figure is gone, 113 times over, and in the other direction.

`smb3/total` at +0.027 % is the one figure above the tolerance, and the lot's own rule requires
the per-function diff rather than a label. `l2sym.py`, moves of 250 000 or more:

| game | symbol | delta |
| --- | --- | ---: |
| `smb3` | `ltable.c:luaH_getshortstr` | **+6 636 177** |
| `smb3` | `Mmc3Board<false, false, false>::update_banks` | +2 836 959 |
| `smb3` | `Mmc3Board<false>::update_banks` | −2 836 959 |
| `zelda2` | `ltable.c:luaH_getshortstr` | **+1 499 147** |
| `zelda2` | `ltable.c:getgeneric` | **+1 382 656** |

**One Lua symbol accounts for more than the whole of `smb3/total`'s rise** — `luaH_getshortstr`
+6 636 177 against a measured +6 433 902, so this project's own code is net *down* on that game —
and `zelda2`'s two Lua symbols come to +2 881 803 against a measured +2 875 333, which is the
whole of it to within 6 470. `Mmc3Board::update_banks` is the rename again, +2 836 959 against
−2 836 959, byte-identical.

### The floor on that column, measured rather than asserted

MF-6a2 step 2 measured the level-2 `total` run-to-run floor at ±2.7 M, and both figures above are
larger than that, so the step measured the floor again instead of leaning on an old number. **The
same binary, at the same path, through `callgrind-l2.sh` twice** (labels `tqrom47b` and
`tqrom47c`, the second a copy of the first binary so the path length is identical):

| | spread over the two runs |
| --- | --- |
| every level-2 `core` stage | **0, to the instruction, on all five games** |
| `smb3/total` | **−4 284 799** (−0.018 %) |
| `zelda2/total` | **−3 097 124** (−0.020 %) |
| `smb1/total` | +274 361 |
| `metroid/total` | −53 986 |
| `mm2/total` | −12 438 |

So on this instrument **`zelda2/total`'s +2 875 333 is smaller than the same binary's own spread
on that column (3 097 124), and `smb3/total`'s +6 433 902 is 1.5 times it** — while every `core`
stage is identical to the instruction across the two runs, which is what makes `core` the column
that can be read. The second run puts `smb3/total` at 24 190 017 429, **+0.009 % against the
baseline**, inside the tolerance: the verdict on that column depends on which of two readings of
the same binary one takes, which is the definition of noise.

Step 0 had already shown the same thing from the other side: `smb3/total` read 24 187 868 326 at
`07325f6e` against MF-6b step 5's 24 193 872 632 for **the same source tree**, a 6.0 M difference
with no code change at all.

**This refines §5.12's figure for a later lot**: the level-2 `total` floor is ±2.7 M on `mm2` and
`metroid` and **±3.1 M to ±4.3 M on `zelda2` and `smb3`**, the two games whose packs run the most
Lua, against **0** on every `core` stage of all five.

### The post-LTO check

`pins63.py`, step 1's dump against the baseline's — the three claimants and the five symbols the
brief names:

| symbol | `07325f6e` | **step 1** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 626 | 1 / 2 / **3 626** |
| `Board::ppu_peek` | 1 / 4 / 272 | 1 / 4 / **272** |
| `NesBus::write_internal` | 1 / 10 / 3 580 | 1 / 10 / 3 743 |
| `Ppu::run_dots` | 1 / 5 / 1 907 | 1 / 5 / **1 907** |
| `NesBus::catch_up_ppu` | 1 / 12 / 188 | 1 / 12 / **188** |

and `shape.py` beside it: `Cpu::resolve` 2 / 6 / **1 948**, `Cpu::branch` 1 / 1 / **457**,
`NesBus::read_cycle` 1 / 65 / **181**, `tick` 1 / 95 / **82**, `read_internal` 1 / 17 / **175**.

**Seven of the eight pinned symbols are identical to the instruction, and so are all five of
`shape.py`'s others.** `zone_offset` did not get dearer; it does not exist as a symbol, which is
the only reading of it that matters. The one that moved is `NesBus::write_internal`, 3 580 →
**3 743** (+163): the TQROM's and the multicarts' register decodes plus the `intercept_cpu_write`
forward. Its dynamic cost did not follow — `m4` +14 566 and `smb3` −10 578 are the two ROMs whose
writes go down that path most — which is the same relation MF-6b step 2 recorded when that
function went *down* with two boards added.

`Board::ppu_peek` back at **272** is the measurement MF-6a2 step 3 asked for by name: that step
found the TQROM's chip-select body took it to **583** and cost `m4` 3.4 M, and named
`#[inline(never)]` on the board's own body as the fix. The fix holds, and the fourth capped hook
comes out of this step **at the figure it went in at**.

### The verdict against the tolerance

| the owner's condition | measured |
| --- | --- |
| at most **+0.02 % per ROM** on PR-1's set | worst is `m0` **+0.0011 %**; set total **−0.0001 %** |
| at most **+0.02 % per ROM** on level 2, `core` | worst is `zelda2` **+0.00005 %**; two of five below |
| at most **+0.02 % per ROM** on level 2, `total` | `smb3` +0.027 % and `zelda2` +0.019 %, both **inside the same binary's own run-to-run spread on that column** (−4 284 799 and −3 097 124), with the whole of both in Lua's `ltable.c` and every `core` stage identical across the two runs |
| the three claimants of `Cpu6502::step` inlined, 0 / 0 / 0 | **0 / 0 / 0** |
| `zone_offset` no dearer | **0 / 0 / 0**, no symbol at all |

**The three boards are inside the tolerance, and `board-119` and `board-mmc3-multicart` stay in
`zax-nes`'s `default`.** Nothing is parked, so `parked-boards` stays empty.

[PROGRESS] MF-6a3 step 2/6 done: worst PR-1 ROM +0.0011 %, all five level-2 cores within ±6 699 with `smb1`'s at 0, the two `total` rises inside the same binary's own ±4.3 M spread on that column, and the three boards default-on.

## Step 3 — one feature per board that measured a rise, at the granularity the measurements exist at

### Which boards, and why these

The owner's rule is "every board that raises any figure gets its own Cargo feature ... convert
every board of MF-6a, MF-6a2 and MF-6b whose step measured any rise". Read off the three progress
files, those are:

| lot and step | board | the rise that step measured |
| --- | --- | --- |
| MF-6a step 1 | Namco 108 / DxROM (**206**, and **76**, **88**, **95**, **154**) | `smb3/core` **+45 707**, `m4` **+66 921** |
| MF-6a step 2 | `SmallBoard`'s first five (**33**, **97**, **140**, **184**, **89**, **93**) | `smb3/core` **+433 086** |
| MF-6a step 3 | four more in the same variant (**70**, **152**, **75**, **232**) | `smb3/core` **+461 302** |
| MF-6a step 4 | **94** on `UxRom`, **155** on `Mmc1`, **185** in `SmallBoard` | `mm2/core` **+10 506**, `smb3/core` **+461 302** |
| MF-6b step 3 | NES-EVENT (**105**) | `smb1/total` **+0.023 %**, `smb3/total` **+0.010 %** |
| MF-6b step 5 | Tengen 800037 (**158**) | `smb3/total` **+0.005 %** |
| MF-6a3 step 1 | TQROM (**119**), the two multicarts (**47**, **37**) | step 2's table |

**MF-6b has nothing parked**, and its step 8 says why in as many words: "No board of this lot was
parked ... every board that missed the bar missed it on a shape that was fixable". So the
`parked-boards` umbrella has nothing to enable, which is the state step 0 created it in.

MF-6b steps 1, 2 and 4 are **not** in the list on purpose: each of those shipped with every one
of PR-1's thirteen ROMs and all five level-2 games at or below `9f9d9d41` on total *and* core, and
`l2-diff.py`'s own verdict line for steps 1, 2 and 4 is "at or below the baseline on every game,
total and core". No figure rose, so no feature. MF-6a2 added no board at all.

### The granularity, stated rather than assumed

The rule's unit is "one feature per board ... or one for a family sharing one variant", and this
step takes the second form wherever a family *does* share a variant, because **that is also the
granularity at which the measured costs exist**:

* `board-namcot108` is one board with four `const`/`bool` parameters — five numbers, one
  `Board` variant, one measurement (MF-6a step 1's);
* `board-small-boards` is nine boards behind **one** `Board` discriminant, which is the whole
  reason [`SmallBoard`] exists: MF-6a step 2 measured five *variants* at 0.067 % to 0.131 % on
  eight of thirteen ROMs and one at nothing. Splitting it into nine features would mean nine
  independent `#[cfg]`s on one enum and **nine measurements that no lot has made** — MF-6a priced
  its steps 2, 3 and 4, not its boards. One feature, three measured step figures in the table;
* `board-mmc3-multicart` is 47 and 37, which the owner's rule names as the example.

A later lot that wants a finer split on `SmallBoard` has to measure the nine boards one at a time
first, and `docs/review/BOARD-FEATURES.md` says so in its own note.

[`SmallBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/small_boards.rs

### What each feature gates

| | gated |
| --- | --- |
| modules and exports | `mappers/mod.rs`: `mod` and `pub use` for 12 board files |
| `Board` variants | `Namcot108`, `SmallBoard`, and their `dispatch!` arms |
| inner variants | `IrqBoard::NesEvent`, `IrqBoard::Tengen800037`, and their `inner!` arms |
| inner forwards | `IrqBoard::prg_ram_enabled` (now `any(board-105, board-119)` — the TQROM needs it too), `IrqBoard::nametable_page` (`board-158`) |
| `const` monomorphisation | `Rambo1Board<true>`'s alias (`board-158`) |
| the two parameters | `UxRom`'s `shifted_field` and `Mmc1`'s `wired` mask, through **`cfg!`** rather than `#[cfg]`: with the feature off each folds to the constant the board always had, so the comparison is gone and the host board is byte-for-byte the code it was |
| the five `Mmc1` accessors | `control_register`, `chr0_register`, `prg_register`, `windows_mut`, `force_chr0_register` — mapper 105 is their only caller, and rustc says so when the feature is off |
| `registry` | `SUPPORTED_MAPPERS` (one `#[cfg]` per number), `create_board` arms, `board_memory_sizes` arms, the imports |
| tests | 44 items across `mapper_tests.rs` and `mapper_irq_tests.rs`, plus three expected lists |

**The two A12 latch lists and `has_bus_conflicts` are deliberately *not* gated.** A number whose
board is off is not in `SUPPORTED_MAPPERS`, so `Cartridge::create` refuses it and no cartridge of
it is ever built: the arm is unreachable either way. Leaving them as one `matches!` whatever the
features means flipping a feature does not touch a load-time decision, which is one less thing
for a later reader to hold.

**Three expected lists are now built rather than written**, which is the one place a feature can
make a sweep lie:

* `req_core_100_every_required_mapper_is_registered` — REQ-CORE-100's own 31 numbers are in no
  feature, so its first loop holds in every configuration; the 37 additions beyond that table are
  a `[(i32, bool); 37]` whose flag is the number's own `cfg!`, so an accidental addition is still
  caught as surely as an accidental removal;
* `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` — its guard against a vacuous
  pass is assembled from the features (119 and the two multicarts, then 158);
* the other three sweeps (`mf_5a_…`, `amd_061_a_reset_…`, `req_core_090_…`) iterate
  `SUPPORTED_MAPPERS` and needed nothing.

### Nine configurations build, pass clippy and pass their tests

| configuration | `zax-nes` tests | the additions its tests make |
| --- | ---: | ---: |
| `--no-default-features` | **782 / 0 / 7** | — |
| `--features board-94` | 783 | +1 |
| `--features board-155` | 783 | +1 |
| `--features board-119` | 784 | +2 |
| `--features board-mmc3-multicart` | 785 | +3 |
| `--features board-158` | 786 | +4 |
| `--features board-105` | 791 | +9 |
| `--features board-namcot108` | 790 | +8 |
| `--features board-small-boards` | 802 | +20 |
| **default** (all eight) | **830 / 0 / 7** | +48 |

782 + 48 = 830 exactly, which is what says each feature carries its own tests and no others.
With `zax-harness` the default figure is **893 passed, 0 failed, 8 ignored**, unchanged from step
1. `cargo fmt --check` clean; clippy `-D warnings` clean on `zax-nes` and `zax-harness` in the
default configuration, with `--features parked-boards`, and with `--no-default-features` —
**no new `allow` of any kind, and no `dead_code` or `unused` warning in any of the nine**. The
registry advertises **68 numbers by default and 45 with every board feature off**.

### The receipt: the retrofit is free in the default build

This step adds `#[cfg]` attributes and nothing else, so with every feature on the code must be
the code step 1 shipped. It is shown twice rather than asserted.

**The post-LTO dump.** `pins63.py`, step 3 against step 1:

| symbol | step 1 | **step 3** |
| --- | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 626 | 1 / 2 / **3 626** |
| `Board::ppu_peek` | 1 / 4 / 272 | 1 / 4 / **272** |
| `NesBus::write_internal` | 1 / 10 / 3 743 | 1 / 10 / **3 743** |
| `Ppu::run_dots` | 1 / 5 / 1 907 | 1 / 5 / **1 907** |
| `NesBus::catch_up_ppu` | 1 / 12 / 188 | 1 / 12 / **188** |

and `cmp4c.py` over its whole pinned list gives **the same instruction count for every one of the
thirty-odd symbols it watches** — `read_cycle` 181, `tick` 82, `read_internal` 175, `branch` 457,
`resolve` 1 948, `run_dma` 992, `background_fetch` 196, `read_nametable_inner` 114,
`Board::ppu_read` 356, `Board::on_cpu_cycle` 460, `Board::on_ppu_address_bus` 341 — with the
normalised bodies IDENTICAL on all but four, and those four differ only in the `.Lanon` and
section names the crate disambiguator changes between any two builds (MF-6a2 step 3 records that
hazard: the two trees are built at different times and carry different `Cs<12>_` hashes).

**PR-1's thirteen.** Step 3 against step 1:

| | |
| --- | --- |
| largest move on any ROM | **+40 812** (`duelito`), **−33 942** (`z2`) |
| set total | **+93 759**, +0.0001 % |
| symbols moving by 200 000 or more, on any ROM | **none at all** |

Every ROM is inside the ±57 k-per-ROM `luaH_getshortstr` floor §5.11 measured, and the
per-symbol diff prints nothing: no emulated symbol moved on any of the thirteen.

**And against the base commit**, which is the figure the tolerance is read on:

| ROM | `07325f6e` | **step 3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 308 378 921 | **3 308 312 658** | −66 263 | −0.0020 % |
| `oamstr` | 3 318 459 130 | 3 318 466 874 | +7 744 | +0.0002 % |
| `m1` | 4 885 841 284 | 4 885 864 197 | +22 913 | +0.0005 % |
| `m0` | 5 520 389 483 | 5 520 457 293 | **+67 810** | **+0.0012 %** |
| `m4` | 5 472 545 224 | 5 472 580 421 | +35 197 | +0.0006 % |
| `metroid` | 5 115 944 243 | **5 115 918 037** | −26 206 | −0.0005 % |
| `smb1` | 5 996 346 457 | **5 996 340 925** | −5 532 | −0.0001 % |
| `mm2` | 6 017 889 238 | **6 017 832 742** | −56 496 | −0.0009 % |
| `z2` | 6 291 431 663 | 6 291 452 064 | +20 401 | +0.0003 % |
| `readbuf` | 6 834 547 836 | **6 834 521 169** | −26 667 | −0.0004 % |
| `duelito` | 7 380 393 405 | 7 380 460 346 | **+66 941** | **+0.0009 %** |
| `smb3` | 7 949 452 274 | 7 949 465 563 | +13 289 | +0.0002 % |
| `mmc5` | 11 286 830 608 | **11 286 813 971** | −16 637 | −0.0001 % |
| **total** | **79 378 449 766** | **79 378 486 260** | **+36 494** | **+0.00005 %** |

Worst ROM **+0.0012 %**, one seventeenth of the tolerance; six of thirteen below the baseline.

**The level-2 run is not repeated at this step, and that is a decision rather than an omission.**
Step 3 changes no instruction of the default build — shown by the dump and by thirteen ROMs with
no symbol moving — so a level-2 run could only re-measure its own `total`-column floor, which
step 2 already measured at ±4.3 M. The shipped tree is measured on both instruments at the end of
step 4, with mapper 116 in it.

[PROGRESS] MF-6a3 step 3/6 done: eight board features, nine configurations green with no `dead_code` warning in any, 782 / 0 / 7 with every board off against 830 / 0 / 7 by default, and the default build's dump instruction-for-instruction step 1's.

## Step 4 — mapper 116, the Gouder SOMARI-P, and Q-130 question 1 answered

### The board, from MF-6b step 6's design

`docs/progress/MF-6b.md` step 6 left mapper 116 "open on purpose", with its register map, its
three constructors, its one `BankWindows` method, its four submappers' readings and its one
ticket all written down; this step is that design built. [`Somari`] **holds** an [`Mmc3`], an
[`Mmc1`] and a [`Vrc2Vrc4`] and delegates to the active one, which is [`super::NesEvent`]'s shape
read three times over — the page requires exactly that: "three chips each keeping its own
registers, so that when switching between modes using the supervisor register, the entire mapper
state changes, including PRG/CHR-ROM switchable and fixed banks as well as IRQ state".

| what the design said | where it is |
| --- | --- |
| supervisor register `$4100`, `(addr & $E100) == $4100`, `[.... .CMM]`, `C` at **bit 2**, power-on `$01` | `Somari::SUPERVISOR_MASK`, `POWER_ON`, `outer_chr` |
| the `address < 0x8000` guard **not** first | `Somari::write_register`, with the reason in its own doc |
| three chips, each keeping its own registers | the three fields, and `Somari::refresh` |
| the VRC2 mode is a **VRC2b**, asked for directly because a mapper-116 image declares no VRC submapper | `Vrc2Vrc4::new_vrc2b` |
| the VRC2's CHR registers powering up to `$FF` | `Vrc2Vrc4::force_chr_registers` |
| an MMC3 that cannot be read as an MMC6 or a TxSROM | `Mmc3::new(memory, 116)` — **no new constructor needed**, because step 1's submapper tightening made `Mmc3Board::new` read a submapper only on mapper 4 |
| only the MMC3 mode has an IRQ | `Somari::irq_asserted` and `on_ppu_address_bus`, both mode-gated |
| the outer CHR bank as a new `BankWindows` method | `BankWindows::offset_chr_banks`, additive and behind `board-116` |
| the MMC1 mode ignoring the outer bank | `Somari::refresh`'s third arm |
| submappers 0 and 1, the two PALs | `Somari::write_supervisor` |
| submapper 2, the Huang-2's 8 KiB PRG | `Somari::refresh_mmc1` |
| submapper 3, the one real gap | **Q-132**, written |

**One thing the design did not anticipate, and it is the step's one real finding.** `Somari` is
**536 bytes** — three sub-boards — so as an inline variant of [`IrqBoard`] it would be the variant
that sets that enum's size, 200 → 544. MF-6b step 1 measured what moving an IRQ board's fields
costs: **one byte** of displacement on `VrcIrq::pending`, which `Board::irq_asserted` reads for
the VRC3 arm of this very enum, re-laid out `Cpu6502::branch` and cost eleven of PR-1's thirteen
ROMs +0.009 % to +0.027 % **with no code added at all**. So the variant is **boxed**:

    size_of::<Board>()     1376   (the MMC5's, unchanged since MF-2)
    size_of::<IrqBoard>()   200   (MF-6a3 step 1's figure, unchanged)
    size_of::<Somari>()     536

`clippy::large_enum_variant` fires on the unboxed form and the lot's terms allow **no new
`allow`**, so the two constraints point the same way. The price is one pointer dereference per
forwarded call **for mapper 116 and no other board**, on a `Board` value built once at load —
§4.10's rule again.

### The save state, and the one ordering trap this board has

[`Mapper::save_state`] and [`Mapper::load_state`] are overridden rather than
`save_registers`/`load_registers`, and the order is the whole reason. The defaults write
`banks()` and then the registers, and `banks()` on this board is **whichever chip the current
mode names** — so a state restored into a board sitting in a different mode would have loaded one
chip's windows into another's. The supervisor register is written **first** and each chip's own
`save_state` after it, so every block has exactly one owner and the mode is known before any of
them is read. `load_state` then puts both chips the outer CHR bank reaches back to their
*register-derived* windows before re-applying it, so a state cannot come back with the bank
counted twice or not at all.

`req_core_090_the_mapper_state_survives_a_save_and_restore` and
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` sweep
`SUPPORTED_MAPPERS` and cover the board without a line written.

### Q-130 question 1 answered, and Q-132 opened

Q-130's provisional choice for mapper 116 was "**not implemented at all**, so REQ-CORE-011
refuses it", pinned by `req_core_102_any_other_mapper_is_refused`'s own comment. R0's decision and
the brief reverse it, so **both tests that carried the choice were edited to say so**:

* `req_core_102_any_other_mapper_is_refused` drops 116 from its sweep when `board-116` is on,
  with the comment pointing at Q-130;
* `req_core_102_only_implemented_mappers_are_accepted` (`rom_loading_tests.rs`) asserted
  `!SUPPORTED_MAPPERS.contains(&116)` and now asserts `cfg!(feature = "board-116") ==
  contains(&116)`.

**REQ-CORE-102's own acceptance criterion is untouched**: it names mapper 200, which §6 item 1 of
the inventory keeps out, and both tests still refuse 6, 177 and 200.

**`spec/changes/Q-132.md`** is written, with two readings and one note:

1. **submapper 3**, the reset-based multicart: the page says nothing about how a reset selects
   the game, and Mesen 2 does not implement it. Provisional and implemented: it loads as a
   Huang-1 with no reset-based outer bank, so the first game runs and the rest are unreachable;
2. **submapper 2**, the Huang-2: "the PRG bank register value is shifted by one bit to the left
   compared to what one would write to a normal MMC1" admits the **formula** (`register >> 1`,
   then the MMC1's own layout) and the **prose** ("addresses 8 KiB rather than 16 KiB banks", so
   an odd register gives an unaligned 16 KiB view). The formula is implemented, because it is
   the sentence and nothing more, and both are in the ticket;
3. **submappers 0 and 1** are *not* a question, and the ticket records why: the implementation
   puts the shift-register reset on submapper 1, and the page closes it itself — "every
   single-game cartridge is submapper 0 except the GK2 *Somari*, and that game does not seem to
   rely on it", so **no software in the sources can observe the difference either way**.

### Six tests, 893 → **899**

| test | what it pins |
| --- | --- |
| `req_core_100_the_somari_supervisor_register_is_at_4100_under_an_e100_mask` | the power-on MMC3 mode, **seven** mirrors the `$E100` mask admits, and `$4000` not being one — which is also the test that fails if the `address < 0x8000` guard is put first |
| `req_core_100_the_somari_switches_the_whole_mapper_state_between_its_three_chips` | a layout only each chip could produce, surviving a round trip through both others, and mode 3 being mode 2 |
| `req_core_100_the_somari_outer_chr_bank_is_bit_2_and_the_mmc1_mode_ignores_it` | `C` at bit 2 and **not** bit 3, 256 KiB of CHR, the bank **not counted twice** on a further register write, and the MMC1 mode ignoring it |
| `req_core_100_the_somari_vrc2_mode_is_a_vrc2b_with_its_chr_registers_at_ff` | the `$FF` power-on, A0 picking the nibble and A1 the half-register, and `$F000-$FFFF` moving nothing because a VRC2 has no IRQ |
| `req_core_100_the_somari_w_pal_resets_the_mmc1_shift_register_on_the_switch` | submapper 1 against submapper 0, through a `$E000` write deliberately left three bits short |
| `req_core_100_the_somari_does_not_disturb_the_three_chips_it_holds` | mappers 4, 23 and 1 unchanged: no supervisor register on any of them, and the VRC2's own power-on CHR still zero |

The third test reads the windows through `chr_peek_view().offsets()` rather than through
`MapperMachine::chr_windows`, and the reason is worth the line: the fixture fills CHR bank N with
`N as u8` and this board's outer bank moves a window by **256** banks, so the two would read the
same byte. The offsets distinguish them.

**Eleven configurations build, pass clippy `-D warnings` and pass their tests:**

| configuration | `zax-nes` tests |
| --- | ---: |
| `--no-default-features` | 782 / 0 / 7 |
| each of the nine features alone | 783 to 802 |
| `--no-default-features --features board-116` | **788 / 0 / 7** |
| default (all nine) | **836 / 0 / 7** |
| default `--features parked-boards` | 836 / 0 / 7 |

With `zax-harness`, **899 passed, 0 failed, 8 ignored** — MF-6b's 887 plus this lot's twelve.
`cargo fmt --check` clean; clippy `-D warnings` clean in all eleven, **no new `allow` of any
kind**. The registry advertises **69 numbers by default** and 45 with every board feature off.

### The shipped tree, on both instruments

**PR-1's thirteen**, against `mf63-0-07325f6e`:

| ROM | board | `07325f6e` | **step 4** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `vblnmi` | MMC1 | 3 308 378 921 | **3 308 288 529** | −90 392 | −0.0027 % |
| `oamstr` | NROM | 3 318 459 130 | 3 318 478 745 | +19 615 | +0.0006 % |
| `m1` | MMC1 | 4 885 841 284 | 4 885 937 821 | **+96 537** | **+0.0020 %** |
| `m0` | NROM | 5 520 389 483 | 5 520 470 984 | **+81 501** | **+0.0015 %** |
| `m4` | **MMC3** | 5 472 545 224 | **5 472 506 826** | −38 398 | −0.0007 % |
| `metroid` | MMC1 | 5 115 944 243 | **5 115 928 064** | −16 179 | −0.0003 % |
| `smb1` | NROM | 5 996 346 457 | 5 996 354 620 | +8 163 | +0.0001 % |
| `mm2` | MMC1 | 6 017 889 238 | **6 017 855 604** | −33 634 | −0.0006 % |
| `z2` | MMC1 | 6 291 431 663 | 6 291 489 092 | +57 429 | +0.0009 % |
| `readbuf` | CNROM | 6 834 547 836 | 6 834 572 417 | +24 581 | +0.0004 % |
| `duelito` | NROM | 7 380 393 405 | 7 380 421 007 | +27 602 | +0.0004 % |
| `smb3` | **MMC3** | 7 949 452 274 | 7 949 490 475 | +38 201 | +0.0005 % |
| `mmc5` | MMC5 | 11 286 830 608 | **11 279 363 868** | **−7 466 740** | **−0.066 %** |
| **total** | | **79 378 449 766** | **79 371 158 052** | **−7 291 714** | **−0.009 %** |

**The worst ROM of the thirteen is `m1` at +0.0020 %**, one tenth of the tolerance, and the set
total is **0.009 % below** the base commit. `m4` and `smb3` — the two the MF lots make the ones to
watch — are **−38 398 and +38 201**.

**`mmc5`'s −7 466 740 is one symbol and it is a debt repaid**: `Cartridge::default_cpu_read`
**−7 456 756**, which is *to the instruction* the **+7 456 756** MF-6b steps 2, 3, 4 and 5 all
recorded on that ROM as the price of `IrqBoard::prg_ram_enabled` becoming a forwarded body. Two
more bodies in that inner match — the Somari's `prg_ram_enabled` and `prg_ram_writable` — and
LLVM laid the function out the way it had before MF-6b step 3. That is the third time this lot
has found the same phenomenon (step 1's `push_expansion_audio` and step 3's nine ROMs are the
others): **a dispatch that a hot function inlines whole has no monotone cost in its arm count.**
The sixth board of `IrqBoard` cost that function 7.46 M and the eighth gave it back.

**Level 2**, `l2-diff.py --bar`:

| game/stage | `07325f6e` | **step 4** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `smb3/core` | 15 315 558 069 | 15 315 558 322 | **+253** | **+0.0000017 %** |
| `mm2/core` | 12 036 819 361 | 12 036 822 861 | **+3 500** | **+0.00003 %** |
| `smb1/core` | 11 515 125 238 | 11 515 128 803 | **+3 565** | **+0.00003 %** |
| `zelda2/core` | 12 775 641 366 | 12 775 651 048 | **+9 682** | **+0.00008 %** |
| `metroid/core` | 11 147 278 310 | 11 147 295 142 | **+16 832** | **+0.00015 %** |
| `mm2/total` | 18 976 300 081 | **18 976 279 805** | −20 276 | −0.0001 % |
| `metroid/total` | 13 735 514 951 | 13 735 583 997 | +69 046 | +0.0005 % |
| `zelda2/total` | 15 358 834 193 | 15 359 752 301 | +918 108 | +0.0060 % |
| `smb1/total` | 13 587 898 562 | 13 588 987 540 | +1 088 978 | +0.0080 % |
| `smb3/total` | 24 187 868 326 | 24 190 548 627 | +2 680 301 | **+0.0111 %** |

**Every figure of both columns is inside the tolerance outright**, with no noise argument needed
at all: the five `core` figures are **+253 to +16 832 instructions** — `smb3/core` is 253
instructions in 15.3 **billion** — and the worst `total` is **+0.011 %**, just over half the
+0.02 % allowed. Step 2's `smb3/total` +0.027 % did need the floor measurement; step 4's does
not, and the floor measurement is still the reason that column can be read at all.

For comparison, MF-6a2 step 3's stop-the-lot figures for 119, 47 and 37 alone were `mm2/core`
**+2 136 243** and `smb3/core` **+848 592**. This tree has those three boards, mapper 116 and
nine Cargo features, and its five `core` figures sum to **+33 832**.

### The post-LTO check

`pins63.py`, the three trees of this lot against the baseline:

| symbol | `07325f6e` | step 3 | **step 4** |
| --- | ---: | ---: | ---: |
| `Board::irq_asserted` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Wide::zone_offset` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::execute_without_operand` | 0 / 0 / 0 | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu::step` | 1 / 2 / 3 626 | 1 / 2 / 3 626 | 1 / 2 / **3 626** |
| `Board::ppu_peek` | 1 / 4 / 272 | 1 / 4 / 272 | 1 / 4 / **272** |
| `NesBus::write_internal` | 1 / 10 / 3 580 | 1 / 10 / 3 743 | 1 / 10 / 3 815 |
| `Ppu::run_dots` | 1 / 5 / 1 907 | 1 / 5 / 1 907 | 1 / 5 / **1 907** |
| `NesBus::catch_up_ppu` | 1 / 12 / 188 | 1 / 12 / 188 | 1 / 12 / **188** |

and `shape.py`: `Cpu::resolve` 2 / 6 / **1 948**, `Cpu::branch` 1 / 1 / **457**,
`NesBus::read_cycle` 1 / 65 / **181**, `tick` 1 / 95 / **82**, `read_internal` 1 / 17 / **175**.

**Seven of the eight pinned symbols, and all five of `shape.py`'s others, are identical to the
instruction from one end of the lot to the other** — four boards added and nine Cargo features
retrofitted, and `Cpu6502::step`, `resolve`, `branch`, `Board::ppu_peek`, `Ppu2C02::run_dots` and
`NesBus::catch_up_ppu` have not moved by one instruction. The three claimants of `step`'s
inlining budget are 0 / 0 / 0 and `zone_offset` has no symbol at all, which is the only reading of
"no dearer" that means anything.

The one that moved is `NesBus::write_internal`, **3 580 → 3 815** (+235) over the whole lot: the
register decodes of the TQROM, the two multicarts and the Somari, plus two `IrqBoard` forwards.
Its dynamic cost did not follow — `m4` −38 398 and `smb3` +38 201 are the two ROMs whose writes
go down that path most.

[`Somari`]: ../../rust/crates/zax-nes/src/cartridges/mappers/somari.rs

[PROGRESS] MF-6a3 step 4/6 done: mapper 116 on three held chips behind `board-116`, Q-130 question 1 answered and Q-132 written, 899 / 0 / 8, and the shipped tree inside the tolerance outright on both runs with `mmc5` −0.066 %.

## Step 5 — the suites, the `--ignored` pair, and the campaign row by row

### The suites, at the shipped tree

| check | result | against `07325f6e` |
| --- | --- | --- |
| `cargo fmt --check` | clean | same |
| clippy `--release -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean, exit 0 | **no new `allow` of any kind** |
| the same, `--features zax-nes/parked-boards` | clean | — |
| the same, `-p zax-nes --no-default-features` | clean | — |
| the same, each of the nine board features alone | clean, nine times | — |
| `cargo test --release -p zax-nes -p zax-harness` | **899 passed, 0 failed, 8 ignored** | 887 + 12 |
| the same, `--features parked-boards` | **899 / 0 / 8** | identical |
| `cargo test --release -p zax-nes --no-default-features` | **782 / 0 / 7** | — |
| `cargo test --release -p zax-harness --test test_roms_tests` | **14 passed** | identical |
| `--ignored`, both ROM folders, `--no-fail-fast` | **7 passed, 1 failed** | identical to MF-6b's and MF-6a2's |

**Eleven configurations, all green and all clippy-clean.** The twelve new tests are six for 119,
47 and 37 (step 1) and six for 116 (step 4); 782 + 48 + 6 = 836 on `zax-nes` alone, plus
`zax-harness`'s 63.

The `--ignored` environment is `ZAXNES_ROMS=G:/3dsen/demo/roms` and
`ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`, with `--no-fail-fast`, without which
`cargo test` stops after the first failing binary and most of the eight never run.

### The two the brief names, both green, and the caveat §3 requires

    test zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack ... ok
    goldens up to frame 1200: 1 of 291 rows matched, 0 pending: [], 290 ROMs missing
    test golden_pixels_tiles_audio_cycles_and_blargg_match_up_to_frame_1200 ... ok

Neither needed a change, and the second is reported with the caveat §3 of
`rust/docs/PERFORMANCE-HISTORY.md` now requires: **`rust/test-roms/` is empty in this worktree,
so golden-core counts 290 of its 291 rows as *missing* rather than failing and its green is green
over one ROM.** MF-6a2 step 4 measured what the real corpus gives (239 of 291, with 51 mismatches
that are the DMC-DMA area REQ-CORE-113 marks P3 and that two existing tests already pin as
accepted divergences) and showed the same set at its own baseline; this lot touches none of those
files and did not re-run that measurement, exactly as MF-6b step 8 did not.

**The one `--ignored` failure is `builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom`**,
the same one MF-6b, MF-6a2, MF-5b-fix and MF-5a all reported, and the proof is the same:
`git diff --name-only 07325f6e HEAD` lists **seventeen** files and `builtin_patches.rs` is not
among them.

### The campaign, field by field

    zax-harness test-roms G:/3dsen/cleanroom/assets-free/test-roms camp-6a3.json --no-shots --quiet
    test-roms: 331 ROMs, 214 pass, 5 fail, 0 timeout, 1 unsupported, 111 manual

`campfields.py camp-6b.json camp-6a3.json` — every key of every row, ignoring `ms` (wall clock)
and `shot` (`--no-shots`):

    base 331 rows, new 331 rows, same set: True
    summaries compare equal as objects: True
      base: {'total': 331, 'pass': 214, 'fail': 5, 'timeout': 0, 'unsupported': 1, 'manual': 111}
      new : {'total': 331, 'pass': 214, 'fail': 5, 'timeout': 0, 'unsupported': 1, 'manual': 111}

    rows differing on any field but ['ms', 'shot']: 0

    mapper numbers the campaign exercises: [0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180]
    of which this lot adds: []

**Zero rows differ on any field** and the summary objects compare equal on all six counts.

**The eighteen MMC3 rows are identical field by field**, named one by one because this lot changed
`Mmc3Board` — two new `const` parameters, three new window setters, four monomorphisations where
there were two: all six of `mmc3_irq_tests`, all six of `mmc3_test`, all six of
`mmc3_test_2/rom_singles`. That is the non-regression steps 1 and 4 both had to show.

**The three required-to-fail rows are still failing with their messages unchanged**:

| row | message |
| --- | --- |
| `mmc3_irq_tests/5.MMC3_rev_A` | "MMC3 IRQ COUNTER REVISION A / FAILED #3" |
| `mmc3_test/6-MMC6` | "IRQ shouldn't occur when reloading after counter normally reaches 0" |
| `mmc3_test_2/rom_singles/6-MMC3_alt` | "IRQ shouldn't be set when reloading to 0 due to counter naturally reaching 0" |

— the revision-A and MMC6 reload cases REQ-CORE-114 and AMD-061 §8 make accepted divergences. The
other two failures are `AccuracyCoin` (128/147, failing `$2004 Stress Test`) and
`blargg_ppu_tests_2005.09.15b/power_up_palette`, both as they were, and the one unsupported row is
`nes-test-roms/other/LINUSMUS.NES` ("Corrupt header: missing iNES signature"), also as it was.

### Why a second campaign with the features off would be the same run

The brief asks for the campaign "feature-off", and the configuration it names is the one measured
above: `parked-boards` is off by default and nothing is parked, so the default build **is** the
feature-off build.

For the nine board features the answer is a proof rather than a second run, and `campfields.py`'s
own last two lines are it:

    campaign numbers: [0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180]
    gated numbers   : [33, 37, 47, 70, 75, 76, 88, 89, 93, 94, 95, 97, 105, 116, 119,
                       140, 152, 154, 155, 158, 184, 185, 206, 232]
    intersection    : []

**The campaign exercises eighteen mapper numbers and not one of them is behind a feature**, so no
flip of any of the nine can move a row of it — which is also §2 of the inventory's own finding
read forwards: there is no image of any of these boards under `assets-free/test-roms`. What the
features *do* change in the registry is covered by
`req_core_100_every_required_mapper_is_registered`, which builds its expected list from the
`cfg!`s and runs in all eleven configurations.

[PROGRESS] MF-6a3 step 5/6 done: eleven configurations green and clippy-clean, 899 / 0 / 8, 14 / 14, the `--ignored` pair green with the golden-core caveat, and 331 campaign rows identical field by field with the eighteen MMC3 rows among them.
