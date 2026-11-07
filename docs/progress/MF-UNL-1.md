# MF-UNL-1 — the JV001 protection ASIC and Sachen's register ASICs

Lot **MF-UNL-1** of `docs/review/UNL-census.md` §7.2, the first of the twelve MF-UNL lots and
the one with the best titles-per-hour in the plan: **nine numbers, three boards, three Cargo
features**, and the whole of the JV001 family is *one chip*.

Base commit: **`481f683d`** (`Z2-patch (3/3)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| | |
| --- | --- |
| `board-jv001` | mappers **36, 132, 136, 147, 172, 173** — the TXC 05-00002-010 / Sachen JV001, one chip with six wirings |
| `board-sachen-8reg` | mappers **150, 243** — Sachen's eight-register ASIC |
| `board-133` | mapper **133** — Sachen 3009 / 72008, a Namco-108 clone with an analog feedback path |

## The steps

- [x] **0/8 — the plan.** This file: the nine numbers, the sources read for each, the three
  features, and the shape each board takes in this tree.
- [x] **1/8 — `board-jv001`, the chip.** `Jv001` in `cartridges/mappers/jv001.rs`, a
  `DecodeBoard` variant: the ASIC's six registers, the $4100-$4103 window (mask $E103), the
  $8000-$FFFF latch, and the six wirings as fields. `registry.rs` arms, `SUPPORTED_MAPPERS`
  entries, `board_memory_sizes` rows, and the tests the census lists from each page.
- [x] **2/8 — `board-sachen-8reg`.** `Sachen8Reg`, the index/data pair at $4100/$4101 (mask
  $C101) with registers 0, 1 and 3 as **no-ops**, R7's four mirroring modes (one of which is a
  nametable-page override), and 243's extra CHR bit on R2.
- [x] **3/8 — `board-133`.** The 72-pin decode, with the choice between the 60-pin (Namco-108
  clone, $8000/$8001, mask $E001, analog feedback) and the 72-pin ($4100, mask $E100) decodes
  recorded in a code comment.
- [x] **4/8 — the four sweeps and the feature matrix.** `req_core_100_*`, `req_core_102_*`, the
  save-state round trip and the reset sweep in every configuration the lot adds; a save-state
  round trip per board with the mapper state dirtied first; `cargo fmt --check` and clippy
  `-D warnings` in the four configurations.
- [x] **5/8 — the documentation.** Three rows in `docs/review/BOARD-FEATURES.md`, the Cargo
  feature comments, and Q-140.
- [x] **6/8 — the callgrind campaign.** PR-1's thirteen ROMs and level 2's five games against
  `481f683d`, every compared commit built from `git archive` in a fresh directory, and the
  feature decision for each of the three boards.
- [x] **7/8 — golden-core against the base commit** over the 98 commercial originals, and the
  full check with `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set.
- [x] **8/8 — the journal and the final pass.**

## The nine numbers, and the page each is built from

Every figure below comes from the number's own **nesdev wiki** page, cross-checked against
`third_party/mesen2`'s class for the same number. Where the two differ the page wins and the
divergence is named in the board's own doc comment — which happened four times in this lot and
is listed under "Four places where this build leaves Mesen 2 for the page" below.

### The JV001 / TXC 05-00002-010, six numbers and one chip

The chip holds, in the words of 132's and 173's pages, **P, R and Output (three bits each) and
S, inCrement and inVert (one bit each)**; 136's, 147's and 172's pages describe the same silicon
as **Input, Output and Register (six bits each) with Mode and Invert**. They are the same chip:
the second description is the first with S folded into a four-bit accumulator, which is why this
build has **one** `Jv001` and an `accumulator_mask` of `$07` or `$0F`.

| # | board | accumulator | CPU data pins | what Output drives | mirroring |
| ---: | --- | :-: | --- | --- | :-: |
| 36 | TXC 01-22000-400 | `$07` | **D4-D5** | 32 KiB PRG `Output & 3`; CHR from the board's own $4200 latch | — |
| 132 | TXC 05-00002-010 (three PCBs) | `$07` | D0-D3 | 32 KiB PRG `(Output >> 2) & 1`, 8 KiB CHR `Output & 3` | — |
| 136 | Sachen 3011 | `$0F` | D0-D5 | 8 KiB CHR `Output & 7`, 32 KiB PRG `(Output >> 4) & 1` | — |
| 147 | Sachen 3018 (60-pin) | `$0F` | **D2-D7** | PRG `((Output & $20) >> 4) \| (Output & 1)`, CHR `(Output & $1E) >> 1` | — |
| 172 | Super Mega P-4070 | `$0F` | **D0-D5 reversed** | 8 KiB CHR `Output & 3` | **yes**, from Invert |
| 173 | Idea-Tek | `$07` | D0-D3 | CHR A13 `Output & 1`, **CHR A14 `!Invert`**, CHR A15 `Output & 2` | — |

The three parameters the census names are all here: the **data-bit order** (172 reverses D0-D5,
147 rotates the byte by two, 36 takes D4-D5), the **protection read width** (36 two bits in
D4-D5, 132 and 173 the low four, 136 and 172 the low six, 147 six in D2-D7) and the **bool for
whether a $8000 write drives mirroring** (172 alone).

### Sachen's eight-register ASIC, 150 and 243

An index write at $4100 and a data write at $4101, both **mask $C101**, both three bits wide,
and all eight registers fully readable and writable. What each drives:

| register | 150 | 243 |
| --- | --- | --- |
| R0, R1, R3 | **no external effect** | **no external effect** |
| R2 | unused | CHR A13 |
| R4 | CHR A15 | CHR A14 |
| R5 | PRG A16-A15 | PRG A16-A15 |
| R6 | CHR A14-A13 | CHR A16-A15 |
| R7 | nametable mirroring, four modes | the same four modes |

R7's four modes are `S0-S0-S0-S1` ("lower right unique, or vertically-flipped L"), Horizontal,
Vertical and "Single-screen, page 1". The first is the one this core cannot say with a
`NametableMirroring`, so the board answers it through `Mapper::nametable_page` — §4.5's cheap
level, the mechanism mappers 95, 118, 158 and 207 already use, and **not** a `read_nametable`
body.

### Mapper 133, Sachen 3009 / 72008

Two decodes on one page, and the code comment step 3 leaves records the choice:

* the **60-pin** Sachen 3009 is a Namco-108 clone ("ASLIC AX-24G") with a select/data pair at
  $8000/$8001, mask $E001, and **two bits of analog feedback** that "may cause rapid bank
  switching if unstable";
* the **72-pin** Sachen 72008 is one latch: `$4100: [.... .PCC]`, mask $E100.

The page's own closing sentence settles it: *"Only the 72-pin variant requires emulation, as the
60-pin version contains backwards-compatible bankswitching code."* So `board-133` is the 72-pin
decode, the analog feedback path is not modelled, and the comment says why.

## Four places where this build leaves Mesen 2 for the page

1. **Mapper 136's PRG line.** The page gives `PRG ROM bank = (Output >> 4) & 1` beside the CHR
   formula and names the one game that uses it (*Mahjong Companion*); Mesen's `Sachen_136` only
   ever selects PRG bank 0. This build implements the page's line. On a 32 KiB image the single
   bank makes it a no-op, which is every other title of the number.
2. **Mapper 172's CHR mask.** The page says `(Output &3)`; Mesen passes the whole `Output` and
   lets its page table wrap. Identical on the board's 32 KiB CHR; this build masks, as the page
   does.
3. **Mapper 173's Y flag does not exist here.** Mesen carries a `_yFlag` recomputed on every
   chip read *and* write, and uses it for CHR A14. On this number the flag is `!Invert` after
   every write and `true` after every read — the read path being an artefact of modelling the
   latch, since the page says flatly "This value is immediately inverted and used as CHR A14".
   This build reads `!invert` where the banks are computed, so the protection read has **no side
   effect at all** and `try_read_prg` (a `&self` method) can answer it.
4. **Mapper 173 has no bus conflicts, and that is Q-140.** The census's §6.2 row and its test
   list give 173 `bus`; the page gives the board no bus-conflict line and its one paragraph on
   *撲克精靈* says the opposite — the game "will not run as Mapper 3 on emulators that emulate
   bus conflicts (such as FCEUX) …, nor will it run on an actual Nintendo CNROM board, for the
   same reason". Giving the board conflicts would break the very title the lot is for. **Q-140**
   records the discrepancy and the choice, and
   `q_140_mapper_173_does_not_and_the_poker_jingling_write_reaches_the_latch_whole` pins it.

## What is recorded rather than built

* **Mapper 133's analog feedback path** (the 60-pin decode) — the page's own words are that the
  60-pin version does not need emulating.
* **Mapper 150's solder pad.** "The circuit board has a solder pad that selects whether ASIC pin
  14 is connected to CPU D2 or to Vcc"; in the Vcc setting writes are seen OR'd with $04 and D2
  is open bus on reads. This build has no cartridge DIP mechanism (only the VS. System has one),
  so it implements the D2-connected setting, which is Mesen's default too.
* **The undriven bits of a protection read.** The hardware floats D6-D7 (and D0-D1 on 36);
  `Mapper::try_read_prg` returns a whole byte and cannot see the bus, so the undriven bits read
  as **0**. The same compromise `Multicart225::cpu_peek_override` already makes, and no page asks
  a game to read past the bits it drives.
* **No ROM runs any of these nine numbers.** §5 of the census counted it: no file in
  `assets-free/test-roms` uses any of the 199 unlicensed numbers, so every test below is written
  from the board's own register table and the ignored-ROM tests of this lot are vacuous by
  construction. The titles R0 would have to source are in §7.2.

## The figures

### Step 1 — `board-jv001`

`Jv001` is **one `DecodeBoard` variant** and one new forward on that enum, `try_read_prg`
(`#[inline(never)]`, the shape MF-6c step 2 measured). `SmallBoard` and `IrqBoard` grow nothing;
`Board` grows no discriminant. Three clippy allows were added with their reasons: a
`match_same_arms` on `Jv001::chr_bank` (132 and 172 answer `Output & 3` for two different
documented reasons), and `too_many_lines` on `registry::create_board`, which the six-number arm
pushed to 103 — the attribute `board_memory_sizes` beside it already carries.

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **936 passed, 0 failed, 9 ignored** (925 / 0 / 9 at `481f683d`: **+11 tests**) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes` | clean |

### Step 2 — `board-sachen-8reg`

`Sachen8Reg` is a second `DecodeBoard` variant and **one** new forward on that enum beyond step
1's, `nametable_page` (`#[inline(never)]`, behind `Cartridge::nametable_pages_vary`, which gains
150 and 243). Mode 0 of R7 is a page override and not a `read_nametable` body, so Q-123's
two-body dispatch is untouched. `board_memory_sizes` gains a real row — `prg_ram: Some(0)` —
because mask $C101 puts both ports inside $6000-$7FFF. Q-141 is mode 3.

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **941 passed, 0 failed, 9 ignored** (+5 over step 1) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes` | clean |

### Step 8 — the final pass

`git diff --stat 481f683d HEAD`:

    docs/progress/MF-UNL-1.md                            | 377 +++++
    docs/review/BOARD-FEATURES.md                        |  42 +-
    rust/crates/zax-nes/Cargo.toml                       |  36 ++
    .../zax-nes/src/cartridges/mappers/decode_boards.rs  |  51 +-
    rust/crates/zax-nes/src/cartridges/mappers/jv001.rs  | 539 +++++
    rust/crates/zax-nes/src/cartridges/mappers/mod.rs    |  12 +
    .../zax-nes/src/cartridges/mappers/sachen_72008.rs   | 103 +++
    .../zax-nes/src/cartridges/mappers/sachen_8reg.rs    | 234 +++
    rust/crates/zax-nes/src/cartridges/mod.rs            |   8 +-
    rust/crates/zax-nes/src/cartridges/registry.rs       |  69 ++
    rust/crates/zax-nes/tests/mapper_tests.rs            | 720 ++++++-
    11 files changed, 2183 insertions(+), 8 deletions(-)

**Eight deletions in the whole lot**, and they are all the two lines a `matches!` list and a
`use` block grew. The one file of the existing core that changes at all is
`cartridges/mod.rs`, by **two numbers in `nametable_pages_vary`** (150 and 243) and the comment
that says why. `registry.rs` gains nine `SUPPORTED_MAPPERS` entries, three `create_board` arms,
one `board_memory_sizes` row, two comments naming the seven numbers deliberately *absent* from
that function, and one `#[allow(clippy::too_many_lines)]`. `decode_boards.rs` gains three
variants, three `inner!` arms, three features in the umbrella's `compile_error!` guard and two
widened `#[cfg]`s on forwards that already existed.

