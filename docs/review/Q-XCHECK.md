# Q-XCHECK — the thirty-two open MF-UNL tickets cross-checked against four other emulators

**Role**: R3 (clean side), review lot **Q-XCHECK**, 2026-11-08.
**Scope**: every ticket from **Q-140 to Q-172** except **Q-153** (audio output, not a mapper) — thirty-two
tickets, raised by the twelve MF-UNL lots.
**Sources read**: `spec/changes/Q-1NN.md`; `rust/crates/zax-nes/src/cartridges/`;
`docs/review/UNL-census.md`; the nesdev wiki (including `?action=raw` for the bit diagrams whose
**columns** carry the reading, which is the method Q-172 §0 asks the next lot to inherit); and the
four GPL emulators under `third_party/`:

| short name | tree | what it is |
| --- | --- | --- |
| **Mesen 2** | `third_party/mesen2` | C++, one class per board under `Core/NES/Mappers/` |
| **FCEUX** | `third_party/fceux` | C, one `*_Init` per board under `src/boards/` |
| **Nestopia UE** | `third_party/nestopia` | C++, one `Board` subclass under `source/core/board/` |
| **puNES** | `third_party/punes` | C, one `mapper_NNN.c` per number under `src/core/mappers/` |

**No Rust code was changed by this lot.** Each ticket file gained one appended
`## Cross-check (Q-XCHECK)` section with its verdict and a link here.

## Licence note

All four projects are GPL (Mesen 2 and FCEUX GPL-2.0-or-later, Nestopia UE GPL-2.0, puNES
GPL-2.0-or-later) and were **read, not copied**. No line of any of them was transcribed into this
repository — not into the Rust sources, not into tests and not into this report. Where a formula
had to be named, it is described in prose or in this project's own notation, with a
`third_party/<project>/<path>:<line>` citation so a reader can go and look. The citations are
pointers, not quotations.

## How the verdicts are to be read

* **CONFIRMED** — this build matches the number's own page **and** at least the majority of the
  four, or the dissenters are visibly the older/less accurate code.
* **CHANGE** — the page looks wrong: most or all four agree against it, with a reason. The exact
  change is spelled out in §CHANGE below.
