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
| **`Mmc5Ext { base: Mmc5, …10 fields }` in `Board::Mmc5Ext(Box<Mmc5Ext>)`** | **this step's choice, and step 4 took the box off it** (the campaign measured the boxed form at `metroid` +0.165 % and the unboxed at +0.010 %, the niche costing more than it saves); the struct and the switch are otherwise what shipped. `Mmc5` is `ca1517dc`'s file restored byte for byte (`git show ca1517dc:…/mmc5.rs`, then the lot's additions appended below it), so the light board is the base commit's code by construction and not by inspection; the full board *contains* it, so forty-one `Mapper` methods minus seven are a forward and the MMC5's PRG decode, sound, IRQ, scanline detection, bank windows and savestate format have **one** implementation. |

**The layout is measured, not assumed**: with the variant boxed, `size_of::<Board>()` is **1376**
in `default` and 1376 with the feature on, equal to `size_of::<Mmc5>()` in both — so the
niche-filling layout survives the thirty-seventh variant and the enum does not grow by a byte.
The cost of that variant was expected to be one more `Board` discriminant, which **MF-6c step 6
measured alone** at `mmc5` −40 814 instructions over 300 frames and worst of thirteen +0.000 %.

**Both halves of that paragraph turned out to be the wrong way round, and step 4 is where.** One
more discriminant is *not* free at thirty-seven (the control: `mmc5` +0.132 %, `metroid` +0.089 %),
and keeping the niche is *not* worth it (the prologue is four instructions with it and one
without). The shipped shape is the unboxed variant with the two MMC5 arms sharing one match arm:
`size_of::<Board>()` **1392** with the feature on, PR-1's worst ROM **+0.010 %** and `mmc5`
**−0.059 %**. Step 4.3 has the four shapes and their figures.

**What the light board gains: nothing.** `mmc5.rs` is `ca1517dc`'s 792 lines with the ext section
appended; the only edits inside them are the struct's doc comment, two `#[cfg]`'d `use`
statements and the `pub const EXATTR_SPLIT` the both-configuration test reads. **No field, no
register, no write arm, no line of `chr_address`, `see_ppu_read`, `save_registers` or
`load_registers`.**

**The switch, and the three places a variant can change.**

1. `Board::intercept_cpu_write`, **inside the MMC5's own arm** — the hot one. `dispatch!` now
   expands `dispatch_arms!`, one list of arms whose MMC5 clause each caller passes in (step 4
   turned that into the two forms `dispatch!` and `dispatch_ext!`), so no other arm and no path
   before or after the dispatch changes: MF-LEG-2
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
- [x] 4/6 the callgrind campaign

### 4/6 — the callgrind campaign, and the four shapes it took

Binaries built by `rust/tools/perf/build-clean.sh` from their own tar into a **fresh** directory
with its own `CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`. Each feature variant
was made from its own shipped tar by adding one line to zax-nes's `default` list.

| label | tree |
| --- | --- |
| `leg3-base` | **`04731c86`**, the lot's base commit |
| `leg3-doc` | **the committed tree**, the feature out of `default` — the figures' own shipped build |
| `leg3-docfeat` | the committed tree plus **`board-mmc5-exattr-split`** in `default` |
| `leg3-x37` | the shipped tree plus a thirty-seventh `Board` variant **never constructed**, the control |
| `leg3-feat`, `leg3-unbox`, `leg3-sharefeat`, `leg3-pinfeat` | the four shapes of 4.3 |

#### 4.0 The shipped build **is** the base commit's binary, md5 for md5

| binary | md5 |
| --- | --- |
| `leg3-base` (`04731c86`) | `104030539c326d44d2ac4f6a2b088e79` |
| `leg3-doc`, the committed tree with the feature off | `104030539c326d44d2ac4f6a2b088e79` |
| `leg2-ship`, **MF-LEG-2's own measured shipped binary** | `104030539c326d44d2ac4f6a2b088e79` |

**The whole file, not `.text` alone**, which is a stronger statement than MF-LEG-2 could make (its
shipped build differed from its base by 37 bytes of build-id and `panic::Location` lines). Nothing
of this lot is compiled with the feature off — not a field, not a variant, not a macro arm — so
"with the feature off: identical to the base" needs no ROM run at all, and the third row says the
lot's base is the previous lot's measured binary, the **fourteenth** lot in a row for which that
holds.

**The figures below are the committed tree's**, re-taken on it rather than carried over from the
build that settled the design: the tree was tarred after `cargo fmt` *and* after the documentation
was written, built again (`leg3-doc`, feature off, md5 unchanged by the comments;
`leg3-docfeat`, feature on, md5 `39f30e0b6144e712ef0bd864daa809bf`), and both the thirteen-ROM set
and the level-2 run were re-run on it. The build that settled the design gave `metroid` +0.009 %
and `mmc5` −0.059 %, within an instruction or two of what the committed tree gives below.

#### 4.1 PR-1's thirteen: **every ROM inside the owner's +0.02 %, and the set total below the base**

`leg3-finalfeat` against `leg3-base`, 300 frames, `bench-core`:

| ROM | board | delta | % |
| --- | :-: | ---: | ---: |
| `mmc5` | **MMC5** | **−6 644 161** | **−0.059** |
| `vblnmi` | MMC1 | −2 035 | −0.000 |
| `oamstr` | NROM | −15 233 | −0.000 |
| `m4` | MMC3 | −127 608 | −0.002 |
| `m1` | MMC1 | +93 844 | +0.002 |
| `m0` | NROM | +245 305 | +0.004 |
| `readbuf` | CNROM | +397 712 | +0.006 |
| `smb3` | MMC3 | +474 520 | +0.006 |
| `smb1` | NROM | +405 123 | +0.007 |
| `duelito` | NROM | +502 213 | +0.007 |
| `mm2` | MMC1 | +461 284 | +0.008 |
| `z2` | MMC1 | +482 938 | +0.008 |
| `metroid` | MMC1 | +503 090 | **+0.010** |
| **total** | | **−3 223 008** | **−0.004** |

**Worst ROM +0.010 %, against a tolerance of +0.02 %, and `mmc5` is 6.6 M instructions *below*
the base.** Against MF-LEG-2's own figures for the same two features — `metroid` +0.101 %, `mm2`
+0.085 %, `m4` +0.061 %, `m1` +0.060 %, `mmc5` **+2.522 %** — every ROM of the set is now inside
the tolerance and the MMC5 is cheaper than before the feature existed.

**`mmc5` is the row to read twice.** It is `mmc5exram`, and step 3 measured that it **arms neither
mode** over the campaign's own 300 frames: it runs `Board::Mmc5`, which is `04731c86`'s code, so
the −0.059 % is not the feature being fast, it is the feature not being *there* — plus the
cheaper dispatch prologue 4.3 explains. A game that does arm a mode pays for the two modes on its
own fetches, which is what the 3D-pack titles the feature exists for will see; no ROM of the
collection drives them (step 3, and MF-LEG-2 step 3 before it).

#### 4.2 Level 2: `core` and `ppu` inside the tolerance, and what `total` cannot measure

Five commercial games with their 3D packs, 600 frames after 120 of warm-up, inputs pressed:

| game | mapper | `core` | `ppu` | `total` | `apu`, `draw_list` |
| --- | :-: | ---: | ---: | ---: | --- |
| `smb3` | 4 | **+0.007 %** | +0.012 % | **−0.005 %** | +0 |
| `mm2` | 1 | **+0.009 %** | +0.016 % | +0.006 % | +0 |
| `zelda2` | 1 | **+0.009 %** | +0.015 % | +0.013 % | +0 |
| `smb1` | 0 | **+0.010 %** | +0.016 % | +0.016 % | +0 |
| `metroid` | 1 | **+0.010 %** | +0.016 % | +0.009 % | +0 |

**All five games are inside +0.02 % on `core` *and* on `total`**, `core` is +0.007 % to +0.010 %,
and `apu` and `draw_list` are +0 to the instruction. Against MF-LEG-2's `metroid` +0.110 % and
`mm2` +0.103 % on the same stage, that is the whole point of the lot.

**One thing about `total` is worth recording, because an earlier build of this same code failed
on it.** The build measured before the documentation was written — the same instructions, three
doc blocks shorter — gave `smb1/total` +0.029 % and `smb3/total` +0.027 %, with `smb1/lua_hooks`
+0.319 %, `smb3/lua_hooks` +0.327 % and `zelda2/lua_hooks` +0.381 %, while `core` and `apu` did
not move underneath. **Comments moved that stage by a hundredth of a percent**, so what it
measures is the layout of the binary and not its emulation — which is not an assertion here, it
is the **control**:

| stage, `leg3-x37` against `leg3-base` — a `Board` variant that is **never constructed** | |
| --- | ---: |
| `zelda2/lua_hooks` | **+0.364 %** |
| `smb1/lua_hooks` | **+0.241 %** |
| `zelda2/convert` | +0.204 % |
| `smb1/convert` | +0.181 % |
| `smb3/core` | −0.163 % |
| `metroid/core` | +0.096 % |

A variant that cannot be reached by any code path moves `lua_hooks` by a third of a percent and
`core` by a sixth, because `Board` changes size, the cartridge's allocation changes with it, and
mlua's pointer-keyed tables and the converter's allocations land elsewhere. **So neither `total`
nor `lua_hooks` resolves +0.02 % on this instrument** — which MF-LEG-1B had already measured as
±0.029 % on `smb1` and MF-LEG-2 repeated — and the shipped build's `core` figures (+0.007 % to
+0.010 %) are *tighter than the control's* on four of the five games, which is the strongest
statement available here.

