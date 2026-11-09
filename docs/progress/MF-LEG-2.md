# MF-LEG-2 — the MMC5's vertical split and extended attribute mode

Role R3, clean side. Base commit **`ca1517dc`** (`MF-LEG-1B (5/5): the final check …`), branch
`mapper-review`, worktree `wt-mappers`.

Sources consulted: `docs/review/LEGACY-XCHECK.md` entry **MMC5-1** (the point, the five votes,
the affected titles and the "what a fix would look like" paragraph this lot follows),
the nesdev *MMC5* page (`$5104`'s ExRAM table and the `AACC CCCC` byte, `$5130`'s two global
bank bits, `$5200`-`$5202`), `docs/progress/MR-1.md` and `MF-4c.md` (the MMC5's earlier work and
its measured traps), `docs/progress/PR-1.md` (the instrument and the thirteen ROMs), and the
five cross-checked emulators in `third_party/` on every point where the page is silent:

| | file:line | what was read |
| --- | --- | --- |
| puNES | `mapper_005.c:243-256`, `:388-406`, `:439-493` | `$5200`-`$5202`, the per-line reset of the split counters, the split and mode-1 fetch hooks |
| Nestopia UE | `NstBoardMmc5.cpp:845-868`, `:900-966`, `:676-706` | `ClockSpliter`, `FetchNt*` (the four composed forms), `GetSpliterAttribute`, `GetSpliterPattern`, `GetExtPattern` |
| Mesen 2 | `MMC5.h:460-555`, `:576-585` | `MapperReadVram`'s split and mode-1 paths, the fine-Y substitution |
| FCEUX | `mmc5.cpp:174-268` | `mmc5_PPURead`'s split and `MMC5HackCHRMode == 1` paths |
| Nintendulator | `h_MMC5.cpp:531-538`, `:576-599` | `$5200`-`$5202` and the PPU read hook |

## What the lot does

**One feature, `board-mmc5-exattr-split`, off by default** (owner's decision, taken before any
measurement). With it off nothing of this lot is compiled: the MMC5's board, the PPU's fetch
path and the tile capture are `ca1517dc`'s, line for line. With it on the board gains

1. **extended attribute mode** (`$5104` = %01): the attribute fetch of a background tile is
   answered from ExRAM — two bits of palette for the whole 8x8 tile — and the two pattern
   fetches take a 4 KiB bank from the same byte's low six bits with `$5130`'s two as bits 6-7;
2. **the vertical split** (`$5200`-`$5202`): a per-line tile counter, a scroll counter of its
   own reloaded from `$5201`, the region's nametable **and** attribute bytes from ExRAM, and its
   patterns from `$5202`'s 4 KiB bank with the low three address bits substituted.

### The shape, and what it costs

Entry MMC5-1 names the trap: "a careless implementation could cost every game a few
instructions per fetch", and the safe shape is "decide everything decidable per *register
write*, and leave the per-fetch path a load and a branch". This lot takes that shape and hangs
the whole thing on a test the PPU already makes.

**With the feature off that is complete**: `.text` and `.rodata` of the shipped binary are the
base commit's **byte for byte**, level-2 `core` is +0 on all five games and PR-1's worst ROM is
+0.000 % (step 4). **With it on it is not**, and the figures are what park the board: the MMC5's
arm of `Board::ppu_peek` and `Board::ppu_read` grows by the one `bool` test, and the dispatch
then costs **about one instruction per CHR fetch to every board** — `metroid` +0.095 %, `mm2`
+0.085 %, `m4` +0.061 %, `m1` +0.060 % on PR-1, and `metroid`/`mm2` `core` +0.110 %/+0.103 % on
level 2, against a tolerance of +0.02 %. Step 4 has the seven measured shapes, the four that
worked and the two that did not.

`Ppu2C02::read_nametable_as` already asks `Cartridge::nametable_page` where a nametable lies
and already branches on `SERVED_BY_CARTRIDGE`. The MMC5 is the only board whose pages move at
run time (`Cartridge::nametable_pages_vary`), so with the feature on it answers
`SERVED_BY_CARTRIDGE` for **all four** tables as soon as mode 1 or the split is on, and serves
every nametable-space byte itself — the ordinary CIRAM tile bytes included. For that it needs to
see console VRAM, which is what the one new hook is for:

```rust
fn read_nametable_from_vram(&self, memory: &CartridgeMemory, address: u16, vram: &[u8]) -> u8
```

a `Mapper` method whose default body forwards to `read_nametable`, overridden by the MMC5 alone.
With the feature off the call site is `ca1517dc`'s `cartridge.read_nametable(address)`; with it
on it is this one, which changes the generated code of **no board that does not already take the
`SERVED_BY_CARTRIDGE` branch** — and of the thirteen ROMs of PR-1's set only `mmc5` takes it.

The per-tile ExRAM index that mode 1 needs is not in the attribute address, so the board latches
it where it already sees every PPU read: `Mmc5::see_ppu_read`, behind the feature's `cfg`. The
split's tile counter, and the dot where a line's fetch sequence starts, are found in the same
place — see step 4, which is where the PPU hook step 1 shipped for that was measured and removed.

