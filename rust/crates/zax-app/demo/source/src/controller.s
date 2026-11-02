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

.include "controller.inc"

.export ReadController, ControllerButtons, ControllerPressed

.segment "ZEROPAGE"
ControllerButtons: .res 1
; Buttons pressed since the last reading (not held)
ControllerPressed: .res 1
PreviousButtons: .res 1
.segment "CODE"

; Retirado de https://www.nesdev.org/wiki/Controller_reading_code

; At the same time that we strobe bit 0, we initialize the ring counter
; so we're hitting two birds with one stone here
.proc ReadController
    lda ControllerButtons
    sta PreviousButtons
    lda #$01
    ; While the strobe bit is set, buttons will be continuously reloaded.
    ; This means that reading from JOYPAD1 will only return the state of the
    ; first button: button A.
    sta JOYPAD1
    sta ControllerButtons
    lsr a        ; now A is 0
    ; By storing 0 into JOYPAD1, the strobe bit is cleared and the reloading stops.
    ; This allows all 8 buttons (newly reloaded) to be read from JOYPAD1.
    sta JOYPAD1
loop:
    lda JOYPAD1
    lsr a        ; bit 0 -> Carry
    rol ControllerButtons  ; Carry -> bit 0; bit 7 -> Carry
    bcc loop
    lda PreviousButtons
    eor #$FF
    and ControllerButtons
    sta ControllerPressed
    rts
.endproc