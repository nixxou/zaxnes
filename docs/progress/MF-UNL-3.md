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

- [ ] **0/9 — the plan.** This file: the nine numbers, the page each is built from, the eight
  features, the shape each board takes in this tree, and the **five** places this lot leaves the
  census's own prescription for the number's page.
- [x] **1/9 — `board-nina03`.** `Nina03`, the $4100 control register for 79 and 146, its
  `registry.rs` arm, `SUPPORTED_MAPPERS`, `board_memory_sizes`, and the tests the census lists —
  including the anti-test for the FCEUX CNROM switch this lot deliberately leaves out.
- [ ] **2/9 — `SingleLatch`, and the four plain latches: 38, 107, 145, 149.** One
  `SmallBoard`-style `DecodeBoard` variant holding the seven remaining boards as a runtime field,
  on the `Jv001` precedent, with four of the seven wired and tested.
- [ ] **3/9 — `board-143` and `board-144`,** the two boards of that variant that are not a bank
  latch: 143's protection read (through the `try_read_prg` forward `board-jv001` already added)
  and 144's resistor rule.
- [ ] **4/9 — `board-190`,** the last of the seven and the only one with more than one window.
- [ ] **5/9 — the four sweeps, the save-state round trip and the feature matrix.** Each of the
  eight features alone, all eight, none, `default` and `--features parked-boards`; `cargo fmt
  --check` and clippy `-D warnings` in each.
- [ ] **6/9 — the documentation.** Eight rows in `docs/review/BOARD-FEATURES.md`, the Cargo
  feature comments, and the question tickets.
- [ ] **7/9 — the callgrind campaign.** PR-1's thirteen ROMs and level 2's five games against
  `452e8464`, every compared commit built from `git archive` in a fresh directory, and the feature
  decision for each of the eight boards.
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
