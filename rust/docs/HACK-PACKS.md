# Hack packs

Status: Mega Man 6 converted, see section 7. Branch `hack-rewrite`.

## 1. Why

Every advanced-mode hack is shipped today as a BPS patch applied to the player's ROM in memory
(`builtin_patches.rs`). The patch carries two things:

1. **The hack's code**, assembled into room it takes from the ROM: free space in a bank (the
   Super Mario Bros. 3 hack in the padding before a DMC sample), or a PRG bank added to the game
   (Mega Man 1, 3 to 6, Metroid: the PRG-ROM grows by 16 KiB and the fixed banks are copied to the
   new end, as MMC3 fixes the last two).
2. **The hooks**: a few bytes of the game's code replaced by a `JSR`/`JMP` into that code
   (17 for Mega Man 6), each checked against the bytes it replaces when the hack is built.

A ROM that differs from the one the patch was made for (a translation, a bug-fix patch, another
revision) is refused: the patch is matched by the CRC-32 of the ROM's data, and its free room or
its hooks may be taken by the other patch.

A hack pack keeps the same two things apart from the ROM: the code lives in memory of the
extension's own, the hooks are a table the emulator applies and checks itself. The ROM keeps its
size and its free room; a ROM whose hooked code is unchanged takes the pack whatever else differs.

## 2. What a pack holds

| Part | Contents |
|---|---|
| Header | `ZXPK`, format version, the pack's name (as `BuiltinPatch::name` today), its version (a hash of the rest: the "hack version" of savestates and movies) |
| Code | the hack's code image, up to 32 KiB, in 256-byte pages; assembled for `$5800` as today |
| Hooks | the list below |
| Boot | the boot window (first page shown at power-on, whether it covers `$6000-$7FFF`), the idle loop (`BuiltinPatch::boot_window`, `boot_window_keeps_prg_ram`, `idle_loop` as today) |

The game's name stays in `BuiltinPatch`, which points at a pack instead of a BPS.

### Components

A pack is one or more **components**, each with its own hooks (and code pages), each turned on
apart from the others:

| Component | Today | In a pack |
|---|---|---|
| Widescreen (advanced mode) | the patch, applied with the advanced mode | its hooks and code, on with the advanced mode |
| Testing aids: "Invincible", "Level select" | options of the patch (`$500A` bits the hack's code reads): only with the advanced mode | components of their own, on or off alone: with the plain widescreen, without widescreen at all, or with the advanced mode |
| "No slowdown" | an option of the patch (the overclock it sets) | stays the widescreen's: it pays for its work |

An aid is a few hooks of its own (the damage test, the stage start) and, when it needs code, a
few bytes of the code memory: it does not use the widescreen's code, nor its variables. Two
components never hook the same bytes (checked when the pack is built); where an aid and the
widescreen would both need the same place, the aid takes a hook of its own just before or after
it. The options menu shows each component as a switch, on or off whatever the others are.

Each hook:

| Field | Size | Meaning |
|---|---|---|
| bank | u8 | the game's 8 KiB PRG bank the hooked code is in (the fixed ones too) |
| offset | u16 | its offset in that bank |
| original | u8 count + bytes | the bytes the game has there: checked before anything is applied |
| new | bytes, as many | what the CPU reads there instead (`JSR wide_place / BCS $E269`, NOPs) |
| context | u8 count + bytes, optional | bytes around the hook that identify the place (section 4) |

Each hook belongs to one component. Built by the hack's `tools/build.py` from its `put()` list,
which already holds every field but the context (Mega Man 6's damage hook, `$8F72` in bank
`$38`, moves to the "Invincible" component); `make_bps.py` gives way to `make_pack.py`.

## 3. How the emulator applies it

* **Code.** The pack's code is a memory of the extension, not of the cartridge. The PRG window
  (`$5008` bit 7 on, `$5009` the first page) shows it at `$5800-$5FFF` (and on to `$7FFF` with
  `$5007` bit 4) instead of a bank of PRG-ROM; bits 0-6 of `$5008` pick a 16 KiB part of it. With
  no pack the window shows PRG-ROM as today: the hacks not yet converted are not touched.
* **Hooks.** At load the emulator checks every hook's original bytes in the player's PRG-ROM and
  writes the new ones into its in-memory copy, as the BPS does today; nothing is written to the
  ROM file. If one hook does not match, no hook is applied and the pack is refused: the hooks of
  a hack depend on each other (half a hack can leave the game in a state it was never made for).
  The ROM then runs as the plain widescreen (margin decoder only), and the log names the hook.
* **Identification.** A pack is offered for a ROM when its hooks all match, not by the ROM's CRC:
  the CRC stays the fast path for the ROM the hack was made for.
* **Savestates, with or without the pack.** The ROM keeps its size and layout (only the hooks'
  bytes differ, in memory, and a state carries the image as the host gave it): the mapper's
  state, the RAM and the PPU mean the same with or without the pack, and the hack's variables
  live in the extension's memory, not the game's. A state is therefore taken where none of the
  hack's code runs: the core waits for the game's wait for the next frame (`idle_loop`, known for
  every hack: Mega Man 6's `$C581`), at most a frame, so that no return address into the hack's
  code is on the stack. Such a state restores with the pack or without it: without, the
  extension's state is left out and the game drops what only the hack kept (enemies in the
  margins, off its screen); with, the hack starts from its empty state (its reserve empty, its
  pointers taken from the game's), as after a load today. The core's check that a state was made
  with or without the extension (`state_fits_builtin_patches`) gives way to this one. Movies stay
  bound to their pack's version: any difference of behaviour makes them drift.

## 4. Moved code (later step)

A translation may move code: the same instructions at another offset. With a context (the bytes
around the hook), the emulator may look for the unique place in the bank where they are and
apply the hook there. Two limits:

* the hack's code jumps back into the game at fixed addresses (`JMP $E27A` at the end of
  `wide_hide_rest`, `LIST_WALK = $813A`): those would need the same search and a relocation of the
  hack's code, which a first version does not do;
* the margin decoders read the game's RAM and its data in PRG-ROM at fixed places (Mega Man 6's
  stage lists in bank `$3A`, the controllers' settings in bank `$39`): a translation seldom moves
  them, nothing guarantees it.

First version: hooks at their exact place only (the ROM's code unchanged where the hack hooks,
whatever else differs). The search comes after, if a real translated ROM needs it.

## 5. First game: Mega Man 6

* Its code (now 10 KiB of a 16 KiB bank added to the game) becomes the pack's code; the window
  registers it already uses do not change.
* Its 17 hooks are `build.py`'s `put()` list as it is.
* The PRG-ROM stays 512 KiB: no bank added, no fixed bank copied to the end.
* Check: the same movies (`G:\3DSen\wide-test-megaman\movies-mm6`) replay the same with the
  pack as with the BPS, frame for frame (RAM traces compared with `--trace-ram`).

Then the other hacks one at a time, each checked the same way. Zelda II stays a BPS: its hack is a
rewritten disassembly, not hooks.

## 6. To decide

1. Refuse the whole pack when one hook does not match (proposed), or apply the others?
2. The code memory's size: 32 KiB (two 16 KiB parts for `$5008`) is enough for every hack today
   (the largest, Mega Man 6, uses 10 KiB).
3. Packs shipped inside the emulator as the BPS are today (proposed), or also loadable from a
   file next to the ROM (a "widescreen pack" for a ROM Zaxnes does not know)?
4. Keep the BPS of each converted hack for a while, to compare, or drop it at once?
5. The aids without any widescreen: shown in the same options menu (proposed), or apart?
6. States taken at the game's wait for the next frame only (proposed, at most a frame late), or
   at once as today, compatible only with the same pack setting?

## 7. Status

Decided (owner): every proposed answer above. The aids are components of their own, independent
of the advanced mode; a state is meant to restore with or without the pack.

Done, branch `hack-rewrite`:

* the format and its reader (`zax-nes/src/hack_pack.rs`), the core applying it
  (`WIDESCREEN-EXTENSION.md` 7.1);
* Mega Man 6 as a pack (`megaman6-usa-widescreen.zxpk`, 4199 bytes, 3840 of code;
  `mm6wide/tools/build.py --pack`). Checked: 7 movies (about 60,000 frames, stages 0, 1, 5 and 6)
  give the same work RAM every frame with the BPS and with the pack, the stack page aside (the
  damage hook is a `JMP` in the pack's "invincible" component, a `JSR` in the BPS). The
  "Invincible" option works without the advanced mode (3000 frames of stage 5: 15 hits taken
  without it, none with it);
* MMC3 recomputes its bank windows after a restore: a state of the BPS build (68 banks) restores
  on the pack (64).

* every other hack converted (`WIDESCREEN-EXTENSION.md` 7.1 lists the kinds): Mega Man,
  2, 3, 4 (one pack for both builds) and 5, Super Mario Bros., Super Mario Bros. 3, Castlevania
  (both builds), Adventure Island, Excitebike, Final Fantasy, Ninja Gaiden III, Metal Force,
  Blaster Master, Metroid, Zelda II. Each checked by RAM traces against its BPS: movies where
  there were some, otherwise a few thousand frames from power-on with scripted inputs; the
  same RAM every frame, but where the BPS's invincibility routine ran at every touch with the
  option off and the pack runs the game's own code: there a few cycles of drift (Mega Man 2 to
  5: where the raster IRQ interrupts the main code), the same with the BPS built without that
  hook (`WIDE_SKIP`). The owner played Mega Man, 3 to 6 on the packs;
* Zelda II rebuilt as hooks and code (`z2pack`), from its fork of the disassembly: checked
  against the pack of the fork's build over 12000 frames (a new game, a palace, the world map),
  the same RAM but for a few transient bytes from its shorter NMI;
* the six hacks that had their code in the game's free space (Mega Man 2, Super Mario Bros. 3,
  Castlevania, both builds, Final Fantasy, Ninja Gaiden III, Metal Force) moved to the
  extension's memory: their source assembled for `$5800` (`tools/build.py --pack`), without the
  reset hook and the NMI's unlock, the boot window `$80`/0 keeping `$6000`-`$7FFF` for the game.
  Checked against their free-space packs, 6000 frames from power-on with scripted inputs each:
  the same RAM every frame for Mega Man 2, Castlevania and Final Fantasy; transient bytes for
  Super Mario Bros. 3 (temporaries over 35 frames, from its shorter NMI) and Ninja Gaiden III
  (three scratch bytes, `$30`-`$32`); Metal Force one frame behind from power-on (its NMI counter
  `$034D` one less, without the old reset hook's work), which only changes its sprite flicker;

* the hacks screen: by the ROM's CRC first, then by the packs' signatures; a ROM that has them
  (another revision, a ROM hack leaving the hooked code alone) gets the hack, its options and
  the margin decoder without forcing. Checked on a Mega Man 6 ROM with one byte changed outside
  the hooks (CRC `D9F52C71`): the pack applies.

Still to do:

* states taken at the game's wait for the next frame (`idle_loop`) and restored with or without
  the pack (question 6): the application still classes a state by its patch mode (section 8.3 of
  `WIDESCREEN-EXTENSION.md`);
* the aids that need the hack's code (the level selectors, some invincibilities) apart from
  the widescreen: the code would have to be in the window without the hack running;
* the BPS files are still in `patches/`, no longer referenced: to remove once the packs are
  validated.

Format 2 and the signatures (`exec-zone` work, phase 3; `WIDESCREEN-EXTENSION.md` 7.1):

* the format: region, mappers, hooks placed by 16 KiB bank (the last counted from the end),
  groups with the earlier groups they require, checks of the game's code the hack relies on;
  each component judged apart, each group on its own; format 1 still read. The shared builder is
  `tools/hackpack.py` (a hack's build imports it from `ZAXNES_TOOLS`); `zax-harness packs <dir>`
  tells how the built-in packs apply to each ROM of a folder;
* Zelda II converted (`z2pack`): NTSC, MMC1 or MMC3; "widescreen" in groups "core", "enemies",
  "edge spawns" (needs "enemies") and "world map", and the aid "invincible". On the USA ROM the
  same RAM as before over 12000 frames (margins 0 and 128). Against a set of 661 Zelda II ROMs
  (dumps, translations, ROM hacks): 598 take the whole pack (585 did with format 1: the Korean
  translation, 256 KiB with its fixed bank at the end, now finds its hooks), 27 all but the world
  map (Winter Solstice, z22 and their translations change its code), 1 the aid only (a European
  dump whose header says NTSC: its code differs but for the aid's bytes), 35 none (the European
  versions, the MMC5 and mapper 19 conversions, Korean ROMs a 128 KiB hack was put over). Whether
  each one runs right is phase 4's;
* the region is the header's: a dump whose header says otherwise than its game passes on its
  bytes alone (no database of the games' regions in the core);
* the signature search takes the pack that applies best (its widescreen, then the most groups):
  two builds of a game may share an aid's bytes but not the widescreen's;
* Zelda II's European builds: PAL packs from the same source (`z2pack`, the game's moved
  addresses per build), one for Europe and its Rev 1, one for the Rev 2, with entries of their
  own (`ZELDA2_EUROPE`, `_REV1`, `_REV2`). On the 661 ROMs: 606 take a whole pack, the 8
  European ones (Europe, Rev 1, Rev 2, the Rev 2 Virtual Console, the Spanish and German
  translations, built on the Rev 2) among them; the European builds run 12000 frames with the
  hack.
