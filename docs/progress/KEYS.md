# Lot KEYS — RetroArch keyboard layout

Branch `feat/keys`, from `gamehash` (b37ccfce), in the `wt-gamehash` worktree (G: too full for a
second target directory). Owner request, 2026-11-25.

## Decisions (owner)

- A setting `ui.key_layout` (Interface tab): **RetroArch** (default) or **Zaxnes** (the old keys).
- The pad keys do not move (arrows, X = A, Z = B, Space = Select, Enter = Start).
- Camera on the keypad: 4/6 turn, 8/2 tilt, 9/3 zoom, 5 default view. Without a keypad, always:
  right Ctrl + arrows turn and tilt (the game does not see the arrows meanwhile), Page Up / Page
  Down zoom, Home default view.
- Fast forward held: L, and Tab (Tab in both layouts).
- Current slot: F2 saves it, F4 loads it, F6/F7 choose it. Shift+F2 / Shift+F4 open the save /
  load screens (Ctrl+S / Ctrl+L kept). No key per slot in the RetroArch layout.
- A notification for a slot change, and for a save or load the player asked for; none for the
  program's own saves (movies, auto save, resume point): only the commands say so
  (`save_slot_announced`, `load_slot_announced`), `save_to_slot` stays silent.
- Escape twice within 2 s quits (`ui.escape_quits`, on by default); the first press warns. A
  modal window closes first, a dialogue swallows it. No row and no effect on the web page or
  Android (`options::escape_can_quit`).
- F1 is the console mode (RetroArch's menu); Home leaves it only in the Zaxnes layout.

## Also in the RetroArch layout

H reset, F full screen, F3 FPS counter, F9 mute, keypad + / - volume, R rewind.

## Migration

`ui.key_layout_applied` (no row) records the layout player one's keys were brought to; 0 for every
set saved before the lot. Each frame `follow_key_layout` puts the layout in force and, when it
differs, moves every action still on its default key of the old layout to the new one
(`default_bindings::bring_to_layout`); keys the player chose are kept.

## Steps

1. Done: layout table, keys, current slot, notifications, Escape, options rows, tests
   (`keys_layout_tests.rs`), goldens and OPTIONS-LAYOUT.
2. Done (2026-11-26): O records a movie, F8 saves a picture of the game in
   `<user data>/screenshots/<rom>-<date>-<time>.png`, M / N go round the shader presets (the
   families of `shader_looks`) of the display mode on screen with "none" among them, E held
   halves the speed. Also: the GoodSets' `zz_` / `z_` ignored by the name rule, the FPS counter
   under the console's title band.
3. Left: cheats (lot CHEATS). Owner: no fast forward toggle on Space.
