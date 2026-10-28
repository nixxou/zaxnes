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
| Target data | CRC-32 `A23663EB` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | PRG grown to 256 KiB (16 banks): the game's banks 0-7 where they were, the hack's code as bank 8, banks 9-14 of `$FF`, the fixed bank copied to bank 15 (Zaxnes' MMC1 fixes bank 15 modulo the bank count); hooks in bank 0 and bank 7 (both copies of the fixed bank) |
| Patch | `rust/crates/zax-nes/patches/metroid-usa-widescreen.bps`, made against the data without header; the fixed bank's copy is SourceCopy, no game data in the patch |

The code runs from the widescreen extension's PRG window at `$5800-$5FFF`, which Zaxnes opens on
bank 8, page 0 at power-on for this patch (`boot_window`): the game has no free room for code that
would unlock the extension. The window stops at `$5FFF` (`$5007` bit 4 clear): the game keeps its
rooms and its world map in PRG-RAM at `$6000-$7FFF`.

---

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
| `SelectRoomRAM` (`$EA05`) | the nametable a room is built on | the same, and the room's map position (`$50`, `$4F`) kept for that nametable |
| `DrawEnemy` (`$DE1B`) | draws the enemy when it shows; `$0405` bit 1 = it shows | draws it when it shows within the margins; bit 1 = it updates (below) |
| `UpdateEnemy_CheckIfVisible` (`$F39F`) | skips the enemy's routine when it does not show | skips it when it does not update; asleep in a margin, it is drawn in its resting pose (`$F682`, then `$F438`: animation and drawing, no movement) |
| an enemy placed by its room (`$EB5E`) | status 1 (resting) | the same, and asleep (`$5300 + slot`, with its type at `$5301 + slot`: a slot holding another type since is awake) |
| a sprite's x (`$DF0E`) | 8-bit | the same, and its high byte to `$5100 + OAM offset`: the 16-bit x nearest the object's whose low byte is the sprite's; 0 for sprites no position test placed (the energy display) |
| `ObjDrawFrame` (`$DED2`) | draws the object when it shows; its `onScreen` byte = it shows | `onScreen` the same; the doors (objects `$80-$B0`) and the item (`$40`) are drawn when they show within the margins |
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
hidden: the decoder draws the background), `$5007 = 2` (sprites free of the extents), `$5006 = 64`
(255 with "No slowdown"), clears the sprite high bytes out of play, then `$1A`).

Variables (extension RAM): `WV_M $5200`, `WV_ORIG $5201` and `WV_WIDE $5202` (the last position
test: on the original screen, within the margins), `WV_VALID $5203` / `WV_BASEL`, `WV_BASEH
$5204/$5205` (the 16-bit screen x of the object being drawn), `WV_ASLEEP $5206` (the last enemy
test), `WV_NT_X $5210-$5211` and `WV_NT_Y $5212-$5213` (by nametable: the map position of the room the game built there last), `WV_T0 $5214`, `WV_SLEEP $5300 + slot offset`, `WV_STYPE $5301 + slot offset`.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length. Those of
the fixed bank (`$C000-$FFFF`) are written into both its copies (7 and 15).

| Address | Replaces | With |
|---|---|---|
| `$C0C7` | `LDA #$00 / STA $1A` | `JSR wide_frame / NOP` |
| `$CB29` | `LDX #$00 / STX $5B` | `JSR wide_world / NOP` |
| `$DFDF` | `LDX #$01 / LDA` (IsObjectVisible) | `JMP wide_visible` |
| `$EA05` | `JSR $EB85` (SelectRoomRAM) | `JSR wide_select_room` |
| `$DE1B` | `JSR $DFDF` (DrawEnemy) | `JMP wide_enemy_draw` |
| `$DED2` | `JSR $DFDF` (ObjDrawFrame) | `JMP wide_object_draw` |
| `$F39F` | `JSR $DFDF` (UpdateEnemy_CheckIfVisible) | `JSR wide_enemy_check` |
| `$EB5E` | `LDA #$01 / STA $6AF4,X` | `JSR wide_spawn / NOP / NOP` |
| `$DF0E` | `ADC $0E / STA $0203,X` | `JSR wide_sprite_x / NOP / NOP` |
| `$DF2D` | `LDA #$00 / STA $6B` | `JSR wide_drawn / NOP` |
| `$CE92` | `LDA $6E / ORA $6F` | `JSR wide_hurt / NOP` |
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
* Only the two rooms the game keeps in PRG-RAM have enemies; the other rooms of the margins are
  empty. The elevators, the statues and the other objects keep the original screen.
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
;    the original screen);
;  * every sprite of an object gets the high byte of its 16-bit x, the room each nametable holds
;    telling which margin an object of the other nametable shows in;
;  * Samus, her shots and the other objects keep the original screen;
;  * the option "Level select" starts a new game in the area a direction held picks.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1 sprites free of the extents
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA

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

