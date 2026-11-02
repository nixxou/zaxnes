; FlappyZaxNes: the widescreen hack (Zaxnes), a hack pack.
;
; The game is left as it is: this code runs in the zone fantôme ($2000, widescreen MMC5: the
; extension's registers and RAM at $3000-$3FFF for these instructions), reached through two
; hooks Zaxnes writes into the game (widescreen/build.py). The game keeps both screens of its
; background in the nametables (vertical mirroring, a 512-pixel ring) and draws a new pipe 352
; pixels ahead of its scroll, so the margins show its own world from the nametables, no decoder:
;
;  * the margin asked for is the host's, at most 80 pixels (a new pipe is drawn past 344);
;  * the clouds, sprites, show in the margins: the game hides a cloud column off the screen,
;    the hack gives it the high byte of its x instead when it falls within the wide view;
;  * so do the caps of the moving pipes (from the tenth pipe on, one in two; their bodies are
;    background, their caps sprites), part of the base: without them the margins would show
;    them capless.
;
; With a margin of 0 every routine does what the code it replaces does, except that a cloud
; column across the left edge shows its part on the screen (the extension draws it).
;
; The game's symbols (TEMP0 to TEMP3) are given on the command line from its labels.

EXT             = $3000         ; the extension's registers and RAM, in the zone fantôme
WIDE_HOST       = EXT + $002    ; R: the margin the host offers
WIDE_REQ        = EXT + $003    ; W: the margin asked for
WIDE_SPRHI      = EXT + $100    ; + 4 n: high byte of the x of sprite n, for the next OAM DMA
WV_M            = EXT + $200    ; the margin of this frame (extension RAM)

MAX_MARGIN      = 80
SPRITE_BUFFER   = $0200
OAMDMA          = $4014

.segment "CODE"
.org $2000

; ---------------------------------------------------------------------------- ;
; NMI (in place of LDA #>SPRITE_BUFFER / STA OAMDMA): the margin, then the sprites' DMA.
wide_nmi:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    LDA     #>SPRITE_BUFFER
    STA     OAMDMA
    RTS

; ---------------------------------------------------------------------------- ;
; A cloud column off the screen (in place of the game's hiding of its two sprites): the game
; has its x, 256 to 511 on the ring, in TEMP1:TEMP0, its y in TEMP2, and Y its first sprite's
; offset in the sprite page. Within the wide view the column shows, the high byte of its x 1
; (right margin) or $FF (left of the screen); otherwise it is hidden as the game does.
wide_cloud_column:
    LDA     TEMP0
    CMP     WV_M
    BCC     @right                      ; 256 + low < 256 + M
    CLC                                 ; left: x - 512 + 8 > -M, low + 8 + M > 256
    ADC     #$08
    BCS     @left
    ADC     WV_M
    BCC     @hide
    BEQ     @hide                       ; exactly 256: past the view
@left:
    LDA     #$FF
    BNE     @show
@right:
    LDA     #$01
@show:
    STA     WIDE_SPRHI,y
    STA     WIDE_SPRHI+4,y
    LDA     TEMP2
    STA     SPRITE_BUFFER,y
    CLC
    ADC     #$08
    STA     SPRITE_BUFFER+4,y
    LDA     TEMP0
    STA     SPRITE_BUFFER+3,y
    STA     SPRITE_BUFFER+7,y
    RTS
@hide:
    LDA     #$FF
    STA     SPRITE_BUFFER,y
    STA     SPRITE_BUFFER+4,y
    RTS

; ---------------------------------------------------------------------------- ;
; A column of a moving pipe's caps off the screen (in place of the game's hiding of its two
; sprites, at MOVER_OAM + Y, the three groups one after the other): its x in TEMP1:TEMP0 as a cloud's, the y of the upper pipe's cap in
; TEMP2, of the lower pipe's in TEMP3.
MOVER_OAM       = SPRITE_BUFFER + 9 * 4

wide_mover_column:
    LDA     TEMP0
    CMP     WV_M
    BCC     @right
    CLC
    ADC     #$08
    BCS     @left
    ADC     WV_M
    BCC     @hide
    BEQ     @hide
@left:
    LDA     #$FF
    BNE     @show
@right:
    LDA     #$01
@show:
    STA     WIDE_SPRHI+9*4,y
    STA     WIDE_SPRHI+9*4+4,y
    LDA     TEMP2
    STA     MOVER_OAM,y
    LDA     TEMP3
    STA     MOVER_OAM+4,y
    LDA     TEMP0
    STA     MOVER_OAM+3,y
    STA     MOVER_OAM+7,y
    RTS
@hide:
    LDA     #$FF
    STA     MOVER_OAM,y
    STA     MOVER_OAM+4,y
    RTS

wide_end:
.assert wide_end <= $3000, error, "widescreen code past the zone's code half"
