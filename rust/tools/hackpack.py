"""Builds Zaxnes hack packs, format 2 (see `zax-nes/src/hack_pack.rs`, `docs/HACK-PACKS.md`).

The one builder the hacks' repositories share: a hack's `tools/build.py` imports it (with this
folder on `sys.path`, say from the environment variable ZAXNES_TOOLS) instead of writing the pack
itself.

    pack = hackpack.Pack('Zelda II (USA) widescreen', code, region='ntsc', mappers=(1, 4))
    core = pack.component('widescreen').group('core')
    core.hook(7, 0xC0AF, 'ae 02 20', jsr(nmi))       # bank 7 of 8: the last, fixed bank
    core.check(7, 0xD5DA, 6)                          # a routine the hack's code calls
    world = pack.component('widescreen').group('world map', requires=['core'])
    pack.component('invincible').group('invincible').hook(7, 0xE2F1, 'ad 18 05', b'\\xa9\\x01\\xea')
    open(out, 'wb').write(pack.build(data, prg_len))

A place is a 16 KiB bank of the reference ROM and a CPU address; the bank is written counted
from the end when it is the last (the fixed bank of MMC1, and of a game that grows), from the
start otherwise, so a ROM whose banks were added to (a translation, a ROM hack) finds its hooks.
`build` checks every hook's original bytes and reads every check's bytes in the reference ROM.

A component applies to a ROM when its first group does; each other group on its own, with the
groups it requires. Components are judged apart: the widescreen and each aid never depend on
each other.
"""

import struct

FORMAT = 2
BANK = 0x4000
REGIONS = {'ntsc': 0, 'pal': 1, 'any': 0xFF}


class Group:
    def __init__(self, name, requires):
        self.name = name
        self.requires = list(requires)
        self.hooks = []         # (bank, address, original bytes, new bytes)
        self.checks = []        # (bank, address, length)

    def hook(self, bank, address, original, new):
        """Replaces the game's `original` bytes (bytes or a hex string) at `address` of PRG bank
        `bank` (16 KiB) with `new`, padded with NOP to the same length."""
        if isinstance(original, str):
            original = bytes.fromhex(original)
        new = bytes(new)
        assert len(new) <= len(original), (hex(address), 'longer than what it replaces')
        new += bytes([0xEA]) * (len(original) - len(new))
        assert 0 < len(original) < 256, hex(address)
        self.hooks.append((bank, address, bytes(original), new))
        return self

    def check(self, bank, address, length):
        """Requires the `length` bytes at `address` of bank `bank` to be the reference ROM's: code
        or data of the game the hack's code relies on."""
        assert 0 < length < 256, hex(address)
        self.checks.append((bank, address, length))
        return self


class Component:
    def __init__(self, name):
        self.name = name
        self.groups = []

    def group(self, name, requires=()):
        """The group called `name`, made the first time with the groups it `requires` (names
        of earlier groups of this component); the first group is the component itself."""
        for group in self.groups:
            if group.name == name:
                return group
        for required in requires:
            assert any(group.name == required for group in self.groups), (name, required)
        group = Group(name, requires)
        self.groups.append(group)
        return group


class Pack:
    def __init__(self, name, code, region='ntsc', mappers=()):
        self.name = name
        self.code = bytes(code)
        self.region = REGIONS[region]
        self.mappers = list(mappers)
        self.components = []

    def component(self, name):
        """The component called `name` ('widescreen', or the key of an aid's option)."""
        for component in self.components:
            if component.name == name:
                return component
        component = Component(name)
        self.components.append(component)
        return component

    def build(self, data, prg_len):
        """The pack's bytes, every hook checked and every check read in `data` (the reference
        ROM's PRG-ROM then CHR-ROM, its PRG-ROM `prg_len` bytes)."""
        banks = prg_len // BANK
        out = bytearray(b'ZXPK') + bytes([FORMAT]) + text(self.name)
        out += bytes([self.region, len(self.mappers)])
        for mapper in self.mappers:
            out += struct.pack('<H', mapper)
        out += struct.pack('<I', len(self.code)) + self.code
        out.append(len(self.components))
        taken = set()
        for component in self.components:
            assert component.groups, component.name
            out += text(component.name) + bytes([len(component.groups)])
            names = [group.name for group in component.groups]
            for group in component.groups:
                out += text(group.name) + bytes([len(group.requires)])
                out += bytes(names.index(required) for required in group.requires)
                out += struct.pack('<H', len(group.hooks))
                for bank, address, original, new in group.hooks:
                    offset = offset_of(bank, address)
                    assert data[offset:offset + len(original)] == original, \
                        f'{component.name}/{group.name}: bank {bank} ${address:04X}: not the expected bytes'
                    span = set(range(offset, offset + len(original)))
                    assert not span & taken, f'two hooks at bank {bank} ${address:04X}'
                    taken |= span
                    out += place(bank, address, banks) + bytes([len(original)]) + original + new
                out += struct.pack('<H', len(group.checks))
                for bank, address, length in group.checks:
                    offset = offset_of(bank, address)
                    out += place(bank, address, banks) + bytes([length]) + bytes(data[offset:offset + length])
        return bytes(out)


def offset_of(bank, address):
    return bank * BANK + (address & (BANK - 1))


def place(bank, address, banks):
    """A place: the bank counted from the end when it is the reference ROM's last."""
    assert 0 <= bank < banks, bank
    spec = 0x80 if bank == banks - 1 else bank
    assert spec == 0x80 or bank < 0x80
    return bytes([spec]) + struct.pack('<H', address)


def text(value):
    data = value.encode('utf-8')
    assert len(data) < 256
    return bytes([len(data)]) + data
