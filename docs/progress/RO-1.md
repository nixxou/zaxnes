# RO-1 — a clearer desktop options window, and a detailed tooltip on every option

Branch `reworkoptions`, from `rustupdated` 0b0aa8c. Presentation lot: no setting is added, removed,
renamed or changed in value, default, domain, persistence key or behaviour, **except** point 7
(finer curvature steps), which is the owner's one deliberate behaviour change.

Build directory: `rust/target-ro` (the owner may hold `rust/target/release/zaxnes.exe` open).
`CARGO_TARGET_DIR=G:/3dsen/cleanroom/impl/rust/target-ro`, and `--features bundled` on Windows
(the workspace has no `SDL2.lib` on the library path).

## Checklist

| # | Step | State |
|---|---|---|
| 1 | Inventory of every shown desktop setting, the golden list, the "nothing disappears" test | done — `4e3221e` |
| 2 | The new tab/group layout, with the dependency greying | done — `58407a7` |
| 3 | Tooltip infrastructure and the texts of the first tabs | done — `PENDING3` |
| 4 | The remaining tooltip texts in both languages, and their completeness test | todo |
| 5 | Compact overscan rows (point 6) and the finer curvature steps (point 7) | todo |
| 6 | The divergence note, the Q ticket, and the closing (full tests, fmt, clippy, release build) | todo |

## Notes

### Uncommitted work found at the start (step 1)

`git status` showed three untracked things, none of them this lot's: `tests/` at the repository
root (the .NET test tree, removed by `d3680e4` and still on disk), and
`rust/crates/zax-render/tests/dump_sources_tmp.rs` and `probe_tmp.rs`, both headed "Scratch probe,
not committed" and gated on `ZAX_PROBE`. All three were left alone — they compile, they run
nothing by default, and none of them is mine to delete.

### Step 1 — the inventory

The window shows **70 settings** in a default build and **71** with the `wgpu` feature
(`video.backend`). Their tabs before this lot:

| Tab | Count | Settings, in the order the window drew them |
|---|---|---|
| Video | 41 (42 with `wgpu`) | `video.vsync`, `video.frame_rate`, `video.skip_frame`, `video.render_method`, `video.max_queued_frames`, `video.res_scale`, `video.anti_alias`, `render.mesh_algo`, `scene.skybox`, `scene.custom_bg_color`, `render.visual_style`, the twelve `render.style.*` (group "Style"), the five curvature settings (group "Geometry"), [`video.backend`], `video.dynamic_resolution`, `video.widescreen_strict_edges`, `video.remove_sprite_limit`, the eight `video.overscan_{ntsc,pal}_*`, `video.overscan_mode`, `video.nes_palette` |
| Camera | 1 | `camera.fov` |
| Light and shadows | 3 | `render.shadow`, `render.backdrop_lighting`, `light.intensity` |
| Audio | 1 | `audio.volume` |
| Emulation | 13 | `emu.game_speed`, `emu.tv_mode`, `emu.auto_tv_mode`, `emu.run_ahead`, `emu.catch_up`, `emu.lua_smooth_gc`, `emu.lua_optimized`, `emu.lua_experimental`, `emu.pipeline`, `emu.pause_3d_in_2d`, `emu.catch_up_last_3d`, `emu.core`, `input.emulated_menu_button` |
| Interface | 6 | `ui.fullscreen`, `ui.sort_option`, `ui.show_fps`, `ui.profiler`, `ui.language`, `ui.roms_folder` |
| RetroAchievements | 5 | `achievements.enabled`, `achievements.encore`, `achievements.unofficial`, `achievements.sound`, `achievements.challenges` |

That is the owner's complaint in one table: 41 of the 70 rows are on one tab, and they are three
unrelated families.

The list is committed as `rust/goldens/desktop-options-shown.txt` (a **set**, sorted by key, with
`video.backend` tagged `[wgpu]`), and `crates/zax-app/tests/desktop_options_golden_tests.rs`
asserts that the window still shows exactly it, each key once, and that `EXCLUDED` is the same
nineteen entries.

### Step 1 — the console's advanced page had to be decoupled first

`ui/handheld/advanced.rs` built its row list by **walking four tabs of the desktop window**
(Video, Camera, Light, Emulation) and keeping every row a pad can step. Reorganising those tabs
would therefore have reordered a console page, which this lot may not touch. The page now carries
its own ordered list, `advanced::ROW_SETTINGS` (56 settings), written out from what the computation
produced at `0b0aa8c`; `rust/goldens/handheld-advanced-rows.txt` pins it, and the test also checks
that every one of the 56 still passes the filter the computation applied (a switch, a choice, or a
number with bounds or named rungs; never `video.backend`, never a folder). A side effect worth
having: adding a setting to the desktop window no longer silently adds a console row.

