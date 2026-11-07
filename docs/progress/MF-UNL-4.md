# MF-UNL-4 — the J.Y. Company ASIC

Lot **MF-UNL-4** of `docs/review/UNL-census.md` §7.2, the fourth of the twelve MF-UNL lots:
**four numbers, one board, one Cargo feature** — the smallest feature matrix of any lot so far and
the largest board, which is why §7.2 prices it at eight hours and calls it the only board in the
plan that needs **four hooks at once**.

Base commit: **`16a30564`** (`MF-UNL-3 (9/9)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | numbers | board |
| --- | --- | --- |
| `board-jy-asic` | **35, 90, 209, 211** | 晶太 (Jīngtài) / J.Y. Company's proprietary ASIC: four PRG banking modes, four CHR banking modes, an MMC4-like CHR latch, ROM nametables and Extended Mirroring, a four-source IRQ counter with a prescaler and an XOR register, and an unsigned 8×8→16 hardware multiplier |

The page is **one page for all four numbers** — nesdev's "J.Y. Company ASIC", to which
`INES Mapper 035`, `INES Mapper 090`, `INES Mapper 209` and `INES Mapper 211` all redirect — and
§1.6 of the census grades it "complete, and the most complete of any unlicensed board in the
census". The four numbers are four PCB configurations of one chip:

| number | what the page says it is |
| ---: | --- |
| **209** | "the **standard implementation**, used for most single-game cartridges. The outer PRG bank size is 512 KiB" |
| **90** | "inhibits, **via jumper**, ROM nametables and Extended Mirroring that would otherwise be enabled via register" |
| **211** | "supposedly has ROM nametables and Extended Mirroring always enabled regardless of register content. **No such PCB exists in real hardware**; the mapper was defined when the function of register $D001 bit 3 was not known, and is a mere duplicate of INES Mapper 209 with correct emulation" |
| **35** | "like Mapper 209 but **explicitly specifies 8 KiB of WRAM**. In the presence of a valid NES 2.0 header, which has its own PRG-RAM field, INES Mapper 035 is an exact duplicate of INES Mappers 209" |

So the per-number wiring is **two `bool`s and nothing else**: whether the jumper inhibits the
nametable registers (90), and whether the board states its own 8 KiB of WRAM (35). 211 and 209 are
the same `const` parameters, which is what the page asks for in so many words.

## The three places this lot leaves the census's prescription, and the one it leaves Mesen's

MF-UNL-2 found that §7.2's steps were drafted from Mesen 2's `MapperFactory` and that three of them
contradicted the numbers' own pages; the rule it set is **the number's own page wins**, each case in
a Q ticket. This lot has four, and three of them are Mesen's reading rather than the census's:

| what | the census / Mesen 2 | the page | ticket |
| --- | --- | --- | --- |
| **mapper 35** | the census: "`MapperFactory` returns … `Mapper35` for 35", and Mesen's `Mapper35.h` is **a different and much smaller board** — four PRG registers, eight CHR registers, an MMC3-style *filtered* A12 counter at $C002/$C003/$C005 and a mirroring bit at $D001, with no multiplier, no banking modes, no nametable control and no prescaler | 35 "is like Mapper 209 but explicitly specifies 8 KiB of WRAM … an exact duplicate" | **Q-148** |
| **mapper 211** | Mesen 2 special-cases it: `(_advancedNtControl \|\| MapperID == 211) && MapperID != 90`, so 211 behaves "as though N were always set" | 211 "is a mere duplicate of INES Mapper 209 **with correct emulation**" | **Q-149** |
| **Extended Mirroring** | Mesen 2 has no decode of `$D001` bit 3 at all: its `_advancedNtControl` is `$D000` bit 5, and it uses that one bit for both ROM nametables and the per-table CIRAM select | `$D001` bit 3 is Extended Mirroring (the per-table CIRAM bank) and `$D000` bit 5 is ROM nametables; the page's last line of the $B00x section says enabling the latter with every `$B00x` bit 7 matching `$D002` bit 7 "effectively duplicates the functionality of Extended Mirroring", which is only a sentence one can write if they are two registers | **Q-149** |
| **PRG mode 3's bit reversal** | Mesen 2's `InvertPrgBits` maps bits 0,1,2 → 6,5,4 and 4,5,6 → 2,1,0 and **drops bit 3** | "8 KiB banking, but with bank numbers **bits 0-6 reversed**", which maps bit 3 to itself | **Q-150** |

And one gap that is this core's and not the page's: **IRQ source 3, "CPU writes"**, has no hook to
hang on — `Mapper::on_cpu_cycle` is told that a CPU cycle elapsed and not whether it was a write,
and a CPU write to RAM never reaches the cartridge at all. Sources 0 (CPU M2), 1 (PPU A12 rise) and
2 (PPU reads) are built; source 3 selects a counter that does not advance. **Q-151**.

## Where the board goes, and why the census's "main risk" is not one

§7.2 step 1 calls the nametable question "the lot's main risk": `Mapper::read_nametable` is
"capped at **two distinct bodies** by the Q-123 measurement, and this board would be a third".

**It would be the sixth, and it adds none.** `Board::read_nametable` already dispatches five
distinct bodies — the MMC5's `ExRAM`/fill, the UNROM 512's four-screen CHR-RAM, the GTROM's
nametable pages, and the two forwards `IrqBoard::read_nametable` (mapper 68, MF-6c step 4) and
`DecodeBoard::read_nametable` (mapper 111, MF-6c step 9) — and MF-6c re-measured the ceiling twice
and did not find it (`Ppu2C02::read_nametable_inner` 114 → 75 at the third body, **75 again** at the
fourth). This board is an `IrqBoard` variant, so its nametable body sits **inside** the
`IrqBoard::read_nametable` forward that mapper 68 already pays for and `Board`'s own dispatch does
not grow an arm. Step 1 measures it rather than quoting that, which is what §4.5 asks the next lot
to do either way.

`IrqBoard` is the right variant for the ordinary reason too — the board's counter pulls /IRQ, which
is that enum's entry condition — and it is the variant that **already forwards every hook this
board needs**: `on_cpu_cycle`, `on_ppu_address_bus` (`#[inline(never)]`), `irq_asserted`
(`#[inline(never)]`), `nametable_page` (`#[inline(never)]`), `read_nametable`, `prg_ram_enabled` and
`try_read_prg`. Four of those seven need their `#[cfg]` list widened and **no new dispatch body is
created in `Board` at all**. MF-CART step 6 measured twelve more variants in a shared enum at
`mmc5` +3 695 and a worst ROM of +0.002 %, so the variant itself is the cheap half.

**Boxed**, on [`IrqBoard::Somari`]'s own measurement: the struct is a `BankWindows` plus about
eighty bytes of registers and a four-entry derived page cache, which inline would be the variant
that sets this enum's size, and MF-6b step 1 priced one byte of displacement in that payload at
+0.009 % to +0.027 % on eleven of lot PR-1's thirteen ROMs.

**The CHR latch goes on `on_ppu_address_bus` and not on `Board::ppu_read`.** The page says the
latch moves when "the PPU **reads** from $0FD8-$0FDF/$1FD8-$1FDF", and every such read already
reaches `Cartridge::on_ppu_address_bus`, which `IrqBoard` forwards out of line. `IrqBoard` does
**not** override `Board::ppu_read` — its arm is `Mapper`'s default, which calls the forwarded
`ppu_peek` — so overriding it would add a distinct body to a dispatch §4.10 prices at "one arm's
worth, +16 measured" for every board in the enum. Mesen 2 puts the same latch on the same hook
(`NotifyVramAddressChange`). `Mapper::advance_chr_latch` is overridden for the tile emitter
(REQ-CORE-039), as the MMC2's is.

## The steps

- [x] **0/9 — the plan.** This file: the four numbers, the one page they share, the two `bool`s
  that are the per-number wiring, the four places this lot leaves a prescription for the page, the
  `IrqBoard` variant and why each of the four hooks is free in `Board`'s dispatches.
- [x] **1/9 — settle the nametable question.** A sixth `Mapper::read_nametable` body measured
  against Q-123's ceiling on this tree, in two shapes, with the figures in
  `rust/docs/PERFORMANCE-HISTORY.md` §5.18. **`Ppu2C02::read_nametable_inner` does not move at
  all**, so the board serves its nametables itself and the feature ships.
- [x] **2/9 — `JyCompany`, the register map and the banking modes.** $5000/$5400/$5C00 solder pad,
  the $5800-$5801 multiplier, the $5802-$5803 accumulator and test register, the four $8000-$8003
  PRG registers with the four modes, the sixteen $9000-$A007 CHR registers with the four modes and
  the $D003 outer bank, and $D000-$D003. The four `registry.rs` arms, the four `SUPPORTED_MAPPERS`
  entries, the four `board_memory_sizes` rows, the save-state block.
- [x] **3/9 — the IRQ.** One prescaler and one counter, four sources (three of them clocked, see
  Q-151), up/down/off counting, the $C006 XOR register, the $C001 prescaler mask, and the page's
  "disabling acknowledges, inhibits counting and resets the prescaler to zero". `on_cpu_cycle`,
  `on_ppu_address_bus`, `irq_asserted`, and `ppu_irq_horizon` answering `None`.
- [x] **4/9 — the MMC4-like CHR latch.** $D003 bit 7, the two latches, the four 4 KiB registers
  they pick between, and `advance_chr_latch` for the tile emitter.
- [x] **5/9 — the nametables, the jumper and the $6000-$7FFF window.** Extended Mirroring
  ($D001 bit 3), ROM nametables for all ($D000 bits 5 and 6) and for some ($D002 bit 7), 90's
  jumper, 35's 8 KiB of WRAM, and $D000 bit 7's 8 KiB PRG-ROM bank at $6000-$7FFF.
- [x] **6/9 — the question tickets, the four sweeps and the clippy configurations.** Q-148 to
  Q-151, the four sweeps over `SUPPORTED_MAPPERS` in every configuration the lot adds, and
  `cargo fmt --check` plus clippy `-D warnings` in the four configurations the brief names.
- [x] **7/9 — the callgrind campaign, per hook.** PR-1's thirteen ROMs and level 2's five games
  against `16a30564`, one binary per hook added, every compared commit built from `git archive` in
  a fresh directory. The feature decision and the `BOARD-FEATURES.md` row.
- [x] **8/9 — golden-core against the base commit, and the full check.** Byte-identical over the
  98 commercial originals; `cargo test --release -p zax-nes -p zax-harness`; the ignored tests with
  `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set.
- [ ] **9/9 — the journal, the census note and the final pass.**

---

## Step 1 — the nametable question, settled: the ceiling is not there

§7.2's step 1 says "Measure a third `Mapper::read_nametable` body against the Q-123 ceiling. If it
costs what Q-123 measured, the board serves nametables through the existing CHR-ROM path instead
and the feature ships; if it does not fit at all, the board is **parked**." Two things about that
sentence had to be fixed before it could be obeyed:

* a body would be the **sixth**, not the third — `Board::read_nametable` dispatches the MMC5's
  `ExRAM`/fill, the UNROM 512's four-screen CHR-RAM, the GTROM's, and the two forwards
  `IrqBoard::read_nametable` (mapper 68, MF-6c step 4) and `DecodeBoard::read_nametable`
  (mapper 111, MF-6c step 9);
* and "serves nametables through the existing CHR-ROM path instead" is not an alternative to
  measuring. There is no existing CHR-ROM nametable path a *second* board can borrow: mapper 68's
  body indexes its own two registers. What there is, is mapper 68's **forward** —
  `IrqBoard::read_nametable` — and a body put inside that forward adds nothing to `Board`'s own
  dispatch. So the step measured both shapes.

### The three binaries

| label | tree |
| --- | --- |
| `unl4base` | **`16a30564`**, the lot's base commit, five bodies |
| `nt6board` | the same source with a **sixth `Board` arm body**: the CHR-ROM indexed load the J.Y. board will use, given to `Fme7`, which is a direct `Board` variant — the shape the census feared |
| `nt6irq` | the same body given to `Sunsoft3`, so it lands **inside** mapper 68's `IrqBoard::read_nametable` forward — the shape this lot ships |

Each built by `build-clean.sh` from its own tar into a fresh directory with its own
`CARGO_TARGET_DIR`, which is then deleted. Both bodies are **dead code** — neither board's
`nametable_page` ever answers `SERVED_BY_CARTRIDGE` — and that is the experiment: Q-123's own
finding is that "the trigger is the third distinct body, **not its size**", so a body merely
*present* in the dispatch is what has to be priced.

### The symbol Q-123 is about

| | `unl4base` | `nt6board` | `nt6irq` |
| --- | ---: | ---: | ---: |
| `Ppu2C02::read_nametable_inner`, static | **80** | **80** | **80** |
| `Ppu2C02::read_nametable_inner` on `m4`, 300 frames | **113 426 776** | **113 426 776** | **113 426 776** |
| `Ppu2C02::read_pattern_planes` on `m4` | 48 633 600 | 48 633 600 | 48 633 600 |

**Identical to the instruction, statically and dynamically, in all three.** `read_pattern_planes`
is identical on all thirteen ROMs of the set in all three as well. MF-5a's 132 → 109 was a property
of a tree where that function still held the `dispatch!` folded in behind its
`page == SERVED_BY_CARTRIDGE` test; here the dispatch leaves by a tail call and the function is 80
instructions whatever the arm count.

### PR-1's thirteen ROMs, 300 frames, no pack

| | set total | % | worst ROM |
| --- | ---: | ---: | --- |
| `nt6board` vs `unl4base` | **+59 925** | **+0.000** | `m1` +26 341 (**+0.001 %**), `oamstr` +0.000 % |
| `nt6irq` vs `unl4base` | **+243 250** | **+0.000** | `oamstr` +50 496 (**+0.002 %**), `m0` +0.001 % |

`readbuf` — the CNROM whose whole run is fetches, and the ROM MF-6c step 4 saw move by −78 628 —
is **−49 190** for `nt6board` and **−36 210** for `nt6irq`. Against the owner's +0.02 % per ROM,
the worst figure of either shape is a tenth of the tolerance.

### Level 2's five games, 600 frames, packs loaded, inputs pressed — `nt6irq` against the base

| game | `core` delta | `core` % | `total` delta | `total` % |
| --- | ---: | ---: | ---: | ---: |
| `smb1` | **+0** | +0.000 | −2 258 325 | −0.017 |
| `smb3` | **+0** | +0.000 | −4 117 632 | −0.017 |
| `zelda2` | **+0** | +0.000 | −590 043 | −0.004 |
| `metroid` | **+0** | +0.000 | −82 536 | −0.001 |
| `mm2` | **+0** | +0.000 | +131 986 | **+0.001** |

**All five `core` figures are identical to the instruction**, `apu`, `ppu` and `draw_list` on every
game too, and the only `total` above the base is `mm2` at +0.001 %, whose whole move is
`convert` +115 746 and `lua_hooks` +115 746 — the same number twice, which is the Lua column the
performance rule names and MF-UNL-3 step 7 priced at ±0.022 % between two binaries with identical
emulation.

### The decision

**The board serves its own nametables, and the body goes inside `IrqBoard::read_nametable`.** No
park, no second feature for the mechanism — the shape mapper 68 needed `board-68-chr-nametables`
for costs nothing here, because mapper 68 already bought the arm in `Board`'s dispatch and this
board shares it. `rust/docs/PERFORMANCE-HISTORY.md` §5.18 is the write-up, with the rule for the
next lot: put the body inside a shared variant's existing forward.

Disk: G: stayed above 31 GiB free; each of the three build directories was deleted by
`build-clean.sh` as soon as its binary was copied out, and the three tars afterwards. Every run in
the foreground with its output read, as the headless rule requires.

---

## Step 2 — the register map, the banking modes, and the four numbers

`rust/crates/zax-nes/src/cartridges/mappers/jy_company.rs`: one struct, the whole register map,
and the four `registry.rs` arms. The IRQ registers ($C000-$C007) and the nametable registers
($B000-$B007) are **decoded and kept** from this step so that the save-state block is the shape it
will stay; what they *drive* is steps 3 and 5.

### The windows, and their masks

| window | mask | decoded here |
| --- | --- | --- |
| `$5000`, `$5400`, `$5C00` read | `$FFFF` | the solder pad, answering 0 |
| `$5800`-`$5801` | `$F803` | the unsigned 8×8→16 multiplier |
| `$5802`-`$5803` | `$F803` | the accumulator and the test register |
| `$8000`-`$8003` | `$F803` | the four PRG registers, seven bits each |
| `$9000`-`$9007` / `$A000`-`$A007` | `$F807` | the eight CHR registers, LSB and MSB |
| `$B000`-`$B003` / `$B004`-`$B007` | `$F807` | the four nametable registers, LSB and MSB |
| `$C000`-`$C007` | `$F007` | the IRQ — the one window that **accepts** A11 |
| `$D000`-`$D003` | `$F803` | the modes, the arrangement, the PPU configuration, the outer bank |

**The one place the page's two masks overlap is `$5C00`**, which is the solder pad under `$FFFF`
and the product's low byte under `$F803`. The page settles it in its own sentence — "Apparently,
the address of this register is fully decoded, as the SC-128 version of *Final Fight 3*, which
otherwise happily randomizes the masked-off address bits, keeps A0-A11 constantly at zero to read
from this register" — so the three fully decoded addresses are tested first.
`req_core_100_jy_asic_solder_pad_register_is_fully_decoded` pins it, and records that Mesen 2
decodes `$5C00` as the product.

### Where the registers are read, and the dispatch that is *not* used

`$5000`-`$5FFF` and the `$6000`-`$7FFF` ROM window are answered in **`Mapper::try_read_prg`**,
which `IrqBoard` already forwards (it was `board-mmc3-scramble`'s doing, for 121's protection
array), and **not** in `Mapper::cpu_read_override`, which `IrqBoard` does not forward and which is
`#[inline(always)]` on every CPU read of an MMC5 game. `Cartridge::default_cpu_read` reaches
`try_read_prg` for every address of `$4020`-`$FFFF` that is not readable PRG-RAM, which is exactly
the three windows this board answers — so the whole read decode costs `Board`'s dispatches nothing,
and the board has no read side effect to need the other method for.

### The bank arithmetic, and the four places it leaves Mesen 2

The page's outer-bank rule is one sentence per memory — "The PRG-ROM bank selected via
$8000-$8003 is always masked to the outer bank size (512/256/128 KiB, depending on PCB/mapper
number). Accessing more than that requires manipulating the Outer Bank register at $D003" — and
`$D003`'s `4.Lc cPPC` is that register. For these four numbers the outer PRG bank is 512 KiB, so a
PRG register is six bits wide and `PP` is PRG A19-A20; a CHR register is masked to 256 or 512 1 KiB
banks depending on `L`, with `C` as CHR A18 and `cc` as CHR A19-A20.

Four readings of that arithmetic differ from Mesen 2's `JyCompany.h`, and **Q-150** records all
four:

1. **PRG mode 3's bit reversal keeps bit 3.** "bits 0-6 reversed" maps bit 3 to itself; Mesen's
   `InvertPrgBits` writes the other six and drops it, so `$08` selects bank 0 there and bank 8
   here. `q_150_jy_asic_prg_mode_3_reverses_bit_3_onto_itself` walks all seven bits.
2. **32 KiB mode's register is a 32 KiB bank.** The page says "$8003: Select 32 KiB PRG-ROM bank
   at CPU $8000-$FFFF" and gives the `$6000` window "$8003 SHL 2 OR 3" — the same bank's last
   8 KiB page. Mesen maps `SelectPrgPage4x(0, prgRegs[3])`, four *consecutive 8 KiB* pages from the
   register, while using `prgRegs[3] * 4 + 3` for `$6000`: the two cannot both be right, and the
   page's reading is the one they agree on.
3. **`L=1` still masks, and still uses `cc`.** "1: Mask $900x/$A00x to 512 KiB, ignore C" leaves
   `cc` as CHR A19-A20; Mesen drops the mask and the outer bank entirely in that case and uses the
   raw sixteen-bit `$900x | $A00x << 8`.
4. **`$D003` bit 7 is the MMC4-like mode and nothing else.** The page: "only meaningful in 4 KiB
   CHR-ROM banking mode". Mesen reads the same bit as a `_mirrorChr` that folds CHR registers 2 and
   3 onto 0 and 1 in modes 2 and 3, which no line of the page describes.

### The four numbers

`registry.rs` gives 35, 90, 209 and 211 one arm. 209, 211 and 35 build the **same** board — the
page calls 211 "a mere duplicate of INES Mapper 209 with correct emulation" (**Q-149**) and 35 "an
exact duplicate" whose only difference is the 8 KiB of WRAM it states (**Q-148**) — and 90 is that
board with `jumper_inhibits_nametables` set, which step 5 is where it shows. 35's whole
implementation is its `board_memory_sizes` row.

`nametable_pages_vary` gains all four (step 5 is what makes them vary), and both PPU latch lists
and `clocks_on_cpu_cycle` gain them too (step 3). The numbers are in those `matches!` whatever
`board-jy-asic` says, for mapper 68's reason: one `matches!` is one `matches!`, and a number whose
board is off is never created.

### Tests from the page

`req_core_100_jy_asic_multiplies_two_bytes_into_a_sixteen_bit_product` (table-driven, as the census
asks, with `$FF × $FF = $FE01` for the unsignedness), `…_accumulates_and_keeps_a_test_register`,
`…_solder_pad_register_is_fully_decoded`, `…_has_four_prg_banking_modes`,
`…_fixes_the_last_window_to_the_outer_banks_last`, `…_masks_the_prg_bank_to_the_outer_bank`,
`…_has_four_chr_banking_modes`, `…_outer_chr_bank_picks_its_own_mask`,
`…_ignores_a_write_with_address_bit_11_set`, `…_maps_a_prg_rom_bank_at_6000_on_demand`,
`…_arrangement_bits_are_this_cores_mirroring_reversed`, `…_chr_write_enable_gates_chr_ram`,
`q_148_jy_asic_mapper_35_is_mapper_209_with_8_kib_of_wram`,
`q_150_jy_asic_prg_mode_3_reverses_bit_3_onto_itself` and
`req_core_090_jy_asic_round_trips_its_whole_register_set`.

**One test needed a new instrument.** The numbered-bank fixture fills 1 KiB CHR bank `b` with
`b as u8`, so banks 0 and 256 read the same byte — and this board's outer CHR bank reaches bank
512. `chr_banks()` reads the raw window offsets off `Cartridge::chr_peek_view().offsets()`, which
is the only way to see them, and `…_outer_chr_bank_picks_its_own_mask` is the test that needs it.

**`$D002` bit 6 is decoded, and it is write-protected at power-on.** The page gives bit 6 a CHR
write-enable meaning and says "Bit 6 only matters when using CHR-RAM"; every register is zero at
power-on, so a CHR-RAM image of one of these numbers has to ask for the RAM before it can fill it.
Mesen 2 does not decode the bit at all. It is not a Q ticket because the page is not ambiguous — it
is recorded here, and in `…_chr_write_enable_gates_chr_ram`, because it is a power-on default a
future reader will want to find deliberate.

`cargo test --release -p zax-nes`: **924 passed / 0 failed / 8 ignored**. `cargo fmt --check` and
`cargo clippy -p zax-nes --all-targets -- -D warnings` clean.

---

## Step 3 — the IRQ: one prescaler, one counter, three clocks, and four readings Mesen 2 does not share

The page's own three steps are the whole counter:

1. "When **counting is enabled** and the counter is clocked from the source selected by $C001
   bits 0-1, the prescaler is increased/decreased (depending on $C001 bits 6-7)."
2. "If the AND-masked prescaler (according to $C001 bit 2) wraps, meaning it is zero after
   increasing or **matches the prescaler mask after decreasing**, the IRQ counter is
   increased/decreased."
3. "If the IRQ counter wraps, meaning it is zero after increasing or $FF after decreasing, an IRQ
   is generated. Disabling the IRQ via registers $C000 or $C002 acknowledges it."

plus the gloss under the register list: "'Disabling' acknowledges a pending IRQ, **inhibits
counting**, and **resets the prescaler to zero**. 'Enabling' means that counting is no longer
inhibited."

### Where each source lands

| `$C001` bits 1-0 | the page | this core |
| ---: | --- | --- |
| **0** | CPU M2 rise | `Mapper::on_cpu_cycle`, behind `clocks_on_cpu_cycle` |
| **1** | PPU A12 rise, **unfiltered**, eight per scanline | `Mapper::on_ppu_address_bus`, a raw edge against `last_ppu_address` |
| **2** | PPU reads, 170 per scanline | the same hook, every call |
| **3** | CPU writes | **nothing — Q-151** |

**Unfiltered is the one word that keeps this board away from
[`A12Watcher`](crate::cartridges::A12Watcher).** The MMC3's counter needs that filter to tell a
real rise from the two-dot dips of the fetch pipeline and so counts one rise a scanline; this one
counts all eight. `req_core_114_jy_asic_a12_counting_is_unfiltered` is the TC0690's own filter test
in this file with the opposite expectation — seven rises with two-dot dips leave it alone and the
eighth fires it.

**`last_ppu_address` is kept whatever the source is**, so that a game selecting source 1 mid-frame
does not read a stale edge. That one byte is what makes all four numbers appear in
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`'s expected list, and the four
numbers are in both PPU latch lists (A12 is driven by `$2006`/`$2007` as much as by a fetch) and in
`clocks_on_cpu_cycle`.

**`ppu_irq_horizon` is `None`**, and not by this board's choice: `IrqBoard` answers `None` for the
whole variant and does not forward the method, because MF-6b step 4 measured any forward at
`NesBus::catch_up_ppu` 188 → 190 instructions and `mmc5` +9 214 379 over 300 frames. It is also the
honest answer for two of the four sources, which *are* the PPU's bus.
`jy_asic_ppu_irq_horizon_is_none_for_every_source` pins the property the census asks for over all
sixteen combinations of source and direction.

### Q-151: four readings of the counter that leave Mesen 2

| | Mesen 2's `JyCompany.h` | the page | the test |
| --- | --- | --- | --- |
| **counting while disabled** | the prescaler and counter advance; `_irqEnabled` is tested only before asserting | "'Disabling' … **inhibits counting**" | `q_151_jy_asic_disabled_counting_does_not_advance_the_counter` |
| **the prescaler on disable** | left where it was | "**resets the prescaler to zero**" | `q_151_jy_asic_disabling_the_irq_resets_the_prescaler_to_zero` |
| **the prescaler's wrap going down** | `if(--prescaler == 0)` — a wrap off a prescaler of **one** | "**matches the prescaler mask** after decreasing" — a wrap off **zero** | `q_151_jy_asic_prescaler_wraps_when_it_reaches_the_mask_going_down` |
| **IRQ source 3, "CPU writes"** | `_console->GetCpu()->IsCpuWrite()` | the source exists | **no hook in this core**: `q_151_jy_asic_irq_source_3_selects_a_counter_that_does_not_advance` |

The fourth is this core's gap and not a reading of the page. `Mapper::on_cpu_cycle` is told that a
CPU cycle elapsed and not whether it was a write, and a CPU write to RAM never reaches the
cartridge at all — so a hook would mean a new parameter on a per-cycle signature that every board
in the library shares, which the lot's performance rule forbids. The register is decoded and kept,
so a save state carries the game's own selection and a lot that adds the hook has one test to come
and rewrite. It is the same kind of gap as MR-30-4's UNROM 512 flash and MF-6c step 3's Datach
barcode scanner.

### Tests from the page

`req_core_114_jy_asic_counts_cpu_m2_rises_on_source_0` (one cycle when both values are one short,
512 when the counter is two short, so the prescaler's 256-cycle period is measured and not
assumed), `…_prescaler_mask_is_ff_or_07`, `…_counts_up_down_or_not_at_all` (directions 0 and 3
must not count at all), `…_xors_the_prescaler_and_counter_writes`,
`…_irq_source_selection_picks_one_clock_and_not_the_other` (the census's own test: M2 counts and
A12 does not, and the reverse), `…_a12_counting_is_unfiltered`,
`…_irq_registers_accept_address_bit_11` (the `$F007` mask, the one window of the chip that does),
`jy_asic_ppu_irq_horizon_is_none_for_every_source`, and the four Q-151 tests.

**One test had to leave `mapper_write` behind.**
`q_151_jy_asic_disabling_the_irq_resets_the_prescaler_to_zero` counts single cycles, and that
helper ticks the bus twice before every write — enough to wrap a prescaler set one short on
purpose. It writes through `write_memory` instead, which is the first test in this file to need
that distinction.

`cargo test --release -p zax-nes`: **936 passed / 0 failed / 8 ignored**. `cargo fmt --check` and
`cargo clippy -p zax-nes --all-targets -- -D warnings` clean — the latter after splitting
`write_register` into three (`write_arithmetic_register`, `write_irq_register`,
`write_config_register`), the IRQ window having taken it past `clippy::too_many_lines`.

---

## Step 4 — the MMC4-like CHR latch, on the hook the page names

"If MMC4-like bankswitching mode is enabled via $D003 bit 7, the latch is set to 0 when the PPU
**reads** from $0FD8-$0FDF/$1FD8-$1FDF and to 1 when the PPU reads from $0FE8-$0FEF/$1FE8-$1FEF",
and in 4 KiB CHR banking with the mode on the four registers are `$9000`/`$9002` for
PPU `$0000`-`$0FFF` and `$9004`/`$9006` for `$1000`-`$1FFF`, each with its `$A00x` MSB.

### Why `on_ppu_address_bus` and not `ppu_read`

The MMC2 and MMC4 of this build put their latch on `Mapper::ppu_read`, and `Board::ppu_read` has an
arm for them. **`IrqBoard` does not override that method** — its arm is `Mapper`'s default, which
calls the forwarded `ppu_peek` — so overriding it would add a distinct body to a dispatch §4.10 of
the inventory prices at "one arm's worth, +16 measured" for **every board of the enum**, eleven of
which have no latch.

`Mapper::on_ppu_address_bus` is already forwarded, already `#[inline(never)]`, and already sees
every PPU read: "the nametable byte, the attribute byte, the two background planes, the two
discarded nametable fetches of dots 338 and 340, and every sprite pattern plane". The page says the
latch moves on a read, and that is the hook for reads. Mesen 2 puts the same latch on the same hook
(`NotifyVramAddressChange`).

The cost of being on that hook is one test — `address >= 0x2000` — which
`req_core_033_jy_asic_a_peek_never_moves_its_latch` pins from the other side: `$2FD8` and `$3FE8`
are nametable and palette space and must not reach the latch.
[`JyCompany::set_latch`] then recomputes the windows **only when the latch moved**, which is
`Mmc2::set_latch`'s own shape and matters here for the same reason: 57 600-odd calls a frame, all
but a handful of them leaving both latches alone.

### The tile emitter's three methods

REQ-CORE-039's contract — read the latch state, walk the line with `advance_chr_latch`, put the
saved value back — needs `chr_latch_state`, `set_chr_latch_state` and `advance_chr_latch`, and
`IrqBoard` forwards all three for this board alone. None of them is on a rendering path: they run
once per captured line, and only when a 3D pack asked for records.

**`advance_chr_latch`'s body is `Mmc2`'s**, and the reason is arithmetic rather than imitation:
`$0FD8`-`$0FDF` is tile `$FD`'s **second bit plane, all eight rows**, and `$0FE8`-`$0FEF` is tile
`$FE`'s — so the page's two address ranges *are* the MMC4's two tile numbers.
`req_core_039_jy_asic_latch_range_is_the_whole_row_of_the_tile` walks all eight addresses of a
range and five addresses outside them.

### One more place the page beats Mesen 2 (Q-150)

Mesen 2 gates its latch on **`_romInfo.MapperID == 209`** and never looks at `$D003` bit 7. The
page gates it on the register, for every number of the chip — and the one title it names as using
the mode, *Mighty Morphin' Power Rangers III* ("uses MMC4-like mode for displaying title screen"),
is listed under **mapper 209**, while the mode is as available to 90, 211 and 35.
`q_150_jy_asic_mmc4_mode_is_d003_bit_7_on_every_number` runs all four.

**Both latches move whatever the CHR banking mode is**, and only mode 1 reads them. "Only
meaningful in 4 KiB CHR-ROM banking mode" says where the latch *matters*, not where it moves, and a
latch frozen by a mode change would be a second piece of state the page does not describe.

### The save state

The two latches are in the block as bytes, and `load_registers` **refuses** a block whose latch is
2 or more rather than masking it: `chr_4k_register` indexes the eight CHR registers through the
latch, so a corrupt value would index out of them. The MMC2 gets that check for free from
`StateReader::read_index`; this board keeps the pair compact and does it by hand.

### Tests from the page

`req_core_039_jy_asic_mmc4_latch_picks_one_of_two_banks_per_pattern_table` (the census's own test:
a fetch of the latch tile switches the bank, and the latch is per pattern table),
`…_latch_range_is_the_whole_row_of_the_tile`, `q_150_jy_asic_mmc4_mode_is_d003_bit_7_on_every_number`,
`req_core_033_jy_asic_a_peek_never_moves_its_latch` and
`req_core_039_jy_asic_advance_chr_latch_walks_the_line_without_moving_the_board`.

`cargo test --release -p zax-nes`: **941 passed / 0 failed / 8 ignored**. `cargo fmt --check` and
`cargo clippy -p zax-nes --all-targets -- -D warnings` clean.

---

## Step 5 — the nametables, and mapper 90's jumper

Four `$B00x` registers, two mechanisms and a jumper that cancels both:

| mechanism | the register | what it answers | §4.5's level |
| --- | --- | --- | --- |
| **Extended Mirroring** | `$D001` bit 3 | a CIRAM bank per table, from `$B00x` bit 0 | the **cheap** one: `nametable_page` |
| **ROM nametables, for all** | `$D000` bits 5 and 6 | `SERVED_BY_CARTRIDGE` for all four | the **dear** one: `read_nametable` |
| **ROM nametables, selected** | `$D000` bit 5, bit 6 clear, `$D002` bit 7 | per table: ROM when `$B00x` bit 7 **differs** from `$D002` bit 7, and a CIRAM bank from bit 0 otherwise | both at once |
| **mapper 90's jumper** | the PCB | neither: "their function is supressed" | — |

**This is the only board in the build that reaches both of §4.5's levels from the same four
registers**, which is why all four numbers are in `nametable_pages_vary`: `Cartridge` re-asks the
four pages after every CPU write to the cartridge, and on this board a `$B000` or `$D000` write can
move a table from CIRAM page 0 to a CHR-ROM bank and back.

### The precedence the page does not state, and the derivation that settles it

Nothing says what happens when `$D000` bit 5 and `$D001` bit 3 are **both** set. The page's own
last line of the `$B00x` section is the answer: "enabling ROM nametables via $D005 bit 5 and all
$B00x registers having bit 7 match $D002 bit 7 effectively duplicates the functionality of Extended
Mirroring". With bit 5 set and bit 6 clear the per-table decode *already contains* Extended
Mirroring's own answer — `$B00x` bit 0 as the CIRAM bank — so the two orders agree there; and where
bit 6 is also set, "ROM nametables enabled globally for all nametables" leaves nothing to choose.
ROM nametables are therefore tested first. A derivation and not a guess, so no ticket.

### Q-149: 211 and 35 are 209, and Extended Mirroring is `$D001` bit 3

Mesen 2 has **no decode of `$D001` bit 3 at all**: its `_advancedNtControl` is `$D000` bit 5, and
it uses that one bit for both ROM nametables and the per-table CIRAM select — then special-cases
211 as `(_advancedNtControl || MapperID == 211) && MapperID != 90`, so 211 behaves "as though N
were always set". The page calls that reading its own history: "the mapper was defined when the
function of register $D001 bit 3 was not known, and is a mere duplicate of INES Mapper 209 **with
correct emulation**". With the register decoded there is nothing left for the special case to do.

`q_149_jy_asic_mapper_211_is_mapper_209_and_not_n_always_set` is the test, and its first assertion
is the one that matters: all three of 211, 35 and 209 **power on with their nametable registers
idle**, where Mesen 2's 211 is already answering a CIRAM bank per table from four registers nobody
has written.

### Q-152: a write to a ROM nametable

Mesen 2's comment on the mechanism is "This behavior only affects reads, not writes", citing a
nesdev forum thread: the CIRAM byte behind a ROM nametable is still written and still there. **This
core cannot express that at one address.** `Mapper::nametable_page` answers once for both
directions, `SERVED_BY_CARTRIDGE` sends the write to the board, and the board has no access to
console VRAM. So the write is dropped — `Mapper::write_nametable`'s default, which is why
`IrqBoard` needs no forward for it — and **Q-152** records the gap with
`q_152_jy_asic_a_write_to_a_rom_nametable_is_dropped` pinning the behaviour. It is the same kind of
gap as Q-151's IRQ source 3: a hardware behaviour this core's shape cannot carry, not a reading of
the page.

### Where the body went, and what it cost

`JyCompany::read_nametable` sits inside the `IrqBoard::read_nametable` forward **mapper 68 already
pays for**, so `Board::read_nametable` does not grow an arm — the `#[cfg]` on that forward gains
`board-jy-asic` and nothing else changes. Step 1 measured the alternative: a sixth `Board` arm body
and a sixth body inside the forward both left `Ppu2C02::read_nametable_inner` at **80 instructions,
identical to the instruction**, so neither shape costs anything and the cheaper one is free by
construction. That forward's own doc comment now carries the measurement, for the lot that reads it
next.

The bank is the **whole sixteen bits** of the `$B00x`/`$B04x` pair, bit 7 of the LSB included: the
page gives that bit the ROM/CIRAM select *and* leaves it in the bank number, and states no
outer-bank mask for the nametable pair — its "always masked to the outer bank size" sentence is
about `$900x`/`$A00x`. Mesen 2 reads it the same way.

### Tests from the page

`q_149_jy_asic_extended_mirroring_is_d001_bit_3`,
`req_core_100_jy_asic_serves_all_four_nametables_from_chr_rom` (the census's own test: a
`$2000`-`$2FFF` read returns CHR-ROM and not CIRAM),
`…_selects_rom_nametables_table_by_table` (the `$D002` bit 7 inversion, both ways round),
`…_nametable_bank_is_the_whole_register_pair`, `…_mapper_90_inhibits_both_nametable_mechanisms`
(and that 90 keeps every *other* register of the chip),
`q_149_jy_asic_mapper_211_is_mapper_209_and_not_n_always_set` and
`q_152_jy_asic_a_write_to_a_rom_nametable_is_dropped`.

`cargo test --release -p zax-nes`: **948 passed / 0 failed / 8 ignored**. `cargo fmt --check` and
`cargo clippy -p zax-nes --all-targets -- -D warnings` clean.

---

## Step 6 — five question tickets, the four sweeps, and the four configurations

### The tickets

| ticket | what it records | where the code points at it |
| --- | --- | --- |
| **Q-148** | mapper 35 is 209's board with 8 KiB of WRAM, where Mesen 2 builds a wholly different and much smaller board for it | `registry::create_board`'s one arm for the four numbers, and mapper 35's `board_memory_sizes` row |
| **Q-149** | Extended Mirroring is `$D001` bit 3, so **mapper 211 is mapper 209** — Mesen 2 decodes that bit nowhere and special-cases 211 instead | `JyCompany::nametable_page_of`, the `$D001` arm of `write_config_register` |
| **Q-150** | **five** places the bank arithmetic differs from Mesen 2's `JyCompany.h` | `prg_register`, `prg_bank`, `prg_last_bank`, `chr_bank`, `chr_4k_register`, `move_chr_latch` |
| **Q-151** | **three** readings of the IRQ page Mesen 2 does not share, and **IRQ source 3, which this core has no hook for** | `tick_irq`, `disable_irq`, `on_cpu_cycle` |
| **Q-152** | a write to a cartridge-served nametable: `SERVED_BY_CARTRIDGE` answers for both directions, and this mechanism needs it to answer for one | `JyCompany::read_nametable`, and the absence of a `write_nametable` beside it |

**Four of the five are "the number's own page wins"**, which is MF-UNL-2's rule and this lot's
brief. The two that are not readings of the page are Q-151's fourth item and Q-152: both are gaps
in what this core's *shape* can carry — a per-CPU-cycle hook that does not say whether the cycle
was a write, and one nametable answer for two directions — recorded rather than silently dropped,
in the manner of MR-30-4's UNROM 512 flash and MF-6c step 3's Datach barcode scanner.

In four of Q-150's five items **the page is internally consistent and Mesen 2 is not**, which is
what makes them transcription findings rather than judgement calls. The sharpest is item 2: Mesen 2
reads `$8003` as a 32 KiB bank for the `$6000` window (`prgRegs[3] * 4 + 3`, the page's formula)
and as an 8 KiB bank for `$8000`-`$FFFF` (`SelectPrgPage4x(0, prgRegs[3])`) in the same build.

### §7.1's four sweeps

`req_core_100_every_required_mapper_is_registered`,
`req_core_102_any_other_mapper_is_refused`,
`req_core_090_the_mapper_state_survives_a_save_and_restore` and
`amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` hold in all four
configurations. The first is the one this lot had to extend: its `additions` table grew from 84 to
88 rows, and it is built from `cfg!` so that a featured number is expected exactly when its feature
is on.

Three more sweeps are this board's own entry conditions and all three had to be told about it:

* **`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`** — the board keeps
  `last_ppu_address` whatever its IRQ source is, so all four numbers move on the bus and all four
  are in that test's expected list. The check is the invariant and not the list, so forgetting the
  latch would have failed loudly.
* **`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched`** — the board must *not* move on a
  bare CPU cycle, and does not: `$C001`'s power-on direction is 0, "Counting disabled", so
  `tick_irq` returns before touching a field. The four numbers are in `clocks_on_cpu_cycle` all the
  same, because source 0 is the power-on source.
* **`mf_4c_the_write_latch_is_the_read_latch_minus_the_boards_that_wait_on_rd`** — all four
  numbers are in **both** PPU latch lists, A12 being driven by `$2006` and `$2007` as much as by a
  fetch.

### The four configurations

| configuration | `cargo test --release -p zax-nes` | clippy `-D warnings` |
| --- | --- | :-: |
| `default` | **948 / 0 / 8** (941 at step 4, 909 at the base commit) | clean |
| `--no-default-features` | **783 / 0 / 8** — the figure `BOARD-FEATURES.md` records, **unchanged** | clean |
| `--features parked-boards` | **1001 / 0 / 8** (962 at the base commit) | clean |
| `--no-default-features --features board-jy-asic` | **822 / 0 / 8** (+39 over the bare build) | clean |

`cargo fmt --all -- --check` clean, and `cargo test --release -p zax-harness` passes.

**The feature alone is +39 and `default` is +39 over the base commit too**, which is the shape to
expect: no test of this lot is shared with another feature, the board being a family of its own.

**Clippy found two things in `default` and nothing the matrix added.** Both were in this board's own
code: `clippy::if_same_then_else` on the two IRQ-source arms of `on_ppu_address_bus`, which became
one `match` yielding a `bool`; and `clippy::too_many_lines` on `write_register` at 102 lines, which
became three methods (`write_arithmetic_register`, `write_irq_register`, `write_config_register`).
Unlike MF-UNL-3, whose degenerate `board-143`-alone configuration found three real faults, this
lot's one-feature build found none — a single board with a single feature has no shared-variant
`cfg` list to get wrong.

### The two enums' sizes

| | |
| --- | --- |
| `size_of::<Board>()` | **1376**, the figure MF-UNL-2 step 2 and MF-UNL-3 step 5 both measured — **no new discriminant** |
| `size_of::<IrqBoard>()` | **208**, unchanged |
| `size_of::<JyCompany>()` | **200** |

**That 200 is why the variant is boxed**, and it is closer than the plan assumed: unboxed, a
`JyCompany` plus a discriminant would be **exactly** this enum's current 208 bytes, so whether it
set the high-water mark would come down to how rustc places the tag. MF-6b step 1 priced one byte
of displacement in that payload at **+0.009 % to +0.027 % on eleven of lot PR-1's thirteen ROMs**,
which is a coin the lot has no reason to toss: boxed, the enum is provably the size MF-6a3 step 1
measured, and the price is one pointer dereference per forwarded call for a cartridge of this board
and nothing for anyone else.

---

## Step 7 — the callgrind campaign per hook, and the one hook that cost

§7.2 step 6 asked for "the PR-1 + level-2 measurement **per hook added**, not just at the end — the
lesson of MF-6a3 step 1, where two `Board` discriminants cost `mmc5` +0.080 %". **That instruction
is what found this lot's regression**, and a campaign run only at the end would have found it too
but not told anyone which of four hooks to blame.

Ten binaries, each built by `build-clean.sh` from its own tar into a **fresh** directory with its
own `CARGO_TARGET_DIR`, which is then deleted. Roots: `ZAX_TEST_ROMS=…/assets-free/test-roms`,
`ZAX_COMM_ROMS=…/demo/roms`, `ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…` (the 101 profiles).

| label | tree |
| --- | --- |
| `unl4base` | **`16a30564`**, the lot's base commit |
| `unl4h2` | `894127f9`, step 2 — the board, the register map, the banking modes, **no hook** |
| `unl4h3` | `cba4c3d9`, step 3 — **+ the three IRQ hooks** (`on_cpu_cycle`, `on_ppu_address_bus`, `irq_asserted`) |
| `unl4h4` | `ed17e94d`, step 4 — **+ the three CHR-latch forwards** |
| `unl4` | `02fdf8e4`, step 6 — **+ the nametables**, the lot as first written |
| `unl4va` | the same with `#[inline(never)]` on the three latch forwards |
| `unl4vd` | the same with a `chr_latch` **latch on `Cartridge`**, §4.10's own mechanism |
| `unl4vc` | the same with the three latch forwards **removed** |
| `unl4fix` | **what ships**: `unl4vc` plus the mechanism behind `board-jy-asic-tile-latch` |
| `unl4off` | the shipped source with `board-jy-asic` out of `default` — the control |

### The per-hook table: PR-1's thirteen ROMs, 300 frames, no pack, each against `16a30564`

| stage | set total | % | worst ROM |
| --- | ---: | ---: | --- |
| `unl4h2` — the board and the banking modes | **−521 735** | −0.001 | **none slower** |
| `unl4h3` — **+ the CPU-cycle IRQ, the A12 IRQ and `irq_asserted`** | **−376 639** | −0.000 | **none slower** |
| `unl4h4` — + the three CHR-latch forwards | **+78 746 640** | **+0.099** | `mmc5` **+0.373 %**, `smb3` +0.229 % |
| `unl4` — + the nametables | +78 874 044 | +0.099 | the same |
| **`unl4fix` — what ships** | **−329 825** | **−0.000** | `mm2` / `mmc5` **+0.000 %** (+623, +29 237) |

**Two of the four hooks are below the base commit on all thirteen ROMs, and the third costs nothing
measurable.** `unl4h3` — the board with its whole register map, its four banking modes and *all
three* IRQ hooks — is faster than the base on every ROM of the set, which is what `IrqBoard`
already forwarding them buys. The nametables, the hook §7.2 called "the lot's main risk", are
`unl4h4` → `unl4` **+127 404 on the set total**, worst ROM +0.001 %, which is step 1's measurement
confirmed in the shipped shape.

**The cost is the fourth hook, and it is not a hook at all.** The three CHR-latch methods
`IrqBoard` forwards for the tile emitter — `chr_latch_state`, `set_chr_latch_state`,
`advance_chr_latch` — cost **+0.373 % on `mmc5` and +0.229 % on `smb3`**, on boards that have no
latch. The whole of it is `Ppu2C02::run_dots` (+42 073 718 on `mmc5`, +18 180 420 on `smb3`) and
`Ppu2C02::emit_sprite`, in spill churn: `Ppu2C02::end_capture_line` reads the latch state once per
captured line, `run_dots` inlines it, and that function is **57 % to 62 % of `NesCore::run_frame`**
(PERFORMANCE-HISTORY §5.17). It is MF-6a step 2's finding about `Board::chr_view` one dispatch
along.

### The level-2 instrument says the same thing, louder

| build | `core` | `total` |
| --- | --- | --- |
| `unl4h3` | **all five +0, identical to the instruction** | −0.022 % to +0.000 % |
| `unl4` (the three forwards as written) | **all five above**: `smb3` **+0.287 %**, `zelda2` +0.016 %, `smb1` +0.007 %, `mm2` and `metroid` +0.004 % | `smb3` **+0.173 %** |
| **`unl4fix`** | **all five +0, identical to the instruction** | −0.016 % to **+0.003 %** (`smb1`) |
| `unl4off` | all five **+0** | −0.021 % to +0.000 % |

`smb3/core` +0.287 % is the clearest figure this lot produced: the level-2 `core` column is the one
PR-1 and level 2 agree on, and three latch bodies moved it fourteen times the tolerance.

### Four shapes of the mechanism, and the one that ships

| shape | set total | worst ROM | verdict |
| --- | ---: | --- | --- |
| the three forwards as first written | +79 123 279 (+0.099 %) | `mmc5` **+0.374 %** | far over |
| the same three `#[inline(never)]` | +7 771 829 (+0.010 %) | `metroid` **+0.030 %**, five ROMs over | over, and the best of the four |
| a `chr_latch` latch on `Cartridge` | +16 981 759 (+0.021 %) | `mmc5` **+0.326 %** — and **ten ROMs faster**, by up to −0.103 % | over |
| **not forwarded at all** | **−417 513 (−0.001 %)** | **none slower** | keepable |

**The third row is the interesting one and the reason it is written down.** §4.10 of the inventory
is a table of hooks made free by a `bool` on `Cartridge`, and this is the one place that mechanism
was tried and did not work: gating the three calls makes `duelito`, `m0`, `m1`, `metroid`, `mm2`,
`readbuf`, `smb1`, `vblnmi`, `z2` and `oamstr` **measurably faster** — −0.058 % to −0.103 %, which
is a large figure by this project's standards — and `m4`, `mmc5` and `smb3`, the three ROMs whose
boards *do* have a latch or a big one, measurably slower. So the structural fix is worth a lot of
its own, with the MMC2's and the MMC3's figures as its subject, and it is not this lot's to make.

### The decision

| feature | decision | on what |
| --- | :-: | --- |
| `board-jy-asic` | **`default`** | PR-1 **no ROM slower**, worst +0.000 %, set total −329 825; level 2 all five `core` **+0** and all five `total` inside, worst `smb1` +0.003 % |
| `board-jy-asic-tile-latch` | **out of `default`** | the best of four shapes is `metroid` +0.030 % with five of thirteen ROMs over the tolerance; `smb3/core` +0.287 % on level 2 for the first shape |

**The board is not parked.** The owner's rule parks a board above the tolerance, and this board is
below it on both instruments; what is above it is **one mechanism** of the board, and the owner's
rule of 2026-11-06 is written for exactly that — "A costly *mechanism* of a board
(Sunsoft-4's CHR-ROM nametables through `read_nametable` …) is implemented too, behind its own
feature if it costs more than the board itself". `board-68-chr-nametables` is the precedent and
this is the second instance.

**`board-jy-asic-tile-latch` is not in `parked-boards` either**, and `BOARD-FEATURES.md`'s own
sentence is why: "that umbrella is for left-out *boards*, and a build that wants everything asks
for the flags by name". The two PPU features set that precedent; this is the first board mechanism
to follow it.

**What is lost with the feature off, exactly.** Nothing of the emulation: `Mapper`'s three defaults
are `0`, nothing and nothing, so the tile emitter saves 0, moves nothing and restores 0, and the
board's own latch — which only `on_ppu_address_bus` moves — never sees it. REQ-CORE-033's "the
emulation is unchanged whether or not records were built" holds **exactly**, and
`req_core_033_jy_asic_without_the_tile_latch_feature_the_emitter_cannot_move_the_latch` pins it.
What is lost is that a 3D tile **record** of a mapper-90 game carries the bank the latch holds at
capture time rather than the one the hardware would show at that column — which is what every board
but the MMC2 and the MMC4 gets, and **no 3D pack exists for any of the thirteen titles**: §5 of the
census counted that there is no ROM to run either.

### The control

`unl4off` is the shipped source with `board-jy-asic` out of `default`, so no board code is
reachable and its emulation is the base commit's byte for byte: PR-1 set total **+5 326
(+0.000 %)**, worst ROM `oamstr` +0.001 %, and all five level-2 `core` figures **+0**. So the four
numbers are reachable or unreachable for the price of a flag in either direction, which is the
owner's rule.

### The documentation this step carries

* **Two rows in `docs/review/BOARD-FEATURES.md`**, `board-jy-asic` and
  `board-jy-asic-tile-latch`, with this step's figures in both cost columns.
* The `--no-default-features` and `parked-boards` counts in that file are brought to this lot's
  (**783** unchanged and **1001**, against **948** by default), the "how to turn a board off"
  command gains `board-jy-asic`, and the note on the features that are out of `default` gains the
  third one.
* **Two Cargo feature comments**, the second of which carries all four shapes' figures so that a
  reader who flips it finds them beside it.
* `IrqBoard`'s own doc comment on the three methods carries the same four-row table, which is
  where the next lot touching `Board::chr_latch_state` will read it.

### Disk and machine

G: stayed above **29 GiB** free: `rust/target-mf` was at 6.7 GB and was deleted before the first
clean build (its figures being recorded), and each of the nine build directories was deleted by
`build-clean.sh` as soon as its binary was copied out. No GPU campaign, no background command —
every long run in the foreground with its output read, as the headless rule requires.

---

## Step 8 — golden-core against the base commit, and the full check

### The binary that was measured is the binary the commit builds

Step 7's figures come from `unl4fix`, which was built from a `make-tree.sh --worktree` tar before
the step's own commit existed. So the first thing this step did was build `unl4ship` from
**`107c53d9`** by `git archive` into a fresh directory and compare:

    92df5735398b4ba09d9dd19c3c9dc312  zax-harness-unl4fix
    92df5735398b4ba09d9dd19c3c9dc312  zax-harness-unl4ship

**Byte-identical**, so every figure of step 7 is the commit's.

### golden-core over the 98 commercial originals, 1 200 frames each

    zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200

| | |
| --- | --- |
| `16a30564` (`unl4base`) against the lot (`unl4ship`) | **byte-identical** |
| `16a30564` against the lot with `board-jy-asic` out of `default` (`unl4off`) | **byte-identical** |

`md5 f79e7df842c3ee43e7d9b67861a927b5` for all three 98-line dumps — **the same digest MF-UNL-2
step 9 and MF-UNL-3 step 8 both recorded**, so three lots and twenty-five mapper numbers later no
commercial original's pixels, tile records, cycles, audio digest or blargg status has moved.
golden-core runs with no hack code, so there is nothing between the ROM and the core.

### The suite, with the ROM roots set

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **1011 passed, 0 failed, 9 ignored** |
| the same `--no-fail-fast -- --include-ignored`, with `ZAXNES_ROMS=…/demo/roms` and `ZAXNES_TEST_ROMS=…/assets-free/test-roms` | **1020 passed, 0 failed, 0 ignored** |

The nine ignored are the ROM-driven ones, and the second row is the proof they ran rather than
passing vacuously: nine moved out of the ignored column and into the passing one.

### fmt and clippy, in every configuration the lot adds

| configuration | `cargo test --release -p zax-nes` | clippy `-D warnings` |
| --- | --- | :-: |
| `default` | **948 / 0 / 8** | clean |
| `--features board-jy-asic-tile-latch` | **948 / 0 / 8** | clean |
| `--no-default-features` | **783 / 0 / 8** | clean |
| `--features parked-boards` | **1001 / 0 / 8** | clean |
| `--no-default-features --features board-jy-asic` | **822 / 0 / 8** | clean |
| `--no-default-features --features board-jy-asic-tile-latch` | **822 / 0 / 8** | clean |

`cargo fmt --all -- --check` clean. **The two tile-latch configurations have the same test count as
their siblings and not more**, because the two tests swap:
`req_core_033_jy_asic_without_the_tile_latch_feature_the_emitter_cannot_move_the_latch` is compiled
when the feature is **off** and `req_core_039_jy_asic_advance_chr_latch_walks_the_line_without_moving_the_board`
when it is on. Both halves of the contract are therefore tested, and neither configuration is the
one where nobody looks.
