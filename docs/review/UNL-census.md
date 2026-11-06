# UNL-census — the unlicensed and homebrew boards, and which of them carry games worth preserving

**Lot**: UNL-census. **Baseline**: `8496c503`, the end of MF-6a3. **This document writes no
emulator code.** It replaces `MF-6-inventory.md` §6 item 1, which excluded 182 numbers on a
performance argument the owner retired on 2026-11-06: a costly board now ships behind its own
Cargo feature, off by default (`BOARD-FEATURES.md`), so no board is excluded for cost of
*execution*. The three reasons left are **no reliable public documentation**, **no title**, and a
**development cost out of proportion to what the board preserves**.

## 0. Scope, method and sources

### The number set

199 numbers, enumerated rather than taken from the inventory's subtotals — which do not add up,
and the correction is in `docs/progress/UNL-census.md` step 0. In short: §3.3 holds **60**, §3.4's
explicit list **83** (not the "134" its header claims), §3.2's MMC3-clone row that §3.4 adds by
reference **48**, and the homebrew numbers **8**. So §3.3 + §3.4 is **191**, not 182, and with the
homebrew numbers the census covers **199**. Mapper 14 is already inside the clone row.

### The categories

| | meaning | consequence |
| --- | --- | --- |
| **A** | **original content** — at least one game that exists on no other board this build supports | a candidate; gets a documentation grade, a hook list and an effort estimate |
| **B** | **derivative** — translations, hacks or ports whose original runs on a supported board | out, on the owner's content rule |
| **C** | **compilation** — multicarts of games available elsewhere | out, on the owner's content rule |
| **D** | **FDS conversion** — a cartridge port of a game the FDS would run | out, on the owner's content rule |
| **E** | **no title / no documentation** | out, on two of the three remaining reasons |

**Two decisions this census makes, because the brief's definitions do not reach them.**

1. **A port of a game from a non-NES platform is A, not B.** The brief defines B as a port "whose
   original runs on a supported board". Hummer Team's *Super Mario World*, Sachen's *Super Pang*
   and the UNL-TF1201 *Lethal Weapon* are ports of SNES, arcade and Konami-arcade games that never
   shipped on a Famicom cartridge; their NES code is written from nothing and exists on no other
   board. They are what the owner's rule calls original content. A port of a game that *does* run
   on a supported board — Waixing's Chinese *Dragon Ball Z: Kyōshū! Saiya-jin*, whose Bandai
   original is mapper 16 — is B.
2. **A is a promotion; B, C and D are not ranked against each other.** Any single category-A title
   promotes the number to A, because that title is what shipping the board would preserve — a board
   carrying one original game and nine multicarts is A, and where that promotion is the only reason
   for the category the row names the single title that does it. Below A the number takes the
   category of the **bulk** of its catalogue, since B, C and D are all excluded under the same
   owner's rule and nothing turns on the order. §2.0 restates this where it first bites.

### Sources, and one that is unreachable

* **NESdev wiki**, `https://www.nesdev.org/wiki/INES_Mapper_NNN` for numbers below 256 and
  `.../NES_2.0_Mapper_NNN` at and above it. The primary source for every row: board name,
  publisher, titles, registers. Where several numbers share one page the row says so — it matters,
  because a shared page does not attribute its titles per number.
* **NesCartDB** (`nescartdb.com`) **answers HTTP 403 to every request from this environment**, so
  the per-number cartridge counts the earlier inventory lots quoted ("39 in NesCartDB") cannot be
  refreshed here. Where a count is quoted below it is quoted *from the NESdev page that states it*,
  and the row says so. This is a real gap in the census and the one thing a reviewer with a browser
  should re-check.
* **Wikipedia** as the second public source where it covers a publisher: `/wiki/Sachen` (the Sachen
  catalogue, with the article's own original-versus-port framing) and `/wiki/Hummer_Team` (the
  Hummer Team ports, with per-title "a port of …" attributions).
* **Mesen 2**, `third_party/mesen2/Core/NES/MapperFactory.cpp:280-560`, as a *local* second source
  for the board identity of every number — its `case N: return new <Class>()` is an independent
  statement of which boards are the same board. Cited as `MapperFactory:<class>`. It is an
  emulator's source, so it grades a number's documentation as "only an emulator's source" and
  never as "complete page".
* **The publishers' own catalogue codes** on the NESdev pages (Sachen `TC-008`, `SA-006`; J.Y.
  `JY-016`; TXC `MGC-023`) as corroboration that a title is a retail product and not a ROM-hack
  scene release.

### What "supported" means

`SUPPORTED_MAPPERS` (`rust/crates/zax-nes/src/cartridges/registry.rs:43-137`), 69 numbers by
default: 0-5, 7, 9-11, 13, 15, 16, 18, 19, 21-28, 30, 32, 33, 34, 37, 41, 47, 48, 64-67, 69, 70,
71, 73, 75, 76, 78, 85, 87, 88, 89, 93, 94, 95, 97, 101, 105, 116, 118, 119, 140, 152, 154, 155,
156, 158, 180, 184, 185, 206, 210, 225, 228, 232.

---

## 1. Part A — §3.3's single-publisher boards (60 numbers)

Three of §3.3's eight group labels are wrong, and the census says so where the row is: **171** is
not Kaiser on its own page, **164** is Dōngdá/Yànchéng rather than Wàixīng, and **61**, **172**,
**173**, **286** and **299** are NTDEC/GS, Super Mega, Idea-Tek, Benshieng and TXC respectively —
the inventory filed four of them under the wrong publisher.

### 1.1 Sachen — 14 numbers, and the largest body of original unlicensed content in the census

| iNES | board (NESdev) | Mesen class | publisher | titles the page names | cat. |
| ---: | --- | --- | --- | --- | :-: |
| **133** | Sachen 3009 (60-pin), Sachen 72008 (72-pin); a Namco 108 clone marked "ASLIC AX-24G" | `Sachen_133` | Sachen | *Jovial Race*; *盜帥 — Master Chu and the Drunkard Hu* | **A** |
| **136** | Sachen 3011, custom IC **JV001** (marked GS63030-A) | `Sachen_136` | Joy Van / Sachen | *未来小子 — Joy Van Kid* (TC-008); *蝶變 — Incantation* (TC-009); *四川麻將 — Mahjong Trap* (TC-010); *Mahjong Companion* | **A** |
| **137** | Sachen **8259D** | `Sachen8259(8259D)` | Sachen | *The Great Wall* — "the only known game" | **A** |
| **138** | Sachen **8259B** | `Sachen8259(8259B)` | Sachen | **one page for 137/138/139/141**, which names *Q-Boy* without saying which variant | **A** (weak) |
| **139** | Sachen **8259C** | `Sachen8259(8259C)` | Sachen | as 138 | **A** (weak) |
| **141** | Sachen **8259A** | `Sachen8259(8259A)` | Sachen | as 138 | **A** (weak) |
| **143** | an NROM board with a 74138 and a 74368 (a copy-protection latch) | `Sachen_143` | Sachen (page names none) | *Dancing Blocks*; *Magical Mathematics* (NTSC only) | **A** |
| **145** | SA-72007 — "yet another CNROM clone", near-identical to mapper 87 | `Sachen_145` | Sachen | *Sidewinder* | **A** |
| **147** | Sachen 3018; **JV001** on the 60-pin, a plain latch on the 72-pin | `Sachen_147` | Sachen | *少林武者 — Chinese Kungfu* (TC-011), localised as *Challenge of the Dragon* | **A** |
| **148** | Sachen SA-008-A, **and Tengen 800008** | `Sachen_148` | Sachen, Tengen | **none.** The page names no title at all | **E** |
| **149** | SA-0036 | `Sachen_149` | Sachen | *Taiwan Mahjong 16* | **A** |
| **150** | Sachen SA-015, SA-630 (chip-on-board); an eight-register ASIC | `Sachen74LS374N` | Sachen | **14**: *영어*, *피라미드*, *将棋学園 — Chess Academy*, *Tasac*, *2-in-1 Cosmo Cop / Cyber Monster*, *2-in-1 Tough Cop / Super Tough Cop*, *臺灣 16 花牌麻将*, *Poker II*, *Strategist*, *Olympic I.Q.*, *Happy Pairs*, *Auto-Upturn*, *Magic Cube*, *Chinese Checkers*, *Poker III 5-in-1* | **A** |
| **243** | Sachen SA-020A; the same eight-register ASIC | `Sachen74LS374N` | Sachen | *美女拳 — Honey Peach* (SA-006), "used for only one game" | **A** |
| **513** | UNL-SA-9602B; an MMC3 with the CHR registers' bits 6-7 repurposed as an outer PRG bank | — (NES 2.0 only) | Sachen | *美少女夢工場三* — an unauthorised port of *Princess Maker 3* | **A** |

