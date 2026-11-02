"""Draws the background of Flappy NES (sky, clouds, sun, city, bushes, ground) and the pipes,
and writes them into char.chr (second pattern table) and src/background.bin
(two screens of 960 bytes of nametable, then 64 of attributes), and regenerates assets/spritesheet.png.

python tools/decor.py [preview.png]

The background palettes (src/palletes.s): pale sky $31 shared;
  0 sky:    1 white (clouds, sun), 2 the sky again (windows), 3 city blue
  1 pipes:  1 dark green, 2 green, 3 lime
  2 bushes: 1 dark green, 2 green, 3 city blue (the city goes on behind them)
  3 ground: 1 dark green, 2 grass, 3 dirt
The pipes move over the scenery (src/pipes.s writes them and puts it back); their gaps stay
above the city (see GenerateNewPipe) and show the sky. The pipe tiles are $B0-$B3 (body), $B4-$B7 (cap on top of the lower pipe) and $B8-$BB
(cap under the upper pipe), four tiles across.
"""
import math
import random
import sys

W, H = 256, 240
PIPE_COL = 28  # where the preview draws a pipe
BODY, TOP_CAP, BOTTOM_CAP = 0xB0, 0xB4, 0xB8
CHR_BG = 4096
# Tiles the decor may use: the font ($20-$7E, $A9) and the logo ($D0-$FF, tools/logo.py) stay.
POOL = ([t for t in range(0x01, 0x20)] + [t for t in range(0x80, 0xB0) if t != 0xA9]
        + [t for t in range(0xBC, 0xD0)] + [0x7F])

SKY_PAL, PIPE_PAL, BUSH_PAL, GROUND_PAL = 0, 1, 2, 3
BUSH_ROW, GROUND_ROW = 24, 26


def palette_of(col16, row16):
    """The palette of the 16x16 block at (col16, row16)."""
    if row16 * 2 >= GROUND_ROW:
        return GROUND_PAL
    if row16 * 2 >= BUSH_ROW:
        return BUSH_PAL
    return SKY_PAL


# The two screens side by side differ: clouds and city groups (left x, [(width, top row)]).
SCREENS = [
    {'clouds': (),
     'city': ((0, ((16, 20), (24, 18), (16, 19))),
              (72, ((24, 19), (16, 21), (32, 18), (16, 20))),
              (176, ((16, 19), (24, 18))))},
    {'clouds': (),
     'city': ((112, ((16, 21), (24, 19), (16, 20))),)},
]


