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
- [ ] 3/5 golden-core and the full test run
- [ ] 4/5 the callgrind campaign
- [ ] 5/5 the final check
