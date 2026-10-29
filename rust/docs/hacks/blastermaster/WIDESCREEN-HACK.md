# Blaster Master (USA): the widescreen hack

This document describes completely the modification of *Blaster Master (USA)* that Zaxnes ships as
a built-in patch, the advanced mode of its widescreen. The appendices hold the new code and the
build script, which writes every hook after checking the bytes it replaces: with them the ROM can
be rebuilt identically.

The background of the margins is not the hack's work: the emulator's Blaster Master margin decoder
reads the area's map in PRG-ROM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4); it widens the game on
its own, the simple mode. The hack does the objects: enemies that spawn, sleep, live and show in
the margins, the way the Mega Man 2 and Metal Force hacks do them (owner request: the game stays
the original's on the original screen).

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Blaster Master (USA)*, MMC1 (SxROM), 128 KiB PRG (8 banks of 16 KiB, bank 7 fixed at `$C000`), 128 KiB CHR-ROM |
| Source data (iNES header excluded) | CRC-32 `3F0FD764` |
| Target data | CRC-32 `70D56BFD` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | PRG grown to 256 KiB (16 banks): the game's banks 0-7 where they were, the hack's code as bank 8, banks 9-14 of `$FF`, the fixed bank copied to bank 15 (Zaxnes' MMC1 fixes bank 15 modulo the bank count); hooks in banks 4, 6 and 7 (both copies of the fixed bank) |
| Patch | `rust/crates/zax-nes/patches/blastermaster-usa-widescreen.bps`, made against the data without header; the game's banks moved or copied are SourceCopy, no game data in the patch |

The code runs from the widescreen extension's PRG window at `$5800-$7FFF` (`$5007` bit 4), which
Zaxnes opens on bank 8, page 0 at power-on for this patch (`boot_window`): the game has no free
room for code that would unlock the extension.

---

**Hack pack.** Zaxnes ships this hack as a hack pack,
`rust/crates/zax-nes/patches/blastermaster-usa-widescreen.zxpk` (`tools/build.py <rom> --pack
<out.zxpk>`, with `ZAXNES_TOOLS` set to Zaxnes' `rust/tools`;
`docs/HACK-PACKS.md`, `WIDESCREEN-EXTENSION.md` 7.1). The pack is format 2, written by Zaxnes'
shared builder (`hackpack.py`): NTSC, MMC1, each hook a 16 KiB bank and a CPU address, the
widescreen in one group ("core", Base); no checks yet. The code goes to the extension's
memory, which the window shows from power-on (`$5008` = `$80`, page 0), and the hooks, each with
the game's bytes it replaces, are checked and applied by Zaxnes; the ROM keeps its size (no bank
added, nothing moved or copied). Its testing aids need the hack's code: they stay with the
widescreen (the advanced mode). The BPS patch described here is kept to compare.

## 2. What the game does, and what changes

The game's frame ends in a wait (`$E936`: `LDA #$80 / STA $12`, then `BIT $12 / BMI` until the NMI
clears it, `$E93A`; Zaxnes' idle loop for this patch). A frame whose work does not fit misses the
next NMI's update: a lag frame.

Objects: 18 slots (`$57` the slot at hand), 0 the player (the tank, or Jason on foot), 1-7 the
shots and the like, 8-17 the enemies the area's list spawns. The object at hand is copied to
`$46-$53` (`$46` type, `$48/$49` x, `$4A/$4B` y in sixteenths of a pixel, `$53` health) while its
routine runs (`$C9A4`, by its type, called by the object loop at `$C987`). The camera is
`$1D:$1C`, `$1F:$1E`, in sixteenths of a pixel. An enemy of the list waits hidden (type 1, its
real type at `$0150 + slot`) until its place comes on the screen, then comes out (bank 6 `$8124`
in the side view, bank 4 `$8AF1` in the overhead view); one that leaves the screen hides again
(type 2) and is removed when far enough.

| Game routine | Original | Hack |
|---|---|---|
| position (`$EF2B`, also `$C0FF`) | `$3E/$3F` the screen x and y; showing when the object (`$40` x `$41`) is wholly on the screen, y within 12-227 | the same for slots 0-7; an enemy (slot 8 up) also shows with its x within `[w/2 - M, 256 - w/2 + M)`; the 16-bit screen x is kept for its sprites |
| sprites to OAM (`$EC73`) | copies the object's sprites from `$0600` to OAM from `$3D`, moving on or back every other frame (the flicker) | the same, and each sprite's high x byte to `$5100 + OAM offset`: the 16-bit x nearest the object's whose low byte is the sprite's; 0 for sprites no position test placed (the power gauges) |
| an enemy comes out (bank 6 `$8134`, bank 4 `$8B01`) | its hidden type becomes its type | in a margin, the first time (type 1), it runs `INIT_FRAMES` (4) frames to set itself up (most types change type and draw nothing at first), then sleeps: it shows, as its own routine draws it, but neither moves nor makes anything until its x reaches the original screen; awake, it stays awake |
| object loop (`$C987`) | runs the object's routine | a sleeping enemy's routine runs (it draws itself) and its 14 bytes are put back as they were; no new object (the slot search `$D7CF` finds none) and no sound (`$DECC`) meanwhile; a slot that holds another object than the one asleep there is awake |
| spawn window (`$CAA4`) | an entry of the area's list spawns within `[0, 22)` 16-pixel units of the screen's left edge, the difference masked to 7 bits | `[0, 22 + K)`, `K = ceil(M / 16)` on each side |
| hidden enemy's removal (bank 6 `$8141`, bank 4 `$8B0E`) | outside `[-8, 24)` 16-pixel units | outside `[-8 - K, 24 + K)` |
| damage (`$D6A2`, `$53 - $45`), hazards (bank 6 `$8605`, `$89A7`, bank 4 `$8DE6`: the health less A), overhead hit (bank 4 `$8C94`, 32 points), fall (bank 6 `$82C8`, 64 points, all past a speed of `$40`) | | none for the player (slot 0) when the option "Invincible" is on |
| new game (`$C2BC`, `LDA #$08 / STA $14`, area 1) | | with the option "Level select", the game's own area select (`$F465`: "SELECT START AREA AND PRESS START", up / down the area, Start), then the new game goes on past its opening scene (`$CDBA`, then `$C2C6`; the scene `$F5D9`, made for area 1, is skipped) |

The player and the shots (slots 0-7) keep the original screen (owner request): a shot leaving it
is removed as in the original. The game's collisions use world coordinates; a sleeping enemy's
state is put back every frame, so it touches nothing it would not touch in place.

Margin: `M = min($5002, 248)`. With `M = 0` every routine does exactly what the code it replaces
does: `K = 0`, the windows are the game's, no enemy shows or comes out off the original screen.

CPU time: 64 lines of overclock (`$5006`) pay for the hack's work and the enemies of the margins;
255 with the option "No slowdown". Measured over 1500 frames of area 1 (88-pixel margin, holding
right): the original runs late 13 frames; the hack with 64 lines 18; with "No slowdown" none (96
lines or more: none too).

## 3. The extension

Set every frame at the end of the game's work (`$E936`, `LDA #$80 / STA $12` becomes
`JSR wide_frame / NOP`, which writes `$5003 = M`, `$5004 = $5005 = 0` (the nametable columns are
hidden: the decoder draws the background), `$5007 = $12` (sprites free of those extents, the
window kept to `$7FFF`), `$5006 = 64` (255 with "No slowdown"), works out `K`, then sets `$12`).

