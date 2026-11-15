# MAPPERS-N — the mappers other emulators run and Zaxnes does not

Branch `feat/mappers-n`, worktree `G:/3DSen/wt-mappers`, base `e2b7fced` (feat/fidplus-1).
Owner's request (2026-11-15): implement the mappers missing compared with the other emulators.

## Resume note (read first after a crash)

* Every finished board is committed; the table below says which. Anything not in a commit is lost.
* Build: one target dir, `rust/target` in this worktree. `df -h /g` before a big build.
* Next step is always the first row of the table whose state is not "done"/"non vérifiée".

## Inventory (step 1)

* `SUPPORTED_MAPPERS` at e2b7fced: 144 numbers.
* Mesen2 `MapperFactory.cpp` (`cleanroom/third_party/mesen2/Core/NES/MapperFactory.cpp`): 309
  `case` labels; **165 numbers Mesen2 lists and Zaxnes does not**, of which ~25 are empty `case`
  labels in Mesen2 itself (186, 256, 257, 271, 295, 315, 322, 327, 335, 337-342, 344, 345, 350,
  524-528). Those are out: no emulator of the allowed list implements them in Mesen2's table.
* Local ROMs: a header scan of every `.nes` under `G:/3DSen` (test-originals, test-originals-extra,
  demo/roms, assets-free/test-roms, mm1corpus, roms-owner, …) finds **two** files on a missing
  number: mapper **250** (`roms-owner/Insector_X/… queen_bee_v_unl_.nes`) and mapper **253**
  (`roms-owner/Excitebike_1_/… vs._excitebike_fds_conversion_b1.nes`). Every other board of this
  lot is therefore "non vérifiée" (unit tests from the page only).
* Game counts: entries per mapper in Mesen2's game database (`ext-tools/MesenCE-2.2.1/MesenNesDB.txt`).

## Design

* No new `Board` variant per board. Two shared wrappers, as `DecodeBoard`/`IrqBoard` already are:
  * `Mmc3Clone` (in `IrqBoard`): an `Mmc3` plus an outer register file and a `kind`; the kind
    decides the register decode and how the MMC3's bank numbers are masked/offset — the shape
    Mesen2 gives these boards (subclasses overriding `SelectPrgPage`/`SelectChrPage`).
  * `LatchMulti` (in `DecodeBoard`): the discrete multicarts — a few latched registers, the kind
    deciding the decode and the banking.
* Features: `board-mmc3-clones` and `board-latch-multi` umbrellas, one `board-NNN` per number.

## Plan, ordered (games = Mesen2 DB entries)

| # | mapper(s) | games | ref | family | difficulty | state |
|---|---|---:|---|---|---|---|
| 1 | 250 | 2 | Mesen2 `MMC3_250.h` | MMC3 clone (address lines) | S | **done**, Queen Bee V plays (4e9082a6) |
| 2 | 253 | 5 | Mesen2 `Mapper253.h` | VRC4 + CHR-RAM | M | non vérifiée: local dump is a bad dump [b1] (ba013045) |
| 3 | 45, 52, 134, 44, 49, 205, 197, 198, 199, 196, 245, 249, 254, 126, 191, 217, 238, 287, 325, 259, 313 | ~450 | Mesen2 `MMC3_*.h` | MMC3 clones | S-M each | todo |
| 4 | 58, 241, 59, 57, 227, 212, 242, 236, 235, 226, 221, 63, 200-204, 213, 214, 229-231, 233, 240, 244, 255, 60-62, 46, 51, 104, 108, 113, 120, 125, 170, 174, 177, 179, 216, 261, 274, 286, 289 | ~550 | Mesen2 `Mapper*.h`, `Bmc*.h` | discrete multicarts | S each | todo |
| 5 | 40, 42, 43, 50, 106, 117, 222, 183, 56/142, 171 | ~40 | Mesen2 | discrete + IRQ | M | todo |
| 6 | 6, 8, 17 | ~430 | Mesen2 `FrontFareast.h` | FFE copier + IRQ | M | todo |
| 7 | 176 | ~420 | Mesen2 `Fk23C.h` | MMC3 multicart, large | L | todo |
| 8 | the rest (Kaiser FDS conversions, Subor, …) | few | Mesen2 | various | M | todo |

