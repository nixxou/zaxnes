# `profile.json`: the description a profile is built from

Zaxnes builds the project's `.3dn` from this file at each reload (`reload_script`, F5);
`zax-harness profile-build <profile.json> <out.3dn>` does the same without the window. Paths are
relative to the folder of `profile.json`.

```json
{
  "id": "lua-mygame-1a2b3c4d",
  "title": "My Game (Lua)",
  "author": "", "publisher": "", "description": "",
  "template": "base.3dn",
  "includes": ["classified.json"],
  "layers": [{ "index": 4, "offset": 0, "aligned": 1 }],
  "tiles": {
    "brick": ["0f0f0f0f0f0f0f0f", "00ff00ff00ff00ff"],
    "coin_flipped": ["...", "...", "h"]
  },
  "patterns": [
    { "grid": [["brick", "brick"], ["brick", null]],
      "depth": 8, "offset": [0, 0, 0], "geometry": 0, "layer": 2,
      "scale": [1, 1, 1], "rotation": [0, 0, 0], "pivot": [0.5, 0.5, 0.5],
      "solid": false, "permanent": true, "self_connected": true,
      "tags": "level1 floor", "empty": 0,
      "texture": "brick.png",
      "copies": [{ "offset": [0, 0, 6], "depth": 2 }],
      "deform": { "amplitude": [0, 1, 0], "pivot": [0.5, 0, 0.5],
                  "speed": [16 numbers], "offset": [16 numbers] } }
  ],
  "scripts": [{ "name": "main", "file": "main.lua" }]
}
```

## The profile

| Field | Meaning |
|---|---|
| `id` | the profile's identifier in the library: keep the one the editor gave |
| `title`, `author`, `publisher`, `description` | what the library shows |
| `template` | optional: a `.3dn` whose settings (layers, camera, lights...) are kept; its patterns and scripts are not |
| `includes` | optional: descriptions whose `tiles` and `patterns` join this one's (the output of `zax-harness classify`); a tile named twice must have the same planes |
| `layers` | optional, needs a `template`: overrides of its six layer sets, `index` 0 to 5, `offset`, `aligned` (0 front, 1 centre, 2 back) |
| `tiles` | the tile pictures, by a name of your choice |
| `patterns` | the patterns, none at all for a profile of scripts only |
| `scripts` | the Lua files, run in the order of their names |

## A tile

`"name": ["<low>", "<high>"]`: the two bit planes of the 8x8 picture, 16 hexadecimal digits
each, as `dump_frame` writes them (bit `y * 8 + (7 - x)` is pixel (x, y)). The colours are not
part of the tile: one picture drawn with two palettes is one tile. A sprite the game draws
mirrored is matched only with that mirroring: give it as a third element, `"h"`, `"v"` or `"hv"`
(the dump's `mirrorH` / `mirrorV`).

## A pattern

| Field | Default | Meaning |
|---|---|---|
| `grid` | (needed) | rows of tile names from the top, `null` for an empty cell |
| `depth` | 8 | its thickness in z (z = 0 is the front, a positive z goes back) |
| `offset` | `[0,0,0]` | its offset; `offset[2]` is its z: it spans `[z, z + depth]` |
| `geometry` | 0 | 0 extruded from the pixels, 1 horizontal cylinder, 2 vertical cylinder, 3 cube, 4 half horizontal cylinder, 5 half vertical cylinder |
| `layer` | 2 | 0 none, 1 game interface (HUD), 2 to 5 the four depth layers |
| `scale`, `rotation`, `pivot` | 1, 0, 0.5 | the volume's scale, rotation in degrees, and the point they turn around |
| `solid` | false | the volume filled rather than following the pixels |
| `permanent` | true | always looked for (false: only when a script asks) |
| `self_connected` | true | the pattern joins with its own copies next to it |
| `tags` | `""` | words the scripts read (`shape.TagStr`, `Pattern.GetPatternWithTag`) |
| `empty` | 0 | a palette colour (1 to 3) left out of the volume |
| `texture` | none | a PNG laid pixel for pixel over the grid (alpha 0 = empty): a look the game does not draw |
| `copies` | none | more variants drawn too, each the main one with its own `offset` / `depth`: a motif repeated in depth |
| `deform` | none | the animated deformation: `amplitude`, `pivot`, and two 4x4 tables `speed`, `offset` |

Pitfalls: `Pattern.GetPatternWithTag` matches the whole tag string, so a pattern found from Lua
carries one word only. Two patterns with the same tile picture are one tile for the game: a
pattern made only for Lua (with its own `texture`) needs a picture no tile of the game has.
