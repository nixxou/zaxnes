# Super Mario Bros. 3 (USA) (Rev 1): the widescreen hack

This document describes completely the modification of *Super Mario Bros. 3 (USA) (Rev 1)* that
Zaxnes ships as a built-in patch (advanced mode). The appendices hold the new code and the build
script, which writes every hook after checking the bytes it replaces: with them the ROM can be
rebuilt identically.

The background of the margins is not the hack's work: the emulator's Super Mario Bros. 3 margin
decoder reads the level's tile map in PRG-RAM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4); it also widens the
unpatched game, which is the simple mode. The hack does the objects: enemies created, kept and
shown in the margins, and the options.

The disassembly used to study the game (captainsouthbird/smb3, which builds this ROM byte for
byte) has no licence: nothing of it is copied here, the names of the game's routines and
variables below are only its labels, to find them.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Super Mario Bros. 3 (USA) (Rev 1)*, MMC3, 256 KiB PRG (32 banks of 8 KiB; in the game's PRG mode bank 30 is fixed at `$8000` and bank 31 at `$E000`, `$A000` and `$C000` switch), 128 KiB CHR-ROM, 8 KiB PRG-RAM |
| Source data (iNES header excluded) | CRC-32 `2E6301ED` |
| Target data | CRC-32 `9B0D1A59` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | 128 bytes at `$E240` of bank 31 (the `$FF` padding before a DMC sample), 2 KiB at `$1600`-`$1DFF` of bank 25 (its empty end), hooks in banks 0, 3, 4, 5, 24, 30 and 31; size and layout unchanged |
| Patch | `rust/crates/zax-nes/patches/smb3-usa-rev1-widescreen.bps`, made against the data without header |

The ROM keeps its size: MMC3 fixes the last banks, and Zaxnes' savestates keep the cartridge's bank
windows as offsets, so a larger PRG-ROM would move the fixed banks under a state of the unpatched
game. A state made without the hack restores with it as it is.

---

**Hack pack.** Zaxnes ships this hack as a hack pack,
`rust/crates/zax-nes/patches/smb3-usa-rev1-widescreen.zxpk` (`tools/build.py <rom> --pack
<out.zxpk>`; `docs/HACK-PACKS.md`, `WIDESCREEN-EXTENSION.md` 7.1): the code goes to the extension's
memory instead of the game's free space described here (the source assembled for `$5800`, without
the reset hook and the NMI's unlock: Zaxnes unlocks the extension and shows the code in the window
from power-on, `$5008` = `$80`, page 0), and the hooks, each with the game's bytes it replaces, are
checked and applied by Zaxnes; the ROM keeps its size (nothing written in its free space). Its
testing aids need the hack's code: they stay with the widescreen (the advanced mode). The BPS patch
described here is kept to compare.

## 2. The extension and the code's place

