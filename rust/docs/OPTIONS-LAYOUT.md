# The options of Zaxnes: the layout of the window and of the console tree, and where they diverge from the specification

Lot **RO-1** (branch `reworkoptions`, from `rustupdated` 0b0aa8c) reorganised the desktop options
window at the owner's request; **RO-2** changed what a script may write and what a game may hold;
**RO-3** reorganised the **console** option pages on the same branch; **RO-5** made the
anti-aliasing labels say what the renderer does and gave the arc centre the quarters between its
two heights. This note records the deliberate divergences and the complete mappings, so that R1 can
amend the specification and so that nobody has to reconstruct a move from a diff.

Every divergence here is raised with R1 as **Q-107**. §5 is the console tree of RO-3; §6 and §7 are
the two owner requests of RO-5.

Lot **RO-4** (patches always hooked) was **withdrawn by the owner** and is not on this branch; it
appears in no section here.

Three settings have joined the **window** since RO-1, and none the console tree. They are not rows
of the mapping of §4 — that table records where RO-1 *moved* each setting, and a setting born
after it has no earlier tab — so they are listed here instead:

| setting | tab | group | lot |
|---|---|---|---|
| `audio.latency_ms` | Audio | Advanced | AUDIO-1 |
| `audio.classic_output` | Audio | Advanced | AUDIO-2 |
| `ui.editor_mode` | Interface | — | EDITOR-1 |

The editor mode is deliberately absent from the console tree: its MCP server and its pens are
desktop-only, so `handheld/advanced.rs::ROW_SETTINGS` does not list it.

The window's layout lives in one table, `crates/zax-app/src/ui/options.rs::LAYOUT`: one line per
tab, one entry per row or group heading, in the order the window draws them. Before RO-1 the
window was *derived* — a setting's theme chose its tab and the catalogue's order (the order of the
settings **file**) chose its place.

The console's layout lives in the tables of
`crates/zax-app/src/ui/handheld/options.rs::OptionsPage`, one per page, and in
`game_options.rs::page_rows` for the reduced tree of one game. Before RO-3 it was four flat lists.

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
on every test run. `EXCLUDED` is the same nineteen entries. The console pages were untouched by
RO-1 — §5 is where RO-3 reorganises them, and `goldens/handheld-option-tree.txt` is what pins them
now — the Lua API dump is unchanged, and so is the settings file.

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
| `emu.smooth_scroll_ahead` | (new, KIRBY-MERGE) | Emulation | — |
| `emu.smooth_scroll_view` | (new, KIRBY-MERGE) | Emulation | — |
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
- **A write from a script snaps on this very grid** since RO-2 part 3 — see §3 below, which
  replaces what RO-1 had done here. RO-1 had kept AMD-040 §3's own rounding for a Lua write (the
  30° grid first, so 15 gave 0); the owner found that rule absurd and RO-2 took it out.
- A script **read** gives back whatever the player chose, fine rung included.
- The harness `--bend` already took any angle, on or off the grid, and still does.
- No rendering code changed: the curvature maths takes degrees as a continuous `f32`
  (`zax-engine3d/src/space_bend.rs`) and the shader variant switch is still "all three angles zero
  or not".
- `scene.arc_center` is not concerned by the angles' grid; it has a grid of its own since RO-5 —
  see §7.

**Two consequences for R1 to weigh.**

1. The **libretro option** of each angle now publishes 23 values, where AMD-043 §6 writes 13. The
   libretro list is also what the console's advanced page steps by, so the two have to agree.
2. The **rung indices** of AMD-059 REQ-LUA-116 (`Setting.VisualStyleParameters`) shift for the
   three angles, the published list having grown from 13 entries to 23: rung 6 was 0° and is now
   −25°. Keeping the reported rungs on the 13-entry list was the alternative, and it was rejected
   because it would have made the log report a value the setting does not hold, which the
   requirement ("the rung each one holds now") forbids. The log and the setting stay consistent;
   the numbers move. The list still goes back *exactly*
   (`ScriptSettings::restore_visual_style_parameters`), and since RO-2 part 3 it does so through
   the one normalisation there is, rather than through a second one added to work around the 30°
   rule.

---

## 3. Divergence: a Lua write of a curvature angle, and the rounding of AMD-040 §3

