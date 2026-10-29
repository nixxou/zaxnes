# Mega Man 5 (USA): the widescreen hack

This document describes completely the modification of *Mega Man 5 (USA)* that Zaxnes ships as a
built-in patch, the advanced mode of its widescreen. The appendices hold the new code and
the build script, which writes every hook after checking the bytes it replaces: with them the ROM
can be rebuilt identically.

The background of the margins is not the hack's work: the emulator's Mega Man 5 margin decoder
reads the stage layout in PRG-ROM (see [`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md),
section 4); it widens the game on its own, the simple mode. The hack does the objects: enemies that
spawn, sleep, live and show in the margins, the way the Mega Man 4 hack does them.

At the screen's edges the decoder draws column 0 while the game hides it (the game keeps there the
column it is drawing) and redraws the screen's last column, which showed the colours of the column
being drawn past the right edge; its margins stay empty while the vertical scroll `$FA` is not 0,
and take no column from the nametables while both show the same 256 pixels. With the video setting
that hides the overscan, the decoder asks for the screen's top and bottom 8 lines to stay hidden
with a vertical margin too, and the vertical margin is drawn black: the game draws the rows coming
in at those edges a piece at a time while it scrolls to another section, which a television hid. The
picture then ends where it does without a vertical margin.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Mega Man 5 (USA)*, MMC3 (mapper 4), 256 KiB PRG (32 banks of 8 KiB, banks 30 and 31 fixed at `$C000` and `$E000`), 256 KiB CHR-ROM |
| Source data (iNES header excluded) | CRC-32 `3EDCF7E8` |
| Target data | CRC-32 `D23A8823` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | PRG grown to 288 KiB (36 banks of 8 KiB): the game's banks 0-31 where they were, the hack's code as banks 32-33 (16 KiB bank 16), the fixed banks copied to banks 34 and 35 (Zaxnes' MMC3 fixes the last two banks, whatever their count); hooks in banks `$1B`, `$1C`, 30 and 31 (both copies of the fixed banks) |
| Patch | `rust/crates/zax-nes/patches/megaman5-usa-widescreen.bps`, made against the data without header; the fixed banks' copies and the CHR-ROM are SourceCopy, the added banks never read from the source, no game data in the patch |

The code runs from the widescreen extension's PRG window at `$5800-$5FFF`, which Zaxnes opens on
bank 32, page 0 at power-on for this patch (`boot_window`): the game has no free room for code that
would unlock the extension. The window is not laid over `$6000-$7FFF` (`$5007` bit 4 off once the
hack has set it): the game's board may have PRG-RAM there. The game only ever selects banks below
32, so the added banks never show at `$8000` or `$A000`.

---

**Hack pack.** Zaxnes ships this hack as a hack pack, `rust/crates/zax-nes/patches/megaman5-usa-
widescreen.zxpk` (`tools/build.py <rom> --pack <out.zxpk>`; `docs/HACK-PACKS.md`, `WIDESCREEN-
EXTENSION.md` 7.1): the code goes to the extension's memory, which the window shows from power-on
(`$5008` = `$80`, page 0), and the hooks, each with the game's bytes it replaces, are checked and
applied by Zaxnes; the ROM keeps its size (no bank added, nothing moved or copied). The option
"Invincible" is a component of its own (`LDA #$02 / SEC` in place of the touch test's `JSR`),
applied with or without the advanced mode. The BPS patch described here is kept to compare.

## 2. What the game does, and what changes

As Mega Man 3's and 4's, the game runs as tasks under a scheduler; it waits for one to wake at
`$FEAE`-`$FEC3` (Zaxnes' idle loop for this patch). Each frame `$DF0C` hides all of OAM (`$C38F`),
then draws the objects (`$DF9E`, each through `$DFC3`) and the energy bars (`$E27E`), the next
sprite's offset in `$9F`. The NMI only updates the screen for a frame the game finished (`$95`
clear); otherwise it is a lag frame.

Objects: 24 slots, `$0300` type (0 free), `$0330`/`$0348` x pixel / screen, `$0378`/`$0390` y,
`$0438` the stage list's entry an enemy came from (`$FF` none), `$0528` drawing flags (bit 7 shown,
which the collisions take; bit 3 kept 48 pixels off the screen). Slot 0 is Mega Man, 1-7 his
weapons and the like, 8-23 the enemies. The camera is `$F9:$FC`; the section being played is
`$2A + 1` screens, the camera on its screen `$2B`; `$AC` bit 0 the camera's last move (right).
The stage list (bank `$1B`, the stage's data at `$A000`) is entries sorted by place: screen
`$AA00`, x `$AA80`, y `$AB00`, type `$AB80` (types `$C0` and up: palettes and the like); `$AD` and
`$AE` the next entries right and left of the spawn window (`$03:$02` to `$05:$04`).

| Game routine | Original | Hack |
|---|---|---|
| spawn window (bank `$1B` `$988A`) | entries between the camera and camera + 255 are taken | between camera - M and camera + 255 + M, within the section |
| an entry of the list (bank `$1B` `$990C`) | another type (`$C0` up) or an enemy | another type still waits for the original window, camera to camera + 255; until then the pass stops on it (the entries past it wait too) |
| a list enemy's spawn (bank `$1B` `$9A12`) | | it sleeps when its place is off the original screen |
| object loop (bank `$1C` `$8004`, `LDY $0300,X / BEQ` next) | runs every object's routine, then an enemy's collisions | an enemy asleep is skipped while off the original screen; on it, it wakes for good |
| an object's place (`$DFCB`, in `$DFC3`) | an object whose x - camera is off the screen is removed (or, bit 3, kept unseen up to 48 pixels away; some types kept) | the same for slots 0-7; an enemy shows while its x - camera is within `[LO, HI)` |
| an object's sprites (`$E19B`) | x = 8-bit screen x + signed offset (mirrored when flipped), the sprite hidden when it carries; the loop stops when OAM is full | 16-bit x, shown within `[-M, 256 + M)`, its high byte to `$5100 + 4 n` (or `$5400 + 4 n`); the loop goes on to the extension's second page (`$5300`) when the first is full, and stops when both are; the flips as the original |
| energy bars (`$E2A1`) | write to OAM from `$9F` | write to the page in use |
| OAM hidden (`$C391`) | from offset X to the end | the same, then the second page and every high byte, back to the first page |
| collisions (`$EF95` Mega Man against an object, `$F002` his weapons against an enemy) | an object shown (`$0528` bit 7) | an object off the original screen is not shown for them: nothing in a margin touches or is touched |
| an enemy's touch (bank `$1C` `$8092`, `JSR $EF87`) | | none with the option "Invincible" |

`LO = max(-M, section start - camera)`, `HI = min(256 + M, section end - camera)`; M is the margin.
Mega Man's weapons (slots 0-7) keep the original limits. An object the game keeps off
the screen is drawn with the low byte of its screen x alone, as the original (Mega Man while the
game scrolls to another section). With M = 0 every routine does exactly what
the code it replaces does.

Margin: `M = min($5002, 248)` (248: the widest the extension's 8-bit registers carry; sprites show
that far with `$5007` bit 1).

CPU time: 64 lines of overclock (`$5006`) pay for the hack's work and the enemies of the margins;
255 with the option "No slowdown" (on by default). Measured on Charge Man's stage (500 frames,
holding right and jumping, counted with a temporary trace of the NMI's updates): the hack at a
margin of 0 without overclock (the original's work and the hooks') runs late 50 frames; at a
128-pixel margin with 64 lines 23, with 255 none.

## 3. The extension

Set every frame the NMI updates the screen (`$C01A`, `LDA $FA / STA $A2` becomes `JSR wide_nmi /
NOP`, which writes `$5003 = M`, `$5004 = $5005 = 0` (the nametable columns are hidden: the decoder
draws the background), `$5007 = $0E` (sprites free of those extents, second sprite page, high bytes
kept from one page to the next: the pages and their high bytes are rebuilt together from `$C391`
every frame), `$5006 = 64` (255 with "No slowdown"), then does the replaced instructions).

Variables (extension RAM): `WV_M $5200`, the live range `WV_LO $5201`/`$5202` and `WV_HI
$5203`/`$5204` (signed 16-bit), the object's screen x `WV_S $5205`/`$5206`, scratch `WV_T0 $5207`,
`WV_PAGE $5209` (bit 7: the second page is in use), `WV_FULL $520A` (both pages full), `WV_Y $520C`
(the sprite's y), `WV_DORMANT $5220 + slot` (the list entry of an enemy asleep, `$FF` awake: a slot
another object took since is awake).

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length. Banks 30 and
31 are the fixed ones (`$C000`, `$E000`), written into both their copies (30 and 34, 31 and 35);
banks `$1B` and `$1C` run at `$8000`.

| Address | Replaces | With |
|---|---|---|
| `$C01A` | `LDA $FA / STA $A2` | `JSR wide_nmi` |
| `$C391` | `LDA #$F8 / STA $0200,X` (head of the loop hiding OAM) | `JMP wide_hide_rest` |
| `$DFCB` | `LDA $0330,X / SEC / SBC $FC / STA $13 / LDA $0348,X / SBC $F9 / BEQ $DFF9` | `JSR wide_place / BEQ $DFF9` |
| `$E19B` | `LDX $9F / BEQ $E1FF` (head of an object's sprite loop) | `JMP wide_draw_loop` |
| `$E2A1` | `LDX $9F / BEQ $E2DE` (head of the energy bars' sprites) | `JMP wide_bars` |
| `$EF95`, `$F002` | `LDA $0528,X` | `JSR wide_touchable` |
| bank `$1B` `$988A` | `CLC / LDA $FC / STA $02 / ADC #$FF / STA $04 / LDA $F9 / STA $03 / ADC #0 / STA $05` | `JSR wide_spawn_window` |
| bank `$1B` `$990C` | `LDA $AB80,Y / CMP #$C0 / BCS $9916` | `JSR wide_entry / BCS $9916` |
| bank `$1B` `$9A12` | `JSR $EA1E` | `JSR wide_spawned` |
| bank `$1C` `$8004` | `LDY $0300,X` | `JSR wide_think` |
| bank `$1C` `$8092` | `JSR $EF87` | `JSR wide_touch` |

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there (`WIDE_OPTIONS`).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 1 | `invincible` | Invincible | off | a testing aid: no enemy touch hurts Mega Man or knocks him back (pits and spikes still kill) |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of 64 |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `MEGAMAN4_OPTIONS`, shared
with Mega Man 4); the ROM does not depend on them being set.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Mega Man 5 (USA).nes" build/mm5wide.bps
```

assembles `src/wide.s` for `$5800` (one 16 KiB bank, the code within its first 2 KiB, `-g`, labels
from `ld65 -Ln`), lays the PRG out in 36 banks of 8 KiB, writes the hooks, `build/mm5wide.nes` and
the BPS patch (`tools/make_bps.py`: the fixed banks' copies and the CHR-ROM are SourceCopy from the
player's ROM, the added banks and the hooks TargetRead). `WIDE_SKIP=<hex addresses>` leaves hooks
out, for testing.

## 7. Limits

* No sprite in the vertical margins: the extension carries a high byte for x only.
* Only the enemies (slots 8 up) live in the margins; Mega Man's weapons keep the original screen.
* An enemy asleep in a margin hangs where its entry puts it until the original screen reaches it.
* Near another entry of the stage list (a palette change and the like), the enemies past it spawn
  at the original edge.
* The overclock removes lag frames, so a replay of the original's inputs frame for frame may drift
  from the original.

---

## Appendices

### A. `src/wide.s`

```asm
; Mega Man 5 (USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added to the game (PRG bank 16 of 18 in 16 KiB units: the game's
; 8 KiB banks 0-31 stay where they were and its two fixed banks are copied to banks 34 and 35, the
; ones MMC3 fixes at $C000 and $E000); the widescreen extension's PRG window shows its first 2 KiB
; at $5800-$5FFF, Zaxnes opening it at power-on (the patch's boot window: bank 16, page 0). The
; window is not laid over $6000-$7FFF ($5007 bit 4 off): the game may have PRG-RAM there. The game
; reaches the code through hooks in its own (tools/build.py). The background of the margins is
; Zaxnes' margin decoder's work; the hack does the objects, the way the Mega Man 4 hack does them:
;
;  * an enemy of the stage's list (object slots 8-23) spawns when it comes within the margin of
;    the screen, and lives and shows until it leaves it, all within the section being played;
;  * one that spawns in a margin sleeps there: it shows but does not move until it comes onto the
;    original screen; awake, it stays awake;
;  * the list's other entries (types $C0 and up: palettes and the like) still wait for the
;    original screen;
;  * every object's sprites get the high byte of their 16-bit x, and the sprites that no longer
;    fit in the console's 64 go on to the extension's second page instead of being dropped;
;  * an object outside the original screen touches nothing and nothing touches it;
;  * Mega Man and his weapons (slots 0-7) keep the original limits.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1 sprites free of the extents, bit 2 second sprite page,
                                ; bit 3 high bytes kept
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA
EXTRA_OAM       = $5300         ; the second sprite page, OAM format
EXTRA_SPRHI     = $5400         ; + 4 n: high byte of the x of its sprite n

OPT_INVINCIBLE  = $02           ; a testing aid: Mega Man takes no damage
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late

MAX_MARGIN      = 248           ; the most the extension shows sprites in
OVERCLOCK_LINES = 64            ; pays for the hack's work and the enemies of the margins
NO_SLOWDOWN_LINES = 255
FIRST_ENEMY     = $08           ; the slots the stage's list spawns into

; Variables in the extension's RAM ($5200-$57FF).
WV_M            = $5200         ; margin of this frame
WV_LOL          = $5201         ; screen x range objects may live in: [LO, HI), signed 16-bit
WV_LOH          = $5202
WV_HIL          = $5203
WV_HIH          = $5204
WV_SL           = $5205         ; screen x of the object at hand, signed 16-bit
WV_SH           = $5206
WV_T0           = $5207         ; scratch
WV_PAGE         = $5209         ; bit 7: the sprites being placed go to the second page
WV_FULL         = $520A         ; non-zero once both pages are full
WV_X            = $520B         ; X, kept
WV_Y            = $520C         ; the y of the sprite at hand
WV_DORMANT      = $5220         ; + slot: the list entry ($0438 + slot) of an enemy asleep, $FF awake

; The game.
OAM             = $0200
CAM_LO          = $FC           ; the camera's x: pixel and screen
CAM_HI          = $F9
SECTION_LENGTH  = $2A           ; the section being played: $2A + 1 screens, the camera on its
SECTION_SCREEN  = $2B           ; screen $2B
DIRECTION       = $AC           ; bit 0: the camera last moved right
OBJ_TYPE        = $0300         ; + slot: 0 free
OBJ_X           = $0330         ; + slot: x, pixel and screen
OBJ_SCREEN      = $0348
OBJ_ENTRY       = $0438         ; + slot: the stage list's entry an enemy came from ($FF none)
OBJ_DRAW        = $0528         ; + slot: bit 7 shown (the collisions take it), bit 4 in the world
OAM_NEXT        = $9F           ; where the next sprite goes in OAM
LIST_SCREEN     = $AA00         ; the stage list (bank $1B, the stage's data at $A000): screen,
LIST_X          = $AA80         ; x
LIST_TYPE       = $AB80         ; and type of each entry
WINDOW_LEFT     = $02           ; the spawner's window: camera ($03:$02) to camera + 255 ($05:$04)
WINDOW_RIGHT    = $04

; The game's routines.
TOUCH_TEST      = $EF87         ; Mega Man against object X: carry clear when they touch
SPAWN_SETUP     = $EA1E         ; the end of a list enemy's spawn
SPAWN_RIGHT_DONE = $98F2        ; bank $1B: the spawner's pass going right, done
SPAWN_LEFT_DONE = $98BD         ; bank $1B: its pass going left, done

.segment "CODE"
.org $5800

; ---------------------------------------------------------------------------- ;
; NMI ($C01A, in place of LDA $FA / STA $A2, the update of a frame the game finished): the
; extension's settings for the next frame.
wide_nmi:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    LDA     #$00
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    LDA     #$0E                        ; sprites free of the extents, second sprite page, high
                                        ; bytes kept (every sprite placed in a margin gets one)
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
    LDA     $FA
    STA     $A2
    RTS

; ---------------------------------------------------------------------------- ;
; The screen x of object X (its x minus the camera's), into WV_SL/WV_SH.
wide_screen_x:
    SEC
    LDA     OBJ_X,x
    SBC     CAM_LO
    STA     WV_SL
    LDA     OBJ_SCREEN,x
    SBC     CAM_HI
    STA     WV_SH
    RTS

; Whether enemy X lives: carry clear when its screen x (WV_S, set) is within
;   [max(-M, section start - camera), min(256 + M, section end - camera)).
wide_inside:
    SEC                                 ; LO = section start - camera
    LDA     #$00
    SBC     CAM_LO
    STA     WV_LOL
    LDA     #$00
    SBC     SECTION_SCREEN
    STA     WV_LOH
    CLC                                 ; LO + M < 0: LO = -M
    LDA     WV_LOL
    ADC     WV_M
    LDA     WV_LOH
    ADC     #$00
    BPL     @hi
    SEC
    LDA     #$00
    SBC     WV_M
    STA     WV_LOL
    LDA     #$00
    SBC     #$00
    STA     WV_LOH
@hi:
    SEC                                 ; HI = section end - camera
    LDA     #$00
    SBC     CAM_LO
    STA     WV_HIL
    LDA     SECTION_LENGTH
    SBC     SECTION_SCREEN
    CLC
    ADC     #$01
    STA     WV_HIH
    SEC                                 ; HI - 256 > M: HI = 256 + M
    LDA     WV_HIL
    SBC     WV_M
    LDA     WV_HIH
    SBC     #$01
    BMI     @test
    BNE     @cap
    LDA     WV_HIL
    CMP     WV_M
    BCC     @test
    BEQ     @test
@cap:
    LDA     WV_M
    STA     WV_HIL
    LDA     #$01
    STA     WV_HIH
@test:
    SEC                                 ; S - LO >= 0
    LDA     WV_SL
    SBC     WV_LOL
    LDA     WV_SH
    SBC     WV_LOH
    BVC     :+
    EOR     #$80
:   BMI     @outside
    SEC                                 ; S - HI < 0
    LDA     WV_SL
    SBC     WV_HIL
    LDA     WV_SH
    SBC     WV_HIH
    BVC     :+
    EOR     #$80
:   BPL     @outside
    CLC
    RTS
@outside:
    SEC
    RTS

; ---------------------------------------------------------------------------- ;
; An object's place ($DFCB, in place of $13 = x - camera and "BEQ on the screen"; then BEQ
; $DFF9): Z set when it shows (the original: on the screen; an enemy: within the margins too),
; carry set when it is right of the camera's screen (the original's test of $DFE3 on an object
; off the screen).
wide_place:
    JSR     wide_screen_x
    LDA     WV_SL
    STA     $13
    LDA     WV_SH
    BEQ     @rts                        ; on the screen
    CPX     #FIRST_ENEMY
    BCC     @off
    JSR     wide_inside
    BCS     @off
    LDA     #$00                        ; an enemy in a margin
@rts:
    RTS
@off:
    LDA     WV_SH                       ; off: C set when right of the camera, Z clear; drawn
    EOR     #$80                        ; (an object the game keeps) with the low byte of its
    CMP     #$80                        ; screen x alone, as the original
    LDA     #$00
    STA     WV_SH
    LDA     #$01
    RTS

; Whether object X can touch or be touched ($EF95 and $F002, in place of LDA $0528,X; then BPL:
; no): not while it is off the original screen.
wide_touchable:
    JSR     wide_screen_x
    LDA     WV_SH
    BNE     @no
    LDA     OBJ_DRAW,x
    RTS
@no:
    LDA     #$00
    RTS

; ---------------------------------------------------------------------------- ;
; Sleeping enemies. An enemy of the stage list that spawns in a margin shows but does not move
; until it comes onto the original screen; from then on it lives as the original does, margins
; included.

; The end of a list enemy's spawn (bank $1B $9A12, in place of JSR $EA1E; X the slot): it sleeps
; when it spawned off the original screen.
wide_spawned:
    JSR     SPAWN_SETUP
    JSR     wide_screen_x
    LDA     WV_SH
    BEQ     @awake
    LDA     OBJ_ENTRY,x
    JMP     @set
@awake:
    LDA     #$FF
@set:
    STA     WV_DORMANT,x
    RTS

; The object loop (bank $1C $8004, in place of LDY $0300,X; then BEQ: next slot): an enemy asleep
; skips its frame (its routine and its collisions) while off the original screen; on it, it wakes.
; Y the object's type, Z set for none.
wide_think:
    CPX     #FIRST_ENEMY
    BCC     @run
    LDA     WV_DORMANT,x
    CMP     #$FF
    BEQ     @run
    CMP     OBJ_ENTRY,x                 ; another object took the slot since: awake
    BNE     @wake
    JSR     wide_screen_x
    LDA     WV_SH
    BEQ     @wake
    LDY     #$00                        ; asleep: as if the slot were free
    RTS
@wake:
    LDA     #$FF
    STA     WV_DORMANT,x
@run:
    LDY     OBJ_TYPE,x
    RTS

; ---------------------------------------------------------------------------- ;
; The spawn window (bank $1B $988A, in place of $03:$02 = camera, $05:$04 = camera + 255): the
; stage list (entries sorted by place) spawns the enemies between camera - M and camera + 255 + M,
; within the section being played.
wide_spawn_window:
    SEC                                 ; left = camera - M
    LDA     CAM_LO
    SBC     WV_M
    STA     WINDOW_LEFT
    LDA     CAM_HI
    SBC     #$00
    STA     WINDOW_LEFT+1
    BCC     @first                      ; before the level
    LDA     CAM_HI                      ; the section's first screen
    SEC
    SBC     SECTION_SCREEN
    STA     WV_T0
    LDA     WINDOW_LEFT+1
    CMP     WV_T0
    BCS     @right
@first:
    LDA     #$00                        ; before the section: its start
    STA     WINDOW_LEFT
    LDA     CAM_HI
    SEC
    SBC     SECTION_SCREEN
    STA     WINDOW_LEFT+1
@right:
    CLC                                 ; right = camera + 255 + M
    LDA     CAM_LO
    ADC     #$FF
    STA     WINDOW_RIGHT
    LDA     CAM_HI
    ADC     #$00
    STA     WINDOW_RIGHT+1
    CLC
    LDA     WINDOW_RIGHT
    ADC     WV_M
    STA     WINDOW_RIGHT
    LDA     WINDOW_RIGHT+1
    ADC     #$00
    STA     WINDOW_RIGHT+1
    LDA     CAM_HI                      ; the section's last screen
    SEC
    SBC     SECTION_SCREEN
    CLC
    ADC     SECTION_LENGTH
    STA     WV_T0
    LDA     WINDOW_RIGHT+1
    CMP     WV_T0
    BCC     @rts
    BEQ     @rts
    LDA     #$FF                        ; past the section: its last pixel
    STA     WINDOW_RIGHT
    LDA     WV_T0
    STA     WINDOW_RIGHT+1
@rts:
    RTS

; An entry of the list (bank $1B $990C, in place of LDA $AB80,Y / CMP #$C0; then BCS: not an
; enemy; Y the entry): one of the other types (palettes and the like) still waits for the original
; window, camera to camera + 255; until then neither it nor the entries past it are taken (back
; to the end of the spawner's pass, the pass's index left on it).
wide_entry:
    LDA     LIST_TYPE,y
    CMP     #$C0
    BCC     @rts
    LDA     LIST_X,y                    ; entry - camera, 16-bit
    SEC
    SBC     CAM_LO
    LDA     LIST_SCREEN,y
    SBC     CAM_HI
    BNE     @wait                       ; not within camera to camera + 255
    LDA     LIST_TYPE,y
    SEC
@rts:
    RTS
@wait:
    PLA                                 ; out of this routine and the spawn routine ($990A)
    PLA
    PLA
    PLA
    LDA     DIRECTION
    AND     #$01
    BEQ     @left
    JMP     SPAWN_RIGHT_DONE
@left:
    JMP     SPAWN_LEFT_DONE

; ---------------------------------------------------------------------------- ;
; Sprites. The console has 64; the extension a second page of 64 at $5300, which the game's
; sprite loops move on to when the first is full, instead of stopping.

; $C391, in place of the loop that hides the sprites from offset X: that, then the second page and
; every high byte (kept from one page to the next: the pages and their high bytes are rebuilt
; together, every frame from here), back to the first page.
wide_hide_rest:
    LDA     #$F8
@game:
    STA     OAM,x
    INX
    INX
    INX
    INX
    BNE     @game
@both:
    LDA     #$00
    STA     WIDE_SPRHI,x
    STA     EXTRA_SPRHI,x
    LDA     #$F8
    STA     EXTRA_OAM,x
    INX
    INX
    INX
    INX
    BNE     @both
    STX     WV_PAGE
    STX     WV_FULL
    RTS

; Moves on to the second page once the first is full (X wrapped to 0); sets WV_FULL once both
; are. Z clear while there is room. X the new offset.
.macro NEXT_PAGE
    BIT     WV_PAGE
    BPL     :+
    INC     WV_FULL
    JMP     :++
:   LDA     #$80
    STA     WV_PAGE
:
.endmacro

; Stores A at byte k of the sprite at offset X of the page in use (flags N and V change).
.macro PUT k
    BIT     WV_PAGE
    BMI     :+
    STA     OAM+k,x
    JMP     :++
:   STA     EXTRA_OAM+k,x
:
.endmacro

; The sprite loop of one object ($E19B, in place of LDX $9F / BEQ: OAM full), the original's, on
; the page in use, the x in 16 bits: a sprite whose x is past the margins is hidden; one in a
; margin gets the high byte of its x. Y the index in the tile list ($02) and the position list
; ($05), $04 sprites less one, $12 the object's y, $13 and WV_SH its screen x, $10 its flips
; (bit 7 vertical, bit 6 horizontal), $11 its priority.
wide_draw_loop:
    LDA     WV_FULL
    BEQ     :+
    RTS
:   LDX     OAM_NEXT
@sprite:
    INY
    LDA     ($02),y                     ; tile
    PUT     1
    LDA     ($05),y                     ; y offset, mirrored when flipped
    STA     $07
    LDA     $10
    BPL     @y
    LDA     #$F8
    SEC
    SBC     $07
    STA     $07
@y:
    LDA     $12
    CLC
    ADC     $07
    STA     WV_Y
    LDA     $07
    BMI     @up
    BCC     @y_ok
    BCS     @far_y
@up:
    BCS     @y_ok
@far_y:
    JMP     @hide_y
@y_ok:
    LDA     WV_Y
    PUT     0
    INY
    LDA     ($02),y                     ; attributes
    EOR     $10
    ORA     $11
    PUT     2
    LDA     ($05),y                     ; x offset, mirrored when flipped
    STA     $07
    LDA     $10
    AND     #$40
    BEQ     @x
    LDA     #$F8
    SEC
    SBC     $07
    STA     $07
@x:
    LDA     $07                         ; x = S + signed offset
    CLC
    BMI     @neg
    ADC     $13
    STA     WV_T0
    LDA     WV_SH
    ADC     #$00
    JMP     @high
@neg:
    ADC     $13
    STA     WV_T0
    LDA     WV_SH
    ADC     #$FF
@high:
    BEQ     @show                       ; 0 <= x < 256
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hide
    PHA
    LDA     WV_T0                       ; x < 0: x + M >= 0?
    CLC
    ADC     WV_M
    PLA
    BCC     @hide
    BCS     @show
@right:
    PHA
    LDA     WV_T0                       ; 256 <= x: below 256 + M?
    CMP     WV_M
    PLA
    BCS     @hide
@show:
    BIT     WV_PAGE
    BMI     @x2
    STA     WIDE_SPRHI,x
    LDA     WV_T0
    STA     OAM+3,x
    JMP     @next
@x2:
    STA     EXTRA_SPRHI,x
    LDA     WV_T0
    STA     EXTRA_OAM+3,x
@next:
    INX
    INX
    INX
    INX
    STX     OAM_NEXT
    BNE     @more
    NEXT_PAGE
    LDA     WV_FULL
    BNE     @rts
@more:
    DEC     $04
    BMI     @rts
    JMP     @sprite
@rts:
    RTS
@hide_y:
    INY
@hide:
    LDA     #$F8
    PUT     0
    JMP     @more

; The energy bars ($E2A1, the rest of the routine, in place of LDX $9F / BEQ: OAM full): $00 the
; energy, $01 the attributes, $02 the x; on the page in use.
wide_bars:
    LDA     WV_FULL
    BNE     @rts
    LDX     OAM_NEXT
    LDA     #$48
    STA     $03
@sprite:
    BIT     WV_PAGE
    BMI     @second
    LDA     $01
    STA     OAM+2,x
    LDA     $02
    STA     OAM+3,x
    LDA     $03
    STA     OAM,x
    JSR     @tile
    STA     OAM+1,x
    JMP     @next
@second:
    LDA     $01
    STA     EXTRA_OAM+2,x
    LDA     $02
    STA     EXTRA_OAM+3,x
    LDA     $03
    STA     EXTRA_OAM,x
    JSR     @tile
    STA     EXTRA_OAM+1,x
@next:
    INX
    INX
    INX
    INX
    STX     OAM_NEXT
    BNE     @step
    NEXT_PAGE
    LDA     WV_FULL
    BNE     @rts
@step:
    LDA     $03
    SEC
    SBC     #$08
    STA     $03
    CMP     #$10
    BNE     @sprite
@rts:
    RTS
@tile:                                  ; the tile of the next 4 points of energy
    LDY     #$04
    LDA     $00
    SEC
    SBC     #$04
    BCS     :+
    LDY     $00
    LDA     #$00
:   STA     $00
    TYA
    ORA     #$60
    RTS

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1, a testing aid): Mega Man takes no damage.

; Bank $1C $8092, in place of JSR $EF87 (an enemy touches Mega Man): invincible, never.
wide_touch:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BEQ     @test
    SEC
    RTS
@test:
    JMP     TOUCH_TEST

wide_end:
.assert wide_end <= $6000, error, "widescreen code past the 2 KiB window"
```

### B. `tools/build.py`

```python
"""Builds the Mega Man 5 (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Mega Man 5 (USA).nes> [out.bps]
       python -I tools/build.py <Mega Man 5 (USA).nes> --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank whose first 2 KiB the widescreen extension's PRG window shows at $5800-$5FFF (Zaxnes
   opens it at power-on for this patch: bank 16, page 0).
2. Lays the PRG-ROM out as 36 banks of 8 KiB: the game's 32 where they were, the hack's 16 KiB as
   banks 32-33, and the game's fixed banks 30 and 31 again as banks 34 and 35, the ones MMC3
   fixes at $C000 and $E000 (Zaxnes' MMC3 takes the last two banks, whatever their count).
3. Writes the hooks into the game's banks, each checked against the bytes it replaces (those of
   the fixed banks into both their copies).
4. Writes build/mm5wide.nes (header: 18 PRG banks of 16 KiB) and, if asked, the BPS patch (data
   without iNES header; the game's banks copied and the CHR-ROM come from the player's ROM).

With --pack, the layout and the patched ROM give way to a Zaxnes hack pack (tools/make_pack.py):
the code (its used part), and the hooks as offsets in the game's data, the ROM keeping its size.
The invincibility hook is a component of its own, "invincible" (`LDA #$02 / SEC` for the `JSR` of
the touch test: no touch), applied by Zaxnes while that option is on, with or without the
widescreen; every other hook is the "widescreen" component.
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCES = {0x3EDCF7E8: 'mm5wide'}
BANK_SIZE = 0x2000
PRG_BANKS = 32
PRG_SIZE = PRG_BANKS * BANK_SIZE
CHR_SIZE = 0x40000
TARGET_BANKS = 36
HACK_BANK = 32
FIXED = (30, 31)
CODE_START = 0x5800
CODE_SIZE = 0x4000

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOP = 0xEA


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def hooks(sym):
    """The patches: (bank, address, original bytes, new bytes); banks 30 and 31 are the fixed
    ones ($C000, $E000), the others are seen at $8000."""
    out = []

    def put(bank, address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((bank, address, original, new))

    # The NMI: the extension's settings.
    put(30, 0xC01A, 'a5 fa 85 a2', jsr(sym['wide_nmi']))
    # Sprites: OAM hidden (and the second page), an object's place, its sprite loop, the energy
    # bars.
    put(30, 0xC391, 'a9 f8 9d 00 02', jmp(sym['wide_hide_rest']))
    put(30, 0xDFCB, 'bd 30 03 38 e5 fc 85 13 bd 48 03 e5 f9 f0 1f',
        jsr(sym['wide_place']) + bytes([0xF0, 0x29]))          # BEQ $DFF9
    put(31, 0xE19B, 'a6 9f f0 60', jmp(sym['wide_draw_loop']))
    put(31, 0xE2A1, 'a6 9f f0 39', jmp(sym['wide_bars']))
    # Collisions: not with an object off the original screen.
    put(31, 0xEF95, 'bd 28 05', jsr(sym['wide_touchable']))
    put(31, 0xF002, 'bd 28 05', jsr(sym['wide_touchable']))
    # The spawn window, the list's other entries, a list enemy's spawn, the object loop
    # (sleeping enemies).
    put(0x1B, 0x988A, '18 a5 fc 85 02 69 ff 85 04 a5 f9 85 03 69 00 85 05',
        jsr(sym['wide_spawn_window']))
    put(0x1B, 0x990C, 'b9 80 ab c9 c0 b0 03', jsr(sym['wide_entry']) + bytes([0xB0, 0x05]))  # BCS $9916
    put(0x1B, 0x9A12, '20 1e ea', jsr(sym['wide_spawned']))
    put(0x1C, 0x8004, 'bc 00 03', jsr(sym['wide_think']))
    # Invincibility: an enemy's touch.
    put(0x1C, 0x8092, '20 87 ef', jsr(sym['wide_touch']))
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
        f'MEMORY {{ CODE: start = ${CODE_START:04X}, size = ${CODE_SIZE:04X}, '
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


# The hook of the option "Invincible": a component of the pack of its own.
INVINCIBLE_HOOK = (0x1c, 0x8092)


def pack(data, code, sym, out):
    sys.path.insert(0, os.path.join(ROOT, 'tools'))
    import make_pack
    widescreen = []
    invincible = []
    for bank, address, original, new in hooks(sym):
        offset = bank * BANK_SIZE + (address & (BANK_SIZE - 1))
        assert data[offset:offset + len(original)] == original, \
            f'bank {bank:02X} ${address:04X}: not the expected bytes'
        if (bank, address) == INVINCIBLE_HOOK:
            # LDA #$02 / SEC: A and the flags as wide_touch leaves them.
            invincible.append((offset, original, bytes([0xA9, 0x02, 0x38])))
        else:
            widescreen.append((offset, original, bytes(new)))
    assert len(invincible) == 1
    used = (sym['wide_end'] - CODE_START + 0xFF) & ~0xFF
    result = make_pack.make('Mega Man 5 (USA) widescreen', code[:used],
                            [('widescreen', widescreen), ('invincible', invincible)])
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes, pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = bytes(image[header:header + PRG_SIZE + CHR_SIZE])
    name = SOURCES.get(zlib.crc32(data))
    assert name, 'not Mega Man 5 (USA)'

    code, sym = assemble()
    assert len(code) == CODE_SIZE
    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        pack(data, code, sym, sys.argv[3])
        return
    banks = [bytearray(data[n * BANK_SIZE:(n + 1) * BANK_SIZE]) for n in range(PRG_BANKS)]
    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for bank, address, original, new in hooks(sym):
        if address in skip:
            continue
        at = address & (BANK_SIZE - 1)
        assert banks[bank][at:at + len(original)] == original, \
            f'bank {bank:02X} ${address:04X}: not the expected bytes'
        banks[bank][at:at + len(new)] = new
    layout = (banks + [bytearray(code[:BANK_SIZE]), bytearray(code[BANK_SIZE:])]
              + [banks[FIXED[0]], banks[FIXED[1]]])
    assert len(layout) == TARGET_BANKS
    target = b''.join(bytes(b) for b in layout) + data[PRG_SIZE:]

    out_header = bytearray(image[:header])
    if header:
        out_header[4] = TARGET_BANKS * BANK_SIZE // 0x4000
    out = os.path.join(ROOT, 'build', name + '.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        fixed = (TARGET_BANKS - 2) * BANK_SIZE
        moved = [(fixed, fixed + 2 * BANK_SIZE, FIXED[0] * BANK_SIZE),
                 (TARGET_BANKS * BANK_SIZE, TARGET_BANKS * BANK_SIZE + CHR_SIZE, PRG_SIZE)]
        added = (PRG_SIZE, fixed)
        patch = make_bps.make(data, target, moved, [added])
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


def make(source, target, moved=(), fresh=()):
    """`moved`: (target start, target end, source start) of data the build moved; `fresh`:
    (target start, target end) of bytes the build made (the added banks), never read from the
    source even where a byte happens to match."""
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
            if any(start <= at < end for start, end in fresh):
                return False
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