The game's NMI (`IntNMI`, `$F486`) saves the registers and jumps to `$9F40`; the hook at `$F496`
jumps to `fixed_nmi` (bank 31, `$E240`) instead. It unlocks the extension when `$5000` does not read
`$57` (`$57 $49 $44 $45` to `$5000`, then checks `$5000` again; without the extension it goes straight
on with the game), opens the PRG window on 16 KiB bank 12 from page `$36` (`$5008 = $8C`, `$5009 =
$36`: bank 25's `$1600` at `$5800`), calls `wide_nmi` there and goes on to `$9F40`. Every other
routine of the hack is in the window, which shows the same code whatever the game maps at `$8000`-
`$FFFF`.

`wide_nmi`, each frame: `M = min($5002, 248)` (`WV_M`), `$5003 = M`, `$5004 = $5005 = 0` (no
nametable columns: the decoder draws the background), `$5007 = $02` (sprites free of those extents),
`M16` = M rounded up to 16 pixels (16-bit, `WV_M16`), `$5006 = 255` with the option "No slowdown"
(`$500A` bit 2), else 0.

Variables (extension RAM): `WV_M $5200`, scratch `WV_T0 $5201` / `WV_T1 $5202`, an object's
screen x + 8 `WV_EST $5203`/`$5204`, `WV_COUNT $5205`, `WV_M16 $5206`/`$5207`, `WV_DEL_HI $5208`,
the sprite block the game gave each object this frame `WV_SPR_BASE $5210 + slot`, X saved
`WV_SAVE_X $5209`, the object that drew each sprite this frame `WV_OWNER $5240 + sprite` (slot + 1,
0 for none), the creation pass ahead `WV_AHEAD $520A` and the game's column meanwhile
`WV_ORIG $520B`/`$520C`, Mario's right limit on the screen `WV_RIGHT $520D`/`$520E` and the
level end's `WV_END $5218`/`$5219`;
the trace build adds the lag counters at `$57F0`.

---

## 3. What the game does, and what changes

Objects: 8 slots (0-4 the level's enemies, 5-7 special ones), x `$91 + slot` / `$76 + slot`, y
`$A3` / `$88`, state `$0661` (0 empty, 1 initialising). Each frame an object gets the six sprites at
`Object_SprRAM` (`$058F + slot`), one of `$40`, `$58`, `$70`, `$88`, `$A0`, `$B8`, `$D0`, `$E8`
(turning every frame against flicker); the draw routines write x = 8-bit screen x + offset, and leave
out the 8-pixel columns marked off the screen in `Objects_SprHVis` (`$0651 + slot`). Camera
`$12:$FD`.

| Game routine | Original | Hack |
|---|---|---|
| horizontal visibility (`Object_DetermineHorzVisY`, `$D7B8`, bank 0) | a column is off the screen unless its x starts within `[0, 256)` | within `[-M, 256 + M)` (`wide_horz_vis`; as the game, Temp_Var15/16 keep the first column's x) |
| sprite block of an object (`Objects_HandleScrollAndUpdate`, `$C9BC`, bank 0) | `Object_SprRAM = SprRamOffsets[slot + Counter_7to0]`; a troopa then adds 8 to it and draws its head tip and wing in the two sprites below | the same, also kept in `WV_SPR_BASE` (`wide_spr_base`) |
| end of the frame's work (`GraphicsBuf_Prep_And_WaitVSync`, `$96E5`, bank 30; the next NMI's OAM DMA follows) | | the high byte of the x of each of the six sprites of every object in a normal state: the instance of its 8-bit x nearest the object's screen x + 8, to `$5100 + OAM offset` (`wide_frame_end`; not on the world map). The six from `WV_SPR_BASE` when `Object_SprRAM` is it or 8 more, else from `Object_SprRAM`; then every sprite `WV_OWNER` names, from its object (the end-of-level card draws in a block of its own), the table cleared as it is read |
| sprite writers (`Object_Draw16x16Sprite`, `$D680`, two 8 x 16 sprites from `Sprite_RAM + Y`; `Object_Draw24x16Sprite`, `$D6C7`, three) | | the object being drawn (`SlotIndexBackup`) noted in `WV_OWNER` for those sprites (`wide_own2`, `wide_own3`) |
| creation as the screen scrolls (`Level_ObjectsSpawnByScroll`, called at `$B82E`, bank 5) | the level's objects are created when the column 16 pixels past the right edge (scrolling right) or 32 past the left one is exactly on them | run twice (`wide_spawn_twice`): first a pass ahead, `M16` further (`wide_spawn_ahead`, `$B838`), which creates any object not created yet between the game's column and that one (`wide_spawn_test`, `$B884`): what an area starts with in between (an end-of-level card after a pipe) would otherwise never meet a column that starts past it; then the game's own pass, as the game. An object already created is not created again |
| deletion off the screen (`Object_DeleteOffScreen`, `$D40F` and `$D41E`, bank 0) | an object off the screen goes past `+$120`/`-$30` (`+$180`/`-$80`, `+$140`/`-$50` for some) of the camera | `M16` further (`wide_delete_low`, `wide_delete_high`): one created there would otherwise go at once. An object within the wide view has no column off the screen, so the game's test does not even start |
| creation of a special object by an object (`SpecialObj_FindEmptyAbort`, `$C454`; the Lava Lotus, `$B0C1`; the Hammer Bro, `$A5E6`) | none while the object is off the screen | the same with the original screen (`wide_original_offscreen`, `wide_original_hoffscreen`): special objects (hammers, fireballs, boomerangs) have an 8-bit x relative to the screen, one thrown from a margin would show and hit 256 pixels away |

Mario:

| Game routine | Original | Hack |
|---|---|---|
| Mario's limits on the screen (`PRG008_B1CE`, bank 8) | his sprite x held within 16-232 (the camera following him, this matters at the level's ends) | moving right, within 232 + M of the screen's left edge (16-bit), never past the level's last screen's 232 (`wide_player_limit`); the left limit as the game |
| Mario and a screen that scrolls by itself (`AScroll_MovePlayer`, `PRG009_BCC0`, bank 9) | under 16 or from 224 on, moving into that edge, he gets the screen's speed: a wall at the original right edge in the middle of the wide view (1-4, the airships) | the right edge at 224 + M, never past the level's last screen's 224 (`wide_ascroll_edge`); the left one as the game |
| crushed by the scrolling (`PRG008_A45A`, bank 8) | a sprite x of `$F8`-`$FF`: pushed 1-8 pixels off the left edge | a 16-bit x of -8 to -1 (`wide_crushed`): Mario past 247 is not crushed |
| Mario's sprites (`wide_frame_end`) | | his six sprites (`Player_SprOff`, `$058E`) get the high byte from his screen x + 8 |

Without margin and in a vertical level, these routines run the game's code as it is.

The game's collisions (Mario and an object, two objects) compare 8-bit screen boxes first and then
the full 16-bit positions: an object of the margins does not touch Mario. An object in the wide view
counts as on the screen for its behaviour (a Paratroopa flies, a Piranha Plant comes out), but
creates no special object until it is on the original screen. With M = 0 every routine does exactly
what the code it replaces does.

CPU time: the enemies of the margins cost time: 1500 frames of 1-1 with a 112-pixel margin lag 30
times (3 in the original game, counted by the trace build: an NMI that finds the main loop busy,
`VBlank_TickEn` `$1C` at 0). With "No slowdown" (255 lines of overclock, on by default), none;
Zaxnes ends the overclock when the game waits for the NMI (`idle_loop` `$96FB`-`$96FE`, `LDA
VBlank_Tick / BPL` in `GraphicsBuf_Prep_And_WaitVSync`), so it costs nothing when not needed.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Bank | Address | Replaces | With |
|---|---|---|---|
| 31 | `$F496` | `JMP $9F40` | `JMP fixed_nmi` |
| 0 | `$D7B8` | `LDA #0 / STA $0651,X` | `JMP wide_horz_vis` |
| 0 | `$C9BC` | `LDA $C401,Y / STA $058F,X` | `JSR wide_spr_base` |
| 30 | `$96E5` | `LDA $5E / ASL A / TAY` | `JSR wide_frame_end` |
| 0 | `$D680` | `LDA $04 / LSR A` | `JSR wide_own2` |
| 0 | `$D6C7` | `LDA $04 / LSR A` | `JSR wide_own3` |
| 5 | `$B82E` | `JMP $B836` (Level_ObjectsSpawnByScroll) | `JMP wide_spawn_twice` |
| 5 | `$B884` | `ASL A x 4 / CMP $05` | `JSR wide_spawn_test` |
| 5 | `$B838` | `LDA $FD / CLC / ADC $B832,Y / AND #$F0 / STA $05 / LDA $12 / ADC $B834,Y` | `JSR wide_spawn_ahead / JMP $B847` |
| 0 | `$D40F` | `LDA $FD / CLC / ADC $D3CC,Y` | `JSR wide_delete_low` |
| 0 | `$D41E` | `ADC $D3D2,Y` | `JSR wide_delete_high` |
| 8 | `$B1CE` | `LDA $AB / LDY $BD` | `JMP wide_player_limit` |
| 9 | `$BCC0` | `LDA $AB / CMP #16 / BCC / CMP #224 / BCC` | `JMP wide_ascroll_edge` |
| 8 | `$A45A` | `LDA $AB / CMP #$F8` | `JSR wide_crushed` |
| 0 | `$C454` | `JSR $DD54` (Object_AnySprOffscreen) | `JSR wide_original_offscreen` |
| 3 | `$B0C1` | `JSR $DD54` | `JSR wide_original_offscreen` |
| 4 | `$A5E6` | `LDA $0651,X` | `JSR wide_original_hoffscreen` |
| 24 | `$AC35` | `LDA $F8 / AND #$C0 / CMP #$C0 / BNE / NOP x 5` | `JSR wide_title / JMP $AC42` |

Banks 0, 3, 4 and 5 are those the game maps (at `$C000` or `$A000`) while the routines run.

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there.

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (B on the 1P/2P menu) / Sélecteur de niveau (B sur le menu 1P/2P) | off | a testing aid: B on the title screen's menu opens the game's own debug menu (title state 7, no way in left in the game): Up and Down choose the world (a cursor at the left, world 1 at the top), Start goes to its map in the game's debug mode (no time limit, Select changes the suit, the inventory full of P-Wings) |
| 1 | `invincible` | Invincible | off | a testing aid: the game's unused debug invincibility (`$7CFF`, which its damage routine tests) set every frame; falling and the timer still kill |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock: the game never runs late |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `PATCHES`); all 0 is
the hack with the game's slowdowns.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Super Mario Bros. 3 (USA) (Rev 1).nes" build/smb3wide.bps
python -I tools/make_doc.py WIDESCREEN-HACK.md
```

`build.py` assembles `src/wide.s` (`-g`, labels from `ld65 -Ln`) into the FIXED segment (`$E240`,
128 bytes) and the WINDOW segment (`$5800`, 2 KiB), checks that both places are still `$FF`, writes
the code and the hooks, `build/smb3wide.nes` and the BPS patch (`tools/make_bps.py`, SourceRead /
TargetRead only). `WIDE_SKIP=<hex addresses>` leaves hooks out; `WIDE_DEFINES=LAG_TRACE` builds the
lag counters (`$57F0` lag frames, `$57F2` NMIs, 16-bit), for measuring only.

## 7. Limits

* Special objects (hammers, fireballs...) still vanish at the original edges, and the objects of
  the margins throw none.
* Objects are created as far ahead as the margin: they start moving that much earlier than in the
  original game; the level events the list holds (Cheep Cheep waves and the like) start that much
  earlier too. The game has 5 slots for the level's enemies; one that finds none free when its
  column comes is not created, as in the original game, which happens a little more often with
  more enemies alive.
* Sprites an object writes itself outside its block and outside the game's two sprite writers get
  no high byte from it: they show 256 pixels away while it is in a margin (none met besides the
  troopas' head and wing, which are handled).
* The vertical levels (four pipe mazes) get no margins from the decoder.
* Mario goes into the right margin only; the left edge still pushes him where the screen scrolls by
  itself (the game's crush test needs it). Objects that look at Mario's 8-bit sprite x (a few turn
  to face him) see him on the wrong side while he is past 255.

---

## Appendices

### A. `src/wide.s`

```asm
; Super Mario Bros. 3 (USA) (Rev 1): the widescreen hack (Zaxnes, advanced mode).
;
; Two pieces of code:
;
; * FIXED, 128 bytes at $E240 of PRG bank 31 (the game's last bank, fixed at $E000; the space is
;   the padding before a DMC sample). The NMI jumps there first: it unlocks the widescreen
;   extension when it is not, opens its PRG window on the hack's bank and calls into it. Without
;   the extension (real hardware, another emulator) it goes straight on with the game's NMI.
; * WINDOW, 2 KiB at the free end of PRG bank 25 (16 KiB bank 12 from its page $36), which the
;   extension's PRG window shows at $5800-$5FFF whatever the cartridge maps.
;
; The ROM keeps its size and layout: a state of the unpatched game restores into the patched one.
;
; With PACK defined (a Zaxnes hack pack), WINDOW only, for $5800 of the extension's memory, which
; Zaxnes shows in the PRG window from power-on, the extension unlocked: no FIXED, the NMI hook jumps
; to pack_nmi in the window (the code only runs while the extension is unlocked, the window open).
;
; The background of the margins is not the hack's work: Zaxnes' margin decoder draws it.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_UNLOCK     = $5000         ; W: "WIDE"; R: $57 once unlocked
WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1: sprites free of the extents
WIDE_WINDOW     = $5008         ; bit 7 on, bits 0-6 the 16 KiB bank shown at $5800
WIDE_PAGE       = $5009         ; its first 256-byte page
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA

HACK_BANK       = 12            ; the 16 KiB bank of the window's code
HACK_PAGE       = $36           ; and its first page
MAX_MARGIN      = 248           ; the most the extension shows sprites in

; Extension RAM ($5200-$57FF).
WV_M            = $5200         ; margin of this frame
WV_T0           = $5201         ; scratch
WV_T1           = $5202
WV_EST_LO       = $5203         ; an object's x on the screen, 16-bit, plus 8
WV_EST_HI       = $5204
WV_COUNT        = $5205
WV_M16_LO       = $5206         ; the margin rounded up to 16 pixels, 16-bit
WV_M16_HI       = $5207
WV_DEL_HI       = $5208         ; the high byte of the deletion limit at hand
WV_SPR_BASE     = $5210         ; + slot: the six sprites the game gave the object this frame
WV_SAVE_X       = $5209
WV_AHEAD        = $520A         ; non-zero during the creation pass as far ahead as the margin
WV_ORIG_LO      = $520B         ; the game's own creation column during that pass (16-bit)
WV_ORIG_HI      = $520C
WV_RIGHT_LO     = $520D         ; how far right of the screen's left edge Mario may go (16-bit)
WV_RIGHT_HI     = $520E
WV_END_LO       = $5218         ; the same, as the game allows it at the end of the level
WV_END_HI       = $5219
WV_OWNER        = $5240         ; + sprite (66 bytes): 1 + the slot of the object that drew it
.ifdef LAG_TRACE
WV_LAG          = $57F0         ; 2 bytes: NMIs that found the game still busy (a trace build)
WV_NMI          = $57F2         ; 2 bytes: NMIs
.endif

; The patch's options ($500A).
OPT_LEVEL_SELECT = $01          ; a testing aid: B on the 1P/2P menu opens the game's debug menu
OPT_INVINCIBLE  = $02           ; a testing aid: Mario is never hurt (pits still kill)
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late
NO_SLOWDOWN_LINES = 255

; The game.
VBLANK_TICK_EN  = $1C           ; non-zero while the main loop waits for the NMI
GAME_NMI        = $9F40         ; PRG030_SUB_9F40, where the game's NMI goes on
TEMP_VAR15      = $0E
TEMP_VAR16      = $0F
HORZ_SCROLL_HI  = $12
GRAPHICS_QUEUE  = $5E
OBJECTS_XHI     = $76           ; + slot
OBJECTS_X       = $91           ; + slot
HORZ_SCROLL     = $FD
SPRITE_RAM      = $0200
OBJECT_SPR_RAM  = $058F         ; + slot: the Sprite_RAM offset of its six sprites this frame
OBJECTS_SPR_HVIS = $0651        ; + slot: a bit per 8-pixel column off the screen
OBJECTS_STATE   = $0661         ; + slot: 0 empty, 1 initialising
OBJECTS_SPR_VVIS = $0681        ; + slot: a bit per 16-pixel row off the screen
OBJ_GROUP_REL_IDX = $03E2       ; the object's index in its group's tables
OBJECT_GROUP_ATTRIBUTES = $A0D8 ; the group's bank (at $A000 while it runs): bits 4-6 the width
LEVEL_TILESET   = $070A         ; 0: the world map
SPR_RAM_OFFSETS = $C401         ; bank 0: the sprite blocks the objects take in turn
OBJECT_SLOTS    = 8
SPRITE_COUNT    = 64
SLOT_INDEX_BACKUP = $CD         ; the object being run
TEMP_VAR5       = $04
OBJECT_SPRITES  = 6
OBJECT_WIDTHS   = $D792         ; bank 0 (at $C000 while objects run): column x offsets
OBJECT_WIDTH_FLAGS = $D798      ; and their bits in Objects_SprHVis
TEMP_VAR6       = $05
TEMP_VAR7       = $06
SCROLL_LAST_DIR = $5B           ; 0 the screen last moved right, 1 left
LEVEL_WIDTH     = $22           ; the level's last screen
PLAYER_XHI      = $75
PLAYER_X        = $90
PLAYER_SPRITE_X = $AB
PLAYER_XVEL     = $BD
PLAYER_SPR_OFF  = $058E         ; the Sprite_RAM offset of Mario's six sprites
LEVEL_VERTICAL  = $03EF
PLAYER_LIMIT_TEST = $B1D2       ; bank 8: the game's screen limit for Mario, from its BMI
PLAYER_LIMIT_SLOW = $B1FC       ; its velocity cut once Mario is held at a limit
PLAYER_LIMIT_DONE = $B208       ; and what follows it
LEVEL_ASCRL_HVEL = $7A0E        ; the screen's own horizontal speed while it scrolls by itself
PLAYER_XVEL_FRAC = $074D
ASCROLL_PRESSED = $BCCA         ; bank 9 (AScroll_MovePlayer): Mario against an edge
ASCROLL_FREE    = $BCE0         ; not against one
ASCROLL_ON      = $BCF1         ; after the edges
PAD_INPUT       = $18           ; the buttons pressed this frame
PAD_B           = $40
TITLE_STATE     = $DE
TITLE_DEBUG_MENU = $07          ; its state with no way in left in the game
PLAYER_DEBUG_NO_HIT = $7CFF     ; the game's unused debug invincibility, cleared at each level
LEVEL_OBJECTS_SPAWN_BY_SCROLL = $B836 ; bank 5
LOSBS_LOOK_AHEAD = $B832        ; bank 5 (at $A000 while objects spawn): 16 and -32, low
LOSBS_LOOK_AHEAD_HI = $B834     ; and high
DELETE_RANGE_LO = $D3CC         ; bank 0: the deletion limits, right (even) and left (odd)
DELETE_RANGE_HI = $D3D2

.ifndef PACK
; ---------------------------------------------------------------------------- ;
.segment "FIXED"

; The NMI ($F496, in place of JMP PRG030_SUB_9F40; the registers are saved).
fixed_nmi:
    LDA     WIDE_UNLOCK
    CMP     #$57
    BEQ     @open
    LDA     #$57                        ; unlock
    STA     WIDE_UNLOCK
    LDA     #$49
    STA     WIDE_UNLOCK
    LDA     #$44
    STA     WIDE_UNLOCK
    LDA     #$45
    STA     WIDE_UNLOCK
    LDA     WIDE_UNLOCK
    CMP     #$57
    BNE     @game                       ; not the extension
@open:
    LDA     #$80 | HACK_BANK            ; the window on the hack's bank (again after a state of
    STA     WIDE_WINDOW                 ; the unpatched game)
    LDA     #HACK_PAGE
    STA     WIDE_PAGE
    JSR     wide_nmi
