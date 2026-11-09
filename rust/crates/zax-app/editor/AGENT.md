# Writing a Zaxnes 3D profile: start here

You are asked to write, or improve, the 3D profile of a NES game for Zaxnes, a NES emulator that
draws games in 3D. Everything you need is in this folder; read this page first, then the
documents it points to when you need them.

## 1. What a profile is

A profile (`.3dn` file) tells Zaxnes how to turn the NES picture into 3D. The NES draws its
picture with 8x8 tiles; the profile gives tiles a 3D look:

* **patterns**: a tile (or a group of tiles) recognised by its picture, with its depth, its
  offset in depth, its geometry (extruded, cube, cylinder), its layer, its tags;
* **Lua scripts**: code run on every frame, which reads the shapes of the frame (the tiles the
  game draws, joined when they touch) and moves, hides, scales, recolours them or adds new ones.

The format is the one of 3dSen (another NES-to-3D program): a profile must stay readable by it.
Never ask for an engine change for one game; what the format cannot say goes into the Lua.

## 2. Your project

The editor made a project folder for the game, `projects/<game>/` next to this file:

| File | What it is |
|---|---|
| `profile.json` | the profile's description: title, tiles, patterns, scripts (`docs/PROFILE-JSON.md`) |
| `main.lua` | the profile's script (`docs/spec/SPEC-05-lua.md` and `docs/LUA-API.txt`) |
| `NOTES.md` | the state of the work: **read it first, keep it up to date** |
| `PROMPT.txt` | what you were asked, with the paths of this session |

Put your dumps, classification rules and scripts in the project folder too (`dumps/`,
`rules.json`...). `NOTES.md` is what lets the work resume after a crash, or be handed to another
agent: after each step that changes something, write down there what is done, the depth plan,
the user's choices (in their words when it matters) and what is left.

A new project holds one pattern, `placeholder` (a tile picture no game is likely to draw): a
profile without any pattern is played in 2D, this one keeps the game in 3D with the default
extrusion of every tile. Remove it once you have patterns of your own.

Zaxnes builds the `.3dn` from `profile.json` each time the scripts are reloaded (F5 in the
window, or the `reload_script` tool): **edit the project, never the `.3dn`**. Textures (PNG) and
more Lua files go in the project folder too, named in `profile.json`. Work only in the project
folder; do not touch the rest of the user's files.

A project can also start from an existing profile (Tools > New Lua profile for this game, on a
game that has one): the profile is kept whole as `base.3dn` (all its patterns and settings), its
scripts are the project's `.lua` files, and the patterns of `profile.json` are added to the
base's. The project's profile has its own identifier and title: the original stays as it is.
`NOTES.md` says where it came from.

## 3. Driving Zaxnes

Zaxnes runs in editor mode with an MCP server on `http://127.0.0.1:8765/mcp` (local only). If
your tools do not show it yet, the user adds it once:

```
claude mcp add --transport http zaxnes http://127.0.0.1:8765/mcp
```

The tools (detail: `docs/EDITOR.md` §4):

* `editor_info`: this session in one text (game, ROM identity, movie, folders, your project,
  the harness): the prompt you were given, again;
* `status`: the game, the movie frame, pause, 2D or 3D view;
* `open_movie`, `seek`, `step`, `play`, `pause`: go where you need to look. By default there
  is no movie: **the user plays**, and you work on the screen they are on (ask them to stop
  where you need, `pause`, look, then let them go on). A movie lets you reach any screen
  yourself; offer it once, never require it: an FCEUX `.fm2` or an older FCE Ultra `.fcm`
  (TASVideos has one for many games; a `.fcm` that starts from its savestate is refused), or the user's own play recorded with Tools >
  Record a movie (a Zaxnes `.zmv`, which `seek` then reaches like any movie);
* `screenshot` (`view: 3d` or `2d`): what the window shows, with the user's drawings;
* `annotations`: the user's drawings (they circle things with the pens and write notes);
* `dump_frame {path}`: every tile of the frame on screen (its two bit planes `low` / `high`,
  which are its identity, `x`, `y`, `palette`, `sprite`): the raw material of the patterns
  (format, pixels and colours: `docs/DUMP-FORMAT.md`);
