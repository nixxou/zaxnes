"""Compares the stage tables of two `callgrind-l2.sh` runs, the way R0 compares them.

    usage: l2-diff.py <dir-a> <dir-b> [--bar]

<dir-a> and <dir-b> are two `$PR1_WORK/l2-<label>` directories. The table is R0's
`vs-<baseline>.txt`: one row per game and stage, the two counts and the percentage.

`--bar` prints only the two rows lot MF-5b-fix is judged on -- `total` and `core` -- and a
verdict line, a variant being keepable only if **no game is above the baseline** on either.

The counts come from `stages.txt`, which is an *inclusive* count per function: `total` is
`bench_core::run` and `core` is `NesCore::run_frame`. They are therefore not the whole-process
figure `<key>.ir` holds (which includes the loader, the ROM read and the process's exit), and the
percentages R0 publishes are these.
"""

import sys
from pathlib import Path

ORDER = [
    "apu",
    "convert",
    "core",
    "draw_list",
    "load_profile",
    "lua_hooks",
    "ppu",
    "total",
]


def stages(d: Path) -> dict[tuple[str, str], int]:
    f = d / "stages.txt"
    if not f.exists():
        print(f"l2-diff: no {f}", file=sys.stderr)
        return {}
    out: dict[tuple[str, str], int] = {}
    for line in f.read_text(errors="replace").splitlines():
        parts = line.split()
        if len(parts) == 3:
            out[(parts[0], parts[1])] = int(parts[2])
    return out


def main() -> int:
    argv = [a for a in sys.argv[1:] if not a.startswith("--")]
    bar_only = "--bar" in sys.argv[1:]
    a, b = Path(argv[0]), Path(argv[1])
    sa, sb = stages(a), stages(b)
    if not sa or not sb:
        return 1

    games = sorted({g for g, _ in sa} | {g for g, _ in sb})
    wanted = ["total", "core"] if bar_only else ORDER
    worse: list[str] = []
    for game in games:
        for stage in wanted:
            x, y = sa.get((game, stage)), sb.get((game, stage))
            if x is None or y is None:
                print(f"{game}/{stage:<14} missing on one side")
                continue
            flag = ""
            if y > x:
                flag = "  ABOVE"
                worse.append(f"{game}/{stage}")
            elif y == x:
                flag = "  equal"
            print(
                f"{game + '/' + stage:<24}{x:>15,}{y:>15,}{y - x:>+15,}"
                f"{(y - x) / x * 100:>+9.3f}%{flag}"
            )
        if not bar_only:
            print()

    print()
    if worse:
        print(f"VERDICT: ABOVE the baseline on {len(worse)}: {' '.join(worse)}")
    else:
        print("VERDICT: at or below the baseline on every game, total and core")
    return 0


if __name__ == "__main__":
    sys.exit(main())
