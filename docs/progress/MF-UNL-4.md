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
- [ ] **3/9 — the IRQ.** One prescaler and one counter, four sources (three of them clocked, see
  Q-151), up/down/off counting, the $C006 XOR register, the $C001 prescaler mask, and the page's
  "disabling acknowledges, inhibits counting and resets the prescaler to zero". `on_cpu_cycle`,
  `on_ppu_address_bus`, `irq_asserted`, and `ppu_irq_horizon` answering `None`.
- [ ] **4/9 — the MMC4-like CHR latch.** $D003 bit 7, the two latches, the four 4 KiB registers
  they pick between, and `advance_chr_latch` for the tile emitter.
- [ ] **5/9 — the nametables, the jumper and the $6000-$7FFF window.** Extended Mirroring
  ($D001 bit 3), ROM nametables for all ($D000 bits 5 and 6) and for some ($D002 bit 7), 90's
  jumper, 35's 8 KiB of WRAM, and $D000 bit 7's 8 KiB PRG-ROM bank at $6000-$7FFF.
- [ ] **6/9 — the question tickets, the four sweeps and the clippy configurations.** Q-148 to
  Q-151, the four sweeps over `SUPPORTED_MAPPERS` in every configuration the lot adds, and
  `cargo fmt --check` plus clippy `-D warnings` in the four configurations the brief names.
- [ ] **7/9 — the callgrind campaign, per hook.** PR-1's thirteen ROMs and level 2's five games
  against `16a30564`, one binary per hook added, every compared commit built from `git archive` in
  a fresh directory. The feature decision and the `BOARD-FEATURES.md` row.
- [ ] **8/9 — golden-core against the base commit, and the full check.** Byte-identical over the
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
