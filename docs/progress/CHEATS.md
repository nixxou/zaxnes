# CHEATS — cheat codes and a RAM search

Branch `feat/cheats`, worktree `wt-cheats`, from `dev` d02d6dd7 (2026-11-24).

## 1. What the other emulators do

| Emulator | Code formats | Memory search | Cheat database |
|---|---|---|---|
| FCEUX | Game Genie, Pro Action Replay, raw `AAAA:VV` and `AAAA?CC:VV`; `.cht` per game | Cheat search window (known value, relative: equal / not equal / greater / less, by delta), RAM Watch, RAM Search, hex editor | none shipped |
| Mesen / Mesen CE / Mesen2 | Game Genie, PAR, custom (address, value, optional compare) | memory viewer, watch, search in the debugger | **built in**: Cheats window, "Cheat Database", pick the game, its codes are imported; import from XML / CHT too |
| BizHawk (NesHawk) | Game Genie / PAR through the Game Genie encoder/decoder tool, raw cheats with compare | RAM Search (previous value, specific value, specific address, change count, difference; 1/2/4 bytes, signed/unsigned/hex, endianness), RAM Watch, hex editor, "freeze" | none |
| Nestopia UE | Game Genie, PAR, raw; XML cheat files | none built in | none built in; a community "Nestopia cheat code pack" exists |
| puNES | Game Genie, raw; imports libretro `.cht` and Nestopia XML | none | none (empty `cheat/` folder in 0.111) |
| RetroArch | `.cht` (libretro-database `cht/`, one file per game, downloaded by the Online Updater) | cheat search: start, then equal / not equal / less / greater / changed by N, 1/2/4 byte sizes, adds a found address as a cheat | **downloadable**: libretro-database `cht/Nintendo - Nintendo Entertainment System/` |
| ares | Game Genie / PAR in `Game.cheats.bml` beside the ROM, Tools > Cheats (since v136) | none | none for NES |
| BeesNES (MIT, `ext-tools/bees-src`) | Mesen JSON types `NesCustom`, `NesGameGenie` | none | **built in**: `Cheats/Cheats.zip`, 1958 games (No-Intro names), 45 686 codes (26 965 Game Genie, 18 721 raw), loaded by `LoadMesenJson`; provenance of the data not documented |

Notes for the NES:

- **Game Genie codes are ROM patches**, not RAM writes: the device replaces the byte the CPU reads at
  `$8000-$FFFF` (6 letters: address + value; 8 letters: + compare value, only replaced when the ROM
  byte equals it, which is what makes them safe with bank switching). They must be applied on the
  CPU read path, never by writing memory.
- **PAR and raw codes are RAM writes** repeated every frame (`AAAA:VV`), optionally only when the
  current value equals `CC` (`AAAA?CC:VV`).
- **Pointers are rare** on the NES: 2 KB of RAM at fixed places, plus up to 8 KB of WRAM at
  `$6000-$7FFF`. A pointer scan (as Cheat Engine does on PCs) buys little; what matters is the
  classic *unknown value* search (snapshot, then increased / decreased / unchanged / changed) and the
  *known value* search, with 8 and 16-bit values (little-endian), and **BCD / one digit per byte**,
  which NES games use for scores, lives and money.

## 2. Plan

| Step | What | State |
|---|---|---|
| 1 | `zax-cheats` crate, no engine dependency: code parsing (Game Genie 6/8, PAR, raw with compare), encoding back, a cheat list type | done |
| 2 | RAM search in `zax-cheats`: snapshot of `$0000-$07FF` and `$6000-$7FFF` through a read closure, filters (known value, unknown value relative to the last snapshot, by delta), sizes 1/2 bytes, BCD, candidate list, "add as cheat" | done (no "add as cheat" yet, it is UI) |
| 3 | Importers: Mesen JSON (BeesNES / Mesen), libretro `.cht`, FCEUX `.cht`, Nestopia XML | Mesen JSON, libretro, FCEUX done; Nestopia XML to do. FCEUX's layout from memory (its `cheat.cpp` is not in the tree): check against FCEUX before relying on it |
| 4 | Engines: RAM cheats through `Engine::write_memory` after each frame (all engines); Game Genie on Zaxnes's CPU read path (one flag tested only above `$8000`, off unless a code is on), FCEUX through its own cheat API, NesHawk through its bus poke | |
| 5 | Application: cheats per game saved beside the other per-game data, Cheats window (list, add, enable), Search window, libretro `retro_cheat_set` | |
| 6 | Database: decide what to ship (see 3.) | |

## 3. Open questions for the owner

- **Shipping a database.** BeesNES's zip (MIT repository, data of unknown origin, Mesen format) and
  libretro-database's `cht` (repository licence disputed between packagers: MIT, CC-BY-SA-4.0, GPL-3)
  are both available. Until decided: no database is shipped; the importers read one the player points
  at.

## 4. Progress

- 2026-11-24: worktree created; survey above; target dirs of every worktree cleaned (35 GB free on G:).
- 2026-11-24: steps 1-3 in `rust/crates/zax-cheats` (10 unit tests, clippy clean). The ignored test
  `tests/mesen_database.rs` read BeesNES's whole database (`ZAXNES_CHEAT_DB=<extracted Cheats.zip>`):
  1958 files, 45 616 cheats read, 70 skipped (notes without a code, `??` templates, 2 type errors in
  the data). It found Mesen's `AAAA:VV:CC` form and one file with an invalid JSON escape, both
  handled. Next: step 4, the engines.
