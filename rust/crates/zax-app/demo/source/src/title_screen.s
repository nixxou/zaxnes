;	Copyright (C) 2025 filipemd
;	
;	This file is part of Flappy NES.
;
;	Flappy NES is free software: you can redistribute it and/or modify
;	it under the terms of the GNU General Public License as published by
;	the Free Software Foundation, either version 3 of the License, or
;	(at your option) any later version.
;
;	Flappy NES is distributed in the hope that it will be useful,
;	but WITHOUT ANY WARRANTY; without even the implied warranty of
;	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
;	GNU General Public License for more details.
;
;	You should have received a copy of the GNU General Public License
;	along with Flappy NES.  If not, see <http://www.gnu.org/licenses/>.

.include "ppu.inc"
.include "controller.inc"
.include "sprite.inc"

.include "text.inc"

.export TitleScreenSetup, TitleScreenLoop, TitleScreenNMI

.import CreatePipe, CreateBackground, SpriteData, GameSceneSetup
.importzp ScrollX, ScrollPage, Scene, ControllerButtons, Temp0, Temp1, Temp2

; Coloca todos os sprites na posição Y $FF para não serem mais visíveis
.macro clear_sprites
	lda #$FF
.repeat SPRITE_AMOUNT, i 
	sta SPRITE_ADDR (SpriteData, i, Sprite::pos_y)
.endrepeat
.endmacro

.macro fill_pallete
	.local @fill_pallete_loop

	lda #$23
	sta PPUADDR
	lda #$C0
	sta PPUADDR
	ldx #$00
@fill_pallete_loop:
	lda #%01010101 ; Paleta 01 => Preta, cinza e branca -- em todos os tiles
	sta PPUDATA
	inx
	cpx #$30 ; Ao invés de terminar em $40, terminar em $30 pra não interferir no chão
	bne @fill_pallete_loop
.endmacro

.macro draw_title
	.local LOGO_WIDTH, LOGO_HEIGHT, LOGO_X, LOGO_Y, TOPLEFT_CORNER_TILE, START_ADDRESS
	.local @draw_row, @end
	; Temp0 = bit menor do endereço dos tiles
	; Temp1 = bit maiot do endereço dos tiles
	; Temp2 = id do tile que será desenhado

	LOGO_WIDTH=16
	LOGO_HEIGHT=3
	LOGO_X=8
	LOGO_Y=2

	TOPLEFT_CORNER_TILE=$D0

	START_ADDRESS = LOGO_Y*32 + LOGO_X + $2000
	lda PPUSTATUS ; Reseta o valor

	lda #>START_ADDRESS
	sta Temp1
	sta PPUADDR
	lda #<START_ADDRESS
	sta Temp0
	sta PPUADDR

	ldx #$00
	ldy #$03

	lda #TOPLEFT_CORNER_TILE
	sta Temp2
@draw_row:
	cpx #LOGO_WIDTH
	bcs @end

	lda Temp2
	sta PPUDATA

	inx
	inc Temp2
	jmp @draw_row
@end:
	ldx #$00

	; Adiciona o valor do id do tile para a próxima coluna
	lda Temp2
	clc
	adc #$10
	; Subtrai o valor de X que estava anteriormente
	sec
	sbc #LOGO_WIDTH
	sta Temp2

	; Soma de 16 bits com o enderço atual ao número 32
	lda Temp0
	clc
	adc #$20 ; 32
	sta Temp0

	lda Temp1
	adc #$00
	sta Temp1

	lda Temp1
	sta PPUADDR
	lda Temp0
	sta PPUADDR

	dey
	bne @draw_row
.endmacro

.proc TitleScreenSetup
	clear_sprites

	lda #$00
	sta ScrollX
	sta ScrollPage

	; Desativa a renderização
	lda #%00000000
	sta $2001 

	; 2. Carrega os dados
	jsr CreateBackground
	; Um cano no lugar dele, como no jogo
	lda #28
	jsr CreatePipe
	draw_title
	DRAW_TEXT_CENTERED $0A, "Press START..."
	DRAW_TEXT $06, $0C, "© 2026 Nixxou"
	DRAW_TEXT $06, $0D, "© 2025 filipemd"
	; o projeto original (a URL não cabe numa linha antes do cano, coluna 28)
	DRAW_TEXT $06, $0E, "(original project :"
	DRAW_TEXT $06, $0F, "github.com/filipemd/"
	DRAW_TEXT $06, $10, "flappy-nes )"

	; Renderização de volta
	lda #%00011110
	sta $2001

	rts
.endproc

.proc TitleScreenLoop
	lda ControllerButtons
	and #BUTTON_START
	bne start_game
	rts

start_game:
	lda #$01
	sta Scene
	jsr GameSceneSetup
	rts
.endproc

.proc TitleScreenNMI
	rts
.endproc