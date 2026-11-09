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
- [ ] 2/6 the mid-frame arming, the save-states and the new tests
- [ ] 3/6 golden-core and the suites with both ROM roots
- [ ] 4/6 the callgrind campaign
- [ ] 5/6 the documentation and the decision
- [ ] 6/6 the final check
