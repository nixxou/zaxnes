# Frame dumps and NES colours

## A dump

`dump_frame {path}` (MCP) and `zax-harness render ... --dump-frame <file.json>` write the same
JSON: every tile the NES drew on the frame, in the order the picture was made.

```json
{
  "palette": [15, 22, 39, 24, 15, 1, 33, ... 32 numbers],
  "tiles": [
    { "low": 1157442765409226768, "high": 0, "x": 16, "y": 200, "palette": 0,
      "mirrorH": false, "mirrorV": false, "hidden": false, "sprite": false }
  ]
}
```

| Field | Meaning |
|---|---|
| `palette` | the 32 bytes of the PPU palette at the end of the frame, each a NES colour index 0 to 63 (table below): bytes 0-15 the four background sub-palettes, 16-31 the four sprite ones, 4 bytes each |
| `low`, `high` | the tile's two bit planes, as **decimal** 64-bit integers; together they are the tile's identity (its picture) |
| `x`, `y` | the screen position of its top-left pixel (background: x from -7, y from -1; sprites: 0 to 255, 1 to 239) |
| `palette` (tile) | its sub-palette, 0 to 3 (of the background or of the sprites) |
| `sprite` | a sprite rather than a background tile |
| `mirrorH`, `mirrorV` | a sprite drawn flipped; the picture in `low` / `high` is the unflipped one |
| `hidden` | a sprite behind the background (priority bit) |

`profile.json` writes the same planes as 16 hexadecimal digits: `format(low, "016x")`.

**The pixels**: bit `y * 8 + (7 - x)` of `low` and of `high` is pixel (x, y), x from the left, y
from the top; its colour number is `2 * high_bit + low_bit`, 0 to 3. Colour 0 is transparent
(the backdrop shows). For a mirrored sprite, flip x and/or y after reading.

**Its colours**: colour number `c` of a background tile of sub-palette `p` is the NES colour
`palette[4 * p + c]`, of a sprite `palette[16 + 4 * p + c]`; colour 0 is always `palette[0]`, the
backdrop. The same picture can be drawn with several sub-palettes: one tile for the profile
(`shape.Palette` tells them apart in Lua).

```python
def pixels(tile, palette):
    base = (16 if tile["sprite"] else 0) + 4 * tile["palette"]
    for y in range(8):
        for x in range(8):
            bit = y * 8 + 7 - x
            c = ((tile["high"] >> bit) & 1) * 2 + ((tile["low"] >> bit) & 1)
            yield x, y, c, (palette[0] if c == 0 else palette[base + c]) & 63
```

`zax-harness classify --sheet` draws the tiles of dumps in their colours, if you would rather
look than decode (`docs/CLASSIFY.md`).

## The 64 NES colours

The RGB Zaxnes draws each NES colour index with (its default palette; the user can choose
another one in the options, which changes the picture, not the indices). Hexadecimal `RRGGBB`:

| index | +0 | +1 | +2 | +3 | +4 | +5 | +6 | +7 | +8 | +9 | +A | +B | +C | +D | +E | +F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0x00 | 788084 | 0000FC | 0000C4 | 4028C4 | 94008C | AC0028 | AC1000 | 8C1800 | 503000 | 007800 | 006800 | 005800 | 004058 | 000000 | 000000 | 000000 |
| 0x10 | BCC0C4 | 0078FC | 0088FC | 6848FC | DC00D4 | E40060 | FC3800 | E46018 | AC8000 | 00B800 | 00A800 | 00A848 | 008894 | 2C2C2C | 000000 | 000000 |
| 0x20 | FCF8FC | 38C0FC | 6888FC | 9C78FC | FC78FC | FC589C | FC7858 | FCA048 | FCB800 | BCF818 | 58D858 | 58F89C | 00E8E4 | 606060 | 000000 | 000000 |
| 0x30 | FCF8FC | A4E8FC | BCB8FC | DCB8FC | FCB8FC | F4C0E0 | F4D0B4 | FCE0B4 | FCD884 | DCF878 | B8F878 | B0F0D8 | 00F8FC | C8C0C0 | 000000 | 000000 |

A texture (a pattern's `texture` PNG) is drawn as it is, whatever the palette: take its colours
from this table, or from the frame you copied it from, so that it matches the game.
