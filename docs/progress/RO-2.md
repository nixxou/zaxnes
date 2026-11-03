# RO-2 — the game options menu, more per-game settings, the Lua curvature rounding, and `Setting.Msaa`

Branch `reworkoptions`, from RO-1 (`c99038e`). Four owner decisions, in four parts:

1. the **RetroAchievements** top-level menu goes away: its game entry into "Game options", its
   account entry into "Tools";
2. **more per-game settings**, on the desktop menu and the console's game-options page;
3. a **Lua write of a curvature angle** snaps on the same grid as the menu;
4. a **Lua write of `Setting.Msaa`** drives the real anti-aliasing, within the owner's rule.

Build directory: `rust/target-ro` (the owner holds a copy of the RO-1 build).
`CARGO_TARGET_DIR=G:/3dsen/cleanroom/impl/rust/target-ro`, `--features zax-app/bundled` on
Windows (the workspace has no `SDL2.lib` on the library path).

## Checklist

| # | Step | State |
|---|---|---|
| 1 | The map of what each part touches, and the test plan | done — `31d5aae` |
| 2 | The RetroAchievements menu move (part 1) | done — `6eb88e6` |
| 3 | The new per-game rows: the data, the desktop menu, the console page (part 2) | done — `8c84587` |
| 4 | The Lua curvature rounding (part 3) | done — `9ee3bc5` |
| 5 | `Setting.Msaa` drives the anti-aliasing (part 4) | done — `484841f` |
| 6 | Closing: the notes, the Q-107 appendix, fmt, clippy, the full tests, the release build | in progress |

## Notes

### Uncommitted work found at the start (step 1)

`git status` showed the same three untracked things RO-1 found and left alone, and for the same
reasons: `tests/` at the repository root (the .NET test tree, removed by `d3680e4` and still on
disk) and the two scratch probes `rust/crates/zax-render/tests/dump_sources_tmp.rs` and
`probe_tmp.rs`, both headed "Scratch probe, not committed" and gated on `ZAX_PROBE`. RO-1's last
two commits exist precisely to take those two back out of the index while leaving them on disk;
nothing of this lot touches them. Nothing of RO-2 was found half-done.

### Step 1 — the map

#### Part 1: the achievements menu

The bar is drawn by `ui/zax_ui/menu.rs::draw_menu_bar`, which walks the **data** model
`ui/menu_bar.rs` (`build_menus` → File, Emulation, Video, Audio, Tools) and then draws two menus
that are not in that model: `draw_game_options` (the game's own options, greyed with no game) and
`draw_achievements_menu` (`ui/zax_ui/achievements.rs`). The achievements menu holds three things:
the account line as disabled text, the **game's achievement list** (enabled only with a game) and
**"Account and options…"**, which opens the options window on its RetroAchievements tab.

The move:

- the list entry and the account **line** go into `draw_game_options`, after the option rows;
- the account **entry** goes into the Tools menu, which is data, so it needs a command. There is
  no host command that opens the options window on a chosen tab, so this is a new
  `UiCommand::AchievementsAccount` — see the decision below.
- `draw_achievements_menu` goes away, and with it the only use of `keys::ACH_MENU`. The key and
  its two texts stay: `OptionsTab::Achievements` draws its title from `TAB_ACHIEVEMENTS`, but
  `ACH_MENU` is the name of the feature and is worth keeping as the Tools entry's prefix.

`ui/shots.rs` has **no** shot of the achievements menu — its two RetroAchievements screens are
`OptionsAchievements` (the options tab) and `AchievementList` (the window), neither of which
moves. Nothing in the capture plan changes; the brief's "update any menu screenshot list" has no
subject here, which is worth saying rather than leaving silent.

#### Part 2: the new per-game rows

The per-game options are one list, `ui/handheld/game_options.rs::GAME_ROWS` (16 rows of
`ui::handheld::options::OptionRow`), with `row_settings(row)` naming the settings each row moves.
Both interfaces read that one list: the console page (`ui/handheld/draw.rs::draw_game_options`,
a scrolling list) and the desktop menu (`ui/zax_ui/menu.rs::draw_game_options`, a submenu per
row). `GameOptionsModel` keeps the global values and the game's own over them, and `normalise`
drops anything equal to the global value — which is what makes "Use Global" the default of a new
row for free.

The 14 settings the brief adds are **all 14 of them** already rows of the console's *Advanced
options* page (`ui/handheld/advanced::ROW_SETTINGS`, pinned by
`goldens/handheld-advanced-rows.txt`). That settles where they go on the console: "the same
advanced-option treatment as their global counterparts" means an **Advanced page of the game's
options**, not the main page, whose 16 rows stay exactly as they are.

It also settles how they are drawn and stepped: `OptionRow::Setting(id)` already exists for the
advanced page and already has `options::label`, `options::value_text` and `options::step_row`
through `advanced::label / value_text / step`. The new rows need no widget code of their own —
only `row_settings` has to answer `&[id]` for them, which it answers with `&[]` today.

#### Part 3: the Lua curvature rounding

RO-1 split one normalisation into two: `Domain::grid`'s own (the 23-rung grid, ties to the rung of
greater magnitude) for the interface, the settings file and the harness, and
`styles::snap_write`'s 30° pre-rounding for a Lua write. The owner's new rule is *the same grid as
the menu, nearest value, ties away from zero* — which is, letter for letter, what
`Domain::grid`'s `nearest_rung` already does. So part 3 is the **removal** of the exception:
`snap_write` becomes the domain's rounding for every style parameter, and the two grids are one
again. `ScriptSettings::record_style_parameter`, which RO-1 added so that REQ-LUA-116's rung
round trip could bypass the coarse rule, collapses back into `set_visual_style_parameter`.

