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

- [ ] **0/9 — the plan.** This file: the twelve numbers, the page each is built from, the four
  features, the shape each board takes in this tree, and the two places this lot leaves the
  census's own prescription for the number's page.
- [ ] **1/9 — the generalisation, and the lot's own regression gate.** `board-119`'s CHR-RAM
  chip select becomes the `const`-parameter form of the census, shared by all five numbers of
  the family. **119 must come out byte-identical**, and the step proves it two ways: an
  exhaustive table test over all 256 register values in both CHR modes, and a golden-core dump
  over a mapper-119 image.
- [ ] **2/9 — `board-mmc3-chrram`.** 74, 192, 194 as three monomorphisations of that form, and
  195 with the page's run-time GAL. `registry.rs` arms, `SUPPORTED_MAPPERS`,
  `board_memory_sizes`, and the tests the census lists.
- [ ] **3/9 — `board-mmc3-scramble`.** `Mmc3Scramble`, one board holding an `Mmc3` with six
  wirings: the two scramble patterns, the outer CHR bank, the three NROM override registers,
  115's IRQ rule against 114's, 121's protection array and extended PRG banking, and the CHR
  A18 that 121 and 187 drive from PPU A12.
- [ ] **4/9 — `board-263`.** The address map and the data permutation.
- [ ] **5/9 — `board-189`.** The 32 KiB PRG bank register over a stock MMC3.
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
