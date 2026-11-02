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


; Os canos que se movem: a partir do MOVER_FROM-ésimo cano, um cano em dois (ao acaso, às vezes
; vários seguidos) tem a abertura (maior, MOVER_GAP pixels) subindo e descendo devagar, pixel
; por pixel. Os corpos dos dois canos são o fundo, até a linha de tiles mais próxima por baixo
; dos chapéus; os chapéus são sprites (oito, duas linhas de quatro) que cobrem o fim dos corpos:
; o cano anda liso, e o fundo só muda quando a borda passa de uma linha de tiles para outra
; (nove linhas de quatro tiles, escritas pelo NMI).
;
; No máximo três canos estão em jogo (o quarto, que sai pela esquerda, é restaurado quando um
; novo aparece), e os três se veem juntos com as margens de um modo widescreen: três grupos de
; sprites. O grupo g anda nas imagens em que MoverTick vale g: um só reescreve o fundo por vez.
;
; Top (G) é o y da borda de baixo do cano de cima: o chapéu de cima ocupa G-8 a G-1, o de
; baixo G+MOVER_GAP a G+MOVER_GAP+7. Com q = G/8: o corpo de cima são as linhas 0 a q-1, o de
; baixo começa na linha q+6, q+7 se G não é múltiplo de 8 (bottom_row).

.include "ppu.inc"
.include "sprite.inc"
.include "pipes.inc"

.export MoverReset, MoverSpawn, MoverEnd, MoveMover, UpdateMover, DrawMoverColumn, MoverFlush
.import PRNG, ColumnTarget, ColumnBuffer, PipeTops
.importzp ScrollX, ScrollPage, Scene, Temp0, Temp1, Temp2, Temp3

MOVER_FROM = 10
MOVER_GROUPS = 3
MOVER_TOP_MIN = 32      ; como o cano normal mais alto
MOVER_TOP_MAX = 96      ; o chapéu de baixo na linha 18, como o do cano normal mais baixo
MOVER_OAM = 9 * 4       ; depois do pássaro e do placar, 32 bytes por grupo
MOVER_ROWS = 9          ; as linhas reescritas: q-1 a q+7

PIPE_BODY = $B0

.segment "BSS2" ; RAM interna ($0400): o NROM não tem RAM em $6000
MoverCount: .res 1      ; os canos até MOVER_FROM
MoverTick: .res 1       ; o grupo que anda nesta imagem
NewGroup: .res 1        ; o grupo do cano novo
; por grupo
Slot: .res MOVER_GROUPS ; o lugar do cano em PipeTops, $FF: grupo livre
Col: .res MOVER_GROUPS  ; a coluna do anel
Top: .res MOVER_GROUPS
Dir: .res MOVER_GROUPS  ; +1 ou -1
Rows: .res MOVER_GROUPS ; q e a linha do corpo de baixo, para ver quando o fundo muda
Bottom: .res MOVER_GROUPS
; as linhas para o NMI
MoverDirty: .res 1
MoverAddrHi: .res MOVER_ROWS
MoverAddrLo: .res MOVER_ROWS
MoverTile: .res MOVER_ROWS  ; o primeiro tile da linha (o corpo) ou 0 (a abertura)

.segment "CODE"

; No começo de um jogo: nenhum cano se move, o primeiro cano conta.
.proc MoverReset
	lda #$FF
	ldx #MOVER_GROUPS - 1
:
	sta Slot, x
	dex
	bpl :-
	lda #$01
	sta MoverCount
	lda #$00
	sta MoverTick
	sta MoverDirty
	rts
.endproc

; Um cano novo em A (sua coluna) e X (seu lugar em PipeTops): o carry diz se ele se move.
.proc MoverSpawn
	sta Temp0
	stx Temp1
	lda MoverCount
	cmp #MOVER_FROM
	bcs :+
	inc MoverCount
	lda MoverCount
	cmp #MOVER_FROM
	bcc no
:
	jsr PRNG
	and #$01
	beq no
	; um grupo livre (sempre há um: o cano que sai já foi liberado)
	ldx #MOVER_GROUPS - 1
:
	lda Slot, x
	bmi found
	dex
	bpl :-
no:
	clc
	rts