**What the specification says.** AMD-040 §3 normalises a style parameter written from a script to
`a + n × step` with `n` the nearest integer, the **even** one when two are equally near. For the
three curvature angles of AMD-043 REQ-SCN-066 the step is 30°, so a write of 45 gives 60, 75 also
gives 60, and 15 gives **0**.

**What RO-1 did.** Kept that rule for a Lua write while giving the interface, the settings file
and the harness a 23-rung grid (§2 above) — two roundings on one key, the Lua one in
`zax_settings::styles::snap_write`.

**What Zaxnes does now** (owner decision, RO-2 part 3): a Lua write snaps on **the same grid as
the menu**, to the nearest rung, **ties away from zero**. There is once again exactly one grid
per setting, and `styles::snap_write` *is* `Domain::snap`.

| Written by a script | RO-1 (AMD-040 §3 on 30°) | Zaxnes since RO-2 |
|---|---|---|
| 45 | 60 | 60 |
| 75 | 60 | **90** |
| −45 | −60 | −60 |
| −75 | −60 | **−90** |
| 15 | 0 | **15** |
| 27.5 | 30 | 30 |
| 12.5 | 0 | **15** |
| every multiple of 30 | itself | itself |
| 500 / −500 | 180 / −180 | 180 / −180 |

**Why.** The owner finds the ties-to-even rule absurd, and it was the only reason two roundings
lived on one key. "Nearest rung, ties away from zero" is what a player reading the menu would
predict, and it is what the domain already did for everybody else.

**What is kept, exactly.**

- Every multiple of 30 is a rung of the grid and is left alone, so a profile written for 3dSen —
  which can only ever have written multiples of 30, those being all AMD-043 published — behaves
  **exactly** as it did. REQ-SCN-066's and REQ-LUA-114's own criterion (45 reads back 60) is
  unchanged, and its test passes with the value the amendment names.
- `spl.arc_center` is not an angle: it keeps its two rungs, 0 and 1.
- The twelve style parameters of AMD-055 REQ-SCN-079 are untouched — their domain is stepped, and
  for an evenly spaced grid the two rules already agreed.
- A script **read** is untouched (REQ-LUA-114), and so is the exact round trip of
  `Setting.VisualStyleParameters` (REQ-LUA-116). What changes there is that a script can now
  *reach* every one of the 23 rungs, where before it could only land on the 13 multiples of 30.
- No rendering code, no settings-file format, no libretro option and no Lua member changed.

**For R1 to weigh.** This is a divergence from AMD-040 §3 for the three curvature angles, and
only for them: the amendment's rounding is still the rule everywhere its step is uniform. The
alternative — amending REQ-SCN-066 back to 13 rungs — would undo §2, which the owner wants.

---

## 4. Divergence changed: `Setting.Msaa` now drives the anti-aliasing, within a rule

**What the specification says.** AMD-059 REQ-LUA-116 §34 makes `Setting.Msaa` a stored value with
the mapping `v ≤ 1 → 0`, `2 ≤ v ≤ 3 → 2`, `v ≥ 4 → 4`, persisted under `video.msaa_2_0`, and
**no effect** — divergence **(D)** of the amendment, whose reason it states: "dans la 2.0 il fixe
l'anticrénelage à la session suivante ; Zaxnes a son propre réglage (`video.anti_alias`, AMD-032)
qu'un script ne doit pas changer à l'insu de l'utilisateur."

**What Zaxnes does now** (owner decision, RO-2 part 4). The write still stores the normalised
value and a read still gives it back, exactly as the amendment specifies. On top of that it
**takes effect on `video.anti_alias`** — 0 → None, 2 or 4 → FFAA, which AMD-032 §2 makes MSAA 4×,
Zaxnes having no 2× — as a **session** value (REQ-PERS-010): in force until the end of the game,
the user's preference restored afterwards, never persisted by the script.

It is ignored for the anti-aliasing when any of these holds:

| Clause | Read from |
|---|---|
| the program shows the console interface: a handheld build (`PortMaster`, Android), `--handheld`, or a desktop opened with `--console` | `AppOptions`, carried to the session (`SessionRequest::console_interface`) and from there to `ScriptSettings` |
| the user's **global** `video.anti_alias` is above 0 | `SettingsModel::preference` — the layers under a game's options and under the session write |
| the game in play has a per-game anti-aliasing option (§5 below), **0 included** | `SettingsModel::has_game_value` |

