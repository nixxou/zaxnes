# Mega Man (USA): the widescreen hack

This document describes completely the modification of *Mega Man (USA)* that Zaxnes ships as a
built-in patch, the advanced mode of its widescreen. The appendices hold the new code and the build
script, which writes every hook after checking the bytes it replaces: with them the ROM can be
rebuilt identically.

The background of the margins is not the hack's work: the emulator's Mega Man margin decoder reads
the stage layout in PRG-ROM (see [`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md),
section 4); it widens the game on its own, the simple mode. The hack does the objects: enemies that
spawn, sleep, live and show in the margins, the way the Mega Man 2 hack does them.

At the screen's edges the decoder keeps to the section being played: past it, and while the game
scrolls to another section (`$26` non-zero), the margins stay empty. With the video setting that
hides the overscan, the decoder asks for the screen's top and bottom 8 lines to stay hidden with a
vertical margin too, and the vertical margin is drawn black: the game draws the rows coming in at
those edges a piece at a time while it scrolls to another section, which a television hid. The
picture then ends where it does without a vertical margin.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Mega Man (USA)*, UxROM (mapper 2), 128 KiB PRG (8 banks of 16 KiB, bank 7 fixed at `$C000`), CHR-RAM |
| Source data (iNES header excluded) | CRC-32 `6EE4BB0A` |
| Target data | CRC-32 `164D836E` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | PRG grown to 256 KiB (16 banks): the game's banks 0-7 where they were, the hack's code as bank 8, banks 9-14 of `$FF`, the fixed bank copied to bank 15 (the one UxROM fixes at `$C000`); hooks in bank 5 and bank 7 (both copies of the fixed bank) |
| Patch | `rust/crates/zax-nes/patches/megaman1-usa-widescreen.bps`, made against the data without header; the fixed bank's copy is SourceCopy, no game data in the patch |

The code runs from the widescreen extension's PRG window at `$5800-$7FFF` (`$5007` bit 4), which
Zaxnes opens on bank 8, page 0 at power-on for this patch (`boot_window`): the game has no free
room for code that would unlock the extension (224 bytes in its fixed bank). The game only ever
selects banks 0-6 (through the bus-conflict table at `$C000`), so the added banks never show at
`$8000`.

---

**Hack pack.** Zaxnes ships this hack as a hack pack,
`rust/crates/zax-nes/patches/megaman1-usa-widescreen.zxpk` (`tools/build.py <rom> --pack
<out.zxpk>`, with `ZAXNES_TOOLS` set to Zaxnes' `rust/tools`; `docs/HACK-PACKS.md`,
`WIDESCREEN-EXTENSION.md` 7.1). The pack is format 2, written by Zaxnes' shared builder
(`hackpack.py`): NTSC, UxROM, each hook a 16 KiB bank and a CPU address, the widescreen in one group
("core", Base); no checks yet. The code goes to the extension's
memory, which the window shows from power-on (`$5008` = `$80`, page 0), and the hooks, each with
the game's bytes it replaces, are checked and applied by Zaxnes; the ROM keeps its size (no bank
added, nothing moved or copied). The option "Invincible" is a component of its own (`LDA #$A2 /
RTS` in place of the health subtraction), applied with or without the advanced mode. The BPS patch
described here is kept to compare.

## 2. What the game does, and what changes

The main loop (bank 5 `$915E`) ends each frame in a wait (`$C01B`: `$1F = 0`, then `LDA $1F / BEQ`
at `$C02B` until the NMI sets it; Zaxnes' idle loop for this patch). An NMI that finds `$1F` still
set (the frame's work not done) only plays the sound (`$D4AD`): a lag frame, which the frame
counter `$23` does not count.

Objects: 32 slots, `$0400` type (`$FF` an enemy of the stage list, run by bank 5 `$AA29`), `$0420`
flags, `$0460`/`$0480` x screen / pixel, `$0600` y (`$F8`: the slot is free), `$6B + slot` the
stage list's entry an enemy came from (`$FF` none). Slot 0 is Mega Man, 1-15 his shots and the
like, `$10`-`$1F` the enemies (`$C576` searches a free slot from `$10`). The camera is `$1B:$1A`;
the section being played spans screens `$27` to `$28`. The stage list (bank 6, `$A452 + 2 stage`)
holds 4-byte entries sorted by place: screen, x, y, type; `$8C` is the next entry right of the
spawn window, `$8B` the last left of it.

| Game routine | Original | Hack |
|---|---|---|
| object loop's screen test (bank 5 `$9946`; also `$BE76`, an enemy of the list's) | an object whose x - camera is off the screen gets its "off the screen" event (removal, for most) | an enemy (slot `$10` up): when it leaves `[LO, HI)` |
| spawn window, right (`$D8BF`, `$D95A`) | entries up to camera + 255 spawn | up to camera + 255 + M, within the section |
| spawn window, left (`$D921`, compares `$D934`, `$D93E`; the entries passed by `$D8FC`, compares `$D906`, `$D90F`) | entries from the camera spawn going left | from camera - M, within the section |
| spawned enemy's screen (`$D9EA`) | the spawn window's edge's screen | its entry's own (the same with the original window); it sleeps when that place is off the original screen |
| enemy of the list (bank 5 `$9905`, `JSR $AA29`) | its frame | an enemy asleep skips its frame while off the original screen (it is still removed past the margins); on it, it wakes for good |
| an object's sprites (`$D21F`, `$D242`) | x = 8-bit screen x + signed offset, the sprite hidden when it carries; the loop stops when OAM is full (sprites 1-63), the objects drawn in the other order every other frame (the flicker) | 16-bit x, shown within `[-M, 256 + M)`, its high byte to `$5100 + 4 n` (or `$5400 + 4 n`); the loop goes on to the extension's second page (`$5300`) when the first is full, and stops when both are |
| energy bars' sprites (`$D41A`) | write to `$0204 + Y` | write to the page in use |
| hiding the sprites (`$D478`) | X sprites from offset Y | the same, then the second page and every high byte, back to the first page |
| damage (bank 5 `$A23F`) | Mega Man's health less the damage, then the knockback | with the option "Invincible", nothing: back to the main loop |

`LO = max(-M, section start - camera)`, `HI = min(256 + M, section end - camera)`; M is the margin
for the enemies, 0 for slots below `$10` (Mega Man's shots keep the original limits). Mega Man and his
shots are drawn with the low byte of their screen x alone, as the original: a scroll to another
section puts Mega Man on the new screen before the camera, and the original draws him on the screen
still. The game's
collisions (`$C9BE` for Mega Man's touch, `$C9C2` for his weapons, against an object) take an
object only when its x - camera is on the screen (high byte 0, `$CA0F`): nothing in a margin touches or is touched, which
keeps the game's logic exactly the original's on the screen. With M = 0 every routine does exactly
what the code it replaces does.

Margin: `M = min($5002, 248)` (248: the widest the extension's 8-bit registers carry; sprites show
that far with `$5007` bit 1).

CPU time: the hack's work and the enemies of the margins cost time the console does not have. The
hack asks for 64 lines of overclock (`$5006`), which leaves the picture and the sound untouched; 255
with the option "No slowdown" (on by default). Measured on Ice Man's stage (200 frames from its
start, holding right, 128-pixel margin, counted with the frame counter `$23`): the original runs late
no frame; the hack without overclock 100 (18 at a margin of 0: its sprite loop is slower), with 64
lines or 255 none. Cut Man's first section (1300 frames): none with 255 lines.

## 3. The extension

Set every frame at the start of the NMI's screen update (`$D4B7`, `LDA $2002 / LDA #0 / STA $2003`
becomes `JSR wide_nmi / NOP` x 5, which writes `$5003 = M`, `$5004 = $5005 = 0` (the nametable
columns are hidden: the decoder draws the background), `$5007 = $1E` (sprites free of those extents,
second sprite page, high bytes kept from one page to the next: the page and its high bytes are
rebuilt together, `$D478` clearing both; the window kept to `$7FFF`), `$5006 = 64` (255 with "No
slowdown"), then does the replaced instructions).

Variables (extension RAM): `WV_M $5200`, the live range `WV_LO $5201`/`$5202` and `WV_HI
$5203`/`$5204` (signed 16-bit), the object's screen x `WV_S $5205`/`$5206`, scratch `WV_T0 $5207`,
`WV_T1 $5208`, `WV_PAGE $5209` (bit 7: the second page is in use), `WV_X $520A`, `WV_DORMANT $5220
+ slot` (the list entry of an enemy asleep, `$FF` awake: a slot another object took since is awake).

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length. Bank 7 is
the fixed bank (`$C000-$FFFF`), written into both its copies (7 and 15).

| Address | Replaces | With |
|---|---|---|
| `$D4B7` | `LDA $2002 / LDA #$00 / STA $2003` | `JSR wide_nmi` |
| `$D478` | `LDA #$F8 / STA $0200,Y` (head of the loop hiding X sprites) | `JMP wide_clear_oam` |
| `$D21F` | `LDY #$02 / SEC / LDA $0480,X / SBC $1A / STA $0F` | `JSR wide_draw_origin` |
| `$D242` | `LDA ($00),Y / STA $0205,X` (head of an object's sprite loop) | `JMP wide_draw_loop` |
| `$D41A` | `LDA $0C / STA $0207,Y` (head of the energy bars' sprite routine) | `JMP wide_bar_sprite` |
| bank 5 `$9946` | `SEC / LDA $0480,X / SBC $1A / LDA $0460,X / SBC $1B / BEQ $9958` | `JSR wide_cull / BCC $9958` |
| bank 5 `$BE76` | `SEC / LDA $0480,X / SBC $1A / LDA $0460,X / SBC $1B / BEQ $BE86` | `JSR wide_cull / BCC $BE86` |
| bank 5 `$9905` | `JSR $AA29` | `JSR wide_think` |
| `$D8BF`, `$D95A` | `CLC / LDA $1A / ADC #$FF / STA $04 / LDA $1B / ADC #0 / STA $05` | `JSR wide_spawn_right` |
| `$D921` | `LDA $1B / STA $05 / LDA $1A / STA $04` | `JSR wide_spawn_left` |
| `$D934`, `$D906` | `CMP $1B` | `CMP $05` |
| `$D93E`, `$D90F` | `CMP $1A` | `CMP $04` |
| `$D8FC` | `LDA $8B / CLC / ADC #$01` | `JSR wide_pass_left` |
| `$D9EA` | `LDA $05 / STA $0460,X` | `JSR wide_spawned` |
| bank 5 `$A23F` | `SEC / LDA $6A / SBC $0C` | `JSR wide_hurt` |

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there (`WIDE_OPTIONS`).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 1 | `invincible` | Invincible | off | a testing aid: Mega Man takes no damage and no knockback (pits and spikes still kill) |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of 64 |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `PATCHES`); the ROM does
not depend on them being set.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Mega Man (USA).nes" build/mm1wide.bps
```

assembles `src/wide.s` for `$5800` (one 16 KiB bank, `-g`, labels from `ld65 -Ln`), lays the PRG
out in 16 banks, writes the hooks, `build/mm1wide.nes` and the BPS patch (`tools/make_bps.py`: the
fixed bank's copy is SourceCopy from the player's ROM, the hooks and the code TargetRead).
`WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.

## 7. Limits

* No sprite in the vertical margins: the extension carries a high byte for x only.
* Only the enemies (slots `$10` up) live in the margins; Mega Man's shots keep the original screen.
* An enemy asleep in a margin hangs where its entry puts it until the original screen reaches it.
* The objects the game makes outside the stage list (what the enemies fire, the bosses) keep the
  game's own limits where their own code removes them.
* The overclock removes lag frames, so a replay of the original's inputs frame for frame may drift
  from the original.

---

## Appendices

### A. `src/wide.s`

```asm
; Mega Man (USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added to the game (PRG bank 8 of 16: the game's banks 0-7 stay
; where they were and its fixed bank is copied to bank 15, the one UxROM fixes at $C000); the
; widescreen extension's PRG window shows it at $5800-$7FFF ($5007 bit 4), Zaxnes opening it at
; power-on (the patch's boot window: bank 8, page 0). The game reaches it through hooks in its own
; code (tools/build.py). The background of the margins is Zaxnes' margin decoder's work; the hack
; does the objects, the way the Mega Man 2 hack does them:
;
;  * an enemy of the stage's list (object slots $10-$1F) spawns when it comes within the margin of
;    the screen, and lives until it leaves it, all within the section being played (screens
;    $27-$28);
;  * one that spawns in a margin sleeps there: it shows but does not move until it comes onto the
;    original screen; awake, it stays awake;
;  * every object's sprites get the high byte of their 16-bit x, and the sprites that no longer
;    fit in the console's 64 go on to the extension's second page instead of flickering;
;  * an object outside the original screen touches nothing: the game's collisions compare the
;    screen x of both, and only on the original screen (its own test);
;  * Mega Man and his shots (slots $00-$0F) keep the original limits.
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
                                ; bit 3 high bytes kept, bit 4 the window to $7FFF
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA
EXTRA_OAM       = $5300         ; the second sprite page, OAM format
EXTRA_SPRHI     = $5400         ; + 4 n: high byte of the x of its sprite n

OPT_INVINCIBLE  = $02           ; a testing aid: Mega Man takes no damage
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late

MAX_MARGIN      = 248           ; the most the extension shows sprites in
OVERCLOCK_LINES = 64            ; pays for the hack's work and the enemies of the margins
NO_SLOWDOWN_LINES = 255
FIRST_ENEMY     = $10           ; the slots the stage's list spawns into

; Variables in the extension's RAM ($5200-$57FF).
WV_M            = $5200         ; margin of this frame
WV_LOL          = $5201         ; screen x range objects may live in: [LO, HI), signed 16-bit
WV_LOH          = $5202
WV_HIL          = $5203
WV_HIH          = $5204
WV_SL           = $5205         ; screen x of the object at hand, signed 16-bit
WV_SH           = $5206
WV_T0           = $5207         ; scratch
WV_T1           = $5208
WV_PAGE         = $5209         ; bit 7: the sprites being placed go to the second page
WV_X            = $520A         ; X, kept
WV_DORMANT      = $5220         ; + slot: the list entry ($6B + slot) of an enemy asleep, $FF awake

; The game.
OAM             = $0200
CAM_LO          = $1A           ; the camera's x: pixel and screen
CAM_HI          = $1B
SECTION_FIRST   = $27           ; the first and last screens of the section being played
SECTION_LAST    = $28
OBJ_SCREEN      = $0460         ; + slot: an object's x, screen and pixel
OBJ_X           = $0480
OBJ_Y           = $0600         ; $F8: the slot is free
LIST_ENTRY      = $6B           ; + slot: the stage list's entry an enemy came from ($FF none)
SPAWN_LIST      = $06           ; pointer to the stage list's entry at hand
SPAWN_NEXT      = $03           ; Y past that entry
HEALTH          = $6A
DAMAGE          = $0C

; The game's routines.
ENEMY_ROUTINE   = $AA29         ; bank 5: an enemy of the stage list's whole frame
NEXT_OBJECT     = $998A         ; bank 5: the object loop's next slot
LOOP_FULL       = $C3B3         ; the sprite loop's exit when OAM is full (maps bank 5 back)

.segment "CODE"
.org $5800

; ---------------------------------------------------------------------------- ;
; NMI ($D4B7, in place of LDA $2002 / LDA #0 / STA $2003): the extension's settings for the next
; frame.
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
    LDA     #$1E                        ; sprites free of the extents, second sprite page, high
                                        ; bytes kept (every sprite placed gets one), the window
                                        ; kept to $7FFF
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
    LDA     $2002
    LDA     #$00
    STA     $2003
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

; The screen x range objects of slot X may live in, into WV_LO/WV_HI:
;   LO = max(-M, section start - camera), HI = min(256 + M, section end - camera),
; M being 0 for Mega Man's own slots. The original's range is [0, 256).
wide_bounds:
    LDA     WV_M
    CPX     #FIRST_ENEMY
    BCS     @margin
    LDA     #$00
@margin:
    STA     WV_T0
    SEC                                 ; LO = section start - camera
    LDA     #$00
    SBC     CAM_LO
    STA     WV_LOL
    LDA     SECTION_FIRST
    SBC     CAM_HI
    STA     WV_LOH
    CLC                                 ; LO + M < 0: LO = -M
    LDA     WV_LOL
    ADC     WV_T0
    LDA     WV_LOH
    ADC     #$00
    BPL     @hi
    SEC
    LDA     #$00
    SBC     WV_T0
    STA     WV_LOL
    LDA     #$00
    SBC     #$00
    STA     WV_LOH
@hi:
    SEC                                 ; HI = section end - camera
    LDA     #$00
    SBC     CAM_LO
    STA     WV_HIL
    LDA     SECTION_LAST
    SBC     CAM_HI
    CLC
    ADC     #$01
    STA     WV_HIH
    SEC                                 ; HI - 256 > M: HI = 256 + M
    LDA     WV_HIL
    SBC     WV_T0
    LDA     WV_HIH
    SBC     #$01
    BMI     @rts
    BNE     @cap
    LDA     WV_HIL
    CMP     WV_T0
    BCC     @rts
    BEQ     @rts
@cap:
    LDA     WV_T0
    STA     WV_HIL
    LDA     #$01
    STA     WV_HIH
@rts:
    RTS

; Whether object X lives: carry clear when its screen x (WV_S, set) is within [LO, HI).
wide_inside:
    JSR     wide_bounds
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

; The object loop's test (bank 5 $9946, in place of "x - camera not on this screen"; also
; $BE76, an enemy of the list's): carry clear when object X lives on.
wide_cull:
    JSR     wide_screen_x
    JMP     wide_inside

; ---------------------------------------------------------------------------- ;
; Sleeping enemies. An enemy of the stage list that spawns in a margin shows but does not move
; until it comes onto the original screen; from then on it lives as the original does, margins
; included.

; The spawn (in place of LDA $05 / STA $0460,X at $D9EA; X the slot, ($06),Y the list entry,
; Y = $03 past it): the enemy's screen is its entry's (the game took the spawn window's, the same
; when the window is the original's), and it sleeps when it spawns off the original screen.
wide_spawned:
    LDY     SPAWN_NEXT
    DEY
    DEY
    DEY
    DEY
    LDA     (SPAWN_LIST),y              ; the entry: screen, x, y, type
    STA     OBJ_SCREEN,x
    INY
    LDA     (SPAWN_LIST),y
    STA     OBJ_X,x
    JSR     wide_screen_x
    LDA     WV_SH
    BEQ     @awake
    LDA     LIST_ENTRY,x                ; asleep: its entry number
    JMP     @set
@awake:
    LDA     #$FF
@set:
    STA     WV_DORMANT,x
    RTS

; The object loop (bank 5 $9905, in place of JSR $AA29, the frame of an enemy of the list): an
; enemy asleep skips its frame while off the original screen (it is still removed past the
; margins); on it, it wakes up.
wide_think:
    LDA     WV_DORMANT,x
    CMP     #$FF
    BEQ     @run
    CMP     LIST_ENTRY,x                ; another object took the slot since: awake
    BNE     @wake
    JSR     wide_screen_x
    LDA     WV_SH
    BEQ     @wake
    JSR     wide_inside
    BCS     @run                        ; past the margins: the enemy's own frame removes it
    RTS
@wake:
    LDA     #$FF
    STA     WV_DORMANT,x
@run:
    JMP     ENEMY_ROUTINE

; ---------------------------------------------------------------------------- ;
; The spawn window: the stage list (screen, x, y, type entries sorted by place) spawns the
; enemies between camera - M and camera + 255 + M, within the section being played.

; $D8BF and $D95A, in place of $05:$04 = camera + 255.
wide_spawn_right:
    CLC
    LDA     CAM_LO
    ADC     #$FF
    STA     $04
    LDA     CAM_HI
    ADC     #$00
    STA     $05
    CLC
    LDA     $04
    ADC     WV_M
    STA     $04
    LDA     $05
    ADC     #$00
    STA     $05
    CMP     SECTION_LAST
    BCC     @rts
    BEQ     @rts
    LDA     #$FF                        ; past the section: its last pixel
    STA     $04
    LDA     SECTION_LAST
    STA     $05
@rts:
    RTS

; $D921, in place of $05:$04 = camera (the list's entries from it spawn going left; $D934 and
; $D93E compare them with $05:$04 instead of the camera).
wide_spawn_left:
    SEC
    LDA     CAM_LO
    SBC     WV_M
    STA     $04
    LDA     CAM_HI
    SBC     #$00
    STA     $05
    BCC     @first                      ; before the level
    CMP     SECTION_FIRST
    BCS     @rts
@first:
    LDA     #$00                        ; before the section: its start
    STA     $04
    LDA     SECTION_FIRST
    STA     $05
@rts:
    RTS

; $D8FC, in place of LDA $8B / CLC / ADC #1: the entries left of the window are passed by
; ($D906 and $D90F compare them with $05:$04 instead of the camera).
wide_pass_left:
    JSR     wide_spawn_left
    LDA     $8B
    CLC
    ADC     #$01
    RTS

; ---------------------------------------------------------------------------- ;
; Sprites. The console has 64 (the game leaves sprite 0 alone: its page starts at $0204); the
; extension a second page of 64 at $5300, which the game's sprite loops move on to when the first
; is full, instead of stopping (the objects left over then flickered, the game drawing them in the
; other order every other frame).

; Stores A at byte k of the sprite at offset X of the page in use (flags N and V change).
.macro PUT k
    BIT     WV_PAGE
    BMI     :+
    STA     OAM+4+k,x
    JMP     :++
:   STA     EXTRA_OAM+k,x
:
.endmacro

; $D478, in place of the loop that hides X sprites from offset Y: that, then the second page and
; every high byte (they are kept from one page to the next: the page and its high bytes are
; rebuilt together), back to the first page.
wide_clear_oam:
    LDA     #$F8
@game:
    STA     OAM,y
    INY
    INY
    INY
    INY
    DEX
    BNE     @game
    STY     WV_T0
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
    LDY     WV_T0
    RTS

; An object's screen x ($D21F, in place of LDY #2 / SEC / LDA $0480,X / SBC $1A / STA $0F): kept
; in 16 bits for an enemy's sprites. Mega Man and his shots keep the original's low byte alone (a
; scroll to another section puts Mega Man on the new screen before the camera, and the original
; draws him on the screen still).
wide_draw_origin:
    JSR     wide_screen_x
    LDA     WV_SL
    STA     $0F
    CPX     #FIRST_ENEMY
    BCS     @enemy
    LDA     #$00
    STA     WV_SH
@enemy:
    LDY     #$02
    RTS

; The sprite loop of one object, for one page (base its OAM, high the high bytes of their x):
; the original's, the x in 16 bits. A sprite whose x is past the margins is hidden; one in a margin
; gets the high byte of its x (one on the screen keeps the 0 the page was cleared with). X the
; offset in the page, Y the index in the tile list ($00), $12 the index in the position list
; ($02), $0E sprites, $10 the object's y, $11 its horizontal flip, $13 priority, $0F and WV_SH
; its screen x. `last` the offset past the page's last sprite, `full` where to go then; `sprite`
; and `next` name the loop's head and its step to the next sprite.
.macro DRAW_LOOP base, high, last, full, sprite, next
.local l_up, l_hide, l_y, l_own, l_x, l_neg, l_hi, l_right, l_show, l_done
sprite:
    LDA     ($00),y                     ; tile
    STA     base+1,x
    INY
    LDA     ($00),y                     ; attributes
    EOR     $11
    ORA     $13
    STA     base+2,x
    INY
    STY     $0D
    LDY     $12
    LDA     ($02),y
    TAY
    CLC
    LDA     $833A,y                     ; y offset
    BMI     l_up
    ADC     $10
    BCC     l_y
    BCS     l_hide
l_up:
    ADC     $10
    BCS     l_y
l_hide:
    LDA     #$F8
    STA     base,x
    LDY     $0D
    JMP     next
l_y:
    STA     base,x
    LDA     $11
    BEQ     l_own
    LDA     $853A,y                     ; x offset, flipped
    JMP     l_x
l_own:
    LDA     $843A,y
l_x:
    CLC
    BMI     l_neg
    ADC     $0F
    STA     base+3,x
    LDA     WV_SH
    ADC     #$00
    JMP     l_hi
l_neg:
    ADC     $0F
    STA     base+3,x
    LDA     WV_SH
    ADC     #$FF
l_hi:
    BEQ     l_show                      ; 0 <= x < 256
    CMP     #$01
    BEQ     l_right
    CMP     #$FF
    BNE     l_hide
    LDA     base+3,x                    ; x < 0: x + M >= 0?
    CLC
    ADC     WV_M
    BCC     l_hide
    LDA     #$FF
    STA     high,x
    JMP     l_show
l_right:
    LDA     base+3,x                    ; 256 <= x: below 256 + M?
    CMP     WV_M
    BCS     l_hide
    LDA     #$01
    STA     high,x
l_show:
    LDY     $0D
    TXA
    CLC
    ADC     #$04
    TAX
    CPX     #last
    BEQ     full
next:
    INC     $12
    DEC     $0E
    BEQ     l_done
    JMP     sprite
l_done:
    STX     $0D
    RTS
.endmacro

; The sprite loop of one object ($D242, in place of its head): on the page in use.
wide_draw_loop:
    BIT     WV_PAGE
    BPL     wide_draw_first
    JMP     wide_draw_second

    DRAW_LOOP OAM+4, WIDE_SPRHI+4, $FC, wide_first_full, wide_draw_first, wide_first_next
wide_first_full:
    LDA     #$80                        ; the first page full: on to the second
    STA     WV_PAGE
    LDX     #$00
    JMP     wide_second_next

    DRAW_LOOP EXTRA_OAM, EXTRA_SPRHI, $00, wide_both_full, wide_draw_second, wide_second_next
wide_both_full:
    PLA                                 ; both full: out of the object loop, as the original
    PLA
    JMP     LOOP_FULL

; One sprite of the energy bars ($D41A, the whole routine, in place of the writes to $0204,Y):
; Y the OAM offset, $0C x, $0F tile, $12 attributes, $0E y (8 less for the next one).
wide_bar_sprite:
    STX     WV_X
    TYA
    TAX
    LDA     $0C
    PUT     3
    LDA     $0F
    PUT     1
    LDA     $12
    PUT     2
    LDA     $0E
    PUT     0
    BIT     WV_PAGE
    BMI     @second
    CPX     #$F8
    BNE     @advance
    LDA     #$80                        ; the first page full: on to the second
    STA     WV_PAGE
    LDX     #$00
    BEQ     @step
@second:
    CPX     #$FC
    BEQ     @done                       ; both full: the last sprite again, as the original
@advance:
    INX
    INX
    INX
    INX
@step:
    LDA     $0E
    CMP     #$F8
    BEQ     @done
    SEC
    SBC     #$08
    STA     $0E
@done:
    TXA
    TAY
    LDX     WV_X
    RTS

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1, a testing aid): Mega Man takes no damage.

; Bank 5 $A23F, in place of SEC / LDA $6A / SBC $0C: his health less the damage. Invincible,
; nothing happens: back to the caller of the collision routine ($A18B, the main loop's), whose
; frame this is, with no knockback either.
wide_hurt:
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BEQ     @hurt
    PLA
    PLA
    RTS
@hurt:
    SEC
    LDA     HEALTH
    SBC     DAMAGE
    RTS

wide_end:
.assert wide_end <= $8000, error, "widescreen code past the window"
```

### B. `tools/build.py`

```python
"""Builds the Mega Man (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Mega Man (USA).nes> [out.bps]
       python -I tools/build.py <Mega Man (USA).nes> --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank the widescreen extension's PRG window shows at $5800-$7FFF (Zaxnes opens it at
   power-on for this patch: bank 8, page 0).
2. Lays the PRG-ROM out as 16 banks: the game's 8 where they were, the hack's bank 8, banks
   9-14 of $FF, and the game's fixed bank again as bank 15, the one UxROM fixes at $C000.
3. Writes the hooks into the game's banks, each checked against the bytes it replaces (those of
   the fixed bank into both its copies).
4. Writes build/mm1wide.nes (header: 16 PRG banks) and, if asked, the BPS patch (data without
   iNES header; the game's banks moved or copied come from the player's ROM).

With --pack, the layout and the patched ROM give way to a Zaxnes hack pack, format 2, written
by Zaxnes' shared builder (tools/hackpack.py of Zaxnes, its folder in the environment variable
ZAXNES_TOOLS): NTSC, UxROM, the code (its used part), and the hooks as a 16 KiB bank and a CPU address
each, the ROM keeping its size.
The invincibility hook is a component of its own, "invincible" (`LDA #$A2 / RTS`, back from the
collision routine), applied by Zaxnes while that option is on, with or without the widescreen;
every other hook is the "widescreen" component, in one group ("core", Base).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x6EE4BB0A
BANK_SIZE = 0x4000
PRG_BANKS = 8
PRG_SIZE = PRG_BANKS * BANK_SIZE
CHR_SIZE = 0
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

    # The NMI: the extension's settings.
    put(7, 0xD4B7, 'ad 02 20 a9 00 8d 03 20', jsr(sym['wide_nmi']))
    # Sprites: both pages hidden, an object's 16-bit screen x, its sprite loop, the energy bars.
    put(7, 0xD478, 'a9 f8 99', jmp(sym['wide_clear_oam']))
    put(7, 0xD21F, 'a0 02 38 bd 80 04 e5 1a 85 0f', jsr(sym['wide_draw_origin']))
    put(7, 0xD242, 'b1 00 9d', jmp(sym['wide_draw_loop']))
    put(7, 0xD41A, 'a5 0c 99', jmp(sym['wide_bar_sprite']))
    # The object loop: an object off the screen (an enemy: past the margins), sleeping enemies.
    put(5, 0x9946, '38 bd 80 04 e5 1a bd 60 04 e5 1b f0 05',
        jsr(sym['wide_cull']) + bytes([0x90, 0x0D]))           # BCC $9958
    put(5, 0xBE76, '38 bd 80 04 e5 1a bd 60 04 e5 1b f0 03',
        jsr(sym['wide_cull']) + bytes([0x90, 0x0B]))           # BCC $BE86
    put(5, 0x9905, '20 29 aa', jsr(sym['wide_think']))
    # The spawn window: right, left, the entries passed by on the left, the enemy's screen.
    put(7, 0xD8BF, '18 a5 1a 69 ff 85 04 a5 1b 69 00 85 05', jsr(sym['wide_spawn_right']))
    put(7, 0xD95A, '18 a5 1a 69 ff 85 04 a5 1b 69 00 85 05', jsr(sym['wide_spawn_right']))
    put(7, 0xD921, 'a5 1b 85 05 a5 1a 85 04', jsr(sym['wide_spawn_left']))
    put(7, 0xD934, 'c5 1b', bytes([0xC5, 0x05]))
    put(7, 0xD93E, 'c5 1a', bytes([0xC5, 0x04]))
    put(7, 0xD8FC, 'a5 8b 18 69 01', jsr(sym['wide_pass_left']))
    put(7, 0xD906, 'c5 1b', bytes([0xC5, 0x05]))
    put(7, 0xD90F, 'c5 1a', bytes([0xC5, 0x04]))
    put(7, 0xD9EA, 'a5 05 9d 60 04', jsr(sym['wide_spawned']))
    # Invincibility: damage.
    put(5, 0xA23F, '38 a5 6a e5 0c', jsr(sym['wide_hurt']))
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


# The hook of the option "Invincible": a component of the pack of its own.
INVINCIBLE_HOOK = (0x05, 0xa23f)


def pack(data, code, sym, out):
    tools = os.environ.get('ZAXNES_TOOLS')
    if not tools or not os.path.isfile(os.path.join(tools, 'hackpack.py')):
        sys.exit('ZAXNES_TOOLS: the rust/tools folder of Zaxnes (its hackpack.py) is needed')
    sys.path.insert(0, tools)
    import hackpack
    used = (sym['wide_end'] - CODE_START + 0xFF) & ~0xFF
    pack = hackpack.Pack('Mega Man (USA) widescreen', code[:used], region='ntsc', mappers=(2,))
    core = pack.component('widescreen').group('core', label='Base', label_fr='Base')
    aid = pack.component('invincible').group('invincible', label='Invincible',
                                             label_fr='Invincible')
    for bank, address, original, new in hooks(sym):
        invincible = (bank, address) == INVINCIBLE_HOOK
        if invincible:
            # LDA #$A2 / RTS: A as wide_hurt's PLA / PLA leaves it (the high byte of its return).
            aid.hook(bank, address, original, bytes([0xA9, 0xA2, 0x60]))
        else:
            core.hook(bank, address, original, new)
    assert len(aid.hooks) == 1
    result = pack.build(data, 8 * 0x4000)
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes ({used} of code, {len(core.hooks)} hooks), '
          f'pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = bytes(image[header:header + PRG_SIZE + CHR_SIZE])
    assert zlib.crc32(data) == SOURCE_CRC, 'not Mega Man (USA)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        pack(data, code, sym, sys.argv[3])
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
    out = os.path.join(ROOT, 'build', 'mm1wide.nes')
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