### The 3D view

`Ppu2C02::capture_background_row` reads each tile's nametable byte **and its attribute byte**
through `read_nametable_inner`, so the two overrides above reach the capture unchanged and the
`TileRecord` carries the per-tile palette the 2D picture shows — which is what
`zax-engine3d`'s `draw_list` and `segmenter` read (`tile.palette()`). The capture walks the row
out of band, so the per-tile index the board latched during rendering is not the one the capture
wants: it is set per column by the capture and put back afterwards, through the **save/restore
the capture already does** for the MMC2/MMC4 latches (`Cartridge::chr_latch_state`,
`begin_capture_line` / `end_capture_line`, REQ-CORE-033). No hot path of the converter is
touched for any game, with the feature on or off.

## Steps

| # | step | the brief's steps |
| --: | --- | --- |
| 1 | the feature, the board's registers and state, mode 1 and the split, the nametable hook, the savestate | 1, 2 |
| 2 | the 3D view: the capture's per-tile palette and the split's tiles, and the 2D-versus-3D agreement test | the 3D view |
| 3 | golden-core against the base commit, and the full test run with both ROM roots | the final check |
| 4 | the callgrind campaign | the final check |
| 5 | the report entry MMC5-1 and `BOARD-FEATURES.md` | — |
| 6 | the final check | the final check |

**Steps 3 to 5 are not the order the plan above was written in.** Steps 1 and 2 shipped with the
documentation step numbered 3; it is **step 5** instead, because entry MMC5-1 and
`BOARD-FEATURES.md`'s row both have to carry the measured figures, and a row written before the
campaign would be a row to rewrite. The commit messages of steps 1 and 2 are therefore the only
place the first numbering survives.

- [x] 1/6 the feature, the board, mode 1, the split, the nametable hook, the savestate

### 1/6 — the feature, the board, the two modes, the nametable hook and the savestate

**The feature.** `board-mmc5-exattr-split`, off by default. (This step shipped it *outside*
`parked-boards`, on the reading that the umbrella is for boards left out for a figure and this
one was out for a decision; **step 4 measured a figure and put it in**, so the `Cargo.toml` note
this paragraph describes has been rewritten there.) Its
`Cargo.toml` note carries the titles, the shape, and the one consequence worth knowing: a
savestate does not cross between the two configurations, because with the feature on the MMC5's
block carries `$5200`-`$5202` after its sound state. That is already true of every board feature
that changes what memories a cartridge has (`board-195-prg-ram-5000`, `board-vs-system`).

**The board** (`mmc5.rs`). Nine new `#[cfg]`'d fields, three of them the registers and six the
per-tile state a line re-derives; a banner comment *Extended attribute mode and the vertical
split* holding the page's sentences, the composition rule and the three points where the five
emulators disagree; and five new methods. Two shapes are deliberate and both are entry MMC5-1's
own warning carried out:

* **`Mmc5::background_substitutes`**, one `bool` derived from `in_frame`, `ex_ram_mode` and
  `$5200` bit 7, is all a fetch tests. It is declared immediately after
  `background_uses_set_b` so that the trailing run of `bool`s is left exactly as `ca1517dc` had
  it and `in_frame` keeps the niche rustc fills with `Board`'s discriminant — the trap
  `docs/progress/MF-2.md` step 0 priced at 1-2 instructions per CHR fetch **for every other
  board**.
* **`Mmc5::substituted_chr_offset` is `#[inline(never)]`**, for the reason `see_ppu_read` is:
  `chr_address` is the MMC5's whole CHR path and a second bank computation inlined into it is
  the same register-pressure trap. In `chr_address` the two modes are one `bool` and one
  compare ahead of the existing `match`.

**Mode 1.** The `ExRAM` index is `address & 0x03FF` of the background nametable fetch, latched
in `see_ppu_read` because the attribute address does not carry it (it keeps coarse Y bits 4-2
and coarse X bits 4-2 only). The attribute answer is `(byte >> 6) * 0x55` — the two bits in all
four quadrants, so the PPU's own shift yields them whatever the tile's position, which is what
all four emulators that implement it do. The bank is `(byte & 0x3F) | (chr_upper << 6)` at 4 KiB,
and the nametable *tile* byte is untouched.

**The split.** `$5200`-`$5202` decoded; the tile counter pre-incremented by each counted
nametable fetch and reset by the new `Mapper::on_tile_row_start`; the row
`(scanline_counter + $5201) % 240`; the tile `((row & 0xF8) << 2) | column`; the attribute from
`ExRAM`'s own `$3C0`-`$3FF` squared; the pattern from `$5202`'s 4 KiB bank with the low three
address bits substituted from the row.

Two details are this build's own and are worth naming:

* **The two discarded nametable fetches of dots 338 and 340 must not be counted.** They repeat
  dot 337's address, so the counter advances only when `address != last_ppu_read_addr` — the
  same test the three-identical-reads detection already makes. Without it the column runs three
  ahead by the end of a line. puNES and Nestopia sidestep this by counting *attribute* fetches;
  this pipeline fetches the nametable byte first, so the count has to be there.