@game:
    JMP     GAME_NMI
.endif

; ---------------------------------------------------------------------------- ;
.segment "WINDOW"

.ifdef PACK
; The NMI ($F496, in place of JMP PRG030_SUB_9F40; the registers are saved).
pack_nmi:
    JSR     wide_nmi
    JMP     GAME_NMI
.endif

; The extension's settings for the frame.
wide_nmi:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    CLC                                 ; rounded up to 16 pixels
    ADC     #15
    AND     #$F0
    STA     WV_M16_LO
    LDA     #$00
    ROL     A
    STA     WV_M16_HI
    LDA     #$00                        ; no nametable columns: the decoder draws the background
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    LDA     #$02                        ; sprites free of those extents
    STA     WIDE_LAYOUT
    LDA     WIDE_OPTIONS                ; no slowdown (option bit 2)
    AND     #OPT_NO_SLOWDOWN
    BEQ     @overclock
    LDA     #NO_SLOWDOWN_LINES
@overclock:
    STA     WIDE_OVERCLOCK
.ifdef LAG_TRACE
    INC     WV_NMI
    BNE     @counted
    INC     WV_NMI + 1
@counted:
    LDA     VBLANK_TICK_EN              ; the main loop was not waiting: a lag frame
    BNE     @on_time
    INC     WV_LAG
    BNE     @on_time
    INC     WV_LAG + 1
