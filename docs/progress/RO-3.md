# RO-3 — the console options laid out as the owner's mockup

Branch `reworkoptions`, from RO-2 (`336dd8f`). The owner reworked the console (handheld) option
pages on an HTML mockup and approved it; this lot implements that layout and aligns the desktop
*Game options* menu on the same tree.

Build directory: `rust/target-ro`.
`CARGO_TARGET_DIR=G:/3dsen/cleanroom/impl/rust/target-ro`, `--features zax-app/bundled` on
Windows (the workspace has no `SDL2.lib` on the library path).

## Checklist

| # | Step | State |
|---|---|---|
| 1 | The map (current rows → new places), the test plan, the coverage test written red | todo |
| 2 | Group rows and page hubs in the handheld UI (the row model, the drawing, the pointer) | todo |
| 3 | The global console pages | todo |
| 4 | The game pages, with the two readability rules | todo |
| 5 | The desktop *Game options* menu | todo |
| 6 | Closing: the notes, the golden, fmt, clippy, the full tests, the release build | todo |

## Notes

### Uncommitted work found at the start (step 1)

`git status` showed the same three untracked things RO-1 and RO-2 found and left alone, for the
same reasons: `tests/` at the repository root (the .NET test tree, removed by `d3680e4` and still
on disk) and the two scratch probes `rust/crates/zax-render/tests/dump_sources_tmp.rs` and
`probe_tmp.rs`, both headed "Scratch probe, not committed" and gated on `ZAX_PROBE`. Nothing of
RO-3 was found half-done.

### Step 1 — what the console pages look like at `336dd8f`

Four flat lists in `ui/handheld/`, plus the overscan screen and the game's options:

| List | Where | Rows |
|---|---|---|
| `options::ROWS` | the main page | 14 |
| `options::TOUCH_ROWS` | the main page on Android | 15 (the touch-pad row after the controls) |
| `options::OPTIMIZATION_ROWS` | the *Optimizations* page | 8 |
| `advanced::ROW_SETTINGS` | the *Advanced options* page | **56**, one flat list |
| `overscan::GLOBAL_ROWS` | the overscan screen | 9 |
| `game_options::GAME_ROWS` + `GAME_ADVANCED_ROWS` | a game's options | 16 + 14 |

`HandheldOptionsModel` holds `page: Option<(OptionRow, usize)>` — **one** level of page. The
mockup's *Advanced options* is a hub of six pages, two of which have a page of their own
(*Enhanced effects* and *Geometry* under *3D rendering*), so the model needs a depth of three.

### Step 1 — the map, row by row

#### The global pages

**Main page** — 12 rows (13 with the touch-pad row), `Language, Volume, RenderScale, Shadows,
Palette, Overscan ›, Region, Optimisations ›, Advanced options ›, Controls ›, RetroAchievements ›,
Profiler`. "Camera view" and "Show FPS" leave it.

**Optimisations** — the eight it had, plus `EmuLuaSmoothGc` moved off the advanced page: 9 rows.

**Overscan** — unchanged (`overscan::GLOBAL_ROWS`, 9 rows, its own screen).

**Advanced options** — a hub of six rows, and the 56 settings spread over them. The destination of
every one of the 56:

| Setting | New place |
|---|---|
| `ui.show_fps` *(from the main page)* | Display |
| `video.vsync`, `video.frame_rate`, `video.skip_frame` | Display |
| `video.res_scale`, `video.dynamic_resolution`, `video.render_method`, `video.anti_alias` | Display › group "Resolution" |
| `video.max_queued_frames` | Display › group "Advanced" |
| `render.visual_style`, `scene.skybox`, `render.mesh_algo` | 3D rendering |
| `render.style.voxel_grid` | 3D rendering › Enhanced effects |
| `render.style.reflections`, `ssr_shine`, `ssr_clarity`, `ssr_length`, `ssr_quality` | 3D rendering › Enhanced effects › group "Reflections" |
| `scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`, `scene.arc_center`, `scene.bend_2d_mode` | 3D rendering › Geometry (curvature) |
| `light.intensity`, `render.backdrop_lighting`, `render.style.pixel_shadow` | Light and shadows |
| `render.style.ao`, `ao_strength`, `ao_radius`, `ao_thickness`, `ao_quality` | Light and shadows › group "Ambient occlusion" |
| `video.remove_sprite_limit`, `video.widescreen_strict_edges` | NES picture |
| `camera.fov` *(and the Camera view row from the main page)* | Camera |
| `emu.run_ahead`, `emu.game_speed`, `emu.auto_tv_mode`, `emu.tv_mode`, `input.emulated_menu_button` | Emulation |
| `video.nes_palette`, `render.shadow` | **dropped from the hub**: the main page's own rows |
| the nine `video.overscan_*` | **dropped from the hub**: the overscan page |
| `emu.catch_up`, `emu.lua_smooth_gc`, `emu.lua_optimized`, `emu.lua_experimental`, `emu.pipeline`, `emu.pause_3d_in_2d`, `emu.catch_up_last_3d` | **dropped from the hub**: the Optimisations page |

#### The game pages

| Row of `GAME_ROWS` (16) | New place |
|---|---|
| Volume, RenderScale, Shadows, Palette, Overscan | main |
| Region | main |
| FrameCap, HalfRate3d, CatchUp, CatchUpLast3d, Pause3dIn2d, Pipeline, LuaOptimized, LuaExperimental | Optimisations |
| ShowFps, Profiler | Advanced › Display |

| Row of `GAME_ADVANCED_ROWS` (14) | New place |
|---|---|
| `video.anti_alias`, `video.max_queued_frames` | Advanced › Display |
| `render.visual_style` | Advanced › 3D rendering |
| `render.style.voxel_grid`, `pixel_shadow`, `ao`, `reflections` | 3D rendering › Enhanced effects |
| the five curvature settings | 3D rendering › Geometry |
| `render.backdrop_lighting` | Advanced › Light and shadows |
| `emu.run_ahead` | Advanced › Emulation |

16 + 14 = 30 rows in, 6 (main) + 8 (optimisations) + 16 (advanced hub) = 30 rows out. **Nothing is
dropped and nothing is added**, which is what the coverage test holds the lot to.

The game main page also gains a position change the mockup asks for: the **widescreen** row moves
from next-to-last to between *Overscan* and *Region*, and the *Hacks* row stays last. So the main
page is no longer "row 0 resets, rows 1…16 are `GAME_ROWS`, then three specials" and needs a row
list of its own.

### Step 1 — the decisions the map forced

**1. One row enum for both interfaces, with two new variants.** `OptionRow` gains
`Page(OptionsPage)` and `Group(&'static str)`. `OptionRow::Optimizations` and
`OptionRow::Advanced` become `Page(OptionsPage::Optimizations)` and `Page(OptionsPage::Advanced)`,
so that *every* row that climbs to a page is the same variant and the tree can be walked by one
recursive function. The alternative — keeping the two old variants and adding `Page` only for the
eight new pages — would have left the walk with two cases to recognise for no gain.

`OptionsPage` is shared by the global tree and the game tree: `rows()` gives the global page,
`game_rows()` the game's, and the two can differ (the game's *Display* holds the profiler, the
global one does not).

**2. A group title is a row, not a property of the row under it.** `Group(key)` draws a smaller,
dimmed, non-selectable line. Up and down skip it, which is one rule in `step_selection` rather
than a rule per page. The mouse of the desktop's console mode needed the same thing: a group row
records **no** zone, and `pointer::presses` counts the row zones between the focused slot and the
clicked one instead of subtracting their slot numbers — otherwise a click past a group title would
have moved the selection one row too far.