* **The reset point is dot 321, not the start of a line.** Dot 321 is where this PPU starts the
  fetch sequence of the *next* line (its first three tiles are prefetched at dots 321, 329 and
  337), so a reset at dot 0 would split the line's own columns across two resets. puNES resets
  at the same dot (`mapper_005.c:391`) and Nestopia at the same point of its own pipeline.
  `Mapper::on_tile_row_start` is therefore a hook of its own, called once per rendered line and
  on no other board, rather than a test inside the fetch pipeline.

**The one new hook, and why nothing else costs anything.** `Ppu2C02::read_nametable_as` already
asks `Cartridge::nametable_page` and already branches on `SERVED_BY_CARTRIDGE`, so with either
mode armed `Mmc5::nametable_page` answers it for **all four** tables and the board serves every
nametable-space byte. The only thing it lacked there was a way to read the CIRAM page `$5105`
names, and that is `Mapper::read_nametable_from_vram(memory, address, vram)`, whose default body
forwards to `read_nametable`. Three consequences, each handled:

1. `Board::read_nametable_from_vram` is **not** a `dispatch!`. `Board::read_nametable`'s own doc
   comment records lot MF-5a measuring that a *third* distinct arm body turns its bitset into a
   jump table and costs every board two instructions per nametable fetch. The new function is
   one test for `Board::Mmc5` and a tail call to `read_nametable` for everything else, so that
   function and its bitset are untouched.
2. A `$2007` **write** to a table CIRAM still backs must still reach CIRAM.
   `Mapper::nametable_write_offset` says where, and `Ppu2C02::write_nametable_byte` uses it
   under the feature's `cfg`. A cold path: a game writes a few hundred nametable bytes a frame
   against the 17 000 fetches of one. `Ppu2C02::nametable_written_as` takes the same route, so a
   widescreen margin decoder is not told "never written" about a byte that was.
3. `Mmc5::nametable_page`'s condition is the two **register** bits alone, never `in_frame`:
   `Cartridge::nametable_pages` is a cache refreshed by a CPU write to the cartridge, and a
   condition that moved inside a frame would leave it stale. The in-frame half of the test is
   made where the byte is answered.

**Q-177** is the new ticket: the three points where the emulators disagree (the threshold
column's side of the comparison, which screen line shows the region's row `$5201`, and whether
the split has a fine vertical scroll at all). All three are shipped as the page reads, each
pinned by a test whose message names the alternative, and each a one-expression change if R0
prefers another reading. `// Q-177` is at `Mmc5::reload_split_row`.

**Tests.** `mf_leg_2_mmc5_exattr_split_tests.rs`, nineteen tests, every one of them driving a
**rendered frame** rather than the board's methods — both features are about what the PPU
fetches, and the per-tile state they need is latched on the board's own bus while a line is
drawn, so a test that called `read_nametable_from_vram` by hand would be testing an expression.
The image's 4 KiB bank `b` holds at tile `t` eight rows of `(b << 4) | (t & 0x0F)` in plane 0 and
**zero** in plane 1, and sub-palette `p`'s entry 1 is `$p1`, so a single pixel names both the
sub-palette the tile was drawn with and which bit of the fetched pattern byte was set: enough to
identify the bank, the tile and the palette of every tile on screen.

Beyond the page's own clauses the set includes the two compositions (mode 1 outside the split
and `ExRAM`'s attribute table inside it, reading the *same* 1 KiB two ways), the counter itself
(an `ExRAM` whose cells carry their column number, so a counter one off draws a neighbour's
tile), the `$2007`-write invariant of point 2 above, and the savestate round trip. Plus
`mapper_tests.rs`'s `mf_leg_2_mmc5_the_two_substituting_modes_exist_only_behind_their_feature`,
which runs in **both** configurations off the new `pub const Mmc5::EXATTR_SPLIT` and asserts
both sides of the owner's decision in one test.

**Checks at this step** (`CARGO_TARGET_DIR=rust/target-mf`): `cargo fmt -p zax-nes -p
zax-harness --check` clean; `cargo clippy --all-targets -- -D warnings` clean on both crates in
`default`, `--features zax-nes/board-mmc5-exattr-split` and `--features zax-nes/parked-boards`,
and on `zax-nes` alone with `--no-default-features`.

| `cargo test --release -p zax-nes -p zax-harness` | result |
| --- | --- |
| `default` | **1167 passed, 0 failed, 10 ignored** |
| `--features zax-nes/board-mmc5-exattr-split` | **1186 passed, 0 failed, 10 ignored** |
| `--features zax-nes/parked-boards` | **1228 passed, 0 failed, 10 ignored** |

The default build gains exactly one test over `ca1517dc` — the both-configuration one — and the
feature build the nineteen of the new file beside it. The base commit's own three figures are
re-measured against this tree at step 4, where the suites are run with both ROM roots set.
- [x] 2/6 the 3D view and the 2D-versus-3D palette agreement test

