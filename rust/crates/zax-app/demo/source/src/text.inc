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

.IFNDEF TEXT_INC
TEXT_INC=1

.include "ppu.inc"

.macro DRAW_TEXT pos_x, pos_y, text
	.local ADDRESS
	ADDRESS = pos_y*32 + pos_x + $2000
	lda #>ADDRESS
	sta PPUADDR
	lda #<ADDRESS
	sta PPUADDR

.repeat .strlen(text), i
	lda #.strat(text, i)
	sta PPUDATA
.endrepeat
.endmacro

.macro DRAW_TEXT_CENTERED pos_y, text
	DRAW_TEXT ($20-.strlen(text))/2, pos_y, text
.endmacro

.ENDIF