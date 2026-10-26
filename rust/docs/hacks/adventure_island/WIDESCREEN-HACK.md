# Adventure Island (USA): the widescreen hack

This document describes completely the modification of *Adventure Island (USA)* that Zaxnes ships
as a built-in patch for the game's **advanced** widescreen mode. The appendices hold the new code
and the build script, which writes every hook after checking the bytes it replaces: with them the
ROM can be rebuilt identically.

The background of the margins is not the hack's work: Zaxnes' Adventure Island margin decoder lays
the round out from its tables in PRG-ROM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4). In the **simple**
mode the ROM is not patched at all and the decoder alone widens the game; the advanced mode adds
this patch: the sprites and objects of the margins, and the options.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Adventure Island (USA)*, CNROM (mapper 3), 32 KiB PRG, 32 KiB CHR |
| Source data (iNES header excluded) | CRC-32 `F8A713BE` (the dump the [disassembly](https://github.com/cyneprepou4uk/NES-Games-Disassembly/tree/main/Adventure%20Island) rebuilds, SHA-1 of the iNES file `22EE75B8...`) |
| Target data | CRC-32 `4CDF2FE8` (the hack's version in savestate headers; `tools/build.py` prints it), 80 KiB: the game's 32 KiB of PRG, a 16 KiB PRG bank added, the 32 KiB of CHR |
| Changes | hooks in the game's PRG (same size as what they replace); the new code in the added bank |
| Patch | `rust/crates/zax-nes/patches/adventure-island-usa-widescreen.bps`, made against the data without header; the CHR-ROM, moved after the new bank, is copied from the player's ROM (BPS SourceCopy), so the patch carries only the hack's bytes |
| Zaxnes declaration | `builtin_patches.rs`: `advanced: true` (applied only in the advanced mode), `boot_window: Some((0x82, 0x00))`, `idle_loop: Some((0xB759, 0xB75C))` |

The game has no free byte left in its PRG. Zaxnes therefore opens the extension at power-on and
reset for this patch (the boot window), as for Super Mario Bros.: unlocked, the PRG window showing
the added bank (bank 2, page 0) at `$5800-$5FFF`, and on to `$7FFF` (`$5007` bit 4; the game has
no PRG-RAM there). CNROM maps the first 32 KiB at `$8000-$FFFF` as before (its register only
switches the CHR), so the game runs as it was; its hooks call the new code in the window. The data
grows by one PRG bank; Zaxnes raises the PRG bank count of the player's header accordingly when it
applies the patch.

---

## 2. What the game does, and what changes

The main loop waits for the NMI at `$B759` (`LDA $0B / BNE`, the yield's idle loop); the NMI copies
the sprites (`$0700`, OAM DMA) only when the frame's logic is done. Objects live in 16 slots
(0 the player), each variable an array indexed by the slot: state `$0554` (0 free), kind `$0564`,
x `$0584` (low) and `$0594` (high), y `$05C4`, half widths `$05E4` (drawing) and `$0604`/`$0614`
(collisions). An object's x is a **screen** x, 16-bit: the game moves every object with the screen
as it scrolls (`sub_8E64`). Objects are created 64 pixels past the right edge (x = 320, `$8C5A`)
and removed past x = 400 or when their left edge (x minus half width) goes past the screen's left
edge (`$8EA7`).

| Part | The game | The hack |
|---|---|---|
| NMI (`$B717`) | | the extension's settings each frame: `$5003 = min($5002, 248)`, extents 0 (the decoder draws the background), `$5007 = $16` (sprites free of the extents, second sprite page, window to `$7FFF`), `$5006` = 255 with the option "No slowdown" else 0 |
| an object's sprites (`sub_C301`, `$C305`) | gives up an object past the right edge (x high byte 1 and up) | gives up an object past the wide view (`x < -M-32` or `x >= 256+M`) |
| each sprite (`$C48B`, `$C4A9`) and a single-sprite object (`$C378`) | leaves out a sprite whose x is not on the screen (high byte not 0) | places a sprite within the wide view (`-M-8 < x < 256+M`) and writes the high byte of its x to the extension (`$5100` + its OAM offset) |
| the left edge (`$8EA7`) | removes an object whose left edge goes past the screen's | keeps it as a **ghost** while `x >= -M-32` (x high byte 0, or `$FF` with `x + 32 + M` carrying); a ghost back on the screen goes, as the game had it go; past the view it goes |
| an object's behaviour (`$8E8E`) | runs it | runs a ghost's too, but keeps the sound it asked for out (`$A0` restored) and creates nothing for it (`wide_find_slot` finds no slot) |
| two objects' collision (`sub_BFF2`) | | none with a ghost |
| a new object's slot (`sub_8DB4`, `$8DB6`) | the highest free slot, none when all are taken | the same, or else a ghost's slot (the ghost is gone) |
| an object's state (`$9534`, the `STA $0554,X` that creates and removes) | | the slot is no ghost any more |
| every object cleared (`sub_8C35`, `$8C41`) | | no ghost any more |
| title screen (`$AE51`) | Start begins; Select only sets the demo's timer again | with the option "Level select": Select goes through the 32 rounds, shown under "PUSH START" in the second sprite page (two digits, area and round) |
| new game (`$8078`) | area 0, round 0 | the round selected, with the option on (not in the demo); the second page hidden |
| an enemy's touch (`$C04D`) | kills (or takes the skateboard, or the fairy protects) | with the option "Invincible": no effect, as if the player were dead already; falls, the water and hunger still kill |

M is the margin of the frame (`WV_M`, at most 248). With M = 0 every routine does what the code it
replaces does: the game runs the same (checked: the same inputs on the original ROM and on the hack
without margins leave the same RAM, but for the stack's unused bytes).

**Why ghosts.** The game's own objects keep their exact behaviour: the hack changes nothing of what
they do on the screen. An object the game would have removed at the left edge only shows, still
moving, in the left margin: it touches nothing (the player, the axes, the other objects), makes no
sound and creates nothing, so whatever it does there cannot change the game. A ghost takes a slot
the game would have had free; a new object takes it back when it needs it. The game has no random
number generator, so running a ghost's behaviour changes nothing elsewhere. Checked: the same inputs
with a 96-pixel margin and the original lead to the same deaths and the same sections; the hack is
a few frames behind after a few thousand frames (the frames it takes longer, without the option
"No slowdown").

**The right margin.** Objects are created 64 pixels past the screen (x = 320) and live up to
x = 400: within a margin of 64 pixels they show, moving, before they reach the screen, as the game
runs them; past it they appear at x = 320.

## 3. Variables (extension RAM)

`WV_M $5200` margin, `WV_A $5201` the animation of `sub_C301` (kept), `WV_T0 $5202` / `WV_T1
$5203` scratch, `WV_LEVEL $5204` the round selected (0-31), `WV_MAGIC $5205` (`$A5` once the RAM is
set up: no ghost, the second page hidden), `WV_GHOST_RUN $5206` (a ghost's behaviour is running),
`WV_SOUND $5207` (the sound before it), `WV_GHOST $5210 + slot`. The second sprite page
`$5300-$5307` holds the title's two digits. The code is at `$5800`-`$7FFF` (the window).

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$B717` | `LDA #<ram_oam / STA $2003` (NMI) | `JSR wide_nmi` |
| `$C305` | `LDY $59 / DEY / BPL $C372` (`sub_C301`) | `JSR wide_object_shown` |
| `$C48B`, `$C4A9` | `LDA $5B / BNE $C4C8` | `JSR wide_sprite` |
| `$C378` | `LDA $59 / BNE $C3A3` | `JSR wide_one_sprite` |
| `$8EA7` | the 13 bytes of "x - half width < 0" (`... / BMI $8EC2`) | `JSR wide_left_gone / BCS $8EC2` |
| `$8E8E` | `JSR sub_929E_object_handler` | `JSR wide_handler` |
| `$BFF2` | `LDA $0614,X / CLC / ADC $0614,Y` (`sub_BFF2`) | `JSR wide_collision` |
| `$8DB6` | the free slot search (`LDA $0554,X / BEQ / DEX / BNE`) | `JSR wide_find_slot / BCS $8DBF / RTS` |
| `$9534` | `STA $0554,X / RTS` | `JMP wide_object_state` |
| `$8C41` | `STX $0536` (`sub_8C35`) | `JSR wide_clear_ghosts` |
| `$AE51` | `CMP #$10 / BNE $AE8F` (title screen) | `JSR wide_title_button` |
| `$8078` | `LDA #0 / STA $37 / STA $38` (new game) | `JSR wide_start_level` |
| `$C04D` | `LDA $72 / ORA $70` (an enemy's touch) | `JSR wide_injury` |

A routine that gives up what the game was doing (an object or a sprite left out, no collision, the
title's Select) drops its return address and goes where the game's branch went (`$C3A3`, `$C4C8`,
`$AE8F`, the caller of `sub_C301` or `sub_BFF2`).

## 5. Options

Zaxnes writes their bits at `$500A` every frame (the game's hacks screen, "Advanced mode" on).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (Select on the title screen) / Sélecteur de niveau (Select sur l'écran titre) | off | a testing aid: Select goes through the 32 rounds (shown "area round" under "PUSH START"), Start begins there |
| 1 | `invincible` | Invincible | off | a testing aid: an enemy's touch does nothing; falls, the water and hunger still kill |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | `$5006 = 255`: no lag frame; with Zaxnes' yield (`idle_loop`), a frame that does not need it costs nothing more |

With the movies of Zaxnes (Tools menu, `.zmv`), the level selector and invincibility make a round
quick to reach and to go through when looking for what shows wrong in the margins.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Adventure Island (USA).nes" build/aiwide.bps
python -I tools/make_doc.py <zaxnes>/rust/docs/hacks/adventure_island/WIDESCREEN-HACK.md
```

`build.py` assembles `src/wide.s` for `$5800` (a 16 KiB bank, `$FF` fill), writes the hooks into the
game's PRG (checked), the target data (PRG, the bank, CHR), `build/aiwide.nes` (header with 3 PRG
banks) and the BPS patch (`make_bps.py`: SourceRead, SourceCopy for the moved CHR, TargetRead,
TargetCopy for runs of one value). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.
`make_doc.py` writes the appendices of this document from the sources.

## 7. Limits

* Past a margin of 64 pixels, objects appear at x = 320 (where the game creates them), not at the
  edge of the view.
* Past the round's end (the boss's room, the last screen) the margins are empty.
* A ghost runs its behaviour without touching anything: an enemy that would have turned back in the
  margin goes back onto the screen and is removed there, as the game had removed it at the edge.
* More sprites show: the game still places 63 at most (`$C464`) and turns which ones are left out
  each frame, so a crowded screen flickers more than on the console.
* The cloud platforms and the other things the game draws over the tables as it draws a column
  appear in the right margin when the game draws them, 8 pixels past the screen (the decoder's
  limit, see the decoder's description).

## Appendices

### A. `src/wide.s`

```asm
; Adventure Island (USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added after the game's 32 KiB; the widescreen extension's PRG
; window shows it at $5800-$7FFF ($5007 bit 4), Zaxnes opening it at power-on (the patch's boot
; window: bank 2, page 0). CNROM maps the first 32 KiB at $8000-$FFFF as before; the game reaches
; this code through hooks in its own (tools/build.py). The background of the margins is not the
; hack's work: Zaxnes' margin decoder draws it.
;
;  * every sprite of an object within the wide view shows, with the high byte of its x;
;  * an object the game removes as it leaves the screen's left edge stays in the left margin as a
;    ghost until it leaves the view: it goes on moving, but touches nothing, creates no object and
;    makes no sound, so the game is the original's; a new object takes a ghost's slot when no slot
;    is free;
;  * options: the level selector (Select on the title screen), invincibility, no slowdown.
;
; With a margin of 0 every routine does what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1 sprites free of the extents, 2 second page, 4 window to $7FFF
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA
EXTRA_OAM       = $5300         ; the second sprite page (the title's level)

MAX_MARGIN      = 248           ; the most the extension shows sprites in

; Extension RAM ($5200-$57FF; $5800 up is the window).
WV_M            = $5200         ; margin of this frame
WV_A            = $5201         ; scratch
WV_T0           = $5202
WV_T1           = $5203
WV_LEVEL        = $5204         ; the level selected on the title screen, 0-31
WV_MAGIC        = $5205         ; MAGIC once the extension's RAM is set up
WV_GHOST_RUN    = $5206         ; non-zero while a ghost's behaviour runs
WV_SOUND        = $5207         ; the sound asked for before it
WV_GHOST        = $5210         ; + slot: non-zero while the object is a ghost in the left margin
MAGIC           = $A5

; The game.
SCRATCH_LO      = $00           ; the object's half width, 16-bit (sub_8310), at the left test
SCRATCH_HI      = $01
AREA            = $37
ROUND           = $38
DEMO            = $40
SPRITE_X_HI     = $59           ; the object's sprites: x (high, low)
SPRITE_X_LO     = $5A
PART_X_HI       = $5B           ; the sprite being placed: x (high, low)
PART_X_LO       = $5C
OAM_INDEX       = $67
DEATH           = $72
FLAG_70         = $70
HIT_KIND        = $9E           ; $FF: a touch that is not an injury
SOUND           = $A0           ; the sound effect asked for this frame
OBJ_STATE       = $0554         ; + slot: 0 free
OBJ_X_LO        = $0584
OBJ_X_HI        = $0594
OBJ_WIDTH       = $0614         ; + slot, collisions

; Routines and places of the game.
OBJECT_HANDLER  = $929E
SPRITE_SKIP     = $C4C8         ; the sprite loop's next sprite, this one left out
ONE_SPRITE_END  = $C3A3
TITLE_WAIT      = $AE8F         ; the title screen's wait, its demo timer set again

; The options ($500A).
OPT_LEVEL_SELECT = $01
OPT_INVINCIBLE  = $02
OPT_NO_SLOWDOWN = $04
NO_SLOWDOWN_LINES = 255

GHOST_REACH     = 32            ; how far left of the view a ghost's x may be (its sprites' width)
DIGIT_TILE      = $F5           ; the sprite tile of 0 (the status bar's digits)

.segment "CODE"

; ---------------------------------------------------------------------------- ;
; NMI ($B717, in place of LDA #<ram_oam / STA $2003): the extension's settings for the frame.
wide_nmi:
    LDA     WV_MAGIC
    CMP     #MAGIC
    BEQ     @set
    JSR     wide_setup
@set:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    LDA     #$00                        ; no nametable columns: the decoder draws the background
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    LDA     #$16                        ; sprites free of those extents, second sprite page, the
    STA     WIDE_LAYOUT                 ; window on over $6000-$7FFF
    LDA     WIDE_OPTIONS                ; no slowdown (option bit 2)
    AND     #OPT_NO_SLOWDOWN
    BEQ     @overclock
    LDA     #NO_SLOWDOWN_LINES
@overclock:
    STA     WIDE_OVERCLOCK
    LDA     #$00
    STA     $2003
    RTS

; The extension's RAM at power-on: no ghost, the second page hidden.
wide_setup:
    LDX     #$0F
    LDA     #$00
@ghosts:
    STA     WV_GHOST,x
    DEX
    BPL     @ghosts
    STA     WV_LEVEL
    STA     WV_GHOST_RUN
    JSR     wide_hide_page
    LDA     #MAGIC
    STA     WV_MAGIC
    RTS

wide_hide_page:
    LDX     #$00
    LDA     #$F8
@hide:
    STA     EXTRA_OAM,x
    INX
    INX
    INX
    INX
    BNE     @hide
    RTS

; ---------------------------------------------------------------------------- ;
; Sprites. The game places an object's sprites from its x (SPRITE_X, 16-bit, the screen's left
; edge at 0) and leaves out every sprite whose x is not on the screen. Within the wide view a
; sprite shows: its high byte goes to the extension.

; C set when the 16-bit x A:Y (high:low) of a sprite (8 pixels wide) is within the wide view:
; -M-8 < x < 256+M.
wide_x_shown:
    CMP     #$00
    BEQ     @shown
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     WV_M
    BEQ     @hidden
    TYA
    CLC
    ADC     #$08
    BCS     @shown
    ADC     WV_M
    BCS     @shown
    BCC     @hidden
@right:
    CPY     WV_M
    BCC     @shown
@hidden:
    CLC
    RTS
@shown:
    SEC
    RTS

; An object's sprites (sub_C301, $C305, in place of LDY $59 / DEY / BPL RTS: the game gives up an
; object whose x is past the screen's right edge). Given up when it is past the wide view; A (the
; animation) kept.
wide_object_shown:
    STA     WV_A
    LDA     SPRITE_X_HI
    BEQ     @shown
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     SPRITE_X_LO                 ; left: x + 32 + M >= 0 (its sprites reach 32 pixels on)
    CLC
    ADC     #GHOST_REACH
    BCS     @shown
    ADC     WV_M
    BCS     @shown
    BCC     @hidden
@right:
    LDA     SPRITE_X_LO
    CMP     WV_M
    BCC     @shown
@hidden:
    PLA                                 ; out of sub_C301
    PLA
@shown:
    LDA     WV_A
    RTS

; One sprite of an object ($C48B and $C4A9, in place of LDA $5B / BNE skip; X its OAM offset):
; within the wide view it is placed with its high byte, otherwise left out.
wide_sprite:
    STY     WV_T0
    LDY     PART_X_LO
    LDA     PART_X_HI
    JSR     wide_x_shown
    LDY     WV_T0
    BCC     @skip
    LDA     PART_X_HI
    STA     WIDE_SPRHI,x
    RTS
@skip:
    PLA
    PLA
    JMP     SPRITE_SKIP

; A single-sprite object (sub_C301, $C378, in place of LDA $59 / BNE end).
wide_one_sprite:
    LDY     SPRITE_X_LO
    LDA     SPRITE_X_HI
    JSR     wide_x_shown
    BCC     @skip
    LDX     OAM_INDEX
    LDA     SPRITE_X_HI
    STA     WIDE_SPRHI,x
    RTS
@skip:
    PLA
    PLA
    JMP     ONE_SPRITE_END

; ---------------------------------------------------------------------------- ;
; Ghosts. The game removes an object whose left edge (x minus its half width) goes past the
; screen's left edge ($8EA7). Such an object stays as a ghost while it shows in the left margin:
; its behaviour still runs ($8E8E), but it collides with nothing ($BFF2), the objects it would
; create are not ($8DB6) and its sounds are not played, so the game is the original's. A ghost
; that comes back onto the screen goes, as the game had it go.

; The left test ($8EA7, in place of the 16-bit x - half width / BMI remove; $00/$01 the half width):
; C set when the object goes.
wide_left_gone:
    LDA     OBJ_X_LO,x
    SEC
    SBC     SCRATCH_LO
    LDA     OBJ_X_HI,x
    SBC     SCRATCH_HI
    BMI     @left
    LDA     WV_GHOST,x                  ; on the screen: a ghost back there goes
    BNE     @gone
    CLC
    RTS
@left:
    LDA     WV_M                        ; past the left edge: x + 32 + M >= 0 keeps it, a ghost
    BEQ     @gone
    LDA     OBJ_X_HI,x
    BEQ     @ghost
    CMP     #$FF
    BNE     @gone
    LDA     OBJ_X_LO,x
    CLC
    ADC     #GHOST_REACH
    BCS     @ghost
    ADC     WV_M
    BCC     @gone
@ghost:
    LDA     #$01
    STA     WV_GHOST,x
    CLC
    RTS
@gone:
    SEC
    RTS

; The object's behaviour ($8E8E, in place of JSR sub_929E_object_handler): a ghost's runs without
; its sounds or the objects it would create.
wide_handler:
    LDA     WV_GHOST,x
    BNE     @ghost
    JMP     OBJECT_HANDLER
@ghost:
    STA     WV_GHOST_RUN
    LDA     SOUND
    STA     WV_SOUND
    JSR     OBJECT_HANDLER
    LDA     WV_SOUND
    STA     SOUND
    LDA     #$00
    STA     WV_GHOST_RUN
    RTS

; Two objects' collision (sub_BFF2, in place of LDA $0614,X / CLC / ADC $0614,Y): none with a
; ghost.
wide_collision:
    LDA     WV_GHOST,x
    ORA     WV_GHOST,y
    BNE     @none
    LDA     OBJ_WIDTH,x
    CLC
    ADC     OBJ_WIDTH,y
    RTS
@none:
    PLA                                 ; out of sub_BFF2: no collision
    PLA
    CLC
    RTS

; A new object's slot (sub_8DB4, $8DB6, in place of its search): a free one, or else a ghost's;
; none for an object a ghost would create. C set when found, X the slot.
wide_find_slot:
    LDA     WV_GHOST_RUN
    BEQ     @search
    CLC
    RTS
@search:
    LDX     #$0F
@free:
    LDA     OBJ_STATE,x
    BEQ     @found
    DEX
    BNE     @free
    LDX     #$0F
@ghost:
    LDA     WV_GHOST,x
    BNE     @found
    DEX
    BNE     @ghost
    CLC
    RTS
@found:
    LDA     #$00
    STA     WV_GHOST,x
    SEC
    RTS

; An object's state set ($9534, in place of STA $0554,X / RTS: creation, removal): a ghost no more.
wide_object_state:
    STA     OBJ_STATE,x
    PHA
    LDA     #$00
    STA     WV_GHOST,x
    PLA
    RTS

; Every object cleared (sub_8C35, $8C41, in place of STX $0536): no ghost either.
wide_clear_ghosts:
    STX     $0536
    LDA     #$00
    LDY     #$0F
@clear:
    STA     WV_GHOST,y
    DEY
    BPL     @clear
    RTS

; ---------------------------------------------------------------------------- ;
; The level selector (option bit 0): Select on the title screen goes through the 32 rounds, shown
; under the title as "area round"; Start begins there.

; The title screen's buttons ($AE51, in place of CMP #$10 / BNE wait; A the buttons pressed):
; Start goes on as the game does; Select chooses the next round.
wide_title_button:
    CMP     #$10
    BNE     @select
    RTS
@select:
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    BEQ     @wait
    LDA     WV_LEVEL
    CLC
    ADC     #$01
    AND     #31
    STA     WV_LEVEL
    JSR     wide_show_level
@wait:
    PLA
    PLA
    JMP     TITLE_WAIT

; The round selected, in the second sprite page: two digits.
wide_show_level:
    LDA     WV_LEVEL
    LSR
    LSR
    CLC
    ADC     #DIGIT_TILE + 1
    STA     EXTRA_OAM+1
    LDA     WV_LEVEL
    AND     #$03
    CLC
    ADC     #DIGIT_TILE + 1
    STA     EXTRA_OAM+5
    LDA     #$A8                        ; y
    STA     EXTRA_OAM
    STA     EXTRA_OAM+4
    LDA     #$00                        ; palette 0
    STA     EXTRA_OAM+2
    STA     EXTRA_OAM+6
    LDA     #$74                        ; x
    STA     EXTRA_OAM+3
    LDA     #$84
    STA     EXTRA_OAM+7
    RTS

; A new game ($8078, in place of LDA #0 / STA area / STA round): the round selected, with the
; option on (not in the demo).
wide_start_level:
    JSR     wide_hide_page
    LDA     #$00
    STA     AREA
    STA     ROUND
    LDA     DEMO
    BNE     @done
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    BEQ     @done
    LDA     WV_LEVEL
    LSR
    LSR
    STA     AREA
    LDA     WV_LEVEL
    AND     #$03
    STA     ROUND
@done:
    RTS

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1, a testing aid): an enemy's touch ($C04D, in place of LDA $72 / ORA
; $70) is no injury, as if the player were dead already; falls, the water and hunger still kill.
wide_injury:
    LDA     HIT_KIND
    CMP     #$FF
    BEQ     @game
    LDA     WIDE_OPTIONS
    AND     #OPT_INVINCIBLE
    BNE     @done
@game:
    LDA     DEATH
    ORA     FLAG_70
@done:
    RTS

wide_end:
.assert wide_end <= $8000, error, "widescreen code past the window"
```

### B. `tools/build.py`

```python
"""Builds the Adventure Island (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Adventure Island (USA).nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank added after the game's 32 KiB, which the widescreen extension's PRG window shows at
   $5800-$7FFF (Zaxnes opens it at power-on for this patch: the game has no room left for the
   code that would). CNROM maps the first 32 KiB at $8000-$FFFF, so the game runs as it was.
2. Writes the hooks into the game's PRG-ROM, each checked against the bytes it replaces.
3. Writes build/aiwide.nes (header: 3 PRG banks) and, if asked, the BPS patch (data without
   iNES header; the CHR-ROM, moved after the new bank, is copied from the player's ROM).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0xF8A713BE
PRG_SIZE = 0x8000
CHR_SIZE = 0x8000
BANK_SIZE = 0x4000
CODE_START = 0x5800

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

BCS = 0xB0
NOP = 0xEA
RTS = 0x60


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    offset = target - (at + 2)
    assert -128 <= offset <= 127, (hex(at), hex(target))
    return bytes([opcode, offset & 0xFF])


def hooks(sym):
    """The patches of the game's PRG-ROM: (address, original bytes, new bytes)."""
    out = []

    def put(address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((address, original, new))

    # NMI: the extension's settings for the frame (LDA #<ram_oam / STA $2003 done there).
    put(0xB717, 'a9 00 8d 03 20', jsr(sym['wide_nmi']))
    # Sprites in the margins: an object past the screen, each of its sprites, a single sprite.
    put(0xC305, 'a4 59 88 10 68', jsr(sym['wide_object_shown']))
    put(0xC48B, 'a5 5b d0 39', jsr(sym['wide_sprite']))
    put(0xC4A9, 'a5 5b d0 1b', jsr(sym['wide_sprite']))
    put(0xC378, 'a5 59 d0 27', jsr(sym['wide_one_sprite']))
    # Ghosts: objects past the left edge kept in the left margin, without behaviour or collision.
    put(0x8EA7, 'bd 84 05 38 e5 00 bd 94 05 e5 01 30 0e',
        jsr(sym['wide_left_gone']) + branch(BCS, 0x8EAA, 0x8EC2))
    put(0x8E8E, '20 9e 92', jsr(sym['wide_handler']))
    put(0xBFF2, 'bd 14 06 18 79 14 06', jsr(sym['wide_collision']))
    put(0x8DB6, 'bd 54 05 f0 04 ca d0 f8',
        jsr(sym['wide_find_slot']) + branch(BCS, 0x8DB9, 0x8DBF) + bytes([RTS]))
    put(0x9534, '9d 54 05 60', jmp(sym['wide_object_state']))
    put(0x8C41, '8e 36 05', jsr(sym['wide_clear_ghosts']))
    # Options: the level selector on the title screen, invincibility.
    put(0xAE51, 'c9 10 d0 3a', jsr(sym['wide_title_button']))
    put(0x8078, 'a9 00 85 37 85 38', jsr(sym['wide_start_level']))
    put(0xC04D, 'a5 72 05 70', jsr(sym['wide_injury']))
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
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Adventure Island (USA)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    prg = bytearray(data[:PRG_SIZE])
    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for address, original, new in hooks(sym):
        if address in skip:
            continue
        at = address - 0x8000
        assert prg[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        prg[at:at + len(new)] = new
    target = bytes(prg) + code + bytes(data[PRG_SIZE:])

    out_header = bytearray(image[:header])
    if header:
        out_header[4] = (PRG_SIZE + BANK_SIZE) // BANK_SIZE
    out = os.path.join(ROOT, 'build', 'aiwide.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        moved = [(PRG_SIZE + BANK_SIZE, PRG_SIZE + BANK_SIZE + CHR_SIZE, PRG_SIZE)]
        patch = make_bps.make(bytes(data), target, moved)
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
