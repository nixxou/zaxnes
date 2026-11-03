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
| 2 | The RetroAchievements menu move (part 1) | in progress |
| 3 | The new per-game rows: the data, the desktop menu, the console page (part 2) | todo |
| 4 | The Lua curvature rounding (part 3) | todo |
| 5 | `Setting.Msaa` drives the anti-aliasing (part 4) | todo |
| 6 | Closing: the notes, the Q-107 appendix, fmt, clippy, the full tests, the release build | todo |

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
