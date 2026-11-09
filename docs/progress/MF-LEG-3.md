# MF-LEG-3 — the MMC5's two substituting modes, armed at run time

Role R3, clean side. Base commit **`04731c86`** (`BOARD-FEATURES: board-mmc5-exattr-split stays
parked …`), branch `mapper-review`, worktree `wt-mappers`.

Sources consulted: `docs/progress/MF-LEG-2.md` (the whole lot, above all **step 4** — the seven
measured shapes, the one symbol the overage comes from and why the feature was parked), the
banner and the `board-mmc5-exattr-split` code of `rust/crates/zax-nes/src/cartridges/mappers/
mmc5.rs`, the `Board` methods MF-LEG-2 added in `cartridges/board.rs`, the note of 2026-11-09 at
the top of `docs/review/BOARD-FEATURES.md`, and the nesdev *MMC5* page for `$5104`, `$5200`-`$5202`.

## The problem this lot solves

With `board-mmc5-exattr-split` on, **every** MMC5 game pays for the two modes whether or not it
arms them: *Castlevania III* (USA) at **+2.992 %** (R0, 600 frames of `bench-core`) and the PR-1
ROM `mmc5` at +2.522 %, and every non-MMC5 game up to **+0.110 %** (`metroid` `core`) through the
one `bool` that grew the MMC5's arm of `Board::ppu_peek` / `Board::ppu_read`. The owner's decision
of 2026-11-09 is to **switch at run time**: a game cannot use either mode without first writing
`$5104` with mode %01 or `$5200` with bit 7 set, and both writes go through the MMC5.

## Steps

| # | step | the brief's points |
| --: | --- | --- |
| 1 | the two variants, the switch, and the light board back to `04731c86`'s code | 1, 2, 3, 6 |
| 2 | the mid-frame arming, the save-states and the new tests | 4, 5, tests |
| 3 | golden-core against the base commit, and the suites with both ROM roots | the final check |
| 4 | the callgrind campaign | the acceptance figures |
| 5 | `BOARD-FEATURES.md`, the `Cargo.toml` note and the feature decision | — |
| 6 | the final check | the final check |

- [x] 1/6 the two variants and the run-time switch

### 1/6 — the two variants, the switch, and the light board back to `04731c86`'s code

**The shape, and why this one.** The brief offers two (`Board::Mmc5Ext`, or a const-generic
`Mmc5<const EXT: bool>` behind two variants) and asks for the shapes tried to be recorded. Three
were weighed on paper and one of them decided the lot before a line was written:

| shape | why it was or was not taken |
| --- | --- |
| `Mmc5<const EXT: bool>`, both variants **unboxed** | **rejected on `Board`'s layout.** `Board`'s tag lives in a *niche* inside the MMC5 variant (the enum's own doc comment: `movzbl 1355(%rcx)` in every dispatch prologue) because that variant is the largest. A second unboxed 1.4 KiB variant overlaps the niche and rustc falls back to a tagged layout — which rewrites **every** dispatch prologue of the file, for all thirty-seven arms. That is MF-2 step 0's reshape, bought for nothing. |
| `Mmc5<const EXT: bool>`, the `true` variant **boxed** | layout-safe, but the `false` instantiation still carries the ten fields the modes need (a `#[cfg]` cannot follow a const generic), so the light board is not `04731c86`'s struct and its savestate, `new` and `reset` all grow. |
| **`Mmc5Ext { base: Mmc5, …10 fields }` in `Board::Mmc5Ext(Box<Mmc5Ext>)`** | **shipped.** `Mmc5` is `ca1517dc`'s file restored byte for byte (`git show ca1517dc:…/mmc5.rs`, then the lot's additions appended below it), so the light board is the base commit's code by construction and not by inspection; the full board *contains* it, so forty-one `Mapper` methods minus seven are a forward and the MMC5's PRG decode, sound, IRQ, scanline detection, bank windows and savestate format have **one** implementation. |

**The layout is measured, not assumed**: `size_of::<Board>()` is **1376** in `default` and 1376
with the feature on, equal to `size_of::<Mmc5>()` in both — so the niche-filling layout survives
the thirty-seventh variant and the enum has not grown by a byte. The cost of that variant is one
more `Board` discriminant, which **MF-6c step 6 measured alone**: `mmc5` −40 814 instructions over
300 frames, PR-1's set total −136 318, worst of thirteen +0.000 %.

