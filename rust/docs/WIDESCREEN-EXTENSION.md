# Widescreen in Zaxnes

Zaxnes can show a NES game wider (and, with a margin decoder, taller) than the 256 x 240 picture
of the console: the core captures more tiles than the PPU shows, the 3D engine converts them like
any other, and the scene (3D or 2D) shows them past the edges of the original picture.

A game takes part in one of two ways, which work together:

* **the widescreen extension**: a register window at `$5000`-`$5FFF` that a ROM made for it
  unlocks. Through it the ROM asks for margins, says how far its picture is drawn, places sprites
  past the edges, gets more CPU time, four nametables and a PRG window. Real hardware has none of
  this;
* **a margin decoder**: Rust code made for one game that reads the game's level data in memory
  and returns the tiles of the margins, as wide (and tall) as the host asks, with no help from
  the ROM.

Both come with a **built-in patch**: a BPS patch shipped inside the emulator and applied in memory
to the ROM it was made for. The only one today is the Zelda II (USA) widescreen hack, documented
in [`hacks/zelda2/WIDESCREEN-HACK.md`](hacks/zelda2/WIDESCREEN-HACK.md).

Nothing changes for any other ROM: the register window stays transparent until a ROM writes the
unlock sequence, and the captures, savestates and goldens of every other game are the standard
ones.

This document describes everything on the emulator side, layer by layer, as implemented on the
`feat/widescreen` branch.

---

## 1. Host settings (application)

All are Rust-only settings (not exposed to Lua), Video options.

| Key | Values | Default | Effect |
|---|---|---|---|
| `video.widescreen` | Off, Window, 3:2, 16:10, 16:9 | Off | the picture ratio the margins are worked out for |
| `video.widescreen_extra` | +0, 5, 10, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200, 300 % (indices 0-13) | +10 % (index 2) | makes the picture that much larger, in both directions |
| `video.widescreen_strict_edges` | bool | false | see 5.3 |
| `emu.builtin_patches` | bool | true | applies the built-in patches at the next ROM load (see 7) |

The margins are computed by `widescreen_margin(choice, extra, window size)` in
`zax-app/src/application.rs`, every frame, and handed to the core:

```
ratio  = window width / height (Window), 3/2, 16/10 or 16/9
bonus  = 1 + percent / 100
margin_x = ceil8(max(0, (240 * ratio * bonus - 256) / 2))   capped at 512   (each side)
margin_y = ceil8(max(0, 240 * (bonus - 1) / 2))              capped at 240   (above and below)
```

`ceil8` rounds up to a whole tile. The picture keeps the chosen ratio: the bonus makes it as much
taller as wider. With no bonus: 3:2 gives 56 px a side (368 x 240), 16:10 64 (384), 16:9 88
(432). 16:9 with +10 % gives 112 a side and 16 above and below.

`NesCore::set_widescreen_margin(px)` and `NesCore::set_widescreen_vertical_margin(px)` are the
same for any other host. Neither margin is saved in savestates: it is a host setting.

The extra margin is not meant to be seen: the camera frames the picture for the chosen ratio,
and what lies past it is captured so that the 3D objects at the edges of the view are already
whole when they come into sight.

---

## 2. The extension (ROM side): registers

`zax-nes/src/wide.rs`, `WideExtension`, owned by the PPU (`Ppu2C02::wide()`).

The window is `$5000`-`$5FFF`. Writes still reach the cartridge (the window only listens); reads
are answered by the window only once it is unlocked. Every register write but `$5000` is ignored
until then.

| Address | Access | Meaning |
|---|---|---|
| `$5000` | W | unlock: write `$57 $49 $44 $45` ("WIDE") in a row. Reads `$57` once unlocked. |
| `$5001` | R | version, 1 |
| `$5002` | R | margin the host offers on each side, in pixels, a multiple of 8 (0 = off), at most 248 |
| `$5003` | R/W | margin the game asks for on each side, in pixels |
| `$5004` | R/W | how far left of x = 0 the picture may show, in pixels (power-on `$FF`) |
| `$5005` | R/W | how far right of x = 255 the picture may show, in pixels (power-on `$FF`) |
| `$5006` | R/W | overclock: extra CPU time each frame, in scanlines (power-on 0) |
| `$5007` | R/W | bit 0: four separate nametables (power-on 0) |
| `$5008` | R/W | PRG window: bit 7 on, bits 0-6 the 16 KiB PRG-ROM bank shown at `$5800`-`$5FFF` |
| `$5009` | R/W | PRG window: first 256-byte page of that bank it shows |
| `$5100`-`$51FF` | W | high byte of the x of sprite *n* = (address & `$FF`) / 4 |
| `$5200`-`$5FFF` | R/W | 3.5 KiB of RAM (power-on 0); `$5800`-`$5FFF` reads the PRG window instead while it is on |

