# MF-UNL-3 — AVE, and the single-latch boards

Lot **MF-UNL-3** of `docs/review/UNL-census.md` §7.2, the third of the twelve MF-UNL lots:
**nine numbers, eight boards, eight Cargo features** — the largest feature matrix any lot in the
plan runs, which is why §7.2 prices it at eight hours and not six.

Base commit: **`452e8464`** (`MF-UNL-2 (9/9)`), the branch tip when the lot opened.
Branch: `mapper-review`. Worktree: the `wt-mappers` worktree.

| feature | numbers | board |
| --- | --- | --- |
| `board-nina03` | **79, 146** | AVE's NINA-03/06 and the Sachen 3015 / SA-016 that are functionally identical to it: one register at $4100 |
| `board-38` | **38** | Bit Corp. UNL-PCI556: a `[xxxx CCPP]` latch at $7000-$7FFF |
| `board-107` | **107** | Magicseries *Magic Dragon*: one `[BBBB BBBb]` register driving CHR and PRG at once |
| `board-143` | **143** | Sachen TCA-01: an NROM board whose only logic is a protection **read** |
| `board-144` | **144** | AGCI 50282 (*Death Race*): the Color Dreams board with a 300 Ω resistor on CPU D0 |
| `board-145` | **145** | Sachen SA-72007: `$4100: [C... ....]`, one CHR bank bit |
| `board-149` | **149** | Sachen SA-0036: the same bit, moved onto the PRG window, so with bus conflicts |
| `board-190` | **190** | Zemina *Magic Kid Googoo*: three windows, each from its own register |

## The steps

- [x] **0/9 — the plan.** This file: the nine numbers, the page each is built from, the eight
  features, the shape each board takes in this tree, and the **five** places this lot leaves the
  census's own prescription for the number's page.
- [x] **1/9 — `board-nina03`.** `Nina03`, the $4100 control register for 79 and 146, its
  `registry.rs` arm, `SUPPORTED_MAPPERS`, `board_memory_sizes`, and the tests the census lists —
  including the anti-test for the FCEUX CNROM switch this lot deliberately leaves out.
- [x] **2/9 — `SingleLatch`, and the four plain latches: 38, 107, 145, 149.** One
  `SmallBoard`-style `DecodeBoard` variant holding the seven remaining boards as a runtime field,
  on the `Jv001` precedent, with four of the seven wired and tested.
- [x] **3/9 — `board-143` and `board-144`,** the two boards of that variant that are not a bank
  latch: 143's protection read (through the `try_read_prg` forward `board-jv001` already added)
  and 144's resistor rule.
- [x] **4/9 — `board-190`,** the last of the seven and the only one with more than one window.
- [x] **5/9 — the four sweeps, the save-state round trip and the feature matrix.** Each of the
  eight features alone, all eight, none, `default` and `--features parked-boards`; `cargo fmt
  --check` and clippy `-D warnings` in each.
- [x] **6/9 — the question tickets, and their code references.** Q-145, Q-146 and Q-147, the
  three places this lot had to choose. The Cargo feature comments are written as each feature is
  added (steps 1 to 4); **the eight `docs/review/BOARD-FEATURES.md` rows state step 7's figures
  and are therefore in step 7's commit**, which is the order MF-UNL-2 used for the same reason.
- [ ] **7/9 — the callgrind campaign, and the documentation it carries.** PR-1's thirteen ROMs
  and level 2's five games against `452e8464`, every compared commit built from `git archive` in
  a fresh directory, the feature decision for each of the eight boards, and the eight rows of
  `docs/review/BOARD-FEATURES.md` with those figures in them.
- [ ] **8/9 — golden-core against the base commit**, and the full check with `ZAXNES_ROMS` and
  `ZAXNES_TEST_ROMS` set.
- [ ] **9/9 — the journal and the final pass.**

## The nine numbers, and the page each is built from

Every figure below comes from the number's own **nesdev wiki** page. MF-UNL-2 step 9 left this
note for the lots that follow and it is the first thing this lot did: *§7.2's steps were drafted
from Mesen 2's `MapperFactory`, so the step text and the page can differ, and the page wins.*
Five of this lot's nine numbers are such a case (see the next section).

