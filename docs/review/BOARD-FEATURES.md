# The board features of `zax-nes`

One Cargo feature per board that raises any figure (owner's rule, 2026-11-06), built by lot
**MF-6a3**. Each gates its board's code, its `Board` variant or `const` monomorphisation, every
dispatch arm, its `SUPPORTED_MAPPERS` entries, its `registry` arms, its `board_memory_sizes` arm,
its save-state block and its tests.

**The tolerance is at most +0.02 % per ROM on both runs** — PR-1's thirteen ROMs, and level 2's
five games on `core` *and* `total` — measured against the base commit of the lot that added the
board. A board inside it is in `zax-nes`'s `default`; a board above it is left out, and the
umbrella feature **`parked-boards`** then enables all the left-out ones at once.

**One feature is parked: `board-vs-system`, the Nintendo VS. System** (MF-VS step 7). The
umbrella was empty through MF-6b and MF-6a3, wired so that a later lot which did park a board
would find the switch ready; **MF-6c step 2 put `board-taito-x1` in it and step 9 `board-111`,
MF-CART step 5 took both back out, and MF-VS step 7 put the VS. System in.** Every *cartridge*
board this build has is in `default`; the VS. System is one flag away
(`--features parked-boards`), and the figures that put it there are in its row below.

**One feature in this table is not a board: `ppu-exact-sprite-eval`** (MF-4e step 3). The owner's
rule is written for boards, but what is behind it is the figure, so lot MF-4e applied it unchanged
to a piece of the PPU. With the feature on, the sprite evaluation is the hardware's byte counter in
two independently clocked halves against a second counter over the secondary OAM, and a `$2004`
read during rendering answers from the OAM copy buffer instead of from `oam[oam_address]`.

| | figure |
| --- | --- |
| what it wins | `AccuracyCoin` **129 → 132 / 147**: `Address $2004 behavior`, `Arbitrary Sprite zero`, `Misaligned OAM behavior` |
| PR-1's thirteen | **above on nine of the eleven** whose behaviour does not move: `metroid` **+0.133 %**, `z2` +0.107 %, `mm2` and `smb1` +0.083 %, down to `m4` +0.041 %; `vblnmi` +0.008 % and `oamstr` +0.001 % inside |
| level 2 | **all ten above**: `zelda2/core` **+0.127 %** down to `smb3/total` +0.057 % |
| gated off | set total **−91 994 (−0.000 %)**, worst ROM **+0.001 %**, and `run_sprite_evaluation`, `background_fetch`, `run_dots`, `paint_sprite` and `rebuild_sprite_line` **identical to the instruction** on five ROMs |
| in `default` | **off** |
| the command | `cargo build -p zax-nes --features ppu-exact-sprite-eval`, and `cargo build -p zax-harness --features ppu-exact-sprite-eval` so the `AccuracyCoin` guard scores the right configuration |

It is **not** in `parked-boards`: that umbrella is for left-out *boards*, and a build that wants
everything asks for both flags. The `$2007`-during-rendering rule of MF-4e step 2 is **not** behind
it — Q-125 and AMD-021 R2-005 both require that rule, and it measures +0.000 % to +0.005 % on the
eleven, so it is in every build.

**Both were parked on one instruction, and it was not theirs — MF-CART removed the instruction.**
`Cartridge::default_cpu_read` held the inlined `Board::try_read_prg` jump table, and past a
certain pressure LLVM materialised `&self.board` above it for that table's *cold* arms (mapper
116's boxed `IrqBoard::Somari(Box<Somari>)`, and the arms that are `#[inline(never)]`), where it
then ran on every call. The MMC5 is the one board that maps PRG-RAM into $8000-$DFFF, so
`Cartridge::read_rom_window` cannot answer and **every** CPU read of an MMC5 game reached that
function — **exactly 7 456 756 of them over 300 frames of `mmc5exram`**, which is why the figure
was always a whole number of units of 7 456 756 (+0.066 % each): it was one instruction per read.
MF-6c measured it nine times in both directions, found each shared enum's own limit (`SmallBoard`
ten, `IrqBoard` ten, `DecodeBoard` four) and could not move it with seven shapes of a board's own
code; MF-CART step 1 measured it a tenth time at **three** units for `board-taito-x1` and step 3 an
eleventh at **four**, the wall moving further away as the tree grew.

