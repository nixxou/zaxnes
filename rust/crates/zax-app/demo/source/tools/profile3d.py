"""Writes the description of the FlappyZaxNes 3D profile (for `zax-harness profile-build`) from
char.chr and src/background.bin, so that it follows the drawn scenery.

python tools/profile3d.py out.json
zax-harness profile-build out.json flappyzaxnes.3dn

Depths and offsets are in pixels; positive z goes away from the camera. Everything sits on
layer L3, set to the centre of the play plane, and is pushed back by its own offset:
  ground   a slab from the play plane to behind the city (everything stands on it)
  pipes    round (vertical cylinders) on the play plane, the bird flies through them
  bushes   behind the pipes, hiding the foot of the city, their tops swaying gently
  city     at the back of the ground and thin: background scenery
  clouds   just behind the city, a plain extrusion (the rounded one showed bands)
The bird is one 3x2 sprite pattern per pose and wing position, its beak one more, flat
with a little thickness.
"""
import json
import math
import sys

CHR = 'char.chr'
BACKGROUND = 'src/background.bin'
SOLID = 4                      # layer L3, set to the centre (see 'layers' below)

GROUND_ROW, BUSH_ROW, PIPE_COL = 26, 24, 28
PIPE_BODY, PIPE_TOP_CAP, PIPE_BOTTOM_CAP = 0xB0, 0xB4, 0xB8
BIRD_BODY, BIRD_BEAK = 0x30, 0x68

data = open(CHR, 'rb').read()
screens = open(BACKGROUND, 'rb').read()


def planes(table, tile):
    o = table * 4096 + tile * 16
    return [f'{int.from_bytes(data[o:o + 8], "little"):016x}',
            f'{int.from_bytes(data[o + 8:o + 16], "little"):016x}']


def pixels(table, tile):
    o = table * 4096 + tile * 16
    return [[((data[o + y] >> (7 - x)) & 1) | (((data[o + 8 + y] >> (7 - x)) & 1) << 1)
             for x in range(8)] for y in range(8)]


tiles, patterns = {}, []


def name(table, tile):
    key = f'{"s" if table == 0 else "b"}{tile:02x}'
    tiles[key] = planes(table, tile)
    return key


def add(grid, **look):
    patterns.append({'grid': grid, 'layer': SOLID, **look})