### 79 and 146 — the NINA-03/06, and the one page that holds both

nesdev's **"NINA-003-006"** page is the page of both numbers:

| | |
| --- | --- |
| company, boards | American Video Entertainment; **NINA-03**, **NINA-06**, which "differ only in CIC type" |
| PRG | capacity 64K, window **32K** |
| PRG RAM | **None** |
| CHR | capacity 64K, window **8k** |
| mirroring | "Fixed H or V" — solder pads, so the header's |
| bus conflicts, IRQ, audio | **No**, No, No |
| 146 | "iNES Mapper 146 denotes the 60-pin Sachen 3015 and SA-016 (chip-on-board) circuit boards, which are functionally identical" |

```text
A~[010x xxx1 xxxx xxxx]   D~[.... PCCC]
                                |+++--  8 KiB CHR bank at PPU $0000-$1FFF
                                +-----  32 KiB PRG bank at CPU $8000-$FFFF
```

The address pattern reads A15, A14, A13 and A8 and nothing else, which is **mask $E100, $4100** —
the same window `Sachen72008` decodes, and the page enumerates it: "$4100-$41FF, $4300-$43FF,
$45xx, $47xx, …, $5Dxx, and $5Fxx".

The page's closing line is the one the census asks be left out: "iNES Mapper **148** has the same
bit assignment in the data latch, but moves the latch register to the CPU $8000-$FFFF register,
introducing bus conflicts." That number is a different mapper and not a fallback for this one.

### 38 — the Bit Corp. UNL-PCI556

| | |
| --- | --- |
| company, board | Bit Corp.; **UNL-PCI556** |
| games | *Crime Busters* |
| PRG | 128K, window 32K |
| PRG RAM | **None** |
| CHR | 32K, window 8k |
| mirroring | **V (hardwired)** |
| bus conflicts, IRQ, audio | No, No, No |

```text
$7000-$7FFF:  [xxxx CCPP]
                   ||++-  32 KiB PRG bank at CPU $8000-$FFFF
                   ++---   8 KiB CHR bank at PPU $0000-$1FFF
```

and the *Hardware* note: "Writes to $F000-$FFFF are **very likely** to also trigger a
bankswitch, but well behaved code has no reason to", because the 74138 "uses /ROMSEL and R/W on
its enable inputs, but /ROMSEL is 1 while M2 is low, not just while M2 is high and A15 is low".

### 107 — Magicseries' *Magic Dragon*

One register over the whole of $8000-$FFFF, and the page's own diagram is what says how wide each
field is:

```text
7  bit  0
---- ----
BBBB BBBb
|||| ||||
++++ ++++- Select 8 KiB CHR bank at PPU $0000-$1FFF
++++ +++-- Select 32 KiB PRG ROM bank at CPU $8000-$FFFF
```

So the CHR bank is **all eight bits** and the PRG bank the **upper seven** — "Unusually, bits 1-7
is used to switch both CHR and PRG banks simultaneously". The page adds that "FCEUX implements
only the bottom 3 bits of the unified register, however MesenCE supports the full 8 bits", which
is the census's "all eight bits are honoured", and that "it's currently unknown whether this
mapper suffers from bus conflicts or not".

### 143 — the Sachen TCA-01, a board with no bank register at all

The page "denotes an NROM circuit board with a simple form of protection", "consistent with a
74138 and a 74368", and the whole of its register section is a **read**:

```text
A:[010. ...1 ..DD DDDD] -> [xxii iiii]
               || ||||      |||| ||||
               ++-++++------||++-++++-- reads provide the logical inversion of DDDDDD
                                        in the bits iiiiii
                            ++--------- open bus
```

`DDDDDD` is **the address**, bits A5-A0 — not the value a game wrote. The mask is $E100 again
($4100), and there is no write side: "There is no bankswitching register section."

The page's other sentence is about the console and not the board: "Nestopia additionally implies
that at least *Dancing Blocks* relies extensively on the values of uninitialized memory, and so
manually sets the emulated console's 2K of internal RAM to a different pattern than Nestopia's
default." The census asks that this go in a comment and not a test, and it does — it is a
RAM-init property.

### 144 — the AGCI 50282 of *Death Race*