## Log

* 2026-11-15 4e9082a6 — `Mmc3Clone` (IrqBoard): 44 45 49 52 134 196 197 205 238 245 249 250. 250 verified
  on Queen Bee V (menu + gameplay with IRQ split). Others: unit tests (`tests/mappers_n_tests.rs`).
* ba013045 — 253 (`Waixing253`, IrqBoard, clocks on CPU cycles). The only local dump has its vectors
  in zero padding: unusable in any emulator; unit tests only.
* a415bbcd — `LatchMulti` (DecodeBoard): 57 58 59 63 200 201 202 203 204 212 213 214 221 226 227 229
  230 231 233 235 236 240 241 242 244 255. **board-200 off by default**: SPEC-02 REQ-CORE-102's
  acceptance criterion is "mapper 200 → échec"; amendment proposed in
  `docs/review/AMD-MAPPERS-N-200.md` (move it to 186).
* Tools: `scratchpad/register.py` (features, SUPPORTED_MAPPERS, create_board, A12 lists) and
  `addtest.py` (req_core_100 table) — not committed; redo by hand if lost.
* Baseline on e2b7fced: campaign 215/4/1/111, golden 98 ROMs in scratchpad `base/golden-base.jsonl`.
* ffc5e4ca — `LatchMulti` + 46 51 60 61 62 104 108 113 120 125 170 174 177 179 216 261 274 286 (ROM at
  $6000, RAM at $C000 for 125, reset-switched 60). DecodeBoard forwards `prg_ram_enabled`.
* 0b9aea7c — `Mmc3Clone` + 126 224 259 287 313 325; 191 in `Mmc3ChrRamBoard`.
  Checkpoint ck1 on this commit: golden-core identical to e2b7fced (98 ROMs), campaign 215/4/1/111.
* c2d6e19f — `IrqLatch` (IrqBoard): 40 42 43 50 106 183 (CPU clock), 117 222 (A12).
* f07f0fc0 — 6 8 17 (Front Fareast) on `IrqLatch`. Refusal tests now use 20 / 84 / 186 / 200.
* Perf: WSL `~/pr1`, labels `mn-base` (e2b7fced) and `mn-ck2` (c2d6e19f), driver script in
  scratchpad `perf/run.sh`; results `~/pr1/cg-mn-*`.
* Disk: `rust/target` reached 14 GB; build with `CARGO_INCREMENTAL=0` and delete stale
  `target/debug/deps/*.exe`.
* Next: 176 (FK23C), then 199/198, Kaiser FDS conversions (56/142, 171, 175, 302-307, 312), Subor
  166/167, 252, 265, 266, 283, 288-290, 300-301, 314, 319-320, 323-324, 328-333, 336, 346, 348-349,
  366, 513, 518-522, 529-530.
