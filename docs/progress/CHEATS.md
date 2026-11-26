# CHEATS — cheat codes and a RAM search

Branch `feat/cheats`, worktree `wt-cheats`, from `dev` d02d6dd7 (2026-11-24). Continued on
`feat/cheats-2` in `wt-gamehash` from `dev` bcc9e86b + e6d19087 (2026-11-26): G: too full for a
second target directory; the two commits of `feat/cheats` cherry-picked.

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
| 3b | Sheet of a ROM: `zax_cheats::sheet::choose` and `zax_app::cheat_database` (folder or zip, Mesen JSON or libretro `.cht`), through the game database's No-Intro name of the data CRC, then the file name | done |
| 4 | Engines: RAM cheats through `Engine::write_memory` after each frame (all engines); Game Genie on Zaxnes's CPU read path, FCEUX through its own cheat API, NesHawk through its bus | done on the three engines (FCEUX through the fork branch `zax-cheats`) |
| 5 | Application: cheats per game saved beside the other per-game data, Cheats window (list, add, enable), Search window, libretro `retro_cheat_set` | done, with the console interface's page and RetroArch's T/Y/U |
| 6 | Database: decide what to ship (see 3.) | none shipped; the player points at one (`cheats.database`) or drops it beside the program |

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
- 2026-11-26: matching a ROM to its sheet. Rule, surest first: the sheet with the ROM's No-Intro
  name (`Exact`), the sheet with its file name (`FileName`, hacks and homebrews), the same title
  with a region in common (`SameRegion`, another revision or a Virtual Console image), the same
  title only (`OtherRegion`, codes may not work). The match kind is kept so the UI can warn below
  `Exact`. Against BeesNES's database, over the 3 144 ROMs of the game database: 1 568 exact,
  428 same region, 385 other region, 763 none (ignored test `coverage_of_a_real_database`).
- 2026-11-26: step 4, first half.
  - `Engine::set_cpu_read_patches(&[CpuReadPatch])` in `zax-core-api` (default: refused unless
    empty). Zaxnes takes them: while a patch is on, every frame (standard, slice, Fidelite+) runs
    the new board kind `PatchedBoard`, the dispatch of every board with `PATCHES_ROM` set, whose
    reads of `$8000-$FFFF` go through `NesBus::read_patched_rom_as` (out of line). The same rule
    as the VS. System's generic path: the specialised and generic paths hold no test at all, the
    only change they see is the boxed `rom_patches` field at the end of `NesBus`. The fast core is
    off while a patch is on. `read_memory` shows the patched bytes too.
  - `EngineHandle::set_cheats(&CheatList)`: RAM codes applied after every frame (whole frame,
    last slice, run-ahead frame) through `read_memory` / `write_memory`, so on every engine; Game
    Genie codes handed to the engine; both dropped by `load_rom`.
  - Tests: `zax-nes/tests/game_genie_tests.rs` (a program's `LDA #` operand patched, with and
    without compare, standard and Fidelite+), `zax-app/tests/cheat_engine_tests.rs`.
  - To do: a callgrind level-1 run of the standard path against the base (C: too full for the
    WSL builds today); FCEUX's and NesHawk's Game Genie; with run-ahead, the RAM codes are applied
    to the frame shown, after the frames run ahead.
  - Seen on the way, not changed: the Fidelite+ loop and the slice loop do not send a VS. System
    image to the generic path as the standard frame does, and `Fid<K>` does not forward
    `RUNS_VS`; the VS quirks may be missing on those paths.
- 2026-11-26: steps 4 and 5 finished (owner: "fais moi une implementation complete").
  - Host (`emulation/host/cheats.rs`): the list of the game in play, kept in
    `cheats/<rom>.json` (the entry's name for a game in an archive), read back at each launch,
    put on the core whenever its stamp (`EngineHandle::cheats_stamp`, 0 after a new ROM) is not
    the host's; a switch for them all; the RAM search runs in the host at the tick, the core
    here (with the separate core the UI draws while the core is away); `cheats.database`
    (internal setting), else `cheats.zip` / `cheats/` beside the program, else
    `cheats/database(.zip)` in the user data; the data CRC is read when the game starts.
  - Run-ahead (on by default) defeated the RAM codes: written to the frame shown, then lost when
    the real machine came back. `EngineHandle::restore_real_frame` and `run_frame_ahead` now
    write them to the real machine too. Found by `tests/cheat_host_tests.rs`.
  - Desktop: Game options > Cheats... (list, typed codes, the database's sheet with its match
    coloured: green exact or file name, orange another version, red another region; Add one on,
    Add all off; Browse for a zip or a sheet of a folder) and RAM search... (1 byte, 2 bytes,
    BCD; changed/unchanged/increased/decreased, a comparison with the previous value, the
    previous plus a delta or a value; undo; the first 500 listed with their value now; add as a
    cheat). RetroArch layout: T/Y the previous/next cheat, U on/off, with a notification.
  - Console interface: a Cheats row in the in-game menu (after Hacks): the switch, the sheet's
    codes taken in one press (off), each cheat on/off. No typing, no RAM search on a pad.
  - libretro: `retro_cheat_reset`, `retro_cheat_set` (RAM codes before each frame, Game Genie on
    the CPU reads).
  - NesHawk: `Bus::rom_patches` on the `$8000-$FFFF` read and peek paths (a `None` test on the
    ROM read, the C#'s path unchanged otherwise), kept through a power cycle, out of the state.
  - FCEUX: `zax_set_read_patches` in the fork (branch `zax-cheats`, worktree
    `cleanroom/fceux-zax-cheats`, commit 0cb755df): FCEUX's substitute cheats (type 1), the
    list replaced as a whole, `savecheats` reset so closing a game never writes FCEUX's `.cht`.
    Only the MSVC library is built; the Linux (zig), aarch64 and Android libraries of the fork
    must be rebuilt from that branch before a release links them, or the link fails.
  - Not done: the callgrind level-1 comparison of the standard path (no Zaxnes core change since
    041b7a6e's boxed field and out-of-line patched path).
