# AUDIO-2 — the AUDIO-1 audio output on `dev`, with the old one as a fallback

Branch `dev`, from `1c2b4639` (version 0.5.0). Worktree `wt-rust`.

Lot AUDIO-1 (branch `audio-fix`, tip `35512550`, built from `v0.4.2`) replaced the audio
**output** — how the APU's samples are buffered and sent to the device — with an SDL callback,
a configurable latency (`audio.latency_ms`, default 60), a prefill, rate control on what is
really buffered, ramps instead of silence blocks, and counters. `dev` has moved on since
`v0.4.2` (the whole mapper-review work) and does not contain it.

This lot brings it onto `dev` and puts `v0.4.2`'s output back beside it, behind a setting that
defaults to the new one.

## Plan — 6 steps

- [ ] **1. Plan.** This file.
- [ ] **2. Merge `audio-fix` into `dev`.** `git merge --no-ff audio-fix`, the one merge this lot
      makes. Resolve the two conflicts keeping both sides' intent; the merge commit builds and
      passes the `zax-app`, `zax-platform` and `zax-settings` tests, and golden-core over the 98
      originals stays byte-identical to `1c2b4639`'s.
- [ ] **3. The fallback option.** `v0.4.2`'s output — the SDL queue of four 512-frame blocks
      refilled once per rendered frame, its own rate control and its silence handling — back
      beside the new one, chosen by a setting in the `audio.` family that defaults to the new
      output, in the desktop options' Audio tab and in the handheld UI, in English and in
      French, taking effect when the device is (re)opened and reopening it when it changes.
      Nothing per sample, nothing in `zax-nes`, at most one branch per rendered frame.
- [ ] **4. Tests.** The setting's default and its round-trip, both outputs opening and pumping
      without a device, and the classic output reproducing `v0.4.2`'s block behaviour.
- [ ] **5. Measurements.** Main-thread frame time at `1c2b4639` and here, 2D and 3D, desktop
      release build, with each output; golden-core against `1c2b4639`.
- [ ] **6. Final pass.** fmt, clippy `-D warnings`, `cargo test --release`, the ignored tests
      with the ROM variables, journal entry.

The protocol's "progress file in the same commit" cannot hold for step 2: that step's commit
message is fixed by the work order (`Merge audio-fix: the AUDIO-1 audio output`). The step is
therefore recorded here in the commit that follows the merge, which carries no code of its own.

## Notes per step
