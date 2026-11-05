"""Tabulates one run's exclusive instruction counts for the symbols of the CPU read path.

    usage: callgrind-symbols.py <cg-dir> [substring ...]

<cg-dir> is a `$PR1_WORK/cg-<label>` directory. With no substring the default list below is
used: the functions a change to `NesBus::read_internal` can move, plus the per-cycle neighbours
MF-3b rule 1 says to watch (a hot callee gaining its first call site is a stop-the-lot event, and
here it shows up as a symbol appearing from nothing).

A dash means the symbol has no exclusive count in that ROM's annotation -- either inlined away,
or never entered.
"""

import re
import sys
from pathlib import Path

# The CPU interpreter is generic over the bus, so its symbols end in a turbofish
# (`Cpu6502>::step::<zax_nes::nes_bus::NesBus>`). Strip it, so that the short name in the list
# below matches.
monomorph = re.compile(r"::<[^<>]*>$")

sys.path.insert(0, str(Path(__file__).parent))
from importlib.machinery import SourceFileLoader  # noqa: E402

_diff = SourceFileLoader("cgdiff", str(Path(__file__).parent / "callgrind-diff.py")).load_module()

DEFAULT = [
    "NesBus>::read_internal",
    "NesBus>::read_cycle",
    "NesBus>::halted_read_cycle",
    "NesBus>::write_internal",
    "NesBus>::tick",
    "Ppu2C02>::read",
    "Mapper>::cpu_read_override",
    "Cartridge>::try_cpu_read",
    "Cartridge>::default_cpu_read",
    "Cartridge>::read_rom_window",
    "Cpu6502>::step",
    "Cpu6502>::resolve",
    "Cpu6502>::branch",
    "Cpu6502>::execute_without_operand",
]


def main() -> int:
    d = Path(sys.argv[1])
    wanted = sys.argv[2:] or DEFAULT
    keys = [p.stem for p in sorted(d.glob("*.ann"))]
    per = {k: _diff.group(_diff.symbols(d, k)) for k in keys}

    width = max(len(w) for w in wanted) + 2
    print(f"{'symbol':{width}}" + "".join(f"{k:>15}" for k in keys))
    for w in wanted:
        cells = []
        for k in keys:
            # Tail match and **no** substring fallback: `Ppu2C02>::read` must not quietly sum
            # `read_vram_inner`, `read_nametable_inner` and `read_pattern_planes` with it, which
            # is what a substring match does and what made the first draft of this table read
            # 71 M instructions for 71 000 register reads.
            hit = [n for n in per[k] if monomorph.sub("", n).endswith(w)]
            cells.append(f"{sum(per[k][n] for n in hit):,}" if hit else "-")
        print(f"{w:{width}}" + "".join(f"{c:>15}" for c in cells))
    return 0


if __name__ == "__main__":
    sys.exit(main())
