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
