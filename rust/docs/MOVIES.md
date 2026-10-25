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
| Play a movie | `tools.play_movie` (path in the argument) | submenu of the 15 newest movies of the `movies` folder; loads the movie's start state and plays its inputs, then gives the game back to the player |
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
* Files: `<user data>/movies/<game> <date>.zmv` (Windows: `%LOCALAPPDATA%\Zaxnes\movies`); each
  note is also written to the log (`movie note at frame N: …`).

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

To analyse a note: replay to its frame and to the frames around it (a few before, a few after),
and look at the view the player used (the 2D view for a player in 2D).

## 5. Determinism and its limits

* A replay is exact when the game sees what it saw while recording: the same ROM and built-in
  patch version (`hackVersion`), the same hack options, and, for a game whose widescreen hack reads
  the host margin (Mega Man 2 spawns its enemies by it), the same margin. A movie made with
  another version of a hack drifts.
* The margin is recorded once, when recording stops: changing the window or the widescreen
  settings in the middle of a recording is not replayed.
* Lua scripts of a profile that change the input are part of the recorded word (the word is
  taken after them); scripts that write memory run again as they did.
