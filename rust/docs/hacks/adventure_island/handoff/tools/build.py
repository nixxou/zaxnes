"""Builds the Adventure Island (USA) widescreen hack (advanced mode) and its BPS patch.

Usage: python -I tools/build.py <Adventure Island (USA).nes> [out.bps]

1. Assembles src/wide.s (ca65 + ld65, run through WSL when not on the PATH) for $5800: a 16 KiB
   PRG bank added after the game's 32 KiB, which the widescreen extension's PRG window shows at
   $5800-$7FFF (Zaxnes opens it at power-on for this patch: the game has no room left for the
   code that would). CNROM maps the first 32 KiB at $8000-$FFFF, so the game runs as it was.
2. Writes the hooks into the game's PRG-ROM, each checked against the bytes it replaces.
3. Writes build/aiwide.nes (header: 3 PRG banks) and, if asked, the BPS patch (data without
   iNES header; the CHR-ROM, moved after the new bank, is copied from the player's ROM).
"""

import os
import shutil
import subprocess
import sys
import zlib

SOURCE_CRC = 0xF8A713BE
PRG_SIZE = 0x8000
CHR_SIZE = 0x8000
BANK_SIZE = 0x4000
CODE_START = 0x5800

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

BCS = 0xB0
NOP = 0xEA
RTS = 0x60


def jsr(target):
    return bytes([0x20, target & 0xFF, target >> 8])


def jmp(target):
    return bytes([0x4C, target & 0xFF, target >> 8])


def branch(opcode, at, target):
    offset = target - (at + 2)
    assert -128 <= offset <= 127, (hex(at), hex(target))
    return bytes([opcode, offset & 0xFF])


def hooks(sym):
    """The patches of the game's PRG-ROM: (address, original bytes, new bytes)."""
    out = []

    def put(address, original, new):
        original = bytes.fromhex(original)
        new = bytes(new) + bytes([NOP]) * (len(original) - len(new))
        assert len(new) == len(original), hex(address)
        out.append((address, original, new))

    # NMI: the extension's settings for the frame (LDA #<ram_oam / STA $2003 done there).
    put(0xB717, 'a9 00 8d 03 20', jsr(sym['wide_nmi']))
    # Sprites in the margins: an object past the screen, each of its sprites, a single sprite.
    put(0xC305, 'a4 59 88 10 68', jsr(sym['wide_object_shown']))
    put(0xC48B, 'a5 5b d0 39', jsr(sym['wide_sprite']))
    put(0xC4A9, 'a5 5b d0 1b', jsr(sym['wide_sprite']))
    put(0xC378, 'a5 59 d0 27', jsr(sym['wide_one_sprite']))
    # Ghosts: objects past the left edge kept in the left margin, without behaviour or collision.
    put(0x8EA7, 'bd 84 05 38 e5 00 bd 94 05 e5 01 30 0e',
        jsr(sym['wide_left_gone']) + branch(BCS, 0x8EAA, 0x8EC2))
    put(0x8E8E, '20 9e 92', jsr(sym['wide_handler']))
    put(0xBFF2, 'bd 14 06 18 79 14 06', jsr(sym['wide_collision']))
    put(0x8DB6, 'bd 54 05 f0 04 ca d0 f8',
        jsr(sym['wide_find_slot']) + branch(BCS, 0x8DB9, 0x8DBF) + bytes([RTS]))
    put(0x9534, '9d 54 05 60', jmp(sym['wide_object_state']))
    put(0x8C41, '8e 36 05', jsr(sym['wide_clear_ghosts']))
    # Options: the level selector on the title screen, invincibility.
    put(0xAE51, 'c9 10 d0 3a', jsr(sym['wide_title_button']))
    put(0x8078, 'a9 00 85 37 85 38', jsr(sym['wide_start_level']))
    put(0xC04D, 'a5 72 05 70', jsr(sym['wide_injury']))
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


def main():
    rom_path = sys.argv[1]
    image = bytearray(open(rom_path, 'rb').read())
    header = 16 if image[:4] == b'NES\x1a' else 0
    data = image[header:header + PRG_SIZE + CHR_SIZE]
    assert zlib.crc32(data) == SOURCE_CRC, 'not Adventure Island (USA)'

    code, sym = assemble()
    assert len(code) == BANK_SIZE
    prg = bytearray(data[:PRG_SIZE])
    skip = {int(a, 16) for a in os.environ.get('WIDE_SKIP', '').split(',') if a}
    for address, original, new in hooks(sym):
        if address in skip:
            continue
        at = address - 0x8000
        assert prg[at:at + len(original)] == original, f'${address:04X}: not the expected bytes'
        prg[at:at + len(new)] = new
    target = bytes(prg) + code + bytes(data[PRG_SIZE:])

    out_header = bytearray(image[:header])
    if header:
        out_header[4] = (PRG_SIZE + BANK_SIZE) // BANK_SIZE
    out = os.path.join(ROOT, 'build', 'aiwide.nes')
    open(out, 'wb').write(bytes(out_header) + target)
    print(f'{out}: data CRC {zlib.crc32(target):08X}')
    if len(sys.argv) > 2:
        sys.path.insert(0, os.path.join(ROOT, 'tools'))
        import make_bps
        moved = [(PRG_SIZE + BANK_SIZE, PRG_SIZE + BANK_SIZE + CHR_SIZE, PRG_SIZE)]
        patch = make_bps.make(bytes(data), target, moved)
        open(sys.argv[2], 'wb').write(patch)
        print(f'{sys.argv[2]}: {len(patch)} bytes')


if __name__ == '__main__':
    main()
