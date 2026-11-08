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
- [x] **2/8** `board-208`: the board, the 256-byte table and the protection tests.
- [x] **3/8** `board-219`: the board, the extended mode and its tests.
- [x] **4/8** §7.1's second common thing: a save-state round-trip per board with its own
      registers dirtied, and a reset per board.
- [x] **5/8** §7.1's first and third common things: the four sweeps, `cargo fmt --check` and
      clippy `-D warnings` in the six configurations.
- [x] **6/8** the callgrind campaign against `01850f36`, the feature decision per board and the
      three `BOARD-FEATURES.md` rows.
- [x] **7/8** golden-core byte-identical over the 98 originals, and the full
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

## Step 2 — `board-208`, the Gouder board and its 256-byte table

`Mmc3Gouder208` (`…/mappers/mmc3_gouder208.rs`): an `Mmc3` held with its PRG bank lines
disconnected — the board banks **32 KiB** at a time, which is `Mmc3Txc189`'s shape — four ports
below `$8000` and the page's `static uint8 lut[256]` as a `const`. One boxed `IrqBoard` variant.

| port | mask | what |
| --- | --- | --- |
| `$4800-$4FFF`, `$6800-$6FFF` w | `$D800` | `..MP ...P` — the 32 KiB page in bits 4 and 0, mirroring in bit 5. Initial value **`$11`** |
| `$5000-$57FF` w | `$F800` | the index into the 256-byte table the next `$5800` write uses |
| `$5800-$5FFF` w | `$F800` | the four protection registers, `datum ^ table[index]`, picked by the two low address bits |
| `$5800-$5FFF` r | `$F800` | the same four registers |

**`$D800` is exactly the page's two windows and nothing else**: A14, A12 and A11 are decoded and
A13 is not, so `$4800` and `$6800` both match while `$5800` and `$7800` — A11 set, A12 clear — do
not. One mask for the pair rather than two tests.

### The three things the page settles that §7.2 does not

1. **`$A000` is inert** (Q-171). The page's register list has no `$A000` section and puts
   "mirroring control" among the board's own additional registers, so `force_ignore_a000` — built
   for mapper 14 — gets its second caller. Submapper 1 says the opposite in its own words
   ("mirroring … performed using the normal MMC3 registers") and so does not call it.
2. **The index is the written value**, not the address: "specifies the index into the 256-byte
   table", a 256-entry table and an 8-bit datum against a 2 KiB window.
3. **The `board_memory_sizes` row** gives submapper 0 `prg_ram: Some(0)`, because `$6800-$6FFF`
   *is* the board's decode of that window — the arm 114, 115 and 182 are already on.
   `Cartridge::cpu_write` returns at `$6000-$7FFF` whenever the cartridge has any PRG-RAM, so
   without the row the register could not be written at all; the register answering there **is**
   the assertion. Submapper 1 is excluded from the arm, having no register there.

### Power-on is in `reset` and not in `new`

"The initial value of this register is `$11`" — page 3, **vertical** mirroring — and a board
cannot set the mirroring from `new`, which takes `&CartridgeMemory`. `Cartridge::create` ends with
`cartridge.reset()`, and `Cartridge::reset` puts the header's mirroring back immediately before
calling the board's, so `Mmc3Gouder208::reset` is the right and only place for it.