@on_time:
.endif
    RTS

; ---------------------------------------------------------------------------- ;
; An object's horizontal visibility (Object_DetermineHorzVisY, $D7B8, in place of LDA #0 / STA
; Objects_SprHVis,X; Y is the width index, the object is in a normal state). The game marks off
; the screen every 8-pixel column of the object that does not start within the screen, and draws
; none of their sprites; here a column starting within the margins shows too. As the game, it
; leaves Temp_Var15/16 with the object's x + Object_Widths[0].
wide_horz_vis:
    LDA     #$00
    STA     OBJECTS_SPR_HVIS,x
@column:
    LDA     OBJECTS_X,x
    CLC
    ADC     OBJECT_WIDTHS,y
    STA     TEMP_VAR15
    LDA     OBJECTS_XHI,x
    ADC     #$00
    STA     TEMP_VAR16
    LDA     TEMP_VAR15                  ; the column's x on the screen, 16-bit
    SEC
    SBC     HORZ_SCROLL
    STA     WV_T0
    LDA     TEMP_VAR16
    SBC     HORZ_SCROLL_HI
    BEQ     @shown                      ; on the screen
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     WV_T0                       ; left: x >= -M
    CLC
    ADC     WV_M
    BCS     @shown
    BCC     @hidden
@right:
    LDA     WV_T0                       ; right: x < 256 + M
    CMP     WV_M
    BCC     @shown
@hidden:
    LDA     OBJECTS_SPR_HVIS,x
    ORA     OBJECT_WIDTH_FLAGS,y
    STA     OBJECTS_SPR_HVIS,x
@shown:
    DEY
    BPL     @column
    RTS

; ---------------------------------------------------------------------------- ;
; The end of a frame's work (GraphicsBuf_Prep_And_WaitVSync, $96E5, in place of LDA
; Graphics_Queue / ASL A / TAY; the OAM DMA of the next NMI follows). The objects' sprites have
; their x on 8 bits: each gets the high byte that puts it nearest the object that drew it, for the
; extension. First the six sprites of each object's block, then every sprite the game's draw
; routines wrote this frame (WV_OWNER, see wide_own), which covers the objects that draw
; elsewhere (the end-of-level card). Not on the world map.
wide_frame_end:
    LDA     WIDE_OPTIONS                ; invincibility (option bit 1)
    AND     #OPT_INVINCIBLE
    BEQ     @hurt
    LDA     #$01
@hurt:
    STA     PLAYER_DEBUG_NO_HIT
    TXA
    PHA
    LDA     WV_M
    BEQ     @clear
    LDA     LEVEL_TILESET
    BNE     @level
@clear:
    LDX     #SPRITE_COUNT - 1           ; no high bytes: only forget who drew what
    LDA     #$00
@forget:
    STA     WV_OWNER,x
    DEX
    BPL     @forget
    JMP     @done
@level:
    LDX     #OBJECT_SLOTS - 1
@object:
    JSR     wide_estimate
    BCC     @next
    LDA     OBJECT_SPR_RAM,x            ; its block: the one the game gave it, which the troopas
    SEC                                 ; move up 8 bytes to draw their head and wing below it;
    SBC     WV_SPR_BASE,x               ; where the object put it otherwise
    BEQ     @given
    CMP     #$08
    BEQ     @given
    LDA     OBJECT_SPR_RAM,x
    JMP     @block
@given:
    LDA     WV_SPR_BASE,x
@block:
    TAY
    LDA     #OBJECT_SPRITES
    STA     WV_COUNT
@sprite:
    JSR     wide_sprite_high
    INY
    INY
    INY
    INY
    DEC     WV_COUNT
    BNE     @sprite
