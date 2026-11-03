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

Off by default. `--mcp` starts it on `127.0.0.1:8765` (`--mcp=<port>` for another port); only the
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
| `get_log {lines}` | the last lines of the log, Lua `print`, warnings and errors included |
| `run_command {name, number, text}` | any command of the menus by name (`docs` of `emulation/commands.rs`) |

## 5. Cost when it is not used

Nothing of it runs in the frame loop unless used: the movie hook already existed (one `Option`
test); the timeline adds one `bool` read beside `rollback_enabled` per emulated frame, and work
only on the frames that are multiples of 60 while a movie plays; the seek is one `Option` test
per tick; the MCP server one `Option` test per displayed frame; the log keeps lines in memory
only once the server started. The core, the conversion, the Lua hooks, the draw list and the
renderer are untouched.
