# REALTIME-PACING — Fidélité+ part B (design, 2026-11-15)

Part B of the Fidélité+ goal (`docs/design/HALF-CYCLE.md`, "The goal"): the machine advanced by
real time, the picture leaving at the console's rate, the pad read when the game strobes it.
Common base by the owner's placement rule (it costs the emulation nothing), offered as an option
of the application because it changes the feel of every game.

## What the hardware does, and what Zaxnes does today

| | Console | Zaxnes today |
|---|---|---|
| Frame rate | 60.0988 Hz NTSC (29 780.5 CPU cycles a frame), 50.0070 Hz PAL | 60 and 50 nominal (DEC-R2-T3); the audio resampler is calibrated on them (REQ-CORE-070) |
| Pad | read when the game strobes `$4016` and clocks the bits, at any point of the frame | one input word sampled by the host before the frame (`run_frame(input_word)`) |
| Time | continuous | whole frames, run when due on the display or audio clock |

BeesNES (MIT) runs its machine from host time converted to master clocks (`CSystem::Tick`) and
polls the pad on the put cycle of the strobe (FIDELITY-LEADS A2).

## Three pieces, separable

1. **Exact rate** (small): the frame pacer and the resampler at the console's rate
   (60.0988 / 50.0070) instead of the nominal one. The display then drifts against a 60 Hz screen
   by one frame every ~10 s: a repeated frame where the console's own picture would also slip on a
   60 Hz CRT-less display. VRR screens show it exactly.
2. **Input at the strobe** (the latency gain): the host keeps writing the pad's state into a shared
   cell as it changes; the core's `$4016` strobe reads the cell's current value instead of the
   frame's word. Needs the core to run close to real time *inside* a frame (piece 3), else "current"
   means the start of the frame anyway. Movies, netplay and rewind keep the per-frame word: the
   option records, for replay, the input the core actually read at each strobe.
3. **Sliced real-time run**: a core entry `run_until(master_clock)` (or `run_cycles(n)`) that stops
   at any CPU instruction boundary, called by the host in slices (1 ms, or per audio buffer), the
   picture presented when a frame completes. The frame records (3D, Lua, widescreen, packs) are
   still per frame: their boundary is unchanged, so they keep working. Run-ahead keeps whole frames
   (it replays from a state), so with run-ahead on, piece 3 is off and the UI says so.

## Options (owner, 2026-11-15: Fidélité+ may be split)

Each piece is its own option: the exact rate (piece 1), the input at the strobe (pieces 2-3, off
while run-ahead is on). A Fidélité+ preset turns on these and the others (machine A, console
model, audio), as the console-model menu already presets its settings.

## Cost and placement

Nothing in the core's per-cycle path: the strobe already syncs the bus; reading a cell instead of
a field is the same instruction. `run_until` is a second exit test in the frame loop, compiled
apart like the Fidélité+ kind (`Fid<K>`) so the standard loop is unchanged. The application side
(pacer, resampler, slices) is where the work is.

## Judge

* Rate: a 10-minute run counts 36 059 frames NTSC (60.0988 x 600), not 36 000.
* Latency: a test ROM that strobes late in the frame (e.g. after vblank of the next frame minus a
  few lines) reads a button pressed mid-frame in that frame; measured with the input-lag test ROMs
  the owner may provide, and with an input timestamp log.
* Audio: no underrun or pitch drift over a 10-minute run at the exact rate.

## Plan

| Step | What |
|---|---|
| B1 | Exact rate in the pacer and the resampler, option `video.exact_rate` (off), tests on frame counts |
| B2 | Core `run_until` / per-instruction exit and the pad cell read at the strobe; equality tests (whole frames vs slices give the same machine) |
| B3 | Application slices, presentation of the last finished frame, input timestamping, UI (run-ahead interplay) |
| B4 | Validation: rate, latency, audio, every existing test |

## B3 as built (2026-11-15)

* Option `emu.realtime_input` ("Real-time pad", off), its own switch beside `emu.exact_rate`.
* Core: `NesCore::run_frame_slice` (B2). A whole frame (`run_frame`, `run_frame_ahead`) asked
  while a sliced frame is in progress finishes that frame first, so the switch can go off at any
  instant.
* Session: `GameSession::run_frame_slice_timed`. A slice that starts a frame runs the first half
  of a frame (the scripts' input hook, the movie word); the one that ends it runs the second half
  (capture, conversion, frame hooks), so profiles, 3D and widescreen see whole frames as before.
  The scripts' pad of the frame is kept for the slices in between, which merge it with the pads of
  their own time.
* Host: `EmulationHost::run_due_slices`. Each tick turns its real time into CPU cycles
  (`FramePacer::slice_frames` x `RegionTiming::cycles_per_frame`, at most the catch-up backlog),
  runs to that cycle with the pads of the tick, and finishes every frame that ends on the way; the
  per-frame work (achievements, rollback and movie checkpoints, step-back copy) runs when a frame
  starts. The slices run on the host's thread: the emulation thread runs whole frames.
* Off, without a word, when run-ahead is on (it replays whole frames from a state, and is on by
  default), when a movie is recorded or played (its word is per frame) and on an engine without
  slices (FCEUX, NesHawk). The option's text says so.
* Granularity: the pads are polled once per host tick (the display frame), so a frame spreads over
  two ticks at most on a 60 Hz display; finer input needs a faster input poll (B4 measures it).
* A savestate made mid-frame keeps the machine at that instruction; the slice's frame bookkeeping
  is not in it, so the frame after a load ends at the next vblank as a whole frame would.

## What B3 gives a player, and what is missing (2026-11-15)

Owner question (an XInput pad): with the pads read once per host tick and the slices run as a
burst in that tick, B3 brings no lower lag: the game reads a fresher pad mid-frame, but the frame's
picture is shown up to a tick later, and run-ahead (the real lag reducer) must be off. B3 is a
fidelity setting (when the pad is read in emulated time) and the groundwork. Its help text says so.

For a real latency gain, two pieces, planned as B5:

1. A pad poll much faster than the display: a thread reading XInput (and SDL pads) at ~1 kHz into
   the live pad cell, each sample timestamped.
2. The machine run along real time on the emulation thread (slices of ~1 ms, sleeping to the
   clock), and the last finished frame shown at the display's vsync.

Then a button pressed while the console builds a frame can be seen in that frame, as on the
console; the gain (up to about half a frame on average) is to be measured in B4/B5.
