# MF-LEG-1B — the MMC1's SNROM `E` bit follows puNES by default

Role R3, clean side. Base commit **`e2507f56`** (`MF-LEG-1 (10/10): the final check …`), branch
`mapper-review`, worktree `wt-mappers`.

Sources consulted: `docs/progress/MF-LEG-1.md` (steps 1, 8, 9 and 10), `spec/changes/Q-176.md`
(the question and **R0's answer**), `docs/review/LEGACY-XCHECK.md` entry MMC1-1,
`docs/review/BOARD-FEATURES.md`, the nesdev *MMC1* page, `assets-free/test-roms/holy-mapperel`
(the two `M1_P128K*` headers and `README.md`), and the five cross-checked emulators in
`third_party/` on the one point this lot decides.

## What the lot does

**One decision, carried out.** R0 answered Q-176 on 2026-11-08: by default this build follows
puNES and the four other emulators — the SNROM `E` bit (bit 4 of the CHR bank registers as a
second chip enable for the PRG-RAM) is **not applied at all**. PRG-RAM is disabled by `$E000`
bit 4 alone, and never on an MMC1A. The bit stays reachable behind a new feature
**`mmc1-snrom-e-bit`**, off by default, and MF-LEG-1's stepping stone `mmc1-snrom-header` is
removed.

| # | step | the brief's steps |
| --: | --- | --- |
| 1 | `mmc1-snrom-e-bit` off by default; `mmc1-snrom-header` removed; the explanation at the `E`-bit site; the two `holy-mapperel` rows | 1, 2, 3, 4 |
| 2 | Q-176's *Decision*, report entry MMC1-1 and `BOARD-FEATURES.md` | 5 |
| 3 | golden-core over the 98 originals against the base commit, and the full test run with both ROM roots | the final check |
| 4 | the callgrind campaign | the final check |
| 5 | the final check | the final check |

**Why the brief's four first steps are one commit.** Removing `mmc1-snrom-header` removes
`Mmc1::SNROM_NEEDS_A_DECLARED_HEADER`, which is what `SNROM_ROW` in the harness and the
`m1_p128k` diagnosis in `report.rs` are written over. The feature swap, the removal and the two
rows' new expectation are therefore one edit that compiles, not three. The rows are *proved* at
step 3, where the suite is run with `ZAXNES_TEST_ROMS` set — which is the only way to run it, and
what MF-LEG-1 step 8 came out of.

## Steps

- [x] 1/5 `mmc1-snrom-e-bit` off by default, `mmc1-snrom-header` removed, the explanation in the code, the two rows

### 1/5 — the feature, the removal, the explanation and the two rows

**The code.** `Mmc1::chr_disable_mask` is now `#[cfg(feature = "mmc1-snrom-e-bit")]`, and so is the
one `Mmc1::is_snrom` left — the **three-way size test** (PRG-ROM at most 256 KiB, declared CHR-ROM
at most 8 KiB, **effective** PRG-RAM exactly 8 KiB) that every build from MF-3 to this lot's base
commit shipped. `Mmc1::refresh_prg_ram_disable` keeps one expression, its second term a `let`
under the same `cfg`:

```rust
#[cfg(feature = "mmc1-snrom-e-bit")]
let e_bit = self.chr0 & self.chr_disable_mask;
#[cfg(not(feature = "mmc1-snrom-e-bit"))]
let e_bit = 0u8;
self.prg_ram_disabled = ((self.prg & self.prg_disable_mask) | e_bit) != 0;
```

so with the feature **on** the committed code is the base commit's, and with it off the second
term folds away: one load, one `and`, one test, and `Mmc1` is the base commit's layout **minus one
byte**. `Mmc1::prg_disable_mask` is untouched, so `$E000` bit 4 still disables on every board that
wires it and still does not on an MMC1A or a mapper 155 — the default build's only wire.

**Removed**: the feature `mmc1-snrom-header` with its `Cargo.toml` block, the NES 2.0
header-declared `is_snrom` body, the const `Mmc1::SNROM_NEEDS_A_DECLARED_HEADER`, and its two
tests (`mf_leg_1_mmc1_an_ines_1_0_dump_with_chr_ram_does_not_get_the_snrom_e_bit`,
`mf_leg_1_mmc1_the_snrom_e_bit_needs_all_three_nes_20_declarations`). Q-176 asked for exactly
that: under R0's answer the default build asks of no dump whether it is an SNROM, so the question
the feature answered has no caller. Nothing is commented out; the `E` bit itself is whole, behind
one flag.

**The explanation is in `mmc1.rs`**, as a banner comment immediately before the `E`-bit machinery
(*the SNROM `E` bit*) so that it reads the same in both configurations, with the field, the board
test and `refresh_prg_ram_disable` pointing at it. It states what the page says (the SNROM's CHR
A16 as a second chip enable, the `E` bit as "PRG-RAM disable (0: enable, 1: open bus)"); what
`holy-mapperel` tests (digit `4xxx`, `README.md:201-203`, and the two images that reach the SNROM
half announcing "SNROM (MMC1)" and "8K PRG RAM OK" over a header whose byte 10 is `$00`); that no
cross-checked emulator applies it, each with its `file:line` in `third_party/` — FCEUX
`mmc1.cpp:40-49`, Nestopia UE `NstBoardMmc1.cpp:138-146`, Nintendulator `h_MMC1.cpp:132-137`,
Mesen 2 `MMC1.h:85-87` and `:131`, puNES `MMC1.c:124-154` with the diagram at `:125-143`, the
*Witch n' Wiz* note at `:145-146`, the commented-out expression at `:147-151` and the shipped one
at `:152-154`; that the SNROM's sizes are the **SGROM's**, so no iNES 1.0 dump can be told apart
(*Zelda*, *Metroid*, *Final Fantasy*, *Mega Man 2*, *Castlevania II*); and why the owner chose
compatibility — the bit can only ever take a work RAM away, so dropping it cannot break a running
game and fixes a named one, while applying it leaves this build alone among six on a wire no
shipped game exercises. **Every citation above was opened and checked against the source in this
step**, not carried over from entry MMC1-1: the five emulators' line numbers, the README digit and
the two 16-byte headers (`4e 45 53 1a 08 00 10 08 00 00 00 07 …`, byte 10 = `$00`, byte 11 =
`$07`).

**The two rows.** `SNROM_ROW` in `zax-harness/tests/test_roms_tests.rs` is now
`if Mmc1::SNROM_E_BIT { Pass } else { Fail }` — the constant that replaces
`SNROM_NEEDS_A_DECLARED_HEADER` — so `M1_P128K.nes` and `M1_P128K_CR8K.nes` are asserted to
**fail in `default` and pass with `mmc1-snrom-e-bit` on**, and a build either way is checked.
`report::diagnose`'s `m1_p128k` entry (which matches both images) carries
`expected: !Mmc1::SNROM_E_BIT` and a text that says the decision, the five votes and the feature,
so the generated report counts the rows under "**n of these are required to fail**" exactly as it
does `mmc3_test/6-MMC6` — never as a regression. The crate's module doc and `SPECIFIC`'s doc say
so too.

**Also updated**, so that no comment still describes the old rule: `Mmc1::new`'s MMC1A note,
`Mapper::prg_ram_enabled`'s doc, `Mmc1::chr_bank0_has_the_e_bit` (kept `pub` and **not** behind
the feature — the five-ROM extra check has to run in the build that ships),
`mf_leg_1_mmc1_e_bit_tests.rs`'s module doc (the same evidence now backs the stronger change),
and `NesEvent::prg_ram_enabled`'s trap note, whose override stands whatever the feature says.
`req_core_100_mmc1_snrom_disables_prg_ram_with_bit_4_of_the_chr_register` moves behind the
feature; `req_core_100_mmc1_only_an_snrom_reads_bit_4_of_the_chr_register_as_a_ram_enable` — the
"all but SNROM" half of digit `4xxx` — stays in `default`, where it is now trivially true and
still worth pinning. The new
`mf_leg_1b_the_snrom_e_bit_is_not_applied_in_the_default_build` asserts the decision on **both**
an iNES 1.0 dump and a fully declared NES 2.0 SNROM, and that `$E000` bit 4 still disables on
each.

