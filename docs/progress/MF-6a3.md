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
