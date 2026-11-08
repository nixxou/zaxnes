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

| Q-164 §1 | 12 | one chip (MMC3A clone), not mapper 116's board | one chip | one chip | one chip | one chip | one chip | one chip | **CONFIRMED** |
| Q-164 §2 | 12 | `$4132`'s power-on value | not given | zero | zero | zero | zero | zero | **CONFIRMED** |
| Q-164 §3 | 12 | which value of the language bit is Chinese | not said; "Chinese in all known copies" | `$00` | — | **1** ("chinese is default") | **`$01`** | a dip switch | **CHANGE** |
| Q-164 §4 | 12 | `$4132` writes take effect immediately | yes (the PCB) | yes | yes | **no (latched)** | yes | yes | **CONFIRMED** |
| Q-164 §5 | 12 | submapper 1 (Magic Card 4M) | copier hardware | refused | run as an MMC3A | run as an MMC3A | run as an MMC3A | **run as the FFE board** | **CONFIRMED** |
| Q-165 §1 | 14 | the supervisor register's bit positions | the diagram: 1/3/5/7 | 1/3/5/7 | 1/3/5/7 | 1/3/5/7 | 1/3/5/7 | 1/3/5/7 | **CONFIRMED** |
| Q-165 §2 | 14 | the decode | `$A131`, mask doubted | the full 16 bits | `addr == $A131` | `A == $A131` | `(addr & $A131) == $A131` | `address == $A131` | **CONFIRMED** |
| Q-165 §3 | 14 | the supervisor's power-on value | not given | `$02` (MMC3 mode) | `0` (VRC2) | `0` (VRC2) | `0` (VRC2) | `0` (VRC2) | **OPEN** |
| Q-165 §4 | 14 | which chip takes a mode-changing write | not said | the **new** mode's | the new mode's | the new mode's | the new mode's | neither (dropped) | **CONFIRMED** |
| Q-165 §5a | 14 | mirroring only in VRC2 mode | yes (per Nestopia) | implemented | no | no | **implemented** | no | **OPEN** |
| Q-165 §5b | 14 | the VRC2 registers ignore address mirrors | yes (per Nestopia) | exact addresses | A1/A0 decoded | A1/A0 decoded | A1/A0 decoded | fully mirrored | **CONFIRMED** |
| Q-165 §6 | 14 | no MMC3A, no VRC2 `$FF` CHR power-on | not stated for 14 | neither given | neither | neither | neither | neither | **CONFIRMED** |
| Q-166 §1 | 83 | the scratch RAM's decode | printed mask impossible; the `A~` row | `(a & $F30C) == $5100` | `$5100-$5103` | `$5100-$5103` | n/s | A8/A10 over `$5000-$5FFF` | **OPEN** |
| Q-166 §2 | 83 | the solder pad at `$5000` | nothing configures it | `const $00`, zeros above | **a 2-bit dip switch**, open bus above | **a reset-cycled switch**, open bus above | n/s | **a dip switch** | **CONFIRMED (gap)** |
| Q-166 §3 | 83 | the `$8318` IRQ source select | "not known which bit" | any non-zero = A12 | — | — | n/s | — | **OPEN** |
| Q-166 §4 | 83 | `$6000-$7FFF` in PRG modes 0/1 | not said | the same composition | — | — | n/s | **same, enable is mode-independent** | **CONFIRMED** |
| Q-167 | 83.2 | the PRG-NVRAM variant | documented | refused | n/s | n/s | n/s | **implemented** | **CONFIRMED (form)** |
| Q-168 §1 | 91 | `$8000-$9FFF` outer bank: A2 = PRG A18 | the diagram's columns | A2 = A18 | — | — | — | **A2 = A18** | **CONFIRMED** |
| Q-168 §2 | 91 | "Mask: unknown" | the page's own range | `$8000-$9FFF` | — | — | — | **`$8000-$9FFF`** | **CONFIRMED** |
| Q-168 §3a | 91.0 | the ÷64 counter's firing rule | not stated | carry out, keeps running | repeats (MMC3 latch 7) | **stops at 8** | stops at 8 | **stops at 8** | **OPEN** |
| Q-168 §3b | 91.1 | the 5/4 M2 counter fires on the borrow | not stated | borrow out | — | — | — | **borrow out** | **CONFIRMED** |
| Q-168 §4 | 91.0 | "hard-wired to last bank" of what | not said | of the inner 128 KiB | of the ROM | of the ROM | of the ROM | **of the inner 128 KiB** | **CONFIRMED** |
| Q-168 §5 | 91.1 | `$6004` selects horizontal | the page's word | horizontal | — | — | — | **horizontal** | **CONFIRMED** |

