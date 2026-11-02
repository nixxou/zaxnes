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
.include "controller.inc"
.include "ppu.inc"

.export Setup, MainLoop, NMI, Tick, Temp0, Temp1, Temp2, Temp3, ScrollX, ScrollPage, Scene

.importzp RNGSeed, ControllerButtons
.import LoadSprites, LoadPalletes, ReadController

.include "pipes.inc"

.import GameSceneSetup, GameSceneLoop, GameSceneNMI
.import TitleScreenSetup, TitleScreenLoop, TitleScreenNMI
.import UpdateClouds, UpdateMusic, UpdateMover

.segment "ZEROPAGE"
NMIReady: .byte $00
Tick: .byte $00

; Você não sabe e nunca vai saber para que servem, e o fato de serem globais fazem seu uso ser
; uma desgraça porque você também não sabe se outra função vai usar isso. É nessas horas que dá
; vontade de programar em Haskell.
Temp0: .res 1
Temp1: .res 1
Temp2: .res 1
Temp3: .res 1

ScrollX: .byte $00
; A tela (0 ou 1) onde começa a parte visível -- o fundo tem duas telas lado a lado
ScrollPage: .byte $00
Scene: .byte $00
.segment "CODE"

.proc Setup
	jsr LoadSprites
	jsr LoadPalletes

;	lda #$01
;	sta Scene

	beq title_screen

	jsr GameSceneSetup
	rts

title_screen:
	jsr TitleScreenSetup

	rts
.endproc

.proc MainLoop
	lda NMIReady
	beq MainLoop  ; wait until frame ready
	lda #$00
	sta NMIReady

	jsr ReadController

add_random_seed:
	lda RNGSeed
	bne jump_setting_seed ; Se não for zero, dane-se, não precisamos definir um valor.
	
	lda ControllerButtons
	beq jump_setting_seed ; A seed será definida no número de ticks até que o primeiro botão seja apertado

	; Define como sendo o tick
	lda Tick
	sta RNGSeed
	; Usa a porta NOT pra dar uma variada
	eor #$FF ; NOT 
	sta RNGSeed+1
jump_setting_seed:
	lda Scene
	beq title_screen

	jsr GameSceneLoop
	jsr UpdateMover
	jmp UpdateClouds
title_screen:
	jsr TitleScreenLoop
	jsr UpdateMover
	jmp UpdateClouds
.endproc

.proc NMI
	inc Tick

	lda #$01
	sta NMIReady

	lda Scene
	beq title_screen

	jsr GameSceneNMI
	jmp end
title_screen:
	jsr TitleScreenNMI

end:
	lda ScrollPage
	ora #%10010000
	sta PPUCTRL
	lda ScrollX
	sta PPUSCROLL   ; X Scroll
	lda #$00
	sta PPUSCROLL   ; Y Scroll

	; a música, depois do trabalho do PPU
	jsr UpdateMusic

	rts
.endproc