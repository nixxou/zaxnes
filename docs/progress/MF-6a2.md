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

## Step 2 — both residuals removed, and a dispatch body is worth 0.08 % to a board that has none

### What was built: three changes, none of them to a board's behaviour

**1. `Rambo1::ppu_irq_horizon` is `#[inline(never)]`.** Step 1 found the `smb3` residual to be one
taken `jmp` per MMC3 horizon, because the MMC3's and the RAMBO-1's arms are the same source
expression and LLVM tail-merges them into a block only one of the two can fall through to. Taking
the RAMBO-1's body out of the dispatch leaves the MMC3 the only claimant, and the merged block
disappears into its arm.

**2. `Mmc3Board::dots_per_clock` is a load-time field, not an `if MC_ACC`.** The second body in
the same dispatch was the MC-ACC's `shlq $3`: `dots_per_clock` was
`if MC_ACC { 8 } else { self.a12.min_rise_interval() }`, so the two monomorphisations had two
arms. Settled in `new`, they have one. Wiring and not state — `reset` leaves it, no savestate byte
carries it, and the value is the same 8 and the same `min_rise_interval()` as before, so **no
board's behaviour moves**. It also takes the `incq` out of the arm, because the field holds
`filter_dots + 1` already.

**3. `Mmc1`'s mapper-155 `bool` becomes two masks** — `prg_disable_mask` and `chr_disable_mask`,
`$10` or `$00` — which is the `ram_bank_mask` idiom already in that struct, and
`Mapper::mapper_number` reads 155 off a zero `prg_disable_mask` rather than off a second field.
`refresh_prg_ram_disable` is then branchless and the same length whatever the board.

### The four variants, and their numbers

The screen is the post-LTO dump, 45 s against a level-2 run's six minutes: `fn.py` on
`NesBus::catch_up_ppu` plus `blocks.py` to see which block holds the fall-through.

| variant | `catch_up_ppu` | the MMC3's arm | PR-1 set total | `smb3` | `m4` |
| --- | ---: | --- | ---: | ---: | ---: |
| `240a2718` | 216 | 13 instr, **falls through** to the merged tail | — | — | — |
| `db804934` | 216 | 14 instr, **`jmp` to the merged tail** | −138 207 | +162 606 | −26 164 |
| **V1** `#[inline(never)]` on the RAMBO-1's horizon | **203** | 13 instr, own tail | **−19 408 499** | −394 211 | −458 476 |
| **V2** V1 + `dots_per_clock` as a field | **188** | **12 instr**, one block, no `incq` | **−19 758 169** | **−704 591** | **−484 586** |
| **V3** V2 + `Mmc1`'s two masks (**shipped**) | **188** | as V2 | −19 586 552 | −688 704 | −477 391 |

V3 against V2 is −171 617 on the set total, which is **inside the ±57 k-per-ROM Lua floor summed
over thirteen ROMs**: no emulated symbol differs between the two on any ROM. V3 ships because
`Mmc1::update_banks` is **220 instructions at `240a2718`, 223 at `db804934` and 215 under the
masks** — five *below* the reference — and because `mm2`'s level-2 core is what that change is for.

### The finding: one body fewer in `Board::ppu_irq_horizon` is worth −0.005 % to −0.082 % to **every** board

This is the step's result and it was not the one expected. `NesBus::catch_up_ppu` inlines
`NesBus::ppu_horizon`, which inlines `Cartridge::ppu_irq_horizon` and with it the whole
`Board::ppu_irq_horizon` dispatch. Removing two of its four bodies — the RAMBO-1's to an
out-of-line call, the MC-ACC's into the MMC3's — took the function from **216 to 188
instructions**, and `catch_up_ppu` fell on **every one of PR-1's thirteen ROMs**, including boards
that have no horizon body at all and never had one:

| ROM | board | `catch_up_ppu` at `240a2718` | V3 | delta |
| --- | --- | ---: | ---: | ---: |
| `mmc5` | MMC5 | 1 314 266 808 | 1 305 052 429 | **−9 214 379** |
| `duelito` | **NROM** | 360 182 526 | 357 749 458 | **−2 433 068** |
| `m4` | MMC3 | 67 193 744 | 66 739 261 | −454 483 |
| `m1` | MMC1 | 64 953 655 | 64 515 198 | −438 457 |
| `mm2` | MMC1 | 35 271 973 | 35 035 079 | −236 894 |
| `metroid` | MMC1 | 35 172 269 | 34 935 881 | −236 388 |
| `m0` | **NROM** | 23 858 961 | 23 698 146 | −160 815 |

An NROM has no IRQ, no A12 watcher and no horizon — it takes the default arm — and it still gains
2.4 M instructions over 300 frames, because the dispatch's *shape* is what `catch_up_ppu` is laid
out around. That is §5.11's "count bodies, not variants" read in the other direction: MF-6a
measured what **adding** a body costs (0.13 % in `chr_view`, 0.67 % in `expansion_audio_output`);
this measures what **removing** one buys, on a dispatch no lot had looked at.

The price is named rather than hidden: **the RAMBO-1 (mappers 64 and 158, six titles) pays a
`leaq`/`callq` per horizon evaluation** where it had thirteen inline instructions. No ROM of either
instrument is a RAMBO-1, so the cost is not measurable here; it is a real cost, and it is MF-6a
step 2's rule in reverse — the gain lands on the whole library and the cost on one board.
`#[cold]` was not added beside it: a mapper-64 game consults that horizon on every PPU batch,
which is not unlikely.

### The bar: every level-2 game below `240a2718` on `core`, and the `total` column cannot be read at this resolution

`l2-diff.py --bar`, V3 against `mf62-0-240a2718`:

| game/stage | `240a2718` | **V3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 148 433 485 | **11 147 898 492** | −534 993 | −0.005 % |
| `smb1/core` | 11 516 670 821 | **11 515 980 923** | −689 898 | −0.006 % |
| `zelda2/core` | 12 778 994 409 | **12 777 709 570** | −1 284 839 | −0.010 % |
| `mm2/core` | 12 043 869 909 | **12 043 306 623** | **−563 286** | −0.005 % |
| `smb3/core` | 15 320 386 010 | **15 318 626 455** | **−1 759 555** | −0.011 % |
| `metroid/total` | 13 736 863 010 | **13 736 160 637** | −702 373 | −0.005 % |
| `mm2/total` | 18 983 269 814 | **18 982 779 228** | −490 586 | −0.003 % |
| `smb1/total` | 13 589 815 609 | **13 587 306 153** | −2 509 456 | −0.018 % |
| `zelda2/total` | 15 364 480 222 | **15 362 494 367** | −1 985 855 | −0.013 % |
| `smb3/total` | 24 190 479 966 | 24 190 909 323 | +429 357 | +0.002 % ABOVE |

**Both residuals are gone and no board was dropped**: `mm2/core` +10 506 → **−563 286**,
`smb3/core` +461 302 → **−1 759 555**. `smb3/apu` and `smb3/draw_list` are identical **to the
instruction**, `smb3/ppu` is −24 155, and the only emulated symbol above 100 000 on `smb3` is
`catch_up_ppu` at **−1 790 440**.

The one figure above is `smb3/total` +429 357, and it is `convert` +2 136 668 with `lua_hooks`
+1 956 135 — the nested pair every MF lot since MF-5b-fix has had to set aside — named in one
symbol: **`getgeneric` +2 247 112**, Lua's table lookup, with `luaH_set` −457 371 and
`luaH_getshortstr` +164 992 beside it. So this step measured that floor instead of asserting it.
**The same binary, at the same path, through `callgrind-l2.sh` twice:**

