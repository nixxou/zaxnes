# EDITOR-1 — Lua profile editor: `.fm2` movie, MCP server, hot reload

Branch `lua-editor`, worktree from `rustupdated` at `553af8b`. No push, no merge.

## Status

| # | Step | Status |
|---|---|---|
| 1 | Exploration and design note (this file); owner's approval | done |
| 2 | `.fm2` reader and its tests | done (`zax-nes/src/movie/fm2.rs`) |
| 3 | Movie playback in the loop, run-ahead 0, checkpoints, seek both ways | done |
| 4 | **Seek bar in the window** (owner's priority, moved up from lot 2) | done (owner's visual check pending) |
| 5 | Script reload, fast and exact, with the core-changing guard | to do |
| 6 | MCP server (Cargo feature, off by default) and its commands; end-to-end test | to do |
| 7 | Zero-cost-when-closed measurement; real run on Squirrel's level | to do |
| L2 | Rest of the strip: chapters on the bar, thumbnails, pad shortcuts, handheld style | to do |

Next step: 5, script reload (fast / exact); then 6, the MCP server (`--mcp`).

## What exists already (and changes the plan)

* **Movies are already in Zaxnes** (`docs/MOVIES.md`, `zax-nes/src/movie.rs`,
  `zax-app/src/emulation/host/movies.rs`): `Movie` = start state (core state with ROM image) +
  one `u16` input word per frame + notes; `MovieRun::input(frame_count, word)` replaces the
  player's word at `GameSession::begin_frame_lent`, the clock being the core's frame count, so
  a restored core state moves the movie with it. The harness replays them (`render --movie`).
  **Consequence**: `.fm2` is an *importer* into `Movie`, not a second playback path. Its start
  state is the power-on state of the ROM (`NesCore::new` + ROM + `save_state(true)`). No
  per-frame code is added: the movie hook already exists and costs one `Option` test.
* **Core state in memory** (`zax_nes::state::CoreState`, ~1 µs save+restore, 19.5 KiB) and the
  rollback history (core + 64 `PData` + camera every 60 frames) give the checkpoint mechanism.
* **A script reload is a new `GameSession`**: `GameSession::create` builds a fresh core, runs the
  chunks (REQ-LUA-015), loads the patterns; then `restore_core` puts the core back. The host
  already does exactly this to load a slot (`jobs.rs`, `ensure_session` + `restore_slot`).
* **The guard** exists as text scans at profile load: `scripts_leave_the_core_alone`
  (`CORE_MEMBERS`) and `scripts_change_the_core` (`WriteMem`, `SetInputType`). The editor adds
  `InputProcess` to its own test (it changes the inputs the core sees, so the core timeline
  depends on the script).

## Design

### Where the code lives

* `zax-nes/src/movie/fm2.rs` (or `zax-nes/src/fm2.rs`): the parser, pure, no feature (small,
  never called unless a `.fm2` is opened; useful to the harness too: `render --movie x.fm2`).
* A new crate **`zax-editor`** (behind the Cargo feature `editor` of `zax-app`, **not** in
  `default`): the timeline (checkpoints, chapters), the reload logic, the MCP server, later the
  film strip. `zax-app` calls into it from **one** place in the tick, behind
  `if let Some(editor) = &mut self.editor` — and with the feature off that field does not exist
  (`#[cfg(feature = "editor")]`), so the PortMaster (`--features firmware-sdl`) and Android
  (`default-features = false`) builds contain nothing of it. The Windows/Linux desktop build
  scripts turn the feature on.
* Activation: `--editor` (implies run-ahead 0, rollback history off: the editor's own
  checkpoints replace it) and `--mcp[=port]`; off by default; the server listens on
  `127.0.0.1` only.

### The timeline

