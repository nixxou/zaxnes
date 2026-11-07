# MF-UNL-2 — the MMC3 clones that need nothing new

Lot **MF-UNL-2** of `docs/review/UNL-census.md` §7.2, the second of the twelve MF-UNL lots:
**twelve numbers, four boards, four Cargo features**, and every one of them is the MMC3 this
build already has.

Base commit: **`186b5656`** (`MF-UNL-1 (8/8)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | numbers | board |
| --- | --- | --- |
| `board-mmc3-chrram` | **74, 192, 194, 195** | the MMC3 with a CHR-RAM window beside its CHR-ROM — the family mapper **119** is already a member of |
| `board-mmc3-scramble` | **114, 115, 121, 123, 182, 187** | the MMC3 with scrambled registers, an outer CHR bank and an NROM-like PRG override |
| `board-263` | **263** | UNL-KOF97: A12→A0 and a data permutation |
| `board-189` | **189** | TXC 01-22017/22018: four 32 KiB PRG banks from a $4100 write |

## The steps

- [x] **0/9 — the plan.** This file: the twelve numbers, the page each is built from, the four
  features, the shape each board takes in this tree, and the two places this lot leaves the
  census's own prescription for the number's page.
- [x] **1/9 — the generalisation, and the lot's own regression gate.** `board-119`'s CHR-RAM
  chip select becomes the `const`-parameter form of the census, shared by all five numbers of
  the family. **119 must come out byte-identical**, and the step proves it two ways: an
  exhaustive table test over all 256 register values in both CHR modes, and a golden-core dump
  over a mapper-119 image.
- [x] **2/9 — `board-mmc3-chrram`.** 74, 192, 194 as three monomorphisations of that form, and
  195 with the page's run-time GAL. `registry.rs` arms, `SUPPORTED_MAPPERS`,
  `board_memory_sizes`, and the tests the census lists.
- [x] **3/9 — `board-mmc3-scramble`.** `Mmc3Scramble`, one board holding an `Mmc3` with six
  wirings: the two scramble patterns, the outer CHR bank, the three NROM override registers,
  115's IRQ rule against 114's, 121's protection array and extended PRG banking, and the CHR
  A18 that 121 and 187 drive from PPU A12.
- [x] **4/9 — `board-263`.** The address map and the data permutation.
- [x] **5/9 — `board-189`.** The 32 KiB PRG bank register over a stock MMC3.
- [ ] **6/9 — the four sweeps and the feature matrix.** Every configuration the lot adds, plus
  `--no-default-features` and `--features parked-boards`; `cargo fmt --check` and clippy
  `-D warnings` in each.
- [ ] **7/9 — the documentation.** Four rows in `docs/review/BOARD-FEATURES.md`, the Cargo
  feature comments, and the question tickets.
- [ ] **8/9 — the callgrind campaign.** PR-1's thirteen ROMs and level 2's five games against
  `186b5656`, every compared commit built from `git archive` in a fresh directory, and the
  feature decision for each of the four boards.
- [ ] **9/9 — golden-core against the base commit**, the full check with `ZAXNES_ROMS` and
  `ZAXNES_TEST_ROMS` set, and the journal.

## The twelve numbers, and the page each is built from

Every figure below comes from the number's own **nesdev wiki** page, cross-checked against
`third_party/mesen2`'s class for the same number. Where the two differ the page wins and the
divergence is named in the board's own doc comment.

### The MMC3 with a CHR-RAM window — 74, 192, 194, 195, and 119 already here

The family's own page, **"MMC3 with CHR ROM and CHR RAM"**, carries the table the whole board
is built from — and it has **four** columns, not the three the census's `(first, last, size)`
names:

| mapper | first RAM bank | size of RAM (1 KiB banks) | RAM mirrored? |
| ---: | --- | ---: | :-: |
| 004 | 0 | 0 or 8 | Yes |
| **074** | 8 | 2 | No |
| **119** | **64, 192** | 8 | **Yes** |
| 176 | 0 | 8 | No |
| 191 | 128 | 2 | Yes |
| **192** | 8 | 4 | No |
| **194** | 0 | 2 | No |
| **195** | 0 | 4 | No |

and its *Behaviour* section says of the three mirrored rows what the "Yes" means: "119: **CHR
A16** switches between CHR ROM and CHR RAM", "191: CHR A17 switches". A single bank line, not a
range — which is why 119's first RAM bank is **64 *and* 192**.

### The six scrambled boards — 114, 115, 121, 123, 182, 187

| # | board | publisher | what it adds to the MMC3 |
| ---: | --- | --- | --- |
| 114 | SuperGame / Hosenkan | SuperGame, Hosenkan | scrambled register **addresses and indices**, two patterns by submapper; outer CHR bank at $6001; NROM override at $6000; **MMC3A IRQ** (latch 0 disables) |
| 182 | the same board | Hosenkan | nothing of its own — NESdev redirects the number to 114's page |
| 115 | Kǎshèng SFC-02B/-03/-004 | Kǎshèng | the same $6000/$6001 pair under mask $E003, no scrambling, **MMC3C IRQ** (latch 0 fires every scanline) |
| 123 | UNL-H2288 | Kǎshèng | index scrambling only; the override at **$5800** with its bits rearranged |
| 187 | Kǎshèng A98402 | Kǎshèng / Hummer Team | the override at **$5000**, a protection read, and **CHR A18 from PPU A12** |
| 121 | Kǎshèng A9711 / A9713 | Kǎshèng | a protection array at $5000-$5FFF, an outer bank at $5180, **extended PRG banking from bit-reversed $8001 values**, and the same CHR A18 trick |

### 263 and 189

* **263** (UNL-KOF97, Rex Soft): mask $F000, A12 driving the MMC3's A0, and the written byte
  permuted `76543210 → 76243051`.
* **189** (TXC 01-22017-000 / 01-22018-400, FC-001EM): a 32 KiB PRG bank register at
  `(address & $E100) == $4100`, over a stock MMC3 whose own PRG registers no longer reach
  $8000-$FFFF.

## The two places this lot leaves the census's prescription for the page

1. **Mapper 119's CHR-RAM select is a line, not a range.** The census says "119 becomes
   `(0x40, 0x7F, 8)`", which is Mesen 2's registration (`MMC3_ChrRam(0x40, 0x7F, 8)`) read as a
   closed range — and under that reading a CHR bank of $C0-$FF reads the CHR-**ROM**. The
   family page says 119's first RAM bank is "64, **192**", that its RAM is mirrored, and that
   "CHR A16 switches between CHR ROM and CHR RAM"; today's `board-119` agrees with the page
   (`self.bank[register] & 0x40`). The lot's own rule — *no change to the emulation of an
   existing mapper* — settles it: the page's reading stays, and the `const` form carries the
   page's fourth column as a fourth parameter so that it can. See step 1.
2. **Mapper 195's RAM window moves at run time, and its power-on row is not `$00-$03`.** The
   census says `(0x00, 0x03, 4)` — again Mesen's registration — and then asks for "the page's
   run-time GAL" in the same sentence. The page's register has a **power-on value of `$80`**,
   which selects banks **`$28-$2B`**, and `$00-$03` is the `$82` row; the page also notes that
   the common *Captain Tsubasa II* images "have been hacked to work with a fixed setting of
   `$82` (`$00-$03`)". The two cannot both be the power-on state. Q-142 records the choice.

## Where each board sits in `Board`

All four are an MMC3 with its scanline counter, so all four answer **yes** to
`Mapper::irq_asserted` — which is `IrqBoard`'s entry condition, and the enum mappers 119, 47
and 37 are already in. No new `Board` discriminant: MF-6a3 step 1 priced two of them at
`mmc5` **+0.080 %** through the jump table `NesBus::push_expansion_audio` inlines whole, and
`Board` stays at thirty-five.

`IrqBoard` grows **four** variants, 10 → 14, and each one is **boxed** for the reason
`IrqBoard::Somari` is: MF-6b step 1 measured one byte of displacement in this enum's payload at
+0.009 % to +0.027 % on eleven of lot PR-1's thirteen ROMs, so a variant that would set the
enum's size goes behind a pointer and the thirteen other boards do not move. The price is one
dereference per forwarded call for a cartridge of this lot and nothing for anybody else, which
is §4.10's rule.

`Mapper::try_read_prg` is forwarded for the first time on this enum, behind
`board-mmc3-scramble` alone: 121's protection array and 187's protection read are both below
$6000 and *The King of Fighters '96* depends on the second one.

## Step 1 — the generalisation, and the lot's own regression gate

`Mmc3Board`'s `const TQROM: bool` is gone; in its place are the four columns of nesdev's
*MMC3 with CHR ROM and CHR RAM*:

```rust
pub struct Mmc3Board<
    const MC_ACC: bool,
    const CHR_RAM_FIRST: u8,
    const CHR_RAM_LAST: u8,
    const CHR_RAM_KIB: u8,
    const CHR_RAM_MIRRORED: bool,
    const MULTICART: bool,
>
```

with `Mmc3ChrRam<FIRST, LAST, SIZE>` as the census's own three-parameter alias and
`Tqrom = Mmc3Board<false, 0x40, 0x7F, 8, true, false>`. `if TQROM` became
`if Self::HAS_CHR_RAM` (`CHR_RAM_KIB != 0`) in the four places it stood, so mappers 4, 118, the
MC-ACC and the two multicarts keep bodies in which the whole mechanism is compile-time dead.

Three functions carry the generalisation:

| | |
| --- | --- |
| `Mmc3Board::chr_ram_selected` | the census's sentence — `FIRST..=LAST` is the RAM — behind `MIRRORED`, which reduces the bank by `& LAST` first |
| `Mmc3Board::chr_bank_of_window` | the 1 KiB bank number a window shows, re-derived from the registers; `refresh_chr_source` and `chr_ram_offset` now share it (it was the body of `tqrom_ram_offset`) |
| `Mmc3Board::CHR_RAM_BANK_MASK` | `SIZE - 1`, the chip's own bank lines; `saturating_sub` so the no-chip monomorphisations have a defined mask |

`refresh_chr_source` is now one pass over the eight windows instead of eight hand-written
`bit(register, slot)` terms: MF-6a3 step 1 could assume a 2 KiB register's two halves land on
the same chip, because its select was one line, and a general `FIRST..=LAST` window cannot be
assumed that.

### The gate, and the fact that the census's literal reading would have broken it

**`req_core_100_tqrom_chr_ram_select_is_chr_a16_over_all_256_bank_numbers`** — the new test —
walks **all 256 values** of a bank register through a 1 KiB register in CHR mode 0, the same
register in CHR mode 1, and a 2 KiB register, and asserts the chip *and* the offset from one
byte. It is written so that it can say both at once: the image has a **64-bank** CHR-ROM, whose
numbered banks therefore read `0..=63`, and the eight CHR-RAM banks are filled with `$40 | bank`
— outside that space, so `$40 | k` means "the RAM chip, bank `k`" and nothing else can.

| | |
| --- | --- |
| the test, run at `186b5656` with the old code | **passes** |
| the test, run after the generalisation | **passes** |

And golden-core, over three mapper-119 images made by patching bytes 6 and 7 of
`holy-mapperel`'s `M4_P256K_C256K`, `M4_P128K` and `M4_P128K_CR8K` (a scratch directory, nothing
committed), 1 200 frames each:

    zax-harness golden-core <dir> gc119-<label>.jsonl 1200

| | |
| --- | --- |
| `186b5656` against the lot through step 1 | **byte-identical**, `md5 c36d16e8…` both |

**Both halves of the gate are sensitive to exactly the rule at issue**, which was checked by
breaking it on purpose: with `MIRRORED` set to `false` — the census's literal
`(0x40, 0x7F, 8)`, which is Mesen 2's `page >= _firstRamBank && page <= _lastRamBank` — the
golden-core dump **differs** from `186b5656`'s and the new test **fails**. So the fourth column
of the family's page is not a formality: reading the census's pair as a closed range is an
observable change to mapper 119, and the lot's own rule forbids it. `Mmc3Board::chr_ram_selected`
carries the argument, and the plan's item 1 above records the choice.

`pixels_sha256` of those three images is the same for mapper 4 and mapper 119 and
`tiles_sha256` is not, so what the dump is sensitive to is the CHR *source*, which is the point.

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **945 passed, 0 failed, 9 ignored** (944/0/9 at `186b5656`, +1 for the gate) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes` | clean |

One more thing this step had to notice: a Cargo feature comment naming `chr_ram_windows` fails
`req_arch_002_the_crate_has_no_platform_dependency`, which scans the manifest's text for
`windows`. The comment says "the byte of per-slot source flags" instead.

## Step 2 — `board-mmc3-chrram`: 74, 192, 194 and 195

Four `const` monomorphisations of the form step 1 built, behind one feature that **implies
`board-119`** (the mechanism is 119's; this adds the rows of the family's page that were not
here) and one new `IrqBoard` variant:

| alias | row of *MMC3 with CHR ROM and CHR RAM* |
| --- | --- |
| `Mmc3ChrRam74 = Mmc3ChrRam<0x08, 0x09, 2>` | first RAM bank 8, 2 banks, not mirrored |
| `Mmc3ChrRam192 = Mmc3ChrRam<0x08, 0x0B, 4>` | first RAM bank 8, 4 banks, not mirrored |
| `Mmc3ChrRam194 = Mmc3ChrRam<0x00, 0x01, 2>` | first RAM bank 0, 2 banks, not mirrored |
| `WaixingFs303 = Mmc3Board<false, 0, 0, 8, false, true, false>` | the GAL: the window is a register, the chip 8 KiB |

`Mmc3ChrRamBoard` (`cartridges/mappers/mmc3_chrram.rs`) holds the four, and
`IrqBoard::Mmc3ChrRam(Box<…>)` holds it. **The box earns its place**: measured on this tree,
`size_of::<IrqBoard>()` is **200** and `size_of::<Mmc3>()` **192** with the feature on and with
it off, and `size_of::<Board>()` **1376** in both — so neither the enum MF-6b step 1 priced one
byte of displacement in nor the MMC3's own struct moved. (195's `chr_ram_gal` byte landed in
padding `Mmc3Board` already had, which is the same thing MF-6a2 step 3 found for
`chr_ram_windows`.)

