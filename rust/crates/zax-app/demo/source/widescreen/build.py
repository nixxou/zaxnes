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
    # The hack writes only its margins (the sprite list, the extension): a state made at the wait
    # for the next frame goes on the same with or without it (zax-nes portable_state_tests.rs).
    pack.states(portable=True)
    result = pack.build(bytes(data), PRG_SIZE)
    open(out, 'wb').write(result)
    # the wait for the next frame (LDA NMIReady / BEQ MainLoop), for builtin_patches.rs' idle_loop
    loop = game['MainLoop']
    branch = loop + (2 if prg[loop - 0x8000] == 0xA5 else 3)
    assert prg[branch - 0x8000] == 0xF0, 'MainLoop: LDA NMIReady / BEQ expected'
    print(f'{out}: {len(result)} bytes ({used} of code), data CRC {zlib.crc32(data):08X}, '
          f'pack CRC {zlib.crc32(result):08X}, idle_loop (0x{loop:04X}, 0x{branch:04X})')


main()
