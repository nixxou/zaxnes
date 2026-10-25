# Mega Man 2 (USA): the widescreen hack

This document describes completely the modification of *Mega Man 2 (USA)* that Zaxnes ships as a
built-in patch. The appendices hold the new code and the build script, which writes every hook
after checking the bytes it replaces: with them the ROM can be rebuilt identically.

The background of the margins is not the hack's work: the emulator's Mega Man 2 margin decoder
reads the stage layout in PRG-ROM (see [`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md),
section 4). The hack does the objects: enemies that spawn, live and show in the margins.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Mega Man 2 (USA)*, MMC1, 256 KiB PRG (16 banks of 16 KiB, bank 15 fixed at `$C000`), CHR-RAM |
| Source data (iNES header excluded) | CRC-32 `0FCFC04D` |
| Target data | CRC-32 `86750B14` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | new code at `$F300` of bank 15 (in the `$FF` fill `$F2E3`-`$F8FF`), hooks in bank 15 and one in bank 14 |
| Patch | `rust/crates/zax-nes/patches/megaman2-usa-widescreen.bps`, made against the data without header |

---

## 2. What the game does, and what changes

Object slots: `$00`-`$0F` Mega Man and his weapons, `$10`-`$1F` enemies and their shots. Per slot:
`$0400` type, `$0420` flags (bit 7 active, bits 0-1 collision kinds), `$0440`/`$0460` x screen /
pixel, `$04A0` y, `$06E0` screen x low byte. Camera `$20:$1F`; the section being played spans
screens `$14` to `$15`.

| Game routine | Original | Hack |
|---|---|---|
| spawn window (`$D658`) | enemies of the stage list (`$B600` screen, `$B700` x, ... sorted by x) spawn between camera and camera + 255 | camera - M to camera + 255 + M, within the section |
| object moved left / right (`$EF20`, tests at `$EF43`, `$EF75`) | removed when its screen x leaves `[8, 248)` | enemies: removed when it leaves `[LO + 8, HI - 8)` |
| object that did not move sideways (`$EFE8`) | removed when off the screen (`$2F != 0`) | enemies: removed outside `[LO, HI)` |
| still on the screen? (`$E4E9`) | removed outside `[0, 256)` | enemies: outside `[LO, HI)` |
| collisions (`$EEBE`, `$EFB7`: `LDA $0420,X / AND #$03`) | on 8-bit screen x | none for an object outside the original screen |
| sprites (`$CE7D`, `$CECE`) | x = 8-bit screen x + signed offset, hidden when it carries | 16-bit x, shown within `[-M, 256 + M)`, high byte to `$5100 + OAM offset` |

| sprite loop of an object (`$CE9A`-`$CEF8`) | stops when the 64 sprites are used (the objects left over flicker, drawn first every other frame) | the same loop, moving on to the extension's second page (`$5300`) when the first is full |
| hiding the sprites (`$CC6C`) | the 64 sprites | both pages |
| energy bars (`$CFAD`, `$CFC9`, `$CFCE`, `$CFD3`) | write to `$0200` | write to the page in use |
| enemy spawn (`$D763`) and enemy loop (bank 14 `$9283`) | | an enemy of the stage list that spawns in a margin is dormant: drawn, but its behaviour is skipped until it comes onto the original screen; it then wakes for good (margins included). A dormant enemy is still removed outside `[LO, HI)` |
| cloud devil palette marker (bank 14 `$AF9D`, in place of `JSR $AF4C`) | an object of type `$42`/`$43` resets the palette of the heads of type `$40`/`$41` (2 or 3) to the first step of their fade-in (sky) when it reaches the screen | resets it only when no head of that type is alive (slots `$10`-`$1F`, `$0420` bit 7); otherwise the head before, still in the left margin, keeps its colours and the next one shows at once (the game skips the fade-in of a head whose palette is in colour, `$AE61`) |

`LO = max(-M, section start - camera)`, `HI = min(256 + M, section end - camera)`; M is the margin,
0 for slots below `$10` (Mega Man's shots keep the original limits). The game's collisions compare
8-bit screen x, so an object 256 pixels away would collide; outside the original screen nothing
collides, which keeps the game's logic exactly the original's on the screen. With M = 0 every
routine does exactly what the code it replaces does.

Margin: `M = min($5002, 248)` (248: the widest the extension's 8-bit registers carry; sprites show that far with `$5007` bit 1). Enemies use
`MOBJ`: `M`, or 0 when the player turns the option "Enemies in the margins" off (`$500A` bit 0,
on by default): they then spawn and go at the original edges, the second sprite page and the 16-bit
sprite x staying.

CPU time: the hack's work and the enemies of the margins cost time the console does not have; the
game, which builds its sprites during the frame, would run out of it and show half-built sprite
pages. The hack asks for 64 lines of overclock (`$5006`), which leaves the picture and the sound
untouched.

## 3. The extension

Unlocked at reset (`$FFE4`, `JMP $F2D1` becomes `JMP wide_reset`, which unlocks then jumps on) and
checked every NMI (`$CFFD`, `LDA $F7 / AND #$7C / STA $F7` becomes `JSR wide_nmi`, which unlocks
again if needed, then writes `$5003 = M`, `$5004 = $5005 = 0` (the nametable columns are hidden:
the decoder draws the background), `$5007 = $0E` (sprites free of those extents, second sprite page, high bytes kept from one page
to the next: the hack writes one for every sprite it places, so a frame the game runs late keeps
them with its half-built page), `$5006 = 64`, `MOBJ` from `$500A`,
and does the replaced instructions).

Variables (extension RAM): `WV_M $5200`, `WV_PAGE $5209` (bit 7: the second page is in use),
`WV_MOBJ $520A`, `WV_DORMANT $5220 + slot`, the live range `WV_LO $5201`/`$5202` and `WV_HI
$5203`/`$5204` (signed 16-bit), the object's screen x `WV_S $5205`/`$5206`, scratch `$5207`,
`$5208`.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$FFE4` | `JMP $F2D1` | `JMP wide_reset` |
| `$CFFD` | `LDA $F7 / AND #$7C / STA $F7` | `JSR wide_nmi` |
| `$D65F` | `CLC / LDA $1F / STA $0A / ADC #$FF / STA $08 / LDA $20 / STA $0B / ADC #0 / STA $09` | `JSR wide_spawn_window / JMP $D670` |
| `$EF43` | the 23-byte test "off the screen or x < 8" | `JSR wide_cull_left / BCS $EF8C / BCC $EF8A` |
| `$EF75` | the 21-byte test "off the screen or x >= 248" | `JSR wide_cull_right / BCS $EF8C` (then `CLC / RTS` at `$EF8A`) |
| `$E4E9` | "x - camera not on this screen" (15 bytes) | `JSR wide_cull_screen / BCS $E4FA` |
| `$EFE8` | `LDA $2F / BNE $EF8C / CLC / RTS` | `JMP wide_cull_still` |
| `$EEBE`, `$EFB7` | `LDA $0420,X / AND #$03 / BEQ` | `JSR wide_hit_flags / BEQ` (same target) |
| `$CE7D` | `SEC / LDA $0460,X / SBC $1F / STA $00 / LDA $0440,X / SBC $20` | `JSR wide_draw_origin` |
| `$CECE` | the 13-byte add and carry tests of a sprite's x | `JSR wide_sprite_x / BCS $CEDB / JMP $CEE2` (the loop around it is replaced, see `$CE9A`) |
| `$CC6C` | the loop hiding the 64 sprites | `JMP wide_clear_oam` |
| `$CE9A` | `LDX $06 / LDY $07` (head of an object's sprite loop) | `JMP wide_draw_loop` |
| `$CFAD`, `$CFC9`, `$CFCE`, `$CFD3` | `STA $0200,X`, `$0201,X`, `$0202,X`, `$0203,X` | `JSR wide_put_y` / `_tile` / `_attribute` / `_x` |
| `$D763` | `TYA / STA $0100,X` | `JSR wide_spawned` |
| bank 14 `$9283` | `LDY $0400,X` | `JSR wide_wake` |

`$EF8C` is the game's own removal (it also lets an enemy of the list spawn again); `$CEDB` hides a
sprite (OAM y `$F8`).

## 5. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Mega Man 2 (USA).nes" build/mm2wide.bps
```

assembles `src/wide.s` for `$F300`-`$F8FF` (`-g`, labels from `ld65 -Ln`), checks the code space is
still `$FF`, writes the code and the hooks, `build/mm2wide.nes` and the BPS patch (`tools/make_bps.py`,
SourceRead / TargetRead only). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.

## 6. Limits

* Enemies a stage's own code removes when off the screen, other than through the routines above,
  still go at the original edge.
* More enemies alive means more CPU time: the game's slowdowns can come more often.
* No vertical margin yet (the decoder draws the sides only).
* The 64 lines of overclock also shorten the game's own slowdowns.

---

## Appendices

### A. `src/wide.s`

```asm
; Mega Man 2 (USA) widescreen hack: the code the patched fixed bank calls.
;
; Assembled for $F300 of the fixed bank (PRG bank 15), the unused $FF fill from $F2E3 to $F8FF.
; Talks to the Zaxnes widescreen extension (rust/docs/WIDESCREEN-EXTENSION.md); the background of
; the margins comes from the emulator's Mega Man 2 margin decoder, the hack does the objects:
;
;  * enemies (object slots $10-$1F) spawn when they come within the margin of the screen, and
;    live until they leave it, all within the section being played (screens $14-$15);
;  * every object's sprites get the high byte of their 16-bit x, so they show in the margins;
;  * an enemy outside the original screen touches nothing and nothing touches it: the game's
;    collisions work on 8-bit screen x, and keep doing so on the original screen only;
;  * Mega Man's own objects (slots $00-$0F) keep the original limits.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

WIDE_SIG        = $5000         ; W: unlock "WIDE"; R: $57 once unlocked
WIDE_HOST       = $5002         ; R: margin the host offers on each side
WIDE_REQ        = $5003         ; W: margin asked for
WIDE_LEFT       = $5004         ; W: background extents (the decoder draws the margins)
WIDE_RIGHT      = $5005
WIDE_OVERCLOCK  = $5006         ; W: extra CPU time each frame, in scanlines
WIDE_LAYOUT     = $5007         ; W: bit 1, sprites free of the background extents
WIDE_OPTIONS    = $500A         ; R: the options the player turned on (bit 0: enemies in the margins)
WIDE_SPRHI      = $5100         ; W: high byte of the x of sprite n at $5100 + 4n
EXTRA_OAM       = $5300         ; the second sprite page ($5007 bit 2), OAM format
EXTRA_SPRHI     = $5400         ; high byte of the x of its sprite n at $5400 + 4n
OAM             = $0200

MAX_MARGIN      = 248           ; the widest margin the extension's 8-bit registers carry
.ifndef OVERCLOCK_LINES
OVERCLOCK_LINES = 64            ; pays for the hack's work and the enemies of the margins
.endif

; Variables in the extension's RAM.
WV_M            = $5200         ; margin of this frame
WV_LOL          = $5201         ; screen x range objects may live in: [LO, HI), signed 16-bit
WV_LOH          = $5202
WV_HIL          = $5203
WV_HIH          = $5204
WV_SL           = $5205         ; screen x of the object at hand, signed 16-bit
WV_SH           = $5206
WV_T0           = $5207
WV_T1           = $5208
WV_PAGE         = $5209         ; bit 7: the sprites being placed go to the second page
WV_MOBJ         = $520A         ; margin enemies spawn and live in: WV_M, or 0 with the option off
WV_DORMANT      = $5220         ; + slot ($10-$1F): non-zero while an enemy that spawned out of
                                ; sight has not come onto the original screen yet

; Game RAM (original meanings).
CAM_LO          = $1F           ; camera x, pixel
CAM_HI          = $20           ; camera x, screen
SECTION_FIRST   = $14           ; first screen of the section
SECTION_LAST    = $15           ; last screen of the section
OBJ_X_HI        = $0440         ; object x, screen
OBJ_X_LO        = $0460         ; object x, pixel

ORIGINAL_RESET  = $F2D1
KILL            = $EF8C         ; removes object X (and lets its enemy spawn again), SEC / RTS
NEXT_ENEMY      = $9299         ; bank 14: the enemy loop's next slot
TIKKI_PALETTE   = $AF4C         ; bank 14: writes step Y/4 of a cloud devil's fade-in to palette X
LAST_SLOT       = $1F           ; the last enemy slot
STAGE_ENEMY_TYPE = $B900        ; stage bank: the type of each entry of the enemy list
FIRST_ENEMY     = $10

.segment "CODE"
.org $F300

; ---------------------------------------------------------------------------- ;
; Reset ($FFE4, in place of JMP $F2D1).
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
    RTS

; ---------------------------------------------------------------------------- ;
; NMI ($CFFD, in place of LDA $F7 / AND #$7C / STA $F7): the extension's settings for the next
; frame (unlocked again first if a state made without the hack left it locked).
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
    LDA      WIDE_OPTIONS               ; enemies in the margins (option bit 0)
    AND      #$01
    BEQ      @objects
    LDA      WV_M
@objects:
    STA      WV_MOBJ
    LDA      #$00
    STA      WIDE_LEFT
    STA      WIDE_RIGHT
    LDA      #$0E                       ; sprites free of the extents, second sprite page,
                                        ; high bytes kept (every sprite placed gets one)
    STA      WIDE_LAYOUT
    LDA      #OVERCLOCK_LINES
    STA      WIDE_OVERCLOCK
    LDA      $F7
    AND      #$7C
    STA      $F7
    RTS

; ---------------------------------------------------------------------------- ;
; The screen x range objects of slot X may live in, into WV_LO/WV_HI:
;   LO = max(-M, section start - camera), HI = min(256 + M, section end - camera),
; M being 0 for Mega Man's own slots. The original's range is [0, 256).
wide_bounds:
    LDA      WV_MOBJ
    CPX      #FIRST_ENEMY
    BCS      @margin
    LDA      #$00
@margin:
    STA      WV_T0
    SEC                                 ; LO = section start - camera
    LDA      #$00
    SBC      CAM_LO
    STA      WV_LOL
    LDA      SECTION_FIRST
    SBC      CAM_HI
    STA      WV_LOH
    CLC                                 ; LO + M < 0: LO = -M
    LDA      WV_LOL
    ADC      WV_T0
    LDA      WV_LOH
    ADC      #$00
    BPL      @hi
    SEC
    LDA      #$00
    SBC      WV_T0
    STA      WV_LOL
    LDA      #$00
    SBC      #$00
    STA      WV_LOH
@hi:
    SEC                                 ; HI = section end - camera
    LDA      #$00
    SBC      CAM_LO
    STA      WV_HIL
    LDA      SECTION_LAST
    SBC      CAM_HI
    CLC
    ADC      #$01
    STA      WV_HIH
    SEC                                 ; HI - (256 + M) >= 0: HI = 256 + M
    LDA      WV_HIL
    SBC      WV_T0
    LDA      WV_HIH
    SBC      #$01
    BMI      @rts
    LDA      WV_T0
    STA      WV_HIL
    LDA      #$01
    STA      WV_HIH
@rts:
    RTS

; WV_S = screen x of object X (object x - camera).
wide_screen_x:
    SEC
    LDA      OBJ_X_LO,x
    SBC      CAM_LO
    STA      WV_SL
    LDA      OBJ_X_HI,x
    SBC      CAM_HI
    STA      WV_SH
    RTS

; Carry set when WV_S - (WV_LO + A) < 0 (A an unsigned offset).
wide_below_lo:
    CLC
    ADC      WV_LOL
    STA      WV_T1
    LDA      WV_LOH
    ADC      #$00
    STA      WV_T0
    LDA      WV_SL
    CMP      WV_T1
    LDA      WV_SH
    SBC      WV_T0
    BMI      @below
    CLC
    RTS
@below:
    SEC
    RTS

; Carry set when WV_S - (WV_HI - A) >= 0 (A an unsigned offset).
wide_from_hi:
    STA      WV_T1
    SEC
    LDA      WV_HIL
    SBC      WV_T1
    STA      WV_T1
    LDA      WV_HIH
    SBC      #$00
    STA      WV_T0
    LDA      WV_SL
    CMP      WV_T1
    LDA      WV_SH
    SBC      WV_T0
    BMI      @inside
    SEC
    RTS
@inside:
    CLC
    RTS

; ---------------------------------------------------------------------------- ;
; Object moved left ($EF43, in place of the 8-bit test "off the screen, or x < 8"): carry set
; when it goes (x < LO + 8, or x >= HI). $08 gets the screen x low byte, as before.
wide_cull_left:
    JSR      wide_bounds
    JSR      wide_screen_x
    LDA      WV_SL
    STA      $08
    LDA      #$08
    JSR      wide_below_lo
    BCS      @rts
    LDA      #$00
    JMP      wide_from_hi
@rts:
    RTS

; Object moved right ($EF75, in place of "off the screen, or x >= 248"): carry set when it goes
; (x >= HI - 8, or x < LO).
wide_cull_right:
    JSR      wide_bounds
    JSR      wide_screen_x
    LDA      WV_SL
    STA      $08
    LDA      #$08
    JSR      wide_from_hi
    BCS      @rts
    LDA      #$00
    JMP      wide_below_lo
@rts:
    RTS

; Object still on the screen? ($E4E9, in place of "x - camera not on this screen"): carry set
; when it goes (x < LO or x >= HI).
wide_cull_screen:
    JSR      wide_bounds
    JSR      wide_screen_x
    LDA      #$00
    JSR      wide_below_lo
    BCS      @rts
    LDA      #$00
    JMP      wide_from_hi
@rts:
    RTS

; Object that did not move sideways ($EFE8, in place of LDA $2F / BNE $EF8C / CLC / RTS: removed
; when off the screen): removed when outside [LO, HI).
wide_cull_still:
    JSR      wide_cull_screen
    BCC      @keep
    JMP      KILL
@keep:
    RTS

; ---------------------------------------------------------------------------- ;
; Collision flags of object X ($EEBE and $EFB7, in place of LDA $0420,X / AND #$03): 0 (Z set)
; when the object is outside the original screen, so it touches nothing.
wide_hit_flags:
    LDA      $0420,x
    AND      #$03
    BEQ      @rts
    STA      WV_T1
    JSR      wide_screen_x
    LDA      WV_SH
    BEQ      @on
    LDA      #$00
    RTS
@on:
    LDA      WV_T1
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; Sprite drawing. Origin ($CE7D, in place of SEC / LDA $0460,X / SBC $1F / STA $00 /
; LDA $0440,X / SBC $20): $00 = screen x low byte, WV_S = the 16-bit screen x.
wide_draw_origin:
    JSR      wide_screen_x
    LDA      WV_SL
    STA      $00
    RTS

; One sprite ($CECE, in place of the 8-bit add and its off-screen tests): A = the sprite's
; signed x offset, X = its OAM offset. Carry set: hidden (outside [-M, 256 + M)); otherwise
; A = OAM x and the high byte is written for the extension.
wide_sprite_x:
    STA      WV_T1
    LDA      #$00
    BIT      WV_T1
    BPL      @positive
    LDA      #$FF
@positive:
    STA      WV_T0                      ; sign of the offset
    CLC
    LDA      WV_SL
    ADC      WV_T1
    STA      WV_T1                      ; x low
    LDA      WV_SH
    ADC      WV_T0
    STA      WV_T0                      ; x high
    BMI      @left
    BEQ      @show                      ; 0 <= x < 256
    CMP      #$01
    BNE      @hide
    LDA      WV_T1                      ; 256 <= x: below 256 + M?
    CMP      WV_M
    BCS      @hide
    BCC      @show
@left:
    CMP      #$FF
    BNE      @hide
    LDA      WV_T1                      ; x < 0: x + M >= 0?
    CLC
    ADC      WV_M
    BCC      @hide
@show:
    LDA      WV_T0
    BIT      WV_PAGE
    BMI      @second
    STA      WIDE_SPRHI,x
    BPL      @low
@second:
    STA      EXTRA_SPRHI,x
@low:
    LDA      WV_T1
    CLC
    RTS
@hide:
    SEC
    RTS

; ---------------------------------------------------------------------------- ;
; Spawn window ($D65F, in place of $0B:$0A = camera, $09:$08 = camera + 255): widened by the
; margin, within the section being played.
wide_spawn_window:
    SEC                                 ; left = camera - M
    LDA      CAM_LO
    SBC      WV_MOBJ
    STA      $0A
    LDA      CAM_HI
    SBC      #$00
    STA      $0B
    BCC      @left_section              ; before the level
    CMP      SECTION_FIRST
    BCS      @right
@left_section:
    LDA      #$00                       ; before the section: its start
    STA      $0A
    LDA      SECTION_FIRST
    STA      $0B
@right:
    CLC                                 ; right = camera + 255 + M
    LDA      CAM_LO
    ADC      #$FF
    STA      $08
    LDA      CAM_HI
    ADC      #$00
    STA      $09
    CLC
    LDA      $08
    ADC      WV_MOBJ
    STA      $08
    LDA      $09
    ADC      #$00
    STA      $09
    CMP      SECTION_LAST
    BCC      @rts
    BEQ      @rts
    LDA      #$FF                       ; past the section: its last pixel
    STA      $08
    LDA      SECTION_LAST
    STA      $09
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; Sprite pages. The console has 64 sprites; the extension a second page of 64 at $5300, which the
; game's sprite loop moves on to when the first is full, instead of stopping (the objects left
; over then flickered, the game drawing them in the other order every other frame).

; $CC6C (in place of the loop that hides the 64 sprites): both pages, their high bytes back to
; 0 (they are kept from one page to the next: the page and its high bytes are rebuilt together),
; back to the first page.
wide_clear_oam:
    LDX      #$00
@hide:
    LDA      #$00
    STA      WIDE_SPRHI,x
    STA      EXTRA_SPRHI,x
    LDA      #$F8
    STA      OAM,x
    STA      EXTRA_OAM,x
    INX
    BNE      @hide
    STX      WV_PAGE
    RTS

; Stores A at byte k of the sprite at offset X of the page in use (flags N and V change).
.macro PUT k
    BIT      WV_PAGE
    BMI      :+
    STA      OAM+k,x
    JMP      :++
:   STA      EXTRA_OAM+k,x
:
.endmacro

; The writes of the energy bars' sprites ($CFAD, $CFC9, $CFCE, $CFD3, in place of STA $020k,X).
wide_put_y:
    PUT      0
    RTS
wide_put_tile:
    PUT      1
    RTS
wide_put_attribute:
    PUT      2
    RTS
wide_put_x:
    PUT      3
    PHA                                 ; the bars are on the original screen: high byte 0
    LDA      #$00
    BIT      WV_PAGE
    BMI      @second
    STA      WIDE_SPRHI,x
    PLA
    RTS
@second:
    STA      EXTRA_SPRHI,x
    PLA
    RTS

; The sprite loop of one object ($CE9A-$CEF8, entered in place of LDX $06 / LDY $07): the
; original's, the sprites written to the page in use; carry set on return when both are full.
; $04 sprites, $07 index in the tile/attribute list ($08) and the position list ($0A), $00/$01
; screen x/y, $02 horizontal flip, $03 palette override, $06 OAM offset.
wide_draw_loop:
@sprite:
    LDX      $06
    LDY      $07
    LDA      ($08),y                    ; tile
    PUT      1
    CLC
    LDA      ($0A),y                    ; y
    ADC      $01
    PUT      0
    INY
    LDA      $03
    BEQ      @own_palette
    LDA      ($08),y
    AND      #$F0
    ORA      $03
    BNE      @attribute
@own_palette:
    LDA      ($08),y
@attribute:
    EOR      $02
    PUT      2
    LDA      $02
    BEQ      @offset
    LDA      ($0A),y                    ; flipped: the mirrored offset
    TAY
    LDA      $8600,y
    JMP      @x
@offset:
    LDA      ($0A),y
@x:
    JSR      wide_sprite_x
    BCS      @hidden
    PUT      3
    CLC
    TXA
    ADC      #$04
    STA      $06
    BEQ      @full
@next:
    INC      $07
    INC      $07
    DEC      $04
    BNE      @sprite
    CLC
    RTS
@hidden:
    LDA      #$F8
    PUT      0
    JMP      @next
@full:
    BIT      WV_PAGE                    ; the first page: on to the second
    BMI      @both
    LDA      #$80
    STA      WV_PAGE
    JMP      @next
@both:
    SEC
    RTS

; ---------------------------------------------------------------------------- ;
; Dormant enemies. An enemy of the stage list that spawns in a margin shows but does not move
; until it comes onto the original screen; from then on it lives as the original does, margins
; included.

; Spawn ($D763, in place of TYA / STA $0100,X; X the enemy slot - $10, Y its list entry, the
; stage bank mapped). Enemies that draw themselves into the background (Air Man's cloud devils)
; are never dormant: they draw as soon as they spawn, as the original does at its screen edge.
wide_spawned:
    TYA
    STA      $0100,x
    LDA      STAGE_ENEMY_TYPE,y
    LDY      #NEVER_DORMANT_COUNT - 1
@exempt:
    CMP      wide_never_dormant,y
    BEQ      @awake
    DEY
    BPL      @exempt
    LDA      #$01
    BNE      @set
@awake:
    LDA      #$00
@set:
    STA      WV_DORMANT+FIRST_ENEMY,x
    LDY      $0100,x                    ; Y back to the list entry
    RTS

wide_never_dormant:
.byt    $40, $41                        ; Air Tikki (cloud devil), both kinds
NEVER_DORMANT_COUNT = * - wide_never_dormant

; The enemy loop (bank 14, $9283, in place of LDY $0400,X; $2E/$2F the enemy's screen x): a
; dormant enemy off the original screen skips its behaviour (it is still removed past the
; margins); one on it wakes up.
wide_wake:
    LDA      WV_DORMANT,x
    BEQ      @behave
    LDA      $2F
    BNE      @asleep
    LDA      #$00
    STA      WV_DORMANT,x
@behave:
    LDY      $0400,x
    RTS
@asleep:
    JSR      wide_cull_screen
    BCC      @skip
    JSR      KILL
@skip:
    PLA                                 ; not back to the loop's dispatch: its next slot
    PLA
    JMP      NEXT_ENEMY

; ---------------------------------------------------------------------------- ;
; Air Man's cloud devils share two background palettes: a head of type $40 is drawn in palette 2,
; one of type $41 in palette 3. A marker of the stage list (type $42 or $43) sets its palette back
; to the first step of the fade-in (sky) when it reaches the screen, so that the next head of
; that type fades in; on the original screen the head before it is gone by then, in a wide view
; it is still in the left margin and would vanish with it. So the palette is reset only when no
; head of that type is alive; otherwise the next head finds it in colour and shows at once (the
; game's own test, $AE61).

; Bank 14, $AF9D, in place of JSR $AF4C (X the palette's offset in the buffer, $08 or $0C; Y 0).
wide_tikki_reset:
    TXA
    LSR
    LSR
    CLC
    ADC      #$3E                       ; $08 -> $40, $0C -> $41
    STA      WV_T0
    LDY      #LAST_SLOT
@slot:
    LDA      $0420,y
    BPL      @next
    LDA      $0400,y
    CMP      WV_T0
    BEQ      @kept
@next:
    DEY
    CPY      #FIRST_ENEMY - 1
    BNE      @slot
    LDY      #$00
    JMP      TIKKI_PALETTE
@kept:
    RTS

wide_end:
.assert wide_end <= $F900, error, "widescreen code past $F8FF"
```

### B. `tools/build.py`

```python
"""Builds the Mega Man 2 (USA) widescreen hack and its BPS patch.

Usage: python -I tools/build.py <Mega Man 2 (USA).nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $F300 of the
   fixed bank, into the unused $FF fill from $F2E3 to $F8FF.
2. Writes the hooks into the fixed bank, each checked against the bytes it replaces.
3. Writes build/mm2wide.nes and, if asked, the BPS patch (data without iNES header).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x0FCFC04D
BANK = 15
BANK_SIZE = 0x4000
FIXED = 0xC000
CODE_START = 0xF300
CODE_END = 0xF900

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    offset = target - (at + 2)
    assert -128 <= offset <= 127, (hex(at), hex(target))
    return bytes([opcode, offset & 0xFF])


BCS, BCC, BEQ = 0xB0, 0x90, 0xF0
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

    # Reset: unlock the extension first.
    put(0xFFE4, '4c d1 f2', jmp(sym['wide_reset']))
    # NMI: the extension's settings for the next frame.
    put(0xCFFD, 'a5 f7 29 7c 85 f7', jsr(sym['wide_nmi']))
    # Spawn window.
    put(0xD65F, '18 a5 1f 85 0a 69 ff 85 08 a5 20 85 0b 69 00 85 09',
        jsr(sym['wide_spawn_window']) + jmp(0xD670))
    # Objects moving left / right: the off-screen tests.
    put(0xEF43, '38 bd 60 04 e5 1f 85 08 bd 40 04 e5 20 d0 3a a5 08 c9 08 90 34 b0 30',
        jsr(sym['wide_cull_left']) + branch(BCS, 0xEF46, 0xEF8C) + branch(BCC, 0xEF48, 0xEF8A))
    put(0xEF75, '38 bd 60 04 e5 1f 85 08 bd 40 04 e5 20 d0 08 a5 08 c9 f8 b0 02',
        jsr(sym['wide_cull_right']) + branch(BCS, 0xEF78, 0xEF8C))
    # Object still on the screen?
    put(0xE4E9, '38 bd 60 04 e5 1f bd 40 04 e5 20 90 04 d0 02',
        jsr(sym['wide_cull_screen']) + branch(BCS, 0xE4EC, 0xE4FA))
    # Object that did not move sideways: removed off the screen.
    put(0xEFE8, 'a5 2f d0 a0 18 60', jmp(sym['wide_cull_still']))
    # Collision flags of an enemy, both object loops.
    put(0xEEBE, 'bd 20 04 29 03 f0 2a', jsr(sym['wide_hit_flags']) + branch(BEQ, 0xEEC1, 0xEEEF))
    put(0xEFB7, 'bd 20 04 29 03 f0 2a', jsr(sym['wide_hit_flags']) + branch(BEQ, 0xEFBA, 0xEFE8))
    # Sprites: the origin and each sprite's x.
    put(0xCE7D, '38 bd 60 04 e5 1f 85 00 bd 40 04 e5 20', jsr(sym['wide_draw_origin']))
    put(0xCECE, '18 30 06 65 00 90 0d b0 04 65 00 b0 07',
        jsr(sym['wide_sprite_x']) + branch(BCS, 0xCED1, 0xCEDB) + jmp(0xCEE2))
    # Dormant enemies: marked at spawn, woken in the enemy loop (bank 14).
    put(0xD763, '98 9d 00 01', jsr(sym['wide_spawned']))
    banked.append((14, 0x9283, bytes.fromhex('bc 00 04'), jsr(sym['wide_wake'])))
    # Cloud devils: a marker resets its palette only once no head of its type is alive.
    banked.append((14, 0xAF9D, bytes.fromhex('20 4c af'), jsr(sym['wide_tikki_reset'])))
    # Sprite pages: both hidden, the loop of one object, the energy bars' writes.
    put(0xCC6C, 'a9 f8 a2 00 9d 00 02 e8 d0 fa 60', jmp(sym['wide_clear_oam']))
    put(0xCE9A, 'a6 06 a4 07', jmp(sym['wide_draw_loop']))
    put(0xCFAD, '9d 00 02', jsr(sym['wide_put_y']))
    put(0xCFC9, '9d 01 02', jsr(sym['wide_put_tile']))
    put(0xCFCE, '9d 02 02', jsr(sym['wide_put_attribute']))
    put(0xCFD3, '9d 03 02', jsr(sym['wide_put_x']))
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
    assert zlib.crc32(data) == SOURCE_CRC, 'not Mega Man 2 (USA)'

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

    out = os.path.join(ROOT, 'build', 'mm2wide.nes')
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
