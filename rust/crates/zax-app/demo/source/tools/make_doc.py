"""Writes the document of FlappyZaxNes' widescreen hack for Zaxnes (rust/docs/hacks/flappyzaxnes/
WIDESCREEN-HACK.md) from this repository: the text below, the hooks found in the build, and the
hack's sources as appendices.

python -I tools/make_doc.py FlappyZaxNes.nes build.lbl <zaxnes>/rust/docs/hacks/flappyzaxnes/WIDESCREEN-HACK.md
"""
import os
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
PRG_SIZE, CHR_SIZE = 0x8000, 0x2000

HOOKS = [
    ('core', 'nmi', 'a9 02 8d 14 40', 64, '`LDA #>$0200 / STA $4014` (the NMI\'s sprite DMA)',
     '`JSR wide_nmi`: the margin, then the DMA'),
    ('core', 'UpdateMover', 'a9 ff 99 24 02 99 28 02', 256,
     '`LDA #$FF / STA $0224,Y / STA $0228,Y` (`UpdateMover` hiding a column of a moving pipe\'s '
     'caps)', '`JSR wide_mover_column`'),
    ('clouds', 'hide', 'a9 ff 99 00 02 99 04 02', 8,
     '`LDA #$FF / STA $0200,Y / STA $0204,Y` (`UpdateClouds` hiding a cloud column)',
     '`JSR wide_cloud_column`'),
]

BODY = """# FlappyZaxNes: the widescreen hack

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
| Source data (iNES header excluded) | CRC-32 `{crc}` |
| Pack | `rust/crates/zax-nes/patches/flappyzaxnes-widescreen.zxpk` ({pack_size} bytes, {code_size} of code), format 2, NTSC, mapper 0 |
| Zaxnes declaration | `builtin_patches.rs`: `FLAPPYZAXNES`, `advanced: false` (the simple mode applies it: the ROM is not otherwise widened), `boot_zone: true`, `zone_data: true`, `idle_loop: Some((0x{loop:04X}, 0x{branch:04X}))` (the game's wait for the NMI, `LDA NMIReady / BEQ`) |

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
{hooks}

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
{wide}```

### B. `widescreen/build.py`

```python
{build}```
"""


def labels(path):
    out = {}
    for line in open(path):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            out.setdefault(parts[2].lstrip('.'), int(parts[1], 16))
    return out


def main():
    rom, lbl, out = sys.argv[1:4]
    image = open(rom, 'rb').read()
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    prg = data[:PRG_SIZE]
    game = labels(lbl)
    rows = []
    for group, label, pattern, within, replaces, by in HOOKS:
        pattern = bytes.fromhex(pattern)
        start = game[label] - 0x8000
        offset = prg.find(pattern, start, start + max(within, len(pattern)))
        assert offset >= 0, label
        rows.append(f'| {group} | `${0x8000 + offset:04X}` | {replaces} | {by} |')
    pack = os.path.join(ROOT, 'widescreen', 'flappyzaxnes-widescreen.zxpk')
    wide = open(os.path.join(ROOT, 'widescreen', 'wide.s'), encoding='utf-8').read()
    code_end = labels(os.path.join(ROOT, 'widescreen', 'build', 'wide.lbl'))['wide_end']
    loop = game['MainLoop']
    branch = loop + (2 if prg[loop - 0x8000] == 0xA5 else 3)
    text = BODY.format(loop=loop, branch=branch,
        crc=f'{zlib.crc32(data):08X}', pack_size=os.path.getsize(pack),
        code_size=(code_end - 0x2000 + 0xFF) & ~0xFF, hooks='\n'.join(rows), wide=wide,
        build=open(os.path.join(ROOT, 'widescreen', 'build.py'), encoding='utf-8').read())
    os.makedirs(os.path.dirname(out), exist_ok=True)
    open(out, 'w', encoding='utf-8', newline='\n').write(text)
    print(out)


main()
