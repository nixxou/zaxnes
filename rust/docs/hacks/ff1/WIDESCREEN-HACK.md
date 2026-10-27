# Final Fantasy (USA): the widescreen hack

This document describes completely the modification of *Final Fantasy (USA)* that Zaxnes ships
as a built-in patch (advanced mode). The appendices hold the new code and the build script, which
writes every hook after checking the bytes it replaces: with them the ROM can be rebuilt
identically.

The background of the margins is not the hack's work: the emulator's Final Fantasy margin decoder
unpacks the world map from PRG-ROM and reads the standard maps (towns, castles, dungeons) in
PRG-RAM (see [`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4), and
widens the unpatched game on its own. The hack is the game's **advanced mode**: the people of the
standard maps and the vehicles, the bridge and the canal of the world map, which the game leaves
out past the edges of the screen, show in the margins.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Final Fantasy (USA)*, MMC1 (mapper 1), 256 KiB PRG (16 banks of 16 KiB, bank 15 fixed at `$C000`-`$FFFF`, the others switched at `$8000`), 8 KiB CHR-RAM, 8 KiB battery PRG-RAM |
| Source data (iNES header excluded) | CRC-32 `CEBD2A31` |
| Target data | CRC-32 `86DBFACC` (the hack's version in savestate headers; `tools/build.py` prints it) |
| Changes | 2 KiB of code at PRG offset `$1B000`-`$1B7FF` (bank 6 from its page `$30`, zeros in the game), four hooks in the fixed bank; size and layout unchanged |
| Patch | `rust/crates/zax-nes/patches/final-fantasy-usa-widescreen.bps`, made against the data without header |

The ROM keeps its size and layout: a state made without the hack restores with it as it is, and
the battery file is the same.

---

## 2. The extension and the code's place

The fixed bank has no room for code that would unlock the extension. The hack's code is the free
end of PRG bank 6 instead (the bank's last 4,966 bytes are zeros), which the extension's PRG window
shows at `$5800`-`$5FFF`, and the built-in patch opens that window from power-on
(`BuiltinPatch::boot_window`, `$5008 = $86`, `$5009 = $30`): the hack needs Zaxnes, and its code
runs whatever bank the game maps at `$8000`. The window is not laid over `$6000-$7FFF` (`$5007`
bit 4 stays clear): the game keeps its save, its maps and its people there.

The game waits for each frame at `$FEA8`: it turns the NMI on and loops on `JMP $FEBB`; the NMI
(`$FE9C`) drops the return address and returns to the wait's caller. A frame whose logic misses the
VBlank is a lag frame. `idle_loop` is `$FEBB`-`$FEBD`.

`wide_frame`, at that wait (once a frame, whatever the game shows): `$5003 = $5002` (the margin
the host offers: the sprites' high bytes are kept that far), `$5004 = $5005 = 0` (no nametable
columns in the margins: the decoder draws the background, and a battle, a menu or the title show
nothing beside them), `$5007 = $02` (sprites free of those extents), `$5006 = 255` with the option
"No slowdown" (`$500A` bit 0), else 0. Then it does the `LDA $2002` it replaces, whose flags the
`BPL` after it reads.

Variables (extension RAM): `WV_XHI` `$5200`, the high byte of the x of the next 16 x 16 sprite the
game draws, 0 after it; `WV_T` `$5201`, scratch.

---

## 3. What the game does, and what changes

The game draws every 16 x 16 sprite of a map with `$E301`: four sprites from OAM offset `$26`
(which it then moves on by 16), at x `$40` and y `$41`, their tiles and attributes from the
pointer `$10`, plus `$12` to the tiles.

* **The people of a standard map**: 15 slots of 16 bytes at `$6F00` (`$6F00` the kind, 0 for an
  empty slot; `$6F01` the flags, bit 7 inside a room, bit 6 still; `$6F04`/`$6F05` the tile they
  are drawn at, `$6F06`/`$6F07` the pixels past it). The game moves one person a frame wherever
  they are (`$E50A`), and draws them all (`$E4C7`, `$E688`, `$E6D8`): their screen y and x come
  from the map's distance to the camera (`$29`/`$2A` the camera's tile, `$35`/`$36` its pixels),
  the map 64 tiles around, and a person is left out unless that distance is 0-15 tiles and x below
  248 (`$E70D`-`$E72B`). The hack keeps the game's y and its bounds; for x, the distance in tiles
  is signed (-32 to 31) and the x 16-bit, and a person shows when any of their 16 pixels falls
  within the margin `M` the host offers: `-16 - M < x < 256 + M`.
* **The world map's vehicles, bridge and canal** (`$6000`-`$600E`, a visibility byte, then the
  tile x and y, for each; `$E373`, `$E38C`, `$E3A6`): `$E3DF` puts one on the screen from its tile
  and the camera (`$27`/`$28`, `$35`/`$36`), the map 256 tiles around, and leaves it out past
  15 tiles or x 248 (`$E3F4`-`$E408`). The hack gives it the same 16-bit x.
* **The sprites' high bytes**: a successful placement always ends in one call of `$E301` (both of
  `$E730`'s ways, and `$E3CD`). The placement leaves the high byte in `WV_XHI`; `$E301`'s hook
  writes it for the left column of the four sprites (`$5103 + X`, `$5107 + X`, X their OAM offset)
  and, with the carry of `x + 8`, for the right one (`$510B + X`, `$510F + X`), then sets `WV_XHI`
  back to 0: every other sprite of the game (the party, the battle, the menus) gets 0, as the
  extension starts each OAM DMA's page.

The extension leaves a sprite with an x outside 0-255 out of the PPU's own picture, and it takes
no place among the eight of its lines: a person in a margin neither flickers nor hides one on the
screen. The game's OAM has room for them all: the party's 4 sprites and 15 people of 4.

A sprite's y stays 8-bit: the people above or below the screen stay hidden, while the decoder
draws the map in the vertical margin.

---

## 4. Hooks

All in the fixed bank (bank 15, PRG offset `$3C000 + (address - $C000)`), unused bytes filled
with `NOP`:

| Address | Replaced bytes | Becomes |
|---|---|---|
| `$FEA8` | `AD 02 20` (`LDA $2002`) | `JSR wide_frame` |
| `$E316` | `A5 40 9D 03 02` (`LDA $40` / `STA $0203,X`) | `JSR wide_sprite_x`, 2 `NOP` |
| `$E70D` | `BD 06 6F 38 E5 35 29 0F 85 10 BD 04 6F E5 29 29 3F C9 10 B0 0A 0A 0A 0A 0A 05 10 C9 F8 90 02` (the person's x and its bounds) | `JSR wide_person_x`, `BCC $E72E` (`90 1C`, to the game's `STA $40`), 26 `NOP` down to the game's `SEC` / `RTS` at `$E72C` |
| `$E3F4` | `8A 38 E5 27 C9 10 B0 11 0A 0A 0A 0A 38 E5 35 90 08 C9 F8 B0 04` (the vehicle's x and its bounds) | `JSR wide_vehicle_x`, `BCS $E40D` (`B0 14`, to the game's `SEC` / `RTS`), 16 `NOP` down to the game's `STA $40` at `$E409` |

The new code, at `$5800` through the window (its labels as `ld65` places them): `wide_frame`
`$5800`, `wide_sprite_x` `$5823`, `wide_person_x` `$5847`, `wide_vehicle_x` `$5861`, `place`
`$586D`. `wide_person_x` and `wide_vehicle_x` work out the distance in tiles (signed) and the
pixels past it as the game does, `place` the 16-bit x and the bounds; carry clear and A the low
byte when shown.

---

## 5. Options

| Bit (`$500A`) | Key | Default | Effect |
|---|---|---|---|
| 0 | `no_slowdown` | on | 255 lines of overclock (`$5006`); the yield ends it at the game's wait (`$FEBB`) |

Measured with a temporary trace of the waits at `$FEA8` (not part of the hack): 600 frames of
walking in Coneria with its people, 600 on the world map and 600 of a battle, at a margin of 0 or
128 pixels: no lag frame, with or without the hack. The option costs nothing then (the yield), and
keeps a heavier scene smooth. Core time, power-on to the world map then 600 frames of walking:
1.03 ms a frame without margins, 1.36 ms at 128 pixels (the decoder), 1.38 ms with the hack.

---

## 6. Building

```
python -I tools/build.py "Final Fantasy (USA).nes" final-fantasy-usa-widescreen.bps
python -I tools/make_doc.py <zaxnes>/rust/docs/hacks/ff1/WIDESCREEN-HACK.md
```

`build.py` assembles `src/wide.s` with ca65/ld65 (through WSL when they are not on the PATH) into
the 2 KiB `WINDOW` segment at `$5800` (fill `$00`, so the bank's unused zeros stay), checks the
ROM's data CRC-32, checks that the space is all zeros and each hook's original bytes, and writes
`build/final-fantasy-usa.nes` and the BPS patch. `make_bps.py` writes the patch with SourceRead
wherever the target keeps the source's byte and TargetRead for the hack's bytes only: no data of
the game travels in it. `make_doc.py` writes this document's appendices from the sources.

---

## 7. Limits

* The people above and below the screen stay hidden (8-bit sprite y).
* The game decides who is drawn as it does on the screen: a person in a room (`$6F01` bit 7)
  shows only from inside it, the others only from outside.
* The black sweep of a change of screen (the game blanks one line after the other, moving down
  2 lines a frame) covers the original screen only: the margins keep the map until the new
  screen is drawn.

---

## Appendices

### A. `src/wide.s`

```asm
; Final Fantasy (USA): the widescreen hack (Zaxnes, advanced mode).
;
; The background of the margins is not the hack's work: Zaxnes' margin decoder draws the world
; map and the standard maps there. The hack draws the people of a standard map (towns, castles,
; dungeons) and the vehicles, the bridge and the canal of the world map past the edges of the
; screen, where the game leaves them out: their sprites get a 16-bit x (the extension's high
; bytes at $5100).
;
; The code is WINDOW, 2 KiB of the free end of PRG bank 6 (from its page $30, zeros in the game),
; which the extension's PRG window shows at $5800-$5FFF. The game's fixed bank has no room for
; code that would unlock the extension, so the built-in patch opens the window from power-on
; (BuiltinPatch::boot_window): the hack needs Zaxnes.
;
; Once a frame, as the game waits for the NMI, the hack writes the extension's settings and the
; overclock of the option "No slowdown".
;
; The ROM keeps its size and layout: a state of the unpatched game restores into the patched one.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1: sprites free of the extents
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA

; Extension RAM ($5200-$57FF).
WV_XHI          = $5200         ; the high byte of the x of the next sprite drawn ($E301), 0 after
WV_T            = $5201         ; scratch

; The options ($500A).
OPT_NO_SLOWDOWN = $01           ; 255 lines of overclock: no lag frame

; ---------------------------------------------------------------------------- ;
; The game.

SPR_X           = $40           ; the x and y of the 16 x 16 sprite $E301 draws
SCRATCH         = $10
OAM             = $0200

WORLD_SCROLL_X  = $27           ; the world map's camera, in tiles of 16 pixels
STANDARD_SCROLL_X = $29         ; a standard map's
FINE_X          = $35           ; the camera's x within its tile

; A standard map's people: 15 slots of 16 bytes.
OBJ_TILE_X      = $6F04         ; the tile they are drawn at
OBJ_FINE_X      = $6F06         ; and the pixels past it

; ---------------------------------------------------------------------------- ;

.segment "WINDOW"

; Each frame, as the game waits for the NMI ($FEA8: replaces its LDA $2002, whose flags the BPL
; after it reads).
.proc wide_frame
        lda WIDE_HOST           ; the margins the decoder draws: the sprites go as far, past
        sta WIDE_REQ            ; extents that keep the nametables out of them (a battle shows
        lda #0                  ; none of its other nametable there)
        sta WIDE_LEFT
        sta WIDE_RIGHT
        lda #$02
        sta WIDE_LAYOUT
        lda WIDE_OPTIONS        ; the overclock: the yield ends it as the game waits ($FEBB)
        and #OPT_NO_SLOWDOWN
        beq :+
        lda #255
:       sta WIDE_OVERCLOCK
        lda $2002
        rts
.endproc

; The game's sprite builder ($E301: four sprites, 16 x 16, at SPR_X; X the OAM offset): the
; high bytes of their x, from WV_XHI, which the hooks below set for a person or a vehicle drawn
; in a margin and every other sprite of the game finds 0. Replaces LDA SPR_X / STA OAM+3,X.
.proc wide_sprite_x
        lda WV_XHI
        sta WIDE_SPRHI + 3,x    ; the left column
        sta WIDE_SPRHI + 7,x
        lda SPR_X               ; the right one, 8 pixels on
        clc
        adc #8
        lda WV_XHI
        adc #0
        sta WIDE_SPRHI + 11,x
        sta WIDE_SPRHI + 15,x
        lda #0
        sta WV_XHI
        lda SPR_X
        sta OAM + 3,x
        rts
.endproc

; A person of a standard map ($E70D; X their slot): their screen x, 16-bit, from their tile and
; fine x and the camera's, the map 64 tiles around (the game's own reading, but with the tile
; distance signed rather than on the screen only). Carry clear and A the low byte when they show
; within the margins (WV_XHI the high byte), carry set otherwise. Replaces the game's x and its
; bounds (LDA OBJ_FINE_X,X / ... / CMP #$F8 / BCC).
.proc wide_person_x
        lda OBJ_FINE_X,x
        sec
        sbc FINE_X
        and #$0F
        sta SCRATCH
        lda OBJ_TILE_X,x
        sbc STANDARD_SCROLL_X
        and #$3F
        cmp #$20                ; -32..31 tiles
        bcc :+
        ora #$C0
:       jmp place
.endproc

; A vehicle, the bridge or the canal of the world map ($E3F4; X their tile x): the same, the map
; 256 tiles around. Replaces TXA / SEC / SBC $27 / ... / CMP #$F8 / BCS.
.proc wide_vehicle_x
        lda #0
        sec
        sbc FINE_X
        and #$0F
        sta SCRATCH
        txa
        sbc WORLD_SCROLL_X      ; the borrow of the fine x
        ; fall through
.endproc

; A = a signed distance in tiles of 16 pixels, SCRATCH the pixels past it: the 16-bit x
; 16 A + SCRATCH, kept when it is within the margin the host offers (the sprite's 16 pixels
; partly in it).
.proc place
        sta WV_T
        asl a
        asl a
        asl a
        asl a
        ora SCRATCH
        pha
        lda WV_T                ; the high byte: A >> 4, signed
        cmp #$80
        ror a
        cmp #$80
        ror a
        cmp #$80
        ror a
        cmp #$80
        ror a
        sta WV_XHI
        beq show                ; on the screen
        cmp #$01
        beq right
        cmp #$FF
        bne hide
        ; Left of the screen: x > -16 - M, that is low > 240 - M.
        lda #240
        sec
        sbc WIDE_HOST
        bcc show                ; M past 240: as far as x = -256
        sta WV_T
        pla
        pha
        cmp WV_T
        beq hide
        bcs show
        bcc hide
right:  ; Right of it: x < 256 + M, that is low < M.
        pla
        pha
        cmp WIDE_HOST
        bcc show
hide:   pla
        lda #0
        sta WV_XHI
        sec
        rts
show:   pla
        clc
        rts
.endproc
```

### B. `tools/build.py`

```python
"""Builds the Final Fantasy (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Final Fantasy (USA).nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH): the WINDOW segment
   for $5800, 2 KiB the widescreen extension's PRG window shows there, taken from the free end of
   PRG bank 6 (from its page $30, zeros in the game). WIDE_DEFINES (comma-separated) is passed to
   ca65 as -D symbols.
2. Writes it into the game's free space (checked to be all zeros) and the hooks into its code
   (each checked against the bytes it replaces). The ROM keeps its size and layout.
3. Writes build/final-fantasy-usa.nes and, if asked, the BPS patch (data without iNES header).
   WIDE_SKIP (comma-separated hex addresses) leaves hooks out, for testing.
"""

import os
import shutil
import subprocess
import sys
import zlib

PRG_SIZE = 0x40000
BANK_16K = 0x4000
FIXED_BANK = 15
WINDOW_START = 0x5800
WINDOW_SIZE = 0x800
WINDOW_BANK = 6
WINDOW_PAGE = 0x30
WINDOW_AT = WINDOW_BANK * BANK_16K + WINDOW_PAGE * 0x100

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NAME = 'final-fantasy-usa'

NOP = 0xEA

# The data CRC-32 of Final Fantasy (USA).
SOURCE_CRC = 0xCEBD2A31


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    """A relative branch at `at` to `target`."""
    distance = target - (at + 2)
    assert -128 <= distance <= 127, hex(at)
    return bytes([opcode, distance & 0xFF])


def offset(bank, address):
    """Where `address` of the game's 16 KiB PRG bank `bank` lies in the PRG-ROM (bank 15 is fixed
    at $C000, the others are seen at $8000)."""
    return bank * BANK_16K + (address & (BANK_16K - 1))


BCC = 0x90
BCS = 0xB0


def hooks(sym):
    """The patches of the game's PRG-ROM: (bank, address, original bytes, new bytes)."""
    out = []

    def put(bank, address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((bank, address, original, new))

    # The wait for the NMI ($FEA8): the extension's settings and the overclock, each frame
    # (LDA $2002).
    put(FIXED_BANK, 0xFEA8, 'ad 02 20', jsr(sym['wide_frame']))
    # The sprite builder ($E301, four sprites at $40/$41): the high bytes of their x
    # (LDA $40 / STA $0203,X).
    put(FIXED_BANK, 0xE316, 'a5 40 9d 03 02', jsr(sym['wide_sprite_x']))
    # A standard map's people: their x, 16-bit, kept within the margins. From LDA $6F06,X to the
    # BCC to their drawing ($E72E, STA $40); not shown, on to the game's SEC / RTS ($E72C).
    put(FIXED_BANK, 0xE70D,
        'bd 06 6f 38 e5 35 29 0f 85 10 bd 04 6f e5 29 29 3f c9 10 b0 0a 0a 0a 0a 0a 05 10 c9 f8 90 02',
        jsr(sym['wide_person_x']) + branch(BCC, 0xE710, 0xE72E))
    # The world map's vehicles, bridge and canal: the same. From TXA to the last BCS; shown, on to
    # STA $40 ($E409), not shown to SEC / RTS ($E40D).
    put(FIXED_BANK, 0xE3F4, '8a 38 e5 27 c9 10 b0 11 0a 0a 0a 0a 38 e5 35 90 08 c9 f8 b0 04',
        jsr(sym['wide_vehicle_x']) + branch(BCS, 0xE3F7, 0xE40D))
    return out


def tool(name):
    return [name] if shutil.which(name) else ['wsl', '-e', name]


def wsl_path(path):
    if shutil.which('ca65'):
        return path
    drive, rest = os.path.splitdrive(os.path.abspath(path))
    return '/mnt/' + drive[0].lower() + rest.replace('\\', '/')


def assemble():
    build = os.path.join(ROOT, 'build', NAME)
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    window_bin = os.path.join(build, 'window.bin')
    open(cfg, 'w', newline='\n').write(
        'MEMORY {\n'
        f'  WINDOW: start = ${WINDOW_START:04X}, size = ${WINDOW_SIZE:04X}, fill = yes, fillval = $00, '
        f'file = "{wsl_path(window_bin)}";\n'
        '}\n'
        'SEGMENTS {\n'
        '  WINDOW: load = WINDOW, type = ro;\n'
        '}\n')
    obj = os.path.join(build, 'wide.o')
    labels = os.path.join(build, 'wide.labels')
    flags = []
    for symbol in [d for d in os.environ.get('WIDE_DEFINES', '').split(',') if d]:
        flags += ['-D', symbol]
    subprocess.run(tool('ca65') + ['-g'] + flags +
                   [wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)], check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-Ln', wsl_path(labels), wsl_path(obj)],
                   check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return open(window_bin, 'rb').read(), sym


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE]
    crc = zlib.crc32(data)
    if crc != SOURCE_CRC:
        sys.exit(f'{rom_path}: not Final Fantasy (USA) (data CRC {crc:08X})')

    window, sym = assemble()
    assert len(window) == WINDOW_SIZE
    prg = bytearray(data)
    assert prg[WINDOW_AT:WINDOW_AT + WINDOW_SIZE] == bytes(WINDOW_SIZE), \
        'bank 6: the space is not free'
    prg[WINDOW_AT:WINDOW_AT + WINDOW_SIZE] = window

    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for bank, address, original, new in hooks(sym):
        if address in skip:
            continue
        at = offset(bank, address)
        assert prg[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        prg[at:at + len(new)] = new
    target = bytes(prg)

    out = os.path.join(ROOT, 'build', NAME + '.nes')
    open(out, 'wb').write(bytes(image[:header]) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        patch = make_bps.make(bytes(data), target)
        open(sys.argv[2], 'wb').write(patch)
        print(f'{sys.argv[2]}: {len(patch)} bytes')


if __name__ == '__main__':
    main()
```

### C. `tools/make_bps.py`

```python
"""Writes a BPS patch from an original ROM's data to the widescreen build's, both without their
iNES header.

Only the bytes the hack writes travel in the patch (TargetRead); everything else is copied from the
player's own ROM: SourceRead where a byte sits where it was, SourceCopy for data the build moved
(the CHR-ROM, after the PRG bank the hack adds). The patch carries no data of the game.
"""

import zlib


def number(value):
    out = bytearray()
    while True:
        low = value & 0x7F
        value >>= 7
        if value == 0:
            out.append(0x80 | low)
            return bytes(out)
        out.append(low)
        value -= 1


def make(source, target, moved=()):
    """`moved`: (target start, target end, source start) of data the build moved."""
    patch = bytearray(b'BPS1')
    patch += number(len(source)) + number(len(target)) + number(0)
    source_relative = 0
    target_relative = 0

    def origin(position):
        """Where target byte `position` comes from in the source, when it may come from there."""
        for start, end, from_ in moved:
            if start <= position < end:
                return from_ + position - start
        return position

    position = 0
    while position < len(target):
        def same(at):
            at_source = origin(at)
            return at_source < len(source) and source[at_source] == target[at]

        copied = same(position)
        shifted = origin(position) != position
        end = position
        while end < len(target) and same(end) == copied and (origin(end) != end) == shifted:
            end += 1
        length = end - position
        pending_start = position
        if not copied:
            # Bytes of the hack; a run of one value (the bank's $FF fill) as its first byte, then
            # a TargetCopy of it.
            run = position
            while position < end:
                run = position + 1
                while run < end and target[run] == target[position]:
                    run += 1
                if run - position >= 8:
                    if pending_start < position:
                        patch += number(((position - pending_start - 1) << 2) | 1) + target[pending_start:position]
                    patch += number(1) + target[position:position + 1]
                    delta = position - target_relative
                    patch += number(((run - position - 2) << 2) | 3)
                    patch += number((abs(delta) << 1) | (1 if delta < 0 else 0))
                    target_relative = position + (run - position - 1)
                    pending_start = run
                position = run
            if pending_start < end:
                patch += number(((end - pending_start - 1) << 2) | 1) + target[pending_start:end]
        elif not shifted:
            patch += number(((length - 1) << 2) | 0)
        else:
            start = origin(position)
            delta = start - source_relative
            patch += number(((length - 1) << 2) | 2)
            patch += number((abs(delta) << 1) | (1 if delta < 0 else 0))
            source_relative = start + length
        position = end
    patch += zlib.crc32(source).to_bytes(4, 'little')
    patch += zlib.crc32(target).to_bytes(4, 'little')
    patch += zlib.crc32(bytes(patch)).to_bytes(4, 'little')
    return bytes(patch)
```
