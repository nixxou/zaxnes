# Metal Force (Korea): the widescreen hack

This document describes completely the modification of *Metal Force (Korea)* that Zaxnes ships as a
built-in patch, the advanced mode of its widescreen. The appendices hold the new code and the build
script, which writes every hook after checking the bytes it replaces: with them the ROM can be
rebuilt identically.

The background of the margins is not the hack's work: the emulator's Metal Force margin decoder
reads the level map the game keeps in PRG-RAM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4); it widens the game on
its own, the simple mode. The hack does the objects: enemies that spawn, sleep, live and show in
the margins, the way the Mega Man 2 hack does them (owner request: the game stays the original's
on the original screen).

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Metal Force (Korea) (Unl)*, mapper 156 (Daou Infosys DIS23C01), 256 KiB PRG (16 banks of 16 KiB, bank 15 fixed at `$C000`), 256 KiB CHR-ROM, 8 KiB PRG-RAM (the level map) |
| Source data (iNES header excluded) | CRC-32 `EC6D910E` |
| Target data | CRC-32 `E0FA0F0A` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | new code at `$F940` of bank 15 (in the `$FF` fill `$F936`-`$FF7F`), hooks in bank 15 and bank 13 |
| Patch | `rust/crates/zax-nes/patches/metalforce-korea-widescreen.bps`, made against the data without header |

---

## 2. What the game does, and what changes

The whole game runs in the NMI (`$EA78`): PPU updates, sprite DMA, sound (`$EB40`), joypad
(`$C649`), then the game state machine (`$EB0E`, state `$034E`, step `$034F`: 1 title, 4 mission
screen, 5 play). The reset ends in `JMP $FFC3` to itself. A frame whose work does not fit runs into
the next NMI, which is then missed (the NMI is turned back on at the end of the handler): a lag
frame.

Enemies: six slots of 16 bytes at `$03A7 + Y` (`Y = 16 k`): `$03A7` state (0 free), `$03A8` the
level list entry it spawned from (or a type), `$03AB`/`$03AC` world x, `$03AD`/`$03AE` world y. The
level list (6 bytes per entry from `$0469`: `$0469` spawned, `$046B` type, `$046C` 16-pixel column,
`$046D` row), sorted by column, is read as the camera scrolls (bank 13 `$8010`). Camera: `$35:$34`
world x of the screen's left edge, `$3C` its 16-pixel column, `$3D` the 16-pixel row of its bottom
edge. Shots: the player's at `$0373 + Y`, the enemies' at `$056B + Y`.

