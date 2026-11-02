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
.import SpriteData, Score

.export ResetScore, IncScore

LOW_NUM = SPRITE_ADDR (Score, 1, Sprite::sprite_num)
HIGH_NUM = SPRITE_ADDR (Score, 0, Sprite::sprite_num)

.proc ResetScore
	lda #$00
	sta HIGH_NUM
	sta LOW_NUM

	rts
.endproc

.proc IncScore
	inc LOW_NUM
	lda LOW_NUM
	; >= 10
	cmp #10
	bcs inc_high
	rts

inc_high:
	inc HIGH_NUM
	lda #$00
	sta LOW_NUM
	rts
.endproc