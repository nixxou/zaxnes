# Mapper 168 (RacerMate Challenge II): the open problem

Parked by the owner on 2026-11-15: worth chasing because it may expose a core defect, but the
core is moving fast on another branch (FIDPLUS), so the investigation waits for that work to land.
This document is meant to let anyone resume it without the session that wrote it.

## The board as implemented

`rust/crates/zax-nes/src/cartridges/mappers/racermate.rs`, feature `board-168` (on by default),
lot MAPPERS-N. Sources: nesdev "INES Mapper 168" and puNES `mapper_168.c`.

* PRG: 16 KiB bank at `$8000` from the register's high bits, last bank fixed at `$C000`.
* CHR: 64 KiB of CHR-RAM in 4 KiB pages, `$1000-$1FFF` banked with `(reg & $0F) ^ 8` (the page
  allows either convention; puNES XORs).
* The control bit comes from A7 or D2 depending on the board revision (nesdev); a 1-to-0 edge
  clears the protection of U2 (CHR pages 8-15). While protected, a CHR read of U2 returns 0
  (the hardware leaves the PPU's open bus: approximation, FIDELITY-LEADS J22).
* A free-running M2 counter whose bit 10 drives /IRQ (puNES).

Mesen2 (`Racermate.h`) and FCEUX (`boards/168.cpp`) are simpler: no XOR, no IRQ, no protection,
and FCEUX decodes `$B000` alone. The cartdb does not correct anything for this mapper.

## What runs

Hackset, `NES Roms/Racermate Challenge II.7z` (the owner's local set). Zaxnes at frame 900,
build `ed2a619a`, no input:

| dump | CRC32 (PRG+CHR) | Zaxnes | FCEUX (lot notes) |
| --- | --- | --- | --- |
| v3.12.027 [USA][Australia] [!!] | `041553C3` | race screen | race screen |
| v3.11.088 [!!] | `74920C13` | black screen | runs |
| v5.01.033 [!!] | `AAEF2264` | plain cyan screen | runs |
| v6.02.001 [!!] | — | black screen | not checked |
| v6.02.002 [!!] | — | **"CART BAD AT 80 75 75 10 0F"** | not checked |
| v9.03.128 [!] | `F6A9CB75` | race screen | not checked |
| v6.02.002 GoodSet, 128 KiB (zz_) | — | "CART BAD", same message | not checked |

(`—`: compute with `zlib.crc32(data[16:])` when resuming; the three first were checked.)

## What was found for v3.11 and v5.01

* The screen stays on the backdrop. The decompressor at `$FCA1-$FCF4` keeps rebuilding the same
  list from `$FB29` and eventually executes code at `$DFxx`.
* **Not the board**: replacing the board with FCEUX's model (no XOR, no IRQ, no protection, each
  removed in turn and all together) gives exactly the same failure.
* `movie-sync` against FCEUX stops at the first divergence, which is the power-on phase (`$00CE`
  `8F`/`80` at frame 5): the two cores start one frame apart, so the real divergence is hidden
  behind it.
* Suspects, in this order: the CPU or the PPU during long `$2007` writes with rendering off
  (the game uploads its CHR-RAM that way), `$2007` read buffering, the open bus of a protected
  CHR read, and the IRQ timing if the game polls it during the upload.

## What is new (this document)

* The two v6.02.002 dumps print **"CART BAD AT 80 75 75 10 0F"**: a self-test of the cartridge
  fails, reporting an address (`$80`?) and the bytes it expected or read. This points at the
  **board model** (U2 protection, the CHR bank XOR, the A7/D2 control bit, or CHR-RAM size), not
  at the core: start here, it is the cheapest lead. Find the test with a trace (`frame-trace`) by
  locating the code that prints "CART BAD" and the reads it compares.
* v6.02.001 is black like v3.11: it may share the v3.11 cause.
* v9.03.128 works, like v3.12.

## How to resume

1. Rebase on the core as it is after FIDPLUS (and FIDPLUS-2) land; rerun the table above. If
   v3.11 / v5.01 now run, the cause was in the core and it is fixed: record which change.
2. v6.02.002 first: trace the "CART BAD" test, compare its expectation with the board model;
   try each variant (XOR 0 / 8, control bit from A7 or D2, protection off, IRQ off).
3. For v3.11 / v5.01: align the power-on of both cores before `movie-sync` (same frame phase,
   same RAM pattern) so that the first divergence reported is the real one. `fceux-frame` and
   `movie-sync` need the harness built with `fceux-core`, which currently fails to compile
   (`movie_sync.rs`: `NesRegion::PalM` / `PalN` not mapped to a FCEUX region); fix that first.
4. Whatever the cause, write it in `docs/review/FIDELITY-LEADS.md` J22 and close this document.

Tools: `zax-harness render --rom <rom> --run 900 --png-nes out.png`, `zax-harness frame-trace
<movie.fm2> <rom> --frame N --out trace.txt`, `zax-harness fceux-frame`, `zax-harness
movie-sync`.
