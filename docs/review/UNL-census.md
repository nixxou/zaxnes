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

---

## 3. Part C — the 48 MMC3 clones of §3.2 (mapper 14 among them)

§3.2 filed these as "MMC3 clones and pirate extensions … **48 numbers, none with a licensed
title**", and §3.4 excluded them by reference. The licensed-title claim is correct. **Nineteen of
the 48 are nonetheless category A** — more than any other group in the census — and the reason is
structural: an MMC3 clone is what a Taiwanese or Chinese publisher put an *original* game on when
the game needed MMC3-class banking, so the clones carry the ports and the Chinese RPGs while the
pure multicart boards cluster in §3.4.

They are also the **cheapest** group to implement, because this build already has the MMC3, the
A12 watcher and the `Mmc3Board` `const`-parameter mechanism that MF-6a3 used for 119, 47 and 37.

### 3.1 The nineteen A numbers

| iNES | board (NESdev) | Mesen class | publisher | the title(s) that make it A | doc | what it adds to the MMC3 |
| ---: | --- | --- | --- | --- | --- | --- |
| **12** | **SL-5020B** (a chip-on-board variant of the **Huang-1** ASIC, with a GAL16V8); submapper 1 is the FFE Magic Card 4M | `MMC3_12` | Gouder; FFE | *Dragon Ball Z 5* (Rex Soft) | partial | 512 KiB of CHR-ROM through an external register at **$4132** |
| **14** | **UNL-SL1632** (SL-1632 PCB), the **Huang-1** again | `MMC3_14` | **Rex Soft** | *Samurai Spirits*, the 8-character version | partial, **and the page says its mirroring comes "according to Nestopia's mapper source code"** and "contradict[s] other emulators and ha[s] not been verified on original hardware" | a supervisor register at **$A131** overlapping the MMC3's own, two modes (VRC2 / MMC3, no MMC1), and **three independent CHR A18 bits** over the three PPU windows $0000-$0FFF, $1000-$17FF and $1800-$1FFF. MF-6b step 6's table |
| **74** | 43-393 / 43-406 / 860908C | `MMC3_ChrRam(0x08,0x09,2)` | **Wàixīng** | *甲A — China Soccer League for Division A*; *第四次: 机器人大战* (the Super Robot Wars IV port); *風雲 — Traitor Legend* (1997). *機甲戰士* is a *Metal Max* translation and is B on its own | **stub** | a 74LS138 and a 74LS139 redirect 1 KiB CHR banks **#8 and #9** to 2 KiB of CHR-RAM |
| **114** | an MMC3 clone with **scrambled register addresses and indices** | `MMC3_114` | **SuperGame**, **Hosenkan** | *Aladdin*, *The Lion King*, *Boogerman* (SuperGame); *Pocohontas*, *Super Donkey Kong* (Hosenkan) | **full**, both submappers | a 256 KiB outer CHR-ROM bank at $6001, an NROM-like PRG override at $6000, and two distinct scramble patterns |
| **115** | **Kǎshèng SFC-02B / -03 / -004** | `MMC3_115` | Kǎshèng, re-releasing **Gouder** | *A.V. 究極麻將 II*, *包青天*, *餓狼伝説 Special*, *雷電 II — Thunderbolt Fighting Plane*, *三國春秋: 四川省*, *幽☆遊☆白書 Final* — **six**, plus a 16-in-1 | partial | a 256 KiB outer CHR bank and an NROM-like PRG override; the $600x registers work regardless of the WRAM enable; a "sharp" IRQ (latch 0 fires every scanline) |
| **121** | **Kǎshèng A9711** (256 KiB) and **A9713** (512 KiB) | `MMC3_121` | 卡聖 (Kǎshèng) | *Street Fighter Zero 2 '97*; *The Panda Prince* / *Super Lion King 2*. Seven further titles are hacks and one is a 3-in-1 | **full**, with the behavioural quirks | a **protection array** at $5000-$5FFF; extended PRG banking at $A000/$C000/$E000 from bit-reversed $8001 values; **CHR A18 driven directly by PPU A12** on the A9711, so the two pattern tables sit in different 256 KiB halves |
| **123** | **UNL-H2288** | `MMC3_123` | Kǎshèng, with **Hummer Team** and Shin-Shin | *Mortal Kombat 3*; *Earthworm Jim 2* | **full** | an NROM-like PRG override at $5800 with a selectable 16 KiB bank, and scrambled register indices |
| **182** | **NESdev redirects `INES Mapper 182` to mapper 114's page**, which states "INES Mapper 182 is a duplicate of Mapper 114. In fact, FCEUX explicitly reassigns Hosenkan's Mapper 182 ROMs to Mapper 114" | `MMC3_182` | Hosenkan | *Pocohontos*; *Super Donkey Kong* | **full** (114's) | nothing of its own. The page's distinction is behavioural: "Hosenkan's games make no use of the $6000-$6001 registers, other than writing zero to both of them on startup" |
| **187** | **Kǎshèng A98402** | `MMC3_187` | Kǎshèng / **Hummer Team** | *Street Fighter Zero 2*; *The King of Fighters '96* | partial | an NROM-like PRG override at $5000 with protection reads, and **CHR A18 selected directly by PPU A12** for 512 KiB |
| **192** | **Wàixīng FS308** | `MMC3_ChrRam(0x08,0x0B,4)` | Wàixīng | *Ying Lie Qun Xia Zhuan* | **stub** | CHR pages **$08-$0B** become 4 KiB of CHR-RAM alongside the CHR-ROM, so the game can draw Chinese characters |
| **194** | not named | `MMC3_ChrRam(0x00,0x01,2)` | not named | *Dai-2-Ji — Super Robot Taisen*, the port | **stub** | 2 KiB of CHR-RAM at pages **$00-$01** |
| **195** | **Wàixīng FS303** | `MMC3_ChrRam(0x00,0x03,4)` | Wàixīng | *哥伦布传 — 黄金中文版* (Columbus); *天神之剑* (the *Crystalis* / God Slayer port). *天使之翼 2* is a hack and 赌神 a re-release | partial | **a GAL that changes at run time which CHR banks are RAM** rather than ROM; 32 KiB of CHR-RAM fitted with only 8 KiB of address lines wired |
| **208** | not named; the copyright notice is **Gouder**'s | `MMC3_208` | Gouder | *快打傳説 — Street Fighter IV*. *Mortal Kombat (JJ-01) (Ch)* is a copyright-stripped hack on submapper 1 | **full**, including a **256-byte protection lookup table** | PRG bank and mirroring at $4800-$4FFF and $6800-$6FFF; a protection index at $5000-$57FF and protection data at $5800-$5FFF, XOR'd against the table |
| **215** | **UNL-8237** (sub 0), **UNL-8237A** (sub 1) | `MMC3_215` | **Sugar Softec** | *Earthworm Jim 2*, *Mortal Kombat 3*, *Mortal Kombat 3 Extra 60*, *Pocahontas Part 2* — **four**, plus eight multicarts | **full** | **a selectable scrambling pattern**: $5007 picks one of eight address/data scrambles, plus outer banks and an NROM override |
| **219** | **Kǎshèng A9461** | `MMC3_219` | Kǎshèng | *Toy Story*, the port. *Super 1997 4-in-1* beside it | partial | an extended mode ($8002 bit 5) giving four independent PRG windows and pseudo-2 KiB/1 KiB CHR switches, with outer bits at $5002/$5003 for 512 KiB |
| **262** | **UNL-SHERO** | `MMC3_StreetHeroes` | **Sachen** | *Street Heroes*; *侍魂 — Samurai Spirits*. Wikipedia's `/wiki/Sachen` independently lists *Street Heroes* in Sachen's own catalogue | partial — several masks are "probably" or "unknown" | 8 KiB of CHR-RAM, **4 KiB of extra VRAM for four-screen mirroring**, and a CHR-ROM/CHR-RAM select plus high CHR bits at **$4100** |
| **263** | **UNL-KOF97** | `MMC3_Kof97` | **Rex Soft** | *Boogerman II: The Final Adventure*; *The King of Fighters '97* | **full** | scrambled address *and* data bits: A12→A0, and the written value permuted by `(V&0xD8) \| ((V&0x20)>>4) \| ((V&4)<<3) \| ((V&2)>>1) \| ((V&1)<<2)` |
| **268** | **COOLBOY**, **MINDKIDS**, and eleven more PCB codes (KT-008, YH2018A, JTH-813, GH2009_V01, SMD173C_60, KP-6022, LD622D, J-852C, …) | `MMC3_Coolboy` | many Chinese manufacturers, **and Retro-Bit / Piko Interactive** | nine Chinese RPGs — *大话水浒*, *楚留香新传*, *魔道劫*, *口袋妖怪: 叶绿* (three volumes), three *最终幻想* titles, *神奇宝贝* — and three **modern licensed reissues**: *Gravity Trooper Metal Storm*, *Holy Diver*, *Quest Forge*. Eleven multicarts beside them | partial | outer bank registers taking the MMC3 from 512 KiB to **32-64 MiB**, mixed CHR-ROM/RAM modes, GNROM-style and "weird" MMC3 variants, and a lockout |
| **292** | **UNL-DRAGONFIGHTER**, PCB BMW8544 | `DragonFighter` | **Flying Star** | *Dragon Fighter* — and the page is explicit that Natsume's **licensed** game of the same name is a different mapper, so this is not a bootleg of it | **full**, with the effective-bank arithmetic worked out | an XOR-based protection, 512 KiB of CHR-ROM, and two registers reached by **reads *and* writes at $6000-$7FFF** that drive CHR A17-A18 and the XOR values per pattern table |

**The three facts that matter for the plan:**

1. **Six of the nineteen are the MMC3 with a CHR-RAM window and nothing else** — 74, 192, 194,
   195, and (as a different shape) 262 and 12. Mesen says the same thing more compactly than any
   prose could: it builds **74, 119, 191, 192, 194 and 195 from one class**,
   `MMC3_ChrRam(first, last, size)` (`MapperFactory:74,119,191,192,194,195`), and this build
   **already has 119** — the TQROM, `board-119`, shipped in MF-6a3 step 2 as "CHR bank bit 6
   selects the CHR-RAM chip instead of the CHR-ROM chip". The same `const` parameters, with
   `first`/`last`/`size` instead of a bit test, cover five more numbers.
2. **Five more are the MMC3 with its registers scrambled and an NROM-like PRG override** — 114,
   115, 121, 123, 187, and 182 for free with 114. One board with a scramble table and an override
   register, and **sixteen original titles** between them.
3. **Nothing in this group needs a hook this build does not have.** Every one of the nineteen uses
   the MMC3's own A12 scanline IRQ, which `Mmc3` has and `irq_asserted` already dispatches. The
   exceptions are not new hooks but new *shapes*: 262's four-screen VRAM (an extra 4 KiB of
   nametable RAM — the inventory's §4.5 territory), 292's read-triggered registers at $6000-$7FFF
   (`Cartridge::cpu_read` already reaches that window), and 268's 64 MiB address space.

### 3.2 The nine B numbers

