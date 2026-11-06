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

---

## 4. The hooks, and which hot path each one touches

A board costs this core in two separate currencies, and the sub-lots have to be budgeted in both:

* **its own arm** in `NesBus::write_internal`, `Board::on_cpu_cycle`, `Board::ppu_read` and so
  on — reached only by a cartridge of that board, and therefore free to every other game. MF-5a
  measured the VRC6's whole register decode at **+141 instructions in `write_internal`** and MF-5b
  the VRC7's at **+138**, both inside their own arm, both costing an NROM nothing.
* **the shared shape** of the dispatch that holds that arm — its prologue, its jump table, the
  inliner's budget for its caller. This is the currency that has bitten every lot, and it is paid
  by **every** game. The sections below are one per hook, each with the measurement that priced it.

### 4.1 `Board::on_cpu_cycle` — a CPU-cycle IRQ or expansion audio

Boards in this inventory that need it: **65, 67, 73, 105**, 82 if its IRQ is implemented, and the
ADPCM of **72, 86, 92** if that were ever done.

**MF-5a step 2 made this hook free to the boards that do not need it**, and the figure is exact:
`Cartridge::step_cpu_cycle` used to dispatch unconditionally, which callgrind priced at
**98 275 760 instructions over 300 frames on every ROM of the thirteen whatever its board —
eleven instructions per CPU cycle**. The `Cartridge::clocks_on_cpu_cycle` latch, tested *before*
the dispatch, took that to **zero** for the twenty-six mapper numbers that inherit the empty
default, at a cost of **two instructions per cycle in `NesBus::tick`** to the twelve that are in
the list.

So a new CPU-cycle board costs every other board nothing — **provided three things**, each a step
in the sub-lot rather than an assumption:

1. **The mapper number goes into the `matches!` in `Cartridge::create`.** Forgetting it is a
   silent correctness bug, not a slow path: the counter is never clocked. This is MF-3b rule 5's
   warning, and it is tested rather than trusted —
   `mf_5a_every_board_that_moves_on_a_cpu_cycle_is_latched` sweeps `SUPPORTED_MAPPERS` ×
   submappers 0 and 3, drives 400 bare CPU cycles at each board and requires the latch wherever
   `Board`'s `PartialEq` says the state moved. **A sub-lot that adds a CPU-cycle board and does
   not extend that list fails an existing test.** Nothing to write.
2. **The arm must be a tail call.** MF-5a's VRC6 inlined its audio into the arm and took the
   dispatch's shared prologue from `pushq %rsi / subq $32, %rsp` to
   `pushq %rsi / pushq %rdi / subq $40, %rsp` — **one more push and pop per CPU cycle for every
   board, an NROM included.** That is MF-3b rule 3 in its exact form: the arm that matters is the
   empty one. What worked was `#[inline(never)]` on the *board's own* `on_cpu_cycle`, not on the
   audio body.
3. **The prologue must be read on the dump of the platform being measured.** MF-5a's own words:
   `#[inline(never)]` on the audio body "put the Windows prologue back", and callgrind then showed
   it "had done nothing at all for the Linux build" — +2 per CPU cycle on all thirteen ROMs.
   Windows x64 and System V differ in argument registers and callee-saved sets, so **an attribute
   chosen to fix a register-allocation symptom in one dump is not evidence about the other.**

### 4.2 `NesBus::irq_asserted` — *any* board that can assert an IRQ

**This is the most expensive hook in the inventory and the one a sub-lot is most likely to trip
over, because it costs nothing in the board's own code.** Boards needing it: 48, 65, 67, 73, 105,
153, 157, 158, 159, and 82 if its IRQ is done.

MF-5b step 1 is the whole story. The VRC7 board is "twenty-two lines of banking and a shared IRQ
counter", and adding it cost **+0.542 % on all thirteen ROMs**, with the symbol diff naming it in
one line on every one of them:

    +169,739,749    0 -> 169,739,749   <NesBus as CpuBus>::irq_asserted

`Cpu6502::end_cycle` samples /IRQ once per CPU cycle through `NesBus::irq_asserted`, which inlines
`Cartridge::irq_asserted` and with it the whole `Board::irq_asserted` dispatch — **the
thirty-second arm grew the callee past the inliner's budget and LLVM outlined the whole chain**,
about **19 instructions per CPU cycle on twelve ROMs of thirteen whose board cannot assert an IRQ
at all.**

