# Ninja Gaiden III (USA): the widescreen hack

This document describes completely the modification of *Ninja Gaiden III - The Ancient Ship of
Doom (USA)* that Zaxnes ships as a built-in patch. The appendices hold the new code and the build
script, which writes every hook after checking the bytes it replaces: with them the ROM can be
rebuilt identically.

The background of the margins is not the hack's work: the emulator's Ninja Gaiden III margin
decoder reads the areas' layout in PRG-ROM (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4), and widens the
unpatched game on its own. The hack is the game's **advanced mode**: the objects, which live and
show in the margins.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Ninja Gaiden III - The Ancient Ship of Doom (USA)*, MMC3, 128 KiB PRG (16 banks of 8 KiB, banks 14 and 15 fixed at `$C000`-`$FFFF`), 128 KiB CHR-ROM |
| Source data (iNES header excluded) | CRC-32 `902E3168` (the whole file with its header: CRC-32 `BABD39FA`) |
| Target data | CRC-32 `6EB84256` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | new code at `$E700` of bank 15 (in the `$FF` fill `$E6F2`-`$F001`), hooks in banks 14, 15, 4 and 5 |
| Patch | `rust/crates/zax-nes/patches/ng3-usa-widescreen.bps`, made against the data without header |

---

**Hack pack.** Zaxnes ships this hack as hack packs (`tools/build.py <rom> --pack <out.zxpk>`,
with `ZAXNES_TOOLS` set to Zaxnes' `rust/tools`; `docs/HACK-PACKS.md`, `WIDESCREEN-EXTENSION.md`
7.1), one per build whose code moved, both from this source: `ng3-usa-widescreen.zxpk` and
`ng3-japan-widescreen.zxpk` for Ninja Ryuuken Den III (Japan) (data CRC-32 `2F2E30F7`; `-D JAPAN`:
the same code and RAM, its fixed banks' code from `$C7A2` on 9 bytes later and from `$CC00` on 54
earlier, bank 4's from `$8ABC` on 8 to 16 earlier, bank 5's collisions 16 earlier; the same area
tables, which the margin decoder reads). The packs are format 2, written by Zaxnes' shared builder
(`hackpack.py`): NTSC, MMC3, each hook a 16 KiB bank and a CPU address with the game's bytes it
replaces, checked and applied by Zaxnes; the ROM keeps its size (nothing written in its free
space). `src/wide.s` is assembled with `-D PACK` for `$2000`: the zone fantôme, which Zaxnes shows
to instruction fetches only from power-on, the extension unlocked (no reset hook, no unlock in the
NMI); its registers and RAM are at `$3000`-`$3FFF` (widescreen MMC5: `$5000`-`$5FFF` stays the
cartridge's), no window. The code reads no data of its own (the game's data reads at
`$2000`-`$3FFF` reach the PPU). The widescreen is in two groups: "core" (Base: the objects' life,
dormancy, collisions and sprites in the margins, the settings at the NMI's end) and "enemies"
(Enemies in the margins, needing it: the area's enemy list), with checks of the game's code the
hack relies on: the behaviour and the background collision the dormant objects skip, where the
sprite builder goes on after an object's origin and where it skips a sprite; where the enemy list
goes on for an entry that does not appear and for one that does. Over the 45 ROMs of the game in a
GoodNES-style set (the USA and Japanese dumps, the Wii and Wii U Virtual Console ones, the
PlayChoice-10, translations and ROM hacks), 42 take a whole pack (the three others pirate versions
of the Japanese one); 3000 frames of each with scripted inputs play as without the pack. Against
the BPS build, the same options, 3000 frames: the same RAM but for the scratch bytes `$30`-`$32`.
The game has no testing aid. The BPS patch described here is kept to compare.

## 2. What the game does, and what changes

The main loop (`$CD1C`, fixed bank) waits for the NMI (`$9A87`, bank 4 at `$8000`: `BIT $15 /
BPL`), reads the pad, scrolls, places the enemies of the area's list (`$826F`), runs the 24 object
slots from 23 down to 0 (`$CD8C`: animation `$802C`, movement `$9873`, background `$8ABC`,
behaviour `$CDFD`), the collisions (bank 5 at `$A000`), then builds the sprites (`$819A`).

Object slots: 0 Ryu, 1-2 his attacks, 3 the HUD's, 4-7 the objects of the area's enemy list
(enemies, spawners, lanterns), 8-10 and 11-15 other objects (11-15 keep a 16-bit x of their own,
`$03DE:$052E`), 16-23 the moving enemies and the shots. Per slot: `$04E6` flags (bit 7 in use),
`$04CE` attributes, `$0516` **screen x** (8 bits), `$04FE` its fraction, `$058E` screen y,
`$0546`/`$055E` speed. The camera is `$DC:$DB`; `$49` is the scroll of the frame (old camera x -
new), `$D1` the camera x / 16.

The game never holds an object's place in the area: its x is a screen x, moved by its speed and by
the scroll (`$99DA`, `$99FA`), and an add that carries past 0 or 255 removes it (`$9A45`). The
hack follows each object's 16-bit screen x in the extension's RAM (`wide_sync`): the signed change
of its 8-bit x since it was last seen, a change larger than 32 pixels being a new place on the
screen (high byte 0), and a slot that was free (the object loop's test and store, `$CD90`,
`$CDB8`) or just made by the list a new object.

| Game routine | Original | Hack |
|---|---|---|
| sideways move (bank 4 `$99DA`, `$99FA`, ending `PHP / JMP $9A45`) | removed when the 8-bit add carries | removed when the 16-bit x leaves `[LO, HI)` |
| enemy list, right edge (bank 4 `$827E`) | entry Y appears when its column is `$D1 + $86` (the one past the right edge), at x `$88` (`$FF`) | up to K = M / 16 columns earlier, at x `$88` + 16 x the columns early (in the right margin); once its slot (`(Y & 3) \| 4`) holds no object on the original screen |
| enemy list, left edge (bank 4 `$8293`) | when its column is `$D1 + 1`, at x `$87` (0) | the same to the left |
| object loop, behaviour (`$CDB3`) and background (`$CDB0`) | for slots 4-7 and 16-23 | skipped while the object is outside the original screen (dormant: drawn, moved by its speed and the scroll) |
| collisions (bank 5 `$A050`, `$A12C`, `$A1DB`, `$A234`, `$A2CA`: `LDA $0516,X`) | 8-bit x differences | an object outside the original screen is put 128 pixels from Ryu: nothing touches it |
| sprites (bank 4 `$81E6`, `$8222`) | x = 8-bit x + offset, a sprite whose add carries skipped, an object flipped at x < 8 skipped | 16-bit x, shown within `[-M - 8, 256 + M)`, high byte to `$5100` + OAM offset |

`LO = -M`, `HI = 256 + M`; M is 0 when the option "Enemies in the margins" is off and in vertical
areas (`$6C` bit 7: as wide as the screen), so with M = 0 objects are removed at the original
edges. The game's behaviours and collisions compare 8-bit x: keeping them to the original screen
keeps the game's logic exactly the original's there.

CPU time: the hack's work and the objects of the margins cost time. The hack asks for 64 lines of
overclock (`$5006`), which leaves the picture and the sound untouched; 255 with the option "No
slowdown" (on by default). Zaxnes ends the overclock as soon as the game waits for the NMI
(`idle_loop` `$9A87`-`$9A8A`), so a frame that does not need it costs nothing more. Measured with a
temporary trace (NMI taken outside the game's two waits, `$9A87` and the IRQ wait `$986C`), 900
frames of each area walking right and attacking: the original game has no lag frame (one in all of
them); the hack at 64 lines with the enemies in the margins lags in 5-1C (area `$28`, 101 frames)
and a few in 4-2B; "No slowdown" leaves none.

## 3. The extension

Unlocked at reset (`$FFF6`, `JMP $CCC6` becomes `JMP wide_reset`) and checked at the end of every
NMI (`$C7A2`, `INC $5E / LDA $25 / STA $8000` becomes `JSR wide_nmi`, after the video updates and
the scroll, so the vblank's work is not delayed), which unlocks again if needed, then writes
`$5003 = M` (`min($5002, 248)`), `$5004 = $5005 = 0` (the nametable columns are hidden: the
decoder draws the background), `$5007 = $02` (sprites free of those extents; the high bytes are
not kept from one page to the next, since the game's other sprite builders write none),
`$5006 = 64` (255 with `$500A` bit 2), the objects' margin, K and the live range, and does the
replaced instructions.

Variables (extension RAM): `WV_M $5200`, `WV_MOBJ $5201`, `WV_K $5202`, the live range
`WV_LO $5203`/`$5204` and `WV_HI $5205`/`$5206` (signed 16-bit), the sprite origin's high byte
`WV_OH $5207`, scratch `$5208`-`$520A`, a list enemy's offset `WV_SPD $520B`/`$520C`; per slot the
8-bit x last seen `WX_LO $5220 + slot`, the high byte `WX_HI $5240 + slot`, followed
`WX_ACT $5260 + slot`.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$FFF6` | `JMP $CCC6` | `JMP wide_reset` |
| `$C7A2` | `INC $5E / LDA $25 / STA $8000` | `JSR wide_nmi` |
| `$CD90` | `LDA $04E6,X` (then `BPL`) | `JSR wide_slot_head` |
| `$CDB8` | `LDA $D2 / STA $04E6,X` | `JSR wide_slot_store` |
| `$CDB3` | `JSR $CDFD` | `JSR wide_behaviour` |
| `$CDB0` | `JSR $8ABC` | `JSR wide_background` |
| bank 4 `$99F6`, `$9A07` | `PHP / JMP $9A45` | `JMP wide_moved` |
| bank 4 `$81E6` | `SEC / LDA $0516,X / BIT $FC / BVC / SBC #8 / BCC $81B1 / STA $FD` | `JSR wide_sprite_origin / JMP $81F4` |
| bank 4 `$8222` | `CLC / ADC $FD / STA $0203,X / PHP / PLA / ROR / ROR / EOR $FF / BMI $8262` | `JSR wide_sprite_x / BMI $8262` |
| bank 4 `$827E` | `CLC / LDA $D1 / ADC $86 / CMP ($8E),Y / BNE $826E` | `JSR wide_list_right / BCC $826E` |
| bank 4 `$8293` | `LDX $D1 / INX / TXA / CMP ($8E),Y / BNE $826E` | `JSR wide_list_left / BCC $826E` |
| bank 4 `$82A8` | `LDA #$80 / STA $04E6,X` | `JSR wide_list_made` |
| bank 4 `$82B2` | `STA $0516,X / LDA $DA / STA $04FE,X` | `JSR wide_list_x` |
| bank 5 `$A050`, `$A12C`, `$A1DB`, `$A234`, `$A2CA` | `LDA $0516,X` | `JSR wide_collision_x` |

## 5. Options

The options the player can turn on or off (the game's hacks screen, with the advanced mode on);
Zaxnes writes their bits at `$500A` every frame, the hack reads them there (`WIDE_OPTIONS`).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 0 | `enemies_in_margins` | Enemies in the margins / Ennemis dans les marges | on | off, M = 0 for the objects: they appear and go at the original edges (the background of the margins and the 16-bit sprites stay) |
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | 255 lines of overclock instead of 64: the game never runs late (no lag frame) |

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Ninja Gaiden III - The Ancient Ship of Doom (USA).nes" build/ng3wide.bps
```

assembles `src/wide.s` for `$E700`-`$EFFF` (`-g`, labels from `ld65 -Ln`), checks the code space
is still `$FF`, writes the code and the hooks, `build/ng3wide.nes` and the BPS patch
(`tools/make_bps.py`, SourceRead / TargetRead only). `WIDE_SKIP=<hex addresses>` leaves hooks out,
for testing.

## 7. Limits

* A dormant object does not behave: an enemy waits in the margin until it comes onto the screen,
  and one that walks off the screen goes on at its speed, its behaviour still, until it leaves the
  margin. A spawner of the list in a margin makes nothing until it is on the screen, so the
  enemies it makes still appear at the original edge.
* The second list of the areas (`$807B`: lanterns and items, placed at a screen x when the camera
  reaches their column) is not widened.
* Slots 11-15 keep their own 16-bit x and hide outside the original screen, as in the game.
* No vertical margin (the decoder draws the sides only), and vertical areas keep their edges.

### A. `src/wide.s`

```asm
; Ninja Gaiden III (USA) widescreen hack: the code the patched banks call.
;
; Assembled for $E700 of the fixed bank (PRG bank 15, at $E000-$FFFF), in the unused $FF fill from
; $E6F2 to $F001. In a Zaxnes hack pack (-D PACK) it is assembled for $2000 instead: the zone
; fantôme, which Zaxnes shows to instruction fetches only from power-on, the extension unlocked
; (no reset hook then, and no unlock in the NMI); the game's data reads there still reach the PPU,
; so the code reads no data of its own. Its registers and RAM are at $3000-$3FFF, which the zone
; holds for its own instructions (widescreen MMC5: $5000-$5FFF stays the cartridge's). Ninja
; Ryuuken Den III (Japan) (-D JAPAN) has the same code and RAM, its fixed banks' code from $C7A2 on
; 9 bytes later and from $CC00 on 54 earlier, bank 4's from $8ABC on 8 to 16 earlier.
; Talks to the Zaxnes widescreen extension (rust/docs/WIDESCREEN-EXTENSION.md); the background of
; the margins comes from the emulator's Ninja Gaiden III margin decoder, the hack does the objects.
;
; The game keeps each object's x as an 8-bit screen x ($0516,X, 24 slots): a move or the scroll
; that carries it past 0 or 255 removes the object. The hack follows every object's 16-bit screen
; x in the extension's RAM (wide_sync: the change of its 8-bit x since it was last seen, small
; steps only, a jump being a new place on the screen), and so:
;
;  * an object lives until it leaves [-M, 256 + M) instead of [0, 256);
;  * the enemies of the area's list (slots 4-7) appear M pixels earlier, in the margins;
;  * an object outside the original screen (slots 4-7 and 16-23) is dormant: drawn and moved by its
;    speed and the scroll, its behaviour and its collision with the background skipped until it
;    comes back onto the screen (the game's behaviours compare 8-bit x);
;  * an object outside the original screen touches nothing: the game's collisions compare 8-bit x,
;    so such an object is put 128 pixels away from Ryu for them;
;  * every sprite gets the high byte of its 16-bit x, so it shows in the margins.
;
; Vertical areas, as wide as the screen, keep the original edges (M = 0). With M = 0 every routine
; does what the code it replaces does.

.ifdef PACK
EXT             = $3000         ; the extension's registers and RAM, in the zone fantôme
.else
EXT             = $5000
.endif
WIDE_SIG        = EXT + $000    ; W: unlock "WIDE"; R: $57 once unlocked
WIDE_HOST       = EXT + $002    ; R: margin the host offers on each side
WIDE_REQ        = EXT + $003    ; W: margin asked for
WIDE_LEFT       = EXT + $004    ; W: background extents (the decoder draws the margins)
WIDE_RIGHT      = EXT + $005
WIDE_OVERCLOCK  = EXT + $006    ; W: extra CPU time each frame, in scanlines
WIDE_LAYOUT     = EXT + $007    ; W: bit 1, sprites free of the background extents
WIDE_OPTIONS    = EXT + $00A    ; R: the options the player turned on (bit 0: enemies in the
                                ; margins, bit 2: no slowdown)
WIDE_SPRHI      = EXT + $100    ; W: + 4 n: high byte of the x of sprite n

MAX_MARGIN      = 248           ; the widest margin the extension's 8-bit registers carry
.ifndef OVERCLOCK_LINES
OVERCLOCK_LINES = 64            ; pays for the hack's work and the objects of the margins
NO_SLOWDOWN_LINES = 255         ; with the option "No slowdown": the game never runs late
.endif
TELEPORT        = 32            ; a change of x larger than this is a new place, not a move

; Variables in the extension's RAM.
WV_M            = EXT + $200    ; margin of this frame (sprites)
WV_MOBJ         = EXT + $201    ; margin objects live in (0: the option is off, a vertical area)
WV_K            = EXT + $202    ; columns of 16 pixels the enemies of the list appear earlier
WV_LOL          = EXT + $203    ; screen x range objects live in: [LO, HI), signed 16-bit
WV_LOH          = EXT + $204
WV_HIL          = EXT + $205
WV_HIH          = EXT + $206
WV_OH           = EXT + $207    ; high byte of the sprite origin of the object being drawn
WV_T0           = EXT + $208    ; scratch
WV_T1           = EXT + $209
WV_T2           = EXT + $20A
WV_SPDL         = EXT + $20B    ; a list enemy's x past the original spawn x, signed 16-bit
WV_SPDH         = EXT + $20C
WX_LO           = EXT + $220    ; per slot: the 8-bit x last seen,
WX_HI           = EXT + $240    ; the high byte of the 16-bit screen x,
WX_ACT          = EXT + $260    ; and 0 for a slot not followed (free, or a new object)

; The game.
.ifdef JAPAN
ORIGINAL_RESET  = $CC90
BEHAVIOUR       = $CDC7         ; runs the behaviour of object X
BG_COLLISION    = $8AB4         ; bank 4: object X against the background
.else
ORIGINAL_RESET  = $CCC6
BEHAVIOUR       = $CDFD
BG_COLLISION    = $8ABC
.endif
FRAME_COUNTER   = $5E
BANK_SELECT     = $25           ; the last MMC3 bank select, written back to $8000 by the NMI
AREA_FLAGS      = $6C           ; bit 7: a vertical area
SLOT            = $45           ; the object loop's slot
OBJ_FLAGS       = $D2           ; the object loop's copy of OBJ_ACTIVE
OBJ_ACTIVE      = $04E6         ; bit 7: the slot is in use
OBJ_X           = $0516         ; screen x
OBJ_XFRAC       = $04FE
CAMERA_COLUMN   = $D1           ; camera x / 16
SPAWN_AHEAD     = $86           ; the list's right edge, in columns past the camera ($10)
CAMERA_FRAC     = $DA
LIST_COLUMNS    = $8E           ; pointer: the column of each entry of the area's enemy list
SPR_FLAGS       = $FC           ; sprite builder: the object's attributes (bit 6: flipped)
SPR_ORIGIN      = $FD           ; sprite builder: the object's x (minus 8 when flipped)
SPR_OFFSET      = $FF           ; sprite builder: the sprite's x offset
OAM_X           = $0203
RYU_X           = $0516

.segment "CODE"
.ifdef PACK
.org $2000
.else
.org $E700
.endif

.ifndef PACK
; ---------------------------------------------------------------------------- ;
; Reset ($FFF6, in place of JMP $CCC6).
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
    STA      WV_MOBJ
    STA      WV_K
    RTS
.endif

; ---------------------------------------------------------------------------- ;
; NMI's end ($C7A2, in place of INC $5E / LDA $25 / STA $8000, after the video updates and the
; scroll): the extension's settings for the next frame (unlocked again first if a state made
; without the hack left it locked, but for the pack, whose code Zaxnes opens again itself).
wide_nmi:
.ifndef PACK
    LDA      WIDE_SIG
    CMP      #$57
    BEQ      @set
    JSR      wide_unlock
@set:
.endif
    LDA      WIDE_HOST
    CMP      #MAX_MARGIN
    BCC      @margin
    LDA      #MAX_MARGIN
@margin:
    STA      WV_M
    STA      WIDE_REQ
    LDA      WIDE_OPTIONS               ; enemies in the margins (option bit 0), horizontal areas
    AND      #$01
    BEQ      @objects
    BIT      AREA_FLAGS
    BMI      @vertical
    LDA      WV_M
    JMP      @objects
@vertical:
    LDA      #$00
@objects:
    STA      WV_MOBJ
    LSR      A                          ; K = M / 16: a list enemy appears within the margin
    LSR      A
    LSR      A
    LSR      A
    STA      WV_K
    SEC                                 ; LO = -M, HI = 256 + M
    LDA      #$00
    SBC      WV_MOBJ
    STA      WV_LOL
    LDA      #$00
    SBC      #$00
    STA      WV_LOH
    LDA      WV_MOBJ
    STA      WV_HIL
    LDA      #$01
    STA      WV_HIH
    LDA      #$00
    STA      WIDE_LEFT
    STA      WIDE_RIGHT
    LDA      #$02                       ; sprites free of the extents
    STA      WIDE_LAYOUT
    LDA      WIDE_OPTIONS               ; no slowdown (option bit 2): nearly a frame more
    AND      #$04
    BEQ      @overclock
    LDA      #NO_SLOWDOWN_LINES - OVERCLOCK_LINES
@overclock:
    CLC
    ADC      #OVERCLOCK_LINES
    STA      WIDE_OVERCLOCK
    INC      FRAME_COUNTER
    LDA      BANK_SELECT
    STA      $8000
    RTS

; ---------------------------------------------------------------------------- ;
; Follows the 16-bit screen x of slot X from its 8-bit x: A = its high byte (Z set when 0). A slot
; not followed yet, or whose x jumped, is taken where its 8-bit x puts it on the screen.
wide_sync:
    LDA      WX_ACT,x
    BEQ      @new
    SEC
    LDA      OBJ_X,x
    SBC      WX_LO,x
    STA      WV_T0                      ; d, the signed change
    CLC
    ADC      #TELEPORT
    CMP      #2 * TELEPORT + 1
    BCS      @new
    LDA      WX_LO,x                    ; high byte += sign of d + carry of (low + d)
    CLC
    ADC      WV_T0
    LDA      WV_T0
    AND      #$80
    BEQ      @positive
    LDA      #$FF
@positive:
    ADC      WX_HI,x
    STA      WX_HI,x
    LDA      OBJ_X,x
    STA      WX_LO,x
    LDA      WX_HI,x
    RTS
@new:
    LDA      OBJ_X,x
    STA      WX_LO,x
    LDA      #$01
    STA      WX_ACT,x
    LDA      #$00
    STA      WX_HI,x
    RTS

; The same, cheaply when the 8-bit x did not change since: A = the high byte (Z set when 0), the
; carry kept.
wide_hi:
    LDA      WX_ACT,x
    BEQ      @sync
    LDA      OBJ_X,x
    EOR      WX_LO,x
    BNE      @sync
    LDA      WX_HI,x
    RTS
@sync:
    PHP
    JSR      wide_sync
    PLP
    LDA      WX_HI,x
    RTS

; ---------------------------------------------------------------------------- ;
; The object loop ($CD90, in place of LDA $04E6,X; then BPL): a free slot is no longer followed.
wide_slot_head:
    LDA      OBJ_ACTIVE,x
    BMI      @rts
    LDA      #$00
    STA      WX_ACT,x
    LDA      OBJ_ACTIVE,x
@rts:
    RTS

; The object loop's end ($CDB8, in place of LDA $D2 / STA $04E6,X): the same for an object its
; turn removed.
wide_slot_store:
    LDA      OBJ_FLAGS
    STA      OBJ_ACTIVE,x
    BMI      @rts
    LDA      #$00
    STA      WX_ACT,x
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; The object loop's behaviour ($CDB3, in place of JSR $CDFD): skipped for an object of slots 4-7
; and 16-23 outside the original screen.
wide_behaviour:
    CPX      #$10
    BCS      @check
    CPX      #$08
    BCS      @run
@check:
    JSR      wide_hi
    BNE      @rts
@run:
    JMP      BEHAVIOUR
@rts:
    RTS

; The object loop's collision with the background ($CDB0, in place of JSR $8ABC, slots 4-7 and
; 16-23): skipped as the behaviour is.
wide_background:
    JSR      wide_hi
    BNE      @rts
    JMP      BG_COLLISION
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; The test after an object moved sideways (bank 4 $99F6 and $9A07, in place of PHP / JMP $9A45,
; reached by JMP): N set when object X left [LO, HI), the caller then removes it. The original
; tests the carry of the 8-bit move.
wide_moved:
    JSR      wide_hi
    LDA      WX_LO,x                    ; x - LO < 0: out
    CMP      WV_LOL
    LDA      WX_HI,x
    SBC      WV_LOH
    BVC      @lo
    EOR      #$80
@lo:
    BMI      @out
    LDA      WX_LO,x                    ; x - HI >= 0: out
    CMP      WV_HIL
    LDA      WX_HI,x
    SBC      WV_HIH
    BVC      @hi
    EOR      #$80
@hi:
    BPL      @out
    LDA      #$00
    RTS
@out:
    LDA      #$80
    RTS

; ---------------------------------------------------------------------------- ;
; The sprite builder, an object's origin (bank 4 $81E6, in place of the 13 bytes that load the
; object's x, minus 8 when it is flipped, and skip an object that would wrap): the 16-bit origin,
; low byte in $FD, high byte in WV_OH.
wide_sprite_origin:
    JSR      wide_hi
    STA      WV_OH
    LDA      OBJ_X,x
    BIT      SPR_FLAGS
    BVC      @done
    SEC
    SBC      #$08
    BCS      @done
    DEC      WV_OH
@done:
    STA      SPR_ORIGIN
    RTS

; The sprite builder, a sprite's x (bank 4 $8222, in place of CLC / ADC $FD / STA $0203,X and the
; test of its carry): A = the sprite's signed offset, X = its OAM offset. N set: the sprite is past
; the margins, skipped; otherwise its x and its high byte are written.
wide_sprite_x:
    CLC
    ADC      SPR_ORIGIN
    STA      WV_T0
    LDA      WV_OH                      ; an object on the screen: the original test (its high
    BNE      @wide                      ; byte stays 0, as each sprite page starts)
    BIT      SPR_OFFSET
    BMI      @back
    BCS      @right
@near:
    LDA      WV_T0
    STA      OAM_X,x
    LDA      #$00
    RTS
@back:
    BCS      @near
    LDA      #$FF
    BNE      @far
@right:
    LDA      #$01
    BNE      @far
@wide:
    LDA      SPR_OFFSET                 ; high byte = origin's + sign of the offset + carry
    AND      #$80
    BEQ      @positive
    LDA      #$FF
@positive:
    ADC      WV_OH
@far:
    STA      WV_T1
    BEQ      @show                      ; on the screen
    CMP      #$01
    BNE      @left
    LDA      WV_T0                      ; right margin: x - 256 < M
    CMP      WV_M
    BCS      @hide
    BCC      @show
@left:
    CMP      #$FF
    BNE      @hide
    LDA      #$F8                       ; left margin: x >= -M - 8 (a sprite is 8 wide)
    SEC
    SBC      WV_M
    STA      WV_T2
    LDA      WV_T0
    CMP      WV_T2
    BCC      @hide
@show:
    LDA      WV_T0
    STA      OAM_X,x
    LDA      WV_T1
    STA      WIDE_SPRHI,x
    LDA      #$00
    RTS
@hide:
    LDA      #$80
    RTS

; ---------------------------------------------------------------------------- ;
; The collisions (bank 5 $A050, $A12C, $A1DB, $A234 and $A2CA, in place of LDA $0516,X): the
; x of object X, or for an object outside the original screen a place 128 pixels from Ryu, which
; nothing near Ryu reaches. The carry is kept.
wide_collision_x:
    JSR      wide_hi
    BNE      @away
    LDA      OBJ_X,x
    RTS
@away:
    LDA      RYU_X
    EOR      #$80
    RTS

; ---------------------------------------------------------------------------- ;
; The enemy list, right side (bank 4 $827E, in place of CLC / LDA $D1 / ADC $86 / CMP ($8E),Y /
; BNE): carry set when entry Y appears now. The original makes it appear when its column is the
; one past the right edge; within the margin, up to K columns earlier, WV_SPD pixels past the
; original spawn x, once its slot is not taken by an object on the screen.
wide_list_right:
    CLC
    LDA      CAMERA_COLUMN
    ADC      SPAWN_AHEAD
    STA      WV_T0                      ; the original column
    LDA      WV_K
    BNE      @wide
    LDA      WV_T0
    CMP      (LIST_COLUMNS),y
    BNE      wide_list_no
    JMP      wide_list_original
@wide:
    LDA      (LIST_COLUMNS),y           ; columns past the original one
    SEC
    SBC      WV_T0
    BCC      wide_list_late
    CMP      WV_K
    BEQ      @in
    BCS      wide_list_no
@in:
    JSR      wide_list_offset
    JMP      wide_list_slot

; The enemy list, left side (bank 4 $8293, in place of LDX $D1 / INX / TXA / CMP ($8E),Y / BNE):
; the same past the left edge.
wide_list_left:
    LDA      CAMERA_COLUMN
    CLC
    ADC      #$01
    STA      WV_T0                      ; the original column
    LDA      WV_K
    BNE      @wide
    LDA      WV_T0
    CMP      (LIST_COLUMNS),y
    BNE      wide_list_no
    JMP      wide_list_original
@wide:
    LDA      WV_T0                      ; columns before the original one
    SEC
    SBC      (LIST_COLUMNS),y
    BCC      wide_list_late
    CMP      WV_K
    BEQ      @in
    BCS      wide_list_no
@in:
    JSR      wide_list_offset
    SEC                                 ; to the left: WV_SPD = -WV_SPD
    LDA      #$00
    SBC      WV_SPDL
    STA      WV_SPDL
    LDA      #$00
    SBC      WV_SPDH
    STA      WV_SPDH
    JMP      wide_list_slot

wide_list_late:                         ; past its column (its slot was taken): the original x
    LDA      #$00
    STA      WV_SPDL
    STA      WV_SPDH
wide_list_slot:                         ; slot (Y & 3) | 4 held by an object on the screen: wait
    TYA
    AND      #$03
    ORA      #$04
    TAX
    LDA      OBJ_ACTIVE,x
    BPL      wide_list_yes
    LDA      WX_ACT,x
    BEQ      wide_list_no
    LDA      WX_HI,x
    BEQ      wide_list_no
wide_list_yes:
    SEC
    RTS
wide_list_original:
    LDA      #$00
    STA      WV_SPDL
    STA      WV_SPDH
    SEC
    RTS
wide_list_no:
    CLC
    RTS

; WV_SPD = 16 A.
wide_list_offset:
    STA      WV_SPDL
    LDA      #$00
    STA      WV_SPDH
    LDA      WV_SPDL
    ASL      A
    ROL      WV_SPDH
    ASL      A
    ROL      WV_SPDH
    ASL      A
    ROL      WV_SPDH
    ASL      A
    ROL      WV_SPDH
    STA      WV_SPDL
    RTS

; A list enemy is made (bank 4 $82A8, in place of LDA #$80 / STA $04E6,X): its slot is a new one.
wide_list_made:
    LDA      #$80
    STA      OBJ_ACTIVE,x
    LDA      #$00
    STA      WX_ACT,x
    RTS

; Its x, horizontal areas (bank 4 $82B2, in place of STA $0516,X / LDA $DA / STA $04FE,X): A = the
; original spawn x ($87 or $88), moved by WV_SPD.
wide_list_x:
    CLC
    ADC      WV_SPDL
    STA      OBJ_X,x
    STA      WX_LO,x
    LDA      #$00
    ADC      WV_SPDH
    STA      WX_HI,x
    LDA      #$01
    STA      WX_ACT,x
    LDA      CAMERA_FRAC
    STA      OBJ_XFRAC,x
    RTS

wide_end:
.ifdef PACK
.assert wide_end <= $3000, error, "widescreen code past the zone's code half"
.else
.assert wide_end <= $F000, error, "widescreen code past the free space"
.endif
```

### B. `tools/build.py`

```python
"""Builds the Ninja Gaiden III (USA) widescreen hack and its BPS patch.

Usage: python -I tools/build.py <Ninja Gaiden III (USA).nes> [out.bps]
       python -I tools/build.py <Ninja Gaiden III (USA) or Ninja Ryuuken Den III (Japan).nes>
                                --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $E700 of the
   fixed bank (PRG bank 15), into the unused $FF fill from $E6F2 to $F001.
2. Writes the hooks, each checked against the bytes it replaces.
3. Writes build/ng3wide.nes and, if asked, the BPS patch (data without iNES header).

With --pack, a Zaxnes hack pack, format 2, instead, written by Zaxnes' shared builder
(tools/hackpack.py of Zaxnes, its folder in the environment variable ZAXNES_TOOLS): NTSC, MMC3,
each hook a 16 KiB bank and a CPU address, the widescreen in two groups ("core", Base: the objects'
life, dormancy, collisions and sprites in the margins; "enemies", needing it: the area's enemy list
in the margins), with checks of the game's code the hack relies on; src/wide.s assembled with PACK
for $2000, the zone fantôme, which Zaxnes shows to instruction fetches from power-on, the extension
unlocked; the hooks, but for the reset hook, each with the game's bytes it replaces, checked by
Zaxnes before it applies them. Nothing goes in the game's free space and the ROM keeps its size.
Ninja Ryuuken Den III (Japan) gets its own pack (-D JAPAN: the same code, moved).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x902E3168
JAPAN = 0x2F2E30F7              # Ninja Ryuuken Den III: Yomi no Hakobune (Japan)
BANK_SIZE = 0x2000              # MMC3: 8 KiB PRG banks
CODE_BANK = 15                  # the fixed bank at $E000
CODE_START = 0xE700
CODE_END = 0xF000
PACK_START = 0x2000             # the pack's code: the zone fantôme, below the extension at $3000
PACK_SIZE = 0x1000

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    offset = target - (at + 2)
    assert -128 <= offset <= 127, (hex(at), hex(target))
    return bytes([opcode, offset & 0xFF])


BMI, BCC = 0x30, 0x90
NOP = 0xEA


def offset_of(bank, address):
    """File offset (data without header) of `address` in PRG bank `bank`; the fixed banks 14
    and 15 are at $C000 and $E000."""
    if address >= 0xC000:
        bank = 14 if address < 0xE000 else 15
    return bank * BANK_SIZE + (address & 0x1FFF)


def hooks(sym, pack=False):
    """The patches: (bank, address, original bytes, new bytes); bank None for the fixed banks.
    The pack has no reset hook: Zaxnes unlocks the extension."""
    out = []

    def put(bank, address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((bank, address, original, new))

    # Reset: unlock the extension first.
    if not pack:
        put(None, 0xFFF6, '4c c6 cc', jmp(sym['wide_reset']))
    # NMI's end: the extension's settings for the next frame.
    put(None, 0xC7A2, 'e6 5e a5 25 8d 00 80', jsr(sym['wide_nmi']))
    # The object loop: free slots, removed objects, behaviour and background of dormant objects.
    put(None, 0xCD90, 'bd e6 04', jsr(sym['wide_slot_head']))
    put(None, 0xCDB8, 'a5 d2 9d e6 04', jsr(sym['wide_slot_store']))
    put(None, 0xCDB3, '20 fd cd', jsr(sym['wide_behaviour']))
    put(None, 0xCDB0, '20 bc 8a', jsr(sym['wide_background']))
    # Bank 4: the tests after a sideways move.
    put(4, 0x99F6, '08 4c 45 9a', jmp(sym['wide_moved']))
    put(4, 0x9A07, '08 4c 45 9a', jmp(sym['wide_moved']))
    # Bank 4: the sprite builder, an object's origin and each sprite's x.
    put(4, 0x81E6, '38 bd 16 05 24 fc 50 04 e9 08 90 bf 85 fd',
        jsr(sym['wide_sprite_origin']) + jmp(0x81F4))
    put(4, 0x8222, '18 65 fd 9d 03 02 08 68 6a 6a 45 ff 30 32',
        jsr(sym['wide_sprite_x']) + branch(BMI, 0x8225, 0x8262))
    # Bank 4: the enemy list, its two edges, the object made and its x.
    put(4, 0x827E, '18 a5 d1 65 86 d1 8e d0 e7',
        jsr(sym['wide_list_right']) + branch(BCC, 0x8281, 0x826E))
    put(4, 0x8293, 'a6 d1 e8 8a d1 8e d0 d3',
        jsr(sym['wide_list_left']) + branch(BCC, 0x8296, 0x826E))
    put(4, 0x82A8, 'a9 80 9d e6 04', jsr(sym['wide_list_made']))
    put(4, 0x82B2, '9d 16 05 a5 da 9d fe 04', jsr(sym['wide_list_x']))
    # Bank 5: the collisions.
    for address in (0xA050, 0xA12C, 0xA1DB, 0xA234, 0xA2CA):
        put(5, address, 'bd 16 05', jsr(sym['wide_collision_x']))
    return out


def tool(name):
    return [name] if shutil.which(name) else ['wsl', '-e', name]


def wsl_path(path):
    if shutil.which('ca65'):
        return path
    drive, rest = os.path.splitdrive(os.path.abspath(path))
    return '/mnt/' + drive[0].lower() + rest.replace('\\', '/')


def assemble(defines=()):
    pack = 'PACK' in defines
    build = os.path.join(ROOT, 'build')
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    start, size = (PACK_START, PACK_SIZE) if pack else (CODE_START, CODE_END - CODE_START)
    open(cfg, 'w', newline='\n').write(
        f'MEMORY {{ CODE: start = ${start:04X}, size = ${size:04X}, '
        'fill = yes, fillval = $FF, file = %O; }\n'
        'SEGMENTS { CODE: load = CODE, type = ro; }\n')
    obj = os.path.join(build, 'wide.o')
    binary = os.path.join(build, 'wide.bin')
    labels = os.path.join(build, 'wide.labels')
    subprocess.run(tool('ca65') + ['-g'] + [a for d in defines for a in ('-D', d)]
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


# Where Ninja Ryuuken Den III (Japan) has the USA's places the pack hooks or checks (the same
# instructions and RAM): its fixed banks' code from $C7A2 on 9 bytes later and from $CC00 on 54
# earlier, bank 4's from $8ABC on 8 to 16 earlier, bank 5's collisions 16 earlier.
JAPAN_PLACES = {
    0xC7A2: 0xC7AB, 0xCD90: 0xCD5A, 0xCDB8: 0xCD82, 0xCDB3: 0xCD7D, 0xCDB0: 0xCD7A,
    0xCDFD: 0xCDC7, 0x8ABC: 0x8AB4, 0x99F6: 0x99E6, 0x9A07: 0x99F7, 0xA050: 0xA040,
    0xA12C: 0xA11C, 0xA1DB: 0xA1CB, 0xA234: 0xA224, 0xA2CA: 0xA2BA,
}
# Its hooks whose bytes differ (a JSR's or a JMP's target moved).
JAPAN_ORIGINAL = {0xCDB0: '20 b4 8a', 0xCDB3: '20 c7 cd', 0x99F6: '08 4c 35 9a',
                  0x9A07: '08 4c 35 9a'}

# The enemies group: the area's enemy list (bank 4).
ENEMIES = {0x827E, 0x8293, 0x82A8, 0x82B2}

# The game's code the hack's code relies on, by group: (bank, USA address, length), checked by
# Zaxnes to be the reference ROM's bytes. Core: the behaviour and the background collision the
# dormant objects skip, where the sprite builder goes on after an object's origin and where a
# sprite past the margins is skipped. Enemies: where the list goes on for an entry that does not
# appear, and for one that does on either side.
CHECKS = {
    'core': [(None, 0xCDFD, 3), (4, 0x8ABC, 3), (4, 0x81F4, 3), (4, 0x8262, 3)],
    'enemies': [(4, 0x826E, 3), (4, 0x8287, 3), (4, 0x829B, 3)],
}


def pack(data, crc, out):
    tools = os.environ.get('ZAXNES_TOOLS')
    if not tools or not os.path.isfile(os.path.join(tools, 'hackpack.py')):
        sys.exit('ZAXNES_TOOLS: the rust/tools folder of Zaxnes (its hackpack.py) is needed')
    sys.path.insert(0, tools)
    import hackpack
    japan = crc == JAPAN
    code, sym = assemble(['PACK'] + (['JAPAN'] if japan else []))
    used = (sym['wide_end'] - PACK_START + 0xFF) & ~0xFF

    def place(bank, address):
        if japan:
            address = JAPAN_PLACES.get(address, address)
        return pack_place(offset_of(bank, address), address)

    name = ('Ninja Ryuuken Den III (Japan)' if japan else 'Ninja Gaiden III (USA)') + ' widescreen'
    pack = hackpack.Pack(name, code[:used], region='ntsc', mappers=(4,))
    widescreen = pack.component('widescreen')
    groups = {
        'core': widescreen.group('core', label='Base', label_fr='Base'),
        'enemies': widescreen.group('enemies', requires=['core'], label='Enemies in the margins',
                                    label_fr='Ennemis dans les marges'),
    }
    for group, checks in CHECKS.items():
        for bank, address, length in checks:
            groups[group].check(*place(bank, address), length)
    for bank, address, original, new in hooks(sym, pack=True):
        if japan and address in JAPAN_ORIGINAL:
            original = bytes.fromhex(JAPAN_ORIGINAL[address])
        at = offset_of(bank, JAPAN_PLACES.get(address, address) if japan else address)
        assert data[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        group = 'enemies' if address in ENEMIES else 'core'
        groups[group].hook(*place(bank, address), bytes(original), bytes(new))
    result = pack.build(data, 0x20000)
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes ({used} of code, '
          f'{" + ".join(str(len(group.hooks)) for group in groups.values())} hooks), '
          f'pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:]
    crc = zlib.crc32(data)
    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        assert crc in (SOURCE_CRC, JAPAN), 'not Ninja Gaiden III (USA) or (Japan)'
        pack(data, crc, sys.argv[3])
        return
    assert crc == SOURCE_CRC, 'not Ninja Gaiden III (USA)'
    code, sym = assemble()
    start = offset_of(CODE_BANK, CODE_START)
    assert all(b == 0xFF for b in data[start:start + len(code)]), 'the code space is not free'
    data[start:start + len(code)] = code

    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for bank, address, original, new in hooks(sym):
        if address in skip:
            continue
        at = offset_of(bank, address)
        where = f'bank {bank} ${address:04X}' if bank is not None else f'${address:04X}'
        assert data[at:at + len(original)] == original, f'{where}: not the expected bytes'
        data[at:at + len(new)] = new

    out = os.path.join(ROOT, 'build', 'ng3wide.nes')
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

Usage: python -I tools/make_bps.py <original.nes> <build/ng3wide.nes> <out.bps>

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