**What the light board gains: nothing.** `mmc5.rs` is `ca1517dc`'s 792 lines with the ext section
appended; the only edits inside them are the struct's doc comment, two `#[cfg]`'d `use`
statements and the `pub const EXATTR_SPLIT` the both-configuration test reads. **No field, no
register, no write arm, no line of `chr_address`, `see_ppu_read`, `save_registers` or
`load_registers`.**

**The switch, and the three places a variant can change.**

1. `Board::intercept_cpu_write`, **inside the MMC5's own arm** — the hot one. `dispatch!` now
   expands `dispatch_arms!`, one list of thirty-seven arms with an override hook for
   `Board::Mmc5`, so no other arm and no path before or after the dispatch changes: MF-LEG-2
   step 4 priced one `bool` on the cartridge's common write path at **+610 864 on `smb3`**, and
   this function is `NesBus::write_internal`'s own body. The test is
   `Mmc5Ext::write_arms_substitution(address, value)` and the construction behind it is
   `#[inline(never)]`, so none of the full board's 1.4 KiB lands in that frame.
2. `Board::load_registers`, after the dispatch (one compare, once per state load).
3. `Board::reset`, after the dispatch — the only one that goes **back**: power-on and reset both
   start on the light variant.

**Which writes arm it.** `$5104` with mode %01 and `$5200` with bit 7 set are the owner's two.
**`$5201` and `$5202` arm it as well**, and that is this step's one decision beyond the brief: a
light board has nowhere to keep them, so a game writing "bank, scroll, then enable" — the natural
order, and the order a Koei title or *Uchuu Keibitai SDF* would use — would otherwise lose two
registers at the moment it enabled the split. `$5200` with bit 7 **clear** does not arm anything
and loses nothing: its other bits only mean something while bit 7 is set, and the write that sets
bit 7 carries them. `$5104` = 0, 2 and 3 do not arm it either.

