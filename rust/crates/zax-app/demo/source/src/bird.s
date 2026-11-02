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

.include "sprite.inc"
.include "controller.inc"
.include "ppu.inc"

.include "pipes.inc"
.include "bird.inc"

.import Bird, SpriteData, CreateTimer, TimerRunning, TitleScreenSetup
.importzp Tick, ControllerPressed, ScrollX, ScrollPage, GameStopped, Scene
.import PipeTops, PipeGaps, PipeCols
.importzp Temp0, Temp1
.export UpdateBird, ResetBird

; Quanto menor, mais difícil
BIRD_JUMP_SPEED = $EE
; Velocidades a partir das quais o pássaro se inclina
BIRD_TILT_UP = $F6
BIRD_TILT_DOWN = $18

.segment "ZEROPAGE"
BirdY: .byte $00, $00
BirdYint: .res 1 ; Valor inteiro, e não decimal
BirdYSpeed: .byte $00
; 0 nivelado, 1 para cima, 2 para baixo
BirdPose: .res 1
Temp: .res 1
TempHi: .res 1
.segment "RODATA"
BirdTile0 = SPRITE_ADDR Bird, 0, Sprite::sprite_num
.include "bird_frames.inc"
.segment "CODE"
; Converte BirdY em ponto fixo 12.4 para número inteiro e coloca em BirdYint
.macro get_integer_y_position
	lda BirdY
	sta $00
	lda BirdY + 1
	sta $01
	lsr $01
	ror $00
	lsr $01
	ror $00
	lsr $01
	ror $00
	lsr $01
	ror $00
	lda $00

	sta BirdYint
.endmacro

.macro draw_bird
	.local @row
	; As três colunas de cima na posição Y, as de baixo 8 abaixo
	lda BirdYint
	sta SPRITE_ADDR Bird, 0, Sprite::pos_y
	sta SPRITE_ADDR Bird, 1, Sprite::pos_y
	sta SPRITE_ADDR Bird, 2, Sprite::pos_y
	clc
	adc #8
	sta SPRITE_ADDR Bird, 3, Sprite::pos_y
	sta SPRITE_ADDR Bird, 4, Sprite::pos_y
	sta SPRITE_ADDR Bird, 5, Sprite::pos_y

	; O bico, no lugar da pose atual
	ldx BirdPose
	lda BirdYint
	clc
	adc BeakDY, x
	sta SPRITE_ADDR Bird, 6, Sprite::pos_y
	lda #BIRD_X
	clc
	adc BeakDX, x
	sta SPRITE_ADDR Bird, 6, Sprite::pos_x
	lda BeakTile, x
	sta SPRITE_ADDR Bird, 6, Sprite::sprite_num
.endmacro

.macro animate_bird
	.local @level, @wings, @copy, @end
	lda GameStopped
	bne @end

	; Inclinação pela velocidade: 0 nivelado, 1 bico para cima, 2 para baixo
	ldx #$00
	lda BirdYSpeed
	bpl @level
	cmp #BIRD_TILT_UP
	bcs @wings
	ldx #$01
	bne @wings
@level:
	cmp #BIRD_TILT_DOWN
	bcc @wings
	ldx #$02
@wings:
	stx BirdPose
	; Quadro = pose * 3 + asas (0-2, pelos últimos bits do Tick)
	txa
	asl a
	adc BirdPose
	tax
	lda Tick
	and #%00011111
	cmp #$0A
	bcc :+
	inx
	cmp #$14
	bcc :+
	inx
:
	; 6 tiles por quadro
	txa
	asl a
	sta Temp
	asl a
	adc Temp
	tax
	ldy #$00
@copy:
	lda BirdFrames, x
	sta BirdTile0, y
	inx
	iny
	iny
	iny
	iny
	cpy #6 * 4
	bne @copy
@end:
.endmacro

.macro bird_movement
	.local @add_pos, @subtract_pos, @done, @jump, @end
	; Não roda a função se o jogo estiver parado
	lda GameStopped
	bne @end

	; Adiciona a velocidade à posição Y (adição de número de 16 bits com de 8)
	lda BirdYSpeed
	bmi @subtract_pos

@add_pos:
	lda BirdY
	clc
	adc BirdYSpeed
	sta BirdY

	lda BirdY+1
	adc #$00
	sta BirdY+1
	jmp @done

@subtract_pos:
	; Isso é uma gambiarra que funcionou e é ainda mais otimizado.
	lda BirdY
	clc
	adc BirdYSpeed ; Adc porque tem que ser o inverso de BirdYSpeed
	sta BirdY

	lda BirdY+1
	sbc #$00
	sta BirdY+1