| | spread over the two runs |
| --- | --- |
| every level-2 `core` stage | **0, to the instruction, on all five games** |
| `apu`, `draw_list`, `ppu` | **0** |
| `smb3/total` | **+2 739 840** |
| `smb1/total` | **+2 659 382** |
| `zelda2/total` | +867 108 |
| `metroid/total` | +31 824 |
| `mm2/total` | −1 084 |

`smb3/total`'s +429 357 is **one sixth of the same binary's own run-to-run spread on that
column**: the second run put `smb3/total` at +3 169 197 and `smb1/total` at +149 926 — a column
that had been −2 509 456 — while every `core` stage stayed identical to the instruction. §5.11
measured this floor at ±57 k on the PR-1 set's whole-process count; on a level-2 `total` it is
**±2.7 M**, forty-eight times larger, because a level-2 run loads a 3D pack and runs Lua frame
hooks and the PR-1 run does neither. The `core` column is the one that can be read at this
resolution, and **all five games are below `240a2718` on it, by 0.005 % to 0.011 %**.

### PR-1's thirteen: all thirteen below, set total −19 586 552

| ROM | `240a2718` | **V3** | delta | |
| --- | ---: | ---: | ---: | ---: |
| `vblnmi` | 3 311 166 765 | **3 310 526 412** | −640 353 | −0.019 % |
| `oamstr` | 3 320 463 094 | **3 319 502 016** | −961 078 | −0.029 % |
| `m1` | 4 887 190 347 | **4 886 656 025** | −534 322 | −0.011 % |
| `m4` | 5 474 269 733 | **5 473 792 342** | −477 391 | −0.009 % |
| `m0` | 5 521 486 292 | **5 521 263 906** | −222 386 | −0.004 % |
| `metroid` | 5 116 414 925 | **5 116 127 287** | −287 638 | −0.006 % |
| `smb1` | 5 997 234 561 | **5 996 876 627** | −357 934 | −0.006 % |
| `mm2` | 6 020 968 359 | **6 020 784 916** | −183 443 | −0.003 % |
| `z2` | 6 293 956 281 | **6 292 636 422** | −1 319 859 | −0.021 % |
| `readbuf` | 6 839 079 589 | **6 836 835 668** | −2 243 921 | −0.033 % |
| `duelito` | 7 385 515 979 | **7 383 061 072** | −2 454 907 | −0.033 % |
| `smb3` | 7 951 492 113 | **7 950 803 409** | −688 704 | −0.009 % |
| `mmc5` | 11 298 830 974 | **11 289 616 358** | −9 214 616 | −0.082 % |
| **total** | **79 418 069 012** | **79 398 482 460** | **−19 586 552** | **−0.025 %** |

`m4` and `smb3`, the two ROMs the brief makes a stop-the-lot condition, are **−477 391** and
**−688 704**. `NesBus::write_internal` is −65 503 on `m0` and −64 164 on `m1` and unmoved
elsewhere; `Ppu2C02::run_dots` is −20 926 on `duelito` and −22 074 on `metroid`.

### Suites

`847 passed, 0 failed, 8 ignored`, which is MF-6a's figure: no test was added or edited at this
step and none changed its answer. `cargo fmt --check` clean, `cargo clippy --release -p zax-nes
--all-targets -- -D warnings` clean, **no new `allow` of any kind**. No save-state byte moved —
`Mmc3Board::save_registers` and `Mmc1::save_registers` are untouched, and both new fields are
wiring recomputed at load.

[PROGRESS] MF-6a2 step 2/4 done: both residuals gone, all five cores and all thirteen ROMs below the reference, −19 586 552.

## Step 3 — 119, 47 and 37: built, tested, measured six ways, and **reverted**

This step was **built in full and then reverted**, on the brief's own stop-the-lot condition.
The three boards worked: `853 passed, 0 failed, 8 ignored` — MF-6a's 847 plus six new tests —
with `cargo fmt --check` and `cargo clippy -p zax-nes --all-targets -- -D warnings` clean and no
new `allow`. What they cost is **+0.004 % to +0.017 % on six of PR-1's thirteen ROMs**, `m4` and
`smb3` among them, and **`mm2/core` +2 136 243 (+0.018 %)** and **`smb3/core` +848 592
(+0.006 %)** at level 2. The brief makes either of those the end of the step, "after you have
tried to fix it": six shapes were measured, the figure did not go away, so the boards are out and
the measurements are here.

