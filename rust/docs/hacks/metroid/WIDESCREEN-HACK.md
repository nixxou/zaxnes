# Metroid (USA): the widescreen hack

This document describes completely the modification of *Metroid (USA)* that Zaxnes ships as a
built-in patch, the advanced mode of its widescreen. The appendices hold the new code and the
build script, which writes every hook after checking the bytes it replaces: with them the ROM can
be rebuilt identically.

The background of the margins is not the hack's work: the emulator's Metroid margin decoder shows
the world map's rooms around the screen, from the game's PRG-RAM and the area's data (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4); it widens the game on
its own, the simple mode. The hack does the enemies: they show, sleep, live and are drawn in the
margins, the way the Mega Man 2 and Blaster Master hacks do them (owner request: the game stays
the original's on the original screen).

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Metroid (USA)*, MMC1 (SNROM), 128 KiB PRG (8 banks of 16 KiB, bank 7 fixed at `$C000`), CHR-RAM, 8 KiB PRG-RAM |
| Source data (iNES header excluded) | CRC-32 `70080810` |
| Target data | CRC-32 `EC5FE12E` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | PRG grown to 256 KiB (16 banks): the game's banks 0-7 where they were, the hack's code as bank 8, banks 9-14 of `$FF`, the fixed bank copied to bank 15 (Zaxnes' MMC1 fixes bank 15 modulo the bank count); hooks in banks 0-5 and 7 (both copies of the fixed bank) |
| Patch | `rust/crates/zax-nes/patches/metroid-usa-widescreen.bps`, made against the data without header; the fixed bank's copy is SourceCopy, no game data in the patch |

The code runs from the widescreen extension's PRG window at `$5800-$5FFF`, which Zaxnes opens on
bank 8, page 0 at power-on for this patch (`boot_window`): the game has no free room for code that
would unlock the extension. The window stops at `$5FFF` (`$5007` bit 4 clear): the game keeps its
rooms and its world map in PRG-RAM at `$6000-$7FFF`.

---

**Hack pack.** Zaxnes ships this hack as hack packs, both from this source:
`rust/crates/zax-nes/patches/metroid-usa-widescreen.zxpk`, NTSC, and
`metroid-europe-widescreen.zxpk`, the same for PAL (Metroid (Europe), data CRC-32 `7751588D`, has
the same code and the area banks' tables at the same places) (`tools/build.py <rom> --pack
<out.zxpk>`, with `ZAXNES_TOOLS` set to Zaxnes' `rust/tools`; `docs/HACK-PACKS.md`,
`WIDESCREEN-EXTENSION.md` 7.1). The packs are format 2, written by Zaxnes' shared builder
(`hackpack.py`): MMC1, each hook a 16 KiB bank and a CPU address with the game's bytes it
replaces, checked and applied by Zaxnes; the ROM keeps its size (no bank added, nothing moved or
copied). `src/wide.s` is assembled with `-D PACK` for `$2000`: the zone fantôme, which Zaxnes
shows to instruction fetches only from power-on, the extension unlocked; its registers and RAM
are at `$3000`-`$3FFF` (widescreen MMC5: `$5000`-`$5FFF` stays the cartridge's), no window. The
game's data reads at `$2000`-`$3FFF` reach the PPU, so the code reads no data of its own: the sizes
of a room's entries are a test (`entry_size`, in the BPS build too). The widescreen is in groups:
"core" (Base: the settings, the rooms of the nametables, the position test and the sprites' high
bytes), "enemies", "doors" (the doors, their twins and the items), "previews" (the enemies of the
rooms ahead), and the two testing aids, "invincible" and "level select", which need the hack's
code; each but the core needs it. Checks of the game's code the hack relies on go with them: the
nametable a room is built on; an enemy's resting animation, its update without moving and where
its drawing goes on; an object's animation and drawing and where they go on; a game's start in an
area. Over a pool of 116 ROMs of the game (the USA, European, Virtual Console and PlayChoice-10
dumps, the French translations, the ROM hacks that are not combined with a translation), 111 take a
whole pack and 2 all but the doors; the others are ROM hacks that rework the game's code (Rogue
Dawn) or put it on MMC3 or MMC5. 3000 frames of each with scripted inputs play as without the
pack, but for four hard ROM hacks where the scripted Samus dies sooner (the enemies of the
margins reach her): with "Invincible" on they go as far. The BPS patch described here is kept to
compare.

## 2. What the game does, and what changes

The main loop (`$C0BC`) runs the frame's work, then waits for the NMI (`$C0C7`: `LDA #$00 /
STA $1A`, then `$C0CB-$C0D2` until the NMI sets `$1A`; Zaxnes' idle loop for this patch). A frame
whose work does not fit misses the next NMI's update: a lag frame.

Enemies: 16 slots of 16 bytes at `$0400` and `$6AF4` (the slot offset in `$4B`); offsets `$00-$50`
the room's enemies, which the room's data places when the game builds the room (`$EB4D`, status
1, resting), `$60` up what enemies fire and the like. `IsObjectVisible` (`$DFDF`) tells whether
an object (its y, x and nametable in `$0A`, `$0B`, `$06`, its x radius in `$09`) shows on the
screen, and leaves its screen x and y in `$0E` and `$10`. An enemy updates (`$F351`) only when it
showed at its last drawing (`$0405` bit 1, set by `DrawEnemy`, `$DDB0`) or the test says it shows
(`$F37F`); otherwise its routine is skipped, and it is not drawn. Samus, her shots, the doors and
the other objects are drawn by `ObjDrawFrame` (`$DE4A`), which keeps the test's answer in their
`onScreen` byte; a shot leaving the screen is removed.

| Game routine | Original | Hack |
|---|---|---|
| `IsObjectVisible` (`$DFDF`) | X = 1 when the object shows | the same test, to the letter; also whether it shows within the margins (scrolling across: its x within `[r - M, 256 + M - r)`, `r` its x radius, of the 16-bit screen x below; up and down: the screen's answer) and its 16-bit screen x: on the nametable of the screen's left edge (`$FF` bit 0), on the screen at or right of the scroll `$FD`, in the left margin left of it; on the other one, by the room it holds (below): the room right of the screen's left one, on the screen left of the scroll and in the right margin right of it, the room left of it (the one Samus came from, the scroll at 0) in the left margin right of the scroll, any other room nowhere (but where the game's test shows it) |
| `DeleteOffscreenRoomSprites` (`$ECB4`), a room built on a nametable | removes that nametable's enemies but those that showed (`$0405` bit 1) | removes them but those on the original screen at their last drawing (`$5302 + slot`): awake in a margin, an enemy updates and its bit 1 is set, but its room is going; kept, it would live on in the new room's place, with that room's walls |
| `SelectRoomRAM` (`$EA05`) | the nametable a room is built on | the same, and the room's map position (`$50`, `$4F`) kept for that nametable |
| `DrawEnemy` (`$DE1B`) | draws the enemy when it shows; `$0405` bit 1 = it shows | draws it when it shows within the margins; bit 1 = it updates (below) |
| `UpdateEnemy_CheckIfVisible` (`$F39F`) | skips the enemy's routine when it does not show | skips it when it does not update; asleep in a margin, it is drawn in its resting pose (`$F682`, then `$F438`: animation and drawing, no movement) |
| an enemy placed by its room (`$EB5E`) | status 1 (resting) | the same, and asleep (`$5300 + slot`, with its type at `$5301 + slot`: a slot holding another type since is awake) |
| a sprite's x (`$DF0E`) | 8-bit | the same, and its high byte: the 16-bit x nearest the object's whose low byte is the sprite's; 0 for sprites no position test placed (the energy display). Kept in a buffer (`$5400 + OAM offset`, cleared at the world's update) and copied to `$5100` at its end: a frame the game runs late copies the sprite page as it is, with the last finished frame's high bytes |
| end of the world's update (`$CB66`) | clears the unused sprites | first the previews: the enemies of the rooms the margins show that no nametable holds (across: the rooms left of the screen's and right of it; in a shaft, the corridors on either side), read from the rooms' data as `$EB4D` places them (none whose slot holds an enemy that shows: the game skips it then, `IsSlotTaken`, `$EB7A`), drawn still in their resting pose by the game's own drawing in enemy slot `$50` (set aside meanwhile), within the margins, while sprites are left (OAM offset below `$C0`); a room a nametable holds is previewed until the game has placed its enemies (`$5A` below `$F0`) and the frame it does; Kraid and Ridley are not; then the frame's high bytes to the extension |
| `ObjDrawFrame` (`$DED2`) | draws the object when it shows; its `onScreen` byte = it shows | `onScreen` the same; the doors (objects `$80-$B0`) and the item (`$40`) are drawn when they show within the margins |
| `DrawDoor` (each area bank, `$8BCC`) | draws the door (`AnimDrawObject`) | the same, then its twin, the facing door of the room across (in a shaft, the corridors on either side, which the game does not build): mirrored, 32 pixels further out (x `$10` <-> `$F0` of the room across), in the door's animation frame, within the margins (scrolling up and down, only while its door shows on the screen: the same lines); not when the other nametable holds the room across and the game keeps its facing door |
| end of an object's drawing (`$DF2D`) | clears `$6B` | the same, and the object's 16-bit x is no longer valid |
| world's update (`$CB29`), main loop's wait (`$C0C7`) | | the high bytes cleared (every frame out of play), the extension's settings |
| `SubtractHealth` (`$CE92`) | | no damage when the option "Invincible" is on (the damage is dropped) |
| a new game (bank 0 `$9354`, the end of `InitializeStats`) | Brinstar's bank | with the option "Level select", a direction held at START picks the area (up Norfair, right Kraid, left Tourian, down Ridley): `$74` set, then the game starts there as a password starts one (`$930D`) |

An enemy updates when it shows on the original screen (it then wakes), or within the margins
awake: what enemies fire (offsets `$60` up) is always awake. Asleep in a margin, a room's enemy
shows but its routine does not run: it neither moves nor attacks until it reaches the original
screen; awake, it stays awake. The doors and the item are drawn within the margins too; the game
keeps them to its own test (it removes a door its test does not show when it builds a room on
that door's nametable). Samus, her shots and the other objects keep the original test: a shot
leaving the original screen is removed as in the original.

Only the two rooms the game keeps in PRG-RAM have enemies: the margins may show other rooms (a
neighbour the screen does not reach yet, a room behind a door), drawn by the decoder, empty.

Margin: `M = min($5002, 128)` for the enemies (the two nametables, 512 pixels, are the screen and
two margins of 128; the decoder is asked for `min($5002, 248)`). With `M = 0` every routine does
exactly what the code it replaces does.

CPU time: 64 lines of overclock (`$5006`) pay for the hack's work and the enemies of the margins;
255 with the option "No slowdown".

## 3. The extension

Set every frame at the main loop's wait (`$C0C7`, `LDA #$00 / STA $1A` becomes `JSR wide_frame /
NOP`, which writes `$5003 = min($5002, 248)`, `$5004 = $5005 = 0` (the nametable columns are
hidden: the decoder draws the background), `$5007 = $0A` (sprites free of the extents, their high bytes kept from one OAM DMA to the next: the NMI copies the sprite page every frame, also one the game has not built again, whose high bytes must stay), `$5006 = 64`
(255 with "No slowdown"), clears the sprite high bytes out of play, then `$1A`).

Variables (extension RAM): `WV_M $5200`, `WV_ORIG $5201` and `WV_WIDE $5202` (the last position
test: on the original screen, within the margins), `WV_VALID $5203` / `WV_BASEL`, `WV_BASEH
$5204/$5205` (the 16-bit screen x of the object being drawn), `WV_ASLEEP $5206` (the last enemy
test), `WV_NT_X $5210-$5211` and `WV_NT_Y $5212-$5213` (by nametable: the map position of the room the game built there last), `WV_T0`, `WV_T1 $5214/$5215`, `WV_TWIN $5216` (a door's twin being drawn), `WV_TWIN_SIDE $5217`, `WV_CNTRL $5218` (the door's `$6B`), `WV_PREVIEW $5219` (a preview being drawn), `WV_PREV_XL/XH/Y $521A-$521C` (its place), `WV_CX/CY $521D/$521E` (the room previewed), `WV_OXL/OXH/OYL/OYH $5220-$5223` (its corner on the screen), `WV_PTRL/H $5224/$5225`, `WV_PTYPE/PPOS $5226/$5227` (the entry read), `WV_FRESH $5228` (1 + the nametable built last), `WV_SPAWNED $5229`, `WV_PSAVE $5230-$524F` (slot `$50` set aside), `WV_HI_BUF $5400 + OAM offset`, `WV_SLEEP $5300 + slot offset`, `WV_SEEN $5302 + slot offset` (on the original screen at its last drawing), `WV_STYPE $5301 + slot offset`.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length. Those of
the fixed bank (`$C000-$FFFF`) are written into both its copies (7 and 15).

| Address | Replaces | With |
|---|---|---|
| `$C0C7` | `LDA #$00 / STA $1A` | `JSR wide_frame / NOP` |
| `$CB29` | `LDX #$00 / STX $5B` | `JSR wide_world / NOP` |
| `$DFDF` | `LDX #$01 / LDA` (IsObjectVisible) | `JMP wide_visible` |
| `$EA05` | `JSR $EB85` (SelectRoomRAM) | `JSR wide_select_room` |
| `$ECB4` | `LDA $0405,X / AND #$02` (DeleteOffscreenRoomSprites) | `JSR wide_room_retire / NOP / NOP` |
| `$DE1B` | `JSR $DFDF` (DrawEnemy) | `JMP wide_enemy_draw` |
| `$DED2` | `JSR $DFDF` (ObjDrawFrame) | `JMP wide_object_draw` |
| `$F39F` | `JSR $DFDF` (UpdateEnemy_CheckIfVisible) | `JSR wide_enemy_check` |
| `$EB5E` | `LDA #$01 / STA $6AF4,X` | `JSR wide_spawn / NOP / NOP` |
| `$DF0E` | `ADC $0E / STA $0203,X` | `JSR wide_sprite_x / NOP / NOP` |
| `$DF2D` | `LDA #$00 / STA $6B` | `JSR wide_drawn / NOP` |
| `$CE92` | `LDA $6E / ORA $6F` | `JSR wide_hurt / NOP` |
| `$CB66` | `LDX $5B / LDA #$F4` (UpdateWorld, the unused sprites) | `JSR wide_previews / NOP` |
| banks 1-5 `$8BCC` | `LDA #$06 / JMP $DE47` (DrawDoor) | `JMP wide_draw_door / NOP / NOP` |
| bank 0 `$9354` | `LDA #$02 / STA $24` | `JSR wide_new_game / NOP` |

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there (`WIDE_OPTIONS`).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (hold a direction at START: ...) / Sélecteur de niveau (...) | off | a testing aid: a new game starts in the area a direction held at START picks: up Norfair, right Kraid, left Tourian, down Ridley (none: Brinstar) |
| 1 | `invincible` | Invincible | off | a testing aid: Samus takes no damage (her knockback stays) |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of 64 |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `PATCHES`); the ROM does
not depend on them being set.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Metroid (USA).nes" build/metroidwide.bps
```

assembles `src/wide.s` for `$5800` (one 16 KiB bank, `-g`, labels from `ld65 -Ln`; the code must
end before `$6000`), lays the PRG out in 16 banks, writes the hooks, `build/metroidwide.nes` and
the BPS patch (`tools/make_bps.py`: the fixed bank's copy is SourceCopy from the player's ROM, the
hooks and the code TargetRead). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.

## 7. Limits

* No sprite in the vertical margins: the extension carries a high byte for x only. Scrolling up
  and down, the enemies keep the original screen.
* Only the two rooms the game keeps in PRG-RAM have living enemies; those of the other rooms of
  the margins are previews, still, from their rooms' data (what spawns otherwise, the pipes' bugs,
  has none). Rooms of another area the margins could touch would preview garbage (the decoder
  hides them; the hack does not tell the areas apart). The elevators, the statues and the other objects keep the original screen.
* A room of the margins past a palette change room shows in the area's other palette (the
  decoder's own margin palette); the game's other palette changes (the item rooms' music, the
  elevators) do not apply to the margins.
* The sprites of the margins share the console's 64 sprites with the screen's: more flicker when
  many enemies show.
* The overclock removes lag frames, so a replay of the original's inputs frame for frame drifts
  from the original.

---

## Appendices

### A. `src/wide.s`

```asm
; Metroid (USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added to the game (PRG bank 8 of 16: the game's banks 0-7 stay
; where they were and its fixed bank is copied to bank 15, the one MMC1 fixes at $C000); the
; widescreen extension's PRG window shows its first 2 KiB at $5800-$5FFF, Zaxnes opening it at
; power-on (the patch's boot window: bank 8, page 0). The window stops at $5FFF: the game keeps
; its rooms and its map in PRG-RAM at $6000-$7FFF. The game reaches the code through hooks in its
; own (tools/build.py). The background of the margins is Zaxnes' margin decoder's work; the hack
; does the enemies, the way the Mega Man 2 and Blaster Master hacks do them:
;
;  * an enemy shows within the margins instead of the original screen (scrolling across: up and
;    down, the screen's width is the room's);
;  * one a room placed (slots 0-5, $EB4D) sleeps in a margin: it shows, drawn and animated in its
;    resting pose, but its routine does not run until it reaches the original screen; awake, it
;    lives within the margins; what enemies fire lives there too;
;  * the doors and the items show within the margins too (the game keeps them to its tests of
;    the original screen), and each door's twin, the facing door of the room across;
;  * the enemies of the rooms the margins show that the game has not built are drawn still as
;    previews, from the rooms' data, until the game builds the room;
;  * every sprite of an object gets the high byte of its 16-bit x, the room each nametable holds
;    telling which margin an object of the other nametable shows in;
;  * Samus, her shots and the other objects keep the original screen;
;  * the option "Level select" starts a new game in the area a direction held picks.
;
; In a Zaxnes hack pack (-D PACK) the code is assembled for $2000 instead: the zone fantôme, which
; Zaxnes shows to instruction fetches only from power-on, the extension unlocked; the game's data
; reads there still reach the PPU, so the code reads no data of its own (the sizes of a room's
; entries are a test). Its registers and RAM are at $3000-$3FFF, which the zone holds for its own
; instructions (widescreen MMC5: $5000-$5FFF stays the cartridge's), and no window is opened.
; Metroid (Europe) has the same code at the same places.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

.ifdef PACK
EXT             = $3000         ; the extension's registers and RAM, in the zone fantôme
.else
EXT             = $5000
.endif

WIDE_HOST       = EXT + $002    ; R: the margin the host offers
WIDE_REQ        = EXT + $003    ; W: the margin asked for
WIDE_LEFT       = EXT + $004    ; nametable columns shown left of the screen
WIDE_RIGHT      = EXT + $005    ; and right of it
WIDE_OVERCLOCK  = EXT + $006    ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = EXT + $007    ; bit 1 sprites free of the extents, bit 3 high bytes kept
WIDE_OPTIONS    = EXT + $00A    ; R: the options the player turned on
WIDE_SPRHI      = EXT + $100    ; + 4 n: high byte of the x of sprite n, for the next OAM DMA

OPT_LEVEL_SELECT = $01         ; a testing aid: a new game in the area a direction held picks
OPT_INVINCIBLE  = $02           ; a testing aid: Samus takes no damage
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late

MAX_MARGIN      = 248           ; the most the decoder is asked for
MAX_SPRITE_MARGIN = 128         ; the most enemies show in: the two nametables, 512 pixels, are
                                ; the screen and two margins of 128
OVERCLOCK_LINES = 64            ; pays for the hack's work and the enemies of the margins
NO_SLOWDOWN_LINES = 255
PROJECTILES     = $60           ; enemy slot offsets: 0-$50 the room's enemies, $60 up what
                                ; enemies fire and the like

; Variables in the extension's RAM (EXT + $200 to EXT + $7FF).
WV_M            = EXT + $200    ; the margin enemies show in, this frame
WV_ORIG         = EXT + $201    ; the last position test: 1 on the original screen, else 0
WV_WIDE         = EXT + $202    ; and 1 within the margins, else 0
WV_VALID        = EXT + $203    ; non-zero while WV_BASE is the object being drawn
WV_BASEL        = EXT + $204    ; the 16-bit screen x of that object ($0E and its high byte)
WV_BASEH        = EXT + $205
WV_ASLEEP       = EXT + $206    ; the last enemy test: 1 when it sleeps in a margin
WV_NT_X         = EXT + $210    ; + nametable (0, 1): the map x and y of the room the game built
WV_NT_Y         = EXT + $212    ; there last
WV_T0           = EXT + $214    ; scratch
WV_T1           = EXT + $215
WV_TWIN         = EXT + $216    ; non-zero while a door's twin is drawn
WV_TWIN_SIDE    = EXT + $217    ; its side: 1 right, $FF left
WV_CNTRL        = EXT + $218    ; the door's $6B
WV_PREVIEW      = EXT + $219    ; non-zero while a preview is drawn
WV_PREV_XL      = EXT + $21A    ; its 16-bit screen x and its screen y
WV_PREV_XH      = EXT + $21B
WV_PREV_Y       = EXT + $21C
WV_CX           = EXT + $21D    ; the map room previewed
WV_CY           = EXT + $21E
WV_OXL          = EXT + $220    ; its top left corner on the screen, 16-bit
WV_OXH          = EXT + $221
WV_OYL          = EXT + $222
WV_OYH          = EXT + $223
WV_PTRL         = EXT + $224    ; where its data is read
WV_PTRH         = EXT + $225
WV_PTYPE        = EXT + $226    ; the enemy entry read: type, position
WV_PPOS         = EXT + $227
WV_FRESH        = EXT + $228    ; 1 + the nametable the game built a room on last
WV_SPAWNED      = EXT + $229    ; non-zero when the game placed a room's enemy this frame
WV_HI_BUF       = EXT + $400    ; + 4 n: the high byte of sprite n, as the frame writes them
WV_PSAVE        = EXT + $230    ; slot $50 set aside (32 bytes)
WV_SLEEP        = EXT + $300    ; + enemy slot offset: 1 asleep (placed by its room, not yet on
                                ; the original screen)
WV_STYPE        = EXT + $301    ; + enemy slot offset: the type it was placed as
WV_SEEN         = EXT + $302    ; + enemy slot offset: 1 when it showed on the original screen at
                                ; its last drawing

; The game.
NMI_STATUS      = $1A           ; 0 while the main loop waits for the NMI
GAME_MODE       = $1D           ; 0: in play
TEMP_RADIUS_X   = $09           ; the object at hand: x radius, hi (nametable), y, x
TEMP_HI         = $06
TEMP_Y          = $0A
TEMP_X          = $0B
SCREEN_X        = $0E           ; its screen x and y, 8-bit
SCREEN_Y        = $10
TEMP_08         = $08
PAGE_INDEX      = $4B           ; also the object at hand (16 per object, $0300)           ; the enemy slot offset at hand (16 per slot)
SCROLL_DIR      = $49           ; bit 1: scrolling across
SCROLLING_RIGHT = 3
MAP_Y           = $4F           ; the room the screen scrolls into: the second of the two on the
MAP_X           = $50           ; screen scrolling right or down, the first scrolling left or up
SPRITE_PAGE_POS = $5B
HEALTH_CHANGE   = $6E           ; 2 bytes: the damage to take
OBJECT_CNTRL    = $6B
SCROLL_Y        = $FC
SCROLL_X        = $FD
PPUCTRL_ZP      = $FF           ; bit 0: the nametable of the screen's left (top) edge
OBJ_STATUS      = $0300         ; + object offset: 0 none
OBJ_HI          = $030C         ; + object offset: its nametable, y and x
OBJ_X           = $030E
TWIN_DISTANCE   = 32            ; a door's twin: from x $10 to $F0 of the room across
OBJ_ON_SCREEN   = $030B         ; + object offset: it shows (the game's test)
POWER_UP_DRAW   = $40           ; object offsets: the item being drawn, the doors
FIRST_DOOR      = $80
LAST_DOOR       = $B0
JOY1_STATUS     = $14           ; the buttons held
BANK_SWITCH     = $24           ; the PRG bank to switch to, plus one
IN_AREA         = $74
EN_Y            = $0400         ; + slot offset: the enemy's 16 bytes from its y
EN_IS_HIT       = $0404
EN_SPECIAL      = $040F         ; bit 7: the tough version
EN_STATUS       = $6AF4         ; + slot offset: its other 16 bytes from its status
EN_RESET_ANIM   = $6AF9
PREVIEW_SLOT    = $50           ; the enemy slot a preview borrows
LAST_PREVIEW_SPRITE = $C0       ; previews stop when the sprites reach this OAM offset
ROOM_PTR_TABLE  = $3B           ; the area's room pointers
WORLD_MAP       = $7000
MAP_SIZE        = 32
ROOM_HEIGHT     = 240
SCROLLING_DOWN  = 1
ROOM_NUMBER     = $5A           ; the room the game builds; $F0 up: its objects done
ROOM_DONE       = $F0
TEMP_00         = $00
EN_DATA05       = $0405         ; + slot offset: bit 1 the enemy updates (it showed)
EN_TYPE         = $6B02         ; + slot offset
OAM_X           = $0203

; The game's routines.
NAMETABLE_AT_SCROLL = $EB85     ; GetNameTableAtScrollDir: the nametable a room is built on
EN_RESTING_ANIM = $F682         ; InitEnRestingAnimIndex
EN_DRAW_NO_MOVE = $F438         ; UpdateEnemyCommon@noMove: animation and drawing only
EN_DRAW_GO      = $DEDE         ; DrawEnemy_NotBlank, drawing the enemy
EN_DRAW_SKIP    = $DE53         ; DrawEnemy_NotBlank, not drawing it
ANIM_DRAW_OBJECT = $DE47        ; AnimDrawObject: animation, then ObjDrawFrame
OBJ_DRAW_FRAME  = $DE4A
OBJ_DRAW_GO     = $DEDE         ; ObjDrawFrame, drawing the object
OBJ_DRAW_SKIP   = $DEE3         ; ObjDrawFrame, not drawing it
AREA_START      = $930D         ; bank 0 (the title's): a game started in the area $74, as a
                                ; password starts one

.segment "CODE"
.ifdef PACK
.org $2000
.else
.org $5800
.endif

; ---------------------------------------------------------------------------- ;
; The main loop's wait for the NMI ($C0C7, in place of LDA #$00 / STA $1A, the game's work for
; the frame done): the extension's settings for the next frame.
wide_frame:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @request
    LDA     #MAX_MARGIN
@request:
    STA     WIDE_REQ
    CMP     #MAX_SPRITE_MARGIN
    BCC     @sprites
    LDA     #MAX_SPRITE_MARGIN
@sprites:
    STA     WV_M
    LDA     #$00
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    ; Sprites free of the extents, their high bytes kept from one OAM DMA to the next: the NMI
    ; copies the sprite page every frame, also one the game has not built again (its work for
    ; the frame not done), whose high bytes must stay.
    LDA     #$0A
    STA     WIDE_LAYOUT
    LDA     WIDE_OPTIONS
    AND     #OPT_NO_SLOWDOWN
    BEQ     @normal
    LDA     #NO_SLOWDOWN_LINES
    BNE     @overclock
@normal:
    LDA     #OVERCLOCK_LINES
@overclock:
    STA     WIDE_OVERCLOCK
    LDA     GAME_MODE                   ; out of play the sprites have no high bytes
    BEQ     @done
    JSR     clear_high_bytes
@done:
    LDA     #$00
    STA     NMI_STATUS
    RTS

; The start of the world's update ($CB29, in place of LDX #$00 / STX $5B): the sprites are built
; again, from no high bytes.
wide_world:
    LDA     #$00                        ; no enemy placed yet this frame
    STA     WV_SPAWNED
    TAX                                 ; the frame's high bytes from 0 (the sprites written
@clear:                                 ; otherwise, the energy display, have none)
    STA     WV_HI_BUF,X
    INX
    BNE     @clear
    LDX     #$00
    STX     SPRITE_PAGE_POS
    RTS

clear_high_bytes:
    LDA     #$00
    TAX
@loop:
    STA     WIDE_SPRHI,X
    INX
    BNE     @loop
    RTS

; ---------------------------------------------------------------------------- ;
; An object's place on the screen (IsObjectVisible, $DFDF, the whole routine): X = 1 when it
; shows on the original screen, else 0, as the game's test (kept to the letter, $0E and $10 the
; screen x and y). Also works out whether it shows within the margins (WV_WIDE) and its 16-bit
; screen x for its sprites (WV_BASE).
wide_visible:
    LDA     WV_PREVIEW
    BEQ     @game
    JMP     preview_place
@game:
    LDX     #$01
    LDA     TEMP_Y
    TAY
    SEC
    SBC     SCROLL_Y
    STA     SCREEN_Y
    LDA     TEMP_X
    SEC
    SBC     SCROLL_X
    STA     SCREEN_X
    PHP                                 ; carry: x >= scroll
    LDA     SCROLL_DIR
    AND     #$02
    BNE     @across
    PLP
    CPY     SCROLL_Y                    ; up and down
    LDA     TEMP_HI
    EOR     PPUCTRL_ZP
    AND     #$01
    BEQ     @v_same
    BCS     @v_hidden
    LDA     SCREEN_Y
    SBC     #$0F
    STA     SCREEN_Y
    LDA     TEMP_RADIUS_X
    CLC
    ADC     SCREEN_Y
    CMP     #$F0
    BCC     @v_shown
    CLC
@v_same:
    BCC     @v_hidden
    LDA     TEMP_RADIUS_X
    CMP     SCREEN_Y
    BCC     @v_shown
@v_hidden:
    DEX
@v_shown:
    STX     WV_ORIG                     ; the margins are the screen's
    STX     WV_WIDE
    LDA     #$00
    JMP     @base

@across:
    PLP
    LDA     TEMP_HI
    EOR     PPUCTRL_ZP
    AND     #$01
    BEQ     @h_same
    BCS     @h_hidden
    LDA     TEMP_RADIUS_X
    CLC
    ADC     SCREEN_X
    BCC     @h_shown
    CLC
@h_same:
    BCC     @h_hidden
    LDA     TEMP_RADIUS_X
    CMP     SCREEN_X
    BCC     @h_shown
@h_hidden:
    DEX
@h_shown:
    STX     WV_ORIG
    ; The 16-bit screen x. On the screen's left nametable: on the screen at or right of the
    ; scroll, in the left margin left of it. On the other one, by the room it holds (the hack
    ; keeps where each nametable's room is on the map, wide_select_room): the room right of the
    ; screen's left one, on the screen left of the scroll and in the right margin right of it;
    ; the room left of it (the one Samus came from, the scroll at 0), in the left margin right of
    ; the scroll; any other room, nowhere (but where the game's test shows it).
    LDA     TEMP_X
    CMP     SCROLL_X                    ; carry: x >= scroll
    LDA     TEMP_HI
    EOR     PPUCTRL_ZP
    AND     #$01
    BEQ     @same
    PHP
    JSR     other_side
    BEQ     @neither
    BMI     @other_left
    PLP
    BCC     @inside
    BCS     @right
@other_left:
    PLP
    BCS     @left
    BCC     @far_left
@neither:
    PLP
    LDA     WV_ORIG
    BNE     @inside
    BEQ     @far
@same:
    BCS     @inside
    BCC     @left
@right:
    ; Right: 256 + x, shown when x + radius < M.
    LDA     SCREEN_X
    CLC
    ADC     TEMP_RADIUS_X
    BCS     @far
    CMP     WV_M
    BCS     @far
    LDA     #$01
    STA     WV_WIDE
    BNE     @base                       ; high byte 1
@left:
    ; Left: x - 256, shown when x - radius >= 256 - M.
    LDA     SCREEN_X
    SEC
    SBC     TEMP_RADIUS_X
    BCC     @far_left
    TAY
    LDA     WV_M
    BEQ     @far_left
    EOR     #$FF                        ; 256 - M = -M
    CLC
    ADC     #$01
    STA     WV_BASEH                    ; scratch
    CPY     WV_BASEH
    BCC     @far_left
    LDA     #$01
    STA     WV_WIDE
    LDA     #$FF
    BNE     @base
@far_left:
    LDA     #$00
    STA     WV_WIDE
    LDA     #$FF
    BNE     @base
@far:
    LDA     #$00
    STA     WV_WIDE
    LDA     #$01
    BNE     @base
@inside:
    LDA     #$01
    STA     WV_WIDE
    LDA     #$00
@base:
    STA     WV_BASEH
    LDA     SCREEN_X
    STA     WV_BASEL
    LDA     #$01
    STA     WV_VALID
    LDA     WV_TWIN
    BEQ     @done
    JSR     twin_place
@done:
    LDX     WV_ORIG
    RTS

; A door's twin (WV_TWIN): the facing door of the room across, 32 pixels further out (the left
; door at x $10, its twin at $F0 of the room on the left, and the other way round). Never on the
; original screen for the game; shown within the margins.
twin_place:
    ; Up and down, the twin shows where its door does (the game's test): the same lines.
    LDA     SCROLL_DIR
    AND     #$02
    BNE     @across
    LDA     WV_ORIG
    BNE     @across
    STA     WV_WIDE
    RTS
@across:
    LDA     #$00
    STA     WV_ORIG
    LDA     WV_TWIN_SIDE
    BMI     @left
    LDA     WV_BASEL
    CLC
    ADC     #TWIN_DISTANCE
    STA     WV_BASEL
    LDA     WV_BASEH
    ADC     #$00
    JMP     @placed
@left:
    LDA     WV_BASEL
    SEC
    SBC     #TWIN_DISTANCE
    STA     WV_BASEL
    LDA     WV_BASEH
    SBC     #$00
@placed:
    STA     WV_BASEH
    LDY     WV_BASEL                    ; the sprites' x from there
    STY     SCREEN_X
    ; Shown when its x is within the margins: high byte 0, 1 below M, or $FF from 256 - M.
    LDY     #$00
    CMP     #$00
    BEQ     @shown
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     WV_M
    BEQ     @hidden
    EOR     #$FF
    CLC
    ADC     #$01
    CMP     WV_BASEL
    BEQ     @shown
    BCC     @shown
    BCS     @hidden
@right:
    LDA     WV_BASEL
    CMP     WV_M
    BCS     @hidden
@shown:
    INY
@hidden:
    STY     WV_WIDE
    RTS

; Which neighbour of the screen's left room the other nametable's (the object at hand's, $06)
; holds: A = 1 the right one, $FF the left one, 0 neither (Z and N set accordingly).
other_side:
    LDA     TEMP_HI
    AND     #$01
    TAY
    LDA     WV_NT_Y,Y
    CMP     MAP_Y
    BNE     @neither
    LDA     MAP_X                       ; the screen's left room: MapPosX, but the second of
    STA     WV_T0                       ; the two scrolling right
    LDA     SCROLL_DIR
    CMP     #SCROLLING_RIGHT
    BNE     @left_room
    LDA     SCROLL_X
    BEQ     @left_room
    DEC     WV_T0
@left_room:
    LDA     WV_NT_X,Y
    SEC
    SBC     WV_T0
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BEQ     @left
@neither:
    LDA     #$00
    RTS
@right:
    LDA     #$01
    RTS
@left:
    LDA     #$FF
    RTS

; A room being built (SelectRoomRAM, $EA05, in place of JSR GetNameTableAtScrollDir: A = the
; nametable it goes on): the hack keeps the room's map position for that nametable.
wide_select_room:
    JSR     NAMETABLE_AT_SCROLL
    PHA
    TAY
    INY                                 ; the room built last: its enemies come at the end
    STY     WV_FRESH                    ; of its objects
    DEY
    LDA     MAP_X
    STA     WV_NT_X,Y
    LDA     MAP_Y
    STA     WV_NT_Y,Y
    PLA
    RTS

; Whether the enemy at hand ($4B) updates: A = 1 when it shows on the original screen, or within
; the margins awake (or not one its room placed), else 0; WV_ASLEEP = 1 when it shows in a margin
; asleep. On the original screen it wakes.
wide_awake:
    LDA     #$00
    STA     WV_ASLEEP
    LDY     PAGE_INDEX
    LDA     WV_ORIG
    BEQ     @not_orig
    LDA     #$00
    STA     WV_SLEEP,Y
    LDA     #$01
    RTS
@not_orig:
    LDA     WV_WIDE
    BEQ     @done
    CPY     #PROJECTILES
    BCS     @awake
    LDA     WV_SLEEP,Y
    BEQ     @awake
    LDA     WV_STYPE,Y                  ; the slot holds another enemy now: awake
    CMP     EN_TYPE,Y
    BNE     @awake
    LDA     #$01
    STA     WV_ASLEEP
    LDA     #$00
    RTS
@awake:
    LDA     #$01
@done:
    RTS

; ---------------------------------------------------------------------------- ;
; An enemy's drawing (DrawEnemy_NotBlank, $DE1B, in place of JSR IsObjectVisible, then the
; game's own setting of the enemy's update bit and its choice to draw): it is drawn within the
; margins; its update bit (EN_DATA05 bit 1) set when it updates.
wide_enemy_draw:
    JSR     wide_visible
    LDA     WV_PREVIEW                  ; a preview: drawn, nothing of the game's touched
    BNE     @go
    JSR     wide_awake
    ASL
    STA     TEMP_08
    LDX     PAGE_INDEX
    LDA     WV_ORIG
    STA     WV_SEEN,X
    LDA     EN_DATA05,X
    AND     #$FD
    ORA     TEMP_08
    STA     EN_DATA05,X
    LDA     WV_WIDE
    BEQ     @skip
@go:
    JMP     EN_DRAW_GO
@skip:
    JMP     EN_DRAW_SKIP

; An enemy's update test (UpdateEnemy_CheckIfVisible, $F39F, in place of JSR IsObjectVisible):
; X = 1 when it updates; asleep in a margin, it is only drawn in its resting pose (X = 0: the
; game skips its routine).
wide_enemy_check:
    JSR     wide_visible
    JSR     wide_awake
    BNE     @updates
    LDA     WV_ASLEEP
    BEQ     @hidden
    LDX     PAGE_INDEX
    JSR     EN_RESTING_ANIM
    LDX     PAGE_INDEX
    JSR     EN_DRAW_NO_MOVE
@hidden:
    LDX     PAGE_INDEX
    LDA     #$00
    STA     WV_SEEN,X
    LDX     #$00
    RTS
@updates:
    LDX     #$01
    RTS

; An object's drawing (ObjDrawFrame, $DED2, in place of JSR IsObjectVisible, then the game's own
; setting of the object's onScreen byte and its choice to draw): onScreen keeps the game's test;
; the doors and the item are drawn within the margins too.
wide_object_draw:
    JSR     wide_visible
    TXA
    LDX     PAGE_INDEX
    LDY     WV_TWIN                     ; a door's twin is not the game's: its door's onScreen
    BNE     @twin                       ; stays
    STA     OBJ_ON_SCREEN,X
    TAY                                 ; Z: the game's test
    BNE     @draw
@twin:
    LDA     WV_WIDE
    BEQ     @skip
    CPX     #POWER_UP_DRAW
    BEQ     @draw
    CPX     #FIRST_DOOR
    BCC     @skip
    CPX     #LAST_DOOR+1
    BCS     @skip
@draw:
    JMP     OBJ_DRAW_GO
@skip:
    JMP     OBJ_DRAW_SKIP

; A door's drawing (DrawDoor, in each area bank at $8BCC, in place of LDA #$06 / JMP
; AnimDrawObject): the door, then its twin, the facing door of the room across (that room is not
; built: in a shaft, the corridors on either side), mirrored, in the door's animation frame;
; unless the room across is the one the other nametable holds, whose own door the game draws.
wide_draw_door:
    LDA     OBJECT_CNTRL
    STA     WV_CNTRL
    LDA     #$06                        ; the door's animation, every 6 frames
    JSR     ANIM_DRAW_OBJECT
    LDX     PAGE_INDEX
    LDY     #$FF                        ; the side: left for the door at x $10
    LDA     OBJ_X,X
    BPL     @side
    LDY     #$01
@side:
    STY     WV_TWIN_SIDE
    LDA     OBJ_HI,X
    AND     #$01
    TAY
    LDA     WV_NT_X,Y                   ; the room across
    CLC
    ADC     WV_TWIN_SIDE
    STA     WV_T0
    LDA     WV_NT_Y,Y
    STA     WV_T1
    TYA
    EOR     #$01
    TAY
    LDA     WV_NT_X,Y
    CMP     WV_T0
    BNE     @twin
    LDA     WV_NT_Y,Y
    CMP     WV_T1
    BNE     @twin
    ; The other nametable holds the room across: its facing door, if the game keeps it.
    STY     WV_T1                       ; the other nametable
    LDA     OBJ_X,X
    EOR     #$E0                        ; $10 <-> $F0
    STA     WV_T0
    LDY     #FIRST_DOOR
@door:
    LDA     OBJ_STATUS,Y
    BEQ     @next
    LDA     OBJ_HI,Y
    AND     #$01
    CMP     WV_T1
    BNE     @next
    LDA     OBJ_X,Y
    CMP     WV_T0
    BEQ     @done
@next:
    TYA
    CLC
    ADC     #$10
    TAY
    CPY     #LAST_DOOR+$10
    BNE     @door
@twin:
    LDA     WV_CNTRL
    EOR     #$10                        ; mirrored
    STA     OBJECT_CNTRL
    LDA     #$01
    STA     WV_TWIN
    JSR     OBJ_DRAW_FRAME
    LDA     #$00
    STA     WV_TWIN
@done:
    RTS

; ---------------------------------------------------------------------------- ;
; Previews (the end of the world's update, $CB66, in place of LDX $5B / LDA #$F4: the unused
; sprites cleared next): the enemies of the rooms the margins show that the game has not built
; (the room the screen scrolls into, before the game builds it; in a shaft, the corridors on
; either side), read from the rooms' data as the game places them ($EB4D) and drawn still in
; their resting pose by the game's own drawing, in enemy slot $50 borrowed meanwhile. When the
; game builds the room, its enemies take over where the previews were (the room previewed until
; the game has placed its enemies, at the end of its objects, and the frame it does: after the
; enemies were drawn).
wide_previews:
    LDA     WV_M
    BEQ     @done
    LDA     SCROLL_DIR
    AND     #$02
    BEQ     @shaft
    ; Across: the rooms left of the screen's left one, and right of the screen.
    LDA     MAP_X                       ; the screen's left room
    STA     WV_CX
    LDA     SCROLL_DIR
    CMP     #SCROLLING_RIGHT
    BNE     @left_room
    LDA     SCROLL_X
    BEQ     @left_room
    DEC     WV_CX
@left_room:
    LDA     MAP_Y
    STA     WV_CY
    LDA     #$00
    STA     WV_OYL
    STA     WV_OYH
    LDA     #$FF                        ; the room on the left
    JSR     preview_across
    LDA     #$01                        ; the next ones
    JSR     preview_across
    LDA     #$02
    JSR     preview_across
    JMP     @done
@shaft:
    ; Up and down: the corridors on either side of the screen's rooms.
    LDA     MAP_Y                       ; the screen's top room
    STA     WV_CY
    LDA     SCROLL_DIR
    CMP     #SCROLLING_DOWN
    BNE     @top_room
    LDA     SCROLL_Y
    BEQ     @top_room
    DEC     WV_CY
@top_room:
    LDA     #$00
    SEC
    SBC     SCROLL_Y
    STA     WV_OYL
    LDA     #$00
    SBC     #$00
    STA     WV_OYH
    JSR     preview_sides
    INC     WV_CY                       ; the room below, 240 lines down
    LDA     WV_OYL
    CLC
    ADC     #ROOM_HEIGHT
    STA     WV_OYL
    LDA     WV_OYH
    ADC     #$00
    STA     WV_OYH
    JSR     preview_sides
@done:
    ; The frame's sprites are written: their high bytes to the extension, at once (a frame the
    ; game runs late copies the sprite page as it is then: the high bytes stay those of the last
    ; finished frame meanwhile).
    LDX     #$00
@copy:
    LDA     WV_HI_BUF,X
    STA     WIDE_SPRHI,X
    INX
    INX
    INX
    INX
    BNE     @copy
    LDX     SPRITE_PAGE_POS
    LDA     #$F4
    RTS

; The rooms left and right of the shaft's room WV_CY.
preview_sides:
    LDA     MAP_X
    STA     WV_CX
    DEC     WV_CX
    LDA     #$FF
    STA     WV_OXH
    LDA     #$00
    STA     WV_OXL
    JSR     preview_room
    INC     WV_CX
    INC     WV_CX
    LDA     #$01
    STA     WV_OXH
    LDA     #$00
    STA     WV_OXL
    JSR     preview_room
    DEC     WV_CX
    RTS

; The room A rooms right of the screen's left one (WV_CX): its x on the screen, A * 256 less the
; scroll.
preview_across:
    PHA
    CLC
    ADC     WV_CX
    STA     WV_CX
    PLA
    PHA
    STA     WV_OXH
    LDA     #$00
    SEC
    SBC     SCROLL_X
    STA     WV_OXL
    LDA     WV_OXH
    SBC     #$00
    STA     WV_OXH
    JSR     preview_room
    PLA                                 ; back to the screen's left room
    EOR     #$FF
    SEC
    ADC     WV_CX
    STA     WV_CX
    RTS

; The previews of map room (WV_CX, WV_CY), its top left corner at (WV_OX, WV_OY) on the screen:
; none when a nametable holds it (the game's own enemies), off the map or no room there.
preview_room:
    LDA     WV_CX
    CMP     #MAP_SIZE
    BCS     @out
    LDA     WV_CY
    CMP     #MAP_SIZE
    BCS     @out
    LDY     #$01
@nametable:
    LDA     WV_NT_X,Y
    CMP     WV_CX
    BNE     @other
    LDA     WV_NT_Y,Y
    CMP     WV_CY
    BNE     @other
    ; Held, but the room the game builds last: previewed until its enemies show, while the
    ; game is still on its objects ($5A below $F0) and the frame it places them (after the
    ; enemies were drawn).
    INY
    CPY     WV_FRESH
    BNE     @out
    DEY
    LDA     ROOM_NUMBER
    CMP     #ROOM_DONE
    BCC     @other
    LDA     WV_SPAWNED
    BEQ     @out
@other:
    DEY
    BPL     @nametable
    BMI     @on_map
@out:
    RTS
@on_map:
    ; The room's number: the map at $7000 + 32 y + x.
    LDA     WV_CY
    STA     TEMP_00+1
    LDA     #$00
    LSR     TEMP_00+1
    ROR
    LSR     TEMP_00+1
    ROR
    LSR     TEMP_00+1
    ROR
    ORA     WV_CX
    STA     TEMP_00
    LDA     TEMP_00+1
    ORA     #>WORLD_MAP
    STA     TEMP_00+1
    LDY     #$00
    LDA     (TEMP_00),Y
    CMP     #$FF
    BEQ     @none
    ASL                                 ; its data (an 8-bit index, as $EA42)
    TAY
    LDA     (ROOM_PTR_TABLE),Y
    STA     WV_PTRL
    INY
    LDA     (ROOM_PTR_TABLE),Y
    STA     WV_PTRH
    LDA     #$01                        ; past the default palette
    JSR     preview_step
@objects:
    LDY     #$00
    LDA     (TEMP_00),Y
    CMP     #$FF
    BEQ     @none
    CMP     #$FD
    BEQ     @list
    CMP     #$FE
    BNE     @object
    LDA     #$01
    BNE     @next_object
@object:
    LDA     #$03
@next_object:
    JSR     preview_step
    JMP     @objects
@list:
    LDA     #$01
    JSR     preview_step
@entry:
    LDY     #$00
    LDA     (TEMP_00),Y
    CMP     #$FF
    BEQ     @none
    AND     #$0F
    CMP     #$01
    BNE     @size
    ; An enemy: none when its slot holds an enemy that shows, as the game skips it then when it
    ; builds the room (IsSlotTaken, $EB7A).
    LDA     (TEMP_00),Y
    AND     #$F0
    TAX
    LDA     EN_STATUS,X
    BEQ     @free
    LDA     EN_DATA05,X
    AND     #$02
    BEQ     @free
    LDA     #$01
    BNE     @size
@free:
    INY
    LDA     (TEMP_00),Y
    STA     WV_PTYPE
    INY
    LDA     (TEMP_00),Y
    STA     WV_PPOS
    JSR     preview_enemy
    BCS     @none                       ; no sprites left
    LDA     #$01
@size:
    TAY
    JSR     entry_size
    JSR     preview_step
    JMP     @entry
@none:
    RTS

; A = the size of a room's entry after $FD of type Y ($EAD4): enemy (1) 3, door (2) 2, elevator
; (4) 2, pipe of bugs (7) 3, the others (statues) 1. A test, not a table: in the pack the code's
; data reads would reach the PPU.
entry_size:
    CPY     #$01
    BEQ     @three
    CPY     #$07
    BEQ     @three
    CPY     #$02
    BEQ     @two
    CPY     #$04
    BEQ     @two
    LDA     #$01
    RTS
@two:
    LDA     #$02
    RTS
@three:
    LDA     #$03
    RTS

; Moves the room data pointer A bytes on and loads it into $00/$01.
preview_step:
    CLC
    ADC     WV_PTRL
    STA     WV_PTRL
    STA     TEMP_00
    LDA     WV_PTRH
    ADC     #$00
    STA     WV_PTRH
    STA     TEMP_00+1
    RTS

; Draws the enemy WV_PTYPE at WV_PPOS (%yyyyxxxx, as $EB4D: y + 8, x + 12) of the room, when it
; shows within the margins; carry set when the sprites run out.
preview_enemy:
    LDA     WV_PTYPE
    AND     #$40                        ; Kraid and Ridley: not previewed
    BNE     @hidden
    LDA     SPRITE_PAGE_POS
    CMP     #LAST_PREVIEW_SPRITE
    BCS     @full
    ; y: the room's top on the screen + (pos & $F0) + 8, on the screen's 240 lines.
    LDA     WV_PPOS
    AND     #$F0
    ORA     #$08
    CLC
    ADC     WV_OYL
    STA     WV_PREV_Y
    LDA     WV_OYH
    ADC     #$00
    BNE     @hidden
    LDA     WV_PREV_Y
    CMP     #$F0
    BCS     @hidden
    ; x: the room's left on the screen + (pos << 4) + 12, within the margins.
    LDA     WV_PPOS
    ASL
    ASL
    ASL
    ASL
    ORA     #$0C
    CLC
    ADC     WV_OXL
    STA     WV_PREV_XL
    LDA     WV_OXH
    ADC     #$00
    STA     WV_PREV_XH
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     WV_M                        ; left: x >= 256 - M
    EOR     #$FF
    CLC
    ADC     #$01
    CMP     WV_PREV_XL
    BEQ     @shown
    BCC     @shown
    BCS     @hidden
@right:
    LDA     WV_PREV_XL                  ; right: x < M
    CMP     WV_M
    BCS     @hidden
@shown:
    JSR     preview_draw
@hidden:
    CLC
    RTS
@full:
    SEC
    RTS

; Draws the preview: slot $50 set aside, the enemy set up there resting (its type, toughness,
; facing right), its resting animation's frame drawn ($F682, then $F438: animation and
; drawing), the slot put back.
preview_draw:
    LDX     #$0F
@save:
    LDA     EN_Y+PREVIEW_SLOT,X
    STA     WV_PSAVE,X
    LDA     EN_STATUS+PREVIEW_SLOT,X
    STA     WV_PSAVE+$10,X
    DEX
    BPL     @save
    LDA     PAGE_INDEX
    PHA
    LDX     #PREVIEW_SLOT
    STX     PAGE_INDEX
    LDA     WV_PTYPE
    AND     #$3F
    STA     EN_TYPE,X
    LDA     WV_PTYPE
    AND     #$80                        ; the tough version: its own palette
    STA     EN_SPECIAL,X
    LDA     #$01                        ; resting
    STA     EN_STATUS,X
    LDA     #$00
    STA     EN_DATA05,X
    STA     EN_IS_HIT,X
    LDA     #$FF                        ; no animation yet: the resting one is set
    STA     EN_RESET_ANIM,X
    LDA     #$01
    STA     WV_PREVIEW
    JSR     EN_RESTING_ANIM
    LDX     #PREVIEW_SLOT
    JSR     EN_DRAW_NO_MOVE
    LDA     #$00
    STA     WV_PREVIEW
    PLA
    STA     PAGE_INDEX
    LDX     #$0F
@restore:
    LDA     WV_PSAVE,X
    STA     EN_Y+PREVIEW_SLOT,X
    LDA     WV_PSAVE+$10,X
    STA     EN_STATUS+PREVIEW_SLOT,X
    DEX
    BPL     @restore
    ; The room data pointer, which the drawing used ($00/$01).
    LDA     WV_PTRL
    STA     TEMP_00
    LDA     WV_PTRH
    STA     TEMP_00+1
    RTS

; The position test for a preview (WV_PREVIEW): its place, worked out by preview_enemy; never on
; the original screen for the game.
preview_place:
    LDA     WV_PREV_XL
    STA     SCREEN_X
    STA     WV_BASEL
    LDA     WV_PREV_XH
    STA     WV_BASEH
    LDA     WV_PREV_Y
    STA     SCREEN_Y
    LDA     #$01
    STA     WV_VALID
    STA     WV_WIDE
    LDX     #$00
    STX     WV_ORIG
    RTS

; A room built on a nametable (DeleteOffscreenRoomSprites, $ECB4, in place of LDA $0405,X /
; AND #$02: the enemies of that nametable the game keeps, those that showed): only those on the
; original screen are kept. Awake in a margin, an enemy shows and updates too, but its room is
; going: kept, it would live on in the new room's place, with its walls.
wide_room_retire:
    LDA     WV_SEEN,X
    ASL
    RTS

; A room's enemy placed ($EB5E, in place of LDA #$01 / STA $6AF4,X: its status, resting): it
; sleeps until it reaches the original screen.
wide_spawn:
    LDA     #$01
    STA     WV_SPAWNED
    STA     $6AF4,X
    STA     WV_SLEEP,X
    LDA     EN_TYPE,X
    STA     WV_STYPE,X
    RTS

; ---------------------------------------------------------------------------- ;
; A sprite's x (WriteSpriteRAM, $DF0E, in place of ADC $0E / STA $0203,X): its high byte, the
; 16-bit x nearest the object's whose low byte is the sprite's (0 for an object no position test
; placed).
wide_sprite_x:
    ADC     SCREEN_X
    STA     OAM_X,X
    LDY     WV_VALID
    BEQ     @zero
    SEC
    SBC     WV_BASEL
    BMI     @before
    LDA     WV_BASEH
    BCS     @store
    ADC     #$01                        ; carry clear
    JMP     @store
@before:
    LDA     WV_BASEH
    BCC     @store
    SBC     #$01                        ; carry set
    JMP     @store
@zero:
    TYA
@store:
    STA     WV_HI_BUF,X
    RTS

; The end of an object's drawing (ClearObjectCntrl, $DF2D, in place of LDA #$00 / STA $6B).
wide_drawn:
    LDA     #$00
    STA     WV_VALID
    STA     OBJECT_CNTRL
    RTS

; ---------------------------------------------------------------------------- ;
; Samus' damage (SubtractHealth, $CE92, in place of LDA $6E / ORA $6F): none with the option
; "Invincible" (the damage is dropped; Z set, the routine returns).
wide_hurt:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BEQ     @hurt
    LDA     #$00
    STA     HEALTH_CHANGE
    STA     HEALTH_CHANGE+1
    RTS
@hurt:
    LDA     HEALTH_CHANGE
    ORA     HEALTH_CHANGE+1
    RTS

; ---------------------------------------------------------------------------- ;
; A new game (bank 0 $9354, the end of InitializeStats, in place of LDA #$02 / STA $24: Brinstar's
; bank): with the option "Level select", a direction held at START picks the area (up Norfair,
; right Kraid, left Tourian, down Ridley), the game then starting there as a password does.
wide_new_game:
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    BEQ     @brinstar
    LDA     JOY1_STATUS
    LDY     #$11                        ; Norfair
    AND     #$0F
    CMP     #$08
    BEQ     @area
    INY                                 ; Kraid
    CMP     #$01
    BEQ     @area
    INY                                 ; Tourian
    CMP     #$02
    BEQ     @area
    INY                                 ; Ridley
    CMP     #$04
    BEQ     @area
@brinstar:
    LDA     #$02
    STA     BANK_SWITCH
    RTS
@area:
    STY     IN_AREA
    JMP     AREA_START

wide_end:
.ifdef PACK
.assert wide_end <= $3000, error, "widescreen code past the zone's code half"
.else
.assert wide_end <= $6000, error, "widescreen code past the window"
.endif
```

### B. `tools/build.py`

```python
"""Builds the Metroid (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Metroid (USA).nes> [out.bps]
       python -I tools/build.py <Metroid (USA) or (Europe).nes> --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank whose first 2 KiB the widescreen extension's PRG window shows at $5800-$5FFF (Zaxnes
   opens it at power-on for this patch: bank 8, page 0).
2. Lays the PRG-ROM out as 16 banks: the game's 8 where they were, the hack's bank 8, banks
   9-14 of $FF, and the game's fixed bank again as bank 15, the one MMC1 fixes at $C000 (Zaxnes'
   MMC1 fixes bank 15 modulo the bank count).
3. Writes the hooks into the game's banks (those of the fixed bank into both its copies), each
   checked against the bytes it replaces.
4. Writes build/metroidwide.nes (header: 16 PRG banks) and, if asked, the BPS patch (data
   without iNES header; the game's banks moved or copied come from the player's ROM).

With --pack, a Zaxnes hack pack, format 2, instead, written by Zaxnes' shared builder
(tools/hackpack.py of Zaxnes, its folder in the environment variable ZAXNES_TOOLS): MMC1, each hook
a 16 KiB bank and a CPU address, the widescreen in groups ("core", Base: the settings, the rooms
of the nametables, the sprites in the margins; "enemies"; "doors": the doors, their twins and the
items; "previews"; "invincible" and "level select", the aids, which need the hack's code) with
checks of the game's code the hack relies on; src/wide.s assembled with PACK for $2000, the zone
fantôme, which Zaxnes shows to instruction fetches from power-on, the extension unlocked; the hooks
as offsets in the PRG-ROM, the ROM keeping its size. Metroid (Europe), the same code at the same
places, gets the same pack for PAL.
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x70080810
EUROPE = 0x7751588D
BANK_SIZE = 0x4000
PRG_BANKS = 8
PRG_SIZE = PRG_BANKS * BANK_SIZE
CHR_SIZE = 0  # CHR-RAM
TARGET_BANKS = 16
HACK_BANK = 8
FIXED = 7
CODE_START = 0x5800
PACK_START = 0x2000             # the pack's code: the zone fantôme, below the extension at $3000
PACK_SIZE = 0x1000
WINDOW_SIZE = 0x800

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOP = 0xEA


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def hooks(sym):
    """The patches: (bank, address, original bytes, new bytes); bank 7 is the fixed one."""
    out = []

    def put(bank, address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((bank, address, original, new))

    # The main loop's wait for the NMI: the extension's settings; the world's update: the
    # sprites built again.
    put(7, 0xC0C7, 'a9 00 85 1a', jsr(sym['wide_frame']))
    put(7, 0xCB29, 'a2 00 86 5b', jsr(sym['wide_world']))
    # An object's place on the screen (IsObjectVisible), an enemy's drawing and its update test.
    put(7, 0xDFDF, 'a2 01 a5', jmp(sym['wide_visible']))
    put(7, 0xDE1B, '20 df df', jmp(sym['wide_enemy_draw']))
    put(7, 0xF39F, '20 df df', jsr(sym['wide_enemy_check']))
    # An object's drawing: the doors and the item in the margins too.
    put(7, 0xDED2, '20 df df', jmp(sym['wide_object_draw']))
    # A room built: where each nametable's room is on the map.
    put(7, 0xEA05, '20 85 eb', jsr(sym['wide_select_room']))
    # A room built on a nametable: its enemies in the margins go with the room it held.
    put(7, 0xECB4, 'bd 05 04 29 02', jsr(sym['wide_room_retire']))
    # A door drawn (each area bank): its twin, the facing door of the room across.
    for bank in range(1, 6):
        put(bank, 0x8BCC, 'a9 06 4c 47 de', jmp(sym['wide_draw_door']))
    # The end of the world's update: the previews of the rooms the game has not built.
    put(7, 0xCB66, 'a6 5b a9 f4', jsr(sym['wide_previews']))
    # A room's enemy placed: it sleeps until it reaches the original screen.
    put(7, 0xEB5E, 'a9 01 9d f4 6a', jsr(sym['wide_spawn']))
    # A sprite's x and the end of an object's drawing: the sprites' high bytes.
    put(7, 0xDF0E, '65 0e 9d 03 02', jsr(sym['wide_sprite_x']))
    put(7, 0xDF2D, 'a9 00 85 6b', jsr(sym['wide_drawn']))
    # Invincibility: Samus' damage.
    put(7, 0xCE92, 'a5 6e 05 6f', jsr(sym['wide_hurt']))
    # Level select: a new game (the title's bank).
    put(0, 0x9354, 'a9 02 85 24', jsr(sym['wide_new_game']))
    return out


def tool(name):
    return [name] if shutil.which(name) else ['wsl', '-e', name]


def wsl_path(path):
    if shutil.which('ca65'):
        return path
    drive, rest = os.path.splitdrive(os.path.abspath(path))
    return '/mnt/' + drive[0].lower() + rest.replace('\\', '/')


def assemble(pack=False):
    build = os.path.join(ROOT, 'build')
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    start, size = (PACK_START, PACK_SIZE) if pack else (CODE_START, BANK_SIZE)
    open(cfg, 'w', newline='\n').write(
        f'MEMORY {{ CODE: start = ${start:04X}, size = ${size:04X}, '
        'fill = yes, fillval = $FF, file = %O; }\n'
        'SEGMENTS { CODE: load = CODE, type = ro; }\n')
    obj = os.path.join(build, 'wide.o')
    binary = os.path.join(build, 'wide.bin')
    labels = os.path.join(build, 'wide.labels')
    subprocess.run(tool('ca65') + ['-g'] + (['-D', 'PACK'] if pack else [])
                   + [wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)], check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-o', wsl_path(binary), '-Ln', wsl_path(labels),
                                   wsl_path(obj)], check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return open(binary, 'rb').read(), sym


def pack_place(offset, address):
    """The place of format 2 for `offset` in the PRG-ROM, run at `address`: its 16 KiB bank, and
    the address with the offset's place in that bank (an 8 KiB bank's half of it)."""
    return offset // 0x4000, (address & 0xC000) | (offset & 0x3FFF)


# The groups past the core: (address of the hook).
GROUPS = {
    'enemies': {0xDE1B, 0xF39F, 0xECB4, 0xEB5E},
    'doors': {0xDED2, 0x8BCC},
    'previews': {0xCB66},
    'invincible': {0xCE92},
    'level select': {0x9354},
}

# The game's code the hack's code relies on, by group: (bank, address, length), checked by Zaxnes
# to be the reference ROM's bytes. Core: the nametable a room is built on. Enemies: an enemy's
# resting animation, its update without moving and where its drawing goes on. Doors: an object's
# animation and drawing and where they go on. Level select: a game's start in an area (bank 0).
CHECKS = {
    'core': [(FIXED, 0xEB85, 3)],
    'enemies': [(FIXED, 0xF682, 3), (FIXED, 0xF438, 3), (FIXED, 0xDEDE, 3), (FIXED, 0xDE53, 3)],
    'doors': [(FIXED, 0xDE47, 3), (FIXED, 0xDE4A, 3), (FIXED, 0xDEE3, 3)],
    'level select': [(0, 0x930D, 3)],
}


def pack(data, crc, out):
    tools = os.environ.get('ZAXNES_TOOLS')
    if not tools or not os.path.isfile(os.path.join(tools, 'hackpack.py')):
        sys.exit('ZAXNES_TOOLS: the rust/tools folder of Zaxnes (its hackpack.py) is needed')
    sys.path.insert(0, tools)
    import hackpack
    code, sym = assemble(pack=True)
    used = (sym['wide_end'] - PACK_START + 0xFF) & ~0xFF
    europe = crc == EUROPE
    name = 'Metroid (Europe)' if europe else 'Metroid (USA)'
    pack = hackpack.Pack(name + ' widescreen', code[:used], region='pal' if europe else 'ntsc',
                         mappers=(1,))
    widescreen = pack.component('widescreen')
    groups = {'core': widescreen.group('core', label='Base', label_fr='Base')}
    for group, label, label_fr in (('enemies', 'Enemies in the margins', 'Ennemis dans les marges'),
                                   ('doors', 'Doors and items in the margins',
                                    'Portes et objets dans les marges'),
                                   ('previews', 'Enemies of the rooms ahead',
                                    'Ennemis des salles à venir'),
                                   ('invincible', 'Invincible', 'Invincible'),
                                   ('level select', 'Level select', 'Sélecteur de niveau')):
        groups[group] = widescreen.group(group, requires=['core'], label=label, label_fr=label_fr)

    def offset(bank, address):
        return bank * BANK_SIZE + address - (0xC000 if bank == FIXED else 0x8000)

    for group, checks in CHECKS.items():
        for bank, address, length in checks:
            groups[group].check(*pack_place(offset(bank, address), address), length)
    for bank, address, original, new in hooks(sym):
        at = offset(bank, address)
        assert data[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        group = next((g for g, hooked in GROUPS.items() if address in hooked), 'core')
        groups[group].hook(*pack_place(at, address), original, bytes(new))
    result = pack.build(data, PRG_SIZE)
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes ({used} of code, '
          f'{" + ".join(str(len(group.hooks)) for group in groups.values())} hooks), '
          f'pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = bytes(image[header:header + PRG_SIZE + CHR_SIZE])
    crc = zlib.crc32(data)
    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        assert crc in (SOURCE_CRC, EUROPE), 'not Metroid (USA) or (Europe)'
        pack(bytes(data), crc, sys.argv[3])
        return
    assert crc == SOURCE_CRC, 'not Metroid (USA)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    # The window shows 2 KiB: the code must end before $6000.
    assert all(code[WINDOW_SIZE:] == bytes([0xFF]) * (BANK_SIZE - WINDOW_SIZE) for _ in [0]),         'the code does not fit in the window'
    banks = [bytearray(data[n * BANK_SIZE:(n + 1) * BANK_SIZE]) for n in range(PRG_BANKS)]
    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for bank, address, original, new in hooks(sym):
        if address in skip:
            continue
        at = address - (0xC000 if bank == FIXED else 0x8000)
        assert banks[bank][at:at + len(original)] == original, \
            f'bank {bank} ${address:04X}: not the expected bytes'
        banks[bank][at:at + len(new)] = new
    fill = bytes([0xFF]) * BANK_SIZE
    layout = banks + [bytearray(code)] + [bytearray(fill)] * (TARGET_BANKS - PRG_BANKS - 2) + [banks[FIXED]]
    target = b''.join(bytes(b) for b in layout) + data[PRG_SIZE:]

    out_header = bytearray(image[:header])
    if header:
        out_header[4] = TARGET_BANKS
    out = os.path.join(ROOT, 'build', 'metroidwide.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        last = (TARGET_BANKS - 1) * BANK_SIZE
        moved = [(last, last + BANK_SIZE, FIXED * BANK_SIZE)]
        patch = make_bps.make(data, target, moved)
        open(sys.argv[2], 'wb').write(patch)
        print(f'{sys.argv[2]}: {len(patch)} bytes')


if __name__ == '__main__':
    main()
```

### C. `tools/make_bps.py`

```python
"""Writes a BPS patch from an original ROM's data to the widescreen build's, both without their
iNES header.

Only the bytes the hack writes travel in the patch (TargetRead); everything else is copied from the
player's own ROM: SourceRead where a byte sits where it was, SourceCopy for data the build moved
(the CHR-ROM, after the PRG bank the hack adds). The patch carries no data of the game.
"""

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


def make(source, target, moved=()):
    """`moved`: (target start, target end, source start) of data the build moved."""
    patch = bytearray(b'BPS1')
    patch += number(len(source)) + number(len(target)) + number(0)
    source_relative = 0
    target_relative = 0

    def origin(position):
        """Where target byte `position` comes from in the source, when it may come from there."""
        for start, end, from_ in moved:
            if start <= position < end:
                return from_ + position - start
        return position

    position = 0
    while position < len(target):
        def same(at):
            at_source = origin(at)
            return at_source < len(source) and source[at_source] == target[at]

        copied = same(position)
        shifted = origin(position) != position
        end = position
        while end < len(target) and same(end) == copied and (origin(end) != end) == shifted:
            end += 1
        length = end - position
        pending_start = position
        if not copied:
            # Bytes of the hack; a run of one value (the bank's $FF fill) as its first byte, then
            # a TargetCopy of it.
            run = position
            while position < end:
                run = position + 1
                while run < end and target[run] == target[position]:
                    run += 1
                if run - position >= 8:
                    if pending_start < position:
                        patch += number(((position - pending_start - 1) << 2) | 1) + target[pending_start:position]
                    patch += number(1) + target[position:position + 1]
                    delta = position - target_relative
                    patch += number(((run - position - 2) << 2) | 3)
                    patch += number((abs(delta) << 1) | (1 if delta < 0 else 0))
                    target_relative = position + (run - position - 1)
                    pending_start = run
                position = run
            if pending_start < end:
                patch += number(((end - pending_start - 1) << 2) | 1) + target[pending_start:end]
        elif not shifted:
            patch += number(((length - 1) << 2) | 0)
        else:
            start = origin(position)
            delta = start - source_relative
            patch += number(((length - 1) << 2) | 2)
            patch += number((abs(delta) << 1) | (1 if delta < 0 else 0))
            source_relative = start + length
        position = end
    patch += zlib.crc32(source).to_bytes(4, 'little')
    patch += zlib.crc32(target).to_bytes(4, 'little')
    patch += zlib.crc32(bytes(patch)).to_bytes(4, 'little')
    return bytes(patch)
```