**What MF-CART did**, in two halves that only work together (`docs/progress/MF-CART.md` steps 2
and 4): the $6000-$7FFF read answer is settled on `Cartridge` beside `rom_windows` through the new
`Mapper::prg_ram_window`, so no `Board` dispatch answers it per read; and
`Cartridge::default_cpu_read` then carries `#[inline(always)]`, which MF-6c step 2 had measured as
its shape c and rejected because `Board::prg_ram_enabled` went out of line as the body moved — the
one drawback the first half removes. The function stops existing, the MMC5's own `try_read_prg` arm
goes out of line, and **`Mmc5::try_read_prg` and `Cartridge::try_cpu_read` then measure 141 678 364
and 429 758 682 whether `Board` has 36, 38 or 39 variants, identical to the instruction.** The
MMC5's read path no longer depends on how many boards exist. `rust/docs/PERFORMANCE-HISTORY.md`
§5.15 has the new limit for a lot that needs to plan against one.

## The table

Costs are the figure the step that added the board measured, with the worst ROM named; `07325f6e`
is the end of MF-6b and `240a2718` the end of MF-5b-fix. Flipping a feature is one command, given
in the last column; it is `-p zax-nes` because that is the crate the feature belongs to, and the
same flag passed to `zax-harness` or `zax-app` as `--features zax-nes/<name>` reaches it too.