The revert is `git checkout f6938486 -- rust/crates/zax-nes/src/cartridges/
rust/crates/zax-nes/tests/mapper_tests.rs`, and the tree ends this step at MF-6a2 step 2's code:
**847 / 0 / 8**, no mapper number added.

### What was built, so the next lot starts from the design and not from the page

MF-6a step 5's design, with one departure measured below. `Mmc3Board` gained `const` parameters;
`Mmc3` and `McAcc` kept their monomorphisations, so **mapper 4 and 118 came out of the compiler
instruction for instruction** — `shape.py` says so at every variant: the three claimants of
`Cpu6502::step`'s budget **0 / 0 / 0**, `step` 3 443, `resolve` 1 854, `read_cycle` 1/65/181,
`tick` 1/95/82, `read_internal` 1/17/175 **unmoved to the instruction**, `branch` 418 → 418 or
420, `run_dots` 1 907.

**Mapper 119 (TQROM).** nesdev ("INES Mapper 119"): "Bit 6 of the CHR bank number is used to
switch between the CHR ROM chip and the CHR RAM chip", the board doing it with "a 74HC32 IC"
that "controls the CHR ROM's enable line. PPU A13 is ORed with CHR A16 in order to create a /CE
signal for the CHR ROM", and "Both TQROM games used 8 KiB CHR RAM". Four consequences, each a
decision:

* the chip select is a line of the **bank number**, so a 2 KiB register — which "ignores bit 0" —
  puts **both** of its 1 KiB windows on the same chip; the derivation records the eight windows'
  source bytes in a `[u8; 8]` refreshed when a register commits, for the reason
  `Mmc3Board::nametable_pages` already gives (`Mapper::ppu_peek` is per fetch and must not
  re-derive the CHR mode there);
* a window left on the ROM needs **no new arithmetic at all** — `BankWindows` already holds the
  raw register wrapped modulo the CHR-ROM's banks, which is that window's offset — so only the
  RAM half is new, three low bits over the 8 KiB chip;