| Q-169 §1 | 111 | the flash save is gated on the battery bit? | — | no | no (a user option) | **yes** | n/s | no | **CONFIRMED** |
| Q-169 §2 | 111 | the file is a diff, not a chip image | — | changed sectors | a ROM diff/patch | the whole 512 KiB | n/s | — | **CONFIRMED** |
| Q-169 §4 | 111 | the savestate carries the changed bytes | — | yes | yes (a ROM diff) | yes (the whole chip) | n/s | — | **CONFIRMED** |
| Q-169 §3 | 111 | `$90`/`$F0`/`$10` unimplemented | documented | not built | **built** | partly | n/s | — | **CONFIRMED (gap)** |
| Q-170 §1.1 | 164 | the 93C66's word size | **8-bit** (ORG to GND) | 8-bit | — | — | n/s | **8-bit** | **CONFIRMED** |
| Q-170 §1.2 | 164 | a type of its own, not the I²C part | — | a second type | — | — | n/s | **a second type** | **CONFIRMED** |
| Q-170 §1.3 | 164 | the 512 bytes live in the battery slice | — | `prg_ram` behind `battery_offset` | — | — | n/s | **the NVRAM slice** | **CONFIRMED** |
| Q-170 §2.1 | 164 | the second EEPROM chip | "unpopulated on all known games" | bit 6 decoded, inert | — | — | n/s | **bit 6 decoded, inert** | **CONFIRMED** |
| Q-170 §2.3 | 164 | the volatile RAM's size | 2 KiB mirrored on all known carts | 2 KiB mirrored | — | — | n/s | the header's, mirrored | **OPEN** |
| Q-170 §2.4 | 164 | the 1 bpp substitution on PPU writes | written as a fetch | yes | — | — | n/s | **yes** | **CONFIRMED** |
| Q-171 §1 | 208 | a "wrong code" state | none described | none | none | none | n/s | none | **CONFIRMED** |
| Q-171 §2 | 215.1 | `$5001`'s two overlaid bit rows | ambiguous | PRG 3/1/0, CHR 3/2/1 | submapper 0's layout only | — | n/s | **PRG 3/1/0, CHR 3/2/1** | **CONFIRMED** |
| Q-171 §3 | 215 | `$5001`'s power-up value | `$xF` | `$0F` | `$03` | — | n/s | `$03` | **OPEN** |
| Q-171 §4 | 208.0 | `$A000` inert, the board's register instead | implied | inert | — | — | n/s | **inert, by submapper** | **CONFIRMED** |
| Q-171 §5 | 208 | the protection index is the written value | ambiguous | the value | — | — | n/s | **the value** | **CONFIRMED** |
| Q-171 §6.1 | 219 | the outer bank is inside `$5000-$5FFF` | "Mask: `$5003`" | `$F003` | — | — | n/s | **inside `$5000-$5FFF`** | **CONFIRMED** |
| Q-171 §6.2 | 219 | the mode bit latched by `$8002` only | derived | yes | `$8002` decoded | — | n/s | **yes, `address & 0x0002`** | **CONFIRMED** |
| Q-171 §6.2' | 219 | the extended CHR latch is persistent | "next" vs "latch" | persistent | — | — | n/s | **persistent** | **CONFIRMED** |
| Q-172 §1.1 | 262 | `$4100`'s CHR-RAM select is bit 6 | the diagram's column | bit 6 | **bit 6** | — | **bit 6** | **bit 6** | **CONFIRMED** |
| Q-172 §1.2 | 262 | the four CHR A18 lines are crossed | the annotations' columns | `[3,3,2,2,0,0,1,1]` | **identical** | — | — | — | **CONFIRMED** |
| Q-172 §1.3 | 262 | the CHR-RAM chip's own address lines | silent | banked by the low 3 bits | **unbanked** | — | **unbanked** | **unbanked** | **CHANGE** |
| Q-172 §1.4 | 262 | the jumper | no header field | `$00` | 0, reset-toggled | — | a dip switch | a dip switch | **CONFIRMED** |
| Q-172 §2 | 292 | where the filled byte comes from | the last CPU write to any address | a bus latch on every write | **zero page `$6A`/`$FF` read directly** | — | — | **zero page `$6A`/`$FF` read directly** | **CHANGE** |
| Q-172 §2.1 | 292 | `R` is bit 5 | the annotation's column | bit 5 | **bit 5** | — | — | **bit 5** | **CONFIRMED** |
| Q-172 §2.2 | 292 | "registers 2/3" when CHR inversion is set | stated | the chip's own window bank | register 0 always | — | — | register 0 always | **OPEN** |
| Q-172 §3.1 | 246 | the register window | `$6000-$601F` (`$FFE4`) | `$6000-$601F` | `$6000-$67FF` | `$6000-$67FF` | — | `$6000-$67FF` | **OPEN** |
| Q-172 §3.1' | 246 | PRG A17 forced on `address & $FFE4 == $FFE4` | stated | the same constant | — | — | — | **the same constant** | **CONFIRMED** |
| Q-172 §3.2 | 246 | the 2 KiB RAM window | `$6800-$6FFF` only | mirrored four times | mirrored twice | **exact** | — | **exact** | **CONFIRMED (cost)** |
| Q-172 §3.3 | 246 | `$6003` is `$FF` at power-on **only** | "only" | kept across a reset | restored on reset | **kept** | — | **kept** | **CONFIRMED** |

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

## Q-164 — mapper 12 (Huang-1 on the SL-5020B)

Boards: Mesen 2 `Core/NES/Mappers/Mmc3Variants/MMC3_12.h`; FCEUX `src/boards/mmc3.cpp:377-409`;
Nestopia UE `source/core/board/NstBoardRexSoftDb5z.cpp`; puNES
`src/core/mappers/mapper_012.c`.

* **§1, one chip: CONFIRMED four for four.** Every project builds mapper 12 as an **MMC3 clone with
  one extra register** and nothing else — Mesen 2 and Nestopia as `MMC3`/`Mmc3` subclasses, FCEUX
  through `GenMMC3_Init` with `isRevB = 0`, puNES through `init_MMC3`. **No project gives the number a
  supervisor register, a VRC2 mode or an MMC1 mode**, so the census's "it is `Somari` with one more
  register" is wrong and the page is right. The MMC3A requirement is honoured where it is expressible:
  Mesen 2 `MMC3_12.h:11` (`ForceMmc3RevAIrqs`), FCEUX `mmc3.cpp:409` (`isRevB = 0`).
* **§2, the power-on value: CONFIRMED four for four at zero.** Mesen 2 `_chrSelection = 0` in
  `InitMapper`, FCEUX `EXPREGS[0] = EXPREGS[1] = 0` (`mmc3.cpp:393`), Nestopia `exReg = 0`, puNES
  `memset(&m012, 0)`.
* **§3, the language bit: CHANGE.** See §CHANGE 6. **FCEUX initialises it to 1 with the comment
  "chinese is default"** (`mmc3.cpp:394`) and **Nestopia answers a hard-coded `0x01`**
  (`NstBoardRexSoftDb5z.cpp:83`); puNES answers a user dip switch
  (`mapper_012.c:75`); Mesen 2 does not implement the read. This build answers `$00`, which on two
  projects' reading selects **English** on a board the page says is wired to Chinese in every known
  copy.
* **§4, writes take effect immediately: CONFIRMED, and the page's attribution is verified.** Mesen 2
  calls `UpdateState()` in the write (`MMC3_12.h:42`), Nestopia calls `Mmc3::UpdateChr()`
  (`NstBoardRexSoftDb5z.cpp:73`), puNES calls `MMC3_chr_fix()` (`mapper_012.c:67`). **FCEUX's
  `M12Write` (`mmc3.cpp:383-386`) sets the two latches and calls no refresh at all**, so the change
  lands on the next MMC3 CHR register write — exactly the behaviour the page describes as FCEUX's and
  says the homebrew *Ultimate Mortal Kombat III* depends on. Three for three with this build and the
  PCB.
* **The A18 wiring itself is five for five**: per **pattern table**, from register bits 0 and 4 —
  Mesen 2 `MMC3_12.h:27-32`, FCEUX `(EXPREGS[(A & 0x1000) >> 12] << 8)` (`mmc3.cpp:380`), Nestopia
  `exReg << ((address & 0x1000) ? 4 : 8) & 0x100` (`:88`), puNES
  `(m012.reg << (((address >> 10) >= 4) ? 4 : 8)) & 0x0100` (`mapper_012.c:88`). Not in dispute, but it
  is the half of the board the census got right and worth recording as unanimous.
* **§5, submapper 1: CONFIRMED as a defensible refusal, with one project doing better.** Mesen 2,
  FCEUX and Nestopia read no submapper, so a Magic Card 4M image runs on them as an MMC3A clone it is
  not. **puNES routes it to its Front Fareast board** — `mapper_012.c:31-32`,
  `if (info.mapper.submapper == 1) map_init_006()`. So refusing is better than three of the four do,
  and the fourth shows the alternative: delegate to an FFE implementation, which this build does not
  have (the census leaves mappers 6, 8 and 17 out). Keep the refusal.