; Variables in the extension's RAM ($5200-$57FF).
WV_M            = $5200         ; the margin enemies show in, this frame
WV_ORIG         = $5201         ; the last position test: 1 on the original screen, else 0
WV_WIDE         = $5202         ; and 1 within the margins, else 0
WV_VALID        = $5203         ; non-zero while WV_BASE is the object being drawn
WV_BASEL        = $5204         ; the 16-bit screen x of that object ($0E and its high byte)
WV_BASEH        = $5205
WV_ASLEEP       = $5206         ; the last enemy test: 1 when it sleeps in a margin
WV_NT_X         = $5210         ; + nametable (0, 1): the map x and y of the room the game built
WV_NT_Y         = $5212         ; there last
WV_T0           = $5214         ; scratch
WV_SLEEP        = $5300         ; + enemy slot offset: 1 asleep (placed by its room, not yet on
                                ; the original screen)
WV_STYPE        = $5301         ; + enemy slot offset: the type it was placed as

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
OBJ_ON_SCREEN   = $030B         ; + object offset: it shows (the game's test)
POWER_UP_DRAW   = $40           ; object offsets: the item being drawn, the doors
FIRST_DOOR      = $80
LAST_DOOR       = $B0
JOY1_STATUS     = $14           ; the buttons held
BANK_SWITCH     = $24           ; the PRG bank to switch to, plus one
IN_AREA         = $74
EN_DATA05       = $0405         ; + slot offset: bit 1 the enemy updates (it showed)
EN_TYPE         = $6B02         ; + slot offset
OAM_X           = $0203

; The game's routines.
NAMETABLE_AT_SCROLL = $EB85     ; GetNameTableAtScrollDir: the nametable a room is built on
EN_RESTING_ANIM = $F682         ; InitEnRestingAnimIndex
EN_DRAW_NO_MOVE = $F438         ; UpdateEnemyCommon@noMove: animation and drawing only
EN_DRAW_GO      = $DEDE         ; DrawEnemy_NotBlank, drawing the enemy
EN_DRAW_SKIP    = $DE53         ; DrawEnemy_NotBlank, not drawing it
OBJ_DRAW_GO     = $DEDE         ; ObjDrawFrame, drawing the object
OBJ_DRAW_SKIP   = $DEE3         ; ObjDrawFrame, not drawing it
AREA_START      = $930D         ; bank 0 (the title's): a game started in the area $74, as a
                                ; password starts one

.segment "CODE"
.org $5800

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
    LDA     #$02                        ; sprites free of the extents
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
    JSR     clear_high_bytes
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
    LDX     WV_ORIG
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
    JSR     wide_awake
    ASL
    STA     TEMP_08
    LDX     PAGE_INDEX
    LDA     EN_DATA05,X
    AND     #$FD
    ORA     TEMP_08
    STA     EN_DATA05,X
    LDA     WV_WIDE
    BEQ     @skip
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
    STA     OBJ_ON_SCREEN,X
    TAY                                 ; Z: the game's test
    BNE     @draw
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

; A room's enemy placed ($EB5E, in place of LDA #$01 / STA $6AF4,X: its status, resting): it
; sleeps until it reaches the original screen.
wide_spawn:
    LDA     #$01
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
    STA     WIDE_SPRHI,X
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
```

### B. `tools/build.py`

```python
"""Builds the Metroid (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Metroid (USA).nes> [out.bps]

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
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x70080810
BANK_SIZE = 0x4000
PRG_BANKS = 8
PRG_SIZE = PRG_BANKS * BANK_SIZE
CHR_SIZE = 0  # CHR-RAM
TARGET_BANKS = 16
HACK_BANK = 8
FIXED = 7
CODE_START = 0x5800
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


def assemble():
    build = os.path.join(ROOT, 'build')
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    open(cfg, 'w', newline='\n').write(
        f'MEMORY {{ CODE: start = ${CODE_START:04X}, size = ${BANK_SIZE:04X}, '
        'fill = yes, fillval = $FF, file = %O; }\n'
        'SEGMENTS { CODE: load = CODE, type = ro; }\n')
    obj = os.path.join(build, 'wide.o')
    binary = os.path.join(build, 'wide.bin')
    labels = os.path.join(build, 'wide.labels')
    subprocess.run(tool('ca65') + ['-g', wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)],
                   check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-o', wsl_path(binary), '-Ln', wsl_path(labels),
                                   wsl_path(obj)], check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return open(binary, 'rb').read(), sym


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = bytes(image[header:header + PRG_SIZE + CHR_SIZE])
    assert zlib.crc32(data) == SOURCE_CRC, 'not Metroid (USA)'

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
