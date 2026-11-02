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
.include "pipes.inc"

.export GameSceneSetup, GameSceneLoop, GameSceneNMI, GameStopped, CreatePipe
.export PipeCols, PipeTops, PipeGaps

.import SpriteData, CreateBackground
.import UpdateBird, ResetBird, ResetScore, IncScore
.import DrawPipeColumn, RestoreColumn, FlushColumn, PipeAttributes, BackgroundAttributes, DrawPipe
.import ColumnTarget
.import GenerateNewPipe
.import MoverReset, MoverSpawn, MoverEnd, MoveMover, DrawMoverColumn, MoverFlush
.importzp PipeHeight, ScrollX, ScrollPage, Temp0, Temp1

; Um cano novo a cada PIPE_SPACING pixels (múltiplo de 32: um bloco de atributos), desenhado
; SPAWN_AHEAD pixels à frente do scroll: fora da tela também com as margens de um modo
; widescreen de até 88 pixels. O cano de três espaçamentos antes, já fora pela esquerda, ocupa
; no anel de 512 pixels as quatro colunas logo à direita do novo: elas são restauradas.
PIPE_SPACING = 160
SPAWN_AHEAD = 352
FIRST_PIPE_COLUMN = 28

.segment "ZEROPAGE"
GameStopped: .byte $00

.segment "BSS2" ; RAM interna ($0400): o NROM não tem RAM em $6000
Phase: .res 1       ; pixels desde o último cano novo (0 a PIPE_SPACING-1)
PhaseFresh: .res 1  ; o laço avançou Phase: o NMI tem um passo a fazer
Events: .res 1      ; canos novos desde o início
NewColumn: .res 1
OldColumn: .res 1
NewHeight: .res 1
NewMover: .res 1    ; o cano novo é o que se move (src/mover.s)
; Os canos em jogo (coluna do anel, $FF para nenhum), o y da borda de baixo do cano de cima e a
; altura da abertura em pixels, para as colisões
PipeCols: .res 4
PipeTops: .res 4
PipeGaps: .res 4

.segment "CODE"

.macro reset_sprites
.repeat SPRITE_AMOUNT, i
	lda SpriteData + .sizeof(Sprite)*i + Sprite::pos_y
	sta SPRITE_ADDR (SpriteData, i, Sprite::pos_y)
.endrepeat
.endmacro

.macro reset_variables
	lda #$00
	sta GameStopped
	sta ScrollPage
	sta ScrollX
	sta PhaseFresh
	sta Events
	sta NewMover
	jsr MoverReset
	; o primeiro evento quando o scroll chega a 32: o cano novo fica a 160 pixels do primeiro
	lda #PIPE_SPACING - 32
	sta Phase
	lda #$FF
	sta ColumnTarget
	sta PipeCols
	sta PipeCols+1
	sta PipeCols+2
	sta PipeCols+3

	jsr ResetBird
	reset_sprites
	jsr ResetScore
.endmacro

.proc GameSceneSetup
	reset_variables
	
	lda #%00000000	  ; Desativa os gráficos
	sta PPUMASK

	jsr CreateBackground

	; O primeiro cano, à direita da tela
	lda #FIRST_PIPE_COLUMN
	jsr CreatePipe
	lda #FIRST_PIPE_COLUMN
	sta PipeCols
	ldx #$00
	jsr store_pipe

	lda #%00011110	  ; Ativa os gráficos novamente, também na primeira coluna
	sta PPUMASK

	rts
.endproc

; A: coluna do anel (múltipla de 4). Um cano de altura nova, inteiro (gráficos desativados).
.proc CreatePipe
	pha
	jsr GenerateNewPipe
	pla
	jmp DrawPipe
.endproc

; X: o lugar do cano, PipeHeight: sua altura. A abertura de um cano normal, para as colisões.
.proc store_pipe
	lda PipeHeight
	clc
	adc #$01            ; a borda do chapéu
	asl a
	asl a
	asl a
	sta PipeTops, x
	lda #PIPES_SEPARATION * 8
	sta PipeGaps, x
	rts
