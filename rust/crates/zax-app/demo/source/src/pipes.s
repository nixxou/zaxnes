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
.include "pipes.inc"

; Os canos não ficam presos às telas: eles aparecem a cada PIPE_SPACING pixels, em qualquer
; coluna (múltipla de 4) do anel de 64 colunas das duas telas. Quando um cano sai pela
; esquerda, o fundo dessas colunas é restaurado a partir de BackgroundData.

.export ColumnTarget, ColumnBuffer
.export DrawPipeColumn, RestoreColumn, FlushColumn, PipeAttributes, BackgroundAttributes, DrawPipe
.export PipeHeight, GenerateNewPipe
.import PRNG, BackgroundData
.importzp Temp0, Temp1, Temp2, Temp3

; Os tiles dos canos (tools/decor.py), quatro de largura
PIPE_BODY = $B0
PIPE_TOP_CAP = $B4
PIPE_BOTTOM_CAP = $B8

.segment "ZEROPAGE"
PipeHeight: .res 1

.segment "BSS2" ; RAM interna ($0400): o NROM não tem RAM em $6000
DrawColumn: .res 1
DrawPart: .res 1
; A coluna preparada para o NMI: sua coluna do anel ($FF nenhuma) e seus tiles
ColumnTarget: .res 1
ColumnBuffer: .res PIPES_HEIGHT

.segment "CODE"

; A: coluna do anel (0-63). Endereço da linha 0 dessa coluna em PPUADDR, e o PPU avançando
; 32 bytes por escrita.
.proc column_address
	pha
	lda PPUSTATUS
	lda #%10010100 ; NMI, fundo na tabela 1, incremento de 32
	sta PPUCTRL
	pla
	pha
	and #%00100000
	lsr a
	lsr a
	lsr a
	ora #$20        ; $2000 ou $2400
	sta PPUADDR
	pla
	and #%00011111
	sta PPUADDR
	rts
.endproc

; A: par de linhas (0-14), PipeHeight: a altura. A paleta desse par de linhas (16 pixels, a
; resolução dos atributos) no bloco do cano: o céu (0) se as duas linhas estão na abertura,
; onde o fundo aparece; o cano (1) senão; o chão (3) embaixo do cano.
.proc pair_palette
	cmp #PIPES_HEIGHT / 2
	bcc :+
	lda #$03
	rts
:
	asl a           ; a primeira linha do par
	sec
	sbc PipeHeight
	beq pipe        ; o chapéu de cima
	bcc pipe        ; acima da abertura
	cmp #PIPES_SEPARATION
	bcs pipe        ; a segunda linha chega ao chapéu de baixo
	lda #$00
	rts
pipe:
	lda #$01
	rts
.endproc

; A: coluna do anel, X: coluna do cano (0-3), PipeHeight: a altura. Prepara em ColumnBuffer a
; coluna do cano (fora do NMI); FlushColumn a escreve. Na abertura, as linhas que têm a paleta do
; céu recebem o fundo (as nuvens passam atrás do cano); as outras o céu.
.proc DrawPipeColumn
	sta Temp2
	sta ColumnTarget
	jsr background_pointer
	ldy #$00
row:
	cpy PipeHeight
	bcc body
	beq bottom_cap
	tya
	sec
	sbc PipeHeight
	cmp #PIPES_SEPARATION+1
	bcc gap
	beq top_cap
body:
	txa
	clc
	adc #PIPE_BODY
	jmp write
bottom_cap:
	txa
	clc
	adc #PIPE_BOTTOM_CAP
	jmp write
top_cap:
	txa
	clc
	adc #PIPE_TOP_CAP
	jmp write
gap:
	tya
	lsr a
	jsr pair_palette
	bne :+
	sty Temp3
	ldy #$00
	lda (Temp0), y
	ldy Temp3
	jmp write
:
	lda #$00
write:
	sta ColumnBuffer, y
	; a linha seguinte do fundo
	lda Temp0
	clc
	adc #$20
	sta Temp0
	bcc :+
	inc Temp1
:
	iny
	cpy #PIPES_HEIGHT
	bne row
	rts
.endproc

; Escreve no PPU a coluna preparada em ColumnBuffer, se houver uma (no NMI, ou com os gráficos
; desativados): 26 bytes de cima para baixo.
.proc FlushColumn
	lda ColumnTarget
	bmi done
	jsr column_address
	ldx #$00
:
	lda ColumnBuffer, x
	sta PPUDATA
	inx
	cpx #PIPES_HEIGHT
	bne :-
	lda #%10010000 ; incremento de volta a 1
	sta PPUCTRL
	lda #$FF
	sta ColumnTarget
