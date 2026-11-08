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

### The shape, and why it costs no other board

Entry MMC5-1 names the trap: "a careless implementation could cost every game a few
instructions per fetch", and the safe shape is "decide everything decidable per *register
write*, and leave the per-fetch path a load and a branch". This lot goes one better and leaves
the per-fetch path of every other board **untouched**, feature on or off, by hanging the whole
thing on a test the PPU already makes.

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
split's tile counter is counted in the same place.

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
| 3 | the report entry MMC5-1, `BOARD-FEATURES.md`, and any Q ticket | — |
| 4 | golden-core against the base commit, and the full test run with both ROM roots | the final check |
| 5 | the callgrind campaign | the final check |
| 6 | the final check | the final check |

- [x] 1/6 the feature, the board, mode 1, the split, the nametable hook, the savestate

### 1/6 — the feature, the board, the two modes, the nametable hook and the savestate

**The feature.** `board-mmc5-exattr-split`, off by default, **not** in `parked-boards` (that
umbrella is for boards left out *for a figure*; this one is out for a decision). Its
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
- [ ] 2/6 the 3D view and the 2D-versus-3D palette agreement test
- [ ] 3/6 the report entry, `BOARD-FEATURES.md` and the Q tickets
- [ ] 4/6 golden-core and the suites with both ROM roots
- [ ] 5/6 the callgrind campaign
- [ ] 6/6 the final check