### The seven tests

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_208_xors_a_write_against_the_pages_own_table` | **all 256 indices** read back through the mechanism, plus the XOR being a XOR (`$FF` complements, the entry itself stores zero), plus the complement of §7.2's missing bullet: the same datum under a different index stores a different byte |
| `…_keeps_four_protection_registers_under_the_two_low_address_bits` | four registers, and `$5FFC-$5FFF` aliasing `$5800-$5803` |
| `…_banks_32_kib_and_drives_the_mirroring_from_4800_and_6800` | the initial `$11`, bit 4 above bit 0, both windows, the mirroring bit, `$A000` inert, and `$4000`/`$5800`/`$7800`/`$6000` not decoded |
| `…_keeps_the_mmc3s_chr_banking_and_loses_its_prg_banking` | the six CHR windows and their swap; `R6`/`R7` reaching nothing |
| `…_submapper_1_takes_its_one_page_from_the_mmc3s_r6` | `R6 >> 2`, the chip's mirroring back, the board's four ports dead and `$5800` reading open bus |
| `req_core_095_mapper_208_gives_submapper_0_no_prg_ram_and_leaves_submapper_1_the_headers` | the `board_memory_sizes` row from both sides |
| `req_core_114_mapper_208_keeps_the_mmc3s_scanline_counter` | the four IRQ ports "identical to the respective MMC3 registers", on both submappers |

**The table test does not copy the `const`.** It rebuilds the 256 entries from the *structure* the
page's array has — four groups of four rows, one four-byte cycle for a group's even rows and
another for its odd ones, written twice across the live half — so eight four-byte cycles in the
test stand against 256 literals in the board, and a slip in either copy fails. The sixteen first
bytes of the array, `59 59 59 59 00 08 00 08 59 59 59 59 01 09 01 09`, are that structure read
down the left edge.

`cargo test --release -p zax-nes`: all suites pass, `mapper_tests` 358 → **365**. fmt clean,
clippy `-D warnings` clean.

[PROGRESS] MF-UNL-11 step 2/8 done: board-208

## Step 3 — `board-219`, the Kasheng A9461 and its extended mode

`Mmc3A9461` (`…/mappers/mmc3_a9461.rs`): an `Mmc3` held, a second banking mode behind `$8002`
bit 5, and a two-bit 128 KiB outer bank over PRG **and** CHR. One boxed `IrqBoard` variant.

### The reading that makes the page consistent, and it is the only one

The `$8002` diagram's `R RRRR` field is **five** bits and the extended index values run to `$26`,
which needs six — so the sixth is bit 5, the mode bit itself. That alone is a contradiction: the
extended PRG indices `$23-$26` carry bit 5 **set** and all three CHR index patterns
(`..00 1..0`, `..00 1RR1`, `..01 RR.1`) carry it **clear**, while every one of the four sections
says "used only if `$8002` bit 5=1". Both hold only if

* the **mode** is latched by a write to `$8002` and by no other even address, and
* the **index** is latched by every even address of `$8000-$9FFF`.

which is also why the page annotates `$8002` bits 6 and 7 as "PRG A14 inversion" and "CHR A12
inversion": those are the MMC3's own `$8000` bits, so `$8002` *is* the chip's `$8000` port with
bit 5 given a meaning, and an even write always reaches the chip.
`q_171_mapper_219_latches_the_mode_bit_from_8002_and_the_index_from_every_even_address` asserts it
from both sides — a `$8000` write with bit 5 set does not open the mode, and a `$8000` write with
bit 5 clear (which every CHR index is) does not close it. In extended mode the **odd** writes are
this board's own registers and never reach the chip, so clearing the mode bit gives an MMC3 back
with the bank registers it had.

### The extended registers

| index | datum | what |
| --- | --- | --- |
| `..10 .RRR`, `$23-$26` | `..BB BB..` | an 8 KiB PRG bank, its four bits **reversed**, at the window `RRR` names (6 → `$8000` … 3 → `$E000`) |
| `..00 1..0` / `..01 ...0`, `$08-$1E` even | `.... .654` | the CHR bank-number latch: bits 4-6 of every following CHR switch |
| `..00 1RR1`, `$09-$0F` odd | `BBBB BBB.` | a 1 KiB CHR bank in one quarter of PPU `$0000-$0FFF`, **with bit 1 of the index ORed into bit 0 of the bank** |
| `..01 RR.1`, `$11-$1F` odd | `BBBB BBB.` | a 1 KiB CHR bank in one quarter of PPU `$1000-$1FFF` |

**The "pseudo 2 KiB" of the third row's title is that index bit**, and it is the only place in
this board where an address bit reaches a bank number: one bank written to all four quarters comes
out `$0A $0B $0A $0B`, two 2 KiB pairs built from two 1 KiB switches. The test asserts exactly
that.

### `Mask: $5003` is read as `$F003` (Q-171)

Taken literally the page's mask decodes A14, A12, A1 and A0 and **not A15**, so `$D002`/`$D003` —
the chip's own IRQ latch and reload — would be the outer bank register too.
`q_171_mapper_219_decodes_the_outer_bank_inside_5000_5fff_and_not_at_d002` pins the choice from
both ends: `$5FFE`/`$5FFF` are aliases of the pair and do bank, `$D002`/`$D003` do not — and the
same write is then shown to have reached the counter, so the test proves where it went and not
only where it did not.

### The six tests

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_219_banks_four_independent_prg_windows_in_extended_mode` | §7.2's own bullet, both halves; the four banks are **1, 2, 4, 8**, one bit each, which is what pins the reversal — bank 1 is written as `$20` and bank 8 as `$04` |
| `…_is_an_mmc3_masked_to_128_kib_under_its_outer_bank` | the chip's banks masked to A16 and the outer two bits handing A17-A18 back, all four outer banks of **both** halves inside a 512 KiB image |
| `…_switches_chr_through_a_latch_and_two_pseudo_switches` | the pseudo-2 KiB pairing, the four 1 KiB windows of the second table, and the latch reached through both of its index patterns |
| `q_171_…_latches_the_mode_bit_from_8002_and_the_index_from_every_even_address` | the reading above |
| `q_171_…_decodes_the_outer_bank_inside_5000_5fff_and_not_at_d002` | the `$F003` reading |
| `req_core_095_…_leaves_6000_7fff_to_the_header_and_decodes_nothing_there` | **no `board_memory_sizes` arm**, and the other half of the mask question: a literal `$5003` would decode `$7002`/`$7003` and that window could then not be RAM |