* **OPEN** — the four are split, silent, or unanimous-but-unreasoned (typically because they share
  one ancestor's expression). The current choice stands and the evidence that would close it is
  named.

One recurring caution, stated once here because it governs half the verdicts: **the four are not
four independent witnesses.** For the unlicensed MMC3 clones in particular, Mesen 2's class,
Nestopia's board and puNES's `mapper_NNN.c` are frequently the *same arithmetic expression* as
FCEUX's `src/boards/` file, character for character modulo language. Where that is so it is said,
and the tally is reported as one lineage rather than as three or four votes.

---

## Summary table

Legend: **page** = the number's own nesdev page; a blank cell means the project does not implement
that point at all; "n/s" = the project does not support the number.

| ticket | mapper(s) | point | page | this build | Mesen 2 | FCEUX | Nestopia | puNES | verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Q-140 | 173 | bus conflicts on the `$8000` write | none | none | none | none | none | none | **CONFIRMED** |
| Q-141 | 150, 243 | R7 mode 3 "single-screen, page 1" | page 1 (`S1`) | `SingleScreenB` | page 0 | page 0 | page 0 | page 1 | **OPEN** |
| Q-141 | 150, 243 | R7 modes 0/1/2 | `(0,0,0,1)` / H / V | same | same | V / H / `(0,1,1,1)` | V / H / `(0,1,1,1)` | same | **CONFIRMED** |
| Q-142 | 195 | is the CHR-RAM window the run-time GAL? | yes | yes | fixed `$00-$03` | fixed `$00-$03` | fixed `$00-$03` | **yes** | **CONFIRMED** |
| Q-142 | 195 | the GAL's row field | bits 6, 3, 1 | bits 6, 3, 1 | — | — | — | bits 6, 3, 1 | **CONFIRMED** |
| Q-142 | 195 | latched by a PPU write to a CHR-ROM window | yes | yes | — | — | — | **yes** | **CONFIRMED** |
| Q-142 | 195 | power-on row | `$80` (`$28-$2B`) | `$80` | `$00-$03` | `$00-$03` | `$00-$03` | `$82` (`$00-$03`) | **OPEN** |
| Q-142 | 195 | RAM chip size | 8 KiB | 8 KiB | 4 KiB | 4 KiB | header's | 8 KiB (+ a per-CRC override) | **CONFIRMED** |
| Q-142 | 195 | 4 KiB PRG-RAM at `$5000` | named | not built | — | **built** | — | **built** | **CHANGE (gap)** |
| Q-143 | 74, 192 | NES 2.0 byte 11 overrides the board's chip | 192's page says yes | no | no | no | no | no (but gates 74 on NES 2.0) | **CONFIRMED** |
| Q-144 §1 | 123 | the `$5800` bank field's bit 1 | reg bit **4** (diagram) / bits 0-3 (prose) | reg bit 4 | reg bit **3** | reg bit **3** | reg bit **3** | reg bit **3** | **CHANGE** |
| Q-144 §2 | 114, 182 | address map | one-to-one (two submappers) | one-to-one | scrambled, non-bijective | scrambled, non-bijective | third shape | **one-to-one** | **CONFIRMED** |
| Q-144 §3 | 115 | `$6000` bit 6 = PRG A18 in both modes | yes | yes | — | — | — | — | **OPEN** |
| Q-144 §4 | 121, 187 | CHR A18 source | PPU A12, always | PPU A12, always | the 2 KiB registers' windows | the 2 KiB registers' windows | the same (187); absent (121) | the same | **OPEN** |
| Q-144 §5 | 115, 187, 121 | Mesen's `$5080`, `$6000` alias, `$5180=$80` | silent | not built | built | `$5080` built | — | — | **CONFIRMED** |
| Q-145 | 143 | the protection read answers | `~address` bits 5-0 | `~address` | `~addr` | `~A` | `~address` | `~address` | **CONFIRMED** |
| Q-145 | 143 | a write side | none | none | none | none | none | none | **CONFIRMED** |
| Q-146 | 144 | the bus-conflict formula | `ROM & (value \| 1)` | same | same (generic AND + bit 0) | `$8000` dropped, no AND | same | same | **CONFIRMED** |
| Q-146 | 144 | the census's `$FE` | — | rejected | rejected | rejected | rejected | rejected | **CONFIRMED** |
| Q-147 §1 | 38 | does `$F000-$FFFF` bank too? | "very likely" | yes | no | no | no (but `$6000-$7FFF` does) | no | **OPEN** |
| Q-147 §2 | 107 | bus conflicts | "unknown" | none | none | none | none | none | **CONFIRMED** |
| Q-147 §2 | 107 | register width | 8 bits | 8 bits | 8 bits | 2/3 bits | 8 bits | the **address**, not the datum | **CONFIRMED** |

*(rows for Q-148 onward are appended by the later batches of this lot)*

---

## Q-140 — mapper 173 has no bus conflicts

**The point**: `UNL-census.md` §6.2 and §7.2 give mapper 173 bus conflicts; the number's own page
has no such line and argues the opposite (the game "will not run as Mapper 3 on emulators that
emulate bus conflicts … nor will it run on an actual Nintendo CNROM board"). This build gives it
none (`registry::has_bus_conflicts`, which names the absence in its own comment).

**The four**:

* **Mesen 2** — `Txc22211C` (`third_party/mesen2/Core/NES/Mappers/Txc/Txc22211C.h:5-21`) overrides
  nothing but `UpdateState`, and `BaseMapper::HasBusConflicts` defaults to false
  (`.../Core/NES/BaseMapper.h:122`). The complete list of boards that do override it
  (`grep HasBusConflicts Core/NES`) has no Txc entry. **No conflicts.**
* **FCEUX** — `src/boards/01-222.cpp:62-66`: the `$8000-$FFFF` handler stores the byte into `cmd`
  and re-syncs; for mapper 173 `Sync` reads `reg[2]` and never looks at `cmd`, so the written value
  is unused and no AND is applied. FCEUX's bus-conflict machinery is a per-board flag passed to
  `Latch_Init` (`src/boards/datalatch.cpp:64`) which this board does not use at all.
  **No conflicts.**
* **Nestopia UE** — `source/core/board/NstBoardTxc.cpp:101-106`: `T22211A::Poke_8000` is declared
  `NES_POKE` (no data parameter) and reloads both windows from `regs[2]`. Nestopia applies bus
  conflicts per board through `GetBusData` (as its mapper-11 and mapper-144 boards do), and this
  one does not call it. **No conflicts.**
* **puNES** — `src/core/mappers/mapper_173.c:36-38` forwards the written value, masked to its low
  nibble, straight to the shared TXC chip model; nothing reads PRG-ROM under the write.
  **No conflicts.**

**Verdict: CONFIRMED.** Four for four with the page, against the census row. The census's sentence
should be corrected (its §6.2 roster and its MF-UNL-1 test list), which the ticket already asks
for. Worth noting for the record that all four also agree with the ticket's *reason*: on this board
the `$8000-$FFFF` write does not use the value at all, so even a board that did have conflicts
would only pay a wasted ROM read.

## Q-141 — "single-screen, page 1" on mappers 150 and 243

**The point**: R7 mode 3. This build reads "page 1" as `S1` (the second CIRAM page,
`NametableMirroring::SingleScreenB`), the page's own `S0`/`S1` notation being introduced four lines
above.

**The four**, and the first thing to notice is that they do not share *one* table:

| | mode 0 | mode 1 | mode 2 | mode 3 |
| --- | --- | --- | --- | --- |
| the page | `(0,0,0,1)` | Horizontal | Vertical | "single-screen, page 1" |
| this build | `(0,0,0,1)` | Horizontal | Vertical | `S1` |
| Mesen 2 | `(0,0,0,1)` | Horizontal | Vertical | **`ScreenAOnly`** |
| FCEUX | **Vertical** | Horizontal | **`(0,1,1,1)`** | **`MI_0`** |
| Nestopia UE | **`(0,1,0,1)`** | `(0,0,1,1)` | **`(0,1,1,1)`** | **`(0,0,0,0)`** |
| puNES | `(0,0,0,1)` | Horizontal | Vertical | **`S1`** |

* **Mesen 2** — `Core/NES/Mappers/Sachen/Sachen74LS374N.h:47-50`, one class for both numbers, the
  field read as `(_regs[7] >> 1) & 3` as the page draws it.
* **FCEUX** — `src/boards/sachen.cpp:26-32` (`S74LS374MSync`), shared by the 74LS374N, the 74LS374NA
  **and all four 8259 types**. Mapper 150 latches `V >> 1` (`:51`) and mapper 243 latches `V & 1`
  (`:112`), so on FCEUX mode 3 is unreachable on 243 at all.
* **Nestopia UE** — `source/core/board/NstBoardSachen74x374.cpp:211-222`, a 4×4 look-up table in the
  same order as FCEUX's `switch`; mapper 150 is `S74x374b` and indexes it with `data >> 1 & 3`
  (`:295`), mapper 243 is `S74x374a` and indexes it with `data & 1` (`:263`).
* **puNES** — `src/core/mappers/mapper_150.c:101-116`, and `src/core/mappers.c:721-723` sends mapper
  243 to the same `map_init_150`, so **both** numbers get the page's four modes with mode 3 =
  `mirroring_SCR1`.

**Verdict on modes 0-2: CONFIRMED** — page, this build, Mesen 2 and puNES agree; FCEUX and Nestopia
carry a table whose modes 0 and 2 are *also* wrong against the page, which is the FCEU-mm ancestry
the 8259 page itself warns about ("gathered primarily from FCEU-mm … likely incomplete").

**Verdict on mode 3: OPEN.** The raw tally is 3-1 against this build, but two of those three
(FCEUX, Nestopia) dissent from the page on half the table as well, so their mode-3 cell is not
independent evidence about mode 3 — it is the same inherited table. Stripped of the lineage the
split is **Mesen 2 against puNES, one each**, and the page's internal argument (it names `S0` and
`S1` in the row above) is unrebutted. Keep `SingleScreenB`.

**What would close it**: a dump of any of the fifteen Sachen titles observed with mode 3 selected —
the symptom is a blank or stale screen rather than a subtly wrong one, so one frame decides it — or
a revision of either page that spells the page number out. The cost of being wrong is one
identifier in `Sachen8Reg::update_mirroring`.

## Q-142 — mapper 195 (Waixing FS303): the GAL, its power-on row and its chip

This is the ticket the cross-check changed the most, because **puNES implements the page's GAL** and
nothing in the census or the ticket knew that.

* **Mesen 2** — `Core/NES/MapperFactory.cpp:468` registers `MMC3_ChrRam(0x00, 0x03, 4)`, and
  `Core/NES/Mappers/Mmc3Variants/MMC3_ChrRam.h` is a **fixed** first/last RAM bank pair plus a
  hard-coded KiB count. No register, no GAL, no power-on row.
* **FCEUX** — `src/boards/mmc3.cpp:1029-1033` (`M195CW`): banks `0-3` go to CHR-RAM, everything else
  to CHR-ROM, with `CHRRAMSIZE = 4096` fixed at `:1047`. Also fixed. FCEUX *does* implement the
  `$5000-$5FFF` 4 KiB PRG-RAM window the page names (`:1036-1041`, `setprg4r(0x10, 0x5000, 2)` plus
  read and write handlers).
* **Nestopia UE** — `source/core/board/NstBoardWaixing.cpp:284-287`, `TypeE::GetChrSource` is
  `bank <= 3`. Fixed, and the chip's size is whatever the image declares.
* **puNES** — `src/core/mappers/mapper_195.c:76-133`. This is the page's board:
  * the register is latched on a **PPU write**, in a `wr_chr` hook, and only when the bank the
    CHR-**ROM** currently drives in that window has bit 7 set — the page's "Writes to CHR ROM" and
    its "Must be 1" bit, read exactly as this build reads them;
  * the row is selected by a **three-bit field at register bits 6, 3 and 1** (`:90`), which is this
    build's `(Z, L, L)` to the bit;
  * the eight rows are the page's eight (`$80`→`$28`, `$82`→`$00`, `$88`→`$4C`, `$8A`→`$64`,
    `$C0`→`$46`, `$C2`→`$7C`, `$C8`→`$0A`, `$CA`→ROM only), and bit 4 ("always enable CHR ROM") is
    honoured at `:111-114`;
  * the window's width follows bit 6 (`:116`, four banks against two), i.e. the page's "Number of
    banks of CHR RAM, 0=4KiB, 1=2KiB";
  * and the offset inside the chip is the bank number wrapped by the chip's own size (`:130`),
    which is the page's worked example ("`$80` … selects … the same 4 KiB as setting `$82` …
    because CHR A13 is not connected") **only on an 8 KiB chip**, which is this build's reading.

