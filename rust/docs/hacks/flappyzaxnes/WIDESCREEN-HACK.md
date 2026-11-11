# FlappyZaxNes: the widescreen hack

This document describes completely the widescreen of *FlappyZaxNes*, the game that ships with
Zaxnes (© 2026 Nixxou, after *Flappy NES* by filipemd, GPL-3.0). The game itself has no
widescreen code: as for the other games, the widescreen is a hack pack Zaxnes applies to the ROM
in memory. The appendices hold the hack's code and its build script, which places every hook
after checking the bytes it replaces.

The background of the margins needs no decoder: the game keeps both screens of its world in the
nametables (vertical mirroring, a 512-pixel ring) and draws each new pipe 352 pixels ahead of its
scroll, so the extension shows the margins from the nametables as they are.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *FlappyZaxNes*, NROM-256 (mapper 0), 32 KiB PRG, 8 KiB CHR, vertical mirroring; built with cc65 from the game's sources (`rust/crates/zax-app/demo/source`) |
| Source data (iNES header excluded) | CRC-32 `08270EB8` |
| Pack | `rust/crates/zax-nes/patches/flappyzaxnes-widescreen.zxpk` (443 bytes, 256 of code), format 2, NTSC, mapper 0, portable states |
| Zaxnes declaration | `builtin_patches.rs`: `FLAPPYZAXNES`, `advanced: false` (the simple mode applies it: the ROM is not otherwise widened), `boot_zone: true`, `zone_data: true`, `idle_loop: Some((0x85AB, 0x85AD))` (the game's wait for the NMI, `LDA NMIReady / BEQ`) |

The pack's code is assembled for `$2000`: the zone fantôme, which Zaxnes shows to instruction
fetches only, from power-on; the extension's registers and RAM are at `$3000`-`$3FFF` (widescreen
MMC5). The game is NROM: nothing is added to the ROM, and the hooks are its only change.

## 2. What the game does, and what changes

