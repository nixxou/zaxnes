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

- [x] **0/6** the plan — this note.
- [ ] **1/6** `Eeprom93C66`: the Microwire device behind `board-164`. The datasheet's seven
      instructions (READ, WRITE, ERASE, ERAL, WRAL, EWEN, EWDS), the start bit, nine address bits,
      the dummy zero before read data, sequential read, the 93C-variant commit edge, the
      EWEN/EWDS latch, and RDY/BSY on DO. Bytes in a caller-owned slice, `Eeprom24C0X`'s shape.
      Save-state block. Q-170.
- [ ] **2/6** `Pec9588`: the five registers, both PRG modes, the two fixed-bank shapes, the
      $5100 outer bits, mirroring, the 1 bpp CHR addressing and the Microwire port. A
      `DecodeBoard` variant; `registry.rs` arm, `SUPPORTED_MAPPERS` entry, `board_memory_sizes`
      row, `battery_offset` arm, both PPU-bus lists, the `board-164` feature.
- [ ] **3/6** the tests the census lists from the page, plus §7.1's three common things in every
      configuration the lot adds.
- [ ] **4/6** `BOARD-FEATURES.md` row; fmt and clippy `-D warnings` in the four configurations
      (default, `--no-default-features`, `--features parked-boards`, `board-164` alone).
- [ ] **5/6** the callgrind campaign against `909c0edd`: PR-1's thirteen ROMs and the five
      level-2 games, **+0.02 % per ROM on each ROM's total**, and the feature decision.
- [ ] **6/6** golden-core byte-identical over the 98 originals; the full
      `--include-ignored --no-fail-fast` suite with both ROM roots; the census note and the
      journal.

[PROGRESS] MF-UNL-10 step 0/6 done: the plan