#### 4.3 The four shapes, and what each cost

Four shapes of the run-time switch were built and measured. Figures are percentages against
`leg3-base`; `metroid` and `m1` stand for the twelve non-MMC5 ROMs and `mmc5` for the board
itself.

| # | shape | `metroid` | `m1` | `mmc5` | set total | kept? |
| --: | --- | ---: | ---: | ---: | ---: | :-: |
| 1 | `Board::Mmc5Ext(Box<Mmc5Ext>)`, the boxed variant: the enum keeps its niche-filling layout | +0.165 | +0.183 | +0.634 | +0.159 | no |
| 2 | **unboxed** `Board::Mmc5Ext(Mmc5Ext)`: the niche goes, the tag becomes a field | −0.004 | −0.004 | +0.218 | +0.032 | step |
| 3 | unboxed, plus the forwarding dispatches binding `Mmc5Ext`'s `base` field | −0.004 | −0.004 | +0.218 | +0.032 | step |
| 4 | **one match arm for both MMC5 variants** (`Board::Mmc5(b) \| Board::Mmc5Ext(Mmc5Ext { base: b, .. })`) | **+0.010** | **+0.002** | **−0.059** | **−0.004** | **shipped** |
| — | the control: a thirty-seventh variant **never constructed** | +0.089 | +0.046 | +0.132 | +0.019 | — |
| — | shape 3 plus `#[inline(always)]` on `Mmc5::on_cpu_cycle` | −0.003 | −0.003 | +0.250 | +0.037 | no |