done:
	rts
.endproc

; Temp2: coluna do anel. Temp0/1 = BackgroundData + tela * 1024 + coluna (a linha 0).
.proc background_pointer
	lda Temp2
	and #%00100000
	lsr a
	lsr a
	lsr a
	clc
	adc #>BackgroundData
	sta Temp1
	lda Temp2
	and #%00011111
	clc
	adc #<BackgroundData
	sta Temp0
	bcc :+
	inc Temp1
:
	rts
.endproc

; A: coluna do anel. Prepara em ColumnBuffer o fundo das linhas do cano (0 a PIPES_HEIGHT-1)
; dessa coluna; FlushColumn o escreve.
.proc RestoreColumn
	sta Temp2
	sta ColumnTarget
	jsr background_pointer
	ldx #$00
	ldy #$00
row:
	lda (Temp0), y
	sta ColumnBuffer, x
	lda Temp0
	clc
	adc #$20
	sta Temp0
	bcc :+
	inc Temp1
:
	inx
	cpx #PIPES_HEIGHT
	bne row
	rts
.endproc

; A: coluna do anel (múltipla de 4). Temp3 recebe o endereço baixo do primeiro byte de
; atributos desse bloco de 4 colunas; Temp2 o alto.
.proc attribute_address
	pha
	and #%00100000
	lsr a
	lsr a
	lsr a
	ora #$23        ; $23C0 ou $27C0
	sta Temp2
	pla
	and #%00011111
	lsr a
	lsr a
	ora #$C0
	sta Temp3
	rts
.endproc

; A: coluna do anel (múltipla de 4), PipeHeight: a altura. Os atributos do bloco do cano:
; paleta dos canos, a do céu nos pares de linhas da abertura, o chão embaixo.
.proc PipeAttributes
	jsr attribute_address
	ldx #$00
loop:
	; par de cima: linhas 4i e 4i+1
	txa
	asl a
	jsr pair_palette
	sta Temp1
	asl a
	asl a
	ora Temp1
	sta Temp1
	; par de baixo: linhas 4i+2 e 4i+3
	txa
	asl a
	ora #$01
	jsr pair_palette
	sta Temp0
	asl a
	asl a
	ora Temp0
	asl a
	asl a
	asl a
	asl a
	ora Temp1
	sta Temp0
	lda PPUSTATUS
	lda Temp2
	sta PPUADDR
	lda Temp3
	sta PPUADDR
	lda Temp0
	sta PPUDATA
	lda Temp3
	clc
	adc #$08
	sta Temp3
	inx
	cpx #7
	bne loop
	rts
.endproc

; A: coluna do anel (múltipla de 4). Recopia os atributos do fundo desse bloco.
.proc BackgroundAttributes
	pha
	jsr attribute_address
	; Temp0/1 = BackgroundData + tela * 1024 + 960 + bloco
	pla
	pha
	and #%00100000
	lsr a
	lsr a
	lsr a
	clc
	adc #>(BackgroundData + 960)
	sta Temp1
	pla
	and #%00011111
	lsr a
	lsr a
	clc
	adc #<(BackgroundData + 960)
	sta Temp0
	bcc :+
	inc Temp1
:
	ldy #$00
loop:
	lda PPUSTATUS
	lda Temp2
	sta PPUADDR
	lda Temp3
	sta PPUADDR
	lda (Temp0), y
	sta PPUDATA
	lda Temp3
	clc
	adc #$08
	sta Temp3
	tya
	clc
	adc #$08
	tay
	cpy #7 * 8
	bne loop
	rts
.endproc

; A: coluna do anel (múltipla de 4), PipeHeight: a altura. O cano inteiro, com os gráficos
; desativados (telas de título e de início).
.proc DrawPipe
	sta DrawColumn
	ldx #$00
loop:
	stx DrawPart
	txa
	clc
	adc DrawColumn
	jsr DrawPipeColumn
	jsr FlushColumn
	ldx DrawPart
	inx
	cpx #PIPES_WIDTH
	bne loop
	lda DrawColumn
	jmp PipeAttributes
.endproc

.proc GenerateNewPipe
	; Gera número aleatório
	jsr PRNG
	and #%00001111   ; 0-15
	; 0-9: a abertura fica sempre acima da cidade
	cmp #10
	bcc :+
	sbc #6
:
	; 3-12: o cano de cima mostra sempre um pedaço do corpo acima do chapéu
	clc
	adc #$03
	
	sta PipeHeight

	rts
.endproc