The console's other three pages (`ROWS`, `TOUCH_ROWS`, `OPTIMIZATION_ROWS`) are plain constants and
are asserted row for row by the same test file.

### Step 1 — one test fix the build directory forced

`asset_free_startup_tests::req_pub_002_the_repository_carries_no_asset_file` walks the repository
and skipped `rust/target` with `Path::starts_with`, which compares whole components: `rust/target-ro`
(and `rust/target-android`, which had the same hole) were walked and their `.obj` files reported.
The skip is now a prefix test, `is_build_output`. `.gitignore` gained `rust/target-ro/`.

### Settings-file format, Lua API

Neither is touched by this lot, and both already have their own guard:
`crates/zax-settings/tests/data/SettingKeys.reference.txt` pins the settings file's keys and
`crates/zax-harness/tests/lua_api_tests.rs` reproduces `goldens/lua-api.txt` byte for byte. Step 6
runs both.

### Step 2 — the layout is now a table, not a derivation

`ui/options.rs` used to derive the window from the catalogue: `tab_of` mapped a *theme* to a tab
and the catalogue's order (the order of the settings **file**) chose the place, with one
hand-written exception that moved the Style rows after `Skybox`. That is exactly how 41 rows ended
up on one tab: the Video, Render and Scene themes all answered "Video".

The window is now `options::LAYOUT`: one line per tab, one entry per row or group heading, in the
order they are drawn. `OptionsModel::new` walks it, drops what `is_shown` rejects (`video.backend`
off a `wgpu` build) and drops a heading left with no row. The catch-all of the old theme mapping
became `is_shown`'s `has_a_tab` — a `vr.*` setting had no row because its theme had no tab, and it
still has none.

New API, all of it pure and testable without a window: `LayoutEntry`, `OptionsGroup`
(`title`, `key`, `is_collapsible`), `TabEntry`, `OptionsModel::entries`, `tab_of(SettingId)`,
`group_of(SettingId)`, `style_group_note`, `inert_note` / `inert_reason`. Gone: `geometry_heading`,
`style_heading`, `style_inert_note`, `move_style_after_skybox`, `text::res_scale_needs_render_texture`
(the reason now comes from `inert_reason`), and `tab_of(&SettingDescriptor)`.

#### The final layout

| Tab (EN / FR) | Group | Settings, in order |
|---|---|---|
| **Display** / *Affichage* | — | `ui.fullscreen`, `video.vsync`, `video.frame_rate` |
| | Resolution / *Résolution* | `video.render_method`, `video.res_scale`, `video.dynamic_resolution`, `video.anti_alias` |
| | Performance / *Performances* | `video.skip_frame`, `emu.catch_up`, `emu.catch_up_last_3d`, `emu.pause_3d_in_2d`, `emu.lua_smooth_gc`, `emu.lua_optimized`, `emu.pipeline`, `emu.lua_experimental` |
| | Advanced / *Avancé* | `video.backend` *(wgpu only)*, `video.max_queued_frames` |
| **NES picture** / *Image NES* | — | `video.nes_palette`, `video.remove_sprite_limit` |
| | Overscan | `video.overscan_mode`, the four NTSC margins, the four PAL margins |
| | Widescreen / *Écran large* | `video.widescreen_strict_edges` |
| **3D rendering** / *Rendu 3D* | — | `scene.skybox`, `scene.custom_bg_color`, `render.visual_style` |
| | Style | `render.style.voxel_grid`, `render.style.reflections`, `ssr_shine`, `ssr_clarity`, `ssr_length`, `ssr_quality` |
| | Geometry / *Géométrie* | `scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`, `scene.arc_center`, `scene.bend_2d_mode` |
| | Advanced / *Avancé* | `render.mesh_algo` |
| **Light and shadows** / *Lumière et ombres* | — | `light.intensity`, `render.shadow`, `render.backdrop_lighting` |
| | Style | `render.style.pixel_shadow`, `render.style.ao`, `ao_strength`, `ao_radius`, `ao_thickness`, `ao_quality` |
| **Camera** / *Caméra* | — | `camera.fov` |
| **Audio** | — | `audio.volume` |
| **Emulation** / *Émulation* | — | `emu.game_speed`, `emu.run_ahead` |
| | Video region / *Région vidéo* | `emu.auto_tv_mode`, `emu.tv_mode` |
| | Advanced / *Avancé* | `input.emulated_menu_button`, `emu.core` |
| **Interface** | — | `ui.language`, `ui.roms_folder`, `ui.sort_option`, `ui.show_fps`, `ui.profiler` |
| **RetroAchievements** | — | `achievements.enabled`, `.sound`, `.challenges`, `.unofficial`, `.encore` |