Three things follow, all three load-bearing for MF-6b:

* **What fixed it was four `#[inline(always)]`** — on `NesBus::irq_asserted`,
  `Cartridge::irq_asserted`, `Cpu6502::execute` and `Board::try_read_prg`. They are in the tree
  now, so the chain is pinned — but **pinning does not make the dispatch free to grow**, it makes
  LLVM keep inlining a dispatch that is getting bigger. The 33rd, 34th and 35th arms are
  unmeasured, and MF-6b adds up to five.
* **The load-time `irq_capable` latch was built and refuted.** MF-3b costed it on paper and
  declined; MF-5b built it and measured **−1.1 to −1.5 % on four ROMs and +1.3 to +1.8 % on
  three, two of them MMC3s**, because a board that *can* assert an IRQ then pays a `callq` per CPU
  cycle instead of an inlined bitset. **A sub-lot must not re-litigate this**: the numbers are in
  `MF-5b.md` step 1 and the answer is no.
* **`callsites.py` now watches `NesBus::irq_asserted` and `Cartridge::irq_asserted`.** MF-3b rule
  1 reported `Board::irq_asserted` 0 → 0 while the 170 M regression was happening, because the
  regression had moved one link up the chain and the watched list only had the last link. That was
  the second time the instrument was blind to a real outlining, and both times the fix was to
  widen it rather than to trust it.

`Cpu6502::branch` **+11** and `Cpu6502::resolve` **+47** are the per-arm cost of one more
`irq_asserted` variant, measured identically in MF-5a step 1 and MF-5b step 1: extra copies of the
inlined three-instruction block by tail duplication, plus one more jump-table slot. Those two
figures are the yardstick — a sub-lot whose numbers are far from +11/+47 per arm has done
something else as well, and should find out what.

### 4.3 `Board::on_ppu_address_bus` — a scanline IRQ through the A12 watcher

Boards needing it: **48**, and **158** inherits the RAMBO-1's, which is already paid for.

Gated by **two** latches, not one (`cartridges/mod.rs:673-700`): `watches_ppu_reads` (mappers 4,
5, 64, 118) for the /RD-low path, and `watches_ppu_bus_without_read` (4, 64, 118) for the
`$2006`/`$2007` path, because "A12 is an address line and is driven by both, so the three A12
counters need this, and a board whose rule is about /RD must not see it". The function's own
comment prices the traffic: **"57 600-odd calls a frame, measured"** — the densest hook in the
core after the CPU clock.

MF-3b step 2 made it free to the boards that do not need it and recorded what it costs the ones
that do: the twenty-six skip it entirely, and mappers 4, 64 and 118 pay "about +2 instructions per
PPU fetch". A sub-lot adding mapper 48 **adds the number to both latches** and is swept by
`mf_3b_every_board_that_moves_on_the_ppu_address_bus_is_latched` with nothing to write.

One structural item is already waiting for whoever does 48, from MR-1's shared A12 section:
`A12Watcher::FILTER_DOTS` is a **constant**, and its three users disagree — the shared watcher
wants ≥11 dots, Mesen's MMC3 ≥3 CPU cycles, `Rambo1` ≥31 dots (MR-64-3). MR-1's own conclusion:
"Ours needs the same: a `FILTER_DOTS` passed in at construction, not a constant." The TC0690's
"about a 4 CPU cycle delay" on the *assertion* is a second parameter beside the filter, and
MR-64-4 is the same shape on the RAMBO-1 (a 1-2 cycle delay, left unimplemented for want of a
cycle-level IRQ probe the fixture does not have). **Mapper 48 should not be the lot that invents
that probe**: it should reuse whatever MR-64-4's fix builds, or record the same gap in the same
words.

Also from MR-1: `A12Watcher::MIN_RISE_INTERVAL` is derived from `FILTER_DOTS` and feeds
`Mmc3::ppu_irq_horizon`, which bounds how far the PPU may run behind the CPU. **The two must move
together** or the horizon becomes a wrong bound and an IRQ lands late. This is the one place in
the mapper code where an accuracy change reaches a scheduling decision, and a sub-lot touching
`A12Watcher` has to say so out loud.

