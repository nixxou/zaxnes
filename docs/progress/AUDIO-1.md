# AUDIO-1 — no more crackles at the audio output

Branch `audio-fix`, from `v0.4.2`. Worktree `wt-audio`.

The signal processing of the audio path is sound (band-limited step resampler, 20 Hz
high-pass, PAL/NTSC sample counts, speed by time compression, run-ahead muting). The defects
are in **how the output device is fed**. Everything this lot changes sits *after* the APU's
sample ring: the emulation core, the resampler and the filters are untouched.

## Plan — 10 steps

- [x] **1. Plan.** This file.
- [x] **2. Verify the findings.** Read the audio path and confirm or refute each of the five
      findings with `file:line`. Write the question ticket for the spec clauses the owner's
      design contradicts (REQ-AUD-003 silence blocks, the latency note, the ring capacity).
- [x] **3. Pull output.** The SDL device is opened with a callback that reads the APU sample
      ring directly on SDL's audio thread. Menu music and interface sounds are *precomputed*
      by the mixer on the main thread into an auxiliary stereo ring, so the callback never
      touches a file, an allocation or the mixer's state machine. No silence is ever inserted
      while data is merely late: on a real underrun the last sample is held and faded to zero
      over a few milliseconds, the event is counted, and the signal is faded back in when data
      returns.
- [x] **4. Latency setting and prefill.** `audio.latency_ms` (default 60, range 20–200) in the
      options' Audio tab, in the settings file, with the English and French strings. Playback
      starts, and restarts, only once the ring holds the configured latency.
- [x] **5. Rate control on what is really buffered.** The ring fill is measured as a latency in
      milliseconds by the audio callback and averaged over about a second; the correction is a
      dead band, a capped proportional term and a slowly accumulating term. Only at 100 % speed
      and not while recording. A far drift resynchronises.
- [x] **6. Pause, resume, state load, rewind.** A short ramp to zero when the console stops
      producing; on resume a clear, a prefill and a ramp in. The same on save-state load and on
      rewind. `AudioOutput::flush` is actually called.
- [x] **7. Counters.** Underrun events, average latency, current rate adjustment, minimum and
      maximum ring fill, in the performance overlay and logged once per second when the frame
      profiler is on.
- [x] **8. Tests.** Ring/callback logic without a device; rate control against a simulated
      device clock; main-thread stalls.
- [x] **9. Measurements.** Main-thread frame time before and after, 2D and 3D, desktop release
      build; golden-core byte-identical to `v0.4.2`; Android and PortMaster `cargo check`.
- [x] **10. Final pass.** fmt, clippy `-D warnings`, `cargo test --release`, the ignored tests
      with the ROM variables, journal entry.

## Design

```
                main thread                          SDL audio thread
   ┌──────────────────────────────┐          ┌──────────────────────────────┐
   │ core → APU ring (unchanged)  │ ───────► │ AudioStream::render          │
   │                              │  mono    │   console samples, partial   │
   │ AudioMixer (unchanged state  │          │   reads allowed              │
   │  machine: fades, track       │ ───────► │ + aux ring (music + sounds)  │
   │  choice, file loads) fills   │  stereo  │ + per-sample gain follower   │
   │  the aux ring ahead of time  │          │ + prefill gate, hold & fade  │
   │                              │ ◄─────── │ + counters (atomics)         │
   │ rate control reads the        │  stats   └──────────────────────────────┘
   │  measured latency             │
   └──────────────────────────────┘
```

The console signal never passes through the main thread again, so a long rendered frame can no
longer starve the device, and a late producer can no longer cause a block of zeros.

## Notes per step

### Step 2 — the findings, verified against the code at `v0.4.2`

All five are **confirmed**. Line numbers are those of `v0.4.2`; paths are relative to
`rust/crates/`.