**No existing mapper changes behaviour and no save format changes.** `NesCore::STATE_VERSION`
stays at 8 and `WIDE_STATE_VERSION` at 9; the diff touches neither `nes_core.rs` nor `state.rs`,
and every new `save_registers` block is inside a board no earlier state can name. golden-core over
the 98 commercial originals is byte-identical to `481f683d`'s (step 7), which is that claim
measured rather than asserted.

#### The final check, on the committed tree

| gate | result |
| --- | --- |
| `cargo fmt --all --check` | **clean** |
| clippy `-D warnings`, `-p zax-nes`, **default** | **clean** |
| clippy `-D warnings`, `-p zax-nes`, `--no-default-features` | **clean** |
| clippy `-D warnings`, `-p zax-nes`, `--features parked-boards` | **clean** |
| clippy `-D warnings`, `-p zax-nes`, **each new feature alone** and the three together | **clean** |
| clippy `-D warnings`, `-p zax-harness` | **clean** |
| `cargo test --release -p zax-nes -p zax-harness` | **944 / 0 / 9** |
| the same, `--no-fail-fast -- --include-ignored`, `ZAXNES_ROMS` + `ZAXNES_TEST_ROMS` | **953 / 0 / 0** |
| golden-core, 98 commercial originals, 1 200 frames | **byte-identical to `481f683d`** |
| PR-1's thirteen ROMs | worst **+0.001 %**, set total **−0.000 %** |
| level 2's five games, `core` | **all five at or below the base** |
| level 2's five games, `total` | four inside; `smb3` +0.023 %, attributed to Lua's `ltable.c` by a same-behaviour pair (step 6) |
| `unsafe` | **none** |
| `spec/` | untouched but for the two new tickets, Q-140 and Q-141 |

