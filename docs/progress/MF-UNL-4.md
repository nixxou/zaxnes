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

- [ ] **0/9 — the plan.** This file: the four numbers, the one page they share, the two `bool`s
  that are the per-number wiring, the four places this lot leaves a prescription for the page, the
  `IrqBoard` variant and why each of the four hooks is free in `Board`'s dispatches.
- [ ] **1/9 — settle the nametable question.** A sixth `Mapper::read_nametable` body measured
  against Q-123's ceiling on this tree, with the figure in `rust/docs/PERFORMANCE-HISTORY.md`. The
  board ships through the existing `IrqBoard` forward if it costs what MF-6c measured, and is
  parked if it does not.
- [ ] **2/9 — `JyCompany`, the register map and the banking modes.** $5000/$5400/$5C00 solder pad,
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
