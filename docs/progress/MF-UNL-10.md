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
- [ ] **2/6** the tests the census lists from the page, plus §7.1's three common things in every
      configuration the lot adds.
- [ ] **3/6** `BOARD-FEATURES.md` row; fmt and clippy `-D warnings` in the four configurations
      (default, `--no-default-features`, `--features parked-boards`, `board-164` alone).
- [ ] **4/6** the callgrind campaign against `909c0edd`: PR-1's thirteen ROMs and the five
      level-2 games, **+0.02 % per ROM on each ROM's total**, and the feature decision.
- [ ] **5/6** golden-core byte-identical over the 98 originals, and the full
      `--include-ignored --no-fail-fast` suite with both ROM roots.
- [ ] **6/6** the census note, the journal, the close.

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