* **Core checkpoints**: a `CoreState` every 60 frames of the movie while it plays, kept for the
  **whole** movie (a 10-minute level = 36 000 frames = 600 states x 19.5 KiB ≈ 12 MiB; fine on
  the desktop). Recorded under one script identity (hash of the profile's sources); invalid when
  that changes *and* the script can change the core.
* **Chapters**: frame + name, stored next to the movie (`<movie>.chapters.json`) with an
  optional "clean point" flag (see below).
* Playback forces run-ahead 0 (`set_run_ahead(0)`, the raw setter; the user's setting is kept
  for after) and the existing per-tick loop does the rest: play, pause, step use the existing
  pause and frame-step machinery.

### The seek bar (owner's priority)

In the windowed (desktop) UI, while a movie plays, a bar along the bottom of the window: a
slider over the movie's frames that the owner drags to any point, with play/pause, step back and
forward, the current frame / total and time. Dragging scrubs at the **instant** level (core only,
from the nearest checkpoint, so the picture follows the mouse); releasing settles at the **warm**
level so the scripts' state is right where the bar stops. Seeking forward past the frames already
played runs the core ahead to build the missing checkpoints (with a progress fill on the bar).
It is drawn by the existing ImGui UI, only while a movie is loaded in the editor. It works for
Zaxnes's own movies (`.mv<n>`) as well as imported `.fm2`.

### Seeking and reloading: three levels of exactness

The core at frame N depends on the movie only (unless the script changes it). The script's
state (Lua globals, smoothed camera, learned conversion) depends on its whole past and cannot be
snapshotted except `PData` + camera. So every move in time says how the script state is rebuilt:

| Level | Core | Script | Cost | Use |
|---|---|---|---|---|
| **instant** | checkpoint ≤ N, then core-only frames to N (no conversion, no Lua) | keeps its state | < 1 ms | scrubbing the strip |
| **warm** (default for `seek`) | checkpoint ≤ N − 60 | runs (with conversion, no drawing) for the last ≥ 60 frames | ~60 frames of work, ~0.1 s | a camera that smooths over a second or two looks right |
| **exact** | from the movie start, or from a chapter marked *clean* | runs every frame from there, `Start` at that point | a few seconds for a level | anything that accumulates |

`reload_script(mode)`:
* **fast** = new session, core put back at N, `Start` runs at N (+ the warm-up of the "warm"
  level as an option): instant;
* **exact** = new session, replay from the movie start (or the last *clean* chapter before N)
  with the new script to N, without drawing; the strip shows progress; cancellable.

**Guard**: when the profile's scripts change the core (`scripts_change_the_core`, plus
`InputProcess`), the core checkpoints are only valid for the script that produced them: a reload
then forces **exact** (fast is refused with a message saying why), and the checkpoints are
rebuilt during that replay. A *clean* chapter is one where the owner states that the script
reinitialises itself (start of a level): exact from there is then trusted.

### `.fm2` import

From the public TASVideos documentation only (no FCEUX code). Header `key value` lines, then
`|commands|port0|port1|port2|` per frame, pads as `RLDUTSBA` (`.` or space = released), mapped to
Zaxnes's word (A, B, Select, Start, Up, Down, Left, Right in bits 0-7 = the reverse order of the
text). Handled: commands column (soft reset, hard reset; FDS/VS commands refused), `palFlag` vs the
ROM's region (warning), `romChecksum` (base64 MD5) vs the ROM (warning), `port0/1` gamepad or
none, `port2`/`fourscore`/Zapper, `binary` and `savestate` movies refused with a clear message at
first. Desync shows (RAM watch diverges, notes) without crashing; the external RAM-dump
comparison (FCEUX run by the owner) can come later as a harness command.

### The MCP server

* Transport: MCP **Streamable HTTP** on `127.0.0.1:<port>` (`claude mcp add --transport http
  zaxnes http://127.0.0.1:<port>/mcp`), because Zaxnes is a window the owner starts, not a
  process Claude Code spawns. JSON responses only (no SSE stream needed for request/response
  tools).
* Proposed implementation: **hand-written**, on one `std::thread`, `std::net::TcpListener`,
  `serde_json` (already a dependency): JSON-RPC `initialize`, `tools/list`, `tools/call`. No
  tokio, no async, ~400 lines. The alternative, `rmcp` (Apache-2.0, the official Rust SDK),
  brings tokio + axum/hyper for HTTP: heavy for the desktop build and pointless for a dozen
  synchronous tools. Owner's choice.
* The server thread never touches the session: each call becomes a message on a channel, the
  host drains it **once per tick** (only when the editor is on) and answers on a reply channel.
  So whatever Claude does goes through the same commands as the window, and is seen there (a
  `seek` moves the strip).
* Tools (first cut, as in the prompt): `load_rom`, `load_profile`, `load_movie`, `status`,
  `seek(frame, level)`, `play`, `pause`, `step(n)`, `reload_script(mode)`,
  `screenshot(frame?, view: 3d|2d)` (PNG returned as MCP image content, read back from the
  window's own framebuffer — the thumbnail path `capture_thumbnail` — so it is what the owner
  sees), `read_ram(addresses)`, `watch_ram(addresses, from, to)` (runs the core over the range
  from checkpoints, without Lua, returns a table), `get_log(since)`, `set_setting(id, value)`,
  `chapters`, `add_chapter(frame, name, clean)`.

