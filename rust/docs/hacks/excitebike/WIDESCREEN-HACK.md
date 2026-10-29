# Excitebike (Japan, USA): the widescreen hack

This document describes completely the modification of *Excitebike (Japan, USA)* that Zaxnes
ships as a built-in patch for the game's **advanced** widescreen mode. The appendices hold the new
code and the build script, which writes every hook after checking the bytes it replaces: with them
the ROM can be rebuilt identically.

The background of the margins is not the hack's work: Zaxnes' Excitebike margin decoder lays the
track out by running the game's own column routine in a sandbox (see
[`../../WIDESCREEN-EXTENSION.md`](../../WIDESCREEN-EXTENSION.md), section 4). In the **simple**
mode the ROM is not patched at all and the decoder alone widens the game; the advanced mode adds
this patch: the riders in the margins, and the option.

---

## 1. Identity

| | |
|---|---|
| Source ROM | *Excitebike (Japan, USA) (En)*, NROM-128 (mapper 0), 16 KiB PRG mirrored at `$C000`, 8 KiB CHR |
| Source data (iNES header excluded) | CRC-32 `49AEB3A6` |
| Target data | CRC-32 `B67CBC02` (the hack's version in savestate headers; `tools/build.py` prints it), 56 KiB: the game's 16 KiB of PRG, a copy of them, a 16 KiB PRG bank added, the 8 KiB of CHR |
| Changes | hooks in both copies of the game's PRG (same size as what they replace); the new code in the added bank |
| Patch | `rust/crates/zax-nes/patches/excitebike-widescreen.bps`, made against the data without header; the copy of the PRG and the CHR-ROM are copied from the player's ROM (BPS SourceCopy), so the patch carries only the hack's bytes |
| Zaxnes declaration | `builtin_patches.rs`: `advanced: true` (applied only in the advanced mode), `boot_window: Some((0x82, 0x00))`, `idle_loop: Some((0xC1D8, 0xC1E0))` |

The game has no free byte in its 16 KiB. The PRG is therefore laid out anew: the game, a copy of
it, then the hack's bank. NROM maps the first 32 KiB at `$8000-$FFFF`, so the copy keeps the
game's mirror at `$C000` (where its vectors are, and most of its calls go), and the game runs as
it was. The hack's bank is only seen through the extension's PRG window: Zaxnes opens it at
power-on and reset for this patch (the boot window): unlocked, the window showing the added bank
(bank 2, page 0) at `$5800-$5FFF`, and on to `$7FFF` (`$5007` bit 4; the game has no PRG-RAM
there). The data grows by two PRG banks; Zaxnes raises the PRG bank count of the player's header
accordingly when it applies the patch.

---

**Hack pack.** Zaxnes ships this hack as a hack pack,
`rust/crates/zax-nes/patches/excitebike-widescreen.zxpk` (`tools/build.py <rom> --pack <out.zxpk>`;
`docs/HACK-PACKS.md`, `WIDESCREEN-EXTENSION.md` 7.1): the code goes to the extension's memory,
which the window shows from power-on (`$5008` = `$80`, page 0), and the hooks, each with the game's
bytes it replaces, are checked and applied by Zaxnes; the ROM keeps its size (no bank added,
nothing moved or copied). The game has no testing aid. The BPS patch described here is kept to
compare.

## 2. What the game does, and what changes

Everything runs in the NMI (`$C23B`); the main loop (`$C1D8`) draws random numbers (`$D326`, a
shift register over `$18-$1F`) while it waits for the next one.

The riders are 4 slots (0 the player): active `$A8,X`, x on its page `$80,X`, the page `$84,X`
(1 the screen, 0 left of it, 2 right of it: the rider's 16-bit x relative to the screen is
`($84 - 1) * 256 + $80`), y `$8C,X`. The game moves them with the screen (`$9A7A`), makes them
appear at an edge (`$9AE3`: x `$A0` on page 0, 96 pixels left of the screen, or `$40` on page 2,
64 right of it) and removes them some 112 to 144 pixels past an edge (`$E47A`: x `$70-$8F` off
the screen's page). Each rider is drawn as 3 columns of 3 sprites (`$E17F`, `sub_D1C7`: 8-bit x)
from its OAM place (`$D8C0`: `$0204`, `$0228`, `$024C`, `$0270`); then `$E456` works out the page
of each column (`$02-$04`: the rider's, one more for a column past the end of its page) and
`sub_E49B` hides (y `$F8`, `sub_E4AD`) every column whose page is not the screen's.

| Part | The game | The hack |
|---|---|---|
| NMI (`$C24B`) | | the extension's settings each frame: `$5003 = min($5002, 248)`, extents 0 (the decoder draws the background), `$5007 = $12` (sprites free of the extents, window to `$7FFF`), `$5006` = 255 with the option "No slowdown" else 0 |
| a rider's columns off the screen (`JSR sub_E49B` at `$E492`, X the rider) | hides them | a column within the wide view (`-M-8 < x < 256+M`, x = (page - 1) * 256 + `$80` + 8 x its column) keeps its sprites, the high byte of its x (page - 1) written to `$5100` + each of its three sprites' OAM offset; any other is hidden as the game does (`sub_E4AD`) |

M is the margin of the frame (`WV_M`, at most 248). With M = 0 every column off the screen is
hidden, as the game does.

**The random numbers.** The game draws one random number for every turn of its main loop, while
it waits for the next frame: how many it draws depends on how long the frame's work took. The
hack's code adds a few dozen cycles to the NMI, which changes that count now and then: the same
inputs do not give the same race as on the console (the rivals' choices), though the race goes on
as the game's own (checked: with the hack and without margins, only the random generator's bytes
and what they decide differ from the original after the same inputs). The emulator's margin
decoder alone changes nothing: with or without margins the original ROM's RAM is the same.

## 3. Variables (extension RAM)

`WV_M $5200` margin, `WV_T0 $5201` scratch, `WV_COLUMN $5202` the rider's column at hand,
`WV_RIDER $5203` the rider. The code is at `$5800`-`$7FFF` (the window).

## 4. Hooks

Each is checked against the bytes it replaces, in both copies of the game (`$8000 + offset` and
`$C000 + offset`); NOP (`$EA`) fill to the original length.

| Address | Replaces | With |
|---|---|---|
| `$C24B` | `LDA #$00 / STA $2003` (NMI) | `JSR wide_nmi` |
| `$E492` | `JSR sub_E49B` | `JSR wide_rider_columns` |

## 5. Options

Zaxnes writes their bits at `$500A` every frame (the game's hacks screen, "Advanced mode" on).

| Bit | Key | Label | Default | Effect |
|---|---|---|---|---|
| 2 | `no_slowdown` | No slowdown / Sans ralentissements | on | `$5006 = 255`: no lag frame; with Zaxnes' yield (`idle_loop`), a frame that does not need it costs nothing more |

Measured: the original game did not lag in a Selection B race (three rivals) of 2,500 frames
(`$3F`, counted each NMI, went up once a frame); the option is there for the rest of the game.

## 6. Building

Requirements: cc65 (ca65, ld65; run through WSL when not on the PATH), Python 3, the source ROM.

```
python -I tools/build.py "Excitebike (Japan, USA) (En).nes" build/excitebikewide.bps
python -I tools/make_doc.py <zaxnes>/rust/docs/hacks/excitebike/WIDESCREEN-HACK.md
```

`build.py` assembles `src/wide.s` for `$5800` (a 16 KiB bank, `$FF` fill), writes the hooks into
both copies of the game's PRG (checked), the target data (PRG, its copy, the bank, CHR),
`build/excitebikewide.nes` (header with 3 PRG banks) and the BPS patch (`make_bps.py`: SourceRead,
SourceCopy for the copy and the moved CHR, TargetRead, TargetCopy for runs of one value).
`WIDE_SKIP=<hex addresses>` leaves hooks out, for testing. `make_doc.py` writes this document
from `tools/doc_body.md` and the sources.

## 7. Limits

* The riders appear where the game makes them: 96 pixels left of the screen or 64 right of it;
  past a margin of 64 (96 on the left) they pop in there, not at the edge of the view.
* They go where the game removes them, 112 to 144 pixels past an edge: in a wider margin they
  vanish there.
* The few sprites a rider has besides its bike (`$A26A`, at the x `$0390,X` keeps) are left as
  the game draws them (on the screen only).
* The races do not repeat the console's for the same inputs (see the random numbers above).

## Appendices

### A. `src/wide.s`

```asm
; Excitebike (Japan, USA): the widescreen hack (Zaxnes, advanced mode).
;
; This code is a 16 KiB PRG bank added after the game's 16 KiB and a copy of them (the game's
; NROM-128 PRG is mirrored at $C000: the copy keeps it so with 48 KiB); the widescreen extension's
; PRG window shows it at $5800-$7FFF ($5007 bit 4), Zaxnes opening it at power-on (the patch's
; boot window: bank 2, page 0). The game reaches this code through hooks in its own, written into
; both copies (tools/build.py). The background of the margins is not the hack's work: Zaxnes'
; margin decoder draws it.
;
;  * the riders past the screen's edges show in the margins: the game keeps them up to some 112
;    pixels past an edge and hides their sprites there; the hack gives a column of their sprites
;    within the wide view the high byte of its x instead;
;  * option: no slowdown.
;
; With a margin of 0 every routine does what the code it replaces does.

; ---------------------------------------------------------------------------- ;
; The widescreen extension (see Zaxnes, docs/WIDESCREEN-EXTENSION.md).

WIDE_HOST       = $5002         ; R: the margin the host offers
WIDE_REQ        = $5003         ; W: the margin asked for
WIDE_LEFT       = $5004         ; nametable columns shown left of the screen
WIDE_RIGHT      = $5005         ; and right of it
WIDE_OVERCLOCK  = $5006         ; extra scanlines of CPU time per frame
WIDE_LAYOUT     = $5007         ; bit 1 sprites free of the extents, bit 4 window to $7FFF
WIDE_OPTIONS    = $500A         ; R: the options the player turned on
WIDE_SPRHI      = $5100         ; + 4 n: high byte of the x of sprite n, for the next OAM DMA

MAX_MARGIN      = 248           ; the most the extension shows sprites in

; Extension RAM ($5200-$57FF; $5800 up is the window).
WV_M            = $5200         ; margin of this frame
WV_T0           = $5201         ; scratch
WV_COLUMN       = $5202         ; the rider's sprite column at hand
WV_RIDER        = $5203         ; the rider at hand

; The game.
OAM_POINTER     = $00           ; the column's sprites (sub_E4AD: $07 + its offset, page 2)
RIDER_PAGES     = $02           ; the page of each of the rider's 3 sprite columns ($02-$04):
                                ; 1 on the screen, 0 left of it, 2 right of it
RIDER_OAM       = $07           ; the rider's first sprite in the OAM page ($0200)
RIDER_X_LO      = $80           ; + rider: its x on its page
OAM             = $0200

; Places of the game.
HIDE_COLUMN     = $E4AD         ; hides the rider's sprite column Y
COLUMN_OFFSETS  = $D8DC         ; + column: the column's first sprite from the rider's (0, 12, 24)

; The options ($500A).
OPT_NO_SLOWDOWN = $04
NO_SLOWDOWN_LINES = 255

.segment "CODE"

; ---------------------------------------------------------------------------- ;
; NMI ($C24B, in place of LDA #$00 / STA $2003): the extension's settings for the frame.
wide_nmi:
    LDA     WIDE_HOST
    CMP     #MAX_MARGIN
    BCC     @margin
    LDA     #MAX_MARGIN
@margin:
    STA     WV_M
    STA     WIDE_REQ
    LDA     #$00                        ; no nametable columns: the decoder draws the background
    STA     WIDE_LEFT
    STA     WIDE_RIGHT
    LDA     #$12                        ; sprites free of those extents, the window on over
    STA     WIDE_LAYOUT                 ; $6000-$7FFF
    LDA     WIDE_OPTIONS                ; no slowdown (option bit 2)
    AND     #OPT_NO_SLOWDOWN
    BEQ     @overclock
    LDA     #NO_SLOWDOWN_LINES
@overclock:
    STA     WIDE_OVERCLOCK
    LDA     #$00
    STA     $2003
    RTS

; ---------------------------------------------------------------------------- ;
; The riders' sprite columns past the screen ($E492, in place of JSR sub_E49B; X the rider): the
; game hides each column whose page is not the screen's; within the wide view the hack shows it,
; the high byte of its x ($5100 + its sprites' OAM offsets) as its page says (page - 1).
wide_rider_columns:
    LDY     #$00
@column:
    STY     WV_COLUMN
    LDA     RIDER_PAGES,y
    LSR
    BCS     @next                       ; on the screen: as the game has it
    TYA                                 ; the column's x on its page: the rider's + 8 per column
    ASL
    ASL
    ASL
    CLC
    ADC     RIDER_X_LO,x
    STA     WV_T0
    LDA     RIDER_PAGES,y               ; its x's high byte: page - 1
    SEC
    SBC     #$01
    STX     WV_RIDER
    LDY     WV_T0
    JSR     wide_x_shown
    LDX     WV_RIDER
    LDY     WV_COLUMN
    BCC     @hide
    LDA     RIDER_PAGES,y               ; shown: its 3 sprites take the high byte
    SEC
    SBC     #$01
    STA     WV_T0
    TXA
    PHA
    LDA     RIDER_OAM
    CLC
    ADC     COLUMN_OFFSETS,y
    TAX
    LDA     WV_T0
    STA     WIDE_SPRHI,x
    STA     WIDE_SPRHI+4,x
    STA     WIDE_SPRHI+8,x
    PLA
    TAX
    JMP     @next
@hide:
    TXA
    PHA
    JSR     HIDE_COLUMN                 ; as sub_E49B does
    PLA
    TAX
@next:
    LDY     WV_COLUMN
    INY
    CPY     #$03
    BCC     @column
    RTS

; C set when the 16-bit x A:Y (high:low) of a sprite (8 pixels wide) is within the wide view:
; -M-8 < x < 256+M.
wide_x_shown:
    CMP     #$00
    BEQ     @shown
    CMP     #$01
    BEQ     @right
    CMP     #$FF
    BNE     @hidden
    LDA     WV_M
    BEQ     @hidden
    TYA
    CLC
    ADC     #$08
    BCS     @shown
    ADC     WV_M
    BCS     @shown
    BCC     @hidden
@right:
    CPY     WV_M
    BCC     @shown
@hidden:
    CLC
    RTS
@shown:
    SEC
    RTS

wide_end:
.assert wide_end <= $8000, error, "widescreen code past the window"
```

### B. `tools/build.py`

```python
"""Builds the Excitebike (Japan, USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Excitebike (Japan, USA) (En).nes> [out.bps]
       python -I tools/build.py <Excitebike (Japan, USA) (En).nes> --pack <out.zxpk>

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank, which the widescreen extension's PRG window shows at $5800-$7FFF (Zaxnes opens it at
   power-on for this patch: the game has no free byte left for the code that would).
2. Lays the PRG out as the game's 16 KiB, a copy of them, then that bank: NROM maps the first
   32 KiB at $8000-$FFFF, so the copy keeps the game's mirror at $C000 (its vectors, most of its
   calls), and the bank is only seen through the window.
3. Writes the hooks into both copies of the game, each checked against the bytes it replaces.
4. Writes build/excitebikewide.nes (header: 3 PRG banks) and, if asked, the BPS patch (data
   without iNES header; the copy and the CHR-ROM are copied from the player's ROM).

With --pack, a Zaxnes hack pack (tools/make_pack.py) instead: the code (its used part) for the
extension's memory, which Zaxnes shows in the window from power-on, and the hooks as offsets in the
PRG-ROM, the ROM keeping its size. The game's 16 KiB stay as they are (NROM mirrors them at $C000
itself); it has no testing aid.
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0x49AEB3A6
PRG_SIZE = 0x4000
CHR_SIZE = 0x2000
BANK_SIZE = 0x4000
CODE_START = 0x5800

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOP = 0xEA


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def hooks(sym):
    """The patches of the game's PRG-ROM: (address, original bytes, new bytes)."""
    out = []

    def put(address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((address, original, new))

    # NMI: the extension's settings for the frame (LDA #0 / STA $2003 done there).
    put(0xC24B, 'a9 00 8d 03 20', jsr(sym['wide_nmi']))
    # The riders' sprite columns past the screen: shown within the wide view.
    put(0xE492, '20 9b e4', jsr(sym['wide_rider_columns']))
    return out


def tool(name):
    return [name] if shutil.which(name) else ['wsl', '-e', name]


def wsl_path(path):
    if shutil.which('ca65'):
        return path
    drive, rest = os.path.splitdrive(os.path.abspath(path))
    return '/mnt/' + drive[0].lower() + rest.replace('\\', '/')


def assemble():
    build = os.path.join(ROOT, 'build')
    os.makedirs(build, exist_ok=True)
    cfg = os.path.join(build, 'wide.cfg')
    open(cfg, 'w', newline='\n').write(
        f'MEMORY {{ CODE: start = ${CODE_START:04X}, size = ${BANK_SIZE:04X}, '
        'fill = yes, fillval = $FF, file = %O; }\n'
        'SEGMENTS { CODE: load = CODE, type = ro; }\n')
    obj = os.path.join(build, 'wide.o')
    binary = os.path.join(build, 'wide.bin')
    labels = os.path.join(build, 'wide.labels')
    subprocess.run(tool('ca65') + ['-g', wsl_path(os.path.join(ROOT, 'src', 'wide.s')), '-o', wsl_path(obj)],
                   check=True)
    subprocess.run(tool('ld65') + ['-C', wsl_path(cfg), '-o', wsl_path(binary), '-Ln', wsl_path(labels),
                                   wsl_path(obj)], check=True)
    sym = {}
    for line in open(labels):
        parts = line.split()
        if len(parts) >= 3 and parts[0] == 'al':
            sym[parts[2].lstrip('.')] = int(parts[1], 16)
    return open(binary, 'rb').read(), sym


def pack(data, code, sym, out):
    sys.path.insert(0, os.path.join(ROOT, 'tools'))
    import make_pack
    widescreen = []
    for address, original, new in hooks(sym):
        offset = address & (PRG_SIZE - 1)
        assert data[offset:offset + len(original)] == original, f'${address:04X}: not the expected bytes'
        widescreen.append((offset, original, bytes(new)))
    used = (sym['wide_end'] - CODE_START + 0xFF) & ~0xFF
    result = make_pack.make('Excitebike (Japan, USA) widescreen', code[:used], [('widescreen', widescreen)])
    open(out, 'wb').write(result)
    print(f'{out}: {len(result)} bytes, pack CRC {zlib.crc32(result):08X}')


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Excitebike (Japan, USA)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    if len(sys.argv) > 3 and sys.argv[2] == '--pack':
        pack(bytes(data), code, sym, sys.argv[3])
        return
    prg = bytearray(data[:PRG_SIZE])
    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for address, original, new in hooks(sym):
        if address in skip:
            continue
        at = address & (PRG_SIZE - 1)
        assert prg[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        prg[at:at + len(new)] = new
    target = bytes(prg) + bytes(prg) + code + bytes(data[PRG_SIZE:])

    out_header = bytearray(image[:header])
    if header:
        out_header[4] = (2 * PRG_SIZE + BANK_SIZE) // 0x4000
    out = os.path.join(ROOT, 'build', 'excitebikewide.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        moved = [(PRG_SIZE, 2 * PRG_SIZE, 0),
                 (2 * PRG_SIZE + BANK_SIZE, 2 * PRG_SIZE + BANK_SIZE + CHR_SIZE, PRG_SIZE)]
        patch = make_bps.make(bytes(data), target, moved)
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