@done:
	; Um só impulso por pressão do botão
	lda ControllerPressed
	and #BUTTON_A
	bne @jump

	; Pássaro cai -- se a velocidade for 127, não aumenta e para
	lda BirdYSpeed
	cmp #$7F
	beq @end
	inc BirdYSpeed

	jmp @end
@jump:
	lda #BIRD_JUMP_SPEED ; Velocidade do pássaro
	sta BirdYSpeed
@end:
	get_integer_y_position
.endmacro

.macro check_collisions
	.local @check_floor_colision, @bird_killed, @end
	; Não roda a função se o jogo estiver parado
	lda GameStopped
	bne @end

	; Verifica a colisão com os canos: para cada cano em jogo, a distância
	; (pássaro + BIRD_WIDTH - cano) no anel de 512 pixels deve ficar abaixo de
	; largura do cano + BIRD_WIDTH para estar na altura dele.
	ldx #$03
@next_pipe:
	lda PipeCols, x
	cmp #$FF
	beq @skip_pipe
	; Temp0/1 = coluna * 8
	sta Temp0
	lda #$00
	sta Temp1
	asl Temp0
	rol Temp1
	asl Temp0
	rol Temp1
	asl Temp0
	rol Temp1
	; (tela:ScrollX) + BIRD_X + BIRD_WIDTH - cano
	lda ScrollX
	clc
	adc #BIRD_X + BIRD_WIDTH
	sta Temp
	lda ScrollPage
	adc #$00
	sta TempHi
	lda Temp
	sec
	sbc Temp0
	sta Temp
	lda TempHi
	sbc Temp1
	and #$01
	bne @skip_pipe
	lda Temp
	cmp #TILE_SIZE*PIPES_WIDTH+BIRD_WIDTH
	bcs @skip_pipe

	; a borda de baixo do cano de cima (game_scene.s: PipeTops), em pixels
	lda PipeTops, x
	sec
	sbc #BIRD_Y_HITBOX_GAP

	cmp BirdYint
	bcs @bird_killed ; Se BirdY for menor que isso, colidiu

	; o chapéu do cano de baixo, a abertura (PipeGaps) mais abaixo
	lda PipeTops, x
	clc
	adc PipeGaps, x
	sec
	sbc #BIRD_HEIGHT
	cmp BirdYint
	bcc @bird_killed
@skip_pipe:
	dex
	bpl @next_pipe

@check_floor_colision:
	; Vê se está colidindo com o chão. O outro valor é necessário pois, quando o pássaro fica
	; muito alto, sua posição fica na parte debaixo da tela.
	lda BirdYint
	cmp #$FF-TILE_SIZE*6-BIRD_HEIGHT
	bcc @end

	cmp #$FF-TILE_SIZE*6
	bcs @end

@bird_killed:
	lda #$01
	sta GameStopped
	
	; Timer de 1.5 segundos (pelo menos em NTSC)
	lda #90
	jsr CreateTimer

@end:
.endmacro

.proc UpdateBird
	bird_movement
	animate_bird
	draw_bird

	jsr TimerRunning
	beq blink_bird

	; Isso rodará somente se o timer não estiver rodando -- ou seja: se ele acabou
	lda GameStopped
	bne switch_to_new_scene

	.ifndef TEST_NO_COLLISION
	check_collisions
	.endif
	jmp end

	; Vai pra cena do título
switch_to_new_scene:
	lda #$00
	sta Scene
	jsr TitleScreenSetup

	rts

blink_bird:
	lda Tick
	and #%00001000
	beq transparent_bird
	jmp end

transparent_bird:
	lda #$FF
	sta SPRITE_ADDR Bird, 0, Sprite::pos_y
	sta SPRITE_ADDR Bird, 1, Sprite::pos_y
	sta SPRITE_ADDR Bird, 2, Sprite::pos_y
	sta SPRITE_ADDR Bird, 3, Sprite::pos_y
	sta SPRITE_ADDR Bird, 4, Sprite::pos_y
	sta SPRITE_ADDR Bird, 5, Sprite::pos_y
	sta SPRITE_ADDR Bird, 6, Sprite::pos_y

end:
	rts
.endproc

.proc ResetBird
	lda #$00
	sta BirdY
	sta BirdY+1
	
	sta BirdYSpeed
	sta BirdYint
	sta BirdPose

	rts
.endproc
