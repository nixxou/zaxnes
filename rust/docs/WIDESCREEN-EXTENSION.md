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
to the ROM it was made for: the Zelda II (USA) widescreen hack
([`hacks/zelda2/WIDESCREEN-HACK.md`](hacks/zelda2/WIDESCREEN-HACK.md)) and the Mega Man 2 (USA)
one ([`hacks/megaman2/WIDESCREEN-HACK.md`](hacks/megaman2/WIDESCREEN-HACK.md)), the Super Mario
Bros. (World) one, its advanced mode ([`hacks/smb1/WIDESCREEN-HACK.md`](hacks/smb1/WIDESCREEN-HACK.md)),
the Ninja Gaiden III (USA) one, its advanced mode too
([`hacks/ng3/WIDESCREEN-HACK.md`](hacks/ng3/WIDESCREEN-HACK.md)), the Super Mario Bros. 3
(USA) (Rev 1) one, its advanced mode too ([`hacks/smb3/WIDESCREEN-HACK.md`](hacks/smb3/WIDESCREEN-HACK.md)),
and the Castlevania (USA) one, built for its Rev 1 too, an advanced mode as well
([`hacks/castlevania/WIDESCREEN-HACK.md`](hacks/castlevania/WIDESCREEN-HACK.md)), and the
Adventure Island (USA) one, its advanced mode too
([`hacks/adventure_island/WIDESCREEN-HACK.md`](hacks/adventure_island/WIDESCREEN-HACK.md)), and
the Excitebike (Japan, USA) one, an advanced mode as well
([`hacks/excitebike/WIDESCREEN-HACK.md`](hacks/excitebike/WIDESCREEN-HACK.md)), and the Final
Fantasy (USA) one, an advanced mode as well
([`hacks/ff1/WIDESCREEN-HACK.md`](hacks/ff1/WIDESCREEN-HACK.md)), and the Metal
Force (Korea) one, an advanced mode as well
([`hacks/metalforce/WIDESCREEN-HACK.md`](hacks/metalforce/WIDESCREEN-HACK.md)), and the Blaster
Master (USA) one, an advanced mode as well
([`hacks/blastermaster/WIDESCREEN-HACK.md`](hacks/blastermaster/WIDESCREEN-HACK.md)), and the Mega
Man (USA) one, an advanced mode as well
([`hacks/megaman1/WIDESCREEN-HACK.md`](hacks/megaman1/WIDESCREEN-HACK.md)), and the Mega
Man 3 (USA) one, an advanced mode as well
([`hacks/megaman3/WIDESCREEN-HACK.md`](hacks/megaman3/WIDESCREEN-HACK.md)), and the Mega
Man 4 (USA) one, built for its Rev 1 too, an advanced mode as well
([`hacks/megaman4/WIDESCREEN-HACK.md`](hacks/megaman4/WIDESCREEN-HACK.md)), and the Mega
Man 5 (USA) one, an advanced mode as well
([`hacks/megaman5/WIDESCREEN-HACK.md`](hacks/megaman5/WIDESCREEN-HACK.md)), and the Mega
Man 6 (USA) one, an advanced mode as well
([`hacks/megaman6/WIDESCREEN-HACK.md`](hacks/megaman6/WIDESCREEN-HACK.md)), and the
Metroid (USA) one, an advanced mode as well
([`hacks/metroid/WIDESCREEN-HACK.md`](hacks/metroid/WIDESCREEN-HACK.md)).

Nothing changes for any other ROM: the register window stays transparent until a ROM writes the
unlock sequence, and the captures, savestates and goldens of every other game are the standard
ones.

This document describes everything on the emulator side, layer by layer, as implemented in
Zaxnes.

---

## 1. Host settings (application)

All are Rust-only settings (not exposed to Lua). The first two are options of each game, set in
its hacks screen (console mode: the ROM list, the game's options, the in-game menu; desktop: Game
options > Hacks..., `ui/handheld/hacks.rs`) and kept in `games.json` with its other options; the
global options window no longer shows them, nor `emu.builtin_patches`.

| Key | Values | Default | Effect |
|---|---|---|---|
| `video.widescreen` | Off, Auto, 3:2, 16:10, 16:9 | Off | the picture ratio the margins are worked out for; Auto is the view's |
| `video.widescreen_extra` | +0, 5, 10, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200, 300 % (indices 0-13) | +20 % (index 4) | makes the picture that much larger, in both directions |
| `video.widescreen_strict_edges` | bool | false | see 5.3 (global, Video options) |
| `emu.builtin_patches` | bool | true | allows the built-in patches; a ROM is patched at load only when its `video.widescreen` is on too (see 7) |

The ROM list knows the games a hack is made for from the CRC-32 of their data
(`builtin_patches::widescreen_capable`: a built-in patch or a margin decoder), kept in
`rom_crc.json` of the user data with each file's size and modification time
(`persistence/rom_crc_cache.rs`, version 2). The data is the PRG-ROM and CHR-ROM within the sizes
the iNES (or NES 2.0) header gives, without the header, a trainer or an overdump
(`builtin_patches::rom_data`, `data_crc`): the CRC No-Intro gives, and what the BPS patches are
made against.

A ROM whose 3D profile title or file name says a game a patch is made for, but whose data is
another version of it (`builtin_patches::for_game_name`; each patch knows its `game` and the
No-Intro name of its ROM; the longest game whose words the name starts with, and not one whose
number goes on, in figures or in Roman numerals: "Super Mario Bros. 3 (Europe)" is Super Mario
Bros. 3's, "Super Mario Bros. 2 (USA)", "Castlevania II" and "Adventure Island II" no one's), shows a red [W]. When its data has the size the patch was made for
(`builtin_patches::source_size`), the patch can be **forced** on it: `emu.forced_patch` (an option
of that game, the patch's name) makes `NesCore::set_forced_patch` apply it at load and restore
through `builtin_patches::apply_with`, which then skips the BPS checksums of the source and the
target (`apply_bps_checked`). The state checks of section 7 know nothing of a forced patch.

The margins are computed by `widescreen_margin(choice, extra, view size)` in
`zax-app/src/application.rs`, every frame, and handed to the core:

```
ratio  = view width / height (Auto), 3/2, 16/10 or 16/9
bonus  = 1 + percent / 100
margin_x = ceil8(max(0, (240 * ratio * bonus - 256) / 2))   capped at 512   (each side)
margin_y = ceil8(max(0, 240 * (bonus - 1) / 2))              capped at 240   (above and below)
```

The view size is the scene's view before any resolution scale: its ratio is the picture's, while
the scene's own size moves with the automatic resolution (and its rounding with it, which would
make the margin jump a tile back and forth).

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
| `$5007` | R/W | bit 0: four separate nametables; bit 1: sprites free of the extents; bit 2: second sprite page; bit 3: sprite high bytes kept from one page to the next (power-on 0) |
| `$5008` | R/W | PRG window: bit 7 on, bits 0-6 the 16 KiB PRG-ROM bank shown at `$5800`-`$5FFF` |
| `$5009` | R/W | PRG window: first 256-byte page of that bank it shows |
| `$500A` | R | the options of the ROM's built-in patch the player turned on, one bit each (see 7) |
| `$5100`-`$51FF` | W | high byte of the x of sprite *n* = (address & `$FF`) / 4 |
| `$5300`-`$53FF` | RAM | with `$5007` bit 2: a second page of 64 sprites, OAM format |
| `$5400`-`$54FF` | RAM | with `$5007` bit 2: high byte of the x of its sprite *n* at `$5400 + 4n` |
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
  `[-min(rom_margin, $5004), 256 + min(rom_margin, $5005))` (`visible_range()`). With `$5007`
  bit 1 the extents apply to the background only: sprites show across the margin the ROM asked
  for, up to 248 (`sprite_range()`; the nametables' 128 does not apply to them), for a game whose margins a decoder draws while the ROM hides the
  nametable columns (extents 0).
* **Sprites.** The x of sprite *n* is the signed 16-bit value `high:OAM x`, `high` being the byte
  last written to `$5100 + 4n` (any of its four addresses) **before the OAM DMA** that copied the
  page (`latch_sprites()`, called by the bus after each `$4014` write). Each DMA takes the bytes
  written since the previous one and starts the next page from 0: a game writes the high byte of
  every sprite it places off the standard picture, every frame. x = -16 is `$FF:$F0`, x = 300 is
  `$01:$2C`. Sprites past the visible range are dropped. Without an unlocked ROM the high byte is
  ignored. A sprite whose x is outside 0..255 is left out of the PPU's own picture (its pattern
  bytes read as 0 at the fetches of dots 257-320, which still take place; the evaluation keeps the
  OAM index of each sprite it copies for that): a hardware PPU, which only knows the low byte,
  would draw a sprite at x = 298 at x = 42. Nor does such a sprite take one of the eight places of a line in the evaluation, or raise the overflow: the sprites on the screen all show.
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
  of a hack. When the game's built-in patch is a hack pack (7.1) whose widescreen runs, the
  window shows the pack's code instead (`WideExtension::set_code`, `code()`): the bank in
  `$5008` bits 0-6 is then a 16 KiB part of that code, `$FF` past its end. The code is the
  build's, set by the core at each load and restore; it is not part of a state, and `reset()`
  keeps it.