.endproc

.proc GameSceneLoop
	jsr UpdateBird
	
	; Para o scroll quando o jogo está parado
	lda GameStopped
	bne end

	inc ScrollX
	bne :+
	; Passa para a outra tela
	lda ScrollPage
	eor #$01
	sta ScrollPage
:
	inc Phase
	lda Phase
	cmp #PIPE_SPACING
	bne :+
	lda #$00
	sta Phase
:
	jsr prepare_step
	; o cano que se move anda fora das fases das colunas e dos atributos
	lda Phase
	cmp #10
	bcc :+
	jsr MoveMover
:
	lda #$01
	sta PhaseFresh
end:
	rts
.endproc

; Um passo por pixel de scroll, preparado aqui (fora do NMI): no passo 0 um cano novo é
; escolhido, nos passos 0-3 as colunas do cano antigo são restauradas, nos passos 4-7 as do novo
; desenhadas (cada uma preparada em ColumnBuffer, que o NMI escreve), no passo 8 o NMI escreve os
; atributos.
.proc prepare_step
	lda Phase
	beq new_pipe
	jmp step
new_pipe:

	; --- Um cano novo ---
	jsr GenerateNewPipe
	lda PipeHeight
	sta NewHeight
	; coluna = ((tela:ScrollX) + SPAWN_AHEAD) / 8, no anel de 64
	lda ScrollX
	clc
	adc #<SPAWN_AHEAD
	sta Temp0
	lda ScrollPage
	adc #>SPAWN_AHEAD
	sta Temp1
	lsr Temp1
	ror Temp0
	lsr Temp1
	ror Temp0
	lsr Temp1
	ror Temp0
	lda Temp0
	and #63
	sta NewColumn
	clc
	adc #PIPES_WIDTH
	and #63
	sta OldColumn
	; guarda o cano para as colisões
	lda Events
	clc
	adc #$01
	and #$03
	tax
	lda NewColumn
	sta PipeCols, x
	jsr store_pipe
	; o cano que sai pela esquerda já não se move; o novo talvez
	lda OldColumn
	jsr MoverEnd
	lda Events
	clc
	adc #$01
	and #$03
	tax
	lda NewColumn
	jsr MoverSpawn
	lda #$00
	rol a
	sta NewMover
	beq :+
	lda Events
	clc
	adc #$01
	and #$03
	tax
	lda #MOVER_GAP
	sta PipeGaps, x
	lda #PIPES_HEIGHT   ; os atributos: o cano em todo o bloco (a abertura é só céu)
	sta NewHeight
:
	; a cada cano novo, o pássaro acabou de passar um (menos no primeiro)
	lda Events
	beq :+
	jsr IncScore
:
	inc Events
	bne step
	lda #$04
	sta Events

step:
	lda Phase
	cmp #4
	bcs :+
	; 0-3: o fundo das colunas do cano antigo
	clc
	adc OldColumn
	and #63
	jmp RestoreColumn
:
	cmp #8
	bcs done
	; 4-7: as colunas do cano novo
	sec
	sbc #4
	tax
	lda NewHeight
	sta PipeHeight
	txa
	clc
	adc NewColumn
	ldy NewMover
	bne mover
	jmp DrawPipeColumn
mover:
	and #63
	jmp DrawMoverColumn
done:
	rts
.endproc

; O NMI: a coluna preparada, ou no passo 8 os atributos dos dois blocos.
.proc GameSceneNMI
	lda PhaseFresh
	bne :+
	rts
:
	lda #$00
	sta PhaseFresh
	jsr MoverFlush
	lda Phase
	cmp #8
	beq :+
	jmp FlushColumn
:
	lda OldColumn
	jsr BackgroundAttributes
	lda NewHeight
	sta PipeHeight
	lda NewColumn
	jmp PipeAttributes
.endproc