| feature | mappers | board | titles | measured cost, PR-1's thirteen | measured cost, level 2 | default | the command that flips it |
| --- | --- | --- | ---: | --- | --- | :-: | --- |
| `board-namcot108` | 76, 88, 95, 154, 206 | Namco 108 / DxROM, with four `const`/`bool` parameters | **45** | worst `m4` **+66 921 (+0.0012 %)** (MF-6a step 1, vs `240a2718`) | `smb3/core` **+45 707 (+0.0003 %)** | **on** | `cargo build -p zax-nes --no-default-features --features <the other eight>` |
| `board-small-boards` | 33, 70, 75, 89, 93, 97, 140, 152, 184, 185, 232 | nine boards behind **one** `Board` variant: Taito TC0190, Bandai 74161/7432, Konami VRC1, both Sunsoft-2 boards, Irem TAM-S1, Jaleco JF-11/14, Sunsoft-1, CNROM + diodes, Camerica BF9096 | **~30** | step 2 `smb3/core` **+433 086**; step 3 **+461 302**; step 4 adds `mm2/core` **+10 506** (MF-6a, vs `240a2718`) | the same three steps' `core` figures, +0.0001 % to +0.0030 % | **on** | as above |
| `board-94` | 94 | HVC-UN1ROM, a wiring of the UxROM board (its bank field shifted two bits) | 1 | in MF-6a step 4's `mm2/core` **+10 506** and `smb3/core` **+461 302** with 155 and 185 | the same | **on** | as above |
| `board-155` | 155 | the MMC1 on a board with no PRG-RAM disable wire | 1-2 | MF-6a2 step 2 traced step 4's `mm2/core` **+10 506** to this parameter — `Mmc1::update_banks` **+8 001** over 600 level-2 frames of *Mega Man 2* — and then **removed it**: the two-mask idiom left that function **215 instructions, five below** `240a2718`'s 220 | as above | **on** | as above |
| `board-105` | 105 | Nintendo NES-EVENT, holding an `Mmc1` | 1 | **all thirteen below** `9f9d9d41` (MF-6b step 3, set −21 123 914) | `smb1/total` **+3 149 624 (+0.023 %)**, `smb3/total` +0.010 % — **both entirely Lua's `ltable.c`**, and both gone when step 4 re-measured | **on** | as above |
| `board-158` | 158 | Tengen 800037, a `const` parameter of the RAMBO-1 | 1 | **all thirteen below** `9f9d9d41` (MF-6b step 5, set −19 980 904) | `smb3/total` **+1 233 269 (+0.005 %)**, of which Lua's `ltable.c` is +4 008 826 and this project's own symbols −3 104 672 | **on** | as above |
| `board-119` | 119 | Nintendo TQROM, a `const` parameter of `Mmc3Board` with a CHR-RAM chip beside its CHR-ROM | 2 | **with the multicart below**: worst `m0` **+58 307 (+0.0011 %)**, set total **−57 265** (MF-6a3 step 2, vs `07325f6e`) | all five `core` within **±6 699**, `smb1/core` **+0**; two `total` rises inside the same binary's own ±4.3 M spread on that column | **on** | as above |
| `board-mmc3-multicart` | 47, 37 | Nintendo's two MMC3 multicarts, one `const` monomorphisation with an outer register at $6000-$7FFF | 2 | as `board-119`, measured together | as `board-119` | **on** | as above |
| `board-116` | 116 | Gouder SOMARI-P / Huang ASIC, **holding** an `Mmc3`, an `Mmc1` and a `Vrc2Vrc4` | **6** | worst `m1` **+96 537 (+0.0020 %)**, set total **−7 291 714 (−0.009 %)**, `mmc5` **−0.066 %** (MF-6a3 step 4, vs `07325f6e`) | all five `core` **+253 to +16 832** (`smb3/core` is 253 instructions in 15.3 G); worst `total` `smb3` **+0.011 %** | **on** | as above |
| `board-153` | 153 | the Bandai FCG board with an LZ93D50 and 8 KiB of battery-backed WRAM, **two appended `bool`s on `BandaiFcg`** | 1 (*Famicom Jump II*) | worst `duelito` **+57 669 (+0.001 %)**, set total +37 631 (MF-6c step 1, vs `8496c503`) | **all five `core` +0**; worst `total` `smb3` +0.005 %, entirely Lua's `ltable.c` | **on** | as above |
| `board-68` | 68 | the Sunsoft-4, **in `IrqBoard` and not `SmallBoard`** — its page says "IRQ: None", but the eleventh `SmallBoard` variant costs `mmc5` +0.066 % and the tenth `IrqBoard` one costs nothing | **~6** | worst `mm2` **+68 558 (+0.001 %)**, set total +23 851, `mmc5` **−35 125** (MF-6c step 4, vs `8496c503`, measured with `board-68-chr-nametables` on) | **at or below the baseline on every game, total and core** | **on** | as above |
| `board-68-chr-nametables` | 68 | **mapper 68's CHR-ROM nametable mode alone**, $E000 bit 4 — the third distinct body of the dispatch Q-123 capped at two. Implies `board-68` | — | against `board-68` alone: set total **+17 007 (+0.000 %)**, worst `metroid` +0.001 %, `readbuf` **−78 628** (MF-6c step 4) | included in `board-68`'s row above | **on** | as above |
| `board-eeprom` | 157, 159 | the Bandai LZ93D50 with a serial EEPROM: `Eeprom24C0X` as **one** state machine for the X24C01 and the X24C02, and `EepromPort` as one `Option` field on `BandaiFcg` | **11** | worst `duelito` **+60 800 (+0.001 %)**, set total **−10 565**, `mmc5` **−40 903** (MF-6c step 3, vs `8496c503`) | **all five `core` +0**; worst `total` `smb3` +0.014 %, entirely Lua's `ltable.c` and `memcpy` | **on** | as above |
| `board-decode-boards` | — | the **second** shared `Board` variant for a board that is a register decode and nothing else, `DecodeBoard`, with no board of its own: each board below implies it. It exists because MF-6c steps 2 to 5 measured `SmallBoard` *and* `IrqBoard` as full, and R0's decision of 2026-11-06 is to price **one** new `Board` discriminant for all four of the lot's remaining boards instead | — | measured with `board-77`, below: the discriminant alone is **free** | as that row | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-77` | 77 | the Irem LROG017: one `[CCCC PPPP]` register, bus conflicts, four-screen mirroring and a **split CHR** — 2 KiB of swappable CHR-ROM at $0000-$07FF with the 6 KiB CHR-RAM chip at $0800-$1FFF, which no `ChrView` can describe. First member of `DecodeBoard`; implies it | 1 (*Napoleon Senki*) | worst `metroid` **+12 780 (+0.000 %)**, set total **−136 318**, `mmc5` **−40 814** (MF-6c step 6, vs `8496c503`) | four `core` **+0** and `smb3/core` **−20 823**; worst `total` `smb3` +0.024 %, which the per-function diff puts **entirely** in Lua's `ltable.c` (`luaH_getshortstr` +4 039 948, `getgeneric` +2 248 672) with no symbol of this project moving by 120 000 on that game. `Ppu2C02::read_pattern_planes` **107, unchanged** | **on** | as above |
| `board-jaleco-72-92-86` | 72, 92, 86 | Jaleco's JF-17, JF-19 and JF-13, the three boards with a µPD7756C ADPCM speech chip. 72 and 92 are one board whose `is_jf19` bool picks which PRG window its rising-edge latch drives; 86 is a GNROM-like board whose ports *are* $6000-$7FFF. Two `DecodeBoard` variants, no `Board` variant; implies `board-decode-boards`. **The audio is not implemented** — REQ-CORE-075 does not list the chip and its samples are a mask ROM inside the ASIC (§4.8), so `expansion_audio_output` stays `None` and a test pins the silence | **6** | worst `duelito` **+15 716 (+0.000 %)**, **twelve of thirteen below the baseline**, set total **−312 171**, `mmc5` −34 885 (MF-6c step 7, vs `8496c503`) | four `core` **+0** and `smb3/core` **−20 823**; the two `total` figures above the baseline are +0.000 % and +0.011 %, **both inside the tolerance on their own**, and the per-function diff puts them in Lua's `ltable.c` | **on** | as above |
| `board-96` | 96 | the Bandai Oeka Kids board: a 32 KiB PRG bank and a 16 KiB outer CHR bank from one CPU register, and the 4 KiB inner CHR-RAM page **latched off the PPU address bus** on the entry into $2000-$2FFF -- including a bare $2006 write. In **both** PPU-bus latches and the first number in either with no IRQ; one `DecodeBoard` variant and one new forward. The Oeka Kids tablet is a recorded gap, and "Bus conflicts: partly" is Q-135. Implies `board-decode-boards` | 2 | worst `duelito` **+70 187 (+0.001 %)**, set total +223 036, `m4` +31 167 on the dispatch it joins (MF-6c step 8, vs `8496c503`) | four `core` **+0** and `smb3/core` **−20 823**; three `total` figures above the baseline at +0.000 %, +0.002 % and +0.004 %, **all inside the tolerance**, and all of them Lua's `ltable.c`. `Board::on_ppu_address_bus` **157 → 161** | **on** | as above |
| `board-111` | 111 | the Membler Industries GTROM / Cheapocabra: a 32 KiB PRG bank, an 8 KiB CHR-RAM bank, **two 8 KiB banks of four distinct nametables** served through `read_nametable` (the fourth body of that dispatch), an SST39SF040 the game saves into, and two LEDs. One `DecodeBoard` variant and three forwards; implies `board-decode-boards`. The LEDs, the register's read side and the flash's persistence are recorded gaps | **7** homebrew | **un-parked by MF-CART step 5**: `mmc5` **−12 564 290 (−0.111 %)**, worst ROM `mm2`/`oamstr` **+0.002 %**, set total **−24 997 757** (vs `mfct-0-44f77f72`). MF-6c step 9 had measured `mmc5` +7 423 140 (+0.066 %) — one unit of `Cartridge::default_cpu_read`'s quantum, which MF-CART step 4 removed. The board measured **without** its nametable banks gave the same +7 416 670 then, so the fifth `DecodeBoard` variant was the cause and the mechanism was free; `read_nametable_inner` stays at **75 with four bodies** | four `core` **below or equal**, `mm2/core` +0.002 %, worst `total` −0.017 % to +0.000 % (MF-CART step 5) | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-taito-x1` | 80, 207, 82, 552 | the Taito X1-005 and X1-017, two `SmallBoard` variants; implies `board-small-boards` | **13** | **un-parked by MF-CART step 5**: `mmc5` **−12 576 627 (−0.112 %)**, worst ROM `mm2` **+0.001 %**, set total **−25 190 380** (vs `mfct-0-44f77f72`). MF-6c step 2 had measured `mmc5` +7 456 756 (+0.066 %) and MF-CART step 1 measured the same two variants at **three** units, +22 370 268 (+0.198 %) — all of it the one instruction step 4 removed | **all five `core` below or equal**, `mm2/core` +0.002 %, worst `total` +0.000 % (MF-CART step 5) | **on** | as above |
| `board-vs-system` | 99, 151 | the **Nintendo VS. System**, single-screen: the arcade machine and the two mapper numbers that exist only on it. Mapper 99 is the VS board itself, **one `DecodeBoard` variant** (no `Board` discriminant); 151 is a VRC1 on a VS board, **a parameter of the existing VRC1** and no variant at all. One feature for the whole machine because its pieces are not separable — the PPU's master-palette remap and the RC2C05's two register quirks, the coin slots, the service button and the eight DIP switches at `$4016`/`$4017`, the swapped sticks, the per-ROM table keyed by the data CRC, and the board — all answering one question settled at load, `crate::vs::VsSystem`. Implies `board-decode-boards`. **MF-VS-fix** made the table's 74 rows carry the *board* the public NES 2.0 database records — mapper, submapper, PRG/CHR-ROM, CHR-RAM, mirroring — and gave it the last word over a header that disagrees, because the circulating iNES 1.0 headers of *Vs. Castlevania* and *Vs. Top Gun* announce mapper 99 with no CHR-ROM where the board is a `UxROM` with CHR-RAM | **74 dumps** in the public NES 2.0 header database (55 of them mapper 99), *Vs. Castlevania*, *Vs. Super Mario Bros.*, *Vs. Excitebike*, *Vs. Gradius* among them | **PARKED**: **every one of the thirteen slower**, `duelito` **+0.083 %**, `readbuf` +0.077 %, `z2` +0.067 %, down to `mmc5` +0.026 % and `oamstr` +0.001 %; set total **+36 862 173 (+0.046 %)** (MF-VS step 7, vs `f1278f46`). **Gated off: set total +90 052 (+0.000 %), worst ROM +0.001 %**. **Re-measured by MF-4e step 5** against that lot's own default, the per-dot PPU code reorganised: `duelito` **+0.087 %**, set total **+36 979 368 (+0.047 %)**, twelve of thirteen above — the same figure, so the reorganisation does not reach it and it stays parked | **all ten figures above**, `zelda2/core` **+0.057 %** down to `smb3/total` +0.034 %. **Gated off: all five `core` +0.000 %** (+64 to +22 400 instructions on 11-15 G) and worst `total` `smb3` +0.019 %. **MF-4e step 5**: ten of ten above again, `zelda2/core` **+0.057 %** — MF-VS's own figure to three decimals. **MF-VS-fix step 4** measured the *gated-off* build of that lot against `0aee4241`: **all five `core` figures +0 instructions**, identical to the instruction, and `total` −0.007 % to +0.008 % — so the VS table's board correction costs the shipped build nothing at all | **off** | `cargo build -p zax-nes --features parked-boards`, and `cargo build -p zax-harness --features parked-boards` for the `vs` subcommand beside it |
| `parked-boards` | 99, 151 | the umbrella: every board left out of `default`. Empty from MF-CART step 5 until **MF-VS step 7**, which put `board-vs-system` in it — so a reader who wants every board this build has turns on this one flag | as that row | as that row | as that row | **off** | `cargo build -p zax-nes --features parked-boards` |

