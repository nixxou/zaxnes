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

.IFNDEF SPRITE_INC
SPRITE_INC = 1

.define SPRITE_AMOUNT 63
SPRITE_BUFFER = $0200

.struct Sprite
	pos_y .byte
	sprite_num .byte
	attributes .byte
	pos_x .byte
.endstruct

.define SPRITE_ADDR (spriteName, index, field) SPRITE_BUFFER + (spriteName - SpriteData) + (index * .sizeof(Sprite)) + field

.ENDIF