* **`chr_peek_view` answers `None`**, because a `ChrView` is one slice plus eight offsets and
  cannot say "this window is the other chip". That is the body the MMC5 and the RAMBO-1 already
  give, so it adds nothing to that dispatch (MF-6a step 2's finding, reused);
* the 8 KiB of CHR-RAM is the **board's** and goes in `registry::board_memory_sizes`, because no
  header field can state it: byte 5 of an iNES 1.0 header is the CHR-*ROM* size, and
  REQ-CORE-012's CHR-RAM default only applies to an image with no CHR-ROM. Mesen 2 agrees
  (`MapperFactory.cpp:404`, `MMC3_ChrRam(0x40, 0x7F, 8)` — chip-select bit $40, 8 KiB).

**Mapper 47.** nesdev ("INES Mapper 047"): `$6000-7FFF` is `[.... ...B]`, "each block has 128k
PRG and 128k CHR", `$8000-FFFF` "Same as MMC3 for selected block". So `B` is PRG A17 above the
MMC3's four bits of 8 KiB bank and CHR A17 above its seven bits of 1 KiB bank, the fixed `-1`
and `-2` windows becoming the **block's** last two rather than the image's. Mesen 2 reads it
identically (`MMC3_47.h`).

**Mapper 37, and the one place the page and Mesen 2 disagree.** The page latches `xxxx xQBB` and
gives the wiring rather than a mask: "PRG ROM A16 = NAND(NAND(Q0,Q1),NAND(Q2,M16))" with "PRG ROM
A17 … driven by Q2", `M16` being the MMC3's own A16. That NAND of two NANDs is

    A16 = (Q0 AND Q1) OR (Q2 AND M16),   A17 = Q2

and **one expression reproduces the page's whole four-row table**, the two 64 KiB windows and the
two 128 KiB ones:

| written | A17 | A16 | PRG window | the page's "PRG bank size" |
| ---: | --- | --- | --- | --- |
| 0, 1, 2 | 0 | 0 | $00000-$0FFFF | 64 KiB |
| 3 | 0 | **1**, forced | $10000-$1FFFF | 64 KiB |
| 4, 5, 6 | 1 | the MMC3's | $20000-$3FFFF | 128 KiB |
| 7 | 1 | **1**, forced | $30000-$3FFFF | 64 KiB |

**Mesen 2 differs on the last row and AMD-061 §1 settles it for the page**: `MMC3_37.h` gives
block 7 `page &= 0x07; page |= 0x20`, which sets A18 and leaves A16 clear — $20000-$27FFF —
where the page's table says $30000-$3FFFF and its own formula says A16 = 1 because `Q0 AND Q1` is
set. Mesen also masks CHR with nothing on 37 (`if block >= 4 page |= 0x80`) where the page has
A17 *driven* by Q2, i.e. the MMC3's bit 7 never reaching the CHR-ROM. Both divergences were
implemented the page's way and recorded in the code.

**The condition both multicart pages put on the register**, and §5 of the inventory's "easy thing
to miss": it is "only writable when MMC3 PRG-RAM is enabled and writable (see $A001)" (047);
"you will need to enable writes to PRG-RAM" (037). That is `Mapper::prg_ram_writable` — $A001
bit 7 set and bit 6 clear — and the range is consumed either way, because the register *is* the
board's decode of $6000-$7FFF. 37's page adds that it "is not readable and attempting to read it
will return open bus", which falls out of giving both numbers `prg_ram: Some(0)`: with no RAM
behind the window, `Cartridge::default_cpu_read` falls through to `try_read_prg` and that
answers `None` below $8000.

**Both A12 latches need the three numbers** — `watches_ppu_reads` and
`watches_ppu_bus_without_read`, mappers `4 | 5 | 37 | 47 | 64 | 118 | 119` and
`4 | 37 | 47 | 64 | 118 | 119` — and this is **not** optional bookkeeping:
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` failed with
`left: [4, 4, 5, 5, 37, 37, 47, 47, 64, 64, 118, 118, 119, 119]` against its own expected list
until both were extended. §4.3's warning, caught by the test that exists for it.

**One reading of the submappers had to be tightened.** `is_mmc6`, `irq_revision_a` and
`ignore_a000`'s submapper-2 clause were `!is_txsrom && header.is_nes20 && submapper == N`, which
is mapper 4's condition expressed as "not 118". nesdev's *NES 2.0 submappers* defines submappers
1-4 for mapper 4 and **none** for 118, 119, 47 or 37, so a NES 2.0 dump of a TQROM carrying a
submapper byte would have been read as an MMC6 or an MMC3A. The three became
`mapper_number == 4 && …`, which is identical for mapper 4 and 118.

**Save states**: one byte appended behind the existing versioning, behind the `const` that is the
only reason the field exists, so no byte of mapper 4's, 118's or the MC-ACC's block moves
(REQ-CORE-090). `chr_source` is **derived and not serialised** — MF-5a step 3's `level` lesson —
and `load_registers` brings it back through `update_banks`.

**Six tests**, 847 → **853**, one per claim of §5's list, all passing:

| test | what it pins |
| --- | --- |
| `req_core_100_tqrom_bit_6_of_a_chr_bank_selects_the_chr_ram_chip` | bit 6 in **both** CHR modes, on a 1 KiB register and on both halves of a 2 KiB one, and `chr_peek_view` answering `None` |
| `req_core_100_tqrom_a_write_to_a_ram_selected_window_is_not_swallowed` | §5's own words: the write lands in RAM, is dropped on a ROM window, is visible through a second window on the same RAM bank and not through another, and the CHR-ROM is untouched |
| `req_core_100_mapper_47_switches_128_kib_blocks_of_prg_and_chr` | the two blocks, PRG and CHR, the fixed windows being the block's last two, a bank number wrapping inside the block, and CHR A17 coming from the outer register |
| `req_core_100_mapper_37_reproduces_its_outer_bank_table` | all eight values × the MMC3's A16 clear and set, i.e. the whole table above, plus bits 3-7 not being latched |
| `req_core_100_the_mmc3_multicart_outer_register_needs_prg_ram_enabled_and_writable` | $A001 = $80 / $C0 / $00 / $80 on **both** numbers, and the window reading open bus |
| `req_core_100_an_ordinary_mapper_4_image_is_unaffected_by_119_47_and_37` | a mapper-4 write to $6000 moving no bank and keeping its PRG-RAM, CHR bank $A0 being bank $A0 and not bank $20 of a block, the four windows spanning the whole image, and mapper 4 still offering a `chr_peek_view` |

`req_core_100_every_required_mapper_is_registered`'s list of numbers beyond REQ-CORE-100's table
was extended with 37, 47 and 119, and the other four sweeps needed nothing once the two latches
were right.

### The departure from the design, and the measurement that forced it

MF-6a step 5 specified **two** `const` parameters, `Tqrom` and `Mmc3Multicart`, hence two new
`Board` variants. That was built first and then measured against one parameter for all three
numbers, because step 2's finding says to count bodies: two monomorphisations give the three
numbers two distinct `write_register` bodies, two `ppu_peek` arms and two `intercept_cpu_write`
arms where one gives them one each. The numbers are in the table below, and they are **not what
the reasoning predicted**: one variant is *worse* on `m4` (+750 676 against +258 640) and on
`NesBus::write_internal` (3 467 against 3 252). Neither shape gets under the bar, and the
difference between them is inside the spread of the six.

### The six shapes, and what each one bought

Every row is a clean `git archive`-equivalent build of the working tree at a 15-character label,
its own directory and its own target directory. `ppu_peek` and `write_internal` are post-LTO
instruction counts; the deltas are against `mf62-0-240a2718`.

| shape | `Board::ppu_peek` | `write_internal` | set total | `m4` | `smb3` |
| --- | ---: | ---: | ---: | ---: | ---: |
| step 2, for reference | 271 | 3 103 | **−19 586 552** | **−477 391** | **−688 704** |
| **A** two `const`s, everything inlined | **583** | 3 233 | −10 600 259 | +3 404 126 | −4 073 914 |
| **B** A + `ppu_peek`/`ppu_write`'s TQROM arms `#[inline(never)]` | **272** | 3 250 | **−18 616 060** | **+240 756** | +391 736 |
| **C** B + one shared `update_banks` body + the outer decode out of line | 272 | 3 252 | **−18 626 213** | +258 640 | **+390 724** |
| **D** one `const`, **one** `Board` variant | 272 | 3 467 | −18 044 083 | +750 676 | +432 335 |
| **E** D + the whole $8000-$FFFF decode out of line | 272 | 3 392 | −18 226 255 | +759 530 | +424 831 |
| **Z** E with **no `ppu_write` override at all** (experiment) | 272 | — | −18 738 695 | +338 827 | +409 723 |

**Shape A → B is the one real finding, and it is step 2's in reverse.** `Board::ppu_peek`'s arms
are inlined into `Ppu2C02::background_fetch`, 7 % to 11 % of every ROM of the set; the TQROM's
chip-select body took that function from **271 instructions to 583** and cost `m4` **3.4 M over
300 frames**, `metroid` 3.5 M and `mm2` 5.4 M — three games on boards the step does not touch.
`#[inline(never)]` on it alone recovered **8.0 M of set total**. That is the third hook on which
"the trigger is a distinct body, not its size" has now been measured: `read_nametable` (Q-123),
`chr_view` (MF-6a step 2), `ppu_irq_horizon` (step 2 above) and now `ppu_peek`.

**Experiment Z is the row that ends the step.** It removes the `ppu_write` override entirely —
the TQROM's CHR-RAM writes broken, so it is an experiment and not a candidate — on the theory
that `Board::ppu_write`'s arms are inlined into `NesBus::write_internal` (they are: a `$2007`
write arrives through it) and that mapper 4 losing its place in the shared default group was the
cost. It is not: **the same six ROMs are above by the same amounts**, `m4` +338 827. Taken with
shapes B-E, which span `write_internal` 3 250 to 3 467 and `m4` +240 756 to +759 530 with no
consistent relation between them, the residual is **not attributable to any one body**. It is the
whole-program relayout every MF lot since MF-5a has had to report, here working against the step
instead of for it — the mirror image of step 2, where two bodies *removed* bought −0.082 % on a
board that has none.