**Worst figure among the features in `default`: +0.0020 % on one ROM**, against a tolerance of
+0.02 %. That figure is `board-116`'s, measured by MF-6a3 step 4; the two features MF-CART step 5
brought in are +0.001 % and +0.002 % on their worst ROM, and the tree they came into is
**−25 551 352 (−0.032 %)** on PR-1's thirteen against `44f77f72`, the lot's own baseline.

**And the worst figure of the one feature that is *not* in `default` is +0.083 %**, four times the
tolerance: `board-vs-system`'s, measured by MF-VS step 7. What makes it the first row of this table
whose cost is **not** its own board's is where the figure sits —
`Ppu2C02::run_dots` +1.1 M to +2.4 M and `Ppu2C02::background_fetch` +0.2 M to +2.4 M, on **every**
ROM of the set, including the ones whose games have no VS layer to run. Those are per-dot paths the
lot does not touch at any line. §5.16 of `rust/docs/PERFORMANCE-HISTORY.md` has the two probes that
attributed it: the `$2002` identifier is about a quarter, the 32 bytes step 2 added to `Ppu2C02`
are **nothing**, and `Ppu2C02::read_pattern_planes` is identical to the instruction in all three
builds.

## MF-CART lifted the limit, and these two sections are what it lifted

Everything from here to "How to turn a board off" is **MF-6c's state of knowledge, kept because
its eleven measurements are the record** — all of them of the same quantum — and superseded on one
point: the limit each shared enum had is **gone**, and `rust/docs/PERFORMANCE-HISTORY.md` §5.15
has the figures that replace it. Read the two sections below as the diagnosis the fix was built
from, not as advice for a new board.