| Game routine | Original | Hack |
|---|---|---|
| spawn window (`$C010`) | columns `$3C - 2` to `$3C + 18`: an enemy spawns 32 pixels past either edge | columns widened by `K = ceil(M / 16)` on each side (at most 11) |
| spawn from the list (`$C0B8`) | | the enemy sleeps (`WV_DORMANT`, `WV_DORMID` = its list entry) |
| enemy loop (bank 13 `$820A`, its call of the collisions `$A0EA` at `$822A`) | collisions, then behaviour (`$8245`) | a sleeping enemy outside `[-48, 304)` of the screen (the original spawn window, whatever the camera's fine x) skips both, as an enemy that is gone does; inside, it wakes for good, margins included |
| enemy off the screen? (bank 13 `$A3BC`) | removed 80 pixels past either edge | awake: `max(80, M + 16)` past either edge (the game has six slots; the enemies to come need them); asleep: `16 K + 64` (it spawned past the margin) |
| sprites of a metasprite (`$CA3D`, `$CACA`, `$CB57`, `$CBFA`, the four flips) | a sprite whose screen x high byte `$22` is not 0 is skipped | shown within `[-M, 256 + M)` |
| OAM write (`$CC3C`) | | the sprite's high byte `$22` to `$5100 + OAM offset` |
| damage (`$E76A`, `$E79E`, `$E7D5`, `$E80F`, `$E849`: 2, 3, 4, 4 and 8 points from the health `$031C`) | nothing while the blinking `$032B` runs | nor when the option "Invincible" is on |
| mission screen (state 4, step 2) | shows the mission `$0370` | with the option "Level select", right / left pick the next / previous mission (1 to 7) and start the screen over (step 0) |

The shots keep the original screen (owner request): the player's are removed at its edges
(`$D97D`), the enemies' as soon as they leave it (bank 13 `$ADE8`); an enemy awake in a margin
fires shots that vanish at once, as off the original screen. The game's collisions use world
coordinates; a sleeping enemy touches nothing.

Margin: `M = min($5002, 248)`. With `M = 0` every routine does exactly what the code it replaces
does: `K = 0`, both removal distances 80, every enemy spawns inside `[-48, 304)` and wakes at once.

CPU time: 64 lines of overclock (`$5006`) pay for the hack's work and the enemies of the margins;
255 with the option "No slowdown". Measured over 1800 frames of play (missions 1, 3 and 6, holding
right and fire, 88-pixel margin): the original misses no NMI while playing, only while it unpacks a
level (about ten frames at each start, the NMI handler running long); with the hack and 32 lines,
mission 3 lagged 6 frames in play, hence 64; with "No slowdown" the level unpacking itself shrinks
to about five frames.

## 3. The extension

Unlocked at reset (`$EBC8`, `JMP $FF80` becomes `JMP wide_reset`, which unlocks then jumps on) and
checked every NMI (`$EAE3`, once the PPU is set and before the game's frame, `JSR $EB40` becomes
`JSR wide_nmi`, which unlocks again if needed, then writes `$5003 = M`, `$5004 = $5005 = 0` (the
nametable columns are hidden: the decoder draws the background), `$5007 = $02` (sprites free of
those extents), `$5006 = 64` (255 with "No slowdown"), works out `K` and the removal distances, runs
the level select, and jumps to the sound).

Variables (extension RAM): `WV_M $5200`, `WV_K $5201`, `WV_THR $5202` / `WV_THRL $5203` (asleep:
`16 K + 64` and `256 -` that), `WV_AWAKE $5204` / `WV_AWAKEL $5205` (awake: `max(80, M + 16)` and
`256 -` that), `WV_DORMANT $5240 + Y`, `WV_DORMID $52A0 + Y`. A slot reused by anything but the
list entry it slept for (`$03A8` differs) is awake.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$EBC8` | `JMP $FF80` | `JMP wide_reset` |
| `$EAE3` | `JSR $EB40` | `JSR wide_nmi` |
| `$C010` | `LDA $3C / CMP #$02` (head of the spawn window) | `JMP wide_spawn_window` |
| `$C0B8` | `LDA #$01 / STA $0469,X` | `JSR wide_spawned / NOP / NOP` |
| bank 13 `$822A` | `JSR $A0EA` | `JSR wide_wake` |
| bank 13 `$A3BC` | `LDA $20 / BEQ $A3D4` | `JMP wide_despawn / NOP` |
| `$CA3D`, `$CACA`, `$CB57`, `$CBFA` | `LDA $22 / BNE +12 / LDA ($1D),Y` | `JSR wide_sprite_visible / BEQ` (the same target) `/ NOP` |
| `$CC3C` | `LDA $0300` | `JSR wide_sprite_high` |
| `$E76F`, `$E7A3`, `$E7DA`, `$E814`, `$E84E` | `LDA $032B` | `JSR wide_hurt` |

`$A3EE` (bank 13) is the game's own removal of an enemy (it also lets its list entry spawn again);
`$A3D4` the vertical half of its off-screen test.

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there (`WIDE_OPTIONS`).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `level_select` | Level select (left / right on the mission screen) / Sélecteur de niveau (gauche / droite sur l'écran de mission) | off | a testing aid: on the mission screen, right / left pick the mission |
| 1 | `invincible` | Invincible | off | a testing aid: no damage (`wide_hurt`); falling into a pit still kills |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of 64 |

The options live in Zaxnes (`rust/crates/zax-nes/src/builtin_patches.rs`, `PATCHES`); the ROM does
not depend on them being set.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Metal Force [Korea][Unl][!].nes" build/metalforcewide.bps
```

assembles `src/wide.s` for `$F940`-`$FF7F` (`-g`, labels from `ld65 -Ln`), checks the code space is
still `$FF`, writes the code and the hooks, `build/metalforcewide.nes` and the BPS patch
(`tools/make_bps.py`, SourceRead / TargetRead only). `WIDE_SKIP=<hex addresses>` leaves hooks out,
for testing.

## 7. Limits

* No sprite in the vertical margins: the extension carries a high byte for x only.
* The game has six enemy slots; enemies alive in the margins hold them a little longer than the
  original (until `M + 16` past the edge), which can delay an enemy of the list behind them.
* The sprites of the margins share the console's 64 sprites with the screen's: more flicker when
  many enemies show.

---

## Appendices

### A. `src/wide.s`

```asm
; Metal Force (Korea) widescreen hack: the code the patched fixed bank calls.
;
; Assembled for $F940 of the fixed bank (PRG bank 15), in the unused $FF fill from $F936 to $FF7F.
; Talks to the Zaxnes widescreen extension (rust/docs/WIDESCREEN-EXTENSION.md); the background of
; the margins comes from the emulator's Metal Force margin decoder, the hack does the objects, the
; way the Mega Man 2 hack does them:
;
;  * enemies of the level list spawn when they come within the margin of the original spawn
;    window (two 16-pixel columns past each edge of the screen), and live until they leave the
;    margin;
;  * an enemy that spawns past the original window shows but sleeps (no behaviour, no collision)
;    until it comes within that window; awake, it stays awake;
;  * every sprite of an object gets the high byte of its 16-bit x, so it shows in the margins;
;  * shots, the player's and the enemies', keep the limits of the original screen.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

WIDE_SIG        = $5000         ; W: unlock "WIDE"; R: $57 once unlocked
WIDE_HOST       = $5002         ; R: margin the host offers on each side
WIDE_REQ        = $5003         ; W: margin asked for
WIDE_LEFT       = $5004         ; W: background extents (the decoder draws the margins)
WIDE_RIGHT      = $5005
WIDE_OVERCLOCK  = $5006         ; W: extra CPU time each frame, in scanlines
WIDE_LAYOUT     = $5007         ; W: bit 1, sprites free of the background extents
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; W: high byte of the x of sprite n at $5100 + 4n

OPT_LEVEL_SELECT = $01          ; a testing aid: left / right on the mission screen pick the mission
OPT_INVINCIBLE  = $02           ; a testing aid: the player takes no damage
OPT_NO_SLOWDOWN = $04           ; nearly a frame more of CPU time: the game never runs late

MAX_MARGIN      = 248           ; the widest margin the extension's 8-bit registers carry
MAX_COLUMNS     = 11            ; spawn columns added on each side at most (16 x 11 + 64 < 256)
OVERCLOCK_LINES = 64            ; pays for the hack's work and the enemies of the margins
NO_SLOWDOWN_LINES = 255
WAKE_LEFT       = $100 - 48     ; an enemy wakes up within [-48, 304) of the screen: the original
WAKE_RIGHT      = 48            ; spawn window, whatever the camera's fine x
MISSIONS        = 7

; Variables in the extension's RAM.
WV_M            = $5200         ; margin of this frame
WV_K            = $5201         ; spawn columns added on each side: ceil(M / 16), at most 11
WV_THR          = $5202         ; sleeping enemies are removed this far past the screen's right
WV_THRL         = $5203         ; edge, and at 256 - this past its left edge (x high byte $FF)
WV_AWAKE        = $5204         ; the same for the enemies awake: max(80, M + 16)...
WV_AWAKEL       = $5205         ; ... and 256 - that
WV_DORMANT      = $5240         ; + enemy slot offset ($00-$50): non-zero while asleep
WV_DORMID       = $52A0         ; + slot: the list entry ($03A8) it slept for

; Game RAM (original meanings).
CAM_COLUMN      = $3C           ; camera, 16-pixel column of the screen's left edge
CAM_ROW         = $3D           ; camera, 16-pixel row of the screen's bottom edge
SCREEN_XL       = $1F           ; object at hand: screen x ($AB92), signed 16-bit
SCREEN_XH       = $20
SPRITE_XL       = $21           ; sprite at hand: screen x, signed 16-bit
SPRITE_XH       = $22
SPRITE_DATA     = $1D           ; the metasprite being drawn
OBJ_LIST        = $03A8         ; + slot: the level list entry it spawned from (or its type)
LIST_SPAWNED    = $0469         ; + list entry: spawned
HIT_TIMER       = $032B         ; the player's blinking after a hit
GAME_STATE      = $034E
GAME_STEP       = $034F
MISSION         = $0370
AREA            = $0372
PRESSED         = $84           ; buttons pressed this frame: A B Select Start Up Down Left Right
OAM_DIRECTION   = $0300

ORIGINAL_RESET  = $FF80
SOUND           = $EB40
ENEMY_HIT       = $A0EA         ; bank 13: an enemy's collisions, carry set when it is gone
ENEMY_REMOVE    = $A3EE         ; bank 13: removes the enemy at slot Y, SEC / RTS
ENEMY_VERTICAL  = $A3D4         ; bank 13: the vertical half of the off-screen test

.segment "CODE"
.org $F940

; ---------------------------------------------------------------------------- ;
; Reset ($EBC8, in place of JMP $FF80).
wide_reset:
    JSR      wide_unlock
    JMP      ORIGINAL_RESET

wide_unlock:
    LDA      #$57
    STA      WIDE_SIG
    LDA      #$49
    STA      WIDE_SIG
    LDA      #$44
    STA      WIDE_SIG
    LDA      #$45
    STA      WIDE_SIG
    LDA      #$00
    STA      WV_M
    STA      WV_K
    LDX      #$5F
@clear:
    STA      WV_DORMANT,x
    DEX
    BPL      @clear
    RTS

; ---------------------------------------------------------------------------- ;
; NMI ($EAE3, in place of JSR $EB40, once the PPU is set for the frame and before the game's
; frame): the extension's settings for the next frame (unlocked again first if a state made
; without the hack left it locked).
wide_nmi:
    LDA      WIDE_SIG
    CMP      #$57
    BEQ      @set
    JSR      wide_unlock
@set:
    LDA      WIDE_HOST
    CMP      #MAX_MARGIN
    BCC      @margin
    LDA      #MAX_MARGIN
@margin:
    STA      WV_M
    STA      WIDE_REQ
    LSR                                 ; K = ceil(M / 16), at most MAX_COLUMNS
    LSR
    LSR
    LSR
    TAX
    LDA      WV_M
    AND      #$0F
    BEQ      @whole
    INX
@whole:
    CPX      #MAX_COLUMNS + 1
    BCC      @columns
    LDX      #MAX_COLUMNS
@columns:
    STX      WV_K
    TXA                                 ; enemies live within 16 K + 64 of the screen, at
    ASL                                 ; least 80 as in the original
    ASL
    ASL
    ASL
    CLC
    ADC      #64
    CMP      #80
    BCS      @threshold
    LDA      #80
@threshold:
    STA      WV_THR
    EOR      #$FF
    CLC
    ADC      #$01
    STA      WV_THRL
    LDA      WV_M                       ; awake: just past the margin seen, at least 80
    CLC
    ADC      #16
    BCC      @awake_max
    LDA      #$FF
@awake_max:
    CMP      #80
    BCS      @awake
    LDA      #80
@awake:
    STA      WV_AWAKE
    EOR      #$FF
    CLC
    ADC      #$01
    STA      WV_AWAKEL
    LDA      #$00
    STA      WIDE_LEFT
    STA      WIDE_RIGHT
    LDA      #$02                       ; sprites free of the extents
    STA      WIDE_LAYOUT
    LDA      WIDE_OPTIONS
    AND      #OPT_NO_SLOWDOWN
    BEQ      @normal
    LDA      #NO_SLOWDOWN_LINES
    BNE      @overclock
@normal:
    LDA      #OVERCLOCK_LINES
@overclock:
    STA      WIDE_OVERCLOCK
    JSR      wide_level_select
    JMP      SOUND

; ---------------------------------------------------------------------------- ;
; Spawn window ($C010, the whole routine): $00/$01 the first and last 16-pixel columns, $02/$03
; the first and last rows the level list spawns in. The columns widen by K on each side.
wide_spawn_window:
    LDA      CAM_COLUMN
    CMP      #$02
    BCC      @left
    SBC      #$02
@left:
    SEC
    SBC      WV_K
    BCS      @first
    LDA      #$00
@first:
    STA      $00
    LDA      CAM_COLUMN
    CLC
    ADC      #$12
    CLC
    ADC      WV_K
    BCC      @last
    LDA      #$FF
@last:
    STA      $01
    LDA      CAM_ROW
    CMP      #$01
    BCC      @top
    SBC      #$01
@top:
    STA      $02
    LDA      CAM_ROW
    CLC
    ADC      #$11
    STA      $03
    RTS

; ---------------------------------------------------------------------------- ;
; Dormant enemies.

; Spawn from the level list ($C0B8, in place of LDA #$01 / STA $0469,X; X the list entry, Y the
; enemy slot): the enemy sleeps until wide_wake finds it within the original window.
wide_spawned:
    LDA      #$01
    STA      LIST_SPAWNED,x
    STA      WV_DORMANT,y
    TXA
    STA      WV_DORMID,y
    RTS

; The enemy loop (bank 13, $822A, in place of JSR $A0EA; $20:$1F the enemy's screen x, Y its
; slot): a sleeping enemy outside [-48, 304) skips its collisions and its behaviour (carry set,
; as for an enemy that is gone); one inside wakes up for good. A slot reused by anything else
; than the entry it slept for is awake.
wide_wake:
    LDA      WV_DORMANT,y
    BEQ      @awake
    LDA      WV_DORMID,y
    CMP      OBJ_LIST,y
    BNE      @wake
    LDA      SCREEN_XH
    BEQ      @wake
    BMI      @left
    CMP      #$01
    BNE      @sleep
    LDA      SCREEN_XL
    CMP      #WAKE_RIGHT
    BCC      @wake
    BCS      @sleep
@left:
    CMP      #$FF
    BNE      @sleep
    LDA      SCREEN_XL
    CMP      #WAKE_LEFT
    BCS      @wake
@sleep:
    SEC
    RTS
@wake:
    LDA      #$00
    STA      WV_DORMANT,y
@awake:
    JMP      ENEMY_HIT

; Enemy off the screen? (bank 13, $A3BC, in place of LDA $20 / BEQ $A3D4; Y its slot): the
; horizontal half. An enemy awake is removed just past the margin seen (80 past either edge in
; the original, the game's six slots kept for the enemies to come); a sleeping one, spawned past
; it, at WV_THR.
wide_despawn:
    LDA      SCREEN_XH
    BEQ      @vertical
    JSR      wide_asleep
    BCS      @sleeping
    LDA      SCREEN_XH
    BMI      @left_awake
    LDA      SCREEN_XL
    CMP      WV_AWAKE
    BCC      @vertical
    JMP      ENEMY_REMOVE
@left_awake:
    LDA      SCREEN_XL
    CMP      WV_AWAKEL
    BCS      @vertical
    JMP      ENEMY_REMOVE
@sleeping:
    LDA      SCREEN_XH
    BMI      @left
    LDA      SCREEN_XL
    CMP      WV_THR
    BCC      @vertical
    JMP      ENEMY_REMOVE
@left:
    LDA      SCREEN_XL
    CMP      WV_THRL
    BCS      @vertical
    JMP      ENEMY_REMOVE
@vertical:
    JMP      ENEMY_VERTICAL

; Carry set when the enemy at slot Y sleeps (A changes).
wide_asleep:
    LDA      WV_DORMANT,y
    BEQ      @no
    LDA      WV_DORMID,y
    CMP      OBJ_LIST,y
    BNE      @no
    SEC
    RTS
@no:
    CLC
    RTS

; ---------------------------------------------------------------------------- ;
; Sprites.

; One sprite of a metasprite ($CA3D, $CACA, $CB57, $CBFA, in place of LDA $22 / BNE skip /
; LDA ($1D),Y): Z set when it is not drawn (outside [-M, 256 + M), or tile 0), otherwise A the
; tile.
wide_sprite_visible:
    LDA      SPRITE_XH
    BEQ      @shown
    CMP      #$FF
    BEQ      @left
    CMP      #$01
    BNE      @hidden
    LDA      SPRITE_XL
    CMP      WV_M
    BCC      @shown
@hidden:
    LDA      #$00
    RTS
@left:
    LDA      SPRITE_XL
    CLC
    ADC      WV_M
    BCC      @hidden
@shown:
    LDA      (SPRITE_DATA),y
    RTS

; OAM write ($CC3C, in place of LDA $0300; X the sprite's offset, forward or backward): the
; sprite's high byte for the extension.
wide_sprite_high:
    LDA      SPRITE_XH
    STA      WIDE_SPRHI,x
    LDA      OAM_DIRECTION
    RTS

; ---------------------------------------------------------------------------- ;
; Invincibility (option bit 1): the five damage routines ($E76F, $E7A3, $E7DA, $E814, $E84E, in
; place of LDA $032B, the blinking that makes a hit do nothing): never hurt.
wide_hurt:
    LDA      WIDE_OPTIONS
    AND      #OPT_INVINCIBLE
    BNE      @rts
    LDA      HIT_TIMER
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; Level select (option bit 0): on the mission screen (state 4, step 2), right and left pick the
; next or previous mission, 1 to 7; the screen starts over with it.
wide_level_select:
    LDA      WIDE_OPTIONS
    AND      #OPT_LEVEL_SELECT
    BEQ      @rts
    LDA      GAME_STATE
    CMP      #$04
    BNE      @rts
    LDA      GAME_STEP
    CMP      #$02
    BNE      @rts
    LDA      PRESSED
    AND      #$03
    BEQ      @rts
    LSR
    BCC      @previous
    LDX      MISSION
    INX
    CPX      #MISSIONS + 1
    BCC      @set
    LDX      #$01
    BNE      @set
@previous:
    LDX      MISSION
    DEX
    BNE      @set
    LDX      #MISSIONS
@set:
    STX      MISSION
    LDA      #$01
    STA      AREA
    LDA      #$00
    STA      GAME_STEP
@rts:
    RTS
```

### B. `tools/build.py`

```python
"""Builds the Metal Force (Korea) widescreen hack and its BPS patch.

Usage: python -I tools/build.py <Metal Force [Korea][Unl][!].nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $F940 of the
   fixed bank, into the unused $FF fill from $F936 to $FF7F.
2. Writes the hooks into the fixed bank and bank 13, each checked against the bytes it replaces.
3. Writes build/metalforcewide.nes and, if asked, the BPS patch (data without iNES header).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0xEC6D910E
BANK = 15
BANK_SIZE = 0x4000
FIXED = 0xC000
CODE_START = 0xF940
CODE_END = 0xFF80

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    offset = target - (at + 2)
    assert -128 <= offset <= 127, (hex(at), hex(target))
    return bytes([opcode, offset & 0xFF])


BEQ = 0xF0
NOP = 0xEA


def hooks(sym):
    """The patches: (address, original bytes, new bytes) of the fixed bank, and
    (bank, address, original bytes, new bytes) of a switchable bank ($8000-$BFFF)."""
    out = []
    banked = []

    def put(address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((address, original, new))

    def put_banked(number, address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        banked.append((number, address, original, new))

    # Reset: unlock the extension first.
    put(0xEBC8, '4c 80 ff', jmp(sym['wide_reset']))
    # NMI: the extension's settings for the next frame, in place of the call of the sound.
    put(0xEAE3, '20 40 eb', jsr(sym['wide_nmi']))
    # Spawn window of the level list.
    put(0xC010, 'a5 3c c9 02', jmp(sym['wide_spawn_window']))
    # Spawn from the level list: the enemy sleeps.
    put(0xC0B8, 'a9 01 9d 69 04', jsr(sym['wide_spawned']))
    # The enemy loop: a sleeping enemy does nothing; the off-screen test (bank 13).
    put_banked(13, 0x822A, '20 ea a0', jsr(sym['wide_wake']))
    put_banked(13, 0xA3BC, 'a5 20 f0 14', jmp(sym['wide_despawn']))
    # Metasprites, four flips: each sprite's x test, then the OAM write's high byte.
    for at in (0xCA3D, 0xCACA, 0xCB57, 0xCBFA):
        put(at, 'a5 22 d0 0c b1 1d', jsr(sym['wide_sprite_visible']) + branch(BEQ, at + 3, at + 0x10))
    put(0xCC3C, 'ad 00 03', jsr(sym['wide_sprite_high']))
    # Invincibility: the five damage routines.
    for at in (0xE76F, 0xE7A3, 0xE7DA, 0xE814, 0xE84E):
        put(at, 'ad 2b 03', jsr(sym['wide_hurt']))
    return out, banked


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
        f'MEMORY {{ CODE: start = ${CODE_START:04X}, size = ${CODE_END - CODE_START:04X}, '
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
    data = image[header:]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Metal Force (Korea)'

    code, sym = assemble()
    bank = BANK * BANK_SIZE
    start = bank + CODE_START - FIXED
    assert all(b == 0xFF for b in data[start:start + len(code)]), 'the code space is not free'
    data[start:start + len(code)] = code

    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    fixed, banked = hooks(sym)
    for number, address, original, new in banked:
        if address in skip:
            continue
        at = number * BANK_SIZE + address - 0x8000
        assert data[at:at + len(original)] == original, f'bank {number} ${address:04X}: not the expected bytes'
        data[at:at + len(new)] = new
    for address, original, new in fixed:
        if address in skip:
            continue
        at = bank + address - FIXED
        assert data[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        data[at:at + len(new)] = new

    out = os.path.join(ROOT, 'build', 'metalforcewide.nes')
    open(out, 'wb').write(bytes(image[:header]) + bytes(data))
    print(f'{out}: data CRC {zlib.crc32(data):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        open(sys.argv[2], 'wb').write(make_bps.make(bytes(image[header:]), bytes(data)))
        print(f'{sys.argv[2]}: written')


if __name__ == '__main__':
    main()
```

### C. `tools/make_bps.py`

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