def draw(screen):
    img = [[0] * W for _ in range(H)]
    rng = random.Random(7)

    def disc(cx, cy, r, v, only_on=None, y_max=H):
        for y in range(max(0, int(cy - r)), min(y_max, int(cy + r) + 1)):
            for x in range(max(0, int(cx - r)), min(W, int(cx + r) + 1)):
                if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                    if only_on is None or img[y][x] in only_on:
                        img[y][x] = v

    # The sun: a white disc in a pale halo.

    # Clouds: white puffs with a pale underside, away from the logo of the title.
    def cloud(cx, cy, size):
        puffs = [(-1.6, 0.3, 0.55), (-0.8, -0.2, 0.75), (0.1, -0.45, 0.9), (1.0, -0.1, 0.7),
                 (1.7, 0.3, 0.5)]
        for dx, dy, r in puffs:
            disc(cx + dx * size, cy + dy * size, r * size, 2)
        for dx, dy, r in puffs:
            disc(cx + dx * size, cy + dy * size - 2, r * size - 1, 1)
        for y in range(int(cy + size * 0.35), int(cy + size)):
            for x in range(W):
                if img[y][x] in (1, 2) and abs(x - cx) < size * 2.5:
                    img[y][x] = 0
    for cx, cy, size in screen['clouds']:
        cloud(cx, cy, size)

    # The city: groups of buildings side by side, of different widths and heights, with empty
    # lots between the groups. Each group: (left x, [(width, top row), ...]), on the tile grid.
    for x, buildings in screen['city']:
        for n, (width, top_row) in enumerate(buildings):
            top = top_row * 8
            for y in range(top, GROUND_ROW * 8):
                for xx in range(x, x + width):
                    img[y][xx] = 3
            # windows: two 2x2 sky-coloured dots per tile, below a plain top band
            for y in range(top + 8, BUSH_ROW * 8, 8):
                for xx in range(x, x + width, 8):
                    for wx in (2, 5):
                        for dy in (2, 3, 5, 6):
                            for dx in (0, 1):
                                img[y + dy][xx + wx + dx] = 2
            # the base of the building, one row above the bushes: a pale line marks its own
            # tiles, which the 3D profile stretches down to the ground behind the bushes
            for xx in range(x, x + width):
                img[BUSH_ROW * 8 - 1][xx] = 2
            # a thin seam against the building before it
            if n:
                for y in range(top, BUSH_ROW * 8):
                    img[y][x] = 2
            x += width

    # Bushes in front of the city: green mounds with a dark rim, lighter tops.
    # Two mound sizes, centred on the tile grid, in a repeating pattern: few distinct tiles.
    mounds = []
    x = 8
    for r in (17, 11, 14, 11, 17, 11, 14, 11, 17):
        mounds.append((x, GROUND_ROW * 8 + 4, r))
        x += 24 if r == 11 else 32
    # the screens follow one another: a mound across an edge goes on on the other side
    mounds += [(cx - W, cy, r) for cx, cy, r in mounds] + [(cx + W, cy, r) for cx, cy, r in mounds]
    for cx, cy, r in mounds:
        disc(cx, cy, r + 1, 1, y_max=GROUND_ROW * 8)
    for cx, cy, r in mounds:
        disc(cx, cy, r, 2, y_max=GROUND_ROW * 8)
    # Below the bush line the city has no windows (palette 2 has no pale blue).
    for y in range(BUSH_ROW * 8, GROUND_ROW * 8):
        for xx in range(W):
            if palette_of(xx // 16, y // 16) == BUSH_PAL and img[y][xx] == 2 \
                    and not any(abs(xx - cx) <= r and y >= cy - r for cx, cy, r in mounds):
                img[y][xx] = 3

    # The ground: a dark rim, grass with a wavy edge, then dirt; repeats every 16 pixels.
    g = GROUND_ROW * 8
    for y in range(g, H):
        for xx in range(W):
            d = y - g
            wave = 7 + (1 if (xx // 4) % 2 == 0 else 0) + (1 if xx % 16 in (5, 6, 7) else 0)
            # The top line is grass (the top face of the ground in 3D), the dark rim under it.
            if d == 1:
                v = 1
            elif d < wave:
                v = 2
            elif d == wave:
                v = 1
            else:
                v = 3
                if (xx + 2 * d) % 16 == 0 and d > 12:
                    v = 1
            img[y][xx] = v
    return img


def pipe_tiles():
    """Rows of 32 pixels for the body and the two caps."""
    # The body: dark rims, a lime highlight on the left, a shadow on the right.
    body = '..1223332322222222222212121111..'
    cap = '1122333323222222222222221212111' + '1'
    body_rows = [body] * 8
    top = ['1' * 32] + [cap] * 6 + ['1' * 32]       # on top of the lower pipe
    bottom = ['1' * 32] + [cap] * 6 + ['1' * 32]    # under the upper pipe
    top[1] = '1' + '3' * 30 + '1'                     # the lit rim of the opening
    bottom[6] = '1' + '2' * 30 + '1'
    return {BODY: body_rows, TOP_CAP: top, BOTTOM_CAP: bottom}


def main():
    images = [draw(screen) for screen in SCREENS]
    data = bytearray(open('char.chr', 'rb').read())

    def encode(pixels):
        out = bytearray(16)
        for y in range(8):
            for x in range(8):
                v = pixels[y][x]
                if v & 1:
                    out[y] |= 0x80 >> x
                if v & 2:
                    out[8 + y] |= 0x80 >> x
        return bytes(out)

    for base, rows in pipe_tiles().items():
        for t in range(4):
            px = [[int(c) if c != '.' else 0 for c in row[t * 8:t * 8 + 8]] for row in rows]
            data[CHR_BG + (base + t) * 16:CHR_BG + (base + t + 1) * 16] = encode(px)

    tiles = {bytes(16): 0x00}
    count = {encode([img[ty * 8 + y][tx * 8:tx * 8 + 8] for y in range(8)])
             for img in images for ty in range(30) for tx in range(32)}
    print(len(count) - 1, 'distinct decor tiles,', len(POOL), 'available')
    pool = iter(POOL)
    attributes = bytearray()
    for ay in range(8):
        for ax in range(8):
            byte = 0
            for q, (dx, dy) in enumerate(((0, 0), (1, 0), (0, 1), (1, 1))):
                row16 = ay * 2 + dy
                byte |= palette_of(ax * 2 + dx, min(row16, 14)) << (q * 2)
            attributes.append(byte)
    screens = bytearray()
    for img in images:
        for ty in range(30):
            for tx in range(32):
                key = encode([img[ty * 8 + y][tx * 8:tx * 8 + 8] for y in range(8)])
                if key not in tiles:
                    tiles[key] = next(pool)
                    tid = tiles[key]
                    data[CHR_BG + tid * 16:CHR_BG + tid * 16 + 16] = key
                screens.append(tiles[key])
        screens += attributes
    open('char.chr', 'wb').write(data)
    open('src/background.bin', 'wb').write(screens)

    from PIL import Image
    sheet = Image.open('assets/spritesheet.png')
    for tile in range(512):
        o = tile * 16
        for y in range(8):
            for x in range(8):
                v = ((data[o + y] >> (7 - x)) & 1) | (((data[o + 8 + y] >> (7 - x)) & 1) << 1)
                sheet.putpixel(((tile % 16) * 8 + x, (tile // 16) * 8 + y), v)
    sheet.save('assets/spritesheet.png')
    print(len(tiles) - 1, 'decor tiles of', len(POOL))

    if len(sys.argv) > 1:
        nes = {0x21: (0x64, 0xB0, 0xFF), 0x30: (0xFF, 0xFE, 0xFF), 0x31: (0xC0, 0xDF, 0xFF),
               0x11: (0x15, 0x5F, 0xD9), 0x3C: (0xB5, 0xEB, 0xF2), 0x09: (0x0B, 0x48, 0x00), 0x1A: (0x0C, 0x93, 0x00),
               0x29: (0x88, 0xD8, 0x00), 0x2A: (0x5C, 0xE4, 0x30), 0x27: (0xEA, 0x9E, 0x22)}
        pals = [(0x31, 0x30, 0x31, 0x21), (0x31, 0x09, 0x1A, 0x29), (0x31, 0x09, 0x1A, 0x21),
                (0x31, 0x09, 0x2A, 0x27)]
        out = Image.new('RGB', (W * 2, H))
        for n, img in enumerate(images):
            for y in range(H):
                for x in range(W):
                    out.putpixel((n * W + x, y), nes[pals[palette_of(x // 16, y // 16)][img[y][x]]])
        # a pipe for the look
        rows = pipe_tiles()
        gap = 9
        for ty in range(26):
            kind = BODY if ty not in (gap, gap + 5) else (BOTTOM_CAP if ty == gap else TOP_CAP)
            if gap < ty < gap + 5:
                continue
            for y in range(8):
                for x in range(32):
                    c = rows[kind][y][x]
                    if c != '.':
                        out.putpixel((224 + x, ty * 8 + y), nes[pals[1][int(c)]])
                        out.putpixel((W + 224 + x, ty * 8 + y), nes[pals[1][int(c)]])
        out.resize((W * 4, H * 2), Image.NEAREST).save(sys.argv[1])


main()
