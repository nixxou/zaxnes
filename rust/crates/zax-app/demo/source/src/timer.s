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

.export CreateTimer, TimerRunning
.importzp Tick

.segment "ZEROPAGE"
timer: .byte $00
.segment "CODE"

; A = tempo do timer em ticks
.proc CreateTimer
	clc
	adc Tick
	sta timer

	beq is_zero
	rts

	; Não pode ser zero pois esse valor é usado para definir se o timer está rodando
is_zero:
	inc timer

	rts
.endproc

; Usar BEQ, vai pular se ainda estiver rodando o timer
.proc TimerRunning
    lda timer
    beq timer_is_done   ; Se o timer for 0, pula para o final (está pronto)

    cmp Tick            ; Compara o valor do timer com o limite (tick)
    beq reset_timer     ; Se forem iguais, chegamos ao limite de tempo

    lda #$00            ; Carrega 0 no Acumulador (indicando "em progresso")
    rts                 ; Retorna

reset_timer:
    lda #$00
    sta timer           ; Reseta a variável de memória para 0

timer_is_done:
    lda #$01            ; Carrega 1 no Acumulador (indicando "finalizado")
    rts                 ; Retorna
.endproc