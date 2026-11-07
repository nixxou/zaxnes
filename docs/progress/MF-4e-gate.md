# Lot MF-4e-gate — the `$2007`-during-rendering rule behind its own feature, off by default

Baseline for behaviour and for performance: **`11407930`** (MF-VS 9/10, the commit lot MF-4e took
as its own baseline). Branch tip at the start: **`15c84cd5`** (MF-VS-fix 6/6).

MF-4e step 2 put the `$2007`-during-rendering rule back and step 4 measured it at +0.000 % to
+0.005 % on the eleven ROMs of PR-1's set whose behaviour does not move, so it shipped **in
`default`**. This lot does not question the rule; it questions the **default**.

**Why (R0, 2026-11-07).** R0's dense A/B of the 3D packs against v0.4.2 shows the rule changes
what the player sees in exactly the two games the owner watches most — *Super Mario Bros. 3*
(5 dumps, 58 of 681 sampled frames differ) and *Zelda II* (2 dumps, 18-19 of 1 001), with and
without pack — and SMB3's split line under the widescreen hack is where the owner saw a bug the
last time this rule shipped (lot MF-4d, backed out by MF-4d-fix). The rule may well be right; the
owner validates it by eye before it is the default. **Until then the default build must render as
v0.4.2 does**, which for this path means as `11407930` does.

## Steps

* [x] **0/6** — the plan, the reading, and exactly what is gated.
* [x] **1/6** — the gate: the feature, the two bodies of `increment_vram_address`, every test that
  pins the rule under it, the inverted test back in the default build, the guard's two floors
  becoming three.
* [x] **2/6** — the row in `docs/review/BOARD-FEATURES.md`, and the ticket.
* [x] **3/6** — fmt, clippy `-D warnings` and `cargo test --release -p zax-nes -p zax-harness` in
  the three configurations, and the ignored suite in the default build.
* [x] **4/6** — golden-core over the commercial originals byte-identical to `11407930`'s, and the
  campaign's 331 rows field by field against MF-VS-fix's in both configurations.
* [x] **5/6** — the callgrind bar: the default build no higher than `11407930`'s on both runs.
* [x] **6/6** — the journal, and the final pass on the committed tree.

## Step 0 — the plan

### What the rule is, and what exactly is being gated

`Ppu2C02::increment_vram_address` is called from **both** `$2007` paths — the read
(`Ppu2C02::read_data`) and the write (register 7 of `Ppu2C02::write`). MF-4e step 2 gave it a
`self.fetching()` branch:

    if self.fetching() {
        self.increment_coarse_x();
        self.increment_y();
        self.v_row_next = self.v;        // and NOT v_row_start — that is R2-005's exception
    } else {
        let step = if (self.control & 0x04) != 0 { 32 } else { 1 };
        self.v = self.v.wrapping_add(step) & 0x7FFF;
        self.v_row_start = self.v;
        self.v_row_next = self.v;
    }

The `else` arm **is `11407930`'s whole body**. So the gate is not a rewrite: with the feature off,
the function is `11407930`'s five lines again, and with it on it is MF-4e step 2's. Two bodies
under `#[cfg]`, which is the idiom `run_sprite_evaluation` already uses in this very file for
`ppu-exact-sprite-eval`.

**The name.** `ppu-2007-render-read`, the brief's. The rule is an *access* rule and not a read
rule — the write side borrows the same increment, and the doc comment and the feature's row both
say so — but the name follows the handle R0 and `AccuracyCoin` both use for it: the row the
feature wins back is literally called `PPU Misc. / $2007 read w/ rendering`.

**What is deliberately *not* gated**, because it is not the rule:

* `Ppu2C02::fetching` stays as it is. The `$2004`-write-while-fetching quirk of MF-4d step 6 is
  kept in every build (MF-4d-fix step 1 kept it; it moves no golden-core ROM) and that predicate
  is its own.
* `increment_coarse_x` and `increment_y` keep their `pub(super)`. Visibility is not a codegen
  input for an item that is crate-private either way, and the alternative is a `#[cfg]` on a
  visibility modifier for no measurable return. Their doc comments gain the feature's name.
* `captured_row_fine_y` and `next_row_fine_y`, the two accessors step 2 added, stay in both
  builds: they report fields that exist in both, and the default build's own test uses them to
  state the other half of the pair (both row fields follow `v`).

### The five things that have to move with it