found:
	stx NewGroup
	lda Temp1
	sta Slot, x
	lda Temp0
	sta Col, x
	; a altura e o sentido iniciais ao acaso
	jsr PRNG
	and #%00111111          ; 0-63
	clc
	adc #MOVER_TOP_MIN
	ldx NewGroup
	sta Top, x
	jsr PRNG
	ldx NewGroup
	and #$01
	bne :+
	lda #$FF
:
	sta Dir, x
	jsr bottom_row
	sta Bottom, x
	lda Top, x
	lsr a
	lsr a
	lsr a
	sta Rows, x
	ldy Slot, x
	lda Top, x
	sta PipeTops, y
	sec
	rts
.endproc

; A: a coluna do cano que sai pela esquerda (que vai ser restaurada): se ele se move, para.
.proc MoverEnd
	ldx #MOVER_GROUPS - 1
loop:
	ldy Slot, x
	bmi next
	cmp Col, x
	bne next
	pha
	lda #$FF
	sta Slot, x
	pla
next:
	dex
	bpl loop
	rts
.endproc

; X: o grupo. A linha onde começa o corpo de baixo.
.proc bottom_row
	lda Top, x
	and #$07
	cmp #$01                ; carry: G não é múltiplo de 8, uma linha a mais
	php
	lda Top, x
	lsr a
	lsr a
	lsr a
	plp
	adc #MOVER_GAP / 8
	rts
.endproc

; A: coluna do anel, X: coluna do cano (0-3). Como DrawPipeColumn, para o cano novo que se move:
; o corpo de cima, a abertura (o céu), o corpo de baixo; os chapéus são sprites.
.proc DrawMoverColumn
	sta ColumnTarget
	stx Temp2
	ldx NewGroup
	jsr bottom_row
	sta Temp1
	lda Top, x
	lsr a
	lsr a
	lsr a
	sta Temp0
	ldy #$00
row:
	lda #$00
	cpy Temp0
	bcc body
	cpy Temp1
	bcc write
body:
	lda Temp2
	clc
	adc #PIPE_BODY
write:
	sta ColumnBuffer, y
	iny
	cpy #PIPES_HEIGHT
	bne row
	rts
.endproc

; Uma vez por pixel de scroll (fora das fases em que o NMI escreve colunas): um grupo anda um
; pixel, cada um a cada três imagens.
.proc MoveMover
	ldx MoverTick
	inx
	cpx #MOVER_GROUPS
	bcc :+
	ldx #$00
:
	stx MoverTick
	lda Slot, x
	bmi done
	; anda, e volta nos limites
	lda Top, x
	clc
	adc Dir, x
	cmp #MOVER_TOP_MIN
	bcc turn
	cmp #MOVER_TOP_MAX + 1
	bcc :+
turn:
	lda #$00
	sec
	sbc Dir, x
	sta Dir, x
	lda Top, x
	clc
	adc Dir, x
:
	sta Top, x
	ldy Slot, x
	sta PipeTops, y
	; o fundo muda quando q ou a linha do corpo de baixo muda
	lsr a
	lsr a
	lsr a
	cmp Rows, x
	bne changed
	jsr bottom_row
	cmp Bottom, x
	beq done
changed:
	jmp prepare_rows
done:
	rts
.endproc

; X: o grupo. As linhas q-1 a q+7 das quatro colunas, para o NMI.
.proc prepare_rows
	jsr bottom_row
	sta Bottom, x
	sta Temp1
	lda Top, x
	lsr a
	lsr a
	lsr a
	sta Rows, x
	sta Temp0
	; Temp2/Temp3: o endereço da linha q-1 na coluna do cano
	sec
	sbc #$01
	sta Temp2               ; a linha
	lda #$00
	sta Temp3
	ldy #$05
:
	asl Temp2               ; linha * 32
	rol Temp3
	dey
	bne :-
	lda Col, x
	and #%00011111
	ora Temp2
	sta Temp2
	lda Col, x
	and #%00100000
	lsr a
	lsr a
	lsr a
	ora #$20
	ora Temp3
	sta Temp3
	ldx #$00
	ldy Temp0
	dey                     ; a linha