**Checks at this step** (`CARGO_TARGET_DIR=rust/target-mf`): `cargo fmt -p zax-nes -p zax-harness
--check` clean; `cargo clippy --all-targets -- -D warnings` clean on both crates in `default`,
`--features zax-nes/mmc1-snrom-e-bit` and `--features zax-nes/parked-boards`, and on `zax-nes`
alone with `--no-default-features`; `cargo build -p zax-nes` in all three feature states.

| `cargo test --release -p zax-nes -p zax-harness` | result |
| --- | --- |
| `default` | **1166 passed, 0 failed, 10 ignored** |
| `--features zax-nes/mmc1-snrom-e-bit` | **1166 passed, 0 failed, 10 ignored** |

The base commit's `default` run was 1166 as well: one test left `default` (behind the feature) and
one joined it. The ten ignored are the ROM-gated ones; step 3 runs them.
- [x] 2/5 Q-176, entry MMC1-1, `BOARD-FEATURES.md`

### 2/5 — the three documents

**Q-176** (`spec/changes/Q-176.md`) gets a **Decision** section and is **closed**: owner R0, date
2026-11-08, answer (b) and further than (b) asks; the default (no `E` bit on any board, `$E000`
bit 4 only, never on an MMC1A or a mapper 155); the feature `mmc1-snrom-e-bit` and what it
restores exactly; what was removed; the two conformance rows and how they are marked; where the
reasoning lives; and the commit. Its *Code reference* header is rewritten to the symbols that now
exist, with a note that `mmc1-snrom-header` and `SNROM_NEEDS_A_DECLARED_HEADER` — which the body
of the ticket discusses at length — are gone. Nothing in the analysis sections is rewritten: they
are the record of the question, and the answer sits under them. *(Q-176 is in `spec/`, which this
lot may not otherwise touch; the brief asks for this one append.)*

