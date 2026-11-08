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

## Step 2 — `board-292`, the BMW8544 / UNL-DRAGONFIGHTER

`rust/crates/zax-nes/src/cartridges/mappers/mmc3_dragonfighter.rs`, one boxed `IrqBoard` variant
holding an `Mmc3` and **three register bytes** — plus **this lot's one hook**, which is step 0.4's
finding made real.

### What the board is

"PRG banking, mirroring and scanline IRQ work as on a normal MMC3", so all of that is the chip's
and unchanged. What the board adds is the CHR half, computed per 1 KiB window from the page's
three *Effective CHR banks* blocks:

| region | the page's own C | what drives it |
| --- | --- | --- |
| PPU `$0000-$07FF` | `extraRegister[0] ^ (mmc3Register[0] >> 1)` | the chip's 2 KiB bank **XORed** with Extra0, and CHR A18 from Extra0 bit 7 |
| PPU `$0800-$0FFF` | `((extraRegister[1] << 1) & 0x80) ^ (mmc3Register[1] >> 1)` | the chip's 2 KiB bank, with **only** CHR A18 from Extra1 bit 6 |
| PPU `$1000-$1FFF` | `extraRegister[1] & 0x3F` | Extra1 alone; the chip drives nothing and "CHR A18: Always 0" |

Two things fall out of reading those blocks as **address lines** rather than as three bank
numbers, and both save code:

* `CHR A10 = PPU A10` makes the chip's own bank bit 0 a don't-care in the two 2 KiB regions, so
  the pair is a true 2 KiB window whatever odd bank the chip put there.
* the note "when Bit 7 in the MMC3's index register is set, MMC3 registers **2/3** instead of
  register 0 apply" needs **no code at all**: bit 7 of `$8000` is the chip's CHR inversion, so
  what the chip drives at `$0000-$07FF` with it set *is* R2 and R3, and
  `Mmc3Board::chr_bank_of_window` is already "the bank the chip drives at that window".

### The hook, and the claim it overrides

> §7.2, step 2: "`Cartridge::cpu_read` already reaches that window, so **no new hook**."

The **read** needs none: `Mapper::cpu_read_override` is a `&mut self` hook
`Cartridge::try_cpu_read` already calls first thing, forwarded here for the first time on
`IrqBoard` and `#[inline(never)]` for `try_read_prg`'s reason. What §7.2 did not account for is
**where the filled byte comes from** — "the data of the last CPU write **to any address**", with
the game writing "the desired values to **zero page**". A zero-page write is RAM and reaches no
cartridge in this core, and `NesBus::data_bus` is not it either: that byte is set by reads as well
as writes, so at the instant of the `$6000` read it holds the high byte of that very address.

So the lot's one addition outside a board:

* `CartridgeMemory::cpu_write_latch`, one `u8` behind `#[cfg(feature = "board-292")]`;
* one `if let` plus one store at the **top** of `NesBus::write_internal`, behind the same `cfg`.

It is in `write_internal` and not in `Cartridge::cpu_write` because the latch has to see the RAM
half of the address space, and it is a store into `CartridgeMemory` and not a `Board` dispatch
because that function runs on every CPU write of every game. **Not in the savestate**: it is a
transient bus byte the next CPU write overwrites, nothing is derived from it until a read of
`$6000-$7FFF` asks, and the two data registers it feeds *are* in the board's own block — so no
board's save block grows by a byte in any configuration. Recorded as Q-172 §2.