**The rule is one-directional, and that is the point.** Zaxnes ships `video.anti_alias` at 1
(AMD-052 REQ-PERS-089), so clause 2 is the ordinary desktop case and the write is ignored there.
What the rule allows is therefore exactly one thing: a script may **add** anti-aliasing for a user
who has none, on a desktop, for one game, for the length of that game. It can never take a user's
own anti-aliasing away, never downgrade SMAA to FFAA, never cost a handheld a frame, and never
override a choice made for that very game. AMD-059's own acceptance criterion
(`Setting.Msaa = 3` → 2, `= 1` → 0, `= 8` → 4, `video.anti_alias` unchanged) therefore still
passes **unchanged**, on a default model, and its test is untouched.

**What is kept, exactly.** The stored key, the mapping, the read-back value, the persistence scope
and the Lua member are all as AMD-059 writes them; `goldens/lua-api.txt` is byte for byte the
same. Nothing new runs per frame: this is a setting read when it changes.

**For R1 to weigh.** Divergence (D) of AMD-059 §34 becomes a narrower one: the member is no longer
"sans effet", it is "sans effet sauf dans un cas où il ne peut rien enlever à l'utilisateur". The
asymmetry comes out of the owner's clause 2 read together with REQ-PERS-089's default of 1, and is
worth confirming: if the intent were "unless the user made an *explicit* choice" rather than
"unless the value is above 0", the clause would have to test the user layer instead, and a write
could then turn a default-on anti-aliasing off. The code follows the letter of the owner's rule.

---

## 5. The console option pages: one flat page of 56 rows becomes a tree

**What the specification says.** Nothing: the console (handheld) interface is Rust-only, an owner
feature with no requirement behind it (Q-036). REQ-UI-207 describes the desktop window only. This
section is therefore not a divergence from a text but the record of a layout the owner designed on
an HTML mockup and approved, kept here beside the window's so that the two can be read together.

**What Zaxnes did until RO-2.** Four flat lists and one screen: the main page (14 rows), its
Android variant (15, the touch-pad row after the controls), an *Optimizations* page (8), an
*Advanced options* page of **56 rows** in one scroll, and the overscan screen (9).

**What Zaxnes does since RO-3.** The 56-row page is a hub of six pages, two of which have a page
of their own, and a group title may head a run of rows inside a page. Nothing else about the
console changed: no setting was added, removed, renamed, or changed in value, default, domain,
persistence key or behaviour.

### The tree

```
options
 ├─ Language, Volume, Render scale, Shadows, Palette
 ├─ Overscan >                 the overscan screen, 9 rows, unchanged
 ├─ Region
 ├─ Optimizations >            9 rows: the 8 performance switches + emu.lua_smooth_gc
 ├─ Advanced options >
 │   ├─ Display >              Show FPS, VSync, Frame rate, Skip frame
 │   │                         [Resolution] Render scale, Dynamic resolution, Render method,
 │   │                                      Anti-aliasing
 │   │                         [Advanced]   GPU queue
 │   ├─ 3D rendering >         Visual style
 │   │   ├─ Enhanced effects > Voxel grid
 │   │   │                     [Reflections] Reflections, SSR shine, clarity, length, quality
 │   │   └─ Geometry (curvature) > Bend L-R, Bend T-B, Arc H, Arc centre, Bend 2D mode
 │   │                         Skybox, Mesh algorithm
 │   ├─ Light and shadows >    Light intensity, Backdrop lighting, Pixel shadows
 │   │                         [Ambient occlusion] AO, strength, radius, thickness, quality
 │   ├─ NES picture >          Remove sprite limit, Widescreen strict edges
 │   ├─ Camera >               Camera view, Field of view
 │   └─ Emulation >            Run-ahead, Game speed, Auto region, Region, Emulated menu button
 ├─ Controls >, RetroAchievements >
 └─ Profiler
```

The options of one **game** are the same tree reduced to what a game can hold: the main screen
keeps the volume, the render scale, the shadows, the palette, the overscan, the widescreen and the
region; *Optimisations* keeps eight of the nine (the smooth collector stays global); the hub has
four pages (*Display*, *3D rendering* with its two, *Light and shadows*, *Emulation*) and not six —
a game has nothing to set on *NES picture* or *Camera*.

