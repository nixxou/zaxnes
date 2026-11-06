# Lot MF-6a2 — the three MMC3 numbers MF-6a left out, and its two level-2 residuals

The baseline is **`db804934`**, the end of MF-6a; the reference for the performance bar is
**`240a2718`**, the end of MF-5b-fix. Read first: `docs/progress/MF-6a.md` (its **step 5** is this
lot's design), `docs/review/MF-6-inventory.md` §5 and §4, `rust/docs/PERFORMANCE-HISTORY.md` §5.10
and §5.11.

## Steps

1. Both runs reproduced at `240a2718` and at `db804934`, on clean `git archive` builds, and the
   two residuals located at **block level** in the post-LTO dump.
2. The two residuals removed.
3. **119 (TQROM), 47 and 37** on `Mmc3`, with MF-6a step 5's design.
4. The suites, the campaign, and the documents.

## The labels are MF-6a's shape, deliberately

`rust/target-mf/mf62.sh` is this lot's driver — scratch, not committed, as `mf6a.sh` was, because
`rust/target-*` is never staged; the commands below are the whole of it. Every label it is given
has the form **`mf62-<step>-<8 chars>`**, 15 characters — the length of MF-6a's
`mf6a-<step>-<hash>`. MF-6a
step 0 measured why: the same commit built at two paths of different lengths differs by up to
1.56 M instructions on a level-2 `total`, all of it in `load_profile` and `lua_hooks`, which open
files and run Lua whose paths are longer or shorter. Keeping the length identical to MF-6a's makes
every figure below directly comparable with the tables in `docs/progress/MF-6a.md`.

## Step 1 — both runs reproduced, and the two residuals read off the post-LTO dump

### The commands

Host (Git Bash), one tar per commit:

    mkdir -p rust/target-mf/tmp
    sh rust/tools/perf/make-tree.sh 240a2718 rust/target-mf/tmp/mf62-0-240a2718.tar
    sh rust/tools/perf/make-tree.sh db804934 rust/target-mf/tmp/mf62-0-db804934.tar

WSL2 Ubuntu-24.04, through the driver, which is four lines of environment around
`rust/tools/perf/`'s own scripts — `PATH=$HOME/.cargo/bin:$PATH`, `PR1_WORK=$HOME/mf62` (so this
lot's scratch is its own), `ZAX_TEST_ROMS` and `ZAX_COMM_ROMS` for the set,
`ZAX_L2_ROMS=/mnt/g/3DSen/test-originals` and
`ZAX_L2_PACKS=/mnt/g/3DSen/pm-test/profiles-stage` for level 2:

    sh rust/target-mf/mf62.sh build mf62-0-240a2718      # 41 s, then the tree is deleted
    sh rust/target-mf/mf62.sh set   mf62-0-240a2718      # 13 ROMs, 8 jobs
    sh rust/target-mf/mf62.sh l2    mf62-0-240a2718      # 5 games, 5 jobs, about six minutes
    sh rust/target-mf/mf62.sh asm   mf62-0-240a2718      # post-LTO asm, into target-mf/asm/

and the same four for `mf62-0-db804934`. The level-2 runs are launched in the **foreground** with
a long timeout, never backgrounded: a headless session ends as soon as it stops answering, and a
process backgrounded inside a `wsl.exe` call is killed when the call returns (MF-6a step 0).

### The instrument has not drifted: the five level-2 `core` figures are MF-6a's, character for character

`l2-diff.py --bar`, `mf62-0-240a2718` against `mf62-0-db804934`:

| game/stage | `mf62-0-240a2718` | `mf62-0-db804934` | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 148 433 485 | **11 148 409 347** | −24 138 | −0.000 % |
| `smb1/core` | 11 516 670 821 | **11 516 657 770** | −13 051 | −0.000 % |
| `zelda2/core` | 12 778 994 409 | **12 778 987 397** | −7 012 | −0.000 % |
| `mm2/core` | 12 043 869 909 | 12 043 880 415 | **+10 506** | **+0.000 % ABOVE** |
| `smb3/core` | 15 320 386 010 | 15 320 847 312 | **+461 302** | **+0.003 % ABOVE** |
| `metroid/total` | 13 736 863 010 | **13 736 687 336** | −175 674 | −0.001 % |
| `mm2/total` | 18 983 269 814 | **18 983 246 065** | −23 749 | −0.000 % |
| `smb1/total` | 13 589 815 609 | **13 588 910 406** | −905 203 | −0.007 % |
| `zelda2/total` | 15 364 480 222 | 15 364 732 351 | +252 129 | +0.002 % ABOVE |
| `smb3/total` | 24 190 479 966 | 24 196 097 496 | +5 617 530 | +0.023 % ABOVE |

Every `240a2718` `core` figure is **character for character** MF-6a step 0's own baseline table
(11 148 433 485 / 11 516 670 821 / 12 778 994 409 / 12 043 869 909 / 15 320 386 010), and both
residuals come out at **exactly** MF-6a step 4's numbers — `smb3` **+461 302**, `mm2` **+10 506**.
Three measurement sessions, two scratch roots, two label prefixes, and the emulated core is the
same code to the instruction. That is MF-5a step 1's drift check at its strongest.

### PR-1's thirteen

`callgrind-diff.py`:

| ROM | `mf62-0-240a2718` | `mf62-0-db804934` | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 311 166 765 | **3 311 159 099** | −7 666 | −0.000 % |
| `oamstr` | 3 320 463 094 | **3 320 437 584** | −25 510 | −0.001 % |
| `m1` | 4 887 190 347 | **4 887 092 103** | −98 244 | −0.002 % |
| `m4` | 5 474 269 733 | **5 474 243 569** | −26 164 | −0.000 % |
| `m0` | 5 521 486 292 | **5 521 383 136** | −103 156 | −0.002 % |
| `metroid` | 5 116 414 925 | **5 116 376 181** | −38 744 | −0.001 % |
| `smb1` | 5 997 234 561 | 5 997 255 017 | +20 456 | +0.000 % |
| `mm2` | 6 020 968 359 | 6 020 988 417 | +20 058 | +0.000 % |
| `z2` | 6 293 956 281 | **6 293 921 449** | −34 832 | −0.001 % |
| `readbuf` | 6 839 079 589 | 6 839 083 368 | +3 779 | +0.000 % |
| `duelito` | 7 385 515 979 | **7 385 497 476** | −18 503 | −0.000 % |
| `smb3` | 7 951 492 113 | 7 951 654 719 | **+162 606** | **+0.002 %** |
| `mmc5` | 11 298 830 974 | 11 298 838 687 | +7 713 | +0.000 % |
| **total** | **79 418 069 012** | **79 417 930 805** | **−138 207** | −0.000 % |

Five ROMs are nominally above, and four of the five are **inside the ±57 k `luaH_getshortstr`
floor** §5.11 measured: at a 5 000-instruction threshold `mm2`, `mmc5`, `readbuf` and `smb1` show
no emulated symbol at all, only `getgeneric` and `luaH_getshortstr` trading places. `smb3` is the
one real move, and it is one symbol:

    +157,741    61,559,831 -> 61,717,572   <zax_nes::nes_bus::NesBus>::catch_up_ppu

`m4` has the same symbol at **+6 612** and nothing else, which is why `m4` ends −26 164 overall.

### Residual 1 — `smb3`: one `jmp` per MMC3 horizon, and the block that lost the fall-through

`shape.py` on the two dumps first, because it says where the regression is **not**: the three
claimants of `Cpu6502::step`'s inlining budget are **0 / 0 / 0 on both**, and `step` 3 443,
`resolve` 1 854, `branch` 418, `read_cycle` 1/65/181, `tick` 1/95/82 and `read_internal`
1/17/175 are **unmoved to the instruction**. Only `write_internal` 3 020 → 3 091 and `run_dots`
1 906 → 1 907 move at all, and neither is `catch_up_ppu`.

`fn.py` on `catch_up_ppu` gives **216 instructions in both dumps**. The function did not grow;
`blocks.py` says what happened instead. `NesBus::catch_up_ppu` inlines `NesBus::ppu_horizon`,
which inlines `Cartridge::ppu_irq_horizon` and with it the whole `Board::ppu_irq_horizon`
dispatch. Three of its arms are bodies: the MC-ACC's, the MMC3's and the RAMBO-1's — and the
MMC3's and the RAMBO-1's are **the same source expression**,

    let clocks_to_zero = if self.irq_counter == 0 || self.irq_reload {
        1 + i64::from(self.irq_latch)
    } else {
        i64::from(self.irq_counter)
    };
    Some((clocks_to_zero - 1) * self.a12.min_rise_interval())

character for character in `mmc3.rs` and `rambo1.rs`, so LLVM **tail-merges their last two
instructions** into one shared block. One of the two arms then falls through into it and the other
has to jump. The block order, `blocks.py` side by side:

| | `240a2718` | `db804934` |
| --- | --- | --- |
| | `_30` MC-ACC, `shlq $3`, `jmp _33` | `_30` MC-ACC, `shlq $3`, `jmp _33` |
| | `_26` RAMBO-1 … **`jmp _25`** | `_21` the `None` arm, `jmp _34` |
| | `_22` **MMC3** … *(falls through)* | `_22` **MMC3** … **`jmp _25`** |
| | `_25` `incq %rdx ; imulq %r8,%rdx ; jmp _33` | `_26` RAMBO-1 … *(falls through)* |
| | `_21` the `None` arm, `jmp _34` | `_25` `incq %rdx ; imulq %r8,%rdx ; jmp _33` |

**The MMC3 and the RAMBO-1 swapped the fall-through.** Every other instruction of
`catch_up_ppu` is identical; the only textual differences in the whole 216-instruction body are
the two `.Lanon` hash names of the division-by-zero panic strings. So the residual is **one taken
`jmp` per evaluation of an MMC3's horizon**, and the figures say so to the instruction:

| | calls implied | over |
| --- | ---: | --- |
| `smb3`, PR-1 set | +157 741 | 300 frames, no pack → 526 a frame |
| `smb3`, level 2 | +430 417 | 600 frames in play → 717 a frame |
| `m4`, PR-1 set | +6 612 | 300 frames |

and `smb3/core`'s +461 302 is that figure plus the rest of its symbol table, exactly:

    +430,417  catch_up_ppu   +55,040  write_internal   −20,823  chr_peek_view   −3,332  run_dots
    = +461,302

MF-6a's step 2-4 sections call this "an MMC3 PPU-catch-up path that LLVM lays out differently",
which is right; what this step adds is **which** block and **why there is only one to win** — a
join point two boards share because their source is identical.

### Residual 2 — `mm2`: not `catch_up_ppu` at all

MF-6a records both residuals as living "in `NesBus::catch_up_ppu`'s relayout", and for `mm2` the
symbol table does not support it: at a 2 000-instruction threshold `catch_up_ppu` does not appear
in `mm2`'s level-2 diff. What does:

| symbol | `240a2718` | `db804934` | delta |
| --- | ---: | ---: | ---: |
| `NesBus::write_internal` | 12 942 397 | 12 955 709 | **+13 312** |
| `Mmc1::update_banks` | 288 036 | 296 037 | **+8 001** |
| `Ppu2C02::run_dots` | 5 091 492 799 | **5 091 482 001** | −10 798 |

+13 312 + 8 001 − 10 798 = +10 515 against a measured +10 506, the rest being symbols under the
threshold. So `mm2`'s residual is **two** things and only one of them is a relayout:
`write_internal` 3 020 → 3 091 static instructions (MF-6a's seventeen register decodes, each
`#[inline(never)]`, re-rolling that function's layout), and **`Mmc1::update_banks` growing by a
field test** — MF-6a step 4's mapper-155 `bool`, which that step priced as free because
"`Mmc1::refresh_prg_ram_disable` runs when a register commits, not on the $6000-$7FFF decode".
It is free *per access*; it is not free *per commit*, and `update_banks` calls
`refresh_prg_ram_disable` unconditionally. `metroid` (+2 217) and `zelda2` (+4 311) show the same
symbol moving the same way; all three are MMC1 games.

`mm2` is the regime §5.10 named — an APU frame IRQ left asserted, so the cartridge half of
`NesBus::irq_asserted`'s `||` is never reached — which is why its figure is four hundredths of
`smb3`'s.

### What step 2 therefore has to do

* `smb3`: get the MMC3's arm back onto the fall-through, or remove the join point the two boards
  contend for.
* `mm2`: take the mapper-155 field test off `Mmc1::update_banks`, and leave `write_internal`'s
  layout to come out where step 3's own changes put it.

[PROGRESS] MF-6a2 step 1/4 done: both runs reproduced, +461 302 and +10 506 to the instruction, and the block that lost the fall-through.
