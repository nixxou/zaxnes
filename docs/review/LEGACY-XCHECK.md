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

## Summary table

**177 points were checked across the 26 boards: 157 OK, 9 DIFFERS, 11 OPEN.** The table below
is every point that is *not* OK — the points of difference. The OK points are listed per board
in the sections that follow, briefly, with their citations; listing all 157 here would bury the
twenty that matter. `—` means the source does not implement the feature at all; a blank cell
means I did not read that source on that point and say so in the entry.

| mapper | board | point | the page | this build | FCEUX | Nestopia | puNES | Nintendulator | Mesen 2 | verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | SNROM | `$A000` bit 4 disables PRG-RAM | yes, on SNROM | **yes** → **no**, decided MF-LEG-1B | no | no | reverted, named game | no | no | **DIFFERS** → agrees with all five |
| 1 | SZROM | which CHR bit banks the 16 KiB PRG-RAM | bit 4 | **bit 3** | bit 3 | bit 3 | bit 4 | no banking | bit 2, "shouldn't happen" | **DIFFERS** |
| 1 | SOROM | which `S` value selects the battery chip | silent | `S`=1 | `S`=1 | `S`=1 | `S`=0 | `S`=0 | `S`=0 | **OPEN** |
| 1 | MMC1A | NES 2.0 submapper 3 | no PRG-RAM disable, bit 4 drives A17 | not handled | mapper 155 only | from the chip list | from the submapper | from the board name | from the database | **DIFFERS** |
| 1 | MMC1 | power-on `$8000` and the PRG-RAM disable | "last bank", rest unspecified | `$0C`, RAM on | `$1F`, RAM on | `$0F`, RAM off on B3 | `$0C`, RAM on | `$0C`, RAM on | random\|`$0C`, RAM off on B | **OPEN** |
| 4 | MMC3 | a `$C001` write racing an A12 clock | silent | not modelled | — | — | **modelled** | — | — | **OPEN** |
| 4 | MMC3 | IRQ assertion delay | silent | none | n/a | none | 1-2 CPU cycles | none | none | **OPEN** |
| 4 | MMC3 | power-on CHR registers | unspecified | all 0 | 0,2,4,5,6,7 | identity | 0,2,4,5,6,7 | identity | all 0 | **OPEN** |
| 5 | MMC5 | vertical split and extended attributes | both documented | **neither** | both | both | both | both | both | **DIFFERS** |
| 5 | MMC5 | power-on `$5104` | reset `$03` | 0 | 0 | 0 | 0 | 0 | | **OPEN** |
| 9 | MMC2 | power-on latch value | silent | `$FE` | `$FE` | `$FD` | `$FD` | `$FD` | | **OPEN** |
| 13 | CPROM | bus conflicts | **"Yes"** | **no** | no | yes | yes | no | no | **DIFFERS** |
| 15 | AMD-021 R2-002 | CHR-RAM write protect | modes 0 and 3 | modes 0 and 3 | mode 3, deliberate | — | never | never, deliberate | modes 0 and 3 | **DIFFERS** |
| 16 | LZ93D50 | does the counter keep counting after the IRQ | silent | yes | no, self-disables | yes | no, stalls at 0 | yes | | **OPEN** |
| 18 | SS88006 | `$9002` PRG-RAM enable / write protect | `.... ..WR` | **not implemented** | no | yes, denied at power-on | yes | no | yes | **DIFFERS** |
| 22/23/25 | VRC2 | the one-bit latch at `$6000-$6FFF` | documented, two games named | **not implemented** | RAM instead | **latch, gated on no WRAM** | **latch, gated on no WRAM** | RAM instead | | **DIFFERS** |
| 30 | UNROM 512 | which submapper gives mapper-controlled H/V | submapper 3 (MR-30-3) | submapper 3 | — | — | submapper 1 | — | submapper 3 | **OPEN** |
| 32 | G-101 sub 1 | which one-screen page *Major League* is held on | CIRAM A10 to +5V, i.e. upper | **lower (screen A)** | — | — | upper (screen B) | — | — | **DIFFERS** |
| 41 | Caltron 6-in-1 | bus conflict on the inner CHR register | not re-read | no | | | yes | | | **OPEN** |
| 101 | AMD-021 R2-003 | the CHR field width and the decode range | whole byte, `$6000-$7FFF` | per the spec | — | — | whole byte | — | whole byte | **OPEN** (Q-110) |

Two further tables sit inside the report rather than here, because they summarise points that
came out **OK**: the shared bus-conflict re-count (eight boards, four new votes) and the MMC3
revision A/B comparison.

**Six of the nine DIFFERS rows were taken by lot MF-LEG-1** (`docs/progress/MF-LEG-1.md`), which
left the two the owner keeps — mapper 15's CHR-RAM write protect and SZROM's bank bit — and the
MMC5 split, which is a lot of its own. Each entry carries a "Fixed in MF-LEG-1" paragraph with its
commit; the rows above are left as this report found them, so that the table and the fix read
against each other.

Two of those six are fixed differently from the way this table states them, both because the page
says something narrower than the paraphrase:

* **1 / MMC1A** — "bit 4 drives A17" is the entry's reading. The page gives `R` = 1 as "fixed bank
  only affects A16..A14, **bit 3** directly controls A17", so it is bit 3, and only for the *fixed*
  window of PRG modes 2 and 3.
* **1 / SNROM** — **decided by R0 in MF-LEG-1B**, and not the way this table's row reads: the
  shipped build applies **no `E` bit at all**, agreeing with all five emulators against the page,
  and MF-LEG-1's header rule is gone. The page's behaviour is behind `mmc1-snrom-e-bit`, off in
  `default`; the price is two `holy-mapperel` rows that are now required failures. That entry and
  **Q-176** have the headers, the five votes and the reason.


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

**Decided in MF-LEG-1B (`e1c4d60b`).** R0's answer to Q-176, 2026-11-08: the entry's option
**(b)**, and further than (b) asks. By default this build applies **no `E` bit at all** — the
PRG-RAM is disabled by `$E000` bit 4 only, and never on an MMC1A — which is what all five of
FCEUX, Nestopia UE, puNES, Nintendulator and Mesen 2 do and what the table above records them
doing. So this row's verdict **DIFFERS** no longer describes the shipped build: it now agrees
with all five and departs from the page, deliberately, for the reason the code records at the
bit's own site. The page's behaviour stays reachable behind **`mmc1-snrom-e-bit`**, off in
`default`, which restores the three-way size test exactly; nothing is dropped, per the owner's
"no exclusion" rule. The cost is `holy-mapperel`'s two SNROM rows, `M1_P128K` and
`M1_P128K_CR8K`, which become **required failures** in `default` — marked `expected` in the
harness report like `mmc3_test/6-MMC6`, and asserted to pass with the feature on.

MF-LEG-1's own two commits are superseded by it and are recorded here because the reasoning took
two lots. **Fixed in MF-LEG-1 (`1cc4ce1f`), and then put behind a feature off by default
(`530979f7`).** Option (a) was built: `Mmc1::is_snrom` asks an NES 2.0 header for no CHR-ROM,
8 KiB of CHR-RAM in byte 11, a non-zero byte 10 totalling 8 KiB and at most 256 KiB of PRG.
**But two statements of this entry are wrong, and they are what sent the fix behind
`mmc1-snrom-header` (Q-176).** `holy-mapperel`'s `M1_P128K.nes` and `M1_P128K_CR8K.nes` are NES
2.0 with no CHR-ROM, 8 KiB of CHR-RAM in byte 11 and **byte 10 = `$00`** — so the old test saw
REQ-CORE-012's 8 KiB default and returned *true* for them, where this entry says it "returns
false for all twelve". They announce themselves as `SNROM (MMC1)`, report `8K PRG RAM OK` and
probe digit `4000`, so the suite **does** exercise the SNROM side, on two rows that lot MF-3
step 1 exists to have made pass. They fail under the narrowing, and under MF-LEG-1B's decision
they fail for a different and better-documented reason. The hardware test is right about
the board and the header rule is right about real dumps, and both cannot hold for a dump whose
header under-declares the cartridge it tests, so `default` keeps the size test and the narrowing
takes one flag. The entry's *Risk of a fix* is confirmed: the five named games were played for
3600 frames each with the register sampled after **every CPU instruction** — 181 M instructions,
not one commit of bit 4 to `$A000` — and golden-core over the 98 originals, 27 of them mapper 1,
is byte-identical both ways. One more correction for the owner: **Zelda II was never affected**,
its dumps carrying 128 KiB of CHR-ROM, which the old size test already excluded.

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

**Fixed in MF-LEG-1 (`ba326895`)**, behind `board-mmc1a`, and **the entry's reading of the `R`
bit is not the page's**. The page gives `R` = 1 on an MMC1A as "fixed bank only affects
A16..A14, **bit 3** directly controls A17 across the entire $8000-$FFFF address range" — so it is
bit 3 that drives A17, and only for the *fixed* window of PRG modes 2 and 3; the switchable
window and 32 KiB mode have bit 3 as A17 already, which is the rest of that sentence. AMD-061
makes the page prevail, so the two fixed-bank numbers are now `prg & r` and
`((0x08 & !r) | (prg & r)) | 0x07`, a mask for the reason `Mmc1::prg_disable_mask`'s own note
gives. Two further notes. The suppression is of `prg_disable_mask` **only**: the SNROM `E` bit is
the board wiring CHR A16 to the RAM's second chip enable, whichever revision of the ASIC sits on
it, where mapper 155's page removes a wire. And `Mapper::mapper_number` had to learn the
difference, both boards now having a zero `prg_disable_mask`. **Follow-up for the owner, not in
this entry**: the page makes **mapper 155** the same MMC1A revision, so it should get the `R`
bit's A17 too; it is out of this lot's scope and is one condition on the same line.

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
| 13 CPROM | **"Subject to bus conflicts: Yes"** | never | never (`datalatch.cpp:243`) | always (`NstBoardCxRom.cpp:81`) | always (`mapper_013.c:46`) | never (`mapper013.cpp:19`) | never |
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
- **CPROM is the one row that turned into a finding.** The *CPROM* board page carries
  "Subject to bus conflicts: Yes" in its characteristics table, Nestopia and puNES both
  implement it, and this build does not — see mapper 13 below.

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

---

## Mapper 4 — MMC3 (TxROM), MMC6 and the MC-ACC