**Disk**: G: stayed between 23 and 31 GiB free, above the brief's 5 GiB floor. Four clean builds,
each in its own fresh directory with its own `CARGO_TARGET_DIR` and deleted as soon as its binary
was out; the tars removed after. No GPU campaign, and never two test suites at once.

The journal entry is in `G:/3dsen/cleanroom/journal/JOURNAL.md`, dated 2026-11-07.

### Step 7 — golden-core, and the full check with the ROM roots set

#### golden-core over the 98 commercial originals, 1 200 frames each

The two Linux binaries of step 6 — `unl1base` from `481f683d` and `unl1` from the lot's tree, each
built clean — run against the same set with the same frame count:

    zax-harness-<label> golden-core /mnt/g/3DSen/test-originals gc-<label>.jsonl 1200

| pair | rows differing |
| --- | ---: |
| **`481f683d` → this lot** | **0 — `diff` of the two 98-row JSONL files is clean** |

**Byte-identical**, which is the lot's requirement met at the strongest strength the instrument
offers: `pixel_sha256`, `tiles_sha256`, the audio hash and the cycle counts all agree on every one
of the 98 ROMs over 1 200 frames each. golden-core runs with no hack code, so there is nothing
else the comparison could be measuring — and nothing else it needed to measure, this lot changing
no line any existing board or the PPU reaches.