@next:
    DEX
    BPL     @object

    LDA     PLAYER_X                    ; Mario's six sprites, from his x on the screen + 8
    SEC
    SBC     HORZ_SCROLL
    STA     WV_EST_LO
    LDA     PLAYER_XHI
    SBC     HORZ_SCROLL_HI
    STA     WV_EST_HI
    LDA     WV_EST_LO
    CLC
    ADC     #$08
    STA     WV_EST_LO
    BCC     @player
    INC     WV_EST_HI
@player:
    LDY     PLAYER_SPR_OFF
    LDA     #OBJECT_SPRITES
    STA     WV_COUNT
@player_sprite:
    JSR     wide_sprite_high
    INY
    INY
    INY
    INY
    DEC     WV_COUNT
    BNE     @player_sprite

    LDY     #(SPRITE_COUNT - 1) * 4     ; the sprites the draw routines wrote
@owned:
    TYA
    LSR     A
    LSR     A
    TAX
    LDA     WV_OWNER,x
    BEQ     @owned_next
    PHA
    LDA     #$00
    STA     WV_OWNER,x
    PLA
    TAX
    DEX
    JSR     wide_estimate
    BCC     @owned_next
    JSR     wide_sprite_high
@owned_next:
    DEY
    DEY
    DEY
    DEY
    CPY     #$FC
    BNE     @owned
@done:
    PLA
    TAX
    LDA     GRAPHICS_QUEUE
    ASL     A
    TAY
    RTS

; Object X's x on the screen + 8, 16-bit, into WV_EST; C clear when it is not in a normal state.
wide_estimate:
    LDA     OBJECTS_STATE,x
    CMP     #$02
    BCC     @none
    LDA     OBJECTS_X,x
    SEC
    SBC     HORZ_SCROLL
    STA     WV_EST_LO
    LDA     OBJECTS_XHI,x
    SBC     HORZ_SCROLL_HI
    STA     WV_EST_HI
    LDA     WV_EST_LO
    CLC
    ADC     #$08
    STA     WV_EST_LO
    BCC     @done
    INC     WV_EST_HI
@done:
    SEC
@none:
    RTS

; The high byte of the sprite at Sprite_RAM + Y, the instance of its 8-bit x nearest WV_EST, to
; the extension (none for a hidden sprite).
wide_sprite_high:
    LDA     SPRITE_RAM,y
    CMP     #$F0
    BCS     @hidden
    LDA     SPRITE_RAM + 3,y            ; x - estimate, 8-bit signed
    SEC
    SBC     WV_EST_LO
    STA     WV_T0
    LDA     #$00                        ; sign extended
    BIT     WV_T0
    BPL     @positive
    LDA     #$FF
@positive:
    STA     WV_T1
    LDA     WV_EST_LO                   ; estimate + difference, high byte
    CLC
    ADC     WV_T0
    LDA     WV_EST_HI
    ADC     WV_T1
    STA     WIDE_SPRHI,y
@hidden:
    RTS

; ---------------------------------------------------------------------------- ;
; The game's sprite writers of objects (Object_Draw16x16Sprite, $D680: two 8 x 16 sprites from
; Sprite_RAM + Y; Object_Draw24x16Sprite, $D6C7: three; in place of LDA Temp_Var5 / LSR A, done
; here): the object being drawn (SlotIndexBackup) is noted as the one that drew them (WV_OWNER,
; per sprite, slot + 1).
wide_own3:
    STX     WV_SAVE_X
    TYA
    LSR     A
    LSR     A
    TAX
    LDA     SLOT_INDEX_BACKUP
    CLC
    ADC     #$01
    STA     WV_OWNER + 2,x
    JMP     wide_own_two
wide_own2:
    STX     WV_SAVE_X
    TYA
    LSR     A
    LSR     A
    TAX
    LDA     SLOT_INDEX_BACKUP
    CLC
    ADC     #$01
wide_own_two:
    STA     WV_OWNER,x
    STA     WV_OWNER + 1,x
    LDX     WV_SAVE_X
    LDA     TEMP_VAR5
    LSR     A
    RTS

; ---------------------------------------------------------------------------- ;
; The creation of the level's objects as the screen scrolls (PRG005_B82E, in place of JMP
; Level_ObjectsSpawnByScroll): run twice, first at the column as far ahead as the margin, then at
; the game's own. The game creates an object only when that column is exactly on it: the column
; ahead alone, which starts that much further where an area starts, would never meet the objects
; in between (an end-of-level card right after a pipe). One already created is not created again.
wide_spawn_twice:
    LDA     WV_M16_LO                   ; no margin: the game's pass alone
    ORA     WV_M16_HI
    BEQ     @own
    LDA     #$01
    STA     WV_AHEAD
    JSR     LEVEL_OBJECTS_SPAWN_BY_SCROLL
@own:
    LDA     #$00
    STA     WV_AHEAD
    JMP     LEVEL_OBJECTS_SPAWN_BY_SCROLL

; ---------------------------------------------------------------------------- ;
; The column where objects of the level are created as the screen scrolls
; (Level_ObjectsSpawnByScroll, $B838, in place of its computation of Temp_Var6/7; Y is
; Scroll_LastDir, 0 right, 1 left). The game creates them 16 pixels past the right edge or 32 past
; the left one; here as much further as the margin, so that they show once they come into the
; wide view. Temp_Var6 gets the column, A its high byte.
wide_spawn_ahead:
    LDA     HORZ_SCROLL
    CLC
    ADC     LOSBS_LOOK_AHEAD,y
    STA     WV_T0
    LDA     HORZ_SCROLL_HI
    ADC     LOSBS_LOOK_AHEAD_HI,y
    STA     WV_T1
    LDA     WV_AHEAD                    ; the game's own pass: its column
    BEQ     @column_low
    LDA     WV_T0                       ; the game's column, for wide_spawn_test
    AND     #$F0
    STA     WV_ORIG_LO
    LDA     WV_T1
    STA     WV_ORIG_HI
    CPY     #$00
    BNE     @left
    LDA     WV_T0
    CLC
    ADC     WV_M16_LO
    STA     WV_T0
    LDA     WV_T1
    ADC     WV_M16_HI
    JMP     @column
@left:
    LDA     WV_T0
    SEC
    SBC     WV_M16_LO
    STA     WV_T0
    LDA     WV_T1
    SBC     WV_M16_HI
@column:
    STA     WV_T1
@column_low:
    LDA     WV_T0
    AND     #$F0
    STA     TEMP_VAR6
    LDA     WV_T1
    RTS

; ---------------------------------------------------------------------------- ;
; The limits past which an object off the screen is deleted (Object_DeleteOffScreen: $D40F in
; place of LDA Horz_Scroll / ADD PRG000_D3CC,Y, and $D41E in place of ADC PRG000_D3D2,Y; Y 0-5,
; even the right limit, odd the left one). As much further as the margin, as objects are created
; (see wide_spawn_ahead): one created there would otherwise be deleted at once.
wide_delete_low:
    LDA     HORZ_SCROLL
    CLC
    ADC     DELETE_RANGE_LO,y
    STA     WV_T0
    LDA     HORZ_SCROLL_HI
    ADC     DELETE_RANGE_HI,y
    STA     WV_DEL_HI
    TYA
    LSR     A
    BCS     @left
    LDA     WV_T0
    CLC
    ADC     WV_M16_LO
    STA     WV_T0
    LDA     WV_DEL_HI
    ADC     WV_M16_HI
    JMP     @limit
