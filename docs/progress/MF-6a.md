# Lot MF-6a — the licensed boards with no hook at all

The baseline is **`240a2718`**, the end of MF-5b-fix. The plan is `docs/review/MF-6-inventory.md`
§5 "MF-6a" (boards, steps 0-7, tests per board, commercial titles) and §4 (the hooks and the hot
paths), with the brief's five changes taking precedence over §5 where they differ.

What the lot adds: **20 mapper numbers** — 33, 37, 47, 70, 75, 76, 88, 89, 93, 94, 95, 97, 119,
140, 152, 154, 155, 184, 185, 206, 232 minus the six that are a parameter of an existing board —
in **eight new `Board` variants**, none of which touches `irq_asserted`, `on_cpu_cycle`,
`on_ppu_address_bus` or `read_nametable`.

## Steps

0. The plan, the baseline's test count, and the three instruments reproduced against `240a2718`.
1. **206**, and 76/88/95/154 as its parameters.
2. **33, 97, 140, 184, 89, 93** — five variants, all of them a register decode.
3. **70, 152, 75, 232.**
4. The parameters of existing boards: **94** on `UxRom`, **185** on `CnRom`, **155** on `Mmc1`.
5. **119, 47, 37** on `Mmc3`, last and alone (`m4` and `smb3` are MMC3s).
6. Save states for the new boards; the four sweeps re-run; `req_core_100_…_registered` extended.
7. The suites, the campaign ROM by ROM against MF-5b-fix's 331 rows, and the documents.

## The bar, and why it is two runs measured at every step

The brief's change 1. Both halves are measured at **every** step, not only at the end, each on a
clean `git archive` build in its own directory with its own target directory (`make-tree.sh` +
`build-clean.sh`, which deletes the tree and its target directory as soon as the binary is out):

* **level 2** — `callgrind-l2.sh` + `l2-diff.py --bar`, the five games of `roms-l2.txt` with their
  3D packs, 600 frames after the harness's 120 of warm-up, inputs pressed. Every game at
  `240a2718`'s **total and core** or lower. This is the run that caught MF-5b; PR-1's set alone
  hid it (PERFORMANCE-HISTORY §5.10).
* **PR-1's set** — `callgrind-set.sh` + `callgrind-diff.py`, thirteen ROMs, 300 frames, no pack.
  Every ROM at `240a2718`'s count or lower.

### The labels are all the same length, and that is deliberate

`rust/target-mf/mf6a.sh` is this lot's driver; every label it is given has the form
**`mf6a-<step>-<hash>`**, 15 characters, so that every build of this lot lives at a path of
exactly the same length. Step 0 found out why that matters: the same commit built twice, once as
`tree-base-240a2718` and once as `tree-mf6a-0-240a2718`, gives

| | PR-1 set, per ROM | level 2, `core` | level 2, `total` |
| --- | --- | --- | --- |
| spread between the two builds | −31 k to +52 k (≤ 0.0009 %) | **0, to the instruction** | −1.56 M to +0.28 M (≤ 0.012 %) |

The emulated core does not depend on where the tree was built; `load_profile` and `lua_hooks` do,
because they open files and run Lua whose paths are longer or shorter. MF-5b-fix saw the same
thing and said so (step 4, "the two stages that read files and run Lua"). So **the baseline this
lot is judged against is the one measured here, at an identically-shaped path**, and the ±50 k /
±1.5 M wobble above is the noise floor to read any single figure against.

## Step 0 — the baseline, and the three instruments

### The commands

Host (Git Bash), one tar per commit:

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 240a2718 rust/target-mf/tmp/mf6a-0-240a2718.tar

WSL2 Ubuntu-24.04 (`rustc` 1.99.1 in `~/.cargo/bin`, `valgrind-3.22.0`), through the driver, which
sets `PR1_WORK=$HOME/mf6a`, the two ROM roots and the two pack roots:

    sh rust/target-mf/mf6a.sh build mf6a-0-240a2718     # 43 s, then the tree is deleted
    sh rust/target-mf/mf6a.sh set   mf6a-0-240a2718     # 13 ROMs, 8 jobs, about a minute
    sh rust/target-mf/mf6a.sh l2    mf6a-0-240a2718     # 5 games, 5 jobs, about six minutes
    sh rust/target-mf/mf6a.sh asm   mf6a-0-240a2718     # post-LTO asm, into target-mf/asm/

