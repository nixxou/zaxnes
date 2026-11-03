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
| 1 | The map (current rows → new places), the test plan, the coverage test written red | done — `f078668` |
| 2–5 | The tree: the row model and the group rows, the global pages, the game pages with their two readability rules, the pointer, the desktop *Game options* menu | done — `683c016` |
| 6 | Closing: the notes, Q-107 §6, `OPTIONS-LAYOUT.md`, fmt, clippy, the full tests, the release build | done — this commit |

Steps 2 to 5 are **one commit**, not four. The brief's suggested order splits work that one type
holds together: `OptionRow` gained `Page` and `Group`, and the two old variants
(`Optimizations`, `Advanced`) became `Page(…)`. From that first line the crate does not compile
again until the drawing of both console screens and the desktop menu have followed, so there is
no intermediate state whose tests pass, and the protocol asks for commits that build. The
coverage test of step 1 is in the same commit, for the same reason: a test file naming a type that
does not exist yet breaks the build of the step that would carry it alone.

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

### Steps 2–5 — the tree as it was built

#### The row model

`ui/handheld/options.rs` holds the whole tree as tables:

| Table | Rows | Where |
|---|---|---|
| `ROWS` / `TOUCH_ROWS` | 12 / 13 | the main page, Android's with the touch-pad row |
| `OPTIMIZATION_ROWS` | 9 | *Optimisations*, the smooth Lua collector joined |
| `ADVANCED_HUB` | 6 | *Advanced options*: six `Page` rows and nothing else |
| `DISPLAY_ROWS` | 11 | with the groups "Resolution" and "Advanced" |
| `RENDERING_3D_ROWS` | 5 | the style, two `Page` rows, the skybox, the mesh algorithm |
| `ENHANCED_EFFECTS_ROWS` | 7 | with the group "Reflections" |
| `GEOMETRY_ROWS` | 5 | the curvature |
| `LIGHT_SHADOWS_ROWS` | 9 | with the group "Ambient occlusion" |
| `NES_PICTURE_ROWS` | 2 | |
| `CAMERA_ROWS` | 2 | the camera view that left the main page, and the field of view |
| `EMULATION_ROWS` | 5 | |

`OptionsPage::{rows, game_rows}` is the only way a page's rows are read, so the walk of the tree
is one recursive function in each place that needs it (the tests, the own-count of a game, the
desktop menu).

`HandheldOptionsModel` and `GameOptionsModel` each hold `trail: [(OptionsPage, usize); MAX_DEPTH]`
and a `depth`: the pages open over the main one, each with the row to come back to. `MAX_DEPTH`
is 3, the depth of the mockup; `open` does nothing beyond it, so a page added under *Enhanced
effects* would need the constant raised and would not misbehave in the meantime.

#### What a group title is

`OptionRow::Group(key)` carries the key of its text and nothing else:

- `options::step_selection` skips it, in both directions, and `first_selectable` keeps a page from
  opening on one. Both are used by the global model and by a game's, so there is one rule.
- `Painter::group_row` draws it at `set_window_font_scale(0.8)` in a colour dimmer than a value's,
  and **records no zone**: the mouse of the desktop's console mode cannot select what the pad
  cannot. The row keeps its full height, so the slot arithmetic of the scrolling list and of the
  pointer is untouched.
- `pointer::presses` counts the row **zones** between the focused slot and the clicked one instead
  of subtracting their slot numbers. Without that, a click past a title would have moved the
  selection one row too far — the bug the group row would otherwise have introduced.

A third baked font size was the alternative to scaling, and was rejected: the handheld atlas is
rebuilt whenever the display changes, and "smaller" is worth one draw-list call, not a third face.

#### The game's screens

`GameRow::{ResetAll, Option(OptionRow), Widescreen, Hacks}` and the table `GAME_MAIN` (11 rows)
replace "row 0 resets, rows 1…16 are `GAME_ROWS`, then three specials". That is what lets the
widescreen move into the middle of the list, which the mockup asks for. `ADVANCED_ROW`,
`WIDESCREEN_ROW` and `HACKS_ROW` are now indices **into that table**, each held to the row it
stands for by a unit test.