@left:
    LDA     WV_T0
    SEC
    SBC     WV_M16_LO
    STA     WV_T0
    LDA     WV_DEL_HI
    SBC     WV_M16_HI
@limit:
    STA     WV_DEL_HI
    LDA     WV_T0
    RTS

wide_delete_high:
    LDA     WV_DEL_HI
    RTS

; ---------------------------------------------------------------------------- ;
; The 1P/2P menu of the title screen (Title_Do1P2PMenu, $AC35, in place of its test of the second
; controller's A+B, whose action the game left as NOPs). With the level select option, B opens the
; game's own debug menu (Title_DebugMenu, title state 7): Up and Down choose the world (a cursor
; at the left, world 1 at the top), Start goes to its map in the game's debug mode (no time
; limit, Select changes the suit, the inventory full of P-Wings).
wide_title:
    LDA     WIDE_OPTIONS
    AND     #OPT_LEVEL_SELECT
    BEQ     @done
    LDA     PAD_INPUT
    AND     #PAD_B
    BEQ     @done
    LDA     #TITLE_DEBUG_MENU
    STA     TITLE_STATE
@done:
    RTS

; ---------------------------------------------------------------------------- ;
; Whether object X is off the original screen, as the game decides it (Object_AnySprOffscreen):
; A non-zero (Z clear) when some 8-pixel column of it does not start within the screen, or some row
; is off it vertically. The special objects (hammers, fireballs, boomerangs...) have an 8-bit x
; only, relative to the screen: one an object of the margins threw would show and hit 256 pixels
; away. So the routines that create them from an object keep the original screen
; (SpecialObj_FindEmptyAbort at $C454 and the Lava Lotus at $B0C1, in place of JSR
; Object_AnySprOffscreen; the Hammer Bro at $A5E6, in place of LDA Objects_SprHVis,X: the
; horizontal test only).
wide_original_offscreen:
    LDA     OBJECTS_SPR_VVIS,x
    BNE     wide_original_done
wide_original_hoffscreen:
    TYA
    PHA
    LDY     OBJ_GROUP_REL_IDX           ; the object's last column (Object_DetermineHorzVis)
    LDA     OBJECT_GROUP_ATTRIBUTES,y
    AND     #$70
    LSR     A
    LSR     A
    LSR     A
    LSR     A
    TAY
    LDA     OBJECTS_X,x                 ; its first column on the screen
    CMP     HORZ_SCROLL
    LDA     OBJECTS_XHI,x
    SBC     HORZ_SCROLL_HI
    BNE     @off
    LDA     OBJECTS_X,x                 ; and its last one
    CLC
    ADC     OBJECT_WIDTHS,y
    STA     WV_T0
    LDA     OBJECTS_XHI,x
    ADC     #$00
    STA     WV_T1
    LDA     WV_T0
    CMP     HORZ_SCROLL
    LDA     WV_T1
    SBC     HORZ_SCROLL_HI
    BNE     @off
    PLA
    TAY
    LDA     #$00
    RTS
@off:
    PLA
    TAY
    LDA     #$01
wide_original_done:
    RTS

; ---------------------------------------------------------------------------- ;
; The sprite block an object gets this frame (Objects_HandleScrollAndUpdate, $C9BC, in place of
; LDA SprRamOffsets,Y / STA Object_SprRAM,X): the same, kept for wide_frame_end, since an object
; may move its Object_SprRAM afterwards (a troopa adds 8 and draws two sprites below it).
wide_spr_base:
    LDA     SPR_RAM_OFFSETS,y
    STA     OBJECT_SPR_RAM,x
    STA     WV_SPR_BASE,x
    RTS

; ---------------------------------------------------------------------------- ;
; Whether an object of the list is at the creation column (Level_ObjectsSpawnByScroll, $B884, in
; place of ASL A x 4 / CMP Temp_Var6; A is the object's column byte, the BNE after skips it; on
; the screen Temp_Var7). The game's pass: exactly the game's column, as the game. The pass ahead:
; anywhere past the game's column up to the column ahead (in the direction the screen moves),
; so that what an area starts with between the two is created too. A gets the object's x within
; its screen, Z set when it is to be created.
wide_spawn_test:
    ASL     A
    ASL     A
    ASL     A
    ASL     A
    STA     WV_T0
    LDA     WV_AHEAD
    BNE     @range
    LDA     WV_T0
    CMP     TEMP_VAR6
    RTS
@range:
    LDA     SCROLL_LAST_DIR
    BNE     @left
    LDA     TEMP_VAR6                   ; right: x <= the column ahead (same screen)
    CMP     WV_T0
    BCC     @no
    LDA     WV_ORIG_LO                  ; and past the game's column
    CMP     WV_T0
    LDA     WV_ORIG_HI
    SBC     TEMP_VAR7
    BCS     @no
@yes:
    LDA     WV_T0
    CMP     WV_T0
    RTS
@left:
    LDA     WV_T0                       ; left: x >= the column ahead
    CMP     TEMP_VAR6
    BCC     @no
    LDA     WV_T0                       ; and short of the game's column
    CMP     WV_ORIG_LO
    LDA     TEMP_VAR7
    SBC     WV_ORIG_HI
    BCC     @yes
@no:
    LDA     #$01
    RTS

; ---------------------------------------------------------------------------- ;
; Mario's limits on the screen (PRG008_B1CE, bank 8, in place of LDA Player_SpriteX / LDY
; Player_XVel). The game holds his sprite x within 16-232 of the screen; the camera following him
; makes it matter only at the level's ends, and where the screen scrolls by itself, where it is a
; wall at the original right edge in the middle of the wide view. Moving right, the limit is
; 232 + M instead (16-bit), never past where the game holds him at the level's end (its last
; screen's 232). The left limit stays: the game takes Mario 1 to 8 pixels left of the screen for
; crushed by the scrolling. Without margin, and in a vertical level, the game's code as it is.
wide_player_limit:
    LDA     LEVEL_VERTICAL
    BNE     @game
    LDA     WV_M16_LO
    ORA     WV_M16_HI
    BNE     @wide
@game:
    LDA     PLAYER_SPRITE_X
    LDY     PLAYER_XVEL
    JMP     PLAYER_LIMIT_TEST
@wide:
    LDY     PLAYER_XVEL
    BPL     @rightward
    LDA     PLAYER_SPRITE_X             ; moving left: the game's limit
    JMP     PLAYER_LIMIT_TEST
