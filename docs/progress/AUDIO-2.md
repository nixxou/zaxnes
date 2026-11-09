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
- [x] **3. The fallback option.** `v0.4.2`'s output — the SDL queue of four 512-frame blocks
      refilled once per rendered frame, its own rate control and its silence handling — back
      beside the new one, chosen by a setting in the `audio.` family that defaults to the new
      output, in the desktop options' Audio tab and in the handheld UI, in English and in
      French, taking effect when the device is (re)opened and reopening it when it changes.
      Nothing per sample, nothing in `zax-nes`, at most one branch per rendered frame.
- [x] **4. Tests.** The setting's default and its round-trip, both outputs opening and pumping
      without a device, and the classic output reproducing `v0.4.2`'s block behaviour.
- [x] **5. Measurements.** Main-thread frame time at `1c2b4639` and here, 2D and 3D, desktop
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

### Step 3 — the fallback option

`audio.classic_output`, a boolean, **default off**: the program opens the AUDIO-1 output unless
the user ticks the box. Label *"Classic audio output (v0.4.2)"* / *« Sortie audio classique
(v0.4.2) »*.

#### What was brought back, and from where

`v0.4.2`'s output is three pieces, restored one by one. None of them is a new design: each is that
version's own code, read with `git show v0.4.2:<path>` and put back beside AUDIO-1's rather than in
place of it.

| piece | where it is now | what it is |
| --- | --- | --- |
| the device | `zax-platform::audio::ClassicAudioOutput` | the `AudioQueue`, `BLOCK_COUNT = 4`, `blocks_to_queue`, `pump`, `flush` — `v0.4.2`'s `AudioOutput`, renamed so that AUDIO-1's keeps its name |
| the mix | `AudioMixer::fill_with_console` and `mix_emulation` | the console read all-or-nothing, one gain per block, then the music and the sounds, then the clamp |
| the rate control | `EmulationHost::correct_audio_drift_classic` | proportional only, `-error × 0.002` capped, on `available_samples()` against two blocks |

`AudioMixer::fill` — AUDIO-1's, the programme alone and no clamp — is untouched: the common part of
the two moved into a private `add_program`, and the two public entries call it. `fill_with_console`
therefore sums in `v0.4.2`'s order (console, music, sounds), so the fallback's block is
**bit-identical** to that version's and not merely equivalent.

The defects Q-153 records are kept with it, deliberately: the block of zeros whenever the core's
ring holds less than 512 samples (§1), the controller that settles below its target for want of an
integral term (§2), the 43 ms buffer (§3) and the hard cut at a pause (§4). A fallback that behaved
better than `v0.4.2` would not be a fallback — it would be a third output nobody has measured.

#### What it costs when it is off

The requirement is "nothing on the emulation path: the choice is made when the device opens, at
most one branch per rendered frame on the main thread, nothing per sample, nothing in `zax-nes`".

* **Nothing in `zax-nes`.** `git diff 1c2b4639 -- rust/crates/zax-nes` is empty.
* **The choice is made once.** `Running::new` reads `audio.classic_output` and `open_audio` opens
  one of the two devices. Nothing reads the setting again except `on_setting_changed`, which runs
  only when it is written.
* **Two branches per tick, both on an already-decided field.** `pump_audio` matches `AudioPath`,
  and `correct_audio_drift` tests `self.classic_audio` — one `bool` field of the host, never a
  settings lookup. A third, `take_audio_reopen`, is one field read and one branch per rendered
  frame, which is how the reopen is noticed without polling the setting every frame.
* **Nothing per sample.** With the default output the mixer's `emulation_source` is `None` and
  `mix_emulation` is never called at all: `fill_with_console` has no caller, `mono` stays an empty
  `Vec` and no allocation is ever made for it. `ClassicAudioOutput` is not constructed.
* **No new `unsafe`,** and none was needed: `AudioQueue` and `AudioCallback` are both safe Rust in
  the `sdl2` crate. AUDIO-1's note is right that the exception the work order allows goes unused,
  and this lot does not use it either.

#### The reopen