#### Part 4: `Setting.Msaa`

`Setting.Msaa` is `video.msaa_2_0` with the mapping of AMD-059 §34 (`v ≤ 1 → 0`, `2 ≤ v ≤ 3 → 2`,
`v ≥ 4 → 4`), written by `api/settings.rs::mapped`. A write reaches the model through
`ScriptSettings::write_int` → `SettingsModel::set_layer(.., SettingLayer::Session)`, so the
session layer and REQ-PERS-010 are already the path a script write takes: nothing new is needed
for "in force until the end of the game, the user's preference restored afterwards, never
persisted" — `end_of_game` drops the session layer.

What is needed is the guard. Its three clauses read:

| Clause | Where the answer is |
|---|---|
| the program runs on a handheld or shows the console interface | a decision of the process (`handheld_mode::resolve`, an Android build), which `zax-scripting` cannot see: a flag set on `ScriptSettings` when the session is made |
| the user's global `video.anti_alias` is above 0 | the model's layers **under** the game's options and the session write — a new `SettingsModel::preference(id)` |
| the game in play has a per-game anti-aliasing option | `SettingsModel::has_game_value(VideoAntiAlias)`, which already exists |

### Step 1 — three decisions the map forced

**1. `UiCommand::AchievementsAccount` rather than a special case in the drawing.** The Tools menu
is data (`ui/menu_bar.rs`), and the point of that module is that "it draws nothing, so the tests
check it without a window". Appending the account entry in `draw_menu_bar` instead would put one
entry of one menu outside the model, where no test can see it. The command is therefore a real
one, and the price is that it joins the bindable commands of the key-config window
(`key_config.rs` chains `UiCommand::ALL`) with no default binding. That is a gain, not a loss, and
it is the only way the new entry is testable; both `ui_language_tests` and
`key_config_window_tests` walk `UiCommand::ALL` and so cover it without a line of their own.

**2. The console's game options grow an Advanced page; the desktop menu grows submenus.** The two
interfaces already disagree about where a costly setting lives — the console hides 56 of them
behind *Advanced options*, the desktop window spreads them over nine tabs — and the brief asks
each to keep its own habit. So: 14 rows on a new `GAME_ADVANCED_ROWS`, drawn on the console as a
page reached from a new row of the game's options, and on the desktop as three top-level rows
(run-ahead, anti-aliasing, GPU queue) plus one *3D rendering* submenu holding the style, the four
effects, the backdrop lighting and a *Curvature* submenu of the five curvature rows. The desktop
groups are slices of the same `GAME_ADVANCED_ROWS`, so neither interface can gain a row the other
has not got; a test asserts the three slices concatenate to it.

