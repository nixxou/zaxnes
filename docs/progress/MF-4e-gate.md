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
