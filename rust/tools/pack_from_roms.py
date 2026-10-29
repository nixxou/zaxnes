"""Writes a Zaxnes hack pack (see docs/HACK-PACKS.md) from a ROM and the hacked ROM built from it,
for a hack that keeps the ROM's size: no code for the extension's memory, the bytes the hack
changes as the "widescreen" component, each run of changed bytes (those less than 8 apart joined,
at most 255) with the game's bytes it replaces. The iNES headers are left out.

Usage: python -I tools/pack_from_roms.py <game.nes> <hacked.nes> <name> <out.zxpk>

The same format as the hack repositories' tools/make_pack.py (mm6wide), whose runs() this is.
"""

import struct
import sys
import zlib

FORMAT = 1


def data(path):
    image = open(path, 'rb').read()
    return image[16:] if image[:4] == b'NES\x1a' else image


def text(value):
    encoded = value.encode('utf-8')
    assert len(encoded) < 256
    return bytes([len(encoded)]) + encoded


def runs(source, target, gap=8):
    assert len(source) == len(target), 'the hack does not keep the ROM\'s size'
    out = []
    at = 0
    while at < len(source):
        if source[at] == target[at]:
            at += 1
            continue
        end = at + 1
        while end < len(source) and end - at < 255:
            if source[end] != target[end]:
                end += 1
            elif any(source[k] != target[k] for k in range(end, min(end + gap, len(source), at + 255))):
                end += 1
            else:
                break
        out.append((at, source[at:end], target[at:end]))
        at = end
    return out


def main():
    source, target = data(sys.argv[1]), data(sys.argv[2])
    hooks = runs(source, target)
    out = bytearray(b'ZXPK') + bytes([FORMAT]) + text(sys.argv[3])
    out += struct.pack('<I', 0)
    out.append(1)
    out += text('widescreen') + struct.pack('<H', len(hooks))
    for offset, original, new in hooks:
        out += struct.pack('<IB', offset, len(new)) + original + new
    open(sys.argv[4], 'wb').write(out)
    print(f'{sys.argv[4]}: {len(out)} bytes, {len(hooks)} hooks, pack CRC {zlib.crc32(out):08X}')


if __name__ == '__main__':
    main()
