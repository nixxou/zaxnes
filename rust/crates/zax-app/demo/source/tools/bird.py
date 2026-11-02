"""Writes the hand-drawn bird of tools/bird_art.py into char.chr and src/bird_frames.inc, and
regenerates assets/spritesheet.png.

python tools/bird.py
The body is 3x2 sprites (palette 0: outline, yellow, white), tiles $30-$65, six per pose and
wing position (top row, then bottom row). The beak is one more sprite over its red
(palette 3: outline, red), tiles $68-$6A, one per pose, placed by BeakDX/BeakDY.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from bird_art import pose  # noqa: E402

POSES = ('level', 'up', 'down')
BODY, BEAK = 0x30, 0x68
COLUMNS = 3  # the body is 3x2 sprites (24x16)
PER = COLUMNS * 2
BODY_VALUE = {'k': 1, 'y': 2, 'l': 3, 'w': 3}
BEAK_VALUE = {'k': 1, 'r': 2}


def main():
    data = bytearray(open('char.chr', 'rb').read())

    def put(tile, x, y, v):
        o = tile * 16
        bit = 1 << (7 - x)
        data[o + y] = (data[o + y] & ~bit) | (bit if v & 1 else 0)
        data[o + 8 + y] = (data[o + 8 + y] & ~bit) | (bit if v & 2 else 0)

    windows = []
    for p, name in enumerate(POSES):
        art = pose(name, 0)
        reds = [(x, y) for y, row in enumerate(art) for x, c in enumerate(row) if c == 'r']
        xs = [x for x, _ in reds]
        ys = [y for _, y in reds]
        bx, by = min(xs), min(ys)
        assert max(xs) - bx < 8 and max(ys) - by < 8, (name, bx, by, max(xs), max(ys))
        windows.append((bx, by))
        # The beak sprite: the red and the outline inside its window.
        for y in range(8):
            for x in range(8):
                c = art[by + y][bx + x] if by + y < 16 and bx + x < len(art[0]) else '.'
                put(BEAK + p, x, y, BEAK_VALUE.get(c, 0))
        for wing in range(3):
            art = pose(name, wing)
            for y in range(16):
                for x in range(8 * COLUMNS):
                    c = art[y][x] if x < len(art[0]) else '.'
                    tile = BODY + (p * 3 + wing) * PER + (y // 8) * COLUMNS + x // 8
                    put(tile, x % 8, y % 8, BODY_VALUE.get(c, 0))
    open('char.chr', 'wb').write(data)

    lines = ['; Written by tools/bird.py from tools/bird_art.py.', '',
             '; Body tiles (top row left to right, then bottom row) by pose and wing',
             'BirdFrames:']
    for p in range(3):
        row = ',  '.join(', '.join(f'${BODY + (p * 3 + w) * PER + k:02X}' for k in range(PER))
                         for w in range(3))
        lines.append(f'\t.byte {row}')
    lines += ['; The beak sprite of each pose: tile, and offset from the body',
              'BeakTile: .byte ' + ', '.join(f'${BEAK + p:02X}' for p in range(3)),
              'BeakDX: .byte ' + ', '.join(f'{x}' for x, _ in windows),
              'BeakDY: .byte ' + ', '.join(f'{y}' for _, y in windows), '']
    open('src/bird_frames.inc', 'w').write('\n'.join(lines))

    from PIL import Image
    sheet = Image.open('assets/spritesheet.png')
    for tile in range(512):
        o = tile * 16
        for y in range(8):
            for x in range(8):
                v = ((data[o + y] >> (7 - x)) & 1) | (((data[o + 8 + y] >> (7 - x)) & 1) << 1)
                sheet.putpixel(((tile % 16) * 8 + x, (tile // 16) * 8 + y), v)
    sheet.save('assets/spritesheet.png')
    print('beak windows', windows)


main()