## Q-165 — mapper 14 (Gouder SL-1632)

Boards: Mesen 2 `Core/NES/Mappers/Mmc3Variants/MMC3_14.h`; FCEUX `src/boards/sl1632.cpp`; Nestopia UE
`source/core/board/NstBoardRexSoftSl1632.cpp`; puNES `src/core/mappers/mapper_014.c`.

* **§1, the bit positions: CONFIRMED four for four, and the prose paraphrase is refuted.** All four
  read the mode at **bit 1** and the three CHR A18 lines at **bits 3, 5 and 7** — Mesen 2
  `_mode & 0x02` / `0x08` / `0x20` / `0x80` (`MMC3_14.h:36-38`, `:69`), FCEUX
  `(bbrk & 0x08) << 5` / `(bbrk & 0x20) << 3` / `(bbrk & 0x80) << 1` and `if (bbrk & 2)`
  (`sl1632.cpp:49-51`, `:64`), Nestopia `exMode & 0x2U` plus a shift table, puNES `chr_base()`'s three
  arms (`mapper_014.c:133-139`) and `m014.reg & 0x02`. **And all four group the windows exactly as
  this build does**: bit 3 over `$0000-$0FFF`, bit 5 over `$1000-$17FF`, bit 7 over `$1800-$1FFF`.
  Nothing anywhere supports the one-place-lower reading.
* **§2, the decode: CONFIRMED.** Mesen 2 `addr == 0xA131` (`MMC3_14.h:65`), FCEUX `A == 0xA131`
  (`sl1632.cpp:63`) and puNES `address == 0xA131` (`mapper_014.c:74`) are **exact equality**, as this
  build is. Nestopia uses `(address & 0xA131) == 0xA131` (`NstBoardRexSoftSl1632.cpp:135`), which is
  looser but still recognises `$A131`. So the page's doubted "Mask: $FFFF?" is three for three.
* **§3, the power-on value: OPEN, and this build is the outlier.** Mesen 2 (`_mode = 0` in
  `InitMapper`), FCEUX (`bbrk` left at its static zero) and Nestopia (`exMode = 0` in `SubReset`) and
  puNES (`memset`) all start in **VRC2 mode**; this build starts at `$02`, MMC3 mode. But none of the
  three *chose* VRC2 mode — it is zero-initialised memory in every case, not a finding, and the page
  gives no value. Keep `$02`, and note that both modes reach a reset vector (FCEUX's VRC2 half fixes
  the same last two 8 KiB banks), so the one known title runs either way. **What would close it**: a
  *Samurai Spirits* trace showing whether it writes `$A131` before it banks.
* **§4, which chip takes a mode-changing write: CONFIRMED 3-1 for "after".** Mesen 2 assigns `_mode`
  and *then* tests it (`MMC3_14.h:65-71`), FCEUX the same (`sl1632.cpp:63-66`), and Nestopia the same
  — and both FCEUX and Nestopia then forward the write to the MMC3, where `$A131 & $E001` is `$A001`,
  **the same register this build gives it** (Nestopia spells it out: `case 0xA001:
  Mmc3::NES_DO_POKE(A001, …)`). puNES alone consumes the supervisor write entirely
  (`mapper_014.c:74-78`, a bare `return`).
