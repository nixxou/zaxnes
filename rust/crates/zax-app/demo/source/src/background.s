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

.export CreateBackground, BackgroundData
.importzp Temp0, Temp1, Temp2, Temp3

.segment "RODATA"
; Duas telas de 960 bytes de tiles e 64 de atributos, escritas por tools/decor.py
BackgroundData:
	.incbin "background.bin"

.segment "CODE"
.proc CreateBackground
	; As duas telas: $2000 e $2400
	lda #<BackgroundData
	sta Temp0
	lda #>BackgroundData
	sta Temp1
	lda #$20
	jsr create_screen
	lda #$24
	jsr create_screen

	; Reseta o scroll
	lda PPUSTATUS
	lda #$00
	sta PPUSCROLL
	sta PPUSCROLL
	rts
.endproc

; A: byte alto do endereço da tela. Copia os 1024 bytes seguintes de BackgroundData (Temp0/1).
.proc create_screen
	ldx PPUSTATUS
	sta PPUADDR
	lda #$00
	sta PPUADDR

	ldx #$04
	ldy #$00
copy:
	lda (Temp0), y
	sta PPUDATA
	iny
	bne copy
	inc Temp1
	dex
	bne copy
	rts
.endproc