**Entry MMC1-1** (`docs/review/LEGACY-XCHECK.md`) gets a **"Decided in MF-LEG-1B (`e1c4d60b`)"**
paragraph above MF-LEG-1's, which it supersedes rather than replaces — the reasoning took two
lots and the record should show it. It says plainly that the entry's own verdict line, **DIFFERS**,
no longer describes the shipped build: this build now agrees with all five emulators and departs
from the page, deliberately. Two other places are brought in line: the summary table's row
(`**yes**` → `**no**, decided MF-LEG-1B`, and the verdict `**DIFFERS**` → `agrees with all five`)
and the *"fixed differently from the way this table states them"* bullet, which described the
removed header rule.

**`docs/review/BOARD-FEATURES.md`**: the `mmc1-snrom-header` row is replaced by the
`mmc1-snrom-e-bit` row — the page, the five emulators with their `file:line`, the named
regression, the SNROM/SGROM indistinguishability, what the feature restores, the two required
failures, why it is not in `parked-boards`, and the flag. Its two figure cells say *MF-LEG-1B
step 4*, where they are filled in. The "worst figure among the features in `default`" paragraph
records the swap and notes that the shipped build now does **less** work than the base commit's
on this path.
- [x] 3/5 golden-core and the full test run

### 3/5 — golden-core over the 98 originals, and the suites with both ROM roots

```
zax-harness golden-core G:/3dsen/test-originals gc98-<label>.jsonl 1200
```

The base commit was extracted with `git archive e2507f56` into a **fresh** directory with its own
`CARGO_TARGET_DIR`, built there, and the directory deleted once the figures were taken.

| build | md5 of the 98-ROM digest |
| --- | --- |
| `e2507f56`, the base commit, fresh tree (binary md5 `f25cecfcd7fee6c7016ef10dca003550`) | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `default` | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `--features zax-nes/mmc1-snrom-e-bit` | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `--features zax-nes/parked-boards` | `f79e7df842c3ee43e7d9b67861a927b5` |