**The savestate, and why the block is the same length in both variants.** The three registers ride
in the spare bits of the `i32` that carries `$5104`'s two (`Mmc5Ext::packed_ex_ram_mode`): that
register only ever holds 0 to 3, so bits 31-8 have been written as zero by every build there has
ever been. The light board stores the `i32` whole — `04731c86`'s code, untouched — and
`Board::load_registers` reads it back off the board to decide the variant. So **a state written by
any build loads**, packs' idle savestates included, which three bytes appended after the sound
block (MF-LEG-2's shape) could not do. Step 2 has the round-trip tests.

**What this step does not do**: the mid-frame analysis, the new tests and the figures. The
twenty-three MF-LEG-2 tests already pass **through the switch** — every one of them arms a mode by
writing `$5104` or `$5200`-`$5202` through the CPU bus — which is the first evidence that the
full variant is reached and complete.

| check at this step | result |
| --- | --- |
| `cargo fmt -p zax-nes -p zax-harness --check` | clean |
| clippy `--all-targets -- -D warnings`, `default` / feature / `parked-boards` / `--no-default-features` | clean |
| `cargo test --release -p zax-nes -p zax-harness`, `default` | **1 167 passed, 0 failed** |
| the same, `--features zax-nes/board-mmc5-exattr-split` | **1 190 passed, 0 failed** |
| the same, `--features zax-nes/parked-boards` | **1 251 passed, 0 failed** |
| `mf_leg_2_mmc5_exattr_split_tests` with the feature on | **23 passed, 0 failed** |

The three totals are MF-LEG-2 step 6's own (1 167 / 1 190 / 1 251) to the test: this step moves no
behaviour that any existing test can see.
- [x] 2/6 the mid-frame arming, the save-states and the new tests

### 2/6 — the mid-frame arming, the save-states, and sixteen tests

`mf_leg_3_mmc5_runtime_switch_tests.rs`, **sixteen tests**, all behind the feature. The variant is
read off the cartridge (`NesCore::cartridge_mut` and `Cartridge::board`, both already `pub`) and
**not** inferred from the picture, because the switch is designed to be invisible from outside: no
register is lost by a board that has not switched, so "did it switch?" has no answer in the
frame. What the two modes *do* once armed is MF-LEG-2's twenty-three tests, which now all reach
the full variant **through** this switch.

| what is pinned | tests |
| --- | --- |
| it switches on `$5104` = %01, on `$5200` bit 7, and on `$5201` / `$5202` | 3 |
| it does **not** switch on `$5104` = 0, 2, 3, on `$5200` with bit 7 clear (three values), or on nine other MMC5 writes — fifteen writes, each on a fresh core — nor after four frames | 2 |
| no write ever takes it back; a reset does, and it arms again afterwards | 2 |
| the full variant with neither mode armed renders **exactly** as the light one | 1 |
| the savestates: equal block length, the light round trip, a full state into a fresh core, each of `$5200`-`$5202` on its own, the variant after a load, the core-state path, run-ahead | 7 |
| arming **mid-frame** | 1 |

**The equivalence test is the one that guards the one body of `mmc5.rs` that is a copy.**
`Mmc5Ext::see_ppu_read` has to be a copy of the light board's — the per-tile latching sits between
the fetch that raises the in-frame flag and the repeat counter that consumes the last read
address — and no comment keeps two copies in step. So
`mf_leg_3_the_full_variant_renders_like_the_light_one_with_neither_mode` runs twenty frames with a
moving scroll through a board that has switched (armed, then disarmed, so every register is the
light board's again) and one that has not, and compares **every pixel of every frame**, `$5204`
after each, and the CPU cycle count. It uses **8x16 sprites and two different CHR register sets**,
so the in-frame flag the scanline detection raises decides which set the background is fetched
from (MR-5-1) and a drift in the detection is a drift in the picture rather than a silent one.

#### The two state paths, and what each does to the variant

The brief's point 5 asked for a round trip in each variant and across a switch. Writing them
turned up a distinction worth recording, because the code was documented wrongly until the test
failed:

| path | what decides the variant |
| --- | --- |
| **savestate** (`NesCore::try_load_state`) | the **block alone**. `restore_state` builds a *new* `Cartridge` from the ROM and loads into it, so the live board is never consulted: the board arrives light and `Board::load_registers` promotes it if the block says a mode is armed. The variant is a pure function of the state, which is what makes rewind and run-ahead deterministic. |
| **core state** (`capture_core_state` / `restore_core_state`, rewind's and run-ahead's own path, REQ-LOOP-067) | the **live board, then the block**. It loads into the cartridge already attached, so a light board is promoted if the block says armed and a full board **stays full** — correct, since the full variant renders a both-modes-off state exactly as the light one does. |

So "it never switches back" is the rule for **writes**, and the first draft's comments that stated
it unconditionally are corrected in `mmc5.rs`, `board.rs` and here. Both paths are tested, in both
directions.

**Why the block is the same length in both variants.** The split's three registers ride in the
spare bits of the `i32` that carries `$5104`'s two: that register only ever holds 0 to 3, so bits
31-8 have been written as zero by every build there has ever been, and a light board stores the
word whole with `04731c86`'s own code. MF-LEG-2's shape — three bytes after the sound block — makes
the block three bytes longer than the default build's, and then **no existing state loads**, which
is what the owner's point 5 forbids. `mf_leg_3_each_split_register_survives_a_round_trip_on_its_own`
puts each of the three through a frame, a save and a load on its own, so the packing cannot have
crossed two fields and passed.

#### Arming mid-frame: exactly one line, and which one

The brief's point 4. The light board does not keep the split's column, its row, mode 1's `ExRAM`
index or the run of pattern fetches that finds the start of a line's fetch sequence, so a board
that switches **inside** a line starts all four at zero. What that costs:

* `ex_attr_index` is re-derived by the **next nametable fetch** — within one tile, which is what
  its own comment promises;
* `chr_reads_since_nt` starts at 0, so no row start is recognised until the **sixteen sprite
  pattern fetches of dots 257-320** push it past four and the nametable fetch at **dot 321** — the
  first of the next line's sequence — reloads the row and puts the column back before 0;
* so the tiles of the arming line fetched after the write are drawn with a column counter
  restarted at 0 and a row of 0, and **every line from the next one on is exact**. That is the
  "a line re-derives it" of `split_column` and `chr_reads_since_nt`, met exactly.

`mf_leg_3_arming_mid_frame_costs_one_line_and_no_more` measures it with three cores on the same
set-up — one that never arms, one that arms before the frame, and one whose **own 6502 program**
arms it, polling `$5204` with the IRQ line down and storing `$5200` the moment the scanline
counter reaches `$5203` = 60. The frame it produces:

| lines | what they are |
| --: | --- |
| 0-59 | the never-armed picture, **to the pixel** |
| **60** | the arming line, and the only line that is neither picture |
| 61-239 | the armed-before-the-frame picture, **to the pixel** |

The test finds that line from the picture rather than assuming it and then asserts the three
statements, so a scanline detection that moved by a line would fail on the figure and not on the
structure. Line 60 rather than 59 is the flag's own line plus the few CPU cycles the poll loop and
the store take.

**And the case a real game produces is better than the worst case this test takes.** An MMC5 game
arming the split from a scanline IRQ handler writes in **HBlank** — dots 257 to 320, after the
line's own fetches and before the dot-321 reload — and then not even that line is wrong. The test
polls instead, so the write lands inside the visible dots: the one line is the bound, not the
expectation.

| check at this step | result |
| --- | --- |
| fmt, and clippy `--all-targets -- -D warnings` in the four configurations | clean |
| `cargo test --release -p zax-nes -p zax-harness`, `default` | **1 167 / 0** |
| the same, `--features zax-nes/board-mmc5-exattr-split` | **1 206 / 0** (+16) |
| the same, `--features zax-nes/parked-boards` | **1 267 / 0** (+16) |
- [x] 3/6 golden-core and the suites with both ROM roots

### 3/6 — golden-core over the 98 originals, and the suites with both ROM roots

```
zax-harness golden-core G:/3dsen/test-originals gc98-<label>.jsonl 1200
```

The base commit was extracted with `git archive 04731c86` into a **fresh** directory with its own
`CARGO_TARGET_DIR`, built there, and the directory deleted as soon as the figures were taken.

| build | md5 of the 98-ROM digest | `cmp` against the base commit's file |
| --- | --- | --- |
| `04731c86`, the base commit, fresh tree | `f79e7df842c3ee43e7d9b67861a927b5` | — |
| this tree, `default` | `f79e7df842c3ee43e7d9b67861a927b5` | **identical** |
| this tree, `--features zax-nes/board-mmc5-exattr-split` | `f79e7df842c3ee43e7d9b67861a927b5` | **identical** |
| this tree, `--features zax-nes/parked-boards` | `f79e7df842c3ee43e7d9b67861a927b5` | **identical** |

**Byte-identical in all four**, `cmp` on the files and not only on the digests, and identical to the
digest fifteen lots have now recorded (it is MF-LEG-2 step 3's own). 98 ROMs run, 98 files.

The 98 use mappers 0, 1, 2, 3, 4 and 156 — **none of them is mapper 5** — so what this row rules
out is that the lot touched a shared path: the thirty-seventh `Board` variant, the
`dispatch_arms!` rewrite of all thirty-seven arms, the new arm of `Board::intercept_cpu_write` and
the two post-dispatch tests in `load_registers` and `reset`, all at once.

#### `mmc5exram`, the PR-1 MMC5 ROM, **never arms either mode**

Measured, not assumed, and it decides the shape of step 4's figures: the ROM was run for 300
frames — the campaign's own length — with the variant read off the cartridge after each, and it is
**`Board::Mmc5` the whole way**. So the ROM that MF-LEG-2 measured at **+2.522 %** runs
`04731c86`'s code from power-on to the last frame, and its figure should be the base's.

That is `exram`'s own subject matter: it tests the `$5C00`-`$5FFF` **CPU-side** modes of `$5104`
(%10 and %11, where `ExRAM` is the CPU's RAM) and the `ExRAM`-as-nametable mode %00, none of which
is extended attributes, and it never touches `$5200`-`$5202`. The three MMC5 ROMs of the
collection keep their outcomes in all three configurations
(`req_core_100_the_mmc5_rom_suites_keep_their_outcomes`, in the runs below).

#### The suites

`ZAXNES_ROMS=G:/3dsen/demo/roms`, `ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`,
`--no-fail-fast -- --include-ignored`:

| run | result | against MF-LEG-2 step 6 |
| --- | --- | --- |
| `default` | **1 177 passed, 0 failed, 0 ignored** | unchanged |
| `--features zax-nes/board-mmc5-exattr-split` | **1 216 passed, 0 failed, 0 ignored** | +16 |
| `--features zax-nes/parked-boards` | **1 277 passed, 0 failed, 0 ignored** | +16 |

**All pass; nothing fails and nothing is skipped.** The +16 are this lot's own tests and the
`default` row is identical, which is the suite's statement of what the feature-off build is.
- [ ] 4/6 the callgrind campaign
- [ ] 5/6 the documentation and the decision
- [ ] 6/6 the final check
