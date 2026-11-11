"""Sets the states section of hack packs already built (see `zax-nes/src/hack_pack.rs`).

    python -I tools/pack_states.py <pack.zxpk>... [--to-off] [--to-on] [--redraw ADDR:EXPECTED:VALUE]... [--unwind N]

The section of kind 1 is replaced (or added), other sections are kept, the hack's bytes before
them are left as they are: the pack's version, which the states it made are checked against, does
not change. EXPECTED is a hex byte or `-` for "always". N is the bytes the game's wait pushes above its
return address, for a hack that calls the wait from its own code (see `hackpack.py`). With no flag and no redraw step the
section is removed. Prints each pack with its version (the CRC-32 of the hack's bytes).

A hack whose repository builds its pack calls `hackpack.Pack.states` instead, with the same
values, so that a rebuild gives the same bytes.
"""

import struct
import sys
import zlib

sys.path.insert(0, __import__('os').path.dirname(__file__))
import hackpack  # noqa: E402


def hack_length(data):
    """The length of the hack: the pack's bytes before its sections (format 1 or 2)."""
    at = 5
    def u8():
        nonlocal at
        at += 1
        return data[at - 1]
    def u16():
        nonlocal at
        at += 2
        return struct.unpack_from('<H', data, at - 2)[0]
    def u32():
        nonlocal at
        at += 4
        return struct.unpack_from('<I', data, at - 4)[0]
    def skip(count):
        nonlocal at
        at += count
    def text():
        skip(u8())
    assert data[:4] == b'ZXPK', 'not a hack pack'
    form = data[4]
    text()
    if form == 2:
        u8()
        for _ in range(u8()):
            u16()
    skip(u32())
    for _ in range(u8()):
        text()
        if form == 1:
            for _ in range(u16()):
                u32()
                skip(2 * u8())
            continue
        for _ in range(u8()):
            text(); text(); text()
            skip(u8())
            for _ in range(u16()):
                skip(3)
                skip(2 * u8())
            for _ in range(u16()):
                skip(3)
                skip(u8())
    return at


def main(argv):
    packs, to_off, to_on, redraw, unwind = [], False, False, [], None
    args = iter(argv)
    for arg in args:
        if arg == '--to-off':
            to_off = True
        elif arg == '--to-on':
            to_on = True
        elif arg == '--unwind':
            unwind = int(next(args))
        elif arg == '--redraw':
            address, expected, value = next(args).split(':')
            redraw.append((int(address, 16), None if expected == '-' else int(expected, 16),
                           int(value, 16)))
        else:
            packs.append(arg)
    for path in packs:
        data = open(path, 'rb').read()
        hack = hack_length(data)
        kept = bytearray()
        at = hack
        while at < len(data):
            kind = data[at]
            length = struct.unpack_from('<H', data, at + 1)[0]
            if kind != hackpack.STATES_SECTION:
                kept += data[at:at + 3 + length]
            at += 3 + length
        assert at == len(data), f'{path}: a section runs past the end'
        out = data[:hack] + bytes(kept)
        if to_off or to_on or redraw or unwind is not None:
            out += hackpack.section(hackpack.STATES_SECTION,
                                    hackpack.states(to_off, to_on, redraw, unwind))
        open(path, 'wb').write(out)
        print(f'{path}: {len(out)} bytes, version {zlib.crc32(data[:hack]):08X}')


if __name__ == '__main__':
    main(sys.argv[1:])
