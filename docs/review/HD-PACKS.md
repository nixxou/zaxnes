# HD packs (feat/hd-pack)

Owner request (2026-11-21): play Mesen HD packs. Branch `feat/hd-pack` from dev, worktree
`G:/3DSen/wt-hdpacks`.

## What the owner asked

- A pack is found in a sub-folder named like the ROM, with the same name tolerance as the 3D
  profiles (`rom_list::normalise`: tags cut, letters and digits only, lower case; same name,
  else a unique prefix), or inside the same archive as the ROM.
- Enabled from the options, for the 2D and/or the 3D, with a per-game option.

## Decisions

- Where packs are looked for: `HdPacks/` beside the ROM, then `<user data>/HdPacks/`. Each holds
  folders (with `hires.txt`) or `.zip` files named like the ROM. A pack inside the ROM's own
  `.zip` / `.7z` comes first.
- Integration into dev (2026-11-26, owner: "même règles que pour les autres fichiers
  additionnels"): a pack of the ROM's archive is a folder of it holding a `hires.txt` (the root
  for one at the top), and it goes with a game by `rom_file::extras::choose`, the rule of the
  archive's profiles and hack packs: the folder named as the game first, then one whose region
  or timing tags are all the game's, then an untagged one, never another game's own
  (`rom_file::read_hd_pack_files`). A game picked inside an archive (`<archive>/<entry>`) looks
  in that archive and in `HdPacks/` beside it, by its entry's name.
- Crate `zax-hdpacks` (safe Rust): `hires.txt` reader and the CPU compositor of the HD frame.
  The parked workshop crate (`feat/hdpack`) is not merged; only ideas came from it.
- Which tile drew a pixel is decided from the core's tile records (background tile covering it,
  up to 4 sprites by OAM order), then **checked against the real picture**: where the model does
  not give the colour the PPU shows (sprite limit, mid-frame palette or scroll changes the
  records do not hold), the original pixel is kept. Mesen records the owner per pixel in its PPU;
  this is the approximation of it the records allow.
- Conditions are evaluated once per tile record (Mesen does it per pixel): same result for the
  usual tile-aligned offsets, far cheaper.

## Status

- [x] crate: loader (tiles, conditions, backgrounds, options), compositor, tests
- [x] discovery (folders, zips, ROM archive) with the name tolerance
- [x] settings: 2D, 3D, per game (handheld game options, Display page)
- [x] 2D: HD picture uploaded instead of the NES picture (GL and wgpu); checked headless with
      `cargo run --release -p zax-hdpacks --example hd_shot` on Castlevania and the workshop's
      exported pack (poc-hdpack/userdata-test/HdPacks)
- [ ] 3D: design to decide (HD textures on the flat tiles?)

## Mesen2 semantics (from its HdPackLoader / HdNesPack, read 2026-11-24)

Followed: backgrounds by priority 0-39 (the first whose conditions hold per priority; 4 layers
around the sprites behind the background, the background tile and the other sprites; Left/Top in
NES pixels; Alpha/Add/Subtract), only whole-frame conditions on a background, `<addition>`
(one more sprite at an offset mirrored with its source, no colours of its own), a condition name
never defined is dropped (the rule holds without it), positionCheck/originPositionCheck,
sppalette0-3, the lowest opaque background-priority sprite hiding the sprites behind it, an HD
background tile drawn even where the original pixel is transparent.

Real pack checked: "Castlevania (U) (PRG 1)" (owner's download, v107, x3, 60k conditions, 51k
backgrounds, 528 additions), read straight from its `.7z` in `HdPacks/`: title screen and stage 1
right; read in 0.1 s; a frame in 2.9 ms over the cores (rayon), 25 ms the first time a picture is
decoded. All its pictures decoded would be 1 GB, so they are decoded when first drawn.

## Scroll ratios (fix/hd-scroll, 2026-11-26)

A `<background>`'s scroll ratios move it by the playfield's scroll times the ratio (fraction
dropped). The scroll is `NesCore::playfield_origin`: the start, on the 512 x 480 plane, of the
longest band of lines drawn with one scroll (the smooth scrolling's band, so a status bar with
a scroll of its own is left out), else the first line with a background. The lines' scroll is
kept only while an HD pack is drawn (`set_tracks_playfield_scroll`). Castlevania's stage 1 needs
it: its lower background, gate and pillar have a ratio of 1; before, they stayed in place and
jumped at each screen. Checked against MesenCE 2.2.1 on the same walk (Start, then Right, a
shot every 180 frames): same pillar position at 3 s, same framing at 6 s. Which line Mesen takes
the scroll from when it changes mid-frame was not read in its code: the band is an assumption
that agrees with Castlevania.

## Not supported yet (and why)

- `ppuMemoryCheck*`: the core API reads CPU memory only; such a condition is false.
- `<background>` scroll ratios on NesHawk and FCEUX: their PPU does not tell the playfield's
  scroll; such a background is drawn fixed there (the Zaxnes core follows it, below).
- Decoded pictures are never released: a long session through every stage can reach the 1 GB
  above. To do: release the pictures not drawn for a while.
- `<fallback>` tiles, `disableCache`, the per-scanline background tile cache: not read.
- `<bgm>`, `<sfx>`, `<patch>`: skipped.
- CHR-ROM tile indices are found back from the tile's bytes (the records do not carry the CHR
  address): a pattern stored twice in the CHR ROM tries each of its indices.
