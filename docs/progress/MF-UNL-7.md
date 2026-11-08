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
- [ ] **3/9** — `board-193`: the TC-112's three unequal CHR windows and its $6004 mirroring.
- [ ] **4/9** — `board-298`: the VRC4b `const` selection and the one documented difference.
- [ ] **5/9** — `board-218`: a board with no CHR memory, four CIRAM A10 wirings.
- [ ] **6/9** — `board-29`: the RET-CUFROM's one register, and the flash path that is not emulated.
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
