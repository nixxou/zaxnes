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
- [ ] **3/8 — `board-133`.** The 72-pin decode, with the choice between the 60-pin (Namco-108
  clone, $8000/$8001, mask $E001, analog feedback) and the 72-pin ($4100, mask $E100) decodes
  recorded in a code comment.
- [ ] **4/8 — the four sweeps and the feature matrix.** `req_core_100_*`, `req_core_102_*`, the
  save-state round trip and the reset sweep in every configuration the lot adds; a save-state
  round trip per board with the mapper state dirtied first; `cargo fmt --check` and clippy
  `-D warnings` in the four configurations.
- [ ] **5/8 — the documentation.** Three rows in `docs/review/BOARD-FEATURES.md`, the Cargo
  feature comments, and Q-140.
- [ ] **6/8 — the callgrind campaign.** PR-1's thirteen ROMs and level 2's five games against
  `481f683d`, every compared commit built from `git archive` in a fresh directory, and the
  feature decision for each of the three boards.
- [ ] **7/8 — golden-core against the base commit** over the 98 commercial originals, and the
  full check with `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set.
- [ ] **8/8 — the journal and the final pass.**

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