## Two findings of MF-6c step 4 that change the advice in note 1 below

**1. Both shared variants are full, and the figure is quantised.**
`Cartridge::default_cpu_read`'s cost on `mmc5exram` moves in units of exactly **7 456 756
instructions over 300 frames (+0.066 %)**, and a variant past a shared enum's own limit buys one
unit — in the inlined `Board::prg_ram_enabled` dispatch, where LLVM hoists
`leaq 240(%rdi), %rax` above the jump table so that it runs on every CPU read of the one board
that maps PRG-RAM into $8000-$DFFF. Four measurements:

| | variants | `mmc5` |
| --- | ---: | ---: |
| `SmallBoard` + the two Taito boards (step 2, seven shapes) | 11, 12 | **+1 unit** |
| the same, on a tree 48 instructions larger (step 3) | 11, 12 | **+1 unit**, identical |
| `SmallBoard` + the Sunsoft-4 alone, one forward (step 4) | 11 | **+1 unit**, identical |
| `IrqBoard` + the Sunsoft-4 (step 4) | 10 | **−35 125** |
| `IrqBoard` + the two Taito boards as well (step 5) | 11, 12 | **+2 units** |

So `SmallBoard` has been the recommended home for a register-decode board since MF-6a step 2 and
that advice has run out; `IrqBoard` had room for exactly one more and the Sunsoft-4 took it. **A
board that is a register decode and nothing else now has no free home**, a `Board` variant of its
own costs the +0.080 % MF-6a3 step 1 measured, and the next lot that needs one has to redesign the
$6000-$7FFF read path — for instance by caching the decode on `Cartridge` the way `rom_windows`
already is, which is the one shape MF-6c's eleven measurements did not try.

