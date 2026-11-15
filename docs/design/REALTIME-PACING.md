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