* **Second sprite page** (`$5007` bit 2): at each OAM DMA the extension latches its RAM at
  `$5300` (64 sprites, OAM format) and `$5400 + 4n` (the high bytes of their x). The core captures
  them after the console's 64, as records 128 and up (below them all in priority), 8 x 8 only; the
  2D picture draws them over the PPU's centre too. A game whose sprites no longer fit in 64 (more
  enemies in the margins) moves on to it instead of flickering. The 8-sprites-per-line limit does
  not apply to captures anyway.

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
  margin decoder, and the CHR banks it was drawn with (`chr`: the offsets of the eight 1 KiB
  windows, `ChrView::offsets()` of `Cartridge::chr_peek_view()`). The decoded tiles of the row
  are read through those banks (`read_row_planes`): the margins are decoded once the frame is
  over, when a game that switches banks during the frame (Super Mario Bros. 3: its status bar
  below line 192, its animated tiles) has other banks in place. A board with no side-effect-free
  view keeps the reading through the banks of the end of the frame.
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
  in the middle of the wide picture. The sprites the PPU did not draw are then drawn over the
  centre from their records: the second page, and those across the left edge (x < 0), whose part
  on the screen the PPU left out with the rest; one behind the background only over the backdrop
  colour. For a decoder that covers the overscan, the lines of the screen it covered (see section
  4, step 3) are composed from the records like the margins. Without margins
  it is the plain 256 x 240 frame.

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

* `DecoderMemory`: CPU RAM (`$0000-$07FF`), PRG-RAM (`$6000-$7FFF`), the whole PRG-ROM image as
  the cartridge runs it (patch applied), and `nametable(address)`, a nametable byte as the PPU
  reads it now (mirroring and layout applied), and `nametable_written(address)`, the rendered
  frame that byte was last written in (0 since power-on or a state load). A decoder can check what
  it decodes for the centre against what the PPU shows, and leave a row that does not match (a
  menu, a transition) to the nametables; and it can tell a nametable column the game drew for a
  place (written in one go) from an older one of the ring, and what objects drew over it later.
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

1. Before the frame, `wide.set_decoded(true)` when the game has a decoder: the one of its
   built-in patch (`wide_decode::for_patch(name)`), or else one made for the unpatched ROM
   (`wide_decode::for_rom(crc)`, the CRC-32 of the ROM's data as the host gave it). The host
   margin then applies; a ROM that never unlocks the extension gets margins from its decoder
   alone (its sprites keep 8-bit x and its own culling).
2. After the frame, for each captured background row, the decoder is asked for screen columns
   `-extra..0` and `32..=32 + extra`. A row it returns `false` for (status bar, title, a mode it
   does not know) keeps the columns captured from the nametables, within the ROM's margin. For a
   decoded row, the nametable margin records are removed and the decoded tiles added, their CHR
   planes read through the PPU as for any background tile; its visible row becomes the full
   margin. The centre 256 pixels always come from the PPU.
3. With a vertical margin, `decode_extra_row` is asked for `margin_y / 8` rows above the first
   captured row and as many below the last, whole width. Each virtual row carries the PPU address it would have on
   the plane of two stacked nametables (60 rows, the vertical nametable bit flipping every 30)
   and a line outside `0..240`. A decoder that cannot returns `false` (the default) and the
   vertical margin stays empty on that side. A decoder whose `covers_overscan()` is true (off by
   default; Adventure Island only) also gets the screen's overscan: the rows asked for cover
   `margin_y` lines past the screen whatever the fine Y scroll (`ceil((first row's y +
   margin_y) / 8)` above, `ceil((240 + margin_y - last row's bottom) / 8)` below: the first
   captured row may start a few lines down, and `margin_y / 8` rows leave those lines empty), and
   the first and last captured rows (across the screen's edge, or ending on it) are asked for
   their centre (columns `0..32`) too: the game shows there a nametable row it keeps for the
   overscan, often another row of its ring (Adventure Island scrolling vertically: past the
   screen, or the whole bottom row when the vertical scroll is a multiple of 8 and the ring's 30
   rows all show). When the decoder gives it, the row's centre records are replaced by the decoded
   ones (same record indices). The lines of the screen a decoded row covers (that edge row's, or the row just
   above the first captured one, or just below the last, when the decoder gives it) are composed
   from the records in the 2D picture too, not from the PPU's pixels (`Ppu2C02::centre_lines`,
   latched by `decode_margins`, `(0, 240)` otherwise): those are the lines a television's
   overscan hides, where the game may show another row of its ring (a band across the picture
   between the screen and the vertical margin). A decoder whose `overscan_lines()` is `n` (default
   0; Blaster Master 8) also gets the centre of the other captured rows starting within `n` lines
   of the screen's top or ending within `n` lines of its bottom (`decode_extra_rows`, after the
   edge rows; a row the decoder declines keeps the PPU's), composed from its records likewise: a
   game whose attribute bytes for the row coming in at one edge also colour the next row, still
   on the screen.

4. A decoder whose `redrawn_right_columns()` is `n` (default 0) is also asked for the screen
   columns `32 - n` to 31 of each row, and those columns' nametable records of a decoded row are
   replaced. The wide picture composes the centre of that band from the records from
   `x = 8 (32 - n) - fine X` on instead of taking the PPU's pixels (`Ppu2C02::redrawn_from`, per
   band of 8 lines, 256 for none; sprites from their records too): a game whose last visible
   columns show colours of the column it is drawing past the edge (Super Mario Bros. 3, 3
   columns: the attribute bytes it writes ahead also cover them).
   The same on the left: a decoder whose `redrawn_left_columns()` is `n` (default 0) is also
   asked for screen columns 0 to `n - 1`, their nametable records are replaced, and the wide
   picture composes the centre of that band from the records up to `x = 8 n - fine X`
   (`Ppu2C02::redrawn_to`, per band of 8 lines, 0 for none): a game that draws the column
   coming in at the left edge with attribute bytes that also colour the visible column next to
   it (Metroid, 2 each side). Extension change: `MarginDecoder::redrawn_left_columns`, the field
   `redrawn_to` (reset with the frame), the first centre column of `decode_margins` the larger of
   the masked column's and `n`, and `wide_picture_into` taking the PPU's pixels from
   `max(redrawn_to, the masked column)` of each band.
5. A decoder whose `decodes_masked_column()` is true (default false) is also asked for screen
   column 0 of each row while the game hides the leftmost 8 pixels of the background (PPUMASK
   bit 1 clear), and that column's nametable record of a decoded row is replaced too: the wide
   picture composes those 8 pixels from the records, and such a game keeps there the column it is
   drawing next (Super Mario Bros. 3, horizontal mirroring: the column it draws past the right
   edge is the same nametable column as the one at the left edge).
6. A decoder's `margin_palette(memory)` (default `None`) is a background palette of its own for
   the frame, 16 NES colour indices laid out as `$3F00-$3F0F`: with one, the decoded tiles it
   hands back in sub-palettes 4-7 (`DecodedTile::palette`) are in its sub-palettes 0-3 (without
   one, 4-7 read as 0-3). For a game that shows a neighbour of the screen in other colours than
   the screen's (Metroid: a room past one where the game toggles the room palette). Extension
   change: `Ppu2C02::margin_palette` (latched by `decode_margins`, cleared with the frame) keeps
   it; the records keep sub-palettes 4-7 (`TileRecord::palette`, background records only); the
   wide picture composes those pixels from it (palette entries `$20-$2F` of the composition,
   `MARGIN_ENTRIES`); `FrameBatch::margin_palette` carries it to the 3D engine (`None` for every
   other frame), which loads it into tables 8-11 of the frame palette
   (`FramePalette::load_margin`, entry 0 of each table the universal colour; those tables
   repeat the background ones otherwise) and colours a background tile in sub-palette 4-7 with
   table `palette + 4` (`draw_list.rs`, REQ-E3D-560). A script that writes those tables as a
   custom palette sees them overwritten in such a frame. Nothing changes for a decoder without
   one.

The CHR banks of the decoded rows: `Mapper::chr_peek_view` (default: `chr_view`) is the view
`ppu_peek` reads, side effects aside. MMC3 has one (its peek is the default one through its bank
windows; only its fetches clock the IRQ counter) although its `chr_view`, the PPU's fast path,
is `None`.

Decoders run only while the built-in patches are on for the game in play (see 7). The decoders:

