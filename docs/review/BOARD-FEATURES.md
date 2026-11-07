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

## Two features in this table are not boards: the PPU features

The owner's rule is written for boards, but what is behind it is the **figure**, so lot MF-4e
applied it unchanged to two pieces of the PPU. Lot MF-4e-gate then added a second reason a piece of
the PPU can be left out of `default`, and it is **not** a figure:

> **A change that moves what the player sees in a game the owner watches waits for the owner's eye,
> whatever it costs.**

Both features are out of `default` and **neither is in `parked-boards`**: that umbrella is for
left-out *boards*, and a build that wants everything asks for the flags by name. They are
independent — the four combinations score 128 / 129 / 131 / 132 on `AccuracyCoin` — and the one
command that gives both is

    cargo build -p zax-nes     --features ppu-2007-render-read,ppu-exact-sprite-eval
    cargo build -p zax-harness --features ppu-2007-render-read,ppu-exact-sprite-eval

`zax-harness` needs the flags too, through its own pass-throughs: Cargo cannot see a dependency's
feature from a test, and `req_core_110_accuracy_coin_keeps_the_rows_mf_4b_to_mf_4e_turned_green`
has to know which configuration it is scoring.

### `ppu-2007-render-read` — the `$2007`-during-rendering rule (MF-4e step 2, gated by MF-4e-gate)

With the feature on, a `$2007` access — the read *and* the register-7 write; the name follows
`AccuracyCoin`'s row and not the code — made while the PPU is fetching moves `v` by a **coarse X
increment and a Y increment at once** instead of by PPUCTRL's 1 or 32 (nesdev, "PPU scrolling",
"$2007 (PPUDATA) reads and writes"; `third_party/mesen2`'s `UpdateVideoRamAddr` takes the same
branch). The borrowed increment reaches `v_row_next` but **not** `v_row_start`, which is what lets
AMD-021 R2-005 criterion 2 hold with it. With it off, `Ppu2C02::increment_vram_address` is
`11407930`'s body line for line.

| | figure |
| --- | --- |
| what it wins | `AccuracyCoin` **128 → 129 / 147**: `PPU Misc. / $2007 read w/ rendering` (132 with `ppu-exact-sprite-eval` beside it) |
| what it changes on screen | **4 ROMs in golden-core, which runs with no hack code**: `Super Mario Bros. 3 (USA)`, `Zelda II (USA)`, `Zelda II (Europe)` and `nes-test-roms/other/Duelito.nes`, all on `pixel_sha256` *and* `tiles_sha256`. The captured-tile counts do **not** move — SMB3 960, both Zelda IIs 1 086 — only the contents |
| PR-1's thirteen | **inside the tolerance**: +0.000 % to +0.005 % on the eleven ROMs whose behaviour does not move, worst `readbuf` +0.002 %, `m4` −0.005 %. `duelito` −4.022 % and `smb3` +0.241 % are the two whose *behaviour* moves, read out of the performance question |
| level 2 | every `core` figure +0.000 % to +0.002 %. The one `total` outside is `smb1` +0.021 %, and its stage table says why: `ppu`, `apu` and `draw_list` are **identical to the instruction** and the whole move is in `convert` and `lua_hooks` — the 3D engine converting a different frame |
| gated off | the default build is **byte-identical to `11407930` in golden-core over the 98 commercial originals**, and no higher than it on either callgrind run (MF-4e-gate steps 4 and 5) |
| in `default` | **off** |
| the command | `cargo build -p zax-nes --features ppu-2007-render-read`, and `cargo build -p zax-harness --features ppu-2007-render-read` so the `AccuracyCoin` guard scores the right configuration |

**Why it is off, which is not the usual reason.** It is **inside** the tolerance; it was in
`default` when MF-4e shipped. R0's dense A/B of the 3D packs against **v0.4.2** (2026-11-07) found
it moving the picture in exactly the two games the owner watches most — *Super Mario Bros. 3*,
**58 of 681 sampled frames** over 5 dumps, and *Zelda II*, **18-19 of 1 001** over 2 — with and
without a pack. And SMB3's split line under the widescreen hack is where the owner saw a bug the
last time this rule shipped: lot MF-4d shipped it, the ground row above the status bar came out cut
and shifted by half a block, and MF-4d-fix backed it out. **The rule may well be right** — the
hardware sources are not in doubt and MF-4e reconciled it with R2-005 — but the owner validates it
by eye before it is the default, and until then the default build renders this path as v0.4.2 does.