**3. The quality and strength sub-parameters stay global.** The brief leaves this open ("may stay
global unless adding them is trivial; say what you chose"). They stay global: the eight of them
(`ao_strength`, `ao_radius`, `ao_thickness`, `ao_quality`, `ssr_shine`, `ssr_clarity`,
`ssr_length`, `ssr_quality`) would take the per-game rows from 14 to 22 and the *3D rendering*
submenu from 7 entries to 15, for parameters that tune an effect rather than decide whether it
runs. The four **switches** are what costs GPU time and what a game is worth setting apart; their
gradations are a preference. Adding them later is one line each in `GAME_ADVANCED_ROWS`.

### Step 1 — the test plan

| Part | Test | File |
|---|---|---|
| 1 | the Tools menu holds the account entry, and the bar has no RetroAchievements menu | `menu_bar_tests.rs` |
| 1 | the account entry opens the options window on the RetroAchievements tab | `achievements_tests.rs` |
| 2 | each new row stores a value of its own, and "Use Global" clears it | `game_options.rs` unit tests |
| 2 | each new row survives a restart through `games.json` | `persistence_tests.rs` |
| 2 | "Put everything back to global" clears every new row | `game_options.rs` unit tests |
| 2 | a per-game value is in force while the game plays and gone at its end | `handheld_ui_tests.rs` |
| 2 | the desktop groups concatenate to `GAME_ADVANCED_ROWS`, every row names its setting | `desktop_options_golden_tests.rs` |
| 2 | the console's advanced page of a game opens, steps a row and goes back | `handheld_ui_tests.rs` |
| 3 | 45 → 60, 75 → 90, −45 → −60, 27.5 → 30, 12.5 → 15, multiples of 30 unchanged | `two_zero_settings.rs` |
| 3 | the menu grid and the Lua grid are the same function, for every style parameter | `two_zero_settings.rs` |
| 3 | a Lua write and a Lua read agree on a fine rung | `zax-scripting::settings::tests` |
| 4 | each of the three clauses ignores the write, and the AA is still stored and read back | `two_zero_api_tests.rs` |
| 4 | with none of them, 0 → off and 2 or 4 → MSAA 4×, in the session layer | `two_zero_api_tests.rs` |
| 4 | the end of the game gives the user's preference back, and nothing is persisted | `two_zero_api_tests.rs`, `persistence_tests.rs` |

### Step 2 — the move itself

The bar is now **five** menus, all of them data: File, Emulation, Video, Audio, Tools. The two
drawn beside the model are down to one, the game options menu.

- `UiCommand::AchievementsAccount` ("RetroAchievements account and options…" / "Compte et options
  RetroAchievements…") is the last entry but one of **Tools**, between About and the movie
  entries. `ZaxUi::run_ui_command` opens the options window on its RetroAchievements tab, which
  is exactly what the old entry did. `ui/frontend.rs::follow_console_request` had to name it
  among the commands that ask nothing of the console mode.
- `draw_achievements_menu` became `draw_achievements_entry`, which draws the account **line** and
  the achievement-list entry, and is called at the end of `draw_game_options` after a separator.
  The list entry keeps its own enabling rule (`host.achievements.game().is_some()`), and the
  whole thing is unreachable without a game because the menu that holds it is greyed — which is
  what the brief asks for.
- `draw_game_options` became a method of `ZaxUi` (it needs `self` for the entry and already
  needed `self.menu_hacks`), and its per-row loop came out as `draw_game_rows(ui, host, model,
  rows)` so that step 3 can call it for a group of rows without copying it.
- `keys::ACH_MENU` is gone, with its two texts: its only reader was the menu, and the string was
  "RetroAchievements" in both languages — word for word `TAB_ACHIEVEMENTS`, which the options tab
  still uses. `keys::ACH_LIST` and `keys::ACH_ACCOUNT` stay; `ACH_ACCOUNT` ("Account and
  options…") is still the label of nothing drawn, so it is a loose end step 6 revisits.
- `ui/shots.rs` needed nothing, as step 1 found: the capture plan never had a shot of the
  achievements menu, only of the options tab and of the achievements window, and neither moved.

Tests: `menu_bar_tests::ro_2_the_retroachievements_account_is_an_entry_of_the_tools_menu` (no
menu of the bar is named after the feature in either language; the entry is in Tools, runs no
host command, is enabled with no game, and opens the window on the right tab) and one line added
to `the_game_options_menu_follows_the_game_in_play` (drawing the game menu, achievements entry
included, opens nothing). `ui_language_tests::amd_035_every_command_and_every_shown_setting_has_a_label`
and `key_config_window_tests` cover the new command's texts and its row for free, walking
`UiCommand::ALL`.

715 tests pass in `zax-app`, `cargo fmt --check` and `cargo clippy -p zax-app --all-targets` are
clean.

### Step 3 — the fourteen new per-game rows

The fourteen settings, in the order the console page lists them:

| Group (desktop) | Rows |
|---|---|
| at the level of the sixteen | `emu.run_ahead`, `video.anti_alias`, `video.max_queued_frames` |
| *3D rendering* submenu | `render.visual_style`, `render.style.voxel_grid`, `render.style.pixel_shadow`, `render.style.ao`, `render.style.reflections`, `render.backdrop_lighting` |
| *Geometry* submenu of it | `scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`, `scene.arc_center`, `scene.bend_2d_mode` |

The labels are the settings' own, so the rows read "Anti Alias: Use Global : FFAA" exactly as the
sixteen read "Shadows: Use Global : enabled". The *3D rendering* and *Geometry* titles are the
window's own `TAB_RENDER_3D` and `GROUP_GEOMETRY` — the five curvature settings are the
*Geometry* group of the window too, so the two places agree without a new string.

#### What the existing structure gave for free

`OptionRow::Setting(SettingId)` already existed for the console's global advanced page, with
`options::label`, `options::value_text` and `options::step_row` routed to `advanced::label /
value_text / step`. So **no widget code was written**: a new row is a line in a table. Two things
had to be taught:

- `row_settings` answers `&[id]` for the fourteen, through a `one_setting(id)` match of
  one-element `'static` slices. A `Setting` row that is *not* one of the fourteen — the
  fifty-six of the global page come through the same variant — answers `&[]`, so it can never
  become an option of a game. A test checks both halves of that.
- `normalise` and `is_game_row_setting` walk `all_game_rows()` = `GAME_ROWS` then
  `GAME_ADVANCED_ROWS`, instead of `GAME_ROWS` alone.

`GameOptionsModel::walk`'s `LIMIT` went from 24 to 48. A curvature angle has 23 rungs since RO-1,
so 24 was *exactly* the length of the longest list: one rung more and the last would have been
dropped in silence. No existing row has more than 11 values, so nothing else changed.

#### The console: a page of the game's options

`GameOptionsModel` gained `page: Option<usize>` — the row of the main screen to come back to —
exactly as `HandheldOptionsModel` holds its two pages. Its API: `in_advanced`, `page_rows`,
`row_count`, and `selected_row` is now public and answers the page's row or the main screen's.
The main screen is 20 rows where it was 19:

| Index | Row |
|---|---|
| 0 | "Put everything back to global" |
| 1 … 16 | the sixteen of `GAME_ROWS`, untouched and in the same order |
| 17 | **Advanced options** (new) — `>`, in the colour of a game's own value when one of the fourteen holds one |
| 18 | the widescreen |
| 19 | the hacks |

On the page: Left/Right/A step the row, Up/Down wrap round the **page**, X still puts every
option of the game back without leaving the page, B comes back to row 17. `select(index)` leaves
the page, which is what the widescreen hold needs when it re-reads the options and selects
`WIDESCREEN_ROW`.

That is the brief's "same advanced-option treatment as their global counterparts", read from the
code rather than from the label: all fourteen are rows of `advanced::ROW_SETTINGS`, the console's
global advanced page, and none is on its main page, its touch page or its Optimizations page. A
test asserts exactly that, so the two pages cannot drift apart.

#### The desktop menu

`draw_game_rows(ui, host, model, rows, id_base)` draws any group; `draw_game_advanced` calls it
three times, the last two inside submenus. `id_base` (0, 100, 200, 300) keeps a row of one group
apart from a row of another at the same level of the menu whatever their labels — ImGui would
have hashed the labels and got away with it today, but not if two groups ever showed the same
text.

Q-103: the run-ahead row of the menu shows `ui::options::forced_run_ahead_note` above its values
while the game in play forces the effective value to 0, and the console page shows the shorter
`handheld::options::forced_run_ahead_note` beside the value, which is what the global page
already did. The guard itself was not touched: it lives where the loop reads the effective value,
so a per-game run-ahead goes through it like a preference.

#### The sub-parameters, and what a game cannot set

Kept global, as step 1 decided and for the reasons written there. Also still global, because the
brief does not list them: the SSR and AO gradations, the mesh algorithm, the skybox, the light,
the camera and the overscan globals (`EXCLUDED`).

#### Tests

`game_options.rs` unit tests: the three desktop groups concatenate to `GAME_ADVANCED_ROWS`, each
row names exactly one setting, no setting is a row twice, none overlaps the sixteen, and a
`Setting` row this lot did not add names nothing
(`ro_2_the_desktop_groups_are_the_advanced_rows_and_nothing_else`); every one of the fourteen
stores its own value, "Use Global" takes it back and "Put everything back to global" clears all
fourteen at once (`ro_2_every_new_row_stores_its_own_value_and_goes_back_to_global`); the page
opens, steps, X resets from inside it and B comes back
(`ro_2_the_advanced_page_of_a_game_opens_steps_and_goes_back`); a curvature row of a game offers
the 23 rungs of RO-1 and not the 13 of AMD-043; the main screen kept its rows and its indices.

`zax-settings::game_config::ro_2_the_new_options_of_a_game_survive_a_restart`: all fourteen
written to `games.json` and read back, including a 15° rung the 30° grid has not got. One thing
the test found: `scene.bend_2d_mode` is a **switch** (AMD-046 REQ-SCN-074), not a number, and
`json::to_json` rightly drops an `Int` written for it — the test says `Bool`.

`handheld_ui_tests::ro_2_the_advanced_options_of_a_game_are_a_page_of_their_own`: the whole path
over a real host — the row, the page, a value set on the anti-aliasing row, the preference left
alone, the value in force while the game plays, the global value back at its end, and the value
still written down afterwards. `HandheldUi::game_options()` was added for it, beside the existing
`hacks()`.

`desktop_options_golden_tests::ro_2_the_new_per_game_rows_change_no_global_page`: none of the
fourteen joined a global console page, every one is still shown by the options window (so "Use
Global" has a global value to point at) and is a row of the console's global advanced page, and
the desktop grouping is the same fourteen.

`zax-app` and `zax-settings` pass with 0 failures; `cargo fmt --check` and
`cargo clippy --workspace --all-targets` are clean.

### Step 4 — one grid again

The owner's rule — "the same grid as the menu, to the nearest value, ties away from zero" — is,
letter for letter, what `Domain::grid`'s `nearest_rung` already did for everybody else
(`domain.rs`: "the nearest of its published rungs, ties going to the one of greater magnitude").
So part 3 is a **removal**, not a new rule:

- `styles::snap_write` is now `descriptor(id).domain().snap(value)` for every style parameter.
  `CURVATURE_LUA_STEP` and `is_curvature_angle` are gone. The function itself stays, rather than
  its callers going to the domain directly, because it is the one place a *script* write is
  normalised — the next divergence between a script and the interface, if the owner ever wants
  one, lands there and nowhere else.
- `ScriptSettings::record_style_parameter` is gone with it: RO-1 had split
  `set_visual_style_parameter` in two so that REQ-LUA-116's rung round trip could bypass the 30°
  rounding, and with one rounding left there is nothing to bypass.
  `restore_visual_style_parameters` now writes a rung through the ordinary write.

Every case the brief names, checked: 45 → 60, 75 → 90, −45 → −60, 27.5 → 30, 12.5 → 15, 15 → 15,
and all thirteen multiples of 30 unchanged. `spl.arc_center` keeps its two rungs.

#### REQ-LUA-116's rung indices

They did **not** have to move again. The indices RO-1 noted as shifted are a consequence of the
**domain** growing from 13 rungs to 23 (RO-1 point 7), which part 3 does not touch; the Lua
*write* rounding never entered them. What part 3 does change is their **reach**: before it, a Lua
write could only land on one of the 13 multiples of 30, so a script could only ever see the rung
indices those occupy; now it can write any of the 23, read it back exactly and put the list back
exactly — through the one normalisation there now is.
`two_zero_api_tests::ro_2_the_written_list_carries_a_fine_curvature_rung_there_and_back` is that,
end to end in Lua, with 15°, −5° and 25°.

#### Tests changed, and why

Two tests pinned the rule the owner removed, and both now pin the new one:

- `zax-scripting::settings::tests::ro_1_a_script_write_of_a_curvature_angle_keeps_the_thirty_degree_rounding`
  → `ro_2_a_script_write_of_a_curvature_angle_snaps_on_the_menu_grid`, with the new cases and a
  sweep over all thirteen multiples of 30, written and read back unchanged.
- `two_zero_api_tests::req_lua_114_a_style_parameter_is_normalised_as_amd_040_requires` keeps its
  name and the amendment's own criterion value (45 → 60); the `15 → 0` case became `15 → 15`, and
  75, 12.5, −45 and 30 joined it.

New in `zax-settings::two_zero_settings`:
`ro_2_a_curvature_value_from_a_script_snaps_on_the_menu_grid` (the owner's cases),
`ro_2_the_menu_grid_and_the_lua_grid_are_the_same` (a 0.5-step sweep over the whole domain of
**every** style parameter and a little past each end, asserting `snap_write == domain.snap` — not
two rules that happen to agree on the named cases, one function), and
`ro_2_the_three_angles_round_alike_from_a_script`.

Documentation: `rust/docs/OPTIONS-LAYOUT.md` gained a §3 for this divergence, with the before/after
table and what is kept exactly; §2's bullet about the Lua write points at it. The comments of
`table.rs::CURVATURE_DEGREES` and `curvature()` and of `Domain::snap` say there is one grid again.

The whole workspace passes.

### Step 5 — `Setting.Msaa`

The session layer needed nothing: a script write already goes to it
(`ScriptSettings::write` → `set_layer(.., SettingLayer::Session)`), and `end_of_game` drops it.
So REQ-PERS-010's "in force until the game ends, the preference back afterwards, never persisted"
came for free, and the work was the guard.

Three pieces:

- `ScriptSettings::write_msaa(written)` replaces the `mapped` property of `Msaa`. It stores
  `normalise_msaa(written)` in `video.msaa_2_0` — the amendment's mapping, unchanged, now a
  public `const fn` so there is one copy of it — and, when the guard allows, writes
  `video.anti_alias`: 0 → None, anything else → FFAA (Zaxnes has no 2×; FFAA is MSAA 4× by
  AMD-032 §2).
- `ScriptSettings::msaa_drives_anti_alias()` is the guard. Clause 1 reads a new field
  `console_interface`, clause 2 a new `SettingsModel::preference(id)`, clause 3 the existing
  `has_game_value`.
- `SettingsModel::preference(id)`: the value the user's **own** choice gives — command line, then
  the preference (or the profile value for a P setting), then the environment default, then the
  catalogue default — skipping a game's options and the session layer. Without it, a session value
  a previous write of the same script left behind would look like the user asking for
  anti-aliasing and the second write would be ignored.

#### The flag, and where it is read

`console_interface` is a fact of the **process**, not of the model, and `zax-scripting` cannot see
it: `handheld_mode::resolve` lives in `zax-app`. So it is carried —
`AppOptions{handheld, console}` → `configure_pacing` → `EmulationHost::console_interface` →
`SessionRequest::console_interface` → `ScriptSettings::set_console_interface` — and read once,
when the session is made. `--console` is in it because the desktop's console mode *is* the console
interface; F1 switching the presentation mid-game is not a change of platform and deliberately
does not change the answer. Default `false`, which is the desktop and what every existing test
harness gets.

#### The rule is one-directional, and AMD-059's criterion still passes unchanged

Worth writing down because it surprised me while reading it. `video.anti_alias` ships at **1**
(AMD-052 REQ-PERS-089, a divergence of its own), so clause 2 — "the user's global value is above
0" — is the **ordinary desktop case**: on a default install the write is ignored. What the rule
allows is exactly one thing: a script may *add* anti-aliasing for a user who has none, on a
desktop, for one game, for the length of that game. It can never take a user's own away, never
downgrade SMAA to FFAA, never cost a handheld a frame, and never override a choice made for that
very game.

That reading is coherent — "a script may never remove the user's anti-aliasing, only add where
there is none" — and it is the letter of the brief, so it is what the code does. The consequence
is that AMD-059's own acceptance criterion (`= 3` → 2, `= 1` → 0, `= 8` → 4, `video.anti_alias`
unchanged) still passes on a default model and its test is **untouched**, which is a good sign
rather than a lucky one. The alternative reading — "unless the user made an *explicit* choice",
i.e. testing the user layer rather than the value — would let a script turn a default-on
anti-aliasing off; it is raised with R1 in Q-107 rather than chosen here.

#### Tests

`two_zero_api_tests`, four tests, one per branch and one for the Lua surface:

- `ro_2_a_lua_msaa_write_drives_the_anti_aliasing_when_the_user_has_none` — the seven mappings,
  the session layer, the preference unmoved, what the settings file would hold, and the end of the
  game giving the preference back;
- `ro_2_a_lua_msaa_write_leaves_the_anti_aliasing_alone_on_a_handheld` (clause 1);
- `ro_2_a_lua_msaa_write_leaves_a_users_own_anti_aliasing_alone` (clause 2, both the shipped
  default of 1 and an explicit SMAA);
- `ro_2_a_lua_msaa_write_leaves_a_games_own_anti_aliasing_alone` (clause 3, with the game's value
  at 0, 1 and 2, and the same write acting once the option is gone — so the clause is what blocks
  it and nothing else);
- `ro_2_the_lua_member_reads_back_the_normalised_value_either_way`, through the member itself.

`zax-settings::layering::ro_2_the_preference_is_read_under_the_game_and_the_session` is the new
accessor: the default, an environment default, the user's own, a game's option, a session write
and a command-line override.

`zax-app::game_session_script_tests::ro_2_the_session_carries_the_console_interface_to_the_lua_view`
is the plumbing from `SessionRequest` to `ScriptSettings`. The rule itself is not re-tested there:
in that harness the session only holds the host's model inside a `lend`, so a bare `evaluate`
writes to a scratch model — a trap worth recording for the next reader.

`goldens/lua-api.txt` is unchanged, as required: the bespoke property publishes the same member
with the same kind and the same rights as the `mapped` helper did.

The whole workspace passes; `cargo fmt --check` and `cargo clippy --workspace --all-targets` are
clean.

### Step 6 — the closing

#### The same wildcard mistake as RO-1, and the same fix

`git add -A rust/crates` in step 5 swept the two scratch probes
(`zax-render/tests/dump_sources_tmp.rs`, `probe_tmp.rs`) into `484841f`, exactly as RO-1's step 2
had done. `git rm --cached` took them back out in a commit of its own (`6acf169`); both files are
still on disk, untracked, as they were found. Worth leaving written down: a wildcard `add` under
`rust/` will do this every time, and the probes are nobody's to delete.

#### Q-107, extended rather than replaced

RO-2 raises no new ticket number: its two divergences are the same family as RO-1's, on the same
settings, and splitting them over two tickets would make R1 read them apart. **Q-107** therefore
gained three sections and two corrections, and its title and preamble now say that two lots wrote
it:

- §3, the Lua curvature rounding, which **replaces** the second bullet of §2's "what is kept"
  (the old text is struck through there rather than deleted, so R1 can see what moved);
- §4, `Setting.Msaa`, which **narrows** AMD-059's divergence (D) — and which carries the one real
  **question** of this lot, see below;
- §5, the fourteen new per-game rows and the menu-bar move, which contradict no requirement (the
  per-game options are Rust-only, Q-036) but are worth recording beside the rest;
- §2's consequence 2 gained a parenthesis: the REQ-LUA-116 rung indices do **not** move a second
  time, and what RO-2 changes is their reach.

Code marks: `// Q-107 §3` on `styles::snap_write`, `// Q-107 §4` on `write_msaa` and
`msaa_drives_anti_alias`, `// Q-107 §5` on `GAME_ADVANCED_ROWS`. Copied to
`impl/docs/spec/changes/Q-107.md`, identical. This lot implements no amendment, so there is nothing
else to copy; the four amendments it touches (AMD-040, AMD-043, AMD-052, AMD-059) were already
there.

#### The one question for R1

`Setting.Msaa`'s clause 2 — "the user's global `video.anti_alias` is above 0" — is worded as a
test of the **value** and justified as a test of the user's **will** ("the user made an explicit
choice"). The shipped default of 1 (AMD-052 REQ-PERS-089) keeps the two apart: with the value test
a write is ignored on a fresh install; with a will test (does the user layer hold anything) a
script could **turn off** a default-on anti-aliasing. The letter of the owner's rule is what is
implemented, being the only one of the two that can never let a script degrade the picture of a
user who asked for nothing. Q-107 §4 puts the choice in front of R1; the fix, either way, is one
line.

#### Closing checks

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --features zax-app/bundled -- -D warnings` | clean |
| `cargo test --release --workspace --features zax-app/bundled --no-fail-fast` | **2453 passed, 0 failed, 83 ignored** |
| `goldens/lua-api.txt` byte for byte (`zax-harness::lua_api_tests`) | passes, unchanged |
| `goldens/desktop-options-shown.txt`, `goldens/handheld-advanced-rows.txt` | pass, unchanged |
| the settings file's keys (`zax-settings::catalogue::no_published_key_disappeared`) | passes, unchanged |
| release `zaxnes.exe` in `rust/target-ro/release/` | built from the final commit, `bundled,wgpu` |

RO-1 left 2428 passing; RO-2 adds 25 and removes none. The 83 ignored are unchanged: they need a
real 3dSen installation, a ROM corpus or a GPU run (`ZAXNES_3DSEN_INSTALL`, `ZAXNES_ROMS`, the
golden-core sweeps), and they are R0's to run.

#### Performance

Nothing new runs in the core, the conversion, the Lua hooks, the draw list or the renderer.

- **The per-game rows** are applied by `set_game_values` when the game starts or the user edits
  one, exactly as the existing sixteen are; the frame loop reads the settings model it always
  read.
- **The Lua rounding** got *cheaper*: `snap_write` lost a branch, a clamp and a multiplication,
  and it runs on a script write, never per frame.
- **`Setting.Msaa`** adds, on a Lua write of that one member, one read of the model's user layer
  and one map lookup. A read of the member is unchanged.
- **The desktop menu** draws fourteen more rows only while the *Game options* menu is open, and
  the two new submenus only while they are open. `GameOptionsModel::walk`'s cap went from 24 to 48
  iterations, which bounds work that happens when a submenu is open and never otherwise.
- **The console** draws its new page only while it is shown; the main page draws one row more.

#### What R0 should look at in the running application

1. The menu bar: five menus, no RetroAchievements; *Tools* → "RetroAchievements account and
   options…" with no game, and the game's achievement list at the end of *Game options* with one.
2. *Game options* with a game: the three new rows at the first level, the *3D rendering* submenu
   and its *Geometry* submenu, and the cyan colour climbing to the group title when a row inside
   holds a value of its own.
3. The console's game options: the new *Advanced options* row at index 17, its page of fourteen,
   and the Q-103 note beside the run-ahead row for a profile whose scripts change the core.
4. A 2.0 profile that writes `Setting.Msaa`: on a default desktop install nothing should change,
   which is the point of the rule — the interesting run is with `video.anti_alias` set to None
   first.
5. `--handheld` with the same profile: the anti-aliasing must stay off.

### Step 1 — what may not change

- `goldens/lua-api.txt`: no member added or removed. `Setting.Msaa` keeps its name, its kind and
  its read/write-ness; only what its setter does changes.
- `goldens/desktop-options-shown.txt` and `EXCLUDED`: the options **window** is not part of this
  lot, and no setting is added or removed anywhere.
- `goldens/handheld-advanced-rows.txt`: the console's *global* advanced page is untouched. The new
  page is the game's, and it has a list of its own.
- `crates/zax-settings/tests/data/SettingKeys.reference.txt`: no new key.
- the per-frame paths: every new value is read when it changes (`set_game_values`), as the
  existing per-game options are.