| Game | Decodes | Source |
|---|---|---|
| Zelda II (USA) | side view (level RAM), world map (run-length map), vertical margin on the world map | `zelda2.rs`, adapted from z2rs (MIT), see the Zelda II document |
| Mega Man 2 (USA) (data CRC-32 `0FCFC04D`), with its built-in patch (also listed for the unpatched ROM) | the stage layout in PRG-ROM, within the section being played | `megaman2.rs`, see below |
| Super Mario Bros. (World) (data CRC-32 `D445F698`), unpatched | the area, laid out by the game's own area parser run in a sandbox | `smb1.rs`, see below |
| Ninja Gaiden III (USA) (data CRC-32 `902E3168`), unpatched and with its built-in patch | the horizontal areas' layout in PRG-ROM, parallax bands included | `ng3.rs`, see below |
| Super Mario Bros. 3 (USA) (Rev 1) (data CRC-32 `2E6301ED`), with its built-in patch (also listed for the unpatched ROM) | a horizontal level or a world map, from its tile map in PRG-RAM, and the vertical margin | `smb3.rs`, see below |
| Castlevania (USA), both builds (data CRC-32 `0AC1AA8F`, Rev 1 `B668C7FC`), unpatched and with their built-in patches | the room in play, from its map and its stage's blocks in PRG-ROM | `castlevania.rs`, see below |
| Excitebike (Japan, USA) (data CRC-32 `49AEB3A6`), unpatched and with its built-in patch | a race's track, laid out by the game's own routine in the sandbox, one lap | `excitebike.rs`, see below |
| Final Fantasy (USA) (data CRC-32 `CEBD2A31`), unpatched and with its built-in patch | the world map (unpacked from PRG-ROM) and the standard maps (PRG-RAM); vertical margin, overscan | `ff1.rs`, see below |
| Adventure Island (USA) (data CRC-32 `F8A713BE`), unpatched (and with its built-in patch) | the round, from its tables in PRG-ROM, slopes and vertical scrolling included; vertical margin | `adventure_island.rs`, see below |
| Metal Force (Korea) (data CRC-32 `EC6D910E`, mapper 156), unpatched and with its built-in patch | the level map in PRG-RAM, sides and vertical margin | `metalforce.rs`, see below |
| Blaster Master (USA) (data CRC-32 `3F0FD764`), unpatched and with its built-in patch | the area's map in PRG-ROM, four levels of blocks, within the room the camera is in; vertical margin, overscan | `blastermaster.rs`, see below |
| Mega Man (USA) (data CRC-32 `6EE4BB0A`), unpatched and with its built-in patch | the stage layout in PRG-ROM, within the section being played | `megaman1.rs` (with `megaman.rs`), see below |
| Mega Man 3 (USA) (data CRC-32 `603AAA57`), unpatched and with its built-in patch | the stage layout in PRG-ROM, within the section being played | `megaman3.rs` (with `megaman.rs`), see below |
| Mega Man 4 (USA), both builds (data CRC-32 `2BC67AA8`, Rev 1 `18A2E74F`), unpatched and with their built-in patches | the stage layout in PRG-ROM, within the section being played | `megaman4.rs` (Mega Man 3's `Layout`), see below |
| Mega Man 5 (USA) (data CRC-32 `3EDCF7E8`), unpatched and with its built-in patch | the stage layout in PRG-ROM, within the section being played | `megaman5.rs` (Mega Man 3's `Layout`), see below |
| Mega Man 6 (USA) (data CRC-32 `988798A8`), unpatched and with its built-in patch | the stage map in PRG-ROM through the pointers the game keeps in RAM, within the part of the stage the camera may show | `megaman6.rs`, see below |
| Metroid (USA) (data CRC-32 `70080810`), unpatched and with its built-in patch | the world map's rooms around the screen: the two the game keeps in PRG-RAM, the others built from the area's data as the game does; vertical margin, overscan | `metroid.rs`, see below |

Mega Man 2: a stage lives in PRG bank `$2A & 7` (`$2A` the stage), as the game's collision lookup
(`$CBC3`) and column drawer (`$C96B`) read it. Screen `s` is 64 bytes at `$8500 + 64 s`, 8 columns
of 8 structures of 32 x 32 pixels, column first; structure `n` is 4 bytes at `$8000 + 4 n`, its
16 x 16 blocks column first, its attribute byte at `$8400 + n`; block `b` (the low 6 bits of the
block byte, the high 2 being its collision) is CHR tiles `4 b` to `4 b + 3`, column first. The
level is a straight list of screens (world x / 256); the camera `$20:$1F` is kept between screens
`$14` and `$15` of the current section, and nothing is drawn past them (the next screens belong
to another section). Each row's centre is checked against the nametables at four columns. A
margin column the nametables still hold for that place (the rows written in the column's most
common frame all show the layout's tiles) is taken from the nametables, with what objects drew
into the background since (Air Man's cloud devils, Bubble Man's lantern fish, ...); any other
column comes from the layout.

**The sandbox** (`wide_decode/guest.rs`): a decoder can run a game's own routines on a `Cpu6502`
over a copy of the console RAM and the PRG-ROM (`$8000-$FFFF`, a fixed 32 KiB image, NROM). Writes
to anything but the RAM go nowhere, reads of it return 0, interrupts never come; `Guest::call`
runs a routine from an empty stack until it returns (to `$0000`) or a cycle cap. The console is not
touched. For a game whose level format is easier to run than to rewrite.

Super Mario Bros.: the game keeps no level map; its area parser (`AreaParserCore`, `$93FC`) reads
the area's objects as the screen scrolls and writes one column of 13 metatiles (`MetatileBuffer`,
`$06A1`, 16 x 16 pixels each, from tile row 4 down) a little past the right edge. The decoder runs
it in the sandbox from the start of the area: `InitializeArea` (`$8FE4`, halfway page and
entrance control set to 0, so from page 0); then the game's area data pointer (`$E7/$E8`, minus
the 2 header bytes) and type (`$074E`) are put back and the header read again (`$9C58`, within
`GetAreaDataAddrs`), since `AreaPointer` may name another area (the next one while Mario goes
down a pipe, anything on the title screen); then `AreaParserCore` and `IncrementColumnPos`
(`$92DB`) for each of 512 columns (32 pages). About 2 ms, once per area: the result is kept
(one area, process-wide) until the area's data pointer or type changes. A metatile is
`%ppnnnnnn`: sub-palette `p`, 4 tiles at `MetatileGraphics_Low/High[p]` (`$8B08`/`$8B0C`) +
4 `n` (top left, bottom left, top right, bottom right); the palette comes from the metatile, not
from the attribute table, which the game writes later than the tiles. Rows 0-3 (the status bar)
get empty margins. A row whose centre differs from the parse at more than one of four columns
(an intermediate screen, a transition) is left to the nametables. A margin column the nametables
hold for that place (Mega Man 2's rule: the rows that differ from the parse written after every
row that matches) is taken from them, with the bricks broken and the coins taken; any other from
the parse. The parse also records, per column, whether the area's own data locked the scrolling there (a scroll lock object): while the game's lock is on in such a room (an underground bonus room, whose area holds other rooms beside it), the margins stay empty; a lock that is not the area's (the flagpole, a fall) keeps them. The decoder works only while the title screen or the game runs an area (`OperMode` 0 or 1, task 3): the black WORLD screen shares its blank tile with the sky. Castle loops (4-4, 7-4, 8-4) send Mario and the screen 4 pages back, coherent with the straight area the parse lays out afterwards; for about a second around each loopback (before it, the game reading the repeated part ahead; after it, the screen's page below 0) the centre does not match the parse and the margins are left to the nametables, mostly empty. Not handled: the path is decided only when the parser reaches the loop point.

