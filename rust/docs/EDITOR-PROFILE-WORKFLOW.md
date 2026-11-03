# Making a 3D profile with the editor: the workflow

How an agent (or a person) builds a Zaxnes 3D profile for a game with the editor tools of
`EDITOR.md`: the movie bar, the script reload, the MCP server, and `zax-harness`. Learnt on Metal
Force (branch work EDITOR-1); the facts below were measured, not guessed.

The profiles stay in the 3dSen format: no engine change for a game. What the format cannot say
(a depth per palette, per level, per place) goes into the profile's Lua script.

## 1. The loop

1. **Run the game** with the server: `zaxnes --rom <game.nes> --profile "<title>" --movie
   <movie.fm2> --mcp`. A TAS movie (`.fm2`) is the easiest way to reach every screen; the movie
   bar and the `seek` tool go to any frame.
2. **Look**: `screenshot` (`view: 3d` or `2d`), from the camera the user set. For a view of your
   own, render off-screen: `zax-harness render --frames rom --rom <game> --profile <file.3dn>
   --movie <movie.fm2> --run <frame> --angles <rx>,<ry> --out <dir>` (`--angles 89,0`: from
   above; `0,89`: from the side).
3. **Read the tiles** of a frame: `dump_frame {path}` (MCP, works on a live game too) or
   `zax-harness render ... --run <frame> --dump-frame <file.json>`. Each tile: `low`, `high`
   (its two bit planes, the tile's identity), `x`, `y`, `palette`, `sprite`.
4. **Classify** the tiles into classes (floor, wall, ladder...) in a small script per level that
   reads the dumps and writes patterns (one-tile `grid`, `tags`, `depth`, `offset`).
5. **Build**: merge the classes into the profile description, `zax-harness profile-build
   <description.json> <profiles folder>/<name>.3dn`.
6. **Reload**: `reload_script` (fast; F5 in the window). Step one frame (`step`) so that the
   frame is converted again, then look.
7. **Check depths** when a picture is ambiguous: `zax-harness render ... --explain` lists every
   group of the draw list with its palette, depth, offset, scale and the `drawn z [a, b]` it
   ends at.

Before rebuilding `zaxnes` while a live game runs: `run_command state.save_slot` (number 9),
kill, rebuild, relaunch, `state.load_slot` 9. The executable is locked while it runs.

## 2. The profile description (`profile-build`)

```json
{ "title": "...", "tiles": { "name": ["<low hex16>", "<high hex16>"] },
  "patterns": [ { "grid": [["name"]], "depth": 48, "offset": [0, 0, 24],
                  "geometry": 0, "texture": "file.png", "tags": "level1 floor" } ],
  "scripts": [ { "name": "main", "file": "main.lua" } ] }
```

`texture`: a PNG laid pixel for pixel over the pattern (alpha 0 = empty), for a picture the
game does not draw (a coin's true face, a 3D model's colours).

## 3. Measured facts

* **Depth axis**: z = 0 is the front; a positive z goes back. A pattern of depth `d` and offset
  `o` spans z `[o, o + d]`.
* **Lua coordinates**: `shape.OriPos` is the shape's centre, its `y` counted from the bottom
  (screen row = 248 - `OriPos.y`). `shape.Shape2D.Start` / `End` are in screen pixels (the first
  and last non-empty pixels, so a tile at row 208 can start at 209).
* **From Lua**, `Offset`, `Rot` and `Scale` work on matched shapes; `sizeZ` does not change the
  volume of a pattern-matched shape: give the depth in the pattern, flatten with `Scale.z`
  (the scale keeps the block's middle: `Offset.z = target + newDepth/2 - depth/2`).
* **Shapes**: touching tiles are joined into one shape, which `UpdateS` sees once. A depth per
  tile is only possible by pattern. `Setting.SameBgPalette = true` keeps tiles of different
  palettes apart.
* **One tile, several looks**: a pattern knows only the tile's picture. The same picture drawn
  with another palette, or in another level, gets the same look. Tell them apart in Lua: by
  `shape.Palette`, or by the level on screen (count the shapes of each level's tags on a frame,
  use the counts on the next one — the same trick recognises a title screen). Reading the RAM
  would do too, but a script reading the core disables the movie's fast seek.
* **The left column** the NES hides is drawn by the 3D with stray colours while the screen
  scrolls: `Clipping.x = 8.1` in `Start()`.
* **Adding objects**: `Frame:CreateNewShape(instance, Vector3(left, 248 + 1 - 8*H - top, z), 0)`
  in `LateUpdate`, once per frame (`H` the pattern's height in tiles, `top` the screen row of
  its top). A pattern made for it (unique tile pictures, `texture`) is a 3D model: Metal Force
  stacks square slices into stepped pyramids for its spikes, hiding the game's flat spikes with
  `shape.Enable = false`.
* **Animated tiles** (a spinning crystal) are a different picture per step: dump enough frames
  to see every step, or a step left out shows with the default extrusion.

## 4. Pitfalls

* A class picked by place catches its neighbours: filter by palette and only take tiles no class
  has yet; add the picks after the precise ones (a ladder caught in a wall pick goes flat).
* A tile used by two levels: let one level own it and handle the other in Lua.
* The 2D view (F11) hides the 3D effects; `status` says `two_dimensional`.
* Fast reload starts the scripts on the frame on screen; step one frame before judging.