### Zero cost when closed

Feature off: nothing compiled. Feature on but editor not started: one `Option` test per tick in
the host (not per frame of the core). The core, conversion, Lua hooks, draw list and renderer are
not touched. Checked as §10 asks: `bench-core` and the post-LTO size of `NesBus::tick` (119
instructions) and `NesCore::run_frame` (2 096) unchanged.

## Questions for the owner

1. Where are Squirrel's ROM, profile (`.3dn`) and script? Is the reference movie a `.fm2`, a
   Zaxnes `.mv<n>`, or still to make?
2. MCP: hand-written minimal server (no tokio) — or `rmcp`?
3. The `editor` feature on in the desktop (Windows/Linux x64) release build, or only in a
   separate "editor" build?
4. Seek levels and checkpoint memory (whole movie, ~12 MiB for 10 minutes): agreed?

## Notes

* 4: `ui/zax_ui/movie_bar.rs`: bar over the bottom of the view (shown near the bottom edge, when
  paused, while seeking or dragging): |< < Play/Pause > >|, slider, `frame / length m:ss.cc`,
  `[n%]` while the explorer runs. Tools > "Open a movie file..." (`tools.open_movie`, `.fm2` or
  `.mv<n>`), `--movie <path>` on the command line.
* Explorer (`emulation/movie_explorer.rs`): a thread runs the whole movie with its own core
  (no scripts, no sound) and hands a checkpoint every 60 frames; trusted once one of them has the
  same `machine_digest` (frame and cycle counts, work RAM, picture) as the session's own on the
  same frame. Byte equality of the state blocks fails on Metal Force: 6 bytes of the APU
  resampler follow the host's audio drift correction. Metal Force: 47 454 frames explored in
  40 s; a seek to the end is then instant. Only when no script reads or drives the core.

* 3b: `emulation/movie_timeline.rs` + `host/movies.rs`. A checkpoint (core state, no ROM) on every
  core frame count multiple of 60 while a movie plays, kept for the whole movie; taken in the
  branch the rollback snapshot already uses (one more `bool` read per frame beside
  `rollback_enabled`). `tools.seek_movie <frame>`: from the checkpoint 60 frames before the
  target, the core alone (when no script reads or drives the core) then 60 warm frames with the
  scripts; 12 ms of work per tick so the window stays responsive; sound off meanwhile. A movie
  with a timeline stops paused on its last frame instead of giving the game back. Tests:
  seeks back, forward, past what was played and to the start land on the RAM of a straight
  playback (`fm2_movie_tests`). Whole `zax-app` suite green.

* 3a: `tools.play_movie` with an `.fm2` path powers the game on (`GameSession::reset`: `End`,
  power cycle, `Start`), takes that as the start state and plays the pads; region / ROM / resets
  are warned about, never refused. An annotated `.fm2` is never written over (its notes go to a
  new `.mv<n>`). Run-ahead: `run_ahead_held_at_zero` = the scripts' Q-103 guard **or** a movie
  playing, recomputed when a movie starts or stops, so the loop still tests the same one `bool`.
  Tests `zax-app/tests/fm2_movie_tests.rs` (needs `--features bundled` on Windows for SDL2).
  No way to pick an `.fm2` from the menu yet (comes with the bar / the MCP).

* `zax-harness render --movie x.fm2`: the Metal Force TAS replays **in sync to its end**
  (frame 48 400 shows the ending's CONGRATULATIONS); about 1 ms per frame (44 s for the whole
  movie with conversion, harness, no Lua profile).

* 2026-11-02: worktree created, design written. Nothing compiled yet.

## Owner's answers (2026-11-02)

* Test movie: the Metal Force TAS `.fm2` (`<downloads>/submission4582/MetalForce-DTC5CompV8.fm2`,
  47 454 frames, from power on, one pad, no reset), ROM `Metal Force [Korea][Unl][!].nes`; its
  `romChecksum` is the MD5 of the data after the 16-byte header, checked (test
  `fm2_file_tests`, run with `ZAXNES_FM2` / `ZAXNES_FM2_ROM`).
* The editor can be in every build, but only active behind an option (`--mcp`), and must cost
  nothing otherwise.
* Claude must see and work on the frame on screen, whether the owner plays the game live, plays
  a Zaxnes movie (`.mv<n>`) or an `.fm2`: the MCP server is independent of movies; the seek bar
  needs a movie (either kind).
* MCP implementation: as simple as possible (hand-written, no tokio).