* **§5a, "mirroring can only be controlled in VRC2 mode": OPEN, and the page's attribution is
  verified.** **Nestopia implements it, and in this build's exact shape**:
  `NstBoardRexSoftSl1632.cpp:152`, the MMC3-mode `$A000` arm is `SetMirroringVH( exNmt )` — it
  re-applies the value the **VRC2's `$9000`** last set and ignores the datum, which is
  `Mmc3Board::force_ignore_a000` plus "whatever `$9000` last set survives a mode switch". Mesen 2,
  FCEUX and puNES all forward MMC3-mode `$A000` to the MMC3's own mirroring, i.e. they contradict it —
  which is precisely what the page says ("These behaviors contradict other emulators and have not been
  verified on original hardware"). So the page's sentence is Nestopia's, the page's caveat is accurate,
  and the lot's rule (the page wins) stands with the caveat recorded. **What would close it**: original
  hardware, as the page says.
* **§5b, "the VRC2 PRG and mirroring registers do not respond to address mirrors": CONFIRMED in
  substance.** Mesen 2 (`addr & 0xF003`), FCEUX (`A & 0xF003`, `sl1632.cpp:78`) and Nestopia
  (`address & 0xF003`) all **decode A1 and A0**, so the VRC2's `$8000` answers at `$8000` and not at
  `$8001-$8003` — which is the rule. puNES uses a fully mirrored VRC2 (`address & 0xF000` plus the
  A0/A1 pin map), i.e. a real VRC2. **This build is stricter than any of them** (the three registers
  answer at those addresses exactly, A2-A11 decoded as well), and one observation is worth recording:
  the stricter form is **not needed** for the consequence it was adopted for. `$A131 & $F003` is
  `$A001`, not `$A000`, so A1/A0 alone already stop a supervisor write from clobbering the VRC2's
  second PRG bank. A later lot could relax the decode to `& $F003` and keep the protection.
* **§6, no MMC3A and no VRC2 `$FF` power-on for this number: CONFIRMED four for four.** None of the
  four forces the revision-A counter for mapper 14 (FCEUX notably *does* for mapper 12, three lines
  away) and none powers its CHR registers on at `$FF`.

## Q-166 and Q-167 — the Cony / Yoko board (mappers 83 and 264)

**Nestopia UE does not support either number.** Mesen 2 is `Core/NES/Mappers/Unlicensed/Mapper83.h`
plus `Yoko` for 264; FCEUX is `src/boards/yoko.cpp` with two init variants; puNES is
`src/core/mappers/mapper_083.c` for **both** numbers.

* **Q-166 §1, the scratch RAM's decode: OPEN.** The page's printed mask is impossible and nobody
  implements the `A~` row's undecoded bits. Mesen 2 claims exactly `$5100-$5103`
  (`Mapper83.h:37`) and so does FCEUX (`yoko.cpp:171-172`, with `low[A & 3]`); puNES decodes the four
  registers across the **whole** of `$5000-$5FFF`, discriminating the pad from the RAM by a single
  address bit (`mapper_083.c:148`, A8 on mapper 83 and A10 on mapper 264). So three implementations,
  three decodes, and the only thing all of them share is that **the four addresses themselves must
  work** — which this build satisfies. Keep the `A~` reading; it is the page's only self-consistent
  statement, and the mirrors it admits (`$5110`) are reachable by nobody else, so nothing can depend on
  them. Note puNES's bit-based split as an alternative if a dump ever shows a mirror in use.
* **Q-166 §2, the solder pad: CONFIRMED as a gap, and the shape of the answer is now known.** All
  three projects expose the pad **to the user**: Mesen 2 declares `GetDipSwitchCount() 2`
  (`Mapper83.h:19`) and answers `$5000` with `(openBus & 0xFC) | GetDipSwitches()` (`:104-105`);
  FCEUX keeps a `dip` that its **reset handler cycles** (`yoko.cpp:181`, `dip = (dip + 1) & 3`) and
  answers `(X.DB & 0xFC) | dip` (`:142`); puNES answers `dipswitch.value`. So the page's "the good
  value is not always 0 or 3" is handled by letting the user pick, which is exactly the "host surface
  for per-cartridge configuration" the ticket says this constant should be wired to. Two further
  details: all three use **true open bus** in bits 7-2 where this build answers zero, and FCEUX's two
  init variants differ in the pad's default (`dip = 3` for one board, `0` for the other), which is
  evidence that the default is per-PCB and not per-chip.
* **Q-166 §3, the `$8318` IRQ source: OPEN, and this build is ahead of the field.** **No project
  implements the register.** Mesen 2 decrements once per CPU cycle and fires at zero
  (`Mapper83.h:61-66`), FCEUX subtracts the elapsed cycles (`yoko.cpp:197-204`), puNES's `$83xx`
  switch ignores index `$18` entirely. All three therefore implement the page's **default** value's
  behaviour (CPU M2) and nothing else. This build's "any non-zero value selects A12" is unrebutted and
  is the only reading that gives both documented values their documented meaning. Keep it.
* **Q-166 §4, `$6000-$7FFF` in PRG modes 0 and 1: CONFIRMED by puNES.** `wram_fix_083`
  (`mapper_083.c:226-232`) gates the window on `m083.mode & 0x20` — the mode register's bit 5 —
  **independently of the PRG banking mode**, and uses the `$8303` register's value in every mode,
  which is this build's "the same composition in every mode" and its reading that bit 5 is not
  silently mode-dependent.
* **Q-167, the five PCBs: CONFIRMED in form, and 83.2 is implementable.** puNES serves **83.2**, the
  one variant this build refuses: `m083tmp.use_wram = (submapper == 2)` (`mapper_083.c:84`) and
  `memmap_wram_8k(0, MMCPU(0x6000), m083.outer >> 6)` (`:227`) — the 32 KiB PRG-NVRAM banked from the
  PRG base register's top two bits, with `wram_set_nvram_size(S32K)` at load, exactly the layout this
  ticket documents in its "what would reopen it" paragraph. puNES also **infers the submapper from the
  CHR size** on an iNES 1.0 image (`:75-82`: ≥1 MiB → 83.2, ≥512 KiB → 83.1) and gives mapper 264 its
  own address permutation (`:112`), which is the page's A11/A10 → A9/A8 rewiring. Mesen 2 and FCEUX
  read no submapper at all, so a 83.2 dump runs on them with PRG-ROM where the NVRAM should be.
  So this build's refusal is the second-best outcome of four and the mechanism cost is the only thing
  standing in the way — see §CHANGE 2's note that mapper 195's `$5000` window wants the same
  `Mapper::prg_ram_window` forward.

## Q-168 — mapper 91

**puNES is the only other project that implements the outer bank register or submapper 1**, and where
it does, it agrees with this build on every point the ticket decided.

* **§1, the outer bank's bit order: CONFIRMED.** `mapper_091.c:119` latches
  `m091.reg = address & 0xFF` — **the address, not the datum**, which is the page's `A~` header — and
  `:158` composes `base = (m091.reg << 3) & ~0x0F`, i.e. **register bit 2 → bank bit 5 (PRG A18) and
  bit 1 → bank bit 4 (PRG A17)**. That is the diagram's column order, which is this build's
  `Mapper091::outer_prg`, and it refutes the "ascending list" reading. The CHR half matches too:
  `chr_fix_091`'s `base = (m091.reg & 0x01) << 8` is the page's `C` at bit 0 driving CHR A19.
* **§2, "Mask: unknown": CONFIRMED.** puNES decodes `case 0x8000: case 0x9000:` — the page's own
  `$8000-$9FFF` range and nothing tighter.
* **§3a, submapper 0's firing rule: OPEN, two-two.** This build's free-running ÷64 counter has Mesen 2
  on its side in effect — `Mapper91.h` borrows the MMC3's counter with a latch of **7**, which reloads
  and so **repeats** — while FCEUX (`src/boards/91.cpp:68`, `if (IRQCount < 8 && IRQa)`) and puNES
  (`mapper_091.c:139`, the same condition) implement the ticket's named alternative: the counter
  **stops at 8 and does not fire again until restarted**. A second, shared observation: all three
  implement the counter as **eight filtered scanline ticks** rather than sixty-four unfiltered A12
  rises, which is the same thing while rendering is on and a different thing while it is off. Keep the
  free-running reading (it is what a ÷64 ripple counter's carry out does) and note that the two
  readings differ only for a game that does not acknowledge.
* **§3b, submapper 1's 5/4 counter: CONFIRMED.** `mapper_091.c:147-152`: a two-bit prescaler, and
  every fourth M2 cycle `counter -= 5` with the IRQ asserted when the counter **goes at or below
  zero** — the borrow out, exactly this build's "the step whose starting value is below five". Nobody
  else implements this submapper at all.
* **§4, "hard-wired to last bank" of what: CONFIRMED.** puNES maps `$C000` and `$E000` as
  `base | (0xFE & mask)` and `base | (0xFF & mask)` with `mask = 0x0F` (`mapper_091.c:157-164`) — the
  last bank **within the outer 128 KiB bank**, which is this build's reading and the one that lets the
  outer register reach the fixed half of its own bank. The same lines confirm the other half of the
  decision: the two switchable registers are masked to four bits, so their upper bits are not latched.
* **§5, the mirroring vocabulary: CONFIRMED.** `mapper_091.c:78-85`: `$6004` sets horizontal and
  `$6005` vertical, which is the page's own word taken literally and this build's choice. The ticket's
  point that mapper 83's page and mapper 91's page use opposite vocabularies is borne out: puNES's
  mapper-83 mirroring arm reads the mode register's `NN` field the other way round.
* **§6, the census under-prices the lot: CONFIRMED.** puNES's mapper 91 has two counters (one per
  submapper), five register groups on submapper 1 and the outer bank register — the same inventory this
  build built and more than §7.2's "one counter and four groups".

## Q-169 — mapper 111's flash save

This ticket is about REQ-CORE-095's shape rather than about a register, so the four can only show what
the alternatives look like. Two of them do:

* **Mesen 2** `Core/NES/Mappers/Homebrew/Cheapocabra.h` declares `GetSaveRamSize() 0` (`:20`) and
  persists the chip as a **ROM patch** instead — `SaveBattery()` calls `SaveRom(_orgPrgRom)` (`:59-67`)
  and `ApplySaveData()` calls `LoadRomPatch`, with `SerializeRomDiff(s, _orgPrgRom)` in the savestate
  (`:47`). Gated not on the battery bit but on a **user option**, `DisableFlashSaves`. So decisions 1
  (a medium beside the battery RAM, not the battery bit), 2 (a diff and not a chip image) and 4 (the
  savestate carries the changed bytes) are each what Mesen 2 does, in its own vocabulary.
* **FCEUX** `src/boards/cheapocabra.cpp:251-257` does the opposite on two of the three: `flash =
  (info->battery != 0)` — **gated on the battery bit** — and it allocates a full 512 KiB `FLASHROM`
  copy which it persists whole through `addSaveGameBuf` and puts whole into the savestate.
* **puNES** has an `SST39SF040` model (`src/core/mappers/mapper_111.c:23`) and routes the board's
  reads and writes through it.
* **Nestopia UE** does not support the number.

**Verdicts**: decisions 1, 2 and 4 **CONFIRMED** — Mesen 2 independently reaches the same three
shapes, and FCEUX's battery-bit gate is the thing the ticket argues against (a GTROM dump is not
required to set the bit, so FCEUX silently loses the save on a dump that does not). Decision 3 (a
reset does not restore the chip) holds in all three: the bytes live in the PRG image.
**The `$90`/`$F0` software-ID and `$10` chip-erase gap is CONFIRMED as a gap and Mesen 2 fills it** —
`FlashSST39SF040.h:78-81` and `:108-115` implement `$80`/`$90`/`$A0`/`$F0` and the `$10` full erase.
So the ticket's "no game of either board's roster reads the chip's ID" is the only thing keeping it
out, and the cost (`Board::cpu_read_override`, +0.066 %) is the ticket's own figure.

