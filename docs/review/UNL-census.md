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
2. **A number is categorised by the highest category any one of its titles reaches**, because the
   question is what shipping the board would preserve. A board carrying one original game and nine
   multicarts is A. Where that promotion is the only reason for the category, the row says so and
   names the single title that does it.

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