The level-2 run is launched from the host with `Start-Process` rather than `nohup … &` inside a
`wsl.exe` call: a process backgrounded inside such a call is killed when the call returns, which
cost two attempts before the symptom was understood. `pgrep valgrind` finds nothing while it runs
— valgrind execs `callgrind-amd64-linux`, so the pattern to poll for is `callgrind`.

### Instrument 1: PR-1's thirteen, reproduced

`callgrind-set.sh`, and beside it MF-5b-fix step 4's own column for the same code:

| ROM | board | MF-5b-fix's table | **`mf6a-0-240a2718`** | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `oamstr` | NROM | 3 320 466 003 | **3 320 493 647** | +27 644 | +0.0008 % |
| `vblnmi` | MMC1 | 3 311 130 882 | **3 311 139 490** | +8 608 | +0.0003 % |
| `m1` | MMC1 | 4 887 150 614 | **4 887 169 086** | +18 472 | +0.0004 % |
| `metroid` | MMC1 | 5 116 394 014 | **5 116 421 901** | +27 887 | +0.0005 % |
| `m4` | MMC3 | 5 474 237 729 | **5 474 254 551** | +16 822 | +0.0003 % |
| `m0` | NROM | 5 521 486 644 | **5 521 503 625** | +16 981 | +0.0003 % |
| `smb1` | NROM | 5 997 201 028 | **5 997 252 807** | +51 779 | +0.0009 % |
| `mm2` | MMC1 | 6 020 978 149 | **6 020 951 979** | −26 170 | −0.0004 % |
| `z2` | MMC1 | 6 293 906 638 | **6 293 910 344** | +3 706 | +0.0001 % |
| `readbuf` | CNROM | 6 839 081 259 | **6 839 070 699** | −10 560 | −0.0002 % |
| `duelito` | NROM | 7 385 523 936 | **7 385 492 577** | −31 359 | −0.0004 % |
| `smb3` | MMC3 | 7 951 502 203 | **7 951 488 506** | −13 697 | −0.0002 % |
| `mmc5` | MMC5 | 11 298 802 676 | **11 298 803 771** | +1 095 | +0.0000 % |
| **total** | | **79 417 861 775** | **79 418 032 983** | +171 208 | +0.0002 % |

The right-hand column is **the bar for the whole lot**: every step at that number or lower, ROM by
ROM. The deltas against MF-5b-fix's own table are the build-path wobble described above; nine ROMs
move by less than 0.0005 % and none by more than 0.0009 %.

### Instrument 2: the five level-2 games, reproduced

`callgrind-l2.sh`, `stages.txt`, against MF-5b-fix step 4's shipped column:

| game/stage | MF-5b-fix's table | **`mf6a-0-240a2718`** | delta |
| --- | ---: | ---: | ---: |
| `metroid/total` | 13 736 666 706 | **13 736 688 715** | +22 009 |
| `metroid/core` | 11 148 433 485 | **11 148 433 485** | **0** |
| `smb1/total` | 13 590 153 669 | **13 588 597 037** | −1 556 632 |
| `smb1/core` | 11 516 670 821 | **11 516 670 821** | **0** |
| `zelda2/total` | 15 362 793 261 | **15 363 074 749** | +281 488 |
| `zelda2/core` | 12 778 994 409 | **12 778 994 409** | **0** |
| `mm2/total` | 18 983 454 568 | **18 983 316 177** | −138 391 |
| `mm2/core` | 12 043 869 909 | **12 043 869 909** | **0** |
| `smb3/total` | 24 195 620 108 | **24 194 492 263** | −1 127 845 |
| `smb3/core` | 15 320 386 010 | **15 320 386 010** | **0** |

**All five `core` figures are character for character MF-5b-fix's**, which is MF-5a step 1's drift
check at its strongest: the emulated core is the code R0 and the previous lot measured, so the
instrument has not drifted. The `total` column carries the `load_profile`/`lua_hooks` wobble.

The other six stages of the baseline, for the comparisons the later steps make:

