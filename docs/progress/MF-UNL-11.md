# MF-UNL-11 — the Kǎshèng protection MMC3s: mappers 215, 208 and 219

Lot **MF-UNL-11** of `docs/review/UNL-census.md` §7.2, the eleventh of the twelve MF-UNL lots:
**three numbers, three boards, three Cargo features, 6 original titles.**

Base commit: **`01850f36`** (`MF-UNL-10 (6/6)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

The six titles §7.2 lists, with the publisher each number's own page gives it:

| number | board | publisher the page names | titles |
| ---: | --- | --- | --- |
| 215 | **UNL-8237** (sub 0), **UNL-8237A** (sub 1) | **Sugar Softec** | *Earthworm Jim 2*, *Mortal Kombat 3*, *Mortal Kombat 3 Extra 60*, *Pocahontas Part 2* |
| 208 | not named; the copyright notice is **Gouder**'s | Gouder | *快打傳説 — Street Fighter IV* |
| 219 | **Kǎshèng A9461** | Kǎshèng | *Toy Story* |

**The lot's title names one of the three publishers.** §7.2 calls these "the Kǎshèng protection
MMC3s" and the census's own roster (§2.5 and §6.2) is the accurate one: only **219** is Kǎshèng.
215's page says "later single-game as well as multicart releases from **Sugar Softec**" and 208's
protection is a **Gouder** board's. Recorded because the grouping is by mechanism — an MMC3 plus a
protection register plus an extended banking mode — and not by publisher.

Sources: nesdev, **"INES Mapper 215"**, **"INES Mapper 208"** and **"INES Mapper 219"**, each page
whole, its register diagrams and its two scramble tables / its 256-byte table included.

## Step 0 — the inventory and the plan

### What exists, and what does not

| census item (§7.2 MF-UNL-11) | state at `01850f36` |
| --- | --- |
| step 1 — `board-215`, the $5007 scramble, the outer banks, the NROM override | **missing**; 215 is in no list |
| step 2 — `board-208`, the $4800/$6800 register, the index and the XOR table | **missing**; 208 is in no list |
| step 3 — `board-219`, the extended mode and the $5002/$5003 outer bits | **missing**; 219 is in no list |
| step 4 — §7.1's three common things | to do |

Nothing of the three numbers exists. What **does** exist and is reused unchanged:

* **[`Mmc3`] held rather than reimplemented** — the shape [`Mmc3Scramble`], [`Mmc3Outer`],
  [`Mmc3Txc189`] and [`Mmc3Kof97`] already have: the board owns an `Mmc3`, forwards every register
  it does not decode to it, and lays its own banking over the windows the chip computed. That is
  the lot brief's "reuse the existing MMC3 through parameters without moving its hot path": no
  `Mmc3Board` method body changes and no `const` parameter is added to it.
* **the A12 scanline counter** — all three numbers' IRQ is the MMC3's, so all three answer **yes**
  to `Mapper::irq_asserted` and belong in [`IrqBoard`], which is where every MMC3 clone in this
  build sits. `on_ppu_address_bus`, `try_read_prg`, `prg_ram_enabled` and `prg_ram_writable` are
  all methods that variant **already forwards**, so `Board`'s own dispatches grow nothing.
* **`BankWindows::remap_prg_banks`** — `(bank & mask) | bits` per 8 KiB window, built for mapper
  115's PRG A18 and 121's `$5180`, which is exactly the shape all three of this lot's outer banks
  need. Its **CHR counterpart does not exist** and is this lot's one addition to `BankWindows`
  (`remap_chr_banks`); `or_chr_banks` can only OR, and 215's `$5000` bit 6 and 219's 128 KiB mask
  both have to clear a line before setting it.
* **`Mmc3Board::force_irq_revision_a`**, **`force_ignore_a000`**, **`refresh_banks`**,
  **`bank_register`** — four accessors that exist for earlier boards; this lot widens their `cfg`
  lists and adds no new one except where noted per step.

So the lot adds **three `IrqBoard` variants, no `Board` discriminant**, one `BankWindows` method
and no change to any existing board's code path. MF-CART step 4 is why three variants are
affordable at all: `Cartridge::try_cpu_read`'s `#[inline(always)]` removed the one-instruction
quantum that MF-6c's eleven measurements paid per variant, and "a board added to any shared enum
no longer touches the MMC5's read path at all". The campaign of step 6 is what settles it.

### The three pages, transcribed

#### 215 — `$5000`, `$5001`, `$5007`, all under mask `$F007`

```text
NROM Override/Mode Register ($5000)      Scrambling Pattern Register ($5007)
 D~7654 3210                              D~7654 3210
   MCS. BBBb                                .... .MMM
   |||  ++++- 16 KiB PRG bank at $8000-$BFFF        +++- scramble mode (0-7)
   |||        and $C000-$FFFF
   ||+------- 0: do not replace bit 0 (NROM-128)
   ||         1: replace bit 0 with CPU A14 (NROM-256)
   |+-------- 0: do not replace PRG A17 and CHR A17 from the MMC3 (256 KiB outer bank)
   |          1: replace PRG A17 and CHR A17 with $5001 bits 4 and 5 (128 KiB outer bank)
   +--------- 0: use PRG bank from MMC3; ignore bits 0-3/5
              1: ignore PRG bank from MMC3; apply bits 0-3/5
 Power-up value: $00                      Power-up value: $00

Outer Bank Register ($5001), sub 0       Outer Bank Register ($5001), sub 1
 D~7654 3210                              D~7654 3210
   ..cp CCPP                                ..cp P.PP
     || ||++- 256 KiB outer PRG (A18/A19)         CCC.
     || ++--- 256 KiB outer CHR (A18/A19)      || +|++- 256 KiB outer PRG (A18-A20)
     |+------ 128 KiB outer PRG (A17) if $5000.6  || +++-- 256 KiB outer CHR (A18-A20)
     +------- 128 KiB outer CHR (A17) if $5000.6  |+------ 128 KiB outer PRG (A17) if $5000.6
                                                  +------- 128 KiB outer CHR (A17) if $5000.6
 Power-up value: $xF                      Power-up value: $xF
```

and the two scramble tables, which the page gives as the whole of `$8000-$FFFF`'s behaviour:

```text
 $5007  -------------address written-----------      $5007  -value written-
 value  8000 8001 A000 A001 C000 C001 E000 E001      value  0 1 2 3 4 5 6 7
 0      8000 8001 A000 A001 C000 C001 E000 E001      0      0 1 2 3 4 5 6 7
 1      A001 A000 8000 C000 8001 C001 E000 E001      1      0 2 6 1 7 3 4 5
 2      8000 8001 A000 A001 C000 C001 E000 E001      2      0 5 4 1 7 2 6 3
 3      C001 8000 8001 A000 A001 E001 E000 C000      3      0 6 3 7 5 2 4 1
 4      A001 8001 8000 C000 A000 C001 E000 E001      4      0 2 5 3 6 1 7 4
 5      8000 8001 A000 A001 C000 C001 E000 E001      5      0 1 2 3 4 5 6 7
 6      8000 8001 A000 A001 C000 C001 E000 E001      6      0 1 2 3 4 5 6 7
 7      8000 8001 A000 A001 C000 C001 E000 E001      7      0 1 2 3 4 5 6 7
```

"Bits 6 and 7 of register `$8000` are kept as they are. The data written to `$8001` and
`$A000-$FFFF` is never scrambled." **Five of the eight address rows are the identity and four of
the eight data rows are** — modes 0, 5, 6 and 7 scramble nothing at all, and modes 2 and 4
scramble data without (2) or with (4) an address permutation. §7.2's "eight cases, one per pattern,
which is the only way to prove the table is transcribed right" is therefore still the right test:
what the eight cases prove is as much *which* rows are the identity as what the other rows are.

#### 208 — `$4800`/`$6800`, `$5000-$57FF`, `$5800-$5FFF`

```text
PRG Bank and Mirroring ($4800-$4FFF, $6800-$6FFF, write-only)
 7  bit  0
 ---- ----
 ..MP ...P
   || |
   |+----+- Select 32KB page of PRG-ROM at $8000
   +------- Select mirroring
              0: Vertical
              1: Horizontal
 The initial value of this register is $11.
```

* **Protection Table Index (`$5000-$57FF`, write-only)**: "Specifies the index into the 256-byte
  table that will be used for the next write to `$5800-$5FFF`."
* **Protection Data Access (`$5800-$5FFF`, write)**: "Writes to one of the four protection
  registers determined by the two lower bits of the address being written to. The value that is
  written to the protection register is the CPU data XORed with the table entry indexed by the last
  write to `$5000-$57FF`."
* **Protection Data Access (`$5800-$5FFF`, read)**: "Returns the content of one of the four
  protection registers determined by the two lower bits of the address being read. The correct
  result is checked for by executing code at `$5800`, which in the case of proper emulation of the
  protection scheme will result in the execution of a JMP opcode to a desired address."
* IRQ latch / reload / disable / enable at `$C000-$FFFF`, "identical to the respective MMC3
  registers"; CHR bankswitching is the MMC3's, the *Banks* section listing the chip's own two
  2 KiB and four 1 KiB windows with their `(or …)` swap.
* **Submapper 1** is the copyright-stripped hack: "There are no additional registers … Instead of
  switching PRG ROM banks MMC3-style, there is only one 32k PRG ROM bank at CPU `$8000`. Its value
  is determined by MMC3 bank register 6; the actual bank number is the bank register value SHR 2.
  CHR ROM bankswitching, mirroring and IRQ control are performed using the normal MMC3 registers."

#### 219 — the mode bit at `$8002`, the extended registers, `$5002`/`$5003`

```text
Mode Register ($8002)            Mask: $E003
 CPMR RRRR
 ||++-++++-*bank register number (0-7 in MMC3 mode, $08-$26 in Extended mode)
 ||+------- 0: MMC3 mode   1: Extended mode
 |+--------*1: PRG A14 inversion in MMC3 mode
 +---------*1: CHR A12 inversion in MMC3 mode

Extended PRG-ROM Bank Index ($8000, $23-$26)/Data ($8001)   Mask: $E001, only if $8002.5=1
 $8000/$8002: ..10 .RRR    6: $8000-$9FFF  5: $A000-$BFFF  4: $C000-$DFFF  3: $E000-$FFFF
 $8001:       ..BB BB..    bits 0-3 (reversed order) of the 8 KiB PRG-ROM bank number

CHR-ROM Bank Number Latch Index ($8000, $08-$1E, even)/Data ($8001)
 $8000: ..00 1..0  or  ..01 ...0
 $8001: .... .654    bits 4-6 OR'd with the next CHR-ROM switch's data

CHR-ROM Pseudo 2 KiB switch Index ($8000, $09-$0F, odd)/Data ($8001)
 $8000: ..00 1RR1    RR: 0: PPU $0000-$03FF  1: $0400-$07FF  2: $0800-$0BFF  3: $0C00-$0FFF
                     and bit 1 is also OR'd with bit 0 of the CHR-ROM bank data
 $8001: BBBB BBB.    bits 0-6 of the 1 KiB CHR bank, OR'd with bit 1 SHR 1 of the $8000 value,
                     then OR'd with bits 0-2 SHL 4 of the value written to the CHR-ROM latch

CHR-ROM 1 KiB switch Index ($8000, $11-$1F, odd)/Data ($8001)
 $8000: ..01 RR.1    RR: 0: PPU $1000-$13FF  1: $1400-$17FF  2: $1800-$1BFF  3: $1C00-$1FFF
 $8001: BBBB BBB.    as above, without the $8000 bit-1 term

Outer Bank Bit 0 ($5002)  Mask: $5003    .... ...B   bit 0 of the 128 KiB outer PRG and CHR bank
Outer Bank Bit 1 ($5003)  Mask: $5003    ..B. ....   bit 1 of the 128 KiB outer PRG and CHR bank
 The Outer Bank register must be initialized to $03 for the menu to appear.
```

**The `R RRRR` field and the extended index values are one reading, and it is the only consistent
one.** The field is five bits wide and `$26` needs six, so the sixth is bit 5 — the mode bit
itself. The extended PRG indices `$23-$26` have bit 5 **set** (`..10 .RRR`) and the three CHR
index patterns have it **clear** (`..00 1..0`, `..00 1RR1`, `..01 RR.1`), while all four sections
say "only if `$8002` bit 5=1". The two statements can only both hold if **the mode bit is latched
by a write to `$8002` and not by a write to `$8000`** — `$8002` is the MMC3's own `$8000` port
under mask `$E001` with bit 5 given a meaning, which is also why the page annotates its bits 6 and
7 as "PRG A14 inversion" and "CHR A12 inversion", the MMC3's own `$8000` bits. The board therefore
latches the index from every even address in `$8000-$9FFF` and the mode from `$8002` alone.

### The ambiguities, and the one §7.2 claim the pages do not carry — **Q-171**

MF-UNL-2's rule applies: where §7.2 and a number's own page differ, **the page wins**, each case
recorded. §7.2's three steps here are substantially right — the registers, the mask, the eight
patterns, the XOR table and the extended mode are all the pages' own — and only one of its test
bullets asks for something no page states. The ticket also holds five things the pages leave open:

1. **§7.2's 208 test bullet**: "a wrong code leaves the board in the state the page describes".
   The page describes **no** wrong-code behaviour: a write to `$5800-$5FFF` always stores
   `data ^ table[index]`, so a wrong index simply stores a wrong byte and the game's `JMP` goes
   somewhere else. The test written is the mechanism the page *does* state — the XOR, the four
   registers and the two address bits — plus the complement: a *different* index over the same
   datum gives a different stored byte.
2. **215 submapper 1's `$5001`**, the one genuinely ambiguous diagram in the lot: two bit rows
   overlaid, `P.PP` over `CCC.`, with four register bits (3-0) carrying six outer lines. The
   reading implemented is `PRG A20/A19/A18 = bits 3/1/0` and `CHR A20/A19/A18 = bits 3/2/1`, which
   is the one both annotation lines support (`+|++-` with the `|` passing bit 2 down to the CHR
   row's `+++--`) and which makes the two fields three bits wide as their own text says. The
   alternative — the CHR row aligned one place right, `CHR A20/A19/A18 = bits 2/1/0` — is recorded.
3. **215's `$5001` power-up value `$xF`**: the page fixes the low nibble and leaves the high one
   unknown. Bits 4 and 5 are the two A17 lines, read only while `$5000` bit 6 is set, and `$5000`
   powers on at `$00`; `$0F` is used and the `x` recorded.
4. **219's `Mask: $5003`** taken literally would decode `$D002`/`$D003` as well — A14 and A12 set,
   A15 not in the mask — which are the MMC3's IRQ ports. The window implemented is `$F003`, so the
   two registers are `$5002`/`$5003` and their aliases inside `$5000-$5FFF` only.
5. **208's `$A000`**: the page's register list has no `$A000` section and puts mirroring in the
   `$4800`/`$6800` register, so the chip's own mirroring register is made inert on submapper 0.
   Submapper 1's own text says the opposite in so many words — "mirroring … performed using the
   normal MMC3 registers" — so that submapper keeps it.
6. **208's protection index** is the written **value** and not the address: "specifies the index
   into the 256-byte table", a 256-entry table and an 8-bit datum against a 2 KiB window.

### The nine steps

- [x] **0/8** the plan — this note.
- [x] **1/8** `board-215`: the board, its registry wiring and the eight-pattern tests.
- [ ] **2/8** `board-208`: the board, the 256-byte table and the protection tests.
- [ ] **3/8** `board-219`: the board, the extended mode and its tests.
- [ ] **4/8** §7.1's three common things, part one: the four sweeps over `SUPPORTED_MAPPERS` in the
      six configurations, and a save-state round-trip per board with its registers dirtied.
- [ ] **5/8** `cargo fmt --check` and clippy `-D warnings` in the six configurations: default,
      `--no-default-features`, `--features parked-boards`, and each of the three features alone.
- [ ] **6/8** the callgrind campaign against `01850f36`, the feature decision per board and the
      three `BOARD-FEATURES.md` rows.
- [ ] **7/8** golden-core byte-identical over the 98 originals, and the full
      `--include-ignored --no-fail-fast` suite with both ROM roots.
- [ ] **8/8** Q-171, the census note, the journal, the close.

[PROGRESS] MF-UNL-11 step 0/8 done: the plan

## Step 1 — `board-215`, the UNL-8237 and UNL-8237A

`Mmc3Unl8237` (`rust/crates/zax-nes/src/cartridges/mappers/mmc3_unl8237.rs`): an `Mmc3` held, the
three registers of `$5000-$5FFF` under mask `$F007`, the page's two eight-row scramble tables and
`refresh` = `refresh_banks` → `apply_override` → `apply_outer`, which is `Mmc3Scramble`'s own
idempotence argument. One boxed `IrqBoard` variant, **no `Board` discriminant**.

### What the outer banks are, in bank-number arithmetic

| `$5000` bit 6 | PRG inner mask (8 KiB banks) | CHR inner mask (1 KiB banks) | what `$5001` adds |
| --- | ---: | ---: | --- |
| 0 — "256 KiB outer bank" | `$1F`, A13-A17 | `$FF`, A10-A17 | the 256 KiB bank above it |
| 1 — "128 KiB outer bank" | `$0F`, A13-A16 | `$7F`, A10-A16 | **A17** from `$5001` bits 4 and 5, and the 256 KiB bank above that |

So `apply_outer` is one `(bank & inner) | outer` per window on **both** tables, which is why
`BankWindows::remap_chr_banks` had to exist: `or_chr_banks` cannot clear the line bit 6 replaces.
`remap_prg_banks`'s `cfg` list grows one feature and its body does not move.

**A consequence worth stating, because every expectation in this step's tests turns on it**: the
MMC3's two fixed windows are the last two banks *of the inner region* and not of the chip. On a
512 KiB image with `$5001` = 0 the layout is `[R6, R7, 30, 31]`, not `[…, 62, 63]` — the chip
addresses 256 KiB and `$5001` says which 256 KiB. The MMC3's own power-on `R7` is **1**
(`Mmc3Board::reset`), so the board's power-on layout is `[0, 1, -2, -1]` under the outer bank of 3
that `$5001`'s `$xF` gives it: `[96, 97, 126, 127]` on a 1.5 MiB image.

### The eight tests

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_215_runs_each_of_the_eight_5007_patterns_over_the_whole_bank_table` | §7.2's eight cases: per mode, **all eight** bank registers through the scrambled index/data pair plus the mirroring port — two columns of the address table, all eight data entries, and a third column |
| `…_decodes_5000_5001_and_5007_under_mask_f007` | `$5FF8` is `$5000`; `$5002`-`$5006` are not decoded |
| `…_overrides_the_prg_banking_with_nrom_128_and_nrom_256` | bit 7, bits 0-3, bit 5, and "ignore bits 0-3/5" when bit 7 is clear |
| `…_puts_the_outer_256_kib_banks_above_the_mmc3_on_the_unl_8237` | submapper 0's `CCPP`: four PRG outer banks and four CHR outer banks |
| `…_reads_5001_the_other_way_on_the_unl_8237a` | submapper 1's overlaid diagram: **bit 0 moves PRG only, bit 2 CHR only**, bit 1 both one line apart, bit 3 A20 on both |
| `…_replaces_prg_a17_and_chr_a17_from_5001_under_5000_bit_6` | the same register value read with the line from the chip and from `$5001`, each line on its own |
| `amd_061_mapper_215_resets_the_outer_bank_register_to_its_power_up_value` | the page's *Note* on the M2 interruption |
| `req_core_095_mapper_215_leaves_6000_7fff_to_the_header_and_decodes_nothing_there` | **no `board_memory_sizes` arm**, which is 121/123/187's choice: every register is below `$6000` |

The discriminating pair of the submapper-1 test is what makes it worth writing: under the
alternative reading of that diagram (CHR at bits 2/1/0) `$5001` bit 0 would move the CHR window,
and it does not. 1.5 MiB of each memory is what puts A18, A19 **and** A20 of both halves inside
the image, so all six rows are real reads.

Two lists outside the board grew the number, both caught by `mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched`:
`watches_ppu_reads`, `watches_ppu_bus_without_read` and that test's own expected list — the board
holds an `Mmc3`, counter included, which is mapper 114's reason in the same words.

`cargo test --release -p zax-nes`: **all suites pass**, `mapper_tests` 350 → **358**. fmt clean,
clippy `-D warnings` clean on `zax-nes`.

[PROGRESS] MF-UNL-11 step 1/8 done: board-215