**1. Silence blocks inserted mid-stream — confirmed.**
`zax-platform/src/audio.rs:105-114` (`AudioOutput::pump`) computes
`blocks_to_queue(self.queue.size())` and calls `fill` that many times unconditionally; it has
no way to know whether the filler has data. `blocks_to_queue`
(`zax-platform/src/audio.rs:42-44`) tops the SDL queue up to `BLOCK_COUNT = 4` blocks
(`audio.rs:23`). `zax-app/src/audio/audio_mixer.rs:325-335` (`mix_emulation`) asks the source
for `frames` samples and, when `written < frames`, *returns having mixed nothing*: the block
keeps the zeros `fill` wrote at `audio_mixer.rs:260`. The source
(`zax-app/src/emulation/host/jobs.rs:319-322`) is `AudioReader::read_samples`, whose ring read
is all-or-nothing — `zax-nes/src/apu/mod.rs:140-142` returns `Ok(0)` without consuming
anything when `available < needed`. So a ring holding 511 samples yields a full block of zeros
while the SDL queue still holds up to three other blocks: a 10.7 ms hole with a discontinuity
at each edge. This is REQ-AUD-003 to the letter; see Q-153 §1.

**2. Rate control settles below its target — confirmed.**
`zax-app/src/emulation/host/mod.rs:1245-1267` (`correct_audio_drift`). It is proportional only
(`-error * 0.002`, `mod.rs:1259`) with no integral term, so a steady clock offset leaves a
steady error: at the ±0.5 % cap (`Apu2A03::MAX_RATIO_ADJUSTMENT`, `apu/mod.rs:274`) the
residual error needed to produce even a 0.1 % correction is 50 % of the target. The target is
`2 * BLOCK = 1024` samples ≈ 21 ms (`mod.rs:1246`), measured on the **APU ring only**
(`mod.rs:1253`) — the SDL queue, which holds the other 30–40 ms, is never looked at. It also
runs *before* the frames and before the pump in both branches of the tick
(`mod.rs:672-688`). The deficit it leaves is paid in the silence blocks of finding 1.

**3. Output buffer too short for a 3D main thread — confirmed.**
`BLOCK_COUNT = 4` blocks of 512 frames (`zax-platform/src/audio.rs:19-23`) is 42.7 ms at
48 kHz, and a partly played block counts as pending (`audio.rs:43`), so the real floor is
nearer 32 ms. It is refilled once per rendered frame, from `pump_audio`
(`zax-app/src/application.rs:1876-1897`), on the main thread. `video.half_rate_3d` renders at
33.3 ms (`zax-settings/src/table.rs:800-808`), which is at or past that floor before any hitch.

**4. Pause / resume / save-state load — confirmed.**
The console gain is the 3.33 s linear fade of REQ-AUD-020
(`zax-app/src/audio/audio_mixer.rs:307-310`, driven from
`zax-app/src/emulation/host/mod.rs:1032-1037`), but the ring runs dry one or two blocks after
the last emulated frame, so what is actually heard is the abrupt end of finding 1 while the
gain is still ≈ 1 — a hard cut, not a fade. On resume the ring holds whatever sub-512 fragment
was left (nothing drains it: `apu/mod.rs:140-142` never consumes a short read), so that stale
fragment is the first thing played. After a state load the ring is deliberately emptied
(`apu/mod.rs:496-498`, called from `apu/mod.rs:933`), and after a pacer reset it is empty too,
so the pump interleaves silence blocks with signal blocks for as long as the producer is
behind. `AudioOutput::flush` (`zax-platform/src/audio.rs:117-119`) exists and has **no caller**
anywhere in the workspace.

**5. No headroom; gain applied per block — confirmed.**
`mix_emulation` reads `self.emulation_gain.value()` once per block
(`zax-app/src/audio/audio_mixer.rs:336`) and applies that single value to all 512 frames, so a
fade is a staircase of 10.7 ms steps. The only limiting is the hard clamp at
`audio_mixer.rs:264-266`, and the APU's own `OUTPUT_GAIN` of 1.526 (`apu/mod.rs:284`) plus
expansion audio and the high-pass overshoot can reach it.

**Spec conflicts recorded in `spec/changes/Q-153.md`**: the silence-block clause of
REQ-AUD-003 (§1), the « < 40 ms » latency note (§2), the ring capacity against a 200 ms
latency (§3, the core's ring is fixed at 208 ms at 48 kHz and
`zax-nes/tests/audio_pacing_tests.rs:262-274` asserts that bound), and the 50 ms rollback hole
of REQ-AUD-012 against a full prefill (§4).

### Step 3 — the pull output

