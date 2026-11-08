# MF-UNL-10 — Dōngdá PEC-9588, mapper 164

Lot **MF-UNL-10** of `docs/review/UNL-census.md` §7.2, the tenth of the twelve MF-UNL lots:
**one number, one board, one Cargo feature, 6 original titles.**

Base commit: **`909c0edd`** (`MF-UNL-9 (6/6)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

The six titles the census lists, with the PEC-9588 catalogue numbers its page gives:
*Final Fantasy 太空戰士 V* (G-003), *大話西游* (G-004), *櫻桃小丸子* (G-005), *岳飛傳* (G-006),
*Dark Seed — 黑暗之蛊* (G-009), *口袋精靈: 金* (Pokémon: Gold Edition).

Source for the board: nesdev, **"INES Mapper 164"** — the whole page, its five register diagrams
included. Source for the chip: Microchip **DS20001795F**, *93AA66A/B/C, 93LC66A/B/C, 93C66A/B/C —
4-Kbit Microwire Compatible Serial EEPROM*, tables 1-3 and 1-4 and sections 2.0-2.9.

## Step 0 — the inventory and the plan

### What exists, and what does not

| census item (§7.2 MF-UNL-10) | state at `909c0edd` |
| --- | --- |
| step 1 — a **Microwire** serial EEPROM device, 93C66, 512 bytes | **missing**; the two EEPROMs here are I²C (`Eeprom24C0X`, `board-eeprom`) |
| step 2 — `Pec9588`: the five registers at $5000/$5100/$5200/$5300/$5500 | **missing**; mapper 164 is in no list |
| step 2 — mirroring from $5300 | missing |
| step 2 — the **1 bpp** CHR addressing mode | missing |
| step 3 — §7.1's three common things | to do |

Nothing of the board exists. What *does* exist and is reused unchanged:

* **the persistence path** — `Cartridge::battery_ram` over `CartridgeMemory::prg_ram` from
  `registry::battery_offset`, which is the path `Eeprom24C0X`'s bytes already take (MF-6c step 3)
  and which `battery_offset` was given its second parameter for;
* **the PPU-address-bus latch** — `Cartridge::watches_ppu_reads` /
  `watches_ppu_bus_without_read` and `Mapper::on_ppu_address_bus`, built for mapper 96 and already
  carrying "PPU A9 latched on last rise of PPU A13" for mappers 162 and 163 (`ChrWindowBoard`);
* **`DecodeBoard`** — the shared `Board` variant, so this lot adds **no `Board` discriminant**.

### The three §7.2 claims that the number's own page contradicts — **Q-170**

MF-UNL-2 established the rule: §7.2's steps were drafted from Mesen's `MapperFactory`, and where
they contradict the number's own page **the page wins**, each case in a Q ticket. Three here:

1. §7.2 step 1: "93C66 is 512 bytes at **16-bit words**". The page: "The EEPROM's *ORG* pin is
   tied to **GND**, selecting a word size of **8 bits**." 512 bytes either way, but the word size
   decides the **address width** — 9 bits (x8) against 8 (x16) — and so every command's bit count.
   The page wins: x8, nine address bits, the datasheet's table 1-4.
2. §7.2 step 1: the device is "shared with, or shared by, 157/159's X24C01 and 24C02". It cannot
   be: those are **two-wire I²C** parts with START/STOP conditions on SDA, this is **three-wire
   Microwire** with CS, CLK and DI and a start *bit*. One state machine cannot be both, and the
   lot brief forbids changing the existing boards' behaviour either way. A second type.
3. §7.2 describes the lot as "a 1 bpp video mode **and** a 93C66", which is right, but puts the
   EEPROM "in `CartridgeMemory`". The bytes go in `prg_ram` behind `battery_offset`, where
   `Eeprom24C0X`'s already are; the *device* is a board field, as `EepromPort` is.

Q-170 also records the four things the page leaves open (the unpopulated second chip, the DO line's
undriven level, the volatile RAM size an iNES 1.0 header cannot give, and whether the 1 bpp
addressing applies to PPU **writes**).

### The six steps

§7.2's steps 1 and 2 are **one commit**: the device is reachable only from the board, so a tree
with the first and not the second has a module nothing calls. The six below are the lot's own.

- [x] **0/6** the plan — this note.
- [x] **1/6** the Microwire device and the board, with Q-170.
- [x] **2/6** the tests the census lists from the page.
- [x] **3/6** fmt and clippy `-D warnings` plus the sweeps in the four configurations (default,
      `--no-default-features`, `--features parked-boards`, `board-164` alone).
- [x] **4/6** the callgrind campaign against `909c0edd`, the feature decision and the
      `BOARD-FEATURES.md` row.
- [x] **5/6** golden-core byte-identical over the 98 originals, and the full
      `--include-ignored --no-fail-fast` suite with both ROM roots.
- [x] **6/6** the census note, the journal, the close.

[PROGRESS] MF-UNL-10 step 0/6 done: the plan

## Step 1 — the Microwire device and the board

### `Eeprom93C66` (`rust/crates/zax-nes/src/cartridges/mappers/eeprom93c66.rs`)

Table 1-4 of DS20001795F in full — seven instructions, a start bit, a two-bit opcode, **nine**
address bits and eight data bits on the two that write:

| instruction | opcode | address field | data | what it does |
| --- | --- | --- | --- | --- |
| READ | `10` | `A8..A0` | — | a dummy zero, then `D7..D0`, then the next word, sequentially while CS is high |
| WRITE | `01` | `A8..A0` | `D7..D0` | the word, committed on the **rising edge of CLK on the last data bit** (the 93C variant) |
| ERASE | `11` | `A8..A0` | — | the word to `$FF` |
| ERAL | `00` | `10xxxxxxx` | — | the array to `$FF` |
| WRAL | `00` | `01xxxxxxx` | `D7..D0` | the array to the datum |
| EWEN | `00` | `11xxxxxxx` | — | programming enabled |
| EWDS | `00` | `00xxxxxxx` | — | programming disabled, which is the power-on state |

The four behavioural claims, each quoted in the module note: the start bit is "CS and DI … both
high with respect to the positive edge of CLK for the first time"; "a dummy zero bit precedes the
8-bit … output string" and "sequential read is possible when CS is held high"; the 93C commit edge
above, which is why a game that never lowers CS still saves; and "the 93XX66A/B/C powers up in the
Erase/Write Disable (EWDS) state", so a WRITE before an EWEN changes nothing.

The bytes are **not** in the type — `Eeprom24C0X`'s shape exactly — so the two EEPROM boards share
one persistence path and this one's reset cannot wipe a save.

### `Pec9588` (`…/mappers/pec9588.rs`)

| port | mask | what |
| --- | --- | --- |
| `$5000` w | `$FF00` | `CSQM PPPp` — 1 bpp enable, fixed-bank select, PRG A18, mode bit, A17..A14 |
| `$5100` w | `$FF00` | `.... ..PP` — PRG A20..A19 |
| `$5200` w | `$FF00` | `.T.S .C.D` — DI, CLK, chip 1 CS, chip 2 CS |
| `$5300` w | `$FF00` | `M... ....` — mirroring when `$5000.4` is set |
| `$5500` r | `$FF00` | `.... .A..` — DAT in, **inverted** |

The bit order of the two PRG modes is the page's own, and the page proves it twice: `111p0` read
A18-first is `$1C`/`$1E`, the pair it names, where A14-first would be 7/15; and `11111` is bank
`$1F`, which its note about the reset vectors being "at the end of 16 KiB bank #31" confirms from
the game's side. So the 16 KiB bank is `A14 | A15<<1 | … | A20<<6` and in BxROM mode the same
nibble is A18..A15, i.e. the 32 KiB bank number itself.

**1 bpp mode is `ChrWindowBoard`'s latch with a second line on the same edge**: CHR A12 from PPU
A9 and CHR A3 from PPU A0, "both latched on last rise of PPU A13". Three consequences:

* mapper 164 joins **both** of `Cartridge`'s PPU-address-bus lists, for mappers 96/162/163's
  reason and not an IRQ's — the board has none;
* **no window moves on the latch**, unlike those three: A3 is below the 1 KiB window granularity,
  so the substitution is in `Pec9588::chr_address` on `ppu_peek` and `ppu_write` and
  `on_ppu_address_bus` stores two bits and returns;
* `chr_peek_view` answers `None` while the mode is on (Q-170 §2.4).

### What it cost the rest of the library

**No `Board` discriminant**: one `DecodeBoard` variant. Three forwards grow their `cfg` by this
feature — `prg_ram_window` (second board), `try_read_prg` (fifth) and `on_ppu_address_bus`
(fourth) — and all three are behaviour-neutral for the boards already there, which take `Mapper`'s
own default as the un-forwarded dispatch gave them. `Pec9588::ppu_peek` is the one new *body* in a
per-fetch dispatch and is `#[inline(never)]`.

`registry.rs`: the `SUPPORTED_MAPPERS` entry, the `create_board` arm, the `board_memory_sizes` row
(2 KiB + 512) and a `battery_offset` arm — that function becomes a `match`, mapper 1's SOROM case
unchanged.

`cargo build -p zax-nes` and `cargo clippy -p zax-nes --all-targets -- -D warnings` clean;
`cargo test -p zax-nes --test mapper_tests` **341 passed, 0 failed**, the four sweeps included
(`req_core_100_every_required_mapper_is_registered` with the new row,
`req_core_102_any_other_mapper_is_refused`,
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` and
`req_core_090_the_mapper_state_survives_a_save_and_restore`).

The generic save-state sweep **dirties nothing of this board** — its ports are at `$5000-$5500`
and the sweep writes `$8000-$E001` and `$6000` — which is the MF-UNL-2 situation and what step 2's
own round-trip test is for.

## Step 2 — the tests from the page

Nine tests and one helper module, `mapper_tests.rs`. The census lists four things; they are the
first four rows, and the other five are what the page's own register table asks for.

| census item / page claim | test |
| --- | --- |
| "the EEPROM's read and write command sequences, **bit by bit** over the serial line, against the 93C66's documented protocol" | `req_core_102_the_93c66_clocks_the_datasheets_seven_instructions` |
| "a written word survives a save-state round-trip and reaches the save file" | `req_core_090_mapper_164_round_trips_its_ports_its_two_latches_and_its_eeprom` |
| "the 1 bpp mode changes which CHR bits a pattern fetch consumes" | `req_core_102_mapper_164_takes_chr_a3_and_a12_from_the_latch_in_1_bpp_mode` |
| "mirroring from $5300" | `req_core_102_mapper_164_forces_vertical_mirroring_until_5300_takes_over` |
| both PRG modes, both fixed-bank shapes, the `$5100` outer bits | `req_core_102_mapper_164_banks_prg_in_both_modes_with_two_fixed_banks` |
| `$5500` bit 2, **inverted**, under mask `$FF00`, nothing else driven | `req_core_102_mapper_164_answers_the_microwire_line_inverted` |
| Q-170 §2.4, the write path | `q_170_the_1_bpp_substitution_is_on_the_write_path_as_well` |
| Q-170 §2.3, 2 KiB mirrored, the EEPROM out of the CPU window | `q_170_mapper_164_mirrors_2_kib_of_volatile_ram_and_keeps_the_eeprom_out_of_the_window` |
| no IRQ, no expansion audio, and **no window moves** on the PPU bus | `req_core_075_mapper_164_has_no_irq_and_moves_no_window_on_the_ppu_bus` |

The `microwire` module is the `i2c` module of MF-6c step 3 for a three-wire bus: `lines`, `clock`,
`select`/`deselect`, `send`, `receive_bit`, `command`, and one function per instruction. `data_out`
inverts, because the board does.

Four observations in there are the ones that would catch a wrong reading rather than a typo:

* **nine address bits** — `$1FF` and `$0FF` differ by A8 alone, so the x16 reading §7.2 asked for
  would alias them (Q-170 §1.1);
* **MSB-first data** — `$B2` reversed is `$4D`;
* **one dummy zero per burst**, not one per word, with three words read sequentially and the
  address wrapping `$1FF` → `$000`;
* **EWDS at power-on** — the first WRITE of the test, before any EWEN, changes nothing, and after
  an EWDS neither WRITE nor ERASE nor WRAL does.

And in the 1 bpp test: all four of `$0000`/`$0008`/`$1000`/`$1008` read **one** byte with the mode
on, the byte the two latches name, **with no register write between the four groups**; `$0234` and
`$1234` both read `$123C` with both latches set, which is fine Y and the tile index passing
through; moving the address *inside* `$2000-$3FFF` latches nothing, so it is the rise; and
`on_ppu_address_bus_without_read` latches too, which is what proves the number belongs in **both**
of §4.3's lists and not just the first.

`cargo test -p zax-nes --test mapper_tests`: **350 passed, 0 failed** (341 before the lot's tests).
fmt and clippy `-D warnings` clean on `zax-nes --all-targets`.

[PROGRESS] MF-UNL-10 step 2/6 done: the tests from the page

## Step 3 — the four configurations

`board-164 = ["board-decode-boards"]`, which the `board-164`-alone configuration is what caught:
the umbrella every member of that enum implies is not optional, and `compile_error!` in
`decode_boards.rs` names it. The lot's feature was added to that `cfg(not(any(…)))` list in step 1
and to the `implies` here.

The one test helper that had to grow a feature is `latch_a13_rise`, gated on
`any(board-162, board-163)` by MF-UNL-5 and now on `board-164` too: mapper 164 latches on the same
edge, which is the whole reason it is one helper.

| configuration | `cargo fmt --check` | `clippy -p zax-nes --all-targets -- -D warnings` | `--test mapper_tests` |
| --- | --- | --- | ---: |
| default | clean | clean | **350 passed, 0 failed** |
| `--no-default-features` | clean | clean | **103 passed, 0 failed** |
| `--features parked-boards` | clean | clean | **350 passed, 0 failed** |
| `--no-default-features --features board-164` | clean | clean | **112 passed, 0 failed** |

The 112 against 103 is this lot's nine tests: every one of them runs with the board's feature as
the **only** board feature, which is what the owner's rule means by reachable behind its feature.

§7.1's first common thing, the four sweeps over `SUPPORTED_MAPPERS`, in each of the four:

| sweep | default | `--no-default-features` | `parked-boards` | `board-164` alone |
| --- | --- | --- | --- | --- |
| `req_core_100_every_required_mapper_is_registered` | ok | ok | ok | ok |
| `req_core_102_any_other_mapper_is_refused` | ok | ok | ok | ok |
| `req_core_090_the_mapper_state_survives_a_save_and_restore` | ok | ok | ok | ok |
| `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` | ok | ok | ok | ok |

The last two reach mapper 164 only in the configurations where it is on, and both write `$5000`
and `$6000` — so the board's reset, its power-on window layout (bank 0 at `$8000`, bank `$1F` at
`$C000`, forced vertical) and its whole savestate block are swept there as well as in the lot's
own tests.

The `board_memory_sizes` row is asserted from both sides: `2 KiB + 512` of PRG-RAM and 512 bytes of
battery slice in `req_core_102_the_93c66_clocks_the_datasheets_seven_instructions`, and the
window's own 2 KiB wrap in `q_170_mapper_164_mirrors_2_kib_of_volatile_ram_and_keeps_the_eeprom_out_of_the_window`.

[PROGRESS] MF-UNL-10 step 3/6 done: the four configurations

## Step 4 — the campaign, and the one symbol of this build that moves

Binaries built by `build-clean.sh` from their own `git archive` tar into a **fresh** directory with
its own `CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`.

| label | tree | md5 |
| --- | --- | --- |
| `mfunl10-0-base` | **`909c0edd`**, the lot's base commit | `ef4cb0fb5539aba502c24d25dfd669df` |
| `mfunl10-1-lot` | **`69672291`**, step 3 — the lot's last code commit | `d4d92c446abb1cafd5194ccbf35d3b1e` |
| `mfunl10-2-noinline` | the working tree with `#[inline(never)]` on one method: the alternative shape, built, measured and discarded | — |
| `mfunl10-0-again` | a byte copy of `mfunl10-0-base`, the **same-binary floor** | same md5 as the base |

The base binary's md5 is **identical to MF-UNL-9's shipped binary** (`zax-harness-mfunl9-2-ship`),
so this lot's base is that lot's measured binary to the byte — the seventh lot in a row for which
that holds, and built rather than assumed.

### The PR-1 set, thirteen ROMs, 300 frames

| ROM | base | lot | delta | % |
| --- | ---: | ---: | ---: | ---: |
| duelito | 7 380 437 081 | 7 380 415 757 | −21 324 | −0.000 |
| m0 | 5 517 238 614 | 5 517 337 951 | +99 337 | +0.002 |
| m1 | 4 880 979 272 | 4 881 150 120 | +170 848 | +0.004 |
| m4 | 5 467 732 625 | 5 468 206 285 | **+473 660** | **+0.009** |
| metroid | 5 115 535 791 | 5 115 572 501 | +36 710 | +0.001 |
| mm2 | 6 017 978 798 | 6 018 009 599 | +30 801 | +0.001 |
| mmc5 | 11 266 808 876 | 11 266 778 583 | −30 293 | −0.000 |
| oamstr | 3 318 449 790 | 3 318 487 258 | +37 468 | +0.001 |
| readbuf | 6 834 476 427 | 6 834 487 071 | +10 644 | +0.000 |
| smb1 | 5 996 364 493 | 5 996 320 357 | −44 136 | −0.001 |
| smb3 | 7 949 391 667 | 7 949 412 085 | +20 418 | +0.000 |
| vblnmi | 3 308 287 184 | 3 308 280 239 | −6 945 | −0.000 |
| z2 | 6 291 494 540 | 6 291 521 327 | +26 787 | +0.000 |
| **total** | **79 345 175 158** | **79 345 979 133** | **+803 975** | **+0.001** |

Worst ROM **+0.009 %** against a tolerance of **+0.02 %**, four of thirteen below.

### The floor, and why this lot may not hide behind it

MF-UNL-8 step 7 found that the determinism `callgrind-set.sh` claims does not hold for the whole
process on this machine, and five lots since have reported figures smaller than that floor.
**This lot's is not.** The base binary run twice:

| | worst ROM | set total | emulation symbols that moved |
| --- | ---: | ---: | --- |
| base vs the **same binary** | `oamstr` **+0.000 %** (+11 524) | **−90 473** | **none**, at a 20 000 threshold |
| base vs the **lot** | `m4` **+0.009 %** (+473 660) | +803 975 | `NesBus::write_internal` on four ROMs |

So the figure is a real cost. It is still less than half the tolerance, and it is attributable to
one instruction-level decision rather than to the board's own code.

### What moved, and the shape that was chosen by measuring it

At a 20 000 threshold the only symbol of this build that moved anywhere in the thirteen ROMs is
**`NesBus::write_internal`**: `m4` **+445 122**, `m1` +141 527, `m0` +65 621, `smb3` +41 201. Every
other mover is Lua's `ltable.c` (`luaH_getshortstr`, `getgeneric`), in both directions, and it
moves by the same ±32 000 between two runs of the **same** binary.

`write_internal` inlines `Cartridge::cpu_write`, which runs `Mapper::cpu_read_windows` after every
CPU write to the cartridge — and that calls `DecodeBoard::prg_ram_window`, which this lot gives a
**second distinct body** (mapper 178 had the only one). MF-6a step 1 priced exactly this and the
answer it established is `#[inline(never)]`. **So that shape was built and measured too, and it is
worse:**

| shape | ROMs whose `write_internal` moves | worst ROM | PR-1 set total |
| --- | ---: | ---: | ---: |
| inlined — **shipped** | **4** of 13 | `m4` **+0.009 %** | **+803 975 (+0.001 %)** |
| `#[inline(never)]` | **11** of 13 | `m1` +0.006 % | +1 590 577 (+0.002 %) |

Out of line the *call site* lands in `write_internal` for every board instead of a body for one:
`mmc5` +407 642, `m1` +308 168, `m4` +234 107, `readbuf` +175 123 and seven more. Twice the set
total and three times as many ROMs, for a slightly better worst ROM. Both are inside the
tolerance; the shipped tree is the lower of the two, and `Pec9588::prg_ram_window`'s own note
carries this table so that a later lot does not re-add the attribute on the usual precedent.

### Level 2, the five games with their 3D packs, 600 frames after 120 of warm-up

| game | stage | base | lot | delta | % |
| --- | --- | ---: | ---: | ---: | ---: |
| metroid | core | 11 146 849 562 | 11 146 883 813 | +34 251 | +0.000 |
| metroid | total | 13 735 121 268 | 13 735 166 276 | +45 008 | +0.000 |
| mm2 | core | 12 037 230 685 | 12 037 289 565 | +58 880 | +0.000 |
| mm2 | total | 18 976 662 258 | 18 976 682 959 | +20 701 | +0.000 |
| smb1 | core | 11 515 129 187 | 11 515 129 187 | **+0** | **+0.000** |
| smb1 | total | 13 588 230 618 | 13 589 335 899 | +1 105 281 | +0.008 |
| smb3 | core | 15 315 040 380 | 15 315 191 123 | +150 743 | +0.001 |
| smb3 | total | 24 189 638 381 | 24 191 334 113 | +1 695 732 | +0.007 |
| zelda2 | core | 12 774 981 932 | 12 775 009 516 | +27 584 | +0.000 |
| zelda2 | total | 15 359 014 140 | 15 359 212 480 | +198 340 | +0.001 |

**`ppu`, `apu` and `draw_list` are identical to the instruction on all five games.** `core` is +0
on `smb1` and +0.000 % to +0.001 % on the other four. **Every `total` is inside the tolerance**, so
the lot's performance rule needs no same-behaviour pair — but the two largest, `smb3` +0.007 % and
`smb1` +0.008 %, are the Lua column again and the symbols say so: `luaH_getshortstr` +1 014 983 and
`luaH_set` +872 694 on `smb3`, +724 404 and +350 303 on `smb1`, against `write_internal` +123 223
on `smb3` and **nothing of this build above 100 000 on `smb1` at all**. `zelda2`'s `getgeneric`
**−691 040** in the same run is the other half of that column's spread.

### The decision

| feature | figure | decision |
| --- | --- | --- |
| `board-164` | PR-1 worst ROM **+0.009 %** (`m4`), set total +0.001 %; level-2 `core` +0 to +0.001 %, worst `total` +0.008 % — all against **+0.02 %** | **in `default`**, and no second feature: the two mechanisms the census calls an L cost nothing measurable of their own, the whole figure being one inlined body on the CPU **write** path |

`docs/review/BOARD-FEATURES.md` has the row, and its "worst figure among the features in
`default`" paragraph is updated: **+0.009 %**, this lot's, where it was +0.003 % — and the first
figure in that table that is neither a control's nor the instrument's.

[PROGRESS] MF-UNL-10 step 4/6 done: the campaign

## Step 5 — golden-core, the full run with both ROM roots, and the test this lot had not run

### The invariant test that caught a missing row

`cargo test --release --no-fail-fast -p zax-nes -p zax-harness -- --include-ignored` failed on
`cartridges::tests::mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`, and the lot's
own steps had not run it: steps 1 to 3 ran `--test mapper_tests` and the four sweeps, and this one
lives in the **library**'s `#[cfg(test)] mod tests`, which `--test mapper_tests` does not build.

The test is the invariant *plus* an expected list — "the board is driven directly … and the flag is
required to be `true` wherever that moves the board's state", then `assert_eq!(moved, expected)` so
that an accidental **addition** is caught as surely as an omission. Mapper 164 moves on the PPU
address bus and was not in that list. The row is now there, twice, the board refusing no submapper
where mapper 268 is in it once (Q-157), and the five lines pushed the function to 102 of
`clippy::too_many_lines`' 100 — so it carries the `#[allow]` that
`registry::board_memory_sizes` and `req_core_100_every_required_mapper_is_registered` carry, for
the reason they do: the body is a flat table.

The change is inside `#[cfg(test)]`, so no shipped byte moves; step 6 measures that rather than
asserting it.

Re-run in all four configurations after the fix, `--lib` **and** `--test mapper_tests`:

| configuration | clippy `-D warnings` | `--lib` | `--test mapper_tests` |
| --- | --- | ---: | ---: |
| default | clean | 64 passed, 2 ignored | 350 passed |
| `--no-default-features` | clean | 64 passed, 2 ignored | 103 passed |
| `--features parked-boards` | clean | 64 passed, 2 ignored | 350 passed |
| `--no-default-features --features board-164` | clean | 64 passed, 2 ignored | 112 passed |

### golden-core over the 98 commercial originals, 1 200 frames each

    zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200

| | |
| --- | --- |
| `909c0edd` (`mfunl10-0-base`) against the lot (`mfunl10-1-lot`) | **byte-identical** (`cmp`, not just the digest) |
| the digest | `md5 f79e7df842c3ee43e7d9b67861a927b5` |

**That is the digest MF-UNL-2 through MF-UNL-9 all recorded.** Nine lots, and not one pixel of a
commercial original has moved — which is what the lot brief asks of this one: "golden-core over the
98 originals must be byte-identical to the base commit's."

It is the measurement that matters most for the one figure this lot does carry. PR-1 has
`NesBus::write_internal` +445 122 on `m4`, a function every board compiles and every CPU write
runs; 98 games at 1 200 frames each, byte-identical, is what separates "the compiler laid that
function out differently" from "a board's behaviour moved".

### The full run with both ROM roots

    ZAXNES_ROMS=G:/3dsen/demo/roms
    ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms
    cargo test --release --no-fail-fast -p zax-nes -p zax-harness -- --include-ignored

**1 127 passed, 0 failed, 0 ignored.** The same command without the two variables and without
`--include-ignored` is **1 118 passed, 0 failed, 9 ignored** — so the nine ROM-driven tests really
ran rather than passing vacuously, which is the check the brief asks for in those words.

[PROGRESS] MF-UNL-10 step 5/6 done: golden-core and the full run

## Step 6 — the final pass, the census note and the journal

### The measured binary is the one the commit ships, shown and not assumed

| | `mfunl10-1-lot` (measured, `69672291`) | `mfunl10-3-ship` (`229334a2`) |
| --- | --- | --- |
| whole binary | `d4d92c446abb1cafd5194ccbf35d3b1e` | **the same** |
| `.text` | `2abfe3367f9200df0b16ffdedd49d8c9` | **the same** |
| `.rodata` | `866c8cb7221651ba0a952a01f8c0a35b` | **the same** |
| golden-core over the 98 originals | `f79e7df842c3ee43e7d9b67861a927b5` | **the same**, and `cmp`-identical to the base's |

The **whole** md5 matches, not only the two sections — MF-UNL-9 had to fall back on sections
because its last step added doc comments to `.rs` files and `build-clean.sh` keeps symbols, so the
debug info moved with the line numbers. Here step 5's change is entirely inside
`#[cfg(test)] mod tests`, which a release build does not compile at all, and the binary says so.
Three golden-core runs — base, measured, shipped — one digest.

### The lot against the brief

| the brief asks | what was done |
| --- | --- |
| `board-164` with its **full** register table | the five ports of the page under its own `$FF00` mask, both PRG modes, both semi-fixed-bank shapes, the `$5100` outer bits, mirroring behind the mode bit |
| the **93C66 512-byte serial EEPROM over Microwire**, a third EEPROM shape | `Eeprom93C66`: all seven instructions of DS20001795F table 1-4 in the **x8** organisation, the start bit, nine address bits, the dummy zero, the sequential read, the 93C commit edge, the EWEN/EWDS latch |
| **share the existing persistence path** | the bytes are in `prg_ram` behind `registry::battery_offset`, `Eeprom24C0X`'s own layout; `Cartridge::battery_ram` is what a host reads and nothing of it changed |
| **do not change the existing EEPROM boards' behaviour or save format** | `eeprom24c0x.rs` and `bandai_fcg.rs` are **untouched**; `board-eeprom`'s tests pass unchanged in all four configurations |
| its `registry.rs` arm, `SUPPORTED_MAPPERS` entry, `board_memory_sizes` row | all three, plus a `battery_offset` arm (that function becomes a `match`, mapper 1's SOROM case unchanged) and both PPU-address-bus lists |
| rows in `BOARD-FEATURES.md` | one row, and the "worst figure in `default`" paragraph updated to this lot's +0.009 % |
| the tests the census lists from each page | nine tests, the census's four and five more from the register table (step 2's table) |
| a Q ticket per §7.2 claim the page contradicts | **Q-170**, three of them, plus the four readings the page leaves open |
| the emulation of an existing mapper | **unchanged, and measured**: golden-core byte-identical over 98 originals; level-2 `ppu`/`apu`/`draw_list` identical to the instruction on five games |
| the save-state format of existing boards | unchanged: the board's block is appended inside its own `Board` arm, `NesCore::STATE_VERSION` does not move, and no other board writes a byte more |
| +0.02 % per ROM on **each ROM's total** | PR-1 worst `m4` **+0.009 %**; level-2 worst `total` `smb1` **+0.008 %**. Nothing over the tolerance, so the rule's same-behaviour-pair clause has no subject — the floor was measured anyway |
| a board over tolerance is parked | **no subject**: `board-164` is inside and stays in `default` |

### Summary

| | |
| --- | --- |
| numbers | **1** (164) |
| Cargo features added | **1**, `board-164` (implies `board-decode-boards`), **in `default`** |
| new modules | `mappers/eeprom93c66.rs` (`Eeprom93C66`), `mappers/pec9588.rs` (`Pec9588`) |
| `Board` discriminants added | **0** — one `DecodeBoard` variant |
| forwards whose `cfg` grew | `prg_ram_window` (2nd board), `try_read_prg` (5th), `on_ppu_address_bus` (4th) |
| savestate format | **unchanged**; the board's own block only |
| save file | the EEPROM's **512 bytes**, through `Cartridge::battery_ram` |
| tests added | **9**, plus one row in the `mf_3b` invariant list and one test-helper `cfg` |
| tickets | **Q-170** |
| PR-1 | worst ROM `m4` **+0.009 %**, set total +0.001 %, floor +0.000 % |
| level 2 | `core` +0 to +0.001 %, every `total` inside, `ppu`/`apu`/`draw_list` +0 on all five |
| golden-core | **byte-identical**, `f79e7df842c3ee43e7d9b67861a927b5` |
| suites | 1 127 / 0 / 0 with both ROM roots; four configurations green on `--lib` and `--test mapper_tests` |

### What is left, and to whom

* **R0 must source the six games.** §5 of the census is the condition: no file in
  `assets-free/test-roms` uses any of the 199 numbers, so the nine tests are written from the
  register table and the datasheet and nothing here has ever run a mapper-164 ROM.
* **Q-170 §2's four readings** stay provisional until one of those games is read back — the DAT
  line's undriven level, the volatile RAM's size, the unpopulated second chip and the write-path
  question.
* **§7.2 of `docs/review/UNL-census.md` should be corrected** on the three claims Q-170 §1 records.
  The note added to its MF-UNL-10 section carries them so the next reader is not misled; the plan
  is a review document and not a source of truth, which is why the page was followed.

[PROGRESS] MF-UNL-10 step 6/6 done: the lot is closed
