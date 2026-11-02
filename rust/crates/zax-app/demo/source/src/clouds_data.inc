; Written by tools/clouds.py.

; The clouds on the 512-pixel ring: x (two bytes), y, and the offset of their first
; sprite in the sprite buffer, and its columns (each of two sprites).
CloudXLo: .byte $28, $C0, $2C, $C0
CloudXHi: .byte $00, $00, $01, $01
CloudY: .byte $28, $48, $2C, $48
CloudOam: .byte $84, $A4, $C4, $DC
CloudCols: .byte $04, $04, $03, $04
CLOUD_COUNT = 4
SPRITES_USED = 63