Four findings, each measured and none reasoned:

1. **The box was the mistake, and the niche with it.** `Board`'s tag lives in a niche inside the
   MMC5 variant and every dispatch prologue is `movzbl 1355(%rcx); subb $2; movl $5; cmovael` —
   four instructions. A second *unboxed* MMC5 variant overlaps that niche, so rustc gives the enum
   a **tagged** layout (`size_of::<Board>()` 1376 → **1392**) and the prologue becomes **one
   load**. That is worth more than the niche costs: shape 1 is `metroid` +0.165 % and shape 2 is
   `metroid` **−0.004 %**, with twelve of thirteen ROMs inside ±0.008 %. `Board`'s own note has
   been rewritten with this; MF-2 step 0's warning is about a reshape that *moves* the tag, and
   removing it is not that.
2. **Any thirty-seventh discriminant costs the MMC5, and the control proves it is the
   discriminant and not this lot's code.** `leg3-x37` — a variant never constructed, whose arm is
   textually the NROM's — is `mmc5` **+0.132 %**, `metroid` +0.089 %, `mm2` +0.075 %. So the
   37-entry jump table is itself the cost, and three of those tables are pinned
   `#[inline(always)]` chains.
3. **One match arm for both MMC5 variants removes all of it.** The two variants reach the same
   `&mut Mmc5` — `Mmc5Ext`'s first field is the light board at offset 0, which a
   `const _: () = assert!(offset_of!(Mmc5Ext, base) == 0)` now pins — so an **or-pattern** gives
   the thirty-three forwarding dispatches *one block* for the two entries. The three symbols
   shape 3 was paying for all go: `Mmc5::on_cpu_cycle` 0 → 310 217 368 (out of line, against
   −301 285 604 in the dispatch) back to inlined, `NesBus::push_expansion_audio` +8 934 160 back
   to 0, `Cartridge::try_cpu_read` +6 868 631 back to 0. `mmc5` goes from +0.218 % to **−0.059 %**.
