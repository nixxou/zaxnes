"""Builds the catalogue's game database: `crates/zax-app/data/gamedb_roms.csv` (one line per ROM
image, by data CRC) and `crates/zax-app/data/gamedb_games.csv` (one line per game).

Sources:
- the Hackset metadata folder (one JSON per game, named after it: `Blaster Master.json` is the game
  `Blaster Master`), for the No-Intro names, the hashes and the GoodSet whole-file CRCs;
- the Rust tables that name ROMs by data CRC (`builtin_patches.rs`, `wide_decode/*.rs`,
  `fceux_validated.rs`), so that every ROM Zaxnes has a feature for is in the database, a homebrew
  the Hackset does not know included (its game is the one the table gives);
- `profiles/index.csv`, the 3D profile built in for each game.

Kept: every No-Intro image and every image a Rust table names. The data CRC is the CRC-32 of the
PRG and CHR at the header's sizes (`builtin_patches::data_crc`), the Hackset's `rom_crc32`.

    gamedb_roms.csv:  data_crc,game,no_intro_name,region,data_md5,file_crc32s
    gamedb_games.csv: game,profile

`file_crc32s` are space-separated CRCs of whole files known to hold the image (GoodSet). Lines are
sorted by key; `build.rs` turns both files into Rust tables.

Run:  python tools/gamedb_build.py <Hackset metadata folder>
"""

import csv
import glob
import io
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..")
OUT = os.path.join(ROOT, "crates", "zax-app", "data")
# No-Intro region names as they appear between parentheses in a No-Intro name.
REGION = re.compile(r"\(([^()]*)\)")
KNOWN_REGIONS = {
    "USA", "Europe", "Japan", "World", "Asia", "Australia", "Brazil", "Canada", "China",
    "France", "Germany", "Hong Kong", "Italy", "Korea", "Netherlands", "Russia", "Spain",
    "Sweden", "Taiwan", "United Kingdom", "Unknown",
}


def region_of(name: str) -> str:
    """The No-Intro regions of `name` (`USA, Europe` -> `USA Europe`), empty when none."""
    for group in REGION.findall(name or ""):
        parts = [part.strip() for part in group.split(",")]
        if parts and all(part in KNOWN_REGIONS for part in parts):
            return " ".join(part.replace(" ", "_") for part in parts)
    return ""


def rust_tables() -> dict[int, tuple[str, str]]:
    """Every data CRC the Rust tables name, with the game the table gives (may be empty) and the
    file it is in."""
    found: dict[int, tuple[str, str]] = {}
    crates = os.path.join(ROOT, "crates")
    sources = [
        os.path.join(crates, "zax-nes", "src", "builtin_patches.rs"),
        os.path.join(crates, "zax-app", "src", "emulation", "fceux_validated.rs"),
    ] + sorted(glob.glob(os.path.join(crates, "zax-nes", "src", "wide_decode", "*.rs")))
    for path in sources:
        text = open(path, encoding="utf-8").read()
        name = os.path.relpath(path, ROOT).replace("\\", "/")
        # A struct literal with a game and a CRC (BuiltinPatch, ValidatedRom).
        for block in re.findall(r"\{[^{}]*\}", text):
            game = re.search(r'\bgame: "([^"]*)"', block)
            crc = re.search(r"\b(?:source_crc|crc): 0x([0-9A-Fa-f_]{8,9})\b", block)
            if game and crc:
                found.setdefault(int(crc.group(1).replace("_", ""), 16), (game.group(1), name))
        # A constant (wide_decode's SOURCE_CRC*).
        for crc in re.findall(r"const SOURCE_CRC\w*: u32 = 0x([0-9A-Fa-f_]{8,9});", text):
            found.setdefault(int(crc.replace("_", ""), 16), ("", name))
    return found