### Tests (5 new, all from the page)

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_292_fills_a_data_register_from_a_read_of_6000_7fff` | §7.2's own test: the **read** moves the bank, and neither the `$6000` write nor the zero-page write does — even though the `$6000` write is itself what the latch then holds. Plus `$A001` clear, where nothing answers |
| `req_core_102_mapper_292_computes_each_regions_bank_the_way_its_own_c_line_does` | the three C lines; that the XOR is an XOR (`$FF` gives the complement, `$00` the chip's own bank); that Extra1 moves **two** regions at once; and that no value of Extra1 takes `$1000-$1FFF` past the first 256 KiB |
| `q_172_mapper_292_takes_registers_2_and_3_when_the_chips_chr_inversion_is_set` | the plural in "registers 2/3": with the inversion set R2 drives the first kilobyte and R3 the second, R4/R5 the `$0800` pair, and the XOR still applies through it |
| `q_172_mapper_292_latches_the_last_cpu_write_to_any_address_zero_page_included` | the hook itself, from four addresses no cartridge is mapped at; that A and B drive nothing (the page's unverified hypothesis is not implemented); and that `R` is bit **5** and not bit 0 |
| `req_core_095_mapper_292_has_no_prg_ram_behind_its_register_window` | a battery dump's `$6000-$7FFF` is the register and not RAM: a write does not persist and a read drives nothing |

### The check at this step

| command | result |
| --- | --- |
| `cargo fmt --check` | **clean** |
| `cargo clippy --release -p zax-nes --all-targets -- -D warnings` | **clean** |
| `cargo test --release -p zax-nes` | **all green**, 47 suites, 0 failed |

## Step 3 — `board-246`, the G0151-1

`rust/crates/zax-nes/src/cartridges/mappers/g0151.rs`, one [`DecodeBoard`] variant — the board has
**no counter**, which is that enum's entry condition — holding a `BankWindows` and eight register
bytes. No `Board` discriminant.

### What the board is

| | the page |
| --- | --- |
| PRG | four 8 KiB banks at `$6000-$6003`, `D~[..bb bbbb]`, "PRG A18..A13" |
| CHR | four 2 KiB banks at `$6004-$6007`, `D~[bbbb bbbb]`, "CHR A18..A11" |
| mask | `A~[0110 0000 000. .?rr]` — `address & $FFE4 == $6000` / `$6004`, so the register window is `$6000-$601F` |
| PRG-RAM | "CPU `$6800-$6FFF`: 2 KiB PRG-RAM, **battery-banked**" |
| mirroring | "hard-wired and set by the iNES header" — no register, and no IRQ at all |
| power-on | "Register `$6003` is set to `$FF` at power-on **only**" |
| the read | "When reading from `$FFE4-$FFE7`, `$FFEC-$FFEF`, `$FFF4-$FFF7`, or `$FFFC-$FFFF`, PRG A17 is forced high, as if register `$6003` were OR'd with `$10`" |

**The register mask and the read condition are the same number.** `A~[0110 0000 000. .0rr]` is
`address & $FFE4 == $6000`; the four read ranges are `address & $FFE4 == $FFE4` — A15-A5 all set
with A2 set, A4, A3, A1 and A0 free. One constant, `G0151::DECODE_MASK`, serves both, and the test
walks all 32 addresses of `$FFE0-$FFFF` rather than asserting the four ranges by hand.

### The two new forwards on `DecodeBoard`, and why each is unavoidable

* **the PRG read cache.** The forced A17 depends on the address *within* the `$E000-$FFFF` window,
  and `Cartridge::read_rom_window` answers a CPU read of `$8000-$FFFF` from a four-entry table
  before the board is consulted at all. `Mapper::rom_windows`'s own documentation states the rule
  — a board whose reads must go through the board "must return `NO_ROM_WINDOW` for them" — and
  `read_rom_window`'s `debug_assert_eq!` against `Mapper::try_read_prg` is what catches a board
  that does not. So this board answers `NO_ROM_WINDOW` for the **fourth window alone**; the other
  three keep the fast path, which is three quarters of the game's code.

  This is **the one body of the lot that lands inside `Board`'s CPU-read-window dispatch**, where
  MF-UNL-10 step 2 measured `m4` **+0.009 %** — this table's worst figure in `default` — for one
  such body. Step 6 measures it.
* **`intercept_cpu_write`.** The eight registers and the RAM chip share the `$6000-$7FFF` window
  that `Cartridge::cpu_write` routes to PRG-RAM, so without the hook no register could be written.
  `G0151::intercept_cpu_write` is `#[inline(never)]`, so the arm it adds to that dispatch is a
  `callq`.

`try_read_prg` is a third forward `DecodeBoard` already had — for four protection reads below
`$8000` — whose `cfg` list this number joins, and it is where the forcing is computed.

### The three things Q-172 records for this number

1. **the register window is `$6000-$601F`, not `$6000-$67FF`.** §7.2's test bullet says the latter,
   which is Mesen's decode; the page's mask requires A11-A5 all clear. The test is written from the
   mask: all 32 addresses it names are registers, and `$6400` — which it does not name — is the RAM
   chip.
2. **the 2 KiB chip appears four times across `$6000-$7FFF`.** The page gives it one window,
   `$6800-$6FFF`, which is A11 of the board's own decode; `Mapper::prg_ram_window` is a base and a
   wrap asked once per CPU write and cannot say "and nowhere else". A short chip mirrored inside
   the window is mapper 164's row's reading of the same shape, and the register decode wins over
   it.