Details:

* **Margins.** Without a decoder the margin in use (`rom_margin`) is
  `min($5003, $5002, 128)` rounded down to a multiple of 8: 128 is what two nametables hold
  (past it the plane wraps). With a decoder (see 4) the host margin applies whatever the ROM
  asked (`margin()`), and the ROM's own margin still bounds what is read from the nametables and
  which sprites keep their high byte.
* **Extents.** `$5004` and `$5005` are read each time a background band is captured: a game
  writes them during the frame to hide the sides behind a status bar or past the end of a level.
  A tile is kept when any of its pixels falls in
  `[-min(rom_margin, $5004), 256 + min(rom_margin, $5005))` (`visible_range()`).
* **Sprites.** The x of sprite *n* is the signed 16-bit value `high:OAM x`, `high` being the byte
  last written to `$5100 + 4n` (any of its four addresses) **before the OAM DMA** that copied the
  page (`latch_sprites()`, called by the bus after each `$4014` write). Each DMA takes the bytes
  written since the previous one and starts the next page from 0: a game writes the high byte of
  every sprite it places off the standard picture, every frame. x = -16 is `$FF:$F0`, x = 300 is
  `$01:$2C`. Sprites past the visible range are dropped. Without an unlocked ROM the high byte is
  ignored.
* **Overclock.** At the start of line 240 the PPU arms `$5006` x 114 CPU cycles
  (`arm_overclock()`). For each of them `NesBus::tick()` runs the CPU and the cartridge only
  (`take_stall_cycle()`): the PPU and the APU stay still, so the picture, the sound and their
  timing do not change, while the frame logic gets that much more time before the NMI. The
  cartridge must keep counting cycles: MMC1 ignores a write that follows another on the next
  cycle, and a frozen cartridge clock turns every bank switch made during the overclock into
  such a write (the game then jumps into the wrong bank).
* **Four nametables.** `$5007` bit 0 lays the console's 2 KiB of VRAM plus 2 KiB more out as
  2 x 2 nametables whatever mirroring the cartridge selects (`Ppu2C02::layout_of(cart)`, used by
  the renderer, the capture and the nametable accesses). The plane is then 512 x 480: a game that
  scrolls both ways can keep columns drawn past both sides.