#### The suite, with the ROM roots set

    ZAXNES_ROMS=G:/3dsen/demo/roms     ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms     cargo test --release -p zax-nes -p zax-harness --no-fail-fast -- --include-ignored

| run | result |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **944 passed, 0 failed, 9 ignored** |
| the same, `--include-ignored --no-fail-fast`, roots set | **953 passed, 0 failed, 0 ignored** |

**Nine of nine ignored tests run and pass**, which is what the two variables buy: without them
they pass vacuously. The nine are `req_core_113_dmc_dma_during_read4_is_p3`,
`the_sprite_dma_and_dmc_dma_collision_is_not_cycle_exact`,
`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`,
`the_cost_of_one_capture_and_one_restore`,
`mr_bus_1_uxrom_submapper_2_write_is_anded_with_the_rom_byte`,
`golden_pixels_tiles_audio_cycles_and_blargg_match_up_to_frame_1200`,
`zr_2_a_zone_fetch_leaves_smb3s_ground_row_whole_under_the_widescreen_hack` and two more in
`zax-harness`. **None of them exercises a number of this lot** — no file in
`assets-free/test-roms` uses any of the nine (§5 of the census) — so what they prove is that the
lot moved nothing they do cover.

### Step 6 — the callgrind campaign, and the feature decision

