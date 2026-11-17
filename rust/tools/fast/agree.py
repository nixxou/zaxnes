"""Agreement between two `golden-core` runs (lot FAST-1): exact core vs fast core.

Usage: python agree.py <reference.jsonl> <capture.jsonl> [--rows]
Prints, per checkpoint frame, how many ROMs have the same picture, tiles, cycles and audio,
and with --rows the ROMs whose picture differs.
"""
import json
import sys


def load(path):
    rows = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            row = json.loads(line)
            rows[row["rom"]] = row
    return rows


def main():
    ref, cap = load(sys.argv[1]), load(sys.argv[2])
    show = "--rows" in sys.argv
    roms = sorted(set(ref) & set(cap))
    fields = ["pixel_sha256", "tiles_sha256", "cycles", "audio_i16_sha256_samples"]
    frames = sorted({k for r in roms for k in ref[r].get("pixel_sha256", {})}, key=int)
    print(f"{len(roms)} ROMs")
    print("frame  " + "  ".join(f"{f.split('_')[0]:>8}" for f in fields))
    for frame in frames:
        counts = []
        for field in fields:
            same = sum(1 for r in roms if ref[r].get(field, {}).get(frame) == cap[r].get(field, {}).get(frame))
            counts.append(same)
        print(f"{frame:>5}  " + "  ".join(f"{c:>4}/{len(roms):<3}" for c in counts))
    if show:
        last = frames[-1]
        for r in roms:
            diff = [f for f in frames if ref[r]["pixel_sha256"].get(f) != cap[r]["pixel_sha256"].get(f)]
            if diff:
                print(f"  mapper {ref[r].get('mapper')!s:>3}  first picture diff at {diff[0]:>4}  {r}")


main()
