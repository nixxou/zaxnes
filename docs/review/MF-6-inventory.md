# MF-6 — the boards still missing, and the plan of the sub-lots

Role R3, clean side. Lot MF-6, baseline `fcfa5cb3` (the end of MF-5b, the VRC7). **This lot
changes no emulation behaviour and writes no emulator code**: not one `.rs` file moves. It is the
document MF-6a, MF-6b and MF-6c are each briefed from.

Sources, and nothing else:

| source | what it settles here |
| --- | --- |
| `rust/crates/zax-nes/src/cartridges/registry.rs:21-24` | `SUPPORTED_MAPPERS`, the 41 numbers this build runs |
| `third_party/mesen2/Core/NES/MapperFactory.cpp:279-648` | the numbers Mesen 2 knows, and which class each builds. Mesen 2, GPL v3, commit `b9fa69d` (`third_party/README.md`) |
| `spec/SPEC-02-coeur-nes.md` §3.11 (REQ-CORE-100/101/102), §3.7 (REQ-CORE-075) | the required table, and that going beyond it is permitted |
| `spec/changes/AMD-061-exactitude-des-cartes.md` §1 | public hardware documentation prevails over the reference program |
| `docs/review/MR-1-mappers.md` | the board inventory as reviewed against Mesen, and the two variant findings MR-4-3 and MR-34-1 |
| `docs/progress/MF-3b.md`, `MF-5a.md`, `MF-5b.md` | how a board was added at zero cost to the others |
| `assets-free/test-roms/` | every image's header decoded, by mapper number |
| the NESdev wiki: "Mapper", "List of mappers", and each board's own `INES Mapper NNN` page | board names, register layouts, and the per-board **"N in NesCartDB"** figure |