The game scrolls one pixel a frame over its 512-pixel ring. Its sprites are, in the OAM page: the
bird (7 sprites), the score (2), the caps of the moving pipes (three groups of 8, `$0224`-`$0283`;
the pipes' bodies are background) and the clouds (30, from `$0284`), all placed by the main loop
from their x on the ring; a column whose x is 256 or more is hidden (y `$FF`).

| Part | The game | The hack |
|---|---|---|
| NMI (sprite DMA) | `LDA #>$0200 / STA $4014` | the margin each frame, `min($3002, 80)` written to `$3003`, then the DMA |
| a column of a moving pipe's caps off the screen | hides its two sprites | a column within the wide view (`-M-8 < x < 256+M`) shows them, the high byte of its x (`$01` right of the screen, `$FF` left of it) written to `$3100` + each sprite's OAM offset; any other is hidden as the game does |
| a cloud column off the screen | hides its two sprites | the same, for the cloud's column |

M is the margin of the frame (`WV_M`, at most 80: a new pipe is drawn 352 pixels ahead, past the
right margin). With M = 0 every column off the screen is hidden as the game does, except that a
column across the left edge shows its part on the screen (the extension draws it). Without the
hack, the game keeps the caps of a moving pipe across the left edge at x 0.

## 3. Variables (extension RAM)

`WV_M $3200` the margin of this frame.

## 4. Hooks

Each is a bank and a CPU address with the game's bytes it replaces, checked by Zaxnes; NOP (`$EA`)
fill to the original length.

| Group | Address | Replaces | With |
|---|---|---|---|
| core | `$820F` | `LDA #>$0200 / STA $4014` (the NMI's sprite DMA) | `JSR wide_nmi`: the margin, then the DMA |
| core | `$885D` | `LDA #$FF / STA $0224,Y / STA $0228,Y` (`UpdateMover` hiding a column of a moving pipe's caps) | `JSR wide_mover_column` |
| clouds | `$825D` | `LDA #$FF / STA $0200,Y / STA $0204,Y` (`UpdateClouds` hiding a cloud column) | `JSR wide_cloud_column` |

The groups: "core" (Base: the margin, and the caps of the moving pipes, without which the margins
would show those pipes capless) and "clouds" (Clouds in the margins, needing it).

## 5. Building

Requirements: cc65 (ca65, ld65 on the PATH), Python 3, Zaxnes' `rust/tools` (`hackpack.py`).

```
make                                   # FlappyZaxNes.nes and build.lbl
ZAXNES_TOOLS=<zaxnes>/rust/tools python -I widescreen/build.py FlappyZaxNes.nes build.lbl flappyzaxnes-widescreen.zxpk
python -I tools/make_doc.py FlappyZaxNes.nes build.lbl <zaxnes>/rust/docs/hacks/flappyzaxnes/WIDESCREEN-HACK.md
```

`build.py` finds the hooks from the build's labels (`ld65 -Ln`), assembles `widescreen/wide.s`
for `$2000` with the game's temporaries' addresses, and writes the pack; it prints the data's
CRC-32, which goes into `builtin_patches.rs`.

## 6. Limits

* A pipe is drawn 352 pixels ahead of the scroll, so the margin is held to 80 pixels.
* The clouds' and caps' columns show only within the margin; past it they are hidden as on the
  console.

## Appendices

### A. `widescreen/wide.s`

```asm
; FlappyZaxNes: the widescreen hack (Zaxnes), a hack pack.
;
; The game is left as it is: this code runs in the zone fantôme ($2000, widescreen MMC5: the
; extension's registers and RAM at $3000-$3FFF for these instructions), reached through two
; hooks Zaxnes writes into the game (widescreen/build.py). The game keeps both screens of its
; background in the nametables (vertical mirroring, a 512-pixel ring) and draws a new pipe 352
; pixels ahead of its scroll, so the margins show its own world from the nametables, no decoder:
;
;  * the margin asked for is the host's, at most 80 pixels (a new pipe is drawn past 344);
;  * the clouds, sprites, show in the margins: the game hides a cloud column off the screen,
;    the hack gives it the high byte of its x instead when it falls within the wide view;
;  * so do the caps of the moving pipes (from the tenth pipe on, one in two; their bodies are
;    background, their caps sprites), part of the base: without them the margins would show
;    them capless.
;
; With a margin of 0 every routine does what the code it replaces does, except that a cloud
; column across the left edge shows its part on the screen (the extension draws it).
;
; The game's symbols (TEMP0 to TEMP3) are given on the command line from its labels.

EXT             = $3000         ; the extension's registers and RAM, in the zone fantôme
WIDE_HOST       = EXT + $002    ; R: the margin the host offers
WIDE_REQ        = EXT + $003    ; W: the margin asked for
WIDE_SPRHI      = EXT + $100    ; + 4 n: high byte of the x of sprite n, for the next OAM DMA
WV_M            = EXT + $200    ; the margin of this frame (extension RAM)

MAX_MARGIN      = 80
SPRITE_BUFFER   = $0200
OAMDMA          = $4014

.segment "CODE"
.org $2000

; ---------------------------------------------------------------------------- ;
; NMI (in place of LDA #>SPRITE_BUFFER / STA OAMDMA): the margin, then the sprites' DMA.
wide_nmi:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    LDA     #>SPRITE_BUFFER
    STA     OAMDMA
    RTS

; ---------------------------------------------------------------------------- ;
; A cloud column off the screen (in place of the game's hiding of its two sprites): the game
; has its x, 256 to 511 on the ring, in TEMP1:TEMP0, its y in TEMP2, and Y its first sprite's
; offset in the sprite page. Within the wide view the column shows, the high byte of its x 1
; (right margin) or $FF (left of the screen); otherwise it is hidden as the game does.
wide_cloud_column:
    LDA     TEMP0
    CMP     WV_M
    BCC     @right                      ; 256 + low < 256 + M
    CLC                                 ; left: x - 512 + 8 > -M, low + 8 + M > 256
    ADC     #$08
    BCS     @left
    ADC     WV_M
    BCC     @hide
    BEQ     @hide                       ; exactly 256: past the view
@left:
    LDA     #$FF
    BNE     @show
@right:
    LDA     #$01
@show:
    STA     WIDE_SPRHI,y
    STA     WIDE_SPRHI+4,y
    LDA     TEMP2
    STA     SPRITE_BUFFER,y
    CLC
    ADC     #$08
    STA     SPRITE_BUFFER+4,y
    LDA     TEMP0
    STA     SPRITE_BUFFER+3,y
    STA     SPRITE_BUFFER+7,y
    RTS
@hide:
    LDA     #$FF
    STA     SPRITE_BUFFER,y
    STA     SPRITE_BUFFER+4,y
    RTS

; ---------------------------------------------------------------------------- ;
; A column of a moving pipe's caps off the screen (in place of the game's hiding of its two
; sprites, at MOVER_OAM + Y, the three groups one after the other): its x in TEMP1:TEMP0 as a cloud's, the y of the upper pipe's cap in
; TEMP2, of the lower pipe's in TEMP3.
MOVER_OAM       = SPRITE_BUFFER + 9 * 4

wide_mover_column:
    LDA     TEMP0
    CMP     WV_M
    BCC     @right
    CLC
    ADC     #$08
    BCS     @left
    ADC     WV_M
    BCC     @hide
    BEQ     @hide
@left:
    LDA     #$FF
    BNE     @show
@right:
    LDA     #$01
@show:
    STA     WIDE_SPRHI+9*4,y
    STA     WIDE_SPRHI+9*4+4,y
    LDA     TEMP2
    STA     MOVER_OAM,y
    LDA     TEMP3
    STA     MOVER_OAM+4,y
    LDA     TEMP0
    STA     MOVER_OAM+3,y
    STA     MOVER_OAM+7,y
    RTS
@hide:
    LDA     #$FF
    STA     MOVER_OAM,y
    STA     MOVER_OAM+4,y
    RTS

wide_end:
.assert wide_end <= $3000, error, "widescreen code past the zone's code half"
```

### B. `widescreen/build.py`

```python
"""Builds the Zaxnes hack pack of FlappyZaxNes: its widescreen.

python -I widescreen/build.py FlappyZaxNes.nes build.lbl flappyzaxnes-widescreen.zxpk
Needs ZAXNES_TOOLS (the rust/tools folder of Zaxnes, its hackpack.py) and ca65/ld65 on the PATH.
build.lbl: the labels of that build of the game (ld65 -Ln), for the hooks and the game's RAM.

The game is not changed: widescreen/wide.s is assembled for $2000, the zone fantôme (widescreen
MMC5), and two hooks reach it, checked against the bytes they replace:
  * core: the NMI's sprite DMA (LDA #>$0200 / STA $4014), where the margin is set each frame;
  * core also: the hiding of a column of a moving pipe's caps off the screen (src/mover.s),
    shown in the margins instead;
  * clouds (needing core): the hiding of a cloud column off the screen, where the hack shows it
    in the margins instead.
"""
import os
import subprocess
import sys
import zlib

PRG_SIZE = 0x8000
CHR_SIZE = 0x2000
PACK_START = 0x2000
HERE = os.path.dirname(os.path.abspath(__file__))
NOP = 0xEA


def labels(path):
    out = {}
    for line in open(path):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            out.setdefault(parts[2].lstrip('.'), int(parts[1], 16))
    return out


def assemble(game, build):
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    open(cfg, 'w', newline='\n').write(
        f'MEMORY {{ CODE: start = ${PACK_START:04X}, size = $1000, fill = yes, fillval = $FF, '
        'file = %O; }\nSEGMENTS { CODE: load = CODE, type = ro; }\n')
    obj, binary, lbl = (os.path.join(build, n) for n in ('wide.o', 'wide.bin', 'wide.lbl'))
    defines = []
    for name in ('Temp0', 'Temp1', 'Temp2', 'Temp3'):
        defines += ['-D', f'{name.upper()}={game[name]}']
    subprocess.run(['ca65', '-g'] + defines + [os.path.join(HERE, 'wide.s'), '-o', obj], check=True)
    subprocess.run(['ld65', '-C', cfg, '-o', binary, '-Ln', lbl, obj], check=True)
    return open(binary, 'rb').read(), labels(lbl)


def find(prg, start, pattern, within=64):
    """The CPU address of `pattern` in the code from `start` on (the game's NROM-256 at $8000)."""
    offset = prg.find(pattern, start - 0x8000, start - 0x8000 + max(within, len(pattern)))
    assert offset >= 0, f'{pattern.hex(" ")} not found after ${start:04X}'
    return 0x8000 + offset


def jsr(target, length):
    return bytes([0x20, target & 0xFF, target >> 8]) + bytes([NOP]) * (length - 3)


def main():
    rom, lbl, out = sys.argv[1:4]
    tools = os.environ.get('ZAXNES_TOOLS')
    if not tools or not os.path.isfile(os.path.join(tools, 'hackpack.py')):
        sys.exit('ZAXNES_TOOLS: the rust/tools folder of Zaxnes (its hackpack.py) is needed')
    sys.path.insert(0, tools)
    import hackpack

    image = open(rom, 'rb').read()
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    prg = data[:PRG_SIZE]
    game = labels(lbl)
    code, sym = assemble(game, os.path.join(HERE, 'build'))
    used = (sym['wide_end'] - PACK_START + 0xFF) & ~0xFF

    # NMI: LDA #>$0200 / STA $4014
    dma = bytes.fromhex('a9 02 8d 14 40')
    nmi = find(prg, game['nmi'], dma)
    # UpdateClouds' hiding of a column: LDA #$FF / STA $0200,Y / STA $0204,Y
    hide_bytes = bytes.fromhex('a9 ff 99 00 02 99 04 02')
    hide = find(prg, game['hide'], hide_bytes, within=len(hide_bytes))
    # UpdateMover's hiding of a column: LDA #$FF / STA $0224,Y / STA $0228,Y
    mover_bytes = bytes.fromhex('a9 ff 99 24 02 99 28 02')
    mover = find(prg, game['UpdateMover'], mover_bytes, within=256)

    pack = hackpack.Pack('FlappyZaxNes widescreen', code[:used], region='ntsc', mappers=(0,))
    widescreen = pack.component('widescreen')
    core = widescreen.group('core', label='Base', label_fr='Base')
    clouds = widescreen.group('clouds', requires=['core'], label='Clouds in the margins',
                              label_fr='Nuages dans les marges')

    def place(address):
        offset = address - 0x8000
        return offset // 0x4000, (address & 0xC000) | (offset & 0x3FFF)

    core.hook(*place(nmi), dma, jsr(sym['wide_nmi'], len(dma)))
    core.hook(*place(mover), mover_bytes, jsr(sym['wide_mover_column'], len(mover_bytes)))
    clouds.hook(*place(hide), hide_bytes, jsr(sym['wide_cloud_column'], len(hide_bytes)))
    result = pack.build(bytes(data), PRG_SIZE)
    open(out, 'wb').write(result)
    # the wait for the next frame (LDA NMIReady / BEQ MainLoop), for builtin_patches.rs' idle_loop
    loop = game['MainLoop']
    branch = loop + (2 if prg[loop - 0x8000] == 0xA5 else 3)
    assert prg[branch - 0x8000] == 0xF0, 'MainLoop: LDA NMIReady / BEQ expected'
    print(f'{out}: {len(result)} bytes ({used} of code), data CRC {zlib.crc32(data):08X}, '
          f'pack CRC {zlib.crc32(result):08X}, idle_loop (0x{loop:04X}, 0x{branch:04X})')


main()
```