The two readability rules:

1. `console_value_text(row)` gives the game's own value plainly, or `"Global: <value>"`
   (`handheld.game_options.global`, "Global : {0}" in French), which the drawing dims. The desktop
   menu keeps `"Use Global : <value>"` for its entry — a menu has room, a console row has not.
   An own value also carries a marker, `*` in RO-2's cyan, drawn as a **badge** before the value
   so that the span a click steps keeps its place.
2. `own_count(page)` counts the game's own values under a page, the pages below included, and
   `own_count_text` turns it into "2 own" ("2 propres"); the row shows "2 own >", or just ">" with
   none. Both read the game-options model, and a console page is drawn only while it is shown, so
   nothing of this runs while a game plays.

The chevron is the ASCII `>` the console rows already used, not the mockup's `›`: U+203A is
outside the atlas's glyph ranges (`ui/fonts.rs::GLYPH_RANGES`) and would draw as a box.

#### The desktop menu

`draw_game_options` walks `GAME_MAIN` in its order, so the first level **is** the console main
page: the reset, the six value rows, the widescreen between the overscan and the region, the two
submenus, the aids last, then the achievements entry RO-2 put at the end. `draw_game_page` draws
one page of the tree as a submenu and calls itself for a page of a page; a group title is
`text_disabled` under a separator. RO-2's `draw_widescreen_and_aids` split in two, the widescreen
and the aids being at opposite ends of the list now.

RO-2's `GAME_DISPLAY_ROWS`, `GAME_3D_ROWS`, `GAME_CURVATURE_ROWS` and `GAME_ADVANCED_ROWS` are
gone as *menu* tables: the menu has no list of its own left, which a test of the source pins
(`ro_3_the_desktop_game_menu_walks_the_console_tree`).

#### What the brief's one-sentence rule really means

Decision 4 above said five settings; the walk found **six**, from three composite rows. The third
is the frame cap, which moves `video.vsync` between its two values and which the mockup keeps on
the Optimisations page while *Display* gets the `video.vsync` row. `TWO_PLACES` in
`handheld_tree_tests.rs` lists the six with the table of which row causes each.

#### Where the old golden went

`goldens/handheld-advanced-rows.txt` is deleted and `goldens/handheld-option-tree.txt` replaces
it: the whole tree, both trees, page › group › row, with a stable name per row (a setting's
settings-file key, or `row.*` / `page.*` / `[group.*]`). An ignored test regenerates it
(`ro_3_write_the_option_tree_golden`). What the old file pinned and the new one does not — that
`advanced::ROW_SETTINGS` holds 56 settings — is now one assertion in
`ro_3_every_console_setting_is_a_row_the_console_can_step`.

### Step 6 — the closing

#### Q-107 grows a §6, and nothing else is copied

RO-3 implements **no amendment**: the console interface is Rust-only, with no requirement behind
it (Q-036, REQ-UI-207 describes the desktop window alone), so there is no amendment text and no
SPEC chapter edit to copy into `impl/docs/spec/changes/`. What there is, is a layout the owner
decided, and it belongs beside RO-1's and RO-2's rather than in a ticket of its own — splitting
three lots of one reorganisation over three numbers would make R1 read them apart. **Q-107**
therefore gained a §6 and two header lines, and `impl/docs/spec/changes/Q-107.md` is the identical
copy.

§6 records the tree, the three things it adds that are not settings, what is kept exactly, and the
one reservation of decision 4 — read of **rows** rather than of settings, with the table of the
three composite rows and the six settings they share. Code mark: `// Q-107 §6` on `OptionsPage`,
and `// Q-107 §5, §6` on `game_options::page_rows`.

`rust/docs/OPTIONS-LAYOUT.md` gained a §5 for the console tree (the tree itself, what moved, the
three additions, the reservation, what is kept, and the desktop menu that walks it), its title and
preamble now name the three lots, and §1's "what did not change" no longer points at the golden
this lot deletes.

