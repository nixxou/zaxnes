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
