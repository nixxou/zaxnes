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

## B4 as measured (2026-11-15)

* Rate: with `emu.exact_rate` the host's pacer runs at 60.0988 Hz; ten simulated minutes give
  36 059-36 061 frames (the pacer hands a frame out at once and starts frames 1 ms early),
  59 more than the nominal rate counted the same way (`realtime_slices_tests`).
* Audio: 3600 frames at 48 kHz give 3600 x 48 000 / 60.0988 samples within one millisecond at
  the exact rate, 3600 x 800 at the nominal one: no drift, no pitch change (`frame_timing_tests`,
  `fidplus_b4_the_exact_rate_makes_the_samples_of_real_time`).
* Latency: a button given in the middle of a frame is read by the game's next strobe in that frame
  (`fidplus_slice_tests`); the host's slices spread a frame over half-frame ticks, the frames still
  at the right count (`realtime_slices_tests`). The gain for a player waits on B5 (pad poll faster
  than the display, real-time emulation thread); measured then, with an input timestamp log.
* Every existing test passes at head e07e6a79 (FIDPLUS-A final validation).

## B5 waits on an owner decision (2026-11-15)

A pad poll faster than the display needs a thread reading the pads on its own. SDL's game
controller state is updated by the main thread's event pump (not thread-safe with it), so the
poll needs either:

1. a new dependency with a safe API reading XInput / HID pads from any thread (`gilrs`): a
   download, gated by the owner; or
2. a direct `XInputGetState` call through `windows-sys` (already a dependency): an FFI call, so
   `unsafe`, which the safe-Rust rule allows only for performance and behind an option.

Without one of them B5 would run the machine along real time with the pads still read once per
display frame, which gains nothing. Parked until the owner chooses; FIDPLUS-C goes on meanwhile.

## B5 step 1: the software latency bench (2026-11-15)

Owner's answers: gilrs is allowed; latency is measured in software (no camera or button rig);
RawInput will be needed later anyway (light guns), not now.

`ext-tools/padbench` (outside the repository, like the netlist bench: it needs `unsafe` FFI for
RawInput and XInput, which a measuring tool may use and Zaxnes may not) timestamps, on one monotonic
clock, the moment each API first sees a button go down after all were up:

| Backend | How |
|---|---|
| `rawinput` (reference) | `WM_INPUT` on a message-only window, buttons parsed from the HID report |
| `xinput-spin` | `XInputGetState` polled as fast as possible |
| `xinput-1khz` | `XInputGetState` every millisecond (the B5 plan) |
| `gilrs` | gilrs 0.11's event queue (Windows.Gaming.Input on Windows) |
| `sdl-event` | SDL's controller events, pumped continuously |
| `sdl-frame` | SDL's state read once per 60 Hz frame, as Zaxnes reads it today |

Run `padbench.exe 60`, press and release one pad button a few dozen times; it prints each press's
delays against RawInput, then mean, median, 95th percentile and maximum per backend. The numbers
decide B5's poll: the backend closest to RawInput, polled from its own thread into the slices of B3.
Waiting for a run with the owner's pads (Xbox / XInput and a HID pad if possible).

## B5 step 2: measured with the owner's pad (2026-11-15)

Xbox 360 pad (XInput), 41 presses, padbench with a focused window, reference `XInputGetState`
polled as fast as possible (RawInput sees nothing of an Xbox 360 pad):

| Backend | Median | 95th percentile | Max |
|---|---|---|---|
| SDL events pumped continuously | +0.12 ms | 0.44 ms | 0.82 ms |
| XInput at 1 kHz (connected slot only) | +0.24 ms | 0.96 ms | 0.98 ms |
| gilrs 0.11 (Windows.Gaming.Input) | +4.5 ms | 8.4 ms | 8.8 ms |
| SDL read once per 60 Hz frame (Zaxnes today) | +9.3 ms | 15.7 ms | 16.2 ms |

* A first run without a window saw nothing in gilrs or SDL: both deliver pad input to a focused
  application only. A first XInput run read all four slots and had 55-87 ms outliers: reading an
  empty slot can block; read the connected ones only.
* gilrs is the slowest of the fast backends (it appears to poll every ~8 ms). Choice for B5 left to
  the owner: an XInput thread at 1 kHz (Windows, `unsafe` FFI behind `emu.realtime_input`), or SDL
  pumped much more often on the main thread.
## B5 step 3: the pad's own report rate (2026-11-15)

`padbench rate` timestamps every new XInput `dwPacketNumber` while the stick moves (or just drifts):
the owner's Xbox 360 pad, wired, reports every **8.00 ms (125 Hz)**, 5th-95th percentile 7.93-8.03
ms, its default USB `bInterval` of 8. Its own latency is therefore 4 ms on average; Zaxnes adds 9.3 ms
today (one read a frame) and about 0.3 ms with a 1 kHz read. Public figures (Gamepadla's hardware
tester, community USB descriptors): 100-250 Hz for most pads over Bluetooth, dongles and many USB
pads, about 200 Hz for an Xbox 360 wireless dongle, 250 Hz for the DualShock 4 / DualSense over USB;
Bluetooth LE allows at most ~133 reports/s at its 7.5 ms minimum interval. Reading at 1 kHz asks
nothing of the pad: it only stops adding delay on top of the pad's own.
## B5 step 4: the real-time pad, on by default (2026-11-16)

Owner decision (2026-11-16): read the pads more often through SDL, behind an option on by default.
`emu.realtime_input` now defaults to **on** and covers both B3 (slices of real time, the pads read
at the game's strobe) and this step:

- `Application::run_the_realtime_pad` runs just before `swap`. While the frame would wait for the
  refresh, it sleeps 1 ms, pumps SDL's events, samples the two players (`sample_players`: buttons
  only, no command edge, no capture, no pointer) and runs the machine up to the present
  (`EmulationHost::realtime_slice`, which keeps the displayed frame's hotkey bits). It stops 2.5 ms
  before the refresh is due (last swap + paced budget) so the swap is not missed.
- The frames finished during that wait are counted with the next displayed frame (`carried_frames`);
  a quit seen by the inner pump is acted on at the next frame (`close_requested`).
- Nothing happens without a refresh to wait for (no vsync and no cap), with run-ahead, during a
  movie, or when the game is not running. The same code runs on Windows, Linux and Android: all
  three read the pads through SDL.

The game therefore reads a pad at most about 1 ms old (plus the pad's own report interval) instead
of one sampled once a refresh: about 9 ms sooner on average, 16 at worst at 60 Hz. The run-ahead
tests turn the option off: they compare whole frames across N, and the real-time pad changes, by
design, the instant the input reaches the game at N = 0.