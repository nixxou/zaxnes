# Lot MF-4d-fix — back out the `$2007`-while-rendering half of MF-4d step 6

Baseline: `fff53974` (ZR-2's end). Reference picture: a clean build of `c4577878` (MF-4a 5/5).

MF-4d step 6 (`a758f923`) made a `$2007` access during rendering move `v` by a coarse X increment
and a Y increment at once. The rule is right on hardware, but applied **blind to the dot** the
access lands on it costs more than it buys:

* **AMD-021 R2-005 criterion 2 fails** — `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`
  emits a background row on Super Mario Bros. 3's split line on 4 986 of 5 000 frames, with no hack
  code at all. It passed at `d96d73b7`. R2-005 is **P1**; one `AccuracyCoin` row is not.
* The golden-core pictures of Super Mario Bros. 3 and Zelda II move.
* The widescreen hack of Super Mario Bros. 3 is visibly broken at frame 1 500.

R0's decision: back out **only** that half, which MF-4d had isolated to
`Ppu2C02::increment_vram_address`. The `$2004` half of step 6 moves no golden-core ROM and stays.
Lot **MF-4e** owns the per-dot work and takes the rule back with R2-005 as a required test
(**Q-125**).

## Steps

1. Back out the `$2007` half; Q-125; the unit test inverted.
2. The ignored commercial-ROM tests, with the ROMs present.
3. The `AccuracyCoin` guard's floor, 129 → 128.
4. The pictures: the owner's fortress state, then golden-core against the `d96d73b7` side.
5. The suites and the campaign, ROM by ROM against ZR-2's.

## Step 1 — the `$2007` half is out, and the code is `d96d73b7`'s again

### What changed

`Ppu2C02::increment_vram_address` loses its `fetching()` branch and its `#[inline]`, and is the
body of `d96d73b7` **line for line**:

    let step = if (self.control & 0x04) != 0 { 32 } else { 1 };
    self.v = self.v.wrapping_add(step) & 0x7FFF;

The comment block above it is new and is the deferral: the hardware rule, its three sources
(nesdev "PPU registers" on PPUDATA, `third_party/mesen2`, `AccuracyCoin`'s `$2007 Read w/
Rendering` 2), what it broke, and that MF-4e must satisfy both it and R2-005. Marked `// Q-125`.

`Ppu2C02::increment_coarse_x` and `Ppu2C02::increment_y` go back from `pub(super)` to private, so
`rust/crates/zax-nes/src/ppu/render.rs` is **byte-identical to `d96d73b7`** (`git diff d96d73b7 --`
is empty).

**What is kept.** The `$2004` half of step 6 is untouched: a register write to `$2004` while the
PPU is fetching does not reach OAM and bumps OAMADDR by `(oam_address + 4) & $FC`. Its test
`req_core_112_a_2004_write_while_fetching_bumps_the_address_by_four_and_writes_nothing` is
unchanged and passes. `Ppu2C02::fetching()` survives for it alone, and its doc comment now says
so and points at Q-125.

### The unit test: **inverted**, not `#[ignore]`d

`req_core_112_a_2007_read_while_fetching_increments_coarse_x_and_y_at_once` becomes
`req_core_112_a_2007_read_while_fetching_still_steps_by_the_ppuctrl_amount_q_125`, asserting
`(6, 8, 2)` on the visible line where it asserted `(6, 8, 3)`. The vblank leg is unchanged.

**Why inverted.** An `#[ignore]`d test pins nothing: it would not run, so it could not catch the
rule coming back by accident, and nothing left in the suite would state what `$2007` actually does
during rendering. The inverted test runs on every suite, states the behaviour the lot deliberately
ships, and **MF-4e has to come here and change it** — which puts the deferral at the exact spot
where the rule returns. Its doc comment carries the hardware rule verbatim and the `(6, 8, 3)` it
will assert again, so nothing is lost by inverting it.

### MF-3b's rule, on the five functions the lot names

Post-LTO dumps of clean builds, `cargo rustc --release -p zax-harness --bin zax-harness --
--emit=asm`: the baseline `fff53974` extracted with `git archive` into its own directory with its
own `CARGO_TARGET_DIR`, against this tree.

**Rule 1, call sites.** `callsites.py`, the whole watched list plus the five the brief names:

| callee | baseline | this tree |
| --- | ---: | ---: |
| `CpuBus::read_code` | 0 defs, 0 sites | **0, 0** |
| `NesBus::zone_read_cycle` | 0 defs, 36 sites | **0, 36** |
| `NesBus::halted_zone_read_cycle` | 0 defs, 1 site | **0, 1** |
| `NesBus::read_cycle` | 0 defs, 65 sites | **0, 65** |
| `NesBus::run_dma` | 2 defs, 2 sites | **2, 2** |
| `Ppu2C02::increment_vram_address` | 0 defs, 0 sites | **0, 0** |

All five identical. **No callee gains a first call site anywhere in the list**, and
`increment_vram_address` is back to 0 definitions and 0 call sites — exactly `d96d73b7`'s shape,
so dropping `#[inline]` did **not** turn it back into an out-of-line function. That was the one
risk in the revert and it is answered.

One site moves, downward: `Ppu2C02::read_nametable_inner` **15 → 14** static sites. That is the
exact mirror of MF-4d step 6's own line ("`read_nametable_inner` goes 14 → **15**: one more
*static* site, from LLVM specialising the `$2007` arm's two branches"). Taking the branch out takes
the specialisation out. It is not a first call site and it is one fewer, not one more.

**Rule 2, post-LTO bodies.** `cmp4c.py` over the pinned list. Two symbols differ and **both are
smaller**:

| symbol | baseline | this tree | delta |
| --- | ---: | ---: | ---: |
| `Ppu2C02::read` | 319 | **263** | **−56** |
| `NesBus::write_internal` | 2 805 | **2 763** | −42 |

`Ppu2C02::read`'s 263 is **MF-4d step 5's own number**, to the instruction: step 6 measured
"`Ppu2C02::read` 263 → 319, +56, inside the register-7 arm", and the 56 are back out. The 42 of
`write_internal` are the same inlined increment on the write side (step 6 measured +56 there
against MF-4d step 5's tree; the baseline here is ZR-2's, whose `write_internal` MF-5a and ZR-1/2
have since changed, so the figure is not the same one).

**Everything else on the pinned list is IDENTICAL, raw**, including every symbol the lot's
performance bar names: `Ppu2C02::run_dots` 1 932 → 1 932, `NesBus::read_cycle` 182 → 182,
`NesBus::read_internal` 176 → 176, `NesBus::run_dma` 994 → 994, `NesBus::dmc_only_dma` 191 → 191,
`NesBus::zone_read_cycle` 18 → 18, `NesBus::halted_zone_read_cycle` 43 → 43, `Cpu6502::step`
916 → 916, `Board::ppu_read` 328 → 328, `Apu2A03::run_cpu_cycles` 875 → 875.

**The bar is met in the only direction a back-out can fail in.** Nothing grew: the two symbols that
moved are strictly shorter, on the `$2007` path alone, and no other code is touched. The
instruction count is measured in step 5.

### Tests

| check | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean, **no new `allow`** |
| `cargo test --release -p zax-nes -p zax-harness` | **746 passed, 0 failed, 8 ignored** |

746 / 0 / 8 is ZR-2's count exactly: the test was renamed and inverted, not added or removed.

**Save states**: no field added, removed or reordered; nothing new serialised; `save_state` and
`load_state` untouched. The format does not move. No `unsafe`.

## Step 2 — R2-005 passes again, and the ignored tests listed one by one

`cargo test --release --no-fail-fast -p zax-nes -p zax-harness -- --ignored --test-threads=1`,
with `ZAXNES_ROMS` on the owner's commercial folder and `ZAXNES_TEST_ROMS` on
`assets-free/test-roms`. All **eight** ignored tests, every one actually run (none skipped for a
missing ROM):

| ignored test | result |
| --- | --- |
| **`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`** | **pass** |
| `zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack` | pass |
| `golden_pixels_tiles_audio_cycles_and_blargg_match_up_to_frame_1200` | pass |
| `req_core_004_apu_reset_needs_a_reset_that_keeps_ram` | pass |
| `req_core_113_dmc_dma_during_read4_is_p3` | pass |
| `the_sprite_dma_and_dmc_dma_collision_is_not_cycle_exact` | pass |
| `the_cost_of_one_capture_and_one_restore` | pass |
| `builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom` | **fail**, out of scope |

**The lot's criterion is met.** `amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`
passes. It had been failing on **4 986 of 5 000 frames** since `a758f923`, the same number in
ZR-1, in ZR-2 and on this lot's baseline; it now reports `at_split_line == 0`, the smallest gap
between two rows ≥ 3, and no frame above the thirty of a split-free image. AMD-021 R2-005
criterion 2 is satisfied, with no hack code involved — which is what made the regression a P1
matter rather than a picture preference.

`zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack`, ZR-2's own harness
regression test, **also passes** — the back-out does not disturb what ZR-2 pinned.

**The one failure is the one the brief puts out of scope.**
`builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom` fails on its second assertion,
`apply(&hack).is_none()` (`builtin_patches.rs:2525`): an already-patched image is patched again. It
is a patch-detection matter with nothing in the core, it fails identically on the baseline and on
ZR-1's and ZR-2's trees, and it is named out of scope here as a separate lot.

### The one row this lot gives up, by name

The non-ignored guard `req_core_110_accuracy_coin_keeps_the_rows_mf_4b_to_mf_4d_turned_green`
fails, and this is the intended cost:

    AccuracyCoin scored 128/147, which is under lot MF-4d's 129

**`PPU Misc. / $2007 read w/ rendering`** goes back from `pass` to `fail 2`, and it is the only
row that moves: 129 − 1 = 128, which is MF-4c's 125 plus `Controller Strobing`, `DMC Reload
Timing` and `$2002 flag timing`. `Address $2004 behavior` is unaffected — it still stops on code
8, the moving OAM address of lot MF-4e, because the `$2004` half is kept. Step 3 lowers the floor
to 128 and takes that one row out of the guard's named list; the other fifteen names stay.

## Step 3 — the guard's floor, 129 → 128, and the row named

`req_core_110_accuracy_coin_keeps_the_rows_mf_4b_to_mf_4d_turned_green` in
`rust/crates/zax-harness/tests/test_roms_tests.rs`:

* the floor goes **129 → 128**, and the message with it ("under lot MF-4d-fix's 128");
* the comment above it names the arithmetic — MF-4c's 125 plus `Controller Strobing`, `DMC Reload
  Timing` and `$2002 flag timing` — and **names the row given up**, with Q-125 and the condition
  for taking it back;
* **`"PPU Misc. / $2007 read w/ rendering"` comes out of the named list**, replaced by a comment
  that says it stood there from MF-4d step 6 until this lot, that it is a `fail 2` again, and that
  lot MF-4e puts it back together with R2-005.

The floor is still a floor and the other **fifteen** names are untouched, so a lot that fixes more
does not have to edit the test and one that breaks any of those fifteen still fails it. When MF-4e
brings the rule back under R2-005, the floor returns to 129 and the row returns to the list.

The row given up, stated once more for the record: **`PPU Misc. / $2007 read w/ rendering`**,
`AccuracyCoin` code 2, 129 → 128 / 147. No other row moves.

| check | result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean |
| `cargo test --release -p zax-harness --test test_roms_tests --test-threads=1` | **14 passed, 0 failed** |
| `cargo test --release -p zax-nes -p zax-harness` | **746 passed, 0 failed, 8 ignored** |

The whole suite is green again: the guard was the only non-ignored failure step 1 left.

## Step 4 — the pictures: the owner's case is byte-identical, and the three ROMs come back

### The owner-reported case

The core state `verify/states/smb3-fortress-pause.state` (the state block names its ROM:
`Super Mario Bros. 3 [USA][Rev 1][!!].nes`, a fortress, paused), two frames after the restore:

    zax-harness render --rom "<Super Mario Bros. 3 [USA][Rev 1][!!].nes>" \
      --load-state <state> --run 2 --out <dir> --png-nes <png>

and the same with `--advanced --widescreen 128`. Three binaries, each a **clean `git archive`
build in its own directory with its own `CARGO_TARGET_DIR`**: the reference `c4577878`, the
baseline `fff53974`, and this tree.

| picture | `c4577878` | baseline `fff53974` | this tree |
| --- | --- | --- | --- |
| plain | `b83105c2583e51cb…` | `c8e03f5e1e270dda…` **differs** | **`b83105c2583e51cb…` identical** |
| `--advanced --widescreen 128` | `5c88a3f88fd01062…` | `00ea68b19a0e038c…` **differs** | **`5c88a3f88fd01062…` identical** |

**Both pictures are byte-identical to a clean build of `c4577878`**, which is what the brief
asked for, and both were different on the baseline — so the A/B says the back-out is what
restores them and nothing else was needed.

**The symptom by eye, confirmed.** On the baseline the top row of the floor above the status bar
is cut and shifted by half a block: in the widescreen capture the left floor section is a broken
strip of half-height blocks instead of a row of whole ones. On this tree it is a complete row
again, in both modes. That is the owner's report, reproduced and closed.

### golden-core, this tree against the `d96d73b7` side

`zax-harness golden-core <roms> <out.jsonl> 120` over the 18 commercial ROMs, a clean build of
`d96d73b7` against this tree: **`diff` clean, all 18 rows identical.** Super Mario Bros. 3 and
Zelda II explicitly, by row hash: `2747c5bc…` = `2747c5bc…` and `82ded78e…` = `82ded78e…`.

The same run on the baseline says which ROMs the back-out is restoring, and it is exactly the set
MF-4d step 7 named:

| ROM | baseline vs `d96d73b7` | **this tree** vs `d96d73b7` |
| --- | --- | --- |
| `Super Mario Bros. 3 (USA).nes` | **DIFFERS** | **same** |
| `Zelda II - The Adventure of Link (USA).nes` | **DIFFERS** | **same** |
| `Zelda II - The Adventure of Link (Europe).nes` | **DIFFERS** | **same** |
| the other 15 | same | same |

**Three ROMs moved at `a758f923` and three come back; nothing else moves in either direction.**
The Europe Zelda II is the third: MF-4d step 7 named "Super Mario Bros. 3 and Zelda II", and both
Zelda II dumps are affected. `Duelito`, the third ROM step 7 named, is a free one and is covered
by the full golden-core run in step 5.

This is also the sharpest statement of the lot's shape available: golden-core runs **without hack
code**, so these rows move only because the core's own `$2007` behaviour changed, and they return
to `d96d73b7`'s exactly. The `$2004` half, which is kept, moves none of the 18 — the same result
MF-4d step 7 measured with a variant carrying only that half.

## Step 5 — the suites, the campaign, golden-core, and a performance cost that does **not** meet the bar

### The suites

| check | result | against ZR-2's |
| --- | --- | --- |
| `cargo fmt --check` | clean | same |
| `cargo clippy -p zax-nes -p zax-harness --all-targets -- -D warnings` | clean, no new `allow` | same |
| `cargo test --release -p zax-nes -p zax-harness` | **746 passed, 0 failed, 8 ignored** | **same** |
| the same `--test-threads=1` with `ZAXNES_TEST_ROMS` | **746 passed, 0 failed, 8 ignored** | same |
| `--ignored` with both ROM folders | **7 of 8 pass**, the Zelda II patch test the only failure | ZR-2 had **6 of 8** |

ZR-2's run reported two failures; this one reports one. The one that went green is
`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line` — the point of the lot.

### The campaign, ROM by ROM

`zax-harness test-roms` over `assets-free/test-roms`, baseline and this tree, compared row by row.

    331 ROMs, 214 pass, 5 fail, 0 timeout, 1 unsupported, 111 manual

**identical totals to ZR-2's, MF-4d's and MF-4c's.** Comparing the 331 rows field by field
(ignoring `.ms`, which is wall clock and moves on 299 rows in any two runs), **exactly two rows
move, and both were named in advance**:

| row | baseline | this tree | why |
| --- | --- | --- | --- |
| `AccuracyCoin/AccuracyCoin.nes` | 129/147 | **128/147** | `PPU Misc. / $2007 read w/ rendering` `pass` → `fail 2`. Step 3's row. |
| `nes-test-roms/other/Duelito.nes` | screen A | **screen B** | a `manual` row, no documented verdict, so no total moves. The third ROM MF-4d step 7 named. |

Within the `AccuracyCoin` row, `.detail.tests` moves on **one key only**, the one above; the other
146 are unchanged, `Address $2004 behavior` still stopping on code 8.

**Both are restorations, and the campaign proves it.** Against a clean build of `d96d73b7` this
tree's 331 rows differ on **one** row and **one** field: `Address $2004 behavior`, `fail 6` →
`fail 8` — which is precisely the `$2004` half the lot keeps. Duelito's screen is `d96d73b7`'s
exactly. So the campaign is `d96d73b7`'s campaign **plus the `$2004` half**, which is the shape
the lot set out to produce.

### golden-core

`golden-core <free> <out.jsonl> 1200`, 331 free ROMs, and `golden-core <comm> <out.jsonl> 120`,
18 commercial ones:

| set | this tree vs baseline | this tree vs `d96d73b7` |
| --- | --- | --- |
| 331 free ROMs, 1 200 frames | **1 row moves**: `Duelito.nes` | 1 row: `8bitpeoples_-_deadline_console_invitro.nes` |
| 18 commercial, 120 frames | **3 rows move**: SMB3, Zelda II USA, Zelda II Europe | **0 — `diff` clean** |

The `deadline` row is **not this lot's**: it is byte-identical between the baseline and this tree
(`baseline == this tree: True`), so it is one of the lots between `d96d73b7` and ZR-2 — the
`$2004` half being the likely one, that demo writing OAM through the register. Duelito is the
mirror: `d96d73b7 == this tree: True`, and it differs from the baseline.

**So against this lot's own baseline exactly four ROMs move in golden-core — Duelito, Super Mario
Bros. 3, Zelda II (USA), Zelda II (Europe) — every one of them back to `d96d73b7`'s output, and
every one of them a ROM MF-4d step 7 had named as moved by the rule.** Nothing else moves.

### The instruction count — **the bar is not met, and here is exactly what it costs**

PR-1's scripts in the WSL2 Ubuntu: `make-tree.sh`, then `build-clean.sh` for each side (a clean
`git archive` tar extracted into its own directory with its own `CARGO_TARGET_DIR`, never reused),
then `callgrind-set.sh` over `roms.txt`' thirteen ROMs, 300 frames, no warm-up, whole process.

| ROM | baseline `fff53974` | this tree | delta | | behaviour |
| --- | ---: | ---: | ---: | ---: | --- |
| `duelito` | 7 100 508 812 | 7 411 012 660 | +310 503 848 | +4.373 % | **moved** |
| `m0` | 5 544 879 030 | 5 544 912 096 | +33 066 | +0.001 % | — |
| `m1` | 4 915 938 139 | 4 915 451 289 | −486 850 | −0.010 % | — |
| `m4` | 5 502 464 705 | 5 501 909 252 | −555 453 | −0.010 % | — |
| `metroid` | 5 154 551 600 | 5 154 609 952 | +58 352 | +0.001 % | — |
| `mm2` | 6 038 173 401 | 6 038 218 937 | +45 536 | +0.001 % | — |
| `mmc5` | 11 420 253 509 | 11 421 265 390 | +1 011 881 | +0.009 % | — |
| `oamstr` | 3 339 441 540 | 3 344 764 569 | **+5 323 029** | **+0.159 %** | — |
| `readbuf` | 6 851 884 536 | 6 862 923 133 | **+11 038 597** | **+0.161 %** | — |
| `smb1` | 6 032 339 510 | 6 033 078 027 | +738 517 | +0.012 % | — |
| `smb3` | 8 005 053 260 | 7 967 968 531 | **−37 084 729** | −0.463 % | **moved** |
| `vblnmi` | 3 328 510 177 | 3 331 792 209 | +3 282 032 | +0.099 % | — |
| `z2` | 6 310 972 435 | 6 316 807 372 | +5 834 937 | +0.092 % | **moved** |
| **total** | 79 544 970 654 | 79 844 713 417 | +299 742 763 | +0.377 % | |
| **the ten whose behaviour is unchanged** | 58 128 436 147 | 58 148 924 854 | **+20 488 707** | **+0.035 %** | |

**The three ROMs marked "moved" are not a code cost.** Duelito, Super Mario Bros. 3 and Zelda II
are exactly the ROMs golden-core says changed output, so their binaries emulate different work;
the proof that this is behaviour and not code is that the moves go in **both** directions —
`smb3` is **37 M faster**, Duelito 310 M slower. They should be read out of the performance
question entirely.

**The ten remaining ROMs are the measurement, and they say +0.035 %.** Far above the ±50 000
per-ROM noise floor ZR-1 and ZR-2 measured for this instrument: `readbuf` +11.0 M and `oamstr`
+5.3 M are two hundred times it, and they are the `$2007` and `$2004` ROMs PR-1 keeps in the set
for exactly this path.

**One symbol carries all of it, and it is not a new call site.** On `vblnmi`, whose output is
byte-identical between the two builds:

| | baseline | this tree |
| --- | ---: | ---: |
| `Ppu2C02::read`, exclusive | 19 247 831 | **22 599 641** (+3 351 810) |
| **calls to `Ppu2C02::read`** | **304 710** | **304 710 — identical** |
| calls to `read_nametable_inner` / `read_pattern_planes` | 32 111 / 7 872 | **32 111 / 7 872 — identical** |
| every other symbol above 100 000 | — | **none moves** |

Identical call counts and +3.35 M executed: **`Ppu2C02::read` costs about eleven more instructions
per call**, and that is the whole of the ten ROMs' delta.

**What it is not.** Not a new call site: `callsites.py` on the post-LTO dumps of **both** targets
says every watched callee is unchanged and `increment_vram_address` is 0 definitions / 0 call
sites on each. Not the `#[inline]`: putting the attribute back was built and measured and moves
`vblnmi` by 6 882 instructions in 3.33 **billion**, which is nothing — so the attribute is gone,
which is `d96d73b7`'s shape, and the comment on the function records the measurement. Not a bigger
function: `Ppu2C02::read` is **smaller** on both targets, 319 → 262 on Linux and 319 → 263 on
Windows, and every other pinned symbol is `IDENTICAL` raw.

**What it is: MF-3b rule 4, a dispatch whose shape flipped because an arm lost its own body.**
Diffing the two Linux bodies instruction by instruction, the register dispatch and the `$2002` arm
sit in the baseline as the **fall-through** immediately after the inlined `apply_decay` unroll; in
this tree the same block is reached by a `jmp` and the `$2004`-during-rendering block has been
folded in beside it. The `$2007` arm losing its `fetching` branch is what let LLVM re-lay the
function — "a new arm with a body of its own can flip a whole dispatch's shape", read backwards.
MF-4d step 6 therefore bought about eleven instructions per register read **by accident**, and
backing its behaviour out gives them back.

`callgrind-diff.py` prints *SLOWER on 10 ROMs*. **Unlike in ZR-1 and ZR-2, that verdict is right
here** — on seven of those ten it is a real per-call cost, and the three others are behaviour.

### The bar, answered honestly

The lot's bar is "identical to the instruction, or lower, on every ROM without hack code".
**This lot does not meet it.** The honest statement is:

* the cost is **+0.035 %** over the ten ROMs whose behaviour is unchanged, worst case **+0.161 %**
  on `readbuf`;
* it is **one symbol**, `Ppu2C02::read`, about **+11 instructions per register read**, with no new
  call site and no larger function anywhere;
* it is a **codegen-layout effect**, not a line of added work — the source this lot ships executes
  strictly less than the baseline's, and `run_dots`, `read_cycle`, `read_internal`, `run_dma`,
  `dmc_only_dma`, `zone_read_cycle`, `halted_zone_read_cycle`, `Cpu6502::step` and the `Board`
  dispatches are all byte-identical;
* it cannot be removed by keeping the behaviour, because the behaviour **is** the branch: the only
  way to hold MF-4d step 6's layout is to hold its `$2007` rule, which is the P1 regression this
  lot exists to undo.

**R0's call, with the trade stated plainly**: AMD-021 R2-005 (P1) and the owner's two pictures,
against +0.035 % of emulated instructions on ten ROMs. I shipped the back-out, as instructed, and
I am not hiding the cost. If R0 wants the layout back, it is a codegen-only change to
`Ppu2C02::read` — forcing the `$2002` arm to the fall-through position — that carries no behaviour
and belongs in its own lot; it is **not** attempted here, because the brief scopes this lot to the
back-out and the first attempt (the `#[inline]`) was measured and refuted. Lot **MF-4e**, which
rewrites this path per dot, will re-lay the function anyway and should measure it then.

**Save states**: no field added, removed or reordered; nothing new serialised; `save_state` and
`load_state` untouched. The format does not move. No `unsafe`. `cargo fmt --check` and clippy
clean, no new `allow`.