Variables (extension RAM): `WV_M $5200`, `WV_K $5201`, `WV_ORIG $5202` (the last position test
found the object on the original screen), `WV_VALID $5203` / `WV_BASEL`, `WV_BASEH $5204/$5205`
(the 16-bit x of the object being drawn, less 4 as `$EFD7` makes `$3E`), `WV_PLACED $520D` (that
base, for the copy under way), `WV_FROZEN $5209` (a sleeping enemy's routine runs), `WV_X $520A`
(X kept across the position test), scratch `WV_T0-T3`, `WV_HALF`, `WV_SAVE $5210-$521D` (the
sleeping enemy's `$46-$53`), `WV_DORM $5220 + slot` (0 awake, 1 asleep, `$80 + n` coming out, `n`
frames left), `WV_DTYPE $5240 + slot` (the type it sleeps as).

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length. Bank 7 is
the fixed bank (`$C000-$FFFF`), written into both its copies (7 and 15).

| Address | Replaces | With |
|---|---|---|
| `$E936` | `LDA #$80 / STA $12` | `JSR wide_frame / NOP` |
| `$EF2B` | `LDA $48 / SEC` (the position routine) | `JMP wide_position` |
| `$EC73` | `LDA $3C / BEQ` (the sprite copy) | `JMP wide_copy` |
| bank 6 `$8134`, bank 4 `$8B01` | `LDX $57 / LDA $0150,X` | `JSR wide_come_out / NOP / NOP` |
| `$C987` | `JSR $C9A4` | `JSR wide_run` |
| `$D7CF` | `LDA $0400,X` | `JMP wide_slot_search` |
| `$DECC` | `STA $E1 / TXA` | `JMP wide_sound` |
| `$CAA4` | `AND #$7F / CLC / ADC #$06 / CMP #$1C` | `JSR wide_spawn_window / NOP` x 4 |
| bank 6 `$8141`, bank 4 `$8B0E` | `CLC / ADC #$08 / AND #$7F / CMP #$20` | `JSR wide_hidden_far / NOP` x 4 |
| `$D6A2` | `LDA $53 / SEC / SBC $45` | `JSR wide_hurt / NOP / NOP` |
| bank 6 `$8605`, `$89A7`, bank 4 `$8DE6` | `EOR #$FF / CLC / ADC #$01 / CLC / ADC $53` | `JSR wide_lose / NOP` x 5 |
| bank 4 `$8C94` | `LDA $53 / SEC / SBC #$20` | `JSR wide_hurt_32 / NOP / NOP` |
| bank 6 `$82C8` | `LDA $4D / CMP #$40 / BCS / LDA $53 / SEC / SBC #$40 / BCS / LDA #$00 / STA $53` | `JSR wide_fall / NOP` x 14 |
| `$C2BC` | `LDA #$08 / STA $14` | `JSR wide_new_game / NOP` |

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there (`WIDE_OPTIONS`).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (at a new game) / Sélecteur de niveau (en début de partie) | off | a testing aid: a new game starts on the game's area select (01-08 the areas, 09-16 their overhead sections) |
| 1 | `invincible` | Invincible | off | a testing aid: the player loses no health (enemies, hazards, falls) |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of 64 |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `PATCHES`); the ROM does
not depend on them being set.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Blaster Master (USA).nes" build/blastermasterwide.bps
```

assembles `src/wide.s` for `$5800` (one 16 KiB bank, `-g`, labels from `ld65 -Ln`), lays the PRG
out in 16 banks, writes the hooks, `build/blastermasterwide.nes` and the BPS patch
(`tools/make_bps.py`: the fixed bank's copy and the CHR-ROM are SourceCopy from the player's ROM,
the hooks and the code TargetRead). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.

## 7. Limits

* No sprite in the vertical margins: the extension carries a high byte for x only.
* Only the enemies of the area's list (slots 8-17) live in the margins; what they fire keeps to
  the game's own limits.
* The sprites of the margins share the console's 64 sprites with the screen's: more flicker when
  many enemies show.
* The overclock removes lag frames (the intro included), so a replay of the original's inputs frame
  for frame drifts from the original.
* The overhead sections (Jason on foot) and the vertical shafts were checked less than the side
  view of area 1.

---

## Appendices

### A. `src/wide.s`

```asm
; Blaster Master (USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added to the game (PRG bank 8 of 16: the game's banks 0-7 stay
; where they were and its fixed bank is copied to bank 15, the one MMC1 fixes at $C000); the
; widescreen extension's PRG window shows it at $5800-$7FFF ($5007 bit 4), Zaxnes opening it at
; power-on (the patch's boot window: bank 8, page 0). The game reaches it through hooks in its own
; code (tools/build.py). The background of the margins is Zaxnes' margin decoder's work; the hack
; does the enemies, the way the Mega Man 2 and Metal Force hacks do them:
;
;  * an enemy of the area's list (object slots 8-17) lives, shows and is drawn within the
;    margins instead of the original screen;
;  * one the game makes come out of its hiding place (types 1 and 2, $8124 / $8AF1) in a margin
;    sleeps there: it shows, as its own routine draws it, but neither moves nor makes anything
;    until it reaches the original screen; awake, it stays awake;
;  * every sprite of an object gets the high byte of its 16-bit x;
;  * the player and the shots (slots 0-7) keep the original screen.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1 sprites free of the extents, bit 4 window to $7FFF
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA

OPT_LEVEL_SELECT = $01          ; a testing aid: the game's own area select at a new game
OPT_INVINCIBLE  = $02           ; a testing aid: the player takes no damage
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late

MAX_MARGIN      = 248           ; the most the extension shows sprites in
OVERCLOCK_LINES = 64            ; pays for the hack's work and the enemies of the margins
NO_SLOWDOWN_LINES = 255
FIRST_ENEMY     = 8             ; the slots the area's list spawns into: 8-17
INIT_FRAMES     = 4             ; frames a sleeping enemy runs for its start before it sleeps

; Variables in the extension's RAM ($5200-$57FF).
WV_M            = $5200         ; margin of this frame
WV_K            = $5201         ; ceil(M / 16): the spawn and removal windows grow by that many
                                ; 16-pixel units on each side
WV_ORIG         = $5202         ; the last position test: non-zero when on the original screen
WV_VALID        = $5203         ; non-zero while WV_BASE is the object being drawn
WV_BASEL        = $5204         ; the 16-bit screen x of that object (as $3E, after $EFD7)
WV_BASEH        = $5205
WV_T0           = $5206         ; scratch
WV_T1           = $5207
WV_HALF         = $5208         ; half the object's width
WV_X            = $520A         ; X, kept across the position test
WV_T2           = $520B         ; scratch
WV_T3           = $520C
WV_PLACED       = $520D         ; WV_VALID for the copy under way (WV_VALID is cleared at once)
WV_FROZEN       = $5209         ; non-zero while a sleeping enemy's routine runs
WV_SAVE         = $5210         ; its $46-$53 (14 bytes, to $521D)
WV_DORM         = $5220         ; + slot (0-17): 0 awake, 1 asleep, $80 + n: coming out, n
                                ; frames left for the routine's own start (it runs)
WV_DTYPE        = $5240         ; + slot: the type it fell asleep as

; The game.
OBJ             = $46           ; the object at hand, 14 bytes ($46-$53): type, ...
OBJ_TYPE        = $46
OBJ_XL          = $48           ; x, y: sixteenths of a pixel
OBJ_XH          = $49
OBJ_WIDTH       = $40           ; the size the position test uses
SLOT            = $57           ; the slot at hand, 0-17
CAM_XL          = $1C           ; the camera: sixteenths of a pixel
CAM_XH          = $1D
SCREEN_X        = $3E           ; the object's screen x and y, 8-bit
HIDDEN_TYPE     = $0150         ; + slot: the type a hidden enemy comes out as
HEALTH          = $53           ; the object at hand's health
DAMAGE          = $45
FALL_SPEED      = $4D
FRAME_FLAG      = $12           ; bit 7: the frame's work is done (the NMI then updates the screen)
AREA            = $14
SPRITE_BUFFER   = $0600
SPRITE_COUNT    = $3C           ; bytes in the buffer
SPRITE_NEXT     = $3D           ; where the next object's sprites go in OAM (the flicker)
FRAME_COUNT     = $11
OAM             = $0200
NO_FLICKER      = $E6BE         ; a ROM byte, 0: the sprites move in OAM every other frame

; The game's routines.
POSITION_TAIL   = $EFD7         ; $3E -= 4, $3F -= 5 or 9
RUN_OBJECT      = $C9A4         ; the object's routine (by its type)
AREA_SELECT     = $F465         ; the game's own area select (SELECT START AREA / PRESS START)
NEW_GAME_INIT   = $CDBA         ; a new game's first call, after the area is set
NEW_GAME_GO     = $C2C6         ; a new game, past its opening scene ($F5D9)
SLOT_SEARCH_GO  = $D7D2         ; the free slot search, after its LDA $0400,X
SOUND_GO        = $DECF         ; a sound request, after its STA $E1 / TXA

.segment "CODE"
.org $5800

; ---------------------------------------------------------------------------- ;
; The frame's wait ($E936, in place of LDA #$80 / STA $12, the game's work for the frame done):
; the extension's settings for the next frame.
wide_frame:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    CLC                                 ; K = ceil(M / 16)
    ADC     #$0F
    ROR
    LSR
    LSR
    LSR
    STA     WV_K
    LDA     #$00
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    LDA     #$12                        ; sprites free of the extents, the window kept to $7FFF
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
    LDA     #$80
    STA     FRAME_FLAG
    RTS

; ---------------------------------------------------------------------------- ;
; An object's place on the screen ($EF2B, the whole routine; also reached by $C0FF): $3E/$3F its
; screen x and y, A = 0 (Z set) when it shows, $FF otherwise. The original test (the object, of
; the size $40 x $41, wholly on the screen, y within 12-227) is kept in WV_ORIG; an enemy of the
; area's list (slot 8 up) also shows within the margins (its x within [w/2 - M, 256 - w/2 + M)).
; The 16-bit x is kept for its sprites (WV_BASE).
wide_position:
    STX     WV_X
    LDA     OBJ_XL                      ; the game's own computation
    SEC
    SBC     CAM_XL
    STA     $3E
    STA     WV_T0
    LDA     OBJ_XH
    SBC     CAM_XH
    STA     WV_T1
    LSR
    ROR     $3E
    LSR
    ROR     $3E
    LSR
    ROR     $3E
    LSR
    ROR     $3E
    STA     $00
    LDA     $4A
    SEC
    SBC     $1E
    STA     $3F
    LDA     $4B
    SBC     $1F
    LSR
    ROR     $3F
    LSR
    ROR     $3F
    LSR
    ROR     $3F
    LSR
    ROR     $3F
    JSR     wide_y_off                  ; y: on the screen, between 12 and 227
    BCC     @y_on
    JMP     @off
@y_on:
    LDX     #$04                        ; x on 16 bits: the difference shifted, signed
@shift:
    LDA     WV_T1
    CMP     #$80
    ROR     WV_T1
    ROR     WV_T0
    DEX
    BNE     @shift
    LDA     OBJ_WIDTH
    LSR
    STA     WV_HALF
    LDA     $00                         ; x: the original test
    AND     #$07
    BNE     @margins
    LDA     WV_HALF
    CLC
    ADC     $3E
    CMP     OBJ_WIDTH
    BCC     @margins
    LDA     #$01
    STA     WV_ORIG
    BNE     @on
@margins:
    LDA     #$00
    STA     WV_ORIG
    LDA     SLOT                        ; the player and the shots: the original screen only
    CMP     #FIRST_ENEMY
    BCC     @off_x
    LDA     WV_M
    BEQ     @off_x
    ; x >= half - M: x - half + M >= 0 (signed 16-bit)
    LDA     WV_T0
    SEC
    SBC     WV_HALF
    STA     WV_T2
    LDA     WV_T1
    SBC     #$00
    STA     WV_T3
    LDA     WV_T2
    CLC
    ADC     WV_M
    LDA     WV_T3
    ADC     #$00
    BMI     @off_x
    ; x < 256 - half + M: x + half - M < 256
    LDA     WV_T0
    CLC
    ADC     WV_HALF
    STA     WV_T2
    LDA     WV_T1
    ADC     #$00
    STA     WV_T3
    LDA     WV_T2
    SEC
    SBC     WV_M
    LDA     WV_T3
    SBC     #$00
    BNE     @off_x
@on:
    JSR     wide_keep_base
    JSR     POSITION_TAIL
    LDX     WV_X
    LDA     #$00
    RTS
@off:
    LDA     #$00
    STA     WV_ORIG
    LDX     #$04
@shift_off:
    LDA     WV_T1
    CMP     #$80
    ROR     WV_T1
    ROR     WV_T0
    DEX
    BNE     @shift_off
@off_x:
    JSR     wide_keep_base
    JSR     POSITION_TAIL
    LDX     WV_X
    LDA     #$FF
    RTS

; The game's vertical test ($EF4F-$EF83; A the y difference's high part shifted, $3F the y):
; carry set when the object is not wholly between 12 and 227.
wide_y_off:
    AND     #$07
    BNE     @off
    LDA     $41
    LSR
    EOR     #$FF
    CLC
    ADC     #$01
    CLC
    ADC     $3F
    BCC     @off
    CMP     #$0C
    BCC     @off
    CLC
    ADC     $41
    BCS     @off
    CMP     #$E4
    BCS     @off
    CLC
    RTS