**Byte-identical in all four** — `cmp` on the files, not only the digests — and identical to the
digest thirteen lots have now recorded. 98 ROMs run, 98 files.

**Why it could not have moved.** The 98 use mappers 0, 1 (**27 ROMs**), 2, 3, 4 and 156, so only
mapper 1 is in play, and MF-LEG-1 step 1's extra check already established that none of the five
dumps the old size test read as SNROMs ever commits bit 4 to `$A000` — 3600 frames each, 181 M
instructions, sampled after every CPU instruction. The 27 mapper-1 ROMs here are a second,
independent run of that at 1200 frames each: a game that never sets the `E` bit cannot notice the
bit being withdrawn. The digest holding with the feature **on** is the same statement from the
other side.

**The suites**, `ZAXNES_ROMS=G:/3dsen/demo/roms` and
`ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`, `--no-fail-fast --
--include-ignored`:

| run | result |
| --- | --- |
| `default` | **1176 passed, 0 failed, 0 ignored** |
| `--features zax-nes/mmc1-snrom-e-bit` | **1176 passed, 0 failed, 0 ignored** |
| `--features zax-nes/parked-boards` | **1237 passed, 0 failed, 0 ignored** |

**This is where the two `holy-mapperel` rows are actually proved**, the suite only running when
`ZAXNES_TEST_ROMS` points at the collection: `SNROM_ROW` is `Fail` in the first run and `Pass` in
the second, and `check` asserts the outcome of every ROM of the group by name, so neither row can
drift unnoticed in either direction. The default and feature totals are equal because one test
left `default` for the feature and one joined it.

**A correction to MF-LEG-1's final check.** Its table records **1244** for the `parked-boards`
run; the figure does not reproduce, and the base commit does not produce it either. The base
commit built from `git archive` in the fresh tree above gives **1237 total** for the same command
(1233 passed and 4 failed, the four being `bench_core_tests`, which needs `rust/test-roms/nestest.nes`
— a file that is deliberately never committed, so a `git archive` tree cannot have it; in a real
checkout they pass). 1237 is therefore the base commit's number as well as this tree's, which is
what the diff requires: the only `#[test]` items this lot moves are one that left `default` and
one that joined it, and in a `parked-boards` build without the new feature those cancel exactly.
**So the `parked-boards` row is unchanged by this lot and MF-LEG-1's 1244 is a transcription
error**, not a regression. Recorded here rather than silently re-stated, since the next lot will
compare against it.
- [x] 4/5 the callgrind campaign

### 4/5 — the callgrind campaign

Binaries built by `rust/tools/perf/build-clean.sh` from their own tar into a **fresh** directory
with its own `CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`. The feature variant
was made from the shipped tar by adding one line to zax-nes's `default` list
(`rust/target-mf/variant-tar-add.sh`, scratch, not committed).

| label | tree | md5 |
| --- | --- | --- |
| `leg1b-base` | **`e2507f56`**, the lot's base commit | `6ec0ea0ce0c33062b93d03ba74cc5b3e` |
| `leg1b-ship` | **the shipped configuration** (`bcf622e2`): the `E` bit out of `default` | `64edf2c20f580029ca0898bc6a87feb3` |
| `leg1b-ebit` | the shipped tree plus **`mmc1-snrom-e-bit`** in `default` | `100b8110bc42eb07a9e6ed979a195fc3` |
| `leg1b-ship2` | a byte copy of `leg1b-ship`, the **same-binary floor** | the same |

**The base binary's md5 is `6ec0ea0ce0c33062b93d03ba74cc5b3e`, which is MF-LEG-1's `leg1b-ship` —
that lot's own measured shipped binary — to the byte.** So this lot's base is the previous lot's
measured binary, the **twelfth** lot in a row for which that holds, and built rather than assumed.
It holds although `e2507f56` is two commits later than the tree MF-LEG-1 measured, because
everything it changed in `src/` was a comment: `git diff 530979f7 e2507f56 -- rust/crates/zax-nes/src/`
has no non-comment line.