The exact tree, both trees, page by page and row by row, is
`rust/goldens/handheld-option-tree.txt`, which replaces `handheld-advanced-rows.txt`. The tables
it is generated from are in `crates/zax-app/src/ui/handheld/options.rs` (`OptionsPage::rows`) and
`game_options.rs` (`page_rows`, `GAME_MAIN`).

### What moved, and that is all that moved

| Row | Before RO-3 | Since |
|---|---|---|
| Camera view | the main page | *Advanced options › Camera* |
| Show FPS | the main page | *Advanced options › Display* |
| `emu.lua_smooth_gc` | the flat advanced page | *Optimisations*, with the other performance switches |
| the other 55 of the flat page | one scroll | the six pages of the hub, by family |
| the widescreen of a game | next to last on its main screen | between the overscan and the region |

The *Advanced options* hub repeats nothing the other pages offer: the palette and the shadows are
main-page rows, the nine overscan settings the overscan screen's, and the seven performance
switches the Optimisations page's.

### Three things the tree adds, none of them a setting

- A **group title** is a non-selectable row: up and down skip it, it is drawn smaller
  (`set_window_font_scale(0.8)`) and dimmer than a value, and it records no pointer zone, so the
  mouse of the desktop's console mode cannot select what the pad cannot.
- On a **game's** screens, a row using the global value shows "Global: value" dimmed; a row with
  the game's own shows it plainly with a `*` marker in the colour RO-2 uses for an own value.
- On a **game's** screens, a row that opens a page shows how many own values are reachable under
  it ("Advanced options   2 own >"), and nothing when there are none. The count is computed from
  the game-options model when the page is drawn; a console page is drawn only while it is shown,
  so nothing of this runs while a game plays.

Four labels are new, in both languages — *Enhanced effects*, *Geometry (curvature)*, *Reflections*,
*Ambient occlusion* — and two texts: "Global: {0}" and "{0} own". The chevron is the ASCII `>` the
console rows already used and not the mockup's `›`: U+203A is outside the atlas's glyph ranges.

### The one reservation: "one place" is a rule about rows

The owner's rule reads "no setting other than `video.res_scale` is reachable from two places".
Read of settings it is false of the mockup itself; read of **rows** it is exactly true — no row
appears twice in the tree. What appears twice is a setting a *composite* row moves together with
others, where the mockup also asks for that setting's own row on a page:

| Composite row | Its settings | The rows of the same settings |
|---|---|---|
| Render scale (main page) | `video.res_scale`, `video.dynamic_resolution`, `video.render_method` | Display › Resolution, all three |
| Region (main page) | `emu.auto_tv_mode`, `emu.tv_mode` | Emulation, both |
| Frame cap (Optimisations) | `video.vsync` | Display |

Six settings, from three rows the mockup keeps where they were. The second and third couples come
out of the mockup's own *Display* and *Emulation* pages, so the one-sentence rule cannot be applied
literally without taking rows out of the mockup. `handheld_tree_tests.rs::TWO_PLACES` pins those
six exactly, so a seventh is a failure and not a drift. Raised with R1 in **Q-107 §6**.

### What is kept, exactly

- The 56 settings the console offers, same count and same keys
  (`ui::handheld::advanced::ROW_SETTINGS`), and the 30 rows a game can hold.
- The overscan screen and its nine rows, untouched.
- `TOUCH_ROWS`: the main page and the touch-pad row after the controls, as before.
- The **desktop options window** of §1: its nine tabs, `goldens/desktop-options-shown.txt` and
  `EXCLUDED` are not part of RO-3 and are unchanged.
- `goldens/lua-api.txt` and `SettingKeys.reference.txt`, identical.
- The handheld defaults (`ui/handheld/defaults.rs`): no setting is new, so there is no new
  handheld default to choose.
- The per-frame paths: a console page is drawn only while it is shown, and the desktop menu draws
  a submenu only while it is open.

### The desktop *Game options* menu follows the same tree