3. **"`$FF` at power-on **only**" means a reset leaves `$6003` alone.** The word "only" is the
   whole of it: the board has no reset line to that register, so the bank the vectors are fetched
   through is whatever the game last wrote. The other seven go back to zero.

And the page's *See also* — the forum thread *"Why does Fong Shen Bang only work in
Famiclones?"* — is recorded in the board's own note as **not a statement about the board**:
whether the game runs is console timing, this build targets NTSC/PAL/Dendy, and the board has no
timing of its own to condition on.

### Tests (4 new, all from the page)

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_246_banks_four_prg_windows_and_four_chr_pairs` | the eight registers, each field's own width (six bits of PRG against eight of CHR), and that no register moves the hard-wired mirroring |
| `req_core_102_mapper_246_forces_prg_a17_on_the_eight_addresses_of_its_own_mask` | all 32 addresses of `$FFE0-$FFFF`, each checked against `address & $FFE4 == $FFE4`; that the forcing is an **OR of one bit** (a register that already has it reads the same bank everywhere); and that it is a different *bank* and not merely a different number |
| `req_core_095_mapper_246_keeps_its_battery_ram_at_6800_and_its_registers_at_6000` | a 2 KiB battery slice; a round trip at `$6800`, `$6C00` and `$6FFF`; all 32 register addresses moving a bank and touching no RAM; and `$6400`/`$6C00` being one byte of the mirrored chip |
| `amd_061_mapper_246_keeps_its_last_prg_bank_across_a_reset` | "power-on **only**", twice over, with the four CHR registers going back to zero beside it |

### The check at this step

| command | result |
| --- | --- |
| `cargo fmt --check` | **clean** |
| `cargo clippy --release -p zax-nes --all-targets -- -D warnings` | **clean** |
| `cargo test --release -p zax-nes` | **all green** |
| `cargo test -p zax-nes --test mapper_tests` (**dev** profile, `debug_assert` live) | **388 passed / 0 failed** |

The dev-profile run is in the table on purpose: `Cartridge::read_rom_window`'s
`debug_assert_eq!(Some(value), self.board.try_read_prg(…))` is the assertion that a board which
changes its bank per address inside a window must not be in the cache, and it is compiled out of
the release profile the other three rows use.

**And one failure worth recording, because it is a trap for the next lot.**
`req_arch_002_the_crate_has_no_platform_dependency` greps `zax-nes/Cargo.toml` for the literal
`windows` among its forbidden crate names — so a feature comment that writes `rom_windows` or
"the other three windows" fails a test about platform dependencies. `BankWindows` is safe
(capital W). This step's first draft tripped it; the comment is phrased around it.

## Step 4 — §7.1's three common things

### 1. The four sweeps over `SUPPORTED_MAPPERS`

All four already exist and all four now reach the three numbers. Two of them needed the number
**added** rather than merely passing, which is the point of having them:

| sweep | what it took |
| --- | --- |
| `req_core_100_every_required_mapper_is_registered` | three rows in its `additions` table, 110 → **113** entries, each gated on its own feature; the table is order-sensitive, so 246 goes between 243 and 262 and 292 between 268 and 298 |
| `req_core_102_any_other_mapper_is_refused` | unchanged — it sweeps 6, 116, 177 and 200, none of which this lot touches (§7.4) |
| the registry round-trip (`Cartridge::SUPPORTED_MAPPERS` ascending, every number creating a board) | passes; the ascending assertion is what caught nothing because the entries were placed in order |
| `mf_5a_every_board_that_moves_on_the_ppu_bus_is_latched` | **262 and 292 twice each** in its expected vector, plus both numbers in `watches_ppu_reads` and `watches_ppu_bus_without_read`. Without those two lists the boards' A12 counters would silently never be clocked — the correctness bug that test exists for |
| `board_memory_sizes` | three new rows: 262 `chr_ram: Some(8192)`, 292 `prg_ram: Some(0)`, 246 `prg_ram: Some(2048)` |

246 is in **neither** PPU-bus list and that is correct: its page gives it no counter and no latch
at all, so nothing of it moves on the PPU's address bus.

### 2. A save-state round trip per board, with the board's own state dirtied first

`req_core_090_the_mapper_state_survives_a_save_and_restore` dirties `$5000`, `$6000` and
`$8000-$E001`. Against this lot that reaches **one** byte of three boards: 292's index register is
at `$6000`. Everything else is this step's business.

| test | what the generic sweep could not reach |
| --- | --- |
| `req_core_090_mapper_262_round_trips_its_4100_register` | the one byte at `$4100` — **one byte with two jobs**, the CHR chip select and four CHR A18 lines, so a block that dropped it would restore the window table and still read the wrong chip. The layout helper carries the CHR **bytes** for exactly that |
| `req_core_090_mapper_292_round_trips_the_two_registers_only_a_read_can_fill` | the two data registers, which **no sweep of this suite can write**: only a read of `$6000-$7FFF` fills them. Taken with the index left selecting #1, and the last four lines check that the restored index *is* the index — one more fill with no `$6000` write of its own must land in #1 |
| `req_core_090_mapper_246_round_trips_its_eight_bank_registers` | seven of the eight registers, the battery chip behind the same window, and — the one thing a correct window table could still get wrong — that the **forced-A17 read follows the restored `$6003`** and not the table |

### 3. The reset

`amd_061_a_reset_puts_both_mmc3_boards_of_mf_unl_12_back_to_their_power_on_state`, on
AMD-061's own sweep's pattern: dirty the board's own registers, require the power-on layout and no
pending IRQ.

**Mapper 246 is deliberately not in it**, and it is this lot's one exception to §7.1's shape: its
page says "Register `$6003` is set to `$FF` at power-on **only**", so the board *cannot* come back
to its power-on layout and must not be asked to.
`amd_061_mapper_246_keeps_its_last_prg_bank_across_a_reset` is that number's own test and Q-172 §3
the reading. AMD-061's global sweep passes over 246 unchanged because the addresses it dirties —
`$5000`, `$6000`, `$8000-$E001` — reach the board's register 0 and not its register 3.

### The check at this step

| command | result |
| --- | --- |
| `cargo fmt --check` | **clean** |
| `cargo clippy --release -p zax-nes --all-targets -- -D warnings` | **clean** |
| `cargo test --release -p zax-nes` | **all green**, `mapper_tests` **392 passed / 0 failed** |
| `cargo test --release -p zax-harness` | **all green** |

**392** is the figure to compare the configurations of step 5 against: 379 at the lot's base
commit plus this lot's thirteen.

## Step 5 — the six configurations

`cargo fmt --check` is **clean** over the whole workspace, and clippy with `-D warnings` on
`zax-nes` — the one crate this lot touches — is clean in all six:

| configuration | clippy | `mapper_tests` |
| --- | --- | ---: |
| `default` | **clean** | **392** passed / 0 failed |
| `--no-default-features` | **clean** | 103 |
| `--features parked-boards` | **clean** | **392** |
| `--no-default-features --features board-262` | **clean** | 108 |
| `--no-default-features --features board-292` | **clean** | 109 |
| `--no-default-features --features board-246` | **clean** | 108 |

`--all-targets` in every row, so the tests are linted too. The full `cargo test --release -p
zax-nes` is green in `default`, `--no-default-features` and `--features parked-boards`.

**392 against a base of 375**: this lot adds **seventeen** tests, and the three single-feature
rows account for sixteen of them — 5 for 262, 6 for 292, 5 for 246. The seventeenth is
`amd_061_a_reset_puts_both_mmc3_boards_of_mf_unl_12_back_to_their_power_on_state`, which is
`#[cfg(all(feature = "board-262", feature = "board-292"))]` and so runs in `default` and in
`parked-boards` and in neither single-feature build. That arithmetic is the check: a test gated on
a feature it does not need would show up here as a row that is too high, and one gated on a
feature it does need but did not name would show up as a build failure.