#### Closing checks

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --features zax-app/bundled -- -D warnings` | clean |
| `cargo test --release --workspace --features zax-app/bundled --no-fail-fast` | **2466 passed, 0 failed, 84 ignored** |
| `goldens/lua-api.txt` (`zax-harness::lua_api_tests`) | passes, unchanged |
| `goldens/desktop-options-shown.txt` | passes, unchanged |
| `goldens/handheld-option-tree.txt` | new, and matches the mockup |
| `goldens/handheld-advanced-rows.txt` | deleted, replaced by the tree golden |
| the settings file's keys (`zax-settings::catalogue::no_published_key_disappeared`) | passes, unchanged |
| release `zaxnes.exe` in `rust/target-ro/release/` | built from the final commit, `bundled,wgpu` |

RO-2 left 2453 passing and 83 ignored. RO-3 adds 13 passing and one ignored — the ignored one is
`ro_3_write_the_option_tree_golden`, the regeneration aid of the tree golden, which R0 never needs
to run. The other 83 are unchanged: they need a real 3dSen installation, a ROM corpus or a GPU run.

Every new label is covered without a test of its own: `ui_language_tests` walks `keys::PUBLISHED`,
which the four page and group titles and the two game texts joined, and asserts a text in both
languages for each.

#### Performance

Nothing new runs in the core, the conversion, the Lua hooks, the draw list or the renderer. No
file outside `ui/handheld/`, `ui/zax_ui/menu.rs` and the two string tables was touched.

- **A console page** is drawn only while it is shown, and the tree made the pages *shorter*: the
  longest is 11 rows where the flat advanced page was 56, so `first_visible` and the row loop do
  less work per frame of that screen, not more. The main page lost two rows.
- **A group title** costs one `add_text` and two `set_window_font_scale` calls, on the page that
  has one, while that page is shown.
- **The own-counts of a game** (§2 rule 2) walk the game tree — 30 rows at most, and only the
  subtree under each page row — once per drawn frame of a game-options screen. They read the
  game-options model, which is built when the screen opens; nothing of them exists while a game
  plays.
- **The desktop menu** walks `GAME_MAIN` (11 entries) at the first level and a page's rows only
  while that submenu is open, where RO-2 walked 16 + 3 + a submenu. `GameOptionsModel::walk`'s cap
  is unchanged at 48.
- **The pointer** now counts zones in a span instead of subtracting two slot numbers: at most one
  pass over the zones of the screen shown, on a click, in the desktop's console mode only.
- **The navigation** gained `step_selection`, a loop bounded by the page's length (11 at most) on
  a press of up or down.

#### What R0 should look at in the running application

1. `--console` (or a handheld build): the main page is 12 rows, with no "Camera view" and no
   "Show FPS"; *Advanced options* opens six rows and not 56.
2. *Advanced options › Display*: the titles "Resolution" and "Advanced" are smaller and greyed,
   up and down step straight over them, and a click just past one moves the selection one row.
3. *Advanced options › 3D rendering › Enhanced effects*: three levels deep, the title band reading
   "Advanced options > 3D rendering > Enhanced effects", and B climbing back one at a time.
4. *Optimisations*: nine rows, the smooth Lua collector last.
5. A game's options (Select on the home screen): the widescreen between the overscan and the
   region, the hacks last; a row on the global value reading "Global: …" dimmed, a row with the
   game's own showing `*` in cyan, and "Advanced options   2 own >" once two values are set under
   it — in two different pages of the hub, to see the count add up.
6. The desktop *Game options* menu with a game: the same order as the console main page, the two
   submenus, and the cyan climbing to a submenu title when a row inside holds a value of its own.

### Step 1 — what may not change

- `goldens/lua-api.txt`: no member added or removed. No Lua surface is touched.
- `goldens/desktop-options-shown.txt` and `EXCLUDED`: the options **window** is not part of this
  lot (RO-1's golden stays valid).
- `crates/zax-settings/tests/data/SettingKeys.reference.txt`: no new key.
- the handheld defaults (`ui/handheld/defaults.rs`): no setting is added, so there is no new
  handheld default to choose.
- the per-frame paths: a console page is drawn only while it is shown, and the per-game counts of
  rule 2 are computed when a page is drawn, from the game-options model.
