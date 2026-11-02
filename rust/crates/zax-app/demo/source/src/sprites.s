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
.include "sprite.inc"
.include "bird.inc"

.export LoadSprites, SpriteData, Bird, Score

.proc LoadSprites
;LOADING SPRITES
	ldx #$00
loadsprites:
	lda SpriteData, x
	sta SPRITE_BUFFER, x
	inx
	cpx #(.sizeof(Sprite) * SPRITE_AMOUNT)
	bne loadsprites

	rts
.endproc

.segment "RODATA"
SpriteData:
	;Y, SPRITE NUM, attributes, X
	;76543210
	;||||||||
	;||||||++- Palette (4 to 7) of sprite
	;|||+++--- Unimplemented
	;||+------ Priority (0: in front of background; 1: behind background)
	;|+------- Flip sprite horizontally
	;+-------- Flip sprite vertically

Bird:
	; FF pra ficar invisível e a posição só ser definida no loop
	.byte $FF, $30, %00000000, BIRD_X
	.byte $FF, $31, %00000000, BIRD_X+8
	.byte $FF, $32, %00000000, BIRD_X+16
	.byte $FF, $33, %00000000, BIRD_X
	.byte $FF, $34, %00000000, BIRD_X+8
	.byte $FF, $35, %00000000, BIRD_X+16
	; O bico (paleta 3), colocado pela pose
	.byte $FF, $68, %00000011, BIRD_X

Score:
	; paleta 2 (a das nuvens: os algarismos só usam a cor 3, preta)
	.byte $10, $09, %00000010, $FF/2-8
	.byte $10, $09, %00000010, $FF/2

Mover:
	; Os chapéus dos canos que se movem (src/mover.s), três grupos, paleta 1 (a dos canos),
	; coluna por coluna: o de baixo do cano de cima ($28-$2B), o de cima do cano de baixo ($24-$27)
.repeat 12, i
	.byte $FF, $28 + (i .mod 4), %00000001, $00
	.byte $FF, $24 + (i .mod 4), %00000001, $00
.endrepeat

.include "clouds_sprites.inc"
Clouds:
	CLOUD_SPRITES