### What the single-feature builds caught, and it is the same class of thing every lot's do

**`board-246` alone did not compile.** `DecodeBoard`'s file opens with a `compile_error!` behind a
`cfg(not(any(…)))` over every board of that enum — the guard that turns
`--features board-decode-boards` on its own into one clear error instead of nine confusing ones —
and its `any` list is the one place a new member of the enum has to be named that is *not* implied
by anything else. The feature was in `default`, so every other configuration had another board of
the enum on and the list's omission was invisible.

That is MF-UNL-11 step 5's finding in a different file (there it was `IrqBoard::try_read_prg`'s
`cfg` list, caught by the `board-208`-alone build), and the rule it gives is the same: **a `cfg`
list is never implied by the variant it belongs to.**

## Step 6 — the campaign, and the one board of twelve lots that is parked

Binaries built by `build-clean.sh` from their own tar into a **fresh** directory with its own
`CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`.

| label | tree | md5 |
| --- | --- | --- |
| `mfunl12-0-base` | **`c02fc02c`**, the lot's base commit | `d031eddea93ab222efc8cbbf61dd5b67` |
| `mfunl12-1-lot` | **`b6467bd1`**, step 5 — all three features in `default` | `4ef3ef4797e89da591b641bbd0eaa475` |
| `mfunl12-2-off` | a probe: all three features in `parked-boards` | — |
| `mfunl12-3-no292`, `mfunl12-4-ship` | **the shipped configuration**: 262 and 246 in `default`, 292 parked | `aee4243f1e16e7aaf876751751f68262` (two independent builds, same md5) |
| `mfunl12-0-again` | a byte copy of `mfunl12-0-base`, the **same-binary floor** | same md5 as the base |

