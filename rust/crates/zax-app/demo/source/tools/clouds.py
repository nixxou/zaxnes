"""Draws the cloud sprites into char.chr and writes src/clouds_data.inc (positions and the
initial sprite entries, in src/clouds_sprites.inc), then regenerates assets/spritesheet.png.
Also copies the pipe caps ($B4-$BB of the background) to the sprites ($24-$2B): the caps of
the moving pipe (src/mover.s).

python tools/clouds.py

A big cloud is 4x2 sprites (32x16 pixels, tiles $70-$77 row by row), a small one 3x2 (24x16,
tiles $14-$19), in sprite palette 2 (1 white, 2 a pale shade). Clouds are sprites so that they
stay whole behind the pipes, in 2D (behind the background) and in 3D. Four clouds go round the
512-pixel ring, one of them small (the sprites: the bird 7, the score 2, the moving pipes' caps 24
(three groups), the clouds 30, of 64); two clouds 128 pixels apart never share a line, so that
with the bird a line never holds more than eight sprites.
"""

# kind: (first tile, columns, rows, puffs (x, y, radius))
KINDS = {
    'big': (0x70, 4, 2, [(7, 10, 5.5), (13, 7, 6.5), (21, 6.5, 6), (26, 10, 5), (16, 11, 5)]),
    'small': (0x14, 3, 2, [(6, 10, 5), (11, 7, 6), (17, 8, 5), (12, 11, 4.5)]),
}
CLOUDS = [(40, 40, False, 'big'), (192, 72, True, 'big'), (300, 44, True, 'small'),
          (448, 72, False, 'big')]
FIRST_SPRITE = 33  # after the bird (7), the score (2) and the moving pipes' caps (24)
PIPE_CAPS, SPRITE_CAPS = 0xB4, 0x24


def draw(cols, rows, puffs):
    w, h = cols * 8, rows * 8
    img = [[0] * w for _ in range(h)]

    def disc(cx, cy, r, v):
        for y in range(h):
            for x in range(w):
                if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                    img[y][x] = v

    for cx, cy, r in puffs:
        disc(cx, cy, r, 2)
    for cx, cy, r in puffs:
        disc(cx - 0.5, cy - 1.5, r - 1, 1)
    return img


def main():
    data = bytearray(open('char.chr', 'rb').read())
    for first, cols, rows, puffs in KINDS.values():
        store(data, first, cols, rows, draw(cols, rows, puffs))
    data[SPRITE_CAPS * 16:SPRITE_CAPS * 16 + 128] = data[4096 + PIPE_CAPS * 16:
                                                         4096 + PIPE_CAPS * 16 + 128]
    write(data)


def store(data, first, cols, rows, img):
    for ty in range(rows):
        for tx in range(cols):
            tile = first + ty * cols + tx
            out = bytearray(16)
            for y in range(8):
                for x in range(8):
                    v = img[ty * 8 + y][tx * 8 + x]
                    if v & 1:
                        out[y] |= 0x80 >> x
                    if v & 2:
                        out[8 + y] |= 0x80 >> x
            data[tile * 16:tile * 16 + 16] = out


def write(data):
    open('char.chr', 'wb').write(data)
    oam, offsets = FIRST_SPRITE * 4, []
    for _, _, _, kind in CLOUDS:
        offsets.append(oam)
        oam += KINDS[kind][1] * KINDS[kind][2] * 4
    assert oam <= 256

    lines = ['; Written by tools/clouds.py.', '',
             '; The clouds on the 512-pixel ring: x (two bytes), y, and the offset of their first',
             '; sprite in the sprite buffer, and its columns (each of two sprites).',
             'CloudXLo: .byte ' + ', '.join(f'${c[0] & 255:02X}' for c in CLOUDS),
             'CloudXHi: .byte ' + ', '.join(f'${c[0] >> 8:02X}' for c in CLOUDS),
             'CloudY: .byte ' + ', '.join(f'${c[1]:02X}' for c in CLOUDS),
             'CloudOam: .byte ' + ', '.join(f'${o:02X}' for o in offsets),
             'CloudCols: .byte ' + ', '.join(f'${KINDS[c[3]][1]:02X}' for c in CLOUDS),
             'CLOUD_COUNT = %d' % len(CLOUDS),
             f'SPRITES_USED = {oam // 4}', '']
    sprites = ['; The cloud sprites: hidden, tiles set (behind the background, palette 2).',
               '.macro CLOUD_SPRITES']
    for _, _, flip, kind in CLOUDS:
        first, cols, rows, _ = KINDS[kind]
        for col in range(cols):
            source = cols - 1 - col if flip else col
            for row in range(rows):
                attr = 0b00100010 | (0b01000000 if flip else 0)
                sprites.append(f'\t.byte $FF, ${first + row * cols + source:02X}, '
                               f'%{attr:08b}, $00')
    sprites.append('.endmacro')
    open('src/clouds_data.inc', 'w').write('\n'.join(lines) + '\n')
    open('src/clouds_sprites.inc', 'w').write('\n'.join(sprites) + '\n')

    from PIL import Image
    sheet = Image.open('assets/spritesheet.png')
    for tile in range(512):
        o = tile * 16
        for y in range(8):
            for x in range(8):
                v = ((data[o + y] >> (7 - x)) & 1) | (((data[o + 8 + y] >> (7 - x)) & 1) << 1)
                sheet.putpixel(((tile % 16) * 8 + x, (tile // 16) * 8 + y), v)
    sheet.save('assets/spritesheet.png')


main()