### 4.4 `Board::ppu_read` and the CHR latch — PPU-read hooks

`Cartridge::ppu_read` dispatches **unconditionally** (`cartridges/mod.rs:606`): every CHR fetch
goes through `Board::ppu_read`, which is 328-336 instructions and gains one arm per board. It is
not latched and cannot be — a pattern byte has to come from the cartridge. MF-5a measured **+16**
for the VRC6's arm and MF-5b reported the function `IDENTICAL raw` at 336 after the VRC7's, so an
arm that is a plain banked read is cheap; the risk is an arm that *calls* something.

No board in §3.1 needs an MMC2-style CHR latch, and the MMC2's own latch does not even ride this
hook: `Cartridge::advance_chr_latch` is called from the tile decode (`ppu/tiles.rs:292`, `:832`),
not from the per-fetch path.

**Mapper 96 (Oeka Kids) is the one PPU-read-shaped hook in the inventory, and it is not this
hook.** It reacts to nametable *addresses*, not to pattern fetches, and its page is explicit that
the board "monitors every change to the PPU Address lines" — including a bare `$2006` write with
no fetch behind it. That is `on_ppu_address_bus_without_read`, i.e. §4.3's second latch, which
puts mapper 96 in an A12 board's cost class even though it has no IRQ.

MR-1's MMC2 section records the one open question here, classified **unknown**: Mesen drives its
latch from every PPU address-bus change where we hook `ppu_read`, so "a bare `$2006` write of
`$0FD8` flips the latch with no pattern fetch at all" on Mesen and not here; REQ-CORE-033 and
REQ-CORE-039 constrain our side anyway. **Mapper 96 would be the first board for which that
distinction is not academic**, because its whole mechanism is the address bus. A sub-lot doing 96
inherits MR-1's open question and must say which way it read it.

### 4.5 `Mapper::read_nametable` — nametable control, and the Q-123 ceiling

**This is the hook the inventory should make a sub-lot most afraid of**, and the reason is a
measurement, not a judgement.

There are two levels of "nametable control" and they cost wildly differently:

* **choosing a CIRAM page per logical table** — `Mapper::nametable_page`, behind the
  `Cartridge::nametable_pages_vary` latch (mappers 5, 19, 24, 26, 118). Cheap. **Boards 95, 154,
  158, 68-in-CIRAM-mode and possibly 111 all land here**: add the number to the latch, return a
  page.
* **serving the nametable byte from the cartridge** — `Mapper::read_nametable`, reached when
  `nametable_page` returns `SERVED_BY_CARTRIDGE`. **This is capped at two distinct arm bodies, and
  the cap is measured.**

Q-123 is the measurement. `Ppu2C02::read_nametable_inner` is 132 instructions with two arm bodies
(the MMC5's ExRAM/fill and the UNROM 512's four-screen CHR-RAM), which LLVM compiles as a bitset.
MF-5a built **five** shapes for a third body — the VRC6's `$B003` bit 4, "nametables from
CHR-ROM" — and every one of them cost every other board:

| shape | `read_nametable_inner` |
| --- | ---: |
| baseline | **132**, bitset |
| the third arm as first written | 83 — the dispatch leaves by a tail call |
| `#[inline(always)]` on `Mapper::read_nametable` | 80 — the boundary moves up to `Cartridge::read_nametable` |
| `#[inline(always)]` on both | 109, and it becomes a **jump table**; the MMC5's arm goes out of line |
| the arm shrunk to one indexed load | 116, and the prologue grows to `pushq %rsi / pushq %rbx / subq $40` |
| **no third arm** | **132, `IDENTICAL` to the baseline** |

**"The trigger is the third distinct body, not its size"** — the fifth row is what proves it, and
it is why no amount of shrinking the arm would have worked. MF-5a took the attributes back out,
left `$B003` bit 4 decoded and ignored, and wrote the measurement on
`<Board as Mapper>::read_nametable` so the next lot would read it "before adding a fourth board".
Q-123 holds the provisional choice and
`q_123_vrc6_keeps_its_nametables_on_ciram_when_n_asks_for_chr_rom` pins it.