Adventure Island: the game draws a round one tile column at a time, 8 pixels past the right edge
(the column of world tile `16 $30 + ($35 & 15)`, the screen's left edge 264 pixels behind), from
tables its RAM points to for the round (`$1A`-`$27`). A round is a list of sections of 16
columns, 3 bytes each: a screen (the first byte's high six bits), the camera's vertical move (its
low two) and a slope (the second byte's high three); a screen is four structures of 32 pixels, six
bits each; a structure a top part (6 metatiles) and a bottom part (5); a metatile 4 x 4 tiles and
an attribute byte. A column is 56 tiles high (the top part's first metatile three times, its
other five, the bottom part's five and its last again). It is put on a plane of rows the camera
moves over, at the row `$31` holds when the game draws it, the nametables holding that plane
modulo 30 rows; a slope moves `$31` by a step every 8 or 16 columns (`$BE0D`, masks `$BE5D`,
steps `$BE63`), so the row of every column follows from the column being drawn. A line shows the
row of the plane congruent to its nametable row nearest `$32` (the top of the rows the game keeps
drawn) plus its line / 8. The game also puts things over the tables when it draws a column (the
cloud platforms of the sea rounds, `$A629`): a margin column it drew already (left of the column
being drawn) shows the nametables', each tile that differs from the tables kept only when written
in or after the older of the frames of its nearest rows above and below that match them (older
ones are another column's; the game also redraws whole rows as it scrolls vertically, newer than
the rest of the column), and
not when it is what the tables give 30 rows up or down (the game draws the rows the screen scrolls
to vertically over the screen's columns only: a margin column keeps there the ring's other row);
a column not written since a state was loaded counts as drawn (write frame 0, the state's own);
the column being drawn shows the game's column buffer (`$0460`) once it is complete (from its
third step, `$06 & 7 = 3`); a column not drawn yet shows the tables, with the platforms the game
will draw over them. Those are objects of kind `$3B` whose extra byte's low 3 bits are 0: created,
such an object asks for its drawing (`$052E` = 4, its extra byte in `$052F`), counted down at the
third step of each column; from the column where it reaches 0 the game writes two tiles over each
of 8 columns (`tbl_A80F[$8E]`, the second row 8 bytes on) at the plane's row `$32 + $8D`, `$8D`
from the ground's y under the first column (`$A607`: `sub_C1AD` at x 265, minus 16 plus
`tbl_A604[$2B]`, rounded, minus twice the extra byte's high nibble, in tiles, plus 1 for style 0),
and the sub-palette `tbl_A81F[$8E]` over attribute row `$34 + $0530`. The decoder shows the
platform being drawn (`$88` columns left, its tiles at `$89`), the one asked for, and those of the
round's three object lists read ahead as `$8C5A` reads them (`$7C` + list, `$0546`/`$0547`,
`$0540`; created when the screen's left edge comes 64 pixels near their position, so drawn from
world tile column position / 8 - 4), with the ground of the game's 5 parts of 128 pixels
(`$0506`, `$050B`, `sub_C1EA`) and past them of the round's sections (as `$BEDE` makes the next
part). The screen's vertical position is taken as it is now. A row whose centre does not show what the tables give, or
what the game drew over them, at four columns is left to the nametables (the title, the screens
between rounds). The vertical margin comes from the tables (past a column's top and bottom its
first and last metatiles go on, as the game repeats them within the column), while the screen's
nearest row wholly on it shows the round; the screen's first and last rows (across its top or
bottom edge, or ending on it: the game keeps there, for the overscan, a row of its ring that may
not be the plane's) are decoded whole width as the margins are, the nametables kept where they
show the round, while the next row towards the centre does.

Ninja Gaiden III: the game draws an area from PRG banks 0-3 (`$C01E`, fixed bank). Area `$5F`
(48 of them) starts at the address `$B960 + a` / `$B990 + a` of bank 1 gives (banks 0 and 1 seen
at `$8000-$BFFF`); bit 7 of `$A0F0 + a` (bank 6) makes it a vertical area (rows of 8 blocks, as
wide as the screen: nothing to show beside it, the decoder leaves it alone); the low nibble of
`$A1B0 + a` is its last camera screen, so it is `8 (n + 1)` columns of 32 pixels. A column is 6
blocks of 32 x 32 pixels from the top of the playfield (nametable rows 0-23; the status bar, rows
24-29, shows above it through the IRQ, and is left to the nametables). Block `b` is 4 sub-blocks
at `($51) + 4 b` (top left, top right, bottom left, bottom right), its attribute byte at
`($53) + b`; sub-block `s` is 4 tiles at `($55) + 4 s`, in the same order; those pointers point
into banks 2 and 3, seen at `$8000-$BFFF`. The camera is `$DC:$DB`. The IRQ scrolls bands of the
playfield on their own (skies, mountains, foregrounds): such a band shows the ring of 64 tile
columns as the game drew it around the camera, so a row whose centre is not the area at the camera
is tried as that ring, each column the layout's column of its ring position nearest to the camera
(within the area). A row is decoded when 20 of its 32 centre columns show the layout's tile (the
game animates some tiles, and a band may show a seam where the ring was drawn at two moments); a
margin column the nametables hold for that place is taken from them (Mega Man 2's rule). A row of
the area the layout does not explain (the desert's dunes, which the game shifts in the nametables
to animate them; the upper layer of 3-1A) takes its margins, 16 columns a side, from the ring of
nametables beside the screen. Past the ends of the area the margins repeat its first or last 32
pixels (owner request: no black hole beside a band that goes on).

Super Mario Bros. 3: the game lays the whole level out in PRG-RAM (`Tile_Mem`, `$6000-$794F`)
before it starts and keeps it up to date (blocks hit, coins taken, bricks broken). A horizontal
level is up to 15 screens of 16 x 27 tiles of 16 x 16 pixels, screen `s` at `$6000 + $1B0 s`, row
first. Tile `t` is the CHR tiles `layout[256 q + t]` of quarter `q` (top left, bottom left, top
right, bottom right), its sub-palette `t >> 6`; the layout of tileset `$070A` is the pointer at
`$99F7 + 2 n` of PRG bank 30 (the game's bank before the last, fixed at `$8000`; the decoder finds
it 16 KiB before the end of the PRG-ROM, where it is with or without the built-in patch), in bank
`PAGE_A000_ByTileset[n]` (`$83E9`) or, for a pointer at `$C000`, `PAGE_C000_ByTileset[n]`
(`$83D6`). The 27 rows are the 54 tile rows of the plane of two stacked nametables (horizontal
mirroring), from its top; the camera's y in the level is `Vert_Scroll` (`$FC`) and picks the
instance of a row, its x `$12:$FD`. Rows past the level (the status bar, below line 192) and a row
whose centre differs from the tile map at more than one of four columns (the title, the bonus
games, a transition) are left to the nametables; the vertical levels (`$03EF` non-zero, four
pipe mazes) are not decoded yet. A world map is in the same tile map (rows 16-26, at most four
screens, the screens past it filled with tile `$02`) with the layout of tileset 0 in bank 12 (the
page table's entry for it is not used): the next screen of the map shows past its frame. The
vertical margin above the screen is decoded once a row halfway down the playfield matches; the one
below stays empty: it is under the status bar, and the level there stuck to the status bar made a
3D profile that tells the status bar by its shape miss it every other frame (its clipping, and the
2D view framed by it, jumped). Column 0 is decoded too
while the game hides it (see 5), and the last 3 columns of the centre are redrawn (see 4).

Castlevania: the two USA builds share their data banks 0-4 byte for byte (only the code, banks
5-7, was rebuilt), so one decoder serves both, given where its build keeps two tables of the fixed
bank (`castlevania::USA` and `USA_REV1`: Rev 1's are `$24` bytes further). The game draws the room
in play from PRG bank 4 a 32 x 32-pixel block at a time (`$F67A`, base build). Room `$46` of stage
`$28` has its map at entry `$46` of the list the maps table (`$FAFF`, Rev 1 `$FB23`) points at for
the stage; the decoder reads it there as `$CFD4` does, the game keeping it in `$33` only once the
camera moves (a stage starts with the right margin empty otherwise). A map is 6 block numbers a
column, top first, room x `32 c` at `map + 6 (c + 4)`; block `n` of
the stage (`$28`) is 17 bytes at the pointer of entry `$28` of the blocks table (`$F9D6`, Rev 1
`$F9FA`) `+ 17 n`, its attribute byte (the block is one attribute cell), then its 16 tiles row
first. The map's first block row is nametable row 4; the status bar keeps rows 0-5, so the rows
decoded are 6-27. A room ends a screen past the last screen of its camera, the nibble of the
lengths table (`$FB8C`, Rev 1 `$FBB0`) for the stage (the high one for room `$46` 0, the low one
for room 1, as `$CFF0` reads it); past both ends the margins stay empty. A room may open on a
screen of filler behind a wall, which the camera never shows (stage 4): its map holds block
numbers past the stage's blocks there (a set ends where the next set of the table starts), and
such leading screens are not the room either. A margin column within 64 pixels of the screen is
taken from the nametables when 8 of its 22 playfield rows show the map (the game draws a column 96
pixels ahead of the right edge a block at a time and keeps those behind the left one, so there
every column is one it finished drawing for its place): what the game redraws into the background
shows there too (the crushers of stage 6 going down, a wall broken); further out the margins come
from the map. The camera is `$2F:$2E`;
some stages draw room x 0 in the right nametable (the table `$FBB2`, Rev 1 `$FBD6`: the camera is
then 256 pixels away on the ring of two nametables; read there, not guessed, since the rows of
those stages repeat and both ways can match), and a row is decoded only when 24 of its 32 centre
columns show what the map gives there or are such a column of the nametables (crushers or a wall
broken in the centre leave the row decoded; the title, the intermissions and the transitions are
left to the nametables).
The built-in patches of both builds (`CASTLEVANIA_USA`, `CASTLEVANIA_USA_REV1`, an advanced
mode) are one hack built for each (`cvwide`): its code, in the free end of PRG bank 5 in the
hack's own build, is the pack's code (7.1), open in the window from power-on (`boot_window`): the
fixed bank has no room to unlock the extension. It draws the objects in the margins: an object's x is 9-bit there (bit 0 of
`$0300` its bit 8, 256-383 right of the screen and 384-511 left of it, as the game's own `$EF45`
reads it), the game keeps its candles and spawners' objects alive from 64 pixels left of the
screen to 64 right of it, and the hack draws them past the edges (the sprite builder of bank 0,
16-bit sprite x and the high bytes at `$5100`), within the room. The enemies the game removes at
the edges of the screen (most of them: 8-bit x, moved by their own routines) are followed in 16
bits and live on up to the objects' margin (`min(M + 16, 120)`); one made at an edge by a spawner
comes in from it; past an edge an object looks for its ground at the edge (the game's terrain is a
ring that follows the nametables, drawn only where the game has drawn). The game's collisions with
Simon only take objects on the screen (`$0300` 0), so nothing in a margin touches him; Simon's
weapons (slots `$14`-`$16`, which the game checks against the objects in 8 bits, `$E5C7`) touch
nothing either while past an edge, so a dagger flying on into a margin cannot hit an object at the
other edge. Weapons and the enemies' shots go on through the margins (the cross keeps the game's
bounds: it reaches an edge only at the end of a room, and turns back there); an enemy only shoots
while on the screen (the game's own check before `$ED38`). An object made at another's place
(`$ED5E` and three more copies: shots, pieces, points) takes its x's bit 8 too, so one made by an
object past an edge is made there. The objects the game keeps in 9 bits (`$EAB9`) are no longer
hidden within 8 pixels of the edges, and go as far as the objects' margin when that is past the
game's 64 pixels. The moving platforms (stages 4, 5 and 10, slots `$17`-`$1A`, bank 6 `$AB1F`)
have their place on the screen in 16 bits, from which the game puts an 8-bit x and, near an edge,
only the piece of them on the screen (its x wrapped: a platform past the left edge showed at the
right one too): the hack keeps the 16-bit x for the sprite builder and, within the view, draws them
whole. Options:
testing aids, off by default (Select in a stage goes on to the next stage through the state that
begins one, with its block's graphics; Simon never hurt), and "No slowdown" (255 lines of
overclock, on by default; the yield ends it in the main loop, `idle_loop` `$C030`-`$C03E`). The
game's split under its status bar is a sprite 0 wait early in the NMI, far from the overclock (at
line 240) and before the hack's code. Measured on a movie of stage 1 and its hall, 5,400 frames
at a 112-pixel margin: the original game lags only in a room change (the same with the hack), the
hack without the overclock some 110 frames more, none with it.


Excitebike: only the track (tile rows 8-21) changes as a race scrolls; the stands, the sky with
its signs, the grass and the status bar are drawn once. The game draws the track one tile column
at a time (`$F4FF`, once a frame from the race's loop, when `$EB` reaches the player's column
`$E0`), 14 tiles from row 8 at the column `$EA:$E9` of the ring of two nametables side by side
(64 columns), some 19 columns past the screen's right edge. The track is a stream of bytes (at
`$ED3A/$ED40` + the track `$43`; the design mode's own in RAM) of pieces and repetitions, each
piece a list of columns; the routine keeps its place in `$ED` (the byte), `$EC` (repetitions
left), `$EF` (the piece's column) and `$EE` (the piece), and starts the stream again past the
finish line. The decoder runs it in the sandbox from the state a race starts with (`$C869`
clears `$4C-$EF`; `$E8-$EB` as `$C859` sets them; the lap counters 0), calling it with `$E0` set
to `$EB` and reading the column from the VRAM buffer (`$0304`-`$0311`), until the stream's state
comes back to the second column's (the first lap alone starts by reading the lap count): the
race's first column and one lap, kept until the stream changes; before or past them, a column is
the same place of the previous or next lap, but in the first lap (`$03A4`, the finish lines
passed, 1) the columns before the race's first are the start's screen, drawn before the race:
the nametables', and left out of the check below. The next column the game
draws is the lap's column whose state is the game's now; a track column within the 64 behind it
is the nametables', one past it the lap's (going round). The other rows show the ring of
nametables around them, the status bar's background goes on beside it, and only in a mode that
runs a race (`$41` 2, 5, 9 or 10: `$C99A`, `$C97C`) whose track centre shows the lap's columns
(three of four checked on row 15).
Final Fantasy: both kinds of map are grids of 16 x 16 tiles drawn through the tileset the game
keeps in RAM (`$D1E4`): tile `t` is the CHR tiles `$0500 + t` (top left), `$0580 + t` (top right),
`$0600 + t` (bottom left), `$0680 + t` (bottom right), its attribute byte (the same sub-palette in
its four quarters) `$0700 + t`. The world map (`$2D` bit 0 clear) is 256 x 256 tiles, a torus,
run-length encoded by rows in PRG bank 1 (`$D157`): row `y` at the pointer `$8000 + 2 y`, going on
into bank 2 past `$BFFF`; a byte below `$80` is one tile, `$80 + t` and a count `t` that many times
(0: 256), `$FF` ends the row. The game unpacks only the 16 rows around the screen
(`$7000 + 256 (y & 15)`); the decoder unpacks any row from the ROM, so the vertical margin goes on.
A standard map (`$2D` bit 0 set: towns, castles, dungeons) is 64 x 64 tiles at `$7000 + 64 y + x`,
the game's copy (chests opened), wrapping as the game reads it. The game writes its scroll as
`16 $27 + $35` across (`$29` in a standard map) and `16 $2F + $36` down (`$C346`, `$CCA6`), `$2F`
being the nametable row (0-14) of the camera's top tile, `$28` (`$2A`) its map row: the screen is
the plane of two nametables side by side, the map's x modulo 512 pixels, and the map's rows are the
nametable's from row `$2F`, modulo 15. A row is placed by its PPU address, the camera picking the
instance; a frame whose centre shows the map at four columns of one of three rows across the screen
(lines 25, 121, 217) is decoded, any other (a battle, a menu, the title, a transition) is left to the
nametables. The game shows a dialog box over the map with a scroll of its own for the box's lines:
a row whose PPU address does not place the map is placed as the camera in RAM says, and gets the
map in its margins. All 15 rows of the nametable ring show: scrolling vertically the game draws the
row coming in over the leaving one, at the screen's top or bottom edge, so the decoder covers the
overscan (`covers_overscan`). A room (`$0D`) only changes the palette (`$03E0`) and which people
show: its tiles are the map's. The people, the vehicles, the bridge and the canal are sprites,
which the game leaves out past the edges: the built-in patch (`FINAL_FANTASY_USA`, an advanced
mode, `ff1wide`) draws them in the margins. Its code, in the free end of PRG bank 6 in the hack's
own build, is the pack's code (7.1), open in the window from power-on (`boot_window`); the window is
not laid over `$6000-$7FFF`, where the game keeps its save and maps. Where the game puts a person
(`$E70D`) or a vehicle (`$E3F4`) on the screen, the hack works out a 16-bit x (the distance in
tiles signed) and keeps it within the host's margin; the sprite builder (`$E301`) writes the high
bytes. Once a frame, at the game's wait for the NMI (`$FEA8`), it sets `$5003` to the host margin,
the extents to 0 and `$5007` to 2. A sprite's y stays 8-bit: the people above and below the screen
stay hidden. Option: "No slowdown" (bit 0, on by default; `idle_loop` `$FEBB`-`$FEBD`, the game's
`JMP` to itself). Measured with a trace of the waits: 600 frames each of a town, the world map
and a battle, at 0 and 128 pixels of margin, lag no frame, with or without the hack.

Metal Force: the game unpacks the whole level into its 8 KiB of PRG-RAM and draws the screen's
columns from it (`$E3DB`, pointer `$E529`). The map is at `$6000`, `$1800` bytes of 16 x 16
metatiles, column after column; a column is `H` metatiles high, `H` = 16, 24, 32, 48, 64, 80, 96,
112, 128, 144, 160 for the level shape `$0352` = 0 to 10, 40 past it, and its rows count upwards
(row `m` is above row `m - 1`). Metatile `n` is 5 bytes at `$7800 + 5 n`: CHR tiles top left, top
right, bottom left, bottom right, then its sub-palette (low two bits). The camera is `$35:$34`
(world x of the screen's left edge) and `$3D` (the map row at the screen's bottom). The game shows
one nametable (one-screen mirroring), a ring of 32 x 15 metatiles where map row `m` is metatile row
`(14 - m) mod 15`: a row's column comes from the PPU's coarse x (modulo 256 pixels, the instance
from `$35:$34`), its map row from its coarse y (modulo 15, the instance from `$3D` and the line).
Each row's centre is checked against the nametable at four columns; a mission screen or a
transition is left to the nametables. Rows above and below the screen (`decode_extra_row`) are
decoded the same way, once the nametable row they would read matches the map one ring away (the
screen shows the whole ring). The game writes the blocks the player breaks back into the map, so
the margins show them broken. A level is made of rooms: the game keeps the camera within
`$0356:$0355`-`$035A:$0359` (x) and `$0358:$0357`-`$035C:$035B` (y of the screen's bottom,
upwards), and the map past them holds the other rooms; the decoder draws the room only (x up to
the last camera x + 256, y up to the last camera y + 240), the rest of the margins stays empty.
The one nametable makes the screen's edge tiles show the column the game
is drawing for the other side (the left one hidden by PPUMASK, the right one partly visible: the
33rd tile of a line; both at the middle of a wide view, which the TV's overscan hid): column 0 is
decoded while the game hides it (`decodes_masked_column`) and the last column of the centre is
redrawn (`redrawn_right_columns` 1). The game
needs mapper 156 (Daou Infosys DIS23C01,
`cartridges/mappers/daou306.rs`, as Mesen's `DaouInfosys`).

Blaster Master: an area is 2048 x 2048 pixels (8 x 8 screens of 256), drawn by the game's column
and row drawers (`$DA15`, `$DAC4`) from four levels of tables in PRG-ROM, whose pointers `$D975`
copies from the area's header into `$27-$30` (the header is entry `$C9F4[$14] & 15` of the
pointers at `$8000` of bank `$C9F4[$14] >> 4`, `$EA3C`; the tables are in that bank). The map
(`$2F/$30`) is 32 x 32 blocks of 64 pixels, row first; a block is 4 blocks of 32 at `($2D) + 4 b`
(top left, top right, bottom left, bottom right), each of them 4 metatiles of 16 at `($2B) + 4 n`,
each of them 4 CHR tiles at `($29) + 4 m`, in the same order; metatile `m`'s sub-palette is the
low two bits of `($27) + m` (the game packs them into the attribute bytes, `$DC47`). The camera is
`$1D:$1C` and `$1F:$1E` in sixteenths of a pixel (the screen's top left corner); the game shows two
nametables side by side, a ring of 512 x 240 pixels at a fixed offset from the world (the scroll
`$FD`, `$F1` bit 0 and `$FC` follow the camera's moves, `$D1AD`), so a row's place comes from its
PPU address, the offset `scroll - camera` and the camera picking the instance. A row is decoded when
28 of its 32 columns show the map (a few may show a door or a block the game changed; a few spread
columns would not do: around a boss the game blanks the nametables where the map has tiles every
few columns), or else its first four and last four columns do, in more than one tile (the game
draws some rooms of the overhead view over the map); the title, the area's name, the pause screen
and the transitions are left to the nametables. The camera stays within a room: the 16 bytes entry
`$CA04[$14]` points to (`$CC6D`) close the boundary left of screen `(x, y)` (byte `8 + y`) or above
it (byte `y`), bit `7 - x`, and the camera stops in the middle of a screen whose boundary ahead is
closed; the margins show the screens the view touches that the camera's own screens reach through
open boundaries (the same room), the rest stays empty; in the overhead view (areas `$14` 0-7) the
camera goes across closed boundaries, so the margins show every screen. The ring is 30 rows, all on the screen:
scrolling vertically the game draws the row coming in at the other edge, in the overscan, so the
decoder covers it (`covers_overscan`). A row at or past the screen's top or bottom edge is checked
on the screen's nearest row that reaches neither edge (its top within lines 1-231), the same
distance away on the ring (its margins, decoded with the other rows, as well as its centre): the
edge row itself may be the game's overscan row, a stale one or one
whose attribute bytes are another row's (the bottom row then shows the rocks in the sky's colours,
a band of "water" a television hid). Those attribute bytes also colour the row next to the edge row,
on the screen: the rows starting or ending within 8 lines of the edges are decoded whole width too
(`overscan_lines` 8).

Mega Man: the first game's stages are laid out as Mega Man 2's (`megaman.rs` holds what the Mega
Man decoders share: the check of the centre, the section, the nametable columns the game still
holds for their place, and the 32 x 32 structures of four 16 x 16 blocks, column first). A stage
lives in PRG bank `$31` (Wily's stages, 6 and up, in bank `$31 - 6`), as the game's column drawer
(`$CE5A`) reads it: screen `s` is room `$8C00[s]`, whose 64 structure bytes (8 columns of 8, column
first) the pointer at `$8C30 + 2 room` gives; structure `n` is 4 block bytes at `$8000 + 4 n`, its
attribute byte at `$8300 + n`; block `b` (the low six bits) is CHR tiles `4 b` to `4 b + 3`. The
camera is `$1B:$1A`, the section being played screens `$27` to `$28`; nothing is drawn past it.
While the game scrolls to another section (`$26` non-zero) the camera is already on the new
section's screen while `$27`/`$28` still give the old one: the margins stay empty (Mega Man 3 to 5
change the camera and the section together, but the rows on the screen while they scroll belong to
both sections: their decoders draw nothing while the vertical scroll `$FA` is not 0, and Mega Man 6
decodes nothing off a screen's top). During such a scroll the game draws the rows coming in at the screen's
top or bottom edge a piece at a time, which a television hid: the Mega Man decoders ask the
screen's top and bottom 8 lines hidden with a vertical margin as without one
(`MarginDecoder::hides_overscan`, see section 6).
The built-in patch (`MEGAMAN1_USA`, an advanced mode, `mm1wide`) draws the enemies in the margins.
Its code is a 16 KiB PRG bank added to the game (bank 8 of 16, the fixed bank copied to bank 15,
the one UxROM fixes), open in the window from power-on (`boot_window`). Options: "Invincible" (a
testing aid, off by default) and "No slowdown" (on by default; `idle_loop` `$C02B`-`$C02E`, the
main loop's `LDA $1F / BEQ`). Measured on Ice Man's stage at a 128-pixel margin, 200 frames: the
original lags no frame, the hack without overclock 100, with 64 or 255 lines none.

Mega Man 3: the stage lives in the 8 KiB PRG bank the fixed bank's table at `$C8B9` gives for the
stage `$22` (the decoder reads that table 16 KiB before the end of the PRG-ROM, where it is with or
without the built-in patch), which the column drawer (`$E882`) maps at `$A000`: screen `s` is room
`$AA00[s]`, 64 structure bytes at `$AF00 + 64 room`, row first (8 rows of 8, as the terrain lookup
`$E913` reads them); structure `n` is 4 block bytes at `$B700 + 4 n`, row first (top left, top
right, bottom left, bottom right); block `b` is CHR tiles `$BB00[b]`, `$BC00[b]`, `$BD00[b]`,
`$BE00[b]` (the same corners), in sub-palette `$BF00[b] & 3`. The camera is `$F9:$FC`; the section
being played is `$2C + 1` screens, the camera on its screen `$2D`. At the end of a section, before a
vertical transition, the game switches the mirroring and both nametables show the same 256 pixels:
the shared decoder places a row at the instance of its pixel nearest the camera modulo 256 pixels
(on the plane of two nametables both instances can lie 256 pixels from the camera, and a row whose
centre happened to match the wrong one had its margins 256 pixels off). While both nametables are the same
(a row's 32 bytes equal on both sides), no margin column is taken from them: the margin's nametable
column is a copy of one on the screen, which a repeating wall makes look like the layout's. A margin column the
nametables hold for its place takes the layout's colours (the game writes a column's attribute bytes
after its tiles), and the Mega Man 1, 3 to 6 decoders accept a row whose centre shows the layout at three of the four
checked columns (a boss's shutter, which the game draws into the background, may stand on one: its
rows' margins went black while the camera scrolled through it). The Mega Man 3 to 5 decoders redraw the screen's last column
(`redrawn_right_columns` 1): it showed the colours of the column being drawn past the edge.
The built-in patch (`MEGAMAN3_USA`, an advanced mode, `mm3wide`) draws the enemies in the margins.
Its code is a 16 KiB PRG bank added to the game (8 KiB banks 32-33 of 64, the fixed banks copied to
banks 62 and 63, the ones MMC3 fixes), of which the window shows the first 2 KiB from power-on
(`boot_window`; its code sets `$5007` without bit 4: the board may have PRG-RAM at `$6000`).
Options: "Invincible" (a testing aid, off by default) and "No slowdown" (on by default; `idle_loop`
`$FEAD`-`$FEC2`, the task scheduler's wait). Measured on Snake Man's stage at a 128-pixel margin,
200 frames: the original lags 11 to 33 frames, the hack with 64 lines 26 to 99, with 255 none.

Mega Man 4 keeps Mega Man 3's layout (`megaman3::Layout`, the decoder of both) at other addresses:
the stage is the 8 KiB bank `$20 | $22` at `$A000` (`$D295`), rooms `$B500`, layouts `$A900`,
structures `$A500`, blocks' corners `$A000`-`$A300`, sub-palettes `$A400`; the section is `$2D + 1`
screens, the camera on its screen `$2E`. Both builds (they differ in three bytes) take the same
decoder and the same hack (`MEGAMAN4_USA`, `MEGAMAN4_USA_REV1`, `mm4wide`): Mega Man 3's, with
Mega Man 4's 24 object slots (the enemies from slot 8), its sprite loop's own cases, and the stage
list's palette changes (types `$C0` and up) kept waiting for the original window. Its code is a
16 KiB PRG bank added to the game (8 KiB banks 64-65 of 68, the fixed banks copied to banks 66 and
67: Zaxnes' MMC3 fixes the last two whatever their count), of which the window shows the first
2 KiB from power-on (`boot_window`). `idle_loop` `$FEA8`-`$FEBC`. Measured on Pharaoh Man's stage
at a 128-pixel margin, 200 frames: the original lags 0 to 2 frames, the hack with 64 lines 1 to
29, with 255 none.

Mega Man 5 keeps that layout too: the stage is the 8 KiB bank `$27` at `$A000` (`$D758`; while
`$26` is `$0E`, each screen's own bank from the fixed bank's table at `$D793`: `StageBank::Ram`,
the bank looked up per screen), rooms `$A900`, layouts `$B600`, structures `$B200`, blocks'
corners `$AD00`-`$B000`, sub-palettes `$B100`; the section is `$2A + 1` screens, the camera on its
screen `$2B`. The Mega Man 3 to 5 decoders draw column 0 while the game hides it
(`decodes_masked_column`): Mega Man 5 keeps there the column it is drawing. The built-in patch
(`MEGAMAN5_USA`, an advanced mode, `mm5wide`) is Mega Man 4's hack at Mega Man 5's addresses (its
sprite loop mirrors the offsets of flipped sprites); its code is a 16 KiB PRG bank added to the
game (8 KiB banks 32-33 of 36, the fixed banks copied to banks 34 and 35), of which the window
shows the first 2 KiB from power-on (`boot_window`). `idle_loop` `$FEAE`-`$FEC3`. Measured on
Charge Man's stage, 500 frames: the hack at a margin of 0 without overclock lags 50 frames, at a
128-pixel margin with 64 lines 23, with 255 none.

Mega Man 6 has an engine of its own. The decoder reads where the stage is from the pointers the
game keeps in RAM for its column drawer (`$D1FB`; map lookup `$D8AB`, `$D900`), the 8 KiB banks
`$6E` at `$8000` and `$6F` at `$A000`: the map at `($52)`, one 256-byte page a row of 32 x 32
structures (8 rows a screen), the screen's 8 structures at `8 s` in the page (wrapping in it), `s`
the camera's screen x plus its screen y (a stage is a straight list of screens, going right or
down); structure `n` is 4 block bytes at `($66) + 4 n`, row first; block `b` is CHR tiles
`($62)[b]`, `($62)[256 + b]`, `($62)[512 + b]`, `($62)[768 + b]` (top left, top right, bottom
left, bottom right), in sub-palette `($62)[1024 + b] & 3`. The camera is `$57:$56`, `$59:$58`; only
rows of a camera on a screen's top are decoded (a vertical scroll leaves the margins to the
nametables). A stage is a straight list of screens, the next one beside, above or below: the
margins show the screens beside the camera's, as far as a join that is not (past it, black). The
stage's controllers (type `$80` in the stage list, bank `$3A` through the pointers
`$74`/`$78`/`$7C`/`$7E`) say which: their settings (bank `$39`, `$82EB`/`$836B` by the entry's
y) are `$71`, two bytes, then actions (kind, `$73`, `$72`, a screen of their own with bit 7): 1
and 3 scroll down to the next or previous screen, 2 and 4 up (ladders), 5 and 6 through a door
to a screen. Without a screen of their own they act from the controller's screen, a controller
that scrolls to the next screen taking its scroll back from that next one; with one, a door
leaves its join apart; without, a door does too when it loads the stage's other tiles (its `$73`
with bit 7, a set of background tiles the game keeps in `$064F`: a machine room's tiles swapped
for the stage's at its door out, the room past the door showed as a jumble, a ghost of the
machine); settings with `$71` past 3 or a kind past 6 (a boss's) say nothing.
Checked on movies of a stage against every change of screen they show (23 joins, 15 beside, 8
not). `$71` (bit 0 right, bit 1 left: where the camera may go now) no longer bounds the margins:
a screen beside past a lock shows. Two raster effects show rows that are not the stage at the
camera, decoded as a layer that goes on as the nametables do: the sunset's band of sky (`$F4` on,
handlers from 14 on in `$0677`, the line after `$0672`, 62 lines: scroll 0, then 0 or 1 every 3
lines, the shimmer) and the empty band above the submarine (rows the game shows from the other
nametable than `$FA` bit 0's, pushing the stage's lower rows down as it rises and sinks); before,
the stage's layout there showed a wall beside the sun and blocks of ice afloat. Column 0
is decoded while the game hides it, and the screen's last column redrawn (`redrawn_right_columns`
1), as in Mega Man 3 to 5. Its first two columns are redrawn too (`redrawn_left_columns` 2): the
column the game draws past the right edge wraps on a single nametable's width over the screen's
first columns, and with a fine X the hidden 8 pixels take part of column 1 as well, which showed
that column's colours (a stripe at the left edge of the original picture). The built-in patch (`MEGAMAN6_USA`, an advanced mode,
`mm6wide`) draws the enemies in the margins: Mega Man 6's objects run as scripts that remove
themselves through the test "on the screen" (`$E485`), which the hack widens for the enemies (not
for the stage's controllers, type `$80` in slot 22, which keep the original spawn window: the stage
list is walked twice, the original's walk for them, a wider one with its own pointers for the
enemies); an enemy asleep in a margin waits in a reserve of the hack's, out of the game's object
slots (the hack draws it in a free or borrowed slot), and takes a slot again on the original
screen; the code runs from a window over `$5800-$7FFF` (`$5007` bit 4: the game has no PRG-RAM); an enemy asleep in a margin runs its script's
first frame, which sets up how it looks, and one spawned while the camera stands (the stage's start,
its tiles not yet in CHR-RAM) shows once the camera moves, one asleep past the next screen of the one
whose tiles the game loaded (`$0680`) not before, and an object the game flags not drawn (`$03B7` bit
0) is not drawn in a margin either; its
sprite loops are the game's four (none, horizontal, vertical, both flips) for each page. Its code
is a 16 KiB PRG bank added to the game (8 KiB banks 64-65 of 68, the fixed banks copied to banks 66
and 67), of which the window shows the first 10 KiB from power-on (`boot_window`, `$5800`-`$7FFF`:
the hack keeps `$5007` bit 4; the game has no PRG-RAM). `idle_loop` `$C581`-`$C584`. Measured on Plant Man's stage, 500 frames: the game with the hack's sprite
routines left out lags 19 frames; the hack at a margin of 0 without overclock 75, at a 128-pixel
margin with 64 lines 62, with 255 none.

Metroid: the world is a map of 32 x 32 rooms of one screen (256 x 240), room numbers at `$7000`
in PRG-RAM (`$FF`: none), each an entry of the area's room table; the area's tables are in its PRG
bank (`InArea` `$74` low nibble through `$CA30`: Brinstar 1, Norfair 2, Kraid 4, Tourian 3, Ridley
5), their addresses copied to `$3B` (rooms), `$3D` (structures) and `$3F` (metatiles). The game
builds a room into PRG-RAM (`SetupRoom`, `$EA2B`: `$6000` for nametable 0, `$6400` for the other,
tiles then attribute bytes as in a nametable) and copies it to the PPU a column or a row at a time.
A room's data is its default sub-palette (the attribute bytes' fill), then objects until `$FD` or
`$FF`: a position `%yyyyxxxx` in metatiles, a structure (read with an 8-bit index) and its
sub-palette (set in the attribute bits when it is not the default, `$EF9E`); a structure is rows of
metatiles, a byte `%xxxxnnnn` (offset in metatiles, count, 0 for 16) then the metatiles, until
`$FF`, a row cut where it would wrap and a structure stopped past room offset `$3A0` (`$EF13`); a
metatile is 4 CHR tiles at `($3F) + (4 n & $FF)`, upper left, upper right, lower left, lower right.
The game scrolls one way at a time (`$49`: 0 up, 1 down, 2 left, 3 right): across, two rooms side
by side (a ring of 512 x 240), or up and down, two stacked (256 x 480). `MapPosX`/`MapPosY`
(`$50`/`$4F`) is the room scrolled into: the second of the two on the screen scrolling right or
down, the first scrolling left or up; `$FF` (both nametable bits toggled together) gives the
nametable of the screen's left or top edge, `$FD`/`$FC` the scroll; so a row's place comes from its
PPU address, the ring's offset from the world and the camera picking the instance. The map's rooms
belong to five areas with their own tables, and the map does not say which: the margins show the
rooms of the area in play only, those its starting room (`$95D7`/`$95D8` of its bank) or the
screen's room reach (worked out once per screen's room) through the edges two neighbours share a
way through (a tile at or past `$A0`, or a block the area's bombs blast, `$D651`, on both sides),
up and down only between two rooms walled on the left and the right (shafts: a corridor's ceiling
may happen to meet a room of another area built with this one's tables), never across an elevator
room's top or bottom. The margins show the map's rooms around the screen, those behind a door too; the room on the screen's left (top)
edge, and the other one while both show, are read from PRG-RAM (blasted blocks included; not a
room the game is still building, `$5A` below `$F0`, on the nametable at the scroll direction), the
others built from the area's data as `SetupRoom` does (kept, by bank and data address). A row is
decoded when 28 of its 32 columns show the room's tile (the title, the password screen and the
transitions are left to the nametables), checked on the screen's nearest row that reaches neither
edge; scrolling up and down the game draws the row coming in at the edge (`covers_overscan`).
Scrolling across, the game draws the column coming in at the screen's edge (`$E70C`) with
attribute bytes that cover two columns, so the visible column next to it shows the incoming one's
colours for a moment: the decoder redraws the two columns at each edge (`redrawn_left_columns`,
`redrawn_right_columns` 2), from the room in PRG-RAM.
An area has two room palettes, 0 and 5 of its palette pointers (`$9560`; `$76` holds one plus the
one in use); the game toggles them when Samus goes through a palette change room (special item
`$0A` of the area's table at `$9598`, `$EEFA`), leaving it by the other door than the one she came
in by (`$8C84`; `$91` keeps the side she entered from). The rooms past such a room, on its other
side from the screen's room (Brinstar's `(12, 7)`, Norfair's `(22, 17)`), are handed back in
sub-palettes 4-7, and the decoder's margin palette (`margin_palette`) is the area's other room
palette: its palette 0, with palette 5's PPU strings over it for 5.

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
  `(margin, margin_y)`. `video.hide_overscan` (the screen's top and bottom 8 lines drawn black,
  and their tiles left out of the 3D conversion) applies to a picture with a vertical margin only
  when the frame's margin decoder asks for it (`MarginDecoder::hides_overscan`, false by default:
  a vertical margin goes on past those lines; true for the Mega Man decoders, whose vertical margin
  stays empty while the game draws rows there a piece at a time). The core latches it per frame
  (`Ppu2C02::hides_overscan`, `NesCore::hides_overscan`), the session carries it with the
  picture it keeps while the core is away (`GameSession::nes_picture_hides_overscan`), and
  `hide_overscan_lines` blacks those lines and the vertical margin past them (the picture ends
  where it does without a vertical margin: only the rows across the screen's edges the PPU drew
  reached into it). Both backends (GL `gl/picture.rs`, wgpu `wgpu_backend/resources.rs`)
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
* `BuiltinPatch::advanced`: an **advanced mode** of a game's widescreen (Super Mario Bros.: the
  enemies in the margins; Adventure Island: the sprites and objects in the margins; Final Fantasy: the people and vehicles in the margins). Such a patch applies only when the game's `video.widescreen_advanced`
  is on too, which it is by default (owner request: the widescreen is the whole hack, its
  testing aids aside; a game keeps a value of its own only when it is turned off)
  (`NesCore::set_advanced_patches`, set with `set_builtin_patches` at each load and
  restore); off, the ROM loads as it is and its margin decoder alone widens it. The hacks screen
  shows the row "Advanced mode" for such a game (`HackRow::Advanced`; on, it turns the widescreen
  on too), and the options of an advanced patch only while it is on. A state is "the hack" or not
  as for any patch; loading one made in the other mode switches the advanced mode (the widescreen
  stays on).
* `$5007` bit 4: the PRG window goes on over `$6000-$7FFF`, 10 KiB from `$5800`, for a game
  with nothing there (no PRG-RAM; the cartridge never sees those reads). A boot window sets it,
  unless its patch keeps the game's PRG-RAM (`BuiltinPatch::boot_window_keeps_prg_ram`, Metroid:
  the game reads its PRG-RAM before the hack's code has run, and the title screen took a byte of
  the window for Samus' suit, `$69B3`, loading her suitless graphics).
* `BuiltinPatch::boot_window`: `(bank, page)` of the extension's PRG window, opened at power-on
  and reset with the extension unlocked (`WideExtension::boot`), and again after a restore that
  left it locked: for a ROM with no room for the code that would unlock it, whose reset or NMI then
  jumps straight into the window.
* `BuiltinPatch::idle_loop`: first and last address of the game's wait for the next frame (Mega
  Man 2: `LDA $1D / BEQ` at `$C08E`; Super Mario Bros.: its main loop `JMP $8057`; Zelda II: its
  main loop `$C010`-`$C01A`, everything else running in the NMI; Ninja Gaiden III: `BIT $15 / BPL` at `$9A87` of bank 4; Adventure Island: `LDA $0B / BNE`
  at `$B759`; Final Fantasy: `JMP $FEBB` to itself, the NMI returning to the wait's caller; Metroid: its main loop's wait
  `TAY / LDA $1A / BNE / JMP` at `$C0CB`-`$C0D2`; Mega
  Man: `LDA $1F / BEQ` at `$C02B`; Mega Man 3: its task scheduler's wait at `$FEAD`-`$FEC2`; Mega Man 4: at `$FEA8`-`$FEBC`; Mega Man 5: at `$FEAE`-`$FEC3`; Mega Man 6: `LDA $F2 / BEQ` at `$C581`). `NesCore::run_frame` ends the
  overclock (`WideExtension::end_stall`) as soon as the CPU is there: the cycles left would only
  run the wait loop, so the game is the same, and a frame that does not need the overclock costs
  nothing more ("yield", owner request). Super Mario Bros. at 128 pixels with 255 lines: 1.38 ms
  per frame of core time with the fixed overclock, 1.11 ms with the yield, as without overclock;
  Mega Man 2's Air Man run keeps no lag frame (232 at 64 lines). Super Mario Bros. 3:
  `GraphicsBuf_Prep_And_WaitVSync`'s `LDA VBlank_Tick / BPL` at `$96FB`, where every loop of the
  game waits; 1500 frames of 1-1 with the enemies in a 112-pixel margin lag 30 times (3 in the
  original game), none with 255 lines; 1.50 ms per frame of core time without the overclock, 1.53
  with it and the yield.
* A patch may add PRG-ROM: data of the original size plus whole 16 KiB banks, the CHR-ROM after
  them (`apply_with` then raises the PRG bank count of the player's header, `resize_prg`). An
  NROM game maps its first 32 KiB as before; the added banks are seen through the window. The BPS
  patch copies the moved CHR-ROM from the player's ROM (SourceCopy) and the bank's fill with
  TargetCopy, so it still carries only the hack's bytes.
* `BuiltinPatch::options`: the options the player can turn on or off (`HackOption`: key, English
  and French labels, bit, default). The core starts with the defaults at each ROM load
  (`default_options()`); the application keeps the player's choices in `emu.hack_options`
  (`<patch>/<key>=0|1` entries separated by `;`), shows them in the game's hacks screen
  (`HacksModel`; the command `video.toggle_hack_option` stays for bindings) and sets them
  every frame (`NesCore::set_hack_options`); the ROM reads them at `$500A`.
* `NesCore::load_rom` applies the matching patch when `set_builtin_patches(true)`
  (`EmulationHost::builtin_patches_wanted`, at each ROM load: `emu.builtin_patches` and the
  game's `video.widescreen` on, owner request), or the forced one (section 1), and keeps the
  **original** image
  (`original_image()`); `builtin_patch()` names the patch.
* The battery file and the savestate fingerprint use the original image: a new version of a hack
  is the same game, and its battery RAM carries over.
* The patch only holds the bytes the hack writes (TargetRead); everything else is copied from the
  player's ROM (SourceRead): no game data ships with the emulator.

### 7.1 Hack packs

`zax-nes/src/hack_pack.rs`, packs in `zax-nes/patches/*.zxpk`; the proposal and its decisions
are in `docs/HACK-PACKS.md`. A `BuiltinPatch` whose `bps` starts with `ZXPK` is a pack
(`hack_pack::is_pack`); every built-in patch is one now, and every one has its code in the
extension's memory: the pack holds the code, the boot window is `$80`/0 (the extension unlocked,
the window on the pack's code from power-on), and the ROM keeps its size: no bank added, no fixed
bank copied to the end, no CHR-ROM moved, nothing written in the game's free space.

* **The hacks that added a PRG bank for the window** (Mega Man, 3 to 6, Super Mario Bros.,
  Adventure Island, Excitebike, Blaster Master, Metroid): their bank is the pack's code, up to
  10 KiB for a game with no PRG-RAM (`$5800`-`$7FFF`).
* **The hacks that had their code in the game's free space** (Mega Man 2, Super Mario Bros. 3,
  Castlevania, Final Fantasy, Ninja Gaiden III, Metal Force): their source assembles for `$5800`
  when the pack is built (`tools/build.py --pack`; ca65 `-D PACK` for the hacks whose code was in
  the fixed bank). Their reset hook and the NMI's unlock are gone (Super Mario Bros. 3: the
  fixed bank's NMI code that unlocked and opened the window, its NMI hook now jumping into the
  window), since the boot window unlocks the extension and the code only runs while it is
  unlocked. Their code is under 2 KiB: the boot window keeps `$6000`-`$7FFF` for the game
  (`boot_window_keeps_prg_ram`), as the hacks' own code did.
* **Zelda II**: its hack, written in a fork of a disassembly, was moved out of the game
  (`z2pack`: the code for `$5800`, its 36 hooks); its boot window keeps the game's PRG-RAM, the
  window showing 2 KiB. `tools/pack_from_roms.py` makes a pack of the bytes a same-size build
  changes, for a hack not yet written as hooks (it made Zelda II's first pack).

The testing aids are components of their own where their hook can do the hack's work in place
of the bytes it replaces ("Invincible" of Mega Man, 2 to 6, Super Mario Bros., Castlevania and
Metal Force); the others (the level selectors, the invincibility of Super Mario Bros. 3,
Adventure Island, Blaster Master and Metroid) need the hack's code and stay in the widescreen
component, with the advanced mode only.

Format 1, little-endian: `ZXPK`, `u8` format, name (`u8` length + UTF-8), `u32` code length and
the code (assembled for the window at `$5800`), `u8` component count, then each component: its
name, `u16` hook count and each hook (`u32` offset in the ROM data without iNES header, `u8`
length, the original bytes, the new ones).

* **Code.** It never goes in the ROM: the extension's PRG window shows it (section 2), the boot
  window (`boot_window`, `$80`/0 for Mega Man 6) opening it at power-on. The ROM keeps its size.
* **Hooks.** The pack fits a ROM when every hook of every component finds its original bytes
  (`HackPack::matches`); a pack is applied whole or not at all. It is found by the ROM's CRC-32,
  and failing that by its hooks (`builtin_patches::pack_for`, `patch_for`): another build of the
  game whose hooked code is the same takes it too.
* **Components.** `"widescreen"` follows the advanced mode: `apply_with` writes its hooks into
  the patched image, like a BPS patch, and `PatchedRom::code` carries the code. Every other
  component is a **testing aid** named after its option's key (Mega Man 6: `"invincible"`). The
  core applies it to the cartridge's PRG-ROM, or puts the game's bytes back, whenever its option
  changes (`NesCore::set_hack_options` -> `sync_aids`), **with or without** the advanced mode
  (owner request: the aids are independent of the widescreen). `NesCore::options_patch()` names
  the patch whose options apply then, and `hack_options()` lists only the aids' options while the
  patch does not run; the hacks screen shows those rows without the advanced mode
  (`BuiltinPatch::is_aid`).
* **Another ROM of the game** (a revision, a ROM hack). The hacks screen opens for a game whose
  name or profile says a hack is made for it, as before. Inside, `HacksModel::for_rom` looks for
  the patch by the ROM's CRC-32 first, then by the signatures of the hack packs
  (`builtin_patches::pack_for` on the file): a ROM that has them is treated as one the hack is
  made for (its rows, its options, no forcing), and the list's widescreen command turns it on
  directly. A pack is never forced: a ROM without its signatures is told so
  (`hacks.pack_signatures`). The margin decoder of such a ROM is that of the pack's patch
  (`NesCore::margin_decoder`, after the CRC's).
* **Version.** `BuiltinPatch::version()` of a pack is the CRC-32 of the pack.
* **Install.** `NesCore::install_pack` runs after the cartridge is attached, at each load and at
  each restore of a state that carries its ROM: the pack made for the host's image, the code in
  the window while its widescreen runs, the window opened on it again (a state may hold another
  window), the aids as their options say.
* `RomImage::prg_rom_mut` and `Cartridge::rom_mut` give the core the PRG-ROM to write the aids'
  hooks to (crate-private).

---

## 8. Savestates

### 8.1 Core block

`NesCore::save_state` writes format version **4** for a machine whose ROM never unlocked the
extension (byte for byte the standard block) and **6** once it did: the extension's registers,
sprite high bytes, RAM, overclock, layout, PRG window and stall counter follow the cartridge
block. Version 5 (before the PRG window) is still read. The block carries the **original** ROM
image; `restore_state` applies the built-in patch of the running build to it (when the patches
are on), so a state survives a new version of the hack's code.

MMC3 (`Mmc3::load_registers`) computes its bank windows again from its registers after a
restore (`update_banks`): the block stores them as offsets in the PRG-ROM it was saved with,
which are wrong for a PRG-ROM of another size (Mega Man 6 saved with its 68-bank BPS patch and
restored with its 64-bank pack: the fixed last bank pointed into bank 66).

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
  5/6) while the patches are not wanted for the game, or the reverse. Shown in **orange** in the
  load menus. Choosing it asks (`TURN_HACK_ON_QUESTION` / `TURN_HACK_OFF_QUESTION`, translated);
  agreeing turns the game's widescreen on (Auto, and `emu.builtin_patches` on) or off, then
  loads the slot, whose restore reloads the ROM from the state with the patch as now set.
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
* a state without the hack restored with it: the extension starts locked; the core unlocks it
  again with the window on the pack's code (`install_pack`, the patch's `boot_window`), since no
  hack unlocks the extension itself any more.

---

## 9. Tools

`zax-harness render` options for widescreen work:

| Option | Effect |
|---|---|
| `--widescreen <px>` | host margin on each side |
| `--widescreen-y <px>` | host margin above and below |
| `--no-builtin-patches` | restore `--load-state` without the built-in patches |
| `--load-state <file>` / `--save-state <file>` | core state block (saved with its ROM image) |
| `--png-nes <file>` | the frame's records composed flat, as wide and as tall as the margins |
| `--dump-mem <file>` | `$0000-$07FF`, `$6000-$7FFF`, `$5000-$5FFF`; prints PC, registers, JAMMED |
| `--press <frames>:<buttons>` | input script |

Tests: `zax-nes` (`wide.rs`, `wide_decode`, `builtin_patches.rs`; one ignored test needs
`ZAXNES_ROMS` with the Zelda II ROM), `zax-app/tests/save_state_tests.rs` (header, thumbnail,
old format), `zax-settings/tests/catalogue.rs` (the Rust-only keys).