**Every title count below is the figure its own NESdev board page gives**, either as that page's
"N in NesCartDB" line or as the length of the game list the page prints; the two are distinguished
in the "titles" column, and where a page gives neither the cell says so rather than guessing.
NesCartDB (bootgod's NES Cart Database) is the public cartridge database the NESdev wiki cites
per board; its own host was unreachable from this session, so the counts here are the wiki's
transcriptions of it and are marked as such. Where NESdev and Mesen disagree on a board's
publisher or its licensed status, both readings are given.

---

## 1. What this build has, and what the spec asks for

`SUPPORTED_MAPPERS` is 41 numbers over 32 board types (`board.rs`, 32 `Board` variants):

    0 1 2 3 4 5 7 9 10 11 13 15 16 18 19 21 22 23 24 25 26 27 28 30 32 34 41
    64 66 69 71 78 85 87 101 118 156 180 210 225 228

**REQ-CORE-100's required table is covered in full.** Its 29 rows are all in the list above, and
the build adds twelve numbers beyond it — 5 is in the table, and 19, 24, 26, 69, 78, 85, 118, 156,
180 and 210 are not — which REQ-CORE-102 permits in as many words: "Ajouter des mappers au-delà de
la liste est permis". So **no board in this document is required by the spec**. Every one is an
addition, and the sub-lots are ordered by what each buys.

Two spec items bear on scope and are picked up again in §6:

* **REQ-CORE-101** (P3) makes mapper 116 optional and records that the reference "l'émule
  seulement comme une carte VRC2 simple, de façon incomplète". AMD-061 §1 now says the hardware
  prevails over the reference. Those two readings give different jobs. **Q-130 question 1.**
* **REQ-CORE-102**'s acceptance criterion is "en-tête annonçant le mapper 200 → échec", and
  mapper 200 is a real board Mesen implements (`Mapper200`). **Q-130 question 2.**

## 2. The ROM situation: there is nothing to run, for any of these boards

Every `.nes`, `.unf` and `.unif` under `assets-free/test-roms` was opened and its header decoded —
the iNES signature, byte 6 bits 4-7, byte 7 bits 4-7, and byte 8 bits 0-3 when byte 7 bits 2-3
declare NES 2.0:

    333 images with an iNES signature, over 18 mapper numbers
    0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 22, 28, 34, 66, 69, 78, 118, 180

and all eighteen are already in `SUPPORTED_MAPPERS`. `holy-mapperel`'s highest target is `M180`;
`m22chrbankingtest` is a VRC2a, which mapper 22 covers. Four files carry no iNES header and are
not loadable by this build at all: `nes-test-roms/other/LINUSMUS.NES` (signature `NES\x1b`) and
the three UNIF images `scanline/scanline.unif`, `scanline-a1/scanline.unif`,
`stomper/smwstomp.unif`, which name a board rather than a number.

**So the "test ROM" column of every table below is `no`, without exception**, and it is stated
once here instead of being repeated 245 times. This is the same position MF-5a and MF-5b were in
(their own ROM sweeps found zero images for mappers 19, 24, 26, 85 and 210), and both shipped from
it: every behavioural claim rests on a unit test written against the NESdev page and cross-checked
against `third_party/mesen2`, with commercial titles named for R0 to run. Each sub-lot below names
its titles for that reason.

### The 101 embedded profiles, and the one of them that names a missing board

`assets-neutral/profiles.json` carries 101 profiles in the `full` edition, every one a licensed
title. The file records a title and a profile blob and **no mapper number**, so which board each
dump sits on cannot be settled from this worktree — REQ-CORE-100's second acceptance criterion
("les 101 titres [...] démarrent et sont jouables") is R0's to check, with the ROMs.

One entry is worth flagging rather than leaving to be discovered: the profile list has
**"Super Spike V'ball"**, and `Super Spike V'Ball + Nintendo World Cup` is the Nintendo multicart
whose board is **mapper 47**, which this build refuses. The US standalone cartridge is an ordinary
mapper 4 and loads today; a user whose dump is the multicart gets REQ-CORE-011's refusal and never
reaches the profile. That is one concrete reason mapper 47 is in MF-6a and not further down.

---

## 3. The inventory

245 of the numbers Mesen 2 builds a board for are numbers this registry refuses. They are not
245 equal items, so they are in four tables: the boards with licensed titles (§3.1), the ones that
are a **parameter of a board we already have** rather than a new board (§3.2), the unlicensed
boards of a single publisher (§3.3), and the pirate multicarts and MMC3 clones (§3.4). §3.5 is the
numbers Mesen itself knows and refuses.

The "hooks" column names what the board needs from this core beyond PRG/CHR banking and a
mirroring register. The hot-path consequences of each hook are §4; the column here only names
them.

### 3.1 Boards with licensed titles, and no counterpart in this build

| iNES | board / chip | family | lic. | titles (source) | hooks it needs |
| ---: | --- | --- | --- | --- | --- |
| **206** | DxROM / Namcot 108, 109, 118, 119; Tengen MIMIC-1 | Namco, Tengen | yes | **39 in NesCartDB** (page "INES Mapper 206") | none. No IRQ, no PRG-RAM "with rare exception", hardwired mirroring, fixed CHR window sizes |
| **33** | Taito TC0190 | Taito | yes | 4 on the page (*Akira*, *Bakushou!! Jinsei Gekijou*, *Don Doko Don*, *Insector X*) | none. The page is explicit that the lack of an IRQ is what separates it from 48 |
| **48** | Taito TC0690 (TC0190 + PAL16R4) | Taito | yes | 6 on the page (*Bakushou!! Jinsei Gekijou 3*, *Bubble Bobble 2*, *Don Doko Don 2*, *Captain Saver*, *The Jetsons*, *The Flintstones*) | **scanline IRQ through the A12 watcher**, MMC3-shaped, with "about a 4 CPU cycle delay" on the assertion |
| **65** | Irem H3001 | Irem | yes | page gives neither a count nor a game list | **CPU-cycle IRQ**: "a 16-bit internal down counter which (when enabled), decrements by 1 every CPU cycle" |
| **67** | Sunsoft-3 | Sunsoft | yes | page gives no count | **CPU-cycle IRQ**: a 16-bit counter decremented every CPU cycle, firing when it wraps $0000 → $FFFF |
| **68** | Sunsoft-4 | Sunsoft | yes | page gives no count; it names *Nantettatte!! Baseball* as the one title using the licensing register | **nametable control from CHR-ROM** ($E000 bit 4 maps CHR-ROM banks over $2000-$2FFF); PRG-RAM enable; a licensing/timer register at $6000-$7FFF used only by *Nantettatte!! Baseball*, which also wants a **second ROM image** (the external option cartridge) |
| **70** | Bandai 74161/7432, without mirroring control | Bandai | yes | page gives no count; several *Family Trainer* titles, which "use the Family Trainer Mat as an input device" | none |
| **72** | Jaleco JF-17 | Jaleco | yes | 3 on the page (*Moero!! Juudou Warriors*, *Moero!! Pro Tennis*, *Pinball Quest*) | a write-then-command latch; **expansion audio**, a µPD7756C ADPCM speech ASIC |
| **73** | Konami VRC3 | Konami | yes | **1 in NesCartDB** (*Salamander*) | **CPU-cycle IRQ**: 16 bits, up-counting, no scanline mode, plus an 8-bit mode where the low half overflows from $FF |
| **75** | Konami VRC1 | Konami | yes | **6 in NesCartDB** | none. Three switchable 8 KiB PRG windows, 4 KiB CHR, a mirroring bit, no IRQ, no bus conflicts |
| **76** | NAMCOT-3446 | Namco | yes | 1 (*Megami Tensei: Digital Devil Story*) | none. The 108 rewired for 128 KiB of CHR in coarser banks |
| **77** | Irem LROG017 (discrete: 2×74161, 74138, 74'21) | Irem | yes | 1 (*Napoleon Senki*) | **an unusual CHR view**: CHR-ROM swappable at $0000-$07FF with **four 2 KiB CHR-RAM banks fixed at $0800-$1FFF**, nametables staying on CIRAM |
| **80** | Taito X1-005 | Taito | yes | **8 in NesCartDB** | **128 B of mapper-internal WRAM, battery-backable** — a save region that is neither the header's PRG-RAM nor an EEPROM; switchable H/V mirroring |
| **82** | Taito X1-017 | Taito | yes | **5 in NesCartDB** | **5 KiB of mapper-internal WRAM, battery-backable**; an IRQ that exists on the chip but which "no commercial games used" |
| **86** | Jaleco JF-13 | Jaleco | yes | 2 (the red and black releases of *Moero!! Pro Yakyuu*) | registers at $6000-$6FFF; **expansion audio**, the same µPD7756C; the page also notes "accidental copies of the registers [...] at $E000-$FFFF" |
| **88** | NAMCOT-3433/3443 | Namco | yes | 3 on the page (*Quinty*, *Namcot Mahjong 3*, *Dragon Spirit*) | none. The 108 with PPU A12 wired to CHR A16, which splits the 128 KiB in half by pattern table |
| **89** | Sunsoft-2 on the Sunsoft-3 board | Sunsoft | yes | 1 (*Tenka no Goikenban: Mito Koumon*) | one-screen mirroring; **bus conflicts** |
| **92** | Jaleco JF-19 | Jaleco | yes | page gives no count | as 72 with a 256 KiB PRG; **expansion audio** |
| **93** | Sunsoft-2 on the Sunsoft-3R board | Sunsoft | yes | 2 (*Shanghai*, *Fantasy Zone*) | a CHR-RAM enable bit whose disabled state is "writes ignored, reads open bus"; **bus conflicts** |
| **94** | HVC-UN1ROM | Nintendo | yes | 1 (*Senjou no Ookami*, the Japanese *Commando*) | none. "Very similar to UxROM, but the register is shifted by two bits" |
| **95** | NAMCOT-3425 | Namco | yes | 1 (*Dragon Buster*) | **nametable control from the CHR register**: CHR A15 drives CIRAM A10 |
| **96** | Oeka Kids (discrete: 74161, 7402, 7474) | Bandai | yes | 2 (*Oeka Kids: Anpanman no Hiragana Daisuki*, *…to Oekaki Shiyou!!*) | **CHR banking driven by the PPU address bus**: a fetch in $2000-$2FFF selects the 4 KiB CHR page from address bits 8-9, so the board must see **every** address-bus change, not only reads. Also wants the Oeka Kids tablet as an input device |
| **97** | Irem TAM-S1 | Irem | yes | 1 (*Kaiketsu Yanchamaru*) | none, and no bus conflicts |
| **99** | VS System | Nintendo | yes | arcade | coin/DIP inputs, a different palette, a 2-bank CHR select. **Set aside by the owner**; see §6 |
| **105** | NES-EVENT | Nintendo | yes | **1 in NesCartDB** (*Nintendo World Championships 1990*) | MMC1-based; **a 30-bit M2 CPU-cycle counter** firing an IRQ at a DIP-selected bit (bits 28-25, ≈5-10 min NTSC) |
| **140** | Jaleco JF-11, JF-14 | Jaleco | yes | page gives one example (*Bio Senshi Dan*) | none. GNROM with the register moved to $6000-$7FFF, so **no PRG-RAM** |
| **152** | Bandai 74161/7432, with mirroring control | Bandai | yes | 2 on the page (*Arkanoid 2*, *Gegege no Kitarou 2*) | one-screen mirroring; **bus conflicts** |
| **157** | Bandai Datach Joint ROM System (LZ93D50) | Bandai | yes | the Datach series | the FCG IRQ (have it); **a 24C02 EEPROM**; a barcode reader as an input device |
| **158** | Tengen 800037 | Tengen | unlic. | 1 (*Alien Syndrome*) | the RAMBO-1's **dual CPU-cycle/scanline IRQ** (have it) plus **nametable control from the CHR registers** |
| **159** | Bandai LZ93D50 with X24C01 | Bandai | yes | 4 on the page (*Dragon Ball Z: Kyoushuu! Saiya-jin*, two *Magical Taruruuto-kun*, *SD Gundam Gaiden*) | the FCG IRQ (have it); **a 128-byte X24C01 EEPROM**, big-endian where Bandai's code assumed little-endian |
| **168** | Racermate Challenge II | Racermate | yes | 1 | needs the Racermate exercise-bike peripheral. Mesen files it under `Unlicensed/` |
| **185** | CNROM with security diodes | Nintendo, Bandai, Namco | yes | the page names *Bird Week*, *B-Wings*, *Mighty Bomb Jack* (J), the *Sansuu* series | a CHR chip-select test; submappers 4-7 say which value enables CHR-ROM, submapper 0 is a heuristic. The page: "emulators need not emulate the functionality of security diodes to run games correctly" |
| **188** | Bandai Karaoke Studio | Bandai | yes | 1 | a microphone as an input device, and an inverted PRG bank field |
| **207** | Taito X1-005, alternate mirroring wiring | Taito | yes | part of the 8 above | as 80, with the mirroring bits reinterpreted |
| **552** | Taito X1-017, correct bank order | Taito | yes | the same 5 as 82 | as 82. NES 2.0 552 "represents the actual mask ROM connection method [...] the correct bank order, while iNES Mapper 082 represents the bank order as it was understood before January 2020" |
| **232** | Camerica BF9096 (Quattro) | Codemasters | unlic. | 3 on the page (*Quattro Adventure*, *Quattro Sports*, *Quattro Arcade*) | none; the lockout-defeat circuit is "mostly unimportant for emulation purposes" |
| **104** | Golden Five | unknown | unlic. | multicart | none beyond a two-register PRG split |
| **111** | Cheapocabra (GTROM) | homebrew | homebrew | several Memblers/homebrew titles | self-flashing PRG; **32 KiB of CHR-RAM with mapper-selected nametable pages**; an LED register |
| **116** | Gouder SOMARI-P (Huang-1/Huang-2) | Gouder | unlic. | 6 on the page (*Somari*, *Kart Fighter*, *Garou Densetsu Special*, …) | **three ASICs in one board** (MMC1, MMC3 and VRC2), each with its own register set, selected by $4100. REQ-CORE-101's optional number; see Q-130 |

Boards 29, 31, 218, 284, 323, 324 and 682 are the remaining `Mappers/Homebrew/` numbers
(SealieComputing RET-CUFROM, NSF cart 31, Magic Floor, Drip Game, Farid's SLROM and UNROM
boards, Rainbow). None has a licensed title; 218 (Magic Floor) is the only one with a hook worth
naming — it has no CHR memory at all and serves patterns from CIRAM.

### 3.2 Numbers that are a *parameter* of a board this build already has

This is the cheapest column in the document, and the point of separating it: these are not new
`Board` variants. `board.rs`'s own note is why that matters — 32 variants, `Mmc5` pinned at index
5 by the niche-filling layout, **appended never inserted** — so a number served by an existing
variant with a load-time flag costs no enum slot, no jump-table entry and no new dispatch arm.
MF-5a did exactly this for mapper 26 ("one `address` rewrite, decided by a load-time `bool`, not a
`match` on the number") and MF-5b for the VRC7's select line.

| iNES | board | the existing board it parameterises | what the parameter is |
| ---: | --- | --- | --- |
| 76, 88, 95, 154 | NAMCOT-3446 / -3433 / -3425 / -3453 | the **206** board of MF-6a | CHR window size (76), PPU A12 → CHR A16 (88), CHR A15 → CIRAM A10 (95), a one-screen bit over all of $8000-$FFFF rather than $8000-$9FFF (154) |
| 206 | DxROM | — | the parent. MR-1's inventory already notes 118 (TxSROM) is served by `Mmc3`; 206 is the *simpler* predecessor and is its own board |
| 37, 47 | Nintendo MMC3 multicarts | `Mmc3` | an outer block register at $6000-$7FFF, writable only while MMC3 PRG-RAM is enabled and writable |
| 119 | TQROM | `Mmc3` | CHR bank bit 6 selects the CHR-RAM chip instead of the CHR-ROM chip |
| 12, 14, 44, 45, 49, 52, 74, 114, 115, 121, 123, 126, 134, 165, 182, 187, 191, 192, 194, 195, 196, 197, 198, 199, 205, 208, 215, 217, 219, 224, 238, 245, 249, 250, 254, 258, 259, 260, 262, 263, 268, 287, 292, 313, 325, 333, 348, 366 | MMC3 clones and pirate extensions | `Mmc3` | one outer register, or a CHR-RAM window, or a scrambled register order, per number. **48 numbers, none with a licensed title**; see §3.4 |
| 105, 155 | NES-EVENT, MMC1 with PRG-RAM always on | `Mmc1` | 105 adds the M2 timer; 155 removes the PRG-RAM disable bit (MR-1-4's submapper-5 fix is the same code site) |
| 153 | Bandai FCG with battery WRAM | `BandaiFcg` | 8 KiB of battery-backed WRAM at $6000-$7FFF; registers "function the same way as on INES Mapper 016, submapper 5" |
| 157, 159 | Datach, LZ93D50+X24C01 | `BandaiFcg` | the EEPROM, which is the one piece that is **not** a flag — see §4.6 |
| 158 | Tengen 800037 | `Rambo1` | nametable control from the CHR registers |
| 185 | CNROM with diodes | `CnRom` | a chip-select test, by submapper |
| 94, 180 | UN1ROM, 7408 UxROM | `UxRom` | 180 is already a parameter of `UxRom` (`registry.rs:186`); 94 is the same shape with the field shifted two bits |
| 152 | Bandai 74161/7432 + mirroring | the **70** board of MF-6a | a one-screen bit |
| 207 | Taito X1-005 alternate | the **80** board of MF-6c | the mirroring bits' meaning |
| 552 | Taito X1-017 correct order | the **82** board of MF-6c | the bank order |
| 92 | Jaleco JF-19 | the **72** board of MF-6c | PRG size and one command bit |
| 151 | VRC1 on a VS board | the **75** board of MF-6a | nothing but the VS System wrapper, which §6 sets aside |
| 4 sub 3 | Acclaim MC-ACC | `McAcc` | **already done**: MR-4-3's finding, shipped as "the same code, monomorphised with the difference as a `const` parameter, so the MMC3 proper pays nothing" (`registry.rs:188-193`) |
| 34 sub 1/2 | NINA-001, BNROM | `Mapper034` | **already done**: MR-34-1, resolved by `mapper_34_is_nina` (`registry.rs:52-59`) |

**The lesson the sub-lots must carry over from the `McAcc` line**: a variant that differs by one
constant becomes a `const` generic parameter on the existing board, monomorphised, so the common
board's generated code is untouched. A variant that differs by a *runtime* `bool` field costs the
existing board a test it did not have. MF-5a's mapper 26 shows the second is acceptable when the
test is off the per-cycle path (it is in the register decode); `McAcc` shows the first is what to
reach for when it is not.

### 3.3 Unlicensed boards of a single publisher

Documented on NESdev, implemented by Mesen, **no licensed title between them.** Listed by number
so the inventory is complete; none is proposed for any sub-lot (§6 says why).

* **Sachen** (14): 133, 136, 137, 138, 139, 141, 143, 145, 147, 148, 149, 150, 243, 513
* **Kaiser** (11): 56, 142, 171, 175, 302, 303, 305, 306, 307, 312, 346 — mostly FDS-to-cartridge conversions
* **Waixing** (8): 162, 164, 176, 178, 242, 252, 253, 286 — 176 is the FK23C, the board of the large Chinese multicarts
* **TXC** (7): 36, 61, 132, 172, 173, 189, 299
* **NTDEC** (7): 63, 112, 174, 193, 221, 290, 298
* **JY Company** (5): 35, 90, 91, 209, 211 — the most capable of this group: its own scanline IRQ and a CHR latch
* **Whirlwind** (5): 40, 125, 304, 309, 522 — FDS conversions
* **FFE copier boards** (3): 6, 8, 17 — Front Far East development/copier hardware, not a retail board at all

### 3.4 Pirate multicarts, BMC boards and MMC3 clones

The remaining 134 numbers, every one of them `Mappers/Unlicensed/`, `Mappers/Mmc3Variants/` or a
`Bmc*` class in Mesen, and none with a licensed title:

    38 39 42 43 46 50 51 53 54 57 58 59 60 62 79 83 103 106 107 108 113 117 120
    144 146 163 166 167 170 177 179 183 190 200 201 202 203 204 212 213 214 216
    222 226 227 229 230 231 233 234 235 236 240 241 244 246 255 261 264 265 266
    274 283 285 288 289 300 301 314 319 320 328 329 331 332 336 349 487 518 519
    521 529 530
    + the 48 MMC3 clones listed in §3.2

Three of these have enough of a following to be named rather than counted: **163** (Nanjing, the
Chinese RPG boards, with a CHR banking mode driven by the PPU scanline), **176** (FK23C) and
**90/209/211** (JY Company). All three are unlicensed, all three have NESdev pages, and none has a
3D profile — which §6 argues is the decisive point for this application.

### 3.5 Numbers Mesen 2 knows and refuses, and the gaps

Mesen has a `case` for these and `break`s out of it, so it documents them as known-and-unhandled;
NESdev has a page for most. They are **not** candidates for any sub-lot.

    186 (Study Box, loaded as a BIOS not a ROM), 256 (ONEBUS), 257 (PEC-586), 271, 295,
    308, 315, 322, 327, 335, 337, 338, 339, 340, 341, 342 (COOLGIRL), 344, 345, 350,
    524, 525, 526, 527, 528

Mesen's own comments also mark these numbers as having no implementation at all: 267, 269-270,
272-273, 275-282, 291, 293-294, 296-297, 310-311, 316-318, 321, 330, 334, 343, 347, 514-517, 520,
523. Plus the UNIF-only boards (`UnifBoards::Ac08`, `Cc21`, `Ghostbusters63in1`, `Gs2013`,
`Malee`, `SssNrom256`, `Unl255in1`, `Unl8237A`, `UnlPuzzle`), which have no iNES number and which
this build cannot reach: it loads iNES and NES 2.0 only.

**The FDS** (`MapperFactory::FdsMapperID`) is out of scope: it is a disk system, not a mapper, and
REQ-CORE-075 names its audio among the optional chips without the spec ever asking for the drive.

