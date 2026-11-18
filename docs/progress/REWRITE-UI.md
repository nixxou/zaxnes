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

## Step 16 (2026-11-18)

Owner: reflections off by default in every mode (still offered from Balanced on); voxel grid and emu.lua_smooth_gc (on by default) no longer managed by the presets; emu.pause_3d_in_2d stays in the presets, on and offered in the two fast modes, off and hidden elsewhere (default unchanged: some scripts lose frame hooks in 2D). Real line breaks in the preset strings turned back into 
. Freed target/debug (13 GB). zax-app lib, desktop_options_golden, pause_3d and handheld_tree tests pass.

## Step 17 (2026-11-18)

A Default button on every tab of the new window, Essentials included. It does not change the chosen preset: what the tab shows (in the mode in force) goes back to the chosen preset's value where the preset manages the setting, to the setting's own default elsewhere; on Essentials it writes the chosen preset again. New tab Widescreen & Hacks: simple = widescreen, extra width, buttons opening the widescreen / hacks windows of the game in play; advanced adds the advanced mode, strict edges and built-in patches. Widescreen left Essentials. draw_tab_body split (draw_tab_entries) so that the new window draws its own reset. zax-app lib and desktop_options_golden tests pass.

## Step 18 (2026-11-18)

Widescreen & Hacks tab = defaults of every game with a widescreen patch (owner): widescreen on by default + ratio (video.widescreen), extra width, advanced mode, and common defaults of the pack options no_slowdown, invincible, level_select and Kirby's margin_enemies / margin_freeze / margin_freeze_tint (meant for other games). Stored in emu.hack_options as */key entries; a pack reads its own entry, then the common one, then its default. Default on that tab drops the */ entries only. NES picture (new window) lost its widescreen group to this tab. Simple mode: widescreen + ratio, advanced mode, no slowdown, game buttons.

## Step 19 (2026-11-18)

Console options get the simple / advanced logic (owner: "fais-moi une proposition au mieux").
- ui.options_advanced and ui.options_preset: new settings shared by the window and the console (saved).
- ui/presets.rs: the presets out of options_new.rs, shared (choose, chosen, matches, reset_with).
- Console simple main page: mode, preset, preset page (rows the preset proposes), language, volume, widescreen on/off, region, simple pages Display / NES picture / 3D / Audio / Emulation (the window's simple tabs), controls, achievements, reset.
- Advanced main page: mode, preset, preset page, then the RO-3 tree plus a Widescreen & Hacks page (default ratio, extra width, advanced mode, common hack defaults, strict edges, built-in patches), reset. Strict edges left NES picture for it.
- A reset row ends every global page: preset values for what the preset manages, own defaults elsewhere; on the preset page it writes the preset again.
- Handheld defaults: the fastest preset, plus the handheld list; automatic resolution kept off (earlier owner decision).
- PC simple Widescreen tab: on/off only.

## Step 20 (2026-11-18)

Console lists get a scrollbar on the left when they are longer than the screen (owner): ROM list, profiles, controls, save states, achievements and every options page. Dragging it (mouse, or a finger on Android), the wheel and a finger sliding over a list scroll the view without moving the selection; a press of the pad brings the list back round its selection; a click or tap on a row selects it even with the selection scrolled out of sight (rows carry their rank, group titles not counted). Touch: TouchPad::free_point feeds the drag.

## Step 21 (2026-11-18)

Console: Page Up / Page Down (new pad buttons) move a screenful of rows, stopping at the list ends. The keys printing A, B, X, Y in the system layout (QWERTY, AZERTY...) press those buttons (Key::printed_letter via SDL keycodes); the letter keys press nothing while the keyboard screen types.

## Step 22 (2026-11-18)

Shader families (owner, proposal researched by a Fable 5.2 sub-agent from the slang-shaders sources): ui/shader_looks.rs, 8 families (CRT Guest, Mega Bezel, CRT light, CRT classic, CRT Royale, composite signal, LCD, smoothing) with picks that choose the preset file (TV frame, quality, signal, model, method) and steps that set parameters (curvature, scanlines, mask, brightness, reflections...), the same names in every family; a step of no parameter is the preset's own value, a value set by hand reads Custom. PC: Style menu (families + every preset of the pack), pre-filter, sub-options, and the full parameter editor folded under them (shader and pre-filter). Console: the slot row steps through the families, rows of sub-options under each slot (dynamic rows). Guest switches to its HD version behind a pre-filter. Step values are a first proposal, to tune by eye.

## Step 23 (2026-11-18)

Signal options (owner, from the BeesNES study): the composite switch became a small family.
- Core: `zax-nes/src/tv.rs` (television models from BeesNES's measured tubes: CRT gamma, primaries, gamut mapping; TV knobs), `composite.rs` DecoderOptions (signal composite / S-Video / RF, sharpness, 3-line comb filter, RF snow and ghost, knobs and tube on the decoded colour), `decode_row_with` (rows around for the comb).
- Palette: `zax_engine3d::palette::set_tv_look`, the 64 shown colours worked out once per change (atomics), so the television colours the 2D palette and the 3D view the same way.
- Settings: video.signal_kind, tv_model, decoder_sharpness, comb_filter (on), frozen_dot_crawl, rf_noise, rf_ghost, tv_hue, tv_saturation, tv_contrast, tv_brightness. Presets: the signal off up to Balanced, proposed on from Faithful.
- PC new window, NES picture: simple = Signal (off / composite / S-Video / RF, one list over two settings), Television, Palette, overscan; advanced adds emphasis in 3D, Decoder group, TV knobs with a Neutral button. Rows with no effect are greyed with their reason (signal off, comb with S-Video, RF only).
- Console: Signal row (cycles the four), Television; advanced NES picture page with every row.
- Recorded as FIDELITY-LEADS H4. Note: the comb is 3 lines, not 2 as in the sketch (the 2C02's phase moves a third of a cycle a line, so it is three lines that cancel).