#### 4.1 The floors

**PR-1's thirteen**, the shipped binary against its own byte copy: worst `m1` **+0.001 %**
(+59 358), set total +171 313 (+0.000 %). So **±0.001 % is noise on PR-1**, a twentieth of the
tolerance, which is what MF-FIX and MF-LEG-1 both recorded.

**Level 2 is worse than MF-LEG-1 measured it, and this lot's figures have to be read against
that**:

| pair (identical binaries) | `smb1` | `metroid` | `zelda2` | `mm2` | `smb3` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `leg1b-ship` vs `leg1b-ship2` | **+0.029** | +0.000 | −0.008 | +0.001 | −0.008 |

**The same binary measured twice moves `smb1` by +0.029 %**, i.e. *above* the owner's +0.02 % per
ROM. MF-LEG-1 step 9.1 recorded ±0.018 % from two pairs; this is a third pair and it is worse. The
level-2 whole-process instrument therefore cannot resolve anything of this size, and a level-2
whole-process figure under about 0.03 % is not evidence either way.

**But the level-2 run also carries a stage breakdown, and that one is exact.**
`callgrind-l2.sh` writes `stages.txt`, whose `core` row is the emulated core's own `Ir` —
`NesCore::run_frame` and everything under it, with no conversion, no Lua and no draw list. Across
the two identical binaries it is **+0 on all five games, identical to the instruction**. So `core`
is the instrument to read here, and the whole-process noise is provably outside the emulation.

#### 4.2 The emulated core, which is the measurement that matters

| game | mapper | `leg1b-base` core `Ir` | `leg1b-ship` | delta | `leg1b-ebit` | delta vs base |
| --- | :-: | ---: | ---: | ---: | ---: | ---: |
| `smb1` | 0 | 11 515 129 187 | 11 515 129 187 | **+0** | 11 515 129 187 | **+0** |
| `metroid` | 1 | 11 146 892 645 | 11 146 889 701 | **−2 944** | 11 146 892 645 | **+0** |
| `zelda2` | 1 | 12 775 026 724 | 12 775 020 988 | **−5 736** | 12 775 026 724 | **+0** |
| `mm2` | 1 | 12 037 321 533 | 12 037 310 877 | **−10 656** | 12 037 321 533 | **+0** |
| `smb3` | 4 | 15 315 255 735 | 15 315 255 735 | **+0** | 15 315 255 735 | **+0** |

Three statements, each exact to the instruction:

1. **The shipped build is cheaper on every mapper-1 game and identical on every other.** `smb1`
   (mapper 0) and `smb3` (mapper 4) do not move at all, and the three mapper-1 games are
   *negative*, which is `Mmc1::refresh_prg_ram_disable` losing a load, a mask and an `or` and
   `Mmc1` losing a byte. This lot does not cost; it refunds.
2. **`mmc1-snrom-e-bit` on is the base commit's emulation exactly**, `+0` on all five games. That
   is step 1's claim — "restores today's behaviour exactly" — measured rather than asserted.
3. **The level-2 whole-process noise is outside the emulation.** `smb1`'s `core` is the same
   number in all four labels while its whole-process figure swings ±0.03 %, which is
   MF-UNL-12's finding again (there, 91 % of the same swing was one Lua symbol).

#### 4.3 PR-1's thirteen

**The lot as it ships, against the base commit**: worst ROM **`m1` +376 791 (+0.008 %)**, every
other ROM inside ±0.001 % — the floor — and set total +412 552 (+0.001 %) of 79 346 126 844.

**`m1` *is* `M1_P128K.nes`, the ROM of Q-176**, and its figure is this lot's decision showing up as
emulation, not as a code cost: with the `E` bit gone the ROM's `4000` digit fails, so from that
point the ROM runs different code. **The control proves it.** `leg1b-ebit` against `leg1b-base` is
the same emulation on all thirteen, and there `m1` is **+19 316 (+0.000 %)** with the worst ROM
`metroid` at +0.001 % and a set total of +123 316 (+0.000 %) — nothing. MF-LEG-1 step 9 measured
+0.008 % on `m1` for the *inverse* change for the same reason; the two lots bracket it.

