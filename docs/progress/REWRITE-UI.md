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
