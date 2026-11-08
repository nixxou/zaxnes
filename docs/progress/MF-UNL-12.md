# MF-UNL-12 — the last three singletons: mappers 262, 292 and 246

Lot **MF-UNL-12** of `docs/review/UNL-census.md` §7.2, the **twelfth and last** of the MF-UNL
lots: **three numbers, three boards, three Cargo features, 4 original titles.**

Base commit: **`c02fc02c`** (`MF-UNL-11 (8/8)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

The four titles §7.2 lists, with the board and publisher each number's own page gives it:

| number | board | publisher the page names | titles |
| ---: | --- | --- | --- |
| 262 | **UNL-SHERO** | **Sachen** | *Street Heroes*, *侍魂 (Shìhún)* — *Samurai Spirits* |
| 292 | **BMW8544** / UNL-DRAGONFIGHTER | "Flying Star" | *Dragon Fighter* (the unlicensed one) |
| 246 | **G0151-1** | not named; the page calls the game Taiwanese | *封神榜 — Fēngshénbǎng: Fúmó Sān Tàizǐ* |

"The last three singletons" is exactly right as a title: no two of the three share a chip, a
publisher or a mechanism. What they share is that each is **one number with one board and a
complete-enough page**, which is why §7.2 put them last — the lowest titles-per-hour in the plan.

Sources: nesdev, **"NES 2.0 Mapper 262"**, **"NES 2.0 Mapper 292"** and **"INES Mapper 246"**,
each page whole, read as **raw wikitext** (`?action=raw`) so that every register diagram's column
positions are the page's own and not a renderer's. That mattered: 262's one diagram is where two
of this lot's three readings come from, and a prose paraphrase of it got both wrong (§0 below).

## Step 0 — the inventory and the plan

### What exists, and what does not

| census item (§7.2 MF-UNL-12) | state at `c02fc02c` |
| --- | --- |
| step 1 — `board-262`, the CHR-RAM select at `$4100`, the high CHR bits, four-screen | **missing**; 262 is in no list |
| step 2 — `board-292`, the XOR protection and the two read-filled registers | **missing**; 292 is in no list |
| step 3 — `board-246`, the eight bank registers, the `$6800` battery RAM, the forced PRG A17 | **missing**; 246 is in no list |
| step 4 — §7.1's three common things | to do |

Nothing of the three numbers exists. What **does** exist and is reused unchanged:

* **[`Mmc3`] held rather than reimplemented** — 262 and 292 take the shape [`Mmc3Scramble`],
  [`Mmc3Unl8237`], [`Mmc3Gouder208`] and [`Mmc3A9461`] already have: the board owns an `Mmc3`,
  forwards every register it does not decode to it, and lays its own banking over the windows the
  chip computed. No `Mmc3Board` method body changes and **no `const` parameter is added to it**,
  which is the lot brief's "reuse the existing boards through parameters without moving their hot
  path or save-state format".
* **[`IrqBoard`]** — 262's and 292's IRQ is the MMC3's, so both answer **yes** to
  `Mapper::irq_asserted` and belong there, boxed, for [`Mmc3Scramble`]'s reason. 246 has **no
  IRQ at all** ("Nametable mirroring is hard-wired", no counter on the page), so it goes in
  [`DecodeBoard`], whose entry condition is exactly that.
* **`BankWindows::or_chr_banks_each`** — built for the Huang-1's per-window CHR A18 (MF-UNL-8
  steps 1 and 2), and 262's `$4100` bits 3-0 are the same shape to the bit: one A18 line per
  **2 KiB** region, over regions that are not in address order. Its `cfg` list widens; no new
  `BankWindows` method is added by this lot.
* **`Mmc3Board::chr_bank_of_window`**, **`force_ignore_a000`** — two accessors that exist for
  earlier boards; their `cfg` lists widen. `chr_bank_of_window` is the one 262 could not do
  without: its own note says the window table holds the bank wrapped modulo the CHR-**ROM**'s
  size and "that wrap need not preserve the chip-select line or the chip's own low bits", which
  is precisely 262's 8 KiB CHR-RAM chip.
* **`ppu::VRAM_SIZE` is 4 KiB** — so 262's "4 KiB of additional VRAM for four-screen mirroring"
  is the console VRAM this core already has, and the board only has to say
  `NametableMirroring::FourScreen`. That is §4.5's cheap level and mapper 77's precedent
  ([`IremLrog017`]), which is what §7.2 means by "the one §4.5 already has a path for".

So the lot adds **two `IrqBoard` variants, one `DecodeBoard` variant, no `Board` discriminant**,
and **one new hook** — see §0.4.

### 0.1 The three pages, transcribed

#### 262 — `$8000`-`$E001` as normal MMC3, and `$4100` under "probably `$C100`"

```text
Mask: probably $E001
   $8000, $8001, $A000, $A001, $C000, $C001, $E000, $E001: As normal MMC3.

Mask: unknown, but probably $C100
 $4100 (read): Read jumper setting            $4100 (write): High CHR bits and CHR-ROM/-RAM
    Bit 7654 3210                                Bit 7654 3210
        ---------                                    ---------
        J... ....                                    .M.. 8421
        +--------- Jumper value                       |   |||+- 256 KiB CHR bank at $1000-$17FF
                   0: Street Heroes                   |   ||+-- 256 KiB CHR bank at $1800-$1FFF
                   1: 侍魂 (Samurai Spirits)          |   |+--- 256 KiB CHR bank at $0800-$0FFF
                                                      |   +---- 256 KiB CHR bank at $0000-$07FF
                                                      +-------- Select CHR-ROM/-RAM
                                                                0: CHR-ROM   1: CHR-RAM
```

plus the opening sentence, which is the whole of the hardware: "an MMC3 clone with both CHR-ROM
and **8 KiB of CHR-RAM**, as well **4 KiB of additional VRAM for four-screen mirroring**".

**Two things in that diagram are not what a paraphrase of it says**, and both are pinned by a
test (Q-172 §1):

* `M` sits in the `7654` group at the position the header line gives bit **6**, not bit 7. Bit 7
  is `.`; the `J` of the *read* diagram is the one at bit 7.
* the four A18 lines are **not** in address order. Reading the `+` of each annotation line
  against the header gives bit 3 → `$0000-$07FF`, bit 2 → `$0800-$0FFF`, bit **1** →
  `$1800-$1FFF`, bit **0** → `$1000-$17FF`. The `8421` label is the four bits' *weights*, and the
  bottom two regions are crossed over against them.

#### 292 — the index register written, the two data registers **read**

```text
Extra Index Register ($6000-$7FFF, write)      Extra Data Register #0 ($6000-$7FFF, read)
 D~[ABR. ....]                                  D~[XXXX XXXX]
    ||+-------- Select Extra Data Register         ++++-++++- XOR value for the 2 KiB CHR bank
    ||          #0 or #1 on next read                        at PPU $0000-$07FF
    ++--------- Always 1
                                               Extra Data Register #1 ($6000-$7FFF, read)
Latch Register (write)                          D~[.BAA AAAA]
 "All CPU writes, including writes to zero          |++-++++- 4 KiB CHR bank at $1000-$1FFF
  page memory, are visible on the cartridge         |         (CHR A17..A12)
  connector. The mapper keeps the data of           +-------- Upper bit of the 2 KiB bank at
  the last CPU write to any address in a                      $0800-$0FFF (CHR A18)
  temporary latch register … The game writes
  the desired values to zero page."
```

"There are four extra registers that are filled by writing or reading (!) from the CPU
`$6000-$7FFF` address range **when PRG-RAM is enabled in the MMC3's A001 register**." The four
are the index, the two data registers and the latch. `R` is bit **5** of the index, not bit 0.

The effective CHR banks, the page's own three blocks and its own C:

```text
PPU $0000-$07FF  A10 = PPU A10;  A11-A17 = Extra0 bits 0-6 XOR MMC3 A11-A17;  A18 = Extra0 bit 7
                 chrBank2K = extraRegister[0] ^ (mmc3Register[0] >> 1);
                 "when Bit 7 in the MMC3's index register is set, MMC3 registers 2/3 … apply"
PPU $0800-$0FFF  A10 = PPU A10;  A11-A17 = MMC3 A11-A17;  A18 = Extra1 bit 6
                 chrBank2K = ((extraRegister[1] << 1) & 0x80) ^ (mmc3Register[1] >> 1);
                 "… MMC3 registers 4/5 instead of register 1 apply"
PPU $1000-$1FFF  A10-A11 = PPU A10-A11;  A12-A17 = Extra1 bits 0-5;  A18 = always 0
                 chrBank4K = extraRegister[1] & 0x3F;
```

"PRG banking, mirroring and scanline IRQ work as on a normal MMC3", and the *Errata*'s first
line: "the PRG-ROM bank at CPU `$8000-$9FFF` is **not** determined by any extra register".

#### 246 — eight registers under one mask, and the same mask again on a read

```text
PRG-ROM bank select ($6000-$6003), write       CHR-ROM bank select ($6004-$6007), write
 A~[0110 0000 000. .0rr] D~[..bb bbbb]          A~[0110 0000 000. .1rr] D~[bbbb bbbb]
                     ||       ++-++++-                              ||     ++++-++++-
                     ||       PRG A18..A13                          ||     CHR A18..A11
                     ++- CPU A14..A13 to switch                     ++- PPU A12..A11 to switch

* "When reading from CPU address $FFE4-$FFE7, $FFEC-$FFEF, $FFF4-$FFF7, or $FFFC-$FFFF,
   PRG A17 is forced high, as if register $6003 were OR'd with $10."
* "Register $6003 is set to $FF at power-on only."
```

"CPU `$6800-$6FFF`: 2 KiB PRG-RAM, **battery-banked**", and "Nametable mirroring is hard-wired
and set by the iNES header".

**The two masks are the same number.** `A~[0110 0000 000. .0rr]` is `address & $FFE4 == $6000`,
and the four read ranges are exactly `address & $FFE4 == $FFE4` — A15-A5 all set with A2 set,
A4, A3, A1 and A0 free. That is not a coincidence to be assumed: it is what the implementation
asserts, and a test walks all 32 addresses of `$FFE0-$FFFF` to show which eight are not forced.

### 0.2 What each board does, and where it sits

| | 262 | 292 | 246 |
| --- | --- | --- | --- |
| holds | an `Mmc3` | an `Mmc3` | a `BankWindows` |
| `Board` home | `IrqBoard`, boxed | `IrqBoard`, boxed | `DecodeBoard` |
| new forward on that enum | `try_read_prg` (cfg widened) | **`cpu_read_override` and `cpu_peek_override`** (new) | `intercept_cpu_write` (new) |
| `board_memory_sizes` | `chr_ram: Some(8192)` | `prg_ram: Some(0)` | `prg_ram: Some(2048)` |
| registers | one byte | three bytes | eight bytes |

### 0.3 The three tests the census lists, per page

* **262**: the `$4100` CHR-ROM/CHR-RAM select flips the source of a CHR read; the four-screen
  arrangement maps all four pages to distinct RAM; the high CHR bits reach banks above 256 KiB —
  and, added by this lot because the diagram demands it, the **crossed** region order of bits 1
  and 0 and the select bit being **6**.
* **292**: a **read** at `$6000-$7FFF` changes the CHR bank, which is what proves the read path
  is wired; the XOR values differ per pattern table; the effective-bank arithmetic against the
  page's own three C lines.
* **246**: the eight bank registers; a PRG-RAM write at `$6800-$6FFF` survives a round trip and a
  write the page's mask decodes does **not** reach PRG-RAM; the A17 forcing over all 32 addresses
  of `$FFE0-$FFFF`.

### 0.4 The one claim of §7.2 a page overrides, and the hook it costs

> "`Cartridge::cpu_read` already reaches that window, so **no new hook**." (§7.2, step 2)

Half of that is right and half is not, and the page is what settles it.

The **read** at `$6000-$7FFF` does need no new hook: `Mapper::cpu_read_override` is a `&mut self`
read hook that `Cartridge::try_cpu_read` already calls first thing, so the register fill has
somewhere to live. What §7.2 does not account for is **where the filled byte comes from**: the
page's *Latch Register* section says it is "the data of the last CPU write **to any address**",
and the game "writes the desired values to **zero page**". A zero-page write is RAM; in this core
it never reaches the cartridge at all, and `NesBus::data_bus` — the one byte that is close to
this — is set by reads as well as writes, so at the moment of the `$6000` read it holds the high
byte of that very address and not the game's datum.

So the latch is a **new hook**, and it is this lot's one cost outside a board: one byte on
`CartridgeMemory` behind `board-292`, stored by `NesBus::write_internal` for every CPU write.
Measured in step 6 like everything else; parked if it is over the tolerance. Recorded as **Q-172**
§2, which is the "the number's own page wins, record each such case in a Q ticket" rule the lot
brief restates from MF-UNL-2.

### 0.5 The steps

| step | what |
| ---: | --- |
| 0 | this plan |
| 1 | `board-262` |
| 2 | `board-292` |
| 3 | `board-246` |
| 4 | §7.1's three common things |
| 5 | fmt and clippy in six configurations |
| 6 | the callgrind campaign, PR-1's thirteen and level 2's five |
| 7 | golden-core over the 98 originals, and the two suites with both ROM roots |
| 8 | the final pass: `BOARD-FEATURES.md`, Q-172, the census note, and the journal entry with the MF-UNL-1 to 12 summary |

[`Mmc3`]: ../../rust/crates/zax-nes/src/cartridges/mappers/mmc3.rs
[`Mmc3Scramble`]: ../../rust/crates/zax-nes/src/cartridges/mappers/mmc3_scramble.rs
[`Mmc3Unl8237`]: ../../rust/crates/zax-nes/src/cartridges/mappers/mmc3_unl8237.rs
[`Mmc3Gouder208`]: ../../rust/crates/zax-nes/src/cartridges/mappers/mmc3_gouder208.rs
[`Mmc3A9461`]: ../../rust/crates/zax-nes/src/cartridges/mappers/mmc3_a9461.rs
[`IrqBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/irq_boards.rs
[`DecodeBoard`]: ../../rust/crates/zax-nes/src/cartridges/mappers/decode_boards.rs
[`IremLrog017`]: ../../rust/crates/zax-nes/src/cartridges/mappers/irem_lrog017.rs

## Step 1 — `board-262`, the Sachen UNL-SHERO

`rust/crates/zax-nes/src/cartridges/mappers/mmc3_shero.rs`, one boxed `IrqBoard` variant holding
an `Mmc3` and **one register byte**. No `Board` discriminant, no new `BankWindows` method, and no
`Mmc3Board` body changed: three `cfg` lists widened (`chr_bank_of_window`, `force_ignore_a000`,
`refresh_banks`) and one more on `BankWindows::or_chr_banks_each`.

### What the board is

* **`$8000-$FFFF`** is the chip's, whole. The page's "Mask: probably `$E001`" *is* the MMC3's own
  mask, so the guess decides nothing — every write is handed to the [`Mmc3`] and decoded there.
* **`$4100` write**, `address & $C100 == $4100` as the page guesses it: bit **6** picks which CHR
  chip the PPU reaches, bits 3-0 drive **CHR A18** over the four 2 KiB regions.
* **`$4100` read**: the title jumper in bit 7, through `IrqBoard::try_read_prg` — a forward that
  variant already had for five earlier boards, so `Board`'s dispatches grow nothing.
* **8 KiB of CHR-RAM** beside the CHR-ROM, a `board_memory_sizes` row for the TQROM's reason (no
  header field can state a CHR-RAM chip beside a CHR-ROM), banked by bits 0-2 of whatever 1 KiB
  bank number the chip put in the window.
