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
| 1 | Inventory of every shown desktop setting, the golden list, the "nothing disappears" test | done — `17f9f2b` |
| 2 | The new tab/group layout, with the dependency greying | todo |
| 3 | Tooltip infrastructure and the texts of the first tabs | todo |
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

### Next step

Step 2: the new layout. The plan, to be confirmed or refined in this file as it is written:

- **Display** — `ui.fullscreen`, `video.vsync`, `video.frame_rate`; group *Resolution*:
  `video.render_method`, `video.res_scale`, `video.dynamic_resolution`, `video.anti_alias`; group
  *Graphics API*: `video.backend`, `video.max_queued_frames`; group *Performance*:
  `video.skip_frame`, `emu.catch_up`, `emu.catch_up_last_3d`, `emu.pause_3d_in_2d`, `emu.pipeline`,
  `emu.lua_smooth_gc`, `emu.lua_optimized`, `emu.lua_experimental`.
- **NES picture** — `video.nes_palette`, `video.remove_sprite_limit`; group *Overscan*:
  `video.overscan_mode` and the two compact rows of point 6; group *Widescreen*:
  `video.widescreen_strict_edges`.
- **3D rendering** — `render.visual_style`, `render.mesh_algo`, `scene.skybox`,
  `scene.custom_bg_color`; group *Style*: `render.style.voxel_grid`, `render.style.reflections` and
  the four `render.style.ssr_*`; group *Geometry*: the five curvature settings.
- **Light and shadows** — `light.intensity`, `render.shadow`, `render.backdrop_lighting`; group
  *Style*: `render.style.pixel_shadow`, `render.style.ao` and its four parameters.
- **Camera**, **Audio**, **Emulation**, **Interface**, **RetroAchievements** — as they are, less
  `ui.fullscreen` (to Display) and the performance switches (to Display).

Open questions to settle in step 2, each by reading the code path rather than guessing: which
dependencies can honestly be greyed (pixel shadows under `render.shadow` off,
`video.dynamic_resolution` under the direct method, `emu.lua_experimental` under
`emu.lua_optimized` off, `scene.custom_bg_color` off the Color skybox, `scene.arc_center` at
`scene.arc_h` = 0, `emu.catch_up_last_3d` at `emu.catch_up` = None, the four
`achievements.*` under `achievements.enabled` off, `emu.tv_mode` under `emu.auto_tv_mode` on), and
whether the Style group's existing **hide** rule stays a hide (first reading of the brief: yes, it
stays, with the reason spelled out beside the group heading instead of the silence there is today).