70 rows, 71 with `wgpu` — the golden set, unchanged.

#### Four refinements of the owner's starting idea, with their reasons

1. **Reflections and the voxel grid stay with the style; the pixel shadows and the occlusion go to
   the light.** The line the brief left open is drawn by what each effect *does*: the Light and
   shadows tab holds the effects that decide **where light does not reach** (the shadows, the pixel
   shadows, which only change how those same shadows are drawn, the ambient occlusion, which is a
   different effect, and the backdrop lighting). The voxel grooves and the floor reflections change
   a **surface**, not the lighting, so they belong with the style. That also keeps the two
   reflection-dependent quartets together under one switch.
2. **`render.mesh_algo` is at the end of the 3D tab, in an Advanced group**, not at the top where
   the owner's list put it. SPEC-10 REQ-UI-061 as amended by AMD-055 requires the `Style` row to
   come **right after the `Skybox` row and its colour button**, and that test still passes; the
   mesh algorithm is the most advanced row of the tab anyway (GREEDY / MARCHING /
   GREEDY_MULTICOLOR).
3. **The Emulation tab is not split further.** The brief suggested pulling conversion, clipping,
   layers and inputs out of it. Every conversion, clipping and layer setting is profile-scoped, so
   none has ever been a row (REQ-UI-061 keeps the video region alone), and the only input setting
   shown is `input.emulated_menu_button`. A tab for one row is worse than the Advanced group it is
   in now.
4. **`ui.fullscreen` left Interface for Display**, which is where the brief puts "window".

#### The window got bigger, and its tab bar scrolls

Nine tabs do not fit in 560 px, and the default fitting policy would shrink the titles to an
ellipsis ("Light and sh…"). The default size is now 820 × 600 with
`TabBarFlags::FITTING_POLICY_SCROLL`: full titles, arrows where the window is still too narrow, and
the window is resizable as before. 520 px of height was sized for a tab 41 rows deep; no tab is
now deeper than 16.

#### The dependency greying — what was verified, and what was not

Every case below was read in the code that *consumes* the setting (the comments of
`options::inert_reason` name the file), never inferred from a label. The row keeps its place and
its value: greying is presentation, and the setting acts again the moment the other one changes.

| Greyed row | While | Where it was verified |
|---|---|---|
| `video.res_scale`, `video.dynamic_resolution` | the render method is Direct | `render_scaling::scale_applies`, `application.rs` arms the governor only for the render texture |
| `video.res_scale` | the automatic resolution is on | `application.rs` reads the governor's factor *instead of* `res_scale_factor()` |
| `render.style.pixel_shadow` | `render.shadow` is off | `renderer.rs` skips the shadow pass and sets `uReceiveShadow` = 0, and the pixel-shadow displacement is inside that branch of `scene.frag` |
| `render.style.ao_thickness` | the MSVO mode is drawn as Depth | unchanged (AMD-055 REQ-SCN-079, `MSVO_PORTED`) |
| `emu.lua_experimental`, `emu.pipeline` | `emu.lua_optimized` is off | `runtime/mod.rs` reads `optimized && experimental`; `PipelineMode::of` returns `Off` outright |
| `scene.custom_bg_color` | the skybox is Gradient or Fantasy | `scene.rs`: those two skies are drawn from the game's own colour and palette and cover the clear in full |
| `scene.arc_center` | `scene.arc_h` is 0 | `space_bend.rs`: the pivot is read only inside `if self.arc.active` |
| `scene.bend_2d_mode` | all three angles are 0 | `scene.rs`: the switch chooses between the curvature and the identity, which coincide |
| `emu.tv_mode` | `emu.auto_tv_mode` is on | `jobs.rs` → `apply_detected_region` *writes* `emu.tv_mode` at the next ROM load |
| `emu.core` | always | `Domain::core_index` takes it modulo 1, so every write reads back 0 and nothing consumes it |
| the four `achievements.*` | `achievements.enabled` is off | `achievements/mod.rs` keeps no client at all, so no option is forwarded and no unlock can fire |

Candidates **rejected** after reading the code, to save the next reader the trip:

- `emu.catch_up_last_3d` at `emu.catch_up` = None: still live whenever the emulated rate exceeds
  the displayed one (fast forward, a low-refresh screen) or the displayed rate is unknown —
  `CatchUp::Off` does not mean "one frame" in `frame_pacer`. The tooltip of step 3 says so instead.
- `video.anti_alias`: inert only where the device grants a single sample, which the settings model
  cannot know.