`zax-platform/src/audio.rs` is now a callback device: `AudioOutput::open` takes a factory, hands
it the rate and the block size SDL negotiated, and gives SDL an `AudioRender` to pull from. The
`AudioQueue`, `BLOCK_COUNT` and `blocks_to_queue` are gone with the push model. **No `unsafe`
was needed**: the `AudioCallback` trait of the `sdl2` crate is safe Rust, so the exception the
work order allowed is not used anywhere.

`zax-app/src/audio/stream.rs` is the renderer, and all the policy lives there, device-free and
testable:

* the console is read from the APU ring through `ConsoleSamples` (`ApuSamples` over
  `AudioReader`; a test installs a fake). Partial reads: the ring's own read is all-or-nothing,
  so it is asked for exactly `min(available, wanted)`.
* a prefill gate: nothing is read until the ring holds the target (the first start) or half of
  it (any later resynchronisation, Q-153 §4), never less than one block.
* on a real drought, one `underruns` event, the last sample held and ramped to zero over
  `RAMP_MS` = 6 ms, and the gate closes again so the next blocks do not each underrun.
* the menu music and the interface sounds come from a separate stereo ring the mixer fills
  `PROGRAM_AHEAD_MS` = 48 ms ahead on the main thread; a shortfall there is ramped too and is
  *not* counted, since it is a dip of a signal at gain 0,02 and not a hole in the game's sound.
* the console gain is followed **per sample** (`GAIN_SLEW_MS` = 8 ms), which also removes the
  block-sized staircase of finding 5.
* the final clamp happens once, on the sum, in the callback.

`AudioMixer` keeps the whole fade state machine of SPEC-11 §3.3 and loses only the console:
`set_emulation_source`, `AudioSampleSource`, `mix_emulation` and `underrun_blocks` are gone, and
`fill` no longer clamps (the callback does). The host publishes `emulation_gain()` and whether
the console is producing into `AudioControl` on every `publish_state`.

The host's `audio_device_open: bool` became `audio: Option<AudioLink>` — the control block, the
console slot and the device's rate. `EmulationHost::audio_report()` is the one line the overlay
and the profiler's log share.

Locks on the audio thread: the APU's ring (the core's own, one acquisition per callback instead
of the producer's one per sample), the console slot, and the programme ring. All three hold only
numbers and a memcpy; the critical sections are sub-microsecond. A lock-free ring would mean
changing the core, which the work order forbids.

One unrelated test fix: `asset_free_startup_tests.rs`'s repository scan excluded `rust/target`
with `Path::starts_with`, which matches whole components and therefore missed the `target-au`
directory this lot builds into — the `.obj` files of a bundled SDL2 then read as assets of the
repository. It now skips any `rust/target*`, so the test no longer depends on what
`CARGO_TARGET_DIR` is called.

Build note for whoever resumes: `cargo test -p zax-app` needs SDL2. There is no `SDL2.lib` on
this machine, so the runs below use `--features zax-app/bundled` with Visual Studio's CMake on
`PATH` (`C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\IDE\CommonExtensions\
Microsoft\CMake\CMake\bin`). `TMP`/`TEMP` must be **Windows-style** paths or `link.exe` cannot
create its temporaries.

Result: `cargo fmt --check` clean, `cargo clippy -p zax-platform -p zax-app --all-targets
--features zax-app/bundled -- -D warnings` clean, every `zax-app` and `zax-platform` test green
(the four REQ-AUD tests that drove the console through the mixer now drive the stream, and
`req_aud_003` asserts the new hold-and-fade instead of the block of zeros).

### Step 4 — the latency setting and the prefill

`audio.latency_ms`, `SettingKind::Int`, default 60, domain 20–200, scope User, theme Audio
(`zax-settings/src/table.rs`, in `reimplementation_extras` with the other Rust-only settings).
It is persisted by key, like every other setting, so no field number of the original's format is
needed. It appears in the options window's **Audio** tab on its own — the theme places it, and
`draw_int` draws a whole slider over the domain — with `setting.audio.latency_ms` in English
("Output latency (ms)") and French ("Latence de sortie (ms)").

Deliberately **no** libretro core option: REQ-AUD-007 leaves the latency to the host in that
frontend, and the normative list of REQ-PERS-081 is not to grow. The slider needs no
discretisation since the setting is whole.

Two clamps:

* the setting's own domain, 20–200 ms;
* `usable_latency_ms`, which bounds the target to what the core's ring can hold with room for
  the rate controller (`capacity − CONTROL_HEADROOM_MS`, 70 ms = the 50 ms resynchronisation
  threshold plus one emulated frame of production). At 48 kHz the ring is 10 000 samples
  ≈ 208 ms, so the effective maximum is about 138 ms; the clamp is logged once when it bites.
  `core_ring_samples` mirrors the core's `max(10 000, rate / 5)` because the target has to be
  decided when the device opens, before any core exists — a read of the core's rule, not a
  change to it. Q-153 §3 records the conflict with the work order's "capacity at least twice
  the latency".

The prefill gate itself is in the stream: the first start waits for the whole target, any later
resynchronisation for half of it, never less than one block. `EmulationHost::resynchronise_audio`
is the one entry point — it empties the core's ring and bumps the control block's counter — and
is used here by a latency change; step 6 wires it to the rest.

Not added to the handheld options screen: that screen is a short curated list (owner request)
and the work order asks for the options' Audio tab.

Two test registries had to learn the new key: `zax-settings/tests/catalogue.rs` (the list of
Rust-only additions beyond REQ-PERS-001) and nothing else, since the setting is not a libretro
option. New test `q153_the_prefill_gates_playback`: one sample short of the target the output is
exact silence and the ring is untouched; the sample that completes it opens the gate, the signal
is ramped in rather than switched on, and no under-run is counted for the wait.

fmt and clippy `-D warnings` clean on `zax-settings`, `zax-platform`, `zax-app`; every test of
those three crates green (15 in `audio_tests`).

### Step 5 — the rate control

`zax-app/src/audio/rate_control.rs`, a state machine of three numbers with its own unit tests;
the host only feeds it the error and passes the result to the core.

The error is **the latency the audio thread measured, in milliseconds, minus the configured
target** — `AudioControl::average_fill`, recorded in the callback after each read and smoothed
with a one-second weight. The old controller measured `available_samples()` from the main
thread, just after producing a frame, against a target of 1024 samples, and never saw the SDL
queue that held most of the latency.

The law:

| term | value | why |
|---|---|---|
| dead band | 3 ms | the pitch does not wander for an error nobody can hear |
| proportional | 1,25e-4 per ms, capped at 0,25 % | reacts to a real excursion |
| accumulation | 4e-5 per ms per second, capped at 0,06 % | cancels the clock offset, so the latency settles *on* its target |
| leak of the accumulation | 0,05 per second | the damping, see below |
| resynchronisation | 50 ms, with a 2 s cooldown | a stall or a fast-forward burst is not a clock offset |

**The leak is not decoration.** A dead band in front of the proportional term leaves the loop
undamped inside it: the ring's fill and the accumulated correction form an oscillator of period
`2π / √(1000 · OFFSET_GAIN)` ≈ 31 s and nothing opposes it, so the latency would swing a couple
of milliseconds for ever (and, with the one-second lag of the measurement adding phase, slowly
grow until it left the dead band). The leak damps it for a residual error of
`LEAK · drift / OFFSET_GAIN` ≈ 0,4 ms at a drift of 0,03 % — far inside the dead band.

Gating, as the work order asks: the correction runs only in Running, at exactly 100 % speed,
outside a fast-forward burst and outside a recording (REQ-AUD-011 — the core retimes its own
resampler for the speed factor — and a recorded movie must not depend on this machine's sound
card). Elsewhere `idle` sends 0 to the core but **keeps** what was learnt of the clock, since
the sound card has not changed. A resynchronisation does clear it: the fill it was learnt from
is gone.

Measured, by simulating the closed loop at 60 steps a second with the one-second measurement lag
included (`a_device_clock_off_by_three_hundredths_of_a_percent_settles_inside_the_dead_band`):
a device clock 0,03 % fast or slow, over **one simulated hour**, never moves the fill more than
**1,36 ms** from the target after the first minute and never takes the ring below 40 ms of the
60 ms target — so the stream has nothing to under-run on. At ten times that drift (0,1 %) the
accumulation saturates, the proportional term carries the rest, and the ring still holds above
30 ms.

One subtlety fixed in the stream while it was being measured: the averaged fill is now *set*
rather than blended while the prefill gate is shut, so that on the block where the gate opens the
controller reads the real fill. Blending from a stale value would have shown it a 60 ms error at
the start of every game and had it resynchronise for nothing.

