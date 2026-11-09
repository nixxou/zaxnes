# LUA-MERGE — branch `lua-editor` (the editor mode) onto `dev`

Branch `dev`, from `ab68ee48`. Worktree `wt-rust`. No push, no rebase, no amend, no stash.

`lua-editor` and `dev` diverged at **`553af8b4`** (2026-11-02). Since then:

| side | commits | what it is |
| --- | --- | --- |
| `lua-editor` | **40** | EDITOR-1: `.fm2` movies played in the app, the movie bar and its timeline, script reload, the MCP server (`--mcp`), the editor's pens, the `editor/` folder for AI agents; profile descriptions moved to `zax-profiles`; FCE Ultra `.fcm`; `zax-harness classify` and `profile-build`; docs |
| `dev` | **500** | the whole mapper review (~140 mappers, the MMC5 split, MF-LEG-1..3, MF-UNL-1..12, MF-VS), the core timing lots MF-4a..MF-4e, the non-linear mixer, RO-1/RO-2/RO-3/RO-5 (the options rework), AUDIO-1 and AUDIO-2 with `audio.classic_output`, version 0.5.0, regenerated core goldens |

## Plan — 5 steps

- [x] **1. Plan.** This file.
- [ ] **2. Merge `lua-editor` into `dev`.** `git merge --no-ff lua-editor`, message
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
