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

.export LoadPalletes

.proc LoadPalletes
	lda #$3f	;$3F00
	sta PPUADDR
	lda #$00
	sta PPUADDR
	
	ldx #$00
loadpalettes:
	lda palettedata, x
	sta PPUDATA
	inx
	cpx #$20
	bne loadpalettes

	rts
.endproc

.segment "RODATA"
palettedata:
	.byte $31, $30, $31, $21,	 $31, $09, $1A, $29,	 $31, $09, $1A, $21,	 $31, $09, $2A, $27 ; Background: sky, pipes, bushes, ground (tools/decor.py)
	.byte $31, $07, $11, $20,	 $31, $09, $1A, $29,	 $31, $30, $3D, $0F,	 $31, $07, $16, $27