**What the gate buys back, beyond the picture.** MF-4e step 2 had to relax one assertion of
`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line` from `smallest_gap >= 3` to `>= 2`,
because the rule moves the banner's first row from line 195 to line 194 and that puts lines 192 and
194 two apart. Q-137 had to ask R0 for the relaxation by name. With the rule out of `default`, **the
restatement of a P1 requirement is back at full strength in the build the owner runs** and the
relaxed form only exists in a build asked for by name. `SMALLEST_GAP` in that test is the one knob.

### `ppu-exact-sprite-eval` — the exact sprite-evaluation pipeline (MF-4e step 3)

With the feature on, the sprite evaluation is the hardware's byte counter in two independently
clocked halves against a second counter over the secondary OAM, and a `$2004` read during rendering
answers from the OAM copy buffer instead of from `oam[oam_address]`.

| | figure |
| --- | --- |
| what it wins | `AccuracyCoin` **+3 rows**: `Address $2004 behavior`, `Arbitrary Sprite zero`, `Misaligned OAM behavior` — 128 → **131 / 147**, or 129 → **132** with `ppu-2007-render-read` beside it |
| what it changes on screen | **no commercial original at all** (`diff` clean over all 18 of MF-4e's set) and **two free ROMs**, both test ROMs for the very behaviour it changes: `rastesam4.nes` and `read2004.nes` |
| PR-1's thirteen | **above on nine of the eleven** whose behaviour does not move: `metroid` **+0.133 %**, `z2` +0.107 %, `mm2` and `smb1` +0.083 %, down to `m4` +0.041 %; `vblnmi` +0.008 % and `oamstr` +0.001 % inside |
| level 2 | **all ten above**: `zelda2/core` **+0.127 %** down to `smb3/total` +0.057 % |
| gated off | set total **−91 994 (−0.000 %)**, worst ROM **+0.001 %**, and `run_sprite_evaluation`, `background_fetch`, `run_dots`, `paint_sprite` and `rebuild_sprite_line` **identical to the instruction** on five ROMs |
| in `default` | **off** |
| the command | `cargo build -p zax-nes --features ppu-exact-sprite-eval`, and `cargo build -p zax-harness --features ppu-exact-sprite-eval` so the `AccuracyCoin` guard scores the right configuration |

**Why it is off: the ordinary reason, the figure.** Above the +0.02 % tolerance on nine of PR-1's
eleven and on all ten level-2 figures, against a path `run_dots` makes 57 % to 62 % of
`NesCore::run_frame` on all five level-2 games. Not dropped — one flag gives the exact pipeline.

## The boards, and the one instruction that used to park two of them

**`board-taito-x1` and `board-111` were parked on one instruction, and it was not theirs — MF-CART
removed the instruction.**
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
| `board-jv001` | 36, 132, 136, 147, 172, 173 | the **TXC 05-00002-010 / Sachen JV001** protection ASIC: one chip on six boards from four publishers, with the CPU data-bus wiring, the accumulator width, the banking pins and the one mirroring wire as fields of a single `DecodeBoard` variant. One forward that enum did not have, `try_read_prg` (`#[inline(never)]`), because the protection register is at $4100. Implies `board-decode-boards` | **~20** TXC, Idea-Tek, Super Mega and Sachen originals: *Strike Wolf*, *Policeman*, *F-15 City War*, *棋王*, *Creatom*, *Rad Racket*, *1991 賭馬 Racing*, *Venice Beach Volley*, *撲克精靈*, *未来小子*, *蝶變*, *少林武者* | measured **with the lot's other two**, against `481f683d`: worst ROM `smb1` **+47 883 (+0.001 %)**, set total **-148 638 (-0.000 %)**, five of thirteen below (MF-UNL-1 step 6) | **all five `core` at or below the base** -- `smb3/core` **-20 823**, `mm2/core` -18 944, `metroid/core` -9 952, `smb1/core` and `zelda2/core` **+0**. Four `total` inside at +0.001 % to +0.009 %; `smb3/total` **+0.023 %** is outside and is **Lua's `ltable.c`** -- see the note under the table | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-sachen-8reg` | 150, 243 | **Sachen's eight-register ASIC** (SA-015 / SA-630 and SA-020A), one class in Mesen 2 and one variant here: an index port and a data port under mask $C101, three bits wide, R0/R1/R3 stored and inert, and R7's four nametable layouts. One forward, `nametable_page`, for the `S0-S0-S0-S1` mode no `NametableMirroring` can say. Implies `board-decode-boards` | **15** Sachen titles: *Tasac*, *将棋学園*, *Strategist*, *Olympic I.Q.*, *美女拳 — Honey Peach* among them | measured **with the lot's other two**, against `481f683d`: worst ROM `smb1` **+47 883 (+0.001 %)**, set total **-148 638 (-0.000 %)**, five of thirteen below (MF-UNL-1 step 6) | **all five `core` at or below the base** -- `smb3/core` **-20 823**, `mm2/core` -18 944, `metroid/core` -9 952, `smb1/core` and `zelda2/core` **+0**. Four `total` inside at +0.001 % to +0.009 %; `smb3/total` **+0.023 %** is outside and is **Lua's `ltable.c`** -- see the note under the table | **on** | as above |
| `board-133` | 133 | the **Sachen 72008** (`UNL-SA-72008`): one latch, `$4100: [.... .PCC]` under mask $E100. The page's 60-pin Sachen 3009 decode — a Namco-108 clone with an analog feedback path — is **not** built, on the page's own sentence that "only the 72-pin variant requires emulation"; `Sachen72008`'s doc comment records it. One `DecodeBoard` variant, no new forward. Implies `board-decode-boards` | 2: *Jovial Race*, *盜帥 — Master Chu and the Drunkard Hu* | measured **with the lot's other two**, against `481f683d`: worst ROM `smb1` **+47 883 (+0.001 %)**, set total **-148 638 (-0.000 %)**, five of thirteen below (MF-UNL-1 step 6) | **all five `core` at or below the base** -- `smb3/core` **-20 823**, `mm2/core` -18 944, `metroid/core` -9 952, `smb1/core` and `zelda2/core` **+0**. Four `total` inside at +0.001 % to +0.009 %; `smb3/total` **+0.023 %** is outside and is **Lua's `ltable.c`** -- see the note under the table | **on** | as above |
| `board-mmc3-chrram` | 74, 192, 194, 195 | the **other four members of mapper 119's own family**: an MMC3 with a CHR-RAM chip beside its CHR-ROM, which is how a Chinese game draws logographic characters. Four `const` monomorphisations of `Mmc3Board` behind **one** boxed `IrqBoard` variant holding the inner enum `Mmc3ChrRamBoard`, and **it implies `board-119`** because the mechanism is that feature's -- the chip select, the two `#[inline(never)]` fetch bodies, the byte of per-slot source flags. MF-UNL-2 step 1 generalised 119's one `const bool` into the **four columns** of nesdev's *MMC3 with CHR ROM and CHR RAM* (first RAM bank, size, `RAM mirrored?`), and `update_banks` comes out of the compiler **identical to the instruction** on `m4` and `smb3`. 195 carries the page's run-time **GAL** besides, one three-bit field selecting eight windows, latched by a PPU write to a CHR-ROM window (Q-142) | **6** Waixing originals and ports: *哥伦布传*, *天神之剑*, *甲A*, *第四次: 机器人大战*, *機甲戰士*, *Ying Lie Qun Xia Zhuan* | measured **with the lot's other three**, against `186b5656`: worst ROM `mm2` **+54 574 (+0.001 %)**, set total **-585 425 (-0.000 %)**, eight of thirteen below -- `m4`, the MMC3 this lot generalises, is the biggest gain at **-280 090** (MF-UNL-2 step 8) | **all five `core` inside at +0.000 % to +0.001 %**, `smb1/core` **+0**. Four `total` inside at -0.007 % to +0.000 %, three of them *below* the base; `smb3/total` **+0.039 %** is outside and is **Lua's `ltable.c`** -- see the note under the table | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-mmc3-scramble` | 114, 115, 121, 123, 182, 187 | the **six MMC3 clones with scrambled registers**, from SuperGame, Hosenkan and 卡聖 (Kǎshèng): one board with six wirings -- two scramble rows (address and index are **separate** choices, 123 having only the second), three NROM-override ports with three bit orders, an outer 256 KiB CHR bank, CHR A18 driven from **PPU A12** on the A9711 and the A98402, the MMC3A counter on 114 and 182, and 121's protection array with its `$8003` table of bit-reversed `$8001` values. One **boxed** `IrqBoard` variant holding an `Mmc3`, and the first board to make that enum forward `Mapper::try_read_prg` -- the dispatch **MF-CART freed**, which this lot re-measured rather than taking on trust. **Q-144** holds four places this family's page beats Mesen 2, of which 123's `$5800` bit order is observable | **~14** originals: *Aladdin*, *The Lion King*, *Boogerman*, *Pocohontas*, *Super Donkey Kong*, the six Gouder titles Kǎshèng re-released, *Mortal Kombat 3*, *Earthworm Jim 2*, *Street Fighter Zero 2*, *The King of Fighters '96* | measured **with the lot's other three**, against `186b5656`: worst ROM `mm2` **+54 574 (+0.001 %)**, set total **-585 425 (-0.000 %)**, eight of thirteen below -- `m4`, the MMC3 this lot generalises, is the biggest gain at **-280 090** (MF-UNL-2 step 8) | **all five `core` inside at +0.000 % to +0.001 %**, `smb1/core` **+0**. Four `total` inside at -0.007 % to +0.000 %, three of them *below* the base; `smb3/total` **+0.039 %** is outside and is **Lua's `ltable.c`** -- see the note under the table | **on** | as above |
| `board-263` | 263 | the Rex Soft **UNL-KOF97**: an MMC3 clone "with scrambled mapper address and data bits" and no register of its own -- the board's **A12 drives the chip's A0**, which one line reproduces, and the written byte is permuted `76543210 -> 76243051`. One `IrqBoard` variant, **unboxed**, holding an `Mmc3` and two translations in front of it. The page states the permutation twice, as a bit map and as an expression, so the test builds it from the map and checks all 256 bytes against the board | 2: *The King of Fighters '97*, *Boogerman II: The Final Adventure* | measured **with the lot's other three**, against `186b5656`: worst ROM `mm2` **+54 574 (+0.001 %)**, set total **-585 425 (-0.000 %)**, eight of thirteen below -- `m4`, the MMC3 this lot generalises, is the biggest gain at **-280 090** (MF-UNL-2 step 8) | **all five `core` inside at +0.000 % to +0.001 %**, `smb1/core` **+0**. Four `total` inside at -0.007 % to +0.000 %, three of them *below* the base; `smb3/total` **+0.039 %** is outside and is **Lua's `ltable.c`** -- see the note under the table | **on** | as above |
| `board-189` | 189 | the TXC **01-22017-000**/**01-22018-400** and the Yoko **FC-001EM**: an MMC3 clone whose PRG-ROM is banked in **32 KiB** amounts from a register at `$4100`, so the chip's own PRG bank lines reach nothing. One `IrqBoard` variant, **unboxed**, holding an `Mmc3` plus one byte. The page's "address bit 9" is **A8**, which its mask `$E100`, its address diagram and the `$4132` its own oversize game writes all say; three bits of the byte are taken so that **one decode runs the original and the oversize board**. The **generic pirate** variants (A-37, K-3086), which move the register onto the PRG-RAM interface, are deliberately not decoded on the page's own paragraph about them | 2 originals and their many variants: *Thunder Warrior — 轰天至尊*, *Street Fighter II: The World Warrior* | measured **with the lot's other three**, against `186b5656`: worst ROM `mm2` **+54 574 (+0.001 %)**, set total **-585 425 (-0.000 %)**, eight of thirteen below -- `m4`, the MMC3 this lot generalises, is the biggest gain at **-280 090** (MF-UNL-2 step 8) | **all five `core` inside at +0.000 % to +0.001 %**, `smb1/core` **+0**. Four `total` inside at -0.007 % to +0.000 %, three of them *below* the base; `smb3/total` **+0.039 %** is outside and is **Lua's `ltable.c`** -- see the note under the table | **on** | as above |
| `board-nina03` | 79, 146 | the **AVE NINA-03/06** and the 60-pin Sachen **3015** / **SA-016** that nesdev's one page for both numbers calls "functionally identical": one register, `$4100: [.... PCCC]` under mask $E100, a 32 KiB PRG bank and an 8 KiB CHR bank. One `DecodeBoard` variant. **FCEUX's extra CNROM switch at $8000-$FFFF is deliberately not implemented** -- the page's own last line attributes a latch there to mapper **148** and gives this board "Bus conflicts: No", so a dump that needs it is a mislabelled mapper-3 image and its header is the fix; a test pins the silence on four addresses. Implies `board-decode-boards` | **~19** AVE titles -- *Dudes with Attitude*, *Deathbots*, *Krazy Kreatures*, *Tiles of Fate*, *Double Strike*, *Impossible Mission II*, *Solitaire*, *Blackjack*, *Pyramid*, *Mermaids of Atlantis*, *Ultimate League Soccer*, *Wally Bear and the NO! Gang* -- plus Sachen's *Magical Mathematics*: the largest single catalogue in the census | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-38` | 38 | the Bit Corp. **UNL-PCI556** of *Crime Busters*: `[xxxx CCPP]` at $7000-$7FFF. The page's "writes to $F000-$FFFF are **very likely** to also trigger a bankswitch" is implemented as the same decode (A15 not read) and recorded in **Q-147**, because the page says "very likely" rather than asserting it. "PRG RAM capacity: None", so `board_memory_sizes` clears $6000-$7FFF and the latch is reachable at all. One of the seven boards of the shared `SingleLatch` variant | 1 (*Crime Busters*) | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-107` | 107 | the **Magicseries** board of *Magic Dragon*: one `[BBBB BBBb]` register over the whole of $8000-$FFFF driving both banks -- the 8 KiB CHR bank from **all eight** bits and the 32 KiB PRG bank from the upper **seven**, which is the page's diagram and `MesenCE`'s reading where FCEUX takes three bits. Its "it's currently **unknown** whether this mapper suffers from bus conflicts" is answered **no** (**Q-147**): a board wrongly given conflicts latches `written & ROM` on every write. `SingleLatch` | 1 (*Magic Dragon*) | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-143` | 143 | the Sachen **TCA-01** of *Dancing Blocks* and *Magical Mathematics*: "an NROM circuit board with a simple form of protection", whose whole register section is a **read** -- the logical inversion of the **address**'s bits A5-A0 under mask $E100, with bits 7-6 open bus and **no bankswitching register at all**. The census asked for the inversion of the *written* bits, which a board of "a 74138 and a 74368" cannot do; **Q-145** records that the page wins. The one board of this lot that needs a forward, `try_read_prg`, which `DecodeBoard` has carried since MF-UNL-1. `SingleLatch` | 2 (*Dancing Blocks*, *Magical Mathematics*) | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-144` | 144 | the **AGCI 50282** of *Death Race*: "a intentionally defective variant of the Color Dreams board (mapper 11)" with "a 300 ohm resistor ... between CPU D0 and the combination of mapper hardware and ROM", so that "only the ROM's least significant bit always wins bus conflicts". The register is mapper 11's `[CCCC PPPP]`, read exactly as `ColorDreams` reads it; the latched value is the page's own "most succinct" formula `ROM[address] & (written | 1)`. **The first board here whose bus conflict is not the generic AND**, so it is deliberately absent from `registry::has_bus_conflicts` -- that path would AND bit 0 away before the board saw it. **Q-146** holds the page's three candidates and the census's own figure, which none of them produces. `SingleLatch` | 1 (*Death Race*) | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-145` | 145 | the Sachen **SA-72007** of *Sidewinder*: `$4100: [C... ....]` under mask $E100, one 8 KiB CHR bank bit -- and the bit is **bit 7**, which is the one thing the page's diagram says and "Yet Another CNROM clone" does not. No PRG bank line. `SingleLatch` | 1 (*Sidewinder*) | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-149` | 149 | the Sachen **SA-0036** of *Taiwan Mahjong 16*: the same one bit, "just like mapper 145, but moves the register to overlap with PRG ROM and so **has bus conflicts**" -- mask **$8000**, so every address in $8000-$FFFF is the register, and the AND with the ROM byte is `registry::has_bus_conflicts`'s generic path. `SingleLatch` | 1 (*Taiwan Mahjong 16*) | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `board-190` | 190 | the **Zemina** board of *Magic Kid Googoo*, the one of the lot's seven with more than one window: a 16 KiB PRG bank at $8000-$BFFF whose top bit is **the address's A14** ($8000-$9FFF writes the low eight banks, $C000-$DFFF the high eight), 16 KiB fixed to the **first** bank at $C000-$FFFF, and four 2 KiB CHR banks written at $A000-$BFFF with A1-A0 picking the window. 8 KiB of RAM at $6000-$7FFF, which its *Banks* section states and `board_memory_sizes` therefore does too. Its four CHR bytes are in the shared variant's one savestate block, as mapper 36's CHR latch is in `Jv001`'s. `SingleLatch` | 1 (*Magic Kid Googoo*) | measured **with the lot's other seven**, against `452e8464`: worst ROM `m1` / `oamstr` **+0.003 %** (+123 989 and +83 513), set total **+595 604 (+0.001 %)**, and the only symbol of this project that moves is `NesBus::write_internal` **+74 561** on `m1` -- the eight arms this lot adds to the `DecodeBoard` match that `Board::write_register` inlines (MF-UNL-3 step 7) | **all five `core` figures inside**, three of them **+0, identical to the instruction** (`smb1`, `smb3`, `zelda2`) and the other two +0.000 % (`metroid` +9 952, `mm2` +18 944). **All five `total` figures inside too**: -0.012 % (`smb1`) to **+0.013 %** (`smb3`), whose per-function diff is `luaH_getshortstr` +3 427 861 with no symbol of this project moving by 300 000. See the note under the table on what the `total` column can resolve | **on** | `cargo build -p zax-nes --no-default-features --features <the others>` |
| `parked-boards` | 99, 151 | the umbrella: every board left out of `default`. Empty from MF-CART step 5 until **MF-VS step 7**, which put `board-vs-system` in it — so a reader who wants every board this build has turns on this one flag | as that row | as that row | as that row | **off** | `cargo build -p zax-nes --features parked-boards` |

**Worst figure among the features in `default`: +0.003 % on one ROM**, against a tolerance of
+0.02 %. That figure is MF-UNL-3's, measured on `m1` and `oamstr` with its eight features
together; before that lot it was `board-116`'s +0.0020 %. That figure is `board-116`'s, measured by MF-6a3 step 4; the two features MF-CART step 5
brought in are +0.001 % and +0.002 % on their worst ROM, and the tree they came into is
**−25 551 352 (−0.032 %)** on PR-1's thirteen against `44f77f72`, the lot's own baseline.

**And the worst figure of the one *board* feature that is not in `default` is +0.083 %**, four times
the tolerance: `board-vs-system`'s, measured by MF-VS step 7. (The two PPU features above are also
out of `default`, one of them for a reason that is not a figure at all.) What makes it the first row
of this table
whose cost is **not** its own board's is where the figure sits —
`Ppu2C02::run_dots` +1.1 M to +2.4 M and `Ppu2C02::background_fetch` +0.2 M to +2.4 M, on **every**
ROM of the set, including the ones whose games have no VS layer to run. Those are per-dot paths the
lot does not touch at any line. §5.16 of `rust/docs/PERFORMANCE-HISTORY.md` has the two probes that
attributed it: the `$2002` identifier is about a quarter, the 32 bytes step 2 added to `Ppu2C02`
are **nothing**, and `Ppu2C02::read_pattern_planes` is identical to the instruction in all three
builds.

## MF-UNL-1's one figure outside the tolerance, and the pair that puts it on Lua

`smb3/total` is **+0.023 %** with the lot's three features on, against a tolerance of +0.02 %,
and the three boards are in `default` all the same. The argument is `board-77`'s row's, with one
measurement that row did not have.

**Where the figure is.** The per-function diff of that game's run gives
`luaH_getshortstr` **+23 042 742 / -20 058 471**, `getgeneric` **+9 291 059 / -7 043 815** and
`luaH_set` **+330 712** against a whole-program **+5 668 674** — so Lua's string-key hash table
*is* the figure. No symbol of this project moves by more than **20 823** on that game, and the
one that does, `Board::chr_peek_view`, moves **down**. `core`, `ppu`, `apu` and `draw_list` are
`+0` or below; `convert` and `lua_hooks` carry all of it.

**The pair.** MF-UNL-1 step 6 built a **same-behaviour** variant of its own tree — two disjoint
`registry::create_board` arms swapped, identical emulation — and measured it on the same
instrument. Against each other the two binaries are `smb1/total` **+0.016 %**, `zelda2/total`
+0.007 %, `smb3/total` **−0.010 %**, `metroid/total` −0.001 %, `mm2/total` −0.000 %, and **all
five `core` figures identical to the instruction**. Against the base commit the variant is
`smb1/total` **+0.024 %** where the lot's own tree is +0.008 %, and `smb3/total` +0.013 % where
it is +0.023 %.

So the `total` column has a spread of about ±0.02 % between two builds whose emulation is the
same byte for byte, and the `core` column has none. **A `total` figure inside that spread is not
a board's cost**, and the three features are measured where the instrument can tell: PR-1's
thirteen (worst +0.001 %) and level 2's five `core` figures (all at or below the base).

**And the gating is free.** The same tree with the three features out of `default` — no new board
code reachable at all — is PR-1 set total **−7 224 (−0.000 %)**, worst ROM +0.001 %, all five
level-2 `core` **+0**, worst `total` `smb1` +0.004 %.

## MF-UNL-3's two control builds, and why `total` is not the column the tolerance can use

Lot MF-UNL-3 is the first of the MF-UNL lots with **no figure outside the tolerance on either
instrument** -- worst PR-1 ROM **+0.003 %**, all five level-2 `core` figures at **+0.000 %** (three
of them identical to the instruction), worst `total` **`smb3` +0.013 %** -- and it is also the lot
that measured the `total` column's spread most directly, because it built **two** controls instead
of one.

| build | what it is | level-2 `core` | level-2 `total` |
| --- | --- | --- | --- |
| `unl3` | the lot, all eight features in `default` | **+0** on three, +0.000 % on two | -0.012 % to **+0.013 %** |
| `unl3off` | **the same source with the eight features out of `default`** -- no new board code reachable, so the emulation is the base commit's | **all five +0, identical to the instruction** | -0.001 % to **+0.022 %** (`smb3`) |
| `unl3swap` | the lot's source with two **disjoint** `registry::create_board` arms exchanged (13 and 15) -- same boards, same registers, same emulation | **all five +0** | -0.002 % to **+0.016 %** (`smb1`) |

**The middle row is the finding.** A build whose emulation is the base commit's byte for byte is
`smb3/total` **+0.022 %** -- *above* the owner's tolerance, and above the lot's own +0.013 % --
while every one of its `core` figures is identical to the instruction. The third row says the same
thing from the other side: two binaries with identical emulation differ by +0.016 % on
`smb1/total`.

So on a level-2 run with a pack loaded, **`core` is the figure the tolerance applies to and `total`
is not resolvable below about 0.02 %**, which is the reading MF-UNL-2 step 7 introduced with a
0.034 % pair and which these two controls confirm from a build that changes no behaviour at all.
Where `total` moves, the per-function diff says what it is: on `smb3` the lot's +3.19 M in the
stage total is `luaH_getshortstr` **+3 427 861**, with no symbol of this project moving by 300 000.

PR-1's thirteen ROMs, where there is no pack and no Lua frame hook, are the instrument that *can*
resolve this lot: the same two controls are set total **-62 379** and **-97 496** there, against
the lot's own **+595 604 (+0.001 %)**.

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

### `smb3/total` and Lua's `ltable.c`, re-measured with a pair whose spread is twice the tolerance

Three lots have now shipped a board with `smb3/total` as their one figure outside the owner's
+0.02 %: `board-77` at +0.024 %, MF-UNL-1's three at +0.023 %, and **MF-UNL-2's four at
+0.039 %**. Every time the per-function diff has put the whole move in Lua's string-key hash
table, and MF-UNL-2 step 8 is the measurement that settles what the figure *is*.

Its `unl2swap` binary is `unl2`'s own source with **two disjoint arms of `registry::create_board`
exchanged** — mappers 13 and 15, neither of them in the lot. The same boards, the same registers,
the same emulation; a different binary layout. Against each other, over 600 frames of each
level-2 game with its pack:

| | `core` | `total` |
| --- | ---: | ---: |
| `smb1` | **+0** | −1 887 183 (**−0.014 %**) |
| `mm2` | **+0** | −61 914 (−0.000 %) |
| `metroid` | **+0** | +107 996 (+0.001 %) |
| `smb3` | **+0** | **−8 302 849 (−0.034 %)** |
| `zelda2` | **+0** | +1 938 474 (+0.013 %) |

**`core` is identical to the instruction on all five and `total` moves by −0.034 % to +0.013 %.**
So `smb3/total` has an instrument spread of 0.034 % on this tree — *larger than the tolerance it
is being judged against* — and the same pair gives that figure +0.005 % against the base commit
where `unl2` gives +0.039 %. The two binaries trade places.

What the figure is made of, on `unl2` against `unl2base`: `luaH_getshortstr` **+7 120 332** and
`getgeneric` **+2 247 528**, which is 9.37 M of the 9.39 M; no symbol of this project moves by
more than +817 768 (`Board::write_register`, paid back by −667 295 in `NesBus::write_internal`);
and the stage table has `ppu`, `apu` and `draw_list` **identical to the instruction** with the
whole move in `convert` (+0.308 %) and `lua_hooks` (+0.534 %) — the 3D engine converting against
a differently-laid-out heap.

**The reading this table takes**: on a level-2 run with a pack loaded, `core` is the figure the
owner's tolerance can be applied to and `total` is not resolvable below about 0.035 %. A board
whose level-2 `core` figures are at or below the base is **not** parked on a `total` that a
same-behaviour pair of binaries moves further than the tolerance itself. PR-1's thirteen ROMs,
which load no pack and run no Lua, show the same pair at worst ROM +0.001 % and set total
+0.000 % — so the instrument that *can* resolve these boards does, and says they cost nothing.

## How to turn a board off

There is no `--no-feature` flag in Cargo, so turning one board off means naming the others:

    cargo build -p zax-nes --no-default-features --features \
        board-94,board-105,board-111,board-116,board-153,board-155,board-158,board-68,\
        board-68-chr-nametables,board-77,board-96,board-eeprom,board-jaleco-72-92-86,\
        board-mmc3-multicart,board-namcot108,board-small-boards,board-taito-x1,\
        board-jv001,board-sachen-8reg,board-133,board-mmc3-chrram,board-mmc3-scramble,\
        board-263,board-189,\
        board-nina03,board-38,board-107,board-143,board-144,board-145,\
        board-149,board-190
    # ... which is every board but `board-119` -- except that `board-mmc3-chrram` **implies**
    # it, so this exact line still builds mapper 119. Turning 119 itself off means dropping
    # `board-mmc3-chrram` from the list too, the CHR-RAM mechanism being one feature's.
    # `board-vs-system` is in neither list: it is
    # parked, so `default` does not have it and `--features parked-boards` is how it comes on.

and `cargo test -p zax-nes --no-default-features` is the configuration with **every** board off —
**783 passed, 0 failed, 8 ignored** -- unchanged by MF-UNL-2 and by MF-UNL-3, every one of their
tests being behind one of their features -- against **909** by default since MF-UNL-3 added its
eight.
`--features parked-boards` is **no longer** the same build as `default`: MF-VS step 7 put
`board-vs-system` in the umbrella, so that configuration is the one with the VS. System and its 45
tests (**962 passed, 0 failed, 8 ignored** against 909 by default). Every configuration (each
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