"A intentionally defective variant of the Color Dreams board (mapper 11)", the PCB "labelled
50282" and "almost identical to the revision B Color Dreams boards", with one modification: "a
300 Ω resistor was added between CPU D0 and the combination of mapper hardware and ROM. The
effect of this is that **only the ROM's least significant bit always wins bus conflicts**."

Three candidate formulas, all three on the page:

| | |
| --- | --- |
| Nestopia 1.4.0 | `EffectiveData = (WrittenData & ROM[address]) \| (ROM[address] & 1)` |
| FCEUX 2.1.5 | "ignores writes to $8000" |
| "most succinct" | `EffectiveData = (ROM[address] & (WrittenData \| 1))` |

The first and the third are the **same function**; the second is a workaround for the one address
*Death Race* writes. The register itself is mapper 11's, `[CCCC PPPP]`.

### 145 and 149 — two Sachen boards and one bit

| | 145 | 149 |
| --- | --- | --- |
| board | **SA-72007** | **SA-0036** |
| game | *Sidewinder* | *Taiwan Mahjong 16* |
| the page's own words | "Yet Another CNROM clone", "very similar to iNES Mapper 087" | "just like mapper 145, but moves the register to overlap with PRG ROM and so **has bus conflicts**" |
| mask | **$E100** | **$8000** |
| register | `$4100: [C... ....]` — select 8 KiB CHR bank | `$8000: [C... ....]` — the same |

**The bank bit is bit 7 on both**, which is the one thing a test of these two has to pin: `[C...
....]` is the page's own diagram and it puts the bit at the top of the byte. 149's mask reads
**A15 alone**, so every address in $8000-$FFFF is the register.

### 190 — Zemina's *Magic Kid Googoo*

The one board of the seven with more than one window:

```text
CPU $6000-$7FFF:  8 KiB RAM
CPU $8000-$BFFF:  16 KiB switchable PRG ROM bank
CPU $C000-$FFFF:  16 KiB PRG ROM, fixed to the FIRST bank
PPU $0000-$1FFF:  four 2 KiB switchable CHR ROM banks

CHR bank select ($A000-$BFFF):  A~[101. .... .... ..bb]  D~[CCCC CCCC]
    2 KiB CHR bank at the PPU address starting at bb*$800

PRG bank select ($8000-$9FFF, $C000-$DFFF):  A~[1P0. .... .... ....]  D~[.... .PPP]
    16 KiB PRG bank at $8000-$BFFF
```

`P` in the **address** is the high bit of the four-bit bank number — $8000-$9FFF writes the low
eight banks and $C000-$DFFF the high eight — and the fixed window is the **first** bank, not the
last. The four CHR windows are picked by address bits 1-0 and the bank is the whole written byte.

## The five places this lot leaves the census's prescription for the page

| | the census's step or test | the page | this lot |
| ---: | --- | --- | --- |
| **143** | "a read returns the logical inversion of the **written bits**, open bus elsewhere" | the inversion of **address** bits A5-A0; there is nothing to write | the page — **Q-145** |
| **144** | "a write of `$FF` to $8000 over a ROM byte of `$00` yields bank `$FE`, not `$FF`" | all three of its formulas give **`$00`** for that pair; what the resistor does is make the ROM's **bit 0** win | the page, and a test that pins the rule the page states — **Q-146** |
| **38** | "$F000-$FFFF very likely switches too, implemented as the same decode" | "**very likely** … but well behaved code has no reason to" | the census's decode, recorded as a choice the page does not assert — **Q-147** |
| **107** | — | "it's currently **unknown** whether this mapper suffers from bus conflicts or not" | no conflicts, which is the reading that latches what a game wrote — **Q-147** |
| **145** | "a one-register CHR bank switch" | the bit is **bit 7**, `[C... ....]` | the page; the test names the bit |

Item 144 is the one worth stating twice: the census's figure is not produced by any of the three
formulas its own sentence refers to, so a test written from the census would have failed against
every implementation the page names.

## Where each board sits in `Board`

**No new `Board` discriminant.** Every one of the eight is a register decode with no IRQ, no
per-cycle hook and no PPU-bus latch, which is `DecodeBoard`'s entry condition — the second
shared variant MF-6c step 6 priced at zero and MF-UNL-1 filled three more slots of.

