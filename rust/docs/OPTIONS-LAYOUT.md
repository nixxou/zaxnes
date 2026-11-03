# The desktop options window: the layout of Zaxnes, and where it diverges from the specification

Lot **RO-1** (branch `reworkoptions`, from `rustupdated` 0b0aa8c) reorganised the desktop options
window at the owner's request. This note records two deliberate divergences and the complete
old-tab → new-tab mapping, so that R1 can amend the specification and so that nobody has to
reconstruct the move from a diff.

Both divergences are raised with R1 as **Q-107**.

The layout itself lives in one table, `crates/zax-app/src/ui/options.rs::LAYOUT`: one line per
tab, one entry per row or group heading, in the order the window draws them. Before RO-1 the
window was *derived* — a setting's theme chose its tab and the catalogue's order (the order of the
settings **file**) chose its place.

---

## 1. Divergence: the tab list of SPEC-10 REQ-UI-202

**What the specification says.** REQ-UI-202 gives the window six tabs: *Video*, *Camera*,
*Light and shadows*, *Audio*, *Emulation*, *Interface*. Zaxnes had added a seventh,
*RetroAchievements* (Rust-only, owner request).

**What Zaxnes does now.** Nine tabs. The *Video* tab is replaced by three:

| REQ-UI-202 | Zaxnes since RO-1 |
|---|---|
| Video | **Display** (*Affichage*) — how the picture reaches the screen and what it costs |
| | **NES picture** (*Image NES*) — what the console puts out, before any 3D |
| | **3D rendering** (*Rendu 3D*) — what the 3D scene looks like |
| Light and shadows | Light and shadows (*Lumière et ombres*) — unchanged in name, and it gained the Enhanced effects that decide where light does not reach |
| Camera, Audio, Emulation, Interface | unchanged |
| *(Rust-only)* | RetroAchievements — unchanged |

**Why.** The *Video* tab received three themes of the settings catalogue at once (`Video`, `Render`
and `Scene`), which made it **41 of the window's 70 rows** and three unrelated families on one
page: what the NES produces (palette, overscan, sprite limit), how the picture reaches the screen
(vsync, render rate, resolution, anti-aliasing, graphics API, the performance switches) and the
look of the 3D scene (style, mesh algorithm, background, curvature). A second complaint was that
the shadows were scattered: the shadow switch and the backdrop lighting were on *Light and
shadows*, while the pixel shadows and the ambient occlusion were on *Video* under the Enhanced
style, with nothing saying how the three relate.

**What did not change.** No setting was added, removed, renamed, or changed in value, default,
domain, persistence key or behaviour by this divergence. The set of settings the window shows is
pinned by `goldens/desktop-options-shown.txt`, computed from the rules of `0b0aa8c` and compared
on every test run. `EXCLUDED` is the same nineteen entries. The console pages are untouched
(`goldens/handheld-advanced-rows.txt`), the Lua API dump is unchanged, and so is the settings file.

### The complete mapping, setting by setting

70 settings, 71 in a build carrying the `wgpu` feature.

