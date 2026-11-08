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

| Q-148 | 35 | which board the number is | 209's, + 8 KiB WRAM | 209's | the SC-127 | the SC-127 | n/s | **209's** | **OPEN** |
| Q-149 | 35/90/209/211 | `$D001` bit 3 = Extended Mirroring | yes | yes | — | — | — | **yes** | **CONFIRMED** |
| Q-149 | 211 | a special case, or 209? | 209 | 209 | special-cased | special-cased | dip default "on" | **209** | **CONFIRMED** |
| Q-149 | 35/90/209/211 | the per-table RAM/ROM compare | `$D002` bit 7 | `$D002` bit 7 | `$D002` bit 7 | `$D001` bit 7 | `$D002` bit 7 | `$D002` bit 7 | **CONFIRMED** |
| Q-150 §1 | 35/90/209/211 | PRG mode 3 reverses bit 3 onto itself | yes | yes | no (six terms) | not implemented | no (six terms) | **yes** | **CONFIRMED** |
| Q-150 §2 | 35/90/209/211 | `$8003` in 32 KiB mode | a 32 KiB bank | 32 KiB | **8 KiB** at `$8000`, 32 KiB at `$6000` | 32 KiB | 32 KiB | 32 KiB | **CONFIRMED** |
| Q-150 §3 | 35/90/209/211 | with `L=1`, mask and outer bank still apply | yes | yes | no | no | no | **yes** | **OPEN** |
| Q-150 §4 | 35/90/209/211 | `$D003` bit 7 folds CHR regs 2/3 in 2K/1K modes | no | no | **yes** | no | no | no | **CONFIRMED** |
| Q-150 §5 | 35/90/209/211 | the MMC4-like latch is gated on `$D003` bit 7, all numbers | yes | yes | no gate, 209 only | no gate, 209 only | no gate, all numbers | **yes, all numbers** | **CONFIRMED** |
| Q-151 §1 | 35/90/209/211 | a disabled counter does not advance | yes | yes | advances | advances | does not | does not | **CONFIRMED** |
| Q-151 §2 | 35/90/209/211 | disabling resets the prescaler | yes | yes | no | no | no | **yes** | **OPEN** |
| Q-151 §3 | 35/90/209/211 | the prescaler's down-wrap | at the mask | at the mask | at zero | at the mask | at the mask | at the mask | **CONFIRMED** |
| Q-151 §4 | 35/90/209/211 | IRQ source 3 ("CPU writes") | documented | not clocked | clocked (CPU-level) | not clocked | not clocked | clocked, cartridge writes only | **OPEN (gap)** |
| Q-152 | 35/90/209/211 | a write to a ROM nametable | silent | dropped | **reaches CIRAM** | dropped | dropped | dropped | **CONFIRMED** |

| Q-154 §1 | 178 | a PPU-driven 4 KiB CHR-RAM window | none | none | none | none | none | none | **CONFIRMED** |
| Q-154 §2 | 178 | the register decode | `$4800-$4803`, no mask | those four | `$4800-$4FFF` | `$4800-$4FFF` | `$4800`/`$4801`/`$4802` | `$4800-$48FF` | **OPEN** |
| Q-154 §3 | 178 | PRG-RAM: size / battery | 8 KiB window, banked, no battery line | floor 8 KiB, header decides, battery bit decides | 32 KiB, never battery | 32 KiB, battery bit decides | none served | header decides | **CONFIRMED** |
| Q-155 | 163 | which ports the `$5300` bit 0 swap covers | "$5000-$5200" (ambiguous) | `$5000`, `$5200` — reading (b) | no swap | no swap | no swap | **all three — reading (a)** | **OPEN → (a) likely** |
| Q-155 | 163 | the feedback read's other seven bits | silent | 0 | register mix | register mix | `$5300` byte | **0** | **CONFIRMED** |
| Q-156 | 268 | the register lockout | works, and keeps the WRAM | implemented, MMC3 mode only | implemented, MMC3 mode only | implemented, MMC3 mode only | n/s | implemented, **no mode gate** | **CONFIRMED** |
| Q-157 | 268 | the thirteen PCB codes | six submapper groups | 0/1 served, rest refused | one layout for all | **0-9 served, 10/11 refused** | n/s | 0-9 served, 10/11 fall through | **CONFIRMED (form)** |
| Q-158 §1 | 268 | PRG A25 and the interleaved reset row | documented | first half | — | — | n/s | — | **OPEN** |
| Q-158 §2 | 268 | a PRG/CHR size ceiling | — | none | — | — | n/s | — | **CONFIRMED** |
| Q-159 | 268 | `$xxx3` bit 5, PRG-RAM at `$5000-$5FFF` | one clause | COOLBOY only, chip's 2nd 4 KiB, `$A001`-gated | — | an unconditional write window (crash hack) | n/s | — | **OPEN** |