* 6446182e — 117 of 142 done (see the summary above for the 25 left).
* Hackset (`C:\Users\…\Downloads\Hackset-NES`, owner's local set; its per-archive JSON gives each
  ROM's mapper, so nothing is scanned blind): 31 of the 117 new mappers have dumps, plus 7 of the
  25 left (165 168 188 198 199 234 252). Dumps chosen [!]/[!!] first, then GoodSet without hack tags,
  hacks last (owner: some hacks do not work). Rendered at frame 900 with Start presses; suspects
  checked against FCEUX (`zax-harness fceux-run/fceux-compare`, harness built with `fceux-core`).
  - Verified (boot + picture): 6 8 17 42 46 56 62 106 113 120 142 148 177 183 191 196 216 222 238
    240 241 242 244 245 250 253 254 290 521, then 108 and 176 after the fixes below.
  - Not the board: 42 Rush'n Attack conversion (FCEUX garbles it too), 244 Asmik-kun Land [t1]
    (trainer hack, FCEUX grey too), 254 [b1] bad dump.
* Fixes from the Hackset: 108 per nesdev/puNES submappers (FIDELITY-LEADS J11); 8 KiB iNES 1.0
  CHR-RAM on the new boards (J12); FK23C CHR-RAM bit needs CHR-RAM (J13); `registry.rs` 262 arm
  had lost its `cfg` to the 6/8/17 arm in a scripted insertion, restored.
* Next: cross-check every new board against nesdev + puNES (+ FCEUX where it has the board), log
  divergences in FIDELITY-LEADS; then the 7 remaining mappers that have Hackset dumps.
* Old exotic boards, Hackset check (owner: "certains ont été fait avec de la théorie seule"):
  114 old mappers with Hackset dumps, 189 ROMs ([!] first), rendered at frame 900 and reviewed on
  contact sheets; 166 good, 19 suspects compared with FCEUX (triage only: "fceux n'est pas un
  modèle de fiabilité") and decided on nesdev / puNES / Mesen2.
  - 0b963dee mapper 34 NINA-001: `$7FFD` write did not refresh the cached read windows
    (Impossible Mission II).
  - 3431b154 mapper 15: CHR-RAM write protection only on the K-1029 (nesdev), Bao Xiao San Guo.
  - 98ab699f battery RAM powers on as a seeded pattern without a save (owner: "fais ce qui te
    semble le mieux"), Famicom Jump II; movie replay keeps zeros. AMD-MAPPERS-N-200 now proposes
    84 (186 is run by FCEUX and puNES).
  - 753ee24e cartridge database `bNNN` EEPROM sizes reach byte 10: Datach Battle Rush.
  - 285d40ea mapper 111 with CHR-ROM = parallel MMC1 (Ninja Ryukenden Chinese, puNES).
  - Not the board: 74_0 (mislabelled FK23C game, runs as 176 sub 2), 143 Dancing Blocks (reads
    uninitialised work RAM), 14_0 / 15_1 (wrong headers), 153 needed the battery-RAM change,
    42 / 244 / 254 bad dumps or hacks.
  - Scratchpad: `old/` ROMs, `old_sel.json`, `old_sheet*.png`, `susp*.png`.
* Next: the 7 remaining mappers with Hackset dumps (165 168 188 198 199 234 252), then the
  nesdev/puNES/FCEUX cross-check of the 117 new boards, the 18 other remaining numbers, final
  validation (golden will move for battery games and the 53 `bNNN` database entries: explain).
* Hackset-verifiable remainder: 234 (1654f521, Maxi 15 x3), 252 (7246ef17, Sangokushi Chinese;
  its state reload refresh fixed in the next commit), 165 (Fire Emblem Gaiden Chinese x3). Left:
  198, 199, 168, 188. AMD-MAPPERS-N-200 accepted and applied (6e10b0ef).
* 168 RacerMate Challenge II (nesdev + puNES): v3.12 reaches the race screen; v3.11 and v5.01
  stay on the backdrop in Zaxnes but run in FCEUX, with FCEUX's board model as well, so the
  cause is in the core (FIDELITY-LEADS J22). Next: cartdb whitelist 199->176 / 74->176, then 188.
* Cartridge database: the mapper number is corrected for the two mislabellings nesdev documents
  (199 -> 176, 74 -> 176/191/192/194/195), on an exact CRC match only (docs/review/CARTDB.md).
  The three Hackset "199" games and the two 74 dumps boot. Golden/campaign may move for a test
  ROM with such a header: check at final validation.
* 188 Bandai Karaoke Studio (nesdev + Mesen2 + puNES): the three Hackset [!] dumps reach their
  song lists; the microphone buttons are controller 1's A and B (`Mapper::set_input_word`, one
  board dispatch per frame behind `board-188`), level 0 (FIDELITY-LEADS J23).
* To report to the owner: `cargo check -p zax-nes --no-default-features` fails at the base
  e2b7fced already (`ppu/mod.rs` `read_nametable_as::<K>` outside a generic, and registry's
  `Mmc3ChrRamBoard` used without its feature) -- not this lot's, not touched (ppu is off limits).
* 31, 103, 175, 307 in `FineRom` (now any board decoding `$6000-$FFFF` itself, RAM included).
  103 verified on Doki Doki Panic (world 1-1); the three others have no dump. The
  `nametable_pages_vary` list left `Cartridge::create` for a const fn, as the clock list did.
  Remaining: 53 260 266 284 303 323 328 487 513 518 519 529 530 682.
* 303 (KS7017) in `IrqBoard`, no dump. Remaining: 53 260 266 284 323 328 487 513 518 519 529
  530 682.
* 53, 487, 518, 519 in a new `MiscN` (DecodeBoard), no dumps. Remaining: 260 266 284 323 328 513
  529 530 682.
* 323 (`FaridSlrom`, a wrapper of `Mmc1`), no dump. Remaining: 260 266 284 328 513 529 530 682.
* 513 (`Mmc3Clone` M513), no dump. Remaining: 260 266 284 328 529 530 682.
* 530 (`Ax5705`, a wrapper of `Vrc2Vrc4`), no dump. Remaining: 260 266 284 328 529 682.
* 529 (`T230`, a wrapper of `Vrc2Vrc4`), no dump; its 93C56 EEPROM is an open lead (J30).
  Remaining: 260 266 284 328 682.
* 266 (`CityFighter`, a wrapper of `Vrc2Vrc4`), no dump; DAC silent (J31). Remaining: 260 284
  328 682.
* 328 (`MiscN` M328), no dump. Remaining: 260 284 682.
* 260 (`Mmc3Clone` M260), no dump. Remaining: 284 (Drip, with its sound chip), 682 (Rainbow).

## Not implemented, with the reason (owner's call to reopen)

* **284 — reopened and done** (see the end of this note): the "PPU files" reason was wrong, the
  PPU side of the MMC5's path is already generic and the board filter is in `board.rs`.
  Original note kept for the record:
* **284, the Drip homebrew board** (Mesen2 `Homebrew/UnlDripGame.h`). Its banking, IRQ and two
  FIFO PCM channels fit the existing hooks (`expansion_audio_output`), but its *extended
  attributes* replace the attribute byte of **rendering fetches only**, per tile, from 2 KiB of
  board memory: that needs the PPU's fetch path to tell a rendering read from a `$2007` read and
  to hand the board the tile being fetched. Today only the MMC5 has such a path
  (`Board::read_nametable_from_vram` / `position_captured_background_tile`, behind
  `board-mmc5-exattr-split`, measured on every fetch), and this lot may not touch `ppu/`. A board
  without the attributes would show the demo in the wrong colours, so it is left out rather than
  half-done. Reopening it means widening that MMC5 path to a second board, with a perf check.
* **682, Rainbow** (Mesen2 `Homebrew/Rainbow.cpp`, 1 300+ lines): an ESP32 Wi-Fi co-processor
  with its own message protocol and network access, self-flashing PRG/CHR, its own audio and
  several nametable/attribute extension modes. A board of its own size, with a network side this
  core has nowhere to put; out of this lot's scope.
* 284 (`Drip`, IrqBoard) done: extended attributes ride the MMC5's `board-mmc5-exattr-split` path
  with one more arm in each of `Board`'s six MMC5 methods; PCM channels as expansion audio
  (J34). No dump. Next: 682 Rainbow without Wi-Fi, then its Wi-Fi behind an emulation option off
  by default (owner, 2026-11-15), then callgrind and the merge request.

## Mapper 682, Rainbow (Broke Studio) — plan (owner, 2026-11-15: all of it, Wi-Fi behind an option)

Sources: Mesen2 `Homebrew/Rainbow.{h,cpp}`, `RainbowAudio.h`, `FlashS29.h` (mainline: everything
but Wi-Fi, `$4190-$4194` are flags only); Broke Studio's `rainbow-net/NES/mapper-doc.md` and
`rainbow-net.md` (the ESP message protocol, ~80 commands). The Broke Studio Mesen fork (with the
ESP) is not local: downloading it needs the owner's consent; otherwise the protocol doc is the
source.

Mapping onto Zaxnes's hooks (no `ppu/` change):
* CPU: every read `$4100-$FFFF` through `cpu_read_override` (all windows `NO_ROM_WINDOW`): 4 KiB
  high banks (5 modes) and 4/8 KiB low banks mixing PRG-ROM, PRG-RAM and FPGA-RAM, FPGA-RAM at
  `$4800-$4FFF` (last 2 KiB) and `$5000-$5FFF` (4 KiB bank), registers, `$415F` auto-increment
  port, `$4280-$47FF` generated OAM code, NMI/IRQ vector override at `$FFFA/$FFFE`.
* PPU: `on_ppu_address_bus` (every rendered fetch) runs Mesen2's counters (`DetectScanlineStart`,
  in-frame, idle counter on the CPU clock, NT fetch counter, scanline IRQ at offset), computes
  the window flag and the ext data; `read_nametable_from_vram` serves the four nametable sources
  (CIRAM / CHR-RAM / FPGA-RAM / CHR-ROM), fill mode, attribute ext mode and the window;
  `ppu_read` substitutes background ext tiles, window tiles and sprite ext tiles; the 3D capture
  through `position_captured_background_tile` (the Drip's arms in `board.rs`).
* Bus additions (`nes_bus.rs`, allowed): `$2003`/`$2004` writes to the cartridge (sprite ext's OAM
  Y copy), `$4011` read (audio level / IRQ ack); `$2000` via `on_ppu_control`.
* Audio: two VRC6 pulses and a saw (reuse `vrc6` audio if it splits), volume `$41AA`.
* Flash: `FlashS29` (S29GL) self-programming of PRG and CHR, saved as a ROM patch (Mesen2
  `SaveRom`); phase 4.
* Wi-Fi: an ESP model answering the protocol, behind an emulation option **off by default**;
  sockets on a worker thread, files in a per-ROM folder; savestates/rewind/replay marked
  offline while it is on. Phase 3.

Phases: 1 CPU side + nametable sources/fill/attr ext + IRQs + audio + vectors; 2 window, bg/sprite
ext, capture path, bus hooks; 3 Wi-Fi option; 4 flash. Each phase committed on its own.

### Rainbow progress
* Phase 1 done (`rainbow.rs`, `board-682`): CPU map, PRG/CHR banking, FPGA RAM and port, four
  nametable sources, fill, attribute/background extended modes, window, both IRQs, vectors, OAM
  routines, audio on `Vrc6Audio`. Test `mappers_n_682_banks_fills_and_counts`; zax-nes suite and
  clippy green. Leads in J35.
* Phase 2 done: `NesBus::snoops_cpu_bus` (attach-time flag) feeds `$2003`/`$2004` writes and the
  `$4011` read to the board; separate attribute answer; 3D capture arms in `board.rs`. Tests
  `mappers_n_682_*` (2), zax-nes suite and clippy green; workspace builds (`zax-app` link needs the
  SDL2 cmake build that a target purge removed, not rebuilt for disk space).
* Phase 4 done: S29 flash on PRG (save file and savestate through `FlashRom`) and CHR (in place);
  test `mappers_n_682_programs_and_erases_its_prg_flash`.
* Phase 3 done: `rainbow_esp.rs`, the ESP protocol (status, debug, buffers, Wi-Fi, random, server
  TCP/UDP/UDP pool, network registry, the 16 file commands, HTTP download); the host network
  behind the setting `emu.rainbow_network` (Emulation page, off by default, NES and app strings
  in English and French). Test `mappers_n_682_talks_to_its_esp`. Leads J36.
* Rainbow is complete as planned; no dump tested (none local, downloading needs consent).

### Callgrind (level 1, clean builds from `git archive`, mm2 and zelda2, 300 frames)
`core` against the base e2b7fced:

| commit | mm2 core | zelda2 core |
| --- | ---: | ---: |
| 3d513439 (the ~60 boards before 284) | +0.001 % | +0.001 % |
| 2eb5cd03 (284) | +0.045 % | +0.044 % |
| c8d5c9af (682 phase 1) | +0.045 % | +0.044 % |
| bc828851 (682 phase 2) | +0.046 % | +0.046 % |
| 274ff3e9 (682 complete) | +0.046 % | +0.046 % |
| 3e7e6570 (284 off, 682 on) | +0.025 % | +0.025 % |
| a8eb1aab (both off, the default) | +0.001 % | +0.001 % |
| b96b839b (both on, one out-of-line arm in `board.rs`) | +0.025 % | +0.025 % |
| 692b3254 (`read_nametable_inner` generic over the board kind) | +0.046 % | +0.046 % |
| ed2a619a (plus `BoardKind::SERVES_NAMETABLES`) | **-0.061 %** | **-0.060 %** |

Both are over the +0.02 % per board of the mapper rule, on MMC1 games that use neither: the cost
is code layout in the MMC5 ext-attr dispatches (`Board::read_nametable_from_vram` and its
siblings) that each board widens. So `board-284` and `board-682` are **off by default** (parked,
not deleted; `--features board-284,board-682` builds and passes the suite and clippy). The
setting `emu.rainbow_network` stays in the app and does nothing without `board-682`. Making them
free (one shared arm for every cartridge-served board in those dispatches) is a lot of its own.

**Resolution (owner, 2026-11-15: "fait le et documente bien même dans la source"; `ppu/mod.rs`
and `ppu/tiles.rs` opened for this).** The cost was never the arms themselves: it was a stack
frame (`push`/`pop`) that any non-tail call in the served-nametable branch gives
`Ppu2C02::read_nametable_inner` on every nametable fetch of every game. Fix:
`read_nametable_inner` / `read_nametable_as` are generic over `BoardKind`, and
`BoardKind::SERVES_NAMETABLES` (false for the six one-board kinds of `board_kind!`, true for the
dispatch and the two MMC5 kinds) compiles that branch out of the specialised paths. Specialising
alone (692b3254) did not help, because the `unreachable!` of `BoardKind::board` brought the same
frame back. Both boards are on by default again, golden-core identical, and the specialised paths
are 0.06 % faster than the base. The source notes on `read_nametable_inner`,
`BoardKind::SERVES_NAMETABLES` and `Cartridge::read_nametable_from_vram_as` carry the figures for
the backport.

### VS. System unparked (owner, 2026-11-15: "bien entendu qu'il faut l'implémenter")
* Cost found: four VS fields in `Ppu2C02` changed its layout and `run_dots`' code (+0.090 % /
  +0.103 % `core`), then one test on `NesBus::read_cycle_as` (+0.045 % zelda2).
* Fix: the three PPU quirks (`$2002` identifier, `$2000`/`$2001` swap, palette remap) applied by
  `NesBus` from the `VsSystem` it holds (`vs_port` became a byte of flags, same size);
  `Ppu2C02::remap_palette` re-shows the palette after each palette write, the power-on and a
  state load; VS images always run on the generic path, `BoardKind::RUNS_VS` compiles the quirks
  out of the specialised ones. Result (clean builds, c89d20f1): `core` +0.000 % / +0.001 %.
* Inputs: hotkeys Coin 1, Coin 2, Service (keys 5, 6, 9; rebindable), commands in the menu,
  `NesCore::vs_insert_coin` / `vs_press_service` / `set_vs_dip_switches`, settings
  `emu.vs_dip_1..8` (Emulation page, next load). The service button is a 4-frame pulse.
* The VS tests had been broken since lots FIDELITY/CARTDB (reset line, palette-address backdrop,
  cartdb), unseen because the feature was parked; repaired.
* Not done: dual-system VS (VS Tennis, VS Mahjong...: two consoles), as in MF-VS; a movie
  (`.bk2`) does not record coins.