`DecodeBoard` grows **two** variants:

* `Nina03`, for 79 and 146, which are one board under two numbers;
* `SingleLatch`, for the other seven, **one variant with a runtime field** on the `Jv001` and
  `Namcot108` precedent: every difference between the seven is read only on a CPU access to the
  board's own window, "the rarest bus event there is", so seven monomorphisations would have
  bought seven arms in every inner match of this enum and one field buys one.

The only forward either board needs is `Mapper::try_read_prg`, for 143's protection read, and
`DecodeBoard` has carried it since MF-UNL-1 step 1 (`#[inline(never)]`, so the arm
`Board::try_read_prg`'s `#[inline(always)]` folds into `Cartridge::try_cpu_read` is one `callq`).
Its `cfg` gains `board-143`.

**144 does its own bus-conflict arithmetic** and is therefore *not* on
`registry::has_bus_conflicts`: that function's generic path ANDs the written value with the ROM
byte, which destroys the ROM's bit 0 before the board can see it. **149 is** on it — "has bus
conflicts" with nothing qualifying it, so the generic AND is exactly right.

## Step 1 — `board-nina03`: the AVE NINA-03/06 and the Sachen 3015 / SA-016

`Nina03` (`cartridges/mappers/nina03.rs`), one `DecodeBoard` variant and one byte: the
`[.... PCCC]` latch under mask $E100, bits 2-0 the 8 KiB CHR bank and bit 3 the 32 KiB PRG bank.
Two numbers on one board, because the page says in so many words that they are one board.

`board_memory_sizes` gives both numbers `prg_ram: Some(0)` — on the page's **"PRG RAM capacity:
None"** and not because the register is in the way. `$6100 & $E100` is `$6100`, so nothing forces
the question here the way mappers 140 and 184 force it; the page states it, so the function does.

**FCEUX's CNROM switch at $8000-$FFFF is not implemented, and it is a decision.** The page's own
closing line gives a latch at $8000-$FFFF to mapper **148** — "the same bit assignment in the data
latch, but moves the latch register to the CPU $8000-$FFFF register, **introducing bus
conflicts**" — and gives *this* board "Bus conflicts: No", which is only true of a board whose
register the PRG-ROM does not drive. So a dump that wants a CHR switch at $8000 is a mislabelled
mapper-3 image and its header is the fix. `Nina03`'s doc comment carries the argument and
`req_core_102_mapper_79_ignores_a_write_at_8000` pins the silence on four addresses.

| test | what it pins |
| --- | --- |
| `req_core_102_the_nina_03_banks_prg_and_chr_from_one_latch_at_4100` | both numbers: `CCC` is three bits of an eight-bank CHR-ROM and reaches no PRG window, `P` is bit 3 and is two 32 KiB banks, and the two fields are independent |
| `req_core_102_mapper_79_ignores_a_write_at_8000` | the page's enumerated mirrors ($4300, $5F00, $41FF) are the same latch; A8 clear is not ($4000, $4200, $5E00); $6100 is not; and **$8000, $8100, $C100 and $FFFF change neither window** |
| `req_core_090_the_nina_03_latch_survives_a_save_and_restore` | §7.1 item 2 for this board, which the two generic sweeps cannot reach: they write $5000-$E001 and this board's only register is $4100 |

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **964 passed, 0 failed, 9 ignored** (961 / 0 / 9 at `452e8464`, +3) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes --all-targets` | clean |

## Step 2 — `SingleLatch`, and the four plain latches: 38, 107, 145, 149

`SingleLatch` (`cartridges/mappers/single_latch.rs`) is **one `DecodeBoard` variant for all
seven** of the lot's single-latch boards, with a `LatchBoard` field saying which — the shape
`Jv001` uses for six numbers and `Namcot108` for five, and the one the census asks for ("one
`SmallBoard`-style variant holding the seven remaining latches"). Seven `DecodeBoard` arms would
still have been free of the `Board` dispatch, which is what MF-6c step 6 paid for; what they
would have cost is seven arms in each of that enum's ten inner matches.

The field is a runtime field and not a `const`, for `Jv001`'s own reason: every difference
between the seven is read only by `write_register` and `update_banks`, i.e. only on a CPU write
to the board's own window.

| # | window | the page's register | this build |
| ---: | --- | --- | --- |
| 38 | `$7000-$7FFF` **and `$F000-$FFFF`** | `[xxxx CCPP]` | `address & $7000 == $7000` — A15 not decoded, which is the page's *Hardware* note (**Q-147**) |
| 107 | `$8000-$FFFF` | `[BBBB BBBb]` | `chr_8k(v)`, `prg_32k(v >> 1)`: eight bits of CHR and seven of PRG |
| 145 | `$4100`, mask `$E100` | `[C... ....]` | `chr_8k((v >> 7) & 1)`, PRG bank 0 |
| 149 | `$8000`, mask **`$8000`** | the same | the same, plus `has_bus_conflicts` |

Three registry rows besides the four `create_board` arms and the four `SUPPORTED_MAPPERS`
entries:

* **`board_memory_sizes`: mapper 38 gets `prg_ram: Some(0)`** — "PRG RAM capacity: None" *and*
  the register is the upper half of that window, so it is mappers 140's and 184's arm for both of
  their reasons at once;
* **`has_bus_conflicts`: mapper 149 is `true`**, on its page's own "moves the register to
  overlap with PRG ROM and so has bus conflicts", with no submapper clause to qualify it;
* and that function's comment now records why **38, 79, 107 and 145 are absent** — including
  107's "it's currently unknown whether this mapper suffers from bus conflicts or not", answered
  **no** and filed as Q-147.

### What the tests had to notice

**Bit 7 of mapper 107's register is invisible in a power-of-two image.** `$88` and `$08` differ
by 128 in the CHR field and by 64 in the PRG field, and sixteen 8 KiB CHR banks wrap both to the
same bank — the first version of the test asserted a difference that cannot exist and failed on
it. The test now proves the field layout on a 256 KiB/128 KiB image and then the **top bit** on a
second image of **24** 8 KiB PRG banks and 24 1 KiB CHR banks, a count that divides neither
distance.

**Mapper 149's bank 1 is only reachable over an `$FF` PRG bank.** The generic conflict path ANDs
the written value with the ROM byte and a numbered bank carries bits 0-3 alone, so a `$80` write
over bank 0 latches nothing — which `MapperRom::prg_bank_all_ones` exists for, and which the test
then asserts in *both* directions: `$80` over the `$FF` bank switches, `$80` over a numbered bank
does not.

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_38_banks_from_7000_and_from_f000` | `PP` and `CC` as separate fields; **$F000 and $FFFF are the same latch**; $6FFF, $6000, $8000 and $EFFF are not |
| `req_core_102_mapper_107_honours_all_eight_bits_of_its_one_register` | bit 0 reaches CHR and not PRG; a bank above 7; **bit 7 in both fields**; the whole of $8000-$FFFF is the register and nothing below it |
| `req_core_102_mappers_145_and_149_switch_chr_from_bit_7_alone` | bit 7 and no other bit; no PRG bank line on either; 145's mask $E100 and 149's mask $8000; and 149's conflict both ways |
| `req_core_090_every_single_latch_board_round_trips_with_its_latch_dirtied` | §7.1 item 2 per board, with a second dirtying that `assert_ne!` proves is not vacuous, and a reset |

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **968 passed, 0 failed, 9 ignored** (+4 over step 1) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes --all-targets` | clean |

## Step 3 — `board-143` and `board-144`: the two that are not a bank latch

Two more `LatchBoard` variants, two more features, and the two places in this lot where the
number's own page and the census's own test cannot both be right.

### 143 — the protection read, and the census's "written bits"

The board has **no write side at all**: "an NROM circuit board with a simple form of protection",
"consistent with a 74138 and a 74368", and one read under mask $E100 whose answer is the logical
inversion of the **address**'s bits A5-A0. The census asks for "the logical inversion of the
written bits"; a 74368 is an inverting bus buffer with nothing to latch into, the page's diagram
names `DDDDDD` inside the *address* pattern, and `req_core_102_mapper_143_answers_the_inversion_of_its_own_address_bits`
walks all 64 of them. **Q-145** records the divergence.

The read goes through `Mapper::try_read_prg`, which `DecodeBoard` has forwarded since MF-UNL-1
step 1 for `Jv001`'s port at the same $4100; that forward's `cfg` gains `board-143` and nothing
else moves. The two bits the page calls open bus read as **0**, which is `Jv001`'s own compromise
for the same reason — the hook returns a byte and cannot see the CPU data bus.

*Dancing Blocks*' reliance on uninitialised RAM is in the board's doc comment and is **not** a
test, as the census asks: it is a property of the console's 2 KiB and not of this board.

### 144 — the resistor, and a figure that no formula on the page produces

`EffectiveData = ROM[address] & (WrittenData | 1)`, the page's own "most succinct" candidate and
the same function as its Nestopia one; FCEUX's "ignores writes to 0x8000" is a workaround for one
address and is not taken. The register itself is mapper 11's `[CCCC PPPP]`, read exactly as
`ColorDreams` reads it, because it is the same latch on the same board.

**The board is deliberately not on `registry::has_bus_conflicts`.** That function's generic path
ANDs the written value with the ROM byte inside `Cartridge::cpu_write`, which destroys bit 0
before the board could put the ROM's bit 0 back. `SingleLatch` reads the byte itself, through the
windows as they stand — i.e. before this write moves them, which is the order the hardware
latches in — and that function's comment now records both why 144 is absent and why 149 is
present.

**The census's figure is not any of the three formulas.** It asks that a write of `$FF` over a
ROM byte of `$00` yield bank `$FE`; all three candidates give `$00`, and the rule the page
states is about bit 0 alone. The test pins that rule instead — `$0F & ($30 | 1)` is `$01`, PRG
bank **1**, where an unmodified mapper 11 would show bank 0 — and **Q-146** records it.

### One thing the test had to notice

**The fixture's last PRG bank does not read as its own number at $E000.** `MapperRom::image`
writes an endless loop (`4C 00 E0`) at the start of the last 8 KiB bank and the three vectors at
its end, so a bus-conflict test that writes to $E000 conflicts against `$4C`. The first run
failed on exactly that; the test writes to **$E100** and says so.

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_143_answers_the_inversion_of_its_own_address_bits` | all 64 address values at $4100, $4140 and $5F00; A8 clear is not the port; **no write anywhere moves a window**; and the read is stateless |
| `req_core_102_mapper_144_lets_the_roms_bit_0_win_its_bus_conflict` | an `$FF` ROM byte latches the whole value; a ROM byte with bit 0 set and a written bit 0 clear gives the **ROM's** bit 0; bits 7-1 are the ordinary AND; nothing below $8000 is the register |

144 joins `req_core_090_every_single_latch_board_round_trips_with_its_latch_dirtied`; **143 is
deliberately not in that table** and the test says why — a board with no write side has no state
to dirty, so the `assert_ne!` that keeps the round-trip honest could not hold.

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **970 passed, 0 failed, 9 ignored** (+2 over step 2) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes --all-targets` | clean |

## Step 4 — `board-190`: the last of the seven, and the only one with more than one window

`LatchBoard::Zemina190` and four more bytes on `SingleLatch`. The page is one of the shortest in
the census — no capacity table at all, just a *Banks* list and two register diagrams — and three
of its statements are the ones an implementation gets wrong:

1. **The fixed window is the *first* bank.** "CPU $C000-$FFFF: 16 KiB PRG ROM, fixed to the FIRST
   bank", so the four 8 KiB windows power on as 0, 1, **0, 1** where every UxROM-like board in
   this build would show 0, 1, 30, 31.
2. **`P` is an address line.** `A~[1P0. .... .... ....]` with `D~[.... .PPP]`: A14 is bit 3 of the
   16 KiB bank number, so $8000-$9FFF writes the low eight banks and $C000-$DFFF the high eight,
   and the *same* data byte selects two different banks from the two ports. A13 set is neither
   port, which is why $E000-$FFFF reaches nothing.
3. **`bb` is the address's A1-A0** and the data is a whole 2 KiB bank number, so one byte reaches
   one of four windows.

The board is decoded **before** the single-latch `match` of `write_register` rather than inside
it, because it is the one board here with two ports; `register` carries its 16 KiB PRG bank and
the new `chr: [u8; 4]` its four CHR registers. Those four bytes are in the **family's one
savestate block** and are therefore paid by the six boards that do not use them — which is what
sharing a variant means, and is what `Jv001` already does with mapper 36's CHR latch. Every board
in the variant is new in this lot, so no existing savestate format moves.

`board_memory_sizes` gains a row: **8 KiB of PRG-RAM**, on the page's own first *Banks* line
("CPU $6000-$7FFF: 8 KiB RAM"). It is the figure REQ-CORE-012's default would give an iNES 1.0
image anyway and is stated because the page states it.

### What the tests had to notice

**The power-on CHR layout of a four-register board is not linear.** All four registers are 0, so
each 2 KiB window shows 2 KiB bank 0 and the eight 1 KiB windows read `0, 1, 0, 1, 0, 1, 0, 1` —
four copies, not `0..7`. The first run of both tests asserted a linear layout and failed on it;
the comment now says what the four zeroes mean.

| test | what it pins |
| --- | --- |
| `req_core_102_mapper_190_banks_three_windows_from_two_ports` | the **first** bank fixed at $C000; three bits from each PRG port and **A14 as the fourth**; $E000 and $FFFF are no port; the four CHR windows one at a time, including from $BFFF; the CHR port does not move the PRG bank; the 8 KiB of RAM at $6000 reads back what was written; nothing below $8000 is a register |
| `req_core_090_mapper_190_round_trips_its_prg_bank_and_its_four_chr_registers` | all **five** registers dirtied, saved, dirtied differently and restored — MF-UNL-2 step 6's shape, which is what catches a field missing from the block |

| | |
| --- | --- |
| `cargo test --release -p zax-nes -p zax-harness` | **972 passed, 0 failed, 9 ignored** (+2 over step 3) |
| `cargo fmt --check`, clippy `-D warnings` on `zax-nes --all-targets` | clean |

## Step 5 — the four sweeps, the save-state round trips, and the twelve-configuration matrix

**Eight features is eleven configurations plus the umbrella's refusal**, which is the largest
matrix any lot of the plan runs and the reason §7.2 prices this lot at eight hours.

| configuration | `cargo test -p zax-nes` | clippy `-D warnings` |
| --- | --- | :-: |
| `default` | **909 / 0 / 8** (898 / 0 / 8 at `452e8464`) | clean |
| `--no-default-features` | **783 / 0 / 8** — the figure `BOARD-FEATURES.md` records, **unchanged** | clean |
| `--features parked-boards` | **962 / 0 / 8** (951 at the base) | clean |
| `--no-default-features --features board-nina03` | 786 / 0 / 8 (+3) | clean |
| `--no-default-features --features board-38` | 785 / 0 / 8 (+2) | clean |
| `--no-default-features --features board-107` | 785 / 0 / 8 (+2) | clean |
| `--no-default-features --features board-143` | 784 / 0 / 8 (+1) | clean |
| `--no-default-features --features board-144` | 785 / 0 / 8 (+2) | clean |
| `--no-default-features --features board-145` | 785 / 0 / 8 (+2) | clean |
| `--no-default-features --features board-149` | 785 / 0 / 8 (+2) | clean |
| `--no-default-features --features board-190` | 785 / 0 / 8 (+2) | clean |
| `--no-default-features` + all eight | **794 / 0 / 8** (+11) | clean |
| `default`, both crates | **972 / 0 / 9** | clean (`zax-harness` too) |

The eight deltas sum to **+16** and all eight together is **+11**, and the difference is exactly
the tests two or more features share: `req_core_102_mappers_145_and_149_switch_chr_from_bit_7_alone`
is one test for two boards, and `req_core_090_every_single_latch_board_round_trips_with_its_latch_dirtied`
one for five.

And the umbrella still refuses to be selected alone:
`cargo build -p zax-nes --no-default-features --features board-decode-boards` stops at
`decode_boards.rs`'s own `compile_error!`, whose `any` list this lot extended by eight names.

§7.1's four sweeps — `req_core_100_every_required_mapper_is_registered`,
`req_core_102_any_other_mapper_is_refused`, `req_core_090_the_mapper_state_survives_a_save_and_restore`
and `amd_061_a_reset_puts_every_board_but_mapper_28_back_to_its_power_on_state` — hold in all
twelve. The first is the one this lot had to extend, and its table is built from `cfg!` so that a
featured number is expected exactly when its feature is on.

### Three things only the matrix found, all three in `--features board-143` alone

That configuration is the lot's degenerate case — **a board with no write side at all** — and it
is the only one of the twelve that failed anything:

1. **`DecodeBoard`'s `inner!` arm had the wrong `cfg` list.** The variant, the `use` and the
   module gate had been extended to seven features and the macro's own arm to four, so with only
   `board-143` on, ten of that enum's inner matches became non-exhaustive. The failure is loud
   (`E0004`), which is the one mercy of a shared-variant enum.
2. **A `match` all of whose arms diverge makes the code after it unreachable.**
   `SingleLatch::write_register` ended its decode with `_ => return`, and with 143 the only board
   there is no other arm — so `self.register = latched` was unreachable and `-D warnings` refused
   it. The decode now yields an `Option` and the assignment is behind `if let`, which is also the
   honest shape: this function has boards that latch nothing.
3. **A binding no arm reads is a warning too.** `update_banks` hoisted
   `let register = i32::from(self.register)` above its `match`, and mapper 143 has no register;
   each arm now reads `self.register` itself.

None of the three is reachable from `default`, which is what the matrix is for.

### The two enums' sizes

| | |
| --- | --- |
| `size_of::<Board>()` | **1376**, the figure MF-UNL-2 step 2 measured — no new discriminant |
| `size_of::<DecodeBoard>()` | **144** |
| `size_of::<SingleLatch>()` | **120** |

`SingleLatch` is **24 bytes below the enum that holds it**, so neither variant this lot adds is
the one that sets `DecodeBoard`'s size — which is why the four bytes mapper 190's CHR registers
cost are free to the other six boards and to every board MF-6c and MF-UNL-1 put in that enum.

## Step 6 — the three question tickets

Each is a place where this lot had to choose, each is implemented on the provisional choice, and
each is referenced from the code with `// Q-NNN`.

| ticket | the question | the choice |
| --- | --- | --- |
| **Q-145** | mapper 143's protection read: the census's "logical inversion of the **written** bits", or the page's inversion of the **address**'s A5-A0 | **the page.** The board has no write side at all — "an NROM circuit board with a simple form of protection", "a 74138 and a 74368", no *Banks* section — and `DDDDDD` sits inside the page's *address* pattern. A test written from the census's sentence could not have passed against any reading the page admits. The two open-bus bits read as 0, which is `Jv001`'s own compromise |
| **Q-146** | mapper 144's bus conflict: which of the page's **three** candidate formulas, and the census's expected figure, which none of them produces | **the page's own "most succinct" form**, `ROM[address] & (written \| 1)` — the same function as its Nestopia candidate; FCEUX's "ignores writes to $8000" is a workaround for one address and is not a rule. The census asks that `$FF` over a ROM byte of `$00` give `$FE`; all three candidates give `$00`, and `$FE` is a fourth reading in which bits 7-1 do not conflict at all — which the page's "almost identical to the revision B Color Dreams boards" denies. **A reviewer with a *Death Race* dump should close it**: the readings differ on every write whose bit 0 is clear |
| **Q-147** | two places a page states a possibility rather than a fact: mapper 38's "writes to $F000-$FFFF are **very likely** to also trigger a bankswitch", and mapper 107's "it's currently **unknown** whether this mapper suffers from bus conflicts" | **the page's inference** for 38 (A15 is not decoded, so `$F000-$FFFF` is the same register) and **no conflicts** for 107. The second is chosen on which error is recoverable: a board wrongly given conflicts latches `written & ROM` on every write, where one wrongly denied them runs a game that relied on the AND slightly differently — and no documentation claims any game does |

Q-146 is the one a reviewer should take first. It is also the one that forced a structural note:
**mapper 144 is the first board in this build whose bus conflict is not the generic AND**, so it
is deliberately absent from `registry::has_bus_conflicts` — that function applies its `bool` in
`Cartridge::cpu_write` *before* `Mapper::write_register` sees the value, which would destroy the
bit 0 this board has to take from the ROM. If a second such board appears, the shape to consider
is a per-board enum on `Cartridge::bus_conflicts` rather than a second `bool`; the ticket says so.
