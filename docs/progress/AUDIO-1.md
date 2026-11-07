# AUDIO-1 — no more crackles at the audio output

Branch `audio-fix`, from `v0.4.2`. Worktree `wt-audio`.

The signal processing of the audio path is sound (band-limited step resampler, 20 Hz
high-pass, PAL/NTSC sample counts, speed by time compression, run-ahead muting). The defects
are in **how the output device is fed**. Everything this lot changes sits *after* the APU's
sample ring: the emulation core, the resampler and the filters are untouched.

## Plan — 10 steps

- [ ] **1. Plan.** This file.
- [ ] **2. Verify the findings.** Read the audio path and confirm or refute each of the five
      findings with `file:line`. Write the question ticket for the spec clauses the owner's
      design contradicts (REQ-AUD-003 silence blocks, the latency note, the ring capacity).
- [ ] **3. Pull output.** The SDL device is opened with a callback that reads the APU sample
      ring directly on SDL's audio thread. Menu music and interface sounds are *precomputed*
      by the mixer on the main thread into an auxiliary stereo ring, so the callback never
      touches a file, an allocation or the mixer's state machine. No silence is ever inserted
      while data is merely late: on a real underrun the last sample is held and faded to zero
      over a few milliseconds, the event is counted, and the signal is faded back in when data
      returns.
- [ ] **4. Latency setting and prefill.** `audio.latency_ms` (default 60, range 20–200) in the
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

(filled in as the steps are done)