**3. The game's main page is a list of `GameRow`.** `GameRow::{ResetAll, Option(OptionRow),
Widescreen, Hacks}` — the three specials beside the ordinary option rows, in whatever order the
mockup wants. `GameOptionsModel::{ADVANCED_ROW, WIDESCREEN_ROW, HACKS_ROW, ROW_COUNT}` become
functions over that list (`index_of`), so moving a row is a line of the table and not an audit of
four constants.

**4. What "no setting is reachable from two places" means here.** The brief's rule is written of
settings but is true of **rows**: no row appears twice in the tree. Two main-page rows are
*composite* — "Render scale" moves `video.res_scale`, `video.dynamic_resolution` and
`video.render_method` together, and "Region" moves `emu.auto_tv_mode` and `emu.tv_mode` together —
and the mockup deliberately keeps each beside the `Setting` rows of the page that holds the same
settings one at a time (Display's "Render scale", "Dynamic resolution" and "Render method";
Emulation's "Auto region" and "Region"). So five settings, not one, are reachable from two rows:

```
video.res_scale  video.dynamic_resolution  video.render_method    (the Render scale row)
emu.auto_tv_mode  emu.tv_mode                                     (the Region row)
```

The coverage test asserts **exactly that set**, with the reason, rather than the brief's single
name: the brief's own *Emulation* page creates the second half of it, so the one-sentence rule
cannot be read literally. Recorded for R1 in Q-107 §6.

### Step 1 — the test plan

| Part | Test | File |
|---|---|---|
| coverage | every row the console offered at `336dd8f` is reachable from the main page | `handheld_tree_tests.rs` |
| coverage | no row is reachable twice; the settings reachable twice are the five of decision 4 | `handheld_tree_tests.rs` |
| coverage | the same two, for the **game** tree, against `GAME_ROWS` + `GAME_ADVANCED_ROWS` of `336dd8f` | `handheld_tree_tests.rs` |
| golden | the whole tree (page › group › row) matches `goldens/handheld-option-tree.txt` | `handheld_tree_tests.rs` |
| tree | every page is reachable from the main page, and every page's rows are non-empty | `handheld_tree_tests.rs` |
| labels | every group title and every page row has a label in both languages | `ui_language_tests.rs` |
| navigation | a group title is skipped by up and down, in both directions, on every page that has one | `options.rs` unit tests |
| navigation | the three levels open and B climbs back one at a time to the row that opened each | `options.rs` unit tests |
| navigation | the optimisations page holds the smooth GC row and steps it | `options.rs` unit tests |
| pointer | a click past a group title moves the selection by one row, not two | `pointer.rs` unit tests |
| game | the main page's rows and their order, the widescreen between the overscan and the region | `game_options.rs` unit tests |
| game | each of the 30 rows stores a value of its own and "Use Global" clears it | `game_options.rs` unit tests |
| game | a row that uses the global value shows "Global: value" dimmed; its own shows a marker | `game_options.rs` unit tests |
| game | a page row counts the own values under it, nothing when there are none | `game_options.rs` unit tests |
| game | the count is of the model, not of a frame: it follows a change at once | `game_options.rs` unit tests |
| game | the whole path over a real host: the pages open, a value is set, it is in force and written | `handheld_ui_tests.rs` |
| desktop | the menu's first level is the console main page, and its submenus are the tree | `desktop_options_golden_tests.rs` |
| unchanged | `goldens/lua-api.txt`, `goldens/desktop-options-shown.txt`, `SettingKeys.reference.txt` | their own tests |

### Step 1 — what may not change

- `goldens/lua-api.txt`: no member added or removed. No Lua surface is touched.
- `goldens/desktop-options-shown.txt` and `EXCLUDED`: the options **window** is not part of this
  lot (RO-1's golden stays valid).
- `crates/zax-settings/tests/data/SettingKeys.reference.txt`: no new key.
- the handheld defaults (`ui/handheld/defaults.rs`): no setting is added, so there is no new
  handheld default to choose.
- the per-frame paths: a console page is drawn only while it is shown, and the per-game counts of
  rule 2 are computed when a page is drawn, from the game-options model.