**Documentation**: full register tables on **136**, **147**, **150** and the 8259 page; partial on
**133** (the 72-pin half is one line, "Mask $E100, $4100: [.... .PCC]"), **243** ("registers 0, 1
and 3 have no external effect", with no account of what they do) and **513**; stubs — one register
each — on **145**, **148** and **149**. The 8259 page carries its own warning, and it governs four
numbers: *"The following information was gathered primarily from FCEU-mm and while it appears to be
a good enough behavioral description to run many ROMs, it is likely incomplete."*

**Hooks**: nothing beyond banking on 133, 136, 143, 145, 147; **nametable control** on the four
8259 numbers (the page's mirroring modes include `(0,1,1,1)` and one-screen A) and on 243;
**bus conflicts** on 148 and 149; **save RAM** on 513 (32 KiB battery-backed **CHR**-RAM, which is
not a shape this build's save path has). No IRQ anywhere in the group; no expansion audio.

**The second source.** Wikipedia's `/wiki/Sachen` independently lists *Chinese KungFu*,
*Honey Peach*, *Auto-Upturn*, *Chinese Checkers*, *Happy Pairs*, *Magic Cube*, *Poker III*,
*Strategist*, *Tagin' Dragon*, *Galactic Crusader*, *Mission Cobra*, *Jurassic Boy*,
*Street Heroes*, *Worm Visitor*, *2-in-1 Cosmocop* and *Colorful Dragon* among Sachen's
own titles, and separately frames *Super Pang*, *Super Pang II* and *Frog Adventure* as
"unofficial port[s]" of arcade originals — which this census counts as A per §0's first decision,
since no Famicom cartridge of *Pang* or *Frogger* exists. It calls Sachen "the most prolific
unlicensed publisher". Where it disagrees with NESdev it is on *which number* carries a title, not
on whether the title exists; **neither source maps the 8259 titles onto 137/138/139/141
individually**, and that is the group's one real documentation gap.

**One number is E and it is 148.** The page names no game. Mesen builds a board for it and the
board name pairs a Sachen PCB with **Tengen 800008**, which is suggestive of an unlicensed Tengen
release, but neither source this role may use names it. 148 is out under "no title".

### 1.2 Kaiser — 11 numbers, and ten of them are not original content

| iNES | board (NESdev) | Mesen class | titles | cat. |
| ---: | --- | --- | --- | :-: |
| **56** | KS202 ASIC | `Kaiser202` | *Super Mario Bros. 3* — a **reproduction** of the licensed Nintendo cartridge | **B** |
| **142** | UNL-KS7032, KS202; "an upgrade to Konami's VRC3" | `Kaiser202` | *Kaiser's SMB2J* — the page: the PCB is "used for several of Kaiser's unlicensed ports of FDS games" | **D** |
| **171** | **步步高 (BBK) Floppy-1 and SC-98 keyboard famiclones** | `Kaiser7058` | **none** | **E** |
| **175** | not named on the page | `Kaiser7022` | a Kaiser *15-in-1*, which the page calls "a pirate multicart" | **C** |
| **302** | UNL-KS7057 | `Kaiser7057` | *Gyruss* (FDS) | **D** |
| **303** | UNL-KS7017 | `Kaiser7017` | *アルマナの奇跡 — Almana no Kiseki* (FDS) | **D** |
| **305** | KS7031 | — | *ドラキュラ II 呪いの封印 — Dracula II* (FDS) | **D** |
| **306** | UNL-KS7016 | — | *Exciting Basket* (FDS), the Japanese *Double Dribble* | **D** |
| **307** | UNL-KS7037 | — | *Metroid* (FDS) | **D** |
| **312** | UNL-KS7013B | — | *Highway Star*, which the page calls a bootleg — Wikipedia's `/wiki/Rad_Racer`: *Highway Star* is the Japanese title of Square's **licensed** 1987 Famicom cartridge *Rad Racer* | **B** |
| **346** | UNL-KS7012 | — | *Zanac* (FDS) | **D** |

**Seven D, two B, one C, one E — not one A.** §3.3's own note ("mostly FDS-to-cartridge
conversions") is right, and the owner's content rule retires the whole group: an FDS conversion
preserves nothing the FDS would not run, and a reproduction of *Super Mario Bros. 3* preserves
nothing mapper 4 does not already run here.

**The mislabel worth recording: 171 is not a Kaiser board.** Its own page is about 步步高 (BBK)
Floppy-1 and SC-98 keyboard famiclones and states that *"Mapper 171 had previously been used also
for the incompatible Kaiser KS-7058 and the JY043 circuit boards"*, which were reassigned. Mesen
still builds `Kaiser7058` for it (`MapperFactory:171`), so **the two sources disagree about what
the number even is** — which is itself a reason the number is out: a board whose identity two
sources contradict has no reliable documentation, and the page names no title either way.

**Documentation**, for the record, since several of these are well documented and it is the content
rule and not the documentation that excludes them: full tables on **302**, **305**, **307** and
**312**; partial on **142** (the IRQ registers have prose, not bit layouts), **303**, **306** and
**346** (*"the actual address masks are unknown"*); stubs on **171** and **175**. **56**'s page is
a full table, and its hook is a **CPU-cycle IRQ in the KS202** plus 8 KiB of non-battery PRG-RAM.
Four of the FDS conversions (**303**, **307**, **309** below, **125** below) *"retain all writes to
the FDS sound channel registers, so an emulator could provide the expansion sound channel even
though the original Kaiser cartridge did not"* — a note a later FDS-audio lot may want, and no use
to this one.

### 1.3 Wàixīng and the numbers filed under it — 8 numbers, two of them A

| iNES | board (NESdev) | Mesen class | publisher per the page | titles | cat. |
| ---: | --- | --- | --- | --- | :-: |
| **162** | Wàixīng **FS304**; a compatible Nánjīng board, PCB code unknown | `Waixing162` | Wàixīng, Nánjīng | Wàixīng: *Mummy — 神鬼传奇*, *Zelda 传说: 三神之力*, *法老王 — Pharaoh*, *火焰纹章 — 圣战的系谱* (and the *聖火徽章 III* title-screen hack of it), *西游记后传*. Nánjīng: three title-screen hacks (*梁山英雄*, *时空斗士*, *农场小精灵*) | **A** |
| **164** | **Dōngdá PEC-9588** "Pyramid Educational Computer"; Yànchéng cy2000-3 | `Waixing164` | **Dōngdá / Yànchéng**, not Wàixīng | *Final Fantasy 太空戰士 V* (G-003), *大話西游* (G-004), *櫻桃小丸子* (G-005), *岳飛傳* (G-006), *Dark Seed — 黑暗之蛊* (G-009), *口袋精靈: 金* | **A** |
| **176** | the **8025** "enhanced MMC3" chipset — the **FK23C** | `Fk23C` | Techno Source, Wàixīng, others | *Rockman 6-in-1*, *245-in-1 Real Game*, *Super Mario 160-in-1 Funny Time*, *GameStar Smart Genius Deluxe*, *Game 500-in-1*, the *Intellivision X2* plug-and-play | **C** |
| **178** | **FS305** (Wàixīng), **NJ0430** (Nánjīng) | `Waixing178` | Wàixīng, Nánjīng, Jncota, Hengge Dianzi, GameStar/Belsonic | *宠物: 小精灵 IV*; *Ping Pong*; *Education Computer 32-in-1* | **A** |
| **242** | ET-113; UNIF `UNL-43272` | `Mapper242` | Wàixīng (one variant) | **none named.** The page describes "a menu of its own" and "replacement menu plus one or two replacement games" | **C** |
| **252** | a Konami **VRC4e clone** (= mapper 23 submapper 2) | `Waixing252` | Wàixīng | *三国志: 中原の覇者* — a Chinese localisation of a **Namcot** game | **B** |
| **253** | a Konami **VRC4e clone** | `Mapper253` | Wàixīng | *Dragon Ball Z: Kyōshū! Saiya-jin* — a Chinese localisation of the **Bandai** original | **B** |
| **286** | **Benshieng** multicart hardware; UNIF `BMC-BS-5` | `Bs5` | **Benshieng**, not Wàixīng | **none named** | **C** |

**252 and 253 are the cleanest B in the census.** Both are VRC4e clones — the same chip as mapper
**23 submapper 2**, which this build already has — and both carry a single Chinese translation of a
game whose licensed original runs on a board this build supports (*Sangokushi: Chūgen no Hasha* on
Namco's mapper **19**, *Dragon Ball Z: Kyōshū! Saiya-jin* on Bandai's mapper **16**). They would
each cost roughly one `registry.rs` arm, and they would preserve nothing.

**176 is the number §6 named as revisitable, and the content rule is what finally excludes it.**
Every title on its page is a multicart or an educational plug-and-play; the page names no original
single game. It is also the most expensive board in the group — six submappers, up to 32 KiB of
WRAM, an MMC3 scanline IRQ, a CPU-cycle IRQ in extended mode and a selectable CHR A10-12 mode —
so it is out on **C** *and* on disproportionate cost, which is two of the three remaining reasons.

**162 and 164 are A, and their hooks are the interesting part.** 162 needs a **PPU-read hook**:
its 4 KiB CHR-RAM page is selected automatically from the PPU address lines, not from a register.
164 needs a **93C66 512-byte serial EEPROM over a Microwire interface** — a third save shape, next
to the X24C01 and 24C02 the inventory's §4.6 already lists for Bandai — and implements a **1 bpp
video mode** that changes CHR addressing. Both carry battery-backed PRG-RAM.

**178's A titles and 178's exotic hooks are on different submappers**, which makes it cheaper than
it reads: *宠物: 小精灵 IV* and *Ping Pong* are plain submapper-0 cartridges, while submapper 1's
infrared-sensor **CPU-cycle IRQ** and submapper 2's **TMS5220 speech chip** belong to the
educational multicarts. The page also warns that *"two ROM images will not work when emulated using
the above description"* because their register meanings are swapped — which it attributes to
dumping errors rather than to a hardware variant — and that most educational variants want **Dendy
timing**, not NTSC.

### 1.4 The numbers filed under TXC — 7, four of them A, and three publishers mislabelled

| iNES | board (NESdev) | Mesen class | publisher per the page | titles | cat. |
| ---: | --- | --- | --- | --- | :-: |
| **36** | TXC PCB 01-22000-400; a 74138, a 74175 and the 24-pin **ASIC 05-00002-010** | `Txc22000` | TXC; Gluk for the alternate releases | *Strike Wolf* (= *Comando de Lobos*), *Policeman*, *F-15 City War*, *Volley Ball*, *Puzzle* | **A** |
| **61** | **GS-2017**, NTDEC 0324, NTDEC BS-N032 | `Mapper61` | **NTDEC and GS**, not TXC | *方塊外傳 9合1 — Tetris Family 9-in-1*, *HQ 高品質合卡 15-in-1*, *32-in-1* — all multicarts | **C** |
| **132** | TXC 01-22003-400 (72-pin), 01-22111-100, 01-22270-000 (60-pin); **TXC 05-00002-010** or a 74LS161 | `Txc22211A` | TXC | *棋王 — Chinese Chess*, *Creatom*, *小瑪琍*, *麻将方块 — Mahjong Block*, *Venice Beach Volley*, *Rad Racket — Deluxe Tennis II* | **A** |
| **172** | **Super Mega P-4070**, custom IC **JV001** | `Txc22211B` | **Super Mega**, with one TXC re-release | *1991 賭馬 Racing*, *麻将方块 — Mahjong Block*, *Venice Beach Volley* | **A** |
| **173** | a custom IC whose real number is **05-00002-010** under a fake "ITC20V8-10LP" marking | `Txc22211C` | **Idea-Tek**, not TXC | *Puzzle* (ET.01), *小瑪琍* (ET.02), *F-15 City War* (ET.03), *撲克精靈 — Poker Jīnglíng* (ET.04), *戰國四川省* (ET.05, by Computer & Entertainment) | **A** |
| **189** | TXC 01-22017-000 (60-pin), 01-22018-400 (72-pin), FC-001EM/EMC, MACP 811023, A-37/810809-C, K-3086/810836-C | `MMC3_189` | TXC, Yoko Soft | *Thunder Warrior — 轰天至尊*, *Gluk the Thunder Warrior*, *Street Fighter II: The World Warrior*, *Super Fighter II´*, *Master Fighter II*, *Master Fighter III*, *Mario Fighter III'* | **A** |
| **299** | UNIF `BMC-11160` | `Bmc11160` | TXC | the TXC *6-in-1* multicart (MGC-023) | **C** |

**This is the densest cluster of A in the census after J.Y. Company, and it is one ASIC.** 36, 132,
172 and 173 are all the **05-00002-010 / JV001** protection chip — "six registers: P, R and Output
(three bits each); S, inCrement and inVert (one bit each)", with a copy-protection read at $4100
whose value is the register incremented and inverted. Mesen's four class names say the same thing
(`Txc22000`, `Txc22211A/B/C`), and 136 and 147 in §1.1 are the *same* JV001 on Sachen boards. One
board with a handful of wiring parameters covers **six numbers** — 36, 132, 136, 147, 172, 173 —
and between them roughly **twenty original titles**. That is the single best effort-to-titles ratio
in the whole document, and §6's plan is built around it.

**189 is an MMC3 clone with original content**, which no other number in the census manages: TXC's
*Thunder Warrior* is an unlicensed original and Hummer Team's *Street Fighter II: The World
Warrior* is a port of an arcade game that never shipped on a Famicom cartridge (Wikipedia's
`/wiki/Hummer_Team`: "A port of *Street Fighter II*", 1992). Its documentation is full and covers
three board variants plus the generic pirate boards. The remaining five titles are hacks of those
two and are B on their own.

**Documentation**: full tables on **132**, **172**, **173**, **189**; partial on **36** (the PRG
side documents five separate operations at $4100-$4103 and $8000 without consolidating them),
**61** and **299**. **Hooks**: none at all on 36, 132, 172, 173 beyond the protection read — no
IRQ, no PPU hook, no save; 189 needs the MMC3's **A12 scanline IRQ**, which this build has; 172
needs **nametable control** (the mirroring bit's meaning depends on the Invert state) and 173
needs **bus conflicts** for *撲克精靈*.

### 1.5 The numbers filed under NTDEC — 7, three of them A

| iNES | board (NESdev) | Mesen class | titles | cat. |
| ---: | --- | --- | --- | :-: |
| **63** | NTDEC TH2291-3, CH-011, 82AB | `Bmc63` | multicarts only: *Powerful 250-in-1*, *Hello Kitty 255-in-1*, an 82-in-1 | **C** |
| **112** | the page names **no board**; "quite similar to iNES Mapper 206" with the registers scrambled | `Mapper112` | *Huang Di*; *San Guo Zhi — Qun Xiong Zheng Ba* | **A** |
| **174** | discrete: two **GD74LS161A** counters and one **GD74LS153** multiplexer | `Mapper174` | a 5-in-1 of *The Legend of Kage*, *The Goonies*, *Arkanoid*, *Urban Champion*, *Galaxian* + a menu | **C** |
| **193** | **NTDEC TC-112** mapper IC | `NtdecTc112` | *Fighting Hero*; *War in the Gulf* (with a Brazilian variant) | **A** |
| **221** | NTDEC **N625092** PCB; UNIF `BMC-N625092` | `Mapper221` | **none named**; "used for multicarts" | **C** |
| **290** | UNIF `BMC-NTD-03` | `BmcNtd03` | the *Asder 20-in-1*, on which the page names one game, *Benico* | **C** |
| **298** | **UNL-TF1201**, a **VRC4 clone in VRC4b configuration** | `Tf1201` | *Lethal Weapon* — the page: "actually a backport" of Konami's *Lethal Enforcers* | **A** |

**298 is the cheapest A in the census.** It is a VRC4b, which is exactly `Vrc2Vrc4`'s
`VrcVariant::Vrc4b` — this build's mapper **21/25** chip — with **one** documented difference:
*"$F003 is implemented differently than on the original ASIC: writing to $F003 only acknowledges a
pending IRQ"*. Its address lines are CPU `$02`→A0 and `$01`→A1, which the existing arm computes.
One `const` parameter on an existing board, in the manner of `McAcc`, for one game that exists
nowhere else on the NES. Its documentation grade is nonetheless **partial**, because the page is
only that difference — everything else it inherits from the VRC4 page, which is complete.

**112 and 193 are both near-206 boards.** 112 is a mapper-206 with scrambled registers and three
registers documented in full ($8000, $A000, $E000); 193 is NTDEC's TC-112 with CHR in three
registers over unequal windows, PRG at $8000-$FFFF and mirroring at **$6004**, and the page notes
"no games are known to rely on changing mirroring at run-time". This build has the 206 board
(`board-namcot108`), so both are cheap. 112's page names **no publisher** — §3.3 files it under
NTDEC and Mesen under `Unlicensed/`; the titles are Chinese-market originals either way.

**174 is the clearest C in the census and the clearest illustration of the content rule.** Its five
games are *The Legend of Kage*, *The Goonies*, *Arkanoid*, *Urban Champion* and *Galaxian* — five
licensed Famicom titles, every one of them on a Nintendo or Taito board this build already runs.
Shipping 174 would add a menu.

### 1.6 J.Y. Company — 5 numbers, and the strongest A in the census

**35, 90, 209 and 211 are one page and one chip**: the **J.Y. Company ASIC** (晶太, Jīngtài), which
the page says serves eight iNES/NES 2.0 numbers (035, 090, 209, 211, 281, 282, 295, 358) plus
386-388 and 397. Mesen agrees exactly — `MapperFactory` returns `JyCompany` for 90, 209 and 211
and `Mapper35` for 35. **91** is a different, simpler J.Y. board.

| iNES | board | Mesen class | titles | cat. |
| ---: | --- | --- | --- | :-: |
| **35**, **90**, **209**, **211** | J.Y. Company ASIC (晶太); boards include EL861226C | `Mapper35`, `JyCompany` ×3 | **15 on the page**: *Mighty Morphin' Power Rangers III* / *The Movie IV*, *Mike Tyson's Punch-Out!!*, *Shin Samurai Spirits 2*, *1998 Super 45-in-1*, *Aladdin* / *Aladdin II/III* / *Popeye II: Travels in Persia*, *Contra Spirits*, *Final Fight 3*, *Mickey Mania 7* / *China Rabbit Baby*, *Mortal Kombat II Special* / *Mortal Kombat 3 Special 56 Peoples*, *Super Aladdin: The Return of Jaffar*, *Super Mario World*, *Tekken 2* / *Battle Arena Toushinden*, *Donkey Kong Country 4* / *The Jungle Book 2*, *Tiny Toon Adventures 6* / *Porky Pig & Daffy Duck*, *Warioland II* | **A** |
| **91** | JY830623C, YY840238C, EJ-006-1 | `Mapper91` | *Street Fighter 3*, *Mortal Kombat II*, *Dragon Ball Z 2*, *Mario & Sonic 2*, *Super Fighter III*, *1995 Super HIK 4-in-1* (JY-016), *1995 Super HiK 4-in-1* (JY-017) | **A** |

**Why this is A and not C, title by title.** Thirteen of the fifteen are Hummer Team's unlicensed
NES ports of SNES, Mega Drive and arcade games — *Super Mario World*, *Aladdin* (Wikipedia:
"Based on the Capcom title released for the Super Nintendo"), *Final Fight 3*, *Tekken 2*,
*Donkey Kong Country 4* ("An unlicensed port of *Donkey Kong Country*"), *Contra Spirits*,
*Mortal Kombat II Special*, *Mickey Mania 7*, *Tiny Toon Adventures 6*, *Warioland II*,
*Shin Samurai Spirits 2*, *MMPR III*, *Super Aladdin*. **Not one of those originals ever shipped on
a Famicom cartridge**, so by §0's first decision every one of them is NES code that exists on no
other board — and this is, by a wide margin, the most-played unlicensed NES software there is.
Two titles are not A and do not need to be: *Mike Tyson's Punch-Out!!* is a pirate copy of a
licensed Nintendo cartridge (**B**) and *1998 Super 45-in-1* is a multicart (**C**).

Wikipedia's `/wiki/Hummer_Team` corroborates twelve ports title by title with "a port of …"
attributions and adds two Hummer Team **originals** the NESdev page does not name —
*Jīng Kē Xīn Zhuàn* (1992, "a Chinese language RPG") and *Titenic* (1998) — plus *Kart Fighter*
and *Somari*, which are mapper **116**'s titles and which this build **already runs**
(`board-116`, MF-6a3 step 4). That is the precedent: the owner has already paid for a board whose
only titles are Hummer Team ports, and 90's board carries thirteen more of them.

**Documentation: complete, and the most complete of any unlicensed board in the census.** Full
register tables for PRG/CHR banking modes, nametable selection, the IRQ, and the miscellaneous
registers including an **8×8→16 hardware multiplier**, over $5000-$D003.

**Hooks, and they are the price**: a **CPU-cycle IRQ *and* an A12 scanline IRQ** with a selectable
clock source (so `on_cpu_cycle`, `on_ppu_address_bus` *and* an `irq_asserted` arm), a **PPU-read
CHR latch** in the manner of the MMC4 (so `Board::ppu_read`), **nametable control serving
nametables from CHR-ROM** (so `Mapper::read_nametable` — and §4.5 of the inventory caps that at
two distinct bodies, which is the one hook here that is not free), 8 KiB of PRG-RAM, and the
multiplier. **91** is much cheaper: CHR banks, mirroring, an IRQ counter and PRG banks, with no
CHR latch and no nametable control. The page's one warning on 91 is that *"the commonly available
ROM image of Super Fighter III is a mapper hack for INES Mapper 197"* — so that title is not
evidence for 91.

### 1.7 Whirlwind Manu — 5 numbers, all D

| iNES | board | Mesen class | titles | cat. |
| ---: | --- | --- | --- | :-: |
| **40** | NTDEC 2722 PCB; submapper 1 is the NTDEC 2752 multicart PCB | `Mapper40` | *Super Mario Bros. 2* (Whirlwind Manu), *Super Mario Bros. II+* (Hey Sung), *1990 Super Mario Bros. 4* — all SMB2J conversions | **D** |
| **125** | UNL-LH32 | `Lh32` | *モンティのドキドキ大脱走 — Monty no Doki Doki Daisassō* (FDS) | **D** |
| **304** | 09-034A | `Smb2j` | *Super Mario Bros. 2* (J), *Volleyball* (J), *Zanac* (J) — all FDS | **D** |
| **309** | UNL-LH51 | — | *愛戦士ニコル — Ai Senshi Nicol* (FDS) | **D** |
| **522** | UNL-LH10 | — | *風雲 少林拳 — Fūun Shōrinken* (FDS) | **D** |

**Five D, no A.** §3.3's note ("FDS conversions") is exactly right. Their documentation is also the
weakest in the census where it matters: **304**'s PRG and IRQ registers both carry *"Mask:
Unknown"* and it has a block of "Unknown Registers" at $4042-$4055 documented only by the
requirement that reads return $FF, and **125**, **309** and **522** all say their mask is
"probably $E000". **40** alone is a full table, and its hook is a **4096-M2-cycle IRQ** from a
CD4020 13-bit counter. Out on the content rule, and three of the five would also be out on
documentation.

### 1.8 FFE — 3 numbers, and they are not retail boards

**6**, **8** and **17** are all the **Front Fareast Super Magic Card** (Magic Card 1M/2M) — copier
and development hardware, a RAM cartridge into which disk images are loaded. Mesen returns the
same class, `FrontFareast`, for all three (`MapperFactory:6,8,17`). The pages name **no game**,
because there is no game: the ROM images are, in the page's own words, *"ROM images that have been
extracted from disk images"*, carrying a **512-byte trainer** at $7000-$71FF (or $5D00/$5E00/$5F00
by submapper) that must be `JSR`'d or `JMP`'d before the reset vector. Mapper 8's page is a
duplicate of mapper 6's. The pages also say the register documentation is *"outdated"* and refer to
a separate *Super Magic Card* article for bankswitching.

**All three are E**, on both of the remaining reasons that apply: no title, and no reliable
documentation of their own. They are also the one group in the census where supporting the number
would mean supporting a **trainer-loading convention** in the loader rather than a board in the
cartridge layer. Mapper **6** is additionally one of the four numbers
`req_core_102_any_other_mapper_is_refused` sweeps (`mapper_tests.rs:69`), and this census's
recommendation is that it stay unsupported — which, with 177 and 200 below, leaves that test
standing.

### 1.9 Part A's totals

| group | numbers | A | B | C | D | E |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Sachen | 14 | **13** | 0 | 0 | 0 | 1 |
| Kaiser | 11 | **0** | 2 | 1 | 7 | 1 |
| "Wàixīng" | 8 | **3** | 2 | 3 | 0 | 0 |
| "TXC" | 7 | **5** | 0 | 2 | 0 | 0 |
| "NTDEC" | 7 | **3** | 0 | 4 | 0 | 0 |
| J.Y. Company | 5 | **5** | 0 | 0 | 0 | 0 |
| Whirlwind Manu | 5 | **0** | 0 | 0 | 5 | 0 |
| FFE | 3 | **0** | 0 | 0 | 0 | 3 |
| **total** | **60** | **29** | **4** | **10** | **12** | **5** |

**Just under half of §3.3 is category A.** The inventory's §6 excluded all sixty with one sentence;
**29** of them carry at least one game that exists on no other board this build supports, and four
numbers — **36/132/172/173** — are a single protection ASIC that also covers two of the Sachen
numbers.

---

## 2. Part B — §3.4's 83 explicitly listed numbers

### 2.0 One refinement to the category rule

**A is a promotion and B/C/D are not ranked against each other.** Any single category-A title
promotes the number to A, because that title is what shipping the board would preserve. Below A,
the number takes the category of the **bulk** of its catalogue rather than of its most unusual
cartridge — a 22-in-1 with a pirate *Contra* in a reset-selected slot is **C**, not B. Nothing
turns on the distinction: B, C and D are all out under the same owner's rule, and the rows name
the titles either way.

### 2.1 The fifteen category-A numbers of §3.4

| iNES | board (NESdev) | Mesen class | publisher | the title(s) that make it A | doc | hooks beyond banking |
| ---: | --- | --- | --- | --- | --- | --- |
| **38** | UNL-PCI556 | `UnlPci556` | **Bit Corp.** | *Crime Busters* — the page quotes **1 game in NesCartDB** | partial | none. The page warns writes to $F000-$FFFF "are very likely to also trigger a bankswitch", and the 74138's enable depends on /ROMSEL, R/W and M2 |
| **79** | **NINA-03, NINA-06** (72-pin) | `Nina03_06(false)` | **American Video Entertainment** | the page names none, but Wikipedia's `/wiki/American_Video_Entertainment` lists **19** AVE NES titles, every one of them NES-exclusive unlicensed software: *Dudes with Attitude*, *Trolls on Treasure Island*, *Deathbots*, *Krazy Kreatures*, *Tiles of Fate*, *Double Strike*, *Impossible Mission II*, *Solitaire*, *Blackjack*, *F-15 City War*, *Puzzle*, *Pyramid*, *Rad Racket*, *Mermaids of Atlantis*, *Ultimate League Soccer*, *Venice Beach Volleyball*, *Wally Bear and the NO! Gang*, *Cue Stick* (cancelled), *Maxivision 15-in-1* | **full** | none at all. A one-byte control register at $4100: 8 KiB CHR bank, 32 KiB PRG bank |
| **83** | a chip-on-board ASIC by **Cony (Yoko)**; PCBs NC-24/82-07-17J, NC-14/82-05-26J, N-C88-1603/83-08-19J, N-C44/82-08-24J | `Mapper83` | Cony / Yoko | *Street Fighter II Pro*, *Fatal Fury 2*, *World Heroes 2'*, *Super Blaster VII Turbo 28* — ports of Capcom, SNK and Neo Geo games that never shipped on a Famicom cartridge. Four multicarts beside them | **full** | an **IRQ counter with two selectable clock sources — CPU M2 *or* PPU A12 rising edge**; mirroring control; a solder-pad read at $5000; 4 bytes of scratch RAM at $5100-$5103 |
| **107** | not named; an 8-bit unified register | `Mapper107` | **Magicseries** | *Magic Dragon* | **full** | none. The page notes FCEUX implements three bits of the register and MesenCE all eight; bus-conflict behaviour undetermined |
| **144** | a Color Dreams revision, PCB **50282** | `ColorDreams` | Color Dreams | *Death Race* | **stub** | **bus conflicts, and an unusual kind**: a 300 Ω resistor between CPU D0 and the mapper means only the ROM's least significant bit wins. The page gives three alternative formulas and no register table |
| **146** | the **same NINA-03/06 circuit** as 79, as the Sachen 3015 / SA-016 60-pin chip-on-board | `Nina03_06(false)` | Sachen | **none attributed to this number by either source.** Graded A by family, and the grading is moot: it is the same board as 79 and implementing 79 implements it | **full** (the family's) | as 79 |
| **163** | **Nánjīng FC-001** | `Nanjing` | 南晶 (Nánjīng) | **7**: *牧场物语 — Harvest Moon* (NJ011), *水浒神兽* (NJ019), *暗黑破坏神 — Diablo* (NJ037), *轩辕剑外传 之 天之痕* (NJ045), *Final Fantasy IV — 最终幻想 4* (NJ098), *雷电皇 比卡丘传说* (NJ046), *真 Samurai Spirits — 武士道列传: 侍魂* (NJ-033, submapper 1) | **full** — five registers at $5000-$5300 with a feedback read | **a PPU-read hook** (the 4 KiB CHR-RAM page is selected automatically from the PPU address lines), battery-backed 8 KiB PRG-RAM, and **expansion audio on submapper 1** (an ADPCM chip) |
| **170** | not named | `Mapper170` | not named | *Shiko Game Syu*, an original unlicensed Famicom title | **stub — one sentence**, no register, no mask | unknown, because nothing is documented |
| **190** | not named | `MagicKidGooGoo` | **Zemina** | *Magic Kid Googoo* | partial | none. CHR at $A000-$BFFF, PRG at $8000-$9FFF and $C000-$DFFF; no initialisation values, no timing |
| **216** | not named | `Mapper216` | **RCM Group** | *Bonza*, *Videopoker Bonza*, *Magic Jewelry II* | **stub** | **Dendy timing** — the page says some revisions need it to display correctly, which is a console-timing concern and not a cartridge one |
| **240** | two PCB variants, neither named | `Mapper240` | not named | *荊軻新傳 — Jing Ke Xin Zhuan*, *聖火列傳 — Sheng Huo Lie Zhuan*. Wikipedia's `/wiki/Hummer_Team` independently attributes *Jīng Kē Xīn Zhuàn* (1992) to Hummer Team and calls it "a Chinese language RPG" | **stub** — "Selects 32k PRG @ $8000", "Selects 8k CHR @ $0000", two different decodings | PRG-RAM, and a heuristic to tell the two PCBs apart |
| **244** | not named | `Mapper244` | **C&E** | *Decathlon* | **stub** | none documented |
| **246** | **G0151-1** | `Mapper246` | not named | *封神榜 — Fēngshénbǎng: Fúmó Sān Tàizǐ* | partial | battery-backed PRG-RAM at **$6800-$6FFF** (not the usual window), and a **PRG A17 forced on reads from certain CPU addresses**. The page warns the game "only work[s] in Famiclones" |
| **264** | the **same Cony/Yoko page as 83**, the 2 KiB-CHR variant (83.1/264) | `Yoko` | Cony / Yoko | as 83 | **full** | as 83 |
| **521** | **DREAMTECH01** | — | not named | *장두진 바둑교실: 입문편*, a Korean Igo tutorial | **stub** | none. A bank register at $5020 (mask $F030), a fixed last bank, 8 KiB CHR-RAM |

**Six of the fifteen have a stub for documentation** — 144, 170, 216, 240, 244, 521 — and that is
where one of the three remaining exclusion reasons still bites *inside* category A. 170 is the
extreme case: one sentence, no register, no mask, nothing to implement from. §6 sorts them.

**Three clusters are worth seeing as clusters**, because they change the effort arithmetic:

* **79 + 113 + 146 + 148 are one circuit.** NESdev gives them **one page**, "NINA-003-006", and
  Mesen returns `Nina03_06` for 79, 113 and 146 (`MapperFactory:79,113,146`). The differences are
  stated on that page in one line each: 113 "extend[s] the NINA-03/06 with an extra 4 bits of
  control register" plus run-time mirroring, 146 is the 60-pin Sachen chip-on-board of the same
  circuit, 148 "relocates the latch register, introducing bus conflicts". **Four numbers, one
  board, and AVE's whole 19-title catalogue behind it.**
* **83 + 264 are one page and one ASIC** — and 487 (AVE **NINA-08**) belongs to the same discussion
  for a different reason: its register set implements *both* the NINA-03 and the Color Dreams
  bankswitching schemes, selected by a bit at $4180, "so that AVE and Color Dreams games ran
  completely unmodified". A board that is two supported boards in one.
* **163 is the Nánjīng board §6 named as revisitable, and it survives the content rule**: seven
  titles with their own catalogue numbers, including the Chinese *Harvest Moon*, *Diablo* and
  *Final Fantasy IV* — RPGs written for the Famicom and existing on no other platform's cartridge.
  Its page is a full register table. It is a genuine A.

### 2.2 The seven B numbers — bootlegs, hacks and ports of games already here

| iNES | board | the title, and what it derives from | doc |
| ---: | --- | --- | --- |
| **106** | discrete, a 74LS670 register file and ROM chips 5601/5602 | *Super Mario Bros. 3* bootleg, "modified from [the] MMC3 original, with copyright removed" — mapper **4**, which this build has | full; a 16-bit CPU-cycle IRQ. The page documents FCEUX's implementation as inaccurate on real hardware, especially on PAL |
| **117** | not named | *Crayon Shin-Chan (Ch)* and *San Guo Zhi 4 — Chi Bi Feng Yun*, both hacks/ports | partial, and the IRQ is guessed: "selects PPU scanline MINUS $14 (?)", with $A008-$A00F marked unknown |
| **183** | a VRC4e clone with extra logic | *Shui Guan Pipe*, a pirate variant of Sunsoft's *Gimmick!* — mapper **69**, which this build has | partial, plus an errata correcting an early wrong implementation |
| **222** | **CTC-31**, corporation unknown | pirate ports of Data East's *Dragon Ninja* and Taito's *Don Doko Don 2* (the latter renamed *Super Mario Bros. 8*) | **stub**, and the page says so outright: *"No better information than FCEUX's source is currently available"* |
| **266** | **UNL-CITYFIGHT** | *City Fighter IV*, which the page calls a hack of *Master Fighter II* — itself a mapper-189 hack of *Street Fighter II* | full. Its hook is remarkable and recorded for a later audio lot: a **4-bit PCM DAC at $980C** through an R-2R ladder, driven during speech while the VRC4's pseudo-scanline IRQ mode runs |
| **529** | UNL-T-230 (also YY0807, J-2148) | bootlegs of *Datach Dragon Ball Z* and *Dragon Ball Z Gaiden* | partial. A VRC4 clone plus a **93C56 EEPROM** over Microwire, whose register overlaps the VRC4's, so the game writes $FFFx to reach it |
| **530** | **UNL-AX5705** | *Super Mario Bros. Pocker Mali* (a hack) and a bootleg of Bandai's *Crayon Shin-chan: Ora to Poi Poi* | **stub**. A VRC4f clone with PRG A14↔A16 and CHR A15↔A16 swapped |

**529 is the one row where the category rule and the owner's intent pull apart, and it is worth a
sentence.** Its two titles are bootlegs of **licensed Bandai cartridges** — so they are not
original content and the number is out — but the licensed originals are the **Datach** (mapper
**157**) and the LZ93D50+X24C01 (**159**), *neither* of which this build has. The right way to
preserve those two games is §3.1's licensed Bandai row, not a bootleg board: 157 and 159 are
already in the inventory's MF-6c plan. The census's recommendation is to leave 529 out and let that
plan stand.

### 2.3 The six D numbers — FDS conversions

| iNES | board | titles | doc |
| ---: | --- | --- | --- |
| **42** | not named | *Ai Senshi Nicol*; *Mario Baby* (actually *Bio Miracle Bokutte Upa*) | full; a 15-bit counter with 24 576-cycle timing. The page: "while they likely used different hardware, they can both be encompassed by a single mapper" |
| **43** | **TONY-I**, **YS-612** | *Super Mario Bros. 2* (J); *Mr. Mary 2* (from a multicart only) | full; a 12-bit M2-cycle IRQ, register at $4022 |
| **50** | **N-32** conversion, PCB 761214 | *Super Mario Bros. 2* (J); an alt-levels *Super Mario Bros.* hack | partial. The page calls the PRG setup "bizarre" and questions whether the counter is 16-bit or "13-bit with inverted MSB" |
| **103** | discrete: three 7400, two 7410, a 7432, a 7474, a 74138, a 74157, a 74161 | *Doki Doki Panic* — "a specific pirate port of the FDS version" | full, from Krzysiobal's reverse engineering |
| **108** | **DH-08** (sub 1), **UNL-BB** (subs 2 and 4) | Whirlwind Manu conversions: *Bubble Bobble* (LH31), *Falsion* (LH54), *Meikyuu Jiin Dababa* (LH28), *Pro Wrestling* (LE05) | full per submapper, plus the FCEUX/Mesen heuristic that disambiguates the PCBs without configuration |
| **120** | **LH15**, Whirlwind Manu | *とびだせ大作戦 — Tobidase Daisakusen* | partial; one register at $41FF |

Six more FDS conversions on top of §3.3's twelve. **Eighteen of the 143 numbers in §3.3 and §3.4's
own lists exist to put an FDS disk on a cartridge**, and the owner's rule excludes every one.

### 2.4 The eight E numbers

| iNES | why | detail |
| ---: | --- | --- |
| **59** | **no title** | BMC-T3H53, BMC-D1038. A full register table *and* a jumper read, and no game named. The page records that "NESten's Mapper DLL v1.0 contained an incorrect implementation", corrected in v1.2 but not universally, so **59 and 60 are inconsistently applied across ROM files and emulators** |
| **166** | **no title** | Subor's educational-computer board, in the bank order that was later found wrong: 167's errata says 166 had "a fixed bank $07 rather than $20 in mode 0, and a non-inverted PRG A14 in modes 2 and 3" |
| **167** | **no title** | the same Subor board, correct. Full register table, four registers with XOR-combined bank selection and three layout modes. The page says only "various mini-games" and names none |
| **177** | **no game** | 恒格电子 (Hénggé Diànzǐ). Two titles, both **educational cartridges** rather than games: *Nikita PC Vivaz* and *英语发声词霸 — ABC 声霸*. Documentation is a full table, and the hooks are heavy: a **TMS5220-compatible LPC speech chip at $5000-$5FFF**, 8-32 KiB of battery WRAM, and a 16 KiB CHR-RAM mode that auto-switches on the PPU address |
| **179** | **no documentation, no title** | "a Nestopia assignment which duplicates iNES Mapper 176"; the page is a redirect notice saying 176 should be used |
| **301** | **no documentation** | **NESdev's `NES 2.0 Mapper 301` redirects to INES Mapper 242**, a different board, while Mesen builds `Bmc8157` for it (`MapperFactory:301`). The two sources name different hardware and neither documents 301 itself |
| **328** | **no game** | UNL-RT-01, whose only title is *Test Ver. 1.01 Dlya Proverki TV Pristavok* — a television test cartridge. Stub documentation, mask "unknown, possibly $BFFF", and a weak-bit random-number generator at $CE80-$CEFF |
| **329** | **no game** | the "Keyboard Transformer", a Russian cartridge that attaches an IBM PC/AT keyboard. Its titles are a modified *Family BASIC* — a licensed Nintendo product — and *Мооспирв обучающий компьютер 2000*. Stub documentation, no pinout, no protocol |

**177 is one of the four numbers `req_core_102_any_other_mapper_is_refused` sweeps**
(`mapper_tests.rs:69`), along with 6, 116 and 200. This census leaves **6**, **177** and **200**
unsupported, so the test keeps three numbers and holds unchanged; Q-130 question 2's observation
that it holds no number guaranteed never to become supported is still true and still unresolved.

### 2.5 The forty-seven C numbers

Every one is a multicart, and the owner's content rule excludes the whole column. Listed with its
board so the document is complete and so a later lot never has to re-derive the identification.

| iNES | board | the multicart(s) | doc |
| ---: | --- | --- | --- |
| **39** | oversize BNROM, Subor | *Study & Game 32-in-1*. The page says 241 provides the same thing | stub |
| **46** | **Rumble Station**, a NES-on-a-chip | *Rumblestation 15-in-1*, of licensed Color Dreams titles | partial; bus conflicts "No idea" |
| **51** | not named | *11-in-1 Ball Games*, and the JY-010/850828C variant as submapper 1 | stub |
| **53** | not named | *Supervision 16-in-1* | stub — one sentence |
| **54** | BMC-NovelDiamond9999999in1 | *Novel Diamond 9999999-in-1* | stub, **and the number is disputed**: the page says Nestopia claims 54 for this board while GoodNES 3.1.4 and FCEUX have no allocation and treat it as a UNIF dump |
| **57** | not named | *GK 47-in-1*; *6-in-1 / SuperGK* | partial |
| **58** | not named | eight: *21-in-1* (AS-5321), *50-in-1* (WQ1806 B), *55-in-1* (WQ2006 B), *68-in-1* (HKX5268), *75-in-1* (WQ1905 E), *86-in-1* (AP-5486), *92-in-1* (WQ1605 E), *97-in-1* (WQ1708 B) | partial. 213 duplicates it |
| **60** | reset-based NROM-128 4-in-1 | *Reset Based 4-in-1*, titles unnamed | stub: *"I would assume the register is 2 bits wide? Don't know for sure"* |
| **62** | not named | *Super 700-in-1* | partial, Disch's notes |
| **113** | **HES NTD-8** | *HES 6-in-1*, *Mind Blower Pak*, *Total Funpak* | full. The NINA-03/06 with four more control bits |
| **200** | **MG109** | *1993 Super 50 in 1* | partial, two submappers |
| **201** | not named | *8-in-1*; *21-in-1* (2006-CA). "All known games use only the two least significant bits" | partial |
| **202** | not named | the *150-in-1* shipped with the dual-slot SP60 famiclone | partial |
| **203** | not named | a *35-in-1* of *Duck Hunt*, *Hogan's Alley*, *Wild Gunman* and *Battle City "Tank 1990"* — four licensed Nintendo titles | partial |
| **204** | not named | *4-in-1*, *64-in-1*, *80-in-1*, *150-in-1* | stub |
| **212** | "BMC Super HiK 300-in-1" per Nestopia | the 300-in-1, titles unnamed | partial |
| **213** | **a duplicate of 58** | *9999999-in-1*; *168-in-1*. The page: "both ROM files run well as mapper 58" | stub |
| **214** | not named | *Super Gun 20-in-1* | stub — one sentence |
| **226** | discrete: 74LS74A, 74LS273, 74LS139, 74LS02, 74LS153 | *42-in-1*, *63-in-1*, *76-in-1*, *Super 42-in-1* | partial; depends on uninitialised memory for the game count |
| **227** | 810449-C-A1; **FW-01** (Wàixīng); N120-72 | *1992 Contra 120 in 1* | partial, three submappers; battery WRAM on the Wàixīng variant |
| **229** | BMC 31-IN-1 | the 31-in-1, titles unnamed | stub; Nestopia and Nintendulator disagree on the CHR width |
| **230** | not named | *22-in-1*, with a pirate *Contra* reachable by soft reset | partial |
| **231** | not named | *20-in-1* | stub |
| **233** | not named | a 42-in-1 whose table lists **32 licensed titles** (*Galaxian*, *Balloon Fight*, *Donkey Kong*, *Excitebike*, *Ice Climber*, …) | partial, **and the dump is doubted**: "clarify actual mapper behavior", "determine if ROM is valid or corrupted" |
| **234** | **Maxi 15** (AVE's *Maxivision 15-in-1*) | the 15-in-1 | **full**. Read-triggered bankswitching at $FF80-$FF9F/$FFE8-$FFF7, a lockout-defeat register, dual CNROM/NINA-03 modes, bus conflicts resolved from ROM contents |
| **235** | "Golden Game" 150-in-1 | ~60 titles, which the page says are all **hacked** | partial |
| **236** | **Realtec** 8031, 8155, 8099, 8106 | *35-in-1* (HM5511), *56-in-1* (NT-009), *68-in-1*, *70-in-1*, *智慧大全 12-in-1*, *800-in-1* | full, both CHR-ROM and CHR-RAM variants |
| **241** | BxROM with WRAM | *小霸王 2合1: 仓库世家 & 动脑筋*. The page attributes the number mostly to **Hengge** educational cartridges and to mapper hacks of other publishers' games | stub; optional TMS5220C at $5000-$5FFF |
| **255** | 110-in-1: four 512 KiB PRG ROMs, two 512 KiB CHR ROMs, a 74'273, a 74139 and a PAL16L8 | the 110-in-1 | partial; **duplicates 225**, and "the banking register boots with random contents" |
| **261** | BMC-810544-C-A1 | *14-in-1* (NTDEC); a *200-in-1* containing *Elfland* | partial |
| **265** | BMC-T-262 | T-262 multicarts, titles unnamed | partial |
| **274** | BMC-80013-B, **RCM Group** | *Cartridge Story* (72-in-1), *II* (80-in-1), *III* (90-in-1, a.k.a. *90-in-1 Hwang Shinwei*) | full |
| **283** | BMC-GS-2004 (with an 8 KiB PRG chip) and BMC-GS-2013 (without) | *Block Family 6-in-1 / 7-in-1* | stub |
| **285** | BMC-A65AS; BMC-GF-401CD (sub 1) | *A65AS*; *JY-066*; and a Russian *Highway Star* / Русские Гонки bootleg | full, both submappers |
| **288** | **GKCX1 21-in-1** | two GKCX1 21-in-1s | stub. GoodNES 3.23b files them as mapper 133, "even though it is entirely incompatible" |
| **289** | BMC-60311C | *Super HiK 17-in-1*; *1995 Champion Cassette 1200-in-1*; *Super 76-in-1 with Contra* | full |
| **300** | BMC-190in1 | *Golden 190-in-1* | partial |
| **314** | PB011003-3x1, UNIF **BMC-64in1NoRepeat** | *32-in-1*, *64-in-1*, *76-in-1* Y2K | partial; masks "Unknown, probably" |
| **319** | **HP-898F**, KD-7/9-E | six, incl. *Prima Soft 9999999-in-1* and *Olympic 2000 1000000-in-1* | partial |
| **320** | BMC-830425C-4391T | *Super HiK 6-in-1* (A-030) | full; "basically UxROM" with an outer-bank latch |
| **331** | BMC-12-IN-1 | *7-in-1* (NS03) | partial. Notable hook: **separate inner-bank registers for the $0000-$0FFF and $1000-$1FFF PPU windows**, switched to avoid mid-render conflicts |
| **332** | BMC-WS | *Super 40-in-1*; *Super 500-in-1* | partial; a solder-pad probe that checks whether $8000-$FFFF returns open bus |
| **336** | BMC-K-3046 | *11-in-1* (K-3046), *2-in-1* (YH-217), *4-in-1* (YH-4109), *Top Gun III 2-in-1*, *2-in-1* (YH-218) | full, three submappers. Submapper 1 needs **144's resistor trick** |
| **349** | BMC-G-146 | *1994 Super HIK 14-in-1* (G-136) | partial |
| **487** | **AVE NINA-08** | *Maxivision 30-in-1*, **unreleased** | **full**. 1536 KiB of PRG and of CHR, and **both** the NINA-03 and Color Dreams bankswitching schemes, selected by a bit at $4180 — "so that AVE and Color Dreams games ran completely unmodified" |
| **518** | **UNL-DANCE2000**, Subor | *小霸王 Subor 999*, *小霸王 Subor V*, *跳舞天使: 動感 2000 — Dance 2000 12-in-1* | full. And the heaviest hook list in the census: a **µPD765-compatible floppy disk controller**, an LPC speech chip in two formats, a CHR-RAM auto-bankswitch during rendering, mid-scanline nametable switching, a second PRG-RAM, and **Dendy timing only** |
| **519** | UNL-EH8813A | a Chinese-language multicart, titles unnamed | full |

### 2.6 Part B's totals

| | A | B | C | D | E | total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| §3.4's explicit list | **15** | 7 | **47** | 6 | 8 | **83** |

**Fifty-three of the 83 are multicarts, bootlegs or FDS conversions** — which is what §3.4 said
they were, and the owner's content rule confirms it independently of any performance argument.
**Fifteen are A**, and four of those fifteen (79, 146, 83, 264) are two boards, not four.