Four binaries, each built by `build-clean.sh` from a `git archive` (or `git ls-files`) tar into a
**fresh** directory with its own `CARGO_TARGET_DIR`, which is then deleted:

| label | tree |
| --- | --- |
| `unl1base` | **`481f683d`**, the lot's base commit |
| `unl1` | `3214eb87`, the lot through step 5, the three features in `default` |
| `unl1off` | the same source with the three features **out of `default`** — no new board code reachable |
| `unl1swap` | the same source as `unl1` with **two disjoint `create_board` arms swapped** — identical emulation, different layout |

#### PR-1's thirteen ROMs, 300 frames, no pack — `unl1` against `unl1base`

| ROM | base | lot | delta | % |
| --- | ---: | ---: | ---: | ---: |
| `smb1` | 5 996 342 771 | 5 996 390 654 | **+47 883** | **+0.001** |
| `readbuf` | 6 834 481 600 | 6 834 510 597 | +28 997 | +0.000 |
| `duelito` | 7 380 412 743 | 7 380 439 413 | +26 670 | +0.000 |
| `oamstr` | 3 318 458 847 | 3 318 468 347 | +9 500 | +0.000 |
| `metroid` | 5 115 544 329 | 5 115 551 860 | +7 531 | +0.000 |
| `m0` | 5 517 376 709 | 5 517 382 587 | +5 878 | +0.000 |
| `mmc5` | 11 266 782 006 | 11 266 779 975 | −2 031 | −0.000 |
| `z2` | 6 291 504 795 | 6 291 500 773 | −4 022 | −0.000 |
| `mm2` | 6 017 968 607 | 6 017 962 145 | −6 462 | −0.000 |
| `smb3` | 7 949 377 635 | 7 949 353 143 | −24 492 | −0.000 |
| `vblnmi` | 3 308 318 704 | 3 308 266 130 | −52 574 | −0.002 |
| `m4` | 5 468 255 607 | 5 468 199 057 | −56 550 | −0.001 |
| `m1` | 4 881 265 531 | 4 881 136 565 | −128 966 | −0.003 |
| **total** | 79 346 089 884 | 79 345 941 246 | **−148 638** | **−0.000** |