## Q-170 — mapper 164's 93C66

**Only puNES implements the EEPROM at all, and it confirms every one of §1's three contradictions.**
Mesen 2's `Waixing164` is a 16-bit PRG bank register and nothing else; FCEUX's `Mapper164_Init`
(`src/boards/164.cpp`) likewise; Nestopia UE does not support the number.

* **§1.1, the word size: CONFIRMED.** `src/core/mappers/mapper_164.c:69` is
  `ee93cx6_init(wram_nvram_pnt(), wram_nvram_size(), 8)` — **eight-bit words**, the page's "the ORG pin
  is tied to GND" and not §7.2's 16. puNES's device is also keyed to a **512-byte** part
  (`:57`, `wram_nvram_size() == S512B`).
* **§1.2, a type of its own: CONFIRMED.** puNES's Microwire part is `EE93Cx6.c`, a separate file from
  its two-wire parts, exactly as `Eeprom93C66` is separate from `Eeprom24C0X` here.
* **§1.3, where the bytes live: CONFIRMED.** `wram_nvram_pnt()` — the **NVRAM slice**, which is
  puNES's equivalent of `battery_offset`'s region, not a field of the device.
* **§2.1, the second chip: CONFIRMED.** `mapper_164.c:84-95` writes the device from `$5200` bits 4
  (CS), 2 (CLK) and 0 (DI) and **transcribes the bit-6 "EEPROM #2 CS" line in a comment while driving
  nothing with it** — bit 6 decoded and inert, which is this build's choice word for word.
* **§2.2, the undriven DAT level: unresolved but narrowed.** puNES answers `$5500` bit 2 as the
  **inversion of its device's `DO`** (`:107-111`, `ee93cx6_read() ? 0x00 : 0x04`), so the idle bit is
  whatever its model idles at — the same question one level down. The page and the datasheet are still
  the whole of the evidence; keep High-Z-reads-as-1.
* **§2.3, the volatile RAM's size: OPEN.** puNES serves the `$6000` window from whatever the header
  declares and **disables it when the header declares none** (`:201-209`), mirroring by its memory
  map's own wrap. This build hard-codes 2 KiB mirrored. Both mirror; they differ on where the figure
  comes from. The page's "all known carts mount either no or a 2 KiB chip" supports either, and puNES
  is the only other data point.
* **§2.4, the 1 bpp substitution on the write path: CONFIRMED.** puNES applies its latch function on
  the **CHR write** path as well as the read path (`:126` and `:136`), and latches the two lines on the
  **rise of PPU A13** (`:220-228`) from PPU A0 and A9 — the page's two source lines and this build's
  `Pec9588::chr_address` on both paths.

## Q-171 — the Kǎshèng protection MMC3s (215, 208, 219)

Nestopia UE supports **208** (as `GOUDER_37017`) and **219** (as `UNL_A9746`) and not 215; FCEUX has
all three; Mesen 2 has all three; puNES has all three. **puNES is the decisive witness on five of the
six points, and it agrees with this build on every one of them.**

* **§1, 208's "wrong code" state: CONFIRMED — there is none.** puNES's protection is two lines
  (`src/core/mappers/mapper_208.c:106` latches the index, `:109` stores `value ^ table[index]`) with no
  detection and no latched failure state, which is the page's mechanism and this build's. §7.2's
  bullet describes behaviour no implementation has.
* **§2, mapper 215 submapper 1's `$5001`: CONFIRMED, and the ambiguity is settled.** puNES
  (`mapper_215.c:139-176`) composes, **for submapper 1 only**, PRG from `((reg[1] & 0x03) << 5) |
  ((reg[1] & 0x08) << 4)` — register bits **3, 1 and 0** as PRG A20/A19/A18 — and CHR from
  `((reg[1] & 0x0E) << 7)` — register bits **3, 2 and 1** as CHR A20/A19/A18. That is this build's
  reading of the two overlaid rows, to the bit, including the sharing of bits 3 and 1 between the two
  fields; and it keeps bits 4 and 5 as the two A17 lines (`:153`, `:175`). Mesen 2 implements only
  submapper 0's `..cp CCPP` layout for both submappers
  (`MMC3_215.h:55`, `:71`). **The ticket says "the page cannot distinguish them"; puNES does.**
* **§3, `$5001`'s power-up value: OPEN.** The page says `$xF`; **Mesen 2 and puNES both power it on at
  `$03`** (`MMC3_215.h:40`, `mapper_215.c:51`), i.e. the outer PRG bank at 3 and the outer CHR bank at
  0 where the page's low nibble sets both. Nothing can see it before the first write and every game
  writes `$5001` before it banks, so keep `$0F`; but two of the three implementations differ and the
  page's `$x` only hides the high nibble.
* **§4, 208's `$A000`: CONFIRMED, and puNES splits it by submapper exactly as this build does.**
  `mapper_208.c:138-147`: submapper 1 takes the MMC3's own mirroring; **submapper 0 takes the board's
  `$4800` register bit 5 and the MMC3's `$A000` drives nothing.** That is
  `Mmc3Board::force_ignore_a000` on submapper 0 and live on submapper 1, which is this build's reading
  of the two submappers' opening paragraphs.