| iNES | board | the title, and what it derives from | doc |
| ---: | --- | --- | --- |
| **165** | not named | *Fire Emblem (Unl)*, which the page calls a pirate hack | **stub**. Its shape is interesting — the MMC3 with **MMC2-style CHR swapping and a CHR latch**, toggled by reading PPU $xFD0/$xFE0 through $2007, selecting 4 KiB of CHR-RAM |
| **197** | **UNL-FC-28-5027** (sub 1); Kǎshèng NT-series codes | *Super Fighter III* (two submappers), *Mortal Kombat III Special* (NT-328/YY-030) — all hacks — and a *1995 Super 2-in-1* | **full**, four submappers. Raises CHR granularity from 1/2 KiB to 2/4 KiB for 512 KiB |
| **198** | not named | *吞食天地: 三国外传*, a Chinese translation of Capcom's *Destiny of an Emperor II* | **stub**. 8 KiB CHR-RAM, 8 KiB battery WRAM at $6000-$7FFF **and 4 KiB more at $5000-$5FFF**, across two PRG chips |
| **199** | **FS309** | *成吉思汗* (a *Genghis Khan* translation), *風色幻想* (a *Square's Tom Sawyer* translation), a *吞食天地* re-release | partial. Over 512 KiB of PRG, and **32 KiB of battery PRG-RAM of which 12 KiB are mapped at $5000-$7FFF** |
| **238** | not named | *Contra Fighter*, "a hack of *G.I. Joe* with Street Fighter II characters" | **stub — the page is that one sentence.** MF-6b step 6 left it out for exactly this reason: "a board cannot be written from it" |
| **245** | **Wàixīng F003** | *勇者斗恶龙 VII*, a Chinese *Dragon Quest IV* | **full**. Repurposes the MMC3's CHR outputs as high PRG lines for **1 MiB**, with 8 KiB battery WRAM and 8 KiB unbanked CHR-RAM |
| **250** | not named, by **Nitra** | *Time Diver Avenger*, *Queen Bee V* — pirate hacks | **stub**. Uses **A10 instead of A0** to pick the register and **A7..A0 instead of D7..D0** as the value |
| **254** | not named | *Pikachu Y2K* (a hack); the number is itself a pirate-hack variant of *Felix the Cat* | **stub**, marked TODO. A PRG-RAM read XOR'd with $A001 until the game touches $8000 |
| **325** | **UNL-MALISB** | *Super Mali Splash Bomb*, a bootleg hack of *Keioppi to Keroriinu no Splash Bomb* | **stub**. CPU A0 routed to A3 (and A2 when A14 is set), PRG bank bits 2↔3 and CHR bank bits 1↔5 swapped |

**245 and 199 are worth one sentence each, because they are the strongest "nearly A" rows in the
census.** Both are Wàixīng boards whose only titles are *translations* of licensed Famicom RPGs —
*Dragon Quest IV* and *Genghis Khan* — so they are out on the content rule. But they are also the
two most capable MMC3 clones here (1 MiB of PRG; 32 KiB of battery save with 12 KiB mapped), and if
a later owner decision ever values Chinese-language translations, these two and 74's *Metal Max*
are where to look first.

### 3.3 The fifteen C numbers

| iNES | board | the multicart(s) | doc |
| ---: | --- | --- | --- |
| **44** | not named | *Super Big 7-in-1* | **full**; a block register at $A001, six 128 KiB blocks and one 256 KiB |
| **45** | **GA23C** ASIC multicart PCB | *Super New Year Cart 15-in-1* | partial; four outer-bank registers at $6000 with AND/OR masks, DIP or address-line menu select |
| **49** | not named | *Super HIK 4-in-1* | partial. Its one notable rule: "when the 'O' mode bit is clear, ordinary MMC3 PRG regs are ignored" and a 32 KiB page is swapped in instead |
| **52** | **Realtec 8213** and variants; **AB892** (sub 14) | *"Mario Party" 7 in 1*, *AB-128 Well 8-in-1*, *AB-134* | partial; an outer register at $6000-$7FFF driving PRG and CHR A17-A19 |
| **126** | **TEC9719**, **ING003C**, **ING-022** ASICs — **one page shared with mappers 422 and 534** | *Power Joy Classic TV Game 84-in-1*; *Gamezone 118-in-1* | **full** across the three numbers; four outer registers at $6000-$7FFF for up to 4 MiB |
| **134** | **T4A54A**, WX-KB4K, BS-5652 | *2-in-1 Family Kid & Aladdin 4*, two *Super Cool Boy 4-in-1*s (CB-4034 hacked, CB-4035), *Super 4-in-1 1998* (JH-274), *1998 HIGH 4-in-1* (YH-463) | partial; outer banks at $6001/$6002 and a mode register at $6000 with a CNROM mode |
| **191** | not named | *赌神* — itself a hack of Technos' *Sugoro Quest* — plus three unnamed bonus games, as a 4-in-1 | partial; dual CHR-ROM plus 2 KiB CHR-RAM, outer bank at **$90AA** |
| **205** | **BMC-JC-016-2** | *15-in-1*; *3-in-1* | partial; game select at $6000-$7FFF partitions the ROM into blocks the MMC3 then banks inside |
| **217** | not named | a *500-in-1* and a *2000-in-1* | **stub — one sentence**, no technical content at all |
| **260** | **BMC-HPxx**, BMC-HP2018-A | *KY6009 6-in-1* | partial; NROM-128/256, CNROM and MMC3 modes with base registers at $5001/$5002, a DIP and a lock. The page notes **no available ROM image uses the CNROM mode** |
| **287** | **BMC-411120-C**, **BMC-K-3088** | 4-in-1s (411120-C, 811120-C) | **stub**; an outer register at $6000-$7FFF reachable only while MMC3 $A001 bit 7 enables WRAM — the same gate `board-mmc3-multicart` already implements for 47 and 37 |
| **313** | **BMC-RESET-TXROM** | nine: *1995/1997 Super HiK 4-in-1* (JY-008, JY-024, JY-040, JY-044, JY-047), *4-in-1* (CK-015), *Mario Series 4-in-1* (JY-013), *Sheng Tian Gao K 4-in-1*, *4-in-1* (A01-1) | partial, five submappers. **Reset-based**: "when an interruption of the M2 signal is detected, the higher address lines are automatically incremented" |
| **333** | **BMC-8-IN-1**, BMC-NEWSTAR-GRM070-8IN1 | *New Star Super 8-in-1* | **stub**; a 128 KiB outer bank and an NROM-256-or-MMC3 mode switch at $9000-$9FFF |
| **348** | **BMC-830118C** | *1994 New Series Red Pig 7-in-1* | partial; outer bank at $6800 driving A17-A18, with a GNROM-like mode when both are set |
| **366** | **GN-45** | *4-in-1* (K-3131GS); *4-in-1* (K-3131SS) | partial, the outer register's mask "unknown"; a lock so the games' WRAM accesses do not disturb it |

### 3.4 The five E numbers

| iNES | why | detail |
| ---: | --- | --- |
| **196** | **no title** | the **MRCM** board. Its whole documented content is a copy protection: the MMC3 clone's A0 input is rerouted to A1, A2 or A3 by submapper, and submappers 0 and 3 add mapper-189-style PRG banking after a $6000-$7FFF write. The page names no game |
| **224** | **no title** | the **KT-008** PCB by 晶科泰 (Jncota). A stub whose only technical statement is that it "includes the same chipset as the 'MINDKIDS' multicarts" — i.e. it points at **268** and stops |
| **249** | **no title** | the **T9552** Wàixīng chip. Partial documentation of a register at $5000 selecting one of four PRG and one of eight CHR address-bit **scrambles**, and no game named to test any of them against |
| **258** | **no documentation, no title** | the page in full: mapper 258 "has been tentatively reserved by the FCEUX developers to mean the same hardware as UNIF MAPR 158B". No board, no publisher, no title, no register |
| **259** | **no documentation, no title** | the same shape: "tentatively reserved … to mean the same hardware as UNIF MAPR F-15, an MMC3 multicart" |

### 3.5 Part C's totals

| | A | B | C | D | E | total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| §3.2's MMC3-clone row | **19** | 9 | 15 | 0 | 5 | **48** |

**Nineteen A, and §3.2's own claim that none has a licensed title is exactly why.** These are the
boards unlicensed publishers used for games that needed real banking, so they carry the ports
(*Aladdin*, *The Lion King*, *Boogerman*, *Earthworm Jim 2*, *Mortal Kombat 3*, *Toy Story*,
*Street Fighter Zero 2*, *KOF '96*, *KOF '97*, *Samurai Spirits*) and the Chinese RPGs, while the
pure compilation hardware sits in §3.4. They are also the cheapest group to build: the MMC3, the
A12 watcher and the `Mmc3Board` `const`-parameter pattern are all already here, and **six of the
nineteen are one Mesen class that this build already implements one member of** (mapper 119).

---

## 4. Part D — the eight homebrew numbers

§3.1's note lists 29, 31, 218, 284, 323, 324 and 682 as "the remaining `Mappers/Homebrew/`
numbers", and §3.1's table has **111** (Cheapocabra). Homebrew is where the owner's content rule is
least ambiguous: a homebrew cartridge is **nothing but** original content — there is no licensed
original anywhere behind it, so no number here can be B, C or D. The only questions are whether a
number has a **title** and whether it has **documentation**, and they split the eight four and four.

| iNES | board | author | titles | doc | cat. |
| ---: | --- | --- | --- | --- | :-: |
| **29** | **RET-CUFROM revD** | **Sealie Computing** | *Glider* | **full** — register layout, PRG/CHR setup diagrams **and the schematic** | **A** |
| **111** | **Cheapocabra (GTROM)** | **Membler Industries**, who also make the cartridges | **seven**: *Anguna: Scourge of the Goblin King*, *Candelabra: Estoscerro*, *The Incident*, *NEScape*, *Project Blue*, *Spook-o-tron*, *Storied Sword* | **full** — registers, bank configurations, a hardware teardown with the logic functions, and schematic references | **A** |
| **218** | **Magic Floor** | homebrew, author not named on the page | *Magic Floor*; *Starfight* | **full** — four configurations, each with its PPU-address-line wiring and its effect | **A** |
| **284** | **UNL-DRIPGAME**, an FPGA mapper | **Quietust** | *Drip* | the mapper-284 page is a **stub** that says "See: UNIF/UNL-DripGame" — and **that** page is comprehensive: memory map, every readable and writable register with bit layouts, the extended attribute tables, the IRQ, the audio | **A** |
| **31** | an NSF-to-cartridge board, no PCB name | various | *2A03 Puritans*, *Famicompo Pico*, *RNDM*, *EZNSF*, *ZENSF*, "plus test ROMs" — **chiptune compilations and NSF players, not games** | **full** — PRG bank register at $5FF8-$5FFF, eight 4 KiB banks, the high bank initialised to the last at power-on | **E** |
| **323** | **FARID_SLROM_8-IN-1** | Farid | **none named** | partial — an outer bank at $6000-$7FFF (mask $E000, write-only, gated on the MMC1's WRAM enable) over a stock MMC1 | **E** |
| **324** | **FARID_UNROM_8-IN-1** | Farid | **none named** | partial — a UNROM latch plus an outer bank for 128 KiB, with a lock, a D7-transition clock generator, and a power-up rule that zeroes the middle four bits and preserves the outer four | **E** |
| **682** | **Rainbow**, an FPGA mapper | **Broke Studio** | **none named on the wiki** | **stub** — a brief reference to an external GitHub document and nothing else. Its feature list is scanline IRQs, cycle IRQs, read triggers, single-screen mirroring, a large PRG-RAM, the whole of $4020-$5FFF, **and Wi-Fi**. The page: "Everything but Rainbow's Wi-Fi functionality is supported by mainline Mesen" | **E** |

### The four A numbers, and what each actually costs

* **218 (Magic Floor) is the cheapest board in the entire census, and it is cheap because it has no
  CHR memory at all.** Its 2 KiB of internal nametable RAM *is* its CHR-RAM, and the four
  configurations differ only in which PPU address line CIRAM A10 is wired to — A10, A11, A12 or
  A13. That is a nametable-mapping variant, which this build expresses through
  `Mapper::nametable_page`, plus a pattern-table read that comes out of CIRAM. The inventory's §3.1
  note already picked it out as "the only [homebrew number] with a hook worth naming"; the hook is
  §4.5's, and §4.10 prices `Mapper::nametable_page` at **0** behind its `nametable_pages_vary`
  latch. Two titles.
* **29 (RET-CUFROM) is the second cheapest**: a single register at $8000 holding two bits of
  CHR-RAM bank and three of PRG bank, hard-wired vertical mirroring, 8 KiB of WRAM in the standard
  window. Four 8 KiB CHR-RAM banks and eight 16 KiB PRG banks. The flash-reprogramming path (EXP0
  and R/W both low) is a **CopyNES** feature, not something a game uses, so it need not be
  emulated. One title.
* **111 (Cheapocabra) is the one homebrew board with a real catalogue — seven commercial homebrew
  releases — and it has the most unusual save path in the census.** It has no PRG-RAM: the
  **SST39SF040 flash chip is self-flashed by the running game**, so a save means recognising the
  flash command sequences and persisting a sector of what the loader thinks is ROM. Beside that,
  bit 5 of the control register selects between **two 8 KiB CHR-RAM pages used as nametable RAM**
  (with four-screen mirroring forced and no alternative), and bits 6-7 drive two LEDs which the
  page itself says are "currently universally unsupported" and which this build can ignore
  without consequence. The flash path is the cost; everything else is a latch.
* **284 (Drip) is an A that the third exclusion reason reaches**, and it is the clearest case of
  disproportionate cost in the census: one game, and the board wants a cycle-countdown IRQ with a
  reload buffer, **two FIFO-buffered expansion-audio channels** with independent period and volume,
  an SRAM write-enable flag — and **extended attribute tables giving per-tile palette selection**.
  That last one is not a cartridge feature at all: it changes what the PPU fetches for attributes,
  which is SPEC-06 territory and `zax-ppu`'s hot loop. One homebrew title does not justify
  reaching into the renderer.

### The four E numbers, and why three of them are nearly free anyway

**323 and 324 are Farid's homebrew 8-in-1 multicarts** — an MMC1 and a UNROM with an outer bank
register — and they would each cost about what `board-mmc3-multicart` cost for 47 and 37 (an outer
register gated on a WRAM-enable bit, which `Mmc3`'s version of that gate already demonstrates).
They are out because **neither page names a single title**, so there is nothing to say they
preserve; and because an 8-in-1 of unnamed homebrew is a compilation by construction.

**31 is the interesting E.** Its documentation is a *full* table and the board is three lines — a
4 KiB PRG bank register at $5FF8-$5FFF with the top bank fixed at power-on. It is out because its
titles are **not games**: *2A03 Puritans* and *Famicompo Pico* are chiptune compilations and
*EZNSF*/*ZENSF* are NSF players. Preserving them is preserving music, which `profiles.json` has
nothing to do. **This is a judgement the owner may want to overturn cheaply**, and the census
records the price so the decision has a number on it: mapper 31 is an S, it needs no hook at all,
and §6 lists it as the one E that a later lot could pick up for almost nothing. It is *not*
proposed here.

**682 (Rainbow) is out on documentation and on cost together.** The wiki page is a stub; the real
specification is an external vendor document on GitHub, which is public but is a moving target for
an FPGA development board whose feature list includes Wi-Fi. Mesen's own position — "everything but
Rainbow's Wi-Fi functionality is supported by mainline Mesen" — is a statement about an upstream
project with an FPGA author in the loop, not a specification this role can build against. No title
is named on the page either.

---

## 5. The whole census in one table

| group | numbers | **A** | B | C | D | E |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| §3.3, the single-publisher boards | 60 | **29** | 4 | 10 | 12 | 5 |
| §3.4, the explicit pirate/BMC list | 83 | **15** | 7 | 47 | 6 | 8 |
| §3.2's MMC3-clone row | 48 | **19** | 9 | 15 | 0 | 5 |
| the homebrew numbers | 8 | **4** | 0 | 0 | 0 | 4 |
| **total** | **199** | **67** | **20** | **72** | **18** | **22** |

**Sixty-seven of the 199 numbers are category A** — a third of them — against §6 item 1's blanket
exclusion of all of them. The other 132 are out under the owner's content rule (110: 20 B, 72 C,
18 D) or for want of a title or documentation (22 E), and **none** of those exclusions rests on a
performance argument.

### The one thing every MF-UNL lot has to live with: there is no ROM to run

A header scan of all **334** files in `assets-free/test-roms` finds these mapper numbers and no
others:

    0 (225 files), 1 (32), 2 (4), 3 (10), 4 (28), 5 (4), 7 (3), 9 (1), 10 (2), 11 (6),
    22 (1), 28 (4), 34 (3), 66 (1), 69 (2), 78 (1), 118 (1), 180 (2)

**Every one of those eighteen numbers is already supported, and not one of the 199 census numbers
appears.** The inventory's §2 said this in prose ("there is nothing to run, for any of these
boards"); it is still true, it is now counted, and it is the binding constraint on the plan rather
than a footnote. Consequences §6 takes as given:

1. **No MF-UNL lot can have a ROM-level acceptance gate.** Its gate is the unit tests written from
   the board's own page, plus the four sweeps over `SUPPORTED_MAPPERS` that every MF lot runs, plus
   the save-state round-trip. That is what MF-6a3 did for mapper 116, which also had no ROM.
2. **The per-board tests must come from the page's register table, not from a game's behaviour.**
   Which is why the documentation grade in §§1-4 is not bookkeeping: a board graded "stub" has
   **nothing to write a test from**, and that — not its runtime cost — is what keeps it out.
3. **R0's commercial-title list for each lot is a list of titles R0 would have to source
   separately.** §6 gives it per lot anyway, because it is the only way to say what the lot is for.

---

## 6. The category-A roster: documentation, hooks, effort

Sixty-seven numbers, and the first thing to say about them is that **they are not 67 boards**.
Collapsing the numbers that NESdev documents on one page and that Mesen builds from one class
leaves **44 distinct boards**: **nine families covering 32 numbers, and 35 singletons.**

### 6.1 The nine families

| family | numbers | why they are one board | effort for the whole family |
| --- | --- | --- | :-: |
| **Sachen 8259** | 137, 138, 139, 141 | one NESdev page; Mesen's `Sachen8259(Sachen8259Variant::A/B/C/D)` — **one class, one enum parameter** | **M** |
| **The JV001 / TXC 05-00002-010 protection ASIC** | 36, 132, 136, 147, 172, 173 | the same custom IC on six boards from four publishers. Mesen: `Txc22000`, `Txc22211A/B/C`, `Sachen_136`, `Sachen_147` — five classes for one chip with different wirings and a reversed data-bit order on 172 | **M** |
| **NINA-03 / NINA-06** | 79, 146 | one NESdev page ("NINA-003-006"); `Nina03_06` for both. 113 (C) and 148 (E) are the same circuit | **S** |
| **Cony / Yoko COB ASIC** | 83, 264 | one page; 264 is the 2 KiB-CHR variant (83.1) | **M** |
| **J.Y. Company ASIC** | 35, 90, 209, 211 | one page; `JyCompany` for three of them and `Mapper35` for the fourth. The page says the chip serves eight numbers | **L** |
| **Sachen's eight-register ASIC** | 150, 243 | Mesen builds **one class**, `Sachen74LS374N`, for both | **M** |
| **MMC3 + a CHR-RAM window** | 74, 192, 194, 195 | Mesen: `MMC3_ChrRam(first, last, size)` — **and this build already ships one member of it**, mapper **119** (`board-119`, MF-6a3 step 2) | **S** |
| **MMC3 + scrambled registers + an NROM-like PRG override** | 114, 115, 121, 123, 182, 187 | the same two additions on six Kǎshèng/SuperGame/Hosenkan/Gouder boards; NESdev redirects 182 to 114's page | **M** |
| **Huang-1** | 12, 14 | the same ASIC as mapper **116**, which this build already has as `Somari` (`board-116`, MF-6a3 step 4) | **M** |

**A family is one feature and one code unit, with the differences as parameters** — which is what
`BOARD-FEATURES.md` note 1 means by "the granularity is the granularity the measurements exist at",
and what `board-small-boards` (nine boards, one `Board` variant) and `board-namcot108` (five
numbers, four parameters) already do. §6.2's roster nonetheless gives **115, 121 and 187 their own
rows** inside the MMC3-scramble family, because each adds something beyond the shared scramble and
override — 115 a "sharp" IRQ rule, 121 a protection array, 121 and 187 a CHR A18 driven from PPU
A12 — and those extras are what makes the family an M rather than an S. The other 35 A numbers are
singletons: no other number shares their page or their Mesen class.

### 6.2 The roster

Effort scale, anchored on lots this project has actually run:

* **S** — a `const` or `bool` parameter on a board already here, or a new board with one register
  and no hook. The shape MF-6a3 step 1 used for 119/47/37 and MR-4-3 for `McAcc`. **≈1-2 h.**
* **M** — a new board with its own register set, at most one hook this build already has.
  MF-6b's mappers 65, 67 and 73 were this. **≈3-5 h.**
* **L** — a new board with a hook or a save shape this build does **not** have, or more than one
  chip. MF-6a3 step 4's mapper 116 was this. **≈6 h or more.**

Hook abbreviations follow the inventory's §4: **cyc** CPU-cycle IRQ (§4.1), **a12** A12 scanline
IRQ (§4.3), **ppu** a PPU-read hook or CHR latch (§4.4), **nt** nametable control (§4.5),
**aud** expansion audio (§4.8), **sav** a save path (§4.6), **bus** bus conflicts (§4.7).
"(have)" marks a hook already built and already free to boards that do not use it.

| # | board | doc | hooks | effort | what it preserves |
| ---: | --- | --- | --- | :-: | --- |
| **79, 146** | NINA-03/06 (AVE; Sachen 60-pin) | **full** | none | **S** | AVE's 19-title catalogue |
| **74, 192, 194, 195** | MMC3 + a CHR-RAM window | 74/192/194 **stub**, 195 partial | a12 (have) | **S** | 6 Wàixīng/Chinese originals and ports |
| **114, 182** | MMC3, scrambled + NROM override | **full** | a12 (have) | **S** | *Aladdin*, *The Lion King*, *Boogerman*, *Pocohontas*, *Super Donkey Kong* |
| **123** | UNL-H2288 | **full** | a12 (have) | **S** | *Mortal Kombat 3*, *Earthworm Jim 2* |
| **263** | UNL-KOF97 (Rex Soft) | **full** | a12 (have) | **S** | *KOF '97*, *Boogerman II* |
| **298** | UNL-TF1201 (VRC4b, $F003 differs) | partial | cyc (have) | **S** | *Lethal Weapon* |
| **112** | a mapper-206 with scrambled registers | **full** | none | **S** | *Huang Di*, *San Guo Zhi — Qun Xiong Zheng Ba* |
| **193** | NTDEC TC-112 | **full** | nt (have) | **S** | *Fighting Hero*, *War in the Gulf* |
| **189** | TXC 01-22017/22018 (MMC3 + $4100 PRG) | **full** | a12 (have) | **S** | *Thunder Warrior*; *Street Fighter II: The World Warrior* |
| **38** | UNL-PCI556 (Bit Corp.) | partial | none | **S** | *Crime Busters* |
| **107** | Magicseries | **full** | none | **S** | *Magic Dragon* |
| **144** | Color Dreams 50282 | **stub** | bus (have) | **S** | *Death Race* |
| **145** | Sachen SA-72007 | **stub** | none | **S** | *Sidewinder* |
| **149** | Sachen SA-0036 | **stub** | bus (have) | **S** | *Taiwan Mahjong 16* |
| **143** | Sachen NROM + 74138/74368 | **stub** | none | **S** | *Dancing Blocks*, *Magical Mathematics* |
| **190** | Zemina | partial | none | **S** | *Magic Kid Googoo* |
| **29** | RET-CUFROM revD (Sealie) | **full**, with schematic | none | **S** | *Glider* |
| **218** | Magic Floor | **full** | nt (have) | **S** | *Magic Floor*, *Starfight* |
| **12** | SL-5020B (Huang-1 COB) | partial | a12 (have) | **S** | *Dragon Ball Z 5* |
| **170** | not named | **stub, one sentence** | unknown | — | *Shiko Game Syu* |
| **216** | not named (RCM Group) | **stub** | Dendy timing | — | *Bonza*, *Videopoker Bonza*, *Magic Jewelry II* |
| **240** | two unnamed PCBs | **stub** | sav | — | *Jing Ke Xin Zhuan*, *Sheng Huo Lie Zhuan* |
| **244** | not named (C&E) | **stub** | none | — | *Decathlon* |
| **521** | DREAMTECH01 | **stub** | none | — | *장두진 바둑교실* |
| **137-139, 141** | Sachen 8259 A/B/C/D | partial (FCEU-mm derived) | nt | **M** | *The Great Wall*, *Q-Boy* |
| **133** | Sachen 3009 / 72008 (108 clone) | partial | none | **M** | *Jovial Race*, *Master Chu and the Drunkard Hu* |
| **150, 243** | Sachen's eight-register ASIC | 150 **full**, 243 partial | nt | **M** | **15** Sachen titles |
| **36, 132, 136, 147, 172, 173** | the JV001 / 05-00002-010 ASIC | 132/172/173/136/147 **full**, 36 partial | nt (172), bus (173) | **M** | **~20** TXC, Idea-Tek, Super Mega and Sachen originals |
| **83, 264** | Cony / Yoko COB ASIC | **full** | cyc **or** a12, selectable (both have) | **M** | *Street Fighter II Pro*, *Fatal Fury 2*, *World Heroes 2'*, *Super Blaster VII* |
| **91** | JY830623C / YY840238C / EJ-006-1 | **full** | a12, cyc (have) | **M** | *Street Fighter 3*, *Mortal Kombat II*, *Dragon Ball Z 2*, *Mario & Sonic 2* |
| **115** | Kǎshèng SFC-02B/-03/-004 | partial | a12 (have), a "sharp" IRQ rule | **M** | **6** Gouder originals |
| **121** | Kǎshèng A9711 / A9713 | **full** | a12 (have), **ppu** (A9711 drives CHR A18 from PPU A12) | **M** | *Street Fighter Zero 2 '97*, *The Panda Prince* |
| **187** | Kǎshèng A98402 | partial | a12 (have), **ppu** (same A18 trick) | **M** | *Street Fighter Zero 2*, *KOF '96* |
| **208** | Gouder, with a 256-byte protection table | **full** | a12 (have) | **M** | *快打傳説 — Street Fighter IV* |
| **215** | UNL-8237 / 8237A (Sugar Softec) | **full** | a12 (have) | **M** | *Earthworm Jim 2*, *Mortal Kombat 3*, *MK3 Extra 60*, *Pocahontas Part 2* |
| **219** | Kǎshèng A9461 | partial | a12 (have) | **M** | *Toy Story* |
| **262** | UNL-SHERO (Sachen) | partial | a12 (have), **nt** (4 KiB extra VRAM, four-screen) | **M** | *Street Heroes*, *侍魂 — Samurai Spirits* |
| **292** | UNL-DRAGONFIGHTER (Flying Star) | **full** | a12 (have), registers on **reads** at $6000-$7FFF | **M** | *Dragon Fighter* |
| **14** | UNL-SL1632 (Huang-1) | partial, **mirroring unverified** | a12 (have) | **M** | *Samurai Spirits*, the 8-character version |
| **162** | Wàixīng FS304 | **full** | **ppu**, sav | **M** | *Mummy*, *Pharaoh*, *西游记后传*, the *Fire Emblem* port |
| **246** | G0151-1 | partial | sav (an unusual window) | **M** | *封神榜* |
| **163** | Nánjīng FC-001 | **full** | **ppu**, sav; **aud** on submapper 1 | **M** / **L** | **7** Nánjīng RPGs incl. the Chinese *Harvest Moon*, *Diablo*, *FF IV* |
| **178** | Wàixīng FS305 / Nánjīng NJ0430 | **full** | sav, nt; **cyc** and **aud** on submappers 1-2 | **M** / **L** | *宠物: 小精灵 IV*, *Ping Pong* |
| **35, 90, 209, 211** | the J.Y. Company ASIC | **full** | **cyc + a12 selectable, ppu (CHR latch), nt (from CHR-ROM)**, a multiplier, sav | **L** | **13** Hummer Team ports incl. *Super Mario World*, *Aladdin*, *Final Fight 3*, *Tekken 2*, *Donkey Kong Country 4* |
| **111** | Cheapocabra (GTROM) | **full** | **sav — self-flashing PRG**, nt (CHR-RAM pages), forced four-screen | **L** | **7** commercial homebrew releases |
| **164** | Dōngdá PEC-9588 | **full** | **sav — a 93C66 Microwire EEPROM**, nt, a 1 bpp video mode | **L** | 6 Dōngdá titles incl. the *FF V* port and *Dark Seed* |
| **513** | UNL-SA-9602B (Sachen) | partial | a12 (have), **sav — 32 KiB battery CHR-RAM** | **L** | the *Princess Maker 3* port |
| **268** | COOLBOY / MINDKIDS | partial | a12 (have), **32-64 MiB address space** | **L** | 9 Chinese RPGs + 3 Piko reissues |
| **284** | UNL-DRIPGAME (Quietust) | **full** (on the UNIF page) | cyc, **aud (two FIFO channels)**, **nt + extended attribute tables** | **L** | *Drip* |

### 6.3 What the roster says

**Twenty-four of the 67 are S**, and together they need no hook this build does not already have.
**Five carry no effort estimate at all** — 170, 216, 240, 244, 521 — because there is nothing to
estimate: §7 excludes them on documentation. The remaining 38 are M or L.

**"Stub" is doing two different jobs in the doc column, and the roster has to be read with that in
mind.** Seven S numbers are graded stub — 74, 143, 144, 145, 149, 192, 194 — and every one of them
is a **stub that is complete for its board**: 74/192/194's page documents exactly the one thing
that differs from the MMC3 (which 1 KiB CHR pages become RAM, and how many), and 143/144/145/149's
documents the one register the board has, which is all the board is. A page can be a stub and still
be sufficient when the board is a single latch. The five numbers with **no** estimate are the other
kind: 170's page is one sentence with no register and no mask, and 216/240/244/521's give a title
and a sentence of banking with no mask, no power-on state and no protection behaviour. The test
§7 applies is not "is the page long" but **"can a unit test be written from it"** — which, with no
ROM in `assets-free/test-roms` for any of these boards (§5), is the only verification a lot gets.

**The hook ledger is short, and that is the census's most useful single finding.** Across all 67 A
numbers, exactly **five** hooks appear that this build does not already have:

| the hook | numbers that need it | what it is |
| --- | --- | --- |
| a **self-flashing PRG** save | 111 | the game rewrites its own flash; a save is a persisted ROM sector |
| a **serial EEPROM over Microwire** | 164 | a 93C66; a third EEPROM shape beside the X24C01 and 24C02 the inventory's §4.6 already plans for Bandai |
| **battery-backed CHR-RAM** | 513 | 32 KiB on the CHR side, which `CartridgeMemory`'s save path does not model |
| **two FIFO expansion-audio channels** | 284 | a new generator in the mixer |
| **extended attribute tables** | 284 | per-tile palettes — a change to what the **PPU** fetches, not to the cartridge |

Everything else the 67 need — the A12 scanline IRQ, the CPU-cycle IRQ, the CHR latch, nametable
control, bus conflicts, PRG-RAM saves — is built, latched and priced at **0** to the boards that do
not use it (§4.10). **Two of the five new hooks belong to a single board each**, and one of those
two (284) is excluded on cost.

**Expansion audio appears three times and is optional every time**: 163 submapper 1 (ADPCM), 178
submapper 2 (a TMS5220 speech chip) and 284. On 163 and 178 the A titles are on submapper 0, so
both boards can ship without their audio chip and still preserve everything that made them A — a
point §7 builds two lots on.

---

## 7. The plan of the MF-UNL lots

### 7.1 Conventions, and the three things every lot does

**Effort scale**, re-anchored on what this project's lots have actually delivered — MF-6a shipped
**nine** small boards in one lot and MF-6b **five** IRQ boards, so the units are smaller than a
first reading of §6.2 suggests:

* **S ≈ 1 h** — a `const`/`bool` parameter on a board already here, or a new board with one
  register and no hook, plus its tests.
* **M ≈ 2-3 h** — a new board with its own register set and at most one hook already built.
* **L ≈ 6 h+** — a new hook or save shape, or more than one chip in the board.

**A lot is at most ~10 boards and ~6-8 h.** Lots are ordered by **original titles per hour**, which
is the ordering the owner's rule implies: the measure of a board is what it preserves.

**Every board gets its own Cargo feature**, per the owner's rule and `BOARD-FEATURES.md`'s
mechanism. The naming follows the existing rows: `board-<number>` for a single number,
`board-<family>` for a family (`board-nina03`, `board-jv001`, `board-sachen8259`,
`board-mmc3-chrram`, `board-mmc3-scramble`, `board-jy-asic`, `board-huang1`, `board-cony`,
`board-sachen-8reg`). A family is **one** feature because — as `BOARD-FEATURES.md` note 1 says —
"the granularity is the granularity the measurements exist at", and a family is one `Board` variant
or one `const` monomorphisation.

**Three things every lot does**, inherited from MF-6a/6b/6a3 and not restated per lot:

1. **The four sweeps over `SUPPORTED_MAPPERS`** (`req_core_100_*`, `req_core_102_*`, the
   registry round-trip, `board_memory_sizes`), in every feature configuration the lot adds, plus
   `--no-default-features` and `--features parked-boards`.
2. **A save-state round-trip per board** with mapper state dirtied first.
3. **`cargo fmt --check`, clippy `-D warnings`, and a PR-1 thirteen-ROM + level-2 five-game
   measurement against the lot's base commit**, with the +0.02 %-per-ROM tolerance of
   `BOARD-FEATURES.md`. A board over tolerance is **parked** — moved out of `default` into
   `parked-boards` — not dropped.

**And one thing no lot can do: run a game.** §5 counted it — no file in `assets-free/test-roms`
uses any of the 199 numbers. So the "tests per board" below are **written from the board's own
register table**, which is why §6 graded documentation, and the "titles R0 should run" are titles
**R0 would have to source**, listed so each lot's purpose is stated rather than assumed.

### 7.2 The twelve lots

| lot | boards | numbers | effort | original titles | titles/h |
| --- | ---: | ---: | ---: | ---: | ---: |
| **MF-UNL-1** — the JV001 protection ASIC and Sachen's register ASICs | 3 | 9 | ~7.5 h | **37** | **4.9** |
| **MF-UNL-2** — the MMC3 clones that need nothing new | 4 | 12 | ~7 h | **26** | 3.7 |
| **MF-UNL-3** — AVE, and the single-latch boards | 8 | 9 | ~8 h | **27** | 3.4 |
| **MF-UNL-4** — the J.Y. Company ASIC | 1 | 4 | ~8 h | **13** | 1.6 |
| **MF-UNL-5** — the Chinese-RPG boards with a PPU-driven CHR-RAM window | 3 | 3 | ~8 h | **13** | 1.6 |
| **MF-UNL-6** — COOLBOY / MINDKIDS | 1 | 1 | ~8 h | **12** | 1.5 |
| **MF-UNL-7** — the Sachen 8259, the 206 clones and two homebrew boards | 6 | 9 | ~7 h | **10** | 1.4 |
| **MF-UNL-8** — Huang-1's last two numbers, Cony/Yoko and J.Y.'s simple board | 3 | 5 | ~8.5 h | **10** | 1.2 |
| **MF-UNL-9** — Cheapocabra (GTROM) | 1 | 1 | ~6 h | **7** | 1.2 |
| **MF-UNL-10** — Dōngdá PEC-9588 | 1 | 1 | ~7 h | **6** | 0.9 |
| **MF-UNL-11** — the Kǎshèng protection MMC3s | 3 | 3 | ~7 h | **6** | 0.9 |
| **MF-UNL-12** — the last three singletons | 3 | 3 | ~7 h | **4** | 0.6 |
| **total** | **37** | **60** | **~89 h** | **~171** | |

The title counts double-count nothing within a lot but do count a title once per board that carries
it; where two numbers share a title the roster in §6.2 says so.

---

#### MF-UNL-1 — the JV001 protection ASIC and Sachen's register ASICs

**Boards and features**: `board-jv001` (36, 132, 136, 147, 172, 173 — the TXC/Idea-Tek
05-00002-010 and the Sachen JV001, one chip with five wirings); `board-sachen-8reg` (150, 243);
`board-133` (Sachen 3009 / 72008). **9 numbers, 3 features, ~7.5 h, ~37 original titles.**

**Why first**: the best titles-per-hour in the plan by a wide margin, and the chip is one chip.

**Steps**
1. `Jv001`, a new board holding the six registers the page names — P, R and Output (three bits
   each), S, inCrement and inVert (one bit each) — with the register window at $4100-$4103 and the
   mirroring write at $8000-$FFFF. Parameters: a `const` for the **data-bit order** (172 reverses
   D0-D5), a `const` for the **protection read width** (36 and 173 check the low three-to-four
   bits, 172 the low six), and a `bool` for whether $8000 writes drive mirroring.
2. `registry.rs` arms for 36, 132, 136, 147, 172, 173 behind `board-jv001`, each with its own
   parameters; `SUPPORTED_MAPPERS` entries; `board_memory_sizes`.
3. `Sachen8Reg`, the eight-register ASIC of 150 and 243: an index write, a data write, registers
   R4-R7 driving CHR bank, PRG bank and mirroring, and the page's statement that registers 0, 1 and
   3 "have no external effect" encoded as a no-op (**not** as an `unreachable!`).
4. `board-133`: the Namco-108 clone with the analog feedback path. The 72-pin half of the page is
   one line (`Mask $E100, $4100: [.... .PCC]`), so submapper or heuristic selection between the
   60-pin and 72-pin decodes, with the choice recorded in the code comment.
5. The three common things of §7.1, plus the feature matrix: each of the three features alone,
   all three, none.

**Tests from the pages**
* **JV001 protection**: write a value to $4100, read $4100, assert the result is the written value
  **incremented and inverted** over the board's width. One test per width (3, 4, 6 bits).
* **172's reversed data bus**: the same write/read pair with D0-D5 reversed yields a *different*
  register value than 136's — the test that proves the `const` is wired.
* **Mirroring from the Invert state** (172): a $8000-$FFFF write sets horizontal with Invert clear
  and vertical with it set.
* **173's bus conflict** (*撲克精靈*): a write to $8000 with a ROM byte present ANDs, through the
  existing `bus_conflicts` latch.
* **150/243**: index 4 selects a CHR bank, 5 a PRG bank, 7 the mirroring; indices 0, 1 and 3 change
  nothing observable. 243's `switchable H/V/vertically-flipped L/one-screen` as four cases.
* **133**: a $4100 write with mask $E100 selects the PRG and CHR banks; an address outside the mask
  does not.

**Titles R0 should run**: *Strike Wolf*, *Policeman*, *F-15 City War*, *棋王 — Chinese Chess*,
*Creatom*, *Rad Racket — Deluxe Tennis II*, *1991 賭馬 Racing*, *Venice Beach Volley*,
*撲克精靈*, *戰國四川省*, *未来小子 — Joy Van Kid*, *蝶變 — Incantation*,
*少林武者 — Chinese Kungfu* / *Challenge of the Dragon*, *Tasac*, *将棋学園 — Chess Academy*,
*Strategist*, *Olympic I.Q.*, *美女拳 — Honey Peach*, *Jovial Race*,
*盜帥 — Master Chu and the Drunkard Hu*.

---

#### MF-UNL-2 — the MMC3 clones that need nothing new

**Boards and features**: `board-mmc3-chrram` (74, 192, 194, 195); `board-mmc3-scramble` (114, 115,
121, 123, 182, 187); `board-263` (UNL-KOF97); `board-189` (TXC 01-22017/22018).
**12 numbers, 4 features, ~7 h, ~26 original titles.**

**Why second**: every board is the MMC3 this build already has, with the A12 watcher already built
and `Mmc3Board`'s `const`-parameter mechanism already proven by MF-6a3 step 2 on 119, 47 and 37.
Mapper **119 is literally a member of the first family** — Mesen builds 74, 119, 191, 192, 194 and
195 from one class — so the first step is a generalisation of code that is already in `default`.

**Steps**
1. Generalise `board-119`'s CHR-RAM selection into `Mmc3ChrRam<const FIRST: u8, const LAST: u8,
   const SIZE: u8>`: the 1 KiB CHR banks in `FIRST..=LAST` come from the CHR-RAM chip, the rest
   from CHR-ROM. 119 becomes `(0x40, 0x7F, 8)` and must produce a **byte-identical** dump against
   its current implementation — the lot's own regression gate, and the one place this lot can test
   against something that already works.
2. Add 74 `(0x08, 0x09, 2)`, 192 `(0x08, 0x0B, 4)`, 194 `(0x00, 0x01, 2)`,
   195 `(0x00, 0x03, 4)`. 195 additionally wants the page's run-time GAL, so it carries a `bool`
   for "the RAM window is selectable at run time" and the 32 KiB-fitted/8 KiB-wired note in a
   comment.
3. `Mmc3Scramble`: an MMC3 with (a) a `const` scramble table for the register address and index,
   (b) an outer CHR-ROM bank register, (c) an NROM-like PRG override register. Per number: 114 and
   182 at $6000/$6001 with two scramble patterns; 115 the same with the $600x registers live
   regardless of the WRAM enable **and the "sharp" IRQ rule** (latch 0 fires every scanline); 123
   the override at $5800; 187 the override at $5000 with protection reads and CHR A18 from PPU A12;
   121 the $5000-$5FFF protection array, bit-reversed $8001 values feeding $A000/$C000/$E000, and
   the A9711's CHR A18 from PPU A12.
4. `board-263`: A12→A0 and the page's data permutation
   `(V&0xD8) | ((V&0x20)>>4) | ((V&4)<<3) | ((V&2)>>1) | ((V&1)<<2)`.
5. `board-189`: four 32 KiB PRG banks from bits 4-5 of a write to $4100-$5FFF with address bit 9
   set, over a stock MMC3.
6. §7.1's three common things, and the configurations.

**Tests from the pages**
* **119 byte-identical** before and after the generalisation (step 1's gate).
* Per CHR-RAM number: a CHR read inside `FIRST..=LAST` hits RAM and is writable; a read one bank
  below `FIRST` and one above `LAST` hits ROM and ignores writes.
* **114/182 scramble**: writing the MMC3 register at the *scrambled* address and index produces the
  same bank layout as an unscrambled MMC3; writing at the unscrambled address does not.
* **The NROM override**: with the override register set, the MMC3's own PRG registers stop
  affecting $8000-$FFFF; clearing it restores them.
* **115's sharp IRQ**: latch = 0 asserts on every scanline, where the MMC3 proper does not.
* **121's CHR A18 from PPU A12**: a fetch in $0000-$0FFF and one in $1000-$1FFF land in different
  256 KiB halves with identical MMC3 CHR registers. Same test for 187.
* **263's permutation**: a table-driven test over all 256 input bytes against the page's formula.
* **182 writes zero to $6000/$6001 at startup and never again** — the page's own observation, as a
  test that the board tolerates it.
* **189**: a $4100 write with A9 set switches the 32 KiB bank; one with A9 clear does not.

**Titles R0 should run**: *Aladdin*, *The Lion King*, *Boogerman* (SuperGame); *Pocohontas*,
*Super Donkey Kong* (Hosenkan); *Mortal Kombat 3*, *Earthworm Jim 2* (H2288); *KOF '97*,
*Boogerman II: The Final Adventure* (Rex Soft); *Street Fighter Zero 2 '97*, *The Panda Prince*
(A9711); *Street Fighter Zero 2*, *KOF '96* (A98402); *A.V. 究極麻將 II*, *包青天*,
*餓狼伝説 Special*, *雷電 II*, *三國春秋: 四川省*, *幽☆遊☆白書 Final* (SFC-02B);
*第四次: 机器人大战*, *甲A*, *風雲 — Traitor Legend*, *Ying Lie Qun Xia Zhuan*,
*Dai-2-Ji — Super Robot Taisen*, *哥伦布传*, *天神之剑*; *Thunder Warrior*,
*Street Fighter II: The World Warrior*.

---

#### MF-UNL-3 — AVE, and the single-latch boards

**Boards and features**: `board-nina03` (79, 146); `board-38`; `board-107`; `board-143`;
`board-144`; `board-145`; `board-149`; `board-190`. **9 numbers, 8 features, ~8 h, ~27 titles.**

**Why third**: it is where the largest single catalogue in the census — AVE's nineteen titles —
sits behind the simplest board in the census, one register at $4100. The other seven are each one
latch, and the lot exists to do them all at once in the manner of MF-6a's `SmallBoard`.

**Steps**
1. `Nina03`: the $4100 control register, 8 KiB CHR bank and 32 KiB PRG bank, for 79 and 146.
   FCEUX's extra CNROM switch at $8000-$FFFF is **not** implemented — the page presents it as a
   workaround for mislabelled mapper-3 images, not as hardware. Record that in a comment.
2. One `SmallBoard`-style variant holding the seven remaining latches, each a `const`
   parameterisation: 38's $7000-$7FFF bank select (with the page's warning that $F000-$FFFF very
   likely switches too, implemented as the same decode); 107's single 8-bit register driving CHR
   and PRG together, **with all eight bits** (the page notes FCEUX implements three and MesenCE
   eight); 143's inverted-data protection read; 144's bus conflict where only the ROM's least
   significant bit wins; 145's and 149's one-register CNROM clones; 190's CHR at $A000-$BFFF and
   PRG at $8000-$9FFF/$C000-$DFFF.
3. Eight features, eight `registry.rs` arms, `SUPPORTED_MAPPERS`, `board_memory_sizes`.
4. §7.1's three common things. **Eight features is eight configurations plus all-and-none** — the
   largest matrix any lot in the plan runs, and the reason this lot is eight hours and not six.

**Tests from the pages**
* **79/146**: a $4100 write selects the CHR and PRG banks; a write at $8000 changes nothing (the
  anti-test for the FCEUX behaviour deliberately left out).
* **38**: a $7000 write switches; so does a $F000 write; a $6FFF write does not.
* **107**: all eight bits of the register are honoured — a bank number above 7 is reachable.
* **143**: a read returns the **logical inversion** of the written bits, open bus elsewhere. The
  page's note that *Dancing Blocks* "relies extensively on the values of uninitialized memory" goes
  in a comment, not a test — it is a RAM-init property, not a mapper one.
* **144**: a write of `$FF` to $8000 over a ROM byte of `$00` yields bank `$FE`, not `$FF` — the
  300 Ω resistor rule. The page gives three candidate formulas; the test pins the one chosen and
  the comment names the other two.
* **145, 149**: a one-register CHR bank switch; 149 additionally ANDs with the ROM byte.
* **190**: the three windows, each from its own register.

**Titles R0 should run**: *Dudes with Attitude*, *Trolls on Treasure Island*, *Deathbots*,
*Krazy Kreatures*, *Tiles of Fate*, *Double Strike*, *Impossible Mission II*, *Solitaire*,
*Blackjack*, *Pyramid*, *Mermaids of Atlantis*, *Ultimate League Soccer*,
*Wally Bear and the NO! Gang* (AVE); *Crime Busters* (Bit Corp.); *Magic Dragon* (Magicseries);
*Dancing Blocks*, *Magical Mathematics*, *Sidewinder*, *Taiwan Mahjong 16* (Sachen);
*Death Race* (Color Dreams); *Magic Kid Googoo* (Zemina).

---

#### MF-UNL-4 — the J.Y. Company ASIC

**Board and feature**: `board-jy-asic` (35, 90, 209, 211). **4 numbers, 1 feature, ~8 h,
13 original titles.**

**Why fourth**: the single most-played body of unlicensed NES software there is, and the precedent
is already paid — `board-116` exists because the owner accepted a board whose only titles are
Hummer Team ports (*Somari*, *Kart Fighter*). This board carries thirteen more of them.

**Why it is an L on its own**: it is the only board in the plan needing **four** hooks at once —
a CPU-cycle IRQ *and* an A12 scanline IRQ with a selectable clock source, an MMC4-style CHR latch,
and nametables served from CHR-ROM. The last is the one that is **not free**: §4.5's
`Mapper::read_nametable` is capped at **two distinct bodies** by the Q-123 measurement, and this
board would be a third. **That is the lot's main risk**, and the step list puts it first.

**Steps**
1. **Settle the nametable question before writing the board.** Measure a third
   `Mapper::read_nametable` body against the Q-123 ceiling. If it costs what Q-123 measured, the
   board serves nametables through the existing CHR-ROM path instead and the feature ships; if it
   does not fit at all, the board is **parked** (`parked-boards`), which is exactly what the
   mechanism is for. Record the figure in `PERFORMANCE-HISTORY.md`.
2. `JyCompany`: the register map at $5000-$D003 — PRG and CHR banking modes, the nametable select,
   the IRQ configuration, and the 8×8→16 **hardware multiplier**.
3. The IRQ: one counter, a selectable source (CPU M2, PPU A12, or the other documented sources),
   and one arm each on `on_cpu_cycle`, `on_ppu_address_bus` and `irq_asserted`. `ppu_irq_horizon`
   answers `None` when the source is not A12.
4. The CHR latch on `Board::ppu_read`, in the manner of the existing MMC4 path.
5. 8 KiB of PRG-RAM through the existing save path; four `registry.rs` arms for 35, 90, 209, 211
   behind the one feature.
6. §7.1's three common things, and the PR-1 + level-2 measurement **per hook added**, not just at
   the end — the lesson of MF-6a3 step 1, where two `Board` discriminants cost `mmc5` +0.080 %.

**Tests from the page**
* The **multiplier**: write two bytes, read the 16-bit product. A table-driven test.
* **IRQ source selection**: with the source set to M2, the counter advances on CPU cycles and not
  on A12 edges; with it set to A12, the reverse.
* **The CHR latch**: a fetch of the latch tile switches the bank for subsequent fetches, and the
  latch is per pattern table.
* **Nametables from CHR-ROM**: with the nametable select pointing at CHR-ROM, a $2000-$2FFF read
  returns CHR-ROM and not CIRAM; the four mirroring modes each map the four pages correctly.
* The **banking modes**, one case per documented mode, PRG and CHR.
* `ppu_irq_horizon` returns `None` for every non-A12 source — the property MF-6b's lots pinned for
  every IRQ board.

**Titles R0 should run**: *Super Mario World*, *Aladdin*, *Final Fight 3*, *Tekken 2*,
*Donkey Kong Country 4*, *Contra Spirits*, *Mortal Kombat II Special*, *Mickey Mania 7*,
*Tiny Toon Adventures 6*, *Warioland II*, *Shin Samurai Spirits 2*,
*Mighty Morphin' Power Rangers III*, *Super Aladdin: The Return of Jaffar*.

---

#### MF-UNL-5 — the Chinese-RPG boards with a PPU-driven CHR-RAM window

**Boards and features**: `board-163` (Nánjīng FC-001); `board-162` (Wàixīng FS304); `board-178`
(Wàixīng FS305 / Nánjīng NJ0430). **3 numbers, 3 features, ~8 h, 13 original titles.**

**Why these three together**: all three select a 4 KiB CHR-RAM page **from the PPU address lines
rather than from a register** — one hook (`Board::on_ppu_address_bus` / `ppu_read`, §4.4, already
built and latched), three boards, and three complete register tables. All three also want
battery-backed PRG-RAM, which the save path has.

**Scope decision this lot makes explicitly**: **submapper 0 only.** 163's submapper 1 ADPCM and
178's submapper 2 TMS5220 speech are **deferred to a later expansion-audio lot**, and 178's
submapper 1 infrared-sensor IRQ with it. Every category-A title on both boards is a submapper-0
cartridge, so the lot preserves everything that made them A. 178's educational variants also want
**Dendy timing**, which is a console-timing matter and out of this lot either way.

**Steps**
1. `Nanjing`: $5000 (PRG low / CHR-RAM switch), $5200 (PRG high), $5100-$5101 (feedback write),
   $5500-$5501 (feedback read), $5300 (mode). Battery PRG-RAM. The automatic 4 KiB CHR-RAM page
   from the PPU address.
2. `Waixing162`: the four registers at $5000/$5100/$5200/$5300 and the same automatic CHR-RAM page.
3. `Waixing178`: the four registers, mirroring control, PRG-RAM banking at $6000-$7FFF. The page's
   warning that "two ROM images will not work when emulated using the above description" because
   their register meanings are swapped is recorded as a **dumping-error note in a comment**, not
   worked around — the page attributes it to bad dumps, not to a hardware variant.
4. `expansion_audio_output` returns `None` for all three; `clocks_on_cpu_cycle` stays false. The
   deferral is therefore invisible to every other board.
5. §7.1's three common things.

**Tests from the pages**
* **The automatic CHR-RAM page**: a PPU fetch at an address in one quarter of the window selects
  one 4 KiB page and a fetch in another selects another, with **no register write between them** —
  the test that distinguishes this hook from register-driven banking.
* **The feedback register** (163): write to $5100/$5101, read $5500/$5501, assert the documented
  relation.
* **PRG high/low composition** (163): $5000 and $5200 together select a bank above 256 KiB.
* **Mirroring** (178): both arrangements from the register.
* **Battery PRG-RAM**: a write at $6000-$7FFF survives a save-state round-trip and appears in the
  save file, for all three.
* `expansion_audio_output` is `None` and `ppu_irq_horizon` is `None` for all three.

**Titles R0 should run**: *牧场物语 — Harvest Moon* (NJ011), *水浒神兽* (NJ019),
*暗黑破坏神 — Diablo* (NJ037), *轩辕剑外传 之 天之痕* (NJ045),
*Final Fantasy IV — 最终幻想 4* (NJ098), *雷电皇 比卡丘传说* (NJ046) (Nánjīng);
*Mummy — 神鬼传奇*, *法老王 — Pharaoh*, *西游记后传*, *火焰纹章 — 圣战的系谱* (Wàixīng FS304);
*宠物: 小精灵 IV*, *Ping Pong* (FS305).

---

#### MF-UNL-6 — COOLBOY / MINDKIDS

**Board and feature**: `board-268`. **1 number, 1 feature, ~8 h, 12 original titles.**

**Why it is its own lot, and the one thing to watch**: twelve original titles — nine Chinese RPGs
and three modern licensed reissues (*Gravity Trooper Metal Storm*, *Holy Diver*, *Quest Forge*,
through Retro-Bit / Piko Interactive) — behind an MMC3 whose outer registers take it from 512 KiB
to **32-64 MiB**. The cost is not the registers, it is that **the page lists thirteen PCB codes**
(COOLBOY, MINDKIDS, KT-008, YH2018A, JTH-813, GH2009_V01, SMD173C_60, SMD173C_L1, KP-6022, LD622D,
J-852C, SMD72A_V5S_V01, SMD172C-L1) and its documentation is **partial**. A single implementation
covering all thirteen is a guess; the lot should implement what the page documents, name in the
code which PCB codes it is known to serve, and leave the rest refusing.

Mapper **224** (census E) is the same chipset per its own page and would come along if that
implementation ever covers the MINDKIDS variant — but 224 has no title, so this lot does not add
it.

**Steps**
1. `Mmc3Outer`: an MMC3 with the page's outer bank registers, the mixed CHR-ROM/RAM modes and the
   GNROM-style and "weird" MMC3 variants it documents.
2. Lift the **PRG/CHR size ceiling** in the loader and in `board_memory_sizes` to what a 64 MiB
   image needs — a check this build has never had to make, and the lot's real risk.
3. The lockout: documented, and implemented as a no-op with the reason in a comment, as the
   Camerica row of `BOARD-FEATURES.md` already does for 232 ("mostly unimportant for emulation").
4. §7.1's three common things, plus a **header-size regression test**: an image declaring 64 MiB of
   PRG loads, and one declaring more is refused with REQ-CORE-011's message rather than panicking.

**Tests from the page**: each documented banking mode, one case each; an outer-bank write selects a
bank above 512 KiB; the CHR-ROM/CHR-RAM mix; and the size-ceiling pair above.

**Titles R0 should run**: *大话水浒*, *楚留香新传*, *魔道劫*, *口袋妖怪: 叶绿* (vols 1-3),
three *最终幻想* titles, *神奇宝贝*; *Gravity Trooper Metal Storm*, *Holy Diver*, *Quest Forge*.

---

#### MF-UNL-7 — the Sachen 8259, the 206 clones and two homebrew boards

**Boards and features**: `board-sachen8259` (137, 138, 139, 141); `board-112`; `board-193`;
`board-298`; `board-218` (Magic Floor); `board-29` (RET-CUFROM). **9 numbers, 6 features, ~7 h,
10 original titles.**

**Why these together**: five of the six are a variant of something already here — 112 and 193 are
near-206 boards and `board-namcot108` has the 206; 298 is a `Vrc2Vrc4` `Vrc4b`; 218 and 29 are one
latch each. The 8259 is the only new board, and it is one chip with four wirings.

**The documentation caveat this lot has to carry**: the 8259 page says its information "was
gathered primarily from FCEU-mm and while it appears to be a good enough behavioral description to
run many ROMs, it is likely incomplete", and **it governs all four numbers**. Neither NESdev nor
Wikipedia attributes the 8259 titles to 137/138/139/141 individually. So the lot implements the
four variants from the page's own A/B/C/D table and **records the attribution gap in the code
comment** — it does not guess which variant *The Great Wall* and *Q-Boy* are on beyond what the
page says (137 is the D and the page gives *The Great Wall* as "the only known game").

**Steps**
1. `Sachen8259<const VARIANT: u8>`: the $4100 index / $4101 data pair, internal registers 0-3
   (CHR select), 4 (CHR outer bank), 5 (PRG bank) and 7 (mode and mirroring), with the per-variant
   CHR shift the page's table gives. Four `registry.rs` arms behind one feature.
2. `board-298`: a `const` parameter on `Vrc2Vrc4` selecting `Vrc4b` plus the one documented
   difference — **"writing to $F003 only acknowledges a pending IRQ"** — in the manner of `McAcc`.
3. `board-112`: the 206 with the page's scrambled registers at $8000/$A000/$E000.
4. `board-193`: NTDEC's TC-112 — CHR in three registers over unequal windows, PRG at $8000-$FFFF,
   mirroring at **$6004**.
5. `board-218`: no CHR memory at all. The 2 KiB of CIRAM serves the pattern tables, and the four
   configurations are which PPU address line drives CIRAM A10 — A10, A11, A12 or A13 — through
   `Mapper::nametable_page`, which §4.10 prices at **0**.
6. `board-29`: the $8000 register, two bits of CHR-RAM bank and three of PRG bank, hard-wired
   vertical mirroring, 8 KiB WRAM. The CopyNES flash-write path (EXP0 and R/W low) is **not**
   emulated — it is a programmer feature, not a game one; comment says so.
7. §7.1's three common things.

**Tests from the pages**
* **8259**: index/data writes reach each of registers 0-5 and 7; the four mirroring modes including
  `(0,1,1,1)` and one-screen A; the per-variant CHR shift distinguishes A from B from C from D;
  the CHR-RAM variant (the 2M-RAM-COB board) ignores CHR writes when disabled.
* **298**: a write to $F003 acknowledges an IRQ and does **not** reload the counter — the single
  difference from mapper 21/25, as a direct A/B against the existing VRC4b.
* **112**: the scrambled register order produces a 206's bank layout; the unscrambled order does
  not.
* **193**: the three CHR windows are unequal and each from its own register; a $6004 write changes
  mirroring (and the page's note that "no games are known to rely on changing mirroring at
  run-time" is a comment, not a reason to skip the test).
* **218**: a pattern-table fetch returns CIRAM; each of the four configurations gives the documented
  two-screen or one-screen arrangement; **there is no CHR bank register to test**, which is the
  point.
* **29**: four CHR-RAM banks, eight PRG banks, writes to CHR-RAM visible on read-back.

**Titles R0 should run**: *The Great Wall*, *Q-Boy* (Sachen 8259); *Huang Di*,
*San Guo Zhi — Qun Xiong Zheng Ba* (112); *Fighting Hero*, *War in the Gulf* (193);
*Lethal Weapon* (298); *Magic Floor*, *Starfight* (218); *Glider* (29).

---

#### MF-UNL-8 — Huang-1's last two numbers, Cony/Yoko, and J.Y.'s simple board

**Boards and features**: `board-huang1` (12, 14); `board-cony` (83, 264); `board-91`.
**5 numbers, 3 features, ~8.5 h, 10 original titles.**

**Why these together**: all three hold a board that is a near-relative of something this build has.
12 and 14 are the **Huang-1**, the same ASIC as mapper **116**, which `board-116` already
implements as `Somari` — so `Somari`'s three-chip delegation is the starting point and MF-6b step 6
already wrote 14's design. 83/264 and 91 are both IRQ boards whose hooks are built.

**Steps**
1. `board-huang1`, from `Somari`: 12 is the SL-5020B chip-on-board — the same ASIC plus an external
   register at **$4132** for 512 KiB of CHR — so it is `Somari` with one more register.
   Submapper 1 (the FFE Magic Card 4M) is **not** implemented: it is copier hardware with a trainer,
   the same reason §1.8 puts 6/8/17 out, and the code comment says so.
2. 14, exactly to MF-6b step 6's table: the supervisor at **$A131** overlapping the MMC3's and the
   VRC2's own registers (so the board's decode runs *before* the sub-board's), two modes (VRC2 and
   MMC3, **no MMC1 mode**), and **three independent CHR A18 bits** applied to 1 KiB window groups
   `0-3`, `4-5` and `6-7`. Its mirroring rule comes from Nestopia's source by the page's own
   admission, so the implementation follows the page and the comment records the caveat.
3. `board-cony`: the $8000 PRG base, the $8100 mode register (mirroring, banking modes, IRQ
   control), the $8300-$8303 8 KiB PRG banks, the $8310-$8317 CHR banks (2 KiB on 264/83.1), the
   $5000 solder-pad read and the $5100-$5103 scratch RAM. One IRQ counter, **source selectable
   between CPU M2 and PPU A12** — both hooks built.
4. `board-91`: CHR banks, mirroring, the IRQ counter and PRG banks. The page's warning that "the
   commonly available ROM image of Super Fighter III is a mapper hack for INES Mapper 197" is a
   comment — that title is **not** evidence for this board and is not in R0's list below.
5. §7.1's three common things.

**Tests from the pages**
* **14's supervisor decode**: a write to $A131 switches mode *and* is seen by the sub-board's
  register at the same address — the ordering test that the overlap makes necessary.
* **14's three CHR A18 bits**: with the three bits set differently, windows 0-3, 4-5 and 6-7 land in
  different 256 KiB halves. One case per group.
* **12's $4132**: an outer CHR bank above 256 KiB is reachable; with $4132 clear it is not.
* **Cony's IRQ source**: as MF-UNL-4's — M2 advances on CPU cycles only, A12 on edges only.
* **Cony's scratch RAM**: four bytes at $5100-$5103 read back what was written and survive a
  save-state round-trip; the **solder-pad read at $5000** returns the configured value.
* **91's IRQ counter** and its four register groups, one case each.
* `ppu_irq_horizon` is `None` whenever the source is not A12, on both IRQ boards.

**Titles R0 should run**: *Dragon Ball Z 5* (12); *Samurai Spirits*, the 8-character version (14);
*Street Fighter II Pro*, *Fatal Fury 2*, *World Heroes 2'*, *Super Blaster VII Turbo 28* (Cony);
*Street Fighter 3*, *Mortal Kombat II*, *Dragon Ball Z 2*, *Mario & Sonic 2* (91).

---

#### MF-UNL-9 — Cheapocabra (GTROM)

**Board and feature**: `board-111`. **1 number, 1 feature, ~6 h, 7 original titles.**

**Why it is worth an L for one number**: seven *commercially released* homebrew games —
*Anguna: Scourge of the Goblin King*, *Candelabra: Estoscerro*, *The Incident*, *NEScape*,
*Project Blue*, *Spook-o-tron*, *Storied Sword* — on a fully documented board with a schematic,
from a maker that still sells the cartridges. It is the best titles-per-number ratio of any single
number in the census outside the J.Y. ASIC.

**Why it is an L**: the save path. The board has **no PRG-RAM**; the game rewrites its own
**SST39SF040 flash**, so a save means recognising the chip's command sequences and persisting a
sector of what the loader believes is read-only ROM. That is a shape `CartridgeMemory` does not
model, and it is the whole of this lot's difficulty.

**Steps**
1. A `FlashRom` memory kind in `CartridgeMemory`: the SST39SF040's command sequence (the
   unlock writes, sector erase, byte program), a dirty-sector set, and a save file that is the
   changed sectors rather than a separate RAM image. **This is the step to design before coding** —
   it is the first time this build persists something that the ROM window reads.
2. `Gtrom`: the control register's PRG bank, its **bit 5 selecting between two 8 KiB CHR-RAM pages
   used as nametable RAM**, and four-screen mirroring forced with no alternative.
3. Bits 6-7 (the two LEDs) are decoded and discarded, with the page's own "currently universally
   unsupported" quoted in the comment.
4. §7.1's three common things, with the save-state round-trip extended to cover a **flashed**
   sector as well as mapper state.

**Tests from the page**
* The flash command sequence: an unlock-then-program sequence changes a ROM byte; an **incomplete**
  sequence does not; a sector erase sets a whole sector to `$FF`.
* A flashed byte survives a save-state round-trip **and** appears in the save file.
* Bit 5 switches the nametable between the two 8 KiB CHR-RAM pages; a write to one page is not
  visible in the other.
* Mirroring is four-screen regardless of the header's flag.
* The LED bits change nothing observable.

**Titles R0 should run**: the seven above.

---

#### MF-UNL-10 — Dōngdá PEC-9588

**Board and feature**: `board-164`. **1 number, 1 feature, ~7 h, 6 original titles.**

**Six titles** — *Final Fantasy 太空戰士 V* (G-003), *大話西游* (G-004), *櫻桃小丸子* (G-005),
*岳飛傳* (G-006), *Dark Seed — 黑暗之蛊* (G-009), *口袋精靈: 金* — with catalogue numbers, behind a
**full** register table. **An L for two reasons**: a **93C66 512-byte serial EEPROM over a Microwire
interface**, which is a third EEPROM shape beside the X24C01 and 24C02 the inventory's §4.6 already
plans for Bandai's 157/159; and a **1 bpp video mode** that changes CHR addressing.

**Note for sequencing**: if MF-6c's Bandai row (157, 159) is built first, its EEPROM work is most
of this lot's step 1. **This lot should follow MF-6c, not precede it**, and that is the only
ordering constraint the plan has beyond "MF-UNL-2 after nothing".

**Steps**
1. A Microwire serial-EEPROM device in `CartridgeMemory`, parameterised by word width — 93C66 is
   512 bytes at 16-bit words. Shared with, or shared by, 157/159's X24C01 and 24C02.
2. `Pec9588`: the five registers at $5000/$5100/$5200/$5300/$5500, mirroring from $5300, and the
   1 bpp CHR addressing mode.
3. §7.1's three common things, with the save path exercised through the EEPROM.

**Tests from the page**: the EEPROM's read and write command sequences, bit by bit over the serial
line, against the 93C66's documented protocol; a written word survives a save-state round-trip and
reaches the save file; the 1 bpp mode changes which CHR bits a pattern fetch consumes; mirroring
from $5300.

**Titles R0 should run**: the six above.

---

#### MF-UNL-11 — the Kǎshèng protection MMC3s

**Boards and features**: `board-215` (UNL-8237/8237A); `board-208`; `board-219`.
**3 numbers, 3 features, ~7 h, 6 original titles.**

All three are an MMC3 with a protection mechanism and an extended banking mode, and all three use
only hooks this build has. They are late in the plan because the titles-per-hour is low, not because
anything is hard.

**Steps**
1. `board-215`: the $5007 register selecting one of **eight** address/data scramble patterns, plus
   the outer banks and the NROM override. Submapper 0 is UNL-8237 and 1 is UNL-8237A.
2. `board-208`: the PRG bank and mirroring at $4800-$4FFF and $6800-$6FFF, the protection index at
   $5000-$57FF and the protection data at $5800-$5FFF **XOR'd against the page's 256-byte table**,
   which goes in the source as a `const` array with the page cited.
3. `board-219`: the extended mode at $8002 bit 5 — four independent PRG windows and pseudo-2 KiB
   and 1 KiB CHR switches — with outer bits at $5002/$5003 for 512 KiB, and standard MMC3 behaviour
   when the mode bit is clear.
4. §7.1's three common things.

**Tests from the pages**
* **215**: each of the eight $5007 patterns produces its documented bank layout — eight cases, one
  per pattern, which is the only way to prove the table is transcribed right.
* **208**: write an index, write data, read back and assert the XOR against the table; a wrong code
  leaves the board in the state the page describes.
* **219**: with $8002 bit 5 set, the four PRG windows are independent; with it clear, the board is
  an MMC3. The pseudo-2 KiB CHR switch at the documented PPU range.

**Titles R0 should run**: *Earthworm Jim 2*, *Mortal Kombat 3*, *Mortal Kombat 3 Extra 60*,
*Pocahontas Part 2* (215); *快打傳説 — Street Fighter IV* (208); *Toy Story* (219).

---

#### MF-UNL-12 — the last three singletons

**Boards and features**: `board-262` (UNL-SHERO); `board-292` (UNL-DRAGONFIGHTER); `board-246`
(G0151-1). **3 numbers, 3 features, ~7 h, 4 original titles.**

Last because the ratio is lowest. Each is nonetheless a clean board with a complete-enough page.

**Steps**
1. `board-262`: an MMC3 clone with 8 KiB of CHR-RAM, a CHR-ROM/CHR-RAM select and the high CHR bits
   at **$4100**, and **4 KiB of extra VRAM for four-screen mirroring** — the one nametable shape in
   this lot, and the one §4.5 already has a path for. Several masks on the page are "probably" or
   "unknown"; the implementation follows the page and the comment lists which masks are guesses.
2. `board-292`: the XOR protection and the two registers reached by **reads as well as writes** at
   $6000-$7FFF driving CHR A17-A18 and the per-pattern-table XOR values. `Cartridge::cpu_read`
   already reaches that window, so no new hook.
3. `board-246`: four PRG banks at $6000-$6003 and four CHR banks at $6004-$6007, battery PRG-RAM at
   the unusual **$6800-$6FFF**, and a **forced PRG A17 on reads from certain CPU addresses**. The
   page's note that *封神榜* "only work[s] in Famiclones" is recorded in the comment — this build
   targets NTSC/PAL/Dendy timing, and whether the game runs is a timing matter outside the board.
4. §7.1's three common things.

**Tests from the pages**
* **262**: the CHR-ROM/CHR-RAM select at $4100 flips the source for a CHR read; the four-screen
  arrangement maps all four pages to distinct RAM; the high CHR bits reach banks above 256 KiB.
* **292**: a **read** at $6000-$7FFF changes the CHR bank, which is the test that proves the
  read path is wired; the XOR values differ per pattern table; the effective-bank arithmetic
  checked against the page's own worked table.
* **246**: the eight bank registers; a PRG-RAM write at $6800-$6FFF survives a round-trip and a
  write at $6000-$67FF does **not** reach PRG-RAM (it is a bank register); the A17 forcing on the
  documented read addresses.

**Titles R0 should run**: *Street Heroes*, *侍魂 — Samurai Spirits* (262); *Dragon Fighter* (292);
*封神榜 — Fēngshénbǎng: Fúmó Sān Tàizǐ* (246).

### 7.3 The category-A boards this plan excludes, and the only reasons left

Seven of the 44 A boards are out. **No exclusion below is a performance argument** — the owner
retired that — and each cites one of the three reasons that remain.

| board | numbers | reason | the argument |
| --- | ---: | --- | --- |
| not named | **170** | **no reliable public documentation** | the page is **one sentence**: the mapper is used by one unlicensed Famicom title. No register, no mask, no mode, no publisher, no board name. *Shiko Game Syu* is a real original game and this is the census's clearest "worth preserving, impossible to build" row — the same position MF-6b step 6 put mapper 238 in. **If a register table appears, this becomes an S.** |
| not named (RCM Group) | **216** | **no reliable public documentation** | three original titles (*Bonza*, *Videopoker Bonza*, *Magic Jewelry II*) and a stub. The only technical statement on the page is that some revisions need **Dendy timing** to display correctly — which is a console-timing note, not a mapper description. Nothing to write a test from. |
| two unnamed PCBs | **240** | **no reliable public documentation** | two original titles, one of them Hummer Team's *Jīng Kē Xīn Zhuàn* (Wikipedia: "a Chinese language RPG"), and a page that says "Selects 32k PRG @ $8000" and "Selects 8k CHR @ $0000" with two different decodings and **no mask, no power-on state and no way to tell the two PCBs apart** except by a heuristic the page does not give. |
| not named (C&E) | **244** | **no reliable public documentation** | one original title, *Decathlon*, and a page with no technical content at all. |
| DREAMTECH01 | **521** | **no reliable public documentation** | one original title, a Korean Igo tutorial, and a stub: a bank register at $5020 with mask $F030 and a fixed last bank, with no power-on state and no account of what the other bits do. **Borderline** — closer to implementable than the four above — and the honest call is that one title does not justify guessing. |
| UNL-SA-9602B | **513** | **development cost out of proportion** | one title (the *Princess Maker 3* port), and it would need **32 KiB of battery-backed CHR-RAM** — a save shape on the CHR side that `CartridgeMemory` does not model and that no other board in the census wants. A new save path for one game. If a later lot adds CHR-side saves for another reason, 513 becomes an **S** and should be revisited. |
| UNL-DRIPGAME | **284** | **development cost out of proportion** | one homebrew title, *Drip*, and the page (comprehensively) asks for a cycle IRQ with a reload buffer, **two FIFO-buffered expansion-audio channels**, and **extended attribute tables giving per-tile palette selection**. That last is not a cartridge feature: it changes what the **PPU** fetches for attributes, so it is `zax-ppu`'s hot loop and SPEC-06's text, not the cartridge layer. One game does not justify reaching into the renderer. |

**And one E worth naming, because the owner may want to overturn it cheaply: mapper 31.** Full
documentation, no hook at all, three lines of code — a 4 KiB PRG bank register at $5FF8-$5FFF with
the top bank fixed at power-on. It is **E** because its titles are not games: *2A03 Puritans* and
*Famicompo Pico* are chiptune compilations and *EZNSF*/*ZENSF* are NSF players. The census does
**not** propose it, and records the price so the decision has a number on it: **S, no hook, one
feature**. The same holds, less cheaply, for **323** and **324** (Farid's homebrew 8-in-1s, an MMC1
and a UNROM with an outer bank), which are out because **neither page names a single title**.

### 7.4 What the plan does not change

* **`req_core_102_any_other_mapper_is_refused` holds unchanged.** It sweeps 6, 116, 177 and 200;
  116 is already supported, and this plan leaves **6** (category E, copier hardware), **177**
  (category E, no game) and **200** (category C, a multicart) out. Q-130 question 2's observation —
  that the test holds no number guaranteed never to become supported — is still true and is
  restated with candidate replacements in **Q-133**.
* **REQ-CORE-100's criterion is untouched.** The 101 embedded profiles are all licensed titles;
  none of the 60 numbers has a profile, and every feature this plan adds is measured against the
  +0.02 % tolerance that protects the default build.
* **Nothing here is a spec change.** Whether REQ-CORE-102's permission reaches 60 numbers is
  **Q-133 question 1**, filed with the provisional choice (the permissive reading) that this plan
  is written on. No code has been written on it, so a restrictive answer costs the plan and nothing
  else.

---

## 8. The check

This lot changes no `.rs` file, so the check is that nothing moved.

    git diff --stat 8496c503 HEAD
     docs/progress/UNL-census.md |  245 +++++++
     docs/review/UNL-census.md   | 1552 ++++++++++++++++++++++++++++++++++++++
     2 files changed, 1797 insertions(+)

    git diff --name-only 8496c503 HEAD -- '*.rs'   →  0 files

With `TMP`, `TEMP` and `CARGO_TARGET_DIR` under `rust/target-mf`:

| command | result |
| --- | --- |
| `cargo fmt --check` | **clean** |
| `cargo test --release -p zax-nes -p zax-harness` | **899 passed, 0 failed, 8 ignored** |

**899 / 0 / 8 is the figure MF-6a3 step 5 recorded** for the same two crates at `8496c503`
(`docs/progress/MF-6a3.md`), so the suite is byte-for-byte the suite the baseline left — which is
the only thing a documentation lot can be asked to prove.

### The one gap a reviewer should close

**NesCartDB answers HTTP 403 to every request from this environment** (§0), so no per-number
cartridge count in this census could be refreshed from it. Every title list above comes from the
NESdev page that names it, cross-checked against Wikipedia where Wikipedia covers the publisher
(`/wiki/Sachen`, `/wiki/Hummer_Team`, `/wiki/American_Video_Entertainment`, `/wiki/Rad_Racer`) and
against Mesen 2's `MapperFactory.cpp` for board identity. A reviewer with a browser should
re-check the counts for the three boards the plan leans on hardest — **79** (AVE's nineteen
titles), **35/90/209/211** (the J.Y. ASIC's thirteen) and the **JV001** family's twenty — because
MF-UNL-1 through MF-UNL-4's ordering rests on them.
