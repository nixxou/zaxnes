# The profile editor: movies, the movie bar, script reload, the MCP server

What it is for: writing a Lua profile while looking at the game, at a precise place of it, and
letting Claude Code do the same through MCP ("raise these blocks by 2 at the boss": edit the
script, reload, look at the same frame). Branch work EDITOR-1 (`docs/progress/EDITOR-1.md`).

## 1. Movies

* **Open**: Tools > "Open a movie file..." (`tools.open_movie`), or `--movie <file>` on the
  command line (played once the `--rom` game has started). Two kinds:
  * an **FCEUX `.fm2`** (text format, read from its public documentation): played from power on.
    A region (`palFlag`) or ROM (`romChecksum`, the MD5 of the data after the iNES header) that
    differs is warned about, not refused. Refused with a message: movies starting from a
    savestate, binary movies, Zapper, Four Score, FDS, VS. System. Resets inside the movie are
    not replayed yet (warned);
  * an **FCE Ultra `.fcm`** (0.98.x binary movies, version 2, read from the TASVideos "FCM"
    page): read into the same frames as an `.fm2` and played the same way. Its controller data
    toggles buttons, each update followed by the frames to the next one; the header's frame
    count is the length. Refused: movies starting from their savestate (every `.fcm` holds one;
    the start is from reset when the header's flag says so or the first command is a reset or
    a power cycle), Four Score, FDS and VS. System commands;
  * a **Zaxnes movie** (`.mv<n>`, `docs/MOVIES.md`).
* While a movie plays the run-ahead is held at 0 (its frames are shown as they are), the
  setting kept for after.
* At its last frame the movie stops, paused, like a video; Tools > "Stop the movie" gives the
  game back to the player.

## 2. The movie bar

At the bottom of the window, over the view, shown when the pointer nears the bottom edge, while
paused, while a seek runs and while the slider is held: `|<` `<` Play/Pause `>` `>|`, the slider,
`frame / length m:ss.cc`, and `[n%]` while the movie is still being explored.

Going to a frame (the slider, the buttons, `tools.seek_movie <frame>`):

* a **checkpoint** (core state) every 60 frames over the whole movie, about 20 KiB each
  (about 12 MiB for ten minutes);
* an **explorer** thread runs the whole movie ahead with its own core (no scripts, no sound) and
  hands its checkpoints over; they are used once one of them is found to be the same machine as
  the session's own checkpoint of the same frame (frame and cycle counts, work RAM, picture).
  Metal Force's 47 454 frames take about 40 s; any seek is then quick;
* a seek starts from the checkpoint a second before its target, runs the core alone up to that
  second, then the last 60 frames with the scripts and the conversion, so that a smoothed camera
  or an accumulated value is right where it stops. 12 ms of work per displayed frame: the window
  stays responsive, the sound is off meanwhile.
* The explorer and the core-only frames are used only when no script reads or drives the core
  (`CORE_MEMBERS` of `game_session.rs`): otherwise the core's past depends on the scripts.

## 3. Reloading the scripts

Tools > "Reload the profile's scripts" (F5, `tools.reload_script`; number 1 = exact): the
profile file is read again and a new session is made of it, the game staying where it is.

* **fast** (F5): the new scripts start (`Start`) on the frame on screen; for what does not
  accumulate (an offset, a colour, a camera setting);
* **exact** (with a movie): the movie is replayed from its start with the new scripts up to the
  frame on screen, so that what they smooth or accumulate is as if they had always been there;
* when the old or the new scripts **change or drive the core** (`WriteMem`, `SetInputType`,
  `InputProcess`...), a reload with a movie is always exact and the checkpoints are taken again.

Without a movie, a reload is fast.

## 4. The MCP server

Off by default (making a profile with it: `EDITOR-PROFILE-WORKFLOW.md`). `--mcp` starts it on `127.0.0.1:8765` (`--mcp=<port>` for another port); only the
loopback interface is bound, and a request from a web page of another host is refused. It is
hand-written (Streamable HTTP, JSON answers, `serde_json`, no async runtime).

Adding it to Claude Code, once:

```
claude mcp add --transport http zaxnes http://127.0.0.1:8765/mcp
```

then start Zaxnes with `--mcp` (and usually `--rom`, `--profile`, `--movie`):

```
zaxnes --rom <dossier des ROM>/game.nes --profile <profile file> --movie <movie.fm2> --mcp
```