**Mapper 68 (Sunsoft-4) is that third body.** Its `$E000` bit 4 maps CHR-ROM banks over
$2000-$2FFF, which is a cartridge-served nametable in the exact sense Q-123 is about — and it is
the *same feature* the VRC6 has and we deliberately do not implement. **Mapper 111 (Cheapocabra)
would be a fourth** if its 32 KiB of CHR-RAM nametable pages do not reduce to `nametable_page`
alone. **Mapper 77 (Napoleon Senki) is not one**: its cartridge RAM sits at $0800-$1FFF, inside
CHR space, with nametables staying on CIRAM, so it is a `chr_view` question and not this one.

The honest conclusion for the plan: **a board whose only reason to exist is a cartridge-served
nametable is not worth 132 → 109 on every game in the library.** If MF-6c does mapper 68, the
CIRAM half (the four mirroring modes, the PRG-RAM enable, the banking) is free, and the CHR-ROM
nametable mode has to either reopen Q-123 with a shape nobody has tried, or be decoded and ignored
with a test pinning that — which is what the VRC6 already does, so both the precedent and the
test's shape exist. MF-6c's step for 68 is written that way in §5.

### 4.6 PRG-RAM, mapper-internal WRAM and EEPROM — the save path

Three different things, and the inventory separates them because only one of the three is new:

* **ordinary PRG-RAM at $6000-$7FFF**, sized by the board. `registry::board_memory_sizes` already
  does this per mapper (0 for BNROM and UNROM 512, 1 KiB for the MMC6, 16 KiB of CHR-RAM for
  CPROM, the Namco 163's four windows, the MMC5's floor), and `registry::battery_offset` says
  which part a host persists. **Boards 153, 47, 37, 140, 184 and 97 need nothing new here** — one
  arm of an existing `match`, with the page quoted in the comment as the existing arms do.
* **mapper-internal WRAM**: **80 and 207** carry 128 bytes inside the ASIC, **82 and 552** carry
  5 KiB, both battery-backable. These are not the header's PRG-RAM, and REQ-CORE-095 only promises
  a host "la PRG-RAM (8 Kio)". The pattern to follow already exists and is already written down:
  `registry.rs:90-97` puts the Namco 163's internal 128 bytes "at the end of PRG-RAM so that
  REQ-CORE-095's single battery slice carries it, which is also Mesen's `.sav` layout".
  **So 80, 82, 207 and 552 are the Namco 163's shape, not a new mechanism** — the single most
  useful thing this section can hand MF-6c.
* **a serial EEPROM**: **157** (24C02) and **159** (X24C01). This *is* new. There is no
  counterpart in this build — MR-1's shared table records "no counterpart" against Mesen's
  `BaseEeprom24C0X.h`, `Eeprom24C01.h` and `Eeprom24C02.h` — and REQ-CORE-100's mapper-16 row
  makes it **P3** in as many words ("IRQ ; EEPROM : P3"). An EEPROM is a state machine clocked by
  register writes, so it costs nothing on any hot path: its whole cost is in `write_register` and
  in a save-state block. **159's one trap is documented and should be quoted into the sub-lot**:
  "Bandai's developers consistently thought the X24C01 was a little-endian device" and it is
  big-endian, so the address increments differently across a multi-byte operation.

### 4.7 Bus conflicts

**Already built, and the cheapest hook in the document.** MR-BUS-1 was a finding; MF-1/MF-2
shipped `registry::has_bus_conflicts` plus one test in `Cartridge::cpu_write`, latched into
`Cartridge::bus_conflicts` so the decision is taken once at load. Boards **89, 93 and 152**
declare conflicts on their NESdev pages; each is **one arm of the `match` at
`registry.rs:166-177`**. No hot path moves: the test is already there for every board, taken or
not.

### 4.8 Expansion audio and the mixer

Boards: **72, 86, 92** — the Jaleco µPD7756C — and nothing else in the inventory.

REQ-CORE-075 is the governing text and it **does not list this chip**: "L'audio d'extension des
cartouches (VRC6, VRC7, MMC5, Namco 163, Sunsoft 5B, FDS) PEUT être émulé." Six chips, and the
µPD7756C is not one of them. More decisively, the chip plays **ADPCM samples held in a mask ROM
inside the ASIC**: the NESdev page for 086 says an implementation needs "sample files for the
speech synthesis chip emulation". Those bytes are not in `assets-free/`, not in
`assets-neutral/`, not in the spec, and not obtainable from any source this role may use.

