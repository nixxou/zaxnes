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


; A small music driver: the four voices of "Saut de puce" (music/partition.json, converted by
; tools/music.py), played during a game only. The tempo is an accumulator of frames (136 BPM),
; each voice a list of notes in time order: the step it starts on, its length in frames, its
; period. The melody (pulse 1) starts loud and fades a little, the accompaniment (pulse 2) stays
; quiet, the bass (triangle) is cut between notes, the percussion (noise) is a short burst.
; When the bird dies the music stops on a crash: a noise burst and a falling pulse, both on the
; APU's own envelopes.

.export UpdateMusic
.importzp Scene, GameStopped

.segment "RODATA"
.include "music_data.inc"

.segment "BSS2"
MusicOn: .res 1
MusicStep: .res 2
MusicAcc: .res 2
P1Index: .res 1
P1Timer: .res 1
P1Volume: .res 1
P2Index: .res 1
P2Timer: .res 1
P2Volume: .res 1
TrIndex: .res 1
TrTimer: .res 1
TrHigh: .res 1
NoIndex: .res 1
NoTimer: .res 1

PULSE_CONTROL = %01110000   ; duty 25 %, length counter halted, constant volume
P1_VOLUME = 10
P1_FLOOR = 5
P2_VOLUME = 4
P2_FLOOR = 2
NOISE_VOLUME = 4

.segment "CODE"

; Once a frame (from the NMI): plays while a game runs, silent otherwise.
.proc UpdateMusic
	lda Scene
	beq stop
	lda GameStopped
	bne stop
	lda MusicOn
	bne play
	jsr start
	jmp notes
stop:
	lda MusicOn
	beq done
	lda #$00
	sta MusicOn
	jsr silence
	lda Scene
	beq done            ; back to the title: silence only
	jmp crash
done:
	rts

play:
	; the tempo: a step each MUSIC_STEP_FRAMES_256 / 256 frames
	inc MusicAcc+1
	lda MusicAcc
	sec
	sbc #<MUSIC_STEP_FRAMES_256
	tax
	lda MusicAcc+1
	sbc #>MUSIC_STEP_FRAMES_256
	bcc tick
	sta MusicAcc+1
	stx MusicAcc
	inc MusicStep
	bne :+
	inc MusicStep+1
:	lda MusicStep
	cmp #<MUSIC_STEPS
	bne notes
	lda MusicStep+1
	cmp #>MUSIC_STEPS
	bne notes
	jsr rewind
notes:
	jsr start_notes
tick:
	jmp tick_voices
.endproc

.proc start
	lda #$01
	sta MusicOn
	lda #$0F
	sta $4015
	lda #$08
	sta $4001           ; no sweep (negate set, so that low notes are not muted)
	sta $4005
	lda #$00
	sta MusicAcc
	sta MusicAcc+1
.endproc
.proc rewind
	lda #$00
	sta MusicStep
	sta MusicStep+1
	sta P1Index
	sta P2Index
	sta TrIndex
	sta NoIndex
	rts
.endproc

.proc silence
	lda #PULSE_CONTROL
	sta $4000
	sta $4004
	lda #%00110000
	sta $400C
	lda #$80
	sta $4008
	lda TrHigh
	sta $400B
	lda #$00
	sta P1Timer
	sta P2Timer
	sta TrTimer
	sta NoTimer
	rts
.endproc

; The crash ("paf"): a low noise burst and pulse 1 swept down from a high note, both decaying
; on their envelopes; their length counters end them.
.proc crash
	lda #%00000101      ; envelope, decay period 5
	sta $400C
	lda #$06
	sta $400E
	lda #$90            ; length index 18: 24 frames
	sta $400F
	lda #%10000011      ; duty 50 %, envelope, decay period 3
	sta $4000
	lda #%10010010      ; sweep: on, period 1, shift 2, pitch falling
	sta $4001
	lda #$C0
	sta $4002
	lda #$10            ; length index 2: 20 frames, the period's high bits 0
	sta $4003
	rts
.endproc

; Whether the note at x of a voice starts on this step: compares its two step bytes.
.macro starts_now lo, hi, skip
	lda lo, x
	cmp MusicStep
	bne skip
	lda hi, x
	cmp MusicStep+1
	bne skip
.endmacro

; The notes that start on this step.
.proc start_notes
pulse1:
	ldx P1Index
	cpx #Pulse1Count
	bcs pulse2
	starts_now Pulse1StepLo, Pulse1StepHi, pulse2
	lda Pulse1Frames, x
	sta P1Timer
	lda #P1_VOLUME
	sta P1Volume
	ora #PULSE_CONTROL
	sta $4000
	lda Pulse1Low, x
	sta $4002
	lda Pulse1High, x
	sta $4003
	inc P1Index
pulse2:
	ldx P2Index
	cpx #Pulse2Count
	bcs triangle
	starts_now Pulse2StepLo, Pulse2StepHi, triangle
	lda Pulse2Frames, x
	sta P2Timer
	lda #P2_VOLUME
	sta P2Volume
	ora #PULSE_CONTROL
	sta $4004
	lda Pulse2Low, x
	sta $4006
	lda Pulse2High, x
	sta $4007
	inc P2Index
triangle:
	ldx TrIndex
	cpx #TriangleCount
	bcs noise
	starts_now TriangleStepLo, TriangleStepHi, noise
	lda TriangleFrames, x
	sta TrTimer
	lda #$FF            ; linear counter held at its longest
	sta $4008
	lda TriangleLow, x
	sta $400A
	lda TriangleHigh, x
	sta TrHigh
	sta $400B
	inc TrIndex
noise:
	ldx NoIndex
	cpx #NoiseCount
	bcs done
	starts_now NoiseStepLo, NoiseStepHi, done
	lda NoiseFrames, x
	sta NoTimer
	lda #%00110000 | NOISE_VOLUME
	sta $400C
	lda NoiseLow, x
	sta $400E
	lda #$08
	sta $400F
	inc NoIndex
done:
	rts
.endproc

; Each frame: the notes' lengths and the pulses' fade.
.proc tick_voices
	; pulse 1
	lda P1Timer
	beq pulse2
	dec P1Timer
	bne :+
	lda #PULSE_CONTROL
	sta $4000
	jmp pulse2
:
	lda P1Timer
	and #$03
	bne pulse2
	lda P1Volume
	cmp #P1_FLOOR + 1
	bcc pulse2
	dec P1Volume
	lda P1Volume
	ora #PULSE_CONTROL
	sta $4000
pulse2:
	lda P2Timer
	beq triangle
	dec P2Timer
	bne :+
	lda #PULSE_CONTROL
	sta $4004
	jmp triangle
:
	lda P2Timer
	and #$03
	bne triangle
	lda P2Volume
	cmp #P2_FLOOR + 1
	bcc triangle
	dec P2Volume
	lda P2Volume
	ora #PULSE_CONTROL
	sta $4004
triangle:
	lda TrTimer
	beq noise
	dec TrTimer
	bne noise
	lda #$80            ; cut: the linear counter reloads to 0
	sta $4008
	lda TrHigh
	sta $400B
noise:
	lda NoTimer
	beq done
	dec NoTimer
	bne done
	lda #%00110000
	sta $400C
done:
	rts
.endproc