### One thing the lot nearly shipped broken

`req_arch_002_the_crate_has_no_platform_dependency` reads `zax-nes/Cargo.toml` as text and
forbids the substring `windows`, among five crate names. This step's feature comment used the word
in its ordinary sense and the test caught it; the comment is reworded. Worth recording because the
check is on the manifest's prose and not on its dependency table.

`cargo test --release -p zax-nes`: all suites pass, `mapper_tests` 365 → **371**. fmt clean,
clippy `-D warnings` clean.

[PROGRESS] MF-UNL-11 step 3/8 done: board-219

## Step 4 — §7.1's second common thing: three round trips and three resets

`req_core_090_the_mapper_state_survives_a_save_and_restore` sweeps every number of
`SUPPORTED_MAPPERS` but dirties each one through `$8000`-`$E001` and `$6000` — and **not one of
this lot's own registers is at any of those addresses**: 215's three are at `$5000`/`$5001`/
`$5007`, 208's four at `$4800`/`$5000`/`$5800`/`$6800`, and 219's two outer bits at
`$5002`/`$5003`. 219's extended registers *are* at `$8000`/`$8001`, which is worse than absent:
while the mode bit is set those addresses mean something else entirely, and the generic sweep
never sets it. So the sweep proves the chip's block round-trips and says nothing about the bytes
this lot appends.

| test | what it moves, and what would otherwise pass |
| --- | --- |
| `req_core_090_mapper_215_round_trips_its_three_registers` | all three registers, with `$5007` on a row that is neither the identity nor row 1. **A block that dropped `$5007` would round-trip the windows and diverge on the next write**, because the same written address would then mean a different MMC3 register — which is why the assertion on the re-saved bytes is the one that matters |
| `req_core_090_mapper_208_round_trips_its_bank_index_and_four_protection_registers` | the page, the mirroring, the index and all four protection registers. **Those four are state no window shows**, so the test reads the `$5800-$5803` window back as well as comparing the block |
| `req_core_090_mapper_219_round_trips_its_extended_mode_registers` | taken **in** extended mode, with sixteen extended bank bytes, the latch and the outer bank set. A block that dropped the mode bit would restore an MMC3's windows from the chip's own registers and look right until the next odd write — so the test ends with one more odd write and asks for an extended register |
| `amd_061_a_reset_puts_every_board_of_mf_unl_11_back_to_its_power_on_state` | all three, each through its own ports |

The reset test is the one that needs the two non-zero power-on states stated: 215's `$5001` is
**`$xF`**, so a reset puts the outer bank back to 3 on both halves, which the page's *Note*
requires; and 208's bank register is **`$11`**, page 3 with **vertical** mirroring, which
`Cartridge::reset` cannot know — it puts the *header's* mirroring back immediately before calling
the board's `reset`, so the board has to state its own.

