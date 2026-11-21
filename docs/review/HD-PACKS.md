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
  folders (with `hires.txt`) or `.zip` files named like the ROM. A `hires.txt` inside the ROM's
  own `.zip` / `.7z` comes first.
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

## Not supported yet (and why)

- `ppuMemoryCheck*`: the core API reads CPU memory only; such a condition is false.
- `<background>` scroll ratios: the NES scroll is not known to the app; drawn fixed.
- `<bgm>`, `<sfx>`, `<patch>`: skipped.
- CHR-ROM tile indices are found back from the tile's bytes (the records do not carry the CHR
  address): a pattern stored twice in the CHR ROM tries each of its indices.
