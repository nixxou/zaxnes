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

.import Setup, MainLoop, CreateBackground, NMI

.segment "HEADER"
	.byte "NES"	;identification string
	.byte $1A
	.byte $02	  ;amount of PRG ROM in 16K units
	.byte $01	  ;amount of CHR ROM in 8K units
	.byte $01	  ;mapper and mirroing (vertical: two screens side by side)
	.byte $00, $00, $00, $00
	.byte $00, $00, $00, $00, $00

.segment "ZEROPAGE"
.segment "STARTUP"
.segment "CODE"
reset:
	sei	 ;disables interupts
	cld	 ;turn off decimal mode
	
	ldx #%01000000	  ;disable sound IRQ
	stx $4017
	ldx #$00
	stx $4010	   ;disable PCM
	
	;initialize the stack register
	ldx #$ff
	txs	 ;transfer x to the stack
	
	; Clear PPU registers
	ldx #$00
	stx PPUCTRL
	stx PPUMASK
	
	;WAIT FOR VBLANK
:
	bit PPUSTATUS
	bpl :-
	
	;CLEARING 2K MEMORY
	txa

	ldx #$00
clear_memory:
	sta $0000, x
	sta $0100, x
	; sta $0200, x
	sta $0300, x
	sta $0400, x
	sta $0500, x
	sta $0600, x
	sta $0700, x

	; Limpa a memória da OAM para não ficar um sprite aleatório na borda
	lda #$FF
	sta $0200, X
	lda #$00

	inx
	bne clear_memory

	;WAIT FOR VBLANK
:
	bit PPUSTATUS
	bpl :-

	jsr Setup
	
;ENABLE INTERUPTS
	cli
	
	lda #%10010000
	sta PPUCTRL	 ;WHEN VBLANK OCCURS CALL NMI

infloop:
	jsr MainLoop
	jmp infloop

nmi:
	; 1. SAVE REGISTERS (The "Context Save")
	pha	 ; Push A
	txa
	pha	 ; Push X
	tya
	pha	 ; Push Y

	; 2. OAM DMA (Graphics Update)
	lda #>SPRITE_BUFFER	; High byte of $0200
	sta OAMDMA

	lda #%10010000  ; Re-enable NMI, select BG table
	sta PPUCTRL
	
	jsr NMI

	; 4. RESTORE REGISTERS
	pla	 ; Pull Y
	tay
	pla	 ; Pull X
	tax
	pla	 ; Pull A
	
	rti

irq:
	rti

.segment "VECTORS"
	.word nmi
	.word reset
	.word irq

.segment "CHARS"
	.incbin "char.chr"