**`mmc1-snrom-e-bit` alone**, `leg1b-ship` → `leg1b-ebit`: `m1` **−357 475 (−0.007 %)** — the ROM
going back to passing — worst other ROM `m4` +0.001 %, set total **−289 236**.

#### 4.4 Level 2, whole process, for the record

| pair | worst game | set total |
| --- | --- | --- |
| the lot as it ships, against the base | `zelda2` **+0.006 %** | +14 976 (+0.000 %) |
| `mmc1-snrom-e-bit` on, against the base | `smb1` +0.023 % | +0.005 % |
| `mmc1-snrom-e-bit` alone, against the shipped | `smb1` +0.036 % | +0.005 % |

The shipped build's worst is +0.006 %, a fifth of this instrument's own floor. The two feature
rows land on `smb1` **at and above** the tolerance — and `smb1` is **mapper 0**, which this
feature cannot touch: its `core` is the same number to the instruction in every label (§4.2), and
the same binary measured twice moves it +0.029 %. Those two figures are the instrument, not the
feature.

#### 4.5 The feature decision

| feature | PR-1's thirteen | level 2, emulated core | level 2, whole process | in `default`? |
| --- | --- | --- | --- | :-: |
| **the lot as it ships** (no `E` bit) | worst `m1` **+0.008 %**, the test ROM's own path; every other ROM at the ±0.001 % floor; total +0.001 % | **−2 944 to −10 656 on the three mapper-1 games, +0 on the other two** | worst `zelda2` +0.006 % | — |
| `mmc1-snrom-e-bit` | against the shipped build: `m1` **−0.007 %**, worst other +0.001 %, total **−289 236**. Against the base, same emulation: worst +0.001 %, `m1` +0.000 % | **+0 on all five, identical to the base commit** | `smb1` +0.036 % on a game it cannot touch, against a same-binary floor of +0.029 % on that game | **off — Q-176, and not for a figure** |

**Nothing is parked, and nothing could be.** The owner's rule parks a *board* that costs more than
+0.02 % per ROM; this lot adds no board, and the one feature it adds is already out of `default` by
R0's decision rather than by a figure. There is no cost to park either: the shipped build is
measurably **cheaper** on the emulated core of every mapper-1 game and identical on every other,
and the feature build is the base commit's emulation to the instruction. The only figure above the
tolerance anywhere in the campaign is `smb1`'s level-2 whole process, which the same binary
reproduces against itself.
- [x] 5/5 the final check

### 5/5 — the final check

`zax-app` cannot build in this worktree, as the brief says; the crates this lot touches are
`zax-nes` (the MMC1, and one comment on the NES-EVENT) and `zax-harness` (the `holy-mapperel`
expectation and the `m1_p128k` diagnosis). `CARGO_TARGET_DIR=rust/target-mf`, `TMP`/`TEMP` in
`rust/target-mf/tmp`.

**Formatting.** `cargo fmt -p zax-nes -p zax-harness -- --check` clean.

**Clippy, `-D warnings`, `--all-targets`** — the four configurations the brief asks for:

| configuration | crates | result |
| --- | --- | --- |
| `default` | `zax-nes`, `zax-harness` | clean |
| `--features zax-nes/mmc1-snrom-e-bit` (the new feature alone) | `zax-nes`, `zax-harness` | clean |
| `--features zax-nes/parked-boards` | `zax-nes`, `zax-harness` | clean |
| `--no-default-features` | `zax-nes` | clean |

**Tests.**

| run | result |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **1166 passed, 0 failed, 10 ignored** |
| the same `--no-fail-fast -- --include-ignored` with `ZAXNES_ROMS=G:/3dsen/demo/roms` and `ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms` | **1176 passed, 0 failed, 0 ignored** |
| the same plus `--features zax-nes/mmc1-snrom-e-bit` | **1176 passed, 0 failed, 0 ignored** |
| the same plus `--features zax-nes/parked-boards` | **1237 passed, 0 failed, 0 ignored** |
| `cargo test --release -p zax-nes --no-default-features` | 790 passed, 0 failed, 9 ignored |