**Verdicts**:

* **the GAL exists: CONFIRMED.** The page and puNES against three fixed-window boards. The three
  are not evidence against the GAL: each registers the `$82` row, which is exactly the hacked
  *Captain Tsubasa II* images the page describes, and puNES keeps a **CRC32 special case** for that
  very title (`:106-108`) — i.e. puNES implements the GAL *and* has had to pin one game to `$82`,
  which is the clearest possible corroboration that the GAL is real and that the hacked dumps are a
  separate problem.
* **the bit field (6, 3, 1): CONFIRMED.** Independently identical in puNES.
* **the latch being a PPU write to a CHR-ROM window: CONFIRMED.** Independently identical in puNES,
  which also answers the ticket's own open question ("a source that says the GAL latches on
  something other than a PPU write to CHR-ROM").
* **the 8 KiB chip: CONFIRMED** — puNES's arithmetic needs the same three low bits.
* **the power-on row: OPEN.** The page says `$80`; puNES powers on at mask `0xFC`/compare `0x00`
  (`:53`, `:109`), i.e. the **`$82`** row, and the other three are permanently `$82`. So no
  implementation starts at `$28-$2B`. Nothing rebuts the page's "Power-on value: $80" — but four for
  four start at `$00-$03`, and the ticket's own falsifier ("a game that never writes the register
  and expects `$00-$03`") is apparently what the other implementers assumed. Keep `$80` — it is the
  weaker claim, since any game that writes `CHRB` is unaffected — and note that the only images
  known to depend on the difference are the hacked ones, which write `$82` themselves. A dump that
  boots under `$82` and not under `$80` closes it.