Pages: *MMC3* (<https://www.nesdev.org/wiki/MMC3>), *MMC6*
(<https://www.nesdev.org/wiki/MMC6>), *NES 2.0 submappers*. Ours: `mappers/mmc3.rs`
(the generic `Mmc3Board`, of which `Mmc3` and `McAcc` are two monomorphisations),
`cartridges/a12_watcher.rs`, `registry.rs:1185-1187` for the MC-ACC dispatch.
Theirs: `fceux/src/boards/mmc3.cpp`, `nestopia/source/core/board/NstBoardMmc3.cpp` and `.hpp`,
`punes/src/core/mappers/MMC3.c`, `punes/src/core/mappers/mapper_004.c`,
`punes/src/core/irqA12.c` and `irqA12.h`, `nintendulator-mappers/src/Hardware/h_MMC3.cpp`,
`nintendulator-mappers/src/iNES/mapper004.cpp`,
`mesen2/Core/NES/Mappers/Nintendo/MMC3.h`.

This is the board MR-1 examined hardest — nine findings, six fixed in MF-1, one in MF-2 — so
the register map is re-verified briefly and the weight goes on the IRQ, where the four new
sources disagree with each other more than they disagree with this build.

### The register map — OK

- **$8000 is `CPxx xRRR`**: bits 2-0 select R0-R7, bit 6 swaps the $8000 and $C000 PRG
  windows, bit 7 inverts the CHR layout (`mmc3.rs:790-807`, `mmc3.rs:1238-1241`). Same in all
  four: FCEUX recomputes both layouts from the same two bits (`mmc3.cpp:76-101`), Nestopia XORs
  the window index with bits derived from them (`NstBoardMmc3.cpp:283-302`), puNES the same
  (`MMC3.c:208-236`), Nintendulator XORs the bank index (`h_MMC3.cpp:73-83`).
- **$8001 writes the selected register** — OK (`mmc3.rs:1242-1245`), all four the same.
- **The fixed windows: $A000-$BFFF is always R7, and the un-swapped half of
  $8000/$C000 is always the second-to-last bank, with the last bank at $E000** — OK
  (`mmc3.rs:791-799`). All four the same (`mmc3.cpp:84-85`, `NstBoardMmc3.cpp:287-290`,
  `MMC3.c:209-216`, `h_MMC3.cpp:73-78`).
- **R0 and R1 are 2 KiB and ignore bit 0** — OK (`mmc3.rs:803-804`, `bank[n] >> 1` into a 2 KiB
  window). The others expand the pair into two 1 KiB windows with bit 0 forced low and high
  (`mmc3.cpp:91-94`, `NstBoardMmc3.cpp:224-225`, `h_MMC3.cpp:155-158`). Same banks. MR-1
  re-checked this and it stands.
- **$A000 bit 0: 1 = horizontal, 0 = vertical** — OK (`mmc3.rs:1246-1253`). All four read the
  bit the same way round: FCEUX inverts it into its own enum, which comes out the same
  (`mmc3.cpp:253`), puNES (`MMC3.c:252-263`), Nintendulator drives CIRAM A10 from PPU A11 when
  the bit is set (`h_MMC3.cpp:66-71`), Nestopia stores it and derives the same table
  (`NstBoardMmc3.cpp:238-240`).
- **A four-screen header suppresses $A000 entirely** — OK (`mmc3.rs:451-453`,
  `mmc3.rs:1247`), and a three-way agreement: puNES returns early from its mirroring update
  with the same comment (`MMC3.c:252-257`), and Nestopia simply does not install a handler on
  $A000 when the board is four-screen (`NstBoardMmc3.cpp:90-95`). FCEUX and Nintendulator leave
  the PPU's four-screen wiring to absorb it.
- **$A001: bit 7 is the PRG-RAM chip enable, bit 6 the write protect** — OK
  (`mmc3.rs:1146-1160`). Three of the four agree exactly: Nestopia makes the read permission
  bit 7 and the write permission "bit 7 set and bit 6 clear" (`NstBoardMmc3.cpp:238-247`),
  puNES computes read from bit 7 and write from read-and-not-bit-6 (`MMC3.c:240-243`),
  Nintendulator gates reads on bit 7 (`h_MMC3.cpp:91-100`) and blocks writes on bit 6
  (`h_MMC3.cpp:144-148`). **FCEUX implements neither**: it stores $A001 and then maps
  $6000-$7FFF as plain readable/writable RAM, consulting the register only in its bootleg
  derivatives (`mmc3.cpp:174`, `mmc3.cpp:286-289`, and the only in-file use at
  `mmc3.cpp:623`, a non-MMC3 board). Four to one for this build.
- **$C000 latch, $C001 reload, $E000 disable-and-acknowledge, $E001 enable** — OK
  (`mmc3.rs:1263-1277`). All four the same (`mmc3.cpp:181-186`, `NstBoardMmc3.cpp:249-271`,
  `MMC3.c:114-137`, `h_MMC3.cpp:178-193`).
- **$E000 acknowledges a pending IRQ as well as disabling** — OK (`mmc3.rs:1274-1275`); all
  four clear the CPU line there too.
- **The MC-ACC (submapper 3) is a board of its own** — OK. `registry.rs:1185-1187` builds
  `McAcc` for a NES 2.0 mapper 4 with submapper 3, which counts A12 **falls** divided by eight
  instead of filtering rises (`mmc3.rs:430-439`, `mmc3.rs:1285-1306`), and whose $C001 resets
  the prescaler (`mmc3.rs:1269-1271`). This is the page's own description of the Acclaim part.
  Nestopia has the board as a separate class (`NstBoardAcclaimMcAcc.cpp`); the other three do
  not implement it at all. MR-4-3 raised it and MF-1/MF-2 closed it; re-confirmed here.

### MMC3-1 — the revision A/B difference, and what the other four actually do — OK

**Point.** The page: revision A "checks the IRQ counter transition 1→0, whether from
decrementing or reloading", giving a **single** IRQ when the latch is $00; revisions B and C
generate an IRQ "when the scanline counter is *equal* to 0", giving one on **every** clock
when the latch is $00. The page elsewhere attributes the repeating behaviour to Sharp MMC3B and
MMC3C and the single IRQ to Sharp MMC3A and all NEC parts, and *NES 2.0 submappers* gives
mapper 4 submapper 4 as "NEC MMC3 (Loading latch with 0 disables IRQ)".

**This build.** Revision B by default; revision A when the header declares submapper 4
(`mmc3.rs:457`). The revision A condition is "the counter was moving" — non-zero before the
clock **or** a reload pending (`mmc3.rs:947`) — which is the page's "whether from decrementing
or reloading" and therefore yields exactly one IRQ with a latch of $00.

**The others.**

| | default revision | latch $00 gives | reads the submapper? | file:line |
| --- | --- | --- | --- | --- |
| this build | B | an IRQ on every clock | yes, submapper 4 → A | `mmc3.rs:457`, `mmc3.rs:937-951` |
| FCEUX | B | an IRQ on every clock | no | `mmc3.cpp:60`, `mmc3.cpp:207-219` |
| Nestopia | B | an IRQ on every clock | no — the revision comes from its board database | `NstBoardMmc3.cpp:38`, `NstBoardMmc3.hpp:64-78` |
| puNES | **A** | a single IRQ | yes, submapper 4 → the *repeating* clock | `irqA12.h:33-53`, `MMC3.c:171-181`, `mapper_004.c:59-61` |
| Nintendulator | **A**, always | a single IRQ | no | `h_MMC3.cpp:194-210` |
| Mesen 2 | B | an IRQ on every clock | yes, submapper 3 (its MC-ACC) | — |

Two things worth recording:

- **This build and Nestopia implement revision A identically**, down to the expression:
  Nestopia's clock computes "counter non-zero or reload pending" before the reload and ANDs it
  with a `persistent` flag that is false only for its REV_A (`NstBoardMmc3.hpp:64-78`). FCEUX
  implements revision A with the reload term **missing** (`mmc3.cpp:214`: the pre-clock count
  alone), so on FCEUX a revision A part with a latch of $00 gives *no* IRQ at all rather than
  one — which contradicts the page's "single IRQ".
- **puNES has the two revisions the wrong way round.** Its default clock requires the saved
  previous counter to be non-zero, which is revision A's single-IRQ behaviour
  (`irqA12.h:33-53`), and the clock it installs *for submapper 4*, the NEC part the page says
  gives a single IRQ, fires whenever the counter is zero — revision B's repeating behaviour
  (`MMC3.c:171-181`, selected at `mapper_004.c:59-61`). Nintendulator applies revision A to
  every mapper-4 image.

**Verdict: OK.** This build matches the page on both revisions, matches Nestopia exactly on the
revision A expression, matches FCEUX, Nestopia and Mesen on the default, and is the only one
besides Mesen that reads the submapper correctly. REQ-CORE-100 pins revision B for the default
and `mmc3_test_2` test 6 requires it, so nothing here asks to move.

### MMC3-2 — the A12 filter: this build's shape is the majority's, its length is in the middle — OK

**Point.** The page: the counter is clocked by a rising edge of A12 "after the line has
remained low for three falling edges of M2". Three M2 falls is about nine PPU dots. MR-A12-1
already settled the *unit* question against Mesen.

**This build.** `A12Watcher::rose` accepts a rise when A12 has been low for at least ten dots
(`a12_watcher.rs:71-80`, `A12Watcher::MMC3_FILTER_DOTS = 10`) — a **low-duration** filter, the
page's own shape.

**The others — two different shapes, and one that does not model A12 at all.**

| | shape | threshold | file:line |
| --- | --- | --- | --- |
| this build | A12 must have been low for N dots | 10 dots (≈3.3 CPU cycles) | `a12_watcher.rs:41`, `a12_watcher.rs:71-80` |
| Nintendulator | A12 must have been low for N dots | **8 dots** | `h_MMC3.cpp:194-210` (an 8-dot countdown rearmed on every high) |
| Mesen 2 | A12 must have been low for N dots | ≈9 dots | — (settled in MR-A12-1) |
| Nestopia | at least N dots since the **previous accepted rise** | 16 dots | `NstBoardMmc3.hpp:42-45`, `NstTimer.hpp:154-182` |
| puNES | more than N dots since the **previous accepted rise** | 18 dots | `irqA12.h:30`, `irqA12.h:34` |
| FCEUX | no A12 model: one clock per rendered scanline, from the PPU's hblank hook | — | `mmc3.cpp:221-223`, `mmc3.cpp:345` |

**Verdict: OK.** On shape this build is with Nintendulator and Mesen — the three that model what
the page describes — against Nestopia's and puNES's "minimum interval between clocks", which is
a different filter that happens to behave the same during ordinary rendering. On length, ten
dots sits between Nintendulator's eight and Mesen's nine on one side and nothing on the other,
and all three bracket the documented three M2 falls. The readings only part on an A12 low of
7-9 dots, which MR-A12-1 already recorded as unproducible by any rendering pattern, and on
rapid hand-clocking through $2006, where Nestopia and puNES would drop clocks this build
accepts. FCEUX's scanline approximation is not a vote on the filter at all.

### MMC3-3 — the $C001 reload glitch the page documents, and which this build does not need — OK

**Point.** New enough on the page to be worth stating: a write to $C001 does not clear the
counter on hardware. It "instantly set[s] the counter to a value above 127 to prevent it from
reaching 0" — OR $F0 on an NEC MMC3A, OR $FF on an NEC MMC3B-C, OR $80 on a Sharp MMC3A-C —
and the reload from $C000 then happens at the next A12 rise.

**This build.** $C001 sets the counter to **0** and raises the reload flag
(`mmc3.rs:1264-1266`).

**The others.** All four do what this build does: counter to zero plus a reload flag
(`mmc3.cpp:183` sets only the flag, `NstBoardMmc3.cpp:255-259`, `MMC3.c:117-128`,
`h_MMC3.cpp:178-186`). None of them implements the OR-mask glitch.

**Verdict: OK, and provably equivalent.** The MMC3 has no readable counter, so the only way the
value between the $C001 write and the next A12 rise can be observed is through the next clock —
and at that clock both readings take the same branch: this build because the counter is zero
*and* the reload flag is set, hardware because the reload flag is set. It is also equivalent
under revision A: this build's "moving" test is satisfied by the pending reload
(`mmc3.rs:947`), and hardware's by the OR'd value being non-zero. Recorded because the page's
wording invites the reader to think a fix is needed; it is not.

### MMC3-4 — the power-on bank registers and $A001 — OPEN, and benign

**Point.** The page: "Because the values in R6, R7, and $8000 are unspecified at power on, the
reset vector must point into $E000-$FFFF, and code must initialize these before jumping out of
$E000-$FFFF." It gives no power-on value for $A001 either.

**This build.** R0-R5 = 0, R6 = 0, R7 = 1, $8000 = 0, $A001 = $80 — PRG-RAM enabled and
writable (`mmc3.rs:1162-1171`, `mmc3.rs:482-488`); on an MMC6, $A001 = $00, which *is*
documented (MR-4-7).

**The others.**

| | the eight CHR registers R0-R5 | R6, R7 | $A001 | file:line |
| --- | --- | --- | --- | --- |
| this build | all 0 | 0, 1 | $80 | `mmc3.rs:1163-1166`, `mmc3.rs:482-488` |
| FCEUX | 0, 2, 4, 5, 6, 7 | 0, 1 | 0, and never consulted | `mmc3.cpp:103-115`, `mmc3.cpp:278` |
| puNES | 0, 2, 4, 5, 6, 7 | 0, 1 | **$80** | `MMC3.c:183-199` |
| Nestopia | the identity, 0…7 as eight 1 KiB banks | 0, 1 (its other two slots are the fixed $3E/$3F) | disabled **and** unwritable | `NstBoardMmc3.cpp:61-74` |
| Nintendulator | the identity, 0…7 | the last four 8 KiB banks | $80 for an iNES image, 0 otherwise, "for compatibility" | `h_MMC3.cpp:30-45` |
| Mesen 2 | all 0 | 0 | 0 | — (MR-4-4) |

**Verdict: OPEN**, closing the same way MR-PWR-1 closed it: the page says the values are
unspecified, so no layout can be called wrong. Two pieces of new information, though:

- **The vote on $A001 goes this build's way.** MR-4-5 kept $80 because it is the only value
  that lets blargg's MMC3 ROMs — which never write $A001 — use the PRG-RAM from a cold start.
  puNES powers $A001 up at exactly $80 (`MMC3.c:197`) and Nintendulator at $80 for every iNES
  image, with a comment giving the same reason ("for compatibility, enable RAM by default").
  Three of six against Nestopia's disabled-and-unwritable, Mesen's 0 and FCEUX's
  stored-but-ignored.
- **On the CHR registers this build is alone.** R6 = 0 and R7 = 1 are shared with FCEUX,
  puNES and Nestopia, but all four of the others power the *CHR* registers up to something
  monotonic — the identity 0…7, or 0, 2, 4, 5, 6, 7, which is the same layout expressed in
  the register's own units — where this build leaves all six at zero, so the first pattern
  table is eight copies of CHR bank 0 until the game writes $8001 six times. Nothing in the
  page forbids it and no game can rely on it, but if the project ever wants a frame of a
  not-yet-initialised MMC3 game to look like it does elsewhere, the monotonic seeding is the
  four-way consensus and costs a six-element array initialiser at load.

**What would settle it.** Nothing public can: the page states the values are undefined.

### MMC3-5 — a $C001 write in the same cycle as an A12 clock — OPEN

**Point.** puNES carries a mechanism no other source mentions: a `race` record taken on every
$C001 write, holding the counter and reload flag as they stood *before* the write, so that an
A12 clock arriving in the same cycle is computed against the pre-write values
(`MMC3.c:117-128`, used at `irqA12.h:38-41`).

**This build.** No such mechanism. `write_register`'s $C001 arm and `on_ppu_address_bus` are
ordered by whichever the bus calls first (`mmc3.rs:1264-1271`, `mmc3.rs:1285-1306`).

**The others.** FCEUX, Nestopia, Nintendulator and Mesen have nothing equivalent.

**Verdict: OPEN.** One implementation out of six thought the race worth modelling, and its
source does not say which game made it necessary; the page does not describe the race at all.
Five-to-one is not a majority this build is on the wrong side of, but a lone implementer who
went to the trouble of recording two shadow fields usually had a symptom. Consequence if it
matters: one scanline of IRQ timing on a game that writes $C001 from inside a tight raster
loop.

**What would settle it.** `assets-free/test-roms/nes-test-roms/mmc3_irq_tests` and
`mmc3_test_2`, run on this build with the IRQ cycle reported; neither targets this race
specifically, so strictly it needs a new test ROM that writes $C001 at a known dot of a known
scanline and reports the IRQ cycle.

### MMC3-6 — the IRQ assertion delay — OPEN

**Point.** puNES delays the CPU IRQ line by one CPU cycle after the counter reaches zero, and
by two when the clock lands on the penultimate cycle of an instruction, with a comment
explaining that its CPU/PPU synchronisation runs one CPU clock ahead
(`irqA12.h:56-67`, `mapper_004.c:67`).

**This build.** `clock_irq_counter` raises `irq_pending` immediately (`mmc3.rs:948-949`) and
`Mapper::irq_asserted` answers from it (`mmc3.rs:1125-1127`).

**The others.** Nintendulator asserts immediately (`h_MMC3.cpp:204-206`); Nestopia routes the
assertion through its own A12 timer, which has a template `Delay` parameter defaulting to zero
and left at zero for the plain MMC3 (`NstBoardMmc3.hpp:112-117`); FCEUX asserts from the
scanline hook. MR-64-4 examined the same question on the RAMBO-1, where the page *does*
document a delay.

**Verdict: OPEN**, and probably an artefact of puNES's own scheduling rather than hardware —
its comment says so. Recorded because it is the one place where a source adds latency this
build does not have, and because the symptom would be identical to MMC3-5's: an IRQ serviced
one instruction later.

### The MMC6, re-checked against the new sources

MR-4-1, MR-4-6, MR-4-7 and MR-4-8 all concerned the MMC6 and all four were fixed in MF-1. Of
the four new sources only two implement the chip at all: puNES behind submapper 1, with a
512-byte RAM region and its own read handler that answers 0 from the disabled half and open bus
when neither half is readable (`mapper_004.c:51-58`, `mapper_004.c:70-77`), and FCEUX behind a
board flag with a 1 KiB RAM at $7000-$7FFF (`mmc3.cpp:260-266`, `mmc3.cpp:281-284`).
Nintendulator and Nestopia do not have it as a mapper-4 submapper.

- **The 1 KiB size** (MR-4-6) — confirmed by FCEUX's `WRAMSIZE == 1024` test (`mmc3.cpp:281`).
- **Open bus when no half is readable** (MR-4-8) — confirmed by puNES, which returns the
  incoming open-bus byte in exactly that case (`mapper_004.c:72-74`). This build's
  `cpu_peek_override` now does the same.
- **$A001 held at zero while the RAM is disabled** (MR-4-7) — neither of the two models it;
  the page states it, and this build follows the page.

**Verdict: OK on all three**, with MR-4-7 resting on the page alone.

**Test ROMs.** The richest set of any board here. In
`assets-free/test-roms/nes-test-roms`: `mmc3_test` and `mmc3_test_2` (six numbered tests
including the latch-$00 revision question of MMC3-1), `mmc3_irq_tests`, and `scanline` /
`scanline-a1` which depend on the A12 counter. In `assets-free/test-roms/holy-mapperel`:
`M4_P128K.nes`, `M4_P128K_CR8K.nes`, `M4_P128K_CR32K.nes`, `M4_P256K_C256K.nes`. The project
**does** run part of this: REQ-CORE-114 names `mmc3_test_2` and MR-1 records tests 1-5 passing
and test 6 as the revision gate, so the MMC3 is the one board of the 26 whose IRQ is covered by
a ROM test in the harness. `mmc3_irq_tests` and the four `M4_*` images are not run.

Count: 12 OK, 0 DIFFERS, 3 OPEN.

---

## Mapper 5 — MMC5 (ExROM), with its expansion audio

Pages: *MMC5* (<https://www.nesdev.org/wiki/MMC5>), *MMC5 audio*
(<https://www.nesdev.org/wiki/MMC5_audio>). Ours: `mappers/mmc5.rs`, `apu/mmc5_audio.rs`,
`apu/mmc5_pulse_channel.rs`. Theirs: `fceux/src/boards/mmc5.cpp`,
`nestopia/source/core/board/NstBoardMmc5.cpp` and `.hpp`,
`punes/src/core/mappers/mapper_005.c`, `punes/src/core/apu.h`,
`nintendulator-mappers/src/Hardware/h_MMC5.cpp`,
`nintendulator-mappers/src/Hardware/Sound/s_MMC5.cpp`,
`mesen2/Core/NES/Mappers/Nintendo/MMC5.h`, `mesen2/Core/NES/Mappers/Audio/Mmc5Audio.h`.

MR-1 reviewed this board against Mesen and MF-1 fixed MR-5-1, MR-5-3 and MR-5-5; MR-5-4 (the
in-frame detection) was reworked into the bus-sniffing form the page describes, and Q-121 holds
the one line the page does not pin. What the four new sources add is a clear majority on two
things MR-1 could only see one side of: the split screen, and the audio's low-period cutoff.

### The register map — OK

- **$5100 PRG mode, $5101 CHR mode, $5104 ExRAM mode, $5105 nametable mapping, $5106/$5107
  fill, $5113 PRG-RAM bank, $5114-$5117 PRG banks, $5120-$512B CHR banks, $5130 upper CHR bits,
  $5203/$5204 IRQ, $5205/$5206 multiplier** — all decoded, all at the page's addresses and with
  the page's fields (`mmc5.rs:163-205`). The four agree register for register
  (`mmc5.cpp:490-570`, `NstBoardMmc5.cpp:1209-1420`, `mapper_005.c:182-300`,
  `h_MMC5.cpp:477-560`).
- **All four PRG modes, and the mask each window applies** — OK (`mmc5.rs:207-241`). puNES
  writes the masks out per mode — the 32 KiB window takes the register's bits 6-2, the 16 KiB
  windows bits 6-1 (`mapper_005.c:520-542`) — where this build derives them from the window
  count; same banks.
- **Bit 7 of $5114-$5116 chooses RAM or ROM, and $5117 is always ROM** — OK
  (`mmc5.rs:227-241`); puNES tests the same bit (`mapper_005.c:544-545`). MR-1 proved this
  predicate equivalent to Mesen's register-based one.
- **$5102 = $02 and $5103 = $01 are both required to write PRG-RAM** — OK
  (`mmc5.rs:547-549`), exactly the page's requirement; puNES stores the same pair
  (`mapper_005.c:192`).
- **$5203 = $00 never produces an IRQ** — OK (`mmc5.rs:313-314`), which is the page's "special
  case".
- **The pending flag sets whether or not the IRQ is enabled, and only the AND of the two drives
  /IRQ** — OK: `detect_scanline` sets `irq_pending` unconditionally (`mmc5.rs:315`) and
  `refresh_irq_line` ANDs it with the enable (`mmc5.rs:245-247`).
- **$5204 read: bit 7 the pending flag, bit 6 in-frame, and reading clears the pending flag
  only** — OK (`mmc5.rs:155-157`, `mmc5.rs:616-623`). The page also gives **bit 0** as an
  "abnormal M2/CPU A0 sequence detection flag"; no implementation here or among the four exposes
  it, and no game reads it.
- **Scanline detection: three identical nametable reads then any PPU read, at PPU cycle 4** —
  OK (`mmc5.rs:261-318`), the page's own mechanism, with `need_in_frame` making the first
  detected line arm-and-zero without a compare. This is MR-5-3 and MR-5-4 as MF-1 left them.
- **In-frame drops on the PPU going idle and on an $FFFA/$FFFB read** — OK
  (`mmc5.rs:320-326`). The page lists two more triggers (the V-blank NMI and a PPUMASK
  disable), both of which the idle counter reaches by a different route.
- **The 64 KiB PRG-RAM forced regardless of the header** — OK (`registry.rs`
  `minimum_prg_ram_size`, `Mmc5::MAX_PRG_RAM_SIZE = 64 KiB`), which MR-1 showed equivalent to
  Mesen's "emulate as if a single 64k block existed" for an iNES image.
- **$5113 masked to three bits** — OK with a note. The page says "bits 7-4 ignored", i.e. four
  bits; this build and Mesen use three (`mmc5.rs:178`). Three bits address the whole 64 KiB,
  so the fourth can only select a chip that no licensed board has.
- **Expansion audio register map ($5000-$5007 two pulses, $5010 PCM mode/IRQ, $5011 raw PCM,
  $5015 status), no sweep unit, envelopes and length counters both at a fixed 240 Hz, both PCM
  modes, and the PCM IRQ on the CPU line** — OK, all of it the page's text, and the PCM IRQ
  driving /IRQ is implemented here (`apu/mmc5_audio.rs:23-31`, `mmc5.rs:245-247`) where Mesen
  left it a TODO. Nintendulator is the only one of the four that also implements read mode and
  the IRQ (`s_MMC5.cpp:143-153`, `s_MMC5.cpp:186-226`).
- **The output polarity is reversed relative to the APU** — OK. The page: "The polarity of all
  MMC5 channels is reversed compared to the APU." This build subtracts `M` from the APU's `A`
  (AMD-061 §6.2, `apu/mmc5_audio.rs:30-33`) and Mesen negates; FCEUX adds the amplitude into
  its wave buffer (`mmc5.cpp:797`, `mmc5.cpp:827`), Nintendulator sums the two squares into its
  positive accumulator (`s_MMC5.cpp:236-241`), and Nestopia sums through its ordinary external
  channel path. Two of six follow the page and this build is one of them; the other three are
  simply summing every channel with one sign, which is a mixer convention rather than a reading
  of the board.

### MMC5-1 — the vertical split and extended attribute mode are not implemented here and are implemented by all five others — DIFFERS

**Point.** Two features of this board that change what is on screen:

- **Extended attribute mode** ($5104 = %01). Each ExRAM byte supplies, per background tile, a
  6-bit 4 KiB CHR bank and a 2-bit palette override (`AACC CCCC`), so the background escapes
  both the 8x8 attribute grid and the single CHR window.
- **Vertical split mode** ($5200-$5202). A tile-count threshold splits the screen vertically;
  the split region scrolls independently, fetches its nametable from ExRAM only, and takes its
  CHR from the $5202 bank.

**This build.** Neither. The board's own doc comment says so in one sentence
(`mmc5.rs:12-15`), `write_control` has no $5200-$5202 arm at all (`mmc5.rs:163-204`, which
falls through to `_ => {}`), and `ex_ram_mode` 1 is treated exactly like mode 0.

**The others.** All five implement both:

| | split | extended attributes | file:line |
| --- | --- | --- | --- |
| FCEUX | yes | yes | `mmc5.cpp:568-570` ($5200-$5202), `mmc5.cpp:178-267` (the split and `MMC5HackCHRMode == 1` paths in the PPU read hook) |
| Nestopia | yes | yes, and the two composed | `NstBoardMmc5.hpp:191-240` (`ClockSpliter`, `GetSpliterAttribute`, `FetchNtExtSplit`, `CRomExtSplit`), dispatched at `NstBoardMmc5.cpp:1038-1041` |
| puNES | yes | yes | `mapper_005.c:243-256` ($5200-$5202 with side, threshold, scroll and bank), `mapper_005.c:459` (the per-tile 4 KiB bank), `mapper_005.c:494` (the per-tile palette) |
| Nintendulator | yes | yes | `h_MMC5.cpp:18`, `h_MMC5.cpp:531-538`, `h_MMC5.cpp:576-599` |
| Mesen 2 | yes | yes | `MMC5.h` `_verticalSplitEnabled`, `_splitTileNumber` (MR-1) |

**Verdict: DIFFERS** — unanimously, five to one, and against the page. **This is not a
regression and not an oversight**: REQ-CORE-100's mapper-5 row asks for "au minimum : modes de
banques PRG et CHR, PRG-RAM, IRQ de ligne" and marks "ExRAM, attributs étendus, split
vertical" as **P2**, so the spec itself records the gap, MR-1 recorded it, and the code comment
records it. It is reported here because the brief asks for the comparison against the page and
the majority, and because it is by a wide margin the largest *visible* difference in the whole
v0.4.2 set.

**Which games it affects, and which are known affected.** The page names them, which is better
evidence than any of the five implementations:

- **Extended attribute mode**: *Just Breed*, *Yakuman Tengoku*, and the Koei strategy titles —
  *Nobunaga's Ambition II*, *Romance of the Three Kingdoms II*, *Uncharted Waters*, *Bandit
  Kings of Ancient China*. On this build these render with the ordinary 8x8 attribute grid and
  one CHR window, so backgrounds show wrong colours per tile and, where the per-tile bank
  differs, wrong tiles.
- **Vertical split**: *Uchuu Keibitai SDF* (the intro) and *Bandit Kings of Ancient China*
  (the ending). On this build the split region is simply absent — the normal background is
  drawn across the whole line.
- **Not affected**: *Castlevania III* uses the PRG/CHR banking and the scanline IRQ, which are
  implemented, so the most-played MMC5 game is not in this list.

**Risk of a fix.** Contained but not trivial, and this is the part worth being honest about.
Extended attribute mode needs the PPU's nametable and attribute fetches routed through the
board per tile, which the current `nametable_page` / `ppu_peek` interface does not express:
`chr_address` is called with a `ChrFetch` kind but not with the tile index, and the attribute
byte is not asked of the board at all. The split needs a per-line tile counter fed from the
PPU's fetch sequence. Both put work on the background fetch path, which the comments in
`mmc5.rs` and `mmc3.rs` show is the one place this project has repeatedly measured regressions
(`Mmc5::see_ppu_read` carries an `#[inline(never)]` and a callgrind figure for exactly this
reason). SMB3, Zelda II and the widescreen packs are not mapper 5 and cannot be moved by a
mapper-5 change, but a careless implementation could cost every game a few instructions per
fetch. The safe shape is the one the board already uses: decide everything decidable per
*register write*, and leave the per-fetch path a load and a branch.

**What a fix would look like.** Three pieces, in order of value: (a) ExRAM mode 1's per-tile
palette override, which is what the six Koei/Just Breed titles need most visibly and which
needs only the attribute fetch to consult ExRAM; (b) the per-tile 4 KiB CHR bank from the same
byte; (c) $5200-$5202 and the split tile counter, which is two games. (a) and (b) are one
feature in the hardware and probably one change here.

### MMC5-2 — the low-period cutoff: this build follows the page and three of the four do not — OK

**Point.** *MMC5 audio* is explicit: "Frequency values less than 8 do not silence the MMC5
pulse channels; they can output ultrasonic frequencies." The 2A03's pulse channels do silence
below 8, as a side effect of the sweep unit's overflow logic; the MMC5's, having no sweep unit,
do not.

**This build.** No cutoff. This is where Q-111 ended up: `Mmc5PulseChannel` exists as a type of
its own precisely so that it can drop the two units an APU pulse has — the sweep and the cut
below period 8 — without touching the APU pulse that every game runs
(`apu/mmc5_pulse_channel.rs:6-18`, and `output` at `:30-43` has no period test at all,
quoting the page's sentence in its doc comment). AMD-061 §6.1 is the amendment that made it so.

**The others — nobody agrees with anybody, and the page's own reading is the minority.**

| | silences when | file:line |
| --- | --- | --- |
| this build | **never** | `apu/mmc5_pulse_channel.rs:30-43` |
| puNES | **never** — its low-period cutoff lives in `sweep_silence` (`apu.h:103-110`), and the MMC5 squares never run a sweep, so the flag stays clear | `apu.h:125-130`, `mapper_005.c:507-518` |
| Mesen 2 | **never**, with an `_isMmc5Square` flag threaded through its shared pulse class specifically to disable the cutoff for this board | MR-1 |
| FCEUX | period + 1 < 8 | `mmc5.cpp:788`, `mmc5.cpp:818` |
| Nestopia | period < 4 (`MIN_FRQ`) | `NstBoardMmc5.hpp:104`, `NstBoardMmc5.cpp:264-267` |
| Nintendulator | period ≤ 1, with the reason in the comment: "JustBreed sets Freq=1 in places, which sounds very bad due to lack of downsampling" | `s_MMC5.cpp:55-57` |

**Verdict: OK**, and worth recording as a success rather than a draw: the page, this build,
puNES and Mesen have no cutoff; FCEUX, Nestopia and Nintendulator each invented a different
threshold. Nintendulator's comment says *why*, and it is the useful part of this entry — an
ultrasonic square aliases badly without downsampling, so three emulators added a cutoff the
hardware does not have, as a resampling workaround. **The practical warning for this project is
therefore the mirror image of a finding**: because this build correctly lets the channel run
ultrasonic, *Just Breed* — the title Nintendulator names as setting period 1 — will put a
near-Nyquist square through this project's output path, and whether that sounds like silence,
a tone or aliasing noise depends entirely on the resampling downstream of
`Mmc5Audio::output`. If anyone ever reports *Just Breed* sounding harsh on this build, the
cause is here and the fix belongs in the resampler, not in the channel.

MR-1's Q-111 table is therefore closed on both of its points: the cutoff went the page's way
(AMD-061 §6.1), and so did the polarity (§6.2) — `apu/mixer.rs:34` quotes the same page, and
`apu/mod.rs:14` records that `M` is subtracted from the APU's `A`.

### MMC5-3 — the power-on value of $5104 — OPEN


**Point.** The *MMC5* page gives reset values for several registers: $5100 = $03, $5102 = $01,
$5103 = $10, $5104 = **$03**, $5117 = $FF, $5130 = $00, $5200/$5201 = $00.

**This build.** $5100 = 3 ✓, $5117 = $FF ✓, $5130 = 0 ✓, $5102/$5103 = 0 (which is
"not writable", the same effect as the page's pair) — and **$5104 = 0**, where the page says 3
(`mmc5.rs:127-136`).

**The others.** FCEUX powers up in mode 0 (`mmc5.cpp:1047`), Nestopia in mode 0
(`NstBoardMmc5.cpp:94`, `EXRAM_MODE_PPU_NT`), puNES zeroes its whole state on a hard reset and then
re-seeds $5100 to mode 3 and $5101 to mode 0 but not $5104 (`mapper_005.c:57-62`). Nintendulator zeroes the split registers explicitly and
leaves the mode at zero (`h_MMC5.cpp:188`).

**Verdict: OPEN.** The page gives a reset value and four of four implementations ignore it, so
either the page's figure is an artefact of how it was measured or every emulator has the same
harmless bug. Harmless is the operative word: mode 0 and mode 3 differ only in whether the CPU
can read $5C00-$5FFF and whether the PPU can see ExRAM, and every MMC5 game writes $5104 in its
reset code before using either.

**What would settle it.** A power-on read of $5C00 on hardware before any write to $5104 — or,
failing that, an authoritative edit to the page, since no emulator author appears to have
believed it.

**Test ROMs.** `assets-free/test-roms/nes-test-roms/mmc5test` and `mmc5test_v2`, and
`assets-free/test-roms/nes-test-roms/exram`. These are the only mapper-specific ROM tests in
the suite besides the MMC3's, and they are the ones that would exercise MMC5-1 and MMC5-3
directly: `exram` is about the $5C00-$5FFF modes and `mmc5test_v2` covers the split and the
scanline IRQ. **Neither is run by the project today.** Running `exram` would settle MMC5-3 at
first hand; running `mmc5test_v2` would quantify MMC5-1 rather than leave it as a feature
checklist. There is no audio test ROM for the MMC5, so MMC5-2 stays a documentation question.

Count: 16 OK, 1 DIFFERS, 1 OPEN.

---

## Mappers 9 and 10 — MMC2 (PxROM) and MMC4 (FxROM)

Pages: *MMC2* (<https://www.nesdev.org/wiki/MMC2>), *MMC4*
(<https://www.nesdev.org/wiki/MMC4>). Ours: `mappers/mmc2.rs` (one type for both).
Theirs: `fceux/src/boards/mmc2and4.cpp`, `nestopia/source/core/board/NstBoardMmc2.cpp` and
`NstBoardMmc4.cpp`, `punes/src/core/mappers/MMC2.c`, `mapper_009.c` and `mapper_010.c`,
`nintendulator-mappers/src/Hardware/h_MMC2.cpp`,
`nintendulator-mappers/src/iNES/mapper009.cpp` and `mapper010.cpp`.

### The register map and the PRG layout — OK

- **$A000 PRG bank (4 bits), $B000/$C000 the two CHR registers of the $0000 window,
  $D000/$E000 those of the $1000 window, $F000 bit 0 the mirroring** — OK
  (`mmc2.rs:139-158`). The order is the page's: $B000 is the register used when latch 0 = $FD,
  $C000 when it is $FE, and the same pair for latch 1. All four agree
  (`mmc2and4.cpp:58-64`, `NstBoardMmc2.cpp:114-119`, `MMC2.c:45-63`, `h_MMC2.cpp:82-86`).
- **$F000 bit 0: 1 = horizontal, 0 = vertical** — OK (`mmc2.rs:146-152`), the page's wording
  exactly; FCEUX inverts into its own enum and lands on the same answer
  (`mmc2and4.cpp:63`).
- **MMC2's PRG: 8 KiB switchable at $8000, the last three 8 KiB banks fixed** — OK
  (`mmc2.rs:37-40`), identical in all four (`mmc2and4.cpp:46-50`, `MMC2.c:108-112`,
  `mapper009.cpp:12`).
- **MMC4's PRG: 16 KiB switchable at $8000, the last 16 KiB fixed** — OK
  (`mmc2.rs:34-35`), same in all four (`mmc2and4.cpp:42-44`).
- **MMC4 has 8 KiB of fixed PRG-RAM; MMC2 has none except on the PlayChoice board** — OK in
  effect. FCEUX maps the RAM only for mapper 10 (`mmc2and4.cpp:95-99`) and Nintendulator only
  in `mapper010.cpp:15`; this build gives both boards the REQ-CORE-012 default, which is the
  same note as NROM — observable only by a program that reads $6000 on an MMC2 expecting open
  bus, and the only MMC2 game is *Mike Tyson's Punch-Out!!*.
- **The latch is consulted on the fetch and moved after it** — OK (`mmc2.rs:160-164`: the
  value is read first, then `move_latches` runs), so the register change takes effect from the
  next fetch. All four do the same: FCEUX, Nestopia and Nintendulator hook the PPU read and
  swap the bank after returning the byte (`mmc2and4.cpp:67-89`, `NstBoardMmc2.cpp:96-112`,
  `h_MMC2.cpp:95-110`), puNES hooks "after read CHR" (`MMC2.c:72-88`).
- **The CHR register field width** — OK, with the page wider than this build. The *MMC2* page
  gives the CHR registers as **bits 5-0**, six bits; this build masks to five
  (`mmc2.rs:142-145`), as does puNES (`mapper_009.c:41-43`). FCEUX and Nestopia store the
  whole byte and let the CHR size reduce it (`mmc2and4.cpp:59-62`,
  `NstBoardMmc2.cpp:117`). Equivalent on every real image: *Punch-Out!!* carries 128 KiB of
  CHR, thirty-two 4 KiB banks, so bit 5 addresses nothing.

### MMC2/4-1 — the MMC2's asymmetric latch decode: this build and one other get it right — OK

**Point.** The *MMC2* page is unusually specific, and the two windows are **not** symmetrical:

- latch 0 moves on a PPU read of **exactly $0FD8** or **exactly $0FE8**, and the page explains
  the consequence — it "only switches on the *top row* of the 8x8 tile";
- latch 1 moves on a read anywhere in **$1FD8-$1FDF** or **$1FE8-$1FEF**, and so "switches on
  *every row* of the 8x8 tile".

The MMC4 page gives both windows the eight-address ranges.

**This build.** Implements the asymmetry, explicitly and only for mapper 9: `move_latches`
tests the two exact addresses for the $0xxx window when the board is an MMC2 and the ranges
when it is an MMC4, and uses the ranges for the $1xxx window on both (`mmc2.rs:49-73`).

**The others.**

| | $0xxx window | $1xxx window | file:line |
| --- | --- | --- | --- |
| this build (mapper 9) | exactly $0FD8 / $0FE8 | $1FD8-$1FDF / $1FE8-$1FEF | `mmc2.rs:50-66` |
| this build (mapper 10) | $0FD8-$0FDF / $0FE8-$0FEF | the same | `mmc2.rs:51-55` |
| Nintendulator | exactly $0FD8 / $0FE8, with the comment "different from Read7!" | the eight-address range | `h_MMC2.cpp:95-110`, `h_MMC2.cpp:112-126` |
| Nestopia | the eight-address range | the eight-address range | `NstBoardMmc2.cpp:101-106` (`address & 0xFF8`) |
| FCEUX | **$0FD0-$0FDF / $0FE0-$0FEF** — the whole sixteen | the same | `mmc2and4.cpp:67-89` (`A & 0xF0`) |
| puNES | **$0FD0-$0FDF / $0FE0-$0FEF** | the same | `MMC2.c:72-88` (`address & 0xFFF0`) |

**Verdict: OK**, and this is the most precise point in the batch. Only this build and
Nintendulator reproduce the page's asymmetry — and Nintendulator's own comment shows its author
noticed the same thing. Nestopia widens the $0xxx window to eight addresses, and FCEUX and
puNES widen both windows to sixteen, which also makes mapper 9 and mapper 10 identical in those
two emulators. The difference is reachable: *Punch-Out!!* draws its background from the $0xxx
window, so a tile fetched at $0FD9-$0FDF would move the latch on FCEUX, puNES and Nestopia and
not here or on hardware. That this build is on the page's side is worth stating because it is a
detail the single Mesen comparison of MR-1 could not establish.

One extra path neither the page nor the others have: **puNES also moves the latch on a $2006
write** (`MMC2.c:89-91`, routing its `update_r2006` hook into the same decoder). Nothing
documents that, and no other implementation does it.

### MMC2/4-2 — the power-on latch value — OPEN

**Point.** Neither page states the power-on value of the two latches.

**This build.** Both latches select the **$FE** register (`mmc2.rs:27-28`, `mmc2.rs:133-135`).

**The others.** FCEUX powers both at $FE (`mmc2and4.cpp:93`); puNES powers both at **$FD**
(`MMC2.c:94-99`, `latch[0] = 0`, `latch[1] = 2`); Nintendulator powers both at **$FD**
(`h_MMC2.cpp:28-32`, `LatchState[i] = 0`); Nestopia powers both at **$FD**
(`NstBoardMmc2.cpp:36-43`, `selector[0] = 0`, `selector[1] = 2`).

**Verdict: OPEN**, three to two against this build, with nothing in either page to appeal to.
It is benign: *Punch-Out!!* and the MMC4 titles (*Fire Emblem*, *Famicom Wars*) write all four
CHR registers in their reset code, and until they do all four registers read 0 here anyway, so
the choice is invisible. Recorded only because three of the four chose the other value.

**What would settle it.** A power-on CHR read on hardware before any $B000-$E000 write. No
public test ROM does this; `holy-mapperel`'s `M9_P128K_C64K.nes` and the two `M10_*` images
exercise the latches but after initialising the registers.

**Test ROMs.** `assets-free/test-roms/holy-mapperel/testroms/M9_P128K_C64K.nes`,
`M10_P128K_C64K_W8K.nes` and `M10_P128K_C64K_S8K.nes` — the last two also covering the MMC4's
PRG-RAM in its work-RAM and battery forms. None is run by the project today. Running `M9` would
check MMC2/4-1 at first hand, since the driver reads known CHR addresses and reports the bank
it saw.

Count: 8 OK, 0 DIFFERS, 1 OPEN.

---

## Mapper 11 — Color Dreams

Pages: *INES Mapper 011* (<https://www.nesdev.org/wiki/INES_Mapper_011>), *Color Dreams*
(<https://www.nesdev.org/wiki/Color_Dreams>). Ours: `mappers/discrete.rs:421-469`.
Theirs: `fceux/src/boards/datalatch.cpp:219-228`,
`punes/src/core/mappers/mapper_011.c`, `nintendulator-mappers/src/iNES/mapper011.cpp`.
Nestopia has no mapper-11 board class in `source/core/board/`, so this board is a four-way
comparison.

- **The latch is `CCCC LLPP`: bits 1-0 the 32 KiB PRG bank, bits 7-4 the 8 KiB CHR bank, bits
  3-2 the CIC lockout defeat** — OK, and this is MR-11-1 confirmed from a second direction.
  This build masks PRG to two bits and CHR to four (`discrete.rs:437-438`); Nintendulator
  masks to exactly the same two fields (`mapper011.cpp:12-13`); FCEUX takes four PRG bits
  (`datalatch.cpp:222`) and puNES the whole byte (`mapper_011.c:55`), both relying on the
  128 KiB PRG size to reduce it, which it does. MR-1 withdrew its own finding here and the
  page's diagram is on this build's side; the two extra votes split the same way Mesen's did,
  for the same oversized-image reason.
- **The whole of $8000-$FFFF decodes** — OK (`discrete.rs:464`), all three the same.
- **Bus conflicts, unconditionally** — OK per the page ("All commercially-released games are
  subject to bus conflicts"), and this build and Mesen are the only two that implement them;
  see the shared table. The *Color Dreams* board page adds that only the *Free Fall*
  prototype carries conflict-prevention logic, which is why the condition has no submapper
  clause here.
- **Power-on latch 0** — OK (`discrete.rs:458-461`), all three the same.

Count: 4 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 13 — CPROM

Page: *CPROM* (<https://www.nesdev.org/wiki/CPROM>). Ours: `mappers/discrete.rs:360-419`.
Theirs: `fceux/src/boards/datalatch.cpp:234-244`,
`nestopia/source/core/board/NstBoardCxRom.cpp:79-85`, `punes/src/core/mappers/mapper_013.c`,
`nintendulator-mappers/src/iNES/mapper013.cpp`.

- **$0000-$0FFF fixed to CHR-RAM bank 0, $1000-$1FFF switched by bits 1-0, PRG not banked** —
  OK (`discrete.rs:383-387`, `discrete.rs:415`). Identical in all four
  (`datalatch.cpp:236-240`, `NstBoardCxRom.cpp:79-85`, `mapper_013.c:57-60`,
  `mapper013.cpp:12-14`).
- **16 KiB of CHR-RAM, "in two 8KiB RAMs"** — OK, forced regardless of the header
  (`discrete.rs:374`, `CpRom::CHR_RAM_SIZE`). This was MR-13-1 and MF-1 fixed it; the page's
  wording is unchanged.
- **The nametable arrangement is wired vertical** — OK, and this build is the only one that
  enforces it. The page's characteristics table says "Nametable mirroring: Vertical" with no
  solder pad, which is MR-13-2; `CpRom::reset` sets it (`discrete.rs:408-409`), while FCEUX,
  Nestopia, puNES and Nintendulator all leave the header to decide (`mapper013.cpp:17` calls
  `iNES_SetMirroring`). Equivalent on the one real dump, whose header says vertical, and
  better than the others on a mis-headered one.
- **Power-on latch 0** — OK (`discrete.rs:406-411`).

### 13-1 — the board has bus conflicts and this build does not implement them — DIFFERS

**Point.** The *CPROM* characteristics table carries the row **"Subject to bus conflicts:
Yes"**, with no submapper clause — the same unqualified row the *GxROM* page has, and which
`registry::has_bus_conflicts` already honours for mapper 66.

**This build.** `registry::has_bus_conflicts` has no arm for 13 (`registry.rs:1105-1159`), so
the latch stores whatever the CPU wrote.

**The others.** Nestopia maps the board through its bus-conflict-aware CHR swap helper
(`NstBoardCxRom.cpp:81`, `CHR_SWAP_4K_1_BC`) and puNES ANDs the written value with the PRG-ROM
byte at the same address before storing it (`mapper_013.c:46`). FCEUX passes `busc = 0`
(`datalatch.cpp:243`), Nintendulator passes `FALSE` (`mapper013.cpp:19`), and Mesen does not
implement it. Three against three, with the page on the other three's side.

**Verdict: DIFFERS** — against the page and against half the field. MR-1's MR-BUS-1 enumerated
the boards whose pages carry the row and this one was missed, which is understandable: mapper 13
was reviewed for its CHR-RAM size and its mirroring, not its write path.

**Which games it affects, and which are known affected.** One cartridge exists: *Videomation*,
and it is the only game the page lists. **Whether it is affected is not knowable from here** —
it depends on whether *Videomation* ever writes a CHR bank number that differs from the PRG-ROM
byte at the address it writes to. Games on conflict-prone boards normally write through a table
whose values match, which is exactly why the hardware tolerates the conflict, so the likeliest
answer is that nothing changes either way.

**Risk of a fix.** None measurable. `has_bus_conflicts` is evaluated once at load and the AND
happens on the board's write path, which for this board is reached by a single `STA` per bank
change. SMB3, Zelda II and the widescreen packs are not mapper 13. The *only* risk is the one
the fix is meant to produce: if *Videomation* does write a mismatched value, it would start
banking differently — which is why the fix should be verified against the game rather than
assumed.

**What a fix would look like.** One number added to the unconditional list at
`registry.rs:1156`, beside 11, 66, 70, 78 and the rest. The machinery is already there.

**Fixed in MF-LEG-1 (`4807030a`)**, exactly as described: `13` joins the unconditional list.
One existing test had to move with it — `req_core_100_cprom_fixes_the_first_4_kib_of_chr_ram`
selected its CHR banks by writing `$8000`, which over the fixture's zero-filled PRG bank 0 now
lands as 0 — and it is the only test in the suite the change touched.

- **No PRG-RAM** — the page says "PRG RAM: None" where this build gives the board the
  REQ-CORE-012 default 8 KiB. Same note as NROM and MMC2; FCEUX allocates none for this board
  either (`datalatch.cpp:243`, `wram = 0`). Not counted separately.

Count: 4 OK, 1 DIFFERS, 0 OPEN.

---

## Mapper 15 — the 100-in-1 Contra Function 16 multicart

Page: *INES Mapper 015* (<https://www.nesdev.org/wiki/INES_Mapper_015>). Ours:
`mappers/multicart.rs:116-208`. Theirs: `fceux/src/boards/15.cpp`,
`punes/src/core/mappers/mapper_015.c`, `nintendulator-mappers/src/iNES/mapper015.cpp`.
Nestopia has no mapper-15 class, so this is a four-way comparison.

MR-1b settled this board against the page and Q-110 holds what is left for the spec's owner.
The three new sources are unusually informative, because two of them carry comments explaining
choices.

- **The mode comes from address bits 1-0 and the value is the bank register** — OK
  (`multicart.rs:187-193`). Same in all three (`15.cpp:38`, `mapper_015.c:56`,
  `mapper015.cpp:11-38` via its `Mode` field).
- **`B` is bits 5-0, and bit 6 is the mirroring bit** — OK (`multicart.rs:132`,
  `multicart.rs:159`). FCEUX and puNES use the same two fields (`15.cpp:37`, `15.cpp:51`,
  `mapper_015.c:54`, `mapper_015.c:77-82`). This is AMD-021's reading and MR-1b confirmed it
  from the page; the two extra votes agree.
- **Mode 1's fixed window is `B | 7`, the last 16 KiB of the 128 KiB block** — OK, and now
  confirmed three times over. This is the point MR-1 called "the heaviest" and AMD-061 §2
  changed: FCEUX ORs 7 into the upper two 8 KiB windows in mode 1 and not in mode 3
  (`15.cpp:47-56`), puNES writes `bank | 0x07` literally (`mapper_015.c:61`), and
  Nintendulator spells the intent out by masking to the block first and then forcing the low
  three bits, `(BankReg & 0x38) | 0x07` (`mapper015.cpp:21`). Three independent
  implementations of exactly the expression this build now uses, and the one point where the
  four new sources overturn nothing and confirm an amendment.
- **Mode 2 maps the same 8 KiB everywhere, with bit 7 as PRG A13** — OK
  (`multicart.rs:147-151`); FCEUX (`15.cpp:43-46`) and puNES (`mapper_015.c:65-70`) both
  compute `(B << 1) | (value >> 7)` and repeat it across all four windows. The page's "PRG
  A13 if SS=2, ignored otherwise" is implemented by all three the same way.
- **Mode 3 maps the same 16 KiB at $8000 and $C000** — OK (`multicart.rs:152-155`); same in
  both (`15.cpp:48-56` with the OR suppressed, `mapper_015.c:71-74`).
- **Mode 0 is two consecutive 16 KiB banks, `B` and `B+1`** — OK, two to two. This build
  (`multicart.rs:135-138`) and FCEUX (`15.cpp:39-42`, `(B << 1) + i` in 8 KiB units) keep
  bit 0 of the field, so a 32 KiB window can sit on an odd 16 KiB boundary; puNES maps a
  32 KiB bank of `B >> 1` (`mapper_015.c:57-59`) and Nintendulator `(B & 0x3E) >> 1`
  (`mapper015.cpp:17`), both of which discard that bit. The page's "bits 5-0 are PRG
  A19..A14" makes `B` a 16 KiB index, so this build's reading is the page's; the two aligned
  readings only differ on an image whose menu puts a 32 KiB game at an odd 16 KiB offset.
- **Mirroring: bit 6 set = horizontal** — OK (`multicart.rs:159`); FCEUX inverts into its own
  enum and lands on the same answer (`15.cpp:37`), puNES states it directly
  (`mapper_015.c:78-82`), Nintendulator drives CIRAM A10 from PPU A11 when the bit is set
  (`mapper015.cpp:35-37`).

### 15-1 — the CHR-RAM write protection: two emulators disabled it on purpose — DIFFERS

**Point.** The page: "CHR-RAM is write-protected in PRG banking modes 0 and 3, and
write-enabled in modes 1 and 2."

**This build.** Protects modes 0 and 3 (`multicart.rs:195-207`). This is MR-15-1 and MF-1
(969c1b2e) implemented it.

**The others.**

| | protects CHR-RAM in | file:line |
| --- | --- | --- |
| this build | modes 0 and 3 | `multicart.rs:199-202` |
| Mesen 2 | modes 0 and 3 | MR-1b |
| FCEUX | **mode 3 only**, deliberately | `15.cpp:64-70` |
| puNES | **never** | `mapper_015.c` has no CHR write path at all |
| Nintendulator | **never**, deliberately | `mapper015.cpp:34` |

The two deliberate choices come with reasons written in the source, and they are the point of
this entry. FCEUX's comment says there appear to be **two different mapper-15 boards** — "the
one for 110in1 or 168in1 carts with complex multi game features" and "a simplified version for
subor/waixing chinese originals and hacks with no different modes, working only in mode 0 and
which does not expect there is any CHR write protection" — and that "protecting CHR writes only
for mode 3 fixes the problem, all roms may be run on the same source again"
(`15.cpp:64-70`). Nintendulator's is shorter: "For compatibility with mapper hacks, do not
write-protect" (`mapper015.cpp:34`).

**Verdict: DIFFERS** — from the majority of emulators, not from the page. This build and Mesen
follow the page; FCEUX follows it halfway for a named compatibility reason and puNES and
Nintendulator decline entirely.

**Which games it affects, and which are known affected.** Mapper-15 images that write CHR-RAM
while in mode 0. FCEUX's comment says these exist and names the family — the Subor and Waixing
Chinese originals and their hacks, which run only in mode 0 — but not an individual title, so
**the affected set is known by family and not by name**. The 110-in-1 and 168-in-1 multicarts
the page describes are the ones the protection is for. None of this touches a protected game:
mapper 15 is not SMB3, not Zelda II and not in the widescreen packs.

**Risk of a fix.** This is one to think about before touching. Relaxing the protection to
mode 3 only would match FCEUX and make the mode-0-only hacks work, at the cost of the page's
stated behaviour and of Mesen's agreement with it; keeping it as it is risks a blank or
corrupt screen on exactly the images FCEUX describes. The cost either way is one condition in
`ppu_write` (`multicart.rs:200`), so this is a correctness judgement and not an engineering
one.

**What would make the decision.** A mapper-15 Subor or Waixing image run on this build: if the
screen is blank or the tiles are wrong where FCEUX's is right, the protection is too strict for
that family and the mode-3-only reading is the better default. That is a concrete, cheap
experiment and it is the one thing this review cannot do from the sources.

Count: 7 OK, 1 DIFFERS, 0 OPEN.

---

## Mapper 16 — Bandai FCG (FCG-1/2 and LZ93D50)

Page: *INES Mapper 016* (<https://www.nesdev.org/wiki/INES_Mapper_016>). Ours:
`mappers/bandai_fcg.rs`. Theirs: `fceux/src/boards/bandai.cpp`,
`nestopia/source/core/board/NstBoardBandaiLz93d50.cpp`,
`punes/src/core/mappers/LZ93D50.c`, `nintendulator-mappers/src/iNES/mapper016.cpp`.

- **$x000-$x007 the eight 1 KiB CHR banks, $x008 the 16 KiB PRG bank, $x009 the mirroring,
  $x00A the IRQ control, $x00B/$x00C the IRQ value, $x00D the EEPROM port** — OK
  (`bandai_fcg.rs:338-388`). All four decode the same sixteen registers
  (`bandai.cpp:257-277`, `NstBoardBandaiLz93d50.cpp:66-80`, `LZ93D50.c:95-130`,
  `mapper016.cpp:370-382`).
- **The register window is $8000-$800F on the LZ93D50 and $6000-$600F on the FCG-1/2** — OK,
  and submapper-aware: `register_floor` chooses between them at load
  (`bandai_fcg.rs:138-160`). FCEUX decodes $6000-$FFFF for both (`bandai.cpp:309`), which is a
  superset; the page splits them by submapper as this build does.
- **$x009 mirroring: 0 vertical, 1 horizontal, 2 one-screen A, 3 one-screen B** — OK
  (`bandai_fcg.rs:356-361`), the page's order, same in all four.
- **The IRQ counter counts down on every M2 cycle while enabled** — OK
  (`bandai_fcg.rs:420-430`).
- **$x00A acknowledges the IRQ, sets the enable from bit 0, and copies the latch into the
  counter on an LZ93D50** — OK (`bandai_fcg.rs:362-372`). FCEUX
  (`bandai.cpp:275`), Nestopia (`NstBoardBandaiLz93d50.cpp:182-188`) and puNES
  (`LZ93D50.c:109-117`) all copy a latch; **Nintendulator has no latch at all** and lets
  $x00B/$x00C write the counter directly on every mapper-16 image (`mapper016.cpp:379-380`),
  which is the FCG-1/2 behaviour applied unconditionally.
- **MR-16-1: submapper 4 writes the counter directly** — OK and now a sharper result than MR-1
  could reach. This build is the only one of the five that keys the latch on the submapper
  (`bandai_fcg.rs:109`, `bandai_fcg.rs:166-177`); FCEUX, Nestopia and puNES always latch and
  Nintendulator never does. MF-1's fix is right and is the page's reading.
- **The IRQ fires when the counter holds zero** — OK, and this build and Nestopia are
  expression-for-expression identical: this build decrements then tests `< 0`
  (`bandai_fcg.rs:425-429`) and Nestopia returns `(count-- & 0xFFFF) == 0`
  (`NstBoardBandaiLz93d50.cpp:202-205`); both therefore fire on the clock whose pre-decrement
  value was zero.

### 16-1 — whether the counter keeps counting after the IRQ — OPEN

**Point.** The page says the counter "decreases on every M2 cycle" and that an IRQ is generated
when it "holds a value of zero". It does **not** say what happens next.

**This build.** The counter wraps to $FFFF and keeps counting, so the IRQ repeats every 65 536
cycles until the game writes $x00A (`bandai_fcg.rs:427-428`).

**The others.** Nestopia wraps and keeps counting (`NstBoardBandaiLz93d50.cpp:202-205`), as
does Nintendulator (`mapper016.cpp:352-356`). FCEUX stops: it fires once, sets the counter to
−1 and **clears its own enable flag** (`bandai.cpp:289-296`). puNES stops too, by a different
route — its condition is `count && !(--count)`, so once the counter reaches zero it never
decrements again (`LZ93D50.c:155-162`).

**Verdict: OPEN**, three to two in this build's favour but with nothing in the page to settle
it. The practical difference is a spurious second IRQ 65 536 CPU cycles (about 37 ms, two
frames) after the first, on a game that enables the counter and then never writes $x00A again.
Games that use the IRQ for a raster effect rewrite $x00A every frame and cannot see it; a game
that uses it as a one-shot timer could.

**What would settle it.** A hardware capture of the /IRQ line on *Famicom Jump II* — the title
FCEUX singles out in its save-state comment as needing the latch — left running past one wrap.
No public test ROM covers mapper 16.

### 16-2 — the "IRQ immediately" case of $x00A — OK

**Point.** The page: "If a write to this register enables counting while the counter is holding
a value of zero, an IRQ is generated immediately."

**This build.** Not modelled as a special case, and it does not need to be: `$x00A` loads the
counter from the latch and the next `on_cpu_cycle` finds it at zero and fires
(`bandai_fcg.rs:369`, `bandai_fcg.rs:425-429`), so the IRQ arrives one CPU cycle later than
"immediately".

**The others.** puNES models it explicitly, scheduling the IRQ with a one-cycle delay when
$x00A enables counting with a zero reload (`LZ93D50.c:112-114`). FCEUX, Nestopia and
Nintendulator do not model it at all.

**Verdict: OK.** One CPU cycle, and puNES's own implementation of the special case also inserts
a one-cycle delay — so the two agree on the number even though only one of them wrote the rule
down.

**Test ROMs.** None for mapper 16 in `assets-free`.

Count: 7 OK, 0 DIFFERS, 1 OPEN.

---

## Mapper 18 — Jaleco SS88006

Page: *INES Mapper 018* (<https://www.nesdev.org/wiki/INES_Mapper_018>). Ours:
`mappers/jaleco_ss88006.rs`. Theirs: `fceux/src/boards/18.cpp`,
`nestopia/source/core/board/NstBoardJalecoSs88006.cpp`,
`punes/src/core/mappers/mapper_018.c`, `nintendulator-mappers/src/iNES/mapper018.cpp`.

- **Every bank register is a nibble pair, low nibble at the even address and high at the odd
  one** — OK (`jaleco_ss88006.rs:20-26`, `jaleco_ss88006.rs:134-154`): three PRG banks over
  $8000-$9001 and eight 1 KiB CHR banks over $A000-$D003, with $E000-$FFFF fixed to the last
  bank. Identical in all four (`18.cpp:41-56`, `NstBoardJalecoSs88006.cpp:216-219`,
  `mapper_018.c:70-78`, `mapper018.cpp:100-140`).
- **$E000-$E003 are the four nibbles of the IRQ reload value, least significant first** — OK
  (`jaleco_ss88006.rs:156-159`); same in all four (`18.cpp:59-62`, `mapper018.cpp:142-145`).
- **$F000 reloads the counter and acknowledges; $F001 sets the enable and the width and
  acknowledges** — OK (`jaleco_ss88006.rs:42-60`). FCEUX acknowledges on $F001 only
  (`18.cpp:63-64`); Nintendulator acknowledges on $F000 (`mapper018.cpp:152-153`); this build
  does both, which is the page.
- **The width bits: bit 3 gives 4 bits, bit 2 gives 8, bit 1 gives 12, none gives 16, "F
  overrides E overrides T"** — OK, and a four-way agreement on the priority order:
  `jaleco_ss88006.rs:51-59`, `NstBoardJalecoSs88006.cpp:266-269`, `mapper_018.c:166-174`,
  `mapper018.cpp:33-39`. **FCEUX has no width modes at all** — its counter is always 16 bits
  (`18.cpp:98-105`), which makes it the only one of the five that cannot run a game using the
  4-bit mode as a scanline timer.
- **$F002 mirroring: 0 horizontal, 1 vertical, 2 one-screen A, 3 one-screen B** — OK
  (`jaleco_ss88006.rs:61-66`), the page's order; FCEUX stores the same two bits
  (`18.cpp:65`).
- **$F003, the µPD7755C ADPCM sound** — not implemented, and REQ-CORE-075 leaves it optional.
  None of the four implements it either (FCEUX's `18.cpp` has no $F003 arm), so there is
  nothing to compare. MR-18-1 already recorded this.

### 18-1 — the IRQ fires on the wrap, and three of the five fire a cycle early — OK

**Point.** The page's worked example is the whole argument: "if the counter is in 4-bit mode and
contains $1232, it counts down to $1230, then **wraps to $123F where an IRQ is triggered**".
Two things follow — the IRQ is on the wrap and not on the transition to zero, and the wrap
affects only the counted field, leaving the bits above it alone.

**This build.** Exactly that: the clock that finds the counted field at zero raises the IRQ and
sets that field to its mask, leaving the high bits (`jaleco_ss88006.rs:169-181`).

**The others.** Nestopia (`NstBoardJalecoSs88006.cpp:286-289`) and puNES
(`mapper_018.c:175`) both compute `(count & mask) && !(--count & mask)` — they fire one cycle
earlier, when the field *leaves* 1, and they decrement the whole 16-bit counter so the wrap
carries into the high bits. Mesen does the same (MR-18-1). Nintendulator fires on the same
cycle as this build — `if (!(IRQcounter & IRQmask)) SetIRQ(0)` before the decrement
(`mapper018.cpp:64-68`) — but then decrements the whole counter too, so its $1230 becomes
$122F and not $123F. FCEUX has no field at all.

**Verdict: OK**, and it strengthens MF-2's closure of MR-18-1: of the six implementations, this
build is the only one that reproduces both halves of the page's example — the firing cycle *and*
the field-only wrap. Nintendulator gets the cycle, Nestopia, puNES and Mesen get neither.
Audible/visible consequence: one CPU cycle of IRQ phase, and, on a game that reloads a narrow
width and reads the counter's effect over many wraps, a drift the others accumulate and this
build does not.

### 18-2 — $9002, the PRG-RAM enable and write protect, is not implemented — DIFFERS

**Point.** The page gives $9002 as `.... ..WR`: bit 0 is the PRG-RAM chip enable and bit 1
allows writes — the same pair of functions the MMC3's $A001 carries, and the page says so
("same as MMC3").

**This build.** `write_register`'s $9000-group arm handles only indices 0 and 1, the high PRG
bank's two nibbles, and falls through for $9002 and $9003
(`jaleco_ss88006.rs:140-145`). `JalecoSs88006` does not override `prg_ram_enabled` or
`prg_ram_writable`, so it takes the trait defaults — always enabled, always writable.

**The others.**

| | $9002 | file:line |
| --- | --- | --- |
| this build | ignored | `jaleco_ss88006.rs:140-145` |
| Nestopia | implemented: bit 0 the read permission, bit 1 the write permission, and the board powers up with **both denied** | `NstBoardJalecoSs88006.cpp:90`, `:201-205`, `:77` |
| puNES | implemented: read from bit 0, write from read-and-bit-1 | `mapper_018.c:197-200` |
| FCEUX | ignored — $6000-$7FFF is mapped as plain readable/writable RAM and no handler decodes $9002 | `18.cpp:91`, `18.cpp:92` |
| Nintendulator | ignored — `SetPRG_RAM8(0x6, 0)` unconditionally | `mapper018.cpp:19` |
| Mesen 2 | implemented (MR-1 raised no finding against it) | — |

**Verdict: DIFFERS** — against the page and against three of the five. Nestopia's choice is the
telling one: it powers the board up with the RAM **unreadable and unwritable**, which only works
if every mapper-18 game writes $9002 before touching its save — so the games do write it, and
the register is live rather than vestigial.

**Which games it can affect, and which are known affected.** The battery-backed mapper-18
titles — *Pizza Pop!*, the *Moero!! Pro Yakyuu* family, *Shin Moero!! Pro Yakyuu* — plus any
mapper-18 game that uses $6000-$7FFF as work RAM. **No game is known to be affected**: the
failure mode needs a game that *relies* on the protection, i.e. that writes to $6000-$7FFF
while the chip is disabled and expects the write to be discarded. That is the shape of a
save-protection routine run during a crash or a reset, which is exactly what the MMC3's
equivalent bit exists for and exactly what no emulator author has reported a symptom from.
Reading is the safer half: with the chip disabled the window should be open bus here and
answers RAM.

**Risk of a fix.** None for the protected games — SMB3, Zelda II and the widescreen packs are
not mapper 18. The real risk is the one Nestopia's power-on value hints at: if the two
permission bits are implemented *and* the board is powered up denied, a game that happens not to
write $9002 loses its save RAM entirely. The safe fix is to implement the bits but power up
**permitted**, which is what puNES does and what this build's MMC3 already does for the same
reason (MR-4-5).

**What a fix would look like.** One more arm in the $9000 group of `write_register`
(`jaleco_ss88006.rs:140-145`) storing the byte, plus `prg_ram_enabled` and `prg_ram_writable`
overrides reading its two bits — the same three-line shape `Mmc3Board` has
(`mmc3.rs:1146-1160`). Both are load-free and off every hot path.

**Fixed in MF-LEG-1 (`d172e5a6`)**, in exactly that shape, with the power-on value this entry
asks for: `$03`, both permitted. `$9003` is left undecoded, the page listing no register there.
Q-175 records the two choices the page does not settle — the power-on value, where Nestopia
comes up denied — and that the two bits stay **out of the savestate**, a byte at the end of this
board's block leaving every older mapper-18 state one short of it.

**Test ROMs.** None for mapper 18 in `assets-free`.

Count: 7 OK, 1 DIFFERS, 0 OPEN.

---

## Mappers 21, 22, 23, 25 and 27 — Konami VRC2 and VRC4

Page: *VRC2 and VRC4* (<https://www.nesdev.org/wiki/VRC2_and_VRC4>), *NES 2.0 submappers*.
Ours: `mappers/vrc2_vrc4.rs`, `mappers/vrc_irq.rs`. Theirs:
`fceux/src/boards/vrc2and4.cpp`, `nestopia/source/core/board/NstBoardKonamiVrc2.cpp` and
`NstBoardKonamiVrc4.cpp`, `punes/src/core/mappers/VRC2and4.c` and `mapper_021.c` …
`mapper_027.c`, `nintendulator-mappers/src/iNES/mapper021.cpp` … `mapper027.cpp`.

MR-21-1, MR-21-2 and MR-21-3 were all fixed in MF-1, so the variant table, the VRC2's missing
swap mode and mapper 27 are settled. Re-verified against the page, plus one thing MR-1 did not
look for.

### The variant table and the register map — OK

- **Nine wirings, resolved from the mapper number and the NES 2.0 submapper, with an OR'd
  fallback when the submapper is absent** — OK (`vrc2_vrc4.rs:34-54`,
  `vrc2_vrc4.rs:58-68`). Every address-line pair matches the page's table, including VRC4c's
  A6/A7 — the one pair no OR of A0-A3 can reach, which is why the fallback cannot cover it.
  Nestopia parameterises the same two lines (`NstBoardKonamiVrc2.cpp:83-85`,
  `prgLineA`/`prgLineB`), puNES and Nintendulator each have a per-number file that fixes them.
- **Mapper 22 is always VRC2a and mapper 27 always the pirate board** — OK
  (`vrc2_vrc4.rs:36-37`). FCEUX agrees for 22, with the reason in a comment: "no IRQ (all
  mapper 22 games are VRC2)" (`vrc2and4.cpp:208`).
- **VRC2a ignores the low bit of the CHR bank number (the value is right-shifted by one)** —
  OK (`vrc2_vrc4.rs:180`, `vrc2_vrc4.rs:318-322`), the page's own wording.
- **$8000 and $A000 are the two switchable 8 KiB PRG banks, five bits each; $C000-$DFFF is the
  second-to-last bank and $E000-$FFFF the last** — OK (`vrc2_vrc4.rs:306-316`,
  `vrc2_vrc4.rs:385`, `vrc2_vrc4.rs:422`). Same in all four.
- **$9002 bit 1 is the PRG swap mode, and it moves the switchable bank to $C000 while $8000
  becomes the second-to-last** — OK (`vrc2_vrc4.rs:412`, `vrc2_vrc4.rs:307-313`), the page's
  "when M is set" case.
- **$9000-$9001 mirroring: 0 vertical, 1 horizontal, 2 one-screen lower, 3 one-screen upper on
  a VRC4; one bit only on a VRC2** — OK (`vrc2_vrc4.rs:394-410`). This is MR-21-2 and the page
  confirms both halves; Nestopia uses its one-bit `NMT_SWAP_HV` helper across the whole of
  $9000-$9FFF for the VRC2 (`NstBoardKonamiVrc2.cpp:80`), which is the same thing.
- **$B000-$E003: each CHR bank is a nibble pair, low four bits at the even address and the high
  bits at the odd one** — OK (`vrc2_vrc4.rs:425-436`).
- **The high half of a VRC2 CHR register is four bits, not five** — OK in effect. The page says
  the VRC2 "only has 4 high bits of CHR select"; this build uses a five-bit mask for every
  variant but mapper 27 (`vrc2_vrc4.rs:183`). A VRC2b/c tops out at 256 KiB of CHR, 256 banks
  of 1 KiB, so the ninth bit is folded away by the bank mask. Equivalent on every real image.
- **$F000-$F003 are the VRC4 IRQ, and a VRC2 has none** — OK (`vrc2_vrc4.rs:437-440`,
  `vrc2_vrc4.rs:275-280`), shared with the VRC6 through `VrcIrq`. FCEUX installs no IRQ hook at
  all for mapper 22 (`vrc2and4.cpp:208-210`).

### VRC2-1 — the VRC2's single-bit latch at $6000-$6FFF is not implemented — DIFFERS

**Point.** The page describes a feature that is not a bank register at all: "The VRC2 was
supposed to have shipped with a Microwire interface for save games. However, Konami never used
it", and the consequence — games "rely on the two Data pins being connected to each other, and
so expect to be able to read the written value back". A write to $6000-$6FFF latches one bit
and a read returns it, the rest of the byte being open bus. The page names the games that need
it: **Contra (J)** and **Ganbare Goemon 2 (J)**.

**This build.** No latch. `Vrc2Vrc4` has no $6000 path at all, and `try_read_prg` answers
`None` below $8000 (`vrc2_vrc4.rs:369-372` refuses writes below $8000 as well). What happens
instead depends entirely on the header:

- an **iNES 1.0** dump gets REQ-CORE-012's default 8 KiB of PRG-RAM, so a write to $6000 is
  stored in RAM and read back — the whole byte rather than one bit, which satisfies the games'
  check **by accident**;
- an **NES 2.0** dump that correctly declares **no** PRG-RAM gets none, so $6000 reads open
  bus and the written value is lost.

**The others.** The two that model it both gate it on exactly that condition:

| | $6000-$6FFF on a VRC2 | file:line |
| --- | --- | --- |
| this build | PRG-RAM if the header gives any, otherwise open bus | `vrc2_vrc4.rs:369-372` |
| Nestopia | **the one-bit latch, installed only when the board has no WRAM** | `NstBoardKonamiVrc2.cpp:74-76` (`if (!board.GetWram())`) |
| puNES | **the one-bit latch, installed only when the image is NES 2.0 and declares no WRAM**; a read returns the latched bit in D0 with the rest of the byte open bus | `VRC2and4.c:51-56`, `VRC2and4.c:153-158` |
| FCEUX | PRG-RAM, unconditionally allocated | `vrc2and4.cpp:129-130` |
| Nintendulator | PRG-RAM, unconditionally mapped | `mapper022.cpp:15` |
| Mesen 2 | not raised by MR-1 | — |

**Verdict: DIFFERS** — from the page, and from the two implementations that read it carefully.
This build is in FCEUX's and Nintendulator's position: the games work because there happens to
be RAM where the latch should be.

**Which games it affects, and which are known affected.** The page names them: **Contra (J)**
and **Ganbare Goemon 2 (J)**, both mapper 23 or 25 VRC2 titles and both well known. They are
**not** affected today, because every common dump of them is iNES 1.0 and gets the default
PRG-RAM. They become affected the moment someone loads a properly headered NES 2.0 dump that
declares no PRG-RAM — which is what a modern, correct dump of a VRC2 cartridge *is*, since the
cartridge has no RAM chip. The failure would be a hang or a reset loop at boot, because the
check is a protection check.

**Risk of a fix.** None for any protected game — SMB3, Zelda II and the widescreen packs are
not VRC2. The condition is narrow by construction: install the latch only when the board has no
PRG-RAM, which is both Nestopia's and puNES's gate, so an iNES 1.0 dump keeps the RAM it has
today and nothing changes for it.

**What a fix would look like.** One `u8` field on `Vrc2Vrc4`, set only when the variant is a
VRC2 and `memory.prg_ram()` is empty; `intercept_cpu_write` latches bit 0 (or whichever bit the
page's diagram names) on a write in $6000-$6FFF, and `try_read_prg` answers it there. Both are
off every hot path — the board already has an `intercept_cpu_write`-shaped hook for mapper 14's
mirror guard.

**Fixed in MF-LEG-1 (`75d34118`)**, behind `board-vrc2-latch`, in that shape with two
differences. The gate is the **header's** declaration and not `memory.prg_ram()`: byte 10 of an
NES 2.0 header, because `nes20_prg_ram_size` floors at 8 KiB so the buffer is never empty on its
own — `registry::board_memory_sizes` is what then gives such a board no PRG-RAM, the latch
replacing the chip rather than sitting beside it, which is also what makes `$7000-$7FFF` the open
bus the page requires. And the write is taken in `write_register`'s existing `address < 0x8000`
arm rather than in `intercept_cpu_write`, so the board gains no dispatch body. Q-174 holds two
choices this entry does not raise: the top seven bits read 0 where the page says open bus, the
only hook that carries the bus byte to a board being behind the parked `board-143-open-bus`; and
the bit is outside the savestate, a byte written only when the latch exists leaving every older
state of the same ROM one short.

**Test ROMs.** `assets-free/test-roms/nes-test-roms/m22chrbankingtest` — a VRC2a CHR banking
test, which would exercise the VRC2a right-shift of `vrc2_vrc4.rs:318-322` directly. **Not run
by the project today**, and it is one of only four mapper-specific ROM tests in the suite (with
`mmc3_test`, `mmc5test` and `exram`). Nothing tests the $6000 latch.

Count: 10 OK, 1 DIFFERS, 0 OPEN.

---

## Mapper 28 — Action 53

Page: *INES Mapper 028* (<https://www.nesdev.org/wiki/INES_Mapper_028>). Ours:
`mappers/multicart.rs:470-607`. Theirs: `fceux/src/boards/28.cpp`,
`nestopia/source/core/board/NstBoardAction53.cpp`, `punes/src/core/mappers/mapper_028.c`,
`nintendulator-mappers/src/iNES/mapper028.cpp` — all four have the board.

MR-1b settled all three of its points against the page and all four confirm each of them, which
matters because one of the three was a disagreement with Mesen.

- **The register select comes from a write to $5000-$5FFF, bits 7 and 0** — OK
  (`multicart.rs:579-581`). FCEUX stores `value & 0x81` and switches on the four combinations
  (`28.cpp:132`, `28.cpp:141-158`); Nestopia maps $5000-$5FFF to its own poke
  (`NstBoardAction53.cpp:36`); Nintendulator latches the whole byte from $5xxx and switches on
  `regSel & 0x81` (`mapper028.cpp:67-81`); puNES keeps the same index (`mapper_028.c:56`).
- **The four registers: CHR bank, inner PRG (4 bits), mode (6 bits), outer PRG (6 bits)** — OK
  (`multicart.rs:592-602`); same masks in both (`28.cpp:152`, `28.cpp:157`;
  `NstBoardAction53.cpp:107-117`, where the mode register is masked to `0x3c` because Nestopia
  keeps the mirroring bits elsewhere).
- **The mirroring bit is captured *inside* the mode register and only while the mode is
  one-screen** — OK, and **FCEUX implements it line for line**: `if ((mode & 2) == 0) { mode
  &= 0xfe; mode |= value >> 4 & 1; }` (`28.cpp:45-49`) is this build's `take_mirroring_bit`
  (`multicart.rs:491-495`). So do the other two, with the "only from registers $00 and $01"
  clause spelled out: Nintendulator `if (!(regSel & 0x80) && !(regVal[2] & 0x2))`
  (`mapper028.cpp:88-89`) and puNES `if (!(index & 0x02) && !(reg[2] & 0x02))`
  (`mapper_028.c:57-59`). This was MR-1b's answer to Q-110 question 3 — AMD-021 R2-001 right,
  Mesen's separate always-updated bit wrong — and it is now a **five-way agreement against
  Mesen**, the strongest single result in this lot.
- **Mirroring values 0 one-screen A, 1 one-screen B, 2 vertical, 3 horizontal** — OK
  (`multicart.rs:499-504`); identical in FCEUX (`28.cpp:34-39`), puNES
  (`mapper_028.c:79-93`) and Nintendulator (`mapper028.cpp:14-21`).
- **The size mask from mode bits 5-4, and the four PRG layouts** — OK
  (`multicart.rs:509-526`). Nestopia writes the same algebra out as sixteen explicit cases
  (`NstBoardAction53.cpp:131-170`) and so does Nintendulator (`mapper028.cpp:23-50`); the
  expressions differ and the banks do not, because the outer bank is always even.
- **Power-on: outer = 63, inner = 15, mode = 0, so the last 16 KiB is at $C000** — OK
  (`multicart.rs:480-489`), and now a **three-way agreement** rather than MR-1b's "both
  conforming": Nestopia seeds exactly `preg[1] = 0xf`, `preg[3] = 0x3f`
  (`NstBoardAction53.cpp:41-44`) and FCEUX seeds `outer = 63` (`28.cpp:167`). MR-28-2 can be
  upgraded from "equivalent" to "the layout the other implementations also chose".
- **The mapper state survives a reset** — OK, implemented under AMD-061 §3/§5 after Q-112
  (`multicart.rs:558-566`), which is the page's "The mapper state is unchanged on reset".
  None of the four models a reset distinct from a power-on, so there is no vote.

**Test ROMs.** `holy-mapperel/testroms/M28_P512K.nes` and `M28_P512K_CR32K.nes`. Not run today.

Count: 7 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 30 — UNROM 512

Page: *UNROM 512* (<https://www.nesdev.org/wiki/UNROM_512>). Ours:
`mappers/discrete.rs:546-848`, `cartridges/flash.rs`. Theirs:
`fceux/src/boards/unrom512.cpp`, `nestopia/source/core/board/NstBoardUxRom.cpp:46-80`,
`punes/src/core/mappers/mapper_030.c`, `nintendulator-mappers/src/iNES/mapper030.cpp`.

MR-30-1 through MR-30-6 were all raised by MR-1 and five of the six fixed in MF-1. The four new
sources mostly confirm, with one disagreement worth recording.

- **The register is `NCCP PPPP`: five PRG bits, two CHR bits, one nametable bit** — OK
  (`discrete.rs:759` masks the CHR field to two bits). Nestopia uses the same two bits
  (`NstBoardUxRom.cpp:76`, `(data >> 5) & 0x3`) and so does Nintendulator
  (`mapper030.cpp:15`, `(Data & 0x60) >> 5`). **FCEUX uses three** (`unrom512.cpp:60`,
  `latche >> 5`), which on a 32 KiB CHR-RAM board reaches a bank that does not exist.
  This build is with the page and with two of the three.
- **Where the register lives depends on whether the board is self-flashable** — OK, and this is
  Q-117: $8000-$FFFF on a bus-conflicting board and $C000-$FFFF with $8000-$BFFF as the flash
  window otherwise (`discrete.rs:569-580`). FCEUX routes writes through its flash state machine
  (`unrom512.cpp:70-77`) and Nintendulator refuses them with a diagnostic
  (`mapper030.cpp:24-28`, "ERROR: game attempted to write to Flash ROM!"), so of the four only
  FCEUX and puNES emulate the flash at all (`mapper_030.c:113-117`). This build does
  (`cartridges/flash.rs`), which puts it in the better half.
- **The four mirroring modes of header byte 6, read as bits 3 and 0 together** — OK
  (`discrete.rs:546-564`, MR-30-1, fixed in MF-1). Nestopia reads only the two-value
  arrangement (`NstBoardUxRom.cpp:53-57`); FCEUX applies one-screen unconditionally
  (`unrom512.cpp:61`, `MI_0 + ((latche >> 7) & 1)`) despite its own header comment saying
  mirroring mode 2 is what enables it (`unrom512.cpp:27`). This build is the only one of the
  four that implements all four modes, which is what the page asks for.
- **Submapper 4's LED register at $8000-$BFFF** — OK (`discrete.rs:577-580`, MR-30-5). None of
  the four implements it.
- **No work RAM** — OK (MR-30-2, fixed in MF-1).

### 30-1 — which submapper gives the mapper control of horizontal/vertical — OPEN

**Point.** MR-30-3 cited *UNROM 512* for "submapper 3: No bus conflicts, mapper-controlled
horizontal/vertical nametable arrangement", and MF-1 implemented it on **submapper 3**
(`discrete.rs:620`).

**The others.** puNES implements the same behaviour on **submapper 1**
(`mapper_030.c:129-133`: for submapper 1, register bit 7 picks vertical or horizontal; for
everything else it falls into the byte-6 decode). FCEUX, Nestopia and Nintendulator do not
implement a mapper-controlled H/V mode at all.

**Verdict: OPEN**, one against one with three abstentions. MR-30-3 quotes the page and MF-1
followed it, so the likeliest reading is that puNES has the submapper number wrong; but with
only two implementations and one of them disagreeing on the *number* rather than the
*behaviour*, the honest verdict is open.

**What would settle it.** Re-reading the *UNROM 512* submapper list — specifically whether
submapper 1 is "no bus conflicts" alone and submapper 3 "no bus conflicts plus
mapper-controlled H/V". A correctly headered homebrew image using the mode would settle it in
practice; the affected set is homebrew only, nothing commercial, and nothing protected.

Count: 5 OK, 0 DIFFERS, 1 OPEN.

---

## Mapper 32 — Irem G-101

Page: *INES Mapper 032* (<https://www.nesdev.org/wiki/INES_Mapper_032>). Ours:
`mappers/iremg101.rs`. Theirs: `fceux/src/boards/32.cpp`,
`nestopia/source/core/board/NstBoardIremG101.cpp`, `punes/src/core/mappers/mapper_032.c`,
`nintendulator-mappers/src/iNES/mapper032.cpp`.

- **$8000 and $A000 the two switchable 8 KiB PRG banks, $9000 bit 1 the swap mode, bit 0 the
  mirroring, $B000-$B007 the eight 1 KiB CHR banks** — OK (`iremg101.rs:34-48`,
  `iremg101.rs:85-101`). All four the same, and all four implement the swap by XOR-ing the
  window address or by selecting between the two (`32.cpp:38-43`,
  `NstBoardIremG101.cpp:97-98`, `mapper_032.c:88-92`, `mapper032.cpp:16-19`).
- **Mirroring: bit 0 set = horizontal** — OK (`iremg101.rs:97-101`); FCEUX inverts into its own
  enum to the same effect (`32.cpp:38`), Nestopia states it directly
  (`NstBoardIremG101.cpp:113`), Nintendulator drives CIRAM A10 from PPU A11 when set
  (`mapper032.cpp:22-24`).
- **The register field is five bits and the board tops out at 256 KiB** — OK, equivalent;
  MR-32-1 confirmed it.
- **Submapper 1 (*Major League*) disables $9000 entirely** — OK (`iremg101.rs:93-95`), and
  **this build and puNES are the only two that implement it** (`mapper_032.c:88`, where the
  swap is forced off for submapper 1, and `mapper_032.c:105-110`, where the mirroring is
  forced). FCEUX, Nestopia and Nintendulator let *Major League* swap its PRG layout. MF-1's fix
  is right and now has company.

### 32-1 — which one-screen page *Major League* is held on — DIFFERS, with no observable effect

**Point.** The page: "Major League wants hardwired 1-screen mirroring. (CIRAM A10 is tied to
**+5V** on this game)." CIRAM A10 held high selects the **upper** 1 KiB page.

**This build.** `NametableMirroring::SingleScreenA` — the **lower** page
(`iremg101.rs:78-80`).

**The others.** puNES uses `mirroring_SCR1`, the upper page (`mapper_032.c:107`). The other
three do not implement the case.

**Verdict: DIFFERS** from the page's +5V and from the one other implementation — and
**unobservable**, which is why it is recorded here rather than raised as a bug. With all four
logical nametables mapped to the same physical 1 KiB page, the game writes and reads through
the same mapping whichever page it is; the other kilobyte of CIRAM is never addressed. The
difference could only surface through something that inspects CIRAM directly — a savestate
compared across emulators, or this project's 3D nametable path if it ever reads the unselected
page.

**Risk of a fix.** None: one enum variant at `iremg101.rs:79`. *Major League* is the only
affected game and it is not in the widescreen packs.

**Fixed in MF-LEG-1 (`8b793d47`)**: `NametableMirroring::SingleScreenB`, the upper page, which
is the page's +5V and puNES's `mirroring_SCR1`. It is the one point of this lot where the build
now follows the page **against Mesen**, whose `ScreenAOnly` is the lower page — AMD-061 §1. The
MR-32-1 test gained the other half of its own finding as a second assertion: a `$9000` write must
not move the mirroring of a submapper-1 dump.

Count: 4 OK, 1 DIFFERS, 0 OPEN.

---

## Mapper 34 — BNROM and NINA-001

Page: *INES Mapper 034* (<https://www.nesdev.org/wiki/INES_Mapper_034>). Ours:
`mappers/multicart.rs:1-110`. Theirs: `fceux/src/boards/34.cpp`,
`nestopia/source/core/board/NstBoardAveNina.cpp`, `punes/src/core/mappers/mapper_034.c`,
`nintendulator-mappers/src/iNES/mapper034.cpp`.

- **NINA-001: $7FFD the 32 KiB PRG bank, $7FFE and $7FFF the two 4 KiB CHR banks** — OK
  (`multicart.rs:94-99`). Identical in all four (`34.cpp:44-46`,
  `NstBoardAveNina.cpp:38`, `mapper_034.c:94-95`, `mapper034.cpp:17-19`).
- **BNROM: a 32 KiB bank register over $8000-$FFFF and no $7FFD-$7FFF at all** — OK
  (MR-34-1, fixed in MF-1), and `intercept_cpu_write` returns early when the board is not a
  NINA (`multicart.rs:90-92`). Nintendulator keeps a separate BNROM sync that only banks PRG
  and fixes CHR-RAM (`mapper034.cpp:24-25`).
- **Which of the two a submapper-0 dump is, decided from the CHR-ROM size** — OK, and
  **puNES decides it exactly as this build does**: submapper 1 or (no submapper and CHR
  present) → NINA, submapper 2 or (no submapper and no CHR) → BNROM
  (`mapper_034.c:40-43`). That is MR-34-1's rule with the submappers added, and it is the same
  rule Mesen uses in its factory.
- **BNROM has no work RAM and its register has bus conflicts; the NINA boards have neither** —
  OK (MR-34-2, the work-RAM half fixed in MF-1 and the bus-conflict half by MF-2:
  `registry.rs:1158`, `34 => !mapper_34_is_nina(header)`).

**Test ROMs.** `holy-mapperel/testroms/M34_P128K_H.nes` and `M34_P128K_CR8K_H.nes` — a NINA
and a BNROM configuration. Not run today.

Count: 4 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 41 — Caltron 6-in-1

Page: *INES Mapper 041* (<https://www.nesdev.org/wiki/INES_Mapper_041>). Ours:
`mappers/multicart.rs` (`Caltron6In1`). Theirs: `punes/src/core/mappers/mapper_041.c`;
`fceux/src/boards/41.cpp`, `nestopia/source/core/board/NstBoardNtdec.cpp` and
`nintendulator-mappers/src/iNES/mapper041.cpp` were not read.

MF-1 fixed three findings here (1e789ca4) and puNES agrees with the result on every point I
checked:

- **The outer register is the *address* of a write in $6000-$67FF, not the value** — OK; puNES
  stores the address (`mapper_041.c:49`).
- **The 32 KiB PRG bank is the outer register's bits 2-0, the mirroring its bit 5, and the CHR
  high bits its bits 4-3** — OK; puNES computes PRG from `reg[0] & 0x07`
  (`mapper_041.c:68`), mirroring from bit 5 (`mapper_041.c:74`) and CHR from
  `((reg[0] & 0x18) >> 1) | (reg[1] & 0x03)` (`mapper_041.c:71`).
- **The inner CHR register at $8000-$FFFF is only writable while the outer register's bit 2 is
  set** — OK; puNES gates it the same way (`mapper_041.c:55`).
- **And it has a bus conflict** — puNES ANDs the written value with the PRG-ROM byte
  (`mapper_041.c:57`). This build does **not** give mapper 41 bus conflicts
  (`registry.rs:1105-1159` has no arm for 41). One implementation against none, and the page's
  characteristics row was not re-read for this lot: recorded as an **OPEN** below rather than
  asserted as a finding.

### 41-1 — a bus conflict on the inner CHR register — OPEN

puNES is the only source I read that ANDs the value (`mapper_041.c:57`), and I did not confirm
a "Subject to bus conflicts" row on the *INES Mapper 041* page. The board writes its inner
register into the $8000-$FFFF range, which is where the PRG-ROM drives, so a conflict is
physically plausible — the same argument that put mappers 11, 66 and 78 on the unconditional
list. **What would settle it**: reading the mapper-41 page's characteristics table, and, if the
row is there, adding 41 beside them at `registry.rs:1156` — a one-number change with no hot-path
cost, affecting only the Caltron 6-in-1 multicart. Mentioned here so it is not lost; this lot
does not change code.

Count: 3 OK, 0 DIFFERS, 1 OPEN.

---

## Mapper 64 — Tengen RAMBO-1

Page: *RAMBO-1* (<https://www.nesdev.org/wiki/RAMBO-1>). Ours: `mappers/rambo1.rs`,
`cartridges/a12_watcher.rs`. Theirs: `punes/src/core/mappers/mapper_064.c`;
`nestopia/source/core/board/NstBoardTengen.cpp`, `fceux/src/boards/tengen.cpp` and
`nintendulator-mappers/src/iNES/mapper064.cpp` were not read (see the note below).

MR-1b is the authority here: seven findings, re-read against the page, its talk page and a
hardware thread, with MR-64-7 fixed in MF-1 (3e3b0001) and the A12 filter length settled at 48
dots (`a12_watcher.rs:RAMBO1_FILTER_DOTS`). That work used sources this lot cannot improve on —
a capture-by-capture hardware investigation — so this board is re-checked rather than
re-reviewed.

- **The PRG mode swaps R6 and R15 rather than rotating three windows** — OK (MR-64-7, fixed in
  MF-1), and puNES implements the same swap: its three 8 KiB windows take `prg[2]`, `prg[0]`,
  `prg[1]` in the one mode and a rotation of the same three registers in the other
  (`mapper_064.c:225-227`).
- **A separate mirroring register, not an MMC3-style bit in the bank select** — OK; puNES keeps
  `m064.mirroring` as its own byte (`mapper_064.c:128-129`).
- **Two IRQ modes, scanline and cycle, with a ÷4 divider in cycle mode, a one-clock extra
  divider tick on leaving cycle mode, and a delayed assertion** — OK, all four from MR-64-1,
  MR-64-4 and MR-64-5, which closed against the hardware thread.
- **Bit 0 of a 2 KiB CHR register is *not* ignored** — OK (MR-64-2), the opposite answer from
  the MMC3's and documented as such on both pages.

**Honest note on depth.** For this board I verified the points above against puNES and left
FCEUX, Nestopia and Nintendulator unread. The reason is that MR-1b's RAMBO-1 section already
rests on the hardware investigation thread rather than on any emulator, so a fourth and fifth
emulator vote would not move a verdict; and the lot's remaining time was better spent on the
boards MR-1 had left with one source. Anyone resuming should read
`nestopia/source/core/board/NstBoardTengen.cpp` and `fceux/src/boards/tengen.cpp` against MR-64-1's
counter algorithm if a RAMBO-1 game ever misbehaves.

Count: 4 OK, 0 DIFFERS, 0 OPEN (0 re-opened).

---

## Mapper 71 — Camerica / Codemasters BF909x

Page: *INES Mapper 071* (<https://www.nesdev.org/wiki/INES_Mapper_071>). Ours:
`mappers/discrete.rs:471-544`. Theirs: `punes/src/core/mappers/mapper_071.c`,
`fceux/src/boards/71.cpp` (not read), `nestopia/source/core/board/NstBoardCamerica.cpp`
(not read),
`nintendulator-mappers/src/iNES/mapper071.cpp` (not read).

MR-71-1 and MR-71-2 were both closed in MR-1b, the first **against Mesen**. puNES is a third
vote and it goes this build's way on both:

- **The one-screen polarity: register bit 4 set selects the *second* CIRAM page** — OK, and
  confirmed. The page prints the bit as "Select 1 KiB CIRAM bank", so set = the second page =
  `SingleScreenB` (`discrete.rs:495-500`). puNES maps bit 4 set to `mirroring_SCR1`, the upper
  page (`mapper_071.c:72-79`). Mesen inverts it (MR-71-1). **Two to one, with the page, and
  the next lot should still not "fix" this to match Mesen.**
- **The bank register is at $C000-$FFFF and the mirroring register at $9000** — OK
  (`discrete.rs:532-540`); puNES decodes exactly those two ranges and nothing else
  (`mapper_071.c:45-61`).
- **The $9000 mirroring register exists only on *Fire Hawk*** — a small difference. The page
  notes a submapper has been allocated for it; puNES gates the register on **submapper 1**
  (`mapper_071.c:47-51`, `mapper_071.c:73`), while this build engages it on the first $9000
  write whatever the header says (`discrete.rs:533-535`). Equivalent for *Fire Hawk*, which
  writes $9000; a non-*Fire Hawk* mapper-71 game that wrote $9000 would start driving its
  mirroring here and not on puNES. MR-71-2 already called the missing submapper harmless and
  that still holds, so this stays **OK**.

Count: 3 OK, 0 DIFFERS, 0 OPEN.

---

## Mappers 87 and 101 — Jaleco J87

Pages: *INES Mapper 087* (<https://www.nesdev.org/wiki/INES_Mapper_087>), *INES Mapper 101*
(<https://www.nesdev.org/wiki/INES_Mapper_101>). Ours: `mappers/multicart.rs:609-690`
(`JalecoJ87`). Theirs: `punes/src/core/mappers/mapper_087.c` and `mapper_101.c`,
`fceux/src/boards/datalatch.cpp:324-333` (mapper 87).

- **Mapper 87: the CHR bank is the two low bits of the value, **swapped*** — OK. puNES computes
  `((reg & 0x01) << 1) | ((reg & 0x02) >> 1)` (`mapper_087.c:60`) and FCEUX
  `((latche >> 1) & 1) | ((latche << 1) & 2)` (`datalatch.cpp:327`) — the same swap by two
  routes, and MR-1b confirmed it against the page. PRG is not banked in any of the three.
- **Mapper 101: the register is at $6000-$7FFF only, and it uses the whole byte** — OK as
  **the spec leaves it**, and this is Q-110 question 4. The page and puNES
  (`mapper_101.c:62`, the whole byte) agree with Mesen against AMD-021 R2-003's `value & 3`;
  MR-1b recorded that it is the amendment's reading that costs something, since it caps CHR at
  32 KiB on a board the page calls an "oversize extension" of J87. puNES is a second vote for
  the page. **Still a spec question and not a bug**, exactly as MR-1b left it: R2-003 carries an
  acceptance criterion and only its owner can change it. Recorded here as an **OPEN** so the
  new vote reaches Q-110.
- **FCEUX decodes mapper 87 over $6000-$FFFF** (`datalatch.cpp:333`) where the page gives
  $6000-$7FFF; this build uses the page's range. One against one, page on this build's side.

Count: 2 OK, 0 DIFFERS, 1 OPEN (a vote added to Q-110, not a new question).

---

## Mapper 156 — Daou Infosys DIS23C01

Page: *INES Mapper 156* (<https://www.nesdev.org/wiki/INES_Mapper_156>). Ours:
`mappers/daou306.rs`. Theirs: `punes/src/core/mappers/mapper_156.c`.

MR-1b found one new finding here and MF-1 fixed it (97c274ff). puNES agrees on each point I
checked:

- **16 KiB switchable PRG at $8000 with the last 16 KiB fixed, eight 1 KiB CHR banks** — OK;
  puNES the same (`mapper_156.c:86-92`).
- **The mirroring register, and its power-on value** — OK. puNES seeds `m156.mirroring = 2` at
  power-on and then takes `value & 0x01` from the register (`mapper_156.c:42`,
  `mapper_156.c:73`). This is the finding MF-1 fixed: the board powers up in **four-screen**
  (puNES's 2) rather than following the header, and only a register write moves it. Confirmed.

Count: 2 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 225 — the 52/64-in-1 multicart

Page: *INES Mapper 225* (<https://www.nesdev.org/wiki/INES_Mapper_225>). Ours:
`mappers/multicart.rs:210-307`. Theirs: `punes/src/core/mappers/mapper_225.c`,
`fceux/src/boards/225.cpp` (not read).

- **Everything is in the address of the write: the 16/32 KiB mode bit, the bank field and the
  CHR field** — OK (`multicart.rs:303-307` latches the address). puNES does the same, splitting
  the latched address into a 16 KiB-or-32 KiB choice, a bank and a CHR bank
  (`mapper_225.c:76-83`).
- **The 32 KiB mode drops the low bit of the bank** — OK; puNES `bank >> 1`
  (`mapper_225.c:79`).
- **The CHR bank takes a high bit from address bit 14** — OK; puNES
  `((reg & 0x4000) >> 8) | (reg & 0x3F)` (`mapper_225.c:83`).
- **The four nibbles of RAM at $5800-$5803, mirrored through $5FFF** — OK
  (`multicart.rs:289-301`), which was MR-225-1 and MF-1 fixed it. puNES has no equivalent, so
  this build and Mesen are the only two with it.

Count: 4 OK, 0 DIFFERS, 0 OPEN.

---

## Mapper 228 — Action 52

Page: *INES Mapper 228* (<https://www.nesdev.org/wiki/INES_Mapper_228>). Ours:
`mappers/multicart.rs` (`Action52`). Theirs: `punes/src/core/mappers/mapper_228.c`,
`fceux/src/boards/228.cpp` (not read).

- **Both the address and the value matter: the address carries the mode and the PRG and CHR
  high bits, the value the CHR low bits** — OK. puNES latches both
  (`mapper_228.c:67-68`), takes the bank from `(reg[0] >> 6) & 0x7F`
  (`mapper_228.c:80`), the 16-or-32 KiB choice from address bit 5 (`mapper_228.c:82-86`) and
  the CHR bank from `((reg[0] << 2) & 0x3C) | (reg[1] & 0x03)` (`mapper_228.c:90`). Same
  fields as this build.
- **The mirroring bit** — OK; puNES has it as its own fix (`mapper_228.c:71`).

Count: 2 OK, 0 DIFFERS, 0 OPEN.

---

# For the owner

Nine differences, ranked by how many people would notice. Two sentences each, no jargon.

1. **MMC5 games lose the split screen and the per-tile colours (mapper 5).** Six Japanese
   strategy and simulation games — *Just Breed*, the Koei titles like *Nobunaga's Ambition II*
   and *Romance of the Three Kingdoms II* — draw their backgrounds with a feature this build
   does not have, so their colours come out wrong; two more lose a split-screen effect
   entirely. This is a known gap the specification itself marks as lower priority, and
   *Castlevania III*, the popular MMC5 game, is not affected.

2. **Zelda, Metroid and Final Fantasy could lose their saves if a game writes one particular
   value (mapper 1).** This build implements a PRG-RAM switch that the hardware documentation
   describes and that every other emulator — all five of them — deliberately ignores; one of
   them removed it because a homebrew game broke. The games at risk are the biggest names on
   the board, so this is the entry worth looking at first even though no commercial game is
   known to trip it.

3. **Contra (Japan) and Ganbare Goemon 2 work here only by luck (mappers 22/23/25).** Those
   cartridges check a one-bit memory the board has instead of save RAM, and this build does not
   have it; the games pass their check only because old-style ROM headers cause the emulator to
   put 8 KiB of RAM there by accident. A properly made modern dump of either game, which says
   the cartridge has no RAM, would hang at boot.

4. **Videomation may bank its graphics differently from the hardware (mapper 13).** The board's
   documentation says writes to it collide with the cartridge's own ROM, which changes the value
   that gets stored, and this build does not model that collision. Half the other emulators do;
   whether the one game on this board actually notices can only be found out by running it.

5. **Chinese mapper-15 multicart hacks may show a blank or corrupt screen.** This build blocks
   graphics writes in two of the board's four modes, exactly as the documentation says; two other
   emulators deliberately stopped doing that because a family of hacked ROMs only works in the
   mode where the block applies. Nothing commercial or Western is affected, and the choice is a
   judgement call rather than a clear bug.

6. **Mapper-18 games cannot protect their save RAM (mapper 18).** The board has a switch that
   turns its save memory off and another that makes it read-only, and this build implements
   neither, so a game that tries to guard its save during a crash cannot. Three of the five
   other emulators have the same gap and no game is known to suffer from it, so this is tidiness
   rather than a visible fault.

7. **SZROM's one game puts its save in the wrong place (mapper 1).** *A Ressha de Ikou*, a
   Japanese train simulator, is the only cartridge on this board variant, and this build reads
   the wrong bit to choose which half of its save memory is in use. The fix is a single extra
   case and nothing else on the board can reach it.

8. **Two header variants of common boards are not recognised (mappers 1 and 30).** A correctly
   labelled dump of an early MMC1 cartridge, and a homebrew UNROM-512 image that asks the board
   to control its screen layout, are each run under the wrong rule here. No commercial game uses
   either label.

9. **Major League is held on the wrong half of the video memory (mapper 32).** The
   documentation says the cartridge wires the screen selector high and this build wires it low.
   Nothing can see the difference, because the game only ever uses the half it is given — it is
   recorded for completeness.

**Eleven more points are open**, meaning the documentation is silent or the emulators disagree
and I could not settle it from the sources: power-on values on four boards, two MMC3 IRQ timing
details that only one other emulator models, which chip an MMC1 SOROM save lands on, and three
smaller ones. None of them is known to affect a game; each entry says what evidence would close
it, and in several cases that evidence is a test ROM already sitting in `assets-free`.

**Nothing in this report asks for a change to the games that must not move.** SMB3 is mapper 4,
Zelda II is mapper 1, and the widescreen packs are on boards the mapper work added later. Of the
nine differences, only entry 2 touches a board any of them runs on, and the change it suggests
can only *enable* memory that is currently switched off — a game working today keeps working.

**And lot MF-LEG-1 confirmed that, by measurement rather than by argument.** Six of the nine were
fixed; golden-core over the 98 original ROMs is **byte-identical** to the commit before the lot,
and the five games of entry 2 were played for 3600 frames each with the register sampled after
**every CPU instruction** — 181 million instructions, and not one of them ever writes the value
that would have switched its save RAM off. Two corrections for the owner, both in the entries:

* **Zelda II was never at risk at all.** Entry 2 names it first; its dumps carry 128 KiB of
  CHR-ROM, which the old test already excluded. The games that really are on the switch are
  *Final Fantasy*, *The Legend of Zelda* and *Metroid*, and the fix leaves all three exactly as
  they are — their headers declare the board. The one dump the fix does change is *Mega Man 2*,
  whose header says it has **no save RAM**, which is correct: that cartridge has none.
* **Entry 2's fix is the one thing this lot did not switch on.** The hardware test ROM in the
  repository disagrees with it — two of its boards pass today and fail under the fix, because the
  dump that tests the switch does not declare the memory chip it is testing. The fix is built and
  one flag away; **Q-176** sets out the choice, and it is the owner's.

Entry 9's *Major League* is the only place this lot moved against Mesen 2 and with the hardware
page; entries 4, 6 and 8's mapper-1 half went in unconditionally and cost nothing measurable
(worst ROM +0.001 %, the instrument's own floor, over a thirteen-ROM set).

**And the single best thing this lot found is not a difference at all.** The mapper-conformance
test ROMs are already in the repository and the harness does not run any of them:
`holy-mapperel` ships configurations for sixteen of the numbers under review — mappers 0, 1, 2,
3, 4, 7, 9, 10, 11, 28, 34 and 66 among them — and `nes-test-roms` has `mmc3_test`,
`mmc3_test_2`, `mmc3_irq_tests`, `mmc5test`, `mmc5test_v2`, `exram`, `MMC1_A12` and
`m22chrbankingtest`. The project runs the CPU, PPU and APU suites and `mmc3_test_2`, and nothing
else. Wiring the `holy-mapperel` set into the harness would check most of the register maps in
this report at first hand, and would have settled four of the twenty entries above without any
reading at all.

## Counts

| | |
| --- | --- |
| points checked | **177** |
| **OK** | **157** |
| **DIFFERS** | **9** |
| **OPEN** | **11** |
| boards with no difference at all | **13 of 26** |
| MR-1b conclusions overturned by the new sources | **0** |
| MR-1b conclusions independently confirmed | **14** |

The last two rows are the result worth keeping: four more emulators and a second pass over the
hardware pages did not reverse a single one of MR-1b's verdicts, and confirmed fourteen of them
— including three where MR-1b had ruled **against** Mesen (mapper 71's one-screen polarity,
mapper 28's captured mirroring bit, mapper 11's two-bit PRG field) and one where it ruled against
Mesen's convention on the MMC1's outer bank. The nine differences this lot found are all in
places MR-1 did not look: a board page's bus-conflict row, a register below $8000, a submapper
byte, or a feature the spec had already deferred.

One qualification on "overturned: 0". MR-BUS-1's **list** turned out to be incomplete rather
than wrong: the *CPROM* board page carries the same unqualified "Subject to bus conflicts: Yes"
row that put mappers 11, 66 and 78 on the list, and mapper 13 was not on it. That is a gap in
the enumeration, not a reversal of a verdict, and mapper 41 may be a second instance of the same
gap — see 41-1.