Its first level **is** the console main screen of a game, row for row and in its order; below it
come the *Optimisations* and *Advanced options* submenus, the latter holding *Display*,
*3D rendering* (with *Enhanced effects* and *Geometry*), *Light and shadows* and *Emulation*; the
RetroAchievements entry stays at the end, where RO-2 put it. The menu carries **no list of its
own**: it walks `GAME_MAIN` and `OptionsPage::game_rows`, which is what keeps the two interfaces
from drifting. RO-2's `GAME_DISPLAY_ROWS`, `GAME_3D_ROWS`, `GAME_CURVATURE_ROWS` and
`GAME_ADVANCED_ROWS` are gone with it.

---

## 6. Divergence: the anti-aliasing levels are labelled as the multisampling they are

**What the specification says.** AMD-032 §2 names the three levels of `video.anti_alias`
"None / FFAA / SMAA", and the same paragraph decides what they *do*: FFAA is rendered as MSAA 4×,
SMAA as MSAA 8×. Zaxnes implements neither of the two algorithms those acronyms stand for — FXAA-ish
and *Subpixel Morphological Anti-Aliasing* are post-processes, and multisampling is a different
thing altogether.

**What Zaxnes does now** (owner decision, RO-5 part 1): everywhere the user **reads** the setting,
the three levels are shown as what the renderer produces.

| Value | English | French |
|---|---|---|
| 0 | None | Aucun |
| 1 | **MSAA 4×** | **MSAA 4×** |
| 2 | **MSAA 8×** | **MSAA 8×** |

That is the desktop options window, the console global pages and game pages of §5, the desktop
*Game options* menu, and the tooltip. The tooltip is rewritten: the sample counts first, then the
sentence that **3dSen calls these levels FFAA and SMAA, and that those two names are what the
settings file and the libretro option hold**, Zaxnes drawing them as what they are. The old
sentence ("the two names are the ones the specification gives the two levels") is gone — true and
useless.

**Why.** A user reading "SMAA" believes they are choosing a morphological post-process; they get
eight samples per pixel. The label should say what is delivered.

**What is kept, exactly.**

- The stored **values** 0, 1, 2 and the default (1 on the desktop, 0 on a handheld).
- The **settings file**, which holds the domain label: `"anti_alias": "FFAA"`.
- The **3dSen settings import**, field 32.
- The **Lua API**: `Video.AntiAlias` reads and writes the same numbers, `Setting.Msaa` keeps 0/2/4
  and the rule of §4; `goldens/lua-api.txt` is unchanged byte for byte.
- The row's place in the window and in the tree, and `EXCLUDED`.
- No code at all: the change is **four string-table entries** (`option.FFAA` and `option.SMAA` in
  both languages, and the rewritten tooltip in both). All four interfaces read the value through
  `ui::text::label_of_option`, which looks a domain label up as `option.<label>` and falls back to
  the label itself when there is no entry — which is exactly why the user read "FFAA".

**The reservation: libretro keeps the old names.** A `CoreOption` carries one `values: Vec<String>`
per value, which is at once what RetroArch persists in its `.opt` file and what it displays; this
port does not expose `retro_core_option_v2`'s separate display label. Changing the string would
therefore reset a user's persisted choice, so **the libretro values stay `None` / `FFAA` / `SMAA`**
and the libretro core is the one place the two acronyms are still read. Giving libretro a display
label of its own is a lot of its own (`core_options.rs` and the libretro shim). Recorded in
Q-107 §7.

---

## 7. Divergence: `scene.arc_center` takes the quarters between its two heights

**What the specification says.** AMD-043 REQ-SCN-066 makes the arc centre an integer with two
values — 0 the screen centre, 1 the bottom edge of the reference frame, that is the pivot
`y = −120 · s` relative to `C` — and §6 publishes "one option, 2 values"
(`Screen Center` / `Screen Edge`).

**What Zaxnes does now** (owner decision, RO-5 part 2): **five** rungs, an intermediate one placing
the pivot at the matching linear interpolation between the two heights, `pivot = −120 · s · c`.

| Value | English | French | Domain label |
|---|---|---|---|
| 0 | Screen centre | Centre de l'écran | `Screen Center` |
| 0.25 | 25 % | 25 % | `25%` |
| 0.5 | 50 % | 50 % | `50%` |
| 0.75 | 75 % | 75 % | `75%` |
| 1 | Screen edge | Bord de l'écran | `Screen Edge` |

