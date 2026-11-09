# Classifying tiles: `zax-harness classify`

The first step of a profile: sort the tiles of the game into classes (floor, wall, background,
ladder, sprites, HUD...) and give each class a 3D look. `classify` does it from frame dumps and
rules, and writes patterns for `profile.json`.

```
zax-harness classify rules.json --out classified.json --sheet sheet.png [dumps or folders...]
```

1. Dump many frames (`seek`, `dump_frame`): every screen of the level, and for animated tiles
   (water, fire, a spinning coin) several frames in a row (`step 1` between dumps), or a step of
   the animation is left out and keeps the default look. One folder per level is handy.
2. Write the rules (an example: `examples/classify-rules.json`).
3. Run `classify`, look at `sheet.png` (one band of tiles per class in their colours, then the
   unclaimed ones; the summary gives each band's rows) and at the report.
4. Adjust the rules until each band holds what it should, then add `"includes":
   ["classified.json"]` to `profile.json` and `reload_script`.

## The rules

```json
{ "dumps": ["dumps/level1"],
  "classes": [
    { "name": "hud", "where": { "y": [0, 31] }, "pattern": { "layer": 1, "depth": 2 } },
    { "name": "sprites", "where": { "sprite": true }, "pattern": { "depth": 6 } },
    { "name": "back", "where": { "palette": [1], "sprite": false },
      "pattern": { "depth": 2, "offset": [0, 0, 46] } },
    { "name": "floor", "where": { "palette": [0, 3] }, "pattern": { "depth": 48 } } ] }
```

* `dumps`: dump files or folders (their `*.json`), relative to the rules file; more can be given
  on the command line.
* A class: a one-word `name`, the conditions `where`, and `pattern`: any field of a pattern of
  `profile.json` except `grid` (`depth`, `offset`, `geometry`, `layer`, `tags`, `copies`...).
* `where` (all must hold for an occurrence of a tile):

| Condition | Holds when the occurrence... |
|---|---|
| `palette: [0, 2]` | uses one of these sub-palettes |
| `sprite: true/false` | is a sprite / a background tile |
| `x: [min, max]`, `y: [min, max]` | has its top-left pixel in that range (inclusive) |
| `frames: ["d1200.json"]` | is in one of these dump files |
| `colours: [17, 33]` | shows at least one of these NES colours (`docs/DUMP-FORMAT.md`) |
| `tiles: ["t...."]` | is one of these pictures (keys of the report) |
| `not_tiles: ["t...."]` | is none of these |

An empty `where` takes everything left: put it last, as a default class.

## How a picture is given its class

A pattern knows only a tile's picture, so a picture is in one class only: the **first** class,
in the order of the list, that one of its occurrences matches. Put the precise classes first and
the broad ones (a whole region, a whole palette) after. A sprite drawn mirrored is a picture of
its own.

The report (`classified.report.json` beside the output, or `--report`):

* `classes`: each class's tiles, with the tile name of the output (`floor_12`) and the picture's
  key (`t<low hex><high hex>`, `_h`/`_v`/`_hv` for a mirrored sprite);
* `shared`: pictures whose occurrences matched another class (or none) somewhere: the same tile
  used in two roles. Either it is fine (the first class is right everywhere), or give it a
  better class with `tiles` / `not_tiles`, or handle it in Lua (by `shape.Palette`, by the level
  on screen, by its place);
* `unclaimed`: pictures no class took, with their planes, count, palette and an example place.

## The output

A description with `tiles` (named `<class>_<n>`) and one one-tile `pattern` each, tagged with the
class name followed by the class's own `tags`. Include it from `profile.json` rather than copying
it: run `classify` again after new dumps or new rules, and the hand-written patterns of
`profile.json` stay as they are. A tile named in both files must have the same planes.

In Lua, `string.find(shape.TagStr, "floor", 1, true)` finds the class of a shape. Remember that
`Pattern.GetPatternWithTag` matches the whole tag string: a pattern you look up from Lua is better
hand-written in `profile.json` with a one-word tag.
