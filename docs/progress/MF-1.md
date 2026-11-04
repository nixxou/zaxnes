# Lot MF-1 — the zero-cost mapper fixes of the MR-1/MR-1b review

Role R3 (clean-side implementer), worktree `wt-mappers`, branch `mapper-review`.

Source of truth for the findings: `docs/review/MR-1-mappers.md`, section
**"## MR-1b — the authoritative findings table"** (the MR-1 sections before it are superseded
wherever MR-1b disagrees). The failing tests live in
`rust/crates/zax-nes/tests/mr1_review_tests.rs`.

Build environment used for every cargo command (the C: drive is nearly full):

    TMP=<worktree>/rust/target-mf/tmp  TEMP=<worktree>/rust/target-mf/tmp
    CARGO_TARGET_DIR=<worktree>/rust/target-mf

`zax-app` cannot build in this worktree (its demo ROM is git-ignored), so clippy is scoped to
`-p zax-nes`.

## Steps

- [x] 1/10 — Caltron 6-in-1: MR-41-1, MR-41-2, MR-41-3
- [ ] 2/10 — MMC6: MR-4-1, MR-4-7, MR-4-6, MR-4-8
- [ ] 3/10 — RAMBO-1 PRG mode: MR-64-7
- [ ] 4/10 — MMC5: MR-5-1, MR-5-3, MR-5-5
- [ ] 5/10 — MMC1: MR-1-1, MR-1-3, MR-1-4, MR-1-5
- [ ] 6/10 — VRC2/VRC4: MR-21-1, MR-21-2, MR-21-3
- [ ] 7/10 — mapper 34: MR-34-1 and the work-RAM half of MR-34-2
- [ ] 8/10 — MR-32-1, MR-156-1, MR-16-1
- [ ] 9/10 — MR-13-1, MR-13-2, MR-15-1, MR-225-1, MR-4-9, MR-30-1, MR-30-2, MR-30-3, MR-30-5
- [ ] 10/10 — full suite, the `--ignored` list, the R0 table and the review annotations

## Out of scope (lot MF-2 or later), left untouched

MR-BUS-1, MR-66-1, MR-64-3, MR-64-4, MR-5-2, MR-5-4, MR-30-4, MR-30-6, MR-4-3 (MC-ACC is a
new board), the three remaining unknowns (MR-64-1, MR-64-5, MR-18-1), and everything under
Q-110, Q-111 and Q-112 (mapper 15's banking modes, mappers 28 and 101, MMC5 audio, the reset
state): those wait for amendment AMD-061. The MR-1 fixes MR-1b withdrew (MR-64-1, MR-64-2,
MR-11-1, MR-4-2 and the rest of its "Left `our bug`" table) are **not** applied.

## Findings → fix → test → games (for R0)

The `Fixed in MF-1 (<hash>)` lines of `docs/review/MR-1-mappers.md` are appended in step 10,
because a step cannot write its own commit hash.

| finding | fix (file:line) | test | commercial games it changes |
| --- | --- | --- | --- |
| MR-41-1 | `multicart.rs:264` (`outer & 0x07`) | `mr_41_1_caltron_prg_field_is_three_bits` | Caltron 6-in-1 (games 5-6 become reachable) |
| MR-41-2 | `multicart.rs:267` (`outer & 0x20`) | `mr_41_2_caltron_mirroring_comes_from_address_bit_5` | Caltron 6-in-1 |
| MR-41-3 | `multicart.rs:318` (gate on `outer & 0x04`) | `mr_41_3_caltron_inner_chr_is_gated_on_the_prg_bank` | Caltron 6-in-1 |

### Step 1 notes

- The outer register now keeps six address bits instead of five
  (`multicart.rs:305`); the saved state is still the same two bytes, so a state written
  before this lot loads unchanged — only the meaning of the bits in `outer` moves, which is
  the fix itself.
- `req_core_100_caltron_takes_the_prg_bank_from_the_address_and_the_chr_from_the_data`
  (`mapper_tests.rs:701`) encoded the old layout in its address and had to be re-written to
  the `..MC CPPP` one; it asserts the same windows as before. No other test touches mapper 41.
- `SPEC-02-coeur-nes.md:414` lists mapper 41 with an empty behaviour column, so REQ-CORE-100's
  "comportement documenté sur nesdev" governs and no Q ticket is needed.