| Setting | Old tab | New tab | New group |
|---|---|---|---|
| `ui.fullscreen` | Interface | Display | — |
| `video.vsync` | Video | Display | — |
| `video.frame_rate` | Video | Display | — |
| `video.render_method` | Video | Display | Resolution |
| `video.res_scale` | Video | Display | Resolution |
| `video.dynamic_resolution` | Video | Display | Resolution |
| `video.anti_alias` | Video | Display | Resolution |
| `video.skip_frame` | Video | Display | Performance |
| `emu.catch_up` | Emulation | Display | Performance |
| `emu.catch_up_last_3d` | Emulation | Display | Performance |
| `emu.pause_3d_in_2d` | Emulation | Display | Performance |
| `emu.lua_smooth_gc` | Emulation | Display | Performance |
| `emu.lua_optimized` | Emulation | Display | Performance |
| `emu.pipeline` | Emulation | Display | Performance |
| `emu.lua_experimental` | Emulation | Display | Performance |
| `video.backend` *(wgpu)* | Video | Display | Advanced |
| `video.max_queued_frames` | Video | Display | Advanced |
| `video.nes_palette` | Video | NES picture | — |
| `video.remove_sprite_limit` | Video | NES picture | — |
| `video.overscan_mode` | Video | NES picture | Overscan |
| `video.overscan_ntsc_top` | Video | NES picture | Overscan (compact row) |
| `video.overscan_ntsc_bottom` | Video | NES picture | Overscan (compact row) |
| `video.overscan_ntsc_left` | Video | NES picture | Overscan (compact row) |
| `video.overscan_ntsc_right` | Video | NES picture | Overscan (compact row) |
| `video.overscan_pal_top` | Video | NES picture | Overscan (compact row) |
| `video.overscan_pal_bottom` | Video | NES picture | Overscan (compact row) |
| `video.overscan_pal_left` | Video | NES picture | Overscan (compact row) |
| `video.overscan_pal_right` | Video | NES picture | Overscan (compact row) |
| `video.widescreen_strict_edges` | Video | NES picture | Widescreen |
| `scene.skybox` | Video | 3D rendering | — |
| `scene.custom_bg_color` | Video | 3D rendering | — |
| `render.visual_style` | Video | 3D rendering | — |
| `render.style.voxel_grid` | Video | 3D rendering | Style |
| `render.style.reflections` | Video | 3D rendering | Style |
| `render.style.ssr_shine` | Video | 3D rendering | Style |
| `render.style.ssr_clarity` | Video | 3D rendering | Style |
| `render.style.ssr_length` | Video | 3D rendering | Style |
| `render.style.ssr_quality` | Video | 3D rendering | Style |
| `scene.bend_lr` | Video | 3D rendering | Geometry |
| `scene.bend_tb` | Video | 3D rendering | Geometry |
| `scene.arc_h` | Video | 3D rendering | Geometry |
| `scene.arc_center` | Video | 3D rendering | Geometry |
| `scene.bend_2d_mode` | Video | 3D rendering | Geometry |
| `render.mesh_algo` | Video | 3D rendering | Advanced |
| `light.intensity` | Light and shadows | Light and shadows | — |
| `render.shadow` | Light and shadows | Light and shadows | — |
| `render.backdrop_lighting` | Light and shadows | Light and shadows | — |
| `render.style.pixel_shadow` | Video | Light and shadows | Style |
| `render.style.ao` | Video | Light and shadows | Style |
| `render.style.ao_strength` | Video | Light and shadows | Style |
| `render.style.ao_radius` | Video | Light and shadows | Style |
| `render.style.ao_thickness` | Video | Light and shadows | Style |
| `render.style.ao_quality` | Video | Light and shadows | Style |
| `camera.fov` | Camera | Camera | — |
| `audio.volume` | Audio | Audio | — |
| `emu.game_speed` | Emulation | Emulation | — |
| `emu.run_ahead` | Emulation | Emulation | — |
| `emu.auto_tv_mode` | Emulation | Emulation | Video region |
| `emu.tv_mode` | Emulation | Emulation | Video region |
| `input.emulated_menu_button` | Emulation | Emulation | Advanced |
| `emu.core` | Emulation | Emulation | Advanced |
| `ui.language` | Interface | Interface | — |
| `ui.roms_folder` | Interface | Interface | — |
| `ui.sort_option` | Interface | Interface | — |
| `ui.show_fps` | Interface | Interface | — |
| `ui.profiler` | Interface | Interface | — |
| `achievements.enabled` | RetroAchievements | RetroAchievements | — |
| `achievements.sound` | RetroAchievements | RetroAchievements | — |
| `achievements.challenges` | RetroAchievements | RetroAchievements | — |
| `achievements.unofficial` | RetroAchievements | RetroAchievements | — |
| `achievements.encore` | RetroAchievements | RetroAchievements | — |

### Related, smaller divergences of the same reorganisation

