# LUA-MERGE — branch `lua-editor` (the editor mode) onto `dev`

Branch `dev`, from `ab68ee48`. Worktree `wt-rust`. No push, no rebase, no amend, no stash.

`lua-editor` and `dev` diverged at **`553af8b4`** (2026-11-02). Since then:

| side | commits | what it is |
| --- | --- | --- |
| `lua-editor` | **40** | EDITOR-1: `.fm2` movies played in the app, the movie bar and its timeline, script reload, the MCP server (`--mcp`), the editor's pens, the `editor/` folder for AI agents; profile descriptions moved to `zax-profiles`; FCE Ultra `.fcm`; `zax-harness classify` and `profile-build`; docs |
| `dev` | **500** | the whole mapper review (~140 mappers, the MMC5 split, MF-LEG-1..3, MF-UNL-1..12, MF-VS), the core timing lots MF-4a..MF-4e, the non-linear mixer, RO-1/RO-2/RO-3/RO-5 (the options rework), AUDIO-1 and AUDIO-2 with `audio.classic_output`, version 0.5.0, regenerated core goldens |

## Plan — 5 steps

- [x] **1. Plan.** This file.
- [x] **2. Merge `lua-editor` into `dev`.** `git merge --no-ff lua-editor`, message
      `Merge lua-editor: the editor mode`. Resolve the three textual conflicts keeping both
      sides' intent, then hunt the semantic ones the markers do not show (list below). The merge
      commit must build the whole workspace and pass the tests of the crates both sides touched
      (`zax-app`, `zax-harness`, `zax-nes`, `zax-scripting`, `zax-settings`, `zax-profiles`).
- [x] **3. The editor on `dev`'s core.** `.fm2`/`.fcm` play from power-on and are checked by
      machine digest; `dev`'s core timing moved (MF-4a..e) and its save-states gained board
      state (the MMC5 now has two variants). Check the movie tests, the Metal Force TAS sync
      test, the movie explorer's checkpoints, the script reload against `dev`'s save-states, and
      the MCP server's end-to-end test. Fix in the editor's code, never in the core.
- [x] **4. Measurements.** Callgrind in WSL (`rust/tools/perf/`) on the five level-2 games with
      their 3D profiles and the thirteen PR-1 ROMs, against `ab68ee48`, each commit built from
      `git archive` in a fresh directory. Tolerance **+0.02 % per ROM**, totals included.
      Anything over: find the cause, and if it is editor-only work done every frame, put it
      behind editor mode or an option **off by default** — never remove a feature. Plus the app's
      main-thread frame time, 2D and 3D, before and after.
- [ ] **5. Final pass.** fmt; clippy `-D warnings` on every crate touched; `cargo test --release`
      on the whole workspace that builds here with `--include-ignored --no-fail-fast` and both
      ROM variables; golden-core over the 98 originals byte-identical to `ab68ee48`'s; the
      Android and PortMaster feature sets `cargo check`.

The protocol's "progress file in the same commit" cannot hold for step 2: its commit message is
fixed by the work order. Step 2 is therefore recorded here in the commit that follows the merge,
which carries no code of its own — the same arrangement AUDIO-2 used.

## Step 1 — what each side changed where they meet

`git diff --name-only 553af8b4 dev` lists **505** files, `… lua-editor` **72**; the intersection
is **20**. Everything else of either side lands untouched.

