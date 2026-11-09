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

- [x] **1. Plan.** This file.
- [x] **2. Merge `audio-fix` into `dev`.** `git merge --no-ff audio-fix`, the one merge this lot
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

### Step 2 — the merge (`6c8ee59c`)

`git merge --no-ff audio-fix`, nineteen files auto-merged, **two conflicts** and **two semantic
conflicts** that no marker showed.

#### The two textual conflicts

**`rust/crates/zax-app/tests/asset_free_startup_tests.rs`** — both sides had independently written
the *same* fix, and the merge found the two copies on top of each other. `audio-fix` added
`is_build_output` because `Path::starts_with` compares whole components, so the exclusion of
`rust/target` missed the `rust/target-au` that lot built into and the `.obj` files of a bundled
SDL2 then read as assets of the repository (AUDIO-1 step 3's "one unrelated test fix"). `dev` added
a function of the same name and the same body for the same reason, naming `rust/target-android` and
`rust/target-ro` instead. **Kept**: `dev`'s body (identical in effect, and it is the one the rest of
`dev` was tested against) with a doc comment carrying both sides' reasons — the whole-component
rule from `audio-fix`, the list of directories from `dev` — and `dev`'s comment at the call site.
Nothing of either intent is lost.

**`rust/crates/zax-app/src/application.rs`** — two hunks, both in `struct Running` and its
initialiser, and both because the two sides added a field next to the same line.

| | `dev` had | `audio-fix` had | kept |
| --- | --- | --- | --- |
| the field | `pending_state_load: Option<String>` (AMD-053 `-slot=<n>`, `"1"`–`"36"` or `"quick"`) | `pending_state_load: Option<i32>` (`v0.4.2`'s `--load-state`) **plus** `audio_logged_at: Instant` | `dev`'s `Option<String>` **and** `audio-fix`'s `audio_logged_at` |
| the initialiser | `pending_state_load: options.slot.clone()` | `pending_state_load: options.load_state`, `audio_logged_at: Instant::now()` | `options.slot.clone()` **and** `Instant::now()` |

`audio-fix` only touched `pending_state_load` by proximity — it never changed that field — so
taking `dev`'s type and `audio-fix`'s new field keeps both intents whole. Everything else of
`application.rs` auto-merged, including the two places the lots both rewrote without overlapping:
`HostParts` now carries `dev`'s `suspend_resume` and `emu_stats` beside `audio-fix`'s
`audio: Option<AudioLink>`, and the frame keeps `dev`'s `wait_for_the_gpu_queue` /
`close_the_gpu_queue` around the drawing with `audio-fix`'s four-argument `pump_audio` after it.

#### The two semantic conflicts

Neither produced a marker; both are `dev` asserting something about the options window that
`audio.latency_ms` — a setting born on `v0.4.2`, where that assertion did not exist — had no way to
satisfy. They are the whole reason the work order asks the merge commit to pass the tests.

1. **`audio.latency_ms` had no row in the layout table.** Lot RO-1 replaced the window's implicit
   ordering with an explicit `LAYOUT` in `ui/options.rs`, and
   `desktop_options_golden_tests::ro_1_the_layout_table_places_every_shown_setting_exactly_once`
   and `options_window_tests::req_ui_202_every_user_preference_is_shown` both failed with
   `["audio.latency_ms"]`. The Audio tab was a one-row literal; it is now a named `AUDIO` table of
   the volume and the latency, which is exactly where AUDIO-1 step 4 put the slider.
2. **The golden list and the tooltips.** `rust/goldens/desktop-options-shown.txt` is RO-1's record
   of the set of settings the window shows, and its own header says no key may join it without
   being added in the same commit — `audio.latency_ms` is added. `ro_1_every_shown_setting_has_a_
   detailed_tooltip_in_both_languages` then asks every shown row for a `detail.<key>` paragraph of
   more than eighty characters in English *and* French; AUDIO-1 had only written the label, RO-1
   being later than `v0.4.2`. Both paragraphs are written: what the latency buys and costs, the
   clamp at the core's ring (Q-153 §3) and the dip a change makes.

#### The checks the work order asks of the merge commit

| check | result |
| --- | --- |
| `cargo build -p zax-app --features zax-app/bundled` | clean |
| `cargo test -p zax-app -p zax-platform -p zax-settings --features zax-app/bundled` | **1 055 passed, 0 failed**, 2 ignored |
| `cargo fmt --check` | clean |
| golden-core over the 98 originals of `G:/3DSen/test-originals`, 1 200 frames | `f79e7df842c3ee43e7d9b67861a927b5`, **`1c2b4639`'s own digest** |

`git diff 1c2b4639 -- rust/crates/zax-nes rust/crates/zax-harness rust/crates/zax-goldens` is
empty, which is why the digest could not have moved; the run is the proof rather than the argument.
`1c2b4639` is "Version 0.5.0", four files of version bumps over `75b35900`, whose own note records
that same digest.

`rust/target-au2/` joined `.gitignore` beside the other session build directories, so that the
worktree reads clean while this lot builds.
