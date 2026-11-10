# Movies

Zaxnes records movies: a machine state to start from and the input of every frame after it, so
that a game played once replays exactly, in the application or in the harness. A movie also
holds notes the player wrote on given frames ("here, this spike shows on my right, it should
not"): a bug seen while playing reaches whoever analyses it with the frame it happened on.

Code: `rust/crates/zax-nes/src/movie.rs` (format, `MovieRun`), `GameSession` (the hook on each
frame's input), `rust/crates/zax-app/src/emulation/host/movies.rs` (commands, files),
`rust/crates/zax-harness/src/render/rom.rs` (`--movie`).

---

## 1. Using it

Tools menu:

| Entry | Command | What it does |
|---|---|---|
| Record a movie | `tools.record_movie` (toggle) | starts recording from the state on screen; again, stops and saves |
| Play a movie | `tools.play_movie` (path in the argument) | submenu of the 15 newest movies of the game chosen (none without a game); loads the movie's start state and plays its inputs, then gives the game back to the player |
| Note on this frame… | `tools.annotate_movie`, Ctrl+M | pauses the game and asks for a note on the frame on screen (Write button or Ctrl+Enter; Escape cancels) |
| Stop the movie | `tools.stop_movie` | stops recording (saves) or playing |

The entries live in the menu bar only: in the desktop's console mode (F1 / Home) the keyboard
drives the pad and Ctrl+M does nothing; F1 or Home goes back to the menu bar first. A movie being
recorded goes on in the console mode, and a state loaded from its state screens takes it back as
from the menu.

* While recording, **loading a save state or rolling back takes the movie back** to that state's
  frame: the inputs from there are dropped and recording goes on ("I fall, I reload, the movie
  picks up from the reload"). Notes written on the dropped frames are kept, marked abandoned.
* Recording while a movie plays takes the game over from the frame on screen.
* Notes can be written while recording or playing; the notes written on a played movie are
  written back to its file when it stops.
* A movie being recorded is saved when it stops, and also when the game stops, is reset, or is
  reloaded (switching the built-in hack).
* Files: `<user data>/movies/<rom>.mv<n>` (Windows: `%LOCALAPPDATA%\Zaxnes\movies`), named after
  the ROM's file name without its extension, `<n>` one past the highest of that ROM (owner
  request); a movie named as before (`<game> <date>.zmv`) is renamed so when its game is chosen.
  Each note is also written to the log (`movie note at frame N: …`). Save states follow the same
  rule (`saves/<rom>.ss<n>`, the quick slot `.ss0`, the thumbnail `.ss<n>.png`; only those of the
  game chosen are listed), and so does the battery RAM (`battery/<rom>.srm`).

## 2. The clock

The movie's clock is the core's frame count (`NesCore::frame_count()`), which a save state holds:
frame `n` of a movie is the picture after `n` input words from the start state, and is the core
frame count at the start plus `n`. At each frame (`GameSession::begin_frame_lent`, the input word
about to go to the core), `MovieRun::input(frame_count, word)`:

* recording: the frame's index is `frame_count - start`; the inputs past it are dropped (the
  notes on later frames marked abandoned), blanks fill a gap (a state from after the end), the
  word is pushed;
* playing: the movie's word replaces the player's; past its end, or before its start, the movie
  is played out and the host gives the game back.

Rollback snapshots and save states are core states: both restore the frame count, which is all
the rewinding needs.

## 3. File format (`.zmv`)

Little-endian throughout.

| Field | Size | Contents |
|---|---|---|
| signature | 8 | `ZAXMOVIE` |
| version | u32 | 1 |
| header | u32 length + bytes | JSON, UTF-8 (below) |
| start state | u32 length + bytes | a core state block with its ROM image (`NesCore::save_state(true)`) |
| inputs | u32 length + bytes | the input word of each frame, u16 (`run_frame`'s word: port 1 in the low byte, port 2 or the Zapper byte in the high one) |
| notes | u32 length + bytes | u32 count, then for each: frame (u32), abandoned (u8, 1 = the recording went back before it afterwards), text (u32 length + UTF-8) |

Header fields, for people and tools (free form; readers ignore what they do not know):

| Key | Contents |
|---|---|
| `game` | the game's title |
| `romSha1` | fingerprint of the original ROM |
| `hackName`, `hackVersion` | the built-in patch and its version (target CRC-32), or null |
| `hackOptions` | the patch's option bits (`$500A`) when recording stopped |
| `widescreenMargin` | the horizontal margin the host offered, in pixels |
| `appVersion`, `savedAtLocal` | the application's version, the local date and time |

## 4. Replaying in the harness

```
zax-harness render --rom <rom.nes> --movie <file.zmv> --run <frame> --out <dir> [--widescreen <px>]
```

restores the start state, sets the movie's hack options (`--hack-options <bits>` overrides them)
and, without `--widescreen`, its margin; feeds the movie's inputs for its frames (`--press`
beyond them); prints the notes with their frames. `--run`, `--poke` and `--press` count frames
from the movie's start; `--run 0` runs none (to read the start state with `--dump-mem`). The
outputs are those of any run: `<game>.png` (3D view), `<game>-2d.png` (the 2D view, as wide as the
margins make it), `--dump-mem`, `--dump-frame`, `--png-nes`, `--save-state`.

`--movie` also takes an `.fm2` movie (FCEUX's text format, read from its public documentation,
`zax-nes/src/movie/fm2.rs`): it plays from power on; a region (`palFlag`) or `romChecksum` that
differs from the ROM's is warned about, resets in it are not replayed yet, and movies that start
from a savestate, use the binary encoding, a Zapper, the Four Score, the FDS or VS. commands are
refused. Checked on a complete Metal Force TAS (47 454 frames): in sync to its end.

To analyse a note: replay to its frame and to the frames around it (a few before, a few after),
and look at the view the player used (the 2D view for a player in 2D).

`--trace-ram <file>` writes the CPU's 2 KiB of work RAM after every frame run, one block after
the other (2 KiB a frame): what a game did over a whole movie, at once (each change of screen, a
variable's course), instead of one replay per frame looked at.

## 5. Determinism and its limits

* A replay is exact when the game sees what it saw while recording: the same ROM and built-in
  patch version (`hackVersion`), the same hack options, and, for a game whose widescreen hack reads
  the host margin (Mega Man 2 spawns its enemies by it), the same margin. A movie made with
  another version of a hack drifts.
* The margin is recorded once, when recording stops: changing the window or the widescreen
  settings in the middle of a recording is not replayed.
* Lua scripts of a profile that change the input are part of the recorded word (the word is
  taken after them); scripts that write memory run again as they did.

## 6. `.fm2` on the FCEUX engine (lot FCEUX-48)

**A build with the `fceux-core` feature plays an `.fm2` on FCEUX**, which is the only engine it
stays in sync on, and that is the owner's own reason for having the engine at all: *play a complete
TAS and edit the 3dn profile on the fly*.

### How to open one

Exactly as before — **Tools ▸ Play a movie**, the file browser (`.fm2` is already in its filter),
`--movie <file.fm2>` on the command line, or `tools.open_movie` over MCP. Nothing new to learn: the
engine is chosen by the file, not by the user.

What happens then, and it is said on screen:

* the game's session **switches to FCEUX** for that playback. The game's own **Emulation engine**
  option is *not* changed — in the options window, in `games.json`, or at the next load. A movie is
  not a change of mind;
* the machine is re-powered with the region (`palFlag`) and the RAM fill (`RAMInitOption`,
  `RAMInitSeed`) the movie's header asks for, which is the whole reason playback goes through
  FCEUX's own movie mode: FCEUX reseeds its random RAM fill on every power-on *unless a movie is
  active*, so a movie's seed cannot be honoured any other way;
* **stopping the movie does not switch back.** The game goes on from the frame it is on, on FCEUX,
  until it is loaded again — an engine change re-powers the machine and must never happen mid-game,
  and a stop has no reason to throw away the frame you are looking at. The message says so.

Everything around the playback is what it always was: the movie bar, pause, frame step, seeking
(backwards through savestates taken while it plays, forwards by replaying), the 3D view live, the
profile's Lua hooks, and **the profile editor** — `F5` / `tools.reload_script` while the movie plays
or is paused reads the profile again and comes back to the same frame with the new scripts.

Three differences from a Zaxnes movie, each for a reason:

| | |
|---|---|
| **no note on a frame** (`Ctrl`+`M`) | a note is written to a Zaxnes movie file and an `.fm2` is never written over. The entry is not offered |
| **no "explored" share on the bar** | the share comes from a thread replaying the movie in a *second* core, and FCEUX keeps one machine per process. The checkpoints come from the playback itself instead |
| a reload is always **exact** | the movie is loaded again by the reload, so the frame is reached by replaying to it. On a Zaxnes movie a reload is fast unless you ask for exact |

### What is refused, and why

A movie whose header asks for something this build has not is **refused with the reason named**, and
your game is left running. There is no half-switched state: the header is read and judged before
anything is committed.

| the header says | the message says |
|---|---|
| `FDS 1` | a Famicom Disk System disk — this build loads iNES images and has no disk BIOS |
| `NewPPU 1` | FCEUX's new PPU — the tile capture the 3D needs does nothing there |
| `fourscore 1` | a Four Score (four pads) — the engine drives two |
| `microphone 1` | the Famicom microphone |
| `port0`/`port1` not a standard pad, or `port2` set | an input device other than a standard pad / an expansion-port device |
| a `romChecksum` that is not this ROM's | recorded on another ROM, so it would not replay |
| anything that is not a version-3 `.fm2` | not a version 3 .fm2 movie (or the file is truncated) |

A movie that states **no** ROM checksum is played rather than refused: claiming nothing is not
claiming the wrong thing.

A **`.fcm`** (FCE Ultra's binary format) is *not* played on FCEUX — FCEUX's own movie code refuses
that format outright — so it keeps Zaxnes's clean-room importer of §4, as does every `.fm2` in a
build without the feature. Nothing about the default build's movie behaviour changed.

### Measured

*Super Mario Bros.* with its 3dn profile, a 5 400-frame movie, through the real host headless
(`docs/progress/FCEUX-48.md` step 5): the `.fm2` replays the recorded run **identically at every
one of the 5 400 frames**; an interrupted playback (pause, frame steps, a 300-frame seek back, a
resume) shows the same frames as an uninterrupted one; a profile reload at frame 2 000 comes back
to a byte-identical machine; and playback with the whole 3D path runs at **1 726 frames/s**, about
28× real time.