def sway(level):
    """The bush deformation of a tile `level` rows above the bottom of the bushes: x moves with
    sin(height * pi/8) * sin(1.6 t), continuous from one row to the next, still at the ground."""
    step = math.pi / 8
    half = math.pi / 2
    return {'amplitude': [0.22, 0.0, 0.0], 'pivot': [0.0, 0.0, 0.0],
            'speed': [0.0, step, 0.0, 1.6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            'offset': [half, level * step, half, 0.0, half, half, half, half,
                       half, half, half, half, half, half, half, half]}


# --- The scenery: every tile of both screens, by where it is. -------------------------------
seen = {}
for screen in range(2):
    base = screen * 1024
    for row in range(30):
        for col in range(32):
            tile = screens[base + row * 32 + col]
            if tile == 0 or tile in seen:
                continue
            values = {v for line in pixels(1, tile) for v in line}
            if row >= GROUND_ROW:
                # a slab from just in front of the play plane to just behind the city
                look = dict(depth=68, geometry=0, offset=[0, 0, 2])
            elif row >= BUSH_ROW and (1 in values or 2 in values):
                # a bush tile; where the city shows behind it (colour 3), that colour is left
                # out of the bush's volume: the city's own tiles stand behind
                look = dict(depth=12, geometry=0, offset=[0, 0, 22],
                            deform=sway(GROUND_ROW - 1 - row))
                if 3 in values and 0 not in values:
                    look['empty'] = 3
            elif 3 in values and row == BUSH_ROW - 1:
                # the base row of the city (its tiles carry a pale bottom line): stretched
                # down three times from its top, to the ground behind the bushes
                look = dict(depth=8, geometry=0, offset=[0, 0, 32], scale=[1, 3, 1],
                            pivot=[0.5, 1.0, 0.5])
            elif 3 in values:
                look = dict(depth=8, geometry=0, offset=[0, 0, 32])           # city
            else:
                look = dict(depth=8, geometry=0, offset=[0, 0, 56])           # cloud
            seen[tile] = look
            add([[name(1, tile)]], **look)

# --- The clouds: sprites (tools/clouds.py), just behind the city. --------------------------
# Every other cloud is mirrored: its tiles get a mirrored pattern too, or they would fall back
# to the sprites' default depth, near the play plane.
for tile in list(range(0x70, 0x78)) + list(range(0x14, 0x1A)):
    if any(data[tile * 16:tile * 16 + 16]):
        add([[name(0, tile)]], depth=8, geometry=0, offset=[0, 0, 56])
        mirrored = f's{tile:02x}h'
        tiles[mirrored] = planes(0, tile) + ['h']
        add([[mirrored]], depth=8, geometry=0, offset=[0, 0, 56])

# --- The pipes: four tiles across, round. ----------------------------------------------------
add([[name(1, PIPE_BODY + i) for i in range(4)]], depth=26, geometry=2,
    offset=[0, 0, 0])
add([[name(1, PIPE_TOP_CAP + i) for i in range(4)]], depth=32, geometry=2,
    offset=[0, 0, 0])
add([[name(1, PIPE_BOTTOM_CAP + i) for i in range(4)]], depth=32, geometry=2,
    offset=[0, 0, 0])

# --- The moving pipes' caps (src/mover.s): sprites, as the background's caps. --------------
SPRITE_TOP_CAP, SPRITE_BOTTOM_CAP = 0x24, 0x28
for first in (SPRITE_TOP_CAP, SPRITE_BOTTOM_CAP):
    add([[name(0, first + i) for i in range(4)]], depth=32, geometry=2, offset=[0, 0, 0])

# --- The bird: 3x2 sprites per pose and wing position, flat with a little thickness. -------
for frame in range(9):
    first = BIRD_BODY + frame * 6
    add([[name(0, first + i) for i in range(3)], [name(0, first + 3 + i) for i in range(3)]],
        depth=6, geometry=0, offset=[0, 0, 0])
for pose in range(3):
    add([[name(0, BIRD_BEAK + pose)]], depth=6, geometry=0, offset=[0, 0, 0])

# --- The text: the font of the title and the score digits (the same glyphs), flat. ---------
for glyph in list(range(0x21, 0x7F)) + [0xA9]:
    if any(data[4096 + glyph * 16:4096 + glyph * 16 + 16]):
        add([[name(1, glyph)]], depth=2, geometry=0, offset=[0, 0, -4])
for digit in range(10):
    if planes(0, digit) not in [tiles[k] for k in tiles if k.startswith('b')]:
        add([[name(0, digit)]], depth=2, geometry=0, offset=[0, 0, -4])

# --- The title: the logo stands out a little, the text is flat. ------------------------------
for row in (0xD0, 0xE0, 0xF0):
    for i in range(16):
        add([[name(1, row + i)]], depth=6, geometry=0, offset=[0, 0, -8])

spec = {
    'id': 'flappyzaxnes',
    'title': 'FlappyZaxNes',
    'author': 'Zaxnes (profile); game by Nixxou, after Flappy NES by filipemd (GPL-3.0)',
    'publisher': 'Nixxou',
    'description': 'FlappyZaxNes, the demo game of Zaxnes, after Flappy NES by filipemd '
                   '(GPL-3.0).',
    'template': 'empty.3dn',
    'layers': [{'index': SOLID, 'offset': 0, 'aligned': 1}],
    'tiles': tiles,
    'patterns': patterns,
}
json.dump(spec, open(sys.argv[1], 'w'), indent=1)
print(len(tiles), 'tiles,', len(patterns), 'patterns')