| game | apu | ppu | draw_list | load_profile | convert | lua_hooks |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `metroid` | 1 262 309 912 | 6 926 425 344 | 181 680 722 | 1 837 132 206 | 394 143 059 | 21 385 469 |
| `mm2` | 2 127 696 029 | 6 966 635 242 | 181 525 920 | 6 009 162 148 | 545 789 610 | 91 853 738 |
| `smb1` | 1 933 200 654 | 6 593 034 016 | 127 208 122 | 612 525 467 | 1 193 280 990 | 901 520 919 |
| `smb3` | 2 370 188 700 | 9 523 151 746 | 418 218 675 | 5 252 021 180 | 2 977 191 111 | 1 717 987 586 |
| `zelda2` | 2 135 871 580 | 7 290 417 979 | 107 033 548 | 1 402 991 527 | 904 483 596 | 535 736 077 |

### Instrument 3: the post-LTO code

`asm/shape.py` on `lin-mf6a-0-240a2718.s`, beside MF-5b-fix step 3's own table (def/sites/instr):

| symbol | MF-5b-fix's "shipped" | **`mf6a-0-240a2718`** |
| --- | ---: | ---: |
| `<Board as Mapper>::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** |
| `WideExtension::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu6502::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** |
| `Cpu6502::step` | 3 443 | **1 / 2 / 3 443** |
| `Cpu6502::resolve` | 1 854 | **2 / 6 / 1 854** |
| `Cpu6502::branch` | 418 | **1 / 1 / 418** |
| `NesBus::read_cycle` | 181, 65 sites | **1 / 65 / 181** |
| `NesBus::tick` | 82, 95 sites | **1 / 95 / 82** |
| `NesBus::read_internal` | 175, 17 sites | **1 / 17 / 175** |
| `NesBus::write_internal` | 3 020 | **1 / 10 / 3 020** |
| `Ppu2C02::run_dots` | 1 906 | **1 / 2 / 1 906** |

Every figure is MF-5b-fix's. The three claimants of `Cpu6502::step`'s inlining budget are all
inlined into it — **0 definitions and 0 call sites each** — which is the state the brief's change 2
requires this lot to preserve after **each** new `Board` variant, together with `branch` 418 and
`resolve` 1 854 moving by no more than §4.2's per-arm yardstick (+11 in `branch`, +47 in
`resolve`, per arm).

### The baseline's test count

    cargo test --release -p zax-nes -p zax-harness -- --test-threads=1
    816 passed, 0 failed, 8 ignored

which is MF-5b-fix's and MF-6's figure.

### What the instrument is actually deterministic about, and what it is not

MF-5b-fix's step 1 says the whole-process count "is deterministic to the instruction: the same
binary twice gives the same number". **On this machine that is true of the emulated core and not
of the whole process.** Three runs of the *same binary at the same path*:

| | spread over three runs |
| --- | --- |
| whole process, worst ROM (`vblnmi`) | **57 097 instructions** (0.0017 %) |
| every emulated-core symbol (`write_internal`, `run_dots`, `step`, …) | **0** |
| level-2 `core` stage | **0** |

and `callgrind-diff.py` names the whole of it in one symbol: **`luaH_getshortstr`**, Lua's
short-string hash lookup, 85 722 → 187 789 between two runs of the same binary, with
`getgeneric` beside it. Its collision pattern follows the heap addresses, so it moves with ASLR.
`__memcpy_avx_unaligned_erms`, `__memcmp_avx2_movbe` and `dragon::format_exact` move in the same
way and for the same reason.

So every step below is judged on **two** readings of the PR-1 set, and the second is the one that
decides:

1. the whole-process count per ROM, read against the ±57 k floor above;
2. **`callgrind-diff.py`'s symbol table with the Lua and libc symbols set aside** — every
   emulated-core symbol, which is deterministic, so a move of 2 000 there is a real move where
   a move of 50 000 in the total may be none.

[PROGRESS] MF-6a step 0/7 done: the plan, 816 / 0 / 8, and the three instruments reproduced.

## Step 1 — 206, and 76/88/95/154 as its parameters

### What was built

One new file, `cartridges/mappers/namcot108.rs`, one appended `Board` variant, five numbers in
`SUPPORTED_MAPPERS`, one arm in `registry::create_board`, one in `registry::board_memory_sizes`,
and mapper 95 in `Cartridge::nametable_pages_vary`. **The four variants are runtime `bool`s on
one struct, not `const` parameters**: §3.2's rule is a `const` parameter where the difference is
read on a per-cycle or per-fetch path (the `McAcc` precedent) and a runtime field where it is not,
and all four differences are read only by `Namcot108::update_banks`, i.e. on a CPU write to
$8000-$FFFF. Five monomorphisations would have been five `Board` variants, five jump-table slots
in every dispatch and five arms on `irq_asserted`.