1. **`zax-nes`'s `Cargo.toml`** — `ppu-2007-render-read = []`, out of `default`, with its own note.
   **Not** in `parked-boards`, for the reason `ppu-exact-sprite-eval` is not: that umbrella is for
   left-out *boards*.
2. **`zax-harness`'s `Cargo.toml`** — a pass-through, for the reason `vs-system` and
   `ppu-exact-sprite-eval` have one: Cargo cannot see a dependency's feature from a test, and the
   `AccuracyCoin` guard has to know which configuration it is scoring.
3. **`ppu_rendering_tests.rs`** — `req_core_112_a_2007_read_while_fetching_increments_coarse_x_and_y_at_once`
   and `amd_021_r2_005_a_2007_read_while_fetching_moves_the_next_lines_row_and_not_this_lines`
   behind the feature; **`req_core_112_a_2007_read_while_fetching_still_steps_by_the_ppuctrl_amount_q_125`,
   the test MF-4d-fix inverted, back in the default build**, with the row fields asserted beside
   the `(6, 8, 2)` so that the default build pins both halves and not one.
4. **`background_row_split_tests.rs`** — `smallest_gap`. MF-4e step 2 relaxed the bound from 3 to
   2 because the rule moves the banner's first row from line 195 to line 194, which puts lines 192
   and 194 two apart. With the rule off the geometry is `11407930`'s again, so **the bound is 3 in
   the default build** and 2 only behind the feature. Relaxing an assertion of a P1 requirement
   now costs nothing in the build the owner runs, which is the single best thing this gate does
   for Q-137.
5. **`test_roms_tests.rs`** — the guard's floor. Two floors become **three terms**: 128 is
   MF-4d-fix's, `ppu-2007-render-read` is worth +1 (`$2007 read w/ rendering`) and
   `ppu-exact-sprite-eval` +3 (`Address $2004 behavior`, `Arbitrary Sprite zero`, `Misaligned OAM
   behavior`). 128 / 129 / 131 / 132 for the four combinations, and each row named only in the
   configuration that ships it.

### What the figures are expected to be, so that a surprise is visible

| | `default` | `--features ppu-2007-render-read` |
| --- | --- | --- |
| `AccuracyCoin` | **128 / 147** | 129 / 147 |
| `cargo test --release -p zax-nes -p zax-harness` | 926 − 2 + 1 = **925** | **926**, MF-VS-fix's |
| golden-core, 98 commercial originals | **byte-identical to `11407930`** | MF-4e's three rows move |
| `amd_021_r2_005_smb3_…_at_the_split_line` | passes with `smallest_gap >= 3` | passes with `>= 2` |
| callgrind | **no higher than `11407930`'s** | MF-4e step 4's figures |

925 is `11407930`'s own count, which is the arithmetic sanity check: MF-4e added one test and
inverted one, so a build with the rule off is back to the baseline's number exactly.

## Step 1 — the gate, and the default build is `11407930` on this path

### The code: two `#[cfg]` bodies, and the off one is the baseline's five lines

`Ppu2C02::increment_vram_address` is now two functions under `#[cfg(feature =
"ppu-2007-render-read")]` and `#[cfg(not(...))]`. The off body is `11407930`'s **line for line** —
`git show 11407930:…/ppu/mod.rs` against it is the same five statements and the same two comments,
checked before the first build.

**Two bodies rather than one body with a `cfg!` in it**, and the reason is measured rather than
stylistic: MF-4d-fix step 5 and Q-125 established that *the presence of this branch* re-lays
`Ppu2C02::read` by about **+11 instructions per register read** — "a new arm with a body of its own
can flip a whole dispatch's shape", MF-3b rule 4. A gate that folds a constant `false` leaves the
arm in the source and asks LLVM to be the one that removes it; a gate that `#[cfg]`s it out means
the default build's token stream for this function *is* the baseline's. Step 5 measures whether
that was enough.

`run_sprite_evaluation` in `ppu/render.rs` already uses exactly this idiom for
`ppu-exact-sprite-eval`, so the file gains no new shape.

### Everything that moved, and the three things that deliberately did not