| common file | `dev` | `lua-editor` |
| --- | --- | --- |
| `zax-app/src/application.rs` | +275: AUDIO-2's two outputs (`ClassicAudioOutput`, `audio_logged_at`, `reopen_the_audio_output`), RO-1/RO-5 wiring, `-slot=<n>` | +295: the MCP bridge field, `EditorState`, `follow_the_editor_setting`, movie/pens plumbing |
| `zax-app/src/emulation/host/mod.rs` | +346: audio paths, `attach_console`, suspend/resume, emu stats | +88: movie timeline / explorer / annotations fields, `advance_movie_seek` in the tick, checkpoints beside the rollback snapshot |
| `zax-app/src/emulation/host/jobs.rs` | +104 | +15: `--movie` played after a launch; `ensure_session` and `apply_profile_scene` made `pub(super)` for the script reload |
| `zax-app/src/game_session.rs` | +13: `SessionRequest::console_interface` (RO-2 part 4), `take_late_objects` | +78: `run_ahead_held_at_zero`, `core_follows_the_inputs`, `run_movie_frame_core_only`, movie-run move across a reload |
| `zax-app/src/graphics.rs` | +35 | +15: `SceneView::capture_rgb` (the MCP screenshot) |
| `zax-app/src/persistence/app_settings.rs` | +21 | +12: `editor_mode()` / `set_editor_mode()` |
| `zax-app/src/scene_script_bridge.rs` | +104 | +5: the Lua log remembered for `get_log` |
| `zax-app/src/ui/keys.rs` | +198 | +36: the movie dialog, the movie bar and the pens' keys, all in `PUBLISHED` |
| `zax-app/src/ui/menu_bar.rs` | +5 | +6: five Tools entries (`OpenMovie`, `ReloadScript`, `ToggleEditorMode`, `NewLuaProfile`, `CopyAgentPrompt`) |
| `zax-app/src/ui/strings/{english,french}.rs` | +788 / +783: RO-1's `detail.*` paragraphs, AUDIO-2's labels | +23 each: the editor's labels, `setting.ui.editor_mode` |
| `zax-app/src/ui/text.rs` | −6 | +6: `open_movie_title` |
| `zax-app/src/ui/zax_ui/mod.rs` | +6 | +15: the movie bar and the pens drawn, `BrowserPurpose::Movie` |
| `zax-harness/src/lib.rs` | +3: `test_roms`, `vs` | +1: `classify` |
| `zax-harness/src/main.rs` | +37: the `test-roms` and `vs` subcommands | +15: the `classify` subcommand |
| `zax-harness/src/render/rom.rs` | +27 | +46: `render --movie` takes `.fm2`/`.fcm` |
| `zax-scripting/src/lib.rs` | ±1: `read_msaa` exported | +1: `pub mod macros` |
| `zax-settings/src/catalogue.rs` | +11: `scene.bend_2d_mode`, `render.backdrop_lighting`, `audio.latency_ms`, `audio.classic_output` | +1: `ui.editor_mode` |
| `zax-settings/src/table.rs` | +406 | +15: the `ui.editor_mode` descriptor (Interface, user, immediate, off) |
| `zax-settings/tests/catalogue.rs` | +8: the two audio keys and `render.backdrop_lighting` in the addition list | +3: `ui.editor_mode` in it |

### The three textual conflicts (trial merge, then aborted)

1. **`zax-app/src/application.rs`**, four hunks, all "a field / a line added next to the same
   line": the `zax_platform` import list (`ClassicAudioOutput` vs `ProfileImportOutcome`), the
   `Running` fields (`audio_logged_at` vs `mcp` + `editor`), the initialiser, and the first two
   calls of `frame` (`reopen_the_audio_output` vs `follow_the_editor_setting`). **Keep both** in
   every one.
2. **`zax-harness/src/main.rs`**, four hunks: the `use` list (`test_roms` vs `classify`), the
   `USAGE` last line (`vs` vs `classify`), the subcommand arm (`vs` under `#[cfg]` vs
   `classify`), and the `help` format string (`dev` has eight arguments plus `{vs_usage}`,
   `lua-editor` eight). **Keep all three subcommands.**
3. **`zax-settings/tests/catalogue.rs`**, one hunk: `render.backdrop_lighting` vs
   `ui.editor_mode` inserted at the same place of the sorted list. **Keep both**, in order.

### The semantic conflicts to hunt (no marker will show them)

AUDIO-2's merge found two of this kind, both `dev` asserting something about the options window
that a setting born before RO-1 could not satisfy. `ui.editor_mode` is in exactly that position,
and the places `dev` enumerates things are:

| place `dev` enumerates | what `lua-editor` adds to it | expected |
| --- | --- | --- |
| `ui/options.rs` `LAYOUT` + `ro_1_the_layout_table_places_every_shown_setting_exactly_once` | `ui.editor_mode` is `SettingScope::User`, theme `Interface`, so `is_shown` accepts it | **fails**: a row must be added to the Interface tab |
| `options_window_tests::req_ui_202_every_user_preference_is_shown` | same | **fails** for the same reason |
| `goldens/desktop-options-shown.txt` + `ro_1_the_desktop_window_shows_exactly_the_settings_it_showed_before` | same | **fails**: the key joins the golden in the same commit, as the file's own header demands |
| `ro_1_every_shown_setting_has_a_detailed_tooltip_in_both_languages` | `lua-editor` wrote `setting.ui.editor_mode` but no `detail.ui.editor_mode` | **fails**: both paragraphs (> 80 characters) to write |
| `docs/OPTIONS-LAYOUT.md` (RO-1's table, `dev`-only) | — | a row to add |
| `handheld/advanced.rs` `ROW_SETTINGS` + `ro_3_every_console_setting_is_a_row_the_console_can_step` (asserts exactly 57) | — | **leave alone**: the editor mode is desktop-only (pens, MCP server), so it must *not* join the console tree, and `goldens/handheld-option-tree.txt` stays as it is |
| `ro_1_the_excluded_list_is_unchanged` (19 ids) | — | **leave alone**: excluding the key would break this test and hide the row |
| `menu_bar_tests::req_ui_200_every_entry_of_the_table_exists_and_names_a_command` and `…each_entry_shows_its_default_shortcut` | five Tools entries | to check: every new entry needs a command, a label and no shortcut clash |
| `command_model_tests::req_ui_205_the_model_enumerates_every_menu_command` | eight commands, already listed by `lua-editor` | should pass |
| `ui_language_tests::amd_035_the_two_tables_have_the_same_keys…` / `…every_published_key_is_in_both_tables` / `…every_command_and_every_shown_setting_has_a_label` | the editor's keys | `lua-editor` wrote both tables; the new `detail.*` keys must go in both too |
| harness subcommands (`USAGE`, `help`) | `classify` beside `test-roms` and `vs` | the textual conflict above; no golden pins the help text |
| the app's CLI (`app_options.rs`, `three_d_sen_cli_tests.rs`, `app_options_tests.rs`) | `--movie`, `--editor`, `--mcp[=port]` | `dev` touched none of these files — no conflict expected |
| movie formats (`host/movies.rs`, `zax-nes/src/movie.rs`) | `.fm2`, `.fcm` beside Zaxnes's `.mv<n>` | `dev` touched neither — no conflict expected |
| `asset_free_startup_tests::req_pub_002_the_repository_carries_no_asset_file` | the `editor/` folder (`.md`, `.lua`, one `.json`) | should pass; to verify |
| `settings_file.rs`, `goldens/lua-api.txt` | — | neither enumerates exhaustively over the new key; `lua-editor` adds no Lua API surface |

### The risks

1. **The core moved under the editor.** MF-4a..MF-4e changed the core's timing and `ab68ee48`
   regenerated 51 golden rows. The editor's timeline records a `machine_digest` per checkpoint and
   the explorer verifies the movie against it; a `.fm2` that stayed in sync on `553af8b4` need not
   stay in sync on `dev`'s core. The Metal Force TAS test is the one that will say so. This is
   step 3, and the fix belongs to the editor (or to the test's expectation), never to the core.
2. **Save-states gained board state.** The MMC5 has two board variants on `dev` and other boards
   gained state. The timeline's checkpoints are `CoreState` values in memory, so no format
   compatibility is at stake, but `script_reload` restores a core into a *freshly built* session,
   and a board whose state is now larger or differently keyed could refuse the restore.
3. **The editor is not behind a Cargo feature.** EDITOR-1's design note planned a `zax-editor`
   crate behind a non-default feature; the branch as it stands put the code in `zax-app` with no
   feature at all, so the MCP server, the pens and the timeline compile into the PortMaster and
   Android builds as well. That is a step 4 and step 5 question (the per-frame `Option` tests in
   the tick, and the two `cargo check`s).
4. **Per-frame cost on the emulation path.** `lua-editor` added, in the host's tick,
   `advance_movie_seek()` (one `Option` test), a second `bool` in the rollback-snapshot
   condition (`self.movie_timeline.is_some()`), and in `GameSession` a `run_ahead_held_at_zero`
   read where `run_ahead_forced_to_zero` was. Small, but the rule is +0.02 % per ROM and
   `zax-scripting` is on the Lua path of the five level-2 games.
5. **Step 7 of EDITOR-1 was never done** (its own note says so): the zero-cost measurement, the
   PortMaster/Android builds, and the harness golden renders. Steps 4 and 5 here do that work.
6. **`zax-profiles` grew `description.rs`** (617 lines moved out of `zax-harness/src/profile_tool.rs`).
   `dev` touched neither file, so the move lands clean, but `zax-profiles` is a dependency of the
   app and of the Android build: its tests belong to step 2's list.

## Notes per step

### Step 2 — the merge (`30d92cd4`)

A merge was already in progress when this session opened: an earlier session had resolved the
three conflicted files in the working tree but had staged none of them and had committed nothing.
The resolutions were checked hunk by hunk against both stages (`git diff :2:<path>` and
`:3:<path>`) before being kept; all three were correct and complete. The merge was continued, not
restarted.

#### The three textual conflicts — what each side had, what was kept

| # | file | `dev` had | `lua-editor` had | kept |
| --- | --- | --- | --- | --- |
| 1a | `zax-app/src/application.rs`, the `zax_platform` import | `ClassicAudioOutput` in the list (AUDIO-2) | `ProfileImportOutcome` from `zax_profiles::catalogue` | **both** — they are two different `use` lines, not one list |
| 1b | … the `Running` fields | `audio_logged_at: Instant` | `mcp: Option<McpBridge>`, `editor: EditorState` | **all three** |
| 1c | … the initialiser | `audio_logged_at: Instant::now()` | the `mcp` start block and `editor: EditorState{…}` | **all three** |
| 1d | … the first calls of `frame` | `self.reopen_the_audio_output()` | `self.follow_the_editor_setting()` | **both**, audio first (it is `dev`'s existing order and the editor's call does not depend on it) |
| 2a | `zax-harness/src/main.rs`, the `use` list | `test_roms` | `classify` | **both**, plus `dev`'s `#[cfg(feature = "vs-system")] use … vs` above |
| 2b | … the `USAGE` last lines | the `test-roms` and `vs` lines | the `classify` line | **all three lines** |
| 2c | … the subcommand arms | `test-roms`, and `vs` under `#[cfg]` | `classify` | **all three arms** |
| 2d | … the `help` format string | 7 `{}` + `{vs_usage}`, 7 arguments | 7 `{}`, 7 arguments | **8 `{}` + `{vs_usage}` and 8 arguments** (`test_roms`, `meshes`, `lua_api`, `render`, `compare`, `bench_core`, `profile_tool`, `classify`) |
| 3 | `zax-settings/tests/catalogue.rs` | `render.backdrop_lighting` in the addition list, and the two audio keys in the comment | `ui.editor_mode` in both | **both**, in sorted order |

#### The semantic conflicts no marker showed

Five were found. Four are the ones the plan predicted — all of them `dev` enumerating the options
window in a way a setting born before RO-1 cannot satisfy — and the fifth was not predicted and is
the only one that was a *compile* error.

| what broke | `dev` | `lua-editor` | what was done |
| --- | --- | --- | --- |
| **`NesCore::pixel_frame` is gone** (`movie_timeline.rs:37`, `machine_digest`) | AMD-050 REQ-CORE-049 split it into `pixel_frame_with_mask()` (the same `&self.pixels`, the opacity bit now in bit 6) and `pixel_frame_indices_into(&mut Vec)` (a masked copy) | called `core.pixel_frame()` to hash the last picture into the checkpoint digest | the digest now calls **`pixel_frame_with_mask()`**: it is the same borrow, so it costs the same and keeps the digest's meaning, and the extra bit only makes it finer. `pixel_frame_indices_into` was rejected — it allocates and copies 61 440 bytes per digest. Fixed in the editor's code, as the work order requires; `zax-nes` untouched |
| `options.rs::LAYOUT` had no row for `ui.editor_mode` (`ro_1_the_layout_table_places_every_shown_setting_exactly_once`, `req_ui_202_every_user_preference_is_shown`) | the nine-tab `LAYOUT` of RO-1, every shown setting placed exactly once | a `User`/`Interface`/`Immediate` setting, so `is_shown` accepts it, with no row anywhere | a row added to `INTERFACE`, **last, after `ui.profiler`** — the tab is ordered most-common-to-most-advanced and the editor mode is, like the profiler, a tool for whoever makes a profile |
| `goldens/desktop-options-shown.txt` (`ro_1_the_desktop_window_shows_exactly_the_settings_it_showed_before`) | the golden's own header: "no key may join it without leaving this list in the same commit" | — | `ui.editor_mode` added, plain, in sorted order — the same way the merge of AUDIO-1 added `audio.latency_ms` (`6c8ee59c`) |
| `ro_1_every_shown_setting_has_a_detailed_tooltip_in_both_languages` | every shown setting needs a `detail.*` paragraph over 80 characters in **both** tables | wrote `setting.ui.editor_mode` in both, but no `detail.*` | `detail.ui.editor_mode` written in `english.rs` and `french.rs`, saying what the mode turns on (the editor folder, the MCP server on the loopback, the pens), that it changes nothing about how a game plays or looks, and the default |
| `docs/OPTIONS-LAYOUT.md` claimed "no setting has been added to the window or the tree since RO-1" | the claim, already false since AUDIO-1/AUDIO-2 added two audio keys without correcting it | — | the claim replaced by a small table of the **three** settings that have joined the window since RO-1 (`audio.latency_ms`, `audio.classic_output`, `ui.editor_mode`), with a line saying why the editor mode is deliberately absent from the console tree. They are *not* rows of the §4 mapping: that table records where RO-1 **moved** each setting, and a setting born after it has no earlier tab — which is why AUDIO-2 added no row either |

The places that were checked and deliberately **left alone**, as the plan asked:

- `handheld/advanced.rs::ROW_SETTINGS` and `goldens/handheld-option-tree.txt` — the editor mode is
  desktop-only (its MCP server and its pens have no console interface), so it does not join the
  console tree and `ro_3_every_console_setting_is_a_row_the_console_can_step` still counts 57.
- `options.rs::EXCLUDED` — excluding the key would have silenced
  `req_ui_202_every_user_preference_is_shown` by hiding the row, and would have broken
  `ro_1_the_excluded_list_is_unchanged` (19 ids).

Everything else the plan listed as "to check" passed untouched: the five Tools menu entries
(`req_ui_200_every_entry_of_the_table_exists_and_names_a_command`,
`…each_entry_shows_its_default_shortcut`), the eight commands
(`req_ui_205_the_model_enumerates_every_menu_command`), the two label tables (the four `amd_035_*`
tests, the new `detail.*` keys being in both), the `editor/` folder against
`req_pub_002_the_repository_carries_no_asset_file`, the app's CLI (`--movie`, `--editor`,
`--mcp[=port]`, which `dev` never touched) and the movie formats.

#### One more difference, not a conflict: rustfmt

`cargo fmt --all -- --check` was dirty on two of `lua-editor`'s files —
`zax-nes/src/movie/fcm.rs` (an import list sorted the other way round:
`{COMMAND_HARD_RESET, …, Fm2Frame}` where this toolchain wants `{Fm2Frame, …, COMMAND_HARD_RESET}`)
and `zax-app/src/emulation/host/movies.rs` (one blank line). Neither branch has a `rustfmt.toml`,
so the cause is the rustfmt **version** the branch was written with. `cargo fmt --all` was run and
the tree's own toolchain won; both changes are cosmetic, and the workspace was re-checked after.

#### The gate

| command | result |
| --- | --- |
| `cargo build --workspace --all-targets --features zax-app/bundled` | clean |
| `cargo check --release --workspace --all-targets --features zax-app/bundled` (after `cargo fmt`) | clean |
| `cargo test --release -p zax-app -p zax-settings -p zax-profiles -p zax-scripting --features zax-app/bundled --no-fail-fast` | **1 413 passed, 0 failed**, 70 ignored, 75 binaries |
| `cargo test --release -p zax-nes -p zax-harness --no-fail-fast` | **0 failed** over 60 binaries |

Both runs with `ZAXNES_ROMS=G:/3dsen/demo/roms` and
`ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`.

Two notes for whoever resumes:

- **`--features zax-app/bundled` is required** for anything that links SDL2 (`zax-platform`'s and
  `zax-app`'s test binaries): there is no `SDL2.lib` on this machine and no `.cargo/config.toml`
  pointing at one. Without it the link fails with `LNK1181: cannot open input file 'SDL2.lib'`.
  This is the convention AUDIO-2 used.
- **`-j 2`.** Linking the 69 debug test binaries of `zax-app` at full job count exhausted the
  linker's memory (`LNK1102`). Two jobs link them all.
- **The disk filled during the step** (G: reached 1.1 MB free), which showed up as two bogus
  failures — `LNK1318: unexpected PDB error; LIMIT (12)` on the harness binary and a phantom
  `could not compile zax-harness (test "render_rom_tests")`. Both vanished once
  `rust/target-lm/debug` (24 GB, its figures already recorded) was deleted. A link or PDB error
  here is worth checking `df` for before being believed.

### Step 3 — the editor on `dev`'s core

**Nothing had to be fixed in the editor for the core's sake, and nothing was changed in the core.**
The one thing the merge broke was a compile error, not a behaviour: `machine_digest` called
`NesCore::pixel_frame`, which `dev` had split in two. That is recorded under step 2 (it had to be
fixed for the merge commit to build at all).

#### What was run, and what it says

| check | result |
| --- | --- |
| `fm2_movie_tests` (4) — an `.fm2` from power-on at run-ahead zero, a refused `.fm2`, **a seek landing on the same state as playing there**, a movie stopping paused on its last frame | **4 passed**. The third is the movie explorer's checkpoints end to end: it plays 900 frames, waits for the explorer's states to meet the session's own checkpoints (`movie_explorer_trusted() == Some(true)`) and then seeks back, forward, past what was played, and to the start, comparing the work RAM of every landing against a straight play-through |
| `script_reload_tests` (4) — a reload without a movie, a fast reload keeping the frame, **an exact reload replaying the movie with the new scripts**, scripts that change the core forcing an exact reload | **4 passed**. The third and fourth are the save-state path: `old.core().save_state(true)` into `session.restore_core(&state)` on a session built again |
| `zax-nes` `fm2_file_tests` + the `.fcm` reader's tests | **passed** — the readers compile and parse (step 2's run, 0 failures over `zax-nes` and `zax-harness`) |
| the MCP server's own tests (5), `initialize_and_list_answer_without_the_main_thread` among them | **5 passed** — that is the end-to-end one: it drives a real socket through `initialize` and `tools/list` without the main thread serving |
| **new:** `movie_checkpoint_state_tests` (2) | **2 passed** — see below |

#### The new test, and why the obvious version of it was wrong

`fm2_movie_tests` proves the checkpoint invariant, but on a **synthetic NROM image**, and the
mapper review is precisely where board state moved. `tests/movie_checkpoint_state_tests.rs` now
checks the explorer's premise on the real boards of the demo ROMs: for each of fifteen games
(NROM, MMC1, MMC3, UNROM), run 120 frames, `save_state(true)`, build a **fresh** core from the
same ROM — which is what the script reload does — load the block into it, and compare. Ten ROMs
must be found or the test fails rather than quietly passing. All fifteen present pass.

Writing it turned up something worth recording, because it looks like a desynchronisation and is
not one. The first version compared `machine_digest` straight after the restore and **failed on
every ROM**. The cause is not `dev`: the picture is deliberately not part of `CoreState` —
REQ-CORE-121 leaves the frame the host has not finished reading where it is, and `state.rs` names
no pixel buffer on **either** branch (checked: `git show lua-editor:…/state.rs` has no `pixels`
either) — while `machine_digest` does hash the picture. So the digest only means anything between
two cores that have each *run* the frame being compared, which is exactly how the explorer uses
it: it matches an explored state against the session's checkpoint of the **same frame**, both
reached by running. The test now compares the frame count, the cycle count and the work RAM
straight after the restore, and the digest only once both cores have run — and it keeps running
both for 120 more frames, since a board that restored only partly agrees on the counts and the RAM
and still diverges in motion. A second test checks the digest separates frames at all (over 80
distinct values in 90 frames), so the first cannot pass by comparing a constant.

#### The MMC5's two board variants

No MMC5 game is in `G:/3dsen/demo/roms` (that would be Castlevania III), so the new test cannot
reach one. It is covered where it already was: the editor's checkpoints go through
`NesCore::save_state` / `try_load_state` and add no serialisation of their own, and `dev`'s
`zax-nes` tests exercise that same path over the variants —
`mf_leg_3_mmc5_runtime_switch_tests.rs` and `mf_leg_2_mmc5_exattr_split_tests.rs` both use
`save_state` and both pass. Risk 2 of the plan is retired at the mechanism, not by sampling.

#### Movie sync — the owner's decision of 2026-11-09

**No test had to be marked `#[ignore]`.** Every movie test in the repository passes on `dev`'s
core, so there was no desynchronisation to record and none was debugged. The Metal Force TAS is
**not** an automated test: EDITOR-1 checked it by hand with `zax-harness render --movie`, against
a ROM and a 47 454-frame `.fm2` that live outside the repository. It was not re-run — the owner's
decision puts movie sync out of scope, and the readers' own parsing tests are what the work order
asks for.

#### One clippy failure the merge caused, fixed here

`cargo clippy -p zax-app --all-targets -- -D warnings` refused
`EmulationHost::new` at **103 lines of 100 allowed**: `dev` added fields to that initialiser
(`audio`, `classic_audio`, `console_interface`, `suspend_resume`, `emu_stats`, …) and
`lua-editor` added its own (`movie_timeline`, `movie_after_launch`, `movie_explorer`,
`annotations`, `editor_request`), and together they went over. No `#[allow]` was added — AUDIO-2
set that norm. The eleven trailing statements were extracted into a named
`EmulationHost::start_up`, leaving `new` a field-by-field initialiser and nothing else. Behaviour
is identical (same statements, same order, immediately after the struct stands) and the suite was
re-run after it.

#### The gate

| command | result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --release -p zax-app --all-targets --features zax-app/bundled -- -D warnings` | clean, no `#[allow]` added |
| `cargo test --release -p zax-app -p zax-settings -p zax-profiles -p zax-scripting --features zax-app/bundled --no-fail-fast` | **1 415 passed, 0 failed** over 76 binaries (1 413 over 75 at step 2, plus the two new tests) |

### Step 4 — measurements

**The verdict: within tolerance everywhere, and the core is instruction-identical.** Nothing had
to be moved behind editor mode or an option. But the two scripted verdicts both said "ABOVE", and
the reason they are wrong is the substance of this step, so it is written out.

#### What was built

`ab68ee48` (label `base`) and the merge `3579b2c6` (label `head`), each from `git archive` into a
brand-new directory with its own `CARGO_TARGET_DIR`, as `build-clean.sh` does it; both trees
deleted as soon as their binary was out.

#### The thirteen PR-1 ROMs (`callgrind-set.sh`, 300 frames, no pack)

| | |
| --- | --- |
| TOTAL | 79,343,182,333 to 79,343,013,992, **−168,341 (−0.000 %)** |
| per ROM | twelve of thirteen lower; `readbuf` +57,395 (**+0.00084 %**) |
| the script's verdict | `SLOWER on 1 ROM(s): readbuf` |

`readbuf`'s +0.00084 % is **24 times under** the +0.02 % tolerance, so the owner's rule is met
whatever else is true. But the verdict is PR-1's stricter rule — *no ROM slower, to the
instruction* — and that rule rests on a premise this measurement no longer satisfies.

**The premise has failed: the Ir count is no longer deterministic.** `callgrind-set.sh`'s own
comment says "the same binary twice gives the same number". It does not. The same `base` binary,
same ROM, same frames, six times: 6,834,878,034 / 6,834,892,671 / 6,834,888,008 / 6,834,906,400 /
6,834,921,385 / 6,834,894,743 — a spread of **43,351**, and `head`'s six span **62,937**. The
noise floor (about ±50,000 Ir, ±0.0007 %) is *larger than every delta in the table above*, and
over six samples `head`'s mean is **4,300 lower** than `base`'s on the very ROM the verdict names.

The cause is visible in which symbols moved. At a 5,000-instruction threshold, every single one is
Lua's string and table machinery (`luaH_getshortstr`, `getgeneric`, `luaS_newlstr`) or libc
`memcpy`/`memset` — and it moves **both ways**: `readbuf` gains +52,870 on `luaH_getshortstr`
while `smb1` *loses* 48,749 on the same symbol. That is Lua 5.4's per-process randomised string
hashing changing bucket distribution, not a code change. No symbol of the emulated core moved at
all.

#### The five level-2 games (`callgrind-l2.sh`, 600 frames, 3D pack loaded, inputs pressed)

The stage table, which is the instrument that matters here because the pack makes the Lua path run:

| stage | result |
| --- | --- |
| **`core`, `ppu`, `apu`** | **+0 Ir, exactly, on all five games** |
| `total` | metroid −0.006 %, mm2 −0.001 %, smb1 −0.000 %, **zelda2 +0.014 %**, **smb3 +0.043 %** |
| `lua_hooks` | metroid −0.344 %, smb1 −0.005 %, mm2 +0.027 %, zelda2 +0.361 %, smb3 +0.567 % |

`core`, `ppu` and `apu` at **+0 Ir** is the owner's first rule, met exactly and not approximately:
the emulated core executes the identical instruction stream on all five games. And those three
stages are perfectly reproducible — `core` read 15,316,326,163 on **all six** smb3 runs and
12,776,155,979 on all six zelda2 runs. **The core is deterministic; the Lua path is not.** The
noise is confined to exactly the stages that are not deterministic.

`smb3/total` at +0.043 % is over tolerance on that single pair of runs, so it was chased two ways.

**Statistically.** Four runs of each binary on the two games that flagged:

| | base spread | head spread | mean delta | ranges overlap |
| --- | --- | --- | --- | --- |
| smb3/total | 9,280,737 | 6,024,169 | **+0.0142 %** | yes |
| zelda2/total | 701,688 | 2,085,270 | **+0.0039 %** | yes |

`base`'s own spread on smb3 (9,280,737) is nearly the whole single-run delta that triggered the
verdict (10,469,553), the two binaries' ranges overlap, and the four-run mean is **+0.0142 %**,
under the +0.02 % tolerance. The `lua_hooks` ranges overlap too: base's maximum (1,723,292,568) is
above head's minimum (1,717,827,072).

**Structurally, which settles it.** The only thing `lua-editor` adds to the Lua path is one call in
`load_scripts`: `crate::macros::expand(source)` before `compile`. `expand` begins

    if !enabled() || !source.contains("#zax") { return Cow::Borrowed(source); }

and **none of the five packs contains `#zax` or `#legacy-begin`** (checked on each `.3dn`). So for
these five games `expand` is one `str::contains` per script at load time, allocates nothing, and
hands `compile` a **byte-identical** source. The compiled chunk is therefore identical and the
per-frame Lua work *cannot* have changed. Any `lua_hooks` difference is measurement noise by
construction, which is also why it comes out negative on two of the five games.

What `expand` does cost lands in `load_profile`, once per load, and it is +0.016 % on smb3 and
+0.009 % on zelda2 — under tolerance, and not per frame.

#### The app's main-thread frame time

Release builds of both commits (`ab68ee48` extracted with `git archive`, its two untracked demo
files `FlappyZaxNes.nes` and `flappyzaxnes.3dn` copied in by hand or `zax-app` does not compile,
built with its own `CARGO_TARGET_DIR` and deleted after). `Super Mario Bros. 3 [USA][Rev 1][!!]` —
the one game that came near tolerance above — 1280x720, vsync 1, `ZAXNES_PROFILE=1`,
`--smoke-emulated 1200` (20 s), RTX 3050. Mean ms per **displayed** frame over the last three
two-second windows. Editor mode is **off**, which is its default and so the shipped configuration.

| mode | build | frame | core | conv | lua | list | scene | ui |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2D | `ab68ee48` | 16.663 | 2.207 | 0.103 | 0.010 | 0.00 | 0.01 | 14.133 |
| 2D | this tree | 16.663 | 2.213 | 0.107 | 0.010 | 0.00 | 0.01 | 14.117 |
| 3D | `ab68ee48` | 16.667 | **2.200** | **0.070** | **0.160** | 0.03 | 0.19 | 13.290 |
| 3D | this tree | 16.663 | **2.200** | **0.070** | **0.160** | 0.04 | 0.19 | 13.297 |

In 3D every stage agrees **to the hundredth of a millisecond**; in 2D the differences are
±0.006 ms on `core` and `conv`, well inside the spread of a single configuration's own three
windows. Risk 4 of the plan — `advance_movie_seek()`, the second `bool` in the rollback-snapshot
condition, the `run_ahead_held_at_zero` read — costs nothing measurable on the main thread, which
is what those `Option` tests should cost when no movie plays.

A trap worth recording: `--profile <path>` is **not** enough to get a 3D run. A pack that is not in
the user folder's `profiles` is refused with "is not in the library ... Falling back to 2D Mode",
and the run then silently measures 2D. The first 3D attempt here did exactly that; the figures
above are from runs with the pack copied into `<userdata>/profiles` first, and the logs were
checked for that message. The 2D/3D difference in `lua` (0.010 to 0.160) and `scene` (0.01 to
0.19) is what says a run really loaded the pack.

#### Note for whoever measures next

The perf scripts' "deterministic to the instruction" premise holds for `core`, `ppu` and `apu` and
**not** for `total`, `convert`, `lua_hooks` or `load_profile`, because Lua seeds its string hashing
per process. On the Lua-bearing stages a single pair of runs cannot resolve anything below roughly
±0.04 % on `total`; take a mean of several, and prefer the `core` row, which is exact. This says
nothing about earlier lots, whose figures on this instrument were one to two orders of magnitude
above this floor.
