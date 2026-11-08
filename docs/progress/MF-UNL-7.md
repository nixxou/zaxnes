# MF-UNL-7 — the Sachen 8259, the 206 clones and two homebrew boards

Lot **MF-UNL-7** of `docs/review/UNL-census.md` §7.2, the seventh of the twelve MF-UNL lots:
**nine numbers, six boards, six Cargo features, ~10 original titles.**

Base commit: **`fc64566a`** (`MF-UNL-6 (8/8)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | numbers | board |
| --- | ---: | --- |
| `board-sachen8259` | **137, 138, 139, 141** | the Sachen **8259**, one chip with four wirings (8259D, B, C, A), an index/data port pair at $4100/$4101 under mask $C101 |
| `board-112` | **112** | the 206's register file with the ports split across $8000/$A000/$E000 and the register order permuted |
| `board-193` | **193** | NTDEC's **TC-112**: three CHR registers over **unequal** windows at $6000-$6002, one PRG window, mirroring at $6004 |
| `board-298` | **298** | a VRC4 clone in the **VRC4b** configuration whose $F003 "only acknowledges a pending IRQ" |
| `board-218` | **218** | the single-chip homebrew board of *Magic Floor*: **no CHR memory at all**, the 2 KiB of CIRAM serving the pattern tables |
| `board-29` | **29** | the Sealie **RET-CUFROM**: one $8000 register, two bits of CHR-RAM bank and three of PRG bank |

## The three things this lot has to carry from the census

### 1. The 8259's documentation caveat, and the attribution gap

§7.2 quotes the page on itself: its information "was gathered primarily from FCEU-mm and while it
appears to be a good enough behavioral description to run many ROMs, it is likely incomplete",
and it governs four numbers at once. So the lot implements the four variants from the page's own
A/B/C/D rewiring table and **records the attribution gap in the code comment**: neither NESdev nor
Wikipedia attributes *The Great Wall* and *Q-Boy* to 137/138/139/141 beyond what the pages do say
— 137's own page gives *The Great Wall* as "the only known 8259D game", and the 8259 page gives
*Q-Boy* as the **2M-RAM-COB** board, "usually set to mapper 141". Nothing is guessed past that.

### 2. MF-UNL-2's rule: where §7.2 and a number's own page differ, the page wins

Checked step by step as the lot goes, and each case gets a Q ticket.

### 3. The performance rule as the instruction states it

The +0.02 % tolerance is on **each ROM's total** instruction count — the PR-1 set and the level-2
games' `total` — and not only on the `core` column. A total that moves further than that with the
cause outside the emulation (the Lua `ltable.c` column is the one this project has seen) is shown
with a same-behaviour pair and recorded; the tolerance is not redefined.

## Steps

- [x] **0/9** — the plan: this file.
- [x] **1/9** — `board-sachen8259`: the index/data pair, registers 0-5 and 7, the per-variant CHR
  shift, the four mirroring modes and the CHR-RAM board.
- [x] **2/9** — `board-112`: the 206 with the page's three ports and its permuted register order.
- [x] **3/9** — `board-193`: the TC-112's three unequal CHR windows and its $6004 mirroring.
- [x] **4/9** — `board-298`: the VRC4b `const` selection and the one documented difference.
- [x] **5/9** — `board-218`: a board with no CHR memory, four CIRAM A10 wirings.
- [x] **6/9** — `board-29`: the RET-CUFROM's one register, and the flash path that is not emulated.
- [ ] **7/9** — the question tickets, the four sweeps, the save-state round-trips and the feature
  configurations.
- [ ] **8/9** — the callgrind campaign against the base commit, and the feature decision per board.
- [ ] **9/9** — golden-core, the full `--include-ignored` run with both ROM roots,
  `BOARD-FEATURES.md`, the census note, the journal and the final pass.

(Ten entries and not eight: §7.2's step list puts 218 and 29 in one step, and they share nothing —
one is the only board in this build with no CHR memory and the other is a plain latch — so each
takes its own.)

---

## Step 1 — the Sachen 8259, one chip with four wirings

`board-sachen8259`, in `default`, implying `board-decode-boards`. One new module,
`cartridges/mappers/sachen_8259.rs`, one new `DecodeBoard` variant (no `Board` variant — MF-6c
step 7's finding is that a board joining that enum is free to the rest of the library), four
numbers in `nametable_pages_vary` and one `registry::board_memory_sizes` row for the four.

### The ports, which are the eight-register ASIC's

`$4100` index and `$4101` data, both `.... .rrr` / `.... .ddd` — three bits — under **mask
$C101**. That is A15, A14, A8 and A0 and nothing between, so the pair repeats through the whole of
$4100-$7FFF and `board_memory_sizes` has to give all four numbers `prg_ram: Some(0)`, exactly as
MF-UNL-1 step 2 did for mappers 150 and 243: `$6100 & $C101` is `$4100`, and a game writing its
CHR block at $6101 must reach the ASIC rather than work RAM.

**One difference from that board, and it is an absence.** Mapper 150's page says "All three bits in
all eight registers are fully readable and writable"; this one says nothing of the kind, so the
data port is **write-only** here and the board leaves `Mapper::try_read_prg` at `Mapper`'s own
default. `DecodeBoard::try_read_prg`'s `cfg` list is therefore unchanged by this lot.

### The four wirings, read off the page's *CHR Rewiring* table as arithmetic

The chip always chooses one of four CHR registers with "PPU A[12:11]", i.e. a 2 KiB window, and
the variants differ in how many of the PPU's low lines it **passes through** to the CHR ROM:

| variant | # | passes | the six register bits drive | capacity | as a 2 KiB bank |
| --- | ---: | --- | --- | ---: | --- |
| 8259B | 138 | PPU A10-A0 | CHR A16-A11 | 128 KiB | `bank` |
| 8259A | 141 | PPU A11-A0 | CHR A17-A12 | 256 KiB | `(bank << 1) \| (w & 1)` |
| 8259C | 139 | PPU A12-A0 | CHR A18-A13 | 512 KiB | `(bank << 2) \| (w & 3)` |

A11 on the 8259A is both an address line of a 4 KiB bank *and* the window selector, which is the
whole of that row; the C does the same thing one bit further. The page's three capacities are the
arithmetic's own check — six bits of register times a 2, 4 or 8 KiB unit — and the test asserts
the three power-on signatures those shifts produce with every register zero, which are different
pictures of the same silicon: `[0,1,0,1,0,1,0,1]`, `[0,1,2,3,0,1,2,3]` and `[0,1,2,3,4,5,6,7]`.

### The 8259D, which is mapper 137 and has a page of its own

Four **1 KiB** windows over $0000-$0FFF and "PPU $1000-$1FFF: fixed to last 4KiB of CHR ROM", and
an R4 that is one bit **per block** rather than three common upper bits:

```text
...0 0ccc   CHR register 0        Internal 4: .... .DCB
...B 0ccc   CHR register 1        Internal 6: .... ...E
...C 0ccc   CHR register 2
...D Eccc   CHR register 3
```

So window 0 reaches eight banks, windows 1 and 2 sixteen with a hole in the middle, and **only
window 3 reaches the whole 32 KiB** the page gives the chip. That is the page's own "only register
3 accesses the full ROM", and it is why `Internal 6` exists at all — a register the family page
does not mention.

### Q-160: the two places the two pages contradict each other

MF-UNL-2's rule is that the number's own page wins, so the 8259D takes mapper 137's page and the
other three take the family page. The two differences are both in R7 (`.... .mms`):

| | family page (138, 139, 141) | mapper 137's page |
| --- | --- | --- |
| `mm` = 0 | `V` | `H` |
| `mm` = 1 | `H` | `V` |
| `s` fixes the mirroring to | `V` | `H` |
| `s` and the banks | "all 4 CHR banks use the CHR bank 0 register" | "banks become weird" |

The first three lines are one statement once seen the right way round: **simple mode fixes the
mirroring to that page's own mode 0**, on both pages, so one call says it for all four numbers.
The fourth is a gap — "weird" is all the 8259D's page says — and the 137 board takes the family
page's concrete sentence for want of any other. Q-160 records all four rows.

`mm` = 2 is `(0,1,1,1)` on both pages: one table on CIRAM page 0 and the other three on page 1, a
per-table layout no `NametableMirroring` can say, so it comes out of `Mapper::nametable_page` —
§4.5's cheap level and the eight-register ASIC's mode 0 read the other way round. `mm` = 3 is
`1scA` on both.

### The 2M-RAM-COB board

*Q-Boy*'s cartridge "replaces CHR-ROM with 8 KiB of unbanked CHR-RAM, and is usually set to mapper
141". An image with no CHR-ROM is therefore a board with **no CHR bank lines**: `update_banks`
returns after the PRG bank, the 8 KiB window stays where `BankWindows::new` put it, and a write to
R0-R4 or R6 moves nothing. The registers are still latched, because the index and data ports are
the chip's and not the CHR chip's — which is what the test pins, together with the PRG bank and the
mirroring still working.

### The attribution gap, recorded and not guessed

The census asks for this by name. Two sentences are all the sources give, and two sentences are all
this board claims: *The Great Wall* is the 8259D's "only one known" game with "32KiB PRG ROM and
32KiB CHR ROM", and *Q-Boy* is the 2M-RAM-COB board "usually set to mapper 141". Nothing assigns a
title to 138 or 139.

### Checks

`cargo fmt --check` clean; clippy `-D warnings` on `zax-nes --all-targets` clean in all four
configurations (default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-sachen8259`). `mapper_tests` **291/0/0** by default
(282 before this step, plus the nine this step adds — eight tests and the sweep's four new rows),
109/0/0 with the feature alone, 102/0/0 with no features, 291/0/0 with `parked-boards`; the three
latch sweeps of `cartridges::tests` pass in every one.

---

## Step 2 — mapper 112, the 206 with its ports split and its order rotated

`board-112`, in `default`, implying `board-namcot108`. **No new module, no new `Board` variant, no
new `DecodeBoard` variant and no new savestate shape**: one `bool` on `Namcot108`, one function
beside its `write_register`, one `SUPPORTED_MAPPERS` entry, one `create_board` arm and a note in
`board_memory_sizes` saying why there is no row.

### Why it is a parameter and not a board

The page's own sentence is the whole argument:

> This mapper is quite similar to iNES Mapper 206, which resembles an simplified MMC3; however,
> the registers appears to be scrambled a bit.

And the scramble turns out to be **entirely above the register file**. Below it — the two 8 KiB
PRG windows, the last two fixed to the end, the two 2 KiB CHR windows and the four 1 KiB ones —
the board *is* the 206, window for window, so `Namcot108::update_banks` is reused to the line and
the savestate block is the same eight bank bytes plus the select that MF-6a step 1 wrote. That is
the shape the lot instruction asks for by name ("reuse it through parameters **without** moving the
existing boards' hot path or save-state format").

**With `board-112` off, `Namcot108` is byte for byte what MF-6a step 1 shipped**: the field, the
function and the branch are all behind the feature, so the five Namco numbers cannot pay for a
sixth that is not theirs.

### The two halves of the scramble

**The ports.** Three under the page's own mask `$E001`, where the 206 has an even/odd pair:

```text
Bank select ($8000)     Bank data ($A000)     Nametable arrangement ($E000)
xxxx xRRR               DDDD DDDD             xxxx xxxN
```

$8001, $A001, $C000, $C001 and $E001 are inside that mask and are **not** registers: the page
names three addresses and the decode answers three.

**The order — a rotation by six.** `RRR` names the 206's `(RRR + 6) mod 8`:

| `RRR` | the page's own line | the 206's register |
| ---: | --- | ---: |
| 0 | 8 KB PRG ROM bank at $8000-$9FFF | R6 |
| 1 | 8 KB PRG ROM bank at $A000-$BFFF | R7 |
| 2 | 2 KB CHR bank at PPU $0000-$07FF | R0 |
| 3 | 2 KB CHR bank at PPU $0800-$0FFF | R1 |
| 4-7 | 1 KB CHR bank at PPU $1000 / $1400 / $1800 / $1C00 | R2-R5 |

`req_core_102_mapper_112_rotates_the_206s_register_order_by_six` is the census's test in both
directions: eight bytes through mapper 112's ports at its own indices give exactly the windows
eight bytes through a **mapper 206** give at the rotated indices, and the same eight written
unrotated give something else (`[4, 6, 14, 15]` / `[20, 21, 20, 21, 22, 23, 5, 9]`, the PRG banks
landing where the CHR registers belong).

### Q-161: the two things the page does not say

* **$C000-$DFFF.** The page lists eight selects covering two PRG windows and the whole of CHR, and
  says nothing about the upper 16 KiB. The 206's "the last two 8KiB banks [are] fixed to the end"
  is taken: it is the board this one is "quite similar to", and a board with no vectors cannot run.
* **the low bit of the two 2 KiB CHR selects.** The 206's page is explicit that "only bits 5-1
  exist for 2KB CHR banks"; this one gives the data port a whole byte and uses the MMC3's own
  wording, "Select 2 KB CHR bank". The 206's rule is taken, because it is a property of the
  silicon the page says this is — the register drives CHR A11 upwards and the window's own A10
  comes from the PPU. `q_161_mapper_112_ignores_the_low_bit_of_its_two_2_kib_chr_selects` pins
  both sides: selects 2 and 3 ignore bit 0, selects 4-7 do not.

### One thing it does say, worth quoting

> Nametable arrangement (0: horizontal (A10); 1: vertical (A11))

The parentheses settle it. `horizontal (A10)` is CIRAM A10 driven from **PPU A10**, which is this
core's `NametableMirroring::Vertical` — the two logical tables distinct across A10 — so the
numbering is the MMC3's own "0: vertical; 1: horizontal" after all and the word "horizontal" is
about the *arrangement of the screens* rather than the mirroring. The test asserts both values.

### Checks

`cargo fmt --check` clean; clippy `-D warnings` on `zax-nes --all-targets` clean in four
configurations (default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-112`). `mapper_tests` **295/0/0** by default, 114/0/0 with
the feature alone, 102/0/0 with no features, 295/0/0 with `parked-boards`; the three latch sweeps
pass in every one.

---

## Step 3 — mapper 193, NTDEC's TC-112

`board-193`, in `default`, implying `board-namcot108`. A second `bool` on `Namcot108`, two
functions beside its own, a `SUPPORTED_MAPPERS` entry, a `create_board` arm and a
`board_memory_sizes` row. **No new module, no new `Board` or `DecodeBoard` variant, and the same
savestate block.**

### What it shares with the 206, and what it does not

Nothing of the register file. The TC-112 has five registers at fixed addresses and no select/data
pair at all:

```text
Range,Mask:   $6000-7FFF, $E007

$6000:  [CCCC CC..]   CHR Reg 0        $6003:  [.... PPPP]   PRG Reg
$6001:  [CCCC CCC.]   CHR Reg 1        $6004:  [.... ...M]   Mirroring
$6002:  [CCCC CCC.]   CHR Reg 2
```

What it does share is the window table, the eight-byte register file and this struct's savestate
block — which is what §7.2's "112 and 193 are near-206 boards and `board-namcot108` has the 206"
asks be reused. `write_register_193` and `update_banks_193` have no line in common with the five
Namco numbers', and both are behind the feature, so with `board-193` off `Namcot108` is byte for
byte what MF-6a step 1 shipped.

**One ordering detail matters.** Every other number of this board begins its decode with
`if address < 0x8000 { return; }`; mapper 193's registers are *below* that line, so its branch
comes first in `write_register`.

### The three CHR windows, which are unequal

```text
  $0000                           $1000           $1800
+-------------------------------+---------------+---------------+
|           <<$6000>>           |    <$6001>    |    <$6002>    |
+-------------------------------+---------------+---------------+
```

4 KiB, 2 KiB, 2 KiB — and **all three registers hold a 1 KiB bank number**, which is what makes
their diagrams differ: `CCCC CC..` loses two bits because its window is 4 KiB and `CCCC CCC.` one
because its windows are 2 KiB. So the bank is the byte *shifted* and not masked, exactly as the
206's own R0 and R1 are shifted one place for their 2 KiB windows. The test writes an odd byte and
the even one below it into each and asserts they are the same bank, then asserts the three widths
by moving one window at a time.

### PRG and the window the registers occupy

`| $6003 | { -3} | { -2} | { -1} |`: one switchable 8 KiB window of four and the other three fixed
to the **end**, from four bits over this image's sixteen banks.

The mask `$E007` reads A15, A14, A13 and A2-A0, so the five registers repeat every eight bytes
through the whole of $6000-$7FFF — which is why `board_memory_sizes` gives the number
`prg_ram: Some(0)`, mappers 114, 115 and 182's row in the same words. $6005-$6007 are inside the
window without being registers, and A15 being clear in the mask means **no write above $8000 can
reach this board at all**; the test asserts both.

### Q-161: the mirroring register the page tells you no game uses

> No games are known to rely on changing mirroring at run-time. At least some games have
> hard-wired mirroring.

That is a note about the **catalogue** and not about the register, so $6004 is implemented and
`q_161_mapper_193_changes_mirroring_from_6004` is the test the census asks for by name. The
polarity is the page's own "0 = vertical, 1 = horizontal".

### Checks

`cargo fmt --check` clean; clippy `-D warnings` on `zax-nes --all-targets` clean in four
configurations (default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-193`). `mapper_tests` **299/0/0** by default, 114/0/0 with
the feature alone, 102/0/0 with no features, 299/0/0 with `parked-boards`; the three latch sweeps
pass in every one.

---

## Step 4 — mapper 298, a VRC4b clone with its own acknowledge

`board-298`, in `default`, with no dependency on anything. One `bool` on `Vrc2Vrc4`, one
three-line method on it, one `const fn` on `VrcIrq`, one arm in `VrcVariant::resolve`, the number
in `SUPPORTED_MAPPERS`, in `create_board` and in `Cartridge::clocks_on_cpu_cycle`, and a note in
`board_memory_sizes` saying why there is no row. **No new module, no enum variant anywhere, and no
savestate byte.**

### The page is one sentence of configuration and one of difference

> The board uses a VRC4 clone in the VRC4b configuration (CPU 0x02s to VRC4 A0, CPU 0x01s to VRC4
> A1).

which is `VrcVariant::Vrc4b`'s `index_lines` of `[1, 0]` exactly — so the whole register file comes
for free, and `req_core_102_mapper_298_is_a_vrc4b_bank_for_bank` is the test: the same writes
through mapper 25 submapper 1 and through 298 give the same four PRG windows, the same CHR bank and
the same mirroring. The assertion that makes it a *`VRC4b`* check and not a *VRC4* one is the CHR
nibble pair: the high nibble of CHR register 0 is at **$B002**, where a `VRC4f` would put it at
$B001.

> writing to $F003 only acknowledges a pending IRQ, but it does not move the 'A' control bit to the
> 'E' control bit [the way an original VRC4 does].

$F003 is the acknowledge on a `VRC4b` (index 3 from A1 and A0 both set), and $F001 is its control
register — which is why the page can name one address and no registers at all.

### Why a field and not a `const` parameter

§7.2 asks for "a `const` parameter on `Vrc2Vrc4` selecting `Vrc4b` … in the manner of `McAcc`".
Taken literally that is a second monomorphisation of this board, and a second monomorphisation
needs a second `Board` discriminant — which MF-6c steps 2 to 5 priced at `mmc5` **+0.066 %**
against a tolerance of +0.02 %. `McAcc` could afford its discriminant because MR-4-3's difference
is read on *every PPU fetch*; this one is read on a write to $F003.

So it is a load-time `bool`, the shape `Namcot108`'s own note argues for at length, and
`acknowledge_pending_only` is a second method on `VrcIrq` rather than a flag inside it — because
one byte more in that struct moves `pending`'s offset inside `Vrc2Vrc4`, `Vrc6` and `Vrc7`, and
MF-6b step 1 priced that at eleven of PR-1's thirteen ROMs, +0.009 % to +0.027 %.

### The A/B, and what it shows

`req_core_114_mapper_298_acknowledges_at_f003_without_copying_a_into_e` runs the same sequence on
both chips, twice over:

| | mapper 25.1 (Konami) | mapper 298 (the clone) |
| --- | --- | --- |
| `E`=1, `A`=0, fire, then $F003 | the counter **stops** (`A` copied into `E`) | it **fires again** |
| `E`=0, `A`=1, then $F003 | the counter **starts** (256 ticks to the first IRQ) | it never starts |

Both chips acknowledge the pending IRQ; only Konami's moves the enable.

### Q-162: the census describes the difference as something else

§7.2's test line is "a write to $F003 acknowledges an IRQ and does **not reload the counter** — the
single difference from mapper 21/25". That is not a difference: a VRC4's acknowledge does not
reload either, and only `$F001` with `E` set does ("If E is set, the IRQ counter is reloaded with
the latch value"). The page wins, which is MF-UNL-2's rule, and
`q_162_no_vrc4_acknowledge_reloads_the_counter_on_either_chip` pins the half of the census's
sentence that is true — of **both** chips.

### Checks

`cargo fmt --check` clean; clippy `-D warnings` on `zax-nes --all-targets` clean in four
configurations (default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-298`). `mapper_tests` **301/0/0** and `mapper_irq_tests`
**57/0/0** by default; 104/0/0 and 42/0/0 with the feature alone; 102/0/0 and 40/0/0 with no
features; 301/0/0 and 57/0/0 with `parked-boards`. The three latch sweeps pass in every one, and
`mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` is what would have caught the number
missing from `clocks_on_cpu_cycle`.

---

## Step 5 — mapper 218, Magic Floor: the board with no CHR memory

`board-218`, in `default`, implying `board-decode-boards`. One new module,
`cartridges/mappers/magic_floor.rs`, one new `DecodeBoard` variant, two forwards that enum gains
(`read_nametable` and `write_nametable`, which it had for mapper 111 alone), the number in
`nametable_pages_vary`, one `SUPPORTED_MAPPERS` entry, one `create_board` arm and one
`board_memory_sizes` row. **No `Board` variant.**

### The whole board

> There is no CHR-ROM or CHR-RAM on the cartridge. Instead, the console's internal 2KiB nametable
> RAM is mapped as CHR-RAM.

and "the cartridge board only contains a single PRG-ROM chip (up to 32KiB)" beside a CIC. **No
register, no latch, no bank line and no CHR memory** — the only board in this build with nothing a
game can write, which is why the census's own test line for it ends "**there is no CHR bank
register to test**, which is the point".

### The four configurations, and the one expression that is all four

> [It] can be used in four configurations by wiring CIRAM A10 to one of the PPU A10..A13 address
> lines.

| iNES byte 6 | line | the page's name for it |
| --- | --- | --- |
| `$A1` | PPU A10 | Two-screen, Horizontal arrangement ("Vertical mirroring") |
| `$A0` | PPU A11 | Two-screen, Vertical arrangement ("Horizontal mirroring") |
| `$A8` | PPU A12 | Single-screen arrangement BLK0 |
| `$A9` | PPU A13 | Single-screen arrangement BLK1 |

Bit 3 of byte 6 — the four-screen flag — picks the pair and bit 0 picks within it; the two
two-screen rows are the ordinary iNES reading of bit 0 after all (`0` is CIRAM A10 from PPU A11,
`1` from PPU A10). Byte 6 is read whole, which is the shape MR-30-1 gave the UNROM 512 and the
reason `InesHeader::byte6` is exposed.

Every access, pattern table or nametable, then lands at

```text
CIRAM offset = ((PPU A<line>) << 10) | (PPU A9..A0)
```

**One expression, because that is the wiring** — A10 of the RAM comes from one PPU line and its
nine low lines come from the PPU's. Everything the page describes falls out of it, and the test
asserts it as eight 1 KiB pattern windows plus four nametables rather than as a claim:

| line | the four nametables | the eight 1 KiB pattern windows |
| ---: | --- | --- |
| A10 | `0 1 0 1` | `0 1 0 1 0 1 0 1` |
| A11 | `0 0 1 1` | `0 0 1 1 0 0 1 1` |
| A12 | `0 0 0 0` | `0 0 0 0 1 1 1 1` |
| A13 | `1 1 1 1` | `0 0 0 0 0 0 0 0` |

and the page's two capacities come out as a count: a two-screen wiring shows **both** blocks inside
one pattern table ("2KiB (128 tiles)") and a single-screen one shows a single block ("1KiB (64
tiles)"). On the A12 wiring the block the background shows is the page's "swappable via PPUCTRL",
because that register's pattern-table select *is* A12; on the A13 wiring the pattern tables never
see block 1 at all.

### Where the 2 KiB lives, and why that is not a liberty

CIRAM is in the console, and a board that maps it into the pattern tables needs **one** 2 KiB
memory to answer both windows — which this architecture cannot do with the PPU's own VRAM, because
`Mapper::ppu_peek` has no way to reach it. So the 2 KiB is the cartridge's CHR-RAM
(`board_memory_sizes` gives the board exactly that much and no PRG-RAM) and `nametable_page`
answers `SERVED_BY_CARTRIDGE` for all four tables — MF-6c step 9's shape for the GTROM and
MR-30-6's for the UNROM 512's four-screen mode.

**It is the same chip either way.** Which side of the connector an emulator keeps CIRAM on is
invisible to a game; that the pattern tables and the nametables are one memory is not, and it is
what the board exists for — *Magic Floor* draws its tiles through the nametable window. A board
that served patterns from the cartridge and nametables from console VRAM would be two memories and
*would* be wrong. `req_core_102_mapper_218_serves_its_pattern_tables_from_ciram` pins it in both
directions: a byte written at $2000 reads back through the pattern window, and a byte written at
$0010 reads back at $2010.

This is the **fifth** distinct body of the `Mapper::read_nametable` dispatch Q-123 once measured a
cap of two on, and the **second inside `DecodeBoard`** — so `Board::read_nametable`'s own jump
table does not grow by an entry, which is what MF-6c step 7 established for a board joining that
enum. What it costs is step 8's to measure.

### PRG, and the two things that are not emulated

$8000-$FFFF is bank 0 of 32 KiB, unbanked, which a 16 KiB image shows twice as `Nrom` does. The
test writes $FF at seven addresses from $6000 to $FFFF and asserts that **nothing moves** — the
board has no register, and `board_memory_sizes` clearing $6000-$7FFF is what sends those writes to
`Mapper::write_register`'s empty default rather than into work RAM.

The CIC is not emulated, which is true of every board here and not a property of this one.

### Checks

`cargo fmt --check` clean; clippy `-D warnings` on `zax-nes --all-targets` clean in four
configurations (default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-218`). `mapper_tests` **305/0/0** by default, 106/0/0 with
the feature alone, 102/0/0 with no features, 305/0/0 with `parked-boards`; the three latch sweeps
pass in every one.

---

## Step 6 — mapper 29, the RET-CUFROM, as an eighth `SingleLatch` board

`board-29`, in `default`, implying `board-decode-boards`. **No new module and no new variant of
any enum but `LatchBoard`**: MF-UNL-3 step 2 built that one for exactly this shape, so the board is
an eighth value of a runtime field, two arms in `SingleLatch::update_banks` and
`write_register`, one in `mapper_number`, a mirroring line in `reset`, one `SUPPORTED_MAPPERS`
entry, one `create_board` arm and one `board_memory_sizes` row.

### The board

```text
Range,Mask:   $8000-FFFF, $8000

$8000:  [...P PPCC]
```

`CC` is the 8 KiB CHR-**RAM** bank over "4x banks worth of full 8KB CHR ram" and `PPP` the 16 KiB
PRG bank over "8x 16KB PRG banks", with $C000-$FFFF fixed to the last bank. Mask $8000 is A15
alone, so **every** address in $8000-$FFFF is the register — mapper 149's decode, and the test
walks five addresses from $8000 to $FFFF to say so. Bits 7-5 are not register bits, which the
`...` of the diagram is and which a write of $FF pins.

The page gives the board no bus-conflict line, so it is **not** on `registry::has_bus_conflicts`
and the whole written value arrives at the latch.

### Two things the page states flatly and no header can

* "It is hard-wired for vertical mirroring" — so `SingleLatch::reset` says so, and it is the one
  board of the eight that writes a mirroring at all. The other seven keep whatever their dump's
  header said, because their pages have no such line.
* "contains 8KB of WRAM mounted in the usual place", and "4x banks worth of full 8KB CHR ram" —
  `board_memory_sizes` gives both. The CHR figure matters on its own account: REQ-CORE-012's
  CHR-RAM default is 8 KiB, which would leave three of this board's four banks unaddressable.

This is the **first member of `LatchBoard` with CHR-RAM**, which is why the census's test line for
it ends "writes to CHR-RAM visible on read-back" — four banks of RAM cannot be told apart any other
way, so the test writes a marker into each of the four and then reads all four back.

### The CopyNES path, which is not emulated

> If EXP0 and R/W go low, /WR reaches the flash ROM. When EXP0 is high, the internal bankswitching
> register gets accessed.

EXP0 is a cartridge **edge pin**, driven by a programmer and not by a game, so a running game
always takes the second branch — the branch this board implements. The census asks that the choice
be recorded in a comment and `SingleLatch`'s own note is where it is, with the reason the flash of
the UNROM 512 and the GTROM *is* implemented: games on those boards save through it, and no
mapper-29 title does.

### Checks

`cargo fmt --check` clean; clippy `-D warnings` on `zax-nes --all-targets` clean in four
configurations (default, `--no-default-features`, `--features parked-boards`,
`--no-default-features --features board-29`). `mapper_tests` **308/0/0** by default, 105/0/0 with
the feature alone, 102/0/0 with no features, 308/0/0 with `parked-boards`; the three latch sweeps
pass in every one.

**All six boards of the lot are in, and the nine numbers are 29, 112, 137, 138, 139, 141, 193, 218
and 298.** Steps 7 to 9 are the question tickets, the sweeps in every configuration the lot adds,
the campaign and the final pass.
