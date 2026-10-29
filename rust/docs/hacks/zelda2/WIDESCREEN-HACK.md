# Zelda II (USA): the widescreen hack

This document describes, completely, the modification of *Zelda II - The Adventure of Link (USA)*
that Zaxnes ships as a built-in patch: what it does, how, where every byte goes, how to build it
and how to check the result. The appendices hold the complete new sources and the exact
difference against the disassembly it starts from; with them the ROM can be rebuilt identically
(same CRC-32), and without them the text is meant to be enough to re-implement it.

The emulator side (the widescreen extension the hack talks to, the margin decoder, savestates)
is described in [`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md). Read its
section 2 (registers) first: the hack is a client of that protocol.

The patched ROM is not meant for real hardware: without the extension it would read open bus
where it expects registers.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Zelda II - The Adventure of Link (USA)*, MMC1, 128 KiB PRG (8 banks of 16 KiB, bank 7 fixed at `$C000`), 128 KiB CHR-ROM |
| Source data (iNES header excluded) | CRC-32 `BA322865`, SHA-1 `11333adb723a5975e0ecca3aee8f4747aa8d2d26` |
| Target data (iNES header excluded) | CRC-32 `ADCEA53F` (the hack's *version* in savestate headers) |
| Size | unchanged (262 144 bytes of data) |
| Patch | `rust/crates/zax-nes/patches/zelda2-usa-widescreen.bps`, BPS, made against the data without header; the player's header is kept |
| Base | FiendsOfTheElements *z2disassembly* (ca65), upstream commit `c4c3a4c` ("cleanup more reset code", 2023-01-18), plus one fix (1.1) |

### 1.1 The base and its differences with the ROM

Upstream commit `c4c3a4c` does not assemble to the USA ROM byte for byte. One change is needed
for the game to run at all, and the target keeps the others.

* **The fix (commit `c7103f0`)**: in `src/prg7.asm`, NMI, the loop that decrements the `$05xx`
  timers ends with `BPL :--` (upstream wrote `BPL :-`, which branches to the `DEX` and loops
  forever). The byte at `$C181` goes back to `$F5`, as in the ROM.
* **Kept from upstream** (852 bytes, all harmless): banks 0 to 6 end with `$90` zero bytes at
  `$BF70`-`$BFFF` where the ROM holds a copy of the reset code and vectors (the game always runs
  with bank 7 fixed at `$C000`); and five `JSR` that call that bank-local copy in the ROM call the
  fixed bank's instead: bank 0 `$814B` and `$8150` (`JSR $BF9D`, `JSR $BFB1` become `JSR $FF9D`,
  `JSR $FFB1`), bank 0 `$A86B` (`JSR $BFB1`), bank 5 `$A714` (`JSR $BF9D`) and `$A728`
  (`JSR $BFB1`): only the high byte of the target changes (`$BF` to `$FF`). The routines at both addresses are the same.

The target is therefore "the base (with the fix) + the hack". Appendix E lists every byte range
where the target differs from the ROM, so either can be checked.

---

**Hack pack.** Zaxnes ships this hack as a hack pack,
`rust/crates/zax-nes/patches/zelda2-usa-widescreen.zxpk`, built by the `z2pack` repository
(`docs/HACK-PACKS.md`, `WIDESCREEN-EXTENSION.md` 7.1): the hack moved out of the game. Its code
(this fork's `inc/wide_bank0.asm`, bank 0 part and window part together) is assembled for the
extension's memory at `$5800` (1953 bytes, with the pack's own code of 5.5), which Zaxnes shows
from power-on (`$5008` = `$80`, page 0; 2 KiB, the PRG-RAM staying at `$6000`); the game's code is
changed only by the hooks of section 4 onwards (all but the encounters' life, 6.1: 35), the same
bytes but for the addresses of the hack's routines, and the pack's own 11 (5.5). Not in the pack:
the power-on detour and the NMI's unlock and window reopening (Zaxnes keeps the window open; the
NMI hook calls `wr_nmi` directly), and the base's differences with the ROM (1.1), where the game
keeps its bytes. The segments of section 3 and the BPS patch describe this fork's build, kept to
compare.

## 2. What the hack does

The NES draws 256 pixels across. Zaxnes can show more: a ROM that unlocks the widescreen
extension tells the emulator how far past each edge its picture is valid, and places sprites
there. The hack makes Zelda II such a ROM:

1. **Side view** (towns, palaces, caves, fields): the background past the edges is what the game
   already keeps drawn in its two nametables (it streams a window of 27 metatile columns, 432
   pixels, around the screen); the hack tells the emulator how much of it lies past each edge,
   frame by frame. The status bar stays 256 wide.
2. **Enemies** in the side view are drawn past the edges (display only: the game's logic still
   works on its 8-bit screen x, so contact, removal and AI are unchanged), their sprites get the
   high byte of their 16-bit x, and new enemies spawn past the margins instead of popping in.
   Townsfolk walk in from past the margins. The hack pack (`z2pack`) goes further: enemies live
   as far as the margins need, and those the game makes at an edge of the screen come in from
   past the margin, stopping before a wall (5.5).
3. **World map**: the hack switches to four nametables, keeps 32 metatile columns drawn (at least
   112 pixels past each edge), scrolls on 9 bits, and makes the encounters ("blobs") live and move
   in the margins.
4. **CPU time**: drawing and running enemies in the margins costs time the console does not have;
   the hack asks for 48 extra scanlines per frame (extension overclock).
5. **Robustness**: the extension is unlocked at power-on and again by the NMI if it finds it
   locked (a state made without the hack restored with it).

The margin the hack uses is what the host offers, capped at **112 pixels** (`WIDE_MAX_MARGIN`):
what both the side view's window and the world map's ring always keep drawn. With the emulator's
Zelda II margin decoder, the margins can be wider than that (the decoder reads the level data
itself); the hack's own logic stays within 112.

Projectiles are deliberately not shown in the margins: the game destroys them at the edge of the
original screen.

---

## 3. Layout of the new code

The disassembly is not relocatable (the fixed bank jumps to some 66 hard addresses in the banks,
the banks to some 130 in the fixed bank), so nothing may move: every patch in existing code has the
same size as what it replaces, and new code goes only in space the ROM leaves unused (`$FF`
fill). `nes.cfg` gets new segments at those places, each followed by a segment that resumes the
original code at its original address:

| Segment | Bank | Start | Room | Holds |
|---|---|---|---|---|
| `WIDE0` | 0 | `$AA40` | to `$B081` (1602 bytes) | most of the hack (`inc/wide_bank0.asm`), replacing `bank0_unused_space_big` |
| `PRG0B` | 0 | `$B082` | | the rest of bank 0 |
| `WIDE6W` | 6 | `$AD00` | to `$B081` (898 bytes) | the code the fixed bank calls, assembled for `$5800` (`.org WV_CODE`), shown there by the extension's PRG window; replaces `Bank6__UNUSED_2` (`$ACF9`-`$B081`, the 7 bytes before `$AD00` become `$00` fill) |
| `PRG6B` | 6 | `$B082` | | the rest of bank 6 |
| `WIDE7A` | 7 | `$D39A` | 48 bytes | `wide7_nmi` |
| `PRG7B` | 7 | `$D3CA` | | |
| `WIDE7B` | 7 | `$FEAA` | 38 bytes | `wide7_unlock` (exactly full) |
| `PRG7C` | 7 | `$FED0` | | |
| `WIDE7C` | 7 | `$FF4C` | 36 bytes | `wide_reset` |
| `PRG7D` | 7 | `$FF70` | | |

ld65 fills the unused end of each new segment with `$00`. The window code is placed in bank 6
(not 0) because bank 0 has no room left for it; it runs at `$5800`-`$5B81` whatever bank is
mapped at `$8000`, because the extension shows bank 6 page `$2D` there (`$5008 = $86`,
`$5009 = $2D`).

### 3.1 Symbols and variables (`inc/wide.inc`)

Registers: `WIDE_SIG $5000`, `WIDE_HOST $5002`, `WIDE_REQ $5003`, `WIDE_LEFT $5004`,
`WIDE_RIGHT $5005`, `WIDE_OVERCLOCK $5006`, `WIDE_LAYOUT $5007`, `WIDE_WINDOW_BANK $5008`,
`WIDE_WINDOW_PAGE $5009`, `WIDE_SPRHI $5100`. Constants: `OVERCLOCK_LINES = 48`,
`WIDE_MAX_MARGIN = 112`, `WV_CODE = $5800`, `WV_CODE_PAGE = $2D`, `WV_CODE_BANK = $86`,
`WV_ENEMY_ON = $A5`.

Variables live in the extension's RAM (never in the game's RAM, which stays as the original uses
it):

| Name | Address | Meaning |
|---|---|---|
| `WV_M` | `$5200` | margin in use this frame (side view), pixels on each side |
| `WV_2M` | `$5201` | twice the margin |
| `WV_WL`, `WV_WR` | `$5202`, `$5203` | left and right extents of the side view's playfield for the next frame |
| `WV_ENEMY` | `$5204` | `$A5` while the enemy loop runs one enemy with a margin |
| `WV_T0`-`WV_T3` | `$5205`-`$5208` | scratch |
| `WV_VLO`, `WV_VHI` | `$520A`, `$520B` | range of level columns (0-63) drawn in the nametables, `[VLO, VHI)` mod 64 |
| `WV_TOWNHI` | `$520C` | high byte of the townsfolk spawn offset |
| `WV_DMASK` | `$520D` | display mask of the current enemy's columns |
| `WV_LEFTIDX` | `$520E` | scratch (world map attributes) |
| `WV_BLOBHI` | `$5210`-`$5217` | bit 8 of the x of world map encounters 0-7 |

Game RAM the hack reads (original meanings): `$0736` game mode (5 world map, `$0B` side view),
`$072A`/`$072C` side view scroll page/pixel, `$0732`/`$0733` and `$0734`/`$0735` left/right
streaming cursors (page, metatile column), `$0720` scroll direction bit, `$D1` last page of the
area, `$3C,x`/`$4E,x` enemy x high/low, `$B6,x` enemy slot active, `$91,x` OAM offset of an
enemy's sprites, `$42,x`/`$54,x`/`$87,x`/`$97,x` projectile x high/low, type, OAM offset, `$C9`
mask of an object's on-screen columns, `$0A` columns drawn, `$0E`/`$0F` column being tested;
world map: `$73`/`$74` Link's row/column, `$76`/`$77` map column/row being drawn, `$79`/`$7A` PPU
address of the update, `$7E` vertical step state, `$FD` horizontal scroll, `$0747` scroll flags,
`$0562` facing, `$0575,x` encounter speed, `$050E,x` encounter life, `$0410`/`$0470` decoded map
rows.

---

## 4. Boot, NMI, frame

* **Reset** (`$FF9A`, `JMP bank7_PowerON_code` becomes `JMP wide_reset`): `wide_reset` (bank 7,
  `$FF4C`) maps bank 0, calls `wide_boot`, maps bank 7 back (`LDA #$07 / JSR SwapPRG`) and jumps
  to `bank7_PowerON_code`. `wide_boot` writes "WIDE" to `$5000`, opens the PRG window
  (`$5008 = $86`, `$5009 = $2D`) and zeroes `WV_M`, `WV_2M`, `WV_WL`, `WV_WR`, `WV_ENEMY`.
* **NMI** (`$C0AF`, `LDX PPU_STATUS` becomes `JSR wide7_nmi`): `wide7_nmi` (bank 7, `$D39A`)
  checks `$5000 == $57`; if not, `wide7_unlock` (`$FEAA`) does what `wide_boot` does but the
  window. It then (re)opens the PRG window and jumps to `wr_nmi` (`$5800`), whose body:
  on the world map (`$0736 == 5`) asks for `min($5002, 112)` (`$5003`) and shows both sides
  whole (`$5004 = $5005 = $FF`); elsewhere hides both sides (`0`) for the top of the frame (the
  side view's status bar; the split opens them). It ends with `LDX $2002`, the instruction it
  replaces.
* **Side-view frame** (`$99E6`, `LDA $0740` of `L99E6`, the column streaming state machine, the
  last thing a side-view frame does, becomes `JMP wide_frame`; `wide_frame` ends with
  `LDA $0740 / JMP wide_l99e9`): when unlocked, sets the cartridge's mirroring (`$5007 = 0`), the
  margin `WV_M = min($5002, 112)` (also written to `$5003`), `WV_2M`; with a margin, asks for the
  overclock (`$5006 = 48`), works out the extents (`wide_extents`) and the sprites' high bytes
  (`wide_sprites`); without, clears the overclock and the extents.
* **Split** (`$D4C5`, the 9 bytes `STA $2000 / STX $2005 / STY $2005` that set the playfield's
  scroll under the status bar, become `JSR wr_split / JMP LD4CE / NOP x3`): does the three writes,
  then `$5004 = WV_WL`, `$5005 = WV_WR`. The emulator reads the extents per 8-line band, so the
  bar stays 256 wide and the playfield gets its margins.

---

## 5. Side view

### 5.1 Extents

The game streams level columns into the two nametables (a 32-metatile ring) between its left and
right cursors. The hack tracks which level columns are really drawn: the state machine's entries
2 and 6 ("second half of a column", `.word bank0_unknown26` at `$9A22` and `$9A2A`) point to
`wide_column_done` instead, which calls `bank0_unknown26` then records the column of the cursor of
the current direction (`$0720` bit 0 selects cursor 0 or 1; column = `page * 16 + column`, mod
64) into `[WV_VLO, WV_VHI)`: extending the range by one at either end, restarting it at a column
outside it, and keeping it at most 32 wide (the oldest is overwritten in the ring).

`wide_extents`, each frame:

* `WV_WL = min(WV_M, scroll - VLO * 16)`, the distance taken mod 1024 (`$FF` when farther than a
  page), and on the first page also at most the scroll itself (nothing left of the level);
* `WV_WR = min(WV_M, VHI * 16 - scroll - 256, level end - scroll - 256)`, `0` when the drawn end
  is left of the right edge, the level end being page `$D1 + 1`.

### 5.2 Enemies drawn in the margins

The game decides whether each 8-pixel column of an object is on screen in `bank7_code51`
(`$F294`): `LDA $0E / CMP $072C / LDA $0F / SBC $072A / BEQ LF2AF` (12 bytes) becomes
`JSR wr_coltest / BEQ LF2AF / BNE $F2A0 / NOP x5`. `wr_coltest` returns exactly the original Z
(so `$C8`-`$CA`, which the game's logic uses, are unchanged). In addition, while `WV_ENEMY == $A5`
and the object is an enemy (`$00 == 1`), it builds `WV_DMASK`: starting at the object's first
column (`Y == 3`), the bit `wr_column_bits[Y]` (`$08,$04,$02,$01`) of every column whose
`x + margin` is outside `[0, 256 + 2 margin)`.

`WV_ENEMY` is set around each enemy of the enemy loop: at `$D5CC`, `JSR LD5DA` becomes
`JSR wr_enemy`, which sets `$A5`, calls `LD5DA`, clears it (straight `JMP LD5DA` without a
margin). At `$D5D1`, `JSR LD56D` (projectiles) becomes `JSR wr_proj`, which is `JMP LD56D` (kept
as a hook point; projectiles are not widened).

`LF1D7` hides the off-screen columns of what was just drawn: at `$F1DB`, `LDA $0A / AND $C9`
(4 bytes) becomes `JSR wr_hidecol / NOP`, which uses `WV_DMASK` instead of `$C9` while
`WV_ENEMY == $A5`. So an enemy in the margins is drawn (its sprites keep their 8-bit OAM x) and
the game logic never knows.

### 5.3 Sprite high bytes

`wide_sprites`, each side-view frame with a margin: for each active enemy slot (`$B6,x != 0`,
x = 5..0) with screen x `E = ($3C,x:$4E,x) - ($072A:$072C)`, when E's high byte is `$FF` or `1`,
or 0 with the low byte below `$20` or at least `$E0` (objects near an edge), the six sprites at
`$0200 + $91,x` each get, at `$5100 + offset`, the high byte of `E + signed(L - E_low)` (`L` the
sprite's OAM x; sprites with OAM y at least `$F0` are skipped). The same for each projectile
(`$87,x != 0`, x from `$42,x:$54,x`, two sprites at `$97,x`). Sprites of objects wholly on screen
keep high byte 0 (the extension resets the page at each OAM DMA).

### 5.4 Spawns

* Enemies of the level list spawn when a streaming cursor reaches their column (`LD625`). At
  `$D625`, `LDA $0732,x / STA $00 / LDA $0734,x / STA $01` (10 bytes) becomes
  `JSR wr_spawncol / JMP $D62F / NOP x4`. `wr_spawncol` does the four original loads, then, with
  a margin over 72 and X = 0 or 1, unless called by the room loader (the `JSR` at `$D132`, which pushes `$D134`: checked as
  `$0104,x == $D1` and `$0103,x == $34` after `TSX`), pushes the compared column out by
  `(margin - 57) / 16` columns (left cursor further left, the page set to `$FF` before the level;
  right cursor further right). The cursors themselves are untouched (the streaming uses them).
* Townsfolk (bank 3, `$9722` and `$972A`): `ADC table_low,y` and `ADC table_high,y` become
  `JSR wr_town` and `JSR wr_town_hi`. The offsets from the scroll become `-32 - margin` (Y = 0)
  and `264 + margin` (Y = 1) instead of -32 and 264.

### 5.5 Enemies past the edges (the hack pack, `z2pack`)

The game keeps an enemy alive, its behaviour running and its slot taken (6 slots, `$B6,x`), up to
96 pixels past the edges of the screen, and removes it further out (`LDE6C`, after the common
display and collision code `LDE40`); off the screen it touches nothing. The margins show those
enemies as they are. The pack adds (all of it only with a margin):

* **How far they live.** At `$DE7A` and `$DE8B` the limits `LDA $072C / SEC / SBC #$60 / STA $01`
  and `LDA $072D / CLC / ADC #$60 / STA $03` become `JSR wr_live_left` and `JSR wr_live_right`:
  96 pixels as the game for a margin up to 72, `M + 48` above (the level list's enemies then
  spawn up to `M + 42` pixels out, 5.4).
* **Enemies made at an edge of the screen.** The generators (Mau heads and Tinsuits in the
  palaces, Mobys and Moblins outside) and the heads that come back (Ra and its like) put their
  enemy at the scroll plus a table's screen x (8 or 248, 0 or 239, 48 or 208...), which the
  margins would show in the middle of the picture. In the six places that do it (bank 1 `$9B66`,
  bank 4 `$ACB5`, `$B899`, `$B943`, `$B9FC`, bank 5 `$9643`), `LDA $072C / ADC table,y /
  STA $4E,x / LDA $072A / ADC #0 / STA $3C,x` becomes `LDA table,y / JSR wr_edge_spawn`: the
  game's place, then moved out by the margin 8 pixels at a time, stopping before a solid tile in
  the rows of Link's body (screen y `$29 - 32` and `$29 - 16`, the level starting below the
  32-pixel status bar; the tile read from the level at `$6000 + page * $D0`, its solidity the area
  bank's own test at `$850C`) or the end of the level: no enemy starts in a wall or behind one.
  The Bago Bagos (bank 7 `$D7AF`) are left alone: the game removes them as soon as they are off
  the screen.
* **Heads that come back.** Ra and its like (bank 4 `$ACDA`, `$BA20`, bank 5 `$9667`) are placed
  anew once their middle columns leave the screen (`LDA $C9 / AND #$0C`): that becomes
  `JSR wr_head_gone`, which tests the display mask of the wide view (`WV_DMASK`) instead, or a
  head made past the margin would be placed anew every frame.

The collision rule stays the game's: an enemy in the margins touches nothing until it is on the
original screen.

---

## 6. World map

The original draws the map in two vertically stacked nametables (horizontal mirroring), so the
horizontal ring is one screen (16 metatile columns) wide: nothing exists past the edges. The hack
turns on four nametables (`$5007 = 1`), making the horizontal ring 32 metatile columns (512
pixels), and keeps map columns `$74 - 16 .. $74 + 15` drawn (Link is at screen column 8): at
least 7 columns (112 pixels) past each edge.

* **Init** (`Initialization_stuff`): at `$8162`, `SBC #$08` becomes `SBC #$10` (`$76`, the first
  drawn column, is now `$74 - 16`); at `$817E`, the 15 bytes computing `$FD` and `$0747` from `$76`
  become `JSR wide_ow_init_scroll / JMP $818D / NOP x9`. `wide_ow_init_scroll`: `$0747 =
  (Y << 1) & 2` (the row group, as before) `| ((($74 - 8) >> 4) & 1)` (nametable x of the left
  edge), `$FD = ($74 - 8) << 4`, and `$5007 = 1`.
* **Columns drawn 16 further out**: the step code's offsets at `$861D` (`ADC #$07`), `$864E`,
  `$867F`, `$86A1` (`SBC #$07`) become `#$0F`.
* **9-bit scroll**: at `$86C6`, `DEC $FD / LSR / BCC L86CF / INC $FD / INC $FD` (9 bytes) becomes
  `JSR wide_ow_hscroll / JMP L86CF / NOP x3`. `wide_ow_hscroll` (A = facing & 3: 1 right, 2 left)
  moves `$FD` by one pixel and flips `$0747` bit 0 (nametable x) when it wraps.
* **Column update address**: `LDF01` (`$DF31`): `LDA $0A / AND #$10 / LSR / ORA $00 / STA $79`
  (9 bytes) becomes `JSR wr_ldf01 / JMP $DF3A / NOP x3`; `wr_ldf01` does the same then sets
  bit 2 of `$79` when bit 4 of `$76` is set (right nametable). The attribute address
  (`$8B78`: `LDA $77 / AND #$10 / LSR / ORA #$23`, 7 bytes) becomes `JSR wide_attr_hi / NOP x4`,
  adding the same nametable x bit (`($76 & $10) >> 2`).
* **Partner column alias** at `$8B2C`: the data `$0F,$F1` becomes `$1F,$E1` (the column offsets
  of the 32-wide ring), `AND #$F8` at `$89FC` becomes `AND #$FC`.
* **Rows 32 metatiles wide**: the decoded row buffer is `$0470`-`$04CF` (indexed by map column
  + 16); its edge fill at `$8BFE`-`$8C13` (`LDX #$3E / STA $048B,x` and `LDX #$14 / STA $0477,x`)
  becomes `LDX #$5F / STA $0470,x` and `LDX #$1B / STA $0470,x`. A vertical step draws its row in
  four 32-tile half-row packets and its attributes in two 8-byte packets, one per frame, so the
  per-frame VRAM load stays the original's: at `$88B3`, `LDA a:$7E` (dispatch of the vertical
  step's states) becomes `JMP wide_vdispatch`, an 8-state table: decode (`L8BFE`), top-left,
  top-right, bottom-left, bottom-right half rows (`wide_row_half` + `wide_row_next` after the
  right halves), the attribute partner row (`overworld6`), the attributes
  (`wide_row_attributes`), done (`LDFD1`). The step count checks follow: `CMP #$06` at `$884E`
  and `LDA #$06` at `$8A2C` become `#$07`.
  * `wide_row_half(A = tile row 0/1, X = 0/16)`: PPU address from `$79`/`$7A` (bit 2 of `$79` for
    the nametable), `$20` tiles, for each of 16 ring slots the two tiles of the metatile of
    buffer index `wide_slot_index(slot)` (map column `$76 + ((slot - $76) & 31)`, + 16; water
    `$0C` past `$60`) from `Overworld_Tile_Mappings_and_Overworld_Palette_Codes`, terminator
    `$FF` at `$0325`.
  * `wide_row_next`: next tile row of the nametable (`$7A += $20`), wrapping from row 30 of one
    stacked nametable to the other (`$79 ^ 8`).
  * `wide_row_attributes`: two packets of 8 attribute bytes (left, right nametable) from the top
    row `$0410` and bottom row `$0470` via `bank0_Overworld_Palette_Codes_0_3`, nibbles swapped
    when moving down except on a nametable's last row, as the original.
* **Black strip**: the original hides x 248-255 of the map behind 15 black sprites (`L877C`); in
  a wide picture they would sit in the middle. At `$877C`, `LDX #$0E / LDY #$00` becomes
  `JMP wide_strip / NOP` (label `wide_l8780` after it): with a margin the 15 sprites at `$0208`
  get OAM y `$F8`; without, the original runs.

### 6.1 Encounters

World map encounters (slots 0-7) get a 9-bit x: `$4E,x` plus `WV_BLOBHI,x`, in the 512-pixel
ring; the scroll is `$FD` plus `$0747` bit 0. They live on screen x `-M .. 247 + M`.

* Spawn (`$8310`): `CLC / ADC $FD / STA $4E,x` becomes `JSR wide_blob_spawn / NOP / NOP`: an
  encounter left of Link (table x below `$80`) starts M further left, right of Link M further
  right, above or below stays; `WV_T3 = M / 16` extra life. Life (`$831A`): `STA $050E,x` becomes
  `JSR wide_blob_life` (adds `WV_T3`).
  In the hack pack (`z2pack`) neither: an encounter spawns where the game puts it (32 or 64
  pixels beside Link), with the game's life (`$831A` left alone); pushing it out made it much
  easier to avoid.
* Move and place (`$8359`, 20 bytes) become `JSR wide_blob_x / JMP $836D / NOP x14`: 9-bit move
  by `$0575,x`, screen x via `wide_blob_screen` (-152..359), OAM x of both halves (`$0283,y`, `$0287,y`) and their high
  bytes (`$5180 + y`, `$5184 + y`); past the margins, OAM y `$F8`.
* Off-screen removal (`$8404`): `LDA $0283,y / CMP #$F8 / BCC L8412` becomes `JMP L8412 / NOP x4`
  (`wide_blob_x` already hides them).
* Chase steering (`$8451`): `LDA $4E,x / SEC / SBC $FD / CMP $0203 / BCC L845D` becomes
  `JSR wide_blob_cmp / BCC L845D / BCS $845B / NOP x3`, comparing the 9-bit screen x with Link's.
* `wide_ow_margin`: the margin (`min($5002, 112)`), 0 when the extension is locked.

---

## 7. Building

Requirements: cc65 (ca65, ld65), a POSIX shell, Python 3, the source ROM.

1. Clone `https://github.com/FiendsOfTheElements/z2disassembly`, check out `c4c3a4c`.
2. Apply appendix D (the difference of `nes.cfg` and `src/prg0.asm`, `prg3.asm`, `prg6.asm`,
   `prg7.asm`, the 1.1 fix included); add `inc/wide.inc` (appendix B) and `inc/wide_bank0.asm`
   (appendix C), and `build-wide.sh` (appendix F). Line endings do not matter to ca65.
3. `./build-wide.sh "Zelda II - The Adventure of Link (USA).nes"`: assembles the eight banks,
   links them with `nes.cfg`, concatenates the ROM's iNES header, the eight PRG banks and the
   ROM's CHR (from file offset 131 088) into `build/z2wide.nes`.
4. Check: CRC-32 of `build/z2wide.nes` without its 16-byte header is `ADCEA53F`; appendix E lists
   every difference with the source ROM. (Checked: a clean checkout of `c4c3a4c` with appendices
   B, C, D and F extracted from this document, the diff applied with `git apply
   --ignore-whitespace`, builds exactly that.)
5. Patch: `python -I tools/make_bps.py <source rom> build/z2wide.nes zelda2-usa-widescreen.bps`
   (appendix G): a BPS patch of the data without headers, made only of SourceRead (unchanged
   runs) and TargetRead (changed bytes), so it carries no game data.
6. In Zaxnes, the patch goes to `rust/crates/zax-nes/patches/` (`include_bytes!`); its entry in
   `builtin_patches::PATCHES` names the source CRC (`0xBA32_2865`).

Assembly options used during development (none in the shipped build): `CAFLAGS=-DNO_EXTENTS`,
`-DNO_SPRITES`, `-DNO_FLAG`, `-DEXTENT_LOG` disable or log parts of the side view.

---

## 8. Emulator-side companions (Zaxnes)

These are not part of the ROM, but the hack is designed with them:

* **Margin decoder** (`zax-nes/src/wide_decode/zelda2.rs`, adapted from z2rs,
  <https://github.com/troyedwardsjr/z2rs>, MIT): fills the margins from the game's own data,
  wider than the hack's 112 pixels:
  * side view (`$0736 == $0B`, `$0524 == 0`, nametable rows 4-29): the area is 4 pages of 13 rows
    of 16 metatiles at PRG-RAM `$6000 + page * $D0 + row * 16 + column`; a metatile code gives four
    CHR tiles through the pointer table at `$8500` of the area's PRG bank (`$0769`), group
    `code >> 6` (also the sub-palette), entry `(code & $3F) * 4`, column first (TL, BL, TR, BR);
    outside the area, metatile `$42` when `$010C == 0`, else `$40`. Level row r covers nametable
    rows 4 + 2r and 5 + 2r. World tile column of screen column 0 = `resolve_ring(ring_x, 512,
    $072A:$072C) >> 3`;
  * world map (`$0736 == 5`, Link placed: `$73 - $1E` in 0..75): one terrain nibble per 16 x 16
    cell, run-length coded per row (`(length - 1) << 4 | terrain`), row starts from the game's
    table at PRG-RAM `$6000` (75 little-endian addresses into `$7000-$7FFF`) or rebuilt from the
    copy at `$7C00` (a row ends at the run that reaches the 65th column); left of the map:
    mountains in regions (`$0706`) 0-1, water otherwise; right of it or outside the rows: water.
    Tiles and sub-palettes per terrain: see `TERRAIN_TILES` / `TERRAIN_PALETTES`. Left column:
    `resolve_ring(ring_x mod 256, 256, 16 * ($74 - 8) -/+ $7D)` (`$0562` 1 right, 2 left), row:
    `resolve_ring((nt_y * 30 + coarse_y) / 2, 30, $73 - $1E - 7 + line / 16)`. The world map also
    fills the vertical margin.
* **A state made the other way** (`after_other_way_state`): a world map state (`$0736 == 5`)
  restored with the hack set the other way gets `$0736 = 2` (the game's map initialisation),
  which redraws the map around Link in the layout the running code expects (two or four
  nametables). Together with `wide7_unlock` and the core resetting the extension for a hack state
  restored without the hack, states load both ways.

---

## Appendices

The appendices are generated from the hack's repository (`tools/make_doc.py`) and are therefore
exactly what builds the target.

### A. `nes.cfg`

```
MEMORY
{
	MEM0: start = $8000, size = $4000, file = "prg0.bin", fill = yes, fillval = $00;
	MEM1: start = $8000, size = $4000, file = "prg1.bin", fill = yes, fillval = $00;
	MEM2: start = $8000, size = $4000, file = "prg2.bin", fill = yes, fillval = $00;
	MEM3: start = $8000, size = $4000, file = "prg3.bin", fill = yes, fillval = $00;
	MEM4: start = $8000, size = $4000, file = "prg4.bin", fill = yes, fillval = $00;
	MEM5: start = $8000, size = $4000, file = "prg5.bin", fill = yes, fillval = $00;
	MEM6: start = $8000, size = $4000, file = "prg6.bin", fill = yes, fillval = $00;
	MEM7: start = $C000, size = $4000, file = "prg7.bin", fill = yes, fillval = $00;
}

SEGMENTS
{
	PRG0: load = MEM0, type = ro, align = $4000;
	WIDE0: load = MEM0, type = ro, start = $AA40;
	PRG0B: load = MEM0, type = ro, start = $B082;
	PRG1: load = MEM1, type = ro, align = $4000;
	PRG2: load = MEM2, type = ro, align = $4000;
	PRG3: load = MEM3, type = ro, align = $4000;
	PRG4: load = MEM4, type = ro, align = $4000;
	PRG5: load = MEM5, type = ro, align = $4000;
	PRG6: load = MEM6, type = ro, align = $4000;
	WIDE6W: load = MEM6, type = ro, start = $AD00;
	PRG6B: load = MEM6, type = ro, start = $B082;
	PRG7: load = MEM7, type = ro, align = $4000;
	WIDE7A: load = MEM7, type = ro, start = $D39A;
	PRG7B: load = MEM7, type = ro, start = $D3CA;
	WIDE7B: load = MEM7, type = ro, start = $FEAA;
	PRG7C: load = MEM7, type = ro, start = $FED0;
	WIDE7C: load = MEM7, type = ro, start = $FF4C;
	PRG7D: load = MEM7, type = ro, start = $FF70;
	VECTORS: load = MEM7, type = ro, start = $FFFA;
}
```

### B. `inc/wide.inc`

```asm
; Widescreen hack: registers of the emulator's widescreen extension and the variables the hack
; keeps in the extension's RAM. Protocol: rust/docs/WIDESCREEN-EXTENSION.md of Zaxnes.

WIDE_SIG        = $5000         ; W: unlock sequence "WIDE"; R: $57 once unlocked
WIDE_HOST       = $5002         ; R: margin the host offers on each side, in pixels
WIDE_REQ        = $5003         ; W: margin the game asks for on each side
WIDE_LEFT       = $5004         ; W: how far left of x = 0 the picture may show
WIDE_RIGHT      = $5005         ; W: how far right of x = 255 the picture may show
WIDE_LAYOUT     = $5007         ; W: bit 0 four nametables
WIDE_OVERCLOCK  = $5006         ; W: extra CPU time per frame, in scanlines
.ifndef OVERCLOCK_LINES
OVERCLOCK_LINES = 48
.endif
WIDE_MAX_MARGIN = 112           ; widest margin the hack keeps drawn
WIDE_SPRHI      = $5100         ; W: high byte of the x of sprite n at $5100 + 4n

; Variables, in the extension's RAM ($5200-$5FFF).
WV_M            = $5200         ; margin in use, pixels on each side
WV_2M           = $5201         ; twice the margin
WV_WL           = $5202         ; left extent of the next frame
WV_WR           = $5203         ; right extent of the next frame
WV_ENEMY        = $5204         ; $A5 while the enemy loop places an enemy
WV_T0           = $5205         ; scratch
WV_T1           = $5206
WV_T2           = $5207
WV_T3           = $5208
WV_VLO          = $520A         ; first level column drawn in the nametables (0-63)
WV_VHI          = $520B         ; one past the last one
WV_TOWNHI       = $520C         ; high byte of the townsfolk spawn offset
WV_LEFTIDX      = $520E         ; scratch of the world map attributes
WV_BLOBHI       = $5210         ; 8 bytes: bit 8 of the world map encounters' x
WV_DMASK        = $520D         ; columns of the current enemy hidden for display (margins accepted)

; Code copied from bank 0 at power-on: it runs whatever bank is mapped at $8000.
WIDE_WINDOW_BANK = $5008
WIDE_WINDOW_PAGE = $5009
WV_CODE         = $5800
WV_CODE_PAGE    = $2D           ; ($AD00 - $8000) / 256
WV_CODE_BANK    = $86           ; window on, bank 6
WV_ENEMY_ON     = $A5
```

### C. `inc/wide_bank0.asm`

Included by `src/prg0.asm` in segment `WIDE0`; its second part switches to segment `WIDE6W` (bank 6, run at `$5800`).

```asm
; Widescreen hack, bank 0 part (segment WIDE0, the unused space at $AA40).
;
; Side view only for now:
;  * the extents: the game already keeps a window of 27 metatile columns drawn in the two
;    nametables, between its left and right streaming cursors ($0732/$0734 and $0733/$0735).
;    Each frame the hack works out how much of it lies past each side of the screen, clamps it
;    to the level and to the margin, and has the split that sets the scroll pass it on;
;  * enemies: the column test of bank7_code51 accepts the margins while the enemy loop runs,
;    and every sprite an enemy placed gets the high byte of its x before the next OAM DMA.

.include "wide.inc"

.export wide_boot
.export wide_column_done
.export wr_nmi
.export wr_split
.export wr_enemy
.export wr_coltest
.export wr_town
.export wr_town_hi
.export wr_proj
.export wr_hidecol
.export wr_spawncol
.export wr_ldf01
.export wide_ow_init_scroll
.export wide_ow_hscroll
.export wide_attr_hi
.export wide_vdispatch
.export wide_strip

; ---------------------------------------------------------------------------- ;
; Power-on (bank 0 mapped, called from the reset trampoline): unlock the extension and open its
; PRG window on the code the fixed-bank hooks call. The hack needs the extension.
wide_boot:
    LDA      #$57
    STA      WIDE_SIG
    LDA      #$49
    STA      WIDE_SIG
    LDA      #$44
    STA      WIDE_SIG
    LDA      #$45
    STA      WIDE_SIG
    LDA      #WV_CODE_BANK             ; PRG window: bank 6 from $AD00 at $5800
    STA      WIDE_WINDOW_BANK
    LDA      #WV_CODE_PAGE
    STA      WIDE_WINDOW_PAGE
    LDA      #$00
    STA      WV_M
    STA      WV_2M
    STA      WV_WL
    STA      WV_WR
    STA      WV_ENEMY
    RTS

; ---------------------------------------------------------------------------- ;
; Side-view frame hook, entered from L99E6 (the column streaming state machine, the last
; thing a side-view frame does) in place of its first instruction.
wide_frame:
    LDA      WIDE_SIG
    CMP      #$57
    BNE      @done
    LDA      #$00                      ; side view: the cartridge's mirroring
    STA      WIDE_LAYOUT
    JSR      wide_host
    STA      WV_M
    STA      WIDE_REQ
    ASL      A
    STA      WV_2M
    BEQ      @off
    LDA      #OVERCLOCK_LINES          ; the enemies of the margins cost CPU time
    STA      WIDE_OVERCLOCK
.ifndef NO_EXTENTS
    JSR      wide_extents
.endif
.ifndef NO_SPRITES
    JSR      wide_sprites
.endif
    JMP      @done
@off:
    STA      WIDE_OVERCLOCK
    STA      WV_WL
    STA      WV_WR
@done:
    LDA      $0740
    JMP      wide_l99e9

; ---------------------------------------------------------------------------- ;
; The margin the host offers, capped at what the hack keeps drawn: the side view's 32 streamed
; columns and the world map's ring leave at least 112 pixels on each side (and twice the margin
; must fit in a byte).
wide_host:
    LDA      WIDE_HOST
    CMP      #WIDE_MAX_MARGIN
    BCC      @rts
    LDA      #WIDE_MAX_MARGIN
@rts:
    RTS

;  WV_WL = min(margin, scroll - first drawn column, scroll)
;  WV_WR = min(margin, end of the drawn columns - scroll - 256, level end - scroll - 256)
; The drawn columns are tracked by wide_column_done; distances are taken mod 1024 (the level is
; at most 4 pages) and anything past 255 is capped.
wide_extents:
    LDA      WV_VLO
    JSR      wide_column_x             ; WV_T2:WV_T3 = column * 16
    LDA      $072C                     ; scroll - first column
    SEC
    SBC      WV_T2
    STA      WV_T0
    LDA      $072A
    SBC      WV_T3
    AND      #$03
    BEQ      @left_near
    LDA      #$FF
    STA      WV_T0
@left_near:
    LDA      $072A                     ; first page: nothing left of the level
    BNE      @left_level
    LDA      $072C
    CMP      WV_T0
    BCS      @left_level
    STA      WV_T0
@left_level:
    LDA      WV_T0
    CMP      WV_M
    BCC      @left_set
    LDA      WV_M
@left_set:
    STA      WV_WL

    LDA      WV_VHI
    JSR      wide_column_x
    LDA      WV_T2                     ; end column - scroll - 256
    SEC
    SBC      $072C
    STA      WV_T0
    LDA      WV_T3
    SBC      $072A
    SEC
    SBC      #$01
    AND      #$03
    BEQ      @right_near
    CMP      #$03                      ; drawn end left of the screen's right edge: nothing
    BNE      @right_far
    LDA      #$00
    STA      WV_T0
    BEQ      @right_near
@right_far:
    LDA      #$FF
    STA      WV_T0
@right_near:
    LDA      #$00                      ; level end: ($D1 + 1) pages
    SEC
    SBC      $072C
    STA      WV_T1
    LDA      $D1
    SBC      $072A
    BMI      @right_none
    BEQ      @right_level
    LDA      #$FF
    STA      WV_T1
    BNE      @right_level
@right_none:
    LDA      #$00
    STA      WV_T1
@right_level:
    LDA      WV_T0
    CMP      WV_T1
    BCC      @right_min
    LDA      WV_T1
@right_min:
    CMP      WV_M
    BCC      @right_set
    LDA      WV_M
@right_set:
    STA      WV_WR
.ifdef EXTENT_LOG
    LDX      $5800                     ; debug: log WL/WR per frame at $5801
    LDA      WV_WL
    STA      $5801,x
    LDA      WV_WR
    STA      $5901,x
    INC      $5800
.endif
    RTS

; WV_T2 (low) and WV_T3 (page) = A * 16, A a level column 0-63.
wide_column_x:
    PHA
    ASL      A
    ASL      A
    ASL      A
    ASL      A
    STA      WV_T2
    PLA
    LSR      A
    LSR      A
    LSR      A
    LSR      A
    STA      WV_T3
    RTS

; ---------------------------------------------------------------------------- ;
; Second half of a column written (state machine entries 6 and 2, in place of bank0_unknown26):
; the column of the cursor of the current direction is now in the nametables. Keep the range
; of level columns drawn there, at most 32 (the two nametables hold 32 metatile columns).
wide_column_done:
    JSR      bank0_unknown26
    LDA      WIDE_SIG
    CMP      #$57
    BNE      @rts
    LDA      $0720
    AND      #$01
    TAY
    LDA      $0732,y                   ; column = page * 16 + column in page
    ASL      A
    ASL      A
    ASL      A
    ASL      A
    ORA      $0734,y
    AND      #$3F
    STA      WV_T0
    CMP      WV_VHI
    BEQ      @grow_right
    CLC
    ADC      #$01
    AND      #$3F
    CMP      WV_VLO
    BEQ      @grow_left
    LDA      WV_T0                     ; inside the range: nothing changes
    SEC
    SBC      WV_VLO
    AND      #$3F
    STA      WV_T1
    LDA      WV_VHI
    SEC
    SBC      WV_VLO
    AND      #$3F
    CMP      WV_T1
    BEQ      @restart
    BCS      @rts
@restart:                              ; somewhere else: a new range
    LDA      WV_T0
    STA      WV_VLO
    CLC
    ADC      #$01
    AND      #$3F
    STA      WV_VHI
@rts:
    RTS
@grow_right:
    CLC
    ADC      #$01
    AND      #$3F
    STA      WV_VHI
    SEC
    SBC      WV_VLO
    AND      #$3F
    CMP      #33
    BCC      @rts
    LDA      WV_VHI                    ; 32 at most: the oldest column was overwritten
    SEC
    SBC      #32
    AND      #$3F
    STA      WV_VLO
    RTS
@grow_left:
    LDA      WV_T0
    STA      WV_VLO
    LDA      WV_VHI
    SEC
    SBC      WV_VLO
    AND      #$3F
    CMP      #33
    BCC      @rts
    LDA      WV_VLO
    CLC
    ADC      #32
    AND      #$3F
    STA      WV_VHI
    RTS

; ---------------------------------------------------------------------------- ;
; High bytes of the x of the sprites of every enemy slot: an enemy at screen x (16-bit) E
; whose draw routine put a sprite at OAM x L made it E + (L - (E & $FF)) as a signed byte.
; Each slot owns the six sprites at $0200 + $91,x.
wide_sprites:
    LDX      #$05
@slot:
    LDA      $B6,x
    BEQ      @next
    LDA      $4E,x
    SEC
    SBC      $072C
    STA      WV_T0                     ; E low
    LDA      $3C,x
    SBC      $072A
    STA      WV_T1                     ; E high
    BEQ      @near                     ; on the screen: only a right edge can spill over
    CMP      #$01
    BEQ      @scan
    CMP      #$FF
    BNE      @next                     ; too far for any margin
    BEQ      @scan
@near:                                 ; on the screen: high bytes stay 0 unless sprites, up to 32 px from the origin, cross an edge
    LDA      WV_T0
    CMP      #$20
    BCC      @scan
    CMP      #$E0
    BCC      @next
@scan:
    LDY      $91,x
    LDA      #$06
    JSR      wide_sprite_run
@next:
    DEX
    BPL      @slot

    LDX      #$05                      ; the projectile of each slot: two sprites at $97,x
@proj:
    LDA      $87,x                     ; projectile type
    BEQ      @proj_next
    LDA      $54,x
    SEC
    SBC      $072C
    STA      WV_T0
    LDA      $42,x
    SBC      $072A
    STA      WV_T1
    BEQ      @proj_near
    CMP      #$01
    BEQ      @proj_scan
    CMP      #$FF
    BNE      @proj_next
    BEQ      @proj_scan
@proj_near:
    LDA      WV_T0
    CMP      #$20
    BCC      @proj_scan
    CMP      #$E0
    BCC      @proj_next
@proj_scan:
    LDY      $97,x
    LDA      #$02
    JSR      wide_sprite_run
@proj_next:
    DEX
    BPL      @proj
    RTS

; High bytes of A sprites from $0200,y on, owned by an object at screen x WV_T1:WV_T0.
wide_sprite_run:
    STA      WV_T2
@sprite:
    LDA      $0200,y
    CMP      #$F0
    BCS      @skip
    LDA      $0203,y
    SEC
    SBC      WV_T0
    STA      WV_T3
    LDA      WV_T0
    CLC
    ADC      WV_T3
    LDA      WV_T1
    ADC      #$00
    BIT      WV_T3
    BPL      @store
    CLC
    ADC      #$FF
@store:
    STA      WIDE_SPRHI,y
@skip:
    INY
    INY
    INY
    INY
    DEC      WV_T2
    BNE      @sprite
    RTS

; ============================================================================ ;
; World map. With four nametables (the extension's layout bit) the horizontal ring is 32
; metatile columns, and the renderer keeps map columns $74-16 .. $74+15 drawn: at least 7
; columns (112 px) past each side of the screen. Columns are drawn 16 further out than the
; original (the step code), rows are drawn 32 metatiles wide in four half-row packets, and the
; attribute rows in two packets, so the vblank load per frame stays the original's.

; Overworld init (Initialization_stuff), in place of the $FD / $0747 setup from $76: $76 is now
; $74-16 (the first drawn column) while the screen still starts at $74-8. Y = the nametable row
; group from LDF3F, as before. Turns the four nametables on.
wide_ow_init_scroll:
    TYA
    ASL      A
    AND      #$02
    STA      $0747
    LDA      $74
    SEC
    SBC      #$08
    PHA
    ASL      A
    ASL      A
    ASL      A
    ASL      A
    STA      $FD
    PLA
    LSR      A
    LSR      A
    LSR      A
    LSR      A
    AND      #$01                      ; nametable x of the screen's left edge
    ORA      $0747
    STA      $0747
    LDA      #$01
    STA      WIDE_LAYOUT
    RTS

; One frame of a horizontal step (L86AF), in place of DEC $FD / LSR / BCC / INC $FD / INC $FD:
; A = facing & 3 (1 right, 2 left). The scroll is 9 bits: the nametable x bit of $0747 flips
; when $FD wraps.
wide_ow_hscroll:
    CMP      #$01
    BEQ      @right
    DEC      $FD
    LDA      $FD
    CMP      #$FF
    BNE      @rts
    BEQ      @flip
@right:
    INC      $FD
    BNE      @rts
@flip:
    LDA      $0747
    EOR      #$01
    STA      $0747
@rts:
    RTS

; Attribute byte address of a column (L8B76), in place of LDA $77 / AND #$10 / LSR / ORA #$23:
; the nametable x bit comes from bit 4 of the map column.
wide_attr_hi:
    LDA      $76
    AND      #$10
    LSR      A
    LSR      A
    STA      WV_T0
    LDA      $77
    AND      #$10
    LSR      A
    ORA      #$23
    ORA      WV_T0
    RTS

; The vertical step's states (L88B3), one per frame.
wide_vdispatch:
    LDA      $7E
    JSR      bank7_PullAddrFromTableFollowingThisJSR_withIndexOfA_then_JMP
.word    L8BFE                         ; decode the row
.word    wide_row_top_left
.word    wide_row_top_right
.word    wide_row_bottom_left
.word    wide_row_bottom_right
.word    overworld6                    ; the attribute partner row (or straight to the end)
.word    wide_row_attributes
.word    LDFD1                         ; done

wide_row_top_left:
    LDA      #$00
    LDX      #$00
    JMP      wide_row_half
wide_row_top_right:
    LDA      #$00
    LDX      #$10
    JSR      wide_row_half
    JMP      wide_row_next
wide_row_bottom_left:
    LDA      #$01
    LDX      #$00
    JMP      wide_row_half
wide_row_bottom_right:
    LDA      #$01
    LDX      #$10
    JSR      wide_row_half
    JMP      wide_row_next

; Y = row buffer index of ring slot A (0-31): map column $76 + ((slot - $76) & 31), plus 16.
wide_slot_index:
    SEC
    SBC      $76
    AND      #$1F
    CLC
    ADC      $76
    CLC
    ADC      #$10
    TAY
    RTS

; One 32-tile packet: tile row A (0 top: TL TR, 1 bottom: BL BR) of the 16 ring slots from X
; (0: left nametable, 16: right), into $0302.
wide_row_half:
    STA      WV_T0
    STX      WV_T1
    LDA      $79
    AND      #$FB
    CPX      #$10
    BCC      @left
    ORA      #$04
@left:
    STA      $0302
    LDA      $7A
    AND      #$E0
    STA      $0303
    LDA      #$20
    STA      $0304
    LDA      #$00
    STA      WV_T2
@slot:
    LDA      WV_T1
    JSR      wide_slot_index
    LDA      #$0C                      ; water past the buffer
    CPY      #$60
    BCS      @code
    LDA      $0470,y
@code:
    ASL      A
    ASL      A
    CLC
    ADC      WV_T0
    TAX
    LDY      WV_T2
    LDA      Overworld_Tile_Mappings_and_Overworld_Palette_Codes,x
    STA      $0305,y
    LDA      Overworld_Tile_Mappings_and_Overworld_Palette_Codes+2,x
    STA      $0306,y
    INY
    INY
    STY      WV_T2
    INC      WV_T1
    LDA      WV_T1
    AND      #$0F
    BNE      @slot
    LDA      #$FF
    STA      $0325
    INC      $7E
    RTS

; Next tile row of the nametable (the tail of the original L8C57), the x bit kept.
wide_row_next:
    LDA      $7A
    CLC
    ADC      #$20
    STA      $7A
    LDA      $79
    ADC      #$00
    STA      $79
    AND      #$03
    CMP      #$03
    BNE      @rts
    LDA      $7A
    AND      #$F0
    CMP      #$C0
    BCC      @rts
    LDA      $7A
    AND      #$1F
    STA      $7A
    LDA      $79
    EOR      #$08
    AND      #$FC
    STA      $79
@rts:
    RTS

; The attribute row (L8A79 for 32 metatiles): two 8-byte packets, left then right nametable.
; Top row $0410, bottom row $0470 (raw, indexed by map column + 16); moving down the halves
; are swapped, as the original does, except on the last row of a nametable.
wide_row_attributes:
    LDA      #$00
    STA      WV_T1                     ; first slot
    LDX      #$00
    JSR      wide_attribute_packet
    LDA      #$10
    STA      WV_T1
    LDX      #$0B
    JSR      wide_attribute_packet
    LDA      #$FF
    STA      $0318
    INC      $7E
    RTS

; One packet at $0302,x for the 8 slot pairs from WV_T1.
wide_attribute_packet:
    LDA      $77
    AND      #$10
    LSR      A
    ORA      #$23
    LDY      WV_T1
    BEQ      @left
    ORA      #$04
@left:
    STA      $0302,x
    LDA      $77
    AND      #$0E
    ASL      A
    ASL      A
    ORA      #$C0
    STA      $0303,x
    LDA      #$08
    STA      $0304,x
    STX      WV_T3                     ; where the next byte goes, minus 3
    LDA      #$08
    STA      WV_T2                     ; bytes left
@byte:
    LDA      WV_T1                     ; buffer indexes of the pair
    JSR      wide_slot_index
    STY      WV_LEFTIDX
    LDA      WV_T1
    CLC
    ADC      #$01
    JSR      wide_slot_index           ; Y: right slot
    LDX      $0470,y                   ; BR (bits 6-7)
    LDA      bank0_Overworld_Palette_Codes_0_3,x
    ASL      A
    ASL      A
    STA      WV_T0
    LDY      WV_LEFTIDX
    LDX      $0470,y                   ; BL (bits 4-5)
    LDA      bank0_Overworld_Palette_Codes_0_3,x
    ORA      WV_T0
    ASL      A
    ASL      A
    STA      WV_T0
    LDA      WV_T1
    CLC
    ADC      #$01
    JSR      wide_slot_index
    LDX      $0410,y                   ; TR (bits 2-3)
    LDA      bank0_Overworld_Palette_Codes_0_3,x
    ORA      WV_T0
    ASL      A
    ASL      A
    STA      WV_T0
    LDY      WV_LEFTIDX
    LDX      $0410,y                   ; TL (bits 0-1)
    LDA      bank0_Overworld_Palette_Codes_0_3,x
    ORA      WV_T0
    STA      WV_T0
    LDA      $0562                     ; moving down, not on a nametable's last row: swap
    AND      #$04
    BEQ      @store
    LDA      $77
    AND      #$0F
    CMP      #$0E
    BEQ      @store
    LDA      WV_T0
    ASL      A
    ASL      A
    ASL      A
    ASL      A
    STA      WV_LEFTIDX
    LDA      WV_T0
    LSR      A
    LSR      A
    LSR      A
    LSR      A
    ORA      WV_LEFTIDX
    STA      WV_T0
@store:
    LDX      WV_T3
    LDA      WV_T0
    STA      $0305,x
    INX
    STX      WV_T3
    INC      WV_T1
    INC      WV_T1
    DEC      WV_T2
    BEQ      @done
    JMP      @byte
@done:
    RTS

; ============================================================================ ;
; World map encounters ("blobs", slots 0-7). Their x is 9 bits: $4E,x plus WV_BLOBHI,x, in the
; 512-pixel ring of the four nametables; the scroll is $FD plus bit 0 of $0747. They live on
; screen x -M .. 247+M (the original's 0 .. 247 with no margin) and those spawned beside Link
; start M pixels further out, with M/16 more life ticks to come back.

; Margin of the world map: what the host offers (0 without the extension).
wide_ow_margin:
    LDA      WIDE_SIG
    CMP      #$57
    BNE      @none
    JMP      wide_host
@none:
    LDA      #$00
    RTS

; Screen x of blob X: A = low byte, WV_T1 = high byte ($FF, 0 or 1), from -152 to 359.
wide_blob_screen:
    LDA      $0747
    AND      #$01
    STA      WV_T1
    LDA      $4E,x
    SEC
    SBC      $FD
    STA      WV_T0
    LDA      WV_BLOBHI,x
    SBC      WV_T1
    AND      #$01
    STA      WV_T1
    BEQ      @done
    LDA      WV_T0
    CMP      #$68
    BCC      @done
    LDA      #$FF
    STA      WV_T1
@done:
    LDA      WV_T0
    RTS

; Spawn (overworld1), in place of CLC / ADC $FD / STA $4E,x: A = the table's screen x.
wide_blob_spawn:
    STA      WV_T0
    LDA      #$00
    STA      WV_T1
    STA      WV_T3                     ; pushed out: the life store adds ticks
    JSR      wide_ow_margin
    STA      WV_T2
    BEQ      @place
    LDA      WV_T0
    CMP      #$80
    BEQ      @place                    ; above or below Link: stays
    BCS      @right
    SEC                                ; left of Link: further left
    SBC      WV_T2
    STA      WV_T0
    LDA      #$00
    SBC      #$00
    STA      WV_T1
    JMP      @pushed
@right:
    CLC
    ADC      WV_T2
    STA      WV_T0
    LDA      #$00
    ADC      #$00
    STA      WV_T1
@pushed:
    LDA      WV_T2
    LSR      A
    LSR      A
    LSR      A
    LSR      A
    STA      WV_T3
@place:
    LDA      $FD                       ; world x = scroll + screen x, 9 bits
    CLC
    ADC      WV_T0
    STA      $4E,x
    LDA      $0747
    ADC      WV_T1
    AND      #$01
    STA      WV_BLOBHI,x
    RTS

; Life timer of a new blob, in place of STA $050E,x.
wide_blob_life:
    CLC
    ADC      WV_T3
    STA      $050E,x
    RTS

; Move and place blob X (L8342), in place of the 8-bit x move and the OAM x of its two halves
; (Y = its OAM offset from $0280). A blob past the margins gets OAM Y $F8, which the original
; then removes.
wide_blob_x:
    LDA      $4E,x
    CLC
    ADC      $0575,x
    STA      $4E,x
    LDA      $0575,x
    BMI      @left
    BCC      @moved
    BCS      @flip
@left:
    BCS      @moved
@flip:
    LDA      WV_BLOBHI,x
    EOR      #$01
    STA      WV_BLOBHI,x
@moved:
    JSR      wide_blob_screen
    STA      $0283,y
    CLC
    ADC      #$08
    STA      $0287,y
    LDA      WV_T1
    STA      WIDE_SPRHI+$80,y
    ADC      #$00
    STA      WIDE_SPRHI+$84,y
    JSR      wide_ow_margin            ; x + margin in [0, 248 + 2 margin)?
    STA      WV_T2
    LDA      WV_T0
    CLC
    ADC      WV_T2
    STA      WV_T3
    LDA      WV_T1
    ADC      #$00
    BMI      @hide
    BEQ      @low
    CMP      #$01
    BNE      @hide
    LDA      WV_T2                     ; 256 <= x + margin: below 248 + 2 margin - 256?
    ASL      A
    SEC
    SBC      #$08
    BCC      @hide                     ; 2 margin < 8: nothing past 256
    CMP      WV_T3
    BEQ      @hide
    BCS      @rts
    BCC      @hide
@low:
    LDA      WV_T2
    CMP      #$04
    BCS      @rts                      ; 248 + 2 margin >= 256
    LDA      WV_T3
    CMP      #$F8
    BCC      @rts
@hide:
    LDA      #$F8
    STA      $0280,y
    STA      $0284,y
@rts:
    RTS

; Chase steering (L8451), in place of LDA $4E,x / SEC / SBC $FD / CMP $0203: carry set when the
; blob's screen x is not left of Link's sprite.
wide_blob_cmp:
    JSR      wide_blob_screen
    LDA      WV_T1
    BMI      @left
    BNE      @right
    LDA      WV_T0
    CMP      $0203
    RTS
@left:
    CLC
    RTS
@right:
    SEC
    RTS

; L877C (the column of black sprites hiding x 248-255 of the world map), in place of
; LDX #$0E / LDY #$00: with a margin that column would sit in the middle of the picture; its
; 15 sprites are put out of sight instead.
wide_strip:
    LDA      WIDE_SIG
    CMP      #$57
    BNE      @original
    LDA      WIDE_HOST
    BEQ      @original
    LDX      #$0E
    LDY      #$00
    LDA      #$F8
@hide:
    STA      $0208,y
    INY
    INY
    INY
    INY
    DEX
    BPL      @hide
    RTS
@original:
    LDX      #$0E
    LDY      #$00
    JMP      wide_l8780

; ---------------------------------------------------------------------------- ;
; Code the fixed bank calls: placed in bank 6 at $AD00 and run at $5800 through the extension's PRG window,
; whatever bank is mapped at $8000.
.segment "WIDE6W"
.org WV_CODE

; Entry points at fixed addresses.
wr_nmi:     JMP wr_nmi_body
wr_split:   JMP wr_split_body
wr_enemy:   JMP wr_enemy_body
wr_coltest: JMP wr_coltest_body
wr_town:    JMP wr_town_body
wr_town_hi: JMP wr_town_hi_body
wr_proj:    JMP wr_proj_body
wr_hidecol: JMP wr_hidecol_body
wr_spawncol: JMP wr_spawncol_body
wr_ldf01:   JMP wr_ldf01_body

; NMI, in place of LDX PPU_STATUS: the frame starts with the sides hidden (status bar).
wr_nmi_body:
    LDA      $0736                     ; game mode: the world map shows its margins whole
    CMP      #$05
    BNE      @hide
    LDA      WIDE_HOST
    CMP      #WIDE_MAX_MARGIN
    BCC      @capped
    LDA      #WIDE_MAX_MARGIN
@capped:
    STA      WIDE_REQ
    LDA      #$FF
    BNE      @set
@hide:
    LDA      #$00                      ; side view: the status bar first, the split opens them
@set:
    STA      WIDE_LEFT
    STA      WIDE_RIGHT
    LDX      $2002
    RTS

; Side-view split, in place of STA $2000 / STX $2005 / STY $2005: the playfield below shows
; the extents worked out for it.
wr_split_body:
    STA      $2000
    STX      $2005
    STY      $2005
    LDA      WV_WL
    STA      WIDE_LEFT
    LDA      WV_WR
    STA      WIDE_RIGHT
    RTS

; The projectile of one enemy slot (LD56D: placement and display): its column test accepts the
; margins too.
wr_proj_body:
    JMP      LD56D                     ; as the original for now (see wr_coltest_body)

; One enemy of the enemy loop (LD5DA: placement, behaviour, display): every column test it
; makes accepts the margins.
wr_enemy_body:
.ifdef NO_FLAG
    JMP      LD5DA
.endif
    LDA      WV_M
    BNE      @wide
    JMP      LD5DA
@wide:
    LDA      #WV_ENEMY_ON
    STA      WV_ENEMY
    JSR      LD5DA
    LDA      #$00
    STA      WV_ENEMY
    RTS

; The column test of bank7_code51, in place of
;   LDA $0E / CMP $072C / LDA $0F / SBC $072A
; Z set when the column at $0E/$0F is on the screen, exactly as the original: $C8-$CA keep their
; meaning for the game's logic (contact, removal), which works on the 8-bit screen x. For an enemy
; (object index $00 = 1) placed by the enemy loop with a margin, the test also builds WV_DMASK, the
; columns that stay hidden for DISPLAY only, the margins accepted; wr_hidecol draws with it.
wr_coltest_body:
    LDA      $0E
    CMP      $072C
    LDA      $0F
    SBC      $072A
    STA      WV_T1                     ; high byte of column - scroll
    LDA      WV_ENEMY
    CMP      #WV_ENEMY_ON
    BNE      @original
    LDA      $00
    CMP      #$01
    BNE      @original
    CPY      #$03                      ; first column of the object: a new mask
    BNE      @test
    LDA      #$00
    STA      WV_DMASK
@test:
    LDA      $0E
    SEC
    SBC      $072C
    CLC
    ADC      WV_M                      ; x + margin
    STA      WV_T0
    LDA      WV_T1
    ADC      #$00
    BEQ      @original                 ; 0 <= x + margin < 256: shows
    CMP      #$01
    BNE      @hidden
    LDA      WV_T0
    CMP      WV_2M
    BCC      @original                 ; < 256 + 2 margin: shows
@hidden:
    LDA      wr_column_bits,y
    ORA      WV_DMASK
    STA      WV_DMASK
@original:
    LDA      WV_T1                     ; Z: the original test
    RTS

wr_column_bits:
.byt    $08,$04,$02,$01

; LF1D7 (hide the off-screen columns of what was just drawn), in place of LDA $0A / AND $C9:
; while the enemy loop runs with a margin, the display mask of wr_coltest_body decides.
wr_hidecol_body:
    LDA      WV_ENEMY
    CMP      #WV_ENEMY_ON
    BEQ      @display
    LDA      $0A
    AND      $C9
    RTS
@display:
    LDA      $0A
    AND      WV_DMASK
    RTS

; Townsfolk spawn (bank 3), in place of ADC table_low,y / ADC table_high,y: the offsets from the
; scroll are -32 - margin (Y = 0) and 264 + margin (Y = 1) instead of -32 and 264, so they walk
; in from past the margins. A = scroll low, carry clear on entry, like the ADC it replaces.
wr_town_body:
    STA      WV_T0
    CPY      #$00
    BNE      @right
    LDA      #$E0                      ; low byte of -32 - margin
    SEC
    SBC      WV_M
    STA      WV_T1
    LDA      #$FF
    BNE      @high
@right:
    LDA      #$08                      ; low byte of 264 + margin
    CLC
    ADC      WV_M
    STA      WV_T1
    LDA      #$01
@high:
    STA      WV_TOWNHI
    LDA      WV_T0
    CLC
    ADC      WV_T1
    RTS

wr_town_hi_body:
    ADC      WV_TOWNHI
    RTS

; LD625 (spawn the enemy of the list whose column the streaming cursor X just reached), in
; place of LDA $0732,x / STA $00 / LDA $0734,x / STA $01. The cursors run 73 to 99 pixels past the
; screen, inside a 16:9 margin: with a margin over 72 the column compared is pushed out by
; (margin - 57) / 16 columns, so a new 16-pixel enemy starts wholly outside the margin. The
; cursors themselves stay (the level streaming uses them), and so does the room loader's sweep
; (called from $D132), which places the enemies of the screen Link arrives on.
wr_spawncol_body:
    LDA      $0732,x
    STA      $00
    LDA      $0734,x
    STA      $01
    LDA      WV_M
    CMP      #73
    BCC      @rts
    CPX      #$02
    BCS      @rts
    STX      WV_T2
    TSX                                ; $0103,x/$0104,x: where LD625 returns to
    LDA      $0104,x
    CMP      #$D1
    BNE      @shift
    LDA      $0103,x
    CMP      #$34
    BEQ      @restore                  ; the room loader
@shift:
    LDA      WV_M                      ; columns = (margin - 57) / 16
    SEC
    SBC      #57
    LSR      A
    LSR      A
    LSR      A
    LSR      A
    STA      WV_T3
    LDA      $00                       ; level column = page * 16 + column
    ASL      A
    ASL      A
    ASL      A
    ASL      A
    ORA      $01
    LDX      WV_T2
    BNE      @right
    SEC                                ; left cursor: further left
    SBC      WV_T3
    BCS      @store
    LDA      #$FF                      ; before the level: page $FF matches no entry
    STA      $00
    BNE      @restore
@right:
    CLC                                ; right cursor: further right
    ADC      WV_T3
@store:
    PHA
    AND      #$0F
    STA      $01
    PLA
    LSR      A
    LSR      A
    LSR      A
    LSR      A
    STA      $00
@restore:
    LDX      WV_T2
@rts:
    RTS

; LDF01 (PPU address of the next world map update), in place of
; LDA $0A / AND #$10 / LSR / ORA $00 / STA $79: with four nametables the map ring is 32 metatile
; columns wide, so bit 4 of the map column ($76) selects the left or right nametable.
wr_ldf01_body:
    LDA      $0A
    AND      #$10
    LSR      A
    ORA      $00
    STA      $79
    LDA      $76
    AND      #$10
    BEQ      @rts
    LDA      $79
    ORA      #$04
    STA      $79
@rts:
    RTS

wr_end:
.reloc
.assert wr_end - WV_CODE <= $382, error, "widescreen window code past $B081"
```

### D. Difference against the base

`git diff --ignore-cr-at-eol c4c3a4c HEAD -- nes.cfg src/`, line endings normalised (the repository stores the sources with CRLF; apply with `git apply --ignore-whitespace` or by hand). The removed `.byt $FF` lines are the unused space the new segments take.

```diff
diff --git a/nes.cfg b/nes.cfg
index 820163a..8bbcb6c 100644
--- a/nes.cfg
+++ b/nes.cfg
@@ -13,12 +13,22 @@ MEMORY
 SEGMENTS
 {
 	PRG0: load = MEM0, type = ro, align = $4000;
+	WIDE0: load = MEM0, type = ro, start = $AA40;
+	PRG0B: load = MEM0, type = ro, start = $B082;
 	PRG1: load = MEM1, type = ro, align = $4000;
 	PRG2: load = MEM2, type = ro, align = $4000;
 	PRG3: load = MEM3, type = ro, align = $4000;
 	PRG4: load = MEM4, type = ro, align = $4000;
 	PRG5: load = MEM5, type = ro, align = $4000;
 	PRG6: load = MEM6, type = ro, align = $4000;
+	WIDE6W: load = MEM6, type = ro, start = $AD00;
+	PRG6B: load = MEM6, type = ro, start = $B082;
 	PRG7: load = MEM7, type = ro, align = $4000;
+	WIDE7A: load = MEM7, type = ro, start = $D39A;
+	PRG7B: load = MEM7, type = ro, start = $D3CA;
+	WIDE7B: load = MEM7, type = ro, start = $FEAA;
+	PRG7C: load = MEM7, type = ro, start = $FED0;
+	WIDE7C: load = MEM7, type = ro, start = $FF4C;
+	PRG7D: load = MEM7, type = ro, start = $FF70;
 	VECTORS: load = MEM7, type = ro, start = $FFFA;
 }
\ No newline at end of file
diff --git a/src/prg0.asm b/src/prg0.asm
index a653115..b6d9a2d 100644
--- a/src/prg0.asm
+++ b/src/prg0.asm
@@ -73,6 +73,8 @@ LEC02 = $EC02
 .import bank7_code33
 .import bank7_code34
 .import bank7_code51
+.import LD5DA
+.import LD56D
 .import bank7_code55
 .import bank7_Erase_Name_Tables_0and1__set_scroll_to_0_0
 .import bank7_Links_Display_Routine
@@ -294,7 +296,7 @@ Initialization_stuff:
     STA      $0562                     ; 0x16c $815C 8D 62 05                  ; Link's facing direction in OW (04 = down)
     LDA      $74                       ; 0x16f $815F A5 74                     ; X position in OW (square unit)
     SEC                                ; 0x171 $8161 38                        ;
-    SBC      #$08                      ; 0x172 $8162 E9 08                     ; relative X offset
+    SBC      #$10                      ; 0x172 $8162 E9 08                     ; relative X offset
     STA      $76                       ; 0x174 $8164 85 76                     ;; X Position on OW (Link)
 L8167     = * + $0001                                                          ;
     LDA      $73                       ; 0x176 $8166 A5 73                     ; Y position in OW (square unit)
@@ -312,16 +314,10 @@ L8167     = * + $0001                                                          ;
 L817B:                                                                          ;
     ASL                                ; 0x18b $817B 0A                        ;
     STA      $FC                       ; 0x18c $817C 85 FC                     ;
-    LDA      $76                       ; 0x18e $817E A5 76                     ; X position on map (Link)
-    ASL                                ; 0x190 $8180 0A                        ;
-    ASL                                ; 0x191 $8181 0A                        ;
-    ASL                                ; 0x192 $8182 0A                        ;
-    ASL                                ; 0x193 $8183 0A                        ;
-    STA      $FD                       ; 0x194 $8184 85 FD                     ;
-    TYA                                ; 0x196 $8186 98                        ;
-    ASL                                ; 0x197 $8187 0A                        ;
-    AND      #$02                      ; 0x198 $8188 29 02                     ; keep bits .... ..x.
-    STA      $0747                     ; 0x19a $818A 8D 47 07                  ;
+    JSR      wide_ow_init_scroll       ; widescreen, was $FD/$0747 from $76
+    JMP      wide_l818d
+.byt    $EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA
+wide_l818d:
     LDA      #$00                      ; 0x19d $818D A9 00                     ; A = 00
     STA      $7D                       ; 0x19f $818F 85 7D                     ;;number of pixels to move? automove? on overworld, only partially, causes bug
     STA      $7E                       ; 0x1a1 $8191 85 7E                     ;
@@ -577,12 +573,12 @@ L82F7:
     ASL                                ; 0x31d $830D 0A                        ;
     ASL                                ; 0x31e $830E 0A                        ;
     ASL                                ; 0x31f $830F 0A                        ;
-    CLC                                ; 0x320 $8310 18                        ;
-    ADC      $FD                       ; 0x321 $8311 65 FD                     ;
-    STA      $4E,x                     ; 0x323 $8313 95 4E                     ; Enemy X position (low byte)
+    JSR      wide_blob_spawn           ; widescreen, was CLC / ADC $FD / STA $4E,x
+    NOP
+    NOP
     LDY      $0B                       ; 0x325 $8315 A4 0B                     ;
     LDA      L8246,y                   ; 0x327 $8317 B9 46 82                  ;
-    STA      $050E,x                   ; 0x32a $831A 9D 0E 05                  ; Timer for Overworld Demon
+    JSR      wide_blob_life            ; widescreen, was STA $050E,x
     LDY      $08                       ; 0x32d $831D A4 08                     ;
     LDA      L8265,y                   ; 0x32f $831F B9 65 82                  ;
     STA      $82,x                     ; 0x332 $8322 95 82                     ;; Demon Type (1 = Weak, 2 = Strong, 3 = Fairy) (OW)
@@ -625,17 +621,10 @@ L8342:
     SBC      $7F                       ; 0x361 $8351 E5 7F                     ;
     STA      $0280,y                   ; 0x363 $8353 99 80 02                  ;
     STA      $0284,y                   ; 0x366 $8356 99 84 02                  ;
-    LDA      $4E,x                     ; 0x369 $8359 B5 4E                     ; Enemy X position
-    CLC                                ; 0x36b $835B 18                        ;
-    ADC      $0575,x                   ; 0x36c $835C 7D 75 05                  ;
-    STA      $4E,x                     ; 0x36f $835F 95 4E                     ; Enemy X position
-    SEC                                ; 0x371 $8361 38                        ;
-    SBC      $FD                       ; 0x372 $8362 E5 FD                     ;
-    STA      $0283,y                   ; 0x374 $8364 99 83 02                  ;
-    CLC                                ; 0x377 $8367 18                        ;
-L8368:                                                                          ;
-    ADC      #$08                      ; 0x378 $8368 69 08                     ;
-    STA      $0287,y                   ; 0x37a $836A 99 87 02                  ;
+    JSR      wide_blob_x               ; widescreen, was the 8-bit x move and the OAM x of both halves
+    JMP      wide_l836d
+.byt    $EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA,$EA
+wide_l836d:
     TXA                                ; 0x37d $836D 8A                        ;
     PHA                                ; 0x37e $836E 48                        ;
     LDA      $12                       ; 0x37f $836F A5 12                     ;; Frame Counter (ascending)
@@ -722,10 +711,8 @@ L83FD:
     LDA      $0280,y                   ; 0x40d $83FD B9 80 02                  ;
     CMP      #$F8                      ; 0x410 $8400 C9 F8                     ;
     BCS      L840B                     ; 0x412 $8402 B0 07                     ;
-    LDA      $0283,y                   ; 0x414 $8404 B9 83 02                  ;
-    CMP      #$F8                      ; 0x417 $8407 C9 F8                     ;
-L8409:                                                                          ;
-    BCC      L8412                     ; 0x419 $8409 90 07                     ;
+    JMP      L8412                     ; widescreen, was LDA $0283,y / CMP #$F8 / BCC L8412 (wide_blob_x hides what is past the margins)
+.byt    $EA,$EA,$EA,$EA
 L840B:                                                                          ;
     LDA      #$00                      ; 0x41b $840B A9 00                     ; A = 00
     STA      $82,x                     ; 0x41d $840D 95 82                     ;; Demon Type (1 = Weak, 2 = Strong, 3 = Fairy) (OW)
@@ -775,11 +762,11 @@ L8435:
     CMP      #$20                      ; 0x45b $844B C9 20                     ;
     BCS      L8464                     ; 0x45d $844D B0 15                     ;
     LDY      #$01                      ; 0x45f $844F A0 01                     ; Y = 01
-    LDA      $4E,x                     ; 0x461 $8451 B5 4E                     ; Enemy X position
-    SEC                                ; 0x463 $8453 38                        ;
-    SBC      $FD                       ; 0x464 $8454 E5 FD                     ;
-    CMP      $0203                     ; 0x466 $8456 CD 03 02                  ;
-    BCC      L845D                     ; 0x469 $8459 90 02                     ;
+    JSR      wide_blob_cmp             ; widescreen, was LDA $4E,x / SEC / SBC $FD / CMP $0203
+    BCC      L845D
+    BCS      wide_l845b
+.byt    $EA,$EA,$EA
+wide_l845b:
     LDY      #$FF                      ; 0x46b $845B A0 FF                     ; Y = FF
 L845D:                                                                          ;
     TYA                                ; 0x46d $845D 98                        ;
@@ -1043,7 +1030,7 @@ L8601:
 ; ---------------------------------------------------------------------------- ;
 L861B:                                                                          ;
     LDA      $74                       ; 0x62b $861B A5 74                     ; X position on map (square unit)
-    ADC      #$07                      ; 0x62d $861D 69 07                     ;
+    ADC      #$0F                      ; 0x62d $861D 69 07                     ;
 L861F:                                                                          ;
     STA      $76                       ; 0x62f $861F 85 76                     ; X position on map (Link)
     LDA      $73                       ; 0x631 $8621 A5 73                     ; Y position on map (square unit)
@@ -1076,7 +1063,7 @@ L8647:
 ; ---------------------------------------------------------------------------- ;
 L864C:                                                                          ;
     LDA      $74                       ; 0x65c $864C A5 74                     ; X position on map (square unit)
-    SBC      #$07                      ; 0x65e $864E E9 07                     ;
+    SBC      #$0F                      ; 0x65e $864E E9 07                     ;
     STA      $76                       ; 0x660 $8650 85 76                     ;; X Position on OW (Link)
     LDA      $73                       ; 0x662 $8652 A5 73                     ;; Overworld Y; Y Position on OW (square unit)
     SEC                                ; 0x664 $8654 38                        ;
@@ -1107,7 +1094,7 @@ L866B:
 ; ---------------------------------------------------------------------------- ;
 L867D:                                                                          ;
     LDA      $74                       ; 0x68d $867D A5 74                     ;; Overworld X; X Position on OW (square unit)
-    SBC      #$07                      ; 0x68f $867F E9 07                     ;
+    SBC      #$0F                      ; 0x68f $867F E9 07                     ;
     STA      $76                       ; 0x691 $8681 85 76                     ;; X Position on OW (Link)
     LDA      $73                       ; 0x693 $8683 A5 73                     ;; Overworld Y; Y Position on OW (square unit)
     CLC                                ; 0x695 $8685 18                        ;
@@ -1130,7 +1117,7 @@ L869E:
 ; ---------------------------------------------------------------------------- ;
 L869F:                                                                          ;
     LDA      $74                       ; 0x6af $869F A5 74                     ;; Overworld X; X Position on OW (square unit)
-    SBC      #$07                      ; 0x6b1 $86A1 E9 07                     ;
+    SBC      #$0F                      ; 0x6b1 $86A1 E9 07                     ;
     STA      $76                       ; 0x6b3 $86A3 85 76                     ;; X Position on OW (Link)
     LDA      $73                       ; 0x6b5 $86A5 A5 73                     ;; Overworld Y; Y Position on OW (square unit)
     SEC                                ; 0x6b7 $86A7 38                        ;
@@ -1150,11 +1137,9 @@ L86BB:
     LDA      $0562                     ; 0x6cf $86BF AD 62 05                  ; Link's facing direction in OW
     AND      #$03                      ; 0x6d2 $86C2 29 03                     ; keep bits .... ..x.
     BEQ      L86D6                     ; 0x6d4 $86C4 F0 10                     ;
-    DEC      $FD                       ; 0x6d6 $86C6 C6 FD                     ;
-    LSR                                ; 0x6d8 $86C8 4A                        ;
-    BCC      L86CF                     ; 0x6d9 $86C9 90 04                     ;
-    INC      $FD                       ; 0x6db $86CB E6 FD                     ;
-    INC      $FD                       ; 0x6dd $86CD E6 FD                     ;
+    JSR      wide_ow_hscroll           ; widescreen, was DEC $FD / LSR / BCC L86CF / INC $FD / INC $FD
+    JMP      L86CF
+.byt    $EA,$EA,$EA
 L86CF:                                                                          ;
     LDA      $75                       ; 0x6df $86CF A5 75                     ; Y position on map (Link)
     STA      $01                       ; 0x6e1 $86D1 85 01                     ;
@@ -1264,8 +1249,9 @@ L876B:
 L8779:                                                                          ;
     STX      $0202                     ; 0x789 $8779 8E 02 02                  ;
 L877C:                                                                          ;
-    LDX      #$0E                      ; 0x78c $877C A2 0E                     ; X = 0E
-    LDY      #$00                      ; 0x78e $877E A0 00                     ; Y = 00
+    JMP      wide_strip                ; widescreen, was LDX #$0E / LDY #$00
+.byt    $EA
+wide_l8780:
     STY      $00                       ; 0x790 $8780 84 00                     ;
 L8782:                                                                          ;
     LDA      $00                       ; 0x792 $8782 A5 00                     ;
@@ -1371,7 +1357,7 @@ L882C:
     LDA      #$FF                      ; 0x857 $8847 A9 FF                     ; A = FF
     STA      L0302                     ; 0x859 $8849 8D 02 03                  ;; Used when writing text to screen
     LDA      $7E                       ; 0x85c $884C A5 7E                     ;
-    CMP      #$06                      ; 0x85e $884E C9 06                     ;
+    CMP      #$07                      ; 0x85e $884E C9 06                     ;
     BNE      L882C                     ; 0x860 $8850 D0 DA                     ;
     INC      $75                       ; 0x862 $8852 E6 75                     ;; Y Position on OW (Link)
     LDA      $75                       ; 0x864 $8854 A5 75                     ;; Y Position on OW (Link)
@@ -1428,7 +1414,7 @@ L88B2:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 L88B3:                                                                          ;
-    LDA       a:$7E                     ; 0x8c3 $88B3 AD 7E 00                  ;
+    JMP      wide_vdispatch            ; widescreen, was LDA a:$7E (the vertical states are in wide_vdispatch)
     JSR      bank7_PullAddrFromTableFollowingThisJSR_withIndexOfA_then_JMP; 0x8c6 $88B6 20 85 D3;
 unknown_pointer_table1:                                                         ;
 .word    L8BFE                         ; 0x8c9 $88B9 FE 8B                     ;
@@ -1624,7 +1610,7 @@ L89E6:
     STA      $7A                       ; 0xa06 $89F6 85 7A                     ;;are used to draw the Overworld tiles? offset in the Name Table(s) for Overworld redrawing.
     LDA      $79                       ; 0xa08 $89F8 A5 79                     ;;are used to draw the Overworld tiles? offset in the Name Table(s) for Overworld redrawing.
     EOR      #$08                      ; 0xa0a $89FA 49 08                     ; flip bits .... x...
-    AND      #$F8                      ; 0xa0c $89FC 29 F8                     ; keep bits xxxx x...
+    AND      #$FC                      ; 0xa0c $89FC 29 F8                     ; keep bits xxxx x...
     STA      $79                       ; 0xa0e $89FE 85 79                     ;;are used to draw the Overworld tiles? offset in the Name Table(s) for Overworld redrawing.
     LDA      $07                       ; 0xa10 $8A00 A5 07                     ;
     STA      $17                       ; 0xa12 $8A02 85 17                     ;; Link's Shield Position; Link's Shield Position (0 = Link ducked)
@@ -1658,7 +1644,7 @@ overworld6:
     LSR                                ; 0xa39 $8A29 4A                        ;
     BCS      L8A36                     ; 0xa3a $8A2A B0 0A                     ;
 L8A2C:                                                                          ;
-    LDA      #$06                      ; 0xa3c $8A2C A9 06                     ; A = 06
+    LDA      #$07                      ; 0xa3c $8A2C A9 06                     ; A = 06
     STA      $7E                       ; 0xa3e $8A2E 85 7E                     ;
     RTS                                ; 0xa40 $8A30 60                        ;
                                                                                ;
@@ -1812,7 +1798,7 @@ L8B1C:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 bank0_unknown8:                                                                 ;
-.byt    $0F,$F1                        ; 0xb3c $8B2C 0F F1                     ;
+.byt    $1F,$E1                        ; 0xb3c $8B2C 0F F1                     ;
 ; ---------------------------------------------------------------------------- ;
 overworld7:                                                                     ;
     DEY                                ; 0xb3e $8B2E 88                        ;
@@ -1865,10 +1851,11 @@ L8B6E:
     BEQ      L8B42                     ; 0xb84 $8B74 F0 CC                     ;
 L8B76:                                                                          ;
     STX      L000E                     ; 0xb86 $8B76 86 0E                     ;
-    LDA      $77                       ; 0xb88 $8B78 A5 77                     ;;projectile speed	77,78,79,7A,7B,7C	77,Y
-    AND      #$10                      ; 0xb8a $8B7A 29 10                     ; keep bits ...x ....
-    LSR                                ; 0xb8c $8B7C 4A                        ;
-    ORA      #$23                      ; 0xb8d $8B7D 09 23                     ;;Set Bits:0010_0011
+    JSR      wide_attr_hi              ; widescreen, was LDA $77 / AND #$10 / LSR / ORA #$23
+    NOP
+    NOP
+    NOP
+    NOP
     STA      L0302,x                   ; 0xb8f $8B7F 9D 02 03                  ;
     LDA      $77                       ; 0xb92 $8B82 A5 77                     ;;projectile speed	77,78,79,7A,7B,7C	77,Y
     AND      #$0E                      ; 0xb94 $8B84 29 0E                     ;;Keep Bits:0000_1110
@@ -1948,19 +1935,19 @@ L8BF5:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 L8BFE:                                                                          ;
-    LDX      #$3E                      ; 0xc0e $8BFE A2 3E                     ; X = 3E
+    LDX      #$5F                      ; 0xc0e $8BFE A2 3E                     ; X = 3E
     LDA      #$0C                      ; 0xc10 $8C00 A9 0C                     ; A = 0C (Water)
 L8C02:                                                                          ;
-    STA      $048B,x                   ; 0xc12 $8C02 9D 8B 04                  ;
+    STA      $0470,x                   ; 0xc12 $8C02 9D 8B 04                  ;
     DEX                                ; 0xc15 $8C05 CA                        ;
     BPL      L8C02                     ; 0xc16 $8C06 10 FA                     ;
-    LDX      #$14                      ; 0xc18 $8C08 A2 14                     ; X = 14
+    LDX      #$1B                      ; 0xc18 $8C08 A2 14                     ; X = 14
     LDY      $0706                     ; 0xc1a $8C0A AC 06 07                  ; Current Region
     CPY      #$02                      ; 0xc1d $8C0D C0 02                     ;
     BCS      L8C13                     ; 0xc1f $8C0F B0 02                     ;
     LDA      #$0B                      ; 0xc21 $8C11 A9 0B                     ; A = 0B (Mountain)
 L8C13:                                                                          ;
-    STA      $0477,x                   ; 0xc23 $8C13 9D 77 04                  ;
+    STA      $0470,x                   ; 0xc23 $8C13 9D 77 04                  ;
     DEX                                ; 0xc26 $8C16 CA                        ;
     BPL      L8C13                     ; 0xc27 $8C17 10 FA                     ;
     LDX      #$00                      ; 0xc29 $8C19 A2 00                     ; X = 00
@@ -4056,7 +4043,8 @@ L99E5:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 L99E6:                                                                          ;
-    LDA      $0740                     ; 0x19f6 $99E6 AD 40 07                 ;
+    JMP      wide_frame                ; widescreen hook, was LDA $0740 (3 bytes)
+wide_l99e9:
     BNE      L9A04                     ; 0x19f9 $99E9 D0 19                    ;
     LDA      $0721                     ; 0x19fb $99EB AD 21 07                 ;
     BMI      L99F9                     ; 0x19fe $99EE 30 09                    ;
@@ -4090,11 +4078,11 @@ L9A18:
 bank0_unknown_pointer_table2:                                                   ;
 .word    L9B13                         ; 0x1a2e $9A1E 13 9B                    ;
 .word    bank0_unknown22               ; 0x1a30 $9A20 30 9A                    ;
-.word    bank0_unknown26               ; 0x1a32 $9A22 B8 9A                    ;
+.word    wide_column_done              ; widescreen, was bank0_unknown26 (second half of a column)
 .word    bank0_unknown26               ; 0x1a34 $9A24 B8 9A                    ;
 .word    L9A77                         ; 0x1a36 $9A26 77 9A                    ;
 .word    bank0_unknown22               ; 0x1a38 $9A28 30 9A                    ;
-.word    bank0_unknown26               ; 0x1a3a $9A2A B8 9A                    ;
+.word    wide_column_done              ; widescreen, was bank0_unknown26 (second half of a column)
 .word    bank0_unknown26               ; 0x1a3c $9A2C B8 9A                    ;
 .word    L9A77                         ; 0x1a3e $9A2E 77 9A                    ;
 ; ---------------------------------------------------------------------------- ;
@@ -6205,208 +6193,10 @@ LAA36:
     RTS                                ; 0x2a4f $AA3F 60                       ;
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
-bank0_unused_space_big:                                                         ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a50 $AA40 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a58 $AA48 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a60 $AA50 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a68 $AA58 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a70 $AA60 FF FF FF FF FF FF FF FF  ; if Slash Step >= 2, goto ResetVariable
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a78 $AA68 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a80 $AA70 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a88 $AA78 FF FF FF FF FF FF FF FF  ; if (A >= 30) Flashing Routine
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a90 $AA80 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2a98 $AA88 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2aa0 $AA90 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2aa8 $AA98 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ab0 $AAA0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ab8 $AAA8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ac0 $AAB0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ac8 $AAB8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ad0 $AAC0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ad8 $AAC8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ae0 $AAD0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ae8 $AAD8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2af0 $AAE0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2af8 $AAE8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b00 $AAF0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b08 $AAF8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b10 $AB00 FF FF FF FF FF FF FF FF  ; Randomizer
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b18 $AB08 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b20 $AB10 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b28 $AB18 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b30 $AB20 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b38 $AB28 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b40 $AB30 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b48 $AB38 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b50 $AB40 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b58 $AB48 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b60 $AB50 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b68 $AB58 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b70 $AB60 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b78 $AB68 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b80 $AB70 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b88 $AB78 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b90 $AB80 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2b98 $AB88 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ba0 $AB90 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ba8 $AB98 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bb0 $ABA0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bb8 $ABA8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bc0 $ABB0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bc8 $ABB8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bd0 $ABC0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bd8 $ABC8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2be0 $ABD0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2be8 $ABD8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bf0 $ABE0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2bf8 $ABE8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c00 $ABF0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c08 $ABF8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c10 $AC00 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c18 $AC08 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c20 $AC10 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c28 $AC18 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c30 $AC20 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c38 $AC28 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c40 $AC30 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c48 $AC38 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c50 $AC40 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c58 $AC48 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c60 $AC50 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c68 $AC58 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c70 $AC60 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c78 $AC68 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c80 $AC70 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c88 $AC78 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c90 $AC80 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2c98 $AC88 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ca0 $AC90 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ca8 $AC98 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cb0 $ACA0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cb8 $ACA8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cc0 $ACB0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cc8 $ACB8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cd0 $ACC0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cd8 $ACC8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ce0 $ACD0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ce8 $ACD8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cf0 $ACE0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2cf8 $ACE8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d00 $ACF0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d08 $ACF8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d10 $AD00 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d18 $AD08 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d20 $AD10 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d28 $AD18 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d30 $AD20 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d38 $AD28 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d40 $AD30 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d48 $AD38 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d50 $AD40 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d58 $AD48 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d60 $AD50 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d68 $AD58 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d70 $AD60 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d78 $AD68 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d80 $AD70 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d88 $AD78 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d90 $AD80 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2d98 $AD88 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2da0 $AD90 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2da8 $AD98 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2db0 $ADA0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2db8 $ADA8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2dc0 $ADB0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2dc8 $ADB8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2dd0 $ADC0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2dd8 $ADC8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2de0 $ADD0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2de8 $ADD8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2df0 $ADE0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2df8 $ADE8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e00 $ADF0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e08 $ADF8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e10 $AE00 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e18 $AE08 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e20 $AE10 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e28 $AE18 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e30 $AE20 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e38 $AE28 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e40 $AE30 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e48 $AE38 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e50 $AE40 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e58 $AE48 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e60 $AE50 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e68 $AE58 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e70 $AE60 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e78 $AE68 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e80 $AE70 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e88 $AE78 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e90 $AE80 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2e98 $AE88 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ea0 $AE90 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ea8 $AE98 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2eb0 $AEA0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2eb8 $AEA8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ec0 $AEB0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ec8 $AEB8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ed0 $AEC0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ed8 $AEC8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ee0 $AED0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ee8 $AED8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ef0 $AEE0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ef8 $AEE8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f00 $AEF0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f08 $AEF8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f10 $AF00 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f18 $AF08 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f20 $AF10 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f28 $AF18 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f30 $AF20 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f38 $AF28 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f40 $AF30 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f48 $AF38 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f50 $AF40 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f58 $AF48 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f60 $AF50 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f68 $AF58 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f70 $AF60 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f78 $AF68 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f80 $AF70 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f88 $AF78 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f90 $AF80 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2f98 $AF88 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fa0 $AF90 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fa8 $AF98 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fb0 $AFA0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fb8 $AFA8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fc0 $AFB0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fc8 $AFB8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fd0 $AFC0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fd8 $AFC8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fe0 $AFD0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2fe8 $AFD8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ff0 $AFE0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x2ff8 $AFE8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3000 $AFF0 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3008 $AFF8 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3010 $B000 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3018 $B008 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3020 $B010 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3028 $B018 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3030 $B020 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3038 $B028 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3040 $B030 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3048 $B038 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3050 $B040 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3058 $B048 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3060 $B050 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3068 $B058 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3070 $B060 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3078 $B068 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3080 $B070 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3088 $B078 FF FF FF FF FF FF FF FF  ;
-.byt    $FF,$FF                        ; 0x3090 $B080 FF FF                    ;
+; widescreen: the code of the hack lives in the unused space of the bank
+.segment "WIDE0"
+.include "wide_bank0.asm"
+.segment "PRG0B"
 LB082:                                                                          ;
 .byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x3092 $B082 FF FF FF FF FF FF FF FF  ;
 .byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x309a $B08A FF FF FF FF FF FF FF FF  ;
diff --git a/src/prg3.asm b/src/prg3.asm
index 380f2ea..8444603 100644
--- a/src/prg3.asm
+++ b/src/prg3.asm
@@ -76,6 +76,8 @@ LEA32 = $EA32
 LF0D7 = $F0D7
 LF1F4 = $F1F4
 
+.import wr_town
+.import wr_town_hi
 .import bank7_Change_Enemy_Facing_Direction_and_X_Velocity
 .import bank7_Determine_Enemy_Facing_Direction_relative_to_Link
 .import bank7_Display
@@ -1712,10 +1714,10 @@ L971B:
     TAY                                ; 0xd72d $971D A8                       ;
     LDA      $072C                     ; 0xd72e $971E AD 2C 07                 ; Scrolling Offset Low Byte
     CLC                                ; 0xd731 $9721 18                       ;
-    ADC      bank3_Townfolks_initial_X_Position_adjustment_2low_then_2high,y; 0xd732 $9722 79 DC 96;
+    JSR      wr_town                   ; widescreen, was ADC bank3_Townfolks_initial_X_Position_adjustment_2low_then_2high,y
     STA      $4E,x                     ; 0xd735 $9725 95 4E                    ; Enemy X Position (low byte)
     LDA      $072A                     ; 0xd737 $9727 AD 2A 07                 ; Scrolling Offset High Byte
-    ADC      L96DE,y                   ; 0xd73a $972A 79 DE 96                 ;
+    JSR      wr_town_hi                ; widescreen, was ADC L96DE,y
     STA      $3C,x                     ; 0xd73d $972D 95 3C                    ; Enemy X Position (high byte)
     INY                                ; 0xd73f $972F C8                       ;
     STY      $60,x                     ; 0xd740 $9730 94 60                    ; Enemy facing direction
diff --git a/src/prg6.asm b/src/prg6.asm
index 7b8b3ea..0cb2064 100644
--- a/src/prg6.asm
+++ b/src/prg6.asm
@@ -3612,120 +3612,8 @@ Bank6__UNUSED_2:
 .byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1acf1 $ACE1 FF FF FF FF FF FF FF FF ;
 .byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1acf9 $ACE9 FF FF FF FF FF FF FF FF ;
 .byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad01 $ACF1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad09 $ACF9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad11 $AD01 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad19 $AD09 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad21 $AD11 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad29 $AD19 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad31 $AD21 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad39 $AD29 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad41 $AD31 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad49 $AD39 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad51 $AD41 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad59 $AD49 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad61 $AD51 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad69 $AD59 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad71 $AD61 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad79 $AD69 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad81 $AD71 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad89 $AD79 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad91 $AD81 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ad99 $AD89 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ada1 $AD91 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ada9 $AD99 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1adb1 $ADA1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1adb9 $ADA9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1adc1 $ADB1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1adc9 $ADB9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1add1 $ADC1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1add9 $ADC9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ade1 $ADD1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ade9 $ADD9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1adf1 $ADE1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1adf9 $ADE9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae01 $ADF1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae09 $ADF9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae11 $AE01 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae19 $AE09 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae21 $AE11 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae29 $AE19 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae31 $AE21 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae39 $AE29 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae41 $AE31 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae49 $AE39 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae51 $AE41 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae59 $AE49 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae61 $AE51 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae69 $AE59 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae71 $AE61 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae79 $AE69 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae81 $AE71 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae89 $AE79 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae91 $AE81 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ae99 $AE89 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aea1 $AE91 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aea9 $AE99 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aeb1 $AEA1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aeb9 $AEA9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aec1 $AEB1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aec9 $AEB9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aed1 $AEC1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aed9 $AEC9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aee1 $AED1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aee9 $AED9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aef1 $AEE1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aef9 $AEE9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af01 $AEF1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af09 $AEF9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af11 $AF01 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af19 $AF09 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af21 $AF11 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af29 $AF19 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af31 $AF21 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af39 $AF29 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af41 $AF31 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af49 $AF39 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af51 $AF41 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af59 $AF49 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af61 $AF51 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af69 $AF59 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af71 $AF61 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af79 $AF69 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af81 $AF71 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af89 $AF79 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af91 $AF81 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1af99 $AF89 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afa1 $AF91 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afa9 $AF99 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afb1 $AFA1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afb9 $AFA9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afc1 $AFB1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afc9 $AFB9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afd1 $AFC1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afd9 $AFC9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afe1 $AFD1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1afe9 $AFD9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aff1 $AFE1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1aff9 $AFE9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b001 $AFF1 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b009 $AFF9 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b011 $B001 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b019 $B009 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b021 $B011 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b029 $B019 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b031 $B021 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b039 $B029 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b041 $B031 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b049 $B039 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b051 $B041 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b059 $B049 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b061 $B051 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b069 $B059 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b071 $B061 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b079 $B069 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b081 $B071 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b089 $B079 FF FF FF FF FF FF FF FF ;
-.byt    $FF                            ; 0x1b091 $B081 FF                      ;
+; widescreen: the code of the hack the fixed bank calls lives here (segment WIDE6W)
+.segment "PRG6B"
 LB082:                                                                          ;
 .byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b092 $B082 FF FF FF FF FF FF FF FF ;
 .byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1b09a $B08A FF FF FF FF FF FF FF FF ;
diff --git a/src/prg7.asm b/src/prg7.asm
index 4f26f1a..20665a4 100644
--- a/src/prg7.asm
+++ b/src/prg7.asm
@@ -109,6 +109,15 @@ LBF00 = $BF00
 .import Some_Palettes_Data_related_to_Falling_Animation
 .import Spell_Casting_Routine
 .import startup_init_begin_game
+.import wide_boot
+.import wr_nmi
+.import wr_split
+.import wr_enemy
+.import wr_proj
+.import wr_hidecol
+.import wr_spawncol
+.import wr_ldf01
+.import wr_coltest
 .import Tables_for_Game_Over_screen_text
 
 .export bank7_Change_Enemy_Facing_Direction_and_X_Velocity
@@ -182,6 +191,8 @@ LBF00 = $BF00
 .export bank7_code45
 .export bank7_code49
 .export bank7_code51
+.export LD5DA
+.export LD56D
 .export bank7_code55
 .export bank7_forest_chop_with_hammer
 .export bank7_idem__maybe
@@ -195,6 +206,7 @@ LBF00 = $BF00
 .export SwapToSavedPRG
 .export L_Bank6Code0
 
+.include "wide.inc"
 .segment "PRG7"
 
 bank7_PowerON_code:                                                            ;
@@ -301,7 +313,7 @@ bank7_NMI_Entry_Point:                                                         ;
     STA      $FE                       ; 0x1c0b8 $C0A8 85 FE                   ; does interesting effects when changed, perhaps involves palette? 
     AND      #$E1                      ; 0x1c0ba $C0AA 29 E1                   ; keep bits xxx. ...x
     STA      PPU_MASK                  ; 0x1c0bc $C0AC 8D 01 20                ; PPU Control Register 2
-    LDX      PPU_STATUS                ; 0x1c0bf $C0AF AE 02 20                ; PPU Status Register
+    JSR      wide7_nmi                 ; widescreen, was LDX PPU_STATUS
     LDA      #$00                      ; 0x1c0c2 $C0B2 A9 00                   ; A = 00
     STA      PPU_SCROLL                ; 0x1c0c4 $C0B4 8D 05 20                ; Screen Scroll Offsets
     STA      PPU_SCROLL                ; 0x1c0c7 $C0B7 8D 05 20                ;
@@ -398,7 +410,7 @@ bank7_NMI_Entry_Point:                                                         ;
     DEC      $0501,x                   ; 0x1c18c $C17C DE 01 05                ; decrease all 5xx timers (if > 0)
 :                                                                              ;
     DEX                                ; 0x1c18f $C17F CA                      ;
-    BPL      :-                        ; 0x1c190 $C180 10 F5                   ;
+    BPL      :--                       ; 0x1c190 $C180 10 F5                   ;
     INC      a:$12                     ; 0x1c192 $C182 EE 12 00                ;
     LDX      #$00                      ; 0x1c195 $C185 A2 00                   ; X = 00
     LDY      #$09                      ; 0x1c197 $C187 A0 09                   ; Y = 09
@@ -3132,12 +3144,22 @@ bank7_PullAddrFromTableFollowingThisJSR_withIndexOfA_then_JMP:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 bank7_UNUSED_D39A:                                                              ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1d3aa $D39A FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1d3b2 $D3A2 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1d3ba $D3AA FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1d3c2 $D3B2 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1d3ca $D3BA FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1d3d2 $D3C2 FF FF FF FF FF FF FF FF ;
+.segment "WIDE7A"
+; widescreen: NMI hook, from the fixed bank: (re)opens the extension's PRG window on the hack's
+; code first, then runs it. A machine restored from a state made without the hack has the
+; extension locked: it is unlocked again first.
+wide7_nmi:
+    LDA      WIDE_SIG
+    CMP      #$57
+    BEQ      @open
+    JSR      wide7_unlock
+@open:
+    LDA      #WV_CODE_BANK
+    STA      WIDE_WINDOW_BANK
+    LDA      #WV_CODE_PAGE
+    STA      WIDE_WINDOW_PAGE
+    JMP      wr_nmi
+.segment "PRG7B"
 bank7_table13:                                                                  ;
 .byt    $80,$40                        ; 0x1d3da $D3CA 80 40                   ;
 ; ---------------------------------------------------------------------------- ;
@@ -3269,9 +3291,9 @@ LD4B2:
 LD4C2:                                                                          ;
     DEY                                ; 0x1d4d2 $D4C2 88                      ;
     BNE      LD4C2                     ; 0x1d4d3 $D4C3 D0 FD                   ;
-    STA      $2000                     ; 0x1d4d5 $D4C5 8D 00 20                ;
-    STX      $2005                     ; 0x1d4d8 $D4C8 8E 05 20                ;
-    STY      $2005                     ; 0x1d4db $D4CB 8C 05 20                ;
+    JSR      wr_split                  ; widescreen, was STA $2000 / STX $2005 / STY $2005
+    JMP      LD4CE
+.byt    $EA,$EA,$EA                    ; unused
 LD4CE:                                                                          ;
     JSR      Hub_Update_Routine        ; 0x1d4de $D4CE 20 8D 96                ; Hub Update Routine
     LDA      $0565                     ; 0x1d4e1 $D4D1 AD 65 05                ;; Hit Points - Display
@@ -3402,9 +3424,9 @@ LD5BE:
     STA      $91,x                     ; 0x1d5d5 $D5C5 95 91                   ;
     LDA      LD59B,y                   ; 0x1d5d7 $D5C7 B9 9B D5                ;
     STA      $97,x                     ; 0x1d5da $D5CA 95 97                   ;
-    JSR      LD5DA                     ; 0x1d5dc $D5CC 20 DA D5                ;
+    JSR      wr_enemy                  ; widescreen, was JSR LD5DA
     LDX      $10                       ; 0x1d5df $D5CF A6 10                   ;; used as monster x register ;draw boss hp bar
-    JSR      LD56D                     ; 0x1d5e1 $D5D1 20 6D D5                ;
+    JSR      wr_proj                   ; widescreen, was JSR LD56D
     LDX      $10                       ; 0x1d5e4 $D5D4 A6 10                   ;; used as monster x register ;draw boss hp bar
     DEX                                ; 0x1d5e6 $D5D6 CA                      ;
     BPL      LD5B0                     ; 0x1d5e7 $D5D7 10 D7                   ;
@@ -3462,10 +3484,10 @@ LD624:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 LD625:                                                                          ;
-    LDA      $0732,x                   ; 0x1d635 $D625 BD 32 07                ;
-    STA      $00                       ; 0x1d638 $D628 85 00                   ;
-    LDA      $0734,x                   ; 0x1d63a $D62A BD 34 07                ;
-    STA      $01                       ; 0x1d63d $D62D 85 01                   ;
+    JSR      wr_spawncol               ; widescreen, was LDA $0732,x / STA $00 / LDA $0734,x / STA $01
+    JMP      wide_ld62f
+.byt    $EA,$EA,$EA,$EA                ; unused
+wide_ld62f:
     LDX      $10                       ; 0x1d63f $D62F A6 10                   ;; used as monster x register ;draw boss hp bar
     LDY      #$01                      ; 0x1d641 $D631 A0 01                   ; Y = 01
 LD633:                                                                          ;
@@ -4841,11 +4863,10 @@ LDF01:
     ASL                                ; 0x1df3c $DF2C 0A                      ;
     ADC      $01                       ; 0x1df3d $DF2D 65 01                   ;
     STA      $7A                       ; 0x1df3f $DF2F 85 7A                   ;;are used to draw the Overworld tiles? offset in the Name Table(s) for Overworld redrawing.
-    LDA      $0A                       ; 0x1df41 $DF31 A5 0A                   ;
-    AND      #$10                      ; 0x1df43 $DF33 29 10                   ; keep bits ...x ....
-    LSR                                ; 0x1df45 $DF35 4A                      ;
-    ORA      $00                       ; 0x1df46 $DF36 05 00                   ;
-    STA      $79                       ; 0x1df48 $DF38 85 79                   ;;are used to draw the Overworld tiles? offset in the Name Table(s) for Overworld redrawing.
+    JSR      wr_ldf01                  ; widescreen, was LDA $0A / AND #$10 / LSR / ORA $00 / STA $79
+    JMP      wide_ldf3a
+.byt    $EA,$EA,$EA
+wide_ldf3a:
     LDA      #$00                      ; 0x1df4a $DF3A A9 00                   ; A = 00
     STA      $7E                       ; 0x1df4c $DF3C 85 7E                   ;
     RTS                                ; 0x1df4e $DF3E 60                      ;
@@ -7522,8 +7543,8 @@ LF1D7:
     LDA      #$08                      ; 0x1f1e7 $F1D7 A9 08                   ; A = 08
     STA      $0A                       ; 0x1f1e9 $F1D9 85 0A                   ;
 LF1DB:                                                                          ;
-    LDA      $0A                       ; 0x1f1eb $F1DB A5 0A                   ;
-    AND      $C9                       ; 0x1f1ed $F1DD 25 C9                   ;
+    JSR      wr_hidecol                ; widescreen, was LDA $0A / AND $C9
+    NOP
     BEQ      LF1E6                     ; 0x1f1ef $F1DF F0 05                   ;
     LDA      #$F8                      ; 0x1f1f1 $F1E1 A9 F8                   ; A = F8
     STA      $0200,y                   ; 0x1f1f3 $F1E3 99 00 02                ;
@@ -7626,11 +7647,11 @@ LF286:
     LDA      $3B,x                     ; 0x1f29e $F28E B5 3B                   ; Link's Current X position (high byte)
     ADC      #$00                      ; 0x1f2a0 $F290 69 00                   ;
     STA      $0F                       ; 0x1f2a2 $F292 85 0F                   ;
-    LDA      L000E                     ; 0x1f2a4 $F294 A5 0E                   ;
-    CMP      $072C                     ; 0x1f2a6 $F296 CD 2C 07                ; Scrolling Offset Low Byte
-    LDA      $0F                       ; 0x1f2a9 $F299 A5 0F                   ;
-    SBC      $072A                     ; 0x1f2ab $F29B ED 2A 07                ; Scrolling Offset High Byte
-    BEQ      LF2AF                     ; 0x1f2ae $F29E F0 0F                   ;
+    JSR      wr_coltest                ; widescreen, was LDA $0E / CMP $072C / LDA $0F / SBC $072A
+    BEQ      LF2AF
+    BNE      wide_lf2a0
+.byt    $EA,$EA,$EA,$EA,$EA            ; unused (5 bytes of the original test)
+wide_lf2a0:
     STY      $01                       ; 0x1f2b0 $F2A0 84 01                   ;
     LDA      LF270,y                   ; 0x1f2b2 $F2A2 B9 70 F2                ;
     LDY      $00                       ; 0x1f2b5 $F2A5 A4 00                   ;
@@ -8156,11 +8177,26 @@ LFEA6:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 bank7_UNUSED_FEAA:                                                              ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1feba $FEAA FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1fec2 $FEB2 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1feca $FEBA FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1fed2 $FEC2 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF        ; 0x1feda $FECA FF FF FF FF FF FF       ;
+.segment "WIDE7B"
+; widescreen: what the power-on hook does (bank 0's wide_boot) bar the window, which the NMI hook
+; opens right after.
+wide7_unlock:
+    LDA      #$57
+    STA      WIDE_SIG
+    LDA      #$49
+    STA      WIDE_SIG
+    LDA      #$44
+    STA      WIDE_SIG
+    LDA      #$45
+    STA      WIDE_SIG
+    LDA      #$00
+    STA      WV_M
+    STA      WV_2M
+    STA      WV_WL
+    STA      WV_WR
+    STA      WV_ENEMY
+    RTS
+.segment "PRG7C"
 ; ---------------------------------------------------------------------------- ;
 bank7_code55:                                                                   ;
     JSR      SwapToSavedPRG; 0x1fee0 $FED0 20 C9 FF                ; Load Bank $0769
@@ -8237,11 +8273,15 @@ LFF36:
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 bank7_UNUSED_FF4C:                                                              ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ff5c $FF4C FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ff64 $FF54 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ff6c $FF5C FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF,$FF,$FF,$FF,$FF; 0x1ff74 $FF64 FF FF FF FF FF FF FF FF ;
-.byt    $FF,$FF,$FF,$FF                ; 0x1ff7c $FF6C FF FF FF FF             ;
+.segment "WIDE7C"
+; widescreen: power-on detour through bank 0 (unlock the extension, copy its RAM code)
+wide_reset:
+    JSR      SwapToPRG0
+    JSR      wide_boot
+    LDA      #$07
+    JSR      SwapPRG
+    JMP      bank7_PowerON_code
+.segment "PRG7D"
 ; ---------------------------------------------------------------------------- ;
 bank7_reset:                                                                   ;
     SEI                                ; 0x1ff80 $FF70 78                      ;
@@ -8264,7 +8304,7 @@ bank7_reset:                                                                   ;
     JSR      SwapCHR                   ; 0x1ffa2 $FF92 20 B1 FF                ;
     LDA      #$07                      ; 0x1ffa5 $FF95 A9 07                   ; A = 07
     JSR      SwapPRG                   ; 0x1ffa7 $FF97 20 CC FF                ;
-    JMP      bank7_PowerON_code        ; 0x1ffaa $FF9A 4C 00 C0                ;
+    JMP      wide_reset                ; widescreen, was JMP bank7_PowerON_code
                                                                                ;
 ; ---------------------------------------------------------------------------- ;
 ConfigureMMC1:                                                                 ;
```

### E. Where the target differs from the source ROM

Build: CRC-32 of the data `ADCEA53F`. Bank, CPU addresses, length, file offset (with a 16-byte header) and, for short runs, the bytes of the target. A run may include an unchanged byte between two changed ones. The `$BF70`-`$BFFF` runs of banks 0-6 and the single bytes of banks 0 and 5 outside the hack come from the base (section 1.1).

| Bank | CPU | Bytes | File | Target bytes |
|---|---|---|---|---|
| 0 | `$814D`-`$814D` | 1 | 0x0015D | FF |
| 0 | `$8152`-`$8152` | 1 | 0x00162 | FF |
| 0 | `$8163`-`$8163` | 1 | 0x00173 | 10 |
| 0 | `$817E`-`$818C` | 15 | 0x0018E | 20 9E AC 4C 8D 81 EA EA EA EA EA EA EA EA EA |
| 0 | `$8310`-`$8314` | 5 | 0x00320 | 20 D6 AE EA EA |
| 0 | `$831A`-`$831C` | 3 | 0x0032A | 20 2F AF |
| 0 | `$8359`-`$836C` | 20 | 0x00369 | 20 37 AF 4C 6D 83 EA EA EA EA EA EA EA EA EA EA EA EA EA EA |
| 0 | `$8404`-`$840A` | 7 | 0x00414 | 4C 12 84 EA EA EA EA |
| 0 | `$8451`-`$845A` | 10 | 0x00461 | 20 AF AF 90 07 B0 03 EA EA EA |
| 0 | `$861E`-`$861E` | 1 | 0x0062E | 0F |
| 0 | `$864F`-`$864F` | 1 | 0x0065F | 0F |
| 0 | `$8680`-`$8680` | 1 | 0x00690 | 0F |
| 0 | `$86A2`-`$86A2` | 1 | 0x006B2 | 0F |
| 0 | `$86C6`-`$86CE` | 9 | 0x006D6 | 20 C4 AC 4C CF 86 EA EA EA |
| 0 | `$877C`-`$877F` | 4 | 0x0078C | 4C C4 AF EA |
| 0 | `$884F`-`$884F` | 1 | 0x0085F | 07 |
| 0 | `$88B3`-`$88B5` | 3 | 0x008C3 | 4C F3 AC |
| 0 | `$89FD`-`$89FD` | 1 | 0x00A0D | FC |
| 0 | `$8A2D`-`$8A2D` | 1 | 0x00A3D | 07 |
| 0 | `$8B2C`-`$8B2D` | 2 | 0x00B3C | 1F E1 |
| 0 | `$8B78`-`$8B7E` | 7 | 0x00B88 | 20 DF AC EA EA EA EA |
| 0 | `$8BFF`-`$8BFF` | 1 | 0x00C0F | 5F |
| 0 | `$8C03`-`$8C03` | 1 | 0x00C13 | 70 |
| 0 | `$8C09`-`$8C09` | 1 | 0x00C19 | 1B |
| 0 | `$8C14`-`$8C14` | 1 | 0x00C24 | 70 |
| 0 | `$99E6`-`$99E8` | 3 | 0x019F6 | 4C 70 AA |
| 0 | `$9A22`-`$9A23` | 2 | 0x01A32 | 62 AB |
| 0 | `$9A2A`-`$9A2B` | 2 | 0x01A3A | 62 AB |
| 0 | `$A86D`-`$A86D` | 1 | 0x0287D | FF |
| 0 | `$AA40`-`$B081` | 1602 | 0x02A50 | (see the appendices) |
| 0 | `$BF70`-`$BFFF` | 144 | 0x03F80 | (see the appendices) |
| 1 | `$BF70`-`$BFFF` | 144 | 0x07F80 | (see the appendices) |
| 2 | `$BF70`-`$BFFF` | 144 | 0x0BF80 | (see the appendices) |
| 3 | `$9722`-`$9724` | 3 | 0x0D732 | 20 0C 58 |
| 3 | `$972A`-`$972C` | 3 | 0x0D73A | 20 0F 58 |
| 3 | `$BF70`-`$BFFF` | 144 | 0x0FF80 | (see the appendices) |
| 4 | `$BF70`-`$BFFF` | 144 | 0x13F80 | (see the appendices) |
| 5 | `$A716`-`$A716` | 1 | 0x16726 | FF |
| 5 | `$A72A`-`$A72A` | 1 | 0x1673A | FF |
| 5 | `$BF70`-`$BFFF` | 144 | 0x17F80 | (see the appendices) |
| 6 | `$ACF9`-`$B081` | 905 | 0x1AD09 | (see the appendices) |
| 6 | `$BF70`-`$BFFF` | 144 | 0x1BF80 | (see the appendices) |
| 7 | `$C0AF`-`$C0B1` | 3 | 0x1C0BF | 20 9A D3 |
| 7 | `$D39A`-`$D3C9` | 48 | 0x1D3AA | AD 00 50 C9 57 F0 03 20 AA FE A9 86 8D 08 50 A9 2D 8D 09 50 4C 00 58 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 |
| 7 | `$D4C5`-`$D4CD` | 9 | 0x1D4D5 | 20 03 58 4C CE D4 EA EA EA |
| 7 | `$D5CD`-`$D5CE` | 2 | 0x1D5DD | 06 58 |
| 7 | `$D5D2`-`$D5D3` | 2 | 0x1D5E2 | 12 58 |
| 7 | `$D625`-`$D62E` | 10 | 0x1D635 | 20 18 58 4C 2F D6 EA EA EA EA |
| 7 | `$DF31`-`$DF39` | 9 | 0x1DF41 | 20 1B 58 4C 3A DF EA EA EA |
| 7 | `$F1DB`-`$F1DE` | 4 | 0x1F1EB | 20 15 58 EA |
| 7 | `$F294`-`$F29F` | 12 | 0x1F2A4 | 20 09 58 F0 16 D0 05 EA EA EA EA EA |
| 7 | `$FEAA`-`$FECF` | 38 | 0x1FEBA | A9 57 8D 00 50 A9 49 8D 00 50 A9 44 8D 00 50 A9 45 8D 00 50 A9 00 8D 00 52 8D 01 52 8D 02 52 8D 03 52 8D 04 52 60 |
| 7 | `$FF4C`-`$FF6F` | 36 | 0x1FF5C | 20 C5 FF 20 40 AA A9 07 20 CC FF 4C 00 C0 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 |
| 7 | `$FF9B`-`$FF9C` | 2 | 0x1FFAB | 4C FF |

### F. `build-wide.sh`

```sh
#!/bin/sh
# Builds build/z2wide.nes from the sources and the CHR banks of a Zelda II (USA) ROM.
# Usage: ./build-wide.sh /path/to/Zelda2-USA.nes   (run under WSL or any shell with cc65)
set -e
ROM="$1"
cd "$(dirname "$0")"
mkdir -p build
for i in 0 1 2 3 4 5 6 7; do ca65 $CAFLAGS -I inc src/prg$i.asm -o build/prg$i.o; done
ld65 -C nes.cfg -o build/unused.bin build/prg0.o build/prg1.o build/prg2.o build/prg3.o \
    build/prg4.o build/prg5.o build/prg6.o build/prg7.o -m build/map.txt -Ln build/labels.txt
cat prg0.bin prg1.bin prg2.bin prg3.bin prg4.bin prg5.bin prg6.bin prg7.bin > build/prg.bin
rm -f prg?.bin build/unused.bin
head -c 16 "$ROM" > build/header.bin
tail -c +131089 "$ROM" > build/chr.bin
cat build/header.bin build/prg.bin build/chr.bin > build/z2wide.nes
echo "built build/z2wide.nes"
```

### G. `tools/make_bps.py`

```python
"""Writes a BPS patch from an original ROM to the widescreen build, both without their iNES header.

Usage: python -I tools/make_bps.py <original.nes> <build/z2wide.nes> <out.bps>

Only the bytes the hack changes travel in the patch (TargetRead); everything else is copied from
the player's own ROM (SourceRead), so the patch carries no data of the game.
"""

import sys
import zlib


def number(value):
    out = bytearray()
    while True:
        low = value & 0x7F
        value >>= 7
        if value == 0:
            out.append(0x80 | low)
            return bytes(out)
        out.append(low)
        value -= 1


def body(path):
    data = open(path, 'rb').read()
    return data[16:] if data[:4] == b'NES\x1a' else data


def make(source, target):
    patch = bytearray(b'BPS1')
    patch += number(len(source)) + number(len(target)) + number(0)
    position = 0
    while position < len(target):
        same = position < len(source) and source[position] == target[position]
        end = position
        while end < len(target) and (end < len(source) and source[end] == target[end]) == same:
            end += 1
        length = end - position
        if same:
            patch += number(((length - 1) << 2) | 0)
        else:
            patch += number(((length - 1) << 2) | 1) + target[position:end]
        position = end
    patch += zlib.crc32(source).to_bytes(4, 'little')
    patch += zlib.crc32(target).to_bytes(4, 'little')
    patch += zlib.crc32(bytes(patch)).to_bytes(4, 'little')
    return bytes(patch)


def main():
    source = body(sys.argv[1])
    target = body(sys.argv[2])
    patch = make(source, target)
    open(sys.argv[3], 'wb').write(patch)
    print(f'{sys.argv[3]}: {len(patch)} bytes, source crc {zlib.crc32(source):08X}, '
          f'target crc {zlib.crc32(target):08X}')


if __name__ == '__main__':
    main()
```
