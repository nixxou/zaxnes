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