### Mapper 195's GAL

`Mmc3Board::gal_window` is the page's eight-row table read as **one three-bit field** — `(Z, L, L)`
= bits 6, 3 and 1 — which is what its diagram's own arrow draws and what makes the page's list
`$80, $82, $88, $8A, $C0, $C2, $C8, $CA` consecutive. So it is one table and not two, bits 5, 2
and 0 are don't-cares, bit 4 (`D`) wins over the field, and bit 7 "Must be 1".

The register is latched by **a PPU write to a window the CHR-ROM drives**, which is the page's
"Writes to CHR ROM" read as the only way a board with one PPU write line can tell a register
write from a RAM write. It lives in `chr_ram_write`'s existing "the ROM chip drives this window,
so the write goes nowhere" branch, so no other board of the family gains an instruction.

Two question tickets, both referenced from the code:

* **Q-142** — 195's window and its power-on state. The census says `(0x00, 0x03, 4)`, which is
  Mesen's static registration and the `$82` row; the page says the window is the GAL's, its
  power-on is `$80` (`$28-$2B`) and the chip is **8 KiB**, because its own worked example
  ("`$80` … selects different 4 KiB … from `$88` … but the same 4 KiB as `$82` … because CHR A13
  is not connected") cannot be true of a 4 KiB chip. The page is implemented; the ticket records
  that the census's reading is defensible for the *hacked* *Captain Tsubasa II* images the page
  itself names, and that the choice made here runs those too.
* **Q-143** — 192's "If the NES 2.0 header is present, 74 and 192 should be emulated
  identically, as NES 2.0 specifies CHR RAM size elsewhere". Not implemented: the board's chip is
  the board's, which is the contract `board_memory_sizes`'s own doc comment states, and taking RAM
  away from a 192 image whose header said 2 KiB is the worse of the two errors.

### Two latches a new MMC3 number has to be added to, and the test that caught it

`Cartridge::watches_ppu_reads` and `watches_ppu_bus_without_read` are tested **before** the
`Board` dispatch, so a number missing from either has its A12 counter silently never clocked.
All four were added to both, and `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`
is what found it: the new `req_core_114_…` IRQ test failed first, and that sweep then failed on
its own expected list. §4.3's warning, exactly as MF-6a3 step 1 recorded it for 119.

### Tests from the pages

| test | what it pins |
| --- | --- |
| `req_core_100_the_chr_ram_family_puts_its_own_window_on_the_ram_chip` | per number: every bank of the window is a writable RAM; one below `FIRST` and one above `LAST` are ROM and swallow a write; **`FIRST + SIZE` is ROM**, which is the `RAM mirrored? No` column and what separates these three from the TQROM; and the chip is the row's size |
| `req_core_100_mapper_195_gal_moves_the_chr_ram_window_at_run_time` | all 256 register values against all 256 bank numbers, the 8 KiB chip, the `$80` power-on, "Must be 1", `$CA`/`D` = only CHR-ROM, and the page's worked example about CHR A13 |
| `req_core_114_the_chr_ram_family_keeps_the_mmc3s_scanline_counter` | per number: the counter asserts on the fifth A12 rise with latch 4 and not before, and `$A001` protects the PRG-RAM — `IrqBoard`'s entry condition, and the two latches above |

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **948 passed, 0 failed, 9 ignored** (+3 over step 1) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes` | clean |

## Step 3 — `board-mmc3-scramble`: 114, 115, 121, 123, 182 and 187

`Mmc3Scramble` (`cartridges/mappers/mmc3_scramble.rs`) is **one board with six wirings** on the
`Jv001` precedent, and it **holds** an `Mmc3` on `Somari`'s and `NesEvent`'s: the chip keeps its
own registers and is asked to recompute from them after every write, then the board lays its own
banking over. `IrqBoard::Mmc3Scramble(Box<…>)` holds it, boxed for the reason `Somari` is.

What the number picks:

| field | 114 | 182 | 115 | 123 | 187 | 121 |
| --- | --- | --- | --- | --- | --- | --- |
| address scramble | sub 0 / sub 1 | sub 0 | — | — | — | — |
| index scramble | sub 0 / sub 1 | sub 0 | — | **sub 0's** | — | — |
| override port | `$6000` / `$E001` | the same | `$6000` / `$E003` | `$5800` / `$F800` | `$5000` / `$F001` | — |
| outer CHR bank | `$6001` | `$6001` | `$6001` | — | — | `$5180` |
| CHR A18 from PPU A12 | — | — | — | — | **yes** | **yes** |
| IRQ revision | **A (MMC3A)** | **A** | B (MMC3C) | B | B | B |
| protection | — | — | — | — | one read | the array, `$8003`'s table |

Three things the step had to get right that the census's sentence does not say:

1. **The address scramble and the index scramble are two choices, not one.** 123 scrambles
   indices and not addresses — "drops … register address scrambling while retaining index
   register scrambling (using the same pattern)" — so `Mmc3Scramble` carries two row numbers.
2. **The override-enable bit is bit 7 on three diagrams and bit 6 on 123's.** `[.M31 .2S0]`
   puts `M` one place right. The first run of
   `req_core_100_the_nrom_override_takes_prg_away_from_the_mmc3` failed on exactly that, which
   is what the test is for.
3. **115's "sharp" IRQ rule is this build's default, and the contrast that tests it is 114.**
   The census writes the test as "latch = 0 asserts on every scanline, where the MMC3 proper does
   not", and in *this* build the MMC3 proper is revision B and **does** — REQ-CORE-114's own
   `mmc3_test_2` test 6. So 115 needs no code at all and 114 needs
   `Mmc3Board::force_irq_revision_a`, whose note carries both pages' sentences.

Two additive methods on `BankWindows`, both behind the feature and both **idempotent** so that
the overlay can be re-applied after every write: `remap_prg_banks(mask, bits)` for an outer PRG
bank that is an address *line* (115's A18, 121's `$5180`), and `or_chr_banks(from_slot, bits)`
for CHR A18 — whose `from_slot` **is** the PPU-A12 wiring, four windows and not eight.

`IrqBoard` forwards `Mapper::try_read_prg` for the first time, for 121's protection array and
187's protection read. This is the dispatch MF-CART freed, and step 8 re-measures it.

### Q-144 — four places this family's page beats Mesen 2, and one that is observable

The ticket holds all four with the symptom of each; the one that matters is the first:

* **123's `$5800` bank field.** The page's diagram puts the bank's bit 1 at register bit **4**
  and flags the field "(notice the bit order)"; `MMC3_123.h` reads register bit **3**. Half of
  all `$5800` values differ between the two readings, so *Mortal Kombat 3* would jump into the
  wrong 16 KiB bank. **No mapper-123 image is available to this project**, so nothing here can
  settle it and a reviewer with a ROM should.
* **114 and 182's one-to-one address map**, which 114's page argues from a comparison of
  *Aladdin* with *Super Aladdin* and which Mesen implements in two different refuted ways.
* **115's `$6000` bit 6, "PRG A18"**, the one field of that register with no "If M=1" qualifier,
  which Mesen does not read at all.
* **121's and 187's CHR A18 from PPU A12**, "regardless of what the MMC3 is set to do", where
  Mesen drives it from the MMC3's CHR mode instead.

And four things of Mesen's that are on no page and are deliberately not built: 115's `$5080`,
187's `$6000`, 121's `$5180` power-on of `$80` ("Hack for Super 3-in-1" in its own source) and
121's second protection array, which the page attributes to FCEUX rather than asserting.

115's own readable register, the **`$6002` solder pad**, is a recorded gap: the page gives the
field and never the value, and no game in its list is said to read it.

### One refactor this step forced

`Cartridge::create` went past `clippy::too_many_lines` with six more numbers in each of the two
PPU-bus latch lists, so those two `matches!` are now the free `const fn` `watches_ppu_reads` and
`watches_ppu_bus_without_read`, with the whole of the lot-by-lot commentary moved onto the first.
The decision is the same one, made at the same moment, from the same numbers.

### Tests from the pages

| test | what it pins |
| --- | --- |
| `req_core_100_the_scrambled_boards_reproduce_an_unscrambled_mmc3` | all seven wirings against a plain mapper-4 image of the same ROM: the scrambled address **and** index give the same PRG and CHR layout, and on 114/182 the *unscrambled* pair does not |
| `req_core_100_the_nrom_override_takes_prg_away_from_the_mmc3` | 114, 115, 123 and 187: NROM-128 shows one 16 KiB bank twice, NROM-256 a 32 KiB window, the MMC3's own registers stop reaching the bus while the override is set, and come back when it is cleared |
| `req_core_100_the_outer_chr_bank_of_114_and_115_is_chr_a18` | `$6001` bit 0 moves the CHR window by exactly 256 KiB, and back |
| `req_core_100_mappers_121_and_187_drive_chr_a18_from_ppu_a12` | the left pattern table in the first 256 KiB half and the right in the second, **in both CHR modes** — which is the only form in which the page and Mesen differ |
| `req_core_114_mapper_114_disables_the_irq_on_a_zero_latch_and_115_does_not` | 1 IRQ over three A12 rises on 114 and 182, 3 on 115, 121, 123 and 187 |
| `req_core_100_mappers_121_and_187_answer_their_protection_reads` | 121's four-byte array over all four indices and at a second address under its mask, that only bits 0-1 are the index, and that 187's read has bit 7 set |
| `req_core_100_mapper_121_banks_prg_from_bit_reversed_8001_values` | `$26`/`$28`/`$2A` bank `$E000`/`$C000`/`$A000` from the bit-reversed `$8001`, `$26` is sticky on a later `$8001`, `$2F` keeps what was selected, and an invalid index lets the MMC3's own banking back |
| `req_core_100_mapper_182_tolerates_the_zero_its_games_write_at_startup` | the page's own observation about Hosenkan's games, as the census asks |

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **956 passed, 0 failed, 9 ignored** (+8 over step 2) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes` | clean |

## Steps 4 and 5 — `board-263` and `board-189`

Both are an `Mmc3` with something in front of it, so both are the `Somari` shape again and
neither needs boxing: `Mmc3Kof97` is an `Mmc3` and nothing else and `Mmc3Txc189` an `Mmc3` plus
one byte, so each is the size of the `Tqrom` variant this enum already had.

### `board-263` — the Rex Soft UNL-KOF97

Two translations in front of the chip and no register of its own, which is all its page
documents.

* **The address**: "A12 is used for the MMC3's A0", which one line reproduces —
  `(address & $E000) | ((address >> 12) & 1)` walks the page's eight-row table exactly. The
  board's A12 is a bit an MMC3 ignores entirely, which is what makes it free to carry A0.
* **The data**: `76543210 → 76243051`, implemented as the page's own expression
  `(V & 0xD8) | ((V & 0x20) >> 4) | ((V & 4) << 3) | ((V & 2) >> 1) | ((V & 1) << 2)`.

The page states the permutation **twice** — once as a bit map and once as that expression — so
`req_core_100_mapper_263_permutes_every_one_of_the_256_written_bytes` builds it from the **bit
map** and checks all 256 inputs against the board. The two statements prove each other.

The first run of that test failed, and the reason is worth recording: **the register index is
permuted too**, being a byte written to the same clone, so selecting R2 means writing the byte
that permutes to 2. That is the first thing a test of this board has to notice and the test now
says so in a comment.

The page's own *Note* is about the dumps and not the board — "Most versions of *The King of
Fighters '97* are hacked for Mapper 4. Several of them also fail to disable the frame IRQ,
freezing on emulators that accurately power-on with the frame IRQ enabled" — and this core does
power on with it enabled (REQ-CORE-113), so such a dump freezes here too, correctly and on the
APU rather than on the board. Recorded in `Mmc3Kof97`'s own note.

### `board-189` — the TXC 01-22017/01-22018 and the Yoko FC-001EM

A 32 KiB PRG bank register over a stock MMC3, and the chip's PRG bank lines reach nothing: the
number exists because the board "bank[s] PRG-ROM in 32 KiB amounts, like AxROM, BNROM and
GNROM", so `R6` and `R7` latch and drive no window, which the test asserts directly.

**"Address bit 9" is A8, the ninth line**, and the page says so three ways: its mask `$E100`
reads A15, A14, A13 and **A8**; its address diagram `A~[010. ...1 .... ....]` sets the bit in the
nibble that is bits 11-8; and the oversize game "still writes to the original `$4132` address",
which has A8 set and A9 **clear**. So the decode is `(address & $E100) == $4100`, the census's
"a `$4100` write with A9 set" is `$4100` against `$4000`, and the test uses `$4132` by name.

**Three bits of the byte are taken, not two**, which is the page's own *Generalized definition
for emulation* read for the case it describes — "respond to writes to `$4100-$5FFF` with address
bit 9 set, and only use the upper nibble of the data byte being written". The oversize board's
`[.PPP ....]` is the original's `[..PP ....]` with PRG A17 above it, and on the original board's
128 KiB that bit wraps away, so **one decode runs both boards**. The test makes that claim on two
images, a 256 KiB one where all eight banks are reachable and a 128 KiB one where the third bit
wraps.

**The generic pirate boards are deliberately not decoded** — A-37/810809-C and
K-3086/810836-C, which move the register onto the MMC3's PRG-RAM interface at `$6000` — and the
page's own paragraph is the reason: "There are countless pirated variants of the (already
unauthorized) *Street Fighter II* 8-bit backport. … doing so requires **modifying the PRG-ROM
content** to change the write address". Mesen 2 decodes the whole of `$4120-$7FFF` and ORs the
two nibbles of every byte to cover them. The same shape, and the same reason, as the 60-pin
decode `Sachen72008` records. The test pins it: `$4100 = $01` changes nothing.

