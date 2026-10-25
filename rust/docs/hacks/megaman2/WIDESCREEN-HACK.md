# Mega Man 2 (USA): the widescreen hack

This document describes completely the modification of *Mega Man 2 (USA)* that Zaxnes ships as a
built-in patch. The appendices hold the new code and the build script, which writes every hook
after checking the bytes it replaces: with them the ROM can be rebuilt identically.

The background of the margins is not the hack's work: the emulator's Mega Man 2 margin decoder
reads the stage layout in PRG-ROM (see [`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md),
section 4). The hack does the objects: enemies that spawn, live and show in the margins.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Mega Man 2 (USA)*, MMC1, 256 KiB PRG (16 banks of 16 KiB, bank 15 fixed at `$C000`), CHR-RAM |
| Source data (iNES header excluded) | CRC-32 `0FCFC04D` |
| Target data | CRC-32 `3EE5A6CA` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | bank 15 only: new code at `$F300` (in the `$FF` fill `$F2E3`-`$F8FF`) and eleven hooks |
| Patch | `rust/crates/zax-nes/patches/megaman2-usa-widescreen.bps`, made against the data without header |

---

## 2. What the game does, and what changes

Object slots: `$00`-`$0F` Mega Man and his weapons, `$10`-`$1F` enemies and their shots. Per slot:
`$0400` type, `$0420` flags (bit 7 active, bits 0-1 collision kinds), `$0440`/`$0460` x screen /
pixel, `$04A0` y, `$06E0` screen x low byte. Camera `$20:$1F`; the section being played spans
screens `$14` to `$15`.

| Game routine | Original | Hack |
|---|---|---|
| spawn window (`$D658`) | enemies of the stage list (`$B600` screen, `$B700` x, ... sorted by x) spawn between camera and camera + 255 | camera - M to camera + 255 + M, within the section |
| object moved left / right (`$EF20`, tests at `$EF43`, `$EF75`) | removed when its screen x leaves `[8, 248)` | enemies: removed when it leaves `[LO + 8, HI - 8)` |
| object that did not move sideways (`$EFE8`) | removed when off the screen (`$2F != 0`) | enemies: removed outside `[LO, HI)` |
| still on the screen? (`$E4E9`) | removed outside `[0, 256)` | enemies: outside `[LO, HI)` |
| collisions (`$EEBE`, `$EFB7`: `LDA $0420,X / AND #$03`) | on 8-bit screen x | none for an object outside the original screen |
| sprites (`$CE7D`, `$CECE`) | x = 8-bit screen x + signed offset, hidden when it carries | 16-bit x, shown within `[-M, 256 + M)`, high byte to `$5100 + OAM offset` |

`LO = max(-M, section start - camera)`, `HI = min(256 + M, section end - camera)`; M is the margin,
0 for slots below `$10` (Mega Man's shots keep the original limits). The game's collisions compare
8-bit screen x, so an object 256 pixels away would collide; outside the original screen nothing
collides, which keeps the game's logic exactly the original's on the screen. With M = 0 every
routine does exactly what the code it replaces does.

Margin: `M = min($5002, 128)` (128: what sprites can show through the extension).

## 3. The extension

Unlocked at reset (`$FFE4`, `JMP $F2D1` becomes `JMP wide_reset`, which unlocks then jumps on) and
checked every NMI (`$CFFD`, `LDA $F7 / AND #$7C / STA $F7` becomes `JSR wide_nmi`, which unlocks
again if needed, then writes `$5003 = M`, `$5004 = $5005 = 0` (the nametable columns are hidden:
the decoder draws the background), `$5007 = 2` (sprites free of those extents), and does the
replaced instructions).

Variables (extension RAM): `WV_M $5200`, the live range `WV_LO $5201`/`$5202` and `WV_HI
$5203`/`$5204` (signed 16-bit), the object's screen x `WV_S $5205`/`$5206`, scratch `$5207`,
`$5208`.

## 4. Hooks

Each is checked against the bytes it replaces; NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$FFE4` | `JMP $F2D1` | `JMP wide_reset` |
| `$CFFD` | `LDA $F7 / AND #$7C / STA $F7` | `JSR wide_nmi` |
| `$D65F` | `CLC / LDA $1F / STA $0A / ADC #$FF / STA $08 / LDA $20 / STA $0B / ADC #0 / STA $09` | `JSR wide_spawn_window / JMP $D670` |
| `$EF43` | the 23-byte test "off the screen or x < 8" | `JSR wide_cull_left / BCS $EF8C / BCC $EF8A` |
| `$EF75` | the 21-byte test "off the screen or x >= 248" | `JSR wide_cull_right / BCS $EF8C` (then `CLC / RTS` at `$EF8A`) |
| `$E4E9` | "x - camera not on this screen" (15 bytes) | `JSR wide_cull_screen / BCS $E4FA` |
| `$EFE8` | `LDA $2F / BNE $EF8C / CLC / RTS` | `JMP wide_cull_still` |
| `$EEBE`, `$EFB7` | `LDA $0420,X / AND #$03 / BEQ` | `JSR wide_hit_flags / BEQ` (same target) |
| `$CE7D` | `SEC / LDA $0460,X / SBC $1F / STA $00 / LDA $0440,X / SBC $20` | `JSR wide_draw_origin` |
| `$CECE` | the 13-byte add and carry tests of a sprite's x | `JSR wide_sprite_x / BCS $CEDB / JMP $CEE2` |

`$EF8C` is the game's own removal (it also lets an enemy of the list spawn again); `$CEDB` hides a
sprite (OAM y `$F8`).

## 5. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Mega Man 2 (USA).nes" build/mm2wide.bps
```

assembles `src/wide.s` for `$F300`-`$F8FF` (`-g`, labels from `ld65 -Ln`), checks the code space is
still `$FF`, writes the code and the hooks, `build/mm2wide.nes` and the BPS patch (`tools/make_bps.py`,
SourceRead / TargetRead only). `WIDE_SKIP=<hex addresses>` leaves hooks out, for testing.

## 6. Limits

* Enemies a stage's own code removes when off the screen, other than through the routines above,
  still go at the original edge.
* More enemies alive means more CPU time: the game's slowdowns can come more often.
* No vertical margin yet (the decoder draws the sides only).

---

## Appendices

### A. `src/wide.s`

```asm
; Mega Man 2 (USA) widescreen hack: the code the patched fixed bank calls.
;
; Assembled for $F300 of the fixed bank (PRG bank 15), the unused $FF fill from $F2E3 to $F8FF.
; Talks to the Zaxnes widescreen extension (rust/docs/WIDESCREEN-EXTENSION.md); the background of
; the margins comes from the emulator's Mega Man 2 margin decoder, the hack does the objects:
;
;  * enemies (object slots $10-$1F) spawn when they come within the margin of the screen, and
;    live until they leave it, all within the section being played (screens $14-$15);
;  * every object's sprites get the high byte of their 16-bit x, so they show in the margins;
;  * an enemy outside the original screen touches nothing and nothing touches it: the game's
;    collisions work on 8-bit screen x, and keep doing so on the original screen only;
;  * Mega Man's own objects (slots $00-$0F) keep the original limits.
;
; With a margin of 0 every routine does exactly what the code it replaces does.

WIDE_SIG        = $5000         ; W: unlock "WIDE"; R: $57 once unlocked
WIDE_HOST       = $5002         ; R: margin the host offers on each side
WIDE_REQ        = $5003         ; W: margin asked for
WIDE_LEFT       = $5004         ; W: background extents (the decoder draws the margins)
WIDE_RIGHT      = $5005
WIDE_LAYOUT     = $5007         ; W: bit 1, sprites free of the background extents
WIDE_SPRHI      = $5100         ; W: high byte of the x of sprite n at $5100 + 4n

MAX_MARGIN      = 128           ; what sprites can show (the extension's own limit)

; Variables in the extension's RAM.
WV_M            = $5200         ; margin of this frame
WV_LOL          = $5201         ; screen x range objects may live in: [LO, HI), signed 16-bit
WV_LOH          = $5202
WV_HIL          = $5203
WV_HIH          = $5204
WV_SL           = $5205         ; screen x of the object at hand, signed 16-bit
WV_SH           = $5206
WV_T0           = $5207
WV_T1           = $5208

; Game RAM (original meanings).
CAM_LO          = $1F           ; camera x, pixel
CAM_HI          = $20           ; camera x, screen
SECTION_FIRST   = $14           ; first screen of the section
SECTION_LAST    = $15           ; last screen of the section
OBJ_X_HI        = $0440         ; object x, screen
OBJ_X_LO        = $0460         ; object x, pixel

ORIGINAL_RESET  = $F2D1
KILL            = $EF8C         ; removes object X (and lets its enemy spawn again), SEC / RTS
FIRST_ENEMY     = $10

.segment "CODE"
.org $F300

; ---------------------------------------------------------------------------- ;
; Reset ($FFE4, in place of JMP $F2D1).
wide_reset:
    JSR      wide_unlock
    JMP      ORIGINAL_RESET

wide_unlock:
    LDA      #$57
    STA      WIDE_SIG
    LDA      #$49
    STA      WIDE_SIG
    LDA      #$44
    STA      WIDE_SIG
    LDA      #$45
    STA      WIDE_SIG
    LDA      #$00
    STA      WV_M
    RTS

; ---------------------------------------------------------------------------- ;
; NMI ($CFFD, in place of LDA $F7 / AND #$7C / STA $F7): the extension's settings for the next
; frame (unlocked again first if a state made without the hack left it locked).
wide_nmi:
    LDA      WIDE_SIG
    CMP      #$57
    BEQ      @set
    JSR      wide_unlock
@set:
    LDA      WIDE_HOST
    CMP      #MAX_MARGIN
    BCC      @margin
    LDA      #MAX_MARGIN
@margin:
    STA      WV_M
    STA      WIDE_REQ
    LDA      #$00
    STA      WIDE_LEFT
    STA      WIDE_RIGHT
    LDA      #$02
    STA      WIDE_LAYOUT
    LDA      $F7
    AND      #$7C
    STA      $F7
    RTS

; ---------------------------------------------------------------------------- ;
; The screen x range objects of slot X may live in, into WV_LO/WV_HI:
;   LO = max(-M, section start - camera), HI = min(256 + M, section end - camera),
; M being 0 for Mega Man's own slots. The original's range is [0, 256).
wide_bounds:
    LDA      WV_M
    CPX      #FIRST_ENEMY
    BCS      @margin
    LDA      #$00
@margin:
    STA      WV_T0
    SEC                                 ; LO = section start - camera
    LDA      #$00
    SBC      CAM_LO
    STA      WV_LOL
    LDA      SECTION_FIRST
    SBC      CAM_HI
    STA      WV_LOH
    CLC                                 ; LO + M < 0: LO = -M
    LDA      WV_LOL
    ADC      WV_T0
    LDA      WV_LOH
    ADC      #$00
    BPL      @hi
    SEC
    LDA      #$00
    SBC      WV_T0
    STA      WV_LOL
    LDA      #$00
    SBC      #$00
    STA      WV_LOH
@hi:
    SEC                                 ; HI = section end - camera
    LDA      #$00
    SBC      CAM_LO
    STA      WV_HIL
    LDA      SECTION_LAST
    SBC      CAM_HI
    CLC
    ADC      #$01
    STA      WV_HIH
    SEC                                 ; HI - (256 + M) >= 0: HI = 256 + M
    LDA      WV_HIL
    SBC      WV_T0
    LDA      WV_HIH
    SBC      #$01
    BMI      @rts
    LDA      WV_T0
    STA      WV_HIL
    LDA      #$01
    STA      WV_HIH
@rts:
    RTS

; WV_S = screen x of object X (object x - camera).
wide_screen_x:
    SEC
    LDA      OBJ_X_LO,x
    SBC      CAM_LO
    STA      WV_SL
    LDA      OBJ_X_HI,x
    SBC      CAM_HI
    STA      WV_SH
    RTS

; Carry set when WV_S - (WV_LO + A) < 0 (A an unsigned offset).
wide_below_lo:
    CLC
    ADC      WV_LOL
    STA      WV_T1
    LDA      WV_LOH
    ADC      #$00
    STA      WV_T0
    LDA      WV_SL
    CMP      WV_T1
    LDA      WV_SH
    SBC      WV_T0
    BMI      @below
    CLC
    RTS
@below:
    SEC
    RTS

; Carry set when WV_S - (WV_HI - A) >= 0 (A an unsigned offset).
wide_from_hi:
    STA      WV_T1
    SEC
    LDA      WV_HIL
    SBC      WV_T1
    STA      WV_T1
    LDA      WV_HIH
    SBC      #$00
    STA      WV_T0
    LDA      WV_SL
    CMP      WV_T1
    LDA      WV_SH
    SBC      WV_T0
    BMI      @inside
    SEC
    RTS
@inside:
    CLC
    RTS

; ---------------------------------------------------------------------------- ;
; Object moved left ($EF43, in place of the 8-bit test "off the screen, or x < 8"): carry set
; when it goes (x < LO + 8, or x >= HI). $08 gets the screen x low byte, as before.
wide_cull_left:
    JSR      wide_bounds
    JSR      wide_screen_x
    LDA      WV_SL
    STA      $08
    LDA      #$08
    JSR      wide_below_lo
    BCS      @rts
    LDA      #$00
    JMP      wide_from_hi
@rts:
    RTS

; Object moved right ($EF75, in place of "off the screen, or x >= 248"): carry set when it goes
; (x >= HI - 8, or x < LO).
wide_cull_right:
    JSR      wide_bounds
    JSR      wide_screen_x
    LDA      WV_SL
    STA      $08
    LDA      #$08
    JSR      wide_from_hi
    BCS      @rts
    LDA      #$00
    JMP      wide_below_lo
@rts:
    RTS

; Object still on the screen? ($E4E9, in place of "x - camera not on this screen"): carry set
; when it goes (x < LO or x >= HI).
wide_cull_screen:
    JSR      wide_bounds
    JSR      wide_screen_x
    LDA      #$00
    JSR      wide_below_lo
    BCS      @rts
    LDA      #$00
    JMP      wide_from_hi
@rts:
    RTS

; Object that did not move sideways ($EFE8, in place of LDA $2F / BNE $EF8C / CLC / RTS: removed
; when off the screen): removed when outside [LO, HI).
wide_cull_still:
    JSR      wide_cull_screen
    BCC      @keep
    JMP      KILL
@keep:
    RTS

; ---------------------------------------------------------------------------- ;
; Collision flags of object X ($EEBE and $EFB7, in place of LDA $0420,X / AND #$03): 0 (Z set)
; when the object is outside the original screen, so it touches nothing.
wide_hit_flags:
    LDA      $0420,x
    AND      #$03
    BEQ      @rts
    STA      WV_T1
    JSR      wide_screen_x
    LDA      WV_SH
    BEQ      @on
    LDA      #$00
    RTS
@on:
    LDA      WV_T1
@rts:
    RTS

; ---------------------------------------------------------------------------- ;
; Sprite drawing. Origin ($CE7D, in place of SEC / LDA $0460,X / SBC $1F / STA $00 /
; LDA $0440,X / SBC $20): $00 = screen x low byte, WV_S = the 16-bit screen x.
wide_draw_origin:
    JSR      wide_screen_x
    LDA      WV_SL
    STA      $00
    RTS

; One sprite ($CECE, in place of the 8-bit add and its off-screen tests): A = the sprite's
; signed x offset, X = its OAM offset. Carry set: hidden (outside [-M, 256 + M)); otherwise
; A = OAM x and the high byte is written for the extension.
wide_sprite_x:
    STA      WV_T1
    LDA      #$00
    BIT      WV_T1
    BPL      @positive
    LDA      #$FF
@positive:
    STA      WV_T0                      ; sign of the offset
    CLC
    LDA      WV_SL
    ADC      WV_T1
    STA      WV_T1                      ; x low
    LDA      WV_SH
    ADC      WV_T0
    STA      WV_T0                      ; x high
    BMI      @left
    BEQ      @show                      ; 0 <= x < 256
    CMP      #$01
    BNE      @hide
    LDA      WV_T1                      ; 256 <= x: below 256 + M?
    CMP      WV_M
    BCS      @hide
    BCC      @show
@left:
    CMP      #$FF
    BNE      @hide
    LDA      WV_T1                      ; x < 0: x + M >= 0?
    CLC
    ADC      WV_M
    BCC      @hide
@show:
    LDA      WV_T0
    STA      WIDE_SPRHI,x
    LDA      WV_T1
    CLC
    RTS
@hide:
    SEC
    RTS

; ---------------------------------------------------------------------------- ;
; Spawn window ($D65F, in place of $0B:$0A = camera, $09:$08 = camera + 255): widened by the
; margin, within the section being played.
wide_spawn_window:
    SEC                                 ; left = camera - M
    LDA      CAM_LO
    SBC      WV_M
    STA      $0A
    LDA      CAM_HI
    SBC      #$00
    STA      $0B
    BCC      @left_section              ; before the level
    CMP      SECTION_FIRST
    BCS      @right
@left_section:
    LDA      #$00                       ; before the section: its start
    STA      $0A
    LDA      SECTION_FIRST
    STA      $0B
@right:
    CLC                                 ; right = camera + 255 + M
    LDA      CAM_LO
    ADC      #$FF
    STA      $08
    LDA      CAM_HI
    ADC      #$00
    STA      $09
    CLC
    LDA      $08
    ADC      WV_M
    STA      $08
    LDA      $09
    ADC      #$00
    STA      $09
    CMP      SECTION_LAST
    BCC      @rts
    BEQ      @rts
    LDA      #$FF                       ; past the section: its last pixel
    STA      $08
    LDA      SECTION_LAST
    STA      $09
@rts:
    RTS

wide_end:
.assert wide_end <= $F900, error, "widescreen code past $F8FF"
```

### B. `tools/build.py`

```python
"""Builds the Mega Man 2 (USA) widescreen hack and its BPS patch.

Usage: python -I tools/build.py <Mega Man 2 (USA).nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $F300 of the
   fixed bank, into the unused $FF fill from $F2E3 to $F8FF.
2. Writes the hooks into the fixed bank, each checked against the bytes it replaces.
3. Writes build/mm2wide.nes and, if asked, the BPS patch (data without iNES header).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x0FCFC04D
BANK = 15
BANK_SIZE = 0x4000
FIXED = 0xC000
CODE_START = 0xF300
CODE_END = 0xF900

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    offset = target - (at + 2)
    assert -128 <= offset <= 127, (hex(at), hex(target))
    return bytes([opcode, offset & 0xFF])


BCS, BCC, BEQ = 0xB0, 0x90, 0xF0
NOP = 0xEA


def hooks(sym):
    """(address, original bytes, new bytes): every patch of the fixed bank."""
    out = []

    def put(address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((address, original, new))

    # Reset: unlock the extension first.
    put(0xFFE4, '4c d1 f2', jmp(sym['wide_reset']))
    # NMI: the extension's settings for the next frame.
    put(0xCFFD, 'a5 f7 29 7c 85 f7', jsr(sym['wide_nmi']))
    # Spawn window.
    put(0xD65F, '18 a5 1f 85 0a 69 ff 85 08 a5 20 85 0b 69 00 85 09',
        jsr(sym['wide_spawn_window']) + jmp(0xD670))
    # Objects moving left / right: the off-screen tests.
    put(0xEF43, '38 bd 60 04 e5 1f 85 08 bd 40 04 e5 20 d0 3a a5 08 c9 08 90 34 b0 30',
        jsr(sym['wide_cull_left']) + branch(BCS, 0xEF46, 0xEF8C) + branch(BCC, 0xEF48, 0xEF8A))
    put(0xEF75, '38 bd 60 04 e5 1f 85 08 bd 40 04 e5 20 d0 08 a5 08 c9 f8 b0 02',
        jsr(sym['wide_cull_right']) + branch(BCS, 0xEF78, 0xEF8C))
    # Object still on the screen?
    put(0xE4E9, '38 bd 60 04 e5 1f bd 40 04 e5 20 90 04 d0 02',
        jsr(sym['wide_cull_screen']) + branch(BCS, 0xE4EC, 0xE4FA))
    # Object that did not move sideways: removed off the screen.
    put(0xEFE8, 'a5 2f d0 a0 18 60', jmp(sym['wide_cull_still']))
    # Collision flags of an enemy, both object loops.
    put(0xEEBE, 'bd 20 04 29 03 f0 2a', jsr(sym['wide_hit_flags']) + branch(BEQ, 0xEEC1, 0xEEEF))
    put(0xEFB7, 'bd 20 04 29 03 f0 2a', jsr(sym['wide_hit_flags']) + branch(BEQ, 0xEFBA, 0xEFE8))
    # Sprites: the origin and each sprite's x.
    put(0xCE7D, '38 bd 60 04 e5 1f 85 00 bd 40 04 e5 20', jsr(sym['wide_draw_origin']))
    put(0xCECE, '18 30 06 65 00 90 0d b0 04 65 00 b0 07',
        jsr(sym['wide_sprite_x']) + branch(BCS, 0xCED1, 0xCEDB) + jmp(0xCEE2))
    return out


def tool(name):
    return [name] if shutil.which(name) else ['wsl', '-e', name]


def wsl_path(path):
    if shutil.which('ca65'):
        return path
    drive, rest = os.path.splitdrive(os.path.abspath(path))
    return '/mnt/' + drive[0].lower() + rest.replace('\\', '/')


def assemble():
    build = os.path.join(ROOT, 'build')
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    open(cfg, 'w', newline='\n').write(
        f'MEMORY {{ CODE: start = ${CODE_START:04X}, size = ${CODE_END - CODE_START:04X}, '
        'fill = yes, fillval = $FF, file = %O; }\n'
        'SEGMENTS { CODE: load = CODE, type = ro; }\n')
    obj = os.path.join(build, 'wide.o')
    binary = os.path.join(build, 'wide.bin')
    labels = os.path.join(build, 'wide.labels')
    subprocess.run(tool('ca65') + ['-g', wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)],
                   check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-o', wsl_path(binary), '-Ln', wsl_path(labels),
                                   wsl_path(obj)], check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return open(binary, 'rb').read(), sym


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Mega Man 2 (USA)'

    code, sym = assemble()
    bank = BANK * BANK_SIZE
    start = bank + CODE_START - FIXED
    assert all(b == 0xFF for b in data[start:start + len(code)]), 'the code space is not free'
    data[start:start + len(code)] = code

    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for address, original, new in hooks(sym):
        if address in skip:
            continue
        at = bank + address - FIXED
        assert data[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        data[at:at + len(new)] = new

    out = os.path.join(ROOT, 'build', 'mm2wide.nes')
    open(out, 'wb').write(bytes(image[:header]) + bytes(data))
    print(f'{out}: data CRC {zlib.crc32(data):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        open(sys.argv[2], 'wb').write(make_bps.make(bytes(image[header:]), bytes(data)))
        print(f'{sys.argv[2]}: written')


if __name__ == '__main__':
    main()
```

### C. `tools/make_bps.py`

```python
"""Writes a BPS patch from an original ROM to the widescreen build, both without their iNES header.

Usage: python -I tools/make_bps.py <original.nes> <build/z2wide.nes> <out.bps>

Only the bytes the hack changes travel in the patch (TargetRead); everything else is copied from
the player's own ROM (SourceRead), so the patch carries no data of the game.
"""

import sys
import zlib


def number(value):
    out = bytearray()
    while True:
        low = value & 0x7F
        value >>= 7
        if value == 0:
            out.append(0x80 | low)
            return bytes(out)
        out.append(low)
        value -= 1


def body(path):
    data = open(path, 'rb').read()
    return data[16:] if data[:4] == b'NES\x1a' else data


def make(source, target):
    patch = bytearray(b'BPS1')
    patch += number(len(source)) + number(len(target)) + number(0)
    position = 0
    while position < len(target):
        same = position < len(source) and source[position] == target[position]
        end = position
        while end < len(target) and (end < len(source) and source[end] == target[end]) == same:
            end += 1
        length = end - position
        if same:
            patch += number(((length - 1) << 2) | 0)
        else:
            patch += number(((length - 1) << 2) | 1) + target[position:end]
        position = end
    patch += zlib.crc32(source).to_bytes(4, 'little')
    patch += zlib.crc32(target).to_bytes(4, 'little')
    patch += zlib.crc32(bytes(patch)).to_bytes(4, 'little')
    return bytes(patch)


def main():
    source = body(sys.argv[1])
    target = body(sys.argv[2])
    patch = make(source, target)
    open(sys.argv[3], 'wb').write(patch)
    print(f'{sys.argv[3]}: {len(patch)} bytes, source crc {zlib.crc32(source):08X}, '
          f'target crc {zlib.crc32(target):08X}')


if __name__ == '__main__':
    main()
```