row:
	lda Temp3
	sta MoverAddrHi, x
	lda Temp2
	sta MoverAddrLo, x
	lda #PIPE_BODY
	cpy Temp0
	bcc :+
	cpy Temp1
	bcs :+
	lda #$00
:
	sta MoverTile, x
	lda Temp2
	clc
	adc #$20
	sta Temp2
	bcc :+
	inc Temp3
:
	iny
	inx
	cpx #MOVER_ROWS
	bne row
	lda #$01
	sta MoverDirty
	rts
.endproc

; No NMI: as linhas preparadas, quatro tiles cada.
.proc MoverFlush
	lda MoverDirty
	beq done
	lda #$00
	sta MoverDirty
	lda PPUSTATUS
	lda #%10010000          ; incremento de 1
	sta PPUCTRL
	ldx #$00
row:
	lda MoverAddrHi, x
	sta PPUADDR
	lda MoverAddrLo, x
	sta PPUADDR
	lda MoverTile, x
	beq gap
	sta PPUDATA
	clc
	adc #$01
	sta PPUDATA
	adc #$01
	sta PPUDATA
	adc #$01
	sta PPUDATA
	jmp next
gap:
	sta PPUDATA
	sta PPUDATA
	sta PPUDATA
	sta PPUDATA
next:
	inx
	cpx #MOVER_ROWS
	bne row
done:
	rts
.endproc

; Cada imagem: os oito sprites dos chapéus de cada grupo, coluna por coluna (o de cima e o de
; baixo), na tela ou escondidos.
.proc UpdateMover
	ldx #$00
group:
	txa
	asl a
	asl a
	asl a
	asl a
	asl a
	tay                     ; 32 bytes por grupo
	lda Scene
	beq off
	lda Slot, x
	bpl on
off:
	lda #$FF                ; (nesta ordem: o padrão do esconder abaixo fica único)
	sta SPRITE_BUFFER + MOVER_OAM + 4, y
	sta SPRITE_BUFFER + MOVER_OAM, y
	iny
	iny
	iny
	iny
	iny
	iny
	iny
	iny
	tya
	and #%00011111
	bne off
	jmp next_group
on:
	; Temp0/1 = (x do cano - scroll) no anel de 512 pixels
	lda Col, x
	asl a
	asl a
	asl a
	sta Temp0
	lda Col, x
	lsr a
	lsr a
	lsr a
	lsr a
	lsr a
	sta Temp1
	lda Temp0
	sec
	sbc ScrollX
	sta Temp0
	lda Temp1
	sbc ScrollPage
	and #$01
	sta Temp1
	; os y dos sprites (o sprite aparece uma linha abaixo do seu y)
	lda Top, x
	sec
	sbc #$09
	sta Temp2
	clc
	adc #MOVER_GAP + 8
	sta Temp3
column:
	lda Temp1
	bne hide        ; 256 ou mais: fora da tela
	lda Temp2
	sta SPRITE_BUFFER + MOVER_OAM, y
	lda Temp3
	sta SPRITE_BUFFER + MOVER_OAM + 4, y
	lda Temp0
	sta SPRITE_BUFFER + MOVER_OAM + 3, y
	sta SPRITE_BUFFER + MOVER_OAM + 7, y
	jmp next
hide:
	lda #$FF
	sta SPRITE_BUFFER + MOVER_OAM, y
	sta SPRITE_BUFFER + MOVER_OAM + 4, y
	; sobre a borda esquerda (x de -7 a -1) os chapéus ficam em x = 0: sem eles o corpo
	; acabaria sem chapéu nesses pixels. Um modo widescreen já os mostrou no lugar (y não é $FF).
	lda Temp0
	cmp #256 - 7
	bcc next
	lda SPRITE_BUFFER + MOVER_OAM, y
	cmp #$FF
	bne next
	lda Temp2
	sta SPRITE_BUFFER + MOVER_OAM, y
	lda Temp3
	sta SPRITE_BUFFER + MOVER_OAM + 4, y
	lda #$00
	sta SPRITE_BUFFER + MOVER_OAM + 3, y
	sta SPRITE_BUFFER + MOVER_OAM + 7, y
next:
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
	adc #$08
	tay
	and #%00011111
	bne column
next_group:
	inx
	cpx #MOVER_GROUPS
	beq :+
	jmp group
:
	rts
.endproc