> **MF-CART is the lot that did that, and the paragraph above is why it exists.** Caching the
> decode was half of it and was not enough on its own — its step 3 measured the same quantum,
> four units — and what finished it was `#[inline(always)]` on `Cartridge::default_cpu_read`,
> the shape MF-6c step 2 had measured and rejected and which the cache made safe. **All three
> enums' limits are void**: §5.15 of `PERFORMANCE-HISTORY.md` has the four configurations
> MF-CART step 6 measured.

**2. Q-123's ceiling does not reproduce.** MF-5a measured a third `Mapper::read_nametable` arm
body at `Ppu2C02::read_nametable_inner` **132 → 109 for every game in the library**. On this tree
that function is **114 → 75** — the body leaving by a tail call, MF-5a's own second shape and not
its third — and the thirteen ROMs of lot PR-1 move by **+17 007 in total** with `readbuf` **−78
628**. The dispatch is reached only behind `page == SERVED_BY_CARTRIDGE`, so a board that does not
serve its own nametables never calls it. Mapper 68's CHR-ROM nametable mode therefore ships in
`default`, and a **fourth** body should be re-measured the same way rather than assumed to cost
132 → 109.

## The boards that were parked, and why (MF-6c's account, superseded by MF-CART)

`board-taito-x1` is **+0.066 % on one ROM of one instrument and zero everywhere else**, and the
whole of it is a single instruction. `Cartridge::default_cpu_read` has exactly **two** measured
layouts on `mmc5exram` — **283 356 728** and **290 813 484** exclusive instructions, 7 456 756
apart — and the dearer one is the layout in which LLVM hoists `leaq 240(%rdi), %rax`, the address
of `self.board` wanted only by the *cold* arms of the inlined `Board::prg_ram_enabled` jump table,
**above** that dispatch, where it runs on every call. The MMC5 is the one board that maps PRG-RAM
into $8000-$DFFF, so every CPU read of an MMC5 game reaches that function.

**The two states are a coin this project has now seen land three times, twice the other way**:
MF-6b steps 2-5 were charged `default_cpu_read` **+7 456 756** on `mmc5` for `IrqBoard`'s sixth
body, MF-6a3 step 4 was given **−7 456 756** back for its eighth and ninth, and MF-6c step 2 was
charged it again. `8496c503` sits on the favourable side by luck.

**What the step could not find is a shape of the Taito code that avoids it.** MF-6c step 2
measured **seven** — moving the window decode from `cpu_peek_override` to `try_read_prg`, taking
either forward out of line, pinning one, two and three links of the chain, and taking
`Board::prg_ram_offset` out of `default_cpu_read` — and MF-6c step 3 measured an **eighth**: the
same two boards on a tree where that function had grown from 377 to **425** instructions for
unrelated reasons and `mmc5` was *below* the baseline. The figure was +7 456 756 every time, to
the instruction. So the trigger is the two `SmallBoard` variants themselves and not the size of
the function or the shape of the boards' own code, and nothing short of not having the variants
moves it.