- `video.max_queued_frames`: inert on the wgpu backend and at depth 2, neither of which is a
  setting the row can point at honestly.
- `video.widescreen_strict_edges`: needs the game's widescreen patch to be applied, which is a
  per-game state, not a global setting.
- the twelve Enhanced parameters in Classic: they stay **hidden**, as REQ-SCN-079 says, because
  "never hidden more than today" is a floor and the brief keeps the Style group's own rules unless
  the aim needs otherwise. What RO-1 adds is one sentence: the heading is now drawn in Classic too,
  with "Style = Classic: these effects are off" under it (`style_group_note`), so a player on the
  Light and shadows tab can see that pixel shadows and ambient occlusion exist at all — which is
  the owner's second complaint, answered without touching a visibility rule.

#### Other families split over several rows of one quantity (point 6's question)

Besides the eight overscan margins, which step 5 compacts, the window holds no other family of the
same shape: the four `video.overscan_*` globals and the three widescreen settings are in `EXCLUDED`
(a game's own options edit them), `light.direction` is a profile vector with no row, and the four
`clip.*` margins and the thirty layer settings are profile-scoped. The Style group's two quartets
(`ao_*`, `ssr_*`) are four **different** quantities, not four sides of one, so they stay four rows.

### Step 3 — the tooltip infrastructure, and the first three tabs

A second key family, `detail.<setting key>`, holds the paragraph; `options::tooltip_of(id)` puts
the short `help.<key>` sentence of the amendment **first and unchanged** — those are the
amendment's own words, and the AMD-055 and AMD-043 tests still check them letter for letter — then
a blank line, then the paragraph. A setting no amendment wrote a sentence for has the paragraph
alone. Nothing is duplicated: the paragraphs were written not to repeat the sentence above them.

Three details of the drawing that matter:

- `ImGui`'s item box for a slider, a combo and a check box **includes its label**, so one
  `is_item_hovered` covers "the row's label or widget" with no extra hit test.
- it is read with `ItemHoveredFlags::ALLOW_WHEN_DISABLED`: without it a **greyed** row reports no
  hover at all, and a greyed row is the one whose explanation is worth the most.
- the tooltip is drawn *after* the disabled scope is dropped, so the explanation of a greyed row is
  not itself greyed, and wrapped at 420 px so a paragraph stays a block.

`tooltip_of` allocates only for the row under the pointer, so an open window costs nothing for the
sixty-nine rows that are not hovered — and the window is not drawn at all while it is closed.

Texts written in this step: the 17 rows of Display (`video.backend` included), the 12 of NES
picture and the 15 of 3D rendering, in English and French. Each was written from `table.rs`,
`ui/handheld/defaults.rs`, `docs/PERFORMANCE-HISTORY.md` §6 and the code path that reads the
setting. Four things the reading turned up and the texts now say:

- `video.anti_alias` is **multisampling**, not a post filter (AMD-032): FFAA is 4 samples, SMAA is
  8 (`geometry::anti_alias_samples`). The two names are only the ones REQ-PERS-017 gave the levels.
- `video.skip_frame` (REQ-SCN-060) keeps the previous picture when no new emulated frame has
  arrived — the camera is the one thing that still moves between emulated frames, which is the
  reason to turn it off.
- the Vulkan and DirectX 12 back end draws **none** of the Enhanced style and applies no automatic
  resolution (`wgpu_backend/renderer.rs`), which the `video.backend` and `render.visual_style`
  texts now warn about.
- `render.mesh_algo`: GREEDY merges faces without regard to colour, GREEDY_MULTICOLOR only equal
  colours, MARCHING builds a smoothed marching-cubes surface (`settings::RenderAlgorithm`), and a
  profile's shape can override the global choice (`instance.effective_algorithm`).

One more row joined the greyed list: `input.emulated_menu_button`, whose `SettingEffect` is
`NoEffect` by DEC-08-6 and DEC-R2-D7 — stored, handed to Lua, acted on by nothing. It is greyed for
the same reason `emu.core` is.

#### Two stale figures found in `docs/PERFORMANCE-HISTORY.md` §6

Reported, not fixed (the table is R0's record, and this lot is a UI lot): §6 gives `emu.catch_up`
a desktop default of 5, which AMD-052 REQ-LOOP-074 took to 2 (`table.rs`), and
`emu.lua_experimental` a handheld default of off, which `ui/handheld/defaults.rs` sets to **on**.
The tooltips follow the code.

### Next step

Step 4: the remaining 27 texts (Light and shadows, Camera, Audio, Emulation, Interface,
RetroAchievements) and the completeness test — every shown setting has a non-empty tooltip in both
languages, and every `detail.*` key belongs to a shown setting.
