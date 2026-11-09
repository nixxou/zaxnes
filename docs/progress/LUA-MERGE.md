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
- [ ] **3. The editor on `dev`'s core.** `.fm2`/`.fcm` play from power-on and are checked by
      machine digest; `dev`'s core timing moved (MF-4a..e) and its save-states gained board
      state (the MMC5 now has two variants). Check the movie tests, the Metal Force TAS sync
      test, the movie explorer's checkpoints, the script reload against `dev`'s save-states, and
      the MCP server's end-to-end test. Fix in the editor's code, never in the core.
- [ ] **4. Measurements.** Callgrind in WSL (`rust/tools/perf/`) on the five level-2 games with
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
