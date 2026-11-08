# LEGACY-XCHECK — the pre-mapper-work boards, cross-checked against the hardware pages and four more emulators

Role R3, clean side. Lot LEGACY-XCHECK. **This lot changes nothing.** It adds no code, no test,
no spec file and no ticket; the only files it writes are this report and
`docs/progress/LEGACY-XCHECK.md`.

## What is under review

The boards this build carried **before** the mapper-review work — the set advertised by
`registry.rs` at tag `v0.4.2`:

```
0 1 2 3 4 5 7 9 10 11 13 15 16 18 21 22 23 25 27 28 30 32 34 41 64 66 71 87 101 156 225 228
```

32 iNES numbers over 26 board types. They were written from the project's own specification
(`spec/`, REQ-CORE-100 and the cartridge chapters), not from the hardware documentation, and
they had been compared with exactly one other emulator, once: `docs/review/MR-1-mappers.md`
(Mesen 2 only). MF-1 then fixed most of what MR-1 found.

What is reviewed here is **the current implementation at the branch tip**, in
`rust/crates/zax-nes/src/cartridges/`, against

1. the **nesdev wiki** page of the mapper and of its chip — registers, bit layouts, power-on
   state, mirroring, IRQ timing, bus conflicts, PRG-RAM enable/protect, submappers;
2. **FCEUX**, **Nestopia UE**, **puNES** and **Nintendulator's mapper plugins**, cited
   `file:line`;
3. **Mesen 2** as a fifth vote only, and only on points MR-1 did not already settle.

Verdicts:

- **OK** — matches the page and the majority of the other emulators;
- **DIFFERS** — this build differs from the page and/or the majority; the entry says exactly
  how, which games it can affect, whether a known game is affected, the risk a fix would carry
  for the games that must not move (SMB3, Zelda II, the widescreen packs), and what a fix would
  look like — no code;
- **OPEN** — the sources disagree or are silent; the entry says what evidence would settle it.

Paths: ours are relative to `rust/crates/zax-nes/src/cartridges/`; the others are relative to
`third_party/`.

## Licence note

FCEUX, Nestopia UE, puNES, Nintendulator's mapper plugins and Mesen 2 are GPL projects, present
in `third_party/` under the terms `third_party/README.md` records. They were **read, not
copied**. No line of their code is transcribed in this report or anywhere in this repository:
every behavioural statement below is a description in my own words, with a `file:line` citation
so the reader can check it at the source.

<!-- SUMMARY-TABLE -->

---

# Per board

## Mapper 0 — NROM