- **SPEC-10 REQ-UI-061 as amended by AMD-055 §3 point 6** asks for the `Style` row right after the
  `Skybox` row and "les douze paramètres … à la suite". The `Style` row is still right after the
  `Skybox` row and its colour button, and six of the twelve follow it; the other six are in a
  `Style` group of the *Light and shadows* tab, which is the answer to the owner's second
  complaint. The visibility rules of REQ-SCN-079 are untouched — in Classic none of the twelve is
  drawn — but the group **heading** is now drawn in Classic too, with the one sentence
  "Style = Classic: these effects are off" where there used to be silence.
- **AMD-043 §6** asks for the curvature group to be "marqué avancé" by a heading. It is now a
  collapsible header, as the `Style` group is; the rule is three rows or more
  (`OptionsGroup::is_collapsible`).
- **RO-1 point 6**: the eight `video.overscan_{ntsc,pal}_*` margins are drawn as **two** rows, one
  per region, with the four sides side by side under short captions. Every one of the eight is
  still its own setting with its own widget and its own tooltip; only the place changed.
- **RO-1 point 3**: every shown row carries a detailed tooltip on hover, in English and French
  (`detail.<setting key>`). Desktop only — a pad and a touch screen have no hover. The short
  `help.*` sentences the amendments give are kept word for word and shown first.
- The window's default size is 820 × 600 (it was 560 × 520) and its tab bar scrolls rather than
  shrinking nine titles to an ellipsis.
- Rows that cannot act while another setting is set a certain way are drawn **greyed with the
  reason beside them** rather than looking live (`options::inert_reason`). The setting keeps its
  value and acts again as soon as the other one changes. The full list is in
  `impl/docs/progress/RO-1.md`.

---

## 2. Divergence: the curvature steps of AMD-043 REQ-SCN-066

**What the specification says.** The three curvature angles `scene.bend_lr`, `scene.bend_tb` and
`scene.arc_h` take −180° to 180° **in steps of 30°** — 13 rungs — and AMD-043 §6 publishes 13
libretro values for each. A value between two rungs is rounded by AMD-040 §3: the nearest rung,
ties to the even one, so a write of 45 gives 60.

**What Zaxnes does now** (owner decision, RO-1 point 7 — the one behaviour change of the lot):
23 rungs, 5° between −30° and 30° and 30° beyond.

```
-180 -150 -120 -90 -60 -30 -25 -20 -15 -10 -5 0 5 10 15 20 25 30 60 90 120 150 180
```

**Why.** A slight bend is what a player wants to tune; 30° is already a deliberate deformation.

**What is kept, exactly.**

- Every one of the amendment's 13 rungs is still a rung, so a settings file written before this lot
  loads unchanged. A value off the new grid is brought onto the nearest rung, ties going to the one
  of greater magnitude — which keeps REQ-SCN-066's own acceptance criterion true: writing 45 still
  reads back 60.
- **A write from a script keeps AMD-040 §3 exactly**: it is rounded onto the 30° grid first
  (`zax_settings::styles::snap_write`), so 45 still gives 60 and 15 still gives 0, and a profile
  written for 3dSen behaves as it always did. The REQ-LUA-114 criterion test passes unchanged.
- A script **read** gives back whatever the player chose, fine rung included.
- The harness `--bend` already took any angle, on or off the grid, and still does.
- No rendering code changed: the curvature maths takes degrees as a continuous `f32`
  (`zax-engine3d/src/space_bend.rs`) and the shader variant switch is still "all three angles zero
  or not".
- `scene.arc_center` is not concerned.

**Two consequences for R1 to weigh.**

1. The **libretro option** of each angle now publishes 23 values, where AMD-043 §6 writes 13. The
   libretro list is also what the console's advanced page steps by, so the two have to agree.
2. The **rung indices** of AMD-059 REQ-LUA-116 (`Setting.VisualStyleParameters`) shift for the
   three angles, the published list having grown from 13 entries to 23: rung 6 was 0° and is now
   −25°. Keeping the reported rungs on the 13-entry list was the alternative, and it was rejected
   because it would have made the log report a value the setting does not hold, which the
   requirement ("the rung each one holds now") forbids. The log and the setting stay consistent;
   the numbers move. `restore_visual_style_parameters` still puts a list back *exactly*: it writes
   a rung through the domain's own rounding, never through the 30° one.