* **§5, the protection index: CONFIRMED.** puNES latches the **written value** (`:106`), not the
  address — an 8-bit datum indexing a 256-entry table, which is this build's reading.
* **§6.1, 219's outer bank window: CONFIRMED.** puNES decodes it at **`$5000-$5FFF`** with A0 choosing
  which of the two bits (`mapper_219.c:60-67`) and has **nothing at `$D002`/`$D003`** — the page's
  printed `Mask: $5003` taken as this build takes it. (puNES decodes only A0 where this build decodes
  `$F003`; the point at issue — that `$D002` is the IRQ latch and not the outer bank — is identical.)
* **§6.2, 219's mode bit: CONFIRMED exactly.** `mapper_219.c:70-74`: the `$8000`-group write always
  reaches the chip, and `if (address & 0x0002) { m219.reg[1] = value & 0x20; }` latches the mode bit
  **only when A1 is set, i.e. only from `$8002`**, while the index comes from the chip's own
  `bank_to_update` on every even write. That is the derivation this ticket makes from the page's three
  statements, implemented. And the extended CHR latch is **persistent** in puNES too (`:84-95`
  accumulates into `mmc3.reg[index]` and nothing clears it), which is §6.2's last open reading.

## Q-172 — the last three singletons (262, 292, 246)

### Mapper 262 — the column readings are verified, the CHR-RAM banking is not

* **§1.1, the CHR-RAM select is bit 6: CONFIRMED three for three.** Mesen 2 `_exReg & 0x40`
  (`MMC3_StreetHeroes.h:44`), Nestopia `exReg & 0x40`
  (`NstBoardSachenStreetHeroes.cpp:137`, `:157`), puNES `m262.reg & 0x40` (`mapper_262.c:90`).
  **The paraphrase that put it at bit 7 is refuted everywhere** — which is the best possible vindication
  of this ticket's raw-wikitext method.
* **§1.2, the four CHR A18 lines are crossed: CONFIRMED by Mesen 2, identically.**
  `MMC3_StreetHeroes.h:48-51` gives `$0000-$07FF` register bit **3**, `$0800-$0FFF` bit **2**,
  `$1000-$17FF` bit **0** and `$1800-$1FFF` bit **1** — i.e. `[3, 3, 2, 2, 0, 0, 1, 1]` per 1 KiB
  window, which is `Mmc3Shero::CHR_A18_LINES` to the entry, **crossing included**. An independent
  reading of the same annotation columns.
* **§1.3, the CHR-RAM chip's own address lines: CHANGE.** All three serve the chip **unbanked**: Mesen
  2 maps one 8 KiB page (`:12-13`, `GetChrRamPageSize`/`GetChrRamSize` both `0x2000`, and `:45`
  `SelectChrPage(0, 0, ChrRam)`), Nestopia the same
  (`NstBoardSachenStreetHeroes.cpp:158`, `chr.Source(1).SwapBank<SIZE_8K,0x0000>(0)`), puNES maps each
  1 KiB window to **its own slot** (`mapper_262.c:91`, `memmap_vram_1k(…, slot)`), which is PPU
  A10-A12 straight through and so the same thing. This build banks it by the low three bits of the
  MMC3's bank number. See §CHANGE 7 — and note this is the alternative the ticket itself names and
  prices at three lines.