`audio_rate_adjustment` on the host turned out to be a duplicate of `RateControl::adjustment()`
and is gone.

fmt and clippy `-D warnings` clean; `zax-app` (8 new unit tests in `rate_control`) and `zax-nes`
tests green.

### Step 6 — pause, resume, state load, rewind

One entry point, `EmulationHost::resynchronise_audio`: it empties the core's ring, bumps the
control block's resynchronisation counter and clears what the rate controller had learnt. The
stream then waits for half the configured latency (30 ms at the default) and ramps the signal in.

Wired at:

* **`set_state`, entering Running** — the resume of a pause, and every transition a load or a
  rollback makes. Leaving Running needs nothing of its own: `publish_state` tells the audio
  thread the console has stopped producing and the stream ramps out over `RAMP_MS` = 6 ms. That
  ramp *is* the "short ramp to zero when the console stops producing" the work order asks for;
  the 3,33 s fade of REQ-AUD-020 is still there underneath and still governs the gain, but what
  the ear hears at the moment the ring runs dry is now a 6 ms ramp instead of the hard cut of
  finding 4. Leaving Running also restarts the controller, since the fill it learnt from is
  about to drain.
* **`finish_launch`** — a game starting, and a slot loaded **while the game plays**, which
  leaves the state at Running so `set_state` returns early. The explicit
  `drop_pending_samples()` that used to sit there is gone: `resynchronise_audio` does it and
  arms the gate as well.
* **the rollback** (`save_states.rs`) — same reason: a rewind taken while running does not
  change the state. REQ-AUD-012 tolerates the discontinuity and asks for no silence over 50 ms;
  the mid-stream prefill is half the latency, so 30 ms at the default (Q-153 §4 records that a
  user who chooses a latency above 100 ms accepts a longer hole).

**On `AudioOutput::flush`.** Finding 4 is right that it was never called, but in a pull model it
has no successor: there is no queue of blocks outside the ring to drop, so the two things that
needed flushing — the stale sub-512 fragment in the core's ring and the blocks already handed to
SDL — are now handled by `resynchronise_audio` and by the gate. The method is gone rather than
left uncalled. `AudioOutput` keeps `pause`/`resume` for the device itself; nothing calls them,
because REQ-AUD-013 says losing focus must not cut the sound and an explicit suspend already
pauses the game, which silences the console through the ramp.

New test `q153_a_resume_clears_the_ring_and_ramps_back_in`: after a resynchronisation nothing
stale comes out, the gate opens at half the latency and not before, the first sample of the new
signal is still near zero (ramped in, not switched on), and the whole thing counts no under-run.

fmt and clippy `-D warnings` clean; all `zax-app` tests green (16 in `audio_tests`).

### Step 7 — the counters

`AudioControl` carries them all as atomics the audio thread writes and the main thread reads:
`underruns`, `rendered_frames`, `average_fill` (the one-second average the rate controller also
uses), `playing`, and the extremes of the ring's fill.

The extremes are **double-buffered**: the callback keeps a live minimum and maximum with
`fetch_min`/`fetch_max`, and `close_window` — which the host calls once a second, timed by
`RateControl::window_closed` — moves them to the pair the readers see and starts another window.
Without that, the overlay (drawn every frame) would read the window still filling and show a
minimum and a maximum a few milliseconds old and all but equal.

`EmulationHost::audio_report()` is the single line both readers share:

```
latency 59.4 / 60 ms  [56-63 ms]  rate +0.028 %  underruns 0
```

— the measured latency against the configured one, the extremes of the last complete second,
the correction the controller is asking of the core as a percentage, the count of droughts since
the device opened, and `prefilling` while the gate is shut.

* **The performance overlay** (F12, `ui/debug_overlay.rs`) shows it under the mixer's line,
  which keeps the sample rate, the console gain, the music gain and the track.
* **The log**, once a second, while the frame profiler is on (`ZAXNES_PROFILE=1` or
  `ui.profiler`), from `Application::log_audio_counters`. The profiler's own report stays on its
  two-second window; the audio line has its own timer so it lands every second as the work order
  asks.

How to read them: `underruns` should stay at **0**. The measured latency should sit within two or
three milliseconds of the target and the extremes should straddle it; a minimum falling towards
0 means the main thread is being starved and the latency wants raising. `rate` should settle at
this machine's clock offset — a few hundredths of a percent — and anything pinned at ±0,250 % or
beyond means the controller is saturated and the latency is drifting faster than it can hold.