* **PRG window.** `$5008`/`$5009` show 2 KiB of PRG-ROM at `$5800`-`$5FFF` whatever bank the
  mapper has at `$8000` (`prg_window(address)`, read by `NesBus::wide_window_read` from the
  cartridge's PRG-ROM): code the fixed bank calls can live in any bank. Being ROM, it is always
  the code of the ROM loaded; a copy in RAM would travel in savestates and outlive a new version
  of a hack.
* The 64-sprite OAM and the 8-sprites-per-line limit are unchanged (the core already captures
  every sprite).

Typical use:

```
        LDA #$57 : STA $5000     ; unlock
        LDA #$49 : STA $5000
        LDA #$44 : STA $5000
        LDA #$45 : STA $5000
        LDA $5000 : CMP #$57     ; not the extension: open bus
        BNE no_wide
        LDA $5002 : STA $5003    ; take what the host offers
```

---

## 3. Capture (core, PPU)

`zax-nes/src/ppu/tiles.rs`, `tile_buffer.rs`, `frame_batch.rs`.

The core captures one `TileRecord` per 8 x 8 tile (planes, x, y, palette, flags, index), 3dSen
style: the background row of each band on the line where its fine Y is 1, and the sprites of
each band. Widescreen changes:

* **Latching.** `begin_rendered_frame()` latches the frame's margins: `frame_margin =
  wide.margin()`, `frame_vertical_margin = wide.vertical_margin()`, and sizes the tile buffer
  (`TileBuffer::set_extra_area(columns, rows)`: `(31 + 2 rows) x (33 + 2 columns)` background
  records at most, 128 sprites). Every band of one frame has the same columns.
* **Background bands.** With `extra = frame_margin / 8`, a band captures screen columns
  `k = -extra ..= 32 + extra` (x = 8k - fine X; the last one only while it starts left of
  `256 + 8 extra`), reading on through the horizontally neighbouring nametable
  (`(coarse X + k) mod 64`), and keeps those inside `visible_range()`. Record index:
  `row * (33 + 2 extra) + k + extra`.
* **Captured rows.** Each background band also records where it was read from
  (`CapturedRow`: PPU address `v` at the start of the line, fine X, y, pattern table), for a
  margin decoder.
* **Visible rows.** `FrameBatch::visible_rows` (30 bands of 8 lines) holds, per band, the range
  of x the band could show tiles in: `visible_range()` at its capture, or the full margin for a
  band a decoder filled.
* **Sprites.** `x = wide.sprite_x(n, oam_x)` (16-bit); dropped outside `visible_range()`.
* **The batch.** `FrameBatch { records, palette, mask_x, mask_y, margin, visible_rows,
  margin_y }`. Background records reach `-margin - 7 .. 255 + margin` in x and
  `-margin_y - 8 .. 239 + margin_y` in y.
* **The 2D picture.** `NesCore::wide_picture_into(out) -> rows` composes a
  `(256 + 2 margin) x (240 + 2 margin_y)` picture of NES colour indices: the centre is the PPU's
  own pixels, the rest is drawn from the records (sprites behind, background, sprites in front,
  by OAM priority), unfilled pixels taking the backdrop colour. When the game blanks the left 8
  pixels (PPUMASK), those are composed from the records too, otherwise they would be a black band
  in the middle of the wide picture. Without margins it is the plain 256 x 240 frame.

---

## 4. Margin decoders

`zax-nes/src/wide_decode/` (`mod.rs`: the interface; one module per game).

The nametables only hold what a game keeps drawn, two screens at most. A game that keeps its whole
area in RAM can do better: a decoder turns that RAM into the tiles of the margins.

```rust
pub trait MarginDecoder: Sync {
    fn decode_row(&self, memory: &DecoderMemory, row: &BackgroundRow,
                  columns: &mut dyn Iterator<Item = i32>, out: &mut Vec<DecodedTile>) -> bool;
    fn decode_extra_row(&self, memory: &DecoderMemory, row: &BackgroundRow,
                        columns: &mut dyn Iterator<Item = i32>, out: &mut Vec<DecodedTile>) -> bool {
        false
    }
}
```

* `DecoderMemory`: CPU RAM (`$0000-$07FF`), PRG-RAM (`$6000-$7FFF`) and the whole PRG-ROM image as
  the cartridge runs it (patch applied).
* `BackgroundRow`: the line, the PPU address `v` at its start, fine X; helpers `coarse_x`,
  `coarse_y`, `nametable_x`, `nametable_y`, `ring_x` (pixel of the 512-pixel plane of two
  nametables at the left edge).
* `DecodedTile { column, tile, palette }`: screen column `column` (x = 8 column - fine X, 0 being
  the leftmost column of the 256-pixel picture), a CHR tile of the row's background pattern table,
  a sub-palette.
* `resolve_ring(ring_position, ring, estimate)`: the instance of a wrapping coordinate nearest to
  an estimate. RAM is read after the frame and may already be a step ahead of the scroll the frame
  used: the position comes from the PPU (exact modulo the ring) and RAM only picks the instance.

How the core uses it (`NesCore::run_frame`, then `Ppu2C02::decode_margins`):

1. Before the frame, `wide.set_decoded(true)` when the ROM's built-in patch has a decoder
   (`wide_decode::for_patch(name)`): the host margin then applies.
2. After the frame, for each captured background row, the decoder is asked for screen columns
   `-extra..0` and `32..=32 + extra`. A row it returns `false` for (status bar, title, a mode it
   does not know) keeps the columns captured from the nametables, within the ROM's margin. For a
   decoded row, the nametable margin records are removed and the decoded tiles added, their CHR
   planes read through the PPU as for any background tile; its visible row becomes the full
   margin. The centre 256 pixels always come from the PPU.
3. With a vertical margin, `decode_extra_row` is asked for `margin_y / 8` rows above the first
   captured row and as many below the last, whole width. Each virtual row carries the PPU address
   it would have on the plane of two stacked nametables (60 rows, the vertical nametable bit
   flipping every 30) and a line outside `0..240`. A decoder that cannot returns `false` (the
   default) and the vertical margin stays empty.

Decoders run only while the built-in patches are on. The decoders:

| Game | Decodes | Source |
|---|---|---|
| Zelda II (USA) | side view (level RAM), world map (run-length map), vertical margin on the world map | `zelda2.rs`, adapted from z2rs (MIT), see the Zelda II document |

---

## 5. The 3D engine

`zax-engine3d`.

### 5.1 Grid

`grid.rs`: the background and sprite grids reserve `WIDE_COLUMNS = 64` cells left of cell x = 0
and right of x = 34, and `WIDE_ROWS = 30` above y = 0 and below the last row. Cell coordinates
keep their standard meaning (a margin tile simply has a negative cell x or y, or one past the
standard range); only the array index is offset: `index(x, y) = x + WIDE_COLUMNS + (y + WIDE_ROWS)
* WIDTH`, `WIDTH = 35 + 2 * WIDE_COLUMNS`. `contains`, `cell_of` and the frame's cell list follow.

### 5.2 Clip area and intake

`engine.rs`: `ConversionEngine::convert(batch)` latches `batch.margin` (capped at 512),
`batch.visible_rows` and `batch.margin_y` (capped at 240) on the frame. The clip test of the
intake is `is_inside_tall_clip_area(x, y, clip, (margin, margin_y))`: the profile's clip margins
count from the wider and taller edges. Background tiles landing on the outermost cell of the
grid are dropped, as in the standard grid.

### 5.3 Finished shapes

A shape touching the unreliable border band (REQ-E3D-045) is "unfinished" and gets a temporary
pattern; once whole it gets its permanent one. With margins, `FrameShape::inside()` measures the
border from the wider and taller edges. `ConversionFrame::is_shape_finished(id)` (used by the
segmenter) also checks, on a widescreen frame, that the shape stays clear of the edge of what each
of its bands showed (`visible_rows`): a shape cut where the game stops drawing is not finished.

The application passes the real visible rows only with `video.widescreen_strict_edges` on;
off (the default), every band counts as drawn across the whole margin (`GameSession`
replaces them), because a one-tile object standing at such an edge would otherwise stay
temporary.

---

## 6. Rendering

`zax-render`, `zax-app`.

* `SceneState::margin` and `margin_y` (NES pixels) come from the published frame
  (`ConversionFrame::margin()`, `margin_y()`), set by `update_scene` in `application.rs`.
* `SceneState::view_clipping()`: the profile's clip margins with the left/right ones moved out by
  `margin` and the top/bottom ones by `margin_y` (a negative margin reaches past the picture). The
  visible rectangle (fragment clip `uClipRect`, camera aim point, shadow volume) uses it; the
  camera framing itself is unchanged.
* 2D mode (F11, or a game without a 3D profile): `SceneFrame { nes_picture, nes_picture_height }`
  carries the wide picture and its rows. `interop::picture_size(len, height)` validates it,
  `interop::wide_quad_for(picture, height, cached)` builds the textured quad (x from -margin to
  256 + margin, y up to `margin_y` past the top and bottom of the NES plane), cached per
  `(margin, margin_y)`. Both backends (GL `gl/picture.rs`, wgpu `wgpu_backend/resources.rs`)
  reallocate the picture texture when its size changes.

---

## 7. Built-in patches

`zax-nes/src/builtin_patches.rs`, patches in `zax-nes/patches/`.

```rust
pub struct BuiltinPatch {
    pub name: &'static str,
    pub source_crc: u32,                              // CRC-32 of the ROM without iNES header
    pub bps: &'static [u8],                           // include_bytes!
    pub after_other_way_state: Option<fn(&mut [u8])>, // see 8.3
}
```

* A ROM is recognised by the CRC-32 of its data without the iNES header; the BPS patch was made
  against that data, and the player's header is kept. `apply(image)` checks the BPS source,
  target and patch checksums, so a `Some` is exactly the build.
* `BuiltinPatch::version()` is the target CRC-32 read from the BPS footer: any change to a hack
  changes it.
* `NesCore::load_rom` applies the matching patch when `set_builtin_patches(true)` (the setting
  `emu.builtin_patches`, read at each ROM load) and keeps the **original** image
  (`original_image()`); `builtin_patch()` names the patch.
* The battery file and the savestate fingerprint use the original image: a new version of a hack
  is the same game, and its battery RAM carries over.
* The patch only holds the bytes the hack writes (TargetRead); everything else is copied from the
  player's ROM (SourceRead): no game data ships with the emulator.

---

## 8. Savestates

### 8.1 Core block

`NesCore::save_state` writes format version **4** for a machine whose ROM never unlocked the
extension (byte for byte the standard block) and **6** once it did: the extension's registers,
sprite high bytes, RAM, overclock, layout, PRG window and stall counter follow the cartridge
block. Version 5 (before the PRG window) is still read. The block carries the **original** ROM
image; `restore_state` applies the built-in patch of the running build to it (when the patches
are on), so a state survives a new version of the hack's code.

### 8.2 State file (application)

`zax-app/src/persistence/save_state_file.rs`: signature `ZAXNESAV`, container version 1, then
sections each prefixed by its length: JSON header, core block, scripts' data, camera, learned
conversion state, and an optional sixth section, a **JPEG thumbnail** of the 2D picture
(`thumbnail_jpeg`: the centre 320 columns of the wide picture, 4:3, quality 85, `jpeg-encoder`).
Files without the sixth section read as before. Header fields (camelCase):

| Field | Meaning |
|---|---|
| `coreId`, `coreStateVersion`, `gameId`, `slotId`, `savedAt` | as before |
| `romSha1` | fingerprint of the original ROM (PRG + CHR) |
| `isHack` | whether a built-in patch ran |
| `hackName`, `hackVersion` | the patch's name and `version()` as 8 hex digits |
| `appVersion` | the application's version |
| `savedAtLocal` | local date and time, `YYYY-MM-DD hh:mm:ss` |

The new fields are optional when read.

### 8.3 Loading a state made the other way

`EmulationHost::state_patch_status(state)` (`emulation/host/jobs.rs`) classifies a state:

* `Fits`: loads as it is (also any state of a ROM no built-in patch is made for).
* `OtherMode { hack }`: made with the hack (`isHack`, or for older files a core block of version
  5/6) while `emu.builtin_patches` is off, or the reverse. Shown in **orange** in the load menus.
  Choosing it asks (`TURN_HACK_ON_QUESTION` / `TURN_HACK_OFF_QUESTION`, translated); agreeing
  sets `emu.builtin_patches`, then loads the slot, whose restore reloads the ROM from the state
  with the patch as now set.
* `OtherVersion`: made with the hack and a `hackVersion` other than the running patch's. Shown in
  **red**; refused (`OTHER_HACK_VERSION_MESSAGE`). Older files without `hackVersion` count as the
  current version.

The status is cached per slot, save time and setting (`slot_patch_cache`). Restoring the other
way also needs the machine put right, which the core does in `restore_state`:

* a state of the hack restored without it: the extension is reset (the original game never
  unlocks it, and four nametables or margins must not stay);
* a state made the other way in either direction: the patch's `after_other_way_state` runs on CPU
  RAM (Zelda II: the world map is sent back through the game's map initialisation, which redraws
  it in the layout the running code expects);
* a state without the hack restored with it: the extension starts locked; the hack itself
  unlocks it again on the next NMI.

---

## 9. Tools

`zax-harness render` options for widescreen work:

| Option | Effect |
|---|---|
| `--widescreen <px>` | host margin on each side |
| `--widescreen-y <px>` | host margin above and below |
| `--no-builtin-patches` | restore `--load-state` without the built-in patches |
| `--load-state <file>` / `--save-state <file>` | core state block (saved with its ROM image) |
| `--png-nes <file>` | the frame's records composed flat, as wide as the margin |
| `--dump-mem <file>` | `$0000-$07FF`, `$6000-$7FFF`, `$5000-$5FFF`; prints PC, registers, JAMMED |
| `--press <frames>:<buttons>` | input script |

Tests: `zax-nes` (`wide.rs`, `wide_decode`, `builtin_patches.rs`; one ignored test needs
`ZAXNES_ROMS` with the Zelda II ROM), `zax-app/tests/save_state_tests.rs` (header, thumbnail,
old format), `zax-settings/tests/catalogue.rs` (the Rust-only keys).