### The bar, on both instruments

**PR-1's thirteen**, the best shape (C):

| ROM | board | `240a2718` | shape C | delta | |
| --- | --- | ---: | ---: | ---: | ---: |
| `mmc5` | MMC5 | 11 298 830 974 | **11 282 830 673** | −16 000 301 | −0.142 % |
| `duelito` | NROM | 7 385 515 979 | **7 384 231 152** | −1 284 827 | −0.017 % |
| `readbuf` | CNROM | 6 839 079 589 | **6 837 775 128** | −1 304 461 | −0.019 % |
| `oamstr` | NROM | 3 320 463 094 | **3 319 424 431** | −1 038 663 | −0.031 % |
| `metroid` | MMC1 | 5 116 414 925 | **5 115 419 239** | −995 686 | −0.019 % |
| `z2` | MMC1 | 6 293 956 281 | **6 293 322 965** | −633 316 | −0.010 % |
| `smb1` | NROM | 5 997 234 561 | **5 996 902 586** | −331 975 | −0.006 % |
| `vblnmi` | MMC1 | 3 311 166 765 | 3 311 372 696 | **+205 931** | +0.006 % |
| `m4` | **MMC3** | 5 474 269 733 | 5 474 528 373 | **+258 640** | **+0.005 %** |
| `m1` | MMC1 | 4 887 190 347 | 4 887 519 180 | +328 833 | +0.007 % |
| `smb3` | **MMC3** | 7 951 492 113 | 7 951 882 837 | **+390 724** | **+0.005 %** |
| `m0` | NROM | 5 521 486 292 | 5 522 272 652 | +786 360 | +0.014 % |
| `mm2` | MMC1 | 6 020 968 359 | 6 021 960 887 | +992 528 | +0.016 % |
| **total** | | **79 418 069 012** | **79 399 442 799** | **−18 626 213** | **−0.023 %** |