@off:
    SEC
    RTS

; WV_BASE = the 16-bit x (WV_T1:WV_T0) - 4, as $3E becomes in $EFD7.
wide_keep_base:
    LDA     WV_T0
    SEC
    SBC     #$04
    STA     WV_BASEL
    LDA     WV_T1
    SBC     #$00
    STA     WV_BASEH
    LDA     #$01
    STA     WV_VALID
    RTS

; ---------------------------------------------------------------------------- ;
; The object's sprites to OAM ($EC73, the whole routine): the game's copy of the buffer at $0600
; ($3C bytes) to OAM from $3D, moving on or back every other frame for the flicker; each sprite's
; high byte goes with it to $5100: the 16-bit x nearest the object's (WV_BASE) whose low byte is
; the sprite's, 0 for sprites no position test placed (the power gauges).
wide_copy:
    LDA     WV_VALID                    ; the base serves this copy only, whichever way it ends
    STA     WV_PLACED
    LDA     #$00
    STA     WV_VALID
    LDA     SPRITE_COUNT
    BEQ     @rts
    LDA     NO_FLICKER
    BNE     @forward
    LDA     FRAME_COUNT
    LSR
    BCS     @backward
@forward:
    LDA     SPRITE_NEXT
    TAY
    CLC
    ADC     SPRITE_COUNT
    BCC     @start