So the park is the owner's rule applied, not a verdict on the boards: with the feature **on**, all
five level-2 `core` figures are **+0** and the worst `total` is `smb3` at +0.003 %, on the
instrument that loads a 3D pack. A later lot that adds any body to that chain may hand the figure
straight back.

> **It did better than that.** MF-CART step 5 re-measured both features on a tree where the
> function the figure lived in no longer exists, and they are `mmc5` **−0.112 %** and
> **−0.111 %** with worst ROMs of **+0.001 %** and **+0.002 %**. Both are in `default` and
> `parked-boards` is empty.

## How to turn a board off

There is no `--no-feature` flag in Cargo, so turning one board off means naming the others:

    cargo build -p zax-nes --no-default-features --features \
        board-94,board-105,board-111,board-116,board-153,board-155,board-158,board-68,\
        board-68-chr-nametables,board-77,board-96,board-eeprom,board-jaleco-72-92-86,\
        board-mmc3-multicart,board-namcot108,board-small-boards,board-taito-x1
    # ... which is every board but `board-119`. `board-vs-system` is in neither list: it is
    # parked, so `default` does not have it and `--features parked-boards` is how it comes on.

and `cargo test -p zax-nes --no-default-features` is the configuration with **every** board off —
**783 passed, 0 failed, 7 ignored**, against **862** by default since MF-CART step 5 put the last
two features in it. `--features parked-boards` is **no longer** the same build as `default`: MF-VS step 7 put
`board-vs-system` in the umbrella, so that configuration is the one with the VS. System and its 45
tests (**907 passed, 0 failed, 7 ignored** against 862 by default). Every configuration (each
feature alone, none, all, and all plus `parked-boards`) builds, passes clippy `-D warnings` and
passes its tests; MF-6a3 step 3's table has the per-feature test counts.

A downstream crate reaches the features through its dependency: `--features zax-nes/board-119`.

## Three notes for whoever revisits these choices

**1. The granularity is the granularity the measurements exist at.** `board-small-boards` is
eleven mapper numbers because they are **one `Board` variant** — which is the whole reason
`SmallBoard` exists: MF-6a step 2 measured five *variants* at 0.067 % to 0.131 % on eight of
thirteen ROMs and one variant at nothing. Splitting it into nine features would need **nine
measurements no lot has made**: MF-6a priced its steps 2, 3 and 4, not its individual boards. The
same goes for `board-namcot108`'s five numbers, which are one board with four parameters.

**2. The three MF-6b steps that are *not* in the table are not an omission.** Steps 1 (mapper 73),
2 (65 and 67) and 4 (48) each shipped with every one of PR-1's thirteen ROMs and all five level-2
games at or below `9f9d9d41` on total *and* core — `l2-diff.py`'s own verdict line for all three
is "at or below the baseline on every game, total and core". No figure rose, so no feature. MF-6a2
added no board at all.

**3. A board's cost is not its own, and the lot that proved it is MF-6a3** — and MF-CART is the
lot that stopped it being true of the one path where it mattered most. Three of MF-6a3's
measurements are the same phenomenon:

* two new `Board` discriminants cost `mmc5` **+8 994 278 (+0.080 %)** — one taken `jmp` per
  expansion-audio sample, because the 35-entry jump table of `Board::expansion_audio_output`
  grew by two and LLVM moved the fall-through off the MMC5's arm (step 1);
* the eighth and ninth bodies of `IrqBoard`'s inner match gave `Cartridge::default_cpu_read`
  **−7 456 756** on the same ROM, which is *to the instruction* the figure MF-6b steps 2 to 5
  charged it for the sixth (step 4);
* retrofitting nine features onto the existing boards moved thirteen ROMs by at most **±40 812**
  with **no symbol above 200 000 changing at all** (step 3) — the proof that the gating itself is
  free when every feature is on.

So a feature's measured cost is **the figure its own step saw in its own tree**, and turning two
features off does not subtract two costs. A reader who wants a board's cost in *today's* tree has
to measure that configuration; `rust/docs/PERFORMANCE-HISTORY.md` §2 and §3 have the method and
`rust/tools/perf/` the scripts.