def main() -> None:
    folder = sys.argv[1]
    rows: dict[int, dict] = {}
    seen_games: set[str] = set()
    hackset: dict[int, dict] = {}
    for file in sorted(os.listdir(folder)):
        if not file.endswith(".json"):
            continue
        game = file[: -len(".json")]
        try:
            data = json.load(open(os.path.join(folder, file), encoding="utf-8"))
        except (OSError, ValueError):
            continue
        if not isinstance(data, dict):
            continue
        seen_games.add(game)
        for entry in data.values():
            header = entry.get("nesheaderinfo") or {}
            crc_text = header.get("rom_crc32")
            if not crc_text:
                continue
            crc = int(crc_text, 16)
            row = hackset.setdefault(crc, {
                "game": game, "no_intro_name": "", "md5": (header.get("rom_md5") or "").lower(),
                "files": set(), "games": set(),
            })
            row["games"].add(game)
            nointro = entry.get("nointro") or {}
            if nointro.get("name") and not row["no_intro_name"]:
                row["no_intro_name"] = nointro["name"]
                row["game"] = game
            goodset = entry.get("goodset") or {}
            if goodset.get("crc"):
                row["files"].add(goodset["crc"].upper())

    for crc, row in hackset.items():
        if row["no_intro_name"]:
            rows[crc] = row

    warnings = []
    for crc, (game, source) in sorted(rust_tables().items()):
        if crc in hackset:
            rows[crc] = hackset[crc]
            if game and game != rows[crc]["game"]:
                warnings.append(f"{crc:08X}: {source} says '{game}', the Hackset '{rows[crc]['game']}'")
        elif game:
            rows[crc] = {"game": game, "no_intro_name": "", "md5": "", "files": set(), "games": {game}}
            warnings.append(f"{crc:08X}: not in the Hackset, kept as '{game}' ({source})")
        else:
            warnings.append(f"{crc:08X}: not in the Hackset and no game given ({source}), left out")

    profiles = {}
    index = os.path.join(ROOT, "profiles", "index.csv")
    with open(index, encoding="utf-8", newline="") as file:
        for record in csv.DictReader(line for line in file if not line.startswith("#")):
            profiles[record["game"]] = record["profile"]
            if not os.path.isfile(os.path.join(ROOT, "profiles", record["profile"])):
                sys.exit(f"profiles/index.csv: no file {record['profile']}")
    games_with_rows = {row["game"] for row in rows.values()}
    for game in profiles:
        if game not in games_with_rows:
            sys.exit(f"profiles/index.csv: no ROM of the game '{game}'")
    for path in glob.glob(os.path.join(ROOT, "profiles", "*.3dn")):
        if os.path.basename(path) not in profiles.values():
            sys.exit(f"profiles/index.csv: no line for {os.path.basename(path)}")

    os.makedirs(OUT, exist_ok=True)
    buffer = io.StringIO()
    buffer.write("# Generated by tools/gamedb_build.py from the Hackset metadata and the Rust tables.\n")
    buffer.write("# data_crc,game,no_intro_name,region,data_md5,file_crc32s\n")
    writer = csv.writer(buffer, lineterminator="\n")
    for crc in sorted(rows):
        row = rows[crc]
        writer.writerow([
            f"{crc:08X}", row["game"], row["no_intro_name"], region_of(row["no_intro_name"]),
            row["md5"], " ".join(sorted(row["files"])),
        ])
    with open(os.path.join(OUT, "gamedb_roms.csv"), "w", encoding="utf-8", newline="\n") as file:
        file.write(buffer.getvalue())

    buffer = io.StringIO()
    buffer.write("# Generated by tools/gamedb_build.py from profiles/index.csv.\n")
    buffer.write("# game,profile\n")
    writer = csv.writer(buffer, lineterminator="\n")
    for game in sorted(games_with_rows):
        writer.writerow([game, profiles.get(game, "")])
    with open(os.path.join(OUT, "gamedb_games.csv"), "w", encoding="utf-8", newline="\n") as file:
        file.write(buffer.getvalue())

    for warning in warnings:
        print("warning:", warning)
    print(f"{len(rows)} ROMs, {len(games_with_rows)} games, {len(profiles)} profiles -> {os.path.normpath(OUT)}")


if __name__ == "__main__":
    main()