@rts:
    RTS
@backward:
    LDA     SPRITE_NEXT
    SEC
    SBC     SPRITE_COUNT
    BCS     @from
    LDA     #$00
    SEC
    SBC     SPRITE_COUNT
@from:
    TAY
@start:
    STA     SPRITE_NEXT
    LDX     #$00
@sprite:
    LDA     SPRITE_BUFFER,x             ; y, tile
    STA     OAM,y
    INX
    INY
    LDA     SPRITE_BUFFER,x
    STA     OAM,y
    INX
    INY
    LDA     SPRITE_BUFFER,x             ; attributes, x
    STA     OAM,y
    INX
    INY
    LDA     SPRITE_BUFFER,x
    STA     OAM,y
    JSR     wide_high_byte
    STA     WIDE_SPRHI,y
    INX
    INY
    CPX     SPRITE_COUNT
    BNE     @sprite
    LDA     #$00
    STA     SPRITE_COUNT
    RTS

; The high byte of a sprite whose x is A (X and Y kept).
wide_high_byte:
    PHA
    LDA     WV_PLACED
    BNE     @placed
    PLA
    LDA     #$00
    RTS
@placed:
    PLA
    SEC
    SBC     WV_BASEL
    BCC     @below
    CMP     #$80                        ; at or right of the base's low byte
    LDA     WV_BASEH
    BCC     @rts
    SEC                                 ; 128 or more right: the page before
    SBC     #$01
    RTS