Tools (each action goes through the window's own commands, so it shows there):

| Tool | What it does |
|---|---|
| `status` | game, state, movie frame / length, seek under way, explored share, whether the scripts drive the core |
| `open_movie {path}` | plays an `.fm2` or a Zaxnes movie |
| `seek {frame}` | pauses and goes to the frame; answers once there |
| `step {count}` | pauses and moves by `count` frames (negative: back) |
| `play`, `pause` | |
| `reload_script {mode: fast/exact}` | the reload of section 3; answers once done |
| `screenshot {view: 3d/2d, width, height}` | the picture on screen as a PNG (3D view scaled, default 960 x 720; or the NES picture) |
| `read_ram {address, length}` | CPU memory without side effects, hexadecimal |
| `dump_frame {path}` | the tile records of the frame on screen to a JSON file, as `zax-harness render --dump-frame` writes them (works on a live game too) |
| `annotations {width, height}` | the user's drawings (section 5): each stroke's pen and points, from 0 to 1 and in pixels of the view |
| `clear_annotations` | wipes the drawings |
| `get_log {lines}` | the last lines of the log, Lua `print`, warnings and errors included |
| `editor_info` | in editor mode: the session as the agent's prompt gives it (game, ROM identity, movie, folders, project, harness) |
| `run_command {name, number, text}` | any command of the menus by name (`docs` of `emulation/commands.rs`) |

## 5. The pens

With `--mcp`, a tool box at the left of the window while a game runs: the hand (`M`, the
pointer turns the camera as usual), a red, a blue, a black and a white pen, undo (`<`) and clear
(`X`). With a pen in hand, a drag draws over the view instead of turning the camera: circle a
zone, write a note, draw what is to be modelled and where. The strokes stay over the view
(kept from 0 to 1 of the window, so a resize keeps them in place) until undone or cleared.

`screenshot` (3D view, which is the picture on screen) draws them over the picture it returns
(`annotations: false` leaves them out); `annotations` lists them. The NES picture (`view: 2d`)
has none: it is the game's own picture.

Under the pens, the views: `2D` (the NES picture, as F11), `3D` (as Zaxnes draws it), `3S` (as
3dSen draws it: the `#zax` blocks of the scripts stay comments) and `HH` (as Zaxnes draws it on a
handheld: the blocks read with the `handheld` condition). A change of reading reloads the scripts
(fast); the framed button is the view shown.

## 6. Cost when it is not used

Nothing of it runs in the frame loop unless used (the editor mode, section 7, adds one setting
read per displayed frame while it is off): the movie hook already existed (one `Option`
test); the timeline adds one `bool` read beside `rollback_enabled` per emulated frame, and work
only on the frames that are multiples of 60 while a movie plays; the seek is one `Option` test
per tick; the MCP server one `Option` test per displayed frame; the log keeps lines in memory
only once the server started. The core, the conversion, the Lua hooks, the draw list and the
renderer are untouched.

## 7. The editor mode

Tools > "Editor mode" (`tools.editor_mode`, the setting `ui.editor_mode`, kept; `--editor` turns
it on) gathers the above for writing a profile with an AI agent (`editor_mode.rs`):

* the MCP server (section 4) and the pens (section 5), started and stopped with the mode (the
  server closes its port when the mode is turned off; `--mcp` keeps it on for the session);
* the **editor folder** of the user data (`<data>/editor`), laid out from documents the
  executable carries: `AGENT.md` (what an agent reads first), `docs/` (this page, the workflow,
  `PROFILE-JSON.md`, `DUMP-FORMAT.md` with the NES colours, `CLASSIFY.md`, `RECIPES.md`,
  `CHECKLIST.md`, `LUA-API.txt`, SPEC-00, 03, 04, 04B and 05), `examples/` (the FlappyZaxNes
  description and classification rules), `templates/` (`main.lua`, `NOTES.md`). A
  document is written when it is missing or when the set changed (a digest in
  `editor/.documents`); the projects are never written again;
* `zax-harness classify <rules.json> --out <classified.json> --sheet <sheet.png>`: the tiles of
  frame dumps sorted into pattern classes by rules (palette, place, NES colour, sprite, dump),
  each picture to its first class; a report of the pictures used in two roles and of those no
  class took, and a sheet of the classes in the game's colours. `profile.json` takes its output
  through `"includes"`;
* **projects**: `editor/projects/<game>/profile.json` (the description of `zax-harness
  profile-build`, now `zax_profiles::description`), `main.lua`, `NOTES.md` (the state of the work, kept by the agent), `PROMPT.txt`, `build/`. Turning
  the mode on while a game runs on a generic entry ("2D Mode", "3D by default"), or Tools > "New
  Lua profile for this game" (`tools.new_lua_profile`), makes one: its profile (a placeholder
  pattern keeps it in 3D, a profile without patterns playing in 2D) is built, imported into the
  library and the game started again on it, its movie too. In editor mode a script reload first
  builds the profile file of a project-made entry again from its project
  (`GameCatalog::rebuild_from_project`), so the agent edits the project, never the `.3dn`;
* on a game whose entry has a profile file, "New Lua profile for this game" makes the project
  from it (`export_project`): the file kept whole as `base.3dn` (`"base"` of the description:
  every pattern, permanent and temporary, and every setting), its scripts written as `.lua`
  files with their `enabled` and `tags`, a new identifier and " (Lua)" after the title, so the
  original entry stays. On a game already on a project's profile it only copies the prompt;
* Tools > "Copy the prompt for the AI agent" (`tools.copy_agent_prompt`): the text to give the
  agent, with this session's paths (the editor folder, the project, the MCP address,
  `zax-harness` when it is next to the executable) and the source repository, into the
  clipboard; also written into the project's `PROMPT.txt` and copied when a project is made.

The Windows and Linux packages ship `zax-harness` next to `zaxnes` for it.