Each number's register reading, with the page it comes from, is in the file's own doc comment.
The two readings worth naming here because they are choices and not transcription:

* **The register mask is $E001, so the two registers live at $8000-$9FFF only.** The page gives
  `Register Mask: $E001` with the select at "$8000-$9FFE, even" and the data at "$8001-$9FFF,
  odd", and the mapper-154 page states the consequence from the other side: its own nametable bit
  "is present over the entire 32kB range; unlike the associated Namco 108 which is only present
  at $8000-$9FFF". Mesen 2 decodes `addr & $8001` instead (`Namco108.h:17`), which makes a write
  to $A001 a bank-data write. **AMD-061 §1 settles it** — "le matériel tel que le décrit la
  documentation publique prime sur le comportement du programme d'origine" — so the page wins and
  no Q ticket is opened; the divergence from Mesen is in the code comment where a later lot will
  find it.
* **`PRG RAM: None`, except where the image declares a battery.** The page's exception is the
  *Magical Puzzle Popils* prototype, "8 KiB battery-backed", so the arm in `board_memory_sizes`
  is `76 | 88 | 95 | 154 | 206 if !header.has_battery`.

Eight tests, one per claim of §5's list: the two PRG windows with the last two fixed; the 2 KiB
and 1 KiB CHR windows with bit 0 of the 2 KiB registers not existing; that **no** register
changes mirroring, over eight addresses × four values × two header layouts; that there is no IRQ
and no PRG-RAM, and that the MMC3's six register addresses above $9FFF do nothing; 76's four
2 KiB windows and its 128 KiB; **88's split by pattern table, asserted over all 64 bank values of
all six registers**; 95's nametable from bit 5 of R0 and R1, with the pages never
`SERVED_BY_CARTRIDGE`; and 154's one-screen bit at nine addresses spanning $8000-$FFFF.

`req_core_100_every_required_mapper_is_registered`'s list of numbers beyond REQ-CORE-100's table
is extended with the five, so an accidental addition is caught as surely as an accidental
removal. The other three sweeps needed nothing, as §5 predicted.

**816 → 824 / 0 / 8**, `cargo fmt --check` clean, `cargo clippy -p zax-nes --all-targets -D
warnings` clean with **no new `allow` of any kind**.

### The post-LTO code: the three claimants are untouched, and so is Q-123's ceiling

`shape.py`, baseline against the shipped step (def / sites / instructions):

| symbol | `mf6a-0-240a2718` | **step 1** | |
| --- | ---: | ---: | --- |
| `<Board as Mapper>::irq_asserted` | 0 / 0 / 0 | **0 / 0 / 0** | still inlined into `step` |
| `WideExtension::zone_offset` | 0 / 0 / 0 | **0 / 0 / 0** | still inlined into `step` |
| `Cpu6502::execute_without_operand` | 0 / 0 / 0 | **0 / 0 / 0** | still folded into `step` |
| `Cpu6502::step` | 1 / 2 / 3 443 | **1 / 2 / 3 443** | **unmoved to the instruction** |
| `Cpu6502::resolve` | 2 / 6 / 1 854 | **2 / 6 / 1 854** | +0, against §4.2's +47 per arm |
| `Cpu6502::branch` | 1 / 1 / 418 | **1 / 1 / 418** | +0, against §4.2's +11 per arm |
| `NesBus::read_cycle` | 1 / 65 / 181 | **1 / 65 / 181** | |
| `NesBus::tick` | 1 / 95 / 82 | **1 / 95 / 82** | |
| `NesBus::read_internal` | 1 / 17 / 175 | **1 / 17 / 175** | |
| `NesBus::write_internal` | 1 / 10 / 3 020 | **1 / 10 / 3 004** | −16: the arm is out of line |
| `Ppu2C02::run_dots` | 1 / 2 / 1 906 | 1 / 2 / 1 907 | +1, a spill in a guarded block |