@below:
    CMP     #$80                        ; left of it: 128 or more back, the same page
    LDA     WV_BASEH
    BCS     @rts
    CLC                                 ; less: the next page
    ADC     #$01
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; Sleeping enemies.

; A hidden enemy comes out ($8134 in bank 6, $8B01 in bank 4, in place of LDX $57 / LDA $0150,X;
; the position test just found it showing): an enemy coming out for the first time (type 1) in a
; margin will sleep, once its routine has started it (most types first set themselves up and
; change type, drawing nothing); one that hid after it was out (type 2), or one coming out on the
; original screen, is awake. X = the slot, A = the type it comes out as.
wide_come_out:
    LDX     SLOT
    LDA     OBJ_TYPE
    CMP     #$01
    BNE     @awake
    LDA     WV_ORIG
    BNE     @awake
    LDA     #$80 + INIT_FRAMES
    STA     WV_DORM,x
    LDA     HIDDEN_TYPE,x
    STA     WV_DTYPE,x
    RTS
@awake:
    LDA     #$00
    STA     WV_DORM,x
    LDA     HIDDEN_TYPE,x
    RTS

; The object loop ($C987, in place of JSR $C9A4, the object's routine): a sleeping enemy whose x
; is on the original screen wakes for good; otherwise its routine runs (it draws itself) and its
; 14 bytes are put back as they were, the routine making nothing meanwhile (no new object, no
; sound). A slot that holds another object than the one that fell asleep there is awake.
wide_run:
    LDX     SLOT
    LDA     WV_DORM,x
    BEQ     @run
    LDA     OBJ_TYPE
    CMP     WV_DTYPE,x
    BNE     @wake
    LDA     OBJ_XL                      ; on the original screen: x - camera within 0-4095
    SEC                                 ; (sixteenths of a pixel, 0-255 pixels)
    SBC     CAM_XL
    LDA     OBJ_XH
    SBC     CAM_XH
    AND     #$F0
    BEQ     @wake
    LDA     WV_DORM,x
    BPL     @asleep
    JSR     RUN_OBJECT                  ; coming out: its routine starts it
    LDX     SLOT
    LDA     OBJ_TYPE
    CMP     WV_DTYPE,x
    BNE     @started
    DEC     WV_DORM,x
    LDA     WV_DORM,x
    CMP     #$80
    BNE     @rts
@started:
    LDA     OBJ_TYPE                    ; asleep as it is now
    STA     WV_DTYPE,x
    LDA     #$01
    STA     WV_DORM,x
@rts:
    RTS
@asleep:
    LDX     #$0D
@save:
    LDA     OBJ,x
    STA     WV_SAVE,x
    DEX
    BPL     @save
    INC     WV_FROZEN
    JSR     RUN_OBJECT
    LDA     #$00
    STA     WV_FROZEN
    LDX     #$0D
@restore:
    LDA     WV_SAVE,x
    STA     OBJ,x
    DEX
    BPL     @restore
    RTS
@wake:
    LDX     SLOT
    LDA     #$00
    STA     WV_DORM,x
@run:
    JMP     RUN_OBJECT

; The free slot search ($D7CF, in place of LDA $0400,X): none while a sleeping enemy runs.
wide_slot_search:
    LDA     WV_FROZEN
    BEQ     @search
    LDA     #$00                        ; Z set: no slot
    RTS
@search:
    LDA     $0400,x
    JMP     SLOT_SEARCH_GO

; A sound request ($DECC, in place of STA $E1 / TXA): none while a sleeping enemy runs.
wide_sound:
    PHA
    LDA     WV_FROZEN
    BEQ     @play
    PLA
    RTS
@play:
    PLA
    STA     $E1
    TXA
    JMP     SOUND_GO

; ---------------------------------------------------------------------------- ;
; The windows that grow with the margin (16-pixel units, the difference masked to 7 bits).

; The area list's spawn window ($CAA4, in place of AND #$7F / CLC / ADC #$06 / CMP #$1C): an
; enemy spawns within [0, 22 + K) of the screen's left edge (the game's [0, 22): it masks the
; difference before adding 6).
wide_spawn_window:
    AND     #$7F
    CLC
    ADC     #$06
    ADC     WV_K
    STA     WV_T0
    LDA     WV_K
    ASL
    ADC     #$1C
    STA     WV_T1
    LDA     WV_T0
    CMP     WV_T1
    RTS

