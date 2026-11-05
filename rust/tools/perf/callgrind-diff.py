"""Compares two `callgrind-set.sh` runs: the whole process per ROM, then the symbols that moved.

    usage: callgrind-diff.py <dir-a> <dir-b> [symbol-threshold]

<dir-a> and <dir-b> are two `$PR1_WORK/cg-<label>` directories. The verdict line at the end is
the one lot PR-1 is judged on: a variant is keepable only if **no ROM of the set is slower**,
to the instruction, and the total is lower.

`symbol-threshold` (default 100000) is the smallest exclusive-count move worth printing.
"""

import re
import sys
from pathlib import Path

# `callgrind_annotate` lines look like
#     240,208,974 ( 3.88%)  ???:<zax_nes::nes_bus::NesBus>::read_internal [/path/to/zax-harness]
# with the self (exclusive) count first, the source file (`???` without debug line info) before
# the first colon, and the object in brackets at the end for everything but the main one. A Rust
# name carries spaces of its own (`<T as Trait>::m`), so the symbol is everything between the
# first colon and an optional bracket.
LINE = re.compile(r"^\s*([\d,]+)\s*(?:\([^)]*\))?\s+[^:\s]*:(.+?)(?:\s+\[[^\]]*\])?\s*$")
TOTALS = re.compile(r"^\s*([\d,]+)\s*\([^)]*\)\s+PROGRAM TOTALS\s*$")


def ir(d: Path, key: str) -> int | None:
    """The whole-process count, taken from the annotation itself so a mispaired `.ir` cannot lie.

    The `.ir` file `callgrind-set.sh` writes from valgrind's own "Collected :" line is used as a
    cross-check: a disagreement means the two files come from different runs, which is reported
    rather than silently averaged.
    """
    ann, counted = d / f"{key}.ann", d / f"{key}.ir"
    total = None
    if ann.exists():
        for line in ann.read_text(errors="replace").splitlines():
            m = TOTALS.match(line)
            if m:
                total = int(m.group(1).replace(",", ""))
                break
    if counted.exists() and (text := counted.read_text().strip()):
        collected = int(text)
        if total is None:
            total = collected
        elif total != collected:
            print(f"{key}: WARNING .ann says {total:,} and .ir says {collected:,} -- mispaired")
    return total


def symbols(d: Path, key: str) -> dict[str, int]:
    f = d / f"{key}.ann"
    out: dict[str, int] = {}
    if not f.exists():
        return out
    for line in f.read_text(errors="replace").splitlines():
        if TOTALS.match(line):
            continue
        m = LINE.match(line)
        if not m:
            continue
        name = m.group(2)
        out[name] = out.get(name, 0) + int(m.group(1).replace(",", ""))
    return out


def group(sym: dict[str, int]) -> dict[str, int]:
    """Folds a symbol's LLVM-cloned copies (`...::foo.llvm.123`, `foo.cold`) into one entry."""
    out: dict[str, int] = {}
    for name, count in sym.items():
        base = re.sub(r"\.llvm\.\d+$", "", name)
        base = re.sub(r"\.cold$", "", base)
        out[base] = out.get(base, 0) + count
    return out


def main() -> int:
    a, b = Path(sys.argv[1]), Path(sys.argv[2])
    floor = int(sys.argv[3]) if len(sys.argv) > 3 else 100_000
    keys = [p.stem for p in sorted(a.glob("*.ir"))]

    print(f"{'rom':10}{a.name:>18}{b.name:>18}{'delta':>16}{'%':>9}")
    worse, total_a, total_b = [], 0, 0
    for key in keys:
        x, y = ir(a, key), ir(b, key)
        if x is None or y is None:
            print(f"{key:10}{'-':>18}{'-':>18}  missing on one side")
            continue
        total_a += x
        total_b += y
        pct = (y - x) / x * 100
        flag = ""
        if y > x:
            worse.append(key)
            flag = "  SLOWER"
        elif y == x:
            flag = "  equal"
        print(f"{key:10}{x:>18,}{y:>18,}{y - x:>+16,}{pct:>+9.3f}{flag}")
    if total_a:
        print(
            f"{'TOTAL':10}{total_a:>18,}{total_b:>18,}{total_b - total_a:>+16,}"
            f"{(total_b - total_a) / total_a * 100:>+9.3f}"
        )

    for key in keys:
        sa, sb = group(symbols(a, key)), group(symbols(b, key))
        moved = sorted(
            (
                (sb.get(n, 0) - sa.get(n, 0), n)
                for n in set(sa) | set(sb)
                if abs(sb.get(n, 0) - sa.get(n, 0)) >= floor
            ),
            key=lambda t: -abs(t[0]),
        )
        # What the per-symbol table does not explain. `callgrind_annotate` folds the tail of the
        # list away, so a few million always sit outside it; a *moving* remainder means the
        # change is hiding somewhere this table cannot show, and has to be said rather than
        # left to be noticed.
        ta, tb = ir(a, key), ir(b, key)
        rest_a = (ta - sum(sa.values())) if ta is not None else 0
        rest_b = (tb - sum(sb.values())) if tb is not None else 0
        if not moved and abs(rest_b - rest_a) < floor:
            continue
        print(f"\n{key}: symbols that moved by {floor:,} or more")
        for delta, name in moved:
            print(f"  {delta:>+16,}  {sa.get(name, 0):>16,} -> {sb.get(name, 0):>16,}  {name}")
        if abs(rest_b - rest_a) >= floor:
            print(
                f"  {rest_b - rest_a:>+16,}  {rest_a:>16,} -> {rest_b:>16,}"
                "  (not in the annotation's list)"
            )

    print()
    if worse:
        print(f"VERDICT: SLOWER on {len(worse)} ROM(s): {' '.join(worse)}")
    elif total_b < total_a:
        print("VERDICT: no ROM slower, total lower -- keepable")
    else:
        print("VERDICT: no ROM slower, but the total is not lower")
    return 0


if __name__ == "__main__":
    sys.exit(main())