| Q-160 §1 | 137 vs 138/139/141 | R7 `mm` 0/1 differ between the two pages | yes | each page's own | **each page's own** | one table for all four | one table for all four | the two differ (family value is a third thing) | **CONFIRMED** |
| Q-160 §2 | 137 vs 138/139/141 | what `s` fixes mirroring to | that page's mode 0 | that page's mode 0 | **that page's mode 0** | V for all four | V for all four | **that page's mode 0** | **CONFIRMED** |
| Q-160 §3 | 137/138/139/141 | "banks become weird" = all four windows from reg 0 | family page's sentence | yes | yes | yes | yes | yes | **CONFIRMED** |
| Q-160 §4 | 137 | internal register 6 exists | yes (CHR bit 3) | implemented | implemented | implemented | implemented | implemented | **CONFIRMED** |
| Q-161 (a) | 112 | `$C000-$FFFF` | not described | the 206's fixed last two | fixed | fixed | fixed | fixed | **CONFIRMED** |
| Q-161 (b) | 112 | bit 0 of the two 2 KiB CHR selects | not described | discarded (the 206's rule) | **used** | discarded | discarded | discarded | **CONFIRMED** |
| Q-161 (c) | 193 | the `$6004` mirroring register | documented, "no game uses it" | implemented, 1 = horizontal | — | — | — | **implemented, 1 = horizontal** | **CONFIRMED** |
| Q-161 (c') | 193 | the register mask | `$E007` (A2 decoded) | `$E007` | `& 0x03` | `$6000-$6003` | `& 0x03` | **`& 0x07`** | **CONFIRMED** |
| Q-161 (d) | 193 | PRG-RAM behind the register window | no capacity table | `Some(0)` | none | none | none | none | **CONFIRMED** |
| Q-162 (a) | 298 | `$F003` acknowledges without copying A into E | yes | yes | ack only (non-VRC IRQ) | `$F003` = `$F001` | `$F003` = `$F001` | **yes, via a per-board flag** | **CONFIRMED** |
| Q-162 (a') | 298 and 21/25 | does any acknowledge reload the counter? | no | no | no | no | no | no | **CONFIRMED** |
| Q-162 (b) | 298 | a load-time flag on the shared VRC2/VRC4 board | — | load-time `bool` | its own board | its own board | its own board | **a load-time argument on the shared board** | **CONFIRMED** |
| Q-163 | 218 | the 2 KiB serves the pattern tables too | "mapped as CHR-RAM" | yes, reading (b) | **yes** | **yes** | n/s | **yes** | **CONFIRMED** |
| Q-163 | 218 | the four wirings' window table | derived | A10/A11/A12/A13 per iNES byte 6 | identical | identical | n/s | identical | **CONFIRMED** |

*(rows for Q-164 onward are appended by the later batches of this lot)*

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

## Q-148 to Q-152 — the J.Y. Company ASIC (mappers 35, 90, 209, 211)

One finding governs all five tickets and should be read first: **puNES treats mappers 35, 90, 209 and
211 as one board** (`third_party/punes/src/core/mappers.c:160-161`, `:316-317`, `:627-628`,
`:633-634`, all four `case`s calling `map_init_209`), with exactly **one** per-number parameter —
`init_JYASIC(info.mapper.id != 90, …)` at `src/core/mappers/mapper_209.c:43`. That is this build's
shape to the letter: one board, no per-number difference but mapper 90's jumper. The shared chip
model is `src/core/mappers/JYASIC.c` (535 lines) with the number-specific bank composition in
`mapper_209.c`. Nestopia has three classes with shared code and differing dip-switch defaults
(`source/core/board/NstBoardJyCompany.cpp`, 858 lines); Mesen 2 has one class plus a separate small
board for 35; FCEUX has one file with two `is209`/`is211` flags (`src/boards/90.cpp`, 507 lines).

So on the arithmetic tickets the tally is usually **the page + this build + puNES against Mesen 2 and
FCEUX, with Nestopia splitting** — and puNES is a genuinely independent reading, because where it
agrees with this build it agrees on details (the seven-bit reversal's middle bit, the `L=1` mask, the
`$D003` bit 7 gate) that Mesen's and FCEUX's shared ancestor gets differently.

### Q-148 — is mapper 35 the J.Y. ASIC's board, or a board of its own?

* **the page** — "[INES Mapper 035] is like Mapper 209 but explicitly specifies 8 KiB of WRAM … an
  exact duplicate of INES Mappers 209".
* **this build** — mapper 209's board, with the 8 KiB stated in `board_memory_sizes`.
* **Mesen 2** — `Core/NES/Mappers/JyCompany/Mapper35.h` (note the directory), a 75-line board with
  four PRG registers at `$8000-$8003`, eight CHR registers at `$9000-$9007`, `$C002`/`$C003`/`$C005`
  and `$D001`, and an MMC3-style **filtered A12** down-counter through `A12Watcher`.
* **FCEUX** — `src/ines.cpp:567` maps mapper 35 to `UNLSC127_Init`, i.e. `src/boards/sc-127.cpp`,
  which is the **UNL-SC-127** board and is the same register set and the same filtered-A12 counter as
  Mesen's `Mapper35`. The entry's own UNIF board name is "Wario Land 2". So Mesen's class is this
  board, and FCEUX is where it comes from (`src/unif.cpp:412` registers the same `SC-127`).
* **Nestopia UE** — **no mapper 35 at all**: `source/core/board/NstBoardJyCompany.cpp`'s ines switch
  goes from `case 34` straight to `case 36`.
* **puNES** — mapper 209's board (`src/core/mappers.c:160-161`).

**Verdict: OPEN**, and the cross-check hardened the question rather than settling it. Two useful
observations:

1. **The SC-127's registers are a strict subset of the J.Y. ASIC's addresses.** `$8000-$8003` are the
   chip's four PRG registers in 8 KiB mode, `$9000-$9007` its eight CHR registers in 1 KiB mode,
   `$D001` its nametable arrangement, and `$C002`/`$C003` its IRQ disable/enable with `$C005` its IRQ
   counter. That is exactly this ticket's reason 3 ("a mapper-35 image whose game uses only the
   registers Mesen's small board has runs identically on this build"), and it is now verified against
   the register list rather than asserted.
2. **The one register that is not a subset is the counter**, which is the ticket's own falsifier.
   Both SC-127 implementations clock it from a **filtered PPU A12** and fire when it reaches zero
   counting **down**; the J.Y. ASIC powers on with source 0 (CPU M2), a prescaler at `$C004` and a
   direction in `$C001` bits 6-7. A dump that writes only `$C002`/`$C003`/`$C005` therefore gets a
   scanline IRQ on the SC-127 board and an M2-rate (or stopped) counter on this one.

So: keep the page's reading — it is the number's own documentation and puNES agrees — but **the
observation that closes this ticket is now precisely specified**: does *Warioland II* write `$C001`
and `$C004`? If it does not, the SC-127 board is what the dump was built for and this build needs
either a submapper or a per-ROM exception. Two of the four run the number's one title on that board,
which is the strongest argument available for it.

### Q-149 — `$D001` bit 3 is Extended Mirroring, and mapper 211 is therefore mapper 209

* **`$D001` bit 3** — **puNES decodes it.** `JYASIC.c:160` keeps bit 3 of `$D001` only for the numbers
  whose per-number flag is set (`mode[1] = !extended_mode ? value & ~0x08 : value`), and
  `JYASIC.c:462-469` is the mechanism: with bit 3 set and ROM nametables off, each of the four tables
  takes its CIRAM page from `$B000-$B003` **bit 0**. That is the page's Extended Mirroring and this
  build's `JyCompany::nametable_page_of`.
  Mesen 2 (`JyCompany.h:354`, `_mirroringReg = value & 0x03`) and FCEUX
  (`src/boards/90.cpp:96-104`, `switch (tkcom[1] & 3)`) decode two bits and no more; Nestopia
  (`NstBoardJyCompany.cpp:841`, `SetMirroringVH01( regs.ctrl[1] )`) likewise.
  **CONFIRMED** — the page and puNES against three that read only `MM`.
* **mapper 211** — **puNES has no special case**: all four numbers go to `map_init_209` and the only
  flag is `id != 90`. Mesen 2 has `|| _romInfo.MapperID == 211` twice (`JyCompany.h:260`, `:387`);
  FCEUX has `((tkcom[0]&0x20&&is209)||is211)` (`src/boards/90.cpp:76`); Nestopia makes it a user **dip
  switch** whose default is "controlled by the register" for 209 and "always on" for 211
  (`NstBoardJyCompany.cpp:61-64`, `:832`). **CONFIRMED** — and the three dissenters' shape is exactly
  the historical definition the page calls an error ("the mapper was defined when the function of
  register `$D001` bit 3 was not known"), which is now visible as a lineage: decode the bit and the
  special case has nothing left to do, which is what puNES demonstrates.
* **a third point the ticket does not raise, found here**: the per-table RAM/ROM compare. The page
  says `$B00x` bit 7 against **`$D002`** bit 7; this build, Mesen 2 (`JyCompany.h`, `_ntRegs` vs
  `_chrBlockMode` register) , Nestopia (`NstBoardJyCompany.cpp:838`, `(banks.nmt[i] ^ regs.ctrl[2])`)
  and puNES (`JYASIC.c:454`, `(mode[2] ^ nmt.reg[i]) & 0x80`) all use `$D002`; **FCEUX uses `$D001`
  bit 7** (`src/boards/90.cpp:88`, `(tkcom[1]&0x80)==(names[x]&0x80)`). FCEUX alone. **CONFIRMED** for
  this build.

### Q-150 — the five places the page and Mesen 2 differ on bank arithmetic

1. **PRG mode 3 reverses bits 0-6, bit 3 onto itself.** puNES's `prg_reverse_JYASIC`
   (`JYASIC.c:519-534`) has **seven** terms, including `((value & 0x08) << 0)` with the comment
   `// 0x08`. Identical to this build's `JyCompany::prg_register`. Mesen 2's `InvertPrgBits`
   (`JyCompany.h:161`) has six and Nestopia's `Banks::Unscramble`
   (`NstBoardJyCompany.cpp:434-443`) has the same six; FCEUX does not implement the mode at all — its
   `tekprom` has `case 03: // bit reversion` falling through to the unreversed `case 02`
   (`src/boards/90.cpp:121-122`). **CONFIRMED**: the page, this build and puNES; the six-term form is
   a dropped middle term in one lineage and FCEUX's comment shows it was never written.
2. **`$8003` is a 32 KiB bank in 32 KiB mode.** puNES `JYASIC.c:333-339` (`last_bank <<= 2`, four
   consecutive 8 KiB banks) and `:421-426` (`$6000` = `prg[3] << 2 | 3`, the page's "SHL 2 OR 3") —
   consistent. Nestopia `NstBoardJyCompany.cpp:717-720` (`SwapBank<SIZE_32K>((last & 0xF) | …)`) and
   `:703-707` — consistent. FCEUX `src/boards/90.cpp:131` (`setprg32`) and `:129` — consistent.
   **Mesen 2 alone is inconsistent with itself**: `JyCompany.h:174` is `SelectPrgPage4x(0,
   prgRegs[3])`, four *8 KiB* pages from that register (its fixed default `0x3C` confirms the unit),
   while `:177` is `prgRegs[3] * 4 + 3`. **CONFIRMED** — three of the four and the page. *(A second
   Mesen inconsistency, not in the ticket: in 16 KiB mode `:182` shifts `prgRegs[1]` but `:183` does
   not shift `prgRegs[3]`, so `$C000` takes it as an 8 KiB bank.)*
3. **With `L=1` the CHR bank is still masked and `cc` still applies.** **puNES implements the page
   exactly** — `mapper_209.c:66-75`: the base is `C` at bank bit 8 and `cc` at bits 9-10, and with
   `L` set the base keeps only `cc` and the mask widens to nine bits. Identical to this build's
   `JyCompany::chr_bank`. Mesen 2 (`JyCompany.h:212-216`), FCEUX (`src/boards/90.cpp:152`,
   `if(!(tkcom[3]&0x20))`) and Nestopia (`NstBoardJyCompany.cpp:758-761`, `mask = 0xFFFF; bank = 0`)
   all drop both when `L` is set and use the raw 16-bit register pair. **Verdict: OPEN** — 3-1 against
   this build on the count, but the three are one expression (FCEUX's is the original) and the page is
   explicit and internally consistent ("Mask `$900x`/`$A00x` to 512 KiB, ignore C", with `cc` never
   qualified). Keep. **What would close it**: a J.Y. image with more than 512 KiB of CHR-ROM that sets
   `$D003` bit 5 — below that size the two readings both wrap to the same bank.
4. **`$D003` bit 7 is the MMC4-like mode and nothing else.** puNES never folds CHR registers
   (`JYASIC.c:378-404`: 2 KiB mode uses registers 0, 2, 4, 6 and 1 KiB mode 0 to 7), nor does
   Nestopia (`NstBoardJyCompany.cpp:790-813`) nor FCEUX (`src/boards/90.cpp:174-181`). Mesen 2's
   `_mirrorChr` fold (`JyCompany.h:208`) is **Mesen's alone**. **CONFIRMED**, three of four and the
   page.
5. **The latch is gated on `$D003` bit 7, on every number.** **puNES gates on exactly that bit and
   checks no mapper number**: `JYASIC.c:237` is `if (jyasic.mode[3] & 0x80)` around the
   `$0FD8`/`$0FE8` decode, and `mapper_209.c` installs the hook for all four numbers. Identical to
   this build. Mesen 2 (`JyCompany.h:387` region, `MapperID == 209`) and FCEUX
   (`src/boards/90.cpp:390`, `if(is209)` with `chr[0]=0; chr[1]=4` forced otherwise) gate on the
   **number** and not the bit; Nestopia gates on neither — its latch runs for all three numbers but
   is conditioned only on the CHR 4 KiB mode (`NstBoardJyCompany.cpp:485-489`). **CONFIRMED** — the
   page and puNES, with Nestopia agreeing on the half that matters most (the mode is not
   mapper-specific).

A detail worth recording because it is identical in all five implementations: the latch **values**.
This build's `chr_4k_register` is `window * 4 + latch * 2` (registers 0/2 for the left table and 4/6
for the right); puNES's latch arithmetic (`JYASIC.c:241`) yields `{0,2}` and `{4,6}`, Nestopia's
(`:485`) the same expression, and FCEUX's `chr[]` is initialised to `{0,4}` (`src/boards/90.cpp:415`).
Five for five.

### Q-151 — the IRQ

* **§1, "inhibits counting".** This build returns from `tick_irq` while disabled. **Nestopia agrees**
  — its counter is only ever reached through `IsEnabled(source) && base.Clock()`
  (`NstBoardJyCompany.cpp:420-431`) — and **puNES agrees** (`JYASIC.c:490-492`, an early return on
  `!jyasic.irq.active`). Mesen 2 advances regardless and tests the flag only at the fire
  (`JyCompany.h:442`); FCEUX does the same (`src/boards/90.cpp:331-347`, `CCL` tests `IRQa` only
  inside the `if (IRQCount == 0)`). **CONFIRMED**, 3-2 for the page, and the page's own step 1 begins
  "When counting is enabled".
* **§2, "resets the prescaler to zero".** **Only puNES does it** (`JYASIC.c:122-126` for `$C000` with
  bit 0 clear and `:134-137` for `$C002`, both setting `prescaler = 0`). Mesen 2, FCEUX
  (`src/boards/90.cpp:250-252`) and Nestopia (`NstBoardJyCompany.cpp:611-619`) all leave the
  prescaler where it was. **Verdict: OPEN.** Three omissions of a one-line documented effect are not
  evidence that the page is wrong, and the page's gloss is unambiguous ("'Disabling' acknowledges a
  pending IRQ, inhibits counting, and resets the prescaler to zero"). Keep. **What would close it**: a
  game that disables and re-enables without rewriting `$C004` and whose raster effect lands a whole
  prescaler period out — which is what this build's own test constructs.
* **§3, the prescaler's down-wrap.** This build wraps when the decremented prescaler **matches the
  mask**. FCEUX (`src/boards/90.cpp:365`), Nestopia (`NstBoardJyCompany.cpp:418`) and puNES
  (`JYASIC.c:505`) all write the same condition. **Mesen 2 alone** wraps off a prescaler of one
  (`JyCompany.h:432` region). **CONFIRMED**, 3-1 and the page.
* **§4, IRQ source 3 ("CPU writes").** This build does not clock it. **FCEUX does not either** — its
  three dispatchers cover sources 0, 1 and 2 (`src/boards/90.cpp:372`, `:378`, `:384`) and there is
  no fourth. **Nestopia does not**: it names the mode (`NstBoardJyCompany.hpp:212`,
  `MODE_CPU_WRITE`) and clocks only M2, A12 and PPU-read. **Mesen 2 does, completely**, by asking the
  CPU (`JyCompany.h:373`, `_console->GetCpu()->IsCpuWrite()`) — which is exactly the hook this core
  has not got, so Mesen 2 confirms the ticket's analysis of the cost rather than contradicting it.
  **puNES clocks it partially**: `JYASIC.c:63-65` clocks the prescaler at the top of the cartridge's
  own write handler, so cartridge-space writes count and zero-page writes do not — the page's "CPU
  writes" is all writes. **Verdict: OPEN (a recorded gap)**, shared with two of the four. Worth adding
  to the ticket: puNES's shape is a cheap partial that this core could copy for nothing (the
  `Cartridge::cpu_write` path already exists) if a title turns out to select source 3 and to write
  mostly to cartridge space; the full version needs the hook the ticket prices.

### Q-152 — a write to a cartridge-served nametable

* **Mesen 2** keeps the CIRAM byte under the overlay, installing the mapping on the **read** path only
  and saying so in a comment that cites a nesdev forum thread (`JyCompany.h:385-387`).
* **FCEUX** switches the nametable pointer itself (`src/boards/90.cpp:81`, `:89-91`,
  `setntamem(CHRptr[0] + …, 0, x)`), whose second argument marks the page as **not RAM**, so a write
  to it is discarded.
* **Nestopia UE** swaps the nametable's *source* to CHR-ROM (`NstBoardJyCompany.cpp:838`), and a
  write to a ROM source is discarded.
* **puNES** maps it with `memmap_nmt_chrrom_1k`, whose implementation sets the region's write flag to
  false (`src/core/memmap.c:1592-1596`, `BYTE wr = FALSE`), so the write is dropped.

**Verdict: CONFIRMED.** Three of the four do exactly what this build does — drop the write and lose
the byte underneath. Mesen 2's behaviour is the more accurate one and it is the only project that has
it, which is the ticket's own reading: the right answer costs a change to the
`SERVED_BY_CARTRIDGE` contract, and nobody but Mesen 2 has paid it. The ticket's choice stands and its
test stays as the thing to rewrite if that contract ever grows a direction.

## Q-154 — mapper 178 (Waixing FS305)

Boards: Mesen 2 `Core/NES/Mappers/Waixing/Waixing178.h`; FCEUX `src/boards/178.cpp`; Nestopia UE
`source/core/board/NstBoardWaixingSgzlz.cpp` (named "Waixing San Guo Zhong Lie Zhuan"); puNES
`src/core/mappers/mapper_178.c`.

* **§1, no PPU-driven CHR window: CONFIRMED, four for four.** Mesen 2 sets an 8 KiB CHR page once in
  `InitMapper` and never again, with `GetChrPageSize() 0x2000` and no VRAM address hook; FCEUX's
  `Sync` calls `setchr8(0)` unconditionally; puNES's `chr_fix_178` maps a fixed 8 KiB page (and
  takes `reg[3]` as a CHR bank only for mapper **551**, a different number); Nestopia's board has no
  CHR handling at all. **Not one of the four has a PPU hook, a 4 KiB CHR window or a CHR A12
  substitution for this number** — so the lot's section heading, and not the implementation, is what
  was wrong, which is exactly what the ticket asks R0 to confirm. The anti-test
  `q_154_mapper_178_has_no_ppu_driven_chr_window` is the right shape.
* **§2, the register decode: OPEN, and no project decodes only four addresses.** Mesen 2
  (`RegisterStartAddress 0x4800` / `RegisterEndAddress 0x4FFF`, register index `addr & 0x03`) and
  FCEUX (`src/boards/178.cpp`, `SetWriteHandler(0x4800, 0x4fff, M178Write)`) both decode
  **`$4800-$4FFF`** with A1-A0; puNES decodes **`$4800-$48FF`** (`mapper_178.c:66`,
  `switch (address & 0xFF00) case 0x4800`); Nestopia decodes **three single addresses**,
  `$4800`/`$4801`/`$4802`, and gives `$4800` the mirroring alone
  (`NstBoardWaixingSgzlz.cpp`, its `SubReset` map). So the four are genuinely split and the page
  states no mask. Keep the page's literal four addresses. **If a dump ever shows a mirror in use,
  `$4800-$4FFF` has two votes**, which is also the ticket's own guess ("a board of this era would more
  likely decode `$4800-$4FFF` with A1-A0"); that is one constant in `WaixingFs305::PORT_LAST`'s place
  and a widened test.
* **§3, the PRG-RAM: CONFIRMED on the battery, informative on the size.** On the battery the page
  says nothing and this build lets the dump's battery bit decide. **FCEUX does exactly that** —
  `Mapper178_Init` allocates the RAM and calls `addSaveGameBuf` only `if (info->battery)`. puNES maps
  it through its `auto` helper, which follows the header. Mesen 2 declares it `GetWorkRamSize()`,
  i.e. **never** persisted. Nestopia serves no `$6000` window at all. So 2-1 for "the header decides",
  which is this build's rule and `board_memory_sizes`'s rule everywhere else: **CONFIRMED**, and
  §7.2's "all three also want battery-backed PRG-RAM" is wrong for this number.
  On the **size**, both Mesen 2 and FCEUX fix it at **32 KiB** — four 8 KiB banks, which is all the
  two low bits of `$4803` that either of them reads can reach. This build's `min_prg_ram: 8 KiB` with
  the header deciding serves that case when the header declares it and falls back to one mirrored
  chip when it does not, so nothing is broken; but **32 KiB is the figure to adopt if R0 wants an
  exact one**, and it is the answer to the ticket's third question.

## Q-155 — mapper 163's "swap D0,D1 on writes to `$5000-$5200`"

**This is the ticket the cross-check moved furthest, and in the direction the ticket rejected.**

* **Mesen 2** (`Core/NES/Mappers/Unlicensed/Nanjing.h`), **FCEUX** (`src/boards/164.cpp`, the
  `Write2`/`Mapper163_Init` half) and **Nestopia UE**
  (`source/core/board/NstBoardNanjing.cpp:157-160`) all latch `$5300` and use it **only in the
  protection reads**. None of the three implements a D0/D1 swap at all, so none of them has an opinion
  on its scope.
* **puNES implements the swap, and implements it on all three ports — reading (a).**
  `src/core/mappers/mapper_163.c:66-100`: the `$5000` arm, the **`$5100` arm** and the `$5200` arm each
  begin by exchanging D0 and D1 when `reg[3] & 0x01` is set. The `$5101` strobe test that flips the
  feedback bit is then applied to the **swapped** value (`:80-82`), which is precisely the sequence
  this ticket constructs as the discriminator.
* And puNES carries a **hardware reason for its one exception**, in its own comment at `:88-90`:
  1 MiB games wire both of the ASIC's PRG A19 and A20 outputs to ROM A19, "effectively exempting this
  register from the bit-swap", so `$5200` is swapped only on images of 2 MiB or more. That is an
  *output*-side exemption, which is coherent with the swap itself sitting on the ASIC's **data input
  pins** — i.e. on everything the `$5xxx` decode latches, `$5100` included.

**Verdict: OPEN, leaning to reading (a).** The count is 1-0 with three abstentions, so this is not a
CHANGE on the arithmetic of votes; but the one project that has the mechanism chose the reading this
build rejected, and did so with a note about the board's wiring. That is the acceptance vector the
ticket asked for ("Confirm reading (b), or supply reading (a) with the sequence above as its
acceptance vector"), and §CHANGE 4 holds the edit so a later lot does not have to re-derive it.

**The smaller reading in the same function: CONFIRMED.** On the feedback read this build drives bit 2
and answers zero elsewhere; **puNES does the same** (`mapper_163.c:105`, the answer is the inverted
feedback bit alone, every other bit zero). Mesen 2 and FCEUX answer an OR of several registers and
Nestopia answers the whole `$5300` byte, none of which the page describes. Two of four, and the two
that agree are this build and the only other project that models the port as a port.

## Q-156 to Q-159 — mapper 268 (COOLBOY / MINDKIDS)

**Nestopia UE does not support the number**, so these four tickets are a three-way comparison. Mesen
2's board is `Core/NES/Mappers/Mmc3Variants/MMC3_Coolboy.h` and says so in its first comment line
("Mostly taken from FCEUX's code"); FCEUX's is `src/boards/coolboy.cpp` (CaH4e3 and Cluster, 2018),
whose header comment is a transcription of the same four register diagrams the page carries; puNES's
is `src/core/mappers/mapper_268.c`.

### Q-156 — the lockout

**CONFIRMED, three for three, and §7.2's "implemented as a no-op" is simply wrong.**

* **FCEUX** `src/boards/coolboy.cpp:286`: `if ((EXPREGS[3] & 0x90) != 0x80) EXPREGS[A & 3] = V;`, with
  its own comment "Deny any further writes when 7th bit is 1 AND 4th is 0" — bit 7 set **and** the
  banking mode in MMC3 mode, which is this build's condition exactly. And the WRAM write happens
  first and independently (`:281-283` calling `AA6023WramWrite`, which is gated on the MMC3's
  `$A001`): the page's three-instruction sequence works.
* **Mesen 2** `MMC3_Coolboy.h:94-101`: the same two steps in the same order — `WritePrgRam` when
  `RegA001 & 0x80`, then the registers only when `(_exRegs[3] & 0x90) != 0x80`.
* **puNES** `mapper_268.c:74`: `if (!(m268.reg[3] & 0x80) || ((address & 0x07) == 2))` — bit 7 blocks
  every register **but the one at `$xxx2`**, which is the page's parenthesis implemented literally
  (Mesen 2 and FCEUX block `$xxx2` too, since they mask the index to two bits and gate the whole
  write). **puNES does not gate on the banking mode**, so its lockout also works in GNROM mode, which
  the page's two "Lockout DOES NOT work in this mode" sentences deny.
* Also worth recording, since the ticket raises it: **whether the bit can be cleared.** All three
  projects re-read the register on every write exactly as this build does — none latches a separate
  flag — so the bit is sticky until a reset in all four implementations. The ticket's smaller reading
  is the consensus.

So on the one point §7.2 got wrong, three of three agree with the page and with this build, and one
of them annotates the condition in the same words. **This ticket can be closed in this build's
favour**; what remains is for §7.2's step 3 to be corrected, which is what the ticket asks.

### Q-157 — the thirteen PCB codes, and what "leave the rest refusing" means

**Reading (b) CONFIRMED in form; and the cross-check shows the four layouts this build refuses are
implementable.**

* **FCEUX** is reading (b) exactly: `CommonInit` (`src/boards/coolboy.cpp:408-438`) switches the
  register base on the submapper (`$7000` for 2, `$6000` for 0/4/6/8, `$5000` for 1/3/5/7/9), sets
  four layout flags `flag23`, `flag45`, `flag67`, `flag89` for the four non-COOLBOY groups, and for
  anything else calls `FCEU_PrintError("Submapper #%d is not supported", submapper)` — i.e. it serves
  submappers 0 to 9 and **refuses 10/11**.
* **puNES** has the same five groups (`mapper_268.c:106-133`, `switch (info.mapper.submapper & ~1)`
  with arms for 2, 4, 6 and a `default` for 0/1, plus the submapper-8/9 CHR-RAM write protect at
  `:161`), but its `default` also swallows 10/11, so a submapper-10 dump runs on the COOLBOY layout
  rather than being refused.
* **Mesen 2** reads no submapper at all: one layout, one register base (`RegisterStartAddress`
  `$6000`), nothing refused — reading (a).

So the *principle* this build chose is FCEUX's, including the error message, and the three
implementations between them demonstrate the premise reading (b) rests on: the groups really do need
separate arms (both FCEUX and puNES have them, and they are not small). **CONFIRMED.**

What the cross-check adds is an opportunity rather than a correction: the four layouts this build
refuses (submapper groups 2/3, 4/5, 6/7 and 8/9) are implemented by **two** of the three projects
that support the number, and two of the features behind them are named in both — the submapper-6/7
**two-chip PRG select** (FCEUX `:239-242`, `flag67 && EXPREGS[0] & 0x08` adding `ROM_size` as a chip
offset; puNES `:126-132`, the same thing expressed as a half-ROM offset) and the submapper-8/9
**CHR-RAM write protect** (FCEUX `:135-140`; puNES `:161`). See §CHANGE 5 — this is a scope note, not
a defect: nothing the lot exists to preserve is behind a refused submapper, as the ticket says.

### Q-158 — the 64 MiB mapping

**OPEN, and all three are silent.** Not one of Mesen 2, FCEUX or puNES reads `$6001` bit 0 or `$6003`
bit 0 as SC0/SC1, and none has any PRG A25 line; FCEUX's and puNES's only multi-chip mechanism is the
submapper-6/7 chip select described above, which is the page's `D` bit on the J-852C and a different
thing. So the interleaved reset row — the one a bank-window core cannot express — is unimplemented
everywhere, and this build's "answer the first 32 MiB half" is no worse than the field. Keep it; the
falsifier stays a 64 MiB dump whose two halves differ at the reset vector, and the page itself says
the existing multicarts duplicate both halves.

**Part 2 (the size ceiling): CONFIRMED by inspection of this build's own loader**, nothing to
cross-check — but worth one observation from FCEUX: `GenMMC3_Init(info, 2048, …)` caps that board's
PRG at **2048** 8 KiB banks, i.e. 16 MiB, so FCEUX *does* have the ceiling §7.2 feared and this build
does not. That is the shape of the check §7.2 asked for, found in a project rather than in this
repository, and it is why the ticket's "no ceiling was added either" is the right call.

### Q-159 — `$xxx3` bit 5, "Also enable PRG RAM in `$5000-$5FFF`"

**OPEN, and nobody implements the bit.** Mesen 2 and puNES do not decode it at all. FCEUX installs a
`$5000-$5FFF` **write** handler whenever the register base is not `$5000`
(`src/boards/coolboy.cpp:385-386`) — unconditionally, not from the bit — and its own comment says why:
"some games access random unmapped areas and crashes because of KT-008 PCB hack in MMC3 source lol".
That is a crash workaround, not the page's mechanism, and there is no matching read handler.

So this build is the only implementation of the clause anywhere, and all three of its readings stand
unchallenged:

* **(1) a COOLBOY only** — supported indirectly: on a MINDKIDS the register file *is* `$5000-$5FFF` in
  all three projects (FCEUX `regs_base = 0x5000`, puNES `m268tmp.rstart = 0x5000`), so there is no
  address left for RAM there, which is this build's argument;
* **(2) CPU A0-A12 straight through** — untested anywhere;
* **(3) "Also" adds a window, `$A001` still gates it** — consistent with all three projects' handling
  of the `$6000-$7FFF` window, which is `$A001`-gated in every one of them.

Keep all three. The observation that closes it is still one dump that sets the bit.

## Q-160 — the Sachen 8259's two pages

Boards: Mesen 2 `Core/NES/Mappers/Sachen/Sachen8259.h` (one class, a four-valued variant enum);
FCEUX `src/boards/sachen.cpp`'s `S8259*_Init` family (one file, a `type` integer); Nestopia UE
`source/core/board/NstBoardSachenS8259.cpp` (one class, a board enum); puNES one file per number
(`mapper_137.c`, `_138.c`, `_139.c`, `_141.c`).

* **§1, modes 0 and 1 swapped between the two pages: CONFIRMED, and Mesen 2 does exactly what this
  build does.** `Sachen8259.h:47-48`: mode 0 is `_variant == Sachen8259D ? Horizontal : Vertical` and
  mode 1 the converse — i.e. **the 8259D follows the mapper-137 page and the A/B/C follow the family
  page**, which is `Sachen8259::two_screen`'s single `bool` in this build. puNES also makes the two
  numbers differ (`mapper_137.c:76-89` gives `mm`=0 horizontal; `mapper_138.c:77-90` gives `mm`=0
  something else), though its family value is a third thing — `(0,0,0,1)`, which is the *other*
  Sachen page's mode 0 (the 74LS374N's, Q-141) rather than vertical. FCEUX
  (`src/boards/sachen.cpp:26-32`, `S74LS374MSync` shared by all four 8259 types **and** by the
  74LS374N) and Nestopia (`NstBoardSachenS8259.cpp:103-111`, one 4×4 look-up table) use **one table
  for all four numbers**, so on both of them the 8259D contradicts its own page. Two clean votes for
  the per-page swap, one partial, two against — and the two against are the pair that share the
  FCEU-mm table Q-141 already found wrong on two rows.
* **§2, what `s` fixes the mirroring to: CONFIRMED, 3-2.** `Sachen8259.h:53-56` sets simple mode to
  `_variant == Sachen8259D ? Horizontal : Vertical` — **each variant's own mode 0**, which is this
  build's reading and the one thing the ticket says the two pages agree on once read the right way
  round. puNES agrees (`mapper_137.c`'s `default` arm is horizontal, `mapper_138.c`'s is vertical, and
  an odd `R7` lands in both). FCEUX forces `MI_V` for all four (`sachen.cpp:165`) and Nestopia forces
  `lut[0]`, which is its vertical row, for all four.
* **§3, "banks become weird": CONFIRMED four for four.** Every project reads **CHR register 0 for all
  four windows** when `R7` bit 0 is set — Mesen 2 `_regs[simpleMode ? 0 : n]`
  (`Sachen8259.h:61-71`), FCEUX `if (latch[7] & 1) bank = latch[0] …` (`sachen.cpp:140-144`),
  Nestopia `regs[(regs[7] & 0x1U) ? 0 : n]` (`NstBoardSachenS8259.cpp:138-141`), puNES
  `reg[(reg[7] & 0x01) ? 0 : n]` (`mapper_137.c:70-73`). So the family page's concrete sentence **is**
  what the 8259D's "banks become weird" means, including on the 8259D itself, which is exactly this
  build's reading.
* **§4, internal register 6: CONFIRMED four for four, and §7.2's omission is only an omission.** All
  four give the 8259D's fourth CHR window a bit 3 from register 6: Mesen 2
  `((_regs[6] & 0x01) << 3)` (`Sachen8259.h:64`), FCEUX `((latch[6] & 1) << 3)`
  (`sachen.cpp:154`), Nestopia `(regs[6] << 3 & 0x08U)` (`NstBoardSachenS8259.cpp:128`), puNES
  `((m137.reg[6] & 0x01) << 3)` (`mapper_137.c:73`). And all four compose the other three windows
  the same way this build does — register 4's bits 0, 1 and 2 at **bank bit 4** of windows 1, 2 and 3
  respectively.
* **A fifth point, not in the ticket, five for five**: the 8259D's `$1000-$1FFF` is the **last 4 KiB**
  of CHR-ROM in every project (Mesen 2 `SelectChrPage4x(1, -4)`, FCEUX `setchr4(0x1000, ~0)`,
  Nestopia `chr.SwapBank<SIZE_4K,0x1000>(~0U)`, puNES `memmap_auto_4k(0, MMPPU(0x1000), 0xFF)`) and in
  this build (`sachen_8259.rs:31`, `:261`). Nothing in dispute, recorded because it is the half of the
  8259D's CHR layout the ticket does not mention.

So the two questions the ticket puts to R0 come back as: **this build's per-page reading is Mesen 2's
exactly**, and the "weird banks" sentence is the consensus of all four. Nothing to change.

## Q-161 — mappers 112 and 193

* **(a) mapper 112's `$C000-$FFFF`: CONFIRMED four for four.** The page describes nothing there and
  every project fixes the top: Mesen 2 `SelectPrgPage(2, -2)` and `(3, -1)`
  (`Core/NES/Mappers/Ntdec/Mapper112.h:27-28`), FCEUX `setprg16(0xC000, ~0)`
  (`src/boards/112.cpp:68`), puNES banks `0xFE`/`0xFF` (`mapper_112.c:86-89`), Nestopia maps only the
  two switchable windows and leaves the rest on its board default (the last 16 KiB). This build's
  borrowing of the 206's "last two fixed to the end" is unanimous.
* **(b) bit 0 of the two 2 KiB CHR selects: CONFIRMED, 3-1.** FCEUX `setchr2(0x0000, reg[2] >> 1)`
  (`src/boards/112.cpp:43`), puNES `memmap_auto_2k(…, m112.reg[2] >> 1)` (`mapper_112.c:93`) and
  Nestopia `banks.chr[address] = data >> uint(address < 4)` (`NstBoardNtdec.cpp:125` — the shift
  applied only to registers 2 and 3) all discard bit 0, which is the 206's "only bits 5-1 exist for
  2KB CHR banks" and this build's reading. **Mesen 2 alone keeps it**:
  `SelectChrPage2x(0, _registers[2])` (`Mapper112.h:45`) maps 1 KiB pages `page` and `page+1`
  (`BaseMapper.cpp:413-417`), so an odd register value gives a misaligned pair. Three for three with
  this build, and the dissenter is the one project whose helper makes the mistake easy.
* **(c) mapper 193's `$6004` mirroring register: CONFIRMED by the one project that decodes it.**
  **puNES implements it with this build's polarity** — `mapper_193.c:73-77`, `reg[4] & 0x01` →
  horizontal, else vertical, which is the page's "0 = vertical, 1 = horizontal". Mesen 2
  (`Core/NES/Mappers/Ntdec/NtdecTc112.h:23`, `switch(addr & 0x03)`), FCEUX
  (`src/boards/193.cpp`, `SetWriteHandler(0x6000, 0x6003, …)`) and Nestopia
  (`NstBoardNtdec.cpp:151-161`, `address & 0x03`) all decode **four** registers and have no mirroring
  register at all — i.e. three omissions of a register the page documents and says no game uses, which
  is not a contradiction. The census was right to ask for the test.
* **(c′) and that is the same finding as the mask.** The page's "Range,Mask: `$6000-7FFF`, `$E007`"
  decodes A2, which is the only way `$6004` is reachable; **this build and puNES both use the
  eight-register decode** (`m193.reg[address & 0x07]`, `mapper_193.c:52`) while the other three mask
  to two bits. So the three that have no `$6004` could not have one.
* **(d) `prg_ram: Some(0)`: CONFIRMED four for four.** No project puts RAM behind mapper 193's
  register window: Mesen 2 claims `$6000-$7FFF` as a register range, FCEUX allocates no WRAM for the
  number at all, Nestopia maps the range to its poke handler, and in puNES the whole range is the
  register file. The mask settles it, as the ticket says, and nobody disagrees.
* **Two side notes.** The page's three CHR registers are a *shifted* byte in every project — FCEUX
  `reg[0] >> 2`, `reg[1] >> 1`, `reg[2] >> 1` (`src/boards/193.cpp:40-42`), Nestopia the same
  (`NstBoardNtdec.cpp:157-159`), Mesen 2 the same (`NtdecTc112.h:25-36`), puNES the same
  (`mapper_193.c:67-70`) — which is this build's "the bank is the byte *shifted* and not masked".
  And on mapper 112's mirroring bit, Mesen 2 (`Mapper112.h:60`, `value & 0x01 ? Horizontal :
  Vertical`), FCEUX (`src/boards/112.cpp:40` + `:28`, `setmirror(mirror ^ 1)` with `MI_H = 0`,
  `MI_V = 1`) and puNES (`mapper_112.c:96-100`) all read bit 0 set as horizontal, which is this
  build's reading of the page's parentheses.

## Q-162 — mapper 298's `$F003`

**Both halves CONFIRMED, and puNES independently chose this build's architecture.**

`third_party/punes/src/core/mappers/mapper_298.c:31` is one line:
`init_VRC2and4(VRC24_VRC4, 0x02, 0x01, FALSE, info.reset)` — the shared VRC2/VRC4 chip model, in the
**VRC4b** pin configuration (CPU `$02` to A0, `$01` to A1), with a fourth argument. That argument is
`irq_repeated` (`VRC2and4.c:199`), and the **only** place it is read is the `$F003` arm of the IRQ
register group: `VRC2and4.c:139`, `if (vrc2and4tmp.irq_repeated) { enabled = acknowledge; }` followed
by the acknowledge. Every Konami number passes **TRUE** (`mapper_021.c:34`, `mapper_023.c:35`,
`mapper_025.c:34`, `mapper_022.c:33`, `mapper_014.c:59`, `mapper_183.c:43`); **mapper 298 passes
FALSE**.

So puNES:

1. implements **the page's clause and nothing else** — at `$F003` mapper 298 acknowledges a pending
   IRQ and does **not** copy `A` into `E`, where the Konami chips do;
2. with **the same shape this build chose**: one shared board, one **load-time** per-number flag, no
   second class and no `const` monomorphisation. That is Q-162's item (b), confirmed by an
   independent implementation rather than argued from a measurement.
3. and it never touches the counter in that arm — so **"does not reload the counter" is true of both
   chips**, which is item (a)'s other half: §7.2's sentence was a paraphrase, not a second
   observation. **No project reloads the counter on an acknowledge**, Konami or clone.

The other three do not model the number as a VRC4 at all, which is itself informative:

* **Mesen 2** `Core/NES/Mappers/Ntdec/Tf1201.h` is a hand-written board whose IRQ is a 341/3 CPU-cycle
  scaler, with a single enable bit at `$F001` and **`$F003` clearing the IRQ source and nothing else**
  (`:115-117`). The *effect* matches the page — acknowledge only — but there is no `A` bit for the
  question to arise about.
* **FCEUX** `src/boards/tf-1201.cpp:82-83` makes `$F003` **a second copy of `$F001`**: it sets the
  enable from the written value, acknowledges, and subtracts 8 from the counter outside vblank.
* **Nestopia UE** does the same by mapping the two addresses to one handler
  (`NstBoardUnlTf1201.cpp:62`, `Map(0xF003 + i, &Tf1201::Poke_F001)`).

Neither of those two is the VRC4's acknowledge either, so neither contradicts the page's clause; they
simply describe a different chip. Between the page, this build and puNES on one side and two
hand-rolled scanline counters on the other, this build's reading stands and its *shape* is
corroborated.

## Q-163 — mapper 218 (Magic Floor)

**CONFIRMED, three for three** (Nestopia UE does not support the number), and the agreement is
bit-for-bit.

All three projects serve the **pattern tables** from the 2 KiB nametable memory — which is the half
`Mapper::nametable_page` cannot reach and the whole point of this ticket:

* **Mesen 2** `Core/NES/Mappers/Homebrew/MagicFloor218.h:28` maps all eight 1 KiB pattern windows with
  `ChrMemoryType::NametableRam`, choosing the page as `((i * 0x400) & mask)` where `mask` is `$0400`,
  `$0800`, `$1000` or `$2000` — **the four PPU lines of this build's table**, derived from the header's
  mirroring exactly as this build derives them (including `Byte6 & 0x01` picking screen A or B when
  the four-screen bit is set).
* **FCEUX** `src/boards/datalatch.cpp:483-490` writes the table out as a literal array, with the
  comment "this could be done by bit logic, but this is self-documenting":
  `0,1,0,1,0,1,0,1` / `0,0,1,1,0,0,1,1` / `0,0,0,0,1,1,1,1` / `0,0,0,0,0,0,0,0` for the four values of
  the two header bits, pointed at `NTARAM`, with `PPUCHRRAM = 0xFF` so all eight are writable. **That
  is this build's four rows, in order.**
* **puNES** `src/core/mappers/mapper_218.c:40-62` computes the same thing: a shift of 1, 0, 2 or 3
  from `iNES` byte 6 bits 3 and 0 (`va10()`), applied as `(i >> shift) & 1` for the eight pattern
  windows and `((i | 0x08) >> shift) & 1` for the four nametables, through
  `memmap_chrrom_nmt_1k` — the cartridge-side nametable memory serving the pattern tables, i.e.
  **reading (b)**.

Expand any of the three and you get this build's table, nametable column included:

| line | iNES byte 6 | the four nametables | the eight pattern windows |
| ---: | --- | --- | --- |
| A10 | `$A1` | `0 1 0 1` | `0 1 0 1 0 1 0 1` |
| A11 | `$A0` | `0 0 1 1` | `0 0 1 1 0 0 1 1` |
| A12 | `$A8` | `0 0 0 0` | `0 0 0 0 1 1 1 1` |
| A13 | `$A9` | `1 1 1 1` | `0 0 0 0 0 0 0 0` |

And **no project does it through a nametable-page hook**: each one remaps the pattern-table windows,
which is precisely §7.2's under-pricing. Mesen 2 and FCEUX point the PPU at the console's own RAM
(the ticket's reading (a) made to work by a hook this core has not got) and puNES keeps the memory on
the cartridge side as this build does; the two are indistinguishable to a game, which is the ticket's
own argument ("it is the same chip either way"). **Nothing to change, and the ticket's question 1 is
answered: reading (b) is right, and reading (a) is only reachable with a PPU/cartridge boundary change
that two of the three projects happen to have.**

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

### CHANGE 4 — mapper 163's D0/D1 swap should probably cover `$5100` as well (conditional)

* **Ticket**: Q-155, reading (a). **Do not apply without R0's confirmation** — the page is ambiguous
  and the evidence is one implementation, but that implementation is the only one that has the
  mechanism at all.
* **File / function**: `rust/crates/zax-nes/src/cartridges/mappers/chr_window.rs`,
  `ChrWindowBoard::bank_port_value` (and its call sites).
* **The edit**: apply the D0/D1 exchange to the `$5100-$5101` feedback port too, not only to `$5000`
  and `$5200` — i.e. make the swap a property of the `$5xxx` decode's data path rather than of the two
  bank ports. The swap must be applied **before** the `$5101` "flip F when E=1" test reads bit 0, so
  that a game writing `$01` to `$5101` with the swap on arrives as `$02` and flips nothing. That is
  the sequence the ticket already documents as the discriminator, with its expected answer inverted
  (`$5500` answers `$04` instead of `$00`).
* **A second, independent change worth adopting with it** (puNES has it and no page states it):
  exempt `$5200` from the swap on images **smaller than 2 MiB**, because on a 1 MiB board both of the
  ASIC's PRG A19 and A20 outputs go to ROM A19
  (`third_party/punes/src/core/mappers/mapper_163.c:88-93`). If reading (a) is adopted without this,
  the 1 MiB titles get a wrong outer bank whenever the swap bit is set.
* **Titles affected**: the six category-A mapper-163 titles, and only those that set `$5300` bit 0;
  the page's own *Notes* ("cartridges should be dumped with `$5300=$04`") means most dumps circulate
  with the swap **off**, so the blast radius is small and the symptom is a failed protection check
  rather than a wrong tile.
* **Risk to existing boards**: `ChrWindowBoard` carries mappers 162, 163 and (per the lot) their
  siblings; the swap is gated on `$5300` bit 0, which only mapper 163 has, so no other number is
  reachable. No save-state byte moves. The existing `q_155_…` test's expectations invert, which is
  the point of the test.

### CHANGE 5 — mapper 268's refused submapper groups are implementable (scope note)

* **Ticket**: Q-157. **Not a defect**: no original title is behind a refused submapper, as the ticket
  establishes. This entry exists so that a lot which decides to widen the board does not start from
  the page alone.
* **Where to look**: FCEUX `third_party/fceux/src/boards/coolboy.cpp` and puNES
  `third_party/punes/src/core/mappers/mapper_268.c` both implement the four layouts this build
  refuses, as a register base plus one flag per group (FCEUX `:408-438`; puNES `:106-133`). The two
  features that are more than a bit shuffle are named in both: the submapper-**6/7** two-chip PRG
  select (the page's `D` bit, FCEUX `:239-242`, puNES `:126-132`) and the submapper-**8/9** CHR-RAM
  write protect (FCEUX `:135-140`, puNES `:161`).
* **The shape this build would need**: `Mmc3Outer::new` already answers `None` above submapper 1, so
  the change is a `const` row per group behind that gate — the ticket's own "each is a `const` table
  and a handful of `match` arms". Both projects put the register base in a variable rather than a
  constant, which is also what submappers 2/3's `$7000-$7FFF` window needs.
* **Risk**: none to the served submappers if the new arms are `const`-selected at load; the board is
  already feature-gated. Submapper 10/11 should **stay refused** — FCEUX refuses them too, with an
  error message, and puNES silently runs them on the COOLBOY layout, which is the failure mode
  reading (b) exists to avoid.

---

## OPEN — the points that stay as they are, and what would close each

| ticket | point | current choice | what would close it |
| --- | --- | --- | --- |
| Q-141 | 150/243 R7 mode 3 | `SingleScreenB` (`S1`) | any of the fifteen Sachen titles observed with mode 3 selected; a blank/stale screen is the symptom. Mesen 2 says page 0, puNES says page 1, and FCEUX's and Nestopia's page-0 cell comes with a mode table that is also wrong against the page, so it is not independent. |
| Q-142 | 195's power-on row | `$80` (`$28-$2B`) | a mapper-195 dump that boots at `$00-$03` and not at `$28-$2B`. All four start at `$00-$03`; none states a power-on value of its own. |
| Q-144 §3 | 115's `$6000` bit 6 = PRG A18 | driven in both modes | a mapper-115 image larger than 256 KiB. None is known, and three of the four cap the board's PRG below the point where the bit could matter. |
| Q-144 §4 | 121/187's CHR A18 | PPU A12, both modes, right table = second half | *Street Fighter Zero 2 '97* or *The King of Fighters '96*, one screen. See CHANGE 3. |
| Q-147 §1 | 38's `$F000-$FFFF` | banks, as the page infers | a *Crime Busters* trace with a write in `$F000-$FFFF`, or a photograph of the UNL-PCI556's 74138 enables. All four decode `$7000-$7FFF` only (Nestopia widens downward to `$6000` instead). |
| Q-148 | mapper 35's board | 209's board + 8 KiB WRAM | whether *Warioland II* writes `$C001` and `$C004`. Mesen 2 and FCEUX run the number's one title on the **UNL-SC-127** board, whose registers are a subset of the J.Y. ASIC's except for the counter: a filtered-A12 down-counter against this chip's prescaler-driven one. If the dump never selects a source, the SC-127 is what it was built for. |
| Q-150 §3 | the CHR mask and outer bank with `L=1` | masked to 512 KiB, `cc` applies | a J.Y. image with more than 512 KiB of CHR-ROM that sets `$D003` bit 5. puNES agrees with this build; Mesen 2, FCEUX and Nestopia use the raw 16-bit register pair, which is one FCEUX-derived expression. |
| Q-151 §2 | disabling resets the prescaler | yes | a game that disables and re-enables without rewriting `$C004`. Only puNES implements the page's third effect; the other three simply omit it. |
| Q-151 §4 | IRQ source 3, "CPU writes" | documented, not clocked | whether any of the thirteen titles writes `$C001` with bits 1-0 = 3. FCEUX and Nestopia have the same gap; Mesen 2 implements it with a CPU-level "is this a write" query this core has not got; **puNES gets a cheap partial** by clocking on cartridge writes only, which this core could copy on the existing `Cartridge::cpu_write` path. |
| Q-154 §2 | 178's register decode | `$4800-$4803` exactly | a mapper-178 game writing a mirror (`$4C00`, `$4801`+`$100`…). **No project decodes only four addresses**: Mesen 2 and FCEUX use `$4800-$4FFF`, puNES `$4800-$48FF`, Nestopia three single addresses. If a mirror is ever seen in use, `$4800-$4FFF` has two votes. |
| Q-155 | 163's D0/D1 swap scope | reading (b), `$5000` and `$5200` | **the lean has changed**: puNES is the only project with the mechanism and it implements reading (a) — all three ports, `$5100` included — with a hardware note about the 1 MiB boards' A19/A20 wiring. One mapper-163 game that sets `$5300` bit 0 and uses the protection closes it. See §CHANGE 4. |
| Q-158 §1 | 268's PRG A25 interleave | the first 32 MiB half | a 64 MiB dump whose halves differ at the reset vector. **None of the three projects that support the number implements SC0/SC1 or PRG A25 at all**, so this build is not behind the field. |
| Q-159 | 268's `$5000-$5FFF` PRG-RAM | COOLBOY only, the chip's second 4 KiB, `$A001`-gated | one dump that sets `$xxx3` bit 5. **Nobody implements the bit**: FCEUX's `$5000` write window is an unconditional crash workaround by its own comment, and Mesen 2 and puNES do not decode it. All three of this build's readings stand unchallenged. |
