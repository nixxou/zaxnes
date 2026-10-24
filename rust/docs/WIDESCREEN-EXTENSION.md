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
| Window | enough to fill the window at 240 lines, up to 96 | up to 448 x 240 |
| 3:2 | 56 | 368 x 240 |
| 16:10 | 64 | 384 x 240 |
| 16:9 | 88 | 432 x 240 |

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
