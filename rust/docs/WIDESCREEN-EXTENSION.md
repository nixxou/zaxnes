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
one ([`hacks/megaman2/WIDESCREEN-HACK.md`](hacks/megaman2/WIDESCREEN-HACK.md)), and the Super Mario Bros. (World) one, its advanced mode ([`hacks/smb1/WIDESCREEN-HACK.md`](hacks/smb1/WIDESCREEN-HACK.md)).

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
No-Intro name of its ROM), shows a red [W]. When its data has the size the patch was made for
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
  of a hack.
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
  in the middle of the wide picture. The sprites the PPU did not draw are then drawn over the
  centre from their records: the second page, and those across the left edge (x < 0), whose part
  on the screen the PPU left out with the rest; one behind the background only over the backdrop
  colour. Without margins it is the plain 256 x 240 frame.

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
   captured row and as many below the last, whole width. Each virtual row carries the PPU address
   it would have on the plane of two stacked nametables (60 rows, the vertical nametable bit
   flipping every 30) and a line outside `0..240`. A decoder that cannot returns `false` (the
   default) and the vertical margin stays empty.

Decoders run only while the built-in patches are on for the game in play (see 7). The decoders:

| Game | Decodes | Source |
|---|---|---|
| Zelda II (USA) | side view (level RAM), world map (run-length map), vertical margin on the world map | `zelda2.rs`, adapted from z2rs (MIT), see the Zelda II document |
| Mega Man 2 (USA) (data CRC-32 `0FCFC04D`), with its built-in patch (also listed for the unpatched ROM) | the stage layout in PRG-ROM, within the section being played | `megaman2.rs`, see below |
| Super Mario Bros. (World) (data CRC-32 `D445F698`), unpatched | the area, laid out by the game's own area parser run in a sandbox | `smb1.rs`, see below |

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
* `BuiltinPatch::advanced`: an **advanced mode** of a game's widescreen (Super Mario Bros.: the
  enemies in the margins). Such a patch applies only when the game's `video.widescreen_advanced`
  is on too (`NesCore::set_advanced_patches`, set with `set_builtin_patches` at each load and
  restore); off, the ROM loads as it is and its margin decoder alone widens it. The hacks screen
  shows the row "Advanced mode" for such a game (`HackRow::Advanced`; on, it turns the widescreen
  on too), and the options of an advanced patch only while it is on. A state is "the hack" or not
  as for any patch; loading one made in the other mode switches the advanced mode (the widescreen
  stays on).
* `$5007` bit 4: the PRG window goes on over `$6000-$7FFF`, 10 KiB from `$5800`, for a game
  with nothing there (no PRG-RAM; the cartridge never sees those reads). A boot window sets it.
* `BuiltinPatch::boot_window`: `(bank, page)` of the extension's PRG window, opened at power-on
  and reset with the extension unlocked (`WideExtension::boot`), and again after a restore that
  left it locked: for a ROM with no room for the code that would unlock it, whose reset or NMI then
  jumps straight into the window.
* `BuiltinPatch::idle_loop`: first and last address of the game's wait for the next frame (Mega
  Man 2: `LDA $1D / BEQ` at `$C08E`; Super Mario Bros.: its main loop `JMP $8057`; Zelda II: its
  main loop `$C010`-`$C01A`, everything else running in the NMI). `NesCore::run_frame` ends the
  overclock (`WideExtension::end_stall`) as soon as the CPU is there: the cycles left would only
  run the wait loop, so the game is the same, and a frame that does not need the overclock costs
  nothing more ("yield", owner request). Super Mario Bros. at 128 pixels with 255 lines: 1.38 ms
  per frame of core time with the fixed overclock, 1.11 ms with the yield, as without overclock;
  Mega Man 2's Air Man run keeps no lag frame (232 at 64 lines).
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
