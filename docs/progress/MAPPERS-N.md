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