The base binary's md5 is **identical to MF-UNL-11's shipped binary** (`zax-harness-mfunl11-2-ship`),
so this lot's base is that lot's measured binary to the byte — the **ninth** lot in a row for which
that holds, and built rather than assumed.

### 6.1 The first measurement, all three features on: a real regression

| ROM | base | lot | delta | % |
| --- | ---: | ---: | ---: | ---: |
| duelito | 7 380 403 016 | 7 380 566 874 | +163 858 | +0.002 |
| m0 | 5 517 372 614 | 5 517 919 495 | +546 881 | +0.010 |
| m1 | 4 881 091 252 | 4 881 932 529 | +841 277 | +0.017 |
| m4 | 5 468 140 245 | 5 468 675 621 | +535 376 | +0.010 |
| metroid | 5 115 551 905 | 5 116 329 341 | +777 436 | +0.015 |
| mm2 | 6 017 978 118 | 6 018 322 515 | +344 397 | +0.006 |
| mmc5 | 11 266 796 431 | 11 390 607 031 | **+123 810 600** | **+1.099** |
| oamstr | 3 318 523 121 | 3 319 324 968 | +801 847 | **+0.024** |
| readbuf | 6 834 520 361 | 6 835 187 099 | +666 738 | +0.010 |
| smb1 | 5 996 351 415 | 5 997 070 808 | +719 393 | +0.012 |
| smb3 | 7 949 398 009 | 7 949 966 031 | +568 022 | +0.007 |
| vblnmi | 3 308 266 455 | 3 308 942 824 | +676 369 | **+0.020** |
| z2 | 6 291 535 122 | 6 291 794 132 | +259 010 | +0.004 |
| **total** | **79 345 928 064** | **79 476 639 268** | **+130 711 204** | **+0.165** |

Thirteen ROMs slower, three at or above the tolerance, and `mmc5` at **fifty-five times** it. Two
symbols carry the whole of it: `Cartridge::try_cpu_read` **+122 851 006** on `mmc5exram`, and
`NesBus::write_internal` **+157 036 to +862 997 on every ROM of the set**.

### 6.2 The attribution: it is `board-292`, entirely

The same tree with `board-292` alone moved out of `default` (`mfunl12-3-no292`) against the tree
with it in (`mfunl12-1-lot`) — so the difference *is* that feature:

| | worst ROM | set total |
| --- | --- | ---: |
| `board-292` alone | `mmc5` **+1.098 %**, `oamstr` +0.026 %, `vblnmi` +0.020 %, `m1` +0.017 % | **+130 067 231** |
| `board-262` + `board-246` together | `m4` **+0.003 %** | +643 973 (+0.001 %) |

**Two causes, and neither has a fix.**

1. **`Board::cpu_read_override` gains a body.** `IrqBoard` had no forward for it, so it shared that
   dispatch's **empty** arm — and that dispatch carries `#[inline(always)]`, pinned by lot PR-1
   step 3, and folds into `Cartridge::try_cpu_read`, which the MMC5 reaches on **every CPU read**
   because it is the board that maps PRG-RAM into `$8000-$DFFF`. That is the +122.8 M, and it is
   MF-5b step 1's "the arm that matters is the empty one" one dispatch further along. A board whose
   registers a **read** fills needs a `&mut self` read hook, and this build has exactly one.
