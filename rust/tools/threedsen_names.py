"""Fills `profiles/3dsen_names.csv`: which game of the game database each 3dSen profile is for,
by its name only (the file name without its extension; nothing inside the profile is read, its
format having needed workarounds).

    profile,game,only

`only` restricts a profile to some ROM images of its game, separated by `;`: exact No-Intro names
(the Tengen Tetris and the Nintendo one are two profiles of the game `Tetris`) or data CRCs in
eight hexadecimal digits (a profile made for a hack, whose image No-Intro does not list). A game
may have several profiles. An empty `game` is a profile no game of the database matches: it keeps
the name rule of the list.

Lines already in the file are kept as they are (hand-made ones win); the profiles of the folder
it does not have yet are added, matched by their cleaned name (tags in brackets and region words
dropped, letters and digits compared in lowercase) against the Hackset game ids, then against the
No-Intro names of their images (`Contra` is an image of the game `Probotector`), then against the
start of a game id when only one has it.

Run:  python tools/threedsen_names.py <Hackset metadata folder> <folder of 3dSen profiles>
"""

import csv
import io
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "profiles", "3dsen_names.csv")
TAGS = re.compile(r"\([^)]*\)|\[[^]]*\]")
REGION_WORDS = re.compile(r"\b(USA|U|J|E|EUR|Europe|Japan|World|PRG ?\d)\b", re.IGNORECASE)
HEADER = [
    "# The game of each 3dSen profile, by its name (tools/threedsen_names.py, then by hand).",
    "# only: exact No-Intro names or data CRCs, separated by ';', when the profile fits only them.",
]


def normalise(name: str) -> str:
    return "".join(c for c in name.lower() if c.isalnum())


def cleaned(name: str) -> str:
    return normalise(REGION_WORDS.sub("", TAGS.sub("", name).replace("_", " ")))


def indexes(hackset: str) -> tuple[dict[str, str], dict[str, set[str]]]:
    """The games by normalised id, and by cleaned No-Intro name of their images."""
    by_id: dict[str, str] = {}
    by_no_intro: dict[str, set[str]] = {}
    for file in sorted(os.listdir(hackset)):
        if not file.endswith(".json"):
            continue
        game = file[: -len(".json")]
        by_id.setdefault(normalise(game), game)
        try:
            data = json.load(open(os.path.join(hackset, file), encoding="utf-8"))
        except (OSError, ValueError):
            continue
        if not isinstance(data, dict):
            continue
        for entry in data.values():
            name = (entry.get("nointro") or {}).get("name")
            if name:
                by_no_intro.setdefault(cleaned(name), set()).add(game)
    return by_id, by_no_intro


def match(stem: str, by_id: dict[str, str], by_no_intro: dict[str, set[str]]) -> str:
    key = cleaned(stem)
    if key in by_id:
        return by_id[key]
    if len(by_no_intro.get(key, ())) == 1:
        return next(iter(by_no_intro[key]))
    if len(key) > 5:
        candidates = [game for k, game in by_id.items() if k.startswith(key)]
        if len(candidates) == 1:
            return candidates[0]
    return ""


def main() -> None:
    hackset, profiles = sys.argv[1], sys.argv[2]
    by_id, by_no_intro = indexes(hackset)

    kept: dict[str, list[str]] = {}
    if os.path.isfile(OUT):
        with open(OUT, encoding="utf-8", newline="") as file:
            for row in csv.reader(line for line in file if not line.startswith("#")):
                if row and row[0] != "profile":
                    kept[row[0]] = (row + ["", ""])[:3]

    added = matched = 0
    for file in sorted(os.listdir(profiles)):
        stem, extension = os.path.splitext(file)
        if extension.lower() not in (".3dn", ".3dh") or stem in kept:
            continue
        game = match(stem, by_id, by_no_intro)
        kept[stem] = [stem, game, ""]
        added += 1
        matched += bool(game)

    buffer = io.StringIO()
    for line in HEADER:
        buffer.write(line + "\n")
    writer = csv.writer(buffer, lineterminator="\n")
    writer.writerow(["profile", "game", "only"])
    for name in sorted(kept, key=str.lower):
        writer.writerow(kept[name])
    with open(OUT, "w", encoding="utf-8", newline="\n") as file:
        file.write(buffer.getvalue())
    unmatched = sorted((name for name, row in kept.items() if not row[1]), key=str.lower)
    print(f"{added} added ({matched} matched), {len(kept)} profiles, {len(unmatched)} without a game:")
    for name in unmatched:
        print("   ", name)


if __name__ == "__main__":
    main()
