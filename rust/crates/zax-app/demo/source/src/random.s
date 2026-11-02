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

; Retirado de https://www.nesdev.org/wiki/Random_number_generator

; prng
;
; Returns a random 8-bit number in A (0-255), clobbers Y (0).
;
; Requires a 2-byte value on the zero page called "seed".
; Initialize seed to any value except 0 before the first call to prng.
; (A seed value of 0 will cause prng to always return 0.)
;
; This is a 16-bit Galois linear feedback shift register with polynomial $0039.
; The sequence of numbers it generates will repeat after 65535 calls.
;
; Execution time is an average of 125 cycles (excluding jsr and rts)

.export RNGSeed, PRNG

.zeropage
RNGSeed: .res 2       ; initialize 16-bit seed to any value except 0

.code
; Returns a random 8-bit number in A (0-255), clobbers Y (unknown).
PRNG:
	lda RNGSeed+1
	tay ; store copy of high byte
	; compute seed+1 ($39>>1 = %11100)
	lsr ; shift to consume zeroes on left...
	lsr
	lsr
	sta RNGSeed+1 ; now recreate the remaining bits in reverse order... %111
	lsr
	eor RNGSeed+1
	lsr
	eor RNGSeed+1
	eor RNGSeed+0 ; recombine with original low byte
	sta RNGSeed+1
	; compute seed+0 ($39 = %111001)
	tya ; original high byte
	sta RNGSeed+0
	asl
	eor RNGSeed+0
	asl
	eor RNGSeed+0
	asl
	asl
	asl
	eor RNGSeed+0
	sta RNGSeed+0
	rts