| file | change |
| --- | --- |
| `zax-nes/Cargo.toml` | `ppu-2007-render-read = []`, out of `default`, out of `parked-boards`, with its own note: what it changes, the row it wins, the figure, and **that R0 parked it for the picture and not the figure** |
| `zax-harness/Cargo.toml` | the pass-through, for the reason `vs-system` and `ppu-exact-sprite-eval` have one |
| `ppu/mod.rs` | the two bodies; `fetching`'s note (it serves the `$2004` quirk alone in `default`); the two row accessors' notes |
| `ppu/render.rs` | `increment_coarse_x` and `increment_y` keep `pub(super)`, and say why the visibility is not `#[cfg]`'d with the caller |
| `ppu_rendering_tests.rs` | the two tests that pin the rule behind the feature; **the inverted test back**, with the row fields asserted beside the `(6, 8, 2)` |
| `background_row_split_tests.rs` | `SMALLEST_GAP`, 2 behind the feature and **3 in `default`** |
| `test_roms_tests.rs` | the floor as a sum — 128, +1, +3 — and the `$2007` row named only where it ships |

Not gated: `Ppu2C02::fetching` itself (the `$2004`-write quirk of MF-4d step 6 is in every build and
that predicate is its own), the `pub(super)` on the two increment halves (crate-private either way,
so not a codegen input), and the `captured_row_fine_y` / `next_row_fine_y` accessors (they report
fields that exist in both builds, and the default build's own test now uses them).

### The suites, five configurations

| configuration | `cargo test --release -p zax-nes -p zax-harness` |
| --- | --- |
| `default` | **925 / 0 / 8** |
| `--features ppu-2007-render-read` | **926 / 0 / 8** |
| `--features parked-boards` | **985 / 0 / 8** |
| `--features ppu-exact-sprite-eval` | **925 / 0 / 8** |
| both PPU features | **926 / 0 / 8** |

MF-VS-fix's counts were 926 and 986, and **the −1 in each is accounted for exactly**: the gate
turns MF-4e's two tests into one in the default build (two behind the feature, the restored
inverted one without it), so the default build is back to `11407930`'s own **925** and the feature
build is MF-VS-fix's 926. `ppu-exact-sprite-eval` adds no test, which is why it is 925 alone and
926 with the other.

**One flake, named because it is not this lot's.** The first multi-threaded run of the
both-features configuration ended in `STATUS_ACCESS_VIOLATION` inside `test_roms_tests`'s
fourteen-test target. Re-run `--test-threads=1`: **14 passed**. Re-run multi-threaded: **926 / 0 /
8**, clean. Fourteen concurrent test-ROM drives is the condition, not the feature — the same target
passed multi-threaded in the four other configurations on the first attempt.

### `AccuracyCoin`: the floor's arithmetic, verified on all four combinations

| configuration | score | `PPU Misc. / $2007 read w/ rendering` |
| --- | ---: | --- |
| `default` | **128 / 147** | **`fail 2`** |
| `ppu-2007-render-read` | **129 / 147** | **`pass`** |
| `ppu-exact-sprite-eval` | **131 / 147** | `fail 2` |
| both | **132 / 147** | `pass` |

128 is MF-4d-fix's figure and `11407930`'s. 129, 131 and 132 are 128 + 1, + 3 and + 1 + 3, so the
guard's three-term floor is not an assumption but a measurement, and the 131 is new information:
MF-4e only ever reported 132 because step 2's rule was underneath step 3's.

### The ignored suite in the default build, and the assertion that comes back un-relaxed