Page: *NROM* (<https://www.nesdev.org/wiki/NROM>). Ours: `mappers/nrom.rs`.
Theirs: `fceux/src/boards/datalatch.cpp` (`NROM_Init`), `nestopia/source/core/board/NstBoardNRom.hpp`,
`punes/src/core/mappers/mapper_000.c`, `nintendulator-mappers/src/iNES/mapper000.cpp`.

Nothing to bank, so the whole board is four points.

- **16 KiB PRG mirrored at $C000** — OK. `Nrom::try_read_prg` (`nrom.rs:34-41`) reduces the
  offset modulo the PRG-ROM length, so an NROM-128 image answers the same byte at $8000 and at
  $C000, which is what the page requires and what all four do by mapping the same 16 KiB page
  twice.
- **CHR is fixed, 8 KiB** — OK. `Nrom::ppu_peek`/`ppu_write` (`nrom.rs:53-59`) address CHR
  linearly with `address & 0x1FFF`.
- **A CHR-ROM board ignores PPU writes** — OK, and it is handled centrally rather than in the
  board: `CartridgeMemory::write_chr_direct` (`mod.rs:276-281`) writes only when there is no
  CHR-ROM and the CHR-RAM buffer is non-empty.
- **Nametable layout comes from the header and never moves** — OK; the board never calls
  `set_mirroring`.
- **Work RAM at $6000-$7FFF** — OK with a note. `Nrom` does not override
  `Mapper::prg_ram_enabled`, so it takes the default `true` (`mappers/mod.rs:391-393`), and
  REQ-CORE-012 hands an iNES 1.0 image 8 KiB of PRG-RAM. A real NROM cartridge has none, so
  $6000-$7FFF should be open bus; all five of the others answer RAM there too whenever their
  loader allocated any, and *Family Basic* is an NROM-class board that does have RAM. The
  difference is only observable by a program that reads $6000 expecting open bus, which no NROM
  game does.

**Test ROMs.** Most of the CPU and PPU suites in `assets-free/test-roms/nes-test-roms` are NROM
images (`instr_test-v5`, `cpu_timing_test6`, `blargg_ppu_tests_2005.09.15b`, `ppu_vbl_nmi`,
`sprite_hit_tests_2005.10.05`, …), so the board is exercised by nearly every ROM test the
project runs; `holy-mapperel/testroms` has three NROM configurations — `M0_P32K_C8K_V.nes`,
`M0_P32K_CR8K_V.nes` and `M0_P32K_CR32K_V.nes` — that probe the board directly.
`nrom368` exists in the suite but targets the NROM-368 homebrew board, which is not mapper 0 as
this build implements it and which no REQ asks for.

Count: 5 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 1 — MMC1 (SxROM)

Pages: *MMC1* (<https://www.nesdev.org/wiki/MMC1>), *SxROM* (<https://www.nesdev.org/wiki/SxROM>),
*NES 2.0 submappers*. Ours: `mappers/mmc1.rs`, plus `registry.rs:1054-1056` for the battery
slice. Theirs: `fceux/src/boards/mmc1.cpp`, `nestopia/source/core/board/NstBoardMmc1.cpp` and
`.hpp`, `nestopia/source/core/board/NstBoardSxRom.hpp`, `punes/src/core/mappers/MMC1.c` and
`mapper_001.c`, `nintendulator-mappers/src/Hardware/h_MMC1.cpp`,
`nintendulator-mappers/src/iNES/mapper001.cpp`, `nintendulator-mappers/src/UNIF/NES/NES_SxROM.cpp`,
`mesen2/Core/NES/Mappers/Nintendo/MMC1.h`.

MR-1 reviewed this board closely and MF-1 fixed five findings, so most of the register map is
already settled. What follows checks the register map again against the page and then spends
its time on the four things MR-1 never looked at: the SNROM `E` bit as the other four emulators
treat it, the SZROM PRG-RAM bit, the SOROM chip polarity, and NES 2.0 submapper 3.

### The register map — OK

- **$8000 control**: mirroring 0-3 = one-screen A, one-screen B, vertical, horizontal
  (`mmc1.rs:241-246`); PRG mode in bits 3-2 with modes 0 and 1 both meaning 32 KiB
  (`mmc1.rs:218-231`); CHR mode in bit 4 (`mmc1.rs:234-239`). Identical in all five:
  `mmc1.cpp:96-121`, `NstBoardMmc1.cpp:129-176`, `MMC1.c:158-188`, `h_MMC1.cpp:77-130`,
  `MMC1.h:59-89`.
- **32 KiB mode ignores the low bit of the PRG register** — OK: ours shifts the 16 KiB bank
  right by one (`mmc1.rs:220`), the others mask it off (`mmc1.cpp:107`, `h_MMC1.cpp:92-102`,
  `MMC1.h:155-172`). Same result.
- **8 KiB CHR mode ignores the low bit of CHR bank 0** — OK (`mmc1.rs:238`; `mmc1.cpp:72`,
  `NstBoardMmc1.cpp:156-164`, `MMC1.c:187`, `h_MMC1.cpp:118-130`).
- **Mode 2 fixes the *first* bank at $8000, mode 3 the *last* at $C000** — OK, and on a 512 KiB
  board both fixed banks stay inside the selected 256 KiB half (`mmc1.rs:222-230`), which is the
  page's "the 256 KiB outer PRG bank selection applies to all the PRG area, including the
  normally fixed bank". FCEUX adds the same offset to both windows (`mmc1.cpp:96-110`),
  Nestopia ORs it into both (`NstBoardMmc1.cpp:129-134`), Nintendulator passes it as the `OR`
  argument of both (`NES_SxROM.cpp:106-111`), puNES ORs it in `prg_swap_mmc1_001`
  (`mapper_001.c:59-66`).
- **The serial register takes bit 0, LSB first, and commits on the fifth write** — OK
  (`mmc1.rs:338-352`). All five agree.
- **A write with bit 7 set clears the shift register and ORs $0C into control** — OK
  (`mmc1.rs:326-332`), matching the page verbatim and all five.
- **The outer 256 KiB PRG bank and the PRG-RAM bank are taken from CHR bank 0** — OK. Ours
  always reads `chr0` (`mmc1.rs:206`, `mmc1.rs:310`). Nestopia (`NstBoardMmc1.cpp:129-150`),
  FCEUX (`mmc1.cpp:52-56`, `mmc1.cpp:77`), Nintendulator (`h_MMC1.cpp:118`, used by
  `NES_SxROM.cpp:106`) and puNES (`mapper_001.c:62`, `mapper_001.c:68`) do the same; only Mesen
  tracks the last-written CHR register (`MMC1.h:123`). MR-1-2 already settled that the hardware
  answer is neither — in 4 KiB CHR mode the lines follow PPU A12 — and that the question is
  unreachable because every SUROM/SXROM title runs in 8 KiB CHR mode. Four-to-one for ours.
- **NES 2.0 submapper 5 (SEROM/SHROM/SH1ROM) has fixed 32 KiB PRG** — OK (`mmc1.rs:215-216`);
  puNES does the same explicitly (`mapper_001.c:59-63`), Mesen too (`MMC1.h:153-156`).
- **The consecutive-write filter, and the reset bit escaping it** — OK as the page describes it,
  and ours is the strictest reading. Ours ignores any data write less than two CPU cycles after
  the previous write and lets a bit-7 reset through regardless (`mmc1.rs:319-336`); Mesen does
  exactly the same (`MMC1.h:203-211`). The other four arm the filter **only in a window after a
  reset write**: FCEUX compares against a timestamp it updates only in the reset branch
  (`mmc1.cpp:134`, `mmc1.cpp:141`), puNES ignores the next write only if a `reset` flag is set
  and the CPU reports a double write (`MMC1.c:49-56`), Nestopia opens a two-CPU-cycle window on
  the reset write and drops everything inside it including a second reset
  (`NstBoardMmc1.cpp:222-224`, `NstBoardMmc1.hpp` `Serial::RESET_CYCLES`), and Nintendulator has
  no cycle counter at all and instead clears the shift register whenever the register index
  changes, a hack it labels as such (`h_MMC1.cpp:67-70`). Ours is a superset of all of them on
  the only sequences hardware can produce, and it is the one the page's "the bit 7 reset is never
  ignored" asks for. The three games the others name — *AD&D Hillsfar*, *Bill & Ted's Excellent
  Video Game Adventure*, *Snow Brothers* — are covered by ours as well as by theirs.
- **Four-screen header** — OK. puNES refuses to apply MMC1 mirroring when the header says
  four-screen (`mapper_001.c:80-84`); ours always applies it. No MMC1 board has four-screen VRAM,
  so there is nothing for the guard to protect.

### MMC1-1 — the SNROM `E` bit disables PRG-RAM here and in no other emulator — DIFFERS

**Point.** On an SNROM board, bit 4 of the CHR bank registers is a second chip enable for the
PRG-RAM: the page gives it as "PRG RAM disable (0: enable, 1: open bus)" and says the board
"uses the upper CHR bank select line coming out of the mapper (CHR A16) as an additional chip
enable for the PRG-RAM".

**This build.** Implements it. `Mmc1::new` (`mmc1.rs:86`) decides from a three-way size test
(`Mmc1::is_snrom`, `mmc1.rs:124-129`: PRG-ROM ≤ 256 KiB, declared CHR-ROM ≤ 8 KiB, PRG-RAM
exactly 8 KiB) whether the board is an SNROM, and if so folds bit 4 of `chr0` into the PRG-RAM
disable beside bit 4 of the PRG register (`Mmc1::refresh_prg_ram_disable`, `mmc1.rs:196-199`).

**The others.** None of the five implements it:

| | PRG-RAM disable condition | file:line |
| --- | --- | --- |
| FCEUX | `$E000` bit 4 only | `mmc1.cpp:40-49` |
| Nestopia | `$E000` bit 4 only, and only when the revision is not MMC1A | `NstBoardMmc1.cpp:138-146` |
| puNES | `$E000` bit 4 only | `MMC1.c:152-154` |
| Nintendulator | `$E000` bit 4 only | `h_MMC1.cpp:132-136` |
| Mesen 2 | `$E000` bit 4 only | `MMC1.h:85-87`, `MMC1.h:131` |

puNES is the interesting one: it *has* the `E`-bit expression, written out in full with the
page's own bit diagram quoted above it, and it is **commented out** with a named regression —
the maintainer's note says that with the check live, *Witch n' Wiz* shows dirty screens after
the save-slot selection (`MMC1.c:145-154`). The same comment records the community's own view
that "emulators tend not to implement it" because the bit was not confirmed until 2010.

**The test ROM.** `assets-free/test-roms/holy-mapperel` takes the page's side. Its README
reserves error digit `4xxx` for exactly this question: "$A000 bit 4 does not disable WRAM
(SNROM), or $A000 bit 4 disables WRAM (all but SNROM)" (`holy-mapperel/README.md:201-203`). So
the hardware test expects the `E` bit to work on an SNROM and expects it *not* to work anywhere
else — which is what this build implements and what none of the five do.

But the suite does not actually exercise the SNROM side. Its twelve `M1_*` images are
`M1_P128K`, `M1_P128K_C128K[_S8K|_W8K]`, `M1_P128K_C32K[_S8K|_W8K]`, `M1_P128K_CR8K`,
`M1_P512K[_CR8K]_S8K` and `M1_P512K[_CR8K]_S32K`: the CHR-RAM configurations carry no work RAM,
the ones that carry work RAM carry CHR-ROM, and the only CHR-RAM-plus-RAM images are 512 KiB
(SUROM/SXROM). None of them is a 128/256 KiB board with 8 KiB of CHR-RAM *and* 8 KiB of PRG-RAM,
which is the SNROM shape. So the suite would confirm that this build correctly withholds the
`E` bit from every non-SNROM configuration — `Mmc1::is_snrom` returns false for all twelve — and
would say nothing about the path it does take.

**Verdict: DIFFERS** — this build follows the page and the hardware test ROM's stated
expectation; all five emulators follow a convention that deliberately ignores it, and one of
them reverted the page's behaviour because of a game. This is not a regression introduced by the
mapper work: MR-1 recorded it as a deliberate choice under AMD-061 ("the public documentation
prevails"). What MR-1 did not know is that four more emulators vote the same way as Mesen, and
that there is a named casualty.

**Which games it can affect.** Every mapper-1 image with 8 KiB of PRG-RAM, no CHR-ROM and
≤ 256 KiB of PRG — i.e. every SNROM and, because an iNES 1.0 dump cannot be told apart, every
SGROM dump too. That is a large and very popular set: *The Legend of Zelda*, *Zelda II*,
*Metroid*, *Final Fantasy*, *Mega Man 2*, *Castlevania II*, *Teenage Mutant Ninja Turtles*. The
bit only bites when a game writes $A000 with bit 4 set, which on an 8 KiB CHR-RAM board is a
meaningless CHR bank number; a game that does so loses its PRG-RAM — saves and, if it uses
$6000-$7FFF as scratch, its working memory.

**Known affected game: yes.** *Witch n' Wiz* (aftermarket homebrew, mapper 1) is named by
puNES's own source as breaking under exactly this check. I cannot run it here, so I cannot say
whether this build reproduces the dirty screens, only that it implements the condition puNES
removed to fix them.

**Risk of a fix.** Low for the games that must not move. SMB3 and the widescreen packs are not
mapper 1. Zelda II is mapper 1 and is in the protected set: it would be *affected by the present
code*, not by the fix — removing the `E` bit can only ever re-enable PRG-RAM that is currently
disabled, so a game that works today keeps working, and a game that was losing its save starts
keeping it. The one way a fix could move a working game is if some game relies on PRG-RAM
reading open bus, which no source suggests.

**What a fix would look like.** The weak part is not the bit, which the page and the test ROM
both want; it is the **heuristic that decides the board is an SNROM**, which fires on every
iNES 1.0 mapper-1 dump with CHR-RAM and the default 8 KiB of PRG-RAM — i.e. on the whole popular
set above, SGROM dumps included. Three options, in increasing cost: (a) narrow the heuristic, so
the `E` bit applies only when the header gives positive evidence of an SNROM (NES 2.0 with a
declared 8 KiB CHR-RAM *and* a declared 8 KiB PRG-RAM), leaving iNES 1.0 dumps on the
five-emulator convention; (b) drop the `E` bit entirely, which is the five-emulator consensus
and makes `is_snrom`, `chr_disable_mask` and the two-register form of
`refresh_prg_ram_disable` dead; (c) keep it and add a per-ROM exception list, which REQ-CORE-100
does not ask for. (a) is the one to prefer and (b) is free; both are a change to `Mmc1::new`
alone, and `refresh_prg_ram_disable` stays one expression either way.

### MMC1-2 — SZROM takes the PRG-RAM bank from bit 4, not bit 3 — DIFFERS

**Point.** *SxROM* gives SZROM 128/256 KiB PRG, **16 KiB PRG-RAM** and 16-64 KiB of CHR-**ROM**,
and *MMC1* gives that board bit 4 of the CHR register as its 8 KiB PRG-RAM bank select — a
different bit from SOROM's, which is bit 3. The two boards are told apart by the CHR: SOROM has
8 KiB of CHR-RAM, SZROM has CHR-ROM.

**This build.** `Mmc1::new` (`mmc1.rs:62-67`) chooses the shift from the PRG-RAM size alone: two
8 KiB windows → shift 3, mask 1. So an SZROM dump is banked as if it were a SOROM, one bit off.

**The others.** puNES is the only one that distinguishes the boards, and it does it exactly as
the page does — on the presence of CHR-ROM (`mapper_001.c:68-76`: for a 16 KiB PRG-RAM board it
takes bit 4 when there is CHR-ROM and bit 3 when there is not). FCEUX uses bit 3 for any 16 KiB
board (`mmc1.cpp:52-57`). Nestopia uses bit 3 for any 16 KiB board (`NstBoardMmc1.cpp:147-150`:
the shift is `2 + (size == 16K)`). Nintendulator's iNES path keys on the NES 2.0 PRG-RAM byte
and has a SOROM case and an SXROM case but no SZROM case, so an SZROM dump gets bank 0 and never
banks at all (`iNES/mapper001.cpp:20-40`). Mesen's 16 KiB branch takes bit 3 only when the split
is exactly 8 KiB save + 8 KiB work and otherwise falls into a branch its own comment calls
"Unknown, shouldn't happen", which uses bit 2 (`MMC1.h:136-143`).

**Verdict: DIFFERS** — the page and puNES say bit 4; this build, FCEUX and Nestopia say bit 3;
Nintendulator does not bank and Mesen admits it does not know. The page is unambiguous and has
one implementer, so this build is wrong, but it is wrong in the company of two of the four.

**Which games it can affect.** Mapper-1 images with 16 KiB of PRG-RAM **and** CHR-ROM. *SxROM*
says "the only currently-known game on SZROM is *A Ressha de Ikou*", a Japanese train
simulator. Nothing in the widescreen packs and nothing popular.

**Known affected game: yes, one** — *A Ressha de Ikou*, if a dump of it reaches this build with
a 16 KiB PRG-RAM declaration (which needs NES 2.0 byte 10; an iNES 1.0 dump gets 8 KiB, falls
into the `0 | 1` arm of `mmc1.rs:64` and does not bank at all, which is what Nintendulator also
does).

**Risk of a fix.** None for the protected games: the condition would be "16 KiB PRG-RAM and
CHR-ROM present", which no SNROM, SOROM, SUROM or SXROM title satisfies. SMB3, Zelda II and the
widescreen packs cannot reach it.

**What a fix would look like.** One more arm in the `match` of `mmc1.rs:62-67`, keyed on
`header.chr_rom_size != 0` beside the `2` case, giving `(4, 0x01)`. It is decided at load, so
the hot path keeps its single shift and mask.

### MMC1-3 — which SOROM chip the `S` bit selects, and which one has the battery — OPEN

**Point.** *MMC1* says only that on SOROM and SZROM "the battery is connected to only one
PRG-RAM chip. The first RAM chip will not retain its data, but the second one will." It does not
say whether `S` = 0 selects the first chip or the second.

**This build.** `prg_ram_window` (`mmc1.rs:309-312`) maps bank `S` at offset `S × 8 KiB`, and
`registry::battery_offset` (`registry.rs:1054-1056`) makes the battery slice the **second** 8
KiB. So `S` = 1 reaches the battery-backed chip.

**The others — a dead split.**

| | `S` = 0 selects | battery slice | file:line |
| --- | --- | --- | --- |
| this build | bank 0, not battery-backed | second 8 KiB | `mmc1.rs:310`, `registry.rs:1056` |
| FCEUX | bank 0, not battery-backed | second 8 KiB | `mmc1.cpp:56`, `mmc1.cpp:299`, `mmc1.cpp:309` |
| Nestopia | bank 0, not battery-backed | second 8 KiB | `NstBoardMmc1.cpp:149`, `NstBoardMmc1.cpp:115-127` |
| puNES | bank 1 (the selector is inverted) | — | `mapper_001.c:72` |
| Nintendulator | bank 1, "bank 1 is batteried while bank 0 is not, so we need to switch it around" | — | `iNES/mapper001.cpp:24-27`, `NES_SxROM.cpp:99` |
| Mesen 2 | the save-RAM chip | — | `MMC1.h:139` |

Three say `S` = 0 is the battery chip and three (including this build) say it is the volatile
one. The page supports neither.

**Verdict: OPEN.** The consequence if this build is on the wrong side is narrow but real: a
SOROM game writes its save into the half that is not persisted, so the save is lost on a cold
boot while appearing to work within a session. The affected titles are the SOROM ones FCEUX
names by CRC — *Romance of the Three Kingdoms*, *Genghis Khan*, *Nobunaga's Ambition*
(`mmc1.cpp:185-196`) — all Japanese strategy games, none in the widescreen packs.

**What would settle it.** Not `holy-mapperel`: its `M1_*` set has no 16 KiB-PRG-RAM
configuration at all (the sizes it ships are 8 KiB and 32 KiB), so the SOROM selector is outside
the suite. What would settle it is a save-and-cold-boot cycle of *Nobunaga's Ambition* on
hardware — write a save, power-cycle, and see which `S` value reads it back. Failing hardware, a
comparison of real save files produced by the two camps would at least establish which
convention the existing save ecosystem uses, which for a save file is the thing that actually
matters.

### MMC1-4 — NES 2.0 submapper 3 (MMC1A) is not handled — DIFFERS, cosmetic

**Point.** *NES 2.0 submappers* gives mapper 1 submapper 3 as the MMC1A, whose bit 4 of the PRG
register is not a PRG-RAM disable at all: PRG-RAM is always enabled and the bit drives PRG A17.

**This build.** `Mmc1::new` reads the submapper only to test for 5 (`mmc1.rs:82`). A
submapper-3 dump is run as an MMC1B: its PRG-RAM is disabled whenever the game sets bit 4, and
A17 is not driven.

**The others.** puNES maps submapper 3 to its `MMC1A` type (`mapper_001.c:46`) and then both
suppresses the PRG-RAM disable (`MMC1.c:152-154`) and implements the A17 behaviour
(`MMC1.c:182-184`). Nintendulator has the same two behaviours behind an `IsMMC1A` flag
(`h_MMC1.cpp:100-101`, `h_MMC1.cpp:132-136`, `h_MMC1.cpp:157-166`) but sets it from the board
name, not the submapper. Nestopia has the revision as `REV_A` and suppresses the disable
(`NstBoardMmc1.cpp:142`, `NstBoardSxRom.hpp:40`), chosen from the NES 2.0 chip list rather than
the submapper. Mesen keys `_forceWramOn` off its game database's board string
(`MMC1.h:194`). FCEUX exposes it only as mapper 155 (`mmc1.cpp:333-336`).

**Verdict: DIFFERS**, and the smallest kind. MR-1 already recorded the MMC1A/MMC1B difference as
"equivalent for unlisted ROMs" because distinguishing them needs a game database; what is new
here is that **the submapper is a header field, not a database lookup**, and three of the four
emulators do implement the distinction one way or another. This build already has the machinery:
`prg_disable_mask` exists exactly so that mapper 155 can zero it (`mmc1.rs:72`), and submapper 3
is the same board under a different number.

**Which games it can affect.** Any dump headered as mapper 1 submapper 3. No commercial game is
known to need it; the submapper exists for correctly-headered dumps of early MMC1A boards. No
protected game is mapper 1 submapper 3.

**Risk of a fix.** None. The condition is a header test at load; `wired` at `mmc1.rs:72` would
gain `|| header.submapper == 3` and nothing on any hot path changes.

### MMC1-5 — the power-on value of the control register, and of the PRG-RAM disable — OPEN

**Point.** The page says only that "all versions seem to reliably power on in the last bank" —
i.e. PRG mode 3 — and flags even that as fragile. It says nothing about mirroring, CHR mode or
the PRG-RAM disable at power-on.

**This build.** `control = 0x0C` (`mmc1.rs:77`, `mmc1.rs:297`): PRG mode 3, CHR 8 KiB mode,
mirroring one-screen A. `prg = 0`, so PRG-RAM is **enabled** at power-on.

**The others.** Four different answers for the control register: puNES `0x0C` (`MMC1.c:95`),
Nintendulator `0x0C` (`h_MMC1.cpp:26`), Nestopia `0x0F` — mode 3 plus horizontal mirroring
(`NstBoardMmc1.hpp` `CTRL_HARD_RESET`, used at `NstBoardMmc1.cpp:53`), FCEUX `0x1F` — mode 3
plus 4 KiB CHR mode plus horizontal mirroring (`mmc1.cpp:172`), Mesen a power-on byte ORed with
`0x0C`, i.e. deliberately not deterministic (`MMC1.h:188`). For the PRG-RAM disable, Nestopia
powers an MMC1B3 up with it **set** (`NstBoardMmc1.cpp:56`) and Mesen does the same when its
database calls the board MMC1B (`MMC1.h:191`); the other three, and this build, power up
enabled.

**Verdict: OPEN**, and benign. Every MMC1 game writes all four registers in its reset code
before it needs them — that is why the five implementations can disagree this widely without
anyone noticing. The one shape of bug it could hide is a game that reads PRG-RAM before writing
`$E000`, which would see RAM here and open bus on Mesen and Nestopia.

**What would settle it.** A power-on read of the PRG window on hardware, which is what the
page's own hedge ("this is fragile") is about. `holy-mapperel`'s MMC1 configurations probe the
`$A000` bit-4 wiring but not the power-on register values.

### Re-checked, and MR-1's conclusions stand

- **512 KiB test** (`> 256 KiB` here, `== 512 KiB` in Mesen). puNES applies the outer bit with
  no size test at all and lets the PRG mask absorb it (`mapper_001.c:62`); Nintendulator tests
  `PRGSize == 32` 16 KiB units, i.e. exactly 512 KiB (`iNES/mapper001.cpp:13`); Nestopia ORs it
  in unconditionally (`NstBoardMmc1.cpp:129`). All equivalent — no 384 KiB MMC1 dump exists.
- **SXROM uses bits 3-2** — confirmed four ways: ours (`mmc1.rs:66`), Nestopia
  (`NstBoardMmc1.cpp:149`), Nintendulator (`iNES/mapper001.cpp:30`, `NES_SxROM.cpp:119`),
  Mesen (`MMC1.h:135`). puNES uses `(bank & 0x0C) >> 3`, which is bits 3-2 shifted one too far
  (`mapper_001.c:74`) — puNES looks wrong here, and it is the only one that is.
- **SOROM's battery is half the RAM** — confirmed: FCEUX saves 8 KiB of the 16
  (`mmc1.cpp:299`, `mmc1.cpp:309`), Nestopia saves from offset 8 KiB (`NstBoardMmc1.cpp:117`),
  Mesen splits save-RAM from work-RAM (`MMC1.h:139`). MF-1's fix is right; only the polarity
  (MMC1-3) is open.

**Test ROMs.** `assets-free/test-roms/nes-test-roms/MMC1_A12` (the A12-driven CHR/PRG
interaction MR-1-2 describes), and the twelve `M1_*` images of
`assets-free/test-roms/holy-mapperel/testroms`, whose driver probes the PRG window, the CHR
window, the WRAM window and the `$A000` bit-4 question of MMC1-1. Neither is run by the
project's test suite today: the ROM tests this build runs are the CPU/PPU/APU suites, and no
mapper-conformance ROM is in the harness. Running the `M1_*` set would check the whole register
map at first hand and would confirm the non-SNROM half of MMC1-1; the SNROM half, MMC1-2 and
MMC1-3 are not covered by any ROM in `assets-free`.

Count: 11 OK, 3 DIFFERS, 2 OPEN.

---

## Shared: the bus conflict, re-counted

MR-BUS-1 was MR-1's single most widely shared finding and MF-2 implemented it;
`registry::has_bus_conflicts` (`registry.rs:1099-1160`) now decides it once at load, from the
page's own rows and submapper clauses. Because it is shared by eight of the boards below, the
four new votes are counted here once.

| board | page | this build | FCEUX | Nestopia | puNES | Nintendulator | Mesen 2 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2 UxROM | submapper 2 | submapper 2 | submapper 2 (`datalatch.cpp:180`) | always, on a database-identified UNROM/UOROM board (`NstBoardUxRom.cpp:41`) | submapper 2 (`mapper_002.c:43-46`) | never (`mapper002.cpp:22`, `h_Latch.cpp:15-20`) | submapper 2 |
| 3 CNROM | submapper 2 | submapper 2 | submapper 2 (`datalatch.cpp:192`) | always, on a database-identified CNROM (`NstBoardCxRom.cpp:71`) | submapper 2 (`mapper_003.c:64-67`) | never (`mapper003.cpp:18`) | submapper 2 |
| 7 AxROM | submapper 2 ("submapper 0: emulators *shouldn't* emulate bus conflicts") | submapper 2 | never (`datalatch.cpp:204`) | on a database-identified AMROM/AOROM (`NstBoardAxRom.cpp:37`, `:51-54`) | submapper 2 (`mapper_007.c:45-48`) | never (`mapper007.cpp:24`) | submapper 2 |
| 11 Color Dreams | always | always | never (`datalatch.cpp:227`) | — | — | never (`mapper011.cpp:18`) | always |
| 13 CPROM | — | never | never (`datalatch.cpp:243`) | always (`NstBoardCxRom.cpp:81`) | — | never (`mapper013.cpp:19`) | never |
| 66 GxROM | **"Yes"**, unconditional | always | never (`datalatch.cpp:279`) | always (`NstBoardGxRom.cpp:45`) | never (`mapper_066.c:44-48`) | never (`mapper066.cpp:18`) | never |
| 30 UNROM 512 | submapper 2, or submapper 0 without battery | the same | — | battery means none (`NstBoardUxRom.cpp:74`) | — | — | the same |
| 34 BNROM | BNROM yes, NINA no | the same | — | — | — | — | the same |

Two conclusions, both reassuring and one new:

- **The submapper-gated boards (2, 3, 7) are a four-way agreement.** The page, this build, FCEUX,
  puNES and Mesen all gate on submapper 2 and all leave submapper 0 alone, which is what the
  AxROM page asks for in so many words. Nestopia gates on its own board database instead, which
  is the same answer by a different route; Nintendulator compiles the machinery
  (`h_Latch.cpp:88-99`, behind `BUS_CONFLICTS`) and then passes `FALSE` for every one of these
  boards.
- **Mapper 66 is the one where this build is in a minority of two.** The *GxROM* page's
  bus-conflict row is a flat "Yes" with no submapper clause, and only Nestopia implements it
  (`NstBoardGxRom.cpp:45`, `GetBusData`); FCEUX, puNES, Nintendulator and Mesen all latch the
  written value. MR-66-1 already recorded Mesen's omission; the new information is that three
  more emulators omit it too. This build follows the page, so the verdict stays **OK** — see
  mapper 66 below for what it would take to be wrong.
- **CPROM is the mirror image**: Nestopia gives it conflicts and the page does not mention them,
  so this build's "none" is the majority and the page's silence.

---

## Mapper 2 — UxROM

Page: *UxROM* (<https://www.nesdev.org/wiki/UxROM>). Ours: `mappers/discrete.rs:8-98`.
Theirs: `fceux/src/boards/datalatch.cpp:164-181`,
`nestopia/source/core/board/NstBoardUxRom.cpp:34-80`, `punes/src/core/mappers/mapper_002.c`,
`nintendulator-mappers/src/iNES/mapper002.cpp`.

- **A 16 KiB window at $8000 from the latch, the last 16 KiB fixed at $C000** — OK
  (`discrete.rs:58-61`). Identical in all four (`datalatch.cpp:174-176`,
  `NstBoardUxRom.cpp:62` via Nestopia's shared `PRG_SWAP_16K_0` helper, `mapper_002.c:55-58`,
  `mapper002.cpp:12-13`).
- **The whole of $8000-$FFFF decodes** — OK (`discrete.rs:89`); same in all four.
- **The latch is taken whole, and the unused high bits are absorbed by the PRG size** — OK.
  The page's field is `.... PPPP`; ours stores all eight bits and lets `map_prg_16k` reduce
  (`discrete.rs:93`), as do FCEUX (`datalatch.cpp:174`) and puNES (`mapper_002.c:56`).
  Nintendulator stores all eight too (`mapper002.cpp:8`). Equivalent for every image: a UNROM
  is 128 KiB and a UOROM 256 KiB, so bits 7-4 cannot address anything the mask does not already
  fold away.
- **CHR is fixed and not switched** — OK (`discrete.rs:63`), all four the same.
- **No CHR-ROM: the board carries 8 KiB of CHR-RAM** — OK; `map_chr_8k` serves whichever of the
  two exists, which is Nintendulator's explicit `if (ROM->INES_CHRSize)` test
  (`mapper002.cpp:15-17`).
- **Nametable layout is the header's and never moves** — OK. FCEUX carries a commented-out
  mapper-2 variant that would drive mirroring from latch bit 3 for a "Highway Star" hack
  (`datalatch.cpp:167-173`), disabled "till new mapper defined"; Nestopia has a similar extended
  decode but only for its UNROM 512 board (`NstBoardUxRom.cpp:72-80`). Neither is mapper 2.
- **Bus conflicts** — OK, submapper-gated; see the shared table.
- **Power-on latch value 0** — OK (`discrete.rs:84`): FCEUX `latcheinit = 0`
  (`datalatch.cpp:180`), Nintendulator `Data = 0` (`h_Latch.cpp:26`), puNES zeroes the struct
  (`mapper_002.c:35-37`). All four agree, which for this board matters: the first bank is what
  the game sees before its reset code runs.

**Test ROMs.** `holy-mapperel/testroms/M2_P128K_V.nes` and `M2_P128K_CR8K_V.nes`. Not run by the
project today.

Count: 8 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 3 — CNROM

Page: *CNROM* (<https://www.nesdev.org/wiki/CNROM>). Ours: `mappers/discrete.rs:100-148`.
Theirs: `fceux/src/boards/datalatch.cpp:184-193`,
`nestopia/source/core/board/NstBoardCxRom.cpp:62-105`, `punes/src/core/mappers/mapper_003.c`,
`nintendulator-mappers/src/iNES/mapper003.cpp`.

- **The latch selects the 8 KiB CHR bank; PRG is fixed 32 KiB** — OK
  (`discrete.rs:115-118`), all four the same (`datalatch.cpp:185-189`,
  `NstBoardCxRom.cpp:98`, `mapper_003.c:89-94`, `mapper003.cpp:12-13`).
- **The whole of $8000-$FFFF decodes** — OK (`discrete.rs:143`), all four the same.
- **Only two CHR address lines are wired; the rest of the latch is don't-care** — OK by
  masking. The page's field is two bits; ours stores all eight and `map_chr_8k` reduces modulo
  the CHR size (`discrete.rs:144`), exactly as FCEUX (`datalatch.cpp:186`), puNES
  (`mapper_003.c:93`) and Nintendulator (`mapper003.cpp:8`) do. A CNROM is 32 KiB of CHR at
  most, so the readings cannot diverge.
- **Work RAM at $6000-$7FFF** — OK. FCEUX allocates it for this board by name, with *Hayauchy
  IGO* as the reason (`datalatch.cpp:187`, `datalatch.cpp:192`), and puNES maps it
  unconditionally (`mapper_003.c:95-97`); ours comes from REQ-CORE-012's default and the
  trait's default `prg_ram_enabled`, so the same window answers.
- **CHR disable / open bus** — OK, and correctly *not* here. Nestopia's CNROM class carries a
  chip-enable path that answers $FF from the CHR bus when the latch matches a wiring read from
  its NES 2.0 pin list (`NstBoardCxRom.cpp:34-57`, `NstBoardCxRom.cpp:87-104`). That is the
  CNROM-with-CHR-disable board — mapper 185, which this build implements separately
  (`mappers/cnrom_protect.rs`) and which is not mapper 3. Plain mapper 3 takes Nestopia's
  non-chip-enable path.
- **Bus conflicts** — OK, submapper-gated; see the shared table.
- **Power-on latch 0** — OK (`discrete.rs:138`), all four the same.

**Test ROMs.** `holy-mapperel/testroms/M3_P32K_C32K_H.nes`. Not run by the project today.

Count: 7 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 7 — AxROM

Page: *AxROM* (<https://www.nesdev.org/wiki/AxROM>). Ours: `mappers/discrete.rs:150-203`.
Theirs: `fceux/src/boards/datalatch.cpp:195-205`,
`nestopia/source/core/board/NstBoardAxRom.cpp:34-55`, `punes/src/core/mappers/mapper_007.c`,
`nintendulator-mappers/src/iNES/mapper007.cpp`.

- **One register: 32 KiB PRG bank, plus a one-screen nametable select in bit 4** — OK
  (`discrete.rs:165-173`). All four the same, and all four read bit 4 the same way round —
  clear = screen A, set = screen B (`datalatch.cpp:199`, `NstBoardAxRom.cpp:48`,
  `mapper_007.c:61-66`, `mapper007.cpp:17-19`).
- **CHR is fixed** — OK (`discrete.rs:167`).
- **$8000-$FFFF decodes** — OK (`discrete.rs:198`), and here FCEUX is the outlier: it maps the
  latch over **$4020-$FFFF** (`datalatch.cpp:204`), i.e. it accepts writes from the whole
  expansion and I/O area. The page gives the range as $8000-$FFFF and the other three agree
  with this build (`NstBoardAxRom.cpp:37-39`, `mapper_007.c:44`, `h_Latch.cpp:29-30`). One
  against four; ours matches the page.
- **The width of the PRG field** — OK, with the page's field narrower than ours. The page prints
  `xxxM xPPP`: **three** bank bits, bit 3 explicitly unused. Ours takes four
  (`discrete.rs:166`), as does FCEUX (`datalatch.cpp:198`); Nestopia and puNES take the whole
  byte and let the size mask reduce it (`NstBoardAxRom.cpp:47`, `mapper_007.c:59`);
  Nintendulator takes exactly the page's three (`mapper007.cpp:12`). Equivalent on every real
  image — AOROM tops out at 256 KiB, eight 32 KiB banks, so bit 3 addresses nothing — and the
  readings only part on an oversize homebrew image of 512 KiB, where this build and FCEUX bank
  all sixteen and Nintendulator wraps. Since no such image is in REQ-CORE-100's scope, **OK**.
- **Bus conflicts, and the page's instruction for submapper 0** — OK. The page is unusually
  explicit: "Submapper 0: Bus conflict behavior undefined, but emulators *shouldn't* emulate bus
  conflicts". This build gates on submapper 2 (`registry.rs:1106`), which is that instruction
  followed to the letter, and puNES and Mesen do the same.
- **Power-on: bank 0, screen A** — OK (`discrete.rs:192-195`). The page says nothing; all four
  power the latch up at zero, which gives the same result.

**Test ROMs.** `holy-mapperel/testroms/M7_P128K.nes` and `M7_P128K_CR8K.nes`. Not run by the
project today.

Count: 7 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 66 — GxROM

Page: *GxROM* (<https://www.nesdev.org/wiki/GxROM>). Ours: `mappers/discrete.rs:309-358`.
Theirs: `fceux/src/boards/datalatch.cpp:271-280`,
`nestopia/source/core/board/NstBoardGxRom.cpp:34-48`, `punes/src/core/mappers/mapper_066.c`,
`nintendulator-mappers/src/iNES/mapper066.cpp`.

- **The register is `xxPP xxCC`: bits 5-4 the 32 KiB PRG bank, bits 1-0 the 8 KiB CHR bank** —
  OK, and this build is the only one that reads the page's fields exactly
  (`discrete.rs:324-328`). FCEUX takes bits 7-4 and 3-0 (`datalatch.cpp:274-275`), Nintendulator
  the same (`mapper066.cpp:12-13`), puNES the same (`mapper_066.c:54-59`), Nestopia the whole byte
  for CHR and the top nibble for PRG (`NstBoardGxRom.cpp:46-47`). Equivalent on every real
  image: GNROM is 128 KiB PRG / 32 KiB CHR and MHROM 64 / 32, four banks each way, so bits 7-6
  and 3-2 address nothing that the size mask does not fold. The four-bit readings would only
  part from ours on an oversize image, where the page's own diagram is on this build's side.
- **$8000-$FFFF decodes** — OK (`discrete.rs:353`), all four the same.
- **Bus conflicts: the page says "Yes", unconditionally** — OK, and this build says yes. See the
  shared table: only Nestopia agrees; FCEUX, puNES, Nintendulator and Mesen all latch the
  written value. MR-66-1 reached this conclusion against Mesen alone, and the three extra votes
  do not change the page. **The one thing that would make this build wrong** is a mapper-66 game
  that writes a bank number which is *not* the byte the PRG-ROM holds at that address, because
  the AND would then mis-bank it; the two best-known mapper-66 cartridges —
  *Super Mario Bros. + Duck Hunt* and the *Dragon Ball* games — write through a table whose
  values match, which is precisely why the hardware tolerates the conflict. Recorded as OK with
  this caveat rather than OPEN, because the page is not ambiguous.
- **Power-on latch 0** — OK (`discrete.rs:347-350`), all four the same.

**Test ROMs.** `holy-mapperel/testroms/M66_P64K_C16K_V.nes` — an MHROM configuration, which also
exercises the bus-conflict path since its driver writes known values. Not run by the project
today.

Count: 4 OK, 0 DIFFERS, 0 OPEN.

<!-- NEXT-BOARD -->