**So the decision is taken here rather than deferred: mappers 72, 86 and 92 get their boards and
not their audio.** The three games are playable without the speech; what is lost is recorded
voice. A test pins the silence so that it is a choice and not a bug, exactly as the VRC6's
Q-123 test pins its ignored bit. **No Q ticket**: REQ-CORE-075's list settles it, and AMD-061 §1
does not reopen it, because no documentation this role may read contains the samples.

The mixer cost, recorded in case a later lot finds them: `NesBus::push_expansion_audio` has **15
call sites**, and MF-5a priced the MMC5's naive version at **178 683 220 instructions over 300
frames, ~20 per CPU cycle**, because the board recomputed `M` from scratch every cycle. The fix is
the cached `level` + `refresh_level` shape the VRC6 established and the MMC5 and Namco 163 now
share, with `level` **derived and therefore not serialised**. Any new chip follows it.

### 4.9 The `Board` enum itself

Not a hook, but a shared cost the plan must respect, and `board.rs`'s own note is the authority:
32 variants today, **appended never inserted**, for two measured reasons — `Mmc5` must stay at
index 5 because it is the largest variant and the enum's niche tag lives inside it, so every
dispatch prologue is a literal `movl $5`; and the existing discriminants are the jump-table slots
of every dispatch, so appending leaves the earlier arms instruction-identical where inserting in
the middle shuffles four of them.

Two consequences for the sub-lots:

1. **Every new board goes at the end of `Board`, whatever its number.** Non-negotiable, and
   already written in the file.
2. **A board larger than the MMC5's ExRAM would move the niche tag and rewrite every prologue.**
   Nothing in this inventory comes close — but a sub-lot that gave a board a kilobyte-scale buffer
   would pay MF-2 step 0's reshape cost. The 5 KiB of mapper-internal WRAM on 82/552 is a reason
   to put it in `CartridgeMemory`, which is exactly what the Namco 163 pattern in §4.6 does.

### 4.10 Summary: what each hook costs the games that do not use it

| hook | gate today | cost to a board that does not need it | the lot that made it free |
| --- | --- | --- | --- |
| `Board::on_cpu_cycle` | `clocks_on_cpu_cycle` latch | **0** (was 11 instructions per CPU cycle) | MF-5a step 2 |
| `Board::on_ppu_address_bus` | `watches_ppu_reads`, `watches_ppu_bus_without_read` | **0** (was the dispatch on 57 600 fetches a frame) | MF-3b step 2 |
| `Mapper::nametable_page` | `nametable_pages_vary` latch | **0** | MF-3b / MF-5a |
| `Cartridge::read_rom_window` | `rom_windows` latch | **0** | earlier MF lots |
| bus conflicts | `bus_conflicts` latch | **0**, the test is already in `cpu_write` | MF-1 / MF-2 |
| `Cartridge::mapper_number` | a plain `i32` field | **0** (was a per-sprite dispatch; −4 in `run_dots`) | MF-5a step 1 |
| `NesBus::irq_asserted` | **none — four `#[inline(always)]`** | **one arm's worth**, ≈+11 in `branch` and +47 in `resolve`; the latch was built and refuted | MF-5b step 1 |
| `Board::ppu_read` | **none**, dispatches unconditionally | **one arm's worth**, +16 measured | — |
| `Mapper::read_nametable` | **2 distinct bodies, a measured cap** | **132 → 109 for every game** at the third body | Q-123, MF-5a step 1 |
| `NesBus::push_expansion_audio` | `expansion_audio_output` returning `None` | one arm's worth; the per-cycle recompute is the real cost | MF-5a steps 2-3 |

**The two unlatched rows are the whole performance story of MF-6**: `NesBus::irq_asserted`, which
every IRQ board grows and which MF-6b grows five times, and `Mapper::read_nametable`, which is
capped at two bodies. Everything else in this inventory is free to the rest of the library by
construction, and the tests that prove it already exist and sweep `SUPPORTED_MAPPERS`.