`--ignored --no-fail-fast --test-threads=1` with `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set:
**7 of 8**, the one failure being `builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom`
— the known one the brief puts out of scope, which fails identically on the baseline.

Both tests the brief names pass:

* **`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`** — and it passes with
  `SMALLEST_GAP == 3`, not 2. **This is the best single thing the gate does.** MF-4e step 2 had to
  relax that bound from 3 to 2 and Q-137 had to ask R0 for the relaxation by name; with the rule
  out of `default`, the restatement of a **P1** requirement is back at full strength in the build
  the owner runs, and the relaxed form only exists in a build asked for by name.
* **`zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack`** — ZR-2's own
  regression test, pass.

**Save states**: no field added, removed or reordered; nothing new serialised; `save_state` and
`load_state` untouched. `NesCore::STATE_VERSION` stays at 8, and it has to: a feature that changed
the format would make the two builds' states incompatible, and this one does not touch a field.
No `unsafe`. `cargo fmt --check` clean and clippy `-D warnings` clean on `zax-nes` and `zax-harness`
`--all-targets` in **four** configurations (`default`, the new feature, `parked-boards`,
`ppu-exact-sprite-eval`), with no new `allow`.

## Step 2 — the row, and the project rule the lot actually adds

### `docs/review/BOARD-FEATURES.md`: one section for the two PPU features

The file's opening had one paragraph for "the one feature in this table that is not a board". There
are now two, they are out of `default` **for two different reasons**, and that difference is worth
more than a row. So the paragraph became a section, **"Two features in this table are not boards:
the PPU features"**, with a table each on the shape every board row in the file already uses —
what it wins, the figures on both runs, gated off, default, the command — plus a line the board
rows do not need:

| | `ppu-2007-render-read` | `ppu-exact-sprite-eval` |
| --- | --- | --- |
| what it wins | `AccuracyCoin` 128 → **129** | **+3 rows**, 128 → **131** |
| **what it changes on screen** | **4 golden-core ROMs**: SMB3, both Zelda IIs, `Duelito.nes` | **no commercial original**; 2 free ROMs, both test ROMs for the behaviour |
| PR-1's thirteen | **inside**: +0.000 % to +0.005 % | **above on nine of eleven**: up to +0.133 % |
| level 2 | every `core` +0.000 % to +0.002 % | **all ten above** |
| in `default` | **off** | **off** |
| why | **the picture, not the figure** | the figure |

Three things the section states that were not written down anywhere before:

1. **The four combinations are 128 / 129 / 131 / 132**, so the two features are independent and the
   guard's floor is a sum. MF-4e only ever reported 132 because step 2's rule sat underneath step
   3's; **131 is new information** this lot measured.
2. **The one command that gives both**, with `zax-harness` beside `zax-nes` and the reason the
   harness needs it (the `AccuracyCoin` guard has to know what it is scoring).
3. The paragraph under the board table that read "the worst figure of **the one feature** that is
   not in `default`" now says "the one **board** feature", because that sentence had already been
   false since MF-4e and this lot makes it two ways false. The heading above the MF-CART paragraph
   — whose "Both were parked on one instruction" meant `board-taito-x1` and `board-111` and would
   now read as the two PPU features — names those two boards instead.

### The ticket: **Q-139**, and it is not a question about behaviour

`G:/3dsen/cleanroom/spec/changes/Q-139.md`. It records two things and asks three.

**What it records.** First, a **project rule that is new**, because the owner's rule of 2026-11-06
is a *cost* rule and this feature is inside the tolerance:

> **A change that moves what the player sees in a game the owner watches waits for the owner's eye,
> whatever it costs.**

Orthogonal to the cost rule, and the two PPU features are now the two worked examples — one out of
`default` for the figure, one for the picture.

Second, **what the gate hands back to Q-137 without being asked**. MF-4e step 2 relaxed
`smallest_gap >= 3` to `>= 2` on a **P1** requirement's own test and Q-137 has been waiting on R0
for that relaxation ever since. With the rule gated, `SMALLEST_GAP` is **3 without the feature** and
2 with it, both measured passing: criterion 2 itself never moved, its geometric restatement is at
full strength in `default`, and the relaxed form only exists in a build asked for by name. Q-137
stays open on the substance — the reading of R2-005's exception is still an extension of the text
rather than the text — but **the urgency of its second request falls away**, because no relaxation
reaches the default delivery any more.

**What it asks**, none of it blocking: whether the project rule is right in those words and binding
on later lots; whether the feature should be renamed `ppu-2007-render-access`, since the
register-7 **write** borrows the same increment and the name follows the brief and `AccuracyCoin`'s
row rather than the code (the function's comment and the feature's row both say the write is
covered); and what happens next either way — if the owner validates the picture the feature joins
`default` and `SMALLEST_GAP` collapses back to a single 2, which puts Q-137 point 2 back in front;
if he invalidates it, the four-variant table measured on SMB3 is where to reopen.

## Step 3 — fmt, clippy and the suites in every configuration

### The matrix

| check | `default` | `ppu-2007-render-read` | `parked-boards` | `ppu-exact-sprite-eval` |
| --- | --- | --- | --- | --- |
| `cargo fmt --check` | clean | clean | clean | clean |
| clippy `-D warnings`, `zax-nes` + `zax-harness`, `--all-targets` | clean | clean | clean | clean |
| `cargo test --release -p zax-nes -p zax-harness` | **925 / 0 / 8** | **926 / 0 / 8** | **985 / 0 / 8** | **925 / 0 / 8** |
| `--ignored --no-fail-fast --test-threads=1`, both ROM folders | **7 of 8** | **7 of 8** | **7 of 8** | not run |
| `AccuracyCoin` | **128 / 147** | **129 / 147** | 128 / 147 | 131 / 147 |

`cargo fmt` is one check for the tree and not four — it does not read features — but it is listed
per column because clippy is genuinely four runs and the two are always reported together. The
fifth configuration, both PPU features, is **926 / 0 / 8** and **132 / 147**.

**No new `allow` anywhere.** The only thing clippy had to say about this lot was
`items_after_statements` on the restored test, where the second nested `fn` sat below the first
`assert_eq!`; both helpers now stand at the top of the body, which is the shape the file's other
tests use.

### The ignored suite, test by test

The same eight tests in all three configurations run, none skipped for a missing ROM:

| ignored test | `default` | `ppu-2007-render-read` | `parked-boards` |
| --- | --- | --- | --- |
| **`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`** | **pass**, `SMALLEST_GAP == 3` | **pass**, `SMALLEST_GAP == 2` | **pass**, 3 |
| **`zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack`** | **pass** | **pass** | **pass** |
| `golden_pixels_tiles_audio_cycles_and_blargg_match_up_to_frame_1200` | pass | pass | pass |
| `req_core_004_apu_reset_needs_a_reset_that_keeps_ram` | pass | pass | pass |
| `req_core_113_dmc_dma_during_read4_is_p3` | pass | pass | pass |
| `the_sprite_dma_and_dmc_dma_collision_is_not_cycle_exact` | pass | pass | pass |
| `the_cost_of_one_capture_and_one_restore` | pass | pass | pass |
| `builtin_patches::tests::the_zelda_ii_patch_applies_to_the_usa_rom` | **fail**, out of scope | **fail** | **fail** |

**R2-005's test passes on both sides of the gate**, which is the pair this lot had to keep: the
default build satisfies criterion 2 with the geometry `11407930` had and the bound at **3**, and the
feature build satisfies it with MF-4e's geometry and the bound at 2. The one failure in every
configuration is the known Zelda II patch-detection test, which the brief puts out of scope and
which fails identically on the baseline, on MF-4e's tree and on MF-VS-fix's.

And `zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack` — the test that
exists because of the widescreen symptom the owner reported against MF-4d — passes in **both**
configurations, so this lot's gate is not what the owner will be looking at when he validates: the
harness-level widescreen regression is clean either way, and what he is judging is the 58 and 18-19
frames R0's A/B found.

## Step 4 — golden-core is byte-identical to `11407930`, and the campaign is `11407930`'s to the field

One clean `git archive` build of **`11407930`** in its own directory with its own
`CARGO_TARGET_DIR`, deleted as soon as the binary was copied out, and three host binaries of this
tree (`default`, `ppu-2007-render-read`, both features) kept only for these runs.

### golden-core, 98 commercial originals, 1 200 frames

`golden-core /g/3DSen/test-originals <out>.jsonl 1200` — MF-VS-fix step 4's own set and frame count.

| pair | rows differing |
| --- | ---: |
| **`11407930` → this lot's `default`** | **0 — `diff` of the two JSONL files is clean** |
| this lot's `default` → `ppu-2007-render-read` | **11** |

**The first line is the lot's requirement and it is met at the strongest available strength**: the
two 98-row files are byte-identical, so `pixel_sha256`, `tiles_sha256`, the audio hash and the
cycle counts all agree on every one of the 98 ROMs over 1 200 frames each. golden-core runs with no
hack code, so there is nothing else the comparison could be measuring.

### The eleven rows the feature moves, and they are a sharper result than MF-4e's

Every one of them moves on `pixel_sha256` **and** `tiles_sha256`, and the set is:

| game | dumps that move |
| --- | ---: |
| **Super Mario Bros. 3** | **8** — Europe, Europe VC, Japan, Japan Rev 1, Japan Rev 1 VC, USA, USA Rev 1, USA Rev 1 VC |
| **Zelda II** | **3** — USA, Europe, Europe Rev 2 |
| every other commercial original | **0** |

MF-4e step 2 named three ROMs out of its 18-ROM set; on the 98-ROM set the answer is **every dump
of Super Mario Bros. 3 and every dump of Zelda II, all eleven of them, and no other commercial game
at all**. That is an independent confirmation of R0's A/B from the other direction: he sampled
frames of 5 SMB3 dumps and 2 Zelda II dumps against v0.4.2 and found 58 of 681 and 18-19 of 1 001
differing; golden-core hashes whole frames of all 98 originals and finds **exactly those two games
and nothing else**. The rule's blast radius is now bounded by measurement rather than by inference,
and it is bounded to the two games the owner named.

### The campaign, 331 rows, field by field

`test-roms` over `assets-free/test-roms`, compared on every field but `ms` (wall clock) and `shot`
(`--no-shots`), with the `AccuracyCoin` row's 147 `detail.tests` keys compared one by one:

| pair | summary objects | rows differing |
| --- | --- | ---: |
| **`11407930` → this lot's `default`** | **equal** | **0** |
| `11407930` → this lot's `ppu-2007-render-read` | equal | 2 |
| MF-VS-fix's `default` → this lot's `ppu-2007-render-read` | equal | **0** |
| MF-VS-fix's `default` → this lot's `default` | equal | 2 |

    331 ROMs, 214 pass, 5 fail, 0 timeout, 1 unsupported, 111 manual

in all of them. **The first and third lines are the two halves of what a gate has to prove**: with
the feature off the campaign is the baseline's, to the field, on all 331 rows — including
`Duelito`'s screen string and all 147 `AccuracyCoin` test keys; and with the feature on it is
MF-VS-fix's, to the field, on all 331 rows. The gate therefore selects between two campaigns that
already existed and introduces no third.

The two rows that do move between the configurations are the two MF-4e step 2 named, in the
opposite direction:

| row | `11407930` / this `default` | with the feature | why no verdict moves |
| --- | --- | --- | --- |
| `AccuracyCoin/AccuracyCoin.nes` | **128 / 147** | **129 / 147** | one `detail.tests` key of 147, `PPU Misc. / $2007 read w/ rendering` `fail 2` ↔ `pass`. The campaign's verdict for this ROM is `fail` either way, so no total moves |
| `nes-test-roms/other/Duelito.nes` | screen A | screen B | a **`manual`** row: the campaign records the screen and has no documented verdict for it. Duelito reads `$2007` while the PPU is fetching, which is the rule by name |

**No verdict falls or rises in any configuration**, which is why the four summary objects compare
equal as objects and not merely as totals.

## Step 5 — the callgrind bar: the emulated core is +0 on every figure of both runs

Five clean `git archive` builds in the WSL2 Ubuntu 24.04 (valgrind 3.22.0, rustc 1.99.1), each
extracted into a fresh directory with its own `CARGO_TARGET_DIR` and **deleted as soon as its
binary was copied out** (`tools/perf/build-clean.sh`). Driver `target-mf/mf4eg.sh`, MF-VS-fix's
`mfvsfix.sh` with the label prefix changed; scratch, never staged. All five labels are **16
characters**, which is MF-6a step 0's rule — equal path lengths keep `load_profile` and `lua_hooks`
comparable.

| label | what it is |
| --- | --- |
| `mf4eg-0-11407930` | the baseline the brief names |
| `mf4eg-1-6dd1bcf1` | **this lot's shipped default**, the rule gated off |
| `mf4eg-2-privvis0` | the same, with `increment_coarse_x`/`increment_y` back to **private** |
| `mf4eg-3-15c84cd5` | the **branch tip** this lot started from, whose default has the rule |
| `mf4eg-4-ruleonwt` | this worktree **with `ppu-2007-render-read` in `default`** |

### The bar as the brief states it: `11407930` → this lot's default

**PR-1's thirteen**, 300 frames, no warm-up, whole process:

| | figure |
| --- | --- |
| every ROM | **+0.000 % or +0.001 %**; six are *below* the baseline (`m0` −10 011, `oamstr` −12 518, `m1` −4 766, `m4` −3 014, `z2` −163) and the worst above is `mmc5` **+58 213 (+0.001 %)** |
| set total | 79 346 060 407 → 79 346 211 570, **+151 163 (+0.0002 %)** |
| `duelito` and `smb3` | **+4 850 and +30 049** — and this is a *behaviour* check, not a cost one: when the rule is in, MF-4e step 4 measured these two at **−4.022 % and +0.241 %**. At ±0.000 % they are emulating the same frames, which is the baseline's behaviour confirmed a third way after golden-core and the campaign |

**Level 2**, five games with their 3D packs, 600 frames after 120 of warm-up:

| column | result |
| --- | --- |
| **`core`** (`NesCore::run_frame`) | **+0 instructions on all five.** 11 146 862 384 / 12 037 146 109 / 11 515 129 187 / 15 314 947 822 / 12 774 928 372, identical to the instruction |
| **`ppu`** (`Ppu2C02::run_dots`) | **+0 on all five** |
| **`apu`** | **+0 on all five** |
| `draw_list` | +0 on four; `smb3` +2 203 |
| `load_profile`, `lua_hooks`, `convert` | move, in **both** directions |
| `total` | **−0.030 %** (`smb1`) and −0.022 % (`smb3`) *below*; +0.001 %, +0.001 % and +0.004 % above |

**The emulated core meets the bar exactly, at zero, on every figure of both runs.** `core`, `ppu`
and `apu` are identical to the instruction on all five games — fifteen figures, fifteen zeroes.

### Where the +151 163 is, named symbol by symbol

An all-symbol diff of the callgrind annotations on all thirteen ROMs. The movers are, in order of
size: `luaH_getshortstr`, `getgeneric`, `luaH_set`, `luaS_newlstr`, `mainpositionTV`, `luaH_newkey`
— **Lua's string hash table** — then libc's `__vfscanf_internal`, `____strtoul_l_internal`,
`_int_malloc`, `_IO_*`, `memcpy`/`memcmp`/`memset`, and Rust's float formatter.

**Filter Lua, libc and `core`/`std`/`alloc` out and the residue per ROM is +106 to +521
instructions** — on 3.3 to 11.3 **billion** — and it is `_IO_sputbackc` +88, `getdelim` +81,
`_IO_setb` +29, `pthread_getattr_np` +24, Lua's GC `reallymarkobject`, and one
`zax_settings::catalogue` hash-map lookup of a few dozen. The first six of those carry the **same
value on every one of the thirteen ROMs**, which is what a fixed load-time cost looks like and not
emulation.

**Not one symbol of the emulated core moves anywhere**, and the eleven pinned symbols are identical
to the instruction on `mmc5`, `smb3`, `m4`, `vblnmi` and `readbuf`:

| | baseline | this lot's default |
| --- | ---: | ---: |
| `Cpu6502::step` / `branch` / `resolve` (`mmc5`) | 419 548 544 / 163 582 727 / 61 863 835 | **identical, 0 / 0 / 0** |
| `Ppu2C02::read_pattern_planes` (`mmc5`, `readbuf`) | 96 388 300 / 88 146 700 | **identical** |
| `Ppu2C02::run_dots` (`mmc5`) | 3 104 067 813 | **identical** |
| `Ppu2C02::run_sprite_evaluation`, `background_fetch`, `paint_sprite`, `rebuild_sprite_line` | — | **identical on all five ROMs** |
| **`Ppu2C02::read`** (`mmc5`, `vblnmi`) | 287 286 413 / 23 513 461 | **identical** |
| `WideExtension::zone_offset` | not a symbol of its own | **still not one** — inlined into the fetch path, as every earlier lot found |

**`Ppu2C02::read` is the one that mattered and it did not move.** MF-4d-fix step 5 and Q-125
measured that taking this branch out costs that function about **+11 instructions per register
read**, +0.161 % on `readbuf`; it is the reason this lot used two `#[cfg]` bodies rather than a
folded `cfg!`. The honest result is that **the precaution was unnecessary**: the function is
identical to the instruction with the branch and without it, in all four builds. MF-4e step 4 had
predicted exactly this and said why — MF-5a's `#[inline(never)]` on `Ppu2C02::read_data` has since
insulated the dispatch, so the layout no longer hinges on that arm having a body. The two bodies
cost nothing and are still the right shape (the default build's token stream for the function is
the baseline's), but the +11 instructions are not a live risk any more and this is the measurement
that says so.

### And the residue is not this lot's — proved with a same-behaviour pair

`11407930` is **ten commits** behind this lot: MF-4e's six, MF-VS-fix's seven minus the overlap. So
that comparison measures this lot *plus* everything in between. To isolate this lot, a pair whose
**behaviour is identical**: the branch tip `15c84cd5`, whose `default` has the rule, against this
worktree **with `ppu-2007-render-read` put into `default`** — same emulation, so the only difference
is this lot's own source.

| | figure |
| --- | --- |
| set total | 79 068 925 559 → 79 068 894 622, **−30 937 (−0.000 %)** — *below* the tip |
| every ROM | within **±57 758**, all +0.000 % / −0.001 %, and it moves **both ways**: six faster (`duelito` −57 758, `m1` −18 415, `m4` −15 486, `readbuf` −12 389, `smb3` −26 163), seven slower (worst `z2` +35 435) |
| pinned symbols | **identical to the instruction** on `mmc5`, `smb3` and `vblnmi`, `Ppu2C02::read` included |
| non-Lua/libc residue | **−148** on `mmc5` and **−169** on `z2` — negative |

`duelito` −57 758 and `smb3` −26 163 are the behaviour check for this pair too: with the rule on
both sides they are at ±0.001 %, where a behaviour difference would put them in the millions.

**So this lot's own code is at or below the branch tip, and the +151 163 against `11407930` belongs
to the ten commits in between** — which is the same Lua-hash-table lottery MF-6a, MF-6b, MF-6c and
MF-CART each attributed by name, moving in both directions and landing on no symbol of this
project.

### The visibility question, settled by byte-identity rather than by argument

Step 1 kept `pub(super)` on `increment_coarse_x` and `increment_y` unconditionally and the doc
comment claims visibility "is not a codegen input for an item that is crate-private either way".
`mf4eg-2-privvis0` is that claim built: the same tree with both functions back to private.

    7781224  03e1627053753fda  zax-harness-mf4eg-1-6dd1bcf1
    7781224  03e1627053753fda  zax-harness-mf4eg-2-privvis0   <- cmp: IDENTICAL

**Byte-identical, same SHA-256.** So the unconditional `pub(super)` costs exactly zero bytes and
zero instructions, and the comment is a measurement now rather than a reasonable belief.

That probe also refuted this lot's first guess at the 96-byte size difference from `11407930`
(7 781 128 → 7 781 224): it is **not** the visibility, and it is not a string-table change either —
of about 30 000 strings in each binary, **6** differ, and all six are machine-code byte runs that
happen to be printable, with no source path and no panic location among them. It is the ten
commits, and the same-behaviour pair above is what locates it there.

## What this lot leaves open

1. **The owner's eye**, which is the whole point: he validates Super Mario Bros. 3 and Zelda II and
   then either the feature joins `default` — in which case `SMALLEST_GAP` collapses back to a
   single 2 and **Q-137 point 2 is in front again** — or MF-4e step 2's rule is reopened, and the
   four-variant table measured on SMB3 (the doc comment on
   `Ppu2C02::increment_vram_address`, and `MF-4e.md` step 2) is where to start.
2. **Q-139**: the project rule in those words and binding on later lots; and the name, which says
   "read" for an access rule.
3. **Q-137 on the substance** — the reading of R2-005's exception as being about the nature of the
   increment rather than its instant is still an extension of the text. Only the urgency of its
   second request has gone.
4. **Nothing else of MF-4e moved.** `ppu-exact-sprite-eval` keeps its figures and its place out of
   `default`; `board-vs-system` stays parked; MF-4e's six unwon `AccuracyCoin` rows and the
   `OAM Corruption` `$2003`-write mechanism are still that lot's open list, untouched here.

## Step 6 — the journal, and the final pass on the committed tree

`G:/3dsen/cleanroom/journal/JOURNAL.md` gains the lot's entry: role, date, base and reference
commits, folders consulted, why the lot exists in R0's own figures, the six steps, what the gate
hands back to Q-137, the eleven golden-core rows, **the two negative results**, the disk-space
note, the save-state statement and the open items.

Re-run on the committed tree, nothing uncommitted but `rust/target-mf/` (scratch, never staged):

| configuration | `cargo fmt --check` | clippy `-D warnings` | `cargo test --release -p zax-nes -p zax-harness` |
| --- | --- | --- | --- |
| `default` | clean | clean | **925 passed, 0 failed, 8 ignored** |
| `--features ppu-2007-render-read` | clean | clean | **926 passed, 0 failed, 8 ignored** |
| `--features parked-boards` | clean | clean | **985 passed, 0 failed, 8 ignored** |
| `--features ppu-exact-sprite-eval` | clean | clean | (step 3: 925 / 0 / 8) |

### Disk and machine

`rust/target-mf/debug` — **18 GB** of rebuildable cargo scratch left by an earlier session, 7.8 GB
of it `incremental` — was deleted at the start, which took G: from **17 GB** free to 35 GB. It
never went below **33 GB** afterwards, well above the brief's 5 GB floor. Five clean `git archive`
builds were made, each in a fresh directory with its own `CARGO_TARGET_DIR` and each deleted as
soon as its binary was copied out; the callgrind annotations, the WSL scratch root and the host's
probe binaries and tars were deleted as their figures were recorded. **No GPU campaign of any
kind.**

### The lot in one line

**The default build renders this path exactly as `11407930` does** — golden-core byte-identical
over 98 commercial originals at 1 200 frames, the campaign's 331 rows identical field by field,
`AccuracyCoin` back at 128, R2-005's bound back at 3, and the emulated core at **+0 instructions on
all fifteen figures of both callgrind runs** — **and one flag gives the rule back.**
