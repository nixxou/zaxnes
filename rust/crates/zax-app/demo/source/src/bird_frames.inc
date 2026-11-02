; Written by tools/bird.py from tools/bird_art.py.

; Body tiles (top row left to right, then bottom row) by pose and wing
BirdFrames:
	.byte $30, $31, $32, $33, $34, $35,  $36, $37, $38, $39, $3A, $3B,  $3C, $3D, $3E, $3F, $40, $41
	.byte $42, $43, $44, $45, $46, $47,  $48, $49, $4A, $4B, $4C, $4D,  $4E, $4F, $50, $51, $52, $53
	.byte $54, $55, $56, $57, $58, $59,  $5A, $5B, $5C, $5D, $5E, $5F,  $60, $61, $62, $63, $64, $65
; The beak sprite of each pose: tile, and offset from the body
BeakTile: .byte $68, $69, $6A
BeakDX: .byte 13, 15, 13
BeakDY: .byte 7, 5, 9