2. **the latch store in `NesBus::write_internal`**, worst `oamstr` **+0.026 %** — a ROM whose whole
   run is OAM writes. **Splitting the board into two features would not save it**: `oamstr`'s figure
   is +862 997 of +874 540, so the latch *on its own* is already over the tolerance.

### 6.3 The decision

* **`board-292` is parked** — moved out of `default` into `parked-boards`, never dropped. Every
  board stays reachable behind its feature, which is the owner's rule; one flag,
  `--features parked-boards`, turns it on.
* **`board-262` and `board-246` stay in `default`**: +0.003 % on their worst ROM.

It is the **first and only board of the twelve MF-UNL lots to be parked**, and the second
cartridge board of this build after `board-vs-system`.

### 6.4 The shipped configuration on PR-1's thirteen, and the two floors

| ROM | base | ship | delta | % |
| --- | ---: | ---: | ---: | ---: |
| duelito | 7 380 403 016 | 7 380 523 008 | +119 992 | +0.002 |
| m0 | 5 517 372 614 | 5 517 362 549 | −10 065 | −0.000 |
| m1 | 4 881 091 252 | 4 881 121 542 | +30 290 | +0.001 |
| m4 | 5 468 140 245 | 5 468 326 704 | +186 459 | **+0.003** |
| metroid | 5 115 551 905 | 5 115 660 128 | +108 223 | +0.002 |
| mm2 | 6 017 978 118 | 6 018 062 064 | +83 946 | +0.001 |
| mmc5 | 11 266 796 431 | 11 266 883 281 | +86 850 | +0.001 |
| oamstr | 3 318 523 121 | 3 318 447 543 | −75 578 | −0.002 |
| readbuf | 6 834 520 361 | 6 834 584 961 | +64 600 | +0.001 |
| smb1 | 5 996 351 415 | 5 996 413 457 | +62 042 | +0.001 |
| smb3 | 7 949 398 009 | 7 949 480 930 | +82 921 | +0.001 |
| vblnmi | 3 308 266 455 | 3 308 266 977 | +522 | +0.000 |
| z2 | 6 291 535 122 | 6 291 589 932 | +54 810 | +0.001 |
| **total** | **79 345 928 064** | **79 346 723 076** | **+795 012** | **+0.001** |

Worst ROM **+0.003 %** against a tolerance of +0.02 %, and `mmc5` — the ROM the parked feature cost
1.1 % — is back at +0.001 %.

| floor | worst ROM | set total |
| --- | --- | ---: |
| the **ship** binary run twice | `duelito` +0.001 % | +151 039 |
| the **base** binary run twice | `oamstr` −0.002 %, `duelito` +0.001 % | +19 525 |

### 6.5 Level 2, and a valgrind limitation this lot has to work around

**This machine's valgrind 3.22 silently declines to read the shipped binary's symbols.** Verbose
output shows no `Reading syms from …/zax-harness-mfunl12-4-ship` line at all, no warning, and the
annotations come out as raw addresses — so `callgrind-l2.sh`'s stage table, which is built by
matching function names in `callgrind_annotate --inclusive=yes`, is **empty** for it. Two
independent builds of that tree give the same md5 and the same failure, and `readelf -lW` shows
what differs from every binary that does work:

    base:  GNU_RELRO  FileSiz 0x0157e0  MemSiz 0x015980   .relro_padding in the .data.rel.ro LOAD
    ship:  GNU_RELRO  FileSiz 0x0157e0  MemSiz 0x0157e0   .relro_padding in the .data/.bss LOAD

A linker-layout difference, deterministic for that feature set and outside this lot's control. It
is recorded here because the next lot may meet it: **the whole-process `Ir` is still exact**, and it
is what the shipped configuration's level-2 row is measured on.

| game | base | ship | delta | % |
| --- | ---: | ---: | ---: | ---: |
| metroid | 13 735 989 397 | 13 736 146 552 | +157 155 | +0.001 |
| mm2 | 18 977 275 426 | 18 977 571 409 | +295 983 | +0.002 |
| smb1 | 13 586 825 218 | 13 592 319 380 | +5 494 162 | **+0.040** |
| smb3 | 24 193 127 575 | 24 191 998 195 | −1 129 380 | −0.005 |
| zelda2 | 15 359 321 035 | 15 360 214 505 | +893 470 | +0.006 |
| **total** | **85 852 538 651** | **85 858 250 041** | **+5 711 390** | **+0.007** |