`unl11_layout` compares the four PRG windows, the eight CHR window **offsets** and the mirroring.
The offsets and not the fixture's fill bytes, for `chr_offsets`'s own reason: all three boards move
a CHR window by 128 or 256 banks and a numbered bank reads the same byte 256 banks apart. That
helper's `cfg` list grows this lot's three features.

`cargo test --release -p zax-nes`: all suites pass, `mapper_tests` 371 → **375**. fmt clean,
clippy `-D warnings` clean.

[PROGRESS] MF-UNL-11 step 4/8 done: the round trips and the resets

## Step 5 — the six configurations

None of the three features implies another, so the matrix is the three alone plus the three
umbrellas. `cargo fmt --check` is clean over the whole workspace.

| configuration | clippy `-D warnings` on `zax-nes --all-targets` | `--test mapper_tests` |
| --- | --- | ---: |
| default | clean | **375 passed, 0 failed** |
| `--no-default-features` | clean | **103 passed, 0 failed** |
| `--features parked-boards` | clean | **375 passed, 0 failed** |
| `--no-default-features --features board-215` | clean | **112 passed, 0 failed** |
| `--no-default-features --features board-208` | clean | **111 passed, 0 failed** |
| `--no-default-features --features board-219` | clean | **110 passed, 0 failed** |

The three single-feature counts against the 103 of the empty build are **9, 8 and 7** — this
lot's own tests, every one of them running with its board's feature as the **only** board feature,
which is what the owner's rule means by reachable behind its feature. 375 − 350 at the base commit
is the lot's 25 tests: 8 + 7 + 6 from the three pages, three round trips and one reset.

§7.1's first common thing, the four sweeps over `SUPPORTED_MAPPERS`, in each of the six:

| sweep | default | none | `parked-boards` | 215 | 208 | 219 |
| --- | --- | --- | --- | --- | --- | --- |
| `req_core_100_every_required_mapper_is_registered` | ok | ok | ok | ok | ok | ok |
| `req_core_102_any_other_mapper_is_refused` | ok | ok | ok | ok | ok | ok |
| `req_core_090_the_mapper_state_survives_a_save_and_restore` | ok | ok | ok | ok | ok | ok |
| `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` | ok | ok | ok | ok | ok | ok |

The `board_memory_sizes` row is asserted from the boards' own tests, as MF-UNL-10 step 3 did:
`req_core_095_mapper_208_gives_submapper_0_no_prg_ram_and_leaves_submapper_1_the_headers` from
both sides of 208's arm, and the two `…_leaves_6000_7fff_to_the_header_…` tests for the two
numbers that deliberately have none.

### What the `board-208`-alone configuration caught

`IrqBoard::try_read_prg` is a **forwarded** method behind its own `cfg(any(…))` list, not one the
variant implies: with `board-208` as the only board feature, that list was off and the protection
read of `$5800-$5FFF` fell through to the trait default, so three of the lot's tests failed with
open bus where a protection register should have been. `board-208` is now the fifth entry in the
list. In `default` the change is a no-op — `board-mmc3-scramble` already turns the method on — so
nothing but that one configuration could have found it, which is the whole point of running it.

`cargo test --release -p zax-nes -p zax-harness`: every suite passes.

[PROGRESS] MF-UNL-11 step 5/8 done: the six configurations

## Step 6 — the campaign, and a lot that costs nothing measurable

Binaries built by `build-clean.sh` from their own `git archive` tar into a **fresh** directory with
its own `CARGO_TARGET_DIR`, each deleted as soon as the binary was copied out. Roots:
`ZAX_TEST_ROMS=…/assets-free/test-roms`, `ZAX_COMM_ROMS=…/demo/roms`,
`ZAX_L2_ROMS=…/test-originals`, `ZAX_L2_PACKS=…/campaign/play-user/profiles`.

| label | tree | md5 |
| --- | --- | --- |
| `mfunl11-0-base` | **`01850f36`**, the lot's base commit | `d4d92c446abb1cafd5194ccbf35d3b1e` |
| `mfunl11-1-lot` | **`2b38776c`**, step 5 — the lot's last code commit | `d031eddea93ab222efc8cbbf61dd5b67` |
| `mfunl11-0-again` | a byte copy of `mfunl11-0-base`, the **same-binary floor** | same md5 as the base |

