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
- [ ] **5. Rate control on what is really buffered.** The ring fill is measured as a latency in
      milliseconds by the audio callback and averaged over about a second; the correction is a
      dead band, a capped proportional term and a slowly accumulating term. Only at 100 % speed
      and not while recording. A far drift resynchronises.
- [ ] **6. Pause, resume, state load, rewind.** A short ramp to zero when the console stops
      producing; on resume a clear, a prefill and a ramp in. The same on save-state load and on
      rewind. `AudioOutput::flush` is actually called.
- [ ] **7. Counters.** Underrun events, average latency, current rate adjustment, minimum and
      maximum ring fill, in the performance overlay and logged once per second when the frame
      profiler is on.
- [ ] **8. Tests.** Ring/callback logic without a device; rate control against a simulated
      device clock; main-thread stalls.
- [ ] **9. Measurements.** Main-thread frame time before and after, 2D and 3D, desktop release
      build; golden-core byte-identical to `v0.4.2`; Android and PortMaster `cargo check`.
- [ ] **10. Final pass.** fmt, clippy `-D warnings`, `cargo test --release`, the ignored tests
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