### 2/6 — the 3D view: the per-tile palette and the split's tiles in the records

**Where the 3D conversion reads the per-tile palette.** `zax-engine3d` does not read the
attribute table at all: a block's palette table is `tile.palette()` of the `TileRecord`
(`draw_list.rs:481-487`, and `segmenter.rs:207`, `:259`, `:333`, `:376` group blocks by it), and
the records come from `Ppu2C02::capture_background_row` (REQ-CORE-037), which reads each tile's
nametable byte, its **attribute byte** and its two pattern planes through the inert path. So
both overrides reach the converter through the capture, and **no hot path of the converter is
touched for any game** — with the feature on or off.

**The one thing the capture needed.** It walks a row **out of display order** at the end of the
line, so the board's per-tile state — mode 1's `ExRAM` index, the split's column, row, tile and
region flag — is whatever the last *rendered* fetch left. It therefore has to be positioned per
captured column and then put back, and putting it back is not optional: the tile whose
nametable byte was fetched at dot 337 has its attribute and pattern fetches on the **next**
line and would otherwise find the capture's leftovers.

That is exactly the save/walk/restore the capture already performs for the MMC2 and MMC4 latches
(REQ-CORE-039, and REQ-CORE-033's "the emulation is identical whether or not the host registered
a video callback"), so the three new hooks are that pair's shape without disturbing its
contract — `Mmc5::chr_latch_state` stays the trait's 0:

| hook | what it does |
| --- | --- |
| `Mapper::background_capture_state` | the per-tile state packed into one `u64` — six fields, 34 bits, which is why it is not an `i32` |
| `Mapper::set_background_capture_state` | puts it back, in `end_capture_line` beside `restore_chr_latch` |
| `Mapper::position_captured_background_tile(address, column, line)` | puts it on the tile about to be read |

`Board`'s three forwarders are one `Board::Mmc5` test each and **not** a `dispatch!`, for
`read_nametable_from_vram`'s reason squared: these sit on `Ppu2C02::read_pattern_planes`, the
function lot MF-2 built `read_nametable_inner`'s `#[inline(never)]` shield for.

`column` may be negative or above 31 — the widescreen extension captures `extra` columns on each
side — and the split region is a property of the 32 columns the PPU draws, so a column outside
them is never in it. `line` is the band's **top** screen line (`y`, which is `line - 1`), the one
whose split row the band shows.

**The one limitation, and it is in the code.** `Mmc5::split_capture` exists because the capture
reads a tile's **eight rows at once** while a rendered line reads one: substituting the region's
fine vertical scroll into all eight would collapse the tile to a single line, so the capture
keeps the PPU's own low three address bits. The consequence is exact: the split's **tile** scroll
reaches the 3D view and its **fine** scroll does not. A region scrolled by a fraction of a tile
straddles two `ExRAM` rows inside one 8-pixel band, and no single 8x8 record can carry both — the
same reason the capture emits one row per eight lines in the first place (REQ-CORE-037). The
field's doc comment says so and the test below pins `$5201` to multiples of 8 and says why.

**Not covered, and named rather than discovered later**: the widescreen margin decoders
(`Ppu2C02::decode_margins` and `read_row_planes`) supply their own tile numbers and palettes out
of the game's memory, so neither mode reaches them. It costs nothing today — the only MMC5 3D
pack is *Castlevania III*'s, which uses neither mode — and a widescreen pack for *Just Breed* or
a Koei title would need a decoder written against `ExRAM`, which is a lot of its own.

**The tests**, four of them, and they are stronger than the brief's "2D-versus-3D palette
agreement test on a synthetic frame" because the record carries **both** halves of the answer —
the palette and the two bit planes — and the two modes move both. So
`assert_records_agree_with_the_picture` checks all 960 records of a frame **pixel for pixel**
against the 2D picture: a record that had taken the right palette from `ExRAM` and the wrong CHR
bank would pass a palette-only comparison and fails this one. It also asserts the record count
(30 x 32) and that more than a thousand pixels were opaque, so a frame that drew nothing cannot
pass.

| test | the frame it compares |
| --- | --- |
| `mode_1_reaches_the_tile_records_with_the_same_palette_and_bank` | mode 1 with the palette cycling by column and the bank by row, so no two neighbouring blocks agree by accident |
| `the_split_reaches_the_tile_records_with_its_own_tiles` | the split at `$5201` = 0 and 8 and on both sides, every `ExRAM` cell its own tile |
| `the_two_modes_composed_reach_the_tile_records` | the state *Bandit Kings of Ancient China* reaches: the same 1 KiB read as tile numbers inside the region and as `AACC CCCC` outside it |
| `the_records_agree_with_the_picture_with_neither_mode_armed` | neither mode, so the comparison is not passing because the capture and the picture share a bug |

**Checks at this step**: `cargo fmt -p zax-nes -p zax-harness --check` clean; clippy
`--all-targets -- -D warnings` clean on both crates in `default`, with the feature and with
`parked-boards`, and on `zax-nes` alone with `--no-default-features`.

| `cargo test --release -p zax-nes -p zax-harness` | result |
| --- | --- |
| `default` | **1167 passed, 0 failed, 10 ignored** |
| `--features zax-nes/board-mmc5-exattr-split` | **1190 passed, 0 failed, 10 ignored** |
| `--features zax-nes/parked-boards` | **1228 passed, 0 failed, 10 ignored** |
- [x] 3/6 golden-core and the suites with both ROM roots

### 3/6 — golden-core over the 98 originals, and the suites with both ROM roots

```
zax-harness golden-core G:/3dsen/test-originals gc98-<label>.jsonl 1200
```

The base commit was extracted with `git archive ca1517dc` into a **fresh** directory with its own
`CARGO_TARGET_DIR`, built there, and that directory deleted as soon as the figures were taken.

| build | md5 of the 98-ROM digest |
| --- | --- |
| `ca1517dc`, the base commit, fresh tree (binary md5 `b4a3cbdb13b932e77bd9c84f24ba10a6`) | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `default` | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `--features zax-nes/board-mmc5-exattr-split` | `f79e7df842c3ee43e7d9b67861a927b5` |
| this tree, `--features zax-nes/parked-boards` | `f79e7df842c3ee43e7d9b67861a927b5` |

**Byte-identical in all four** — `cmp` on the files, not only the digests — and identical to the
digest fourteen lots have now recorded. 98 ROMs run, 98 files.

**Why the feature build is identical too, which is the interesting row.** The 98 originals use
mappers 0, 1, 2, 3, 4 and 156. **None of them is mapper 5**, so neither substituting mode can be
reached and the only way the digest could move is if the lot had touched a shared path — which
is exactly what this row rules out, for the `read_nametable_as` call site, the
`write_nametable_byte` and `nametable_written_as` branches, the dot-321 hook in
`run_rendering_dot` and the two capture hooks in `tiles.rs` all at once. *Castlevania III* is not
in the 98 (it is not in `G:/3dsen/test-originals`), and the MMC5 is covered instead by `mmc5exram`
in the callgrind set at step 4 and by the twenty-three tests of step 1 and 2.

**The suites**, `ZAXNES_ROMS=G:/3dsen/demo/roms` and
`ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`, `--no-fail-fast --
--include-ignored`:

| run | result |
| --- | --- |
| `default` | **1177 passed, 0 failed, 0 ignored** |
| `--features zax-nes/board-mmc5-exattr-split` | **1200 passed, 0 failed, 0 ignored** |
| `--features zax-nes/parked-boards` | **1238 passed, 0 failed, 0 ignored** |

Against MF-LEG-1B's own final figures (1176 / — / 1237) that is **+1 in `default` and +1 in
`parked-boards`** — the one both-configuration test of step 1 — and +23 more with the feature on,
the nineteen of step 1 plus the four of step 2. **No test-ROM row moves in either direction**, and
that is worth more than one line, because entry MMC5-1's closing paragraph bets on these ROMs:

> `exram` is about the $5C00-$5FFF modes and `mmc5test_v2` covers the split and the scanline
> IRQ. **Neither is run by the project today.** Running `exram` would settle MMC5-3 at first
> hand; running `mmc5test_v2` would quantify MMC5-1 rather than leave it as a feature checklist.

**Half of that is already out of date and the other half does not work.** All three MMC5 ROMs of
the collection *are* run — lot MF-4c step 4 put them in
`req_core_100_the_mmc5_rom_suites_keep_their_outcomes`, as `Manual`, "they print a screen with no
documented verdict" — and this step ran them in both configurations with `--shots` and compared
the PNGs:

| ROM | default vs feature, final frame |
| --- | --- |
| `nes-test-roms/exram/mmc5exram.nes` | **byte-identical** |
| `nes-test-roms/mmc5test/mmc5test.nes` | **byte-identical** |
| `nes-test-roms/mmc5test_v2/mmc5test.nes` | **byte-identical** |

`mmc5test_v2` is an **interactive** register test: the frame it lands on is its menu — "bg
pattern table", "bank set order", "current chr banks", "controls" — and reaching its split
sub-test needs button presses the campaign does not make. So the collection cannot quantify
MMC5-1 as the entry hoped, with or without this feature, and the twenty-three tests of steps 1
and 2 are what pins the behaviour instead. Recorded here so the next lot does not spend the same
afternoon on it; the entry's own paragraph is corrected at step 5.
- [x] 4/6 the callgrind campaign

### 4/6 — the callgrind campaign

Binaries built by `rust/tools/perf/build-clean.sh` from their own tar into a **fresh** directory
with its own `CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`. The feature variant
was made from the shipped tar by adding one line to zax-nes's `default` list.

| label | tree |
| --- | --- |
| `leg2-base` | **`ca1517dc`**, the lot's base commit, md5 `64edf2c20f580029ca0898bc6a87feb3` |
| `leg2-ship` | **the shipped configuration**, the feature out of `default` |
| `leg2-feat` | the shipped tree plus **`board-mmc5-exattr-split`** in `default` |
| `leg2-ship2` | a byte copy of `leg2-ship`, the **same-binary floor** |

**The base binary's md5 is `64edf2c20f580029ca0898bc6a87feb3`, which is MF-LEG-1B's `leg1b-ship`
— that lot's own measured shipped binary — to the byte.** So this lot's base is the previous
lot's measured binary, the **thirteenth** lot in a row for which that holds, and built rather
than assumed. The ship and feature md5s are deliberately not quoted: five shapes were built and
measured before the shipped one was settled, so one number would be the last of five rather than
*the* answer. What each variant measured is below.

#### 4.0 The shipped build is the base commit's code, proved on the binary

Before any ROM was run, `leg2-base` and the shipped binary were compared **section by section**.
They are the same size (7 897 608 bytes) with the same 6 216 symbols at the same sizes, and
**37 bytes differ in the whole file**:

| bytes | where | what |
| --: | --- | --- |
| 20 | `.note.gnu.build-id`, `0x32c`-`0x33f` | the build-id, a hash of the build inputs, which a changed **comment** moves |
| 17 | `.data.rel.ro`, 24 bytes apart | `core::panic::Location` records — one decoded as `{len: 31, line: 810, col: 29}` in the base and `{len: 31, line: 837, col: 29}` in the shipped build: the same file and column, the line moved by the comments this lot added |

**`.text` and `.rodata` are byte-identical.** So every instruction of the default build is
`ca1517dc`'s, which answers the brief's "the release binary's hot functions unchanged" on the
whole binary rather than on a list of functions. The callgrind figures below are the same
statement, measured.

#### 4.1 The floors

**PR-1's thirteen**, the shipped binary against its own byte copy: worst ROM **+0.000 %**, set
total −70 379 (−0.000 %) — so ±0.001 % is noise, as every MF lot has recorded. A second and
stronger floor is available here: **`leg2-base` against `leg2-ship` is two separately built
binaries with identical `.text`**, and it gives worst **+0.001 %** (`metroid`) and a set total of
−46 137.

**Level 2's `core` is exact**: across base and ship it is **+0 on all five games, identical to
the instruction**, and so is the `ppu` stage. The whole-process `total` moves −0.024 % to
+0.002 %, which is the Lua and conversion noise MF-LEG-1B measured at ±0.029 % on `smb1`.

#### 4.2 The shipped configuration — nothing, as the binary says

| | worst ROM | set total |
| --- | --- | --- |
| PR-1's thirteen, ship vs base | `vblnmi` **−0.001 %**; worst *slower* `metroid` **+0.001 %** | **−46 137 (−0.000 %)** |
| level 2, `core` | **+0 on all five, identical to the instruction** | +0 |
| level 2, `total` | `smb1` −0.025 %; worst slower `mm2` +0.000 % | — |

#### 4.3 The feature, and the seven shapes it took to get here

The first measured feature build was **+0.538 % on the set, with twelve of thirteen ROMs above
the tolerance**. The campaign that followed is recorded in full, because four of the seven
attempts worked and are in the shipped code and two did not and were reverted — the next lot
should not re-derive either list. Figures are percentages against `leg2-base`.

| # | what was tried | `duelito` | `m0` | `metroid` | `m4` | set total | kept? |
| --: | --- | ---: | ---: | ---: | ---: | ---: | :-: |
| 0 | the first build: a `dot == 321` hook in `Ppu2C02::run_rendering_dot` | +0.144 | +0.055 | +0.165 | +0.110 | +0.538 | — |
| 1 | **the hook removed**, the row start found on the board's own bus | +0.128 | +0.040 | +0.144 | +0.103 | +0.441 | **yes** |
| 2 | the per-column capture call gated on a `bool` hoisted out of the loop | +0.120 | +0.032 | +0.134 | +0.097 | +0.432 | no |
| 3 | **the capture row `const`-monomorphised** on `SUBSTITUTED` | +0.096 | +0.008 | +0.100 | +0.081 | +0.411 | **yes** |
| 4 | **the per-line save/restore folded into** that body | +0.100 | +0.013 | +0.106 | +0.084 | +0.411 | **yes** |
| 5 | **the substituting body behind `#[inline(never)]`** | −0.033 | −0.081 | −0.020 | +0.167 | +0.394 | **yes** |
| 6 | the capture's predicate cached as a `bool` on `Cartridge` | +0.101 | +0.013 | +0.106 | +0.087 | +0.411 | no |
| 7 | **attempt 6 reverted; 1, 3, 4 and 5 kept** | **+0.004** | **+0.002** | +0.095 | +0.058 | **+0.373** | **shipped** |

Four findings, each measured and not reasoned:

1. **A `dot == 321` test inside `run_rendering_dot` cost every ROM of the set.** That function
   inlines into `Ppu2C02::run_dots`, and the test moved **+5.4 M to +13.6 M instructions** onto
   it — `duelito` +13 640 525 — by changing the inliner's mind. It is `docs/progress/MF-2.md`
   step 0's trap one function further out. Attempt 1 removes it: the board finds that dot itself
   by counting the pattern fetches between two nametable-space reads — two inside a tile group,
   sixteen across the sprite window of dots 257 to 320 — which is `Mmc5::chr_reads_since_nt`.
   **The PPU hook and its `Mapper`, `Board` and `Cartridge` forwarders are all gone**, and the
   split's columns are proved right by
   `mf_leg_2_the_split_reads_the_ex_ram_cell_of_its_own_column`, which a counter one off cannot
   pass.
2. **Gating the capture's per-column call on a `bool` is not enough; monomorphising it is.**
   Attempt 2 moved `duelito` by 0.008 points. Attempt 3 —
   `capture_background_row_as<const SUBSTITUTED: bool>`, chosen once per row — moved it by 0.024
   and `m0` by 0.024, because in the `false` instantiation the `&mut Cartridge` reborrow **does
   not exist** rather than being skipped.
3. **The decisive one is `#[inline(never)]` on the substituting body.** With both instantiations
   inlinable, `run_dots` carried two copies of the capture and paid their register pressure even
   on an NROM that never reaches one: `duelito` +7.5 M. Behind a call —
   `Ppu2C02::capture_background_row_substituted` — `run_dots` ends up **smaller than the base's**
   on ten of the thirteen ROMs, which is why eight of them are now inside ±0.006 %.
4. **Caching the predicate on `Cartridge` made things worse and was reverted.** A `bool` field
   beside `nametable_pages` — the shape `watches_ppu_reads` has — made
   `Cartridge::refresh_rom_windows` too large to inline, putting **+610 864 instructions on
   `smb3` and +101 088 on `m1`** on a function that runs after every CPU write to the cartridge,
   and the capture's own figure did not move by an instruction. The `Cartridge` method calls the
   `Board` dispatch instead, and its doc comment records why.

#### 4.4 The figures that park the board

**PR-1's thirteen, the feature build against the base:**

| ROM | board | delta | % |
| --- | :-: | ---: | ---: |
| `smb3` | MMC3 | −3 892 793 | **−0.049** |
| `z2` | MMC1 | −829 985 | **−0.013** |
| `oamstr` | NROM | +114 947 | +0.003 |
| `m0` | NROM | +408 550 | +0.007 |
| `vblnmi` | MMC1 | +215 718 | +0.007 |
| `duelito` | NROM | +609 017 | +0.008 |
| `readbuf` | CNROM | +684 160 | +0.010 |
| `smb1` | NROM | +597 390 | +0.010 |
| `m1` | MMC1 | +2 932 206 | **+0.060** |
| `m4` | MMC3 | +3 351 021 | **+0.061** |
| `mm2` | MMC1 | +5 132 359 | **+0.085** |
| `metroid` | MMC1 | +5 147 317 | **+0.101** |
| `mmc5` | **MMC5** | +284 172 985 | **+2.522** |
| **total** | | +298 642 892 | +0.376 |

**Eight of thirteen are inside the owner's +0.02 %, two of those below the base, and four are
above it.** `mmc5` at +2.522 % is the board paying for its own feature and is not a tolerance question:
`mmc5exram` drives the ExRAM modes, so `Mmc5::see_ppu_read` (+106 726 522) and the newly
out-of-line `Mmc5::ppu_peek` (+439 047 320, against `Board::ppu_peek` **−113 398 000** and
`Board::ppu_read` **−149 299 500** as LLVM moves the arm out of the dispatch) are the feature
doing its work.

**The four that are over the tolerance are one symbol**, and the same one in every variant
measured above:

| ROM | symbol | delta |
| --- | --- | ---: |
| `metroid` | `Board::ppu_peek` | +4 528 000 |
| `mm2` | `Board::ppu_peek` | +4 485 120 |
| `m1` | `Board::ppu_peek` | +2 288 640 |
| `m4` | `Board::ppu_read` | +3 082 184 |

Those are **≈ 1 instruction per CHR fetch** in the dispatch, on about 4.5 M fetches over 300
frames. The cause is named in `Board`'s own doc comment and in `Mmc5::background_substitutes`:
the MMC5's arm grows by the `bool` test and the dispatch's prologue grows with it, **for every
board**. `docs/progress/MF-2.md` step 0 priced the same byte at 1-2 instructions per CHR fetch,
and `Mmc5::see_ppu_read`'s own comment at +16.6 M on SMB3. **None of the seven attempts moved
this figure by a single instruction**, in any variant — which is itself the finding: it is not
the capture, not the hook and not the inliner, it is the size of the arm.

`m1` also carries `NesBus::write_internal` +312 961 (+0.006 % of the ROM), which is
`Ppu2C02::write_nametable_byte`'s new `nametable_write_offset` branch on the ROM that writes
`$2007` 219 times a frame — the heaviest `$2007` writer of the set (PR-1 step 1's table).

**Level 2, the `core` stage** — exact; the whole-process `total` is inside the ±0.029 % noise
MF-LEG-1B measured:

| game | mapper | base `core` | feature `core` | delta | % |
| --- | :-: | ---: | ---: | ---: | ---: |
| `smb3` | 4 | 15 315 255 735 | 15 305 861 154 | −9 394 581 | **−0.061** |
| `zelda2` | 1 | 12 775 020 988 | 12 773 153 103 | −1 867 885 | **−0.015** |
| `smb1` | 0 | 11 515 129 187 | 11 516 588 456 | +1 459 269 | +0.013 |
| `mm2` | 1 | 12 037 310 877 | 12 049 767 967 | +12 457 090 | **+0.103** |
| `metroid` | 1 | 11 146 889 701 | 11 159 173 931 | +12 284 230 | **+0.110** |

Two of five above the tolerance, and the same two MMC1 games PR-1 puts there. The `ppu` stage
carries all of it — **+0.176 % on both** — which is where the CHR fetches are.

#### 4.5 The decision

**The board is parked.** The owner's rule is that "a board over the tolerance is parked: moved
out of `default` into `parked-boards`, never dropped", and four of PR-1's thirteen and two of
level 2's five are over it. `board-mmc5-exattr-split` is in **`parked-boards`** as of this step,
beside `board-vs-system`, `board-292` and `board-143-open-bus`, and `--features parked-boards`
is the one flag that turns it on. It was already out of `default` by the owner's decision taken
before measurement, so **this changes no shipped behaviour**; what it changes is that the
umbrella now reaches it, which is what the rule asks. The `parked-boards` suite therefore grows
by the 23 tests of steps 1 and 2 — **1 261 passed, 0 failed, 0 ignored** with both ROM roots set,
against step 3's 1 238 — and `default` and the feature build are step 3's 1 177 and 1 200
unchanged, this step having moved no behaviour.

Two things R0 may want to weigh against the figures:

* the cost is **not** in the two features' own code but in the size of the MMC5's dispatch arm,
  so any implementation of them in this architecture would pay it. Entry MMC5-1's warning that
  "a careless implementation could cost every game a few instructions per fetch" turns out to
  apply to a careful one as well, and the seven attempts above are the evidence;
* it is **one instruction per CHR fetch**, paid by every game, against the seven titles it buys
  — *Just Breed*, *Nobunaga's Ambition II*, *Romance of the Three Kingdoms II*, *Uncharted
  Waters*, *Bandit Kings of Ancient China*, *Yakuman Tengoku* and *Uchuu Keibitai SDF* — which
  render visibly wrong without it. *Castlevania III*, the most-played MMC5 game, needs neither
  and pays the instruction either way.
- [x] 5/6 the report entry and `BOARD-FEATURES.md`

### 5/6 — entry MMC5-1, `BOARD-FEATURES.md` and Q-177

**Entry MMC5-1** gains a *resolution note* at its heading and keeps every paragraph it had,
because three of its predictions can now be checked and one of them is wrong. What was added:

* the note itself — both features implemented, behind `board-mmc5-exattr-split`, off in
  `default` and **in `parked-boards`**, so **the verdict DIFFERS still stands for the shipped
  build** and is answered for a build that asks;
* a marker on *"**This build.** Neither"*, which is now true only of the default configuration,
  with the three `#[cfg]`s named and the binary-level proof that the sentence is exact there;
* *"**Risk of a fix**"* marked as written before the fix, and a new section **"What the fix
  actually looked like"** that scores its four predictions: *(a) and (b) are one feature* — right,
  and (c) belongs with them; *the interface does not express it* — right about the problem,
  wrong about the remedy; *the split needs a counter fed from the PPU's fetch sequence* — right
  that it needs one, wrong that the PPU must feed it, and the `dot == 321` hook that tried cost
  every ROM of PR-1's set; *a careless implementation could cost every game a few instructions
  per fetch* — right, and it applies to a careful one too;
* the 3D view's result and its one limitation;
* **a correction to the entry's closing paragraph.** It says `exram` and `mmc5test_v2` are "not
  run by the project today" and that running the latter "would quantify MMC5-1". Both are run,
  as `Manual`, since **MF-4c step 4** — and running it does **not** quantify anything: step 3
  ran all three MMC5 ROMs in both configurations and the final frames are byte-identical,
  `mmc5test_v2` being an interactive register test whose final frame is its menu. The paragraph
  is kept with that clause struck through and the correction beside it, and the section's count
  line now says whose DIFFERS it is.

**`BOARD-FEATURES.md`** gains the feature's row — the mechanism, the seven titles, both measured
columns, the gated-off figures and the command — and four edits elsewhere: "three features are
parked" becomes four, the umbrella's row and its mapper list take it, the "first entry that is
not a board" note gains the second and largest such entry, and MF-VS's "the worst figure of the
one *board* feature that is not in `default`" paragraph is left as written with the two rows that
have joined it since recorded underneath.

**Q-177** was written at step 1 and is unchanged: three readings the five emulators disagree on,
each shipped as the page reads, each a one-expression change if R0 prefers another, and each
pinned by a test whose message names the alternative.
- [ ] 6/6 the final check