**The PT8154's missing counter is not a parameter.** *Thunder Warrior*'s 28-pin PT8154 "is a
stripped-down MMC3 without any IRQ functionality" and *Street Fighter II*'s 40-pin pair "needs a
fully functional MMC3 clone" — and **nothing in an image distinguishes them**: the page gives the
number no submapper and the boards differ in pin count. So the counter is built, unchanged, and a
*Thunder Warrior* dump simply never writes `$C000-$E001`, which is the harmless direction. The
test drives the counter to prove it is there.

### The check for both steps

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **959 passed, 0 failed, 9 ignored** (+3 over step 3) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes` | clean |
| golden-core over the three mapper-119 images, against `186b5656` | **still byte-identical** with all five boards built and in `default` |

| test | what it pins |
| --- | --- |
| `req_core_100_mapper_263_permutes_every_one_of_the_256_written_bytes` | the page's bit map against the board's expression, over all 256 bytes |
| `req_core_100_mapper_263_drives_the_mmc3s_a0_from_a12` | the whole bank layout against a plain mapper-4 image of the same ROM, plus the three registers an address table alone cannot reach — `$A000`'s mirroring, and `$C000`/`$D000`/`$F000` as the latch, the reload and the enable |
| `req_core_100_mapper_189_banks_32_kib_of_prg_from_4100` | all eight 32 KiB banks from bits 4-6; the MMC3's own PRG registers reaching nothing; `$4132`; `$4000` not decoded; the low nibble not decoded; PRG A17 wrapping on 128 KiB; and the MMC3's CHR banking, mirroring and counter intact |