`audio_rate_adjustment` and a separate window timer on the host both turned out to belong in
`RateControl` (it is the one thing the host already steps with the tick's real time), which also
kept `EmulationHost::new` under the hundred-line clippy limit.

### Step 8 — the tests

Every test the work order asks for, none of them needing a device: the stream is driven by
calling `render_block` directly, with a `ConsoleSamples` the test fills by hand.

| test | what it pins down |
|---|---|
| `req_aud_003_a_late_console_is_held_and_faded_rather_than_zeroed` | a late producer yields **no zero block**: the samples it had are played, the last is held and ramped, no two neighbouring samples jump by more than the ramp allows, one under-run is counted |
| `q153_the_prefill_gates_playback` | one sample short of the target the output is exact silence and the ring is untouched; the sample that completes it opens the gate and the signal is ramped in |
| `q153_a_resume_clears_the_ring_and_ramps_back_in` | a resynchronisation plays nothing stale, waits for half the latency and no more, ramps in, counts no under-run |
| `q153_a_forty_millisecond_stall_at_sixty_of_latency_does_not_under_run` | finding 3: a 40 ms stall of the main thread at 60 ms of latency gives **0** under-runs; a 120 ms stall is noticed; and — the control that the harness measures what it claims — 40 ms at 30 ms of latency, which is what v0.4.2's four queued blocks amounted to, does under-run |
| `req_aud_005_a_drifting_device_clock_neither_under_runs_nor_drifts_away` | the closed loop: the real stream, the real `RateControl` and a device clock ±0,03 % off, ten minutes each way → **0** under-runs, latency settled within the 3 ms dead band |
| `req_aud_005_an_hour_against_a_drifting_clock_under_runs_nothing` | the same over a **full hour** (about 200 000 blocks). `#[ignore]`d: 0,7 s in release, most of a minute in debug |
| `rate_control`'s eight unit tests | the law itself: dead band, capped and opposing proportional, the accumulation absorbing an offset inside the dead band, one resynchronisation per excursion, no correction while prefilling, idling keeping what was learnt, and the open-loop simulation over an hour with the measurement lag included |

The closed-loop test is the one that would have caught finding 2: v0.4.2's proportional-only
controller needed a standing error to correct anything, and that standing error was paid in
blocks of silence.

Release run of `zax-app`, `zax-platform`, `zax-settings` and `zax-nes`: all green, with
`--include-ignored` on `audio_tests` too (19 tests, 0,74 s).

### Step 9 — the measurements

**Golden-core: byte-identical.** `v0.4.2` was extracted with `git archive` into a throwaway tree
(inside `target-au`, since nothing there is ever staged, and deleted afterwards), its harness
built, and both run over the 98 ROMs of `G:/3DSen/test-originals` at 1200 frames each:

```
6ae5cd1dd246d67dc330e9a690aac15f1c5356fc55ff1ed5a048e51662906daa  v0.4.2
6ae5cd1dd246d67dc330e9a690aac15f1c5356fc55ff1ed5a048e51662906daa  audio-fix
cmp: BYTE-IDENTICAL   (98 ROMs, 145 334 bytes)
```

Which is what the diff already said: `git diff v0.4.2 -- rust/crates/zax-nes rust/crates/zax-harness
rust/crates/zax-goldens` is empty. Of the harness's dependencies only `zax-settings` changed, by
one descriptor, and `golden_core.rs` does not mention settings at all.

**Frame time.** Release build, Castlevania (USA), 1280×720, vsync on, `ZAXNES_PROFILE=1`,
`--smoke-emulated 1200` (20 s of play), RTX 3050, 24 CPUs; the last three two-second windows of
each run. The figures are mean/max milliseconds per displayed frame.

| | audio stage | whole frame |
|---|---|---|
| v0.4.2, 2D | 0,01-0,02 / 0,01-0,17 | 16,66 / 17,31-17,45 |
| audio-fix, 2D | **0,00 / 0,00-0,01** | 16,66 / 16,99-18,29 |
| v0.4.2, 3D (default profile) | 0,01 / 0,01-0,03 | 16,66-16,67 / 17,79-28,00 |
| audio-fix, 3D | **0,00 / 0,00** | 16,66-16,67 / 17,46-17,91 |

The audio stage got **cheaper**, which is what the design predicts: the main thread no longer
mixes the console signal, no longer converts a block to 16-bit PCM and no longer calls into SDL.
All it does now is copy the menu music and the interface sounds into a ring, and in the menu —
where the console is silent — that is all there ever was. The whole frame is pinned at 16,66 ms
by vsync in every run; the maxima differ by less than the run-to-run noise (the 28,00 ms in one
v0.4.2 3D window is a first-window shader-and-mesh hitch, not a steady cost).

**So it is on by default.** No option, nothing behind a switch: there is no cost to hide.

**The counters, from the same runs** (and this is what the design is for):

```
2D:  audio  latency 57.0 / 60 ms  [46-68 ms]  rate -0.005 %  underruns 0
3D:  audio  latency 60.2 / 60 ms  [49-71 ms]  rate +0.021 %  underruns 0
```

The latency holds on its target, the extremes straddle it by about ±11 ms (one emulated frame of
production either side, which is the shape of the sawtooth and not a drift), the correction
settles at this machine's real clock offset of two hundredths of a percent, and **nothing
under-ran** over twenty seconds of play in either mode.

**Two defects this run found and fixed** — neither visible to a unit test, which is why the work
order asked for the measurement:

1. *The latency measurement was a block low.* `record_fill` recorded what was **left** after the
   callback took its block, so a nominal 60 ms read as 31 ms and the controller held the ring a
   whole block fuller than it had been told to. It now records what was waiting when the device
   came for its block, which is the latency in the plain sense and the number
   `audio.latency_ms` promises.
2. *The first start of a game got half a prefill.* `set_state(Running)` and `finish_launch` each
   asked for a resynchronisation, and the stream's "have I ever played" flag could flip between
   the two, so the second call downgraded the first start to the mid-stream half-prefill — the
   game began with half its margin and spent ten seconds climbing to the configured latency. The
   rule is now race-free and better: half the target **only while something is actually being
   heard** (a rollback, a slot loaded mid-game, a drought), the whole target whenever the output
   is already silent (a game starting, a resume from a pause), where the wait costs nothing
   audible.

**Android and PortMaster: not checkable on this machine, and here is why that is safe to say.**
`cargo check -p zax-android --target aarch64-linux-android` fails in `sdl2-sys`'s build script
for want of `ANDROID_NDK_HOME` (no NDK installed), and `aarch64-unknown-linux-gnu` is not an
installed target and has no cross linker here. What was checked instead:

* `cargo check -p zax-app --no-default-features --features android` — clean;
* `cargo check -p zax-app --no-default-features --features firmware-sdl` — clean.

Those are the two feature sets those builds use; the features only decide **how SDL2 is linked**
(bundled static, bundled shared, firmware shared), never which API is called. And the new code
contains no `cfg` of any kind outside `cfg(test)`: the callback device is plain `sdl2::audio`,
and `SDL_OpenAudioDevice` with a callback is the older and more universally supported of the two
SDL2 audio paths — `SDL_QueueAudio`, which v0.4.2 used, is the newer one.

**SMB3, Zelda II and the widescreen** are untouched: nothing outside the audio path changed, the
core and the 3D engine are byte-identical to `v0.4.2`, and their tests are in the green runs
above.

The four profiler logs are in the session scratchpad (`bench/before-2d.log`, `before-3d.log`,
`after-2d.log`, `after-3d.log`); the extracted `v0.4.2` tree was deleted afterwards (G: back to
18 GB free).

### Step 10 — the final pass

* `cargo fmt --check` — clean.
* `cargo clippy -p zax-platform -p zax-settings -p zax-app --all-targets --features
  zax-app/bundled -- -D warnings` — clean. No `#[allow]` was added to silence anything: the one
  that exists (`dead_code` on the `Running::audio` field) says that the field is the device's
  RAII handle, and the two `too_many_lines` warnings the new host fields caused were fixed by
  removing duplicated state rather than by an allow.
* `cargo test --release --no-fail-fast -p zax-platform -p zax-settings -p zax-app --features
  zax-app/bundled -- --include-ignored`, with `ZAXNES_ROMS=G:/3dsen/demo/roms` and
  `ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`:
  **810 passed, 0 failed, 0 ignored** across 51 test binaries. Nothing was left vacuous: the
  ignored count came out at zero, so every ROM-gated test really ran.
* **golden-core: byte-identical to `v0.4.2`**, re-run on the final tree
  (`6ae5cd1dd246d67dc330e9a690aac15f1c5356fc55ff1ed5a048e51662906daa`, 98 ROMs). `git diff v0.4.2`
  over `zax-nes`, `zax-harness`, `zax-goldens`, `zax-engine3d`, `zax-render`, `zax-profiles` and
  `zax-scripting` is empty: the core, the 3D engine and the renderer were not touched at all.
* **Frame time: the change costs nothing and is on by default.** See step 9 for the table; the
  audio stage went from 0,01-0,02 ms to 0,00 ms of mean per displayed frame, in 2D and in 3D.
  There is nothing to put behind an option.

## What the owner should listen for

1. **The pops themselves.** Play the games where they were worst, and especially with
   `video.half_rate_3d` on — the 33,3 ms frames of that mode were at or past the whole output
   buffer of v0.4.2, so it should be the clearest difference.
2. **Pausing and resuming.** The sound should now *stop* rather than be cut: a 6 ms ramp instead
   of the ring running dry mid-note. On resume it fades in over the same 6 ms after a 60 ms wait.
3. **Rewind (`,`) and quick load.** A short dip, about 30 ms, with a ramp at each end; no click
   and no fragment of the moment before.
4. **A loud game with expansion audio** (VRC6, N163, VRC7). Nothing changed there deliberately —
   see the note on headroom below — so it should sound exactly as before.
5. **The menu music and the interface sounds.** Unchanged in level and in behaviour; the only
   difference is that they are produced 48 ms ahead instead of up to 43 ms, which is within the
   same order.
6. **`audio.latency_ms`** in Options → Audio. 60 ms is the default. Lower it until the pops come
   back to find this machine's floor; raise it if a heavy 3D scene still stutters. Above about
   138 ms at 48 kHz it is clamped by the core's ring and the log says so (Q-153 §3).

## How to read the new counters

Press **F12** for the performance overlay; the audio line sits under the mixer's:

```
latency 57.0 / 60 ms  [46-68 ms]  rate -0.005 %  underruns 0
```

| field | meaning | what is healthy |
|---|---|---|
| `latency a / b` | the fill the audio thread measured, averaged over a second, against `audio.latency_ms` | `a` within two or three milliseconds of `b` |
| `[low-high]` | the least and the most the ring held over the **last complete second** | straddling the target by about one emulated frame either side (±11 ms at 60 Hz); a low falling towards 0 means the main thread is being starved and the latency wants raising |
| `rate ±x %` | the correction being asked of the core's resampler | settles at this machine's clock offset, a few hundredths of a percent; pinned at ±0,250 % or beyond means the controller is saturated |
| `underruns n` | droughts of the console signal since the device opened | **0**. Any increase is a pop, and now a ramped one rather than a block of zeros |
| `prefilling` | appears while the gate is shut | only at a game start, a resume, a state load or a rewind, and only for a few tens of milliseconds |

With the frame profiler on (`ZAXNES_PROFILE=1`, or Options → Interface → profiler) the same line
goes to the log once a second, prefixed `audio`.

## Headroom: left alone, and why

Finding 5 is real — the APU's `OUTPUT_GAIN` of 1,526 plus expansion audio plus the high-pass
overshoot can reach the ±1 clamp — but the work order allows the fix only "if it changes no
golden", and neither candidate passes that test against the **spec** rather than the goldens:

* a fixed −3 dB breaks REQ-AUD-006 and test vector A1, which say the gain at `V = 1` is exactly
  1 and a peak of *x* comes out at *x*;
* a soft limiter with a knee below full scale alters samples in, say, 0,8 to 1,0 that were never
  clipped, which is an audible change to every loud game and not a bug fix.

So the hard clamp of SPEC-11 §4 stays. What *was* fixed is the other half of finding 5: the gain
is now applied **per sample** by the audio thread instead of once per 512-frame block, so a fade
is no longer a staircase of 10,7 ms steps, and the final clamp happens once on the sum of the
console and the programme instead of twice.

If the owner does hear clipping on a specific expansion-audio game, the right lever is a
limiter **behind an option that is off by default** — which this lot deliberately did not add,
because no such clipping was reported.