; A hidden enemy's removal ($8141 in bank 6, $8B0E in bank 4, in place of CLC / ADC #$08 /
; AND #$7F / CMP #$20): removed outside [-8 - K, 24 + K).
wide_hidden_far:
    CLC
    ADC     #$08
    CLC
    ADC     WV_K
    AND     #$7F
    STA     WV_T0
    LDA     WV_K
    ASL
    ADC     #$20
    STA     WV_T1
    LDA     WV_T0
    CMP     WV_T1
    RTS

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1).

; Damage ($D6A2, in place of LDA $53 / SEC / SBC $45): the player (slot 0) keeps its health,
; carry set as when the damage leaves some.
wide_hurt:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BEQ     @hurt
    LDA     SLOT
    BNE     @hurt
    LDA     HEALTH
    SEC
    RTS
@hurt:
    LDA     HEALTH
    SEC
    SBC     DAMAGE
    RTS

; Damage the side view's and the overhead view's hazards take away (bank 6 $8605 and $89A7,
; bank 4 $8DE6, in place of EOR #$FF / CLC / ADC #$01 / CLC / ADC $53: the health less A): the
; player keeps its health, carry set as when the damage leaves some.
wide_lose:
    PHA
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BEQ     @lose
    LDA     SLOT
    BNE     @lose
    PLA
    LDA     HEALTH
    SEC
    RTS
@lose:
    PLA
    EOR     #$FF
    CLC
    ADC     #$01
    CLC
    ADC     HEALTH
    RTS

; A hit in the overhead view (bank 4 $8C94, in place of LDA $53 / SEC / SBC #$20): none for the
; player.
wide_hurt_32:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BEQ     @hurt
    LDA     SLOT
    BNE     @hurt
    LDA     HEALTH
    SEC
    RTS
@hurt:
    LDA     HEALTH
    SEC
    SBC     #$20
    RTS

; A fall (bank 6 $82C8, in place of the 17 bytes that take 64 from the health, or all of it past
; a speed of $40): none for the player.
wide_fall:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BEQ     @fall
    LDA     SLOT
    BEQ     @rts
@fall:
    LDA     FALL_SPEED
    CMP     #$40
    BCS     @dead
    LDA     HEALTH
    SEC
    SBC     #$40
    BCS     @set
@dead:
    LDA     #$00
@set:
    STA     HEALTH
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; Level select (option bit 0): a new game ($C2BC, in place of LDA #$08 / STA $14, area 1) starts
; with the game's own area select (up / down the area: 01-08 the areas, 09-16 their dungeons;
; Start), then goes on as a new game does past its opening scene (made for area 1: $C2C0
; JSR $CDBA is done here, JSR $F5D9 skipped, back at $C2C6).
wide_new_game:
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    BEQ     @first
    JSR     AREA_SELECT
    JSR     NEW_GAME_INIT
    PLA                                 ; not back to $C2C0
    PLA
    JMP     NEW_GAME_GO
@first:
    LDA     #$08
    STA     AREA
    RTS
```

### B. `tools/build.py`

```python
"""Builds the Blaster Master (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Blaster Master (USA).nes> [out.bps]
       python -I tools/build.py <Blaster Master (USA).nes> --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank the widescreen extension's PRG window shows at $5800-$7FFF (Zaxnes opens it at
   power-on for this patch: bank 8, page 0).
2. Lays the PRG-ROM out as 16 banks: the game's 8 where they were, the hack's bank 8, banks
   9-14 of $FF, and the game's fixed bank again as bank 15, the one MMC1 fixes at $C000 (Zaxnes'
   MMC1 fixes bank 15 modulo the bank count).
3. Writes the hooks into the game's banks, each checked against the bytes it replaces (those of
   the fixed bank into both its copies).
4. Writes build/blastermasterwide.nes (header: 16 PRG banks) and, if asked, the BPS patch (data
   without iNES header; the game's banks moved or copied come from the player's ROM).

With --pack, a Zaxnes hack pack, format 2, instead, written by Zaxnes' shared builder
(tools/hackpack.py of Zaxnes, its folder in the environment variable ZAXNES_TOOLS): NTSC, MMC1,
each hook a 16 KiB bank and a CPU address, the widescreen in one group ("core", Base); the code (its used part) for the
extension's memory, which Zaxnes shows in the window from power-on, and the hooks as offsets in the
PRG-ROM, the ROM keeping its size. The testing aids need the hack's code: they stay in the
"widescreen" component with the other hooks.
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x3F0FD764
BANK_SIZE = 0x4000
PRG_BANKS = 8
PRG_SIZE = PRG_BANKS * BANK_SIZE
CHR_SIZE = 0x20000
TARGET_BANKS = 16
HACK_BANK = 8
FIXED = 7
CODE_START = 0x5800

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

    # The frame's wait: the extension's settings.
    put(7, 0xE936, 'a9 80 85 12', jsr(sym['wide_frame']))
    # An object's place on the screen, and its sprites to OAM.
    put(7, 0xEF2B, 'a5 48 38', jmp(sym['wide_position']))
    put(7, 0xEC73, 'a5 3c f0', jmp(sym['wide_copy']))
    # Sleeping enemies: coming out of hiding (side view, overhead), the object loop, no new
    # object or sound while one runs.
    put(6, 0x8134, 'a6 57 bd 50 01', jsr(sym['wide_come_out']))
    put(4, 0x8B01, 'a6 57 bd 50 01', jsr(sym['wide_come_out']))
    put(7, 0xC987, '20 a4 c9', jsr(sym['wide_run']))
    put(7, 0xD7CF, 'bd 00 04', jmp(sym['wide_slot_search']))
    put(7, 0xDECC, '85 e1 8a', jmp(sym['wide_sound']))
    # The windows that grow with the margin: the list's spawning, a hidden enemy's removal.
    put(7, 0xCAA4, '29 7f 18 69 06 c9 1c', jsr(sym['wide_spawn_window']))
    put(6, 0x8141, '18 69 08 29 7f c9 20', jsr(sym['wide_hidden_far']))
    put(4, 0x8B0E, '18 69 08 29 7f c9 20', jsr(sym['wide_hidden_far']))
    # Invincibility: damage, a fall.
    put(7, 0xD6A2, 'a5 53 38 e5 45', jsr(sym['wide_hurt']))
    put(6, 0x82C8, 'a5 4d c9 40 b0 07 a5 53 38 e9 40 b0 02 a9 00 85 53', jsr(sym['wide_fall']))
    for bank, address in ((6, 0x8605), (6, 0x89A7), (4, 0x8DE6)):
        put(bank, address, '49 ff 18 69 01 18 65 53', jsr(sym['wide_lose']))
    put(4, 0x8C94, 'a5 53 38 e9 20', jsr(sym['wide_hurt_32']))
    # Level select: a new game.
    put(7, 0xC2BC, 'a9 08 85 14', jsr(sym['wide_new_game']))
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


def pack_place(offset, address):
    """The place of format 2 for `offset` in the PRG-ROM, run at `address`: its 16 KiB bank, and
    the address with the offset's place in that bank (an 8 KiB bank's half of it)."""
    return offset // 0x4000, (address & 0xC000) | (offset & 0x3FFF)


def pack(data, code, sym, out):
    tools = os.environ.get('ZAXNES_TOOLS')
    if not tools or not os.path.isfile(os.path.join(tools, 'hackpack.py')):
        sys.exit('ZAXNES_TOOLS: the rust/tools folder of Zaxnes (its hackpack.py) is needed')
    sys.path.insert(0, tools)
    import hackpack
    pack = hackpack.Pack('', b'', region='ntsc', mappers=(1,))
    core = pack.component('widescreen').group('core', label='Base', label_fr='Base')
    for hook in hooks(sym):
        bank, address, original, new = hook if len(hook) == 4 else (None, *hook)
        offset = bank * BANK_SIZE + address - (0xC000 if bank == FIXED else 0x8000)
        assert data[offset:offset + len(original)] == original, f'${address:04X}: not the expected bytes'
        core.hook(*pack_place(offset, address), original, bytes(new))
    # Zaxnes reads $FF past the code's end: its trailing fill is left out.
    used = (len(code.rstrip(b'\xff')) + 0xFF) & ~0xFF
    pack.name = 'Blaster Master (USA) widescreen'
    pack.code = code[:used]
    result = pack.build(data, 0x20000)
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes, pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = bytes(image[header:header + PRG_SIZE + CHR_SIZE])
    assert zlib.crc32(data) == SOURCE_CRC, 'not Blaster Master (USA)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        pack(bytes(data), code, sym, sys.argv[3])
        return
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
    out = os.path.join(ROOT, 'build', 'blastermasterwide.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        last = (TARGET_BANKS - 1) * BANK_SIZE
        moved = [(last, last + BANK_SIZE, FIXED * BANK_SIZE),
                 (TARGET_BANKS * BANK_SIZE, TARGET_BANKS * BANK_SIZE + CHR_SIZE, PRG_SIZE)]
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