**Worst ROM +0.001 %**, against a tolerance of +0.02 %, and the set total is below the base.

#### Level 2's five games, 600 frames, packs loaded, inputs pressed

| game | `core` delta | `core` % | `total` delta | `total` % |
| --- | ---: | ---: | ---: | ---: |
| `metroid` | **−9 952** | −0.000 | +95 510 | +0.001 |
| `mm2` | **−18 944** | −0.000 | +125 160 | +0.001 |
| `smb1` | **+0** | +0.000 | +1 148 500 | +0.008 |
| `smb3` | **−20 823** | −0.000 | +5 666 251 | **+0.023** |
| `zelda2` | **+0** | +0.000 | +1 333 089 | +0.009 |

**All five `core` figures are at or below the base**, three of them below and two identical to
the instruction. Four of the five `total` figures are inside the tolerance. The fifth,
`smb3/total` **+0.023 %**, is outside it, and the rest of this step is what it is.

#### Where `smb3/total` is, and why it is not the boards'

The per-function diff of that one run, exclusive counts, `unl1` against `unl1base`:

| symbol | delta |
| --- | ---: |
| `luaH_getshortstr` | **+23 042 742** and **−20 058 471** (two annotation entries) |
| `getgeneric` | **+9 291 059** and **−7 043 815** |
| `luaH_set` | +330 712 |
| `__memcmp_avx2_movbe` | +109 057 |
| `__memcpy_avx_unaligned_erms` | +33 795 |
| `Board::chr_peek_view` | **−20 823** |
| PROGRAM TOTALS | **+5 668 674** |

Lua's string-key hash table **is** the figure. No symbol of this project moves by more than
20 823 on that game and the one that does moves **down**; the stage table says the same —
`core`, `ppu`, `apu` and `draw_list` are `+0` or below and the whole move is in `convert`
(+0.190 %) and `lua_hooks` (+0.335 %), the 3D engine converting against a differently-laid-out
heap.

**The same-behaviour pair settles it.** `unl1swap` is `unl1`'s source with two **disjoint** match
arms of `registry::create_board` exchanged: the same boards, the same registers, the same
emulation, a different binary layout. Against each other:

| | `core` | `total` |
| --- | ---: | ---: |
| `metroid` | **+0** | −115 283 (−0.001 %) |
| `mm2` | **+0** | −47 965 (−0.000 %) |
| `smb1` | **+0** | +2 146 044 (**+0.016 %**) |
| `smb3` | **+0** | −2 536 876 (**−0.010 %**) |
| `zelda2` | **+0** | +1 027 482 (+0.007 %) |

Two binaries whose emulation is identical differ by **−0.010 % to +0.016 %** on `total` and by
**nothing at all** on `core`. And against the base commit the variant is `smb1/total`
**+0.024 %** where `unl1` is +0.008 %, and `smb3/total` +0.013 % where `unl1` is +0.023 % — the
two games trade places. `smb3/total` +0.023 % is inside the spread of an instrument that cannot
resolve it, which is the same finding `board-77`'s row records (+0.024 % on the same game,
"entirely Lua's `ltable.c`") now with a pair behind it.

#### Gated off, the shipped-today build pays nothing

`unl1off` against `unl1base` — the lot's source with the three features out of `default`:

* PR-1's thirteen: set total **−7 224 (−0.000 %)**, worst ROM `metroid` / `readbuf`
  **+0.001 %**, six of thirteen below.
* Level 2: **all five `core` figures +0, identical to the instruction**; `total` +0.000 % to
  **+0.004 %** (`smb1`), every one inside the tolerance.

#### The feature decision

| feature | decision | on what |
| --- | :-: | --- |
| `board-jv001` | **`default`** | PR-1 worst +0.001 %, all five level-2 `core` at or below the base |
| `board-sachen-8reg` | **`default`** | the same measurement; the three were measured together, as MF-6a3 step 1 and MF-6c step 2 measured their steps' boards together |
| `board-133` | **`default`** | the same |