* `read_ram`: the CPU memory (reading it from Lua slows the movie's seek: prefer the picture);
* `reload_script {mode}`: rebuild the `.3dn` from the project and restart the scripts;
* `get_log`: the log, with Lua `print` output and errors;
* `run_command`: any command of the menus.

## 4. The loop

1. Go to a screen (`seek`), take a `screenshot` in 3D and in 2D, `dump_frame` it (many frames:
   every screen of the level, consecutive frames for animations).
2. Classify the tiles (floor, wall, background, ladder, enemy...) with `zax-harness classify`
   (`docs/CLASSIFY.md`): rules by palette, place, colour; it writes the patterns, a report of the
   tiles used in two roles, and a picture sheet to check the classes by eye. Include its output
   from `profile.json` (`"includes"`).
3. Write in `main.lua` what patterns cannot say: a depth per palette or per level, objects added
   with `Frame:CreateNewShape`, hidden tiles, animation.
4. `reload_script`, `step` one frame (a fast reload restarts the scripts on the frame on screen),
   look again. Read `get_log` after each reload: a Lua error stops the script.
5. Show the user, take their notes (`annotations`), iterate. Save often.

The method learnt on a full game, its measured facts and its pitfalls:
`docs/EDITOR-PROFILE-WORKFLOW.md`. Read its §3 and §5 before placing anything; the usual Lua
techniques are in `docs/RECIPES.md`, a complete `profile.json` in `examples/flappyzaxnes/`.
Before saying a part is done: `docs/CHECKLIST.md`.

## 4b. Rules

* **3dSen compatible**: the Lua API of `docs/LUA-API.txt` is the one 3dSen offers; code for
  Zaxnes alone (lighter effects on a handheld) only inside `--[[#zax ...]]` comments
  (`docs/RECIPES.md`). Check the `3S` view. Never ask for an engine change for one game.
* **Speed**: Zaxnes also runs on handheld consoles. What is added in Lua (`CreateNewShape`,
  loops over every shape) costs on every frame: prefer patterns (`copies` in `profile.json`) to
  shapes added from Lua, look up patterns once (keep `Pattern.GetPatternWithTag` results), and
  check with the FPS counter and the `HH` view.
* **The movie's seek** stays fast only while no script reads or writes the core (`ReadMem`,
  `WriteMem`, input hooks): recognise screens by their tiles rather than by the RAM when you can.
* **The user decides** the looks: show, ask, and write their choices in `NOTES.md`.

Key facts: z = 0 is the front, a positive z goes back; a pattern of depth `d` and offset `o`
spans `[o, o + d]`. `shape.OriPos.y` counts from the bottom (screen row = 248 - `OriPos.y`).

## 5. The harness

`zax-harness` (next to `zaxnes`, when the package ships it) works without the window:

* `zax-harness profile-dump <file.3dn> [--tiles]`: a profile's patterns and their 3D looks;
* `zax-harness profile-build <profile.json> <out.3dn>`: what the reload does;
* `zax-harness render --frames rom --rom <game.nes> --profile <file.3dn> --movie <movie.fm2>
  --run <frame> --angles <rx>,<ry> --out <dir>`: a picture from any angle (`--angles 89,0` from
  above, `0,89` from the side); `--explain` lists every drawn group with its depth;
  `--dump-frame <file.json>` dumps the tiles;
* `zax-harness lua-api`: the Lua API as the runtime sees it (`docs/LUA-API.txt`);
* `zax-harness classify <rules.json> --out <classified.json> --sheet <sheet.png>`: the tiles of
  dumps sorted into pattern classes (`docs/CLASSIFY.md`).

## 6. The documents

| File | What it holds |
|---|---|
| `docs/EDITOR.md` | the editor: movies, movie bar, reload, MCP tools, pens, views |
| `docs/EDITOR-PROFILE-WORKFLOW.md` | the method, measured facts, pitfalls, Zaxnes-only code |
| `docs/PROFILE-JSON.md` | every field of `profile.json` |
| `docs/DUMP-FORMAT.md` | the frame dumps, how to read a tile's pixels and colours, the 64 NES colours |
| `docs/CLASSIFY.md` | `zax-harness classify`: from dumps to patterns |
| `docs/RECIPES.md` | Lua techniques: depth, levels, models in place of tiles, motifs, margins, Zaxnes-only code |
| `docs/CHECKLIST.md` | what to check before saying a part is done |
| `examples/` | a complete `profile.json` (FlappyZaxNes) and classification rules |
| `templates/` | the `main.lua` and `NOTES.md` of a new project |
| `docs/LUA-API.txt` | every global, constant and member the scripts see |
| `docs/spec/SPEC-05-lua.md` | the Lua runtime: hooks, order, every API with its meaning |
| `docs/spec/SPEC-03-format-profil.md` | the `.3dn` format |
| `docs/spec/SPEC-04-conversion.md`, `SPEC-04B-geometrie.md` | how tiles become shapes and volumes |
| `docs/spec/SPEC-00-glossaire.md` | the words of the specifications |

The specifications are in French. Zaxnes is free software (GPL v3); its source, if you need more
than these pages: https://github.com/nixxou/zaxnes