Four of five inside the tolerance. **`smb1` is over it, and the lot's performance rule says to show
the cause with a same-behaviour pair rather than redefine the tolerance.** Three measurements do
that:

| | smb1 | what it is |
| --- | ---: | --- |
| the **same binary** run twice (`3-no292` vs `4-ship`) | **+0.018 %** | the instrument's own spread on this game |
| the base binary run twice | +0.003 % | the same, on the other binary |
| **the same-behaviour pair**: base vs `mfunl12-2-off` | **+0.015 %** | the lot's own tree with all three features out of `default`, so the emulation of all five games is the base commit's byte for byte |

and the pair's per-symbol diff names the cause outright: **`luaH_getshortstr` +1 903 930 of a
+2 083 363 whole-game move, 91 % of it**, with `__memcpy_avx_unaligned_erms` +336 778 and
`_int_malloc` −153 632 beside it and **nothing of this project moving at all**. That is Lua's
string-key hash table, the column every lot since MF-UNL-1 has recorded a ±0.02 % spread on.

So `smb1`'s +0.040 % is about twice a spread of 0.018 % on a figure whose 91 % mover is `ltable.c`,
and it is recorded rather than explained away.

### 6.6 The stage table, where it can be read

`mfunl12-2-off` is the identical-emulation tree and `mfunl12-1-lot` the all-features-on one; both
symbolize.

| | `core` | worst `total` |
| --- | --- | --- |
| base vs **`2-off`** (identical emulation) | **+0 on all five games, identical to the instruction** | `smb1` +0.015 %, `smb3` −0.023 % |
| base vs **`1-lot`** (all three features on) | **above on all five**, +0.009 % to +0.017 % | `metroid` +0.014 % |
| base vs **the floor** (same binary) | **+0 on all five** | `zelda2` +0.006 %, `smb1` +0.003 % |

The middle row is the park's second justification and the clearest one in the whole campaign: with
`board-292` on, `NesCore::run_frame` itself is measurably larger on **all five games** — that is
the latch store, which sits inside the emulated frame — and with it parked `core` is +0 everywhere.
The behaviour claim is step 7's golden-core, not this table.

### 6.7 What the suites say after the park

| configuration | `mapper_tests` |
| --- | ---: |
| `default` (292 parked) | **385** passed / 0 failed |
| `--features parked-boards` | **392** passed / 0 failed |

385 is 392 less the seven tests that need `board-292`: its five from step 2, its round trip from
step 4, and the shared reset test, which is `cfg(all(board-262, board-292))`. `cargo fmt --check`
and clippy `-D warnings` are clean in both.

## Step 7 — golden-core, and the full run with both ROM roots

### golden-core over the 98 commercial originals, 1 200 frames each

    zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200

| | |
| --- | --- |
| `c02fc02c` (`mfunl12-0-base`) against the **shipped** tree (`mfunl12-4-ship`) | **byte-identical** (`cmp`, not just the digest) |
| and against the tree with **all three features on**, `board-292` included (`mfunl12-1-lot`) | **byte-identical** |
| the digest | `md5 f79e7df842c3ee43e7d9b67861a927b5` |

**That is the digest MF-UNL-2 through MF-UNL-11 all recorded.** Eleven lots, and not one pixel of a
commercial original has moved — which is what the lot brief asks of this one in those words:
"golden-core over the 98 originals must be byte-identical to the base commit's."

**The second row is this lot's own addition to the pattern, and it is the row that matters here.**
`board-292` is parked, so a reader could object that the shipped tree proves nothing about the code
the park switches off — the latch field on `CartridgeMemory`, the store in
`NesBus::write_internal`, the two new forwards on `IrqBoard`. That row is the answer: with all of
it compiled in and reachable, 98 games at 1 200 frames each are **still byte-identical on every
field golden-core records**. The parked feature is a *figure*, not a behaviour change, which is
exactly the distinction the owner's "no exclusion for performance" rule rests on.

It is also what turns the lot's structural claims into facts rather than intentions: three variants
were added to the shared enums (two to `IrqBoard`, one to `DecodeBoard`), `Mmc3Board` grew two
widened `cfg` lists and two accessors gained callers, `BankWindows::or_chr_banks_each` gained a
second caller, `DecodeBoard` gained three forwards and `IrqBoard` three — every one of those is
code that **existing** boards compile.