* **the `$5000-$5FFF` 4 KiB PRG-RAM window: CHANGE (a gap, not a wrong reading).** The page names the
  compatible board that has it (*天神之剑*), this build does not implement it, and **two of the four
  do** — FCEUX unconditionally (`src/boards/mmc3.cpp:1036-1041`) and puNES unconditionally
  (`mapper_195.c:134-137`, `memmap_auto_4k(0, MMCPU(0x5000), 2)`). See §CHANGE 2.

## Q-143 — mappers 74 and 192: no NES 2.0 CHR-RAM override

**The point**: 192's page says "If the NES 2.0 header is present, 74 and 192 should be emulated
identically, as NES 2.0 specifies CHR RAM size elsewhere". This build ignores that and gives each
number the chip its own page's *Size of RAM* column states.

**The four** — none of them resizes the redirect window from a header field:

* **Mesen 2** — `Core/NES/MapperFactory.cpp:366` and `:465`: `MMC3_ChrRam(0x08, 0x09, 2)` for 74 and
  `(0x08, 0x0B, 4)` for 192, the KiB count a constructor argument.
* **FCEUX** — `src/boards/mmc3.cpp:710-714` (74: banks 8 and 9, `CHRRAMSIZE = 2048` at `:720`) and
  `:992-999` (192: banks 8 to 11, `CHRRAMSIZE = 4096` at `:1004`).
* **Nestopia UE** — `source/core/board/NstBoardWaixing.cpp:264-267` (`TypeA`: `bank - 8 <= 1`) and
  `:274-277` (`TypeC`: `bank - 8 <= 3`). The window is the board's; only the chip behind it comes
  from the image.
* **puNES** — `src/core/mappers/mapper_074.c:53-60` and `mapper_192.c:53-59`. The windows are fixed
  per number as well. puNES does read the header, but for a **different** question: mapper 74's
  CHR-RAM is served **only on an NES 2.0 image** (`mapper_074.c:54`, `info.format != NES_2_0` →
  write-protected CHR-ROM and no RAM at all) and only if the header declares VRAM.

**Verdict: CONFIRMED.** Four for four with this build's rule ("the board's chip, unconditionally"),
and with `board_memory_sizes`'s own contract. puNES's extra condition is the nearest thing to the
192 page's sentence that anyone implements, and it is the opposite shape: it makes an **iNES 1.0**
mapper-74 image lose its RAM, not an NES 2.0 one gain any. Nothing in the four would change a byte
of this build. The 192 page's sentence is best read, as the ticket reads it, as advice to emulator
authors rather than a description of silicon.

## Q-144 — mapper 123 and three more places this family's page and Mesen 2 disagree

### §1 — the `$5800` bank field's bit order (mapper 123) — **the one with a visible effect**

The ticket's reading of the diagram is right, and the diagram is **not** the whole of the page.
Read from `?action=raw`, the register is

```
 D~7654 3210
   ---------
   .M31 .2S0
```

with the field line's nine characters column-aligned under the header's, so `M` is bit 6, `3` bit 5,
`1` bit **4**, `2` bit 2, `S` bit 1, `0` bit 0 — and register bit 3 is a `.`. The first annotation
row's `+` marks and the two vertical lines that descend from it to `M`'s and `S`'s own annotations
are consistent with exactly that field (bits 5, 4, 2, 0, with `|` passing through bit 6 and bit 1).
So the ASCII art says **bank bit 1 ← register bit 4**, which is what
`Mmc3Scramble::apply_override`'s `Port5800` arm implements (`(register >> 3) & 0x02`).

**But the same page's prose says the other thing, twice.** Bit 6's own annotation is "0: Use PRG
bank from MMC3; **ignore `$5800` bits 0-3/5** / 1: Ignore PRG bank from MMC3; **apply `$5800` bits
0-3/5**". Bits 0-3 and 5 is bits 0, 1, 2, 3, 5 — `S` at bit 1 plus bank bits at 0, 2, 3 and 5, and
**bit 4 unused**. Under the diagram's columns the same sentence would have had to read "bits 0-2/4-5".
The page contradicts itself, and the contradiction is between its art and its words.

**All four emulators implement the prose**, with the same expression:

| project | citation | what it does |
| --- | --- | --- |
| Mesen 2 | `Core/NES/Mappers/Mmc3Variants/MMC3_123.h:14` | keeps register bits 0 and 2 in place, moves bit 5 down two to bank bit 3 and **bit 3 down two to bank bit 1** |
| FCEUX | `src/boards/h2288.cpp:28` | the identical expression |
| Nestopia UE | `source/core/board/NstBoardKayH2288.cpp:111` | the identical expression, written as two shifted masks |
| puNES | `src/core/mappers/mapper_123.c:100` | the identical expression |

Four for four, and genealogically one: FCEUX's `h2288.cpp` is CaH4e3's 2005 file and the other three
are the same arithmetic. So the raw tally overstates the independence. What does *not* overstate it
is this: **these four emulators run *Mortal Kombat 3* (UNL-H2288) and *Earthworm Jim 2***. A wrong
bit in a 16 KiB PRG bank field is not a subtle rendering difference — it is a jump into the wrong
bank — so a reading that is wrong here does not produce a playable game. Four shipping
implementations of a playable game, plus the page's own prose, against one column reading of one
ASCII diagram.