**Why.** Two heights are a coarse choice for a transformation that is continuous in the pivot's
height: between an arc pivoting at the centre of the image and one pivoting eight units below its
edge there was nothing.

**Where it reaches the picture.** The fraction arrives at the GPU through the uniform `uBendPivot`
**that was already there**, computed in `SpaceBend::new` when a curvature setting changes. The
three paths that read it — the scene pass, the shadow pass and the 2D-mode rectangle — therefore
get the interpolation with **no shader change and nothing new per frame**.

**What is kept, exactly.**

- The two heights of REQ-SCN-066: at 0 and at 1 the arithmetic is **bit-exact** (`x · 1 = x`, and
  the near end is written `0.0` rather than multiplied, a product by zero of a negative number
  being `−0.0`). A capture at either end is pixel-identical to the one before this lot, by
  construction and not by measurement.
- The thirteen vectors of REQ-SCN-067 and the ordering clause of REQ-SCN-068, both stated for the
  two heights.
- The **settings file**, for the two values that existed: `json::float_json` writes the shortest
  text that reads back as the same float, which is `0` and `1` — exactly what an `Int` wrote — and
  the `Float` branch of `from_json` reads any JSON number. A settings file, a `games.json` or a
  profile written before this lot loads unchanged. `SettingKeys.reference.txt` holds keys only and
  does not change.
- **3dSen compatibility**: a Lua write of 0 or of 1 is a rung and goes through untouched; a 3dSen
  2.0 profile can write only those two; a Lua read of a setting holding 0 or 1 gives the same
  number and the same Lua type as before. A write off the grid snaps to the nearest quarter, ties
  away from zero — the rule §3 gives the three angles. A read of a setting the player put at 0.5
  gives 0.5: `spl.arc_center` is reachable only as a style parameter (REQ-LUA-114), and
  `visual_style_parameter` has always returned an `f64`, so **no member changes type** and
  `goldens/lua-api.txt` is unchanged.
- No setting added or removed, no new key, no row moved: `goldens/desktop-options-shown.txt` and
  `goldens/handheld-option-tree.txt` hold **keys**, never values, so five rungs read there exactly
  as two did.
- The handheld defaults: the setting keeps its own default of 0 on both platforms, which costs
  nothing, the pivot being read only while the arc is active.

**Three consequences for R1 to weigh.**

1. The key's **type goes from integer to real**. `Domain::coerce` rebuilds an `Int` value with
   `truncate_to_i32`, so an integer-kinded setting cannot hold 0.25 at all. SPEC-09 §4.1 gives the
   key the type "entier"; that is the divergence. The file format of the key is unchanged for the
   two values that existed (above).
2. The domain is a **grid, not a step**. The owner asks for the nearest rung with ties **away from
   zero**, which is `Domain::grid` and not `Domain::stepped` — the latter rounds ties to the even
   rung, so 0.125 would give 0 and 0.375 would give 0.5. This is letter for letter the rule §3
   gives the three angles, so the branch keeps one rule.
3. The **libretro option** publishes 5 values where AMD-043 §6 writes 2, and the **rung indices**
   of AMD-059 REQ-LUA-116 shift for this one parameter (rung 1 was 1.0 and is now 0.25) — the same
   consequence, for the same reason, as §2's second one. No persisted libretro choice is lost:
   `core_options::try_apply` matches a value by **label**, and the two the amendment names are
   still there, at the two ends. The round trip REQ-LUA-116 promises is still exact.

**A smaller reservation.** The window decides on a drop-down from "the domain publishes rungs
**and** the kind is enumerated or integer". Adding "or real" would turn `audio.volume`,
`camera.fov` and `light.intensity` — stepped reals whose rungs are labelled with their own numbers
— into twenty-entry drop-downs. Hence an explicit list, `ui::options::LABELLED_FRACTIONS`, holding
only `scene.arc_center` today, read by the window *and* by the console page so the two cannot
disagree. Recorded in Q-107 §8, which asks R1 whether a declarative rule (a marker on the
descriptor) would be preferable.

**The harness.** `--bend`'s fourth part accepted the literal strings `"0"` and `"1"` and nothing
else; it now parses any number and refuses anything outside `[0, 1]`, snapping included none — so a
capture campaign can compare pivots the menu does not offer, as it already could with the angles.