The ten ignored are the ROM-gated ones, MF-LEG-1 step 1's five-ROM extra check among them; with
the roots set they do work rather than returning early, and **the `holy-mapperel` suite only runs
with `ZAXNES_TEST_ROMS` set**, which is where this lot's two rows are checked — `Fail` in the
first two runs and `Pass` in the third, asserted by name either way. `parked-boards` is 1237 and
not MF-LEG-1's recorded 1244 for the reason step 3 establishes: 1237 is the base commit's own
number.

**golden-core over the 98 originals**, re-run from the final tree (binary md5
`9ec6a04f156fa3dacc42bcc2a087c053`):

| build | md5 of the digest |
| --- | --- |
| `e2507f56`, the base commit, built from `git archive` in a fresh tree | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `default` | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `--features zax-nes/mmc1-snrom-e-bit` | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `--features zax-nes/parked-boards` | `f79e7df842c3ee43e7d9b67861a927b5` |

**Byte-identical in all four** (`cmp`, not only the digest), and identical to the digest thirteen
lots have now recorded.

**Save states.** Not one byte changed. `Mmc1::save_registers` writes the shift register, the
shift count, the three register bytes and the last-write cycle, and no mask: `chr_disable_mask`
was load-time derived state and its disappearance from the struct cannot be seen by the format.
`load_registers` still calls `refresh_prg_ram_disable`, which is what recomputes
`prg_ram_disabled` (REQ-CORE-091). An MMC1 state written by any earlier build loads into this one
and vice versa, and no other board is touched at all.

**The figures and the feature decision**, which is the last thing the brief asks — §4.5 has the
table. In one line: **nothing is parked, and the shipped build is cheaper than its base** by
2 944 to 10 656 core instructions on each of the three mapper-1 level-2 games and `+0` to the
instruction on the two that are not, while `mmc1-snrom-e-bit` reproduces the base commit's
emulation exactly, `+0` on all five.

**The one figure a reader should not misread**: `m1` **+0.008 %** on PR-1 in the shipped build.
`m1` is `M1_P128K.nes`, the ROM of Q-176; with the `E` bit withdrawn its `4000` digit fails and
from there it runs different code. The same-emulation control (`mmc1-snrom-e-bit` against the base)
puts `m1` at **+0.000 %**, so the figure is this lot's decision showing up as emulation and not a
cost. MF-LEG-1 step 9 measured the same +0.008 % on `m1` for the inverse change.

## What this lot leaves behind

* **Q-176 is closed**, with its *Decision* section and the commit that carries it.
* **Entry MMC1-1's verdict line no longer describes the shipped build**, and the entry says so:
  this build now agrees with all five cross-checked emulators and departs from the nesdev page,
  deliberately and for a reason recorded at the bit's own site. That is the first place in the
  mapper work where AMD-061 is set aside by decision rather than by oversight, which is why the
  reasoning is in the code and not only in the ticket.
* **Two `holy-mapperel` rows are required failures in `default`** and pass with one flag. They
  are marked `expected` so no report ever reads them as defects, and they are asserted both ways
  so neither can drift.
* **A correction to MF-LEG-1's final check**: its `parked-boards` figure of 1244 does not
  reproduce and the base commit gives 1237 (step 3).
* **A worse level-2 floor than any lot has recorded**: the same binary measured twice moves
  `smb1` by **+0.029 %**, above the owner's tolerance. The `stages.txt` `core` row is `+0` across
  identical binaries and is the instrument to use instead (step 4.1). The next lot that measures
  a level-2 whole-process figure under about 0.03 % should treat it as noise.
* **The follow-up MF-LEG-1 step 5 already named is still open**: mapper 155 is the same ASIC
  revision as the MMC1A and does not get the `R` bit's A17 in this build, being outside that
  lot's scope. Nothing in this lot changes it.