The set total is still **0.023 % below the reference** and seven of thirteen are below it, but
`m4` and `smb3` are above and the ROMs furthest above — `m0` an NROM, `mm2` and `m1` MMC1s — are
games with no connection to a mapper this step adds.

**Level 2**, shape E, `l2-diff.py --bar` against `mf62-0-240a2718`:

| game/stage | `240a2718` | shape E | delta | |
| --- | ---: | ---: | ---: | ---: |
| `metroid/core` | 11 148 433 485 | **11 146 275 120** | −2 158 365 | −0.019 % |
| `zelda2/core` | 12 778 994 409 | **12 777 919 847** | −1 074 562 | −0.008 % |
| `smb1/core` | 11 516 670 821 | **11 515 902 830** | −767 991 | −0.007 % |
| `smb3/core` | 15 320 386 010 | 15 321 234 602 | **+848 592** | **+0.006 %** |
| `mm2/core` | 12 043 869 909 | 12 046 006 152 | **+2 136 243** | **+0.018 %** |
| `metroid/total` | 13 736 863 010 | **13 734 502 436** | −2 360 574 | −0.017 % |
| `zelda2/total` | 15 364 480 222 | **15 361 395 480** | −3 084 742 | −0.020 % |
| `smb1/total` | 13 589 815 609 | **13 588 100 595** | −1 715 014 | −0.013 % |
| `mm2/total` | 18 983 269 814 | 18 985 483 040 | +2 213 226 | +0.012 % |
| `smb3/total` | 24 190 479 966 | 24 195 736 066 | +5 256 100 | +0.022 % |