`SettingEffect::Immediate`, so a write reaches `on_setting_changed`, which sets a one-shot
`audio_reopen` when the setting disagrees with the output that is open. `Application::frame` drains
it with `reopen_the_audio_output`, beside `follow_the_profiler_setting`: the old device is dropped
first (which stops SDL's callbacks, and on the default output drops the stream with them), the host
is told the device has gone, the new one is opened, and `EmulationHost::attach_audio` re-plumbs it —
the running game's samples onto the new path, `audio.latency_ms` applied, the rate controller
restarted, a resynchronisation asked for. Switching while a game plays is therefore a dip of a few
tens of milliseconds, not a silence.

`attach_console` is the one place that knows where a game's samples go: the audio thread's console
slot on the default output, the mixer's `AudioSampleSource` on the classic one. `jobs.rs` calls it
at a launch instead of installing the slot itself.

One call `v0.4.2` had and AUDIO-1 removed is back in `finish_launch`: `drop_pending_samples()`
before `set_state(Running)`. The classic output has nothing else that would empty the ring — no
gate, no ramp, and `resynchronise_audio` returns early without an `AudioLink` — so REQ-AUD-010
needs it there. On the default output it is simply the first of two idempotent calls, so that path
is unchanged.

#### Where the setting appears

* **The desktop options window**, Audio tab, under an *Advanced* heading with the latency: volume,
  then `audio.latency_ms`, then `audio.classic_output`. The latency row is **greyed** while the
  fallback is on (`options::inert_reason`, hint `hint.classic_audio` — "(no effect: the classic
  audio output has its own fixed buffer)"), which is the work order's second option and the better
  one: that output never sees the latency, and `stream.rs`, where the prefill, the ramps and the
  counters live, is not open at all.
* **The handheld UI**, *Options › Advanced options › Audio*, a new page of the RO-3 tree with this
  one row. Not the main page: that page is the owner's mockup and
  `ro_3_the_handheld_main_pages_are_the_mockups` pins it row for row, so a row added there would be
  a layout decision this lot has no mandate for. The page is on the **global** hub only — the
  choice of output belongs to the device, not to a game. `audio.latency_ms` stays off the console
  screen, as AUDIO-1 left it (owner request).
* **The hint.** `help.audio.classic_output` is the one line the work order asks for, in both
  languages, shown at the head of the tooltip: "Use if the new audio output misbehaves on this
  device; it has the old, shorter buffer and can crackle." The handheld screen has no per-row help
  line of any kind — its "hints" are the button bar at the bottom — so there is nowhere to put a
  second copy there; the label is what it shows.
* **The detailed tooltip** `detail.audio.classic_output`, in both languages, as RO-1 requires of
  every shown row: what the buffer is, why it can crackle, that the latency setting and the
  counters do not apply, and that the choice takes effect when the device is reopened. The
  latency's own paragraph gained the same note and a mention of the greying.
* **The performance overlay** (F12) and the profiler's per-second line now report
  `classic output (v0.4.2): 4 blocks of 512 frames, silence blocks N` instead of AUDIO-1's latency
  line, which has no meaning there. `N` is the mixer's `underrun_blocks` — the blocks of zeros of
  Q-153 §1, counted so that the owner can see the defect rather than only hear it.

#### Registrations the new key needed

`SettingId::AudioClassicOutput` / `"audio.classic_output"`, `SettingKind::Bool`, default `false`,
scope User, effect Immediate, theme Audio, `Domain::switch()`. No libretro core option and nothing
in Lua: the output is a property of this frontend's device. Beyond the table it had to join
`zax-settings/tests/catalogue.rs`'s list of Rust-only additions, `ui/options.rs`'s `AUDIO` layout,
`rust/goldens/desktop-options-shown.txt`, `handheld::advanced::ROW_SETTINGS` (56 → 57) and
`rust/goldens/handheld-option-tree.txt` (regenerated by the lot's own ignored test — two lines).

#### Checks

| check | result |
| --- | --- |
| `cargo test -p zax-app -p zax-platform -p zax-settings --features zax-app/bundled` | **1 056 passed, 0 failed**, 2 ignored |
| `cargo fmt --check` | clean |
| `cargo clippy -p zax-platform -p zax-settings -p zax-app --all-targets --features zax-app/bundled -- -D warnings` | clean, and no new `#[allow]` |
| `cargo check -p zax-app --no-default-features --features android` | clean |
| `cargo check -p zax-app --no-default-features --features firmware-sdl` | clean |

`cargo check -p zax-android --target aarch64-linux-android` still cannot run on this machine:
`cc-rs` fails to find `clang.exe` (no NDK), exactly as AUDIO-1 reported. The two feature sets above
are what those builds compile with, and the surrogate is sound for the same reason it was then: the
new code contains no `cfg` outside `cfg(test)`, and `SDL_QueueAudio` — what the classic output uses
— is what `v0.4.2` shipped on both of those builds.

### Step 4 — the tests

Eight new tests, none of them needing a sound device. Seven are in
`zax-app/tests/audio_tests.rs` beside AUDIO-1's, which is where the default output is already
driven without one; the round trip is in `zax-settings/tests/settings_file.rs`, with the other
settings-file tests.

| test | what the work order asked for, and what it pins |
| --- | --- |
| `audio_2_the_new_output_is_the_default` | **the default.** `audio.classic_output` reads `false` on a fresh model, and a host built without the flag reports `classic_audio() == false` |
| `audio_2_the_classic_output_defaults_off_and_round_trips` (`zax-settings`) | **the round trip.** Off by default; written, found in the file's `values` as `"audio.classic_output": true`, read back by a fresh model; `audio.latency_ms` beside it; then back to `false` and round-tripped again, so the test is not a one-way write of `true` |
| `audio_2_the_classic_output_keeps_four_blocks_and_refills_once_per_frame` | **`v0.4.2`'s block behaviour.** `BLOCK_COUNT == 4`; the `div_ceil` that makes a partly played block count as a whole one (which is why the real floor of the buffer is nearer 32 ms than 43 — finding 3); an over-drained queue asked for no more than four; and ten frames of the pump's own loop over a device playing one block a frame give `[4, 1, 1, 1, 1, 1, 1, 1, 1, 1]`, with four pending at the end of every frame |
| `audio_2_the_classic_block_is_the_console_on_both_channels_clamped_once` | the block itself: the console on both channels at the gain of the moment (REQ-AUD-002), and a signal beyond full scale clamped once on the sum (SPEC-11 §4) |
| `audio_2_a_late_console_costs_the_classic_output_a_whole_block_of_zeros` | the defect the fallback is kept **with** (Q-153 §1): one sample short of a block, the whole block is exact zeros, nothing is consumed, one silence block is counted; the sample that completes it switches the signal on at full gain with no ramp; and outside Running a dry ring counts nothing (REQ-AUD-010). Its control is AUDIO-1's `req_aud_003_a_late_console_is_held_and_faded_rather_than_zeroed`, which shows the default output holding and ramping in exactly that situation |
| `audio_2_both_outputs_pump_and_correct_without_a_device` | **both outputs without a device.** The classic path, 30 frames of a real game: the ring fills past two blocks, `v0.4.2`'s proportional law saturates at `-MAX_RATIO_ADJUSTMENT` and the core takes it, and `audio_report()` is the classic line. The default path with no device: no correction at all (`audio_ratio_adjustment() == 0`) and no report |
| `audio_2_changing_the_setting_asks_for_one_reopening` | the reopening is asked for **once** per real change, drained by the loop, and not asked for at all when the value written agrees with the output that is open |
| `audio_2_a_new_game_drops_the_pending_audio_on_the_classic_output_too` | REQ-AUD-010 on the classic path, which has no gate and no `resynchronise_audio`: the launch empties the ring, as `v0.4.2` did, so no fragment of the previous session is the first thing heard |

**On the one thing a test cannot reach.** `ClassicAudioOutput::pump` wraps `blocks_to_queue` in six
lines around `AudioQueue::queue_audio`, and an `AudioQueue` cannot be made without a sound device.
The test therefore drives the rule and a replica of those six lines, and says so in its own
documentation. What is left untested is the SDL call itself — which is `v0.4.2`'s, and was shipped.
`zax-platform`'s own `the_classic_queue_is_kept_at_four_blocks` is that version's unit test of
`blocks_to_queue`, restored with the function.

`cargo test -p zax-app -p zax-platform -p zax-settings --features zax-app/bundled`: **1 064
passed, 0 failed**, 2 ignored. fmt and clippy `-D warnings` clean.

A ninth test joined them in step 5, for the defect that step found — see below.

### Step 5 — the measurements

#### The defect the measurement found, which no test would have

The first bench run came back with `audio latency 0.0 / 60 ms [0-0 ms] rate +0.000 % underruns 0
prefilling` for the whole twenty seconds. The device had opened — the log says
`audio: 48000 Hz, blocks of 512 frames, target latency 60 ms` — but the prefill gate never opened,
which means the callback never saw a single sample.

The cause was step 3's own: `attach_console` filtered the session with `has_rom()`, and at the one
moment it is called from a launch — `launch_session`, just after `GameSession::create` — the core
has not been reset yet, so `has_rom()` is still **false**. `v0.4.2` and AUDIO-1 both installed the
console there with no such test. **The default output would have been silent in every game**, and
the whole 1 064-test suite passed: the stream's own tests install their console by hand, and the
host tests of this lot ran with no link at all.

Fixed by dropping the filter — any session has a core, and the reader is a clone of the APU's ring
handle, which survives the reset — and pinned by
`audio_2_a_launch_hands_the_game_s_samples_to_the_default_output`: with a detached link, a launch
fills the console slot, the slot sees more than a block of samples, a read gets them, and the host
has published the gain and the producing flag. The run after the fix reads
`latency 65.5 / 60 ms [55-78 ms] rate -0.129 % underruns 0`.

This is the second lot running in which the measurement caught something the unit tests could not;
AUDIO-1's step 9 found two.

#### Frame time

Release build, `Castlevania [USA][!!]`, 1280×720, vsync on, `ZAXNES_PROFILE=1`,
`--smoke-emulated 1200` (20 s of play), RTX 3050, 24 CPUs; the last three two-second windows of
each run. Mean/max milliseconds per **displayed** frame; the `audio` column is the loop's audio
stage, which is the whole of what either output costs the main thread.

`1c2b4639` was extracted with `git archive` into a throwaway directory inside `target-au2` (nothing
there is ever staged), built there with its own `CARGO_TARGET_DIR`, and deleted as soon as the
figures were taken. Its two demo files had to be copied in by hand: `FlappyZaxNes.nes` and
`flappyzaxnes.3dn` are in the worktree but not in the commit, so `git archive` leaves them out and
`zax-app` does not compile without them.

| build | audio stage | whole frame |
| --- | --- | --- |
| `1c2b4639`, 2D | 0,01 / 0,01-0,03 | 16,66-16,67 / 16,86-16,90 |
| this tree, **new** output, 2D | **0,00 / 0,00** | 16,66-16,67 / 16,86-17,32 |
| this tree, **classic** output, 2D | 0,01 / 0,01-0,03 | 16,66-16,67 / 16,84-17,39 |
| `1c2b4639`, 3D (default profile) | 0,01 / 0,01-0,08 | 16,66-16,67 / 17,81-18,90 |
| this tree, **new** output, 3D | **0,00 / 0,00-0,01** | 16,66-16,67 / 17,91-18,02 |
| this tree, **classic** output, 3D | 0,01 / 0,02-0,04 | 16,66-16,67 / 17,96-18,71 |

**Neither output costs more than noise against `1c2b4639`.**

* The **new** output is *cheaper*, in 2D and in 3D, for the reason AUDIO-1's own measurement gave:
  the main thread no longer mixes the console, converts a block to 16-bit PCM or calls into SDL.
  The stage reads 0,00 ms.
* The **classic** output reads 0,01 ms — `1c2b4639`'s figure to the hundredth, in both modes,
  which is what it should read, since it *is* `1c2b4639`'s output. That it did not come out
  cheaper or dearer is the evidence that the restoration is faithful.
* The **whole frame** is pinned at 16,66 ms by vsync in all six runs, and the maxima differ by
  less than the run-to-run noise (0,5 ms spreads inside a single configuration's own three
  windows).

#### The counters, from the same runs

```
new output,     2D:  audio  latency 57.2 / 60 ms  [46-69 ms]  rate +0.023 %  underruns 0
new output,     3D:  audio  latency 63.4 / 60 ms  [51-86 ms]  rate -0.058 %  underruns 0
classic output, 2D:  audio  classic output (v0.4.2): 4 blocks of 512 frames, silence blocks 4
classic output, 3D:  audio  classic output (v0.4.2): 4 blocks of 512 frames, silence blocks 5
```

Which is finding 1 of Q-153, measured rather than argued: over twenty seconds of the same game, on
the same machine, the new output under-ran **nothing** and the classic one sent out **four blocks
of zeros in 2D and five in 3D** — four or five 10,7 ms holes with a discontinuity at each edge.
That is what the owner will hear if the fallback is ever switched on, and it is why it is the
fallback.

#### golden-core

```
f79e7df842c3ee43e7d9b67861a927b5   1c2b4639 (recorded by 75b35900 and MF-LEG-3)
f79e7df842c3ee43e7d9b67861a927b5   this tree
cmp: BYTE-IDENTICAL   (98 ROMs, 1 200 frames each)
```

`cmp` on the files, not only on the digests, against the run of step 2 — itself `1c2b4639`'s.
`git diff 1c2b4639` over `zax-nes`, `zax-harness`, `zax-goldens`, `zax-engine3d`, `zax-render`,
`zax-profiles` and `zax-scripting` is **empty**: the core, the 3D engine and the renderer were not
touched at all, which is also why **SMB3, Zelda II and the widescreen** cannot have moved — nothing
outside the audio path and the options window changed.

The four profiler logs and the two baseline ones are in the session scratchpad (`bench/after-new-2d.log`,
`after-new-3d.log`, `after-classic-2d.log`, `after-classic-3d.log`, `before-2d.log`, `before-3d.log`).
G: was at 4,9 GB when the step began — the floor the work order sets — and `target-au2/debug/incremental`
(4,3 GB, pure build cache) was dropped before anything was built; it ended at 8,5 GB with the
baseline tree deleted.
