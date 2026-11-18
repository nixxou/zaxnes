# REWRITE-UI: progress log

Branch `rewrite-ui`, worktree `wt-rewrite-ui`, from `feat/fast-core` c7eef08c (2026-11-17).

Goal (owner, 2026-11-17): a new PC options window next to the old one (a button in each goes to
the other): a simple mode (an Essentials tab plus Controllers, the other tabs hidden) and an
advanced mode (every tab); on Essentials a performance <-> fidelity slider in five presets, a
box saying what the preset turns on, and the preset's own checkboxes. Fast core and Fidelity+
switched without reloading. Only "next game" / "next start" are labelled. Mock-up:
claude.ai artifact "Zaxnes options window".

Inventory of the settings that wait (catalogue `SettingEffect`): restart = `video.backend`
(and the window rect); next game = `emu.lua_optimized`, `emu.lua_experimental`,
`layer.default_layer`; next ROM load = 34 (power-on state, revisions, engine, cartridge DB,
hardware defects, VS DIPs, ...). Of these the fast core and Fidelity+ are now immediate.

## Steps

* 1. Branch and worktree.
* 2. `emu.fast_core` and `emu.fidelity_plus` immediate (host `on_setting_changed`); the fast
  core now runs the real-time slices too (they bypassed it, so with the default "real-time
  pad" the fast core did nothing): `NesBus::fast_limit` holds its counting to the slice's end.
  `fast_1_real_time_slices_match`: 320 ROMs, 600 frames in 144 Hz slices, identical.
* 3-4. Presets and the new window (`ui/zax_ui/options_new.rs`): simple mode = Essentials +
  Controllers, advanced mode = every tab of the classic window as well; the five-step slider
  writes its preset's values, the box says what it turns on, the preset's check boxes are the
  real settings (amber "at the next game" / "at the next start" when one waits); "(modified)"
  when the settings left the preset. The classic window stays: a button in each opens the other.
  The mode and the last preset are not saved yet (window state). Catalogue test: `emu.fast_core`
  added to the list of additions (also on feat/fast-core).
* 5. Owner's selection (2026-11-17): the simple mode also shows Display (fullscreen, vsync,
  anti-aliasing, frame skip), NES picture (palette, composite signal, overscan group), Audio
  (volume, console audio profile), Emulation (run-ahead, smooth scrolling, Rainbow network,
  console frame rate, real-time pad) and Achievements whole. The advanced mode's 3D render tab
  takes Light and Camera (FOV) under it; Interface only in advanced. Classic window unchanged.
* 6. No music in the interface (owner): the mixer's menu music is never enabled, its track never decoded; the sound effects stay. The setting `audio.bg_music` stays in the model, without effect.
* 7. Display (simple) adds the graphics API (amber "at the next start") and a "Configure the
  shaders..." button. No welcome window (owner): a game opens from File.
* 8. File menu (owner): Favorite, Favorites only, Delete the game, Create the cover, Import a
  cover leave the PC menu (they served a game library the PC no longer shows; the commands
  stay). "Load a state" / "Save the state": hovered, the slots; clicked, the slots with their
  pictures (the "...Ctrl+L/S" duplicates removed; the shortcuts stay). `ui_round_one_fix_tests`
  updated (no DeleteGame entry). **App tests not rerun for steps 7-8: G: full (735 MB), a debug
  test build needs about 16 GB.**
* 9. Owner (2026-11-17): favourites, covers and the deletion of a game are gone, console and PC.
  Commands `favorite.toggle`, `library.filter_favorites`, `file.delete_game`,
  `file.create_game_art`, `file.import_cover` removed with their handlers; the console list's L2
  and its marker, the PC chooser's filter, marker and button, the cover browser, the 2D Mode's
  automatic cover, the `covers` folder's creation; `ui.sort_option` loses « Favorites first » (a
  saved 2 falls back to A-Z). Kept: the `favorite`/`cover` fields of `library.json` (read, ignored)
  and `GameCatalog::delete_game`/`game_art::delete_cover` (no caller left in the app). Tests:
  command_model, resume_point, round_one_fix, native_dialog, ui_round_one_fix, menu_bar, ui_flow,
  desktop_options_golden, handheld_tree and the lib all pass (release, one by one: G: full).
* 10. Fidelity+ with the widescreen overclock (owner's savestate, black bands): no edges while the extension stalls the PPU (`edges_on_for`); FIDELITY-LEADS I6. fidplus test files pass (equivalence 764 s).
* 11. Simple mode shows the 3D render tab: visual style, reflections, the Geometry group, light intensity, shadows, field of view (owner).
* 12. Presets from the owner's review (docs/design/PRESETS.csv): values of any kind (run-ahead, pipeline, resolution, shadows...), exact rate offered unticked in Faithful, OAMADDR write corruption ticked in Most faithful, catch-up last 3D on up to Balanced, resolution offered (native) and experimental Lua offered in Fastest.
* 13. Every setting a mode sets is shown under the slider (owner); the CSV now reads the presets from the code.
* 14. Every preset writes the whole managed set (18 settings), each value proposed (shown) or set and hidden, so a mode puts back what another changed (owner's per-mode choices). Essentials lighter: the slider and the widescreen; language, ROM folder and FPS go to a light Interface tab in the simple mode.

## Step 15 (2026-11-18)

The presets also manage render.visual_style and six Enhanced-style parameters (reflections, ssr_quality, ao, ao_quality, pixel_shadow, voxel_grid): Classic in the two fast modes, Enhanced from Balanced on with more effects towards the faithful end. Voxel grid stays Off and hidden (it is a look, not a quality). Tests not run (464 MB free on G:). Commit 637d151e.