@rightward:
    LDA     PLAYER_X                    ; Mario's x on the screen, 16-bit
    SEC
    SBC     HORZ_SCROLL
    STA     WV_T0
    LDA     PLAYER_XHI
    SBC     HORZ_SCROLL_HI
    STA     WV_T1
    LDA     #232                        ; 232 + M
    CLC
    ADC     WV_M
    STA     WV_RIGHT_LO
    LDA     #$00
    ADC     #$00
    STA     WV_RIGHT_HI
    LDA     #232                        ; the level's last screen's 232, on this screen
    SEC
    SBC     HORZ_SCROLL
    STA     WV_END_LO
    LDA     LEVEL_WIDTH
    SBC     HORZ_SCROLL_HI
    STA     WV_END_HI
    LDA     WV_END_LO                   ; the nearer of the two
    CMP     WV_RIGHT_LO
    LDA     WV_END_HI
    SBC     WV_RIGHT_HI
    BPL     @limit
    LDA     WV_END_LO
    STA     WV_RIGHT_LO
    LDA     WV_END_HI
    STA     WV_RIGHT_HI
@limit:
    LDA     WV_T0                       ; short of it: free
    CMP     WV_RIGHT_LO
    LDA     WV_T1
    SBC     WV_RIGHT_HI
    BMI     @free
    LDA     HORZ_SCROLL                 ; held there, as the game holds him at 232
    CLC
    ADC     WV_RIGHT_LO
    STA     PLAYER_X
    LDA     HORZ_SCROLL_HI
    ADC     WV_RIGHT_HI
    STA     PLAYER_XHI
    LDA     WV_RIGHT_LO
    STA     PLAYER_SPRITE_X
    JMP     PLAYER_LIMIT_SLOW
@free:
    JMP     PLAYER_LIMIT_DONE