`parked-boards` is unchanged: it still holds `board-vs-system` alone. No board of this lot is
above the tolerance on any figure the instrument can resolve, and none is dropped — all three are
one `--no-default-features --features …` away either direction, which is the owner's rule.

### Step 5 — the documentation

Three rows in `docs/review/BOARD-FEATURES.md`, between `board-vs-system` and the `parked-boards`
umbrella, with the cost columns filled by step 6; the `--no-default-features` and
`parked-boards` test counts in that file brought to this lot's (783 / 0 / 8 and 934 / 0 / 8
against 881 by default); the three features added to its "how to turn a board off" command; and
one Cargo feature comment each in `rust/crates/zax-nes/Cargo.toml`.

Two question tickets, both in `spec/changes/` and both referenced from the code:

* **Q-140** — mapper 173 and bus conflicts. The census gives the number `bus`; the page gives the
  board no such line and its *撲克精靈* paragraph is an argument that the board has **none**.
  Provisional choice: no bus conflicts, which is also Mesen 2's reading. The ticket records that
  the choice is observationally cheap either way on this family — no board of the six uses the
  value written at $8000-$FFFF — and names what would overturn it.
* **Q-141** — "Single-screen, page 1" in R7 mode 3 of mappers 150 and 243. The page names its two
  screens `S0` and `S1` four lines above, so "page 1" is `S1` and this build selects
  `SingleScreenB`; Mesen 2 reads the same line as screen A. The ticket records both readings and
  the symptom if the choice is wrong (a one-screen game drawing on the page it is not writing).

### Step 4 — the feature matrix

Every configuration below builds, passes `cargo fmt --check` and clippy `-D warnings` on
`zax-nes`, and runs the four sweeps of §7.1 — `req_core_100_every_required_mapper_is_registered`
(whose expected list is **built** from the features, so each configuration checks its own),
`req_core_102_any_other_mapper_is_refused`, `req_core_090_the_mapper_state_survives_a_save_and_restore`
and `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state`.

| configuration | `cargo test -p zax-nes` | clippy `-D warnings` |
| --- | --- | :-: |
| `default` | **881 / 0 / 8** (862 / 0 / 7 at `481f683d`) | clean |
| `--no-default-features` | **783 / 0 / 8** — the figure `BOARD-FEATURES.md` already records, unchanged | clean |
| `--features parked-boards` | **934 / 0 / 8** (907 at the base) | clean |
| `--no-default-features --features board-jv001` | **794 / 0 / 8** (+11) | clean |
| `--no-default-features --features board-sachen-8reg` | **788 / 0 / 8** (+5) | clean |
| `--no-default-features --features board-133` | **786 / 0 / 8** (+3) | clean |
| `--no-default-features` + all three | **802 / 0 / 8** (+19, the three sums) | clean |

The ignored count is 8 in every configuration and 9 for `-p zax-nes -p zax-harness` together,
which is `481f683d`'s; this lot adds no ignored test, because **no ROM in
`assets-free/test-roms` uses any of the nine numbers** (§5 of the census).

And the umbrella still refuses to be selected alone —
`cargo build -p zax-nes --no-default-features --features board-decode-boards` stops at
`decode_boards.rs`'s own `compile_error!`, whose `any(…)` list gained the lot's three features.

### Step 3 — `board-133`

`Sachen72008` is the lot's third and last `DecodeBoard` variant and adds **no** forward: one
latch, one byte of savestate. The 60-pin Sachen 3009's Namco-108 decode and its analog feedback
path are **not** built, and the page's own closing sentence is the reason — recorded in
`Sachen72008`'s doc comment as the lot asked.

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **944 passed, 0 failed, 9 ignored** (+3 over step 2) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes` | clean |