4. **Pinning `Mmc5::on_cpu_cycle` instead of fixing the arm made it worse** (+0.218 % →
   +0.250 % on `mmc5`) and was reverted. Forcing a body back inline is not the same as giving the
   two arms one body: the second copy is still a second copy. It is also *free* with the feature
   off — the feature-off binary stayed md5-identical to the base with the attribute in place — so
   the attribute would have been invisible in the shipped build and misleading in the source.

**What the PPU's fetch path cost, which is the question MF-LEG-2 could not answer: nothing.**
`Board::ppu_peek` and `Board::ppu_read` are **64 496 640** and **22 674 330** on `m0` in the base
*and* in every feature shape above — identical to the instruction. The three dispatches the full
variant implements itself (`ppu_peek`, `ppu_read`, `on_ppu_address_bus`) carry a distinct arm and
it costs nothing, where MF-LEG-2's one `bool` inside the MMC5's *existing* arm cost every board
one instruction per CHR fetch. The difference is a new arm against a grown one.

#### 4.4 The decision

**Every acceptance figure the brief names is met**, except the one the brief says to record rather
than meet:

| the brief asks | measured |
| --- | --- |
| every PR-1 ROM except `mmc5` within +0.02 %, with the feature on | **worst +0.010 %** (`metroid`); eleven of the twelve at or below +0.008 % |
| every level-2 game within +0.02 %, totals and cores | **all five inside, on both**: `core` **+0.007 % to +0.010 %**, `total` −0.005 % to +0.016 %, `apu` and `draw_list` +0 |
| `mmc5` recorded separately, and whether it arms either mode | **−0.059 %**, and it **arms neither** (step 3, measured over the campaign's own 300 frames) |
| with the feature off, identical to the base | the **same binary**, md5 `104030539c326d44d2ac4f6a2b088e79` |

*Castlevania III* is R0's to measure. What this lot can say about it is structural and was the
owner's own reason for the switch: it arms neither mode, so it runs `Board::Mmc5` — the base
commit's code — for its whole life, and the only figure it can pay is the one `mmc5` pays on the
same board and the same paths, which is **−0.059 %**. MF-LEG-2 measured it at **+2.992 %**.

Whether the feature joins `default` is R0's and the owner's call; step 5 states it for
`BOARD-FEATURES.md`.
- [x] 5/6 the documentation and the decision

### 5/6 — `BOARD-FEATURES.md`, the `Cargo.toml` note, and what is *not* changed

**`docs/review/BOARD-FEATURES.md`** takes four edits and no deletions:

* a **second dated note of 2026-11-09** under the owner's of the same day, which it answers: the
  condition that note sets — "lowering the MMC5's own cost is the condition for un-parking it" —
  is met, with the figures, and the last sentence says plainly that **nothing in the table has
  moved**: the feature is still out of `default` and still in `parked-boards`;
* the **row's two measured columns rewritten** to carry MF-LEG-3's figures first and MF-LEG-2's
  underneath, "for the record and because they are what the parking was for". A row that kept
  only the new figures would make the parking unreadable;
* the row's **savestate sentence**, which said a state does not cross between the two
  configurations and is now the opposite;
* the closing note on `board-vs-system` ("it is no longer the only one, and no longer the worst"),
  which MF-LEG-2 had extended with a paragraph on the MMC5's arm growing. That paragraph is kept
  in the past tense with **the other half of its own claim** beside it: an arm's size is a cost, a
  *jump table's* size is a cost, and an arm no other board reaches is neither. It also records
  that `board-mmc5-exattr-split` is the first entry in the table whose parking figure has been
  **withdrawn by a later lot** rather than confirmed.

**`rust/crates/zax-nes/Cargo.toml`**, the feature's note: the run-time switch and what it means
for a game that arms neither mode; MF-LEG-3's figures, with MF-LEG-2's kept below them; the
binary-level statement of the gated-off build (the base commit's binary **md5 for md5, whole
file**, where MF-LEG-2 could only say `.text` and `.rodata`); and the savestate paragraph
inverted — a state now crosses, which is what let the switch ship at all.

**The decision.** Every acceptance figure the brief sets is met (step 4.4), so the condition the
owner's note of 2026-11-09 attaches to un-parking is satisfied. **This lot does not un-park it**:
the brief gives that to R0 and the owner ("R0 will do it if the figures below pass"), and the
owner's rule reads on the *figure*, which is now R0's to confirm against *Castlevania III*. What
the lot can say about that game is structural and is in step 4.4 — it arms neither mode, so it
runs the base commit's board and the only figure it can pay is `mmc5`'s, **−0.059 %**, against
MF-LEG-2's +2.992 %.

**Entry MMC5-1 of `LEGACY-XCHECK.md` is left as MF-LEG-2 step 5 wrote it.** Its resolution note
says both features are implemented behind `board-mmc5-exattr-split`, off in `default` and in
`parked-boards`, and that the verdict **DIFFERS still stands for the shipped build** — all three
still true of this tree. Its one paragraph this lot could have strengthened is the "risk of a fix"
scoring, and the brief asks for `BOARD-FEATURES.md` and the `Cargo.toml` note, not for a second
pass over the report; the figures that would feed such a pass are in step 4 of this note, where a
later lot or R0 will find them.
- [x] 6/6 the final check

### 6/6 — the final check

Everything below was run on the committed tree of step 5 (`9c9c58f4`), not on an earlier shape.

**fmt**: `cargo fmt -p zax-nes -p zax-harness --check` clean.

**clippy `--all-targets -- -D warnings`**, the four configurations the brief names:

| configuration | result |
| --- | --- |
| `default` | clean |
| `--features zax-nes/board-mmc5-exattr-split` (the feature alone) | clean |
| `--features zax-nes/parked-boards` | clean |
| `zax-nes --no-default-features` | clean |

**`cargo test --release -p zax-nes -p zax-harness`**:

| configuration | without the ROM roots | with both roots, `--include-ignored --no-fail-fast` |
| --- | --- | --- |
| `default` | **1 167 / 0 / 10** | **1 177 / 0 / 0** |
| `--features zax-nes/board-mmc5-exattr-split` | **1 206 / 0 / 10** | **1 216 / 0 / 0** |
| `--features zax-nes/parked-boards` | **1 267 / 0 / 10** | **1 277 / 0 / 0** |

`ZAXNES_ROMS=G:/3dsen/demo/roms`, `ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`.
**All pass; nothing fails and nothing is skipped** in the three runs with the roots set. Against
MF-LEG-2 step 6's own figures (1 177 / 1 200 / 1 261) that is **`default` unchanged** and **+16**
in the other two — this lot's sixteen tests, with MF-LEG-2's twenty-three all passing through the
run-time switch.

**golden-core over the 98 originals**, 1 200 frames each, re-run on the final tree:

| build | md5 | `cmp` against the base commit's file |
| --- | --- | --- |
| `04731c86`, fresh tree (step 3) | `f79e7df842c3ee43e7d9b67861a927b5` | — |
| `default` | `f79e7df842c3ee43e7d9b67861a927b5` | **identical** |
| `--features zax-nes/board-mmc5-exattr-split` | `f79e7df842c3ee43e7d9b67861a927b5` | **identical** |
| `--features zax-nes/parked-boards` | `f79e7df842c3ee43e7d9b67861a927b5` | **identical** |

**The binary, one last time.** The committed tree was tarred and built clean once more with the
feature off: md5 **`104030539c326d44d2ac4f6a2b088e79`**, the base commit's binary **whole-file**,
and MF-LEG-2's measured shipped binary with it. So "with the feature off, identical to the base"
is settled on the artefact and not on a ROM run, for the fourth time in this note and the last.

**No savestate byte of any other board is touched, and the light MMC5's is not either.** Three
independent statements: the shipped binary *is* the base commit's, so no `save_registers` of any
board can have moved; `Mmc5::save_registers` and `Mmc5::load_registers` are `ca1517dc`'s source
verbatim, without a `#[cfg]` in them, so with the feature on they still write the block the
default build writes; and the format version does not move, because the full variant's three
registers ride in bits of a word the block already carried
(`mf_leg_3_the_savestate_block_is_the_same_length_in_both_variants`, and the five other state
tests of step 2).

**No mapper outside mapper 5 changes.** Three statements of that too: the same binary with the
feature off; golden-core identical over the 98 originals (mappers 0, 1, 2, 3, 4 and 156) **with
the feature on as well**; and level-2 `apu` and `draw_list` +0 with `core` +0.007 % to +0.010 %,
which is the dispatch and not an emulation difference.

**The callgrind figures and the decision**, from step 4:

| | shipped (feature off) | feature on |
| --- | --- | --- |
| PR-1's thirteen, worst ROM | **the same binary** | `metroid` **+0.010 %** |
| PR-1's thirteen, set total | — | **−3 223 008 (−0.004 %)**, below the base |
| `mmc5`, which **arms neither mode** | — | **−0.059 %** |
| level 2, `core` | — | **+0.007 % to +0.010 %** on all five |
| level 2, `total` | — | −0.005 % to +0.016 % on all five |
| over the +0.02 % tolerance | **nothing** | **nothing** |

**Decision: the figures pass and `board-mmc5-exattr-split` is left where it is** — out of
`default` and in `parked-boards`. Un-parking it is R0's and the owner's call (the brief: "R0 will
do it if the figures below pass"), and the owner's note of 2026-11-09 sets *Castlevania III* as
the test; this lot's answer to that is structural and measured on the same board and the same
paths — it arms neither mode, so it runs `04731c86`'s MMC5 and the figure it can pay is `mmc5`'s
**−0.059 %**, against MF-LEG-2's **+2.992 %**.

**Licence.** No new third-party source was read for this lot: the five GPL emulators in
`third_party/` informed MF-LEG-2's banner, which this lot **moved** rather than rewrote, with its
`file:line` citations intact and no line transcribed. `mmc5.rs`'s existing Mesen attribution header
is unchanged and still accurate.

**Open afterwards**: **Q-177**, MF-LEG-2's three readings where the five emulators disagree,
still awaiting R0 — this lot moved the code and the `// Q-177` marker with it and changed none of
the three. Entry MMC5-1's verdict **DIFFERS** still stands for the shipped build by design, and
the gap its closing paragraph names — no test ROM in the collection can judge these two features —
is still open: step 3 re-measured that `mmc5exram` does not even arm them.

**One thing a later lot should know**, because it cost this one three builds to find: on this
enum, at this size, **removing `Board`'s niche-filling layout is cheaper than keeping it**. The
tag in a niche costs four instructions of prologue (`movzbl 1355(%rcx); subb $2; movl $5;
cmovael`) where a tagged layout costs one load, and the 16 bytes `Board` grows by are paid once
at load time. MF-2 step 0's warning is about a reshape that *moves* the tag between variants and
is not about this.
