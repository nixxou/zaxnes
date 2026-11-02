;	Copyright (C) 2026 Nixxou
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

; As nuvens são sprites atrás do fundo: inteiras atrás dos canos, também em 3D.
.export UpdateClouds
.importzp ScrollX, ScrollPage, Temp0, Temp1, Temp2, Temp3

.segment "RODATA"
.include "clouds_data.inc"

.segment "CODE"
; Põe cada coluna de cada nuvem na sua posição na tela, ou a esconde quando está fora.
.proc UpdateClouds
	ldx #$00
cloud:
	; Temp0/1 = (x da nuvem - scroll) no anel de 512 pixels
	lda CloudXLo, x
	sec
	sbc ScrollX
	sta Temp0
	lda CloudXHi, x
	sbc ScrollPage
	and #$01
	sta Temp1
	lda CloudY, x
	sta Temp2
	lda CloudCols, x
	sta Temp3       ; colunas que faltam
	ldy CloudOam, x
column:
	lda Temp1
	bne hide        ; 256 ou mais: fora da tela
	lda Temp2
	sta SPRITE_BUFFER, y
	clc
	adc #$08
	sta SPRITE_BUFFER + 4, y
	lda Temp0
	sta SPRITE_BUFFER + 3, y
	sta SPRITE_BUFFER + 7, y
	jmp next
hide:
	lda #$FF
	sta SPRITE_BUFFER, y
	sta SPRITE_BUFFER + 4, y
next:
	; a coluna seguinte, 8 pixels à direita
	lda Temp0
	clc
	adc #$08
	sta Temp0
	lda Temp1
	adc #$00
	and #$01
	sta Temp1
	tya
	clc
	adc #8
	tay
	dec Temp3
	bne column

	inx
	cpx #CLOUD_COUNT
	bne cloud
	rts
.endproc