The base binary's md5 is **identical to MF-UNL-10's shipped binary** (`zax-harness-mfunl10-3-ship`
and `-1-lot` both), so this lot's base is that lot's measured binary to the byte — the **eighth**
lot in a row for which that holds, and built rather than assumed.

### The PR-1 set, thirteen ROMs, 300 frames

| ROM | base | lot | delta | % |
| --- | ---: | ---: | ---: | ---: |
| duelito | 7 380 425 366 | 7 380 400 626 | −24 740 | −0.000 |
| m0 | 5 517 311 068 | 5 517 312 364 | +1 296 | +0.000 |
| m1 | 4 881 118 436 | 4 881 091 726 | −26 710 | −0.001 |
| m4 | 5 468 173 957 | 5 468 148 039 | −25 918 | −0.000 |
| metroid | 5 115 555 558 | 5 115 556 927 | +1 369 | +0.000 |
| mm2 | 6 018 007 634 | 6 017 994 846 | −12 788 | −0.000 |
| mmc5 | 11 266 796 935 | 11 266 786 726 | −10 209 | −0.000 |
| oamstr | 3 318 454 125 | 3 318 464 799 | +10 674 | +0.000 |
| readbuf | 6 834 511 568 | 6 834 482 778 | −28 790 | −0.000 |
| smb1 | 5 996 363 392 | 5 996 344 729 | −18 663 | −0.000 |
| smb3 | 7 949 476 038 | 7 949 410 397 | −65 641 | −0.001 |
| vblnmi | 3 308 285 084 | 3 308 328 828 | **+43 744** | **+0.001** |
| z2 | 6 291 504 063 | 6 291 538 385 | +34 322 | +0.001 |
| **total** | **79 345 983 224** | **79 345 861 170** | **−122 054** | **−0.000** |

Worst ROM **+0.001 %** against a tolerance of **+0.02 %**, seven of thirteen below, and the set
total **below** the base commit's.

### The floor is worse than the figure, and no emulation symbol moves at all

| | worst ROM | set total | emulation symbols that moved |
| --- | ---: | ---: | --- |
| base vs the **lot** | `vblnmi` **+0.001 %** (+43 744) | **−122 054** | **none**, at a 20 000 threshold |
| base vs the **same binary** | `m1` +0.001 % (+50 390), `m4` +50 957 | **+82 759** | **none** |

Every symbol that moved in either run is Lua's `ltable.c` — `getgeneric` and `luaH_getshortstr`,
in both directions, by ±20 000 to ±44 000 — and they move by the same amounts between two runs of
the **same** binary. MF-UNL-8 step 7's finding that `callgrind-set.sh`'s determinism does not hold
for the whole process on this machine is what this is, and unlike MF-UNL-10's figure **this lot's
is inside it**: three new `IrqBoard` variants, three boards holding an `Mmc3`, one new
`BankWindows` method and five widened `cfg` lists cost nothing a 20 000-instruction threshold can
see.

That is also the re-measurement MF-CART step 4 asked for and MF-UNL-2 step 8 promised: three
variants added to a shared `Board` enum at once, and `Cartridge::try_cpu_read` does not move.
`mmc5` — the one ROM whose every CPU read reached that function before step 4 inlined it away — is
**−10 209**.

### Level 2, the five games with their 3D packs, 600 frames after 120 of warm-up

| game | stage | base | lot | delta | % |
| --- | --- | ---: | ---: | ---: | ---: |
| metroid | core | 11 146 883 813 | 11 146 883 813 | **+0** | **+0.000** |
| metroid | total | 13 735 144 718 | 13 735 147 404 | +2 686 | +0.000 |
| mm2 | core | 12 037 289 565 | 12 037 289 565 | **+0** | **+0.000** |
| mm2 | total | 18 976 701 932 | 18 976 757 114 | +55 182 | +0.000 |
| smb1 | core | 11 515 129 187 | 11 515 129 187 | **+0** | **+0.000** |
| smb1 | total | 13 588 398 616 | 13 590 205 556 | +1 806 940 | **+0.013** |
| smb3 | core | 15 315 191 123 | 15 315 191 123 | **+0** | **+0.000** |
| smb3 | total | 24 191 384 463 | 24 187 507 015 | −3 877 448 | −0.016 |
| zelda2 | core | 12 775 009 516 | 12 775 009 516 | **+0** | **+0.000** |
| zelda2 | total | 15 359 932 329 | 15 359 460 188 | −472 141 | −0.003 |

