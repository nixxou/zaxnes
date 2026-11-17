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
