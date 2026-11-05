# Example: the FlappyZaxNes profile

FlappyZaxNes is the demo game shipped with Zaxnes (GPL v3, after Flappy NES by filipemd); its 3D
profile is in the library as "FlappyZaxNes". This folder holds the description it is built from,
as a complete example of `profile.json`:

* `profile.json`: 290 tiles and 230 patterns: ground slab, round pipes (vertical cylinders),
  bushes, city, clouds at their depths, the bird one pattern per pose, all on layer L3 set to the
  centre of the play plane (`layers`, `template`).
* `profile3d.py`: the script that wrote it. FlappyZaxNes's sources were at hand, so it reads the
  game's CHR and screen data directly; for a game without sources the same work is done from
  frame dumps (`docs/CLASSIFY.md`), but the patterns it writes are the same kind.

Not runnable as is: its `template` (`empty.3dn`, a profile used only for its settings) is not
part of this folder; remove the `template` and `layers` lines to build it. It has no Lua: see
`docs/RECIPES.md` for scripts.