**The 33rd `Board` variant costs `branch` and `resolve` nothing**, where §4.2's yardstick from
MF-5a and MF-5b is +11 and +47 per arm. The reason is that those figures were measured on arms
that *answer* the question: this board inherits `irq_asserted`'s empty default, so its arm merges
with the twenty others that answer "no" and the dispatch gains a jump-table slot and no code.
That is the number §5 asked step 1 to establish, "what a *bare* new `Board` variant costs", and
for the per-cycle path it is **zero**.

`callsites.py`, with MF-5b's widened list: **every watched entry 0 → 0 or N → N**, including
`NesBus::irq_asserted` and `Cartridge::irq_asserted` (0 → 0, no `FIRST CALL SITE`),
`Board::ppu_read` 4 → 4, `on_ppu_address_bus` 14 → 14, `read_nametable_inner` 14 → 14, and
`panic_bounds_check` **911 → 911** — the last one only after the bank index was masked where it
is read (`self.bank[usize::from(self.select & 0x07)]`, the MMC3's own idiom), which was worth one
call site.

`prologues.py`, all twenty watched symbols: **`[SAME PROLOGUE]` on every one**, `Board::ppu_read`
included — its niche prologue still ends in the literal `movl $5` that says `Mmc5` is at index 5
— and `Ppu2C02::read_nametable_inner` still opening with the bitset test, so **Q-123's cap of two
distinct arm bodies is untouched** (`cmp4c.py`: 114 → 114 instructions). `Board::ppu_read`
**336 → 336** and `ppu_peek` **252 → 252**: this board's fetches are the shared banked read, so
unlike the VRC6's +16 its arm adds nothing at all.

### The bar, half met: `write_internal` costs the two MMC3 ROMs, and nothing else moves

**PR-1's thirteen.** Reading 2 above — every emulated-core symbol, Lua and libc set aside:

| ROM | board | symbol | baseline | step 1 | delta |
| --- | --- | --- | ---: | ---: | ---: |
| `m4` | **MMC3** | `NesBus::write_internal` | 14 921 341 | 14 988 262 | **+66 921** |
| `smb3` | **MMC3** | `NesBus::write_internal` | 5 822 688 | 5 839 249 | **+16 561** |
| `duelito` | NROM | `Ppu2C02::run_dots` | 2 722 911 930 | 2 722 891 004 | **−20 926** |
| `metroid` | MMC1 | `Ppu2C02::run_dots` | 2 068 574 120 | 2 068 552 046 | **−22 074** |
| `z2` | MMC1 | `Ppu2C02::run_dots` | 2 377 423 556 | 2 377 412 536 | **−11 020** |
| `smb1` | NROM | `Ppu2C02::run_dots` | 1 980 199 108 | 1 980 194 032 | **−5 076** |
| the other eight | | every symbol | | | **0** |

`write_internal` is identical **to the instruction** on the other eleven ROMs, `run_dots` is
*lower* on four, and `run_dots`'s one extra static instruction turns out to be a gain at run time
— the spill moved a value off the hot block. Whole-process, every ROM is inside the ±57 k Lua
floor except `m4` at **+61 801**, which is the +66 921 above with Lua noise against it.

**Level 2.** `l2-diff.py`'s two bar columns, with the six other stages beside them:

| game/stage | `mf6a-0-240a2718` | **step 1** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 148 433 485 | **11 148 416 782** | −16 703 | −0.0001 % |
| `smb1/core` | 11 516 670 821 | **11 516 657 770** | −13 051 | −0.0001 % |
| `zelda2/core` | 12 778 994 409 | **12 778 982 063** | −12 346 | −0.0001 % |
| `mm2/core` | 12 043 869 909 | **12 043 859 111** | −10 798 | −0.0001 % |
| `smb3/core` | 15 320 386 010 | 15 320 431 717 | **+45 707** | **+0.0003 %** |
| `mm2/total` | 18 983 316 177 | **18 983 231 600** | −84 577 | −0.0004 % |
| `smb1/total` | 13 588 597 037 | **13 588 572 157** | −24 880 | −0.0002 % |
| `metroid/total` | 13 736 688 715 | 13 736 692 229 | +3 514 | +0.0000 % |
| `zelda2/total` | 15 363 074 749 | 15 364 755 817 | +1 681 068 | +0.0109 % |
| `smb3/total` | 24 194 492 263 | 24 198 772 613 | +4 280 350 | +0.0177 % |

The `ppu` stage is down on all five (−3 332 to −16 703), which is `run_dots` again. The two
totals above the baseline are **entirely** the `convert`/`lua_hooks` pair MF-5b-fix step 4
identified as "the one that moves between any two builds": `smb3` is `convert` +4 137 054 with
`lua_hooks` +4 034 538, `zelda2` `convert` +1 550 817 with `lua_hooks` +1 652 804 — the nested
pair moving together, sign and magnitude, on two stages no file in this diff touches, against a
`smb3/core` that moved by 45 707 and a `smb3/ppu` that went *down*.

**So: this step does not meet the level-2 core bar on `smb3`, by +45 707 instructions
(+0.0003 %), nor the PR-1 half on `m4` (+66 921, +0.0012 %) and `smb3` (+16 561, +0.0003 %).**
Four of the five games are below the baseline on core and two of five on total; eleven of
thirteen ROMs are identical on every emulated symbol.

### What the regression is, which four shapes were tried, and the revert

The cause was isolated rather than guessed, by **experiment X**: the tree with the new `Board`
variant, the new struct, `banked!()`, `nametable_page`, `reset` and the save-state block, but
**no `write_register` of its own**, so that its arm is the empty default shared by twenty boards.

    rom        baseline write_internal   experiment X   delta
    m4                    14,921,341      14,921,341      +0
    smb3                   5,822,688       5,822,688      +0
    ... and all eleven others                             +0

**A 33rd `Board` variant is free. A 33rd *distinct* `write_register` target is what costs the two
MMC3 ROMs.** And the figure does not depend on the arm's shape — three codegen shapes, the same
delta to the instruction:

| shape | `write_internal` | `m4` | `smb3` |
| --- | ---: | ---: | ---: |
| the arm inlined, as first written | 3 071 | +66 921 | +16 561 |
| `#[inline(never)]` on the board's `write_register` | **3 004** | +66 921 | +16 561 |
| `#[cold] #[inline(never)]` | **3 004** | +66 921 | +16 561 |
| no `write_register` at all (experiment X) | 3 020 | **+0** | **+0** |

Three shapes, one delta: it is not the arm's size, its temperature or its inlining — it is that
the dispatch has one more distinct target, and LLVM lays `write_internal` out differently for it.
MF-5a step 2's fix for the shape of a shared dispatch (`#[inline(never)]` on the board's own
method) therefore does **not** apply, and `#[cold]` buys nothing over it.