**`core` is +0 on all five games, identical to the instruction**, and so are `apu`, `ppu` and
`draw_list`. Every `total` is inside the tolerance, so the lot's performance rule needs no
same-behaviour pair — but the largest of them, `smb1` **+0.013 %**, is the Lua column and the
stage table says so: `convert` +1 895 363 and `lua_hooks` +1 903 711 against `core` +0.

**And the level-2 floor was run too**, which no earlier lot's write-up has: the same binary twice.

| game | `core` | `total` |
| --- | ---: | ---: |
| metroid | +0 | +0.001 % |
| mm2 | +0 | +0.000 % |
| smb1 | +0 | **+0.007 %** |
| smb3 | +0 | −0.001 % |
| zelda2 | +0 | **−0.007 %** |

So `core` is +0 everywhere on an identical binary, as it must be, and `total` on that instrument
moves by up to ±0.007 % on the very game whose figure is the lot's largest. +0.013 % on `smb1` is
that column's own spread and not a cost of these boards — stated with the measurement rather than
asserted.

### The decision

| feature | figure | decision |
| --- | --- | --- |
| `board-215` | the three measured together: PR-1 worst ROM **+0.001 %**, set total −0.000 %; level-2 `core` **+0 on all five**, worst `total` +0.013 % against a floor of +0.007 % on the same game — all against **+0.02 %** | **in `default`** |
| `board-208` | as above | **in `default`** |
| `board-219` | as above | **in `default`** |

**No second feature inside any of the three**, and nothing to park. The owner's rule prices a
mechanism separately when it costs more than its board; here no symbol of the build moves at all,
so there is nothing to separate. The three are measured **together** because they ship together
and the figure is the instrument's either way; a per-feature split would be three more campaigns
reporting the same spread.

`docs/review/BOARD-FEATURES.md` has the three rows, and its "worst figure among the features in
`default`" paragraph is extended rather than changed: MF-UNL-10's **+0.009 %** stands, and this
lot's row is recorded as back among the ones whose figure is the instrument's.

[PROGRESS] MF-UNL-11 step 6/8 done: the campaign

## Step 7 — golden-core, and the full run with both ROM roots

### golden-core over the 98 commercial originals, 1 200 frames each

    zax-harness-<label> golden-core /mnt/g/3dsen/test-originals gc98-<label>.jsonl 1200

| | |
| --- | --- |
| `01850f36` (`mfunl11-0-base`) against the lot (`mfunl11-1-lot`) | **byte-identical** (`cmp`, not just the digest) |
| the digest | `md5 f79e7df842c3ee43e7d9b67861a927b5` |

**That is the digest MF-UNL-2 through MF-UNL-10 all recorded.** Ten lots, and not one pixel of a
commercial original has moved — which is what the lot brief asks of this one in those words:
"golden-core over the 98 originals must be byte-identical to the base commit's."

It is the measurement that turns the lot's two structural claims into facts rather than
intentions. Three variants were added to `IrqBoard`, `Mmc3Board` grew five widened `cfg` lists and
two accessors gained a caller, `BankWindows` gained a method, and `IrqBoard::try_read_prg`'s `cfg`
list gained an entry — every one of those is code that **existing** boards compile. 98 games at
1 200 frames each, byte-identical on every field golden-core records, is what separates "the
boards were added beside the others" from "one of them moved".

### The full run with both ROM roots

    ZAXNES_ROMS=G:/3dsen/demo/roms
    ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms
    cargo test --release --no-fail-fast -p zax-nes -p zax-harness -- --include-ignored

**1 152 passed, 0 failed, 0 ignored.** The same command without the two variables and without
`--include-ignored` is **1 143 passed, 0 failed, 9 ignored** — so the nine ROM-driven tests really
ran rather than passing vacuously, which is the check the brief asks for.

1 152 against MF-UNL-10's 1 127 is this lot's **25** tests.

[PROGRESS] MF-UNL-11 step 7/8 done: golden-core and the full run
