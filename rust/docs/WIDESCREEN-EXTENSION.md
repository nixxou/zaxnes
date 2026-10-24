# Widescreen extension

A ROM made for Zaxnes can draw past the 256 pixels of the NES picture. The core then captures
extra tile columns on each side, and the 3D scene shows them in a wide window. Real hardware has
none of this: such a ROM is made for this emulator (it may still boot elsewhere, without the
extra columns).

Nothing changes for the other ROMs: the register window stays transparent until a ROM writes the
unlock sequence, and the captures, savestates and goldens of every other game are the standard
ones.

## Host side

`video.widescreen` (Video options, Rust only) says how wide the picture may get:

| Choice | Margin offered on each side | Picture |
|---|---|---|
| Off | 0 | 256 x 240 |
| Window | enough to fill the window at 240 lines | |
| 3:2 | 56 | 368 x 240 |
| 16:10 | 64 | 384 x 240 |
| 16:9 | 88 | 432 x 240 |

`video.widescreen_extra` (+0 % to +300 %, +10 % by default) widens the picture beyond the ratio,
so that the 3D objects at the edges of the view are already whole when they come into sight. The
host margin is at most 512 pixels on each side (a 1280 x 240 picture). Through the nametables a
ROM can show 128 at most (they wrap past that), and it can ask for 248 at most (8-bit register).

`video.widescreen_strict_edges` (off by default) tells the 3D engine where each band of 8 lines
stops being drawn (`FrameBatch::visible_rows`): a shape cut there stays unfinished until it is
whole. Off, every band counts as drawn across the whole margin.

## Margin decoders

The nametables only hold what a game keeps drawn, two screens at most. A game that keeps its
whole area in RAM can do better: a margin decoder (`zax_nes::wide_decode::MarginDecoder`) turns
that RAM into the tiles of the margins, as wide as the host asks. After each frame the core hands
the decoder every captured background row (the PPU address and fine X it was drawn with) and the
memory (CPU RAM, PRG-RAM, PRG-ROM); the decoder returns, per screen column of the margins, a CHR
tile of the row's pattern table and a sub-palette. A row it does not handle (status bar, a mode
it does not know) keeps the columns captured from the nametables. The centre 256 pixels always
come from the PPU.

A decoder belongs to a built-in patch (`wide_decode::for_patch`) and runs while the built-in
patches are on. With a decoder the host margin applies whatever the ROM asked for. Decoders:

| Game | Decodes |
|---|---|
| Zelda II (USA) | side view (level RAM at `$6000`), world map (run-length map in PRG-RAM); adapted from z2rs (MIT) |

`NesCore::set_widescreen_margin(pixels)` is the same thing for other hosts.

## ROM side: registers

The window is `$5000`-`$5FFF`. Writes still reach the cartridge (the window only listens); reads
are answered by the window only once it is unlocked.

| Address | Access | Meaning |
|---|---|---|
| `$5000` | W | unlock: write `$57 $49 $44 $45` ("WIDE") in a row. Reads `$57` once unlocked. |
| `$5001` | R | version, 1 |
| `$5002` | R | margin the host offers on each side, in pixels, a multiple of 8 (0 = off) |
| `$5003` | R/W | margin the game asks for on each side, in pixels |
| `$5004` | R/W | how far left of x = 0 the picture may show, in pixels (power-on `$FF`) |
| `$5005` | R/W | how far right of x = 255 the picture may show, in pixels (power-on `$FF`) |
| `$5006` | R/W | overclock: extra CPU time each frame, in scanlines (power-on 0) |
| `$5007` | R/W | bit 0: four separate nametables (2 x 2, the console's 4 KiB of VRAM), whatever mirroring the cartridge selects (power-on 0) |
| `$5100`-`$51FF` | W | high byte of the x of sprite *n* = (address & `$FF`) / 4 |
| `$5008` | R/W | PRG window: bit 7 on, bits 0-6 the 16 KiB PRG-ROM bank shown at `$5800`-`$5FFF` (power-on 0) |
| `$5009` | R/W | PRG window: first 256-byte page of that bank it shows (power-on 0) |
| `$5200`-`$5FFF` | R/W | 3.5 KiB of RAM for the ROM's own variables and code: the CPU runs code from it, whatever bank the mapper shows (power-on 0) |

* The margin in use is `min($5003, $5002)`, rounded down to a multiple of 8. It is latched at the
  start of each frame, so every band of a frame has the same columns.
* Background: each 8-line band captures `margin / 8` more columns on each side, read on through
  the horizontally neighbouring nametable (the 512-pixel plane wraps), from the same scroll as
  the standard columns.
* `$5004` and `$5005` are read each time a band is captured: a game writes them during the frame
  to hide the sides behind a status bar, or past the end of a level. A tile is kept when any of
  its pixels falls in `[-min(margin, $5004), 256 + min(margin, $5005))`.
* Sprites: the x of sprite *n* is the signed 16-bit value `high:OAM x`, `high` being the byte last
  written to `$5100 + 4n` (any of the four addresses of the sprite) **before the OAM DMA** that
  copied the page. Each `$4014` write takes the bytes written since the previous one and starts
  the next page from 0, so a game writes the high byte of every sprite it places off the standard
  picture, every frame, and the others stay at 0. A sprite at x = -16 is `$FF:$F0`; at x = 300,
  `$01:$2C`. Sprites past the visible range are dropped, like background tiles.
* Overclock: when the PPU finishes the picture (start of line 240) the PPU and the APU stay
  still for `$5006` x 114 CPU cycles while the CPU runs on (the cartridge still counts them, so
  that MMC1 keeps telling consecutive writes apart): the frame logic gets
  that much more time before the NMI, and the picture, the sound and their timing do not
  change. Showing and running enemies in the margins costs CPU time the console does not have;
  this is where it comes from.
* Four nametables (`$5007` bit 0): with a horizontally mirrored cartridge the two nametables are
  stacked and the picture can only wrap 256 pixels wide; with four the plane is 512 x 480, so a
  game that scrolls both ways (a world map) can keep columns drawn past both sides.
* PRG window (`$5008`/`$5009`): 2 KiB of PRG-ROM shown at `$5800`-`$5FFF` whatever bank the
  mapper has at `$8000`, so code the fixed bank calls can live in any bank; being ROM, it is
  always the code of the ROM loaded (a copy in RAM would travel in savestates).
* The 64-sprite OAM and the eight-sprites-per-line limit are unchanged (the core already captures
  every sprite, eight per line or not).

## Typical use

```
        LDA #$57 : STA $5000     ; unlock
        LDA #$49 : STA $5000
        LDA #$44 : STA $5000
        LDA #$45 : STA $5000
        LDA $5000 : CMP #$57     ; not the extension: open bus
        BNE no_wide
        LDA $5002 : STA $5003    ; take what the host offers
```

## Savestates

A state saved after a ROM unlocked the extension has format version 5 and carries the registers
after the cartridge block; every other state keeps version 4, byte for byte.