* **four-screen** from the board's wiring, which is `NametableMirroring::FourScreen` plus
  `Mmc3Board::force_ignore_a000` — `ppu::VRAM_SIZE` being 4 KiB is the whole of the "4 KiB of
  additional VRAM".

### The four readings Q-172 records, and why the page was read as raw wikitext

| what | the page | implemented |
| --- | --- | --- |
| the CHR-ROM/-RAM select's bit | `.M.. 8421` under `Bit 7654 3210` | bit **6** — `M` is at the header's 6 and bit 7 is `.` |
| the four A18 lines' regions | four annotation lines, each `+` naming its bit | bit 3 → `$0000`, bit 2 → `$0800`, bit **1** → `$1800`, bit **0** → `$1000` |
| the CHR-RAM chip's address lines | **nothing at all** | the MMC3's own bank bits 0-2, the TQROM's reading |
| the jumper | a board wiring; no submapper defined | **0**, *Street Heroes*, with the gap recorded |
| `$A000` | "As normal MMC3", beside four-screen VRAM | inert, because a four-screen board drives CIRAM A10 **and** A11 |
| the two masks | "probably `$E001`", "unknown, but probably `$C100`" | as written, both |

The first two are column arithmetic, and a prose paraphrase of that diagram got **both** wrong —
it put the select at bit 7 and the four regions in address order. That is why every diagram of
this lot was taken from `?action=raw`.