**This is the figure that decides.** `mm2/core` is **+2 136 243**, which is step 2's residual
(+10 506, and taken to −563 286) back **two hundred times larger**, and it is in the `core`
column — the one that is exactly reproducible, 0 spread over two runs of the same binary at the
same path (step 2's table). It cannot be read as instrument noise the way a `total` can.

### Why the instrument had to be repaired, and how

**Valgrind 3.22 stops reading this binary's Rust symbols at step 3 and reports no error.** Every
tree of lots PR-1 through MF-6a2 step 2 annotated with names; every step-3 tree annotates
`???:0x...`, although `nm` lists **4 522 text symbols** and `file` says "with debug_info, not
stripped". `--read-inline-info=no` and `--read-var-info=no` change nothing, there is no warning
in valgrind's own output, and three separate builds of the same tree behave identically — so it
is a property of the binary and not of a run.

The counts are unaffected: three builds of the step-3 tree agree on the set total to within
186 220 instructions over 79 G (0.0002 %), inside the thirteen-ROM Lua floor. So the
whole-process column was read as measured and `rust/target-mf/resolvesyms.py` was written to put
the names back from `nm`, deriving the constant bias valgrind's addresses carry
(**`0x108000`** here) by tallying which candidate puts the most addresses exactly on a symbol
start — 721 of 769. The translated table is **coarse** and was not used for a verdict: valgrind
merges a tail-called function into its caller, so `Cpu6502::branch` reads 0 and `Cpu6502::step`
+197 M where `shape.py` shows `branch` with its own definition and its own call site in both
dumps. The post-LTO dump was therefore the symbol-level instrument for this step, and the
whole-process count the bar.

### What a later lot should do differently

1. **`Board::ppu_peek` is the fourth hook with a measured body cap**, and the first one a
   board-adding lot hits without warning: §4.4 of the inventory prices an arm there at **+16**
   from the VRC6, and this step measured one at **+312 static instructions and 3.4 M on `m4`**.
   The entry belongs in §4.4 beside the VRC6's, and the fix is `#[inline(never)]` on the board's
   own body, which costs that board a `callq` per fetch and the rest of the library nothing.
2. **The three numbers are not worth 0.018 % on Mega Man 2's emulated core**, which is what the
   revert says. The trade is **five titles** — *High Speed*, *Pin\*Bot*, *Super Spike V'Ball +
   Nintendo World Cup*, *Super Mario Bros. + Tetris + Nintendo World Cup* and one more — against
   a measurable regression on two of R0's five level-2 games.
3. **The shape to try next is the one shape not tried here**: not a parameter of `Mmc3Board` at
   all, but a board of its own, so that no dispatch arm mapper 4 shares is re-laid-out. MF-6a
   step 4 made exactly that departure for mapper 185 — "a disabled CHR changes what a fetch
   reads, so the parameter would have to reach `Mapper::chr_view`" — and put it in `SmallBoard`,
   whose `chr_view` answers `None` for the whole variant. A `Tqrom` struct with its own
   `banked!()`, its own `write_register` and `SmallBoard`'s `None` answers would copy the MMC3's
   twenty lines of banking and its A12 counter, which is duplication the `const` parameter exists
   to avoid — but it is the only shape that cannot move mapper 4's code, because it shares none
   of it. The 47/37 half needs no copy at all beyond the outer register.

[PROGRESS] MF-6a2 step 3/4 **not done**: 119, 47 and 37 built, tested and measured six ways, then reverted on the stop-the-lot condition.