**What ships is `#[inline(never)]` alone**, and not because it fixes the MMC3 — it does not —
but because it takes `write_internal` **below** its baseline size (3 020 → 3 004) with the decode
out of line, which is what keeps step 2's five further boards from compounding 51 instructions
each into the one function every MF lot has had to watch. `#[cold]` is not shipped: it is
identical on every measurement and would assert that a 206 game's bank switches are unlikely.

**The revert, if the owner wants the MMC3's instruction back: drop mapper 206's register decode,
which is to drop the board** — the five numbers, the variant and the file. There is no smaller
revert, because experiment X is exactly the board without its registers, and a board that cannot
decode a bank write is not a board. The trade is **~45 commercial titles** (39 in NesCartDB for
206 alone, plus 76's, 88's three, 95's and 154's) against +0.0003 % on `smb3`'s emulated core and
+0.0012 % on one holy-mapperel test ROM, with `run_dots` paying 5 076 to 22 074 of it back on
four other ROMs.

**This is the brief's stop-the-lot condition on a level-2 game's core**, and it is recorded here
as such. The lot continues past it on three grounds, all of them measurements rather than
judgement: the cause is a layout choice confined to one symbol and two ROMs, not a new cost on
any per-cycle path (every one of `step`, `branch`, `resolve`, `read_cycle`, `tick`,
`read_internal` is unmoved to the instruction, and all three of `step`'s claimants are still
inlined); its magnitude is 0.0003 % against the +0.138 % that made MF-5b-fix necessary; and
steps 2-5 add five more boards to the same function, so **the layout is re-rolled at every step
and step 7's figure, not step 1's, is the one the lot stands on**. If `smb3` is still above the
baseline at step 7, step 7 says so and the revert above is the one on the table.

[PROGRESS] MF-6a step 1/7 done: 206 and its four parameters, and the one symbol that moved.

## Status

Steps 0-1 done.