### Tests (4 new, all from the page)

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_262_selects_the_chr_ram_chip_from_4100_bit_6` | the select from **both** sides — bit 7 set changes nothing, bit 6 set swaps the chip — plus the chip's three address lines: banks `$11` and `$19` are one kilobyte and `$12` another |
| `q_172_mapper_262_crosses_the_two_bottom_chr_a18_lines_against_their_regions` | one bit at a time over all eight windows, so the crossing is visible; then all four at once, which reaches every bank of a 512 KiB CHR-ROM; then twice, which is idempotent because a line is a line |
| `q_172_mapper_262_is_four_screen_from_its_wiring_and_its_a000_is_inert` | four distinct kilobytes of VRAM for all four header mirroring bits, through five values of `$A000` and a reset |
| `q_172_mapper_262_answers_its_title_jumper_under_the_pages_c100_mask` | the port answers at six addresses of `address & $C100 == $4100` |

### The check at this step

| command | result |
| --- | --- |
| `cargo fmt --check` | **clean** |
| `cargo clippy --release -p zax-nes --all-targets -- -D warnings` | **clean** |
| `cargo test --release -p zax-nes` | **all green**, `mapper_tests` 379 passed / 0 failed |

Two sweeps needed the number added rather than merely passing, which is the point of having
them: `req_core_100_every_required_mapper_is_registered`'s `additions` table (110 → 111 entries)
and `mf_5a_every_board_that_moves_on_the_ppu_bus_is_latched`'s expected vector — 262 **twice**,
the page giving the number no submapper and the board refusing neither half of the pair.
