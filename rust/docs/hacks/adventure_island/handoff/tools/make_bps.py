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