**Verdict: CHANGE.** See §CHANGE 1 for the exact edit. This supersedes the ticket's provisional
choice; the ticket itself names the falsifier ("a dump … run far enough to write `$5800` with bits 3
and 4 unequal") and the four emulators' own game compatibility is the nearest available substitute
for that dump.

### §2 — mappers 114 and 182: the one-to-one address map

The page's *Notes* (fetched verbatim) states the refutation the ticket quotes, and gives the two
submappers' tables. Submapper 0 is `$8000→$A001, $8001→$A000, $A000→$8000, $A001→$C000, $C000→$8001,
$C001→$C001, $E000→$E000, $E001→$E001`; submapper 1 differs in four rows. That is exactly
`Mmc3Scramble::ADDRESS_SCRAMBLE` rows 1 and 2, and exactly `INDEX_SCRAMBLE` rows 1 and 2.

* **Mesen 2** — `MMC3_114.h:40-67`: `$8001→$A000`, `$A000→$8000` **and** sets a flag, `$C000` is
  *gated on that flag* (`:56`), `$A001` and `$C001` reach the IRQ latch/reload directly, `$E000`
  clears. Not a bijection. `MMC3_182.h:31-35`: `$C001` writes **both** MMC3 `$C000` and `$C001`, and
  there is no `$A001` arm — i.e. precisely the two claims 114's page refutes by name.
* **FCEUX** — `src/boards/mmc3.cpp:744-746` is Mesen's 114 (the gate on `cmdin` at `:746`), and
  `src/ines.cpp:714` **comments mapper 182 out** — "Deprecated, dupe" — so FCEUX runs 182 ROMs as
  114, which is what 114's page says FCEUX does. FCEUX forces the MMC3A counter (`isRevB = 0`,
  `:773`), which the page requires.
* **Nestopia UE** — a third shape again. `NstBoardSupergameLionking.cpp` maps whole 8 KiB blocks
  (`$8000-$9FFF` → mirroring, `$A000-$BFFF` → bank select, `$C000-$DFFF` → bank data gated at
  `:116`), and `$E003` writes **three** MMC3 registers at once (`:123-127`). Mapper 182 is a
  separate, standalone board (`NstBoardHosenkan.cpp:55`, `:115`) that is not an MMC3 at all and
  whose `$E003` is its whole IRQ interface.
* **puNES** — `src/core/mappers/mapper_114.c:66-69` holds **both** of the page's permutation tables
  as a `[2][8]` array indexed by submapper, and `:70-75` applies the index permutation on top;
  `mapper_182.c:25` is one line, `map_init_114()`. Byte for byte the page's tables and this build's.

**Verdict: CONFIRMED.** The page *argues* this one (the *Aladdin* / *Super Aladdin* comparison) rather
than asserting it, puNES implements it exactly, and the three dissenters are the lineage the page
names and refutes. Nothing to change. Worth recording that puNES is independent corroboration of
this build's two tables down to the submapper split, which no other project makes.

### §3 — mapper 115's `$6000` bit 6, "PRG A18"

* **Mesen 2** — `MMC3_115.h:35-41` reads bit 7 (mode), bit 5 (NROM-256) and bits 3-0 (bank). Bit 6
  is never read.
* **FCEUX** — `src/boards/mmc3.cpp:784-792`, the same three fields, with the comment "real hardware
  tests, info 100% now lol" against the bit-5 branch. Bit 6 never read; and `GenMMC3_Init(info, 128,
  512, …)` caps PRG at 128 KiB, where A18 cannot reach.
* **Nestopia UE** — `NstBoardKasing.cpp:89-90`: with bit 7 set it swaps a 32 KiB bank from
  `exRegs[0] >> 1` and has **no NROM-128 mode at all**. Bit 6 therefore lands inside the bank number
  by accident, as PRG A20 rather than A18 — not the page's line.
* **puNES** — `src/core/mappers/mapper_115.c:96-105`: bit 7, bit 5, bits 3-0. Bit 6 never read.

**Verdict: OPEN.** No project implements the line, but none of them contradicts it either: the page's
own six Gouder titles are 256 KiB or smaller, where the bit wraps away, and three of the four cap
the board's PRG below the point where it could matter. This is unanimous silence on an unobservable
bit, not a finding. Keep the current choice (the bit is above the MMC3's own lines, so it is driven
in both modes — which is what the annotation's missing "If M=1" qualifier says). **What would close
it**: a mapper-115 image larger than 256 KiB. None is known.

### §4 — mappers 121 and 187: CHR A18 from PPU A12

The page's sentence, fetched verbatim from mapper 187's page: "CHR A18 (the second half of CHR-ROM)
is directly selected by PPU A12, regardless of what the MMC3 is set to do." and "This means that the
'left' pattern table always uses the first and the 'right' pattern table always uses the second 256
KiB half of CHR-ROM." `Mmc3Scramble::apply_chr_a18` sets A18 on the upper half of the windows — the
right pattern table — which is that sentence.

* **Mesen 2** — `MMC3_121.h:52` and `MMC3_187.h:26`, the `_chrMode`-dependent test the ticket quotes.
* **FCEUX** — `src/boards/121.cpp:51-52` and `src/boards/187.cpp:25-26`:
  `(A & 0x1000) == ((MMC3_cmd & 0x80) << 5)`. Read against `FixMMC3CHR`'s own
  `cbase = (V & 0x80) << 5` (`src/boards/mmc3.cpp`), that condition selects exactly **the four
  windows the two 2 KiB registers drive**, whichever pattern table the CHR-inversion bit has put
  them in.
* **Nestopia UE** — `NstBoardUnlKingOfFighters96.cpp:88-89`, the same condition for 187. For **121**
  (`NstBoardKayPandaPrince.cpp`) there is no `UpdateChr` override at all: Nestopia does not implement
  the line for that number.
* **puNES** — `mapper_187.c:101` and `mapper_121.c:120`, the same condition again (and on 121 only
  when the image is ≤256 KiB of PRG, the larger A9713 taking its A18 from `$5180` instead).

So all four implement one expression, and it differs from the page **twice**: it is
register-dependent where the page says "regardless", and its polarity is inverted (with the CHR
inversion clear — the ordinary state — the four give the *left* table the second half, where the page
gives it the first).

**Verdict: OPEN, and this is the open point most worth a reviewer's ROM.** The page states a wiring
and gives a physical reason; the four state a register dependence and give none, and are one
lineage. But the polarity half is empirically testable without any hardware: *Street Fighter Zero 2
'97* (121) and *The King of Fighters '96* (187) are the only two images with 512 KiB of CHR, both run
in all four emulators, and under this build's reading every tile in one pattern table comes from the
other half of the chip. If those two titles render correctly in the four and incorrectly here, the
page's sentence is wrong and §CHANGE 3 is the edit. **What would close it**: either dump, one frame.

### §5 — the three things of Mesen 2's that are on no page

The ticket records them so a reader does not take them for omissions. Confirmed, with their origin:

* **115's `$5080`** — `MMC3_115.h:53` is FCEUX's `src/boards/mmc3.cpp:794-803` (`M115Write`'s
  `A == 0x5080` arm and the `$5000-$5FFF` read handler returning it). Neither Nestopia nor puNES has
  it. So it is one FCEUX behaviour inherited once, not a consensus. This build's open bus stands.
* **187's `$6000`** — `MMC3_187.h:71` accepts the override at `$5000` **or** `$6000`. FCEUX
  (`src/boards/187.cpp`), Nestopia and puNES decode `$5000` only. This build's `$5000`-only decode
  matches three of the four.
* **121's `$5180` power-on `$80`** — `MMC3_121.h:25` (`_exRegs[3] = 0x80`, labelled a hack for
  *Super 3-in-1* in its own source). No other project does it. This build's zero stands.

All three: **CONFIRMED** as absences.

## Q-145 — mapper 143 answers the inversion of its own address

**The four are unanimous and so is the page**; the census's "logical inversion of the written bits"
has no support anywhere.

* **Mesen 2** — `Core/NES/Mappers/Sachen/Sachen_143.h:24`: the read answers the complement of the
  low six bits of the **address**, with bits 7-6 forced to `01`. The class has no `WriteRegister`.
* **FCEUX** — `src/boards/sachen.cpp:389-394` (`TCA01Read`): the complement of the **address**'s low
  six bits with the real data bus in bits 7-6, and `TCA01Power` installs **no write handler at all**.
* **Nestopia UE** — `source/core/board/NstBoardSachenTca01.cpp:65`: identical to Mesen's, bits 7-6
  forced to `01`. No poke map.
* **puNES** — `src/core/mappers/mapper_143.c:35` (complement of the address, open bus in bits 7-6)
  and `:31`, whose write handler is **an empty function body**.

**Verdict: CONFIRMED**, both halves — the read is a function of the address, and there is no write
side. Two notes for the record:

* on the **two undriven bits**, the four split three ways: `01` (Mesen 2, Nestopia), true open bus
  (FCEUX, puNES), and this build's `00`. Nothing a game can do distinguishes them under the one use
  the page describes, and the ticket already records the compromise; but if `Mapper::try_read_prg`
  ever gains the data bus, `01` has two votes and open bus two.
* puNES additionally answers `$FF` for `$5000-$5FFF` reads with A8 **clear** (`:36-38`), which no page
  describes; this build leaves them to the default path, and so do the other three.

## Q-146 — mapper 144's bus-conflict formula

**The point**: the page offers three candidate formulas and calls its own the most succinct;
`SingleLatch::write_register`'s `Agci144` arm implements `ROM[address] & (value | 1)` and mapper 144
is deliberately absent from `registry::has_bus_conflicts`.

* **Mesen 2** — `Core/NES/Mappers/Unlicensed/ColorDreams.h:10` gives the class bus conflicts, so
  `BaseMapper::WriteRam` (`Core/NES/BaseMapper.cpp:884-895`) ANDs the written byte with the PRG byte
  under the address **before** `WriteRegister` runs, and `:22` then ORs the ROM byte's bit 0 back in.
  Composed: `(value & ROM) | (ROM & 1)`.
* **FCEUX** — `src/boards/datalatch.cpp:231`: mapper 11's sync function with the register window
  starting at **`$8001`** and the bus-conflict flag **off**. So a write to `$8000` is dropped and
  every other write latches unchanged. This is the page's "FCEUX 2.1.5 ignores writes to `0x8000`"
  still in the tree, and it is not a rule about hardware.
* **Nestopia UE** — `source/core/board/NstBoardAgci.cpp:47-48`: it fetches the ROM byte under the
  address and computes `(data & ROM) | (ROM & 1)`.
* **puNES** — `src/core/mappers/mapper_144.c:48`: it fetches the PRG-ROM byte and computes
  `ROM & ((value & ROM) | 1)`.

Mesen 2's, Nestopia's, puNES's and this build's four expressions are **the same function**: bit 0 is
the ROM's whatever the CPU drove, and bits 7-1 are the ordinary Color Dreams AND. (`ROM & (v|1)`,
`(v & ROM) | (ROM & 1)` and `ROM & ((v & ROM) | 1)` agree on every input.)

**Verdict: CONFIRMED**, on both halves of the ticket:

* the formula — three of the four compute exactly what this build computes, and the fourth is the
  page's own named workaround;
* the census's expected `$FE` — **no implementation anywhere produces it.** A written `$FF` over a
  ROM byte of `$00` gives `$00` in this build, in Mesen 2, in Nestopia and in puNES, and leaves the
  register untouched in FCEUX. The census's test line should be corrected.

A third point the ticket makes and the four support: the conflict has to be computed **inside the
board**, because the generic AND destroys bit 0 first. Mesen 2 is the exception that proves it — it
*can* use its generic path only because `WriteRegister` runs afterwards and can put bit 0 back.

## Q-147 — mappers 38 and 107, where the page declines to assert

### §1 — mapper 38: does `$F000-$FFFF` bank as well?

The page's own verb is "very likely", with a complete hardware argument (the 74138 enables on
/ROMSEL and R/W and decodes A14, A13 and A12 but **not** A15). This build decodes
`(address & $7000) == $7000`, so `$7000-$7FFF` and `$F000-$FFFF` are one register.

* **Mesen 2** — `Core/NES/Mappers/Unlicensed/UnlPci556.h`, `RegisterStartAddress` `$7000` /
  `RegisterEndAddress` `$7FFF`. `$F000` is not a register.
* **FCEUX** — `src/boards/datalatch.cpp:268`, the latch window is `$7000-$7FFF`.
* **Nestopia UE** — `source/core/board/NstBoardDiscrete.cpp:51`, `$6000-$7FFF`: **wider than the
  page at the bottom** (A12 not decoded) and still nothing above `$8000`.
* **puNES** — `src/core/mappers/mapper_038.c:47`, `$7000-$7FFF` exactly.

**Verdict: OPEN.** All four decline the page's inference, but none of them argues against it: three
implement the documented range and the fourth implements a *different* undocumented widening
(Nestopia's `$6000-$6FFF`, which this build and the page both exclude). Unanimity without a reason,
against a page that gives one, is not a finding. Keep the current decode; the risk is
one-directional (a game storing to `$F000-$FFFF` for another purpose would bankswitch) and the page's
own mitigation is that *Crime Busters* is the number's one game and "well behaved code has no reason
to". **What would close it**: a *Crime Busters* trace showing a write in `$F000-$FFFF`, or a board
photograph of the UNL-PCI556's 74138 enable wiring. Changing it is one constant in `PORT_MASK`'s
place, as the ticket says. Nestopia's `$6000-$6FFF` widening is worth one line of thought if a dump
ever appears, but no page supports it.

### §2 — mapper 107: bus conflicts, and the register's width

* **bus conflicts** — none of the four gives the number any: Mesen 2's `Mapper107`
  (`Core/NES/Mappers/Unlicensed/Mapper107.h`) overrides no `HasBusConflicts`; FCEUX passes
  `busc = 0` (`src/boards/datalatch.cpp:392`); Nestopia's `MagicDragon`
  (`source/core/board/NstBoardMagicSeries.cpp:44-48`) does not call `GetBusData`; puNES's
  `mapper_107.c` applies no AND. **CONFIRMED** — four for four with this build, and the page's
  "currently unknown" is answered the same way by everyone.
* **the register's width** — the page gives a whole byte, PRG from bits 7-1 and CHR from bits 7-0.
  Mesen 2 `Mapper107.h:19-20` and Nestopia `NstBoardMagicSeries.cpp:46-47` both compute exactly
  that. FCEUX masks to two PRG bits and three CHR bits (`src/boards/datalatch.cpp:386-389`), which
  is the page's named third position and is simply the known image's size. **puNES takes the written
  *address* and ignores the datum** (`mapper_107.c:45`, whose `value` parameter is declared
  `UNUSED`), which is a fourth reading no page supports and which this report flags as a probable
  slip in puNES rather than as evidence. **CONFIRMED**: this build matches the page, Mesen 2 and
  Nestopia.

---

## CHANGE — ready-to-use fix descriptions for a later lot

*(No code was written by this lot. Each entry names the file, the function, the edit, the titles
affected and the risk to existing boards.)*

### CHANGE 1 — mapper 123's `$5800` bank field takes its bit 1 from register bit 3

* **Ticket**: Q-144 §1. **Priority**: highest of this report — it is the only point with a visible
  in-game effect and it affects the number's only two titles.
* **File / function**: `rust/crates/zax-nes/src/cartridges/mappers/mmc3_scramble.rs`,
  `Mmc3Scramble::apply_override`, the `OverridePort::Port5800` arm.
* **The edit**: the bank's bit 1 must come from register bit **3**, not register bit 4, and register
  bit **4** becomes the don't-care. Bank bits 3, 2 and 0 are unchanged (register bits 5, 2 and 0),
  as is `S` at bit 1 and `M` at bit 6. In the arm's present shape that is one term:
  `((register >> 3) & 0x02)` becomes `((register >> 2) & 0x02)`.
* **Also to update, in the same edit**: the type's own doc-comment table (the `| 123 |` row, which
  reads "bits **5, 4, 2, 0**"); the sentence beginning "**123's bit order is the page's and not
  Mesen 2's**", which should be rewritten to record that the page's *prose* ("ignore `$5800` bits
  0-3/5", twice) and all four emulators agree on bit 3 while the page's *diagram* columns say bit 4;
  and the comment "the bank's bits 3, 2, 1 and 0 are the register's 5, 2, 4 and 0".
* **Test to change**: whichever case of the mapper-123 test walks `$5800` with bits 3 and 4 unequal.
  The new expectation is that bit 3 moves the bank and bit 4 does not — i.e. the test's two
  assertions swap. Keep a test for bit 4 being inert, so the two readings stay distinguishable.
* **Titles affected**: *Mortal Kombat 3* (UNL-H2288) and *Earthworm Jim 2* (Kǎshèng / Shin-Shin) —
  every `$5800` value whose bits 3 and 4 differ, i.e. half of them, currently loads the wrong 16 KiB
  PRG bank.
* **Risk to existing boards**: none. The arm is reached only by mapper 123 (`OverridePort::Port5800`
  is that number's only port); mappers 114, 115, 121, 182 and 187 use other arms of the same `match`
  and are untouched. No save-state byte moves, no `Board` dispatch changes, no hot path is involved.
* **Also worth doing**: a note in Q-144 recording that nesdev's own page is self-contradictory here,
  and (R0's call) a message to the wiki so the diagram and the prose stop disagreeing.

### CHANGE 2 — mapper 195's 4 KiB PRG-RAM window at `$5000-$5FFF` (a gap to close)

* **Ticket**: Q-142, last bullet of "What would overturn it". This is an **omission** this build
  already records, not a wrong reading; it is listed here because the cross-check found that two of
  the four implement it and so it is no longer only the page's word.
* **Evidence**: the page names the compatible board (*天神之剑*) and says the NES 2.0 header is the
  only way to recognise it. FCEUX maps the window on **every** mapper-195 image
  (`third_party/fceux/src/boards/mmc3.cpp:1036-1041`) and so does puNES
  (`third_party/punes/src/core/mappers/mapper_195.c:134-137`). Mesen 2 and Nestopia do not.
* **The edit**: a 4 KiB `Mapper::prg_ram_window` at `$5000-$5FFF` for mapper 195, over the second
  4 KiB of the board's 8 KiB WRAM, plus the `board_memory_sizes` row to carry it. Whether it should
  be unconditional (FCEUX's and puNES's choice, which is what makes the named title run on an
  iNES 1.0 dump) or gated on the NES 2.0 header (which is what the page's recognition sentence
  implies) is **the question for R0**; the two differ only on an image that declares no PRG-RAM.
* **Titles affected**: *天神之剑* and any other image on the FS303's compatible board. The eleven
  ordinary mapper-195 titles are unaffected either way.
* **Risk**: this is the shape Q-167 prices — `IrqBoard` does not forward `Mapper::prg_ram_window`,
  and MF-6c steps 2 to 5 measured one new body in a `Board` dispatch at `mmc5` +0.066 % against a
  +0.02 % tolerance. Mapper 195's board is `Mmc3Board`, so the forward may already exist for that
  variant; if it does not, this change and **Q-167's mapper 83.2** want the same forward and should
  be costed together. That pairing is the main reason to record this entry.

### CHANGE 3 — mappers 121 and 187's CHR A18 polarity (conditional on a ROM)

* **Ticket**: Q-144 §4. **Do not apply without the observation named below** — this entry exists so
  that whoever has the ROM does not have to re-derive the edit.
* **File / function**: `rust/crates/zax-nes/src/cartridges/mappers/mmc3_scramble.rs`,
  `Mmc3Scramble::apply_chr_a18`, the `from_a12` branch.
* **The edit, if the observation goes against the page**: replace the fixed upper-half window range
  with the windows the MMC3's two 2 KiB registers currently drive — i.e. the lower half of the
  windows when the chip's CHR-inversion bit is clear and the upper half when it is set. The
  inversion bit is already readable through the held `Mmc3`; nothing else in the function changes.
  The call site must then re-run on an `$8000` write, which it does.
* **Observation that decides it**: load *Street Fighter Zero 2 '97* (121) or *The King of Fighters
  '96* (187) — the only two images with 512 KiB of CHR — and look at whether the background and
  sprite tiles come from the right half of the chip. Under this build's reading the right pattern
  table takes the second 256 KiB half always; under all four emulators' reading the half follows the
  2 KiB registers, which with the inversion bit clear puts the second half in the **left** table.
  The two differ on every tile, so one screenshot settles it.
* **Titles affected**: those two and no others — every smaller image wraps the line away.
* **Risk**: none to other boards (the flag is set only for 121 and 187), but it moves a CHR
  composition that runs on every bank refresh for those two numbers. Measure as a feature, per the
  owner's rule.

---

## OPEN — the points that stay as they are, and what would close each

| ticket | point | current choice | what would close it |
| --- | --- | --- | --- |
| Q-141 | 150/243 R7 mode 3 | `SingleScreenB` (`S1`) | any of the fifteen Sachen titles observed with mode 3 selected; a blank/stale screen is the symptom. Mesen 2 says page 0, puNES says page 1, and FCEUX's and Nestopia's page-0 cell comes with a mode table that is also wrong against the page, so it is not independent. |
| Q-142 | 195's power-on row | `$80` (`$28-$2B`) | a mapper-195 dump that boots at `$00-$03` and not at `$28-$2B`. All four start at `$00-$03`; none states a power-on value of its own. |
| Q-144 §3 | 115's `$6000` bit 6 = PRG A18 | driven in both modes | a mapper-115 image larger than 256 KiB. None is known, and three of the four cap the board's PRG below the point where the bit could matter. |
| Q-144 §4 | 121/187's CHR A18 | PPU A12, both modes, right table = second half | *Street Fighter Zero 2 '97* or *The King of Fighters '96*, one screen. See CHANGE 3. |
| Q-147 §1 | 38's `$F000-$FFFF` | banks, as the page infers | a *Crime Busters* trace with a write in `$F000-$FFFF`, or a photograph of the UNL-PCI556's 74138 enables. All four decode `$7000-$7FFF` only (Nestopia widens downward to `$6000` instead). |