* **§1.4, the jumper: CONFIRMED, and the gap's shape again.** Mesen 2 keeps a `_resetSwitch`
  initialised to **0** and toggled by a soft reset (`:19`, `:34-38`); Nestopia answers a user dip
  switch (`NstBoardSachenStreetHeroes.cpp:166`, `cartSwitches.GetRegion()`); puNES answers
  `dipswitch.value` (`mapper_262.c:74`). So `JUMPER = $00` is Mesen 2's initial value and the page's
  first-listed title, and all three make the choice **user-facing** — the third board in this report
  whose unconfigurable constant is a host setting elsewhere (with mapper 83's solder pad and mapper
  12's language bit).
* **§1.5 and §1.6** (four-screen with an inert `$A000`, and the two guessed masks) are unobservable in
  the three: Mesen 2 and puNES decode `$4100` alone or `$4000-$4FFF` rather than the page's `$C100`,
  and none of them documents a four-screen decision. Nothing to add; this build's readings stand.

### Mapper 292 — §7.2 was right that no new hook is needed, for a reason it did not give

**This is the most actionable finding of the batch.** Mesen 2 and puNES both fill the board's two data
registers by **reading the game's zero page directly**, not from a bus latch:

* **Mesen 2** `Core/NES/Mappers/Unlicensed/DragonFighter.h:50-58`: on a read of `$6000-$7FFF` it takes
  `DebugRead(0x6A)` or `DebugRead(0xFF)` depending on the control register.
* **puNES** `src/core/mappers/mapper_292.c:86-98`: the same two addresses,
  `m292.reg[1] = ram_rd(nidx, 0x06A)` or `m292.reg[2] = ram_rd(nidx, 0x0FF)`.

Both are derived from the page's own sentence — "**The game writes the desired values to zero
page**" — and both avoid the latch entirely. So **§7.2's "no new hook" is correct after all**, by a
route §7.2 did not describe, and the +1.098 % that parked `board-292` is avoidable. See §CHANGE 8.

* **§2.1, `R` is bit 5: CONFIRMED.** Both select between the two registers on bits 7-5 of the control
  register (`(reg & 0xE0) == 0xC0` in puNES), so `R` is bit **5** and `A`/`B` are bits 7 and 6, as the
  annotation's columns say. One divergence worth recording: both projects **require** `A` and `B` to be
  set for the `$6A` register, where this build latches them and acts on neither — unobservable, since
  the page says the game keeps both set.
* **§2.2, "MMC3 registers 2/3 instead of register 0": OPEN.** Neither project implements it — both
  take `mmc3.reg[0] >> 1` and `mmc3.reg[1] >> 1` **unconditionally**, ignoring the chip's CHR
  inversion bit (Mesen 2 `:33-35`, puNES `:113-114`). The page states the behaviour and this build
  implements it for nothing (the chip's own per-window bank number already follows the bit); keep it,
  and note that the two implementations' silence means no game exercises it.
* **§2.3, the *Errata*'s "better than original hardware" suggestion**: neither project implements it
  either. Nothing to add.

### Mapper 246 — two of three agree with the page where this build does

* **§3.1, the register window: OPEN.** All three decode **`$6000-$67FF`** (Mesen 2
  `Core/NES/Mappers/Unlicensed/Mapper246.h`'s `RegisterStartAddress`/`EndAddress`; FCEUX
  `src/boards/246.cpp`'s `SetWriteHandler(0x6000, 0x67FF, …)`; puNES `mapper_246.c:69`), which is
  §7.2's decode and not the page's `$FFE4` mask. Three against this build's narrower window, with no
  reason given anywhere; the page's mask is explicit and requires A11-A5 clear. Keep it. The difference
  is a write in `$6020-$67FF`: a register there, the RAM chip here.
* **§3.1′, and the same mask on the read side: CONFIRMED.** **puNES forces PRG A17 under exactly this
  constant** — `mapper_246.c:82`, `(address > 0xFF00) && (address & 0xFFE4) == 0xFFE4`, then
  `(m246.prg[3] | 0x10) << 13` — which is `G0151::DECODE_MASK` serving both purposes, the very point
  the ticket makes.
* **§3.2, the 2 KiB window: CONFIRMED as a recorded cost, and two projects get it exact.** FCEUX
  (`src/boards/246.cpp`, read and write handlers on `$6800-$6FFF` only) and puNES
  (`mapper_246.c:109-112`, `memmap_disable_8k(0, MMCPU(0x6000))` then a 2 KiB window at `$6800`)
  serve the chip at `$6800-$6FFF` **and nowhere else**, which is the page. Mesen 2 mirrors it twice.
  So the ticket's diagnosis is right — "making it exact needs a per-address enable in that hook, which
  is a `Mapper` change and not a board's" — and two implementations show it is worth having. The cost
  stays recorded rather than paid.
* **§3.3, `$6003` is `$FF` at power-on only: CONFIRMED 2-1.** FCEUX sets the four PRG registers in
  `M246Power` and installs **no reset handler**; **puNES sets `m246.prg[3] = 0xFF` under
  `if (info.reset >= HARD)`** (`mapper_246.c:52-58`), so a soft reset leaves it — this build's reading
  exactly, and the page's "only". Mesen 2 restores it on every reset
  (`Mapper246.h`'s `Reset(bool softReset)`), against the page. So AMD-061 §5's list is right to gain
  this number.
* **§3.4** (the *See also* forum thread) needs no cross-check; nothing in the three reads a console
  timing for this board either.

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

### CHANGE 6 — mapper 12's language bit should read `$01`, not `$00`

* **Ticket**: Q-164 §3. **Priority**: second only to CHANGE 1 — it has a visible effect on the
  number's main title and the fix is one constant.
* **File / constant**: `rust/crates/zax-nes/src/cartridges/mappers/huang1.rs`,
  `Huang1::LANGUAGE_BIT` — change `$00` to `$01`. (Bits 7-1 stay zero; only bit 0 is the jumper.)
* **The evidence**: the page says the GAL16V8 has the value hard-wired "to Chinese text in all known
  copies" and does not say which value that is. **FCEUX initialises its read register to 1 with the
  comment "chinese is default"** (`third_party/fceux/src/boards/mmc3.cpp:394`) and **Nestopia answers a
  hard-coded `0x01`** (`third_party/nestopia/source/core/board/NstBoardRexSoftDb5z.cpp:83`). puNES
  answers a user dip switch, so it takes no position; Mesen 2 does not implement the read at all. Two
  independent projects, one of them labelling the value, against a constant this build picked for
  uniformity with its other narrow registers.
* **Titles affected**: *Dragon Ball Z 5* — under `$00` the game selects **English** text where every
  known cartridge shows Chinese. Nothing else on the number reads `$4132`.
* **Risk to existing boards**: none. The constant is read only by mapper 12's `try_read_prg` arm; no
  other number reaches `Huang1`'s read side (mapper 14's supervisor register is write-only). No
  save-state byte, no dispatch, no hot path.
* **Also worth doing in the same edit**: the test
  `req_core_102_mapper_12_answers_a_read_of_4132_with_the_language_bit` asserts `Some(0)` at `$4132`;
  it becomes `Some(1)`, and its assertion that the board drives nothing outside the `$E100` mask is
  unchanged. The comment naming the choice should record that FCEUX and Nestopia both answer 1 and
  that puNES makes it user-settable — which is the shape to adopt if a host surface for per-cartridge
  configuration ever exists (see the solder-pad note under Q-166 §2).

### CHANGE 7 — mapper 262's CHR-RAM chip is probably unbanked

* **Ticket**: Q-172 §1.3, whose own text names this as the alternative and prices it at three lines.
* **File / function**: `rust/crates/zax-nes/src/cartridges/mappers/mmc3_shero.rs`,
  `Mmc3Shero::chr_ram_offset` — replace "bits 0-2 of whatever 1 KiB bank number the chip put in that
  window" with the PPU address's own A10-A12, i.e. the window index, so the 8 Ki × 8 SRAM follows
  PPU A0-A12 straight through.
* **The evidence**: the page gives the chip a size and no banking, and **all three projects that
  support the number serve it unbanked** — Mesen 2 maps one 8 KiB page
  (`third_party/mesen2/Core/NES/Mappers/Mmc3Variants/MMC3_StreetHeroes.h:12-13`, `:45`), Nestopia the
  same (`third_party/nestopia/source/core/board/NstBoardSachenStreetHeroes.cpp:158`), and puNES maps
  each 1 KiB window to its own slot (`third_party/punes/src/core/mappers/mapper_262.c:91`), which is
  the same wiring written per-window.
* **Titles affected**: *Street Heroes* and *侍魂*, whenever `$4100` bit 6 selects the RAM. Under the
  current reading a game that writes a tile through one CHR bank number and reads it back under
  another sees a different kilobyte.
* **Risk**: none to other boards — `chr_ram_offset` is mapper 262's alone. The test in Q-172 §1.1's
  second half (banks `$11`/`$19` being one kilobyte and `$12` another) inverts: every bank becomes the
  same kilobyte for a given window, and what distinguishes the windows is the PPU address.

### CHANGE 8 — mapper 292 needs no bus latch, which would unpark `board-292`

* **Ticket**: Q-172 §2. **This is the highest-value change in the report after CHANGE 1**, because it
  removes a +1.098 % feature rather than fixing a byte.
* **Files / functions**: `rust/crates/zax-nes/src/cartridges/mappers/mmc3_dragonfighter.rs`
  (`Mmc3Dragonfighter::fill_data_register`), and the deletion of `CartridgeMemory::cpu_write_latch`
  together with the `if let` at the top of `NesBus::write_internal` that feeds it.
* **The change**: fill the two data registers from the console's **zero page** at `$6A` and `$FF`
  instead of from the last-CPU-write latch, choosing between them on the control register's bits 7-5
  exactly as now. Both other implementations do this: Mesen 2 reads `$6A`/`$FF` through its memory
  manager (`third_party/mesen2/Core/NES/Mappers/Unlicensed/DragonFighter.h:50-58`) and puNES reads
  them from RAM directly (`third_party/punes/src/core/mappers/mapper_292.c:86-98`). The page's own
  sentence is the justification — "**The game writes the desired values to zero page**" — so this is
  not a hack but the same observation read one step further on.
* **What it buys**: `Cartridge::cpu_write`'s path and `NesBus::write_internal` lose the per-write
  store that MF-UNL-12 step 6 measured at `oamstr` +0.026 % and `mmc5exram` +1.098 %. With the latch
  gone the board is a read hook and two registers, and `board-292` can plausibly leave
  `parked-boards`. **Measure it**: the ticket records that the latch store alone was +862 997 of
  +874 540 instructions on `oamstr`, so removing it should recover almost all of that feature's cost.
* **What it costs in accuracy**: generality. The latch is right for *any* `$6000-$7FFF` read; reading
  `$6A`/`$FF` is right only for a game that stages its values there, which is what the page says the
  one known game (*Dragon Fighter*) does. The honest shape is a comment naming the narrowing and the
  page sentence it rests on — and a note that if a second mapper-292 image appears that stages
  elsewhere, the latch is the general answer and its price is already measured.
* **Blocker**: the board needs to read console RAM, which `Mapper::cpu_read_override` is not handed
  today. That is a smaller hook than the write latch (a read of two fixed addresses on a path that
  already runs only for this board's window) but it is still a core change, so **R0 should scope it
  before a lot takes it**.

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
| Q-165 §3 | 14's supervisor power-on | `$02`, MMC3 mode | a *Samurai Spirits* trace showing whether it writes `$A131` before it banks. All four other implementations start in **VRC2 mode** — but every one of them gets that from zero-initialised memory rather than from a decision, and the page gives no value. Both modes reach a reset vector. |
| Q-165 §5a | 14's "mirroring only in VRC2 mode" | implemented | original hardware, as the page itself says. **Nestopia implements the rule in this build's exact shape** (MMC3-mode `$A000` re-applies the VRC2's `$9000` value and ignores the datum), which verifies the page's attribution; Mesen 2, FCEUX and puNES contradict it, which is what the page says they do. |
| Q-166 §1 | 83's scratch RAM decode | the `A~` row, `(a & $F30C) == $5100` | a dump using a mirror. Three projects, three decodes: Mesen 2 and FCEUX claim exactly `$5100-$5103`, puNES splits `$5000-$5FFF` by one address bit. All that is shared is that the four addresses must work, which this build satisfies. |
| Q-166 §3 | 83's `$8318` IRQ source | any non-zero selects A12 | a source for which data bit is decisive. **No project implements the register**; all three hard-wire the CPU-M2 source, i.e. the page's default value. This build is ahead of the field and unrebutted. |
| Q-170 §2.3 | 164's volatile RAM size | 2 KiB mirrored | an NES 2.0 dump of any of the six games with byte 10's volatile nibble filled in. puNES is the only other data point and it takes the figure from the header, disabling the window when none is declared; both mirror. |
| Q-171 §3 | 215's `$5001` power-up value | `$0F` | a corrected page, or a dump that depends on it. **Mesen 2 and puNES both power it on at `$03`** — outer PRG 3, outer CHR 0 — where the page's `$xF` sets both. Unobservable before the first write, and every game writes it before banking. |
| Q-172 §2.2 | 292's "registers 2/3" | the chip's own per-window bank | a dump that sets the chip's CHR inversion bit. **Neither Mesen 2 nor puNES implements the clause** — both take registers 0 and 1 unconditionally — which suggests no game exercises it. This build's reading costs nothing, so keep it. |
| Q-172 §3.1 | 246's register window | `$6000-$601F` (the page's `$FFE4`) | a mapper-246 write in `$6020-$67FF`. All three projects decode `$6000-$67FF`, which is §7.2's figure and not the page's mask; none gives a reason, and the page's mask requires A11-A5 clear. |
| Q-168 §3a | 91 submapper 0's ÷64 counter | a carry out that keeps running | a mapper-91 game that does not acknowledge. Mesen 2 repeats (an MMC3 latch of 7), FCEUX and puNES **stop at 8** — the ticket's named alternative — so it is two-two. All three count eight *filtered* scanline ticks rather than sixty-four unfiltered A12 rises, which differs only while rendering is off. |

---

## Totals, and the one thing a reader should take away

Thirty-two tickets, **147 verdict rows**:

| verdict | rows | the shape of them |
| --- | ---: | --- |
| **CONFIRMED** | ~110 | including a dozen points where a ticket expected to stand alone and puNES turned out to have reached the identical reading |
| **CHANGE** | 8 | CHANGE 1 (mapper 123's bank bit) and CHANGE 6 (mapper 12's language bit) have visible in-game effects; CHANGE 8 would unpark `board-292`; the rest are gaps or conditionals |
| **OPEN** | ~18 | every one with a named observation that would close it |

**The one methodological lesson.** The four projects are *not* four independent witnesses, and the
report says so at every point where it matters. For the unlicensed MMC3 clones in particular, Mesen
2's class, Nestopia's board and (less often) puNES's file are frequently FCEUX's expression character
for character — mapper 123's bank field, mappers 121/187's CHR A18, the Sachen mirroring table,
mapper 114's scrambled map. A 4-0 tally against this build means one thing when it is four codebases
and another when it is one 2005 file quoted four times, and the useful discriminators turned out to
be:

1. **whether the projects' games run.** Mapper 123 is the clearest case: four emulators play *Mortal
   Kombat 3*, and a wrong bit in a 16 KiB PRG field does not produce a playable game. That is the
   closest thing to hardware evidence available on this side, and it is why CHANGE 1 is a change.
2. **which project read the page rather than its predecessor.** For the J.Y. ASIC, mapper 195's GAL,
   mapper 114's one-to-one map, mapper 215 submapper 1's overlaid rows, mapper 219's mode bit and
   mapper 91's outer bank, **puNES is an independent reading of the same page** and it agrees with this
   build on details the older lineage gets differently. Where puNES and this build agree against three
   others, the three are usually one.
3. **what a project's own comment says.** FCEUX's "chinese is default", its "Deny any further writes
   when 7th bit is 1 AND 4th is 0", its "this could be done by bit logic, but this is
   self-documenting", Mesen 2's "This behavior only affects reads, not writes" with a forum link,
   puNES's note on the 1 MiB boards' A19/A20 wiring — each of those settled a ticket that the code
   alone would have left ambiguous.

A fourth observation, offered because three tickets independently ran into it: **this build has three
unconfigurable constants where every other project has a user setting** — mapper 12's language bit
(Q-164 §3), mapper 83's solder pad (Q-166 §2) and mapper 262's jumper (Q-172 §1.4). The pages say in
all three cases that the value is a wire or a pad with no header field. If a host surface for
per-cartridge configuration is ever built, those are its first three customers, and two of them
(mapper 83's pad, mapper 262's jumper) are documented to change what a game *displays*.