### The full run with both ROM roots

    ZAXNES_ROMS=G:/3dsen/demo/roms
    ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms
    cargo test --release --no-fail-fast -p zax-nes -p zax-harness -- --include-ignored

**1 162 passed, 0 failed, 0 ignored.** The same command without the two variables and without
`--include-ignored` is **1 153 passed, 0 failed, 9 ignored** — so the nine ROM-driven tests really
ran rather than passing vacuously, which is the check the brief asks for.

1 162 against MF-UNL-11's 1 152 is **ten** tests, not the lot's seventeen: seven of them need
`board-292` and that feature is parked, so they run under `--features parked-boards` and not in
`default`. Step 5's `mapper_tests` table is where all seventeen are accounted for.

## Step 8 — the final pass, Q-172, the census note and the journal

### The shipped binary is the measured binary, to the byte

`make-tree.sh --worktree` was taken of the tree as it ships and built by `build-clean.sh` in a
fresh directory of its own:

| | md5 |
| --- | --- |
| `mfunl12-5-verify`, the shipped tree | `aee4243f1e16e7aaf876751751f68262` |
| `mfunl12-4-ship`, the **measured** binary | `aee4243f1e16e7aaf876751751f68262` |

**Identical** — three independent builds of that configuration now, all three the same md5 — so
step 6's and step 7's figures are the shipped commit's to the byte. Steps 7 and 8 touch only
Markdown and the Cargo manifest's **comments**, which do not reach codegen.

### What this step wrote

| file | what |
| --- | --- |
| `spec/changes/Q-172.md` | **new**. Nine readings across the three numbers, a §0 on the method (raw wikitext) and a §4 table of what a dump would falsify with the cost of each being wrong |
| `docs/review/BOARD-FEATURES.md` | three rows, the `parked-boards` row's mapper list and the "worst figure in `default`" paragraph |
| `docs/review/UNL-census.md` | §7.2's table marks the lot **shipped**, and the MF-UNL-12 section gains the note every shipped lot has |
| `rust/crates/zax-nes/Cargo.toml` | `board-292`'s own note gains the park, its figures and its two causes (step 6) |
| `journal/JOURNAL.md` | the lot's entry, the **MF-UNL-1 to 12 summary** the brief asks for, and the roster of unanswered tickets |

### The lot, end to end

| | |
| --- | --- |
| numbers | **3** — 262, 292, 246 |
| features | **3** — `board-262` and `board-246` in `default`, **`board-292` parked** |
| `Board` discriminants | **0** — two `IrqBoard` variants, one `DecodeBoard` variant |
| new forwards | three on `IrqBoard` (`cpu_read_override`, `cpu_peek_override`, and `try_read_prg`'s `cfg` widened), two on `DecodeBoard` (the PRG read cache, `intercept_cpu_write`) plus `try_read_prg`'s `cfg` |
| outside a board | **one** — `CartridgeMemory::cpu_write_latch` and one store in `NesBus::write_internal`, both behind `board-292` |
| existing boards' behaviour | **unchanged**, and golden-core over the 98 originals says so — with the parked feature **on** as well |
| tests | **17**; `mapper_tests` 385 in `default`, 392 with `parked-boards` |
| tickets | **1**, Q-172 |
| commits | **9**, `00451797` to this one |

### The three things this lot would tell the next one

1. **Read the diagram, not the prose, and read it as raw wikitext.** Two of mapper 262's five
   readings are column positions, and a paraphrase of that one diagram got both wrong — the select
   bit and the order of four lines. `?action=raw` on a nesdev page gives the columns the author
   aligned.
2. **A `cfg` list is never implied by the variant it belongs to.** The `board-246`-alone build did
   not compile, because `DecodeBoard`'s `compile_error!` guard lists every member of that enum and
   nothing else names a new one. MF-UNL-11 found the same thing in `IrqBoard::try_read_prg`'s list.
   The single-feature configurations are the only build that can find it.
3. **A board that wants a `&mut self` read hook is expensive and the cost is structural.**
   `Board::cpu_read_override` carries `#[inline(always)]` and folds into `Cartridge::try_cpu_read`,
   which the MMC5 reaches on every CPU read; a shared enum leaving that dispatch's **empty** arm
   cost `mmc5exram` +1.098 %. There is no `#[inline(never)]` that recovers it, because what is paid
   is the arm and not the call. The next board whose registers a read fills should expect to be
   parked — or to come with a redesign of that hook.