; ---------------------------------------------------------------------------- ;
; Mario against the edges of a screen that scrolls by itself (AScroll_MovePlayer, PRG009_BCC0,
; bank 9, in place of LDA Player_SpriteX / CMP #16 / BLT / CMP #224 / BLT): the game gives him the
; screen's speed when his sprite x is under 16 or from 224 on and he moves into that edge. The
; right edge is 224 + M (16-bit), never past the level's last screen's 224; the left one stays.
; Without margin, and in a vertical level, the game's tests.
wide_ascroll_edge:
    LDA     LEVEL_VERTICAL
    BNE     @game
    LDA     WV_M16_LO
    ORA     WV_M16_HI
    BNE     @wide
@game:
    LDA     PLAYER_SPRITE_X
    CMP     #16
    BCC     @game_pressed
    CMP     #224
    BCC     @game_free
@game_pressed:
    JMP     ASCROLL_PRESSED
@game_free:
    JMP     ASCROLL_FREE
@wide:
    LDA     PLAYER_X                    ; Mario's x on the screen, 16-bit
    SEC
    SBC     HORZ_SCROLL
    STA     WV_T0
    LDA     PLAYER_XHI
    SBC     HORZ_SCROLL_HI
    STA     WV_T1
    BMI     @left                       ; left of the screen
    BNE     @right_test                 ; past 255
    LDA     WV_T0
    CMP     #16
    BCC     @left
@right_test:
    LDA     #224                        ; 224 + M
    CLC
    ADC     WV_M
    STA     WV_RIGHT_LO
    LDA     #$00
    ADC     #$00
    STA     WV_RIGHT_HI
    LDA     #224                        ; the level's last screen's 224, on this screen
    SEC
    SBC     HORZ_SCROLL
    STA     WV_END_LO
    LDA     LEVEL_WIDTH
    SBC     HORZ_SCROLL_HI
    STA     WV_END_HI
    LDA     WV_END_LO                   ; the nearer of the two
    CMP     WV_RIGHT_LO
    LDA     WV_END_HI
    SBC     WV_RIGHT_HI
    BPL     @edge
    LDA     WV_END_LO
    STA     WV_RIGHT_LO
    LDA     WV_END_HI
    STA     WV_RIGHT_HI
@edge:
    LDA     WV_T0
    CMP     WV_RIGHT_LO
    LDA     WV_T1
    SBC     WV_RIGHT_HI
    BMI     @free
    LDA     PLAYER_XVEL                 ; right edge: held when he moves right at least as fast
    SEC                                 ; as the screen
    SBC     LEVEL_ASCRL_HVEL
    BMI     @on
    BPL     @push
@left:
    LDA     PLAYER_XVEL                 ; left edge: pushed when slower than the screen
    SEC
    SBC     LEVEL_ASCRL_HVEL
    BPL     @on
@push:
    LDA     LEVEL_ASCRL_HVEL
    STA     PLAYER_XVEL
    LDA     #$00
    STA     PLAYER_XVEL_FRAC
@on:
    JMP     ASCROLL_ON
@free:
    JMP     ASCROLL_FREE

; ---------------------------------------------------------------------------- ;
; Crushed by the scrolling (PRG008_A45A, bank 8, in place of LDA Player_SpriteX / CMP #$F8; the
; BLT after skips the death): the game takes a sprite x of $F8-$FF for Mario pushed 1 to 8 pixels
; off the left edge. With the right limit past 255, that is a 16-bit x of -8 to -1 only. C clear:
; not crushed.
wide_crushed:
    LDA     WV_M16_LO
    ORA     WV_M16_HI
    BNE     @wide
    LDA     PLAYER_SPRITE_X
    CMP     #$F8
    RTS
@wide:
    LDA     PLAYER_X
    SEC
    SBC     HORZ_SCROLL
    STA     WV_T0
    LDA     PLAYER_XHI
    SBC     HORZ_SCROLL_HI
    CMP     #$FF
    BNE     @no
    LDA     WV_T0
    CMP     #$F8
    RTS
@no:
    CLC
    RTS
```

### B. `tools/build.py`

```python
"""Builds the Super Mario Bros. 3 (USA) (Rev 1) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Super Mario Bros. 3 (USA) (Rev 1).nes> [out.bps]
       python -I tools/build.py <Super Mario Bros. 3 (USA) (Rev 1).nes> --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH): the FIXED segment
   for $E240 of PRG bank 31 (the last 8 KiB bank, fixed at $E000; free space), the WINDOW segment
   for $5800, 2 KiB the widescreen extension's PRG window shows there, taken from the free end of
   PRG bank 25 (16 KiB bank 12, from its page $36). WIDE_DEFINES (comma-separated) is passed to
   ca65 as -D symbols (LAG_TRACE: a build that counts the lag frames).
2. Writes both into the game's free space (checked to be all $FF) and the hooks into its code
   (each checked against the bytes it replaces). The ROM keeps its size and layout.
3. Writes build/smb3wide.nes and, if asked, the BPS patch (data without iNES header).

With --pack, a Zaxnes hack pack (tools/make_pack.py) instead: src/wide.s assembled with PACK, the
WINDOW segment only, for $5800 of the extension's memory, which Zaxnes shows in the PRG window
from power-on, the extension unlocked (no FIXED: the NMI hook jumps to pack_nmi); the hooks, each
with the game's bytes it replaces, checked by Zaxnes before it applies them. Nothing goes in the
game's free space and the ROM keeps its size.
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x2E6301ED
PRG_SIZE = 0x40000
CHR_SIZE = 0x20000
BANK_8K = 0x2000
FIXED_START = 0xE240                # free space of bank 31 (padding before a DMC sample)
FIXED_SIZE = 0x80
WINDOW_START = 0x5800
WINDOW_SIZE = 0x800
WINDOW_AT = 25 * BANK_8K + 0x1600   # free space at the end of bank 25 (16 KiB bank 12, page $36)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOP = 0xEA


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def offset(bank, address):
    """Where `address` of the game's 8 KiB PRG bank `bank` lies in the PRG-ROM."""
    return bank * BANK_8K + (address & (BANK_8K - 1))


def hooks(sym, pack=False):
    """The patches of the game's PRG-ROM: (bank, address, original bytes, new bytes)."""
    out = []

    def put(bank, address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((bank, address, original, new))

    # NMI: the extension unlocked and set for the frame, then the game's NMI (JMP PRG030_SUB_9F40).
    put(31, 0xF496, '4c 40 9f', jmp(sym['pack_nmi' if pack else 'fixed_nmi']))
    # An object's sprite columns: those within the margins shown too (Object_DetermineHorzVisY).
    put(0, 0xD7B8, 'a9 00 9d 51 06', jmp(sym['wide_horz_vis']))
    # The end of the frame's work: the high bytes of the objects' sprites
    # (GraphicsBuf_Prep_And_WaitVSync).
    put(30, 0x96E5, 'a5 5e 0a a8', jsr(sym['wide_frame_end']))
    # The sprite block an object gets, kept (Objects_HandleScrollAndUpdate).
    put(0, 0xC9BC, 'b9 01 c4 9d 8f 05', jsr(sym['wide_spr_base']))
    # The sprites an object's draw routines write: who drew them (Object_Draw16x16Sprite,
    # Object_Draw24x16Sprite).
    put(0, 0xD680, 'a5 04 4a', jsr(sym['wide_own2']))
    put(0, 0xD6C7, 'a5 04 4a', jsr(sym['wide_own3']))
    # Objects created as far ahead as the margin (Level_ObjectsSpawnByScroll)...
    put(5, 0xB838, 'a5 fd 18 79 32 b8 29 f0 85 05 a5 12 79 34 b8',
        jsr(sym['wide_spawn_ahead']) + jmp(0xB847))
    # ... an object between the game's column and the column ahead too ...
    put(5, 0xB884, '0a 0a 0a 0a c5 05', jsr(sym['wide_spawn_test']))
    # ... the game's own column too, after (PRG005_B82E, JMP Level_ObjectsSpawnByScroll) ...
    put(5, 0xB82E, '4c 36 b8', jmp(sym['wide_spawn_twice']))
    # ... and deleted that much further (Object_DeleteOffScreen).
    put(0, 0xD40F, 'a5 fd 18 79 cc d3', jsr(sym['wide_delete_low']))
    put(0, 0xD41E, '79 d2 d3', jsr(sym['wide_delete_high']))
    # The special objects an object creates: only from the original screen.
    put(0, 0xC454, '20 54 dd', jsr(sym['wide_original_offscreen']))
    put(3, 0xB0C1, '20 54 dd', jsr(sym['wide_original_offscreen']))
    put(4, 0xA5E6, 'bd 51 06', jsr(sym['wide_original_hoffscreen']))
    # Mario's right limit on the screen: the wide view's (PRG008_B1CE).
    put(8, 0xB1CE, 'a5 ab a4 bd', jmp(sym['wide_player_limit']))
    # ... and where the screen scrolls by itself (AScroll_MovePlayer); crushed only off the left.
    put(9, 0xBCC0, 'a5 ab c9 10 90 04 c9 e0 90 16', jmp(sym['wide_ascroll_edge']))
    put(8, 0xA45A, 'a5 ab c9 f8', jsr(sym['wide_crushed']))
    # Options: the level select on the title screen (Title_Do1P2PMenu).
    put(24, 0xAC35, 'a5 f8 29 c0 c9 c0 d0 05 ea ea ea ea ea', jsr(sym['wide_title']) + jmp(0xAC42))
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
    fixed_bin = os.path.join(build, 'fixed.bin')
    window_bin = os.path.join(build, 'window.bin')
    fixed_memory = ('' if pack else
                    f'  FIXED: start = ${FIXED_START:04X}, size = ${FIXED_SIZE:04X}, fill = yes, '
                    f'fillval = $FF, file = "{wsl_path(fixed_bin)}";\n')
    open(cfg, 'w', newline='\n').write(
        'MEMORY {\n' + fixed_memory +
        f'  WINDOW: start = ${WINDOW_START:04X}, size = ${WINDOW_SIZE:04X}, fill = yes, fillval = $FF, '
        f'file = "{wsl_path(window_bin)}";\n'
        '}\n'
        'SEGMENTS {\n' + ('' if pack else '  FIXED: load = FIXED, type = ro;\n') +
        '  WINDOW: load = WINDOW, type = ro;\n'
        '}\n')
    obj = os.path.join(build, 'wide.o')
    labels = os.path.join(build, 'wide.labels')
    defines = ['-D', 'PACK'] if pack else []
    for name in os.environ.get('WIDE_DEFINES', '').split(','):
        if name:
            defines += ['-D', name]
    subprocess.run(tool('ca65') + ['-g'] + defines +
                   [wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)], check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-Ln', wsl_path(labels), wsl_path(obj)],
                   check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return None if pack else open(fixed_bin, 'rb').read(), open(window_bin, 'rb').read(), sym


def pack(data, out):
    _, window, sym = assemble(pack=True)
    assert len(window) == WINDOW_SIZE
    sys.path.insert(0, os.path.join(ROOT, 'tools'))
    import make_pack
    widescreen = []
    for bank, address, original, new in hooks(sym, pack=True):
        at = offset(bank, address)
        assert data[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        widescreen.append((at, bytes(original), bytes(new)))
    # Zaxnes reads $FF past the code's end: its trailing fill is left out.
    used = (len(window.rstrip(b'\xff')) + 0xFF) & ~0xFF
    result = make_pack.make('Super Mario Bros. 3 (USA) (Rev 1) widescreen', window[:used],
                            [('widescreen', widescreen)])
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes ({used} of code, {len(widescreen)} hooks), '
          f'pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Super Mario Bros. 3 (USA) (Rev 1)'

    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        pack(bytes(data), sys.argv[3])
        return
    fixed, window, sym = assemble()
    assert len(fixed) == FIXED_SIZE and len(window) == WINDOW_SIZE
    prg = bytearray(data[:PRG_SIZE])
    for at, code, what in [(offset(31, FIXED_START), fixed, 'bank 31'), (WINDOW_AT, window, 'bank 25')]:
        assert prg[at:at + len(code)] == bytes([0xFF]) * len(code), f"{what}: the space is not free"
        prg[at:at + len(code)] = code

    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for bank, address, original, new in hooks(sym):
        if address in skip:
            continue
        at = offset(bank, address)
        assert prg[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        prg[at:at + len(new)] = new
    target = bytes(prg) + bytes(data[PRG_SIZE:])

    out = os.path.join(ROOT, 'build', 'smb3wide.nes')
    open(out, 'wb').write(bytes(image[:header]) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        patch = make_bps.make(bytes(data), target)
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
