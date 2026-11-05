# Before saying it is done

For a level, then for the whole profile:

1. **Every screen of the part**, not only the ones you worked on: with a movie, seek by steps
   of a few hundred frames, then around each transition (a door, a boss, a scroll); without
   one, ask the user to play through the part again while you look. Look in 3D, and from the side (`zax-harness render ... --angles 0,89`) where depth matters.
2. **The four views** of the tool box: `3D`, `2D` (the game's picture: does the 3D keep its
   meaning?), `3S` (as 3dSen reads the script: `#zax` code left out) and `HH` (as a handheld
   reads it).
3. **The log**: `get_log` without any Lua error or warning (a script that errors is turned off
   for the rest of the game).
4. **Animated and rare tiles**: every step of each animation, the tiles of rare screens (title,
   game over, ending, boss) have their look.
5. **Speed**: shapes added with `CreateNewShape` cost on every frame; with the FPS counter on
   (Video menu) the game keeps its speed. Heavy effects get a lighter `#zax handheld` reading.
6. **Clean up**: the `placeholder` pattern removed, debug `print` removed, `title`, `author`,
   `description` of `profile.json` filled in.
7. **NOTES.md** up to date: what is done, the depth plan, the user's choices, what is left.

Then tell the user what was checked and what was not.
