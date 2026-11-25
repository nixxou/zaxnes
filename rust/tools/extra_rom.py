"""Adds ROM images the Hackset does not know (homebrews such as Shiru's Chase or Zooming Secretary)
to `profiles/extra_roms.csv`, which `tools/gamedb_build.py` puts in the game database:

    data_crc,game,name,data_md5,file_crc32

The data CRC and MD5 are taken over the PRG and CHR at the header's sizes, as Zaxnes's
`builtin_patches::data_crc` and `data_md5` (no header, trainer or overdump); `file_crc32` is the
whole file's. A line already there for the same data CRC is replaced.

Run:  python tools/extra_rom.py <rom.nes> <game> [<name>]
      (`game` is the game id the 3dSen profile names use in profiles/3dsen_names.csv; `name`
      defaults to the file name without its extension)
"""

import csv
import hashlib
import io
import os
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "profiles", "extra_roms.csv")
HEADER = [
    "# ROM images the Hackset does not know (tools/extra_rom.py), put in the game database.",
]
FIELDS = ["data_crc", "game", "name", "data_md5", "file_crc32"]


def data_of(image: bytes) -> bytes:
    """The PRG and CHR of `image` at the header's sizes (iNES / NES 2.0)."""
    if not image.startswith(b"NES\x1a") or len(image) <= 16:
        return image
    start = 16 + (512 if image[6] & 0x04 else 0)
    nes2 = image[7] & 0x0C == 0x08
    if nes2 and (image[9] & 0x0F == 0x0F or image[9] >> 4 == 0x0F):
        return image[start:]
    prg = image[4] | ((image[9] & 0x0F) << 8 if nes2 else 0)
    chr_banks = image[5] | ((image[9] >> 4) << 8 if nes2 else 0)
    size = prg * 16 * 1024 + chr_banks * 8 * 1024
    return image[start:start + size] if size else image[start:]


def main() -> None:
    path, game = sys.argv[1], sys.argv[2]
    name = sys.argv[3] if len(sys.argv) > 3 else os.path.splitext(os.path.basename(path))[0]
    image = open(path, "rb").read()
    data = data_of(image)
    row = {
        "data_crc": "%08X" % (zlib.crc32(data) & 0xFFFFFFFF),
        "game": game,
        "name": name,
        "data_md5": hashlib.md5(data).hexdigest(),
        "file_crc32": "%08X" % (zlib.crc32(image) & 0xFFFFFFFF),
    }
    rows = {}
    if os.path.isfile(OUT):
        with open(OUT, encoding="utf-8", newline="") as file:
            for record in csv.DictReader(line for line in file if not line.startswith("#")):
                rows[record["data_crc"]] = record
    rows[row["data_crc"]] = row
    buffer = io.StringIO()
    for line in HEADER:
        buffer.write(line + "\n")
    writer = csv.DictWriter(buffer, FIELDS, lineterminator="\n")
    writer.writeheader()
    for crc in sorted(rows):
        writer.writerow(rows[crc])
    with open(OUT, "w", encoding="utf-8", newline="\n") as file:
        file.write(buffer.getvalue())
    print(", ".join(f"{key} {value}" for key, value in row.items()))


if __name__ == "__main__":
    main()
